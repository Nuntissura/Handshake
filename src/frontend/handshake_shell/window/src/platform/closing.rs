//! Requests to close one window, and whether the Operator's own close gesture made one (MT-214 E3-F6, E4-F2).
//!
//! The hook of the window (`window.rs`) tells this what it sees before the window library does, and when the library
//! is done with it: the Operator's presses, the system's close command (`WM_SYSCOMMAND`, `SC_CLOSE`) and the close
//! message (`WM_CLOSE`). The request is opened at the close message, not at the library's event for it, which the
//! library can hold back. The decision is `policy::CloseWatch`.
//!
//! A gesture counts only through the command the system makes of it while it handles the gesture's message (E4-F2).
//! The system sends that command from within the handling (the close button and a double-click on the system menu, as
//! the evaluation's probe saw for the latter), or posts it then (Wine's default window procedure does for Alt+F4;
//! Windows' own is not documented). A sent one arrives while the press is being handled; a posted one is seen in the
//! queue once the handling ended ([`queued_close`]), when none was queued before it, and is the next close command
//! taken out of the queue. A gesture whose handling made no command lends nothing to a close command that comes after.
//!
//! Only Alt+F4 may have its command posted (E5-F4). The handling of a press on the close button lasts as long as the
//! button is held, and another program can see that hold (the button's window captures the mouse) and post a close
//! command meanwhile; were a posted command counted for it, a press dragged off the button would credit that program's
//! command to the Operator. The close button and the system menu send theirs, so a posted one is never theirs.

use std::cell::Cell;

use windows_sys::Win32::Foundation::{HWND, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_F4;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, HTCLOSE, HTSYSMENU, MSG, PM_NOREMOVE, PM_NOYIELD, PM_QS_POSTMESSAGE, SC_CLOSE,
    WM_CLOSE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN,
    WM_NCMBUTTONDOWN, WM_NCRBUTTONDOWN, WM_NCXBUTTONDOWN, WM_POINTERDOWN, WM_RBUTTONDOWN,
    WM_SYSCOMMAND, WM_SYSKEYDOWN, WM_XBUTTONDOWN,
};

use super::retrieval::Arrival;
use crate::origin::{EventTime, OPERATOR_AUTHORITY_LIFETIME};
use crate::policy::{CloseWatch, CommandFrom};

#[derive(Clone, Copy)]
struct Request {
    at: EventTime,
    operators: bool,
}

/// What [`Closing::before`] leaves for [`Closing::after`].
#[derive(Clone, Copy, Debug)]
pub(super) enum Pending {
    /// A close command ended, and whether one that the Operator's gesture made was under way before it.
    Command(bool),
    /// The handling of the Operator's press ended. `before`: whether a close gesture was being handled before it;
    /// `may_post`: this press is Alt+F4, whose handling may post its close command (E5-F4), and no close command
    /// waited in the queue before it.
    Press { before: bool, may_post: bool },
}

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

    fn change<T>(&self, change: impl FnOnce(&mut CloseWatch) -> T) -> T {
        let mut watch = self.watch.get();
        let result = change(&mut watch);
        self.watch.set(watch);
        result
    }

    /// Before the window library sees `message`. `operators_input`: the Operator's own input, which the operating
    /// system reported as hardware and the loop took out of its queue. `queued`: whether a close command for the window
    /// waits in the queue now ([`queued_close`]); asked only for Alt+F4, the one gesture whose command may be posted
    /// (E5-F4), and never while the watch changes, since a look at the queue may run messages other threads sent.
    pub(super) fn before(
        &self,
        now: EventTime,
        message: u32,
        wparam: WPARAM,
        arrival: Arrival,
        operators_input: bool,
        queued: impl FnOnce() -> bool,
    ) -> Option<Pending> {
        if operators_input {
            if let Some(gesture) = press(message, wparam) {
                let may_post = gesture && message == WM_SYSKEYDOWN && !queued();
                let before = self.change(|watch| watch.press_begins(gesture));
                return Some(Pending::Press { before, may_post });
            }
        }
        match message {
            WM_SYSCOMMAND if (wparam & 0xFFF0) == SC_CLOSE as usize => {
                let from = match arrival {
                    Arrival::Queue => CommandFrom::Queue,
                    Arrival::ThisThread => CommandFrom::ThisThread,
                    Arrival::OtherThread => CommandFrom::OtherThread,
                };
                let before = self.change(|watch| watch.command_begins(now, from));
                Some(Pending::Command(before))
            }
            WM_CLOSE => {
                let operators = self
                    .watch
                    .get()
                    .message_is_operators(arrival == Arrival::ThisThread);
                // Anybody else's request leaves an open one of the Operator's as it is.
                if operators || self.open(now) != Some(true) {
                    self.request.set(Some(Request { at: now, operators }));
                }
                None
            }
            _ => None,
        }
    }

    /// The window library handled the message [`Closing::before`] left `pending` for. `queued` as there: after Alt+F4,
    /// a close command in the queue that was not there before is the one the system posted for it.
    pub(super) fn after(&self, now: EventTime, pending: Pending, queued: impl FnOnce() -> bool) {
        match pending {
            Pending::Command(before) => self.change(|watch| watch.command_ends(before)),
            Pending::Press { before, may_post } => {
                // The handling is over before the queue is looked at.
                self.change(|watch| watch.press_ends(before));
                if may_post && queued() {
                    self.change(|watch| watch.command_posted(now));
                }
            }
        }
    }
}

/// Whether the first system command that waits in this thread's queue for `hwnd` is a close command. Takes nothing out,
/// looks at posted messages only, and does not yield.
pub(super) fn queued_close(hwnd: HWND) -> bool {
    // SAFETY: a message structure that outlives the call; nothing is taken out of the queue.
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        let flags = PM_NOREMOVE | PM_NOYIELD | PM_QS_POSTMESSAGE;
        PeekMessageW(&mut message, hwnd, WM_SYSCOMMAND, WM_SYSCOMMAND, flags) != 0
            && (message.wParam & 0xFFF0) == SC_CLOSE as usize
    }
}

/// A press of a key or a button: whether it is one the system turns into a close of the window (Alt+F4, its close
/// button, a double-click on its system menu); `None` for any other message. Whether the system does is what its
/// handling shows (E4-F2): F4 without Alt, or a press on the close button that is dragged off it, makes no command.
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

    const ALT_F4: (u32, WPARAM) = (WM_SYSKEYDOWN, VK_F4 as WPARAM);
    const BUTTON: (u32, WPARAM) = (WM_NCLBUTTONDOWN, HTCLOSE as WPARAM);

    /// What the system does while it handles the Operator's gesture.
    #[derive(Clone, Copy, Debug)]
    enum Handling {
        /// It sends the close command, and the close message inside it.
        Sends,
        /// It posts the close command.
        Posts,
        /// Nothing: a press on the close button dragged off it, F4 without Alt.
        Nothing,
    }

    /// The hook's calls for the Operator's gesture: `queue` is what waits in the queue, `true` for a close command.
    fn gesture(closing: &Closing, gesture: (u32, WPARAM), handling: Handling, queue: &Cell<bool>) {
        let pending = closing
            .before(at(1), gesture.0, gesture.1, Arrival::Queue, true, || queue.get())
            .expect("a press");
        match handling {
            Handling::Sends => command(closing, 2, Arrival::ThisThread, Arrival::ThisThread),
            Handling::Posts => queue.set(true),
            Handling::Nothing => {}
        }
        closing.after(at(3), pending, || queue.get());
    }

    /// A close command, and the close message the system sends inside it.
    fn command(closing: &Closing, millis: u64, command: Arrival, message: Arrival) {
        let ends = closing.before(at(millis), WM_SYSCOMMAND, SC_CLOSE as usize, command, false, || false);
        closing.before(at(millis), WM_CLOSE, 0, message, false, || false);
        closing.after(at(millis), ends.expect("a close command"), || false);
    }

    /// The command waiting in the queue is taken out and carried out.
    fn taken_out(closing: &Closing, queue: &Cell<bool>, millis: u64) {
        assert!(queue.replace(false), "a close command waits");
        command(closing, millis, Arrival::Queue, Arrival::ThisThread);
    }

    /// MT-214 E3-F6, as the hook carries it out.
    #[test]
    fn only_the_operators_close_gesture_opens_a_request_that_is_the_operators() {
        for (made, handling) in [(BUTTON, Handling::Sends), (ALT_F4, Handling::Posts), (ALT_F4, Handling::Sends)] {
            let closing = Closing::default();
            let queue = Cell::new(false);
            gesture(&closing, made, handling, &queue);
            if matches!(handling, Handling::Posts) {
                taken_out(&closing, &queue, 4);
            }
            assert_eq!(closing.open(at(5)), Some(true), "{handling:?}");
            closing.end();
            assert_eq!(closing.open(at(5)), None);
        }
        // A close message posted, or sent by another thread, inside the Operator's own close command.
        for message in [Arrival::Queue, Arrival::OtherThread] {
            let closing = Closing::default();
            let pending = closing
                .before(at(1), BUTTON.0, BUTTON.1, Arrival::Queue, true, || false)
                .unwrap();
            command(&closing, 2, Arrival::ThisThread, message);
            closing.after(at(3), pending, || false);
            assert_eq!(closing.open(at(4)), Some(false), "{message:?}");
        }
        // A close command another thread sent while the Operator's gesture was handled.
        let closing = Closing::default();
        let pending = closing
            .before(at(1), BUTTON.0, BUTTON.1, Arrival::Queue, true, || false)
            .unwrap();
        command(&closing, 2, Arrival::OtherThread, Arrival::ThisThread);
        closing.after(at(3), pending, || false);
        assert_eq!(closing.open(at(4)), Some(false));
        // A gesture another program's input made, or another press: no gesture.
        let closing = Closing::default();
        let queue = Cell::new(false);
        let other = closing.before(at(1), ALT_F4.0, ALT_F4.1, Arrival::Queue, false, || false);
        assert!(other.is_none(), "not the Operator's");
        let pending = closing
            .before(at(1), WM_KEYDOWN, 0x41, Arrival::Queue, true, || panic!("no look for a key"))
            .unwrap();
        queue.set(true);
        closing.after(at(2), pending, || panic!("no look after a key"));
        taken_out(&closing, &queue, 3);
        assert_eq!(closing.open(at(4)), Some(false));
    }

    /// MT-214 E4-F2: a gesture whose handling made no command (a press on the close button dragged off it, F4 without
    /// Alt) lends nothing to a close command another program posts right after it, nor to one taken out of the queue
    /// while it is handled. A close command that waited in the queue before the gesture is not the gesture's either.
    #[test]
    fn a_gesture_that_made_no_command_approves_no_close_that_follows() {
        for made in [BUTTON, ALT_F4] {
            let closing = Closing::default();
            let queue = Cell::new(false);
            gesture(&closing, made, Handling::Nothing, &queue);
            queue.set(true);
            taken_out(&closing, &queue, 4);
            assert_eq!(closing.open(at(5)), Some(false), "posted after: {made:?}");
        }
        let closing = Closing::default();
        let pending = closing
            .before(at(1), BUTTON.0, BUTTON.1, Arrival::Queue, true, || false)
            .unwrap();
        // The tracking of the close button takes another program's close command out of the queue.
        command(&closing, 2, Arrival::Queue, Arrival::ThisThread);
        closing.after(at(3), pending, || false);
        assert_eq!(closing.open(at(4)), Some(false), "taken out inside the handling");
        // Queued before the gesture, and the system posts its own behind it: the first is taken out first.
        let closing = Closing::default();
        let queue = Cell::new(true);
        gesture(&closing, ALT_F4, Handling::Posts, &queue);
        taken_out(&closing, &queue, 4);
        assert_eq!(closing.open(at(5)), Some(false), "the command queued before");
    }

    /// MT-214 E5-F4: only Alt+F4's handling may post its close command. A close command another program posts while the
    /// close button is held (its handling lasts the whole hold, which that program can watch) or while a double-click on
    /// the system menu is handled, whose handling made none, is nobody's; the queue is not even looked at for them.
    #[test]
    fn a_close_posted_while_a_button_gesture_is_handled_is_nobodys() {
        const SYSTEM_MENU: (u32, WPARAM) = (WM_NCLBUTTONDBLCLK, HTSYSMENU as WPARAM);
        for made in [BUTTON, SYSTEM_MENU] {
            let closing = Closing::default();
            let queue = Cell::new(false);
            let pending = closing
                .before(at(1), made.0, made.1, Arrival::Queue, true, || {
                    panic!("the queue looked at before {made:?}")
                })
                .expect("a press");
            // Another program posts a close command while the gesture is handled; the handling makes none.
            queue.set(true);
            closing.after(at(3), pending, || panic!("the queue looked at after {made:?}"));
            taken_out(&closing, &queue, 4);
            assert_eq!(closing.open(at(5)), Some(false), "posted during the handling: {made:?}");
        }
        // Alt+F4's own posted command still counts.
        let closing = Closing::default();
        let queue = Cell::new(false);
        gesture(&closing, ALT_F4, Handling::Posts, &queue);
        taken_out(&closing, &queue, 4);
        assert_eq!(closing.open(at(5)), Some(true));
    }

    #[test]
    fn anybody_elses_request_leaves_the_operators_open_and_requests_end() {
        let closing = Closing::default();
        let queue = Cell::new(false);
        gesture(&closing, BUTTON, Handling::Sends, &queue);
        closing.before(at(5), WM_CLOSE, 0, Arrival::Queue, false, || false);
        assert_eq!(closing.open(at(6)), Some(true), "not overtaken");
        let lifetime = u64::try_from(OPERATOR_AUTHORITY_LIFETIME.as_millis()).unwrap();
        assert_eq!(closing.open(at(2 + lifetime)), Some(true));
        assert_eq!(closing.open(at(3 + lifetime)), None, "ended with its lifetime");
        closing.before(at(10 + lifetime), WM_CLOSE, 0, Arrival::Queue, false, || false);
        assert_eq!(closing.open(at(11 + lifetime)), Some(false));
        assert_eq!(press(WM_NCLBUTTONDBLCLK, HTSYSMENU as usize), Some(true));
        assert_eq!(press(0x0200, 0), None, "a pointer move is no press");
    }
}
