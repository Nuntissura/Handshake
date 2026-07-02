//! Single-image embed renderer (WP-KERNEL-012 MT-014).
//!
//! Renders a resolved `images` embed: the decoded image at aspect-correct width (bounded by
//! the available content width), click-to-enlarge into a full-size overlay, and an
//! AccessKit-addressable container. The DECODE is the load-bearing part:
//!
//! - **Off the UI thread (MC-001 / impl note 1):** `image::load_from_memory` is CPU-heavy for
//!   a large image, so it runs on `tokio::spawn_blocking`; ONLY the decoded RGBA bytes cross
//!   the thread boundary back to the egui thread.
//! - **TextureHandle on the egui thread (impl note 2):** the `egui::TextureHandle` is created
//!   via `ctx.load_texture` ON the egui thread from the RGBA bytes; the handle is cached per
//!   asset id in [`EmbedTextureCache`] so the image is not re-uploaded every frame.
//! - **Decode failure is fail-closed (MC-005):** a corrupt byte buffer makes
//!   `image::load_from_memory` return `Err`; that transitions the asset to
//!   [`EmbedError::MediaLoadFailed`] and the view renders the typed error chip — NEVER a
//!   panic, NEVER a corrupted/blank texture rect.

use egui::{ColorImage, TextureHandle, TextureOptions};

use crate::rich_editor::embeds::asset_resolver::{EmbedError, ResolvedAsset};

/// Decode `bytes` into a [`ColorImage`] (RGBA8) using the `image` crate. Returns a typed
/// [`EmbedError::MediaLoadFailed`] (NOT a panic) when the bytes are not a decodable image
/// (MC-005). This is the CPU-heavy step the caller runs on `tokio::spawn_blocking`; it returns
/// the platform-independent RGBA buffer that crosses the thread boundary to the egui thread.
pub fn decode_rgba(bytes: &[u8]) -> Result<ColorImage, EmbedError> {
    let dynamic = image::load_from_memory(bytes)
        .map_err(|e| EmbedError::MediaLoadFailed(format!("could not decode image: {e}")))?;
    let rgba = dynamic.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return Err(EmbedError::MediaLoadFailed(
            "decoded image has zero dimensions".to_owned(),
        ));
    }
    Ok(ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw()))
}

/// A per-editor cache of uploaded [`TextureHandle`]s keyed by asset id, so a resolved image is
/// uploaded to the GPU ONCE and reused across frames (impl note 4: cache the decoded
/// TextureHandle keyed by asset_id to avoid re-decoding/re-uploading every frame). Lives in
/// `RichEditorState` (owned by the shell frame) like the resolution cache.
#[derive(Default)]
pub struct EmbedTextureCache {
    handles: std::collections::HashMap<String, TextureHandle>,
}

impl EmbedTextureCache {
    /// An empty texture cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// True when `asset_id` already has an uploaded texture (so the caller skips re-upload).
    pub fn contains(&self, asset_id: &str) -> bool {
        self.handles.contains_key(asset_id)
    }

    /// Get the cached handle for `asset_id`, if uploaded.
    pub fn get(&self, asset_id: &str) -> Option<&TextureHandle> {
        self.handles.get(asset_id)
    }

    /// Upload `image` as a texture (once) and cache it under `asset_id`, returning the handle.
    /// If already cached, returns the existing handle without re-uploading. The upload uses
    /// `ctx.load_texture`, which MUST be called on the egui thread (impl note 2).
    pub fn upload(
        &mut self,
        ctx: &egui::Context,
        asset_id: &str,
        image: ColorImage,
    ) -> TextureHandle {
        if let Some(existing) = self.handles.get(asset_id) {
            return existing.clone();
        }
        let handle = ctx.load_texture(
            format!("embed-texture-{asset_id}"),
            image,
            TextureOptions::LINEAR,
        );
        self.handles.insert(asset_id.to_owned(), handle.clone());
        handle
    }

    /// Number of cached textures (test/diagnostic helper).
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    /// True when no textures are cached.
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}

/// Compute the aspect-correct on-screen size for an image of intrinsic `(tex_w, tex_h)` bounded
/// to `max_width` (the available content width). The image is never UPSCALED past its intrinsic
/// width (so a small thumbnail does not blur to fill the column), and the height follows the
/// aspect ratio. Returns `(width, height)` in points.
pub fn aspect_fit_size(tex_w: f32, tex_h: f32, max_width: f32) -> egui::Vec2 {
    if tex_w <= 0.0 || tex_h <= 0.0 {
        return egui::vec2(0.0, 0.0);
    }
    let target_w = tex_w.min(max_width.max(1.0));
    let scale = target_w / tex_w;
    egui::vec2(target_w, tex_h * scale)
}

/// Render a single resolved image into `ui` at aspect-correct width, returning the
/// [`egui::Response`] of the clickable image (so the caller can open the full-size overlay).
/// The texture must already be uploaded (the caller uploads via [`EmbedTextureCache::upload`]
/// after the off-thread decode). The image is wrapped in a `Sense::click` so a click toggles
/// the enlarge overlay; the AccessKit author_id for the image control is set by the caller.
pub fn render_image(
    ui: &mut egui::Ui,
    texture: &TextureHandle,
    resolved: &ResolvedAsset,
    max_width: f32,
) -> egui::Response {
    let [w, h] = texture.size();
    let size = aspect_fit_size(w as f32, h as f32, max_width);
    let image_widget = egui::Image::new(texture)
        .fit_to_exact_size(size)
        .sense(egui::Sense::click());
    let alt = resolved
        .asset
        .original_filename
        .clone()
        .unwrap_or_else(|| resolved.asset.asset_id.clone());
    ui.add(image_widget).on_hover_text(alt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_failure_is_typed_error_not_panic_mc005() {
        // MC-005: intentionally corrupt bytes -> MediaLoadFailed (never a panic).
        let err = decode_rgba(b"not a real image").unwrap_err();
        assert_eq!(err.kind_str(), "media_load_failed");
        // Empty bytes -> also a typed error, no panic.
        assert_eq!(
            decode_rgba(&[]).unwrap_err().kind_str(),
            "media_load_failed"
        );
    }

    #[test]
    fn decode_valid_png_succeeds() {
        // A real 2x2 PNG encoded with the `image` crate (round-trip proves decode_rgba works).
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut img = image::RgbaImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 1, image::Rgba([0, 255, 0, 255]));
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        let decoded = decode_rgba(buf.get_ref()).unwrap();
        assert_eq!(decoded.size, [2, 2]);
    }

    #[test]
    fn aspect_fit_never_upscales_and_keeps_ratio() {
        // A 200x100 image bounded to 400px wide is NOT upscaled (stays 200x100).
        let s = aspect_fit_size(200.0, 100.0, 400.0);
        assert_eq!(s, egui::vec2(200.0, 100.0));
        // The same image bounded to 100px wide scales to 100x50 (aspect preserved).
        let s = aspect_fit_size(200.0, 100.0, 100.0);
        assert_eq!(s, egui::vec2(100.0, 50.0));
        // Degenerate dimensions -> zero size (no NaN / no panic).
        assert_eq!(aspect_fit_size(0.0, 100.0, 400.0), egui::vec2(0.0, 0.0));
    }

    #[test]
    fn texture_cache_skips_reupload() {
        let ctx = egui::Context::default();
        let mut cache = EmbedTextureCache::new();
        let _ = ctx.run(Default::default(), |ctx| {
            let img = ColorImage::from_rgba_unmultiplied([1, 1], &[1, 2, 3, 4]);
            assert!(!cache.contains("a1"));
            let _h1 = cache.upload(ctx, "a1", img.clone());
            assert!(cache.contains("a1"));
            assert_eq!(cache.len(), 1);
            // A second upload for the SAME id returns the cached handle (no second entry).
            let _h2 = cache.upload(ctx, "a1", img);
            assert_eq!(
                cache.len(),
                1,
                "re-upload of the same asset id does not add a new texture"
            );
        });
    }
}
