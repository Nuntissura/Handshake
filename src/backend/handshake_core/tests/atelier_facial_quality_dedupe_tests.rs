use handshake_core::atelier::facial::{
    generate_facial_ingest_analysis, FacialIngestAnalysisItem, FacialIngestAnalysisRow,
    GenerateFacialIngestAnalysisRequest,
};
use handshake_core::atelier::facial_native::models::{
    ARCFACE_ENV_KEY, FRAMING_CLOSEUP_ENV_KEY, FRAMING_THREEQUARTER_ENV_KEY,
    IDENTITY_COUNT_THRESHOLD_ENV_KEY, IDENTITY_MARGIN_ENV_KEY, IDENTITY_THRESHOLD_ENV_KEY,
    LANDMARK_ENV_KEY, YUNET_ENV_KEY,
};
use sha2::{Digest, Sha256};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::Mutex;

static IDENTITY_ENV_LOCK: Mutex<()> = Mutex::new(());

fn item(
    item_id: &str,
    file_name: &str,
    content_hash: Option<&str>,
    byte_len: i64,
) -> FacialIngestAnalysisItem {
    FacialIngestAnalysisItem {
        item_id: item_id.to_owned(),
        source_ref: format!("dataset://source/{file_name}"),
        local_path_hint: None,
        file_name: file_name.to_owned(),
        byte_len,
        content_hash: content_hash.map(ToOwned::to_owned),
        lane: "pending".to_owned(),
    }
}

fn analysis(
    items: Vec<FacialIngestAnalysisItem>,
) -> handshake_core::atelier::facial::FacialIngestAnalysisExport {
    with_identity_env_cleared(|| {
        generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-000000000026".to_owned(),
            profile: "quality+dedupe+review".to_owned(),
            requested_by: "facial-agent-026".to_owned(),
            items,
        })
        .expect("facial analysis")
    })
}

fn with_identity_env_cleared<T, F: FnOnce() -> T>(f: F) -> T {
    let _guard = IDENTITY_ENV_LOCK.lock().expect("identity env lock");
    let keys = [
        ARCFACE_ENV_KEY,
        YUNET_ENV_KEY,
        LANDMARK_ENV_KEY,
        IDENTITY_THRESHOLD_ENV_KEY,
        IDENTITY_MARGIN_ENV_KEY,
        IDENTITY_COUNT_THRESHOLD_ENV_KEY,
        FRAMING_CLOSEUP_ENV_KEY,
        FRAMING_THREEQUARTER_ENV_KEY,
    ];
    let saved = keys
        .iter()
        .map(|key| (*key, std::env::var(key).ok()))
        .collect::<Vec<_>>();
    for key in keys {
        std::env::remove_var(key);
    }
    let result = catch_unwind(AssertUnwindSafe(f));
    for key in keys {
        std::env::remove_var(key);
    }
    for (key, value) in saved {
        if let Some(value) = value {
            std::env::set_var(key, value);
        }
    }
    match result {
        Ok(value) => value,
        Err(payload) => resume_unwind(payload),
    }
}

#[test]
fn facial_quality_rows_use_native_source_fields_not_proxy() {
    let export = analysis(vec![item("item-a", "a.jpg", Some("hash-a"), 2_000_000)]);

    let row = &export.rows[0];
    assert_eq!(row.quality_source, "facet_native_metadata_only_v1");
    assert_ne!(row.quality_source, "handshake_native_proxy_v1");
    assert_eq!(row.quality_source_family, "facet");
    assert_eq!(row.quality_feature_id, "facet:quality_pass");
    assert_eq!(row.quality_method, "native_metadata_only_quality_score");
    assert_eq!(
        row.quality_metrics["source"].as_str(),
        Some("facet_native_metadata_only_v1")
    );
    assert_eq!(
        row.quality_metrics["status"].as_str(),
        Some("metadata_only_degraded")
    );
    assert_eq!(
        row.ofiq_quality["source"].as_str(),
        Some("python_ofiq_native_metadata_only_v1")
    );
    assert!(row.ofiq_quality["missing_source_dimensions"]
        .as_array()
        .expect("missing source dimensions")
        .iter()
        .any(|value| value == "sharpness"));
    assert_eq!(
        export.summary.quality_source,
        "facet_native_metadata_only_v1"
    );
    assert_eq!(
        export.analysis_json["summary"]["native_feature_outputs"]["python-ofiq:vector_quality"]
            ["feature_id"]
            .as_str(),
        Some("python-ofiq:vector_quality")
    );
    assert!(export.analysis_json["summary"]["native_feature_outputs"]
        ["python-ofiq:vector_quality"]["missing_source_dimensions"]
        .as_array()
        .expect("summary missing source dimensions")
        .iter()
        .any(|value| value == "sharpness"));
}

#[test]
fn facial_dedupe_exact_hash_groups_are_stable_and_missing_hashes_are_singletons() {
    let export_a = analysis(vec![
        item("item-a", "a.jpg", Some("same-hash"), 2_000_000),
        item("item-b", "b.jpg", Some("same-hash"), 1_000_000),
        item("item-c", "c.jpg", None, 1_000_000),
        item("item-d", "c.jpg", None, 1_000_000),
    ]);
    let export_b = analysis(vec![
        item("item-b", "b.jpg", Some("same-hash"), 1_000_000),
        item("item-c", "c.jpg", None, 1_000_000),
        item("item-d", "c.jpg", None, 1_000_000),
        item("item-a", "a.jpg", Some("same-hash"), 2_000_000),
    ]);

    let a_group = export_a.rows[0].duplicate_group_id.clone();
    let b_group = export_b
        .rows
        .iter()
        .find(|row| row.item_id == "item-a")
        .expect("item-a in second export")
        .duplicate_group_id
        .clone();
    assert_eq!(a_group, b_group);
    assert_eq!(export_a.rows[0].duplicate_group_size, 2);
    assert_eq!(export_a.rows[0].duplicate_role, "representative");
    assert_eq!(export_a.rows[1].duplicate_group_size, 2);
    assert_eq!(export_a.rows[1].duplicate_role, "duplicate");
    assert_eq!(export_a.rows[2].duplicate_group_size, 1);
    assert_eq!(export_a.rows[2].duplicate_role, "singleton");
    assert_eq!(export_a.rows[3].duplicate_group_size, 1);
    assert_eq!(export_a.rows[3].duplicate_role, "singleton");
    assert_ne!(
        export_a.rows[2].duplicate_group_id,
        export_a.rows[3].duplicate_group_id
    );
    assert_eq!(
        export_a.rows[2].dedupe_source,
        "imagededup_native_missing_hash_singleton_v1"
    );
    assert_eq!(
        export_a.analysis_json["summary"]["native_feature_outputs"]["imagededup:hash_duplicates"]
            ["count"]
            .as_u64(),
        Some(1)
    );
    assert_eq!(
        export_a.analysis_json["summary"]["native_feature_outputs"]["imagededup:remove_candidates"]
            ["policy"]["non_destructive"]
            .as_bool(),
        Some(true)
    );
    assert_eq!(
        export_a.analysis_json["summary"]["native_feature_outputs"]["imagededup:remove_candidates"]
            ["remove_list"][0]["action"]
            .as_str(),
        Some("review_remove_candidate")
    );
    assert_eq!(
        export_a.analysis_json["summary"]["native_feature_outputs"]["imagededup:remove_candidates"]
            ["remove_list"][0]["keep"]
            .as_str(),
        Some("dataset://source/a.jpg")
    );
}

#[test]
fn facial_legacy_v1_rows_deserialize_with_compat_defaults() {
    let legacy_row = serde_json::json!({
        "item_id": "legacy-a",
        "source_ref": "dataset://source/legacy-a.jpg",
        "file_name": "legacy-a.jpg",
        "lane": "pending",
        "byte_len": 1234,
        "content_hash": "legacy-hash",
        "decode_status": "probe_unavailable",
        "quality_source": "handshake_native_proxy_v1",
        "quality_score": 42,
        "quality_band": "weak",
        "headshot_candidate": false,
        "duplicate_group_id": "facial-dedupe-legacy",
        "duplicate_group_size": 1,
        "duplicate_role": "singleton",
        "dedupe_source": "content_hash_exact",
        "identity_proxy_key": "facial-identity-proxy-legacy",
        "identity_source": "handshake_proxy_no_model",
        "identity_verdict": "proxy_unverified",
        "review_recommendation": "review",
        "reasons": ["legacy_artifact"],
    });

    let row: FacialIngestAnalysisRow =
        serde_json::from_value(legacy_row).expect("legacy @1 row remains readable");

    assert_eq!(row.quality_source_family, "legacy_unknown");
    assert_eq!(row.quality_feature_id, "legacy:quality_proxy");
    assert_eq!(
        row.quality_metrics["status"].as_str(),
        Some("legacy_missing")
    );
    assert_eq!(row.dedupe_source_family, "legacy_unknown");
    assert_eq!(row.dedupe_feature_id, "legacy:dedupe_proxy");
    assert_eq!(row.dedupe_record["status"].as_str(), Some("legacy_missing"));
}

#[test]
fn facial_artifact_hashes_are_stable_for_equal_inputs() {
    let items = vec![
        item("item-a", "a.jpg", Some("hash-a"), 2_000_000),
        item("item-b", "b.jpg", Some("hash-b"), 1_000_000),
    ];
    let export_a = analysis(items.clone());
    let export_b = analysis(items);

    assert_eq!(export_a.analysis_sha256, export_b.analysis_sha256);
    assert_eq!(export_a.content_hash, export_b.content_hash);
    assert_eq!(
        export_a.summary.native_run.run_hash,
        export_b.summary.native_run.run_hash
    );
}

#[test]
fn facial_artifact_hash_changes_when_classification_source_hash_changes() {
    let export_a = analysis(vec![item("item-a", "a.jpg", Some("hash-a"), 2_000_000)]);
    let export_b = analysis(vec![item("item-a", "a.jpg", Some("hash-b"), 2_000_000)]);

    assert_ne!(export_a.analysis_sha256, export_b.analysis_sha256);
    assert_ne!(export_a.content_hash, export_b.content_hash);
    assert_ne!(
        export_a.rows[0].duplicate_group_id,
        export_b.rows[0].duplicate_group_id
    );
}

#[test]
fn facial_ediffiqa_model_features_are_explicitly_unavailable() {
    let export = analysis(vec![item("item-a", "a.jpg", Some("hash-a"), 2_000_000)]);

    let ediffiqa = &export.analysis_json["summary"]["native_feature_outputs"]["ediffiqa"];
    assert_eq!(ediffiqa["status"].as_str(), Some("unavailable"));
    assert!(ediffiqa["features"]
        .as_array()
        .expect("features array")
        .iter()
        .any(|entry| entry["feature_id"] == "ediffiqa:batch_inference"
            && entry["reason"] == "ediffiqa_model_not_configured"));
}

#[test]
fn facial_local_image_inspection_is_read_only() {
    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 15, 4, 0, 9,
        251, 3, 253, 167, 164, 37, 219, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let dir = tempfile::tempdir().expect("tempdir");
    let image_path = dir.path().join("probe.png");
    std::fs::write(&image_path, PNG_1X1).expect("write png");
    let before = sha256_file(&image_path);

    let export = with_identity_env_cleared(|| {
        generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-000000000026".to_owned(),
            profile: "quality+dedupe".to_owned(),
            requested_by: "facial-agent-026".to_owned(),
            items: vec![FacialIngestAnalysisItem {
                item_id: "item-local".to_owned(),
                source_ref: image_path.to_string_lossy().into_owned(),
                local_path_hint: None,
                file_name: "probe.png".to_owned(),
                byte_len: PNG_1X1.len() as i64,
                content_hash: Some("png-content-hash".to_owned()),
                lane: "pending".to_owned(),
            }],
        })
        .expect("local image analysis")
    });

    let after = sha256_file(&image_path);
    assert_eq!(before, after);
    assert_eq!(export.rows[0].decode_status, "decoded");
    assert_eq!(export.rows[0].image_width, Some(1));
    assert_eq!(export.rows[0].image_height, Some(1));
}

fn sha256_file(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).expect("read file");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
