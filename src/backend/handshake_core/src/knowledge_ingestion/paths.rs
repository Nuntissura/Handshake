//! MT-083 PathPortabilityNormalizer: one normalization gate every ingested
//! path passes before it may appear in durable records.
//!
//! Builds ON the storage-layer normalizer
//! (`storage::knowledge::normalize_repo_relative_path`, mirrored by the
//! `chk_*_path_portable` DB constraints) and strengthens it for ingestion
//! input, which can carry operator-pasted Windows paths and arbitrary
//! filenames:
//!
//! * forward slashes only; backslashes are separators, normalized to `/`
//! * no drive letters (`C:`), rooted paths (`/x`, `\x`), or UNC (`//server`,
//!   `\\?\C:\...`) — machine-local anchoring is runtime configuration, never
//!   stored authority ([GLOBAL-PORTABILITY])
//! * no `..` escapes; `.` segments and empty segments (`a//b`, `a/./b`)
//!   collapse away; a leading `./` is stripped
//! * Unicode NFC so `é` (composed) and `e\u{301}` (decomposed) address the
//!   SAME source row regardless of which filesystem produced the name
//! * no control characters / NUL
//! * Windows-interop hardening: reserved device names (`CON`, `NUL`,
//!   `COM1`…) and trailing dots/spaces in segments are rejected — such paths
//!   cannot round-trip through a Windows checkout
//!
//! The output is idempotent: `normalize(normalize(x)) == normalize(x)`.

use unicode_normalization::{is_nfc, UnicodeNormalization};

use super::{IngestionError, IngestionResult};

/// Windows reserved device names (case-insensitive, with or without
/// extension): files with these names break Windows checkouts.
const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

fn validation(detail: impl Into<String>) -> IngestionError {
    IngestionError::Validation(detail.into())
}

/// Normalize a candidate source-relative path for durable ingestion records.
///
/// Returns the canonical repo/root-relative POSIX form, or a typed
/// validation error naming the violated rule. The empty string (the root
/// itself) is NOT a valid source path — sources are files.
pub fn normalize_source_relative_path(input: &str) -> IngestionResult<String> {
    if input.trim() != input {
        return Err(validation(
            "path must not carry surrounding whitespace".to_string(),
        ));
    }
    if input.is_empty() {
        return Err(validation("source path must not be empty"));
    }
    if input.chars().any(|c| c.is_control()) {
        return Err(validation(
            "path must not contain control characters".to_string(),
        ));
    }

    // Separator normalization first so all later checks see POSIX form.
    let slashed = input.replace('\\', "/");

    // Machine-local authority rejections.
    if slashed.starts_with("//") {
        return Err(validation(format!(
            "UNC/network path authority is forbidden: {input}"
        )));
    }
    if slashed.starts_with('/') {
        return Err(validation(format!(
            "rooted path authority is forbidden, paths must be root-relative: {input}"
        )));
    }
    let mut chars = slashed.chars();
    if let (Some(first), Some(':')) = (chars.next(), chars.next()) {
        if first.is_ascii_alphabetic() {
            return Err(validation(format!(
                "drive-letter path authority is forbidden: {input}"
            )));
        }
    }
    if slashed.contains(':') {
        // Also catches `\\?\C:\...` after separator normalization and NTFS
        // alternate data stream syntax (`file.txt:stream`).
        return Err(validation(format!(
            "':' is not portable in relative paths: {input}"
        )));
    }

    // Segment-wise normalization: drop `.` and empty segments, reject `..`,
    // reject Windows-breaking segment shapes.
    let mut segments: Vec<&str> = Vec::new();
    for segment in slashed.split('/') {
        match segment {
            "" | "." => continue,
            ".." => {
                return Err(validation(format!(
                    "parent-directory escape is forbidden: {input}"
                )))
            }
            _ => {
                if segment.ends_with('.') || segment.ends_with(' ') {
                    return Err(validation(format!(
                        "segment '{segment}' ends with a dot/space and cannot round-trip on Windows"
                    )));
                }
                let stem = segment.split('.').next().unwrap_or(segment);
                if WINDOWS_RESERVED.contains(&stem.to_ascii_lowercase().as_str()) {
                    return Err(validation(format!(
                        "segment '{segment}' is a reserved Windows device name"
                    )));
                }
                segments.push(segment);
            }
        }
    }
    if segments.is_empty() {
        return Err(validation(format!("path normalizes to nothing: {input}")));
    }

    let joined = segments.join("/");
    // Unicode NFC: one canonical byte form per visible name.
    let normalized = if is_nfc(&joined) {
        joined
    } else {
        joined.nfc().collect::<String>()
    };
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_separators_dot_segments_and_duplicate_slashes() {
        for (input, expected) in [
            ("src\\backend\\mod.rs", "src/backend/mod.rs"),
            ("./docs/readme.md", "docs/readme.md"),
            ("a//b///c.txt", "a/b/c.txt"),
            ("a/./b/.//c.rs", "a/b/c.rs"),
            ("mixed\\sep/path\\file.ts", "mixed/sep/path/file.ts"),
        ] {
            assert_eq!(
                normalize_source_relative_path(input).expect(input),
                expected
            );
        }
    }

    #[test]
    fn rejects_machine_local_authority_windows_edge_cases() {
        for bad in [
            "C:/projects/x.rs",
            "c:\\projects\\x.rs",
            "Z:relative.txt",
            "/var/data.txt",
            "\\rooted.txt",
            "//server/share/file.txt",
            "\\\\server\\share\\file.txt",
            "\\\\?\\C:\\long\\path.txt",
            "file.txt:ads_stream",
        ] {
            let err = normalize_source_relative_path(bad).expect_err(&format!("must reject {bad}"));
            assert!(matches!(err, IngestionError::Validation(_)));
        }
    }

    #[test]
    fn rejects_traversal_and_empty_shapes() {
        for bad in [
            "../up.txt",
            "a/../b.txt",
            "a/b/..",
            "..",
            ".",
            "./",
            "",
            " padded.txt",
        ] {
            assert!(
                normalize_source_relative_path(bad).is_err(),
                "must reject {bad:?}"
            );
        }
    }

    #[test]
    fn rejects_windows_breaking_segments() {
        for bad in [
            "con",
            "NUL.txt",
            "logs/com1.log",
            "dir/file.",
            "dir/file ",
            "aux/x.rs",
        ] {
            assert!(
                normalize_source_relative_path(bad).is_err(),
                "must reject {bad:?}"
            );
        }
        // Near-misses stay valid.
        for good in ["console.rs", "nulled.txt", "communication/x.rs"] {
            assert!(normalize_source_relative_path(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn unicode_nfc_makes_composed_and_decomposed_names_identical() {
        let composed = "docs/caf\u{e9}.md"; // café, NFC
        let decomposed = "docs/cafe\u{301}.md"; // cafe + combining acute, NFD
        let a = normalize_source_relative_path(composed).expect("composed");
        let b = normalize_source_relative_path(decomposed).expect("decomposed");
        assert_eq!(a, b, "NFC must unify equivalent names");
        assert_eq!(a, composed, "NFC form is the canonical one");
    }

    /// Property-style: normalization is idempotent and outputs always pass
    /// the storage-layer normalizer + DB-constraint shape.
    #[test]
    fn normalization_is_idempotent_and_storage_compatible() {
        let inputs = [
            "src\\a.rs",
            "./b/c.txt",
            "d//e.md",
            "docs/cafe\u{301}.md",
            "deep/nested/path/file.tsx",
            "UPPER/Case.SQL",
        ];
        for input in inputs {
            let once = normalize_source_relative_path(input).expect(input);
            let twice = normalize_source_relative_path(&once).expect(&once);
            assert_eq!(once, twice, "idempotence for {input}");
            // Storage layer accepts every output unchanged.
            let storage_form =
                crate::storage::knowledge::normalize_repo_relative_path(&once).expect(&once);
            assert_eq!(storage_form, once, "storage compatibility for {input}");
            // DB-constraint shape (chk_*_path_portable mirror).
            assert!(!once.starts_with('/'));
            assert!(!once.contains('\\'));
            assert!(!once.split('/').any(|s| s == ".."));
        }
    }
}
