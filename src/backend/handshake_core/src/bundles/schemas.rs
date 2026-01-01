use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestScope {
    pub kind: ScopeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Problem,
    Job,
    TimeWindow,
    Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifestFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingEvidence {
    pub kind: MissingEvidenceKind,
    pub id: String,
    pub reason: MissingEvidenceReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissingEvidenceKind {
    Job,
    Diagnostic,
    Event,
    Artifact,
}

impl MissingEvidenceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MissingEvidenceKind::Job => "job",
            MissingEvidenceKind::Diagnostic => "diagnostic",
            MissingEvidenceKind::Event => "event",
            MissingEvidenceKind::Artifact => "artifact",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissingEvidenceReason {
    RetentionExpired,
    Redacted,
    AccessDenied,
    NotFound,
}

impl MissingEvidenceReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            MissingEvidenceReason::RetentionExpired => "retention_expired",
            MissingEvidenceReason::Redacted => "redacted",
            MissingEvidenceReason::AccessDenied => "access_denied",
            MissingEvidenceReason::NotFound => "not_found",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub schema_version: String,
    pub bundle_id: String,
    pub bundle_kind: String,
    pub created_at: DateTime<Utc>,
    pub scope: ManifestScope,
    pub redaction_mode: RedactionMode,
    pub workflow_run_id: String,
    pub job_id: String,
    pub exporter_version: String,
    pub platform: PlatformInfo,
    pub files: Vec<BundleManifestFile>,
    pub included: IncludedCounts,
    pub missing_evidence: Vec<MissingEvidence>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bundle_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub app_version: String,
    pub build_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludedCounts {
    pub job_count: u32,
    pub diagnostic_count: u32,
    pub event_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleEnv {
    pub app_version: String,
    pub build_hash: String,
    pub platform: PlatformInfoMinimal,
    pub rust_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    pub config: EnvConfig,
    pub feature_flags: Vec<String>,
    pub redaction_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfoMinimal {
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    pub model_runtime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    pub flight_recorder_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleJobError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleJobMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_out: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleJob {
    pub job_id: String,
    pub job_kind: String,
    pub protocol_id: String,
    pub status: BundleJobStatus,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    pub profile_id: String,
    pub capability_profile_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_id: Option<String>,
    pub inputs_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BundleJobError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<BundleJobMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job_id: Option<String>,
    pub diagnostic_ids: Vec<String>,
    pub event_ids: Vec<String>,
}

pub type BundleJobs = Vec<BundleJob>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BundleJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl BundleJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BundleJobStatus::Queued => "queued",
            BundleJobStatus::Running => "running",
            BundleJobStatus::Completed => "completed",
            BundleJobStatus::Failed => "failed",
            BundleJobStatus::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDiagnostic {
    pub id: String,
    pub fingerprint: String,
    pub severity: BundleDiagnosticSeverity,
    pub source: String,
    pub surface: String,
    pub code: String,
    pub title: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<u32>,
    pub link_confidence: BundleLinkConfidence,
    pub evidence_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_seen: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BundleDiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl BundleDiagnosticSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            BundleDiagnosticSeverity::Error => "error",
            BundleDiagnosticSeverity::Warning => "warning",
            BundleDiagnosticSeverity::Info => "info",
            BundleDiagnosticSeverity::Hint => "hint",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BundleLinkConfidence {
    Direct,
    Inferred,
    Ambiguous,
    Unlinked,
}

impl BundleLinkConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            BundleLinkConfidence::Direct => "direct",
            BundleLinkConfidence::Inferred => "inferred",
            BundleLinkConfidence::Ambiguous => "ambiguous",
            BundleLinkConfidence::Unlinked => "unlinked",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionReport {
    pub report_generated_at: DateTime<Utc>,
    pub retention_policy: RetentionPolicy,
    pub available: AvailableCounts,
    pub expired: ExpiredEvidence,
    pub evidence_gaps: Vec<EvidenceGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub flight_recorder_days: u32,
    pub diagnostics_days: u32,
    pub job_metadata_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCounts {
    pub jobs: u32,
    pub diagnostics: u32,
    pub events: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiredEvidence {
    pub jobs: Vec<ExpiredJob>,
    pub diagnostics: Vec<ExpiredDiagnostic>,
    pub event_ranges: Vec<ExpiredRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiredJob {
    pub job_id: String,
    pub expired_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiredDiagnostic {
    pub diagnostic_id: String,
    pub expired_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiredRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceGap {
    pub kind: String,
    pub description: String,
    pub impact: ImpactLevel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImpactLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionDetector {
    pub id: String,
    pub version: String,
    pub patterns_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionSummary {
    pub files_scanned: u32,
    pub files_redacted: u32,
    pub total_redactions: u32,
    pub by_category: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionLogEntry {
    pub file: String,
    pub location: String,
    pub category: String,
    pub detector_id: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub item_kind: String,
    pub item_id: String,
    pub decision: PolicyDecisionOutcome,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyDecisionOutcome {
    Include,
    Exclude,
    Redact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionReport {
    pub redaction_mode: RedactionMode,
    pub report_generated_at: DateTime<Utc>,
    pub detectors: Vec<RedactionDetector>,
    pub summary: RedactionSummary,
    pub redactions: Vec<RedactionLogEntry>,
    pub policy_decisions: Vec<PolicyDecision>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RedactionMode {
    SafeDefault,
    Workspace,
    FullLocal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableInventory {
    pub jobs: Vec<ExportableJob>,
    pub diagnostics: Vec<ExportableDiagnostic>,
    pub time_range: Option<ExportableRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableJob {
    pub job_id: String,
    pub job_kind: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableDiagnostic {
    pub diagnostic_id: String,
    pub severity: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableRange {
    pub earliest: DateTime<Utc>,
    pub latest: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportableFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}
