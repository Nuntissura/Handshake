//! Who may move focus, and where a window goes in the Z order. Decisions only; `platform::place` carries them out.

use crate::command::{Refusal, WindowClass};
use crate::origin::Authority;

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

/// ACTIVATION_AFTER_REMOVAL (MT-214 R2-N5, proposed for the Operator to confirm): where the keyboard goes when a window
/// that held the activation is minimised, hidden or destroyed with the Operator's approval (the Operator's own command,
/// or an open request to close it) and the activation went nowhere. To the window's owner, else to the topmost other
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
