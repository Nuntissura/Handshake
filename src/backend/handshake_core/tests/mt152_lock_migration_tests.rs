#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-152 I-152-2: the eight product-data process-global mutexes (Loom, Canvas,
//! wiki overlay / compile / markdown import, block views, bridge, Locus
//! dependencies) are gone; every store previously behind them runs through the
//! per-`SurrealDatabase` keyed-lock registry on the narrowest stable key with the
//! bounded MT-142 retry. Against the real embedded RocksDB store, per family:
//!
//! * disjoint records commit concurrently - `SurrealStorage::lease_high_water`
//!   (incremented inside `with_lease`, the engine window) reaches >= 2 for
//!   barrier-aligned writes to different Loom blocks, different Canvas boards
//!   and different wiki projections, and the keyed registry is idle afterwards;
//! * a same-record race has exactly one winner and typed losers: the Loom
//!   `expected_updated_at` compare-and-set, the Canvas stale-viewport
//!   compare-and-set, and a wiki overlay delete (one deletion receipt, every
//!   other racer the typed `NotFound`); same-title wiki compiles converge on one
//!   projection row.
//!
//! Every task is joined and every wait is bounded (tokio `Barrier` + `timeout`
//! only, no sleeps); each test clones the process template store from
//! `swarm_support` so no proof pays a cold schema apply.

#[path = "swarm_support/mod.rs"]
mod swarm_support;

use std::sync::Arc;
use std::time::Duration;

use handshake_core::storage::surreal::SurrealDatabase;
use handshake_core::storage::{
    Database, LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomBlockUpdate,
    LoomCanvasBoard, LoomWikiProjection, NewLoomBlock, StorageError, WriteContext,
    LOOM_CANVAS_BOARD_SCHEMA_ID,
};
use serde_json::json;
use swarm_support::*;
use tokio::sync::Barrier;
use tokio::time::timeout;

/// Disjoint records per overlap proof (contract: 16 workers).
const DISJOINT_RECORDS: usize = 16;
/// Racers per same-record proof (contract: N >= 8).
const RACERS: usize = 12;
const RACE_BOUND: Duration = Duration::from_secs(60);
/// Whole-test bound (same derivation as `surreal_swarm_semantics_tests`): a
/// stalled proof FAILS naming itself instead of hanging the shared cargo lane.
const TEST_BOUND: Duration = Duration::from_millis(900_000);
const STORE_LIFECYCLE_BOUND: Duration = Duration::from_millis(720_000);
const SETUP_BOUND: Duration = Duration::from_millis(300_000);

async fn run_bounded_test<F: std::future::Future<Output = ()>>(name: &str, body: F) {
    let _lane = serial_lane().await;
    timeout(TEST_BOUND, body).await.unwrap_or_else(|_| {
        panic!(
            "{name} exceeded its whole-test bound of {} ms (a proof that cannot finish is a failure, never an ignored test)",
            TEST_BOUND.as_millis()
        )
    });
}

fn ctx() -> WriteContext {
    WriteContext::human(None)
}

fn new_block(workspace_id: &str, content_type: LoomBlockContentType, title: &str) -> NewLoomBlock {
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
        derived: LoomBlockDerived::default(),
    }
}

async fn create_blocks(
    db: &SurrealDatabase,
    workspace_id: &str,
    content_type: LoomBlockContentType,
    prefix: &str,
    count: usize,
) -> Vec<LoomBlock> {
    let mut blocks = Vec::with_capacity(count);
    for index in 0..count {
        blocks.push(
            op_within(
                &format!("create {prefix} {index}"),
                SETUP_BOUND,
                db.create_loom_block(
                    &ctx(),
                    new_block(workspace_id, content_type.clone(), &format!("{prefix} {index}")),
                ),
            )
            .await
            .expect("create loom block"),
        );
    }
    blocks
}

fn board_state(pan_x: f64) -> serde_json::Value {
    json!({
        "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
        "pan_x": pan_x,
        "pan_y": 0.0,
        "zoom": 1.0,
    })
}

/// Runs one barrier-aligned operation per `inputs` entry on `db`, bounded per
/// operation, and returns every outcome in input order. The store's lease
/// high-water mark is reset right before the barrier releases so the reading
/// afterwards is this race's engine-window concurrency alone.
async fn barrier_race<I, T, F, Fut>(
    db: &SurrealDatabase,
    inputs: Vec<I>,
    label: &'static str,
    operation: F,
) -> Vec<Result<T, StorageError>>
where
    I: Send + 'static,
    T: Send + 'static,
    F: Fn(SurrealDatabase, I) -> Fut + Clone + Send + 'static,
    Fut: std::future::Future<Output = Result<T, StorageError>> + Send,
{
    let barrier = Arc::new(Barrier::new(inputs.len()));
    db.storage().reset_lease_high_water();
    let mut tasks = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.into_iter().enumerate() {
        let db = db.clone();
        let barrier = Arc::clone(&barrier);
        let operation = operation.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            timeout(PER_OPERATION_TIMEOUT, operation(db, input))
                .await
                .unwrap_or_else(|_| {
                    panic!(
                        "{label} {index}: exceeded the {} ms per-operation bound (hang)",
                        PER_OPERATION_TIMEOUT.as_millis()
                    )
                })
        }));
    }
    let mut outcomes = Vec::with_capacity(tasks.len());
    for task in tasks {
        outcomes.push(task.await.expect("racer task joined"));
    }
    outcomes
}

/// Every disjoint write committed, the engine window saw >= 2 of them at once,
/// and the keyed registry holds no entry afterwards.
fn assert_disjoint_overlap<T: std::fmt::Debug>(
    store: &SwarmStore,
    outcomes: &[Result<T, StorageError>],
    context: &str,
) {
    for (index, outcome) in outcomes.iter().enumerate() {
        if let Err(error) = outcome {
            panic!("{context}: disjoint write {index} must commit (no unrelated waiting or conflict), got: {error}");
        }
    }
    let lease_high_water = store.storage.lease_high_water();
    println!(
        "MT152_OVERLAP context={context} operations={} lease_high_water={lease_high_water}",
        outcomes.len()
    );
    assert!(
        lease_high_water >= 2,
        "{context}: disjoint-record writes must be observably concurrent inside the engine (lease high-water >= 2), got {lease_high_water}; this is the global-serialisation signature of the removed process-global mutex"
    );
    assert_eq!(
        store.db.lock_registry().entry_count(),
        0,
        "{context}: no disjoint-record write may leave a keyed-lock entry behind"
    );
}

/// Exactly one `Ok`; every other outcome satisfies `typed_loser`.
fn assert_one_winner<T: std::fmt::Debug>(
    outcomes: Vec<Result<T, StorageError>>,
    context: &str,
    typed_loser: impl Fn(&StorageError) -> bool,
) -> T {
    let mut winner = None;
    let mut losers = 0;
    for (index, outcome) in outcomes.into_iter().enumerate() {
        match outcome {
            Ok(value) => {
                assert!(
                    winner.is_none(),
                    "{context}: more than one racer won (racer {index} also succeeded: {value:?})"
                );
                winner = Some(value);
            }
            Err(error) => {
                assert!(
                    typed_loser(&error),
                    "{context}: racer {index} must lose with the typed outcome, got {error}"
                );
                losers += 1;
            }
        }
    }
    assert_eq!(losers, RACERS - 1, "{context}: every non-winner is a typed loser");
    winner.unwrap_or_else(|| panic!("{context}: no racer won"))
}

// ---------------------------------------------------------------------------
// Loom blocks
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn loom_disjoint_blocks_update_concurrently() {
    run_bounded_test("loom_disjoint_blocks_update_concurrently", async {
        let store = open_store_measured().await;
        let workspace_id = op_within("create workspace", SETUP_BOUND, store.create_workspace()).await;
        let blocks = create_blocks(
            &store.db,
            &workspace_id,
            LoomBlockContentType::Note,
            "loom-disjoint",
            DISJOINT_RECORDS,
        )
        .await;
        let inputs: Vec<(String, String)> = blocks
            .iter()
            .map(|block| (workspace_id.clone(), block.block_id.clone()))
            .collect();
        let outcomes = timeout(
            RACE_BOUND,
            barrier_race(&store.db, inputs, "loom disjoint update", |db, (ws, block_id)| async move {
                db.update_loom_block(
                    &ctx(),
                    &ws,
                    &block_id,
                    LoomBlockUpdate {
                        title: Some(format!("renamed {block_id}")),
                        ..LoomBlockUpdate::default()
                    },
                )
                .await
            }),
        )
        .await
        .expect("disjoint Loom updates finish inside their bound");
        assert_disjoint_overlap(&store, &outcomes, "16 disjoint Loom blocks, one workspace");
        for block in &blocks {
            let current = store
                .db
                .get_loom_block(&workspace_id, &block.block_id)
                .await
                .expect("re-read block");
            assert_eq!(current.title.as_deref(), Some(format!("renamed {}", block.block_id).as_str()));
        }
        op_within("close store", STORE_LIFECYCLE_BOUND, store.close_and_remove())
            .await
            .expect("close store");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn loom_same_block_expected_updated_at_race_has_one_winner() {
    run_bounded_test("loom_same_block_expected_updated_at_race_has_one_winner", async {
        let store = open_store_measured().await;
        let workspace_id = op_within("create workspace", SETUP_BOUND, store.create_workspace()).await;
        let block = create_blocks(&store.db, &workspace_id, LoomBlockContentType::Note, "loom-race", 1)
            .await
            .pop()
            .expect("one block");
        let inputs: Vec<usize> = (0..RACERS).collect();
        let (ws, block_id, expected) = (workspace_id.clone(), block.block_id.clone(), block.updated_at);
        let outcomes = timeout(
            RACE_BOUND,
            barrier_race(&store.db, inputs, "loom same-block update", move |db, racer| {
                let (ws, block_id) = (ws.clone(), block_id.clone());
                async move {
                    db.update_loom_block(
                        &ctx(),
                        &ws,
                        &block_id,
                        LoomBlockUpdate {
                            title: Some(format!("racer {racer}")),
                            expected_updated_at: Some(expected),
                            ..LoomBlockUpdate::default()
                        },
                    )
                    .await
                }
            }),
        )
        .await
        .expect("same-block race finishes inside its bound");
        let winner = assert_one_winner(outcomes, "Loom same-block expected_updated_at", |error| {
            matches!(error, StorageError::Conflict("loom_block_stale_updated_at"))
        });
        let current = store
            .db
            .get_loom_block(&workspace_id, &block.block_id)
            .await
            .expect("re-read block");
        assert_eq!(current.title, winner.title, "the acknowledged winner is the durable state");
        assert_eq!(store.db.lock_registry().entry_count(), 0, "registry idle after the race");
        op_within("close store", STORE_LIFECYCLE_BOUND, store.close_and_remove())
            .await
            .expect("close store");
    })
    .await;
}

// ---------------------------------------------------------------------------
// Canvas boards
// ---------------------------------------------------------------------------

async fn create_boards(
    store: &SwarmStore,
    workspace_id: &str,
    count: usize,
) -> Vec<LoomCanvasBoard> {
    let blocks = create_blocks(
        &store.db,
        workspace_id,
        LoomBlockContentType::Canvas,
        "canvas",
        count,
    )
    .await;
    let mut boards = Vec::with_capacity(count);
    for block in &blocks {
        boards.push(
            op_within(
                &format!("create board {}", block.block_id),
                SETUP_BOUND,
                store
                    .db
                    .create_canvas_board(&ctx(), workspace_id, &block.block_id, board_state(0.0)),
            )
            .await
            .expect("create canvas board"),
        );
    }
    boards
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn canvas_disjoint_boards_update_concurrently() {
    run_bounded_test("canvas_disjoint_boards_update_concurrently", async {
        let store = open_store_measured().await;
        let workspace_id = op_within("create workspace", SETUP_BOUND, store.create_workspace()).await;
        let boards = create_boards(&store, &workspace_id, DISJOINT_RECORDS).await;
        let inputs: Vec<(String, String, String)> = boards
            .iter()
            .map(|board| {
                (
                    workspace_id.clone(),
                    board.block_id.clone(),
                    board.event_ledger_event_id.clone(),
                )
            })
            .collect();
        let outcomes = timeout(
            RACE_BOUND,
            barrier_race(&store.db, inputs, "canvas disjoint viewport", |db, (ws, board_id, revision)| async move {
                db.update_canvas_board_state(&ctx(), &ws, &board_id, board_state(7.0), &revision)
                    .await
            }),
        )
        .await
        .expect("disjoint Canvas updates finish inside their bound");
        assert_disjoint_overlap(&store, &outcomes, "16 disjoint Canvas boards, one workspace");
        for board in &boards {
            let current = store
                .db
                .get_canvas_board(&workspace_id, &board.block_id)
                .await
                .expect("re-read board");
            assert_eq!(current.board.board_state["pan_x"], json!(7.0));
            assert_ne!(current.board.event_ledger_event_id, board.event_ledger_event_id);
        }
        op_within("close store", STORE_LIFECYCLE_BOUND, store.close_and_remove())
            .await
            .expect("close store");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn canvas_same_board_stale_viewport_race_has_one_winner() {
    run_bounded_test("canvas_same_board_stale_viewport_race_has_one_winner", async {
        let store = open_store_measured().await;
        let workspace_id = op_within("create workspace", SETUP_BOUND, store.create_workspace()).await;
        let board = create_boards(&store, &workspace_id, 1).await.pop().expect("one board");
        let inputs: Vec<usize> = (0..RACERS).collect();
        let (ws, board_id, revision) = (
            workspace_id.clone(),
            board.block_id.clone(),
            board.event_ledger_event_id.clone(),
        );
        let outcomes = timeout(
            RACE_BOUND,
            barrier_race(&store.db, inputs, "canvas same-board viewport", move |db, racer| {
                let (ws, board_id, revision) = (ws.clone(), board_id.clone(), revision.clone());
                async move {
                    db.update_canvas_board_state(
                        &ctx(),
                        &ws,
                        &board_id,
                        board_state(racer as f64 + 1.0),
                        &revision,
                    )
                    .await
                }
            }),
        )
        .await
        .expect("same-board race finishes inside its bound");
        let winner = assert_one_winner(outcomes, "Canvas same-board stale viewport", |error| {
            matches!(
                error,
                StorageError::Conflict("loom_canvas_board_stale_event_revision")
            )
        });
        let current = store
            .db
            .get_canvas_board(&workspace_id, &board.block_id)
            .await
            .expect("re-read board");
        assert_eq!(current.board.board_state, winner.board_state);
        assert_eq!(current.board.event_ledger_event_id, winner.event_ledger_event_id);
        assert_eq!(store.db.lock_registry().entry_count(), 0, "registry idle after the race");
        op_within("close store", STORE_LIFECYCLE_BOUND, store.close_and_remove())
            .await
            .expect("close store");
    })
    .await;
}

// ---------------------------------------------------------------------------
// Wiki projections
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn wiki_disjoint_projections_compile_concurrently() {
    run_bounded_test("wiki_disjoint_projections_compile_concurrently", async {
        let store = open_store_measured().await;
        let workspace_id = op_within("create workspace", SETUP_BOUND, store.create_workspace()).await;
        let sources = create_blocks(
            &store.db,
            &workspace_id,
            LoomBlockContentType::Note,
            "wiki-source",
            DISJOINT_RECORDS,
        )
        .await;
        let inputs: Vec<(String, String, Vec<String>)> = sources
            .iter()
            .enumerate()
            .map(|(index, block)| {
                (
                    workspace_id.clone(),
                    format!("Topic {index}"),
                    vec![block.block_id.clone()],
                )
            })
            .collect();
        let outcomes: Vec<Result<LoomWikiProjection, StorageError>> = timeout(
            RACE_BOUND,
            barrier_race(&store.db, inputs, "wiki disjoint compile", |db, (ws, title, block_ids)| async move {
                db.compile_loom_wiki_projection(&ws, &title, &block_ids).await
            }),
        )
        .await
        .expect("disjoint wiki compiles finish inside their bound");
        assert_disjoint_overlap(&store, &outcomes, "16 disjoint wiki projections, one workspace");
        let ids: std::collections::BTreeSet<String> = outcomes
            .iter()
            .filter_map(|outcome| outcome.as_ref().ok())
            .map(|projection| projection.projection_id.clone())
            .collect();
        assert_eq!(ids.len(), DISJOINT_RECORDS, "every compile produced its own projection");
        op_within("close store", STORE_LIFECYCLE_BOUND, store.close_and_remove())
            .await
            .expect("close store");
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn wiki_same_record_races_have_one_winner() {
    run_bounded_test("wiki_same_record_races_have_one_winner", async {
        let store = open_store_measured().await;
        let workspace_id = op_within("create workspace", SETUP_BOUND, store.create_workspace()).await;
        let source = create_blocks(&store.db, &workspace_id, LoomBlockContentType::Note, "wiki-race", 1)
            .await
            .pop()
            .expect("one source block");

        // Same title from every racer: the (workspace, title) natural key is the
        // idempotent identity (`uq_knowledge_wiki_projections_identity`), so every
        // compile converges on ONE projection row - no duplicate page, no raw error.
        let inputs: Vec<usize> = (0..RACERS).collect();
        let (ws, block_id) = (workspace_id.clone(), source.block_id.clone());
        let outcomes = timeout(
            RACE_BOUND,
            barrier_race(&store.db, inputs, "wiki same-title compile", move |db, _racer| {
                let (ws, block_id) = (ws.clone(), block_id.clone());
                async move {
                    db.compile_loom_wiki_projection(&ws, "Shared topic", &[block_id])
                        .await
                }
            }),
        )
        .await
        .expect("same-title compiles finish inside their bound");
        let mut projection_ids = std::collections::BTreeSet::new();
        for (index, outcome) in outcomes.iter().enumerate() {
            match outcome {
                Ok(projection) => {
                    projection_ids.insert(projection.projection_id.clone());
                }
                Err(error) => panic!(
                    "same-title compile {index} must converge on the stable identity, got: {error}"
                ),
            }
        }
        assert_eq!(
            projection_ids.len(),
            1,
            "same-title compiles must converge on exactly one projection row, got {projection_ids:?}"
        );
        let projection_id = projection_ids
            .into_iter()
            .next()
            .expect("one projection id");

        // Same overlay from every racer: exactly one deletion receipt; every other
        // racer is the typed `NotFound`, never a raw engine error.
        let overlay = op_within(
            "add overlay",
            SETUP_BOUND,
            store
                .db
                .add_loom_wiki_overlay(&workspace_id, &projection_id, "race me", None),
        )
        .await
        .expect("add overlay");
        let inputs: Vec<usize> = (0..RACERS).collect();
        let (ws, overlay_id) = (workspace_id.clone(), overlay.overlay_id.clone());
        let outcomes = timeout(
            RACE_BOUND,
            barrier_race(&store.db, inputs, "wiki same-overlay delete", move |db, _racer| {
                let (ws, overlay_id) = (ws.clone(), overlay_id.clone());
                async move { db.delete_loom_wiki_overlay(&ws, &overlay_id).await }
            }),
        )
        .await
        .expect("same-overlay deletes finish inside their bound");
        assert_one_winner(outcomes, "wiki same-overlay delete", |error| {
            matches!(error, StorageError::NotFound("loom_wiki_overlay"))
        });
        let remaining = store
            .db
            .list_loom_wiki_overlays(&workspace_id, &projection_id)
            .await
            .expect("list overlays");
        assert!(remaining.is_empty(), "the one winning delete removed the overlay");
        assert_eq!(store.db.lock_registry().entry_count(), 0, "registry idle after the races");
        op_within("close store", STORE_LIFECYCLE_BOUND, store.close_and_remove())
            .await
            .expect("close store");
    })
    .await;
}
