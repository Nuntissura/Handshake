use std::sync::Arc;

use handshake_core::ace::ArtifactHandle;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, DiagnosticsStore};
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEventType};
use handshake_core::mex::conformance::{
    single_engine_registry, ConformanceCase, ConformanceHarness, TestEngineAdapter,
};
use handshake_core::mex::envelope::{
    BudgetSpec, DeterminismLevel, EvidencePolicy, OutputSpec, PlannedOperation, POE_SCHEMA_VERSION,
};
use handshake_core::mex::gates::{
    BudgetGate, CapabilityGate, DetGate, GatePipeline, IntegrityGate, ProvenanceGate, SchemaGate,
};
use handshake_core::mex::runtime::{EngineAdapter, MexRuntime, MexRuntimeError};
use uuid::Uuid;

fn recorder() -> Arc<DuckDbFlightRecorder> {
    Arc::new(DuckDbFlightRecorder::new_in_memory(32).expect("flight recorder should init"))
}

#[test]
fn conformance_harness_runs_all_cases() {
    let registry = single_engine_registry("engine.spatial");
    let capability_registry = CapabilityRegistry::new();
    let adapter = Arc::new(TestEngineAdapter);
    let recorder = recorder();
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let harness = ConformanceHarness::new(
        registry,
        capability_registry,
        flight_recorder,
        diagnostics,
        adapter,
        "engine.spatial",
    );

    let results = harness.run("engine.spatial");
    assert_eq!(results.len(), 6);
    assert!(results.iter().all(|r| r.passed));
    assert!(results
        .iter()
        .any(|r| r.case == ConformanceCase::SchemaValidation));
}

struct PassThroughAdapter;

#[async_trait::async_trait]
impl EngineAdapter for PassThroughAdapter {
    async fn invoke(
        &self,
        op: &PlannedOperation,
    ) -> Result<
        handshake_core::mex::envelope::EngineResult,
        handshake_core::mex::runtime::AdapterError,
    > {
        let provenance = handshake_core::mex::envelope::ProvenanceRecord {
            engine_id: op.engine_id.clone(),
            engine_version: op.engine_version_req.clone(),
            implementation: Some("passthrough".to_string()),
            determinism: op.determinism,
            config_hash: None,
            inputs: op.inputs.clone(),
            outputs: vec![ArtifactHandle::new(Uuid::new_v4(), "/out".to_string())],
            capabilities_granted: op.capabilities_requested.clone(),
            environment: None,
        };

        Ok(handshake_core::mex::envelope::EngineResult::success(
            op.op_id,
            provenance.outputs.clone(),
            provenance,
        ))
    }
}

#[tokio::test]
async fn runtime_executes_with_gates_and_adapter() {
    let registry = single_engine_registry("engine.spatial");
    let recorder = recorder();
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let gates = GatePipeline::new(vec![
        Box::new(SchemaGate),
        Box::new(CapabilityGate::new(CapabilityRegistry::new())),
        Box::new(IntegrityGate),
        Box::new(BudgetGate),
        Box::new(ProvenanceGate),
        Box::new(DetGate),
    ]);

    let runtime = MexRuntime::new(registry, flight_recorder, diagnostics, gates)
        .with_adapter("engine.spatial", Arc::new(PassThroughAdapter));

    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id: Uuid::new_v4(),
        engine_id: "engine.spatial".to_string(),
        engine_version_req: None,
        operation: "spatial.build_model".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "/input".to_string())],
        params: serde_json::json!({"script_ref": "artifact://script"}),
        capabilities_requested: vec!["fs.read".to_string(), "fs.write".to_string()],
        budget: BudgetSpec {
            cpu_time_ms: Some(1000),
            wall_time_ms: Some(2000),
            memory_bytes: Some(64 * 1024 * 1024),
            output_bytes: Some(2 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D2,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: None,
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.model3d".to_string()],
            max_bytes: Some(2 * 1024 * 1024),
        },
    };

    let result = runtime.execute(op).await.expect("runtime should succeed");
    assert_eq!(
        result.status,
        handshake_core::mex::envelope::EngineStatus::Succeeded
    );
    assert_eq!(result.outputs.len(), 1);
}

#[tokio::test]
async fn gate_pass_logs_outcome() {
    let registry = single_engine_registry("engine.spatial");
    let recorder = recorder();
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let gates = GatePipeline::new(vec![
        Box::new(SchemaGate),
        Box::new(CapabilityGate::new(CapabilityRegistry::new())),
        Box::new(IntegrityGate),
        Box::new(BudgetGate),
        Box::new(ProvenanceGate),
        Box::new(DetGate),
    ]);

    let runtime = MexRuntime::new(registry, flight_recorder, diagnostics, gates)
        .with_adapter("engine.spatial", Arc::new(PassThroughAdapter));

    let op_id = Uuid::new_v4();
    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.spatial".to_string(),
        engine_version_req: None,
        operation: "spatial.build_model".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "/input".to_string())],
        params: serde_json::json!({"script_ref": "artifact://script"}),
        capabilities_requested: vec!["fs.read".to_string(), "fs.write".to_string()],
        budget: BudgetSpec {
            cpu_time_ms: Some(1000),
            wall_time_ms: Some(2000),
            memory_bytes: Some(64 * 1024 * 1024),
            output_bytes: Some(2 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D2,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: None,
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.model3d".to_string()],
            max_bytes: Some(2 * 1024 * 1024),
        },
    };

    let _ = runtime.execute(op).await.expect("runtime should succeed");

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");

    let has_pass = events.iter().any(|event| {
        if event.event_type != FlightRecorderEventType::System {
            return false;
        }
        let details = event.payload.get("details");
        event.payload.get("message").and_then(|v| v.as_str()) == Some("gate_outcome")
            && details
                .and_then(|v| v.get("outcome"))
                .and_then(|v| v.as_str())
                == Some("pass")
    });

    assert!(has_pass, "expected pass gate outcome event");

    for capability_id in ["fs.read", "fs.write"] {
        let has_capability = events.iter().any(|event| {
            event.event_type == FlightRecorderEventType::CapabilityAction
                && event.capability_id.as_deref() == Some(capability_id)
                && event.payload.get("capability_id").and_then(|v| v.as_str())
                    == Some(capability_id)
                && event.payload.get("outcome").and_then(|v| v.as_str()) == Some("allowed")
                && event.payload.get("action").and_then(|v| v.as_str())
                    == Some("mex.capability_check")
        });

        assert!(
            has_capability,
            "expected capability_action allowed event for {}",
            capability_id
        );
    }
}

#[tokio::test]
async fn gate_denial_records_diagnostic_and_event() {
    let registry = single_engine_registry("engine.spatial");
    let recorder = recorder();
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let gates = GatePipeline::new(vec![
        Box::new(SchemaGate),
        Box::new(CapabilityGate::new(CapabilityRegistry::new())),
        Box::new(IntegrityGate),
        Box::new(BudgetGate),
        Box::new(ProvenanceGate),
        Box::new(DetGate),
    ]);

    let runtime = MexRuntime::new(registry, flight_recorder, diagnostics, gates)
        .with_adapter("engine.spatial", Arc::new(PassThroughAdapter));

    let op_id = Uuid::new_v4();
    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.spatial".to_string(),
        engine_version_req: None,
        operation: "spatial.build_model".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "/input".to_string())],
        params: serde_json::json!({"script_ref": "artifact://script"}),
        capabilities_requested: vec!["magic.wand".to_string()],
        budget: BudgetSpec {
            cpu_time_ms: Some(1000),
            wall_time_ms: Some(2000),
            memory_bytes: Some(64 * 1024 * 1024),
            output_bytes: Some(2 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D2,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: None,
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.model3d".to_string()],
            max_bytes: Some(2 * 1024 * 1024),
        },
    };

    let result = runtime.execute(op).await;
    let denial = match result {
        Err(handshake_core::mex::runtime::MexRuntimeError::Gate(denial)) => denial,
        other => panic!("expected gate denial, got {other:?}"),
    };

    assert_eq!(denial.code.as_deref(), Some("HSK-4001"));

    let diagnostics = recorder
        .list_diagnostics(DiagFilter::default())
        .await
        .expect("diagnostics should be queryable");
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.job_id.as_deref() == Some(&op_id.to_string())),
        "expected diagnostic tied to op_id"
    );

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");

    let has_diagnostic_event = events.iter().any(|event| {
        event.event_type == FlightRecorderEventType::Diagnostic
            && event.payload.get("diagnostic_id").is_some()
    });
    assert!(has_diagnostic_event, "expected FR-EVT-003 diagnostic event");

    let has_deny = events.iter().any(|event| {
        if event.event_type != FlightRecorderEventType::System {
            return false;
        }
        let details = event.payload.get("details");
        event.payload.get("message").and_then(|v| v.as_str()) == Some("gate_outcome")
            && details
                .and_then(|v| v.get("outcome"))
                .and_then(|v| v.as_str())
                == Some("deny")
    });
    assert!(has_deny, "expected deny gate outcome event");

    let has_denied_capability = events.iter().any(|event| {
        event.event_type == FlightRecorderEventType::CapabilityAction
            && event.capability_id.as_deref() == Some("magic.wand")
            && event.payload.get("capability_id").and_then(|v| v.as_str()) == Some("magic.wand")
            && event.payload.get("outcome").and_then(|v| v.as_str()) == Some("denied")
            && event.payload.get("action").and_then(|v| v.as_str()) == Some("mex.capability_check")
    });
    assert!(
        has_denied_capability,
        "expected capability_action denied event for magic.wand"
    );
}

#[tokio::test]
async fn d0_missing_evidence_records_diagnostic() {
    let registry = single_engine_registry("engine.spatial");
    let recorder = recorder();
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();
    let gates = GatePipeline::new(vec![
        Box::new(SchemaGate),
        Box::new(CapabilityGate::new(CapabilityRegistry::new())),
        Box::new(IntegrityGate),
        Box::new(BudgetGate),
        Box::new(ProvenanceGate),
        Box::new(DetGate),
    ]);

    let runtime = MexRuntime::new(registry, flight_recorder, diagnostics, gates)
        .with_adapter("engine.spatial", Arc::new(PassThroughAdapter));

    let op_id = Uuid::new_v4();
    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.spatial".to_string(),
        engine_version_req: None,
        operation: "spatial.build_model".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "/input".to_string())],
        params: serde_json::json!({"script_ref": "artifact://script"}),
        capabilities_requested: vec!["fs.read".to_string(), "fs.write".to_string()],
        budget: BudgetSpec {
            cpu_time_ms: Some(1000),
            wall_time_ms: Some(2000),
            memory_bytes: Some(64 * 1024 * 1024),
            output_bytes: Some(2 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D0,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: None,
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.model3d".to_string()],
            max_bytes: Some(2 * 1024 * 1024),
        },
    };

    let result = runtime.execute(op).await;
    assert!(matches!(
        result,
        Err(MexRuntimeError::EvidenceMissing(DeterminismLevel::D0))
    ));

    let diagnostics = recorder
        .list_diagnostics(DiagFilter::default())
        .await
        .expect("diagnostics should be queryable");
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.job_id.as_deref() == Some(&op_id.to_string())),
        "expected diagnostic tied to op_id"
    );

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");

    let has_diagnostic_event = events.iter().any(|event| {
        event.event_type == FlightRecorderEventType::Diagnostic
            && event.payload.get("diagnostic_id").is_some()
    });
    assert!(has_diagnostic_event, "expected FR-EVT-003 diagnostic event");
}
