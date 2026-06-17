//! WP-KERNEL-009 MT-248 durable workspace-settings proof.
//!
//! Settings, theme, and app keybindings are operator support state, but they
//! must be workspace-scoped PostgreSQL state with EventLedger receipts rather
//! than localStorage-only UI preferences.

mod knowledge_pg_support;

use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{
    Database, StorageError, WORKSPACE_SETTINGS_SCHEMA_ID, WorkspaceSettingsStateInput,
};
use knowledge_pg_support::knowledge_pg;
use serde_json::json;
use sqlx::Row;

macro_rules! pg_or_skip {
    () => {{
        match knowledge_pg().await {
            Some(pg) => pg,
            None => {
                panic!("MT-248 workspace settings proof requires real PostgreSQL");
            }
        }
    }};
}

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
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let err = pg
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
}

#[tokio::test]
async fn mt248_workspace_settings_rejects_wrong_schema_id() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let err = pg
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
}

#[tokio::test]
async fn mt248_workspace_settings_rejects_duplicate_chords_before_eventledger() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let err = pg
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

    let mut conn = pg.raw_connection().await;
    let event_count: i64 = sqlx::query(
        r#"
        SELECT COUNT(*)::BIGINT AS count
        FROM kernel_event_ledger
        WHERE aggregate_type = 'workspace_settings_state'
          AND aggregate_id = $1
        "#,
    )
    .bind(&ws)
    .fetch_one(&mut conn)
    .await
    .expect("query settings event count")
    .get("count");
    assert_eq!(
        event_count, 0,
        "invalid settings must fail before EventLedger append"
    );
}

#[tokio::test]
async fn mt248_workspace_settings_persists_with_eventledger() {
    let pg = pg_or_skip!();
    let ws = pg.create_workspace().await;

    let initial = pg
        .db
        .get_workspace_settings_state(&ws)
        .await
        .expect("read empty settings");
    assert!(
        initial.is_none(),
        "new workspace should not synthesize settings state"
    );

    let first = pg
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

    let updated = pg
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

    let loaded = pg
        .db
        .get_workspace_settings_state(&ws)
        .await
        .expect("load settings")
        .expect("settings exists");
    assert_eq!(loaded.settings_state["theme"], "light");
    assert_eq!(loaded.event_ledger_event_id, updated.event_ledger_event_id);

    let mut conn = pg.raw_connection().await;
    let event_count: i64 = sqlx::query(
        r#"
        SELECT COUNT(*)::BIGINT AS count
        FROM kernel_event_ledger
        WHERE event_id = $1
          AND event_type = $2
          AND aggregate_type = 'workspace_settings_state'
          AND payload ->> 'workspace_id' = $3
          AND payload -> 'settings_state' ->> 'theme' = 'light'
        "#,
    )
    .bind(&updated.event_ledger_event_id)
    .bind(KernelEventType::KnowledgeWorkspaceSettingsStateRecorded.as_str())
    .bind(&ws)
    .fetch_one(&mut conn)
    .await
    .expect("query matching kernel event")
    .get("count");
    assert_eq!(event_count, 1);

    let row_count: i64 = sqlx::query(
        r#"
        SELECT COUNT(*)::BIGINT AS count
        FROM knowledge_workspace_settings_states s
        JOIN kernel_event_ledger e
          ON e.event_id = s.event_ledger_event_id
        WHERE s.workspace_id = $1
          AND e.event_type = $2
        "#,
    )
    .bind(&ws)
    .bind(KernelEventType::KnowledgeWorkspaceSettingsStateRecorded.as_str())
    .fetch_one(&mut conn)
    .await
    .expect("query settings row event FK")
    .get("count");
    assert_eq!(
        row_count, 1,
        "workspace settings row must retain its EventLedger FK"
    );
}
