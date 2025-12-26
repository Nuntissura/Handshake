use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
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
    Diagnostic,
    CapabilityAction,
}

impl fmt::Display for FlightRecorderEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlightRecorderEventType::System => write!(f, "system"),
            FlightRecorderEventType::LlmInference => write!(f, "llm_inference"),
            FlightRecorderEventType::Diagnostic => write!(f, "diagnostic"),
            FlightRecorderEventType::CapabilityAction => write!(f, "capability_action"),
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
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvt001System {
    pub component: String,
    pub message: String,
    pub level: String,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvt002LlmInference {
    pub model_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub prompt_hash: Option<String>,
    pub response_hash: Option<String>,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrEvt003Diagnostic {
    pub diagnostic_id: String,
    pub severity: String,
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
