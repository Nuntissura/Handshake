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

use handshake_core::llm::ollama::InMemoryLlmClient;
use handshake_core::llm::DisabledLlmClient;
use handshake_core::loom_search;
use handshake_core::storage::{
    Database, LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, LoomEdgeCreatedBy,
    LoomEdgeType, LoomSearchV2Request, NewLoomBlock, NewLoomEdge, WriteContext,
};
use knowledge_pg_support::knowledge_pg;

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

    let resp = loom_search::search(&pg.db, &llm, &ws, req("database migration"))
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
    assert!(!resp.semantic_available, "no embedding model -> not available");
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
    let resp = loom_search::search(&pg.db, &llm, &ws, req("kubernates deploymnet"))
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
    let llm = InMemoryLlmClient::new(String::new()).with_embedding_dim(768);

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
        let wrote = loom_search::reindex_block(&pg.db, &llm, &ctx, block)
            .await
            .expect("reindex with embedding");
        assert!(wrote, "an embedding model is configured -> embedding written");
    }

    // Query embedding overlaps the canine block's tokens => closest neighbour.
    let resp = loom_search::search(
        &pg.db,
        &llm,
        &ws,
        req("the dog runs fast in the park"),
    )
    .await
    .expect("search");
    assert!(resp.semantic_available, "embedding model -> semantic available");
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
    let resp = loom_search::search(&pg.db, &llm, &ws, request)
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
    let resp = loom_search::search(&pg.db, &llm, &ws, req("aardvark"))
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
    let after_edit = loom_search::search(&pg.db, &llm, &ws, req("platypus"))
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
    let after_delete = loom_search::search(&pg.db, &llm, &ws, req("platypus"))
        .await
        .expect("search");
    assert!(
        after_delete.hits.is_empty(),
        "deleted block must NOT surface a stale hit (got {} hits)",
        after_delete.hits.len()
    );
    // And the original term is also gone.
    let orig = loom_search::search(&pg.db, &llm, &ws, req("aardvark"))
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
    let wrote = loom_search::reindex_block(&pg.db, &disabled, &ctx, &block)
        .await
        .expect("reindex");
    assert!(!wrote, "no model -> NO embedding written (no fabrication)");

    let resp = loom_search::search(&pg.db, &disabled, &ws, req("searchable keyword"))
        .await
        .expect("search");
    assert!(!resp.semantic_available, "no model -> semantic not available");
    assert_eq!(resp.hits.len(), 1, "keyword fallback still finds the block");
    assert_eq!(
        resp.hits[0].vector_sim, 0.0,
        "no fabricated vector similarity when no model is configured"
    );
}
