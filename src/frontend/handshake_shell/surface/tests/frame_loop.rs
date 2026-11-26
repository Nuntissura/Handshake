//! The presenter on a real loop with real windows: quiet, shown at the bottom of the Z order of a private desktop that
//! receives no input, each test in a process of its own. A test is a list of steps a wake of the application runs one
//! after the other; a step that does not finish in time fails the test with its name.
//!
//! - `an_idle_window_submits_nothing_and_one_damage_presents_one_frame`: AC214-4, and the capture of a presented
//!   swapchain image compared texel by texel; the watch of MT-215 HB-4 says the wanted frame was presented.
//! - `resize_minimise_restore_and_scale_change_recreate_the_swapchain`: AC214-3; every replaced swapchain ends.
//! - `closing_a_window_releases_its_target_once_and_keeps_the_other_presenting`: AC214-3, AC214-8.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use handshake_shell_surface::{
    Background, ClearFrame, PresentConfig, PresentHost, PresentReport, ReadbackError, Shell,
    ShellApp,
};
use handshake_shell_window::{
    CommandSource, DispatchedWake, Display, Event, LogicalSize, LoopConfig, PhysicalPosition,
    Placement, RunReport, ShellHost, WakeLane, WindowClass, WindowCommand, WindowEvent, WindowKey,
    WindowSpec,
};
use handshake_vulkan_renderer::ModuleId;
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, SendMessageW, WM_DPICHANGED};

use support::desktop::{must_stay_inactive, on_private_desktop, run_script, stop_watching};
use support::gpu_env::{in_own_process, ran_in_its_own_process};

const LIMIT: Duration = Duration::from_secs(100);
const STEP_LIMIT: Duration = Duration::from_secs(15);
const TICK: Duration = Duration::from_millis(10);
const FRAME: ClearFrame = ClearFrame::new(Background::DARK);

/// The surface offered transfer source use, and the presented frame was read back.
static CAPTURED: AtomicBool = AtomicBool::new(false);

type Run = Box<dyn FnMut(&Shell<'_>, &App) -> bool>;

struct Step {
    name: &'static str,
    run: Run,
}

fn step(name: &'static str, run: impl FnMut(&Shell<'_>, &App) -> bool + 'static) -> Step {
    Step {
        name,
        run: Box::new(run),
    }
}

/// The application: it opens the windows, shows them, then runs the steps on its own lane.
struct App {
    wanted: usize,
    lane: RefCell<Option<WakeLane>>,
    steps: RefCell<VecDeque<Step>>,
    started: Cell<Instant>,
    windows: RefCell<Vec<WindowKey>>,
    hwnds: RefCell<BTreeMap<WindowKey, HWND>>,
    destroyed: RefCell<BTreeSet<WindowKey>>,
    shut: Cell<bool>,
}

impl App {
    fn window(&self, number: usize) -> WindowKey {
        self.windows.borrow()[number]
    }

    fn hwnd(&self, number: usize) -> HWND {
        self.hwnds.borrow()[&self.window(number)]
    }

    fn destroyed(&self, key: WindowKey) -> bool {
        self.destroyed.borrow().contains(&key)
    }

    fn next_tick(&self) {
        if let Some(lane) = self.lane.borrow().as_ref() {
            lane.request(0, TICK).expect("the lane takes a wake");
        }
    }
}

impl ShellApp for App {
    fn on_event(&self, shell: &Shell<'_>, event: &Event) {
        match event {
            Event::LoopStarted => {
                *self.lane.borrow_mut() = Some(shell.control().open_wake_lane().expect("a lane"));
                for number in 0..self.wanted {
                    let at = 40 + 60 * number as i32;
                    let spec = WindowSpec {
                        title: "frame loop".to_owned(),
                        inner: LogicalSize::new(240.0, 160.0),
                        min_inner: None,
                        placement: Placement::At(PhysicalPosition::new(at, at)),
                        class: WindowClass::Quiet,
                    };
                    shell
                        .open_window(&CommandSource::host(number as u64 + 1), spec)
                        .expect("a window");
                }
            }
            Event::Window { window, event } => match event {
                WindowEvent::Created { .. } => {
                    // Read and let go at once: a token held here would keep the window from being destroyed.
                    let hwnd = shell
                        .control()
                        .native_window(*window)
                        .expect("a token")
                        .win32()
                        .hwnd
                        .get();
                    must_stay_inactive(hwnd);
                    self.hwnds.borrow_mut().insert(*window, hwnd);
                    let presentation = shell.presentation(*window).expect("presents");
                    assert!(presentation.swapchain.is_some(), "a swapchain before the first show");
                    shell
                        .control()
                        .command(
                            &CommandSource::host(60),
                            *window,
                            WindowCommand::Show { activate: false },
                        )
                        .expect("shown");
                    self.windows.borrow_mut().push(*window);
                    if self.windows.borrow().len() == self.wanted {
                        self.started.set(Instant::now());
                        self.next_tick();
                    }
                }
                WindowEvent::Destroyed => {
                    self.destroyed.borrow_mut().insert(*window);
                    if let Some(hwnd) = self.hwnds.borrow().get(window) {
                        stop_watching(*hwnd);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn on_wake(&self, shell: &Shell<'_>, _: &DispatchedWake<'_>) {
        if self.shut.get() {
            return;
        }
        let mut steps = self.steps.borrow_mut();
        let Some(step) = steps.front_mut() else {
            drop(steps);
            self.shut.set(true);
            shell.shutdown(0).expect("shutdown begins");
            return;
        };
        if (step.run)(shell, self) {
            steps.pop_front();
            self.started.set(Instant::now());
        } else {
            assert!(
                self.started.get().elapsed() < STEP_LIMIT,
                "step \"{}\" did not finish within {STEP_LIMIT:?}; {:?}",
                step.name,
                shell.report()
            );
        }
        drop(steps);
        self.next_tick();
    }
}

struct Ended {
    report: PresentReport,
    run: RunReport,
    present_fences: bool,
}

/// Run `steps` (made on the loop's thread) with `windows` presenting windows.
fn present_loop(
    windows: usize,
    steps: impl FnOnce() -> Vec<Step> + Send + 'static,
) -> Option<Ended> {
    on_private_desktop(LIMIT, move || {
        let app = Rc::new(App {
            wanted: windows,
            lane: RefCell::default(),
            steps: RefCell::new(steps().into()),
            started: Cell::new(Instant::now()),
            windows: RefCell::default(),
            hwnds: RefCell::default(),
            destroyed: RefCell::default(),
            shut: Cell::new(false),
        });
        let config = PresentConfig::new(support::policy(), ModuleId(214));
        let log = Arc::new(support::EventLog::default());
        let host = match PresentHost::open(config, log, app.clone()) {
            Ok(host) => host,
            Err(reason) => {
                support::skip_without_device(reason);
                return None;
            }
        };
        let present_fences = support::has_present_fences(host.owner());
        let (events, wakes) = (Rc::clone(&host), Rc::clone(&host));
        let run = run_script(
            LoopConfig::default(),
            move |control, event| events.on_event(control, event),
            move |control, wake| wakes.on_wake(control, wake),
        )
        .expect("the loop ran");
        assert!(app.steps.borrow().is_empty(), "every step ran");
        assert_eq!(run.quiet_broken, 0);
        assert_eq!(run.windows_created, run.windows_destroyed);
        Some(Ended {
            report: host.report(),
            run,
            present_fences,
        })
    })
}

/// Submissions of the owner, presents of the window's surface, frames due of the window.
fn counts(shell: &Shell<'_>, window: WindowKey) -> (u64, u64, u64) {
    let presented = shell
        .presentation(window)
        .and_then(|presentation| presentation.progress)
        .map_or(0, |progress| progress.presented);
    let due = shell
        .control()
        .frame_progress(window)
        .map_or(0, |progress| progress.due);
    (shell.owner().counters().submitted, presented, due)
}

/// A frame was presented for `window` and none is wanted any more.
fn settled(shell: &Shell<'_>, window: WindowKey, presented_above: u64) -> bool {
    let (_, presented, _) = counts(shell, window);
    let waiting = shell
        .control()
        .frame_progress(window)
        .and_then(|progress| progress.waiting);
    presented > presented_above && waiting.is_none()
}

fn swapchain_number(shell: &Shell<'_>, window: WindowKey) -> u64 {
    shell
        .presentation(window)
        .and_then(|presentation| presentation.swapchain)
        .map_or(0, |info| info.swapchain)
}

fn command(shell: &Shell<'_>, window: WindowKey, command: WindowCommand) {
    shell
        .control()
        .command(&CommandSource::host(70), window, command)
        .expect("the window takes the command");
}

#[test]
fn an_idle_window_submits_nothing_and_one_damage_presents_one_frame() {
    in_own_process!();
    let Some(ended) = present_loop(1, || {
        let base = Rc::new(Cell::new((0, 0, 0)));
        let idle_since = Rc::new(Cell::new(Instant::now()));
        let (set, idle) = (Rc::clone(&base), Rc::clone(&base));
        let (set_since, since) = (Rc::clone(&idle_since), Rc::clone(&idle_since));
        let after = Rc::clone(&base);
        vec![
            step("the first frame is presented", |shell, app| {
                settled(shell, app.window(0), 0)
            }),
            step("note the counts", move |shell, app| {
                set.set(counts(shell, app.window(0)));
                set_since.set(Instant::now());
                true
            }),
            step("idle for 500 ms", move |shell, app| {
                assert_eq!(counts(shell, app.window(0)), idle.get(), "an idle window drew");
                since.get().elapsed() >= Duration::from_millis(500)
            }),
            step("damage once", |shell, app| {
                shell.damage(app.window(0)).expect("damaged");
                true
            }),
            step("one damage, one frame, one submission", move |shell, app| {
                let (submitted, presented, due) = counts(shell, app.window(0));
                let (base_submitted, base_presented, base_due) = after.get();
                if presented == base_presented {
                    return false;
                }
                assert_eq!(presented, base_presented + 1);
                assert_eq!(submitted, base_submitted + 1, "one frame is one submission");
                assert_eq!(due, base_due + 1);
                true
            }),
            step("capture the next presented frame", |shell, app| {
                shell.capture_next(app.window(0)).expect("capture asked for");
                true
            }),
            step("the presented frame reads back texel for texel", |shell, app| {
                match shell.take_capture(app.window(0)) {
                    None => false,
                    Some(Ok(readback)) => {
                        CAPTURED.store(true, Ordering::Release);
                        let comparison = readback.compare(&FRAME).expect("compared");
                        assert!(comparison.matches(), "{comparison:?}");
                        let info = shell.presentation(app.window(0)).unwrap().swapchain.unwrap();
                        assert_eq!((readback.extent, readback.format), (info.extent, info.format));
                        println!(
                            "HSK_SHELL_CAPTURE format={} extent={}x{} mode={} images={}",
                            info.format.key(),
                            info.extent.width,
                            info.extent.height,
                            info.mode.key(),
                            info.images
                        );
                        true
                    }
                    Some(Err(ReadbackError::NotReadable)) => {
                        println!("HSK_SHELL_SKIP capture=surface_without_transfer_src");
                        true
                    }
                    Some(Err(error)) => panic!("capture: {}", error.code()),
                }
            }),
            step("the watch saw the wanted frame presented", |shell, app| {
                let reading = shell.watch(app.window(0)).expect("a watch").read();
                assert!(!reading.never_presents(Duration::ZERO), "{reading:?}");
                assert!(reading.present.expect("a surface").presented >= 3);
                true
            }),
        ]
    }) else {
        return;
    };
    assert_eq!(ended.report.surfaces_made, 1);
    assert_eq!(ended.report.targets_released, 1, "{:?}", ended.report);
    assert_eq!(ended.report.captures, u64::from(CAPTURED.load(Ordering::Acquire)));
    assert_eq!(ended.report.errors, 0, "{:?}", ended.report.last_error);
    assert!(ended.report.shutdown.is_some());
}

#[test]
fn resize_minimise_restore_and_scale_change_recreate_the_swapchain() {
    in_own_process!();
    let Some(ended) = present_loop(1, || {
        let number = Rc::new(Cell::new(0));
        let presented = Rc::new(Cell::new(0));
        let mark = {
            let (number, presented) = (Rc::clone(&number), Rc::clone(&presented));
            move |shell: &Shell<'_>, window| {
                number.set(swapchain_number(shell, window));
                presented.set(counts(shell, window).1);
            }
        };
        let replaced = {
            let (number, presented) = (Rc::clone(&number), Rc::clone(&presented));
            move |shell: &Shell<'_>, window| {
                let state = shell.control().window_state(window).expect("the window");
                let info = shell.presentation(window).and_then(|presentation| presentation.swapchain);
                info.is_some_and(|info| {
                    info.swapchain > number.get()
                        && (info.extent.width, info.extent.height)
                            == (state.inner.width, state.inner.height)
                }) && settled(shell, window, presented.get())
            }
        };
        let one_swapchain = |shell: &Shell<'_>| {
            shell.owner().memory_use().is_ok_and(|usage| usage.swapchains == 1)
        };
        let (mark_1, mark_2, mark_3) = (mark.clone(), mark.clone(), mark);
        let (replaced_1, replaced_2, replaced_3) = (replaced.clone(), replaced.clone(), replaced);
        let quiet_since = Rc::new(Cell::new((Instant::now(), 0)));
        let (note_quiet, check_quiet) = (Rc::clone(&quiet_since), quiet_since);
        let dpi = Rc::new(Cell::new(0));
        let (send_dpi, check_dpi) = (Rc::clone(&dpi), dpi);
        vec![
            step("the first frame is presented", |shell, app| {
                settled(shell, app.window(0), 0)
            }),
            step("resize", move |shell, app| {
                mark_1(shell, app.window(0));
                command(shell, app.window(0), WindowCommand::SetInnerSize(LogicalSize::new(200.0, 120.0)));
                true
            }),
            step("a new swapchain at the new size is presented", move |shell, app| {
                replaced_1(shell, app.window(0))
            }),
            step("the old swapchain ends", move |shell, _| one_swapchain(shell)),
            step("minimise", move |shell, app| {
                mark_2(shell, app.window(0));
                command(shell, app.window(0), WindowCommand::Minimize);
                true
            }),
            step("suspended", move |shell, app| {
                let presentation = shell.presentation(app.window(0)).expect("presents");
                let suspended = presentation.suspended && presentation.swapchain.is_none();
                if suspended {
                    note_quiet.set((Instant::now(), shell.report().frames_due));
                }
                suspended
            }),
            step("nothing is drawn while minimised", move |shell, app| {
                let (since, due) = check_quiet.get();
                assert_eq!(shell.report().frames_due, due, "a minimised window drew");
                assert_eq!(shell.control().window_state(app.window(0)).unwrap().display, Display::Minimized);
                since.elapsed() >= Duration::from_millis(300)
            }),
            step("restore", move |shell, app| {
                command(shell, app.window(0), WindowCommand::Restore { activate: false });
                true
            }),
            step("restore recreates and presents", move |shell, app| {
                replaced_2(shell, app.window(0))
            }),
            step("change the scale", move |shell, app| {
                mark_3(shell, app.window(0));
                let now = shell.control().window_state(app.window(0)).unwrap().scale.dpi();
                let to = if now == 144 { 120 } else { 144 };
                send_dpi.set(to);
                send_scale(app.hwnd(0), to);
                true
            }),
            step("the new scale is presented at its size", move |shell, app| {
                let scale = shell.control().window_state(app.window(0)).unwrap().scale.dpi();
                scale == check_dpi.get() && replaced_3(shell, app.window(0))
            }),
            step("every replaced swapchain ends", move |shell, _| one_swapchain(shell)),
        ]
    }) else {
        return;
    };
    let report = ended.report;
    assert!(report.swapchains_replaced >= 3, "{report:?}");
    assert!(report.suspends >= 1, "{report:?}");
    assert_eq!(report.targets_released, 1, "{report:?}");
    assert_eq!(report.errors, 0, "{:?}", report.last_error);
    if !ended.present_fences {
        assert!(report.retire_frames > 0, "old swapchains were ended by later presents: {report:?}");
    }
}

/// The operating system's message for a scale change, to a window of this thread (as the window crate's own test).
fn send_scale(hwnd: HWND, dpi: u32) {
    let mut area = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: the rectangle outlives both calls; the message is the system's, to a window of this thread.
    unsafe {
        GetWindowRect(hwnd, &mut area);
        let both = ((dpi as usize) << 16) | dpi as usize;
        SendMessageW(hwnd, WM_DPICHANGED, both, &area as *const RECT as LPARAM);
    }
}

#[test]
fn closing_a_window_releases_its_target_once_and_keeps_the_other_presenting() {
    in_own_process!();
    let Some(ended) = present_loop(2, || {
        let other_presented = Rc::new(Cell::new(0));
        let (note, check) = (Rc::clone(&other_presented), other_presented);
        vec![
            step("both windows present", |shell, app| {
                settled(shell, app.window(0), 0) && settled(shell, app.window(1), 0)
            }),
            step("close the first", move |shell, app| {
                assert_eq!(shell.report().targets_released, 0);
                note.set(counts(shell, app.window(1)).1);
                shell.close_window(app.window(0)).expect("closing");
                let hidden = shell
                    .control()
                    .window_state(app.window(0))
                    .is_none_or(|state| state.display == Display::Hidden);
                assert!(hidden, "a closed window is hidden at once");
                true
            }),
            step("damage the other", |shell, app| {
                shell.damage(app.window(1)).expect("damaged");
                true
            }),
            step("the other still presents", move |shell, app| {
                settled(shell, app.window(1), check.get())
            }),
            step("the first lets its window go, once", |shell, app| {
                let first = app.window(0);
                if support::has_present_fences(shell.owner()) {
                    // Its swapchain's present fences prove its end: the owner drops the target, the window goes.
                    app.destroyed(first) && shell.report().targets_released == 1
                } else {
                    // Nothing proves its last swapchain's presents: kept, with its target, until the owner closes.
                    assert_eq!(shell.report().targets_released, 0);
                    let state = shell.control().window_state(first).expect("kept");
                    assert_eq!(state.native_tokens, 1);
                    assert!(shell.presentation(first).expect("closing").closing);
                    true
                }
            }),
        ]
    }) else {
        return;
    };
    let report = ended.report;
    assert_eq!(report.surfaces_made, 2);
    assert_eq!(report.targets_released, 2, "each target once: {report:?}");
    assert_eq!(ended.run.windows_destroyed, 2);
    assert_eq!(report.errors, 0, "{:?}", report.last_error);
}
