//! Embedded SurrealDB implementation of LoomSearchV2.
//!
//! The source-forward schema currently stores flattened text and optional
//! embeddings without a FULLTEXT analyzer or vector index. Reads therefore
//! fetch the workspace-scoped persisted projection and persisted Loom edges
//! through bound SurrealQL, then perform deterministic exact-token, trigram,
//! cosine, and graph-degree fusion in Rust. This keeps SurrealDB as the sole
//! durable authority without inventing vectors or retaining a PostgreSQL
//! compatibility path.

use std::collections::{BTreeMap, HashMap, HashSet};

use regex::{Regex, RegexBuilder};
use serde_json::json;
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::{event_ledger, loom_store, SurrealDataContext, SurrealStorageError};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::{
    LoomSearchV2Hit, LoomSearchV2Request, LoomSearchV2Response, MutationMetadata, StorageError,
    StorageResult, WriteActorKind,
};

const SEARCH_INDEX_TABLE: &str = "loom_block_search_index";
const BLOCKS_TABLE: &str = "loom_blocks";
const WORKSPACES_TABLE: &str = "workspaces";
const EMBEDDING_DIM: usize = crate::loom_search::LOOM_SEARCH_EMBEDDING_DIM;
const DEFAULT_LIMIT: usize = 25;
const TRIGRAM_MATCH_THRESHOLD: f64 = 0.1;
const VECTOR_MATCH_THRESHOLD: f64 = 0.45;

fn map_err(error: SurrealStorageError) -> StorageError {
    let rendered = error.to_string();
    if rendered.contains("HSK-LOOM-SEARCH-BLOCK-NOT-FOUND") {
        StorageError::NotFound("loom_block")
    } else {
        StorageError::Database(rendered)
    }
}

fn thing(table: &str, id: impl Into<String>) -> RecordId {
    RecordId::new(table, id.into())
}

fn record_key(record: RecordId, table: &'static str) -> StorageResult<String> {
    if record.table.as_str() != table {
        return Err(StorageError::Serialization(format!(
            "expected {table} record link, got {}",
            record.table.as_str()
        )));
    }
    match record.key {
        RecordIdKey::String(id) => Ok(id),
        _ => Err(StorageError::Serialization(format!(
            "{table} record link is not a string key"
        ))),
    }
}

#[derive(SurrealValue)]
struct SearchIndexWriteBinding {
    block: RecordId,
    search: RecordId,
    workspace: RecordId,
    search_text: String,
    embedding: Option<Vec<f64>>,
    embedding_model: Option<String>,
    indexed_at: Datetime,
    ledger: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct SearchIndexWriteRow {
    block_id: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceBinding {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct SearchIndexRow {
    block_id: RecordId,
    content_type: String,
    search_text: String,
    embedding: Option<Vec<f64>>,
}

#[derive(SurrealValue)]
struct SearchEdgeRow {
    source_block_id: RecordId,
    target_block_id: RecordId,
    edge_type: String,
}

fn event_actor(metadata: &MutationMetadata) -> KernelActor {
    let actor_id = metadata
        .actor_id
        .clone()
        .unwrap_or_else(|| "loom-search-index".to_owned());
    match metadata.actor_kind {
        WriteActorKind::Human => KernelActor::Operator(actor_id),
        WriteActorKind::Ai => KernelActor::ModelAdapter(actor_id),
        WriteActorKind::System => KernelActor::System(actor_id),
    }
}

fn build_index_event(
    workspace_id: &str,
    block_id: &str,
    search_text: &str,
    embedding: Option<&[f32]>,
    embedding_model: Option<&str>,
    metadata: &MutationMetadata,
) -> StorageResult<NewKernelEvent> {
    let run_id = format!("LOOM-SEARCH-INDEX-{workspace_id}");
    let search_text_sha256 = format!("{:x}", Sha256::digest(search_text.as_bytes()));
    let embedding_sha256 = embedding.map(|values| {
        let mut hasher = Sha256::new();
        for value in values {
            hasher.update(value.to_le_bytes());
        }
        format!("{:x}", hasher.finalize())
    });
    NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomBlockIndexed,
        event_actor(metadata),
    )
    .aggregate("loom_block_search_index", block_id.to_owned())
    .idempotency_key(format!(
        "LOOM-SEARCH-INDEX:{block_id}:{}",
        metadata.edit_event_id
    ))
    .source_component("loom_search_v2")
    .payload(json!({
        "type": "knowledge_loom_block_indexed",
        "schema_id": "hsk.loom_block_search_index@1",
        "workspace_id": workspace_id,
        "block_id": block_id,
        "search_text_bytes": search_text.len(),
        "search_text_sha256": search_text_sha256,
        "semantic_available": embedding.is_some(),
        "embedding_sha256": embedding_sha256,
        "embedding_model": embedding_model,
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom search index event build failed"))
}

/// Refreshes one persisted LoomSearchV2 projection row and appends its typed
/// EventLedger receipt in the same transaction.
///
/// Result-set index 4 is the `UPSERT ... RETURN AFTER` statement:
/// 0 BEGIN, 1 block guard, 2 content-type LET, 3 ledger append, 4 UPSERT,
/// 5 COMMIT.
pub(crate) async fn reindex_loom_block_search(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    search_text: &str,
    embedding: Option<&[f32]>,
    embedding_model: Option<&str>,
    metadata: MutationMetadata,
) -> StorageResult<()> {
    if metadata.resource_id != block_id {
        return Err(StorageError::Guard("guarded resource id mismatch"));
    }
    if embedding.is_some_and(|values| values.len() != EMBEDDING_DIM) {
        return Err(StorageError::Validation(
            "loom search embedding dimensionality mismatch (expected 768)",
        ));
    }
    if embedding.is_some_and(|values| values.iter().any(|value| !value.is_finite())) {
        return Err(StorageError::Validation(
            "loom search embedding values must be finite",
        ));
    }

    let event = build_index_event(
        workspace_id,
        block_id,
        search_text,
        embedding,
        embedding_model,
        &metadata,
    )?;
    let (_, ledger) = event_ledger::prepare_event(event)?;
    let rows = db
        .query_values_at::<SearchIndexWriteRow, _>(
            "BEGIN TRANSACTION; \
             IF (SELECT VALUE id FROM $block WHERE workspace_id = $workspace LIMIT 1)[0] = NONE { THROW 'HSK-LOOM-SEARCH-BLOCK-NOT-FOUND'; }; \
             LET $content_type = (SELECT VALUE content_type FROM $block WHERE workspace_id = $workspace LIMIT 1)[0]; \
             IF (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $ledger.idempotency_key LIMIT 1)[0] != NONE { \
                IF (SELECT VALUE payload_hash FROM kernel_event_ledger WHERE idempotency_key = $ledger.idempotency_key LIMIT 1)[0] != $ledger.payload_hash { THROW 'HSK-EVENT-LEDGER-IDEMPOTENCY-CONFLICT'; }; \
             } ELSE { \
                CREATE $ledger.record CONTENT { event_id: $ledger.event_id, event_version: $ledger.event_version, kernel_task_run_id: $ledger.kernel_task_run_id, session_run_id: $ledger.session_run_id, aggregate_type: $ledger.aggregate_type, aggregate_id: $ledger.aggregate_id, idempotency_key: $ledger.idempotency_key, event_type: $ledger.event_type, actor_kind: $ledger.actor_kind, actor_id: $ledger.actor_id, causation_id: $ledger.causation_id, correlation_id: $ledger.correlation_id, payload_hash: $ledger.payload_hash, source_component: $ledger.source_component, payload: $ledger.payload, created_at: $ledger.created_at }; \
             }; \
             UPSERT $search SET block_id = $block, workspace_id = $workspace, content_type = $content_type, search_text = $search_text, embedding = $embedding, embedding_model = $embedding_model, indexed_at = $indexed_at RETURN AFTER; \
             COMMIT TRANSACTION;",
            SearchIndexWriteBinding {
                block: thing(BLOCKS_TABLE, block_id),
                search: thing(SEARCH_INDEX_TABLE, block_id),
                workspace: thing(WORKSPACES_TABLE, workspace_id),
                search_text: search_text.to_owned(),
                embedding: embedding.map(|values| values.iter().map(|value| f64::from(*value)).collect()),
                embedding_model: embedding_model.map(str::to_owned),
                indexed_at: Datetime::from(metadata.timestamp),
                ledger,
            },
            4,
        )
        .await
        .map_err(map_err)?;
    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| StorageError::Database("loom search reindex returned no row".to_owned()))?;
    if record_key(row.block_id, BLOCKS_TABLE)? != block_id {
        return Err(StorageError::Database(
            "loom search reindex returned the wrong block".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct QueryText {
    normalized: String,
    tokens: Vec<String>,
    highlight: Option<Regex>,
}

impl QueryText {
    fn new(query: &str) -> StorageResult<Self> {
        let normalized = query.to_lowercase();
        let mut tokens = normalized
            .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
            .filter(|token| !token.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        tokens.sort();
        tokens.dedup();
        let highlight = if tokens.is_empty() {
            None
        } else {
            let pattern = tokens
                .iter()
                .map(|token| regex::escape(token))
                .collect::<Vec<_>>()
                .join("|");
            Some(
                RegexBuilder::new(&pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|_| StorageError::Validation("invalid loom search query"))?,
            )
        };
        Ok(Self {
            normalized,
            tokens,
            highlight,
        })
    }
}

fn token_rank(search_text: &str, query: &QueryText) -> (bool, f64) {
    if query.tokens.is_empty() {
        return (false, 0.0);
    }
    let haystack = search_text.to_lowercase();
    let haystack_tokens = haystack
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .collect::<HashSet<_>>();
    let matched = query
        .tokens
        .iter()
        .filter(|token| haystack_tokens.contains(token.as_str()))
        .count();
    (
        matched == query.tokens.len(),
        matched as f64 / query.tokens.len() as f64,
    )
}

fn trigrams(value: &str) -> HashSet<String> {
    value
        .to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .flat_map(|word| {
            let padded = format!("  {word} ").chars().collect::<Vec<_>>();
            padded
                .windows(3)
                .map(|window| window.iter().collect::<String>())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn trigram_similarity(left: &str, right: &str) -> f64 {
    let left = trigrams(left);
    let right = trigrams(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let common = left.intersection(&right).count() as f64;
    common / (left.len() + right.len() - common as usize) as f64
}

fn cosine_similarity(left: &[f64], right: &[f32]) -> f64 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let (dot, left_norm, right_norm) = left.iter().zip(right.iter()).fold(
        (0.0, 0.0, 0.0),
        |(dot, left_norm, right_norm), (left, right)| {
            let right = f64::from(*right);
            (
                dot + left * right,
                left_norm + left * left,
                right_norm + right * right,
            )
        },
    );
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        (dot / (left_norm.sqrt() * right_norm.sqrt())).clamp(-1.0, 1.0)
    }
}

fn char_window(value: &str, match_start: usize, match_end: usize) -> &str {
    let mut start = match_start.saturating_sub(120);
    while start > 0 && !value.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = match_end.saturating_add(120).min(value.len());
    while end < value.len() && !value.is_char_boundary(end) {
        end += 1;
    }
    &value[start..end]
}

fn highlight(search_text: &str, query: &QueryText) -> String {
    let Some(regex) = query.highlight.as_ref() else {
        return search_text.chars().take(240).collect();
    };
    let fragment = regex
        .find(search_text)
        .map(|matched| char_window(search_text, matched.start(), matched.end()))
        .unwrap_or_else(|| {
            let end = search_text
                .char_indices()
                .nth(240)
                .map(|(index, _)| index)
                .unwrap_or(search_text.len());
            &search_text[..end]
        });
    regex
        .replace_all(fragment, |captures: &regex::Captures<'_>| {
            format!("<mark>{}</mark>", &captures[0])
        })
        .into_owned()
}

#[derive(Debug)]
struct ScoredRow {
    block_id: String,
    score: f64,
    fts_rank: f64,
    trgm_sim: f64,
    vector_sim: f64,
    edge_degree: i64,
    highlight: String,
}

/// Count the complete query/tag-matched set, then report whether this row belongs in the active
/// content-type result set. Keeping both operations in one helper makes their order explicit: an active
/// facet filters hits, never the sibling facet vocabulary used to switch filters in the mounted UI.
fn record_facet_then_matches_filter(
    facets: &mut BTreeMap<String, i64>,
    row_content_type: &str,
    active_content_type: Option<&str>,
) -> bool {
    *facets.entry(row_content_type.to_owned()).or_insert(0) += 1;
    active_content_type.is_none_or(|active| active == row_content_type)
}

/// Runs LoomSearchV2 over the persisted SurrealDB projection and graph.
/// Filters, total, and facets are resolved before deterministic pagination.
pub(crate) async fn loom_search_v2(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    request: LoomSearchV2Request,
) -> StorageResult<LoomSearchV2Response> {
    let query_text = request.query.trim();
    let semantic_available = request.query_embedding.is_some();
    if query_text.is_empty() {
        return Ok(LoomSearchV2Response {
            hits: Vec::new(),
            content_type_facets: BTreeMap::new(),
            semantic_available,
            total: 0,
        });
    }
    if request
        .query_embedding
        .as_ref()
        .is_some_and(|values| values.len() != EMBEDDING_DIM)
    {
        return Err(StorageError::Validation(
            "loom search embedding dimensionality mismatch (expected 768)",
        ));
    }
    if request
        .query_embedding
        .as_ref()
        .is_some_and(|values| values.iter().any(|value| !value.is_finite()))
    {
        return Err(StorageError::Validation(
            "loom search embedding values must be finite",
        ));
    }
    if !request.graph_boost.is_finite() {
        return Err(StorageError::Validation(
            "loom search graph_boost must be finite",
        ));
    }

    let workspace = WorkspaceBinding {
        workspace: thing(WORKSPACES_TABLE, workspace_id),
    };
    let index_rows = db
        .query_values::<SearchIndexRow, _>(
            "SELECT block_id, content_type, search_text, embedding FROM loom_block_search_index WHERE workspace_id = $workspace;",
            workspace,
        )
        .await
        .map_err(map_err)?;
    let edge_rows = db
        .query_values::<SearchEdgeRow, _>(
            "SELECT source_block_id, target_block_id, edge_type FROM loom_edges WHERE workspace_id = $workspace;",
            WorkspaceBinding {
                workspace: thing(WORKSPACES_TABLE, workspace_id),
            },
        )
        .await
        .map_err(map_err)?;

    let mut degree = HashMap::<String, i64>::new();
    let mut tags = HashMap::<String, HashSet<String>>::new();
    for edge in edge_rows {
        let source = record_key(edge.source_block_id, BLOCKS_TABLE)?;
        let target = record_key(edge.target_block_id, BLOCKS_TABLE)?;
        *degree.entry(source.clone()).or_default() += 1;
        if source != target {
            *degree.entry(target.clone()).or_default() += 1;
        }
        if edge.edge_type == "tag" {
            tags.entry(source).or_default().insert(target);
        }
    }

    let query = QueryText::new(query_text)?;
    let graph_boost = request.graph_boost.max(0.0);
    let requested_tags = request
        .tag_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut scored = Vec::new();
    let mut content_type_facets = BTreeMap::new();
    for row in index_rows {
        let block_id = record_key(row.block_id, BLOCKS_TABLE)?;
        if !requested_tags.is_empty()
            && tags.get(&block_id).is_none_or(|actual| {
                !actual
                    .iter()
                    .any(|tag| requested_tags.contains(tag.as_str()))
            })
        {
            continue;
        }

        let (fts_match, fts_rank) = token_rank(&row.search_text, &query);
        let trgm_sim = trigram_similarity(&row.search_text, &query.normalized);
        let vector_sim = request
            .query_embedding
            .as_deref()
            .zip(row.embedding.as_deref())
            .map(|(query, stored)| cosine_similarity(stored, query))
            .unwrap_or(0.0);
        if !fts_match
            && trgm_sim <= TRIGRAM_MATCH_THRESHOLD
            && (!semantic_available || vector_sim <= VECTOR_MATCH_THRESHOLD)
        {
            continue;
        }

        // Facets describe the complete query/tag-matched set before the active content-type filter is
        // applied. Otherwise selecting one facet makes every sibling facet disappear, preventing the
        // mounted search panel from switching filters without first clearing the active one.
        if !record_facet_then_matches_filter(
            &mut content_type_facets,
            &row.content_type,
            request
                .content_type
                .as_ref()
                .map(|content_type| content_type.as_str()),
        ) {
            continue;
        }

        let edge_degree = degree.get(&block_id).copied().unwrap_or(0);
        let score = fts_rank + trgm_sim * 0.6 + vector_sim * 1.2 + edge_degree as f64 * graph_boost;
        scored.push(ScoredRow {
            block_id,
            score,
            fts_rank,
            trgm_sim,
            vector_sim,
            edge_degree,
            highlight: highlight(&row.search_text, &query),
        });
    }

    scored.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.block_id.cmp(&right.block_id))
    });
    let total = i64::try_from(scored.len())
        .map_err(|_| StorageError::Serialization("loom search total exceeds i64".to_owned()))?;
    let offset = request.offset as usize;
    let limit = if request.limit == 0 {
        DEFAULT_LIMIT
    } else {
        request.limit as usize
    };
    let page = scored.into_iter().skip(offset).take(limit);
    let mut hits = Vec::new();
    for row in page {
        let block = loom_store::get_loom_block(db, workspace_id, &row.block_id).await?;
        hits.push(LoomSearchV2Hit {
            block,
            score: row.score,
            fts_rank: row.fts_rank,
            trgm_sim: row.trgm_sim,
            vector_sim: row.vector_sim,
            edge_degree: row.edge_degree,
            highlight: row.highlight,
        });
    }

    Ok(LoomSearchV2Response {
        hits,
        content_type_facets,
        semantic_available,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoring_helpers_preserve_keyword_fuzzy_and_semantic_modalities() {
        let query = QueryText::new("Rust editor").expect("valid query");
        let (matched, rank) = token_rank("A fast Rust native editor", &query);
        assert!(matched);
        assert_eq!(rank, 1.0);
        assert!(trigram_similarity("native editor", "nativ editor") > TRIGRAM_MATCH_THRESHOLD);
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]), 1.0);
    }

    #[test]
    fn highlight_marks_original_case_without_cutting_utf8() {
        let query = QueryText::new("rust").expect("valid query");
        let rendered = highlight("Notes about ééé Rust storage", &query);
        assert!(rendered.contains("<mark>Rust</mark>"));
    }

    #[test]
    fn active_content_type_filters_hits_without_hiding_sibling_facets() {
        let mut facets = BTreeMap::new();
        let admitted = ["note", "code", "note"]
            .into_iter()
            .filter(|content_type| {
                record_facet_then_matches_filter(&mut facets, content_type, Some("note"))
            })
            .collect::<Vec<_>>();

        assert_eq!(admitted, vec!["note", "note"]);
        assert_eq!(facets.get("note"), Some(&2));
        assert_eq!(facets.get("code"), Some(&1));
    }
}
