//! Which keyboard messages the window library sees (MT-214 W6, R2-N2).
//!
//! winit 0.30.13 builds one key event over several messages (`keyboard.rs`, `process_message`). On a key down it looks
//! at the queue with `PeekMessageW(.., PM_NOREMOVE)` (`next_kbd_msg`, line 115) and keeps the key open only after that
//! look (124-135); on a dead key it unwraps the open key (161) and panics without one, and its panic ends the loop.
//!
//! The look itself is the hazard: during it the system dispatches the messages other threads sent to the thread's
//! windows, and only those (PeekMessage, Remarks). A dead key another thread sends then reaches winit inside its own
//! key down, before it opened the key.
//!
//! So, before winit sees a message: (1) a keyboard message sent from another thread is answered with 0 and never
//! reaches it, wherever it comes, also inside its look at the queue. Real input is never one: it comes through the
//! queue, and the characters `TranslateMessage` makes are posted. (2) Of the queued ones, a dead key that would find
//! no open key (posted by another program, or out of order) is answered with 0 as well; the gate follows winit's own
//! decision, looking at the queue before it does. With (1) nothing changes the queue between the two looks but posts
//! added behind, which can only make winit open a key the gate did not; a dead key the gate then keeps from it leaves
//! that key open in winit until its next key down resets it (125). No path of winit's unwraps there.

use std::cell::Cell;

use windows_sys::Win32::Foundation::{HWND, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F4;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, MSG, PM_NOREMOVE, WM_CHAR, WM_DEADCHAR, WM_KEYDOWN, WM_KEYFIRST, WM_KEYLAST,
    WM_KEYUP, WM_SYSCHAR, WM_SYSDEADCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// Why the gate kept a message from the window library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kept {
    /// Sent from another thread.
    Sent,
    /// A dead key without an open key.
    StrayDeadKey,
}

/// The gate of one window.
#[derive(Debug, Default)]
pub(super) struct KeyGate {
    /// The window library holds a key press open.
    open: Cell<bool>,
}

impl KeyGate {
    /// Whether `message` reaches the window library. `sent`: from another thread. `next` gives the next keyboard
    /// message in the queue, as the library looks for it; it is asked only where the library asks.
    pub(super) fn check(
        &self,
        message: u32,
        wparam: WPARAM,
        sent: bool,
        next: impl Fn() -> Option<u32>,
    ) -> Result<(), Kept> {
        if !(WM_KEYFIRST..=WM_KEYLAST).contains(&message) {
            return Ok(());
        }
        if sent {
            return Err(Kept::Sent);
        }
        match message {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // The library leaves Alt+F4 alone.
                if !(message == WM_SYSKEYDOWN && wparam == usize::from(VK_F4)) {
                    let belongs = next().is_some_and(|next| {
                        !matches!(next, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP)
                    });
                    self.open.set(belongs);
                }
                Ok(())
            }
            WM_DEADCHAR | WM_SYSDEADCHAR if !self.open.replace(false) => Err(Kept::StrayDeadKey),
            WM_CHAR | WM_SYSCHAR => {
                let more = next().is_some_and(|next| matches!(next, WM_CHAR | WM_SYSCHAR));
                self.open.set(self.open.get() && more);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// The next keyboard message of `hwnd` in the queue, as the window library looks for it.
pub(super) fn next_key_message(hwnd: HWND) -> Option<u32> {
    // SAFETY: a message structure that outlives the call; nothing is taken out of the queue.
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        (PeekMessageW(&mut message, hwnd, WM_KEYFIRST, WM_KEYLAST, PM_NOREMOVE) != 0)
            .then_some(message.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: [u32; 8] = [
        WM_KEYDOWN,
        WM_SYSKEYDOWN,
        WM_KEYUP,
        WM_SYSKEYUP,
        WM_CHAR,
        WM_SYSCHAR,
        WM_DEADCHAR,
        WM_SYSDEADCHAR,
    ];

    fn queued(gate: &KeyGate, message: u32, next: Option<u32>) -> Result<(), Kept> {
        gate.check(message, 0x41, false, || next)
    }

    /// R2-N2: a dead key sent while a key is open, as it comes inside the library's look at the queue, never reaches
    /// it; nor does any other keyboard message sent from another thread, which would start or change a key.
    #[test]
    fn a_keyboard_message_sent_from_another_thread_never_reaches_the_library() {
        for message in KEYS {
            let gate = KeyGate::default();
            assert_eq!(queued(&gate, WM_KEYDOWN, Some(WM_DEADCHAR)), Ok(()));
            let asked = Cell::new(false);
            let next = || {
                asked.set(true);
                Some(WM_DEADCHAR)
            };
            assert_eq!(gate.check(message, 0x41, true, next), Err(Kept::Sent), "{message:#x}");
            assert!(!asked.get(), "the queue is not even looked at");
            assert_eq!(queued(&gate, WM_DEADCHAR, None), Ok(()), "the key stayed open for its own dead key");
        }
        let gate = KeyGate::default();
        assert_eq!(gate.check(0x0200, 0, true, || None), Ok(()), "not a keyboard message");
    }

    #[test]
    fn a_dead_key_reaches_the_library_only_after_a_key_down_it_belongs_to() {
        let gate = KeyGate::default();
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Err(Kept::StrayDeadKey), "posted alone");
        assert_eq!(queued(&gate, WM_KEYDOWN, Some(WM_DEADCHAR)), Ok(()));
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Ok(()));
        assert_eq!(queued(&gate, WM_SYSDEADCHAR, None), Err(Kept::StrayDeadKey), "a second one");
        assert_eq!(queued(&gate, WM_KEYDOWN, Some(WM_KEYUP)), Ok(()));
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Err(Kept::StrayDeadKey), "the key closed at once");
        // Characters keep it open while more follow, and close it with the last.
        assert_eq!(queued(&gate, WM_KEYDOWN, Some(WM_CHAR)), Ok(()));
        assert_eq!(queued(&gate, WM_CHAR, Some(WM_CHAR)), Ok(()));
        assert_eq!(queued(&gate, WM_CHAR, None), Ok(()));
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Err(Kept::StrayDeadKey));
        // Alt+F4 neither opens nor closes one.
        assert_eq!(queued(&gate, WM_KEYDOWN, Some(WM_DEADCHAR)), Ok(()));
        let f4 = usize::from(VK_F4);
        assert_eq!(gate.check(WM_SYSKEYDOWN, f4, false, || Some(WM_KEYUP)), Ok(()));
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Ok(()));
    }
}
