use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::Path;

use regex::Regex;
use uuid::Uuid;

use crate::bundles::exporter::{BundleExportError, FindingSeverity, ValidationFinding};
use crate::bundles::schemas::{
    BundleDiagnostic, BundleEnv, BundleJobs, BundleManifest, RedactionMode, RedactionReport,
    RetentionReport, ScopeKind,
};
use crate::bundles::zip::{compute_bundle_hash, sha256_hex};

#[derive(Debug, Clone)]
pub struct BundleValidationReport {
    pub valid: bool,
    pub schema_version: String,
    pub findings: Vec<ValidationFinding>,
}

#[derive(Default)]
pub struct ValBundleValidator;

impl ValBundleValidator {
    pub fn validate_dir(&self, path: &Path) -> Result<BundleValidationReport, BundleExportError> {
        if !path.exists() {
            return Err(BundleExportError::Validation(format!(
                "bundle path does not exist: {}",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Err(BundleExportError::Validation(format!(
                "expected bundle directory, got: {}",
                path.display()
            )));
        }

        let mut entries: HashSet<String> = HashSet::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            entries.insert(entry.file_name().to_string_lossy().to_string());
        }

        let mut bytes: HashMap<String, Vec<u8>> = HashMap::new();
        for candidate in BUNDLE_CANDIDATE_FILES {
            if !entries.contains(*candidate) {
                continue;
            }
            if let Ok(content) = fs::read(path.join(candidate)) {
                bytes.insert((*candidate).to_string(), content);
            }
        }

        Ok(validate_contents(BundleContents { entries, bytes }))
    }

    pub fn validate_zip(&self, path: &Path) -> Result<BundleValidationReport, BundleExportError> {
        if !path.exists() {
            return Err(BundleExportError::Validation(format!(
                "bundle path does not exist: {}",
                path.display()
            )));
        }
        if !path.is_file() {
            return Err(BundleExportError::Validation(format!(
                "expected bundle ZIP file, got: {}",
                path.display()
            )));
        }

        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut entries: HashSet<String> = HashSet::new();
        for idx in 0..archive.len() {
            let file = archive.by_index(idx)?;
            if file.is_dir() {
                continue;
            }
            entries.insert(file.name().to_string());
        }

        let mut bytes: HashMap<String, Vec<u8>> = HashMap::new();
        for candidate in BUNDLE_CANDIDATE_FILES {
            if !entries.contains(*candidate) {
                continue;
            }
            let mut file = archive.by_name(candidate)?;
            let mut buf: Vec<u8> = Vec::new();
            file.read_to_end(&mut buf)?;
            bytes.insert((*candidate).to_string(), buf);
        }

        Ok(validate_contents(BundleContents { entries, bytes }))
    }
}

const BUNDLE_CANDIDATE_FILES: &[&str] = &[
    "bundle_manifest.json",
    "env.json",
    "jobs.json",
    "job.json",
    "trace.jsonl",
    "diagnostics.jsonl",
    "retention_report.json",
    "redaction_report.json",
    "repro.md",
    "coder_prompt.md",
];

#[derive(Debug)]
struct BundleContents {
    entries: HashSet<String>,
    bytes: HashMap<String, Vec<u8>>,
}

fn validate_contents(contents: BundleContents) -> BundleValidationReport {
    let mut findings: Vec<ValidationFinding> = Vec::new();

    // 1) Required files present
    for required in [
        "bundle_manifest.json",
        "env.json",
        "trace.jsonl",
        "diagnostics.jsonl",
        "retention_report.json",
        "redaction_report.json",
        "repro.md",
        "coder_prompt.md",
    ] {
        if !contents.entries.contains(required) {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-001:MISSING_FILE".to_string(),
                message: format!("Missing required file `{}`", required),
                file: Some(required.to_string()),
                path: None,
            });
        }
    }

    let has_jobs_json = contents.entries.contains("jobs.json");
    let has_job_json = contents.entries.contains("job.json");
    if !has_jobs_json && !has_job_json {
        findings.push(ValidationFinding {
            severity: FindingSeverity::Error,
            code: "VAL-BUNDLE-001:MISSING_FILE".to_string(),
            message: "Missing required file `jobs.json` OR `job.json`".to_string(),
            file: Some("jobs.json".to_string()),
            path: None,
        });
    }

    // 2) Schema compliance: parse bundle_manifest.json first; required for the rest.
    let (manifest, schema_version) = match contents.bytes.get("bundle_manifest.json") {
        Some(bytes) => match serde_json::from_slice::<BundleManifest>(bytes) {
            Ok(parsed) => {
                let schema = parsed.schema_version.clone();
                (Some(parsed), schema)
            }
            Err(err) => {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:PARSE_ERROR".to_string(),
                    message: format!("bundle_manifest.json parse error: {}", err),
                    file: Some("bundle_manifest.json".to_string()),
                    path: None,
                });
                (None, "unknown".to_string())
            }
        },
        None => (None, "unknown".to_string()),
    };

    let jobs_file_name = manifest
        .as_ref()
        .map(|m| match m.scope.kind {
            ScopeKind::Job => "job.json",
            _ => "jobs.json",
        })
        .unwrap_or_else(|| {
            if has_job_json {
                "job.json"
            } else {
                "jobs.json"
            }
        });

    let env = parse_json::<BundleEnv>(
        &contents,
        "env.json",
        &mut findings,
        "VAL-BUNDLE-001:PARSE_ERROR",
    );
    let jobs = parse_json::<BundleJobs>(
        &contents,
        jobs_file_name,
        &mut findings,
        "VAL-BUNDLE-001:PARSE_ERROR",
    );
    let diagnostics = parse_jsonl::<BundleDiagnostic>(
        &contents,
        "diagnostics.jsonl",
        &mut findings,
        "VAL-BUNDLE-001:PARSE_ERROR",
    );
    let trace_events = parse_jsonl_value(&contents, "trace.jsonl", &mut findings);
    let retention = parse_json::<RetentionReport>(
        &contents,
        "retention_report.json",
        &mut findings,
        "VAL-BUNDLE-001:PARSE_ERROR",
    );
    let redaction = parse_json::<RedactionReport>(
        &contents,
        "redaction_report.json",
        &mut findings,
        "VAL-BUNDLE-001:PARSE_ERROR",
    );

    // 3) Internal consistency + 5) Missing evidence accounting + 4) Redaction compliance.
    if let Some(manifest) = manifest.as_ref() {
        if manifest.schema_version != "1.0" {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                message: format!(
                    "schema_version must be \"1.0\", got \"{}\"",
                    manifest.schema_version
                ),
                file: Some("bundle_manifest.json".to_string()),
                path: Some("$.schema_version".to_string()),
            });
        }
        if manifest.bundle_kind != "debug_bundle" {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                message: format!(
                    "bundle_kind must be \"debug_bundle\", got \"{}\"",
                    manifest.bundle_kind
                ),
                file: Some("bundle_manifest.json".to_string()),
                path: Some("$.bundle_kind".to_string()),
            });
        }
        if Uuid::parse_str(&manifest.bundle_id).is_err() {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                message: "bundle_id must be a uuid".to_string(),
                file: Some("bundle_manifest.json".to_string()),
                path: Some("$.bundle_id".to_string()),
            });
        }

        // Scope conditional requirements.
        match manifest.scope.kind {
            ScopeKind::Problem => {
                if manifest
                    .scope
                    .problem_id
                    .as_deref()
                    .unwrap_or("")
                    .is_empty()
                {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                        message: "scope.problem_id is required when scope.kind=problem".to_string(),
                        file: Some("bundle_manifest.json".to_string()),
                        path: Some("$.scope.problem_id".to_string()),
                    });
                }
            }
            ScopeKind::Job => {
                if manifest.scope.job_id.as_deref().unwrap_or("").is_empty() {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                        message: "scope.job_id is required when scope.kind=job".to_string(),
                        file: Some("bundle_manifest.json".to_string()),
                        path: Some("$.scope.job_id".to_string()),
                    });
                }
            }
            ScopeKind::TimeWindow => {
                if let Some(range) = manifest.scope.time_range.as_ref() {
                    if range.start > range.end {
                        findings.push(ValidationFinding {
                            severity: FindingSeverity::Error,
                            code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                            message: "scope.time_range.start must be <= end".to_string(),
                            file: Some("bundle_manifest.json".to_string()),
                            path: Some("$.scope.time_range".to_string()),
                        });
                    }
                } else {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                        message: "scope.time_range is required when scope.kind=time_window"
                            .to_string(),
                        file: Some("bundle_manifest.json".to_string()),
                        path: Some("$.scope.time_range".to_string()),
                    });
                }
            }
            ScopeKind::Workspace => {
                if manifest.scope.wsid.as_deref().unwrap_or("").is_empty() {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                        message: "scope.wsid is required when scope.kind=workspace".to_string(),
                        file: Some("bundle_manifest.json".to_string()),
                        path: Some("$.scope.wsid".to_string()),
                    });
                }
            }
        }

        let expected_jobs_file = match manifest.scope.kind {
            ScopeKind::Job => "job.json",
            _ => "jobs.json",
        };
        if !contents.entries.contains(expected_jobs_file) {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-001:MISSING_FILE".to_string(),
                message: format!(
                    "Missing required file `{}` for scope.kind={:?}",
                    expected_jobs_file, manifest.scope.kind
                ),
                file: Some(expected_jobs_file.to_string()),
                path: None,
            });
        }

        // redaction_report.json consistency.
        if let Some(redaction) = redaction.as_ref() {
            if redaction.redaction_mode != manifest.redaction_mode {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                    message: format!(
                        "redaction_report.redaction_mode={:?} does not match manifest.redaction_mode={:?}",
                        redaction.redaction_mode, manifest.redaction_mode
                    ),
                    file: Some("redaction_report.json".to_string()),
                    path: Some("$.redaction_mode".to_string()),
                });
            }
            if redaction.redactions.len() as u32 != redaction.summary.total_redactions {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                    message: format!(
                        "redaction_report.summary.total_redactions={} does not match redactions.len()={}",
                        redaction.summary.total_redactions,
                        redaction.redactions.len()
                    ),
                    file: Some("redaction_report.json".to_string()),
                    path: Some("$.summary.total_redactions".to_string()),
                });
            }
        }

        // Manifest files inventory + hashes.
        let required_inventory_files = [
            "env.json",
            expected_jobs_file,
            "trace.jsonl",
            "diagnostics.jsonl",
            "retention_report.json",
            "redaction_report.json",
            "repro.md",
            "coder_prompt.md",
        ];

        let mut manifest_paths: HashSet<&str> = HashSet::new();
        for entry in &manifest.files {
            if !manifest_paths.insert(entry.path.as_str()) {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                    message: format!("Duplicate manifest.files entry for `{}`", entry.path),
                    file: Some("bundle_manifest.json".to_string()),
                    path: Some("$.files".to_string()),
                });
            }
        }

        for required in &required_inventory_files {
            if !manifest_paths.contains(required) {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                    message: format!(
                        "bundle_manifest.files must include `{}` (required bundle file)",
                        required
                    ),
                    file: Some("bundle_manifest.json".to_string()),
                    path: Some("$.files".to_string()),
                });
            }
        }

        let mut expected_hash_re = None;
        if let Ok(re) = Regex::new(r"^[0-9a-f]{64}$") {
            expected_hash_re = Some(re);
        }

        for file_entry in &manifest.files {
            if file_entry.path == "bundle_manifest.json" {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                    message: "bundle_manifest.json must not appear in bundle_manifest.files"
                        .to_string(),
                    file: Some("bundle_manifest.json".to_string()),
                    path: Some("$.files".to_string()),
                });
            }

            if let Some(re) = expected_hash_re.as_ref() {
                if !re.is_match(&file_entry.sha256) {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                        message: format!(
                            "Invalid sha256 format for `{}` (expected lowercase hex)",
                            file_entry.path
                        ),
                        file: Some("bundle_manifest.json".to_string()),
                        path: Some("$.files".to_string()),
                    });
                }
            }

            let Some(bytes) = contents.bytes.get(&file_entry.path) else {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                    message: format!("Manifest references missing file `{}`", file_entry.path),
                    file: Some("bundle_manifest.json".to_string()),
                    path: Some("$.files".to_string()),
                });
                continue;
            };
            let actual_sha = sha256_hex(bytes);
            if actual_sha != file_entry.sha256 {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:HASH_MISMATCH".to_string(),
                    message: format!(
                        "sha256 mismatch for `{}` (manifest={}, actual={})",
                        file_entry.path, file_entry.sha256, actual_sha
                    ),
                    file: Some(file_entry.path.clone()),
                    path: None,
                });
            }
            let actual_size = bytes.len() as u64;
            if actual_size != file_entry.size_bytes {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:SCHEMA_VIOLATION".to_string(),
                    message: format!(
                        "size_bytes mismatch for `{}` (manifest={}, actual={})",
                        file_entry.path, file_entry.size_bytes, actual_size
                    ),
                    file: Some(file_entry.path.clone()),
                    path: None,
                });
            }
        }

        // "files array matches actual bundle contents" (excluding bundle_manifest.json and the download ZIP).
        let actual_files: HashSet<&str> = contents
            .entries
            .iter()
            .filter(|name| !name.ends_with(".zip"))
            .filter(|name| !name.starts_with('.'))
            .filter(|name| name.as_str() != "bundle_manifest.json")
            .map(|name| name.as_str())
            .collect();
        let extra_files: Vec<&str> = actual_files.difference(&manifest_paths).copied().collect();
        if !extra_files.is_empty() {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                message: format!(
                    "Bundle contains files not listed in bundle_manifest.files: {}",
                    extra_files.join(", ")
                ),
                file: Some("bundle_manifest.json".to_string()),
                path: Some("$.files".to_string()),
            });
        }

        // bundle_hash algorithm check.
        let manifest_hashes: Vec<(String, String)> = manifest
            .files
            .iter()
            .map(|f| (f.path.clone(), f.sha256.clone()))
            .collect();
        let computed = compute_bundle_hash(manifest, &manifest_hashes);
        if manifest.bundle_hash != computed {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: "VAL-BUNDLE-001:HASH_MISMATCH".to_string(),
                message: format!(
                    "bundle_hash mismatch (manifest={}, computed={})",
                    manifest.bundle_hash, computed
                ),
                file: Some("bundle_manifest.json".to_string()),
                path: Some("$.bundle_hash".to_string()),
            });
        }

        // Internal references: coder_prompt.md -> jobs/diagnostics/trace.
        if let (Some(jobs), Some(diagnostics), Some(trace_events)) =
            (jobs.as_ref(), diagnostics.as_ref(), trace_events.as_ref())
        {
            let job_ids: HashSet<&str> = jobs.iter().map(|j| j.job_id.as_str()).collect();
            let diag_ids: HashSet<&str> = diagnostics.iter().map(|d| d.id.as_str()).collect();
            let event_ids: HashSet<&str> = trace_events
                .iter()
                .filter_map(|v| v.get("event_id").and_then(|v| v.as_str()))
                .collect();

            let coder_prompt = contents
                .bytes
                .get("coder_prompt.md")
                .map(|b| String::from_utf8_lossy(b).to_string())
                .unwrap_or_default();

            for referenced in extract_ids_with_label(&coder_prompt, "Job ID") {
                if !job_ids.contains(referenced.as_str()) {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                        message: format!(
                            "coder_prompt.md references job_id `{}` that is not present in `{}`",
                            referenced, expected_jobs_file
                        ),
                        file: Some("coder_prompt.md".to_string()),
                        path: None,
                    });
                }
            }

            for referenced in extract_ids_with_label(&coder_prompt, "Diagnostic ID") {
                if !diag_ids.contains(referenced.as_str()) {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                        message: format!(
                            "coder_prompt.md references diagnostic_id `{}` that is not present in diagnostics.jsonl",
                            referenced
                        ),
                        file: Some("coder_prompt.md".to_string()),
                        path: None,
                    });
                }
            }

            if let Some(event_line) = line_after_label(&coder_prompt, "Event IDs") {
                for referenced in extract_uuids(event_line) {
                    if !event_ids.contains(referenced.as_str()) {
                        findings.push(ValidationFinding {
                            severity: FindingSeverity::Error,
                            code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                            message: format!(
                                "coder_prompt.md references event_id `{}` that is not present in trace.jsonl",
                                referenced
                            ),
                            file: Some("coder_prompt.md".to_string()),
                            path: None,
                        });
                    }
                }
            }

            // Scope referenced IDs should exist too.
            if matches!(manifest.scope.kind, ScopeKind::Job) {
                if let Some(scope_job_id) = manifest.scope.job_id.as_deref() {
                    if !job_ids.contains(scope_job_id) {
                        findings.push(ValidationFinding {
                            severity: FindingSeverity::Error,
                            code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                            message: format!(
                                "scope.job_id `{}` not present in `{}`",
                                scope_job_id, expected_jobs_file
                            ),
                            file: Some("bundle_manifest.json".to_string()),
                            path: Some("$.scope.job_id".to_string()),
                        });
                    }
                }
            }
            if matches!(manifest.scope.kind, ScopeKind::Problem) {
                if let Some(problem_id) = manifest.scope.problem_id.as_deref() {
                    if !diag_ids.contains(problem_id) {
                        findings.push(ValidationFinding {
                            severity: FindingSeverity::Error,
                            code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                            message: format!(
                                "scope.problem_id `{}` not present in diagnostics.jsonl",
                                problem_id
                            ),
                            file: Some("bundle_manifest.json".to_string()),
                            path: Some("$.scope.problem_id".to_string()),
                        });
                    }
                }
            }

            if let (Some(env), Some(wsid)) = (env.as_ref(), manifest.scope.wsid.as_deref()) {
                if env.wsid.as_deref() != Some(wsid) {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Warning,
                        code: "VAL-BUNDLE-001:INTERNAL_MISMATCH".to_string(),
                        message: format!(
                            "env.wsid `{}` does not match manifest.scope.wsid `{}`",
                            env.wsid.as_deref().unwrap_or("n/a"),
                            wsid
                        ),
                        file: Some("env.json".to_string()),
                        path: Some("$.wsid".to_string()),
                    });
                }
            }
        }

        // 5) Missing evidence accounting (retention_report aligns with missing_evidence).
        if let Some(retention) = retention.as_ref() {
            let retention_job_ids: HashSet<&str> = retention
                .expired
                .jobs
                .iter()
                .map(|j| j.job_id.as_str())
                .collect();
            let retention_diag_ids: HashSet<&str> = retention
                .expired
                .diagnostics
                .iter()
                .map(|d| d.diagnostic_id.as_str())
                .collect();

            let missing_job_retention: HashSet<&str> = manifest
                .missing_evidence
                .iter()
                .filter(|m| m.kind.as_str() == "job" && m.reason.as_str() == "retention_expired")
                .map(|m| m.id.as_str())
                .collect();
            let missing_diag_retention: HashSet<&str> = manifest
                .missing_evidence
                .iter()
                .filter(|m| {
                    m.kind.as_str() == "diagnostic" && m.reason.as_str() == "retention_expired"
                })
                .map(|m| m.id.as_str())
                .collect();

            for id in &missing_job_retention {
                if !retention_job_ids.contains(id) {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:MISSING_EVIDENCE_MISMATCH".to_string(),
                        message: format!(
                            "missing_evidence includes job `{}` retention_expired but retention_report.expired.jobs does not",
                            id
                        ),
                        file: Some("retention_report.json".to_string()),
                        path: Some("$.expired.jobs".to_string()),
                    });
                }
            }
            for id in &missing_diag_retention {
                if !retention_diag_ids.contains(id) {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:MISSING_EVIDENCE_MISMATCH".to_string(),
                        message: format!(
                            "missing_evidence includes diagnostic `{}` retention_expired but retention_report.expired.diagnostics does not",
                            id
                        ),
                        file: Some("retention_report.json".to_string()),
                        path: Some("$.expired.diagnostics".to_string()),
                    });
                }
            }
            for id in &retention_job_ids {
                if !missing_job_retention.contains(id) {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:MISSING_EVIDENCE_MISMATCH".to_string(),
                        message: format!(
                            "retention_report.expired.jobs includes `{}` but missing_evidence does not contain retention_expired job entry",
                            id
                        ),
                        file: Some("bundle_manifest.json".to_string()),
                        path: Some("$.missing_evidence".to_string()),
                    });
                }
            }
            for id in &retention_diag_ids {
                if !missing_diag_retention.contains(id) {
                    findings.push(ValidationFinding {
                        severity: FindingSeverity::Error,
                        code: "VAL-BUNDLE-001:MISSING_EVIDENCE_MISMATCH".to_string(),
                        message: format!(
                            "retention_report.expired.diagnostics includes `{}` but missing_evidence does not contain retention_expired diagnostic entry",
                            id
                        ),
                        file: Some("bundle_manifest.json".to_string()),
                        path: Some("$.missing_evidence".to_string()),
                    });
                }
            }
        }

        // 4) Redaction compliance: SAFE_DEFAULT bundles must contain no secrets/PII/paths.
        if matches!(manifest.redaction_mode, RedactionMode::SafeDefault) {
            validate_safe_default_leaks(&contents, &mut findings);
        }
    }

    let valid = findings
        .iter()
        .all(|f| f.severity != FindingSeverity::Error);

    BundleValidationReport {
        valid,
        schema_version,
        findings,
    }
}

fn parse_json<T: serde::de::DeserializeOwned>(
    contents: &BundleContents,
    file: &str,
    findings: &mut Vec<ValidationFinding>,
    code: &str,
) -> Option<T> {
    let bytes = contents.bytes.get(file)?;
    match serde_json::from_slice::<T>(bytes) {
        Ok(value) => Some(value),
        Err(err) => {
            findings.push(ValidationFinding {
                severity: FindingSeverity::Error,
                code: code.to_string(),
                message: format!("{} parse error: {}", file, err),
                file: Some(file.to_string()),
                path: None,
            });
            None
        }
    }
}

fn parse_jsonl<T: serde::de::DeserializeOwned>(
    contents: &BundleContents,
    file: &str,
    findings: &mut Vec<ValidationFinding>,
    code: &str,
) -> Option<Vec<T>> {
    let bytes = contents.bytes.get(file)?;
    let text = String::from_utf8_lossy(bytes);
    let mut out: Vec<T> = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(trimmed) {
            Ok(value) => out.push(value),
            Err(err) => {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: code.to_string(),
                    message: format!("{} parse error on line {}: {}", file, idx + 1, err),
                    file: Some(file.to_string()),
                    path: Some(format!("line:{}", idx + 1)),
                });
                return None;
            }
        }
    }
    Some(out)
}

fn parse_jsonl_value(
    contents: &BundleContents,
    file: &str,
    findings: &mut Vec<ValidationFinding>,
) -> Option<Vec<serde_json::Value>> {
    parse_jsonl::<serde_json::Value>(contents, file, findings, "VAL-BUNDLE-001:PARSE_ERROR")
}

const UUID_RE: &str =
    r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}";

fn extract_ids_with_label(content: &str, label: &str) -> Vec<String> {
    let pattern = format!(r"(?m){}\s*:\s*`?({})`?", regex::escape(label), UUID_RE);
    let Ok(label_re) = Regex::new(&pattern) else {
        return Vec::new();
    };
    label_re
        .captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn line_after_label<'a>(content: &'a str, label: &str) -> Option<&'a str> {
    content
        .lines()
        .find(|line| line.contains(label) && line.contains(':'))
}

fn extract_uuids(content: &str) -> Vec<String> {
    let Ok(re) = Regex::new(UUID_RE) else {
        return Vec::new();
    };
    re.find_iter(content)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn validate_safe_default_leaks(contents: &BundleContents, findings: &mut Vec<ValidationFinding>) {
    let mut leak_patterns: Vec<(&str, Regex)> = Vec::new();
    if let Ok(re) = Regex::new(r#"(?m)([A-Z]:\\[^\s"']+|/(Users|home)/[^\s"']+)"#) {
        leak_patterns.push(("path_absolute", re));
    }
    if let Ok(re) =
        Regex::new(r"(?i)(sk-[a-z0-9]{10,}|api_[a-z0-9]{10,}|bearer\s+[a-z0-9\-\._~\+\/]+=*)")
    {
        leak_patterns.push(("secret_api_key", re));
    }
    if let Ok(re) = Regex::new(r"(\$[A-Z][A-Z0-9_]*|%[A-Z0-9_]+%|\$\{[A-Z0-9_]+\})") {
        leak_patterns.push(("env_var", re));
    }
    if let Ok(re) =
        Regex::new(r"(?i)(AKIA[0-9A-Z]{16}|aws_secret_access_key\s*[:=]\s*[A-Za-z0-9/+=]{20,})")
    {
        leak_patterns.push(("secret_aws", re));
    }
    if let Ok(re) = Regex::new(r"(?i)(postgres|mysql|mongodb|redis)://[^\s]+") {
        leak_patterns.push(("secret_db_url", re));
    }
    if let Ok(re) =
        Regex::new(r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----")
    {
        leak_patterns.push(("secret_private_key", re));
    }
    if let Ok(re) = Regex::new(r"(?i)(password|passwd)\s*[:=]\s*\S+") {
        leak_patterns.push(("secret_password", re));
    }
    if let Ok(re) = Regex::new(r"[A-Za-z0-9-_]{20,}\.[A-Za-z0-9-_]{10,}\.[A-Za-z0-9-_]{10,}") {
        leak_patterns.push(("secret_token", re));
    }
    if let Ok(re) = Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}") {
        leak_patterns.push(("pii_email", re));
    }
    if let Ok(re) = Regex::new(r"\+?\d[\d\-()]{0,4}\s[\d\s\-()]{6,}\d") {
        leak_patterns.push(("pii_phone", re));
    }

    for (file, bytes) in &contents.bytes {
        if file.ends_with(".zip") {
            continue;
        }
        let content = String::from_utf8_lossy(bytes);
        for (detector_id, regex) in &leak_patterns {
            if regex.is_match(&content) {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-REDACT-001".to_string(),
                    message: format!(
                        "SAFE_DEFAULT redaction violation: detector `{}` matched in `{}`",
                        detector_id, file
                    ),
                    file: Some(file.to_string()),
                    path: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn val_bundle_001_reports_missing_files() -> Result<(), BundleExportError> {
        let dir = tempdir()?;
        // create only one required file
        std::fs::write(dir.path().join("bundle_manifest.json"), "{}")?;
        let validator = ValBundleValidator;
        let report = validator.validate_dir(dir.path())?;
        assert!(
            !report.valid,
            "expected validator to report missing files for incomplete bundle"
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code.starts_with("VAL-BUNDLE-001")),
            "expected VAL-BUNDLE-001 finding"
        );
        Ok(())
    }
}
