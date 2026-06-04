use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::DateTime;
use serde_json::Value;
use uuid::Uuid;

use handshake_core::hbr::handoff_gate::{
    HandoffEventLedger, HandoffEventLedgerError, HandoffGate, HandoffRule, HandoffTransition,
    HbrAcceptanceMatrix, HbrMatrixRow, HbrNotApplicableRow, HbrPacket,
};
use handshake_core::kernel::{KernelEvent, KernelEventType, NewKernelEvent};

#[derive(Clone, Default)]
struct InMemoryHandoffLedger {
    events: Arc<Mutex<Vec<NewKernelEvent>>>,
}

#[async_trait]
impl HandoffEventLedger for InMemoryHandoffLedger {
    async fn append_handoff_event(
        &self,
        event: NewKernelEvent,
    ) -> Result<KernelEvent, HandoffEventLedgerError> {
        self.events.lock().expect("events lock").push(event.clone());
        Ok(KernelEvent::from_new(event))
    }
}

impl InMemoryHandoffLedger {
    fn events(&self) -> Vec<NewKernelEvent> {
        self.events.lock().expect("events lock").clone()
    }
}

fn handoff_rules() -> Vec<HandoffRule> {
    vec![
        HandoffRule::new("HBR-MAN-001", "manual_diff_in_same_commit"),
        HandoffRule::new("HBR-INT-001", "test_run_with_ledger_replay"),
        HandoffRule::new("HBR-INT-007", "test_run_with_trace_projection_replay"),
        HandoffRule::new(
            "HBR-INT-008",
            "grep_zero_v4_in_touched_files_plus_v7_version_assert_test",
        ),
        HandoffRule::new("HBR-SWARM-001", "concurrent_session_test_run"),
        HandoffRule::new(
            "HBR-QUIET-004",
            "foreground_exception_declaration_and_operator_warning",
        ),
        HandoffRule::new("HBR-MAN-002", "no_context_model_operation_test"),
        HandoffRule::new(
            "HBR-STOP-001",
            "absence_of_capacity_reasoning_in_receipts_and_lifecycle",
        ),
    ]
}

#[derive(Clone, Default)]
struct FailingHandoffLedger;

#[async_trait]
impl HandoffEventLedger for FailingHandoffLedger {
    async fn append_handoff_event(
        &self,
        _event: NewKernelEvent,
    ) -> Result<KernelEvent, HandoffEventLedgerError> {
        Err(HandoffEventLedgerError::new("ledger unavailable"))
    }
}

fn proved_row(hbr_id: &str) -> HbrMatrixRow {
    HbrMatrixRow {
        hbr_id: hbr_id.to_string(),
        status: "PROVED".to_string(),
        evidence_pointer: Some(format!("receipts://{hbr_id}")),
        validator_verdict: Some("PROVED".to_string()),
    }
}

fn pending_row(hbr_id: &str) -> HbrMatrixRow {
    HbrMatrixRow {
        hbr_id: hbr_id.to_string(),
        status: "PENDING".to_string(),
        evidence_pointer: None,
        validator_verdict: None,
    }
}

fn packet(rows: Vec<HbrMatrixRow>) -> HbrPacket {
    HbrPacket {
        wp_id: "WP-KERNEL-004-TEST".to_string(),
        acceptance_matrix: HbrAcceptanceMatrix {
            hbr: rows,
            hbr_not_applicable: Vec::new(),
        },
    }
}

fn gate(ledger: InMemoryHandoffLedger) -> HandoffGate<InMemoryHandoffLedger> {
    HandoffGate::new(ledger, handoff_rules())
}

fn event_payload<'a>(event: &'a NewKernelEvent, transition: &str) -> &'a Value {
    assert_eq!(event.event_type, KernelEventType::HbrHandoffGate);
    assert_eq!(event.event_type.as_str(), "HBR_HANDOFF_GATE");
    assert_eq!(event.aggregate_type, "work_packet");
    assert_eq!(event.aggregate_id, "WP-KERNEL-004-TEST");
    assert_eq!(event.source_component, "hbr_handoff_gate");
    assert_eq!(event.payload["event_type"], "HBR_HANDOFF_GATE");
    assert_eq!(event.payload["wp_id"], "WP-KERNEL-004-TEST");
    assert_eq!(event.payload["transition"], transition);
    DateTime::parse_from_rfc3339(
        event.payload["timestamp_utc"]
            .as_str()
            .expect("timestamp_utc string"),
    )
    .expect("timestamp_utc is RFC3339");
    &event.payload
}

#[tokio::test]
async fn every_governed_transition_emits_typed_hbr_handoff_gate_event() {
    let cases = [
        (
            HandoffTransition::RefinementToCoder,
            "RefinementToCoder",
            "HBR-MAN-001",
        ),
        (
            HandoffTransition::CoderToWpValidator,
            "CoderToWpValidator",
            "HBR-INT-001",
        ),
        (
            HandoffTransition::WpValidatorToIntegrationValidator,
            "WpValidatorToIntegrationValidator",
            "HBR-SWARM-001",
        ),
        (
            HandoffTransition::IntegrationValidatorToOrchestrator,
            "IntegrationValidatorToOrchestrator",
            "HBR-MAN-002",
        ),
        (
            HandoffTransition::IntegrationValidatorToOrchestrator,
            "IntegrationValidatorToOrchestrator",
            "HBR-QUIET-004",
        ),
    ];

    for (transition, transition_name, hbr_id) in cases {
        let ledger = InMemoryHandoffLedger::default();
        let result = gate(ledger.clone())
            .evaluate(&packet(vec![proved_row(hbr_id)]), transition)
            .await
            .expect("proved relevant row must pass");

        assert_eq!(result.transition, transition);
        assert_eq!(result.evaluated_rules.len(), 1);
        assert_eq!(result.evaluated_rules[0].hbr_id, hbr_id);
        assert_eq!(result.gate_uuid.get_version_num(), 7);

        let events = ledger.events();
        assert_eq!(events.len(), 1);
        let payload = event_payload(&events[0], transition_name);
        assert_eq!(payload["gate_uuid"], result.gate_uuid.to_string());
        assert_eq!(payload["verdict"]["kind"], "Pass");
        assert_eq!(payload["evaluated_rules"][0]["hbr_id"], hbr_id);
        assert_eq!(
            payload["evaluated_rules"][0]["evidence_pointer"],
            format!("receipts://{hbr_id}")
        );
    }
}

#[tokio::test]
async fn pending_relevant_row_blocks_transition_and_still_emits_event() {
    let ledger = InMemoryHandoffLedger::default();

    let block = gate(ledger.clone())
        .evaluate(
            &packet(vec![pending_row("HBR-INT-001")]),
            HandoffTransition::CoderToWpValidator,
        )
        .await
        .expect_err("PENDING relevant row must block handoff");

    assert_eq!(block.transition, HandoffTransition::CoderToWpValidator);
    assert_eq!(block.failing_rules.len(), 1);
    assert_eq!(block.failing_rules[0].hbr_id, "HBR-INT-001");
    assert!(block.failing_rules[0].reason.contains("PENDING"));
    assert_eq!(
        block.event.as_ref().expect("block event").event_type,
        KernelEventType::HbrHandoffGate
    );

    let events = ledger.events();
    assert_eq!(events.len(), 1);
    let payload = event_payload(&events[0], "CoderToWpValidator");
    assert_eq!(payload["gate_uuid"], block.gate_uuid.to_string());
    assert_eq!(payload["verdict"]["kind"], "Block");
    assert_eq!(
        payload["verdict"]["failing_rules"][0]["hbr_id"],
        "HBR-INT-001"
    );
}

#[tokio::test]
async fn proved_rows_require_evidence_pointer_and_validator_proved_verdict() {
    let ledger = InMemoryHandoffLedger::default();
    let mut row = proved_row("HBR-INT-007");
    row.validator_verdict = Some("PASS".to_string());

    let block = gate(ledger)
        .evaluate(&packet(vec![row]), HandoffTransition::CoderToWpValidator)
        .await
        .expect_err("PROVED row with non-PROVED validator verdict must block");

    assert_eq!(block.failing_rules[0].hbr_id, "HBR-INT-007");
    assert!(block.failing_rules[0].reason.contains("validator_verdict"));
}

#[tokio::test]
async fn not_applicable_rows_require_matching_reason_ledger() {
    let ledger = InMemoryHandoffLedger::default();
    let row = HbrMatrixRow {
        hbr_id: "HBR-INT-001".to_string(),
        status: "NOT_APPLICABLE".to_string(),
        evidence_pointer: None,
        validator_verdict: None,
    };
    let packet = HbrPacket {
        wp_id: "WP-KERNEL-004-TEST".to_string(),
        acceptance_matrix: HbrAcceptanceMatrix {
            hbr: vec![row.clone()],
            hbr_not_applicable: Vec::new(),
        },
    };

    gate(ledger.clone())
        .evaluate(&packet, HandoffTransition::CoderToWpValidator)
        .await
        .expect_err("NOT_APPLICABLE without reason ledger must block");

    let packet = HbrPacket {
        wp_id: "WP-KERNEL-004-TEST".to_string(),
        acceptance_matrix: HbrAcceptanceMatrix {
            hbr: vec![row],
            hbr_not_applicable: vec![HbrNotApplicableRow {
                hbr_id: "HBR-INT-001".to_string(),
                reason: "EventLedger primitive not yet implemented at packet creation time."
                    .to_string(),
            }],
        },
    };

    gate(ledger)
        .evaluate(&packet, HandoffTransition::CoderToWpValidator)
        .await
        .expect("NOT_APPLICABLE with reason ledger is accepted");
}

#[tokio::test]
async fn gate_uuid_is_uuid_v7_and_time_ordered() {
    let ledger = InMemoryHandoffLedger::default();
    let first = gate(ledger.clone())
        .evaluate(
            &packet(vec![proved_row("HBR-INT-008")]),
            HandoffTransition::CoderToWpValidator,
        )
        .await
        .expect("first gate passes")
        .gate_uuid;

    std::thread::sleep(std::time::Duration::from_millis(2));

    let second = gate(ledger)
        .evaluate(
            &packet(vec![proved_row("HBR-INT-008")]),
            HandoffTransition::CoderToWpValidator,
        )
        .await
        .expect("second gate passes")
        .gate_uuid;

    assert_eq!(first.get_version_num(), 7);
    assert_eq!(second.get_version_num(), 7);
    assert!(
        first <= second,
        "sequential HBR gate UUIDs must retain v7 ordering: {first} <= {second}"
    );
    assert_ne!(Uuid::nil(), first);
    assert_ne!(Uuid::nil(), second);
}

#[tokio::test]
async fn stop_pillar_pending_row_blocks_coder_handoff() {
    // MT-004 finding #1: a STOP-pillar row in PENDING state must block the
    // CoderToWpValidator handoff. Before the fix its evidence_kind was absent
    // from the transition's checkable set, so the row was skipped and the gate
    // passed silently while the scope-discipline gate was bypassed.
    let ledger = InMemoryHandoffLedger::default();
    let block = gate(ledger)
        .evaluate(
            &packet(vec![pending_row("HBR-STOP-001")]),
            HandoffTransition::CoderToWpValidator,
        )
        .await
        .expect_err("PENDING STOP row must block coder handoff");
    assert_eq!(block.evaluated_rules.len(), 1);
    assert_eq!(block.evaluated_rules[0].hbr_id, "HBR-STOP-001");
    assert_eq!(block.failing_rules.len(), 1);
    assert_eq!(block.failing_rules[0].hbr_id, "HBR-STOP-001");
    assert!(block.failing_rules[0].reason.contains("PENDING"));
}

#[tokio::test]
async fn stop_pillar_proved_row_passes_coder_handoff() {
    let ledger = InMemoryHandoffLedger::default();
    let evidence = gate(ledger)
        .evaluate(
            &packet(vec![proved_row("HBR-STOP-001")]),
            HandoffTransition::CoderToWpValidator,
        )
        .await
        .expect("PROVED STOP row with evidence + verdict passes");
    assert_eq!(evidence.evaluated_rules.len(), 1);
    assert_eq!(evidence.evaluated_rules[0].hbr_id, "HBR-STOP-001");
}

#[tokio::test]
async fn ledger_append_failure_merges_matrix_failures_with_namespaced_infra_sentinel() {
    // MT-004 findings #2 + #3: when the EventLedger append fails AND the matrix
    // has violations, the block must report BOTH the matrix failures and a
    // namespaced INFRA- infrastructure sentinel — never an HBR- id, and never
    // dropping the matrix failures.
    let block = HandoffGate::new(FailingHandoffLedger, handoff_rules())
        .evaluate(
            &packet(vec![pending_row("HBR-INT-001")]),
            HandoffTransition::CoderToWpValidator,
        )
        .await
        .expect_err("ledger append failure must block");

    assert!(block.event.is_none(), "no event when the ledger append failed");
    let ids: Vec<&str> = block
        .failing_rules
        .iter()
        .map(|rule| rule.hbr_id.as_str())
        .collect();
    assert!(
        ids.contains(&"HBR-INT-001"),
        "matrix violation must be preserved on ledger failure: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id.starts_with("INFRA-")),
        "infrastructure sentinel must be namespaced INFRA-, not an HBR- id: {ids:?}"
    );
    assert!(
        !ids.iter().any(|id| *id == "HBR-HANDOFF-GATE-EVENT-LEDGER"),
        "the old HBR- ledger sentinel must no longer be emitted: {ids:?}"
    );
}
