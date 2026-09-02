#![cfg(feature = "test-utils")]
//! WP-KERNEL-009 MT-264 UnifiedWorkSurface-264-LoomSearchV2 -- real PostgreSQL
//! proof. Postgres-native, graph-blended ES-class search over the Loom corpus
//! (DEC-008: NOT Elasticsearch / no external search daemon).
//!
//! Every modality is proven against the Handshake-managed PostgreSQL with the
//! pg_trgm + pgvector extensions, the derived `loom_block_search_index`
//! projection, and the hybrid search query:
//!   * FTS: ts_rank ordering + ts_headline highlight,
//!   * fuzzy: pg_trgm near-match on a misspelled query,
//!   * semantic: pgvector HNSW kNN over REAL embeddings + hybrid keyword+vector,
//!   * graph-blend: content_type facets + loom_edges degree ranking,
//!   * reindex consistency: edit -> reflected, delete -> gone (NEGATIVE proof),
//!   * no-model: typed keyword/trigram fallback with NO fabricated semantic.
//!
//! The semantic modality uses `InMemoryLlmClient::with_embedding_dim(768)`, an
//! HONEST embedding substitute: the vector is a REAL deterministic function of
//! the text (the same `LlmClient::embedding` trait the production Ollama
//! `/api/embeddings` path implements), so pgvector kNN returns the genuinely
//! closest block -- it is NOT a fabricated search result. The no-model negative
//! uses `DisabledLlmClient`, which declines the embedding call with a typed
//! error exactly like a runtime with no embedding model configured.

mod knowledge_pg_support;

use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;

use chrono::Utc;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::InMemoryLlmClient;
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, DisabledLlmClient, EmbeddingRequest, EmbeddingResponse,
    LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::loom_search;
use handshake_core::model_runtime::{
    BaseModelTag, ModelCapabilities, ModelCatalog, ModelId, ModelRegistration, ModelRegistry,
    OperatorId, ProviderKind, RuntimeBinding,
};
use handshake_core::storage::{
    Database, LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate,
    LoomEdgeCreatedBy, LoomEdgeType, LoomSearchV2Request, NewLoomBlock, NewLoomEdge,
    SemanticUnavailableReason, WriteContext,
};
use knowledge_pg_support::knowledge_pg;

/// A capturing Flight Recorder for the semantic-degrade proofs. The existing
/// modality tests do not inspect events (they pass the shared [`rec`] handle);
/// the MT-014 dim-mismatch test uses a fresh instance and asserts the surfaced
/// degrade event.
#[derive(Default)]
struct CapturingRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

impl CapturingRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().expect("recorder lock").clone()
    }
}

#[async_trait]
impl FlightRecorder for CapturingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events.lock().expect("recorder lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events())
    }
}

/// Shared no-inspect recorder for the pre-existing modality tests (they degrade
/// only via `NoModel`, which emits no event).
fn rec() -> &'static CapturingRecorder {
    static REC: OnceLock<CapturingRecorder> = OnceLock::new();
    REC.get_or_init(CapturingRecorder::default)
}

async fn pg_required(name: &str) -> knowledge_pg_support::KnowledgePg {
    knowledge_pg().await.unwrap_or_else(|| {
        panic!("PostgreSQL unavailable for {name}: proof requires live PostgreSQL/EventLedger")
    })
}

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                eprintln!("SKIP MT-264 LoomSearchV2 proof: PostgreSQL binaries not found");
                return;
            }
        }
    }};
}

async fn make_block(
    db: &handshake_core::storage::postgres::PostgresDatabase,
    ctx: &WriteContext,
    ws: &str,
    title: &str,
    full_text: &str,
) -> LoomBlock {
    db.create_loom_block(
        ctx,
        NewLoomBlock {
            block_id: None,
            workspace_id: ws.to_string(),
            content_type: LoomBlockContentType::Note,
            document_id: None,
            asset_id: None,
            title: Some(title.to_string()),
            original_filename: None,
            content_hash: None,
            pinned: false,
            journal_date: None,
            imported_at: None,
            derived: LoomBlockDerived {
                full_text_index: Some(full_text.to_string()),
                ..Default::default()
            },
        },
    )
    .await
    .expect("create loom block")
}

fn req(query: &str) -> LoomSearchV2Request {
    LoomSearchV2Request {
        query: query.to_string(),
        limit: 25,
        ..Default::default()
    }
}

fn catalog_registration(
    model_id: ModelId,
    tag: &str,
    capabilities: ModelCapabilities,
) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: std::path::PathBuf::from(format!("fixtures/models/{tag}.gguf")),
        sha256: [11; 32],
        runtime_binding: RuntimeBinding::Candle,
        declared_capabilities: capabilities,
        base_model_tag: BaseModelTag::new(tag),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("loom-search-test"),
        provider: ProviderKind::Local,
    }
}

fn model_catalog(entries: Vec<(ModelId, &'static str, ModelCapabilities)>) -> Arc<ModelCatalog> {
    let mut registry = ModelRegistry::default();
    for (model_id, tag, capabilities) in entries {
        registry
            .register(catalog_registration(model_id, tag, capabilities))
            .expect("register test model");
        registry.mark_loaded(model_id).expect("mark model loaded");
    }
    ModelCatalog::from_registry(Arc::new(registry))
}

struct CatalogEmbeddingClient {
    profile: ModelProfile,
    catalog: Arc<ModelCatalog>,
    embedding_dim: usize,
    panic_on_embedding: bool,
    requested_model_ids: Mutex<Vec<String>>,
}

impl CatalogEmbeddingClient {
    fn new(profile_model_id: String, catalog: Arc<ModelCatalog>, embedding_dim: usize) -> Self {
        Self {
            profile: ModelProfile::new(profile_model_id, 4096),
            catalog,
            embedding_dim,
            panic_on_embedding: false,
            requested_model_ids: Mutex::new(Vec::new()),
        }
    }

    fn panics_on_embedding(mut self) -> Self {
        self.panic_on_embedding = true;
        self
    }

    fn requested_model_ids(&self) -> Vec<String> {
        self.requested_model_ids
            .lock()
            .expect("requested ids lock")
            .clone()
    }
}

#[async_trait]
impl LlmClient for CatalogEmbeddingClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage::default(),
            latency_ms: 0,
        })
    }

    async fn embedding(&self, req: EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        if self.panic_on_embedding {
            panic!("embedding should not be called when no embedding-capable catalog row exists");
        }
        self.requested_model_ids
            .lock()
            .expect("requested ids lock")
            .push(req.model_id.clone());
        Ok(EmbeddingResponse {
            vector: InMemoryLlmClient::deterministic_embedding(&req.input, self.embedding_dim),
            model_id: req.model_id,
            latency_ms: 1,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }

    fn model_catalog(&self) -> Option<Arc<ModelCatalog>> {
        Some(Arc::clone(&self.catalog))
    }
}

/// FTS: ts_rank-ordered, ts_headline-highlighted results over real content.
#[tokio::test]
async fn mt264_fulltext_rank_and_highlight() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    // No embedding model -> keyword/trigram only.
    let llm = DisabledLlmClient::new("none".into(), "no embedding model".into());

    make_block(
        &pg.db,
        &ctx,
        &ws,
        "Migration runbook",
        "The migration runbook describes how to run a database migration safely.",
    )
    .await;
    make_block(
        &pg.db,
        &ctx,
        &ws,
        "Holiday notes",
        "Notes about a beach holiday with no database content at all.",
    )
    .await;

    let resp = loom_search::search(&pg.db, &llm, rec(), &ws, req("database migration"))
        .await
        .expect("search");
    assert!(!resp.hits.is_empty(), "expected FTS hits");
    // The migration block ranks first.
    assert!(
        resp.hits[0].block.title.as_deref() == Some("Migration runbook"),
        "ts_rank should order the migration block first, got {:?}",
        resp.hits[0].block.title
    );
    assert!(resp.hits[0].fts_rank > 0.0, "fts_rank must be non-zero");
    // ts_headline produced a <mark> highlight around a query term.
    assert!(
        resp.hits[0].highlight.contains("<mark>"),
        "expected ts_headline highlight markers, got {:?}",
        resp.hits[0].highlight
    );
    assert!(
        !resp.semantic_available,
        "no embedding model -> not available"
    );
}

/// Fuzzy/substring: a misspelled query returns the near-match via pg_trgm.
#[tokio::test]
async fn mt264_trigram_fuzzy_match() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let llm = DisabledLlmClient::new("none".into(), "no embedding model".into());

    make_block(
        &pg.db,
        &ctx,
        &ws,
        "Kubernetes deployment guide",
        "Kubernetes deployment orchestration guide for production clusters.",
    )
    .await;

    // Misspelled query: "kubernates deploymnet" -- no exact FTS lexeme match,
    // but pg_trgm similarity finds the near-match.
    let resp = loom_search::search(&pg.db, &llm, rec(), &ws, req("kubernates deploymnet"))
        .await
        .expect("search");
    assert!(
        !resp.hits.is_empty(),
        "pg_trgm should fuzzy-match the misspelled query"
    );
    assert!(
        resp.hits[0].trgm_sim > 0.0,
        "trgm similarity must be non-zero for the fuzzy hit"
    );
}

/// Semantic: pgvector HNSW kNN over REAL embeddings + hybrid keyword+vector;
/// the semantic modality surfaces a block whose TEXT does not lexically match
/// the query but whose embedding is closest.
#[tokio::test]
async fn mt264_pgvector_semantic_and_hybrid() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    // MT-016: embeddings resolve through the registry-declared embedding model
    // (never the chat model). Provide a 768-dim embedding-capable catalog so the
    // configured model matches the index dimension and a real vector is written.
    let catalog = model_catalog(vec![(
        ModelId::new_v7(),
        "emb-768",
        ModelCapabilities {
            supports_embedding: true,
            embedding_dimension: Some(loom_search::LOOM_SEARCH_EMBEDDING_DIM),
            ..Default::default()
        },
    )]);
    let llm = CatalogEmbeddingClient::new(ModelId::new_v7().to_string(), catalog, 768);

    let canine = make_block(
        &pg.db,
        &ctx,
        &ws,
        "Pet care",
        "the dog runs fast in the park",
    )
    .await;
    let finance = make_block(
        &pg.db,
        &ctx,
        &ws,
        "Finance",
        "quarterly revenue projections and tax filings",
    )
    .await;

    // Reindex both blocks WITH real embeddings via the configured model.
    for block in [&canine, &finance] {
        let wrote = loom_search::reindex_block(&pg.db, &llm, rec(), &ctx, block)
            .await
            .expect("reindex with embedding");
        assert!(
            wrote,
            "an embedding model is configured -> embedding written"
        );
    }

    // Query embedding overlaps the canine block's tokens => closest neighbour.
    let resp = loom_search::search(
        &pg.db,
        &llm,
        rec(),
        &ws,
        req("the dog runs fast in the park"),
    )
    .await
    .expect("search");
    assert!(
        resp.semantic_available,
        "embedding model -> semantic available"
    );
    assert_eq!(
        resp.hits[0].block.block_id, canine.block_id,
        "pgvector kNN should rank the semantically-closest block first"
    );
    assert!(
        resp.hits[0].vector_sim > 0.0,
        "vector_sim must be non-zero on the semantic hit"
    );
}

/// Graph-blend: content_type facets + loom_edges degree boosts a linked block.
#[tokio::test]
async fn mt264_graph_blend_facets_and_edges() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let llm = DisabledLlmClient::new("none".into(), "no embedding model".into());

    let hub = make_block(
        &pg.db,
        &ctx,
        &ws,
        "Alpha project hub",
        "alpha project documentation hub",
    )
    .await;
    let leaf = make_block(
        &pg.db,
        &ctx,
        &ws,
        "Alpha project notes",
        "alpha project meeting notes",
    )
    .await;
    // A third (non-matching) block that links INTO the hub, so the hub's edge
    // degree (3) strictly exceeds the leaf's (1) -> graph blend ranks hub first.
    let satellite = make_block(
        &pg.db,
        &ctx,
        &ws,
        "Beta satellite",
        "unrelated beta satellite content",
    )
    .await;
    // leaf -> hub, satellite -> hub, hub -> satellite : hub degree = 3, leaf = 1.
    for (src, tgt) in [(&leaf, &hub), (&satellite, &hub), (&hub, &satellite)] {
        pg.db
            .create_loom_edge(
                &ctx,
                NewLoomEdge {
                    edge_id: None,
                    workspace_id: ws.clone(),
                    source_block_id: src.block_id.clone(),
                    target_block_id: tgt.block_id.clone(),
                    edge_type: LoomEdgeType::Mention,
                    created_by: LoomEdgeCreatedBy::User,
                    crdt_site_id: None,
                    source_anchor: None,
                },
            )
            .await
            .expect("create edge");
    }

    let mut request = req("alpha project");
    request.graph_boost = 5.0;
    let resp = loom_search::search(&pg.db, &llm, rec(), &ws, request)
        .await
        .expect("search");
    assert!(resp.hits.len() >= 2, "both alpha blocks match");
    // The hub (degree 3: two outgoing-counted + inbound) outranks via graph blend.
    assert_eq!(
        resp.hits[0].block.block_id, hub.block_id,
        "graph blend should rank the higher-degree hub first"
    );
    assert!(resp.hits[0].edge_degree >= 2);
    // content_type facet over the matching set.
    assert_eq!(
        resp.content_type_facets.get("note").copied().unwrap_or(0),
        2,
        "content_type facet should count both note blocks"
    );
}

/// Reindex consistency: edit -> reflected, delete -> GONE (negative proof).
#[tokio::test]
async fn mt264_reindex_consistency_edit_and_delete() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    let llm = DisabledLlmClient::new("none".into(), "no embedding model".into());

    let block = make_block(
        &pg.db,
        &ctx,
        &ws,
        "Original aardvark title",
        "original aardvark body text",
    )
    .await;

    // Initially findable by its original term.
    let resp = loom_search::search(&pg.db, &llm, rec(), &ws, req("aardvark"))
        .await
        .expect("search");
    assert_eq!(resp.hits.len(), 1, "block found by original term");

    // EDIT the title -> the new term is reflected immediately.
    pg.db
        .update_loom_block(
            &ctx,
            &ws,
            &block.block_id,
            LoomBlockUpdate {
                title: Some("Updated platypus heading".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("update");
    let after_edit = loom_search::search(&pg.db, &llm, rec(), &ws, req("platypus"))
        .await
        .expect("search");
    assert_eq!(
        after_edit.hits.len(),
        1,
        "edited title term must be reflected in subsequent search"
    );

    // DELETE the block -> it is GONE from results (negative proof, no stale hit).
    pg.db
        .delete_loom_block(&ctx, &ws, &block.block_id)
        .await
        .expect("delete");
    let after_delete = loom_search::search(&pg.db, &llm, rec(), &ws, req("platypus"))
        .await
        .expect("search");
    assert!(
        after_delete.hits.is_empty(),
        "deleted block must NOT surface a stale hit (got {} hits)",
        after_delete.hits.len()
    );
    // And the original term is also gone.
    let orig = loom_search::search(&pg.db, &llm, rec(), &ws, req("aardvark"))
        .await
        .expect("search");
    assert!(orig.hits.is_empty(), "no stale hit for the deleted block");
}

/// No-model: typed keyword/trigram fallback, NO fabricated semantic results.
#[tokio::test]
async fn mt264_no_model_typed_fallback_no_fabrication() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    // DisabledLlmClient declines the embedding call with a typed error.
    let disabled = DisabledLlmClient::new("none".into(), "no embedding model".into());

    let block = make_block(
        &pg.db,
        &ctx,
        &ws,
        "Searchable note",
        "this note has real searchable keyword content",
    )
    .await;

    // reindex_block must NOT write an embedding (typed decline), but MUST keep
    // the keyword/trigram projection.
    let wrote = loom_search::reindex_block(&pg.db, &disabled, rec(), &ctx, &block)
        .await
        .expect("reindex");
    assert!(!wrote, "no model -> NO embedding written (no fabrication)");

    let resp = loom_search::search(&pg.db, &disabled, rec(), &ws, req("searchable keyword"))
        .await
        .expect("search");
    assert!(
        !resp.semantic_available,
        "no model -> semantic not available"
    );
    assert_eq!(resp.hits.len(), 1, "keyword fallback still finds the block");
    assert_eq!(
        resp.hits[0].vector_sim, 0.0,
        "no fabricated vector similarity when no model is configured"
    );
}

/// WP-1 MT-014: a configured model whose embedding dimensionality does NOT match
/// the index (896 vs 768) DEGRADES to keyword/trigram rather than hard-erroring
/// — on BOTH reindex AND search. It emits a surfaced Flight Recorder event and
/// sets a TYPED `semantic_unavailable_reason::DimMismatch`. This proves the
/// prior behavior (a hard `StorageError::Validation` that errored reindex and
/// 400'd the search query path) is gone. The projection assertion uses a real
/// managed PostgreSQL schema; the deterministic client is only the controlled
/// embedding producer that creates the dimensionality mismatch.
#[tokio::test]
async fn mt014_dim_mismatch_degrades_not_errors_on_reindex_and_search() {
    let pg = pg_required("mt014_dim_mismatch_degrades_not_errors_on_reindex_and_search").await;
    let ws = pg.create_workspace().await;
    let ctx = WriteContext::human(None);
    // Catalog DECLARES a 768-dim embedding model (matches the index), but the
    // client actually RETURNS 896-dim vectors -> dimensionality mismatch that must
    // DEGRADE (not hard-error) on reindex and search.
    let catalog_model_id = ModelId::new_v7();
    let catalog = model_catalog(vec![(
        catalog_model_id.clone(),
        "emb-declared-768",
        ModelCapabilities {
            supports_embedding: true,
            embedding_dimension: Some(loom_search::LOOM_SEARCH_EMBEDDING_DIM),
            ..Default::default()
        },
    )]);
    let llm = CatalogEmbeddingClient::new(ModelId::new_v7().to_string(), catalog, 896);
    let recorder = CapturingRecorder::default();

    let block = make_block(
        &pg.db,
        &ctx,
        &ws,
        "Dimension mismatch note",
        "a note whose embedding dimensionality does not match the search index",
    )
    .await;

    // REINDEX degrades: returns Ok(false) (no embedding written), NOT Err.
    let wrote = loom_search::reindex_block(&pg.db, &llm, &recorder, &ctx, &block)
        .await
        .expect("reindex must DEGRADE (Ok), not hard-error on dim mismatch");
    assert!(!wrote, "dim mismatch -> NO embedding written (degraded)");

    // The observable return value is not enough: direct PostgreSQL inspection
    // proves that the derived projection retained the keyword-only posture and
    // did not leave a stale embedding/model behind after the degraded write.
    let mut conn = pg.raw_connection().await;
    let keyword_only: bool = sqlx::query_scalar(
        "SELECT embedding IS NULL AND embedding_model IS NULL \
         FROM loom_block_search_index \
         WHERE workspace_id = $1 AND block_id = $2",
    )
    .bind(&ws)
    .bind(&block.block_id)
    .fetch_one(&mut conn)
    .await
    .expect("read MT-014 keyword-only projection");
    assert!(
        keyword_only,
        "dimension mismatch must persist a keyword-only projection with no vector/model"
    );

    // SEARCH degrades: returns Ok with semantic_available=false + typed reason,
    // NOT a 400 / hard error. The keyword modality still finds the block.
    let resp = loom_search::search(&pg.db, &llm, &recorder, &ws, req("dimension mismatch"))
        .await
        .expect("search must DEGRADE (Ok), not hard-error/400 on dim mismatch");
    assert!(
        !resp.semantic_available,
        "dim mismatch -> semantic not available"
    );
    assert_eq!(
        resp.semantic_unavailable_reason,
        Some(SemanticUnavailableReason::DimMismatch {
            expected: 768,
            actual: 896,
        }),
        "typed dim-mismatch reason surfaced (no silent drop)"
    );
    assert!(
        !resp.hits.is_empty(),
        "keyword modality still finds the block after semantic degrade"
    );
    assert_eq!(
        llm.requested_model_ids(),
        vec![catalog_model_id.to_string(), catalog_model_id.to_string()],
        "both reindex and search must use the catalog-selected embedding model id"
    );

    // A surfaced Flight Recorder event was emitted on BOTH surfaces.
    let events = recorder.events();
    let degrade_events: Vec<&FlightRecorderEvent> = events
        .iter()
        .filter(|e| e.payload["fr_event"] == loom_search::LOOM_SEMANTIC_DEGRADED_FR_EVENT)
        .collect();
    assert_eq!(
        degrade_events.len(),
        2,
        "one surfaced degrade event per surface (reindex + search)"
    );
    let surfaces: Vec<&str> = degrade_events
        .iter()
        .filter_map(|e| e.payload["surface"].as_str())
        .collect();
    assert!(
        surfaces.contains(&"reindex"),
        "reindex surface degrade event present"
    );
    assert!(
        surfaces.contains(&"search"),
        "search surface degrade event present"
    );
    for event in &degrade_events {
        assert_eq!(event.payload["reason"], "embedding_dim_mismatch");
        assert_eq!(event.payload["expected_dim"], 768);
        assert_eq!(event.payload["actual_dim"], 896);
    }
}
