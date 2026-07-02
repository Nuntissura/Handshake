//! Canonical-JSON SHA-256 of a `content_json` value — the LOAD-BEARING draft-hash seam
//! (WP-KERNEL-012 MT-020, MC-005).
//!
//! ## Why this is NOT `serde_json::to_vec`
//!
//! The MT contract body proposed computing `base_content_sha256` over "compact
//! serde_json::to_vec, deterministic field order". That is WRONG against the real backend
//! (the MT-011 hsLink lesson: verify the real backend, never trust the contract wording).
//! The backend computes the document `content_sha256` as
//! `sha256_hex(canonical_json_bytes(content_json))` (verified READ-ONLY in
//! `src/backend/handshake_core/src/storage/knowledge.rs::knowledge_canonical_json_sha256`
//! -> `kernel/context_bundle.rs::{canonical_json_bytes, sha256_hex}`), where the CANONICAL
//! form is:
//! - object keys sorted lexicographically (ascending by Rust `String` ord),
//! - NO whitespace between tokens (`{"a":1,"b":[2,3]}`),
//! - numbers rendered by `serde_json::Number::to_string`,
//! - strings escape ONLY `"`, `\`, `\n`, `\r`, `\t` (NOT `/`, `<`, `>`, or non-ASCII —
//!   those pass through verbatim).
//!
//! The draft `base_content_sha256` MUST equal this exact value or the backend's draft
//! endpoint rejects the upsert with HTTP 409 ("draft base content hash does not match the
//! current document" — verified in `upsert_document_draft`). So this module ports the
//! backend's canonical writer byte-for-byte and proves byte-equality in the unit tests.
//!
//! ## Why a hand-rolled hex encoder (no `hex` crate)
//!
//! The backend uses `hex::encode`; the frontend crate does not carry `hex`. The output is a
//! plain lowercase hex string, so a 4-line encoder is preferable to pulling a new dependency
//! family. The `sha256` digest itself uses the already-present `sha2` crate (a workspace dep).

use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

/// The canonical SHA-256 hex (lowercase) of a `content_json` value, byte-identical to the
/// backend's `knowledge_canonical_json_sha256`. This is the value sent as
/// `base_content_sha256` in a draft upsert (MC-005), so it MUST match the backend's stored
/// `content_sha256` for the SAME doc node or the upsert 409s.
pub fn canonical_content_sha256(content_json: &JsonValue) -> String {
    sha256_hex(&canonical_json_bytes(content_json))
}

/// SHA-256 over `bytes`, rendered as a lowercase hex string (mirrors the backend's
/// `sha256_hex` = `hex::encode(Sha256::digest(bytes))`).
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    encode_hex(&hasher.finalize())
}

/// Lowercase-hex encode (the same output as `hex::encode`): each byte -> two hex chars.
fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// The canonical JSON byte encoding of `value` — a DIRECT port of the backend's
/// `canonical_json_bytes` / `write_canonical_json` (verified read-only). Object keys are
/// sorted, no whitespace is emitted, and only the five backend-escaped chars are escaped.
pub fn canonical_json_bytes(value: &JsonValue) -> Vec<u8> {
    let mut output = String::new();
    write_canonical_json(&mut output, value);
    output.into_bytes()
}

/// Recursively write the canonical encoding of `value` into `output`. Byte-for-byte mirror of
/// the backend writer (any divergence here would silently break draft hashing — the unit
/// tests pin the exact output for objects, arrays, escapes, and nested docs).
fn write_canonical_json(output: &mut String, value: &JsonValue) {
    match value {
        JsonValue::Null => output.push_str("null"),
        JsonValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => output.push_str(&n.to_string()),
        JsonValue::String(s) => write_canonical_string(output, s),
        JsonValue::Array(items) => {
            output.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    output.push(',');
                }
                write_canonical_json(output, item);
            }
            output.push(']');
        }
        JsonValue::Object(map) => {
            output.push('{');
            // Keys sorted ascending — the backend collects `map.keys()` then `keys.sort()`,
            // which is the default `String` lexicographic (byte) order.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    output.push(',');
                }
                write_canonical_string(output, key);
                output.push(':');
                if let Some(v) = map.get(*key) {
                    write_canonical_json(output, v);
                }
            }
            output.push('}');
        }
    }
}

/// Write a JSON string token with the backend's EXACT escape set: only `"`, `\`, `\n`, `\r`,
/// `\t` are escaped; every other char (including `/`, `<`, control chars beyond the four, and
/// non-ASCII) passes through verbatim. This deliberately does NOT match `serde_json`'s escaping
/// (which also escapes other control chars) because the HASH must match the backend, not serde.
fn write_canonical_string(output: &mut String, s: &str) {
    output.push('"');
    for ch in s.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other => output.push(other),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_keys_are_sorted_with_no_whitespace() {
        // The backend sorts keys + emits no whitespace, so `{b,a}` canonicalizes to a<b order.
        let v = json!({ "b": 1, "a": 2, "c": [3, 4] });
        let bytes = canonical_json_bytes(&v);
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            r#"{"a":2,"b":1,"c":[3,4]}"#
        );
    }

    #[test]
    fn string_escapes_match_backend_exactly() {
        // ONLY the five chars are escaped; `/` and `<` pass through (unlike serde_json's HTML-safe
        // or full-control escaping). This is what makes the hash match the backend.
        let v = json!({ "k": "a\"b\\c\nd\re\tf/g<h" });
        let bytes = canonical_json_bytes(&v);
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\"k\":\"a\\\"b\\\\c\\nd\\re\\tf/g<h\"}"
        );
    }

    #[test]
    fn double_serialize_is_byte_identical() {
        // RISK-5 / red-team control: canonicalizing the same value twice yields identical bytes
        // (deterministic — no key-order drift between calls).
        let v = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"hi"}]}]});
        assert_eq!(canonical_json_bytes(&v), canonical_json_bytes(&v));
        assert_eq!(canonical_content_sha256(&v), canonical_content_sha256(&v));
    }

    #[test]
    fn sha256_is_lowercase_hex_64_chars() {
        let v = json!({});
        let h = canonical_content_sha256(&v);
        assert_eq!(h.len(), 64, "sha256 hex is 64 chars");
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        // Known vector: sha256 of the canonical empty object "{}" (2 bytes).
        // Independently: echo -n '{}' | sha256sum -> 44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a
        assert_eq!(
            h,
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }

    #[test]
    fn nested_object_within_array_sorts_each_level() {
        let v = json!({ "z": [ { "y": 1, "x": 2 } ], "a": 3 });
        let bytes = canonical_json_bytes(&v);
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            r#"{"a":3,"z":[{"x":2,"y":1}]}"#
        );
    }
}
