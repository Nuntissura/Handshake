//! MT-035: Stdout/Stderr log capture (bounded).
//!
//! Acceptance: logs never live only in terminal output. Captured logs are
//! stored as `Kb003ArtifactClass::SandboxLog` artifacts with a strict
//! per-stream byte cap; truncation is recorded explicitly so the
//! reconstruction side cannot pretend it has the full stream when it does
//! not.

use serde::{Deserialize, Serialize};

use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;

/// Default per-stream cap: 256 KiB. Keeps logs review-friendly and bounds
/// SQLite/Postgres row size.
pub const DEFAULT_LOG_CAPTURE_BYTES_PER_STREAM: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedStream {
    pub kind: StreamKind,
    pub bytes: Vec<u8>,
    pub truncated_at_bytes: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogCaptureArtifact {
    pub artifact_class: Kb003ArtifactClass,
    pub max_bytes_per_stream: usize,
    pub stdout: CapturedStream,
    pub stderr: CapturedStream,
}

impl LogCaptureArtifact {
    /// Capture stdout/stderr with a per-stream byte cap.
    pub fn capture(stdout: &[u8], stderr: &[u8]) -> Self {
        Self::capture_with_cap(stdout, stderr, DEFAULT_LOG_CAPTURE_BYTES_PER_STREAM)
    }

    pub fn capture_with_cap(stdout: &[u8], stderr: &[u8], cap_bytes: usize) -> Self {
        Self {
            artifact_class: Kb003ArtifactClass::SandboxLog,
            max_bytes_per_stream: cap_bytes,
            stdout: cap_stream(StreamKind::Stdout, stdout, cap_bytes),
            stderr: cap_stream(StreamKind::Stderr, stderr, cap_bytes),
        }
    }

    /// Whether either stream was truncated during capture.
    pub fn any_truncated(&self) -> bool {
        self.stdout.truncated_at_bytes.is_some() || self.stderr.truncated_at_bytes.is_some()
    }
}

fn cap_stream(kind: StreamKind, raw: &[u8], cap: usize) -> CapturedStream {
    if raw.len() <= cap {
        CapturedStream {
            kind,
            bytes: raw.to_vec(),
            truncated_at_bytes: None,
        }
    } else {
        CapturedStream {
            kind,
            bytes: raw[..cap].to_vec(),
            truncated_at_bytes: Some(cap),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_streams_pass_through_uncut() {
        let cap = LogCaptureArtifact::capture(b"hello\n", b"err\n");
        assert_eq!(cap.stdout.bytes, b"hello\n");
        assert_eq!(cap.stderr.bytes, b"err\n");
        assert!(!cap.any_truncated());
    }

    #[test]
    fn over_cap_truncated_with_marker() {
        let big = vec![b'x'; 1024];
        let cap = LogCaptureArtifact::capture_with_cap(&big, &big, 100);
        assert_eq!(cap.stdout.bytes.len(), 100);
        assert_eq!(cap.stdout.truncated_at_bytes, Some(100));
        assert!(cap.any_truncated());
        assert_eq!(cap.artifact_class, Kb003ArtifactClass::SandboxLog);
    }

    #[test]
    fn logs_stored_not_only_in_terminal() {
        // The acceptance criterion is structural: we own bytes + class.
        let cap = LogCaptureArtifact::capture(b"out", b"err");
        // Persisted byte-form is non-empty and class-tagged for storage.
        assert!(!cap.stdout.bytes.is_empty());
        assert_eq!(cap.artifact_class, Kb003ArtifactClass::SandboxLog);
    }
}
