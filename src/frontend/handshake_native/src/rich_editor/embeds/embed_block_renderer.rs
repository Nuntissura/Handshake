//! Embed dispatch + interactive rendering (WP-KERNEL-012 MT-014).
//!
//! Given a media-embed `hsLink` node (an [`crate::rich_editor::document_model::node::HsLinkNode`]
//! whose `ref_kind` is one of the [`MEDIA_EMBED_REF_KINDS`]), this module:
//!   1. spawns the async asset resolution (once, cached — AC-9) onto the editor's tokio runtime,
//!   2. dispatches the resolved state to the correct view (image / slideshow / album / video),
//!   3. renders the Resolving spinner / Ok media / typed Err chip (fail-closed, never blank),
//!   4. emits the AccessKit nodes the ACs name (container + prev/next/cell/play controls).
//!
//! ## Why this is the dispatch hub (not block_renderer.rs's painter path)
//!
//! `renderer::block_renderer` paints text blocks via an `egui::Painter` (no interactivity).
//! Embeds are INTERACTIVE (prev/next buttons, click-to-enlarge modal, play button), so they
//! need an `egui::Ui`. The renderer (`rich_editor_widget::render_blocks`) therefore routes a
//! paragraph that contains a media-embed `hsLink` to [`render_embed`] HERE, which owns the
//! `egui::Ui`-based interactive render. `block_renderer` exposes [`super::super::renderer::block_renderer::block_media_embed`]
//! to detect such a paragraph; the two seams keep the painter path and the interactive path
//! cleanly separated.
//!
//! ## State ownership (impl note 5)
//!
//! The resolution cache, texture cache, and per-node view states live in
//! [`EmbedRuntime`], owned by `RichEditorState` (the shell frame), NOT inside these render
//! functions — so they persist across frames. The runtime carries the tokio `Handle` + the
//! fetcher + the workspace id; a render call borrows it `&mut`.

use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::rich_editor::document_model::node::HsLinkNode;
use crate::rich_editor::embeds::album_view::{self, AlbumViewState};
use crate::rich_editor::embeds::asset_resolver::{
    AssetMetadataFetcher, EmbedError, EmbedResolutionCache, EmbedResolutionState, MediaEmbedKind,
    ResolvedAsset, SequenceItem,
};
use crate::rich_editor::embeds::image_view::{self, EmbedTextureCache};
use crate::rich_editor::embeds::slideshow_view::{self, SlideshowViewState};
use crate::rich_editor::embeds::video_view::{
    self, InlineRevealPlayHandler, VideoPlayHandler, VideoViewState,
};
use crate::theme::HsPalette;

/// One-slot delivery cell for an off-thread single-asset resolution (the MT-009 delivery-cell
/// pattern reused across the shell): the spawned task writes the terminal state here; the egui
/// UI thread drains it next frame into the [`EmbedResolutionCache`].
type SingleDeliveryCell = Arc<Mutex<Option<(String, EmbedResolutionState)>>>;

/// One-slot delivery cell for an off-thread album/slideshow sequence resolution: keyed by the
/// embed's ref_value, carrying the per-member items.
type SequenceDeliveryCell = Arc<Mutex<Option<(String, Result<Vec<SequenceItem>, EmbedError>)>>>;

/// Multi-slot delivery cell for off-thread image CONTENT decode results (MC-001): the spawned
/// task fetches `GET .../content`, decodes the bytes on `tokio::spawn_blocking`, and writes the
/// decoded [`egui::ColorImage`] (or a typed decode/fetch error) here keyed by asset id. The egui
/// UI thread drains it next frame and uploads the `ColorImage` as a `TextureHandle` (the upload
/// MUST happen on the egui thread — only the platform-independent RGBA `ColorImage` crosses the
/// thread boundary, impl note 2). A `Vec` (not a single slot) so several images in one document
/// can deliver in the same frame without clobbering each other.
type ContentDeliveryCell = Arc<Mutex<Vec<(String, Result<egui::ColorImage, EmbedError>)>>>;

/// A resolved (or failed) album/slideshow sequence, cached per ref_value so the sequence is
/// resolved ONCE (AC-9 at the sequence level).
#[derive(Clone)]
pub enum SequenceState {
    /// The sequence resolution is in flight.
    Resolving,
    /// The sequence resolved; the members are individually Ok/Err (per-item fail-closed).
    Items(Arc<Vec<SequenceItem>>),
    /// The whole sequence failed (empty/oversized/no-workspace) with a typed error.
    Err(EmbedError),
}

/// The per-editor embed runtime: caches + view states + the async transport. Owned by
/// `RichEditorState`. Stores everything that must survive across frames so a re-render reuses
/// resolved assets/textures (AC-9) and remembers slideshow/album/video paging.
pub struct EmbedRuntime {
    /// The workspace whose assets embeds resolve against (from the document context).
    pub workspace_id: String,
    /// REST base the content/thumbnail URLs resolve against (matches the fetcher's base).
    pub base_url: String,
    /// The async metadata fetcher (production: reqwest; tests: a counted mock).
    pub fetcher: Arc<dyn AssetMetadataFetcher>,
    /// The tokio runtime handle resolutions spawn onto (None in a headless unit test that does
    /// not exercise the spawn path — the standalone view/validation tests do not need it).
    pub runtime: Option<tokio::runtime::Handle>,
    /// Per-asset single-resolution cache (AC-9).
    pub resolutions: EmbedResolutionCache,
    /// Per-ref_value sequence-resolution cache (AC-9 at the sequence level).
    pub sequences: std::collections::HashMap<String, SequenceState>,
    /// Per-asset uploaded GPU texture cache (avoid re-upload every frame).
    pub textures: EmbedTextureCache,
    /// Per-asset decoded `ColorImage` awaiting upload on the egui thread. Populated by
    /// [`Self::drain_deliveries`] from the off-thread content-fetch+decode pipeline; consumed
    /// (uploaded + removed) by [`render_resolved_image`] on the egui thread. A decode error is
    /// folded into the resolution cache as `Err` so the typed error chip shows (MC-005).
    pub decoded_images: std::collections::HashMap<String, egui::ColorImage>,
    /// Asset ids whose CONTENT fetch+decode has been kicked off (so the bytes pipeline runs
    /// ONCE per asset, mirroring the metadata AC-9 caching for the content fetch).
    content_in_flight: std::collections::HashSet<String>,
    /// Per-ref_value slideshow paging state.
    pub slideshow_states: std::collections::HashMap<String, SlideshowViewState>,
    /// Per-ref_value album modal state.
    pub album_states: std::collections::HashMap<String, AlbumViewState>,
    /// Per-asset video reveal state.
    pub video_states: std::collections::HashMap<String, VideoViewState>,
    /// Delivery cell for off-thread single resolutions (drained at frame top).
    single_cell: SingleDeliveryCell,
    /// Delivery cell for off-thread sequence resolutions (drained at frame top).
    sequence_cell: SequenceDeliveryCell,
    /// Delivery cell for off-thread image-content decodes (drained at frame top).
    content_cell: ContentDeliveryCell,
}

impl EmbedRuntime {
    /// Build a runtime over `fetcher` for `workspace_id`/`base_url`, bridging async resolution
    /// onto `runtime` (pass `None` only for headless tests that do not spawn).
    pub fn new(
        workspace_id: impl Into<String>,
        base_url: impl Into<String>,
        fetcher: Arc<dyn AssetMetadataFetcher>,
        runtime: Option<tokio::runtime::Handle>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            base_url: base_url.into(),
            fetcher,
            runtime,
            resolutions: EmbedResolutionCache::new(),
            sequences: std::collections::HashMap::new(),
            textures: EmbedTextureCache::new(),
            decoded_images: std::collections::HashMap::new(),
            content_in_flight: std::collections::HashSet::new(),
            slideshow_states: std::collections::HashMap::new(),
            album_states: std::collections::HashMap::new(),
            video_states: std::collections::HashMap::new(),
            single_cell: Arc::new(Mutex::new(None)),
            sequence_cell: Arc::new(Mutex::new(None)),
            content_cell: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Drain any off-thread resolution results delivered since the last frame into the caches.
    /// Called at the top of a render frame so a completed fetch updates the cache before the
    /// embed re-renders. Returns true when a result was applied (the caller can request a
    /// repaint so the new state shows immediately).
    pub fn drain_deliveries(&mut self) -> bool {
        let mut applied = false;
        if let Ok(mut slot) = self.single_cell.lock() {
            if let Some((asset_id, state)) = slot.take() {
                self.resolutions.insert(asset_id, state);
                applied = true;
            }
        }
        if let Ok(mut slot) = self.sequence_cell.lock() {
            if let Some((ref_value, result)) = slot.take() {
                let state = match result {
                    Ok(items) => SequenceState::Items(Arc::new(items)),
                    Err(e) => SequenceState::Err(e),
                };
                self.sequences.insert(ref_value, state);
                applied = true;
            }
        }
        // Drain off-thread image-content decode results. A decoded ColorImage is parked in
        // `decoded_images` for the egui thread to upload (impl note 2: the upload must be on the
        // egui thread). A decode/fetch FAILURE is folded into the resolution cache as Err so the
        // typed error chip shows the failure (MC-005) instead of an eternal placeholder.
        if let Ok(mut deliveries) = self.content_cell.lock() {
            for (asset_id, result) in deliveries.drain(..) {
                match result {
                    Ok(image) => {
                        self.decoded_images.insert(asset_id, image);
                    }
                    Err(e) => {
                        self.resolutions
                            .insert(asset_id, EmbedResolutionState::Err(e));
                    }
                }
                applied = true;
            }
        }
        applied
    }

    /// Ensure a single asset is being (or has been) resolved: if it has no terminal state and is
    /// not already in flight, mark it `Resolving` and spawn the fetch (AC-9: a terminal asset is
    /// never re-fetched). A no-op when there is no runtime (headless test path).
    fn ensure_single(&mut self, kind: MediaEmbedKind, asset_id: &str) {
        if !self.resolutions.needs_fetch(asset_id) {
            return; // already resolving / resolved / failed — do not re-spawn (AC-9).
        }
        self.resolutions
            .insert(asset_id.to_owned(), EmbedResolutionState::Resolving);
        let Some(runtime) = self.runtime.clone() else {
            return; // headless: the caller seeds the cache directly in tests.
        };
        let fetcher = Arc::clone(&self.fetcher);
        let cell = Arc::clone(&self.single_cell);
        let workspace_id = self.workspace_id.clone();
        let base_url = self.base_url.clone();
        let asset_id = asset_id.to_owned();
        runtime.spawn(async move {
            let result = crate::rich_editor::embeds::asset_resolver::resolve_one(
                kind,
                &workspace_id,
                &asset_id,
                &base_url,
                fetcher.as_ref(),
            )
            .await;
            let state = match result {
                Ok(r) => EmbedResolutionState::Ok(r),
                Err(e) => EmbedResolutionState::Err(e),
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((asset_id, state));
            }
        });
    }

    /// Ensure the CONTENT bytes for a resolved image asset are being (or have been) fetched +
    /// decoded off-thread (MC-001): once metadata resolved Ok, this fetches `GET .../content` and
    /// runs [`image_view::decode_rgba`] on `tokio::spawn_blocking`, delivering the decoded
    /// [`egui::ColorImage`] back through the content delivery cell for the egui thread to upload.
    /// Runs ONCE per asset (`content_in_flight` guard), mirroring the metadata AC-9 caching. A
    /// no-op when there is no runtime (headless path — a test injects the decoded image directly).
    /// This is the production path that makes [`render_resolved_image`] reach its texture branch.
    fn ensure_image_content(&mut self, asset_id: &str) {
        // Already uploaded, already decoded-and-waiting, or already fetching -> do not re-fetch.
        if self.textures.contains(asset_id)
            || self.decoded_images.contains_key(asset_id)
            || self.content_in_flight.contains(asset_id)
        {
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            return; // headless: the caller delivers the decoded image directly in tests.
        };
        self.content_in_flight.insert(asset_id.to_owned());
        let fetcher = Arc::clone(&self.fetcher);
        let cell = Arc::clone(&self.content_cell);
        let workspace_id = self.workspace_id.clone();
        let asset_id = asset_id.to_owned();
        runtime.spawn(async move {
            // 1) Fetch the raw content bytes (GET .../content).
            let bytes = fetcher.fetch_content(&workspace_id, &asset_id).await;
            // 2) Decode off the async/UI thread (MC-001: decode is CPU-heavy — spawn_blocking).
            let decoded = match bytes {
                Ok(bytes) => tokio::task::spawn_blocking(move || image_view::decode_rgba(&bytes))
                    .await
                    .unwrap_or_else(|join_err| {
                        Err(EmbedError::MediaLoadFailed(format!(
                            "image decode task failed: {join_err}"
                        )))
                    }),
                Err(e) => Err(e),
            };
            // 3) Deliver the decoded ColorImage (or typed error) for the egui thread to upload.
            if let Ok(mut deliveries) = cell.lock() {
                deliveries.push((asset_id, decoded));
            }
        });
    }

    /// Ensure an album/slideshow sequence is being (or has been) resolved (AC-9 at sequence
    /// level). A no-op when there is no runtime (headless test path).
    fn ensure_sequence(&mut self, kind: MediaEmbedKind, ref_value: &str) {
        if self.sequences.contains_key(ref_value) {
            return; // already resolving / resolved.
        }
        self.sequences
            .insert(ref_value.to_owned(), SequenceState::Resolving);
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
        let fetcher = Arc::clone(&self.fetcher);
        let cell = Arc::clone(&self.sequence_cell);
        let workspace_id = self.workspace_id.clone();
        let base_url = self.base_url.clone();
        let ref_value = ref_value.to_owned();
        runtime.spawn(async move {
            let result = crate::rich_editor::embeds::asset_resolver::resolve_sequence(
                kind,
                &workspace_id,
                &ref_value,
                &base_url,
                fetcher,
            )
            .await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((ref_value, result));
            }
        });
    }
}

/// Render a media-embed `hsLink` node interactively into `ui`, dispatching on `ref_kind` to the
/// correct view. This is the [`block_renderer`] match-arm target for an embed (impl note 5 /
/// contract step 3 + 5). Fail-closed: an unknown kind, an empty ref, or any resolution error
/// renders a VISIBLE typed chip — never blank, never a panic.
///
/// [`super::super::renderer::block_renderer`] never paints these; the renderer calls this from
/// its `egui::Ui` context for an embed-bearing block.
pub fn render_embed(
    ui: &mut egui::Ui,
    link: &HsLinkNode,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
) {
    let Some(kind) = MediaEmbedKind::from_ref_kind(&link.ref_kind) else {
        // Not a media kind — this should not be routed here (the renderer only routes media
        // embeds), but fail-closed with a visible chip rather than silently drawing nothing.
        render_error_chip(
            ui,
            &link.ref_value,
            &EmbedError::InvalidRef(format!("'{}' is not a media embed kind", link.ref_kind)),
            palette,
        );
        return;
    };

    // An empty ref is fail-closed at the dispatch boundary (AC-2): the empty_ref chip shows
    // BEFORE any resolution attempt.
    if link.ref_value.trim().is_empty() {
        render_error_chip(ui, &link.ref_value, &EmbedError::EmptyRef, palette);
        return;
    }

    let max_width = ui.available_width().max(1.0);
    match kind {
        MediaEmbedKind::Images => {
            render_single_image(ui, kind, &link.ref_value, runtime, palette, max_width)
        }
        MediaEmbedKind::Video => render_video(ui, &link.ref_value, runtime, palette),
        MediaEmbedKind::Slideshow => {
            render_slideshow(ui, &link.ref_value, runtime, palette, max_width)
        }
        MediaEmbedKind::Album => render_album(ui, &link.ref_value, runtime, palette, max_width),
    }
}

/// Render a single `images` embed: resolve (once), decode off-thread + upload, then draw at
/// aspect-correct width with click-to-enlarge.
fn render_single_image(
    ui: &mut egui::Ui,
    kind: MediaEmbedKind,
    ref_value: &str,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
    max_width: f32,
) {
    // Validate first so a bad ref is a visible chip with NO fetch (AC-3/AC-4 at render time).
    let asset_id = match crate::rich_editor::embeds::asset_resolver::validate_asset_ref(ref_value) {
        Ok(id) => id,
        Err(e) => {
            render_error_chip(ui, ref_value, &e, palette);
            return;
        }
    };
    runtime.ensure_single(kind, &asset_id);

    match runtime.resolutions.get(&asset_id).cloned() {
        None | Some(EmbedResolutionState::Resolving) => render_spinner(ui, kind, ref_value),
        Some(EmbedResolutionState::Err(e)) => render_error_chip(ui, ref_value, &e, palette),
        Some(EmbedResolutionState::Ok(resolved)) => {
            render_resolved_image(ui, &asset_id, &resolved, runtime, palette, max_width);
        }
    }
}

/// Draw a resolved image. This is where the off-thread content-fetch+decode pipeline lands on
/// the egui thread:
///   1. If a texture is already uploaded for this asset, render it (the steady state).
///   2. Else, if the off-thread pipeline DELIVERED a decoded `ColorImage` (drained into
///      `decoded_images` at frame top), upload it via [`EmbedTextureCache::upload`] HERE on the
///      egui thread (impl note 2 — `ctx.load_texture` is egui-thread-only) and render it this
///      frame. A repaint is requested so the just-uploaded texture shows without an idle stall.
///   3. Else, kick off (once) the content fetch + off-thread decode via
///      [`EmbedRuntime::ensure_image_content`] and show the "decoding pixels" spinner while it is
///      in flight (never blank). A decode/fetch failure becomes `Err` in the resolution cache
///      (drained in step 2's sibling path), so the NEXT frame shows the typed error chip (MC-005).
fn render_resolved_image(
    ui: &mut egui::Ui,
    asset_id: &str,
    resolved: &ResolvedAsset,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
    max_width: f32,
) {
    let author = format!("embed-image-{asset_id}");

    // (2) Upload a freshly-decoded image (delivered off-thread) on the egui thread, before the
    // texture-branch check, so the first frame after delivery already renders the real texture.
    if !runtime.textures.contains(asset_id) {
        if let Some(image) = runtime.decoded_images.remove(asset_id) {
            let _texture = runtime.textures.upload(ui.ctx(), asset_id, image);
            ui.ctx().request_repaint();
        }
    }

    // (1) Texture ready -> render the decoded image at aspect-correct width (AC-1).
    if let Some(texture) = runtime.textures.get(asset_id).cloned() {
        let resp = ui
            .scope(|ui| image_view::render_image(ui, &texture, resolved, max_width))
            .inner;
        emit_node_author(ui.ctx(), resp.id, accesskit::Role::Image, &author);
        return;
    }

    // (3) No texture yet: drive the content fetch + off-thread decode (once) and show the
    // decoding spinner while the pixels load (fail-closed, never blank). The content URL is shown
    // beneath the spinner so the operator can inspect exactly what is loading.
    runtime.ensure_image_content(asset_id);
    let label = resolved
        .asset
        .original_filename
        .clone()
        .unwrap_or_else(|| asset_id.to_owned());
    let frame = egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(8.0)
        .corner_radius(6.0);
    let resp = frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new());
                ui.colored_label(palette.text, format!("Decoding {label}…"));
            });
            ui.colored_label(palette.text_subtle, &resolved.content_url);
        })
        .response;
    emit_node_author(ui.ctx(), resp.id, accesskit::Role::Image, &author);
}

/// Render a `video` embed: poster/placeholder + play button + filename + content URL (never an
/// external launch). Fail-closed and HBR-QUIET by construction.
fn render_video(
    ui: &mut egui::Ui,
    ref_value: &str,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
) {
    let asset_id = match crate::rich_editor::embeds::asset_resolver::validate_asset_ref(ref_value) {
        Ok(id) => id,
        Err(e) => {
            render_error_chip(ui, ref_value, &e, palette);
            return;
        }
    };
    runtime.ensure_single(MediaEmbedKind::Video, &asset_id);

    let resolved = match runtime.resolutions.get(&asset_id).cloned() {
        None | Some(EmbedResolutionState::Resolving) => {
            render_spinner(ui, MediaEmbedKind::Video, ref_value);
            return;
        }
        Some(EmbedResolutionState::Err(e)) => {
            render_error_chip(ui, ref_value, &e, palette);
            return;
        }
        Some(EmbedResolutionState::Ok(r)) => r,
    };

    let state = runtime.video_states.entry(asset_id.clone()).or_default();
    let filename = resolved
        .asset
        .original_filename
        .clone()
        .unwrap_or_else(|| asset_id.clone());
    let container_author = video_view::container_author_id(&asset_id);
    let play_author = video_view::play_author_id(&asset_id);

    let frame = egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(8.0)
        .corner_radius(6.0);
    let container = frame.show(ui, |ui| {
        ui.colored_label(palette.text, format!("Video: {filename}"));
        // The play button: clicking dispatches through the focus-safe handler (no OS launch).
        let play = ui.button("\u{25B6} Play");
        emit_node_author(ui.ctx(), play.id, accesskit::Role::Button, &play_author);
        if play.clicked() {
            let handler = InlineRevealPlayHandler;
            // The handler is focus-safe: it reveals the content URL inline (HBR-QUIET).
            let _activation = handler.on_play(&resolved.content_url);
            state.url_revealed = true;
        }
        // The content URL is ALWAYS visible (red-team RISK-4 control: the operator can inspect
        // exactly what would play); after a play click it is emphasized as the revealed target.
        let url_color = if state.url_revealed {
            palette.text
        } else {
            palette.text_subtle
        };
        ui.colored_label(url_color, &resolved.content_url);
        ui.colored_label(
            palette.text_subtle,
            "In-process video decode is deferred to a future MT (poster placeholder).",
        );
    });
    emit_node_author(
        ui.ctx(),
        container.response.id,
        accesskit::Role::Group,
        &container_author,
    );
}

/// Render a `slideshow` embed: one member at a time with wrapping prev/next. `_max_width` is
/// accepted for parity with the single-image path (the sequence frame currently shows the
/// member's metadata + content URL; the per-member texture-decode is the same follow-on as the
/// single image, exercised by the integration test).
fn render_slideshow(
    ui: &mut egui::Ui,
    ref_value: &str,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
    _max_width: f32,
) {
    runtime.ensure_sequence(MediaEmbedKind::Slideshow, ref_value);
    let seq = runtime.sequences.get(ref_value).cloned();
    let items = match seq {
        None | Some(SequenceState::Resolving) => {
            render_spinner(ui, MediaEmbedKind::Slideshow, ref_value);
            return;
        }
        Some(SequenceState::Err(e)) => {
            render_error_chip(ui, ref_value, &e, palette);
            return;
        }
        Some(SequenceState::Items(items)) => items,
    };
    let len = items.len();
    let first_token = slideshow_view::first_asset_token(ref_value);
    let container_author = slideshow_view::container_author_id(ref_value);

    let state = runtime
        .slideshow_states
        .entry(ref_value.to_owned())
        .or_default();
    let idx = state.clamped_index(len);

    let frame = egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(8.0)
        .corner_radius(6.0);
    let container = frame.show(ui, |ui| {
        // Current frame.
        if let Some(item) = items.get(idx) {
            match &item.resolution {
                Ok(resolved) => {
                    ui.colored_label(
                        palette.text,
                        format!(
                            "{} ({}/{})",
                            resolved
                                .asset
                                .original_filename
                                .clone()
                                .unwrap_or_else(|| item.ref_value.clone()),
                            idx + 1,
                            len
                        ),
                    );
                    ui.colored_label(palette.text_subtle, &resolved.content_url);
                }
                Err(e) => render_error_chip(ui, &item.ref_value, e, palette),
            }
        }
        // Prev / position / next controls.
        ui.horizontal(|ui| {
            let prev = ui.button("\u{2039}");
            emit_node_author(
                ui.ctx(),
                prev.id,
                accesskit::Role::Button,
                &slideshow_view::prev_author_id(&first_token),
            );
            if prev.clicked() {
                state.prev(len);
            }
            ui.colored_label(palette.text_subtle, format!("{}/{}", idx + 1, len));
            let next = ui.button("\u{203A}");
            emit_node_author(
                ui.ctx(),
                next.id,
                accesskit::Role::Button,
                &slideshow_view::next_author_id(&first_token),
            );
            if next.clicked() {
                state.next(len);
            }
        });
    });
    emit_node_author(
        ui.ctx(),
        container.response.id,
        accesskit::Role::Group,
        &container_author,
    );
}

/// Render an `album` embed: a 3-per-row thumbnail grid, click-to-enlarge modal.
fn render_album(
    ui: &mut egui::Ui,
    ref_value: &str,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
    _max_width: f32,
) {
    runtime.ensure_sequence(MediaEmbedKind::Album, ref_value);
    let seq = runtime.sequences.get(ref_value).cloned();
    let items = match seq {
        None | Some(SequenceState::Resolving) => {
            render_spinner(ui, MediaEmbedKind::Album, ref_value);
            return;
        }
        Some(SequenceState::Err(e)) => {
            render_error_chip(ui, ref_value, &e, palette);
            return;
        }
        Some(SequenceState::Items(items)) => items,
    };
    let len = items.len();
    let grid_author = album_view::grid_author_id(ref_value);
    let state = runtime
        .album_states
        .entry(ref_value.to_owned())
        .or_default();

    let frame = egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(8.0)
        .corner_radius(6.0);
    let container = frame.show(ui, |ui| {
        egui::Grid::new(("album-grid", ref_value))
            .num_columns(album_view::ALBUM_COLUMNS)
            .spacing(egui::vec2(6.0, 6.0))
            .show(ui, |ui| {
                for (i, item) in items.iter().enumerate() {
                    let cell_label = match &item.resolution {
                        Ok(resolved) => resolved
                            .asset
                            .original_filename
                            .clone()
                            .unwrap_or_else(|| item.ref_value.clone()),
                        Err(e) => format!("[{}]", e.kind_str()),
                    };
                    let cell = ui.button(cell_label);
                    let cell_author = album_view::cell_author_id(&item.ref_value);
                    emit_node_author(ui.ctx(), cell.id, accesskit::Role::Button, &cell_author);
                    if cell.clicked() {
                        state.open(i, len);
                    }
                    if (i + 1) % album_view::ALBUM_COLUMNS == 0 {
                        ui.end_row();
                    }
                }
            });
    });
    emit_node_author(
        ui.ctx(),
        container.response.id,
        accesskit::Role::Group,
        &grid_author,
    );

    // The full-size modal for the open member (egui::Window) — AC-6 click-to-enlarge.
    if let Some(open_idx) = state.open_index {
        let mut keep_open = true;
        egui::Window::new("album-modal")
            .id(egui::Id::new(("album-modal", ref_value)))
            .collapsible(false)
            .open(&mut keep_open)
            .show(ui.ctx(), |ui| {
                if let Some(item) = items.get(open_idx) {
                    match &item.resolution {
                        Ok(resolved) => {
                            ui.colored_label(
                                palette.text,
                                resolved
                                    .asset
                                    .original_filename
                                    .clone()
                                    .unwrap_or_else(|| item.ref_value.clone()),
                            );
                            ui.colored_label(palette.text_subtle, &resolved.content_url);
                        }
                        Err(e) => render_error_chip(ui, &item.ref_value, e, palette),
                    }
                }
            });
        // Re-borrow state (the closure borrowed `items`, not `state`) to sync the closed flag.
        if !keep_open {
            if let Some(state) = runtime.album_states.get_mut(ref_value) {
                state.close();
            }
        }
    }
}

/// Render the Resolving spinner state (an `egui::Spinner` + a label). Non-interactive.
fn render_spinner(ui: &mut egui::Ui, kind: MediaEmbedKind, ref_value: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new());
        ui.label(format!("Resolving {} embed {ref_value}…", kind.ref_kind()));
    });
}

/// Render a typed, VISIBLE, fail-closed embed error chip (never blank). A colored rounded rect
/// (theme `error_bg` fill, `error_text` text) carrying the error-kind text + detail, with the
/// AccessKit author_id `embed-error-{asset_id}` (the contract id) so an out-of-process agent
/// reads the failure. Colors are theme tokens only (CONTROL-4: no hardcoded hex).
fn render_error_chip(ui: &mut egui::Ui, ref_value: &str, error: &EmbedError, palette: &HsPalette) {
    let kind = error.kind_str();
    let author = format!("embed-error-{}", error_chip_token(ref_value));
    let frame = egui::Frame::new()
        .fill(palette.error_bg)
        .stroke(egui::Stroke::new(1.0, palette.error_text))
        .inner_margin(6.0)
        .corner_radius(6.0);
    let resp = frame
        .show(ui, |ui| {
            ui.colored_label(
                palette.error_text,
                format!("Embed failed ({kind}): {error}"),
            );
        })
        .response;
    // A Label-role addressable node so the gate (which only flags UNNAMED interactive nodes)
    // is satisfied and an agent can read the error by id. The chip is not a control, so Label
    // is correct (an author_id on a Label is allowed; see the registry gate doc).
    emit_node_author(ui.ctx(), resp.id, accesskit::Role::Label, &author);
}

/// The stable token used in an error chip's author_id. For a single ref it is the trimmed
/// value (the asset id); for a comma-list it is the first token, matching the contract's
/// `embed-error-{asset_id}` shape. Empty refs use a fixed sentinel so the id is never blank.
fn error_chip_token(ref_value: &str) -> String {
    let token = ref_value.split(',').next().unwrap_or("").trim();
    if token.is_empty() {
        "empty".to_owned()
    } else {
        token.to_owned()
    }
}

/// Emit a stable AccessKit author_id (+ role) onto an already-rendered node, REUSING the WP-011
/// live-emission hook. For an interactive node (a Button) egui already set its interactive
/// role/actions; setting only the author_id keeps those intact (the
/// [`crate::accessibility::emit_interactive_node`] contract). For a non-interactive container
/// (Group/Image/Label) we set the role too so the node is addressable with the right semantics.
fn emit_node_author(ctx: &egui::Context, id: egui::Id, role: accesskit::Role, author_id: &str) {
    let role_for_closure = role;
    let author = author_id.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        // Only stamp the role for non-interactive container/label/image nodes; for an
        // interactive Button egui already chose Role::Button and we must not overwrite it with a
        // generic role, so we leave the role as egui set it and add only the author_id.
        if !matches!(role_for_closure, accesskit::Role::Button) {
            node.set_role(role_for_closure);
        }
        node.set_author_id(author);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::embeds::asset_resolver::{EmbedAssetMetadata, MetadataFuture};

    /// A fetcher that always errors (used to drive the headless Err-chip path without a backend).
    struct NeverFetcher;
    impl AssetMetadataFetcher for NeverFetcher {
        fn fetch_metadata<'a>(&'a self, _ws: &'a str, _id: &'a str) -> MetadataFuture<'a> {
            Box::pin(async { Err(EmbedError::NotFound("never".to_owned())) })
        }
    }

    fn headless_runtime() -> EmbedRuntime {
        EmbedRuntime::new("ws", "http://b", Arc::new(NeverFetcher), None)
    }

    fn ok_resolved(asset_id: &str) -> ResolvedAsset {
        ResolvedAsset {
            asset: EmbedAssetMetadata {
                asset_id: asset_id.to_owned(),
                workspace_id: "ws".to_owned(),
                kind: "image".to_owned(),
                mime: "image/png".to_owned(),
                original_filename: Some(format!("{asset_id}.png")),
                content_hash: String::new(),
                size_bytes: 0,
                width: Some(10),
                height: Some(10),
            },
            content_url: format!("http://b/workspaces/ws/assets/{asset_id}/content"),
            thumbnail_url: format!("http://b/workspaces/ws/assets/{asset_id}/thumbnail"),
        }
    }

    #[test]
    fn empty_ref_renders_error_chip_no_fetch_ac2() {
        // AC-2: an empty ref renders the typed empty_ref chip (not blank), with no fetch attempt.
        let mut rt = headless_runtime();
        let link = HsLinkNode::new("images", "", "");
        let ctx = egui::Context::default();
        let pal = crate::theme::HsTheme::Dark.palette();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_embed(ui, &link, &mut rt, &pal);
            });
        });
        // No asset was ever fetched (the empty ref short-circuits before ensure_single).
        assert!(
            rt.resolutions.is_empty(),
            "AC-2: an empty ref issues no resolution"
        );
    }

    #[test]
    fn drain_applies_single_delivery() {
        let mut rt = headless_runtime();
        // Simulate an off-thread delivery landing in the cell.
        *rt.single_cell.lock().unwrap() =
            Some(("a1".to_owned(), EmbedResolutionState::Ok(ok_resolved("a1"))));
        assert!(rt.drain_deliveries());
        assert!(matches!(
            rt.resolutions.get("a1"),
            Some(EmbedResolutionState::Ok(_))
        ));
    }

    #[test]
    fn ensure_single_is_idempotent_ac9() {
        // AC-9: a terminal asset is never re-marked / re-spawned. Seed an Ok, then ensure_single
        // must NOT downgrade it back to Resolving.
        let mut rt = headless_runtime();
        rt.resolutions
            .insert("a1", EmbedResolutionState::Ok(ok_resolved("a1")));
        rt.ensure_single(MediaEmbedKind::Images, "a1");
        assert!(
            matches!(rt.resolutions.get("a1"), Some(EmbedResolutionState::Ok(_))),
            "AC-9: ensure_single must not re-resolve a terminal asset"
        );
    }

    #[test]
    fn ensure_sequence_is_idempotent() {
        let mut rt = headless_runtime();
        rt.sequences
            .insert("a1,a2".to_owned(), SequenceState::Err(EmbedError::EmptyRef));
        rt.ensure_sequence(MediaEmbedKind::Album, "a1,a2");
        assert!(matches!(
            rt.sequences.get("a1,a2"),
            Some(SequenceState::Err(_))
        ));
    }
}
