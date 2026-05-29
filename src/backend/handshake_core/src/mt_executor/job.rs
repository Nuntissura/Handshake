//! MT-184 MicroTaskJob primitive + EscalationTier.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MicroTaskJobId(pub Uuid);

impl MicroTaskJobId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for MicroTaskJobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 6-tier escalation chain per folded WP-1-Micro-Task-Executor-v1 stub.
///
/// NOTE on wire form: each variant carries an explicit `#[serde(rename = "...")]`
/// because `rename_all = "snake_case"` emits `t7_b` for `T7B` (snake_case splits
/// at the trailing capital), but the Postgres schema, the `as_str()` form, the
/// migration indexes, and the queue's `.bind(tier.as_str())` write path all use
/// the compact wire form (`t7b`, `t7b_alt`, ...). Without these explicit
/// renames, write-via-`as_str` and read-via-`serde::from_value` would disagree
/// and silently corrupt the round-trip for any tier with a letter-digit-letter
/// shape. MT-184 `tests/micro_task_job_tests.rs` locks the wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EscalationTier {
    #[serde(rename = "t7b")]
    T7B,
    #[serde(rename = "t7b_alt")]
    T7BAlt,
    #[serde(rename = "t13b")]
    T13B,
    #[serde(rename = "t13b_alt")]
    T13BAlt,
    #[serde(rename = "t32b")]
    T32B,
    #[serde(rename = "hard_gate")]
    HardGate,
}

impl EscalationTier {
    pub fn next(self) -> Option<EscalationTier> {
        match self {
            Self::T7B => Some(Self::T7BAlt),
            Self::T7BAlt => Some(Self::T13B),
            Self::T13B => Some(Self::T13BAlt),
            Self::T13BAlt => Some(Self::T32B),
            Self::T32B => Some(Self::HardGate),
            Self::HardGate => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::T7B => "t7b",
            Self::T7BAlt => "t7b_alt",
            Self::T13B => "t13b",
            Self::T13BAlt => "t13b_alt",
            Self::T32B => "t32b",
            Self::HardGate => "hard_gate",
        }
    }

    pub fn base_weight(self) -> i32 {
        match self {
            Self::HardGate => 1000,
            Self::T32B => 100,
            Self::T13BAlt => 80,
            Self::T13B => 60,
            Self::T7BAlt => 40,
            Self::T7B => 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscalationStep {
    pub from_tier: EscalationTier,
    pub to_tier: EscalationTier,
    pub reason: String,
    pub recorded_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicroTaskJobState {
    Queued,
    Claimed,
    Running,
    AwaitingVerification,
    Escalated,
    Completed,
    HardGated,
    Cancelled,
    CancellationRequested,
    Failed,
}

impl MicroTaskJobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Claimed => "claimed",
            Self::Running => "running",
            Self::AwaitingVerification => "awaiting_verification",
            Self::Escalated => "escalated",
            Self::Completed => "completed",
            Self::HardGated => "hard_gated",
            Self::Cancelled => "cancelled",
            Self::CancellationRequested => "cancellation_requested",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionSignal {
    Success { summary: String },
    Failure { reason: String },
    NeedsVerification { reason: String },
    Escalate { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunLedgerPointer {
    pub run_id: String,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroTaskJob {
    pub job_id: MicroTaskJobId,
    pub wp_id: String,
    pub mt_id: String,
    pub mt_contract_path: PathBuf,
    pub iteration_n: u32,
    pub max_iterations: u32,
    pub escalation_tier: EscalationTier,
    pub escalation_history: Vec<EscalationStep>,
    pub task_tags: Vec<String>,
    pub lora_id: Option<String>,
    pub mailbox_thread_id: Option<Uuid>,
    pub state: MicroTaskJobState,
    pub claimed_by_session: Option<Uuid>,
    pub claimed_at_utc: Option<DateTime<Utc>>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
    pub completion_signal: Option<CompletionSignal>,
    pub progress_artifact_ref: Option<String>,
    pub run_ledger_ref: Option<RunLedgerPointer>,
}

impl MicroTaskJob {
    pub fn queue(
        wp_id: impl Into<String>,
        mt_id: impl Into<String>,
        mt_contract_path: PathBuf,
        max_iterations: u32,
        task_tags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            job_id: MicroTaskJobId::new_v7(),
            wp_id: wp_id.into(),
            mt_id: mt_id.into(),
            mt_contract_path,
            iteration_n: 0,
            max_iterations,
            escalation_tier: EscalationTier::T7B,
            escalation_history: Vec::new(),
            task_tags,
            lora_id: None,
            mailbox_thread_id: None,
            state: MicroTaskJobState::Queued,
            claimed_by_session: None,
            claimed_at_utc: None,
            created_at_utc: now,
            updated_at_utc: now,
            completion_signal: None,
            progress_artifact_ref: None,
            run_ledger_ref: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalation_chain_is_monotonic() {
        let mut tier = EscalationTier::T7B;
        let mut steps = 0;
        while let Some(n) = tier.next() {
            assert!(n.base_weight() >= tier.base_weight() || matches!(n, EscalationTier::HardGate));
            tier = n;
            steps += 1;
        }
        assert_eq!(steps, 5);
        assert!(EscalationTier::HardGate.next().is_none());
    }

    #[test]
    fn job_id_is_v7() {
        assert_eq!(MicroTaskJobId::new_v7().as_uuid().get_version_num(), 7);
    }

    #[test]
    fn queue_helper_initialises_state() {
        let j = MicroTaskJob::queue("WP-A", "MT-1", PathBuf::from("a.json"), 3, vec![]);
        assert_eq!(j.state, MicroTaskJobState::Queued);
        assert_eq!(j.escalation_tier, EscalationTier::T7B);
        assert_eq!(j.iteration_n, 0);
    }
}
