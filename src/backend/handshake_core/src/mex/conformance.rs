use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::capabilities::CapabilityRegistry;
use crate::diagnostics::DiagnosticsStore;
use crate::flight_recorder::FlightRecorder;
use crate::mex::envelope::{
    BudgetSpec, DeterminismLevel, EngineResult, EngineStatus, EvidencePolicy, OutputSpec,
    PlannedOperation, ProvenanceRecord, POE_SCHEMA_VERSION,
};
use crate::mex::gates::{
    BudgetGate, CapabilityGate, DetGate, GatePipeline, IntegrityGate, ProvenanceGate, SchemaGate,
};
use crate::mex::registry::{EngineSpec, MexRegistry, OperationSpec};
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
        diagnostics: Arc<dyn DiagnosticsStore>,
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

        let runtime = MexRuntime::new(registry, flight_recorder, diagnostics, gates)
            .with_adapter(engine_id, adapter);

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
            capability_profile_id: Some("Coder".to_string()),
            human_consent_obtained: false,
            budget: BudgetSpec {
                cpu_time_ms: Some(1000),
                wall_time_ms: Some(2000),
                memory_bytes: Some(64 * 1024 * 1024),
                output_bytes: Some(8 * 1024 * 1024),
            },
            determinism: DeterminismLevel::D2,
            evidence_policy: Some(EvidencePolicy {
                required: true,
                notes: Some("Conformance evidence".to_string()),
            }),
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
        op.evidence_policy = None;
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
pub struct TestEngineAdapter;

#[async_trait::async_trait]
impl EngineAdapter for TestEngineAdapter {
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
        ops: vec![
            OperationSpec {
                name: "conformance.test".to_string(),
                schema_ref: Some(POE_SCHEMA_VERSION.to_string()),
                params_schema: None,
                capabilities: vec!["fs.read".to_string(), "fs.write".to_string()],
                output_types: vec!["artifact.document".to_string()],
            },
            OperationSpec {
                name: "spatial.build_model".to_string(),
                schema_ref: Some(POE_SCHEMA_VERSION.to_string()),
                params_schema: None,
                capabilities: vec!["fs.read".to_string(), "fs.write".to_string()],
                output_types: vec!["artifact.model3d".to_string()],
            },
        ],
    };

    let mut map = HashMap::new();
    map.insert(engine_id.to_string(), spec);
    MexRegistry::from_map(map)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use serde_json::Value;
    use uuid::Uuid;

    use super::*;
    use crate::capabilities::CapabilityRegistry;
    use crate::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEventType};
    use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
    use crate::mex::gates::{
        BudgetGate, CapabilityGate, DetGate, GatePipeline, IntegrityGate, ProvenanceGate,
        SchemaGate,
    };

    fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(path);
            }
        }
        Ok(())
    }

    #[test]
    fn test_no_shadow_engine_adapter_invoke_call_sites() -> Result<(), Box<dyn std::error::Error>> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let src_root = manifest_dir.join("src");
        let allowed = src_root.join("mex").join("runtime.rs");

        let mut files = Vec::new();
        collect_rs_files(&src_root, &mut files)?;

        let mut offenders = Vec::new();
        let needle = format!(".invoke({})", "&op");
        for file in files {
            let content = fs::read_to_string(&file).unwrap_or_default();
            if content.contains(&needle) && file != allowed {
                offenders.push(
                    file.strip_prefix(&manifest_dir)
                        .unwrap_or(&file)
                        .display()
                        .to_string(),
                );
            }
        }

        if !offenders.is_empty() {
            let mut msg =
                "Shadow EngineAdapter invoke call sites detected outside mex/runtime.rs: "
                    .to_string();
            msg.push_str(&offenders.join(", "));
            return Err(msg.into());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_mex_runtime_emits_tool_call_and_result_in_fr_events(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::time::{timeout, Duration};

        let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
        let registry = single_engine_registry("test_engine");
        let capability_registry = CapabilityRegistry::new();
        let gates = GatePipeline::new(vec![
            Box::new(SchemaGate),
            Box::new(CapabilityGate::new(capability_registry)),
            Box::new(IntegrityGate),
            Box::new(BudgetGate),
            Box::new(ProvenanceGate),
            Box::new(DetGate),
        ]);

        let runtime = MexRuntime::new(registry, recorder.clone(), recorder.clone(), gates)
            .with_adapter("test_engine", Arc::new(TestEngineAdapter));

        let op = PlannedOperation {
            schema_version: POE_SCHEMA_VERSION.to_string(),
            op_id: Uuid::new_v4(),
            engine_id: "test_engine".to_string(),
            engine_version_req: None,
            operation: "conformance.test".to_string(),
            inputs: vec![ArtifactHandle::new(
                Uuid::new_v4(),
                "/artifact/input".to_string(),
            )],
            params: serde_json::json!({"kind": "test"}),
            capabilities_requested: vec!["fs.read".to_string(), "fs.write".to_string()],
            capability_profile_id: Some("Coder".to_string()),
            human_consent_obtained: false,
            budget: BudgetSpec {
                cpu_time_ms: Some(1000),
                wall_time_ms: Some(2000),
                memory_bytes: Some(64 * 1024 * 1024),
                output_bytes: Some(8 * 1024 * 1024),
            },
            determinism: DeterminismLevel::D2,
            evidence_policy: None,
            output_spec: OutputSpec {
                expected_types: vec!["artifact.document".to_string()],
                max_bytes: Some(8 * 1024 * 1024),
             },
         };

        timeout(Duration::from_secs(10), runtime.execute(op.clone()))
            .await
            .map_err(|_| "mex runtime.execute timed out")??;

        let job_id = op.op_id.to_string();
        let (kinds, payloads) = {
            let conn_handle = recorder.connection();
            let conn = match conn_handle.lock() {
                Ok(conn) => conn,
                Err(poisoned) => poisoned.into_inner(),
            };
            let mut stmt = conn.prepare(
                "SELECT event_kind, job_id, payload FROM fr_events WHERE job_id = ? ORDER BY event_id ASC",
            )?;
            let rows = stmt.query_map(duckdb::params![job_id.clone()], |row| {
                let event_kind: String = row.get(0)?;
                let job_id: Option<String> = row.get(1)?;
                let payload: Option<String> = row.get(2)?;
                Ok((event_kind, job_id, payload))
            })?;

            let mut kinds = Vec::new();
            let mut payloads = Vec::new();
            for row in rows {
                let (kind, jid, payload_str) = row?;
                assert_eq!(jid.as_deref(), Some(job_id.as_str()));
                kinds.push(kind);
                if let Some(payload_str) = payload_str {
                    payloads.push(
                        serde_json::from_str::<Value>(&payload_str).unwrap_or(Value::Null),
                    );
                }
            }

            (kinds, payloads)
        };

        assert!(
            kinds.iter().any(|k| k == "tool.call"),
            "expected tool.call in fr_events, got: {:?}",
            kinds
        );
        assert!(
            kinds.iter().any(|k| k == "tool.result"),
            "expected tool.result in fr_events, got: {:?}",
            kinds
        );

        let required = [
            "tool_name",
            "tool_version",
            "inputs",
            "outputs",
            "status",
            "duration_ms",
            "error_code",
            "job_id",
            "workflow_run_id",
            "trace_id",
            "capability_id",
        ];
        for payload in payloads {
            for key in required {
                assert!(
                    payload.get(key).is_some(),
                    "missing required payload key: {} (payload={})",
                    key,
                    payload
                );
            }
        }

        let events = recorder
            .list_events(EventFilter {
                trace_id: Some(op.op_id),
                ..Default::default()
            })
            .await?;
        let tool_call = events
            .iter()
            .find(|evt| matches!(evt.event_type, FlightRecorderEventType::ToolCall))
            .expect("expected ToolCall event in Flight Recorder");
        assert_eq!(tool_call.payload.get("type").and_then(|v| v.as_str()), Some("tool_call"));
        assert_eq!(
            tool_call.payload.get("transport").and_then(|v| v.as_str()),
            Some("mex")
        );
        assert_eq!(
            tool_call.payload.get("trace_id").and_then(|v| v.as_str()),
            Some(job_id.as_str())
        );
        assert_eq!(
            tool_call.payload.get("tool_call_id").and_then(|v| v.as_str()),
            Some(job_id.as_str())
        );
        let args_ref = tool_call.payload.get("args_ref").and_then(|v| v.as_str()).unwrap_or("");
        assert!(args_ref.starts_with("artifact:"), "expected args_ref artifact handle");
        let args_hash = tool_call
            .payload
            .get("args_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(args_hash.len(), 64, "expected sha256 args_hash");
        let result_ref = tool_call
            .payload
            .get("result_ref")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(result_ref.starts_with("artifact:"), "expected result_ref artifact handle");
        let result_hash = tool_call
            .payload
            .get("result_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(result_hash.len(), 64, "expected sha256 result_hash");

        Ok(())
    }
}
