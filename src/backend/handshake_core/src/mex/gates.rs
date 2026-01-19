use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::capabilities::{CapabilityRegistry, RegistryError};
use crate::mex::envelope::{DeterminismLevel, PlannedOperation, POE_SCHEMA_VERSION};
use crate::mex::registry::MexRegistry;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DenialSeverity {
    Error,
    Warn,
}

/// Gate failure payload routed to Flight Recorder + Problems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateDenial {
    pub gate: String,
    pub reason: String,
    pub code: Option<String>,
    pub details: Option<Value>,
    pub severity: DenialSeverity,
}

pub trait Gate: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, op: &PlannedOperation, registry: &MexRegistry) -> Result<(), GateDenial>;
}

pub struct GatePipeline {
    gates: Vec<Box<dyn Gate>>,
}

impl GatePipeline {
    pub fn new(gates: Vec<Box<dyn Gate>>) -> Self {
        Self { gates }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Box<dyn Gate>> {
        self.gates.iter()
    }
}

pub struct SchemaGate;

impl Gate for SchemaGate {
    fn name(&self) -> &'static str {
        "G-SCHEMA"
    }

    fn check(&self, op: &PlannedOperation, _registry: &MexRegistry) -> Result<(), GateDenial> {
        if op.schema_version != POE_SCHEMA_VERSION {
            return Err(GateDenial {
                gate: self.name().to_string(),
                reason: "Invalid schema_version (expected poe-1.0)".to_string(),
                code: None,
                details: Some(Value::String(op.schema_version.clone())),
                severity: DenialSeverity::Error,
            });
        }

        if op.operation.trim().is_empty() || op.engine_id.trim().is_empty() {
            return Err(GateDenial {
                gate: self.name().to_string(),
                reason: "operation and engine_id must be non-empty".to_string(),
                code: None,
                details: None,
                severity: DenialSeverity::Error,
            });
        }

        Ok(())
    }
}

pub struct CapabilityGate {
    registry: CapabilityRegistry,
}

impl CapabilityGate {
    pub fn new(registry: CapabilityRegistry) -> Self {
        Self { registry }
    }
}

impl Gate for CapabilityGate {
    fn name(&self) -> &'static str {
        "G-CAP"
    }

    fn check(&self, op: &PlannedOperation, registry: &MexRegistry) -> Result<(), GateDenial> {
        if op.capabilities_requested.is_empty() {
            return Err(GateDenial {
                gate: self.name().to_string(),
                reason: "No capabilities requested; default-deny".to_string(),
                code: None,
                details: None,
                severity: DenialSeverity::Error,
            });
        }

        let engine_spec = registry
            .get_engine(&op.engine_id)
            .ok_or_else(|| GateDenial {
                gate: self.name().to_string(),
                reason: "Engine not registered in MexRegistry".to_string(),
                code: None,
                details: Some(Value::String(op.engine_id.clone())),
                severity: DenialSeverity::Error,
            })?;

        let operation_spec = registry
            .get_operation(&op.engine_id, &op.operation)
            .ok_or_else(|| GateDenial {
                gate: self.name().to_string(),
                reason: "Operation not registered for engine".to_string(),
                code: None,
                details: Some(json!({
                    "engine_id": op.engine_id.clone(),
                    "operation": op.operation.clone(),
                })),
                severity: DenialSeverity::Error,
            })?;

        let mut allowed_caps = engine_spec.required_caps.clone();
        allowed_caps.extend(operation_spec.capabilities.iter().cloned());
        allowed_caps.sort();
        allowed_caps.dedup();

        for cap in &op.capabilities_requested {
            match self.registry.enforce_can_perform(cap, &allowed_caps) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(GateDenial {
                        gate: self.name().to_string(),
                        reason: "Capability not granted".to_string(),
                        code: None,
                        details: Some(Value::String(cap.clone())),
                        severity: DenialSeverity::Error,
                    });
                }
                Err(RegistryError::UnknownCapability(_)) => {
                    return Err(GateDenial {
                        gate: self.name().to_string(),
                        reason: RegistryError::UnknownCapability(cap.clone()).to_string(),
                        code: Some("HSK-4001".to_string()),
                        details: Some(Value::String(cap.clone())),
                        severity: DenialSeverity::Error,
                    });
                }
                Err(err) => {
                    return Err(GateDenial {
                        gate: self.name().to_string(),
                        reason: format!("Capability check failed: {err}"),
                        code: None,
                        details: Some(Value::String(cap.clone())),
                        severity: DenialSeverity::Error,
                    });
                }
            };
        }

        Ok(())
    }
}

pub struct IntegrityGate;

impl Gate for IntegrityGate {
    fn name(&self) -> &'static str {
        "G-INTEGRITY"
    }

    fn check(&self, op: &PlannedOperation, _registry: &MexRegistry) -> Result<(), GateDenial> {
        // Enforce artifact-first by bounding inline params payload.
        if let Ok(raw) = serde_json::to_vec(&op.params) {
            const INLINE_LIMIT: usize = 32 * 1024;
            if raw.len() > INLINE_LIMIT {
                return Err(GateDenial {
                    gate: self.name().to_string(),
                    reason: "Inline params exceed 32KB; use artifact handles".to_string(),
                    code: None,
                    details: Some(Value::Number(serde_json::Number::from(raw.len() as u64))),
                    severity: DenialSeverity::Error,
                });
            }
        }

        Ok(())
    }
}

pub struct BudgetGate;

impl Gate for BudgetGate {
    fn name(&self) -> &'static str {
        "G-BUDGET"
    }

    fn check(&self, op: &PlannedOperation, _registry: &MexRegistry) -> Result<(), GateDenial> {
        // Require at least one budget limit to avoid unbounded runs.
        if op.budget.cpu_time_ms.is_none()
            && op.budget.wall_time_ms.is_none()
            && op.budget.memory_bytes.is_none()
            && op.budget.output_bytes.is_none()
        {
            return Err(GateDenial {
                gate: self.name().to_string(),
                reason: "Missing budget caps (cpu/wall/memory/output)".to_string(),
                code: None,
                details: None,
                severity: DenialSeverity::Error,
            });
        }

        if let (Some(max_output), Some(spec_max)) =
            (op.budget.output_bytes, op.output_spec.max_bytes)
        {
            if spec_max > max_output {
                return Err(GateDenial {
                    gate: self.name().to_string(),
                    reason: "output_spec exceeds budgeted output_bytes".to_string(),
                    code: None,
                    details: Some(Value::Number(serde_json::Number::from(spec_max))),
                    severity: DenialSeverity::Error,
                });
            }
        }

        Ok(())
    }
}

pub struct ProvenanceGate;

impl Gate for ProvenanceGate {
    fn name(&self) -> &'static str {
        "G-PROVENANCE"
    }

    fn check(&self, op: &PlannedOperation, _registry: &MexRegistry) -> Result<(), GateDenial> {
        let evidence_required = op
            .evidence_policy
            .as_ref()
            .map(|policy| policy.required)
            .unwrap_or(false);
        if matches!(op.determinism, DeterminismLevel::D0 | DeterminismLevel::D1)
            && !evidence_required
        {
            return Err(GateDenial {
                gate: self.name().to_string(),
                reason: "Evidence policy missing for D0/D1 operation".to_string(),
                code: None,
                details: None,
                severity: DenialSeverity::Error,
            });
        }

        if op.capabilities_requested.is_empty() {
            return Err(GateDenial {
                gate: self.name().to_string(),
                reason: "Provenance requires explicit capabilities granted".to_string(),
                code: None,
                details: None,
                severity: DenialSeverity::Error,
            });
        }

        Ok(())
    }
}

pub struct DetGate;

impl Gate for DetGate {
    fn name(&self) -> &'static str {
        "G-DET"
    }

    fn check(&self, op: &PlannedOperation, registry: &MexRegistry) -> Result<(), GateDenial> {
        let Some(engine_spec) = registry.get_engine(&op.engine_id) else {
            return Err(GateDenial {
                gate: self.name().to_string(),
                reason: "Engine not registered in MexRegistry".to_string(),
                code: None,
                details: Some(Value::String(op.engine_id.clone())),
                severity: DenialSeverity::Error,
            });
        };

        if op.determinism.rank() > engine_spec.determinism_ceiling.rank() {
            return Err(GateDenial {
                gate: self.name().to_string(),
                reason: "Determinism level exceeds engine ceiling".to_string(),
                code: None,
                details: Some(Value::String(format!(
                    "requested={:?}, ceiling={:?}",
                    op.determinism, engine_spec.determinism_ceiling
                ))),
                severity: DenialSeverity::Error,
            });
        }

        Ok(())
    }
}
