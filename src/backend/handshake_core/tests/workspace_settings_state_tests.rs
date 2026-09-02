//! WP-KERNEL-009 MT-248 durable workspace-settings proof.
//!
//! Settings, theme, and app keybindings are operator support state, but they
//! must be workspace-scoped embedded state with EventLedger receipts rather
//! than localStorage-only UI preferences.

#[allow(dead_code)]
mod user_manual_support;

use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{
    Database, StorageError, WorkspaceSettingsStateInput, WORKSPACE_SETTINGS_SCHEMA_ID,
};
use serde_json::json;
use user_manual_support::manual_test_backend;

fn settings_state(
    theme: &str,
    quick_switcher_chord: &str,
    command_palette_chord: &str,
) -> serde_json::Value {
    json!({
        "schema_id": WORKSPACE_SETTINGS_SCHEMA_ID,
        "theme": theme,
        "custom_theme_tokens": {
            "--hs-color-accent": "#22c55e"
        },
        "keybindings": {
            "app.quick_switcher.open": quick_switcher_chord,
            "app.command_palette.open": command_palette_chord
        },
        "settings": {
            "view_mode": "SFW",
            "swarm_board_default_open": true
        }
    })
}

#[tokio::test]
async fn mt248_workspace_settings_rejects_non_object_state() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let ws = backend.create_workspace().await;

    let err = backend
        .db
        .save_workspace_settings_state(
            &ws,
            WorkspaceSettingsStateInput {
                settings_state: json!(["not", "an", "object"]),
            },
        )
        .await
        .expect_err("non-object settings state must be rejected");

    assert!(matches!(
        err,
        StorageError::Validation("workspace settings_state must be a JSON object")
    ));
    backend
        .close_and_remove()
        .await
        .expect("close embedded backend");
}

#[tokio::test]
async fn mt248_workspace_settings_rejects_wrong_schema_id() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let ws = backend.create_workspace().await;

    let err = backend
        .db
        .save_workspace_settings_state(
            &ws,
            WorkspaceSettingsStateInput {
                settings_state: json!({
                    "schema_id": "hsk.workspace_settings_state@0",
                    "theme": "dark"
                }),
            },
        )
        .await
        .expect_err("wrong settings schema id must be rejected");

    assert!(matches!(
        err,
        StorageError::Validation(
            "workspace settings_state schema_id must be hsk.workspace_settings_state@1"
        )
    ));
    backend
        .close_and_remove()
        .await
        .expect("close embedded backend");
}

#[tokio::test]
async fn mt248_workspace_settings_rejects_duplicate_chords_before_eventledger() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let ws = backend.create_workspace().await;

    let err = backend
        .db
        .save_workspace_settings_state(
            &ws,
            WorkspaceSettingsStateInput {
                settings_state: settings_state("dark", "Mod-p", "Mod-p"),
            },
        )
        .await
        .expect_err("duplicate app keybindings must be rejected");

    assert!(matches!(
        err,
        StorageError::Validation("workspace settings_state duplicate keybinding chord")
    ));

    let event_count = backend
        .db
        .list_kernel_events_for_aggregate("workspace_settings_state", &ws)
        .await
        .expect("query settings event count")
        .len();
    assert_eq!(
        event_count, 0,
        "invalid settings must fail before EventLedger append"
    );
    backend
        .close_and_remove()
        .await
        .expect("close embedded backend");
}

#[tokio::test]
async fn mt248_workspace_settings_persists_with_eventledger() {
    let backend = manual_test_backend().await.expect("open embedded backend");
    let ws = backend.create_workspace().await;

    let initial = backend
        .db
        .get_workspace_settings_state(&ws)
        .await
        .expect("read empty settings");
    assert!(
        initial.is_none(),
        "new workspace should not synthesize settings state"
    );

    let first = backend
        .db
        .save_workspace_settings_state(
            &ws,
            WorkspaceSettingsStateInput {
                settings_state: settings_state("dark", "Alt-q", "Mod-Shift-p"),
            },
        )
        .await
        .expect("save first settings");

    assert_eq!(first.workspace_id, ws);
    assert_eq!(
        first.settings_state["schema_id"],
        WORKSPACE_SETTINGS_SCHEMA_ID
    );
    assert_eq!(first.settings_state["theme"], "dark");
    assert!(!first.event_ledger_event_id.trim().is_empty());

    let updated = backend
        .db
        .save_workspace_settings_state(
            &ws,
            WorkspaceSettingsStateInput {
                settings_state: settings_state("light", "Mod-p", "Alt-c"),
            },
        )
        .await
        .expect("save updated settings");

    assert_ne!(
        first.event_ledger_event_id, updated.event_ledger_event_id,
        "each settings mutation must retain its own EventLedger receipt"
    );

    let loaded = backend
        .db
        .get_workspace_settings_state(&ws)
        .await
        .expect("load settings")
        .expect("settings exists");
    assert_eq!(loaded.settings_state["theme"], "light");
    assert_eq!(loaded.event_ledger_event_id, updated.event_ledger_event_id);

    let events = backend
        .db
        .list_kernel_events_for_aggregate("workspace_settings_state", &ws)
        .await
        .expect("query matching kernel event");
    let event_count = events
        .iter()
        .filter(|event| {
            event.event_id == updated.event_ledger_event_id
                && event.event_type.as_str()
                    == KernelEventType::KnowledgeWorkspaceSettingsStateRecorded.as_str()
                && event.payload["workspace_id"] == ws
                && event.payload["settings_state"]["theme"] == "light"
        })
        .count();
    assert_eq!(event_count, 1);

    let row_count = events
        .iter()
        .filter(|event| {
            event.event_id == loaded.event_ledger_event_id
                && event.event_type.as_str()
                    == KernelEventType::KnowledgeWorkspaceSettingsStateRecorded.as_str()
        })
        .count();
    assert_eq!(
        row_count, 1,
        "workspace settings row must retain its EventLedger FK"
    );
    backend
        .close_and_remove()
        .await
        .expect("close embedded backend");
}
