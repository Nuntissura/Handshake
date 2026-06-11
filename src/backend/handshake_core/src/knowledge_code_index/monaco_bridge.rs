//! WP-KERNEL-009 MT-109 MonacoCodeLensBridge.
//!
//! Master Spec anchor: 2.3.13.11 (Monaco is a bundled product library;
//! "feed code navigation and diagnostics into embedded Monaco without requiring
//! external editor services"). This module produces the BACKEND PAYLOAD the
//! embedded Monaco editor consumes for code lenses and hovers: for one indexed
//! file it emits, per symbol, a stable id, the definition range, reference
//! ranges, the doc text, and the staleness flag.
//!
//! This is the BACKEND SHAPE + the query that fills it — not the React wiring
//! (that lands in a later GUI group). The typed contract here is what the
//! frontend will deserialize. The bridge reads the indexed graph through
//! `KnowledgeStore` (entities/spans/edges) + the code-file index state
//! (`knowledge_code_files`) and never re-parses.

use serde::{Deserialize, Serialize};

use crate::storage::knowledge::{
    KnowledgeEdgeType, KnowledgeEntityKind, KnowledgeSpanKind, KnowledgeStore,
};
use crate::storage::postgres::PostgresDatabase;

use super::staleness::{evaluate_staleness, IndexedState, LiveSourceState, StalenessVerdict};
use super::{CodeIndexError, CodeIndexResult};

/// A 1-based inclusive line range Monaco can turn into a `monaco.Range`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRange {
    pub start_line: i32,
    pub end_line: i32,
}

/// One code lens / hover entry for a symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeLensEntry {
    /// Stable symbol entity id (KEN-...), used as the lens id.
    pub symbol_entity_id: String,
    /// Stable symbol key (`{lang}:{path}#{symbol_path}`).
    pub symbol_key: String,
    pub display_name: String,
    /// The symbol kind string (function/struct/method/...).
    pub symbol_kind: String,
    /// Definition range (the symbol's `ast` span).
    pub definition: LineRange,
    /// In-file reference ranges (from `references`/`validates` edges whose
    /// evidence spans live in this file).
    pub references: Vec<LineRange>,
    /// Doc comment text attached to this symbol (from a `documents` edge), if
    /// any.
    pub doc: Option<String>,
    /// Count of callers (incoming `references` edges) across the workspace.
    pub caller_count: u32,
}

/// The whole bridge payload for one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonacoCodeLensPayload {
    pub workspace_id: String,
    pub relative_path: String,
    /// Staleness of this file's index; the frontend MUST surface it so a stale
    /// lens is never shown as authoritative.
    pub staleness: StalenessVerdict,
    pub entries: Vec<CodeLensEntry>,
}

/// Build the Monaco code-lens payload for one indexed code file.
///
/// `current_content_hash` / `current_parser_version` are the LIVE source state
/// (what the open editor buffer hashes to, and the adapter version for its
/// language); they let the bridge flag staleness against the indexed state.
pub async fn build_monaco_payload(
    db: &PostgresDatabase,
    workspace_id: &str,
    relative_path: &str,
    current_content_hash: &str,
    current_parser_version: &str,
) -> CodeIndexResult<MonacoCodeLensPayload> {
    // Resolve the file entity + its source.
    let file_key = format!("file:{relative_path}");
    let file_entity = db
        .get_knowledge_entity_by_identity(workspace_id, KnowledgeEntityKind::File, &file_key)
        .await?
        .ok_or_else(|| {
            CodeIndexError::Validation(format!("file '{relative_path}' is not indexed"))
        })?;
    let source_id = file_entity.primary_source_id.clone().ok_or_else(|| {
        CodeIndexError::Validation(format!("indexed file '{relative_path}' has no source"))
    })?;

    // Staleness from the code-file index state.
    let staleness = match db.get_knowledge_code_file_by_source(&source_id).await? {
        Some(code_file) => evaluate_staleness(
            &IndexedState {
                indexed_content_hash: code_file.indexed_content_hash,
                indexed_parser_version: code_file.parser_version,
                marked_stale: code_file.stale,
            },
            &LiveSourceState {
                current_content_hash: current_content_hash.to_string(),
                current_parser_version: current_parser_version.to_string(),
            },
        ),
        // No code-file row: treat as marked stale (it is not freshly indexed).
        None => StalenessVerdict::MarkedStale,
    };

    // All spans of the source, indexed by id (for reference ranges + defs).
    let spans = db.list_knowledge_spans_for_source(&source_id).await?;
    let span_by_id: std::collections::HashMap<String, LineRange> = spans
        .iter()
        .filter(|s| matches!(s.span_kind, KnowledgeSpanKind::Ast))
        .map(|s| {
            (
                s.span_id.clone(),
                LineRange {
                    start_line: s.line_start.unwrap_or(0),
                    end_line: s.line_end.unwrap_or(0),
                },
            )
        })
        .collect();

    // Symbol entities of this workspace whose key targets this file.
    let prefix_a = format!("rust:{relative_path}#");
    let prefix_b = format!("typescript:{relative_path}#");
    let prefix_c = format!("tsx:{relative_path}#");
    let prefix_d = format!("javascript:{relative_path}#");
    let all_symbols = db
        .list_knowledge_entities_by_kind(workspace_id, KnowledgeEntityKind::Symbol)
        .await?;

    let mut entries = Vec::new();
    for symbol in all_symbols {
        if !(symbol.entity_key.starts_with(&prefix_a)
            || symbol.entity_key.starts_with(&prefix_b)
            || symbol.entity_key.starts_with(&prefix_c)
            || symbol.entity_key.starts_with(&prefix_d))
        {
            continue;
        }

        // Definition range: the symbol's evidence span.
        let def_span_ids = db.list_knowledge_entity_span_ids(&symbol.entity_id).await?;
        let definition = def_span_ids
            .iter()
            .find_map(|id| span_by_id.get(id).cloned())
            .unwrap_or(LineRange {
                start_line: 0,
                end_line: 0,
            });

        // Edges touching this symbol: incoming references = callers; outgoing
        // documents edges (source=concept) attach docs.
        let edges = db
            .list_knowledge_edges_for_entity(&symbol.entity_id)
            .await?;
        let mut references: Vec<LineRange> = Vec::new();
        let mut caller_count = 0u32;
        for edge in &edges {
            if edge.edge_type == KnowledgeEdgeType::References
                && edge.target_entity_id == symbol.entity_id
            {
                caller_count += 1;
                // Reference ranges whose evidence spans live in this file.
                let span_ids = db.list_knowledge_edge_span_ids(&edge.edge_id).await?;
                for sid in span_ids {
                    if let Some(range) = span_by_id.get(&sid) {
                        references.push(range.clone());
                    }
                }
            }
        }
        references.sort_by_key(|r| (r.start_line, r.end_line));
        references.dedup();

        // Doc text: a `documents` edge whose TARGET is this symbol; the source
        // concept's display_name is the doc text.
        let mut doc = None;
        for edge in &edges {
            if edge.edge_type == KnowledgeEdgeType::Documents
                && edge.target_entity_id == symbol.entity_id
            {
                if let Some(concept) = db.get_knowledge_entity(&edge.source_entity_id).await? {
                    doc = Some(concept.display_name);
                    break;
                }
            }
        }

        let symbol_kind = symbol
            .detection_provenance
            .get("symbol_kind")
            .and_then(|v| v.as_str())
            .unwrap_or("symbol")
            .to_string();

        entries.push(CodeLensEntry {
            symbol_entity_id: symbol.entity_id.clone(),
            symbol_key: symbol.entity_key.clone(),
            display_name: symbol.display_name.clone(),
            symbol_kind,
            definition,
            references,
            doc,
            caller_count,
        });
    }
    entries.sort_by(|a, b| {
        a.definition
            .start_line
            .cmp(&b.definition.start_line)
            .then(a.symbol_key.cmp(&b.symbol_key))
    });

    Ok(MonacoCodeLensPayload {
        workspace_id: workspace_id.to_string(),
        relative_path: relative_path.to_string(),
        staleness,
        entries,
    })
}
