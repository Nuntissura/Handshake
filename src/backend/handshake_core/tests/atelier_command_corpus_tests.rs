//! WP-KERNEL-005 atelier command-corpus parity: real PostgreSQL round-trip
//! proofs for the command-corpus / action-catalog parity submodule (MT-206).
//! Run with a live DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_command_corpus_tests -- --nocapture
//!
//! No mocks: each test connects the actual `AtelierStore` to a real Postgres,
//! ensures the schema, exercises the command-corpus module with REAL data, and
//! asserts the load-bearing invariants from Section 10.19 (parity contract).
//! Tables persist between runs, so all action ids / manual ids are made unique
//! per run via `Uuid::new_v4()` to avoid cross-run UNIQUE collisions. Only
//! `handshake_core` + `tokio` + `uuid` + `serde_json` (+ std) are used; sqlx is
//! never imported directly. This module has no character FK.

mod atelier_pg_support;

use handshake_core::atelier::command_corpus::{
    builtin_command_corpus, command_corpus_event_family, BlockedReason, CorpusErrorVariant,
    CorpusSource, ExecutionClass, RecordCommandCorpusInvocation, UpsertCommandCorpusEntry,
    MANUAL_ANCHOR_BLOCKED,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

/// A run-unique, valid (non-blocked, non-governed) catalog descriptor input.
/// Uses `PureProjection` so no `evidence_class` is required by the upsert guard.
fn unique_entry_input(manual_anchor: &str) -> UpsertCommandCorpusEntry {
    let action_id = format!("test.command-{}", Uuid::new_v4());
    UpsertCommandCorpusEntry {
        action_id,
        corpus_source: CorpusSource::IpcHandler,
        owner: "atelier.command_corpus".to_string(),
        actor_eligibility: vec!["operator".to_string(), "model".to_string()],
        params_schema_ref: format!("schema://params/{}", Uuid::new_v4()),
        input_schema_version: 1,
        capabilities: vec!["corpus.read".to_string()],
        execution_class: ExecutionClass::PureProjection,
        receipt_shape: format!("schema://receipt/{}", Uuid::new_v4()),
        errors: vec![CorpusErrorVariant {
            code: "denied".to_string(),
            recovery_instruction: "request the corpus.read capability".to_string(),
        }],
        foreground_flag: false,
        manual_anchor: manual_anchor.to_string(),
        evidence_class: vec!["atelier.command_corpus.entry_upserted".to_string()],
    }
}

fn assert_blocked_invocation_denial(err: AtelierError, action_id: &str, detail: &str) {
    match err {
        AtelierError::Validation(message) => {
            assert!(
                message.contains("command_blocked"),
                "blocked invocation denial must carry the command_blocked code: {message}"
            );
            assert!(
                message.contains(action_id),
                "blocked invocation denial must name the action_id: {message}"
            );
            assert!(
                message.contains(detail),
                "blocked invocation denial must include {detail}: {message}"
            );
        }
        other => panic!("expected blocked invocation validation denial, got {other:?}"),
    }
}

#[tokio::test]
async fn corpus_entry_upsert_roundtrip_idempotency_and_event() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP corpus_entry_upsert_roundtrip_idempotency_and_event: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    // --- register a fresh descriptor with a live manual anchor ---
    let manual_anchor = format!("manual.cmd-{}", Uuid::new_v4());
    let input = unique_entry_input(&manual_anchor);
    let action_id = input.action_id.clone();

    let entry = store
        .upsert_command_corpus_entry(&input)
        .await
        .expect("upsert command corpus entry");

    // (3) round-trip: every typed field comes back exactly as written.
    assert_eq!(entry.action_id, action_id, "action_id round-trips");
    assert_eq!(entry.corpus_source, CorpusSource::IpcHandler);
    assert_eq!(entry.owner, "atelier.command_corpus");
    assert_eq!(
        entry.actor_eligibility,
        vec!["operator".to_string(), "model".to_string()],
        "actor_eligibility list round-trips in order"
    );
    assert_eq!(entry.execution_class, ExecutionClass::PureProjection);
    assert_eq!(
        entry.manual_anchor, manual_anchor,
        "live manual anchor stored"
    );
    assert_eq!(entry.input_schema_version, 1);
    assert_eq!(entry.errors.len(), 1, "typed error variant round-trips");
    assert_eq!(entry.errors[0].code, "denied");
    assert!(!entry.foreground_flag, "foreground flag round-trips false");

    // fetch by action_id returns the same descriptor.
    let fetched = store
        .get_command_corpus_entry(&action_id)
        .await
        .expect("get command corpus entry")
        .expect("entry present");
    assert_eq!(
        fetched.entry_id, entry.entry_id,
        "fetch returns the same row"
    );

    // (4) IDEMPOTENCY: re-projecting the same action_id updates in place; the
    // entry_id is stable (ON CONFLICT (action_id) DO UPDATE), no duplicate row.
    let mut input2 = unique_entry_input(&manual_anchor);
    input2.action_id = action_id.clone();
    input2.owner = "atelier.command_corpus.v2".to_string();
    let entry2 = store
        .upsert_command_corpus_entry(&input2)
        .await
        .expect("re-upsert same action_id");
    assert_eq!(
        entry.entry_id, entry2.entry_id,
        "re-upserting the same action_id keeps a stable entry_id (no duplicate)"
    );
    assert_eq!(
        entry2.owner, "atelier.command_corpus.v2",
        "owner updated in place"
    );

    // Exactly one row for this owner namespace's action id (no duplicate).
    let listed = store
        .list_command_corpus_entries(Some("atelier.command_corpus.v2"))
        .await
        .expect("list entries by owner");
    let matches = listed.iter().filter(|e| e.action_id == action_id).count();
    assert_eq!(matches, 1, "action_id appears exactly once after re-upsert");

    // (6) event emission: two upserts emitted two CORPUS_ENTRY_UPSERTED events.
    let event_count = store
        .count_events_for_aggregate(
            command_corpus_event_family::CORPUS_ENTRY_UPSERTED,
            "atelier_command_corpus_entry",
            &action_id,
        )
        .await
        .expect("count run-scoped upsert events");
    assert_eq!(
        event_count, 2,
        "each upsert emits exactly one CORPUS_ENTRY_UPSERTED event"
    );
}

#[tokio::test]
async fn corpus_external_execution_requires_evidence_invariant() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP corpus_external_execution_requires_evidence_invariant: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    // (5) DOMAIN INVARIANT (LAW-CORPUS-PARITY-004 / EVIDENCE-001): an external
    // governed-execution descriptor (workflow_job / ai_job) with no
    // evidence_class MUST be rejected, not silently stored.
    let mut bad = unique_entry_input("manual.cmd-evidence");
    bad.execution_class = ExecutionClass::AiJob;
    bad.evidence_class = Vec::new();
    let err = store.upsert_command_corpus_entry(&bad).await;
    assert!(
        err.is_err(),
        "ai_job with no evidence_class must be rejected as a validation error"
    );

    // The same descriptor WITH an evidence_class is accepted and round-trips.
    let mut good = bad.clone();
    good.evidence_class = vec!["atelier.workflow.job_completed".to_string()];
    let entry = store
        .upsert_command_corpus_entry(&good)
        .await
        .expect("ai_job with evidence_class is accepted");
    assert_eq!(entry.execution_class, ExecutionClass::AiJob);
    assert_eq!(
        entry.evidence_class,
        vec!["atelier.workflow.job_completed".to_string()],
        "evidence_class round-trips for governed execution"
    );
    assert!(
        entry.execution_class.requires_governed_execution(),
        "ai_job is classified as governed execution"
    );
}

#[tokio::test]
async fn corpus_blocked_record_upsert_idempotency_clear_on_anchor_and_events() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP corpus_blocked_record_upsert_idempotency_clear_on_anchor_and_events: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    // Project a descriptor first so we can prove the anchor flips its
    // manual_anchor between the BLOCKED sentinel and a live id.
    let input = unique_entry_input(MANUAL_ANCHOR_BLOCKED);
    let action_id = input.action_id.clone();
    store
        .upsert_command_corpus_entry(&input)
        .await
        .expect("upsert entry that will be blocked");

    // --- record a BLOCKED no-manual-anchor record ---
    let record = store
        .record_blocked_command(
            &action_id,
            BlockedReason::NoManualAnchor,
            CorpusSource::IpcHandler,
            "bind a live ModelManual command id via anchor_command_manual",
        )
        .await
        .expect("record blocked command");
    assert_eq!(
        record.action_id, action_id,
        "blocked record round-trips action_id"
    );
    assert_eq!(record.blocked_reason, BlockedReason::NoManualAnchor);
    assert_eq!(record.discovered_in, CorpusSource::IpcHandler);

    // Recording NoManualAnchor pins the descriptor's manual_anchor to BLOCKED.
    let blocked_entry = store
        .get_command_corpus_entry(&action_id)
        .await
        .expect("get blocked entry")
        .expect("entry present");
    assert_eq!(
        blocked_entry.manual_anchor, MANUAL_ANCHOR_BLOCKED,
        "blocking on no_manual_anchor pins the descriptor to the BLOCKED sentinel"
    );

    // (4) IDEMPOTENCY: re-recording the same (action_id, reason) refreshes the
    // existing record (stable blocked_id), keeps first_seen_utc, never dups.
    let record_again = store
        .record_blocked_command(
            &action_id,
            BlockedReason::NoManualAnchor,
            CorpusSource::Both,
            "still needs a manual anchor",
        )
        .await
        .expect("re-record same (action_id, reason)");
    assert_eq!(
        record.blocked_id, record_again.blocked_id,
        "re-recording the same (action_id, reason) keeps a stable blocked_id"
    );
    assert_eq!(
        record.first_seen_utc, record_again.first_seen_utc,
        "first_seen_utc is preserved across refresh (BLOCKED-002)"
    );
    assert!(
        record_again.last_seen_utc >= record.first_seen_utc,
        "last_seen_utc is monotonic and not before first_seen_utc"
    );

    // (5) DOMAIN INVARIANT: the BLOCKED record is preserved and listable until
    // its anchor is supplied (BLOCKED-003) -- exactly one record for this id.
    let open = store
        .list_blocked_commands(Some(&action_id))
        .await
        .expect("list blocked for action_id");
    assert_eq!(
        open.len(),
        1,
        "exactly one open BLOCKED record (no duplicate from re-record)"
    );

    // --- supplying a live anchor clears the no_manual_anchor block ---
    let live_manual = format!("manual.cmd-{}", Uuid::new_v4());
    let anchored = store
        .anchor_command_manual(&action_id, &live_manual)
        .await
        .expect("anchor command to live manual id");
    assert_eq!(
        anchored.manual_anchor, live_manual,
        "anchoring points the descriptor at the live ModelManual id"
    );

    // (5) INVARIANT: clear-on-anchor -- the BLOCKED record is gone.
    let open_after = store
        .list_blocked_commands(Some(&action_id))
        .await
        .expect("list blocked after anchor");
    assert!(
        open_after.is_empty(),
        "supplying the manual anchor clears the no_manual_anchor BLOCKED record"
    );

    // anchoring with an empty/BLOCKED manual id is rejected (lineage binding
    // rejection): you cannot anchor to a non-id sentinel.
    let other_action = format!("test.command-{}", Uuid::new_v4());
    let bad_anchor = store
        .anchor_command_manual(&other_action, MANUAL_ANCHOR_BLOCKED)
        .await;
    assert!(
        bad_anchor.is_err(),
        "anchoring to the BLOCKED sentinel (not a real id) must be rejected"
    );

    // (6) event emission: one block recorded, one cleared, one anchored.
    let blk_after = store
        .count_events_for_aggregate(
            command_corpus_event_family::CORPUS_BLOCKED_RECORDED,
            "atelier_command_corpus_blocked",
            &action_id,
        )
        .await
        .expect("count run-scoped blocked-recorded events");
    let clr_after = store
        .count_events_for_aggregate(
            command_corpus_event_family::CORPUS_BLOCKED_CLEARED,
            "atelier_command_corpus_blocked",
            &action_id,
        )
        .await
        .expect("count run-scoped blocked-cleared events");
    let anc_after = store
        .count_events_for_aggregate(
            command_corpus_event_family::CORPUS_ENTRY_ANCHORED,
            "atelier_command_corpus_entry",
            &action_id,
        )
        .await
        .expect("count run-scoped anchored events");
    assert_eq!(
        blk_after, 2,
        "two record_blocked_command calls emit two CORPUS_BLOCKED_RECORDED events"
    );
    assert_eq!(
        clr_after, 1,
        "clearing the block on anchor emits one CORPUS_BLOCKED_CLEARED event"
    );
    assert_eq!(
        anc_after, 1,
        "anchoring emits one CORPUS_ENTRY_ANCHORED event"
    );
}

#[tokio::test]
async fn corpus_invocation_guard_denies_blocked_actions_until_anchor_is_supplied() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP corpus_invocation_guard_denies_blocked_actions_until_anchor_is_supplied: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let input = unique_entry_input(MANUAL_ANCHOR_BLOCKED);
    let action_id = input.action_id.clone();
    store
        .upsert_command_corpus_entry(&input)
        .await
        .expect("upsert command whose manual anchor is blocked");

    let sentinel_denial = store
        .guard_command_corpus_invocable(&action_id)
        .await
        .expect_err("BLOCKED manual anchor must deny invocation even before a record exists");
    assert_blocked_invocation_denial(sentinel_denial, &action_id, "manual_anchor=BLOCKED");

    store
        .record_blocked_command(
            &action_id,
            BlockedReason::NoManualAnchor,
            CorpusSource::IpcHandler,
            "bind a live ModelManual command id before invocation",
        )
        .await
        .expect("record missing-manual blocked command");

    let recorded_denial = store
        .guard_command_corpus_invocable(&action_id)
        .await
        .expect_err("open BLOCKED record must deny invocation");
    assert_blocked_invocation_denial(recorded_denial, &action_id, "no_manual_anchor");

    let live_manual = format!("manual.cmd-{}", Uuid::new_v4());
    store
        .anchor_command_manual(&action_id, &live_manual)
        .await
        .expect("anchor command to clear missing-manual block");

    let invocable = store
        .guard_command_corpus_invocable(&action_id)
        .await
        .expect("anchored command with no open blocks is invocable");
    assert_eq!(invocable.action_id, action_id);
    assert_eq!(invocable.manual_anchor, live_manual);
}

#[tokio::test]
async fn corpus_invocation_emits_declared_eventledger_event_and_denies_blocked() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP corpus_invocation_emits_declared_eventledger_event_and_denies_blocked: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let declared_event_family = "atelier.workflow.command_invoked";
    let mut input = unique_entry_input(&format!("manual.cmd-{}", Uuid::new_v4()));
    input.execution_class = ExecutionClass::WorkflowJob;
    input.owner = "atelier.workflow_engine".to_string();
    input.capabilities = vec!["workflow.dispatch".to_string()];
    input.evidence_class = vec![declared_event_family.to_string()];
    let action_id = input.action_id.clone();
    let entry = store
        .upsert_command_corpus_entry(&input)
        .await
        .expect("upsert workflow command with declared invocation evidence");

    let invocation_id = Uuid::new_v4();
    let receipt = store
        .record_command_corpus_invocation(&RecordCommandCorpusInvocation {
            invocation_id,
            action_id: action_id.clone(),
            actor_id: "operator-alpha".to_string(),
            evidence_event_family: declared_event_family.to_string(),
            input_ref: format!("input://command-corpus/{}", Uuid::new_v4()),
            receipt_ref: format!("receipt://command-corpus/{}", Uuid::new_v4()),
        })
        .await
        .expect("record invocation evidence for live command");
    assert_eq!(receipt.invocation_id, invocation_id);
    assert_eq!(receipt.action_id, action_id);
    assert_eq!(receipt.evidence_event_family, declared_event_family);
    assert_eq!(receipt.execution_class, ExecutionClass::WorkflowJob);
    assert_eq!(receipt.receipt_shape, entry.receipt_shape);

    let events = database
        .list_kernel_events_for_aggregate(
            "atelier_command_corpus_invocation",
            &invocation_id.to_string(),
        )
        .await
        .expect("list kernel invocation events");
    assert_eq!(
        events.len(),
        1,
        "exactly one canonical EventLedger row is written for the invocation"
    );
    let event = &events[0];
    assert_eq!(
        event.event_type,
        KernelEventType::AtelierDomainEventRecorded
    );
    assert_eq!(event.source_component, "atelier");
    assert_eq!(
        event.payload["event_family"], declared_event_family,
        "kernel payload event_family must be the descriptor-declared evidence class"
    );
    assert_eq!(
        event.payload["atelier_payload"]["action_id"],
        receipt.action_id
    );
    assert_eq!(
        event.payload["atelier_payload"]["execution_class"],
        "workflow_job"
    );
    assert_eq!(
        event.payload["atelier_payload"]["input_ref"],
        receipt.input_ref
    );
    let serialized_payload = event.payload.to_string();
    assert!(
        !serialized_payload.contains("http://")
            && !serialized_payload.contains("https://")
            && !serialized_payload.contains("C:\\")
            && !serialized_payload.contains("secret"),
        "invocation evidence payload must stay leak-safe and ref-only: {serialized_payload}"
    );

    let undeclared_invocation_id = Uuid::new_v4();
    let undeclared = store
        .record_command_corpus_invocation(&RecordCommandCorpusInvocation {
            invocation_id: undeclared_invocation_id,
            action_id: receipt.action_id.clone(),
            actor_id: "operator-alpha".to_string(),
            evidence_event_family: "atelier.workflow.not_declared".to_string(),
            input_ref: format!("input://command-corpus/{}", Uuid::new_v4()),
            receipt_ref: format!("receipt://command-corpus/{}", Uuid::new_v4()),
        })
        .await
        .expect_err("undeclared evidence event family must be rejected");
    match undeclared {
        AtelierError::Validation(message) => assert!(
            message.contains("declared evidence_class"),
            "undeclared evidence denial must name declared evidence_class: {message}"
        ),
        other => panic!("expected validation error for undeclared evidence family, got {other:?}"),
    }
    let undeclared_events = database
        .list_kernel_events_for_aggregate(
            "atelier_command_corpus_invocation",
            &undeclared_invocation_id.to_string(),
        )
        .await
        .expect("list undeclared invocation events");
    assert!(
        undeclared_events.is_empty(),
        "rejected undeclared evidence family must not append an invocation event"
    );

    let mut blocked_input = unique_entry_input(MANUAL_ANCHOR_BLOCKED);
    blocked_input.execution_class = ExecutionClass::WorkflowJob;
    blocked_input.evidence_class = vec![declared_event_family.to_string()];
    let blocked_action_id = blocked_input.action_id.clone();
    store
        .upsert_command_corpus_entry(&blocked_input)
        .await
        .expect("upsert blocked workflow command");
    store
        .record_blocked_command(
            &blocked_action_id,
            BlockedReason::NoManualAnchor,
            CorpusSource::IpcHandler,
            "bind a live ModelManual command id before invocation",
        )
        .await
        .expect("record missing manual anchor block");
    let blocked_invocation_id = Uuid::new_v4();
    let blocked = store
        .record_command_corpus_invocation(&RecordCommandCorpusInvocation {
            invocation_id: blocked_invocation_id,
            action_id: blocked_action_id.clone(),
            actor_id: "operator-alpha".to_string(),
            evidence_event_family: declared_event_family.to_string(),
            input_ref: format!("input://command-corpus/{}", Uuid::new_v4()),
            receipt_ref: format!("receipt://command-corpus/{}", Uuid::new_v4()),
        })
        .await
        .expect_err("BLOCKED command must not emit invocation evidence");
    assert_blocked_invocation_denial(blocked, &blocked_action_id, "no_manual_anchor");
    let blocked_events = database
        .list_kernel_events_for_aggregate(
            "atelier_command_corpus_invocation",
            &blocked_invocation_id.to_string(),
        )
        .await
        .expect("list blocked invocation events");
    assert!(
        blocked_events.is_empty(),
        "blocked invocation must not append an invocation event"
    );
}

#[tokio::test]
async fn corpus_parity_report_projection_counts_and_event() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP corpus_parity_report_projection_counts_and_event: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    // Seed a covered (live-anchored, not blocked) descriptor and a blocked one
    // so the projection has at least one of each to count.
    let covered = unique_entry_input(&format!("manual.cmd-{}", Uuid::new_v4()));
    let covered_id = covered.action_id.clone();
    store
        .upsert_command_corpus_entry(&covered)
        .await
        .expect("upsert covered entry");

    let blocked = unique_entry_input(MANUAL_ANCHOR_BLOCKED);
    let blocked_id = blocked.action_id.clone();
    store
        .upsert_command_corpus_entry(&blocked)
        .await
        .expect("upsert to-be-blocked entry");
    store
        .record_blocked_command(
            &blocked_id,
            BlockedReason::NoManualAnchor,
            CorpusSource::Preload,
            "needs a manual anchor",
        )
        .await
        .expect("record block for parity entry");

    // An orphaned manual id (PARITY-MANUAL-002): a manual entry naming an
    // action_id that is absent from the corpus.
    let orphan = format!("test.orphan-{}", Uuid::new_v4());

    let report = store
        .build_command_corpus_parity_report(&[orphan.clone()])
        .await
        .expect("build parity report");

    // (3) round-trip / (5) projection invariants: the report is a deterministic
    // mechanical count over the whole corpus, so >= the rows we just seeded.
    assert!(
        report.total_corpus >= 2,
        "total_corpus counts every descriptor (>= our two seeds)"
    );
    assert!(
        report.blocked_count >= 1,
        "blocked_count includes our blocked descriptor"
    );
    assert!(
        report.covered_count >= 1,
        "covered_count includes our live-anchored, non-blocked descriptor"
    );
    assert_eq!(
        report.orphaned_manual_count, 1,
        "the single orphaned manual id is counted exactly once"
    );

    // The seeded blocked id appears as a `blocked` defect row; the orphan as an
    // `orphaned_manual` defect row.
    assert!(
        report
            .defects
            .iter()
            .any(|d| d.action_id == blocked_id && d.defect_kind == "blocked"),
        "blocked descriptor surfaces a 'blocked' defect row"
    );
    assert!(
        report
            .defects
            .iter()
            .any(|d| d.action_id == orphan && d.defect_kind == "orphaned_manual"),
        "orphaned manual id surfaces an 'orphaned_manual' defect row"
    );
    // The covered descriptor is NOT a defect.
    assert!(
        !report.defects.iter().any(|d| d.action_id == covered_id),
        "a covered descriptor produces no defect row"
    );

    // latest report is the one we just built.
    let latest = store
        .latest_command_corpus_parity_report()
        .await
        .expect("fetch latest parity report")
        .expect("a report exists");
    assert_eq!(
        latest.report_id, report.report_id,
        "the latest parity report is the one just materialized"
    );

    // (6) event emission: building the report emitted one CORPUS_PARITY_REPORTED.
    let report_event_count = store
        .count_events_for_aggregate(
            command_corpus_event_family::CORPUS_PARITY_REPORTED,
            "atelier_command_corpus_parity_report",
            &report.report_id.to_string(),
        )
        .await
        .expect("count run-scoped parity report event");
    assert_eq!(
        report_event_count, 1,
        "building a parity report emits exactly one CORPUS_PARITY_REPORTED event"
    );
}

/// MT-206 headline proof: the FULL builtin CKC command corpus (~100+ commands,
/// one descriptor per handler registered in `handshake_invoke_handlers!`) is
/// enumerated, projected into PostgreSQL, RE-READ from PostgreSQL, and
/// cross-checked descriptor-by-descriptor against the live ModelManual --
/// covered commands carry their manual id, uncovered commands are BLOCKED with
/// a durable `no_manual_anchor` record, and the parity report counts the full
/// enumeration (never `>= 2` synthetic rows).
#[tokio::test]
async fn corpus_full_builtin_enumeration_bootstraps_and_parity_checks_against_manual() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP corpus_full_builtin_enumeration_bootstraps_and_parity_checks_against_manual: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    // --- the const enumeration itself: the full ~100+ corpus headline ---
    let builtin = builtin_command_corpus();
    assert!(
        builtin.len() >= 100,
        "the builtin CKC command corpus must enumerate the full ~100+ command \
         surface, got only {}",
        builtin.len()
    );
    let builtin_ids: std::collections::BTreeSet<&str> =
        builtin.iter().map(|e| e.action_id.as_str()).collect();
    assert_eq!(
        builtin_ids.len(),
        builtin.len(),
        "every builtin action_id is unique (LAW-CORPUS-PARITY-001: one catalog, \
         each action id exactly once)"
    );
    for input in &builtin {
        assert!(
            !input.owner.trim().is_empty() && !input.owner.eq_ignore_ascii_case("ui"),
            "{}: owner must be a real backend module, never 'ui'",
            input.action_id
        );
        assert!(
            !input.params_schema_ref.trim().is_empty()
                && !input.receipt_shape.trim().is_empty(),
            "{}: typed params/receipt schema refs are mandatory (10.19.2)",
            input.action_id
        );
        assert!(
            !input.errors.is_empty(),
            "{}: every descriptor enumerates typed error variants",
            input.action_id
        );
        if input.execution_class.requires_governed_execution() {
            assert!(
                !input.evidence_class.is_empty(),
                "{}: governed execution_class {:?} must declare an evidence class \
                 (EVIDENCE-001)",
                input.action_id,
                input.execution_class
            );
        }
    }

    // --- bootstrap: project the FULL corpus into PostgreSQL ---
    let receipt = store
        .bootstrap_builtin_command_corpus()
        .await
        .expect("bootstrap the full builtin command corpus");
    assert_eq!(
        receipt.total_commands,
        builtin.len(),
        "the bootstrap receipt counts the full enumeration"
    );
    assert_eq!(
        receipt.covered_count + receipt.blocked_count,
        receipt.total_commands,
        "every builtin command is either manual-covered or BLOCKED, never \
         silently omitted (BLOCKED-001)"
    );

    // --- RE-READ from PostgreSQL: the whole catalog, then index by action_id ---
    let listed = store
        .list_command_corpus_entries(None)
        .await
        .expect("list the persisted catalog");
    let persisted: std::collections::BTreeMap<&str, _> = listed
        .iter()
        .map(|e| (e.action_id.as_str(), e))
        .collect();

    // --- live ModelManual cross-check, descriptor by descriptor ---
    // The covering rule mirrors the bootstrap contract: a manual
    // CommandReference covers a corpus command when its tauri_command binding
    // (or its own id/name) names the action id. This scan runs against the
    // LIVE manual content, so the expected covered/blocked split is derived,
    // never hardcoded.
    let manual = handshake_core::model_manual::model_manual();
    let mut expected_covered: usize = 0;
    let mut expected_blocked: usize = 0;
    let mut blocked_builtin_ids: Vec<String> = Vec::new();
    for input in &builtin {
        let entry = persisted.get(input.action_id.as_str()).unwrap_or_else(|| {
            panic!(
                "builtin command {} must be persisted in the PostgreSQL catalog",
                input.action_id
            )
        });
        // Re-read field fidelity: the persisted descriptor matches the
        // enumerated one.
        assert_eq!(entry.owner, input.owner, "{}: owner persists", input.action_id);
        assert_eq!(
            entry.corpus_source, input.corpus_source,
            "{}: corpus_source persists",
            input.action_id
        );
        assert_eq!(
            entry.execution_class, input.execution_class,
            "{}: execution_class persists",
            input.action_id
        );
        assert_eq!(
            entry.foreground_flag, input.foreground_flag,
            "{}: foreground_flag persists (HBR-QUIET)",
            input.action_id
        );

        let covering = manual.command_reference.iter().find(|cmd| {
            cmd.tauri_command == Some(input.action_id.as_str())
                || cmd.id == input.action_id
                || cmd.name == input.action_id
        });
        let open_blocks = store
            .list_blocked_commands(Some(&input.action_id))
            .await
            .expect("list open BLOCKED records for builtin command");
        match covering {
            Some(cmd) => {
                expected_covered += 1;
                assert_eq!(
                    entry.manual_anchor, cmd.id,
                    "{}: manual-covered command is anchored to the live \
                     ModelManual id",
                    input.action_id
                );
                assert!(
                    !open_blocks
                        .iter()
                        .any(|b| b.blocked_reason == BlockedReason::NoManualAnchor),
                    "{}: manual-covered command carries no stale no_manual_anchor \
                     BLOCKED record",
                    input.action_id
                );
            }
            None => {
                expected_blocked += 1;
                blocked_builtin_ids.push(input.action_id.clone());
                assert_eq!(
                    entry.manual_anchor, MANUAL_ANCHOR_BLOCKED,
                    "{}: command without manual coverage is pinned to the \
                     BLOCKED sentinel",
                    input.action_id
                );
                assert!(
                    open_blocks
                        .iter()
                        .any(|b| b.blocked_reason == BlockedReason::NoManualAnchor),
                    "{}: command without manual coverage carries a durable \
                     no_manual_anchor BLOCKED record (BLOCKED-001)",
                    input.action_id
                );
            }
        }
    }
    assert_eq!(
        receipt.covered_count, expected_covered,
        "receipt covered_count matches the independent live-manual scan"
    );
    assert_eq!(
        receipt.blocked_count, expected_blocked,
        "receipt blocked_count matches the independent live-manual scan"
    );

    // --- EventLedger evidence for the full set: every builtin projection
    // emitted CORPUS_ENTRY_UPSERTED, and the anchored/blocked event matching
    // its manual-coverage outcome. Builtin ids are stable across runs, so the
    // counts are cumulative (>= 1). ---
    for input in &builtin {
        let upserts = store
            .count_events_for_aggregate(
                command_corpus_event_family::CORPUS_ENTRY_UPSERTED,
                "atelier_command_corpus_entry",
                &input.action_id,
            )
            .await
            .expect("count upsert events for builtin command");
        assert!(
            upserts >= 1,
            "{}: projecting the descriptor emits CORPUS_ENTRY_UPSERTED",
            input.action_id
        );
        let outcome_events = if blocked_builtin_ids.contains(&input.action_id) {
            store
                .count_events_for_aggregate(
                    command_corpus_event_family::CORPUS_BLOCKED_RECORDED,
                    "atelier_command_corpus_blocked",
                    &input.action_id,
                )
                .await
                .expect("count blocked-recorded events for builtin command")
        } else {
            store
                .count_events_for_aggregate(
                    command_corpus_event_family::CORPUS_ENTRY_ANCHORED,
                    "atelier_command_corpus_entry",
                    &input.action_id,
                )
                .await
                .expect("count anchored events for builtin command")
        };
        assert!(
            outcome_events >= 1,
            "{}: the manual cross-check outcome (anchored or blocked) is \
             EventLedger evidence",
            input.action_id
        );
    }

    // --- the parity report asserts the FULL enumeration ---
    // The catalog is shared with other tests' run-unique seeds, so the report
    // covers at least the full builtin corpus.
    let report = store
        .build_command_corpus_parity_report(&receipt.manual_advertised_action_ids)
        .await
        .expect("build parity report over the bootstrapped corpus");
    assert!(
        report.total_corpus >= builtin.len() as i64,
        "parity report total_corpus ({}) covers the full builtin enumeration ({})",
        report.total_corpus,
        builtin.len()
    );
    assert!(
        report.covered_count >= expected_covered as i64,
        "parity report covered_count ({}) includes every manual-covered builtin \
         command ({expected_covered})",
        report.covered_count
    );
    assert!(
        report.blocked_count >= expected_blocked as i64,
        "parity report blocked_count ({}) includes every BLOCKED builtin \
         command ({expected_blocked})",
        report.blocked_count
    );
    for blocked_id in &blocked_builtin_ids {
        assert!(
            report
                .defects
                .iter()
                .any(|d| &d.action_id == blocked_id && d.defect_kind == "blocked"),
            "{blocked_id}: every BLOCKED builtin command surfaces a 'blocked' \
             defect row in the parity report"
        );
    }
    // Manual-advertised ids that ARE in the corpus are not orphans; the manual
    // binds its tauri_command names to registered handlers, so the builtin
    // enumeration must leave no manual binding orphaned.
    assert!(
        !report.defects.iter().any(|d| {
            d.defect_kind == "orphaned_manual" && builtin_ids.contains(d.action_id.as_str())
        }),
        "no builtin command may simultaneously be an orphaned_manual defect"
    );

    // --- idempotency: re-bootstrapping re-projects in place ---
    let receipt2 = store
        .bootstrap_builtin_command_corpus()
        .await
        .expect("re-bootstrap the builtin corpus");
    assert_eq!(
        receipt2.total_commands, receipt.total_commands,
        "re-bootstrap projects the same full enumeration"
    );
    assert_eq!(receipt2.covered_count, receipt.covered_count);
    assert_eq!(receipt2.blocked_count, receipt.blocked_count);
    let relisted = store
        .list_command_corpus_entries(None)
        .await
        .expect("re-list the persisted catalog");
    for input in &builtin {
        let rows: Vec<_> = relisted
            .iter()
            .filter(|e| e.action_id == input.action_id)
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "{}: exactly one catalog row after re-bootstrap (stable upsert key)",
            input.action_id
        );
        let before = persisted
            .get(input.action_id.as_str())
            .expect("entry was persisted in the first bootstrap");
        assert_eq!(
            rows[0].entry_id, before.entry_id,
            "{}: entry_id is stable across re-bootstrap (no duplicate row)",
            input.action_id
        );
    }
}
