//! WP-1 dual-substrate import smoke: proves the imported embedded-SurrealDB
//! seam is live end-to-end in THIS worktree — open, schema bootstrap, a
//! trivial typed write+read through the sealed `SurrealDataContext` facade,
//! and a clean shutdown. Runs against a scratch store under the external
//! `Handshake_Artifacts/handshake-product` runtime root [CX-212E], resolved
//! repo-relatively so no machine-local absolute path is baked in [CX-109B].

use std::path::PathBuf;

use surrealdb::types::SurrealValue;

use super::schema::bootstrap_schema;
use super::{SurrealStorage, SurrealStorageConfig};

const SMOKE_ROOT_RELATIVE: &str =
    "../../../../Handshake_Artifacts/handshake-product/wp1-mmo-surreal-smoke";

fn smoke_store_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(SMOKE_ROOT_RELATIVE)
        .join(format!("run-{}", uuid::Uuid::now_v7().simple()))
}

#[derive(SurrealValue)]
struct SmokeWorkspaceContent {
    name: String,
}

#[tokio::test]
async fn surreal_substrate_smoke() {
    let store_root = smoke_store_root();
    std::fs::create_dir_all(&store_root).expect("create smoke store root");

    let config = SurrealStorageConfig::for_data_dir(&store_root).expect("configure smoke store");
    let storage = SurrealStorage::open(config)
        .await
        .expect("open smoke store");

    let report = bootstrap_schema(&storage)
        .await
        .expect("bootstrap smoke schema");
    assert!(!report.reused_existing_schema, "smoke store must be fresh");
    assert!(report.table_names.iter().any(|table| table == "workspaces"));

    let written_name = "wp1-substrate-smoke".to_owned();
    let readback = storage
        .with_data_operation(|data| {
            let written_name = written_name.clone();
            Box::pin(async move {
                data.upsert_one::<surrealdb::types::Value, _>(
                    "workspaces",
                    "wp1_substrate_smoke",
                    SmokeWorkspaceContent { name: written_name },
                )
                .await?;
                data.query_first::<String, _>(
                    "SELECT VALUE name FROM ONLY type::thing('workspaces', $id);",
                    ("id", "wp1_substrate_smoke".to_owned()),
                )
                .await
            })
        })
        .await
        .expect("write and read smoke record through the data facade");
    assert_eq!(readback.as_deref(), Some("wp1-substrate-smoke"));

    storage.shutdown().await.expect("clean smoke shutdown");
    assert!(storage.is_closed().await);

    // Best-effort scratch hygiene; the WP proof step removes the smoke root.
    let _ = std::fs::remove_dir_all(&store_root);
}
