//! Ingestion span model: the typed extraction output every extractor
//! (MT-086..MT-090) produces and `knowledge_ingestion_spans` (0163) stores.
//!
//! Spec 2.3.13.11: "`KnowledgeSpan`: a byte, text, AST, media-time, page,
//! cell, or rich-document range anchored to a KnowledgeSource. A span is the
//! minimum citeable evidence unit". The ingestion-side span rows carry the
//! anchor (typed per kind), the stored content (post-redaction — raw secret
//! bytes never land here, MT-091), a verifiability hash of that stored
//! content, and link candidates for later graph work (MT-090 wikilinks).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{IngestionError, IngestionResult};

/// Typed anchor of a span inside its source.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "anchor_kind", rename_all = "snake_case")]
pub enum SpanAnchor {
    ByteRange {
        byte_start: u64,
        byte_end: u64,
    },
    LineRange {
        line_start: u32,
        line_end: u32,
        /// Heading context for markdown/text sections (may be empty).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        heading_path: Vec<String>,
    },
    PdfPage {
        /// 1-based page number.
        page: u32,
    },
    MediaTime {
        start_ms: u64,
        end_ms: u64,
        /// Cue index in the transcript artifact (0-based).
        cue_index: u32,
    },
    JsonPointer {
        /// RFC 6901 JSON pointer into the artifact.
        pointer: String,
        /// JSONL line (0-based) when the artifact is line-delimited.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        jsonl_line: Option<u32>,
        /// Stored content was truncated to the span size cap.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
    },
    HeadingPath {
        path: Vec<String>,
        line_start: u32,
        line_end: u32,
    },
}

impl SpanAnchor {
    /// String form matching the `knowledge_ingestion_spans.anchor_kind`
    /// CHECK constraint.
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::ByteRange { .. } => "byte_range",
            Self::LineRange { .. } => "line_range",
            Self::PdfPage { .. } => "pdf_page",
            Self::MediaTime { .. } => "media_time",
            Self::JsonPointer { .. } => "json_pointer",
            Self::HeadingPath { .. } => "heading_path",
        }
    }

    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({}))
    }

    pub fn from_json(value: &Value) -> IngestionResult<Self> {
        serde_json::from_value(value.clone())
            .map_err(|err| IngestionError::Validation(format!("invalid span anchor JSON: {err}")))
    }
}

/// A wikilink (or similar) reference found inside a span: recorded as a link
/// CANDIDATE for later graph/edge work, never resolved at ingestion time.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkCandidate {
    /// Raw matched syntax, e.g. `[[Target Note|label]]`.
    pub raw: String,
    /// Extracted target, e.g. `Target Note`.
    pub target: String,
    /// Optional display label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Redaction state of one stored span.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpanRedaction {
    None,
    Redacted,
}

impl SpanRedaction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Redacted => "redacted",
        }
    }
}

impl std::str::FromStr for SpanRedaction {
    type Err = IngestionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "redacted" => Ok(Self::Redacted),
            other => Err(IngestionError::Validation(format!(
                "invalid span redaction state: {other}"
            ))),
        }
    }
}

/// One extracted span, ready for persistence.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractedSpan {
    pub anchor: SpanAnchor,
    /// Raw byte offsets in the source payload, when meaningful.
    pub byte_start: Option<i64>,
    pub byte_end: Option<i64>,
    /// Stored content. Post-redaction: secret regions are
    /// `[REDACTED:<kind>]` markers, never raw bytes (MT-091).
    pub content: String,
    pub redaction: SpanRedaction,
    pub link_candidates: Vec<LinkCandidate>,
}

impl ExtractedSpan {
    pub fn new(anchor: SpanAnchor, content: impl Into<String>) -> Self {
        Self {
            anchor,
            byte_start: None,
            byte_end: None,
            content: content.into(),
            redaction: SpanRedaction::None,
            link_candidates: Vec::new(),
        }
    }

    pub fn with_bytes(mut self, byte_start: i64, byte_end: i64) -> Self {
        self.byte_start = Some(byte_start);
        self.byte_end = Some(byte_end);
        self
    }
}

/// Detect `[[wikilink]]` / `[[target|label]]` candidates in text (MT-090).
pub fn detect_wikilinks(text: &str) -> Vec<LinkCandidate> {
    let mut candidates = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else {
            break;
        };
        let inner = &after[..end];
        // Wikilinks never nest: a stray `[[` inside the candidate region
        // means THIS opener is malformed — rescan from the inner opener so a
        // well-formed link after it (e.g. `[[Nested[[x]]` -> `x`) survives.
        if inner.contains("[[") {
            rest = after;
            continue;
        }
        // Wikilinks never span lines.
        if !inner.is_empty() && !inner.contains('\n') {
            let (target, label) = match inner.split_once('|') {
                Some((t, l)) => (t.trim(), Some(l.trim().to_string())),
                None => (inner.trim(), None),
            };
            if !target.is_empty() {
                candidates.push(LinkCandidate {
                    raw: format!("[[{inner}]]"),
                    target: target.to_string(),
                    label: label.filter(|l| !l.is_empty()),
                });
            }
        }
        rest = &after[end + 2..];
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_round_trip_through_json() {
        let anchors = [
            SpanAnchor::ByteRange {
                byte_start: 0,
                byte_end: 10,
            },
            SpanAnchor::LineRange {
                line_start: 1,
                line_end: 4,
                heading_path: vec!["Intro".into()],
            },
            SpanAnchor::PdfPage { page: 3 },
            SpanAnchor::MediaTime {
                start_ms: 1000,
                end_ms: 4000,
                cue_index: 0,
            },
            SpanAnchor::JsonPointer {
                pointer: "/scope/constraints/0".into(),
                jsonl_line: Some(2),
                truncated: false,
            },
            SpanAnchor::HeadingPath {
                path: vec!["A".into(), "B".into()],
                line_start: 10,
                line_end: 20,
            },
        ];
        for anchor in anchors {
            let value = anchor.to_json();
            assert_eq!(value["anchor_kind"], anchor.kind_str());
            let back = SpanAnchor::from_json(&value).expect("round trip");
            assert_eq!(back, anchor);
        }
    }

    #[test]
    fn wikilink_detection_handles_labels_and_malformed_input() {
        let text = "See [[Project Roadmap]] and [[WP-009|the work packet]].\nBroken [[unclosed and [[Nested[[x]] stays sane.";
        let links = detect_wikilinks(text);
        assert_eq!(links.len(), 3, "{links:?}");
        assert_eq!(links[0].target, "Project Roadmap");
        assert_eq!(links[0].label, None);
        assert_eq!(links[1].target, "WP-009");
        assert_eq!(links[1].label.as_deref(), Some("the work packet"));
        assert_eq!(links[2].target, "x");
        assert!(detect_wikilinks("no links here").is_empty());
        assert!(detect_wikilinks("[[]]").is_empty());
        assert!(detect_wikilinks("[[a\nb]]").is_empty());
    }
}
