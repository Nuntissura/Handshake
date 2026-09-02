use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use handshake_core::{
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    llm::{
        CompletionRequest, CompletionResponse, EmbeddingRequest, EmbeddingResponse, LlmClient,
        LlmError, ModelProfile, TokenUsage,
    },
    loom_search,
    model_runtime::{
        BaseModelTag, ModelCapabilities, ModelId, ModelRegistration, ModelRuntimeRole, OperatorId,
        ProviderKind, RoleBoundModelRegistration, RuntimeBinding,
    },
    storage::{
        surreal::{bootstrap_schema, SurrealLoomSearchStore, SurrealStorage, SurrealStorageConfig},
        LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomSearchV2Request, PreviewStatus,
    },
    swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
        OwnerAccountId, WorkspaceScopeRef,
    },
};
use tempfile::TempDir;

struct NoChatEmbeddingClient {
    profile: ModelProfile,
}

impl NoChatEmbeddingClient {
    fn new() -> Self {
        Self {
            profile: ModelProfile::new(ModelId::new_v7().to_string(), 4096),
        }
    }
}

#[async_trait]
impl LlmClient for NoChatEmbeddingClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage::default(),
            latency_ms: 0,
        })
    }

    async fn embedding(&self, _req: EmbeddingRequest) -> Result<EmbeddingResponse, LlmError> {
        panic!("chat model must not be called as the MT-016 embedding fallback")
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

fn scope(workspace_id: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(workspace_id).expect("valid workspace"),
    }
}

fn embedding_registration(role: ModelRuntimeRole) -> RoleBoundModelRegistration {
    RoleBoundModelRegistration {
        registration: ModelRegistration {
            model_id: ModelId::new_v7(),
            artifact_path: PathBuf::from("fixtures/mt016/dedicated-embedding.safetensors"),
            sha256: [0x16; 32],
            runtime_binding: RuntimeBinding::Candle,
            declared_capabilities: ModelCapabilities {
                supports_embedding: true,
                embedding_dimension: Some(768),
                ..ModelCapabilities::default()
            },
            base_model_tag: BaseModelTag::new("mt016-dedicated-embedding"),
            registered_at_utc: Utc::now(),
            registered_by: OperatorId::new("mt016-test"),
            provider: ProviderKind::Local,
        },
        runtime_role: role,
    }
}

fn block(workspace_id: &str, block_id: &str, text: &str) -> LoomBlock {
    LoomBlock {
        block_id: block_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        content_type: LoomBlockContentType::Note,
        document_id: None,
        asset_id: None,
        title: Some(text.to_owned()),
        original_filename: None,
        content_hash: Some(format!("hash-{block_id}")),
        pinned: false,
        favorite: false,
        pin_order: None,
        journal_date: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        imported_at: None,
        derived: LoomBlockDerived {
            full_text_index: Some(text.to_owned()),
            preview_status: PreviewStatus::None,
            ..LoomBlockDerived::default()
        },
    }
}

async fn open_store(config: SurrealStorageConfig) -> (SurrealStorage, SurrealLoomSearchStore) {
    let storage = SurrealStorage::open(config)
        .await
        .expect("open embedded SurrealDB");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap canonical schema");
    let store = SurrealLoomSearchStore::open(storage.clone())
        .await
        .expect("bootstrap MT-016 schema");
    (storage, store)
}

#[tokio::test]
async fn mt016_surreal_registration_index_search_are_idempotent_and_restart_stable() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "mt016",
        "restart_stable",
    )
    .expect("config");
    let exact = scope("WS-MT016-A");
    let (storage, store) = open_store(config.clone()).await;
    store
        .ensure_workspace_fixture(exact.workspace_id.as_str())
        .await
        .expect("workspace");

    let denied = store
        .register_embedding_model(
            &exact,
            &embedding_registration(ModelRuntimeRole::Completion),
        )
        .await;
    assert!(
        denied.is_err(),
        "chat/completion registration must never become embedding authority"
    );

    let registration = embedding_registration(ModelRuntimeRole::Embedding);
    let first = store
        .register_embedding_model(&exact, &registration)
        .await
        .expect("register");
    let retry = store
        .register_embedding_model(&exact, &registration)
        .await
        .expect("retry");
    assert!(first.changed);
    assert!(!retry.changed);
    assert_eq!(first.embedding_space_id, retry.embedding_space_id);
    assert_eq!(first.selection_event_id, retry.selection_event_id);

    let source = block(
        exact.workspace_id.as_str(),
        "BLOCK-MT016-A",
        "alpha semantic needle",
    );
    store
        .upsert_block_fixture(&exact, &source)
        .await
        .expect("source");
    let vector = vec![0.25_f32; 768];
    let indexed = store
        .reindex_block(&exact, &source, Some(&first), Some(vector.clone()))
        .await
        .expect("index");
    let index_retry = store
        .reindex_block(&exact, &source, Some(&first), Some(vector.clone()))
        .await
        .expect("index retry");
    assert!(indexed.changed);
    assert!(!index_retry.changed);
    assert_eq!(indexed.index_event_id, index_retry.index_event_id);

    let request = LoomSearchV2Request {
        query: "semantic needle".to_owned(),
        query_embedding: Some(vector),
        query_embedding_model: Some(first.embedding_space_id.clone()),
        limit: 20,
        ..LoomSearchV2Request::default()
    };
    let search = store.search(&exact, &request).await.expect("search");
    let search_retry = store.search(&exact, &request).await.expect("search retry");
    assert!(search.changed);
    assert!(!search_retry.changed);
    assert_eq!(search.trace_id, search_retry.trace_id);
    assert_eq!(search.receipt_event_id, search_retry.receipt_event_id);
    assert_eq!(search.response.hits[0].block.block_id, source.block_id);
    assert!(search.response.hits[0].vector_sim > 0.99);

    drop(store);
    storage.shutdown().await.expect("close first boot");
    let (storage, store) = open_store(config).await;
    let recovered = store
        .resolve_embedding_model(&exact)
        .await
        .expect("resolve")
        .expect("selection");
    assert_eq!(recovered.embedding_space_id, first.embedding_space_id);
    assert_eq!(recovered.selection_event_id, first.selection_event_id);
    storage.shutdown().await.expect("close second boot");
}

#[tokio::test]
async fn mt016_surreal_scope_lifecycle_and_orphan_receipts_fail_closed() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "mt016",
        "negative_paths",
    )
    .expect("config");
    let exact = scope("WS-MT016-B");
    let cross_scope = scope("WS-MT016-B");
    let (storage, store) = open_store(config).await;
    store
        .ensure_workspace_fixture(exact.workspace_id.as_str())
        .await
        .expect("workspace");
    let registration = store
        .register_embedding_model(&exact, &embedding_registration(ModelRuntimeRole::Embedding))
        .await
        .expect("register");
    let source = block(exact.workspace_id.as_str(), "BLOCK-MT016-B", "scope secret");
    store
        .upsert_block_fixture(&exact, &source)
        .await
        .expect("source");
    let vector = vec![0.5_f32; 768];
    let mutation = store
        .reindex_block(&exact, &source, Some(&registration), Some(vector.clone()))
        .await
        .expect("index");
    let request = LoomSearchV2Request {
        query: "scope secret".to_owned(),
        query_embedding: Some(vector.clone()),
        query_embedding_model: Some(registration.embedding_space_id.clone()),
        limit: 20,
        ..LoomSearchV2Request::default()
    };

    let cross = store
        .search(
            &cross_scope,
            &LoomSearchV2Request {
                query: request.query.clone(),
                limit: 20,
                ..LoomSearchV2Request::default()
            },
        )
        .await
        .expect("exact-scope empty result");
    assert!(
        cross.response.hits.is_empty(),
        "cross-scope source/index must not widen"
    );

    store
        .set_index_lifecycle_fixture(&source.block_id, "stale")
        .await
        .expect("stale index");
    let stale = store
        .search(
            &exact,
            &LoomSearchV2Request {
                query: "scope secret stale".to_owned(),
                limit: 20,
                ..LoomSearchV2Request::default()
            },
        )
        .await
        .expect("stale filter");
    assert!(stale.response.hits.is_empty());
    store
        .set_index_lifecycle_fixture(&source.block_id, "active")
        .await
        .expect("reactivate index");

    let search = store.search(&exact, &request).await.expect("search");
    store
        .delete_search_result_set_fixture(&search.result_set_id)
        .await
        .expect("orphan search receipt");
    assert!(
        store.search(&exact, &request).await.is_err(),
        "receipt without result set must fail closed"
    );

    store
        .delete_index_mutation_fixture(&source.block_id)
        .await
        .expect("orphan index receipt");
    assert!(
        store
            .reindex_block(&exact, &source, Some(&registration), Some(vector))
            .await
            .is_err(),
        "receipt without index mutation must fail closed"
    );

    store
        .set_embedding_lifecycle_fixture(&registration.registry_row_id, "revoked")
        .await
        .expect("revoke");
    assert!(store
        .resolve_embedding_model(&exact)
        .await
        .expect("resolve revoked")
        .is_none());
    assert!(!mutation.index_event_id.is_empty());
    storage.shutdown().await.expect("close");
}

#[tokio::test]
async fn mt016_no_dedicated_embedding_selection_never_calls_chat_fallback() {
    let temp = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        temp.path().join("store"),
        "mt016",
        "no_chat_fallback",
    )
    .expect("config");
    let exact = scope("WS-MT016-C");
    let (storage, store) = open_store(config).await;
    store
        .ensure_workspace_fixture(exact.workspace_id.as_str())
        .await
        .expect("workspace");
    let source = block(
        exact.workspace_id.as_str(),
        "BLOCK-MT016-C",
        "keyword fallback",
    );
    store
        .upsert_block_fixture(&exact, &source)
        .await
        .expect("source");
    let llm = NoChatEmbeddingClient::new();
    let recorder = NoopRecorder;
    let wrote_vector = loom_search::reindex_block(&store, &llm, &recorder, &exact, &source)
        .await
        .expect("keyword-only index");
    assert!(!wrote_vector);
    let response = loom_search::search(
        &store,
        &llm,
        &recorder,
        &exact,
        LoomSearchV2Request {
            query: "keyword fallback".to_owned(),
            limit: 20,
            ..LoomSearchV2Request::default()
        },
    )
    .await
    .expect("keyword-only search");
    assert!(!response.semantic_available);
    assert_eq!(response.hits.len(), 1);
    storage.shutdown().await.expect("close");
}
