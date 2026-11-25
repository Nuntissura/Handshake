//! A panic of the host ends the loop, leaves no window behind, and goes on to whoever called `run`.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::time::Duration;

use handshake_shell_window::{
    CommandSource, Event, LogicalSize, LoopConfig, Placement, WindowClass, WindowEvent, WindowSpec,
};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindow};

use support::{on_private_desktop, run_script, wide};

#[test]
fn a_panicking_host_leaves_nothing_behind() {
    on_private_desktop(Duration::from_secs(60), || {
        let hwnd: Rc<Cell<HWND>> = Rc::default();
        let calls_after: Rc<Cell<u32>> = Rc::default();
        let (seen, after) = (Rc::clone(&hwnd), Rc::clone(&calls_after));
        let panicked = Cell::new(false);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_script(
                LoopConfig::default(),
                move |control, event| {
                    if panicked.get() {
                        after.set(after.get() + 1);
                        return;
                    }
                    match event {
                        Event::LoopStarted => {
                            let spec = WindowSpec {
                                title: "test".to_owned(),
                                inner: LogicalSize::new(200.0, 100.0),
                                min_inner: None,
                                placement: Placement::OutsideAllMonitors,
                                class: WindowClass::Quiet,
                            };
                            control
                                .create_window(&CommandSource::host(1), spec)
                                .unwrap();
                        }
                        Event::Window {
                            window,
                            event: WindowEvent::Created { .. },
                        } => {
                            seen.set(control.native_window(window).unwrap().win32().hwnd.get());
                            panicked.set(true);
                            panic!("the host's own panic");
                        }
                        _ => {}
                    }
                },
                |_, _| {},
            )
        }));
        let payload = outcome.expect_err("the panic reaches the caller");
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"the host's own panic")
        );
        assert_eq!(
            calls_after.get(),
            0,
            "a host that panicked is not called again"
        );
        assert_ne!(hwnd.get(), 0);
        // SAFETY: plain values and a name that outlives the call, read only.
        unsafe {
            assert_eq!(IsWindow(hwnd.get()), 0, "its window is gone");
            let service = wide("hsk_shell_service");
            assert_eq!(
                FindWindowW(service.as_ptr(), std::ptr::null()),
                0,
                "and so is the loop's own"
            );
        }
    });
}
