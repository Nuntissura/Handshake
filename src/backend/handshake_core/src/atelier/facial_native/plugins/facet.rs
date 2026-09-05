use crate::atelier::facial_native::common::FacialNativeImageContext;
use serde_json::{json, Value};

pub const QUALITY_FEATURE_ID: &str = "facet:quality_pass";
pub const QUALITY_SOURCE_FAMILY: &str = "facet";
pub const QUALITY_SOURCE: &str = "facet_native_metadata_only_v1";
pub const QUALITY_METHOD: &str = "native_metadata_only_quality_score";

pub fn quality_score(ctx: &FacialNativeImageContext) -> u8 {
    let mut score: i32 = 18;
    if ctx.byte_len > 0 {
        score += 12;
    }
    if ctx.byte_len >= 128_000 {
        score += 8;
    }
    if ctx.byte_len >= 1_000_000 {
        score += 6;
    }
    if ctx.has_content_hash() {
        score += 8;
    }
    match (ctx.image_width, ctx.image_height) {
        (Some(width), Some(height)) => {
            score += 18;
            let megapixels = (width as f64) * (height as f64) / 1_000_000.0;
            if megapixels >= 0.5 {
                score += 8;
            }
            if megapixels >= 1.5 {
                score += 8;
            }
            let short_side = width.min(height);
            if short_side >= 512 {
                score += 8;
            }
            let ratio = width.max(height) as f64 / width.min(height).max(1) as f64;
            if ratio <= 2.2 {
                score += 5;
            }
        }
        _ => {
            if likely_image_extension(&ctx.file_name) || likely_image_extension(&ctx.source_ref) {
                score += 8;
            }
        }
    }
    score.clamp(0, 100) as u8
}

pub fn quality_band(score: u8) -> &'static str {
    match score {
        82..=100 => "excellent",
        68..=81 => "good",
        55..=67 => "usable",
        40..=54 => "weak",
        _ => "reject",
    }
}

pub fn quality_metrics(ctx: &FacialNativeImageContext, score: u8) -> Value {
    let megapixels = ctx.megapixels.unwrap_or(0.0);
    let short_side = ctx.image_width.zip(ctx.image_height).map(|(w, h)| w.min(h));
    let long_side = ctx.image_width.zip(ctx.image_height).map(|(w, h)| w.max(h));
    let aspect_balance = ctx
        .image_width
        .zip(ctx.image_height)
        .map(|(w, h)| {
            let ratio = w.max(h) as f64 / w.min(h).max(1) as f64;
            (100.0 - ((ratio - 1.0) * 28.0)).clamp(0.0, 100.0)
        })
        .unwrap_or(0.0);
    let technical_sharpness = short_side
        .map(|side| ((side as f64 / 768.0) * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    let dynamic_range = long_side
        .map(|side| ((side as f64 / 1600.0) * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    let exposure = if ctx.is_decoded() { 72.0 } else { 0.0 };
    let color_balance = if ctx.is_decoded() {
        aspect_balance
    } else {
        0.0
    };
    let noise_estimate = if ctx.byte_len <= 0 {
        100.0
    } else {
        (100.0 - ((ctx.byte_len as f64).ln().max(1.0) * 6.0)).clamp(0.0, 100.0)
    };

    json!({
        "feature_id": QUALITY_FEATURE_ID,
        "source_family": QUALITY_SOURCE_FAMILY,
        "source": QUALITY_SOURCE,
        "method": QUALITY_METHOD,
        "status": "metadata_only_degraded",
        "path": ctx.source_ref,
        "file_size": ctx.byte_len,
        "width": ctx.image_width,
        "height": ctx.image_height,
        "megapixels": megapixels,
        "quality": score,
        "technical_sharpness": technical_sharpness,
        "eyes_sharpness": null,
        "eyes_sharpness_status": "unavailable_without_landmarks",
        "exposure": exposure,
        "color_balance": color_balance,
        "dynamic_range": dynamic_range,
        "noise_estimate": noise_estimate,
        "quality_band": quality_band(score),
        "headshot_candidate": score >= 68 && ctx.is_decoded(),
        "decode_status": ctx.decode_status,
        "limitations": [
            "metadata_only_quality_not_pixel_analysis_parity",
            "no_real_face_detector",
            "no_landmark_eye_sharpness",
        ],
    })
}

fn likely_image_extension(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".bmp"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}
