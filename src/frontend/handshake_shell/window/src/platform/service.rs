//! The loop's own window. It is never shown: it receives wakes and what the system tells every program.
//!
//! Top-level and not message-only, because a message-only window is not told about sleep, the end of the session and
//! display changes.

use std::ptr;
use std::rc::Rc;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::{
    RegisterSuspendResumeNotification, UnregisterSuspendResumeNotification, HPOWERNOTIFY,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, RegisterClassExW,
    SetWindowLongPtrW, DEVICE_NOTIFY_WINDOW_HANDLE, GWLP_USERDATA, PBT_APMRESUMEAUTOMATIC,
    PBT_APMSUSPEND, SPI_SETWORKAREA, WM_DISPLAYCHANGE, WM_ENDSESSION, WM_POWERBROADCAST,
    WM_QUERYENDSESSION, WM_SETTINGCHANGE, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP,
};

use super::loop_core::Core;
use super::retrieval::{self, Arrival};
use super::wake::{WM_PROBE, WM_WAKE};
use super::{guard, last_error};
use crate::event::Event;
use crate::host::LoopError;
use crate::queue::MAX_EVENTS;

/// "hsk_shell_service", ended by 0.
const CLASS: [u16; 18] = wide(b"hsk_shell_service\0");

const fn wide<const N: usize>(text: &[u8; N]) -> [u16; N] {
    let mut units = [0; N];
    let mut at = 0;
    while at < N {
        units[at] = text[at] as u16;
        at += 1;
    }
    units
}

/// The service window. Dropping it destroys the window.
pub(super) struct ServiceWindow {
    pub(super) hwnd: HWND,
    power: HPOWERNOTIFY,
    /// The reference the window procedure uses.
    core: *const Core,
}

impl ServiceWindow {
    pub(super) fn create(core: &Rc<Core>) -> Result<Self, LoopError> {
        // SAFETY: every pointer outlives its call, and the class name is ended by 0. Registering the class again fails
        // and leaves the first.
        let hwnd = unsafe {
            let instance = GetModuleHandleW(ptr::null());
            let class = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(procedure),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: 0,
                hCursor: 0,
                hbrBackground: 0,
                lpszMenuName: ptr::null(),
                lpszClassName: CLASS.as_ptr(),
                hIconSm: 0,
            };
            RegisterClassExW(&class);
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                CLASS.as_ptr(),
                ptr::null(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                0,
                0,
                instance,
                ptr::null(),
            )
        };
        if hwnd == 0 {
            return Err(LoopError::System(last_error()));
        }
        if !guard::mark_quiet(hwnd) {
            // SAFETY: the window made above, of this thread.
            unsafe { DestroyWindow(hwnd) };
            return Err(LoopError::System(0));
        }
        let core = Rc::into_raw(Rc::clone(core));
        // SAFETY: the window is of this thread; the reference is given back when the window is gone.
        let power = unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, core as isize);
            // A machine with modern standby reports sleep only to who asks. Where this fails, the broadcast remains.
            RegisterSuspendResumeNotification(hwnd, DEVICE_NOTIFY_WINDOW_HANDLE)
        };
        Ok(Self { hwnd, power, core })
    }
}

impl Drop for ServiceWindow {
    fn drop(&mut self) {
        // SAFETY: the window and the registration are this one's; the reference is given back after the window's last
        // message.
        unsafe {
            if self.power != 0 {
                UnregisterSuspendResumeNotification(self.power);
            }
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            DestroyWindow(self.hwnd);
            drop(Rc::from_raw(self.core));
        }
    }
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: 0, or the reference that `ServiceWindow` holds for as long as it is set here. This one keeps the core
    // alive until the procedure returns.
    let core = unsafe {
        let core = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Core;
        if core.is_null() {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
        Rc::increment_strong_count(core);
        Rc::from_raw(core)
    };
    match core.guarded(|| answer(&core, hwnd, message, wparam, lparam)).flatten() {
        Some(result) => result,
        // SAFETY: the message as it came.
        None => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn answer(
    core: &Rc<Core>,
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<LRESULT> {
    match message {
        WM_WAKE => {
            // A wake is a posted message the loop took out of its queue, also inside a loop that runs inside a message
            // another thread sent (`retrieval.rs`). One sent from another thread is not, even with a genuine number;
            // one sent from this thread carries no number of an open post (`Lane::take`).
            if retrieval::arrival(hwnd, message, wparam, lparam) == Arrival::OtherThread {
                core.update_report(|report| {
                    report.sent_wakes_refused = report.sent_wakes_refused.saturating_add(1);
                });
            } else if let Ok(lane) = u32::try_from(wparam) {
                core.dispatch_wake(lane, lparam);
            }
            Some(0)
        }
        // Sent, so it comes inside any call that looks at the queue. It runs nothing of the host's.
        WM_PROBE => {
            core.probe_answered(lparam);
            Some(0)
        }
        WM_POWERBROADCAST => {
            match wparam as u32 {
                PBT_APMSUSPEND => core.push(Event::SystemSuspending),
                PBT_APMRESUMEAUTOMATIC => core.push(Event::SystemResumed),
                _ => {}
            }
            Some(1)
        }
        // The host decides nothing here: it is told when the session really ends.
        WM_QUERYENDSESSION => Some(1),
        WM_ENDSESSION => {
            if wparam != 0 {
                core.push(Event::SessionEnding);
                // The process may be gone once this returns.
                core.drain(MAX_EVENTS);
            }
            Some(0)
        }
        WM_DISPLAYCHANGE => {
            core.push(Event::MonitorsChanged);
            None
        }
        WM_SETTINGCHANGE => {
            if wparam as u32 == SPI_SETWORKAREA {
                core.push(Event::MonitorsChanged);
            }
            None
        }
        _ => None,
    }
}
