use handshake_core::atelier::facial::{
    generate_facial_ingest_analysis, FacialIngestAnalysisItem, FacialIngestAnalysisRow,
    GenerateFacialIngestAnalysisRequest,
};
use handshake_core::atelier::facial_native::identity::{
    IDENTITY_SOURCE_MODEL_UNAVAILABLE, IDENTITY_SOURCE_NO_MODEL,
    IDENTITY_VERDICT_MODEL_UNAVAILABLE, IDENTITY_VERDICT_PROXY_UNVERIFIED,
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

fn item(item_id: &str, file_name: &str, content_hash: Option<&str>) -> FacialIngestAnalysisItem {
    FacialIngestAnalysisItem {
        item_id: item_id.to_owned(),
        source_ref: format!("dataset://source/{file_name}"),
        local_path_hint: None,
        file_name: file_name.to_owned(),
        byte_len: 2_000_000,
        content_hash: content_hash.map(ToOwned::to_owned),
        lane: "pending".to_owned(),
    }
}

fn local_item(
    item_id: &str,
    file_name: &str,
    content_hash: Option<&str>,
    path: &std::path::Path,
) -> FacialIngestAnalysisItem {
    FacialIngestAnalysisItem {
        item_id: item_id.to_owned(),
        source_ref: format!("file://{file_name}"),
        local_path_hint: Some(path.to_string_lossy().into_owned()),
        file_name: file_name.to_owned(),
        byte_len: std::fs::metadata(path)
            .map(|meta| meta.len() as i64)
            .unwrap_or(0),
        content_hash: content_hash.map(ToOwned::to_owned),
        lane: "pending".to_owned(),
    }
}

fn source_ref_local_item(
    item_id: &str,
    file_name: &str,
    content_hash: Option<&str>,
    path: &std::path::Path,
) -> FacialIngestAnalysisItem {
    FacialIngestAnalysisItem {
        item_id: item_id.to_owned(),
        source_ref: path.to_string_lossy().into_owned(),
        local_path_hint: None,
        file_name: file_name.to_owned(),
        byte_len: std::fs::metadata(path)
            .map(|meta| meta.len() as i64)
            .unwrap_or(0),
        content_hash: content_hash.map(ToOwned::to_owned),
        lane: "pending".to_owned(),
    }
}

fn analyze(
    items: Vec<FacialIngestAnalysisItem>,
) -> handshake_core::atelier::facial::FacialIngestAnalysisExport {
    generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
        batch_id: "018f7848-1111-7000-9000-000000000027".to_owned(),
        profile: "quality+identity+review".to_owned(),
        requested_by: "facial-agent-027".to_owned(),
        items,
    })
    .expect("facial identity analysis")
}

fn with_identity_env<F: FnOnce()>(pairs: &[(&str, String)], f: F) {
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
    for (key, value) in pairs {
        std::env::set_var(key, value);
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
    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

#[test]
fn facial_identity_no_model_never_emits_match_or_real_source() {
    with_identity_env(&[], || {
        let export = analyze(vec![
            item("item-a", "a.jpg", Some("hash-a")),
            item("item-b", "b.jpg", Some("hash-b")),
        ]);

        for row in &export.rows {
            assert_eq!(row.identity_source, IDENTITY_SOURCE_NO_MODEL);
            assert_eq!(row.identity_verdict, IDENTITY_VERDICT_PROXY_UNVERIFIED);
            assert!(row.identity_model_sha256.is_none());
            assert!(row.detector_model_sha256.is_none());
            assert!(row.identity_threshold.is_none());
            assert!(row.face_box.is_none());
            assert_eq!(
                row.identity_record["status"].as_str(),
                Some("unavailable_no_model")
            );
            assert_ne!(row.identity_verdict, "match");
            assert_ne!(row.identity_verdict, "no_match");
        }
        assert_eq!(export.summary.identity_source, IDENTITY_SOURCE_NO_MODEL);
        assert_eq!(
            export.analysis_json["summary"]["identity_provenance"]["proxy_unverified_row_count"]
                .as_u64(),
            Some(2)
        );
        assert_eq!(
            export.analysis_json["summary"]["native_feature_outputs"]
                ["identity_gate:arcface_embedding"]["runtime_status"]
                .as_str(),
            Some("not_configured")
        );
        assert_eq!(
            export.receipt_json["identity_provenance"],
            export.analysis_json["summary"]["identity_provenance"]
        );
    });
}

#[test]
fn facial_identity_configured_model_hash_is_provenance_not_verdict_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let arcface = dir.path().join("arcface.onnx");
    let yunet = dir.path().join("yunet.onnx");
    let landmark = dir.path().join("pipnet.onnx");
    std::fs::write(&arcface, b"fake-arcface-model").expect("write arcface");
    std::fs::write(&yunet, b"fake-yunet-model").expect("write yunet");
    std::fs::write(&landmark, b"fake-pipnet-model").expect("write landmark");
    let arcface_sha = sha256_bytes(b"fake-arcface-model");
    let yunet_sha = sha256_bytes(b"fake-yunet-model");
    let landmark_sha = sha256_bytes(b"fake-pipnet-model");

    with_identity_env(
        &[
            (ARCFACE_ENV_KEY, arcface.to_string_lossy().into_owned()),
            (YUNET_ENV_KEY, yunet.to_string_lossy().into_owned()),
            (LANDMARK_ENV_KEY, landmark.to_string_lossy().into_owned()),
            (IDENTITY_THRESHOLD_ENV_KEY, "0.73".to_owned()),
            (IDENTITY_MARGIN_ENV_KEY, "0.21".to_owned()),
            (IDENTITY_COUNT_THRESHOLD_ENV_KEY, "0.88".to_owned()),
            (FRAMING_CLOSEUP_ENV_KEY, "0.12".to_owned()),
            (FRAMING_THREEQUARTER_ENV_KEY, "0.04".to_owned()),
        ],
        || {
            let export = analyze(vec![item("item-a", "a.jpg", Some("hash-a"))]);
            let row = &export.rows[0];

            assert_eq!(row.identity_source, IDENTITY_SOURCE_MODEL_UNAVAILABLE);
            assert_eq!(row.identity_verdict, IDENTITY_VERDICT_MODEL_UNAVAILABLE);
            assert_eq!(
                row.identity_model_sha256.as_deref(),
                Some(arcface_sha.as_str())
            );
            assert_eq!(row.detector_source.as_deref(), Some("YuNet"));
            assert_eq!(
                row.detector_model_sha256.as_deref(),
                Some(yunet_sha.as_str())
            );
            assert_eq!(
                row.landmark_model_sha256.as_deref(),
                Some(landmark_sha.as_str())
            );
            assert_eq!(row.identity_threshold, Some(0.73));
            assert_eq!(row.identity_required_margin, Some(0.21));
            assert_eq!(row.identity_count_threshold, Some(0.88));
            assert!(row.face_box.is_none());
            assert_eq!(row.identity_record["model_backed"].as_bool(), Some(false));
            assert_eq!(
                row.identity_record["models"]["arcface"]["sha256"].as_str(),
                Some(arcface_sha.as_str())
            );
            assert_eq!(
                row.identity_record["models"]["yunet"]["sha256"].as_str(),
                Some(yunet_sha.as_str())
            );
            assert_eq!(
                row.identity_record["models"]["landmark"]["sha256"].as_str(),
                Some(landmark_sha.as_str())
            );
            assert!(!export.analysis_json.to_string().contains("arcface.onnx"));
            assert_eq!(
                export.receipt_json["identity_provenance"],
                export.analysis_json["summary"]["identity_provenance"]
            );
            let provenance = &export.receipt_json["identity_provenance"];
            assert_eq!(provenance["threshold"].as_f64(), Some(0.73));
            assert_eq!(provenance["required_margin"].as_f64(), Some(0.21));
            assert_eq!(provenance["count_threshold"].as_f64(), Some(0.88));
            assert_eq!(provenance["framing_closeup_min"].as_f64(), Some(0.12));
            assert_eq!(provenance["framing_threequarter_min"].as_f64(), Some(0.04));
            if !cfg!(feature = "facial-onnx-runtime") {
                assert_eq!(
                    provenance["runtime_status"].as_str(),
                    Some("runtime_feature_disabled")
                );
                assert_eq!(
                    provenance["runtime_reason"].as_str(),
                    Some("native_arcface_onnx_runtime_feature_disabled")
                );
                assert_eq!(
                    provenance["runtime_error_counts"]
                        ["native_arcface_onnx_runtime_feature_disabled"]
                        .as_u64(),
                    Some(1)
                );
            }
            assert_eq!(
                provenance["models"]["arcface"]["sha256"].as_str(),
                Some(arcface_sha.as_str())
            );
            assert_eq!(
                provenance["models"]["yunet"]["sha256"].as_str(),
                Some(yunet_sha.as_str())
            );
            assert_eq!(
                provenance["models"]["landmark"]["sha256"].as_str(),
                Some(landmark_sha.as_str())
            );
            assert_eq!(
                provenance["model_hashes_by_role"]["arcface"].as_str(),
                Some(arcface_sha.as_str())
            );
            assert_eq!(
                provenance["model_hashes_by_role"]["yunet"].as_str(),
                Some(yunet_sha.as_str())
            );
            assert_eq!(
                provenance["model_hashes_by_role"]["landmark"].as_str(),
                Some(landmark_sha.as_str())
            );
            assert_eq!(
                export.analysis_json["summary"]["identity_provenance"]
                    ["model_unavailable_row_count"]
                    .as_u64(),
                Some(1)
            );
        },
    );
}

#[test]
fn facial_identity_invalid_model_path_stays_unavailable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("missing.onnx");

    with_identity_env(
        &[(ARCFACE_ENV_KEY, missing.to_string_lossy().into_owned())],
        || {
            let export = analyze(vec![item("item-a", "a.jpg", Some("hash-a"))]);
            let row = &export.rows[0];

            assert_eq!(row.identity_source, IDENTITY_SOURCE_MODEL_UNAVAILABLE);
            assert_eq!(row.identity_verdict, IDENTITY_VERDICT_MODEL_UNAVAILABLE);
            assert!(row.identity_model_sha256.is_none());
            assert_eq!(
                row.identity_record["models"]["arcface"]["status"].as_str(),
                Some("read_error")
            );
            assert_eq!(row.identity_record["model_backed"].as_bool(), Some(false));
            let provenance = &export.receipt_json["identity_provenance"];
            assert_eq!(
                provenance["runtime_status"].as_str(),
                Some("runtime_load_or_inference_failed")
            );
            assert_eq!(
                provenance["runtime_error_counts"]["arcface_runtime_unavailable"].as_u64(),
                Some(1)
            );
        },
    );
}

#[test]
fn facial_identity_legacy_rows_deserialize_with_new_fields_missing() {
    let legacy_row = serde_json::json!({
        "item_id": "legacy-a",
        "source_ref": "dataset://source/legacy-a.jpg",
        "file_name": "legacy-a.jpg",
        "lane": "pending",
        "byte_len": 1234,
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
        "reasons": ["legacy_artifact"]
    });

    let row: FacialIngestAnalysisRow = serde_json::from_value(legacy_row).expect("legacy row");

    assert!(row.identity_source_family.is_none());
    assert!(row.identity_model_sha256.is_none());
    assert_eq!(
        row.identity_record["status"].as_str(),
        Some("legacy_missing")
    );
}

#[test]
fn facial_identity_new_files_do_not_hardcode_machine_local_paths() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "src/atelier/facial_native/models.rs",
        "src/atelier/facial_native/identity.rs",
        "src/atelier/facial_native/landmarks.rs",
        "tests/atelier_facial_identity_tests.rs",
    ];
    let banned = [
        format!("{}{}{}", 'D', ':', '\\'),
        format!("{}{}{}", 'D', ':', '/'),
        format!("{}{}{}{}", 'C', ':', '\\', "Users"),
        ["LLM", " ", "projects"].concat(),
        ["USER", "PROFILE"].concat(),
    ];
    for file in files {
        let text = std::fs::read_to_string(root.join(file)).expect("read scanned file");
        for pattern in &banned {
            assert!(
                !text.contains(pattern),
                "{file} contains machine-local path marker {pattern:?}"
            );
        }
    }
}

#[test]
#[ignore = "env-gated local ONNX smoke; set HANDSHAKE_TEST_FACIAL_ARCFACE_ONNX to run"]
fn facial_identity_real_model_env_gate_smoke() {
    if !cfg!(feature = "facial-onnx-runtime") {
        eprintln!(
            "SKIP facial_identity_real_model_env_gate_smoke: build without facial-onnx-runtime"
        );
        return;
    }
    let Some(arcface) = std::env::var("HANDSHAKE_TEST_FACIAL_ARCFACE_ONNX").ok() else {
        eprintln!("SKIP facial_identity_real_model_env_gate_smoke: HANDSHAKE_TEST_FACIAL_ARCFACE_ONNX absent");
        return;
    };
    let mut env = vec![(ARCFACE_ENV_KEY, arcface)];
    if let Ok(yunet) = std::env::var("HANDSHAKE_TEST_FACIAL_YUNET_ONNX") {
        env.push((YUNET_ENV_KEY, yunet));
    }
    with_identity_env(&env, || {
        let dir = tempfile::tempdir().expect("tempdir");
        let image_path = dir.path().join("identity-smoke.png");
        let mut image = image::RgbImage::new(112, 112);
        for y in 0..112 {
            for x in 0..112 {
                image.put_pixel(
                    x,
                    y,
                    image::Rgb([(x.saturating_mul(2)) as u8, (y.saturating_mul(2)) as u8, 96]),
                );
            }
        }
        image.save(&image_path).expect("write smoke image");
        let source_ref_only_path = dir.path().join("identity-smoke-source-ref.png");
        std::fs::copy(&image_path, &source_ref_only_path).expect("copy smoke image");

        let export = analyze(vec![
            local_item("item-a", "identity-smoke.png", Some("hash-a"), &image_path),
            source_ref_local_item(
                "item-b",
                "identity-smoke-source-ref.png",
                Some("hash-b"),
                &source_ref_only_path,
            ),
        ]);
        assert_eq!(export.rows.len(), 2);
        let sha = export.rows[0]
            .identity_model_sha256
            .as_deref()
            .expect("configured model sha");
        assert_eq!(sha.len(), 64);
        for row in &export.rows {
            assert_eq!(row.identity_source, "real");
            assert_eq!(row.identity_verdict, "unsure");
            assert_eq!(
                row.identity_method.as_deref(),
                Some("arcface_onnx_resize_112_v1")
            );
            assert_eq!(row.identity_record["model_backed"].as_bool(), Some(true));
            assert_eq!(row.embedding_sha256.as_deref().map(str::len), Some(64));
            assert!(row.embedding_dimensions.unwrap_or(0) > 0);
        }
        assert_eq!(
            export.receipt_json["identity_provenance"]["runtime_status"].as_str(),
            Some("arcface_runtime_loaded")
        );
        assert_eq!(
            export.receipt_json["identity_provenance"]["real_model_backed_row_count"].as_u64(),
            Some(2)
        );
        assert_eq!(
            export.receipt_json["identity_provenance"]["runtime_error_counts"]
                .as_object()
                .map(|map| map.len()),
            Some(0)
        );
    });
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
