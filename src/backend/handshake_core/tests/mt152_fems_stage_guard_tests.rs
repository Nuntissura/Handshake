#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-152 I-152-4: FEMS_MUTATION_LOCK and STAGE_INSERT_LOCK are gone; every
//! invariant they held is database-side. These proofs race the two invariants
//! that were lock-only or lock-shaped, with two `SurrealDatabase` wrappers over
//! one engine (one keyed, one `LockMode::Disabled`) and a task-local barrier
//! that parks both racers right before they send their transactions, so the
//! engine alone decides:
//!
//!   1. MT-146 D-146-1: a workspace delete racing a FEMS proposal insert never
//!      leaves a proposal referencing the deleted workspace. Both transactions
//!      write the `fems_workspace_write_anchors` key, so they collide at commit;
//!      the loser's bounded retry converges (delete re-cascades, insert fails
//!      closed with NotFound).
//!   2. STAGE: two inserts of one (workspace, idempotency_key) end with one
//!      artifact; `uq_stage_capture_artifacts_idempotency` admits one, the
//!      other returns the winner as a replay; a different request hash under the
//!      same key is the typed conflict.

#[path = "swarm_support/mod.rs"]
mod swarm_support;

use std::future::Future;
use std::sync::Arc;

use chrono::Utc;
use handshake_core::ace::{
    FemsSourceRef, FemsSourceRefKind, MemoryItemProvenance, MemoryMutationOp, MemoryWriteOp,
    MemoryWritePolicy, MemoryWriteProposal, PartialMemoryItem,
};
use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::storage::fems_memory::{self, StoredMemoryProposal};
use handshake_core::storage::surreal::keyed_lock::race_test_support::with_pause_after_decision;
use handshake_core::storage::surreal::keyed_lock::{KeyedLockRegistry, LockMode};
use handshake_core::storage::surreal::{RowFilter, SurrealDatabase};
use handshake_core::storage::{
    Database, NewStageCaptureArtifact, StageArtifactStore, StorageError, WriteContext,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use swarm_support::{
    op_within, open_store_measured, SwarmStore, PER_OPERATION_TIMEOUT, PER_WORKER_TIMEOUT,
    WHOLE_TEST_TIMEOUT,
};
use tokio::sync::Barrier;
use uuid::Uuid;

const RACE_ITERATIONS: usize = 6;

// Bounds are the swarm profile's (`tests/swarm_support/mod.rs`): every store call runs under
// `PER_OPERATION_TIMEOUT` (5 s), every racer task under `PER_WORKER_TIMEOUT`, every test body
// under `WHOLE_TEST_TIMEOUT`, and the store is a clone of the process template opened under
// `STORE_OPEN_BOUND` by `open_store_measured` (a cold apply is minutes on this disk), so a
// stall fails naming its operation instead of hanging the lane.

/// Whole-test bound: the named test fails instead of hanging the lane.
async fn whole_test<F: Future<Output = ()>>(name: &str, body: F) {
    op_within(name, WHOLE_TEST_TIMEOUT, body).await;
}

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

/// A canonical review-gated proposal the way the intake route persists it.
fn stored_proposal(workspace_id: &str, label: &str) -> (StoredMemoryProposal, NewKernelEvent) {
    let proposal_id = Uuid::now_v7().to_string();
    let request_id = format!("mt152-{label}-{}", Uuid::now_v7());
    let created_at = Utc::now();
    let content_hash = hex::encode(Sha256::digest(label.as_bytes()));
    let document_id = format!("doc-{label}");
    let source_refs = vec![FemsSourceRef {
        kind: FemsSourceRefKind::DocBlock,
        id: document_id.clone(),
        hash: Some(content_hash.clone()),
        selector: Some("bytes:0-4".to_owned()),
        created_at: Some(created_at.to_rfc3339()),
        classification: Some("low".to_owned()),
    }];
    let canonical = MemoryWriteProposal {
        schema_version: "hsk.memory_write_proposal@0.1".to_owned(),
        proposal_id: proposal_id.clone(),
        created_at: created_at.to_rfc3339(),
        created_by_job_id: "mt152-task".to_owned(),
        scope_refs: Vec::new(),
        source_refs: source_refs.clone(),
        policy: MemoryWritePolicy {
            allow_procedural: false,
            require_human_review: true,
            max_ops: 1,
        },
        ops: vec![MemoryWriteOp {
            op: MemoryMutationOp::Add,
            temp_id: Some("m1".to_owned()),
            memory_id: None,
            item: PartialMemoryItem {
                memory_class: Some("semantic".to_owned()),
                item_type: Some("fact".to_owned()),
                scope_refs: Some(Vec::new()),
                content: Some(label.to_owned()),
                confidence: Some(1.0),
                trust_level: Some("user_asserted".to_owned()),
                provenance: Some(MemoryItemProvenance {
                    source_refs,
                    created_by_job_id: "mt152-task".to_owned(),
                }),
                classification: Some("low".to_owned()),
                ..PartialMemoryItem::default()
            },
            rationale: "Editor selection proposed from source_refs[0]".to_owned(),
            confidence: 1.0,
            requires_review: true,
        }],
    };
    let stored = StoredMemoryProposal {
        proposal_id: proposal_id.clone(),
        request_id: request_id.clone(),
        workspace_id: workspace_id.to_owned(),
        document_id: document_id.clone(),
        selection_start: 0,
        selection_end: 4,
        content_hash: content_hash.clone(),
        memory_class: "semantic".to_owned(),
        status: "pending_review".to_owned(),
        review_gated: true,
        created_at,
        proposal: json!({
            "_canonical_artifact": serde_json::to_value(&canonical).expect("canonical artifact"),
            "proposal_id": proposal_id,
            "request_id": request_id,
            "workspace_id": workspace_id,
            "class": "semantic",
            "content": label,
            "review_gated": true,
            "status": "pending_review",
            "actor_id": "mt152-operator",
        }),
    };
    let receipt = proposal_receipt(&stored);
    (stored, receipt)
}

/// The intake receipt for `stored`, derived from its durable fields only so a
/// replay builds the identical receipt.
fn proposal_receipt(stored: &StoredMemoryProposal) -> NewKernelEvent {
    NewKernelEvent::builder(
        "mt152-task",
        "mt152-session",
        KernelEventType::ArtifactProposed,
        KernelActor::Operator("mt152-operator".to_owned()),
    )
    .aggregate("fems_memory_proposal", stored.proposal_id.clone())
    .idempotency_key(format!("fems-memory-proposal:{}", stored.proposal_id))
    .correlation_id(format!("fems-memory-proposal:{}", stored.proposal_id))
    .source_component("fems_memory_proposal_intake")
    .payload(json!({
        "receipt_kind": "fems_memory_write_proposal",
        "proposal_id": stored.proposal_id,
        "workspace_id": stored.workspace_id,
        "document_id": stored.document_id,
        "selection_start": stored.selection_start,
        "selection_end": stored.selection_end,
        "content_hash": stored.content_hash,
        "memory_class": stored.memory_class,
        "review_gated": stored.review_gated,
        "status": stored.status,
        "never_editor_direct": true,
    }))
    .build()
    .expect("valid proposal receipt")
}

/// The FEMS tables a workspace delete must leave empty for that workspace, with
/// the field that references it (the anchor keys by the bare workspace id).
const WORKSPACE_SCOPED_FEMS_TABLES: [(&str, &str); 3] = [
    ("fems_memory_proposals", "workspace_id"),
    ("fems_memory_lifecycle_fr_outbox", "workspace_id"),
    ("fems_workspace_write_anchors", "workspace_key"),
];

/// Rows of `table` whose `field` references `workspace_id`.
async fn rows_referencing_workspace(
    store: &SwarmStore,
    table: &str,
    field: &str,
    workspace_id: &str,
) -> usize {
    let inspector = store.storage.test_inspector();
    let selector = inspector
        .table_selector(table)
        .await
        .expect("table selector");
    op_within(
        "inspector project",
        PER_OPERATION_TIMEOUT,
        inspector.project(
            &selector,
            &[selector.field(field).expect("workspace reference field")],
            RowFilter::All,
        ),
    )
    .await
    .expect("project workspace references")
        .into_iter()
        .filter(|row| row.values[field].to_string().contains(workspace_id))
        .count()
}

fn is_workspace_not_found(error: &StorageError) -> bool {
    matches!(error, StorageError::NotFound("workspace"))
}

/// D-146-1 under both wrapper assignments and both await orders: no proposal
/// (or its lifecycle outbox row) outlives the workspace, the delete always
/// converges, and the insert either committed first (and was cascaded) or
/// failed closed with the typed NotFound.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_delete_racing_proposal_insert_never_leaves_an_orphan_without_the_static_lock(
) {
    whole_test(
        "workspace_delete_racing_proposal_insert_never_leaves_an_orphan_without_the_static_lock",
        workspace_delete_racing_proposal_insert_body(open_store_measured().await),
    )
    .await;
}

async fn workspace_delete_racing_proposal_insert_body(store: SwarmStore) {
    let (keyed, disabled) = independent_wrappers(&store.db);
    let mut inserted_first = 0usize;
    let mut failed_closed = 0usize;

    for iteration in 0..RACE_ITERATIONS {
        let workspace_id = op_within(
            "create workspace",
            PER_OPERATION_TIMEOUT,
            store.create_workspace(),
        )
        .await;
        let (proposal, receipt) = stored_proposal(&workspace_id, &format!("race-{iteration}"));
        let barrier = Arc::new(Barrier::new(2));
        let delete_db = if iteration % 2 == 0 { disabled.clone() } else { keyed.clone() };
        let insert_db = if iteration % 2 == 0 { keyed.clone() } else { disabled.clone() };

        let delete_task = {
            let barrier = Arc::clone(&barrier);
            let workspace_id = workspace_id.clone();
            tokio::spawn(op_within("delete racer", PER_WORKER_TIMEOUT, async move {
                with_pause_after_decision(barrier, async move {
                    op_within(
                        "delete_workspace",
                        PER_OPERATION_TIMEOUT,
                        delete_db.delete_workspace(&WriteContext::human(None), &workspace_id),
                    )
                    .await
                })
                .await
            }))
        };
        let insert_task = {
            let barrier = Arc::clone(&barrier);
            tokio::spawn(op_within("insert racer", PER_WORKER_TIMEOUT, async move {
                with_pause_after_decision(barrier, async move {
                    op_within(
                        "insert_memory_proposal_with_receipt",
                        PER_OPERATION_TIMEOUT,
                        fems_memory::insert_memory_proposal_with_receipt(
                            &insert_db, &proposal, receipt,
                        ),
                    )
                    .await
                })
                .await
            }))
        };
        let (delete_outcome, insert_outcome) = if iteration % 2 == 0 {
            let d = delete_task.await.expect("delete task");
            let i = insert_task.await.expect("insert task");
            (d, i)
        } else {
            let i = insert_task.await.expect("insert task");
            let d = delete_task.await.expect("delete task");
            (d, i)
        };
        delete_outcome.expect("workspace delete must converge through its bounded retry");
        match insert_outcome {
            Ok(_) => inserted_first += 1,
            Err(error) if is_workspace_not_found(&error) => failed_closed += 1,
            Err(error) => panic!("insert must be Ok or the typed NotFound, got {error}"),
        }

        assert!(
            op_within(
                "get_workspace",
                PER_OPERATION_TIMEOUT,
                keyed.get_workspace(&workspace_id)
            )
            .await
            .expect("read workspace")
            .is_none(),
            "workspace must be gone"
        );
        for (table, field) in WORKSPACE_SCOPED_FEMS_TABLES {
            assert_eq!(
                rows_referencing_workspace(&store, table, field, &workspace_id).await,
                0,
                "iteration {iteration}: {table} must hold no row for the deleted workspace"
            );
        }
        assert!(
            op_within(
                "list_memory_proposals",
                PER_OPERATION_TIMEOUT,
                fems_memory::list_memory_proposals(store.db.storage(), &workspace_id, 10),
            )
            .await
            .expect("list proposals")
            .is_empty(),
            "iteration {iteration}: no proposal may outlive its workspace"
        );
    }
    eprintln!(
        "MT152_D146_1_RACE inserted_first={inserted_first} failed_closed={failed_closed} iterations={RACE_ITERATIONS}"
    );
    assert_eq!(inserted_first + failed_closed, RACE_ITERATIONS);
    op_within("close_and_remove", PER_OPERATION_TIMEOUT, store.close_and_remove())
        .await
        .expect("close and remove the guard-proof store");
}

/// A proposal inserted into a live workspace is visible, and a later delete of
/// that workspace cascades it (the non-racing baseline of the same guard).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn proposal_insert_then_workspace_delete_cascades_and_reclaims_the_anchor() {
    whole_test(
        "proposal_insert_then_workspace_delete_cascades_and_reclaims_the_anchor",
        proposal_insert_then_workspace_delete_body(open_store_measured().await),
    )
    .await;
}

async fn proposal_insert_then_workspace_delete_body(store: SwarmStore) {
    let (keyed, disabled) = independent_wrappers(&store.db);
    let workspace_id = op_within(
        "create workspace",
        PER_OPERATION_TIMEOUT,
        store.create_workspace(),
    )
    .await;
    let (proposal, receipt) = stored_proposal(&workspace_id, "baseline");
    let stored = op_within(
        "insert_memory_proposal_with_receipt",
        PER_OPERATION_TIMEOUT,
        fems_memory::insert_memory_proposal_with_receipt(&disabled, &proposal, receipt),
    )
    .await
    .expect("insert into a live workspace");
    assert_eq!(stored.proposal_id, proposal.proposal_id);
    assert_eq!(
        rows_referencing_workspace(
            &store,
            "fems_workspace_write_anchors",
            "workspace_key",
            &workspace_id
        )
        .await,
        1,
        "the insert must have written the workspace write anchor"
    );
    // An exact replay of the same request converges on the same row.
    let replayed = op_within(
        "insert replay",
        PER_OPERATION_TIMEOUT,
        fems_memory::insert_memory_proposal_with_receipt(
            &keyed,
            &proposal,
            proposal_receipt(&proposal),
        ),
    )
    .await
    .expect("exact replay converges");
    assert_eq!(replayed.proposal_id, proposal.proposal_id);

    op_within(
        "delete_workspace",
        PER_OPERATION_TIMEOUT,
        keyed.delete_workspace(&WriteContext::human(None), &workspace_id),
    )
    .await
    .expect("delete workspace");
    for (table, field) in WORKSPACE_SCOPED_FEMS_TABLES {
        assert_eq!(
            rows_referencing_workspace(&store, table, field, &workspace_id).await,
            0,
            "{table} must be cascaded / reclaimed with the workspace"
        );
    }
    let (late, late_receipt) = stored_proposal(&workspace_id, "after-delete");
    let after = op_within(
        "insert after delete",
        PER_OPERATION_TIMEOUT,
        fems_memory::insert_memory_proposal_with_receipt(&disabled, &late, late_receipt),
    )
    .await
    .expect_err("insert into a deleted workspace must fail closed");
    assert!(is_workspace_not_found(&after), "got {after}");
    op_within("close_and_remove", PER_OPERATION_TIMEOUT, store.close_and_remove())
        .await
        .expect("close and remove the guard-proof store");
}

fn stage_receipt(event_type: KernelEventType, idempotency_key: &str) -> NewKernelEvent {
    NewKernelEvent::builder(
        "mt152-stage-task",
        "mt152-stage-session",
        event_type,
        KernelActor::Operator("mt152-stage".to_owned()),
    )
    .aggregate("stage_capture_proof", "pending")
    .idempotency_key(idempotency_key)
    .correlation_id("mt152-stage-correlation")
    .source_component("mt152_fems_stage_guard_tests")
    .payload(json!({"proof": true}))
    .build()
    .expect("valid stage proof event")
}

fn stage_input(workspace_id: &str, idempotency_key: &str, request: &str) -> NewStageCaptureArtifact {
    NewStageCaptureArtifact {
        workspace_id: workspace_id.to_owned(),
        content_kind: "canvas_node".to_owned(),
        label: "Stage card".to_owned(),
        content_type: "text/markdown".to_owned(),
        content_json: json!({"text": request}),
        content_bytes: request.as_bytes().to_vec(),
        source_ref: None,
        idempotency_key: idempotency_key.to_owned(),
        request_hash: hex::encode(Sha256::digest(request.as_bytes())),
        actor_kind: "operator".to_owned(),
        actor_id: "mt152-stage".to_owned(),
        correlation_id: "mt152-stage-correlation".to_owned(),
        approval_id: "mt152-stage-approval".to_owned(),
        decision_receipt: stage_receipt(
            KernelEventType::ToolDecisionRecorded,
            &format!("{idempotency_key}-decision"),
        ),
        receipt: stage_receipt(
            KernelEventType::ArtifactStored,
            &format!("{idempotency_key}-stored"),
        ),
    }
}

fn artifact_count_value(value: &Value) -> String {
    value.to_string()
}

/// STAGE: the idempotency invariant is `uq_stage_capture_artifacts_idempotency`
/// (plus the ledger idempotency index for the receipts). Two racers with one
/// key, both past the replay preflight, end with exactly one artifact: one
/// stored it, the other replays it; a different request under the same key is
/// the typed conflict.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stage_inserts_with_one_idempotency_key_admit_one_artifact_without_the_static_lock() {
    whole_test(
        "stage_inserts_with_one_idempotency_key_admit_one_artifact_without_the_static_lock",
        stage_inserts_with_one_idempotency_key_body(open_store_measured().await),
    )
    .await;
}

async fn stage_inserts_with_one_idempotency_key_body(store: SwarmStore) {
    let (keyed, disabled) = independent_wrappers(&store.db);
    let workspace_id = op_within(
        "create workspace",
        PER_OPERATION_TIMEOUT,
        store.create_workspace(),
    )
    .await;

    for iteration in 0..RACE_ITERATIONS {
        let key = format!("mt152-stage-{iteration}");
        let request = format!("stage request {iteration}");
        let barrier = Arc::new(Barrier::new(2));
        let stores = if iteration % 2 == 0 {
            [
                StageArtifactStore::with_database(keyed.clone()),
                StageArtifactStore::with_database(disabled.clone()),
            ]
        } else {
            [
                StageArtifactStore::with_database(disabled.clone()),
                StageArtifactStore::with_database(keyed.clone()),
            ]
        };
        let racers = stores.map(|stage| {
            let barrier = Arc::clone(&barrier);
            let input = stage_input(&workspace_id, &key, &request);
            tokio::spawn(op_within("stage racer", PER_WORKER_TIMEOUT, async move {
                with_pause_after_decision(barrier, async move {
                    op_within(
                        "insert_stage_artifact",
                        PER_OPERATION_TIMEOUT,
                        stage.insert_stage_artifact(input),
                    )
                    .await
                })
                .await
            }))
        });
        let [left, right] = racers;
        let left = left.await.expect("stage racer").expect("stage insert converges");
        let right = right.await.expect("stage racer").expect("stage insert converges");
        assert_eq!(
            left.artifact.artifact_id, right.artifact.artifact_id,
            "iteration {iteration}: both racers must resolve to the one stored artifact"
        );
        assert_ne!(
            left.replayed, right.replayed,
            "iteration {iteration}: exactly one racer stored, the other replayed"
        );
        let stored = op_within(
            "get_stage_artifact",
            PER_OPERATION_TIMEOUT,
            StageArtifactStore::new(store.storage.clone())
                .get_stage_artifact(&workspace_id, &left.artifact.artifact_id),
        )
        .await
        .expect("read artifact")
        .expect("artifact exists");
        assert_eq!(stored.idempotency_key, key);

        // The same key with a different request is the typed conflict, from either wrapper.
        let conflict = op_within(
            "insert_stage_artifact (hash mismatch)",
            PER_OPERATION_TIMEOUT,
            StageArtifactStore::with_database(disabled.clone())
                .insert_stage_artifact(stage_input(&workspace_id, &key, "a different request")),
        )
        .await
        .expect_err("request-hash mismatch must be rejected");
        assert!(
            matches!(
                conflict,
                StorageError::Conflict(
                    "stage capture idempotency key was reused with a different request"
                )
            ),
            "got {conflict}"
        );
    }
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector("stage_capture_artifacts")
        .await
        .expect("artifact table selector");
    let keys: Vec<String> = op_within(
        "inspector project",
        PER_OPERATION_TIMEOUT,
        inspector.project(
            &table,
            &[table.field("idempotency_key").expect("idempotency_key field")],
            RowFilter::All,
        ),
    )
    .await
    .expect("project artifacts")
        .into_iter()
        .map(|row| artifact_count_value(&row.values["idempotency_key"]))
        .collect();
    assert_eq!(
        keys.len(),
        RACE_ITERATIONS,
        "one artifact per idempotency key: {keys:?}"
    );
    op_within("close_and_remove", PER_OPERATION_TIMEOUT, store.close_and_remove())
        .await
        .expect("close and remove the guard-proof store");
}
