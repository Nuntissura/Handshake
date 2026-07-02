//! Per-document reading/preview mode for the native rich-text editor (WP-KERNEL-012 MT-055).
//!
//! This is the Obsidian "reading view" parity item and the native equivalent of the React
//! `ViewModeToggle`: any note can be flipped between an editable WYSIWYG view ([`ViewMode::Edit`])
//! and a clean, read-only rendered presentation ([`ViewMode::Reading`]). The chosen mode is
//! persisted PER `document_id` so reopening a note restores its last view.
//!
//! ## What this MT does NOT do (the parity invariant — RISK-001 / MC-001)
//!
//! Reading mode does NOT introduce a second rendering path. It reuses the MT-011 `DocModel` as the
//! single source of truth and the MT-012 block renderer pipeline
//! ([`crate::rich_editor::renderer::rich_editor_widget`]), switching only the interaction/chrome
//! behavior between Edit and Reading. The ONLY differences a `read_only` render makes are:
//! (a) no caret/selection state is resolved/painted, (b) no editable `TextEdit` runs are emitted,
//! (c) input-handler edit dispatch is skipped, and (d) wider reading margins/typography are applied.
//! Wikilink chips (MT-015) and embeds/node-views (MT-014) stay interactive in both modes.
//!
//! ## Why the store is egui-free
//!
//! [`ReadingModeStore`] is pure data (a `HashMap<document_id, ViewMode>`) so the persistence and
//! toggle logic are unit-testable WITHOUT an `egui::Context` (the per-document isolation +
//! flip-back-and-forth proofs construct a store directly). The egui-context accessor
//! ([`reading_mode_store`]) lives in this same module but is a thin wrapper over
//! `ctx.data_mut(get_persisted_mut_or_default)`, so the per-document choice survives re-render and
//! (via egui's `persistence` layer) app restart.
//!
//! ## Persistence target (NOT the EventLedger)
//!
//! View-mode state is PURE frontend view-state with no backend binding. It persists via egui's local
//! context persistence only — it is NOT routed through the EventLedger/PostgreSQL (those govern
//! durable backend state, which this MT does not touch).

use std::collections::HashMap;

use egui::accesskit;

use crate::theme::HsPalette;

/// The stable egui persisted-data key the per-document [`ReadingModeStore`] lives under. A fixed
/// string id so the store round-trips across re-render and app restart through egui's persistence
/// layer (the host reads `store.get(document_id)` every frame).
pub const READING_MODE_STORE_ID: &str = "handshake_rich_reading_modes";

/// The AccessKit author_id for the Edit|Reading toggle container (`Role::Group`). A swarm agent
/// reads the current view mode + drives the toggle by these stable ids (HBR-SWARM).
pub const TOGGLE_CONTAINER_AUTHOR_ID: &str = "rich-reading-mode-toggle";
/// The AccessKit author_id for the "Edit" segment (`Role::Button`).
pub const TOGGLE_EDIT_AUTHOR_ID: &str = "rich-reading-mode-edit";
/// The AccessKit author_id for the "Reading" segment (`Role::Button`).
pub const TOGGLE_READING_AUTHOR_ID: &str = "rich-reading-mode-reading";

/// The reading-mode content column clamp (logical points): the rendered text column is centered and
/// clamped to this width in Reading mode so long notes get a distraction-free reading measure
/// (Obsidian/Notion "readable line length"). A NAMED const (not a scattered literal) per the MT
/// contract's THEME TYPOGRAPHY note — the column-width/spacing may be named consts, but any COLOR
/// must come from theme tokens. ~720pt sits in the contract's 700–760 logical-px band.
pub const READING_COLUMN_WIDTH_PTS: f32 = 720.0;

/// Extra vertical paragraph spacing (logical points) added between blocks in Reading mode on top of
/// the editor's normal block gap, for a roomier reading rhythm. A NAMED const per the THEME
/// TYPOGRAPHY note.
pub const READING_EXTRA_BLOCK_SPACING_PTS: f32 = 6.0;

/// Which view a document is presented in.
///
/// `Edit` is the full MT-012 editable WYSIWYG surface (caret, selection, typing, formatting).
/// `Reading` is a clean, read-only rendered presentation of the SAME `DocModel` (no editing
/// affordances). Derives `serde` so [`ReadingModeStore`] round-trips through egui's persistence
/// layer; derives `Copy`/`Eq` so the host can cheaply compare + thread it as a `read_only` flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ViewMode {
    /// The editable WYSIWYG editor (the MT-012 default).
    Edit,
    /// The read-only rendered reading/preview view (the Obsidian "reading view" parity item).
    Reading,
}

impl ViewMode {
    /// The other mode (Edit <-> Reading). Used by [`ReadingModeStore::toggle`].
    pub fn toggled(self) -> Self {
        match self {
            ViewMode::Edit => ViewMode::Reading,
            ViewMode::Reading => ViewMode::Edit,
        }
    }

    /// True when this mode renders the document read-only (the `read_only` flag the renderer honors).
    pub fn is_read_only(self) -> bool {
        matches!(self, ViewMode::Reading)
    }

    /// The human/model-readable label for this mode's toggle segment.
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Edit => "Edit",
            ViewMode::Reading => "Reading",
        }
    }
}

impl Default for ViewMode {
    /// A document with no remembered choice opens in [`ViewMode::Edit`] (the editor default).
    fn default() -> Self {
        ViewMode::Edit
    }
}

/// A per-`document_id` map of the chosen [`ViewMode`]. Pure data (no egui dependency) so the
/// persistence + per-document-isolation logic is unit-testable without an `egui::Context`. Derives
/// `serde` + `Default` + `Clone` so egui's `get_persisted_mut_or_default` can round-trip it across
/// re-render and app restart (RISK-004 / MC-004: the choice is keyed PER document, never global).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReadingModeStore {
    /// document_id -> chosen mode. A document absent from the map defaults to [`ViewMode::Edit`]
    /// (so a never-toggled document opens editable, and a different document never inherits another
    /// document's Reading state — per-document isolation).
    modes: HashMap<String, ViewMode>,
}

impl ReadingModeStore {
    /// An empty store (every document defaults to [`ViewMode::Edit`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// The mode for `document_id`, defaulting to [`ViewMode::Edit`] when the document has no
    /// remembered choice (per-document isolation: an unknown document is Edit, not whatever another
    /// document was set to).
    pub fn get(&self, document_id: &str) -> ViewMode {
        self.modes.get(document_id).copied().unwrap_or_default()
    }

    /// Set `document_id` to `mode`. Keyed per document, so setting one document's mode never affects
    /// another's (RISK-004 / MC-004).
    pub fn set(&mut self, document_id: &str, mode: ViewMode) {
        self.modes.insert(document_id.to_owned(), mode);
    }

    /// Flip `document_id` between Edit and Reading and return the NEW mode. A document with no
    /// remembered choice toggles from its default (Edit) to Reading.
    pub fn toggle(&mut self, document_id: &str) -> ViewMode {
        let next = self.get(document_id).toggled();
        self.set(document_id, next);
        next
    }

    /// The number of documents with a remembered (non-default) choice. For tests/diagnostics.
    pub fn len(&self) -> usize {
        self.modes.len()
    }

    /// True when no document has a remembered choice.
    pub fn is_empty(&self) -> bool {
        self.modes.is_empty()
    }
}

/// A CLONE of the persisted [`ReadingModeStore`] from this egui context's persisted data (keyed by
/// [`READING_MODE_STORE_ID`]). egui's `get_persisted_mut_or_default` returns a `&mut` into the data
/// map; we clone it out so the caller holds an owned store it can read+mutate, then write back via
/// [`write_reading_mode_store`]. The clone is cheap (a small `HashMap`) and avoids holding the egui
/// data lock across the editor render.
pub fn reading_mode_store(ctx: &egui::Context) -> ReadingModeStore {
    let id = egui::Id::new(READING_MODE_STORE_ID);
    ctx.data_mut(|d| {
        d.get_persisted_mut_or_default::<ReadingModeStore>(id)
            .clone()
    })
}

/// Write `store` back into this egui context's persisted data (keyed by [`READING_MODE_STORE_ID`]),
/// so a toggle this frame survives re-render and (via egui persistence) app restart. Pairs with
/// [`reading_mode_store`].
pub fn write_reading_mode_store(ctx: &egui::Context, store: &ReadingModeStore) {
    let id = egui::Id::new(READING_MODE_STORE_ID);
    ctx.data_mut(|d| d.insert_persisted(id, store.clone()));
}

/// Render the Edit|Reading segmented toggle inside the native editor chrome and return the active
/// mode AFTER handling clicks for `document_id`. The toggle is two selectable segments; clicking a
/// segment sets that mode in `store` (the caller persists the store). Reuses the WP-011 theme
/// palette for the segment fill (no hardcoded color — CONTROL-4) and registers the AccessKit
/// author_ids via the WP-011 `accessibility::*` live-emission helpers.
///
/// AccessKit (HBR-SWARM): the container is `rich-reading-mode-toggle` (`Role::Group`); the Edit
/// segment is `rich-reading-mode-edit` (`Role::Button`) and the Reading segment is
/// `rich-reading-mode-reading` (`Role::Button`). The ACTIVE segment is marked toggled/selected
/// (`node.set_toggled(Toggled::True)`) so a no-context swarm agent reads the current view mode
/// without inferring from labels.
pub fn view_mode_toggle(
    ui: &mut egui::Ui,
    document_id: &str,
    store: &mut ReadingModeStore,
) -> ViewMode {
    let current = store.get(document_id);
    // Resolve the active palette from egui's current visuals (always present): the editor host seeds
    // egui's base Visuals from the active HsTheme, so dark-mode visuals -> the dark palette and vice
    // versa. This keeps the toggle's segment colors theme-driven (CONTROL-4: no hardcoded Color32)
    // without depending on a ctx-data palette the host may not have installed at the widget level.
    let palette = if ui.visuals().dark_mode {
        HsPalette::dark()
    } else {
        HsPalette::light()
    };

    let mut clicked: Option<ViewMode> = None;

    // The segmented toggle container is a horizontal group; its container node carries the
    // `rich-reading-mode-toggle` author_id with Role::Group (a swarm agent locates the toggle here,
    // then reads which segment is toggled).
    let group = ui.horizontal(|ui| {
        if segment_button(ui, ViewMode::Edit, current, TOGGLE_EDIT_AUTHOR_ID, &palette) {
            clicked = Some(ViewMode::Edit);
        }
        if segment_button(
            ui,
            ViewMode::Reading,
            current,
            TOGGLE_READING_AUTHOR_ID,
            &palette,
        ) {
            clicked = Some(ViewMode::Reading);
        }
    });

    // Emit the Group container node onto the horizontal layout's response id (REUSE the WP-011
    // accessibility live helper rather than emitting an AccessKit node inline). The container is
    // non-interactive (a Group), so it OWNS its role+author_id+label.
    let container_id = group.response.id;
    ui.ctx().accesskit_node_builder(container_id, |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(TOGGLE_CONTAINER_AUTHOR_ID.to_owned());
        node.set_label("View mode".to_owned());
    });

    if let Some(mode) = clicked {
        store.set(document_id, mode);
        return mode;
    }
    current
}

/// Render ONE segment of the Edit|Reading toggle: a selectable button styled from the theme accent
/// (active) or surface (inactive), carrying its stable `author_id` and — when active — a toggled
/// AccessKit state so a swarm agent reads the current mode. Returns `true` when this segment was
/// clicked.
fn segment_button(
    ui: &mut egui::Ui,
    mode: ViewMode,
    current: ViewMode,
    author_id: &str,
    palette: &HsPalette,
) -> bool {
    let active = mode == current;
    // The segment fill comes from theme tokens ONLY (CONTROL-4 — no hardcoded Color32): the active
    // segment uses the accent, the inactive uses the surface; text uses the on-accent / normal text
    // token so contrast reads in both light and dark.
    let (fill, text_color) = if active {
        (palette.accent, palette.surface)
    } else {
        (palette.surface, palette.text)
    };
    let resp = ui.add(
        egui::Button::new(egui::RichText::new(mode.label()).color(text_color))
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, palette.border)),
    );
    // egui already derives Role::Button + Action::Click + Action::Focus for a Button response; we
    // ADD the stable author_id (REUSE the WP-011 emit_interactive_node helper so the live node keeps
    // egui's interactive role/actions) and mark the ACTIVE segment toggled so the current mode is
    // readable out-of-process.
    crate::accessibility::emit_interactive_node(ui.ctx(), resp.id, author_id);
    if active {
        ui.ctx().accesskit_node_builder(resp.id, |node| {
            node.set_toggled(accesskit::Toggled::True);
        });
    }
    resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-007 (per-document isolation + flip): toggle for "doc-A" flips Edit->Reading->Edit; "doc-B"
    // still defaults to Edit.
    #[test]
    fn toggle_flips_and_is_per_document() {
        let mut store = ReadingModeStore::new();
        // A never-toggled document defaults to Edit.
        assert_eq!(store.get("doc-A"), ViewMode::Edit);
        assert_eq!(store.get("doc-B"), ViewMode::Edit);
        // Toggle doc-A: Edit -> Reading -> Edit.
        assert_eq!(store.toggle("doc-A"), ViewMode::Reading);
        assert_eq!(store.get("doc-A"), ViewMode::Reading);
        assert_eq!(store.toggle("doc-A"), ViewMode::Edit);
        assert_eq!(store.get("doc-A"), ViewMode::Edit);
        // doc-B is unaffected — per-document isolation (RISK-004 / MC-004).
        assert_eq!(store.get("doc-B"), ViewMode::Edit);
    }

    #[test]
    fn set_is_per_document() {
        let mut store = ReadingModeStore::new();
        store.set("doc-A", ViewMode::Reading);
        assert_eq!(store.get("doc-A"), ViewMode::Reading);
        assert_eq!(store.get("doc-B"), ViewMode::Edit, "doc-B must stay Edit");
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn reading_is_read_only_edit_is_not() {
        assert!(ViewMode::Reading.is_read_only());
        assert!(!ViewMode::Edit.is_read_only());
    }

    #[test]
    fn default_view_mode_is_edit() {
        assert_eq!(ViewMode::default(), ViewMode::Edit);
        assert!(ReadingModeStore::new().is_empty());
    }

    // The store round-trips through serde (the egui-persistence shape) — a Reading choice survives
    // a serialize/deserialize cycle (the same path egui's persisted-data layer uses across restart).
    #[test]
    fn store_round_trips_through_serde() {
        let mut store = ReadingModeStore::new();
        store.set("doc-A", ViewMode::Reading);
        store.set("doc-B", ViewMode::Edit);
        let json = serde_json::to_string(&store).expect("serialize");
        let back: ReadingModeStore = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.get("doc-A"), ViewMode::Reading);
        assert_eq!(back.get("doc-B"), ViewMode::Edit);
        assert_eq!(back, store);
    }
}
