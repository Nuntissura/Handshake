use std::sync::Arc;

use handshake_core::ace::ArtifactHandle;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::mex::conformance::{
    single_engine_registry, ConformanceCase, ConformanceHarness, TestEngineAdapter,
};
use handshake_core::mex::envelope::{
    BudgetSpec, DeterminismLevel, EvidencePolicy, OutputSpec, PlannedOperation, POE_SCHEMA_VERSION,
};
use handshake_core::mex::gates::{
    BudgetGate, CapabilityGate, DetGate, GatePipeline, IntegrityGate, ProvenanceGate, SchemaGate,
};
use handshake_core::mex::runtime::EngineAdapter;
use handshake_core::mex::runtime::MexRuntime;
use uuid::Uuid;

fn recorder() -> Arc<dyn handshake_core::flight_recorder::FlightRecorder> {
    Arc::new(DuckDbFlightRecorder::new_in_memory(32).expect("flight recorder should init"))
}

#[test]
fn conformance_harness_runs_all_cases() {
    let registry = single_engine_registry("engine.spatial");
    let capability_registry = CapabilityRegistry::new();
    let adapter = Arc::new(TestEngineAdapter);
    let harness = ConformanceHarness::new(
        registry,
        capability_registry,
        recorder(),
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
    let gates = GatePipeline::new(vec![
        Box::new(SchemaGate),
        Box::new(CapabilityGate::new(CapabilityRegistry::new())),
        Box::new(IntegrityGate),
        Box::new(BudgetGate),
        Box::new(ProvenanceGate),
        Box::new(DetGate),
    ]);

    let runtime = MexRuntime::new(registry, recorder(), gates)
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
        evidence_policy: EvidencePolicy {
            required: true,
            notes: None,
        },
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
