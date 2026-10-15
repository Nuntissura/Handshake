use image::{Rgba, RgbaImage};

use crate::{POSEKIT_OPENPOSE_EXPORT_HEIGHT, POSEKIT_OPENPOSE_EXPORT_WIDTH};

pub(crate) fn draw_hand_keypoints(image: &mut RgbaImage, points: &[(f32, f32, f32)], color: Rgba<u8>) {
    for finger in 0..5 {
        let base = 1 + finger * 4;
        draw_openpose_segment_if_visible(image, points[0], points[base], color);
        for offset in 0..3 {
            draw_openpose_segment_if_visible(
                image,
                points[base + offset],
                points[base + offset + 1],
                color,
            );
        }
    }
    for point in points {
        if visible_openpose_point(*point) {
            draw_disc(image, point.0, point.1, 2.8, color);
        }
    }
}

pub(crate) fn draw_openpose_segment_if_visible(
    image: &mut RgbaImage,
    from: (f32, f32, f32),
    to: (f32, f32, f32),
    color: Rgba<u8>,
) {
    if visible_openpose_point(from) && visible_openpose_point(to) {
        draw_line(image, from.0, from.1, to.0, to.1, color);
    }
}

pub(crate) fn visible_openpose_point(point: (f32, f32, f32)) -> bool {
    point.2 > 0.0 && point.0 > 0.0 && point.1 > 0.0
}

fn draw_line(image: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgba<u8>) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        draw_disc(image, x0 + dx * t, y0 + dy * t, 2.0, color);
    }
}

pub(crate) fn draw_disc(image: &mut RgbaImage, cx: f32, cy: f32, radius: f32, color: Rgba<u8>) {
    let min_x = (cx - radius).floor() as i32;
    let max_x = (cx + radius).ceil() as i32;
    let min_y = (cy - radius).floor() as i32;
    let max_y = (cy + radius).ceil() as i32;
    let r2 = radius * radius;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x < 0
                || y < 0
                || x >= POSEKIT_OPENPOSE_EXPORT_WIDTH
                || y >= POSEKIT_OPENPOSE_EXPORT_HEIGHT
            {
                continue;
            }
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if dx * dx + dy * dy <= r2 {
                image.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}
