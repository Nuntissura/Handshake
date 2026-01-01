use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Clone, Debug)]
pub struct RedactionResult {
    pub redacted: String,
    pub matched: bool,
}

pub trait SecretRedactor: Send + Sync {
    fn redact_command(&self, command: &str) -> RedactionResult;
    fn redact_output(&self, stdout: &[u8], stderr: &[u8]) -> RedactionResult;
}

pub struct PatternRedactor;

static REDACTION_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    let patterns = [
        r#"(?i)(api[_-]?key|secret|token|password)\s*=\s*[^\s"]+"#,
        r#"(?i)bearer\s+[A-Za-z0-9\.\-_]+"#,
        r#"(?i)[A-Z0-9_]{3,}\s*=\s*[^\s"]+"#,
        r#"(?i)(aws|gcp|azure)_[A-Z0-9_]+\s*=\s*[^\s"]+"#,
    ];
    patterns
        .iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect()
});

fn apply_patterns(text: &str) -> RedactionResult {
    let mut matched = false;
    let mut redacted = text.to_string();
    for pattern in REDACTION_PATTERNS.iter() {
        if pattern.is_match(&redacted) {
            matched = true;
            redacted = pattern.replace_all(&redacted, "***REDACTED***").to_string();
        }
    }
    RedactionResult { redacted, matched }
}

impl SecretRedactor for PatternRedactor {
    fn redact_command(&self, command: &str) -> RedactionResult {
        apply_patterns(command)
    }

    fn redact_output(&self, stdout: &[u8], stderr: &[u8]) -> RedactionResult {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(stdout),
            String::from_utf8_lossy(stderr)
        );
        apply_patterns(&combined)
    }
}
