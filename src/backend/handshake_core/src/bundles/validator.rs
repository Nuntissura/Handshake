use std::fs;
use std::path::Path;

use regex::Regex;

use crate::bundles::exporter::{BundleExportError, FindingSeverity, ValidationFinding};
use crate::bundles::schemas::BundleManifest;

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
        let mut findings = Vec::new();
        let required = vec![
            "bundle_manifest.json",
            "env.json",
            "jobs.json",
            "trace.jsonl",
            "diagnostics.jsonl",
            "retention_report.json",
            "redaction_report.json",
            "repro.md",
            "coder_prompt.md",
        ];

        for required_file in &required {
            let candidate = path.join(required_file);
            if !candidate.exists() {
                findings.push(ValidationFinding {
                    severity: FindingSeverity::Error,
                    code: "VAL-BUNDLE-001:MISSING".to_string(),
                    message: format!("Missing required file {}", required_file),
                    file: Some(required_file.to_string()),
                    path: None,
                });
            }
        }

        let manifest_path = path.join("bundle_manifest.json");
        let manifest = if manifest_path.exists() {
            match fs::read_to_string(&manifest_path) {
                Ok(contents) => serde_json::from_str::<BundleManifest>(&contents).ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        let schema_version = manifest
            .as_ref()
            .map(|m| m.schema_version.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Minimal SAFE_DEFAULT leakage check: scan for absolute paths or obvious secrets.
        let mut leak_patterns = Vec::new();
        if let Ok(re) = Regex::new(r"(?m)([A-Z]:\\[^\\s]+|/[^\\s]+)") {
            leak_patterns.push(("path", re));
        }
        if let Ok(re) = Regex::new(r"(?i)sk-[a-z0-9]{10,}") {
            leak_patterns.push(("api_key", re));
        }

        for name in &["env.json", "diagnostics.jsonl"] {
            let candidate = path.join(name);
            if let Ok(content) = fs::read_to_string(&candidate) {
                for (cat, regex) in &leak_patterns {
                    if regex.is_match(&content) {
                        findings.push(ValidationFinding {
                            severity: FindingSeverity::Error,
                            code: "VAL-REDACT-001".to_string(),
                            message: format!("Detected potential {} leak in {}", cat, name),
                            file: Some(name.to_string()),
                            path: None,
                        });
                    }
                }
            }
        }

        let valid = findings
            .iter()
            .all(|f| f.severity != FindingSeverity::Error);

        Ok(BundleValidationReport {
            valid,
            schema_version,
            findings,
        })
    }

    pub fn validate_zip(&self, _path: &Path) -> Result<BundleValidationReport, BundleExportError> {
        // For now, instruct callers to unpack; future work could stream-validate ZIPs.
        Err(BundleExportError::Validation(
            "ZIP validation not implemented".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn val_bundle_001_reports_missing_files() {
        let dir = tempdir().expect("tmp dir");
        // create only one required file
        std::fs::write(dir.path().join("bundle_manifest.json"), "{}").expect("write manifest");
        let report = ValBundleValidator::default()
            .validate_dir(dir.path())
            .expect("validator");
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
    }
}
