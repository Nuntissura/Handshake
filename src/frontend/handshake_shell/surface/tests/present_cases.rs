//! The GPU owner's presentation cases (`handshake_native/vulkan-renderer/tests/support/present_cases.rs`), hosted with
//! real windows of the window crate: quiet, shown at the bottom of the Z order of a private desktop that receives no
//! input, each case in a process of its own (one loop and one owner per process). The cases use the owner's public
//! surface only; this file supplies their `HostWindow` and runs each inside the loop, on the windows' own thread.
//!
//! What each guards:
//! - `frames_are_presented_and_counted`: acquire, record, present; the probe counts what the queue took (AC214-4 report).
//! - `a_resize_ends_the_old_swapchain_once`: a resize recreates the swapchain, and the old one ends once, by its own
//!   surface's proof (AC214-3, MT-203 S2-F2).
//! - `minimise_suspends_and_restore_resumes`: no swapchain of no size; restore recreates (AC214-3).
//! - `a_dropped_frame_needs_a_new_configuration`: a frame dropped unpresented leaves the swapchain out of date.
//! - `a_surface_proven_before_release_lets_its_window_go`: one present on a new swapchain whose image comes back ends
//!   both swapchains, and the window's target is dropped before the close (MT-214 SF-2).
//! - `a_released_surface_lets_its_window_go`: the window's target is dropped once, after the surface ended.
//! - `another_windows_presents_never_end_this_ones_swapchain`: retirement stays per surface (AC214-8, S2-F2).
//! - `a_blocked_present_does_not_stall_another_windows_acquire`: a FIFO present that waits for the vertical blank holds
//!   its own surface only (AC214-8, S2-F5).
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::thread::ThreadId;
use std::time::Duration;

use handshake_shell_surface::ShellWindowTarget;
use handshake_shell_window::{
    CommandSource, Control, Event, FrameOutcome, LogicalSize, LoopConfig, NativeWindowToken,
    PhysicalPosition, PhysicalSize, Placement, WindowClass, WindowCommand, WindowEvent, WindowKey,
    WindowSpec,
};
use handshake_vulkan_renderer::{ClientId, Extent, GpuClient, GpuOwner, ModuleId, SurfaceTarget};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetClientRect, PeekMessageW, PostQuitMessage, TranslateMessage, MSG,
    PM_REMOVE, WM_QUIT,
};

use support::desktop::{must_stay_inactive, on_private_desktop, run_script, stop_watching};
use support::gpu_env::{in_own_process, ran_in_its_own_process};
use support::present_cases::{self as cases, HostWindow};

/// Inside the owner's 120 s per test process.
const LIMIT: Duration = Duration::from_secs(100);

thread_local! {
    /// The loop, while a case runs on its thread.
    static CONTROL: RefCell<Option<Control>> = const { RefCell::new(None) };
}

/// A shown quiet window, driven by a case.
struct CaseWindow {
    key: WindowKey,
    token: NativeWindowToken,
    thread: ThreadId,
}

impl CaseWindow {
    fn hwnd(&self) -> HWND {
        self.token.win32().hwnd.get()
    }

    fn command(&self, command: WindowCommand) {
        CONTROL.with(|control| {
            let control = control.borrow();
            let control = control.as_ref().expect("on the loop's thread, while a case runs");
            control
                .command(&CommandSource::host(90), self.key, command)
                .expect("the window takes the command");
        });
    }

    fn scale(&self) -> handshake_shell_window::ScaleFactor {
        CONTROL.with(|control| {
            control
                .borrow()
                .as_ref()
                .and_then(|control| control.window_state(self.key))
                .expect("the window exists")
                .scale
        })
    }

    /// Run the window's messages until its client size is what `done` wants: what the driver reports as the surface's
    /// extent on Win32.
    fn pump_until(&self, what: &str, done: impl Fn(Extent) -> bool) {
        for _ in 0..2_000 {
            self.pump();
            if done(self.size()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("the window did not {what}: {:?}", self.size());
    }
}

impl HostWindow for CaseWindow {
    fn target(&self) -> Box<dyn SurfaceTarget> {
        Box::new(ShellWindowTarget::new(self.token.clone()))
    }

    fn size(&self) -> Extent {
        let mut area = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: the window exists while its token does; the rectangle outlives the call.
        unsafe { GetClientRect(self.hwnd(), &mut area) };
        Extent {
            width: u32::try_from(area.right - area.left).unwrap_or(0),
            height: u32::try_from(area.bottom - area.top).unwrap_or(0),
        }
    }

    fn resize(&self, to: Extent) {
        let scale = self.scale();
        let logical = PhysicalSize::new(to.width, to.height).to_logical(scale);
        self.command(WindowCommand::SetInnerSize(logical));
        let wanted = logical.to_physical(scale);
        self.pump_until("take its new size", |now| {
            (now.width, now.height) == (wanted.width, wanted.height)
        });
    }

    fn minimise(&self) {
        self.command(WindowCommand::Minimize);
        self.pump_until("minimise", |now| now.width == 0 && now.height == 0);
    }

    fn restore(&self) {
        self.command(WindowCommand::Restore { activate: false });
        self.pump_until("restore", |now| now.width > 0 && now.height > 0);
    }

    fn pump(&self) {
        if std::thread::current().id() != self.thread {
            return;
        }
        // SAFETY: a structure of this function; messages of this thread's windows, dispatched on it. The window crate
        // takes a nested loop of the host's (its `test_wakes` runs one).
        unsafe {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, 0, 0, 0, PM_REMOVE) != 0 {
                if message.message == WM_QUIT {
                    PostQuitMessage(message.wParam as i32);
                    break;
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

type Case = Box<dyn FnOnce(&GpuOwner, &GpuClient, &[CaseWindow]) + Send>;

struct Run {
    owner: GpuOwner,
    case: RefCell<Option<Case>>,
    wanted: usize,
    windows: RefCell<Vec<CaseWindow>>,
    hwnds: RefCell<BTreeMap<WindowKey, HWND>>,
}

impl Run {
    fn event(&self, control: &Control, event: Event) {
        match event {
            Event::LoopStarted => {
                for number in 0..self.wanted {
                    let at = 30 + 40 * number as i32;
                    let spec = WindowSpec {
                        title: "present case".to_owned(),
                        inner: LogicalSize::new(320.0, 240.0),
                        min_inner: None,
                        placement: Placement::At(PhysicalPosition::new(at, at)),
                        class: WindowClass::Quiet,
                    };
                    control
                        .create_window(&CommandSource::host(number as u64 + 1), spec)
                        .expect("a window");
                }
            }
            Event::Window { window, event } => match event {
                WindowEvent::Created { .. } => {
                    let token = control.native_window(window).expect("a token");
                    let hwnd = token.win32().hwnd.get();
                    must_stay_inactive(hwnd);
                    self.hwnds.borrow_mut().insert(window, hwnd);
                    control
                        .command(
                            &CommandSource::host(50),
                            window,
                            WindowCommand::Show { activate: false },
                        )
                        .expect("shown");
                    self.windows.borrow_mut().push(CaseWindow {
                        key: window,
                        token,
                        thread: std::thread::current().id(),
                    });
                    if self.windows.borrow().len() == self.wanted {
                        self.run_case(control);
                    }
                }
                WindowEvent::FrameDue { ticket, .. } => {
                    let _ = control.frame_done(window, ticket, FrameOutcome::Unchanged);
                }
                WindowEvent::Destroyed => {
                    if let Some(hwnd) = self.hwnds.borrow_mut().remove(&window) {
                        stop_watching(hwnd);
                    }
                }
                _ => {}
            },
            Event::ShutdownBegan => control.finish(0).expect("no surface holds a window any more"),
            _ => {}
        }
    }

    /// The case, inside this event handler: it runs the windows' messages itself.
    fn run_case(&self, control: &Control) {
        CONTROL.with(|slot| *slot.borrow_mut() = Some(control.clone()));
        let client = self
            .owner
            .attach_client(ModuleId(214), ClientId(1))
            .expect("a client");
        let windows = std::mem::take(&mut *self.windows.borrow_mut());
        let case = self.case.borrow_mut().take().expect("one case");
        case(&self.owner, &client, &windows);
        drop(client);
        // Swapchains that nothing proves ended go with the device, and with them the windows' targets.
        let _ = self.owner.close();
        drop(windows);
        CONTROL.with(|slot| slot.borrow_mut().take());
        control.begin_shutdown().expect("shutdown begins");
    }
}

/// Run `case` with `windows` shown quiet windows, on a private desktop, inside the loop.
fn run_case(
    windows: usize,
    case: impl FnOnce(&GpuOwner, &GpuClient, &[CaseWindow]) + Send + 'static,
) {
    on_private_desktop(LIMIT, move || {
        let Some((owner, _log)) = support::open_owner(true) else {
            return;
        };
        let run = Rc::new(Run {
            owner,
            case: RefCell::new(Some(Box::new(case))),
            wanted: windows,
            windows: RefCell::default(),
            hwnds: RefCell::default(),
        });
        let events = Rc::clone(&run);
        let report = run_script(
            LoopConfig::default(),
            move |control, event| events.event(control, event),
            |_, _| {},
        )
        .expect("the loop ran");
        assert!(run.case.borrow().is_none(), "the case ran");
        assert_eq!(report.windows_created, windows as u64);
        assert_eq!(report.windows_destroyed, windows as u64);
        assert_eq!(report.quiet_broken, 0);
    });
}

#[test]
fn frames_are_presented_and_counted() {
    in_own_process!();
    run_case(1, |_, client, windows| {
        cases::frames_are_presented_and_counted(client, &windows[0])
    });
}

#[test]
fn a_resize_ends_the_old_swapchain_once() {
    in_own_process!();
    run_case(1, |owner, client, windows| {
        cases::a_resize_ends_the_old_swapchain_once(owner, client, &windows[0])
    });
}

#[test]
fn minimise_suspends_and_restore_resumes() {
    in_own_process!();
    run_case(1, |_, client, windows| {
        cases::minimise_suspends_and_restore_resumes(client, &windows[0])
    });
}

#[test]
fn a_dropped_frame_needs_a_new_configuration() {
    in_own_process!();
    run_case(1, |_, client, windows| {
        cases::a_dropped_frame_needs_a_new_configuration(client, &windows[0])
    });
}

#[test]
fn a_surface_proven_before_release_lets_its_window_go() {
    in_own_process!();
    run_case(1, |owner, client, windows| {
        cases::a_surface_proven_before_release_lets_its_window_go(owner, client, &windows[0])
    });
}

#[test]
fn a_released_surface_lets_its_window_go() {
    in_own_process!();
    run_case(1, |owner, client, windows| {
        cases::a_released_surface_lets_its_window_go(owner, client, &windows[0])
    });
}

#[test]
fn another_windows_presents_never_end_this_ones_swapchain() {
    in_own_process!();
    run_case(2, |owner, client, windows| {
        cases::another_windows_presents_never_end_this_ones_swapchain(
            owner,
            client,
            &windows[0],
            &windows[1],
        )
    });
}

#[test]
fn a_blocked_present_does_not_stall_another_windows_acquire() {
    in_own_process!();
    run_case(2, |_, client, windows| {
        cases::a_blocked_present_does_not_stall_another_windows_acquire(
            client,
            &windows[0],
            &windows[1],
        )
    });
}
