//! Which keyboard messages the window library sees (MT-214 W6, R2-N2, E3-F5).
//!
//! winit 0.30.13 builds one key event over several messages (`keyboard.rs`, `process_message`). On a key down it looks
//! at the queue with `PeekMessageW(.., PM_NOREMOVE)` (`next_kbd_msg`, line 115) and keeps the key open only after that
//! look (124-135); on a dead key it unwraps the open key (161) and panics without one, and its panic ends the loop.
//!
//! The look itself is the hazard: during it the system dispatches the messages other threads sent to the thread's
//! windows, and only those (PeekMessage, Remarks). A dead key another thread sends then reaches winit inside its own
//! key down, before it opened the key. So does a dead key that a loop dispatches which runs inside such a sent message.
//!
//! So, before winit sees a message: (1) a keyboard message another thread sent is answered with 0 and never reaches
//! it, wherever it comes, also inside its look at the queue. A key that was posted and taken out of the queue is none,
//! also when a loop that runs inside a message another thread sent dispatches it (`retrieval.rs`, E3-F5): the
//! Operator's keys, and those an on-screen keyboard, a remote desktop or `SendInput` puts into the input stream, pass.
//! Where a key came from is said by its stamp, not here. (2) A dead key that would find no open key is answered with 0
//! as well: one posted by another program or out of order, and any dead key that comes while winit still handles a
//! keyboard message of the window, before it opened or closed its key. The gate follows winit's decision, looking at
//! the queue before it does, and takes it over when winit returns; if another keyboard message reached winit
//! meanwhile, from a loop inside that one, it counts no key as open. With (1) nothing changes the queue between the two
//! looks but posts added behind, which can only make winit open a key the gate did not; a dead key the gate then keeps
//! from it leaves that key open in winit until its next key down resets it (125). No path of winit's unwraps there.

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

/// A keyboard message the window library is handling. Hand it back with [`KeyGate::done`] when the library returns.
#[derive(Debug)]
#[must_use]
pub(super) struct Passed {
    /// Whether the library holds a key open once it is done with this message; `None` when it does not change that.
    open_after: Option<bool>,
    /// The number of this message among those the library got.
    number: u64,
}

/// The gate of one window.
#[derive(Debug, Default)]
pub(super) struct KeyGate {
    /// The window library holds a key press open.
    open: Cell<bool>,
    /// Keyboard messages the library is handling now, one inside another.
    inside: Cell<u32>,
    /// Keyboard messages the library got so far.
    passed: Cell<u64>,
}

impl KeyGate {
    /// Whether `message` reaches the window library. `sent`: from another thread. `next` gives the next keyboard
    /// message in the queue, as the library looks for it; it is asked only where the library asks. `None` for a
    /// message that is not a keyboard message.
    pub(super) fn check(
        &self,
        message: u32,
        wparam: WPARAM,
        sent: bool,
        next: impl Fn() -> Option<u32>,
    ) -> Result<Option<Passed>, Kept> {
        if !(WM_KEYFIRST..=WM_KEYLAST).contains(&message) {
            return Ok(None);
        }
        if sent {
            return Err(Kept::Sent);
        }
        let open_after = match message {
            // The library leaves Alt+F4 alone.
            WM_SYSKEYDOWN if wparam == usize::from(VK_F4) => None,
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let belongs = next().is_some_and(|next| {
                    !matches!(next, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP)
                });
                Some(belongs)
            }
            WM_DEADCHAR | WM_SYSDEADCHAR => {
                // Inside another keyboard message the library may not have opened its key yet.
                if self.inside.get() > 0 || !self.open.replace(false) {
                    return Err(Kept::StrayDeadKey);
                }
                None
            }
            WM_CHAR | WM_SYSCHAR => {
                let more = next().is_some_and(|next| matches!(next, WM_CHAR | WM_SYSCHAR));
                Some(self.open.get() && more)
            }
            _ => None,
        };
        let number = self.passed.get().wrapping_add(1);
        self.passed.set(number);
        self.inside.set(self.inside.get().saturating_add(1));
        Ok(Some(Passed { open_after, number }))
    }

    /// The window library returned from a message [`KeyGate::check`] let through.
    pub(super) fn done(&self, passed: Passed) {
        self.inside.set(self.inside.get().saturating_sub(1));
        if let Some(open) = passed.open_after {
            // Another keyboard message reached the library meanwhile: what it left open is not known.
            let alone = self.passed.get() == passed.number;
            self.open.set(open && alone);
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

    /// A queued message, through the library and back.
    fn queued(gate: &KeyGate, message: u32, next: Option<u32>) -> Result<(), Kept> {
        let passed = gate.check(message, 0x41, false, || next)?;
        if let Some(passed) = passed {
            gate.done(passed);
        }
        Ok(())
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
            assert_eq!(gate.check(message, 0x41, true, next).err(), Some(Kept::Sent), "{message:#x}");
            assert!(!asked.get(), "the queue is not even looked at");
            assert_eq!(queued(&gate, WM_DEADCHAR, None), Ok(()), "the key stayed open for its own dead key");
        }
        let gate = KeyGate::default();
        assert!(matches!(gate.check(0x0200, 0, true, || None), Ok(None)), "not a keyboard message");
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
        let passed = gate.check(WM_SYSKEYDOWN, f4, false, || Some(WM_KEYUP)).unwrap().unwrap();
        gate.done(passed);
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Ok(()));
    }

    /// E3-F5 keeps R2-N2 closed: a posted dead key that a loop inside the library's look at the queue dispatches comes
    /// while the library handles its key down and before it opened the key; it never reaches the library, even where
    /// the key before left one open.
    #[test]
    fn a_dead_key_that_comes_while_the_library_handles_a_key_is_kept_from_it() {
        let gate = KeyGate::default();
        assert_eq!(queued(&gate, WM_KEYDOWN, Some(WM_DEADCHAR)), Ok(()));
        let outer = gate.check(WM_KEYDOWN, 0x42, false, || Some(WM_DEADCHAR)).unwrap().unwrap();
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Err(Kept::StrayDeadKey), "inside the key down");
        gate.done(outer);
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Ok(()), "its own, after it");
    }

    #[test]
    fn a_key_another_keyboard_message_came_inside_of_leaves_no_key_open() {
        let gate = KeyGate::default();
        let outer = gate.check(WM_KEYDOWN, 0x41, false, || Some(WM_DEADCHAR)).unwrap().unwrap();
        assert_eq!(queued(&gate, WM_KEYDOWN, Some(WM_KEYUP)), Ok(()), "a key a loop inside it dispatched");
        gate.done(outer);
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Err(Kept::StrayDeadKey));
        // Not counted as inside any more.
        assert_eq!(queued(&gate, WM_KEYDOWN, Some(WM_DEADCHAR)), Ok(()));
        assert_eq!(queued(&gate, WM_DEADCHAR, None), Ok(()));
    }
}
