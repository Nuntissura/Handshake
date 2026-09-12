#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-151 I-151-4 (RESIDUAL-MT142-EARLY-LOCK-RELEASE-ON-STATEMENT-TIMEOUT): the
//! caller-side statement bound in `SurrealStorage::with_data_operation` drops a
//! process-global mutation mutex guard on timeout while the abandoned statement
//! may still be live in the engine. The invariants those mutexes alone used to
//! hold now have database-side guards; these proofs race each guarded path with
//! two `SurrealDatabase` wrappers over one engine - one keyed, one with
//! `LockMode::Disabled` (MT-152 moved the statics onto that registry, so the
//! disabled wrapper IS the bypass) - and pause both racers after their
//! Rust-side read-decide step so both commit against the same stale decision.
//! Exactly one wins; the loser is a typed `Conflict`.
//!
//!   1. journal get-or-create: `uq_loom_blocks_journal_key` (schema.surql) -
//!      one journal block per (workspace, date) on every write path.
//!   2. folder re-parent: per-workspace `storage_graph_anchors` version
//!      compare-and-set in `update_loom_folder` - no cycle from concurrent
//!      re-parents that each saw an acyclic tree.
//!   3. work-packet dependency add: global `storage_graph_anchors` version
//!      compare-and-set in `add_dependency` - no cycle from concurrent adds.

#[path = "knowledge_ingestion_support/mod.rs"]
mod embedded_knowledge_support;

use std::sync::Arc;

use embedded_knowledge_support::open_embedded_store;
use handshake_core::storage::surreal::keyed_lock::race_test_support::with_pause_after_decision;
use handshake_core::storage::surreal::keyed_lock::{KeyedLockRegistry, LockMode};
use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::{
    Database, LoomBlockContentType, LoomBlockDerived, LoomFolderSortMode, LoomFolderUpdate,
    NewLoomBlock, NewLoomFolder, StorageError, WriteContext,
};
use handshake_core::workflows::locus::types as locus;
use tokio::sync::Barrier;

/// Two wrappers over one engine: the store under test keeps its keyed registry,
/// the second runs with `LockMode::Disabled` (AC-142-8 independent-client model).
fn independent_wrappers(store_db: &SurrealDatabase) -> (SurrealDatabase, SurrealDatabase) {
    let keyed = store_db.clone();
    let disabled = SurrealDatabase::with_lock_registry(
        store_db.storage().clone(),
        KeyedLockRegistry::disabled(),
    );
    assert_eq!(keyed.lock_registry().mode(), LockMode::Keyed);
    assert_eq!(disabled.lock_registry().mode(), LockMode::Disabled);
    (keyed, disabled)
}

fn split_outcomes<T: std::fmt::Debug>(
    left: Result<T, StorageError>,
    right: Result<T, StorageError>,
    label: &str,
) -> (T, StorageError) {
    match (left, right) {
        (Ok(winner), Err(loser)) | (Err(loser), Ok(winner)) => (winner, loser),
        (Ok(a), Ok(b)) => panic!("{label}: both racers won: {a:?} / {b:?}"),
        (Err(a), Err(b)) => panic!("{label}: both racers lost: {a} / {b}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn journal_get_or_create_admits_one_block_per_date_without_the_static_lock() {
    let Some(store) = open_embedded_store().await else {
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (keyed, disabled) = independent_wrappers(&store.db);
    let date = "2026-09-11";
    let barrier = Arc::new(Barrier::new(2));

    let racers = [keyed.clone(), disabled.clone()].map(|db| {
        let workspace_id = workspace_id.clone();
        let barrier = Arc::clone(&barrier);
        tokio::spawn(async move {
            with_pause_after_decision(barrier, async move {
                db.get_or_create_daily_journal_block(
                    &WriteContext::human(None),
                    &workspace_id,
                    date,
                )
                .await
            })
            .await
        })
    });
    let [left, right] = racers;
    let (left, right) = (
        left.await.expect("journal racer task"),
        right.await.expect("journal racer task"),
    );
    // Both racers observed "no journal for this date" (the barrier sits after that
    // read), so both sent a CREATE with a fresh block id: only the index can decide.
    let (winner, loser) = split_outcomes(left, right, "journal get-or-create");
    assert!(
        matches!(loser, StorageError::Conflict("loom_journal_date_exists")),
        "loser must be the typed unique-index outcome, got {loser}"
    );
    assert_eq!(winner.journal_date.as_deref(), Some(date));

    // Idempotent afterwards, from either wrapper, and exactly one journal row exists.
    for db in [&keyed, &disabled] {
        let again = db
            .get_or_create_daily_journal_block(&WriteContext::human(None), &workspace_id, date)
            .await
            .expect("get-or-create after the race replays the winner");
        assert_eq!(again.block_id, winner.block_id);
    }
    // The same key is guarded on the plain create path, which never consulted the
    // journal lookup even under the lock.
    let plain_create = keyed
        .create_loom_block(
            &WriteContext::human(None),
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Journal,
                document_id: None,
                asset_id: None,
                title: Some("duplicate journal".to_owned()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: Some(date.to_owned()),
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await;
    assert!(
        matches!(
            plain_create,
            Err(StorageError::Conflict("loom_journal_date_exists"))
        ),
        "plain create of a second journal for the date must lose to the index"
    );
    // A non-journal block may carry the same journal_date (the key is NONE for it).
    keyed
        .create_loom_block(
            &WriteContext::human(None),
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.clone(),
                content_type: LoomBlockContentType::Note,
                document_id: None,
                asset_id: None,
                title: Some("note with a journal date".to_owned()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: Some(date.to_owned()),
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await
        .expect("non-journal blocks are not constrained by the journal key");
    store
        .close_and_remove()
        .await
        .expect("close and remove the race-proof store");
}

async fn create_folder(db: &SurrealDatabase, workspace_id: &str, name: &str) -> String {
    db.create_loom_folder(
        workspace_id,
        NewLoomFolder {
            folder_id: None,
            workspace_id: workspace_id.to_owned(),
            parent_folder_id: None,
            name: name.to_owned(),
            color: None,
            sort_mode: LoomFolderSortMode::UpdatedDesc,
            sort_order: None,
            project_ref: None,
        },
    )
    .await
    .expect("create folder")
    .folder_id
}

fn reparent(parent_id: &str) -> LoomFolderUpdate {
    LoomFolderUpdate {
        parent_folder_id: Some(Some(parent_id.to_owned())),
        ..LoomFolderUpdate::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn folder_reparents_that_would_form_a_cycle_admit_one_without_the_static_lock() {
    let Some(store) = open_embedded_store().await else {
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (keyed, disabled) = independent_wrappers(&store.db);
    let folder_a = create_folder(&keyed, &workspace_id, "A").await;
    let folder_b = create_folder(&keyed, &workspace_id, "B").await;
    let barrier = Arc::new(Barrier::new(2));

    // A -> under B on the keyed wrapper, B -> under A on the disabled wrapper. Each
    // racer's Rust-side cycle walk sees two roots (the barrier sits after the walk).
    let moves = [
        (keyed.clone(), folder_a.clone(), folder_b.clone()),
        (disabled.clone(), folder_b.clone(), folder_a.clone()),
    ];
    let racers = moves.map(|(db, folder_id, parent_id)| {
        let workspace_id = workspace_id.clone();
        let barrier = Arc::clone(&barrier);
        tokio::spawn(async move {
            with_pause_after_decision(barrier, async move {
                db.update_loom_folder(&workspace_id, &folder_id, reparent(&parent_id))
                    .await
            })
            .await
        })
    });
    let [left, right] = racers;
    let (left, right) = (
        left.await.expect("folder racer task"),
        right.await.expect("folder racer task"),
    );
    let (winner, loser) = split_outcomes(left, right, "folder re-parent");
    assert!(
        matches!(
            loser,
            StorageError::Conflict("loom_folder_tree_changed_concurrently")
        ),
        "loser must be the typed anchor outcome, got {loser}"
    );
    assert!(winner.parent_folder_id.is_some());

    // The persisted tree is acyclic: exactly one of the two folders has a parent.
    let folders = keyed
        .list_loom_folders(&workspace_id)
        .await
        .expect("list folders");
    let parented = folders
        .iter()
        .filter(|folder| folder.parent_folder_id.is_some())
        .count();
    assert_eq!(parented, 1, "exactly one re-parent may persist: {folders:?}");
    // The lock path still rejects the now-visible cycle with the validation error.
    let (loser_folder, loser_parent) = if winner.folder_id == folder_a {
        (folder_b.clone(), folder_a.clone())
    } else {
        (folder_a.clone(), folder_b.clone())
    };
    let sequential = keyed
        .update_loom_folder(&workspace_id, &loser_folder, reparent(&loser_parent))
        .await;
    assert!(
        matches!(sequential, Err(StorageError::Validation(_))),
        "sequential re-run must see the committed edge: {sequential:?}"
    );
    store
        .close_and_remove()
        .await
        .expect("close and remove the race-proof store");
}

fn work_packet(wp_id: &str) -> locus::LocusCreateWpParams {
    locus::LocusCreateWpParams {
        wp_id: wp_id.to_owned(),
        title: format!("MT-151 race proof {wp_id}"),
        description: "Dependency-graph anchor proof.".to_owned(),
        priority: 1,
        kind: locus::WorkPacketType::Test,
        phase: locus::WorkPacketPhase::Phase1,
        routing: locus::RoutingPolicy::GovStandard,
        task_packet_path: Some(format!(".GOV/task_packets/{wp_id}/packet.json")),
        assignee: None,
        labels: None,
        spec_session_id: None,
        reporter: "mt151-race-proof".to_owned(),
    }
}

fn dependency(id: &str, from: &str, to: &str) -> locus::LocusOperation {
    locus::LocusOperation::AddDependency(locus::LocusAddDependencyParams {
        dependency_id: id.to_owned(),
        from_wp_id: from.to_owned(),
        to_wp_id: to.to_owned(),
        kind: locus::DependencyType::Blocks,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dependency_adds_that_would_form_a_cycle_admit_one_without_the_static_lock() {
    let Some(store) = open_embedded_store().await else {
        return;
    };
    let (keyed, disabled) = independent_wrappers(&store.db);
    let suffix = uuid::Uuid::now_v7().simple().to_string();
    let wp_a = format!("WP-MT151-A-{suffix}");
    let wp_b = format!("WP-MT151-B-{suffix}");
    for wp_id in [&wp_a, &wp_b] {
        keyed
            .execute_locus_operation(locus::LocusOperation::CreateWp(work_packet(wp_id)))
            .await
            .expect("create work packet");
    }
    let barrier = Arc::new(Barrier::new(2));

    let adds = [
        (keyed.clone(), format!("DEP-MT151-AB-{suffix}"), wp_a.clone(), wp_b.clone()),
        (disabled.clone(), format!("DEP-MT151-BA-{suffix}"), wp_b.clone(), wp_a.clone()),
    ];
    let racers = adds.map(|(db, dependency_id, from, to)| {
        let barrier = Arc::clone(&barrier);
        tokio::spawn(async move {
            with_pause_after_decision(barrier, async move {
                db.execute_locus_operation(dependency(&dependency_id, &from, &to))
                    .await
            })
            .await
        })
    });
    let [left, right] = racers;
    let (left, right) = (
        left.await.expect("dependency racer task"),
        right.await.expect("dependency racer task"),
    );
    let (winner, loser) = split_outcomes(left, right, "dependency add");
    assert!(
        matches!(
            loser,
            StorageError::Conflict("dependency graph changed concurrently")
        ),
        "loser must be the typed anchor outcome, got {loser}"
    );
    let winner_id = winner["dependency_id"]
        .as_str()
        .expect("winner receipt carries dependency_id")
        .to_owned();

    // Sequential re-run of the loser sees the committed edge and is rejected as a cycle.
    let (loser_id, from, to) = if winner_id.contains("-AB-") {
        (format!("DEP-MT151-BA-{suffix}"), wp_b.clone(), wp_a.clone())
    } else {
        (format!("DEP-MT151-AB-{suffix}"), wp_a.clone(), wp_b.clone())
    };
    let sequential = keyed
        .execute_locus_operation(dependency(&loser_id, &from, &to))
        .await;
    assert!(
        matches!(sequential, Err(StorageError::Validation(_))),
        "sequential re-run must see the committed edge: {sequential:?}"
    );
    // The graph is acyclic: the loser's edge was never persisted, the winner's was.
    let loser_row = keyed
        .execute_locus_operation(locus::LocusOperation::RemoveDependency(
            locus::LocusRemoveDependencyParams {
                dependency_id: loser_id.clone(),
            },
        ))
        .await;
    assert!(
        matches!(loser_row, Err(StorageError::NotFound("dependency"))),
        "loser edge must not persist: {loser_row:?}"
    );
    let winner_row = keyed
        .execute_locus_operation(locus::LocusOperation::RemoveDependency(
            locus::LocusRemoveDependencyParams {
                dependency_id: winner_id.clone(),
            },
        ))
        .await
        .expect("winner edge persisted");
    assert_eq!(winner_row["deleted"], serde_json::Value::Bool(true));
    store
        .close_and_remove()
        .await
        .expect("close and remove the race-proof store");
}
