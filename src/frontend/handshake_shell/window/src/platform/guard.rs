//! The loop thread's activation guard (MT-214 W1, AC214-2).
//!
//! `SWP_NOACTIVATE` added in `WM_WINDOWPOSCHANGING` does not stop an activation: a plain `SetWindowPos`, a show after a
//! hide and a restore after a quiet minimise activate all the same (MT-214 evaluation probe). A CBT hook is told before
//! a window is activated or given the keyboard focus, and returning 1 prevents it ("HCBT_ACTIVATE", "HCBT_SETFOCUS";
//! learn.microsoft.com/windows/win32/winmsg/cbtproc). Installed for this thread only (`dwThreadId` the thread's own,
//! `hmod` NULL for a procedure of this process; learn.microsoft.com/windows/win32/api/winuser/nf-winuser-setwindowshookexw),
//! it sees the windows of this thread and nothing of other threads or programs.
//!
//! Refused: (1) a quiet window, or a window it owns, at any time, and the keyboard focus for one; (2) any window of
//! this thread while this crate carries out a change that must not activate ([`still`]), except the one window an
//! activating show names ([`allowing`]). Everything else, such as the Operator's click, Alt+Tab or the taskbar on an
//! Operator window, goes on to the next hook unchanged.
//!
//! Lifetime: [`ActivationGuard::install`] on the loop's thread before the window library makes its first window; its
//! drop removes the hook (`UnhookWindowsHookEx`) when the loop is over, also when it unwinds. A thread hook runs on its
//! own thread only, so nothing is inside it when removal returns (unhookwindowshookex, Remarks). One guard per thread.
//! The hook neither panics nor unwinds into the system.

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetAncestor, GetWindow, SetWindowsHookExW, UnhookWindowsHookEx, GA_ROOT,
    GW_OWNER, HCBT_ACTIVATE, HCBT_SETFOCUS, HHOOK, WH_CBT,
};

/// The longest owner chain followed.
const MAX_OWNERS: usize = 32;

/// The most windows marked quiet at once: every window of a loop, its own and the library's, with room.
const MAX_QUIET: usize = 128;

#[derive(Default)]
struct State {
    hook: Cell<HHOOK>,
    quiet: RefCell<Vec<HWND>>,
    /// How many [`still`] sections are open.
    still: Cell<u32>,
    /// The window the innermost section lets be activated; 0 for none.
    allowed: Cell<HWND>,
    refused: Cell<u64>,
}

thread_local! {
    static STATE: State = State::default();
}

/// The hook, for as long as this value lives. Not `Send`: it belongs to the thread it was installed on.
pub(super) struct ActivationGuard {
    hook: HHOOK,
    here: PhantomData<*const ()>,
}

/// Why no guard was installed.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum GuardRefused {
    /// This thread has one.
    AlreadyInstalled,
    /// The operating system refused, with its error number.
    System(u32),
}

impl ActivationGuard {
    pub(super) fn install() -> Result<Self, GuardRefused> {
        if STATE.with(|state| state.hook.get()) != 0 {
            return Err(GuardRefused::AlreadyInstalled);
        }
        // SAFETY: a procedure of this process for this thread, so no module (setwindowshookexw, `hmod`).
        let hook = unsafe { SetWindowsHookExW(WH_CBT, Some(on_cbt), 0, GetCurrentThreadId()) };
        if hook == 0 {
            return Err(GuardRefused::System(super::last_error()));
        }
        STATE.with(|state| {
            state.hook.set(hook);
            state.refused.set(0);
        });
        Ok(Self {
            hook,
            here: PhantomData,
        })
    }
}

impl Drop for ActivationGuard {
    fn drop(&mut self) {
        // SAFETY: the hook this guard installed, on its own thread.
        unsafe { UnhookWindowsHookEx(self.hook) };
        let _ = STATE.try_with(|state| {
            state.hook.set(0);
            state.still.set(0);
            state.allowed.set(0);
            if let Ok(mut quiet) = state.quiet.try_borrow_mut() {
                quiet.clear();
            }
        });
    }
}

/// Never activate `hwnd`, nor give it the keyboard focus. Returns false when it cannot be kept, so the caller does not
/// use the window.
pub(super) fn mark_quiet(hwnd: HWND) -> bool {
    STATE
        .try_with(|state| {
            let Ok(mut quiet) = state.quiet.try_borrow_mut() else {
                return false;
            };
            if quiet.contains(&hwnd) {
                return true;
            }
            if quiet.len() >= MAX_QUIET {
                return false;
            }
            quiet.push(hwnd);
            true
        })
        .unwrap_or(false)
}

/// The window is gone; its handle may be given to another one.
pub(super) fn forget(hwnd: HWND) {
    let _ = STATE.try_with(|state| {
        if let Ok(mut quiet) = state.quiet.try_borrow_mut() {
            quiet.retain(|kept| *kept != hwnd);
        }
    });
}

/// Activations the guard refused so far on this thread.
pub(super) fn refused() -> u64 {
    STATE.try_with(|state| state.refused.get()).unwrap_or(0)
}

/// Run `work`, in which no window of this thread is activated.
pub(super) fn still<T>(work: impl FnOnce() -> T) -> T {
    let _section = Section::open(0);
    work()
}

/// Run `work`, in which only `hwnd` may be activated: an activating show the policy planned.
pub(super) fn allowing<T>(hwnd: HWND, work: impl FnOnce() -> T) -> T {
    let _section = Section::open(hwnd);
    work()
}

/// An open section; closed also when its work unwinds.
struct Section {
    allowed_before: HWND,
}

impl Section {
    fn open(allowed: HWND) -> Self {
        let allowed_before = STATE
            .try_with(|state| {
                state.still.set(state.still.get().saturating_add(1));
                state.allowed.replace(allowed)
            })
            .unwrap_or(0);
        Self { allowed_before }
    }
}

impl Drop for Section {
    fn drop(&mut self) {
        let _ = STATE.try_with(|state| {
            state.still.set(state.still.get().saturating_sub(1));
            state.allowed.set(self.allowed_before);
        });
    }
}

impl State {
    /// `hwnd`, its top-level window or one of their owners is quiet. A registry that cannot be read counts as quiet.
    fn is_quiet(&self, hwnd: HWND) -> bool {
        let Ok(quiet) = self.quiet.try_borrow() else {
            return true;
        };
        if quiet.contains(&hwnd) {
            return true;
        }
        // SAFETY: plain values, read only; 0 for a window that is gone.
        let mut current = match unsafe { GetAncestor(hwnd, GA_ROOT) } {
            0 => hwnd,
            root => root,
        };
        for _ in 0..MAX_OWNERS {
            if current == 0 {
                return false;
            }
            if quiet.contains(&current) {
                return true;
            }
            // SAFETY: as above.
            current = unsafe { GetWindow(current, GW_OWNER) };
        }
        true
    }

    fn refuses(&self, code: u32, hwnd: HWND) -> bool {
        let refuses = match code {
            HCBT_ACTIVATE => {
                self.is_quiet(hwnd) || (self.still.get() > 0 && hwnd != self.allowed.get())
            }
            HCBT_SETFOCUS => hwnd != 0 && self.is_quiet(hwnd),
            _ => false,
        };
        if refuses {
            self.refused.set(self.refused.get().saturating_add(1));
        }
        refuses
    }
}

unsafe extern "system" fn on_cbt(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // A negative code goes on unseen (cbtproc, `nCode`). For both codes looked at, `wparam` is the window.
    if let Ok(code) = u32::try_from(code) {
        // Without the thread's state (it is being torn down) an activation is refused.
        let refused = STATE
            .try_with(|state| state.refuses(code, wparam as HWND))
            .unwrap_or(code == HCBT_ACTIVATE);
        if refused {
            return 1;
        }
    }
    let hook = STATE.try_with(|state| state.hook.get()).unwrap_or(0);
    // SAFETY: the values as they came, to the next hook in the chain.
    unsafe { CallNextHookEx(hook, code, wparam, lparam) }
}
