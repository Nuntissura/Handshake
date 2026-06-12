//! WP-KERNEL-009 MT-135 EvidenceSnippetAssembler.
//!
//! Spec 2.3.13.11 (KnowledgeSpan is "the minimum citeable evidence unit";
//! claims "MUST carry evidence spans") + 2.6.6.7.14.4 (an evidence item is "a
//! bounded excerpt backed by one or more SourceRefs"). Assemble evidence
//! snippets that carry: the source path, the span range, the span content hash,
//! the extraction-receipt EventLedger id, and an explicit UNSUPPORTED-CLAIM
//! marker when a candidate cites a claim that has no backing evidence span.
//!
//! An unsupported claim is not silently dropped — it is surfaced with
//! `supported = false` so a downstream consumer (and the operator) can see a
//! citation that the index cannot back. This is the anti-hallucination contract:
//! every snippet states whether the index can prove it.
//!
//! Reads go through the committed `KnowledgeStore` (spans, claims, sources).

use serde::{Deserialize, Serialize};

use crate::storage::knowledge::KnowledgeStore;
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

/// A single assembled evidence snippet (a projection; authority stays in the
/// span/claim/source rows it cites).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSnippet {
    /// The span this snippet is anchored to (the citeable evidence unit).
    pub span_id: String,
    /// The source the span belongs to.
    pub source_id: String,
    /// Human/model-citeable source path (relative path of the source), when set.
    pub source_path: Option<String>,
    /// Span range within the source.
    pub range_start: i64,
    pub range_end: i64,
    /// Optional line range for code/text spans.
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    /// The span content hash — lets a consumer detect drift against the source.
    pub content_sha256: String,
    /// The bounded display snippet text, when the span recorded one.
    pub excerpt: Option<String>,
    /// The EventLedger extraction receipt that produced the span.
    pub extraction_receipt_event_id: Option<String>,
    /// True when this snippet backs a real claim with evidence; false marks an
    /// unsupported claim (citation the index cannot prove).
    pub supported: bool,
    /// When `supported == false`, why (the unsupported-claim marker).
    pub unsupported_reason: Option<String>,
}

/// Assemble an evidence snippet for a span id. Loads the span + its source so
/// the snippet carries the source path and content hash. A missing span yields
/// an explicit unsupported marker rather than an error, so a citation to a span
/// the index no longer holds is visible.
pub async fn assemble_span_snippet(
    db: &PostgresDatabase,
    span_id: &str,
) -> StorageResult<EvidenceSnippet> {
    let Some(span) = db.get_knowledge_span(span_id).await? else {
        return Ok(EvidenceSnippet {
            span_id: span_id.to_string(),
            source_id: String::new(),
            source_path: None,
            range_start: 0,
            range_end: 0,
            line_start: None,
            line_end: None,
            content_sha256: String::new(),
            excerpt: None,
            extraction_receipt_event_id: None,
            supported: false,
            unsupported_reason: Some("span not found in index".to_string()),
        });
    };

    // Adversarial-v2 MT-135 LOW: a span whose SOURCE is gone is not a
    // supported citation. The schema makes this state unrepresentable
    // (knowledge_spans.source_id is ON DELETE CASCADE, so deleting the source
    // deletes the span and the span-missing path above fires) — this branch is
    // the defensive belt under that FK suspender.
    let (source_path, supported, unsupported_reason) =
        match db.get_knowledge_source(&span.source_id).await? {
            Some(source) => (source.relative_path, true, None),
            None => (
                None,
                false,
                Some("the span's source no longer exists in the index".to_string()),
            ),
        };

    Ok(EvidenceSnippet {
        span_id: span.span_id,
        source_id: span.source_id,
        source_path,
        range_start: span.range_start,
        range_end: span.range_end,
        line_start: span.line_start,
        line_end: span.line_end,
        content_sha256: span.content_sha256,
        excerpt: span.display_snippet,
        extraction_receipt_event_id: span.extraction_receipt_event_id,
        supported,
        unsupported_reason,
    })
}

/// Assemble the evidence snippets that back a claim. A claim with NO evidence
/// spans yields a single unsupported marker snippet (the anti-hallucination
/// contract): the claim is surfaced but flagged as something the index cannot
/// prove.
pub async fn assemble_claim_snippets(
    db: &PostgresDatabase,
    claim_id: &str,
) -> StorageResult<Vec<EvidenceSnippet>> {
    let Some(_claim) = db.get_knowledge_claim(claim_id).await? else {
        return Ok(vec![EvidenceSnippet {
            span_id: String::new(),
            source_id: String::new(),
            source_path: None,
            range_start: 0,
            range_end: 0,
            line_start: None,
            line_end: None,
            content_sha256: String::new(),
            excerpt: None,
            extraction_receipt_event_id: None,
            supported: false,
            unsupported_reason: Some(format!("claim {claim_id} not found in index")),
        }]);
    };

    let span_ids = db.list_knowledge_claim_span_ids(claim_id).await?;
    if span_ids.is_empty() {
        return Ok(vec![EvidenceSnippet {
            span_id: String::new(),
            source_id: String::new(),
            source_path: None,
            range_start: 0,
            range_end: 0,
            line_start: None,
            line_end: None,
            content_sha256: String::new(),
            excerpt: None,
            extraction_receipt_event_id: None,
            supported: false,
            unsupported_reason: Some(
                "claim has no backing evidence spans (unsupported claim)".to_string(),
            ),
        }]);
    }

    let mut snippets = Vec::with_capacity(span_ids.len());
    for span_id in span_ids {
        snippets.push(assemble_span_snippet(db, &span_id).await?);
    }
    Ok(snippets)
}

impl EvidenceSnippet {
    /// A stable citation string for an admitted snippet:
    /// `path:line_start-line_end@sha8` or `source:span@sha8` when no path/lines.
    pub fn citation(&self) -> String {
        let location = match (&self.source_path, self.line_start, self.line_end) {
            (Some(path), Some(ls), Some(le)) => format!("{path}:{ls}-{le}"),
            (Some(path), _, _) => format!("{path}@{}-{}", self.range_start, self.range_end),
            (None, _, _) => format!("{}#{}", self.source_id, self.span_id),
        };
        let hash_prefix = if self.content_sha256.len() >= 8 {
            &self.content_sha256[..8]
        } else {
            self.content_sha256.as_str()
        };
        if self.supported {
            format!("{location}@{hash_prefix}")
        } else {
            format!("{location}@UNSUPPORTED")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(supported: bool, path: Option<&str>) -> EvidenceSnippet {
        EvidenceSnippet {
            span_id: "SPAN-1".to_string(),
            source_id: "SRC-1".to_string(),
            source_path: path.map(ToString::to_string),
            range_start: 10,
            range_end: 42,
            line_start: Some(3),
            line_end: Some(5),
            content_sha256: "abcdef0123456789".to_string(),
            excerpt: Some("fn foo() {}".to_string()),
            extraction_receipt_event_id: Some("EVT-1".to_string()),
            supported,
            unsupported_reason: if supported {
                None
            } else {
                Some("no spans".to_string())
            },
        }
    }

    #[test]
    fn supported_citation_includes_path_lines_and_hash() {
        let c = snippet(true, Some("src/lib.rs")).citation();
        assert_eq!(c, "src/lib.rs:3-5@abcdef01");
    }

    #[test]
    fn unsupported_citation_is_marked() {
        let c = snippet(false, Some("src/lib.rs")).citation();
        assert!(c.ends_with("@UNSUPPORTED"));
    }

    #[test]
    fn citation_without_path_uses_source_and_span() {
        let c = snippet(true, None).citation();
        assert!(c.starts_with("SRC-1#SPAN-1"));
    }
}
