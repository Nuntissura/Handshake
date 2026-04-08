use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::capabilities::{CapabilityRegistry, RegistryError};
use crate::mex::envelope::{
    BudgetSpec, DeterminismLevel, OutputSpec, PlannedOperation, POE_SCHEMA_VERSION,
};
use crate::mex::registry::MexRegistry;
use crate::workspace_safety::{
    enforce_cross_session_access, enforce_workspace_isolation, SessionWorktreeRegistry,
};

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

    /// Run all gates against an operation, collecting every denial.
    /// Unlike the runtime's short-circuit loop, this reports all failures
    /// so callers can surface a complete problem list.
    pub fn evaluate(
        &self,
        op: &PlannedOperation,
        registry: &MexRegistry,
    ) -> Vec<GateDenial> {
        self.gates
            .iter()
            .filter_map(|gate| gate.check(op, registry).err())
            .collect()
    }
}

fn gate_probe_operation() -> PlannedOperation {
    PlannedOperation {
        schema_version: POE_SCHEMA_VERSION.to_string(),
        op_id: Uuid::nil(),
        engine_id: "workspace_safety".to_string(),
        engine_version_req: None,
        operation: "workspace_safety.enforce".to_string(),
        inputs: Vec::new(),
        params: json!({}),
        capabilities_requested: vec!["fs.read".to_string()],
        capability_profile_id: Some("Coder".to_string()),
        human_consent_obtained: false,
        budget: BudgetSpec {
            cpu_time_ms: None,
            wall_time_ms: Some(1),
            memory_bytes: None,
            output_bytes: Some(1),
        },
        determinism: DeterminismLevel::D3,
        evidence_policy: None,
        output_spec: OutputSpec {
            expected_types: Vec::new(),
            max_bytes: Some(1),
        },
    }
}

pub fn evaluate_session_safety_gates(
    session_id: &str,
    target_path: &Path,
    operator_approved: bool,
    worktree_registry: SessionWorktreeRegistry,
) -> Vec<GateDenial> {
    let pipeline = GatePipeline::new(vec![
        Box::new(IsolationGate::new(
            session_id.to_string(),
            worktree_registry.clone(),
        )),
        Box::new(CrossSessionGate::new(
            session_id.to_string(),
            target_path.to_path_buf(),
            operator_approved,
            worktree_registry,
        )),
    ]);
    let registry = MexRegistry::from_map(std::collections::HashMap::new());
    pipeline.evaluate(&gate_probe_operation(), &registry)
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

        let capability_profile_id = op
            .capability_profile_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| GateDenial {
                gate: self.name().to_string(),
                reason: "Missing capability_profile_id; default-deny".to_string(),
                code: None,
                details: None,
                severity: DenialSeverity::Error,
            })?;

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
                        reason: "Capability not allowlisted for engine/operation".to_string(),
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
                        reason: format!("Capability allowlist check failed: {err}"),
                        code: None,
                        details: Some(Value::String(cap.clone())),
                        severity: DenialSeverity::Error,
                    });
                }
            }

            match self.registry.profile_can(capability_profile_id, cap) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(GateDenial {
                        gate: self.name().to_string(),
                        reason: "Capability not granted by capability_profile_id".to_string(),
                        code: None,
                        details: Some(json!({
                            "capability_profile_id": capability_profile_id,
                            "capability_id": cap,
                        })),
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
                Err(RegistryError::UnknownProfile(_)) => {
                    return Err(GateDenial {
                        gate: self.name().to_string(),
                        reason: RegistryError::UnknownProfile(capability_profile_id.to_string())
                            .to_string(),
                        code: None,
                        details: Some(Value::String(capability_profile_id.to_string())),
                        severity: DenialSeverity::Error,
                    });
                }
                Err(err) => {
                    return Err(GateDenial {
                        gate: self.name().to_string(),
                        reason: format!("Capability profile check failed: {err}"),
                        code: None,
                        details: Some(Value::String(cap.clone())),
                        severity: DenialSeverity::Error,
                    });
                }
            }
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

/// INV-WS-002: Fail-closed isolation gate.
/// Denies any operation from a session that has no worktree allocation.
pub struct IsolationGate {
    session_id: String,
    worktree_registry: SessionWorktreeRegistry,
}

impl IsolationGate {
    pub fn new(session_id: impl Into<String>, worktree_registry: SessionWorktreeRegistry) -> Self {
        Self {
            session_id: session_id.into(),
            worktree_registry,
        }
    }
}

impl Gate for IsolationGate {
    fn name(&self) -> &'static str {
        "G-ISOLATION"
    }

    fn check(&self, op: &PlannedOperation, _registry: &MexRegistry) -> Result<(), GateDenial> {
        match enforce_workspace_isolation(&self.worktree_registry, &self.session_id, op.op_id) {
            Ok(_) => Ok(()),
            Err((denial, fr_event)) => Err(GateDenial {
                gate: self.name().to_string(),
                reason: denial.to_string(),
                code: Some("INV-WS-002".to_string()),
                details: Some(json!({
                    "session_id": self.session_id,
                    "fr_event_type": fr_event.event_type.to_string(),
                    "fr_payload": fr_event.payload,
                })),
                severity: DenialSeverity::Error,
            }),
        }
    }
}

/// INV-WS-003: Cross-session access gate.
/// Denies operations that target paths inside another session's worktree
/// unless explicit operator approval was provided.
pub struct CrossSessionGate {
    session_id: String,
    target_path: PathBuf,
    operator_approved: bool,
    worktree_registry: SessionWorktreeRegistry,
}

impl CrossSessionGate {
    pub fn new(
        session_id: impl Into<String>,
        target_path: impl Into<PathBuf>,
        operator_approved: bool,
        worktree_registry: SessionWorktreeRegistry,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            target_path: target_path.into(),
            operator_approved,
            worktree_registry,
        }
    }
}

impl Gate for CrossSessionGate {
    fn name(&self) -> &'static str {
        "G-CROSS-SESSION"
    }

    fn check(&self, op: &PlannedOperation, _registry: &MexRegistry) -> Result<(), GateDenial> {
        match enforce_cross_session_access(
            &self.worktree_registry,
            &self.session_id,
            &self.target_path,
            self.operator_approved,
            op.op_id,
        ) {
            Ok((_result, _fr_event)) => Ok(()),
            Err((denial, fr_event)) => Err(GateDenial {
                gate: self.name().to_string(),
                reason: denial.to_string(),
                code: Some("INV-WS-003".to_string()),
                details: Some(json!({
                    "session_id": self.session_id,
                    "target_path": self.target_path.display().to_string(),
                    "operator_approved": self.operator_approved,
                    "fr_event_type": fr_event.event_type.to_string(),
                    "fr_payload": fr_event.payload,
                })),
                severity: DenialSeverity::Error,
            }),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::ArtifactHandle;
    use crate::mex::envelope::{
        BudgetSpec, DeterminismLevel, EvidencePolicy, OutputSpec, POE_SCHEMA_VERSION,
    };
    use crate::workspace_safety::{SessionWorktreeAllocation, SessionWorktreeRegistry};
    use std::collections::HashMap;
    use uuid::Uuid;

    fn dummy_op() -> PlannedOperation {
        PlannedOperation {
            schema_version: POE_SCHEMA_VERSION.to_string(),
            op_id: Uuid::nil(),
            engine_id: "test_engine".to_string(),
            engine_version_req: None,
            operation: "test.op".to_string(),
            inputs: vec![ArtifactHandle::new(Uuid::nil(), "/test".to_string())],
            params: serde_json::json!({}),
            capabilities_requested: vec!["fs.read".to_string()],
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
                notes: None,
            }),
            output_spec: OutputSpec {
                expected_types: vec!["artifact.document".to_string()],
                max_bytes: Some(8 * 1024 * 1024),
            },
        }
    }

    fn two_session_registry() -> SessionWorktreeRegistry {
        let mut registry = SessionWorktreeRegistry::new();
        registry.put(SessionWorktreeAllocation::new(
            "session-a",
            "/worktrees/session-a",
        ));
        registry.put(SessionWorktreeAllocation::new(
            "session-b",
            "/worktrees/session-b",
        ));
        registry
    }

    #[test]
    fn isolation_gate_passes_when_session_registered() {
        let registry = two_session_registry();
        let mex_registry = MexRegistry::from_map(HashMap::new());
        let gate = IsolationGate::new("session-a", registry);
        let pipeline = GatePipeline::new(vec![Box::new(gate)]);
        let denials = pipeline.evaluate(&dummy_op(), &mex_registry);
        assert!(
            denials.is_empty(),
            "registered session must pass isolation gate"
        );
    }

    #[test]
    fn isolation_gate_denies_unregistered_session() {
        let registry = two_session_registry();
        let mex_registry = MexRegistry::from_map(HashMap::new());
        let gate = IsolationGate::new("unknown-session", registry);
        let pipeline = GatePipeline::new(vec![Box::new(gate)]);
        let denials = pipeline.evaluate(&dummy_op(), &mex_registry);
        assert_eq!(denials.len(), 1);
        assert_eq!(denials[0].gate, "G-ISOLATION");
        assert_eq!(denials[0].code.as_deref(), Some("INV-WS-002"));
    }

    #[test]
    fn cross_session_gate_passes_for_own_path() {
        let registry = two_session_registry();
        let mex_registry = MexRegistry::from_map(HashMap::new());
        let gate = CrossSessionGate::new(
            "session-a",
            "/worktrees/session-a/src/lib.rs",
            false,
            registry,
        );
        let pipeline = GatePipeline::new(vec![Box::new(gate)]);
        let denials = pipeline.evaluate(&dummy_op(), &mex_registry);
        assert!(
            denials.is_empty(),
            "own session path must pass cross-session gate"
        );
    }

    #[test]
    fn cross_session_gate_denies_other_session_path() {
        let registry = two_session_registry();
        let mex_registry = MexRegistry::from_map(HashMap::new());
        let gate = CrossSessionGate::new(
            "session-a",
            "/worktrees/session-b/src/main.rs",
            false,
            registry,
        );
        let pipeline = GatePipeline::new(vec![Box::new(gate)]);
        let denials = pipeline.evaluate(&dummy_op(), &mex_registry);
        assert_eq!(denials.len(), 1);
        assert_eq!(denials[0].gate, "G-CROSS-SESSION");
        assert_eq!(denials[0].code.as_deref(), Some("INV-WS-003"));
    }

    #[test]
    fn cross_session_gate_passes_with_operator_approval() {
        let registry = two_session_registry();
        let mex_registry = MexRegistry::from_map(HashMap::new());
        let gate = CrossSessionGate::new(
            "session-a",
            "/worktrees/session-b/src/main.rs",
            true,
            registry,
        );
        let pipeline = GatePipeline::new(vec![Box::new(gate)]);
        let denials = pipeline.evaluate(&dummy_op(), &mex_registry);
        assert!(
            denials.is_empty(),
            "operator-approved cross-session access must pass"
        );
    }

    #[test]
    fn pipeline_evaluate_collects_multiple_denials() {
        let registry = SessionWorktreeRegistry::new();
        let mex_registry = MexRegistry::from_map(HashMap::new());
        let isolation = IsolationGate::new("no-session", registry.clone());
        let cross = CrossSessionGate::new("no-session", "/some/path", false, registry);
        let pipeline = GatePipeline::new(vec![Box::new(isolation), Box::new(cross)]);
        let denials = pipeline.evaluate(&dummy_op(), &mex_registry);
        // IsolationGate denies; CrossSessionGate passes (empty registry = no other sessions)
        assert_eq!(denials.len(), 1);
        assert_eq!(denials[0].gate, "G-ISOLATION");
    }
}
