use image::{ImageEncoder, Rgba, RgbaImage};

use crate::{
    BODY_KEYPOINT_COUNT, FACE_KEYPOINT_COUNT, HAND_KEYPOINT_COUNT,
    POSEKIT_OPENPOSE_EXPORT_HEIGHT, POSEKIT_OPENPOSE_EXPORT_WIDTH,
};
use crate::raster::{
    draw_disc, draw_hand_keypoints, draw_openpose_segment_if_visible, visible_openpose_point,
};

pub fn render_posekit_openpose_png(openpose_json: &serde_json::Value) -> Result<Vec<u8>, String> {
    let mut image = RgbaImage::from_pixel(
        POSEKIT_OPENPOSE_EXPORT_WIDTH as u32,
        POSEKIT_OPENPOSE_EXPORT_HEIGHT as u32,
        Rgba([0, 0, 0, 255]),
    );
    let cyan = Rgba([70, 220, 255, 255]);
    let amber = Rgba([255, 190, 80, 255]);
    let green = Rgba([120, 255, 150, 255]);

    let body = openpose_points(openpose_json, "pose_keypoints_2d", BODY_KEYPOINT_COUNT)?;
    let face = openpose_points(openpose_json, "face_keypoints_2d", FACE_KEYPOINT_COUNT)?;
    let left_hand = openpose_points(openpose_json, "hand_left_keypoints_2d", HAND_KEYPOINT_COUNT)?;
    let right_hand = openpose_points(
        openpose_json,
        "hand_right_keypoints_2d",
        HAND_KEYPOINT_COUNT,
    )?;

    for (from, to) in [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (1, 5),
        (5, 6),
        (6, 7),
        (1, 8),
        (8, 9),
        (9, 10),
        (1, 11),
        (11, 12),
        (12, 13),
        (0, 14),
        (14, 16),
        (0, 15),
        (15, 17),
    ] {
        draw_openpose_segment_if_visible(&mut image, body[from], body[to], cyan);
    }
    for point in &body {
        if visible_openpose_point(*point) {
            draw_disc(&mut image, point.0, point.1, 4.0, amber);
        }
    }

    for point in &face {
        if visible_openpose_point(*point) {
            draw_disc(&mut image, point.0, point.1, 3.0, amber);
        }
    }
    draw_hand_keypoints(&mut image, &left_hand, green);
    draw_hand_keypoints(&mut image, &right_hand, green);

    encode_posekit_png(&image)
}

pub fn openpose_points(
    openpose_json: &serde_json::Value,
    field: &str,
    expected_count: usize,
) -> Result<Vec<(f32, f32, f32)>, String> {
    let person = openpose_json
        .get("people")
        .and_then(|people| people.as_array())
        .and_then(|people| people.first())
        .ok_or_else(|| {
            "Posekit OpenPose JSON must contain people[0]".to_owned()
        })?;
    let values = person
        .get(field)
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            format!("Posekit OpenPose JSON missing array {field}")
        })?;
    if values.len() != expected_count * 3 {
        return Err(format!(
            "Posekit OpenPose JSON field {field} must have {} values",
            expected_count * 3
        ));
    }
    values
        .chunks_exact(3)
        .map(|chunk| {
            let x = chunk[0].as_f64().ok_or_else(|| {
                format!(
                    "Posekit OpenPose JSON field {field} contains non-number x"
                )
            })?;
            let y = chunk[1].as_f64().ok_or_else(|| {
                format!(
                    "Posekit OpenPose JSON field {field} contains non-number y"
                )
            })?;
            let confidence = chunk[2].as_f64().ok_or_else(|| {
                format!(
                    "Posekit OpenPose JSON field {field} contains non-number confidence"
                )
            })?;
            Ok((x as f32, y as f32, confidence as f32))
        })
        .collect()
}

fn encode_posekit_png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|err| format!("Posekit PNG encode failed: {err}"))?;
    Ok(png)
}
