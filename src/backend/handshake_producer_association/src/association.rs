use std::collections::{BTreeSet, HashSet};
use std::fmt;

use serde::de::{DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Audit data only. None of these values grants current resource authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope5V1 {
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssociationBodyV1 {
    pub format_version: u32,
    pub original_operation_id: Uuid,
    pub request_id: String,
    pub idempotency_key: String,
    pub causation_ref: String,
    pub original_issuer: Scope5V1,
    pub child_scope: Scope5V1,
    pub actual_instance: InstanceGenerationV1,
    pub server_normalized_selection: Value,
    pub run: RunTraceV1,
    pub launch: Value,
    pub source_chain: Vec<SourceChildPairV1>,
    pub cleanup: ChildCleanupInventoryV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceGenerationV1 {
    pub instance_id: String,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunTraceV1 {
    pub run_id: String,
    pub trace_id: String,
    pub work_packet_id: Option<String>,
    pub micro_task_id: Option<String>,
    pub locus: Value,
    pub full_run_trace: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceV1 {
    pub resource_kind: String,
    pub resource_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCapabilityV1 {
    pub action: String,
    pub capability_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantAuditV1 {
    pub grant_id: String,
    pub grant_version: i64,
    pub policy_versions: Vec<i64>,
    pub revocation_epochs: Vec<i64>,
    pub expires_at: Option<String>,
    pub full_grant_provenance: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceChildPairV1 {
    pub source_grant: GrantAuditV1,
    pub child_grant: GrantAuditV1,
    pub source_resource: ResourceV1,
    pub child_resource: ResourceV1,
    pub source_action_capability: ActionCapabilityV1,
    pub child_action_capability: ActionCapabilityV1,
    pub source_delegation_chain: Vec<Value>,
    pub child_delegation_chain: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantIdentityV1 {
    pub grant_id: String,
    pub grant_version: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildGrantCleanupV1 {
    pub source_parent_grant_id: String,
    pub source_parent_grant_version: i64,
    pub child_grant_id: String,
    pub child_grant_version: i64,
    pub expected_cas: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildCleanupInventoryV1 {
    pub child_principal_id: String,
    pub child_session_id: String,
    pub child_grants: Vec<ChildGrantCleanupV1>,
    pub full_canonical_cleanup_inventory: Value,
}

/// Expected identities are supplied data, not evidence of their origin or authority.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedAssociationInventoryV1 {
    pub original_principal_ids: Vec<String>,
    pub original_session_ids: Vec<String>,
    pub original_grants: Vec<GrantIdentityV1>,
    pub created: ChildCleanupInventoryV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssociationRecordV1 {
    pub run_id: String,
    pub event_ledger_stream_id: String,
    pub association: AssociationBodyV1,
}

/// Errors deliberately omit untrusted payloads, credentials and provenance contents.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssociationCodecError {
    #[error("invalid JSON")]
    InvalidJson,
    #[error("duplicate JSON object key")]
    DuplicateKey,
    #[error("unknown wire field")]
    UnknownField,
    #[error("invalid wire shape: {0}")]
    Shape(&'static str),
    #[error("noncanonical JSON")]
    NonCanonical,
    #[error("identity mismatch: {0}")]
    IdentityMismatch(&'static str),
    #[error("source/child resource action-capability mismatch")]
    ActionCapabilityMismatch,
    #[error("expiry representation mismatch")]
    ExpiryRepresentationMismatch,
    #[error("record/event link mismatch: {0}")]
    LinkMismatch(&'static str),
    #[error("full event payload hash mismatch")]
    PayloadHashMismatch,
}

pub(crate) struct CheckedAssociationRecord {
    record: AssociationRecordV1,
    record_json: String,
    source_chain_sha256: String,
}

impl CheckedAssociationRecord {
    pub(crate) fn record(&self) -> &AssociationRecordV1 { &self.record }
    pub(crate) fn body(&self) -> &AssociationBodyV1 { &self.record.association }
    pub(crate) fn record_json(&self) -> &str { &self.record_json }
    pub(crate) fn source_chain_sha256(&self) -> &str { &self.source_chain_sha256 }
}

pub(crate) fn encode_record_checked(
    body: AssociationBodyV1,
    original_stream: &str,
    expected: &ExpectedAssociationInventoryV1,
) -> Result<CheckedAssociationRecord, AssociationCodecError> {
    validate_body(&body, expected)?;
    require_id(original_stream, "producer stream")?;
    let source_chain_sha256 = sha256_hex(canonical_json(&body.source_chain)?.as_bytes());
    let record = AssociationRecordV1 {
        run_id: body.run.run_id.clone(),
        event_ledger_stream_id: original_stream.to_owned(),
        association: body,
    };
    let record_json = canonical_json(&record)?;
    Ok(CheckedAssociationRecord { record, record_json, source_chain_sha256 })
}

pub(crate) fn decode_record_checked(
    record_json: &str,
    expected: &ExpectedAssociationInventoryV1,
) -> Result<CheckedAssociationRecord, AssociationCodecError> {
    let record: AssociationRecordV1 = parse_canonical(record_json)?;
    if record.run_id != record.association.run.run_id {
        return Err(AssociationCodecError::IdentityMismatch("record run"));
    }
    encode_record_checked(record.association, &record.event_ledger_stream_id, expected)
}

fn require_id(value: &str, field: &'static str) -> Result<(), AssociationCodecError> {
    if value.is_empty() { Err(AssociationCodecError::Shape(field)) } else { Ok(()) }
}

fn require_object(value: &Value, field: &'static str) -> Result<(), AssociationCodecError> {
    if value.is_object() { Ok(()) } else { Err(AssociationCodecError::Shape(field)) }
}

fn validate_scope(scope: &Scope5V1) -> Result<(), AssociationCodecError> {
    for id in [&scope.owner_account_id, &scope.actor_principal_id,
        &scope.authenticated_session_id, &scope.access_space_id, &scope.workspace_id] {
        require_id(id, "scope identity")?;
    }
    Ok(())
}

fn validate_grant(grant: &GrantAuditV1) -> Result<(), AssociationCodecError> {
    require_id(&grant.grant_id, "grant identity")?;
    // Expiry and signed policy arrays are retained as observed, never evaluated here.
    require_object(&grant.full_grant_provenance, "grant provenance")
}

fn validate_body(
    body: &AssociationBodyV1,
    expected: &ExpectedAssociationInventoryV1,
) -> Result<(), AssociationCodecError> {
    if body.format_version != 1 { return Err(AssociationCodecError::Shape("body version")); }
    validate_scope(&body.original_issuer)?;
    validate_scope(&body.child_scope)?;
    require_id(&body.actual_instance.instance_id, "instance identity")?;
    require_id(&body.run.run_id, "run identity")?;
    require_object(&body.server_normalized_selection, "normalized selection")?;
    require_object(&body.run.locus, "run locus")?;
    require_object(&body.run.full_run_trace, "full run trace")?;
    require_object(&body.launch, "full launch")?;
    require_object(&body.cleanup.full_canonical_cleanup_inventory, "cleanup inventory")?;
    if body.cleanup != expected.created {
        return Err(AssociationCodecError::IdentityMismatch("expected child inventory"));
    }
    if body.cleanup.child_principal_id != body.child_scope.actor_principal_id
        || body.cleanup.child_session_id != body.child_scope.authenticated_session_id {
        return Err(AssociationCodecError::IdentityMismatch("child scope inventory"));
    }
    let mut original_principals = unique_ids(&expected.original_principal_ids, "original principals")?;
    let mut original_sessions = unique_ids(&expected.original_session_ids, "original sessions")?;
    original_principals.insert(body.original_issuer.actor_principal_id.as_str());
    original_sessions.insert(body.original_issuer.authenticated_session_id.as_str());
    if original_principals.contains(body.cleanup.child_principal_id.as_str())
        || original_sessions.contains(body.cleanup.child_session_id.as_str()) {
        return Err(AssociationCodecError::IdentityMismatch("parent cleanup target"));
    }
    let mut original_grants = BTreeSet::new();
    for grant in &expected.original_grants {
        require_id(&grant.grant_id, "original grant")?;
        if !original_grants.insert(grant.grant_id.as_str()) {
            return Err(AssociationCodecError::IdentityMismatch("duplicate original grant"));
        }
    }
    if body.cleanup.child_grants.len() != body.source_chain.len() {
        return Err(AssociationCodecError::IdentityMismatch("child grant inventory length"));
    }
    let mut children = BTreeSet::new();
    for (pair, cleanup) in body.source_chain.iter().zip(&body.cleanup.child_grants) {
        validate_grant(&pair.source_grant)?;
        validate_grant(&pair.child_grant)?;
        for resource in [&pair.source_resource, &pair.child_resource] {
            require_id(&resource.resource_kind, "resource kind")?;
            require_id(&resource.resource_id, "resource identity")?;
        }
        for action in [&pair.source_action_capability, &pair.child_action_capability] {
            require_id(&action.action, "resource action")?;
            require_id(&action.capability_id, "resource capability")?;
        }
        if pair.source_resource != pair.child_resource
            || pair.source_action_capability != pair.child_action_capability {
            return Err(AssociationCodecError::ActionCapabilityMismatch);
        }
        for link in pair.source_delegation_chain.iter().chain(&pair.child_delegation_chain) {
            require_object(link, "ordered delegation link")?;
        }
        if !expected.original_grants.iter().any(|original|
            original.grant_id == pair.source_grant.grant_id
                && original.grant_version == pair.source_grant.grant_version) {
            return Err(AssociationCodecError::IdentityMismatch("source grant inventory"));
        }
        if original_grants.contains(pair.child_grant.grant_id.as_str())
            || !children.insert(pair.child_grant.grant_id.as_str()) {
            return Err(AssociationCodecError::IdentityMismatch("parent or duplicate child grant"));
        }
        require_object(&cleanup.expected_cas, "grant expected CAS")?;
        if cleanup.source_parent_grant_id != pair.source_grant.grant_id
            || cleanup.source_parent_grant_version != pair.source_grant.grant_version
            || cleanup.child_grant_id != pair.child_grant.grant_id
            || cleanup.child_grant_version != pair.child_grant.grant_version {
            return Err(AssociationCodecError::IdentityMismatch("source child cleanup pair"));
        }
    }
    Ok(())
}

fn unique_ids<'a>(
    ids: &'a [String], field: &'static str,
) -> Result<BTreeSet<&'a str>, AssociationCodecError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        require_id(id, field)?;
        if !seen.insert(id.as_str()) { return Err(AssociationCodecError::Shape(field)); }
    }
    Ok(seen)
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = StrictValue;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value without duplicate object keys")
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Bool(value)))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value).map(|number| StrictValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(value.to_owned())))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(value)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(StrictValue(value)) = sequence.next_element::<StrictValue>()? {
                    values.push(value);
                }
                Ok(StrictValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut seen = HashSet::new();
                let mut object = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(serde::de::Error::custom("duplicate JSON object key"));
                    }
                    let StrictValue(value) = map.next_value::<StrictValue>()?;
                    object.insert(key, value);
                }
                Ok(StrictValue(Value::Object(object)))
            }
        }
        deserializer.deserialize_any(StrictVisitor)
    }
}

pub(crate) fn parse_strict<T: DeserializeOwned>(input: &str) -> Result<T, AssociationCodecError> {
    let StrictValue(value) = serde_json::from_str::<StrictValue>(input).map_err(|error| {
        if error.to_string().contains("duplicate JSON object key") {
            AssociationCodecError::DuplicateKey
        } else { AssociationCodecError::InvalidJson }
    })?;
    serde_json::from_value(value).map_err(|error| {
        if error.to_string().starts_with("unknown field") { AssociationCodecError::UnknownField }
        else { AssociationCodecError::Shape("typed JSON value") }
    })
}

pub(crate) fn parse_canonical<T: DeserializeOwned + Serialize>(
    input: &str,
) -> Result<T, AssociationCodecError> {
    let decoded: T = parse_strict(input)?;
    if canonical_json(&decoded)? != input { return Err(AssociationCodecError::NonCanonical); }
    Ok(decoded)
}

pub(crate) fn canonical_json<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String, AssociationCodecError> {
    fn render(value: &Value, output: &mut String) -> Result<(), AssociationCodecError> {
        match value {
            Value::Null => output.push_str("null"),
            Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Value::Number(value) => output.push_str(&value.to_string()),
            Value::String(value) => output.push_str(&serde_json::to_string(value)
                .map_err(|_| AssociationCodecError::InvalidJson)?),
            Value::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 { output.push(','); }
                    render(value, output)?;
                }
                output.push(']');
            }
            Value::Object(values) => {
                output.push('{');
                let mut entries: Vec<_> = values.iter().collect();
                entries.sort_by(|left, right| left.0.cmp(right.0));
                for (index, (key, value)) in entries.into_iter().enumerate() {
                    if index > 0 { output.push(','); }
                    output.push_str(&serde_json::to_string(key)
                        .map_err(|_| AssociationCodecError::InvalidJson)?);
                    output.push(':');
                    render(value, output)?;
                }
                output.push('}');
            }
        }
        Ok(())
    }
    let value = serde_json::to_value(value).map_err(|_| AssociationCodecError::InvalidJson)?;
    let mut output = String::new();
    render(&value, &mut output)?;
    Ok(output)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn fixture() -> (AssociationBodyV1, ExpectedAssociationInventoryV1) {
        let issuer = Scope5V1 { owner_account_id: "account".into(), actor_principal_id: "parent".into(),
            authenticated_session_id: "parent-session".into(), access_space_id: "space".into(), workspace_id: "workspace".into() };
        let child = Scope5V1 { actor_principal_id: "child".into(), authenticated_session_id: "child-session".into(), ..issuer.clone() };
        let grant = |id: &str, version| GrantAuditV1 { grant_id: id.into(), grant_version: version,
            policy_versions: vec![i64::MIN, 0, i64::MAX], revocation_epochs: vec![-1, i64::MAX], expires_at: None,
            full_grant_provenance: json!({"session": "actual-session", "expiry": null, "profile": {"caps": ["fs.read"]}}) };
        let resource = ResourceV1 { resource_kind: "document".into(), resource_id: "doc".into() };
        let cap = ActionCapabilityV1 { action: "read".into(), capability_id: "fs.read".into() };
        let pair = SourceChildPairV1 { source_grant: grant("source", i64::MIN), child_grant: grant("child-grant", i64::MAX),
            source_resource: resource.clone(), child_resource: resource, source_action_capability: cap.clone(), child_action_capability: cap,
            source_delegation_chain: vec![json!({"principal":"root","session":"root-session"}), json!({"principal":"parent","session":"parent-session"})],
            child_delegation_chain: vec![json!({"principal":"parent","session":"parent-session"}), json!({"principal":"child","session":"child-session"})] };
        let cleanup = ChildCleanupInventoryV1 { child_principal_id: "child".into(), child_session_id: "child-session".into(),
            child_grants: vec![ChildGrantCleanupV1 { source_parent_grant_id: "source".into(), source_parent_grant_version: i64::MIN,
                child_grant_id: "child-grant".into(), child_grant_version: i64::MAX, expected_cas: json!({"status":"active","policy_epochs":[-9,8],"intent":{"all":true}}) }],
            full_canonical_cleanup_inventory: json!({"full_intent":{"resource":"doc"},"account_space":{"account":"account","space":"space"},"runtime_generation":u64::MAX}) };
        let expected = ExpectedAssociationInventoryV1 { original_principal_ids: vec!["parent".into()], original_session_ids: vec!["parent-session".into()],
            original_grants: vec![GrantIdentityV1 { grant_id: "source".into(), grant_version: i64::MIN }], created: cleanup.clone() };
        let body = AssociationBodyV1 { format_version: 1, original_operation_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap(),
            request_id: String::new(), idempotency_key: String::new(), causation_ref: String::new(), original_issuer: issuer, child_scope: child,
            actual_instance: InstanceGenerationV1 { instance_id: "instance".into(), generation: u64::MAX }, server_normalized_selection: json!({"tools":["read"],"revision":-1}),
            run: RunTraceV1 { run_id:"run".into(),trace_id:"trace".into(),work_packet_id:None,micro_task_id:None,locus:json!({"kind":"workspace"}),full_run_trace:json!({"ordered":[3,2,1]}) },
            launch:json!({"complete":true}),source_chain:vec![pair],cleanup };
        (body, expected)
    }

    #[test]
    fn signed_provenance_and_full_ordered_chains_survive_record_round_trip() {
        let (body, expected) = fixture();
        let checked = encode_record_checked(body.clone(), "original-stream", &expected).unwrap();
        let decoded = decode_record_checked(checked.record_json(), &expected).unwrap();
        assert_eq!(decoded.body(), &body);
        assert_eq!(decoded.record(), checked.record());
        assert_eq!(decoded.source_chain_sha256(), checked.source_chain_sha256());
        assert!(checked.record_json().contains("-9223372036854775808"));
        assert!(checked.record_json().contains("18446744073709551615"));
    }

    #[test]
    fn one_source_can_map_to_multiple_unique_children_without_cross_product() {
        let (mut body, mut expected) = fixture();
        let mut pair = body.source_chain[0].clone();
        pair.child_grant.grant_id = "another-child".into();
        body.source_chain.push(pair);
        let mut cleanup = body.cleanup.child_grants[0].clone();
        cleanup.child_grant_id = "another-child".into();
        body.cleanup.child_grants.push(cleanup);
        expected.created = body.cleanup.clone();
        assert!(encode_record_checked(body.clone(), "stream", &expected).is_ok());
        body.source_chain[1].child_action_capability.capability_id = "fs.write".into();
        assert!(matches!(encode_record_checked(body,"stream",&expected), Err(AssociationCodecError::ActionCapabilityMismatch)));
    }

    #[test]
    fn chain_order_and_complete_cas_content_are_not_projected_away() {
        let (body, expected) = fixture();
        let first = encode_record_checked(body.clone(), "stream", &expected).unwrap();
        let mut changed = body;
        changed.source_chain[0].source_delegation_chain.reverse();
        let second = encode_record_checked(changed, "stream", &expected).unwrap();
        assert_ne!(first.source_chain_sha256(), second.source_chain_sha256());
        assert_ne!(first.record_json(), second.record_json());
        let mut different_expected = expected;
        different_expected.created.child_grants[0].expected_cas["status"] = json!("revoked");
        assert!(matches!(decode_record_checked(first.record_json(), &different_expected), Err(AssociationCodecError::IdentityMismatch("expected child inventory"))));
    }

    #[test]
    fn parent_targets_duplicate_children_and_wrong_versions_are_rejected() {
        let (mut body, mut expected) = fixture();
        body.cleanup.child_principal_id = "parent".into();
        body.child_scope.actor_principal_id = "parent".into();
        expected.created = body.cleanup.clone();
        assert!(matches!(encode_record_checked(body,"stream",&expected), Err(AssociationCodecError::IdentityMismatch("parent cleanup target"))));
        let (mut body, mut expected) = fixture();
        body.source_chain.push(body.source_chain[0].clone());
        body.cleanup.child_grants.push(body.cleanup.child_grants[0].clone());
        expected.created = body.cleanup.clone();
        assert!(matches!(encode_record_checked(body,"stream",&expected), Err(AssociationCodecError::IdentityMismatch("parent or duplicate child grant"))));
        let (mut body, mut expected) = fixture();
        body.cleanup.child_grants[0].child_grant_version = -8;
        expected.created = body.cleanup.clone();
        assert!(matches!(encode_record_checked(body,"stream",&expected), Err(AssociationCodecError::IdentityMismatch("source child cleanup pair"))));
    }

    #[test]
    fn resource_identity_and_original_versions_must_match_their_pair() {
        let (mut body, expected) = fixture();
        body.source_chain[0].child_resource.resource_id = "another-document".into();
        assert!(matches!(encode_record_checked(body,"stream",&expected), Err(AssociationCodecError::ActionCapabilityMismatch)));
        let (body, mut expected) = fixture();
        expected.original_grants[0].grant_version = 0;
        assert!(matches!(encode_record_checked(body,"stream",&expected), Err(AssociationCodecError::IdentityMismatch("source grant inventory"))));
        let (mut body, mut expected) = fixture();
        body.cleanup.child_session_id = "parent-session".into();
        body.child_scope.authenticated_session_id = "parent-session".into();
        expected.created = body.cleanup.clone();
        assert!(matches!(encode_record_checked(body,"stream",&expected), Err(AssociationCodecError::IdentityMismatch("parent cleanup target"))));
    }

    #[test]
    fn strict_parser_rejects_decoded_duplicate_keys_at_every_depth() {
        for input in [r#"{"a":1,"a":2}"#, r#"{"a":1,"\u0061":2}"#, r#"[{"nested":{"x":0,"x":1}}]"#] {
            assert_eq!(parse_strict::<Value>(input), Err(AssociationCodecError::DuplicateKey));
        }
        assert_eq!(parse_strict::<Value>("{} null"), Err(AssociationCodecError::InvalidJson));
    }

    #[test]
    fn typed_unknown_fields_and_noncanonical_bytes_are_rejected() {
        let (body, expected) = fixture();
        let checked = encode_record_checked(body,"stream",&expected).unwrap();
        let mut record: Value = parse_strict(checked.record_json()).unwrap();
        record["unknown"] = json!(null);
        assert!(matches!(decode_record_checked(&canonical_json(&record).unwrap(), &expected), Err(AssociationCodecError::UnknownField)));
        let padded = format!(" {}", checked.record_json());
        assert!(matches!(decode_record_checked(&padded,&expected), Err(AssociationCodecError::NonCanonical)));
    }

    #[test]
    fn canonical_renderer_preserves_complete_json_values_and_array_order() {
        let value = json!({"z":[null,true,-5,1.25,"quote\"\\\n\t雪"],"a":{"b":2,"a":1}});
        let rendered = canonical_json(&value).unwrap();
        assert!(rendered.starts_with("{\"a\":{\"a\":1,\"b\":2},\"z\":[null,true,-5,1.25,"));
        assert_eq!(parse_canonical::<Value>(&rendered).unwrap(), value);
        let different = json!({"z":[true,null,-5,1.25,"quote\"\\\n\t雪"],"a":{"a":1,"b":2}});
        assert_ne!(sha256_hex(rendered.as_bytes()), sha256_hex(canonical_json(&different).unwrap().as_bytes()));
    }
}
