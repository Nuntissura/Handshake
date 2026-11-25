//! Who caused something, and when.
//!
//! An [`InputStamp`] of the Operator's is made in one place: where the event loop took an input message from the
//! operating system. A command gets the Operator's authority from such a stamp and nowhere else, so a model or the
//! shell itself cannot claim it.

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

/// How long after the Operator's input a command may still carry the Operator's authority.
pub const OPERATOR_AUTHORITY_LIFETIME: Duration = Duration::from_secs(5);

/// How long after the loop was created the Operator's launch of the process may still show its first window in front.
/// Longer than input, because opening the GPU comes between. The operating system refuses it anyway once the Operator
/// works elsewhere.
pub const LAUNCH_AUTHORITY_LIFETIME: Duration = Duration::from_secs(60);

/// Who issued a command, and the issuer's number for it.
#[derive(Debug)]
pub struct CommandSource {
    id: u64,
    kind: SourceKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceKind {
    OperatorInput(EventTime),
    OperatorLaunch(EventTime),
    OtherProgram,
    Host,
    Model(Attribution),
}

impl CommandSource {
    /// A command that answers an input. It has the Operator's authority only when that input was the Operator's, and
    /// only for [`OPERATOR_AUTHORITY_LIFETIME`].
    pub fn from_input(stamp: &InputStamp, id: u64) -> Self {
        let kind = match stamp.origin {
            Origin::OperatorDevice => SourceKind::OperatorInput(stamp.received),
            Origin::ModelAction(attribution) => SourceKind::Model(attribution),
            Origin::OtherProgram | Origin::Unattributed => SourceKind::OtherProgram,
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

    pub(crate) fn authority(&self, now: EventTime) -> Authority {
        let while_recent = |at, lifetime| {
            if now.since(at) <= lifetime {
                Authority::Operator
            } else {
                Authority::Host
            }
        };
        match self.kind {
            SourceKind::OperatorInput(at) => while_recent(at, OPERATOR_AUTHORITY_LIFETIME),
            SourceKind::OperatorLaunch(at) => while_recent(at, LAUNCH_AUTHORITY_LIFETIME),
            SourceKind::OtherProgram => Authority::OwnForeground,
            SourceKind::Host => Authority::Host,
            SourceKind::Model(_) => Authority::Model,
        }
    }
}

/// What a command may do to focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Authority {
    /// Anything the Operator may.
    Operator,
    /// Activate a window only while one of ours is in the foreground.
    OwnForeground,
    /// Never activate.
    Host,
    /// Never activate, and never take the foreground window away.
    Model,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(nanos: u64) -> EventTime {
        EventTime { nanos }
    }

    fn stamp(origin: Origin, received: u64) -> InputStamp {
        InputStamp {
            received: at(received),
            os_millis: None,
            origin,
            device: DeviceKind::Unknown,
            modifiers: Modifiers::default(),
        }
    }

    const MODEL: Attribution = Attribution {
        actor: 7,
        session: 8,
        correlation: 9,
    };

    #[test]
    fn only_the_operators_own_input_carries_the_operators_authority() {
        let authority = |origin| CommandSource::from_input(&stamp(origin, 10), 1).authority(at(10));
        assert_eq!(authority(Origin::OperatorDevice), Authority::Operator);
        assert_eq!(authority(Origin::OtherProgram), Authority::OwnForeground);
        assert_eq!(authority(Origin::Unattributed), Authority::OwnForeground);
        assert_eq!(authority(Origin::ModelAction(MODEL)), Authority::Model);
        assert_eq!(CommandSource::host(1).authority(at(10)), Authority::Host);
        assert_eq!(
            CommandSource::model(MODEL, 1).authority(at(10)),
            Authority::Model
        );
    }

    #[test]
    fn the_operators_authority_ends_with_its_lifetime() {
        let lifetime = u64::try_from(OPERATOR_AUTHORITY_LIFETIME.as_nanos()).unwrap();
        let source = CommandSource::from_input(&stamp(Origin::OperatorDevice, 100), 1);
        assert_eq!(source.authority(at(100 + lifetime)), Authority::Operator);
        assert_eq!(source.authority(at(101 + lifetime)), Authority::Host);
    }

    #[test]
    fn the_launch_authority_ends_too() {
        let lifetime = u64::try_from(LAUNCH_AUTHORITY_LIFETIME.as_nanos()).unwrap();
        let source = CommandSource::operator_launch(at(0), 1);
        assert_eq!(source.authority(at(lifetime)), Authority::Operator);
        assert_eq!(source.authority(at(lifetime + 1)), Authority::Host);
    }

    #[test]
    fn a_stamp_a_caller_can_make_is_never_the_operators() {
        let made = InputStamp::model_action(MODEL, at(5));
        assert_eq!(made.origin(), Origin::ModelAction(MODEL));
        assert_eq!(
            CommandSource::from_input(&made, 1).authority(at(5)),
            Authority::Model
        );
    }
}
