use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{json, Value};
use thiserror::Error;

use crate::diagnostics::{
    DiagnosticActor, DiagnosticInput, DiagnosticSeverity, DiagnosticSource, DiagnosticSurface,
    DiagnosticsStore, LinkConfidence,
};
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::mex::envelope::{DeterminismLevel, EngineResult, PlannedOperation};
use crate::mex::gates::{DenialSeverity, GateDenial, GatePipeline};
use crate::mex::registry::{MexRegistry, RegistryError};

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("Engine adapter error: {0}")]
    Engine(String),
}

#[async_trait]
pub trait EngineAdapter: Send + Sync {
    async fn invoke(&self, op: &PlannedOperation) -> Result<EngineResult, AdapterError>;
}

#[derive(Debug, Error)]
pub enum MexRuntimeError {
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("Gate denied: {0:?}")]
    Gate(GateDenial),
    #[error("Engine result missing evidence for determinism {0:?}")]
    EvidenceMissing(DeterminismLevel),
    #[error("Engine adapter missing for {0}")]
    AdapterMissing(String),
    #[error("Adapter failed: {0}")]
    Adapter(AdapterError),
    #[error("Flight Recorder error: {0}")]
    Logging(String),
}

pub struct MexRuntime {
    registry: MexRegistry,
    flight_recorder: Arc<dyn FlightRecorder>,
    diagnostics: Arc<dyn DiagnosticsStore>,
    adapters: HashMap<String, Arc<dyn EngineAdapter>>,
    gates: GatePipeline,
}

impl MexRuntime {
    pub fn new(
        registry: MexRegistry,
        flight_recorder: Arc<dyn FlightRecorder>,
        diagnostics: Arc<dyn DiagnosticsStore>,
        gates: GatePipeline,
    ) -> Self {
        Self {
            registry,
            flight_recorder,
            diagnostics,
            adapters: HashMap::new(),
            gates,
        }
    }

    pub fn with_adapter(
        mut self,
        engine_id: impl Into<String>,
        adapter: Arc<dyn EngineAdapter>,
    ) -> Self {
        self.adapters.insert(engine_id.into(), adapter);
        self
    }

    pub fn registry(&self) -> &MexRegistry {
        &self.registry
    }

    fn artifact_refs(handles: &[crate::ace::ArtifactHandle]) -> Vec<String> {
        handles.iter().map(|h| h.canonical_id()).collect()
    }

    fn record_tool_fr_event(
        &self,
        op: &PlannedOperation,
        event_kind: &str,
        level: &str,
        message: &str,
        payload: Value,
    ) -> Result<(), MexRuntimeError> {
        let Some(conn) = self.flight_recorder.duckdb_connection() else {
            return Ok(());
        };
        let conn = conn
            .lock()
            .map_err(|_| MexRuntimeError::Logging("duckdb connection lock error".to_string()))?;

        let next_id: i64 = conn
            .prepare("SELECT COALESCE(MAX(event_id), 0) + 1 FROM fr_events")
            .map_err(|e| MexRuntimeError::Logging(e.to_string()))?
            .query_row([], |row| row.get(0))
            .map_err(|e| MexRuntimeError::Logging(e.to_string()))?;

        let payload_str =
            serde_json::to_string(&payload).map_err(|e| MexRuntimeError::Logging(e.to_string()))?;

        let source = format!("mex:{}", op.engine_id);
        let job_id = op.op_id.to_string();
        let workflow_run_id: Option<&str> = None;
        let session_id: Option<&str> = None;
        let task_id: Option<&str> = None;

        conn.execute(
            r#"
            INSERT INTO fr_events (
                event_id,
                ts_utc,
                session_id,
                task_id,
                job_id,
                workflow_run_id,
                event_kind,
                source,
                level,
                message,
                payload
            ) VALUES (?, CURRENT_TIMESTAMP, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            duckdb::params![
                next_id,
                session_id,
                task_id,
                job_id,
                workflow_run_id,
                event_kind,
                source,
                level,
                message,
                payload_str
            ],
        )
        .map_err(|e| MexRuntimeError::Logging(e.to_string()))?;

        Ok(())
    }

    fn record_tool_call(&self, op: &PlannedOperation) -> Result<(), MexRuntimeError> {
        let capability_id = op.capabilities_requested.first().cloned();
        let payload = json!({
            "tool_name": format!("mex:{}", op.engine_id),
            "tool_version": null,
            "operation": op.operation,
            "inputs": Self::artifact_refs(&op.inputs),
            "outputs": [],
            "status": "success",
            "duration_ms": null,
            "error_code": null,
            "job_id": op.op_id.to_string(),
            "workflow_run_id": null,
            "trace_id": op.op_id.to_string(),
            "capability_id": capability_id,
            "capabilities": op.capabilities_requested,
            "budget": op.budget,
            "determinism": op.determinism,
        });

        self.record_tool_fr_event(op, "tool.call", "INFO", "tool invocation started", payload)
    }

    fn record_tool_result(
        &self,
        op: &PlannedOperation,
        result: Option<&EngineResult>,
        duration_ms: Option<u64>,
        status: &str,
        error_code: Option<String>,
    ) -> Result<(), MexRuntimeError> {
        let tool_version = result
            .and_then(|r| r.provenance.engine_version.clone())
            .unwrap_or_default();
        let tool_version = if tool_version.trim().is_empty() {
            Value::Null
        } else {
            Value::String(tool_version)
        };

        let outputs: Vec<String> = result
            .map(|r| {
                let mut out = Self::artifact_refs(&r.outputs);
                out.extend(Self::artifact_refs(&r.evidence));
                if let Some(logs) = &r.logs_ref {
                    out.push(logs.canonical_id());
                }
                out
            })
            .unwrap_or_default();

        let capability_id = op.capabilities_requested.first().cloned();
        let payload = json!({
            "tool_name": format!("mex:{}", op.engine_id),
            "tool_version": tool_version,
            "operation": op.operation,
            "inputs": Self::artifact_refs(&op.inputs),
            "outputs": outputs,
            "status": status,
            "duration_ms": duration_ms,
            "error_code": error_code,
            "job_id": op.op_id.to_string(),
            "workflow_run_id": null,
            "trace_id": op.op_id.to_string(),
            "capability_id": capability_id,
            "capabilities": op.capabilities_requested,
            "budget": op.budget,
            "determinism": op.determinism,
        });

        let level = if status == "success" { "INFO" } else { "ERROR" };
        self.record_tool_fr_event(
            op,
            "tool.result",
            level,
            "tool invocation finished",
            payload,
        )
    }

    pub async fn execute(&self, op: PlannedOperation) -> Result<EngineResult, MexRuntimeError> {
        for gate in self.gates.iter() {
            match gate.check(&op, &self.registry) {
                Ok(()) => {
                    if gate.name() == "G-CAP" {
                        for capability_id in &op.capabilities_requested {
                            self.record_capability_action(&op, capability_id, "allow")
                                .await?;
                        }
                    }
                    self.record_gate_outcome(&op, gate.name(), "pass", None, None)
                        .await?;
                }
                Err(denial) => {
                    if gate.name() == "G-CAP" {
                        if let Some(capability_id) = Self::denied_capability_id(&denial) {
                            self.record_capability_action(&op, &capability_id, "deny")
                                .await?;
                        }
                    }
                    let diagnostic_id = self.record_denial_diagnostic(&op, &denial).await?;
                    self.record_gate_outcome(
                        &op,
                        gate.name(),
                        "deny",
                        Some(&denial),
                        Some(diagnostic_id),
                    )
                    .await?;
                    return Err(MexRuntimeError::Gate(denial));
                }
            }
        }

        let engine_id = op.engine_id.clone();
        let adapter = self
            .adapters
            .get(&engine_id)
            .ok_or_else(|| MexRuntimeError::AdapterMissing(engine_id.clone()))?;

        self.record_tool_call(&op)?;

        // WAIVER [CX-573E]: Instant::now() is required for duration measurement (observability only).
        let start = Instant::now();
        let invoke_result = adapter.invoke(&op).await;
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut result = match invoke_result {
            Ok(result) => result,
            Err(err) => {
                self.record_tool_result(
                    &op,
                    None,
                    Some(duration_ms),
                    "error",
                    Some("MEX_ADAPTER_ERROR".to_string()),
                )?;
                return Err(MexRuntimeError::Adapter(err));
            }
        };

        if op.determinism.requires_evidence() && result.evidence.is_empty() {
            self.record_missing_evidence_diagnostic(&op).await?;
            self.record_tool_result(
                &op,
                Some(&result),
                Some(duration_ms),
                "error",
                Some("MEX_EVIDENCE_MISSING".to_string()),
            )?;
            return Err(MexRuntimeError::EvidenceMissing(op.determinism));
        }

        // Attach engine_id to provenance if missing.
        result.provenance = result.provenance.with_engine_id(&engine_id);

        self.record_tool_result(&op, Some(&result), Some(duration_ms), "success", None)?;

        Ok(result)
    }

    fn denied_capability_id(denial: &GateDenial) -> Option<String> {
        match denial.details.as_ref() {
            Some(serde_json::Value::String(value)) => Some(value.clone()),
            _ => None,
        }
    }

    async fn record_denial_diagnostic(
        &self,
        op: &PlannedOperation,
        denial: &GateDenial,
    ) -> Result<uuid::Uuid, MexRuntimeError> {
        let severity = match denial.severity {
            DenialSeverity::Error => DiagnosticSeverity::Error,
            DenialSeverity::Warn => DiagnosticSeverity::Warning,
        };

        let detail_note = denial.details.as_ref().map(|v| v.to_string());
        let message = match detail_note {
            Some(details) => format!("{} (details: {})", denial.reason, details),
            None => denial.reason.clone(),
        };

        let diagnostic = DiagnosticInput {
            title: format!("MEX gate denied: {}", denial.gate),
            message,
            severity,
            source: DiagnosticSource::Engine,
            surface: DiagnosticSurface::System,
            tool: Some(denial.gate.clone()),
            code: denial.code.clone(),
            tags: None,
            wsid: None,
            job_id: Some(op.op_id.to_string()),
            model_id: None,
            actor: Some(DiagnosticActor::System),
            capability_id: None,
            policy_decision_id: None,
            locations: None,
            evidence_refs: None,
            link_confidence: LinkConfidence::Direct,
            status: None,
            count: None,
            first_seen: None,
            last_seen: None,
            timestamp: None,
            updated_at: None,
        }
        .into_diagnostic()
        .map_err(|err| MexRuntimeError::Logging(err.to_string()))?;

        let diagnostic_id = diagnostic.id;
        self.diagnostics
            .record_diagnostic(diagnostic)
            .await
            .map_err(|err| MexRuntimeError::Logging(err.to_string()))?;
        Ok(diagnostic_id)
    }

    async fn record_missing_evidence_diagnostic(
        &self,
        op: &PlannedOperation,
    ) -> Result<uuid::Uuid, MexRuntimeError> {
        let diagnostic = DiagnosticInput {
            title: "MEX result missing evidence".to_string(),
            message: format!(
                "D0/D1 operation returned no evidence artifacts (determinism={:?})",
                op.determinism
            ),
            severity: DiagnosticSeverity::Error,
            source: DiagnosticSource::Engine,
            surface: DiagnosticSurface::System,
            tool: Some("mex_runtime".to_string()),
            code: None,
            tags: None,
            wsid: None,
            job_id: Some(op.op_id.to_string()),
            model_id: None,
            actor: Some(DiagnosticActor::System),
            capability_id: None,
            policy_decision_id: None,
            locations: None,
            evidence_refs: None,
            link_confidence: LinkConfidence::Direct,
            status: None,
            count: None,
            first_seen: None,
            last_seen: None,
            timestamp: None,
            updated_at: None,
        }
        .into_diagnostic()
        .map_err(|err| MexRuntimeError::Logging(err.to_string()))?;

        let diagnostic_id = diagnostic.id;
        self.diagnostics
            .record_diagnostic(diagnostic)
            .await
            .map_err(|err| MexRuntimeError::Logging(err.to_string()))?;
        Ok(diagnostic_id)
    }

    async fn record_capability_action(
        &self,
        op: &PlannedOperation,
        capability_id: &str,
        decision_outcome: &str,
    ) -> Result<(), MexRuntimeError> {
        let actor_id = "mex_runtime";
        let job_id = op.op_id.to_string();
        let payload = json!({
            "capability_id": capability_id,
            "actor_id": actor_id,
            "job_id": job_id,
            "decision_outcome": decision_outcome,
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::CapabilityAction,
            FlightRecorderActor::System,
            op.op_id,
            payload,
        )
        .with_job_id(op.op_id.to_string())
        .with_actor_id(actor_id)
        .with_capability(capability_id.to_string());

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|err| MexRuntimeError::Logging(err.to_string()))
    }

    async fn record_gate_outcome(
        &self,
        op: &PlannedOperation,
        gate: &str,
        outcome: &str,
        denial: Option<&GateDenial>,
        diagnostic_id: Option<uuid::Uuid>,
    ) -> Result<(), MexRuntimeError> {
        let level = match denial.map(|d| d.severity.clone()) {
            Some(DenialSeverity::Warn) => "warning",
            Some(DenialSeverity::Error) => "error",
            None => "info",
        };

        let payload = json!({
            "component": "mex_runtime",
            "message": "gate_outcome",
            "level": level,
            "details": {
                "gate": gate,
                "outcome": outcome,
                "op_id": op.op_id,
                "engine_id": op.engine_id,
                "operation": op.operation,
                "code": denial.and_then(|d| d.code.clone()),
                "reason": denial.map(|d| d.reason.clone()),
                "severity": denial.map(|d| format!("{:?}", d.severity)),
                "diagnostic_id": diagnostic_id.map(|id| id.to_string()),
            }
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            op.op_id,
            payload,
        )
        .with_job_id(op.op_id.to_string())
        .with_actor_id("mex_runtime");

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|err| MexRuntimeError::Logging(err.to_string()))
    }
}

impl From<RegistryError> for MexRuntimeError {
    fn from(err: RegistryError) -> Self {
        MexRuntimeError::Registry(err.to_string())
    }
}
