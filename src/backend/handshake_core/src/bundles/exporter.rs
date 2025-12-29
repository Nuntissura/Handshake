use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::bundles::redactor::SecretRedactor;
use crate::bundles::schemas::{
    AvailableCounts, BundleDiagnostic, BundleEnv, BundleJob, BundleJobError, BundleJobMetrics,
    BundleManifest, BundleManifestFile, EvidenceGap, ExpiredEvidence, ExportableDiagnostic,
    ExportableFilter, ExportableInventory, ExportableJob, ExportableRange, IncludedCounts,
    ManifestScope, MissingEvidence, PlatformInfo, PlatformInfoMinimal, RedactionLogEntry,
    RedactionMode, RedactionReport, RetentionPolicy, RetentionReport, ScopeKind, TimeRange,
};
use crate::bundles::templates::{render_coder_prompt, render_repro_md};
use crate::bundles::validator::ValBundleValidator;
use crate::bundles::zip::{
    compute_bundle_hash, sha256_hex, write_deterministic_zip, BundleFileEntry,
};
use crate::diagnostics::{DiagFilter, Diagnostic};
use crate::flight_recorder::{
    EventFilter, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    FrEvt005DebugBundleExport,
};
use crate::storage::{AiJob, AiJobListFilter, JobState, StorageError};
use crate::AppState;

static BUNDLE_STORE: Lazy<Mutex<HashMap<String, PathBuf>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn bundle_path(bundle_id: &str) -> Option<PathBuf> {
    BUNDLE_STORE
        .lock()
        .ok()
        .and_then(|store| store.get(bundle_id).cloned())
}

fn hash_value(value: &Value) -> String {
    let serialized = serde_json::to_vec(value).unwrap_or_default();
    sha256_hex(&serialized)
}

fn hash_optional_value(value: &Option<Value>) -> String {
    match value {
        Some(val) => hash_value(val),
        None => sha256_hex(&[]),
    }
}

fn preview_json(value: &Option<Value>, max_len: usize) -> Option<String> {
    let serialized = value
        .as_ref()
        .and_then(|v| serde_json::to_string(v).ok())
        .unwrap_or_default();
    if serialized.is_empty() {
        None
    } else {
        Some(serialized.chars().take(max_len).collect::<String>())
    }
}

fn job_status_from_state(state: &JobState) -> String {
    match state {
        JobState::Queued => "queued",
        JobState::Running | JobState::AwaitingUser | JobState::AwaitingValidation => "running",
        JobState::Completed | JobState::CompletedWithIssues => "completed",
        JobState::Failed | JobState::Poisoned => "failed",
        JobState::Cancelled => "cancelled",
        JobState::Stalled => "running",
    }
    .to_string()
}

fn job_matches_wsid(job: &AiJob, wsid: &str) -> bool {
    let entity_match = job
        .entity_refs
        .iter()
        .any(|r| r.entity_kind == "workspace" && r.entity_id == wsid);
    let input_match = job
        .job_inputs
        .as_ref()
        .and_then(|v| v.get("wsid"))
        .and_then(|v| v.as_str())
        .map(|v| v == wsid)
        .unwrap_or(false);
    entity_match || input_match
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugBundleRequest {
    pub scope: BundleScope,
    pub redaction_mode: RedactionMode,
    pub output_path: Option<PathBuf>,
    pub include_artifacts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BundleScope {
    Problem {
        diagnostic_id: String,
    },
    Job {
        job_id: String,
    },
    TimeWindow {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        wsid: Option<String>,
    },
    Workspace {
        wsid: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableFilterInput {
    pub wsid: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
}

#[derive(Debug, Error)]
pub enum BundleExportError {
    #[error("HSK-400-INVALID-SCOPE: Invalid export scope: {0}")]
    InvalidScope(String),
    #[error("HSK-403-CAPABILITY: Missing capability: {0}")]
    CapabilityDenied(String),
    #[error("HSK-404-NOT-FOUND: {kind} not found: {id}")]
    NotFound { kind: String, id: String },
    #[error("HSK-409-POLICY: Export blocked by policy: {0}")]
    PolicyDenied(String),
    #[error("HSK-500-EXPORT: Export failed: {0}")]
    ExportFailed(String),
    #[error("HSK-500-IO: IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("HSK-500-VALIDATION: Validation error: {0}")]
    Validation(String),
    #[error("HSK-500-ZIP: {0}")]
    Zip(String),
}

impl From<zip::result::ZipError> for BundleExportError {
    fn from(err: zip::result::ZipError) -> Self {
        BundleExportError::Zip(err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFinding {
    pub severity: FindingSeverity,
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingSeverity {
    Error,
    Warning,
    Info,
}

#[async_trait]
pub trait DebugBundleExporter: Send + Sync {
    async fn export(
        &self,
        request: DebugBundleRequest,
    ) -> Result<BundleManifest, BundleExportError>;

    async fn validate(
        &self,
        bundle_path: &Path,
    ) -> Result<crate::bundles::validator::BundleValidationReport, BundleExportError>;

    async fn list_exportable(
        &self,
        filter: ExportableFilter,
    ) -> Result<ExportableInventory, BundleExportError>;
}

#[derive(Clone)]
pub struct DefaultDebugBundleExporter {
    state: AppState,
}

impl DefaultDebugBundleExporter {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    fn default_output_path(bundle_id: &str) -> PathBuf {
        PathBuf::from("data")
            .join("bundles")
            .join(format!("bundle-{}", bundle_id))
    }

    fn store_bundle_location(&self, bundle_id: &str, path: PathBuf) {
        if let Ok(mut guard) = BUNDLE_STORE.lock() {
            guard.insert(bundle_id.to_string(), path);
        }
    }

    fn build_manifest_scope(&self, scope: &BundleScope) -> ManifestScope {
        match scope {
            BundleScope::Problem { diagnostic_id } => ManifestScope {
                kind: ScopeKind::Problem,
                problem_id: Some(diagnostic_id.clone()),
                job_id: None,
                time_range: None,
                wsid: None,
            },
            BundleScope::Job { job_id } => ManifestScope {
                kind: ScopeKind::Job,
                problem_id: None,
                job_id: Some(job_id.clone()),
                time_range: None,
                wsid: None,
            },
            BundleScope::TimeWindow { start, end, wsid } => ManifestScope {
                kind: ScopeKind::TimeWindow,
                problem_id: None,
                job_id: None,
                time_range: Some(TimeRange {
                    start: *start,
                    end: *end,
                }),
                wsid: wsid.clone(),
            },
            BundleScope::Workspace { wsid } => ManifestScope {
                kind: ScopeKind::Workspace,
                problem_id: None,
                job_id: None,
                time_range: None,
                wsid: Some(wsid.clone()),
            },
        }
    }

    fn build_env(&self, wsid: Option<String>, redaction_mode: RedactionMode) -> BundleEnv {
        BundleEnv {
            app_version: "0.1.0".to_string(),
            build_hash: "unknown".to_string(),
            platform: PlatformInfoMinimal {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
            },
            rust_version: "unknown".to_string(),
            node_version: None,
            wsid,
            workspace_name: None,
            config: crate::bundles::schemas::EnvConfig {
                model_runtime: "unknown".to_string(),
                default_model: None,
                flight_recorder_retention_days: 7,
            },
            feature_flags: Vec::new(),
            redaction_note: match redaction_mode {
                RedactionMode::SafeDefault => {
                    "Paths, env vars, and secrets removed per SAFE_DEFAULT policy".to_string()
                }
                RedactionMode::Workspace => {
                    "Workspace context included, secrets removed".to_string()
                }
                RedactionMode::FullLocal => "Full payloads included (policy override)".to_string(),
            },
        }
    }

    async fn collect_diagnostics(
        &self,
        scope: &BundleScope,
    ) -> Result<(Vec<Diagnostic>, Vec<MissingEvidence>), BundleExportError> {
        let mut missing = Vec::new();
        let diagnostics = match scope {
            BundleScope::Problem { diagnostic_id } => {
                let parsed = Uuid::parse_str(diagnostic_id).map_err(|_| {
                    BundleExportError::InvalidScope("diagnostic_id must be a UUID".to_string())
                })?;
                match self.state.diagnostics.get_diagnostic(parsed).await {
                    Ok(diag) => vec![diag],
                    Err(StorageError::NotFound(_)) => {
                        missing.push(MissingEvidence {
                            kind: "diagnostic".to_string(),
                            id: diagnostic_id.clone(),
                            reason: "not_found".to_string(),
                        });
                        Vec::new()
                    }
                    Err(err) => {
                        return Err(BundleExportError::ExportFailed(format!(
                            "diagnostic fetch failed: {}",
                            err
                        )))
                    }
                }
            }
            BundleScope::Job { job_id } => {
                let parsed = Uuid::parse_str(job_id).map_err(|_| {
                    BundleExportError::InvalidScope("job_id must be a UUID".to_string())
                })?;
                let mut filter = DiagFilter::default();
                filter.job_id = Some(parsed);
                filter.limit = Some(1000);
                self.state
                    .diagnostics
                    .list_diagnostics(filter)
                    .await
                    .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?
            }
            BundleScope::TimeWindow { start, end, wsid } => {
                let mut filter = DiagFilter::default();
                filter.from = Some(*start);
                filter.to = Some(*end);
                filter.wsid = wsid.clone();
                filter.limit = Some(1000);
                self.state
                    .diagnostics
                    .list_diagnostics(filter)
                    .await
                    .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?
            }
            BundleScope::Workspace { wsid } => {
                let mut filter = DiagFilter::default();
                filter.from = Some(Utc::now() - Duration::hours(24));
                filter.to = Some(Utc::now());
                filter.wsid = Some(wsid.clone());
                filter.limit = Some(1000);
                self.state
                    .diagnostics
                    .list_diagnostics(filter)
                    .await
                    .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?
            }
        };

        Ok((diagnostics, missing))
    }

    async fn collect_events(
        &self,
        scope: &BundleScope,
        diagnostics: &[Diagnostic],
    ) -> Result<Vec<FlightRecorderEvent>, BundleExportError> {
        let mut filter = EventFilter::default();
        match scope {
            BundleScope::Job { job_id } => {
                filter.job_id = Some(job_id.clone());
            }
            BundleScope::Problem { .. } => {
                if let Some(diag) = diagnostics.first() {
                    if let Some(job_id) = diag.job_id.clone() {
                        filter.job_id = Some(job_id);
                    } else {
                        filter.from = Some(diag.timestamp - Duration::hours(1));
                        filter.to = Some(diag.timestamp + Duration::hours(1));
                    }
                }
            }
            BundleScope::TimeWindow {
                start,
                end,
                wsid: _,
            } => {
                filter.from = Some(*start);
                filter.to = Some(*end);
            }
            BundleScope::Workspace { .. } => {
                filter.from = Some(Utc::now() - Duration::hours(24));
                filter.to = Some(Utc::now());
            }
        }

        let events = self
            .state
            .flight_recorder
            .list_events(filter)
            .await
            .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?;

        let filtered = match scope {
            BundleScope::Workspace { wsid } => events
                .into_iter()
                .filter(|evt| evt.wsids.iter().any(|w| w == wsid))
                .collect(),
            BundleScope::TimeWindow {
                wsid: Some(wsid), ..
            } => events
                .into_iter()
                .filter(|evt| evt.wsids.iter().any(|w| w == wsid))
                .collect(),
            _ => events,
        };

        Ok(filtered)
    }

    async fn collect_jobs(
        &self,
        scope: &BundleScope,
        diagnostics: &[Diagnostic],
        events: &[FlightRecorderEvent],
    ) -> Result<(Vec<AiJob>, Vec<MissingEvidence>), BundleExportError> {
        let mut missing = Vec::new();
        let mut job_ids: HashSet<String> = HashSet::new();

        if let BundleScope::Job { job_id } = scope {
            job_ids.insert(job_id.clone());
        }
        for diag in diagnostics {
            if let Some(id) = diag.job_id.clone() {
                job_ids.insert(id);
            }
        }
        for evt in events {
            if let Some(id) = evt.job_id.clone() {
                job_ids.insert(id);
            }
        }

        let mut jobs: Vec<AiJob> = Vec::new();

        for job_id in job_ids {
            match self.state.storage.get_ai_job(&job_id).await {
                Ok(job) => jobs.push(job),
                Err(StorageError::NotFound(_)) => missing.push(MissingEvidence {
                    kind: "job".to_string(),
                    id: job_id,
                    reason: "not_found".to_string(),
                }),
                Err(err) => {
                    return Err(BundleExportError::ExportFailed(format!(
                        "job fetch failed: {}",
                        err
                    )))
                }
            }
        }

        if let BundleScope::TimeWindow { start, end, wsid } = scope {
            let filter = AiJobListFilter {
                from: Some(*start),
                to: Some(*end),
                ..Default::default()
            };
            let list = self
                .state
                .storage
                .list_ai_jobs(filter)
                .await
                .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?;
            for job in list {
                if let Some(wsid_val) = wsid {
                    if !job_matches_wsid(&job, wsid_val) {
                        continue;
                    }
                }
                if jobs.iter().any(|j| j.job_id == job.job_id) {
                    continue;
                }
                jobs.push(job);
            }
        }

        if let BundleScope::Workspace { wsid } = scope {
            let filter = AiJobListFilter {
                from: Some(Utc::now() - Duration::hours(24)),
                to: Some(Utc::now()),
                ..Default::default()
            };
            if let Ok(list) = self.state.storage.list_ai_jobs(filter).await {
                for job in list {
                    if !job_matches_wsid(&job, wsid) {
                        continue;
                    }
                    if jobs.iter().any(|j| j.job_id == job.job_id) {
                        continue;
                    }
                    jobs.push(job);
                }
            }
        }

        jobs.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok((jobs, missing))
    }

    fn map_diagnostic(&self, diagnostic: &Diagnostic) -> BundleDiagnostic {
        let file_path = diagnostic
            .locations
            .as_ref()
            .and_then(|locs| locs.first())
            .and_then(|loc| loc.path.clone());

        let (line_start, line_end) = diagnostic
            .locations
            .as_ref()
            .and_then(|locs| locs.first())
            .and_then(|loc| loc.range.as_ref())
            .map(|range| (Some(range.start_line as u32), Some(range.end_line as u32)))
            .unwrap_or((None, None));

        let mut evidence_refs: Vec<String> = Vec::new();
        if let Some(refs) = diagnostic.evidence_refs.as_ref() {
            if let Some(ids) = refs.fr_event_ids.as_ref() {
                evidence_refs.extend(ids.clone());
            }
            if let Some(ids) = refs.related_job_ids.as_ref() {
                evidence_refs.extend(ids.clone());
            }
            if let Some(ids) = refs.related_activity_span_ids.as_ref() {
                evidence_refs.extend(ids.clone());
            }
            if let Some(ids) = refs.related_session_span_ids.as_ref() {
                evidence_refs.extend(ids.clone());
            }
        }

        BundleDiagnostic {
            id: diagnostic.id.to_string(),
            fingerprint: diagnostic.fingerprint.clone(),
            severity: diagnostic.severity.as_str().to_string(),
            source: diagnostic.source.as_str(),
            surface: diagnostic.surface.as_str().to_string(),
            code: diagnostic.code.clone().unwrap_or_else(|| "n/a".to_string()),
            title: diagnostic.title.clone(),
            message: diagnostic.message.clone(),
            created_at: diagnostic.timestamp,
            wsid: diagnostic.wsid.clone(),
            job_id: diagnostic.job_id.clone(),
            workflow_run_id: None,
            file_path,
            line_start,
            line_end,
            link_confidence: diagnostic.link_confidence.as_str().to_string(),
            evidence_refs,
            occurrence_count: diagnostic.count.map(|c| c as u32),
            first_seen: diagnostic.first_seen,
            last_seen: diagnostic.last_seen,
        }
    }

    fn map_job(
        &self,
        job: &AiJob,
        diagnostics: &[BundleDiagnostic],
        events: &[FlightRecorderEvent],
        redaction_mode: RedactionMode,
    ) -> BundleJob {
        let wsid = job
            .entity_refs
            .iter()
            .find(|r| r.entity_kind == "workspace")
            .map(|r| r.entity_id.clone())
            .or_else(|| {
                job.job_inputs
                    .as_ref()
                    .and_then(|v| v.get("wsid"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });
        let doc_id = job
            .entity_refs
            .iter()
            .find(|r| r.entity_kind == "document")
            .map(|r| r.entity_id.clone())
            .or_else(|| {
                job.job_inputs
                    .as_ref()
                    .and_then(|v| v.get("doc_id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        let status = job_status_from_state(&job.state);
        let started_at = Some(job.created_at);
        let ended_at = match job.state {
            JobState::Completed
            | JobState::CompletedWithIssues
            | JobState::Failed
            | JobState::Cancelled
            | JobState::Poisoned => Some(job.updated_at),
            _ => None,
        };

        let diagnostics_for_job: Vec<String> = diagnostics
            .iter()
            .filter(|d| d.job_id.as_deref() == Some(&job.job_id.to_string()))
            .map(|d| d.id.clone())
            .collect();
        let event_ids: Vec<String> = events
            .iter()
            .filter(|e| e.job_id.as_deref() == Some(&job.job_id.to_string()))
            .map(|e| e.event_id.to_string())
            .collect();

        let metrics = if job.metrics.duration_ms == 0
            && job.metrics.total_tokens == 0
            && job.metrics.input_tokens == 0
            && job.metrics.output_tokens == 0
        {
            None
        } else {
            Some(BundleJobMetrics {
                duration_ms: Some(job.metrics.duration_ms),
                tokens_in: Some(job.metrics.input_tokens as u64),
                tokens_out: Some(job.metrics.output_tokens as u64),
                model_name: None,
            })
        };

        let error = if matches!(job.state, JobState::Failed | JobState::Poisoned)
            && job.error_message.is_some()
        {
            Some(BundleJobError {
                code: "job_failed".to_string(),
                message: job.error_message.clone().unwrap_or_default(),
                diagnostic_id: diagnostics_for_job.first().cloned(),
            })
        } else {
            None
        };

        BundleJob {
            job_id: job.job_id.to_string(),
            job_kind: job.job_kind.as_str().to_string(),
            protocol_id: job.protocol_id.clone(),
            status,
            created_at: job.created_at,
            started_at,
            ended_at,
            profile_id: job.profile_id.clone(),
            capability_profile_id: job.capability_profile_id.clone(),
            wsid,
            doc_id,
            inputs_hash: hash_optional_value(&job.job_inputs),
            outputs_hash: job.job_outputs.as_ref().map(hash_value),
            inputs_preview: match redaction_mode {
                RedactionMode::SafeDefault => None,
                _ => preview_json(&job.job_inputs, 200),
            },
            outputs_preview: match redaction_mode {
                RedactionMode::SafeDefault => None,
                _ => preview_json(&job.job_outputs, 200),
            },
            error,
            metrics,
            workflow_run_id: job.workflow_run_id.map(|id| id.to_string()),
            parent_job_id: None,
            diagnostic_ids: diagnostics_for_job,
            event_ids,
        }
    }

    fn build_retention_report(
        &self,
        available: AvailableCounts,
        missing: &[MissingEvidence],
    ) -> RetentionReport {
        RetentionReport {
            report_generated_at: Utc::now(),
            retention_policy: RetentionPolicy {
                flight_recorder_days: 7,
                diagnostics_days: 30,
                job_metadata_days: 30,
            },
            available,
            expired: ExpiredEvidence {
                jobs: Vec::new(),
                diagnostics: Vec::new(),
                event_ranges: Vec::new(),
            },
            evidence_gaps: missing
                .iter()
                .map(|m| EvidenceGap {
                    kind: m.kind.clone(),
                    description: format!("{} missing ({})", m.id, m.reason),
                    impact: "medium".to_string(),
                })
                .collect(),
        }
    }

    fn build_redaction_report(
        &self,
        mode: RedactionMode,
        redactor: &SecretRedactor,
        logs: &[RedactionLogEntry],
        files_scanned: u32,
    ) -> RedactionReport {
        let mut report = redactor.build_report(logs, mode, files_scanned);
        report.policy_decisions = SecretRedactor::policy_decision_allow_all();
        report
    }

    fn emit_fr_event(
        &self,
        bundle_id: &str,
        scope: &ManifestScope,
        redaction_mode: RedactionMode,
        jobs: &[BundleJob],
        diagnostics: &[BundleDiagnostic],
        event_count: usize,
        missing: &[MissingEvidence],
    ) {
        let payload = FrEvt005DebugBundleExport {
            bundle_id: bundle_id.to_string(),
            scope: format!("{:?}", scope.kind).to_lowercase(),
            redaction_mode: format!("{:?}", redaction_mode).to_uppercase(),
            included_job_ids: jobs.iter().map(|j| j.job_id.clone()).collect(),
            included_diagnostic_ids: diagnostics.iter().map(|d| d.id.clone()).collect(),
            included_wsids: scope.wsid.clone().into_iter().collect::<Vec<String>>(),
            event_count,
            missing_evidence: missing.iter().map(|m| json!(m)).collect(),
        };

        let mut event = FlightRecorderEvent::new(
            FlightRecorderEventType::DebugBundleExport,
            FlightRecorderActor::Agent,
            Uuid::new_v4(),
            json!(payload),
        )
        .with_job_id(bundle_id.to_string());

        if let Some(wsid) = scope.wsid.clone() {
            event = event.with_wsids(vec![wsid]);
        }

        let recorder = self.state.flight_recorder.clone();
        tokio::spawn(async move {
            let _ = recorder.record_event(event).await;
        });
    }
}

#[async_trait]
impl DebugBundleExporter for DefaultDebugBundleExporter {
    async fn export(
        &self,
        request: DebugBundleRequest,
    ) -> Result<BundleManifest, BundleExportError> {
        let bundle_id = Uuid::new_v4().to_string();
        let output_dir = request
            .output_path
            .clone()
            .unwrap_or_else(|| Self::default_output_path(&bundle_id));
        fs::create_dir_all(&output_dir)?;

        let mut scope = self.build_manifest_scope(&request.scope);
        let redactor = SecretRedactor::new();
        let (diagnostics_raw, mut missing_evidence) =
            self.collect_diagnostics(&request.scope).await?;
        let mut events_raw = self
            .collect_events(&request.scope, &diagnostics_raw)
            .await?;
        events_raw.sort_by(|a, b| {
            a.timestamp
                .cmp(&b.timestamp)
                .then_with(|| a.event_id.cmp(&b.event_id))
        });
        let (jobs_raw, missing_jobs) = self
            .collect_jobs(&request.scope, &diagnostics_raw, &events_raw)
            .await?;
        missing_evidence.extend(missing_jobs);

        let mut diagnostics: Vec<BundleDiagnostic> = diagnostics_raw
            .iter()
            .map(|d| self.map_diagnostic(d))
            .collect();
        diagnostics.sort_by(|a, b| a.id.cmp(&b.id));

        let mut jobs: Vec<BundleJob> = jobs_raw
            .iter()
            .map(|j| self.map_job(j, &diagnostics, &events_raw, request.redaction_mode))
            .collect();
        jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));

        if scope.wsid.is_none() {
            scope.wsid = diagnostics_raw
                .iter()
                .filter_map(|d| d.wsid.clone())
                .next()
                .or_else(|| jobs.iter().filter_map(|j| j.wsid.clone()).next());
        }

        let env = self.build_env(scope.wsid.clone(), request.redaction_mode);

        let timeline = if diagnostics_raw.is_empty() {
            None
        } else {
            let mut timestamps: Vec<DateTime<Utc>> =
                diagnostics_raw.iter().map(|d| d.timestamp).collect();
            timestamps.sort();
            Some((
                *timestamps.first().unwrap_or(&Utc::now()),
                *timestamps.last().unwrap_or(&Utc::now()),
                diagnostics_raw.len(),
            ))
        };

        let available_counts = AvailableCounts {
            jobs: jobs.len() as u32,
            diagnostics: diagnostics.len() as u32,
            events: events_raw.len() as u32,
        };
        let retention_report =
            self.build_retention_report(available_counts.clone(), &missing_evidence);

        // Prepare file payloads (JSON structures)
        let mut files: Vec<BundleFileEntry> = Vec::new();
        let mut redaction_logs: Vec<RedactionLogEntry> = Vec::new();

        // env.json
        let (env_value, logs_env) = redactor.redact_value(
            &serde_json::to_value(&env).unwrap_or(Value::Null),
            request.redaction_mode,
            "$.env",
        );
        redaction_logs.extend(
            logs_env
                .into_iter()
                .map(|mut log| {
                    log.file = "env.json".to_string();
                    log
                })
                .collect::<Vec<_>>(),
        );
        let env_bytes = serde_json::to_vec_pretty(&env_value)
            .map_err(|e| BundleExportError::ExportFailed(format!("serialize env.json: {}", e)))?;
        files.push(BundleFileEntry {
            path: "env.json".to_string(),
            bytes: env_bytes,
            redacted: true,
        });

        // jobs.json
        let (jobs_value, logs_jobs) = redactor.redact_value(
            &serde_json::to_value(&jobs).unwrap_or(Value::Null),
            request.redaction_mode,
            "$.jobs",
        );
        redaction_logs.extend(
            logs_jobs
                .into_iter()
                .map(|mut log| {
                    log.file = "jobs.json".to_string();
                    log
                })
                .collect::<Vec<_>>(),
        );
        let jobs_bytes = serde_json::to_vec_pretty(&jobs_value)
            .map_err(|e| BundleExportError::ExportFailed(format!("serialize jobs.json: {}", e)))?;
        files.push(BundleFileEntry {
            path: "jobs.json".to_string(),
            bytes: jobs_bytes,
            redacted: true,
        });

        // diagnostics.jsonl
        let mut diag_lines = Vec::new();
        for (idx, diagnostic) in diagnostics.iter().enumerate() {
            let (value, diag_logs) = redactor.redact_value(
                &serde_json::to_value(diagnostic).unwrap_or(Value::Null),
                request.redaction_mode,
                &format!("$[{idx}]"),
            );
            redaction_logs.extend(
                diag_logs
                    .into_iter()
                    .map(|mut log| {
                        log.file = "diagnostics.jsonl".to_string();
                        log
                    })
                    .collect::<Vec<_>>(),
            );
            let line = serde_json::to_string(&value).map_err(|e| {
                BundleExportError::ExportFailed(format!("serialize diagnostics entry: {}", e))
            })?;
            diag_lines.push(line);
        }
        let diagnostics_bytes = diag_lines.join("\n").into_bytes();
        files.push(BundleFileEntry {
            path: "diagnostics.jsonl".to_string(),
            bytes: diagnostics_bytes,
            redacted: true,
        });

        // trace.jsonl
        let mut trace_lines = Vec::new();
        for (idx, event) in events_raw.iter().enumerate() {
            let mut event_value = serde_json::to_value(event).unwrap_or(Value::Null);
            if let Some(obj) = event_value.as_object_mut() {
                if let Some(payload) = obj.remove("payload") {
                    let replacement = match request.redaction_mode {
                        RedactionMode::SafeDefault => Value::String(format!(
                            "[REDACTED:payload_hash:{}]",
                            hash_value(&payload)
                        )),
                        RedactionMode::Workspace => {
                            let preview = serde_json::to_string(&payload).unwrap_or_default();
                            Value::String(preview.chars().take(500).collect())
                        }
                        RedactionMode::FullLocal => payload,
                    };
                    obj.insert("payload".to_string(), replacement);
                }
            }
            let (value, event_logs) =
                redactor.redact_value(&event_value, request.redaction_mode, &format!("$[{idx}]"));
            redaction_logs.extend(
                event_logs
                    .into_iter()
                    .map(|mut log| {
                        log.file = "trace.jsonl".to_string();
                        log
                    })
                    .collect::<Vec<_>>(),
            );
            let line = serde_json::to_string(&value).map_err(|e| {
                BundleExportError::ExportFailed(format!("serialize trace entry: {}", e))
            })?;
            trace_lines.push(line);
        }
        let trace_bytes = trace_lines.join("\n").into_bytes();
        files.push(BundleFileEntry {
            path: "trace.jsonl".to_string(),
            bytes: trace_bytes,
            redacted: true,
        });

        // retention_report.json
        let retention_bytes = serde_json::to_vec_pretty(&retention_report).map_err(|e| {
            BundleExportError::ExportFailed(format!("serialize retention_report: {}", e))
        })?;
        files.push(BundleFileEntry {
            path: "retention_report.json".to_string(),
            bytes: retention_bytes,
            redacted: false,
        });

        // redaction_report.json
        let redaction_report =
            self.build_redaction_report(request.redaction_mode, &redactor, &redaction_logs, 4);
        let redaction_bytes = serde_json::to_vec_pretty(&redaction_report).map_err(|e| {
            BundleExportError::ExportFailed(format!("serialize redaction_report: {}", e))
        })?;
        files.push(BundleFileEntry {
            path: "redaction_report.json".to_string(),
            bytes: redaction_bytes,
            redacted: false,
        });

        // repro.md
        let repro_content = render_repro_md(
            &env,
            &scope,
            timeline.map(|(f, l, c)| (f, l, c)),
            jobs.get(0),
            diagnostics.get(0),
            false,
        );
        files.push(BundleFileEntry {
            path: "repro.md".to_string(),
            bytes: repro_content.into_bytes(),
            redacted: false,
        });

        // coder_prompt.md
        let coder_prompt = render_coder_prompt(
            diagnostics.get(0),
            &env,
            &scope,
            jobs.get(0),
            &missing_evidence,
            events_raw.len(),
        );
        files.push(BundleFileEntry {
            path: "coder_prompt.md".to_string(),
            bytes: coder_prompt.into_bytes(),
            redacted: false,
        });

        // bundle_manifest.json (constructed after hashes)
        let mut file_hashes: Vec<(String, String)> = Vec::new();
        for entry in &files {
            let hash = sha256_hex(&entry.bytes);
            file_hashes.push((entry.path.clone(), hash));
        }

        let manifest = BundleManifest {
            schema_version: "1.0".to_string(),
            bundle_id: bundle_id.clone(),
            bundle_kind: "debug_bundle".to_string(),
            created_at: Utc::now(),
            scope: scope.clone(),
            redaction_mode: request.redaction_mode,
            workflow_run_id: Uuid::new_v4().to_string(),
            job_id: Uuid::new_v4().to_string(),
            exporter_version: "0.1.0".to_string(),
            platform: PlatformInfo {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                app_version: "0.1.0".to_string(),
                build_hash: "unknown".to_string(),
            },
            files: files
                .iter()
                .map(|f| BundleManifestFile {
                    path: f.path.clone(),
                    sha256: sha256_hex(&f.bytes),
                    size_bytes: f.bytes.len() as u64,
                    redacted: f.redacted,
                })
                .collect(),
            included: IncludedCounts {
                job_count: jobs.len() as u32,
                diagnostic_count: diagnostics.len() as u32,
                event_count: events_raw.len() as u32,
            },
            missing_evidence: missing_evidence.clone(),
            bundle_hash: String::new(),
        };

        let mut manifest_with_hash = manifest.clone();
        let bundle_hash = compute_bundle_hash(&manifest, &file_hashes);
        manifest_with_hash.bundle_hash = bundle_hash.clone();

        let manifest_bytes = serde_json::to_vec_pretty(&manifest_with_hash).map_err(|e| {
            BundleExportError::ExportFailed(format!("serialize bundle_manifest: {}", e))
        })?;
        let manifest_entry = BundleFileEntry {
            path: "bundle_manifest.json".to_string(),
            bytes: manifest_bytes,
            redacted: false,
        };

        let mut all_files = files.clone();
        all_files.push(manifest_entry);

        // write files to disk for download/status
        for entry in &all_files {
            let full_path = output_dir.join(&entry.path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::File::create(&full_path)?;
            file.write_all(&entry.bytes)?;
        }

        // Build zip
        let zip_path = output_dir.join(format!("{}.zip", bundle_id));
        write_deterministic_zip(&zip_path, &all_files)?;

        // emit FR event
        self.emit_fr_event(
            &bundle_id,
            &scope,
            request.redaction_mode,
            &jobs,
            &diagnostics,
            events_raw.len(),
            &missing_evidence,
        );

        self.store_bundle_location(&bundle_id, output_dir.clone());

        Ok(manifest_with_hash)
    }

    async fn validate(
        &self,
        bundle_path: &Path,
    ) -> Result<crate::bundles::validator::BundleValidationReport, BundleExportError> {
        let validator = ValBundleValidator::default();
        validator.validate_dir(bundle_path)
    }

    async fn list_exportable(
        &self,
        filter: ExportableFilter,
    ) -> Result<ExportableInventory, BundleExportError> {
        let mut diag_filter = DiagFilter::default();
        diag_filter.wsid = filter.wsid.clone();
        diag_filter.from = filter.start;
        diag_filter.to = filter.end;
        diag_filter.limit = filter.limit.or(Some(50));
        let diagnostics = self
            .state
            .diagnostics
            .list_diagnostics(diag_filter)
            .await
            .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?;

        let job_filter = AiJobListFilter {
            from: filter.start,
            to: filter.end,
            ..Default::default()
        };
        let jobs = self
            .state
            .storage
            .list_ai_jobs(job_filter.clone())
            .await
            .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?;

        let mut exportable_jobs: Vec<_> = jobs
            .into_iter()
            .filter(|job| {
                filter
                    .wsid
                    .as_ref()
                    .map(|ws| job_matches_wsid(job, ws))
                    .unwrap_or(true)
            })
            .map(|job| ExportableJob {
                job_id: job.job_id.to_string(),
                job_kind: job.job_kind.as_str().to_string(),
                status: job_status_from_state(&job.state),
                created_at: job.created_at,
            })
            .collect();

        if let Some(limit) = filter.limit {
            exportable_jobs.truncate(limit as usize);
        }

        let exportable_diagnostics: Vec<_> = diagnostics
            .into_iter()
            .map(|diag| ExportableDiagnostic {
                diagnostic_id: diag.id.to_string(),
                severity: diag.severity.as_str().to_string(),
                title: diag.title,
            })
            .collect();

        let now = Utc::now();
        let time_range = Some(ExportableRange {
            earliest: filter.start.unwrap_or(now - Duration::hours(24)),
            latest: filter.end.unwrap_or(now),
        });

        Ok(ExportableInventory {
            jobs: exportable_jobs,
            diagnostics: exportable_diagnostics,
            time_range,
        })
    }
}
