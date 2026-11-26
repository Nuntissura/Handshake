//! How the message a window procedure of the loop handles reached it (MT-214 E3-F5).
//!
//! `InSendMessageEx` says whether the thread handles a message that another thread sent ("the current window procedure
//! is processing a message that was sent from another thread"), and it keeps saying so while a loop that runs inside
//! that message, a dialog's or the system's move loop, dispatches posted ones (E3-F5). So it cannot tell a key the
//! Operator typed into such a loop from one another thread sends.
//!
//! A `WH_GETMESSAGE` hook can: the system calls it "whenever the GetMessage or PeekMessage function has retrieved a
//! message from an application message queue", before the caller gets it, with `PM_REMOVE` when the message was taken
//! out (learn.microsoft.com/windows/win32/winmsg/getmsgproc). A sent message never passes it. Installed for this thread
//! only (`dwThreadId` the thread's own, `hmod` NULL for a procedure of this process; SetWindowsHookExW), it sees this
//! thread's own queue and nothing of other threads or programs; it reads and changes nothing. It notes the newest
//! message taken out and whether the thread was inside a sent message then. `DispatchMessage` follows the retrieval
//! with no look at the queue between, so a window procedure that gets the same message in the same state gets it out
//! of the queue; the note is used up by it.
//!
//! What it cannot tell: a copy another thread sends of a message that was taken out and not dispatched, while this
//! thread handles a sent message and before it takes out the next one. Outside a sent message a copy is always told
//! apart (it comes inside one), which is where the loop and every loop the host runs from an event or a wake take
//! messages out.
//!
//! Lifetime as `guard.rs`: installed on the loop's thread before the window library makes its first window, removed
//! when the loop is over, also when it unwinds. One per thread. The hook neither panics nor unwinds into the system.

use std::cell::Cell;
use std::marker::PhantomData;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, InSendMessageEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK,
    ISMEX_NOSEND, MSG, PM_REMOVE, WH_GETMESSAGE,
};

/// How a message reached the window procedure that handles it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Arrival {
    /// Taken out of the thread's queue and dispatched: posted, by anybody, or input.
    Queue,
    /// Sent by this thread, or by the system on its behalf, outside any message another thread sent.
    ThisThread,
    /// Sent by another thread, or sent by this one while it handles a message another thread sent.
    OtherThread,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Taken {
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    /// The thread was inside a message another thread sent when it took this one out.
    inside_sent: bool,
}

#[derive(Default)]
struct State {
    hook: Cell<HHOOK>,
    newest: Cell<Option<Taken>>,
}

thread_local! {
    static STATE: State = State::default();
}

/// The hook, for as long as this value lives. Not `Send`: it belongs to the thread it was installed on.
pub(super) struct RetrievalWatch {
    hook: HHOOK,
    here: PhantomData<*const ()>,
}

/// Why no watch was installed.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum WatchRefused {
    /// This thread has one.
    AlreadyInstalled,
    /// The operating system refused, with its error number.
    System(u32),
}

impl RetrievalWatch {
    pub(super) fn install() -> Result<Self, WatchRefused> {
        if STATE.with(|state| state.hook.get()) != 0 {
            return Err(WatchRefused::AlreadyInstalled);
        }
        // SAFETY: a procedure of this process for this thread, so no module (setwindowshookexw, `hmod`).
        let hook =
            unsafe { SetWindowsHookExW(WH_GETMESSAGE, Some(on_get_message), 0, GetCurrentThreadId()) };
        if hook == 0 {
            return Err(WatchRefused::System(super::last_error()));
        }
        STATE.with(|state| {
            state.hook.set(hook);
            state.newest.set(None);
        });
        Ok(Self {
            hook,
            here: PhantomData,
        })
    }
}

impl Drop for RetrievalWatch {
    fn drop(&mut self) {
        // SAFETY: the hook this value installed, on its own thread.
        unsafe { UnhookWindowsHookEx(self.hook) };
        let _ = STATE.try_with(|state| {
            state.hook.set(0);
            state.newest.set(None);
        });
    }
}

/// The thread handles a message another thread sent, or a message a loop inside one dispatches.
fn inside_sent() -> bool {
    // SAFETY: the one argument is reserved and must be null.
    unsafe { InSendMessageEx(std::ptr::null()) != ISMEX_NOSEND }
}

/// How the message the calling window procedure handles reached it. Uses up the note of the message taken out, when
/// this is that message.
pub(super) fn arrival(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> Arrival {
    let inside = inside_sent();
    let matched = STATE
        .try_with(|state| {
            let newest = state.newest.get()?;
            let same = newest.hwnd == hwnd
                && newest.message == message
                && newest.wparam == wparam
                && newest.lparam == lparam;
            if !same {
                return None;
            }
            state.newest.set(None);
            Some(newest.inside_sent)
        })
        .ok()
        .flatten();
    decide(matched, inside)
}

/// `matched`: the message is the one taken out last, and whether the thread was inside a sent message then.
fn decide(matched: Option<bool>, inside_sent: bool) -> Arrival {
    match (matched, inside_sent) {
        (Some(_), false) | (Some(true), true) => Arrival::Queue,
        // Inside a sent message, a message taken out outside one is a copy that arrived since.
        (Some(false) | None, true) => Arrival::OtherThread,
        (None, false) => Arrival::ThisThread,
    }
}

unsafe extern "system" fn on_get_message(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // A negative code goes on unseen (getmsgproc, `code`); `lparam` points to the message only for HC_ACTION.
    if code == HC_ACTION as i32 && (wparam as u32 & PM_REMOVE) != 0 && lparam != 0 {
        // SAFETY: for HC_ACTION the system passes the message it retrieved, valid during the call; read only.
        let message = unsafe { &*(lparam as *const MSG) };
        let taken = Taken {
            hwnd: message.hwnd,
            message: message.message,
            wparam: message.wParam,
            lparam: message.lParam,
            inside_sent: inside_sent(),
        };
        let _ = STATE.try_with(|state| state.newest.set(Some(taken)));
    }
    let hook = STATE.try_with(|state| state.hook.get()).unwrap_or(0);
    // SAFETY: the values as they came, to the next hook in the chain.
    unsafe { CallNextHookEx(hook, code, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MT-214 E3-F5: inside a sent message, a message the thread took out there is from the queue; a sent copy of one
    /// taken out before is not, and neither is one nobody took out.
    #[test]
    fn inside_a_sent_message_only_what_was_taken_out_there_counts_as_posted() {
        assert_eq!(decide(Some(true), true), Arrival::Queue, "dispatched by a loop inside it");
        assert_eq!(decide(Some(false), true), Arrival::OtherThread, "a copy sent back");
        assert_eq!(decide(None, true), Arrival::OtherThread);
        assert_eq!(decide(Some(false), false), Arrival::Queue);
        assert_eq!(decide(None, false), Arrival::ThisThread);
    }

    #[test]
    fn the_note_is_used_up_by_the_message_it_names() {
        let taken = Taken {
            hwnd: 5,
            message: 0x100,
            wparam: 0x41,
            lparam: 1,
            inside_sent: false,
        };
        STATE.with(|state| state.newest.set(Some(taken)));
        assert_eq!(arrival(5, 0x100, 0x41, 2), Arrival::ThisThread, "another message");
        assert_eq!(arrival(5, 0x100, 0x41, 1), Arrival::Queue);
        assert_eq!(arrival(5, 0x100, 0x41, 1), Arrival::ThisThread, "used up");
    }
}
