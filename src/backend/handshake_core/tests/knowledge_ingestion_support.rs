//! Shared support for the WP-KERNEL-009 SourceIngestionAndEvidence
//! integration tests (MT-081..MT-096).
//!
//! Builds on `knowledge_pg_support` (real Handshake-managed PostgreSQL,
//! per-test isolated schema, full migration chain — no SQLite, no mocks) and
//! adds the ingestion engine wiring: a second `PostgresDatabase` handle into
//! the SAME isolated schema feeds `IngestionEngine::from_database`, so the
//! engine, the storage layer, and the raw assertion connection all see one
//! durable state.
#![allow(dead_code)]

use std::sync::Arc;

use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_ingestion::engine::{IngestionContext, IngestionEngine};
use handshake_core::storage::postgres::PostgresDatabase;
use uuid::Uuid;

use crate::knowledge_pg_support::{knowledge_pg, KnowledgePg};

/// One isolated ingestion test environment.
pub struct IngestionPg {
    pub pg: KnowledgePg,
    pub engine: IngestionEngine,
}

/// Fresh isolated schema + migrations + ingestion engine. Returns `None`
/// only when PostgreSQL binaries are absent (caller must SKIP loudly).
pub async fn ingestion_pg() -> Option<IngestionPg> {
    let pg = knowledge_pg().await?;
    // Name the isolated schema in the test log so a failing run can be
    // inspected directly on the cluster.
    eprintln!("knowledge ingestion test schema: {}", pg.schema);
    let db = PostgresDatabase::connect(&pg.schema_url, 5)
        .await
        .expect("connect ingestion engine handle to isolated schema");
    let engine = IngestionEngine::from_database(Arc::new(db));
    Some(IngestionPg { pg, engine })
}

/// Backend-navigation context for tests (spec 2.3.13.11: actor + session +
/// correlation ids on every mutation receipt).
pub fn test_ctx(label: &str) -> IngestionContext {
    let suffix = Uuid::now_v7();
    IngestionContext {
        actor: KernelActor::System(format!("ingestion-test-{label}")),
        kernel_task_run_id: format!("KTR-INGEST-{suffix}"),
        session_run_id: format!("SR-INGEST-{suffix}"),
        correlation_id: Some(format!("CORR-INGEST-{suffix}")),
    }
}

/// Register a root of the given kind under the default allowlist policy.
pub async fn register_root(
    env: &IngestionPg,
    ctx: &IngestionContext,
    workspace_id: &str,
    repo_relative_path: &str,
    root_kind: handshake_core::storage::knowledge::KnowledgeRootKind,
) -> handshake_core::storage::knowledge::KnowledgeSourceRoot {
    use handshake_core::knowledge_ingestion::engine::RootRegistrationRequest;
    let (root, _decision) = env
        .engine
        .register_root(
            ctx,
            RootRegistrationRequest {
                workspace_id: workspace_id.to_string(),
                display_name: if repo_relative_path.is_empty() {
                    "test root (repo)".to_string()
                } else {
                    format!("test root {repo_relative_path}")
                },
                root_kind,
                repo_relative_path: repo_relative_path.to_string(),
                file_allowlist_policy: serde_json::json!({"include": ["**/*"], "exclude": []}),
                // Operator-gated kinds need the explicit wave-through.
                operator_approved: true,
            },
        )
        .await
        .expect("register test root");
    root
}
