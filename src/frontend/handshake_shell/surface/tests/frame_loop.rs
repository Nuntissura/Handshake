//! The presenter on a real loop with real windows: quiet, shown at the bottom of the Z order of a private desktop that
//! receives no input, each test in a process of its own. A test is a list of steps a wake of the application runs one
//! after the other; a step that does not finish in time fails the test with its name.
//!
//! - `an_idle_window_submits_nothing_and_one_damage_presents_one_frame`: AC214-4, also while input, focus and move
//!   messages reach the window on the real loop (MT-214 SF-1), and the capture of a presented swapchain image compared
//!   texel by texel; the watch of MT-215 HB-4 says the wanted frame was presented. Posted focus messages make no focus
//!   event on the loop, so the stir is complete once keys, pointer and the move arrived (E5-F3).
//! - `resize_minimise_restore_and_scale_change_recreate_the_swapchain`: AC214-3; every replaced swapchain ends.
//! - `closing_a_window_releases_its_target_once_and_keeps_the_other_presenting`: AC214-3, AC214-8; the closed window
//!   is destroyed before shutdown, with or without `VK_EXT_swapchain_maintenance1` (MT-214 SF-2), and is never hidden
//!   while its presents are still being proven (E5-F2).
//! - `windows_opened_and_closed_one_after_another_leave_no_swapchain_behind`: MT-214 SF-2, E5-F2.
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
    CommandSource, DispatchedWake, Display, Event, FrameCause, LogicalSize, LoopConfig,
    PhysicalPosition, Placement, RunReport, ShellHost, WakeLane, WindowClass, WindowCommand,
    WindowEvent, WindowKey, WindowSpec,
};
use handshake_vulkan_renderer::ModuleId;
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, PostMessageW, SendMessageW, WM_DPICHANGED, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS,
    WM_MOUSEMOVE, WM_SETFOCUS,
};

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

/// Events of the windows that change nothing shown, and frames due by their cause, as the application heard them.
/// Focus events are not counted: whatever posted focus messages make is held to drawing nothing like every other event,
/// but on the loop they make none (E5-F3), so nothing waits for one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Seen {
    keys: u64,
    pointer: u64,
    moved: u64,
    requested: u64,
    exposed: u64,
}

impl Seen {
    fn note(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Key(_) | WindowEvent::Text(_) | WindowEvent::ModifiersChanged { .. } => {
                self.keys += 1
            }
            WindowEvent::PointerMoved(_)
            | WindowEvent::PointerEntered(_)
            | WindowEvent::PointerLeft(_)
            | WindowEvent::PointerButton(_)
            | WindowEvent::Wheel(_) => self.pointer += 1,
            WindowEvent::Moved { .. } => self.moved += 1,
            WindowEvent::FrameDue {
                cause: FrameCause::Requested,
                ..
            } => self.requested += 1,
            WindowEvent::FrameDue {
                cause: FrameCause::Exposed,
                ..
            } => self.exposed += 1,
            _ => {}
        }
    }

    /// Keys, the pointer and a move all reached the application since `before`. Not focus: posted `WM_SETFOCUS` and
    /// `WM_KILLFOCUS` made no focus event in 965 checks of the evaluation of 29763b430 (MT-214 E5-F3).
    fn stirred_since(self, before: Self) -> bool {
        self.keys > before.keys && self.pointer > before.pointer && self.moved > before.moved
    }
}

/// A quiet window at `at`.
fn quiet_window(at: i32) -> WindowSpec {
    WindowSpec {
        title: "frame loop".to_owned(),
        inner: LogicalSize::new(240.0, 160.0),
        min_inner: None,
        placement: Placement::At(PhysicalPosition::new(at, at)),
        class: WindowClass::Quiet,
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
    seen: Cell<Seen>,
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
                    shell
                        .open_window(&CommandSource::host(number as u64 + 1), quiet_window(at))
                        .expect("a window");
                }
            }
            Event::Window { window, event } => {
                let mut seen = self.seen.get();
                seen.note(event);
                self.seen.set(seen);
                self.window_event(shell, window, event);
            }
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

impl App {
    fn window_event(&self, shell: &Shell<'_>, window: &WindowKey, event: &WindowEvent) {
        match event {
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
        }
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
            seen: Cell::default(),
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

/// MT-214 E5-F2: a closing window is hidden only once its surface was released, after the proof of its presents
/// ended. Every acquire of a hidden window's swapchain is out of date, so a proof begun before the hide never comes.
///
/// E6-F6: that same out-of-date acquire ends the proof and drops the swapchain at once, so the swapchain cannot show a
/// hide that came first (the evaluation's S-M1 hid before the proof and this stayed silent). What does not depend on it:
/// a hidden window whose proof ended unproven since its close began (`unproven_before`, the presenter's
/// `closes_unproven` then). A proof that runs out of its time ends unproven too; these tests allow none.
fn not_hidden_while_proving(shell: &Shell<'_>, window: WindowKey, unproven_before: u64) {
    let hidden = shell
        .control()
        .window_state(window)
        .is_some_and(|state| state.display == Display::Hidden);
    let report = shell.report();
    let proving = shell
        .presentation(window)
        .is_some_and(|presentation| presentation.swapchain.is_some());
    assert!(
        !hidden_while_proving(hidden, proving, report.closes_unproven, unproven_before),
        "hidden while its presents were being proven: {report:?}"
    );
}

/// Whether a window was hidden while its presents were being proven: it is hidden and either the swapchain of its proof
/// still stands or a proof ended unproven since its close began. The second does not depend on the swapchain (E6-F6).
fn hidden_while_proving(hidden: bool, proving: bool, unproven: u64, unproven_before: u64) -> bool {
    hidden && (proving || unproven > unproven_before)
}

/// MT-214 E6-F6: the check holds when the swapchain is already gone, as it is once the out-of-date acquire of a hidden
/// window ended the proof (the evaluation's mutation S-M1). Fails if it depends on the swapchain again.
#[test]
fn a_hide_before_the_proof_is_caught_without_the_swapchain() {
    assert!(hidden_while_proving(true, false, 1, 0), "cut off, swapchain gone");
    assert!(hidden_while_proving(true, true, 0, 0), "still proving");
    assert!(!hidden_while_proving(true, false, 1, 1), "an earlier one is not this close's");
    assert!(!hidden_while_proving(false, true, 2, 0), "not hidden");
}

/// Input, focus and move messages for the window, as the operating system posts them to it, and a move by the host:
/// all reach the presenter as events that change nothing shown (AC214-4, MT-214 SF-1). Posted to a window of this
/// thread on the private desktop; nothing of the system's input, focus or cursor changes.
fn stir(shell: &Shell<'_>, window: WindowKey, hwnd: HWND) {
    let post = |message: u32, wparam: WPARAM, lparam: LPARAM| {
        // SAFETY: plain values, to a window of this thread.
        assert_ne!(unsafe { PostMessageW(hwnd, message, wparam, lparam) }, 0);
    };
    post(WM_MOUSEMOVE, 0, (20 << 16) | 30);
    post(WM_MOUSEMOVE, 0, (24 << 16) | 34);
    post(WM_KEYDOWN, 0x41, 0x001E_0001);
    post(WM_KEYUP, 0x41, 0xC01E_0001_u32 as LPARAM);
    post(WM_SETFOCUS, 0, 0);
    post(WM_KILLFOCUS, 0, 0);
    let at = shell.control().window_state(window).expect("the window").outer.origin;
    command(shell, window, WindowCommand::MoveTo(PhysicalPosition::new(at.x + 12, at.y + 12)));
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
        let (stir_base, stirred_base) = (Rc::clone(&base), Rc::clone(&base));
        let (stir_since, stirred_since) = (Rc::clone(&idle_since), Rc::clone(&idle_since));
        let seen_before = Rc::new(Cell::new(Seen::default()));
        let (note_seen, check_seen) = (Rc::clone(&seen_before), seen_before);
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
            step("stir the window: input, focus and a move", move |shell, app| {
                stir_base.set(counts(shell, app.window(0)));
                note_seen.set(app.seen.get());
                stir(shell, app.window(0), app.hwnd(0));
                stir_since.set(Instant::now());
                true
            }),
            step("stirred for 500 ms, nothing asked for a frame", move |shell, app| {
                // MT-214 SF-1: what decide.rs says of these events, on the real loop. Only an exposure of the
                // operating system's may draw meanwhile, and none is expected.
                let (seen, before) = (app.seen.get(), check_seen.get());
                assert_eq!(seen.requested, before.requested, "an event asked for a frame: {seen:?}");
                let exposed = seen.exposed - before.exposed;
                let (submitted, ..) = counts(shell, app.window(0));
                assert_eq!(submitted, stirred_base.get().0 + exposed, "an event drew: {seen:?}");
                let done = seen.stirred_since(before)
                    && stirred_since.get().elapsed() >= Duration::from_millis(500);
                if done {
                    stirred_base.set(counts(shell, app.window(0)));
                }
                done
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
        // The presenter's unproven closes before the close (E6-F6).
        let unproven = Rc::new(Cell::new(0));
        let before = [0; 4].map(|_| Rc::clone(&unproven));
        let [at_close, at_damage, at_present, at_end] = before;
        vec![
            step("both windows present", |shell, app| {
                settled(shell, app.window(0), 0) && settled(shell, app.window(1), 0)
            }),
            step("close the first", move |shell, app| {
                assert_eq!(shell.report().targets_released, 0);
                note.set(counts(shell, app.window(1)).1);
                at_close.set(shell.report().closes_unproven);
                shell.close_window(app.window(0)).expect("closing");
                // Hidden at once with present fences; without them once its presents are proven ended (E5-F2).
                not_hidden_while_proving(shell, app.window(0), at_close.get());
                true
            }),
            step("damage the other", move |shell, app| {
                not_hidden_while_proving(shell, app.window(0), at_damage.get());
                shell.damage(app.window(1)).expect("damaged");
                true
            }),
            step("the other still presents", move |shell, app| {
                not_hidden_while_proving(shell, app.window(0), at_present.get());
                settled(shell, app.window(1), check.get())
            }),
            step("the first lets its window go, once, before shutdown", move |shell, app| {
                // Its presents are proven ended, by its present fences or, without them, by one present on a new
                // swapchain whose image came back while it was still shown (MT-214 SF-2, E5-F2): the owner drops the
                // target, the window goes.
                not_hidden_while_proving(shell, app.window(0), at_end.get());
                let report = shell.report();
                assert_eq!(report.closes_unproven, 0, "{report:?}");
                assert!(report.targets_released <= 1, "once: {report:?}");
                app.destroyed(app.window(0)) && report.targets_released == 1
            }),
            step("only the other's swapchain is left", |shell, _| {
                let _ = shell.owner().poll();
                shell.owner().memory_use().is_ok_and(|usage| usage.swapchains == 1)
            }),
        ]
    }) else {
        return;
    };
    let report = ended.report;
    assert_eq!(report.surfaces_made, 2);
    assert_eq!(report.targets_released, 2, "each target once: {report:?}");
    assert_eq!(report.closes_unproven, 0, "{report:?}");
    assert_eq!(ended.run.windows_destroyed, 2);
    assert_eq!(report.errors, 0, "{:?}", report.last_error);
}

/// MT-214 SF-2: windows opened and closed one after another, with a window that stays: each closed window's
/// swapchains end and its window is destroyed before the next opens, so what exists does not grow with the windows
/// closed, with `VK_EXT_swapchain_maintenance1` or without it.
#[test]
fn windows_opened_and_closed_one_after_another_leave_no_swapchain_behind() {
    in_own_process!();
    const CYCLES: usize = 6;
    let Some(ended) = present_loop(1, || {
        let swapchains = |shell: &Shell<'_>| {
            let _ = shell.owner().poll();
            shell.owner().memory_use().map_or(u64::MAX, |usage| usage.swapchains)
        };
        let mut steps = vec![step("the window that stays presents", |shell, app| {
            settled(shell, app.window(0), 0)
        })];
        for cycle in 0..CYCLES {
            let number = cycle + 1;
            // The presenter's unproven closes before this window's close (E6-F6).
            let at_close = Rc::new(Cell::new(0));
            let at_gone = Rc::clone(&at_close);
            steps.extend([
                step("open one more", move |shell, _| {
                    let at = 100 + 20 * cycle as i32;
                    shell
                        .open_window(&CommandSource::host(200 + cycle as u64), quiet_window(at))
                        .expect("a window");
                    true
                }),
                step("it presents", move |shell, app| {
                    app.windows.borrow().len() > number && settled(shell, app.window(number), 0)
                }),
                step("close it", move |shell, app| {
                    at_close.set(shell.report().closes_unproven);
                    shell.close_window(app.window(number)).expect("closing");
                    not_hidden_while_proving(shell, app.window(number), at_close.get());
                    true
                }),
                step("it is gone, with its swapchains and target", move |shell, app| {
                    not_hidden_while_proving(shell, app.window(number), at_gone.get());
                    let now = swapchains(shell);
                    // The one that stays, and the closing window's old swapchain and the new one of its proof.
                    assert!(now <= 3, "{now} swapchains while window {number} closes");
                    let report = shell.report();
                    assert_eq!(report.closes_unproven, 0, "{report:?}");
                    app.destroyed(app.window(number))
                        && report.targets_released == number as u64
                        && now == 1
                }),
            ]);
        }
        steps
    }) else {
        return;
    };
    let report = ended.report;
    assert_eq!(report.surfaces_made, CYCLES as u64 + 1);
    assert_eq!(report.targets_released, CYCLES as u64 + 1, "each target once: {report:?}");
    assert_eq!(report.closes_unproven, 0, "{report:?}");
    assert_eq!(ended.run.windows_destroyed, CYCLES as u64 + 1);
    assert_eq!(report.errors, 0, "{:?}", report.last_error);
}
