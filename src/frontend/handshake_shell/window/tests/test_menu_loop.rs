//! Wakes inside the operating system's menu loop. A process has one loop, so this is a target of its own.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use std::time::Duration;

use handshake_shell_window::{Event, LoopConfig, WakeLane};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, EndMenu, TrackPopupMenuEx, MF_STRING, TPM_NONOTIFY,
    TPM_RETURNCMD,
};

use support::{on_private_desktop, run_script, wide, Decoy};

const TICK: Duration = Duration::from_millis(10);

#[derive(Default)]
struct State {
    lane: RefCell<Option<WakeLane>>,
    menu_open: Cell<bool>,
    inside_menu: Cell<u32>,
}

impl State {
    fn tick(&self) {
        let lane = self.lane.borrow();
        lane.as_ref().unwrap().request(1, TICK).unwrap();
    }
}

#[test]
fn a_menu_the_host_opens_does_not_stop_wakes() {
    on_private_desktop(Duration::from_secs(60), || {
        let state = Rc::new(State::default());
        let (events, wakes) = (Rc::clone(&state), Rc::clone(&state));
        let report = run_script(
            LoopConfig::default(),
            move |control, event| match event {
                Event::LoopStarted => {
                    *events.lane.borrow_mut() = Some(control.open_wake_lane().unwrap());
                    let owner = Decoy::create();
                    let item = wide("item");
                    events.menu_open.set(true);
                    events.tick();
                    // SAFETY: a menu made and destroyed here, for a window of this thread, on the private desktop.
                    unsafe {
                        let menu = CreatePopupMenu();
                        assert_ne!(AppendMenuW(menu, MF_STRING, 1, item.as_ptr()), 0);
                        TrackPopupMenuEx(
                            menu,
                            TPM_RETURNCMD | TPM_NONOTIFY,
                            20,
                            20,
                            owner.0,
                            ptr::null(),
                        );
                        DestroyMenu(menu);
                    }
                    events.menu_open.set(false);
                    control.begin_shutdown().unwrap();
                }
                Event::ShutdownBegan => control.finish(0).unwrap(),
                _ => {}
            },
            move |_, wake| {
                if !wakes.menu_open.get() {
                    return;
                }
                assert!(wake.inside_host_call());
                wakes.inside_menu.set(wakes.inside_menu.get() + 1);
                if wakes.inside_menu.get() < 3 {
                    wakes.tick();
                } else {
                    // SAFETY: ends the menu of this thread.
                    unsafe { EndMenu() };
                }
            },
        )
        .unwrap();
        assert_eq!(
            state.inside_menu.get(),
            3,
            "the menu loop dispatched every wake"
        );
        assert!(report.wakes >= 3);
    });
}
