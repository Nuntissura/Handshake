//! Declared role evidence. Integrity checks do not authenticate an issuer or grant authority.

use std::fmt;
use handshake_resource_scope::{ActorPrincipalId, ExactResourceScopeAttribution};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentSha256([u8; 32]);

impl ContentSha256 {
    pub fn from_hex(value: &str) -> Result<Self, RoleError> {
        if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(RoleError::InvalidField { field: "sha256" });
        }
        let mut bytes = [0; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let digit = |b: u8| if b.is_ascii_digit() { b - b'0' } else { b - b'a' + 10 };
            bytes[index] = digit(pair[0]) * 16 + digit(pair[1]);
        }
        Ok(Self(bytes))
    }

    pub fn to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut result = String::with_capacity(64);
        for byte in self.0 {
            result.push(HEX[(byte >> 4) as usize] as char);
            result.push(HEX[(byte & 15) as usize] as char);
        }
        result
    }
}

impl Serialize for ContentSha256 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}
impl<'de> Deserialize<'de> for ContentSha256 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::from_hex(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

macro_rules! evidence_ref {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(String);
        impl $name {
            pub fn parse(value: &str) -> Result<Self, RoleError> {
                let suffix = value.strip_prefix($prefix).ok_or(RoleError::InvalidField { field: stringify!($name) })?;
                crate::canonical::validate_string(suffix, stringify!($name))?;
                Ok(Self(value.to_owned()))
            }
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
        }
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> { serializer.serialize_str(&self.0) }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}
evidence_ref!(RoleEvidenceRef, "eventledger://role-authority/");
evidence_ref!(DesignationRef, "eventledger://validator-designation/");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionKey { pub action_id: String, pub request_id: String, pub idempotency_key: String }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunBinding {
    pub run_id: String, pub trace_id: String, pub coordinator_session_id: String,
    pub owner_session: String, pub work_packet_id: String, pub micro_task_id: String,
    pub task_board_id: String, pub locus_binding_ref: String, pub launch_plan_hash: ContentSha256,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSubject { pub artifact_ref: String, pub manifest_ref: String, pub version: String, pub sha256: ContentSha256 }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedBase {
    pub aggregate_id: String, pub aggregate_version: u64, pub base_ref: Option<String>,
    pub base_vector_ref: Option<String>, pub schema_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionBinding {
    pub policy_ref: String, pub authority_class: String, pub replay_policy_ref: String,
    pub artifact_retention_ttl_days: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role { Operator, Validator }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageAckKey {
    pub execution_id: String, pub stage_id: String, pub lane_id: String,
    pub authority_request_message_ref: String, pub prior_receipt_ref: RoleEvidenceRef,
    pub selecting_decision_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose { PriorSelection, StageAcknowledgement(StageAckKey) }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalEventKey { pub event_id: String, pub stream_id: String, pub event_seq: u64, pub transaction_seq: u64 }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorExecutionEvidence {
    pub runner_principal: ActorPrincipalId, pub runner_lane_id: String, pub invocation_ref: String,
    pub request_message_ref: String, pub execution_event: CanonicalEventKey, pub verdict_artifact: ArtifactSubject,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleEvidenceBody {
    pub role: Role, pub issuer: ActorPrincipalId, pub scope: ExactResourceScopeAttribution,
    pub action: ActionKey, pub run: RunBinding, pub subject: ArtifactSubject, pub purpose: Purpose,
    pub designation: Option<DesignationRef>, pub validator_execution: Option<ValidatorExecutionEvidence>,
    pub expected: ExpectedBase, pub retention: RetentionBinding, pub predecessor_refs: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleReceipt { pub body: RoleEvidenceBody, pub content_hash: ContentSha256, pub event: CanonicalEventKey }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalEventPayload { pub key: CanonicalEventKey, pub payload_json: String, pub payload_hash: ContentSha256 }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionExpectation {
    pub receipt_scope: ExactResourceScopeAttribution, pub role: Role, pub run: RunBinding,
    pub subject: ArtifactSubject, pub purpose: Purpose, pub expected: ExpectedBase,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveOperatorArtifact {
    pub action: ActionKey, pub scope: ExactResourceScopeAttribution, pub run: RunBinding,
    pub subject: ArtifactSubject, pub expected: ExpectedBase, pub retention: RetentionBinding,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordValidatorVerdict {
    pub action: ActionKey, pub scope: ExactResourceScopeAttribution, pub designation_ref: DesignationRef,
    pub execution: ValidatorExecutionEvidence, pub run: RunBinding, pub subject: ArtifactSubject,
    pub purpose: Purpose, pub expected: ExpectedBase, pub retention: RetentionBinding,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcknowledgeOperatorStage {
    pub approval: RoleEvidenceRef, pub action: ActionKey, pub scope: ExactResourceScopeAttribution,
    pub run: RunBinding, pub subject: ArtifactSubject, pub stage: StageAckKey,
    pub expected: ExpectedBase, pub retention: RetentionBinding,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcknowledgeValidatorStage {
    pub verdict: RoleEvidenceRef, pub designation_ref: DesignationRef, pub execution: ValidatorExecutionEvidence,
    pub action: ActionKey, pub scope: ExactResourceScopeAttribution, pub run: RunBinding,
    pub subject: ArtifactSubject, pub stage: StageAckKey, pub expected: ExpectedBase, pub retention: RetentionBinding,
}

/// Shape-checked declarations, not authenticated authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurePreparedRoleIntent { body: RoleEvidenceBody }
impl PurePreparedRoleIntent {
    pub(crate) fn new_checked(body: RoleEvidenceBody) -> Result<Self, RoleError> {
        crate::canonical::validate_body(&body)?;
        Ok(Self { body })
    }
    pub fn body(&self) -> &RoleEvidenceBody { &self.body }
    pub fn role(&self) -> &Role { &self.body.role }
    pub fn action(&self) -> &ActionKey { &self.body.action }
    pub fn claimed_scope(&self) -> &ExactResourceScopeAttribution { &self.body.scope }
    pub fn run(&self) -> &RunBinding { &self.body.run }
    pub fn subject(&self) -> &ArtifactSubject { &self.body.subject }
    pub fn purpose(&self) -> &Purpose { &self.body.purpose }
    pub fn expected(&self) -> &ExpectedBase { &self.body.expected }
    pub fn retention(&self) -> &RetentionBinding { &self.body.retention }
}

/// Integrity-checked receipt; no issuer, designation, freshness or entitlement attestation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PureVerifiedRoleEvidence { receipt: RoleReceipt, reference: RoleEvidenceRef }
impl PureVerifiedRoleEvidence {
    pub(crate) fn new_checked(receipt: RoleReceipt, reference: RoleEvidenceRef) -> Result<Self, RoleError> {
        crate::canonical::validate_body(&receipt.body)?;
        crate::canonical::validate_event_key(&receipt.event)?;
        if crate::canonical::body_content_hash(&receipt.body)? != receipt.content_hash { return Err(RoleError::ContentHashMismatch); }
        if reference.as_str() != format!("eventledger://role-authority/{}", receipt.event.event_id) { return Err(RoleError::EventIdentityMismatch); }
        Ok(Self { receipt, reference })
    }
    pub fn receipt(&self) -> &RoleReceipt { &self.receipt }
    pub fn reference(&self) -> &RoleEvidenceRef { &self.reference }
    pub fn event(&self) -> &CanonicalEventKey { &self.receipt.event }
    pub fn content_hash(&self) -> &ContentSha256 { &self.receipt.content_hash }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryDisposition { ExactExisting }

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RoleError {
    #[error("invalid field: {field}")] InvalidField { field: &'static str },
    #[error("content hash mismatch")] ContentHashMismatch,
    #[error("event hash mismatch")] EventHashMismatch,
    #[error("event identity mismatch")] EventIdentityMismatch,
    #[error("payload mismatch")] PayloadMismatch,
    #[error("binding mismatch: {field}")] BindingMismatch { field: &'static str },
    #[error("retry conflict")] RetryConflict,
    #[error("acknowledgement mismatch: {field}")] AckMismatch { field: &'static str },
    #[error("JSON: {0}")] Json(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hashes_and_references_reject_noncanonical_shapes() {
        let hash = "ab".repeat(32);
        assert_eq!(ContentSha256::from_hex(&hash).unwrap().to_hex(), hash);
        assert!(ContentSha256::from_hex(&"AB".repeat(32)).is_err());
        assert!(ContentSha256::from_hex("00").is_err());
        let reference = RoleEvidenceRef::parse("eventledger://role-authority/exact-id").unwrap();
        assert_eq!(serde_json::from_str::<RoleEvidenceRef>(&serde_json::to_string(&reference).unwrap()).unwrap(), reference);
        assert!(RoleEvidenceRef::parse("eventledger://role-authority/").is_err());
        assert!(DesignationRef::parse("eventledger://role-authority/id").is_err());
        assert!(RoleEvidenceRef::parse(&format!("eventledger://role-authority/{}", "x".repeat(513))).is_err());
    }
}
