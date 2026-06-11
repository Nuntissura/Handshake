//! WP-KERNEL-005 MT-151 proof — validator first-pass runs in a real
//! PG-backed sandbox.
//!
//! The REAL `ValidatorFirstPassEvaluator` from the self-improvement loop is
//! driven through the production implementations: `PgSelfImproveSandbox`
//! provisions a real per-run sandbox workspace and persists the run to
//! `atelier_self_improve_sandbox_run`, and `HbrFirstPassRunner` executes
//! the real HBR handoff gate per corpus item (appending the canonical
//! `HBR_HANDOFF_GATE` EventLedger row) and persists every execution to
//! `atelier_validator_first_pass_run`. All rows are RE-READ from
//! Handshake-managed PostgreSQL and cross-checked against the evaluator's
//! metrics — no stubs, no fresh-UUID sandboxes, no mock runners.

mod atelier_pg_support;

use std::collections::BTreeMap;
use std::sync::Arc;

use atelier_pg_support::database_url;
use handshake_core::atelier::validator_first_pass::{
    snapshot_sha256, validator_first_pass_event_family, FirstPassFixture, FirstPassFixtureMatrix,
    FirstPassFixtureMatrixRow, FirstPassFixtureNaRow, FirstPassFixtureRule, HbrFirstPassRunner,
    PgSelfImproveSandbox, SANDBOX_RUN_STATUS_PROVISIONED,
};
use handshake_core::atelier::AtelierStore;
use handshake_core::kernel::KernelEventType;
use handshake_core::self_improve::corpus::{
    CorpusItem, HbrTestPacketCorpus, StaticKeyProvider, ValidatorVerdict,
};
use handshake_core::self_improve::editable_surface::EditableSurfaceSnapshot;
use handshake_core::self_improve::evaluator::{ValidatorFirstPassEvaluator, ValidatorRunner};
use handshake_core::self_improve::loop_core::{Evaluator, LoopSandbox};
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

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

const FIRST_PASS_TRANSITION: &str = "CoderToWpValidator";
const FIRST_PASS_EVIDENCE_KIND: &str = "test_run_with_ledger_replay";

/// A fixture whose acceptance matrix satisfies the real handoff gate.
fn passing_fixture(hbr_id: &str) -> serde_json::Value {
    serde_json::to_value(FirstPassFixture {
        transition: FIRST_PASS_TRANSITION.to_string(),
        rules: vec![FirstPassFixtureRule {
            hbr_id: hbr_id.to_string(),
            evidence_kind: FIRST_PASS_EVIDENCE_KIND.to_string(),
        }],
        acceptance_matrix: FirstPassFixtureMatrix {
            hbr: vec![FirstPassFixtureMatrixRow {
                hbr_id: hbr_id.to_string(),
                status: "PROVED".to_string(),
                evidence_pointer: Some("tests/atelier_event_ledger_tests.rs".to_string()),
                validator_verdict: Some("PROVED".to_string()),
            }],
            hbr_not_applicable: vec![],
        },
    })
    .expect("serialize passing fixture")
}

/// A fixture the real handoff gate must block (PENDING row).
fn failing_fixture(hbr_id: &str) -> serde_json::Value {
    serde_json::to_value(FirstPassFixture {
        transition: FIRST_PASS_TRANSITION.to_string(),
        rules: vec![FirstPassFixtureRule {
            hbr_id: hbr_id.to_string(),
            evidence_kind: FIRST_PASS_EVIDENCE_KIND.to_string(),
        }],
        acceptance_matrix: FirstPassFixtureMatrix {
            hbr: vec![FirstPassFixtureMatrixRow {
                hbr_id: hbr_id.to_string(),
                status: "PENDING".to_string(),
                evidence_pointer: None,
                validator_verdict: None,
            }],
            hbr_not_applicable: vec![FirstPassFixtureNaRow {
                hbr_id: "HBR-UNRELATED".to_string(),
                reason: "different rule entirely".to_string(),
            }],
        },
    })
    .expect("serialize failing fixture")
}

fn corpus_item(index: u32, run_tag: &str, passing: bool) -> CorpusItem {
    let hbr_id = format!("HBR-MT151-{index:02}");
    CorpusItem {
        id: Uuid::now_v7(),
        hbr_rule_id: hbr_id.clone(),
        packet_under_test: format!("WP-MT151-{run_tag}-{index:02}"),
        expected_first_pass_verdict: if passing {
            ValidatorVerdict::Pass
        } else {
            ValidatorVerdict::Fail
        },
        fixtures: if passing {
            passing_fixture(&hbr_id)
        } else {
            failing_fixture(&hbr_id)
        },
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt151_real_evaluator_runs_first_pass_through_pg_backed_sandbox_and_hbr_gate() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt151_first_pass_sandbox: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let run_tag = Uuid::now_v7().simple().to_string();
    let items: Vec<CorpusItem> = (1..=6)
        .map(|index| corpus_item(index, &run_tag, index <= 4))
        .collect();
    let expected: BTreeMap<Uuid, ValidatorVerdict> = items
        .iter()
        .map(|item| (item.id, item.expected_first_pass_verdict))
        .collect();
    let corpus = HbrTestPacketCorpus::from_items(items).expect("build MT-151 corpus");
    let key_provider = StaticKeyProvider::deterministic("mt151-key");
    let split = corpus
        .split(7, &key_provider, "mt151-key")
        .expect("split MT-151 corpus");

    // Production sandbox + production runner, wired to the same PG store.
    let sandbox_root = tempfile::tempdir().expect("create sandbox root");
    let sandbox = PgSelfImproveSandbox::new(store.clone(), sandbox_root.path().to_path_buf());
    let run_slot = sandbox.run_slot();
    let runner = HbrFirstPassRunner::new(store.clone(), database.clone(), sandbox.run_slot());
    let evaluator = ValidatorFirstPassEvaluator::new(&sandbox, &runner);

    let snapshot = EditableSurfaceSnapshot::ModelManual {
        manual_section_id: format!("manual.capsule.mt151-{run_tag}"),
        before_text: "baseline manual guidance".to_string(),
        after_text: "candidate manual guidance tuned by the loop".to_string(),
    };

    let result = evaluator
        .evaluate(&split, &key_provider, &snapshot)
        .expect("real evaluator must run through the production sandbox + gate");

    // --- Sandbox run: persisted, re-read, and materialised on disk. ------
    let sandbox_run_id = run_slot
        .lock()
        .expect("run slot lock")
        .expect("sandbox must record its persisted run id");
    let sandbox_run = store
        .get_self_improve_sandbox_run(sandbox_run_id)
        .await
        .expect("re-read sandbox run from PostgreSQL");
    assert_eq!(sandbox_run.status, SANDBOX_RUN_STATUS_PROVISIONED);
    assert_eq!(sandbox_run.surface_kind, "model_manual");
    assert_eq!(
        sandbox_run.snapshot_sha256,
        snapshot_sha256(&snapshot).expect("hash snapshot"),
        "persisted hash must match the evaluated snapshot"
    );
    assert!(sandbox_run.completed_at_utc >= sandbox_run.started_at_utc);
    let workspace = std::path::PathBuf::from(&sandbox_run.workspace_ref);
    let candidate_file = workspace.join("model_manual_section.txt");
    let materialised =
        std::fs::read_to_string(&candidate_file).expect("sandbox candidate file must exist");
    assert!(
        materialised.contains("candidate manual guidance tuned by the loop"),
        "the sandbox world must carry the candidate after-value"
    );

    let sandbox_events = database
        .list_kernel_events_for_aggregate(
            "atelier_self_improve_sandbox_run",
            &sandbox_run_id.to_string(),
        )
        .await
        .expect("list sandbox run EventLedger rows");
    assert!(
        sandbox_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == validator_first_pass_event_family::SANDBOX_RUN_PROVISIONED
        }),
        "sandbox provisioning must emit its canonical EventLedger event"
    );

    // --- First-pass rows: one per corpus item, verdicts from the REAL gate.
    let runs = store
        .list_validator_first_pass_runs_for_sandbox(sandbox_run_id)
        .await
        .expect("re-read first-pass runs from PostgreSQL");
    assert_eq!(runs.len(), 6, "train + dev + holdout must all persist");
    let mut pg_pass_count = 0u32;
    for run in &runs {
        let expected_verdict = expected
            .get(&run.corpus_item_id)
            .expect("first-pass row must cite a known corpus item");
        assert_eq!(
            run.verdict, *expected_verdict,
            "PG-persisted verdict must come from the real HBR gate"
        );
        assert_eq!(run.transition, FIRST_PASS_TRANSITION);
        assert!(
            run.gate_event_id.is_some(),
            "every first-pass must link its handoff gate evaluation"
        );
        assert!(run.capsule_bytes > 0);
        match run.verdict {
            ValidatorVerdict::Pass => {
                pg_pass_count += 1;
                assert_eq!(run.failing_rule_count, 0);
            }
            ValidatorVerdict::Fail => {
                assert!(
                    run.failing_rule_count >= 1,
                    "a blocked gate must persist its failing rules"
                );
            }
            ValidatorVerdict::Skip => panic!("no fixture in this corpus is skippable"),
        }
    }

    // --- Evaluator metrics agree with the PG-re-read reality. ------------
    let metric_pass_count =
        result.train.pass_count + result.dev.pass_count + result.holdout.pass_count;
    let metric_total =
        result.train.total_count + result.dev.total_count + result.holdout.total_count;
    assert_eq!(metric_total, 6);
    assert_eq!(
        metric_pass_count, pg_pass_count,
        "evaluator pass counts must match the persisted first-pass rows"
    );
    assert_eq!(pg_pass_count, 4, "4 of 6 fixtures satisfy the real gate");

    // --- The real HBR gate appended its own canonical ledger row. --------
    let passing_run = runs
        .iter()
        .find(|run| run.verdict == ValidatorVerdict::Pass)
        .expect("a passing first-pass run exists");
    let gate_events = database
        .list_kernel_events_for_aggregate("work_packet", &passing_run.packet_under_test)
        .await
        .expect("list HBR handoff gate EventLedger rows");
    assert!(
        gate_events
            .iter()
            .any(|event| event.event_type == KernelEventType::HbrHandoffGate),
        "the first-pass must run the real handoff gate, which appends HBR_HANDOFF_GATE"
    );

    // --- And the first-pass persistence mirrored through the ledger. -----
    let first_pass_events = database
        .list_kernel_events_for_aggregate(
            "atelier_validator_first_pass_run",
            &passing_run.first_pass_run_id.to_string(),
        )
        .await
        .expect("list first-pass EventLedger rows");
    let event = first_pass_events
        .iter()
        .find(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == validator_first_pass_event_family::VALIDATOR_FIRST_PASS_RECORDED
        })
        .expect("first-pass persistence must emit its canonical EventLedger event");
    assert_eq!(
        event.payload["atelier_payload"]["verdict"],
        serde_json::json!("pass")
    );
    assert_eq!(
        event.payload["atelier_payload"]["sandbox_run_id"],
        serde_json::json!(sandbox_run_id)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt151_runner_rejects_fixtureless_items_without_persisting_rows() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt151_fixtureless: PostgreSQL unavailable");
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let sandbox_root = tempfile::tempdir().expect("create sandbox root");
    let sandbox = PgSelfImproveSandbox::new(store.clone(), sandbox_root.path().to_path_buf());
    let snapshot = EditableSurfaceSnapshot::ModelManual {
        manual_section_id: "manual.capsule.mt151-neg".to_string(),
        before_text: "a".to_string(),
        after_text: "b".to_string(),
    };
    // Provision a real sandbox run so the negative path is linked too.
    sandbox.run(&snapshot).expect("provision sandbox");
    let sandbox_run_id = sandbox
        .run_slot()
        .lock()
        .expect("run slot lock")
        .expect("sandbox run id");

    let runner = HbrFirstPassRunner::new(store.clone(), database.clone(), sandbox.run_slot());
    let item = CorpusItem {
        id: Uuid::now_v7(),
        hbr_rule_id: "HBR-MT151-NEG".to_string(),
        packet_under_test: "WP-MT151-NEG".to_string(),
        expected_first_pass_verdict: ValidatorVerdict::Fail,
        fixtures: serde_json::json!({}),
    };
    runner
        .run(&item, &snapshot)
        .expect_err("an item without a first-pass fixture must be a typed runner error");

    let runs = store
        .list_validator_first_pass_runs_for_sandbox(sandbox_run_id)
        .await
        .expect("re-read first-pass runs");
    assert!(
        runs.is_empty(),
        "a rejected fixture must not persist a first-pass row"
    );
}
