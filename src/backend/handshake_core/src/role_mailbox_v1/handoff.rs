//! MT-183 Handoff bundle + announce-back provenance primitive.

use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::families::{AnnounceBackBody, ArtifactPointer, CapabilityGrant, CompletionState};
use super::message::{RoleMailboxMessage, RoleMailboxMessageId};
use super::router::ExecutorKind;
use super::thread::RoleMailboxThreadId;
use crate::role_mailbox::RoleId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptPointer {
    pub transcript_id: String,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxHandoffBundleV1 {
    pub bundle_id: Uuid,
    pub source_thread_id: Uuid,
    pub source_message_id: Uuid,
    pub target_role: RoleId,
    pub target_executor_kind: ExecutorKind,
    pub context_summary: String,
    pub linked_artifacts: Vec<ArtifactPointer>,
    pub transcript_pointer: Option<TranscriptPointer>,
    pub capability_grants: Vec<CapabilityGrant>,
    pub expires_at_utc: Option<DateTime<Utc>>,
    /// sha256 hex digest of canonical JSON of (everything else minus this field
    /// minus created_at_utc).
    pub content_hash: String,
    pub created_at_utc: DateTime<Utc>,
    pub created_by_session: Uuid,
}

impl MailboxHandoffBundleV1 {
    /// Recomputes `content_hash` from the canonical representation. Used both
    /// by `HandoffBundleBuilder::build` and by repository inserts to reject
    /// tampered input.
    pub fn recompute_hash(&self) -> String {
        let canonical = serde_json::json!({
            "bundle_id": self.bundle_id,
            "source_thread_id": self.source_thread_id,
            "source_message_id": self.source_message_id,
            "target_role": self.target_role.to_string(),
            "target_executor_kind": self.target_executor_kind,
            "context_summary": self.context_summary,
            "linked_artifacts": self.linked_artifacts,
            "transcript_pointer": self.transcript_pointer,
            "capability_grants": self.capability_grants,
            "expires_at_utc": self.expires_at_utc.map(postgres_microsecond_precision),
            "created_by_session": self.created_by_session,
        });
        let bytes = serde_json::to_vec(&canonical).expect("canonical bundle serializable");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hex::encode(hasher.finalize())
    }

    pub fn verify_hash(&self) -> bool {
        self.content_hash == self.recompute_hash()
    }
}

fn postgres_microsecond_precision(value: DateTime<Utc>) -> DateTime<Utc> {
    let nanos = value.nanosecond();
    value
        .with_nanosecond(nanos - (nanos % 1_000))
        .expect("nanosecond truncation stays in valid range")
}

// ---------- Typestate builder ----------

/// Marker types for typestate. Each required field has a corresponding state.
mod state {
    pub struct Missing;
    pub struct Set;
}

pub struct HandoffBundleBuilder<SourceThread, SourceMsg, TargetRole, TargetKind, Context> {
    _source_thread: std::marker::PhantomData<SourceThread>,
    _source_msg: std::marker::PhantomData<SourceMsg>,
    _target_role: std::marker::PhantomData<TargetRole>,
    _target_kind: std::marker::PhantomData<TargetKind>,
    _context: std::marker::PhantomData<Context>,
    source_thread_id: Option<RoleMailboxThreadId>,
    source_message_id: Option<RoleMailboxMessageId>,
    target_role: Option<RoleId>,
    target_executor_kind: Option<ExecutorKind>,
    context_summary: Option<String>,
    linked_artifacts: Vec<ArtifactPointer>,
    transcript_pointer: Option<TranscriptPointer>,
    capability_grants: Vec<CapabilityGrant>,
    expires_at_utc: Option<DateTime<Utc>>,
    created_by_session: Option<Uuid>,
}

impl Default
    for HandoffBundleBuilder<
        state::Missing,
        state::Missing,
        state::Missing,
        state::Missing,
        state::Missing,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

impl
    HandoffBundleBuilder<
        state::Missing,
        state::Missing,
        state::Missing,
        state::Missing,
        state::Missing,
    >
{
    pub fn new() -> Self {
        Self {
            _source_thread: std::marker::PhantomData,
            _source_msg: std::marker::PhantomData,
            _target_role: std::marker::PhantomData,
            _target_kind: std::marker::PhantomData,
            _context: std::marker::PhantomData,
            source_thread_id: None,
            source_message_id: None,
            target_role: None,
            target_executor_kind: None,
            context_summary: None,
            linked_artifacts: Vec::new(),
            transcript_pointer: None,
            capability_grants: Vec::new(),
            expires_at_utc: None,
            created_by_session: None,
        }
    }
}

impl<ST, SM, TR, TK, CTX> HandoffBundleBuilder<ST, SM, TR, TK, CTX> {
    pub fn source_thread(
        self,
        thread_id: RoleMailboxThreadId,
    ) -> HandoffBundleBuilder<state::Set, SM, TR, TK, CTX> {
        HandoffBundleBuilder {
            _source_thread: std::marker::PhantomData,
            _source_msg: self._source_msg,
            _target_role: self._target_role,
            _target_kind: self._target_kind,
            _context: self._context,
            source_thread_id: Some(thread_id),
            source_message_id: self.source_message_id,
            target_role: self.target_role,
            target_executor_kind: self.target_executor_kind,
            context_summary: self.context_summary,
            linked_artifacts: self.linked_artifacts,
            transcript_pointer: self.transcript_pointer,
            capability_grants: self.capability_grants,
            expires_at_utc: self.expires_at_utc,
            created_by_session: self.created_by_session,
        }
    }

    pub fn source_message(
        self,
        message_id: RoleMailboxMessageId,
    ) -> HandoffBundleBuilder<ST, state::Set, TR, TK, CTX> {
        HandoffBundleBuilder {
            _source_thread: self._source_thread,
            _source_msg: std::marker::PhantomData,
            _target_role: self._target_role,
            _target_kind: self._target_kind,
            _context: self._context,
            source_thread_id: self.source_thread_id,
            source_message_id: Some(message_id),
            target_role: self.target_role,
            target_executor_kind: self.target_executor_kind,
            context_summary: self.context_summary,
            linked_artifacts: self.linked_artifacts,
            transcript_pointer: self.transcript_pointer,
            capability_grants: self.capability_grants,
            expires_at_utc: self.expires_at_utc,
            created_by_session: self.created_by_session,
        }
    }

    pub fn target_role(self, role: RoleId) -> HandoffBundleBuilder<ST, SM, state::Set, TK, CTX> {
        HandoffBundleBuilder {
            _source_thread: self._source_thread,
            _source_msg: self._source_msg,
            _target_role: std::marker::PhantomData,
            _target_kind: self._target_kind,
            _context: self._context,
            source_thread_id: self.source_thread_id,
            source_message_id: self.source_message_id,
            target_role: Some(role),
            target_executor_kind: self.target_executor_kind,
            context_summary: self.context_summary,
            linked_artifacts: self.linked_artifacts,
            transcript_pointer: self.transcript_pointer,
            capability_grants: self.capability_grants,
            expires_at_utc: self.expires_at_utc,
            created_by_session: self.created_by_session,
        }
    }

    pub fn target_executor_kind(
        self,
        kind: ExecutorKind,
    ) -> HandoffBundleBuilder<ST, SM, TR, state::Set, CTX> {
        HandoffBundleBuilder {
            _source_thread: self._source_thread,
            _source_msg: self._source_msg,
            _target_role: self._target_role,
            _target_kind: std::marker::PhantomData,
            _context: self._context,
            source_thread_id: self.source_thread_id,
            source_message_id: self.source_message_id,
            target_role: self.target_role,
            target_executor_kind: Some(kind),
            context_summary: self.context_summary,
            linked_artifacts: self.linked_artifacts,
            transcript_pointer: self.transcript_pointer,
            capability_grants: self.capability_grants,
            expires_at_utc: self.expires_at_utc,
            created_by_session: self.created_by_session,
        }
    }

    pub fn context_summary(
        self,
        summary: impl Into<String>,
    ) -> HandoffBundleBuilder<ST, SM, TR, TK, state::Set> {
        HandoffBundleBuilder {
            _source_thread: self._source_thread,
            _source_msg: self._source_msg,
            _target_role: self._target_role,
            _target_kind: self._target_kind,
            _context: std::marker::PhantomData,
            source_thread_id: self.source_thread_id,
            source_message_id: self.source_message_id,
            target_role: self.target_role,
            target_executor_kind: self.target_executor_kind,
            context_summary: Some(summary.into()),
            linked_artifacts: self.linked_artifacts,
            transcript_pointer: self.transcript_pointer,
            capability_grants: self.capability_grants,
            expires_at_utc: self.expires_at_utc,
            created_by_session: self.created_by_session,
        }
    }

    pub fn linked_artifacts(mut self, artifacts: Vec<ArtifactPointer>) -> Self {
        self.linked_artifacts = artifacts;
        self
    }

    pub fn transcript(mut self, ptr: TranscriptPointer) -> Self {
        self.transcript_pointer = Some(ptr);
        self
    }

    pub fn capability_grants(mut self, grants: Vec<CapabilityGrant>) -> Self {
        self.capability_grants = grants;
        self
    }

    pub fn expires_at(mut self, ts: DateTime<Utc>) -> Self {
        self.expires_at_utc = Some(ts);
        self
    }

    pub fn created_by_session(mut self, session_id: Uuid) -> Self {
        self.created_by_session = Some(session_id);
        self
    }
}

impl HandoffBundleBuilder<state::Set, state::Set, state::Set, state::Set, state::Set> {
    pub fn build(self) -> MailboxHandoffBundleV1 {
        let mut bundle = MailboxHandoffBundleV1 {
            bundle_id: Uuid::now_v7(),
            source_thread_id: self.source_thread_id.unwrap().as_uuid(),
            source_message_id: self.source_message_id.unwrap().as_uuid(),
            target_role: self.target_role.unwrap(),
            target_executor_kind: self.target_executor_kind.unwrap(),
            context_summary: self.context_summary.unwrap(),
            linked_artifacts: self.linked_artifacts,
            transcript_pointer: self.transcript_pointer,
            capability_grants: self.capability_grants,
            expires_at_utc: self.expires_at_utc,
            content_hash: String::new(),
            created_at_utc: Utc::now(),
            created_by_session: self.created_by_session.unwrap_or_else(Uuid::now_v7),
        };
        bundle.content_hash = bundle.recompute_hash();
        bundle
    }
}

// ---------- Provenance ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceLink {
    pub predecessor_message_id: Uuid,
    pub content_hash: String,
}

/// Maximum provenance-chain depth for `AnnounceBackComposer::verify_chain`.
///
/// Per MT-183 subagent contract red-team coverage: an attacker who can craft
/// arbitrary ProvenanceLink vectors could otherwise force unbounded memory or
/// hash recomputation. The bound is enforced with a typed error so callers
/// cannot silently accept oversized chains.
pub const MAX_PROVENANCE_CHAIN_DEPTH: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChainError {
    #[error("chain broken at message {message_id}: hash mismatch")]
    HashMismatch { message_id: Uuid },
    #[error("empty chain")]
    Empty,
    #[error("missing predecessor for message {message_id}")]
    MissingPredecessor { message_id: Uuid },
    #[error("chain depth {depth} exceeds limit {limit}")]
    DepthExceeded { depth: usize, limit: usize },
    /// MT-183 adversarial: an AnnounceBack carrying a bundle_id whose
    /// referenced HandoffBundle is not present in the verifier's bundle map
    /// (dangling correlation).
    #[error("dangling bundle correlation: bundle {bundle_id} not found")]
    DanglingBundleCorrelation { bundle_id: Uuid },
    /// MT-183 adversarial: an AnnounceBack was composed without a prior
    /// handoff bundle (no `bundle_id`).
    #[error("announce-back without prior handoff bundle reference")]
    MissingBundleReference,
}

pub struct AnnounceBackComposer;

impl AnnounceBackComposer {
    /// Compose an AnnounceBack message body referencing the handoff bundle.
    pub fn compose(
        bundle: &MailboxHandoffBundleV1,
        summary: impl Into<String>,
        artifacts: Vec<ArtifactPointer>,
        completion_state: CompletionState,
        provenance_chain: Vec<ProvenanceLink>,
    ) -> AnnounceBackBody {
        AnnounceBackBody {
            sub_session_id: Some(bundle.created_by_session),
            summary: summary.into(),
            artifacts,
            completion_state,
            provenance_chain,
            bundle_id: Some(bundle.bundle_id),
        }
    }

    /// Walk a chain of messages from announce-back back to the original
    /// delegate_work, asserting each `ProvenanceLink.content_hash` matches the
    /// referenced message's recomputed hash.
    ///
    /// Chain depth is bounded by `MAX_PROVENANCE_CHAIN_DEPTH`; longer chains
    /// fail with `ChainError::DepthExceeded` rather than burning unbounded
    /// CPU on hash recomputation.
    pub fn verify_chain(
        messages_by_id: &std::collections::HashMap<Uuid, RoleMailboxMessage>,
        chain: &[ProvenanceLink],
    ) -> Result<(), ChainError> {
        if chain.is_empty() {
            return Err(ChainError::Empty);
        }
        if chain.len() > MAX_PROVENANCE_CHAIN_DEPTH {
            return Err(ChainError::DepthExceeded {
                depth: chain.len(),
                limit: MAX_PROVENANCE_CHAIN_DEPTH,
            });
        }
        for link in chain {
            let msg = messages_by_id.get(&link.predecessor_message_id).ok_or(
                ChainError::MissingPredecessor {
                    message_id: link.predecessor_message_id,
                },
            )?;
            let recomputed = recompute_message_hash(msg);
            if recomputed != link.content_hash {
                return Err(ChainError::HashMismatch {
                    message_id: link.predecessor_message_id,
                });
            }
        }
        Ok(())
    }

    /// Verify that an `AnnounceBackBody` is properly paired with a known
    /// handoff bundle: (a) the body carries a `bundle_id`, (b) the referenced
    /// bundle exists in `bundles_by_id`, and (c) the bundle's stored
    /// `content_hash` matches its recomputed canonical-JSON hash (defence
    /// against an attacker who pre-tampered the bundle row).
    pub fn verify_announce_back_pairing(
        announce_back: &AnnounceBackBody,
        bundles_by_id: &std::collections::HashMap<Uuid, MailboxHandoffBundleV1>,
    ) -> Result<(), ChainError> {
        let bundle_id = announce_back
            .bundle_id
            .ok_or(ChainError::MissingBundleReference)?;
        let bundle = bundles_by_id
            .get(&bundle_id)
            .ok_or(ChainError::DanglingBundleCorrelation { bundle_id })?;
        if !bundle.verify_hash() {
            return Err(ChainError::HashMismatch {
                message_id: bundle.source_message_id,
            });
        }
        Ok(())
    }
}

pub fn recompute_message_hash(msg: &RoleMailboxMessage) -> String {
    let canonical = serde_json::json!({
        "message_id": msg.message_id.as_uuid(),
        "thread_id": msg.thread_id.as_uuid(),
        "message_type": msg.message_type.as_str(),
        "from_role": msg.from_role.to_string(),
        "to_roles": msg.to_roles.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
        "body": msg.body,
        "parent_message_id": msg.parent_message_id.map(|id| id.as_uuid()),
    });
    let bytes = serde_json::to_vec(&canonical).expect("canonical message serializable");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role_mailbox_v1::message::MessageType;

    #[test]
    fn builder_produces_valid_bundle() {
        let bundle = HandoffBundleBuilder::new()
            .source_thread(RoleMailboxThreadId::new_v7())
            .source_message(RoleMailboxMessageId::new_v7())
            .target_role(RoleId::Coder)
            .target_executor_kind(ExecutorKind::LocalSmallModel)
            .context_summary("context")
            .build();
        assert!(bundle.verify_hash());
    }

    #[test]
    fn tampered_bundle_fails_verify() {
        let mut bundle = HandoffBundleBuilder::new()
            .source_thread(RoleMailboxThreadId::new_v7())
            .source_message(RoleMailboxMessageId::new_v7())
            .target_role(RoleId::Coder)
            .target_executor_kind(ExecutorKind::LocalSmallModel)
            .context_summary("c")
            .build();
        bundle.context_summary = "tampered".to_string();
        assert!(!bundle.verify_hash());
    }

    #[test]
    fn announce_back_compose_carries_bundle_ref() {
        let bundle = HandoffBundleBuilder::new()
            .source_thread(RoleMailboxThreadId::new_v7())
            .source_message(RoleMailboxMessageId::new_v7())
            .target_role(RoleId::Coder)
            .target_executor_kind(ExecutorKind::LocalSmallModel)
            .context_summary("c")
            .build();
        let body = AnnounceBackComposer::compose(
            &bundle,
            "done",
            vec![],
            CompletionState::Completed,
            vec![],
        );
        assert_eq!(body.bundle_id, Some(bundle.bundle_id));
    }

    #[test]
    fn bundle_hash_survives_postgres_microsecond_timestamp_round_trip() {
        let expires_at = chrono::DateTime::parse_from_rfc3339("2026-05-25T06:56:56.123456789Z")
            .expect("fixture timestamp")
            .with_timezone(&Utc);
        let mut round_tripped = HandoffBundleBuilder::new()
            .source_thread(RoleMailboxThreadId::new_v7())
            .source_message(RoleMailboxMessageId::new_v7())
            .target_role(RoleId::Validator)
            .target_executor_kind(ExecutorKind::Validator)
            .context_summary("postgres precision fixture")
            .expires_at(expires_at)
            .build();
        assert!(round_tripped.verify_hash());

        round_tripped.expires_at_utc = Some(
            chrono::DateTime::parse_from_rfc3339("2026-05-25T06:56:56.123456Z")
                .expect("fixture timestamp")
                .with_timezone(&Utc),
        );
        assert!(
            round_tripped.verify_hash(),
            "Postgres stores timestamptz at microsecond precision, so hash canonicalization must match persisted precision"
        );
    }

    #[test]
    fn verify_chain_detects_tamper() {
        let mut msg = RoleMailboxMessage::new(
            RoleMailboxThreadId::new_v7(),
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"k": "v"}),
        );
        let valid_hash = recompute_message_hash(&msg);
        let mut messages = std::collections::HashMap::new();
        messages.insert(msg.message_id.as_uuid(), msg.clone());
        let good_chain = vec![ProvenanceLink {
            predecessor_message_id: msg.message_id.as_uuid(),
            content_hash: valid_hash,
        }];
        assert!(AnnounceBackComposer::verify_chain(&messages, &good_chain).is_ok());
        // Now tamper the message body in the hashmap and verify fail.
        msg.body = serde_json::json!({"k": "tampered"});
        messages.insert(msg.message_id.as_uuid(), msg);
        assert!(matches!(
            AnnounceBackComposer::verify_chain(&messages, &good_chain),
            Err(ChainError::HashMismatch { .. })
        ));
    }
}
