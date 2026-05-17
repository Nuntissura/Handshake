//! `ValidationRun` lifecycle types.
//!
//! A `ValidationRun` is the durable record of one execution of the validation
//! runner against a single candidate. It carries a `ValidationRunStatus`
//! lifecycle (Queued -> Running -> Completed | Aborted) plus enough metadata
//! for receipts to correlate ledger events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::kb003_schemas::SCHEMA_KERNEL_VALIDATION_RUN_V1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationRunStatus {
    Queued,
    Running,
    Completed,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationRun {
    pub schema_version: &'static str,
    pub run_id: Uuid,
    pub candidate_id: String,
    pub session_id: String,
    pub task_id: String,
    pub status: ValidationRunStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Artifact-ref strings (resolved by the artifact bundle in `report.rs`).
    pub artifact_refs: Vec<String>,
    /// MT-049: present when this run is a *replay* of an earlier run.
    /// Carries the original `run_id` so the EventLedger and `ValidationReport`
    /// can link the replay to its source without scanning artifact_refs.
    /// `None` for fresh, non-replay runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_run_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationRunError {
    InvalidTransition { from: ValidationRunStatus, to: ValidationRunStatus },
    EmptyField(&'static str),
}

impl std::fmt::Display for ValidationRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => write!(
                f,
                "invalid ValidationRun transition: {from:?} -> {to:?}"
            ),
            Self::EmptyField(name) => write!(f, "ValidationRun.{name} must not be empty"),
        }
    }
}

impl std::error::Error for ValidationRunError {}

impl ValidationRun {
    pub fn new(
        candidate_id: impl Into<String>,
        session_id: impl Into<String>,
        task_id: impl Into<String>,
    ) -> Result<Self, ValidationRunError> {
        let candidate_id = candidate_id.into();
        if candidate_id.trim().is_empty() {
            return Err(ValidationRunError::EmptyField("candidate_id"));
        }
        let session_id = session_id.into();
        if session_id.trim().is_empty() {
            return Err(ValidationRunError::EmptyField("session_id"));
        }
        let task_id = task_id.into();
        if task_id.trim().is_empty() {
            return Err(ValidationRunError::EmptyField("task_id"));
        }
        Ok(Self {
            schema_version: SCHEMA_KERNEL_VALIDATION_RUN_V1,
            run_id: Uuid::new_v4(),
            candidate_id,
            session_id,
            task_id,
            status: ValidationRunStatus::Queued,
            created_at: Utc::now(),
            completed_at: None,
            artifact_refs: Vec::new(),
            original_run_id: None,
        })
    }

    /// MT-049: construct a replay of an existing run. The new run gets a
    /// fresh `run_id` and `created_at`, but carries `original.run_id` in
    /// `original_run_id` so the EventLedger and projection layers can link
    /// replay to source without scanning artifact_refs.
    ///
    /// The replay copies `candidate_id` from the original (replays MUST target
    /// the same candidate) but accepts fresh `session_id` / `task_id` because
    /// the replay typically happens in a different governed session.
    pub fn replay_of(
        original: &ValidationRun,
        session_id: impl Into<String>,
        task_id: impl Into<String>,
    ) -> Result<Self, ValidationRunError> {
        let session_id = session_id.into();
        if session_id.trim().is_empty() {
            return Err(ValidationRunError::EmptyField("session_id"));
        }
        let task_id = task_id.into();
        if task_id.trim().is_empty() {
            return Err(ValidationRunError::EmptyField("task_id"));
        }
        Ok(Self {
            schema_version: SCHEMA_KERNEL_VALIDATION_RUN_V1,
            run_id: Uuid::new_v4(),
            candidate_id: original.candidate_id.clone(),
            session_id,
            task_id,
            status: ValidationRunStatus::Queued,
            created_at: Utc::now(),
            completed_at: None,
            artifact_refs: Vec::new(),
            original_run_id: Some(original.run_id),
        })
    }

    /// True iff this run is a replay (i.e. carries an `original_run_id`).
    pub fn is_replay(&self) -> bool {
        self.original_run_id.is_some()
    }

    pub fn start(&mut self) -> Result<(), ValidationRunError> {
        if self.status != ValidationRunStatus::Queued {
            return Err(ValidationRunError::InvalidTransition {
                from: self.status.clone(),
                to: ValidationRunStatus::Running,
            });
        }
        self.status = ValidationRunStatus::Running;
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), ValidationRunError> {
        if self.status != ValidationRunStatus::Running {
            return Err(ValidationRunError::InvalidTransition {
                from: self.status.clone(),
                to: ValidationRunStatus::Completed,
            });
        }
        self.status = ValidationRunStatus::Completed;
        self.completed_at = Some(Utc::now());
        Ok(())
    }

    pub fn abort(&mut self) -> Result<(), ValidationRunError> {
        if matches!(
            self.status,
            ValidationRunStatus::Completed | ValidationRunStatus::Aborted
        ) {
            return Err(ValidationRunError::InvalidTransition {
                from: self.status.clone(),
                to: ValidationRunStatus::Aborted,
            });
        }
        self.status = ValidationRunStatus::Aborted;
        self.completed_at = Some(Utc::now());
        Ok(())
    }

    pub fn attach_artifact(&mut self, artifact_ref: impl Into<String>) {
        self.artifact_refs.push(artifact_ref.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_run_starts_queued_and_carries_schema_id() {
        let run = ValidationRun::new("cand-1", "sess-1", "task-1").unwrap();
        assert_eq!(run.status, ValidationRunStatus::Queued);
        assert_eq!(run.schema_version, SCHEMA_KERNEL_VALIDATION_RUN_V1);
        assert!(run.completed_at.is_none());
    }

    #[test]
    fn lifecycle_transitions_are_enforced() {
        let mut run = ValidationRun::new("c", "s", "t").unwrap();
        run.start().unwrap();
        assert_eq!(run.status, ValidationRunStatus::Running);
        run.complete().unwrap();
        assert!(run.completed_at.is_some());
        // Cannot complete twice.
        assert!(run.complete().is_err());
        // Cannot abort after completion.
        assert!(run.abort().is_err());
    }

    #[test]
    fn empty_identifiers_rejected() {
        assert!(ValidationRun::new("", "s", "t").is_err());
        assert!(ValidationRun::new("c", "", "t").is_err());
        assert!(ValidationRun::new("c", "s", "").is_err());
    }

    // MT-049 regression guards.

    #[test]
    fn fresh_run_has_no_original() {
        let run = ValidationRun::new("c", "s", "t").unwrap();
        assert_eq!(run.original_run_id, None);
        assert!(!run.is_replay());
    }

    #[test]
    fn replay_of_carries_original_run_id() {
        let first = ValidationRun::new("cand-A", "sess-1", "task-1").unwrap();
        let replay = ValidationRun::replay_of(&first, "sess-2", "task-2").unwrap();
        assert_eq!(replay.original_run_id, Some(first.run_id));
        assert_ne!(replay.run_id, first.run_id, "replay must mint a fresh run_id");
        assert!(replay.is_replay());
        // Replay copies candidate, accepts fresh session/task.
        assert_eq!(replay.candidate_id, "cand-A");
        assert_eq!(replay.session_id, "sess-2");
        assert_eq!(replay.task_id, "task-2");
        assert_eq!(replay.status, ValidationRunStatus::Queued);
    }

    #[test]
    fn replay_of_rejects_empty_identifiers() {
        let first = ValidationRun::new("c", "s", "t").unwrap();
        assert!(ValidationRun::replay_of(&first, "", "task").is_err());
        assert!(ValidationRun::replay_of(&first, "session", "").is_err());
    }

    #[test]
    fn replay_run_serde_round_trips_original_run_id() {
        let first = ValidationRun::new("c", "s", "t").unwrap();
        let replay = ValidationRun::replay_of(&first, "s2", "t2").unwrap();
        let json = serde_json::to_string(&replay).expect("serialize");
        assert!(
            json.contains("original_run_id"),
            "serde must surface original_run_id: {json}"
        );
        let back: ValidationRun = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.original_run_id, Some(first.run_id));
    }

    #[test]
    fn fresh_run_serde_omits_original_run_id_when_none() {
        // skip_serializing_if keeps the wire form clean for the common case.
        let run = ValidationRun::new("c", "s", "t").unwrap();
        let json = serde_json::to_string(&run).expect("serialize");
        assert!(
            !json.contains("original_run_id"),
            "fresh-run JSON should omit the field: {json}"
        );
    }
}
