use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read, Write};
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
    AvailableCounts, BundleDiagnostic, BundleDiagnosticSeverity, BundleEnv, BundleJob,
    BundleJobError, BundleJobMetrics, BundleJobStatus, BundleLinkConfidence, BundleManifest,
    BundleManifestFile, EvidenceGap, ExpiredEvidence, ExportableDiagnostic, ExportableFilter,
    ExportableInventory, ExportableJob, ExportableRange, ImpactLevel, IncludedCounts,
    ManifestScope, MissingEvidence, MissingEvidenceKind, MissingEvidenceReason, PlatformInfo,
    PlatformInfoMinimal, RedactionLogEntry, RedactionMode, RedactionReport, RetentionPolicy,
    RetentionReport, ScopeKind, TimeRange,
};
use crate::bundles::templates::{render_coder_prompt, render_repro_md};
use crate::bundles::zip::{sha256_hex, BundleFileEntry};
use crate::diagnostics::{DiagFilter, Diagnostic, DiagnosticSeverity, LinkConfidence};
use crate::flight_recorder::{
    EventFilter, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    FrEvt005DebugBundleExport,
};
use crate::governance_pack::{
    DeterminismLevel, ExportActor, ExportRecord, ExportTarget, ExporterInfo,
};
use crate::storage::artifacts::{
    artifact_root_rel, bundle_index_content_hash, bundle_index_json, compute_entries_index,
    materialize_local_dir, resolve_workspace_root, write_dir_artifact, ArtifactClassification,
    ArtifactError, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind, ArtifactWriteEntry,
    BundleIndexEntry,
};
use crate::storage::{AiJob, AiJobListFilter, JobState, StorageError};
use crate::AppState;

static BUNDLE_STORE: Lazy<Mutex<HashMap<String, PathBuf>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn deterministic_zip_bytes(files: &[BundleFileEntry]) -> Result<Vec<u8>, BundleExportError> {
    let cursor = Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let timestamp = zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
        .unwrap_or_else(|_| zip::DateTime::default());
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6))
        .last_modified_time(timestamp);

    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));

    for entry in sorted {
        writer.start_file(entry.path, options)?;
        writer.write_all(&entry.bytes)?;
    }

    let cursor = writer.finish()?;
    Ok(cursor.into_inner())
}

#[derive(Debug, Clone)]
struct ExportProvenance {
    bundle_id: String,
    workflow_run_id: String,
    trace_id: Uuid,
    export_job_id: Option<Uuid>,
}

impl ExportProvenance {
    fn standalone() -> Self {
        let bundle_id = Uuid::new_v4().to_string();
        Self {
            bundle_id,
            workflow_run_id: Uuid::new_v4().to_string(),
            trace_id: Uuid::new_v4(),
            export_job_id: None,
        }
    }
}

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

fn job_status_from_state(state: &JobState) -> BundleJobStatus {
    match state {
        JobState::Queued => BundleJobStatus::Queued,
        JobState::Running
        | JobState::AwaitingUser
        | JobState::AwaitingValidation
        | JobState::Stalled => BundleJobStatus::Running,
        JobState::Completed | JobState::CompletedWithIssues => BundleJobStatus::Completed,
        JobState::Failed | JobState::Poisoned => BundleJobStatus::Failed,
        JobState::Cancelled => BundleJobStatus::Cancelled,
    }
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

    pub async fn export_for_job(
        &self,
        request: DebugBundleRequest,
        export_job_id: Uuid,
        workflow_run_id: Uuid,
        trace_id: Uuid,
    ) -> Result<BundleManifest, BundleExportError> {
        export_impl(
            self,
            request,
            ExportProvenance {
                bundle_id: export_job_id.to_string(),
                workflow_run_id: workflow_run_id.to_string(),
                trace_id,
                export_job_id: Some(export_job_id),
            },
        )
        .await
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
                            kind: MissingEvidenceKind::Diagnostic,
                            id: diagnostic_id.clone(),
                            reason: MissingEvidenceReason::NotFound,
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
                let filter = DiagFilter {
                    job_id: Some(parsed),
                    limit: Some(1000),
                    ..Default::default()
                };
                self.state
                    .diagnostics
                    .list_diagnostics(filter)
                    .await
                    .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?
            }
            BundleScope::TimeWindow { start, end, wsid } => {
                let filter = DiagFilter {
                    from: Some(*start),
                    to: Some(*end),
                    wsid: wsid.clone(),
                    limit: Some(1000),
                    ..Default::default()
                };
                self.state
                    .diagnostics
                    .list_diagnostics(filter)
                    .await
                    .map_err(|e| BundleExportError::ExportFailed(e.to_string()))?
            }
            BundleScope::Workspace { wsid } => {
                let now = Utc::now();
                let filter = DiagFilter {
                    from: Some(now - Duration::hours(24)),
                    to: Some(now),
                    wsid: Some(wsid.clone()),
                    limit: Some(1000),
                    ..Default::default()
                };
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
                    kind: MissingEvidenceKind::Job,
                    id: job_id,
                    reason: MissingEvidenceReason::NotFound,
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

        let severity = match diagnostic.severity {
            DiagnosticSeverity::Fatal | DiagnosticSeverity::Error => {
                BundleDiagnosticSeverity::Error
            }
            DiagnosticSeverity::Warning => BundleDiagnosticSeverity::Warning,
            DiagnosticSeverity::Info => BundleDiagnosticSeverity::Info,
            DiagnosticSeverity::Hint => BundleDiagnosticSeverity::Hint,
        };

        let link_confidence = match diagnostic.link_confidence {
            LinkConfidence::Direct => BundleLinkConfidence::Direct,
            LinkConfidence::Inferred => BundleLinkConfidence::Inferred,
            LinkConfidence::Ambiguous => BundleLinkConfidence::Ambiguous,
            LinkConfidence::Unlinked => BundleLinkConfidence::Unlinked,
        };

        BundleDiagnostic {
            id: diagnostic.id.to_string(),
            fingerprint: diagnostic.fingerprint.clone(),
            severity,
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
            link_confidence,
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
                    kind: m.kind.as_str().to_string(),
                    description: format!("{} missing ({})", m.id, m.reason.as_str()),
                    impact: ImpactLevel::Medium,
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

    #[allow(clippy::too_many_arguments)]
    fn emit_fr_event(
        &self,
        trace_id: Uuid,
        bundle_id: &str,
        scope: &ManifestScope,
        redaction_mode: RedactionMode,
        jobs: &[BundleJob],
        diagnostics: &[BundleDiagnostic],
        event_count: usize,
        missing: &[MissingEvidence],
    ) {
        let scope_kind = match scope.kind {
            ScopeKind::Problem => "problem",
            ScopeKind::Job => "job",
            ScopeKind::TimeWindow => "time_window",
            ScopeKind::Workspace => "workspace",
        };

        let redaction_mode = match redaction_mode {
            RedactionMode::SafeDefault => "SAFE_DEFAULT",
            RedactionMode::Workspace => "WORKSPACE",
            RedactionMode::FullLocal => "FULL_LOCAL",
        };

        let payload = FrEvt005DebugBundleExport {
            bundle_id: bundle_id.to_string(),
            scope: scope_kind.to_string(),
            redaction_mode: redaction_mode.to_string(),
            included_job_ids: jobs.iter().map(|j| j.job_id.clone()).collect(),
            included_diagnostic_ids: diagnostics.iter().map(|d| d.id.clone()).collect(),
            included_wsids: scope.wsid.clone().into_iter().collect::<Vec<String>>(),
            event_count,
            missing_evidence: missing
                .iter()
                .map(|m| json!({ "kind": m.kind.as_str(), "reason": m.reason.as_str() }))
                .collect(),
        };

        let mut event = FlightRecorderEvent::new(
            FlightRecorderEventType::DebugBundleExport,
            FlightRecorderActor::Agent,
            trace_id,
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

async fn export_impl(
    exporter: &DefaultDebugBundleExporter,
    request: DebugBundleRequest,
    provenance: ExportProvenance,
) -> Result<BundleManifest, BundleExportError> {
    let ExportProvenance {
        bundle_id,
        workflow_run_id,
        trace_id,
        export_job_id,
    } = provenance;

    let workspace_root = resolve_workspace_root().map_err(|e| {
        BundleExportError::ExportFailed(format!("workspace root resolve failed: {e}"))
    })?;

    let output_dir = match request.output_path.clone() {
        Some(path) => {
            if !path.is_absolute() {
                return Err(BundleExportError::Validation(
                    "output_path must be an absolute directory path".to_string(),
                ));
            }
            path
        }
        None => workspace_root.join(DefaultDebugBundleExporter::default_output_path(&bundle_id)),
    };
    fs::create_dir_all(&output_dir)?;
    let output_dir = fs::canonicalize(&output_dir)?;

    let mut scope = exporter.build_manifest_scope(&request.scope);
    let redactor = SecretRedactor::new();

    let (diagnostics_raw, mut missing_evidence) =
        exporter.collect_diagnostics(&request.scope).await?;
    let mut events_raw = exporter
        .collect_events(&request.scope, &diagnostics_raw)
        .await?;
    events_raw.sort_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then_with(|| a.event_id.cmp(&b.event_id))
    });

    let max_events_limit = matches!(
        &request.scope,
        BundleScope::TimeWindow { .. } | BundleScope::Workspace { .. }
    );
    let truncated_event_count = if max_events_limit && events_raw.len() > 10_000 {
        let original = events_raw.len();
        events_raw.truncate(10_000);
        Some(original - 10_000)
    } else {
        None
    };

    let (jobs_raw, missing_jobs) = exporter
        .collect_jobs(&request.scope, &diagnostics_raw, &events_raw)
        .await?;
    missing_evidence.extend(missing_jobs);

    let present_event_ids: HashSet<String> =
        events_raw.iter().map(|e| e.event_id.to_string()).collect();

    let mut candidate_event_ids: Vec<String> = Vec::new();
    for diagnostic in &diagnostics_raw {
        if let Some(ids) = diagnostic
            .evidence_refs
            .as_ref()
            .and_then(|refs| refs.fr_event_ids.as_ref())
        {
            candidate_event_ids.extend(ids.clone());
        }
    }

    let mut event_ids_for_prompt: Vec<String> = Vec::new();
    for event_id in candidate_event_ids {
        if present_event_ids.contains(&event_id) {
            event_ids_for_prompt.push(event_id);
        }
    }
    event_ids_for_prompt.sort();
    event_ids_for_prompt.dedup();
    if event_ids_for_prompt.is_empty() {
        event_ids_for_prompt = events_raw
            .iter()
            .take(25)
            .map(|e| e.event_id.to_string())
            .collect();
    }
    if event_ids_for_prompt.len() > 50 {
        event_ids_for_prompt.truncate(50);
    }

    let mut diagnostics: Vec<BundleDiagnostic> = diagnostics_raw
        .iter()
        .map(|d| exporter.map_diagnostic(d))
        .collect();
    diagnostics.sort_by(|a, b| a.id.cmp(&b.id));

    let mut jobs: Vec<BundleJob> = jobs_raw
        .iter()
        .map(|j| exporter.map_job(j, &diagnostics, &events_raw, request.redaction_mode))
        .collect();
    jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));

    if scope.wsid.is_none() {
        scope.wsid = diagnostics_raw
            .iter()
            .filter_map(|d| d.wsid.clone())
            .next()
            .or_else(|| jobs.iter().filter_map(|j| j.wsid.clone()).next());
    }

    let env = exporter.build_env(scope.wsid.clone(), request.redaction_mode);

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
    let mut retention_report =
        exporter.build_retention_report(available_counts.clone(), &missing_evidence);
    if let Some(truncated) = truncated_event_count {
        retention_report.evidence_gaps.push(EvidenceGap {
            kind: "event_limit".to_string(),
            description: format!(
                "{} events omitted due to max_events=10000 constraint",
                truncated
            ),
            impact: ImpactLevel::Low,
        });
    }

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

    let jobs_file_name = if matches!(scope.kind, ScopeKind::Job) {
        "job.json"
    } else {
        "jobs.json"
    };

    // jobs.json / job.json
    let (jobs_value, logs_jobs) = redactor.redact_value(
        &serde_json::to_value(&jobs).unwrap_or(Value::Null),
        request.redaction_mode,
        "$.jobs",
    );
    redaction_logs.extend(
        logs_jobs
            .into_iter()
            .map(|mut log| {
                log.file = jobs_file_name.to_string();
                log
            })
            .collect::<Vec<_>>(),
    );
    let jobs_bytes = serde_json::to_vec_pretty(&jobs_value).map_err(|e| {
        BundleExportError::ExportFailed(format!("serialize {}: {}", jobs_file_name, e))
    })?;
    files.push(BundleFileEntry {
        path: jobs_file_name.to_string(),
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
                    RedactionMode::SafeDefault => {
                        Value::String(format!("[REDACTED:payload_hash:{}]", hash_value(&payload)))
                    }
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

    // repro.md
    let repro_content = render_repro_md(
        &env,
        &scope,
        timeline,
        jobs.first(),
        diagnostics.first(),
        false,
    );
    let (repro_value, repro_logs) = redactor.redact_value(
        &Value::String(repro_content),
        request.redaction_mode,
        "$.repro",
    );
    let repro_redacted = !repro_logs.is_empty();
    redaction_logs.extend(
        repro_logs
            .into_iter()
            .map(|mut log| {
                log.file = "repro.md".to_string();
                log
            })
            .collect::<Vec<_>>(),
    );
    files.push(BundleFileEntry {
        path: "repro.md".to_string(),
        bytes: repro_value.as_str().unwrap_or_default().as_bytes().to_vec(),
        redacted: repro_redacted,
    });

    // coder_prompt.md
    let coder_prompt = render_coder_prompt(
        diagnostics.first(),
        &env,
        &scope,
        jobs.first(),
        jobs_file_name,
        &missing_evidence,
        events_raw.len(),
        diagnostics.len(),
        &event_ids_for_prompt,
    );
    let (prompt_value, prompt_logs) = redactor.redact_value(
        &Value::String(coder_prompt),
        request.redaction_mode,
        "$.coder_prompt",
    );
    let prompt_redacted = !prompt_logs.is_empty();
    redaction_logs.extend(
        prompt_logs
            .into_iter()
            .map(|mut log| {
                log.file = "coder_prompt.md".to_string();
                log
            })
            .collect::<Vec<_>>(),
    );
    files.push(BundleFileEntry {
        path: "coder_prompt.md".to_string(),
        bytes: prompt_value
            .as_str()
            .unwrap_or_default()
            .as_bytes()
            .to_vec(),
        redacted: prompt_redacted,
    });

    // redaction_report.json (must include all redactions across bundle files)
    let redaction_report =
        exporter.build_redaction_report(request.redaction_mode, &redactor, &redaction_logs, 6);
    let redaction_bytes = serde_json::to_vec_pretty(&redaction_report).map_err(|e| {
        BundleExportError::ExportFailed(format!("serialize redaction_report: {}", e))
    })?;
    files.push(BundleFileEntry {
        path: "redaction_report.json".to_string(),
        bytes: redaction_bytes,
        redacted: false,
    });

    // Canonical BundleIndex (spec 2.3.10.7): sorted paths + per-item content_hash + size_bytes.
    // Note: BundleIndex excludes bundle_manifest.json and bundle_index.json to avoid recursion and
    // timestamp-based hash drift.
    let mut index_entries: Vec<BundleIndexEntry> = Vec::with_capacity(files.len());
    for entry in &files {
        index_entries.push(BundleIndexEntry {
            path: entry.path.clone(),
            content_hash: sha256_hex(&entry.bytes),
            size_bytes: entry.bytes.len() as u64,
        });
    }
    index_entries.sort_by(|a, b| a.path.cmp(&b.path));
    let bundle_index_bytes = bundle_index_json(&index_entries).map_err(map_artifact_error)?;
    let bundle_hash = bundle_index_content_hash(&bundle_index_bytes);
    files.push(BundleFileEntry {
        path: "bundle_index.json".to_string(),
        bytes: bundle_index_bytes.clone(),
        redacted: false,
    });

    let manifest = BundleManifest {
        schema_version: "1.0".to_string(),
        bundle_id: bundle_id.clone(),
        bundle_kind: "debug_bundle".to_string(),
        created_at: Utc::now(),
        scope: scope.clone(),
        redaction_mode: request.redaction_mode,
        workflow_run_id,
        job_id: bundle_id.clone(),
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
        bundle_hash: bundle_hash.clone(),
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| {
        BundleExportError::ExportFailed(format!("serialize bundle_manifest: {}", e))
    })?;
    let manifest_entry = BundleFileEntry {
        path: "bundle_manifest.json".to_string(),
        bytes: manifest_bytes,
        redacted: false,
    };

    let mut all_files = files.clone();
    all_files.push(manifest_entry);

    // Persist the canonical bundle payload as an artifact inside the workspace.
    let artifact_id = trace_id;
    let artifact_entries: Vec<ArtifactWriteEntry> = all_files
        .iter()
        .map(|entry| ArtifactWriteEntry {
            rel_path: entry.path.clone(),
            bytes: entry.bytes.clone(),
        })
        .collect();

    let mut hash_exclude_paths: Vec<String> = vec![
        "bundle_manifest.json".to_string(),
        "bundle_index.json".to_string(),
    ];
    hash_exclude_paths.sort();

    let (_artifact_index, artifact_size_bytes) =
        compute_entries_index(&artifact_entries, &hash_exclude_paths)
            .map_err(map_artifact_error)?;
    let artifact_manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L3,
        kind: ArtifactPayloadKind::Bundle,
        mime: "application/x-directory".to_string(),
        filename_hint: Some(format!("debug_bundle_{bundle_id}")),
        created_at: Utc::now(),
        created_by_job_id: export_job_id,
        source_entity_refs: Vec::new(),
        source_artifact_refs: Vec::new(),
        content_hash: bundle_hash.clone(),
        size_bytes: artifact_size_bytes,
        classification: if request.redaction_mode == RedactionMode::FullLocal {
            ArtifactClassification::High
        } else {
            ArtifactClassification::Medium
        },
        exportable: request.redaction_mode != RedactionMode::FullLocal,
        retention_ttl_days: if request.redaction_mode == RedactionMode::FullLocal {
            Some(30)
        } else {
            None
        },
        pinned: None,
        hash_basis: Some("bundle_index_v1".to_string()),
        hash_exclude_paths,
    };

    write_dir_artifact(&workspace_root, &artifact_manifest, &artifact_entries)
        .map_err(map_artifact_error)?;

    // Materialize payload to the LocalFile export target directory.
    let mut materialized_paths =
        materialize_local_dir(&output_dir, &artifact_entries, true).map_err(map_artifact_error)?;

    // Build zip
    let zip_name = format!("{}.zip", bundle_id);
    let zip_bytes = deterministic_zip_bytes(&all_files)?;
    materialize_local_dir(
        &output_dir,
        &[ArtifactWriteEntry {
            rel_path: zip_name.clone(),
            bytes: zip_bytes,
        }],
        true,
    )
    .map_err(map_artifact_error)?;
    materialized_paths.push(zip_name);
    materialized_paths.sort();

    // emit FR event
    exporter.emit_fr_event(
        trace_id,
        &bundle_id,
        &scope,
        request.redaction_mode,
        &jobs,
        &diagnostics,
        events_raw.len(),
        &missing_evidence,
    );

    exporter.store_bundle_location(&bundle_id, output_dir.clone());

    // Also record a canonical ExportRecord (re-uses GovernancePackExport event schema).
    record_export_record(
        exporter,
        trace_id,
        export_job_id,
        &bundle_id,
        &scope,
        request.redaction_mode,
        !redaction_logs.is_empty(),
        &bundle_hash,
        artifact_id,
        &materialized_paths,
        &output_dir,
    );

    Ok(manifest)
}

fn map_artifact_error(err: ArtifactError) -> BundleExportError {
    match err {
        ArtifactError::PathTraversalBlocked { path } => {
            BundleExportError::Validation(format!("path traversal blocked: {path}"))
        }
        ArtifactError::RootEscape { path } => {
            BundleExportError::Validation(format!("target escapes root: {path}"))
        }
        ArtifactError::MissingRetentionTtlDays { artifact_id, kind } => {
            BundleExportError::Validation(format!(
                "missing retention_ttl_days for high-sensitivity artifact: {artifact_id} ({kind:?})"
            ))
        }
        ArtifactError::ContentHashMismatch => {
            BundleExportError::Validation("content hash mismatch".to_string())
        }
        ArtifactError::InvalidSha256Hex { field } => {
            BundleExportError::Validation(format!("invalid sha256 hex: {field}"))
        }
        ArtifactError::InvalidRelPath { path } => {
            BundleExportError::Validation(format!("invalid path: {path}"))
        }
        ArtifactError::WorkspaceRootResolve(message) => {
            BundleExportError::ExportFailed(format!("workspace root resolve failed: {message}"))
        }
        ArtifactError::Serialization(source) => {
            BundleExportError::ExportFailed(format!("serialization error: {source}"))
        }
        ArtifactError::Io(source) => BundleExportError::IoError(source),
    }
}

fn record_export_record(
    exporter: &DefaultDebugBundleExporter,
    trace_id: Uuid,
    export_job_id: Option<Uuid>,
    bundle_id: &str,
    scope: &ManifestScope,
    redaction_mode: RedactionMode,
    redactions_applied: bool,
    bundle_hash: &str,
    artifact_id: Uuid,
    materialized_paths: &[String],
    output_dir: &Path,
) {
    let mut source_entity_refs: Vec<crate::storage::EntityRef> = Vec::new();
    if let Some(wsid) = scope.wsid.clone() {
        source_entity_refs.push(crate::storage::EntityRef {
            entity_id: wsid,
            entity_kind: "workspace".to_string(),
        });
    }
    if let Some(job_id) = scope.job_id.clone() {
        source_entity_refs.push(crate::storage::EntityRef {
            entity_id: job_id,
            entity_kind: "job".to_string(),
        });
    }
    if let Some(problem_id) = scope.problem_id.clone() {
        source_entity_refs.push(crate::storage::EntityRef {
            entity_id: problem_id,
            entity_kind: "problem".to_string(),
        });
    }
    if source_entity_refs.is_empty() {
        source_entity_refs.push(crate::storage::EntityRef {
            entity_id: bundle_id.to_string(),
            entity_kind: "debug_bundle".to_string(),
        });
    }

    let policy_id = match redaction_mode {
        RedactionMode::SafeDefault => "SAFE_DEFAULT",
        RedactionMode::Workspace => "WORKSPACE",
        RedactionMode::FullLocal => "FULL_LOCAL",
    }
    .to_string();

    let config_hash =
        sha256_hex(format!("handshake.debug_bundle_export.v1\npolicy_id={policy_id}\n").as_bytes());

    let export_record = ExportRecord {
        export_id: trace_id,
        created_at: Utc::now(),
        actor: ExportActor::HumanDev,
        job_id: export_job_id,
        source_entity_refs,
        source_hashes: vec![bundle_hash.to_string()],
        display_projection_ref: None,
        export_format: "debug_bundle".to_string(),
        exporter: ExporterInfo {
            engine_id: "handshake.debug_bundle_export".to_string(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            config_hash,
        },
        determinism_level: DeterminismLevel::Structural,
        export_target: ExportTarget::LocalFile {
            path: output_dir.to_path_buf(),
        },
        policy_id,
        redactions_applied,
        output_artifact_handles: vec![crate::ace::ArtifactHandle::new(
            artifact_id,
            artifact_root_rel(ArtifactLayer::L3, artifact_id),
        )],
        materialized_paths: materialized_paths.to_vec(),
        warnings: vec![
            "hash_basis: bundle_index_v1 (sorted paths + sha256(bytes) + size_bytes; excludes bundle_manifest.json and bundle_index.json)".to_string(),
        ],
        errors: Vec::new(),
    };

    let payload = match serde_json::to_value(&export_record) {
        Ok(value) => value,
        Err(_) => return,
    };

    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::GovernancePackExport,
        FlightRecorderActor::Agent,
        trace_id,
        payload,
    )
    .with_job_id(bundle_id.to_string());

    if let Some(wsid) = scope.wsid.clone() {
        event = event.with_wsids(vec![wsid]);
    }

    let recorder = exporter.state.flight_recorder.clone();
    tokio::spawn(async move {
        let _ = recorder.record_event(event).await;
    });
}

fn validate_bundle_path(
    bundle_path: &Path,
) -> Result<crate::bundles::validator::BundleValidationReport, BundleExportError> {
    if !bundle_path.exists() {
        return Err(BundleExportError::Validation(format!(
            "bundle path does not exist: {}",
            bundle_path.display()
        )));
    }
    if bundle_path.is_dir() {
        validate_bundle_dir(bundle_path)
    } else if bundle_path.is_file() {
        validate_bundle_zip(bundle_path)
    } else {
        Err(BundleExportError::Validation(format!(
            "unsupported bundle path type: {}",
            bundle_path.display()
        )))
    }
}

fn validate_bundle_dir(
    bundle_dir: &Path,
) -> Result<crate::bundles::validator::BundleValidationReport, BundleExportError> {
    let mut findings: Vec<ValidationFinding> = Vec::new();

    let manifest_path = bundle_dir.join("bundle_manifest.json");
    let manifest_bytes = fs::read(&manifest_path).map_err(BundleExportError::IoError)?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes).map_err(|e| {
        BundleExportError::Validation(format!("bundle_manifest.json parse error: {e}"))
    })?;

    for required in [
        "bundle_manifest.json",
        "bundle_index.json",
        "env.json",
        "trace.jsonl",
        "diagnostics.jsonl",
        "retention_report.json",
        "redaction_report.json",
        "repro.md",
        "coder_prompt.md",
    ] {
        if !bundle_dir.join(required).exists() {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-001:MISSING_FILE".to_string(),
                message: format!("Missing required file `{required}`"),
                file: Some(required.to_string()),
                path: None,
            });
        }
    }

    let has_jobs_json = bundle_dir.join("jobs.json").exists();
    let has_job_json = bundle_dir.join("job.json").exists();
    if !has_jobs_json && !has_job_json {
        findings.push(ValidationFinding {
            severity: FindingSeverity::Error,
            code: "VAL-BUNDLE-001:MISSING_FILE".to_string(),
            message: "Missing required file `jobs.json` OR `job.json`".to_string(),
            file: Some("jobs.json".to_string()),
            path: None,
        });
    }

    // Validate manifest.files entries.
    for file_entry in &manifest.files {
        let path = bundle_dir.join(&file_entry.path);
        if !path.exists() {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-002:MISSING_FILE".to_string(),
                message: format!("Manifest references missing file `{}`", file_entry.path),
                file: Some(file_entry.path.clone()),
                path: None,
            });
            continue;
        }
        let bytes = fs::read(&path).map_err(BundleExportError::IoError)?;
        let actual_sha = sha256_hex(&bytes);
        if actual_sha != file_entry.sha256 {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-002:HASH_MISMATCH".to_string(),
                message: format!(
                    "File sha256 mismatch for `{}` (expected {}, got {})",
                    file_entry.path, file_entry.sha256, actual_sha
                ),
                file: Some(file_entry.path.clone()),
                path: None,
            });
        }
        if bytes.len() as u64 != file_entry.size_bytes {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-002:SIZE_MISMATCH".to_string(),
                message: format!(
                    "File size mismatch for `{}` (expected {}, got {})",
                    file_entry.path,
                    file_entry.size_bytes,
                    bytes.len()
                ),
                file: Some(file_entry.path.clone()),
                path: None,
            });
        }
    }

    // Canonical bundle hash: sha256(bundle_index.json bytes).
    let bundle_index_path = bundle_dir.join("bundle_index.json");
    if let Ok(index_bytes) = fs::read(&bundle_index_path) {
        let computed = bundle_index_content_hash(&index_bytes);
        if computed != manifest.bundle_hash {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-003:BUNDLE_HASH_MISMATCH".to_string(),
                message: format!(
                    "bundle_hash mismatch (expected {}, got {})",
                    manifest.bundle_hash, computed
                ),
                file: Some("bundle_index.json".to_string()),
                path: None,
            });
        }

        if let Ok(index_entries) = serde_json::from_slice::<Vec<BundleIndexEntry>>(&index_bytes) {
            // Enforce sorted index (lexicographic).
            let mut last: Option<&str> = None;
            for entry in &index_entries {
                if let Some(prev) = last {
                    if entry.path.as_str() < prev {
                        findings.push(ValidationFinding {
                            severity: FindingSeverity::Error,
                            code: "VAL-BUNDLE-003:INDEX_NOT_SORTED".to_string(),
                            message: "bundle_index.json entries must be sorted by path".to_string(),
                            file: Some("bundle_index.json".to_string()),
                            path: None,
                        });
                        break;
                    }
                }
                last = Some(entry.path.as_str());

                let path = bundle_dir.join(&entry.path);
                if !path.exists() {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-003:INDEX_MISSING_FILE".to_string(),
                        message: format!(
                            "bundle_index.json references missing file `{}`",
                            entry.path
                        ),
                        file: Some(entry.path.clone()),
                        path: None,
                    });
                    continue;
                }
                let bytes = fs::read(&path).map_err(BundleExportError::IoError)?;
                let actual_sha = sha256_hex(&bytes);
                if actual_sha != entry.content_hash {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-003:INDEX_HASH_MISMATCH".to_string(),
                        message: format!(
                            "bundle_index.json content_hash mismatch for `{}` (expected {}, got {})",
                            entry.path, entry.content_hash, actual_sha
                        ),
                        file: Some(entry.path.clone()),
                        path: None,
                    });
                }
                if bytes.len() as u64 != entry.size_bytes {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-003:INDEX_SIZE_MISMATCH".to_string(),
                        message: format!(
                            "bundle_index.json size_bytes mismatch for `{}` (expected {}, got {})",
                            entry.path,
                            entry.size_bytes,
                            bytes.len()
                        ),
                        file: Some(entry.path.clone()),
                        path: None,
                    });
                }
            }
        }
    }

    let valid = !findings
        .iter()
        .any(|f| f.severity == FindingSeverity::Error);
    Ok(crate::bundles::validator::BundleValidationReport {
        valid,
        schema_version: manifest.schema_version.clone(),
        findings,
    })
}

fn validate_bundle_zip(
    bundle_zip: &Path,
) -> Result<crate::bundles::validator::BundleValidationReport, BundleExportError> {
    let mut findings: Vec<ValidationFinding> = Vec::new();

    let file = fs::File::open(bundle_zip)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut entries: HashSet<String> = HashSet::new();
    let mut manifest_bytes: Option<Vec<u8>> = None;
    let mut index_bytes: Option<Vec<u8>> = None;
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        if file.is_dir() {
            continue;
        }
        entries.insert(file.name().to_string());
        match file.name() {
            "bundle_manifest.json" => {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                manifest_bytes = Some(buf);
            }
            "bundle_index.json" => {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                index_bytes = Some(buf);
            }
            _ => {}
        }
    }

    let manifest_bytes = manifest_bytes.ok_or_else(|| {
        BundleExportError::Validation("bundle_manifest.json missing from zip".to_string())
    })?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes).map_err(|e| {
        BundleExportError::Validation(format!("bundle_manifest.json parse error: {e}"))
    })?;

    let index_bytes = index_bytes.ok_or_else(|| {
        BundleExportError::Validation("bundle_index.json missing from zip".to_string())
    })?;
    let computed = bundle_index_content_hash(&index_bytes);
    if computed != manifest.bundle_hash {
        findings.push(ValidationFinding {
            severity: FindingSeverity::Error,
            code: "VAL-BUNDLE-003:BUNDLE_HASH_MISMATCH".to_string(),
            message: format!(
                "bundle_hash mismatch (expected {}, got {})",
                manifest.bundle_hash, computed
            ),
            file: Some("bundle_index.json".to_string()),
            path: None,
        });
    }

    // Validate manifest.files entries (hash/size) by reading each from the archive.
    for file_entry in &manifest.files {
        let mut file = match archive.by_name(&file_entry.path) {
            Ok(f) => f,
            Err(_) => {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-002:MISSING_FILE".to_string(),
                    message: format!("Manifest references missing file `{}`", file_entry.path),
                    file: Some(file_entry.path.clone()),
                    path: None,
                });
                continue;
            }
        };
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf)?;
        let actual_sha = sha256_hex(&buf);
        if actual_sha != file_entry.sha256 {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-002:HASH_MISMATCH".to_string(),
                message: format!(
                    "File sha256 mismatch for `{}` (expected {}, got {})",
                    file_entry.path, file_entry.sha256, actual_sha
                ),
                file: Some(file_entry.path.clone()),
                path: None,
            });
        }
        if buf.len() as u64 != file_entry.size_bytes {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-002:SIZE_MISMATCH".to_string(),
                message: format!(
                    "File size mismatch for `{}` (expected {}, got {})",
                    file_entry.path,
                    file_entry.size_bytes,
                    buf.len()
                ),
                file: Some(file_entry.path.clone()),
                path: None,
            });
        }
    }

    let has_jobs_json = entries.contains("jobs.json");
    let has_job_json = entries.contains("job.json");
    if !has_jobs_json && !has_job_json {
        findings.push(ValidationFinding {
            severity: FindingSeverity::Error,
            code: "VAL-BUNDLE-001:MISSING_FILE".to_string(),
            message: "Missing required file `jobs.json` OR `job.json`".to_string(),
            file: Some("jobs.json".to_string()),
            path: None,
        });
    }

    let valid = !findings
        .iter()
        .any(|f| f.severity == FindingSeverity::Error);
    Ok(crate::bundles::validator::BundleValidationReport {
        valid,
        schema_version: manifest.schema_version.clone(),
        findings,
    })
}

#[async_trait]
impl DebugBundleExporter for DefaultDebugBundleExporter {
    async fn export(
        &self,
        request: DebugBundleRequest,
    ) -> Result<BundleManifest, BundleExportError> {
        export_impl(self, request, ExportProvenance::standalone()).await
    }

    async fn validate(
        &self,
        bundle_path: &Path,
    ) -> Result<crate::bundles::validator::BundleValidationReport, BundleExportError> {
        validate_bundle_path(bundle_path)
    }

    async fn list_exportable(
        &self,
        filter: ExportableFilter,
    ) -> Result<ExportableInventory, BundleExportError> {
        let diag_filter = DiagFilter {
            wsid: filter.wsid.clone(),
            from: filter.start,
            to: filter.end,
            limit: filter.limit.or(Some(50)),
            ..Default::default()
        };
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
                status: job_status_from_state(&job.state).as_str().to_string(),
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
