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
//!
//! Dim-mismatch degrade (WP-1 MT-014): when the configured model returns an
//! embedding whose dimensionality does NOT match [`LOOM_SEARCH_EMBEDDING_DIM`]
//! (e.g. a normal chat model configured as the local default), the mismatch is
//! a DISTINCT typed outcome ([`EmbedOutcome::DimMismatch`]) — NOT folded into
//! `NoModel`. Both `reindex_block` and `search` DEGRADE (keyword/trigram) rather
//! than hard-erroring: they emit a surfaced Flight Recorder event and set the
//! response's `semantic_unavailable_reason` to a typed
//! [`SemanticUnavailableReason::DimMismatch`]. This fixes the prior behavior
//! where a mismatch propagated a hard `StorageError::Validation` that 400'd the
//! search query path and errored reindex.

use serde_json::json;
use uuid::Uuid;

use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::llm::{EmbeddingRequest, LlmClient, LlmError};
use crate::storage::{
    Database, LoomBlock, LoomSearchV2Request, LoomSearchV2Response, SemanticUnavailableReason,
    StorageResult, WriteContext,
};

/// The canonical embedding dimensionality for LoomSearchV2 (matches the
/// `vector(768)` column in migration 0336). A model that returns a different
/// dimensionality degrades the semantic modality (typed, surfaced) rather than
/// being silently truncated/padded or hard-erroring.
pub const LOOM_SEARCH_EMBEDDING_DIM: usize = 768;

/// Stable Flight Recorder event key for a surfaced semantic-degrade (the
/// configured embedding model is misconfigured for this index).
pub const LOOM_SEMANTIC_DEGRADED_FR_EVENT: &str = "FR-EVT-LOOM-SEMANTIC-DEGRADED";

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
    Embedded {
        vector: Vec<f32>,
        embedding_space_id: String,
    },
    /// No embedding model configured (typed decline) — caller must degrade to
    /// keyword/trigram, never fabricate.
    NoModel,
    /// A model IS configured but returned an embedding of the WRONG
    /// dimensionality (misconfiguration). Distinct from `NoModel` so the
    /// degrade is surfaced (event + typed reason), never a silent drop.
    DimMismatch { expected: usize, actual: usize },
}

/// Embeds `text` via the configured model. Maps a typed model decline to
/// [`EmbedOutcome::NoModel`]; a wrong-dimensionality response to a DISTINCT
/// [`EmbedOutcome::DimMismatch`] (degrade, not error).
async fn embed_text(llm: &dyn LlmClient, text: &str) -> EmbedOutcome {
    let Some(entry) = llm
        .model_catalog()
        .and_then(|catalog| catalog.embedding_model_for_dim(LOOM_SEARCH_EMBEDDING_DIM))
    else {
        return EmbedOutcome::NoModel;
    };
    let Some(embedding_space_id) = entry.embedding_space_id() else {
        return EmbedOutcome::NoModel;
    };
    let req = EmbeddingRequest::new(Uuid::now_v7(), text.to_string(), entry.model_id);
    match llm.embedding(req).await {
        Ok(resp) => {
            if resp.vector.len() != LOOM_SEARCH_EMBEDDING_DIM {
                EmbedOutcome::DimMismatch {
                    expected: LOOM_SEARCH_EMBEDDING_DIM,
                    actual: resp.vector.len(),
                }
            } else {
                EmbedOutcome::Embedded {
                    vector: resp.vector,
                    embedding_space_id,
                }
            }
        }
        Err(LlmError::EmbeddingDimensionMismatch { expected, actual }) => {
            EmbedOutcome::DimMismatch { expected, actual }
        }
        // Any other typed LLM error => no embedding model available. Degrade,
        // do not fabricate. (Covers EmbeddingUnsupported + provider/transport.)
        Err(LlmError::EmbeddingUnsupported) | Err(_) => EmbedOutcome::NoModel,
    }
}

/// Emits a surfaced Flight Recorder event recording a semantic-degrade due to a
/// dimensionality mismatch (WP-1 MT-014). Non-fatal: a recorder failure is
/// logged, never propagated, so the degraded-but-usable search/reindex still
/// succeeds.
async fn emit_dim_mismatch_event(
    recorder: &dyn FlightRecorder,
    surface: &str,
    workspace_id: &str,
    expected: usize,
    actual: usize,
) {
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::System,
        FlightRecorderActor::System,
        Uuid::now_v7(),
        json!({
            "fr_event": LOOM_SEMANTIC_DEGRADED_FR_EVENT,
            "type": "loom_search_v2_semantic_degraded",
            "reason": "embedding_dim_mismatch",
            "surface": surface,
            "workspace_id": workspace_id,
            "expected_dim": expected,
            "actual_dim": actual,
        }),
    )
    .with_wsids(vec![workspace_id.to_string()]);
    if let Err(err) = recorder.record_event(event).await {
        tracing::warn!(
            target: "handshake_core::loom_search",
            error = %err,
            workspace_id = %workspace_id,
            surface = %surface,
            "failed to record loom semantic-degrade event"
        );
    }
}

/// Refreshes the full search-index projection (keyword/trigram text + REAL
/// embedding when a model is configured) for one block, through the receipted
/// storage path. Returns whether a semantic embedding was written.
///
/// A dimensionality mismatch DEGRADES (writes keyword/trigram only, emits a
/// surfaced Flight Recorder event) and returns `Ok(false)` — it does NOT
/// propagate a hard error.
pub async fn reindex_block(
    db: &dyn Database,
    llm: &dyn LlmClient,
    recorder: &dyn FlightRecorder,
    ctx: &WriteContext,
    block: &LoomBlock,
) -> StorageResult<bool> {
    let text = block_search_text(block);
    let mut embedding_model: Option<String> = None;
    let embedding: Option<Vec<f32>> = match embed_text(llm, &text).await {
        EmbedOutcome::Embedded {
            vector,
            embedding_space_id,
        } => {
            embedding_model = Some(embedding_space_id);
            Some(vector)
        }
        EmbedOutcome::NoModel => None,
        EmbedOutcome::DimMismatch { expected, actual } => {
            emit_dim_mismatch_event(recorder, "reindex", &block.workspace_id, expected, actual)
                .await;
            None
        }
    };
    db.reindex_loom_block_search(
        ctx,
        &block.workspace_id,
        &block.block_id,
        &text,
        embedding.as_deref(),
        embedding_model.as_deref(),
    )
    .await?;
    Ok(embedding.is_some())
}

/// Runs a hybrid LoomSearchV2 query. Embeds the query through the configured
/// model for the semantic modality; on a typed model decline OR a
/// dimensionality mismatch, omits the embedding (keyword/trigram only,
/// `semantic_available=false`) and records a TYPED
/// [`SemanticUnavailableReason`]. A dimensionality mismatch additionally emits
/// a surfaced Flight Recorder event. NO hard error / 400 on either degrade path.
pub async fn search(
    db: &dyn Database,
    llm: &dyn LlmClient,
    recorder: &dyn FlightRecorder,
    workspace_id: &str,
    mut request: LoomSearchV2Request,
) -> StorageResult<LoomSearchV2Response> {
    let mut degrade_reason: Option<SemanticUnavailableReason> = None;
    if request.query_embedding.is_none() && !request.query.trim().is_empty() {
        match embed_text(llm, &request.query).await {
            EmbedOutcome::Embedded {
                vector,
                embedding_space_id,
            } => {
                request.query_embedding = Some(vector);
                request.query_embedding_model = Some(embedding_space_id);
            }
            EmbedOutcome::NoModel => {
                degrade_reason = Some(SemanticUnavailableReason::NoModel);
            }
            EmbedOutcome::DimMismatch { expected, actual } => {
                degrade_reason = Some(SemanticUnavailableReason::DimMismatch { expected, actual });
                emit_dim_mismatch_event(recorder, "search", workspace_id, expected, actual).await;
            }
        }
    }
    let mut response = db.loom_search_v2(workspace_id, request).await?;
    // Surface the typed degrade reason so a dropped semantic modality is never
    // silent. A degrade always means semantic was NOT contributed.
    if let Some(reason) = degrade_reason {
        response.semantic_available = false;
        response.semantic_unavailable_reason = Some(reason);
    }
    Ok(response)
}
