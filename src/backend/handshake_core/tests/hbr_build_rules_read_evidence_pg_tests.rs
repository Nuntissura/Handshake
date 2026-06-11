//! WP-KERNEL-005 MT-164 proof — Build Rules Read Evidence at runtime.
//!
//! The CKC build-rule gate is preserved as model-visible read/acknowledge
//! evidence before build/test/release WPs. The static half (the `build-rules`
//! ModelManual group + `hbr_violation_emit` / `hbr_matrix_check` /
//! `hbr_validator_scan` rows) is covered by `model_manual_tests.rs`; this file
//! adds the runtime half the contract's proof_target demands: the REAL
//! `HandoffGate` evaluated against Handshake-managed PostgreSQL, with the
//! canonical `HBR_HANDOFF_GATE` EventLedger row RE-READ from
//! `kernel_event_ledger` afterwards.
//!
//! Proven here:
//!   * a packet whose acceptance matrix carries read/acknowledge evidence
//!     (PROVED row + evidence_pointer) passes the gate, and the appended
//!     EventLedger row records that evidence;
//!   * a packet that never acknowledged the build rule (PENDING row, no
//!     evidence) is BLOCKED before the build/test/release handoff, and the
//!     block verdict is itself durable in PostgreSQL;
//!   * the manual's HBR rows are status-vs-route consistent: every Wired row
//!     names a concrete invocation route (CLI flag) even when no IPC channel
//!     exists.
//!
//! Gated on `atelier_pg_support::database_url()` (Handshake-managed
//! PostgreSQL); prints SKIP when no database is available. Never SQLite.

mod atelier_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use atelier_pg_support::database_url;
use handshake_core::hbr::handoff_gate::{
    HandoffEventLedger, HandoffEventLedgerError, HandoffGate, HandoffRule, HandoffTransition,
    HbrAcceptanceMatrix, HbrMatrixRow, HbrNotApplicableRow, HbrPacket,
};
use handshake_core::kernel::{KernelEvent, KernelEventType, NewKernelEvent};
use handshake_core::model_manual::{model_manual, CommandStatus};
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

const BUILD_RULE_ID: &str = "HBR-BUILD-RULES-READ-EVIDENCE";
const BUILD_RULE_EVIDENCE_KIND: &str = "test_run_with_ledger_replay";
const MANUAL_HBR_ROW_IDS: [&str; 3] =
    ["hbr_violation_emit", "hbr_matrix_check", "hbr_validator_scan"];

/// EventLedger handle the gate appends through — the same production wrapper
/// shape `HbrFirstPassRunner` uses (`src/atelier/validator_first_pass.rs`):
/// every gate event lands in the real PostgreSQL `kernel_event_ledger`.
struct ArcDatabaseLedger(Arc<dyn Database>);

#[async_trait]
impl HandoffEventLedger for ArcDatabaseLedger {
    async fn append_handoff_event(
        &self,
        event: NewKernelEvent,
    ) -> Result<KernelEvent, HandoffEventLedgerError> {
        self.0
            .append_kernel_event(event)
            .await
            .map_err(|error| HandoffEventLedgerError::new(error.to_string()))
    }
}

async fn connected_database(url: &str) -> Arc<dyn Database> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool);
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    database.into_arc()
}

fn gate(database: Arc<dyn Database>) -> HandoffGate<ArcDatabaseLedger> {
    HandoffGate::new(
        ArcDatabaseLedger(database),
        vec![HandoffRule::new(BUILD_RULE_ID, BUILD_RULE_EVIDENCE_KIND)],
    )
}

fn packet(wp_id: &str, status: &str, evidence_pointer: Option<&str>) -> HbrPacket {
    HbrPacket {
        wp_id: wp_id.to_string(),
        acceptance_matrix: HbrAcceptanceMatrix {
            hbr: vec![HbrMatrixRow {
                hbr_id: BUILD_RULE_ID.to_string(),
                status: status.to_string(),
                evidence_pointer: evidence_pointer.map(str::to_string),
                validator_verdict: evidence_pointer.map(|_| "PROVED".to_string()),
            }],
            hbr_not_applicable: Vec::<HbrNotApplicableRow>::new(),
        },
    }
}

/// MT-164: read/acknowledge evidence passes the real gate, and the gate's
/// canonical `HBR_HANDOFF_GATE` EventLedger row — including the acknowledged
/// evidence pointer — is durable in and RE-READ from PostgreSQL.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt164_build_rule_read_evidence_passes_gate_and_persists_ledger_row() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt164_pass: PostgreSQL unavailable");
        return;
    };
    let database = connected_database(&url).await;

    let wp_id = format!("WP-MT164-PASS-{}", Uuid::now_v7().simple());
    let evidence_pointer = "tests/hbr_build_rules_read_evidence_pg_tests.rs";
    let evidence = gate(database.clone())
        .evaluate(
            &packet(&wp_id, "PROVED", Some(evidence_pointer)),
            HandoffTransition::CoderToWpValidator,
        )
        .await
        .expect("acknowledged build rule must pass the handoff gate");
    assert_eq!(evidence.wp_id, wp_id);
    assert_eq!(evidence.event.event_type, KernelEventType::HbrHandoffGate);

    // RE-READ from PostgreSQL: the gate event is durable on the work packet
    // aggregate, with the read/acknowledge evidence inside the payload.
    let events = database
        .list_kernel_events_for_aggregate("work_packet", &wp_id)
        .await
        .expect("re-read gate events from kernel_event_ledger");
    let gate_event = events
        .iter()
        .find(|event| event.event_type == KernelEventType::HbrHandoffGate)
        .expect("the HBR_HANDOFF_GATE event must be persisted in PostgreSQL");
    assert_eq!(gate_event.payload["wp_id"], serde_json::json!(wp_id));
    assert_eq!(
        gate_event.payload["transition"],
        serde_json::json!("CoderToWpValidator")
    );
    assert_eq!(gate_event.payload["verdict"]["kind"], serde_json::json!("Pass"));
    let evaluated = gate_event.payload["evaluated_rules"]
        .as_array()
        .expect("evaluated_rules array");
    assert!(
        evaluated.iter().any(|rule| {
            rule["hbr_id"] == serde_json::json!(BUILD_RULE_ID)
                && rule["evidence_pointer"] == serde_json::json!(evidence_pointer)
        }),
        "the persisted gate row must carry the acknowledged evidence pointer, got {evaluated:?}"
    );
}

/// MT-164: a build rule that was never read/acknowledged (PENDING, no
/// evidence) blocks the build/test/release handoff, and the BLOCK verdict is
/// itself a durable PostgreSQL EventLedger row a model can read back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt164_unacknowledged_build_rule_blocks_gate_with_durable_block_row() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt164_block: PostgreSQL unavailable");
        return;
    };
    let database = connected_database(&url).await;

    let wp_id = format!("WP-MT164-BLOCK-{}", Uuid::now_v7().simple());
    let block = gate(database.clone())
        .evaluate(
            &packet(&wp_id, "PENDING", None),
            HandoffTransition::CoderToWpValidator,
        )
        .await
        .expect_err("an unacknowledged build rule must block the handoff gate");
    assert_eq!(block.wp_id, wp_id);
    assert!(
        block
            .failing_rules
            .iter()
            .any(|rule| rule.hbr_id == BUILD_RULE_ID),
        "the block must name the unacknowledged build rule, got {:?}",
        block.failing_rules
    );
    assert!(
        block.event.is_some(),
        "a matrix block must still append its EventLedger row"
    );

    // RE-READ from PostgreSQL: the block verdict is durable too.
    let events = database
        .list_kernel_events_for_aggregate("work_packet", &wp_id)
        .await
        .expect("re-read gate events from kernel_event_ledger");
    let gate_event = events
        .iter()
        .find(|event| event.event_type == KernelEventType::HbrHandoffGate)
        .expect("the blocked HBR_HANDOFF_GATE event must be persisted");
    assert_eq!(
        gate_event.payload["verdict"]["kind"],
        serde_json::json!("Block")
    );
    let failing = gate_event.payload["verdict"]["failing_rules"]
        .as_array()
        .expect("failing_rules array");
    assert!(
        failing
            .iter()
            .any(|rule| rule["hbr_id"] == serde_json::json!(BUILD_RULE_ID)),
        "the persisted block row must name the unacknowledged rule, got {failing:?}"
    );
}

/// MT-164 status-vs-route consistency for the manual's HBR rows: each of the
/// three build-rule commands is Wired and, although it is a CLI/recipe surface
/// with no IPC channel, names the concrete CLI route a model invokes.
#[test]
fn mt164_manual_hbr_rows_are_wired_with_concrete_routes() {
    let manual = model_manual();
    for row_id in MANUAL_HBR_ROW_IDS {
        let row = manual
            .command_reference
            .iter()
            .find(|command| command.id == row_id)
            .unwrap_or_else(|| panic!("manual must carry the {row_id} build-rule row"));
        assert_eq!(
            row.status,
            CommandStatus::Wired,
            "{row_id} must be Wired (it routes to the real HBR gate/receipt tooling)"
        );
        assert!(
            row.ipc_channel.is_some() || row.cli_flag.is_some(),
            "{row_id} is Wired so it must name a concrete route (IPC channel or CLI flag)"
        );
        assert!(
            !row.schema_fields.is_empty(),
            "{row_id} must document the typed fields a model reads/acknowledges"
        );
    }

    let group = manual
        .feature_groups
        .iter()
        .find(|group| group.id == "diagnostics_build_rules_read_evidence")
        .expect("manual must carry the build-rules read-evidence feature group");
    for row_id in MANUAL_HBR_ROW_IDS {
        assert!(
            group.commands.contains(&row_id),
            "build-rules group must reference {row_id}"
        );
    }
}
