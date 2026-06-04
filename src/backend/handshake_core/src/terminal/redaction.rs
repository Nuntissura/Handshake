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

    /// Redact a single output chunk (one stream, no stdout/stderr join). Used by
    /// the interactive PTY relay and the capture seam, where output arrives as a
    /// stream of byte slices and the `stdout\nstderr` framing of
    /// [`SecretRedactor::redact_output`] would inject a spurious trailing
    /// newline into every redacted chunk. Byte-faithful: a non-matching chunk is
    /// returned unchanged; a matching chunk has only the secret substituted, no
    /// added separators.
    fn redact_chunk(&self, chunk: &[u8]) -> RedactionResult {
        // Default: lossy-decode the single stream and pattern-redact it, with no
        // join. Implementations with a faster path may override.
        apply_patterns(&String::from_utf8_lossy(chunk))
    }
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

    fn redact_chunk(&self, chunk: &[u8]) -> RedactionResult {
        // Byte-faithful single-stream redaction: no stdout/stderr join, so a
        // redacted streaming chunk does not gain a spurious trailing newline.
        apply_patterns(&String::from_utf8_lossy(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_chunk_is_byte_faithful_for_non_matching_chunk() {
        // A chunk with no secret must round-trip with NO added separators (the
        // stdout\nstderr join of redact_output would have appended a newline).
        let chunk = b"plain interactive output line";
        let r = PatternRedactor.redact_chunk(chunk);
        assert!(!r.matched);
        assert_eq!(r.redacted.as_bytes(), chunk);
    }

    #[test]
    fn redact_chunk_no_trailing_newline_vs_redact_output() {
        // redact_output framing adds a trailing '\n'; redact_chunk must not.
        let chunk = b"hello world";
        let chunk_res = PatternRedactor.redact_chunk(chunk);
        let output_res = PatternRedactor.redact_output(chunk, &[]);
        assert_eq!(chunk_res.redacted, "hello world");
        assert_eq!(output_res.redacted, "hello world\n");
        assert!(
            !chunk_res.redacted.ends_with('\n'),
            "chunk redaction must not add a trailing newline"
        );
    }

    #[test]
    fn redact_chunk_still_strips_secrets() {
        let chunk = b"export API_KEY=supersecretvalue123 next";
        let r = PatternRedactor.redact_chunk(chunk);
        assert!(r.matched);
        assert!(r.redacted.contains("REDACTED"));
        assert!(!r.redacted.contains("supersecretvalue123"));
    }
}
