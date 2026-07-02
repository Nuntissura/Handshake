//! The document properties panel for the native rich-text editor (WP-KERNEL-012 MT-017).
//!
//! This is the native Rust port of the document-metadata section of
//! `app/src/components/RichDocumentView.tsx`: a collapsible panel (default collapsed, mounted ABOVE
//! the document content in the shared [`crate::rich_editor::renderer::rich_editor_widget`]) that
//! displays and edits the knowledge document's metadata, bound to the REAL `handshake_core`
//! knowledge-document API.
//!
//! ## VERIFIED BACKEND SHAPE (the MT-017 critical gates, anchored to the real backend)
//!
//! The MT contract assumed the title (and project_ref/folder_ref) save through `PUT /save`. The REAL
//! backend (`src/backend/handshake_core/src/api/knowledge_documents.rs`) is DIFFERENT, and binding the
//! wrong endpoint would silently fail to persist:
//!
//! - `PUT /knowledge/documents/{id}/save` takes `{ expected_version, content_json, … }` ONLY — it is a
//!   CONTENT save and does NOT accept or update `title`. Sending a title there is a no-op (it would be
//!   ignored), so the contract's "save title via PUT /save with the full content_json" is WRONG.
//! - The REAL title update is `POST /knowledge/documents/{id}/rename` with `{ title }`.
//! - The REAL project_ref/folder_ref update is `POST /knowledge/documents/{id}/move` with
//!   `{ project_ref?, folder_ref? }` (absent = unchanged, explicit `null` = clear, string = set).
//! - The metadata SOURCE is `GET /knowledge/documents/{id}` -> `RichDocLoad.document` (a
//!   [`DocMetadata`] mirror), and the backlinks COUNT is `GET /knowledge/documents/{id}/backlinks`.
//!
//! So MT-017 binds the title edit to the REAL `/rename` endpoint (verified shape from
//! `RenameBody { title }`), NOT `/save`. See [`metadata_client`].
//!
//! MC-001 NOTE on the contract's "send the live content_json" control: because the REAL title path is
//! `/rename` (title-only, no content body), there is NO stale-content-clobber risk for a title edit —
//! `/rename` never touches `content_json`. The control's INTENT (never clobber the operator's live
//! edits with a cached copy) is satisfied trivially: a title rename leaves the document body untouched
//! on the backend. The unit test still asserts the title path carries the live title and never a
//! content body, documenting that the rename cannot clobber content.
//!
//! ## tags: a real backend gap (MC-002 / no fake persistence)
//!
//! [`DocMetadata`] mirrors the verified `RichDocument` type from `app/src/lib/api.ts` (lines 3028-3048).
//! It has `title`, `project_ref`, `folder_ref`, `doc_version`, `authority_label`, `owner_actor_*`,
//! `created_at`, `updated_at`, `crdt_document_id` — but NO `tags` field, and the knowledge-document API
//! has no tag endpoint. So the tag editor renders a VISIBLE "Tags not persisted (backend gap)" banner
//! and keeps a LOCAL-ONLY [`PropertiesState::tags`] list (add/remove work in-memory but never persist)
//! rather than faking persistence. See [`tag_editor`].
//!
//! ## Submodules
//!
//! - [`metadata_client`] — the async backend transport trait (rename / move / load / backlinks-count),
//!   the typed [`metadata_client::MetadataError`] vocabulary, the verified backend response shapes, the
//!   production reqwest impl, and the per-editor [`metadata_client::PropertiesRuntime`] (save state +
//!   backlinks-count cell with MC-004 generation cancellation). Unit-testable with a counted mock.
//! - [`fields`] — the individual field renderers: editable single-line title, read-only monospace
//!   document-id with click-to-copy, the local human-readable date display, and the version /
//!   authority-label badges.
//! - [`tag_editor`] — the tag chip row (add/remove) + the backend-gap banner (MC-002).
//! - [`panel`] — the full two-column properties grid widget.
//!
//! Everything REUSES the WP-011 shell: `theme/*` (every color a theme token, no hardcoded hex),
//! `accessibility/*` (live AccessKit emission via `ctx.accesskit_node_builder`), the reqwest REST stack
//! (no new HTTP crate), and `egui::Context::copy_text` for the clipboard (no `arboard` direct dep).

pub mod fields;
pub mod metadata_client;
pub mod panel;
pub mod tag_editor;

use crate::rich_editor::properties::metadata_client::DocMetadata;

/// The AccessKit author_id for the properties panel container (the collapsing header).
pub const PANEL_AUTHOR_ID: &str = "properties-panel";

/// The AccessKit author_id for the editable title field.
pub const TITLE_FIELD_AUTHOR_ID: &str = "properties-title";

/// The AccessKit author_id for the read-only document-id (click-to-copy) label.
pub const DOC_ID_FIELD_AUTHOR_ID: &str = "properties-doc-id";

/// The AccessKit author_id for the tags container (the AC-9 contract id).
pub const TAGS_CONTAINER_AUTHOR_ID: &str = "properties-tags";

/// The AccessKit author_id for the tag "add" button (the contract id).
pub const TAG_ADD_BUTTON_AUTHOR_ID: &str = "tag-add-button";

/// The AccessKit author_id for one tag chip (`tag-chip-{tag}` — the contract id).
pub fn tag_chip_author_id(tag: &str) -> String {
    format!("tag-chip-{tag}")
}

/// The per-document editor properties state: the loaded backend metadata, the in-progress title edit
/// buffer, the LOCAL-ONLY tag list (backend gap — MC-002), and the pending-save flag.
///
/// Owned by [`crate::rich_editor::renderer::rich_editor_widget::RichEditorState`] so it survives across
/// frames (the title edit buffer + tag list + collapsed state persist). The metadata is replaced when a
/// document loads (or a rename/move round-trips).
#[derive(Debug, Clone)]
pub struct PropertiesState {
    /// The loaded backend metadata (the read-only fields + the persisted title).
    pub doc_metadata: DocMetadata,
    /// The in-progress title edit buffer. `None` until the operator focuses the title field; `Some`
    /// while editing. Committing (Enter / focus-lost with a change) sets `pending_save` and clears it.
    pub title_edit: Option<String>,
    /// The LOCAL-ONLY tag list. The backend `RichDocument` has NO tags field (MC-002), so add/remove
    /// mutate this in-memory list and are NEVER persisted; the panel shows a visible backend-gap banner.
    pub tags: Vec<String>,
    /// True once a title edit has been committed and a save (rename) should be dispatched. The host
    /// shell observes this, dispatches the rename through [`metadata_client::PropertiesRuntime`], and
    /// clears it (so it is a one-shot request, not a per-frame poll).
    pub pending_save: bool,
    /// The in-progress NEW-tag input buffer (the add affordance). `None` when not adding; `Some` while
    /// the inline tag TextEdit is open.
    pub new_tag_input: Option<String>,
}

impl PropertiesState {
    /// A fresh properties state over loaded metadata, no edit in progress, no local tags.
    pub fn new(doc_metadata: DocMetadata) -> Self {
        Self {
            doc_metadata,
            title_edit: None,
            tags: Vec::new(),
            pending_save: false,
            new_tag_input: None,
        }
    }

    /// Begin editing the title: seed the edit buffer from the current persisted title (idempotent — a
    /// no-op if an edit is already in progress so re-entry does not discard typed characters).
    pub fn begin_title_edit(&mut self) {
        if self.title_edit.is_none() {
            self.title_edit = Some(self.doc_metadata.title.clone());
        }
    }

    /// Commit the in-progress title edit (Enter / focus-lost). If the buffer differs from the persisted
    /// title (after trimming), mark [`Self::pending_save`] so the host dispatches a rename, and reflect
    /// the new title optimistically in `doc_metadata` so the field shows the typed value immediately.
    /// A blank or unchanged title is a no-op (no save). Returns true when a save was marked.
    pub fn commit_title_edit(&mut self) -> bool {
        let Some(edited) = self.title_edit.take() else {
            return false;
        };
        let trimmed = edited.trim();
        if trimmed.is_empty() || trimmed == self.doc_metadata.title {
            return false; // blank or unchanged -> no save (the backend rejects an empty title).
        }
        self.doc_metadata.title = trimmed.to_owned();
        self.pending_save = true;
        true
    }

    /// Add a tag to the LOCAL-ONLY list (MC-002: never persisted). Trims, ignores blank/duplicate tags,
    /// and returns true when a tag was appended.
    pub fn add_tag(&mut self, tag: impl Into<String>) -> bool {
        let tag = tag.into();
        let tag = tag.trim();
        if tag.is_empty() || self.tags.iter().any(|t| t == tag) {
            return false;
        }
        self.tags.push(tag.to_owned());
        true
    }

    /// Remove a tag from the LOCAL-ONLY list by value. Returns true when a tag was removed.
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        let before = self.tags.len();
        self.tags.retain(|t| t != tag);
        self.tags.len() != before
    }

    /// Replace the loaded metadata after a backend round-trip (load / rename / move), preserving the
    /// local-only tag list and clearing any stale edit/save state for the new metadata.
    pub fn set_metadata(&mut self, doc_metadata: DocMetadata) {
        self.doc_metadata = doc_metadata;
        self.title_edit = None;
        self.pending_save = false;
    }
}

/// Format a backend ISO-8601 timestamp string (e.g. `"2026-06-19T14:32:00Z"` or
/// `"2026-06-19T14:32:00+00:00"`) as a human-readable LOCAL date like `"Jun 19, 2026 14:32"`.
///
/// MC-003 (chrono local-offset fallback): the conversion to the LOCAL timezone can fail on a headless /
/// minimal OS config with no timezone database. We never panic: a parse failure returns the raw input
/// unchanged (so the operator still sees *something* truthful), and a local-conversion failure logs a
/// `tracing::warn` and falls back to formatting in UTC. Both `Z` and `+00:00` suffixes are handled by
/// `DateTime::parse_from_rfc3339`.
pub fn format_iso_local(iso: &str) -> String {
    use chrono::{DateTime, Local};

    let Ok(parsed) = DateTime::parse_from_rfc3339(iso.trim()) else {
        // Unparseable input: surface the raw value rather than an empty/fake date (no fabrication).
        return iso.trim().to_owned();
    };
    // `DateTime<Local>::from` resolves the local offset. On a system with no tz database the standard
    // library can fail to resolve `Local`; chrono returns the offset it can compute. We treat the
    // conversion as best-effort: if formatting in local time yields an empty string (a degenerate
    // offset), fall back to UTC. There is no panic path here — `with_timezone` is total.
    let local: DateTime<Local> = parsed.with_timezone(&Local);
    let formatted = local.format("%b %e, %Y %H:%M").to_string();
    let cleaned = collapse_double_space(&formatted);
    if cleaned.trim().is_empty() {
        tracing::warn!(
            iso = iso,
            "MT-017 properties: local-time format degenerate; falling back to UTC"
        );
        let utc = parsed.with_timezone(&chrono::Utc);
        return collapse_double_space(&utc.format("%b %e, %Y %H:%M UTC").to_string());
    }
    cleaned
}

/// `%e` is space-padded (`" 9"` for day 9), which yields a double space after the month. Collapse it so
/// `"Jun  9, 2026"` reads as `"Jun 9, 2026"`.
fn collapse_double_space(s: &str) -> String {
    s.replace("  ", " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::properties::metadata_client::DocMetadata;

    fn meta() -> DocMetadata {
        DocMetadata {
            rich_document_id: "KRD-1".into(),
            workspace_id: "ws".into(),
            title: "Original Title".into(),
            doc_version: 3,
            authority_label: "draft".into(),
            owner_actor_kind: Some("operator".into()),
            owner_actor_id: Some("ilja".into()),
            project_ref: Some("PRJ-1".into()),
            folder_ref: None,
            crdt_document_id: Some("KCRDT-1".into()),
            created_at: "2026-06-19T14:32:00Z".into(),
            updated_at: "2026-06-20T09:05:00Z".into(),
        }
    }

    #[test]
    fn author_ids_match_contract_shape() {
        assert_eq!(TAGS_CONTAINER_AUTHOR_ID, "properties-tags");
        assert_eq!(TAG_ADD_BUTTON_AUTHOR_ID, "tag-add-button");
        assert_eq!(tag_chip_author_id("rust"), "tag-chip-rust");
    }

    #[test]
    fn commit_title_edit_marks_pending_save_on_change() {
        // AC-3: editing the title and committing marks pending_save = true.
        let mut st = PropertiesState::new(meta());
        st.begin_title_edit();
        st.title_edit = Some("New Title".into());
        assert!(st.commit_title_edit(), "a changed title commits");
        assert!(
            st.pending_save,
            "AC-3: a committed title change marks pending_save"
        );
        assert_eq!(
            st.doc_metadata.title, "New Title",
            "the field shows the new title optimistically"
        );
        assert!(
            st.title_edit.is_none(),
            "the edit buffer is cleared after commit"
        );
    }

    #[test]
    fn commit_title_edit_blank_or_unchanged_is_a_noop() {
        // The backend rejects an empty title (RenameBody validation); a blank/unchanged commit never
        // marks a save (so we never dispatch a rename the backend would 400).
        let mut st = PropertiesState::new(meta());
        st.title_edit = Some("Original Title".into()); // unchanged
        assert!(!st.commit_title_edit(), "an unchanged title does not save");
        assert!(!st.pending_save);

        st.title_edit = Some("   ".into()); // blank
        assert!(!st.commit_title_edit(), "a blank title does not save");
        assert!(!st.pending_save);
        assert_eq!(st.doc_metadata.title, "Original Title");
    }

    #[test]
    fn add_tag_appends_and_dedupes() {
        // AC-4 logic: adding a tag appends it; duplicates and blanks are ignored.
        let mut st = PropertiesState::new(meta());
        assert!(st.add_tag("rust"));
        assert!(st.add_tag("egui"));
        assert!(!st.add_tag("rust"), "duplicate tag is ignored");
        assert!(!st.add_tag("   "), "blank tag is ignored");
        assert_eq!(st.tags, vec!["rust".to_owned(), "egui".to_owned()]);
    }

    #[test]
    fn remove_tag_deletes_from_list() {
        // AC-5: removing a tag deletes it from the list.
        let mut st = PropertiesState::new(meta());
        st.add_tag("rust");
        st.add_tag("egui");
        assert!(
            st.remove_tag("rust"),
            "removing an existing tag returns true"
        );
        assert!(
            !st.remove_tag("rust"),
            "removing an absent tag returns false"
        );
        assert_eq!(st.tags, vec!["egui".to_owned()]);
    }

    #[test]
    fn format_iso_local_renders_human_readable_date() {
        // AC-7: an ISO 8601 string renders as a human-readable local date string. We cannot assert the
        // exact local hour (it depends on the test host's timezone), so we assert the STABLE,
        // tz-independent parts: the month name, the day, and the year — proving the parse + format ran
        // and did not return the raw ISO string.
        let out = format_iso_local("2026-06-19T14:32:00Z");
        assert!(out.contains("Jun"), "month name present (got {out:?})");
        assert!(out.contains("19"), "day present (got {out:?})");
        assert!(out.contains("2026"), "year present (got {out:?})");
        assert_ne!(
            out, "2026-06-19T14:32:00Z",
            "the raw ISO string was reformatted, not echoed"
        );
        assert!(
            !out.contains('T'),
            "no ISO 'T' separator in the human-readable output (got {out:?})"
        );
    }

    #[test]
    fn format_iso_local_handles_offset_suffix() {
        // The impl note: parse_from_rfc3339 handles BOTH 'Z' and '+00:00'.
        let z = format_iso_local("2026-01-02T03:04:00Z");
        let offset = format_iso_local("2026-01-02T03:04:00+00:00");
        assert!(
            z.contains("Jan") && z.contains("2026"),
            "Z suffix parsed (got {z:?})"
        );
        assert!(
            offset.contains("Jan") && offset.contains("2026"),
            "+00:00 suffix parsed (got {offset:?})"
        );
    }

    #[test]
    fn format_iso_local_echoes_unparseable_input_without_panic() {
        // MC-003: a malformed timestamp must not panic; it returns the raw value (truthful, not faked).
        assert_eq!(format_iso_local("not-a-date"), "not-a-date");
        assert_eq!(format_iso_local(""), "");
    }

    #[test]
    fn set_metadata_preserves_local_tags() {
        // A backend round-trip (rename/move) replaces the metadata but the LOCAL-only tags survive
        // (they are not part of the backend record — MC-002).
        let mut st = PropertiesState::new(meta());
        st.add_tag("keep-me");
        st.pending_save = true;
        let mut next = meta();
        next.title = "Renamed".into();
        next.doc_version = 4;
        st.set_metadata(next);
        assert_eq!(st.doc_metadata.title, "Renamed");
        assert_eq!(st.doc_metadata.doc_version, 4);
        assert!(
            !st.pending_save,
            "set_metadata clears the one-shot save flag"
        );
        assert_eq!(
            st.tags,
            vec!["keep-me".to_owned()],
            "local tags survive a metadata refresh"
        );
    }
}
