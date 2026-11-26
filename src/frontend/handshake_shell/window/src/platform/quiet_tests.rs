//! The activation guard and the placement against real windows, without the event loop, on a desktop of the test's own
//! that receives no input. Test only: `tests/test_focus_ban.rs` lets this one file show and order windows directly.
//!
//! The thread's active window is the foreground on such a desktop (`place::foreground`), so the paths that need a
//! foreground window run here: behind the foreground, owner chains, a topmost foreground.

use std::cell::Cell;
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{BOOL, HWND};
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, CreateDesktopW, GetThreadDesktop, GetUserObjectInformationW, SetThreadDesktop,
    UOI_IO,
};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetActiveWindow, VK_F4};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindow, GetWindowLongPtrW, PeekMessageW,
    PostMessageW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GW_HWNDNEXT, HWND_TOP, MSG, PM_REMOVE,
    SC_CLOSE, SC_MINIMIZE, SHOW_WINDOW_CMD, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWMAXIMIZED, SW_SHOWMINNOACTIVE,
    SW_SHOWNOACTIVATE, SW_SHOWNORMAL, WM_CLOSE, WM_SYSCOMMAND, WM_SYSKEYDOWN, WS_EX_TOPMOST,
    WS_POPUP,
};

use super::closing::{self, Closing};
use super::guard::{self, ActivationGuard, GuardRefused};
use super::place::{self, Target};
use super::retrieval::Arrival;
use crate::command::WindowClass;
use crate::origin::{Authority, EventTime};
use crate::policy::{self, Foreground, MinimizePlan, ZPlace};

const GENERIC_ALL: u32 = 0x1000_0000;

/// Run `work` on a thread of its own whose windows are on a new desktop that receives no input.
fn on_private_desktop(work: impl FnOnce() + Send + 'static) {
    static NUMBER: AtomicU32 = AtomicU32::new(0);
    let name: Vec<u16> = format!(
        "hsk-shell-unit-{}-{}",
        std::process::id(),
        NUMBER.fetch_add(1, Ordering::Relaxed)
    )
    .encode_utf16()
    .chain(Some(0))
    .collect();
    // SAFETY: the name outlives the call.
    let desktop = unsafe {
        CreateDesktopW(name.as_ptr(), ptr::null(), ptr::null(), 0, GENERIC_ALL, ptr::null())
    };
    assert_ne!(desktop, 0, "no desktop");
    let done = std::thread::spawn(move || {
        // SAFETY: the thread is new and has no window; the desktop outlives it.
        assert_ne!(unsafe { SetThreadDesktop(desktop) }, 0, "not bound");
        let mut receives: BOOL = 1;
        // SAFETY: this thread's desktop, and a result that outlives the call.
        let asked = unsafe {
            GetUserObjectInformationW(
                GetThreadDesktop(GetCurrentThreadId()),
                UOI_IO,
                (&mut receives as *mut BOOL).cast(),
                size_of::<BOOL>() as u32,
                ptr::null_mut(),
            )
        };
        assert!(asked != 0 && receives == 0, "the desktop must not be the Operator's");
        work();
    })
    .join();
    // SAFETY: created here; its thread is gone.
    unsafe { CloseDesktop(desktop) };
    if let Err(panic) = done {
        std::panic::resume_unwind(panic);
    }
}

/// A plain window of this thread, hidden; with an owner it is an owned one.
fn plain(ex_style: u32, owner: HWND) -> HWND {
    let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    // SAFETY: a class of the system's; every pointer outlives the call.
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            ptr::null(),
            WS_POPUP,
            10,
            10,
            80,
            60,
            owner,
            0,
            0,
            ptr::null(),
        )
    };
    assert_ne!(hwnd, 0);
    hwnd
}

fn show(hwnd: HWND, command: SHOW_WINDOW_CMD) {
    // SAFETY: plain values, for a window of this thread.
    unsafe { ShowWindow(hwnd, command) };
}

/// To the top, shown, and activated unless something refuses it.
fn raise(hwnd: HWND) {
    // SAFETY: as in `show`.
    unsafe { SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW) };
}

fn shown_quietly(hwnd: HWND) {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE;
    // SAFETY: as in `show`.
    unsafe { SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, flags) };
}

fn destroy(hwnd: HWND) {
    guard::forget(hwnd);
    // SAFETY: a window of this thread.
    unsafe { DestroyWindow(hwnd) };
}

fn active() -> HWND {
    // SAFETY: no arguments.
    unsafe { GetActiveWindow() }
}

fn next_below(hwnd: HWND) -> HWND {
    // SAFETY: plain values, read only.
    unsafe { GetWindow(hwnd, GW_HWNDNEXT) }
}

fn is_above(upper: HWND, lower: HWND) -> bool {
    let mut below = upper;
    for _ in 0..10_000 {
        below = next_below(below);
        if below == lower {
            return true;
        }
        if below == 0 {
            return false;
        }
    }
    false
}

/// A named sequence of calls, and whether the documentation says it activates a plain window.
type Sequence = (&'static str, fn(HWND), bool);

/// Each sequence the evaluation found to activate a quiet window, and whether the documentation says it activates a
/// plain one (ShowWindow and SetWindowPos on learn.microsoft.com), which the control then shows.
fn sequences() -> [Sequence; 7] {
    [
        ("SetWindowPos without SWP_NOACTIVATE", raise, true),
        (
            "SW_SHOW after SW_HIDE",
            |hwnd| {
                show(hwnd, SW_HIDE);
                show(hwnd, SW_SHOW);
            },
            true,
        ),
        ("SW_SHOWNORMAL", |hwnd| show(hwnd, SW_SHOWNORMAL), true),
        ("SW_SHOWMAXIMIZED", |hwnd| show(hwnd, SW_SHOWMAXIMIZED), true),
        (
            "SW_SHOWMINNOACTIVE, then SW_RESTORE",
            |hwnd| {
                show(hwnd, SW_SHOWMINNOACTIVE);
                show(hwnd, SW_RESTORE);
            },
            true,
        ),
        (
            "SW_SHOWMINNOACTIVE, then SW_SHOWNOACTIVATE",
            |hwnd| {
                show(hwnd, SW_SHOWMINNOACTIVE);
                show(hwnd, SW_SHOWNOACTIVATE);
            },
            false,
        ),
        (
            "SW_MINIMIZE, then SW_SHOW",
            |hwnd| {
                show(hwnd, SW_MINIMIZE);
                show(hwnd, SW_SHOW);
            },
            false,
        ),
    ]
}

/// The plan for an Operator window shown without activation while another window of ours is in front.
fn behind_plan(hwnd: HWND) -> policy::ShowPlan {
    let foreground = place::foreground(hwnd);
    assert_eq!(foreground, Foreground::OurOther);
    let plan = policy::plan_show(WindowClass::Operator, false, Authority::Host, foreground, false);
    let plan = plan.unwrap();
    assert_eq!(plan.place, ZPlace::BehindForeground);
    plan
}

#[test]
fn every_call_that_activates_a_plain_window_leaves_a_quiet_one_inactive() {
    on_private_desktop(|| {
        let _guard = ActivationGuard::install().unwrap();
        assert_eq!(
            ActivationGuard::install().err(),
            Some(GuardRefused::AlreadyInstalled)
        );
        for (name, sequence, documented) in sequences() {
            let control = plain(0, 0);
            sequence(control);
            if documented {
                assert_eq!(active(), control, "{name}: the control is activated");
            }
            destroy(control);
            let quiet = plain(0, 0);
            assert!(guard::mark_quiet(quiet));
            let refused = guard::refused();
            sequence(quiet);
            assert_ne!(active(), quiet, "{name}");
            if documented {
                // The control took the activation along the same path, so the hook was asked and said no.
                assert!(guard::refused() > refused, "{name}: refused by the guard");
            }
            destroy(quiet);
        }
        let quiet = plain(0, 0);
        assert!(guard::mark_quiet(quiet));
        let owned = plain(0, quiet);
        let refused = guard::refused();
        raise(owned);
        assert_ne!(active(), owned, "a window a quiet one owns");
        assert!(guard::refused() > refused, "refused by the guard");
        destroy(owned);
        destroy(quiet);
    });
}

/// MT-214 R2-N5: ACTIVATION_AFTER_REMOVAL against real windows.
#[test]
fn an_approved_removal_hands_the_keyboard_on_only_where_it_went_nowhere() {
    on_private_desktop(|| {
        let _guard = ActivationGuard::install().unwrap();
        let (lower, upper) = (plain(0, 0), plain(0, 0));
        shown_quietly(lower);
        shown_quietly(upper);
        // The Operator closes the window it works in: the keyboard goes to the topmost other one.
        let removed = plain(0, 0);
        raise(removed);
        assert_eq!(active(), removed);
        let before = place::before_removal(removed);
        assert!(place::destroy(removed));
        assert_ne!(active(), upper, "the guard kept the system from handing it on");
        assert!(place::hand_on(&before, true, &[removed, lower, upper]));
        assert_eq!(active(), upper, "the topmost");
        // Without the Operator's approval nothing is handed on.
        let removed = plain(0, 0);
        raise(removed);
        let before = place::before_removal(removed);
        place::hide(removed);
        assert!(!place::hand_on(&before, false, &[lower, upper]));
        assert!(![lower, upper].contains(&active()));
        destroy(removed);
        // A dialog of the hidden window is still shown and keeps the activation.
        let owner = plain(0, 0);
        shown_quietly(owner);
        let dialog = plain(0, owner);
        raise(dialog);
        let before = place::before_removal(owner);
        place::hide(owner);
        assert_eq!(active(), dialog);
        assert!(!place::hand_on(&before, true, &[owner, lower, upper]));
        assert_eq!(active(), dialog);
        // The owner comes before a window that is higher.
        let owned = plain(0, lower);
        raise(owned);
        shown_quietly(upper);
        assert!(is_above(upper, lower) && active() == owned);
        let before = place::before_removal(owned);
        assert!(place::destroy(owned));
        assert!(place::hand_on(&before, true, &[lower, upper]));
        assert_eq!(active(), lower, "its owner");
        for hwnd in [dialog, owner, lower, upper] {
            destroy(hwnd);
        }
    });
}

#[test]
fn inside_still_no_window_is_activated_and_allowing_names_one() {
    on_private_desktop(|| {
        let _guard = ActivationGuard::install().unwrap();
        let (a, b) = (plain(0, 0), plain(0, 0));
        guard::still(|| raise(a));
        assert_ne!(active(), a);
        guard::allowing(a, || {
            raise(a);
            raise(b);
        });
        assert_eq!(active(), a, "the named one only");
        guard::allowing(b, || guard::still(|| raise(b)));
        assert_eq!(active(), a, "a section inside is stricter");
        raise(b);
        assert_eq!(active(), b, "outside every section the guard lets it be");
        destroy(a);
        destroy(b);
    });
}

#[test]
fn a_minimise_hands_the_activation_to_no_window_of_this_thread() {
    on_private_desktop(|| {
        let _guard = ActivationGuard::install().unwrap();
        let next = plain(0, 0);
        raise(next);
        for plan in [MinimizePlan::ActivateNext, MinimizePlan::ActivateNothing] {
            let first = plain(0, 0);
            raise(first);
            assert_eq!(active(), first);
            place::minimize(first, plan);
            assert_ne!(active(), next, "{plan:?}");
            destroy(first);
        }
        // The control: SW_MINIMIZE "activates the next top-level window in the Z order" (ShowWindow).
        raise(next);
        let second = plain(0, 0);
        raise(second);
        show(second, SW_MINIMIZE);
        assert_eq!(active(), next, "the control is handed the activation");
        destroy(second);
        destroy(next);
    });
}

#[test]
fn the_guard_ends_with_its_value_and_can_be_installed_again() {
    on_private_desktop(|| {
        let quiet = plain(0, 0);
        {
            let _guard = ActivationGuard::install().unwrap();
            assert!(guard::mark_quiet(quiet));
            raise(quiet);
            assert_ne!(active(), quiet);
        }
        raise(quiet);
        assert_eq!(active(), quiet, "removed, and the windows it kept are forgotten");
        let _again = ActivationGuard::install().unwrap();
        assert_eq!(guard::refused(), 0, "counted per guard");
        destroy(quiet);
    });
}

#[test]
fn behind_the_foreground_is_below_its_whole_owner_chain() {
    on_private_desktop(|| {
        let _guard = ActivationGuard::install().unwrap();
        let lowest = plain(0, 0);
        shown_quietly(lowest);
        let owner = plain(0, 0);
        shown_quietly(owner);
        let dialog = plain(0, owner);
        raise(dialog);
        assert_eq!(active(), dialog, "the Operator works in the owner's dialog");
        assert_eq!(place::foreground(owner), Foreground::This, "its owner chain");
        assert_eq!(place::foreground(dialog), Foreground::This);
        let order = Cell::new(None);
        let target = Target {
            hwnd: plain(0, 0),
            class: WindowClass::Operator,
            order_during_show: &order,
        };
        place::show(&target, behind_plan(target.hwnd), false).unwrap();
        assert_eq!(
            next_below(owner),
            target.hwnd,
            "right below the owner, not between it and its dialog"
        );
        assert!(is_above(dialog, target.hwnd) && is_above(target.hwnd, lowest));
        assert_eq!(active(), dialog, "shown without activation");
        for hwnd in [target.hwnd, dialog, owner, lowest] {
            destroy(hwnd);
        }
    });
}

#[test]
fn behind_a_topmost_foreground_is_the_bottom() {
    on_private_desktop(|| {
        let _guard = ActivationGuard::install().unwrap();
        let lowest = plain(0, 0);
        shown_quietly(lowest);
        let topmost = plain(WS_EX_TOPMOST, 0);
        raise(topmost);
        assert_eq!(active(), topmost);
        let order = Cell::new(None);
        let target = Target {
            hwnd: plain(0, 0),
            class: WindowClass::Operator,
            order_during_show: &order,
        };
        place::show(&target, behind_plan(target.hwnd), false).unwrap();
        assert!(is_above(lowest, target.hwnd), "at the bottom, not in the topmost band");
        // SAFETY: plain values, read only.
        let style = unsafe { GetWindowLongPtrW(target.hwnd, GWL_EXSTYLE) } as u32;
        assert_eq!(style & WS_EX_TOPMOST, 0);
        assert_eq!(active(), topmost);
        for hwnd in [target.hwnd, topmost, lowest] {
            destroy(hwnd);
        }
    });
}

#[test]
fn a_hidden_maximised_window_is_not_at_its_own_size() {
    on_private_desktop(|| {
        let hwnd = plain(0, 0);
        assert!(place::is_normal_size(hwnd));
        show(hwnd, SW_SHOWMAXIMIZED);
        show(hwnd, SW_HIDE);
        assert!(!place::is_normal_size(hwnd), "a scale change must not ask it for a size");
        show(hwnd, SW_SHOWMINNOACTIVE);
        show(hwnd, SW_HIDE);
        assert!(!place::is_normal_size(hwnd));
        destroy(hwnd);
    });
}

/// A system command taken out of the queue of `hwnd`, not dispatched.
fn take_command(hwnd: HWND) -> Option<usize> {
    // SAFETY: a message structure of this function; a window of this thread.
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        (PeekMessageW(&mut message, hwnd, WM_SYSCOMMAND, WM_SYSCOMMAND, PM_REMOVE) != 0)
            .then_some(message.wParam)
    }
}

/// MT-214 E4-F2 against a real window and its queue: a close command waiting for the window is seen only when it is
/// the first system command that waits, and is left there. F4 without Alt, as the Operator presses it where no window
/// has the focus, handled by the system's own default window procedure, queues no close command; so a close command
/// another program posts after it is nobody's. (A press on the close button dragged off it needs the mouse button held
/// in the system's tracking, which a desktop without input never has; `closing.rs` proves that case without the system.)
#[test]
fn f4_alone_makes_no_close_command_and_lends_nothing_to_a_posted_one() {
    on_private_desktop(|| {
        let hwnd = plain(0, 0);
        let at = |millis: u64| EventTime {
            nanos: millis * 1_000_000,
        };
        let post = |command: u32| {
            // SAFETY: plain values, to a window of this thread.
            assert_ne!(unsafe { PostMessageW(hwnd, WM_SYSCOMMAND, command as usize, 0) }, 0);
        };
        assert!(!closing::queued_close(hwnd));
        post(SC_MINIMIZE);
        post(SC_CLOSE);
        assert!(!closing::queued_close(hwnd), "a minimise waits first");
        assert_eq!(take_command(hwnd), Some(SC_MINIMIZE as usize));
        assert!(closing::queued_close(hwnd));
        assert!(closing::queued_close(hwnd), "looked at, not taken out");
        assert_eq!(take_command(hwnd), Some(SC_CLOSE as usize));
        assert!(!closing::queued_close(hwnd));
        // F4 alone, the Operator's, through the system's own handling: scan code 0x3E, no KF_ALTDOWN (lParam bit 29).
        let requests = Closing::default();
        let f4 = usize::from(VK_F4);
        let pending = requests
            .before(at(1), (WM_SYSKEYDOWN, f4, 0x003E_0001), Arrival::Queue, true, || closing::queued_close(hwnd))
            .expect("a press");
        // SAFETY: plain values, for a window of this thread; the system's default handling of the key.
        unsafe { DefWindowProcW(hwnd, WM_SYSKEYDOWN, f4, 0x003E_0001) };
        requests.after(at(2), pending, || closing::queued_close(hwnd));
        assert!(!closing::queued_close(hwnd), "F4 alone made no close command");
        // Another program's close command, posted right after, taken out and carried out.
        post(SC_CLOSE);
        assert_eq!(take_command(hwnd), Some(SC_CLOSE as usize));
        let ends = requests.before(at(3), (WM_SYSCOMMAND, SC_CLOSE as usize, 0), Arrival::Queue, false, || false);
        requests.before(at(3), (WM_CLOSE, 0, 0), Arrival::ThisThread, false, || false);
        requests.after(at(3), ends.expect("a close command"), || false);
        assert_eq!(requests.open(at(4)), Some(false), "nobody's");
        destroy(hwnd);
    });
}
