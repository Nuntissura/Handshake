use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::capabilities::CapabilityRegistry;
use crate::flight_recorder::FlightRecorder;
use crate::mex::envelope::{
    BudgetSpec, DeterminismLevel, EngineResult, EngineStatus, EvidencePolicy, OutputSpec,
    PlannedOperation, ProvenanceRecord, POE_SCHEMA_VERSION,
};
use crate::mex::gates::{
    BudgetGate, CapabilityGate, DetGate, GatePipeline, IntegrityGate, ProvenanceGate, SchemaGate,
};
use crate::mex::registry::{EngineSpec, MexRegistry};
use crate::mex::runtime::{AdapterError, EngineAdapter, MexRuntime, MexRuntimeError};
use tokio::runtime::Runtime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConformanceCase {
    SchemaValidation,
    CapabilityDenial,
    BudgetEnforcement,
    ArtifactOnlyIo,
    ProvenanceCompleteness,
    DeterminismEvidence,
}

#[derive(Debug, Clone)]
pub struct ConformanceResult {
    pub case: ConformanceCase,
    pub passed: bool,
    pub notes: Option<String>,
}

pub struct ConformanceHarness {
    runtime: MexRuntime,
}

impl ConformanceHarness {
    pub fn new(
        registry: MexRegistry,
        capability_registry: CapabilityRegistry,
        flight_recorder: Arc<dyn FlightRecorder>,
        adapter: Arc<dyn EngineAdapter>,
        engine_id: &str,
    ) -> Self {
        let gates = GatePipeline::new(vec![
            Box::new(SchemaGate),
            Box::new(CapabilityGate::new(capability_registry)),
            Box::new(IntegrityGate),
            Box::new(BudgetGate),
            Box::new(ProvenanceGate),
            Box::new(DetGate),
        ]);

        let runtime =
            MexRuntime::new(registry, flight_recorder, gates).with_adapter(engine_id, adapter);

        Self { runtime }
    }

    pub fn run(&self, engine_id: &str) -> Vec<ConformanceResult> {
        vec![
            self.case_schema(engine_id),
            self.case_capability_denial(engine_id),
            self.case_budget(engine_id),
            self.case_artifact_only(engine_id),
            self.case_provenance(engine_id),
            self.case_determinism(engine_id),
        ]
    }

    fn base_operation(&self, engine_id: &str) -> PlannedOperation {
        PlannedOperation {
            schema_version: POE_SCHEMA_VERSION.to_string(),
            op_id: Uuid::new_v4(),
            engine_id: engine_id.to_string(),
            engine_version_req: None,
            operation: "conformance.test".to_string(),
            inputs: vec![ArtifactHandle::new(
                Uuid::new_v4(),
                "/artifact/input".to_string(),
            )],
            params: serde_json::json!({"kind": "test"}),
            capabilities_requested: vec!["fs.read".to_string(), "fs.write".to_string()],
            budget: BudgetSpec {
                cpu_time_ms: Some(1000),
                wall_time_ms: Some(2000),
                memory_bytes: Some(64 * 1024 * 1024),
                output_bytes: Some(8 * 1024 * 1024),
            },
            determinism: DeterminismLevel::D2,
            evidence_policy: EvidencePolicy {
                required: true,
                notes: Some("Conformance evidence".to_string()),
            },
            output_spec: OutputSpec {
                expected_types: vec!["artifact.document".to_string()],
                max_bytes: Some(8 * 1024 * 1024),
            },
        }
    }

    fn run_case(&self, op: PlannedOperation, expected_err: Option<&str>) -> (bool, Option<String>) {
        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(err) => return (false, Some(err.to_string())),
        };
        let result = rt.block_on(self.runtime.execute(op));

        match (result, expected_err) {
            (Ok(_), None) => (true, None),
            (Ok(_), Some(note)) => (false, Some(note.to_string())),
            (Err(MexRuntimeError::Gate(denial)), Some(_)) => (true, Some(denial.reason)),
            (Err(err), _) => (false, Some(err.to_string())),
        }
    }

    fn case_schema(&self, engine_id: &str) -> ConformanceResult {
        let mut op = self.base_operation(engine_id);
        op.schema_version = "invalid".to_string();
        let (passed, notes) = self.run_case(op, Some("schema"));
        ConformanceResult {
            case: ConformanceCase::SchemaValidation,
            passed,
            notes,
        }
    }

    fn case_capability_denial(&self, engine_id: &str) -> ConformanceResult {
        let mut op = self.base_operation(engine_id);
        op.capabilities_requested.clear();
        let (passed, notes) = self.run_case(op, Some("capabilities"));
        ConformanceResult {
            case: ConformanceCase::CapabilityDenial,
            passed,
            notes,
        }
    }

    fn case_budget(&self, engine_id: &str) -> ConformanceResult {
        let mut op = self.base_operation(engine_id);
        op.budget = BudgetSpec::default();
        let (passed, notes) = self.run_case(op, Some("budget"));
        ConformanceResult {
            case: ConformanceCase::BudgetEnforcement,
            passed,
            notes,
        }
    }

    fn case_artifact_only(&self, engine_id: &str) -> ConformanceResult {
        let mut op = self.base_operation(engine_id);
        // Force inline params >32KB
        let big_payload = "x".repeat(33 * 1024);
        op.params = serde_json::json!({ "inline": big_payload });
        let (passed, notes) = self.run_case(op, Some("integrity"));
        ConformanceResult {
            case: ConformanceCase::ArtifactOnlyIo,
            passed,
            notes,
        }
    }

    fn case_provenance(&self, engine_id: &str) -> ConformanceResult {
        let mut op = self.base_operation(engine_id);
        op.determinism = DeterminismLevel::D0;
        op.evidence_policy.required = false;
        let (passed, notes) = self.run_case(op, Some("provenance"));
        ConformanceResult {
            case: ConformanceCase::ProvenanceCompleteness,
            passed,
            notes,
        }
    }

    fn case_determinism(&self, engine_id: &str) -> ConformanceResult {
        let mut op = self.base_operation(engine_id);
        op.determinism = DeterminismLevel::D3;
        let (passed, notes) = self.run_case(op, Some("determinism"));
        ConformanceResult {
            case: ConformanceCase::DeterminismEvidence,
            passed,
            notes,
        }
    }
}

/// Minimal in-memory adapter used for conformance harness.
pub struct StubEngineAdapter;

#[async_trait::async_trait]
impl EngineAdapter for StubEngineAdapter {
    async fn invoke(&self, op: &PlannedOperation) -> Result<EngineResult, AdapterError> {
        let provenance = ProvenanceRecord {
            engine_id: op.engine_id.clone(),
            engine_version: op.engine_version_req.clone(),
            implementation: Some("stub".to_string()),
            determinism: op.determinism,
            config_hash: None,
            inputs: op.inputs.clone(),
            outputs: vec![ArtifactHandle::new(
                Uuid::new_v4(),
                "/artifact/output".to_string(),
            )],
            capabilities_granted: op.capabilities_requested.clone(),
            environment: None,
        };

        Ok(EngineResult {
            op_id: op.op_id,
            status: EngineStatus::Succeeded,
            started_at: chrono::Utc::now(),
            ended_at: chrono::Utc::now(),
            outputs: provenance.outputs.clone(),
            evidence: Vec::new(),
            provenance,
            errors: Vec::new(),
            logs_ref: None,
        })
    }
}

/// Helper to load registry for tests/harness.
pub fn load_registry_for_tests(path: &Path) -> MexRegistry {
    MexRegistry::load_from_path(path).unwrap_or_else(|_| MexRegistry::from_map(HashMap::new()))
}

/// Build a single-engine registry entry for harness defaults.
pub fn single_engine_registry(engine_id: &str) -> MexRegistry {
    let spec = EngineSpec {
        engine_id: engine_id.to_string(),
        determinism_ceiling: DeterminismLevel::D2,
        required_caps: vec!["fs.read".to_string(), "fs.write".to_string()],
        required_gates: vec![
            "G-SCHEMA".to_string(),
            "G-CAP".to_string(),
            "G-INTEGRITY".to_string(),
            "G-BUDGET".to_string(),
            "G-PROVENANCE".to_string(),
            "G-DET".to_string(),
        ],
        default_budget: BudgetSpec {
            cpu_time_ms: Some(10_000),
            wall_time_ms: Some(20_000),
            memory_bytes: Some(256 * 1024 * 1024),
            output_bytes: Some(16 * 1024 * 1024),
        },
        ops: vec![],
    };

    let mut map = HashMap::new();
    map.insert(engine_id.to_string(), spec);
    MexRegistry::from_map(map)
}
