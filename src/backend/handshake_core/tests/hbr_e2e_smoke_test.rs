use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use handshake_core::{
    hbr::{
        handoff_gate::{
            HandoffEventLedger, HandoffEventLedgerError, HandoffGate, HandoffRule,
            HandoffTransition, HbrAcceptanceMatrix, HbrMatrixRow, HbrPacket,
        },
        registry::HbrRegistry,
        violation::{
            EvaluationPoint, HbrViolation, HbrViolationRole, ViolationClass, ViolationSink,
        },
    },
    kernel::{KernelEvent, KernelEventType, NewKernelEvent},
    process_ledger::{
        LedgerOverflowEvent, ProcessEngineKind, ProcessLedgerError, ProcessLedgerOverflowSink,
        ProcessLedgerWriter, ProcessStart, ProcessStop, SurrealProcessLedgerStore,
    },
    storage::surreal::RowFilter,
    storage::tests::embedded_test_backend,
};

const WP_ID: &str =
    "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";
const SESSION_ID: &str = "KERNEL_BUILDER-20260518-012310";
#[tokio::test]
async fn hbr_e2e_smoke_test() -> Result<(), Box<dyn Error>> {
    let registry_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hbr/HANDSHAKE_BUILD_RULES.json");
    let registry = HbrRegistry::load_from_path(&registry_path)?;
    assert_eq!(registry.version, "1.11.0");

    let handoff_ledger = InMemoryHandoffLedger::default();
    let handoff_block = HandoffGate::new(
        handoff_ledger.clone(),
        vec![HandoffRule::new(
            "HBR-INT-001",
            "test_run_with_ledger_replay",
        )],
    )
    .evaluate(
        &HbrPacket {
            wp_id: WP_ID.to_string(),
            acceptance_matrix: HbrAcceptanceMatrix {
                hbr: vec![HbrMatrixRow {
                    hbr_id: "HBR-INT-001".to_string(),
                    status: "PENDING".to_string(),
                    evidence_pointer: None,
                    validator_verdict: None,
                }],
                hbr_not_applicable: Vec::new(),
            },
        },
        HandoffTransition::CoderToWpValidator,
    )
    .await
    .expect_err("PENDING HBR-INT-001 must block handoff");
    assert_eq!(handoff_block.failing_rules[0].hbr_id, "HBR-INT-001");
    let handoff_events = handoff_ledger.events();
    assert_eq!(handoff_events.len(), 1);
    assert_eq!(
        handoff_events[0].event_type,
        KernelEventType::HbrHandoffGate
    );
    assert_eq!(handoff_events[0].payload["verdict"]["kind"], "Block");

    let violation_sink = InMemoryViolationSink::default();
    let violation = HbrViolation::new(
        "HBR-INT-001",
        WP_ID,
        Some("MT-009"),
        HbrViolationRole::KernelBuilder,
        EvaluationPoint::Build,
        Some("test://hbr-e2e-smoke/matrix-failure"),
        ViolationClass::MissingEvidence,
        Some(SESSION_ID),
        Some("MT-009 deliberate failure-path proof"),
    );
    violation.emit(&violation_sink)?;
    let canonical_violation = violation_sink.single_row();
    assert_eq!(canonical_violation, violation.to_canonical_jsonl()?);
    let serialized: Value = serde_json::from_str(canonical_violation.trim())?;
    assert_eq!(serialized["hbr_id"], "HBR-INT-001");

    let backend = embedded_test_backend().await?;
    let store = Arc::new(SurrealProcessLedgerStore::new(backend.storage.clone()));
    let (writer, drain) =
        ProcessLedgerWriter::new_manual(8, Arc::new(InMemoryOverflowSink::default()))?;
    let start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "KERNEL_BUILDER",
        Some(WP_ID.to_string()),
    )
    .with_parent_session_id("SR-HBR-E2E-SMOKE")
    .with_sandbox_adapter_id("sandbox-adapter-hbr-e2e")
    .with_work_profile_id("work-profile-hbr-e2e");
    let stop = ProcessStop::from_start(&start, Some(0));
    writer.append_start(start.clone())?;
    writer.append_stop(stop)?;
    drain.drain_available_to(store.clone()).await?;

    let inspector = backend.storage.test_inspector();
    let table = inspector.table_selector("kernel_process_lifecycle").await?;
    let engine_kind = table.field("engine_kind")?;
    let owner_wp = table.field("owner_wp")?;
    let stopped_at = table.field("stopped_at")?;
    let rows = inspector
        .project(
            &table,
            &[engine_kind, owner_wp, stopped_at],
            RowFilter::IdEquals(start.process_uuid.to_string()),
        )
        .await?;
    assert_eq!(rows.len(), 1);
    let row = &rows[0].values;
    assert_eq!(
        row.get("engine_kind").and_then(Value::as_str),
        Some(ProcessEngineKind::HelperSubprocess.as_str())
    );
    assert_eq!(row.get("owner_wp").and_then(Value::as_str), Some(WP_ID));
    assert!(row.get("stopped_at").is_some_and(|value| !value.is_null()));

    drop(inspector);
    drop(store);
    backend.close_and_remove().await?;

    Ok(())
}

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

#[derive(Default)]
struct InMemoryViolationSink {
    rows: Mutex<Vec<String>>,
}

impl InMemoryViolationSink {
    fn single_row(&self) -> String {
        let rows = self.rows.lock().expect("violation rows lock");
        assert_eq!(rows.len(), 1);
        rows[0].clone()
    }
}

impl ViolationSink for InMemoryViolationSink {
    fn write_violation(&self, canonical_jsonl: &str) -> Result<(), std::io::Error> {
        self.rows
            .lock()
            .expect("violation rows lock")
            .push(canonical_jsonl.to_string());
        Ok(())
    }
}

#[derive(Clone, Default)]
struct InMemoryOverflowSink {
    events: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
}

impl ProcessLedgerOverflowSink for InMemoryOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.events.lock().expect("overflow lock").push(event);
        Ok(())
    }
}
