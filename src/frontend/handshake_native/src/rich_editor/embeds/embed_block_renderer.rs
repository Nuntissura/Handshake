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
use std::time::Duration;

use egui::accesskit;

use crate::rich_editor::document_model::node::HsLinkNode;
use crate::rich_editor::embeds::album_view::{self, AlbumViewState};
use crate::rich_editor::embeds::asset_resolver::{
    AssetMetadataFetcher, EmbedError, EmbedResolutionCache, EmbedResolutionState, MediaEmbedKind,
    MediaTier, ResolvedAsset, SequenceItem, EMBED_OPERATION_TIMEOUT, MAX_CONCURRENT_RESOLUTIONS,
};
use crate::rich_editor::embeds::image_view::{self, EmbedTextureCache};
use crate::rich_editor::embeds::slideshow_view::{self, SlideshowViewState};
use crate::rich_editor::embeds::video_view::{
    self, InlineRevealPlayHandler, VideoPlayHandler, VideoViewState,
};
use crate::theme::HsPalette;

type Generation = u64;

/// Generation-stamped multi-result queue for off-thread single-asset resolutions. A queue is
/// required because several embeds may finish between two frames; a one-slot cell silently lost
/// all but the last completion and left the overwritten assets permanently `Resolving`.
type SingleDeliveryCell =
    Arc<Mutex<Vec<(Generation, MediaEmbedKind, String, EmbedResolutionState)>>>;

/// Multi-result delivery queue for off-thread album/slideshow sequence resolutions: keyed by the
/// embed's ref_value, carrying the per-member items.
type SequenceDeliveryCell = Arc<
    Mutex<
        Vec<(
            Generation,
            MediaEmbedKind,
            String,
            Result<Vec<SequenceItem>, EmbedError>,
        )>,
    >,
>;

/// Multi-slot delivery cell for off-thread image CONTENT decode results (MC-001): the spawned
/// task fetches `GET .../content`, decodes the bytes on `tokio::spawn_blocking`, and writes the
/// decoded [`egui::ColorImage`] (or a typed decode/fetch error) here keyed by asset id. The egui
/// UI thread drains it next frame and uploads the `ColorImage` as a `TextureHandle` (the upload
/// MUST happen on the egui thread — only the platform-independent RGBA `ColorImage` crosses the
/// thread boundary, impl note 2). A `Vec` (not a single slot) so several images in one document
/// can deliver in the same frame without clobbering each other.
type ContentDeliveryCell = Arc<
    Mutex<
        Vec<(
            Generation,
            MediaEmbedKind,
            MediaTier,
            String,
            Result<egui::ColorImage, EmbedError>,
        )>,
    >,
>;

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
    /// retained under the matching kind+tier key so the typed error chip shows (MC-005).
    pub decoded_images: std::collections::HashMap<String, egui::ColorImage>,
    /// Terminal fetch/decode failures are kept per kind+tier+asset. A failed thumbnail must not
    /// poison the metadata cache or a poster/full-size request for the same backend asset.
    media_errors: std::collections::HashMap<String, EmbedError>,
    /// Asset ids whose CONTENT fetch+decode has been kicked off (so the bytes pipeline runs
    /// ONCE per asset, mirroring the metadata AC-9 caching for the content fetch).
    content_in_flight: std::collections::HashSet<String>,
    /// Per-ref_value slideshow paging state.
    pub slideshow_states: std::collections::HashMap<String, SlideshowViewState>,
    /// Per-ref_value album modal state.
    pub album_states: std::collections::HashMap<String, AlbumViewState>,
    /// Single-image assets whose full-size modal is open.
    pub image_modals: std::collections::HashSet<String>,
    /// Per-asset video reveal state.
    pub video_states: std::collections::HashMap<String, VideoViewState>,
    /// Delivery cell for off-thread single resolutions (drained at frame top).
    single_cell: SingleDeliveryCell,
    /// Delivery cell for off-thread sequence resolutions (drained at frame top).
    sequence_cell: SequenceDeliveryCell,
    /// Delivery cell for off-thread image-content decodes (drained at frame top).
    content_cell: ContentDeliveryCell,
    /// Workspace snapshot used to detect direct shell rebinding through `workspace_id`.
    context_workspace_id: String,
    /// Monotonic identity carried by every async delivery. Results from an older workspace are
    /// discarded even if they arrive after cancellation.
    generation: Generation,
    /// Owned tasks are aborted on workspace rebind and runtime teardown.
    pending_tasks: Vec<tokio::task::JoinHandle<()>>,
    /// Upper bound for every remote metadata/content operation. Kept on the runtime so tests can
    /// prove timeout recovery without waiting for the production bound.
    operation_timeout: Duration,
    /// One shared <=6 budget across metadata, collection, body fetch, and decode work.
    work_budget: Arc<tokio::sync::Semaphore>,
}

impl EmbedRuntime {
    fn resolution_key(kind: MediaEmbedKind, asset_id: &str) -> String {
        format!("{}:{asset_id}", kind.ref_kind())
    }

    fn sequence_key(kind: MediaEmbedKind, ref_value: &str) -> String {
        format!("{}:{ref_value}", kind.ref_kind())
    }

    fn media_key(kind: MediaEmbedKind, tier: MediaTier, asset_id: &str) -> String {
        format!("{}:{}:{asset_id}", kind.ref_kind(), tier.as_str())
    }

    fn resolution(&self, kind: MediaEmbedKind, asset_id: &str) -> Option<&EmbedResolutionState> {
        self.resolutions.get(&Self::resolution_key(kind, asset_id))
    }

    fn sequence(&self, kind: MediaEmbedKind, ref_value: &str) -> Option<&SequenceState> {
        self.sequences.get(&Self::sequence_key(kind, ref_value))
    }

    fn retry_asset(&mut self, kind: MediaEmbedKind, asset_id: &str) {
        self.resolutions
            .remove(&Self::resolution_key(kind, asset_id));
        for tier in [
            MediaTier::Thumbnail,
            MediaTier::Preview,
            MediaTier::Poster,
            MediaTier::Full,
        ] {
            let key = Self::media_key(kind, tier, asset_id);
            self.textures.remove(&key);
            self.decoded_images.remove(&key);
            self.content_in_flight.remove(&key);
            self.media_errors.remove(&key);
        }
    }

    fn retry_sequence(&mut self, kind: MediaEmbedKind, ref_value: &str) {
        self.sequences.remove(&Self::sequence_key(kind, ref_value));
    }

    /// Build a runtime over `fetcher` for `workspace_id`/`base_url`, bridging async resolution
    /// onto `runtime` (pass `None` only for headless tests that do not spawn).
    pub fn new(
        workspace_id: impl Into<String>,
        base_url: impl Into<String>,
        fetcher: Arc<dyn AssetMetadataFetcher>,
        runtime: Option<tokio::runtime::Handle>,
    ) -> Self {
        let workspace_id = workspace_id.into();
        Self {
            context_workspace_id: workspace_id.clone(),
            workspace_id,
            base_url: base_url.into(),
            fetcher,
            runtime,
            resolutions: EmbedResolutionCache::new(),
            sequences: std::collections::HashMap::new(),
            textures: EmbedTextureCache::new(),
            decoded_images: std::collections::HashMap::new(),
            media_errors: std::collections::HashMap::new(),
            content_in_flight: std::collections::HashSet::new(),
            slideshow_states: std::collections::HashMap::new(),
            album_states: std::collections::HashMap::new(),
            image_modals: std::collections::HashSet::new(),
            video_states: std::collections::HashMap::new(),
            single_cell: Arc::new(Mutex::new(Vec::new())),
            sequence_cell: Arc::new(Mutex::new(Vec::new())),
            content_cell: Arc::new(Mutex::new(Vec::new())),
            generation: 0,
            pending_tasks: Vec::new(),
            operation_timeout: EMBED_OPERATION_TIMEOUT,
            work_budget: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_RESOLUTIONS)),
        }
    }

    /// Reconcile the public shell-bound workspace field with all workspace-local caches. The
    /// shell currently writes `workspace_id` directly; detecting that mutation here keeps every
    /// render path safe without requiring a second host integration point.
    fn sync_context(&mut self) {
        if self.context_workspace_id == self.workspace_id {
            self.pending_tasks.retain(|task| !task.is_finished());
            return;
        }

        self.generation = self.generation.wrapping_add(1);
        self.context_workspace_id.clone_from(&self.workspace_id);
        for task in self.pending_tasks.drain(..) {
            task.abort();
        }
        self.resolutions.clear();
        self.sequences.clear();
        self.textures.clear();
        self.decoded_images.clear();
        self.media_errors.clear();
        self.content_in_flight.clear();
        self.slideshow_states.clear();
        self.album_states.clear();
        self.image_modals.clear();
        self.video_states.clear();
    }

    /// Drain any off-thread resolution results delivered since the last frame into the caches.
    /// Called at the top of a render frame so a completed fetch updates the cache before the
    /// embed re-renders. Returns true when a result was applied (the caller can request a
    /// repaint so the new state shows immediately).
    pub fn drain_deliveries(&mut self) -> bool {
        self.sync_context();
        let mut applied = false;
        if let Ok(mut deliveries) = self.single_cell.lock() {
            for (generation, kind, asset_id, state) in deliveries.drain(..) {
                if generation != self.generation {
                    continue;
                }
                self.resolutions
                    .insert(Self::resolution_key(kind, &asset_id), state);
                applied = true;
            }
        }
        if let Ok(mut deliveries) = self.sequence_cell.lock() {
            for (generation, kind, ref_value, result) in deliveries.drain(..) {
                if generation != self.generation {
                    continue;
                }
                let state = match result {
                    Ok(items) => SequenceState::Items(Arc::new(items)),
                    Err(e) => SequenceState::Err(e),
                };
                self.sequences
                    .insert(Self::sequence_key(kind, &ref_value), state);
                applied = true;
            }
        }
        // Drain off-thread image-content decode results. A decoded ColorImage is parked in
        // `decoded_images` for the egui thread to upload (impl note 2: the upload must be on the
        // egui thread). Decode/fetch failures remain tier-scoped so one failed representation
        // cannot poison valid metadata or another media kind/tier for the same asset id.
        if let Ok(mut deliveries) = self.content_cell.lock() {
            for (generation, kind, tier, asset_id, result) in deliveries.drain(..) {
                if generation != self.generation {
                    continue;
                }
                let media_key = Self::media_key(kind, tier, &asset_id);
                self.content_in_flight.remove(&media_key);
                match result {
                    Ok(image) => {
                        self.media_errors.remove(&media_key);
                        self.decoded_images.insert(media_key, image);
                    }
                    Err(e) => {
                        self.media_errors.insert(media_key, e);
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
        self.sync_context();
        let cache_key = Self::resolution_key(kind, asset_id);
        if self.resolution(kind, asset_id).is_some() {
            return; // already resolving / resolved / failed — do not re-spawn (AC-9).
        }
        let Some(runtime) = self.runtime.clone() else {
            return; // do not strand the asset in Resolving; retry once a runtime is attached.
        };
        self.resolutions
            .insert(cache_key, EmbedResolutionState::Resolving);
        let fetcher = Arc::clone(&self.fetcher);
        let cell = Arc::clone(&self.single_cell);
        let workspace_id = self.workspace_id.clone();
        let base_url = self.base_url.clone();
        let asset_id = asset_id.to_owned();
        let generation = self.generation;
        let operation_timeout = self.operation_timeout;
        let work_budget = Arc::clone(&self.work_budget);
        let task = runtime.spawn(async move {
            let result = tokio::time::timeout(operation_timeout, async {
                let _permit = work_budget
                    .acquire_owned()
                    .await
                    .map_err(|_| EmbedError::ServerError("embed work budget closed".to_owned()))?;
                crate::rich_editor::embeds::asset_resolver::resolve_one(
                    kind,
                    &workspace_id,
                    &asset_id,
                    &base_url,
                    fetcher.as_ref(),
                )
                .await
            })
            .await
            .unwrap_or_else(|_| {
                Err(EmbedError::TimedOut(format!(
                    "resolving asset '{asset_id}' exceeded {operation_timeout:?}"
                )))
            });
            let state = match result {
                Ok(r) => EmbedResolutionState::Ok(r),
                Err(e) => EmbedResolutionState::Err(e),
            };
            if let Ok(mut deliveries) = cell.lock() {
                deliveries.push((generation, kind, asset_id, state));
            }
        });
        self.pending_tasks.push(task);
    }

    /// Ensure the CONTENT bytes for a resolved image asset are being (or have been) fetched +
    /// decoded off-thread (MC-001): once metadata resolved Ok, this fetches `GET .../content` and
    /// runs [`image_view::decode_rgba`] on `tokio::spawn_blocking`, delivering the decoded
    /// [`egui::ColorImage`] back through the content delivery cell for the egui thread to upload.
    /// Runs ONCE per asset (`content_in_flight` guard), mirroring the metadata AC-9 caching. A
    /// no-op when there is no runtime (headless path — a test injects the decoded image directly).
    /// This is the production path that makes [`render_resolved_image`] reach its texture branch.
    fn ensure_image_content(&mut self, kind: MediaEmbedKind, asset_id: &str, tier: MediaTier) {
        self.sync_context();
        let media_key = Self::media_key(kind, tier, asset_id);
        // Already uploaded, already decoded-and-waiting, or already fetching -> do not re-fetch.
        if self.textures.contains(&media_key)
            || self.decoded_images.contains_key(&media_key)
            || self.content_in_flight.contains(&media_key)
            || self.media_errors.contains_key(&media_key)
        {
            return;
        }
        let Some(runtime) = self.runtime.clone() else {
            return; // headless: the caller delivers the decoded image directly in tests.
        };
        self.content_in_flight.insert(media_key);
        let fetcher = Arc::clone(&self.fetcher);
        let cell = Arc::clone(&self.content_cell);
        let workspace_id = self.workspace_id.clone();
        let asset_id = asset_id.to_owned();
        let generation = self.generation;
        let operation_timeout = self.operation_timeout;
        let work_budget = Arc::clone(&self.work_budget);
        let task = runtime.spawn(async move {
            // The single operation deadline covers queueing for the shared six-permit budget,
            // transport/body streaming, and off-thread decode. A saturated budget therefore
            // becomes a typed timeout instead of a permanent spinner.
            let decoded = tokio::time::timeout(operation_timeout, async {
                let permit = work_budget
                    .acquire_owned()
                    .await
                    .map_err(|_| EmbedError::ServerError("embed work budget closed".to_owned()))?;
                // 1) Fetch the raw content bytes (GET .../content).
                let bytes = fetcher
                    .fetch_tier(&workspace_id, &asset_id, kind, tier)
                    .await?;
                // 2) Decode off the async/UI thread (MC-001).
                tokio::task::spawn_blocking(move || {
                    // `spawn_blocking` work cannot be cancelled once it starts. Move the shared
                    // permit into the blocking closure so a timed-out caller cannot release its
                    // concurrency slot while the decode is still consuming CPU/memory.
                    let _permit = permit;
                    image_view::decode_rgba(&bytes)
                })
                .await
                .unwrap_or_else(|join_err| {
                    Err(EmbedError::MediaLoadFailed(format!(
                        "image decode task failed: {join_err}"
                    )))
                })
            })
            .await
            .unwrap_or_else(|_| {
                Err(EmbedError::TimedOut(format!(
                    "loading pixels for asset '{asset_id}' exceeded {operation_timeout:?}"
                )))
            });
            // 3) Deliver the decoded ColorImage (or typed error) for the egui thread to upload.
            if let Ok(mut deliveries) = cell.lock() {
                deliveries.push((generation, kind, tier, asset_id, decoded));
            }
        });
        self.pending_tasks.push(task);
    }

    /// Ensure an album/slideshow sequence is being (or has been) resolved (AC-9 at sequence
    /// level). A no-op when there is no runtime (headless test path).
    fn ensure_sequence(&mut self, kind: MediaEmbedKind, ref_value: &str) {
        self.sync_context();
        let sequence_key = Self::sequence_key(kind, ref_value);
        if self.sequence(kind, ref_value).is_some() {
            return; // already resolving / resolved.
        }
        let Some(runtime) = self.runtime.clone() else {
            return; // retry when the host attaches its runtime; never leave a permanent spinner.
        };
        self.sequences
            .insert(sequence_key, SequenceState::Resolving);
        let fetcher = Arc::clone(&self.fetcher);
        let cell = Arc::clone(&self.sequence_cell);
        let workspace_id = self.workspace_id.clone();
        let base_url = self.base_url.clone();
        let ref_value = ref_value.to_owned();
        let generation = self.generation;
        let operation_timeout = self.operation_timeout;
        let work_budget = Arc::clone(&self.work_budget);
        let task = runtime.spawn(async move {
            let result = tokio::time::timeout(
                operation_timeout,
                crate::rich_editor::embeds::asset_resolver::resolve_sequence_with_budget(
                    kind,
                    &workspace_id,
                    &ref_value,
                    &base_url,
                    fetcher,
                    work_budget,
                ),
            )
            .await
            .unwrap_or_else(|_| {
                Err(EmbedError::TimedOut(format!(
                    "resolving sequence '{ref_value}' exceeded {operation_timeout:?}"
                )))
            });
            if let Ok(mut deliveries) = cell.lock() {
                deliveries.push((generation, kind, ref_value, result));
            }
        });
        self.pending_tasks.push(task);
    }
}

impl Drop for EmbedRuntime {
    fn drop(&mut self) {
        for task in self.pending_tasks.drain(..) {
            task.abort();
        }
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

    match runtime.resolution(kind, &asset_id).cloned() {
        None | Some(EmbedResolutionState::Resolving) => render_spinner(ui, kind, ref_value),
        Some(EmbedResolutionState::Err(e)) => {
            render_retryable_error(ui, kind, &asset_id, &e, runtime, palette)
        }
        Some(EmbedResolutionState::Ok(resolved)) => {
            let response = render_resolved_image(
                ui,
                kind,
                MediaTier::Thumbnail,
                &asset_id,
                &resolved,
                runtime,
                palette,
                max_width,
            );
            if response.clicked() {
                runtime.image_modals.insert(asset_id.clone());
            }

            if runtime.image_modals.contains(&asset_id) {
                let mut keep_open = true;
                let mut close_clicked = false;
                let modal = egui::Window::new(format!("Image: {asset_id}"))
                    .id(egui::Id::new(("image-modal", &asset_id)))
                    .collapsible(false)
                    .open(&mut keep_open)
                    .show(ui.ctx(), |ui| {
                        let close = ui.button("Close");
                        emit_node_author(
                            ui.ctx(),
                            close.id,
                            accesskit::Role::Button,
                            &format!("embed-image-modal-close-{asset_id}"),
                        );
                        close_clicked = close.clicked();
                        let available = ui.available_width().max(1.0);
                        let _ = render_resolved_image(
                            ui,
                            kind,
                            MediaTier::Full,
                            &asset_id,
                            &resolved,
                            runtime,
                            palette,
                            available,
                        );
                    });
                if let Some(modal) = modal {
                    emit_node_author(
                        ui.ctx(),
                        modal.response.id,
                        accesskit::Role::Dialog,
                        &format!("embed-image-modal-{asset_id}"),
                    );
                }
                if close_clicked || !keep_open {
                    runtime.image_modals.remove(&asset_id);
                }
            }
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
#[allow(clippy::too_many_arguments)]
fn render_resolved_image(
    ui: &mut egui::Ui,
    kind: MediaEmbedKind,
    tier: MediaTier,
    asset_id: &str,
    resolved: &ResolvedAsset,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
    max_width: f32,
) -> egui::Response {
    let media_key = EmbedRuntime::media_key(kind, tier, asset_id);
    let author = if tier == MediaTier::Full {
        format!("embed-image-full-{asset_id}")
    } else {
        format!("embed-image-{asset_id}")
    };

    if let Some(error) = runtime.media_errors.get(&media_key).cloned() {
        let response = render_error_chip(ui, asset_id, &error, palette);
        if is_retryable(&error) {
            let retry = ui.button("Retry");
            emit_node_author(
                ui.ctx(),
                retry.id,
                accesskit::Role::Button,
                &format!("embed-retry-{asset_id}"),
            );
            if retry.clicked() {
                runtime.media_errors.remove(&media_key);
                runtime.content_in_flight.remove(&media_key);
                runtime.decoded_images.remove(&media_key);
                runtime.textures.remove(&media_key);
                ui.ctx().request_repaint();
            }
        }
        return response;
    }

    // (2) Upload a freshly-decoded image (delivered off-thread) on the egui thread, before the
    // texture-branch check, so the first frame after delivery already renders the real texture.
    if !runtime.textures.contains(&media_key) {
        if let Some(image) = runtime.decoded_images.remove(&media_key) {
            let _texture = runtime.textures.upload(ui.ctx(), &media_key, image);
            ui.ctx().request_repaint();
        }
    }

    // (1) Texture ready -> render the decoded image at aspect-correct width (AC-1).
    if let Some(texture) = runtime.textures.get(&media_key).cloned() {
        let resp = ui
            .scope(|ui| image_view::render_image(ui, &texture, resolved, max_width))
            .inner;
        emit_node_author(ui.ctx(), resp.id, accesskit::Role::Image, &author);
        return resp;
    }

    // (3) No texture yet: drive the content fetch + off-thread decode (once) and show the
    // decoding spinner while the pixels load (fail-closed, never blank). The content URL is shown
    // beneath the spinner so the operator can inspect exactly what is loading.
    runtime.ensure_image_content(kind, asset_id, tier);
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
            let url = match tier {
                MediaTier::Thumbnail => &resolved.thumbnail_url,
                MediaTier::Preview => &resolved.preview_url,
                MediaTier::Poster => &resolved.poster_url,
                MediaTier::Full => &resolved.content_url,
            };
            ui.colored_label(palette.text_subtle, url);
        })
        .response;
    emit_node_author(ui.ctx(), resp.id, accesskit::Role::Image, &author);
    resp
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

    let resolved = match runtime
        .resolution(MediaEmbedKind::Video, &asset_id)
        .cloned()
    {
        None | Some(EmbedResolutionState::Resolving) => {
            render_spinner(ui, MediaEmbedKind::Video, ref_value);
            return;
        }
        Some(EmbedResolutionState::Err(e)) => {
            render_retryable_error(ui, MediaEmbedKind::Video, &asset_id, &e, runtime, palette);
            return;
        }
        Some(EmbedResolutionState::Ok(r)) => r,
    };

    let was_revealed = runtime
        .video_states
        .get(&asset_id)
        .is_some_and(|state| state.url_revealed);
    let filename = resolved
        .asset
        .original_filename
        .clone()
        .unwrap_or_else(|| asset_id.clone());
    let container_author = video_view::container_author_id(&asset_id);
    let play_author = video_view::play_author_id(&asset_id);
    let mut play_clicked = false;

    let frame = egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(8.0)
        .corner_radius(6.0);
    let container = frame.show(ui, |ui| {
        ui.colored_label(palette.text, format!("Video: {filename}"));
        let poster_width = ui.available_width().max(1.0);
        let _poster = render_resolved_image(
            ui,
            MediaEmbedKind::Video,
            MediaTier::Poster,
            &asset_id,
            &resolved,
            runtime,
            palette,
            poster_width,
        );
        // The play button: clicking dispatches through the focus-safe handler (no OS launch).
        let play = ui.button("\u{25B6} Play");
        emit_node_author(ui.ctx(), play.id, accesskit::Role::Button, &play_author);
        if play.clicked() {
            let handler = InlineRevealPlayHandler;
            // The handler is focus-safe: it reveals the content URL inline (HBR-QUIET).
            let _activation = handler.on_play(&resolved.content_url);
            play_clicked = true;
        }
        // The content URL is ALWAYS visible (red-team RISK-4 control: the operator can inspect
        // exactly what would play); after a play click it is emphasized as the revealed target.
        let url_color = if was_revealed || play_clicked {
            palette.text
        } else {
            palette.text_subtle
        };
        ui.colored_label(url_color, &resolved.content_url);
        ui.colored_label(
            palette.text_subtle,
            "Poster is loaded independently; Play reveals the bounded in-process media target.",
        );
    });
    if play_clicked {
        runtime
            .video_states
            .entry(asset_id.clone())
            .or_default()
            .url_revealed = true;
    }
    emit_node_author(
        ui.ctx(),
        container.response.id,
        accesskit::Role::Group,
        &container_author,
    );
}

/// Render a `slideshow` embed: one decoded image at a time with wrapping prev/next.
fn render_slideshow(
    ui: &mut egui::Ui,
    ref_value: &str,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
    max_width: f32,
) {
    runtime.ensure_sequence(MediaEmbedKind::Slideshow, ref_value);
    let seq = runtime
        .sequence(MediaEmbedKind::Slideshow, ref_value)
        .cloned();
    let items = match seq {
        None | Some(SequenceState::Resolving) => {
            render_spinner(ui, MediaEmbedKind::Slideshow, ref_value);
            return;
        }
        Some(SequenceState::Err(e)) => {
            render_retryable_sequence_error(
                ui,
                MediaEmbedKind::Slideshow,
                ref_value,
                &e,
                runtime,
                palette,
            );
            return;
        }
        Some(SequenceState::Items(items)) => items,
    };
    let len = items.len();
    let first_token = slideshow_view::first_asset_token(ref_value);
    let container_author = slideshow_view::container_author_id(ref_value);

    let idx = runtime
        .slideshow_states
        .entry(ref_value.to_owned())
        .or_default()
        .clamped_index(len);

    let frame = egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(8.0)
        .corner_radius(6.0);
    let mut prev_clicked = false;
    let mut next_clicked = false;
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
                    let _ = render_resolved_image(
                        ui,
                        MediaEmbedKind::Slideshow,
                        MediaTier::Full,
                        &item.ref_value,
                        resolved,
                        runtime,
                        palette,
                        max_width,
                    );
                }
                Err(e) => render_retryable_sequence_error(
                    ui,
                    MediaEmbedKind::Slideshow,
                    ref_value,
                    e,
                    runtime,
                    palette,
                ),
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
                prev_clicked = true;
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
                next_clicked = true;
            }
        });
    });
    if let Some(state) = runtime.slideshow_states.get_mut(ref_value) {
        if prev_clicked {
            state.prev(len);
        }
        if next_clicked {
            state.next(len);
        }
    }
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
    max_width: f32,
) {
    runtime.ensure_sequence(MediaEmbedKind::Album, ref_value);
    let seq = runtime.sequence(MediaEmbedKind::Album, ref_value).cloned();
    let items = match seq {
        None | Some(SequenceState::Resolving) => {
            render_spinner(ui, MediaEmbedKind::Album, ref_value);
            return;
        }
        Some(SequenceState::Err(e)) => {
            render_retryable_sequence_error(
                ui,
                MediaEmbedKind::Album,
                ref_value,
                &e,
                runtime,
                palette,
            );
            return;
        }
        Some(SequenceState::Items(items)) => items,
    };
    let len = items.len();
    let grid_author = album_view::grid_author_id(ref_value);
    let open_index = runtime
        .album_states
        .entry(ref_value.to_owned())
        .or_default()
        .open_index;

    let frame = egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(8.0)
        .corner_radius(6.0);
    let mut clicked_index = None;
    let cell_width = ((max_width - 12.0) / album_view::ALBUM_COLUMNS as f32).max(48.0);
    let container = frame.show(ui, |ui| {
        egui::Grid::new(("album-grid", ref_value))
            .num_columns(album_view::ALBUM_COLUMNS)
            .spacing(egui::vec2(6.0, 6.0))
            .show(ui, |ui| {
                for (i, item) in items.iter().enumerate() {
                    match &item.resolution {
                        Ok(resolved) => {
                            let cell = render_resolved_image(
                                ui,
                                MediaEmbedKind::Album,
                                MediaTier::Thumbnail,
                                &item.ref_value,
                                resolved,
                                runtime,
                                palette,
                                cell_width,
                            );
                            let cell_author = album_view::cell_author_id(&item.ref_value);
                            emit_node_author(
                                ui.ctx(),
                                cell.id,
                                accesskit::Role::Button,
                                &cell_author,
                            );
                            if cell.clicked() {
                                clicked_index = Some(i);
                            }
                        }
                        Err(e) => render_retryable_sequence_error(
                            ui,
                            MediaEmbedKind::Album,
                            ref_value,
                            e,
                            runtime,
                            palette,
                        ),
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
    if let Some(index) = clicked_index {
        runtime
            .album_states
            .entry(ref_value.to_owned())
            .or_default()
            .open(index, len);
    }

    // The full-size modal for the open member (egui::Window) — AC-6 click-to-enlarge.
    let open_index = clicked_index.or(open_index);
    if let Some(open_idx) = open_index {
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
                            let available = ui.available_width().max(1.0);
                            let _ = render_resolved_image(
                                ui,
                                MediaEmbedKind::Album,
                                MediaTier::Full,
                                &item.ref_value,
                                resolved,
                                runtime,
                                palette,
                                available,
                            );
                        }
                        Err(e) => render_retryable_sequence_error(
                            ui,
                            MediaEmbedKind::Album,
                            ref_value,
                            e,
                            runtime,
                            palette,
                        ),
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

/// Render the Resolving spinner state (an `egui::Spinner` + a label). Non-interactive, but
/// AccessKit-addressable as `embed-loading-{token}` so the LOADING state is observable through
/// Argus/an out-of-process agent (HBR-VIS): without a stable id the mounted loading state was
/// invisible to canonical inspection, so a swarm agent could not distinguish "still resolving"
/// from "not present". Uses the same first-comma `token` shape as the error chip.
fn render_spinner(ui: &mut egui::Ui, kind: MediaEmbedKind, ref_value: &str) {
    let resp = ui
        .horizontal(|ui| {
            ui.add(egui::Spinner::new());
            ui.label(format!("Resolving {} embed {ref_value}…", kind.ref_kind()));
        })
        .response;
    emit_node_author(
        ui.ctx(),
        resp.id,
        accesskit::Role::Label,
        &format!("embed-loading-{}", error_chip_token(ref_value)),
    );
}

/// Render a typed, VISIBLE, fail-closed embed error chip (never blank). A colored rounded rect
/// (theme `error_bg` fill, `error_text` text) carrying the error-kind text + detail, with the
/// AccessKit author_id `embed-error-{asset_id}` (the contract id) so an out-of-process agent
/// reads the failure. Colors are theme tokens only (CONTROL-4: no hardcoded hex).
fn render_error_chip(
    ui: &mut egui::Ui,
    ref_value: &str,
    error: &EmbedError,
    palette: &HsPalette,
) -> egui::Response {
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
    resp
}

fn is_retryable(error: &EmbedError) -> bool {
    matches!(
        error,
        EmbedError::NetworkError(_)
            | EmbedError::ServerError(_)
            | EmbedError::TimedOut(_)
            | EmbedError::MediaLoadFailed(_)
            | EmbedError::NotFound(_)
    )
}

fn render_retryable_error(
    ui: &mut egui::Ui,
    kind: MediaEmbedKind,
    asset_id: &str,
    error: &EmbedError,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
) {
    render_error_chip(ui, asset_id, error, palette);
    if is_retryable(error) {
        let retry = ui.button("Retry");
        emit_node_author(
            ui.ctx(),
            retry.id,
            accesskit::Role::Button,
            &format!("embed-retry-{asset_id}"),
        );
        if retry.clicked() {
            runtime.retry_asset(kind, asset_id);
            ui.ctx().request_repaint();
        }
    }
}

fn render_retryable_sequence_error(
    ui: &mut egui::Ui,
    kind: MediaEmbedKind,
    ref_value: &str,
    error: &EmbedError,
    runtime: &mut EmbedRuntime,
    palette: &HsPalette,
) {
    render_error_chip(ui, ref_value, error, palette);
    if is_retryable(error) {
        let token = error_chip_token(ref_value);
        let retry = ui.button("Retry");
        emit_node_author(
            ui.ctx(),
            retry.id,
            accesskit::Role::Button,
            &format!("embed-retry-{token}"),
        );
        if retry.clicked() {
            runtime.retry_sequence(kind, ref_value);
            ui.ctx().request_repaint();
        }
    }
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

/// Emit a stable AccessKit author_id + role onto an already-rendered node, reusing the WP-011
/// live-emission hook. Album images intentionally become Button-role controls because the image
/// itself is the click target that opens the modal.
fn emit_node_author(ctx: &egui::Context, id: egui::Id, role: accesskit::Role, author_id: &str) {
    let role_for_closure = role;
    let author = author_id.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        node.set_role(role_for_closure);
        node.set_author_id(crate::rich_editor::scoped_author_id(author));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::embeds::asset_resolver::{
        ContentFuture, EmbedAssetMetadata, MetadataFuture,
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// A fetcher that always errors (used to drive the headless Err-chip path without a backend).
    struct NeverFetcher;
    impl AssetMetadataFetcher for NeverFetcher {
        fn fetch_metadata<'a>(&'a self, _ws: &'a str, _id: &'a str) -> MetadataFuture<'a> {
            Box::pin(async { Err(EmbedError::NotFound("never".to_owned())) })
        }
    }

    /// A transport that never completes, used to prove the bounded timeout terminal state.
    struct PendingFetcher;
    impl AssetMetadataFetcher for PendingFetcher {
        fn fetch_metadata<'a>(&'a self, _ws: &'a str, _id: &'a str) -> MetadataFuture<'a> {
            Box::pin(std::future::pending())
        }
    }

    struct SharedBudgetFetcher {
        active: AtomicUsize,
        high_water: AtomicUsize,
        png: Vec<u8>,
    }

    impl SharedBudgetFetcher {
        fn enter(&self) {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.high_water.fetch_max(active, Ordering::SeqCst);
        }

        fn leave(&self) {
            self.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl AssetMetadataFetcher for SharedBudgetFetcher {
        fn fetch_metadata<'a>(
            &'a self,
            workspace_id: &'a str,
            asset_id: &'a str,
        ) -> MetadataFuture<'a> {
            Box::pin(async move {
                self.enter();
                tokio::time::sleep(Duration::from_millis(10)).await;
                self.leave();
                Ok(EmbedAssetMetadata {
                    asset_id: asset_id.to_owned(),
                    workspace_id: workspace_id.to_owned(),
                    kind: "image".to_owned(),
                    mime: "image/png".to_owned(),
                    original_filename: None,
                    content_hash: String::new(),
                    size_bytes: self.png.len() as u64,
                    width: Some(1),
                    height: Some(1),
                })
            })
        }

        fn fetch_tier<'a>(
            &'a self,
            _workspace_id: &'a str,
            _asset_id: &'a str,
            _kind: MediaEmbedKind,
            _tier: MediaTier,
        ) -> ContentFuture<'a> {
            Box::pin(async move {
                self.enter();
                tokio::time::sleep(Duration::from_millis(10)).await;
                self.leave();
                Ok(self.png.clone())
            })
        }
    }

    struct DropTrackedFetcher {
        future_dropped: Arc<AtomicBool>,
    }

    impl AssetMetadataFetcher for DropTrackedFetcher {
        fn fetch_metadata<'a>(&'a self, _ws: &'a str, _id: &'a str) -> MetadataFuture<'a> {
            let dropped = Arc::clone(&self.future_dropped);
            Box::pin(async move {
                struct DropMarker(Arc<AtomicBool>);
                impl Drop for DropMarker {
                    fn drop(&mut self) {
                        self.0.store(true, Ordering::SeqCst);
                    }
                }
                let _marker = DropMarker(dropped);
                std::future::pending().await
            })
        }
    }

    fn one_pixel_png() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([1, 2, 3, 255]),
        ))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
        bytes.into_inner()
    }

    fn headless_runtime() -> EmbedRuntime {
        EmbedRuntime::new("ws", "http://b", Arc::new(NeverFetcher), None)
    }

    fn drain_delivery_within(
        rt: &mut EmbedRuntime,
        async_runtime: &tokio::runtime::Runtime,
        label: &str,
    ) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !rt.drain_deliveries() {
            assert!(
                std::time::Instant::now() < deadline,
                "{label} did not deliver within the bounded test deadline"
            );
            async_runtime.block_on(async {
                tokio::task::yield_now().await;
                tokio::time::sleep(Duration::from_millis(2)).await;
            });
        }
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
            thumbnail_url: format!("http://b/workspaces/ws/assets/{asset_id}/content?tier=thumb"),
            preview_url: format!("http://b/workspaces/ws/assets/{asset_id}/content?tier=preview"),
            poster_url: format!("http://b/workspaces/ws/assets/{asset_id}/content?tier=poster"),
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
        rt.single_cell.lock().unwrap().push((
            rt.generation,
            MediaEmbedKind::Images,
            "a1".to_owned(),
            EmbedResolutionState::Ok(ok_resolved("a1")),
        ));
        assert!(rt.drain_deliveries());
        assert!(matches!(
            rt.resolutions.get("images:a1"),
            Some(EmbedResolutionState::Ok(_))
        ));
    }

    #[test]
    fn drain_keeps_all_concurrent_single_and_sequence_deliveries() {
        let mut rt = headless_runtime();
        let generation = rt.generation;
        {
            let mut single = rt.single_cell.lock().unwrap();
            single.push((
                generation,
                MediaEmbedKind::Images,
                "a1".to_owned(),
                EmbedResolutionState::Ok(ok_resolved("a1")),
            ));
            single.push((
                generation,
                MediaEmbedKind::Images,
                "a2".to_owned(),
                EmbedResolutionState::Ok(ok_resolved("a2")),
            ));
        }
        {
            let mut sequence = rt.sequence_cell.lock().unwrap();
            sequence.push((
                generation,
                MediaEmbedKind::Album,
                "a1,a2".to_owned(),
                Ok(Vec::new()),
            ));
            sequence.push((
                generation,
                MediaEmbedKind::Slideshow,
                "b1,b2".to_owned(),
                Ok(Vec::new()),
            ));
        }

        assert!(rt.drain_deliveries());
        assert!(matches!(
            rt.resolutions.get("images:a1"),
            Some(EmbedResolutionState::Ok(_))
        ));
        assert!(matches!(
            rt.resolutions.get("images:a2"),
            Some(EmbedResolutionState::Ok(_))
        ));
        assert!(matches!(
            rt.sequences.get("album:a1,a2"),
            Some(SequenceState::Items(_))
        ));
        assert!(matches!(
            rt.sequences.get("slideshow:b1,b2"),
            Some(SequenceState::Items(_))
        ));
    }

    #[test]
    fn runtime_absence_does_not_strand_resolving_state() {
        let mut rt = headless_runtime();
        rt.ensure_single(MediaEmbedKind::Images, "a1");
        rt.ensure_sequence(MediaEmbedKind::Album, "a1,a2");
        assert!(
            rt.resolutions.get("a1").is_none(),
            "a runtime attached on a later frame must still be able to start the asset fetch"
        );
        assert!(
            !rt.sequences.contains_key("a1,a2"),
            "a runtime attached on a later frame must still be able to start the sequence fetch"
        );
    }

    #[test]
    fn hung_resolution_becomes_typed_timeout_instead_of_permanent_spinner() {
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("timeout test runtime");
        let mut rt = EmbedRuntime::new(
            "ws",
            "http://b",
            Arc::new(PendingFetcher),
            Some(async_runtime.handle().clone()),
        );
        rt.operation_timeout = Duration::from_millis(5);

        rt.ensure_single(MediaEmbedKind::Images, "hung-asset");
        assert!(matches!(
            rt.resolutions.get("images:hung-asset"),
            Some(EmbedResolutionState::Resolving)
        ));
        drain_delivery_within(&mut rt, &async_runtime, "hung asset timeout");
        assert!(matches!(
            rt.resolutions.get("images:hung-asset"),
            Some(EmbedResolutionState::Err(EmbedError::TimedOut(_)))
        ));
    }

    #[test]
    fn saturated_work_budget_is_inside_metadata_operation_deadline() {
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("budget timeout test runtime");
        let mut rt = EmbedRuntime::new(
            "ws",
            "http://b",
            Arc::new(PendingFetcher),
            Some(async_runtime.handle().clone()),
        );
        rt.operation_timeout = Duration::from_millis(5);
        rt.work_budget = Arc::new(tokio::sync::Semaphore::new(0));

        rt.ensure_single(MediaEmbedKind::Images, "queued-asset");
        drain_delivery_within(&mut rt, &async_runtime, "queued asset timeout");
        assert!(matches!(
            rt.resolutions.get("images:queued-asset"),
            Some(EmbedResolutionState::Err(EmbedError::TimedOut(_)))
        ));
    }

    #[test]
    fn saturated_work_budget_is_inside_pixel_operation_deadline() {
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("pixel budget timeout test runtime");
        let mut rt = EmbedRuntime::new(
            "ws",
            "http://b",
            Arc::new(NeverFetcher),
            Some(async_runtime.handle().clone()),
        );
        rt.operation_timeout = Duration::from_millis(5);
        rt.work_budget = Arc::new(tokio::sync::Semaphore::new(0));

        rt.ensure_image_content(
            MediaEmbedKind::Images,
            "queued-pixels",
            MediaTier::Thumbnail,
        );
        drain_delivery_within(&mut rt, &async_runtime, "queued pixel timeout");
        let key = EmbedRuntime::media_key(
            MediaEmbedKind::Images,
            MediaTier::Thumbnail,
            "queued-pixels",
        );
        assert!(matches!(
            rt.media_errors.get(&key),
            Some(EmbedError::TimedOut(_))
        ));
        assert!(!rt.content_in_flight.contains(&key));
    }

    #[test]
    fn workspace_rebind_clears_caches_and_rejects_stale_delivery() {
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("workspace cancellation test runtime");
        let mut rt = EmbedRuntime::new(
            "ws",
            "http://b",
            Arc::new(PendingFetcher),
            Some(async_runtime.handle().clone()),
        );
        rt.ensure_single(MediaEmbedKind::Images, "hung-old-workspace");
        assert_eq!(rt.pending_tasks.len(), 1);
        rt.resolutions
            .insert("same-id", EmbedResolutionState::Ok(ok_resolved("same-id")));
        let old_generation = rt.generation;
        rt.single_cell.lock().unwrap().push((
            old_generation,
            MediaEmbedKind::Images,
            "late-old-id".to_owned(),
            EmbedResolutionState::Ok(ok_resolved("late-old-id")),
        ));

        rt.workspace_id = "other-workspace".to_owned();
        assert!(!rt.drain_deliveries());
        assert!(rt.resolutions.is_empty());
        assert!(rt.resolutions.get("late-old-id").is_none());
        assert_ne!(rt.generation, old_generation);
        assert!(
            rt.pending_tasks.is_empty(),
            "workspace rebind aborts and releases every task owned by the prior context"
        );
    }

    #[test]
    fn ensure_single_is_idempotent_ac9() {
        // AC-9: a terminal asset is never re-marked / re-spawned. Seed an Ok, then ensure_single
        // must NOT downgrade it back to Resolving.
        let mut rt = headless_runtime();
        rt.resolutions
            .insert("images:a1", EmbedResolutionState::Ok(ok_resolved("a1")));
        rt.ensure_single(MediaEmbedKind::Images, "a1");
        assert!(
            matches!(
                rt.resolutions.get("images:a1"),
                Some(EmbedResolutionState::Ok(_))
            ),
            "AC-9: ensure_single must not re-resolve a terminal asset"
        );
    }

    #[test]
    fn resolution_cache_is_partitioned_by_media_kind() {
        let mut rt = headless_runtime();
        rt.resolutions.insert(
            "shared-id",
            EmbedResolutionState::Ok(ok_resolved("shared-id")),
        );
        rt.resolutions.insert(
            "images:shared-id",
            EmbedResolutionState::Ok(ok_resolved("shared-id")),
        );
        assert!(rt.resolution(MediaEmbedKind::Images, "shared-id").is_some());
        assert!(
            rt.resolution(MediaEmbedKind::Video, "shared-id").is_none(),
            "neither an image resolution nor a legacy raw key may poison a video embed"
        );
    }

    #[test]
    fn retry_evicts_only_selected_kind_and_asset() {
        let mut rt = headless_runtime();
        rt.resolutions.insert(
            "images:failed",
            EmbedResolutionState::Err(EmbedError::NetworkError("offline".to_owned())),
        );
        rt.resolutions.insert(
            "video:failed",
            EmbedResolutionState::Ok(ok_resolved("failed")),
        );
        rt.retry_asset(MediaEmbedKind::Images, "failed");
        assert!(rt.resolutions.get("images:failed").is_none());
        assert!(matches!(
            rt.resolutions.get("video:failed"),
            Some(EmbedResolutionState::Ok(_))
        ));
    }

    #[test]
    fn shared_budget_caps_metadata_and_tier_work_together() {
        let async_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("shared budget test runtime");
        let fetcher = Arc::new(SharedBudgetFetcher {
            active: AtomicUsize::new(0),
            high_water: AtomicUsize::new(0),
            png: one_pixel_png(),
        });
        let fetcher_dyn: Arc<dyn AssetMetadataFetcher> = fetcher.clone();
        let mut rt = EmbedRuntime::new(
            "ws",
            "http://b",
            fetcher_dyn,
            Some(async_runtime.handle().clone()),
        );
        for index in 0..8 {
            let id = format!("metadata-{index}");
            rt.ensure_single(MediaEmbedKind::Images, &id);
        }
        for index in 0..8 {
            let id = format!("pixels-{index}");
            rt.ensure_image_content(MediaEmbedKind::Album, &id, MediaTier::Thumbnail);
        }
        async_runtime.block_on(async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        rt.drain_deliveries();
        assert!(
            fetcher.high_water.load(Ordering::SeqCst) <= MAX_CONCURRENT_RESOLUTIONS,
            "metadata and tier fetches must share the same six-permit budget"
        );
    }

    #[test]
    fn dropping_runtime_aborts_owned_pending_fetch() {
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime removal test");
        let future_dropped = Arc::new(AtomicBool::new(false));
        let mut rt = EmbedRuntime::new(
            "ws",
            "http://b",
            Arc::new(DropTrackedFetcher {
                future_dropped: Arc::clone(&future_dropped),
            }),
            Some(async_runtime.handle().clone()),
        );
        rt.ensure_single(MediaEmbedKind::Images, "removed");
        async_runtime.block_on(tokio::task::yield_now());
        drop(rt);
        async_runtime.block_on(tokio::task::yield_now());
        assert!(
            future_dropped.load(Ordering::SeqCst),
            "removing the owning embed runtime must cancel and drop its pending transport future"
        );
    }

    #[test]
    fn ensure_sequence_is_idempotent() {
        let mut rt = headless_runtime();
        rt.sequences.insert(
            "album:a1,a2".to_owned(),
            SequenceState::Err(EmbedError::EmptyRef),
        );
        rt.ensure_sequence(MediaEmbedKind::Album, "a1,a2");
        assert!(matches!(
            rt.sequences.get("album:a1,a2"),
            Some(SequenceState::Err(_))
        ));
    }
}
