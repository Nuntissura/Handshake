//! MT-190 SessionCheckpoint primitive + Postgres schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CHECKPOINT_MAX_BYTES: usize = 32_768;
pub const CHECKPOINT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionCheckpointId(pub Uuid);

impl SessionCheckpointId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointStateKind {
    Periodic,
    EventTriggered,
    PreShutdown,
    PostFailure,
}

impl CheckpointStateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Periodic => "periodic",
            Self::EventTriggered => "event_triggered",
            Self::PreShutdown => "pre_shutdown",
            Self::PostFailure => "post_failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCheckpoint {
    pub checkpoint_id: SessionCheckpointId,
    pub session_id: Uuid,
    pub model_session_id: Uuid,
    pub last_event_ledger_seq: i64,
    pub compact_state: serde_json::Value,
    pub state_kind: CheckpointStateKind,
    pub pending_artifacts: Vec<String>,
    pub created_at_utc: DateTime<Utc>,
    pub created_by_process: i32,
    pub schema_version: u16,
}

impl SessionCheckpoint {
    pub fn new(
        session_id: Uuid,
        model_session_id: Uuid,
        last_event_ledger_seq: i64,
        compact_state: serde_json::Value,
        state_kind: CheckpointStateKind,
    ) -> Result<Self, CheckpointError> {
        let bytes = serde_json::to_vec(&compact_state)?;
        if bytes.len() > CHECKPOINT_MAX_BYTES {
            return Err(CheckpointError::CompactStateTooLarge { size: bytes.len() });
        }
        Ok(Self {
            checkpoint_id: SessionCheckpointId::new_v7(),
            session_id,
            model_session_id,
            last_event_ledger_seq,
            compact_state,
            state_kind,
            pending_artifacts: Vec::new(),
            created_at_utc: Utc::now(),
            created_by_process: std::process::id() as i32,
            schema_version: CHECKPOINT_SCHEMA_VERSION,
        })
    }

    pub fn validate_size(&self) -> Result<(), CheckpointError> {
        let bytes = serde_json::to_vec(&self.compact_state)?;
        if bytes.len() > CHECKPOINT_MAX_BYTES {
            return Err(CheckpointError::CompactStateTooLarge { size: bytes.len() });
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error(
        "compact_state too large: {size} bytes > {} bytes max",
        CHECKPOINT_MAX_BYTES
    )]
    CompactStateTooLarge { size: usize },
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_v7() {
        assert_eq!(SessionCheckpointId::new_v7().as_uuid().get_version_num(), 7);
    }

    #[test]
    fn rejects_oversize_state() {
        let big = serde_json::Value::String("x".repeat(40_000));
        let res = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            0,
            big,
            CheckpointStateKind::Periodic,
        );
        assert!(matches!(
            res,
            Err(CheckpointError::CompactStateTooLarge { .. })
        ));
    }

    #[test]
    fn round_trip() {
        let cp = SessionCheckpoint::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            42,
            serde_json::json!({"k": "v"}),
            CheckpointStateKind::EventTriggered,
        )
        .unwrap();
        let s = serde_json::to_string(&cp).unwrap();
        let back: SessionCheckpoint = serde_json::from_str(&s).unwrap();
        assert_eq!(cp, back);
    }
}
