//! MT-198 FR-EVT-* event ID registry + standardization (typed event taxonomy).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrEventId {
    // Mailbox
    MailboxBackpressure,
    MailboxRoutingDenied,
    MailboxLeaseAcquired,
    MailboxLeaseExpired,
    MailboxLeaseTakeover,
    // MT executor
    MtCancelRequested,
    MtCancelForced,
    MtCancelCleanupFailed,
    MtStarved,
    MtExecError,
    Mt015DistillationCandidate,
    // Checkpoint
    CheckpointOverflow,
    CheckpointShutdownForced,
    // Replay
    ReplayStarted,
    ReplayProgress,
    ReplayCompleted,
    ReplayFailed,
    // Restart resume
    RestartResumeStarted,
    RestartResumeSessionResumed,
    RestartResumeSessionRecoveryFailed,
    RestartResumeDbUnavailable,
    RestartResumeCompleted,
    // Span lifecycle
    SpanStarted,
    SpanEnded,
    SpanFailed,
    // Model runtime
    LlmInferStart,
    LlmInferToken,
    LlmInferEnd,
    LlmInferLoraMount,
    LlmInferLoraUnmount,
    LlmInferLoraSwap,
    LlmInferKvEvict,
    LlmInferKvSetQuantization,
    LlmInferKvPrefixCommit,
    LlmInferKvPrefixRestore,
    LlmInferCancel,
    LlmInferCapsLookup,
    LlmInferSpecAccept,
    LlmInferSpecReject,
}

impl FrEventId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MailboxBackpressure => "FR-EVT-MAILBOX-BACKPRESSURE",
            Self::MailboxRoutingDenied => "FR-EVT-MAILBOX-ROUTING-DENIED",
            Self::MailboxLeaseAcquired => "FR-EVT-MAILBOX-LEASE-ACQUIRED",
            Self::MailboxLeaseExpired => "FR-EVT-MAILBOX-LEASE-EXPIRED",
            Self::MailboxLeaseTakeover => "FR-EVT-MAILBOX-LEASE-TAKEOVER",
            Self::MtCancelRequested => "FR-EVT-MT-CANCEL-REQUESTED",
            Self::MtCancelForced => "FR-EVT-MT-CANCEL-FORCED",
            Self::MtCancelCleanupFailed => "FR-EVT-MT-CANCEL-CLEANUP-FAILED",
            Self::MtStarved => "FR-EVT-MT-STARVED",
            Self::MtExecError => "FR-EVT-MT-EXEC-ERROR",
            Self::Mt015DistillationCandidate => "FR-EVT-MT-015",
            Self::CheckpointOverflow => "FR-EVT-CHECKPOINT-OVERFLOW",
            Self::CheckpointShutdownForced => "FR-EVT-CHECKPOINT-SHUTDOWN-FORCED",
            Self::ReplayStarted => "FR-EVT-REPLAY-STARTED",
            Self::ReplayProgress => "FR-EVT-REPLAY-PROGRESS",
            Self::ReplayCompleted => "FR-EVT-REPLAY-COMPLETED",
            Self::ReplayFailed => "FR-EVT-REPLAY-FAILED",
            Self::RestartResumeStarted => "FR-EVT-RESTART-RESUME-STARTED",
            Self::RestartResumeSessionResumed => "FR-EVT-RESTART-RESUME-SESSION-RESUMED",
            Self::RestartResumeSessionRecoveryFailed => {
                "FR-EVT-RESTART-RESUME-SESSION-RECOVERY-FAILED"
            }
            Self::RestartResumeDbUnavailable => "FR-EVT-RESTART-RESUME-DB-UNAVAILABLE",
            Self::RestartResumeCompleted => "FR-EVT-RESTART-RESUME-COMPLETED",
            Self::SpanStarted => "FR-EVT-SPAN-STARTED",
            Self::SpanEnded => "FR-EVT-SPAN-ENDED",
            Self::SpanFailed => "FR-EVT-SPAN-FAILED",
            Self::LlmInferStart => "FR-EVT-LLM-INFER-START",
            Self::LlmInferToken => "FR-EVT-LLM-INFER-TOKEN",
            Self::LlmInferEnd => "FR-EVT-LLM-INFER-END",
            Self::LlmInferLoraMount => "FR-EVT-LLM-INFER-LORA-MOUNT",
            Self::LlmInferLoraUnmount => "FR-EVT-LLM-INFER-LORA-UNMOUNT",
            Self::LlmInferLoraSwap => "FR-EVT-LLM-INFER-LORA-SWAP",
            Self::LlmInferKvEvict => "FR-EVT-LLM-INFER-KV-EVICT",
            Self::LlmInferKvSetQuantization => "FR-EVT-LLM-INFER-KV-SET-QUANTIZATION",
            Self::LlmInferKvPrefixCommit => "FR-EVT-LLM-INFER-KV-PREFIX-COMMIT",
            Self::LlmInferKvPrefixRestore => "FR-EVT-LLM-INFER-KV-PREFIX-RESTORE",
            Self::LlmInferCancel => "FR-EVT-LLM-INFER-CANCEL",
            Self::LlmInferCapsLookup => "FR-EVT-LLM-INFER-CAPS-LOOKUP",
            Self::LlmInferSpecAccept => "FR-EVT-LLM-INFER-SPEC-ACCEPT",
            Self::LlmInferSpecReject => "FR-EVT-LLM-INFER-SPEC-REJECT",
        }
    }

    pub fn all() -> &'static [FrEventId] {
        &[
            Self::MailboxBackpressure,
            Self::MailboxRoutingDenied,
            Self::MailboxLeaseAcquired,
            Self::MailboxLeaseExpired,
            Self::MailboxLeaseTakeover,
            Self::MtCancelRequested,
            Self::MtCancelForced,
            Self::MtCancelCleanupFailed,
            Self::MtStarved,
            Self::MtExecError,
            Self::Mt015DistillationCandidate,
            Self::CheckpointOverflow,
            Self::CheckpointShutdownForced,
            Self::ReplayStarted,
            Self::ReplayProgress,
            Self::ReplayCompleted,
            Self::ReplayFailed,
            Self::RestartResumeStarted,
            Self::RestartResumeSessionResumed,
            Self::RestartResumeSessionRecoveryFailed,
            Self::RestartResumeDbUnavailable,
            Self::RestartResumeCompleted,
            Self::SpanStarted,
            Self::SpanEnded,
            Self::SpanFailed,
            Self::LlmInferStart,
            Self::LlmInferToken,
            Self::LlmInferEnd,
            Self::LlmInferLoraMount,
            Self::LlmInferLoraUnmount,
            Self::LlmInferLoraSwap,
            Self::LlmInferKvEvict,
            Self::LlmInferKvSetQuantization,
            Self::LlmInferKvPrefixCommit,
            Self::LlmInferKvPrefixRestore,
            Self::LlmInferCancel,
            Self::LlmInferCapsLookup,
            Self::LlmInferSpecAccept,
            Self::LlmInferSpecReject,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown FrEventId string: {0}")]
pub struct UnknownEventId(pub String);

impl FrEventId {
    pub fn from_str_id(s: &str) -> Result<Self, UnknownEventId> {
        for &id in Self::all() {
            if id.as_str() == s {
                return Ok(id);
            }
        }
        Err(UnknownEventId(s.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrEventRegistryEntry {
    pub id: String,
    pub kind: String,
    pub subsystem: String,
    pub added_in_wp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrEventRegistry {
    pub schema: String,
    pub version: String,
    pub events: Vec<FrEventRegistryEntry>,
}

impl FrEventRegistry {
    pub fn from_rust_enum() -> Self {
        let events = FrEventId::all()
            .iter()
            .map(|id| FrEventRegistryEntry {
                id: id.as_str().to_string(),
                kind: "emission".to_string(),
                subsystem: subsystem_for(*id).to_string(),
                added_in_wp: "WP-KERNEL-004".to_string(),
            })
            .collect();
        Self {
            schema: "handshake.fr_event_registry@1".to_string(),
            version: "1.0.0".to_string(),
            events,
        }
    }
}

fn subsystem_for(id: FrEventId) -> &'static str {
    match id {
        FrEventId::MailboxBackpressure
        | FrEventId::MailboxRoutingDenied
        | FrEventId::MailboxLeaseAcquired
        | FrEventId::MailboxLeaseExpired
        | FrEventId::MailboxLeaseTakeover => "role_mailbox",
        FrEventId::MtCancelRequested
        | FrEventId::MtCancelForced
        | FrEventId::MtCancelCleanupFailed
        | FrEventId::MtStarved
        | FrEventId::MtExecError
        | FrEventId::Mt015DistillationCandidate => "mt_executor",
        FrEventId::CheckpointOverflow
        | FrEventId::CheckpointShutdownForced
        | FrEventId::ReplayStarted
        | FrEventId::ReplayProgress
        | FrEventId::ReplayCompleted
        | FrEventId::ReplayFailed
        | FrEventId::RestartResumeStarted
        | FrEventId::RestartResumeSessionResumed
        | FrEventId::RestartResumeSessionRecoveryFailed
        | FrEventId::RestartResumeDbUnavailable
        | FrEventId::RestartResumeCompleted => "session_checkpoint",
        FrEventId::SpanStarted | FrEventId::SpanEnded | FrEventId::SpanFailed => "flight_recorder",
        FrEventId::LlmInferStart
        | FrEventId::LlmInferToken
        | FrEventId::LlmInferEnd
        | FrEventId::LlmInferLoraMount
        | FrEventId::LlmInferLoraUnmount
        | FrEventId::LlmInferLoraSwap
        | FrEventId::LlmInferKvEvict
        | FrEventId::LlmInferKvSetQuantization
        | FrEventId::LlmInferKvPrefixCommit
        | FrEventId::LlmInferKvPrefixRestore
        | FrEventId::LlmInferCancel
        | FrEventId::LlmInferCapsLookup
        | FrEventId::LlmInferSpecAccept
        | FrEventId::LlmInferSpecReject => "model_runtime",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_to_string_round_trip() {
        for id in FrEventId::all() {
            let s = id.as_str();
            let back = FrEventId::from_str_id(s).unwrap();
            assert_eq!(*id, back);
        }
    }

    #[test]
    fn unknown_id_returns_error() {
        let r = FrEventId::from_str_id("FR-EVT-UNKNOWN");
        assert!(r.is_err());
    }

    #[test]
    fn registry_has_one_entry_per_variant() {
        let r = FrEventRegistry::from_rust_enum();
        assert_eq!(r.events.len(), FrEventId::all().len());
    }
}
