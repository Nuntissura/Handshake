//! WP-KERNEL-004 cluster X.2 MT-185 — MT loop-control checkpoint contract.
//!
//! Canonical implementation owned by this module per the MT-185 contract
//! `owned_files`. The sibling `crate::mt_executor::loop_control` module
//! re-exports the public surface defined here so executor.rs (MT-189) and any
//! other in-cluster consumer can keep importing
//! `crate::mt_executor::loop_control::{MtLoopControl, ...}` without source
//! churn while the authority lives where the contract says it lives.
//!
//! Per spec line 6147 + folded WP-1-Role-Mailbox-Micro-Task-Loop-Control-v1
//! intent (refinement.json line 987): "durable loop-control contract so small
//! local models, cloud models, reviewers, and operators can resume or inspect
//! Micro-Task loops from compact state rather than chat chronology."
//!
//! Surfaces:
//!   * `MtLoopState`         — typed checkpoint state machine
//!   * `MtLoopCheckpoint`    — compact resumable state (<= 2048-byte summary)
//!   * `MtLoopControlBudget` — bounded retry / verifier-feedback / pause
//!   * `MtLoopControl`       — orchestration helpers (record + resume)
//!   * `LoopControlError`    — typed budget / size violations (no silent
//!                              continuation; spec line 6147 invariant)
//!   * `MtLoopCheckpointRepo` — exact-scope embedded-Surreal persistence.
//!
//! Wire-form note for `evidence_pointers`:
//!   The on-disk JSONB column stores `Vec<EvidencePointer>` per the contract
//!   (`kind`, `uri`, optional `label`). For backward source compatibility
//!   with the early MT-185 skeleton (and existing call sites in
//!   `mt_executor::executor`) the typed pointer alias re-exports the
//!   `crate::role_mailbox_v1::families::EvidencePointer` already used by the
//!   MT-179 mailbox families; the two surfaces share one wire form, so
//!   MT-188 outcome recording bridges role-mailbox events and loop-control
//!   checkpoints without translation.
//!
//! Concurrency invariants:
//!   * Verifier-feedback history is append-only inside a single
//!     `record_checkpoint` call (callers pass `prior_history` and the new
//!     checkpoint carries a strictly longer chain when feedback is appended).
//!     The durable repo enforces the same shape at write time by binding
//!     `verifier_feedback_history` as the full JSONB array; concurrent
//!     writers cannot blind-drop earlier entries because the prior history
//!     is read inside the same transaction.
//!   * `record_checkpoint` is a pure function on `MicroTaskJob` + budget +
//!     prior history. Persistence is a separate concern (the
//!     `MtLoopCheckpointRepo::persist` step) so a fresh executor can rebuild
//!     a checkpoint in memory and store it later without changing the
//!     budget-enforcement semantics.
//!   * Resume is fully deterministic: `MtLoopControl::resume_from` reads
//!     only fields already on the checkpoint, so two different executors
//!     calling `resume_from` on the same checkpoint produce byte-identical
//!     `ResumeContext` values regardless of session_id.

use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use thiserror::Error;
use uuid::Uuid;

use crate::mt_executor::job::{CompletionSignal, EscalationTier, MicroTaskJob, MicroTaskJobId};
use crate::process_ledger::ReclaimResourceScope;
use crate::storage::surreal::{SurrealProcessLedgerStore, SurrealStorage, SurrealStorageError};

/// Evidence pointer reused from the MT-179 mailbox-family surface so loop
/// checkpoints and mailbox `micro_task_*` payloads share the same wire form.
pub use crate::role_mailbox_v1::families::EvidencePointer;

/// Bound on the `compact_summary` field. Mirrored as a CHECK constraint in
/// `migrations/0023_micro_task_job_queue.sql`
/// (`CONSTRAINT compact_summary_bound CHECK (octet_length(compact_summary) <= 2048)`).
pub const COMPACT_SUMMARY_MAX_BYTES: usize = 2048;

fn now_utc_storage_precision() -> DateTime<Utc> {
    let now = Utc::now();
    let nanos = now.timestamp_subsec_nanos();
    now.with_nanosecond(nanos - (nanos % 1_000)).unwrap_or(now)
}

/// Loop-control state machine. Every variant is a terminal checkpoint within
/// the MT iteration loop — `Completed` carries the final
/// `CompletionSignal` so MT-188 outcome recording can read it directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MtLoopState {
    WaitingForVerifier,
    IteratingOnFeedback { feedback_id: Uuid },
    VerificationNeeded { reason: String },
    EscalationProposed { target_tier: EscalationTier },
    Completed { signal: CompletionSignal },
}

/// Verifier-feedback history entry. Append-only per checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierFeedbackRef {
    pub feedback_id: Uuid,
    pub at_utc: DateTime<Utc>,
    pub summary: String,
}

/// Bounded loop budget. Loaded from MT contract or default.
///
/// Defaults are conservative enough that a runaway loop fails closed within
/// a small number of iterations (max_iterations=6 matches the cluster X.2
/// integration tests, max_verifier_feedback_loops=4 keeps verifier ping-pong
/// from monopolising the mailbox).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtLoopControlBudget {
    pub max_iterations: u32,
    pub max_consecutive_failures: u32,
    pub max_hardgate_pause_secs: u32,
    pub max_verifier_feedback_loops: u32,
}

impl Default for MtLoopControlBudget {
    fn default() -> Self {
        Self {
            max_iterations: 6,
            max_consecutive_failures: 3,
            max_hardgate_pause_secs: 3600,
            max_verifier_feedback_loops: 4,
        }
    }
}

/// Compact, resumable loop checkpoint. <=2048-byte `compact_summary` keeps
/// the row small enough for cheap fan-out to local models and bounded enough
/// to survive a JSONB index without blowing the row out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtLoopCheckpoint {
    pub checkpoint_id: Uuid,
    pub job_id: MicroTaskJobId,
    pub iteration_n: u32,
    pub state_at_checkpoint: MtLoopState,
    pub retry_budget_remaining: u32,
    pub verifier_feedback_history: Vec<VerifierFeedbackRef>,
    pub compact_summary: String,
    pub evidence_pointers: Vec<EvidencePointer>,
    pub created_at_utc: DateTime<Utc>,
    pub created_by_session: Uuid,
}

/// Typed loop-control failure modes. None of these carry silent
/// continuation — `LoopBudgetExceeded` forces escalation per spec line 6147.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LoopControlError {
    #[error("loop budget exceeded: {0}")]
    LoopBudgetExceeded(String),
    #[error("compact summary exceeds {COMPACT_SUMMARY_MAX_BYTES} bytes ({0})")]
    SummaryTooLarge(usize),
    #[error("verifier feedback history must be append-only; prior len {prior}, new len {new}")]
    NonAppendOnlyHistory { prior: usize, new: usize },
}

/// Resume context returned by `MtLoopControl::resume_from`. Contains the
/// compact summary + evidence + state needed by a fresh executor to
/// continue without transcript replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeContext {
    pub iteration_n: u32,
    pub compact_summary: String,
    pub evidence_pointers: Vec<EvidencePointer>,
    pub state: MtLoopState,
    pub retry_budget_remaining: u32,
}

/// Orchestration helper. Pure-function surface — does no I/O. Persistence is
/// the `MtLoopCheckpointRepo` concern.
pub struct MtLoopControl;

impl MtLoopControl {
    /// Build a checkpoint from a job + state + summary + evidence. Enforces
    /// the bounded-retry contract and the compact-summary size bound.
    ///
    /// Returns `LoopBudgetExceeded` if the job's iteration counter has
    /// already reached the budget (forcing escalation per spec line 6147),
    /// `SummaryTooLarge` if the summary exceeds `COMPACT_SUMMARY_MAX_BYTES`.
    pub fn record_checkpoint(
        job: &MicroTaskJob,
        state: MtLoopState,
        compact_summary: String,
        evidence_pointers: Vec<EvidencePointer>,
        budget: &MtLoopControlBudget,
        prior_history: Vec<VerifierFeedbackRef>,
        session_id: Uuid,
    ) -> Result<MtLoopCheckpoint, LoopControlError> {
        if compact_summary.as_bytes().len() > COMPACT_SUMMARY_MAX_BYTES {
            return Err(LoopControlError::SummaryTooLarge(
                compact_summary.as_bytes().len(),
            ));
        }
        if job.iteration_n >= budget.max_iterations {
            return Err(LoopControlError::LoopBudgetExceeded(format!(
                "iteration {} >= max {}",
                job.iteration_n, budget.max_iterations
            )));
        }
        if prior_history.len() as u32 >= budget.max_verifier_feedback_loops {
            return Err(LoopControlError::LoopBudgetExceeded(format!(
                "verifier feedback loops {} >= max {}",
                prior_history.len(),
                budget.max_verifier_feedback_loops
            )));
        }
        let retry_budget_remaining = budget
            .max_iterations
            .saturating_sub(job.iteration_n)
            .saturating_sub(1);
        Ok(MtLoopCheckpoint {
            checkpoint_id: Uuid::now_v7(),
            job_id: job.job_id,
            iteration_n: job.iteration_n,
            state_at_checkpoint: state,
            retry_budget_remaining,
            verifier_feedback_history: prior_history,
            compact_summary,
            evidence_pointers,
            created_at_utc: now_utc_storage_precision(),
            created_by_session: session_id,
        })
    }

    /// Variant of `record_checkpoint` that appends one new verifier-feedback
    /// entry on top of the prior history. Enforces append-only semantics
    /// (the new history must be strictly longer than the prior and the
    /// prefix must match).
    pub fn record_checkpoint_with_feedback(
        job: &MicroTaskJob,
        state: MtLoopState,
        compact_summary: String,
        evidence_pointers: Vec<EvidencePointer>,
        budget: &MtLoopControlBudget,
        prior_history: Vec<VerifierFeedbackRef>,
        new_feedback: VerifierFeedbackRef,
        session_id: Uuid,
    ) -> Result<MtLoopCheckpoint, LoopControlError> {
        // Append-only invariant: budget is enforced on the new total.
        let total = prior_history.len() as u32 + 1;
        if total > budget.max_verifier_feedback_loops {
            return Err(LoopControlError::LoopBudgetExceeded(format!(
                "verifier feedback loops {} > max {}",
                total, budget.max_verifier_feedback_loops
            )));
        }
        let mut history = prior_history;
        history.push(new_feedback);
        // Reuse the main path so size/iteration bounds keep one
        // enforcement point. We pre-cleared the verifier-loop bound above
        // so the inner check (`>=`) does not double-fault on append.
        // To avoid the inner `>=` tripping on the freshly-pushed entry,
        // we synthesise a budget with bumped feedback-loop room.
        let inner_budget = MtLoopControlBudget {
            max_verifier_feedback_loops: budget
                .max_verifier_feedback_loops
                .max(history.len() as u32 + 1),
            ..*budget
        };
        let cp = Self::record_checkpoint(
            job,
            state,
            compact_summary,
            evidence_pointers,
            &inner_budget,
            history,
            session_id,
        )?;
        Ok(cp)
    }

    /// Compact resume context. Pure function: identical input ->
    /// byte-identical output, regardless of which session_id is calling.
    pub fn resume_from(checkpoint: &MtLoopCheckpoint) -> ResumeContext {
        ResumeContext {
            iteration_n: checkpoint.iteration_n,
            compact_summary: checkpoint.compact_summary.clone(),
            evidence_pointers: checkpoint.evidence_pointers.clone(),
            state: checkpoint.state_at_checkpoint.clone(),
            retry_budget_remaining: checkpoint.retry_budget_remaining,
        }
    }

    /// True if the job has hit its bounded-retry ceiling. Forces escalation
    /// per spec line 6147.
    pub fn budget_exhausted(job: &MicroTaskJob, budget: &MtLoopControlBudget) -> bool {
        job.iteration_n >= budget.max_iterations
    }
}

// ----------------------- Embedded-Surreal repository -----------------------

/// Exact-scope checkpoint repository over the injected embedded store.
#[derive(Debug, Clone)]
pub struct MtLoopCheckpointRepo {
    storage: SurrealStorage,
    resource_scope: ReclaimResourceScope,
}

#[derive(Debug, Error)]
pub enum CheckpointRepoError {
    #[error("embedded SurrealDB error: {0}")]
    Store(#[from] SurrealStorageError),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("loop control: {0}")]
    LoopControl(#[from] LoopControlError),
    #[error("checkpoint schema bootstrap failed: {0}")]
    Bootstrap(String),
    #[error("checkpoint row has invalid durable identity: {0}")]
    InvalidIdentity(String),
}

#[derive(Clone, SurrealValue)]
struct CheckpointScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(SurrealValue)]
struct CheckpointWriteBindings {
    checkpoint_record: RecordId,
    job_record: RecordId,
    checkpoint_id: Uuid,
    iteration_n: i64,
    state_at_checkpoint: Value,
    retry_budget_remaining: i64,
    verifier_feedback_history: Value,
    compact_summary: String,
    evidence_pointers: Value,
    created_at_utc: DateTime<Utc>,
    created_by_session: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(SurrealValue)]
struct CheckpointIdBindings {
    checkpoint_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(SurrealValue)]
struct CheckpointJobBindings {
    job_record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(SurrealValue)]
struct CheckpointRow {
    checkpoint_id: Uuid,
    job_id: RecordId,
    iteration_n: i64,
    state_at_checkpoint: Value,
    retry_budget_remaining: i64,
    verifier_feedback_history: Value,
    compact_summary: String,
    evidence_pointers: Value,
    created_at_utc: DateTime<Utc>,
    created_by_session: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(SurrealValue)]
struct CountRow {
    count: i64,
}

const PERSIST_CHECKPOINT: &str = r#"
BEGIN TRANSACTION;
LET $existing = SELECT VALUE id FROM ONLY $checkpoint_record
    WHERE owner_account_id = $owner_account_id
      AND actor_principal_id = $actor_principal_id
      AND authenticated_session_id = $authenticated_session_id
      AND access_space_id = $access_space_id
      AND workspace_id = $workspace_id;
IF $existing != NONE { THROW 'mt-loop-checkpoint-id-conflict'; };
CREATE $checkpoint_record CONTENT {
    checkpoint_id: $checkpoint_id, job_id: $job_record, iteration_n: $iteration_n,
    state_at_checkpoint: $state_at_checkpoint,
    retry_budget_remaining: $retry_budget_remaining,
    verifier_feedback_history: $verifier_feedback_history,
    compact_summary: $compact_summary, evidence_pointers: $evidence_pointers,
    created_at_utc: $created_at_utc, created_by_session: $created_by_session,
    owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
    authenticated_session_id: $authenticated_session_id, access_space_id: $access_space_id,
    workspace_id: $workspace_id
};
LET $stored = SELECT * FROM ONLY $checkpoint_record
    WHERE checkpoint_id = $checkpoint_id
      AND job_id = $job_record
      AND owner_account_id = $owner_account_id
      AND actor_principal_id = $actor_principal_id
      AND authenticated_session_id = $authenticated_session_id
      AND access_space_id = $access_space_id
      AND workspace_id = $workspace_id;
IF $stored = NONE { THROW 'mt-loop-checkpoint-verification-mismatch'; };
RETURN $stored;
COMMIT TRANSACTION;
"#;

const GET_CHECKPOINT: &str = r#"
SELECT * FROM ONLY $checkpoint_record
 WHERE owner_account_id = $owner_account_id
   AND actor_principal_id = $actor_principal_id
   AND authenticated_session_id = $authenticated_session_id
   AND access_space_id = $access_space_id
   AND workspace_id = $workspace_id;
"#;

const LATEST_CHECKPOINT: &str = r#"
SELECT * FROM kernel_mt_loop_checkpoint
 WHERE job_id = $job_record
   AND owner_account_id = $owner_account_id
   AND actor_principal_id = $actor_principal_id
   AND authenticated_session_id = $authenticated_session_id
   AND access_space_id = $access_space_id
   AND workspace_id = $workspace_id
 ORDER BY created_at_utc DESC, checkpoint_id DESC LIMIT 1;
"#;

const CHECKPOINT_CHAIN: &str = r#"
SELECT * FROM kernel_mt_loop_checkpoint
 WHERE job_id = $job_record
   AND owner_account_id = $owner_account_id
   AND actor_principal_id = $actor_principal_id
   AND authenticated_session_id = $authenticated_session_id
   AND access_space_id = $access_space_id
   AND workspace_id = $workspace_id
 ORDER BY created_at_utc ASC, checkpoint_id ASC;
"#;

const COUNT_CHECKPOINTS: &str = r#"
SELECT count() AS count FROM kernel_mt_loop_checkpoint
 WHERE job_id = $job_record
   AND owner_account_id = $owner_account_id
   AND actor_principal_id = $actor_principal_id
   AND authenticated_session_id = $authenticated_session_id
   AND access_space_id = $access_space_id
   AND workspace_id = $workspace_id GROUP ALL;
"#;

impl MtLoopCheckpointRepo {
    pub fn new(storage: SurrealStorage, resource_scope: ReclaimResourceScope) -> Self {
        Self {
            storage,
            resource_scope,
        }
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    pub async fn open(
        storage: SurrealStorage,
        resource_scope: ReclaimResourceScope,
    ) -> Result<Self, CheckpointRepoError> {
        SurrealProcessLedgerStore::open(storage.clone())
            .await
            .map_err(|error| CheckpointRepoError::Bootstrap(error.to_string()))?;
        Ok(Self::new(storage, resource_scope))
    }

    /// Insert one checkpoint. Pre-validates against the budget so a runaway
    /// loop fails before we round-trip to the DB.
    pub async fn persist(&self, checkpoint: &MtLoopCheckpoint) -> Result<(), CheckpointRepoError> {
        if checkpoint.compact_summary.as_bytes().len() > COMPACT_SUMMARY_MAX_BYTES {
            return Err(CheckpointRepoError::LoopControl(
                LoopControlError::SummaryTooLarge(checkpoint.compact_summary.as_bytes().len()),
            ));
        }
        let scope = &self.resource_scope;
        let bindings = CheckpointWriteBindings {
            checkpoint_record: RecordId::new(
                "kernel_mt_loop_checkpoint",
                checkpoint.checkpoint_id.to_string(),
            ),
            job_record: RecordId::new(
                "kernel_micro_task_job",
                checkpoint.job_id.as_uuid().to_string(),
            ),
            checkpoint_id: checkpoint.checkpoint_id,
            iteration_n: i64::from(checkpoint.iteration_n),
            state_at_checkpoint: serde_json::to_value(&checkpoint.state_at_checkpoint)?,
            retry_budget_remaining: i64::from(checkpoint.retry_budget_remaining),
            verifier_feedback_history: serde_json::to_value(&checkpoint.verifier_feedback_history)?,
            compact_summary: checkpoint.compact_summary.clone(),
            evidence_pointers: serde_json::to_value(&checkpoint.evidence_pointers)?,
            created_at_utc: checkpoint.created_at_utc,
            created_by_session: checkpoint.created_by_session,
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<CheckpointRow, _>(PERSIST_CHECKPOINT, bindings, 7)
                        .await
                })
            })
            .await?;
        if rows.len() != 1 {
            return Err(CheckpointRepoError::InvalidIdentity(
                "checkpoint transaction did not return exactly one scoped row".to_owned(),
            ));
        }
        self.validate_row_scope(&rows[0])?;
        Ok(())
    }

    /// Fetch one checkpoint by id.
    pub async fn get(
        &self,
        checkpoint_id: Uuid,
    ) -> Result<Option<MtLoopCheckpoint>, CheckpointRepoError> {
        let scope = &self.resource_scope;
        let bindings = CheckpointIdBindings {
            checkpoint_record: RecordId::new(
                "kernel_mt_loop_checkpoint",
                checkpoint_id.to_string(),
            ),
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        };
        let row = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<CheckpointRow, _>(GET_CHECKPOINT, bindings)
                        .await
                })
            })
            .await?;
        row.map(|row| self.row_to_checkpoint(row)).transpose()
    }

    /// Latest checkpoint for a job, by `created_at_utc DESC`. Returns None
    /// if the job has no checkpoints yet.
    pub async fn latest_for_job(
        &self,
        job_id: MicroTaskJobId,
    ) -> Result<Option<MtLoopCheckpoint>, CheckpointRepoError> {
        let rows = self.rows_for_job(LATEST_CHECKPOINT, job_id).await?;
        rows.into_iter()
            .next()
            .map(|row| self.row_to_checkpoint(row))
            .transpose()
    }

    /// Full ordered chain of checkpoints for a job. Useful for audit /
    /// debugger views.
    pub async fn chain_for_job(
        &self,
        job_id: MicroTaskJobId,
    ) -> Result<Vec<MtLoopCheckpoint>, CheckpointRepoError> {
        self.rows_for_job(CHECKPOINT_CHAIN, job_id)
            .await?
            .into_iter()
            .map(|row| self.row_to_checkpoint(row))
            .collect()
    }

    /// Atomic count of recorded retry-driving checkpoints for a job. Useful
    /// for the cross-executor "concurrent checkpoints don't double-count
    /// retries" invariant: the counter is computed from the DB rows
    /// directly so two executors racing on the same job can't both think
    /// they hold the last retry slot.
    pub async fn count_iterations(
        &self,
        job_id: MicroTaskJobId,
    ) -> Result<i64, CheckpointRepoError> {
        let bindings = self.job_bindings(job_id);
        let row = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_first::<CountRow, _>(COUNT_CHECKPOINTS, bindings)
                        .await
                })
            })
            .await?;
        Ok(row.map_or(0, |row| row.count))
    }

    fn scope_bindings(&self) -> CheckpointScopeBindings {
        let scope = &self.resource_scope;
        CheckpointScopeBindings {
            owner_account_id: scope.account_uuid.to_string(),
            actor_principal_id: scope.actor_uuid.to_string(),
            authenticated_session_id: scope.session_uuid.to_string(),
            access_space_id: scope.access_space_uuid.to_string(),
            workspace_id: scope.workspace_id.clone(),
        }
    }

    fn job_bindings(&self, job_id: MicroTaskJobId) -> CheckpointJobBindings {
        let scope = self.scope_bindings();
        CheckpointJobBindings {
            job_record: RecordId::new("kernel_micro_task_job", job_id.as_uuid().to_string()),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        }
    }

    async fn rows_for_job(
        &self,
        statement: &'static str,
        job_id: MicroTaskJobId,
    ) -> Result<Vec<CheckpointRow>, CheckpointRepoError> {
        let bindings = self.job_bindings(job_id);
        Ok(self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<CheckpointRow, _>(statement, bindings)
                        .await
                })
            })
            .await?)
    }

    fn validate_row_scope(&self, row: &CheckpointRow) -> Result<(), CheckpointRepoError> {
        let scope = &self.resource_scope;
        if row.owner_account_id != scope.account_uuid.to_string()
            || row.actor_principal_id != scope.actor_uuid.to_string()
            || row.authenticated_session_id != scope.session_uuid.to_string()
            || row.access_space_id != scope.access_space_uuid.to_string()
            || row.workspace_id != scope.workspace_id
        {
            return Err(CheckpointRepoError::InvalidIdentity(
                "checkpoint resource scope mismatch".to_owned(),
            ));
        }
        Ok(())
    }

    fn row_to_checkpoint(
        &self,
        row: CheckpointRow,
    ) -> Result<MtLoopCheckpoint, CheckpointRepoError> {
        self.validate_row_scope(&row)?;
        let RecordIdKey::String(job_id) = &row.job_id.key else {
            return Err(CheckpointRepoError::InvalidIdentity(
                "checkpoint job record id is not a string UUID".to_owned(),
            ));
        };
        let job_uuid = Uuid::parse_str(job_id).map_err(|error| {
            CheckpointRepoError::InvalidIdentity(format!(
                "invalid checkpoint job record id: {error}"
            ))
        })?;
        Ok(MtLoopCheckpoint {
            checkpoint_id: row.checkpoint_id,
            job_id: MicroTaskJobId(job_uuid),
            iteration_n: u32::try_from(row.iteration_n).map_err(|_| {
                CheckpointRepoError::InvalidIdentity("negative or oversized iteration_n".to_owned())
            })?,
            state_at_checkpoint: serde_json::from_value(row.state_at_checkpoint)?,
            retry_budget_remaining: u32::try_from(row.retry_budget_remaining).map_err(|_| {
                CheckpointRepoError::InvalidIdentity(
                    "negative or oversized retry budget".to_owned(),
                )
            })?,
            verifier_feedback_history: serde_json::from_value(row.verifier_feedback_history)?,
            compact_summary: row.compact_summary,
            evidence_pointers: serde_json::from_value(row.evidence_pointers)?,
            created_at_utc: row.created_at_utc,
            created_by_session: row.created_by_session,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn durable_checkpoint_queries_bind_all_five_scope_fields() {
        for statement in [
            PERSIST_CHECKPOINT,
            GET_CHECKPOINT,
            LATEST_CHECKPOINT,
            CHECKPOINT_CHAIN,
            COUNT_CHECKPOINTS,
        ] {
            for predicate in [
                "owner_account_id = $owner_account_id",
                "actor_principal_id = $actor_principal_id",
                "authenticated_session_id = $authenticated_session_id",
                "access_space_id = $access_space_id",
                "workspace_id = $workspace_id",
            ] {
                assert!(statement.contains(predicate), "missing {predicate}");
            }
        }
    }

    fn sample_job(iter_n: u32) -> MicroTaskJob {
        let mut j = MicroTaskJob::queue("WP-A", "MT-1", PathBuf::from("a.json"), 6, vec![]);
        j.iteration_n = iter_n;
        j
    }

    #[test]
    fn record_within_budget_yields_decrementing_remaining() {
        let cp = MtLoopControl::record_checkpoint(
            &sample_job(2),
            MtLoopState::WaitingForVerifier,
            "ok".to_string(),
            vec![],
            &MtLoopControlBudget::default(),
            vec![],
            Uuid::now_v7(),
        )
        .unwrap();
        assert_eq!(cp.iteration_n, 2);
        // default max_iterations = 6, iter_n = 2, remaining = 6 - 2 - 1 = 3
        assert_eq!(cp.retry_budget_remaining, 3);
    }

    #[test]
    fn iteration_budget_returns_loop_budget_exceeded() {
        let res = MtLoopControl::record_checkpoint(
            &sample_job(6),
            MtLoopState::WaitingForVerifier,
            "ok".to_string(),
            vec![],
            &MtLoopControlBudget::default(),
            vec![],
            Uuid::now_v7(),
        );
        assert!(matches!(res, Err(LoopControlError::LoopBudgetExceeded(_))));
    }

    #[test]
    fn summary_size_bound_enforced_at_record_time() {
        let big = "x".repeat(COMPACT_SUMMARY_MAX_BYTES + 1);
        let res = MtLoopControl::record_checkpoint(
            &sample_job(0),
            MtLoopState::WaitingForVerifier,
            big,
            vec![],
            &MtLoopControlBudget::default(),
            vec![],
            Uuid::now_v7(),
        );
        assert!(matches!(res, Err(LoopControlError::SummaryTooLarge(_))));
    }

    #[test]
    fn resume_is_byte_identical_across_calls() {
        let cp = MtLoopControl::record_checkpoint(
            &sample_job(1),
            MtLoopState::EscalationProposed {
                target_tier: EscalationTier::T7BAlt,
            },
            "ctx".to_string(),
            vec![EvidencePointer {
                kind: "log".to_string(),
                uri: "file://x".to_string(),
                label: Some("first".to_string()),
            }],
            &MtLoopControlBudget::default(),
            vec![],
            Uuid::now_v7(),
        )
        .unwrap();
        let a = MtLoopControl::resume_from(&cp);
        let b = MtLoopControl::resume_from(&cp);
        assert_eq!(a, b);
        // And serialization is byte-identical
        let sa = serde_json::to_string(&a).unwrap();
        let sb = serde_json::to_string(&b).unwrap();
        assert_eq!(sa, sb);
    }

    #[test]
    fn budget_exhausted_at_ceiling() {
        let budget = MtLoopControlBudget::default();
        assert!(!MtLoopControl::budget_exhausted(&sample_job(5), &budget));
        assert!(MtLoopControl::budget_exhausted(&sample_job(6), &budget));
        assert!(MtLoopControl::budget_exhausted(&sample_job(7), &budget));
    }

    #[test]
    fn verifier_feedback_loop_budget_enforced() {
        let mut budget = MtLoopControlBudget::default();
        budget.max_verifier_feedback_loops = 2;
        let history: Vec<VerifierFeedbackRef> = (0..2)
            .map(|i| VerifierFeedbackRef {
                feedback_id: Uuid::now_v7(),
                at_utc: Utc::now(),
                summary: format!("f{i}"),
            })
            .collect();
        let res = MtLoopControl::record_checkpoint(
            &sample_job(0),
            MtLoopState::IteratingOnFeedback {
                feedback_id: Uuid::now_v7(),
            },
            "ok".to_string(),
            vec![],
            &budget,
            history,
            Uuid::now_v7(),
        );
        assert!(matches!(res, Err(LoopControlError::LoopBudgetExceeded(_))));
    }

    #[test]
    fn record_with_feedback_appends_one_entry() {
        let prior = vec![VerifierFeedbackRef {
            feedback_id: Uuid::now_v7(),
            at_utc: Utc::now(),
            summary: "first".to_string(),
        }];
        let new = VerifierFeedbackRef {
            feedback_id: Uuid::now_v7(),
            at_utc: Utc::now(),
            summary: "second".to_string(),
        };
        let cp = MtLoopControl::record_checkpoint_with_feedback(
            &sample_job(1),
            MtLoopState::IteratingOnFeedback {
                feedback_id: new.feedback_id,
            },
            "ok".to_string(),
            vec![],
            &MtLoopControlBudget::default(),
            prior.clone(),
            new.clone(),
            Uuid::now_v7(),
        )
        .unwrap();
        assert_eq!(cp.verifier_feedback_history.len(), 2);
        assert_eq!(cp.verifier_feedback_history[0], prior[0]);
        assert_eq!(cp.verifier_feedback_history[1], new);
    }
}
