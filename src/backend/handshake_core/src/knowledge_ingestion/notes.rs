//! MT-090 OperatorResearchNoteIngestion: markdown/text notes into heading /
//! paragraph spans with `[[wikilink]]` candidates.
//!
//! Authority labelling (MT-090 contract): operator research notes are
//! CONTEXT, not normative authority — the engine stamps their provenance
//! `{"authority": "non_normative_context"}` unless a spec enrichment
//! promotes them. This module only does the structural split.
//!
//! Span strategy:
//! * The note splits into paragraph blocks (blank-line separated); fenced
//!   code blocks (``` ... ```) stay single blocks, never split inside.
//! * Each block becomes one span with a `line_range` anchor carrying the
//!   active heading path (e.g. `["Research", "PDF crates"]`), so a citation
//!   names both the lines and the section.
//! * Headings themselves are not separate spans; they live in the path.
//! * `[[target]]` / `[[target|label]]` occurrences are recorded per-span as
//!   link CANDIDATES for later graph work (never resolved here).

use super::spans::{detect_wikilinks, ExtractedSpan, SpanAnchor};

/// Parse outcome for one note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteParse {
    pub spans: Vec<ExtractedSpan>,
    /// Distinct heading count, for receipt detail.
    pub headings_seen: usize,
    /// Total link candidates across spans.
    pub link_candidates: usize,
}

fn heading_level(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.bytes().take_while(|b| *b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }
    Some((hashes, rest.trim()))
}

struct Block {
    line_start: u32,
    line_end: u32,
    heading_path: Vec<String>,
    text: String,
}

/// Parse a markdown/plain-text note into paragraph spans.
pub fn parse_note(text: &str) -> NoteParse {
    let mut heading_stack: Vec<(usize, String)> = Vec::new();
    let mut headings_seen = 0usize;
    let mut blocks: Vec<Block> = Vec::new();

    let mut current_lines: Vec<&str> = Vec::new();
    let mut current_start = 0u32;
    let mut in_fence = false;

    let flush = |lines: &mut Vec<&str>,
                 start: u32,
                 end: u32,
                 stack: &[(usize, String)],
                 out: &mut Vec<Block>| {
        if lines.is_empty() {
            return;
        }
        let body = lines.join("\n");
        lines.clear();
        if body.trim().is_empty() {
            return;
        }
        out.push(Block {
            line_start: start,
            line_end: end,
            heading_path: stack.iter().map(|(_, h)| h.clone()).collect(),
            text: body,
        });
    };

    for (idx, line) in text.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        let fence_toggle = line.trim_start().starts_with("```");

        if fence_toggle {
            if !in_fence && current_lines.is_empty() {
                current_start = line_no;
            }
            current_lines.push(line);
            if in_fence {
                // Closing fence ends the block.
                flush(
                    &mut current_lines,
                    current_start,
                    line_no,
                    &heading_stack,
                    &mut blocks,
                );
            }
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            current_lines.push(line);
            continue;
        }

        if let Some((level, title)) = heading_level(line) {
            flush(
                &mut current_lines,
                current_start,
                line_no.saturating_sub(1),
                &heading_stack,
                &mut blocks,
            );
            while matches!(heading_stack.last(), Some((l, _)) if *l >= level) {
                heading_stack.pop();
            }
            heading_stack.push((level, title.to_string()));
            headings_seen += 1;
            continue;
        }

        if line.trim().is_empty() {
            flush(
                &mut current_lines,
                current_start,
                line_no.saturating_sub(1),
                &heading_stack,
                &mut blocks,
            );
            continue;
        }

        if current_lines.is_empty() {
            current_start = line_no;
        }
        current_lines.push(line);
    }
    let last_line = text.lines().count() as u32;
    flush(
        &mut current_lines,
        current_start,
        last_line,
        &heading_stack,
        &mut blocks,
    );

    let mut link_candidates = 0usize;
    let spans = blocks
        .into_iter()
        .map(|block| {
            let links = detect_wikilinks(&block.text);
            link_candidates += links.len();
            let mut span = ExtractedSpan::new(
                SpanAnchor::LineRange {
                    line_start: block.line_start,
                    line_end: block.line_end,
                    heading_path: block.heading_path,
                },
                block.text,
            );
            span.link_candidates = links;
            span
        })
        .collect();

    NoteParse {
        spans,
        headings_seen,
        link_candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "Intro paragraph before any heading.\n\n# Research\n\nFirst finding references [[PDF crates]] and [[lopdf|the lopdf crate]].\n\nSecond paragraph.\n\n## Details\n\nNested detail paragraph.\n\n```rust\nlet code = \"# not a heading\";\n\nlet still_same_block = true;\n```\n\n# Conclusions\n\nFinal thought.\n";

    fn heading_path(span: &ExtractedSpan) -> Vec<String> {
        match &span.anchor {
            SpanAnchor::LineRange { heading_path, .. } => heading_path.clone(),
            other => panic!("unexpected anchor {other:?}"),
        }
    }

    #[test]
    fn paragraphs_carry_heading_paths() {
        let parse = parse_note(NOTE);
        assert_eq!(parse.headings_seen, 3);
        let texts: Vec<&str> = parse.spans.iter().map(|s| s.content.as_str()).collect();
        assert_eq!(texts[0], "Intro paragraph before any heading.");
        assert!(heading_path(&parse.spans[0]).is_empty());

        let first_finding = &parse.spans[1];
        assert!(first_finding.content.contains("First finding"));
        assert_eq!(heading_path(first_finding), vec!["Research".to_string()]);

        let nested = parse
            .spans
            .iter()
            .find(|s| s.content.contains("Nested detail"))
            .expect("nested paragraph span");
        assert_eq!(
            heading_path(nested),
            vec!["Research".to_string(), "Details".to_string()]
        );

        let conclusion = parse
            .spans
            .iter()
            .find(|s| s.content.contains("Final thought"))
            .expect("conclusion span");
        assert_eq!(heading_path(conclusion), vec!["Conclusions".to_string()]);
    }

    #[test]
    fn wikilinks_become_link_candidates() {
        let parse = parse_note(NOTE);
        assert_eq!(parse.link_candidates, 2);
        let span = parse
            .spans
            .iter()
            .find(|s| !s.link_candidates.is_empty())
            .expect("span with links");
        assert_eq!(span.link_candidates[0].target, "PDF crates");
        assert_eq!(span.link_candidates[1].target, "lopdf");
        assert_eq!(
            span.link_candidates[1].label.as_deref(),
            Some("the lopdf crate")
        );
    }

    #[test]
    fn fenced_code_blocks_stay_single_spans() {
        let parse = parse_note(NOTE);
        let code_span = parse
            .spans
            .iter()
            .find(|s| s.content.contains("still_same_block"))
            .expect("code block span");
        assert!(
            code_span.content.contains("# not a heading"),
            "heading-looking line inside fence stays in the block"
        );
        assert!(code_span.content.starts_with("```rust"));
        assert!(code_span.content.ends_with("```"));
        // The fake heading inside the fence never entered the path.
        let after = parse
            .spans
            .iter()
            .find(|s| s.content.contains("Final thought"))
            .expect("span after fence");
        assert_eq!(heading_path(after), vec!["Conclusions".to_string()]);
    }

    #[test]
    fn line_anchors_are_one_based_and_ordered() {
        let parse = parse_note("para one\n\npara two\n");
        match &parse.spans[0].anchor {
            SpanAnchor::LineRange {
                line_start,
                line_end,
                ..
            } => {
                assert_eq!(*line_start, 1);
                assert_eq!(*line_end, 1);
            }
            other => panic!("unexpected anchor {other:?}"),
        }
        match &parse.spans[1].anchor {
            SpanAnchor::LineRange { line_start, .. } => assert_eq!(*line_start, 3),
            other => panic!("unexpected anchor {other:?}"),
        }
    }

    #[test]
    fn empty_and_whitespace_notes_yield_no_spans() {
        assert!(parse_note("").spans.is_empty());
        assert!(parse_note("\n\n  \n").spans.is_empty());
    }
}
