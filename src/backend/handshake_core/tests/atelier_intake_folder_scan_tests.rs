//! MT-026 folder-scan intake proof: a real inbox directory scan creates
//! pending intake items with a max-file bound, duplicate skip, and no source
//! folder mutation.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use handshake_core::atelier::intake::{
    InboxFolderScanRequest, IntakeBatchMode, IntakeLane, IntakeProfileMode, NewIntakeBatch,
    intake_event_family,
};
use handshake_core::atelier::{AtelierStore, event_family};
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

fn write_fixture(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap_or_else(|error| {
        panic!("write fixture {}: {error}", path.display());
    });
}

fn source_snapshot(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    let mut snapshot = BTreeMap::new();
    for entry in fs::read_dir(root).expect("read source folder for snapshot") {
        let entry = entry.expect("read source folder entry");
        let name = entry
            .file_name()
            .into_string()
            .expect("fixture file name is utf-8");
        let file_type = entry.file_type().expect("read fixture file type");
        if file_type.is_file() {
            snapshot.insert(
                name,
                Some(fs::read(entry.path()).expect("read fixture file")),
            );
        } else if file_type.is_dir() {
            snapshot.insert(name, None);
        }
    }
    snapshot
}

#[tokio::test]
async fn mt026_folder_scan_imports_pending_images_with_bound_duplicate_skip_and_no_source_mutation()
{
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP mt026_folder_scan_imports_pending_images_with_bound_duplicate_skip_and_no_source_mutation: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    assert!(
        event_family::ALL.contains(&intake_event_family::INTAKE_FOLDER_SCAN_COMPLETED),
        "folder scan events must be discoverable through the parent atelier event registry"
    );

    let temp = tempfile::tempdir().expect("create inbox fixture dir");
    let inbox = temp.path();
    write_fixture(&inbox.join("a.png"), b"mt026-a-image");
    write_fixture(&inbox.join("b.jpg"), b"mt026-b-image");
    write_fixture(&inbox.join("c.webp"), b"mt026-c-image-over-bound");
    write_fixture(&inbox.join("notes.txt"), b"not an image");
    fs::create_dir(inbox.join("nested")).expect("create nested fixture dir");
    write_fixture(
        &inbox.join("nested").join("nested.png"),
        b"nested image must not be scanned",
    );

    let before = source_snapshot(inbox);
    let idempotency_key = format!("mt-026-inbox-scan-{}", Uuid::new_v4());
    let request = InboxFolderScanRequest {
        idempotency_key: idempotency_key.clone(),
        inbox_root: inbox.to_path_buf(),
        source_label: "operator-inbox-scan".to_string(),
        character_internal_id: None,
        max_files: 2,
        requested_by: "mt-026-test".to_string(),
    };

    let first = store
        .scan_inbox_folder_import(&request)
        .await
        .expect("scan inbox folder");
    assert_eq!(first.imported_count, 2, "max_files bounds imported images");
    assert_eq!(first.duplicate_skipped_count, 0);
    assert_eq!(first.image_candidate_count, 3);
    assert_eq!(first.skipped_over_max_count, 1);
    assert_eq!(first.skipped_non_image_count, 1);
    assert_eq!(first.skipped_subdir_count, 1);
    assert_eq!(first.items.len(), 2);
    assert!(
        first
            .items
            .iter()
            .all(|item| item.lane == IntakeLane::Pending)
    );
    assert_eq!(
        first
            .items
            .iter()
            .map(|item| item.file_name.as_str())
            .collect::<Vec<_>>(),
        vec!["a.png", "b.jpg"],
        "folder scan order is deterministic and max-file bounded"
    );
    assert!(
        first
            .items
            .iter()
            .all(|item| item.source_path.starts_with("source://operator-inbox/")),
        "persisted source refs must be portable inbox refs, not raw local paths"
    );
    assert_eq!(
        source_snapshot(inbox),
        before,
        "folder scan must not modify, move, delete, or create files in the source folder"
    );

    let second = store
        .scan_inbox_folder_import(&request)
        .await
        .expect("rescan inbox folder");
    assert_eq!(second.batch.batch_id, first.batch.batch_id);
    assert_eq!(
        second.imported_count, 0,
        "rescan must not create duplicates"
    );
    assert_eq!(second.duplicate_skipped_count, 2);
    assert_eq!(second.items.len(), 2);

    let persisted = store
        .list_intake_items(first.batch.batch_id, None)
        .await
        .expect("list scan intake items");
    assert_eq!(
        persisted.len(),
        2,
        "rescanning the same folder and idempotency key must not duplicate pending rows"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_FOLDER_SCAN_COMPLETED,
                "atelier_intake_batch",
                &first.batch.batch_id.to_string(),
            )
            .await
            .expect("count folder scan events"),
        2,
        "each scan action writes EventLedger summary evidence"
    );
    assert_eq!(
        source_snapshot(inbox),
        before,
        "rescan must also leave the source folder untouched"
    );
}

#[tokio::test]
async fn mt026_folder_scan_rejects_missing_directory_without_creating_batch() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP mt026_folder_scan_rejects_missing_directory_without_creating_batch: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let temp = tempfile::tempdir().expect("create temp dir");
    let missing = temp.path().join("missing");
    let idempotency_key = format!("mt-026-missing-inbox-{}", Uuid::new_v4());

    let error = store
        .scan_inbox_folder_import(&InboxFolderScanRequest {
            idempotency_key: idempotency_key.clone(),
            inbox_root: missing,
            source_label: "missing-inbox".to_string(),
            character_internal_id: None,
            max_files: 10,
            requested_by: "mt-026-test".to_string(),
        })
        .await
        .expect_err("missing inbox folder must be rejected");
    assert!(
        error.to_string().contains("inbox_root"),
        "missing directory error should name inbox_root, got {error}"
    );
    assert!(
        store
            .get_intake_batch_by_key(&idempotency_key)
            .await
            .expect("query missing-dir batch")
            .is_none(),
        "missing folder scan must not create a batch"
    );
}

#[tokio::test]
async fn mt026_folder_scan_rejects_same_key_different_root_replay() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP mt026_folder_scan_rejects_same_key_different_root_replay: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let first_dir = tempfile::tempdir().expect("create first inbox fixture dir");
    let second_dir = tempfile::tempdir().expect("create second inbox fixture dir");
    write_fixture(&first_dir.path().join("a.png"), b"first-root-image");
    write_fixture(&second_dir.path().join("a.png"), b"second-root-image");
    let idempotency_key = format!("mt-026-root-replay-{}", Uuid::new_v4());

    store
        .scan_inbox_folder_import(&InboxFolderScanRequest {
            idempotency_key: idempotency_key.clone(),
            inbox_root: first_dir.path().to_path_buf(),
            source_label: "root-replay".to_string(),
            character_internal_id: None,
            max_files: 10,
            requested_by: "mt-026-test".to_string(),
        })
        .await
        .expect("initial scan succeeds");

    let error = store
        .scan_inbox_folder_import(&InboxFolderScanRequest {
            idempotency_key,
            inbox_root: second_dir.path().to_path_buf(),
            source_label: "root-replay".to_string(),
            character_internal_id: None,
            max_files: 10,
            requested_by: "mt-026-test".to_string(),
        })
        .await
        .expect_err("same idempotency key with a different root must be rejected");
    assert!(
        error.to_string().contains("inbox_root"),
        "root mismatch error should name inbox_root, got {error}"
    );
}

#[tokio::test]
async fn mt026_folder_scan_rejects_preexisting_same_key_incompatible_batch_before_writes() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP mt026_folder_scan_rejects_preexisting_same_key_incompatible_batch_before_writes: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let temp = tempfile::tempdir().expect("create inbox fixture dir");
    write_fixture(&temp.path().join("a.png"), b"preexisting-mismatch-image");
    let idempotency_key = format!("mt-026-preexisting-mismatch-{}", Uuid::new_v4());
    let preexisting = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: idempotency_key.clone(),
            source_label: "preexisting manual batch".to_string(),
            source_ref: Some(format!("source://manual/{}", Uuid::new_v4())),
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("precreate incompatible batch with same idempotency key");

    let error = store
        .scan_inbox_folder_import(&InboxFolderScanRequest {
            idempotency_key,
            inbox_root: temp.path().to_path_buf(),
            source_label: "folder scan over incompatible batch".to_string(),
            character_internal_id: None,
            max_files: 10,
            requested_by: "mt-026-test".to_string(),
        })
        .await
        .expect_err("folder scan must reject incompatible preexisting batch");
    assert!(
        error.to_string().contains("idempotency_key"),
        "mismatch error should name idempotency_key, got {error}"
    );

    let items = store
        .list_intake_items(preexisting.batch_id, None)
        .await
        .expect("list incompatible preexisting batch items");
    assert!(
        items.is_empty(),
        "rejected folder scan must not add items to an incompatible preexisting batch"
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                intake_event_family::INTAKE_FOLDER_SCAN_COMPLETED,
                "atelier_intake_batch",
                &preexisting.batch_id.to_string(),
            )
            .await
            .expect("count rejected folder-scan events"),
        0,
        "rejected folder scan must not write scan-completed evidence"
    );
    let item_added_events: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_intake_item'
             AND payload->>'batch_id' = $2"#,
    )
    .bind(intake_event_family::INTAKE_ITEM_ADDED)
    .bind(preexisting.batch_id.to_string())
    .fetch_one(store.pool())
    .await
    .expect("count rejected item-added events");
    assert_eq!(
        item_added_events, 0,
        "rejected folder scan must not write item-added evidence"
    );
}

#[tokio::test]
async fn mt026_folder_scan_rejects_over_cap_max_files_before_creating_batch() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP mt026_folder_scan_rejects_over_cap_max_files_before_creating_batch: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let temp = tempfile::tempdir().expect("create inbox fixture dir");
    write_fixture(&temp.path().join("a.png"), b"over-cap-image");
    let idempotency_key = format!("mt-026-over-cap-{}", Uuid::new_v4());

    let error = store
        .scan_inbox_folder_import(&InboxFolderScanRequest {
            idempotency_key: idempotency_key.clone(),
            inbox_root: temp.path().to_path_buf(),
            source_label: "over-cap".to_string(),
            character_internal_id: None,
            max_files: usize::MAX,
            requested_by: "mt-026-test".to_string(),
        })
        .await
        .expect_err("over-cap max_files must be rejected");
    assert!(
        error.to_string().contains("max_files"),
        "max_files cap error should name max_files, got {error}"
    );
    assert!(
        store
            .get_intake_batch_by_key(&idempotency_key)
            .await
            .expect("query over-cap batch")
            .is_none(),
        "rejected over-cap scan must not create a batch"
    );
}

#[tokio::test]
async fn mt026_concurrent_same_folder_scans_count_only_true_inserts() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP mt026_concurrent_same_folder_scans_count_only_true_inserts: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;
    let temp = tempfile::tempdir().expect("create inbox fixture dir");
    write_fixture(&temp.path().join("a.png"), b"concurrent-a");
    write_fixture(&temp.path().join("b.png"), b"concurrent-b");
    let idempotency_key = format!("mt-026-concurrent-{}", Uuid::new_v4());
    let request = InboxFolderScanRequest {
        idempotency_key,
        inbox_root: temp.path().to_path_buf(),
        source_label: "concurrent-scan".to_string(),
        character_internal_id: None,
        max_files: 10,
        requested_by: "mt-026-test".to_string(),
    };

    let first_store = store.clone();
    let second_store = store.clone();
    let first_request = request.clone();
    let second_request = request.clone();
    let (first, second) = tokio::join!(
        async move {
            first_store
                .scan_inbox_folder_import(&first_request)
                .await
                .expect("first concurrent scan")
        },
        async move {
            second_store
                .scan_inbox_folder_import(&second_request)
                .await
                .expect("second concurrent scan")
        }
    );

    assert_eq!(first.batch.batch_id, second.batch.batch_id);
    assert_eq!(
        first.imported_count + second.imported_count,
        2,
        "only true inserts count as imported across concurrent scans"
    );
    assert_eq!(
        first.duplicate_skipped_count + second.duplicate_skipped_count,
        2,
        "second observation of each bounded file is a duplicate skip"
    );
    let persisted = store
        .list_intake_items(first.batch.batch_id, None)
        .await
        .expect("list concurrent scan items");
    assert_eq!(persisted.len(), 2);
    for item in persisted {
        assert_eq!(
            store
                .count_events_for_aggregate(
                    intake_event_family::INTAKE_ITEM_ADDED,
                    "atelier_intake_item",
                    &item.item_id.to_string(),
                )
                .await
                .expect("count item-added event"),
            1,
            "each true inserted item gets exactly one item_added event"
        );
    }
}
