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

pub(crate) fn plan_show(
    class: WindowClass,
    activate: bool,
    authority: Authority,
    foreground: Foreground,
) -> Result<ShowPlan, Refusal> {
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
                    plan_show(WindowClass::Quiet, true, authority, foreground),
                    Err(Refusal::QuietWindow)
                );
                assert_eq!(
                    plan_show(WindowClass::Quiet, false, authority, foreground),
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
                let shown = plan_show(WindowClass::Operator, true, authority, foreground);
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
            let shown = |authority| plan_show(WindowClass::Operator, true, authority, foreground);
            assert!(shown(Authority::OwnForeground).is_ok());
            assert_eq!(shown(Authority::Host), Err(Refusal::NeedsOperator));
            assert_eq!(shown(Authority::Model), Err(Refusal::NeedsOperator));
        }
    }

    #[test]
    fn without_activation_no_window_is_put_above_the_foreground() {
        for authority in AUTHORITIES {
            let place = |foreground| {
                plan_show(WindowClass::Operator, false, authority, foreground)
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
