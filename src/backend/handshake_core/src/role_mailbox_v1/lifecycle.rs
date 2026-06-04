//! MT-176 ThreadLifecycleState + MessageDeliveryState transition matrices.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadLifecycleState {
    Open,
    AwaitingResponse,
    WaitingOnLinkedAuthority,
    Escalated,
    Resolved,
    Expired,
    Archived,
}

impl ThreadLifecycleState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Resolved | Self::Expired | Self::Archived)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::AwaitingResponse => "awaiting_response",
            Self::WaitingOnLinkedAuthority => "waiting_on_linked_authority",
            Self::Escalated => "escalated",
            Self::Resolved => "resolved",
            Self::Expired => "expired",
            Self::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDeliveryState {
    Queued,
    Delivered,
    Acknowledged,
    Replied,
    Ignored,
    Failed,
    DeadLettered,
}

impl MessageDeliveryState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Acknowledged | Self::Replied | Self::Ignored | Self::Failed | Self::DeadLettered
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Delivered => "delivered",
            Self::Acknowledged => "acknowledged",
            Self::Replied => "replied",
            Self::Ignored => "ignored",
            Self::Failed => "failed",
            Self::DeadLettered => "dead_lettered",
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("invalid transition from {from} to {to}")]
pub struct InvalidTransition {
    pub from: String,
    pub to: String,
}

/// Allowed thread lifecycle transitions per spec v02.186 §02-system-architecture.md.
pub fn transition_thread_state(
    current: ThreadLifecycleState,
    requested: ThreadLifecycleState,
) -> Result<ThreadLifecycleState, InvalidTransition> {
    use ThreadLifecycleState::*;
    let allowed = match (current, requested) {
        // No-op self transitions allowed only for non-terminal idempotent updates.
        (a, b) if a == b && !a.is_terminal() => true,
        (Open, AwaitingResponse) => true,
        (Open, WaitingOnLinkedAuthority) => true,
        (Open, Escalated) => true,
        (Open, Resolved) => true,
        (Open, Expired) => true,
        (Open, Archived) => true,
        (AwaitingResponse, Open) => true,
        (AwaitingResponse, WaitingOnLinkedAuthority) => true,
        (AwaitingResponse, Escalated) => true,
        (AwaitingResponse, Resolved) => true,
        (AwaitingResponse, Expired) => true,
        (AwaitingResponse, Archived) => true,
        (WaitingOnLinkedAuthority, Open) => true,
        (WaitingOnLinkedAuthority, AwaitingResponse) => true,
        (WaitingOnLinkedAuthority, Escalated) => true,
        (WaitingOnLinkedAuthority, Resolved) => true,
        (WaitingOnLinkedAuthority, Expired) => true,
        (WaitingOnLinkedAuthority, Archived) => true,
        (Escalated, Resolved) => true,
        (Escalated, Expired) => true,
        (Escalated, Archived) => true,
        // Resolved/Expired may be archived (record retention).
        (Resolved, Archived) => true,
        (Expired, Archived) => true,
        // Everything else is illegal — terminal -> non-terminal forbidden.
        _ => false,
    };
    if allowed {
        Ok(requested)
    } else {
        Err(InvalidTransition {
            from: current.as_str().to_string(),
            to: requested.as_str().to_string(),
        })
    }
}

pub fn transition_message_state(
    current: MessageDeliveryState,
    requested: MessageDeliveryState,
) -> Result<MessageDeliveryState, InvalidTransition> {
    use MessageDeliveryState::*;
    let allowed = match (current, requested) {
        (a, b) if a == b && !a.is_terminal() => true,
        (Queued, Delivered) => true,
        (Queued, Failed) => true,
        (Queued, DeadLettered) => true,
        (Delivered, Acknowledged) => true,
        (Delivered, Replied) => true,
        (Delivered, Ignored) => true,
        (Delivered, Failed) => true,
        (Delivered, DeadLettered) => true,
        (Acknowledged, Replied) => true,
        // Replied/Ignored/Failed/Acknowledged terminal except for explicit dead-letter audit.
        (Replied, DeadLettered) => false,
        (Failed, DeadLettered) => true,
        _ => false,
    };
    if allowed {
        Ok(requested)
    } else {
        Err(InvalidTransition {
            from: current.as_str().to_string(),
            to: requested.as_str().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_to_resolved_allowed() {
        let r = transition_thread_state(ThreadLifecycleState::Open, ThreadLifecycleState::Resolved);
        assert!(r.is_ok());
    }

    #[test]
    fn archived_to_open_rejected() {
        let r = transition_thread_state(ThreadLifecycleState::Archived, ThreadLifecycleState::Open);
        assert!(r.is_err(), "archived -> open must be rejected per spec");
    }

    #[test]
    fn resolved_to_open_rejected() {
        let r = transition_thread_state(ThreadLifecycleState::Resolved, ThreadLifecycleState::Open);
        assert!(r.is_err());
    }

    #[test]
    fn queued_to_acknowledged_skip_delivered_rejected() {
        // Cannot skip Delivered state.
        let r = transition_message_state(
            MessageDeliveryState::Queued,
            MessageDeliveryState::Acknowledged,
        );
        assert!(r.is_err());
    }

    #[test]
    fn exhaustive_thread_transition_matrix() {
        use ThreadLifecycleState::*;
        let all = [
            Open,
            AwaitingResponse,
            WaitingOnLinkedAuthority,
            Escalated,
            Resolved,
            Expired,
            Archived,
        ];
        let mut ok = 0;
        let mut err = 0;
        for &from in &all {
            for &to in &all {
                match transition_thread_state(from, to) {
                    Ok(_) => ok += 1,
                    Err(_) => err += 1,
                }
            }
        }
        assert_eq!(ok + err, 49);
        // Sanity: there are illegal transitions (terminals -> non-terminals).
        assert!(err > 0);
    }
}
