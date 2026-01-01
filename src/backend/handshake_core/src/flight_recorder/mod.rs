use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

pub mod duckdb;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlightRecorderActor {
    Human,
    Agent,
    System,
}

impl fmt::Display for FlightRecorderActor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlightRecorderActor::Human => write!(f, "human"),
            FlightRecorderActor::Agent => write!(f, "agent"),
            FlightRecorderActor::System => write!(f, "system"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlightRecorderEventType {
    System,
    LlmInference,
    TerminalCommand,
    EditorEdit,
    Diagnostic,
    CapabilityAction,
    /// FR-EVT-008: Security violation detected by ACE validators [A2.6.6.7.11]
    SecurityViolation,
    /// FR-EVT-WF-RECOVERY: Workflow recovery initiated [A2.6.1]
    WorkflowRecovery,
    /// FR-EVT-005: Debug Bundle export lifecycle event [11.5]
    DebugBundleExport,
}

impl fmt::Display for FlightRecorderEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlightRecorderEventType::System => write!(f, "system"),
            FlightRecorderEventType::LlmInference => write!(f, "llm_inference"),
            FlightRecorderEventType::TerminalCommand => write!(f, "terminal_command"),
            FlightRecorderEventType::EditorEdit => write!(f, "editor_edit"),
            FlightRecorderEventType::Diagnostic => write!(f, "diagnostic"),
            FlightRecorderEventType::CapabilityAction => write!(f, "capability_action"),
            FlightRecorderEventType::SecurityViolation => write!(f, "security_violation"),
            FlightRecorderEventType::WorkflowRecovery => write!(f, "workflow_recovery"),
            FlightRecorderEventType::DebugBundleExport => write!(f, "debug_bundle_export"),
        }
    }
}

/// Canonical event envelope for Flight Recorder ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightRecorderEvent {
    pub event_id: Uuid,
    pub trace_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor: FlightRecorderActor,
    pub actor_id: String,
    pub event_type: FlightRecorderEventType,
    pub job_id: Option<String>,
    pub workflow_id: Option<String>,
    pub model_id: Option<String>,
    pub wsids: Vec<String>,
    pub activity_span_id: Option<String>,
    pub session_span_id: Option<String>,
    pub capability_id: Option<String>,
    pub policy_decision_id: Option<String>,
    pub payload: Value,
}

impl FlightRecorderEvent {
    pub fn new(
        event_type: FlightRecorderEventType,
        actor: FlightRecorderActor,
        trace_id: Uuid,
        payload: Value,
    ) -> Self {
        let actor_id = actor.to_string();
        Self {
            event_id: Uuid::new_v4(),
            trace_id,
            timestamp: Utc::now(),
            actor,
            actor_id,
            event_type,
            job_id: None,
            workflow_id: None,
            model_id: None,
            wsids: Vec::new(),
            activity_span_id: None,
            session_span_id: None,
            capability_id: None,
            policy_decision_id: None,
            payload,
        }
    }

    pub fn with_job_id(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }

    pub fn with_actor_id(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = actor_id.into();
        self
    }

    pub fn with_workflow_id(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    pub fn with_activity_span(mut self, span: impl Into<String>) -> Self {
        self.activity_span_id = Some(span.into());
        self
    }

    pub fn with_session_span(mut self, span: impl Into<String>) -> Self {
        self.session_span_id = Some(span.into());
        self
    }

    pub fn with_capability(mut self, capability_id: impl Into<String>) -> Self {
        self.capability_id = Some(capability_id.into());
        self
    }

    pub fn with_policy_decision(mut self, policy_decision_id: impl Into<String>) -> Self {
        self.policy_decision_id = Some(policy_decision_id.into());
        self
    }

    pub fn with_wsids(mut self, wsids: Vec<String>) -> Self {
        self.wsids = wsids;
        self
    }

    pub fn validate(&self) -> Result<(), RecorderError> {
        if self.event_id == Uuid::nil() {
            return Err(RecorderError::InvalidEvent(
                "event_id must be a non-nil UUID".to_string(),
            ));
        }
        if self.trace_id == Uuid::nil() {
            return Err(RecorderError::InvalidEvent(
                "trace_id must be a non-nil UUID".to_string(),
            ));
        }
        if self.actor_id.trim().is_empty() {
            return Err(RecorderError::InvalidEvent(
                "actor_id must be present".to_string(),
            ));
        }
        self.validate_payload()?;
        Ok(())
    }

    fn validate_payload(&self) -> Result<(), RecorderError> {
        match self.event_type {
            FlightRecorderEventType::TerminalCommand => {
                validate_terminal_command_payload(&self.payload)
            }
            FlightRecorderEventType::EditorEdit => validate_editor_edit_payload(&self.payload),
            FlightRecorderEventType::Diagnostic => validate_diagnostic_payload(&self.payload),
            FlightRecorderEventType::DebugBundleExport => {
                validate_debug_bundle_payload(&self.payload)
            }
            FlightRecorderEventType::WorkflowRecovery => {
                if self.actor != FlightRecorderActor::System {
                    return Err(RecorderError::InvalidEvent(
                        "workflow_recovery actor must be system".to_string(),
                    ));
                }
                validate_workflow_recovery_payload(&self.payload)
            }
            FlightRecorderEventType::LlmInference => {
                let model_id = self.model_id.as_deref().map(str::trim).unwrap_or("");
                if model_id.is_empty() {
                    return Err(RecorderError::InvalidEvent(
                        "model_id must be present for llm_inference".to_string(),
                    ));
                }
                validate_llm_inference_payload(&self.payload)
            }
            _ => Ok(()),
        }
    }

    /// Normalize all string content in payload to NFC form.
    /// Required by HARDENED_INVARIANTS: Content-Awareness [§11.5].
    /// Prevents Unicode bypass attacks by ensuring consistent text representation.
    pub fn normalize_payload(&mut self) {
        self.payload = normalize_json_value(&self.payload);
        // Also normalize string fields that could contain user-provided content
        self.actor_id = self.actor_id.nfc().collect();
        if let Some(ref job_id) = self.job_id {
            self.job_id = Some(job_id.nfc().collect());
        }
        if let Some(ref workflow_id) = self.workflow_id {
            self.workflow_id = Some(workflow_id.nfc().collect());
        }
        if let Some(ref model_id) = self.model_id {
            self.model_id = Some(model_id.nfc().collect());
        }
        self.wsids = self.wsids.iter().map(|s| s.nfc().collect()).collect();
    }
}

/// Recursively normalize all string values in a JSON Value to NFC form.
fn normalize_json_value(value: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(s.nfc().collect()),
        Value::Array(arr) => Value::Array(arr.iter().map(normalize_json_value).collect()),
        Value::Object(obj) => Value::Object(
            obj.iter()
                .map(|(k, v)| (k.nfc().collect(), normalize_json_value(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn payload_object(payload: &Value) -> Result<&Map<String, Value>, RecorderError> {
    payload
        .as_object()
        .ok_or_else(|| RecorderError::InvalidEvent("payload must be a JSON object".to_string()))
}

fn require_key<'a>(map: &'a Map<String, Value>, key: &str) -> Result<&'a Value, RecorderError> {
    map.get(key)
        .ok_or_else(|| RecorderError::InvalidEvent(format!("missing payload field: {key}")))
}

fn require_string(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    match require_key(map, key)? {
        Value::String(value) if !value.trim().is_empty() => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be a non-empty string"
        ))),
    }
}

fn require_string_or_null(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    match require_key(map, key)? {
        Value::String(_) | Value::Null => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be a string or null"
        ))),
    }
}

fn require_bool(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    match require_key(map, key)? {
        Value::Bool(_) => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be a boolean"
        ))),
    }
}

fn require_number(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    match require_key(map, key)? {
        Value::Number(_) => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be a number"
        ))),
    }
}

fn require_number_or_null(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    match require_key(map, key)? {
        Value::Number(_) | Value::Null => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be a number or null"
        ))),
    }
}

fn require_array(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    match require_key(map, key)? {
        Value::Array(_) => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be an array"
        ))),
    }
}

fn validate_terminal_command_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "command")?;
    require_string(map, "session_id")?;
    require_string_or_null(map, "cwd")?;
    require_number_or_null(map, "exit_code")?;
    require_number_or_null(map, "duration_ms")?;
    require_bool(map, "timed_out")?;
    require_bool(map, "cancelled")?;
    require_number(map, "truncated_bytes")?;
    Ok(())
}

fn validate_diagnostic_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "diagnostic_id")?;
    Ok(())
}

fn validate_debug_bundle_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "bundle_id")?;
    require_string(map, "scope")?;
    require_string(map, "redaction_mode")?;
    Ok(())
}

fn validate_workflow_recovery_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "workflow_run_id")?;
    require_string(map, "from_state")?;
    require_string(map, "to_state")?;
    require_string(map, "reason")?;
    require_string(map, "last_heartbeat_ts")?;
    require_number(map, "threshold_secs")?;
    Ok(())
}

fn validate_editor_edit_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "editor_surface")?;
    require_array(map, "ops")?;
    Ok(())
}

fn validate_llm_inference_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "model_id")?;
    require_number(map, "prompt_tokens")?;
    require_number(map, "completion_tokens")?;
    require_number(map, "total_tokens")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvt001System {
    pub component: String,
    pub message: String,
    pub level: String,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInferenceEvent {
    pub model_id: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub prompt_hash: Option<String>,
    pub response_hash: Option<String>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvt003Diagnostic {
    pub diagnostic_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvt004CapabilityAction {
    pub capability_id: String,
    pub action: String,
    pub outcome: String,
    pub profile_id: Option<String>,
    pub policy_decision_id: Option<String>,
}

/// FR-EVT-008: Security violation event payload [§2.6.6.7.11]
///
/// Emitted when ACE validators detect a security violation such as:
/// - Prompt injection [HSK-ACE-VAL-101]
/// - Cloud leakage [§2.6.6.7.11.5]
/// - Sensitivity violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvt008SecurityViolation {
    /// Type of security violation (prompt_injection, cloud_leakage, etc.)
    pub violation_type: String,
    /// Human-readable description of the violation
    pub description: String,
    /// Source reference where violation was detected (if applicable)
    pub source_id: Option<String>,
    /// The pattern or content that triggered the violation
    pub trigger: String,
    /// Guard/validator that detected the violation
    pub guard_name: String,
    /// Character offset of the detected trigger (if available)
    pub offset: Option<usize>,
    /// Context snippet around the trigger (if available)
    pub context: Option<String>,
    /// Action taken (blocked, poisoned, etc.)
    pub action_taken: String,
    /// Job state transition triggered (e.g., "poisoned")
    pub job_state_transition: Option<String>,
}

/// FR-EVT-WF-RECOVERY: Workflow recovery event payload [§2.6.1]
///
/// Emitted when the system recovers an interrupted workflow at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvtWorkflowRecovery {
    pub workflow_run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub from_state: String,
    pub to_state: String,
    pub reason: String,
    pub last_heartbeat_ts: String,
    pub threshold_secs: u64,
}

/// FR-EVT-001: Terminal command event payload [A10.1.1]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalCommandEvent {
    pub job_id: Option<String>,
    pub model_id: Option<String>,
    pub session_id: Option<String>,
    pub wsids: Vec<String>,
    pub capability_set: Vec<String>,
    pub session_type: String,
    pub human_consent_obtained: bool,
    pub command: String,
    pub cwd: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub cancelled: bool,
    pub truncated_bytes: u64,
    pub capability_id: Option<String>,
    pub redaction_applied: bool,
    pub redacted_output: Option<String>,
}

/// FR-EVT-005: Debug Bundle export payload [11.5]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvt005DebugBundleExport {
    pub bundle_id: String,
    pub scope: String,
    pub redaction_mode: String,
    pub included_job_ids: Vec<String>,
    pub included_diagnostic_ids: Vec<String>,
    pub included_wsids: Vec<String>,
    pub event_count: usize,
    pub missing_evidence: Vec<serde_json::Value>,
}

#[derive(Error, Debug)]
pub enum RecorderError {
    #[error("HSK-400-INVALID-EVENT: Event shape violation: {0}")]
    InvalidEvent(String),
    #[error("HSK-500-DB: Sink error: {0}")]
    SinkError(String),
    #[error("HSK-500-DB: Lock error")]
    LockError,
}

#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub event_id: Option<Uuid>,
    pub job_id: Option<String>,
    pub trace_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait FlightRecorder: Send + Sync {
    /// Records a canonical event. MUST validate shape against FR-EVT schemas.
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError>;

    /// Enforces the 7-day retention policy (purge old events).
    /// Returns the number of events purged.
    async fn enforce_retention(&self) -> Result<u64, RecorderError>;

    /// Lists events based on filter.
    async fn list_events(
        &self,
        filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError>;
}
