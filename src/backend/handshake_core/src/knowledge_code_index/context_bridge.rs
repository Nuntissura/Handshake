//! WP-KERNEL-009 MT-110 CodeContextBundleBridge.
//!
//! Master Spec anchor: 2.3.13.11 (knowledge index feeds bounded, cited context)
//! and the kernel ContextBundle authority (`src/kernel/context_bundle.rs`). This
//! module projects a code symbol's NEIGHBORHOOD — its definition, doc comment,
//! the symbols it calls, and the symbols that call it — into a bounded,
//! citation-bearing `allowed_context` JSON value and wraps it in a kernel
//! [`ContextBundle`]. Every fact carries its source span id + line range, so the
//! bundle is auditable (no uncited code text), and the neighborhood is capped by
//! a token budget so a large fan-out cannot blow a model's context window.
//!
//! Pure projection over the indexed graph (entities/spans/edges through
//! `KnowledgeStore` + the code-file index state for staleness). It NEVER
//! re-parses and writes no durable rows itself: the [`ContextBundle`] it returns
//! is the kernel's existing durable artifact, persisted by the caller through
//! the established context-bundle path. Token accounting is a deterministic
//! character-based estimate (chars/4, the conventional rough token ratio); it is
//! a guardrail, not a tokenizer, and is recorded in the bundle so the caller can
//! see the budget decision.

use serde_json::{json, Value};

use crate::kernel::context_bundle::ContextBundle;
use crate::storage::knowledge::{
    KnowledgeEdgeType, KnowledgeEntity, KnowledgeEntityKind, KnowledgeSpan, KnowledgeStore,
};
use crate::storage::postgres::PostgresDatabase;

use super::staleness::{evaluate_staleness, IndexedState, LiveSourceState, StalenessVerdict};
use super::{CodeIndexError, CodeIndexResult};

/// Default token budget for a code context bundle. A neighborhood that would
/// exceed this is truncated (deterministically, nearest neighbors first) and the
/// bundle records that it was capped.
pub const DEFAULT_CODE_CONTEXT_TOKEN_BUDGET: u32 = 2_000;

/// Rough chars-per-token ratio used for the deterministic budget estimate. This
/// is intentionally conservative (real tokenizers average ~3.5-4 chars/token for
/// code); 4 keeps the estimate stable and host-independent.
const CHARS_PER_TOKEN: usize = 4;

/// A single cited neighbor in the bundle (a related symbol + its evidence).
#[derive(Debug, Clone, PartialEq, Eq)]
struct CitedNeighbor {
    relation: &'static str,
    symbol_key: String,
    display_name: String,
    span_id: Option<String>,
    line_start: i32,
    line_end: i32,
    estimated_tokens: usize,
}

impl CitedNeighbor {
    fn to_json(&self) -> Value {
        json!({
            "relation": self.relation,
            "symbol_key": self.symbol_key,
            "display_name": self.display_name,
            "span_id": self.span_id,
            "line_start": self.line_start,
            "line_end": self.line_end,
            "estimated_tokens": self.estimated_tokens,
        })
    }
}

/// Estimate the tokens a piece of text contributes (deterministic guardrail).
fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(CHARS_PER_TOKEN)
}

/// Build a bounded, cited code context bundle for one symbol, identified by its
/// stable `symbol_key` (`{lang}:{path}#{symbol_path}`). The neighborhood is:
/// the symbol's own definition + doc, the symbols it CALLS (outgoing
/// `references` edges), and the symbols that CALL it (incoming `references`).
/// The result is wrapped in a kernel [`ContextBundle`] keyed by the caller's
/// run/session ids.
///
/// `current_content_hash` / `current_parser_version` describe the live source so
/// the bundle can flag staleness (a stale neighborhood is labeled, never served
/// as fresh). `token_budget` caps the neighborhood; pass
/// [`DEFAULT_CODE_CONTEXT_TOKEN_BUDGET`] for the default.
#[allow(clippy::too_many_arguments)]
pub async fn build_code_context_bundle(
    db: &PostgresDatabase,
    kernel_task_run_id: &str,
    session_run_id: &str,
    workspace_id: &str,
    relative_path: &str,
    symbol_key: &str,
    current_content_hash: &str,
    current_parser_version: &str,
    token_budget: u32,
) -> CodeIndexResult<ContextBundle> {
    // Resolve the focus symbol.
    let focus = db
        .get_knowledge_entity_by_identity(workspace_id, KnowledgeEntityKind::Symbol, symbol_key)
        .await?
        .ok_or_else(|| {
            CodeIndexError::Validation(format!("symbol '{symbol_key}' is not indexed"))
        })?;

    // Staleness of the file the symbol lives in.
    let staleness = file_staleness(
        db,
        workspace_id,
        relative_path,
        current_content_hash,
        current_parser_version,
    )
    .await?;

    // Focus definition span + doc.
    let focus_span = first_ast_span(db, &focus).await?;
    let focus_doc = symbol_doc(db, &focus).await?;

    // Gather neighbors from edges touching the focus symbol.
    let edges = db.list_knowledge_edges_for_entity(&focus.entity_id).await?;
    let mut neighbors: Vec<CitedNeighbor> = Vec::new();
    for edge in &edges {
        if edge.edge_type != KnowledgeEdgeType::References {
            continue;
        }
        // Outgoing reference: focus -> callee (focus is the source).
        // Incoming reference: caller -> focus (focus is the target).
        let (relation, other_id) = if edge.source_entity_id == focus.entity_id {
            ("calls", edge.target_entity_id.clone())
        } else if edge.target_entity_id == focus.entity_id {
            ("called_by", edge.source_entity_id.clone())
        } else {
            continue;
        };
        let Some(other) = db.get_knowledge_entity(&other_id).await? else {
            continue;
        };
        // Only code symbols are useful neighbors.
        if other.entity_kind != KnowledgeEntityKind::Symbol {
            continue;
        }
        let span = first_ast_span(db, &other).await?;
        let (line_start, line_end, span_id) = match &span {
            Some(s) => (
                s.line_start.unwrap_or(0),
                s.line_end.unwrap_or(0),
                Some(s.span_id.clone()),
            ),
            None => (0, 0, None),
        };
        let estimated = estimate_tokens(&other.display_name) + estimate_tokens(&other.entity_key);
        neighbors.push(CitedNeighbor {
            relation,
            symbol_key: other.entity_key,
            display_name: other.display_name,
            span_id,
            line_start,
            line_end,
            estimated_tokens: estimated,
        });
    }

    // Deterministic order: calls before called_by, then by symbol key. Dedup
    // identical (relation, key) pairs (a symbol can be reached by >1 edge).
    neighbors.sort_by(|a, b| {
        a.relation
            .cmp(b.relation)
            .then(a.symbol_key.cmp(&b.symbol_key))
            .then(a.line_start.cmp(&b.line_start))
    });
    neighbors.dedup_by(|a, b| a.relation == b.relation && a.symbol_key == b.symbol_key);

    // Token accounting: the focus definition + doc is always included; neighbors
    // are added until the budget is reached, nearest (smallest) first so the
    // bundle packs the most facts into the budget.
    let focus_tokens = estimate_tokens(&focus.display_name)
        + estimate_tokens(&focus.entity_key)
        + focus_doc.as_deref().map(estimate_tokens).unwrap_or(0);
    let budget = token_budget as usize;
    let mut used = focus_tokens.min(budget);
    let mut included: Vec<&CitedNeighbor> = Vec::new();
    let mut truncated = false;
    // Pack smallest-first within the deterministic order to maximize inclusion.
    let mut by_size: Vec<&CitedNeighbor> = neighbors.iter().collect();
    by_size.sort_by(|a, b| {
        a.estimated_tokens
            .cmp(&b.estimated_tokens)
            .then(a.relation.cmp(b.relation))
            .then(a.symbol_key.cmp(&b.symbol_key))
    });
    for neighbor in by_size {
        if used + neighbor.estimated_tokens > budget {
            truncated = true;
            continue;
        }
        used += neighbor.estimated_tokens;
        included.push(neighbor);
    }
    // Re-sort the included set back into the stable presentation order.
    included.sort_by(|a, b| {
        a.relation
            .cmp(b.relation)
            .then(a.symbol_key.cmp(&b.symbol_key))
    });

    let calls: Vec<Value> = included
        .iter()
        .filter(|n| n.relation == "calls")
        .map(|n| n.to_json())
        .collect();
    let called_by: Vec<Value> = included
        .iter()
        .filter(|n| n.relation == "called_by")
        .map(|n| n.to_json())
        .collect();

    let (focus_line_start, focus_line_end, focus_span_id) = match &focus_span {
        Some(s) => (
            s.line_start.unwrap_or(0),
            s.line_end.unwrap_or(0),
            Some(s.span_id.clone()),
        ),
        None => (0, 0, None),
    };

    let allowed_context = json!({
        "kind": "code_symbol_context",
        "extractor_version": super::CODE_EXTRACTOR_VERSION,
        "workspace_id": workspace_id,
        "relative_path": relative_path,
        "staleness": staleness_label(&staleness),
        "focus": {
            "symbol_entity_id": focus.entity_id,
            "symbol_key": focus.entity_key,
            "display_name": focus.display_name,
            "symbol_kind": focus
                .detection_provenance
                .get("symbol_kind")
                .and_then(|v| v.as_str())
                .unwrap_or("symbol"),
            "definition_span_id": focus_span_id,
            "line_start": focus_line_start,
            "line_end": focus_line_end,
            "doc": focus_doc,
        },
        "calls": calls,
        "called_by": called_by,
        "token_accounting": {
            "token_budget": token_budget,
            "estimated_tokens_used": used,
            "neighbors_total": neighbors.len(),
            "neighbors_included": included.len(),
            "truncated": truncated,
            "chars_per_token": CHARS_PER_TOKEN,
        },
    });

    ContextBundle::new(kernel_task_run_id, session_run_id, allowed_context)
        .map_err(|err| CodeIndexError::Kernel(err.to_string()))
}

/// The first `ast`-kind evidence span of an entity (its definition site).
async fn first_ast_span(
    db: &PostgresDatabase,
    entity: &KnowledgeEntity,
) -> CodeIndexResult<Option<KnowledgeSpan>> {
    let span_ids = db.list_knowledge_entity_span_ids(&entity.entity_id).await?;
    for span_id in span_ids {
        if let Some(span) = db.get_knowledge_span(&span_id).await? {
            if matches!(
                span.span_kind,
                crate::storage::knowledge::KnowledgeSpanKind::Ast
            ) {
                return Ok(Some(span));
            }
        }
    }
    Ok(None)
}

/// The doc text attached to a symbol via an incoming `documents` edge (source
/// concept's display_name carries the doc passage).
async fn symbol_doc(
    db: &PostgresDatabase,
    symbol: &KnowledgeEntity,
) -> CodeIndexResult<Option<String>> {
    let edges = db
        .list_knowledge_edges_for_entity(&symbol.entity_id)
        .await?;
    for edge in &edges {
        if edge.edge_type == KnowledgeEdgeType::Documents
            && edge.target_entity_id == symbol.entity_id
        {
            if let Some(concept) = db.get_knowledge_entity(&edge.source_entity_id).await? {
                return Ok(Some(concept.display_name));
            }
        }
    }
    Ok(None)
}

/// Staleness of the file containing the focus symbol.
async fn file_staleness(
    db: &PostgresDatabase,
    workspace_id: &str,
    relative_path: &str,
    current_content_hash: &str,
    current_parser_version: &str,
) -> CodeIndexResult<StalenessVerdict> {
    let file_key = format!("file:{relative_path}");
    let Some(file_entity) = db
        .get_knowledge_entity_by_identity(workspace_id, KnowledgeEntityKind::File, &file_key)
        .await?
    else {
        return Ok(StalenessVerdict::MarkedStale);
    };
    let Some(source_id) = file_entity.primary_source_id else {
        return Ok(StalenessVerdict::MarkedStale);
    };
    match db.get_knowledge_code_file_by_source(&source_id).await? {
        Some(code_file) => Ok(evaluate_staleness(
            &IndexedState {
                indexed_content_hash: code_file.indexed_content_hash,
                indexed_parser_version: code_file.parser_version,
                marked_stale: code_file.stale,
            },
            &LiveSourceState {
                current_content_hash: current_content_hash.to_string(),
                current_parser_version: current_parser_version.to_string(),
            },
        )),
        None => Ok(StalenessVerdict::MarkedStale),
    }
}

fn staleness_label(verdict: &StalenessVerdict) -> &'static str {
    verdict.label()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_estimate_is_deterministic_and_ceils() {
        // 4 chars => 1 token; 5 chars => 2 tokens (ceil).
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens(""), 0);
        // Determinism.
        assert_eq!(
            estimate_tokens("hello world"),
            estimate_tokens("hello world")
        );
    }

    #[test]
    fn cited_neighbor_json_carries_span_and_lines() {
        let neighbor = CitedNeighbor {
            relation: "calls",
            symbol_key: "rust:src/lib.rs#helper".to_string(),
            display_name: "helper".to_string(),
            span_id: Some("KSP-abc".to_string()),
            line_start: 10,
            line_end: 12,
            estimated_tokens: 5,
        };
        let v = neighbor.to_json();
        assert_eq!(v["relation"], "calls");
        assert_eq!(v["span_id"], "KSP-abc");
        assert_eq!(v["line_start"], 10);
        assert_eq!(v["line_end"], 12);
    }
}
