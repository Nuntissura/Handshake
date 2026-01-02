use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
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

    pub async fn execute(&self, op: PlannedOperation) -> Result<EngineResult, MexRuntimeError> {
        for gate in self.gates.iter() {
            match gate.check(&op, &self.registry) {
                Ok(()) => {
                    if gate.name() == "G-CAP" {
                        for capability_id in &op.capabilities_requested {
                            self.record_capability_action(&op, capability_id, "allowed")
                                .await?;
                        }
                    }
                    self.record_gate_outcome(&op, gate.name(), "pass", None, None)
                        .await?;
                }
                Err(denial) => {
                    if gate.name() == "G-CAP" {
                        if let Some(capability_id) = Self::denied_capability_id(&denial) {
                            self.record_capability_action(&op, &capability_id, "denied")
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

        let mut result = adapter
            .invoke(&op)
            .await
            .map_err(MexRuntimeError::Adapter)?;

        if op.determinism.requires_evidence() && result.evidence.is_empty() {
            self.record_missing_evidence_diagnostic(&op).await?;
            return Err(MexRuntimeError::EvidenceMissing(op.determinism));
        }

        // Attach engine_id to provenance if missing.
        result.provenance = result.provenance.with_engine_id(&engine_id);

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
        outcome: &str,
    ) -> Result<(), MexRuntimeError> {
        let payload = json!({
            "capability_id": capability_id,
            "action": "mex.capability_check",
            "outcome": outcome,
            "profile_id": null,
            "policy_decision_id": null,
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::CapabilityAction,
            FlightRecorderActor::System,
            op.op_id,
            payload,
        )
        .with_job_id(op.op_id.to_string())
        .with_actor_id("mex_runtime")
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
