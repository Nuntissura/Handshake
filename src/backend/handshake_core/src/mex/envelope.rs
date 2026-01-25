use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ace::ArtifactHandle;

/// Canonical PlannedOperation schema version for mechanical engines.
pub const POE_SCHEMA_VERSION: &str = "poe-1.0";

/// Determinism level as defined in Spec §6.3.0.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeterminismLevel {
    D0,
    D1,
    D2,
    D3,
}

impl DeterminismLevel {
    pub fn requires_evidence(self) -> bool {
        matches!(self, Self::D0 | Self::D1)
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::D0 => 0,
            Self::D1 => 1,
            Self::D2 => 2,
            Self::D3 => 3,
        }
    }
}

/// Budget caps for a mechanical engine invocation.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BudgetSpec {
    pub cpu_time_ms: Option<u64>,
    pub wall_time_ms: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub output_bytes: Option<u64>,
}

/// Evidence policy required for non-deterministic modes.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EvidencePolicy {
    pub required: bool,
    pub notes: Option<String>,
}

/// Output specification for an engine operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct OutputSpec {
    pub expected_types: Vec<String>,
    pub max_bytes: Option<u64>,
}

/// PlannedOperation envelope (Spec §11.8.4.1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannedOperation {
    pub schema_version: String,
    pub op_id: Uuid,
    pub engine_id: String,
    pub engine_version_req: Option<String>,
    pub operation: String,
    pub inputs: Vec<ArtifactHandle>,
    pub params: serde_json::Value,
    pub capabilities_requested: Vec<String>,
    #[serde(default)]
    pub capability_profile_id: Option<String>,
    #[serde(default)]
    pub human_consent_obtained: bool,
    pub budget: BudgetSpec,
    pub determinism: DeterminismLevel,
    pub evidence_policy: Option<EvidencePolicy>,
    pub output_spec: OutputSpec,
}

impl PlannedOperation {
    pub fn is_artifact_first(&self) -> bool {
        // All inputs are ArtifactHandle; inline payloads would show up as large params blobs.
        true
    }

    pub fn uses_canonical_schema(&self) -> bool {
        self.schema_version == POE_SCHEMA_VERSION
    }
}

/// Engine execution status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EngineStatus {
    Succeeded,
    Failed,
    Denied,
}

/// Typed engine error payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineError {
    pub code: String,
    pub message: String,
    pub details_ref: Option<ArtifactHandle>,
}

/// Provenance record for EngineResult (Spec §11.8.4.2, §11.8.7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceRecord {
    pub engine_id: String,
    pub engine_version: Option<String>,
    pub implementation: Option<String>,
    pub determinism: DeterminismLevel,
    pub config_hash: Option<String>,
    pub inputs: Vec<ArtifactHandle>,
    pub outputs: Vec<ArtifactHandle>,
    pub capabilities_granted: Vec<String>,
    pub environment: Option<serde_json::Value>,
}

impl ProvenanceRecord {
    pub fn with_engine_id(mut self, engine_id: &str) -> Self {
        self.engine_id = engine_id.to_string();
        self
    }
}

/// EngineResult envelope (Spec §11.8.4.2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineResult {
    pub op_id: Uuid,
    pub status: EngineStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub outputs: Vec<ArtifactHandle>,
    pub evidence: Vec<ArtifactHandle>,
    pub provenance: ProvenanceRecord,
    pub errors: Vec<EngineError>,
    pub logs_ref: Option<ArtifactHandle>,
}

impl EngineResult {
    pub fn success(
        op_id: Uuid,
        outputs: Vec<ArtifactHandle>,
        provenance: ProvenanceRecord,
    ) -> Self {
        let now = Utc::now();
        Self {
            op_id,
            status: EngineStatus::Succeeded,
            started_at: now,
            ended_at: now,
            outputs,
            evidence: Vec::new(),
            provenance,
            errors: Vec::new(),
            logs_ref: None,
        }
    }

    pub fn with_evidence(mut self, evidence: Vec<ArtifactHandle>) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn with_logs(mut self, logs_ref: Option<ArtifactHandle>) -> Self {
        self.logs_ref = logs_ref;
        self
    }
}
