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
//! - metadata summary: `wiki.metadata.{projection_id}` (Role::Label)
//! - stale notice: `wiki.stale.{projection_id}` (Role::Label; fail-closed when verdict is missing)
//! - load error: `wiki.error.{projection_id}` (Role::Label)
//!
//! `{projection_id}` is sanitized to `[a-z0-9-]` via [`crate::project_tree::stable_part`] so a raw id
//! with slashes/colons can never break the AccessKit-tree integrity (the graph/sidebar RISK-3 control).

use egui::accesskit;
use egui::{Sense, Vec2};
use sha2::{Digest, Sha256};

use crate::backend_client::{WikiOverlay, WikiProjection};
use crate::mcp::action::{
    accesskit_string_set_value, serialize_observer_click_applied, serialize_observer_click_failure,
    serialize_observer_click_state, serialize_observer_click_target, ClickCompletionState,
};
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
pub const METADATA_AUTHOR_ID_PREFIX: &str = "wiki.metadata.";
pub const STALE_AUTHOR_ID_PREFIX: &str = "wiki.stale.";
pub const ERROR_AUTHOR_ID_PREFIX: &str = "wiki.error.";
pub const OVERLAYS_AUTHOR_ID_PREFIX: &str = "wiki.overlays.";
pub const OVERLAY_AUTHOR_ID_PREFIX: &str = "wiki.overlay.";
pub const ACTION_STATUS_AUTHOR_ID_PREFIX: &str = "wiki.action-status.";
const ACTION_DISPATCHED_AUTHOR_ID_PREFIX: &str = "wiki.action-dispatched.";
const WIKI_ACTION_EFFECT: &str = "wiki-overlay-action";
const WIKI_ACTION_DETAIL_SCHEMA: &str = "handshake.wiki-action-terminal/v1";

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

/// The stable AccessKit author_id for the projection metadata summary.
pub fn metadata_author_id(projection_id: &str) -> String {
    format!(
        "{METADATA_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}

/// The stable AccessKit author_id for the fail-closed stale notice.
pub fn stale_author_id(projection_id: &str) -> String {
    format!(
        "{STALE_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}

/// The stable AccessKit author_id for the load-error text paired with Retry.
pub fn error_author_id(projection_id: &str) -> String {
    format!(
        "{ERROR_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}

pub fn overlays_author_id(projection_id: &str) -> String {
    format!(
        "{OVERLAYS_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}

pub fn overlay_author_id(overlay_id: &str) -> String {
    format!(
        "{OVERLAY_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(overlay_id)
    )
}

pub fn action_status_author_id(projection_id: &str) -> String {
    format!(
        "{ACTION_STATUS_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}

fn action_dispatched_author_id(projection_id: &str) -> String {
    format!(
        "{ACTION_DISPATCHED_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(projection_id)
    )
}

/// Decide whether a page's `staleness_verdict` indicates STALE (RISK-5 / MC-5). The React type is
/// `unknown`; here it is a `serde_json::Value`. The rule: a page is FRESH only when the verdict is a
/// non-null object whose `state` field is exactly `"fresh"`. ANY other value — `{"state": "stale"}`,
/// `{"state":"unstamped"}`, `{}` (empty object), a bare string, `true`, or `null`/absent — reads as
/// STALE. LM-PWIKI-008 is fail-closed: a page without a verdict is forbidden to render as fresh. Pure so
/// the proof tests it standalone.
pub fn verdict_is_stale(verdict: &serde_json::Value) -> bool {
    match verdict {
        serde_json::Value::Null => true,
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
    Save {
        action_generation: u64,
        annotation: String,
    },
    /// The Cancel button was clicked (AC4): the buffer was already discarded by [`LoomWikiPagePanel::cancel_edit`];
    /// emitted for observability. The host makes NO backend call.
    Cancel,
    /// The optional Rebuild button was clicked: the host runs `POST /loom/wiki/{id}/regenerate` and
    /// re-renders with the rebuilt page on delivery.
    Rebuild,
    /// The error-state Retry button was pressed (AC8): the host re-fires the load.
    Retry,
    /// The overlay POST already succeeded but its follow-up GET failed. Retry only that GET; emitting
    /// another Save here would create a duplicate authority overlay.
    RetryReloadAfterSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WikiObserverPhase {
    Ready,
    Pending,
    Applied,
    Failed,
}

#[derive(Debug, Clone)]
struct WikiActionObserver {
    generation: u64,
    phase: WikiObserverPhase,
    context: String,
    pending_target: Option<String>,
    semantic_value: Option<String>,
    terminal_error: Option<String>,
    terminal_detail: Option<String>,
}

impl Default for WikiActionObserver {
    fn default() -> Self {
        Self {
            generation: 0,
            phase: WikiObserverPhase::Ready,
            context: "wiki-unbound".to_owned(),
            pending_target: None,
            semantic_value: None,
            terminal_error: None,
            terminal_detail: None,
        }
    }
}

impl WikiActionObserver {
    fn prepare_frame(&mut self, context: String) {
        // Terminal results are durable observation records, not transient render state. Capture and
        // navigation passes may render the pane arbitrarily many times before ActionChannel reads
        // the observer. Only an authoritative context change invalidates that record; `begin` below
        // supersedes it when the operator starts the next action generation.
        if matches!(
            self.phase,
            WikiObserverPhase::Applied | WikiObserverPhase::Failed
        ) && self.context != context
        {
            self.phase = WikiObserverPhase::Ready;
            self.pending_target = None;
            self.semantic_value = None;
            self.terminal_error = None;
            self.terminal_detail = None;
        }
        if self.phase == WikiObserverPhase::Ready {
            self.context = context;
        }
    }

    fn next_generation(&self) -> u64 {
        self.generation.wrapping_add(1)
    }

    fn target_value(&self, observer_author_id: &str, semantic: &str) -> Option<String> {
        (self.phase != WikiObserverPhase::Pending).then(|| {
            serialize_observer_click_target(
                WIKI_ACTION_EFFECT,
                &self.context,
                self.generation,
                observer_author_id,
                semantic,
            )
        })?
    }

    fn begin(&mut self, target: String, semantic_value: String) -> Option<u64> {
        if self.phase == WikiObserverPhase::Pending {
            return None;
        }
        self.generation = self.next_generation();
        self.phase = WikiObserverPhase::Pending;
        self.pending_target = Some(target);
        self.semantic_value = Some(semantic_value);
        self.terminal_error = None;
        self.terminal_detail = None;
        Some(self.generation)
    }

    fn applied(&mut self, generation: u64, detail: String) -> bool {
        if self.phase != WikiObserverPhase::Pending || self.generation != generation {
            return false;
        }
        self.phase = WikiObserverPhase::Applied;
        self.terminal_error = None;
        self.terminal_detail = Some(detail);
        true
    }

    fn failed(&mut self, generation: u64, error: String, detail: String) -> bool {
        if self.phase != WikiObserverPhase::Pending || self.generation != generation {
            return false;
        }
        self.phase = WikiObserverPhase::Failed;
        self.terminal_error = Some(error);
        self.terminal_detail = Some(detail);
        true
    }

    fn serialized(&self) -> Option<String> {
        match self.phase {
            WikiObserverPhase::Ready => serialize_observer_click_state(
                WIKI_ACTION_EFFECT,
                &self.context,
                self.generation,
                ClickCompletionState::Ready,
                None,
                None,
            ),
            WikiObserverPhase::Pending => serialize_observer_click_state(
                WIKI_ACTION_EFFECT,
                &self.context,
                self.generation,
                ClickCompletionState::Pending,
                self.pending_target.as_deref(),
                self.semantic_value.as_deref(),
            ),
            WikiObserverPhase::Applied => serialize_observer_click_applied(
                WIKI_ACTION_EFFECT,
                &self.context,
                self.generation,
                self.pending_target.as_deref()?,
                self.semantic_value.as_deref()?,
                self.terminal_detail.as_deref()?,
            ),
            WikiObserverPhase::Failed => serialize_observer_click_failure(
                WIKI_ACTION_EFFECT,
                &self.context,
                self.generation,
                self.pending_target.as_deref()?,
                self.semantic_value.as_deref()?,
                self.terminal_error.as_deref()?,
                self.terminal_detail.as_deref(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WikiSourceSnapshot {
    projection_revision: String,
    staleness_hash: String,
    content_sha256: String,
}

impl WikiSourceSnapshot {
    fn from_page(page: &WikiProjection) -> Self {
        Self {
            projection_revision: page.updated_at.clone(),
            staleness_hash: page.staleness_hash.clone(),
            content_sha256: sha256_hex(page.rendered_content.as_bytes()),
        }
    }
}

#[derive(Debug, Clone)]
struct PendingWikiSave {
    action_generation: u64,
    edit_mode_generation: u64,
    draft_identity: String,
    draft_sha256: String,
    annotation: String,
    source: WikiSourceSnapshot,
    persisted_overlay: Option<WikiOverlay>,
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
    /// The overlay POST succeeded and only the projection/overlay reload remains. While true, Save and
    /// Cancel stay disabled: Save would duplicate the authority row and Cancel cannot undo it.
    pub saved_awaiting_reload: bool,
    /// A rebuild/action error that must not evict the last-good projection.
    pub action_error: Option<String>,
    /// Pane-instance generation assigned by the authoritative mount. It prevents a late result from
    /// an earlier A -> B -> A binding from sharing an observer context with the current pane.
    pane_generation: u64,
    edit_mode_generation: u64,
    action_observer: WikiActionObserver,
    pending_save: Option<PendingWikiSave>,
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
            saved_awaiting_reload: false,
            action_error: None,
            pane_generation: 0,
            edit_mode_generation: 0,
            action_observer: WikiActionObserver::default(),
            pending_save: None,
        }
    }

    /// Bind the panel to the mount's exact pane instance before its first render.
    pub fn bind_pane_generation(&mut self, pane_generation: u64) {
        self.pane_generation = pane_generation;
        self.action_observer = WikiActionObserver::default();
        self.pending_save = None;
    }

    /// Install a loaded projection (AC1), clearing loading/error. If a Save round-trip just completed,
    /// the host calls [`finish_save_success`](Self::finish_save_success) instead.
    pub fn set_page(&mut self, page: WikiProjection) {
        self.page = Some(page);
        self.loading = false;
        self.error = None;
        self.action_error = None;
    }

    /// Record a load failure (AC8): drop any stale page, clear loading, surface the error.
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.page = None;
        self.loading = false;
        self.error = Some(message.into());
    }

    /// Surface a regenerate failure while retaining the last-good page.
    pub fn apply_rebuild_error(&mut self, message: impl Into<String>) {
        self.loading = false;
        self.action_error = Some(message.into());
    }

    /// Enter the Edit overlay (AC2). The buffer starts EMPTY (RISK-4: an overlay annotation is additive;
    /// it is NOT a copy of `rendered_content`, which is read-only and regenerable). Clears any prior save
    /// error. A no-op if there is no loaded page (you cannot annotate a page that has not loaded).
    pub fn begin_edit(&mut self) -> bool {
        if self.page.is_none() || self.saving || self.saved_awaiting_reload {
            return false;
        }
        self.edit_mode = true;
        self.edit_mode_generation = self.edit_mode_generation.wrapping_add(1);
        self.edit_buffer.clear();
        self.save_error = None;
        true
    }

    /// Set the edit buffer (the overlay annotation text), capped at [`OVERLAY_INPUT_CAP`] bytes on a char
    /// boundary (RISK-2). Pure so the cap is testable standalone.
    pub fn set_edit_buffer(&mut self, text: impl Into<String>) {
        if self.saving || self.saved_awaiting_reload {
            return;
        }
        let text = text.into();
        self.edit_buffer = cap_on_char_boundary(&text, OVERLAY_INPUT_CAP);
    }

    /// Cancel the Edit overlay (AC4): discard the buffer, exit edit mode, clear any save error. NO backend
    /// call is made by the widget OR implied for the host (the Cancel event is observability only).
    pub fn cancel_edit(&mut self) -> bool {
        if self.saving || self.saved_awaiting_reload {
            return false;
        }
        self.edit_mode = false;
        self.edit_buffer.clear();
        self.save_error = None;
        true
    }

    /// Mark a Save as in flight (the host calls this when it dispatches the `add_overlay` spawn). Disables
    /// the Save button + shows "Saving…". Returns the buffer to send, or `None` if the buffer is empty
    /// (an empty overlay annotation is not saved — the backend would reject an empty `annotation`).
    pub fn begin_save(&mut self) -> Option<String> {
        let annotation = self.edit_buffer.trim().to_owned();
        if annotation.is_empty() || self.saving || self.saved_awaiting_reload {
            return None;
        }
        self.saving = true;
        self.save_error = None;
        Some(self.edit_buffer.clone())
    }

    #[cfg(any(test, feature = "integration"))]
    fn source_snapshot(&self) -> Option<WikiSourceSnapshot> {
        self.page.as_ref().map(WikiSourceSnapshot::from_page)
    }

    fn draft_identity_for(&self, edit_mode_generation: u64) -> String {
        format!(
            "wiki-draft:{}:{}:{}:{}",
            crate::project_tree::stable_part(&self.workspace_id),
            crate::project_tree::stable_part(&self.projection_id),
            self.pane_generation,
            edit_mode_generation
        )
    }

    fn action_context(&self) -> String {
        let source_revision = self
            .page
            .as_ref()
            .map(|page| page.updated_at.as_str())
            .unwrap_or("unloaded");
        format!(
            "wiki/{}/{}/pane-{}/source-{}",
            crate::project_tree::stable_part(&self.workspace_id),
            crate::project_tree::stable_part(&self.projection_id),
            self.pane_generation,
            crate::project_tree::stable_part(source_revision)
        )
    }

    fn action_semantic(
        &self,
        action: &str,
        action_generation: u64,
        edit_mode_generation: u64,
        draft_identity: &str,
        draft_sha256: &str,
        source: &WikiSourceSnapshot,
    ) -> String {
        serde_json::json!({
            "action": action,
            "action_generation": action_generation,
            "draft_identity": draft_identity,
            "draft_sha256": draft_sha256,
            "edit_mode_generation": edit_mode_generation,
            "pane_generation": self.pane_generation,
            "projection_id": self.projection_id,
            "source_content_sha256": source.content_sha256,
            "source_projection_revision": source.projection_revision,
            "source_staleness_hash": source.staleness_hash,
            "workspace_id": self.workspace_id,
        })
        .to_string()
    }

    fn terminal_detail(
        &self,
        action: &str,
        action_generation: u64,
        edit_mode_generation: u64,
        draft_identity: &str,
        draft_sha256: &str,
        source: &WikiSourceSnapshot,
        outcome: &str,
        write_count: u64,
        overlay: Option<&WikiOverlay>,
        extra: serde_json::Value,
    ) -> String {
        serde_json::json!({
            "action": action,
            "action_generation": action_generation,
            "draft_identity": draft_identity,
            "draft_sha256": draft_sha256,
            "edit_mode_generation": edit_mode_generation,
            "extra": extra,
            "no_write": write_count == 0,
            "outcome": outcome,
            "overlay_created_at": overlay.map(|value| value.created_at.as_str()),
            "overlay_id": overlay.map(|value| value.overlay_id.as_str()),
            "overlay_persisted_revision": overlay.map(|value| value.updated_at.as_str()),
            "overlay_readback_revision": overlay.map(|value| value.updated_at.as_str()),
            "pane_generation": self.pane_generation,
            "projection_id": self.projection_id,
            "schema": WIKI_ACTION_DETAIL_SCHEMA,
            "source_content_sha256": source.content_sha256,
            "source_projection_revision": source.projection_revision,
            "source_staleness_hash": source.staleness_hash,
            "workspace_id": self.workspace_id,
            "write_count": write_count,
        })
        .to_string()
    }

    fn edit_target_value(&self, source: &WikiSourceSnapshot) -> Option<String> {
        let action_generation = self.action_observer.next_generation();
        let edit_generation = self.edit_mode_generation.wrapping_add(1);
        let draft_identity = self.draft_identity_for(edit_generation);
        let semantic = self.action_semantic(
            "edit",
            action_generation,
            edit_generation,
            &draft_identity,
            &sha256_hex(b""),
            source,
        );
        self.action_observer
            .target_value(&action_status_author_id(&self.projection_id), &semantic)
    }

    fn begin_edit_observed(&mut self, source: &WikiSourceSnapshot) -> bool {
        let action_generation = self.action_observer.next_generation();
        let edit_generation = self.edit_mode_generation.wrapping_add(1);
        let draft_identity = self.draft_identity_for(edit_generation);
        let draft_sha256 = sha256_hex(b"");
        let semantic = self.action_semantic(
            "edit",
            action_generation,
            edit_generation,
            &draft_identity,
            &draft_sha256,
            source,
        );
        let target = edit_author_id(&self.projection_id);
        if self.action_observer.begin(target, semantic).is_none() || !self.begin_edit() {
            return false;
        }
        let detail = self.terminal_detail(
            "edit",
            action_generation,
            self.edit_mode_generation,
            &draft_identity,
            &draft_sha256,
            source,
            "applied",
            0,
            None,
            serde_json::json!({"draft_initialized": true, "edit_open": true}),
        );
        self.action_observer.applied(action_generation, detail)
    }

    fn cancel_target_value(&self, source: &WikiSourceSnapshot) -> Option<String> {
        let action_generation = self.action_observer.next_generation();
        let draft_identity = self.draft_identity_for(self.edit_mode_generation);
        let draft_sha256 = sha256_hex(self.edit_buffer.as_bytes());
        let semantic = self.action_semantic(
            "cancel",
            action_generation,
            self.edit_mode_generation,
            &draft_identity,
            &draft_sha256,
            source,
        );
        self.action_observer
            .target_value(&action_status_author_id(&self.projection_id), &semantic)
    }

    fn cancel_edit_observed(&mut self, source: &WikiSourceSnapshot) -> bool {
        let action_generation = self.action_observer.next_generation();
        let edit_generation = self.edit_mode_generation;
        let draft_identity = self.draft_identity_for(edit_generation);
        let draft_sha256 = sha256_hex(self.edit_buffer.as_bytes());
        let semantic = self.action_semantic(
            "cancel",
            action_generation,
            edit_generation,
            &draft_identity,
            &draft_sha256,
            source,
        );
        let target = cancel_author_id(&self.projection_id);
        if self.action_observer.begin(target, semantic).is_none() || !self.cancel_edit() {
            return false;
        }
        let detail = self.terminal_detail(
            "cancel",
            action_generation,
            edit_generation,
            &draft_identity,
            &draft_sha256,
            source,
            "applied",
            0,
            None,
            serde_json::json!({
                "draft_discarded": true,
                "edit_closed": true,
                "original_source_authoritative": true,
            }),
        );
        self.action_observer.applied(action_generation, detail)
    }

    fn save_target_value(&self, source: &WikiSourceSnapshot) -> Option<String> {
        let action_generation = self.action_observer.next_generation();
        let draft_identity = self.draft_identity_for(self.edit_mode_generation);
        let draft_sha256 = sha256_hex(self.edit_buffer.as_bytes());
        let semantic = self.action_semantic(
            "save",
            action_generation,
            self.edit_mode_generation,
            &draft_identity,
            &draft_sha256,
            source,
        );
        self.action_observer
            .target_value(&action_status_author_id(&self.projection_id), &semantic)
    }

    fn begin_save_observed(&mut self, source: &WikiSourceSnapshot) -> Option<(u64, String)> {
        let action_generation = self.action_observer.next_generation();
        let edit_generation = self.edit_mode_generation;
        let draft_identity = self.draft_identity_for(edit_generation);
        let draft_sha256 = sha256_hex(self.edit_buffer.as_bytes());
        let semantic = self.action_semantic(
            "save",
            action_generation,
            edit_generation,
            &draft_identity,
            &draft_sha256,
            source,
        );
        let target = save_author_id(&self.projection_id);
        self.action_observer.begin(target, semantic)?;
        let annotation = self.begin_save()?;
        self.pending_save = Some(PendingWikiSave {
            action_generation,
            edit_mode_generation: edit_generation,
            draft_identity,
            draft_sha256,
            annotation: annotation.clone(),
            source: source.clone(),
            persisted_overlay: None,
        });
        Some((action_generation, annotation))
    }

    /// Integration-proof seam: starts the same observed Save transition the rendered Save button
    /// uses, without fabricating a backend result.
    #[cfg(any(test, feature = "integration"))]
    pub fn begin_observed_save_for_test(&mut self) -> Option<(u64, String)> {
        let source = self.source_snapshot()?;
        self.action_observer.prepare_frame(self.action_context());
        self.begin_save_observed(&source)
    }

    /// Apply a successful Save (AC3): clear saving, exit edit mode, discard the buffer. The host then
    /// re-fetches the projection (a fresh GET shows the page; overlays are a separate read surface). This
    /// is the success counterpart the host calls after the `add_overlay` 2xx + the re-fetch is dispatched.
    pub fn finish_save_success(&mut self) {
        self.saving = false;
        self.saved_awaiting_reload = false;
        self.edit_mode = false;
        self.edit_buffer.clear();
        self.save_error = None;
    }

    /// Apply a failed Save (AC5 / PROOF5): clear saving, KEEP the buffer, STAY in edit mode, surface the
    /// error inline. The edit area (`wiki.edit-area.*`) remains in the AccessKit tree (PROOF5 assertion).
    pub fn apply_save_error(&mut self, message: impl Into<String>) {
        self.saving = false;
        self.saved_awaiting_reload = false;
        self.save_error = Some(message.into());
        // edit_mode stays true; edit_buffer is preserved.
    }

    /// Complete the exact in-flight Save with a transport/backend failure. A stale generation is
    /// ignored and can never reject the current pane's action.
    pub fn apply_save_transport_error(
        &mut self,
        action_generation: u64,
        message: impl Into<String>,
    ) -> bool {
        let Some(pending) = self
            .pending_save
            .as_ref()
            .filter(|pending| pending.action_generation == action_generation)
            .cloned()
        else {
            return false;
        };
        let message = bounded_terminal_error(&message.into());
        self.apply_save_error(message.clone());
        let detail = self.terminal_detail(
            "save",
            action_generation,
            pending.edit_mode_generation,
            &pending.draft_identity,
            &pending.draft_sha256,
            &pending.source,
            "failed",
            0,
            None,
            serde_json::json!({
                "draft_retained": true,
                "edit_open": true,
                "error_kind": "wiki_save_transport",
            }),
        );
        self.pending_save = None;
        self.action_observer.failed(
            action_generation,
            format!("wiki_save_transport: {message}"),
            detail,
        )
    }

    /// The POST has committed one canonical overlay. Lock the editor while the host performs the
    /// follow-up GET; this state is intentionally distinct from a write failure.
    pub fn mark_overlay_saved_awaiting_reload(&mut self) {
        self.saving = true;
        self.saved_awaiting_reload = true;
        self.save_error = None;
    }

    /// Bind the canonical POST receipt to the exact pending Save before issuing the follow-up GET.
    pub fn mark_persisted_overlay_awaiting_readback(
        &mut self,
        action_generation: u64,
        overlay: WikiOverlay,
    ) -> bool {
        let Some(pending) = self
            .pending_save
            .as_mut()
            .filter(|pending| pending.action_generation == action_generation)
        else {
            return false;
        };
        if overlay.workspace_id != self.workspace_id
            || overlay.projection_id != self.projection_id
            || overlay.annotation != pending.annotation
        {
            return false;
        }
        pending.persisted_overlay = Some(overlay);
        self.mark_overlay_saved_awaiting_reload();
        true
    }

    /// The POST succeeded but the follow-up GET failed. Preserve the buffer for context, but expose only
    /// Retry Reload. Save and Cancel remain disabled because the row already exists and Cancel cannot
    /// remove it.
    pub fn apply_reload_after_save_error(&mut self, message: impl Into<String>) {
        self.saving = false;
        self.saved_awaiting_reload = true;
        self.save_error = Some(message.into());
    }

    /// Complete a failed persisted-overlay readback while preserving the exact draft and edit mode.
    pub fn apply_save_readback_error(
        &mut self,
        action_generation: u64,
        message: impl Into<String>,
    ) -> bool {
        let Some(pending) = self
            .pending_save
            .as_ref()
            .filter(|pending| pending.action_generation == action_generation)
            .cloned()
        else {
            return false;
        };
        let message = bounded_terminal_error(&message.into());
        self.apply_reload_after_save_error(message.clone());
        let detail = self.terminal_detail(
            "save",
            action_generation,
            pending.edit_mode_generation,
            &pending.draft_identity,
            &pending.draft_sha256,
            &pending.source,
            "failed",
            u64::from(pending.persisted_overlay.is_some()),
            pending.persisted_overlay.as_ref(),
            serde_json::json!({
                "draft_retained": true,
                "edit_open": true,
                "error_kind": "wiki_save_readback",
            }),
        );
        self.action_observer.failed(
            action_generation,
            format!("wiki_save_readback: {message}"),
            detail,
        )
    }

    /// Accept Save only after the identity-current GET contains the exact overlay returned by POST and
    /// the source projection revision/hash/content remain unchanged. Any mismatch is a typed conflict;
    /// the original loaded source and exact draft remain visible and authoritative.
    pub fn complete_save_readback(&mut self, action_generation: u64, page: WikiProjection) -> bool {
        let Some(pending) = self
            .pending_save
            .as_ref()
            .filter(|pending| pending.action_generation == action_generation)
            .cloned()
        else {
            return false;
        };
        let Some(persisted) = pending.persisted_overlay.as_ref() else {
            return self.apply_save_readback_error(
                action_generation,
                "readback arrived before a canonical POST receipt",
            );
        };
        let readback_source = WikiSourceSnapshot::from_page(&page);
        let source_unchanged = readback_source == pending.source;
        let overlay_read_back = page.overlays.iter().any(|overlay| overlay == persisted);
        if !source_unchanged || !overlay_read_back {
            let conflict = if !source_unchanged {
                "source projection revision/hash/content changed during overlay save"
            } else {
                "persisted overlay receipt was absent or changed in canonical readback"
            };
            self.apply_reload_after_save_error(conflict);
            let detail = self.terminal_detail(
                "save",
                action_generation,
                pending.edit_mode_generation,
                &pending.draft_identity,
                &pending.draft_sha256,
                &pending.source,
                "conflict",
                1,
                Some(persisted),
                serde_json::json!({
                    "draft_retained": true,
                    "edit_open": true,
                    "error_kind": "wiki_save_conflict",
                    "readback_source_content_sha256": readback_source.content_sha256,
                    "readback_source_projection_revision": readback_source.projection_revision,
                    "readback_source_staleness_hash": readback_source.staleness_hash,
                }),
            );
            return self.action_observer.failed(
                action_generation,
                format!("wiki_save_conflict: {conflict}"),
                detail,
            );
        }

        let detail = self.terminal_detail(
            "save",
            action_generation,
            pending.edit_mode_generation,
            &pending.draft_identity,
            &pending.draft_sha256,
            &pending.source,
            "applied",
            1,
            Some(persisted),
            serde_json::json!({
                "draft_discarded": true,
                "edit_closed": true,
                "persisted_and_read_back": true,
            }),
        );
        self.set_page(page);
        self.finish_save_success();
        self.pending_save = None;
        self.action_observer.applied(action_generation, detail)
    }

    pub fn pending_save_action_generation(&self) -> Option<u64> {
        self.pending_save
            .as_ref()
            .map(|pending| pending.action_generation)
    }

    fn begin_retry_reload_after_save(&mut self) -> bool {
        if !self.saved_awaiting_reload || self.saving {
            return false;
        }
        self.saving = true;
        self.save_error = None;
        true
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
        self.action_observer.prepare_frame(self.action_context());
        let event = self.show_body(ui, palette);
        self.emit_action_observer(ui);
        event
    }

    fn emit_action_observer(&mut self, ui: &mut egui::Ui) {
        let Some(value) = self.action_observer.serialized() else {
            return;
        };
        let observer_author = action_status_author_id(&self.projection_id);
        let observer_id = egui::Id::new(("wiki-action-status", observer_author.as_str()));
        emit_status_accesskit(
            ui,
            observer_id,
            &observer_author,
            "Wiki action status",
            &value,
        );
    }

    fn show_body(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<WikiPageEvent> {
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
            let message = format!("Wiki page unavailable: {err}");
            let error = ui.colored_label(palette.error_text, &message);
            emit_label_accesskit(
                ui,
                error.id,
                &error_author_id(&self.projection_id),
                &message,
            );
            let retry = ui.button("Retry");
            emit_button_accesskit(ui, retry.id, &retry_author_id(&self.projection_id), "Retry");
            if retry.clicked() || accesskit_clicked(ui, retry.id) {
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
        let metadata_value = format!(
            "page_type={}; rebuild_status={}; source_count={}",
            page.page_type.as_deref().unwrap_or("unspecified"),
            page.rebuild_status,
            page.source_block_ids.len()
        );
        let metadata_row = ui.horizontal(|ui| {
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
        emit_label_accesskit(
            ui,
            metadata_row.response.id,
            &metadata_author_id(&self.projection_id),
            &metadata_value,
        );

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

        if !page.overlays.is_empty() {
            let heading = format!("Overlay annotations ({})", page.overlays.len());
            let heading_response = ui.label(egui::RichText::new(&heading).strong());
            emit_label_accesskit(
                ui,
                heading_response.id,
                &overlays_author_id(&self.projection_id),
                &heading,
            );
            for overlay in &page.overlays {
                let response = ui.label(format!("• {}", overlay.annotation));
                emit_label_accesskit(
                    ui,
                    response.id,
                    &overlay_author_id(&overlay.overlay_id),
                    &overlay.annotation,
                );
            }
        }

        // Stale footer (AC6): a subtle "Stale" notice when the verdict is not provably fresh.
        if verdict_is_stale(&page.staleness_verdict) {
            let stale_message = if page.page_type.is_some() {
                "Stale — this typed page is behind its sources. Refresh it through the project wiki engine."
            } else {
                "Stale — this projection is behind its source blocks. Rebuild to refresh."
            };
            let stale = ui.colored_label(palette.error_text, format!("⚠ {stale_message}"));
            emit_label_accesskit(
                ui,
                stale.id,
                &stale_author_id(&self.projection_id),
                stale_message,
            );
        }

        ui.separator();

        // Action row: Edit overlay + optional Rebuild. The typed read-only limitation is stated inline so
        // a no-context operator/agent understands `rendered_content` is not directly editable (MC-1).
        if self.action_observer.phase != WikiObserverPhase::Pending {
            let source = WikiSourceSnapshot::from_page(page);
            let edit_value = self.edit_target_value(&source);
            ui.horizontal(|ui| {
                let edit = ui.button("Edit overlay");
                emit_button_accesskit_value(
                    ui,
                    edit.id,
                    &edit_author_id(&self.projection_id),
                    "Edit overlay",
                    edit_value.as_deref(),
                );
                if (edit.clicked() || accesskit_clicked(ui, edit.id))
                    && self.begin_edit_observed(&source)
                {
                    emit_button_accesskit(
                        ui,
                        edit.id,
                        &action_dispatched_author_id(&self.projection_id),
                        "Edit dispatched",
                    );
                    event = Some(WikiPageEvent::EditBegan);
                }

                if page.page_type.is_none() {
                    let rebuild = ui.button("Rebuild");
                    emit_button_accesskit(
                        ui,
                        rebuild.id,
                        &rebuild_author_id(&self.projection_id),
                        "Rebuild projection",
                    );
                    if rebuild.clicked() || accesskit_clicked(ui, rebuild.id) {
                        event = Some(WikiPageEvent::Rebuild);
                    }
                }
            });
        }
        if page.page_type.is_some() {
            ui.colored_label(
                palette.text_subtle,
                "Typed project-wiki pages are rebuilt by the project wiki engine.",
            );
        }
        if let Some(err) = &self.action_error {
            let message = format!("Rebuild failed: {err}");
            let response = ui.colored_label(palette.error_text, &message);
            emit_label_accesskit(
                ui,
                response.id,
                &error_author_id(&self.projection_id),
                &message,
            );
        }
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
        let source = WikiSourceSnapshot::from_page(page);

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

        // Pending omits new action targets while the current generation is in flight. Applied and
        // Failed remain durable for observation, but the controls return immediately; starting one
        // advances the generation and supersedes the prior terminal record.
        if self.action_observer.phase != WikiObserverPhase::Pending {
            let save_value = self.save_target_value(&source);
            let cancel_value = self.cancel_target_value(&source);
            ui.horizontal(|ui| {
                let save_label = if self.saved_awaiting_reload {
                    "Saved"
                } else if self.saving {
                    "Saving…"
                } else {
                    "Save"
                };
                let save = ui.add_enabled(
                    !self.saving
                        && !self.saved_awaiting_reload
                        && !self.edit_buffer.trim().is_empty(),
                    egui::Button::new(save_label),
                );
                emit_button_accesskit_value(
                    ui,
                    save.id,
                    &save_author_id(&self.projection_id),
                    "Save overlay",
                    save_value.as_deref(),
                );
                if save.clicked() || accesskit_clicked(ui, save.id) {
                    if let Some((action_generation, annotation)) = self.begin_save_observed(&source)
                    {
                        emit_button_accesskit(
                            ui,
                            save.id,
                            &action_dispatched_author_id(&self.projection_id),
                            "Save dispatched",
                        );
                        event = Some(WikiPageEvent::Save {
                            action_generation,
                            annotation,
                        });
                    }
                }

                let cancel = ui.add_enabled(
                    !self.saving && !self.saved_awaiting_reload,
                    egui::Button::new("Cancel"),
                );
                emit_button_accesskit_value(
                    ui,
                    cancel.id,
                    &cancel_author_id(&self.projection_id),
                    "Cancel edit",
                    cancel_value.as_deref(),
                );
                if (cancel.clicked() || accesskit_clicked(ui, cancel.id))
                    && self.cancel_edit_observed(&source)
                {
                    emit_button_accesskit(
                        ui,
                        cancel.id,
                        &action_dispatched_author_id(&self.projection_id),
                        "Cancel dispatched",
                    );
                    event = Some(WikiPageEvent::Cancel);
                }
            });
        }

        if self.saved_awaiting_reload {
            let status = if self.saving {
                "Overlay saved. Reloading the page; no second write will be sent."
            } else {
                "Overlay saved, but the page reload failed. Cancel cannot undo the saved overlay."
            };
            ui.colored_label(palette.text_subtle, status);
            if !self.saving {
                let retry = ui.button("Retry Reload");
                emit_button_accesskit(
                    ui,
                    retry.id,
                    &retry_author_id(&self.projection_id),
                    "Retry overlay reload",
                );
                if (retry.clicked() || accesskit_clicked(ui, retry.id))
                    && self.begin_retry_reload_after_save()
                {
                    event = Some(WikiPageEvent::RetryReloadAfterSave);
                }
            }
        }

        // Inline save error (AC5 / PROOF5): shown below the toolbar; the buffer is preserved (we never
        // clear edit_buffer on error) and edit mode stays open.
        if let Some(err) = &self.save_error {
            let message = if self.saved_awaiting_reload {
                format!("Reload failed after the overlay was saved: {err}")
            } else {
                format!("Save failed: {err}")
            };
            let response = ui.colored_label(palette.error_text, &message);
            emit_label_accesskit(
                ui,
                response.id,
                &error_author_id(&self.projection_id),
                &message,
            );
        }

        // The multiline annotation editor — AccessKit Role::MultilineTextInput `wiki.edit-area.{id}`.
        let edit_enabled = !self.saving && !self.saved_awaiting_reload;
        let mut buffer = self.edit_buffer.clone();
        let area = egui::ScrollArea::vertical()
            .id_salt(edit_area_author_id(&self.projection_id))
            .max_height(ui.available_height() - 10.0)
            .show(ui, |ui| {
                ui.add_enabled_ui(edit_enabled, |ui| {
                    ui.add_sized(
                        Vec2::new(ui.available_width(), ui.available_height().max(120.0)),
                        egui::TextEdit::multiline(&mut buffer).hint_text("Write an overlay note…"),
                    )
                })
                .inner
            })
            .inner;
        if buffer != self.edit_buffer {
            // Route through set_edit_buffer so the cap is enforced even on direct typing.
            self.set_edit_buffer(buffer);
        }
        if edit_enabled {
            if let Some(replacement) = accesskit_string_set_value(ui, area.id) {
                // Model-facing SetValue is the same real editor mutation as keyboard input and keeps
                // the same bounded-buffer invariant.
                self.set_edit_buffer(replacement);
            }
        }
        emit_multiline_input_accesskit(
            ui,
            area.id,
            &edit_area_author_id(&self.projection_id),
            "Overlay annotation",
            &self.edit_buffer,
            edit_enabled,
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

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn bounded_terminal_error(message: &str) -> String {
    let sanitized: String = message
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let trimmed = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
    let bounded = cap_on_char_boundary(&trimmed, 768);
    if bounded.is_empty() {
        "unspecified wiki action failure".to_owned()
    } else {
        bounded
    }
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
        // Keep the exact machine-readable summary available as a value even when this node wraps a
        // horizontal row whose visual child labels would otherwise be concatenated by AccessKit.
        node.set_value(label.clone());
    });
}

/// Emit a button's live AccessKit node (Role::Button + Action::Click + author_id).
fn emit_button_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    emit_button_accesskit_value(ui, id, author_id, label, None);
}

fn emit_button_accesskit_value(
    ui: &egui::Ui,
    id: egui::Id,
    author_id: &str,
    label: &str,
    value: Option<&str>,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let value = value.map(str::to_owned);
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        if let Some(value) = &value {
            node.set_value(value.clone());
        }
        node.add_action(accesskit::Action::Click);
    });
}

fn emit_status_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str, value: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let value = value.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Status);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(value.clone());
    });
}

/// Consume a raw model-facing Click request for a custom-author-id control. This keeps the stable
/// AccessKit action path equivalent to a pointer click even when the harness or platform adapter does
/// not fold the request into egui's `Response::clicked()` bit.
fn accesskit_clicked(ui: &egui::Ui, id: egui::Id) -> bool {
    ui.input(|input| {
        input
            .accesskit_action_requests(id, accesskit::Action::Click)
            .next()
            .is_some()
    })
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
fn emit_multiline_input_accesskit(
    ui: &egui::Ui,
    id: egui::Id,
    author_id: &str,
    label: &str,
    value: &str,
    enabled: bool,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let value = value.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::MultilineTextInput);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(value.clone());
        if enabled {
            node.clear_disabled();
            node.add_action(accesskit::Action::SetValue);
        } else {
            node.set_disabled();
            node.remove_action(accesskit::Action::SetValue);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_page() -> WikiProjection {
        // Build via the backend_client parser so the test uses the SAME shape the GET delivers.
        WikiProjection {
            projection_id: "projection-fixture".to_owned(),
            workspace_id: "ws-test".to_owned(),
            title: "Ownership model".to_owned(),
            source_block_ids: vec!["blk-1".to_owned(), "blk-2".to_owned()],
            rendered_content: "# Ownership\nThe borrow checker enforces aliasing rules.".to_owned(),
            staleness_hash: "h1".to_owned(),
            rebuild_status: "fresh".to_owned(),
            created_at: "2026-06-19T00:00:00Z".to_owned(),
            updated_at: "2026-06-19T00:00:00Z".to_owned(),
            page_type: Some("concept".to_owned()),
            overlays: Vec::new(),
            staleness_verdict: serde_json::json!({ "state": "fresh" }),
        }
    }

    fn loaded_panel() -> LoomWikiPagePanel {
        let mut p = LoomWikiPagePanel::new("ws-test", "projection-fixture");
        p.set_page(seeded_page());
        p
    }

    fn persisted_overlay(annotation: &str) -> WikiOverlay {
        WikiOverlay {
            overlay_id: "overlay-7".to_owned(),
            projection_id: "projection-fixture".to_owned(),
            workspace_id: "ws-test".to_owned(),
            annotation: annotation.to_owned(),
            anchor: None,
            created_at: "2026-06-19T00:01:00Z".to_owned(),
            updated_at: "2026-06-19T00:01:01Z".to_owned(),
        }
    }

    fn observer_terminal_detail(panel: &LoomWikiPagePanel) -> serde_json::Value {
        let outer: serde_json::Value =
            serde_json::from_str(&panel.action_observer.serialized().unwrap()).unwrap();
        serde_json::from_str(outer["terminal_detail"].as_str().unwrap()).unwrap()
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
        let mut p = LoomWikiPagePanel::new("ws", "projection-fixture");
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

    #[test]
    fn observed_cancel_proves_exact_draft_discarded_without_a_write() {
        let mut p = loaded_panel();
        p.bind_pane_generation(9);
        let context = p.action_context();
        p.action_observer.prepare_frame(context);
        let source = p.source_snapshot().unwrap();
        assert!(p.begin_edit_observed(&source));
        let context = p.action_context();
        p.action_observer.prepare_frame(context);
        p.set_edit_buffer("discard this exact draft");
        let original_page = p.page.clone();
        let source = p.source_snapshot().unwrap();
        assert!(p.cancel_edit_observed(&source));

        assert!(!p.edit_mode);
        assert!(p.edit_buffer.is_empty());
        assert_eq!(p.page, original_page);
        let detail = observer_terminal_detail(&p);
        assert_eq!(detail["action"], "cancel");
        assert_eq!(detail["pane_generation"], 9);
        assert_eq!(
            detail["source_projection_revision"],
            source.projection_revision
        );
        assert_eq!(detail["write_count"], 0);
        assert_eq!(detail["no_write"], true);
        assert_eq!(detail["extra"]["draft_discarded"], true);
        assert_eq!(detail["extra"]["edit_closed"], true);
        assert_eq!(detail["extra"]["original_source_authoritative"], true);
    }

    #[test]
    fn applied_terminal_survives_repeated_capture_frames_and_next_action_supersedes_it() {
        let mut p = loaded_panel();
        p.bind_pane_generation(9);
        let context = p.action_context();
        p.action_observer.prepare_frame(context.clone());
        let source = p.source_snapshot().unwrap();
        assert!(p.begin_edit_observed(&source));
        let edit_generation = p.action_observer.generation;
        let edit_terminal = p.action_observer.serialized().unwrap();

        for _ in 0..32 {
            p.action_observer.prepare_frame(context.clone());
            assert_eq!(p.action_observer.serialized(), Some(edit_terminal.clone()));
        }

        p.set_edit_buffer("discard after captures");
        let source = p.source_snapshot().unwrap();
        assert!(
            p.cancel_target_value(&source).is_some(),
            "terminal observation must not hide the next legal control target"
        );
        assert!(p.cancel_edit_observed(&source));
        assert_eq!(p.action_observer.generation, edit_generation + 1);
        let detail = observer_terminal_detail(&p);
        assert_eq!(detail["action"], "cancel");
        assert_eq!(detail["action_generation"], edit_generation + 1);
    }

    #[test]
    fn failed_terminal_survives_repeated_capture_frames() {
        let mut p = loaded_panel();
        p.bind_pane_generation(5);
        let context = p.action_context();
        p.action_observer.prepare_frame(context.clone());
        assert!(p.begin_edit());
        p.set_edit_buffer("retain me exactly");
        let (generation, _) = p.begin_observed_save_for_test().unwrap();
        assert!(p.apply_save_transport_error(generation, "database unavailable"));
        let failed_terminal = p.action_observer.serialized().unwrap();

        for _ in 0..32 {
            p.action_observer.prepare_frame(context.clone());
            assert_eq!(
                p.action_observer.serialized(),
                Some(failed_terminal.clone())
            );
        }

        assert_eq!(p.edit_buffer, "retain me exactly");
        assert!(p.save_target_value(&p.source_snapshot().unwrap()).is_some());
    }

    #[test]
    fn terminal_observer_resets_only_when_authoritative_context_changes() {
        let mut p = loaded_panel();
        p.bind_pane_generation(7);
        let context = p.action_context();
        p.action_observer.prepare_frame(context.clone());
        let source = p.source_snapshot().unwrap();
        assert!(p.begin_edit_observed(&source));
        let generation = p.action_observer.generation;
        let terminal = p.action_observer.serialized().unwrap();

        p.action_observer.prepare_frame(context.clone());
        assert_eq!(p.action_observer.serialized(), Some(terminal));

        let changed_context = format!("{context}/replacement-source");
        p.action_observer.prepare_frame(changed_context.clone());
        assert_eq!(p.action_observer.phase, WikiObserverPhase::Ready);
        assert_eq!(p.action_observer.context, changed_context);
        assert_eq!(p.action_observer.generation, generation);
        assert!(p.action_observer.pending_target.is_none());
        assert!(p.action_observer.semantic_value.is_none());
        assert!(p.action_observer.terminal_error.is_none());
        assert!(p.action_observer.terminal_detail.is_none());
        let ready: serde_json::Value =
            serde_json::from_str(&p.action_observer.serialized().unwrap()).unwrap();
        assert_eq!(ready["state"], "ready");
    }

    #[test]
    fn observed_save_applies_only_after_exact_persisted_overlay_readback() {
        let mut p = loaded_panel();
        p.bind_pane_generation(4);
        assert!(p.begin_edit());
        p.set_edit_buffer("persist and read back");
        let (generation, annotation) = p.begin_observed_save_for_test().unwrap();
        let overlay = persisted_overlay(&annotation);
        assert!(p.mark_persisted_overlay_awaiting_readback(generation, overlay.clone()));
        assert!(p.saved_awaiting_reload && p.edit_mode);
        let mut readback = p.page.clone().unwrap();
        readback.overlays.push(overlay.clone());
        assert!(p.complete_save_readback(generation, readback));

        assert!(!p.edit_mode);
        assert!(p.edit_buffer.is_empty());
        let detail = observer_terminal_detail(&p);
        assert_eq!(detail["action"], "save");
        assert_eq!(detail["write_count"], 1);
        assert_eq!(detail["overlay_id"], overlay.overlay_id);
        assert_eq!(detail["overlay_persisted_revision"], overlay.updated_at);
        assert_eq!(detail["overlay_readback_revision"], overlay.updated_at);
        assert_eq!(detail["extra"]["persisted_and_read_back"], true);
    }

    #[test]
    fn save_transport_failure_is_typed_and_retains_exact_draft() {
        let mut p = loaded_panel();
        p.bind_pane_generation(5);
        assert!(p.begin_edit());
        p.set_edit_buffer("retain me exactly");
        let (generation, _) = p.begin_observed_save_for_test().unwrap();
        assert!(!p.apply_save_transport_error(generation + 1, "wrong generation"));
        assert!(p.saving, "stale failure cannot mutate the current Save");
        assert!(p.apply_save_transport_error(generation, "database\n unavailable"));

        assert!(p.edit_mode);
        assert_eq!(p.edit_buffer, "retain me exactly");
        assert!(!p.saving);
        let outer: serde_json::Value =
            serde_json::from_str(&p.action_observer.serialized().unwrap()).unwrap();
        assert_eq!(outer["state"], "failed");
        assert_eq!(
            outer["terminal_error"],
            "wiki_save_transport: database unavailable"
        );
        let detail = observer_terminal_detail(&p);
        assert_eq!(detail["write_count"], 0);
        assert_eq!(detail["extra"]["draft_retained"], true);
        assert_eq!(detail["extra"]["edit_open"], true);
    }

    #[test]
    fn post_success_source_conflict_keeps_original_source_and_locks_duplicate_write() {
        let mut p = loaded_panel();
        p.bind_pane_generation(6);
        assert!(p.begin_edit());
        p.set_edit_buffer("committed once, pending reconciliation");
        let original_page = p.page.clone();
        let (generation, annotation) = p.begin_observed_save_for_test().unwrap();
        let overlay = persisted_overlay(&annotation);
        assert!(p.mark_persisted_overlay_awaiting_readback(generation, overlay));
        let mut conflicting = p.page.clone().unwrap();
        conflicting.updated_at = "2026-06-19T00:02:00Z".to_owned();
        conflicting.rendered_content = "new canonical source".to_owned();
        assert!(p.complete_save_readback(generation, conflicting));

        assert_eq!(
            p.page, original_page,
            "conflicting readback is not installed"
        );
        assert!(p.edit_mode && p.saved_awaiting_reload);
        assert_eq!(p.edit_buffer, "committed once, pending reconciliation");
        assert!(p.begin_save().is_none(), "duplicate POST remains locked");
        assert!(!p.cancel_edit(), "Cancel cannot undo the committed overlay");
        let outer: serde_json::Value =
            serde_json::from_str(&p.action_observer.serialized().unwrap()).unwrap();
        assert_eq!(outer["state"], "failed");
        assert!(outer["terminal_error"]
            .as_str()
            .unwrap()
            .starts_with("wiki_save_conflict:"));
        let detail = observer_terminal_detail(&p);
        assert_eq!(detail["write_count"], 1);
        assert_eq!(detail["outcome"], "conflict");
        assert_eq!(detail["extra"]["draft_retained"], true);
    }

    #[test]
    fn in_flight_save_locks_same_pane_cancel_edit_and_second_save() {
        let mut p = loaded_panel();
        p.begin_edit();
        p.set_edit_buffer("first committed annotation");
        assert_eq!(
            p.begin_save().as_deref(),
            Some("first committed annotation")
        );

        assert!(
            !p.cancel_edit(),
            "Cancel is rejected while Save/reload is in flight"
        );
        p.set_edit_buffer("newer text that an old completion could erase");
        assert_eq!(
            p.edit_buffer, "first committed annotation",
            "typing is locked while the in-flight operation owns the buffer"
        );
        assert!(
            p.begin_save().is_none(),
            "a second same-pane save is rejected"
        );
        assert!(p.saving && p.edit_mode);
    }

    #[test]
    fn rebuild_failure_preserves_last_good_projection() {
        let mut p = loaded_panel();
        let last_good = p.page.clone();
        p.loading = true;
        p.apply_rebuild_error("typed page rebuild rejected");
        assert_eq!(p.page, last_good);
        assert!(!p.loading);
        assert_eq!(
            p.action_error.as_deref(),
            Some("typed page rebuild rejected")
        );
    }

    #[test]
    fn typed_page_has_no_direct_rebuild_target_and_overlays_are_addressable() {
        let page = seeded_page();
        assert!(page.page_type.is_some());
        assert_eq!(
            overlays_author_id(&page.projection_id),
            "wiki.overlays.projection-fixture"
        );
        assert_eq!(overlay_author_id("overlay/1"), "wiki.overlay.overlay-1");
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

    #[test]
    fn saved_overlay_reload_failure_cannot_post_again_or_cancel() {
        let mut p = loaded_panel();
        assert!(p.begin_edit());
        p.set_edit_buffer("persist exactly once");
        assert!(p.begin_save().is_some());
        p.mark_overlay_saved_awaiting_reload();
        p.apply_reload_after_save_error("GET 500");

        assert!(p.saved_awaiting_reload);
        assert!(!p.saving);
        assert!(p.begin_save().is_none(), "Save cannot emit a second POST");
        assert!(!p.cancel_edit(), "Cancel cannot undo a committed overlay");
        assert_eq!(p.edit_buffer, "persist exactly once");
        assert!(p.begin_retry_reload_after_save());
        assert!(p.saving, "Retry Reload locks until the GET completes");

        p.finish_save_success();
        assert!(!p.saved_awaiting_reload);
        assert!(!p.edit_mode);
        assert!(p.edit_buffer.is_empty());
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

    /// AC6 / LM-PWIKI-008: fresh only when state=="fresh"; everything else is fail-closed stale.
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
            verdict_is_stale(&serde_json::Value::Null),
            "a missing/null verdict must fail closed as stale"
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
        assert_eq!(
            title_author_id("projection-fixture"),
            "wiki.title.projection-fixture"
        );
        assert_eq!(
            content_author_id("projection-fixture"),
            "wiki.content.projection-fixture"
        );
        assert_eq!(
            edit_author_id("projection-fixture"),
            "wiki.edit.projection-fixture"
        );
        assert_eq!(
            edit_area_author_id("projection-fixture"),
            "wiki.edit-area.projection-fixture"
        );
        assert_eq!(
            save_author_id("projection-fixture"),
            "wiki.save.projection-fixture"
        );
        assert_eq!(
            cancel_author_id("projection-fixture"),
            "wiki.cancel.projection-fixture"
        );
        assert_eq!(
            metadata_author_id("projection-fixture"),
            "wiki.metadata.projection-fixture"
        );
        assert_eq!(
            stale_author_id("projection-fixture"),
            "wiki.stale.projection-fixture"
        );
        assert_eq!(
            error_author_id("projection-fixture"),
            "wiki.error.projection-fixture"
        );
        assert_eq!(
            action_status_author_id("projection-fixture"),
            "wiki.action-status.projection-fixture"
        );
    }
}
