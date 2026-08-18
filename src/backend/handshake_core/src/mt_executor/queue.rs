//! MT-184 MicroTaskQueue — SurrealDB-backed atomic claim.
//!
//! # Porting notes (PostgreSQL -> embedded SurrealDB)
//!
//! `SELECT ... FOR UPDATE SKIP LOCKED` and `SELECT ... FOR UPDATE` are replaced
//! by guarded single statements. SurrealDB reports affected rows by returning
//! them, so `RETURN AFTER` plus an affected count of zero is the "someone else
//! won" signal the row lock used to give. Each guard below re-states, inside the
//! write, the exact condition the locked read had verified:
//!
//! * `claim_next` re-checks `state = 'queued'` in the claiming UPDATE, so two
//!   claimers racing for the same row cannot both win. `SKIP LOCKED` let the
//!   loser move straight to the next row; the retry loop here reproduces that
//!   by re-selecting, and it terminates because every iteration either claims a
//!   row or observes that no queued row remains.
//! * `update_state` carries the hard-gate refusal as a predicate.
//! * `escalate_inner` compare-and-swaps on `escalation_tier`, and appends to
//!   `escalation_history` server-side with `array::append` so a concurrent
//!   escalation cannot clobber a step by writing back a stale array.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::PathBuf;
use surrealdb::types::{RecordId, SurrealValue};
use thiserror::Error;
use uuid::Uuid;

use super::job::{EscalationStep, EscalationTier, MicroTaskJob, MicroTaskJobId, MicroTaskJobState};
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};

pub(crate) const JOB_TABLE: &str = "kernel_micro_task_job";

/// A claim attempt is retried at most this many times before the queue reports
/// "nothing claimable". Each retry means a different claimer won a specific row,
/// so the bound only matters under extreme contention; it exists so a pathological
/// interleaving cannot spin forever.
const CLAIM_ATTEMPTS: usize = 16;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("storage error: {0}")]
    Storage(#[from] SurrealStorageError),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("invalid escalation transition")]
    InvalidEscalation,
    #[error("job not found")]
    NotFound,
    #[error("hardgated job rejects further claims")]
    HardGated,
    #[error("HardGate transition requires a decision_request mailbox receipt")]
    HardGateMailboxRequired,
}

/// One `kernel_micro_task_job` record.
#[derive(Debug, Clone, SurrealValue)]
pub(crate) struct JobRow {
    pub job_id: Uuid,
    pub wp_id: String,
    pub mt_id: String,
    pub mt_contract_path: String,
    pub iteration_n: i64,
    pub max_iterations: i64,
    pub escalation_tier: String,
    pub escalation_history: Vec<Value>,
    pub task_tags: Vec<String>,
    pub lora_id: Option<String>,
    pub mailbox_thread_id: Option<Uuid>,
    pub state: String,
    pub claimed_by_session: Option<Uuid>,
    pub claimed_at_utc: Option<DateTime<Utc>>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

impl TryFrom<JobRow> for MicroTaskJob {
    type Error = QueueError;

    fn try_from(row: JobRow) -> Result<Self, Self::Error> {
        let escalation_tier: EscalationTier =
            serde_json::from_value(Value::String(row.escalation_tier))?;
        let escalation_history: Vec<EscalationStep> =
            serde_json::from_value(Value::Array(row.escalation_history))?;
        let state: MicroTaskJobState = serde_json::from_value(Value::String(row.state))?;
        Ok(Self {
            job_id: MicroTaskJobId(row.job_id),
            wp_id: row.wp_id,
            mt_id: row.mt_id,
            mt_contract_path: PathBuf::from(row.mt_contract_path),
            iteration_n: row.iteration_n as u32,
            max_iterations: row.max_iterations as u32,
            escalation_tier,
            escalation_history,
            task_tags: row.task_tags,
            lora_id: row.lora_id,
            mailbox_thread_id: row.mailbox_thread_id,
            state,
            claimed_by_session: row.claimed_by_session,
            claimed_at_utc: row.claimed_at_utc,
            created_at_utc: row.created_at_utc,
            updated_at_utc: row.updated_at_utc,
            completion_signal: None,
            progress_artifact_ref: None,
            run_ledger_ref: None,
        })
    }
}

// ── bindings ────────────────────────────────────────────────────────────────

#[derive(SurrealValue)]
struct CreateJobBindings {
    record: RecordId,
    content: surrealdb::types::Value,
}

#[derive(SurrealValue)]
struct JobIdBinding {
    job_id: Uuid,
}

#[derive(SurrealValue)]
struct ClaimBindings {
    job_id: Uuid,
    session_id: Uuid,
    now: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct UpdateStateBindings {
    job_id: Uuid,
    state: String,
    transition_reason: Option<String>,
    allow_hard_gated: bool,
    now: DateTime<Utc>,
}

#[derive(SurrealValue)]
struct EscalateBindings {
    job_id: Uuid,
    current_tier: String,
    tier: String,
    step: Value,
    state: String,
    lora_id: Option<String>,
    transition_reason: String,
    now: DateTime<Utc>,
}

// ── statements ──────────────────────────────────────────────────────────────

const ENQUEUE_QUERY: &str = "CREATE $record CONTENT $content RETURN AFTER;";

const NEXT_QUEUED_QUERY: &str = "SELECT VALUE job_id FROM kernel_micro_task_job \
     WHERE state = 'queued' ORDER BY created_at_utc ASC LIMIT 1;";

/// The `AND state = 'queued'` is the whole claim. It re-states inside the write
/// what the `FOR UPDATE` read had verified, so a row already claimed by another
/// session matches zero rows here.
const CLAIM_QUERY: &str = "UPDATE kernel_micro_task_job SET \
     state = 'claimed', claimed_by_session = $session_id, claimed_at_utc = $now, \
     updated_at_utc = $now \
     WHERE job_id = $job_id AND state = 'queued' RETURN AFTER;";

const UPDATE_STATE_QUERY: &str = "UPDATE kernel_micro_task_job SET \
     state = $state, updated_at_utc = $now, \
     transition_reason = IF $transition_reason = NONE { transition_reason } \
       ELSE { $transition_reason } \
     WHERE job_id = $job_id AND ($allow_hard_gated OR state != 'hard_gated') RETURN AFTER;";

const ESCALATE_QUERY: &str = "UPDATE kernel_micro_task_job SET \
     escalation_tier = $tier, \
     escalation_history = array::append(escalation_history, $step), \
     state = $state, \
     lora_id = IF $lora_id = NONE { lora_id } ELSE { $lora_id }, \
     transition_reason = $transition_reason, \
     updated_at_utc = $now \
     WHERE job_id = $job_id AND escalation_tier = $current_tier RETURN AFTER;";

const GET_JOB_QUERY: &str = "SELECT job_id, wp_id, mt_id, mt_contract_path, iteration_n, \
     max_iterations, escalation_tier, escalation_history, task_tags, lora_id, mailbox_thread_id, \
     state, claimed_by_session, claimed_at_utc, created_at_utc, updated_at_utc \
     FROM kernel_micro_task_job WHERE job_id = $job_id;";

const GET_STATE_QUERY: &str =
    "SELECT VALUE state FROM kernel_micro_task_job WHERE job_id = $job_id;";

const GET_TIER_QUERY: &str =
    "SELECT VALUE escalation_tier FROM kernel_micro_task_job WHERE job_id = $job_id;";

// ── queue ───────────────────────────────────────────────────────────────────

pub struct MicroTaskQueue {
    storage: SurrealStorage,
}

impl MicroTaskQueue {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub(crate) async fn query<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Vec<R>, SurrealStorageError>
    where
        R: SurrealValue + Send + 'static,
        B: SurrealValue + Send + 'static,
    {
        self.storage
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values(statement, bindings).await })
            })
            .await
    }

    pub async fn enqueue(&self, job: &MicroTaskJob) -> Result<(), QueueError> {
        let content = JobRow {
            job_id: job.job_id.as_uuid(),
            wp_id: job.wp_id.clone(),
            mt_id: job.mt_id.clone(),
            mt_contract_path: job.mt_contract_path.to_string_lossy().to_string(),
            iteration_n: job.iteration_n as i64,
            max_iterations: job.max_iterations as i64,
            escalation_tier: job.escalation_tier.as_str().to_string(),
            escalation_history: json_array(&job.escalation_history)?,
            task_tags: job.task_tags.clone(),
            lora_id: job.lora_id.clone(),
            mailbox_thread_id: job.mailbox_thread_id,
            state: job.state.as_str().to_string(),
            claimed_by_session: job.claimed_by_session,
            claimed_at_utc: job.claimed_at_utc,
            created_at_utc: job.created_at_utc,
            updated_at_utc: job.updated_at_utc,
        };
        // CREATE, not UPSERT: a duplicate job id must fail exactly as the
        // original INSERT did rather than silently replacing a live job.
        let _: Vec<JobRow> = self
            .query(
                ENQUEUE_QUERY,
                CreateJobBindings {
                    record: job_record(job.job_id.as_uuid()),
                    content: content.into_value(),
                },
            )
            .await?;
        Ok(())
    }

    /// Atomically claim the next queued job, oldest first.
    ///
    /// Returns `None` only when no queued job remains (or when contention
    /// exhausted [`CLAIM_ATTEMPTS`], which is indistinguishable to the caller
    /// from "another claimer took it" and is safe to retry).
    pub async fn claim_next(&self, session_id: Uuid) -> Result<Option<MicroTaskJobId>, QueueError> {
        for _ in 0..CLAIM_ATTEMPTS {
            let candidates: Vec<Uuid> = self.query(NEXT_QUEUED_QUERY, EmptyBindings {}).await?;
            let Some(job_id) = candidates.into_iter().next() else {
                return Ok(None);
            };
            let claimed: Vec<JobRow> = self
                .query(
                    CLAIM_QUERY,
                    ClaimBindings {
                        job_id,
                        session_id,
                        now: Utc::now(),
                    },
                )
                .await?;
            if !claimed.is_empty() {
                return Ok(Some(MicroTaskJobId(job_id)));
            }
            // Another claimer won this row. Loop: this is the `SKIP LOCKED`
            // behaviour of moving on to the next candidate.
        }
        Ok(None)
    }

    pub async fn update_state(
        &self,
        job_id: MicroTaskJobId,
        new_state: MicroTaskJobState,
        transition_reason: Option<String>,
    ) -> Result<(), QueueError> {
        let allow_hard_gated = matches!(new_state, MicroTaskJobState::HardGated);
        let updated: Vec<JobRow> = self
            .query(
                UPDATE_STATE_QUERY,
                UpdateStateBindings {
                    job_id: job_id.as_uuid(),
                    state: new_state.as_str().to_string(),
                    transition_reason,
                    allow_hard_gated,
                    now: Utc::now(),
                },
            )
            .await?;
        if !updated.is_empty() {
            return Ok(());
        }
        // The guard refused. Re-read only to choose the typed error.
        let states: Vec<String> = self
            .query(
                GET_STATE_QUERY,
                JobIdBinding {
                    job_id: job_id.as_uuid(),
                },
            )
            .await?;
        match states.into_iter().next() {
            None => Err(QueueError::NotFound),
            Some(current) if current == "hard_gated" => Err(QueueError::HardGated),
            Some(_) => Err(QueueError::NotFound),
        }
    }

    pub async fn escalate(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
    ) -> Result<EscalationStep, QueueError> {
        self.escalate_inner(job_id, new_tier, reason, None, None)
            .await
    }

    pub async fn escalate_with_lora(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
        lora_id: Option<String>,
    ) -> Result<EscalationStep, QueueError> {
        self.escalate_inner(job_id, new_tier, reason, lora_id, None)
            .await
    }

    pub async fn hard_gate_after_mailbox_post(
        &self,
        job_id: MicroTaskJobId,
        reason: String,
        decision_request_message_id: Uuid,
    ) -> Result<EscalationStep, QueueError> {
        self.escalate_inner(
            job_id,
            EscalationTier::HardGate,
            reason,
            None,
            Some(decision_request_message_id),
        )
        .await
    }

    async fn escalate_inner(
        &self,
        job_id: MicroTaskJobId,
        new_tier: EscalationTier,
        reason: String,
        lora_id: Option<String>,
        hardgate_decision_request_message_id: Option<Uuid>,
    ) -> Result<EscalationStep, QueueError> {
        let tiers: Vec<String> = self
            .query(
                GET_TIER_QUERY,
                JobIdBinding {
                    job_id: job_id.as_uuid(),
                },
            )
            .await?;
        let Some(current_tier_s) = tiers.into_iter().next() else {
            return Err(QueueError::NotFound);
        };
        let current_tier: EscalationTier =
            serde_json::from_value(Value::String(current_tier_s.clone()))?;
        // Monotonic: new tier must equal current.next() unless re-issuing same.
        let allowed = match current_tier.next() {
            Some(next) => next == new_tier,
            None => false,
        };
        if !allowed {
            return Err(QueueError::InvalidEscalation);
        }
        if matches!(new_tier, EscalationTier::HardGate)
            && hardgate_decision_request_message_id.is_none()
        {
            return Err(QueueError::HardGateMailboxRequired);
        }
        let step = EscalationStep {
            from_tier: current_tier,
            to_tier: new_tier,
            reason: reason.clone(),
            recorded_at_utc: Utc::now(),
        };
        let new_state = if matches!(new_tier, EscalationTier::HardGate) {
            "hard_gated"
        } else {
            "escalated"
        };
        let transition_reason = hardgate_decision_request_message_id
            .map(|message_id| format!("{reason}; decision_request_message_id={message_id}"))
            .unwrap_or_else(|| reason.clone());
        // Compare-and-swap on the tier read above: a concurrent escalation that
        // already moved the tier makes this affect zero rows, so the monotonic
        // ladder cannot be skipped or double-stepped.
        let updated: Vec<JobRow> = self
            .query(
                ESCALATE_QUERY,
                EscalateBindings {
                    job_id: job_id.as_uuid(),
                    current_tier: current_tier_s,
                    tier: new_tier.as_str().to_string(),
                    step: serde_json::to_value(&step)?,
                    state: new_state.to_string(),
                    lora_id,
                    transition_reason,
                    now: Utc::now(),
                },
            )
            .await?;
        if updated.is_empty() {
            return Err(QueueError::InvalidEscalation);
        }
        Ok(step)
    }

    pub async fn get_state(
        &self,
        job_id: MicroTaskJobId,
    ) -> Result<Option<MicroTaskJobState>, QueueError> {
        let states: Vec<String> = self
            .query(
                GET_STATE_QUERY,
                JobIdBinding {
                    job_id: job_id.as_uuid(),
                },
            )
            .await?;
        let Some(state) = states.into_iter().next() else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_value(Value::String(state))?))
    }

    pub async fn get_job(
        &self,
        job_id: MicroTaskJobId,
    ) -> Result<Option<MicroTaskJob>, QueueError> {
        let rows: Vec<JobRow> = self
            .query(
                GET_JOB_QUERY,
                JobIdBinding {
                    job_id: job_id.as_uuid(),
                },
            )
            .await?;
        rows.into_iter().next().map(TryInto::try_into).transpose()
    }
}

#[derive(SurrealValue)]
pub(crate) struct EmptyBindings {}

pub(crate) fn job_record(id: Uuid) -> RecordId {
    RecordId::new(JOB_TABLE, surrealdb::types::Uuid::from(id))
}

fn json_array<T: serde::Serialize>(values: &[T]) -> Result<Vec<Value>, QueueError> {
    values
        .iter()
        .map(|value| serde_json::to_value(value).map_err(QueueError::from))
        .collect()
}
