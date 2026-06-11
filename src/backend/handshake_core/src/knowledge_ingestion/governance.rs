//! MT-089 GovernanceArtifactIngestion: parse JSON/JSONL governance artifacts
//! (work packets, microtask contracts, receipts, registries) into structural
//! spans with RFC 6901 JSON-pointer anchors.
//!
//! This proves the product can index its OWN machine-readable artifacts: a
//! no-context model can later cite `task_packets/MT-081.json#/scope/
//! constraints/2` as evidence. At runtime the engine reads artifacts from a
//! registered governance root; tests use small synthetic fixtures (never the
//! live .GOV junction).
//!
//! Span strategy (bounded, deterministic):
//! * JSON object root: one span per top-level member (`/key`), and for
//!   members that are objects/arrays one more level (`/key/sub`,
//!   `/key/<index>`) up to [`GovernanceSpanLimits::max_depth`].
//! * JSONL: each line is its own document; pointers are relative to that
//!   line's root and the anchor records `jsonl_line`.
//! * Content is the pretty-printed JSON of the node, truncated at
//!   [`GovernanceSpanLimits::max_span_chars`] with the truncation FLAGGED on
//!   the anchor (`truncated: true`) — never silent.
//! * Total spans per artifact are capped (`max_spans`); the parse reports
//!   how many nodes were skipped so the receipt can record partial coverage.

use serde_json::Value;

use super::receipts::IngestionErrorClass;
use super::spans::{ExtractedSpan, SpanAnchor};

/// Bounds for structural span emission.
#[derive(Clone, Copy, Debug)]
pub struct GovernanceSpanLimits {
    /// Pointer depth below the document root to descend (1 = top-level
    /// members only, 2 = one nested level, ...).
    pub max_depth: u32,
    /// Hard cap on spans per artifact.
    pub max_spans: usize,
    /// Stored span content cap (chars); longer nodes are truncated with the
    /// anchor flagged.
    pub max_span_chars: usize,
}

impl Default for GovernanceSpanLimits {
    fn default() -> Self {
        Self {
            max_depth: 2,
            max_spans: 200,
            max_span_chars: 4_000,
        }
    }
}

/// Parse outcome for one governance artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovernanceParse {
    pub spans: Vec<ExtractedSpan>,
    /// Nodes skipped because `max_spans` was reached (partial coverage).
    pub skipped_nodes: usize,
}

/// Typed whole-file parse failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovernanceParseError {
    pub class: IngestionErrorClass,
    pub detail: String,
}

/// RFC 6901 token escaping: `~` -> `~0`, `/` -> `~1`.
fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn render_node(value: &Value, limits: &GovernanceSpanLimits) -> (String, bool) {
    let rendered = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    if rendered.chars().count() > limits.max_span_chars {
        let truncated: String = rendered.chars().take(limits.max_span_chars).collect();
        (format!("{truncated}\n…[truncated]"), true)
    } else {
        (rendered, false)
    }
}

fn emit_node(
    pointer: String,
    jsonl_line: Option<u32>,
    value: &Value,
    depth: u32,
    limits: &GovernanceSpanLimits,
    spans: &mut Vec<ExtractedSpan>,
    skipped: &mut usize,
) {
    if spans.len() >= limits.max_spans {
        *skipped += 1;
        return;
    }
    let (content, truncated) = render_node(value, limits);
    spans.push(ExtractedSpan::new(
        SpanAnchor::JsonPointer {
            pointer: pointer.clone(),
            jsonl_line,
            truncated,
        },
        content,
    ));

    if depth >= limits.max_depth {
        return;
    }
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                emit_node(
                    format!("{pointer}/{}", escape_pointer_token(key)),
                    jsonl_line,
                    child,
                    depth + 1,
                    limits,
                    spans,
                    skipped,
                );
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                emit_node(
                    format!("{pointer}/{index}"),
                    jsonl_line,
                    child,
                    depth + 1,
                    limits,
                    spans,
                    skipped,
                );
            }
        }
        _ => {}
    }
}

/// Parse a `.json` governance artifact into structural spans.
pub fn parse_governance_json(
    text: &str,
    limits: &GovernanceSpanLimits,
) -> Result<GovernanceParse, GovernanceParseError> {
    let value: Value = serde_json::from_str(text).map_err(|err| GovernanceParseError {
        class: IngestionErrorClass::ParseError,
        detail: format!("invalid governance JSON: {err}"),
    })?;

    let mut spans = Vec::new();
    let mut skipped = 0usize;
    match &value {
        Value::Object(map) => {
            for (key, child) in map {
                emit_node(
                    format!("/{}", escape_pointer_token(key)),
                    None,
                    child,
                    1,
                    limits,
                    &mut spans,
                    &mut skipped,
                );
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                emit_node(
                    format!("/{index}"),
                    None,
                    child,
                    1,
                    limits,
                    &mut spans,
                    &mut skipped,
                );
            }
        }
        // A bare scalar document is one whole-document span.
        _ => emit_node(
            String::new(),
            None,
            &value,
            limits.max_depth,
            limits,
            &mut spans,
            &mut skipped,
        ),
    }
    Ok(GovernanceParse {
        spans,
        skipped_nodes: skipped,
    })
}

/// Parse a `.jsonl` governance artifact: one document per non-empty line.
/// Malformed lines are skipped and counted (partial coverage), mirroring the
/// transcript malformed-cue policy.
pub fn parse_governance_jsonl(
    text: &str,
    limits: &GovernanceSpanLimits,
) -> Result<GovernanceParse, GovernanceParseError> {
    let mut spans = Vec::new();
    let mut skipped = 0usize;
    let mut malformed_lines = 0usize;
    let mut total_lines = 0usize;

    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        total_lines += 1;
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            malformed_lines += 1;
            continue;
        };
        emit_node(
            String::new(),
            Some(idx as u32),
            &value,
            // JSONL rows are usually small records: span the whole row only.
            limits.max_depth,
            limits,
            &mut spans,
            &mut skipped,
        );
    }

    if total_lines == 0 {
        return Err(GovernanceParseError {
            class: IngestionErrorClass::ParseError,
            detail: "empty JSONL artifact".to_string(),
        });
    }
    if spans.is_empty() {
        return Err(GovernanceParseError {
            class: IngestionErrorClass::ParseError,
            detail: format!("no parseable JSONL line ({malformed_lines} malformed)"),
        });
    }
    Ok(GovernanceParse {
        spans,
        skipped_nodes: skipped + malformed_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_object_yields_pointer_spans_two_levels_deep() {
        let artifact = r#"{
            "mt_id": "MT-089",
            "scope": {"constraints": ["a", "b"], "title": "x"},
            "tags": ["one", "two"]
        }"#;
        let parse =
            parse_governance_json(artifact, &GovernanceSpanLimits::default()).expect("parse");
        let pointers: Vec<&str> = parse
            .spans
            .iter()
            .map(|s| match &s.anchor {
                SpanAnchor::JsonPointer { pointer, .. } => pointer.as_str(),
                other => panic!("unexpected anchor {other:?}"),
            })
            .collect();
        for expected in [
            "/mt_id",
            "/scope",
            "/scope/constraints",
            "/scope/title",
            "/tags",
            "/tags/0",
            "/tags/1",
        ] {
            assert!(
                pointers.contains(&expected),
                "missing {expected}: {pointers:?}"
            );
        }
        assert_eq!(parse.skipped_nodes, 0);
        // Span content is the actual node JSON.
        let mt_id_span = parse
            .spans
            .iter()
            .find(|s| matches!(&s.anchor, SpanAnchor::JsonPointer { pointer, .. } if pointer == "/mt_id"))
            .expect("mt_id span");
        assert_eq!(mt_id_span.content, "\"MT-089\"");
    }

    #[test]
    fn pointer_tokens_escape_rfc6901_special_chars() {
        let artifact = r#"{"a/b": 1, "c~d": 2}"#;
        let parse =
            parse_governance_json(artifact, &GovernanceSpanLimits::default()).expect("parse");
        let pointers: Vec<String> = parse
            .spans
            .iter()
            .map(|s| match &s.anchor {
                SpanAnchor::JsonPointer { pointer, .. } => pointer.clone(),
                other => panic!("unexpected anchor {other:?}"),
            })
            .collect();
        assert!(pointers.contains(&"/a~1b".to_string()), "{pointers:?}");
        assert!(pointers.contains(&"/c~0d".to_string()), "{pointers:?}");
    }

    #[test]
    fn span_cap_and_truncation_are_explicit() {
        let limits = GovernanceSpanLimits {
            max_depth: 1,
            max_spans: 2,
            max_span_chars: 10,
        };
        let artifact = r#"{"a": "0123456789ABCDEF", "b": 2, "c": 3}"#;
        let parse = parse_governance_json(artifact, &limits).expect("parse");
        assert_eq!(parse.spans.len(), 2, "span cap respected");
        assert_eq!(parse.skipped_nodes, 1, "skipped nodes counted");
        match &parse.spans[0].anchor {
            SpanAnchor::JsonPointer { truncated, .. } => {
                assert!(*truncated, "long node must flag truncation")
            }
            other => panic!("unexpected anchor {other:?}"),
        }
        assert!(parse.spans[0].content.ends_with("…[truncated]"));
    }

    #[test]
    fn jsonl_spans_carry_line_numbers_and_survive_malformed_lines() {
        let artifact = "{\"receipt\": 1}\nnot json at all\n\n{\"receipt\": 2}\n";
        let parse =
            parse_governance_jsonl(artifact, &GovernanceSpanLimits::default()).expect("parse");
        assert_eq!(parse.spans.len(), 2);
        assert_eq!(parse.skipped_nodes, 1, "malformed line counted");
        match &parse.spans[1].anchor {
            SpanAnchor::JsonPointer { jsonl_line, .. } => assert_eq!(*jsonl_line, Some(3)),
            other => panic!("unexpected anchor {other:?}"),
        }
    }

    #[test]
    fn malformed_artifacts_fail_typed() {
        let err = parse_governance_json("{broken", &GovernanceSpanLimits::default())
            .expect_err("invalid JSON");
        assert_eq!(err.class, IngestionErrorClass::ParseError);
        let err =
            parse_governance_jsonl("", &GovernanceSpanLimits::default()).expect_err("empty JSONL");
        assert_eq!(err.class, IngestionErrorClass::ParseError);
        let err = parse_governance_jsonl("nope\nstill nope\n", &GovernanceSpanLimits::default())
            .expect_err("all lines malformed");
        assert_eq!(err.class, IngestionErrorClass::ParseError);
    }
}
