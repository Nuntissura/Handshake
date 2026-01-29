use std::{
    fmt,
    sync::{Arc, Mutex},
};

use ::duckdb::Connection as DuckDbConnection;
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
    /// FR-EVT-MT-001..017: Micro-Task Executor events [§2.6.6.8.12]
    MicroTaskLoopStarted,
    MicroTaskIterationStarted,
    MicroTaskIterationComplete,
    MicroTaskComplete,
    MicroTaskEscalated,
    MicroTaskHardGate,
    MicroTaskPauseRequested,
    MicroTaskResumed,
    MicroTaskLoopCompleted,
    MicroTaskLoopFailed,
    MicroTaskLoopCancelled,
    MicroTaskValidation,
    MicroTaskLoraSelection,
    MicroTaskDropBack,
    MicroTaskDistillationCandidate,
    MicroTaskSkipped,
    MicroTaskBlocked,
    /// FR-EVT-GOV-MAILBOX-001: Role mailbox message created [11.5.3]
    GovMailboxMessageCreated,
    /// FR-EVT-GOV-MAILBOX-002: Role mailbox export updated [11.5.3]
    GovMailboxExported,
    /// FR-EVT-GOV-MAILBOX-003: Role mailbox transcription link created [11.5.3]
    GovMailboxTranscribed,
    /// FR-EVT-005: Debug Bundle export lifecycle event [11.5]
    DebugBundleExport,
    /// Governance Pack export lifecycle event [Spec 2.3.10]
    GovernancePackExport,
    /// FR-EVT-RUNTIME-CHAT-101..103: Frontend conversation telemetry [11.5.10]
    RuntimeChatMessageAppended,
    RuntimeChatAns001Validation,
    RuntimeChatSessionClosed,
    /// FR-EVT-DATA-001..015: AI-Ready Data Architecture events [Â§11.5.5]
    DataBronzeCreated,
    DataSilverCreated,
    DataSilverUpdated,
    DataEmbeddingComputed,
    DataEmbeddingModelChanged,
    DataIndexUpdated,
    DataIndexRebuilt,
    DataValidationFailed,
    DataRetrievalExecuted,
    DataContextAssembled,
    DataPollutionAlert,
    DataQualityDegradation,
    DataReembeddingTriggered,
    DataRelationshipExtracted,
    DataGoldenQueryFailed,
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
            FlightRecorderEventType::MicroTaskLoopStarted => write!(f, "micro_task_loop_started"),
            FlightRecorderEventType::MicroTaskIterationStarted => {
                write!(f, "micro_task_iteration_started")
            }
            FlightRecorderEventType::MicroTaskIterationComplete => {
                write!(f, "micro_task_iteration_complete")
            }
            FlightRecorderEventType::MicroTaskComplete => write!(f, "micro_task_complete"),
            FlightRecorderEventType::MicroTaskEscalated => write!(f, "micro_task_escalated"),
            FlightRecorderEventType::MicroTaskHardGate => write!(f, "micro_task_hard_gate"),
            FlightRecorderEventType::MicroTaskPauseRequested => {
                write!(f, "micro_task_pause_requested")
            }
            FlightRecorderEventType::MicroTaskResumed => write!(f, "micro_task_resumed"),
            FlightRecorderEventType::MicroTaskLoopCompleted => {
                write!(f, "micro_task_loop_completed")
            }
            FlightRecorderEventType::MicroTaskLoopFailed => write!(f, "micro_task_loop_failed"),
            FlightRecorderEventType::MicroTaskLoopCancelled => {
                write!(f, "micro_task_loop_cancelled")
            }
            FlightRecorderEventType::MicroTaskValidation => write!(f, "micro_task_validation"),
            FlightRecorderEventType::MicroTaskLoraSelection => {
                write!(f, "micro_task_lora_selection")
            }
            FlightRecorderEventType::MicroTaskDropBack => write!(f, "micro_task_drop_back"),
            FlightRecorderEventType::MicroTaskDistillationCandidate => {
                write!(f, "micro_task_distillation_candidate")
            }
            FlightRecorderEventType::MicroTaskSkipped => write!(f, "micro_task_skipped"),
            FlightRecorderEventType::MicroTaskBlocked => write!(f, "micro_task_blocked"),
            FlightRecorderEventType::GovMailboxMessageCreated => {
                write!(f, "gov_mailbox_message_created")
            }
            FlightRecorderEventType::GovMailboxExported => write!(f, "gov_mailbox_exported"),
            FlightRecorderEventType::GovMailboxTranscribed => write!(f, "gov_mailbox_transcribed"),
            FlightRecorderEventType::DebugBundleExport => write!(f, "debug_bundle_export"),
            FlightRecorderEventType::GovernancePackExport => write!(f, "governance_pack_export"),
            FlightRecorderEventType::RuntimeChatMessageAppended => {
                write!(f, "runtime_chat_message_appended")
            }
            FlightRecorderEventType::RuntimeChatAns001Validation => {
                write!(f, "runtime_chat_ans001_validation")
            }
            FlightRecorderEventType::RuntimeChatSessionClosed => {
                write!(f, "runtime_chat_session_closed")
            }
            FlightRecorderEventType::DataBronzeCreated => write!(f, "data_bronze_created"),
            FlightRecorderEventType::DataSilverCreated => write!(f, "data_silver_created"),
            FlightRecorderEventType::DataSilverUpdated => write!(f, "data_silver_updated"),
            FlightRecorderEventType::DataEmbeddingComputed => write!(f, "data_embedding_computed"),
            FlightRecorderEventType::DataEmbeddingModelChanged => {
                write!(f, "data_embedding_model_changed")
            }
            FlightRecorderEventType::DataIndexUpdated => write!(f, "data_index_updated"),
            FlightRecorderEventType::DataIndexRebuilt => write!(f, "data_index_rebuilt"),
            FlightRecorderEventType::DataValidationFailed => write!(f, "data_validation_failed"),
            FlightRecorderEventType::DataRetrievalExecuted => write!(f, "data_retrieval_executed"),
            FlightRecorderEventType::DataContextAssembled => write!(f, "data_context_assembled"),
            FlightRecorderEventType::DataPollutionAlert => write!(f, "data_pollution_alert"),
            FlightRecorderEventType::DataQualityDegradation => {
                write!(f, "data_quality_degradation")
            }
            FlightRecorderEventType::DataReembeddingTriggered => {
                write!(f, "data_reembedding_triggered")
            }
            FlightRecorderEventType::DataRelationshipExtracted => {
                write!(f, "data_relationship_extracted")
            }
            FlightRecorderEventType::DataGoldenQueryFailed => write!(f, "data_golden_query_failed"),
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
            FlightRecorderEventType::MicroTaskLoopStarted => {
                validate_micro_task_loop_started_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskIterationStarted => {
                validate_micro_task_iteration_started_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskIterationComplete => {
                validate_micro_task_iteration_complete_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskComplete => {
                validate_micro_task_complete_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskEscalated => {
                validate_micro_task_escalated_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskHardGate => {
                validate_micro_task_hard_gate_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskPauseRequested => {
                validate_micro_task_pause_requested_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskResumed => {
                validate_micro_task_resumed_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskLoopCompleted => {
                validate_micro_task_loop_completed_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskLoopFailed => {
                validate_micro_task_loop_failed_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskLoopCancelled => {
                validate_micro_task_loop_cancelled_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskValidation => {
                validate_micro_task_validation_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskLoraSelection => {
                validate_micro_task_lora_selection_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskDropBack => {
                validate_micro_task_drop_back_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskDistillationCandidate => {
                validate_micro_task_distillation_candidate_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskSkipped => {
                validate_micro_task_skipped_payload(&self.payload)
            }
            FlightRecorderEventType::MicroTaskBlocked => {
                validate_micro_task_blocked_payload(&self.payload)
            }
            FlightRecorderEventType::DebugBundleExport => {
                validate_debug_bundle_payload(&self.payload)
            }
            FlightRecorderEventType::GovernancePackExport => {
                validate_governance_pack_export_payload(&self.payload)
            }
            FlightRecorderEventType::RuntimeChatMessageAppended => {
                if self.actor != FlightRecorderActor::System {
                    return Err(RecorderError::InvalidEvent(
                        "runtime_chat_message_appended actor must be system".to_string(),
                    ));
                }
                validate_runtime_chat_message_appended_payload(&self.payload)
            }
            FlightRecorderEventType::RuntimeChatAns001Validation => {
                if self.actor != FlightRecorderActor::System {
                    return Err(RecorderError::InvalidEvent(
                        "runtime_chat_ans001_validation actor must be system".to_string(),
                    ));
                }
                validate_runtime_chat_ans001_validation_payload(&self.payload)
            }
            FlightRecorderEventType::RuntimeChatSessionClosed => {
                if self.actor != FlightRecorderActor::System {
                    return Err(RecorderError::InvalidEvent(
                        "runtime_chat_session_closed actor must be system".to_string(),
                    ));
                }
                validate_runtime_chat_session_closed_payload(&self.payload)
            }
            FlightRecorderEventType::WorkflowRecovery => {
                if self.actor != FlightRecorderActor::System {
                    return Err(RecorderError::InvalidEvent(
                        "workflow_recovery actor must be system".to_string(),
                    ));
                }
                validate_workflow_recovery_payload(&self.payload)
            }
            FlightRecorderEventType::GovMailboxMessageCreated => {
                validate_gov_mailbox_message_created_payload(&self.payload)
            }
            FlightRecorderEventType::GovMailboxExported => {
                validate_gov_mailbox_exported_payload(&self.payload)
            }
            FlightRecorderEventType::GovMailboxTranscribed => {
                validate_gov_mailbox_transcribed_payload(&self.payload)
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
            FlightRecorderEventType::CapabilityAction => {
                validate_capability_action_payload(&self.payload)
            }
            FlightRecorderEventType::DataBronzeCreated => {
                validate_data_bronze_created_payload(&self.payload)
            }
            FlightRecorderEventType::DataSilverCreated => {
                validate_data_silver_created_payload(&self.payload)
            }
            FlightRecorderEventType::DataSilverUpdated => {
                validate_data_silver_updated_payload(&self.payload)
            }
            FlightRecorderEventType::DataEmbeddingComputed => {
                validate_data_embedding_computed_payload(&self.payload)
            }
            FlightRecorderEventType::DataEmbeddingModelChanged => {
                validate_data_embedding_model_changed_payload(&self.payload)
            }
            FlightRecorderEventType::DataIndexUpdated => {
                validate_data_index_updated_payload(&self.payload)
            }
            FlightRecorderEventType::DataIndexRebuilt => {
                validate_data_index_rebuilt_payload(&self.payload)
            }
            FlightRecorderEventType::DataValidationFailed => {
                validate_data_validation_failed_payload(&self.payload)
            }
            FlightRecorderEventType::DataRetrievalExecuted => {
                validate_data_retrieval_executed_payload(&self.payload)
            }
            FlightRecorderEventType::DataContextAssembled => {
                validate_data_context_assembled_payload(&self.payload)
            }
            FlightRecorderEventType::DataPollutionAlert => {
                validate_data_pollution_alert_payload(&self.payload)
            }
            FlightRecorderEventType::DataQualityDegradation => {
                validate_data_quality_degradation_payload(&self.payload)
            }
            FlightRecorderEventType::DataReembeddingTriggered => {
                validate_data_reembedding_triggered_payload(&self.payload)
            }
            FlightRecorderEventType::DataRelationshipExtracted => {
                validate_data_relationship_extracted_payload(&self.payload)
            }
            FlightRecorderEventType::DataGoldenQueryFailed => {
                validate_data_golden_query_failed_payload(&self.payload)
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

fn require_exact_keys(map: &Map<String, Value>, allowed: &[&str]) -> Result<(), RecorderError> {
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(RecorderError::InvalidEvent(format!(
                "unexpected payload field: {key}"
            )));
        }
    }

    for key in allowed {
        require_key(map, key)?;
    }

    Ok(())
}

fn require_allowed_keys(
    map: &Map<String, Value>,
    required: &[&str],
    optional: &[&str],
) -> Result<(), RecorderError> {
    for key in map.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(RecorderError::InvalidEvent(format!(
                "unexpected payload field: {key}"
            )));
        }
    }

    for key in required {
        require_key(map, key)?;
    }

    Ok(())
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

fn require_string_or_null_nonempty(
    map: &Map<String, Value>,
    key: &str,
) -> Result<(), RecorderError> {
    match require_key(map, key)? {
        Value::Null => Ok(()),
        Value::String(value) if !value.trim().is_empty() => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be a non-empty string or null"
        ))),
    }
}

fn require_rfc3339(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    let value = match require_key(map, key)? {
        Value::String(value) if !value.trim().is_empty() => value.trim(),
        _ => {
            return Err(RecorderError::InvalidEvent(format!(
                "payload field {key} must be a non-empty string"
            )))
        }
    };

    chrono::DateTime::parse_from_rfc3339(value).map_err(|e| {
        RecorderError::InvalidEvent(format!("payload field {key} must be RFC3339: {e}"))
    })?;

    Ok(())
}

fn require_uuid_string_non_nil(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    let value = match require_key(map, key)? {
        Value::String(value) if !value.trim().is_empty() => value.trim(),
        _ => {
            return Err(RecorderError::InvalidEvent(format!(
                "payload field {key} must be a non-empty UUID string"
            )))
        }
    };

    let id = Uuid::parse_str(value).map_err(|e| {
        RecorderError::InvalidEvent(format!("payload field {key} must be a UUID string: {e}"))
    })?;
    if id == Uuid::nil() {
        return Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be a non-nil UUID"
        )));
    }

    Ok(())
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

fn require_string_array<'a>(
    map: &'a Map<String, Value>,
    key: &str,
) -> Result<Vec<&'a str>, RecorderError> {
    match require_key(map, key)? {
        Value::Array(items) => {
            if items.is_empty() {
                return Err(RecorderError::InvalidEvent(format!(
                    "payload field {key} must be a non-empty array"
                )));
            }
            let mut out = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                match item {
                    Value::String(value) if !value.trim().is_empty() => out.push(value.as_str()),
                    _ => {
                        return Err(RecorderError::InvalidEvent(format!(
                            "payload field {key}[{idx}] must be a non-empty string"
                        )))
                    }
                }
            }
            Ok(out)
        }
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be an array of strings"
        ))),
    }
}

fn require_string_array_allow_empty<'a>(
    map: &'a Map<String, Value>,
    key: &str,
) -> Result<Vec<&'a str>, RecorderError> {
    match require_key(map, key)? {
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                match item {
                    Value::String(value) if !value.trim().is_empty() => out.push(value.as_str()),
                    _ => {
                        return Err(RecorderError::InvalidEvent(format!(
                            "payload field {key}[{idx}] must be a non-empty string"
                        )))
                    }
                }
            }
            Ok(out)
        }
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be an array of strings"
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
    if map.contains_key("stdout_ref") {
        require_string_or_null(map, "stdout_ref")?;
    }
    if map.contains_key("stderr_ref") {
        require_string_or_null(map, "stderr_ref")?;
    }
    Ok(())
}

fn validate_capability_action_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;

    require_exact_keys(
        map,
        &["capability_id", "actor_id", "job_id", "decision_outcome"],
    )?;
    require_string(map, "capability_id")?;
    require_string(map, "actor_id")?;
    require_string_or_null_nonempty(map, "job_id")?;
    require_string(map, "decision_outcome")?;

    Ok(())
}

fn validate_diagnostic_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "diagnostic_id")?;
    Ok(())
}

// =============================================================================
// FR-EVT-DATA-* (AI-Ready Data Architecture) payload validators [Â§11.5.5]
// =============================================================================

fn validate_data_bronze_created_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "bronze_id",
            "content_type",
            "content_hash",
            "size_bytes",
            "ingestion_source",
            "ingestion_method",
        ],
    )?;

    require_fixed_string(map, "type", "data_bronze_created")?;
    match require_key(map, "bronze_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field bronze_id must be a safe id".to_string(),
            ))
        }
    }
    require_string(map, "content_type")?;
    require_sha256_hex(map, "content_hash")?;
    require_number(map, "size_bytes")?;
    match require_key(map, "ingestion_source")? {
        Value::String(value) => match value.as_str() {
            "user" | "connector" | "system" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field ingestion_source must be one of user|connector|system"
                        .to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field ingestion_source must be a string".to_string(),
            ))
        }
    }
    match require_key(map, "ingestion_method")? {
        Value::String(value) => match value.as_str() {
            "user_create" | "file_import" | "api_ingest" | "connector_sync" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field ingestion_method must be one of user_create|file_import|api_ingest|connector_sync".to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field ingestion_method must be a string".to_string(),
            ))
        }
    }

    Ok(())
}

fn validate_data_silver_created_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "silver_id",
            "bronze_ref",
            "chunk_index",
            "total_chunks",
            "token_count",
            "chunking_strategy",
            "processing_duration_ms",
        ],
    )?;

    require_fixed_string(map, "type", "data_silver_created")?;
    match require_key(map, "silver_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field silver_id must be a safe id".to_string(),
            ))
        }
    }
    match require_key(map, "bronze_ref")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field bronze_ref must be a safe id".to_string(),
            ))
        }
    }
    require_number(map, "chunk_index")?;
    require_number(map, "total_chunks")?;
    require_number(map, "token_count")?;
    require_string(map, "chunking_strategy")?;
    require_number(map, "processing_duration_ms")?;
    Ok(())
}

fn validate_data_silver_updated_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "superseded_silver_id",
            "new_silver_id",
            "bronze_ref",
            "chunking_strategy",
            "processing_duration_ms",
        ],
    )?;

    require_fixed_string(map, "type", "data_silver_updated")?;
    match require_key(map, "superseded_silver_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field superseded_silver_id must be a safe id".to_string(),
            ))
        }
    }
    match require_key(map, "new_silver_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field new_silver_id must be a safe id".to_string(),
            ))
        }
    }
    match require_key(map, "bronze_ref")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field bronze_ref must be a safe id".to_string(),
            ))
        }
    }
    require_string(map, "chunking_strategy")?;
    require_number(map, "processing_duration_ms")?;
    Ok(())
}

fn validate_data_embedding_computed_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "silver_id",
            "model_id",
            "model_version",
            "dimensions",
            "compute_latency_ms",
            "was_truncated",
        ],
    )?;

    require_fixed_string(map, "type", "data_embedding_computed")?;
    match require_key(map, "silver_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field silver_id must be a safe id".to_string(),
            ))
        }
    }
    require_string(map, "model_id")?;
    require_string(map, "model_version")?;
    require_number(map, "dimensions")?;
    require_number(map, "compute_latency_ms")?;
    require_bool(map, "was_truncated")?;
    Ok(())
}

fn validate_data_embedding_model_changed_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "from_model_id",
            "from_model_version",
            "to_model_id",
            "to_model_version",
            "affected_silver_records",
        ],
    )?;

    require_fixed_string(map, "type", "data_embedding_model_changed")?;
    require_string(map, "from_model_id")?;
    require_string(map, "from_model_version")?;
    require_string(map, "to_model_id")?;
    require_string(map, "to_model_version")?;
    require_number(map, "affected_silver_records")?;
    Ok(())
}

fn validate_data_index_updated_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "index_kind",
            "update_kind",
            "records_affected",
            "duration_ms",
        ],
    )?;

    require_fixed_string(map, "type", "data_index_updated")?;
    match require_key(map, "index_kind")? {
        Value::String(value) => match value.as_str() {
            "vector" | "keyword" | "graph" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field index_kind must be one of vector|keyword|graph".to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field index_kind must be a string".to_string(),
            ))
        }
    }
    match require_key(map, "update_kind")? {
        Value::String(value) => match value.as_str() {
            "insert" | "delete" | "update" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field update_kind must be one of insert|delete|update".to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field update_kind must be a string".to_string(),
            ))
        }
    }
    require_number(map, "records_affected")?;
    require_number(map, "duration_ms")?;
    Ok(())
}

fn validate_data_index_rebuilt_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &["type", "index_kind", "records_indexed", "duration_ms"],
    )?;

    require_fixed_string(map, "type", "data_index_rebuilt")?;
    match require_key(map, "index_kind")? {
        Value::String(value) => match value.as_str() {
            "vector" | "keyword" | "graph" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field index_kind must be one of vector|keyword|graph".to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field index_kind must be a string".to_string(),
            ))
        }
    }
    require_number(map, "records_indexed")?;
    require_number(map, "duration_ms")?;
    Ok(())
}

fn validate_data_validation_failed_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &["type", "silver_id", "failed_checks", "validator_version"],
    )?;

    require_fixed_string(map, "type", "data_validation_failed")?;
    match require_key(map, "silver_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field silver_id must be a safe id".to_string(),
            ))
        }
    }
    require_string_array(map, "failed_checks")?;
    require_string(map, "validator_version")?;
    Ok(())
}

fn validate_data_retrieval_executed_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_allowed_keys(
        map,
        &[
            "type",
            "request_id",
            "query_hash",
            "query_intent",
            "weights",
            "results",
            "latency",
            "reranking_used",
        ],
        &[],
    )?;

    require_fixed_string(map, "type", "data_retrieval_executed")?;
    match require_key(map, "request_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field request_id must be a safe id".to_string(),
            ))
        }
    }
    require_sha256_hex(map, "query_hash")?;
    match require_key(map, "query_intent")? {
        Value::String(value) => match value.as_str() {
            "factual_lookup"
            | "code_search"
            | "similarity_search"
            | "relationship_query"
            | "temporal_query" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field query_intent must be one of factual_lookup|code_search|similarity_search|relationship_query|temporal_query".to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field query_intent must be a string".to_string(),
            ))
        }
    }

    let weights = payload_object(require_key(map, "weights")?)?;
    require_exact_keys(weights, &["vector", "keyword", "graph"])?;
    require_number(weights, "vector")?;
    require_number(weights, "keyword")?;
    require_number(weights, "graph")?;

    let results = payload_object(require_key(map, "results")?)?;
    require_exact_keys(
        results,
        &[
            "vector_candidates",
            "keyword_candidates",
            "after_fusion",
            "final_count",
        ],
    )?;
    require_number(results, "vector_candidates")?;
    require_number(results, "keyword_candidates")?;
    require_number(results, "after_fusion")?;
    require_number(results, "final_count")?;

    let latency = payload_object(require_key(map, "latency")?)?;
    require_allowed_keys(
        latency,
        &[
            "embedding_ms",
            "vector_search_ms",
            "keyword_search_ms",
            "total_ms",
        ],
        &["rerank_ms"],
    )?;
    require_number(latency, "embedding_ms")?;
    require_number(latency, "vector_search_ms")?;
    require_number(latency, "keyword_search_ms")?;
    if latency.contains_key("rerank_ms") {
        require_number(latency, "rerank_ms")?;
    }
    require_number(latency, "total_ms")?;
    require_bool(map, "reranking_used")?;
    Ok(())
}

fn validate_data_context_assembled_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "request_id",
            "selected_chunks",
            "context_size_tokens",
        ],
    )?;

    require_fixed_string(map, "type", "data_context_assembled")?;
    match require_key(map, "request_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field request_id must be a safe id".to_string(),
            ))
        }
    }
    require_number(map, "selected_chunks")?;
    require_number(map, "context_size_tokens")?;
    Ok(())
}

fn validate_data_pollution_alert_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "request_id",
            "pollution_score",
            "threshold",
            "metrics",
            "context_size_tokens",
        ],
    )?;

    require_fixed_string(map, "type", "data_pollution_alert")?;
    match require_key(map, "request_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field request_id must be a safe id".to_string(),
            ))
        }
    }
    require_number(map, "pollution_score")?;
    require_number(map, "threshold")?;
    let metrics = payload_object(require_key(map, "metrics")?)?;
    require_exact_keys(
        metrics,
        &[
            "task_relevance_score",
            "drift_score",
            "redundancy_score",
            "stale_content_ratio",
        ],
    )?;
    require_number(metrics, "task_relevance_score")?;
    require_number(metrics, "drift_score")?;
    require_number(metrics, "redundancy_score")?;
    require_number(metrics, "stale_content_ratio")?;
    require_number(map, "context_size_tokens")?;
    Ok(())
}

fn validate_data_quality_degradation_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "metric_name",
            "current_value",
            "threshold",
            "slo_target",
        ],
    )?;

    require_fixed_string(map, "type", "data_quality_degradation")?;
    match require_key(map, "metric_name")? {
        Value::String(value) => match value.as_str() {
            "mrr"
            | "recall_at_10"
            | "ndcg_at_5"
            | "validation_pass_rate"
            | "metadata_completeness"
            | "p95_latency" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field metric_name must be one of mrr|recall_at_10|ndcg_at_5|validation_pass_rate|metadata_completeness|p95_latency".to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field metric_name must be a string".to_string(),
            ))
        }
    }
    require_number(map, "current_value")?;
    require_number(map, "threshold")?;
    require_number(map, "slo_target")?;
    Ok(())
}

fn validate_data_reembedding_triggered_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "model_id",
            "model_version",
            "affected_silver_records",
        ],
    )?;

    require_fixed_string(map, "type", "data_reembedding_triggered")?;
    require_string(map, "model_id")?;
    require_string(map, "model_version")?;
    require_number(map, "affected_silver_records")?;
    Ok(())
}

fn validate_data_relationship_extracted_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_allowed_keys(
        map,
        &["type", "relationship_type", "source_id", "target_id"],
        &["confidence"],
    )?;

    require_fixed_string(map, "type", "data_relationship_extracted")?;
    require_string(map, "relationship_type")?;
    match require_key(map, "source_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field source_id must be a safe id".to_string(),
            ))
        }
    }
    match require_key(map, "target_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field target_id must be a safe id".to_string(),
            ))
        }
    }
    if map.contains_key("confidence") {
        require_number(map, "confidence")?;
    }
    Ok(())
}

fn validate_data_golden_query_failed_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_exact_keys(
        map,
        &[
            "type",
            "query_hash",
            "expected_ids",
            "retrieved_ids",
            "expected_mrr",
            "actual_mrr",
            "regression_from_baseline",
        ],
    )?;

    require_fixed_string(map, "type", "data_golden_query_failed")?;
    require_sha256_hex(map, "query_hash")?;
    require_string_array_allow_empty(map, "expected_ids")?;
    require_string_array_allow_empty(map, "retrieved_ids")?;
    require_number(map, "expected_mrr")?;
    require_number(map, "actual_mrr")?;
    require_bool(map, "regression_from_baseline")?;
    Ok(())
}

fn validate_debug_bundle_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "bundle_id")?;
    require_string(map, "scope")?;
    require_string(map, "redaction_mode")?;
    Ok(())
}

const RUNTIME_CHAT_SCHEMA_VERSION_V0_1: &str = "hsk.fr.runtime_chat@0.1";

fn validate_runtime_chat_role(map: &Map<String, Value>) -> Result<&str, RecorderError> {
    match require_key(map, "role")? {
        Value::String(value) if matches!(value.as_str(), "user" | "assistant") => {
            Ok(value.as_str())
        }
        _ => Err(RecorderError::InvalidEvent(
            "payload field role must be one of: user, assistant".to_string(),
        )),
    }
}

fn validate_runtime_chat_model_role(map: &Map<String, Value>) -> Result<(), RecorderError> {
    match require_key(map, "model_role")? {
        Value::String(value)
            if matches!(
                value.as_str(),
                "frontend" | "orchestrator" | "worker" | "validator"
            ) =>
        {
            Ok(())
        }
        _ => Err(RecorderError::InvalidEvent(
            "payload field model_role must be one of: frontend, orchestrator, worker, validator"
                .to_string(),
        )),
    }
}

fn validate_runtime_chat_message_appended_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_allowed_keys(
        map,
        &[
            "schema_version",
            "event_id",
            "ts_utc",
            "session_id",
            "type",
            "message_id",
            "role",
            "body_sha256",
        ],
        &[
            "job_id",
            "work_packet_id",
            "spec_id",
            "wsid",
            "model_role",
            "ans001_sha256",
        ],
    )?;

    require_fixed_string(map, "schema_version", RUNTIME_CHAT_SCHEMA_VERSION_V0_1)?;
    require_fixed_string(map, "event_id", "FR-EVT-RUNTIME-CHAT-101")?;
    require_rfc3339(map, "ts_utc")?;
    require_uuid_string_non_nil(map, "session_id")?;
    require_fixed_string(map, "type", "runtime_chat_message_appended")?;

    require_uuid_string_non_nil(map, "message_id")?;
    let role = validate_runtime_chat_role(map)?;
    if role == "assistant" {
        validate_runtime_chat_model_role(map)?;
    }
    require_sha256_hex(map, "body_sha256")?;
    if map.contains_key("ans001_sha256") {
        require_sha256_hex(map, "ans001_sha256")?;
    }

    for key in ["job_id", "work_packet_id", "spec_id", "wsid"] {
        if map.contains_key(key) {
            require_string(map, key)?;
        }
    }

    Ok(())
}

fn validate_runtime_chat_ans001_validation_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_allowed_keys(
        map,
        &[
            "schema_version",
            "event_id",
            "ts_utc",
            "session_id",
            "type",
            "message_id",
            "role",
            "model_role",
            "ans001_compliant",
            "violation_clauses",
            "body_sha256",
            "ans001_sha256",
        ],
        &["job_id", "work_packet_id", "spec_id", "wsid"],
    )?;

    require_fixed_string(map, "schema_version", RUNTIME_CHAT_SCHEMA_VERSION_V0_1)?;
    require_fixed_string(map, "event_id", "FR-EVT-RUNTIME-CHAT-102")?;
    require_rfc3339(map, "ts_utc")?;
    require_uuid_string_non_nil(map, "session_id")?;
    require_fixed_string(map, "type", "runtime_chat_ans001_validation")?;

    require_uuid_string_non_nil(map, "message_id")?;
    match validate_runtime_chat_role(map)? {
        "assistant" => {}
        other => {
            return Err(RecorderError::InvalidEvent(format!(
                "payload field role must be \"assistant\" for runtime_chat_ans001_validation (got {other})"
            )))
        }
    }
    validate_runtime_chat_model_role(map)?;
    require_bool(map, "ans001_compliant")?;
    require_string_array_allow_empty(map, "violation_clauses")?;
    require_sha256_hex(map, "body_sha256")?;
    require_sha256_hex(map, "ans001_sha256")?;

    for key in ["job_id", "work_packet_id", "spec_id", "wsid"] {
        if map.contains_key(key) {
            require_string(map, key)?;
        }
    }

    Ok(())
}

fn validate_runtime_chat_session_closed_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_allowed_keys(
        map,
        &["schema_version", "event_id", "ts_utc", "session_id", "type"],
        &["job_id", "work_packet_id", "spec_id", "wsid"],
    )?;

    require_fixed_string(map, "schema_version", RUNTIME_CHAT_SCHEMA_VERSION_V0_1)?;
    require_fixed_string(map, "event_id", "FR-EVT-RUNTIME-CHAT-103")?;
    require_rfc3339(map, "ts_utc")?;
    require_uuid_string_non_nil(map, "session_id")?;
    require_fixed_string(map, "type", "runtime_chat_session_closed")?;

    for key in ["job_id", "work_packet_id", "spec_id", "wsid"] {
        if map.contains_key(key) {
            require_string(map, key)?;
        }
    }

    Ok(())
}

fn validate_governance_pack_export_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "export_id")?;
    require_string(map, "created_at")?;

    match require_key(map, "actor")? {
        Value::String(value) => match value.as_str() {
            "HUMAN_DEV" | "AI_JOB" | "PLUGIN_TOOL" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field actor must be one of HUMAN_DEV|AI_JOB|PLUGIN_TOOL".to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field actor must be a string".to_string(),
            ))
        }
    }

    require_string_or_null(map, "job_id")?;
    require_array(map, "source_entity_refs")?;
    require_array(map, "source_hashes")?;
    require_string(map, "export_format")?;
    require_bool(map, "redactions_applied")?;
    require_string(map, "policy_id")?;
    require_array(map, "output_artifact_handles")?;
    require_array(map, "materialized_paths")?;
    require_array(map, "warnings")?;
    require_array(map, "errors")?;

    // determinism_level must be a strict enum.
    match require_key(map, "determinism_level")? {
        Value::String(value) => {
            match value.as_str() {
                "bitwise" | "structural" | "best_effort" => {}
                _ => return Err(RecorderError::InvalidEvent(
                    "payload field determinism_level must be one of bitwise|structural|best_effort"
                        .to_string(),
                )),
            }
        }
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field determinism_level must be a string".to_string(),
            ))
        }
    }

    // exporter object (engine_id, engine_version, config_hash)
    let exporter = require_key(map, "exporter")?;
    let exporter = payload_object(exporter)?;
    require_string(exporter, "engine_id")?;
    require_string(exporter, "engine_version")?;
    require_sha256_hex(exporter, "config_hash")?;

    // export_target must match ExportTarget::LocalFile { path: PathBuf }
    let target = require_key(map, "export_target")?;
    let target = payload_object(target)?;
    require_fixed_string(target, "type", "local_file")?;
    require_string(target, "path")?;

    // source_entity_refs[] objects
    match require_key(map, "source_entity_refs")? {
        Value::Array(items) if !items.is_empty() => {
            for (idx, item) in items.iter().enumerate() {
                let obj = payload_object(item).map_err(|_| {
                    RecorderError::InvalidEvent(format!(
                        "payload field source_entity_refs[{idx}] must be an object"
                    ))
                })?;
                require_string(obj, "entity_id")?;
                require_string(obj, "entity_kind")?;
            }
        }
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field source_entity_refs must be a non-empty array".to_string(),
            ))
        }
    }

    // source_hashes[] must be sha256 hex strings.
    match require_key(map, "source_hashes")? {
        Value::Array(items) if !items.is_empty() => {
            for (idx, item) in items.iter().enumerate() {
                let value = match item {
                    Value::String(value) => value.trim(),
                    _ => {
                        return Err(RecorderError::InvalidEvent(format!(
                            "payload field source_hashes[{idx}] must be a string"
                        )))
                    }
                };
                if value.len() != 64 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(RecorderError::InvalidEvent(format!(
                        "payload field source_hashes[{idx}] must be a 64-char hex sha256"
                    )));
                }
            }
        }
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field source_hashes must be a non-empty array".to_string(),
            ))
        }
    }

    // output_artifact_handles[] must be non-empty and have (artifact_id, path).
    match require_key(map, "output_artifact_handles")? {
        Value::Array(items) if !items.is_empty() => {
            for (idx, item) in items.iter().enumerate() {
                let obj = payload_object(item).map_err(|_| {
                    RecorderError::InvalidEvent(format!(
                        "payload field output_artifact_handles[{idx}] must be an object"
                    ))
                })?;
                require_string(obj, "artifact_id")?;
                require_string(obj, "path")?;
            }
        }
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field output_artifact_handles must be a non-empty array".to_string(),
            ))
        }
    }

    // materialized_paths[] must be root-relative, normalized, and sorted.
    let mut last: Option<&str> = None;
    match require_key(map, "materialized_paths")? {
        Value::Array(items) if !items.is_empty() => {
            for (idx, item) in items.iter().enumerate() {
                let path = match item {
                    Value::String(value) if !value.trim().is_empty() => value.as_str(),
                    _ => {
                        return Err(RecorderError::InvalidEvent(format!(
                            "payload field materialized_paths[{idx}] must be a non-empty string"
                        )))
                    }
                };

                if path.contains('\\') {
                    return Err(RecorderError::InvalidEvent(format!(
                        "payload field materialized_paths[{idx}] must use '/' separators"
                    )));
                }
                if path.starts_with('/') || path.contains(':') {
                    return Err(RecorderError::InvalidEvent(format!(
                        "payload field materialized_paths[{idx}] must be root-relative"
                    )));
                }
                if path.split('/').any(|c| c == "..") {
                    return Err(RecorderError::InvalidEvent(format!(
                        "payload field materialized_paths[{idx}] must not contain '..'"
                    )));
                }
                if let Some(prev) = last {
                    if path < prev {
                        return Err(RecorderError::InvalidEvent(
                            "payload field materialized_paths must be sorted".to_string(),
                        ));
                    }
                }
                last = Some(path);
            }
        }
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field materialized_paths must be a non-empty array".to_string(),
            ))
        }
    }

    // display_projection_ref is optional (null or object).
    if let Some(value) = map.get("display_projection_ref") {
        if !value.is_null() && !value.is_object() {
            return Err(RecorderError::InvalidEvent(
                "payload field display_projection_ref must be an object or null".to_string(),
            ));
        }
    }

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

fn validate_micro_task_event_base<'a>(
    payload: &'a Value,
    expected_event_type: &str,
    expected_event_name: &str,
) -> Result<&'a Map<String, Value>, RecorderError> {
    let map = payload_object(payload)?;
    require_fixed_string(map, "event_type", expected_event_type)?;
    require_fixed_string(map, "event_name", expected_event_name)?;
    require_string(map, "timestamp")?;
    require_string(map, "trace_id")?;
    require_string(map, "job_id")?;
    require_string(map, "workflow_run_id")?;
    let inner = require_key(map, "payload")?;
    payload_object(inner)
}

fn validate_micro_task_loop_started_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner =
        validate_micro_task_event_base(payload, "FR-EVT-MT-001", "micro_task_loop_started")?;
    require_string(inner, "wp_id")?;
    require_number(inner, "total_mts")?;
    require_array(inner, "mt_ids")?;
    require_array(inner, "execution_waves")?;
    let policy = require_key(inner, "execution_policy")?;
    if !policy.is_object() {
        return Err(RecorderError::InvalidEvent(
            "payload field execution_policy must be an object".to_string(),
        ));
    }
    Ok(())
}

fn validate_micro_task_iteration_started_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner =
        validate_micro_task_event_base(payload, "FR-EVT-MT-002", "micro_task_iteration_started")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    require_number(inner, "iteration")?;
    Ok(())
}

fn validate_micro_task_iteration_complete_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner =
        validate_micro_task_event_base(payload, "FR-EVT-MT-003", "micro_task_iteration_complete")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    require_number(inner, "iteration")?;

    let model = payload_object(require_key(inner, "model")?)?;
    require_string(model, "base")?;
    if let Some(value) = model.get("lora") {
        if !value.is_null() {
            require_string(model, "lora")?;
        }
    }
    if let Some(value) = model.get("lora_version") {
        if !value.is_null() {
            require_string(model, "lora_version")?;
        }
    }
    if let Some(value) = model.get("quantization") {
        if !value.is_null() {
            require_string(model, "quantization")?;
        }
    }
    require_number(model, "context_window")?;

    let execution = payload_object(require_key(inner, "execution")?)?;
    require_number(execution, "tokens_prompt")?;
    require_number(execution, "tokens_completion")?;
    require_number(execution, "duration_ms")?;
    require_number(execution, "escalation_level")?;

    let outcome = payload_object(require_key(inner, "outcome")?)?;
    require_bool(outcome, "claimed_complete")?;
    if let Some(value) = outcome.get("validation_passed") {
        if !value.is_boolean() && !value.is_null() {
            return Err(RecorderError::InvalidEvent(
                "payload field outcome.validation_passed must be a boolean or null".to_string(),
            ));
        }
    }
    match require_key(outcome, "status")? {
        Value::String(value) => match value.as_str() {
            "SUCCESS" | "RETRY" | "ESCALATE" | "BLOCKED" | "SKIPPED" => {}
            _ => {
                return Err(RecorderError::InvalidEvent(
                    "payload field outcome.status must be one of SUCCESS|RETRY|ESCALATE|BLOCKED|SKIPPED"
                        .to_string(),
                ))
            }
        },
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field outcome.status must be a string".to_string(),
            ))
        }
    }
    if let Some(value) = outcome.get("failure_category") {
        if !value.is_null() && !value.is_string() {
            return Err(RecorderError::InvalidEvent(
                "payload field outcome.failure_category must be a string or null".to_string(),
            ));
        }
    }
    if let Some(value) = outcome.get("error_summary") {
        if !value.is_null() && !value.is_string() {
            return Err(RecorderError::InvalidEvent(
                "payload field outcome.error_summary must be a string or null".to_string(),
            ));
        }
    }

    require_string(inner, "context_snapshot_id")?;
    if let Some(value) = inner.get("evidence_artifact_ref") {
        if !value.is_null() && !value.is_object() {
            return Err(RecorderError::InvalidEvent(
                "payload field evidence_artifact_ref must be an object or null".to_string(),
            ));
        }
    }

    Ok(())
}

fn validate_micro_task_complete_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-004", "micro_task_complete")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    Ok(())
}

fn validate_micro_task_escalated_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-005", "micro_task_escalated")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    require_string(inner, "from_model")?;
    if let Some(value) = inner.get("from_lora") {
        if !value.is_null() {
            require_string(inner, "from_lora")?;
        }
    }
    require_number(inner, "from_level")?;
    require_string(inner, "to_model")?;
    if let Some(value) = inner.get("to_lora") {
        if !value.is_null() {
            require_string(inner, "to_lora")?;
        }
    }
    require_number(inner, "to_level")?;
    require_string(inner, "reason")?;
    require_string(inner, "failure_category")?;
    require_number(inner, "iterations_at_previous_level")?;
    let record_ref = require_key(inner, "escalation_record_ref")?;
    if !record_ref.is_object() {
        return Err(RecorderError::InvalidEvent(
            "payload field escalation_record_ref must be an object".to_string(),
        ));
    }
    Ok(())
}

fn validate_micro_task_hard_gate_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-006", "micro_task_hard_gate")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "reason")?;
    Ok(())
}

fn validate_micro_task_pause_requested_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner =
        validate_micro_task_event_base(payload, "FR-EVT-MT-007", "micro_task_pause_requested")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    Ok(())
}

fn validate_micro_task_resumed_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-008", "micro_task_resumed")?;
    require_string(inner, "wp_id")?;
    Ok(())
}

fn validate_micro_task_loop_completed_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner =
        validate_micro_task_event_base(payload, "FR-EVT-MT-009", "micro_task_loop_completed")?;
    require_string(inner, "wp_id")?;
    Ok(())
}

fn validate_micro_task_loop_failed_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-010", "micro_task_loop_failed")?;
    require_string(inner, "wp_id")?;
    Ok(())
}

fn validate_micro_task_loop_cancelled_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner =
        validate_micro_task_event_base(payload, "FR-EVT-MT-011", "micro_task_loop_cancelled")?;
    require_string(inner, "wp_id")?;
    Ok(())
}

fn validate_micro_task_validation_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-012", "micro_task_validation")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    require_number(inner, "iteration")?;
    require_bool(inner, "passed")?;
    Ok(())
}

fn validate_micro_task_lora_selection_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner =
        validate_micro_task_event_base(payload, "FR-EVT-MT-013", "micro_task_lora_selection")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    require_number(inner, "iteration")?;
    require_string(inner, "model_id")?;
    Ok(())
}

fn validate_micro_task_drop_back_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-014", "micro_task_drop_back")?;
    require_string(inner, "wp_id")?;
    require_number(inner, "from_level")?;
    require_number(inner, "to_level")?;
    Ok(())
}

fn validate_micro_task_distillation_candidate_payload(
    payload: &Value,
) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(
        payload,
        "FR-EVT-MT-015",
        "micro_task_distillation_candidate",
    )?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    let candidate_ref = require_key(inner, "candidate_ref")?;
    if !candidate_ref.is_object() {
        return Err(RecorderError::InvalidEvent(
            "payload field candidate_ref must be an object".to_string(),
        ));
    }
    Ok(())
}

fn validate_micro_task_skipped_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-016", "micro_task_skipped")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    Ok(())
}

fn validate_micro_task_blocked_payload(payload: &Value) -> Result<(), RecorderError> {
    let inner = validate_micro_task_event_base(payload, "FR-EVT-MT-017", "micro_task_blocked")?;
    require_string(inner, "wp_id")?;
    require_string(inner, "mt_id")?;
    require_string(inner, "reason")?;
    Ok(())
}

fn validate_editor_edit_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    require_string(map, "editor_surface")?;
    require_array(map, "ops")?;
    Ok(())
}

fn require_sha256_hex(map: &Map<String, Value>, key: &str) -> Result<(), RecorderError> {
    let value = match require_key(map, key)? {
        Value::String(value) if !value.trim().is_empty() => value,
        _ => {
            return Err(RecorderError::InvalidEvent(format!(
                "payload field {key} must be a non-empty string"
            )))
        }
    };

    let value = value.trim();
    if value.len() != 64 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must be a 64-char hex sha256"
        )));
    }

    Ok(())
}

fn require_fixed_string(
    map: &Map<String, Value>,
    key: &str,
    expected: &str,
) -> Result<(), RecorderError> {
    match require_key(map, key)? {
        Value::String(value) if value == expected => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field {key} must equal \"{expected}\""
        ))),
    }
}

fn is_safe_id(value: &str, max_len: usize) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > max_len {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn is_safe_token(value: &str, max_len: usize) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > max_len {
        return false;
    }
    value.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.' || c == '/'
    })
}

fn validate_role_id_string(value: &str) -> Result<(), RecorderError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(RecorderError::InvalidEvent(
            "role id must be a non-empty string".to_string(),
        ));
    }

    match value {
        "operator" | "orchestrator" | "coder" | "validator" => Ok(()),
        _ => {
            let suffix = value
                .strip_prefix("advisory:")
                .ok_or_else(|| RecorderError::InvalidEvent("invalid role id".to_string()))?;
            if is_safe_id(suffix, 64) {
                Ok(())
            } else {
                Err(RecorderError::InvalidEvent(
                    "invalid advisory role id suffix".to_string(),
                ))
            }
        }
    }
}

fn validate_mailbox_message_type(value: &str) -> Result<(), RecorderError> {
    match value {
        "clarification_request"
        | "clarification_response"
        | "scope_risk"
        | "scope_change_proposal"
        | "scope_change_approval"
        | "waiver_proposal"
        | "waiver_approval"
        | "validation_finding"
        | "handoff"
        | "blocker"
        | "tooling_request"
        | "tooling_result"
        | "fyi" => Ok(()),
        _ => Err(RecorderError::InvalidEvent(format!(
            "payload field message_type has invalid value: {value}"
        ))),
    }
}

fn validate_gov_mailbox_message_created_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    let allowed = [
        "type",
        "spec_id",
        "work_packet_id",
        "governance_mode",
        "thread_id",
        "message_id",
        "from_role",
        "to_roles",
        "message_type",
        "body_ref",
        "body_sha256",
        "idempotency_key",
    ];
    require_exact_keys(map, &allowed)?;

    if map.contains_key("body") || map.contains_key("body_text") || map.contains_key("raw_body") {
        return Err(RecorderError::InvalidEvent(
            "forbidden inline body field in mailbox payload".to_string(),
        ));
    }

    require_fixed_string(map, "type", "gov_mailbox_message_created")?;
    require_string_or_null_nonempty(map, "spec_id")?;
    require_string_or_null_nonempty(map, "work_packet_id")?;

    match require_key(map, "governance_mode")? {
        Value::String(value)
            if matches!(value.as_str(), "gov_strict" | "gov_standard" | "gov_light") => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field governance_mode must be one of: gov_strict, gov_standard, gov_light"
                    .to_string(),
            ))
        }
    }

    match require_key(map, "thread_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field thread_id must be a safe id".to_string(),
            ))
        }
    }
    match require_key(map, "message_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field message_id must be a safe id".to_string(),
            ))
        }
    }

    let from_role = match require_key(map, "from_role")? {
        Value::String(value) => value,
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field from_role must be a string".to_string(),
            ))
        }
    };
    validate_role_id_string(from_role)?;

    let to_roles = require_string_array(map, "to_roles")?;
    for role in to_roles {
        validate_role_id_string(role)?;
    }

    let message_type = match require_key(map, "message_type")? {
        Value::String(value) if !value.trim().is_empty() => value.as_str(),
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field message_type must be a non-empty string".to_string(),
            ))
        }
    };
    validate_mailbox_message_type(message_type)?;

    match require_key(map, "body_ref")? {
        Value::String(value) if is_safe_token(value, 512) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field body_ref must be a bounded artifact handle string".to_string(),
            ))
        }
    }
    require_sha256_hex(map, "body_sha256")?;

    match require_key(map, "idempotency_key")? {
        Value::String(value) if is_safe_token(value, 256) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field idempotency_key must be a bounded string token".to_string(),
            ))
        }
    }

    Ok(())
}

fn validate_gov_mailbox_exported_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    let allowed = [
        "type",
        "export_root",
        "export_manifest_sha256",
        "thread_count",
        "message_count",
    ];
    require_exact_keys(map, &allowed)?;

    if map.contains_key("body") || map.contains_key("body_text") || map.contains_key("raw_body") {
        return Err(RecorderError::InvalidEvent(
            "forbidden inline body field in mailbox payload".to_string(),
        ));
    }

    require_fixed_string(map, "type", "gov_mailbox_exported")?;
    require_fixed_string(map, "export_root", "docs/ROLE_MAILBOX/")?;
    require_sha256_hex(map, "export_manifest_sha256")?;

    match require_key(map, "thread_count")? {
        Value::Number(value) if value.as_u64().is_some() => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field thread_count must be an integer".to_string(),
            ))
        }
    }
    match require_key(map, "message_count")? {
        Value::Number(value) if value.as_u64().is_some() => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field message_count must be an integer".to_string(),
            ))
        }
    }

    Ok(())
}

fn validate_gov_mailbox_transcribed_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;
    let allowed = [
        "type",
        "thread_id",
        "message_id",
        "transcription_target_kind",
        "target_ref",
        "target_sha256",
    ];
    require_exact_keys(map, &allowed)?;

    if map.contains_key("body") || map.contains_key("body_text") || map.contains_key("raw_body") {
        return Err(RecorderError::InvalidEvent(
            "forbidden inline body field in mailbox payload".to_string(),
        ));
    }

    require_fixed_string(map, "type", "gov_mailbox_transcribed")?;

    match require_key(map, "thread_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field thread_id must be a safe id".to_string(),
            ))
        }
    }
    match require_key(map, "message_id")? {
        Value::String(value) if is_safe_id(value, 128) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field message_id must be a safe id".to_string(),
            ))
        }
    }

    match require_key(map, "transcription_target_kind")? {
        Value::String(value) if is_safe_token(value, 64) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field transcription_target_kind must be a bounded string".to_string(),
            ))
        }
    }
    match require_key(map, "target_ref")? {
        Value::String(value) if is_safe_token(value, 512) => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field target_ref must be a bounded artifact handle string".to_string(),
            ))
        }
    }
    require_sha256_hex(map, "target_sha256")?;

    Ok(())
}

fn validate_llm_inference_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;

    match require_key(map, "type")? {
        Value::String(value) if value == "llm_inference" => {}
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field type must equal \"llm_inference\"".to_string(),
            ));
        }
    }

    match require_key(map, "trace_id")? {
        Value::String(value) => {
            let trace_id = Uuid::parse_str(value).map_err(|_| {
                RecorderError::InvalidEvent(
                    "payload field trace_id must be a UUID string".to_string(),
                )
            })?;
            if trace_id == Uuid::nil() {
                return Err(RecorderError::InvalidEvent(
                    "payload field trace_id must be a non-nil UUID".to_string(),
                ));
            }
        }
        _ => {
            return Err(RecorderError::InvalidEvent(
                "payload field trace_id must be a UUID string".to_string(),
            ));
        }
    }

    require_string(map, "model_id")?;

    let token_usage = require_key(map, "token_usage")?;
    let token_usage_map = payload_object(token_usage)?;
    require_number(token_usage_map, "prompt_tokens")?;
    require_number(token_usage_map, "completion_tokens")?;
    require_number(token_usage_map, "total_tokens")?;

    if map.contains_key("latency_ms") {
        require_number_or_null(map, "latency_ms")?;
    }
    if map.contains_key("prompt_hash") {
        require_string_or_null(map, "prompt_hash")?;
    }
    if map.contains_key("response_hash") {
        require_string_or_null(map, "response_hash")?;
    }

    // [§2.6.6.7.12] Optional ACE validation sub-object
    // Present for DocSummarize/DocEdit jobs that run through ValidatorPipeline
    if let Some(ace_val) = map.get("ace_validation") {
        validate_ace_validation_payload(ace_val)?;
    }

    Ok(())
}

/// Validate the ace_validation sub-object per §2.6.6.7.12
fn validate_ace_validation_payload(payload: &Value) -> Result<(), RecorderError> {
    let map = payload_object(payload)?;

    // Required fields for ACE validation
    require_string(map, "scope_document_id")?;
    require_string(map, "scope_inputs_hash")?;
    require_string(map, "determinism_mode")?;

    // Candidate/selected arrays
    require_array(map, "candidate_ids")?;
    require_array(map, "candidate_hashes")?;
    require_array(map, "selected_ids")?;
    require_array(map, "selected_hashes")?;

    // Truncation/compaction
    require_bool(map, "truncation_applied")?;
    require_array(map, "truncation_flags")?;
    require_bool(map, "compaction_applied")?;

    // QueryPlan fields
    require_string(map, "query_plan_id")?;
    require_string(map, "query_plan_hash")?;
    require_string(map, "normalized_query_hash")?;

    // RetrievalTrace fields
    require_string(map, "retrieval_trace_id")?;
    require_string(map, "retrieval_trace_hash")?;

    // Rerank metadata (optional, can be null)
    // These are validated as string_or_null since they may be None
    if map.contains_key("rerank_method") {
        require_string_or_null(map, "rerank_method")?;
    }
    if map.contains_key("rerank_inputs_hash") {
        require_string_or_null(map, "rerank_inputs_hash")?;
    }
    if map.contains_key("rerank_outputs_hash") {
        require_string_or_null(map, "rerank_outputs_hash")?;
    }

    // Diversity metadata (optional)
    if map.contains_key("diversity_method") {
        require_string_or_null(map, "diversity_method")?;
    }
    if map.contains_key("diversity_lambda") {
        require_number_or_null(map, "diversity_lambda")?;
    }

    // Cache markers
    require_array(map, "cache_markers")?;

    // Drift flags and degraded mode
    require_array(map, "drift_flags")?;
    require_bool(map, "degraded_mode")?;

    // Phase 2 fields (null for now)
    // context_snapshot_id, context_snapshot_hash can be null
    // artifact_handles is an array (empty for Phase 1)
    require_array(map, "artifact_handles")?;

    // Validation results
    require_array(map, "guards_passed")?;
    require_array(map, "guards_failed")?;
    require_array(map, "violation_codes")?;
    require_string(map, "outcome")?;

    // Model tier and timing
    require_string(map, "model_tier")?;
    require_number(map, "validation_duration_ms")?;

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
pub struct LlmInferenceTokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInferenceEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub trace_id: Uuid,
    pub model_id: String,
    pub token_usage: LlmInferenceTokenUsage,
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
    pub actor_id: String,
    pub job_id: Option<String>,
    pub decision_outcome: String,
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
    pub stdout_ref: Option<String>,
    pub stderr_ref: Option<String>,
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

    /// If backed by DuckDB, returns the shared connection so subsystems can create additional tables
    /// in the existing `data/flight_recorder.db` file.
    fn duckdb_connection(&self) -> Option<Arc<Mutex<DuckDbConnection>>> {
        None
    }

    /// Enforces the 7-day retention policy (purge old events).
    /// Returns the number of events purged.
    async fn enforce_retention(&self) -> Result<u64, RecorderError>;

    /// Lists events based on filter.
    async fn list_events(
        &self,
        filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ace::ArtifactHandle;
    use crate::governance_pack::{
        DeterminismLevel, ExportActor, ExportRecord, ExportTarget, ExporterInfo,
    };
    use crate::storage::EntityRef;
    use serde_json::json;
    use std::path::PathBuf;

    const DUMMY_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn test_governance_pack_export_event_accepts_export_record_payload(
    ) -> Result<(), serde_json::Error> {
        let export_id = Uuid::new_v4();
        let record = ExportRecord {
            export_id,
            created_at: Utc::now(),
            actor: ExportActor::AiJob,
            job_id: None,
            source_entity_refs: vec![EntityRef {
                entity_id: "Handshake_Master_Spec_v02.112.md".to_string(),
                entity_kind: "master_spec".to_string(),
            }],
            source_hashes: vec![DUMMY_SHA256.to_string()],
            display_projection_ref: None,
            export_format: "governance_pack_template_volume".to_string(),
            exporter: ExporterInfo {
                engine_id: "handshake.governance_pack_export".to_string(),
                engine_version: "0.1.0".to_string(),
                config_hash: DUMMY_SHA256.to_string(),
            },
            determinism_level: DeterminismLevel::Bitwise,
            export_target: ExportTarget::LocalFile {
                path: PathBuf::from("C:\\\\export"),
            },
            policy_id: "SAFE_DEFAULT".to_string(),
            redactions_applied: false,
            output_artifact_handles: vec![ArtifactHandle::new(
                Uuid::new_v4(),
                "gov_pack_template_volume".to_string(),
            )],
            materialized_paths: vec!["docs/START_HERE.md".to_string()],
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        let payload = serde_json::to_value(&record)?;
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::GovernancePackExport,
            FlightRecorderActor::Agent,
            Uuid::new_v4(),
            payload,
        );
        assert!(event.validate().is_ok());
        Ok(())
    }

    fn valid_llm_inference_payload() -> Value {
        json!({
            "type": "llm_inference",
            "trace_id": Uuid::new_v4().to_string(),
            "model_id": "llama3.2",
            "token_usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            },
            "latency_ms": null,
            "prompt_hash": null,
            "response_hash": null
        })
    }

    #[test]
    fn test_llm_inference_payload_validation_requires_trace_id_and_model_id() {
        let mut payload = valid_llm_inference_payload();
        assert!(validate_llm_inference_payload(&payload).is_ok());

        let removed = match payload.as_object_mut() {
            Some(map) => map.remove("trace_id"),
            None => {
                unreachable!("payload must be object");
            }
        };
        assert!(removed.is_some());
        assert!(matches!(
            validate_llm_inference_payload(&payload),
            Err(RecorderError::InvalidEvent(_))
        ));

        {
            match payload.as_object_mut() {
                Some(map) => {
                    map.insert("trace_id".to_string(), json!(Uuid::new_v4().to_string()));
                }
                None => {
                    unreachable!("payload must be object");
                }
            }
        }

        let removed = match payload.as_object_mut() {
            Some(map) => map.remove("model_id"),
            None => {
                unreachable!("payload must be object");
            }
        };
        assert!(removed.is_some());
        assert!(matches!(
            validate_llm_inference_payload(&payload),
            Err(RecorderError::InvalidEvent(_))
        ));
    }

    #[test]
    fn test_llm_inference_payload_validation_requires_token_usage_object() {
        let mut payload = valid_llm_inference_payload();
        assert!(validate_llm_inference_payload(&payload).is_ok());

        let removed = match payload.as_object_mut() {
            Some(map) => map.remove("token_usage"),
            None => {
                unreachable!("payload must be object");
            }
        };
        assert!(removed.is_some());
        assert!(matches!(
            validate_llm_inference_payload(&payload),
            Err(RecorderError::InvalidEvent(_))
        ));

        {
            match payload.as_object_mut() {
                Some(map) => {
                    map.insert("token_usage".to_string(), json!({"prompt_tokens": 10}));
                }
                None => {
                    unreachable!("payload must be object");
                }
            }
        }
        assert!(matches!(
            validate_llm_inference_payload(&payload),
            Err(RecorderError::InvalidEvent(_))
        ));
    }

    fn valid_mailbox_message_created_payload() -> Value {
        json!({
            "type": "gov_mailbox_message_created",
            "spec_id": "spec-1",
            "work_packet_id": "WP-1-Role-Mailbox-v1",
            "governance_mode": "gov_strict",
            "thread_id": "550e8400-e29b-41d4-a716-446655440000",
            "message_id": "550e8400-e29b-41d4-a716-446655440001",
            "from_role": "coder",
            "to_roles": ["validator"],
            "message_type": "handoff",
            "body_ref": "artifact:550e8400-e29b-41d4-a716-446655440002:/data/role_mailbox/bodies/abc",
            "body_sha256": DUMMY_SHA256,
            "idempotency_key": "550e8400-e29b-41d4-a716-446655440003",
        })
    }

    #[test]
    fn test_role_mailbox_message_created_payload_strict_validation() {
        let payload = valid_mailbox_message_created_payload();
        assert!(validate_gov_mailbox_message_created_payload(&payload).is_ok());

        // Missing key
        let mut missing = payload.clone();
        if let Some(obj) = missing.as_object_mut() {
            obj.remove("thread_id");
        } else {
            assert!(false, "expected payload to be a JSON object");
        }
        assert!(matches!(
            validate_gov_mailbox_message_created_payload(&missing),
            Err(RecorderError::InvalidEvent(_))
        ));

        // Extra key
        let mut extra = payload.clone();
        if let Some(obj) = extra.as_object_mut() {
            obj.insert("extra".to_string(), json!(1));
        } else {
            assert!(false, "expected payload to be a JSON object");
        }
        assert!(matches!(
            validate_gov_mailbox_message_created_payload(&extra),
            Err(RecorderError::InvalidEvent(_))
        ));

        // Invalid governance_mode enum
        let mut invalid_enum = payload.clone();
        if let Some(obj) = invalid_enum.as_object_mut() {
            obj.insert("governance_mode".to_string(), json!("invalid"));
        } else {
            assert!(false, "expected payload to be a JSON object");
        }
        assert!(matches!(
            validate_gov_mailbox_message_created_payload(&invalid_enum),
            Err(RecorderError::InvalidEvent(_))
        ));

        // Forbidden body field
        let mut forbidden = payload.clone();
        if let Some(obj) = forbidden.as_object_mut() {
            obj.insert("body".to_string(), json!("leak"));
        } else {
            assert!(false, "expected payload to be a JSON object");
        }
        assert!(matches!(
            validate_gov_mailbox_message_created_payload(&forbidden),
            Err(RecorderError::InvalidEvent(_))
        ));
    }

    fn valid_mailbox_exported_payload() -> Value {
        json!({
            "type": "gov_mailbox_exported",
            "export_root": "docs/ROLE_MAILBOX/",
            "export_manifest_sha256": DUMMY_SHA256,
            "thread_count": 0,
            "message_count": 0,
        })
    }

    #[test]
    fn test_role_mailbox_exported_payload_validation() {
        let payload = valid_mailbox_exported_payload();
        assert!(validate_gov_mailbox_exported_payload(&payload).is_ok());

        let mut bad_root = payload.clone();
        if let Some(obj) = bad_root.as_object_mut() {
            obj.insert("export_root".to_string(), json!("docs/ROLE_MAILBOX"));
        } else {
            assert!(false, "expected payload to be a JSON object");
        }
        assert!(matches!(
            validate_gov_mailbox_exported_payload(&bad_root),
            Err(RecorderError::InvalidEvent(_))
        ));
    }

    fn valid_mailbox_transcribed_payload() -> Value {
        json!({
            "type": "gov_mailbox_transcribed",
            "thread_id": "550e8400-e29b-41d4-a716-446655440000",
            "message_id": "550e8400-e29b-41d4-a716-446655440001",
            "transcription_target_kind": "task_packet",
            "target_ref": "artifact:550e8400-e29b-41d4-a716-446655440004:/docs/task_packets/WP-1-Role-Mailbox-v1.md",
            "target_sha256": DUMMY_SHA256,
        })
    }

    #[test]
    fn test_role_mailbox_transcribed_payload_validation() {
        let payload = valid_mailbox_transcribed_payload();
        assert!(validate_gov_mailbox_transcribed_payload(&payload).is_ok());

        let mut bad_sha = payload.clone();
        if let Some(obj) = bad_sha.as_object_mut() {
            obj.insert("target_sha256".to_string(), json!("not-a-sha"));
        } else {
            assert!(false, "expected payload to be a JSON object");
        }
        assert!(matches!(
            validate_gov_mailbox_transcribed_payload(&bad_sha),
            Err(RecorderError::InvalidEvent(_))
        ));
    }
}
