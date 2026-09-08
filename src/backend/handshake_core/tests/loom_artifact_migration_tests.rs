//! MT-068: real embedded catalog and on-disk migration/retry boundaries.

#[path = "knowledge_ingestion_support.rs"]
mod embedded_knowledge_support;

use embedded_knowledge_support::EmbeddedKnowledgeStore;
use handshake_core::loom_fs::{loom_asset_blob_path, migrate_loom_asset, read_loom_asset_bytes};
use handshake_core::storage::artifacts::{artifact_root_dir, ArtifactLayer};
use handshake_core::storage::knowledge::{
    KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeSourceKind, KnowledgeStore,
    NewKnowledgeSource,
};
use handshake_core::storage::{
    Asset, Database, LoomArtifactBindingState, LoomBlockContentType, LoomBlockDerived, MediaTier,
    MediaTierStatus, MediaTierUpsert, NewAsset, NewLoomBlock, WriteContext,
};
use std::{fs, path::Path};

async fn legacy_asset(
    store: &EmbeddedKnowledgeStore,
    root: &Path,
    ws: &str,
    bytes: &[u8],
    classification: &str,
    proxy_of: Option<String>,
) -> Asset {
    let asset = store
        .db
        .create_asset(
            &WriteContext::human(None),
            NewAsset {
                workspace_id: ws.to_owned(),
                kind: if proxy_of.is_some() {
                    "proxy"
                } else {
                    "original"
                }
                .to_owned(),
                mime: "application/octet-stream".to_owned(),
                original_filename: Some("preserved.bin".to_owned()),
                content_hash: handshake_core::storage::artifacts::sha256_hex(bytes),
                size_bytes: bytes.len() as i64,
                width: Some(41),
                height: Some(23),
                classification: classification.to_owned(),
                exportable: false,
                is_proxy_of: proxy_of,
                proxy_asset_id: None,
            },
        )
        .await
        .expect("seed existing legacy catalog identity");
    let source = loom_asset_blob_path(root, ws, &asset.kind, &asset.content_hash);
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(source, bytes).unwrap();
    asset
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loom_artifact_migration_preserves_identity_and_recovers_real_boundaries() {
    let store = embedded_knowledge_support::open_embedded_store()
        .await
        .expect("mandatory embedded migration store");
    let root = tempfile::tempdir().unwrap();
    let ws = store.create_workspace().await;
    let ctx = WriteContext::human(None);
    let bytes = b"migration-original-identity";
    let original = legacy_asset(&store, root.path(), &ws, bytes, "low", None).await;
    let proxy = legacy_asset(
        &store,
        root.path(),
        &ws,
        b"proxy-bytes",
        "low",
        Some(original.asset_id.clone()),
    )
    .await;
    let block = store
        .db
        .create_loom_block(
            &ctx,
            NewLoomBlock {
                block_id: None,
                workspace_id: ws.clone(),
                content_type: LoomBlockContentType::File,
                document_id: None,
                asset_id: Some(original.asset_id.clone()),
                title: Some("Migration identity".to_owned()),
                original_filename: original.original_filename.clone(),
                content_hash: Some(original.content_hash.clone()),
                pinned: true,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived {
                    proxy_asset_id: Some(proxy.asset_id.clone()),
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    let collection = store
        .db
        .create_loom_collection(&ctx, &ws, Some("Migration references".to_owned()))
        .await
        .unwrap();
    let members = store
        .db
        .set_loom_collection_order(
            &ctx,
            &ws,
            &collection.collection_id,
            &[original.asset_id.clone(), proxy.asset_id.clone()],
        )
        .await
        .unwrap();
    store
        .db
        .upsert_media_tier(
            &ctx,
            MediaTierUpsert {
                workspace_id: ws.clone(),
                asset_id: original.asset_id.clone(),
                tier: MediaTier::Preview,
                status: MediaTierStatus::Ready,
                tier_asset_id: Some(proxy.asset_id.clone()),
                content_hash: Some(proxy.content_hash.clone()),
                failure_reason: None,
            },
        )
        .await
        .unwrap();
    let tiers_before = store
        .db
        .list_media_tiers(&ws, &original.asset_id)
        .await
        .unwrap();
    let knowledge = store
        .db
        .upsert_knowledge_source(NewKnowledgeSource {
            workspace_id: ws.clone(),
            root_id: None,
            source_kind: KnowledgeSourceKind::Asset,
            relative_path: None,
            asset_id: Some(original.asset_id.clone()),
            loom_block_id: None,
            document_id: None,
            content_hash: original.content_hash.clone(),
            size_bytes: Some(original.size_bytes),
            provenance: serde_json::json!({"fixture":"mt068-existing-asset-link"}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        })
        .await
        .expect("existing knowledge source linked to asset");

    // Reservation is durable even if the process stops before touching destination bytes.
    let reserved = store
        .db
        .reserve_loom_artifact_binding(&ctx, &original, None)
        .await
        .unwrap();
    assert_eq!(reserved.state, LoomArtifactBindingState::Reserved);
    let mut stale = original.clone();
    stale.mime = "image/jpeg".to_owned();
    assert!(
        store
            .db
            .reserve_loom_artifact_binding(&ctx, &stale, None)
            .await
            .is_err(),
        "expected-state mismatch must not change the canonical reservation"
    );
    assert!(
        !artifact_root_dir(root.path(), ArtifactLayer::L1, reserved.artifact_id)
            .join("payload")
            .exists()
    );
    let (first, retry) = tokio::join!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &original, None),
        migrate_loom_asset(&store.db, &ctx, root.path(), &original, None),
    );
    assert_eq!(first.expect("first concurrent migration"), bytes);
    assert_eq!(retry.expect("concurrent idempotent migration"), bytes);
    let ready = store
        .db
        .get_loom_artifact_binding(&ws, &original.asset_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ready.artifact_id, reserved.artifact_id);
    assert_eq!(ready.state, LoomArtifactBindingState::Ready);
    assert_eq!(
        store
            .db
            .get_knowledge_source(&knowledge.source_id)
            .await
            .unwrap()
            .unwrap(),
        knowledge,
        "knowledge source identity and scope remain bound to original asset"
    );
    let migration_events: Vec<_> = store
        .db
        .list_kernel_events_for_aggregate("loom_asset", &original.asset_id)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.payload["type"] == "loom_asset_artifact_binding")
        .collect();
    assert_eq!(
        migration_events.len(),
        2,
        "one reservation and one publication despite retries"
    );
    for state in ["reserved", "ready"] {
        let event = migration_events
            .iter()
            .find(|event| event.payload["state"] == state)
            .expect("durable migration transition event");
        assert_eq!(event.payload["artifact_id"], ready.artifact_id.to_string());
        assert_eq!(event.payload["asset_id"], original.asset_id);
        assert_eq!(event.payload["workspace_id"], ws);
    }
    assert_eq!(
        read_loom_asset_bytes(&store.db, &ctx, root.path(), &original)
            .await
            .unwrap(),
        bytes
    );
    assert_eq!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &proxy, None)
            .await
            .unwrap(),
        b"proxy-bytes"
    );
    for asset in [&original, &proxy] {
        assert_eq!(
            serde_json::to_value(store.db.get_asset(&ws, &asset.asset_id).await.unwrap()).unwrap(),
            serde_json::to_value(asset).unwrap(),
            "all catalog metadata and proxy links preserved"
        );
        assert!(
            !loom_asset_blob_path(root.path(), &ws, &asset.kind, &asset.content_hash).exists(),
            "verified migration retires the legacy input"
        );
    }
    assert_eq!(
        serde_json::to_value(store.db.get_loom_block(&ws, &block.block_id).await.unwrap()).unwrap(),
        serde_json::to_value(&block).unwrap(),
        "block and derived proxy references unchanged"
    );
    assert_eq!(
        serde_json::to_value(
            store
                .db
                .get_loom_collection(&ws, &collection.collection_id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(members).unwrap(),
        "collection identity/order unchanged"
    );
    assert_eq!(
        serde_json::to_value(
            store
                .db
                .list_media_tiers(&ws, &original.asset_id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(tiers_before).unwrap(),
        "tier links unchanged"
    );

    // Crash after payload publication but before manifest publication: retry verifies existing bytes.
    let partial_bytes = b"payload-only-interruption";
    let partial = legacy_asset(&store, root.path(), &ws, partial_bytes, "low", None).await;
    let reservation = store
        .db
        .reserve_loom_artifact_binding(&ctx, &partial, None)
        .await
        .unwrap();
    let destination = artifact_root_dir(root.path(), ArtifactLayer::L1, reservation.artifact_id);
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join("payload"), partial_bytes).unwrap();
    assert!(!destination.join("artifact.json").exists());
    assert_eq!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &partial, None)
            .await
            .unwrap(),
        partial_bytes
    );
    assert_eq!(
        store
            .db
            .get_loom_artifact_binding(&ws, &partial.asset_id)
            .await
            .unwrap()
            .unwrap()
            .artifact_id,
        reservation.artifact_id,
        "retry reuses reserved destination identity"
    );
    assert!(destination.join("artifact.json").is_file());

    // A complete verified destination can survive a crash before catalog publication.
    let unpublished_bytes = b"manifest-complete-before-binding";
    let unpublished = legacy_asset(&store, root.path(), &ws, unpublished_bytes, "low", None).await;
    let unpublished_reservation = store
        .db
        .reserve_loom_artifact_binding(&ctx, &unpublished, None)
        .await
        .unwrap();
    let mut unpublished_manifest = handshake_core::storage::artifacts::read_artifact_manifest(
        root.path(),
        ArtifactLayer::L1,
        reservation.artifact_id,
    )
    .unwrap();
    unpublished_manifest.artifact_id = unpublished_reservation.artifact_id;
    unpublished_manifest.content_hash = unpublished.content_hash.clone();
    unpublished_manifest.size_bytes = unpublished.size_bytes as u64;
    handshake_core::storage::artifacts::write_file_artifact(
        root.path(),
        &unpublished_manifest,
        unpublished_bytes,
    )
    .unwrap();
    assert_eq!(
        store
            .db
            .get_loom_artifact_binding(&ws, &unpublished.asset_id)
            .await
            .unwrap()
            .unwrap()
            .state,
        LoomArtifactBindingState::Reserved
    );
    assert_eq!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &unpublished, None)
            .await
            .unwrap(),
        unpublished_bytes
    );
    assert_eq!(
        store
            .db
            .get_loom_artifact_binding(&ws, &unpublished.asset_id)
            .await
            .unwrap()
            .unwrap()
            .artifact_id,
        unpublished_reservation.artifact_id
    );

    // Damaged source never publishes a binding; repairing the source permits retry.
    let damaged_bytes = b"legacy-corruption-retry";
    let damaged = legacy_asset(&store, root.path(), &ws, damaged_bytes, "low", None).await;
    let damaged_source =
        loom_asset_blob_path(root.path(), &ws, &damaged.kind, &damaged.content_hash);
    fs::write(&damaged_source, vec![0; damaged_bytes.len()]).unwrap();
    assert!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &damaged, None)
            .await
            .is_err()
    );
    assert_ne!(
        store
            .db
            .get_loom_artifact_binding(&ws, &damaged.asset_id)
            .await
            .unwrap()
            .map(|r| r.state),
        Some(LoomArtifactBindingState::Ready)
    );
    assert!(damaged_source.exists());
    fs::remove_file(&damaged_source).unwrap();
    assert!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &damaged, None)
            .await
            .is_err(),
        "missing legacy bytes cannot publish a ready binding"
    );
    fs::write(&damaged_source, damaged_bytes).unwrap();
    assert_eq!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &damaged, None)
            .await
            .unwrap(),
        damaged_bytes
    );

    // Existing destination corruption cannot be hidden by serving or recopied legacy bytes.
    // Reintroduce valid legacy bytes as an explicit fallback candidate after retirement.
    let fallback = loom_asset_blob_path(root.path(), &ws, &original.kind, &original.content_hash);
    fs::write(&fallback, bytes).unwrap();
    let canonical =
        artifact_root_dir(root.path(), ArtifactLayer::L1, ready.artifact_id).join("payload");
    let corrupted = vec![0; bytes.len()];
    fs::write(&canonical, &corrupted).unwrap();
    assert!(
        read_loom_asset_bytes(&store.db, &ctx, root.path(), &original)
            .await
            .is_err()
    );
    assert_eq!(
        fs::read(&canonical).unwrap(),
        corrupted,
        "corrupt destination retained for diagnosis"
    );
    assert_eq!(
        fs::read(loom_asset_blob_path(
            root.path(),
            &ws,
            &original.kind,
            &original.content_hash
        ))
        .unwrap(),
        bytes
    );

    let high = legacy_asset(
        &store,
        root.path(),
        &ws,
        b"high-requires-retention",
        "high",
        None,
    )
    .await;
    assert!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &high, None)
            .await
            .is_err(),
        "no invented high retention policy"
    );
    assert_eq!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &high, Some(30))
            .await
            .unwrap(),
        b"high-requires-retention"
    );
    let high_binding = store
        .db
        .get_loom_artifact_binding(&ws, &high.asset_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(high_binding.retention_ttl_days, Some(30));
    assert!(
        migrate_loom_asset(&store.db, &ctx, root.path(), &high, Some(31))
            .await
            .is_err(),
        "retry cannot silently change retention policy"
    );
    let high_manifest = handshake_core::storage::artifacts::read_artifact_manifest(
        root.path(),
        ArtifactLayer::L1,
        high_binding.artifact_id,
    )
    .unwrap();
    assert_eq!(
        high_manifest.classification,
        handshake_core::storage::artifacts::ArtifactClassification::High
    );
    assert!(!high_manifest.exportable);
    store.close_and_remove().await.unwrap();
}
