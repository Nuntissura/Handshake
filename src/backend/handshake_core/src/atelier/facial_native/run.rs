use super::common::{
    FacialNativeRunFeatureRecord, FacialNativeRunReport, FacialNativeRunRequest,
    FACIAL_NATIVE_REGISTRY_SCHEMA_ID, FACIAL_NATIVE_RUN_SCHEMA_ID,
};
use super::registry::facial_feature_registry;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub fn build_facial_native_run_report(
    request: FacialNativeRunRequest,
) -> Result<FacialNativeRunReport, String> {
    if request.batch_id.trim().is_empty() || request.batch_id.trim() != request.batch_id {
        return Err("facial native run batch_id must not be empty or padded".to_owned());
    }
    if request.profile.trim().is_empty() || request.profile.trim() != request.profile {
        return Err("facial native run profile must not be empty or padded".to_owned());
    }
    if request.requested_by.trim().is_empty() || request.requested_by.trim() != request.requested_by
    {
        return Err("facial native run requested_by must not be empty or padded".to_owned());
    }
    if request.profile_tokens.is_empty() {
        return Err("facial native run requires at least one profile token".to_owned());
    }

    let registry = facial_feature_registry();
    let mut selected_feature_ids = Vec::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut degraded_reasons = Vec::<String>::new();
    let mut feature_records = Vec::with_capacity(registry.len());
    for feature in registry {
        let selected = feature.is_selected_by_profile(&request.profile_tokens);
        if selected {
            selected_feature_ids.push(feature.feature_id.clone());
            *status_counts.entry(feature.status.clone()).or_insert(0) += 1;
            if let Some(reason) = &feature.unavailable_reason {
                degraded_reasons.push(format!("{}:{reason}", feature.feature_id));
            }
        }
        feature_records.push(FacialNativeRunFeatureRecord {
            feature_id: feature.feature_id,
            capability: feature.capability,
            source_family: feature.source_family,
            status: feature.status,
            native_route: feature.native_route,
            artifact_contract: feature.artifact_contract,
            selected,
            unavailable_reason: feature.unavailable_reason,
        });
    }
    selected_feature_ids.sort();
    degraded_reasons.sort();

    let run_status = if degraded_reasons.is_empty() {
        "native_ready"
    } else if status_counts
        .keys()
        .any(|status| status.starts_with("native_"))
    {
        "native_partial_degraded"
    } else {
        "native_degraded_unavailable"
    }
    .to_owned();

    let run_hash = stable_run_hash(
        &request.batch_id,
        &request.profile,
        &request.profile_tokens,
        &selected_feature_ids,
        &request.items,
    )?;
    let actor_hash = stable_actor_hash(&request.requested_by)?;
    let run_id = format!("facial-native-run-{run_hash}-actor-{actor_hash}");
    let decoded_count = request
        .items
        .iter()
        .filter(|item| item.decode_status == "decoded")
        .count();

    Ok(FacialNativeRunReport {
        schema_id: FACIAL_NATIVE_RUN_SCHEMA_ID.to_owned(),
        registry_schema_id: FACIAL_NATIVE_REGISTRY_SCHEMA_ID.to_owned(),
        run_id,
        batch_id: request.batch_id,
        profile: request.profile,
        requested_by: request.requested_by,
        profile_tokens: request.profile_tokens,
        item_count: request.items.len(),
        decoded_count,
        selected_feature_ids,
        run_status,
        status_counts,
        degraded_reasons,
        feature_records,
        artifact_refs: Vec::new(),
        manifest_refs: Vec::new(),
        run_hash,
    })
}

fn stable_run_hash(
    batch_id: &str,
    _profile: &str,
    profile_tokens: &[String],
    selected_feature_ids: &[String],
    items: &[super::common::FacialNativeRunItem],
) -> Result<String, String> {
    let mut canonical_profile_tokens = profile_tokens.to_vec();
    canonical_profile_tokens.sort();
    let canonical_profile = canonical_profile_tokens.join("+");
    let mut canonical_items = items.to_vec();
    canonical_items.sort_by(|left, right| {
        (
            left.item_id.as_str(),
            left.source_ref.as_str(),
            left.lane.as_str(),
            left.decode_status.as_str(),
            left.content_hash.as_deref().unwrap_or_default(),
        )
            .cmp(&(
                right.item_id.as_str(),
                right.source_ref.as_str(),
                right.lane.as_str(),
                right.decode_status.as_str(),
                right.content_hash.as_deref().unwrap_or_default(),
            ))
    });
    let payload = (
        FACIAL_NATIVE_RUN_SCHEMA_ID,
        batch_id,
        &canonical_profile,
        &canonical_profile_tokens,
        selected_feature_ids,
        &canonical_items,
    );
    let bytes = serde_json::to_vec(&payload)
        .map_err(|err| format!("serialize facial native run hash payload failed: {err}"))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn stable_actor_hash(requested_by: &str) -> Result<String, String> {
    let payload = (FACIAL_NATIVE_RUN_SCHEMA_ID, "requested_by", requested_by);
    let bytes = serde_json::to_vec(&payload)
        .map_err(|err| format!("serialize facial native actor hash payload failed: {err}"))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atelier::facial_native::common::FacialNativeRunItem;

    fn request(profile: &str) -> FacialNativeRunRequest {
        FacialNativeRunRequest {
            batch_id: "018f7848-1111-7000-9000-000000000025".to_owned(),
            profile: profile.to_owned(),
            requested_by: "facial-agent-025".to_owned(),
            profile_tokens: profile.split('+').map(ToOwned::to_owned).collect(),
            items: vec![FacialNativeRunItem {
                item_id: "item-a".to_owned(),
                source_ref: "artifact://.handshake/artifacts/L1/test/payload".to_owned(),
                lane: "pending".to_owned(),
                decode_status: "decoded".to_owned(),
                content_hash: Some("hash-a".to_owned()),
            }],
        }
    }

    #[test]
    fn facial_native_run_selects_profile_features_and_degraded_reasons() {
        let report =
            build_facial_native_run_report(request("quality+identity")).expect("native run report");

        assert_eq!(report.schema_id, FACIAL_NATIVE_RUN_SCHEMA_ID);
        assert_eq!(report.requested_by, "facial-agent-025");
        assert_eq!(report.decoded_count, 1);
        assert!(report
            .selected_feature_ids
            .contains(&"facet:quality_pass".to_owned()));
        assert!(report
            .selected_feature_ids
            .contains(&"python-ofiq:setup_data".to_owned()));
        assert!(report
            .selected_feature_ids
            .contains(&"python-ofiq:scalar_quality".to_owned()));
        assert!(report
            .selected_feature_ids
            .contains(&"python-ofiq:vector_quality".to_owned()));
        assert!(report
            .selected_feature_ids
            .contains(&"identity_gate:arcface_embedding".to_owned()));
        assert_eq!(report.run_status, "native_partial_degraded");
        assert!(!report
            .degraded_reasons
            .iter()
            .any(|reason| reason.starts_with("identity_gate:arcface_embedding:")));
        assert!(report.feature_records.iter().any(|record| record.feature_id
            == "identity_gate:arcface_embedding"
            && record.status == "runtime_gated_model_backed"
            && record.native_route == "atelier.facial.identity.arcface_runtime_gated"
            && record.unavailable_reason.is_none()));
        assert_eq!(
            report
                .status_counts
                .get("native_quality_metadata_only_v1")
                .copied(),
            Some(4)
        );
    }

    #[test]
    fn facial_native_run_hash_is_stable_for_equal_inputs() {
        let report_a =
            build_facial_native_run_report(request("quality+dedupe")).expect("native run report a");
        let report_b =
            build_facial_native_run_report(request("quality+dedupe")).expect("native run report b");

        assert_eq!(report_a.run_hash, report_b.run_hash);
        assert_eq!(report_a.run_id, report_b.run_id);
    }

    #[test]
    fn facial_native_run_hash_excludes_actor_attribution() {
        let mut request_b = request("quality+identity");
        request_b.requested_by = "facial-agent-026".to_owned();

        let report_a =
            build_facial_native_run_report(request("quality+identity")).expect("native run a");
        let report_b = build_facial_native_run_report(request_b).expect("native run b");

        assert_eq!(report_a.run_hash, report_b.run_hash);
        assert_ne!(report_a.run_id, report_b.run_id);
        assert_ne!(report_a.requested_by, report_b.requested_by);
    }

    #[test]
    fn facial_native_run_hash_canonicalizes_item_order() {
        let request_a = request("quality+dedupe");
        let mut request_b = request("dedupe+quality");
        request_b.items.push(FacialNativeRunItem {
            item_id: "item-b".to_owned(),
            source_ref: "artifact://.handshake/artifacts/L1/test-b/payload".to_owned(),
            lane: "pending".to_owned(),
            decode_status: "decoded".to_owned(),
            content_hash: Some("hash-b".to_owned()),
        });
        let mut request_a = request_a;
        request_a.items.insert(0, request_b.items[1].clone());

        let report_a = build_facial_native_run_report(request_a).expect("native run a");
        let report_b = build_facial_native_run_report(request_b).expect("native run b");

        assert_eq!(report_a.run_hash, report_b.run_hash);
        assert_eq!(report_a.run_id, report_b.run_id);
    }

    #[test]
    fn facial_native_run_rejects_padded_actor() {
        let mut request = request("quality");
        request.requested_by = " facial-agent-025".to_owned();

        let err = build_facial_native_run_report(request).expect_err("padded actor rejected");
        assert!(err.contains("requested_by"));
    }
}
