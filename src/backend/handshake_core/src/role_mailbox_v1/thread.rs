//! MT-176 RoleMailboxThread typed primitive.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::lifecycle::ThreadLifecycleState;
use super::router::ExecutorKind;

/// Newtype wrapping `Uuid::now_v7()` for thread identity. Per HBR-INT-008 all
/// mint sites must use v7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleMailboxThreadId(pub Uuid);

impl RoleMailboxThreadId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for RoleMailboxThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkedRecordKind {
    Wp,
    Mt,
    Rgf,
    Freeform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimMode {
    Exclusive,
    Handoff,
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseAuthorityScope {
    /// Holder of the thread's active lease may respond (default).
    LeaseHolder,
    /// Any executor in the allowlist may respond (no lease required).
    AllowlistOpen,
    /// Holder must also hold the linked MT executor lease (MT-189 wires this
    /// across MicroTaskJob; here it is forward-declared).
    MicroTaskCompletionScope,
    /// Only the operator role may respond.
    OperatorOnly,
}

/// Spec v02.186 §02-system-architecture.md role mailbox subsection [ADD v02.173].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxThread {
    pub thread_id: RoleMailboxThreadId,
    pub title: String,
    pub linked_record_kind: LinkedRecordKind,
    pub linked_record_id: Option<String>,
    pub lifecycle_state: ThreadLifecycleState,
    pub executor_kind_allowlist: Vec<ExecutorKind>,
    pub claim_mode: ClaimMode,
    pub lease_duration_secs: Option<u32>,
    pub takeover_policy: super::lease::TakeoverPolicy,
    pub response_authority_scope: ResponseAuthorityScope,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
    pub expires_at_utc: Option<DateTime<Utc>>,
    pub archived_at_utc: Option<DateTime<Utc>>,
}

impl RoleMailboxThread {
    /// Construct a fresh thread row with `Uuid::now_v7()` identity, current
    /// timestamps, and `lifecycle_state=Open`.
    pub fn open(
        title: impl Into<String>,
        linked_record_kind: LinkedRecordKind,
        linked_record_id: Option<String>,
        executor_kind_allowlist: Vec<ExecutorKind>,
        claim_mode: ClaimMode,
        takeover_policy: super::lease::TakeoverPolicy,
        response_authority_scope: ResponseAuthorityScope,
    ) -> Self {
        let now = Utc::now();
        Self {
            thread_id: RoleMailboxThreadId::new_v7(),
            title: title.into(),
            linked_record_kind,
            linked_record_id,
            lifecycle_state: ThreadLifecycleState::Open,
            executor_kind_allowlist,
            claim_mode,
            lease_duration_secs: None,
            takeover_policy,
            response_authority_scope,
            created_at_utc: now,
            updated_at_utc: now,
            expires_at_utc: None,
            archived_at_utc: None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.lifecycle_state.is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_id_is_v7() {
        let id = RoleMailboxThreadId::new_v7();
        assert_eq!(
            id.as_uuid().get_version_num(),
            7,
            "thread id must be Uuid v7 per HBR-INT-008"
        );
    }

    #[test]
    fn open_thread_has_lifecycle_open() {
        let t = RoleMailboxThread::open(
            "test",
            LinkedRecordKind::Wp,
            Some("WP-X".to_string()),
            vec![ExecutorKind::LocalSmallModel],
            ClaimMode::Exclusive,
            super::super::lease::TakeoverPolicy::Never,
            ResponseAuthorityScope::LeaseHolder,
        );
        assert_eq!(t.lifecycle_state, ThreadLifecycleState::Open);
        assert!(!t.is_terminal());
    }

    #[test]
    fn round_trip_serde() {
        let t = RoleMailboxThread::open(
            "a",
            LinkedRecordKind::Mt,
            None,
            vec![ExecutorKind::Validator, ExecutorKind::CloudModel],
            ClaimMode::Handoff,
            super::super::lease::TakeoverPolicy::OnLeaseExpiry,
            ResponseAuthorityScope::LeaseHolder,
        );
        let s = serde_json::to_string(&t).unwrap();
        let back: RoleMailboxThread = serde_json::from_str(&s).unwrap();
        assert_eq!(t, back);
    }
}
