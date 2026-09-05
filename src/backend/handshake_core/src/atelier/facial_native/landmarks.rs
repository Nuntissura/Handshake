use super::models::{FacialIdentityModelConfig, LANDMARK_ENV_KEY};

pub const LANDMARK_FEATURE_ID: &str = "identity_gate:pipnet_landmarks";
pub const LANDMARK_METHOD_UNAVAILABLE: &str = "pipnet_landmarks_model_unavailable_v1";

pub fn landmark_feature_output(config: &FacialIdentityModelConfig) -> serde_json::Value {
    serde_json::json!({
        "feature_id": LANDMARK_FEATURE_ID,
        "source_family": "PIPNet",
        "method": LANDMARK_METHOD_UNAVAILABLE,
        "status": if config.landmark.sha256.is_some() {
            "configured_not_loaded"
        } else if config.landmark.configured {
            "configured_unavailable"
        } else {
            "not_configured"
        },
        "env_key": LANDMARK_ENV_KEY,
        "model_sha256": config.landmark.sha256,
        "error": config.landmark.error,
        "fields": [
            "eyes_open",
            "ear_left",
            "ear_right",
            "landmark_conf_min"
        ],
        "reason": "pipnet_native_runtime_not_wired_in_handshake_core",
    })
}
