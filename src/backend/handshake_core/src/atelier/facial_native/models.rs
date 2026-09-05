use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const ARCFACE_ENV_KEY: &str = "HANDSHAKE_FACIAL_ARCFACE_ONNX";
pub const YUNET_ENV_KEY: &str = "HANDSHAKE_FACIAL_YUNET_ONNX";
pub const LANDMARK_ENV_KEY: &str = "HANDSHAKE_FACIAL_LANDMARK_MODEL";
pub const IDENTITY_THRESHOLD_ENV_KEY: &str = "HANDSHAKE_FACIAL_IDENTITY_THRESHOLD";
pub const IDENTITY_MARGIN_ENV_KEY: &str = "HANDSHAKE_FACIAL_IDENTITY_MARGIN";
pub const IDENTITY_COUNT_THRESHOLD_ENV_KEY: &str = "HANDSHAKE_FACIAL_IDENTITY_COUNT_THRESHOLD";
pub const FRAMING_CLOSEUP_ENV_KEY: &str = "HANDSHAKE_FACIAL_FRAMING_CLOSEUP_MIN";
pub const FRAMING_THREEQUARTER_ENV_KEY: &str = "HANDSHAKE_FACIAL_FRAMING_THREEQUARTER_MIN";

pub const DEFAULT_IDENTITY_THRESHOLD: f64 = 0.5;
pub const DEFAULT_IDENTITY_MARGIN: f64 = 0.1;
pub const DEFAULT_IDENTITY_COUNT_THRESHOLD: f64 = 0.9;
pub const DEFAULT_FRAMING_CLOSEUP_MIN: f64 = 0.09;
pub const DEFAULT_FRAMING_THREEQUARTER_MIN: f64 = 0.03;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FacialIdentityModelConfig {
    pub arcface: FacialModelAssetStatus,
    pub yunet: FacialModelAssetStatus,
    pub landmark: FacialModelAssetStatus,
    pub threshold: f64,
    pub required_margin: f64,
    pub count_threshold: f64,
    pub framing_closeup_min: f64,
    pub framing_threequarter_min: f64,
    pub runtime_status: String,
    pub runtime_reason: String,
}

impl FacialIdentityModelConfig {
    pub fn source_family(&self) -> &'static str {
        if self.arcface.sha256.is_some() {
            "ArcFace"
        } else {
            "no_model"
        }
    }

    pub fn any_model_configured(&self) -> bool {
        self.arcface.configured || self.yunet.configured || self.landmark.configured
    }

    pub fn configured_model_hashes(&self) -> Vec<String> {
        [&self.arcface, &self.yunet, &self.landmark]
            .into_iter()
            .filter_map(|asset| asset.sha256.clone())
            .collect()
    }

    pub fn configured_model_hashes_by_role(&self) -> serde_json::Value {
        serde_json::json!({
            "arcface": self.arcface.sha256,
            "yunet": self.yunet.sha256,
            "landmark": self.landmark.sha256,
        })
    }

    pub fn to_summary_json(&self, rows: &[serde_json::Value]) -> serde_json::Value {
        let mut verdict_counts = BTreeMap::<String, usize>::new();
        let mut source_counts = BTreeMap::<String, usize>::new();
        let mut runtime_error_counts = BTreeMap::<String, usize>::new();
        let mut real_model_backed_row_count = 0usize;
        let mut runtime_loaded = false;
        let mut runtime_feature_disabled = false;
        let mut runtime_loaded_no_image = false;
        for row in rows {
            if let Some(verdict) = row.get("verdict").and_then(|value| value.as_str()) {
                *verdict_counts.entry(verdict.to_owned()).or_insert(0) += 1;
            }
            if let Some(source) = row.get("source").and_then(|value| value.as_str()) {
                *source_counts.entry(source.to_owned()).or_insert(0) += 1;
            }
            let model_backed = row
                .get("model_backed")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let source_is_real = row
                .get("source")
                .and_then(|value| value.as_str())
                .is_some_and(|value| value == "real");
            let verdict_is_real = row
                .get("verdict")
                .and_then(|value| value.as_str())
                .is_some_and(|value| matches!(value, "match" | "no_match" | "unsure"));
            if model_backed || (source_is_real && verdict_is_real) {
                real_model_backed_row_count += 1;
            }
            if model_backed
                || row
                    .get("status")
                    .and_then(|value| value.as_str())
                    .is_some_and(|value| value == "model_backed_embedding")
            {
                runtime_loaded = true;
            }
            if row
                .get("error")
                .and_then(|value| value.as_str())
                .is_some_and(|value| value == "native_arcface_onnx_runtime_feature_disabled")
            {
                runtime_feature_disabled = true;
            }
            if let Some(error) = row.get("error").and_then(|value| value.as_str()) {
                let bucket = identity_runtime_error_bucket(error);
                if bucket == "local_image_path_unavailable_for_arcface_identity" {
                    runtime_loaded_no_image = true;
                }
                *runtime_error_counts.entry(bucket.to_owned()).or_insert(0) += 1;
            }
        }
        let model_unavailable_row_count = verdict_counts
            .get("model_unavailable")
            .copied()
            .unwrap_or(0);
        let proxy_unverified_row_count =
            verdict_counts.get("proxy_unverified").copied().unwrap_or(0);
        let runtime_status = if runtime_loaded {
            "arcface_runtime_loaded".to_owned()
        } else if runtime_feature_disabled {
            "runtime_feature_disabled".to_owned()
        } else if runtime_loaded_no_image {
            "arcface_runtime_loaded_no_image".to_owned()
        } else if !runtime_error_counts.is_empty() {
            "runtime_load_or_inference_failed".to_owned()
        } else {
            self.runtime_status.clone()
        };
        let runtime_reason = if runtime_loaded {
            "arcface_embedding_completed".to_owned()
        } else if runtime_feature_disabled {
            "native_arcface_onnx_runtime_feature_disabled".to_owned()
        } else if let Some((reason, _)) = runtime_error_counts.iter().next() {
            reason.clone()
        } else {
            self.runtime_reason.clone()
        };
        serde_json::json!({
            "schema_id": "hsk.atelier.facial_identity_provenance@1",
            "runtime_status": runtime_status,
            "runtime_reason": runtime_reason,
            "threshold": self.threshold,
            "required_margin": self.required_margin,
            "count_threshold": self.count_threshold,
            "framing_closeup_min": self.framing_closeup_min,
            "framing_threequarter_min": self.framing_threequarter_min,
            "models": {
                "arcface": self.arcface.to_redacted_json(),
                "yunet": self.yunet.to_redacted_json(),
                "landmark": self.landmark.to_redacted_json(),
            },
            "model_hashes": self.configured_model_hashes(),
            "model_hashes_by_role": self.configured_model_hashes_by_role(),
            "verdict_counts": verdict_counts,
            "source_counts": source_counts,
            "runtime_error_counts": runtime_error_counts,
            "real_model_backed_row_count": real_model_backed_row_count,
            "model_unavailable_row_count": model_unavailable_row_count,
            "proxy_unverified_row_count": proxy_unverified_row_count,
        })
    }
}

fn identity_runtime_error_bucket(error: &str) -> &'static str {
    if error == "native_arcface_onnx_runtime_feature_disabled" {
        "native_arcface_onnx_runtime_feature_disabled"
    } else if error == "local_image_path_unavailable_for_arcface_identity" {
        "local_image_path_unavailable_for_arcface_identity"
    } else if error.contains("arcface_embedding_failed") {
        "arcface_embedding_failed"
    } else if error.contains("parse arcface onnx") {
        "arcface_onnx_parse_failed"
    } else if error.contains("optimize arcface onnx") {
        "arcface_onnx_optimize_failed"
    } else if error.contains("make arcface runnable") {
        "arcface_onnx_runnable_failed"
    } else if error.contains("read arcface model") {
        "arcface_model_read_failed"
    } else if error.contains("decode image for arcface identity") {
        "arcface_image_decode_failed"
    } else if error == "path_value_must_not_be_padded" {
        "path_value_must_not_be_padded"
    } else if error == "arcface_model_unavailable" {
        "arcface_model_unavailable"
    } else {
        "arcface_runtime_unavailable"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacialModelAssetStatus {
    pub role: String,
    pub env_key: String,
    pub configured: bool,
    pub status: String,
    #[serde(skip)]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FacialModelAssetStatus {
    pub fn missing(role: &str, env_key: &str) -> Self {
        Self {
            role: role.to_owned(),
            env_key: env_key.to_owned(),
            configured: false,
            status: "not_configured".to_owned(),
            path: None,
            sha256: None,
            error: None,
        }
    }

    pub fn to_redacted_json(&self) -> serde_json::Value {
        serde_json::json!({
            "role": self.role,
            "env_key": self.env_key,
            "configured": self.configured,
            "status": self.status,
            "sha256": self.sha256,
            "error": self.error,
        })
    }
}

pub fn discover_identity_model_config_from_env() -> FacialIdentityModelConfig {
    let env = |key: &str| std::env::var(key).ok();
    discover_identity_model_config(env)
}

pub fn discover_identity_model_config<F>(mut env: F) -> FacialIdentityModelConfig
where
    F: FnMut(&str) -> Option<String>,
{
    let arcface = discover_asset("arcface_embedder", ARCFACE_ENV_KEY, &mut env);
    let yunet = discover_asset("yunet_detector", YUNET_ENV_KEY, &mut env);
    let landmark = discover_asset("pipnet_landmark", LANDMARK_ENV_KEY, &mut env);
    let threshold = parse_f64_env(
        &mut env,
        IDENTITY_THRESHOLD_ENV_KEY,
        DEFAULT_IDENTITY_THRESHOLD,
    );
    let required_margin = parse_f64_env(&mut env, IDENTITY_MARGIN_ENV_KEY, DEFAULT_IDENTITY_MARGIN);
    let count_threshold = parse_f64_env(
        &mut env,
        IDENTITY_COUNT_THRESHOLD_ENV_KEY,
        DEFAULT_IDENTITY_COUNT_THRESHOLD,
    );
    let framing_closeup_min = parse_f64_env(
        &mut env,
        FRAMING_CLOSEUP_ENV_KEY,
        DEFAULT_FRAMING_CLOSEUP_MIN,
    );
    let framing_threequarter_min = parse_f64_env(
        &mut env,
        FRAMING_THREEQUARTER_ENV_KEY,
        DEFAULT_FRAMING_THREEQUARTER_MIN,
    );
    let runtime_status = if arcface.sha256.is_some() {
        "configured_not_loaded"
    } else if arcface.configured {
        "configured_unavailable"
    } else {
        "not_configured"
    }
    .to_owned();
    let runtime_reason = if arcface.sha256.is_some() {
        "arcface_runtime_requires_feature_build_or_load_attempt"
    } else if arcface.configured {
        "arcface_model_configured_but_unreadable"
    } else {
        "arcface_model_not_configured"
    }
    .to_owned();

    FacialIdentityModelConfig {
        arcface,
        yunet,
        landmark,
        threshold,
        required_margin,
        count_threshold,
        framing_closeup_min,
        framing_threequarter_min,
        runtime_status,
        runtime_reason,
    }
}

fn discover_asset<F>(role: &str, env_key: &str, env: &mut F) -> FacialModelAssetStatus
where
    F: FnMut(&str) -> Option<String>,
{
    let Some(raw) = env(env_key) else {
        return FacialModelAssetStatus::missing(role, env_key);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return FacialModelAssetStatus {
            role: role.to_owned(),
            env_key: env_key.to_owned(),
            configured: false,
            status: "not_configured".to_owned(),
            path: None,
            sha256: None,
            error: None,
        };
    }
    if trimmed != raw {
        return FacialModelAssetStatus {
            role: role.to_owned(),
            env_key: env_key.to_owned(),
            configured: true,
            status: "invalid_path".to_owned(),
            path: None,
            sha256: None,
            error: Some("path_value_must_not_be_padded".to_owned()),
        };
    }
    let path = PathBuf::from(trimmed);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            FacialModelAssetStatus {
                role: role.to_owned(),
                env_key: env_key.to_owned(),
                configured: true,
                status: "configured_hashed".to_owned(),
                path: Some(path),
                sha256: Some(format!("{:x}", hasher.finalize())),
                error: None,
            }
        }
        Err(err) => FacialModelAssetStatus {
            role: role.to_owned(),
            env_key: env_key.to_owned(),
            configured: true,
            status: "read_error".to_owned(),
            path: Some(path),
            sha256: None,
            error: Some(err.kind().to_string()),
        },
    }
}

fn parse_f64_env<F>(env: &mut F, key: &str, default: f64) -> f64
where
    F: FnMut(&str) -> Option<String>,
{
    env(key)
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn config_from(values: BTreeMap<&str, String>) -> FacialIdentityModelConfig {
        discover_identity_model_config(|key| values.get(key).cloned())
    }

    #[test]
    fn facial_identity_model_config_hashes_configured_files_without_path_leak() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model_path = dir.path().join("arcface.onnx");
        std::fs::write(&model_path, b"fake-arcface-model").expect("write model");
        let mut env = BTreeMap::new();
        env.insert(ARCFACE_ENV_KEY, model_path.to_string_lossy().into_owned());
        env.insert(IDENTITY_THRESHOLD_ENV_KEY, "0.77".to_owned());

        let config = config_from(env);

        assert_eq!(config.arcface.status, "configured_hashed");
        assert_eq!(
            config.arcface.sha256.as_deref(),
            Some("0a755780fd8b9f21e0af0775fc52a1afa1ccfd57b5b17bca49293a5cfc0d31e6")
        );
        assert_eq!(config.threshold, 0.77);
        let json = config.arcface.to_redacted_json().to_string();
        assert!(!json.contains("arcface.onnx"));
        assert!(!json.contains(&dir.path().to_string_lossy().to_string()));
    }

    #[test]
    fn facial_identity_model_config_rejects_padded_path() {
        let mut env = BTreeMap::new();
        env.insert(ARCFACE_ENV_KEY, " padded.onnx".to_owned());

        let config = config_from(env);

        assert_eq!(config.arcface.status, "invalid_path");
        assert_eq!(config.arcface.sha256, None);
        assert_eq!(
            config.arcface.error.as_deref(),
            Some("path_value_must_not_be_padded")
        );
    }
}
