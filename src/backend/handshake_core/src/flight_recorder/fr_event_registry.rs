//! MT-198 FR-EVT-* event ID registry + standardization (typed event taxonomy).
//!
//! Authority: cluster X.4 contract
//! `.GOV/task_packets/WP-KERNEL-004-.../MT-198.json`.
//!
//! Surface:
//!  - [`FrEventId`] — exhaustive Rust enum of every registered FR-EVT-* string
//!    identifier emitted by cluster X subsystems plus the LLM-runtime sibling
//!    families that adopted the registry on MT-198 landing (see MT-198
//!    `red_team.minimum_controls` adoption list).
//!  - [`FrEventId::as_str`] / [`FrEventId::from_str_id`] — canonical
//!    round-trippable string mapping. The canonical case is `UPPER-KEBAB-CASE`
//!    after the `FR-EVT-` prefix; lookups are exact (no case folding) so a
//!    typo or alternate case is rejected with a typed [`UnknownEventId`]
//!    error rather than silently mapping to a near-miss variant.
//!  - [`FrEventRegistry`] — wire-format manifest (serialisable to the
//!    `handshake.fr_event_registry@1` schema) that mirrors the Rust enum
//!    byte-for-byte. The matching JSON manifest lives at
//!    `.GOV/roles_shared/records/FR_EVENT_REGISTRY.json` and the alignment
//!    test in `tests/fr_event_registry_tests.rs` asserts the two sides do
//!    not drift.
//!
//! Adopter obligations (per MT-198 red-team controls): MT-182, MT-186,
//! MT-188, MT-189, MT-191, MT-192, MT-193, MT-195 switch every free-form
//! `FR-EVT-*` string literal to `FrEventId::Variant.as_str()` on their own
//! microtask landing. New variants land alongside the adopting microtask
//! and must extend [`FrEventId::all`] + the JSON manifest in the same
//! commit; the alignment test fails CI otherwise.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema id stamped on the JSON manifest. Cross-checked by the alignment
/// test so the manifest cannot drift to a different schema version without
/// updating the Rust constant.
pub const FR_EVENT_REGISTRY_SCHEMA: &str = "handshake.fr_event_registry@1";

/// Manifest version. Bump on a backward-compatible registry update (added
/// variants); bump the schema constant only on a breaking shape change.
pub const FR_EVENT_REGISTRY_VERSION: &str = "1.0.0";

/// Work-packet identifier responsible for landing the registry primitive.
pub const FR_EVENT_REGISTRY_ADDED_IN_WP: &str = "WP-KERNEL-004";

/// Exhaustive enum of every registered FR-EVT-* event id.
///
/// Coverage spans the cluster X emissions listed in the MT-198 contract
/// (`MailboxBackpressure`..`SpanFailed`) plus the LLM runtime sibling
/// families that already adopted the registry on prior MT-196 ship.
///
/// Variants are intentionally non-exhaustive at the type level — adding a
/// new variant is a breaking source change, the alignment test in
/// `tests/fr_event_registry_tests.rs` forces the JSON manifest to update in
/// the same commit, and downstream `match` sites must add a new arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrEventId {
    // ----- role_mailbox (cluster X.1) -----
    MailboxBackpressure,
    MailboxRoutingDenied,
    MailboxLeaseAcquired,
    MailboxLeaseExpired,
    MailboxLeaseTakeover,
    // ----- mt_executor (cluster X.2) -----
    MtCancelRequested,
    MtCancelForced,
    MtCancelCleanupFailed,
    MtStarved,
    MtExecError,
    Mt015DistillationCandidate,
    DistillPiiDetect,
    // ----- session_checkpoint (cluster X.3) -----
    CheckpointOverflow,
    CheckpointShutdownForced,
    ReplayStarted,
    ReplayProgress,
    ReplayCompleted,
    ReplayFailed,
    RestartResumeStarted,
    RestartResumeSessionResumed,
    RestartResumeSessionRecoveryFailed,
    RestartResumeDbUnavailable,
    RestartResumeCompleted,
    // ----- flight_recorder span lifecycle (cluster X.4) -----
    SpanStarted,
    SpanEnded,
    SpanFailed,
    /// Mid-span checkpoint emission for long-running spans; ids reserved
    /// here so MT-199 can emit without further registry churn.
    SpanLifecycleCheckpoint,
    /// Activity span attachment to its parent ModelSessionSpan after the
    /// fact (e.g., resumed via task-local parent context binding).
    SpanLifecycleAttachActivity,
    // ----- model_runtime (cluster X siblings, adopted at registry land) -----
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
    /// Canonical string id. Stable wire format; never change after a
    /// variant ships — add a new variant instead.
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
            Self::DistillPiiDetect => "FR-EVT-DISTILL-PII-DETECT",
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
            Self::SpanLifecycleCheckpoint => "FR-EVT-SPAN-LIFECYCLE-CHECKPOINT",
            Self::SpanLifecycleAttachActivity => "FR-EVT-SPAN-LIFECYCLE-ATTACH-ACTIVITY",
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

    /// Ordered slice of every registered variant. Wire-stable: appending is
    /// allowed; reordering must be paired with a manifest regeneration.
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
            Self::DistillPiiDetect,
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
            Self::SpanLifecycleCheckpoint,
            Self::SpanLifecycleAttachActivity,
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

    /// Reverse lookup: canonical id string -> typed variant. Exact match
    /// only; lower-case, missing prefix, and trailing whitespace all return
    /// `Err(UnknownEventId)`.
    pub fn from_str_id(s: &str) -> Result<Self, UnknownEventId> {
        for &id in Self::all() {
            if id.as_str() == s {
                return Ok(id);
            }
        }
        Err(UnknownEventId(s.to_string()))
    }

    /// Owning subsystem (used by the JSON manifest to group events for
    /// diagnostics-panel filtering).
    pub fn subsystem(self) -> &'static str {
        subsystem_for(self)
    }

    /// Event kind (always `"emission"` today; reserved for future
    /// classes like `"audit"` or `"span_lifecycle"`).
    pub fn kind(self) -> &'static str {
        match self {
            Self::SpanStarted
            | Self::SpanEnded
            | Self::SpanFailed
            | Self::SpanLifecycleCheckpoint
            | Self::SpanLifecycleAttachActivity => "span_lifecycle",
            _ => "emission",
        }
    }

    /// Documented payload field schema for the event id.
    pub fn schema_fields(self) -> &'static [FrEventSchemaField] {
        schema_fields_for(self)
    }
}

impl fmt::Display for FrEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FrEventId {
    type Err = UnknownEventId;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_id(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown FrEventId string: {0}")]
pub struct UnknownEventId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrEventSchemaField {
    pub name: &'static str,
    pub kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrEventRegistryEntrySchemaField {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrEventRegistryEntry {
    pub id: String,
    pub kind: String,
    pub subsystem: String,
    pub added_in_wp: String,
    #[serde(default)]
    pub schema_fields: Vec<FrEventRegistryEntrySchemaField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrEventRegistry {
    pub schema: String,
    pub version: String,
    pub events: Vec<FrEventRegistryEntry>,
}

impl FrEventRegistry {
    /// Build the in-memory registry from the Rust enum. The matching JSON
    /// manifest in `.GOV/roles_shared/records/FR_EVENT_REGISTRY.json` is
    /// generated from this source via the alignment test in
    /// `tests/fr_event_registry_tests.rs`.
    pub fn from_rust_enum() -> Self {
        let events = FrEventId::all()
            .iter()
            .map(|id| {
                let schema_fields = id
                    .schema_fields()
                    .iter()
                    .map(|f| FrEventRegistryEntrySchemaField {
                        name: f.name.to_string(),
                        kind: f.kind.to_string(),
                    })
                    .collect();
                FrEventRegistryEntry {
                    id: id.as_str().to_string(),
                    kind: id.kind().to_string(),
                    subsystem: id.subsystem().to_string(),
                    added_in_wp: FR_EVENT_REGISTRY_ADDED_IN_WP.to_string(),
                    schema_fields,
                }
            })
            .collect();
        Self {
            schema: FR_EVENT_REGISTRY_SCHEMA.to_string(),
            version: FR_EVENT_REGISTRY_VERSION.to_string(),
            events,
        }
    }

    /// Serialise to the canonical 2-space-indented JSON used by the
    /// governance manifest. Trailing newline included so a CI diff
    /// against the on-disk manifest is byte-stable.
    pub fn to_canonical_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("registry is serialisable");
        s.push('\n');
        s
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
        FrEventId::DistillPiiDetect => "distillation",
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
        FrEventId::SpanStarted
        | FrEventId::SpanEnded
        | FrEventId::SpanFailed
        | FrEventId::SpanLifecycleCheckpoint
        | FrEventId::SpanLifecycleAttachActivity => "flight_recorder",
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

fn schema_fields_for(id: FrEventId) -> &'static [FrEventSchemaField] {
    // Minimum documented schema per variant. Adopters extend the
    // payload at emission time but the registry records the
    // diagnostics-panel-displayable shape so filters do not silently
    // break when a sibling MT adds fields.
    match id {
        FrEventId::MailboxBackpressure => &[
            FrEventSchemaField {
                name: "role_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "queue_depth",
                kind: "u64",
            },
            FrEventSchemaField {
                name: "policy",
                kind: "string",
            },
        ],
        FrEventId::MailboxRoutingDenied => &[
            FrEventSchemaField {
                name: "role_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "reason",
                kind: "string",
            },
        ],
        FrEventId::MailboxLeaseAcquired
        | FrEventId::MailboxLeaseExpired
        | FrEventId::MailboxLeaseTakeover => &[
            FrEventSchemaField {
                name: "thread_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "lease_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "role_id",
                kind: "string",
            },
        ],
        FrEventId::MtCancelRequested
        | FrEventId::MtCancelForced
        | FrEventId::MtCancelCleanupFailed => &[
            FrEventSchemaField {
                name: "mt_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "reason",
                kind: "string",
            },
        ],
        FrEventId::MtStarved => &[
            FrEventSchemaField {
                name: "mt_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "wait_ms",
                kind: "i64",
            },
        ],
        FrEventId::MtExecError => &[
            FrEventSchemaField {
                name: "mt_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "error",
                kind: "string",
            },
        ],
        FrEventId::Mt015DistillationCandidate => &[
            FrEventSchemaField {
                name: "wp_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "mt_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "candidate_ref",
                kind: "object",
            },
        ],
        FrEventId::DistillPiiDetect => &[
            FrEventSchemaField {
                name: "turn_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "pii_kinds",
                kind: "string_array",
            },
            FrEventSchemaField {
                name: "severity",
                kind: "string",
            },
        ],
        FrEventId::CheckpointOverflow | FrEventId::CheckpointShutdownForced => &[
            FrEventSchemaField {
                name: "session_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "checkpoint_id",
                kind: "uuid",
            },
        ],
        FrEventId::ReplayStarted
        | FrEventId::ReplayProgress
        | FrEventId::ReplayCompleted
        | FrEventId::ReplayFailed => &[
            FrEventSchemaField {
                name: "session_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "from_seq",
                kind: "i64",
            },
            FrEventSchemaField {
                name: "to_seq",
                kind: "i64",
            },
        ],
        FrEventId::RestartResumeStarted
        | FrEventId::RestartResumeSessionResumed
        | FrEventId::RestartResumeSessionRecoveryFailed
        | FrEventId::RestartResumeDbUnavailable
        | FrEventId::RestartResumeCompleted => &[
            FrEventSchemaField {
                name: "report_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "session_id",
                kind: "uuid",
            },
        ],
        FrEventId::SpanStarted => &[
            FrEventSchemaField {
                name: "span_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "parent_span_id",
                kind: "uuid?",
            },
            FrEventSchemaField {
                name: "model_session_id",
                kind: "uuid?",
            },
            FrEventSchemaField {
                name: "session_id",
                kind: "uuid?",
            },
            FrEventSchemaField {
                name: "activity_kind",
                kind: "string?",
            },
            FrEventSchemaField {
                name: "attributes",
                kind: "object",
            },
        ],
        FrEventId::SpanEnded => &[
            FrEventSchemaField {
                name: "span_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "parent_span_id",
                kind: "uuid?",
            },
            FrEventSchemaField {
                name: "duration_ms",
                kind: "i64",
            },
            FrEventSchemaField {
                name: "is_session_span",
                kind: "bool",
            },
        ],
        FrEventId::SpanFailed => &[
            FrEventSchemaField {
                name: "span_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "parent_span_id",
                kind: "uuid?",
            },
            FrEventSchemaField {
                name: "duration_ms",
                kind: "i64",
            },
            FrEventSchemaField {
                name: "failure_reason",
                kind: "string",
            },
        ],
        FrEventId::SpanLifecycleCheckpoint => &[
            FrEventSchemaField {
                name: "span_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "checkpoint_seq",
                kind: "i64",
            },
            FrEventSchemaField {
                name: "event_ledger_seq",
                kind: "i64",
            },
        ],
        FrEventId::SpanLifecycleAttachActivity => &[
            FrEventSchemaField {
                name: "span_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "parent_span_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "attached_at_utc",
                kind: "rfc3339",
            },
        ],
        FrEventId::LlmInferStart | FrEventId::LlmInferEnd | FrEventId::LlmInferCancel => &[
            FrEventSchemaField {
                name: "model_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "model_session_id",
                kind: "uuid",
            },
        ],
        FrEventId::LlmInferToken => &[
            FrEventSchemaField {
                name: "model_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "model_session_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "token_index",
                kind: "i64",
            },
        ],
        FrEventId::LlmInferLoraMount
        | FrEventId::LlmInferLoraUnmount
        | FrEventId::LlmInferLoraSwap => &[
            FrEventSchemaField {
                name: "model_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "lora_id",
                kind: "string",
            },
        ],
        FrEventId::LlmInferKvEvict
        | FrEventId::LlmInferKvSetQuantization
        | FrEventId::LlmInferKvPrefixCommit
        | FrEventId::LlmInferKvPrefixRestore => &[
            FrEventSchemaField {
                name: "model_session_id",
                kind: "uuid",
            },
            FrEventSchemaField {
                name: "prefix_hash",
                kind: "string",
            },
        ],
        FrEventId::LlmInferCapsLookup => &[
            FrEventSchemaField {
                name: "model_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "capability_id",
                kind: "string",
            },
        ],
        FrEventId::LlmInferSpecAccept | FrEventId::LlmInferSpecReject => &[
            FrEventSchemaField {
                name: "model_id",
                kind: "string",
            },
            FrEventSchemaField {
                name: "draft_tokens",
                kind: "i64",
            },
            FrEventSchemaField {
                name: "accepted_tokens",
                kind: "i64",
            },
        ],
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

    #[test]
    fn fromstr_trait_matches_from_str_id() {
        for id in FrEventId::all() {
            let s = id.as_str();
            let via_trait: FrEventId = s.parse().expect("FromStr should accept canonical id");
            assert_eq!(*id, via_trait);
        }
    }

    #[test]
    fn lowercase_is_not_canonical() {
        // Canonical case is UPPER-KEBAB-CASE after the FR-EVT- prefix;
        // lowercase variants must NOT silently round-trip.
        let lower = "fr-evt-span-started";
        assert!(FrEventId::from_str_id(lower).is_err());
    }

    #[test]
    fn whitespace_is_rejected() {
        // Trailing whitespace must NOT match.
        let padded = "FR-EVT-SPAN-STARTED ";
        assert!(FrEventId::from_str_id(padded).is_err());
    }

    #[test]
    fn display_uses_canonical_form() {
        let s = format!("{}", FrEventId::SpanStarted);
        assert_eq!(s, "FR-EVT-SPAN-STARTED");
    }

    #[test]
    fn ids_have_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for id in FrEventId::all() {
            assert!(seen.insert(id.as_str()), "duplicate id: {}", id.as_str());
        }
    }
}
