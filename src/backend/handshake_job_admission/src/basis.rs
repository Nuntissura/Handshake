use handshake_resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, WorkspaceScopeRef,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{codec::canonical_json_bytes, identity::ProducerJobIdentity};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreationEntityRef {
    pub entity_id: String,
    pub entity_kind: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreationOperationType {
    Read,
    Write,
    Plan,
    Execute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreationPlannedOperation {
    pub op_type: CreationOperationType,
    pub target: CreationEntityRef,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreationDescriptorInput {
    pub job_kind: String,
    pub protocol_id: String,
    pub profile_id: String,
    pub capability_profile_id: String,
    pub access_mode: String,
    pub safety_mode: String,
    pub entity_refs: Vec<CreationEntityRef>,
    pub planned_operations: Vec<CreationPlannedOperation>,
    pub job_inputs: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreationMatch {
    Identical,
    Conflict,
    LegacyUnverifiable,
}

#[derive(Debug, thiserror::Error)]
pub enum CreationCodecError {
    #[error("creation basis JSON is invalid")]
    Json(#[source] serde_json::Error),
    #[error("unsupported creation basis format version {0}")]
    UnsupportedFormat(u32),
    #[error("creation basis bytes are not canonical version 1 JSON")]
    NonCanonical,
    #[error("creation basis SHA256 does not match its exact bytes")]
    HashMismatch,
    #[error("creation basis JSON and SHA256 must both be present or both absent")]
    PartialBasis,
    #[error("creation identity must contain valid producer UUIDv7 identifiers")]
    InvalidIdentity,
    #[error("reserved workflow identity does not match the job kind")]
    ReservedWorkflowMismatch,
    #[error("creation descriptor must name exactly its canonical workspace")]
    WorkspaceMismatch,
}

impl From<serde_json::Error> for CreationCodecError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

// This is immutable data, not a capability or a persisted task-completion receipt.
#[derive(Clone)]
pub struct CheckedJobCreation {
    identity: ProducerJobIdentity,
    scope: ExactResourceScopeAttribution,
    descriptor: CreationDescriptorInput,
    canonical_bytes: Vec<u8>,
    sha256_hex: String,
}

impl CheckedJobCreation {
    pub fn build(
        identity: ProducerJobIdentity,
        scope: ExactResourceScopeAttribution,
        descriptor: CreationDescriptorInput,
    ) -> Result<Self, CreationCodecError> {
        validate_binding(&identity, &scope, &descriptor)?;
        let wire = CreationBasisV1::from_parts(&identity, &scope, &descriptor);
        let canonical_bytes = canonical_json_bytes(&serde_json::to_value(wire)?)?;
        let sha256_hex = sha256_hex(&canonical_bytes);
        Ok(Self { identity, scope, descriptor, canonical_bytes, sha256_hex })
    }

    pub fn decode_persisted(json: &str, hash: &str) -> Result<Self, CreationCodecError> {
        let wire: CreationBasisV1 = serde_json::from_str(json)?;
        let canonical = canonical_json_bytes(&serde_json::to_value(&wire)?)?;
        if canonical != json.as_bytes() {
            return Err(CreationCodecError::NonCanonical);
        }
        if wire.format_version != 1 {
            return Err(CreationCodecError::UnsupportedFormat(wire.format_version));
        }
        if sha256_hex(&canonical) != hash {
            return Err(CreationCodecError::HashMismatch);
        }
        let identity = ProducerJobIdentity::from_parts(
            wire.identity.job_id,
            wire.identity.initial_trace_id,
            wire.identity.reserved_workflow_run_id.0,
        ).ok_or(CreationCodecError::InvalidIdentity)?;
        let scope = wire.original_scope.into_scope()?;
        let descriptor = wire.descriptor.into_input();
        validate_binding(&identity, &scope, &descriptor)?;
        Ok(Self { identity, scope, descriptor, canonical_bytes: canonical, sha256_hex: hash.to_owned() })
    }

    pub fn compare_persisted(
        &self,
        json: Option<&str>,
        hash: Option<&str>,
    ) -> Result<CreationMatch, CreationCodecError> {
        let persisted = match (json, hash) {
            (None, None) => return Ok(CreationMatch::LegacyUnverifiable),
            (Some(json), Some(hash)) => Self::decode_persisted(json, hash)?,
            _ => return Err(CreationCodecError::PartialBasis),
        };
        Ok(if self.canonical_bytes == persisted.canonical_bytes && self.sha256_hex == persisted.sha256_hex {
            CreationMatch::Identical
        } else {
            CreationMatch::Conflict
        })
    }

    pub fn identity(&self) -> &ProducerJobIdentity { &self.identity }
    pub fn scope(&self) -> &ExactResourceScopeAttribution { &self.scope }
    pub fn descriptor(&self) -> &CreationDescriptorInput { &self.descriptor }
    pub fn canonical_bytes(&self) -> &[u8] { &self.canonical_bytes }
    pub fn sha256_hex(&self) -> &str { &self.sha256_hex }
}

fn validate_binding(
    identity: &ProducerJobIdentity,
    scope: &ExactResourceScopeAttribution,
    descriptor: &CreationDescriptorInput,
) -> Result<(), CreationCodecError> {
    if identity.reserved_workflow_run_id().is_some() != (descriptor.job_kind == "model_run") {
        return Err(CreationCodecError::ReservedWorkflowMismatch);
    }
    let mut workspaces = descriptor.entity_refs.iter().filter(|entity| entity.entity_kind == "workspace");
    if scope.workspace_id.as_str().trim().is_empty()
        || workspaces.next().map(|entity| entity.entity_id.as_str()) != Some(scope.workspace_id.as_str())
        || workspaces.next().is_some()
    {
        return Err(CreationCodecError::WorkspaceMismatch);
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Serialize)]
#[serde(transparent)]
struct RequiredNullable<T>(Option<T>);

fn required_nullable<'de, D, T>(deserializer: D) -> Result<RequiredNullable<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(RequiredNullable)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreationBasisV1 {
    format_version: u32,
    identity: StoredIdentity,
    original_scope: StoredScope,
    descriptor: StoredDescriptor,
}

impl CreationBasisV1 {
    fn from_parts(identity: &ProducerJobIdentity, scope: &ExactResourceScopeAttribution, descriptor: &CreationDescriptorInput) -> Self {
        Self {
            format_version: 1,
            identity: StoredIdentity {
                job_id: identity.job_id(),
                initial_trace_id: identity.initial_trace_id(),
                reserved_workflow_run_id: RequiredNullable(identity.reserved_workflow_run_id()),
            },
            original_scope: StoredScope {
                owner_account_id: scope.owner_account_id.as_uuid(),
                actor_principal_id: scope.actor_principal_id.as_uuid(),
                authenticated_session_id: scope.authenticated_session_id.as_uuid(),
                access_space_id: scope.access_space_id.as_uuid(),
                workspace_id: scope.workspace_id.as_str().to_owned(),
            },
            descriptor: StoredDescriptor::from_input(descriptor),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredIdentity {
    job_id: Uuid,
    initial_trace_id: Uuid,
    #[serde(deserialize_with = "required_nullable")]
    reserved_workflow_run_id: RequiredNullable<Uuid>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredScope {
    owner_account_id: Uuid,
    actor_principal_id: Uuid,
    authenticated_session_id: Uuid,
    access_space_id: Uuid,
    workspace_id: String,
}

impl StoredScope {
    fn into_scope(self) -> Result<ExactResourceScopeAttribution, CreationCodecError> {
        Ok(ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::from_uuid(self.owner_account_id),
            actor_principal_id: ActorPrincipalId::from_uuid(self.actor_principal_id),
            authenticated_session_id: AuthenticatedSessionRef::from_uuid(self.authenticated_session_id),
            access_space_id: AccessSpaceRef::from_uuid(self.access_space_id),
            workspace_id: WorkspaceScopeRef::new(self.workspace_id).map_err(|_| CreationCodecError::WorkspaceMismatch)?,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredDescriptor {
    job_kind: String,
    protocol_id: String,
    profile_id: String,
    capability_profile_id: String,
    access_mode: String,
    safety_mode: String,
    entity_refs: Vec<StoredEntityRef>,
    planned_operations: Vec<StoredPlannedOperation>,
    #[serde(deserialize_with = "required_nullable")]
    job_inputs: RequiredNullable<Value>,
}

impl StoredDescriptor {
    fn from_input(input: &CreationDescriptorInput) -> Self {
        Self {
            job_kind: input.job_kind.clone(), protocol_id: input.protocol_id.clone(),
            profile_id: input.profile_id.clone(), capability_profile_id: input.capability_profile_id.clone(),
            access_mode: input.access_mode.clone(), safety_mode: input.safety_mode.clone(),
            entity_refs: input.entity_refs.iter().map(StoredEntityRef::from_input).collect(),
            planned_operations: input.planned_operations.iter().map(|operation| StoredPlannedOperation {
                op_type: operation.op_type, target: StoredEntityRef::from_input(&operation.target),
                description: RequiredNullable(operation.description.clone()),
            }).collect(),
            job_inputs: RequiredNullable(input.job_inputs.clone()),
        }
    }

    fn into_input(self) -> CreationDescriptorInput {
        CreationDescriptorInput {
            job_kind: self.job_kind, protocol_id: self.protocol_id, profile_id: self.profile_id,
            capability_profile_id: self.capability_profile_id, access_mode: self.access_mode, safety_mode: self.safety_mode,
            entity_refs: self.entity_refs.into_iter().map(StoredEntityRef::into_input).collect(),
            planned_operations: self.planned_operations.into_iter().map(|operation| CreationPlannedOperation {
                op_type: operation.op_type, target: operation.target.into_input(), description: operation.description.0,
            }).collect(),
            job_inputs: self.job_inputs.0,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEntityRef {
    entity_id: String,
    entity_kind: String,
}

impl StoredEntityRef {
    fn from_input(input: &CreationEntityRef) -> Self {
        Self { entity_id: input.entity_id.clone(), entity_kind: input.entity_kind.clone() }
    }

    fn into_input(self) -> CreationEntityRef {
        CreationEntityRef { entity_id: self.entity_id, entity_kind: self.entity_kind }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPlannedOperation {
    op_type: CreationOperationType,
    target: StoredEntityRef,
    #[serde(deserialize_with = "required_nullable")]
    description: RequiredNullable<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(inputs: Option<Value>) -> CheckedJobCreation {
        let scope = ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::from_uuid(Uuid::now_v7()),
            actor_principal_id: ActorPrincipalId::from_uuid(Uuid::now_v7()),
            authenticated_session_id: AuthenticatedSessionRef::from_uuid(Uuid::now_v7()),
            access_space_id: AccessSpaceRef::from_uuid(Uuid::now_v7()),
            workspace_id: WorkspaceScopeRef::new("workspace-basis").unwrap(),
        };
        CheckedJobCreation::build(ProducerJobIdentity::allocate(true), scope, CreationDescriptorInput {
            job_kind: "model_run".into(), protocol_id: "operator_chat".into(), profile_id: "default".into(),
            capability_profile_id: "Operator".into(), access_mode: "analysis_only".into(), safety_mode: "normal".into(),
            entity_refs: vec![CreationEntityRef { entity_id: "workspace-basis".into(), entity_kind: "workspace".into() }],
            planned_operations: vec![CreationPlannedOperation {
                op_type: CreationOperationType::Read,
                target: CreationEntityRef { entity_id: "document-basis".into(), entity_kind: "document".into() },
                description: None,
            }],
            job_inputs: inputs,
        }).unwrap()
    }

    fn encoded(value: &Value) -> (String, String) {
        let bytes = canonical_json_bytes(value).unwrap();
        let hash = sha256_hex(&bytes);
        (String::from_utf8(bytes).unwrap(), hash)
    }

    #[test]
    fn basis_roundtrip_binds_full_descriptor_identity_and_scope() {
        let original = fixture(Some(json!({"text": "\0\u{8}\u{c}", "nested": [{"\0key": "é\u{10000}"}]})));
        let decoded = CheckedJobCreation::decode_persisted(std::str::from_utf8(original.canonical_bytes()).unwrap(), original.sha256_hex()).unwrap();
        assert_eq!(decoded.identity(), original.identity());
        assert_eq!(decoded.scope(), original.scope());
        assert_eq!(decoded.descriptor(), original.descriptor());
        assert_eq!(decoded.canonical_bytes(), original.canonical_bytes());
        assert_eq!(decoded.sha256_hex(), sha256_hex(original.canonical_bytes()));
    }

    #[test]
    fn identical_retry_conflict_and_legacy_are_distinct() {
        let basis = fixture(Some(json!({"values": [1, 2]})));
        let text = std::str::from_utf8(basis.canonical_bytes()).unwrap();
        assert_eq!(basis.compare_persisted(Some(text), Some(basis.sha256_hex())).unwrap(), CreationMatch::Identical);
        assert_eq!(basis.compare_persisted(None, None).unwrap(), CreationMatch::LegacyUnverifiable);
        assert!(matches!(basis.compare_persisted(Some(text), None), Err(CreationCodecError::PartialBasis)));
        assert!(matches!(basis.compare_persisted(None, Some(basis.sha256_hex())), Err(CreationCodecError::PartialBasis)));
        let mut different = basis.descriptor().clone();
        different.job_inputs = Some(json!({"values": [2, 1]}));
        let other = CheckedJobCreation::build(basis.identity().clone(), basis.scope().clone(), different).unwrap();
        assert_eq!(basis.compare_persisted(Some(std::str::from_utf8(other.canonical_bytes()).unwrap()), Some(other.sha256_hex())).unwrap(), CreationMatch::Conflict);
    }

    #[test]
    fn explicit_null_fields_are_required_and_not_materialized() {
        let basis = fixture(None);
        let value: Value = serde_json::from_slice(basis.canonical_bytes()).unwrap();
        assert!(value["descriptor"]["job_inputs"].is_null());
        assert!(value["descriptor"]["planned_operations"][0]["description"].is_null());
        for pointer in ["/descriptor/job_inputs", "/descriptor/planned_operations/0/description", "/identity/reserved_workflow_run_id"] {
            let mut missing = value.clone();
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            missing.pointer_mut(parent).unwrap().as_object_mut().unwrap().remove(key);
            let (text, hash) = encoded(&missing);
            assert!(CheckedJobCreation::decode_persisted(&text, &hash).is_err(), "missing {pointer} must fail");
        }
        let mut descriptor = basis.descriptor().clone();
        descriptor.job_kind = "workflow_run".into();
        let without_workflow = CheckedJobCreation::build(
            ProducerJobIdentity::allocate(false), basis.scope().clone(), descriptor,
        ).unwrap();
        let value: Value = serde_json::from_slice(without_workflow.canonical_bytes()).unwrap();
        assert!(value["identity"]["reserved_workflow_run_id"].is_null());
        let decoded = CheckedJobCreation::decode_persisted(
            std::str::from_utf8(without_workflow.canonical_bytes()).unwrap(), without_workflow.sha256_hex(),
        ).unwrap();
        assert_eq!(decoded.identity().reserved_workflow_run_id(), None);
    }

    #[test]
    fn data_clones_cannot_mutate_the_checked_basis() {
        let basis = fixture(Some(json!({"payload": [1, 2]})));
        let bytes = basis.canonical_bytes().to_vec();
        let hash = basis.sha256_hex().to_owned();
        let mut descriptor = basis.descriptor().clone();
        descriptor.protocol_id = "different-protocol".into();
        descriptor.job_inputs = Some(json!({"payload": [2, 1]}));
        let changed = CheckedJobCreation::build(basis.identity().clone(), basis.scope().clone(), descriptor).unwrap();
        assert_eq!(basis.canonical_bytes(), bytes);
        assert_eq!(basis.sha256_hex(), hash);
        assert_ne!(changed.canonical_bytes(), basis.canonical_bytes());
        let cloned = basis.clone();
        assert_eq!(cloned.canonical_bytes(), basis.canonical_bytes());
        assert_eq!(cloned.identity(), basis.identity());
    }

    #[test]
    fn original_scope_changes_are_creation_conflicts() {
        let basis = fixture(None);
        let mut scope = basis.scope().clone();
        scope.authenticated_session_id = AuthenticatedSessionRef::from_uuid(Uuid::now_v7());
        let changed = CheckedJobCreation::build(basis.identity().clone(), scope, basis.descriptor().clone()).unwrap();
        assert_eq!(basis.compare_persisted(
            Some(std::str::from_utf8(changed.canonical_bytes()).unwrap()), Some(changed.sha256_hex()),
        ).unwrap(), CreationMatch::Conflict);
    }

    #[test]
    fn strict_version_unknown_fields_and_hash_are_rejected() {
        let basis = fixture(None);
        let value: Value = serde_json::from_slice(basis.canonical_bytes()).unwrap();
        let mut version = value.clone();
        version["format_version"] = json!(2);
        let (text, hash) = encoded(&version);
        assert!(matches!(CheckedJobCreation::decode_persisted(&text, &hash), Err(CreationCodecError::UnsupportedFormat(2))));
        for pointer in ["", "/descriptor", "/identity", "/original_scope", "/descriptor/entity_refs/0", "/descriptor/planned_operations/0"] {
            let mut extra = value.clone();
            extra.pointer_mut(pointer).unwrap().as_object_mut().unwrap().insert("unknown".into(), json!(true));
            let (text, hash) = encoded(&extra);
            assert!(CheckedJobCreation::decode_persisted(&text, &hash).is_err());
        }
        assert!(matches!(CheckedJobCreation::decode_persisted(std::str::from_utf8(basis.canonical_bytes()).unwrap(), &"0".repeat(64)), Err(CreationCodecError::HashMismatch)));
    }

    #[test]
    fn duplicate_and_escape_equivalent_keys_fail_even_with_matching_hash() {
        let basis = fixture(Some(json!({"nested": {"key": 1}})));
        let original = std::str::from_utf8(basis.canonical_bytes()).unwrap();
        for duplicate in [
            original.replace(r#""key":1"#, r#""key":1,"key":1"#),
            original.replace(r#""key":1"#, r#""key":0,"key":1"#),
            original.replace(r#""key":1"#, r#""\u006bey":1,"key":1"#),
        ] {
            assert_ne!(duplicate, original);
            let hash = sha256_hex(duplicate.as_bytes());
            assert!(matches!(CheckedJobCreation::decode_persisted(&duplicate, &hash), Err(CreationCodecError::NonCanonical)));
        }
        let duplicate_outer = original.replace("\"format_version\":1", "\"format_version\":1,\"format_version\":1");
        assert!(CheckedJobCreation::decode_persisted(&duplicate_outer, &sha256_hex(duplicate_outer.as_bytes())).is_err());
    }

    #[test]
    fn noncanonical_whitespace_is_rejected_before_hash_acceptance() {
        let basis = fixture(None);
        let text = format!(" {}", std::str::from_utf8(basis.canonical_bytes()).unwrap());
        assert!(matches!(CheckedJobCreation::decode_persisted(&text, &sha256_hex(text.as_bytes())), Err(CreationCodecError::NonCanonical)));
    }

    #[test]
    fn workflow_and_workspace_bindings_cannot_be_retargeted() {
        let basis = fixture(None);
        let mut descriptor = basis.descriptor().clone();
        descriptor.job_kind = "workflow_run".into();
        assert!(matches!(CheckedJobCreation::build(basis.identity().clone(), basis.scope().clone(), descriptor), Err(CreationCodecError::ReservedWorkflowMismatch)));
        let mut descriptor = basis.descriptor().clone();
        descriptor.entity_refs[0].entity_id = "other-workspace".into();
        assert!(matches!(CheckedJobCreation::build(basis.identity().clone(), basis.scope().clone(), descriptor), Err(CreationCodecError::WorkspaceMismatch)));
        let mut descriptor = basis.descriptor().clone();
        descriptor.entity_refs.push(descriptor.entity_refs[0].clone());
        assert!(matches!(CheckedJobCreation::build(basis.identity().clone(), basis.scope().clone(), descriptor), Err(CreationCodecError::WorkspaceMismatch)));
    }
}
