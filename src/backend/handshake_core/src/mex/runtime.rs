use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use thiserror::Error;

use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use crate::mex::envelope::{EngineResult, PlannedOperation};
use crate::mex::gates::{GateDenial, GatePipeline};
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
    adapters: HashMap<String, Arc<dyn EngineAdapter>>,
    gates: GatePipeline,
}

impl MexRuntime {
    pub fn new(
        registry: MexRegistry,
        flight_recorder: Arc<dyn FlightRecorder>,
        gates: GatePipeline,
    ) -> Self {
        Self {
            registry,
            flight_recorder,
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
            if let Err(denial) = gate.check(&op, &self.registry) {
                self.record_denial(&op, &denial).await?;
                return Err(MexRuntimeError::Gate(denial));
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

        // Attach engine_id to provenance if missing.
        result.provenance = result.provenance.with_engine_id(&engine_id);

        Ok(result)
    }

    async fn record_denial(
        &self,
        op: &PlannedOperation,
        denial: &GateDenial,
    ) -> Result<(), MexRuntimeError> {
        let payload = json!({
            "op_id": op.op_id,
            "engine_id": op.engine_id,
            "gate": denial.gate,
            "reason": denial.reason,
            "severity": format!("{:?}", denial.severity),
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::Diagnostic,
            FlightRecorderActor::System,
            op.op_id, // reuse op_id as trace_id for linkage
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
