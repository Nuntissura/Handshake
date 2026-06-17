//! WP-KERNEL-009 MT-264 UnifiedWorkSurface-264-LoomSearchV2 (DEC-008).
//!
//! Postgres-native, graph-blended ES-class search over the Loom corpus. This
//! service layer wires the model-runtime embedding surface ([`LlmClient::embedding`],
//! reused from the MT-260 AI-Loom path) into the derived `loom_block_search_index`
//! projection and the hybrid search query:
//!
//!   * [`reindex_block`] computes the block's flattened search text and, when an
//!     embedding model is configured, a REAL embedding via the configured model,
//!     then upserts the projection row through the receipted storage path.
//!   * [`search`] embeds the QUERY through the same surface (when available) and
//!     runs the hybrid FTS + pg_trgm + pgvector kNN query, blended with the Loom
//!     graph and faceted by content_type.
//!
//! No-model path (HARD requirement): when the configured client declines the
//! embedding call with a typed [`LlmError`] (e.g. [`LlmError::EmbeddingUnsupported`]
//! from `DisabledLlmClient`), the semantic modality is OMITTED — the search
//! degrades to keyword + trigram and the response's `semantic_available` flag is
//! `false`. NO vector is fabricated and NO semantic result is invented.

use uuid::Uuid;

use crate::llm::{EmbeddingRequest, LlmClient, LlmError};
use crate::storage::{
    Database, LoomBlock, LoomSearchV2Request, LoomSearchV2Response, StorageError, StorageResult,
    WriteContext,
};

/// The canonical embedding dimensionality for LoomSearchV2 (matches the
/// `vector(768)` column in migration 0336). A model that returns a different
/// dimensionality is rejected loudly rather than silently truncated/padded.
pub const LOOM_SEARCH_EMBEDDING_DIM: usize = 768;

/// Flattens a block into the text the search index covers.
pub fn block_search_text(block: &LoomBlock) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(title) = block.title.as_deref() {
        parts.push(title);
    }
    if let Some(filename) = block.original_filename.as_deref() {
        parts.push(filename);
    }
    if let Some(full_text) = block.derived.full_text_index.as_deref() {
        parts.push(full_text);
    }
    parts.join("\n")
}

/// Typed result of attempting to embed text through the configured model.
enum EmbedOutcome {
    /// A real embedding of the canonical dimensionality.
    Embedded(Vec<f32>),
    /// No embedding model configured (typed decline) — caller must degrade to
    /// keyword/trigram, never fabricate.
    NoModel,
}

/// Embeds `text` via the configured model. Maps a typed model decline to
/// [`EmbedOutcome::NoModel`]; a wrong-dimensionality response is a loud error.
async fn embed_text(llm: &dyn LlmClient, text: &str) -> StorageResult<EmbedOutcome> {
    let model_id = llm.profile().model_id.clone();
    let req = EmbeddingRequest::new(Uuid::now_v7(), text.to_string(), model_id);
    match llm.embedding(req).await {
        Ok(resp) => {
            if resp.vector.len() != LOOM_SEARCH_EMBEDDING_DIM {
                return Err(StorageError::Validation(
                    "loom search embedding dimensionality mismatch (expected 768)",
                ));
            }
            Ok(EmbedOutcome::Embedded(resp.vector))
        }
        // Any typed LLM error => no embedding model available. Degrade, do not
        // fabricate. (Covers EmbeddingUnsupported + provider/transport errors.)
        Err(LlmError::EmbeddingUnsupported) | Err(_) => Ok(EmbedOutcome::NoModel),
    }
}

/// Refreshes the full search-index projection (keyword/trigram text + REAL
/// embedding when a model is configured) for one block, through the receipted
/// storage path. Returns whether a semantic embedding was written.
pub async fn reindex_block(
    db: &dyn Database,
    llm: &dyn LlmClient,
    ctx: &WriteContext,
    block: &LoomBlock,
) -> StorageResult<bool> {
    let text = block_search_text(block);
    let (embedding, model): (Option<Vec<f32>>, Option<String>) =
        match embed_text(llm, &text).await? {
            EmbedOutcome::Embedded(vector) => (Some(vector), Some(llm.profile().model_id.clone())),
            EmbedOutcome::NoModel => (None, None),
        };
    db.reindex_loom_block_search(
        ctx,
        &block.workspace_id,
        &block.block_id,
        &text,
        embedding.as_deref(),
        model.as_deref(),
    )
    .await?;
    Ok(embedding.is_some())
}

/// Runs a hybrid LoomSearchV2 query. Embeds the query through the configured
/// model for the semantic modality; on a typed model decline, omits the
/// embedding (keyword/trigram only, `semantic_available=false`).
pub async fn search(
    db: &dyn Database,
    llm: &dyn LlmClient,
    workspace_id: &str,
    mut request: LoomSearchV2Request,
) -> StorageResult<LoomSearchV2Response> {
    if request.query_embedding.is_none() && !request.query.trim().is_empty() {
        if let EmbedOutcome::Embedded(vector) = embed_text(llm, &request.query).await? {
            request.query_embedding = Some(vector);
        }
    }
    db.loom_search_v2(workspace_id, request).await
}
