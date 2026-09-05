use super::common::FacialNativeImageContext;
use super::models::FacialIdentityModelConfig;
#[cfg(feature = "facial-onnx-runtime")]
use image::RgbImage;
use serde::{Deserialize, Serialize};
#[cfg(feature = "facial-onnx-runtime")]
use sha2::{Digest, Sha256};
#[cfg(feature = "facial-onnx-runtime")]
use std::path::Path;
#[cfg(feature = "facial-onnx-runtime")]
use tract_onnx::prelude::*;

pub const IDENTITY_SOURCE_NO_MODEL: &str = "handshake_proxy_no_model";
pub const IDENTITY_SOURCE_MODEL_UNAVAILABLE: &str = "handshake_identity_model_unavailable";
pub const IDENTITY_SOURCE_REAL: &str = "real";
pub const IDENTITY_VERDICT_PROXY_UNVERIFIED: &str = "proxy_unverified";
pub const IDENTITY_VERDICT_MODEL_UNAVAILABLE: &str = "model_unavailable";
pub const IDENTITY_VERDICT_UNSURE: &str = "unsure";
pub const IDENTITY_FEATURE_ID: &str = "identity_gate:arcface_embedding";
pub const IDENTITY_METHOD_NO_MODEL: &str = "native_no_model_identity_gate_v1";
pub const IDENTITY_METHOD_MODEL_UNAVAILABLE: &str = "native_model_configured_not_loaded_v1";
pub const IDENTITY_METHOD_ARCFACE_RESIZE: &str = "arcface_onnx_resize_112_v1";
#[cfg(feature = "facial-onnx-runtime")]
const ARCFACE_INPUT: usize = 112;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialIdentityAnalysis {
    pub proxy_key: String,
    pub source: String,
    pub source_family: String,
    pub feature_id: String,
    pub method: String,
    pub verdict: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detector_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detector_model_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landmark_model_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_margin: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count_threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_box: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_frac: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_crop_sharpness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yaw_estimate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yaw_ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eyes_open: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ear_left: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ear_right: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landmark_conf_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_dimensions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub record: serde_json::Value,
}

impl FacialIdentityAnalysis {
    pub fn is_real_model_backed(&self) -> bool {
        self.source == "real" && matches!(self.verdict.as_str(), "match" | "no_match" | "unsure")
    }
}

pub struct FacialIdentityRuntime {
    #[cfg(feature = "facial-onnx-runtime")]
    embedder: Option<ArcFaceEmbedder>,
    load_error: Option<String>,
}

impl FacialIdentityRuntime {
    pub fn unavailable(load_error: Option<String>) -> Self {
        Self {
            #[cfg(feature = "facial-onnx-runtime")]
            embedder: None,
            load_error,
        }
    }
}

#[cfg(not(feature = "facial-onnx-runtime"))]
impl FacialIdentityRuntime {
    pub fn load(config: &FacialIdentityModelConfig) -> Self {
        if config.arcface.sha256.is_some() {
            Self::unavailable(Some(
                "native_arcface_onnx_runtime_feature_disabled".to_owned(),
            ))
        } else {
            Self::unavailable(config.arcface.error.clone())
        }
    }
}

#[cfg(feature = "facial-onnx-runtime")]
impl FacialIdentityRuntime {
    pub fn load(config: &FacialIdentityModelConfig) -> Self {
        let Some(path) = config.arcface.path.as_deref() else {
            return Self::unavailable(config.arcface.error.clone());
        };
        if config.arcface.sha256.is_none() {
            return Self::unavailable(config.arcface.error.clone());
        }
        match ArcFaceEmbedder::load(path) {
            Ok(embedder) => Self {
                embedder: Some(embedder),
                load_error: None,
            },
            Err(err) => Self::unavailable(Some(err)),
        }
    }
}

#[cfg(feature = "facial-onnx-runtime")]
struct ArcFaceEmbedder {
    model: TypedRunnableModel<TypedModel>,
}

#[cfg(feature = "facial-onnx-runtime")]
impl ArcFaceEmbedder {
    fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|err| format!("read arcface model: {err}"))?;
        let model = tract_onnx::onnx()
            .model_for_read(&mut std::io::Cursor::new(&bytes))
            .map_err(|err| format!("parse arcface onnx: {err}"))?
            .into_optimized()
            .map_err(|err| format!("optimize arcface onnx: {err}"))?
            .into_runnable()
            .map_err(|err| format!("make arcface runnable: {err}"))?;
        Ok(Self { model })
    }

    fn embed_file(&self, path: &Path) -> Result<Vec<f32>, String> {
        let image = image::open(path)
            .map_err(|err| format!("decode image for arcface identity: {err}"))?
            .to_rgb8();
        self.embed_image(&image)
    }

    fn embed_image(&self, image: &RgbImage) -> Result<Vec<f32>, String> {
        let resized = image::imageops::resize(
            image,
            ARCFACE_INPUT as u32,
            ARCFACE_INPUT as u32,
            image::imageops::FilterType::Triangle,
        );
        let mut data = vec![0f32; 3 * ARCFACE_INPUT * ARCFACE_INPUT];
        for y in 0..ARCFACE_INPUT {
            for x in 0..ARCFACE_INPUT {
                let px = resized.get_pixel(x as u32, y as u32);
                for c in 0..3usize {
                    data[c * ARCFACE_INPUT * ARCFACE_INPUT + y * ARCFACE_INPUT + x] =
                        (px[c] as f32 - 127.5) / 128.0;
                }
            }
        }
        let input = Tensor::from_shape(&[1, 3, ARCFACE_INPUT, ARCFACE_INPUT], &data)
            .map_err(|err| format!("build arcface tensor: {err}"))?;
        let result = self
            .model
            .run(tvec!(input.into()))
            .map_err(|err| format!("arcface inference: {err}"))?;
        let view = result[0]
            .to_array_view::<f32>()
            .map_err(|err| format!("read arcface output: {err}"))?;
        let mut embedding = view.iter().copied().collect::<Vec<_>>();
        l2_normalize(&mut embedding);
        Ok(embedding)
    }
}

#[cfg(feature = "facial-onnx-runtime")]
fn l2_normalize(values: &mut [f32]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values.iter_mut() {
            *value /= norm;
        }
    }
}

#[cfg(feature = "facial-onnx-runtime")]
fn embedding_sha256(values: &[f32]) -> String {
    let mut hasher = Sha256::new();
    for value in values {
        hasher.update(value.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

pub fn analyze_identity(
    ctx: &FacialNativeImageContext,
    config: &FacialIdentityModelConfig,
    runtime: &FacialIdentityRuntime,
    proxy_key: String,
) -> FacialIdentityAnalysis {
    if !config.any_model_configured() {
        let record = serde_json::json!({
            "schema_id": "hsk.atelier.facial_identity_row@1",
            "status": "unavailable_no_model",
            "source": IDENTITY_SOURCE_NO_MODEL,
            "source_family": "no_model",
            "feature_id": IDENTITY_FEATURE_ID,
            "method": IDENTITY_METHOD_NO_MODEL,
            "verdict": IDENTITY_VERDICT_PROXY_UNVERIFIED,
            "proxy_key": proxy_key,
            "model_backed": false,
            "reason": "arcface_model_not_configured",
        });
        return FacialIdentityAnalysis {
            proxy_key,
            source: IDENTITY_SOURCE_NO_MODEL.to_owned(),
            source_family: "no_model".to_owned(),
            feature_id: IDENTITY_FEATURE_ID.to_owned(),
            method: IDENTITY_METHOD_NO_MODEL.to_owned(),
            verdict: IDENTITY_VERDICT_PROXY_UNVERIFIED.to_owned(),
            model_sha256: None,
            detector_source: None,
            detector_model_sha256: None,
            landmark_model_sha256: None,
            threshold: None,
            required_margin: None,
            count_threshold: None,
            face_box: None,
            face_frac: None,
            face_score: None,
            face_crop_sharpness: None,
            yaw_estimate: None,
            yaw_ratio: None,
            eyes_open: None,
            ear_left: None,
            ear_right: None,
            landmark_conf_min: None,
            embedding_sha256: None,
            embedding_dimensions: None,
            error: None,
            record,
        };
    }

    #[cfg(feature = "facial-onnx-runtime")]
    if let Some(embedder) = runtime.embedder.as_ref() {
        if let Some(local_path) = identity_image_path(ctx) {
            match embedder.embed_file(Path::new(local_path)) {
                Ok(embedding) => {
                    let embedding_dimensions = embedding.len();
                    let embedding_sha256 = embedding_sha256(&embedding);
                    let detector_source = if config.yunet.sha256.is_some() {
                        Some("YuNet".to_owned())
                    } else {
                        None
                    };
                    let record = serde_json::json!({
                        "schema_id": "hsk.atelier.facial_identity_row@1",
                        "status": "model_backed_embedding",
                        "source": IDENTITY_SOURCE_REAL,
                        "source_family": "ArcFace",
                        "feature_id": IDENTITY_FEATURE_ID,
                        "method": IDENTITY_METHOD_ARCFACE_RESIZE,
                        "verdict": IDENTITY_VERDICT_UNSURE,
                        "proxy_key": proxy_key,
                        "model_backed": true,
                        "threshold": config.threshold,
                        "required_margin": config.required_margin,
                        "count_threshold": config.count_threshold,
                        "framing_closeup_min": config.framing_closeup_min,
                        "framing_threequarter_min": config.framing_threequarter_min,
                        "models": {
                            "arcface": config.arcface.to_redacted_json(),
                            "yunet": config.yunet.to_redacted_json(),
                            "landmark": config.landmark.to_redacted_json(),
                        },
                        "embedding_sha256": embedding_sha256,
                        "embedding_dimensions": embedding_dimensions,
                        "alignment": "resize_112",
                        "decision_reason": "no_reference_identity_supplied",
                        "face_box": serde_json::Value::Null,
                        "face_frac": serde_json::Value::Null,
                        "face_score": serde_json::Value::Null,
                        "face_crop_sharpness": serde_json::Value::Null,
                        "yaw_estimate": serde_json::Value::Null,
                        "yaw_ratio": serde_json::Value::Null,
                        "landmarks": serde_json::Value::Null,
                        "image_decode_status": ctx.decode_status,
                    });
                    return FacialIdentityAnalysis {
                        proxy_key,
                        source: IDENTITY_SOURCE_REAL.to_owned(),
                        source_family: "ArcFace".to_owned(),
                        feature_id: IDENTITY_FEATURE_ID.to_owned(),
                        method: IDENTITY_METHOD_ARCFACE_RESIZE.to_owned(),
                        verdict: IDENTITY_VERDICT_UNSURE.to_owned(),
                        model_sha256: config.arcface.sha256.clone(),
                        detector_source,
                        detector_model_sha256: config.yunet.sha256.clone(),
                        landmark_model_sha256: config.landmark.sha256.clone(),
                        threshold: Some(config.threshold),
                        required_margin: Some(config.required_margin),
                        count_threshold: Some(config.count_threshold),
                        face_box: None,
                        face_frac: None,
                        face_score: None,
                        face_crop_sharpness: None,
                        yaw_estimate: None,
                        yaw_ratio: None,
                        eyes_open: None,
                        ear_left: None,
                        ear_right: None,
                        landmark_conf_min: None,
                        embedding_sha256: Some(embedding_sha256),
                        embedding_dimensions: Some(embedding_dimensions),
                        error: None,
                        record,
                    };
                }
                Err(err) => {
                    return unavailable_model_analysis(
                        ctx,
                        config,
                        proxy_key,
                        format!("arcface_embedding_failed: {err}"),
                    );
                }
            }
        }
        return unavailable_model_analysis(
            ctx,
            config,
            proxy_key,
            "local_image_path_unavailable_for_arcface_identity".to_owned(),
        );
    }

    let error = runtime.load_error.clone().unwrap_or_else(|| {
        config
            .arcface
            .error
            .clone()
            .unwrap_or_else(|| "arcface_model_unavailable".to_owned())
    });
    unavailable_model_analysis(ctx, config, proxy_key, error)
}

#[cfg(feature = "facial-onnx-runtime")]
fn identity_image_path(ctx: &FacialNativeImageContext) -> Option<&str> {
    ctx.local_path_hint
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            let source_ref = ctx.source_ref.as_str();
            if source_ref.contains("://") || source_ref.trim().is_empty() {
                None
            } else {
                Some(source_ref)
            }
        })
}

fn unavailable_model_analysis(
    ctx: &FacialNativeImageContext,
    config: &FacialIdentityModelConfig,
    proxy_key: String,
    error: String,
) -> FacialIdentityAnalysis {
    let detector_source = if config.yunet.sha256.is_some() {
        Some("YuNet".to_owned())
    } else {
        None
    };
    let record = serde_json::json!({
        "schema_id": "hsk.atelier.facial_identity_row@1",
        "status": "model_unavailable",
        "source": IDENTITY_SOURCE_MODEL_UNAVAILABLE,
        "source_family": config.source_family(),
        "feature_id": IDENTITY_FEATURE_ID,
        "method": IDENTITY_METHOD_MODEL_UNAVAILABLE,
        "verdict": IDENTITY_VERDICT_MODEL_UNAVAILABLE,
        "proxy_key": proxy_key,
        "model_backed": false,
        "threshold": config.threshold,
        "required_margin": config.required_margin,
        "count_threshold": config.count_threshold,
        "framing_closeup_min": config.framing_closeup_min,
        "framing_threequarter_min": config.framing_threequarter_min,
        "models": {
            "arcface": config.arcface.to_redacted_json(),
            "yunet": config.yunet.to_redacted_json(),
            "landmark": config.landmark.to_redacted_json(),
        },
        "face_box": serde_json::Value::Null,
        "face_frac": serde_json::Value::Null,
        "face_score": serde_json::Value::Null,
        "face_crop_sharpness": serde_json::Value::Null,
        "yaw_estimate": serde_json::Value::Null,
        "yaw_ratio": serde_json::Value::Null,
        "landmarks": serde_json::Value::Null,
        "image_decode_status": ctx.decode_status,
        "error": error,
    });

    FacialIdentityAnalysis {
        proxy_key,
        source: IDENTITY_SOURCE_MODEL_UNAVAILABLE.to_owned(),
        source_family: config.source_family().to_owned(),
        feature_id: IDENTITY_FEATURE_ID.to_owned(),
        method: IDENTITY_METHOD_MODEL_UNAVAILABLE.to_owned(),
        verdict: IDENTITY_VERDICT_MODEL_UNAVAILABLE.to_owned(),
        model_sha256: config.arcface.sha256.clone(),
        detector_source,
        detector_model_sha256: config.yunet.sha256.clone(),
        landmark_model_sha256: config.landmark.sha256.clone(),
        threshold: Some(config.threshold),
        required_margin: Some(config.required_margin),
        count_threshold: Some(config.count_threshold),
        face_box: None,
        face_frac: None,
        face_score: None,
        face_crop_sharpness: None,
        yaw_estimate: None,
        yaw_ratio: None,
        eyes_open: None,
        ear_left: None,
        ear_right: None,
        landmark_conf_min: None,
        embedding_sha256: None,
        embedding_dimensions: None,
        error: Some(error),
        record,
    }
}

pub fn native_feature_output(
    config: &FacialIdentityModelConfig,
    row_records: &[serde_json::Value],
) -> serde_json::Value {
    config.to_summary_json(row_records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atelier::facial_native::models::{
        discover_identity_model_config, ARCFACE_ENV_KEY, IDENTITY_THRESHOLD_ENV_KEY,
    };
    use std::collections::BTreeMap;

    fn ctx() -> FacialNativeImageContext {
        FacialNativeImageContext {
            item_id: "item-identity".to_owned(),
            source_ref: "dataset://source/identity.jpg".to_owned(),
            local_path_hint: None,
            file_name: "identity.jpg".to_owned(),
            lane: "pending".to_owned(),
            byte_len: 123,
            content_hash: Some("hash-identity".to_owned()),
            decode_status: "probe_unavailable".to_owned(),
            image_width: None,
            image_height: None,
            megapixels: None,
        }
    }

    #[test]
    fn facial_identity_default_never_claims_real_match() {
        let config = discover_identity_model_config(|_| None);
        let runtime = FacialIdentityRuntime::load(&config);
        let analysis = analyze_identity(
            &ctx(),
            &config,
            &runtime,
            "facial-identity-proxy-abc".to_owned(),
        );

        assert_eq!(analysis.source, IDENTITY_SOURCE_NO_MODEL);
        assert_eq!(analysis.verdict, IDENTITY_VERDICT_PROXY_UNVERIFIED);
        assert!(!analysis.is_real_model_backed());
        assert!(analysis.model_sha256.is_none());
        assert!(analysis.threshold.is_none());
    }

    #[test]
    fn facial_identity_configured_model_hash_stays_unavailable_without_runtime() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model_path = dir.path().join("arcface.onnx");
        std::fs::write(&model_path, b"fake-arcface-model").expect("write model");
        let mut env = BTreeMap::new();
        env.insert(ARCFACE_ENV_KEY, model_path.to_string_lossy().into_owned());
        env.insert(IDENTITY_THRESHOLD_ENV_KEY, "0.81".to_owned());
        let config = discover_identity_model_config(|key| env.get(key).cloned());
        let runtime = FacialIdentityRuntime::load(&config);

        let analysis = analyze_identity(
            &ctx(),
            &config,
            &runtime,
            "facial-identity-proxy-abc".to_owned(),
        );

        assert_eq!(analysis.source, IDENTITY_SOURCE_MODEL_UNAVAILABLE);
        assert_eq!(analysis.verdict, IDENTITY_VERDICT_MODEL_UNAVAILABLE);
        assert_eq!(analysis.threshold, Some(0.81));
        assert!(analysis.model_sha256.is_some());
        assert!(!analysis.is_real_model_backed());
        assert_eq!(
            analysis.record["model_backed"].as_bool(),
            Some(false),
            "hashing a configured file is not identity proof"
        );
    }
}
