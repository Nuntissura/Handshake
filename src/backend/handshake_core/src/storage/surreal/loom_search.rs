use std::{collections::BTreeMap, str::FromStr};

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, SurrealValue};

use crate::{
    kernel::{KernelActor, KernelEvent, KernelEventType, NewKernelEvent},
    model_runtime::{ModelCapabilities, ModelRuntimeRole, RoleBoundModelRegistration},
    storage::{
        LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomSearchV2Hit, LoomSearchV2Request,
        LoomSearchV2Response, SemanticUnavailableReason, StorageError, StorageResult,
    },
    swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
};

use super::{SurrealStorage, SurrealStorageError};

const SCHEMA: &str = include_str!("loom_search_schema.surql");
const EMBEDDING_DIMENSION: usize = 768;
const MODEL_REGISTRY_SCHEMA_ID: &str = "hsk.model_runtime_registry.row@2";
const MODEL_CAPABILITIES_SCHEMA_ID: &str = "hsk.model_runtime.capabilities@1";

#[derive(Clone)]
pub struct SurrealLoomSearchStore {
    storage: SurrealStorage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurrealEmbeddingRegistration {
    pub registry_row_id: String,
    pub artifact_sha256: String,
    pub runtime_model_id: String,
    pub runtime_binding: String,
    pub runtime_role: ModelRuntimeRole,
    pub declared_capabilities: ModelCapabilities,
    pub base_model_tag: String,
    pub embedding_space_id: String,
    pub embedding_dimension: usize,
    pub lifecycle_state: String,
    pub selection_event_id: String,
    pub changed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurrealLoomIndexMutation {
    pub block_id: String,
    pub embedding_space_id: Option<String>,
    pub index_event_id: String,
    pub mutation_fingerprint: String,
    pub changed: bool,
}

#[derive(Clone, Debug)]
pub struct SurrealLoomSearchEvidence {
    pub response: LoomSearchV2Response,
    pub trace_id: String,
    pub result_set_id: String,
    pub receipt_event_id: String,
    pub query_fingerprint: String,
    pub result_set_fingerprint: String,
    pub changed: bool,
}

#[derive(Debug, SurrealValue)]
struct ExactScopeBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
}

#[derive(Debug, SurrealValue)]
struct KernelReceiptWrite {
    event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: Value,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct EmbeddingRegistrationContent {
    registry_row_id: String,
    schema_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    artifact_sha256: String,
    artifact_locator: String,
    last_observed_runtime_model_id: String,
    runtime_binding: String,
    runtime_role: String,
    capabilities_schema_id: String,
    capabilities: Value,
    provider: String,
    base_model_tag: String,
    embedding_space_id: String,
    embedding_dimension: i64,
    lifecycle_state: String,
    mutation_fingerprint: String,
    selection_event_id: RecordId,
    last_observed_at_utc: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct RegisterEmbeddingBindings {
    record: RecordId,
    content: EmbeddingRegistrationContent,
    event: KernelReceiptWrite,
}

#[derive(Debug, SurrealValue)]
struct StoredEmbeddingRegistration {
    registry_row_id: String,
    artifact_sha256: String,
    last_observed_runtime_model_id: String,
    runtime_binding: String,
    runtime_role: String,
    capabilities: Value,
    base_model_tag: String,
    embedding_space_id: String,
    embedding_dimension: i64,
    lifecycle_state: String,
    selection_event_id: String,
    mutation_fingerprint: String,
    changed: bool,
}

#[derive(Debug, SurrealValue)]
struct IndexContent {
    block_id: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    content_type: String,
    search_text: String,
    source_content_hash: String,
    embedding: Option<Vec<f32>>,
    embedding_model: Option<String>,
    embedding_dimension: Option<i64>,
    lifecycle_state: String,
    mutation_fingerprint: String,
    index_event_id: RecordId,
    indexed_at: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct ReindexBindings {
    block_record: RecordId,
    index_record: RecordId,
    content: IndexContent,
    event: KernelReceiptWrite,
}

#[derive(Debug, SurrealValue)]
struct StoredIndexMutation {
    block_id: String,
    embedding_model: Option<String>,
    index_event_id: String,
    mutation_fingerprint: String,
    changed: bool,
}

#[derive(Debug, SurrealValue)]
struct SearchRowsBindings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    query_embedding: Option<Vec<f32>>,
    query_embedding_model: Option<String>,
}

#[derive(Clone, Debug, SurrealValue)]
struct SearchIndexRow {
    block_id: String,
    workspace_id: String,
    content_type: String,
    document_id: Option<String>,
    asset_id: Option<String>,
    title: Option<String>,
    original_filename: Option<String>,
    content_hash: Option<String>,
    pinned: bool,
    favorite: bool,
    pin_order: Option<i64>,
    journal_date: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    imported_at: Option<DateTime<Utc>>,
    derived_json: Value,
    backlink_count: i64,
    mention_count: i64,
    tag_count: i64,
    preview_status: String,
    thumbnail_asset_id: Option<String>,
    proxy_asset_id: Option<String>,
    search_text: String,
    embedding: Option<Vec<f32>>,
    embedding_model: Option<String>,
    vector_sim: f64,
    mutation_fingerprint: String,
    index_event_id: String,
}

#[derive(Debug, SurrealValue)]
struct EdgeRow {
    source_block_id: String,
    target_block_id: String,
    edge_type: String,
}

#[derive(Debug, SurrealValue)]
struct SearchTraceContent {
    trace_id: String,
    workspace_id: RecordId,
    retrieval_mode: String,
    mode_reason: String,
    query_text: Option<String>,
    bundle_id: Option<RecordId>,
    decisions: Value,
    trace_receipt_event_id: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    query_fingerprint: String,
    result_set_fingerprint: String,
}

#[derive(Debug, SurrealValue)]
struct SearchResultSetContent {
    result_set_id: String,
    trace_id: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    query_fingerprint: String,
    result_set_fingerprint: String,
    semantic_available: bool,
    embedding_model: Option<String>,
    total: i64,
    receipt_event_id: RecordId,
}

#[derive(Debug, SurrealValue)]
struct SearchResultContent {
    result_id: String,
    result_set_id: RecordId,
    block_id: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: RecordId,
    result_set_fingerprint: String,
    ordinal: i64,
    score: f64,
    fts_rank: f64,
    trgm_sim: f64,
    vector_sim: f64,
    edge_degree: i64,
    highlight: String,
    source_index_event_id: RecordId,
}

#[derive(Debug, SurrealValue)]
struct PersistSearchBindings {
    trace_record: RecordId,
    result_set_record: RecordId,
    trace: SearchTraceContent,
    result_set: SearchResultSetContent,
    results: Vec<SearchResultContent>,
    event: KernelReceiptWrite,
}

#[derive(Debug, SurrealValue)]
struct StoredSearchEvidence {
    trace_id: String,
    result_set_id: String,
    receipt_event_id: String,
    changed: bool,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct LoomBlockFixtureContent {
    block_id: String,
    workspace_id: RecordId,
    content_type: String,
    title: Option<String>,
    content_hash: Option<String>,
    pinned: bool,
    favorite: bool,
    derived_json: Value,
    preview_status: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(feature = "test-utils")]
#[derive(Debug, SurrealValue)]
struct LifecycleFixtureBindings {
    record: RecordId,
    lifecycle_state: String,
}

struct ScoredSearch {
    response: LoomSearchV2Response,
    all_hits: Vec<(LoomSearchV2Hit, String, String)>,
}

const REGISTER_EMBEDDING_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT
    registry_row_id,
    artifact_sha256,
    last_observed_runtime_model_id,
    runtime_binding,
    runtime_role,
    capabilities,
    base_model_tag,
    embedding_space_id,
    embedding_dimension,
    lifecycle_state,
    record::id(selection_event_id) AS selection_event_id,
    mutation_fingerprint
FROM $record LIMIT 2);
LET $receipt = IF array::len($existing) = 1 {
    (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $existing[0].selection_event_id
       AND event_type = 'MODEL_RUNTIME_SELECTION_RECORDED'
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $existing[0].registry_row_id
       AND payload.mutation_fingerprint = $existing[0].mutation_fingerprint
       AND owner_account_id = $content.owner_account_id
       AND actor_principal_id = $content.actor_principal_id
       AND authenticated_session_id = $content.authenticated_session_id
       AND access_space_id = $content.access_space_id
       AND workspace_id = record::id($content.workspace_id)
     LIMIT 2)
} ELSE { [] };
LET $event_existing = (SELECT event_id FROM kernel_event_ledger
WHERE idempotency_key = $event.idempotency_key
  AND owner_account_id = $event.owner_account_id
  AND actor_principal_id = $event.actor_principal_id
  AND authenticated_session_id = $event.authenticated_session_id
  AND access_space_id = $event.access_space_id
  AND workspace_id = $event.workspace_id
LIMIT 2);
IF array::len($existing) > 1 OR array::len($receipt) > 1 OR array::len($event_existing) > 1 {
    THROW 'MT-016 embedding registration identity or receipt is ambiguous';
} ELSE IF array::len($existing) = 1 {
    IF $existing[0].mutation_fingerprint != $content.mutation_fingerprint {
        THROW 'MT-016 embedding registration immutable selection conflict';
    } ELSE IF array::len($receipt) != 1 {
        THROW 'MT-016 embedding registration has inconsistent receipt evidence';
    };
    RETURN SELECT registry_row_id, artifact_sha256,
        last_observed_runtime_model_id, runtime_binding, runtime_role,
        capabilities, base_model_tag, embedding_space_id,
        embedding_dimension, lifecycle_state,
        record::id(selection_event_id) AS selection_event_id,
        mutation_fingerprint, false AS changed FROM $record;
} ELSE {
    IF array::len($event_existing) != 0 {
        THROW 'MT-016 embedding receipt exists without registration mutation';
    };
    CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT $event;
    CREATE $record CONTENT $content;
    RETURN SELECT *, record::id(selection_event_id) AS selection_event_id, true AS changed FROM $record;
};
COMMIT TRANSACTION;
"#;

const RESOLVE_EMBEDDING_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $rows = (SELECT
    registry_row_id,
    artifact_sha256,
    last_observed_runtime_model_id,
    runtime_binding,
    runtime_role,
    capabilities,
    base_model_tag,
    embedding_space_id,
    embedding_dimension,
    lifecycle_state,
    record::id(selection_event_id) AS selection_event_id,
    mutation_fingerprint,
    false AS changed
FROM model_runtime_registry
WHERE owner_account_id = $owner_account_id
  AND actor_principal_id = $actor_principal_id
  AND authenticated_session_id = $authenticated_session_id
  AND access_space_id = $access_space_id
  AND workspace_id = $workspace_id
  AND runtime_role = 'embedding'
  AND embedding_dimension = 768
  AND lifecycle_state = 'active'
LIMIT 2);
LET $receipts = IF array::len($rows) = 1 {
    (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $rows[0].selection_event_id
       AND event_type = 'MODEL_RUNTIME_SELECTION_RECORDED'
       AND aggregate_type = 'model_runtime_registry'
       AND aggregate_id = $rows[0].registry_row_id
       AND payload.mutation_fingerprint = $rows[0].mutation_fingerprint
       AND owner_account_id = $owner_account_id
       AND actor_principal_id = $actor_principal_id
       AND authenticated_session_id = $authenticated_session_id
       AND access_space_id = $access_space_id
       AND workspace_id = record::id($workspace_id)
     LIMIT 2)
} ELSE { [] };
IF array::len($rows) > 1 OR array::len($receipts) > 1 {
    THROW 'MT-016 active embedding selection is ambiguous';
} ELSE IF array::len($rows) = 1 AND array::len($receipts) != 1 {
    THROW 'MT-016 active embedding selection has inconsistent receipt evidence';
};
RETURN $rows;
COMMIT TRANSACTION;
"#;

const REINDEX_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $source = (SELECT block_id, content_type, content_hash,
    owner_account_id, actor_principal_id, authenticated_session_id,
    access_space_id, workspace_id
FROM $block_record
WHERE owner_account_id = $content.owner_account_id
  AND actor_principal_id = $content.actor_principal_id
  AND authenticated_session_id = $content.authenticated_session_id
  AND access_space_id = $content.access_space_id
  AND workspace_id = $content.workspace_id
LIMIT 2);
LET $existing = (SELECT
    record::id(block_id) AS block_id,
    embedding_model,
    record::id(index_event_id) AS index_event_id,
    mutation_fingerprint
FROM $index_record LIMIT 2);
LET $receipt = IF array::len($existing) = 1 {
    (SELECT event_id FROM kernel_event_ledger
     WHERE event_id = $existing[0].index_event_id
       AND event_type = 'KNOWLEDGE_LOOM_BLOCK_INDEXED'
       AND aggregate_type = 'loom_block_search_index'
       AND aggregate_id = $existing[0].block_id
       AND payload.mutation_fingerprint = $existing[0].mutation_fingerprint
       AND owner_account_id = $content.owner_account_id
       AND actor_principal_id = $content.actor_principal_id
       AND authenticated_session_id = $content.authenticated_session_id
       AND access_space_id = $content.access_space_id
       AND workspace_id = record::id($content.workspace_id)
     LIMIT 2)
} ELSE { [] };
LET $event_existing = (SELECT event_id FROM kernel_event_ledger
WHERE idempotency_key = $event.idempotency_key
  AND owner_account_id = $event.owner_account_id
  AND actor_principal_id = $event.actor_principal_id
  AND authenticated_session_id = $event.authenticated_session_id
  AND access_space_id = $event.access_space_id
  AND workspace_id = $event.workspace_id
LIMIT 2);
IF array::len($source) != 1 {
    THROW 'MT-016 Loom source is absent, unattributed, or outside exact scope';
} ELSE IF array::len($existing) > 1 OR array::len($receipt) > 1 OR array::len($event_existing) > 1 {
    THROW 'MT-016 Loom index identity or receipt is ambiguous';
} ELSE IF array::len($existing) = 1
    AND $existing[0].mutation_fingerprint = $content.mutation_fingerprint {
    IF array::len($receipt) != 1 {
        THROW 'MT-016 Loom index has inconsistent receipt evidence';
    };
    RETURN SELECT record::id(block_id) AS block_id, embedding_model,
        record::id(index_event_id) AS index_event_id, mutation_fingerprint,
        false AS changed FROM $index_record;
} ELSE {
    IF array::len($event_existing) != 0 {
        THROW 'MT-016 Loom index receipt exists without its mutation';
    };
    CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT $event;
    UPSERT $index_record CONTENT $content;
    RETURN SELECT record::id(block_id) AS block_id, embedding_model,
        record::id(index_event_id) AS index_event_id, mutation_fingerprint,
        true AS changed FROM $index_record;
};
COMMIT TRANSACTION;
"#;

const SEARCH_ROWS_QUERY: &str = r#"
SELECT
    record::id(block_id) AS block_id,
    record::id(workspace_id) AS workspace_id,
    content_type,
    record::id(block_id.document_id) AS document_id,
    record::id(block_id.asset_id) AS asset_id,
    block_id.title AS title,
    block_id.original_filename AS original_filename,
    block_id.content_hash AS content_hash,
    block_id.pinned AS pinned,
    block_id.favorite AS favorite,
    block_id.pin_order AS pin_order,
    block_id.journal_date AS journal_date,
    block_id.created_at AS created_at,
    block_id.updated_at AS updated_at,
    block_id.imported_at AS imported_at,
    block_id.derived_json AS derived_json,
    block_id.backlink_count AS backlink_count,
    block_id.mention_count AS mention_count,
    block_id.tag_count AS tag_count,
    block_id.preview_status AS preview_status,
    record::id(block_id.thumbnail_asset_id) AS thumbnail_asset_id,
    record::id(block_id.proxy_asset_id) AS proxy_asset_id,
    search_text,
    embedding,
    embedding_model,
    IF $query_embedding != NONE
       AND embedding != NONE
       AND embedding_model = $query_embedding_model
       AND array::len(embedding) = 768
       AND array::len($query_embedding) = 768 {
        vector::similarity::cosine(embedding, $query_embedding)
    } ELSE { 0.0 } AS vector_sim,
    mutation_fingerprint,
    record::id(index_event_id) AS index_event_id
FROM loom_block_search_index
WHERE owner_account_id = $owner_account_id
  AND actor_principal_id = $actor_principal_id
  AND authenticated_session_id = $authenticated_session_id
  AND access_space_id = $access_space_id
  AND workspace_id = $workspace_id
  AND lifecycle_state = 'active'
  AND block_id.owner_account_id = $owner_account_id
  AND block_id.actor_principal_id = $actor_principal_id
  AND block_id.authenticated_session_id = $authenticated_session_id
  AND block_id.access_space_id = $access_space_id
  AND block_id.workspace_id = $workspace_id
  AND index_event_id.event_type = 'KNOWLEDGE_LOOM_BLOCK_INDEXED'
  AND index_event_id.aggregate_type = 'loom_block_search_index'
  AND index_event_id.aggregate_id = record::id(block_id)
  AND index_event_id.payload.mutation_fingerprint = mutation_fingerprint
  AND index_event_id.owner_account_id = $owner_account_id
  AND index_event_id.actor_principal_id = $actor_principal_id
  AND index_event_id.authenticated_session_id = $authenticated_session_id
  AND index_event_id.access_space_id = $access_space_id
  AND index_event_id.workspace_id = record::id($workspace_id);
"#;

const SEARCH_EDGES_QUERY: &str = r#"
SELECT record::id(source_block_id) AS source_block_id,
       record::id(target_block_id) AS target_block_id,
       edge_type
FROM loom_edges
WHERE workspace_id = $workspace_id
  AND source_block_id.owner_account_id = $owner_account_id
  AND source_block_id.actor_principal_id = $actor_principal_id
  AND source_block_id.authenticated_session_id = $authenticated_session_id
  AND source_block_id.access_space_id = $access_space_id
  AND source_block_id.workspace_id = $workspace_id
  AND target_block_id.owner_account_id = $owner_account_id
  AND target_block_id.actor_principal_id = $actor_principal_id
  AND target_block_id.authenticated_session_id = $authenticated_session_id
  AND target_block_id.access_space_id = $access_space_id
  AND target_block_id.workspace_id = $workspace_id;
"#;

#[cfg(feature = "test-utils")]
const SET_LIFECYCLE_FIXTURE_QUERY: &str =
    "UPDATE $record SET lifecycle_state = $lifecycle_state RETURN AFTER;";

impl SurrealLoomSearchStore {
    pub fn new(storage: SurrealStorage) -> Self {
        Self { storage }
    }

    pub async fn open(storage: SurrealStorage) -> StorageResult<Self> {
        bootstrap_loom_search_schema(&storage).await?;
        Ok(Self::new(storage))
    }

    pub fn storage(&self) -> &SurrealStorage {
        &self.storage
    }

    #[cfg(feature = "test-utils")]
    pub async fn ensure_workspace_fixture(&self, workspace_id: &str) -> StorageResult<()> {
        #[derive(Debug, SurrealValue)]
        struct WorkspaceFixture {
            name: String,
            updated_at: DateTime<Utc>,
        }
        if workspace_id.trim().is_empty() || workspace_id.trim() != workspace_id {
            return Err(StorageError::Validation(
                "MT-016 fixture workspace id is invalid",
            ));
        }
        let workspace_id = workspace_id.to_owned();
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .upsert_one::<Value, _>(
                            "workspaces",
                            &workspace_id,
                            WorkspaceFixture {
                                name: "MT-016 embedded Loom search fixture".to_owned(),
                                updated_at: Utc::now(),
                            },
                        )
                        .await
                })
            })
            .await?;
        Ok(())
    }

    #[cfg(feature = "test-utils")]
    pub async fn upsert_block_fixture(
        &self,
        scope: &ExactResourceScopeAttribution,
        block: &LoomBlock,
    ) -> StorageResult<()> {
        validate_block_scope(scope, block)?;
        let exact = exact_scope_bindings(scope);
        let block_id = block.block_id.clone();
        let derived_json = serde_json::to_value(&block.derived)?;
        let content = LoomBlockFixtureContent {
            block_id: block_id.clone(),
            workspace_id: exact.workspace_id,
            content_type: block.content_type.as_str().to_owned(),
            title: block.title.clone(),
            content_hash: block.content_hash.clone(),
            pinned: block.pinned,
            favorite: block.favorite,
            derived_json,
            preview_status: block.derived.preview_status.as_str().to_owned(),
            owner_account_id: exact.owner_account_id,
            actor_principal_id: exact.actor_principal_id,
            authenticated_session_id: exact.authenticated_session_id,
            access_space_id: exact.access_space_id,
            created_at: block.created_at,
            updated_at: block.updated_at,
        };
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .upsert_one::<Value, _>("loom_blocks", &block_id, content)
                        .await
                })
            })
            .await?;
        Ok(())
    }

    #[cfg(feature = "test-utils")]
    pub async fn set_embedding_lifecycle_fixture(
        &self,
        registry_row_id: &str,
        lifecycle_state: &str,
    ) -> StorageResult<()> {
        set_lifecycle_fixture(
            &self.storage,
            "model_runtime_registry",
            registry_row_id,
            lifecycle_state,
        )
        .await
    }

    #[cfg(feature = "test-utils")]
    pub async fn set_index_lifecycle_fixture(
        &self,
        block_id: &str,
        lifecycle_state: &str,
    ) -> StorageResult<()> {
        set_lifecycle_fixture(
            &self.storage,
            "loom_block_search_index",
            block_id,
            lifecycle_state,
        )
        .await
    }

    #[cfg(feature = "test-utils")]
    pub async fn delete_index_mutation_fixture(&self, block_id: &str) -> StorageResult<()> {
        let block_id = block_id.to_owned();
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .delete_one::<Value>("loom_block_search_index", &block_id)
                        .await
                })
            })
            .await?;
        Ok(())
    }

    #[cfg(feature = "test-utils")]
    pub async fn delete_search_result_set_fixture(&self, result_set_id: &str) -> StorageResult<()> {
        let result_set_id = result_set_id.to_owned();
        self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .delete_one::<Value>("loom_search_result_sets", &result_set_id)
                        .await
                })
            })
            .await?;
        Ok(())
    }

    pub async fn register_embedding_model(
        &self,
        scope: &ExactResourceScopeAttribution,
        registration: &RoleBoundModelRegistration,
    ) -> StorageResult<SurrealEmbeddingRegistration> {
        validate_embedding_registration(registration)?;
        let artifact_sha256 = hex::encode(registration.registration.sha256);
        let embedding_dimension = registration
            .registration
            .declared_capabilities
            .embedding_dimension
            .expect("validated embedding dimension");
        let embedding_space_id = format!(
            "EMS-{}",
            sha256_hex(&format!("{artifact_sha256}\0{embedding_dimension}"))
        );
        let registry_row_id = format!(
            "MRR-{}",
            sha256_hex(&format!(
                "{}\0{}\0{}\0{}\0{}\0{}",
                scope.owner_account_id,
                scope.actor_principal_id,
                scope.authenticated_session_id,
                scope.access_space_id,
                scope.workspace_id.as_str(),
                embedding_space_id
            ))
        );
        let capabilities = serde_json::to_value(&registration.registration.declared_capabilities)?;
        let provider = serde_json::to_value(registration.registration.provider)?
            .as_str()
            .ok_or(StorageError::Validation(
                "model provider serialization is invalid",
            ))?
            .to_owned();
        let mutation_fingerprint = sha256_json(&json!({
            "scope": scope_json(scope),
            "artifact_sha256": artifact_sha256,
            "runtime_binding": registration.registration.runtime_binding.adapter_id(),
            "runtime_role": registration.runtime_role.as_str(),
            "capabilities": capabilities,
            "base_model_tag": registration.registration.base_model_tag.as_str(),
            "embedding_space_id": embedding_space_id,
            "embedding_dimension": embedding_dimension,
        }))?;
        let event = receipt_write(
            receipt_event(
                scope,
                KernelEventType::ModelRuntimeSelectionRecorded,
                "model_runtime_registry",
                &registry_row_id,
                "loom_search::embedding_registry",
                &mutation_fingerprint,
                json!({
                    "action": "embedding_model_registered",
                    "artifact_sha256": artifact_sha256,
                    "embedding_space_id": embedding_space_id,
                    "embedding_dimension": embedding_dimension,
                    "mutation_fingerprint": mutation_fingerprint,
                }),
            )?,
            scope,
        );
        let exact = exact_scope_bindings(scope);
        let artifact_locator = format!("artifact://sha256/{artifact_sha256}");
        let last_observed_runtime_model_id = registration.registration.model_id.to_string();
        let runtime_binding = registration
            .registration
            .runtime_binding
            .adapter_id()
            .to_owned();
        let runtime_role = registration.runtime_role.as_str().to_owned();
        let base_model_tag = registration
            .registration
            .base_model_tag
            .as_str()
            .to_owned();
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredEmbeddingRegistration, _>(
                            REGISTER_EMBEDDING_QUERY,
                            RegisterEmbeddingBindings {
                                record: RecordId::new(
                                    "model_runtime_registry",
                                    registry_row_id.clone(),
                                ),
                                content: EmbeddingRegistrationContent {
                                    registry_row_id,
                                    schema_id: MODEL_REGISTRY_SCHEMA_ID.to_owned(),
                                    owner_account_id: exact.owner_account_id,
                                    actor_principal_id: exact.actor_principal_id,
                                    authenticated_session_id: exact.authenticated_session_id,
                                    access_space_id: exact.access_space_id,
                                    workspace_id: exact.workspace_id,
                                    artifact_sha256,
                                    artifact_locator,
                                    last_observed_runtime_model_id,
                                    runtime_binding,
                                    runtime_role,
                                    capabilities_schema_id: MODEL_CAPABILITIES_SCHEMA_ID.to_owned(),
                                    capabilities,
                                    provider,
                                    base_model_tag,
                                    embedding_space_id,
                                    embedding_dimension: embedding_dimension as i64,
                                    lifecycle_state: "active".to_owned(),
                                    mutation_fingerprint,
                                    selection_event_id: RecordId::new(
                                        "kernel_event_ledger",
                                        event.event_id.clone(),
                                    ),
                                    last_observed_at_utc: Utc::now(),
                                },
                                event,
                            },
                            4,
                        )
                        .await
                })
            })
            .await?;
        stored_embedding(one(rows, "embedding registration")?)
    }

    pub async fn resolve_embedding_model(
        &self,
        scope: &ExactResourceScopeAttribution,
    ) -> StorageResult<Option<SurrealEmbeddingRegistration>> {
        let exact = exact_scope_bindings(scope);
        let mut rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredEmbeddingRegistration, _>(
                            RESOLVE_EMBEDDING_QUERY,
                            exact,
                            4,
                        )
                        .await
                })
            })
            .await?;
        if rows.len() > 1 {
            return Err(StorageError::Conflict(
                "active embedding selection is ambiguous",
            ));
        }
        rows.pop().map(stored_embedding).transpose()
    }

    pub async fn reindex_block(
        &self,
        scope: &ExactResourceScopeAttribution,
        block: &LoomBlock,
        embedding_model: Option<&SurrealEmbeddingRegistration>,
        embedding: Option<Vec<f32>>,
    ) -> StorageResult<SurrealLoomIndexMutation> {
        validate_block_scope(scope, block)?;
        let (embedding_model_id, embedding_dimension) =
            validate_index_embedding(embedding_model, embedding.as_deref())?;
        let search_text = block_search_text(block);
        let source_content_hash = block
            .content_hash
            .clone()
            .unwrap_or_else(|| sha256_hex(&search_text));
        let mutation_fingerprint = sha256_json(&json!({
            "scope": scope_json(scope),
            "block_id": block.block_id,
            "content_type": block.content_type.as_str(),
            "source_content_hash": source_content_hash,
            "search_text": search_text,
            "embedding_model": embedding_model_id,
            "embedding": embedding,
        }))?;
        let event = receipt_write(
            receipt_event(
                scope,
                KernelEventType::KnowledgeLoomBlockIndexed,
                "loom_block_search_index",
                &block.block_id,
                "loom_search::surreal_index",
                &mutation_fingerprint,
                json!({
                    "action": "loom_block_indexed",
                    "block_id": block.block_id,
                    "embedding_space_id": embedding_model_id,
                    "mutation_fingerprint": mutation_fingerprint,
                }),
            )?,
            scope,
        );
        let exact = exact_scope_bindings(scope);
        let block_id = block.block_id.clone();
        let content_type = block.content_type.as_str().to_owned();
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredIndexMutation, _>(
                            REINDEX_QUERY,
                            ReindexBindings {
                                block_record: RecordId::new("loom_blocks", block_id.clone()),
                                index_record: RecordId::new(
                                    "loom_block_search_index",
                                    block_id.clone(),
                                ),
                                content: IndexContent {
                                    block_id: RecordId::new("loom_blocks", block_id),
                                    owner_account_id: exact.owner_account_id,
                                    actor_principal_id: exact.actor_principal_id,
                                    authenticated_session_id: exact.authenticated_session_id,
                                    access_space_id: exact.access_space_id,
                                    workspace_id: exact.workspace_id,
                                    content_type,
                                    search_text,
                                    source_content_hash,
                                    embedding,
                                    embedding_model: embedding_model_id,
                                    embedding_dimension,
                                    lifecycle_state: "active".to_owned(),
                                    mutation_fingerprint,
                                    index_event_id: RecordId::new(
                                        "kernel_event_ledger",
                                        event.event_id.clone(),
                                    ),
                                    indexed_at: Utc::now(),
                                },
                                event,
                            },
                            5,
                        )
                        .await
                })
            })
            .await?;
        let row = one(rows, "Loom index mutation")?;
        Ok(SurrealLoomIndexMutation {
            block_id: row.block_id,
            embedding_space_id: row.embedding_model,
            index_event_id: row.index_event_id,
            mutation_fingerprint: row.mutation_fingerprint,
            changed: row.changed,
        })
    }

    pub async fn search(
        &self,
        scope: &ExactResourceScopeAttribution,
        request: &LoomSearchV2Request,
    ) -> StorageResult<SurrealLoomSearchEvidence> {
        validate_search_request(request)?;
        let exact = exact_scope_bindings(scope);
        let query_embedding = request.query_embedding.clone();
        let query_embedding_model = request.query_embedding_model.clone();
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<SearchIndexRow, _>(
                            SEARCH_ROWS_QUERY,
                            SearchRowsBindings {
                                owner_account_id: exact.owner_account_id,
                                actor_principal_id: exact.actor_principal_id,
                                authenticated_session_id: exact.authenticated_session_id,
                                access_space_id: exact.access_space_id,
                                workspace_id: exact.workspace_id,
                                query_embedding,
                                query_embedding_model,
                            },
                        )
                        .await
                })
            })
            .await?;
        let edge_scope = exact_scope_bindings(scope);
        let edges = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<EdgeRow, _>(
                            SEARCH_EDGES_QUERY,
                            SearchRowsBindings {
                                owner_account_id: edge_scope.owner_account_id,
                                actor_principal_id: edge_scope.actor_principal_id,
                                authenticated_session_id: edge_scope.authenticated_session_id,
                                access_space_id: edge_scope.access_space_id,
                                workspace_id: edge_scope.workspace_id,
                                query_embedding: None,
                                query_embedding_model: None,
                            },
                        )
                        .await
                })
            })
            .await?;
        let scored = score_rows(rows, &edges, request)?;
        self.persist_search(scope, request, scored).await
    }

    async fn persist_search(
        &self,
        scope: &ExactResourceScopeAttribution,
        request: &LoomSearchV2Request,
        scored: ScoredSearch,
    ) -> StorageResult<SurrealLoomSearchEvidence> {
        let query_fingerprint = sha256_json(&json!({
            "scope": scope_json(scope),
            "query": request.query,
            "content_type": request.content_type.as_ref().map(LoomBlockContentType::as_str),
            "tag_ids": request.tag_ids,
            "query_embedding_model": request.query_embedding_model,
            "query_embedding": request.query_embedding,
            "graph_boost": request.graph_boost,
        }))?;
        let result_set_fingerprint = sha256_json(&json!(scored
            .all_hits
            .iter()
            .map(|(hit, index_event_id, mutation_fingerprint)| json!({
                "block_id": hit.block.block_id,
                "score": hit.score,
                "fts_rank": hit.fts_rank,
                "trgm_sim": hit.trgm_sim,
                "vector_sim": hit.vector_sim,
                "edge_degree": hit.edge_degree,
                "source_index_event_id": index_event_id,
                "source_index_fingerprint": mutation_fingerprint,
            }))
            .collect::<Vec<_>>()))?;
        let identity_hash = sha256_hex(&format!(
            "{}\0{}\0{}\0{}\0{}\0{}\0{}",
            scope.owner_account_id,
            scope.actor_principal_id,
            scope.authenticated_session_id,
            scope.access_space_id,
            scope.workspace_id.as_str(),
            query_fingerprint,
            result_set_fingerprint
        ));
        let trace_id = format!("KRT-{identity_hash}");
        let result_set_id = format!("LSR-{identity_hash}");
        let event = receipt_write(
            receipt_event(
                scope,
                KernelEventType::KnowledgeRetrievalTraceRecorded,
                "loom_search_result_set",
                &result_set_id,
                "loom_search::surreal_search",
                &result_set_fingerprint,
                json!({
                    "action": "loom_search_result_set_recorded",
                    "trace_id": trace_id,
                    "query_fingerprint": query_fingerprint,
                    "result_set_fingerprint": result_set_fingerprint,
                    "source_index_event_ids": scored.all_hits.iter().map(|(_, id, _)| id).collect::<Vec<_>>(),
                }),
            )?,
            scope,
        );
        let exact = exact_scope_bindings(scope);
        let semantic_available = scored.response.semantic_available;
        let embedding_model = request.query_embedding_model.clone();
        let results = scored
            .all_hits
            .iter()
            .enumerate()
            .map(|(ordinal, (hit, index_event_id, _))| SearchResultContent {
                result_id: format!("LSH-{}", sha256_hex(&format!("{identity_hash}\0{ordinal}"))),
                result_set_id: RecordId::new("loom_search_result_sets", result_set_id.clone()),
                block_id: RecordId::new("loom_blocks", hit.block.block_id.clone()),
                owner_account_id: scope.owner_account_id.to_string(),
                actor_principal_id: scope.actor_principal_id.to_string(),
                authenticated_session_id: scope.authenticated_session_id.to_string(),
                access_space_id: scope.access_space_id.to_string(),
                workspace_id: RecordId::new("workspaces", scope.workspace_id.as_str().to_owned()),
                result_set_fingerprint: result_set_fingerprint.clone(),
                ordinal: ordinal as i64,
                score: hit.score,
                fts_rank: hit.fts_rank,
                trgm_sim: hit.trgm_sim,
                vector_sim: hit.vector_sim,
                edge_degree: hit.edge_degree,
                highlight: hit.highlight.clone(),
                source_index_event_id: RecordId::new("kernel_event_ledger", index_event_id.clone()),
            })
            .collect::<Vec<_>>();
        let trace = SearchTraceContent {
            trace_id: trace_id.clone(),
            workspace_id: RecordId::new("workspaces", scope.workspace_id.as_str().to_owned()),
            retrieval_mode: "hybrid_rag".to_owned(),
            mode_reason: if semantic_available {
                "exact_scope_keyword_trigram_vector_graph"
            } else {
                "exact_scope_keyword_trigram_graph"
            }
            .to_owned(),
            query_text: Some(request.query.clone()),
            bundle_id: None,
            decisions: json!({
                "content_type": request.content_type.as_ref().map(LoomBlockContentType::as_str),
                "tag_ids": request.tag_ids,
                "semantic_available": semantic_available,
                "result_count": scored.all_hits.len(),
            }),
            trace_receipt_event_id: RecordId::new("kernel_event_ledger", event.event_id.clone()),
            owner_account_id: exact.owner_account_id.clone(),
            actor_principal_id: exact.actor_principal_id.clone(),
            authenticated_session_id: exact.authenticated_session_id.clone(),
            access_space_id: exact.access_space_id.clone(),
            query_fingerprint: query_fingerprint.clone(),
            result_set_fingerprint: result_set_fingerprint.clone(),
        };
        let result_set = SearchResultSetContent {
            result_set_id: result_set_id.clone(),
            trace_id: RecordId::new("knowledge_retrieval_traces", trace_id.clone()),
            owner_account_id: exact.owner_account_id,
            actor_principal_id: exact.actor_principal_id,
            authenticated_session_id: exact.authenticated_session_id,
            access_space_id: exact.access_space_id,
            workspace_id: exact.workspace_id,
            query_fingerprint: query_fingerprint.clone(),
            result_set_fingerprint: result_set_fingerprint.clone(),
            semantic_available,
            embedding_model,
            total: results.len() as i64,
            receipt_event_id: RecordId::new("kernel_event_ledger", event.event_id.clone()),
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredSearchEvidence, _>(
                            PERSIST_SEARCH_QUERY,
                            PersistSearchBindings {
                                trace_record: RecordId::new("knowledge_retrieval_traces", trace_id),
                                result_set_record: RecordId::new(
                                    "loom_search_result_sets",
                                    result_set_id,
                                ),
                                trace,
                                result_set,
                                results,
                                event,
                            },
                            5,
                        )
                        .await
                })
            })
            .await?;
        let stored = one(rows, "Loom search evidence")?;
        Ok(SurrealLoomSearchEvidence {
            response: scored.response,
            trace_id: stored.trace_id,
            result_set_id: stored.result_set_id,
            receipt_event_id: stored.receipt_event_id,
            query_fingerprint,
            result_set_fingerprint,
            changed: stored.changed,
        })
    }
}

pub async fn bootstrap_loom_search_schema(
    storage: &SurrealStorage,
) -> Result<(), SurrealStorageError> {
    storage
        .with_admin_operation(|database| Box::pin(async move { database.query(SCHEMA).await }))
        .await?;
    Ok(())
}

const PERSIST_SEARCH_QUERY: &str = r#"
BEGIN TRANSACTION;
LET $existing = (SELECT result_set_id, record::id(trace_id) AS trace_id,
    record::id(receipt_event_id) AS receipt_event_id, result_set_fingerprint
FROM $result_set_record LIMIT 2);
LET $trace_existing = (SELECT trace_id, result_set_fingerprint,
    record::id(trace_receipt_event_id) AS receipt_event_id
FROM $trace_record LIMIT 2);
LET $stored_results = (SELECT result_set_fingerprint FROM loom_search_results
WHERE result_set_id = $result_set_record
  AND owner_account_id = $result_set.owner_account_id
  AND actor_principal_id = $result_set.actor_principal_id
  AND authenticated_session_id = $result_set.authenticated_session_id
  AND access_space_id = $result_set.access_space_id
  AND workspace_id = $result_set.workspace_id
  AND result_set_fingerprint = $result_set.result_set_fingerprint);
LET $event_existing = (SELECT event_id FROM kernel_event_ledger
WHERE idempotency_key = $event.idempotency_key
  AND owner_account_id = $event.owner_account_id
  AND actor_principal_id = $event.actor_principal_id
  AND authenticated_session_id = $event.authenticated_session_id
  AND access_space_id = $event.access_space_id
  AND workspace_id = $event.workspace_id
LIMIT 2);
IF array::len($existing) > 1 OR array::len($trace_existing) > 1
   OR array::len($event_existing) > 1 {
    THROW 'MT-016 search evidence identity or receipt is ambiguous';
} ELSE IF array::len($existing) = 1 {
    IF $existing[0].result_set_fingerprint != $result_set.result_set_fingerprint
       OR array::len($trace_existing) != 1
       OR $trace_existing[0].result_set_fingerprint != $result_set.result_set_fingerprint
       OR $trace_existing[0].receipt_event_id != $existing[0].receipt_event_id
       OR array::len($event_existing) != 1
       OR array::len($stored_results) != $result_set.total {
        THROW 'MT-016 committed search evidence is internally inconsistent';
    };
    RETURN [{ trace_id: $existing[0].trace_id,
        result_set_id: $existing[0].result_set_id,
        receipt_event_id: $existing[0].receipt_event_id,
        changed: false }];
} ELSE {
    IF array::len($trace_existing) != 0 OR array::len($event_existing) != 0
       OR array::len($stored_results) != 0 {
        THROW 'MT-016 search receipt or derivative exists without result-set authority';
    };
    CREATE type::record('kernel_event_ledger', $event.event_id) CONTENT $event;
    CREATE $trace_record CONTENT $trace;
    CREATE $result_set_record CONTENT $result_set;
    FOR $result IN $results {
        CREATE type::record('loom_search_results', $result.result_id) CONTENT $result;
    };
    RETURN [{ trace_id: $trace.trace_id,
        result_set_id: $result_set.result_set_id,
        receipt_event_id: $event.event_id,
        changed: true }];
};
COMMIT TRANSACTION;
"#;

fn validate_embedding_registration(registration: &RoleBoundModelRegistration) -> StorageResult<()> {
    let capabilities = &registration.registration.declared_capabilities;
    if registration.runtime_role != ModelRuntimeRole::Embedding {
        return Err(StorageError::Validation(
            "MT-016 registration must use the dedicated embedding role",
        ));
    }
    if !capabilities.supports_embedding {
        return Err(StorageError::Validation(
            "MT-016 embedding registration must declare supports_embedding",
        ));
    }
    if capabilities.embedding_dimension != Some(EMBEDDING_DIMENSION) {
        return Err(StorageError::Validation(
            "MT-016 embedding registration must declare dimension 768",
        ));
    }
    if registration.registration.provider != crate::model_runtime::ProviderKind::Local {
        return Err(StorageError::Validation(
            "MT-016 embedded model registration requires the local provider",
        ));
    }
    Ok(())
}

fn validate_block_scope(
    scope: &ExactResourceScopeAttribution,
    block: &LoomBlock,
) -> StorageResult<()> {
    if block.block_id.trim().is_empty() || block.workspace_id != scope.workspace_id.as_str() {
        return Err(StorageError::Validation(
            "MT-016 Loom block must have identity in the exact workspace scope",
        ));
    }
    Ok(())
}

fn validate_index_embedding(
    model: Option<&SurrealEmbeddingRegistration>,
    embedding: Option<&[f32]>,
) -> StorageResult<(Option<String>, Option<i64>)> {
    match (model, embedding) {
        (None, None) => Ok((None, None)),
        (Some(model), Some(embedding)) => {
            if model.runtime_role != ModelRuntimeRole::Embedding
                || !model.declared_capabilities.supports_embedding
                || model.embedding_dimension != EMBEDDING_DIMENSION
                || model.lifecycle_state != "active"
                || embedding.len() != EMBEDDING_DIMENSION
            {
                return Err(StorageError::Validation(
                    "MT-016 index vector/model identity or dimension is invalid",
                ));
            }
            Ok((
                Some(model.embedding_space_id.clone()),
                Some(EMBEDDING_DIMENSION as i64),
            ))
        }
        _ => Err(StorageError::Validation(
            "MT-016 index vectors require the exact dedicated embedding selection",
        )),
    }
}

fn validate_search_request(request: &LoomSearchV2Request) -> StorageResult<()> {
    if request.limit > 1000 {
        return Err(StorageError::Validation("Loom search limit exceeds 1000"));
    }
    if request.query_embedding.is_some() != request.query_embedding_model.is_some() {
        return Err(StorageError::Validation(
            "MT-016 semantic search requires both vector and embedding-space identity",
        ));
    }
    if request
        .query_embedding_model
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(StorageError::Validation(
            "MT-016 embedding-space identity must not be empty",
        ));
    }
    Ok(())
}

fn score_rows(
    rows: Vec<SearchIndexRow>,
    edges: &[EdgeRow],
    request: &LoomSearchV2Request,
) -> StorageResult<ScoredSearch> {
    let normalized_query = request.query.trim().to_lowercase();
    let tokens = normalized_query
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let dimension_matches = request
        .query_embedding
        .as_ref()
        .is_none_or(|embedding| embedding.len() == EMBEDDING_DIMENSION);
    let semantic_available = request.query_embedding.is_some() && dimension_matches;
    let semantic_unavailable_reason = if semantic_available {
        None
    } else if let Some(embedding) = request.query_embedding.as_ref() {
        Some(SemanticUnavailableReason::DimMismatch {
            expected: EMBEDDING_DIMENSION,
            actual: embedding.len(),
        })
    } else {
        Some(SemanticUnavailableReason::NoModel)
    };
    let mut scored = Vec::new();
    for row in rows {
        if request
            .content_type
            .as_ref()
            .is_some_and(|filter| filter.as_str() != row.content_type)
        {
            continue;
        }
        if !request.tag_ids.is_empty()
            && !edges.iter().any(|edge| {
                edge.source_block_id == row.block_id
                    && edge.edge_type == "tag"
                    && request.tag_ids.contains(&edge.target_block_id)
            })
        {
            continue;
        }
        let text = row.search_text.to_lowercase();
        let fts_rank = if tokens.is_empty() {
            1.0
        } else {
            tokens.iter().filter(|token| text.contains(**token)).count() as f64
                / tokens.len() as f64
        };
        let trgm_sim = trigram_similarity(&normalized_query, &text);
        let vector_sim = if semantic_available {
            row.vector_sim
        } else {
            0.0
        };
        if !tokens.is_empty() && fts_rank == 0.0 && trgm_sim == 0.0 && vector_sim <= 0.0 {
            continue;
        }
        let edge_degree = edges
            .iter()
            .filter(|edge| {
                edge.source_block_id == row.block_id || edge.target_block_id == row.block_id
            })
            .count() as i64;
        let score = fts_rank * 0.45
            + trgm_sim * 0.20
            + vector_sim * 0.35
            + edge_degree as f64 * request.graph_boost;
        let block = stored_block(&row)?;
        let highlight = highlight(&row.search_text, &tokens);
        scored.push((
            LoomSearchV2Hit {
                block,
                score,
                fts_rank,
                trgm_sim,
                vector_sim,
                edge_degree,
                highlight,
            },
            row.index_event_id,
            row.mutation_fingerprint,
        ));
    }
    scored.sort_by(|left, right| {
        right
            .0
            .score
            .total_cmp(&left.0.score)
            .then_with(|| left.0.block.block_id.cmp(&right.0.block.block_id))
    });
    let mut facets = BTreeMap::new();
    for (hit, _, _) in &scored {
        *facets
            .entry(hit.block.content_type.as_str().to_owned())
            .or_insert(0) += 1;
    }
    let total = scored.len() as i64;
    let limit = if request.limit == 0 {
        20
    } else {
        request.limit
    } as usize;
    let hits = scored
        .iter()
        .skip(request.offset as usize)
        .take(limit)
        .map(|(hit, _, _)| hit.clone())
        .collect();
    Ok(ScoredSearch {
        response: LoomSearchV2Response {
            hits,
            content_type_facets: facets,
            semantic_available,
            semantic_unavailable_reason,
            total,
        },
        all_hits: scored,
    })
}

fn stored_block(row: &SearchIndexRow) -> StorageResult<LoomBlock> {
    Ok(LoomBlock {
        block_id: row.block_id.clone(),
        workspace_id: row.workspace_id.clone(),
        content_type: LoomBlockContentType::from_str(&row.content_type)?,
        document_id: row.document_id.clone(),
        asset_id: row.asset_id.clone(),
        title: row.title.clone(),
        original_filename: row.original_filename.clone(),
        content_hash: row.content_hash.clone(),
        pinned: row.pinned,
        favorite: row.favorite,
        pin_order: row.pin_order.map(|value| value as i32),
        journal_date: row.journal_date.clone(),
        created_at: row.created_at,
        updated_at: row.updated_at,
        imported_at: row.imported_at,
        derived: serde_json::from_value::<LoomBlockDerived>(row.derived_json.clone())?,
    })
}

fn stored_embedding(
    row: StoredEmbeddingRegistration,
) -> StorageResult<SurrealEmbeddingRegistration> {
    let runtime_role = match row.runtime_role.as_str() {
        "embedding" => ModelRuntimeRole::Embedding,
        "completion" => ModelRuntimeRole::Completion,
        _ => {
            return Err(StorageError::Validation(
                "stored model runtime role is invalid",
            ))
        }
    };
    Ok(SurrealEmbeddingRegistration {
        registry_row_id: row.registry_row_id,
        artifact_sha256: row.artifact_sha256,
        runtime_model_id: row.last_observed_runtime_model_id,
        runtime_binding: row.runtime_binding,
        runtime_role,
        declared_capabilities: serde_json::from_value(row.capabilities)?,
        base_model_tag: row.base_model_tag,
        embedding_space_id: row.embedding_space_id,
        embedding_dimension: usize::try_from(row.embedding_dimension)
            .map_err(|_| StorageError::Validation("stored embedding dimension is invalid"))?,
        lifecycle_state: row.lifecycle_state,
        selection_event_id: row.selection_event_id,
        changed: row.changed,
    })
}

fn one<T>(rows: Vec<T>, label: &'static str) -> StorageResult<T> {
    if rows.len() != 1 {
        return Err(StorageError::Validation(label));
    }
    Ok(rows.into_iter().next().expect("one row checked"))
}

fn exact_scope_bindings(scope: &ExactResourceScopeAttribution) -> ExactScopeBindings {
    ExactScopeBindings {
        owner_account_id: scope.owner_account_id.to_string(),
        actor_principal_id: scope.actor_principal_id.to_string(),
        authenticated_session_id: scope.authenticated_session_id.to_string(),
        access_space_id: scope.access_space_id.to_string(),
        workspace_id: RecordId::new("workspaces", scope.workspace_id.as_str().to_owned()),
    }
}

fn scope_json(scope: &ExactResourceScopeAttribution) -> Value {
    json!({
        "owner_account_id": scope.owner_account_id.to_string(),
        "actor_principal_id": scope.actor_principal_id.to_string(),
        "authenticated_session_id": scope.authenticated_session_id.to_string(),
        "access_space_id": scope.access_space_id.to_string(),
        "workspace_id": scope.workspace_id.as_str(),
    })
}

fn block_search_text(block: &LoomBlock) -> String {
    [
        block.title.as_deref(),
        block.original_filename.as_deref(),
        block.derived.full_text_index.as_deref(),
        block.derived.auto_caption.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
}

fn trigram_similarity(left: &str, right: &str) -> f64 {
    fn trigrams(value: &str) -> std::collections::BTreeSet<String> {
        let chars = format!("  {} ", value).chars().collect::<Vec<_>>();
        chars
            .windows(3)
            .map(|window| window.iter().collect())
            .collect()
    }
    if left.is_empty() {
        return 1.0;
    }
    let left = trigrams(left);
    let right = trigrams(right);
    let union = left.union(&right).count();
    if union == 0 {
        0.0
    } else {
        left.intersection(&right).count() as f64 / union as f64
    }
}

fn highlight(text: &str, tokens: &[&str]) -> String {
    let mut value = text.chars().take(240).collect::<String>();
    for token in tokens.iter().take(3) {
        if let Some(start) = value.to_lowercase().find(*token) {
            let end = start + token.len();
            if value.is_char_boundary(start) && value.is_char_boundary(end) {
                value.insert_str(end, "</mark>");
                value.insert_str(start, "<mark>");
            }
        }
    }
    value
}

fn receipt_event(
    scope: &ExactResourceScopeAttribution,
    event_type: KernelEventType,
    aggregate_type: &str,
    aggregate_id: &str,
    source_component: &str,
    mutation_fingerprint: &str,
    payload: Value,
) -> StorageResult<NewKernelEvent> {
    NewKernelEvent::builder(
        format!("MT016-{mutation_fingerprint}"),
        scope.authenticated_session_id.to_string(),
        event_type,
        KernelActor::System(source_component.to_owned()),
    )
    .aggregate(aggregate_type, aggregate_id)
    .idempotency_key(format!("MT016-{aggregate_type}-{mutation_fingerprint}"))
    .source_component(source_component)
    .payload(payload)
    .build()
    .map_err(|_| StorageError::Validation("MT-016 EventLedger receipt is invalid"))
}

fn receipt_write(
    event: NewKernelEvent,
    scope: &ExactResourceScopeAttribution,
) -> KernelReceiptWrite {
    let event = KernelEvent::from_new(event);
    KernelReceiptWrite {
        event_id: event.event_id,
        event_version: event.event_version,
        kernel_task_run_id: event.kernel_task_run_id,
        session_run_id: event.session_run_id,
        aggregate_type: event.aggregate_type,
        aggregate_id: event.aggregate_id,
        idempotency_key: event.idempotency_key,
        event_type: event.event_type.to_string(),
        actor_kind: event.actor.actor_kind().to_owned(),
        actor_id: event.actor.actor_id().to_owned(),
        causation_id: event.causation_id,
        correlation_id: event.correlation_id,
        payload_hash: event.payload_hash,
        source_component: event.source_component,
        payload: event.payload,
        owner_account_id: scope.owner_account_id.to_string(),
        actor_principal_id: scope.actor_principal_id.to_string(),
        authenticated_session_id: scope.authenticated_session_id.to_string(),
        access_space_id: scope.access_space_id.to_string(),
        workspace_id: scope.workspace_id.as_str().to_owned(),
    }
}

fn sha256_json(value: &Value) -> StorageResult<String> {
    Ok(sha256_hex(&serde_json::to_string(value)?))
}

fn sha256_hex(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(feature = "test-utils")]
async fn set_lifecycle_fixture(
    storage: &SurrealStorage,
    table: &str,
    record_id: &str,
    lifecycle_state: &str,
) -> StorageResult<()> {
    if !matches!(lifecycle_state, "active" | "stale" | "revoked") {
        return Err(StorageError::Validation(
            "MT-016 fixture lifecycle state is invalid",
        ));
    }
    let record = RecordId::new(table, record_id.to_owned());
    let lifecycle_state = lifecycle_state.to_owned();
    let rows = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<Value, _>(
                        SET_LIFECYCLE_FIXTURE_QUERY,
                        LifecycleFixtureBindings {
                            record,
                            lifecycle_state,
                        },
                    )
                    .await
            })
        })
        .await?;
    if rows.len() != 1 {
        return Err(StorageError::Validation(
            "MT-016 fixture target does not exist",
        ));
    }
    Ok(())
}
