//! MT-084 ContentHashStrategy: canonical content hashing for ingestion.
//!
//! DECISION (documented per MT-084): two hashes, two jobs.
//!
//! 1. `raw_sha256` — SHA-256 over the EXACT bytes read from the source.
//!    This is the FIDELITY hash: it is what `knowledge_sources.content_hash`
//!    stores (0132 CHECK: lowercase 64-hex), what extraction receipts pin
//!    (`content hash at extraction time`), and what moved-file detection
//!    compares (MT-093). Raw bytes are hashed as-is — no normalization —
//!    because authority evidence must reflect what was actually on disk.
//!
//! 2. `normalized_text_sha256` — SHA-256 over a normalized TEXT projection:
//!    UTF-8 BOM stripped, CRLF and lone CR normalized to LF. Only computed
//!    for text-decodable content (valid UTF-8, no NUL byte). This is the
//!    CHANGE-DETECTION hash: a Windows/Unix line-ending flip or a BOM churn
//!    does not invalidate spans or force re-extraction, and equivalent text
//!    at different paths is detectable as a duplicate. It is never stored as
//!    `content_hash` authority.
//!
//! Hash primitive: SHA-256 via the existing `ai_ready_data::chunking::
//! sha256_hex` helper (single hashing authority in the crate — no duplicate
//! digest implementation here).

use crate::ai_ready_data::chunking::sha256_hex;
use serde::{Deserialize, Serialize};

/// Canonical content hashes for one ingested payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentHashes {
    /// SHA-256 (lowercase hex) over the exact raw bytes. Fidelity/authority.
    pub raw_sha256: String,
    /// SHA-256 over the normalized text projection; `None` for non-text.
    pub normalized_text_sha256: Option<String>,
    /// Whether the payload decoded as text (valid UTF-8, no NUL byte).
    pub is_text: bool,
}

/// Detect text-decodability: valid UTF-8 and free of NUL bytes.
pub fn decode_text(bytes: &[u8]) -> Option<&str> {
    if bytes.contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

/// Normalized text projection: strip UTF-8 BOM, CRLF/CR -> LF.
pub fn normalize_text_for_hashing(text: &str) -> String {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

/// Compute the canonical hash pair for a payload.
pub fn compute_content_hashes(bytes: &[u8]) -> ContentHashes {
    let raw_sha256 = sha256_hex(bytes);
    match decode_text(bytes) {
        Some(text) => {
            let normalized = normalize_text_for_hashing(text);
            ContentHashes {
                raw_sha256,
                normalized_text_sha256: Some(sha256_hex(normalized.as_bytes())),
                is_text: true,
            }
        }
        None => ContentHashes {
            raw_sha256,
            normalized_text_sha256: None,
            is_text: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_hash_is_exact_bytes_no_normalization() {
        let crlf = compute_content_hashes(b"line one\r\nline two\r\n");
        let lf = compute_content_hashes(b"line one\nline two\n");
        assert_ne!(
            crlf.raw_sha256, lf.raw_sha256,
            "fidelity hash must distinguish raw byte differences"
        );
        assert!(
            crlf.raw_sha256.len() == 64
                && crlf
                    .raw_sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn normalized_hash_unifies_line_endings_and_bom() {
        let crlf = compute_content_hashes(b"line one\r\nline two\r\n");
        let lf = compute_content_hashes(b"line one\nline two\n");
        let cr = compute_content_hashes(b"line one\rline two\r");
        let bom = compute_content_hashes("\u{feff}line one\nline two\n".as_bytes());
        assert_eq!(crlf.normalized_text_sha256, lf.normalized_text_sha256);
        assert_eq!(cr.normalized_text_sha256, lf.normalized_text_sha256);
        assert_eq!(bom.normalized_text_sha256, lf.normalized_text_sha256);
        assert!(lf.is_text);
    }

    #[test]
    fn binary_payloads_get_no_text_hash() {
        let binary = compute_content_hashes(&[0x00, 0xff, 0x13, 0x37]);
        assert!(!binary.is_text);
        assert!(binary.normalized_text_sha256.is_none());
        // Invalid UTF-8 is binary too.
        let invalid = compute_content_hashes(&[0xc3, 0x28]);
        assert!(!invalid.is_text);
    }

    #[test]
    fn identical_content_hashes_identically_regardless_of_origin() {
        let a = compute_content_hashes(b"same content");
        let b = compute_content_hashes(b"same content");
        assert_eq!(a, b, "dedupe detection rests on content-only hashing");
    }
}
