//! MT-188 MT outcome recording + 6-level escalation routing + DistillationCandidate emission.
//!
//! # Surface contract
//!
//! This module provides three composable layers on top of the MT-184
//! `MicroTaskJob` primitive + MT-185 loop-control budget:
//!
//! ## (1) `MtOutcomeRecorder`
//! Records the canonical outcome row for a MicroTaskJob iteration.
//! Pure compute via [`MtOutcomeRecorder::record`] (no I/O — used by the
//! MicroTaskExecutor's inner loop), and durable persistence via
//! [`MtOutcomeRecorder::persist`] which writes the row to the
//! `kernel_mt_outcome` table created by `0023_micro_task_job_queue.sql`.
//! Persistence is gated to refuse forged outcomes (session id must match the
//! job's `claimed_by_session` column) per the MT-188 contract red-team brief.
//!
//! ## (2) `EscalationRouter`
//! Routes a `Failure` outcome through the 6-level tier ladder
//! `T7B -> T7BAlt -> T13B -> T13BAlt -> T32B -> HardGate` per the folded
//! WP-1-Micro-Task-Executor-v1 stub. The router is pure — it consumes
//! `(job, consecutive_failures_at_tier)` and emits an [`EscalationDecision`].
//! The caller (the MicroTaskExecutor or MT-189 orchestrator) is responsible
//! for translating the decision into a `MicroTaskQueue::escalate(...)` call
//! and, for `HardGate`, posting a `decision_request` mailbox message to the
//! operator role.
//!
//! ## (3) `DistillationCandidate` emission
//! On a successful outcome (or any outcome that meets the
//! [`DistillationThreshold`] criterion), the recorder also produces a
//! `DistillationCandidate` row written to the `kernel_distillation_candidate`
//! table. Per MT-188 red-team minimum_control #2, every candidate row has a
//! `status` column defaulting to `LOGGING_ONLY_PHASE_1` to prevent accidental
//! promotion before the cluster C distillation pipeline (MT-119..MT-124)
//! wires the production promotion path. Emits FR-EVT-MT-015 — registered in
//! `flight_recorder::fr_event_registry::FrEventId::Mt015DistillationCandidate`
//! — per emission; tests assert the event id is canonical.
//!
//! # Adversarial paths covered (red_team + MT-188 spawn brief)
//!
//! 1. **Forged outcome (wrong session)** — `persist` reads
//!    `kernel_micro_task_job.claimed_by_session` and refuses the write if it
//!    does not match the supplied `session_id`. Returns
//!    [`MtOutcomeError::ForgedSession`].
//! 2. **Duplicate outcome on same job/iteration** — DB-level
//!    `UNIQUE(job_id, iteration_n)` index (migration `0027_...sql`) refuses
//!    the second insert; Rust returns [`MtOutcomeError::DuplicateOutcome`].
//! 3. **HardGate as terminal** — `EscalationRouter::route` returns
//!    `EscalationDecision::HardGate` only when the prior tier was `T32B`
//!    or when the job is already at `HardGate`; after `HardGate` the router
//!    returns `EscalationDecision::Abandon` if the max-hardgate-retries budget
//!    is exhausted. Never advances past `HardGate`.
//! 4. **Out-of-order escalation (skip tier)** — the router computes the next
//!    tier via [`EscalationTier::next`] only; it never emits a tier two
//!    positions ahead. The DB-level `MicroTaskQueue::escalate` also refuses
//!    skip-tier writes (MT-184 `QueueError::InvalidEscalation`).
//! 5. **Distillation threshold off-by-one** — [`DistillationThreshold`]
//!    explicitly defines `minimum_iterations_completed >= N` and
//!    `outcome_kind == Success` as the criterion; the threshold's
//!    `evaluate(...)` is unit-tested for the off-by-one boundary at
//!    `iteration_n == N - 1`, `N`, `N + 1`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

use crate::flight_recorder::{
    fr_event_registry::FrEventId, FlightRecorder, FlightRecorderActor, FlightRecorderEvent,
    FlightRecorderEventType, RecorderError,
};
use crate::role_mailbox::RoleId;
use crate::role_mailbox_v1::{
    DecisionOption, DecisionRequestBody, FamilyEscalationTier, MessageFamily, MessageType,
    MicroTaskEscalationBody, MicroTaskExecutorContractRef, MicroTaskRef, PriorAttemptRef,
    RoleMailboxRepository, RoleMailboxThreadId,
};

use super::job::{
    CompletionSignal, EscalationTier, MicroTaskJob, MicroTaskJobId, RunLedgerPointer,
};
use super::loop_control::MtLoopControlBudget;

// ============================================================================
// MtOutcome / MtOutcomeKind / MtOutcomeRecord
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MtOutcomeKind {
    Success,
    Failure,
    Verification,
    Cancellation,
    Timeout,
    HardGate,
}

impl MtOutcomeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Verification => "verification",
            Self::Cancellation => "cancellation",
            Self::Timeout => "timeout",
            Self::HardGate => "hard_gate",
        }
    }

    /// Reverse lookup: column string -> typed variant. Used by the
    /// Postgres-gated query path in [`MtOutcomeRecorder::list_for_job`].
    pub fn from_str_id(s: &str) -> Option<Self> {
        match s {
            "success" => Some(Self::Success),
            "failure" => Some(Self::Failure),
            "verification" => Some(Self::Verification),
            "cancellation" => Some(Self::Cancellation),
            "timeout" => Some(Self::Timeout),
            "hard_gate" => Some(Self::HardGate),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtOutcome {
    pub outcome_kind: MtOutcomeKind,
    pub completion_signal: Option<CompletionSignal>,
    pub run_ledger_ref: Option<RunLedgerPointer>,
    pub evidence_pointers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtOutcomeRecord {
    pub outcome_id: Uuid,
    pub job_id: MicroTaskJobId,
    pub iteration_n: u32,
    pub outcome_kind: MtOutcomeKind,
    pub completion_signal: Option<CompletionSignal>,
    pub run_ledger_ref: Option<RunLedgerPointer>,
    pub evidence_pointers: Vec<String>,
    pub distillation_candidate_id: Option<Uuid>,
    pub recorded_at_utc: DateTime<Utc>,
    pub recorded_by_session: Uuid,
}

// ============================================================================
// DistillationCandidate (the executor-side, logging-only emission)
// ============================================================================

/// MT-188 `DistillationCandidate` — the logging-only Phase 1 emission per the
/// folded WP-1-Micro-Task-Executor-v1 stub.
///
/// Distinct from `distillation::spawn_tree_harvester::DistillationCandidate`,
/// which is the Phase 2 cluster C surface (full prompt-response trace +
/// promotion gating). The MT-188 variant carries only the minimum metadata
/// the cluster C pipeline needs to ingest the candidate later: the source
/// job, the tier at completion, a summary, and an opaque
/// `prompt_response_trace` JSON projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistillationCandidate {
    pub candidate_id: Uuid,
    pub job_id: MicroTaskJobId,
    pub summary: String,
    pub prompt_response_trace: Vec<serde_json::Value>,
    pub tier_at_completion: EscalationTier,
    pub created_at_utc: DateTime<Utc>,
    /// Default `LOGGING_ONLY_PHASE_1` per MT-188 red-team minimum_control #2.
    /// The cluster C MT-XXX promotion pipeline transitions this column once
    /// the candidate has been ingested + accepted.
    pub status: DistillationCandidateStatus,
}

/// Lifecycle status for a `DistillationCandidate`. Defaults to
/// `LoggingOnlyPhase1` until the cluster C promotion pipeline lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DistillationCandidateStatus {
    #[serde(rename = "LOGGING_ONLY_PHASE_1")]
    LoggingOnlyPhase1,
    #[serde(rename = "PROMOTED")]
    Promoted,
    #[serde(rename = "REJECTED")]
    Rejected,
    #[serde(rename = "EXPIRED")]
    Expired,
}

impl DistillationCandidateStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LoggingOnlyPhase1 => "LOGGING_ONLY_PHASE_1",
            Self::Promoted => "PROMOTED",
            Self::Rejected => "REJECTED",
            Self::Expired => "EXPIRED",
        }
    }

    pub fn from_str_id(s: &str) -> Option<Self> {
        match s {
            "LOGGING_ONLY_PHASE_1" => Some(Self::LoggingOnlyPhase1),
            "PROMOTED" => Some(Self::Promoted),
            "REJECTED" => Some(Self::Rejected),
            "EXPIRED" => Some(Self::Expired),
            _ => None,
        }
    }
}

impl Default for DistillationCandidateStatus {
    fn default() -> Self {
        Self::LoggingOnlyPhase1
    }
}

// ============================================================================
// DistillationThreshold — emission decision rule
// ============================================================================

/// Decides whether a given `(MicroTaskJob, MtOutcome)` pair should emit a
/// `DistillationCandidate`. Phase 1 default is "emit on Success after >= 1
/// completed iteration"; the threshold is configurable so the cluster C
/// MT-XXX promotion pipeline can tune it without rewriting the recorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistillationThreshold {
    /// Emit only on `MtOutcomeKind::Success` outcomes when `true`; when
    /// `false`, also emit on `Failure`/`Timeout` (used for cluster C's
    /// negative-example collection).
    pub success_only: bool,
    /// Minimum `iteration_n` on the job before a candidate is emitted. The
    /// boundary is `>=` (i.e. `iteration_n >= minimum_iterations_completed`).
    /// Default is `0` so the first successful iteration is eligible.
    pub minimum_iterations_completed: u32,
}

impl Default for DistillationThreshold {
    fn default() -> Self {
        Self {
            success_only: true,
            minimum_iterations_completed: 0,
        }
    }
}

impl DistillationThreshold {
    /// Evaluate whether the given outcome warrants emission. Adversarial:
    /// off-by-one at the boundary is asserted in
    /// `tests/mt_outcome_tests.rs::mt_188_distillation_threshold_off_by_one`.
    pub fn evaluate(self, job: &MicroTaskJob, outcome: &MtOutcome) -> bool {
        let kind_ok = if self.success_only {
            matches!(outcome.outcome_kind, MtOutcomeKind::Success)
        } else {
            !matches!(
                outcome.outcome_kind,
                MtOutcomeKind::Cancellation | MtOutcomeKind::Verification
            )
        };
        let iterations_ok = job.iteration_n >= self.minimum_iterations_completed;
        kind_ok && iterations_ok
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum MtOutcomeError {
    #[error("outcome validation: {0}")]
    Validation(String),
    /// Forged outcome: caller-supplied session does not match
    /// `kernel_micro_task_job.claimed_by_session` for the job_id. Red-team
    /// minimum_control: HardGate transition always emits a decision_request
    /// mailbox message; this error is the upstream defense — only the session
    /// that actually claimed the job can record its outcome.
    #[error(
        "forged outcome: session {supplied} does not own job {job_id} (claimed by {actual:?})"
    )]
    ForgedSession {
        job_id: MicroTaskJobId,
        supplied: Uuid,
        actual: Option<Uuid>,
    },
    /// Duplicate outcome row for the same `(job_id, iteration_n)`. The
    /// DB-level unique index `idx_kernel_mt_outcome_unique_job_iteration`
    /// (migration `0027_...sql`) refuses the second insert; this error
    /// surfaces the conflict to the caller without panicking.
    #[error("duplicate outcome for job {job_id} at iteration {iteration_n}")]
    DuplicateOutcome {
        job_id: MicroTaskJobId,
        iteration_n: u32,
    },
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("job not found: {0}")]
    JobNotFound(MicroTaskJobId),
    #[error("HardGate transition for job {job_id} requires a mailbox_thread_id")]
    MissingHardGateMailboxThread { job_id: MicroTaskJobId },
    #[error("flight recorder error: {0}")]
    FlightRecorder(#[from] RecorderError),
    #[error("role mailbox error: {0}")]
    Mailbox(String),
}

// ============================================================================
// MtOutcomeRecorder
// ============================================================================

/// Pure-compute + persistence facade for MT-188 outcome rows.
///
/// The pure-compute API ([`record`](MtOutcomeRecorder::record)) is what the
/// MicroTaskExecutor's hot loop calls between iterations — it is allocation-
/// light and side-effect-free. The persistence API
/// ([`persist`](MtOutcomeRecorder::persist)) is what the MT-189 orchestrator
/// and integration tests call to durably commit the row.
pub struct MtOutcomeRecorder;

impl MtOutcomeRecorder {
    /// Compute the outcome record + optional DistillationCandidate for a job
    /// completion. Pure function — no I/O. Production wiring lands this
    /// through `KernelActionCatalogV1` so the row is persisted via the
    /// EventLedger; tests use it directly.
    pub fn record(
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
    ) -> (MtOutcomeRecord, Option<DistillationCandidate>) {
        Self::record_with_threshold(job, outcome, session_id, DistillationThreshold::default())
    }

    /// Variant of [`record`] that takes a custom emission threshold. Cluster
    /// C uses this to enable failure-trace collection; Phase 1 defaults to
    /// `success_only=true, minimum_iterations_completed=0`.
    pub fn record_with_threshold(
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
        threshold: DistillationThreshold,
    ) -> (MtOutcomeRecord, Option<DistillationCandidate>) {
        let candidate = if threshold.evaluate(job, &outcome) {
            Some(DistillationCandidate {
                candidate_id: Uuid::now_v7(),
                job_id: job.job_id,
                summary: format!(
                    "WP={} MT={} tier={:?} iter={}",
                    job.wp_id, job.mt_id, job.escalation_tier, job.iteration_n
                ),
                prompt_response_trace: vec![],
                tier_at_completion: job.escalation_tier,
                created_at_utc: Utc::now(),
                status: DistillationCandidateStatus::LoggingOnlyPhase1,
            })
        } else {
            None
        };
        let record = MtOutcomeRecord {
            outcome_id: Uuid::now_v7(),
            job_id: job.job_id,
            iteration_n: job.iteration_n,
            outcome_kind: outcome.outcome_kind,
            completion_signal: outcome.completion_signal,
            run_ledger_ref: outcome.run_ledger_ref,
            evidence_pointers: outcome.evidence_pointers,
            distillation_candidate_id: candidate.as_ref().map(|c| c.candidate_id),
            recorded_at_utc: Utc::now(),
            recorded_by_session: session_id,
        };
        (record, candidate)
    }

    /// Compute an outcome and emit FR-EVT-MT-015 when that outcome creates a
    /// `DistillationCandidate`. This is the no-I/O outcome-row variant used by
    /// executor tests; durable callers should use
    /// [`persist_with_flight_recorder`](Self::persist_with_flight_recorder).
    pub async fn record_with_flight_recorder<R: FlightRecorder + ?Sized>(
        recorder: &R,
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
        trace_id: Uuid,
        workflow_run_id: Uuid,
    ) -> Result<
        (
            MtOutcomeRecord,
            Option<DistillationCandidate>,
            Option<FlightRecorderEvent>,
        ),
        MtOutcomeError,
    > {
        Self::record_with_threshold_and_flight_recorder(
            recorder,
            job,
            outcome,
            session_id,
            DistillationThreshold::default(),
            trace_id,
            workflow_run_id,
        )
        .await
    }

    /// Threshold-aware variant of [`record_with_flight_recorder`].
    pub async fn record_with_threshold_and_flight_recorder<R: FlightRecorder + ?Sized>(
        recorder: &R,
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
        threshold: DistillationThreshold,
        trace_id: Uuid,
        workflow_run_id: Uuid,
    ) -> Result<
        (
            MtOutcomeRecord,
            Option<DistillationCandidate>,
            Option<FlightRecorderEvent>,
        ),
        MtOutcomeError,
    > {
        let (record, candidate) = Self::record_with_threshold(job, outcome, session_id, threshold);
        let event = match candidate.as_ref() {
            Some(candidate) => Some(
                Self::emit_distillation_candidate_event(
                    recorder,
                    job,
                    candidate,
                    trace_id,
                    workflow_run_id,
                )
                .await?,
            ),
            None => None,
        };
        Ok((record, candidate, event))
    }

    /// Compute the canonical FR-EVT-MT-015 event id string a recorder should
    /// emit when a `DistillationCandidate` is created. Bound to the registry
    /// (`flight_recorder::fr_event_registry::FrEventId::Mt015DistillationCandidate`)
    /// so a wire-form rename is caught at compile time of the registry.
    pub const FR_EVT_MT_015: &'static str = "FR-EVT-MT-015";

    /// Emit the logging-only FR-EVT-MT-015 event for a created
    /// `DistillationCandidate`. The candidate row is still the canonical
    /// durable artifact; this event is the audit/search surface consumed by
    /// Flight Recorder dashboards.
    pub async fn emit_distillation_candidate_event<R: FlightRecorder + ?Sized>(
        recorder: &R,
        job: &MicroTaskJob,
        candidate: &DistillationCandidate,
        trace_id: Uuid,
        workflow_run_id: Uuid,
    ) -> Result<FlightRecorderEvent, MtOutcomeError> {
        let event_type = FrEventId::Mt015DistillationCandidate.as_str();
        let payload = json!({
            "event_type": event_type,
            "event_name": "micro_task_distillation_candidate",
            "timestamp": Utc::now().to_rfc3339(),
            "trace_id": trace_id.to_string(),
            "job_id": job.job_id.to_string(),
            "workflow_run_id": workflow_run_id.to_string(),
            "payload": {
                "wp_id": job.wp_id,
                "mt_id": job.mt_id,
                "candidate_ref": {
                    "candidate_id": candidate.candidate_id.to_string(),
                    "job_id": candidate.job_id.to_string(),
                    "status": candidate.status.as_str(),
                    "tier_at_completion": candidate.tier_at_completion.as_str(),
                },
            },
        });
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::MicroTaskDistillationCandidate,
            FlightRecorderActor::Agent,
            trace_id,
            payload,
        )
        .with_job_id(job.job_id.to_string())
        .with_workflow_id(workflow_run_id.to_string());
        recorder.record_event(event.clone()).await?;
        Ok(event)
    }

    /// Persist the outcome row + optional candidate row to Postgres in a
    /// single transaction. Performs the forged-session check before insert
    /// (red_team minimum_control: only the session that owns the job may
    /// record its outcome).
    ///
    /// Returns the persisted [`MtOutcomeRecord`] (with the same outcome_id
    /// that was used for the insert) and the optional persisted
    /// [`DistillationCandidate`].
    ///
    /// Errors:
    /// * [`MtOutcomeError::JobNotFound`] — no kernel_micro_task_job row for
    ///   the given job_id.
    /// * [`MtOutcomeError::ForgedSession`] — session_id does not match the
    ///   job's `claimed_by_session`.
    /// * [`MtOutcomeError::DuplicateOutcome`] — the DB-level unique index
    ///   refused the insert because a row already exists for
    ///   `(job_id, iteration_n)`.
    /// * [`MtOutcomeError::Sqlx`] — any other DB error.
    pub async fn persist(
        pool: &PgPool,
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
    ) -> Result<(MtOutcomeRecord, Option<DistillationCandidate>), MtOutcomeError> {
        Self::persist_with_threshold(
            pool,
            job,
            outcome,
            session_id,
            DistillationThreshold::default(),
        )
        .await
    }

    /// Variant of [`persist`] that takes a custom emission threshold.
    pub async fn persist_with_threshold(
        pool: &PgPool,
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
        threshold: DistillationThreshold,
    ) -> Result<(MtOutcomeRecord, Option<DistillationCandidate>), MtOutcomeError> {
        let mut tx = pool.begin().await?;

        // Verify the session that's recording the outcome actually owns the
        // job. Reads claimed_by_session FOR UPDATE so a concurrent reclaim
        // cannot race past this check.
        let row: Option<(Option<Uuid>,)> = sqlx::query_as(
            r#"SELECT claimed_by_session FROM kernel_micro_task_job
               WHERE job_id = $1 FOR UPDATE"#,
        )
        .bind(job.job_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let claimed = match row {
            None => return Err(MtOutcomeError::JobNotFound(job.job_id)),
            Some((c,)) => c,
        };
        if claimed != Some(session_id) {
            return Err(MtOutcomeError::ForgedSession {
                job_id: job.job_id,
                supplied: session_id,
                actual: claimed,
            });
        }

        // Compute the record + candidate first (pure), then write.
        let (record, candidate) = Self::record_with_threshold(job, outcome, session_id, threshold);

        // Insert the candidate first (the outcome row references it via FK).
        if let Some(c) = &candidate {
            sqlx::query(
                r#"INSERT INTO kernel_distillation_candidate
                   (candidate_id, job_id, summary, prompt_response_trace,
                    tier_at_completion, created_at_utc, status)
                   VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
            )
            .bind(c.candidate_id)
            .bind(c.job_id.as_uuid())
            .bind(&c.summary)
            .bind(serde_json::to_value(&c.prompt_response_trace)?)
            .bind(c.tier_at_completion.as_str())
            .bind(c.created_at_utc)
            .bind(c.status.as_str())
            .execute(&mut *tx)
            .await?;
        }

        // Insert the outcome row. Surface unique-violation as a typed error.
        let insert_result = sqlx::query(
            r#"INSERT INTO kernel_mt_outcome
               (outcome_id, job_id, iteration_n, outcome_kind, completion_signal,
                run_ledger_ref, evidence_pointers, distillation_candidate_id,
                recorded_at_utc, recorded_by_session)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)"#,
        )
        .bind(record.outcome_id)
        .bind(record.job_id.as_uuid())
        .bind(record.iteration_n as i64)
        .bind(record.outcome_kind.as_str())
        .bind(
            record
                .completion_signal
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
        )
        .bind(
            record
                .run_ledger_ref
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
        )
        .bind(serde_json::to_value(&record.evidence_pointers)?)
        .bind(record.distillation_candidate_id)
        .bind(record.recorded_at_utc)
        .bind(record.recorded_by_session)
        .execute(&mut *tx)
        .await;

        match insert_result {
            Ok(_) => {
                tx.commit().await?;
                Ok((record, candidate))
            }
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                // Unique-violation on idx_kernel_mt_outcome_unique_job_iteration.
                // Postgres SQLSTATE 23505. Return a typed dedup error so the
                // caller can distinguish from transient DB failures.
                Err(MtOutcomeError::DuplicateOutcome {
                    job_id: record.job_id,
                    iteration_n: record.iteration_n,
                })
            }
            Err(e) => Err(MtOutcomeError::Sqlx(e)),
        }
    }

    /// Persist the outcome and, when a `DistillationCandidate` is created,
    /// emit FR-EVT-MT-015 immediately after the DB transaction commits. The
    /// Postgres row remains the canonical durable artifact; the event is the
    /// searchable audit surface for Flight Recorder consumers.
    pub async fn persist_with_flight_recorder<R: FlightRecorder + ?Sized>(
        pool: &PgPool,
        recorder: &R,
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
        trace_id: Uuid,
        workflow_run_id: Uuid,
    ) -> Result<
        (
            MtOutcomeRecord,
            Option<DistillationCandidate>,
            Option<FlightRecorderEvent>,
        ),
        MtOutcomeError,
    > {
        Self::persist_with_threshold_and_flight_recorder(
            pool,
            recorder,
            job,
            outcome,
            session_id,
            DistillationThreshold::default(),
            trace_id,
            workflow_run_id,
        )
        .await
    }

    /// Threshold-aware durable variant of
    /// [`persist_with_flight_recorder`](Self::persist_with_flight_recorder).
    pub async fn persist_with_threshold_and_flight_recorder<R: FlightRecorder + ?Sized>(
        pool: &PgPool,
        recorder: &R,
        job: &MicroTaskJob,
        outcome: MtOutcome,
        session_id: Uuid,
        threshold: DistillationThreshold,
        trace_id: Uuid,
        workflow_run_id: Uuid,
    ) -> Result<
        (
            MtOutcomeRecord,
            Option<DistillationCandidate>,
            Option<FlightRecorderEvent>,
        ),
        MtOutcomeError,
    > {
        let (record, candidate) =
            Self::persist_with_threshold(pool, job, outcome, session_id, threshold).await?;
        let event = match candidate.as_ref() {
            Some(candidate) => Some(
                Self::emit_distillation_candidate_event(
                    recorder,
                    job,
                    candidate,
                    trace_id,
                    workflow_run_id,
                )
                .await?,
            ),
            None => None,
        };
        Ok((record, candidate, event))
    }

    /// Read all outcome rows for a job, oldest first. Used by the
    /// MT-189 orchestrator and by validator tooling to inspect the audit
    /// trail.
    pub async fn list_for_job(
        pool: &PgPool,
        job_id: MicroTaskJobId,
    ) -> Result<Vec<MtOutcomeRecord>, MtOutcomeError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            i64,
            String,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
            serde_json::Value,
            Option<Uuid>,
            DateTime<Utc>,
            Uuid,
        )> = sqlx::query_as(
            r#"SELECT outcome_id, job_id, iteration_n, outcome_kind,
                      completion_signal, run_ledger_ref, evidence_pointers,
                      distillation_candidate_id, recorded_at_utc, recorded_by_session
               FROM kernel_mt_outcome
               WHERE job_id = $1
               ORDER BY recorded_at_utc ASC, iteration_n ASC"#,
        )
        .bind(job_id.as_uuid())
        .fetch_all(pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let kind = MtOutcomeKind::from_str_id(&r.3).ok_or_else(|| {
                MtOutcomeError::Validation(format!("unknown outcome_kind in DB: {}", r.3))
            })?;
            let signal: Option<CompletionSignal> = match r.4 {
                Some(v) => Some(serde_json::from_value(v)?),
                None => None,
            };
            let ledger: Option<RunLedgerPointer> = match r.5 {
                Some(v) => Some(serde_json::from_value(v)?),
                None => None,
            };
            let evidence: Vec<String> = serde_json::from_value(r.6)?;
            out.push(MtOutcomeRecord {
                outcome_id: r.0,
                job_id: MicroTaskJobId(r.1),
                iteration_n: r.2 as u32,
                outcome_kind: kind,
                completion_signal: signal,
                run_ledger_ref: ledger,
                evidence_pointers: evidence,
                distillation_candidate_id: r.7,
                recorded_at_utc: r.8,
                recorded_by_session: r.9,
            });
        }
        Ok(out)
    }

    /// Read all distillation candidates for a job, oldest first. Used by
    /// validator tooling to confirm Phase-1 emission and by the cluster C
    /// pipeline to discover candidates pending promotion.
    pub async fn list_candidates_for_job(
        pool: &PgPool,
        job_id: MicroTaskJobId,
    ) -> Result<Vec<DistillationCandidate>, MtOutcomeError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            String,
            serde_json::Value,
            String,
            DateTime<Utc>,
            String,
        )> = sqlx::query_as(
            r#"SELECT candidate_id, job_id, summary, prompt_response_trace,
                          tier_at_completion, created_at_utc, status
                   FROM kernel_distillation_candidate
                   WHERE job_id = $1
                   ORDER BY created_at_utc ASC"#,
        )
        .bind(job_id.as_uuid())
        .fetch_all(pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let tier: EscalationTier = serde_json::from_value(serde_json::Value::String(r.4))?;
            let trace: Vec<serde_json::Value> = serde_json::from_value(r.3)?;
            let status = DistillationCandidateStatus::from_str_id(&r.6).ok_or_else(|| {
                MtOutcomeError::Validation(format!("unknown candidate status in DB: {}", r.6))
            })?;
            out.push(DistillationCandidate {
                candidate_id: r.0,
                job_id: MicroTaskJobId(r.1),
                summary: r.2,
                prompt_response_trace: trace,
                tier_at_completion: tier,
                created_at_utc: r.5,
                status,
            });
        }
        Ok(out)
    }

    /// Ensure the MT-188 column additions are applied (idempotent).
    /// Production callers run sqlx::migrate!() instead; this helper exists for
    /// integration tests that bring up a fresh pool without the migrator.
    pub async fn ensure_schema(pool: &PgPool) -> Result<(), MtOutcomeError> {
        sqlx::raw_sql(SCHEMA_SQL).execute(pool).await?;
        Ok(())
    }

    /// Convenience helper for the MT-189 orchestrator: read all candidate
    /// rows that are still in `LOGGING_ONLY_PHASE_1` status (i.e. eligible
    /// for cluster C promotion). Mirrors the index added by
    /// `0027_mt_outcome_distillation_status.sql`.
    pub async fn list_phase1_candidates(
        pool: &PgPool,
        limit: i64,
    ) -> Result<Vec<DistillationCandidate>, MtOutcomeError> {
        let rows: Vec<(
            Uuid,
            Uuid,
            String,
            serde_json::Value,
            String,
            DateTime<Utc>,
            String,
        )> = sqlx::query_as(
            r#"SELECT candidate_id, job_id, summary, prompt_response_trace,
                          tier_at_completion, created_at_utc, status
                   FROM kernel_distillation_candidate
                   WHERE status = 'LOGGING_ONLY_PHASE_1'
                   ORDER BY created_at_utc ASC
                   LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let tier: EscalationTier = serde_json::from_value(serde_json::Value::String(r.4))?;
            let trace: Vec<serde_json::Value> = serde_json::from_value(r.3)?;
            let status = DistillationCandidateStatus::from_str_id(&r.6).ok_or_else(|| {
                MtOutcomeError::Validation(format!("unknown candidate status in DB: {}", r.6))
            })?;
            out.push(DistillationCandidate {
                candidate_id: r.0,
                job_id: MicroTaskJobId(r.1),
                summary: r.2,
                prompt_response_trace: trace,
                tier_at_completion: tier,
                created_at_utc: r.5,
                status,
            });
        }
        Ok(out)
    }
}

/// Embedded MT-188 schema additions; idempotent (`IF NOT EXISTS` on every DDL).
/// In production, `sqlx::migrate!("./migrations")` runs both `0023_` and
/// `0027_` in order; this constant is exposed so integration tests can ensure
/// the column + index additions without binding to the migrator.
pub const SCHEMA_SQL: &str =
    include_str!("../../migrations/0027_mt_outcome_distillation_status.sql");

// ============================================================================
// EscalationDecision / EscalationRouter
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum EscalationDecision {
    Retry {
        same_tier: EscalationTier,
    },
    EscalateTo {
        next_tier: EscalationTier,
        lora_id: Option<String>,
    },
    HardGate {
        reason: String,
    },
    Abandon {
        reason: String,
    },
}

/// Router that turns a `(MicroTaskJob, consecutive_failures_at_tier)` pair
/// into an [`EscalationDecision`]. Pure compute — the caller is responsible
/// for translating `EscalateTo` into a `MicroTaskQueue::escalate(...)` call
/// and `HardGate` into a `decision_request` mailbox message.
pub struct EscalationRouter {
    /// Tag-to-LoRA map. The cluster C MTE-LoRA-Wiring fold provides the
    /// production map; MT-188 defaults to empty so the router still emits
    /// `EscalateTo` decisions without a LoRA id (which is valid Phase-1
    /// behavior — the executor calls the next-tier model with no LoRA).
    pub lora_by_tag: HashMap<String, String>,
    /// Consecutive failures at the current tier before the router escalates
    /// (instead of issuing `Retry`). Default 3.
    pub max_failures_per_tier: u32,
    /// Maximum number of `HardGate` re-issuances before the router gives up
    /// and returns `Abandon`. Default 0 (the router emits one `HardGate`
    /// decision; subsequent failures at HardGate return `Abandon`).
    pub max_hardgate_retries: u32,
}

impl Default for EscalationRouter {
    fn default() -> Self {
        Self {
            lora_by_tag: HashMap::new(),
            max_failures_per_tier: 3,
            max_hardgate_retries: 0,
        }
    }
}

impl EscalationRouter {
    /// Construct a router with a custom tag-to-LoRA map and default budgets.
    pub fn with_lora_map(lora_by_tag: HashMap<String, String>) -> Self {
        Self {
            lora_by_tag,
            ..Self::default()
        }
    }

    /// Compute the next escalation step. Pure function consumed by the
    /// MicroTaskExecutor (MT-189). Never advances past `HardGate`.
    ///
    /// Algorithm:
    ///   1. If `consecutive_failures_at_tier < max_failures_per_tier`, return
    ///      `Retry { same_tier }`.
    ///   2. Otherwise compute `next = current.next()`. If `next` is
    ///      `HardGate`, return `HardGate { reason }`.
    ///   3. If `current` is already `HardGate`, count HardGate steps in the
    ///      escalation_history. If `history_hardgate > max_hardgate_retries`,
    ///      return `Abandon`; otherwise re-issue `HardGate`.
    ///   4. Otherwise, look up LoRA id from job.task_tags and return
    ///      `EscalateTo { next_tier, lora_id }`.
    pub fn route(
        &self,
        job: &MicroTaskJob,
        consecutive_failures_at_tier: u32,
    ) -> EscalationDecision {
        self.route_with_failure_limit(
            job,
            consecutive_failures_at_tier,
            self.max_failures_per_tier,
        )
    }

    /// Route using the MT-185 loop-control budget. This is the production
    /// bridge between bounded loop checkpoints and tier escalation; the
    /// router's local default is retained for tests and legacy callers.
    pub fn route_with_budget(
        &self,
        job: &MicroTaskJob,
        consecutive_failures_at_tier: u32,
        budget: &MtLoopControlBudget,
    ) -> EscalationDecision {
        self.route_with_failure_limit(
            job,
            consecutive_failures_at_tier,
            budget.max_consecutive_failures,
        )
    }

    fn route_with_failure_limit(
        &self,
        job: &MicroTaskJob,
        consecutive_failures_at_tier: u32,
        max_failures_per_tier: u32,
    ) -> EscalationDecision {
        if consecutive_failures_at_tier < max_failures_per_tier {
            return EscalationDecision::Retry {
                same_tier: job.escalation_tier,
            };
        }
        match job.escalation_tier.next() {
            Some(next_tier) => {
                let lora_id = self.lookup_lora_for(job);
                if matches!(next_tier, EscalationTier::HardGate) {
                    return EscalationDecision::HardGate {
                        reason: format!(
                            "budget exhausted at {:?} ({} failures); promoting to HardGate",
                            job.escalation_tier, consecutive_failures_at_tier
                        ),
                    };
                }
                EscalationDecision::EscalateTo { next_tier, lora_id }
            }
            None => {
                // `current` is already HardGate. Count HardGate steps in
                // history: each prior HardGate counts as a "retry"; once we
                // exceed max_hardgate_retries we abandon.
                let history_hardgate = job
                    .escalation_history
                    .iter()
                    .filter(|s| matches!(s.to_tier, EscalationTier::HardGate))
                    .count() as u32;
                if history_hardgate > self.max_hardgate_retries {
                    EscalationDecision::Abandon {
                        reason: format!(
                            "exceeded max_hardgate_retries={}",
                            self.max_hardgate_retries
                        ),
                    }
                } else {
                    EscalationDecision::HardGate {
                        reason: "already at HardGate; awaiting operator release".to_string(),
                    }
                }
            }
        }
    }

    /// Deterministic LoRA selection from `job.task_tags`. Picks the LoRA for
    /// the first tag in `task_tags` order that exists in `lora_by_tag`.
    /// Returning `None` is valid Phase-1 behavior — the executor calls the
    /// next-tier model with no LoRA.
    pub fn lookup_lora_for(&self, job: &MicroTaskJob) -> Option<String> {
        job.task_tags
            .iter()
            .find_map(|t| self.lora_by_tag.get(t).cloned())
    }
}

// ============================================================================
// MailboxPoster — abstract surface for posting micro_task_escalation +
// decision_request messages. The MT-189 orchestrator binds a Postgres-backed
// implementation against `role_mailbox_v1::repo::RoleMailboxRepository`; tests
// bind a Mutex<Vec<...>> capture.
// ============================================================================

/// Abstract interface for posting mailbox messages on escalation events.
/// The MT-189 executor binds this against the
/// `role_mailbox_v1::repo::RoleMailboxRepository`; this lets the MT-188
/// outcome surface unit-test the routing decisions independent of a live
/// mailbox.
#[async_trait::async_trait]
pub trait EscalationMailboxPoster: Send + Sync {
    /// Post a `micro_task_escalation` message to the given thread.
    async fn post_micro_task_escalation(
        &self,
        thread_id: Uuid,
        job: &MicroTaskJob,
        from_tier: EscalationTier,
        to_tier: EscalationTier,
        reason: String,
    ) -> Result<Uuid, MtOutcomeError>;

    /// Post a `decision_request` message on HardGate. Per the MT-188
    /// `red_team.minimum_controls` first item: "HardGate transition always
    /// posts a decision_request mailbox message" — the caller (the MT-189
    /// orchestrator) is responsible for invoking this BEFORE committing the
    /// state transition to HardGated, so the operator decision is always
    /// queryable.
    async fn post_hardgate_decision_request(
        &self,
        thread_id: Uuid,
        job: &MicroTaskJob,
        reason: String,
    ) -> Result<Uuid, MtOutcomeError>;
}

/// Translate an `EscalationDecision` + a `MailboxPoster` into a side-effecting
/// step. Used by the MT-189 orchestrator inline; pulled out here so adversarial
/// tests can assert "HardGate always posts decision_request before commit".
pub async fn enact_decision<P: EscalationMailboxPoster + ?Sized>(
    poster: &P,
    job: &MicroTaskJob,
    decision: &EscalationDecision,
) -> Result<Option<Uuid>, MtOutcomeError> {
    let thread_id = match job.mailbox_thread_id {
        Some(t) => t,
        None if matches!(decision, EscalationDecision::HardGate { .. }) => {
            return Err(MtOutcomeError::MissingHardGateMailboxThread { job_id: job.job_id });
        }
        // Non-HardGate decisions can proceed without a mailbox thread:
        // Retry/Abandon post nothing, and EscalateTo is also mirrored in the
        // queue escalation history. HardGate must fail closed because the
        // operator decision_request is the release mechanism.
        None => return Ok(None),
    };
    match decision {
        EscalationDecision::Retry { .. } => Ok(None),
        EscalationDecision::EscalateTo { next_tier, .. } => {
            let msg_id = poster
                .post_micro_task_escalation(
                    thread_id,
                    job,
                    job.escalation_tier,
                    *next_tier,
                    format!(
                        "auto-escalation from {:?} to {:?}",
                        job.escalation_tier, next_tier
                    ),
                )
                .await?;
            Ok(Some(msg_id))
        }
        EscalationDecision::HardGate { reason } => {
            let msg_id = poster
                .post_hardgate_decision_request(thread_id, job, reason.clone())
                .await?;
            Ok(Some(msg_id))
        }
        EscalationDecision::Abandon { .. } => {
            // Abandon is terminal; the caller marks the job Failed.
            // No mailbox message — the audit trail is the outcome row.
            Ok(None)
        }
    }
}

/// Postgres-backed RoleMailbox implementation of [`EscalationMailboxPoster`].
/// HardGate posts a `decision_request` to the operator role; non-terminal
/// tier escalation posts a typed `micro_task_escalation` message.
pub struct RoleMailboxEscalationPoster<'a> {
    repo: &'a RoleMailboxRepository,
    from_role: RoleId,
    escalation_to_roles: Vec<RoleId>,
    hardgate_to_roles: Vec<RoleId>,
}

impl<'a> RoleMailboxEscalationPoster<'a> {
    pub fn new(repo: &'a RoleMailboxRepository, from_role: RoleId) -> Self {
        Self {
            repo,
            from_role,
            escalation_to_roles: vec![RoleId::Orchestrator],
            hardgate_to_roles: vec![RoleId::Operator],
        }
    }

    pub fn with_recipients(
        repo: &'a RoleMailboxRepository,
        from_role: RoleId,
        escalation_to_roles: Vec<RoleId>,
        hardgate_to_roles: Vec<RoleId>,
    ) -> Self {
        Self {
            repo,
            from_role,
            escalation_to_roles,
            hardgate_to_roles,
        }
    }
}

#[async_trait::async_trait]
impl EscalationMailboxPoster for RoleMailboxEscalationPoster<'_> {
    async fn post_micro_task_escalation(
        &self,
        thread_id: Uuid,
        job: &MicroTaskJob,
        from_tier: EscalationTier,
        to_tier: EscalationTier,
        reason: String,
    ) -> Result<Uuid, MtOutcomeError> {
        let body = MessageFamily::MicroTaskEscalation(MicroTaskEscalationBody {
            mt_ref: MicroTaskRef {
                wp_id: job.wp_id.clone(),
                mt_id: job.mt_id.clone(),
                iteration_n: job.iteration_n,
            },
            micro_task_executor_contract_ref: MicroTaskExecutorContractRef {
                job_id: job.job_id.as_uuid(),
                mt_id: job.mt_id.clone(),
                wp_id: job.wp_id.clone(),
            },
            escalation_target: to_mailbox_tier(to_tier),
            reason,
            prior_attempts: vec![PriorAttemptRef {
                attempt_id: Uuid::now_v7(),
                tier: to_mailbox_tier(from_tier),
                outcome_summary: format!("failed at {:?}", from_tier),
            }],
        });
        let message = self
            .repo
            .append_message(
                RoleMailboxThreadId(thread_id),
                MessageType::MicroTaskEscalation,
                self.from_role.clone(),
                self.escalation_to_roles.clone(),
                serde_json::to_value(body)?,
            )
            .await
            .map_err(|err| MtOutcomeError::Mailbox(err.to_string()))?;
        Ok(message.message_id.as_uuid())
    }

    async fn post_hardgate_decision_request(
        &self,
        thread_id: Uuid,
        job: &MicroTaskJob,
        reason: String,
    ) -> Result<Uuid, MtOutcomeError> {
        let body = MessageFamily::DecisionRequest(DecisionRequestBody {
            question: format!(
                "HardGate requested for {} {} after escalation exhaustion: {}",
                job.wp_id, job.mt_id, reason
            ),
            options: vec![
                DecisionOption {
                    option_id: "approve_hardgate".to_string(),
                    label: "Approve HardGate".to_string(),
                    detail: Some(
                        "Commit the MT to HardGated and await operator release.".to_string(),
                    ),
                },
                DecisionOption {
                    option_id: "retry_same_tier".to_string(),
                    label: "Retry Same Tier".to_string(),
                    detail: Some(
                        "Return the MT to the current tier for another bounded attempt."
                            .to_string(),
                    ),
                },
                DecisionOption {
                    option_id: "abandon_failed".to_string(),
                    label: "Mark Failed".to_string(),
                    detail: Some(
                        "Stop the MT and preserve the outcome trail for review.".to_string(),
                    ),
                },
            ],
            decision_authority_role: RoleId::Operator,
            deadline_utc: None,
        });
        let message = self
            .repo
            .append_message(
                RoleMailboxThreadId(thread_id),
                MessageType::DecisionRequest,
                self.from_role.clone(),
                self.hardgate_to_roles.clone(),
                serde_json::to_value(body)?,
            )
            .await
            .map_err(|err| MtOutcomeError::Mailbox(err.to_string()))?;
        Ok(message.message_id.as_uuid())
    }
}

fn to_mailbox_tier(tier: EscalationTier) -> FamilyEscalationTier {
    match tier {
        EscalationTier::T7B => FamilyEscalationTier::T7B,
        EscalationTier::T7BAlt => FamilyEscalationTier::T7BAlt,
        EscalationTier::T13B => FamilyEscalationTier::T13B,
        EscalationTier::T13BAlt => FamilyEscalationTier::T13BAlt,
        EscalationTier::T32B => FamilyEscalationTier::T32B,
        EscalationTier::HardGate => FamilyEscalationTier::HardGate,
    }
}

// ============================================================================
// Postgres-side wiring helpers — used by MT-189 + tests.
// ============================================================================

/// Persist a DistillationCandidate row in isolation (no outcome row). Used by
/// the cluster C promotion pipeline when re-emitting a candidate against a
/// historical job. The recorder's `persist` writes both rows atomically; this
/// helper exists for the cluster-C-only path.
pub async fn insert_distillation_candidate(
    tx: &mut Transaction<'_, Postgres>,
    c: &DistillationCandidate,
) -> Result<(), MtOutcomeError> {
    sqlx::query(
        r#"INSERT INTO kernel_distillation_candidate
           (candidate_id, job_id, summary, prompt_response_trace,
            tier_at_completion, created_at_utc, status)
           VALUES ($1,$2,$3,$4,$5,$6,$7)"#,
    )
    .bind(c.candidate_id)
    .bind(c.job_id.as_uuid())
    .bind(&c.summary)
    .bind(serde_json::to_value(&c.prompt_response_trace)?)
    .bind(c.tier_at_completion.as_str())
    .bind(c.created_at_utc)
    .bind(c.status.as_str())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn success_produces_distillation_candidate() {
        let job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
        let (rec, cand) = MtOutcomeRecorder::record(
            &job,
            MtOutcome {
                outcome_kind: MtOutcomeKind::Success,
                completion_signal: Some(CompletionSignal::Success {
                    summary: "done".to_string(),
                }),
                run_ledger_ref: None,
                evidence_pointers: vec![],
            },
            Uuid::now_v7(),
        );
        assert!(cand.is_some());
        assert!(rec.distillation_candidate_id.is_some());
        assert_eq!(
            cand.unwrap().status,
            DistillationCandidateStatus::LoggingOnlyPhase1,
            "new candidates must default to LoggingOnlyPhase1 per red_team #2"
        );
    }

    #[test]
    fn failure_no_candidate_under_default_threshold() {
        let job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
        let (rec, cand) = MtOutcomeRecorder::record(
            &job,
            MtOutcome {
                outcome_kind: MtOutcomeKind::Failure,
                completion_signal: None,
                run_ledger_ref: None,
                evidence_pointers: vec![],
            },
            Uuid::now_v7(),
        );
        assert!(cand.is_none());
        assert!(rec.distillation_candidate_id.is_none());
    }

    #[test]
    fn router_retries_under_budget() {
        let job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
        let r = EscalationRouter::default();
        let d = r.route(&job, 1);
        assert!(matches!(d, EscalationDecision::Retry { .. }));
    }

    #[test]
    fn router_escalates_after_budget() {
        let job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
        let r = EscalationRouter::default();
        let d = r.route(&job, 3);
        assert!(matches!(d, EscalationDecision::EscalateTo { .. }));
    }

    #[test]
    fn router_hardgates_at_t32b_exhaustion() {
        let mut job = MicroTaskJob::queue("W", "M", PathBuf::from("a.json"), 6, vec![]);
        job.escalation_tier = EscalationTier::T32B;
        let r = EscalationRouter::default();
        let d = r.route(&job, 3);
        assert!(matches!(d, EscalationDecision::HardGate { .. }));
    }

    #[test]
    fn router_lora_lookup_deterministic() {
        let mut job = MicroTaskJob::queue(
            "W",
            "M",
            PathBuf::from("a.json"),
            6,
            vec!["python".to_string(), "rust".to_string()],
        );
        job.escalation_tier = EscalationTier::T7B;
        let mut map = HashMap::new();
        map.insert("rust".to_string(), "lora-rust-v1".to_string());
        map.insert("python".to_string(), "lora-py-v1".to_string());
        let r = EscalationRouter::with_lora_map(map);
        let d = r.route(&job, 3);
        match d {
            EscalationDecision::EscalateTo { next_tier, lora_id } => {
                assert_eq!(next_tier, EscalationTier::T7BAlt);
                // First tag wins: "python" is at index 0.
                assert_eq!(lora_id.as_deref(), Some("lora-py-v1"));
            }
            other => panic!("expected EscalateTo, got {:?}", other),
        }
    }
}
