//! WP-KERNEL-005 MT-014 bulk-operation proof.
//!
//! Uses the embedded SurrealDB/EventLedger path. Bulk operations must validate
//! the entire target set before any mutation, then commit target changes, one
//! durable receipt, and one EventLedger event atomically.

mod atelier_surreal_support;

use handshake_core::atelier::exports::{ExportFormat, NewExportRequest, EXPORT_REQUESTED};
use handshake_core::atelier::search::search_event_family;
use handshake_core::atelier::{
    event_family, AtelierError, AtelierStore, BulkTagRequest, BulkTrashMediaRequest,
    DeletionArchiveRequest, DeletionImpactPreviewRequest, DeletionRestoreRequest,
    DeletionTargetKind, DeletionTargetRef, NewCharacter, NewMediaAsset, NewSheetVersion,
    SheetFieldEdit, SheetFieldEditRequest,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::Database;
use std::sync::Arc;
use uuid::Uuid;

async fn connected_store() -> (
    AtelierStore,
    Arc<dyn Database>,
    atelier_surreal_support::AtelierSurrealHarness,
) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    (harness.atelier.clone(), harness.database.clone(), harness)
}

async fn character(store: &AtelierStore, label: &str) -> handshake_core::atelier::Character {
    store
        .create_character(&NewCharacter {
            public_id: format!("mt-014-{label}-{}", Uuid::new_v4()),
            display_name: format!("MT-014 {label}"),
        })
        .await
        .expect("create character")
}

async fn media_asset(store: &AtelierStore, label: &str) -> Uuid {
    let artifact = atelier_surreal_support::write_native_media_artifact(
        format!("mt-014-{label}-media").as_bytes(),
    );
    store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash,
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some("mt-014-bulk-test".to_string()),
            artifact_ref: artifact.artifact_ref,
        })
        .await
        .expect("materialize media asset")
        .asset_id
}

fn raw_sheet(name: &str) -> String {
    format!(
        "\
CHARACTER TEMPLATE (v2.00)
IDENTITY
CHAR-ID-002 - Name: <string>
CHAR-ID-003 - Alias: <string>
Freeform note for {name}
"
    )
}

async fn sheet(
    store: &AtelierStore,
    character_internal_id: Uuid,
    label: &str,
) -> handshake_core::atelier::SheetVersion {
    store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id,
            raw_text: raw_sheet(label),
            author: "mt-014-test".to_string(),
            tool: Some("bulk-operations-test".to_string()),
        })
        .await
        .expect("append sheet version")
}

async fn assert_one_bulk_receipt_event(database: &Arc<dyn Database>, receipt_id: Uuid) {
    let events = database
        .list_kernel_events_for_aggregate("atelier_bulk_operation_receipt", &receipt_id.to_string())
        .await
        .expect("list canonical receipt EventLedger events");
    let matching = events
        .iter()
        .filter(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"] == event_family::BULK_OPERATION_APPLIED
        })
        .count();
    assert_eq!(
        matching, 1,
        "bulk receipt must have one canonical EventLedger row"
    );
}

async fn event_count(
    store: &AtelierStore,
    event_family: &str,
    aggregate_type: &str,
    aggregate_id: &str,
) -> i64 {
    store
        .count_events_for_aggregate(event_family, aggregate_type, aggregate_id)
        .await
        .expect("count aggregate events")
}

async fn receipt_event_payload(
    database: &Arc<dyn Database>,
    receipt_id: Uuid,
) -> serde_json::Value {
    database
        .list_kernel_events_for_aggregate("atelier_bulk_operation_receipt", &receipt_id.to_string())
        .await
        .expect("list bulk receipt EventLedger events")
        .into_iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"] == event_family::BULK_OPERATION_APPLIED
        })
        .map(|event| event.payload["atelier_payload"].clone())
        .expect("read bulk receipt EventLedger payload")
}

async fn bulk_receipt_count(store: &AtelierStore, operation: &str) -> i64 {
    let _ = operation;
    store
        .count_events(event_family::BULK_OPERATION_APPLIED)
        .await
        .expect("count bulk operation receipt events")
}

async fn export_request_count(store: &AtelierStore) -> i64 {
    store
        .count_events(EXPORT_REQUESTED)
        .await
        .expect("count export request events")
}

async fn aggregate_projection_count(
    database: &Arc<dyn Database>,
    aggregate_type: &str,
    aggregate_id: Uuid,
) -> i64 {
    database
        .list_kernel_events_for_aggregate(aggregate_type, &aggregate_id.to_string())
        .await
        .expect("count aggregate EventLedger events")
        .len() as i64
}

async fn trash_marker_exists(store: &AtelierStore, target_type: &str, target_id: Uuid) -> bool {
    match target_type {
        "media_asset" => store
            .is_media_asset_trashed(target_id)
            .await
            .expect("read media trash marker"),
        "sheet_version" => store
            .is_sheet_version_trashed(target_id)
            .await
            .expect("read sheet trash marker"),
        other => panic!("unsupported trash target type: {other}"),
    }
}

#[tokio::test]
async fn bulk_tag_characters_prevalidates_all_targets_and_records_receipt() {
    let (store, database, _harness) = connected_store().await;

    let first = character(&store, "tag-first").await;
    let second = character(&store, "tag-second").await;
    let missing = Uuid::new_v4();
    let tags = vec![" blonde ".to_string(), "Action Pose".to_string()];
    let receipts_before_failure = bulk_receipt_count(&store, "bulk_tag_characters").await;
    let tag_events_before_success = event_count(
        &store,
        search_event_family::CHARACTER_TAGGED,
        "atelier_character_tag",
        "bulk",
    )
    .await;

    let failed = store
        .bulk_tag_characters_with_receipt(&BulkTagRequest {
            character_internal_ids: vec![first.internal_id, missing],
            tags: tags.clone(),
            requested_by: "mt-014-test".to_string(),
        })
        .await;
    assert!(
        failed.is_err(),
        "missing target must reject the whole bulk tag request"
    );
    assert!(
        store
            .list_character_tags(first.internal_id)
            .await
            .expect("list tags after failed bulk")
            .is_empty(),
        "failed bulk tag must not mutate valid targets before detecting a missing target"
    );
    assert_eq!(
        receipts_before_failure,
        bulk_receipt_count(&store, "bulk_tag_characters").await,
        "failed bulk tag must not write a receipt"
    );

    let receipt = store
        .bulk_tag_characters_with_receipt(&BulkTagRequest {
            character_internal_ids: vec![first.internal_id, second.internal_id],
            tags,
            requested_by: "mt-014-test".to_string(),
        })
        .await
        .expect("bulk tag characters with receipt");
    assert_eq!(receipt.operation, "bulk_tag_characters");
    assert_eq!(receipt.target_count, 2);
    assert_eq!(receipt.mutation_count, 4);
    assert_one_bulk_receipt_event(&database, receipt.receipt_id).await;
    assert_eq!(
        event_count(
            &store,
            search_event_family::CHARACTER_TAGGED,
            "atelier_character_tag",
            "bulk",
        )
        .await,
        tag_events_before_success + 1,
        "bulk tag must still emit the documented tag-domain event"
    );

    for character_id in [first.internal_id, second.internal_id] {
        let texts: Vec<String> = store
            .list_character_tags(character_id)
            .await
            .expect("list character tags")
            .into_iter()
            .map(|tag| tag.text)
            .collect();
        assert!(texts.contains(&"blonde".to_string()));
        assert!(texts.contains(&"action pose".to_string()));
    }
}

#[tokio::test]
async fn bulk_sheet_exports_prevalidate_all_targets_and_record_receipt() {
    let (store, database, _harness) = connected_store().await;

    let character = character(&store, "export").await;
    let sheet = sheet(&store, character.internal_id, "export").await;
    let before = export_request_count(&store).await;
    let receipts_before_failure = bulk_receipt_count(&store, "bulk_request_sheet_exports").await;

    let failed = store
        .bulk_request_sheet_exports(
            &[
                NewExportRequest {
                    character_internal_id: character.internal_id,
                    sheet_version_id: sheet.version_id,
                    format: ExportFormat::Markdown,
                    label: Some("good".to_string()),
                    requested_by: "mt-014-test".to_string(),
                },
                NewExportRequest {
                    character_internal_id: character.internal_id,
                    sheet_version_id: Uuid::new_v4(),
                    format: ExportFormat::Json,
                    label: Some("bad".to_string()),
                    requested_by: "mt-014-test".to_string(),
                },
            ],
            "mt-014-test",
        )
        .await;
    assert!(
        failed.is_err(),
        "missing sheet target must reject the whole export batch"
    );
    let after = export_request_count(&store).await;
    assert_eq!(
        before, after,
        "failed bulk export must not write the valid request"
    );
    assert_eq!(
        receipts_before_failure,
        bulk_receipt_count(&store, "bulk_request_sheet_exports").await,
        "failed bulk export must not write a receipt"
    );
    let before_mismatch = after;
    let receipts_before_mismatch = bulk_receipt_count(&store, "bulk_request_sheet_exports").await;
    let mismatch = store
        .bulk_request_sheet_exports(
            &[NewExportRequest {
                character_internal_id: character.internal_id,
                sheet_version_id: sheet.version_id,
                format: ExportFormat::Markdown,
                label: Some("actor-mismatch".to_string()),
                requested_by: "other-actor".to_string(),
            }],
            "mt-014-test",
        )
        .await;
    assert!(
        mismatch.is_err(),
        "bulk export must reject divergent row and receipt requested_by actors"
    );
    let after_mismatch = export_request_count(&store).await;
    assert_eq!(
        before_mismatch, after_mismatch,
        "bulk export actor mismatch must not write a request"
    );
    assert_eq!(
        receipts_before_mismatch,
        bulk_receipt_count(&store, "bulk_request_sheet_exports").await,
        "bulk export actor mismatch must not write a receipt"
    );

    let result = store
        .bulk_request_sheet_exports(
            &[
                NewExportRequest {
                    character_internal_id: character.internal_id,
                    sheet_version_id: sheet.version_id,
                    format: ExportFormat::Markdown,
                    label: Some("sheet-md".to_string()),
                    requested_by: "mt-014-test".to_string(),
                },
                NewExportRequest {
                    character_internal_id: character.internal_id,
                    sheet_version_id: sheet.version_id,
                    format: ExportFormat::Json,
                    label: Some("sheet-json".to_string()),
                    requested_by: "mt-014-test".to_string(),
                },
            ],
            "mt-014-test",
        )
        .await
        .expect("bulk export requests");
    assert_eq!(result.receipt.operation, "bulk_request_sheet_exports");
    assert_eq!(result.receipt.target_count, 2);
    assert_eq!(result.receipt.mutation_count, 2);
    assert_eq!(result.exports.len(), 2);
    assert_one_bulk_receipt_event(&database, result.receipt.receipt_id).await;
    for export in &result.exports {
        assert_eq!(
            event_count(
                &store,
                EXPORT_REQUESTED,
                "atelier_export_request",
                &export.export_id.to_string(),
            )
            .await,
            1,
            "bulk-created export request must emit the normal export requested event"
        );
    }
    let receipt_payload = store
        .get_bulk_operation_receipt(result.receipt.receipt_id)
        .await
        .expect("read durable export receipt")
        .payload;
    let exports = receipt_payload["exports"]
        .as_array()
        .expect("export receipt names validated targets");
    assert_eq!(exports.len(), 2);
    assert_eq!(
        exports[0]["sheet_version_id"],
        serde_json::json!(sheet.version_id)
    );
    assert_eq!(exports[0]["requested_by"], "mt-014-test");
    let event_payload = receipt_event_payload(&database, result.receipt.receipt_id).await;
    let event_json =
        serde_json::to_string(&event_payload).expect("serialize sanitized receipt event payload");
    assert!(
        !event_json.contains(&character.internal_id.to_string()),
        "bulk receipt EventLedger payload must not leak character storage internal_id"
    );
    assert!(
        event_payload["receipt_payload"]["exports"]
            .as_array()
            .expect("bulk receipt event carries receipt payload")
            .iter()
            .all(|export| export.get("character_internal_id").is_none()),
        "bulk receipt EventLedger projection must not carry character_internal_id fields"
    );
    assert_eq!(
        event_payload["receipt_payload"]["exports"]
            .as_array()
            .expect("bulk receipt event carries receipt payload")
            .len(),
        2
    );
}

#[tokio::test]
async fn bulk_trash_media_prevalidates_all_targets_and_records_receipt() {
    let (store, database, _harness) = connected_store().await;

    let first = media_asset(&store, "trash-first").await;
    let second = media_asset(&store, "trash-second").await;
    let receipts_before_failure = bulk_receipt_count(&store, "bulk_trash_media_assets").await;

    let failed = store
        .bulk_trash_media_assets(&BulkTrashMediaRequest {
            asset_ids: vec![first, Uuid::new_v4()],
            reason: "operator rejected".to_string(),
            requested_by: "mt-014-test".to_string(),
        })
        .await;
    assert!(
        failed.is_err(),
        "missing media target must reject the whole trash batch"
    );
    assert!(
        !store
            .is_media_asset_trashed(first)
            .await
            .expect("read trash marker after failed bulk"),
        "failed bulk trash must not mark the valid target"
    );
    assert_eq!(
        receipts_before_failure,
        bulk_receipt_count(&store, "bulk_trash_media_assets").await,
        "failed bulk trash must not write a receipt"
    );

    let receipt = store
        .bulk_trash_media_assets(&BulkTrashMediaRequest {
            asset_ids: vec![first, second],
            reason: "operator rejected".to_string(),
            requested_by: "mt-014-test".to_string(),
        })
        .await
        .expect("bulk trash media assets");
    assert_eq!(receipt.operation, "bulk_trash_media_assets");
    assert_eq!(receipt.target_count, 2);
    assert_eq!(receipt.mutation_count, 2);
    assert_one_bulk_receipt_event(&database, receipt.receipt_id).await;
    assert!(store
        .is_media_asset_trashed(first)
        .await
        .expect("first asset trashed"));
    assert!(store
        .is_media_asset_trashed(second)
        .await
        .expect("second asset trashed"));
}

#[tokio::test]
async fn deletion_impact_preview_archive_and_restore_cover_media_and_sheet_versions() {
    let (store, database, _harness) = connected_store().await;

    let character = character(&store, "recoverable-delete").await;
    let asset_id = media_asset(&store, "recoverable-delete").await;
    let sheet = sheet(&store, character.internal_id, "recoverable-delete").await;
    let targets = vec![
        DeletionTargetRef {
            target_type: DeletionTargetKind::MediaAsset,
            target_id: asset_id,
        },
        DeletionTargetRef {
            target_type: DeletionTargetKind::SheetVersion,
            target_id: sheet.version_id,
        },
    ];
    let preview_receipts_before = bulk_receipt_count(&store, "preview_deletion_impact").await;

    let preview = store
        .preview_deletion_impact(&DeletionImpactPreviewRequest {
            targets: targets.clone(),
            requested_by: "mt-024-test".to_string(),
            reason: "operator cleanup preview".to_string(),
        })
        .await
        .expect("preview deletion impact");
    assert_eq!(preview.target_count, 2);
    assert_eq!(preview.would_archive_count, 2);
    assert_eq!(preview.already_archived_count, 0);
    assert!(
        preview
            .targets
            .iter()
            .all(|target| !target.currently_archived),
        "fresh targets must preview as not already archived"
    );
    assert!(
        !trash_marker_exists(&store, "media_asset", asset_id).await,
        "preview must not soft-delete media assets"
    );
    assert!(
        !trash_marker_exists(&store, "sheet_version", sheet.version_id).await,
        "preview must not soft-delete sheet versions"
    );
    assert_eq!(
        preview_receipts_before,
        bulk_receipt_count(&store, "preview_deletion_impact").await,
        "preview must not create a durable mutation receipt"
    );

    let archive = store
        .archive_deletion_targets(&DeletionArchiveRequest {
            targets: targets.clone(),
            requested_by: "mt-024-test".to_string(),
            reason: "operator cleanup archive".to_string(),
        })
        .await
        .expect("archive deletion targets");
    assert_eq!(archive.operation, "archive_deletion_targets");
    assert_eq!(archive.target_count, 2);
    assert_eq!(archive.mutation_count, 2);
    assert_one_bulk_receipt_event(&database, archive.receipt_id).await;
    assert!(store
        .is_media_asset_trashed(asset_id)
        .await
        .expect("media asset marker after archive"));
    assert!(store
        .is_sheet_version_trashed(sheet.version_id)
        .await
        .expect("sheet version marker after archive"));
    assert!(
        store
            .list_media_gallery_assets(500)
            .await
            .expect("list media assets after archive")
            .iter()
            .any(|asset| asset.asset_id == asset_id),
        "archive must not physically delete media rows"
    );
    assert!(
        store
            .sheet_version_history(character.internal_id)
            .await
            .expect("sheet history after archive")
            .iter()
            .any(|version| version.version_id == sheet.version_id),
        "archive must not remove append-only sheet versions"
    );
    assert!(
        !store
            .list_media_gallery_assets(500)
            .await
            .expect("list gallery after archive")
            .iter()
            .any(|asset| asset.asset_id == asset_id),
        "archived media assets must be hidden from the normal gallery"
    );

    let archived_preview = store
        .preview_deletion_impact(&DeletionImpactPreviewRequest {
            targets: targets.clone(),
            requested_by: "mt-024-test".to_string(),
            reason: "operator cleanup preview after archive".to_string(),
        })
        .await
        .expect("preview archived targets");
    assert_eq!(archived_preview.would_archive_count, 0);
    assert_eq!(archived_preview.already_archived_count, 2);
    assert!(
        archived_preview
            .targets
            .iter()
            .all(|target| target.currently_archived),
        "archived targets must be reported as already archived"
    );

    let restore = store
        .restore_deletion_targets(&DeletionRestoreRequest {
            targets,
            requested_by: "mt-024-test".to_string(),
            reason: "operator restore".to_string(),
        })
        .await
        .expect("restore deletion targets");
    assert_eq!(restore.operation, "restore_deletion_targets");
    assert_eq!(restore.target_count, 2);
    assert_eq!(restore.mutation_count, 2);
    assert_one_bulk_receipt_event(&database, restore.receipt_id).await;
    assert!(!store
        .is_media_asset_trashed(asset_id)
        .await
        .expect("media asset marker after restore"));
    assert!(!store
        .is_sheet_version_trashed(sheet.version_id)
        .await
        .expect("sheet version marker after restore"));
    assert!(
        store
            .list_media_gallery_assets(500)
            .await
            .expect("list gallery after restore")
            .iter()
            .any(|asset| asset.asset_id == asset_id),
        "restored media assets must return to the normal gallery"
    );
}

#[tokio::test]
async fn deletion_archive_restore_prevalidate_all_targets_before_marker_mutation() {
    let (store, _database, _harness) = connected_store().await;

    let character = character(&store, "recoverable-delete-prevalidate").await;
    let asset_id = media_asset(&store, "recoverable-delete-prevalidate").await;
    let sheet = sheet(
        &store,
        character.internal_id,
        "recoverable-delete-prevalidate",
    )
    .await;
    let missing_sheet_id = Uuid::new_v4();
    let valid_targets = vec![
        DeletionTargetRef {
            target_type: DeletionTargetKind::MediaAsset,
            target_id: asset_id,
        },
        DeletionTargetRef {
            target_type: DeletionTargetKind::SheetVersion,
            target_id: sheet.version_id,
        },
    ];
    let invalid_targets = vec![
        DeletionTargetRef {
            target_type: DeletionTargetKind::MediaAsset,
            target_id: asset_id,
        },
        DeletionTargetRef {
            target_type: DeletionTargetKind::SheetVersion,
            target_id: missing_sheet_id,
        },
    ];

    let archive_receipts_before = bulk_receipt_count(&store, "archive_deletion_targets").await;
    let archive_failed = store
        .archive_deletion_targets(&DeletionArchiveRequest {
            targets: invalid_targets.clone(),
            requested_by: "mt-024-test".to_string(),
            reason: "invalid archive".to_string(),
        })
        .await;
    assert!(
        archive_failed.is_err(),
        "missing sheet version must reject the whole archive request"
    );
    assert!(
        !trash_marker_exists(&store, "media_asset", asset_id).await,
        "failed archive must not mark valid media before rejecting a missing target"
    );
    assert_eq!(
        archive_receipts_before,
        bulk_receipt_count(&store, "archive_deletion_targets").await,
        "failed archive must not write a receipt"
    );

    store
        .archive_deletion_targets(&DeletionArchiveRequest {
            targets: valid_targets,
            requested_by: "mt-024-test".to_string(),
            reason: "valid archive".to_string(),
        })
        .await
        .expect("archive valid targets before restore prevalidation probe");
    let restore_receipts_before = bulk_receipt_count(&store, "restore_deletion_targets").await;
    let restore_failed = store
        .restore_deletion_targets(&DeletionRestoreRequest {
            targets: invalid_targets,
            requested_by: "mt-024-test".to_string(),
            reason: "invalid restore".to_string(),
        })
        .await;
    assert!(
        restore_failed.is_err(),
        "missing sheet version must reject the whole restore request"
    );
    assert!(
        trash_marker_exists(&store, "media_asset", asset_id).await,
        "failed restore must not remove the valid media marker before rejecting a missing target"
    );
    assert!(
        trash_marker_exists(&store, "sheet_version", sheet.version_id).await,
        "failed restore must not remove the valid sheet marker before rejecting a missing target"
    );
    assert_eq!(
        restore_receipts_before,
        bulk_receipt_count(&store, "restore_deletion_targets").await,
        "failed restore must not write a receipt"
    );
}

#[tokio::test]
async fn bulk_sheet_field_edits_prevalidate_all_targets_and_record_receipt() {
    let (store, database, _harness) = connected_store().await;

    let first = character(&store, "field-first").await;
    let second = character(&store, "field-second").await;
    let first_sheet = sheet(&store, first.internal_id, "field-first").await;
    let second_sheet = sheet(&store, second.internal_id, "field-second").await;
    let first_parse = store
        .parse_sheet_template_version(
            first_sheet.version_id,
            "mt-014-template",
            Some("test://mt-014/field-first"),
        )
        .await
        .expect("parse first bulk sheet");
    let second_parse = store
        .parse_sheet_template_version(
            second_sheet.version_id,
            "mt-014-template",
            Some("test://mt-014/field-second"),
        )
        .await
        .expect("parse second bulk sheet");
    let receipts_before_failure = bulk_receipt_count(&store, "bulk_apply_sheet_field_edits").await;
    let first_events_before_failure =
        aggregate_projection_count(&database, "atelier_sheet_version", first_sheet.version_id)
            .await;
    let second_events_before_failure =
        aggregate_projection_count(&database, "atelier_sheet_version", second_sheet.version_id)
            .await;

    let valid_request = SheetFieldEditRequest {
        version_id: first_sheet.version_id,
        template_id: "mt-014-template".to_string(),
        source_path: Some("test://mt-014/field-first".to_string()),
        expected_template_hash: Some(first_parse.ast.template_hash.clone()),
        actor_role: "operator".to_string(),
        edits: vec![SheetFieldEdit {
            block_instance_id: None,
            field_id: "CHAR-ID-002".to_string(),
            replacement_text: "Valid Name".to_string(),
        }],
        author: "mt-014-test".to_string(),
        tool: Some("bulk-field-test".to_string()),
    };
    let invalid_request = SheetFieldEditRequest {
        version_id: second_sheet.version_id,
        source_path: Some("test://mt-014/field-second".to_string()),
        expected_template_hash: Some(second_parse.ast.template_hash.clone()),
        edits: vec![SheetFieldEdit {
            block_instance_id: None,
            field_id: "CHAR-MISSING-999".to_string(),
            replacement_text: "Should Not Apply".to_string(),
        }],
        ..valid_request.clone()
    };

    let failed = store
        .bulk_apply_sheet_field_edits(&[valid_request.clone(), invalid_request], "mt-014-test")
        .await;
    assert!(
        failed.is_err(),
        "unknown field target must reject the whole sheet edit batch"
    );
    assert_eq!(
        store
            .sheet_version_history(first.internal_id)
            .await
            .expect("history after failed bulk")
            .len(),
        1,
        "failed bulk field edit must not append the valid edit"
    );
    assert_eq!(
        receipts_before_failure,
        bulk_receipt_count(&store, "bulk_apply_sheet_field_edits").await,
        "failed bulk field edit must not write a receipt"
    );
    assert_eq!(
        first_events_before_failure,
        aggregate_projection_count(&database, "atelier_sheet_version", first_sheet.version_id).await,
        "failed bulk field edit must not write EventLedger/projection rows for valid prechecked target"
    );
    assert_eq!(
        second_events_before_failure,
        aggregate_projection_count(&database, "atelier_sheet_version", second_sheet.version_id).await,
        "failed bulk field edit must not write EventLedger/projection rejection rows for invalid target"
    );

    let second_request = SheetFieldEditRequest {
        version_id: second_sheet.version_id,
        source_path: Some("test://mt-014/field-second".to_string()),
        expected_template_hash: Some(second_parse.ast.template_hash.clone()),
        edits: vec![SheetFieldEdit {
            block_instance_id: None,
            field_id: "CHAR-ID-003".to_string(),
            replacement_text: "Second Alias".to_string(),
        }],
        ..valid_request.clone()
    };
    let result = store
        .bulk_apply_sheet_field_edits(&[valid_request, second_request], "mt-014-test")
        .await
        .expect("bulk apply sheet field edits");
    assert_eq!(result.receipt.operation, "bulk_apply_sheet_field_edits");
    assert_eq!(result.receipt.target_count, 2);
    assert_eq!(result.receipt.mutation_count, 2);
    assert_eq!(result.results.len(), 2);
    assert_one_bulk_receipt_event(&database, result.receipt.receipt_id).await;
    assert_eq!(
        store
            .sheet_version_history(first.internal_id)
            .await
            .expect("first history after successful bulk")
            .len(),
        2
    );
    assert_eq!(
        store
            .sheet_version_history(second.internal_id)
            .await
            .expect("second history after successful bulk")
            .len(),
        2
    );
}

#[tokio::test]
async fn bulk_sheet_field_edits_reject_stale_non_head_source_before_any_mutation() {
    let (store, database, _harness) = connected_store().await;

    let first = character(&store, "field-stale-first").await;
    let second = character(&store, "field-stale-second").await;
    let first_sheet = sheet(&store, first.internal_id, "field-stale-first").await;
    let second_sheet = sheet(&store, second.internal_id, "field-stale-second").await;
    let first_parse = store
        .parse_sheet_template_version(
            first_sheet.version_id,
            "mt-014-stale-template",
            Some("test://mt-014/field-stale-first"),
        )
        .await
        .expect("parse first stale bulk sheet");
    let second_parse = store
        .parse_sheet_template_version(
            second_sheet.version_id,
            "mt-014-stale-template",
            Some("test://mt-014/field-stale-second"),
        )
        .await
        .expect("parse second stale bulk sheet");

    let head_edit = store
        .apply_sheet_field_edits(&SheetFieldEditRequest {
            version_id: first_sheet.version_id,
            template_id: "mt-014-stale-template".to_string(),
            source_path: Some("test://mt-014/field-stale-first".to_string()),
            expected_template_hash: Some(first_parse.ast.template_hash.clone()),
            actor_role: "operator".to_string(),
            edits: vec![SheetFieldEdit {
                block_instance_id: None,
                field_id: "CHAR-ID-002".to_string(),
                replacement_text: "Current Head Name".to_string(),
            }],
            author: "mt-014-test".to_string(),
            tool: Some("bulk-stale-current-head-edit".to_string()),
        })
        .await
        .expect("append current head edit before stale bulk apply");

    let receipts_before_failure = bulk_receipt_count(&store, "bulk_apply_sheet_field_edits").await;
    let second_events_before_failure =
        aggregate_projection_count(&database, "atelier_sheet_version", second_sheet.version_id)
            .await;

    let stale_first_request = SheetFieldEditRequest {
        version_id: first_sheet.version_id,
        template_id: "mt-014-stale-template".to_string(),
        source_path: Some("test://mt-014/field-stale-first".to_string()),
        expected_template_hash: Some(first_parse.ast.template_hash.clone()),
        actor_role: "operator".to_string(),
        edits: vec![SheetFieldEdit {
            block_instance_id: None,
            field_id: "CHAR-ID-003".to_string(),
            replacement_text: "Stale Alias".to_string(),
        }],
        author: "mt-014-test".to_string(),
        tool: Some("bulk-stale-non-head-apply".to_string()),
    };
    let valid_second_request = SheetFieldEditRequest {
        version_id: second_sheet.version_id,
        template_id: "mt-014-stale-template".to_string(),
        source_path: Some("test://mt-014/field-stale-second".to_string()),
        expected_template_hash: Some(second_parse.ast.template_hash.clone()),
        actor_role: "operator".to_string(),
        edits: vec![SheetFieldEdit {
            block_instance_id: None,
            field_id: "CHAR-ID-003".to_string(),
            replacement_text: "Second Alias".to_string(),
        }],
        author: "mt-014-test".to_string(),
        tool: Some("bulk-valid-second-request".to_string()),
    };

    let failed = store
        .bulk_apply_sheet_field_edits(&[stale_first_request, valid_second_request], "mt-014-test")
        .await
        .expect_err("bulk apply must reject valid-hash non-head source versions");
    match failed {
        AtelierError::Validation(message) => assert!(
            message.contains("stale_selection") && message.contains("current head"),
            "stale bulk denial should name current-head protection: {message}"
        ),
        other => panic!("expected stale non-head validation denial, got {other:?}"),
    }

    let first_history = store
        .sheet_version_history(first.internal_id)
        .await
        .expect("first history after failed stale bulk");
    assert_eq!(
        first_history.len(),
        2,
        "stale bulk apply must not append a third first-character version"
    );
    assert_eq!(
        first_history
            .last()
            .expect("first history has head")
            .version_id,
        head_edit.version.version_id,
        "first character current head remains the preexisting head edit"
    );
    assert_eq!(
        store
            .sheet_version_history(second.internal_id)
            .await
            .expect("second history after failed stale bulk")
            .len(),
        1,
        "failed stale bulk apply must not append the valid second-character request"
    );
    assert_eq!(
        receipts_before_failure,
        bulk_receipt_count(&store, "bulk_apply_sheet_field_edits").await,
        "failed stale bulk apply must not write a receipt"
    );
    assert_eq!(
        second_events_before_failure,
        aggregate_projection_count(&database, "atelier_sheet_version", second_sheet.version_id).await,
        "failed stale bulk apply must not write EventLedger/projection rows for the valid sibling target"
    );
}
