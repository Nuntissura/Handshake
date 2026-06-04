//! MT-176 RoleMailboxMessage typed primitive (placeholder MessageType replaced
//! by MT-179 typed families which live in `families.rs`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::lifecycle::MessageDeliveryState;
use super::thread::RoleMailboxThreadId;
use crate::role_mailbox::RoleId;

/// Newtype for message identity. Always minted via `Uuid::now_v7()` per
/// HBR-INT-008.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleMailboxMessageId(pub Uuid);

impl RoleMailboxMessageId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for RoleMailboxMessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Placeholder MessageType retained for MT-176 backward-compat. MT-179
/// `MessageFamily` is the typed family hierarchy that supersedes this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    DelegateWork,
    Blocker,
    ReviewRequest,
    DecisionRequest,
    AnnounceBack,
    MicroTaskRequest,
    MicroTaskFeedback,
    MicroTaskVerificationNeeded,
    MicroTaskEscalation,
    MicroTaskCompletionReport,
    /// Reserved escape hatch for forward-compat with future families.
    Other,
}

impl MessageType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DelegateWork => "delegate_work",
            Self::Blocker => "blocker",
            Self::ReviewRequest => "review_request",
            Self::DecisionRequest => "decision_request",
            Self::AnnounceBack => "announce_back",
            Self::MicroTaskRequest => "micro_task_request",
            Self::MicroTaskFeedback => "micro_task_feedback",
            Self::MicroTaskVerificationNeeded => "micro_task_verification_needed",
            Self::MicroTaskEscalation => "micro_task_escalation",
            Self::MicroTaskCompletionReport => "micro_task_completion_report",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedResponseSpec {
    pub by_role: Option<RoleId>,
    pub deadline_utc: Option<DateTime<Utc>>,
    pub kind_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxMessage {
    pub message_id: RoleMailboxMessageId,
    pub thread_id: RoleMailboxThreadId,
    pub message_type: MessageType,
    pub from_role: RoleId,
    pub to_roles: Vec<RoleId>,
    pub expected_response: Option<ExpectedResponseSpec>,
    pub expires_at_utc: Option<DateTime<Utc>>,
    pub delivery_state: MessageDeliveryState,
    pub body: serde_json::Value,
    pub parent_message_id: Option<RoleMailboxMessageId>,
    pub created_at_utc: DateTime<Utc>,
}

impl RoleMailboxMessage {
    pub fn new(
        thread_id: RoleMailboxThreadId,
        message_type: MessageType,
        from_role: RoleId,
        to_roles: Vec<RoleId>,
        body: serde_json::Value,
    ) -> Self {
        Self {
            message_id: RoleMailboxMessageId::new_v7(),
            thread_id,
            message_type,
            from_role,
            to_roles,
            expected_response: None,
            expires_at_utc: None,
            delivery_state: MessageDeliveryState::Queued,
            body,
            parent_message_id: None,
            created_at_utc: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role_mailbox_v1::thread::RoleMailboxThreadId;

    #[test]
    fn message_id_is_v7() {
        let id = RoleMailboxMessageId::new_v7();
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn message_round_trip_serde() {
        let m = RoleMailboxMessage::new(
            RoleMailboxThreadId::new_v7(),
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"task": "do thing"}),
        );
        let s = serde_json::to_string(&m).unwrap();
        let back: RoleMailboxMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }
}
