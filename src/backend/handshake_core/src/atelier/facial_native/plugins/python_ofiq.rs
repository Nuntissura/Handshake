use crate::atelier::facial_native::common::FacialNativeImageContext;
use serde_json::{json, Value};

pub const SCALAR_FEATURE_ID: &str = "python-ofiq:scalar_quality";
pub const VECTOR_FEATURE_ID: &str = "python-ofiq:vector_quality";
pub const SETUP_FEATURE_ID: &str = "python-ofiq:setup_data";
pub const SOURCE_FAMILY: &str = "python-ofiq";
pub const SOURCE: &str = "python_ofiq_native_metadata_only_v1";
pub const METHOD: &str = "native_metadata_only_dimension_vector";

const DIMENSIONS: [&str; 7] = [
    "decoded",
    "file_size_signal",
    "megapixels",
    "short_side",
    "aspect_balance",
    "content_hash_present",
    "source_ref_stability",
];

const MISSING_SOURCE_DIMENSIONS: [&str; 17] = [
    "sharpness",
    "exposure",
    "contrast",
    "colorfulness",
    "entropy",
    "noise_estimate",
    "dynamic_range",
    "skin_ratio",
    "center_bias",
    "face_confidence",
    "face_count",
    "face_clarity_proxy",
    "sharpness_focus",
    "noise_guard",
    "composition",
    "luma_std",
    "median_luma",
];

pub fn scalar_quality(facet_quality: u8) -> u8 {
    facet_quality
}

pub fn vector_quality(ctx: &FacialNativeImageContext, facet_quality: u8) -> Value {
    let dimensions = dimension_values(ctx);
    let dimension_sum = dimensions
        .iter()
        .filter_map(|entry| entry.get("value").and_then(Value::as_f64))
        .sum::<f64>();
    let dimension_count = dimensions.len();
    let dimension_mean = if dimension_count == 0 {
        0.0
    } else {
        dimension_sum / dimension_count as f64
    };

    json!({
        "source_family": SOURCE_FAMILY,
        "source": SOURCE,
        "method": METHOD,
        "status": "metadata_only_degraded",
        "setup_feature_id": SETUP_FEATURE_ID,
        "scalar_feature_id": SCALAR_FEATURE_ID,
        "vector_feature_id": VECTOR_FEATURE_ID,
        "scalar_quality": scalar_quality(facet_quality),
        "quality_band": quality_band(facet_quality),
        "dimension_sum": dimension_sum,
        "dimension_count": dimension_count,
        "quality_gap_vs_dimension_mean": ((facet_quality as f64) - dimension_mean).abs(),
        "schema": {
            "version": "0.2-handshake-native",
            "dimension_count": DIMENSIONS.len(),
            "dimensions": DIMENSIONS,
        },
        "missing_source_dimensions": MISSING_SOURCE_DIMENSIONS,
        "limitations": [
            "metadata_only_quality_not_python_ofiq_model_parity",
            "source_app_pixel_dimensions_not_available_without_full_image_analysis",
        ],
        "dimensions": dimensions,
        "thresholds": {
            "scalar_quality_headshot_min": 68.0,
            "vector_quality_gap_tolerance": 25.0,
            "quality_score_range": [0.0, 100.0],
        },
    })
}

fn dimension_values(ctx: &FacialNativeImageContext) -> Vec<Value> {
    let file_size_signal = if ctx.byte_len <= 0 {
        0.0
    } else {
        ((ctx.byte_len as f64).ln() / 16.0 * 100.0).clamp(0.0, 100.0)
    };
    let megapixels = ctx.megapixels.unwrap_or(0.0);
    let megapixel_signal = (megapixels / 2.0 * 100.0).clamp(0.0, 100.0);
    let short_side_signal = ctx
        .image_width
        .zip(ctx.image_height)
        .map(|(w, h)| (w.min(h) as f64 / 768.0 * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    let aspect_balance = ctx
        .image_width
        .zip(ctx.image_height)
        .map(|(w, h)| {
            let ratio = w.max(h) as f64 / w.min(h).max(1) as f64;
            (100.0 - ((ratio - 1.0) * 28.0)).clamp(0.0, 100.0)
        })
        .unwrap_or(0.0);
    let values = [
        ("decoded", if ctx.is_decoded() { 100.0 } else { 0.0 }),
        ("file_size_signal", file_size_signal),
        ("megapixels", megapixel_signal),
        ("short_side", short_side_signal),
        ("aspect_balance", aspect_balance),
        (
            "content_hash_present",
            if ctx.has_content_hash() { 100.0 } else { 0.0 },
        ),
        (
            "source_ref_stability",
            if ctx.source_ref.contains("://") {
                80.0
            } else {
                100.0
            },
        ),
    ];

    values
        .into_iter()
        .enumerate()
        .map(|(index, (name, value))| {
            json!({
                "index": index,
                "name": name,
                "value": value,
                "unit": "score_0_100",
            })
        })
        .collect()
}

fn quality_band(score: u8) -> &'static str {
    match score {
        82..=100 => "excellent",
        68..=81 => "good",
        55..=67 => "usable",
        40..=54 => "weak",
        _ => "reject",
    }
}
