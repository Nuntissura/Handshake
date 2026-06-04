//! Model session lifecycle state machine.
//!
//! Mirrors the spirit of [`crate::kernel::session_broker::SessionRunState`] but
//! is specific to a *model* session in the swarm (load -> generate -> done),
//! with the load/cancel intermediate states the broker's task-level machine
//! does not model. Transitions are validated centrally so an out-of-order
//! transition is a typed error, never a silent overwrite.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelSessionState {
    /// Admitted past the bounds, queued behind the concurrency semaphore.
    Queued,
    /// Permit acquired, factory is loading the model / opening the session.
    Loading,
    /// Loaded and idle, holding a live lease.
    Ready,
    /// Actively producing tokens.
    Generating,
    /// A cancel was requested; unload / teardown is in progress.
    Cancelling,
    /// Terminal: finished normally.
    Completed,
    /// Terminal: ended in error.
    Failed,
    /// Terminal: cancelled (operator, reaper, or cascade).
    Cancelled,
}

impl ModelSessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "QUEUED",
            Self::Loading => "LOADING",
            Self::Ready => "READY",
            Self::Generating => "GENERATING",
            Self::Cancelling => "CANCELLING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// A session counts as "occupying a concurrency slot" while it is between
    /// Loading and Generating inclusive. Used by the concurrency invariant
    /// assertions: at no instant may more than `max_concurrent` sessions be in
    /// one of these states.
    pub fn occupies_slot(self) -> bool {
        matches!(
            self,
            Self::Loading | Self::Ready | Self::Generating | Self::Cancelling
        )
    }

    /// Whether `self -> to` is a legal transition.
    pub fn can_transition(self, to: ModelSessionState) -> bool {
        use ModelSessionState::*;
        matches!(
            (self, to),
            (Queued, Loading)
                | (Queued, Cancelling)
                | (Queued, Cancelled)
                | (Loading, Ready)
                | (Loading, Failed)
                | (Loading, Cancelling)
                | (Ready, Generating)
                | (Ready, Completed)
                | (Ready, Failed)
                | (Ready, Cancelling)
                | (Generating, Ready)
                | (Generating, Completed)
                | (Generating, Failed)
                | (Generating, Cancelling)
                | (Cancelling, Cancelled)
                | (Cancelling, Failed)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_transitions_are_legal() {
        use ModelSessionState::*;
        assert!(Queued.can_transition(Loading));
        assert!(Loading.can_transition(Ready));
        assert!(Ready.can_transition(Generating));
        assert!(Generating.can_transition(Completed));
    }

    #[test]
    fn terminal_states_are_sinks() {
        use ModelSessionState::*;
        for to in [Queued, Loading, Ready, Generating, Cancelling] {
            assert!(!Completed.can_transition(to));
            assert!(!Failed.can_transition(to));
            assert!(!Cancelled.can_transition(to));
        }
    }

    #[test]
    fn cannot_skip_loading() {
        use ModelSessionState::*;
        assert!(!Queued.can_transition(Ready));
        assert!(!Queued.can_transition(Generating));
    }

    #[test]
    fn slot_occupancy_excludes_queued_and_terminal() {
        use ModelSessionState::*;
        assert!(!Queued.occupies_slot());
        assert!(Loading.occupies_slot());
        assert!(Generating.occupies_slot());
        assert!(!Completed.occupies_slot());
        assert!(!Cancelled.occupies_slot());
    }
}
