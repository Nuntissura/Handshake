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

use handshake_core::atelier::command_corpus::{
    command_corpus_event_family, BlockedReason, CorpusErrorVariant, CorpusSource,
    ExecutionClass, UpsertCommandCorpusEntry, MANUAL_ANCHOR_BLOCKED,
};
use handshake_core::atelier::AtelierStore;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
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

#[tokio::test]
async fn corpus_entry_upsert_roundtrip_idempotency_and_event() {
    let Some(url) = database_url() else {
        eprintln!("SKIP corpus_entry_upsert_roundtrip_idempotency_and_event: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let before = store
        .count_events(command_corpus_event_family::CORPUS_ENTRY_UPSERTED)
        .await
        .expect("count upsert events before");

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
    assert_eq!(entry.manual_anchor, manual_anchor, "live manual anchor stored");
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
    assert_eq!(fetched.entry_id, entry.entry_id, "fetch returns the same row");

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
    assert_eq!(entry2.owner, "atelier.command_corpus.v2", "owner updated in place");

    // Exactly one row for this owner namespace's action id (no duplicate).
    let listed = store
        .list_command_corpus_entries(Some("atelier.command_corpus.v2"))
        .await
        .expect("list entries by owner");
    let matches = listed.iter().filter(|e| e.action_id == action_id).count();
    assert_eq!(matches, 1, "action_id appears exactly once after re-upsert");

    // (6) event emission: two upserts emitted two CORPUS_ENTRY_UPSERTED events.
    let after = store
        .count_events(command_corpus_event_family::CORPUS_ENTRY_UPSERTED)
        .await
        .expect("count upsert events after");
    assert_eq!(
        after,
        before + 2,
        "each upsert emits exactly one CORPUS_ENTRY_UPSERTED event"
    );
}

#[tokio::test]
async fn corpus_external_execution_requires_evidence_invariant() {
    let Some(url) = database_url() else {
        eprintln!("SKIP corpus_external_execution_requires_evidence_invariant: DATABASE_URL not set");
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
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP corpus_blocked_record_upsert_idempotency_clear_on_anchor_and_events: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let blk_before = store
        .count_events(command_corpus_event_family::CORPUS_BLOCKED_RECORDED)
        .await
        .expect("count blocked-recorded events before");
    let clr_before = store
        .count_events(command_corpus_event_family::CORPUS_BLOCKED_CLEARED)
        .await
        .expect("count blocked-cleared events before");
    let anc_before = store
        .count_events(command_corpus_event_family::CORPUS_ENTRY_ANCHORED)
        .await
        .expect("count anchored events before");

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
    assert_eq!(record.action_id, action_id, "blocked record round-trips action_id");
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
        .count_events(command_corpus_event_family::CORPUS_BLOCKED_RECORDED)
        .await
        .expect("count blocked-recorded events after");
    let clr_after = store
        .count_events(command_corpus_event_family::CORPUS_BLOCKED_CLEARED)
        .await
        .expect("count blocked-cleared events after");
    let anc_after = store
        .count_events(command_corpus_event_family::CORPUS_ENTRY_ANCHORED)
        .await
        .expect("count anchored events after");
    assert_eq!(
        blk_after,
        blk_before + 2,
        "two record_blocked_command calls emit two CORPUS_BLOCKED_RECORDED events"
    );
    assert_eq!(
        clr_after,
        clr_before + 1,
        "clearing the block on anchor emits one CORPUS_BLOCKED_CLEARED event"
    );
    assert_eq!(
        anc_after,
        anc_before + 1,
        "anchoring emits one CORPUS_ENTRY_ANCHORED event"
    );
}

#[tokio::test]
async fn corpus_parity_report_projection_counts_and_event() {
    let Some(url) = database_url() else {
        eprintln!("SKIP corpus_parity_report_projection_counts_and_event: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let rep_before = store
        .count_events(command_corpus_event_family::CORPUS_PARITY_REPORTED)
        .await
        .expect("count parity-reported events before");

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
    let rep_after = store
        .count_events(command_corpus_event_family::CORPUS_PARITY_REPORTED)
        .await
        .expect("count parity-reported events after");
    assert_eq!(
        rep_after,
        rep_before + 1,
        "building a parity report emits exactly one CORPUS_PARITY_REPORTED event"
    );
}
