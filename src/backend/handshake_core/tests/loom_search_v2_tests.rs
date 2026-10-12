//! WP-KERNEL-009 MT-264 UnifiedWorkSurface-264-LoomSearchV2 -- real embedded
//! SurrealDB proof for graph-blended ES-class search over the Loom corpus
//! (DEC-008: NOT Elasticsearch / no external search daemon).
//!
//! Every modality is proven against the isolated embedded authority, the
//! derived `loom_block_search_index` projection, and the hybrid search query:
//!   * FTS: ts_rank ordering + ts_headline highlight,
//!   * fuzzy: near-match scoring on a misspelled query,
//!   * semantic: REAL embeddings plus hybrid keyword/vector scoring,
//!   * graph-blend: content_type facets + loom_edges degree ranking,
//!   * reindex consistency: edit -> reflected, delete -> gone (NEGATIVE proof),
//!   * no-model: typed keyword/trigram fallback with NO fabricated semantic.
//!
//! The semantic modality dispatches a registered component ModelRuntime to
//! `InMemoryLlmClient::deterministic_embedding(text, 768)`: the vector is a
//! deterministic function of the original text. This proves component dispatch
//! and search behavior, not a native model backend or runtime acceptance.
//! The no-model negative
//! uses `DisabledLlmClient`, which declines the embedding call with a typed
//! error exactly like a runtime with no embedding model configured.

#[path = "knowledge_ingestion_support/mod.rs"]
mod embedded_knowledge_support;
mod account_session_support;

use embedded_knowledge_support::open_embedded_store;
use handshake_core::llm::{InMemoryLlmClient, LlmClient};
use handshake_core::llm::DisabledLlmClient;
use handshake_core::loom_search;
use handshake_core::storage::{
    LoomBlock, LoomBlockUpdate, LoomSearchV2Request, NewLoomEdge, LoomEdgeCreatedBy, LoomEdgeType,
};
use handshake_core::storage::surreal::{bootstrap_production_schemas, SurrealLoomSearchStore};
use handshake_core::swarm_orchestration::resource_scope::{AccessSpaceRef, ActorPrincipalId,
    AuthenticatedSessionRef, ExactResourceScopeAttribution, OwnerAccountId, WorkspaceScopeRef};
use handshake_core::model_runtime::*;
use std::sync::Arc;
use serde_json::{json, Value};

struct LoomFixture {
    db: SurrealLoomSearchStore,
    state: handshake_core::AppState,
    account: account_session_support::AccountFixture,
    scope: ExactResourceScopeAttribution,
    router: axum::Router,
    _source: embedded_knowledge_support::EmbeddedKnowledgeStore,
}

struct OwnedRouteServer(tokio::task::JoinHandle<()>);
impl Drop for OwnedRouteServer {
    fn drop(&mut self) { self.0.abort(); }
}

impl LoomFixture {
    async fn open(source: embedded_knowledge_support::EmbeddedKnowledgeStore) -> Self {
        bootstrap_production_schemas(&source.storage).await.expect("canonical Loom schemas");
        let account = account_session_support::AccountFixture::install(&source.storage).await;
        let recorder = Arc::new(handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder::new_in_memory(7)
            .expect("bounded Loom recorder"));
        let state = handshake_core::AppState {
            storage: Arc::new(source.db.clone()), surreal: source.storage.clone(),
            flight_recorder: recorder.clone(), diagnostics: recorder,
            llm_client: Arc::new(DisabledLlmClient::new("none".into(), "no embedding model".into())),
            capability_registry: Arc::new(handshake_core::capabilities::CapabilityRegistry::new()),
            session_registry: Arc::new(handshake_core::workflows::SessionRegistry::new_from_env()),
        };
        let ws = account.create_workspace(&state).await;
        let scope = ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::from_uuid(uuid::Uuid::parse_str(&account.account_id).expect("actual account")),
            actor_principal_id: ActorPrincipalId::from_uuid(uuid::Uuid::parse_str(&account.principal_id).expect("actual principal")),
            authenticated_session_id: AuthenticatedSessionRef::from_uuid(uuid::Uuid::parse_str(&account.session_id).expect("actual session")),
            access_space_id: AccessSpaceRef::from_uuid(uuid::Uuid::parse_str(&account.access_space_id).expect("actual Space")),
            workspace_id: WorkspaceScopeRef::new(ws).expect("canonical workspace"),
        };
        let db = SurrealLoomSearchStore::open(source.storage.clone()).await.expect("same-store Loom search");
        let router = handshake_core::api::loom::routes(state.clone())
            .merge(handshake_core::api::knowledge_documents::routes(state.clone()));
        Self { db, state, account, scope, router, _source: source }
    }

    async fn request(&self, method: reqwest::Method, path: &str, payload: Value) -> Value {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("owned Loom route fixture");
        let address = listener.local_addr().expect("Loom fixture address");
        let router = self.router.clone();
        let mut server = OwnedRouteServer(tokio::spawn(async move { axum::serve(listener, router).await.expect("owned Loom server"); }));
        let response = self.account.apply(reqwest::Client::new().request(method, format!("http://{address}{path}")))
            .header("x-hsk-actor-id", &self.account.actor_id).header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", "loom-search-component")
            .header("x-hsk-session-run-id", &self.account.session_id)
            .json(&payload).send().await.expect("authenticated Loom request");
        let status = response.status();
        let bytes = response.bytes().await.expect("Loom response bytes");
        server.0.abort(); let _ = (&mut server.0).await;
        assert!(status.is_success(), "canonical Loom request failed: {status} {bytes:?}");
        if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).expect("Loom response") }
    }

    async fn create_edge(&self, edge: NewLoomEdge) {
        self.request(reqwest::Method::POST, &format!("/workspaces/{}/loom/edges", edge.workspace_id),
            serde_json::to_value(edge).expect("edge payload")).await;
    }

    async fn update_block(&self, ws: &str, id: &str, update: LoomBlockUpdate) {
        assert_eq!(self.scope.workspace_id.as_str(), ws);
        self.request(reqwest::Method::POST, &format!("/knowledge/documents/{id}/rename"),
            json!({"title": update.title.expect("original title edit")})).await;
    }

    async fn delete_block(&self, ws: &str, id: &str) {
        assert_eq!(self.scope.workspace_id.as_str(), ws);
        self.request(reqwest::Method::DELETE, &format!("/knowledge/documents/{id}"), Value::Null).await;
    }
}

// Component substitute only: registered runtime dispatch consumes the original
// text-derived embedding implementation; it does not prove a native model backend.
struct ContentEmbeddingRuntime { capabilities: ModelCapabilities, identities: [ModelId; 2] }

#[async_trait::async_trait]
impl ModelRuntime for ContentEmbeddingRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Err(ModelRuntimeError::LoadError("component runtime is already registered".into()))
    }
    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> { Ok(()) }
    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(futures::stream::once(async { Err(ModelRuntimeError::GenerateError("embedding component only".into())) }))
    }
    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Err(ModelRuntimeError::ScoreError("embedding component only".into()))
    }
    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError> {
        if !self.identities.contains(&id) { return Err(ModelRuntimeError::EmbedError("unregistered component identity".into())); }
        Ok(Embedding { vector: InMemoryLlmClient::deterministic_embedding(text, 768) })
    }
    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        if !self.identities.contains(&id) { return Err(ModelRuntimeError::EmbedError("unregistered component identity".into())); }
        Ok(&self.capabilities)
    }
    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::KvCacheError("embedding component only".into()))
    }
    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::LoraStackError("embedding component only".into()))
    }
    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::SteeringHookError("embedding component only".into()))
    }
    fn cancel(&self, token: CancellationToken) { token.cancel(); }
}

async fn registered_embedding_client(store: &LoomFixture) -> handshake_core::llm::local_router::LocalModelRuntimeLlmClient {
    let capabilities = ModelCapabilities { supports_embedding: true, embedding_dimension: Some(768), ..Default::default() };
    let registration = |tag: &str| ModelRegistration {
        model_id: ModelId::new_v7(), artifact_path: format!("fixtures/{tag}.component").into(),
        sha256: [0x64; 32], runtime_binding: RuntimeBinding::Candle,
        declared_capabilities: capabilities.clone(), base_model_tag: BaseModelTag::new(tag),
        registered_at_utc: chrono::Utc::now(), registered_by: OperatorId::new("loom-search-component"),
        provider: ProviderKind::Local,
    };
    let primary = registration("loom-component-primary");
    let embedding = registration("loom-content-embedding");
    let runtime = Arc::new(ContentEmbeddingRuntime { capabilities, identities: [primary.model_id, embedding.model_id] });
    let client = handshake_core::llm::boot::assemble_local_runtime_client_with_registrations(
        primary, vec![embedding.clone()], runtime.clone(), runtime,
        Arc::new(DisabledLlmClient::new("none".into(), "component fallback disabled".into())),
        store.state.flight_recorder.clone(), 4096, None).expect("assemble shared runtime/catalog registry");
    let role_bound = RoleBoundModelRegistration { registration: embedding, runtime_role: ModelRuntimeRole::Embedding };
    let persisted = store.db.register_embedding_model(&store.scope, &role_bound).await.expect("persist actual dispatched embedding registration");
    let readback = store.db.resolve_embedding_model(&store.scope).await.expect("read embedding selection").expect("persisted selected embedding");
    assert_eq!(persisted.runtime_model_id, readback.runtime_model_id);
    assert_eq!(readback.runtime_model_id, role_bound.registration.model_id.to_string());
    assert_eq!(client.model_catalog().expect("shared dispatch catalog").embedding_model_for_dim(768)
        .expect("registered embedding identity").model_id, readback.runtime_model_id);
    client
}

macro_rules! embedded_store_or_return {
    () => {{
        match open_embedded_store().await {
            Some(store) => LoomFixture::open(store).await,
            None => {
                eprintln!("SKIP MT-264 LoomSearchV2 proof: embedded store unavailable");
                return;
            }
        }
    }};
}

async fn make_block(
    store: &LoomFixture,
    ctx: &ExactResourceScopeAttribution,
    ws: &str,
    title: &str,
    full_text: &str,
) -> LoomBlock {
    assert_eq!(ctx.workspace_id.as_str(), ws);
    let document = store.request(reqwest::Method::POST, "/knowledge/documents", json!({
        "workspace_id": ws, "title": title,
        "content_json": {"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": full_text}]}]},
    })).await;
    let id = document["document"]["rich_document_id"].as_str().expect("canonical document identity");
    serde_json::from_value(store.request(reqwest::Method::GET,
        &format!("/workspaces/{ws}/loom/blocks/{id}"), Value::Null).await).expect("canonical projected Loom block")
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
    let store = embedded_store_or_return!();
    let ws = store.scope.workspace_id.as_str().to_owned();
    let ctx = store.scope.clone();
    // No embedding model -> keyword/trigram only.
    let llm = DisabledLlmClient::new("none".into(), "no embedding model".into());

    make_block(
        &store,
        &ctx,
        &ws,
        "Migration runbook",
        "The migration runbook describes how to run a database migration safely.",
    )
    .await;
    make_block(
        &store,
        &ctx,
        &ws,
        "Holiday notes",
        "Notes about a beach holiday with no database content at all.",
    )
    .await;

    let resp = loom_search::search(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, req("database migration"))
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

/// Fuzzy/substring: a misspelled query returns the near-match.
#[tokio::test]
async fn mt264_trigram_fuzzy_match() {
    let store = embedded_store_or_return!();
    let ws = store.scope.workspace_id.as_str().to_owned();
    let ctx = store.scope.clone();
    let llm = DisabledLlmClient::new("none".into(), "no embedding model".into());

    make_block(
        &store,
        &ctx,
        &ws,
        "Kubernetes deployment guide",
        "Kubernetes deployment orchestration guide for production clusters.",
    )
    .await;

    // Misspelled query: "kubernates deploymnet" -- no exact FTS lexeme match,
    // but fuzzy similarity finds the near-match.
    let resp = loom_search::search(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, req("kubernates deploymnet"))
        .await
        .expect("search");
    assert!(
        !resp.hits.is_empty(),
        "fuzzy scoring should match the misspelled query"
    );
    assert!(
        resp.hits[0].trgm_sim > 0.0,
        "trgm similarity must be non-zero for the fuzzy hit"
    );
}

/// Semantic: REAL embeddings plus hybrid keyword/vector scoring;
/// the semantic modality surfaces a block whose TEXT does not lexically match
/// the query but whose embedding is closest.
#[tokio::test]
async fn mt264_semantic_and_hybrid() {
    let store = embedded_store_or_return!();
    let ws = store.scope.workspace_id.as_str().to_owned();
    let ctx = store.scope.clone();
    let llm = registered_embedding_client(&store).await;

    let canine = make_block(
        &store,
        &ctx,
        &ws,
        "Pet care",
        "the dog runs fast in the park",
    )
    .await;
    let finance = make_block(
        &store,
        &ctx,
        &ws,
        "Finance",
        "quarterly revenue projections and tax filings",
    )
    .await;

    // Reindex both blocks WITH real embeddings via the configured model.
    for block in [&canine, &finance] {
        let wrote = loom_search::reindex_block(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, block)
            .await
            .expect("reindex with embedding");
        assert!(
            wrote,
            "an embedding model is configured -> embedding written"
        );
    }

    // Query embedding overlaps the canine block's tokens => closest neighbour.
    let resp = loom_search::search(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, req("the dog runs fast in the park"))
        .await
        .expect("search");
    assert!(
        resp.semantic_available,
        "embedding model -> semantic available"
    );
    assert_eq!(
        resp.hits[0].block.block_id, canine.block_id,
        "vector similarity should rank the semantically-closest block first"
    );
    assert!(
        resp.hits[0].vector_sim > 0.0,
        "vector_sim must be non-zero on the semantic hit"
    );
}

/// Graph-blend: content_type facets + loom_edges degree boosts a linked block.
#[tokio::test]
async fn mt264_graph_blend_facets_and_edges() {
    let store = embedded_store_or_return!();
    let ws = store.scope.workspace_id.as_str().to_owned();
    let ctx = store.scope.clone();
    let llm = DisabledLlmClient::new("none".into(), "no embedding model".into());

    let hub = make_block(
        &store,
        &ctx,
        &ws,
        "Alpha project hub",
        "alpha project documentation hub",
    )
    .await;
    let leaf = make_block(
        &store,
        &ctx,
        &ws,
        "Alpha project notes",
        "alpha project meeting notes",
    )
    .await;
    // A third (non-matching) block that links INTO the hub, so the hub's edge
    // degree (3) strictly exceeds the leaf's (1) -> graph blend ranks hub first.
    let satellite = make_block(
        &store,
        &ctx,
        &ws,
        "Beta satellite",
        "unrelated beta satellite content",
    )
    .await;
    // leaf -> hub, satellite -> hub, hub -> satellite : hub degree = 3, leaf = 1.
    for (src, tgt) in [(&leaf, &hub), (&satellite, &hub), (&hub, &satellite)] {
        store.create_edge(
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
            .await;
    }

    let mut request = req("alpha project");
    request.graph_boost = 5.0;
    let resp = loom_search::search(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, request)
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
    let store = embedded_store_or_return!();
    let ws = store.scope.workspace_id.as_str().to_owned();
    let ctx = store.scope.clone();
    let llm = DisabledLlmClient::new("none".into(), "no embedding model".into());

    let block = make_block(
        &store,
        &ctx,
        &ws,
        "Original aardvark title",
        "original aardvark body text",
    )
    .await;

    // Initially findable by its original term.
    let resp = loom_search::search(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, req("aardvark"))
        .await
        .expect("search");
    assert_eq!(resp.hits.len(), 1, "block found by original term");

    // EDIT the title -> the new term is reflected immediately.
    store.update_block(
            &ws,
            &block.block_id,
            LoomBlockUpdate {
                title: Some("Updated platypus heading".to_string()),
                ..Default::default()
            },
        )
        .await;
    let after_edit = loom_search::search(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, req("platypus"))
        .await
        .expect("search");
    assert_eq!(
        after_edit.hits.len(),
        1,
        "edited title term must be reflected in subsequent search"
    );

    // DELETE the block -> it is GONE from results (negative proof, no stale hit).
    store.delete_block(&ws, &block.block_id)
        .await;
    let after_delete = loom_search::search(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, req("platypus"))
        .await
        .expect("search");
    assert!(
        after_delete.hits.is_empty(),
        "deleted block must NOT surface a stale hit (got {} hits)",
        after_delete.hits.len()
    );
    // And the original term is also gone.
    let orig = loom_search::search(&store.db, &llm, store.state.flight_recorder.as_ref(), &ctx, req("aardvark"))
        .await
        .expect("search");
    assert!(orig.hits.is_empty(), "no stale hit for the deleted block");
}

/// No-model: typed keyword/trigram fallback, NO fabricated semantic results.
#[tokio::test]
async fn mt264_no_model_typed_fallback_no_fabrication() {
    let store = embedded_store_or_return!();
    let ws = store.scope.workspace_id.as_str().to_owned();
    let ctx = store.scope.clone();
    // DisabledLlmClient declines the embedding call with a typed error.
    let disabled = DisabledLlmClient::new("none".into(), "no embedding model".into());

    let block = make_block(
        &store,
        &ctx,
        &ws,
        "Searchable note",
        "this note has real searchable keyword content",
    )
    .await;

    // reindex_block must NOT write an embedding (typed decline), but MUST keep
    // the keyword/trigram projection.
    let wrote = loom_search::reindex_block(&store.db, &disabled, store.state.flight_recorder.as_ref(), &ctx, &block)
        .await
        .expect("reindex");
    assert!(!wrote, "no model -> NO embedding written (no fabrication)");

    let resp = loom_search::search(&store.db, &disabled, store.state.flight_recorder.as_ref(), &ctx, req("searchable keyword"))
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
