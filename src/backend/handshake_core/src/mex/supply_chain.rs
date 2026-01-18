use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::capabilities::CapabilityRegistry;
use crate::diagnostics::{
    DiagnosticActor, DiagnosticInput, DiagnosticSeverity, DiagnosticSource, DiagnosticSurface,
    DiagnosticsStore, LinkConfidence,
};
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::mex::envelope::{
    EngineError, EngineResult, EngineStatus, PlannedOperation, ProvenanceRecord,
};
use crate::mex::runtime::EngineAdapter;
use crate::terminal::config::TerminalConfig;
use crate::terminal::guards::TerminalGuard;
use crate::terminal::redaction::SecretRedactor;
use crate::terminal::{JobContext, TerminalMode, TerminalRequest, TerminalResult, TerminalService};

pub const SUPPLY_CHAIN_ALLOWLISTS_VERSION: &str = "supply_chain_allowlists.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SupplyChainReportKind {
    Vuln,
    #[serde(rename = "SBOM")]
    Sbom,
    License,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainReport {
    pub kind: SupplyChainReportKind,
    pub engine_version: String,
    pub timestamp: DateTime<Utc>,
    pub findings: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecretScanAllowlist {
    #[serde(default)]
    pub ignore_rule_ids: Vec<String>,
    #[serde(default)]
    pub ignore_path_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VulnScanAllowlist {
    #[serde(default)]
    pub ignore_vuln_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LicenseScanAllowlist {
    #[serde(default)]
    pub ignore_unknown_path_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyChainAllowlists {
    pub version: String,
    #[serde(default)]
    pub secret_scan: SecretScanAllowlist,
    #[serde(default)]
    pub vuln_scan: VulnScanAllowlist,
    #[serde(default)]
    pub license_scan: LicenseScanAllowlist,
}

impl Default for SupplyChainAllowlists {
    fn default() -> Self {
        Self {
            version: SUPPLY_CHAIN_ALLOWLISTS_VERSION.to_string(),
            secret_scan: SecretScanAllowlist::default(),
            vuln_scan: VulnScanAllowlist::default(),
            license_scan: LicenseScanAllowlist::default(),
        }
    }
}

#[derive(Debug, Error)]
enum SupplyChainError {
    #[error("SUPPLY_CHAIN_INVALID_ARTIFACT_PATH: invalid artifact path {path}")]
    InvalidArtifactPath { path: PathBuf },
    #[error("SUPPLY_CHAIN_IO_CREATE_DIR: mkdir {dir}: {source}")]
    CreateDir {
        dir: PathBuf,
        source: std::io::Error,
    },
    #[error("SUPPLY_CHAIN_IO_READ: read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("SUPPLY_CHAIN_IO_WRITE: write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("SUPPLY_CHAIN_JSON_PARSE: invalid json in {path}: {source}")]
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("SUPPLY_CHAIN_JSON_SERIALIZE: json serialization failed: {source}")]
    JsonSerialize { source: serde_json::Error },
    #[error("SUPPLY_CHAIN_ALLOWLISTS_OVERRIDE_INVALID: invalid allowlists override: {source}")]
    InvalidAllowlistsOverride { source: serde_json::Error },
    #[error("SUPPLY_CHAIN_TOOL_OUTPUT_MISSING: tool={tool} missing={path}")]
    ToolOutputMissing { tool: &'static str, path: PathBuf },
    #[error("SUPPLY_CHAIN_TOOL_INVOCATION_FAILED: tool={tool}")]
    ToolInvocationFailed { tool: &'static str },
    #[error("SUPPLY_CHAIN_TOOL_RUN_FAILED: tool={tool} error={message}")]
    ToolRunnerFailed { tool: &'static str, message: String },
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn rel_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[async_trait]
pub trait ToolRunner: Send + Sync {
    async fn run(&self, req: TerminalRequest, trace_id: Uuid) -> Result<TerminalResult, String>;
}

pub struct TerminalServiceRunner {
    cfg: TerminalConfig,
    registry: CapabilityRegistry,
    flight_recorder: Arc<dyn FlightRecorder>,
    redactor: Arc<dyn SecretRedactor>,
    guards: Vec<Box<dyn TerminalGuard>>,
}

impl TerminalServiceRunner {
    pub fn new(
        cfg: TerminalConfig,
        registry: CapabilityRegistry,
        flight_recorder: Arc<dyn FlightRecorder>,
        redactor: Arc<dyn SecretRedactor>,
        guards: Vec<Box<dyn TerminalGuard>>,
    ) -> Self {
        Self {
            cfg,
            registry,
            flight_recorder,
            redactor,
            guards,
        }
    }
}

#[async_trait]
impl ToolRunner for TerminalServiceRunner {
    async fn run(&self, req: TerminalRequest, trace_id: Uuid) -> Result<TerminalResult, String> {
        TerminalService::run_command(
            req,
            &self.cfg,
            &self.registry,
            self.flight_recorder.as_ref(),
            trace_id,
            self.redactor.as_ref(),
            &self.guards,
        )
        .await
        .map_err(|err| err.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanJobKind {
    SecretScan,
    VulnScan,
    SbomGenerate,
    LicenseScan,
}

impl ScanJobKind {
    fn from_op(op: &PlannedOperation) -> Option<Self> {
        match (op.engine_id.as_str(), op.operation.as_str()) {
            ("engine.guard.secret_scan", "secret_scan") => Some(Self::SecretScan),
            ("engine.supply_chain.vuln", "vuln_scan") => Some(Self::VulnScan),
            ("engine.supply_chain.sbom", "sbom_generate") => Some(Self::SbomGenerate),
            ("engine.supply_chain.license", "license_scan") => Some(Self::LicenseScan),
            _ => None,
        }
    }

    fn tool(&self) -> &'static str {
        match self {
            Self::SecretScan => "gitleaks",
            Self::VulnScan => "osv-scanner",
            Self::SbomGenerate => "syft",
            Self::LicenseScan => "scancode",
        }
    }

    fn requested_capability(&self) -> &'static str {
        match self {
            Self::SecretScan => "proc.exec:gitleaks",
            Self::VulnScan => "proc.exec:osv-scanner",
            Self::SbomGenerate => "proc.exec:syft",
            Self::LicenseScan => "proc.exec:scancode",
        }
    }

    fn report_kind(&self) -> Option<SupplyChainReportKind> {
        match self {
            Self::SecretScan => None,
            Self::VulnScan => Some(SupplyChainReportKind::Vuln),
            Self::SbomGenerate => Some(SupplyChainReportKind::Sbom),
            Self::LicenseScan => Some(SupplyChainReportKind::License),
        }
    }
}

pub struct SupplyChainEngineAdapter {
    tool_runner: Arc<dyn ToolRunner>,
    flight_recorder: Arc<dyn FlightRecorder>,
    diagnostics: Arc<dyn DiagnosticsStore>,
    artifact_root: PathBuf,
    allowlists: SupplyChainAllowlists,
}

impl SupplyChainEngineAdapter {
    pub fn new(
        tool_runner: Arc<dyn ToolRunner>,
        flight_recorder: Arc<dyn FlightRecorder>,
        diagnostics: Arc<dyn DiagnosticsStore>,
        artifact_root: PathBuf,
        allowlists: SupplyChainAllowlists,
    ) -> Self {
        Self {
            tool_runner,
            flight_recorder,
            diagnostics,
            artifact_root,
            allowlists,
        }
    }

    fn artifact_dir_for_op(&self, op: &PlannedOperation) -> PathBuf {
        PathBuf::from("data")
            .join("mex_supply_chain")
            .join(op.operation.replace('/', "_"))
            .join(op.op_id.to_string())
    }

    fn artifact_handle_for_rel(&self, rel_path: &Path) -> ArtifactHandle {
        ArtifactHandle::new(Uuid::new_v4(), rel_path_string(rel_path))
    }

    fn ensure_parent_dir(&self, abs_path: &Path) -> Result<(), String> {
        let Some(parent) = abs_path.parent() else {
            return Err(SupplyChainError::InvalidArtifactPath {
                path: abs_path.to_path_buf(),
            }
            .to_string());
        };
        fs::create_dir_all(parent).map_err(|source| {
            SupplyChainError::CreateDir {
                dir: parent.to_path_buf(),
                source,
            }
            .to_string()
        })
    }

    fn read_bytes(&self, abs_path: &Path) -> Vec<u8> {
        fs::read(abs_path).unwrap_or_default()
    }

    fn read_json_file(&self, abs_path: &Path) -> Result<Value, String> {
        let bytes = fs::read(abs_path).map_err(|source| {
            SupplyChainError::ReadFile {
                path: abs_path.to_path_buf(),
                source,
            }
            .to_string()
        })?;
        serde_json::from_slice(&bytes).map_err(|source| {
            SupplyChainError::InvalidJson {
                path: abs_path.to_path_buf(),
                source,
            }
            .to_string()
        })
    }

    fn write_json_file(&self, abs_path: &Path, value: &Value) -> Result<Vec<u8>, String> {
        let bytes = serde_json::to_vec_pretty(value)
            .map_err(|source| SupplyChainError::JsonSerialize { source }.to_string())?;
        self.ensure_parent_dir(abs_path)?;
        fs::write(abs_path, &bytes).map_err(|source| {
            SupplyChainError::WriteFile {
                path: abs_path.to_path_buf(),
                source,
            }
            .to_string()
        })?;
        Ok(bytes)
    }

    fn write_text_file(&self, abs_path: &Path, text: &str) -> Result<Vec<u8>, String> {
        let bytes = text.as_bytes().to_vec();
        self.ensure_parent_dir(abs_path)?;
        fs::write(abs_path, &bytes).map_err(|source| {
            SupplyChainError::WriteFile {
                path: abs_path.to_path_buf(),
                source,
            }
            .to_string()
        })?;
        Ok(bytes)
    }

    fn config_hash(allowlists: &SupplyChainAllowlists) -> String {
        let json = serde_json::to_string(allowlists).unwrap_or_default();
        sha256_hex(json.as_bytes())
    }

    fn parse_release_mode(op: &PlannedOperation) -> bool {
        op.params
            .get("release_mode")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn parse_allowlists_override(
        &self,
        op: &PlannedOperation,
    ) -> Result<SupplyChainAllowlists, String> {
        let Some(value) = op.params.get("allowlists") else {
            return Ok(self.allowlists.clone());
        };
        serde_json::from_value(value.clone())
            .map_err(|source| SupplyChainError::InvalidAllowlistsOverride { source }.to_string())
    }

    fn base_terminal_request(op: &PlannedOperation, requested_capability: &str) -> TerminalRequest {
        TerminalRequest {
            command: String::new(),
            args: Vec::new(),
            cwd: Some(PathBuf::from(".")),
            mode: TerminalMode::NonInteractive,
            timeout_ms: op.budget.wall_time_ms,
            max_output_bytes: op.budget.output_bytes,
            env_overrides: HashMap::new(),
            capture_stdout: true,
            capture_stderr: true,
            stdin_chunks: Vec::new(),
            idempotency_key: None,
            job_context: JobContext {
                job_id: Some(op.op_id.to_string()),
                model_id: None,
                session_id: None,
                capability_profile_id: None,
                capability_id: None,
                wsids: Vec::new(),
            },
            granted_capabilities: op.capabilities_requested.clone(),
            requested_capability: Some(requested_capability.to_string()),
            session_type: crate::terminal::TerminalSessionType::AiJob,
            human_consent_obtained: false,
        }
    }

    async fn tool_version(&self, kind: ScanJobKind, op: &PlannedOperation) -> String {
        let (command, argv_sets): (&str, Vec<Vec<&str>>) = match kind {
            ScanJobKind::SecretScan => ("gitleaks", vec![vec!["version"]]),
            ScanJobKind::VulnScan => ("osv-scanner", vec![vec!["--version"], vec!["version"]]),
            ScanJobKind::SbomGenerate => ("syft", vec![vec!["version"]]),
            ScanJobKind::LicenseScan => ("scancode", vec![vec!["--version"]]),
        };

        for argv in argv_sets {
            let mut req = Self::base_terminal_request(op, kind.requested_capability());
            req.command = command.to_string();
            req.args = argv.into_iter().map(|a| a.to_string()).collect();
            req.max_output_bytes = Some(128 * 1024);

            let Ok(result) = self.tool_runner.run(req, op.op_id).await else {
                continue;
            };
            if result.exit_code != 0 {
                continue;
            }

            let combined = format!("{}\n{}", result.stdout, result.stderr);
            if let Some(line) = combined.lines().map(str::trim).find(|l| !l.is_empty()) {
                return line.to_string();
            }
        }

        "unknown".to_string()
    }

    async fn emit_supply_chain_event(
        &self,
        op: &PlannedOperation,
        tool: &str,
        tool_version: &str,
        outputs: &[ArtifactHandle],
        evidence: &[ArtifactHandle],
        config_hash: &str,
        diagnostic_id: Option<Uuid>,
    ) -> Result<(), String> {
        let payload = json!({
            "component": "mex_supply_chain",
            "message": "supply_chain_op",
            "level": if diagnostic_id.is_some() { "error" } else { "info" },
            "details": {
                "engine_id": op.engine_id,
                "operation": op.operation,
                "op_id": op.op_id.to_string(),
                "tool": tool,
                "tool_version": tool_version,
                "inputs": op.inputs,
                "outputs": outputs,
                "evidence": evidence,
                "config_hash": config_hash,
                "diagnostic_id": diagnostic_id.map(|id| id.to_string()),
            }
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            op.op_id,
            payload,
        )
        .with_job_id(op.op_id.to_string())
        .with_actor_id("mex_supply_chain");

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|err| err.to_string())
    }

    async fn record_policy_diagnostic(
        &self,
        op: &PlannedOperation,
        title: String,
        message: String,
        severity: DiagnosticSeverity,
        tool: &str,
    ) -> Result<Uuid, String> {
        let diagnostic = DiagnosticInput {
            title,
            message,
            severity,
            source: DiagnosticSource::Engine,
            surface: DiagnosticSurface::System,
            tool: Some(tool.to_string()),
            code: None,
            tags: None,
            wsid: None,
            job_id: Some(op.op_id.to_string()),
            model_id: None,
            actor: Some(DiagnosticActor::System),
            capability_id: None,
            policy_decision_id: None,
            locations: None,
            evidence_refs: None,
            link_confidence: LinkConfidence::Direct,
            status: None,
            count: None,
            first_seen: None,
            last_seen: None,
            timestamp: None,
            updated_at: None,
        }
        .into_diagnostic()
        .map_err(|err| err.to_string())?;

        let id = diagnostic.id;
        self.diagnostics
            .record_diagnostic(diagnostic)
            .await
            .map_err(|err| err.to_string())?;
        Ok(id)
    }

    fn secret_scan_summary(report: &Value, allowlist: &SecretScanAllowlist) -> (u32, Vec<Value>) {
        let Some(items) = report.as_array() else {
            return (0, Vec::new());
        };

        let mut kept: Vec<Value> = Vec::new();
        for item in items {
            let rule_id = item
                .get("RuleID")
                .or_else(|| item.get("rule_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !rule_id.is_empty() && allowlist.ignore_rule_ids.iter().any(|id| id == rule_id) {
                continue;
            }

            let file = item
                .get("File")
                .or_else(|| item.get("file"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !file.is_empty()
                && allowlist
                    .ignore_path_prefixes
                    .iter()
                    .any(|prefix| file.starts_with(prefix))
            {
                continue;
            }

            kept.push(item.clone());
        }

        (kept.len() as u32, kept)
    }

    fn extract_cvss_score(severity: Option<&Value>) -> Option<f64> {
        let Some(severity) = severity else {
            return None;
        };
        let Value::Array(items) = severity else {
            return None;
        };

        let mut max_score: Option<f64> = None;
        for item in items {
            let Value::Object(map) = item else {
                continue;
            };
            let is_cvss = map
                .get("type")
                .and_then(|v| v.as_str())
                .map(|t| t.contains("CVSS"))
                .unwrap_or(false);
            if !is_cvss {
                continue;
            }
            let score = match map.get("score") {
                Some(Value::Number(n)) => n.as_f64(),
                Some(Value::String(s)) => s.parse::<f64>().ok(),
                _ => None,
            };
            let Some(score) = score else {
                continue;
            };
            max_score = Some(max_score.map(|s| s.max(score)).unwrap_or(score));
        }
        max_score
    }

    fn collect_osv_vulns(value: &Value, out: &mut Vec<(String, Option<f64>)>) {
        match value {
            Value::Array(items) => {
                for item in items {
                    Self::collect_osv_vulns(item, out);
                }
            }
            Value::Object(map) => {
                if let Some(Value::String(id)) = map.get("id") {
                    if id.starts_with("CVE-") || id.starts_with("GHSA-") || id.starts_with("OSV-") {
                        let score = Self::extract_cvss_score(map.get("severity"));
                        out.push((id.clone(), score));
                    }
                }
                for (_k, v) in map {
                    Self::collect_osv_vulns(v, out);
                }
            }
            _ => {}
        }
    }

    fn vuln_scan_summary(
        report: &Value,
        allowlist: &VulnScanAllowlist,
    ) -> (Vec<(String, Option<f64>)>, bool) {
        let mut vulns: Vec<(String, Option<f64>)> = Vec::new();
        Self::collect_osv_vulns(report, &mut vulns);
        vulns.sort_by(|a, b| a.0.cmp(&b.0));
        vulns.dedup_by(|a, b| a.0 == b.0);
        vulns.retain(|(id, _)| !allowlist.ignore_vuln_ids.contains(id));

        let high = vulns
            .iter()
            .any(|(_id, score)| score.map(|s| s >= 7.0).unwrap_or(false));
        (vulns, high)
    }

    fn unknown_license_in_scancode(
        report: &Value,
        allowlist: &LicenseScanAllowlist,
    ) -> (bool, Vec<String>) {
        let Some(files) = report.get("files").and_then(|v| v.as_array()) else {
            let tokens = Self::collect_unknown_tokens(report);
            return (!tokens.is_empty(), tokens);
        };

        let mut tokens: Vec<String> = Vec::new();
        for file in files {
            let path = file.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if !path.is_empty()
                && allowlist
                    .ignore_unknown_path_prefixes
                    .iter()
                    .any(|prefix| path.starts_with(prefix))
            {
                continue;
            }
            tokens.extend(Self::collect_unknown_tokens(file));
        }

        tokens.sort();
        tokens.dedup();
        (!tokens.is_empty(), tokens)
    }

    fn collect_unknown_tokens(value: &Value) -> Vec<String> {
        let mut matches: Vec<String> = Vec::new();
        Self::walk_strings(value, &mut |s| {
            let lower = s.to_ascii_lowercase();
            if lower == "unknown" || lower == "noassertion" || lower.contains("unknown") {
                matches.push(s.to_string());
            }
        });
        matches
    }

    fn walk_strings(value: &Value, f: &mut dyn FnMut(&str)) {
        match value {
            Value::String(s) => f(s),
            Value::Array(items) => {
                for item in items {
                    Self::walk_strings(item, f);
                }
            }
            Value::Object(map) => {
                for (_k, v) in map {
                    Self::walk_strings(v, f);
                }
            }
            _ => {}
        }
    }

    fn error_artifact_rel(op: &PlannedOperation) -> PathBuf {
        PathBuf::from("data")
            .join("mex_supply_chain")
            .join("errors")
            .join(format!("{}.txt", op.op_id))
    }
}

#[async_trait]
impl EngineAdapter for SupplyChainEngineAdapter {
    async fn invoke(
        &self,
        op: &PlannedOperation,
    ) -> Result<EngineResult, crate::mex::runtime::AdapterError> {
        let start = Utc::now();

        let Some(kind) = ScanJobKind::from_op(op) else {
            return Err(crate::mex::runtime::AdapterError::Engine(format!(
                "unsupported engine/operation: {}/{}",
                op.engine_id, op.operation
            )));
        };

        let release_mode = Self::parse_release_mode(op);
        let allowlists = self
            .parse_allowlists_override(op)
            .map_err(crate::mex::runtime::AdapterError::Engine)?;
        let config_hash = Self::config_hash(&allowlists);

        let artifact_dir_rel = self.artifact_dir_for_op(op);
        let artifact_dir_abs = self.artifact_root.join(&artifact_dir_rel);
        fs::create_dir_all(&artifact_dir_abs).map_err(|err| {
            crate::mex::runtime::AdapterError::Engine(format!(
                "failed to create {}: {err}",
                artifact_dir_abs.display()
            ))
        })?;

        let tool_version = self.tool_version(kind, op).await;

        let mut outputs: Vec<ArtifactHandle> = Vec::new();
        let mut evidence: Vec<ArtifactHandle> = Vec::new();
        let mut errors: Vec<EngineError> = Vec::new();
        let mut status = EngineStatus::Succeeded;
        let mut diagnostic_id: Option<Uuid> = None;

        let findings_result: Result<Value, String> = match kind {
            ScanJobKind::SecretScan => {
                let report_rel = artifact_dir_rel.join("gitleaks.report.json");
                let report_abs = self.artifact_root.join(&report_rel);

                let mut req = Self::base_terminal_request(op, kind.requested_capability());
                req.command = "gitleaks".to_string();
                req.args = vec![
                    "detect".to_string(),
                    "--source".to_string(),
                    ".".to_string(),
                    "--report-format".to_string(),
                    "json".to_string(),
                    "--report-path".to_string(),
                    report_abs.to_string_lossy().to_string(),
                    "--redact".to_string(),
                ];

                async {
                    let result = self
                        .tool_runner
                        .run(req, op.op_id)
                        .await
                        .map_err(|message| {
                            SupplyChainError::ToolRunnerFailed {
                                tool: kind.tool(),
                                message,
                            }
                            .to_string()
                        })?;
                    if !report_abs.exists() {
                        Err(
                            SupplyChainError::ToolOutputMissing {
                                tool: kind.tool(),
                                path: report_abs.clone(),
                            }
                            .to_string(),
                        )
                    } else {
                        let report_json = self.read_json_file(&report_abs)?;
                        let (count, filtered) =
                            Self::secret_scan_summary(&report_json, &allowlists.secret_scan);

                        let report_bytes = self.read_bytes(&report_abs);
                        let report_sha256 = sha256_hex(&report_bytes);
                        let report_handle = self.artifact_handle_for_rel(&report_rel);
                        outputs.push(report_handle.clone());
                        evidence.push(report_handle.clone());

                        if count > 0 {
                            status = EngineStatus::Failed;
                            diagnostic_id = Some(
                                self.record_policy_diagnostic(
                                    op,
                                    "Secret scan detected findings".to_string(),
                                    format!(
                                        "gitleaks reported {} finding(s). Report is redacted; see {}",
                                        count,
                                        report_handle.path.clone()
                                    ),
                                    DiagnosticSeverity::Fatal,
                                    "gitleaks",
                                )
                                .await?,
                            );
                            errors.push(EngineError {
                                code: "SUPPLY_CHAIN_SECRET_SCAN_BLOCK".to_string(),
                                message: format!("gitleaks findings: {}", count),
                                details_ref: Some(report_handle.clone()),
                            });
                        }

                        Ok(json!({
                            "tool": "gitleaks",
                            "tool_version": tool_version.clone(),
                            "tool_exit_code": result.exit_code,
                            "report": { "path": report_handle.path.clone(), "sha256": report_sha256 },
                            "finding_count": count,
                            "findings_sample": filtered.into_iter().take(20).collect::<Vec<_>>(),
                        }))
                    }
                }
                .await
            }
            ScanJobKind::VulnScan => {
                let report_rel = artifact_dir_rel.join("osv.report.json");
                let report_abs = self.artifact_root.join(&report_rel);

                let candidates: Vec<Vec<String>> = vec![
                    vec![
                        "scan".to_string(),
                        "--format".to_string(),
                        "json".to_string(),
                        "--output".to_string(),
                        report_abs.to_string_lossy().to_string(),
                        "--recursive".to_string(),
                        ".".to_string(),
                    ],
                    vec![
                        "scan".to_string(),
                        "--format".to_string(),
                        "json".to_string(),
                        "--output".to_string(),
                        report_abs.to_string_lossy().to_string(),
                        "-r".to_string(),
                        ".".to_string(),
                    ],
                ];

                async {
                    let mut last_result: Option<TerminalResult> = None;
                let mut ran = false;
                for args in candidates {
                    let mut req = Self::base_terminal_request(op, kind.requested_capability());
                    req.command = "osv-scanner".to_string();
                    req.args = args;
                    match self.tool_runner.run(req, op.op_id).await {
                        Ok(result) => {
                            ran = true;
                            last_result = Some(result);
                            if report_abs.exists() {
                                break;
                            }
                        }
                        Err(_) => continue,
                    }
                }

                if !ran {
                    Err(
                        SupplyChainError::ToolInvocationFailed { tool: kind.tool() }.to_string(),
                    )
                } else {
                    if !report_abs.exists() {
                        if let Some(result) = &last_result {
                            if !result.stdout.trim().is_empty() {
                                let _ = self.write_text_file(&report_abs, &result.stdout);
                            }
                        }
                    }
                    if !report_abs.exists() {
                        Err(
                            SupplyChainError::ToolOutputMissing {
                                tool: kind.tool(),
                                path: report_abs.clone(),
                            }
                            .to_string(),
                        )
                    } else {
                        let report_json = self.read_json_file(&report_abs)?;
                        let (vulns, high) =
                            Self::vuln_scan_summary(&report_json, &allowlists.vuln_scan);

                        let report_bytes = self.read_bytes(&report_abs);
                        let report_sha256 = sha256_hex(&report_bytes);
                        let report_handle = self.artifact_handle_for_rel(&report_rel);
                        evidence.push(report_handle.clone());

                        let supply_report = SupplyChainReport {
                            kind: SupplyChainReportKind::Vuln,
                            engine_version: tool_version.clone(),
                            timestamp: Utc::now(),
                            findings: json!({
                                "tool": "osv-scanner",
                                "tool_version": tool_version.clone(),
                                "tool_exit_code": last_result.as_ref().map(|r| r.exit_code).unwrap_or(-1),
                                "report": { "path": report_handle.path.clone(), "sha256": report_sha256 },
                                "vulnerability_count": vulns.len(),
                                "high_severity_present": high,
                                "vulnerabilities": vulns.iter().take(200).map(|(id, score)| json!({
                                    "id": id,
                                    "cvss_score": score,
                                })).collect::<Vec<_>>(),
                            }),
                        };

                        let supply_rel = artifact_dir_rel.join("supply_chain.report.json");
                        let supply_abs = self.artifact_root.join(&supply_rel);
                        let supply_value =
                            serde_json::to_value(&supply_report).map_err(|e| e.to_string())?;
                        let supply_bytes = self.write_json_file(&supply_abs, &supply_value)?;
                        let supply_sha256 = sha256_hex(&supply_bytes);
                        let supply_handle = self.artifact_handle_for_rel(&supply_rel);
                        outputs.push(supply_handle.clone());

                        if high {
                            let severity = if release_mode {
                                DiagnosticSeverity::Fatal
                            } else {
                                DiagnosticSeverity::Error
                            };
                            diagnostic_id = Some(
                                self.record_policy_diagnostic(
                                    op,
                                    "Vulnerability scan detected HIGH severity".to_string(),
                                    format!(
                                        "osv-scanner reported HIGH severity vulnerabilities (release_mode={}). SupplyChainReport: {} (sha256={})",
                                        release_mode,
                                        supply_handle.path.clone(),
                                        supply_sha256
                                    ),
                                    severity,
                                    "osv-scanner",
                                )
                                .await?,
                            );
                            if release_mode {
                                status = EngineStatus::Failed;
                                errors.push(EngineError {
                                    code: "SUPPLY_CHAIN_VULN_BLOCK".to_string(),
                                    message: "HIGH severity vulnerability detected in release mode"
                                        .to_string(),
                                    details_ref: Some(supply_handle.clone()),
                                });
                            }
                        }

                        Ok(supply_value)
                    }
                }
                }
                .await
            }
            ScanJobKind::SbomGenerate => {
                let sbom_rel = artifact_dir_rel.join("sbom.cyclonedx.json");
                let sbom_abs = self.artifact_root.join(&sbom_rel);

                let candidates: Vec<Vec<String>> = vec![
                    vec![
                        "dir:.".to_string(),
                        "-o".to_string(),
                        format!("cyclonedx-json={}", sbom_abs.to_string_lossy()),
                    ],
                    vec![
                        "dir:.".to_string(),
                        "-o".to_string(),
                        "cyclonedx-json".to_string(),
                    ],
                ];

                async {
                    let mut last_result: Option<TerminalResult> = None;
                let mut ran = false;
                for args in candidates {
                    let mut req = Self::base_terminal_request(op, kind.requested_capability());
                    req.command = "syft".to_string();
                    req.args = args;
                    match self.tool_runner.run(req, op.op_id).await {
                        Ok(result) => {
                            ran = true;
                            last_result = Some(result);
                            if sbom_abs.exists() {
                                break;
                            }
                        }
                        Err(_) => continue,
                    }
                }

                if !ran {
                    Err(
                        SupplyChainError::ToolInvocationFailed { tool: kind.tool() }.to_string(),
                    )
                } else {
                    if !sbom_abs.exists() {
                        if let Some(result) = &last_result {
                            if !result.stdout.trim().is_empty() {
                                let _ = self.write_text_file(&sbom_abs, &result.stdout);
                            }
                        }
                    }
                    if !sbom_abs.exists() {
                        Err(
                            SupplyChainError::ToolOutputMissing {
                                tool: kind.tool(),
                                path: sbom_abs.clone(),
                            }
                            .to_string(),
                        )
                    } else {
                        let sbom_bytes = self.read_bytes(&sbom_abs);
                        let sbom_sha256 = sha256_hex(&sbom_bytes);
                        let sbom_handle = self.artifact_handle_for_rel(&sbom_rel);
                        outputs.push(sbom_handle.clone());
                        evidence.push(sbom_handle.clone());

                        let supply_report = SupplyChainReport {
                            kind: SupplyChainReportKind::Sbom,
                            engine_version: tool_version.clone(),
                            timestamp: Utc::now(),
                            findings: json!({
                                "tool": "syft",
                                "tool_version": tool_version.clone(),
                                "tool_exit_code": last_result.as_ref().map(|r| r.exit_code).unwrap_or(-1),
                                "sbom": { "path": sbom_handle.path.clone(), "sha256": sbom_sha256 },
                            }),
                        };

                        let supply_rel = artifact_dir_rel.join("supply_chain.report.json");
                        let supply_abs = self.artifact_root.join(&supply_rel);
                        let supply_value =
                            serde_json::to_value(&supply_report).map_err(|e| e.to_string())?;
                        let _ = self.write_json_file(&supply_abs, &supply_value)?;
                        let supply_handle = self.artifact_handle_for_rel(&supply_rel);
                        outputs.push(supply_handle);

                        Ok(supply_value)
                    }
                }
                }
                .await
            }
            ScanJobKind::LicenseScan => {
                let report_rel = artifact_dir_rel.join("scancode.report.json");
                let report_abs = self.artifact_root.join(&report_rel);

                let mut req = Self::base_terminal_request(op, kind.requested_capability());
                req.command = "scancode".to_string();
                req.args = vec![
                    "--license".to_string(),
                    "--strip-root".to_string(),
                    "--timeout".to_string(),
                    "1200".to_string(),
                    "--json-pp".to_string(),
                    report_abs.to_string_lossy().to_string(),
                    ".".to_string(),
                    "--ignore".to_string(),
                    ".git".to_string(),
                    "--ignore".to_string(),
                    "node_modules".to_string(),
                    "--ignore".to_string(),
                    "target".to_string(),
                    "--ignore".to_string(),
                    "data".to_string(),
                ];

                async {
                    let result = self
                        .tool_runner
                        .run(req, op.op_id)
                        .await
                        .map_err(|message| {
                            SupplyChainError::ToolRunnerFailed {
                                tool: kind.tool(),
                                message,
                            }
                            .to_string()
                        })?;
                    if !report_abs.exists() {
                        Err(
                            SupplyChainError::ToolOutputMissing {
                                tool: kind.tool(),
                                path: report_abs.clone(),
                            }
                            .to_string(),
                        )
                    } else {
                        let report_json = self.read_json_file(&report_abs)?;
                        let (unknown_present, tokens) = Self::unknown_license_in_scancode(
                            &report_json,
                            &allowlists.license_scan,
                        );

                        let report_bytes = self.read_bytes(&report_abs);
                        let report_sha256 = sha256_hex(&report_bytes);
                        let report_handle = self.artifact_handle_for_rel(&report_rel);
                        evidence.push(report_handle.clone());

                        let supply_report = SupplyChainReport {
                            kind: SupplyChainReportKind::License,
                            engine_version: tool_version.clone(),
                            timestamp: Utc::now(),
                            findings: json!({
                                "tool": "scancode",
                                "tool_version": tool_version.clone(),
                                "tool_exit_code": result.exit_code,
                                "report": { "path": report_handle.path.clone(), "sha256": report_sha256 },
                                "unknown_license_present": unknown_present,
                                "unknown_token_sample": tokens.into_iter().take(20).collect::<Vec<_>>(),
                            }),
                        };

                        let supply_rel = artifact_dir_rel.join("supply_chain.report.json");
                        let supply_abs = self.artifact_root.join(&supply_rel);
                        let supply_value =
                            serde_json::to_value(&supply_report).map_err(|e| e.to_string())?;
                        let supply_bytes = self.write_json_file(&supply_abs, &supply_value)?;
                        let supply_sha256 = sha256_hex(&supply_bytes);
                        let supply_handle = self.artifact_handle_for_rel(&supply_rel);
                        outputs.push(supply_handle.clone());

                        if unknown_present {
                            let severity = if release_mode {
                                DiagnosticSeverity::Fatal
                            } else {
                                DiagnosticSeverity::Warning
                            };
                            diagnostic_id = Some(
                                self.record_policy_diagnostic(
                                    op,
                                    "License scan detected UNKNOWN".to_string(),
                                    format!(
                                        "scancode reported UNKNOWN license signal(s) (release_mode={}). SupplyChainReport: {} (sha256={})",
                                        release_mode,
                                        supply_handle.path.clone(),
                                        supply_sha256
                                    ),
                                    severity,
                                    "scancode",
                                )
                                .await?,
                            );
                            if release_mode {
                                status = EngineStatus::Failed;
                                errors.push(EngineError {
                                    code: "SUPPLY_CHAIN_LICENSE_BLOCK".to_string(),
                                    message: "UNKNOWN license detected in release mode".to_string(),
                                    details_ref: Some(supply_handle.clone()),
                                });
                            }
                        }

                        Ok(supply_value)
                    }
                }
                .await
            }
        };

        if let Err(err) = findings_result {
            status = EngineStatus::Failed;
            let error_rel = Self::error_artifact_rel(op);
            let error_abs = self.artifact_root.join(&error_rel);
            let _ = self
                .write_text_file(&error_abs, &err)
                .unwrap_or_else(|_| err.as_bytes().to_vec());
            let handle = self.artifact_handle_for_rel(&error_rel);
            evidence.push(handle.clone());
            errors.push(EngineError {
                code: "SUPPLY_CHAIN_TOOL_ERROR".to_string(),
                message: err.clone(),
                details_ref: Some(handle.clone()),
            });

            let diag_id = self
                .record_policy_diagnostic(
                    op,
                    "Supply-chain tool invocation failed".to_string(),
                    format!("tool={} error={}", kind.tool(), err),
                    DiagnosticSeverity::Error,
                    kind.tool(),
                )
                .await
                .unwrap_or_else(|_| Uuid::new_v4());
            diagnostic_id = Some(diag_id);
        }

        let _ = self
            .emit_supply_chain_event(
                op,
                kind.tool(),
                &tool_version,
                &outputs,
                &evidence,
                &config_hash,
                diagnostic_id,
            )
            .await;

        let provenance = ProvenanceRecord {
            engine_id: op.engine_id.clone(),
            engine_version: Some(tool_version),
            implementation: Some("mex_supply_chain".to_string()),
            determinism: op.determinism,
            config_hash: Some(config_hash),
            inputs: op.inputs.clone(),
            outputs: outputs.clone(),
            capabilities_granted: op.capabilities_requested.clone(),
            environment: Some(json!({
                "release_mode": release_mode,
                "allowlists_version": allowlists.version,
            })),
        };

        Ok(EngineResult {
            op_id: op.op_id,
            status,
            started_at: start,
            ended_at: Utc::now(),
            outputs,
            evidence,
            provenance,
            errors,
            logs_ref: None,
        })
    }
}
