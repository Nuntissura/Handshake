//! Who may move focus, and where a window goes in the Z order. Decisions only; `platform::place` carries them out.

use std::time::Duration;

use crate::command::{Refusal, WindowClass};
use crate::origin::{Authority, EventTime};

/// How long after the handling of the Operator's close gesture posted its close command that command may come out of the
/// queue (MT-214 E3-F6, E4-F2). A posted command comes out of the queue ahead of input (PeekMessage, Remarks).
pub(crate) const CLOSE_GESTURE_LIFETIME: Duration = Duration::from_secs(1);

/// How a close command reached the window procedure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandFrom {
    /// Taken out of the thread's queue: posted, by anybody.
    Queue,
    /// Sent by this thread, or by the system on its behalf.
    ThisThread,
    /// Sent by another thread.
    OtherThread,
}

/// Whether a request to close a window is the Operator's (MT-214 E3-F6, E4-F2). Only the Operator's own close gesture on
/// the window makes one: Alt+F4, its close button or a double-click on its system menu, which the operating system
/// reported as hardware and the loop took out of its queue. The system turns the gesture into a close command
/// (`WM_SYSCOMMAND`, `SC_CLOSE`) while it handles the gesture's own message, and the command into the close message
/// (`WM_CLOSE`), sent from within it on the same thread (so Wine's default window procedure; should Windows' post it,
/// the Operator's close would count as anybody's, which only withholds the hand-on).
///
/// The command counts only when it is the one the system made of the gesture (E4-F2): sent by this thread while the
/// gesture's message is being handled, or the first one taken out of the queue after the handling of Alt+F4 posted one,
/// when none was queued before it (only Alt+F4's may be posted, E5-F4; `platform::closing`). A gesture whose handling
/// made no command, a press on the close button dragged off it or F4 without Alt, approves nothing, and a close command
/// another program posts after its handling, or during it for any gesture but Alt+F4, is nobody's (one posted within the
/// brief handling of an Alt+F4 that made none is the residual case). A close message
/// posted or sent by another program, or by this one, and a close command another thread sends, are nobody's approval;
/// such a request still lets a model remove the window once, and hands nothing on.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CloseWatch {
    /// The message of the Operator's close gesture is being handled.
    handling: bool,
    /// The handling of the Operator's close gesture posted a close command, then: the next one out of the queue is it.
    posted: Option<EventTime>,
    /// A close command that the Operator's gesture made is being carried out.
    closing: bool,
}

impl CloseWatch {
    /// The handling of the Operator's own press of a key or button, taken out of the queue, begins. Any press ends what
    /// an earlier gesture left. Returns what [`CloseWatch::press_ends`] puts back.
    pub(crate) fn press_begins(&mut self, close_gesture: bool) -> bool {
        self.posted = None;
        std::mem::replace(&mut self.handling, close_gesture)
    }

    /// The handling of the press ended.
    pub(crate) fn press_ends(&mut self, before: bool) {
        self.handling = before;
    }

    /// The handling of the Operator's close gesture just ended, and a close command for the window waits in the queue
    /// now that did not before it: the system posted it for the gesture.
    pub(crate) fn command_posted(&mut self, now: EventTime) {
        self.posted = Some(now);
    }

    /// A close command begins. Returns what [`CloseWatch::command_ends`] puts back.
    pub(crate) fn command_begins(&mut self, now: EventTime, from: CommandFrom) -> bool {
        let before = self.closing;
        self.closing = match from {
            CommandFrom::ThisThread => self.handling,
            CommandFrom::Queue => self
                .posted
                .take()
                .is_some_and(|at| now.since(at) <= CLOSE_GESTURE_LIFETIME),
            CommandFrom::OtherThread => false,
        };
        before
    }

    pub(crate) fn command_ends(&mut self, before: bool) {
        self.closing = before;
    }

    /// A close message: whether it is the Operator's. `sent_by_this_thread`: neither posted nor sent from another
    /// thread, as the system sends it from within a close command.
    pub(crate) fn message_is_operators(&self, sent_by_this_thread: bool) -> bool {
        self.closing && sent_by_this_thread
    }
}

/// Whether the Operator approved a removal (ACTIVATION_AFTER_REMOVAL): the Operator's own command, or an open request to
/// close the window that the Operator's own close gesture made (`close`: an open request, and whether it is one).
pub(crate) fn removal_approved(authority: Authority, close: Option<bool>) -> bool {
    authority == Authority::Operator || close == Some(true)
}

/// What is in the foreground, seen from one window of ours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Foreground {
    /// Nothing.
    Nothing,
    /// This window, or a window it owns: the Operator works in it.
    This,
    /// Another window of this process.
    OurOther,
    /// Another program's window.
    Other {
        /// It is in the topmost band, where a window put behind it would become topmost itself.
        topmost: bool,
    },
}

/// Where a window goes in the Z order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ZPlace {
    /// Where it is.
    Keep,
    /// Below every other window.
    Bottom,
    /// Right behind the foreground window.
    BehindForeground,
}

/// How to show a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShowPlan {
    pub(crate) activate: bool,
    pub(crate) place: ZPlace,
}

/// How to minimise a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MinimizePlan {
    /// The operating system activates the next window.
    ActivateNext,
    /// No window is activated.
    ActivateNothing,
}

/// `resizes`: the show ends a minimised or maximised state, so it changes the window's size and position. That needs
/// the authority a move or a resize needs (MT-214 R2-N1).
pub(crate) fn plan_show(
    class: WindowClass,
    activate: bool,
    authority: Authority,
    foreground: Foreground,
    resizes: bool,
) -> Result<ShowPlan, Refusal> {
    if resizes {
        may_change_geometry(authority, foreground)?;
    }
    if activate {
        if class == WindowClass::Quiet {
            return Err(Refusal::QuietWindow);
        }
        let ours = matches!(foreground, Foreground::This | Foreground::OurOther);
        return match authority {
            Authority::Operator | Authority::Launch => Ok(()),
            Authority::OwnForeground if ours => Ok(()),
            Authority::OwnForeground | Authority::Host | Authority::Model => {
                Err(Refusal::NeedsOperator)
            }
        }
        .map(|()| ShowPlan {
            activate: true,
            place: ZPlace::Keep,
        });
    }
    let place = match (class, foreground) {
        (WindowClass::Quiet, _) => ZPlace::Bottom,
        (WindowClass::Operator, Foreground::This) => ZPlace::Keep,
        (WindowClass::Operator, Foreground::OurOther | Foreground::Other { topmost: false }) => {
            ZPlace::BehindForeground
        }
        (WindowClass::Operator, Foreground::Nothing | Foreground::Other { topmost: true }) => {
            ZPlace::Bottom
        }
    };
    Ok(ShowPlan {
        activate: false,
        place,
    })
}

pub(crate) fn plan_minimize(
    class: WindowClass,
    authority: Authority,
    foreground: Foreground,
) -> Result<MinimizePlan, Refusal> {
    match (class, authority) {
        (WindowClass::Quiet, _) => Ok(MinimizePlan::ActivateNothing),
        (WindowClass::Operator, Authority::Operator) => Ok(MinimizePlan::ActivateNext),
        (WindowClass::Operator, Authority::Model) if foreground == Foreground::This => {
            Err(Refusal::WouldMoveFocus)
        }
        (WindowClass::Operator, _) => Ok(MinimizePlan::ActivateNothing),
    }
}

/// Whether a window may be hidden or destroyed. A model may not take away the window the Operator works in, unless
/// somebody asked to close it and that request is still open.
pub(crate) fn may_remove(
    authority: Authority,
    foreground: Foreground,
    close_requested: bool,
) -> Result<(), Refusal> {
    if authority == Authority::Model && foreground == Foreground::This && !close_requested {
        Err(Refusal::WouldMoveFocus)
    } else {
        Ok(())
    }
}

/// Whether a window may be moved or sized. A model may not do it to the window the Operator works in.
pub(crate) fn may_change_geometry(
    authority: Authority,
    foreground: Foreground,
) -> Result<(), Refusal> {
    if authority == Authority::Model && foreground == Foreground::This {
        Err(Refusal::OperatorsWindow)
    } else {
        Ok(())
    }
}

/// ACTIVATION_AFTER_REMOVAL (MT-214 R2-N5, confirmed by the Operator 2026-11-26: "a model cannot trigger it"): where
/// the keyboard goes when a window that held the activation is minimised, hidden or destroyed with the Operator's
/// approval ([`removal_approved`]) and the activation went nowhere. To the window's owner, else to the topmost other
/// window of this loop that the Operator may work in, else nowhere; and never while another program is in the
/// foreground, so a window the system gave the activation to keeps it. The caller passes only windows that may take
/// it: of the Operator's class, shown, not minimised, enabled. Without the approval nothing is handed on.
pub(crate) fn activation_after_removal<W>(
    approved: bool,
    activation_lost: bool,
    other_program_in_front: bool,
    owner: Option<W>,
    top_down: impl IntoIterator<Item = W>,
) -> Option<W> {
    if !approved || !activation_lost || other_program_in_front {
        return None;
    }
    owner.or_else(|| top_down.into_iter().next())
}

/// Whether the clipboard may be replaced. It belongs to the Operator; the launch of the process is no request to
/// replace it.
pub(crate) fn may_write_clipboard(
    authority: Authority,
    foreground: Foreground,
) -> Result<(), Refusal> {
    let ours = matches!(foreground, Foreground::This | Foreground::OurOther);
    match authority {
        Authority::Operator => Ok(()),
        Authority::OwnForeground if ours => Ok(()),
        Authority::Launch | Authority::OwnForeground | Authority::Host | Authority::Model => {
            Err(Refusal::NeedsOperator)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTHORITIES: [Authority; 5] = [
        Authority::Operator,
        Authority::Launch,
        Authority::OwnForeground,
        Authority::Host,
        Authority::Model,
    ];
    const FOREGROUNDS: [Foreground; 5] = [
        Foreground::Nothing,
        Foreground::This,
        Foreground::OurOther,
        Foreground::Other { topmost: false },
        Foreground::Other { topmost: true },
    ];

    #[test]
    fn a_quiet_window_is_never_activated_and_always_goes_to_the_bottom() {
        for authority in AUTHORITIES {
            for foreground in FOREGROUNDS {
                assert_eq!(
                    plan_show(WindowClass::Quiet, true, authority, foreground, false),
                    Err(Refusal::QuietWindow)
                );
                assert_eq!(
                    plan_show(WindowClass::Quiet, false, authority, foreground, false),
                    Ok(ShowPlan {
                        activate: false,
                        place: ZPlace::Bottom
                    })
                );
                assert_eq!(
                    plan_minimize(WindowClass::Quiet, authority, foreground),
                    Ok(MinimizePlan::ActivateNothing)
                );
            }
        }
    }

    #[test]
    fn only_the_operator_activates_over_another_program() {
        for foreground in [
            Foreground::Nothing,
            Foreground::Other { topmost: false },
            Foreground::Other { topmost: true },
        ] {
            for authority in AUTHORITIES {
                let shown = plan_show(WindowClass::Operator, true, authority, foreground, false);
                assert_eq!(
                    shown.is_ok(),
                    authority.is_operators(),
                    "{authority:?} {foreground:?}"
                );
            }
        }
    }

    #[test]
    fn another_programs_input_activates_only_among_our_own_windows() {
        for foreground in [Foreground::This, Foreground::OurOther] {
            let shown = |authority| plan_show(WindowClass::Operator, true, authority, foreground, false);
            assert!(shown(Authority::OwnForeground).is_ok());
            assert_eq!(shown(Authority::Host), Err(Refusal::NeedsOperator));
            assert_eq!(shown(Authority::Model), Err(Refusal::NeedsOperator));
        }
    }

    #[test]
    fn without_activation_no_window_is_put_above_the_foreground() {
        for authority in AUTHORITIES {
            let place = |foreground| {
                plan_show(WindowClass::Operator, false, authority, foreground, false)
                    .unwrap()
                    .place
            };
            assert_eq!(place(Foreground::Nothing), ZPlace::Bottom);
            assert_eq!(place(Foreground::This), ZPlace::Keep);
            assert_eq!(place(Foreground::OurOther), ZPlace::BehindForeground);
            assert_eq!(
                place(Foreground::Other { topmost: false }),
                ZPlace::BehindForeground
            );
            // Behind a topmost window would still be above every ordinary one.
            assert_eq!(place(Foreground::Other { topmost: true }), ZPlace::Bottom);
        }
    }

    #[test]
    fn minimising_hands_the_focus_on_only_for_the_operator() {
        for foreground in FOREGROUNDS {
            let plan = |authority| plan_minimize(WindowClass::Operator, authority, foreground);
            assert_eq!(plan(Authority::Operator), Ok(MinimizePlan::ActivateNext));
            assert_eq!(plan(Authority::Host), Ok(MinimizePlan::ActivateNothing));
            assert_eq!(
                plan(Authority::Launch),
                Ok(MinimizePlan::ActivateNothing),
                "the launch activates, it does not hand on"
            );
        }
        assert_eq!(
            plan_minimize(WindowClass::Operator, Authority::Model, Foreground::This),
            Err(Refusal::WouldMoveFocus)
        );
        assert_eq!(
            plan_minimize(
                WindowClass::Operator,
                Authority::Model,
                Foreground::OurOther
            ),
            Ok(MinimizePlan::ActivateNothing)
        );
    }

    #[test]
    fn a_model_removes_the_operators_window_only_after_a_close_request() {
        assert_eq!(
            may_remove(Authority::Model, Foreground::This, false),
            Err(Refusal::WouldMoveFocus)
        );
        assert_eq!(may_remove(Authority::Model, Foreground::This, true), Ok(()));
        assert_eq!(
            may_remove(Authority::Model, Foreground::OurOther, false),
            Ok(())
        );
        assert_eq!(may_remove(Authority::Host, Foreground::This, false), Ok(()));
    }

    #[test]
    fn a_model_neither_moves_nor_sizes_the_operators_window() {
        for authority in AUTHORITIES {
            for foreground in FOREGROUNDS {
                let refused = authority == Authority::Model && foreground == Foreground::This;
                assert_eq!(
                    may_change_geometry(authority, foreground).is_err(),
                    refused,
                    "{authority:?} {foreground:?}"
                );
            }
        }
        assert_eq!(
            may_change_geometry(Authority::Model, Foreground::This),
            Err(Refusal::OperatorsWindow)
        );
    }

    /// MT-214 R2-N1: a restore, or a show that ends a minimised state, is geometry.
    #[test]
    fn a_model_neither_restores_nor_unminimises_the_operators_window() {
        for authority in AUTHORITIES {
            for foreground in FOREGROUNDS {
                let refused = authority == Authority::Model && foreground == Foreground::This;
                for activate in [false, true] {
                    let resized = plan_show(WindowClass::Operator, activate, authority, foreground, true);
                    if refused {
                        assert_eq!(resized, Err(Refusal::OperatorsWindow), "{activate}");
                    } else {
                        let kept = plan_show(WindowClass::Operator, activate, authority, foreground, false);
                        assert_eq!(resized, kept, "{authority:?} {foreground:?} {activate}");
                    }
                }
            }
        }
        let shown = plan_show(WindowClass::Operator, false, Authority::Model, Foreground::This, false);
        assert!(shown.is_ok(), "a show that keeps the size is no geometry");
    }

    /// MT-214 R2-N5.
    #[test]
    fn the_keyboard_goes_to_the_owner_then_the_topmost_operator_window_and_never_from_another_program() {
        let after = |approved, lost, other, owner, rest: &[u8]| {
            activation_after_removal(approved, lost, other, owner, rest.iter().copied())
        };
        assert_eq!(after(true, true, false, Some(1), &[2, 3]), Some(1), "the owner first");
        assert_eq!(after(true, true, false, None, &[2, 3]), Some(2), "then the topmost");
        assert_eq!(after(true, true, false, None, &[]), None, "else nowhere");
        assert_eq!(after(false, true, false, Some(1), &[2]), None, "not approved");
        assert_eq!(after(true, false, false, Some(1), &[2]), None, "the activation is where it was");
        assert_eq!(after(true, true, true, Some(1), &[2]), None, "another program is in front");
    }

    fn at(millis: u64) -> EventTime {
        EventTime {
            nanos: millis * 1_000_000,
        }
    }

    /// A close command and the close message inside it.
    fn close(watch: &mut CloseWatch, command_at: u64, from: CommandFrom, message_same_thread: bool) -> bool {
        let before = watch.command_begins(at(command_at), from);
        let operators = watch.message_is_operators(message_same_thread);
        watch.command_ends(before);
        operators
    }

    /// The Operator's close gesture whose handling sends the close command, as the close button and a double-click on
    /// the system menu do: the command, and the message inside it, inside the gesture's handling.
    fn sent_inside(watch: &mut CloseWatch, at_millis: u64, message_same_thread: bool) -> bool {
        let before = watch.press_begins(true);
        let operators = close(watch, at_millis, CommandFrom::ThisThread, message_same_thread);
        watch.press_ends(before);
        operators
    }

    /// The Operator's close gesture whose handling posts the close command, as Alt+F4 in Wine.
    fn posted_by(watch: &mut CloseWatch, at_millis: u64) {
        let before = watch.press_begins(true);
        watch.press_ends(before);
        watch.command_posted(at(at_millis));
    }

    /// MT-214 E3-F6: only the Operator's own close gesture makes a close request that approves the hand-on.
    #[test]
    fn a_close_is_the_operators_only_when_the_operators_own_close_gesture_made_it() {
        let mut watch = CloseWatch::default();
        assert!(!close(&mut watch, 0, CommandFrom::ThisThread, true), "no gesture: a close of anybody's");
        assert!(sent_inside(&mut watch, 10, true), "the close button");
        posted_by(&mut watch, 20);
        assert!(!watch.message_is_operators(true), "a close message outside the command");
        assert!(close(&mut watch, 21, CommandFrom::Queue, true), "Alt+F4, posted");
        assert!(!close(&mut watch, 22, CommandFrom::Queue, true), "the posted command was used");
        // A close command another thread sends is nobody's, inside the gesture's handling too.
        let before = watch.press_begins(true);
        assert!(!close(&mut watch, 30, CommandFrom::OtherThread, true));
        watch.press_ends(before);
        // A close message posted or sent from another thread inside the command is nobody's.
        assert!(!sent_inside(&mut watch, 40, false));
        // A posted command waits its lifetime at most, and any other press of the Operator's ends it.
        let late = u64::try_from(CLOSE_GESTURE_LIFETIME.as_millis()).unwrap();
        posted_by(&mut watch, 100);
        assert!(!close(&mut watch, 101 + late, CommandFrom::Queue, true));
        posted_by(&mut watch, 200);
        let before = watch.press_begins(false);
        watch.press_ends(before);
        assert!(!close(&mut watch, 202, CommandFrom::Queue, true));
        posted_by(&mut watch, 300);
        assert!(close(&mut watch, 300 + late, CommandFrom::Queue, true), "within its lifetime, to the end");
    }

    /// MT-214 E4-F2: a gesture whose handling made no command approves nothing: a press on the close button dragged off
    /// it, F4 without Alt. Neither a close command another program posts after it, at once, nor one taken out of the
    /// queue while the gesture is handled.
    #[test]
    fn a_gesture_that_made_no_command_lends_nothing_to_anybody_elses() {
        let mut watch = CloseWatch::default();
        let before = watch.press_begins(true);
        assert!(!close(&mut watch, 1, CommandFrom::Queue, true), "posted by another program, taken out inside");
        watch.press_ends(before);
        assert!(!close(&mut watch, 2, CommandFrom::Queue, true), "posted by another program, right after");
        assert!(!close(&mut watch, 3, CommandFrom::ThisThread, true), "sent after the handling");
        // A press that is no close gesture, and its handling sends one: nobody's either.
        let before = watch.press_begins(false);
        assert!(!close(&mut watch, 4, CommandFrom::ThisThread, true));
        watch.press_ends(before);
    }

    #[test]
    fn a_close_command_inside_another_puts_back_what_was_there() {
        let mut watch = CloseWatch::default();
        let press = watch.press_begins(true);
        let outer = watch.command_begins(at(1), CommandFrom::ThisThread);
        let inner = watch.command_begins(at(2), CommandFrom::OtherThread);
        assert!(!watch.message_is_operators(true), "inside the inner one");
        watch.command_ends(inner);
        assert!(watch.message_is_operators(true), "the outer one again");
        watch.command_ends(outer);
        assert!(!watch.message_is_operators(true));
        // A press inside the gesture's handling, and back.
        let nested = watch.press_begins(false);
        assert!(!close(&mut watch, 3, CommandFrom::ThisThread, true));
        watch.press_ends(nested);
        assert!(close(&mut watch, 4, CommandFrom::ThisThread, true));
        watch.press_ends(press);
    }

    #[test]
    fn a_request_of_anybody_else_lets_a_model_remove_the_window_and_approves_nothing() {
        assert_eq!(may_remove(Authority::Model, Foreground::This, true), Ok(()));
        assert!(!removal_approved(Authority::Model, Some(false)));
        assert!(!removal_approved(Authority::Host, Some(false)));
        assert!(!removal_approved(Authority::Model, None));
        assert!(removal_approved(Authority::Model, Some(true)), "the Operator asked to close it");
        assert!(removal_approved(Authority::Operator, None));
        assert!(!removal_approved(Authority::Launch, None), "the launch removes nothing");
    }

    #[test]
    fn only_the_operator_replaces_the_clipboard() {
        for foreground in FOREGROUNDS {
            assert_eq!(may_write_clipboard(Authority::Operator, foreground), Ok(()));
            for authority in [Authority::Launch, Authority::Host, Authority::Model] {
                assert_eq!(
                    may_write_clipboard(authority, foreground),
                    Err(Refusal::NeedsOperator)
                );
            }
        }
        let foreign = Foreground::Other { topmost: false };
        assert!(may_write_clipboard(Authority::OwnForeground, foreign).is_err());
        assert!(may_write_clipboard(Authority::OwnForeground, Foreground::OurOther).is_ok());
    }
}
