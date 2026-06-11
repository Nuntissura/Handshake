//! WP-KERNEL-009 MT-105 ScipLspImportBoundary.
//!
//! Master Spec anchor: 2.3.13.11 "LSP/SCIP-style precision is a Handshake-owned
//! import/projection path, NOT an outside service requirement".
//!
//! This is the OWNED import format for SCIP/LSIF-style index data. Handshake
//! never spawns an LSP server or a SCIP indexer; an operator (or an external
//! offline tool) produces an index artifact, and this boundary PARSES that
//! provided artifact into a typed [`ScipDocument`] that the engine projects
//! into knowledge records (entities + spans + edges through
//! `storage::knowledge::KnowledgeStore`). The import is gated/optional: the
//! Tree-sitter core (MT-097..MT-104) does not depend on it.
//!
//! Input format (Handshake-owned, deliberately small and JSON-based — we do not
//! take a protobuf dependency just to import an index, and a non-engineer
//! operator can hand-author or transcode into it): an artifact is a JSON object
//!
//! ```json
//! {
//!   "format": "scip",
//!   "tool": { "name": "scip-rust", "version": "0.3.0" },
//!   "documents": [
//!     {
//!       "relative_path": "src/lib.rs",
//!       "language": "rust",
//!       "symbols": [
//!         { "symbol": "rust:src/lib.rs#alpha", "kind": "function",
//!           "display_name": "alpha", "line_start": 10, "line_end": 12,
//!           "byte_start": 120, "byte_end": 180 }
//!       ],
//!       "occurrences": [
//!         { "symbol": "rust:src/lib.rs#alpha", "role": "reference",
//!           "line_start": 40, "line_end": 40, "byte_start": 500, "byte_end": 505 }
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! Validation is strict and typed: an artifact that is not this shape is
//! REJECTED with a reason (recorded in the import ledger, 0171), never
//! silently dropped. This module performs no DB and no IO; it parses bytes into
//! the typed model and validates it.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The artifact format declared by a SCIP/LSIF import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScipFormat {
    Scip,
    Lsif,
}

/// The producing tool (provenance only; never executed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScipTool {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// Role of an occurrence (a reference or the definition site).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScipRole {
    Definition,
    Reference,
}

/// A symbol declared by an imported document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScipSymbol {
    /// Stable symbol id, used directly as the entity key.
    pub symbol: String,
    pub kind: String,
    pub display_name: String,
    pub line_start: u32,
    pub line_end: u32,
    pub byte_start: u64,
    pub byte_end: u64,
}

/// A symbol occurrence (definition or reference) at a source range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScipOccurrence {
    pub symbol: String,
    pub role: ScipRole,
    pub line_start: u32,
    pub line_end: u32,
    pub byte_start: u64,
    pub byte_end: u64,
}

/// One imported document (corresponds to a source file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScipDocument {
    pub relative_path: String,
    pub language: String,
    #[serde(default)]
    pub symbols: Vec<ScipSymbol>,
    #[serde(default)]
    pub occurrences: Vec<ScipOccurrence>,
}

/// A whole parsed import artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScipArtifact {
    pub format: ScipFormat,
    #[serde(default)]
    pub tool: Option<ScipTool>,
    pub documents: Vec<ScipDocument>,
}

impl ScipArtifact {
    /// Total declared symbols across documents.
    pub fn symbol_count(&self) -> usize {
        self.documents.iter().map(|d| d.symbols.len()).sum()
    }

    /// Total declared occurrences across documents.
    pub fn occurrence_count(&self) -> usize {
        self.documents.iter().map(|d| d.occurrences.len()).sum()
    }
}

/// A typed rejection of an import artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScipImportRejected {
    pub reason: String,
}

impl std::fmt::Display for ScipImportRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scip import rejected: {}", self.reason)
    }
}

/// sha256 hex of the artifact bytes (for the import ledger fidelity hash).
pub fn artifact_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// MT-105 resource caps. A provided import artifact is operator-supplied but
/// UNTRUSTED input; without caps a hostile/huge artifact would OOM the parse or
/// blow up the downstream entity/span/edge writes. These bounds reject such an
/// artifact with a typed reason (recorded in the import ledger) before any
/// allocation of the parsed model beyond the raw bytes.
pub const SCIP_MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
pub const SCIP_MAX_DOCUMENTS: usize = 100_000;
pub const SCIP_MAX_SYMBOLS: usize = 2_000_000;
pub const SCIP_MAX_OCCURRENCES: usize = 5_000_000;

/// Parse and validate an import artifact from bytes. Strict: anything that is
/// not the owned shape — or that exceeds the MT-105 resource caps, escapes the
/// repo with a `..`/`.` path segment, or references an undeclared symbol — is
/// `Err(ScipImportRejected)`.
pub fn parse_scip_artifact(bytes: &[u8]) -> Result<ScipArtifact, ScipImportRejected> {
    // Size cap FIRST, before deserialisation allocates the parsed model.
    if bytes.len() > SCIP_MAX_ARTIFACT_BYTES {
        return Err(ScipImportRejected {
            reason: format!(
                "artifact is {} bytes, exceeds the {SCIP_MAX_ARTIFACT_BYTES}-byte import cap",
                bytes.len()
            ),
        });
    }
    let artifact: ScipArtifact =
        serde_json::from_slice(bytes).map_err(|err| ScipImportRejected {
            reason: format!("artifact is not a valid Handshake SCIP/LSIF import JSON: {err}"),
        })?;
    validate(&artifact)?;
    Ok(artifact)
}

/// True when `path` is a repo-relative POSIX path that does not escape the repo:
/// no leading `/`, no backslash, and no `.`/`..` path segment (the traversal
/// vectors). An empty segment (`a//b`) is also rejected as malformed.
fn is_repo_relative_posix(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') {
        return false;
    }
    path.split('/')
        .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

fn validate(artifact: &ScipArtifact) -> Result<(), ScipImportRejected> {
    if artifact.documents.is_empty() {
        return Err(ScipImportRejected {
            reason: "artifact declares no documents".to_string(),
        });
    }
    // Count caps (the parse already bounded total bytes; these bound the work
    // the downstream projection would do).
    if artifact.documents.len() > SCIP_MAX_DOCUMENTS {
        return Err(ScipImportRejected {
            reason: format!(
                "artifact declares {} documents, exceeds the {SCIP_MAX_DOCUMENTS} cap",
                artifact.documents.len()
            ),
        });
    }
    let total_symbols = artifact.symbol_count();
    if total_symbols > SCIP_MAX_SYMBOLS {
        return Err(ScipImportRejected {
            reason: format!(
                "artifact declares {total_symbols} symbols, exceeds the {SCIP_MAX_SYMBOLS} cap"
            ),
        });
    }
    let total_occurrences = artifact.occurrence_count();
    if total_occurrences > SCIP_MAX_OCCURRENCES {
        return Err(ScipImportRejected {
            reason: format!(
                "artifact declares {total_occurrences} occurrences, exceeds the \
                 {SCIP_MAX_OCCURRENCES} cap"
            ),
        });
    }

    // Collect every symbol id DECLARED anywhere in the artifact so the
    // occurrence->declared-symbol invariant can be enforced (an occurrence that
    // references no declared symbol cannot become an edge and is rejected, not
    // silently dropped).
    let mut declared: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for doc in &artifact.documents {
        for sym in &doc.symbols {
            declared.insert(sym.symbol.as_str());
        }
    }

    for (di, doc) in artifact.documents.iter().enumerate() {
        if doc.relative_path.trim().is_empty() {
            return Err(ScipImportRejected {
                reason: format!("document #{di} has an empty relative_path"),
            });
        }
        if !is_repo_relative_posix(&doc.relative_path) {
            return Err(ScipImportRejected {
                reason: format!(
                    "document #{di} relative_path '{}' must be a repo-relative POSIX path with \
                     no '..'/'.' segment",
                    doc.relative_path
                ),
            });
        }
        for sym in &doc.symbols {
            if sym.symbol.trim().is_empty() || sym.display_name.trim().is_empty() {
                return Err(ScipImportRejected {
                    reason: format!(
                        "document '{}' has a symbol with empty symbol id or display_name",
                        doc.relative_path
                    ),
                });
            }
            if sym.byte_end < sym.byte_start {
                return Err(ScipImportRejected {
                    reason: format!("symbol '{}' has byte_end < byte_start", sym.symbol),
                });
            }
        }
        // Every occurrence must reference a symbol declared SOMEWHERE in the
        // artifact (occurrences without a declaration are not importable as
        // edges) and have a well-formed byte range.
        for occ in &doc.occurrences {
            if occ.byte_end < occ.byte_start {
                return Err(ScipImportRejected {
                    reason: format!("occurrence of '{}' has byte_end < byte_start", occ.symbol),
                });
            }
            if !declared.contains(occ.symbol.as_str()) {
                return Err(ScipImportRejected {
                    reason: format!(
                        "occurrence in '{}' references undeclared symbol '{}' (occurrence-to-\
                         declared-symbol invariant)",
                        doc.relative_path, occ.symbol
                    ),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{
      "format": "scip",
      "tool": { "name": "scip-rust", "version": "0.3.0" },
      "documents": [
        {
          "relative_path": "src/lib.rs",
          "language": "rust",
          "symbols": [
            { "symbol": "rust:src/lib.rs#alpha", "kind": "function",
              "display_name": "alpha", "line_start": 10, "line_end": 12,
              "byte_start": 120, "byte_end": 180 }
          ],
          "occurrences": [
            { "symbol": "rust:src/lib.rs#alpha", "role": "reference",
              "line_start": 40, "line_end": 40, "byte_start": 500, "byte_end": 505 }
          ]
        }
      ]
    }"#;

    #[test]
    fn parses_valid_artifact() {
        let artifact = parse_scip_artifact(GOOD.as_bytes()).expect("valid");
        assert_eq!(artifact.format, ScipFormat::Scip);
        assert_eq!(artifact.symbol_count(), 1);
        assert_eq!(artifact.occurrence_count(), 1);
        assert_eq!(artifact.tool.as_ref().unwrap().name, "scip-rust");
    }

    #[test]
    fn rejects_non_json() {
        let err = parse_scip_artifact(b"not json").unwrap_err();
        assert!(err.reason.contains("not a valid"), "{err}");
    }

    #[test]
    fn rejects_empty_documents() {
        let bytes = br#"{ "format": "scip", "documents": [] }"#;
        let err = parse_scip_artifact(bytes).unwrap_err();
        assert!(err.reason.contains("no documents"), "{err}");
    }

    #[test]
    fn rejects_absolute_path() {
        let bytes = br#"{ "format": "lsif", "documents": [
            { "relative_path": "/etc/passwd", "language": "rust" }
        ] }"#;
        let err = parse_scip_artifact(bytes).unwrap_err();
        assert!(err.reason.contains("repo-relative"), "{err}");
    }

    #[test]
    fn rejects_inverted_byte_range() {
        let bytes = br#"{ "format": "scip", "documents": [
            { "relative_path": "a.rs", "language": "rust", "symbols": [
                { "symbol": "x", "kind": "function", "display_name": "x",
                  "line_start": 1, "line_end": 1, "byte_start": 10, "byte_end": 5 }
            ] }
        ] }"#;
        let err = parse_scip_artifact(bytes).unwrap_err();
        assert!(err.reason.contains("byte_end < byte_start"), "{err}");
    }

    #[test]
    fn artifact_hash_is_deterministic() {
        assert_eq!(artifact_hash(b"abc"), artifact_hash(b"abc"));
        assert_ne!(artifact_hash(b"abc"), artifact_hash(b"abd"));
    }

    #[test]
    fn rejects_dot_dot_path_segment() {
        let bytes = br#"{ "format": "scip", "documents": [
            { "relative_path": "src/../../../etc/passwd", "language": "rust" }
        ] }"#;
        let err = parse_scip_artifact(bytes).unwrap_err();
        assert!(
            err.reason.contains("'..'/'.'") || err.reason.contains("repo-relative"),
            "{err}"
        );
    }

    #[test]
    fn rejects_single_dot_path_segment() {
        let bytes = br#"{ "format": "scip", "documents": [
            { "relative_path": "src/./lib.rs", "language": "rust" }
        ] }"#;
        let err = parse_scip_artifact(bytes).unwrap_err();
        assert!(err.reason.contains("'..'/'.'"), "{err}");
    }

    #[test]
    fn rejects_occurrence_referencing_undeclared_symbol() {
        // The occurrence points at a symbol that is declared NOWHERE in the
        // artifact -> rejected by the occurrence->declared-symbol invariant.
        let bytes = br#"{ "format": "scip", "documents": [
            { "relative_path": "a.rs", "language": "rust",
              "symbols": [
                { "symbol": "rust:a.rs#known", "kind": "function",
                  "display_name": "known", "line_start": 1, "line_end": 1,
                  "byte_start": 0, "byte_end": 10 }
              ],
              "occurrences": [
                { "symbol": "rust:a.rs#ghost", "role": "reference",
                  "line_start": 5, "line_end": 5, "byte_start": 50, "byte_end": 55 }
              ] }
        ] }"#;
        let err = parse_scip_artifact(bytes).unwrap_err();
        assert!(
            err.reason.contains("undeclared symbol") && err.reason.contains("ghost"),
            "{err}"
        );
    }

    #[test]
    fn accepts_occurrence_referencing_symbol_declared_in_other_document() {
        // Cross-document references are valid: the symbol is declared in doc A,
        // the occurrence is in doc B.
        let bytes = br#"{ "format": "scip", "documents": [
            { "relative_path": "a.rs", "language": "rust",
              "symbols": [
                { "symbol": "rust:a.rs#shared", "kind": "function",
                  "display_name": "shared", "line_start": 1, "line_end": 1,
                  "byte_start": 0, "byte_end": 10 }
              ] },
            { "relative_path": "b.rs", "language": "rust",
              "occurrences": [
                { "symbol": "rust:a.rs#shared", "role": "reference",
                  "line_start": 2, "line_end": 2, "byte_start": 20, "byte_end": 26 }
              ] }
        ] }"#;
        let artifact = parse_scip_artifact(bytes).expect("cross-doc reference is valid");
        assert_eq!(artifact.occurrence_count(), 1);
    }

    #[test]
    fn rejects_artifact_exceeding_byte_cap() {
        // A blob just over the byte cap is rejected before JSON parsing.
        let oversize = vec![b' '; SCIP_MAX_ARTIFACT_BYTES + 1];
        let err = parse_scip_artifact(&oversize).unwrap_err();
        assert!(err.reason.contains("import cap"), "{err}");
    }

    #[test]
    fn rejects_too_many_documents() {
        // Build an artifact whose document count exceeds the cap with tiny docs.
        let mut docs = String::new();
        for i in 0..(SCIP_MAX_DOCUMENTS + 1) {
            if i > 0 {
                docs.push(',');
            }
            docs.push_str(&format!(
                r#"{{ "relative_path": "f{i}.rs", "language": "rust" }}"#
            ));
        }
        let json = format!(r#"{{ "format": "scip", "documents": [{docs}] }}"#);
        // Guard: this synthetic artifact must still be under the byte cap so the
        // DOCUMENT cap (not the byte cap) is what fires.
        assert!(json.len() <= SCIP_MAX_ARTIFACT_BYTES);
        let err = parse_scip_artifact(json.as_bytes()).unwrap_err();
        assert!(
            err.reason.contains("documents") && err.reason.contains("cap"),
            "{err}"
        );
    }
}
