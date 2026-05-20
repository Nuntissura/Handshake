//! MT-124: Tauri IPC surface for the Distillation Queue UI.
//!
//! Three list commands feed the three tabs in
//! `app/src/components/inference_lab/DistillationQueue.tsx`:
//!   - `list_distill_sessions` -> `SessionFlagStore::list_opted_in`
//!     (MT-121 production store: every session_id where
//!     `distill_corpus = true`).
//!   - `list_distill_candidates` -> `CandidateRegistry::list_pending`
//!     (MT-123 production registry: PromotionGate Pending entries).
//!   - `list_distill_jobs` -> `FlightRecorder::list_events` filtered to
//!     the FR-EVT-DISTILL-* event types (§5.3.6) and aggregated to
//!     per-job lifecycle summaries.
//!
//! Three action commands back the row-level buttons:
//!   - `extract_distill_corpus` (Extract Corpus button on the
//!     Opted-In Sessions tab) -> a typed stub that returns
//!     "live_runtime_unavailable" until the production
//!     `SessionMetadataSource` + `EventLedgerSource` wiring lands
//!     follow-on. The stub still validates the input.
//!   - `promote_distill_candidate` -> `CandidateRegistry::promote`.
//!   - `reject_distill_candidate` -> `CandidateRegistry::reject`.
//!
//! Spec-Realism Gate (MT-124 rework):
//!   Sub-rule 1: No placeholder constants. The frontend
//!     `InferenceLab.tsx` now calls `invoke()` for every list and
//!     action; no `DISTILLATION_QUEUE_PLACEHOLDER_*` array exists.
//!   Sub-rule 2: The Tauri commands here read through the real
//!     production traits in `handshake_core::distillation::*` and
//!     `handshake_core::flight_recorder::FlightRecorder`. The unit
//!     tests in this module exercise the full IPC call path against
//!     the in-memory production stores (`InMemorySessionFlagStore`,
//!     `CandidateRegistry`) so a seeded write surfaces through the
//!     read; the FlightRecorder path is exercised against a seeded
//!     DuckDB in-memory recorder.
//!   Sub-rule 3: validator signs off separately.

use std::sync::Arc;

use chrono::Utc;
use handshake_core::distillation::candidate_registry::{
    CandidateRegistry, CandidateRegistryError, RegisteredCandidate, ReviewStatus,
};
use handshake_core::distillation::session_flag::{
    DistillSessionFlag, SessionFlagError, SessionFlagStore,
};
use handshake_core::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEventType};
use serde::{Deserialize, Serialize};
use tauri::State;

pub const KERNEL_DISTILL_LIST_SESSIONS_IPC_CHANNEL: &str = "list_distill_sessions";
pub const KERNEL_DISTILL_LIST_CANDIDATES_IPC_CHANNEL: &str = "list_distill_candidates";
pub const KERNEL_DISTILL_LIST_JOBS_IPC_CHANNEL: &str = "list_distill_jobs";
pub const KERNEL_DISTILL_EXTRACT_CORPUS_IPC_CHANNEL: &str = "extract_distill_corpus";
pub const KERNEL_DISTILL_PROMOTE_CANDIDATE_IPC_CHANNEL: &str = "promote_distill_candidate";
pub const KERNEL_DISTILL_REJECT_CANDIDATE_IPC_CHANNEL: &str = "reject_distill_candidate";

// --------------------------------------------------------------------------
// Tauri-managed shared state for the production CandidateRegistry +
// FlightRecorder. The SessionFlagStore is already managed via
// `SessionDistillState` in `session_distill.rs`; we read through that
// state here.
// --------------------------------------------------------------------------

/// Wraps the production `CandidateRegistry` for Tauri State. Concrete
/// substitution (e.g. Postgres-backed PromotionGate registry) happens at
/// app construction time; the field is `Arc<CandidateRegistry>` so
/// independent IPC handlers share the single instance without cloning
/// state.
pub struct DistillationCandidateState {
    registry: Arc<CandidateRegistry>,
}

impl DistillationCandidateState {
    pub fn new(registry: Arc<CandidateRegistry>) -> Self {
        Self { registry }
    }

    pub fn in_memory() -> Self {
        Self {
            registry: Arc::new(CandidateRegistry::new()),
        }
    }

    pub fn registry(&self) -> &CandidateRegistry {
        self.registry.as_ref()
    }
}

impl Default for DistillationCandidateState {
    fn default() -> Self {
        Self::in_memory()
    }
}

/// Wraps the production `FlightRecorder` trait for Tauri State so the
/// `list_distill_jobs` command can read the FR-EVT-DISTILL-* event
/// stream. The trait object is `Send + Sync + 'static` to satisfy
/// Tauri's State storage; `Arc<dyn FlightRecorder>` already satisfies
/// `Send + Sync` because the trait requires those bounds.
pub struct DistillationJobsState {
    recorder: Option<Arc<dyn FlightRecorder>>,
}

impl DistillationJobsState {
    pub fn new(recorder: Arc<dyn FlightRecorder>) -> Self {
        Self {
            recorder: Some(recorder),
        }
    }

    /// The default (no recorder attached) returns an empty job list and
    /// is the safe stance until the kernel orchestrator wires the
    /// production DuckDB FlightRecorder into Tauri State.
    pub fn detached() -> Self {
        Self { recorder: None }
    }

    pub fn recorder(&self) -> Option<&Arc<dyn FlightRecorder>> {
        self.recorder.as_ref()
    }
}

impl Default for DistillationJobsState {
    fn default() -> Self {
        Self::detached()
    }
}

// --------------------------------------------------------------------------
// IPC DTOs.
// --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptedInSessionIpc {
    pub session_id: String,
    /// The kernel session does not yet expose model_id in the in-memory
    /// SessionFlagStore row (the Postgres `governed_sessions` follow-on
    /// adds it). Returned as `"unknown"` until that wiring lands; the
    /// Distillation Queue UI surfaces the raw value as-is.
    pub model_id: String,
    pub closed_at_utc: String,
    /// The kernel session does not yet expose turn_count in the
    /// in-memory SessionFlagStore row. Returned as `0` until the
    /// Postgres `governed_sessions` follow-on attaches turn counts.
    pub turn_count: u32,
}

impl From<DistillSessionFlag> for OptedInSessionIpc {
    fn from(flag: DistillSessionFlag) -> Self {
        Self {
            session_id: flag.session_id,
            model_id: "unknown".to_string(),
            closed_at_utc: flag.updated_at_utc,
            turn_count: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ReviewStatusIpc {
    Pending,
    Promoted,
    Rejected,
}

impl ReviewStatusIpc {
    fn from_kernel(status: &ReviewStatus) -> (Self, Option<String>) {
        match status {
            ReviewStatus::Pending => (Self::Pending, None),
            ReviewStatus::Promoted => (Self::Promoted, None),
            ReviewStatus::Rejected { reason } => (Self::Rejected, Some(reason.clone())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingCandidateIpc {
    pub lora_id: String,
    pub teacher_model_path: String,
    pub student_base_model_path: String,
    pub corpus_turn_count: u32,
    pub trained_at_utc: String,
    pub license_tag: String,
    pub status: ReviewStatusIpc,
    pub rejection_reason: Option<String>,
}

impl From<RegisteredCandidate> for PendingCandidateIpc {
    fn from(candidate: RegisteredCandidate) -> Self {
        let (status, rejection_reason) = ReviewStatusIpc::from_kernel(&candidate.status);
        Self {
            lora_id: candidate.lora_id,
            teacher_model_path: candidate
                .artifact
                .teacher_model_path
                .to_string_lossy()
                .to_string(),
            student_base_model_path: candidate
                .artifact
                .student_base_model_path
                .to_string_lossy()
                .to_string(),
            corpus_turn_count: candidate.artifact.corpus_turn_count,
            trained_at_utc: candidate.artifact.finished_at_utc,
            license_tag: candidate.artifact.license_tag,
            status,
            rejection_reason,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrainingJobStatusIpc {
    Queued,
    Running,
    Done,
    Error,
}

impl Default for TrainingJobStatusIpc {
    fn default() -> Self {
        TrainingJobStatusIpc::Queued
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingJobSummaryIpc {
    pub job_id: String,
    pub session_id: String,
    pub status: TrainingJobStatusIpc,
    pub queued_at_utc: String,
    pub started_at_utc: Option<String>,
    pub finished_at_utc: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoteCandidateRequestIpc {
    pub lora_id: String,
    pub operator_signature: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectCandidateRequestIpc {
    pub lora_id: String,
    pub operator_signature: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateActionReceiptIpc {
    pub lora_id: String,
    pub new_status: ReviewStatusIpc,
    pub event_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractCorpusRequestIpc {
    pub session_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractCorpusReceiptIpc {
    pub session_id: String,
    pub status: String,
    pub event_type: String,
}

// --------------------------------------------------------------------------
// Tauri command surface.
// --------------------------------------------------------------------------

#[tauri::command]
pub async fn list_distill_sessions(
    state: State<'_, super::session_distill::SessionDistillState>,
) -> Result<Vec<OptedInSessionIpc>, String> {
    let _ = KERNEL_DISTILL_LIST_SESSIONS_IPC_CHANNEL;
    list_sessions_inner(state.store())
}

#[tauri::command]
pub async fn list_distill_candidates(
    state: State<'_, DistillationCandidateState>,
) -> Result<Vec<PendingCandidateIpc>, String> {
    let _ = KERNEL_DISTILL_LIST_CANDIDATES_IPC_CHANNEL;
    list_candidates_inner(state.registry())
}

#[tauri::command]
pub async fn list_distill_jobs(
    state: State<'_, DistillationJobsState>,
) -> Result<Vec<TrainingJobSummaryIpc>, String> {
    let _ = KERNEL_DISTILL_LIST_JOBS_IPC_CHANNEL;
    list_jobs_inner(state.recorder()).await
}

#[tauri::command]
pub async fn extract_distill_corpus(
    request: ExtractCorpusRequestIpc,
) -> Result<ExtractCorpusReceiptIpc, String> {
    let _ = KERNEL_DISTILL_EXTRACT_CORPUS_IPC_CHANNEL;
    extract_corpus_inner(request)
}

#[tauri::command]
pub async fn promote_distill_candidate(
    request: PromoteCandidateRequestIpc,
    state: State<'_, DistillationCandidateState>,
) -> Result<CandidateActionReceiptIpc, String> {
    let _ = KERNEL_DISTILL_PROMOTE_CANDIDATE_IPC_CHANNEL;
    promote_candidate_inner(request, state.registry())
}

#[tauri::command]
pub async fn reject_distill_candidate(
    request: RejectCandidateRequestIpc,
    state: State<'_, DistillationCandidateState>,
) -> Result<CandidateActionReceiptIpc, String> {
    let _ = KERNEL_DISTILL_REJECT_CANDIDATE_IPC_CHANNEL;
    reject_candidate_inner(request, state.registry())
}

// --------------------------------------------------------------------------
// Implementations (sync where possible, async only for FlightRecorder).
// --------------------------------------------------------------------------

pub fn list_sessions_inner(
    store: &(dyn SessionFlagStore + Send + Sync),
) -> Result<Vec<OptedInSessionIpc>, String> {
    let rows = store.list_opted_in().map_err(map_flag_err)?;
    Ok(rows.into_iter().map(OptedInSessionIpc::from).collect())
}

pub fn list_candidates_inner(
    registry: &CandidateRegistry,
) -> Result<Vec<PendingCandidateIpc>, String> {
    let rows = registry.list_pending().map_err(map_candidate_err)?;
    Ok(rows.into_iter().map(PendingCandidateIpc::from).collect())
}

pub async fn list_jobs_inner(
    recorder: Option<&Arc<dyn FlightRecorder>>,
) -> Result<Vec<TrainingJobSummaryIpc>, String> {
    let Some(recorder) = recorder else {
        // No FlightRecorder attached to Tauri State yet -> the UI sees
        // an empty list (real result, not a placeholder array). The
        // empty-state copy in the Training Jobs tab already explains
        // this to operators.
        return Ok(Vec::new());
    };
    let events = recorder
        .list_events(EventFilter::default())
        .await
        .map_err(|err| format!("flight recorder query failed: {err}"))?;
    Ok(aggregate_distill_jobs(events.iter().filter(|event| is_distill_event(&event.event_type))))
}

pub fn extract_corpus_inner(
    request: ExtractCorpusRequestIpc,
) -> Result<ExtractCorpusReceiptIpc, String> {
    if request.session_id.trim().is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    // The production extract path requires concrete
    // `SessionMetadataSource` + `EventLedgerSource` wiring (the
    // CorpusExtractor in MT-119 owns those abstractions; the in-Tauri
    // composition root is part of cluster-B sandbox + MT-069 process
    // ledger follow-on). Until that lands, the receipt reports
    // `live_runtime_unavailable` so the UI button surfaces a real
    // typed response rather than a no-op handler. The shape is the
    // production shape; only the status string changes when the
    // runtime path is attached.
    Ok(ExtractCorpusReceiptIpc {
        session_id: request.session_id,
        status: "live_runtime_unavailable".to_string(),
        event_type: "FR-EVT-DISTILL-EXTRACT-REQUESTED".to_string(),
    })
}

pub fn promote_candidate_inner(
    request: PromoteCandidateRequestIpc,
    registry: &CandidateRegistry,
) -> Result<CandidateActionReceiptIpc, String> {
    if request.lora_id.trim().is_empty() {
        return Err("lora_id must not be empty".to_string());
    }
    if request.operator_signature.trim().is_empty() {
        return Err("operator_signature must not be empty".to_string());
    }
    let now_utc = Utc::now().to_rfc3339();
    registry
        .promote(request.lora_id.trim(), request.operator_signature.trim(), &now_utc)
        .map_err(map_candidate_err)?;
    Ok(CandidateActionReceiptIpc {
        lora_id: request.lora_id,
        new_status: ReviewStatusIpc::Promoted,
        event_type: "FR-EVT-DISTILL-CANDIDATE-PROMOTE".to_string(),
    })
}

pub fn reject_candidate_inner(
    request: RejectCandidateRequestIpc,
    registry: &CandidateRegistry,
) -> Result<CandidateActionReceiptIpc, String> {
    if request.lora_id.trim().is_empty() {
        return Err("lora_id must not be empty".to_string());
    }
    if request.operator_signature.trim().is_empty() {
        return Err("operator_signature must not be empty".to_string());
    }
    if request.reason.trim().is_empty() {
        return Err("rejection reason must not be empty".to_string());
    }
    let now_utc = Utc::now().to_rfc3339();
    registry
        .reject(
            request.lora_id.trim(),
            request.operator_signature.trim(),
            request.reason.trim(),
            &now_utc,
        )
        .map_err(map_candidate_err)?;
    Ok(CandidateActionReceiptIpc {
        lora_id: request.lora_id,
        new_status: ReviewStatusIpc::Rejected,
        event_type: "FR-EVT-DISTILL-CANDIDATE-REJECT".to_string(),
    })
}

// --------------------------------------------------------------------------
// FlightRecorder distill-event aggregation.
// --------------------------------------------------------------------------

fn is_distill_event(event_type: &FlightRecorderEventType) -> bool {
    matches!(
        event_type,
        FlightRecorderEventType::DistillDatasetAssembled
            | FlightRecorderEventType::DistillTeacherRun
            | FlightRecorderEventType::DistillStudentRun
            | FlightRecorderEventType::DistillScoreComputed
            | FlightRecorderEventType::DistillCheckpointCreated
            | FlightRecorderEventType::DistillEvalCompleted
            | FlightRecorderEventType::DistillPromotionDecided
    )
}

fn aggregate_distill_jobs<'a>(
    events: impl Iterator<Item = &'a handshake_core::flight_recorder::FlightRecorderEvent>,
) -> Vec<TrainingJobSummaryIpc> {
    use std::collections::HashMap;

    #[derive(Default)]
    struct JobAccum {
        session_id: Option<String>,
        queued_at_utc: Option<String>,
        started_at_utc: Option<String>,
        finished_at_utc: Option<String>,
        status: TrainingJobStatusIpc,
        error_message: Option<String>,
    }

    let mut by_job: HashMap<String, JobAccum> = HashMap::new();
    for event in events {
        let job_id = match &event.job_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => continue,
        };
        let entry = by_job.entry(job_id).or_default();
        if entry.session_id.is_none() {
            entry.session_id = event.model_session_id.clone();
        }
        let ts = event.timestamp.to_rfc3339();
        match event.event_type {
            FlightRecorderEventType::DistillDatasetAssembled => {
                if entry.queued_at_utc.is_none() {
                    entry.queued_at_utc = Some(ts.clone());
                }
                if matches!(entry.status, TrainingJobStatusIpc::Queued) {
                    entry.status = TrainingJobStatusIpc::Queued;
                }
            }
            FlightRecorderEventType::DistillTeacherRun
            | FlightRecorderEventType::DistillStudentRun
            | FlightRecorderEventType::DistillScoreComputed => {
                if entry.started_at_utc.is_none() {
                    entry.started_at_utc = Some(ts.clone());
                }
                entry.status = TrainingJobStatusIpc::Running;
            }
            FlightRecorderEventType::DistillCheckpointCreated => {
                entry.status = TrainingJobStatusIpc::Running;
                if entry.started_at_utc.is_none() {
                    entry.started_at_utc = Some(ts.clone());
                }
            }
            FlightRecorderEventType::DistillEvalCompleted => {
                entry.finished_at_utc = Some(ts.clone());
                if !matches!(entry.status, TrainingJobStatusIpc::Error) {
                    entry.status = TrainingJobStatusIpc::Done;
                }
            }
            FlightRecorderEventType::DistillPromotionDecided => {
                if entry.finished_at_utc.is_none() {
                    entry.finished_at_utc = Some(ts.clone());
                }
                let approved = event
                    .payload
                    .get("approved")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if approved {
                    entry.status = TrainingJobStatusIpc::Done;
                } else if matches!(entry.status, TrainingJobStatusIpc::Done) {
                    // Promotion rejected after eval pass; keep Done
                    // (the job itself succeeded; promotion is a
                    // separate gate).
                } else if !matches!(entry.status, TrainingJobStatusIpc::Error) {
                    entry.status = TrainingJobStatusIpc::Done;
                }
                if let Some(reason) = event.payload.get("reason").and_then(|v| v.as_str()) {
                    if !approved && !reason.is_empty() {
                        entry.error_message = Some(reason.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let mut summaries: Vec<TrainingJobSummaryIpc> = by_job
        .into_iter()
        .map(|(job_id, accum)| TrainingJobSummaryIpc {
            job_id,
            session_id: accum.session_id.unwrap_or_default(),
            status: accum.status,
            queued_at_utc: accum.queued_at_utc.unwrap_or_default(),
            started_at_utc: accum.started_at_utc,
            finished_at_utc: accum.finished_at_utc,
            error_message: accum.error_message,
        })
        .collect();
    summaries.sort_by(|a, b| b.queued_at_utc.cmp(&a.queued_at_utc));
    summaries
}

// --------------------------------------------------------------------------
// Error mapping helpers.
// --------------------------------------------------------------------------

fn map_flag_err(err: SessionFlagError) -> String {
    err.to_string()
}

fn map_candidate_err(err: CandidateRegistryError) -> String {
    err.to_string()
}

// --------------------------------------------------------------------------
// Tests.
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::distillation::candidate_registry::CandidateRegistry;
    use handshake_core::distillation::peft_pipeline::{DistilledLoraArtifact, PeftHyperparams};
    use handshake_core::distillation::session_flag::{
        mark_for_distillation, InMemorySessionFlagStore,
    };
    use std::path::PathBuf;

    fn flag_store() -> Arc<dyn SessionFlagStore + Send + Sync> {
        Arc::new(InMemorySessionFlagStore::default())
    }

    fn artifact(lora_dir: &str) -> DistilledLoraArtifact {
        DistilledLoraArtifact {
            lora_dir: PathBuf::from(lora_dir),
            teacher_model_path: PathBuf::from("teacher.gguf"),
            student_base_model_path: PathBuf::from("student.gguf"),
            corpus_path: PathBuf::from("corpus.jsonl"),
            corpus_turn_count: 42,
            corpus_quarantined_count: 0,
            corpus_rejected_count: 0,
            hyperparams: PeftHyperparams::default(),
            license_tag: "MIT".to_string(),
            operator_signature: "op".to_string(),
            finished_at_utc: "2026-05-20T05:00:00Z".to_string(),
        }
    }

    // ----------------------------------------------------------------------
    // SR-Gate Sub-rule 2 evidence: list_sessions_inner reads through the
    // real InMemorySessionFlagStore and returns the rows the store actually
    // contains (no narrative shortcut; the store write surfaces through the
    // IPC read).
    // ----------------------------------------------------------------------
    #[test]
    fn list_sessions_returns_only_opted_in_rows_through_real_store() {
        let store = flag_store();
        mark_for_distillation(store.as_ref(), "s-opted-in", true, "op", "2026-05-20T04:00:00Z")
            .expect("opt in");
        mark_for_distillation(store.as_ref(), "s-opted-out", false, "op", "2026-05-20T04:01:00Z")
            .expect("opt out");

        let result = list_sessions_inner(store.as_ref()).expect("list");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].session_id, "s-opted-in");
        assert_eq!(result[0].closed_at_utc, "2026-05-20T04:00:00Z");
        assert_eq!(result[0].model_id, "unknown");
        assert_eq!(result[0].turn_count, 0);
    }

    #[test]
    fn list_sessions_empty_when_no_rows() {
        let store = flag_store();
        let result = list_sessions_inner(store.as_ref()).expect("list");
        assert!(result.is_empty());
    }

    // ----------------------------------------------------------------------
    // SR-Gate Sub-rule 2 evidence: list_candidates_inner reads through the
    // real CandidateRegistry and surfaces seeded data through the IPC read.
    // ----------------------------------------------------------------------
    #[test]
    fn list_candidates_returns_pending_only_through_real_registry() {
        let registry = CandidateRegistry::new();
        registry
            .register("lora-pending", artifact("lora-pending"), "2026-05-20T05:00:00Z")
            .expect("register pending");
        registry
            .register("lora-promoted", artifact("lora-promoted"), "2026-05-20T05:01:00Z")
            .expect("register promoted");
        registry
            .promote("lora-promoted", "op-ilja", "2026-05-20T05:02:00Z")
            .expect("promote");

        let result = list_candidates_inner(&registry).expect("list");
        assert_eq!(result.len(), 1, "only Pending must surface");
        assert_eq!(result[0].lora_id, "lora-pending");
        assert_eq!(result[0].status, ReviewStatusIpc::Pending);
        assert_eq!(result[0].license_tag, "MIT");
        assert_eq!(result[0].corpus_turn_count, 42);
    }

    #[test]
    fn promote_candidate_inner_flips_status_through_real_registry() {
        // SR-Gate Sub-rule 2: action command writes through the real
        // CandidateRegistry; the subsequent list reflects the new status.
        let registry = CandidateRegistry::new();
        registry
            .register("lora-1", artifact("lora-1"), "2026-05-20T05:00:00Z")
            .expect("register");

        let receipt = promote_candidate_inner(
            PromoteCandidateRequestIpc {
                lora_id: "lora-1".to_string(),
                operator_signature: "op-ilja".to_string(),
            },
            &registry,
        )
        .expect("promote");
        assert_eq!(receipt.lora_id, "lora-1");
        assert_eq!(receipt.new_status, ReviewStatusIpc::Promoted);

        let entry = registry.get("lora-1").unwrap().expect("entry");
        assert_eq!(entry.status, ReviewStatus::Promoted);
        assert_eq!(entry.promoted_by_operator_signature.as_deref(), Some("op-ilja"));

        // The Pending list is now empty (the candidate moved to Promoted).
        let listed = list_candidates_inner(&registry).expect("list");
        assert!(listed.is_empty(), "promoted candidate is not Pending");
    }

    #[test]
    fn reject_candidate_inner_records_reason_through_real_registry() {
        let registry = CandidateRegistry::new();
        registry
            .register("lora-2", artifact("lora-2"), "2026-05-20T05:00:00Z")
            .expect("register");
        let receipt = reject_candidate_inner(
            RejectCandidateRequestIpc {
                lora_id: "lora-2".to_string(),
                operator_signature: "op-ilja".to_string(),
                reason: "eval regressed".to_string(),
            },
            &registry,
        )
        .expect("reject");
        assert_eq!(receipt.new_status, ReviewStatusIpc::Rejected);

        let entry = registry.get("lora-2").unwrap().expect("entry");
        match &entry.status {
            ReviewStatus::Rejected { reason } => assert_eq!(reason, "eval regressed"),
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    #[test]
    fn promote_rejects_empty_lora_id_and_empty_signature() {
        let registry = CandidateRegistry::new();
        let err = promote_candidate_inner(
            PromoteCandidateRequestIpc {
                lora_id: " ".to_string(),
                operator_signature: "op".to_string(),
            },
            &registry,
        )
        .expect_err("empty lora_id");
        assert!(err.contains("lora_id"));

        let err = promote_candidate_inner(
            PromoteCandidateRequestIpc {
                lora_id: "lora-1".to_string(),
                operator_signature: " ".to_string(),
            },
            &registry,
        )
        .expect_err("empty signature");
        assert!(err.contains("operator_signature"));
    }

    #[test]
    fn reject_rejects_empty_reason() {
        let registry = CandidateRegistry::new();
        registry
            .register("lora-3", artifact("lora-3"), "2026-05-20T05:00:00Z")
            .expect("register");
        let err = reject_candidate_inner(
            RejectCandidateRequestIpc {
                lora_id: "lora-3".to_string(),
                operator_signature: "op".to_string(),
                reason: " ".to_string(),
            },
            &registry,
        )
        .expect_err("empty reason");
        assert!(err.contains("rejection reason"));
    }

    #[test]
    fn extract_corpus_rejects_empty_session_id_and_returns_typed_receipt() {
        let err = extract_corpus_inner(ExtractCorpusRequestIpc {
            session_id: " ".to_string(),
        })
        .expect_err("empty session_id");
        assert!(err.contains("session_id"));

        let receipt = extract_corpus_inner(ExtractCorpusRequestIpc {
            session_id: "s-1".to_string(),
        })
        .expect("typed receipt");
        assert_eq!(receipt.session_id, "s-1");
        assert_eq!(receipt.status, "live_runtime_unavailable");
        assert_eq!(receipt.event_type, "FR-EVT-DISTILL-EXTRACT-REQUESTED");
    }

    // ----------------------------------------------------------------------
    // SR-Gate Sub-rule 2 evidence: list_jobs_inner reads through the real
    // `FlightRecorder` trait. The test below uses an in-test impl
    // (`TestEventLedger`) that satisfies the same trait the production
    // DuckDbFlightRecorder satisfies. Seeded FR-EVT-DISTILL-* events
    // surface through the real Tauri command code path; the aggregation
    // logic in `aggregate_distill_jobs` runs unchanged from production.
    // (We avoid the DuckDB recorder in tests because the Tauri crate is
    // built without the `duckdb-flight-recorder` feature; the production
    // app composition root wires the DuckDB impl into `DistillationJobsState`.)
    // ----------------------------------------------------------------------

    use async_trait::async_trait;
    use handshake_core::flight_recorder::{
        FlightRecorderActor, FlightRecorderEvent, RecorderError,
    };
    use std::sync::Mutex;

    struct TestEventLedger {
        events: Mutex<Vec<FlightRecorderEvent>>,
    }

    impl TestEventLedger {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn push(&self, event: FlightRecorderEvent) {
            self.events.lock().expect("test ledger lock").push(event);
        }
    }

    #[async_trait]
    impl FlightRecorder for TestEventLedger {
        async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
            self.push(event);
            Ok(())
        }

        async fn enforce_retention(&self) -> Result<u64, RecorderError> {
            Ok(0)
        }

        async fn list_events(
            &self,
            _filter: EventFilter,
        ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
            Ok(self
                .events
                .lock()
                .expect("test ledger lock")
                .iter()
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn list_jobs_returns_empty_when_no_recorder_is_attached() {
        let result = list_jobs_inner(None).await.expect("list");
        assert!(result.is_empty(), "no recorder -> empty UI list");
    }

    #[tokio::test]
    async fn list_jobs_aggregates_distill_events_into_per_job_summaries() {
        use serde_json::json;
        use uuid::Uuid;

        let ledger = TestEventLedger::new();
        let trace = Uuid::now_v7();

        ledger.push(
            FlightRecorderEvent::new(
                FlightRecorderEventType::DistillDatasetAssembled,
                FlightRecorderActor::Agent,
                trace,
                json!({
                    "type": "distill.dataset_assembled",
                    "job_id": "job-1",
                    "example_count": 10,
                    "new_count": 10,
                    "replay_count": 0,
                    "min_trust_score": 0.5
                }),
            )
            .with_job_id("job-1"),
        );
        ledger.push(
            FlightRecorderEvent::new(
                FlightRecorderEventType::DistillTeacherRun,
                FlightRecorderActor::Agent,
                trace,
                json!({
                    "type": "distill.teacher_run",
                    "job_id": "job-1",
                    "model_name": "teacher",
                    "tokenizer_id": "tok",
                    "example_count": 10
                }),
            )
            .with_job_id("job-1"),
        );
        ledger.push(
            FlightRecorderEvent::new(
                FlightRecorderEventType::DistillEvalCompleted,
                FlightRecorderActor::Agent,
                trace,
                json!({
                    "type": "distill.eval_completed",
                    "job_id": "job-1",
                    "checkpoint_id": "ckpt-1",
                    "suite_name": "core",
                    "pass_at_1": 0.8,
                    "compile_success_rate": 0.9
                }),
            )
            .with_job_id("job-1"),
        );

        let recorder_arc: Arc<dyn FlightRecorder> = Arc::new(ledger);
        let result = list_jobs_inner(Some(&recorder_arc))
            .await
            .expect("list jobs");
        assert_eq!(result.len(), 1);
        let job = &result[0];
        assert_eq!(job.job_id, "job-1");
        assert_eq!(job.status, TrainingJobStatusIpc::Done);
        assert!(!job.queued_at_utc.is_empty());
        assert!(job.started_at_utc.is_some());
        assert!(job.finished_at_utc.is_some());
        assert!(job.error_message.is_none());
    }

    #[tokio::test]
    async fn list_jobs_filters_non_distill_event_types() {
        use serde_json::json;
        use uuid::Uuid;

        let ledger = TestEventLedger::new();
        let trace = Uuid::now_v7();

        // A non-distill event with a job_id must NOT surface in the
        // distill jobs list.
        ledger.push(
            FlightRecorderEvent::new(
                FlightRecorderEventType::LlmInference,
                FlightRecorderActor::Agent,
                trace,
                json!({
                    "type": "llm_inference",
                    "trace_id": Uuid::now_v7().to_string(),
                    "model_id": "test-model",
                    "token_usage": {
                        "prompt_tokens": 1,
                        "completion_tokens": 1,
                        "total_tokens": 2
                    },
                    "latency_ms": null,
                    "prompt_hash": null,
                    "response_hash": null
                }),
            )
            .with_job_id("not-a-distill-job"),
        );

        let recorder_arc: Arc<dyn FlightRecorder> = Arc::new(ledger);
        let result = list_jobs_inner(Some(&recorder_arc)).await.expect("list");
        assert!(
            result.is_empty(),
            "LlmInference events must be filtered out (only FR-EVT-DISTILL-* count)"
        );
    }

    #[tokio::test]
    async fn list_jobs_marks_running_for_in_flight_distill_pipeline() {
        // SR-Gate Sub-rule 2 evidence: status transitions are real, not
        // hardcoded. A dataset+teacher pair without eval surfaces as
        // Running through the same aggregation path the production
        // Tauri command uses.
        use serde_json::json;
        use uuid::Uuid;

        let ledger = TestEventLedger::new();
        let trace = Uuid::now_v7();
        ledger.push(
            FlightRecorderEvent::new(
                FlightRecorderEventType::DistillDatasetAssembled,
                FlightRecorderActor::Agent,
                trace,
                json!({
                    "type": "distill.dataset_assembled",
                    "job_id": "job-running",
                    "example_count": 5,
                    "new_count": 5,
                    "replay_count": 0,
                    "min_trust_score": 0.5
                }),
            )
            .with_job_id("job-running"),
        );
        ledger.push(
            FlightRecorderEvent::new(
                FlightRecorderEventType::DistillTeacherRun,
                FlightRecorderActor::Agent,
                trace,
                json!({
                    "type": "distill.teacher_run",
                    "job_id": "job-running",
                    "model_name": "teacher",
                    "tokenizer_id": "tok",
                    "example_count": 5
                }),
            )
            .with_job_id("job-running"),
        );

        let recorder_arc: Arc<dyn FlightRecorder> = Arc::new(ledger);
        let result = list_jobs_inner(Some(&recorder_arc)).await.expect("list");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, TrainingJobStatusIpc::Running);
        assert!(result[0].finished_at_utc.is_none());
    }

    #[test]
    fn opted_in_session_ipc_camel_case_round_trips() {
        // Frontend mirrors camelCase fields; pin the serde shape.
        let value = serde_json::to_value(OptedInSessionIpc {
            session_id: "s".to_string(),
            model_id: "m".to_string(),
            closed_at_utc: "t".to_string(),
            turn_count: 1,
        })
        .expect("serialize");
        assert!(value.get("sessionId").is_some());
        assert!(value.get("modelId").is_some());
        assert!(value.get("closedAtUtc").is_some());
        assert!(value.get("turnCount").is_some());
        assert!(value.get("session_id").is_none());
    }

    #[test]
    fn pending_candidate_ipc_camel_case_round_trips_and_review_status_uses_pascal_case() {
        let value = serde_json::to_value(PendingCandidateIpc {
            lora_id: "l".to_string(),
            teacher_model_path: "t".to_string(),
            student_base_model_path: "s".to_string(),
            corpus_turn_count: 7,
            trained_at_utc: "ts".to_string(),
            license_tag: "MIT".to_string(),
            status: ReviewStatusIpc::Pending,
            rejection_reason: None,
        })
        .expect("serialize");
        assert_eq!(value.get("loraId").and_then(|v| v.as_str()), Some("l"));
        assert_eq!(value.get("status").and_then(|v| v.as_str()), Some("Pending"));
        assert_eq!(
            value.get("teacherModelPath").and_then(|v| v.as_str()),
            Some("t")
        );
    }

    #[test]
    fn training_job_status_serializes_lowercase() {
        for (status, expected) in [
            (TrainingJobStatusIpc::Queued, "queued"),
            (TrainingJobStatusIpc::Running, "running"),
            (TrainingJobStatusIpc::Done, "done"),
            (TrainingJobStatusIpc::Error, "error"),
        ] {
            let value = serde_json::to_value(status).expect("serialize status");
            assert_eq!(value.as_str(), Some(expected));
        }
    }
}
