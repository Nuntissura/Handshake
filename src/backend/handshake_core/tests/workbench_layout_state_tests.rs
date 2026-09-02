//! WP-KERNEL-009 MT-246 durable workbench layout-state proof.
//!
//! This covers the backend foundation for split-editor/workbench restoration:
//! layout UI state must persist in the embedded store and retain a typed Kernel
//! EventLedger receipt. It intentionally uses a real isolated embedded store;
//! localStorage or process memory cannot pass.

#[allow(dead_code)]
mod user_manual_support;

use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{
    Database, StorageError, WorkbenchLayoutStateInput, WORKBENCH_LAYOUT_SCHEMA_ID,
};
use serde_json::json;
use user_manual_support::manual_test_backend;

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
    let backend = manual_test_backend().await.expect("open embedded backend");
    let ws = backend.create_workspace().await;

    let err = backend
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
    backend
        .close_and_remove()
        .await
        .expect("close embedded backend");
}

#[tokio::test]
async fn mt246_workbench_layout_rejects_wrong_schema_id() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let ws = backend.create_workspace().await;

    let err = backend
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
    backend
        .close_and_remove()
        .await
        .expect("close embedded backend");
}

#[tokio::test]
async fn mt246_workbench_layout_rejects_schema_correct_malformed_state_before_eventledger() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let ws = backend.create_workspace().await;

    let err = backend
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

    let event_count = backend
        .db
        .list_kernel_events_for_aggregate("workbench_layout_state", &ws)
        .await
        .expect("query layout event count")
        .len();
    assert_eq!(
        event_count, 0,
        "invalid layout must fail before EventLedger append"
    );
    backend
        .close_and_remove()
        .await
        .expect("close embedded backend");
}

#[tokio::test]
async fn mt246_workbench_layout_persists_with_eventledger() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let ws = backend.create_workspace().await;

    let initial = backend
        .db
        .get_workbench_layout_state(&ws)
        .await
        .expect("read empty layout");
    assert!(
        initial.is_none(),
        "new workspace should not synthesize layout state"
    );

    let first = backend
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

    let updated = backend
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

    let loaded = backend
        .db
        .get_workbench_layout_state(&ws)
        .await
        .expect("load layout")
        .expect("layout exists");
    assert_eq!(loaded.layout_state["activePaneId"], "pane-c");
    assert_eq!(loaded.event_ledger_event_id, updated.event_ledger_event_id);

    let events = backend
        .db
        .list_kernel_events_for_aggregate("workbench_layout_state", &ws)
        .await
        .expect("query matching kernel event");
    let event_count = events
        .iter()
        .filter(|event| {
            event.event_id == updated.event_ledger_event_id
                && event.event_type.as_str()
                    == KernelEventType::KnowledgeWorkbenchLayoutStateRecorded.as_str()
                && event.payload["workspace_id"] == ws
                && event.payload["layout_state"]["activePaneId"] == "pane-c"
        })
        .count();
    assert_eq!(event_count, 1);

    let row_count = events
        .iter()
        .filter(|event| {
            event.event_id == loaded.event_ledger_event_id
                && event.event_type.as_str()
                    == KernelEventType::KnowledgeWorkbenchLayoutStateRecorded.as_str()
        })
        .count();
    assert_eq!(
        row_count, 1,
        "layout state row must retain its EventLedger FK"
    );
    backend
        .close_and_remove()
        .await
        .expect("close embedded backend");
}
