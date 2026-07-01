use super::common::FacialNativeFeature;

pub fn facial_feature_registry() -> Vec<FacialNativeFeature> {
    vec![
        FacialNativeFeature {
            feature_id: "facet:quality_pass".to_owned(),
            capability: "quality".to_owned(),
            source_family: "facet".to_owned(),
            native_field: "quality_score, quality_band, headshot_candidate".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "native_proxy_v1".to_owned(),
            native_route: "atelier.facial.quality.proxy_v1".to_owned(),
            provenance_note:
                "Handshake computes deterministic image/ref quality proxies until facet thresholds are ported."
                    .to_owned(),
            required_config_keys: vec![],
            unavailable_reason: None,
        },
        FacialNativeFeature {
            feature_id: "python-ofiq:scalar_quality".to_owned(),
            capability: "quality".to_owned(),
            source_family: "python-ofiq".to_owned(),
            native_field: "quality_score, quality_source".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.summary.quality_source"
                .to_owned(),
            status: "deferred_model_backed".to_owned(),
            native_route: "atelier.facial.quality.ofiq_unavailable".to_owned(),
            provenance_note:
                "OFIQ scalar/vector quality is not claimed until a native model path exists."
                    .to_owned(),
            required_config_keys: vec!["HANDSHAKE_FACIAL_OFIQ_MODEL".to_owned()],
            unavailable_reason: Some("ofiq_model_not_configured".to_owned()),
        },
        FacialNativeFeature {
            feature_id: "ediffiqa:quality_score".to_owned(),
            capability: "quality".to_owned(),
            source_family: "ediffiqa".to_owned(),
            native_field: "quality_score, quality_band".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "deferred_model_backed".to_owned(),
            native_route: "atelier.facial.quality.ediffiqa_unavailable".to_owned(),
            provenance_note:
                "eDifFIQA quality is mapped for future model-backed implementation.".to_owned(),
            required_config_keys: vec!["HANDSHAKE_FACIAL_EDIFFIQA_MODEL".to_owned()],
            unavailable_reason: Some("ediffiqa_model_not_configured".to_owned()),
        },
        FacialNativeFeature {
            feature_id: "imagededup:hash_duplicates".to_owned(),
            capability: "dedupe".to_owned(),
            source_family: "imagededup".to_owned(),
            native_field: "duplicate_group_id, duplicate_group_size, duplicate_role".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "native_content_hash_exact".to_owned(),
            native_route: "atelier.facial.dedupe.content_hash_exact_v1".to_owned(),
            provenance_note:
                "Handshake groups exact content_hash duplicates and leaves missing hashes as singletons."
                    .to_owned(),
            required_config_keys: vec![],
            unavailable_reason: None,
        },
        FacialNativeFeature {
            feature_id: "imagededup:cnn_duplicates".to_owned(),
            capability: "dedupe".to_owned(),
            source_family: "imagededup".to_owned(),
            native_field: "duplicate_group_id".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "deferred_model_backed".to_owned(),
            native_route: "atelier.facial.dedupe.cnn_unavailable".to_owned(),
            provenance_note:
                "CNN perceptual duplicate grouping is not claimed until a native model path is wired."
                    .to_owned(),
            required_config_keys: vec!["HANDSHAKE_FACIAL_IMAGEDEDUP_CNN_MODEL".to_owned()],
            unavailable_reason: Some("imagededup_cnn_model_not_configured".to_owned()),
        },
        FacialNativeFeature {
            feature_id: "identity_gate:yunet_detection".to_owned(),
            capability: "identity".to_owned(),
            source_family: "YuNet".to_owned(),
            native_field: "identity_source, identity_verdict".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "deferred_model_backed".to_owned(),
            native_route: "atelier.facial.identity.yunet_unavailable".to_owned(),
            provenance_note:
                "YuNet face detection is mapped but not claimed until model configuration is wired."
                    .to_owned(),
            required_config_keys: vec!["HANDSHAKE_FACIAL_YUNET_ONNX".to_owned()],
            unavailable_reason: Some("yunet_model_not_configured".to_owned()),
        },
        FacialNativeFeature {
            feature_id: "identity_gate:arcface_embedding".to_owned(),
            capability: "identity".to_owned(),
            source_family: "ArcFace".to_owned(),
            native_field: "identity_proxy_key, identity_source, identity_verdict".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "deferred_model_backed".to_owned(),
            native_route: "atelier.facial.identity.arcface_unavailable".to_owned(),
            provenance_note:
                "Rows expose identity proxy keys but never claim match/no_match without ArcFace assets."
                    .to_owned(),
            required_config_keys: vec!["HANDSHAKE_FACIAL_ARCFACE_ONNX".to_owned()],
            unavailable_reason: Some("arcface_model_not_configured".to_owned()),
        },
        FacialNativeFeature {
            feature_id: "identity_gate:pipnet_landmarks".to_owned(),
            capability: "identity".to_owned(),
            source_family: "PIPNet".to_owned(),
            native_field: "eyes_open, ear_left, ear_right, landmark_conf_min".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "deferred_model_backed".to_owned(),
            native_route: "atelier.facial.identity.pipnet_landmarks_unavailable".to_owned(),
            provenance_note:
                "PIPNet 98-point landmark parity is mapped from Facial but not claimed until the native model path is wired."
                    .to_owned(),
            required_config_keys: vec!["HANDSHAKE_FACIAL_LANDMARK_MODEL".to_owned()],
            unavailable_reason: Some("pipnet_landmark_model_not_configured".to_owned()),
        },
        FacialNativeFeature {
            feature_id: "deepface:detect".to_owned(),
            capability: "identity".to_owned(),
            source_family: "deepface".to_owned(),
            native_field: "decode_status, identity_source".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "deferred_model_backed".to_owned(),
            native_route: "atelier.facial.identity.deepface_detect_unavailable".to_owned(),
            provenance_note:
                "DeepFace detection parity is not claimed in the native route yet.".to_owned(),
            required_config_keys: vec!["HANDSHAKE_FACIAL_DEEPFACE_MODEL".to_owned()],
            unavailable_reason: Some("deepface_model_not_configured".to_owned()),
        },
        FacialNativeFeature {
            feature_id: "deepface:represent".to_owned(),
            capability: "identity".to_owned(),
            source_family: "deepface".to_owned(),
            native_field: "identity_proxy_key".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "deferred_model_backed".to_owned(),
            native_route: "atelier.facial.identity.deepface_represent_unavailable".to_owned(),
            provenance_note:
                "DeepFace embedding/representation is mapped for later native model-backed implementation."
                    .to_owned(),
            required_config_keys: vec!["HANDSHAKE_FACIAL_DEEPFACE_MODEL".to_owned()],
            unavailable_reason: Some("deepface_model_not_configured".to_owned()),
        },
        FacialNativeFeature {
            feature_id: "review:shard_claims".to_owned(),
            capability: "review".to_owned(),
            source_family: "Facial review ledger".to_owned(),
            native_field: "review_recommendation, reasons".to_owned(),
            artifact_contract: "hsk.atelier.facial_ingest_analysis@1.rows[]".to_owned(),
            status: "native_recommendation_v1".to_owned(),
            native_route: "atelier.facial.review.recommendation_v1".to_owned(),
            provenance_note:
                "Rows emit keep/review/cull recommendations for Argus and parallel Ingest reviewers."
                    .to_owned(),
            required_config_keys: vec![],
            unavailable_reason: None,
        },
        FacialNativeFeature {
            feature_id: "review:montage_export".to_owned(),
            capability: "review".to_owned(),
            source_family: "Facial review montage".to_owned(),
            native_field: "analysis_artifact_ref, receipt_ref".to_owned(),
            artifact_contract: "ArtifactStore application/json analysis and receipt".to_owned(),
            status: "unsupported_visual_export".to_owned(),
            native_route: "atelier.facial.review.montage_unavailable".to_owned(),
            provenance_note:
                "Analysis JSON/receipt exist; montage/contact-sheet visual export is tracked separately."
                    .to_owned(),
            required_config_keys: vec![],
            unavailable_reason: Some("montage_export_not_implemented".to_owned()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facial_native_registry_covers_required_source_families() {
        let registry = facial_feature_registry();
        for feature_id in [
            "facet:quality_pass",
            "python-ofiq:scalar_quality",
            "deepface:detect",
            "imagededup:hash_duplicates",
            "ediffiqa:quality_score",
            "identity_gate:yunet_detection",
            "identity_gate:arcface_embedding",
            "identity_gate:pipnet_landmarks",
            "review:shard_claims",
        ] {
            assert!(
                registry
                    .iter()
                    .any(|feature| feature.feature_id == feature_id),
                "{feature_id} missing from registry"
            );
        }
        assert!(registry.iter().any(|feature| {
            feature.feature_id == "review:montage_export"
                && feature.status == "unsupported_visual_export"
                && feature.unavailable_reason.as_deref() == Some("montage_export_not_implemented")
        }));
    }
}
