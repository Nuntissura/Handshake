use serde_json::{json, Value};

pub const SOURCE_FAMILY: &str = "ediffiqa";

pub fn unavailable_records() -> Vec<Value> {
    [
        ("ediffiqa:model_t", "HANDSHAKE_FACIAL_EDIFFIQA_MODEL_T"),
        ("ediffiqa:model_m", "HANDSHAKE_FACIAL_EDIFFIQA_MODEL_M"),
        ("ediffiqa:model_s", "HANDSHAKE_FACIAL_EDIFFIQA_MODEL_S"),
        ("ediffiqa:model_l", "HANDSHAKE_FACIAL_EDIFFIQA_MODEL_L"),
        (
            "ediffiqa:batch_inference",
            "HANDSHAKE_FACIAL_EDIFFIQA_BATCH_MODELS",
        ),
    ]
    .into_iter()
    .map(|(feature_id, required_config_key)| {
        json!({
            "feature_id": feature_id,
            "source_family": SOURCE_FAMILY,
            "status": "unavailable",
            "native_route": "atelier.facial.quality.ediffiqa_unavailable",
            "required_config_keys": [required_config_key],
            "reason": "ediffiqa_model_not_configured",
            "truthfulness": "no eDifFIQA model-backed quality score is emitted by MT-026",
        })
    })
    .collect()
}
