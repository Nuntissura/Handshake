use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use uuid::Uuid;

pub const HBR_VIOLATION_RECEIPT_KIND: &str = "HBR_VIOLATION";
pub const HBR_VIOLATION_SCHEMA_VERSION: u32 = 1;

pub trait ViolationSink {
    fn write_violation(&self, canonical_jsonl: &str) -> Result<(), std::io::Error>;
}

#[derive(Debug, Error)]
pub enum HbrViolationError {
    #[error("HBR_VIOLATION_VALIDATION: {0}")]
    Validation(String),
    #[error("HBR_VIOLATION_SERIALIZE: {source}")]
    Serialize { source: serde_json::Error },
    #[error("HBR_VIOLATION_SINK: {source}")]
    Sink { source: std::io::Error },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HbrViolationRole {
    Orchestrator,
    KernelBuilder,
    Coder,
    WpValidator,
    IntegrationValidator,
    Validator,
    ClassicOrchestrator,
}

impl HbrViolationRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Orchestrator => "ORCHESTRATOR",
            Self::KernelBuilder => "KERNEL_BUILDER",
            Self::Coder => "CODER",
            Self::WpValidator => "WP_VALIDATOR",
            Self::IntegrationValidator => "INTEGRATION_VALIDATOR",
            Self::Validator => "VALIDATOR",
            Self::ClassicOrchestrator => "CLASSIC_ORCHESTRATOR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationPoint {
    Build,
    Handoff,
}

impl EvaluationPoint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Handoff => "handoff",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViolationClass {
    MissingEvidence,
    EvidenceKindMismatch,
    EvidenceProofFailed,
    ApplicabilityMisconfig,
    DowngradeAttempt,
    MatrixSchemaViolation,
}

impl ViolationClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingEvidence => "MISSING_EVIDENCE",
            Self::EvidenceKindMismatch => "EVIDENCE_KIND_MISMATCH",
            Self::EvidenceProofFailed => "EVIDENCE_PROOF_FAILED",
            Self::ApplicabilityMisconfig => "APPLICABILITY_MISCONFIG",
            Self::DowngradeAttempt => "DOWNGRADE_ATTEMPT",
            Self::MatrixSchemaViolation => "MATRIX_SCHEMA_VIOLATION",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbrViolation {
    pub receipt_kind: String,
    pub schema_version: u32,
    pub receipt_uuid: Uuid,
    pub hbr_id: String,
    pub wp_id: String,
    pub mt_id: Option<String>,
    pub role: HbrViolationRole,
    pub evaluation_point: EvaluationPoint,
    pub evidence_pointer: Option<String>,
    pub violation_class: ViolationClass,
    pub emitted_at_utc: DateTime<Utc>,
    pub source_session: Option<String>,
    pub notes: Option<String>,
}

impl HbrViolation {
    pub fn new(
        hbr_id: &str,
        wp_id: &str,
        mt_id: Option<&str>,
        role: HbrViolationRole,
        evaluation_point: EvaluationPoint,
        evidence_pointer: Option<&str>,
        violation_class: ViolationClass,
        source_session: Option<&str>,
        notes: Option<&str>,
    ) -> Self {
        Self {
            receipt_kind: HBR_VIOLATION_RECEIPT_KIND.to_string(),
            schema_version: HBR_VIOLATION_SCHEMA_VERSION,
            receipt_uuid: Uuid::now_v7(),
            hbr_id: hbr_id.to_string(),
            wp_id: wp_id.to_string(),
            mt_id: mt_id.map(str::to_string),
            role,
            evaluation_point,
            evidence_pointer: evidence_pointer.map(str::to_string),
            violation_class,
            emitted_at_utc: Utc::now(),
            source_session: source_session.map(str::to_string),
            notes: notes.map(str::to_string),
        }
    }

    pub fn emit(&self, sink: &dyn ViolationSink) -> Result<(), HbrViolationError> {
        let canonical = self.to_canonical_jsonl()?;
        sink.write_violation(&canonical)
            .map_err(|source| HbrViolationError::Sink { source })
    }

    pub fn to_canonical_jsonl(&self) -> Result<String, HbrViolationError> {
        self.validate()?;
        serde_json::to_string(&self.to_canonical_value())
            .map(|json| format!("{json}\n"))
            .map_err(|source| HbrViolationError::Serialize { source })
    }

    pub fn to_canonical_value(&self) -> Value {
        let mut map = Map::new();
        map.insert(
            "emitted_at_utc".to_string(),
            Value::String(
                self.emitted_at_utc
                    .to_rfc3339_opts(SecondsFormat::Secs, true),
            ),
        );
        map.insert(
            "evaluation_point".to_string(),
            Value::String(self.evaluation_point.as_str().to_string()),
        );
        map.insert(
            "evidence_pointer".to_string(),
            optional_string_value(self.evidence_pointer.as_deref()),
        );
        map.insert("hbr_id".to_string(), Value::String(self.hbr_id.clone()));
        map.insert(
            "mt_id".to_string(),
            optional_string_value(self.mt_id.as_deref()),
        );
        map.insert(
            "notes".to_string(),
            optional_string_value(self.notes.as_deref()),
        );
        map.insert(
            "receipt_kind".to_string(),
            Value::String(HBR_VIOLATION_RECEIPT_KIND.to_string()),
        );
        map.insert(
            "receipt_uuid".to_string(),
            Value::String(self.receipt_uuid.to_string()),
        );
        map.insert(
            "role".to_string(),
            Value::String(self.role.as_str().to_string()),
        );
        map.insert(
            "schema_version".to_string(),
            Value::Number(HBR_VIOLATION_SCHEMA_VERSION.into()),
        );
        map.insert(
            "source_session".to_string(),
            optional_string_value(self.source_session.as_deref()),
        );
        map.insert(
            "violation_class".to_string(),
            Value::String(self.violation_class.as_str().to_string()),
        );
        map.insert("wp_id".to_string(), Value::String(self.wp_id.clone()));
        Value::Object(map)
    }

    fn validate(&self) -> Result<(), HbrViolationError> {
        if self.receipt_kind != HBR_VIOLATION_RECEIPT_KIND {
            return Err(HbrViolationError::Validation(format!(
                "receipt_kind must be {HBR_VIOLATION_RECEIPT_KIND}"
            )));
        }
        if self.schema_version != HBR_VIOLATION_SCHEMA_VERSION {
            return Err(HbrViolationError::Validation(format!(
                "schema_version must be {HBR_VIOLATION_SCHEMA_VERSION}"
            )));
        }
        if self.receipt_uuid.get_version_num() != 7 {
            return Err(HbrViolationError::Validation(
                "receipt_uuid must be UUID v7".to_string(),
            ));
        }
        if !self.hbr_id.starts_with("HBR-") {
            return Err(HbrViolationError::Validation(
                "hbr_id must start with HBR-".to_string(),
            ));
        }
        if self.wp_id.trim().is_empty() {
            return Err(HbrViolationError::Validation(
                "wp_id must be non-empty".to_string(),
            ));
        }
        if self
            .mt_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(HbrViolationError::Validation(
                "mt_id must be null or non-empty".to_string(),
            ));
        }
        if self
            .evidence_pointer
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(HbrViolationError::Validation(
                "evidence_pointer must be null or non-empty".to_string(),
            ));
        }
        Ok(())
    }
}

fn optional_string_value(value: Option<&str>) -> Value {
    value
        .map(|entry| Value::String(entry.to_string()))
        .unwrap_or(Value::Null)
}
