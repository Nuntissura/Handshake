use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use handshake_core::ace::ArtifactHandle;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, DiagnosticsStore};
use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use handshake_core::mex::conformance::{
    single_engine_registry, ConformanceCase, ConformanceHarness, TestEngineAdapter,
};
use handshake_core::mex::envelope::{
    BudgetSpec, DeterminismLevel, EvidencePolicy, OutputSpec, PlannedOperation, POE_SCHEMA_VERSION,
};
use handshake_core::mex::gates::{
    BudgetGate, CapabilityGate, DetGate, GatePipeline, IntegrityGate, ProvenanceGate, SchemaGate,
};
use handshake_core::mex::registry::MexRegistry;
use handshake_core::mex::runtime::{EngineAdapter, MexRuntime, MexRuntimeError};
use handshake_core::mex::supply_chain::ToolRunner;
use handshake_core::mex::{SupplyChainAllowlists, SupplyChainEngineAdapter, TerminalServiceRunner};
use handshake_core::terminal::config::TerminalConfig;
use handshake_core::terminal::redaction::PatternRedactor;
use handshake_core::terminal::{TerminalRequest, TerminalResult};
use tempfile::tempdir;
use uuid::Uuid;

fn recorder() -> Arc<DuckDbFlightRecorder> {
    Arc::new(DuckDbFlightRecorder::new_in_memory(32).expect("flight recorder should init"))
}

struct FakeToolRunner {
    flight_recorder: Arc<dyn FlightRecorder>,
    gitleaks_report: serde_json::Value,
    osv_report: serde_json::Value,
    scancode_report: serde_json::Value,
    sbom_bytes: Vec<u8>,
}

impl FakeToolRunner {
    fn new(
        flight_recorder: Arc<dyn FlightRecorder>,
        gitleaks_report: serde_json::Value,
        osv_report: serde_json::Value,
        scancode_report: serde_json::Value,
        sbom_bytes: Vec<u8>,
    ) -> Self {
        Self {
            flight_recorder,
            gitleaks_report,
            osv_report,
            scancode_report,
            sbom_bytes,
        }
    }

    fn arg_after(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|idx| args.get(idx + 1))
            .cloned()
    }

    fn write_bytes(path: &str, bytes: &[u8]) -> Result<(), String> {
        let path = std::path::Path::new(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    async fn record_terminal_event(
        &self,
        req: &TerminalRequest,
        trace_id: Uuid,
    ) -> Result<(), String> {
        let command_line = if req.args.is_empty() {
            req.command.clone()
        } else {
            format!("{} {}", req.command, req.args.join(" "))
        };

        let payload = serde_json::json!({
            "type": "terminal_command",
            "command": command_line,
            "session_id": trace_id.to_string(),
            "cwd": req.cwd.as_ref().map(|p| p.to_string_lossy().to_string()),
            "exit_code": 0,
            "duration_ms": 1,
            "timed_out": false,
            "cancelled": false,
            "truncated_bytes": 0,
        });

        let mut event = FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            trace_id,
            payload,
        );

        if let Some(job_id) = &req.job_context.job_id {
            event = event.with_job_id(job_id.clone());
        }

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|e| e.to_string())
    }
}

#[async_trait::async_trait]
impl ToolRunner for FakeToolRunner {
    async fn run(&self, req: TerminalRequest, trace_id: Uuid) -> Result<TerminalResult, String> {
        self.record_terminal_event(&req, trace_id).await?;

        match req.command.as_str() {
            "gitleaks" => {
                if req.args.first().map(|a| a.as_str()) == Some("detect") {
                    let report_path = Self::arg_after(&req.args, "--report-path")
                        .ok_or_else(|| "missing --report-path".to_string())?;
                    let bytes =
                        serde_json::to_vec(&self.gitleaks_report).map_err(|e| e.to_string())?;
                    Self::write_bytes(&report_path, &bytes)?;
                }
                Ok(TerminalResult {
                    stdout: "gitleaks 9.9.9".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                    timed_out: false,
                    cancelled: false,
                    truncated_bytes: 0,
                    duration_ms: 1,
                })
            }
            "osv-scanner" => {
                let output_path = Self::arg_after(&req.args, "--output")
                    .ok_or_else(|| "missing --output".to_string())?;
                let bytes = serde_json::to_vec(&self.osv_report).map_err(|e| e.to_string())?;
                Self::write_bytes(&output_path, &bytes)?;
                Ok(TerminalResult {
                    stdout: "osv-scanner 9.9.9".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                    timed_out: false,
                    cancelled: false,
                    truncated_bytes: 0,
                    duration_ms: 1,
                })
            }
            "syft" => {
                let maybe_arg = req
                    .args
                    .iter()
                    .find(|a| a.starts_with("cyclonedx-json="))
                    .cloned();
                if let Some(arg) = maybe_arg {
                    let path = arg
                        .split_once('=')
                        .map(|(_, p)| p.to_string())
                        .ok_or_else(|| "invalid syft output arg".to_string())?;
                    Self::write_bytes(&path, &self.sbom_bytes)?;
                }
                Ok(TerminalResult {
                    stdout: "syft 9.9.9".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                    timed_out: false,
                    cancelled: false,
                    truncated_bytes: 0,
                    duration_ms: 1,
                })
            }
            "scancode" => {
                let output_path = Self::arg_after(&req.args, "--json-pp")
                    .ok_or_else(|| "missing --json-pp".to_string())?;
                let bytes = serde_json::to_vec(&self.scancode_report).map_err(|e| e.to_string())?;
                Self::write_bytes(&output_path, &bytes)?;
                Ok(TerminalResult {
                    stdout: "scancode 9.9.9".to_string(),
                    stderr: String::new(),
                    exit_code: 0,
                    timed_out: false,
                    cancelled: false,
                    truncated_bytes: 0,
                    duration_ms: 1,
                })
            }
            other => Err(format!("unexpected command: {}", other)),
        }
    }
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
                && event
                    .payload
                    .get("decision_outcome")
                    .and_then(|v| v.as_str())
                    == Some("allow")
                && event.payload.get("actor_id").and_then(|v| v.as_str()) == Some("mex_runtime")
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
            && event
                .payload
                .get("decision_outcome")
                .and_then(|v| v.as_str())
                == Some("deny")
            && event.payload.get("actor_id").and_then(|v| v.as_str()) == Some("mex_runtime")
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

#[test]
fn registry_includes_supply_chain_engines() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mechanical_engines.json");
    let registry = MexRegistry::load_from_path(&path).expect("mechanical_engines.json should load");

    for engine_id in [
        "engine.guard.secret_scan",
        "engine.supply_chain.vuln",
        "engine.supply_chain.sbom",
        "engine.supply_chain.license",
    ] {
        assert!(
            registry.get_engine(engine_id).is_some(),
            "expected engine {engine_id} to exist in registry"
        );
    }
}

fn mex_registry_from_disk() -> MexRegistry {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mechanical_engines.json");
    MexRegistry::load_from_path(&path).expect("mechanical_engines.json should load")
}

fn mex_gates() -> GatePipeline {
    GatePipeline::new(vec![
        Box::new(SchemaGate),
        Box::new(CapabilityGate::new(CapabilityRegistry::new())),
        Box::new(IntegrityGate),
        Box::new(BudgetGate),
        Box::new(ProvenanceGate),
        Box::new(DetGate),
    ])
}

fn build_runtime_with_supply_chain_adapter(
    registry: MexRegistry,
    recorder: Arc<DuckDbFlightRecorder>,
    tool_runner: Arc<dyn ToolRunner>,
    artifact_root: PathBuf,
) -> MexRuntime {
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let diagnostics: Arc<dyn DiagnosticsStore> = recorder.clone();

    let adapter = SupplyChainEngineAdapter::new(
        tool_runner,
        flight_recorder.clone(),
        diagnostics.clone(),
        artifact_root,
        SupplyChainAllowlists::default(),
    );
    let adapter: Arc<dyn EngineAdapter> = Arc::new(adapter);

    MexRuntime::new(registry, flight_recorder, diagnostics, mex_gates())
        .with_adapter("engine.guard.secret_scan", adapter.clone())
        .with_adapter("engine.supply_chain.vuln", adapter.clone())
        .with_adapter("engine.supply_chain.sbom", adapter.clone())
        .with_adapter("engine.supply_chain.license", adapter)
}

#[tokio::test]
async fn supply_chain_vuln_release_mode_high_records_diagnostic_and_events() {
    let temp = tempdir().expect("tempdir should create");
    let recorder = recorder();

    let osv_report = serde_json::json!({
        "results": [{
            "vulnerabilities": [{
                "id": "CVE-2020-1234",
                "severity": [{"type": "CVSS_V3", "score": "9.0"}]
            }]
        }]
    });

    let tool_runner: Arc<dyn ToolRunner> = Arc::new(FakeToolRunner::new(
        recorder.clone(),
        serde_json::json!([]),
        osv_report,
        serde_json::json!({"files": []}),
        br#"{"bomFormat":"CycloneDX"}"#.to_vec(),
    ));

    let runtime = build_runtime_with_supply_chain_adapter(
        mex_registry_from_disk(),
        recorder.clone(),
        tool_runner,
        temp.path().to_path_buf(),
    );

    let op_id = Uuid::new_v4();
    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.supply_chain.vuln".to_string(),
        engine_version_req: None,
        operation: "vuln_scan".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "repo:.".to_string())],
        params: serde_json::json!({"release_mode": true}),
        capabilities_requested: vec![
            "fs.read:inputs".to_string(),
            "fs.write:artifacts".to_string(),
            "proc.exec:osv-scanner".to_string(),
        ],
        budget: BudgetSpec {
            cpu_time_ms: Some(1000),
            wall_time_ms: Some(2000),
            memory_bytes: Some(64 * 1024 * 1024),
            output_bytes: Some(2 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D1,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: None,
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.dataset".to_string()],
            max_bytes: Some(2 * 1024 * 1024),
        },
    };

    let result = runtime.execute(op).await.expect("runtime should return");
    assert_eq!(
        result.status,
        handshake_core::mex::envelope::EngineStatus::Failed
    );

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");

    assert!(
        events
            .iter()
            .any(|e| e.event_type == FlightRecorderEventType::TerminalCommand),
        "expected FR-EVT-001 terminal_command event"
    );
    assert!(
        events.iter().any(|e| {
            e.event_type == FlightRecorderEventType::System
                && e.payload.get("message").and_then(|v| v.as_str()) == Some("supply_chain_op")
        }),
        "expected supply-chain system event"
    );
    assert!(
        events.iter().any(|e| {
            e.event_type == FlightRecorderEventType::Diagnostic
                && e.payload.get("diagnostic_id").is_some()
        }),
        "expected FR-EVT-003 diagnostic event"
    );
}

#[tokio::test]
async fn supply_chain_license_release_mode_unknown_records_diagnostic() {
    let temp = tempdir().expect("tempdir should create");
    let recorder = recorder();

    let scancode_report = serde_json::json!({
        "files": [{
            "path": "third_party/foo/LICENSE",
            "licenses": [{"spdx_license_key": "unknown"}]
        }]
    });

    let tool_runner: Arc<dyn ToolRunner> = Arc::new(FakeToolRunner::new(
        recorder.clone(),
        serde_json::json!([]),
        serde_json::json!({"results": []}),
        scancode_report,
        br#"{"bomFormat":"CycloneDX"}"#.to_vec(),
    ));

    let runtime = build_runtime_with_supply_chain_adapter(
        mex_registry_from_disk(),
        recorder.clone(),
        tool_runner,
        temp.path().to_path_buf(),
    );

    let op_id = Uuid::new_v4();
    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.supply_chain.license".to_string(),
        engine_version_req: None,
        operation: "license_scan".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "repo:.".to_string())],
        params: serde_json::json!({"release_mode": true}),
        capabilities_requested: vec![
            "fs.read:inputs".to_string(),
            "fs.write:artifacts".to_string(),
            "proc.exec:scancode".to_string(),
        ],
        budget: BudgetSpec {
            cpu_time_ms: Some(1000),
            wall_time_ms: Some(2000),
            memory_bytes: Some(64 * 1024 * 1024),
            output_bytes: Some(2 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D1,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: None,
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.dataset".to_string()],
            max_bytes: Some(2 * 1024 * 1024),
        },
    };

    let result = runtime.execute(op).await.expect("runtime should return");
    assert_eq!(
        result.status,
        handshake_core::mex::envelope::EngineStatus::Failed
    );

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");

    assert!(
        events.iter().any(|e| {
            e.event_type == FlightRecorderEventType::Diagnostic
                && e.payload.get("diagnostic_id").is_some()
        }),
        "expected FR-EVT-003 diagnostic event"
    );
}

#[tokio::test]
async fn secret_scan_no_findings_emits_terminal_command_event() {
    let temp = tempdir().expect("tempdir should create");
    let recorder = recorder();

    let tool_runner: Arc<dyn ToolRunner> = Arc::new(FakeToolRunner::new(
        recorder.clone(),
        serde_json::json!([]),
        serde_json::json!({"results": []}),
        serde_json::json!({"files": []}),
        br#"{"bomFormat":"CycloneDX"}"#.to_vec(),
    ));

    let runtime = build_runtime_with_supply_chain_adapter(
        mex_registry_from_disk(),
        recorder.clone(),
        tool_runner,
        temp.path().to_path_buf(),
    );

    let op_id = Uuid::new_v4();
    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.guard.secret_scan".to_string(),
        engine_version_req: None,
        operation: "secret_scan".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "repo:.".to_string())],
        params: serde_json::json!({"release_mode": false}),
        capabilities_requested: vec![
            "fs.read:inputs".to_string(),
            "fs.write:artifacts".to_string(),
            "proc.exec:gitleaks".to_string(),
        ],
        budget: BudgetSpec {
            cpu_time_ms: Some(1000),
            wall_time_ms: Some(2000),
            memory_bytes: Some(64 * 1024 * 1024),
            output_bytes: Some(2 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D1,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: None,
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.dataset".to_string()],
            max_bytes: Some(2 * 1024 * 1024),
        },
    };

    let result = runtime.execute(op).await.expect("runtime should return");
    assert_eq!(
        result.status,
        handshake_core::mex::envelope::EngineStatus::Succeeded
    );

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");

    assert!(
        events
            .iter()
            .any(|e| e.event_type == FlightRecorderEventType::TerminalCommand),
        "expected FR-EVT-001 terminal_command event"
    );
    assert!(
        events.iter().any(|e| {
            e.event_type == FlightRecorderEventType::System
                && e.payload.get("message").and_then(|v| v.as_str()) == Some("supply_chain_op")
        }),
        "expected supply-chain system event"
    );
    assert!(
        !events
            .iter()
            .any(|e| e.event_type == FlightRecorderEventType::Diagnostic),
        "did not expect diagnostic event for clean secret scan"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root should resolve")
}

fn env_release_mode() -> bool {
    std::env::var("HSK_RELEASE_MODE")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE"))
        .unwrap_or(false)
}

fn on_disk_recorder(db_path: &std::path::Path) -> Arc<DuckDbFlightRecorder> {
    Arc::new(
        DuckDbFlightRecorder::new_on_path(db_path, 32).expect("flight recorder db should init"),
    )
}

#[tokio::test]
#[ignore]
async fn ci_secret_scan_runs_via_terminal_service() {
    let root = repo_root();
    let op_id = Uuid::new_v4();
    let op_dir = root
        .join("data/mex_supply_chain/secret_scan")
        .join(op_id.to_string());
    std::fs::create_dir_all(&op_dir).expect("op_dir should create");

    let recorder = on_disk_recorder(&op_dir.join("flight_recorder.db"));
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let redactor: Arc<dyn handshake_core::terminal::redaction::SecretRedactor> =
        Arc::new(PatternRedactor);

    let tool_runner: Arc<dyn ToolRunner> = Arc::new(TerminalServiceRunner::new(
        TerminalConfig::new(root.clone()),
        CapabilityRegistry::new(),
        flight_recorder,
        redactor,
        Vec::new(),
    ));

    let runtime = build_runtime_with_supply_chain_adapter(
        mex_registry_from_disk(),
        recorder.clone(),
        tool_runner,
        root.clone(),
    );

    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.guard.secret_scan".to_string(),
        engine_version_req: None,
        operation: "secret_scan".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "repo:.".to_string())],
        params: serde_json::json!({ "release_mode": env_release_mode() }),
        capabilities_requested: vec![
            "fs.read:inputs".to_string(),
            "fs.write:artifacts".to_string(),
            "proc.exec:gitleaks".to_string(),
        ],
        budget: BudgetSpec {
            cpu_time_ms: Some(120000),
            wall_time_ms: Some(300000),
            memory_bytes: Some(512 * 1024 * 1024),
            output_bytes: Some(50 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D1,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: Some("CI supply-chain gate".to_string()),
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.dataset".to_string()],
            max_bytes: Some(50 * 1024 * 1024),
        },
    };

    let result = runtime.execute(op).await.expect("runtime should return");
    assert_eq!(
        result.status,
        handshake_core::mex::envelope::EngineStatus::Succeeded
    );

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");
    assert!(
        events
            .iter()
            .any(|e| e.event_type == FlightRecorderEventType::TerminalCommand),
        "expected FR-EVT-001 terminal_command event"
    );
}

#[tokio::test]
#[ignore]
async fn ci_vuln_scan_runs_via_terminal_service() {
    let root = repo_root();
    let op_id = Uuid::new_v4();
    let op_dir = root
        .join("data/mex_supply_chain/vuln_scan")
        .join(op_id.to_string());
    std::fs::create_dir_all(&op_dir).expect("op_dir should create");

    let recorder = on_disk_recorder(&op_dir.join("flight_recorder.db"));
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let redactor: Arc<dyn handshake_core::terminal::redaction::SecretRedactor> =
        Arc::new(PatternRedactor);

    let tool_runner: Arc<dyn ToolRunner> = Arc::new(TerminalServiceRunner::new(
        TerminalConfig::new(root.clone()),
        CapabilityRegistry::new(),
        flight_recorder,
        redactor,
        Vec::new(),
    ));

    let runtime = build_runtime_with_supply_chain_adapter(
        mex_registry_from_disk(),
        recorder.clone(),
        tool_runner,
        root.clone(),
    );

    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.supply_chain.vuln".to_string(),
        engine_version_req: None,
        operation: "vuln_scan".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "repo:.".to_string())],
        params: serde_json::json!({ "release_mode": env_release_mode() }),
        capabilities_requested: vec![
            "fs.read:inputs".to_string(),
            "fs.write:artifacts".to_string(),
            "proc.exec:osv-scanner".to_string(),
        ],
        budget: BudgetSpec {
            cpu_time_ms: Some(300000),
            wall_time_ms: Some(600000),
            memory_bytes: Some(1024 * 1024 * 1024),
            output_bytes: Some(50 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D1,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: Some("CI supply-chain gate".to_string()),
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.dataset".to_string()],
            max_bytes: Some(50 * 1024 * 1024),
        },
    };

    let _ = runtime.execute(op).await.expect("runtime should return");

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");
    assert!(
        events
            .iter()
            .any(|e| e.event_type == FlightRecorderEventType::TerminalCommand),
        "expected FR-EVT-001 terminal_command event"
    );
}

#[tokio::test]
#[ignore]
async fn ci_sbom_generate_runs_via_terminal_service() {
    let root = repo_root();
    let op_id = Uuid::new_v4();
    let op_dir = root
        .join("data/mex_supply_chain/sbom_generate")
        .join(op_id.to_string());
    std::fs::create_dir_all(&op_dir).expect("op_dir should create");

    let recorder = on_disk_recorder(&op_dir.join("flight_recorder.db"));
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let redactor: Arc<dyn handshake_core::terminal::redaction::SecretRedactor> =
        Arc::new(PatternRedactor);

    let tool_runner: Arc<dyn ToolRunner> = Arc::new(TerminalServiceRunner::new(
        TerminalConfig::new(root.clone()),
        CapabilityRegistry::new(),
        flight_recorder,
        redactor,
        Vec::new(),
    ));

    let runtime = build_runtime_with_supply_chain_adapter(
        mex_registry_from_disk(),
        recorder.clone(),
        tool_runner,
        root.clone(),
    );

    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.supply_chain.sbom".to_string(),
        engine_version_req: None,
        operation: "sbom_generate".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "repo:.".to_string())],
        params: serde_json::json!({ "release_mode": env_release_mode() }),
        capabilities_requested: vec![
            "fs.read:inputs".to_string(),
            "fs.write:artifacts".to_string(),
            "proc.exec:syft".to_string(),
        ],
        budget: BudgetSpec {
            cpu_time_ms: Some(300000),
            wall_time_ms: Some(600000),
            memory_bytes: Some(1024 * 1024 * 1024),
            output_bytes: Some(200 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D1,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: Some("CI supply-chain gate".to_string()),
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.dataset".to_string()],
            max_bytes: Some(200 * 1024 * 1024),
        },
    };

    let _ = runtime.execute(op).await.expect("runtime should return");

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");
    assert!(
        events
            .iter()
            .any(|e| e.event_type == FlightRecorderEventType::TerminalCommand),
        "expected FR-EVT-001 terminal_command event"
    );
}

#[tokio::test]
#[ignore]
async fn ci_license_scan_runs_via_terminal_service() {
    let root = repo_root();
    let op_id = Uuid::new_v4();
    let op_dir = root
        .join("data/mex_supply_chain/license_scan")
        .join(op_id.to_string());
    std::fs::create_dir_all(&op_dir).expect("op_dir should create");

    let recorder = on_disk_recorder(&op_dir.join("flight_recorder.db"));
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let redactor: Arc<dyn handshake_core::terminal::redaction::SecretRedactor> =
        Arc::new(PatternRedactor);

    let tool_runner: Arc<dyn ToolRunner> = Arc::new(TerminalServiceRunner::new(
        TerminalConfig::new(root.clone()),
        CapabilityRegistry::new(),
        flight_recorder,
        redactor,
        Vec::new(),
    ));

    let runtime = build_runtime_with_supply_chain_adapter(
        mex_registry_from_disk(),
        recorder.clone(),
        tool_runner,
        root.clone(),
    );

    let op = PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id,
        engine_id: "engine.supply_chain.license".to_string(),
        engine_version_req: None,
        operation: "license_scan".to_string(),
        inputs: vec![ArtifactHandle::new(Uuid::new_v4(), "repo:.".to_string())],
        params: serde_json::json!({ "release_mode": env_release_mode() }),
        capabilities_requested: vec![
            "fs.read:inputs".to_string(),
            "fs.write:artifacts".to_string(),
            "proc.exec:scancode".to_string(),
        ],
        budget: BudgetSpec {
            cpu_time_ms: Some(300000),
            wall_time_ms: Some(900000),
            memory_bytes: Some(1024 * 1024 * 1024),
            output_bytes: Some(200 * 1024 * 1024),
        },
        determinism: DeterminismLevel::D1,
        evidence_policy: Some(EvidencePolicy {
            required: true,
            notes: Some("CI supply-chain gate".to_string()),
        }),
        output_spec: OutputSpec {
            expected_types: vec!["artifact.dataset".to_string()],
            max_bytes: Some(200 * 1024 * 1024),
        },
    };

    let _ = runtime.execute(op).await.expect("runtime should return");

    let events = recorder
        .list_events(EventFilter {
            job_id: Some(op_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("events should be queryable");
    assert!(
        events
            .iter()
            .any(|e| e.event_type == FlightRecorderEventType::TerminalCommand),
        "expected FR-EVT-001 terminal_command event"
    );
}
