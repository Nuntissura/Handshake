#![cfg(feature = "surreal-test-support")]

use handshake_core::{
    storage::surreal::{
        bootstrap_workspace_settings_schema, workspace_settings_test_event_count,
        workspace_settings_test_seed_legacy_unscoped_row, SurrealStorage, SurrealStorageConfig,
        SurrealWorkspaceSettingsError, SurrealWorkspaceSettingsStore,
        SurrealWorkspaceSettingsWrite,
    },
    swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
        WorkspaceScopeRef,
    },
};
use serde_json::{json, Value};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn provider_uses_canonical_kernel_event_ledger_not_a_parallel_audit_island() {
    let provider = include_str!("../src/storage/surreal/workspace_settings.rs");
    let schema = include_str!("../src/storage/surreal/workspace_settings_schema.surql");
    assert!(!provider.contains("workspace_settings_event_ledger"));
    assert!(!schema.contains("DEFINE TABLE OVERWRITE workspace_settings_event_ledger"));
    assert!(provider.contains("CREATE type::record('kernel_event_ledger'"));
    assert!(schema.contains("TYPE record<kernel_event_ledger>"));
    for field in [
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
    ] {
        assert!(provider.contains(field), "provider omitted {field}");
        assert!(schema.contains(field), "schema omitted {field}");
    }
}

fn exact_scope(workspace_id: &str) -> ResourceScope {
    ResourceScope::new(
        OwnerAccountId::from_uuid(Uuid::now_v7()),
        ActorPrincipalId::from_uuid(Uuid::now_v7()),
    )
    .with_session(AuthenticatedSessionRef::from_uuid(Uuid::now_v7()))
    .with_access_space(AccessSpaceRef::from_uuid(Uuid::now_v7()))
    .with_workspace(WorkspaceScopeRef::new(workspace_id).expect("workspace scope"))
}

fn settings(theme: &str) -> Value {
    json!({
        "schema_id": "hsk.workspace_settings_state@1",
        "theme": theme,
        "custom_theme_tokens": {},
        "keybindings": {
            "app.quick_switcher.open": "Mod-k",
            "app.command_palette.open": "Mod-Shift-p"
        },
        "settings": {
            "view_mode": "NSFW",
            "swarm_board_default_open": false,
            "swarm_max_actions_per_frame": 2,
            "swarm_model_sessions_max_concurrent": 1
        }
    })
}

async fn open_store(
    directory: &TempDir,
) -> (
    SurrealStorageConfig,
    SurrealStorage,
    SurrealWorkspaceSettingsStore,
) {
    let config = SurrealStorageConfig::for_scoped_store(
        directory.path().join("mt021-surreal"),
        "mt021_test",
        "workspace_settings",
    )
    .expect("surreal config");
    let storage = SurrealStorage::open(config.clone())
        .await
        .expect("open embedded SurrealDB");
    let store = SurrealWorkspaceSettingsStore::initialize(storage.clone())
        .await
        .expect("initialize workspace settings schema");
    (config, storage, store)
}

#[tokio::test]
async fn same_scope_generation_idempotency_and_restart_are_durable() {
    let directory = TempDir::new().expect("tempdir");
    let (config, storage, store) = open_store(&directory).await;
    let scope = exact_scope("workspace-a");

    assert!(store
        .get(&scope, "workspace-a")
        .await
        .expect("empty read")
        .is_none());

    let first = store
        .save(
            &scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("dark"),
                expected_generation: Some(0),
                idempotency_key: "request-1".to_owned(),
            },
        )
        .await
        .expect("first save");
    assert_eq!(first.generation, 1);
    assert_eq!(first.settings_state["theme"], "dark");

    let replay = store
        .save(
            &scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("dark"),
                expected_generation: Some(0),
                idempotency_key: "request-1".to_owned(),
            },
        )
        .await
        .expect("idempotent replay");
    assert_eq!(replay.generation, 1);
    assert_eq!(replay.event_ledger_event_id, first.event_ledger_event_id);
    assert_eq!(
        workspace_settings_test_event_count(&storage, &scope, "workspace-a")
            .await
            .expect("event count"),
        1
    );

    let stale = store
        .save(
            &scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("light"),
                expected_generation: Some(0),
                idempotency_key: "request-stale".to_owned(),
            },
        )
        .await
        .expect_err("stale generation must fail");
    assert!(matches!(
        stale,
        SurrealWorkspaceSettingsError::StaleGeneration {
            expected: 0,
            actual: 1
        }
    ));
    assert_eq!(
        workspace_settings_test_event_count(&storage, &scope, "workspace-a")
            .await
            .expect("event count after stale save"),
        1
    );

    let second = store
        .save(
            &scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("light"),
                expected_generation: Some(1),
                idempotency_key: "request-2".to_owned(),
            },
        )
        .await
        .expect("second save");
    assert_eq!(second.generation, 2);
    assert_ne!(second.event_ledger_event_id, first.event_ledger_event_id);

    drop(store);
    storage.shutdown().await.expect("shutdown embedded store");
    let reopened = SurrealStorage::open(config)
        .await
        .expect("reopen same namespace/database");
    let reopened_store = SurrealWorkspaceSettingsStore::initialize(reopened.clone())
        .await
        .expect("recheck schema state");
    let loaded = reopened_store
        .get(&scope, "workspace-a")
        .await
        .expect("post-restart read")
        .expect("settings remain durable");
    assert_eq!(loaded.generation, 2);
    assert_eq!(loaded.settings_state["theme"], "light");
    assert_eq!(loaded.event_ledger_event_id, second.event_ledger_event_id);
    reopened.shutdown().await.expect("shutdown reopened store");
}

#[tokio::test]
async fn every_single_scope_field_mismatch_fails_closed_without_identifier_leakage() {
    let directory = TempDir::new().expect("tempdir");
    let (_, storage, store) = open_store(&directory).await;
    let scope = exact_scope("workspace-a");
    store
        .save(
            &scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("dark"),
                expected_generation: Some(0),
                idempotency_key: "scope-fixture".to_owned(),
            },
        )
        .await
        .expect("fixture save");

    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::from_uuid(Uuid::now_v7());
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::from_uuid(Uuid::now_v7());
    let mut session = scope.clone();
    session.authenticated_session = Some(AuthenticatedSessionRef::from_uuid(Uuid::now_v7()));
    let mut access_space = scope.clone();
    access_space.access_space = Some(AccessSpaceRef::from_uuid(Uuid::now_v7()));

    for mismatch in [&owner, &actor, &session, &access_space] {
        assert!(store
            .get(mismatch, "workspace-a")
            .await
            .expect("mismatched read fails closed as not visible")
            .is_none());
    }

    let mut workspace = scope.clone();
    workspace.workspace = Some(WorkspaceScopeRef::new("workspace-b").expect("workspace scope"));
    assert!(matches!(
        store.get(&workspace, "workspace-a").await,
        Err(SurrealWorkspaceSettingsError::WorkspaceScopeMismatch)
    ));

    let mut incomplete = scope.clone();
    incomplete.authenticated_session = None;
    assert!(matches!(
        store.get(&incomplete, "workspace-a").await,
        Err(SurrealWorkspaceSettingsError::IncompleteScope)
    ));

    let unchanged = store
        .get(&scope, "workspace-a")
        .await
        .expect("same-scope read")
        .expect("same-scope row remains");
    assert_eq!(unchanged.generation, 1);
    assert_eq!(
        workspace_settings_test_event_count(&storage, &scope, "workspace-a")
            .await
            .expect("event count"),
        1
    );
    storage.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn malformed_duplicate_and_idempotency_conflict_writes_append_no_event() {
    let directory = TempDir::new().expect("tempdir");
    let (_, storage, store) = open_store(&directory).await;
    let scope = exact_scope("workspace-a");

    for invalid in [
        json!("not-an-object"),
        json!({"schema_id": "wrong"}),
        json!({
            "schema_id": "hsk.workspace_settings_state@1",
            "theme": "dark",
            "custom_theme_tokens": {},
            "keybindings": {
                "app.quick_switcher.open": "Ctrl-K",
                "app.command_palette.open": "Mod-k"
            },
            "settings": {"view_mode": "NSFW", "swarm_board_default_open": false}
        }),
    ] {
        assert!(matches!(
            store
                .save(
                    &scope,
                    "workspace-a",
                    SurrealWorkspaceSettingsWrite {
                        settings_state: invalid,
                        expected_generation: Some(0),
                        idempotency_key: Uuid::now_v7().to_string(),
                    },
                )
                .await,
            Err(SurrealWorkspaceSettingsError::Validation(_))
        ));
    }
    assert_eq!(
        workspace_settings_test_event_count(&storage, &scope, "workspace-a")
            .await
            .expect("no invalid-write events"),
        0
    );

    store
        .save(
            &scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("dark"),
                expected_generation: Some(0),
                idempotency_key: "same-key".to_owned(),
            },
        )
        .await
        .expect("accepted write");
    assert!(matches!(
        store
            .save(
                &scope,
                "workspace-a",
                SurrealWorkspaceSettingsWrite {
                    settings_state: settings("light"),
                    expected_generation: Some(1),
                    idempotency_key: "same-key".to_owned(),
                },
            )
            .await,
        Err(SurrealWorkspaceSettingsError::IdempotencyConflict)
    ));
    assert_eq!(
        workspace_settings_test_event_count(&storage, &scope, "workspace-a")
            .await
            .expect("idempotency conflict event count"),
        1
    );
    storage.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn concurrent_compare_and_set_has_one_winner_and_one_stale_loser() {
    let directory = TempDir::new().expect("tempdir");
    let (_, storage, store) = open_store(&directory).await;
    let scope = exact_scope("workspace-a");
    store
        .save(
            &scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("dark"),
                expected_generation: Some(0),
                idempotency_key: "seed".to_owned(),
            },
        )
        .await
        .expect("seed");

    let first_store = store.clone();
    let first_scope = scope.clone();
    let second_store = store.clone();
    let second_scope = scope.clone();
    let (first, second) = tokio::join!(
        first_store.save(
            &first_scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("light"),
                expected_generation: Some(1),
                idempotency_key: "race-a".to_owned(),
            }
        ),
        second_store.save(
            &second_scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("dark"),
                expected_generation: Some(1),
                idempotency_key: "race-b".to_owned(),
            }
        )
    );
    let outcomes = [first, second];
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| matches!(
                result,
                Err(SurrealWorkspaceSettingsError::StaleGeneration {
                    expected: 1,
                    actual: 2
                })
            ))
            .count(),
        1
    );
    assert_eq!(
        workspace_settings_test_event_count(&storage, &scope, "workspace-a")
            .await
            .expect("exactly seed plus race winner"),
        2
    );
    assert_eq!(
        store
            .get(&scope, "workspace-a")
            .await
            .expect("latest read")
            .expect("latest row")
            .generation,
        2
    );
    storage.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn legacy_unattributed_row_is_never_visible_or_mutated() {
    let directory = TempDir::new().expect("tempdir");
    let config = SurrealStorageConfig::for_scoped_store(
        directory.path().join("mt021-surreal"),
        "mt021_test",
        "workspace_settings",
    )
    .expect("surreal config");
    let storage = SurrealStorage::open(config).await.expect("open store");
    workspace_settings_test_seed_legacy_unscoped_row(&storage, "workspace-a", settings("dark"))
        .await
        .expect("seed legacy row before schema hardening");
    bootstrap_workspace_settings_schema(&storage)
        .await
        .expect("bootstrap strict schema over legacy data");
    let store = SurrealWorkspaceSettingsStore::new(storage.clone());
    let scope = exact_scope("workspace-a");
    assert!(store
        .get(&scope, "workspace-a")
        .await
        .expect("legacy row must not make the exact query fail")
        .is_none());
    let stored = store
        .save(
            &scope,
            "workspace-a",
            SurrealWorkspaceSettingsWrite {
                settings_state: settings("light"),
                expected_generation: Some(0),
                idempotency_key: "scoped-first-write".to_owned(),
            },
        )
        .await
        .expect("scoped authority is created separately");
    assert_eq!(stored.generation, 1);
    assert_eq!(stored.settings_state["theme"], "light");
    assert_eq!(
        workspace_settings_test_event_count(&storage, &scope, "workspace-a")
            .await
            .expect("only scoped event is visible"),
        1
    );
    storage.shutdown().await.expect("shutdown");
}
