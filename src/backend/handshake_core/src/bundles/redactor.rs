use chrono::Utc;
use regex::Regex;
use serde_json::Value;

use crate::bundles::schemas::{
    PolicyDecision, RedactionDetector, RedactionLogEntry, RedactionMode, RedactionReport,
    RedactionSummary,
};

#[derive(Debug, Clone)]
pub struct SecretPattern {
    pub id: String,
    pub category: String,
    pub regex: Regex,
}

#[derive(Debug, Clone, Default)]
pub struct SecretRedactor {
    patterns: Vec<SecretPattern>,
}

impl SecretRedactor {
    pub fn new() -> Self {
        let mut patterns = Vec::new();
        // Helper to push compiled regex while ignoring invalid patterns gracefully.
        let add = |id: &str, category: &str, expr: &str, vec: &mut Vec<SecretPattern>| {
            if let Ok(regex) = Regex::new(expr) {
                vec.push(SecretPattern {
                    id: id.to_string(),
                    category: category.to_string(),
                    regex,
                });
            }
        };

        add(
            "secret_api_key",
            "api_key",
            r"(?i)sk-[a-z0-9]{10,}",
            &mut patterns,
        );
        add("secret_aws", "aws", r"AKIA[0-9A-Z]{16}", &mut patterns);
        add(
            "secret_db_url",
            "db_url",
            r"(?i)(postgres|mysql|mongodb|redis)://[^\s]+",
            &mut patterns,
        );
        add(
            "secret_private_key",
            "private_key",
            r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----",
            &mut patterns,
        );
        add(
            "secret_password",
            "password",
            r"(?i)password\s*[:=]\s*[^\s]+",
            &mut patterns,
        );
        add(
            "secret_token",
            "token",
            r"[A-Za-z0-9-_]{20,}\.[A-Za-z0-9-_]{10,}\.[A-Za-z0-9-_]{10,}",
            &mut patterns,
        );
        add(
            "pii_email",
            "pii",
            r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
            &mut patterns,
        );
        add("pii_phone", "pii", r"\+?\d[\d\s\-()]{8,}", &mut patterns);
        add(
            "path_absolute",
            "path",
            r"([A-Z]:\\[^\s]+|/[^\s]+)",
            &mut patterns,
        );
        add(
            "env_var",
            "env",
            r"(\$[A-Z][A-Z0-9_]*|%[A-Z0-9_]+%|\$\{[A-Z0-9_]+\})",
            &mut patterns,
        );

        Self { patterns }
    }

    /// Redact a JSON value, returning the redacted value and the log entries.
    pub fn redact_value(
        &self,
        value: &Value,
        mode: RedactionMode,
        location: &str,
    ) -> (Value, Vec<RedactionLogEntry>) {
        match value {
            Value::String(s) => self.redact_string(s, mode, location),
            Value::Array(items) => {
                let mut logs = Vec::new();
                let mut out = Vec::new();
                for (idx, item) in items.iter().enumerate() {
                    let path = format!("{}/{}", location, idx);
                    let (redacted, sublogs) = self.redact_value(item, mode, &path);
                    logs.extend(sublogs);
                    out.push(redacted);
                }
                (Value::Array(out), logs)
            }
            Value::Object(map) => {
                let mut logs = Vec::new();
                let mut out = serde_json::Map::new();
                for (k, v) in map {
                    let path = format!("{}/{}", location, k);
                    let (redacted, sublogs) = self.redact_value(v, mode, &path);
                    logs.extend(sublogs);
                    out.insert(k.clone(), redacted);
                }
                (Value::Object(out), logs)
            }
            other => (other.clone(), Vec::new()),
        }
    }

    fn redact_string(
        &self,
        input: &str,
        mode: RedactionMode,
        location: &str,
    ) -> (Value, Vec<RedactionLogEntry>) {
        // For FULL_LOCAL we do not redact unless the content hits secret patterns; for Workspace/SafeDefault we redact paths and secrets.
        let mut redacted = input.to_string();
        let mut logs = Vec::new();

        for pattern in &self.patterns {
            let should_apply = match mode {
                RedactionMode::SafeDefault => true,
                RedactionMode::Workspace => {
                    pattern.category != "path" || pattern.id == "path_absolute"
                }
                RedactionMode::FullLocal => pattern.category != "path", // allow local paths in full local
            };

            if !should_apply {
                continue;
            }

            let replacement = format!("[REDACTED:{}:{}]", pattern.category, pattern.id);
            if pattern.regex.is_match(&redacted) {
                redacted = pattern
                    .regex
                    .replace_all(&redacted, replacement.as_str())
                    .into_owned();
                logs.push(RedactionLogEntry {
                    file: "".to_string(),
                    location: location.to_string(),
                    category: pattern.category.clone(),
                    detector_id: pattern.id.clone(),
                    replacement,
                });
            }
        }

        (Value::String(redacted), logs)
    }

    pub fn build_report(
        &self,
        logs: &[RedactionLogEntry],
        mode: RedactionMode,
        files_scanned: u32,
    ) -> RedactionReport {
        let mut by_category = serde_json::Map::new();
        for log in logs {
            let counter = by_category
                .entry(log.category.clone())
                .or_insert_with(|| Value::Number(0u64.into()));
            if let Some(n) = counter.as_u64() {
                *counter = Value::Number((n + 1).into());
            }
        }

        let detectors = self
            .patterns
            .iter()
            .map(|p| RedactionDetector {
                id: p.id.clone(),
                version: "1".to_string(),
                patterns_count: 1,
            })
            .collect();

        RedactionReport {
            redaction_mode: mode,
            report_generated_at: Utc::now(),
            detectors,
            summary: RedactionSummary {
                files_scanned,
                files_redacted: if logs.is_empty() { 0 } else { files_scanned },
                total_redactions: logs.len() as u32,
                by_category,
            },
            redactions: logs.to_vec(),
            policy_decisions: Vec::new(),
        }
    }

    pub fn policy_decision_allow_all() -> Vec<PolicyDecision> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redaction_removes_api_keys() {
        let redactor = SecretRedactor::new();
        let (value, logs) = redactor.redact_value(
            &json!("sk-abc123def456ghi789"),
            RedactionMode::SafeDefault,
            "$",
        );
        let rendered = value.as_str().unwrap_or_default().to_string();
        assert!(
            rendered.contains("[REDACTED:api_key:secret_api_key]"),
            "expected api key to be redacted, got {rendered}"
        );
        assert!(!logs.is_empty(), "expected redaction logs to be captured");
    }
}
