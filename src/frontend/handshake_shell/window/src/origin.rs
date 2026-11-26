//! Who caused something, and when.
//!
//! An [`InputStamp`] of the Operator's is made in one place: where the event loop took an input message from the
//! operating system. A command gets the Operator's authority from such a stamp and nowhere else, so a model or the
//! shell itself cannot claim it.
//!
//! Each stamp carries the loop's number for its input. The authority is used up by number: the first command that
//! needs it takes it, for that input and every earlier one, so a copied or kept stamp grants nothing a second time
//! (MT-214 W4). The Operator's launch of the process activates one window, once.

use std::time::Duration;

use crate::input::Modifiers;

/// Opaque numbers that say which model session acted. They grant nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Attribution {
    /// The acting model or agent.
    pub actor: u64,
    /// Its session.
    pub session: u64,
    /// The chain of work this belongs to.
    pub correlation: u64,
}

/// Where an input or a command came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Hardware, as the operating system reports it.
    OperatorDevice,
    /// A model, through a steering route. Never through the operating system's input.
    ModelAction(Attribution),
    /// Input another program injected. Assistive tools do this, and so does automation.
    OtherProgram,
    /// The operating system did not say.
    Unattributed,
}

/// The kind of device an input came from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// A keyboard.
    Keyboard,
    /// A mouse.
    Mouse,
    /// A precision touchpad.
    Touchpad,
    /// A touch screen.
    Touch,
    /// A pen.
    Pen,
    /// The operating system did not say.
    #[default]
    Unknown,
}

/// When something happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventTime {
    pub(crate) nanos: u64,
}

impl EventTime {
    /// A time on the clock of the events, for a recorded event sequence or a test. Grants nothing: the Operator's
    /// authority comes only from a stamp the loop made (MT-214 S3).
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Monotonic nanoseconds since the event loop was created.
    pub fn nanos(self) -> u64 {
        self.nanos
    }

    /// How long after `earlier` this is; zero when it is not.
    pub fn since(self, earlier: Self) -> Duration {
        Duration::from_nanos(self.nanos.saturating_sub(earlier.nanos))
    }
}

/// Where and when one input came from, and the modifiers held with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputStamp {
    pub(crate) received: EventTime,
    pub(crate) os_millis: Option<u32>,
    pub(crate) origin: Origin,
    pub(crate) device: DeviceKind,
    pub(crate) modifiers: Modifiers,
    /// The loop's number for the input; 0 for a stamp the loop did not make.
    pub(crate) serial: u64,
}

impl InputStamp {
    /// The stamp of a model's input, for the steering route that delivers one. Grants nothing.
    pub fn model_action(attribution: Attribution, at: EventTime) -> Self {
        Self {
            received: at,
            os_millis: None,
            origin: Origin::ModelAction(attribution),
            device: DeviceKind::Unknown,
            modifiers: Modifiers::default(),
            serial: 0,
        }
    }

    /// When the event loop took it.
    pub fn received(&self) -> EventTime {
        self.received
    }

    /// The operating system's own time of the input, in milliseconds since an arbitrary start. Wraps.
    pub fn os_millis(&self) -> Option<u32> {
        self.os_millis
    }

    /// Where it came from.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// The kind of device it came from.
    pub fn device(&self) -> DeviceKind {
        self.device
    }

    /// The modifiers held with it.
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }
}

/// How long after the Operator's input a command may still carry the Operator's authority. Also how long a request to
/// close a window lets a model remove the window the Operator works in.
pub const OPERATOR_AUTHORITY_LIFETIME: Duration = Duration::from_secs(5);

/// How long after the loop was created the Operator's launch of the process may still show its first window in front.
/// Longer than input, because opening the GPU comes between. The operating system refuses it anyway once the Operator
/// works elsewhere.
pub const LAUNCH_AUTHORITY_LIFETIME: Duration = Duration::from_secs(60);

/// Who issued a command, and the issuer's number for it. Not `Clone`: a command source is made for one command.
#[derive(Debug)]
pub struct CommandSource {
    id: u64,
    kind: SourceKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceKind {
    OperatorInput { at: EventTime, serial: u64 },
    OperatorLaunch(EventTime),
    OtherProgram,
    Host,
    Model(Attribution),
}

impl CommandSource {
    /// A command that answers an input. It has the Operator's authority only when that input was the Operator's, only
    /// for [`OPERATOR_AUTHORITY_LIFETIME`], and only until a command used it: one input, one such command.
    pub fn from_input(stamp: &InputStamp, id: u64) -> Self {
        let kind = match stamp.origin {
            // A stamp without the loop's number was not made by the loop.
            Origin::OperatorDevice if stamp.serial != 0 => SourceKind::OperatorInput {
                at: stamp.received,
                serial: stamp.serial,
            },
            Origin::ModelAction(attribution) => SourceKind::Model(attribution),
            Origin::OperatorDevice | Origin::OtherProgram | Origin::Unattributed => {
                SourceKind::OtherProgram
            }
        };
        Self { id, kind }
    }

    /// A command of the shell itself, for example during start-up or shutdown.
    pub fn host(id: u64) -> Self {
        Self {
            id,
            kind: SourceKind::Host,
        }
    }

    /// A model's command.
    pub fn model(attribution: Attribution, id: u64) -> Self {
        Self {
            id,
            kind: SourceKind::Model(attribution),
        }
    }

    pub(crate) fn operator_launch(at: EventTime, id: u64) -> Self {
        Self {
            id,
            kind: SourceKind::OperatorLaunch(at),
        }
    }

    /// The issuer's number for this command.
    pub fn id(&self) -> u64 {
        self.id
    }
}

/// What a command may do to focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Authority {
    /// Anything the Operator may.
    Operator,
    /// Activate one window, once: the Operator launched the process.
    Launch,
    /// Activate a window only while one of ours is in the foreground.
    OwnForeground,
    /// Never activate.
    Host,
    /// Never activate, and never take, move or size the window the Operator works in.
    Model,
}

impl Authority {
    /// Only the Operator could have done what this authority was used for; using it uses it up.
    pub(crate) fn is_operators(self) -> bool {
        matches!(self, Self::Operator | Self::Launch)
    }
}

/// What the Operator's authority was already used for, in one loop.
#[derive(Debug, Default)]
pub(crate) struct AuthorityLedger {
    /// Every input up to this number granted its authority.
    used_through: u64,
    launch_used: bool,
}

impl AuthorityLedger {
    /// What `source` may do now. Does not use anything up.
    pub(crate) fn authority(&self, source: &CommandSource, now: EventTime) -> Authority {
        let fresh = |at: EventTime, lifetime| now.since(at) <= lifetime;
        match source.kind {
            SourceKind::OperatorInput { at, serial } => {
                if serial > self.used_through && fresh(at, OPERATOR_AUTHORITY_LIFETIME) {
                    Authority::Operator
                } else {
                    Authority::Host
                }
            }
            SourceKind::OperatorLaunch(at) => {
                if !self.launch_used && fresh(at, LAUNCH_AUTHORITY_LIFETIME) {
                    Authority::Launch
                } else {
                    Authority::Host
                }
            }
            SourceKind::OtherProgram => Authority::OwnForeground,
            SourceKind::Host => Authority::Host,
            SourceKind::Model(_) => Authority::Model,
        }
    }

    /// `source` was used for something only the Operator may do. Its input, and every earlier one, grant nothing more.
    pub(crate) fn use_up(&mut self, source: &CommandSource) {
        match source.kind {
            SourceKind::OperatorInput { serial, .. } => {
                self.used_through = self.used_through.max(serial);
            }
            SourceKind::OperatorLaunch(_) => self.launch_used = true,
            SourceKind::OtherProgram | SourceKind::Host | SourceKind::Model(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(nanos: u64) -> EventTime {
        EventTime { nanos }
    }

    fn stamp(origin: Origin, received: u64, serial: u64) -> InputStamp {
        InputStamp {
            received: at(received),
            os_millis: None,
            origin,
            device: DeviceKind::Unknown,
            modifiers: Modifiers::default(),
            serial,
        }
    }

    const MODEL: Attribution = Attribution {
        actor: 7,
        session: 8,
        correlation: 9,
    };

    #[test]
    fn only_the_operators_own_input_carries_the_operators_authority() {
        let ledger = AuthorityLedger::default();
        let authority = |origin| {
            ledger.authority(&CommandSource::from_input(&stamp(origin, 10, 3), 1), at(10))
        };
        assert_eq!(authority(Origin::OperatorDevice), Authority::Operator);
        assert_eq!(authority(Origin::OtherProgram), Authority::OwnForeground);
        assert_eq!(authority(Origin::Unattributed), Authority::OwnForeground);
        assert_eq!(authority(Origin::ModelAction(MODEL)), Authority::Model);
        assert_eq!(
            ledger.authority(&CommandSource::host(1), at(10)),
            Authority::Host
        );
        assert_eq!(
            ledger.authority(&CommandSource::model(MODEL, 1), at(10)),
            Authority::Model
        );
        let unnumbered = CommandSource::from_input(&stamp(Origin::OperatorDevice, 10, 0), 1);
        assert_eq!(
            ledger.authority(&unnumbered, at(10)),
            Authority::OwnForeground,
            "a hardware stamp the loop did not number"
        );
    }

    #[test]
    fn the_operators_authority_ends_with_its_lifetime() {
        let lifetime = u64::try_from(OPERATOR_AUTHORITY_LIFETIME.as_nanos()).unwrap();
        let ledger = AuthorityLedger::default();
        let source = CommandSource::from_input(&stamp(Origin::OperatorDevice, 100, 1), 1);
        assert_eq!(
            ledger.authority(&source, at(100 + lifetime)),
            Authority::Operator
        );
        assert_eq!(
            ledger.authority(&source, at(101 + lifetime)),
            Authority::Host
        );
    }

    #[test]
    fn an_input_grants_the_operators_authority_to_one_command() {
        let mut ledger = AuthorityLedger::default();
        let key = stamp(Origin::OperatorDevice, 10, 5);
        let copy = key;
        let first = CommandSource::from_input(&key, 1);
        assert_eq!(ledger.authority(&first, at(11)), Authority::Operator);
        ledger.use_up(&first);
        assert_eq!(ledger.authority(&first, at(11)), Authority::Host, "used");
        let again = CommandSource::from_input(&copy, 2);
        assert_eq!(
            ledger.authority(&again, at(11)),
            Authority::Host,
            "a copy of the stamp"
        );
        let older = CommandSource::from_input(&stamp(Origin::OperatorDevice, 9, 4), 3);
        assert_eq!(
            ledger.authority(&older, at(11)),
            Authority::Host,
            "an earlier input"
        );
        let newer = CommandSource::from_input(&stamp(Origin::OperatorDevice, 11, 6), 4);
        assert_eq!(ledger.authority(&newer, at(11)), Authority::Operator);
        ledger.use_up(&CommandSource::host(5));
        assert_eq!(
            ledger.authority(&newer, at(11)),
            Authority::Operator,
            "only the Operator's use counts"
        );
    }

    #[test]
    fn the_launch_activates_one_window_once_and_ends_too() {
        let lifetime = u64::try_from(LAUNCH_AUTHORITY_LIFETIME.as_nanos()).unwrap();
        let mut ledger = AuthorityLedger::default();
        let source = CommandSource::operator_launch(at(0), 1);
        assert_eq!(ledger.authority(&source, at(lifetime)), Authority::Launch);
        assert_eq!(ledger.authority(&source, at(lifetime + 1)), Authority::Host);
        ledger.use_up(&source);
        assert_eq!(ledger.authority(&source, at(1)), Authority::Host, "used");
        assert!(Authority::Launch.is_operators() && Authority::Operator.is_operators());
        assert!(!Authority::OwnForeground.is_operators() && !Authority::Model.is_operators());
    }

    #[test]
    fn a_stamp_a_caller_can_make_is_never_the_operators() {
        let made = InputStamp::model_action(MODEL, at(5));
        assert_eq!(made.origin(), Origin::ModelAction(MODEL));
        let ledger = AuthorityLedger::default();
        assert_eq!(
            ledger.authority(&CommandSource::from_input(&made, 1), at(5)),
            Authority::Model
        );
    }
}
