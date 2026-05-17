//! MT-035: Stdout/Stderr log capture (bounded).
//!
//! Acceptance: logs never live only in terminal output. Captured logs are
//! stored as `Kb003ArtifactClass::SandboxLog` artifacts with a strict
//! per-stream byte cap; truncation is recorded explicitly so the
//! reconstruction side cannot pretend it has the full stream when it does
//! not.
//!
//! Streams are decoded as UTF-8 (with `from_utf8_lossy` fallback on invalid
//! input) and stored as `String`. Byte-cap truncation is rounded down to the
//! nearest UTF-8 character boundary so the stored text is always well-formed
//! UTF-8 — important because downstream JSON serialisation, terminal display,
//! and grep tooling all assume valid UTF-8.

use serde::{Deserialize, Serialize};

use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;

/// Default per-stream cap: 256 KiB. Keeps logs review-friendly and bounds
/// SQLite/Postgres row size.
pub const DEFAULT_LOG_CAPTURE_BYTES_PER_STREAM: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedStream {
    pub kind: StreamKind,
    /// Captured stream content as valid UTF-8 text. Invalid input is recovered
    /// via `String::from_utf8_lossy`. When truncation occurs at the cap, the
    /// boundary is rounded down to the nearest character boundary so the
    /// stored text remains valid UTF-8.
    pub text: String,
    /// Byte index at which truncation occurred (before character-boundary
    /// rounding). `None` when the stream fit under the cap.
    pub truncated_at_bytes: Option<usize>,
    /// Character (`char`) count at which truncation occurred. `None` when the
    /// stream fit under the cap.
    pub truncated_at_chars: Option<usize>,
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
    // Decode as UTF-8, falling back to lossy decoding on invalid input.
    let decoded: String = match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(raw).into_owned(),
    };

    if decoded.len() <= cap {
        return CapturedStream {
            kind,
            text: decoded,
            truncated_at_bytes: None,
            truncated_at_chars: None,
        };
    }

    // Round `cap` down to the nearest UTF-8 char boundary so the resulting
    // `String` is always valid UTF-8. (Manual scan kept for MSRV portability;
    // `str::floor_char_boundary` is unstable / 1.80+.)
    let boundary = floor_char_boundary(&decoded, cap);
    let truncated = decoded[..boundary].to_string();
    let char_count = truncated.chars().count();
    CapturedStream {
        kind,
        text: truncated,
        truncated_at_bytes: Some(cap),
        truncated_at_chars: Some(char_count),
    }
}

/// Largest index `<= cap` that lies on a UTF-8 char boundary within `s`.
/// Mirrors `str::floor_char_boundary` (unstable) but compiles on stable Rust
/// of any MSRV the workspace currently supports.
fn floor_char_boundary(s: &str, cap: usize) -> usize {
    if cap >= s.len() {
        return s.len();
    }
    let mut i = cap;
    let bytes = s.as_bytes();
    // UTF-8 continuation bytes have the top two bits set to `10` (0xxxxxxx is
    // a 1-byte char, 11xxxxxx is a leading byte). Scan backwards until we
    // find a leading byte.
    while i > 0 && (bytes[i] & 0xC0) == 0x80 {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_streams_pass_through_uncut() {
        let cap = LogCaptureArtifact::capture(b"hello\n", b"err\n");
        assert_eq!(cap.stdout.text, "hello\n");
        assert_eq!(cap.stderr.text, "err\n");
        assert!(!cap.any_truncated());
    }

    #[test]
    fn over_cap_truncated_with_marker() {
        let big = vec![b'x'; 1024];
        let cap = LogCaptureArtifact::capture_with_cap(&big, &big, 100);
        assert_eq!(cap.stdout.text.len(), 100);
        // ASCII -> char_count == byte_count.
        assert_eq!(cap.stdout.truncated_at_bytes, Some(100));
        assert_eq!(cap.stdout.truncated_at_chars, Some(100));
        assert!(cap.any_truncated());
        assert_eq!(cap.artifact_class, Kb003ArtifactClass::SandboxLog);
        // Text must be valid UTF-8 even when truncation lands inside what
        // could otherwise be a multibyte sequence.
        assert!(std::str::from_utf8(cap.stdout.text.as_bytes()).is_ok());
    }

    #[test]
    fn multibyte_input_truncates_cleanly() {
        // U+1F600 GRINNING FACE encodes to 4 UTF-8 bytes: F0 9F 98 80.
        // Build "ABC<EMOJI>" repeated so each group is 3 ASCII bytes + 4 emoji
        // bytes = 7 bytes per group.
        let group = format!("ABC{}", '\u{1F600}');
        assert_eq!(group.len(), 7, "test fixture must be 3+4 bytes");
        let s = group.repeat(8); // 56 bytes total
        let bytes = s.as_bytes();
        // cap=5 lands 2 bytes into the first emoji (bytes 3..7).
        // floor_char_boundary must round back to byte 3, giving just "ABC".
        let cap = 5usize;
        let captured = LogCaptureArtifact::capture_with_cap(bytes, b"", cap);
        assert!(std::str::from_utf8(captured.stdout.text.as_bytes()).is_ok());
        assert_eq!(captured.stdout.text, "ABC");
        assert_eq!(captured.stdout.truncated_at_bytes, Some(cap));
        assert_eq!(captured.stdout.truncated_at_chars, Some(3));
        assert!(captured.any_truncated());

        // Sanity: a 3-byte char (CJK) also rounds cleanly.
        // U+65E5 ("\u{65E5}") encodes to E6 97 A5.
        let jp = format!(
            "{}{}{}{}{}{}",
            '\u{65E5}', '\u{672C}', '\u{8A9E}', '\u{65E5}', '\u{672C}', '\u{8A9E}'
        );
        let jp_bytes = jp.as_bytes();
        assert_eq!(jp.len(), 18);
        // cap=4 lands mid second char (bytes 3..6).
        let captured2 = LogCaptureArtifact::capture_with_cap(jp_bytes, b"", 4);
        assert!(std::str::from_utf8(captured2.stdout.text.as_bytes()).is_ok());
        assert_eq!(captured2.stdout.text, format!("{}", '\u{65E5}'));
        assert_eq!(captured2.stdout.truncated_at_chars, Some(1));
    }

    #[test]
    fn invalid_utf8_input_recovers_via_lossy() {
        // Lone continuation byte 0xFF is invalid UTF-8.
        let raw = vec![b'A', 0xFF, b'B'];
        let captured = LogCaptureArtifact::capture(&raw, b"");
        assert!(std::str::from_utf8(captured.stdout.text.as_bytes()).is_ok());
        // U+FFFD REPLACEMENT CHARACTER is what `from_utf8_lossy` inserts.
        assert!(captured.stdout.text.contains('\u{FFFD}'));
    }

    #[test]
    fn logs_stored_not_only_in_terminal() {
        // The acceptance criterion is structural: we own text + class.
        let cap = LogCaptureArtifact::capture(b"out", b"err");
        assert!(!cap.stdout.text.is_empty());
        assert_eq!(cap.artifact_class, Kb003ArtifactClass::SandboxLog);
    }
}
