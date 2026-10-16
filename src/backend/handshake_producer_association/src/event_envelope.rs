use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::association::{
    canonical_json, decode_record_checked, encode_record_checked, parse_canonical, sha256_hex,
    AssociationBodyV1, AssociationCodecError, AssociationRecordV1, CheckedAssociationRecord,
    ChildCleanupInventoryV1, ExpectedAssociationInventoryV1, InstanceGenerationV1, Scope5V1,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssociationEventPayloadV1 {
    pub dexterity_kernel: String,
    pub owner_account_id: String,
    pub actor_principal_id: String,
    pub authenticated_session_id: String,
    pub access_space_id: String,
    pub workspace_id: String,
    pub record: AssociationRecordV1,
    pub original_operation_id: Uuid,
    pub request_id: String,
    pub causation_ref: String,
    pub trace_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssociationOuterV1 {
    pub record_kind: String,
    pub run_id: String,
    pub event_stream_version: u32,
    pub event_payload_json: String,
    pub issuance_operation_id: Uuid,
}

/// Observed row and ledger facts; these values do not grant access to either source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssociationLinksV1 {
    pub record_id: String,
    pub row_event_id: String,
    pub row_event_seq: i64,
    pub row_transaction_seq: i64,
    pub row_event_stream_version: u32,
    pub ledger_event_id: String,
    pub ledger_event_sequence: i64,
    pub ledger_record_id: String,
    pub ledger_source_component: String,
    pub ledger_aggregate_type: String,
    pub ledger_aggregate_id: String,
    pub ledger_event_type: String,
    pub ledger_event_version: String,
    pub ledger_payload_sha256: String,
}

/// Comparison data only. A host must authorize cleanup and check current CAS independently.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupCasWitnessV1 {
    pub original_operation_id: Uuid,
    pub association_record_id: String,
    pub event_id: String,
    pub event_payload_sha256: String,
    pub actual_instance: InstanceGenerationV1,
    pub source_chain_sha256: String,
    pub original_issuer: Scope5V1,
    pub child_scope: Scope5V1,
    pub full_intent: Value,
    pub inventory: ChildCleanupInventoryV1,
}

pub struct CheckedProducerAssociation {
    record: CheckedAssociationRecord,
    event_payload_json: String,
    event_payload_sha256: String,
    outer_json: String,
    cleanup_cas_witness: CleanupCasWitnessV1,
}

impl CheckedProducerAssociation {
    pub fn record_json(&self) -> &str { self.record.record_json() }
    pub fn event_payload_json(&self) -> &str { &self.event_payload_json }
    pub fn event_payload_sha256(&self) -> &str { &self.event_payload_sha256 }
    pub fn outer_json(&self) -> &str { &self.outer_json }
    pub fn body(&self) -> &AssociationBodyV1 { self.record.body() }
    pub fn cleanup_cas_witness(&self) -> &CleanupCasWitnessV1 { &self.cleanup_cas_witness }
}

pub fn encode_checked(
    body: AssociationBodyV1,
    original_stream: &str,
    expected_record_id: &str,
    expected_inventory: &ExpectedAssociationInventoryV1,
    observed: &AssociationLinksV1,
) -> Result<CheckedProducerAssociation, AssociationCodecError> {
    finish(encode_record_checked(body, original_stream, expected_inventory)?, expected_record_id, observed)
}

pub fn decode_checked(
    record_json: &str,
    outer_json: &str,
    expected_record_id: &str,
    expected_inventory: &ExpectedAssociationInventoryV1,
    observed: &AssociationLinksV1,
) -> Result<CheckedProducerAssociation, AssociationCodecError> {
    let record = decode_record_checked(record_json, expected_inventory)?;
    let outer: AssociationOuterV1 = parse_canonical(outer_json)?;
    let payload: AssociationEventPayloadV1 = parse_canonical(&outer.event_payload_json)?;
    if outer.record_kind != "producer_association" || outer.event_stream_version != 1 {
        return Err(AssociationCodecError::Shape("outer kind or version"));
    }
    if outer.run_id != record.body().run.run_id
        || outer.issuance_operation_id != record.body().original_operation_id {
        return Err(AssociationCodecError::IdentityMismatch("outer identity"));
    }
    let expected_payload = event_payload(&record);
    if payload != expected_payload {
        return Err(AssociationCodecError::IdentityMismatch("full event payload"));
    }
    // Compare complete serialized bytes as well as typed fields; no projection is a hash preimage.
    if outer.event_payload_json != canonical_json(&expected_payload)? {
        return Err(AssociationCodecError::NonCanonical);
    }
    let checked = finish(record, expected_record_id, observed)?;
    if checked.outer_json != outer_json {
        return Err(AssociationCodecError::IdentityMismatch("full outer"));
    }
    Ok(checked)
}

fn event_payload(record: &CheckedAssociationRecord) -> AssociationEventPayloadV1 {
    let body = record.body();
    let issuer = &body.original_issuer;
    AssociationEventPayloadV1 {
        dexterity_kernel: "Dexterity".into(),
        owner_account_id: issuer.owner_account_id.clone(),
        actor_principal_id: issuer.actor_principal_id.clone(),
        authenticated_session_id: issuer.authenticated_session_id.clone(),
        access_space_id: issuer.access_space_id.clone(),
        workspace_id: issuer.workspace_id.clone(),
        record: record.record().clone(),
        original_operation_id: body.original_operation_id,
        request_id: body.request_id.clone(),
        causation_ref: body.causation_ref.clone(),
        trace_id: body.run.trace_id.clone(),
    }
}

fn check_links(
    body: &AssociationBodyV1,
    expected_record_id: &str,
    observed: &AssociationLinksV1,
    payload_sha256: &str,
) -> Result<(), AssociationCodecError> {
    if observed.record_id != expected_record_id || observed.ledger_record_id != expected_record_id {
        return Err(AssociationCodecError::LinkMismatch("host record key"));
    }
    if observed.row_event_id.is_empty() || observed.row_event_id != observed.ledger_event_id {
        return Err(AssociationCodecError::LinkMismatch("event identity"));
    }
    if observed.row_event_seq <= 0
        || observed.row_event_seq != observed.row_transaction_seq
        || observed.row_event_seq != observed.ledger_event_sequence {
        return Err(AssociationCodecError::LinkMismatch("event counters"));
    }
    if observed.row_event_stream_version != 1 {
        return Err(AssociationCodecError::Shape("row stream version"));
    }
    if observed.ledger_source_component != "model_lane"
        || observed.ledger_aggregate_type != "producer_association"
        || observed.ledger_aggregate_id != body.original_operation_id.to_string()
        || observed.ledger_event_type != "MODEL_LANE_AUTHORITY_RECORDED"
        || observed.ledger_event_version != "kernel_event_v1" {
        return Err(AssociationCodecError::LinkMismatch("ledger envelope"));
    }
    if observed.ledger_payload_sha256 != payload_sha256 {
        return Err(AssociationCodecError::PayloadHashMismatch);
    }
    Ok(())
}

fn finish(
    record: CheckedAssociationRecord,
    expected_record_id: &str,
    observed: &AssociationLinksV1,
) -> Result<CheckedProducerAssociation, AssociationCodecError> {
    let event_payload_json = canonical_json(&event_payload(&record))?;
    let event_payload_sha256 = sha256_hex(event_payload_json.as_bytes());
    let body = record.body();
    check_links(body, expected_record_id, observed, &event_payload_sha256)?;
    let outer_json = canonical_json(&AssociationOuterV1 {
        record_kind: "producer_association".into(),
        run_id: body.run.run_id.clone(),
        event_stream_version: 1,
        event_payload_json: event_payload_json.clone(),
        issuance_operation_id: body.original_operation_id,
    })?;
    let cleanup_cas_witness = CleanupCasWitnessV1 {
        original_operation_id: body.original_operation_id,
        association_record_id: observed.record_id.clone(),
        event_id: observed.row_event_id.clone(),
        event_payload_sha256: event_payload_sha256.clone(),
        actual_instance: body.actual_instance.clone(),
        source_chain_sha256: record.source_chain_sha256().to_owned(),
        original_issuer: body.original_issuer.clone(),
        child_scope: body.child_scope.clone(),
        full_intent: serde_json::to_value(body).map_err(|_| AssociationCodecError::InvalidJson)?,
        inventory: body.cleanup.clone(),
    };
    Ok(CheckedProducerAssociation { record, event_payload_json, event_payload_sha256, outer_json, cleanup_cas_witness })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::association::{parse_strict, tests::fixture};
    use serde_json::json;

    fn input() -> (AssociationBodyV1, ExpectedAssociationInventoryV1, AssociationLinksV1) {
        let (body, expected) = fixture();
        let record = encode_record_checked(body.clone(), "original-stream", &expected).unwrap();
        let hash = sha256_hex(canonical_json(&event_payload(&record)).unwrap().as_bytes());
        let links = AssociationLinksV1 {
            record_id: "host-key".into(), row_event_id: "event-id".into(),
            row_event_seq: 7, row_transaction_seq: 7, row_event_stream_version: 1,
            ledger_event_id: "event-id".into(), ledger_event_sequence: 7,
            ledger_record_id: "host-key".into(), ledger_source_component: "model_lane".into(),
            ledger_aggregate_type: "producer_association".into(),
            ledger_aggregate_id: body.original_operation_id.to_string(),
            ledger_event_type: "MODEL_LANE_AUTHORITY_RECORDED".into(),
            ledger_event_version: "kernel_event_v1".into(), ledger_payload_sha256: hash,
        };
        (body, expected, links)
    }

    fn checked() -> (CheckedProducerAssociation, ExpectedAssociationInventoryV1, AssociationLinksV1) {
        let (body, expected, links) = input();
        let checked = encode_checked(body, "original-stream", "host-key", &expected, &links).unwrap();
        (checked, expected, links)
    }

    fn outer_with_payload(checked: &CheckedProducerAssociation, payload: &str) -> String {
        let mut outer: AssociationOuterV1 = parse_canonical(checked.outer_json()).unwrap();
        outer.event_payload_json = payload.into();
        canonical_json(&outer).unwrap()
    }

    #[test]
    fn complete_envelope_round_trip_preserves_original_five_and_cleanup_witness() {
        let (checked, expected, links) = checked();
        let decoded = decode_checked(checked.record_json(), checked.outer_json(), "host-key", &expected, &links).unwrap();
        assert_eq!(decoded.body(), checked.body());
        assert_eq!(decoded.event_payload_json(), checked.event_payload_json());
        let payload: AssociationEventPayloadV1 = parse_canonical(decoded.event_payload_json()).unwrap();
        let issuer = &decoded.body().original_issuer;
        assert_eq!(payload.owner_account_id, issuer.owner_account_id);
        assert_eq!(payload.actor_principal_id, issuer.actor_principal_id);
        assert_eq!(payload.authenticated_session_id, issuer.authenticated_session_id);
        assert_eq!(payload.access_space_id, issuer.access_space_id);
        assert_eq!(payload.workspace_id, issuer.workspace_id);
        let witness = decoded.cleanup_cas_witness();
        assert_eq!(witness.full_intent, serde_json::to_value(decoded.body()).unwrap());
        assert_eq!(witness.inventory, expected.created);
        assert_eq!(witness.event_payload_sha256, links.ledger_payload_sha256);
        assert_eq!(witness.event_id, links.row_event_id);
        assert_eq!(witness.actual_instance.generation, u64::MAX);
        assert_eq!(witness.child_scope, decoded.body().child_scope);
    }

    #[test]
    fn full_payload_hash_cannot_be_replaced_by_inner_record_hash() {
        let (checked, expected, mut links) = checked();
        assert_eq!(checked.event_payload_sha256(), sha256_hex(checked.event_payload_json().as_bytes()));
        links.ledger_payload_sha256 = sha256_hex(checked.record_json().as_bytes());
        assert!(matches!(decode_checked(checked.record_json(), checked.outer_json(), "host-key", &expected, &links), Err(AssociationCodecError::PayloadHashMismatch)));
    }

    #[test]
    fn each_flat_issuer_field_is_bound_to_original_not_child_scope() {
        let (checked, expected, links) = checked();
        for field in ["owner_account_id", "actor_principal_id", "authenticated_session_id", "access_space_id", "workspace_id"] {
            let mut payload: Value = parse_strict(checked.event_payload_json()).unwrap();
            payload[field] = json!("different-scope");
            let payload = canonical_json(&payload).unwrap();
            let mut observed = links.clone();
            observed.ledger_payload_sha256 = sha256_hex(payload.as_bytes());
            let outer = outer_with_payload(&checked, &payload);
            assert!(matches!(decode_checked(checked.record_json(), &outer, "host-key", &expected, &observed), Err(AssociationCodecError::IdentityMismatch("full event payload"))));
        }
    }

    #[test]
    fn duplicated_operation_request_cause_trace_and_record_are_exact() {
        let (checked, expected, links) = checked();
        for field in ["original_operation_id", "request_id", "causation_ref", "trace_id", "dexterity_kernel", "record"] {
            let mut payload: Value = parse_strict(checked.event_payload_json()).unwrap();
            payload[field] = match field {
                "original_operation_id" => json!("018f0000-0000-7000-8000-000000000002"),
                "record" => { let mut record = payload["record"].clone(); record["event_ledger_stream_id"] = json!("other-stream"); record },
                _ => json!("different"),
            };
            let outer = outer_with_payload(&checked, &canonical_json(&payload).unwrap());
            assert!(decode_checked(checked.record_json(), &outer, "host-key", &expected, &links).is_err());
        }
    }

    #[test]
    fn strict_outer_and_embedded_payload_reject_duplicates_unknowns_and_noncanonical_bytes() {
        let (checked, expected, links) = checked();
        let duplicate_outer = format!("{{\"run_id\":\"run\",{}", &checked.outer_json()[1..]);
        assert!(matches!(decode_checked(checked.record_json(), &duplicate_outer, "host-key", &expected, &links), Err(AssociationCodecError::DuplicateKey)));
        let duplicate_event = format!("{{\"trace_id\":\"trace\",{}", &checked.event_payload_json()[1..]);
        let duplicate_event = outer_with_payload(&checked, &duplicate_event);
        assert!(matches!(decode_checked(checked.record_json(), &duplicate_event, "host-key", &expected, &links), Err(AssociationCodecError::DuplicateKey)));
        for target in ["outer", "event"] {
            let mut raw: Value = parse_strict(if target == "outer" { checked.outer_json() } else { checked.event_payload_json() }).unwrap();
            raw["unknown"] = json!(true);
            let rendered = canonical_json(&raw).unwrap();
            let outer = if target == "outer" { rendered } else { outer_with_payload(&checked, &rendered) };
            assert!(matches!(decode_checked(checked.record_json(), &outer, "host-key", &expected, &links), Err(AssociationCodecError::UnknownField)));
        }
        let noncanonical = outer_with_payload(&checked, &format!(" {}", checked.event_payload_json()));
        assert!(matches!(decode_checked(checked.record_json(), &noncanonical, "host-key", &expected, &links), Err(AssociationCodecError::NonCanonical)));
    }

    #[test]
    fn host_key_event_identity_counters_and_row_version_are_independently_checked() {
        let (body, expected, links) = input();
        for index in 0..9 {
            let mut bad = links.clone();
            match index {
                0 => bad.record_id = "other".into(),
                1 => bad.ledger_record_id = "other".into(),
                2 => bad.row_event_id = "other".into(),
                3 => bad.ledger_event_id = "other".into(),
                4 => { bad.row_event_seq = 0; bad.row_transaction_seq = 0; bad.ledger_event_sequence = 0; },
                5 => bad.row_transaction_seq += 1,
                6 => bad.ledger_event_sequence += 1,
                7 => bad.row_event_stream_version = 2,
                _ => { bad.row_event_seq = -1; bad.row_transaction_seq = -1; bad.ledger_event_sequence = -1; },
            }
            assert!(encode_checked(body.clone(), "original-stream", "host-key", &expected, &bad).is_err());
        }
        assert!(encode_checked(body, "original-stream", "different-host-key", &expected, &links).is_err());
    }

    #[test]
    fn ledger_component_aggregate_operation_type_and_version_are_fixed() {
        let (body, expected, links) = input();
        for index in 0..5 {
            let mut bad = links.clone();
            match index {
                0 => bad.ledger_source_component = "other".into(),
                1 => bad.ledger_aggregate_type = "other".into(),
                2 => bad.ledger_aggregate_id = "different-operation".into(),
                3 => bad.ledger_event_type = "other".into(),
                _ => bad.ledger_event_version = "other".into(),
            }
            assert!(matches!(encode_checked(body.clone(), "original-stream", "host-key", &expected, &bad), Err(AssociationCodecError::LinkMismatch("ledger envelope"))));
        }
    }

    #[test]
    fn outer_kind_version_run_and_operation_cannot_diverge() {
        let (checked, expected, links) = checked();
        for index in 0..4 {
            let mut outer: AssociationOuterV1 = parse_canonical(checked.outer_json()).unwrap();
            match index {
                0 => outer.record_kind = "other".into(),
                1 => outer.event_stream_version = 2,
                2 => outer.run_id = "other".into(),
                _ => outer.issuance_operation_id = Uuid::parse_str("018f0000-0000-7000-8000-000000000002").unwrap(),
            }
            assert!(decode_checked(checked.record_json(), &canonical_json(&outer).unwrap(), "host-key", &expected, &links).is_err());
        }
    }
}
