//! Requests to close one window, and whether the Operator's own close gesture made one (MT-214 E3-F6).
//!
//! The hook of the window (`window.rs`) tells this what it sees before the window library does: the Operator's
//! presses, the system's close command (`WM_SYSCOMMAND`, `SC_CLOSE`) and the close message (`WM_CLOSE`). The request is
//! opened at the close message, not at the library's event for it, which the library can hold back. The decision is
//! `policy::CloseWatch`. Whether the system posts the close command or sends it (Wine's default window procedure posts
//! it for Alt+F4 and sends it for the close button; Windows' own is not documented), the close message comes from
//! within it, on this thread, and the decision holds for both.

use std::cell::Cell;

use windows_sys::Win32::Foundation::WPARAM;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F4;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HTCLOSE, HTSYSMENU, SC_CLOSE, WM_CLOSE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
    WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN, WM_NCMBUTTONDOWN, WM_NCRBUTTONDOWN, WM_NCXBUTTONDOWN,
    WM_POINTERDOWN, WM_RBUTTONDOWN, WM_SYSCOMMAND, WM_SYSKEYDOWN, WM_XBUTTONDOWN,
};

use super::retrieval::Arrival;
use crate::origin::{EventTime, OPERATOR_AUTHORITY_LIFETIME};
use crate::policy::CloseWatch;

#[derive(Clone, Copy)]
struct Request {
    at: EventTime,
    operators: bool,
}

/// What [`Closing::before`] leaves for [`Closing::after`]: a close command ended, and whether one that used the
/// Operator's gesture was under way before it.
#[derive(Clone, Copy, Debug)]
pub(super) struct CommandEnds(bool);

#[derive(Default)]
pub(super) struct Closing {
    /// The newest request, while it is open.
    request: Cell<Option<Request>>,
    watch: Cell<CloseWatch>,
}

impl Closing {
    /// A request to close the window is open: made within [`OPERATOR_AUTHORITY_LIFETIME`], and neither used nor
    /// overtaken by a show or the Operator's input since. Whether the Operator's own close gesture made it.
    pub(super) fn open(&self, now: EventTime) -> Option<bool> {
        self.request
            .get()
            .filter(|request| now.since(request.at) <= OPERATOR_AUTHORITY_LIFETIME)
            .map(|request| request.operators)
    }

    pub(super) fn end(&self) {
        self.request.set(None);
    }

    fn change(&self, change: impl FnOnce(&mut CloseWatch) -> bool) -> bool {
        let mut watch = self.watch.get();
        let result = change(&mut watch);
        self.watch.set(watch);
        result
    }

    /// Before the window library sees `message`. `operators_input`: the Operator's own input, which the operating
    /// system reported as hardware and the loop took out of its queue.
    pub(super) fn before(
        &self,
        now: EventTime,
        message: u32,
        wparam: WPARAM,
        arrival: Arrival,
        operators_input: bool,
    ) -> Option<CommandEnds> {
        if operators_input {
            if let Some(gesture) = press(message, wparam) {
                self.change(|watch| {
                    watch.operator_pressed(now, gesture);
                    false
                });
            }
        }
        match message {
            WM_SYSCOMMAND if (wparam & 0xFFF0) == SC_CLOSE as usize => {
                let from_other_thread = arrival == Arrival::OtherThread;
                let before = self.change(|watch| watch.command_begins(now, from_other_thread));
                Some(CommandEnds(before))
            }
            WM_CLOSE => {
                let operators = self.watch.get().message_is_operators(arrival == Arrival::ThisThread);
                // Anybody else's request leaves an open one of the Operator's as it is.
                if operators || self.open(now) != Some(true) {
                    self.request.set(Some(Request { at: now, operators }));
                }
                None
            }
            _ => None,
        }
    }

    /// The window library handled the close command.
    pub(super) fn after(&self, ends: CommandEnds) {
        self.change(|watch| {
            watch.command_ends(ends.0);
            false
        });
    }
}

/// A press of a key or a button: whether it is one the system turns into a close of the window (Alt+F4, its close
/// button, a double-click on its system menu); `None` for any other message.
fn press(message: u32, wparam: WPARAM) -> Option<bool> {
    let close = match message {
        WM_SYSKEYDOWN => wparam == usize::from(VK_F4),
        WM_NCLBUTTONDOWN => wparam == HTCLOSE as usize,
        WM_NCLBUTTONDBLCLK => wparam == HTSYSMENU as usize,
        WM_KEYDOWN | WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        | WM_NCRBUTTONDOWN | WM_NCMBUTTONDOWN | WM_NCXBUTTONDOWN | WM_POINTERDOWN => false,
        _ => return None,
    };
    Some(close)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(millis: u64) -> EventTime {
        EventTime {
            nanos: millis * 1_000_000,
        }
    }

    /// The messages of Alt+F4, or of the close button, as the hook sees them: the gesture taken out of the queue, the
    /// close command, and the close message the system sends inside it.
    fn closed(closing: &Closing, gesture: (u32, WPARAM), command: Arrival, message: Arrival) {
        closing.before(at(1), gesture.0, gesture.1, Arrival::Queue, true);
        let ends = closing.before(at(2), WM_SYSCOMMAND, SC_CLOSE as usize, command, false);
        closing.before(at(3), WM_CLOSE, 0, message, false);
        closing.after(ends.expect("a close command"));
    }

    /// MT-214 E3-F6, as the hook carries it out.
    #[test]
    fn only_the_operators_close_gesture_opens_a_request_that_is_the_operators() {
        let alt_f4 = (WM_SYSKEYDOWN, usize::from(VK_F4));
        let button = (WM_NCLBUTTONDOWN, HTCLOSE as usize);
        for gesture in [alt_f4, button] {
            let closing = Closing::default();
            closed(&closing, gesture, Arrival::Queue, Arrival::ThisThread);
            assert_eq!(closing.open(at(4)), Some(true), "{gesture:?}");
            closing.end();
            assert_eq!(closing.open(at(4)), None);
        }
        // A close message posted, or sent by another thread, inside the Operator's own close command.
        for message in [Arrival::Queue, Arrival::OtherThread] {
            let closing = Closing::default();
            closed(&closing, alt_f4, Arrival::ThisThread, message);
            assert_eq!(closing.open(at(4)), Some(false), "{message:?}");
        }
        // A close command another thread sent, after the Operator's gesture.
        let closing = Closing::default();
        closed(&closing, alt_f4, Arrival::OtherThread, Arrival::ThisThread);
        assert_eq!(closing.open(at(4)), Some(false));
        // A gesture another program's input made, or another press: no gesture.
        let closing = Closing::default();
        closing.before(at(1), alt_f4.0, alt_f4.1, Arrival::Queue, false);
        closing.before(at(1), WM_NCLBUTTONDOWN, HTCLOSE as usize, Arrival::Queue, true);
        closing.before(at(1), WM_KEYDOWN, 0x41, Arrival::Queue, true);
        let ends = closing.before(at(2), WM_SYSCOMMAND, SC_CLOSE as usize, Arrival::Queue, false);
        closing.before(at(3), WM_CLOSE, 0, Arrival::ThisThread, false);
        closing.after(ends.unwrap());
        assert_eq!(closing.open(at(4)), Some(false));
    }

    #[test]
    fn anybody_elses_request_leaves_the_operators_open_and_requests_end() {
        let closing = Closing::default();
        closed(&closing, (WM_SYSKEYDOWN, usize::from(VK_F4)), Arrival::Queue, Arrival::ThisThread);
        closing.before(at(5), WM_CLOSE, 0, Arrival::Queue, false);
        assert_eq!(closing.open(at(6)), Some(true), "not overtaken");
        let lifetime = u64::try_from(OPERATOR_AUTHORITY_LIFETIME.as_millis()).unwrap();
        assert_eq!(closing.open(at(3 + lifetime)), Some(true));
        assert_eq!(closing.open(at(4 + lifetime)), None, "ended with its lifetime");
        closing.before(at(10 + lifetime), WM_CLOSE, 0, Arrival::Queue, false);
        assert_eq!(closing.open(at(11 + lifetime)), Some(false));
        assert_eq!(press(WM_NCLBUTTONDBLCLK, HTSYSMENU as usize), Some(true));
        assert_eq!(press(0x0200, 0), None, "a pointer move is no press");
    }
}
