//! MT-136 real embedded-store proofs for the general `Database` surface.
//!
//! These tests deliberately use the production `SurrealDatabase` adapter over
//! isolated RocksDB directories. No mock, memory backend, server URL, or skip
//! path is available. Each test closes and reopens its store before its final
//! assertions so a process-local cache cannot satisfy the durability proof.

use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{
    mt136_proof_harness::{embedded_proof_backend, EmbeddedProofBackend},
    SCHEMA_REVISION,
};
use crate::{
    kernel::{KernelActor, KernelEventType, NewKernelEvent},
    preferences::{
        PreferenceConstraint, PreferenceSchemaEntry, PreferenceScope, PreferenceScopeKind,
        PreferenceSource, RedactionClass,
    },
    storage::{
        stage_artifacts::{NewStageCaptureArtifact, StageArtifactStore},
        BlockViewDefinition, BlockViewField, BlockViewKind, BlockViewQuery, BlockViewSort,
        BlockViewSortDirection, CalendarEventExportMode, CalendarEventStatus, CalendarEventUpsert,
        CalendarEventVisibility, CalendarEventWindowQuery, CalendarSourceProviderType,
        CalendarSourceSyncState, CalendarSourceUpsert, CalendarSourceWritePolicy,
        CompensateLoomCanvasStageCard, Database, DebugBreakpointInput, LoomAuthorityBackend,
        LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate, LoomCanvasPlacementUpdate,
        LoomCanvasStageProvenance, LoomEdgeCreatedBy, LoomEdgeType, LoomFolderSortMode,
        LoomFolderUpdate, LoomSearchResultKind, LoomSearchSourceKind, MediaTier, MediaTierStatus,
        MediaTierUpsert, NewAsset, NewCanvas, NewCanvasEdge, NewCanvasNode, NewLoomBlock,
        NewLoomCanvasPlacement, NewLoomCanvasStageCard, NewLoomEdge, NewLoomFolder, NewWorkspace,
        PreviewStatus, QuickSwitcherRecentInput, StorageError, StorageResult,
        WorkbenchLayoutStateInput, WorkspaceSearchBookmarkStateInput, WorkspaceSettingsStateInput,
        WriteContext, LOOM_CANVAS_BOARD_SCHEMA_ID, LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
    },
};

fn ctx() -> WriteContext {
    WriteContext::human(Some("mt-136-surface-proof".to_owned()))
}

async fn reopen(backend: EmbeddedProofBackend) -> StorageResult<EmbeddedProofBackend> {
    backend.reopen().await
}

async fn workspace(database: &dyn Database, name: &str) -> StorageResult<String> {
    Ok(database
        .create_workspace(
            &ctx(),
            NewWorkspace {
                name: name.to_owned(),
            },
        )
        .await?
        .id)
}

async fn block(
    database: &dyn Database,
    workspace_id: &str,
    title: &str,
    content_type: LoomBlockContentType,
    full_text: Option<&str>,
) -> StorageResult<String> {
    let mut derived = LoomBlockDerived::default();
    derived.full_text_index = full_text.map(str::to_owned);
    Ok(database
        .create_loom_block(
            &ctx(),
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_owned(),
                content_type,
                document_id: None,
                asset_id: None,
                title: Some(title.to_owned()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived,
            },
        )
        .await?
        .block_id)
}

async fn bridged_block(
    database: &dyn Database,
    workspace_id: &str,
    title: &str,
    content_type: LoomBlockContentType,
    full_text: Option<&str>,
) -> StorageResult<String> {
    let block_id = block(database, workspace_id, title, content_type, full_text).await?;
    database
        .bridge_loom_block_to_knowledge(&ctx(), workspace_id, &block_id)
        .await?;
    Ok(block_id)
}

fn preference_entry() -> PreferenceSchemaEntry {
    PreferenceSchemaEntry {
        preference_id: "editor.mt136-proof",
        namespace: "editor",
        scope_kind: PreferenceScopeKind::Workspace,
        label: "MT-136 proof preference",
        constraint: PreferenceConstraint::Bool,
        redaction_class: RedactionClass::Public,
        default_value: json!(false),
    }
}

async fn general_state_media_preferences_and_guard_survive_reopen() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    database.ping().await?;
    assert_eq!(database.migration_version().await?, SCHEMA_REVISION);
    let workspace_id = workspace(database.as_ref(), "MT-136 general surface").await?;

    let asset = database
        .create_asset(
            &ctx(),
            NewAsset {
                workspace_id: workspace_id.clone(),
                kind: "image".to_owned(),
                mime: "image/png".to_owned(),
                original_filename: Some("proof.png".to_owned()),
                content_hash: "a".repeat(64),
                size_bytes: 128,
                width: Some(8),
                height: Some(8),
                classification: "low".to_owned(),
                exportable: true,
                is_proxy_of: None,
                proxy_asset_id: None,
            },
        )
        .await?;
    assert_eq!(
        database
            .get_asset(&workspace_id, &asset.asset_id)
            .await?
            .asset_id,
        asset.asset_id
    );
    assert!(database
        .get_asset(&workspace_id, "missing-asset")
        .await
        .is_err());
    eprintln!("MT136_PROOF_STEP_PASS general_state.asset");

    let failed = database
        .upsert_media_tier(
            &ctx(),
            MediaTierUpsert {
                workspace_id: workspace_id.clone(),
                asset_id: asset.asset_id.clone(),
                tier: MediaTier::Poster,
                status: MediaTierStatus::Failed,
                tier_asset_id: None,
                content_hash: None,
                failure_reason: Some("proof failure".to_owned()),
            },
        )
        .await?;
    assert_eq!(failed.status, MediaTierStatus::Failed);
    assert_eq!(
        database
            .get_media_tier(&workspace_id, &asset.asset_id, MediaTier::Poster)
            .await?
            .expect("poster tier")
            .status,
        MediaTierStatus::Failed
    );
    assert_eq!(
        database
            .list_media_tiers(&workspace_id, &asset.asset_id)
            .await?
            .len(),
        1
    );
    assert_eq!(
        database.list_failed_media_tiers(&workspace_id).await?.len(),
        1
    );
    let retried = database
        .set_media_tier_status(
            &ctx(),
            &workspace_id,
            &asset.asset_id,
            MediaTier::Poster,
            MediaTierStatus::Pending,
            None,
        )
        .await?;
    assert_eq!(retried.status, MediaTierStatus::Pending);
    assert_eq!(retried.attempt_count, 1);
    assert_eq!(
        database
            .delete_media_tiers(&ctx(), &workspace_id, &asset.asset_id)
            .await?,
        1
    );
    assert_eq!(
        database
            .get_asset(&workspace_id, &asset.asset_id)
            .await?
            .asset_id,
        asset.asset_id
    );
    database
        .upsert_media_tier(
            &ctx(),
            MediaTierUpsert {
                workspace_id: workspace_id.clone(),
                asset_id: asset.asset_id.clone(),
                tier: MediaTier::Thumb,
                status: MediaTierStatus::Ready,
                tier_asset_id: Some(asset.asset_id.clone()),
                content_hash: Some(asset.content_hash.clone()),
                failure_reason: None,
            },
        )
        .await?;
    eprintln!("MT136_PROOF_STEP_PASS general_state.media_tiers");

    let invalid_recent = database
        .record_quick_switcher_recent(
            &workspace_id,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::UserManualPage,
                source_kind: LoomSearchSourceKind::UserManualPage,
                ref_id: " ".to_owned(),
                title: "invalid".to_owned(),
                excerpt: String::new(),
                metadata: json!({}),
            },
        )
        .await;
    assert!(matches!(invalid_recent, Err(StorageError::Validation(_))));
    database
        .record_quick_switcher_recent(
            &workspace_id,
            QuickSwitcherRecentInput {
                result_kind: LoomSearchResultKind::UserManualPage,
                source_kind: LoomSearchSourceKind::UserManualPage,
                ref_id: "manual-proof".to_owned(),
                title: "Proof manual".to_owned(),
                excerpt: "embedded".to_owned(),
                metadata: json!({"proof": true}),
            },
        )
        .await?;
    assert_eq!(
        database
            .list_quick_switcher_recents(&workspace_id, 10)
            .await?
            .len(),
        1
    );
    eprintln!("MT136_PROOF_STEP_PASS general_state.quick_switcher");

    database
        .save_workbench_layout_state(
            &workspace_id,
            WorkbenchLayoutStateInput {
                layout_state: json!({
                    "schema_id": "hsk.workbench_layout_state@1",
                    "activePaneId": "pane-a",
                    "activeModule": "MAIN",
                    "splitWeights": {"vertical": 0.5, "horizontal": 0.5},
                    "drawers": {"project": true, "file": true, "bottom": false},
                    "panes": [
                        {"id": "pane-a", "module": "MAIN", "activeTab": "workspace", "tabs": ["workspace"], "locked": false, "projectRef": "", "activeDocumentId": null, "activeCanvasId": null, "openDocuments": []},
                        {"id": "pane-b", "module": "MAIN", "activeTab": "workspace", "tabs": ["workspace"], "locked": false, "projectRef": "", "activeDocumentId": null, "activeCanvasId": null, "openDocuments": []},
                        {"id": "pane-c", "module": "MAIN", "activeTab": "workspace", "tabs": ["workspace"], "locked": false, "projectRef": "", "activeDocumentId": null, "activeCanvasId": null, "openDocuments": []},
                        {"id": "pane-d", "module": "MAIN", "activeTab": "workspace", "tabs": ["workspace"], "locked": false, "projectRef": "", "activeDocumentId": null, "activeCanvasId": null, "openDocuments": []}
                    ]
                }),
            },
        )
        .await?;
    assert!(database
        .get_workbench_layout_state(&workspace_id)
        .await?
        .is_some());
    eprintln!("MT136_PROOF_STEP_PASS general_state.workbench_layout");
    database
        .save_workspace_settings_state(
            &workspace_id,
            WorkspaceSettingsStateInput {
                settings_state: json!({
                    "schema_id": "hsk.workspace_settings_state@1",
                    "theme": "dark",
                    "custom_theme_tokens": {},
                    "keybindings": {
                        "app.quick_switcher.open": "Mod-k",
                        "app.command_palette.open": "Mod-p"
                    },
                    "settings": {
                        "view_mode": "SFW",
                        "swarm_board_default_open": false
                    }
                }),
            },
        )
        .await?;
    assert!(database
        .get_workspace_settings_state(&workspace_id)
        .await?
        .is_some());
    eprintln!("MT136_PROOF_STEP_PASS general_state.workspace_settings");
    database
        .save_workspace_search_bookmark_state(
            &workspace_id,
            WorkspaceSearchBookmarkStateInput {
                bookmark_state: json!({
                    "schema_id": "hsk.workspace_search_bookmark_state@1",
                    "bookmarks": []
                }),
            },
        )
        .await?;
    assert!(database
        .get_workspace_search_bookmark_state(&workspace_id)
        .await?
        .is_some());
    eprintln!("MT136_PROOF_STEP_PASS general_state.search_bookmarks");

    let scope = PreferenceScope::workspace(workspace_id.clone());
    let entry = preference_entry();
    assert_eq!(
        database.preference_get(&scope, &entry).await?.value,
        json!(false)
    );
    let (set, _) = database
        .preference_set(
            &scope,
            &entry,
            json!(true),
            PreferenceSource::Operator,
            "mt-136-operator",
        )
        .await?;
    assert_eq!(set.value, json!(true));
    assert!(database
        .preference_set(
            &scope,
            &entry,
            json!("wrong-type"),
            PreferenceSource::Operator,
            "mt-136-operator",
        )
        .await
        .is_err());
    assert_eq!(
        database
            .preference_history(&scope, entry.preference_id)
            .await?
            .len(),
        1
    );
    assert_eq!(
        database
            .preference_projection(&scope, &[entry.clone()])
            .await?
            .len(),
        1
    );
    let (reset, _) = database
        .preference_reset(&scope, &entry, "mt-136-operator")
        .await?;
    assert_eq!(reset.value, json!(false));
    eprintln!("MT136_PROOF_STEP_PASS general_state.preferences");

    let metadata = database
        .validate_write_with_guard(&ctx(), "mt-136-guard-resource")
        .await?;
    assert_eq!(metadata.resource_id, "mt-136-guard-resource");
    eprintln!("MT136_PROOF_STEP_PASS general_state.guard");

    drop(database);
    let backend = reopen(backend).await?;
    let database = backend.database.clone();
    database.ping().await?;
    assert_eq!(
        database
            .get_asset(&workspace_id, &asset.asset_id)
            .await?
            .asset_id,
        asset.asset_id
    );
    assert_eq!(
        database
            .list_media_tiers(&workspace_id, &asset.asset_id)
            .await?
            .len(),
        1
    );
    assert_eq!(
        database
            .list_quick_switcher_recents(&workspace_id, 10)
            .await?
            .len(),
        1
    );
    assert!(database
        .get_workbench_layout_state(&workspace_id)
        .await?
        .is_some());
    assert!(database
        .get_workspace_settings_state(&workspace_id)
        .await?
        .is_some());
    assert!(database
        .get_workspace_search_bookmark_state(&workspace_id)
        .await?
        .is_some());
    assert_eq!(database.preference_get(&scope, &entry).await?.revision, 2);
    eprintln!("MT136_PROOF_STEP_PASS general_state.reopen");
    drop(database);
    backend.close_and_remove().await?;
    Ok(())
}

async fn loom_navigation_folders_wiki_import_and_debug_survive_reopen() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let workspace_id = workspace(database.as_ref(), "MT-136 Loom surface").await?;

    let asset = database
        .create_asset(
            &ctx(),
            NewAsset {
                workspace_id: workspace_id.clone(),
                kind: "image".to_owned(),
                mime: "image/jpeg".to_owned(),
                original_filename: Some("loom-proof.jpg".to_owned()),
                content_hash: "b".repeat(64),
                size_bytes: 64,
                width: Some(4),
                height: Some(4),
                classification: "low".to_owned(),
                exportable: true,
                is_proxy_of: None,
                proxy_asset_id: None,
            },
        )
        .await?;
    let asset_block = database
        .create_loom_block(
            &ctx(),
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::File,
                document_id: None,
                asset_id: Some(asset.asset_id.clone()),
                title: Some("Asset proof".to_owned()),
                original_filename: Some("loom-proof.jpg".to_owned()),
                content_hash: Some(asset.content_hash.clone()),
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await?;
    assert_eq!(
        database
            .find_loom_block_by_asset_id(&workspace_id, &asset.asset_id)
            .await?
            .expect("asset block")
            .block_id,
        asset_block.block_id
    );
    database
        .set_loom_block_preview(
            &ctx(),
            &workspace_id,
            &asset_block.block_id,
            PreviewStatus::Generated,
            Some(asset.asset_id.clone()),
            None,
        )
        .await?;
    assert_eq!(
        database
            .get_loom_block(&workspace_id, &asset_block.block_id)
            .await?
            .derived
            .preview_status,
        PreviewStatus::Generated
    );
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.asset_preview");

    let target = bridged_block(
        database.as_ref(),
        &workspace_id,
        "Roadmap",
        LoomBlockContentType::Note,
        Some("canonical roadmap target"),
    )
    .await?;
    let source = bridged_block(
        database.as_ref(),
        &workspace_id,
        "Linked source",
        LoomBlockContentType::Note,
        Some("A linked source around Roadmap context"),
    )
    .await?;
    let unlinked = bridged_block(
        database.as_ref(),
        &workspace_id,
        "Unlinked source",
        LoomBlockContentType::Note,
        Some("The Roadmap also appears here without an edge"),
    )
    .await?;
    let tag = bridged_block(
        database.as_ref(),
        &workspace_id,
        "Proof tag",
        LoomBlockContentType::TagHub,
        Some("Proof tag"),
    )
    .await?;
    let subtag = bridged_block(
        database.as_ref(),
        &workspace_id,
        "Proof subtag",
        LoomBlockContentType::TagHub,
        Some("Proof subtag"),
    )
    .await?;
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.bridged_blocks");

    database
        .create_loom_edge(
            &ctx(),
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace_id.clone(),
                source_block_id: source.clone(),
                target_block_id: target.clone(),
                edge_type: LoomEdgeType::Mention,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await?;
    database
        .create_loom_edge(
            &ctx(),
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace_id.clone(),
                source_block_id: target.clone(),
                target_block_id: tag.clone(),
                edge_type: LoomEdgeType::Tag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await?;
    database
        .create_loom_edge(
            &ctx(),
            NewLoomEdge {
                edge_id: None,
                workspace_id: workspace_id.clone(),
                source_block_id: subtag.clone(),
                target_block_id: tag.clone(),
                edge_type: LoomEdgeType::SubTag,
                created_by: LoomEdgeCreatedBy::User,
                crdt_site_id: None,
                source_anchor: None,
            },
        )
        .await?;

    assert_eq!(
        database.get_backlinks(&workspace_id, &target).await?.len(),
        1
    );
    assert_eq!(
        database
            .get_outgoing_edges(&workspace_id, &source)
            .await?
            .len(),
        1
    );
    assert_eq!(
        database
            .get_backlinks_with_context(&workspace_id, &target)
            .await?
            .len(),
        1
    );
    assert!(database
        .scan_unlinked_mentions(&workspace_id, &target, &[], 20)
        .await?
        .iter()
        .any(|mention| mention.source_block.block_id == unlinked));
    assert!(
        database
            .local_graph(&workspace_id, &target, 2, &[], 50)
            .await?
            .nodes
            .len()
            >= 3
    );
    assert!(
        database
            .global_graph(&workspace_id, &[], 100, 100)
            .await?
            .nodes
            .len()
            >= 5
    );
    assert_eq!(database.list_tag_hubs(&workspace_id, 20, 0).await?.len(), 2);
    assert_eq!(
        database
            .get_tag_hub(&workspace_id, &tag)
            .await?
            .block
            .block_id,
        tag
    );
    assert_eq!(
        database
            .list_blocks_for_tag(&workspace_id, &tag, true, 20, 0)
            .await?
            .len(),
        1
    );
    assert!(database.get_tag_hub(&workspace_id, &target).await.is_err());
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.edges_graphs_tags");

    database
        .update_loom_block(
            &ctx(),
            &workspace_id,
            &target,
            LoomBlockUpdate {
                pinned: Some(true),
                ..Default::default()
            },
        )
        .await?;
    database
        .set_loom_block_pin_order(&ctx(), &workspace_id, &target, Some(3))
        .await?;
    let unpinned = database
        .remove_loom_block_pin(&ctx(), &workspace_id, &target)
        .await?;
    assert!(!unpinned.pinned);
    assert_eq!(unpinned.pin_order, None);
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.pin_state");

    let parent = database
        .create_loom_folder(
            &workspace_id,
            NewLoomFolder {
                folder_id: None,
                workspace_id: workspace_id.clone(),
                parent_folder_id: None,
                name: "Parent".to_owned(),
                color: None,
                sort_mode: LoomFolderSortMode::NameAsc,
                sort_order: None,
                project_ref: None,
            },
        )
        .await?;
    let child = database
        .create_loom_folder(
            &workspace_id,
            NewLoomFolder {
                folder_id: None,
                workspace_id: workspace_id.clone(),
                parent_folder_id: Some(parent.folder_id.clone()),
                name: "Child".to_owned(),
                color: Some("#123456".to_owned()),
                sort_mode: LoomFolderSortMode::Manual,
                sort_order: Some(1),
                project_ref: None,
            },
        )
        .await?;
    assert_eq!(
        database
            .get_loom_folder(&workspace_id, &child.folder_id)
            .await?
            .name,
        "Child"
    );
    assert_eq!(database.list_loom_folders(&workspace_id).await?.len(), 2);
    let updated_folder = database
        .update_loom_folder(
            &workspace_id,
            &child.folder_id,
            LoomFolderUpdate {
                name: Some("Child renamed".to_owned()),
                color: Some(None),
                ..Default::default()
            },
        )
        .await?;
    assert_eq!(updated_folder.name, "Child renamed");
    assert!(database
        .update_loom_folder(
            &workspace_id,
            &parent.folder_id,
            LoomFolderUpdate {
                parent_folder_id: Some(Some(child.folder_id.clone())),
                ..Default::default()
            },
        )
        .await
        .is_err());
    database
        .add_block_to_loom_folder(&workspace_id, &child.folder_id, &target, Some(1))
        .await?;
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.folders");
    assert_eq!(
        database
            .list_loom_folder_blocks(&workspace_id, &child.folder_id, 20, 0)
            .await?
            .len(),
        1
    );
    database
        .remove_block_from_loom_folder(&workspace_id, &child.folder_id, &target)
        .await?;
    assert!(database
        .list_loom_folder_blocks(&workspace_id, &child.folder_id, 20, 0)
        .await?
        .is_empty());
    database
        .add_block_to_loom_folder(&workspace_id, &child.folder_id, &target, Some(1))
        .await?;

    let projection = database
        .compile_loom_wiki_projection(
            &workspace_id,
            "Proof topic",
            &[target.clone(), source.clone()],
        )
        .await?;
    assert_eq!(
        database
            .get_loom_wiki_projection(&workspace_id, &projection.projection_id)
            .await?
            .projection_id,
        projection.projection_id
    );
    assert!(
        !database
            .loom_wiki_projection_is_stale(&workspace_id, &projection.projection_id)
            .await?
    );
    database
        .update_loom_block(
            &ctx(),
            &workspace_id,
            &source,
            LoomBlockUpdate {
                title: Some("Linked source changed".to_owned()),
                ..Default::default()
            },
        )
        .await?;
    assert!(
        database
            .loom_wiki_projection_is_stale(&workspace_id, &projection.projection_id)
            .await?
    );
    database
        .regenerate_loom_wiki_projection(&workspace_id, &projection.projection_id)
        .await?;
    let overlay = database
        .add_loom_wiki_overlay(
            &workspace_id,
            &projection.projection_id,
            "Operator note",
            Some(&target),
        )
        .await?;
    assert_eq!(
        database
            .list_loom_wiki_overlays(&workspace_id, &projection.projection_id)
            .await?
            .len(),
        1
    );
    assert!(matches!(
        database
            .delete_loom_wiki_overlay("mt136-wrong-workspace", &overlay.overlay_id)
            .await,
        Err(StorageError::NotFound("loom_wiki_overlay"))
    ));
    database
        .delete_loom_wiki_overlay(&workspace_id, &overlay.overlay_id)
        .await?;
    assert!(matches!(
        database
            .delete_loom_wiki_overlay(&workspace_id, &overlay.overlay_id)
            .await,
        Err(StorageError::NotFound("loom_wiki_overlay"))
    ));
    assert!(database
        .list_loom_wiki_overlays(&workspace_id, &projection.projection_id)
        .await?
        .is_empty());
    let disposable_projection = database
        .compile_loom_wiki_projection(&workspace_id, "Disposable", &[target.clone()])
        .await?;
    database
        .delete_loom_wiki_projection(&workspace_id, &disposable_projection.projection_id)
        .await?;
    assert!(database
        .get_loom_wiki_projection(&workspace_id, &disposable_projection.projection_id)
        .await
        .is_err());
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.wiki");

    let imported = database
        .import_markdown_to_loom(
            &ctx(),
            &workspace_id,
            "Imported proof",
            "# Imported proof\n\nRoadmap body.",
        )
        .await?;
    database
        .bridge_loom_block_to_knowledge(&ctx(), &workspace_id, &imported.block.block_id)
        .await?;
    database
        .add_block_to_loom_folder(
            &workspace_id,
            &child.folder_id,
            &imported.block.block_id,
            Some(2),
        )
        .await?;
    let breadcrumbs = database
        .loom_block_breadcrumbs(&workspace_id, &imported.block.block_id)
        .await?;
    assert!(breadcrumbs
        .crumbs
        .iter()
        .any(|crumb| crumb.kind == "folder"));
    assert!(database
        .loom_block_breadcrumbs(&workspace_id, "missing-block")
        .await
        .is_err());

    assert!(database
        .list_debug_breakpoints(&imported.rich_document_id)
        .await?
        .is_empty());
    database
        .set_debug_breakpoints(
            &imported.rich_document_id,
            &workspace_id,
            vec![DebugBreakpointInput {
                source_url: "file:///mt136-proof.js".to_owned(),
                line: 7,
                condition: Some("ready".to_owned()),
                verified: true,
            }],
        )
        .await?;
    assert_eq!(
        database
            .list_debug_breakpoints(&imported.rich_document_id)
            .await?
            .len(),
        1
    );
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.import_debug");

    let bridges = database
        .list_loom_block_knowledge_bridges(&workspace_id)
        .await?;
    assert!(bridges.iter().any(|bridge| bridge.block_id == target));
    assert!(database
        .get_loom_block_knowledge_bridge(&workspace_id, &target)
        .await?
        .is_some());
    let debug = database
        .loom_visual_debug_snapshot(&workspace_id, &target, "Roadmap", 20)
        .await?;
    assert_eq!(debug.start_block_id, target);
    assert_eq!(
        debug.authority_backend,
        LoomAuthorityBackend::SurrealEventLedger
    );
    assert_eq!(debug.authority_backend.as_str(), "surreal_event_ledger");
    assert!(!debug.authority_backend.as_str().contains("postgres"));
    assert!(matches!(
        database
            .loom_visual_debug_snapshot("mt136-missing-workspace", &target, "Roadmap", 20)
            .await,
        Err(StorageError::NotFound("workspace"))
    ));
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.bridge_visual_debug");

    drop(database);
    let backend = reopen(backend).await?;
    let database = backend.database.clone();
    assert_eq!(
        database.get_backlinks(&workspace_id, &target).await?.len(),
        1
    );
    assert_eq!(
        database
            .get_loom_folder(&workspace_id, &child.folder_id)
            .await?
            .name,
        "Child renamed"
    );
    assert_eq!(
        database
            .get_loom_wiki_projection(&workspace_id, &projection.projection_id)
            .await?
            .projection_id,
        projection.projection_id
    );
    assert_eq!(
        database
            .list_debug_breakpoints(&imported.rich_document_id)
            .await?
            .len(),
        1
    );
    assert_eq!(
        database
            .find_loom_block_by_asset_id(&workspace_id, &asset.asset_id)
            .await?
            .expect("durable asset block")
            .block_id,
        asset_block.block_id
    );
    database
        .delete_loom_folder(&workspace_id, &parent.folder_id)
        .await?;
    assert!(database
        .get_loom_block(&workspace_id, &target)
        .await
        .is_ok());
    eprintln!("MT136_PROOF_STEP_PASS loom_navigation.reopen");
    drop(database);
    backend.close_and_remove().await?;
    Ok(())
}

fn stage_receipt(event_type: KernelEventType, idempotency_key: &str) -> NewKernelEvent {
    NewKernelEvent::builder(
        "mt-136-stage-task",
        "mt-136-stage-session",
        event_type,
        KernelActor::Operator("mt-136-surface-proof".to_owned()),
    )
    .aggregate("stage_capture_proof", "pending")
    .idempotency_key(idempotency_key)
    .correlation_id("mt-136-stage-correlation")
    .source_component("mt136_database_surface_proof_a")
    .payload(json!({"proof": true}))
    .build()
    .expect("valid stage proof event")
}

fn board_state(pan_x: f64) -> Value {
    json!({
        "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
        "pan_x": pan_x,
        "pan_y": 0.0,
        "zoom": 1.0
    })
}

async fn canvas_board_stage_block_view_calendar_and_canvas_crud_survive_reopen() -> StorageResult<()>
{
    let backend = embedded_proof_backend().await?;
    let database = backend.database.clone();
    let workspace_id = workspace(database.as_ref(), "MT-136 canvas calendar surface").await?;

    let canvas_block = bridged_block(
        database.as_ref(),
        &workspace_id,
        "Canvas board",
        LoomBlockContentType::Canvas,
        Some("canvas"),
    )
    .await?;
    let first = bridged_block(
        database.as_ref(),
        &workspace_id,
        "First card",
        LoomBlockContentType::Note,
        Some("first"),
    )
    .await?;
    let second = bridged_block(
        database.as_ref(),
        &workspace_id,
        "Second card",
        LoomBlockContentType::Note,
        Some("second"),
    )
    .await?;
    database
        .create_canvas_board(&ctx(), &workspace_id, &canvas_block, board_state(0.0))
        .await?;
    assert_eq!(
        database
            .get_canvas_board(&workspace_id, &canvas_block)
            .await?
            .placements
            .len(),
        0
    );
    database
        .update_canvas_board_state(&ctx(), &workspace_id, &canvas_block, board_state(10.0))
        .await?;
    assert!(database
        .create_canvas_board(&ctx(), &workspace_id, &first, board_state(0.0))
        .await
        .is_err());
    eprintln!("MT136_PROOF_STEP_PASS canvas_calendar.board");

    let first_placement = database
        .place_block_on_canvas(
            &ctx(),
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_block.clone(),
                workspace_id: workspace_id.clone(),
                placed_block_id: first.clone(),
                x: 1.0,
                y: 2.0,
                w: 100.0,
                h: 80.0,
                z_index: 1,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await?;
    let second_placement = database
        .place_block_on_canvas(
            &ctx(),
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_block.clone(),
                workspace_id: workspace_id.clone(),
                placed_block_id: second.clone(),
                x: 110.0,
                y: 2.0,
                w: 100.0,
                h: 80.0,
                z_index: 2,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await?;
    let moved = database
        .update_canvas_placement(
            &ctx(),
            &workspace_id,
            &first_placement.placement_id,
            LoomCanvasPlacementUpdate {
                x: Some(5.0),
                group_id: Some(Some("proof-group".to_owned())),
                ..Default::default()
            },
        )
        .await?;
    assert_eq!(moved.x, 5.0);
    let visual_edge = database
        .add_canvas_visual_edge(
            &ctx(),
            &workspace_id,
            &canvas_block,
            &first_placement.placement_id,
            &second_placement.placement_id,
            Some("visual only".to_owned()),
        )
        .await?;
    assert_eq!(
        database
            .get_canvas_board(&workspace_id, &canvas_block)
            .await?
            .visual_edges
            .len(),
        1
    );
    database
        .remove_canvas_visual_edge(&ctx(), &workspace_id, &visual_edge.visual_edge_id)
        .await?;
    database
        .remove_canvas_placement(&ctx(), &workspace_id, &second_placement.placement_id)
        .await?;
    assert!(database
        .get_loom_block(&workspace_id, &second)
        .await
        .is_ok());
    eprintln!("MT136_PROOF_STEP_PASS canvas_calendar.placements");

    let stage_bytes = b"MT-136 stage card".to_vec();
    let request_hash = hex::encode(Sha256::digest(b"mt-136-stage-card-request"));
    let artifact = StageArtifactStore::new(backend.storage.clone())
        .insert_stage_artifact(NewStageCaptureArtifact {
            workspace_id: workspace_id.clone(),
            content_kind: "canvas_node".to_owned(),
            label: "Stage card".to_owned(),
            content_type: "text/markdown".to_owned(),
            content_json: json!({"text": "Stage proof"}),
            content_bytes: stage_bytes,
            source_ref: None,
            idempotency_key: "mt-136-stage-card-artifact".to_owned(),
            request_hash,
            actor_kind: "operator".to_owned(),
            actor_id: "mt-136-surface-proof".to_owned(),
            correlation_id: "mt-136-stage-correlation".to_owned(),
            approval_id: "mt-136-stage-approval".to_owned(),
            decision_receipt: stage_receipt(
                KernelEventType::ToolDecisionRecorded,
                "mt-136-stage-card-decision",
            ),
            receipt: stage_receipt(KernelEventType::ArtifactStored, "mt-136-stage-card-stored"),
        })
        .await?
        .artifact;
    let provenance = LoomCanvasStageProvenance {
        schema_id: LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA.to_owned(),
        artifact_id: artifact.artifact_id.clone(),
        sha256: artifact.content_sha256.clone(),
        manifest_ref: artifact.manifest_ref.clone(),
        causal_action_id: artifact.correlation_id.clone(),
    };
    let provenance_key = hex::encode(Sha256::digest(serde_json::to_vec(&provenance)?));
    let stage_request = NewLoomCanvasStageCard {
        canvas_block_id: canvas_block.clone(),
        workspace_id: workspace_id.clone(),
        title: format!("Stage capture {}", provenance.artifact_id),
        markdown: serde_json::to_string(&provenance)?,
        stage_provenance_key: provenance_key.clone(),
        stage_provenance: provenance.clone(),
        x: 10.0,
        y: 20.0,
        w: 200.0,
        h: 120.0,
        z_index: 3,
    };
    let stage_card = database
        .create_stage_canvas_card(&ctx(), stage_request)
        .await?;
    assert!(stage_card.created_by_request);
    let compensation_request = CompensateLoomCanvasStageCard {
        canvas_block_id: canvas_block.clone(),
        workspace_id: workspace_id.clone(),
        placement_id: stage_card.placement.placement_id.clone(),
        placed_block_id: stage_card.block.block_id.clone(),
        stage_provenance_key: provenance_key,
        stage_provenance: provenance,
    };
    let mut wrong_workspace_compensation = compensation_request.clone();
    wrong_workspace_compensation.workspace_id = "mt136-missing-workspace".to_owned();
    assert!(matches!(
        database
            .compensate_stage_canvas_card(&ctx(), wrong_workspace_compensation)
            .await,
        Err(StorageError::Validation(
            "Canvas Stage compensation receipt is absent but owned authority residue remains"
        ))
    ));
    assert!(database
        .get_canvas_board(&workspace_id, &canvas_block)
        .await?
        .placements
        .iter()
        .any(|placement| placement.placement_id == stage_card.placement.placement_id));
    let compensation = database
        .compensate_stage_canvas_card(&ctx(), compensation_request.clone())
        .await?;
    assert!(compensation.removed_by_request);
    let repeated_compensation = database
        .compensate_stage_canvas_card(&ctx(), compensation_request.clone())
        .await?;
    assert!(!repeated_compensation.removed_by_request);
    let mut missing_workspace_compensation = compensation_request;
    missing_workspace_compensation.workspace_id = "mt136-missing-workspace".to_owned();
    let missing_workspace_compensation = database
        .compensate_stage_canvas_card(&ctx(), missing_workspace_compensation)
        .await?;
    assert!(!missing_workspace_compensation.removed_by_request);
    eprintln!("MT136_PROOF_STEP_PASS canvas_calendar.stage_compensation");

    let view_id = uuid::Uuid::now_v7().to_string();
    let definition = BlockViewDefinition {
        kind: BlockViewKind::Table,
        query: BlockViewQuery::default(),
        columns: vec![BlockViewField::Title],
        group_by: None,
        sort: Some(BlockViewSort {
            field: BlockViewField::Title,
            direction: BlockViewSortDirection::Asc,
        }),
        calendar_date_field: None,
    };
    database
        .create_block_view(
            &ctx(),
            &workspace_id,
            &view_id,
            Some("Proof view".to_owned()),
            definition.clone(),
        )
        .await?;
    assert_eq!(
        database
            .get_block_view(&workspace_id, &view_id)
            .await?
            .block
            .block_id,
        view_id
    );
    let updated_definition = BlockViewDefinition {
        sort: Some(BlockViewSort {
            field: BlockViewField::Updated,
            direction: BlockViewSortDirection::Desc,
        }),
        ..definition
    };
    database
        .update_block_view_definition(&ctx(), &workspace_id, &view_id, updated_definition.clone())
        .await?;
    assert!(
        database
            .query_block_view_results(&workspace_id, &updated_definition, 50, 0)
            .await?
            .total_returned
            >= 3
    );
    eprintln!("MT136_PROOF_STEP_PASS canvas_calendar.block_view");

    let source_id = "mt-136-calendar-source".to_owned();
    database
        .upsert_calendar_source(
            &ctx(),
            CalendarSourceUpsert {
                id: source_id.clone(),
                workspace_id: workspace_id.clone(),
                display_name: "Proof calendar".to_owned(),
                provider_type: CalendarSourceProviderType::Local,
                write_policy: CalendarSourceWritePolicy::TwoWayMirror,
                default_tzid: "UTC".to_owned(),
                auto_export: false,
                credentials_ref: None,
                provider_calendar_id: None,
                capability_profile_id: None,
                config: json!({}),
                sync_state: CalendarSourceSyncState::default(),
            },
        )
        .await?;
    assert_eq!(
        database.list_calendar_sources(&workspace_id).await?.len(),
        1
    );
    assert!(database
        .get_calendar_source(&workspace_id, &source_id)
        .await?
        .is_some());
    let start = Utc.with_ymd_and_hms(2026, 8, 22, 9, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 8, 22, 10, 0, 0).unwrap();
    database
        .upsert_calendar_event(
            &ctx(),
            CalendarEventUpsert {
                id: "mt-136-calendar-event".to_owned(),
                workspace_id: workspace_id.clone(),
                source_id: source_id.clone(),
                external_id: Some("provider-event".to_owned()),
                external_etag: Some("etag-proof".to_owned()),
                title: "Proof event".to_owned(),
                description: None,
                location: None,
                start_ts_utc: start,
                end_ts_utc: end,
                start_local: Some("2026-08-22T09:00:00".to_owned()),
                end_local: Some("2026-08-22T10:00:00".to_owned()),
                tzid: "UTC".to_owned(),
                all_day: false,
                start_date: None,
                end_date_exclusive: None,
                was_floating: false,
                normalization_note: None,
                status: CalendarEventStatus::Confirmed,
                visibility: CalendarEventVisibility::Private,
                export_mode: CalendarEventExportMode::FullExport,
                rrule: None,
                rdate: Vec::new(),
                exdate: Vec::new(),
                is_recurring: false,
                series_id: None,
                instance_key: None,
                is_override: false,
                source_last_seen_at: None,
                attendees: json!([]),
                links: json!([]),
                provider_payload: None,
            },
        )
        .await?;
    let event_query = CalendarEventWindowQuery {
        workspace_id: workspace_id.clone(),
        query_start_date: NaiveDate::from_ymd_opt(2026, 8, 22).unwrap(),
        query_end_date_exclusive: NaiveDate::from_ymd_opt(2026, 8, 23).unwrap(),
        window_start_utc: Utc.with_ymd_and_hms(2026, 8, 22, 0, 0, 0).unwrap(),
        window_end_utc: Utc.with_ymd_and_hms(2026, 8, 23, 0, 0, 0).unwrap(),
        source_ids: vec![source_id.clone()],
    };
    assert_eq!(
        database
            .query_calendar_events(event_query.clone())
            .await?
            .len(),
        1
    );
    eprintln!("MT136_PROOF_STEP_PASS canvas_calendar.calendar");

    let canvas = database
        .create_canvas(
            &ctx(),
            NewCanvas {
                workspace_id: workspace_id.clone(),
                title: "Generic canvas".to_owned(),
            },
        )
        .await?;
    assert_eq!(database.list_canvases(&workspace_id).await?.len(), 1);
    let graph = database
        .update_canvas_graph(
            &ctx(),
            &canvas.id,
            vec![
                NewCanvasNode {
                    id: Some("node-a".to_owned()),
                    kind: "note".to_owned(),
                    position_x: 1.0,
                    position_y: 2.0,
                    data: Some(json!({"title": "A"})),
                },
                NewCanvasNode {
                    id: Some("node-b".to_owned()),
                    kind: "note".to_owned(),
                    position_x: 3.0,
                    position_y: 4.0,
                    data: Some(json!({"title": "B"})),
                },
            ],
            vec![NewCanvasEdge {
                id: Some("edge-a-b".to_owned()),
                from_node_id: "node-a".to_owned(),
                to_node_id: "node-b".to_owned(),
                kind: "link".to_owned(),
            }],
        )
        .await?;
    assert_eq!(graph.nodes.len(), 2);
    let stale_time = graph.canvas.updated_at - chrono::Duration::seconds(1);
    assert!(database
        .rename_canvas(&ctx(), &canvas.id, "stale", Some(stale_time))
        .await
        .is_err());
    database
        .rename_canvas(
            &ctx(),
            &canvas.id,
            "Renamed generic canvas",
            Some(graph.canvas.updated_at),
        )
        .await?;
    assert_eq!(
        database
            .get_canvas_with_graph(&canvas.id)
            .await?
            .nodes
            .len(),
        2
    );
    eprintln!("MT136_PROOF_STEP_PASS canvas_calendar.canvas_crud");

    drop(database);
    let backend = reopen(backend).await?;
    let database = backend.database.clone();
    assert_eq!(
        database
            .get_canvas_board(&workspace_id, &canvas_block)
            .await?
            .placements
            .len(),
        1
    );
    assert_eq!(
        database
            .get_block_view(&workspace_id, &view_id)
            .await?
            .block
            .block_id,
        view_id
    );
    assert_eq!(database.query_calendar_events(event_query).await?.len(), 1);
    assert_eq!(
        database
            .get_canvas_with_graph(&canvas.id)
            .await?
            .nodes
            .len(),
        2
    );
    database
        .delete_calendar_data_by_source(&ctx(), &workspace_id, &source_id)
        .await?;
    assert!(database
        .get_calendar_source(&workspace_id, &source_id)
        .await?
        .is_none());
    database.delete_canvas(&ctx(), &canvas.id).await?;
    assert!(database.list_canvases(&workspace_id).await?.is_empty());
    eprintln!("MT136_PROOF_STEP_PASS canvas_calendar.reopen_cleanup");
    drop(database);
    backend.close_and_remove().await?;
    Ok(())
}

pub(super) async fn run_all() -> StorageResult<()> {
    eprintln!("MT136_PROOF_CASE_START database_surface_a.general_state");
    general_state_media_preferences_and_guard_survive_reopen().await?;
    eprintln!("MT136_PROOF_CASE_PASS database_surface_a.general_state");
    eprintln!("MT136_PROOF_CASE_START database_surface_a.loom_navigation");
    loom_navigation_folders_wiki_import_and_debug_survive_reopen().await?;
    eprintln!("MT136_PROOF_CASE_PASS database_surface_a.loom_navigation");
    eprintln!("MT136_PROOF_CASE_START database_surface_a.canvas_calendar");
    canvas_board_stage_block_view_calendar_and_canvas_crud_survive_reopen().await?;
    eprintln!("MT136_PROOF_CASE_PASS database_surface_a.canvas_calendar");
    Ok(())
}

#[cfg(test)]
#[tokio::test]
async fn mt136_database_surface_proof_a() -> StorageResult<()> {
    run_all().await
}
