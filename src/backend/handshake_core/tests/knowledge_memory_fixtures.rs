//! Shared test fixtures for the WP-KERNEL-009 MemoryGraphAndClaims tests
//! (MT-113..MT-128). Builds on the embedded knowledge test support and adds the
//! memory-graph fixture: workspace -> root -> source -> span, plus convenience
//! constructors for entities and evidence-backed claims that memory facts
//! attach to.
//!
//! Real embedded storage only. No server fallback or mocks. The fixture owns
//! an isolated on-disk store for the duration of each test.

// `embedded_knowledge_support` is compiled into each integration-test binary that
// declares it; this support file re-uses it as a sibling module path.
#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

pub use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};

use handshake_core::storage::knowledge::{
    KnowledgeClaim, KnowledgeClaimKind, KnowledgeEntityKind, KnowledgeIndexingEligibility,
    KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeRootKind, KnowledgeSourceKind,
    KnowledgeSpanKind, KnowledgeStore, NewKnowledgeClaim, NewKnowledgeEntity, NewKnowledgeSource,
    NewKnowledgeSourceRoot, NewKnowledgeSpan,
};
use handshake_core::storage::surreal::SurrealStorage;
use serde_json::json;
use uuid::Uuid;

const HASH_SRC: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_SPAN: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A configured memory-graph fixture: a workspace with one source and one span
/// that fresh claims cite as evidence.
pub struct MemoryFixture {
    pub store: EmbeddedKnowledgeStore,
    pub workspace_id: String,
    pub source_id: String,
    pub span_id: String,
}

impl MemoryFixture {
    /// Build the fixture over a real embedded store.
    pub async fn setup() -> Option<Self> {
        let store = open_embedded_store().await?;
        let workspace_id = store.create_workspace().await;
        let root = store
            .db
            .create_knowledge_source_root(NewKnowledgeSourceRoot {
                workspace_id: workspace_id.clone(),
                display_name: "core".to_string(),
                root_kind: KnowledgeRootKind::ProjectRepo,
                repo_relative_path: format!("src/{}", Uuid::now_v7().simple()),
                allowlist_policy: json!({"include": ["**/*"], "exclude": []}),
                indexing_eligibility: KnowledgeIndexingEligibility::Eligible,
            })
            .await
            .expect("root");
        let source = store
            .db
            .upsert_knowledge_source(NewKnowledgeSource {
                workspace_id: workspace_id.clone(),
                root_id: Some(root.root_id),
                source_kind: KnowledgeSourceKind::File,
                relative_path: Some("memory/graph.rs".to_string()),
                asset_id: None,
                loom_block_id: None,
                document_id: None,
                content_hash: HASH_SRC.to_string(),
                size_bytes: Some(2048),
                provenance: json!({"discovered_by": "memory_fixture"}),
                permission_scope: KnowledgePermissionScope::Workspace,
                redaction_state: KnowledgeRedactionState::None,
                source_modified_at: None,
            })
            .await
            .expect("source");
        let span = store
            .db
            .create_knowledge_span(NewKnowledgeSpan {
                source_id: source.source_id.clone(),
                span_kind: KnowledgeSpanKind::Text,
                range_start: 0,
                range_end: 200,
                line_start: Some(1),
                line_end: Some(5),
                section_path: None,
                content_sha256: HASH_SPAN.to_string(),
                parser_version: "text_v1".to_string(),
                extraction_receipt_event_id: None,
                index_run_id: None,
                display_snippet: Some("memory graph fixture span".to_string()),
            })
            .await
            .expect("span");
        Some(Self {
            workspace_id,
            source_id: source.source_id,
            span_id: span.span_id,
            store,
        })
    }

    /// Create (or fetch the upserted) knowledge entity of the given kind/key.
    pub async fn entity(&self, kind: &str, key: &str, display: &str) -> String {
        let entity_kind: KnowledgeEntityKind = kind.parse().expect("valid entity kind");
        let entity = self
            .store
            .db
            .upsert_knowledge_entity(NewKnowledgeEntity {
                workspace_id: self.workspace_id.clone(),
                entity_kind,
                entity_key: key.to_string(),
                display_name: display.to_string(),
                detection_provenance: json!({"extractor": "memory_fixture"}),
                primary_source_id: Some(self.source_id.clone()),
                detected_in_run: None,
                evidence_span_ids: vec![self.span_id.clone()],
            })
            .await
            .expect("entity");
        entity.entity_id
    }

    /// Create an evidence-backed proposed claim (cites the fixture span).
    pub async fn claim(&self, text: &str) -> KnowledgeClaim {
        self.store
            .db
            .create_knowledge_claim(NewKnowledgeClaim {
                workspace_id: self.workspace_id.clone(),
                claim_kind: KnowledgeClaimKind::ProductBehavior,
                claim_text: text.to_string(),
                subject_entity_id: None,
                temporal_qualifier: None,
                granularity_qualifier: None,
                confidence: 0.7,
                proposed_in_run: None,
                evidence_span_ids: vec![self.span_id.clone()],
            })
            .await
            .expect("claim")
    }
}

/// Borrow a clone of the fixture's embedded storage handle. Memory free
/// functions accept `&SurrealStorage`, so every call remains on the same real
/// on-disk authority.
pub async fn pool_for(store: &EmbeddedKnowledgeStore) -> SurrealStorage {
    store.storage.clone()
}
