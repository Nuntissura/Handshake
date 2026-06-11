//! MT-094 SourceRepairQueue: typed model for the durable repair queue
//! (`knowledge_ingestion_repair_queue`, 0164).
//!
//! An entry exists per source with an OPEN problem (failed/partial/deferred
//! extraction, missing asset, denied path, stale hash). Lifecycle is
//! terminal-safe (resolved/dead_letter never transition again) and budgeted
//! (attempts vs max_attempts -> dead_letter). The reasons are
//! operator-visible: `reason_class` is machine-typed, `reason_detail` is the
//! JSON payload a no-context model needs to decide the repair action.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::receipts::IngestionErrorClass;
use super::IngestionError;

/// Queue lifecycle state.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepairState {
    Queued,
    Retrying,
    Resolved,
    DeadLetter,
}

impl RepairState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Retrying => "retrying",
            Self::Resolved => "resolved",
            Self::DeadLetter => "dead_letter",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Resolved | Self::DeadLetter)
    }

    pub fn is_open(&self) -> bool {
        !self.is_terminal()
    }
}

impl std::str::FromStr for RepairState {
    type Err = IngestionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "queued" => Ok(Self::Queued),
            "retrying" => Ok(Self::Retrying),
            "resolved" => Ok(Self::Resolved),
            "dead_letter" => Ok(Self::DeadLetter),
            other => Err(IngestionError::Validation(format!(
                "invalid repair state: {other}"
            ))),
        }
    }
}

/// Why a source sits in the repair queue. Superset of the receipt error
/// classes plus queue-specific conditions (MT-094 contract: failed
/// extraction, missing assets, denied paths, stale hashes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepairReason {
    Extraction(IngestionErrorClass),
    MissingAsset,
    DeniedPath,
    StaleHash,
}

impl RepairReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Extraction(class) => class.as_str(),
            Self::MissingAsset => "MISSING_ASSET",
            Self::DeniedPath => "DENIED_PATH",
            Self::StaleHash => "STALE_HASH",
        }
    }
}

impl std::str::FromStr for RepairReason {
    type Err = IngestionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "MISSING_ASSET" => Ok(Self::MissingAsset),
            "DENIED_PATH" => Ok(Self::DeniedPath),
            "STALE_HASH" => Ok(Self::StaleHash),
            other => Ok(Self::Extraction(other.parse()?)),
        }
    }
}

/// Durable row of `knowledge_ingestion_repair_queue`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RepairEntry {
    pub repair_id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub receipt_id: Option<String>,
    pub reason_class: RepairReason,
    pub reason_detail: Value,
    pub state: RepairState,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub resolved_receipt_id: Option<String>,
    pub enqueue_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insert payload for [`RepairEntry`].
#[derive(Clone, Debug)]
pub struct NewRepairEntry {
    pub workspace_id: String,
    pub source_id: String,
    pub receipt_id: Option<String>,
    pub reason_class: RepairReason,
    pub reason_detail: Value,
    pub max_attempts: i32,
    pub enqueue_event_id: Option<String>,
}

/// Outcome of one retry attempt, reported back to the queue.
#[derive(Clone, Debug)]
pub enum RepairAttemptOutcome {
    /// The retry produced a usable extraction; entry resolves.
    Resolved { resolved_receipt_id: String },
    /// The retry failed again; entry re-queues or dead-letters on budget.
    FailedAgain {
        receipt_id: Option<String>,
        reason_detail: Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_round_trip_and_classify_terminality() {
        for state in [
            RepairState::Queued,
            RepairState::Retrying,
            RepairState::Resolved,
            RepairState::DeadLetter,
        ] {
            let parsed: RepairState = state.as_str().parse().expect("round trip");
            assert_eq!(parsed, state);
        }
        assert!(RepairState::Resolved.is_terminal());
        assert!(RepairState::DeadLetter.is_terminal());
        assert!(RepairState::Queued.is_open());
        assert!(RepairState::Retrying.is_open());
    }

    #[test]
    fn reasons_cover_extraction_and_queue_specific_classes() {
        let reason: RepairReason = "NO_TEXT_LAYER".parse().expect("extraction class");
        assert_eq!(
            reason,
            RepairReason::Extraction(IngestionErrorClass::NoTextLayer)
        );
        assert_eq!(reason.as_str(), "NO_TEXT_LAYER");
        for (raw, expected) in [
            ("MISSING_ASSET", RepairReason::MissingAsset),
            ("DENIED_PATH", RepairReason::DeniedPath),
            ("STALE_HASH", RepairReason::StaleHash),
        ] {
            let parsed: RepairReason = raw.parse().expect(raw);
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), raw);
        }
        assert!("NOT_A_REASON".parse::<RepairReason>().is_err());
    }
}
