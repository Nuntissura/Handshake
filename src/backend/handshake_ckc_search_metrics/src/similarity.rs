use std::collections::HashMap;

/// dHash hex must be exactly 16 lowercase hex chars (64 bits). Mirrors legacy source
/// `dhash.js::isHex64`.
pub fn is_hex64(s: &str) -> bool {
    let t = s.trim();
    t.len() == 16 && t.chars().all(|c| c.is_ascii_hexdigit())
}

/// Hamming distance between two 16-char hex (64-bit) hashes. Mirrors legacy source
/// `dhash.js::hammingDistanceHex64`: invalid inputs return the max distance 64.
pub fn hamming_distance_hex64(a: &str, b: &str) -> i32 {
    let sa = a.trim().to_ascii_lowercase();
    let sb = b.trim().to_ascii_lowercase();
    if !is_hex64(&sa) || !is_hex64(&sb) {
        return 64;
    }
    let na = u64::from_str_radix(&sa, 16).unwrap_or(u64::MAX);
    let nb = u64::from_str_radix(&sb, 16).unwrap_or(0);
    (na ^ nb).count_ones() as i32
}

pub fn compute_similarity_from_image_bytes(
    image_bytes: &[u8],
) -> Result<(String, serde_json::Value), String> {
    let image = image::load_from_memory(image_bytes).map_err(|err| {
        format!("similarity image decode failed: {err}")
    })?;
    let dhash_hex = compute_dhash_hex(&image);
    let palette_json = compute_palette_json(&image);
    Ok((dhash_hex, palette_json))
}

fn compute_dhash_hex(image: &image::DynamicImage) -> String {
    let gray = image.to_luma8();
    let resized = image::imageops::resize(&gray, 9, 8, image::imageops::FilterType::Triangle);
    let mut hash = 0u64;
    let mut bit = 0u32;
    for y in 0..8 {
        for x in 0..8 {
            let left = resized.get_pixel(x, y)[0];
            let right = resized.get_pixel(x + 1, y)[0];
            if left > right {
                hash |= 1u64 << (63 - bit);
            }
            bit += 1;
        }
    }
    format!("{hash:016x}")
}

fn compute_palette_json(image: &image::DynamicImage) -> serde_json::Value {
    let rgb = image.to_rgb8();
    let sample = image::imageops::thumbnail(&rgb, 64, 64);
    let mut counts: HashMap<[u8; 3], i64> = HashMap::new();
    for pixel in sample.pixels() {
        let [r, g, b] = pixel.0;
        *counts.entry([r, g, b]).or_insert(0) += 1;
    }
    let sampled_pixels: i64 = counts.values().sum();
    let mut entries: Vec<(String, i64)> = counts
        .into_iter()
        .map(|([r, g, b], count)| (format!("#{r:02x}{g:02x}{b:02x}"), count))
        .collect();
    entries.sort_by(|(hex_a, count_a), (hex_b, count_b)| {
        count_b.cmp(count_a).then_with(|| hex_a.cmp(hex_b))
    });
    let dominant: Vec<serde_json::Value> = entries
        .into_iter()
        .take(8)
        .map(|(hex, count)| {
            serde_json::json!({
                "hex": hex,
                "count": count,
                "ratio": if sampled_pixels == 0 {
                    0.0
                } else {
                    count as f64 / sampled_pixels as f64
                },
            })
        })
        .collect();
    serde_json::json!({
        "algorithm": "rgb_exact_thumbnail_v1",
        "sampled_pixels": sampled_pixels,
        "dominant": dominant,
    })
}

pub fn cosine_similarity_stored(stored: &[f64], query: &[f32]) -> f64 {
    if stored.len() != query.len() || stored.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f64;
    let mut left = 0.0_f64;
    let mut right = 0.0_f64;
    for (a, b) in stored.iter().zip(query.iter()) {
        let b = f64::from(*b);
        dot += a * b;
        left += a * a;
        right += b * b;
    }
    if left <= 0.0 || right <= 0.0 {
        return 0.0;
    }
    dot / (left.sqrt() * right.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex64_validation() {
        assert!(is_hex64("0123456789abcdef"));
        assert!(is_hex64("FFFFFFFFFFFFFFFF"));
        assert!(!is_hex64("0123456789abcde")); // 15 chars
        assert!(!is_hex64("0123456789abcdeg")); // non-hex
        assert!(!is_hex64(""));
    }

    #[test]
    fn hamming_matches_legacy_source_semantics() {
        // Identical hashes => 0.
        assert_eq!(
            hamming_distance_hex64("0000000000000000", "0000000000000000"),
            0
        );
        // Single bit set => 1.
        assert_eq!(
            hamming_distance_hex64("0000000000000000", "0000000000000001"),
            1
        );
        // All bits differ => 64.
        assert_eq!(
            hamming_distance_hex64("0000000000000000", "ffffffffffffffff"),
            64
        );
        // Invalid input => max distance 64.
        assert_eq!(hamming_distance_hex64("zzzz", "0000000000000000"), 64);
    }
}
