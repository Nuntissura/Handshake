use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{KernelError, KernelEventType, KernelResult};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KernelTaskRun {
    pub kernel_task_run_id: String,
    pub source: String,
    pub intent_payload: Value,
    pub created_at: DateTime<Utc>,
}

impl KernelTaskRun {
    pub fn new(source: impl Into<String>, intent_payload: Value) -> Self {
        Self {
            kernel_task_run_id: format!("KTR-{}", Uuid::new_v4()),
            source: source.into(),
            intent_payload,
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionRun {
    pub session_run_id: String,
    pub kernel_task_run_id: String,
    pub adapter_id: String,
    pub state: SessionRunState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SessionRun {
    pub fn queued(kernel_task_run_id: impl Into<String>, adapter_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            session_run_id: format!("SR-{}", Uuid::new_v4()),
            kernel_task_run_id: kernel_task_run_id.into(),
            adapter_id: adapter_id.into(),
            state: SessionRunState::Queued,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn transition_to(&self, to: SessionRunState) -> KernelResult<Self> {
        if !SessionBroker::can_transition(self.state, to) {
            return Err(KernelError::InvalidSessionTransition {
                from: self.state.as_str().to_string(),
                to: to.as_str().to_string(),
            });
        }
        let mut next = self.clone();
        next.state = to;
        next.updated_at = Utc::now();
        Ok(next)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionRunState {
    Queued,
    Claimed,
    Running,
    Completed,
    Failed,
    Cancelled,
    BackpressureDelayed,
    RetryScheduled,
    DeadLettered,
}

impl SessionRunState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "QUEUED",
            Self::Claimed => "CLAIMED",
            Self::Running => "RUNNING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
            Self::BackpressureDelayed => "BACKPRESSURE_DELAYED",
            Self::RetryScheduled => "RETRY_SCHEDULED",
            Self::DeadLettered => "DEAD_LETTERED",
        }
    }

    pub fn parse(value: &str) -> KernelResult<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "QUEUED" => Ok(Self::Queued),
            "CLAIMED" => Ok(Self::Claimed),
            "RUNNING" => Ok(Self::Running),
            "COMPLETED" => Ok(Self::Completed),
            "FAILED" => Ok(Self::Failed),
            "CANCELLED" => Ok(Self::Cancelled),
            "BACKPRESSURE_DELAYED" => Ok(Self::BackpressureDelayed),
            "RETRY_SCHEDULED" => Ok(Self::RetryScheduled),
            "DEAD_LETTERED" => Ok(Self::DeadLettered),
            other => Err(KernelError::InvalidSessionTransition {
                from: other.to_string(),
                to: "known session state".to_string(),
            }),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::DeadLettered
        )
    }
}

pub struct SessionBroker;

impl SessionBroker {
    pub fn can_transition(from: SessionRunState, to: SessionRunState) -> bool {
        matches!(
            (from, to),
            (SessionRunState::Queued, SessionRunState::Claimed)
                | (SessionRunState::Queued, SessionRunState::Cancelled)
                | (
                    SessionRunState::Queued,
                    SessionRunState::BackpressureDelayed
                )
                | (SessionRunState::Claimed, SessionRunState::Running)
                | (SessionRunState::Claimed, SessionRunState::Cancelled)
                | (SessionRunState::Claimed, SessionRunState::RetryScheduled)
                | (SessionRunState::Running, SessionRunState::Completed)
                | (SessionRunState::Running, SessionRunState::Failed)
                | (SessionRunState::Running, SessionRunState::Cancelled)
                | (SessionRunState::Failed, SessionRunState::RetryScheduled)
                | (SessionRunState::Failed, SessionRunState::DeadLettered)
                | (
                    SessionRunState::BackpressureDelayed,
                    SessionRunState::Queued
                )
                | (
                    SessionRunState::BackpressureDelayed,
                    SessionRunState::Cancelled
                )
                | (
                    SessionRunState::BackpressureDelayed,
                    SessionRunState::DeadLettered
                )
                | (SessionRunState::RetryScheduled, SessionRunState::Queued)
                | (SessionRunState::RetryScheduled, SessionRunState::Claimed)
                | (
                    SessionRunState::RetryScheduled,
                    SessionRunState::DeadLettered
                )
        )
    }

    pub fn transition_event_type(
        from: SessionRunState,
        to: SessionRunState,
    ) -> KernelResult<KernelEventType> {
        if !Self::can_transition(from, to) {
            return Err(KernelError::InvalidSessionTransition {
                from: from.as_str().to_string(),
                to: to.as_str().to_string(),
            });
        }
        let event_type = match to {
            SessionRunState::Queued => KernelEventType::SessionQueued,
            SessionRunState::Claimed => KernelEventType::SessionClaimed,
            SessionRunState::Running => KernelEventType::SessionStarted,
            SessionRunState::Completed => KernelEventType::SessionCompleted,
            SessionRunState::Failed => KernelEventType::SessionFailed,
            SessionRunState::Cancelled => KernelEventType::SessionCancelled,
            SessionRunState::BackpressureDelayed => KernelEventType::SessionBackpressureDelayed,
            SessionRunState::RetryScheduled => KernelEventType::SessionRetryScheduled,
            SessionRunState::DeadLettered => KernelEventType::SessionDeadLettered,
        };
        Ok(event_type)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KernelSessionLease {
    pub session_run_id: String,
    pub kernel_task_run_id: String,
    pub adapter_id: String,
    pub state: SessionRunState,
    pub claimed_by: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub attempt_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
