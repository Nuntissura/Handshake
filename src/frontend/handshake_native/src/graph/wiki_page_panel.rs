//! Loom wiki-page projection panel + editable overlay (WP-KERNEL-012 MT-025, cluster E3).
//!
//! ## What this is
//!
//! [`LoomWikiPagePanel`] is the native, AccessKit-addressable port of the React `LoomWikiPagePanel.tsx`
//! (MT-184/185 parity), extended with the editable OVERLAY the MT title names. It loads a
//! [`crate::backend_client::WikiProjection`] from the REAL PostgreSQL/EventLedger backend through the
//! WP-011 [`crate::backend_client::LoomWikiClient`] (`GET /workspaces/{id}/loom/wiki/{projection_id}`)
//! and renders, read-only: the title, a `page_type` badge, a `rebuild_status` badge, the source count,
//! the `rendered_content` text in a scroll area, and a "Stale" footer when the page's
//! `staleness_verdict` is not provably fresh. There is NO Tauri anywhere — the MT step-3 "Tauri
//! intercept" reference is the LEGACY React/webview stack; the KERNEL_BUILDER gate corrected it to
//! `backend_client.rs`, the same typed HTTP client MT-021/022/024 use.
//!
//! ## SPEC-REALISM GATE — `rendered_content` is READ-ONLY; the "Edit overlay" is a REAL annotation
//!
//! The MT-025 KERNEL_BUILDER gate (impl-note 11) + RISK-1/MC-1 demanded a verify-don't-assume pass on
//! whether editing `rendered_content` is actually persisted and rebuild-safe. VERIFIED against
//! `src/backend/handshake_core/src/{api,storage}/loom.rs`: a `LoomWikiProjection.rendered_content` is a
//! DERIVED/GENERATED read-through view — the storage doc says it verbatim: "The rendered wiki markdown
//! (regenerable; never authority)". It is recompiled FROM `source_block_ids` and OVERWRITTEN on every
//! `regenerate`. **There is NO PATCH or PUT route that edits `rendered_content`.** Shipping a fake
//! Edit-save on `rendered_content` would 404 or be silently clobbered on the next rebuild — exactly the
//! silently-broken write the Spec-Realism rule forbids.
//!
//! The backend's REAL, persisted, canonical wiki-page write is an **overlay annotation**
//! (`POST /workspaces/{ws}/loom/wiki/{projection_id}/overlays`, body `{ "annotation" }`), stored in its
//! OWN authority row precisely so (storage doc) "editing it never makes the projection canonical". So
//! this panel's "Edit overlay" mode is exactly that: it lets the operator author an overlay annotation
//! that IS persisted and survives a rebuild, while `rendered_content` stays read-only with a clearly
//! labeled typed limitation ("Read-only projection — edit the source blocks; your note is saved as an
//! overlay"). This honors the MT title ("WikiPageProjectionOverlay" / "editable overlay") AND the
//! Spec-Realism contract: no fake write, a real persisted one.
//!
//! ## Repaint discipline (the MT-015 idle-repaint lesson)
//!
//! The loading spinner animates ONLY while a genuine in-flight fetch is dispatched (`loading=true`); a
//! headless / no-runtime render shows a neutral "Loading wiki page…" / "No backend" state and never
//! enters a perpetual repaint. A kittest drives the panel with `step()`, never `run()`-to-convergence,
//! and the widget requests a repaint ONLY on a frame where a genuine spinner is active.
//!
//! ## Edit-buffer semantics (RISK-4, the MT impl-notes)
//!
//! The overlay edit buffer is initialized at the moment **Edit is clicked** (`begin_edit`), not at load
//! time, so it always starts empty for a NEW annotation (an overlay is additive, never a mutation of the
//! existing projection). `Save` (`request_save` -> the host's `add_overlay` spawn) persists it; on
//! success the host re-fetches the projection and exits edit mode (AC3). `Cancel` (`cancel_edit`)
//! discards the buffer and exits with NO backend call (AC4). A failed save (`apply_save_error`) keeps
//! the buffer and shows the error inline without leaving edit mode (AC5 / PROOF5).
//!
//! ## Large-content cap (RISK-2 / MC-2)
//!
//! `rendered_content` over [`CONTENT_DISPLAY_CAP`] bytes is truncated in the read-only scroll area with a
//! "showing first N of M bytes" notice (an egui `TextEdit`/`Label` over a multi-hundred-KB string lags),
//! so a huge wiki page can never stall the frame. The overlay annotation editor is a fresh small buffer,
//! independently capped at [`OVERLAY_INPUT_CAP`].
//!
//! ## AccessKit (HBR-SWARM) — author_ids exactly as the MT names them
//!
//! - title label: `wiki.title.{projection_id}` (Role::Label)
//! - content area: `wiki.content.{projection_id}` (Role::Document)
//! - edit button: `wiki.edit.{projection_id}` (Role::Button)
//! - edit area: `wiki.edit-area.{projection_id}` (Role::MultilineTextInput)
//! - save button: `wiki.save.{projection_id}` (Role::Button)
//! - cancel button: `wiki.cancel.{projection_id}` (Role::Button)
//! - rebuild button (optional): `wiki.rebuild.{projection_id}` (Role::Button)
//! - retry button (error state): `wiki.retry.{projection_id}` (Role::Button)
//!
//! `{projection_id}` is sanitized to `[a-z0-9-]` via [`crate::project_tree::stable_part`] so a raw id
//! with slashes/colons can never break the AccessKit-tree integrity (the graph/sidebar RISK-3 control).

use egui::accesskit;
use egui::{Sense, Vec2};

use crate::backend_client::WikiProjection;
use crate::theme::HsPalette;

/// Max bytes of `rendered_content` shown in the read-only scroll area before truncation (RISK-2 / MC-2).
/// A long wiki page over this is clipped with a notice; the full content is never loaded into a laggy
/// widget. 50 KB matches the MT RISK-2 cap.
pub const CONTENT_DISPLAY_CAP: usize = 50 * 1024;

/// Max bytes accepted into the overlay-annotation editor (RISK-2 keeps the editable buffer small — an
/// overlay note is a short annotation, not a whole document).
pub const OVERLAY_INPUT_CAP: usize = 50 * 1024;

/// AccessKit author_id prefixes (the full id is `{prefix}{sanitized_projection_id}`). Public so the
/// proof tests address the exact nodes the MT AC7 names.
pub const TITLE_AUTHOR_ID_PREFIX: &str = "wiki.title.";
pub const CONTENT_AUTHOR_ID_PREFIX: &str = "wiki.content.";
pub const EDIT_AUTHOR_ID_PREFIX: &str = "wiki.edit.";
pub const EDIT_AREA_AUTHOR_ID_PREFIX: &str = "wiki.edit-area.";
pub const SAVE_AUTHOR_ID_PREFIX: &str = "wiki.save.";
pub const CANCEL_AUTHOR_ID_PREFIX: &str = "wiki.cancel.";
pub const REBUILD_AUTHOR_ID_PREFIX: &str = "wiki.rebuild.";
pub const RETRY_AUTHOR_ID_PREFIX: &str = "wiki.retry.";

/// The stable AccessKit author_id for the title label: `wiki.title.{sanitized_projection_id}`.
pub fn title_author_id(projection_id: &str) -> String {
    format!(
        "{TITLE_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}
/// The stable AccessKit author_id for the content area: `wiki.content.{sanitized_projection_id}`.
pub fn content_author_id(projection_id: &str) -> String {
    format!(
        "{CONTENT_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}
/// The stable AccessKit author_id for the Edit button: `wiki.edit.{sanitized_projection_id}`.
pub fn edit_author_id(projection_id: &str) -> String {
    format!(
        "{EDIT_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}
/// The stable AccessKit author_id for the overlay edit area: `wiki.edit-area.{sanitized_projection_id}`.
pub fn edit_area_author_id(projection_id: &str) -> String {
    format!(
        "{EDIT_AREA_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}
/// The stable AccessKit author_id for the Save button: `wiki.save.{sanitized_projection_id}`.
pub fn save_author_id(projection_id: &str) -> String {
    format!(
        "{SAVE_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}
/// The stable AccessKit author_id for the Cancel button: `wiki.cancel.{sanitized_projection_id}`.
pub fn cancel_author_id(projection_id: &str) -> String {
    format!(
        "{CANCEL_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}
/// The stable AccessKit author_id for the optional Rebuild button: `wiki.rebuild.{sanitized}`.
pub fn rebuild_author_id(projection_id: &str) -> String {
    format!(
        "{REBUILD_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}
/// The stable AccessKit author_id for the error-state Retry button: `wiki.retry.{sanitized}`.
pub fn retry_author_id(projection_id: &str) -> String {
    format!(
        "{RETRY_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}

/// Decide whether a page's `staleness_verdict` indicates STALE (RISK-5 / MC-5). The React type is
/// `unknown`; here it is a `serde_json::Value`. The rule: a page is FRESH only when the verdict is a
/// non-null object whose `state` field is exactly `"fresh"`. ANY other non-null value — `{"state":
/// "stale"}`, `{"state":"unstamped"}`, `{}` (empty object), a bare string, `true` — reads as STALE
/// (unstamped MUST NEVER render as fresh, the backend LM-PWIKI-008 fail-closed rule). A `null`/absent
/// verdict is treated as NOT stale (no verdict was attached — the read-only `null` baseline). Pure so the
/// proof tests it standalone.
pub fn verdict_is_stale(verdict: &serde_json::Value) -> bool {
    match verdict {
        serde_json::Value::Null => false,
        // An object: fresh ONLY when state == "fresh"; everything else (stale/unstamped/missing) is stale.
        serde_json::Value::Object(_) => {
            verdict.get("state").and_then(|s| s.as_str()) != Some("fresh")
        }
        // Any other non-null JSON (bare string/bool/number/array) is treated as a non-fresh signal.
        _ => true,
    }
}

/// The typed event a [`LoomWikiPagePanel`] interaction produces this frame, for the host to apply. The
/// widget NEVER touches the network (HBR-QUIET); the host owns the backend wiring (load / regenerate /
/// add-overlay) + the event-bus emit after a mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WikiPageEvent {
    /// The Edit button was clicked (AC2): the host may do nothing (state is local) — emitted for
    /// observability / event-bus parity. The buffer was already initialized by [`LoomWikiPagePanel::begin_edit`].
    EditBegan,
    /// The Save button was clicked with a non-empty buffer (AC3): the host runs the verified
    /// `POST /loom/wiki/{id}/overlays { annotation }`, and on success re-fetches the projection and calls
    /// [`LoomWikiPagePanel::finish_save_success`]. `annotation` is the current edit buffer.
    Save { annotation: String },
    /// The Cancel button was clicked (AC4): the buffer was already discarded by [`LoomWikiPagePanel::cancel_edit`];
    /// emitted for observability. The host makes NO backend call.
    Cancel,
    /// The optional Rebuild button was clicked: the host runs `POST /loom/wiki/{id}/regenerate` and
    /// re-renders with the rebuilt page on delivery.
    Rebuild,
    /// The error-state Retry button was pressed (AC8): the host re-fires the load.
    Retry,
}

/// The wiki-page panel state. Held by the host (the pane), mutated in place by [`LoomWikiPagePanel::show`].
#[derive(Debug, Clone)]
pub struct LoomWikiPagePanel {
    pub workspace_id: String,
    pub projection_id: String,
    /// The loaded projection (AC1). `None` while loading or on error.
    pub page: Option<WikiProjection>,
    /// True while the Edit overlay is open (AC2). The read-only view hides; the editor shows.
    pub edit_mode: bool,
    /// The overlay-annotation edit buffer. Initialized EMPTY at Edit-click (RISK-4: an overlay is an
    /// additive annotation, never a mutation of the existing `rendered_content`).
    pub edit_buffer: String,
    /// True while the initial GET is in flight (drives the spinner; the MT-015 idle-repaint rule).
    pub loading: bool,
    /// The load error (AC8): shows the error text + a Retry button.
    pub error: Option<String>,
    /// True while an overlay Save is in flight (drives the Save button's "Saving…" + disables it).
    pub saving: bool,
    /// The save error (AC5 / PROOF5): shown inline below the toolbar; the buffer is PRESERVED and edit
    /// mode is NOT exited.
    pub save_error: Option<String>,
}

impl LoomWikiPagePanel {
    /// A fresh panel for `(workspace_id, projection_id)` with nothing loaded yet (the host calls
    /// `fetch_projection` and sets `loading=true`).
    pub fn new(workspace_id: impl Into<String>, projection_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            projection_id: projection_id.into(),
            page: None,
            edit_mode: false,
            edit_buffer: String::new(),
            loading: false,
            error: None,
            saving: false,
            save_error: None,
        }
    }

    /// Install a loaded projection (AC1), clearing loading/error. If a Save round-trip just completed,
    /// the host calls [`finish_save_success`](Self::finish_save_success) instead.
    pub fn set_page(&mut self, page: WikiProjection) {
        self.page = Some(page);
        self.loading = false;
        self.error = None;
    }

    /// Record a load failure (AC8): drop any stale page, clear loading, surface the error.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.page = None;
        self.loading = false;
        self.error = Some(message.into());
    }

    /// Enter the Edit overlay (AC2). The buffer starts EMPTY (RISK-4: an overlay annotation is additive;
    /// it is NOT a copy of `rendered_content`, which is read-only and regenerable). Clears any prior save
    /// error. A no-op if there is no loaded page (you cannot annotate a page that has not loaded).
    pub fn begin_edit(&mut self) -> bool {
        if self.page.is_none() {
            return false;
        }
        self.edit_mode = true;
        self.edit_buffer.clear();
        self.save_error = None;
        true
    }

    /// Set the edit buffer (the overlay annotation text), capped at [`OVERLAY_INPUT_CAP`] bytes on a char
    /// boundary (RISK-2). Pure so the cap is testable standalone.
    pub fn set_edit_buffer(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.edit_buffer = cap_on_char_boundary(&text, OVERLAY_INPUT_CAP);
    }

    /// Cancel the Edit overlay (AC4): discard the buffer, exit edit mode, clear any save error. NO backend
    /// call is made by the widget OR implied for the host (the Cancel event is observability only).
    pub fn cancel_edit(&mut self) {
        self.edit_mode = false;
        self.edit_buffer.clear();
        self.save_error = None;
    }

    /// Mark a Save as in flight (the host calls this when it dispatches the `add_overlay` spawn). Disables
    /// the Save button + shows "Saving…". Returns the buffer to send, or `None` if the buffer is empty
    /// (an empty overlay annotation is not saved — the backend would reject an empty `annotation`).
    pub fn begin_save(&mut self) -> Option<String> {
        let annotation = self.edit_buffer.trim().to_owned();
        if annotation.is_empty() {
            return None;
        }
        self.saving = true;
        self.save_error = None;
        Some(self.edit_buffer.clone())
    }

    /// Apply a successful Save (AC3): clear saving, exit edit mode, discard the buffer. The host then
    /// re-fetches the projection (a fresh GET shows the page; overlays are a separate read surface). This
    /// is the success counterpart the host calls after the `add_overlay` 2xx + the re-fetch is dispatched.
    pub fn finish_save_success(&mut self) {
        self.saving = false;
        self.edit_mode = false;
        self.edit_buffer.clear();
        self.save_error = None;
    }

    /// Apply a failed Save (AC5 / PROOF5): clear saving, KEEP the buffer, STAY in edit mode, surface the
    /// error inline. The edit area (`wiki.edit-area.*`) remains in the AccessKit tree (PROOF5 assertion).
    pub fn apply_save_error(&mut self, message: impl Into<String>) {
        self.saving = false;
        self.save_error = Some(message.into());
        // edit_mode stays true; edit_buffer is preserved.
    }

    /// True when the loaded page is stale per its `staleness_verdict` (AC6). False when no page is loaded.
    pub fn is_stale(&self) -> bool {
        self.page
            .as_ref()
            .map(|p| verdict_is_stale(&p.staleness_verdict))
            .unwrap_or(false)
    }

    /// Render the panel, returning the typed event this frame (if any) for the host to apply. `palette`
    /// supplies every colour (no hardcoded hex — the architecture-guard invariant). The widget never
    /// blocks on the network.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<WikiPageEvent> {
        // Loading state (AC8): spinner + label. The spinner animates ONLY during a genuine in-flight
        // fetch; request a repaint just for this frame so it advances without a perpetual idle repaint.
        if self.loading {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new());
                ui.colored_label(palette.text_subtle, "Loading wiki page…");
            });
            ui.ctx().request_repaint();
            return None;
        }

        // Error state (AC8): error text + Retry button.
        if let Some(err) = self.error.clone() {
            ui.colored_label(palette.error_text, format!("Wiki page unavailable: {err}"));
            let retry = ui.button("Retry");
            emit_button_accesskit(ui, retry.id, &retry_author_id(&self.projection_id), "Retry");
            if retry.clicked() {
                return Some(WikiPageEvent::Retry);
            }
            return None;
        }

        // No page yet and not loading (e.g. a headless render before a fetch): neutral, non-animating.
        let Some(page) = self.page.clone() else {
            ui.colored_label(palette.text_subtle, "No wiki page loaded.");
            return None;
        };

        if self.edit_mode {
            self.show_edit_overlay(ui, palette, &page)
        } else {
            self.show_read_only(ui, palette, &page)
        }
    }

    /// Render the read-only view (AC1): title, badges, source count, content scroll area, stale footer,
    /// and the Edit / optional Rebuild buttons.
    fn show_read_only(
        &mut self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        page: &WikiProjection,
    ) -> Option<WikiPageEvent> {
        let mut event = None;

        // Title (large bold) — AccessKit Role::Label `wiki.title.{id}`.
        let title_resp = ui.add(
            egui::Label::new(
                egui::RichText::new(&page.title)
                    .heading()
                    .color(palette.text),
            )
            .sense(Sense::hover()),
        );
        emit_label_accesskit(
            ui,
            title_resp.id,
            &title_author_id(&self.projection_id),
            &page.title,
        );

        // Metadata chip row: page_type badge, rebuild_status badge, "(N sources)".
        ui.horizontal(|ui| {
            if let Some(pt) = &page.page_type {
                render_badge(ui, palette, pt, BadgeKind::Neutral);
            }
            render_badge(
                ui,
                palette,
                &page.rebuild_status,
                BadgeKind::for_rebuild_status(&page.rebuild_status),
            );
            ui.colored_label(
                palette.text_subtle,
                format!("({} sources)", page.source_block_ids.len()),
            );
        });

        ui.separator();

        // Rendered content as FORMATTED MARKDOWN (WP-KERNEL-012 MT-059 — resolves the MT-025 deferral that
        // shipped this as a single raw egui::Label printing `rendered_content` verbatim). The read-only
        // view now parses `rendered_content` as CommonMark and paints headings/lists/tables/quotes/code/
        // links via the SHARED `rich_editor::markdown_render` adapter (the SAME styling the MT-012 block
        // renderer uses — one rendering path for wiki pages, reading mode, and the editor). Capped at
        // CONTENT_DISPLAY_CAP bytes BEFORE parsing (RISK-2: a multi-hundred-KB page can never stall the
        // frame). AccessKit Role::Document `wiki.content.{id}` is PRESERVED on the ScrollArea response id
        // (AC7 — downstream swarm selectors depend on it); the markdown blocks render INSIDE it.
        let (shown, truncated) = display_content(&page.rendered_content);
        let content_scroll = egui::ScrollArea::vertical()
            .id_salt(content_author_id(&self.projection_id))
            .max_height(ui.available_height() - 40.0)
            .show(ui, |ui| {
                if shown.trim().is_empty() {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("No rendered wiki content.")
                                .color(palette.text_subtle),
                        )
                        .sense(Sense::hover()),
                    );
                } else {
                    let blocks = crate::rich_editor::markdown_render::parse_markdown(&shown);
                    crate::rich_editor::markdown_render::render_blocks(ui, &blocks);
                }
            });
        // Emit the Document node onto the ScrollArea response id so the content area stays addressable and
        // carries the rendered source as its value (the markdown blocks render inside this node). REUSE the
        // MT-025 `emit_document_accesskit` helper unchanged so the node role (Document), author_id
        // (`wiki.content.{id}`), and value-cap behavior are byte-identical to MT-025 (AC7 — no node
        // removed/renamed; only the Label was swapped for rendered markdown blocks).
        emit_document_accesskit(
            ui,
            content_scroll.id,
            &content_author_id(&self.projection_id),
            &page.rendered_content,
        );
        if truncated {
            ui.colored_label(
                palette.text_subtle,
                format!(
                    "Showing first {} of {} bytes (open the source blocks for the full page).",
                    CONTENT_DISPLAY_CAP,
                    page.rendered_content.len()
                ),
            );
        }

        // Stale footer (AC6): a subtle "Stale" notice when the verdict is not provably fresh.
        if verdict_is_stale(&page.staleness_verdict) {
            ui.colored_label(
                palette.error_text,
                "⚠ Stale — this projection is behind its source blocks. Rebuild to refresh.",
            );
        }

        ui.separator();

        // Action row: Edit overlay + optional Rebuild. The typed read-only limitation is stated inline so
        // a no-context operator/agent understands `rendered_content` is not directly editable (MC-1).
        ui.horizontal(|ui| {
            let edit = ui.button("Edit overlay");
            emit_button_accesskit(
                ui,
                edit.id,
                &edit_author_id(&self.projection_id),
                "Edit overlay",
            );
            if edit.clicked() && self.begin_edit() {
                event = Some(WikiPageEvent::EditBegan);
            }

            let rebuild = ui.button("Rebuild");
            emit_button_accesskit(
                ui,
                rebuild.id,
                &rebuild_author_id(&self.projection_id),
                "Rebuild projection",
            );
            if rebuild.clicked() {
                event = Some(WikiPageEvent::Rebuild);
            }
        });
        ui.colored_label(
            palette.text_subtle,
            "Read-only projection — rendered from the source blocks. \
             Your note is saved as an overlay annotation (the source content is edited via its blocks).",
        );

        event
    }

    /// Render the Edit overlay (AC2): a multiline editor for the overlay annotation + Save / Cancel
    /// toolbar, and the inline save error (AC5) when present.
    fn show_edit_overlay(
        &mut self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        page: &WikiProjection,
    ) -> Option<WikiPageEvent> {
        let mut event = None;

        // Keep the title visible while editing so the operator knows which page they are annotating.
        let title_resp = ui.add(
            egui::Label::new(
                egui::RichText::new(&page.title)
                    .heading()
                    .color(palette.text),
            )
            .sense(Sense::hover()),
        );
        emit_label_accesskit(
            ui,
            title_resp.id,
            &title_author_id(&self.projection_id),
            &page.title,
        );
        ui.colored_label(
            palette.text_subtle,
            "New overlay annotation (saved alongside the page):",
        );

        // Toolbar: Save + Cancel.
        ui.horizontal(|ui| {
            let save_label = if self.saving { "Saving…" } else { "Save" };
            let save = ui.add_enabled(!self.saving, egui::Button::new(save_label));
            emit_button_accesskit(
                ui,
                save.id,
                &save_author_id(&self.projection_id),
                "Save overlay",
            );
            if save.clicked() {
                if let Some(annotation) = self.begin_save() {
                    event = Some(WikiPageEvent::Save { annotation });
                }
            }

            let cancel = ui.button("Cancel");
            emit_button_accesskit(
                ui,
                cancel.id,
                &cancel_author_id(&self.projection_id),
                "Cancel edit",
            );
            if cancel.clicked() {
                self.cancel_edit();
                event = Some(WikiPageEvent::Cancel);
            }
        });

        // Inline save error (AC5 / PROOF5): shown below the toolbar; the buffer is preserved (we never
        // clear edit_buffer on error) and edit mode stays open.
        if let Some(err) = &self.save_error {
            ui.colored_label(palette.error_text, format!("Save failed: {err}"));
        }

        // The multiline annotation editor — AccessKit Role::MultilineTextInput `wiki.edit-area.{id}`.
        let mut buffer = self.edit_buffer.clone();
        let area = egui::ScrollArea::vertical()
            .id_salt(edit_area_author_id(&self.projection_id))
            .max_height(ui.available_height() - 10.0)
            .show(ui, |ui| {
                ui.add_sized(
                    Vec2::new(ui.available_width(), ui.available_height().max(120.0)),
                    egui::TextEdit::multiline(&mut buffer).hint_text("Write an overlay note…"),
                )
            })
            .inner;
        if buffer != self.edit_buffer {
            // Route through set_edit_buffer so the cap is enforced even on direct typing.
            self.set_edit_buffer(buffer);
        }
        emit_multiline_input_accesskit(
            ui,
            area.id,
            &edit_area_author_id(&self.projection_id),
            "Overlay annotation",
        );

        event
    }
}

/// Truncate `content` to at most [`CONTENT_DISPLAY_CAP`] BYTES on a char boundary, returning
/// `(shown, truncated)`. Never splits a multibyte char (no panic on a UTF-8 boundary).
fn display_content(content: &str) -> (String, bool) {
    if content.len() <= CONTENT_DISPLAY_CAP {
        return (content.to_owned(), false);
    }
    let mut end = CONTENT_DISPLAY_CAP;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }
    (content[..end].to_owned(), true)
}

/// Cap `s` to at most `max` BYTES on a char boundary (never splits a multibyte char). Used to bound the
/// overlay-annotation editor buffer (RISK-2).
fn cap_on_char_boundary(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_owned()
}

/// A metadata badge's colour class. `for_rebuild_status` maps the backend `rebuild_status` string to a
/// class (green = fresh/ok, amber/attention = stale/rebuilding, red = failed). Colours come from the
/// shared theme — no hardcoded hex (the architecture-guard invariant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BadgeKind {
    /// Neutral chip (page_type), accent-soft background.
    Neutral,
    /// Green — fresh / ok.
    Ok,
    /// Attention — stale / rebuilding (accent, not red: it is recoverable).
    Attention,
    /// Red — failed.
    Error,
}

impl BadgeKind {
    fn for_rebuild_status(status: &str) -> Self {
        match status.to_ascii_lowercase().as_str() {
            "fresh" | "ok" | "ready" => BadgeKind::Ok,
            "failed" | "error" => BadgeKind::Error,
            // stale / rebuilding / unknown -> attention.
            _ => BadgeKind::Attention,
        }
    }

    /// `(background, foreground)` from the shared palette (no hardcoded hex).
    fn colors(self, palette: &HsPalette) -> (egui::Color32, egui::Color32) {
        match self {
            BadgeKind::Neutral => (palette.accent_soft, palette.accent),
            BadgeKind::Ok => (palette.success_bg, palette.success_text),
            BadgeKind::Attention => (palette.accent_soft, palette.accent),
            BadgeKind::Error => (palette.error_bg, palette.error_text),
        }
    }
}

/// Render a small rounded badge chip with `label`, coloured by `kind` from the shared theme. Cosmetic
/// (no AccessKit node — the meaning is carried by the title/content/badge text the agent reads).
fn render_badge(ui: &mut egui::Ui, palette: &HsPalette, label: &str, kind: BadgeKind) {
    let (bg, fg) = kind.colors(palette);
    egui::Frame::new()
        .fill(bg)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.colored_label(fg, label);
        });
}

// ── AccessKit emit helpers (HBR-SWARM) ───────────────────────────────────────────────────────────────

/// Emit a label's live AccessKit node (Role::Label + author_id + the label text).
fn emit_label_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Label);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
    });
}

/// Emit a button's live AccessKit node (Role::Button + Action::Click + author_id).
fn emit_button_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
    });
}

/// Emit the read-only content area's live AccessKit node (Role::Document + author_id; the rendered text
/// is exposed as the node value so a swarm agent can read the page content by id). The MT AC7 names this
/// `wiki.content.{id}` role=Document — `accesskit::Role::Document` is the field-correct 0.21.1 variant.
fn emit_document_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, content: &str) {
    let author = author_id.to_owned();
    // Cap the exposed value so a huge page does not bloat the AccessKit tree (RISK-2).
    let value = cap_on_char_boundary(content, CONTENT_DISPLAY_CAP);
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Document);
        node.set_author_id(author.clone());
        node.set_value(value.clone());
    });
}

/// Emit the overlay editor's live AccessKit node (Role::MultilineTextInput + author_id). The MT AC7 names
/// the edit area role=MultiLineTextInput; `accesskit::Role::MultilineTextInput` is the field-correct
/// 0.21.1 variant (verified present in the pinned accesskit version).
fn emit_multiline_input_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::MultilineTextInput);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_page() -> WikiProjection {
        // Build via the backend_client parser so the test uses the SAME shape the GET delivers.
        WikiProjection {
            projection_id: "proj-001".to_owned(),
            workspace_id: "ws-test".to_owned(),
            title: "Ownership model".to_owned(),
            source_block_ids: vec!["blk-1".to_owned(), "blk-2".to_owned()],
            rendered_content: "# Ownership\nThe borrow checker enforces aliasing rules.".to_owned(),
            staleness_hash: "h1".to_owned(),
            rebuild_status: "fresh".to_owned(),
            page_type: Some("concept".to_owned()),
            staleness_verdict: serde_json::json!({ "state": "fresh" }),
        }
    }

    fn loaded_panel() -> LoomWikiPagePanel {
        let mut p = LoomWikiPagePanel::new("ws-test", "proj-001");
        p.set_page(seeded_page());
        p
    }

    /// PROOF1: begin_edit initializes the buffer EMPTY (an overlay is additive, not a copy of
    /// rendered_content — RISK-4), enters edit mode, clears prior save error.
    #[test]
    fn begin_edit_initializes_empty_buffer() {
        let mut p = loaded_panel();
        p.save_error = Some("old error".to_owned());
        assert!(p.begin_edit(), "begin_edit succeeds with a loaded page");
        assert!(p.edit_mode, "edit mode entered");
        assert_eq!(
            p.edit_buffer, "",
            "buffer starts empty (overlay is additive, not a content copy)"
        );
        assert!(
            p.save_error.is_none(),
            "prior save error cleared on a fresh edit"
        );
    }

    /// begin_edit is a no-op without a loaded page (you cannot annotate a page that has not loaded).
    #[test]
    fn begin_edit_noop_without_page() {
        let mut p = LoomWikiPagePanel::new("ws", "proj-001");
        assert!(!p.begin_edit(), "begin_edit fails with no page");
        assert!(!p.edit_mode);
    }

    /// PROOF1: cancel_edit discards the buffer and exits edit mode with NO mutation of the page or any
    /// backend implication (AC4 — cancel-no-mutation).
    #[test]
    fn cancel_edit_discards_buffer_no_mutation() {
        let mut p = loaded_panel();
        p.begin_edit();
        p.set_edit_buffer("THROWAWAY");
        let page_before = p.page.clone();
        p.cancel_edit();
        assert!(!p.edit_mode, "edit mode exited");
        assert_eq!(p.edit_buffer, "", "buffer discarded");
        assert_eq!(
            p.page, page_before,
            "the page is UNCHANGED by cancel (cancel-no-mutation)"
        );
    }

    /// AC5 / PROOF5: a failed save KEEPS the buffer and STAYS in edit mode with the error surfaced.
    #[test]
    fn save_error_preserves_buffer_and_edit_mode() {
        let mut p = loaded_panel();
        p.begin_edit();
        p.set_edit_buffer("important note");
        let sent = p
            .begin_save()
            .expect("non-empty buffer yields an annotation to send");
        assert_eq!(sent, "important note");
        assert!(p.saving, "save marked in flight");
        p.apply_save_error("500 Internal Server Error");
        assert!(!p.saving, "saving cleared after the error");
        assert!(p.edit_mode, "AC5: edit mode is NOT exited on a save error");
        assert_eq!(
            p.edit_buffer, "important note",
            "AC5: the buffer is PRESERVED on a save error"
        );
        assert_eq!(p.save_error.as_deref(), Some("500 Internal Server Error"));
    }

    /// AC3: a successful save exits edit mode and discards the buffer (the host then re-fetches).
    #[test]
    fn save_success_exits_edit_and_clears_buffer() {
        let mut p = loaded_panel();
        p.begin_edit();
        p.set_edit_buffer("note");
        p.begin_save();
        p.finish_save_success();
        assert!(!p.edit_mode);
        assert_eq!(p.edit_buffer, "");
        assert!(p.save_error.is_none());
    }

    /// begin_save refuses an empty/whitespace buffer (the backend rejects an empty `annotation`).
    #[test]
    fn begin_save_refuses_empty_buffer() {
        let mut p = loaded_panel();
        p.begin_edit();
        p.set_edit_buffer("   \n  ");
        assert!(
            p.begin_save().is_none(),
            "whitespace-only buffer is not saved"
        );
        assert!(!p.saving);
    }

    /// AC6 / RISK-5 / MC-5: verdict_is_stale — fresh only when state=="fresh"; everything else stale;
    /// null is not stale.
    #[test]
    fn verdict_staleness_rule() {
        assert!(
            !verdict_is_stale(&serde_json::json!({ "state": "fresh" })),
            "fresh is not stale"
        );
        assert!(
            verdict_is_stale(&serde_json::json!({ "state": "stale" })),
            "stale state is stale"
        );
        assert!(
            verdict_is_stale(&serde_json::json!({ "state": "unstamped" })),
            "unstamped is stale"
        );
        assert!(
            verdict_is_stale(&serde_json::json!({})),
            "empty object (no state) is stale"
        );
        assert!(
            !verdict_is_stale(&serde_json::Value::Null),
            "null verdict is not stale"
        );
        assert!(
            verdict_is_stale(&serde_json::json!("anything")),
            "a bare non-null value is stale"
        );
    }

    /// is_stale reflects the loaded page's verdict (AC6).
    #[test]
    fn is_stale_reflects_page_verdict() {
        let mut p = loaded_panel();
        assert!(!p.is_stale(), "the fresh seeded page is not stale");
        if let Some(page) = p.page.as_mut() {
            page.staleness_verdict = serde_json::json!({ "state": "stale" });
        }
        assert!(p.is_stale(), "a stale verdict makes is_stale true");
    }

    /// RISK-2 / MC-2: the overlay buffer is capped at OVERLAY_INPUT_CAP bytes on a char boundary.
    #[test]
    fn edit_buffer_is_capped() {
        let mut p = loaded_panel();
        p.begin_edit();
        let huge = "x".repeat(OVERLAY_INPUT_CAP + 5000);
        p.set_edit_buffer(huge);
        assert_eq!(
            p.edit_buffer.len(),
            OVERLAY_INPUT_CAP,
            "buffer capped at OVERLAY_INPUT_CAP bytes"
        );
    }

    /// RISK-2: display_content truncates a huge page on a char boundary with the truncated flag set.
    #[test]
    fn display_content_truncates_huge_page() {
        let small = "short content";
        let (shown, trunc) = display_content(small);
        assert_eq!(shown, small);
        assert!(!trunc);

        let huge = "y".repeat(CONTENT_DISPLAY_CAP + 1000);
        let (shown, trunc) = display_content(&huge);
        assert_eq!(shown.len(), CONTENT_DISPLAY_CAP, "content shown is capped");
        assert!(trunc, "the truncated flag is set");
    }

    /// The author_id helpers produce the exact MT AC7 ids for a clean projection id.
    #[test]
    fn author_ids_match_contract() {
        assert_eq!(title_author_id("proj-001"), "wiki.title.proj-001");
        assert_eq!(content_author_id("proj-001"), "wiki.content.proj-001");
        assert_eq!(edit_author_id("proj-001"), "wiki.edit.proj-001");
        assert_eq!(edit_area_author_id("proj-001"), "wiki.edit-area.proj-001");
        assert_eq!(save_author_id("proj-001"), "wiki.save.proj-001");
        assert_eq!(cancel_author_id("proj-001"), "wiki.cancel.proj-001");
    }
}
