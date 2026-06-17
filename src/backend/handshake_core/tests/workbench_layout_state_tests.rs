//! WP-KERNEL-009 MT-246 durable workbench layout-state proof.
//!
//! This covers the backend foundation for split-editor/workbench restoration:
//! layout UI state must persist in PostgreSQL and retain a typed Kernel
//! EventLedger receipt. It intentionally uses a real isolated PostgreSQL schema;
//! localStorage or process memory cannot pass.

mod knowledge_pg_support;

use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{
    Database, StorageError, WORKBENCH_LAYOUT_SCHEMA_ID, WorkbenchLayoutStateInput,
};
use knowledge_pg_support::knowledge_pg;
use serde_json::json;
use sqlx::Row;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                panic!("MT-246 workbench layout proof requires real PostgreSQL");
            }
        }
    }};
}

fn layout_state(
    workspace_id: &str,
    active_pane_id: &str,
    active_module: &str,
) -> serde_json::Value {
    json!({
        "schema_id": WORKBENCH_LAYOUT_SCHEMA_ID,
        "activePaneId": active_pane_id,
        "activeModule": active_module,
        "splitWeights": { "vertical": 0.62, "horizontal": 0.44 },
        "drawers": { "project": true, "file": false, "bottom": true },
        "panes": [
            {
                "id": "pane-a",
                "module": "MAIN",
                "activeTab": "workspace",
                "tabs": ["workspace"],
                "locked": false,
                "projectRef": workspace_id
            },
            {
                "id": "pane-b",
                "module": "CKC",
                "activeTab": "kernel-dcc",
                "tabs": ["kernel-dcc", "workspace"],
                "locked": false,
                "projectRef": workspace_id
            },
            {
                "id": "pane-c",
                "module": "INGEST",
                "activeTab": "flight-recorder",
                "tabs": ["flight-recorder"],
                "locked": false,
                "projectRef": workspace_id
            },
            {
                "id": "pane-d",
                "module": "STAGE",
                "activeTab": "fonts",
                "tabs": ["fonts"],
                "locked": false,
                "projectRef": workspace_id
            }
        ]
    })
}

#[tokio::test]
async fn mt246_workbench_layout_rejects_non_object_state() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let err = pg
        .db
        .save_workbench_layout_state(
            &ws,
            WorkbenchLayoutStateInput {
                layout_state: json!(["not", "an", "object"]),
            },
        )
        .await
        .expect_err("non-object layout state must be rejected");

    assert!(matches!(
        err,
        StorageError::Validation("workbench layout_state must be a JSON object")
    ));
}

#[tokio::test]
async fn mt246_workbench_layout_rejects_wrong_schema_id() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let err = pg
        .db
        .save_workbench_layout_state(
            &ws,
            WorkbenchLayoutStateInput {
                layout_state: json!({
                    "schema_id": "hsk.workbench_layout_state@0",
                    "activePaneId": "pane-a"
                }),
            },
        )
        .await
        .expect_err("wrong layout schema id must be rejected");

    assert!(matches!(
        err,
        StorageError::Validation(
            "workbench layout_state schema_id must be hsk.workbench_layout_state@1"
        )
    ));
}

#[tokio::test]
async fn mt246_workbench_layout_rejects_schema_correct_malformed_state_before_eventledger() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let err = pg
        .db
        .save_workbench_layout_state(
            &ws,
            WorkbenchLayoutStateInput {
                layout_state: json!({
                    "schema_id": WORKBENCH_LAYOUT_SCHEMA_ID
                }),
            },
        )
        .await
        .expect_err("schema-correct but render-invalid layout state must be rejected");

    assert!(matches!(
        err,
        StorageError::Validation(
            "workbench layout_state must match hsk.workbench_layout_state@1 renderable shape"
        )
    ));

    let mut conn = pg.raw_connection().await;
    let event_count: i64 = sqlx::query(
        r#"
        SELECT COUNT(*)::BIGINT AS count
        FROM kernel_event_ledger
        WHERE aggregate_type = 'workbench_layout_state'
          AND aggregate_id = $1
        "#,
    )
    .bind(&ws)
    .fetch_one(&mut conn)
    .await
    .expect("query layout event count")
    .get("count");
    assert_eq!(
        event_count, 0,
        "invalid layout must fail before EventLedger append"
    );
}

#[tokio::test]
async fn mt246_workbench_layout_persists_with_eventledger() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let initial = pg
        .db
        .get_workbench_layout_state(&ws)
        .await
        .expect("read empty layout");
    assert!(
        initial.is_none(),
        "new workspace should not synthesize layout state"
    );

    let first = pg
        .db
        .save_workbench_layout_state(
            &ws,
            WorkbenchLayoutStateInput {
                layout_state: layout_state(&ws, "pane-b", "CKC"),
            },
        )
        .await
        .expect("save first layout");

    assert_eq!(first.workspace_id, ws);
    assert_eq!(first.layout_state["schema_id"], WORKBENCH_LAYOUT_SCHEMA_ID);
    assert_eq!(first.layout_state["activePaneId"], "pane-b");
    assert!(!first.event_ledger_event_id.trim().is_empty());

    let updated = pg
        .db
        .save_workbench_layout_state(
            &ws,
            WorkbenchLayoutStateInput {
                layout_state: layout_state(&ws, "pane-c", "INGEST"),
            },
        )
        .await
        .expect("save updated layout");

    assert_ne!(
        first.event_ledger_event_id, updated.event_ledger_event_id,
        "each layout mutation must retain its own EventLedger receipt"
    );

    let loaded = pg
        .db
        .get_workbench_layout_state(&ws)
        .await
        .expect("load layout")
        .expect("layout exists");
    assert_eq!(loaded.layout_state["activePaneId"], "pane-c");
    assert_eq!(loaded.event_ledger_event_id, updated.event_ledger_event_id);

    let mut conn = pg.raw_connection().await;
    let event_count: i64 = sqlx::query(
        r#"
        SELECT COUNT(*)::BIGINT AS count
        FROM kernel_event_ledger
        WHERE event_id = $1
          AND event_type = $2
          AND aggregate_type = 'workbench_layout_state'
          AND payload ->> 'workspace_id' = $3
          AND payload -> 'layout_state' ->> 'activePaneId' = 'pane-c'
        "#,
    )
    .bind(&updated.event_ledger_event_id)
    .bind(KernelEventType::KnowledgeWorkbenchLayoutStateRecorded.as_str())
    .bind(&ws)
    .fetch_one(&mut conn)
    .await
    .expect("query matching kernel event")
    .get("count");
    assert_eq!(event_count, 1);

    let row_count: i64 = sqlx::query(
        r#"
        SELECT COUNT(*)::BIGINT AS count
        FROM knowledge_workbench_layout_states s
        JOIN kernel_event_ledger e
          ON e.event_id = s.event_ledger_event_id
        WHERE s.workspace_id = $1
          AND e.event_type = $2
        "#,
    )
    .bind(&ws)
    .bind(KernelEventType::KnowledgeWorkbenchLayoutStateRecorded.as_str())
    .fetch_one(&mut conn)
    .await
    .expect("query layout row event FK")
    .get("count");
    assert_eq!(
        row_count, 1,
        "layout state row must retain its EventLedger FK"
    );
}
