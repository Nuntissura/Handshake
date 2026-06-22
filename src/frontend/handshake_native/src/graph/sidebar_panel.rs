//! Pins / Favorites / Backlinks / Unlinked-mentions sidebar + breadcrumb strip
//! (WP-KERNEL-012 MT-024, cluster E3).
//!
//! ## What this is
//!
//! [`LoomSidebarPanel`] is the native, AccessKit-addressable Obsidian-parity knowledge-navigation
//! sidebar. It is the native peer of the React `WorkspaceSidebar.tsx` Bookmarks section, extended with
//! the per-block Backlinks + Unlinked-mentions panels and a breadcrumb strip. Five surfaces:
//!
//! - **Breadcrumb strip** — a horizontal `Home > … > Block` history of the last 5 opened blocks; each
//!   crumb is a clickable [`accesskit::Role::Link`] that fires [`SidebarEvent::Open`].
//! - **Pins** — every pinned [`LoomBlock`] (`GET /loom/views/pins`). Each row shows the title, a
//!   content-type chip, and a Remove button. Remove is the React two-call flow: `PUT /pin-order` with
//!   `{pin_order:null}` THEN `PATCH` with `{pinned:false}` (RISK-1 / MC-1). Optimistic with rollback.
//! - **Favorites** — every favorited block (`GET /loom/views/favorites`). Remove = `PATCH {favorite:false}`.
//! - **Backlinks** — only when an active block is set: the blocks that LINK to it (a real `LoomEdge`),
//!   each with its edge-type label. Distinct list from Unlinked (impl-note 67 / MC-4).
//! - **Unlinked mentions** — only when an active block is set: blocks whose text mentions the active
//!   block's title but have NO edge to it. Deduped against the Backlinks list (RISK-4 / MC-4).
//!
//! ## Backend reality (Spec-Realism Gate — the MT-008/021/022/023 "verify, don't trust the contract" rule)
//!
//! VERIFIED READ-ONLY against the running backend (`src/backend/handshake_core/src/{api,storage}/loom.rs`):
//!   - `GET  /workspaces/{ws}/loom/views/pins?limit=100`      -> `LoomViewResponse::Pins { blocks }`
//!     (`parse_view_type` accepts `pins`; CONFIRMED real, unlike the MT-022/023 stale view types).
//!   - `GET  /workspaces/{ws}/loom/views/favorites?limit=100` -> `LoomViewResponse::Favorites { blocks }`.
//!   - `PUT  /workspaces/{ws}/loom/blocks/{id}/pin-order` body `{ "pin_order": null }`
//!     (`SetPinOrderRequest`, MT-183 — the field is `pin_order`, NOT the contract's `ordinal`).
//!   - `PATCH /workspaces/{ws}/loom/blocks/{id}` body `{ "pinned": false }` / `{ "favorite": false }`
//!     (`LoomBlockUpdate`, MT-022 confirmed: `pinned`+`favorite` are `Option<bool>` PATCH fields).
//!   - **Backlinks correction (disclosed):** the contract's `graph-search?mention_ids={id}` IS a real
//!     param, but the DEDICATED `GET /workspaces/{ws}/loom/blocks/{id}/backlinks` -> `Vec<LoomBacklink>`
//!     (MT-178, `get_backlinks_with_context`) is the field-correct surface: each backlink carries the
//!     incoming `edge` (with `edge_type`) + the `source_block`. That is exactly the AC4 "source block
//!     title + edge_type label" data, so the backend client binds THAT route (verified) instead of
//!     synthesizing a star from graph-search. Both were checked; the dedicated route is bound.
//!   - **Unlinked correction (disclosed):** the contract names `GET /loom/views/unlinked` (a WORKSPACE
//!     unlinked view). For the *per-active-block* Unlinked section the correct verified surface is the
//!     DEDICATED `GET /workspaces/{ws}/loom/blocks/{id}/unlinked-mentions` -> `Vec<LoomUnlinkedMention>`
//!     (MT-178, `scan_loom_block_unlinked_mentions`): blocks whose text mentions the active block's title
//!     with NO edge. That is the AC5 semantics ("unlinked blocks that textually mention the active block,
//!     no edge"), so the backend client binds THAT route. The workspace `/views/unlinked` would NOT be
//!     scoped to the active block.
//!
//! ## Repaint discipline (the MT-015 idle-repaint lesson, per-section)
//!
//! Each section has its OWN loading flag ([`SectionKind`] -> bool). A spinner animates ONLY while that
//! section's genuine in-flight fetch is dispatched; a headless / no-runtime render shows the static
//! neutral state and never enters a perpetual `Loading…` / perpetual repaint. A slow Backlinks fetch
//! never blocks Pins from rendering (impl-note 70). A repaint is requested ONLY for a frame where a
//! genuine spinner is active.
//!
//! ## Stale-response cancellation (RISK-2)
//!
//! Rapid `active_block_id` navigation is guarded by a per-section GENERATION COUNTER (the MT-015
//! backlinks-generation pattern): each Backlinks/Unlinked reload bumps the generation; a delivery whose
//! generation is older than the current one is DROPPED. The host owns the debounce (150ms) but the
//! generation guard is the hard correctness control: a stale response can never overwrite a newer one.
//!
//! ## Collapse persistence (RISK-5 / MC-5)
//!
//! Section collapse state lives in [`egui::Memory`] keyed by a stable [`egui::Id`] derived from the
//! panel's `Id` salt, so it survives a widget rebuild (egui re-creates widgets every frame). Default is
//! expanded. A collapsed section renders NO rows -> its rows are absent from the AccessKit tree (AC8).
//!
//! ## AccessKit (HBR-SWARM)
//!
//! - breadcrumb crumb: `sidebar.breadcrumb.{idx}` (Role::Link, Action::Click).
//! - pin row: `sidebar.pin.{sanitized_block_id}` (Role::ListItem); remove button
//!   `sidebar.pin.{sanitized_block_id}.remove` (Role::Button).
//! - favorite row: `sidebar.favorite.{sanitized_block_id}`; remove `sidebar.favorite.{…}.remove`.
//! - backlink row: `sidebar.backlink.{sanitized_block_id}` (Role::ListItem).
//! - unlinked row: `sidebar.unlinked.{sanitized_block_id}` (Role::ListItem).
//! - per-section Retry button: `sidebar.{section}.retry` (Role::Button).
//!
//! Ids are sanitized to `[a-z0-9-]` via [`crate::project_tree::stable_part`] so a raw id with
//! slashes/colons can never break the tree.

use std::collections::{HashMap, HashSet};

use egui::accesskit;
use egui::{Sense, Vec2};

use crate::graph::graph_view::content_type_color;
use crate::theme::HsPalette;

/// Max breadcrumb entries retained (RISK-3 / MC-3): the oldest is dropped past this.
pub const MAX_BREADCRUMBS: usize = 5;

/// Max characters of a breadcrumb label before truncation (impl-note 72).
pub const BREADCRUMB_LABEL_MAX: usize = 20;

/// Size of a content-type chip swatch (px). Cosmetic.
const CHIP_SWATCH_SIZE: f32 = 10.0;

/// AccessKit author_id prefix for a breadcrumb crumb: `sidebar.breadcrumb.{idx}`.
pub const BREADCRUMB_AUTHOR_ID_PREFIX: &str = "sidebar.breadcrumb.";

/// AccessKit author_id prefix for a pin row: `sidebar.pin.{sanitized_block_id}`.
pub const PIN_ROW_AUTHOR_ID_PREFIX: &str = "sidebar.pin.";

/// AccessKit author_id prefix for a favorite row: `sidebar.favorite.{sanitized_block_id}`.
pub const FAVORITE_ROW_AUTHOR_ID_PREFIX: &str = "sidebar.favorite.";

/// AccessKit author_id prefix for a backlink row: `sidebar.backlink.{sanitized_block_id}`.
pub const BACKLINK_ROW_AUTHOR_ID_PREFIX: &str = "sidebar.backlink.";

/// AccessKit author_id prefix for an unlinked-mention row: `sidebar.unlinked.{sanitized_block_id}`.
pub const UNLINKED_ROW_AUTHOR_ID_PREFIX: &str = "sidebar.unlinked.";

/// AccessKit author_id for a crumb at `idx`: `sidebar.breadcrumb.{idx}`.
pub fn breadcrumb_author_id(idx: usize) -> String {
    format!("{BREADCRUMB_AUTHOR_ID_PREFIX}{idx}")
}

/// AccessKit author_id for a pin row: `sidebar.pin.{sanitized_block_id}`.
pub fn pin_row_author_id(block_id: &str) -> String {
    format!("{PIN_ROW_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(block_id))
}

/// AccessKit author_id for a pin row's Remove button: `sidebar.pin.{sanitized_block_id}.remove`.
pub fn pin_remove_author_id(block_id: &str) -> String {
    format!("{}.remove", pin_row_author_id(block_id))
}

/// AccessKit author_id for a favorite row: `sidebar.favorite.{sanitized_block_id}`.
pub fn favorite_row_author_id(block_id: &str) -> String {
    format!("{FAVORITE_ROW_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(block_id))
}

/// AccessKit author_id for a favorite row's Remove button.
pub fn favorite_remove_author_id(block_id: &str) -> String {
    format!("{}.remove", favorite_row_author_id(block_id))
}

/// AccessKit author_id for a backlink row: `sidebar.backlink.{sanitized_block_id}`.
pub fn backlink_row_author_id(block_id: &str) -> String {
    format!("{BACKLINK_ROW_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(block_id))
}

/// AccessKit author_id for an unlinked-mention row: `sidebar.unlinked.{sanitized_block_id}`.
pub fn unlinked_row_author_id(block_id: &str) -> String {
    format!("{UNLINKED_ROW_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(block_id))
}

/// AccessKit author_id for a section's Retry button: `sidebar.{section}.retry`.
pub fn section_retry_author_id(section: SectionKind) -> String {
    format!("sidebar.{}.retry", section.slug())
}

/// The four data sections the sidebar loads independently (impl-note 70). The breadcrumb strip is not a
/// fetched section so it is not a variant. Pins/Favorites reload on `workspace_id` change; Backlinks/
/// Unlinked reload on `active_block_id` change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionKind {
    Pins,
    Favorites,
    Backlinks,
    Unlinked,
}

impl SectionKind {
    /// Every section, in render order.
    pub const ALL: [SectionKind; 4] = [
        SectionKind::Pins,
        SectionKind::Favorites,
        SectionKind::Backlinks,
        SectionKind::Unlinked,
    ];

    /// The header title for this section.
    pub fn title(self) -> &'static str {
        match self {
            SectionKind::Pins => "Pins",
            SectionKind::Favorites => "Favorites",
            SectionKind::Backlinks => "Backlinks",
            SectionKind::Unlinked => "Unlinked Mentions",
        }
    }

    /// The lowercase slug used in AccessKit ids + egui Memory keys.
    pub fn slug(self) -> &'static str {
        match self {
            SectionKind::Pins => "pins",
            SectionKind::Favorites => "favorites",
            SectionKind::Backlinks => "backlinks",
            SectionKind::Unlinked => "unlinked",
        }
    }

    /// True for the two sections that render ONLY when an active block is set (Backlinks/Unlinked).
    fn requires_active_block(self) -> bool {
        matches!(self, SectionKind::Backlinks | SectionKind::Unlinked)
    }
}

/// One block row in the Pins / Favorites sections (the bookmark surface). Carries the open key, a
/// display title, and the `content_type` (drives the chip color via the shared theme — no hardcoded hex).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarBlock {
    pub block_id: String,
    pub title: String,
    pub content_type: String,
}

impl SidebarBlock {
    pub fn new(
        block_id: impl Into<String>,
        title: impl Into<String>,
        content_type: impl Into<String>,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
            content_type: content_type.into(),
        }
    }
}

/// One backlink row: the source block that LINKS to the active block, plus the edge-type label (AC4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacklinkRow {
    /// The block that references the active block (the edge SOURCE).
    pub block_id: String,
    pub title: String,
    /// The incoming edge's type ("mention" / "tag" / …) shown as a label chip (AC4).
    pub edge_type: String,
}

impl BacklinkRow {
    pub fn new(
        block_id: impl Into<String>,
        title: impl Into<String>,
        edge_type: impl Into<String>,
    ) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
            edge_type: edge_type.into(),
        }
    }
}

/// One unlinked-mention row: a block whose text mentions the active block's title but has NO edge (AC5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlinkedRow {
    pub block_id: String,
    pub title: String,
}

impl UnlinkedRow {
    pub fn new(block_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
        }
    }
}

/// One breadcrumb entry: a block in the navigation history (RISK-3). `title` is the display label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbEntry {
    pub block_id: String,
    pub title: String,
}

impl BreadcrumbEntry {
    pub fn new(block_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
        }
    }

    /// The crumb label truncated to [`BREADCRUMB_LABEL_MAX`] chars with an ellipsis (impl-note 72).
    fn label(&self) -> String {
        truncate_label(&self.title, BREADCRUMB_LABEL_MAX)
    }
}

/// Truncate `s` to at most `max` chars, appending `…` when it was cut. Char-count based (never panics on
/// a multibyte boundary). Pure so the breadcrumb-truncation proof can test it standalone.
pub fn truncate_label(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let kept: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// The typed event a [`LoomSidebarPanel`] interaction produces this frame, for the host to apply. The
/// widget NEVER touches the network (HBR-QUIET); the host owns the backend wiring + navigation routing +
/// the event-bus emit after a mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarEvent {
    /// A row or breadcrumb was clicked: navigate to `block_id` (fires the `on_open` callback). The host
    /// pushes a breadcrumb for this block.
    Open { block_id: String },
    /// A pin's Remove button was clicked (AC2): the host runs the two-call removal
    /// (`PUT /pin-order null` THEN `PATCH {pinned:false}`), emits the bookmark-changed event, and
    /// re-fetches Pins. The row was already removed OPTIMISTICALLY from the local list (RISK-1).
    RemovePin { block_id: String },
    /// A favorite's Remove button was clicked (AC3): the host runs `PATCH {favorite:false}`, emits the
    /// changed event, and re-fetches Favorites. The row was already removed optimistically.
    RemoveFavorite { block_id: String },
    /// A section's Retry button was pressed (AC9): the host re-fires that section's load.
    Retry { section: SectionKind },
}

/// The sidebar widget state. Held by the host (the pane), mutated in place by [`LoomSidebarPanel::show`].
#[derive(Debug, Clone)]
pub struct LoomSidebarPanel {
    pub workspace_id: String,
    /// The currently-open block (drives the Backlinks + Unlinked sections). `None` => those two sections
    /// render a neutral "Open a block to see its backlinks" prompt and make no fetch.
    pub active_block_id: Option<String>,
    pub pins: Vec<SidebarBlock>,
    pub favorites: Vec<SidebarBlock>,
    pub backlinks: Vec<BacklinkRow>,
    pub unlinked: Vec<UnlinkedRow>,
    pub breadcrumbs: Vec<BreadcrumbEntry>,
    /// Per-section in-flight flag (impl-note 70): a section's spinner animates ONLY while its genuine
    /// fetch is dispatched. Absent => not loading.
    pub loading_section: HashSet<SectionKind>,
    /// Per-section error (AC9): an error shows an inline banner + Retry in THAT section only.
    pub error_section: HashMap<SectionKind, String>,
    /// Per-section generation counter (RISK-2): bumped on each reload; a stale delivery for an older
    /// generation is dropped by the host. Exposed so the host can compare on delivery.
    pub generation: HashMap<SectionKind, u64>,
    /// The stable egui::Id salt for this panel's persistent collapse state (RISK-5). Defaults to a fixed
    /// salt; the host may set a unique salt when multiple sidebars coexist.
    pub id_salt: String,
}

impl Default for LoomSidebarPanel {
    fn default() -> Self {
        Self {
            workspace_id: String::new(),
            active_block_id: None,
            pins: Vec::new(),
            favorites: Vec::new(),
            backlinks: Vec::new(),
            unlinked: Vec::new(),
            breadcrumbs: Vec::new(),
            loading_section: HashSet::new(),
            error_section: HashMap::new(),
            generation: HashMap::new(),
            id_salt: "loom-sidebar".to_owned(),
        }
    }
}

impl LoomSidebarPanel {
    /// A fresh panel for `workspace_id` with nothing loaded yet.
    pub fn new(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            ..Self::default()
        }
    }

    /// Push a block onto the breadcrumb history (a navigation step). De-dups a consecutive repeat (opening
    /// the same block twice in a row adds no crumb) and caps the history at [`MAX_BREADCRUMBS`], dropping
    /// the OLDEST (RISK-3 / MC-3). Pure (no egui) so PROOF1/PROOF5 test it standalone.
    pub fn push_breadcrumb(&mut self, block_id: impl Into<String>, title: impl Into<String>) {
        let entry = BreadcrumbEntry::new(block_id, title);
        if self.breadcrumbs.last().map(|b| &b.block_id) == Some(&entry.block_id) {
            // Re-opening the current block is a no-op for the trail.
            return;
        }
        self.breadcrumbs.push(entry);
        // Cap to the last MAX_BREADCRUMBS, dropping the oldest.
        if self.breadcrumbs.len() > MAX_BREADCRUMBS {
            let overflow = self.breadcrumbs.len() - MAX_BREADCRUMBS;
            self.breadcrumbs.drain(0..overflow);
        }
    }

    /// Bump a section's generation counter and return the NEW value (RISK-2). The host calls this when it
    /// dispatches a reload, stamps the spawned fetch with the returned generation, and on delivery drops
    /// the result if `self.generation[section]` has since advanced past the stamped value.
    pub fn bump_generation(&mut self, section: SectionKind) -> u64 {
        let g = self.generation.entry(section).or_insert(0);
        *g += 1;
        *g
    }

    /// The current generation for a section (0 if never bumped). Used by the host's stale-drop check.
    pub fn current_generation(&self, section: SectionKind) -> u64 {
        self.generation.get(&section).copied().unwrap_or(0)
    }

    /// Install Pins from a `GET /loom/views/pins` result, clearing that section's loading/error.
    pub fn set_pins(&mut self, pins: Vec<SidebarBlock>) {
        self.pins = pins;
        self.loading_section.remove(&SectionKind::Pins);
        self.error_section.remove(&SectionKind::Pins);
    }

    /// Install Favorites from a `GET /loom/views/favorites` result, clearing loading/error.
    pub fn set_favorites(&mut self, favorites: Vec<SidebarBlock>) {
        self.favorites = favorites;
        self.loading_section.remove(&SectionKind::Favorites);
        self.error_section.remove(&SectionKind::Favorites);
    }

    /// Install Backlinks from a `GET /loom/blocks/{id}/backlinks` result, clearing loading/error. The
    /// Unlinked list is automatically deduped against backlinks at render time ([`visible_unlinked`]),
    /// so the two sections never show the same block twice (RISK-4 / MC-4).
    pub fn set_backlinks(&mut self, backlinks: Vec<BacklinkRow>) {
        self.backlinks = backlinks;
        self.loading_section.remove(&SectionKind::Backlinks);
        self.error_section.remove(&SectionKind::Backlinks);
    }

    /// Install Unlinked mentions from a `GET /loom/blocks/{id}/unlinked-mentions` result, clearing
    /// loading/error.
    pub fn set_unlinked(&mut self, unlinked: Vec<UnlinkedRow>) {
        self.unlinked = unlinked;
        self.loading_section.remove(&SectionKind::Unlinked);
        self.error_section.remove(&SectionKind::Unlinked);
    }

    /// Record a section error (AC9): set the inline banner + Retry and clear that section's loading flag.
    pub fn set_error(&mut self, section: SectionKind, message: impl Into<String>) {
        self.error_section.insert(section, message.into());
        self.loading_section.remove(&section);
    }

    /// Mark a section as in-flight (the host calls this when it dispatches the fetch).
    pub fn set_loading(&mut self, section: SectionKind) {
        self.loading_section.insert(section);
        self.error_section.remove(&section);
    }

    /// Optimistically remove a pin from the local list (RISK-1): the row disappears immediately on
    /// remove-click; the host then runs the two-call backend removal and, on FAILURE, calls
    /// [`rollback_pin`](Self::rollback_pin) to re-insert it. Returns the removed block (for rollback).
    pub fn optimistic_remove_pin(&mut self, block_id: &str) -> Option<SidebarBlock> {
        if let Some(pos) = self.pins.iter().position(|b| b.block_id == block_id) {
            Some(self.pins.remove(pos))
        } else {
            None
        }
    }

    /// Re-insert a pin removed optimistically (RISK-1 rollback on backend failure). Inserts at `pos`
    /// (clamped) so the row returns to roughly its place.
    pub fn rollback_pin(&mut self, block: SidebarBlock, pos: usize) {
        let pos = pos.min(self.pins.len());
        self.pins.insert(pos, block);
    }

    /// Optimistically remove a favorite from the local list (the favorite peer of `optimistic_remove_pin`).
    pub fn optimistic_remove_favorite(&mut self, block_id: &str) -> Option<SidebarBlock> {
        if let Some(pos) = self.favorites.iter().position(|b| b.block_id == block_id) {
            Some(self.favorites.remove(pos))
        } else {
            None
        }
    }

    /// Re-insert a favorite removed optimistically (rollback on backend failure).
    pub fn rollback_favorite(&mut self, block: SidebarBlock, pos: usize) {
        let pos = pos.min(self.favorites.len());
        self.favorites.insert(pos, block);
    }

    /// The unlinked rows actually shown after deduping against the backlinks list AND the active block
    /// itself (RISK-4 / MC-4). A block that already has a real edge (appears in Backlinks) must NOT also
    /// appear as "unlinked"; the active block can never be its own mention. Pure so MC-4 is unit-testable.
    pub fn visible_unlinked(&self) -> Vec<&UnlinkedRow> {
        let backlinked: HashSet<&str> = self.backlinks.iter().map(|b| b.block_id.as_str()).collect();
        let active = self.active_block_id.as_deref();
        self.unlinked
            .iter()
            .filter(|u| !backlinked.contains(u.block_id.as_str()) && Some(u.block_id.as_str()) != active)
            .collect()
    }

    /// Whether a section is currently expanded. Reads the persistent collapse state from
    /// [`egui::Memory`] keyed by a stable Id (RISK-5 / MC-5); default is expanded (`true`).
    fn is_expanded(&self, ui: &egui::Ui, section: SectionKind) -> bool {
        let id = self.collapse_id(ui, section);
        ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(true)
    }

    /// Persist a section's expanded state into [`egui::Memory`] (RISK-5).
    fn set_expanded(&self, ui: &egui::Ui, section: SectionKind, expanded: bool) {
        let id = self.collapse_id(ui, section);
        ui.ctx().data_mut(|d| d.insert_temp(id, expanded));
    }

    /// The stable egui Id for a section's collapse state (salted by `id_salt` so multiple sidebars do
    /// not collide).
    fn collapse_id(&self, ui: &egui::Ui, section: SectionKind) -> egui::Id {
        ui.id().with(&self.id_salt).with("collapse").with(section.slug())
    }

    /// Render the whole sidebar and return the typed event (if any) this frame produced. Requests a
    /// repaint ONLY for a frame where a genuine section spinner is active (idle-repaint discipline).
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<SidebarEvent> {
        let mut event: Option<SidebarEvent> = None;

        // ── Breadcrumb strip (AC6) ────────────────────────────────────────────────────────────────────
        if let Some(ev) = self.show_breadcrumbs(ui, palette) {
            event = Some(ev);
        }
        ui.separator();

        let mut any_spinner = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for section in SectionKind::ALL {
                    if let Some(ev) = self.show_section(ui, palette, section, &mut any_spinner) {
                        event = Some(ev);
                    }
                }
            });

        // Idle-repaint discipline: only animate while a genuine fetch is in flight.
        if any_spinner {
            ui.ctx().request_repaint();
        }

        event
    }

    /// Render the horizontal breadcrumb strip: `Home > Folder > Block`. Each crumb is a clickable Link
    /// that fires [`SidebarEvent::Open`] (AC6). Truncated to the last [`MAX_BREADCRUMBS`] entries.
    fn show_breadcrumbs(&self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<SidebarEvent> {
        let mut event = None;
        ui.horizontal_wrapped(|ui| {
            // "Home" is a static, non-navigating anchor label.
            ui.colored_label(palette.text_subtle, "Home");
            for (idx, crumb) in self.breadcrumbs.iter().enumerate() {
                ui.colored_label(palette.text_subtle, "›");
                let resp = ui.add(
                    egui::Label::new(
                        egui::RichText::new(crumb.label()).color(palette.accent),
                    )
                    .sense(Sense::click()),
                );
                emit_link_accesskit(ui, resp.id, &breadcrumb_author_id(idx), &crumb.title);
                if resp.clicked() {
                    event = Some(SidebarEvent::Open { block_id: crumb.block_id.clone() });
                }
            }
        });
        event
    }

    /// Render one collapsible section header + (when expanded) its body. Sets `any_spinner` if this
    /// section's body shows a genuine in-flight spinner.
    fn show_section(
        &mut self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        section: SectionKind,
        any_spinner: &mut bool,
    ) -> Option<SidebarEvent> {
        let mut event = None;

        // Collapsible header: a clickable header row toggles the persistent expanded state (RISK-5).
        let expanded = self.is_expanded(ui, section);
        let count = self.section_count(section);
        let arrow = if expanded { "▾" } else { "▸" };
        let header_text = format!("{arrow} {} ({count})", section.title());
        let header = ui.add(
            egui::Label::new(egui::RichText::new(header_text).color(palette.text).strong())
                .sense(Sense::click()),
        );
        if header.clicked() {
            self.set_expanded(ui, section, !expanded);
        }

        if !expanded {
            // A collapsed section renders NO rows -> its rows are absent from the AccessKit tree (AC8).
            ui.add_space(2.0);
            return event;
        }

        ui.indent(section.slug(), |ui| {
            // Error banner + Retry (AC9) — this section only.
            if let Some(err) = self.error_section.get(&section).cloned() {
                ui.horizontal(|ui| {
                    ui.colored_label(palette.error_text, format!("⚠ {err}"));
                    let retry = ui.button("Retry");
                    emit_button_accesskit(ui, retry.id, &section_retry_author_id(section), "Retry");
                    if retry.clicked() {
                        event = Some(SidebarEvent::Retry { section });
                    }
                });
                return;
            }

            // Per-section bounded spinner (impl-note 70): only while a genuine fetch is in flight.
            if self.loading_section.contains(&section) {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading…");
                });
                *any_spinner = true;
                return;
            }

            // Backlinks / Unlinked require an active block; otherwise a neutral prompt, no fetch.
            if section.requires_active_block() && self.active_block_id.is_none() {
                ui.weak("Open a block to see its links");
                return;
            }

            if let Some(ev) = self.show_section_body(ui, palette, section) {
                event = Some(ev);
            }
        });
        ui.add_space(4.0);

        event
    }

    /// The N shown in a section header. For Unlinked it is the DEDUPED count (RISK-4) so the header never
    /// over-counts blocks that are filtered out.
    fn section_count(&self, section: SectionKind) -> usize {
        match section {
            SectionKind::Pins => self.pins.len(),
            SectionKind::Favorites => self.favorites.len(),
            SectionKind::Backlinks => self.backlinks.len(),
            SectionKind::Unlinked => self.visible_unlinked().len(),
        }
    }

    /// Render the rows of an expanded, loaded, non-error section.
    fn show_section_body(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        section: SectionKind,
    ) -> Option<SidebarEvent> {
        let mut event = None;
        match section {
            SectionKind::Pins => {
                if self.pins.is_empty() {
                    ui.weak("No pinned blocks");
                }
                for block in &self.pins {
                    if let Some(ev) = render_bookmark_row(block, ui, palette, BookmarkKind::Pin) {
                        event = Some(ev);
                    }
                }
            }
            SectionKind::Favorites => {
                if self.favorites.is_empty() {
                    ui.weak("No favorites");
                }
                for block in &self.favorites {
                    if let Some(ev) = render_bookmark_row(block, ui, palette, BookmarkKind::Favorite) {
                        event = Some(ev);
                    }
                }
            }
            SectionKind::Backlinks => {
                if self.backlinks.is_empty() {
                    ui.weak("No backlinks");
                }
                for row in &self.backlinks {
                    if let Some(ev) = render_backlink_row(row, ui, palette) {
                        event = Some(ev);
                    }
                }
            }
            SectionKind::Unlinked => {
                let visible = self.visible_unlinked();
                if visible.is_empty() {
                    ui.weak("No unlinked mentions");
                }
                for row in visible {
                    if let Some(ev) = render_unlinked_row(row, ui, palette) {
                        event = Some(ev);
                    }
                }
            }
        }
        event
    }
}

/// Which bookmark section a row belongs to (drives the Remove event + the AccessKit id namespace).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BookmarkKind {
    Pin,
    Favorite,
}

/// Render one Pins/Favorites row: a content-type chip + the title (click => Open) + a Remove button
/// (click => RemovePin/RemoveFavorite). The row is an addressable ListItem (AC7); the Remove button is
/// its own addressable Button (AC2/AC3).
fn render_bookmark_row(
    block: &SidebarBlock,
    ui: &mut egui::Ui,
    palette: &HsPalette,
    kind: BookmarkKind,
) -> Option<SidebarEvent> {
    let mut event = None;
    let chip_color = content_type_color(&block.content_type, palette);

    let (row_id, remove_id, remove_label) = match kind {
        BookmarkKind::Pin => (
            pin_row_author_id(&block.block_id),
            pin_remove_author_id(&block.block_id),
            "Remove pin",
        ),
        BookmarkKind::Favorite => (
            favorite_row_author_id(&block.block_id),
            favorite_remove_author_id(&block.block_id),
            "Remove favorite",
        ),
    };

    let (title_resp, remove_clicked) = ui
        .horizontal(|ui| {
            // The content-type chip (color from the shared theme — no hardcoded hex).
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(CHIP_SWATCH_SIZE), Sense::hover());
            if ui.is_rect_visible(rect) {
                ui.painter().rect_filled(rect, 2.0, chip_color);
            }
            let title = ui.add(
                egui::Label::new(egui::RichText::new(&block.title).color(palette.text))
                    .sense(Sense::click()),
            );
            // Right-aligned Remove button (its own AccessKit node).
            let removed = ui
                .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn = ui.button("✕");
                    emit_button_accesskit(ui, btn.id, &remove_id, remove_label);
                    btn.clicked()
                })
                .inner;
            (title, removed)
        })
        .inner;

    emit_list_item_accesskit(ui, title_resp.id, &row_id, &block.title, &block.content_type);

    if title_resp.clicked() {
        event = Some(SidebarEvent::Open { block_id: block.block_id.clone() });
    }
    if remove_clicked {
        event = Some(match kind {
            BookmarkKind::Pin => SidebarEvent::RemovePin { block_id: block.block_id.clone() },
            BookmarkKind::Favorite => {
                SidebarEvent::RemoveFavorite { block_id: block.block_id.clone() }
            }
        });
    }
    event
}

/// Render one Backlinks row: the source block title + an edge-type label chip (AC4). Click => Open the
/// source block. Addressable ListItem with the `sidebar.backlink.{id}` author_id.
fn render_backlink_row(row: &BacklinkRow, ui: &mut egui::Ui, palette: &HsPalette) -> Option<SidebarEvent> {
    let mut event = None;
    let resp = ui
        .horizontal(|ui| {
            let title = ui.add(
                egui::Label::new(egui::RichText::new(&row.title).color(palette.text))
                    .sense(Sense::click()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(palette.text_subtle, &row.edge_type);
            });
            title
        })
        .inner;
    emit_list_item_accesskit(
        ui,
        resp.id,
        &backlink_row_author_id(&row.block_id),
        &row.title,
        &format!("backlink via {}", row.edge_type),
    );
    if resp.clicked() {
        event = Some(SidebarEvent::Open { block_id: row.block_id.clone() });
    }
    event
}

/// Render one Unlinked-mention row: the source block title (no edge). Click => Open. Addressable
/// ListItem with the `sidebar.unlinked.{id}` author_id (AC7).
fn render_unlinked_row(row: &UnlinkedRow, ui: &mut egui::Ui, palette: &HsPalette) -> Option<SidebarEvent> {
    let mut event = None;
    let resp = ui.add(
        egui::Label::new(egui::RichText::new(&row.title).color(palette.text)).sense(Sense::click()),
    );
    emit_list_item_accesskit(
        ui,
        resp.id,
        &unlinked_row_author_id(&row.block_id),
        &row.title,
        "unlinked mention (no edge)",
    );
    if resp.clicked() {
        event = Some(SidebarEvent::Open { block_id: row.block_id.clone() });
    }
    event
}

// ── AccessKit emit helpers (HBR-SWARM) ───────────────────────────────────────────────────────────────

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

/// Emit a breadcrumb crumb's live AccessKit node (Role::Link + Action::Click + author_id). The contract
/// names a crumb a Link (AC7 "role=Link"); `accesskit::Role::Link` is the field-correct 0.21 variant.
fn emit_link_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Link);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
    });
}

/// Emit a list-row's live AccessKit node: Role::ListItem, label = title, author_id, Click default
/// action, plus an accessible description (the content type / edge type).
fn emit_list_item_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, title: &str, desc: &str) {
    let author = author_id.to_owned();
    let label = title.to_owned();
    let description = desc.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::ListItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_description(description.clone());
        node.add_action(accesskit::Action::Click);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PROOF1 / RISK-3 / MC-3: breadcrumb push caps at 5, dropping the OLDEST.
    #[test]
    fn breadcrumb_push_caps_at_five() {
        let mut panel = LoomSidebarPanel::new("ws");
        for i in 0..8 {
            panel.push_breadcrumb(format!("blk-{i}"), format!("Block {i}"));
        }
        assert_eq!(panel.breadcrumbs.len(), MAX_BREADCRUMBS, "capped at 5 entries");
        // The oldest (blk-0..blk-2) were dropped; the last 5 (blk-3..blk-7) remain, in order.
        let ids: Vec<&str> = panel.breadcrumbs.iter().map(|b| b.block_id.as_str()).collect();
        assert_eq!(ids, ["blk-3", "blk-4", "blk-5", "blk-6", "blk-7"]);
    }

    /// PROOF1: re-opening the current block is a breadcrumb no-op (no consecutive duplicate).
    #[test]
    fn breadcrumb_dedups_consecutive_repeat() {
        let mut panel = LoomSidebarPanel::new("ws");
        panel.push_breadcrumb("a", "A");
        panel.push_breadcrumb("a", "A");
        panel.push_breadcrumb("b", "B");
        panel.push_breadcrumb("b", "B");
        let ids: Vec<&str> = panel.breadcrumbs.iter().map(|b| b.block_id.as_str()).collect();
        assert_eq!(ids, ["a", "b"], "consecutive repeats collapse");
    }

    /// MC-4 / RISK-4: unlinked rows are deduped against backlinks AND against the active block itself.
    #[test]
    fn unlinked_dedups_against_backlinks_and_active() {
        let mut panel = LoomSidebarPanel::new("ws");
        panel.active_block_id = Some("active".to_owned());
        panel.set_backlinks(vec![BacklinkRow::new("b1", "Linked One", "mention")]);
        panel.set_unlinked(vec![
            UnlinkedRow::new("b1", "Linked One"),     // also a backlink -> filtered
            UnlinkedRow::new("active", "Active Self"), // the active block -> filtered
            UnlinkedRow::new("u2", "Unlinked Two"),    // genuine unlinked -> kept
        ]);
        let visible: Vec<&str> = panel.visible_unlinked().iter().map(|u| u.block_id.as_str()).collect();
        assert_eq!(visible, ["u2"], "only the genuine unlinked block survives the dedup");
        assert_eq!(panel.section_count(SectionKind::Unlinked), 1, "header count is the deduped count");
    }

    /// RISK-1: optimistic pin removal + rollback restores the row on a simulated backend failure.
    #[test]
    fn optimistic_remove_pin_then_rollback() {
        let mut panel = LoomSidebarPanel::new("ws");
        panel.set_pins(vec![
            SidebarBlock::new("p1", "Pin One", "note"),
            SidebarBlock::new("p2", "Pin Two", "note"),
            SidebarBlock::new("p3", "Pin Three", "note"),
        ]);
        let removed = panel.optimistic_remove_pin("p2").expect("p2 was present");
        assert_eq!(panel.pins.len(), 2, "row removed optimistically");
        assert!(!panel.pins.iter().any(|b| b.block_id == "p2"));
        // Simulated backend failure -> rollback re-inserts at the original index.
        panel.rollback_pin(removed, 1);
        let ids: Vec<&str> = panel.pins.iter().map(|b| b.block_id.as_str()).collect();
        assert_eq!(ids, ["p1", "p2", "p3"], "rollback restores the row in place");
    }

    /// RISK-2: the generation counter monotonically increases per section, independently.
    #[test]
    fn generation_counter_is_monotonic_per_section() {
        let mut panel = LoomSidebarPanel::new("ws");
        assert_eq!(panel.current_generation(SectionKind::Backlinks), 0);
        let g1 = panel.bump_generation(SectionKind::Backlinks);
        let g2 = panel.bump_generation(SectionKind::Backlinks);
        assert_eq!((g1, g2), (1, 2), "backlinks generation increments");
        // Unlinked is independent.
        assert_eq!(panel.bump_generation(SectionKind::Unlinked), 1);
        assert_eq!(panel.current_generation(SectionKind::Backlinks), 2, "sections are independent");
    }

    /// truncate_label cuts long titles with an ellipsis and leaves short ones intact (impl-note 72).
    #[test]
    fn truncate_label_caps_long_titles() {
        assert_eq!(truncate_label("short", 20), "short");
        let long = "a-very-long-block-title-way-past-twenty";
        let t = truncate_label(long, BREADCRUMB_LABEL_MAX);
        assert_eq!(t.chars().count(), BREADCRUMB_LABEL_MAX, "truncated to exactly the cap");
        assert!(t.ends_with('…'), "an ellipsis marks truncation");
    }

    /// AccessKit author_ids are sanitized to `[a-z0-9-]` (no slashes/colons can break the tree).
    #[test]
    fn author_ids_are_sanitized() {
        let row = pin_row_author_id("ws:1/blk 7#x");
        assert!(row.starts_with(PIN_ROW_AUTHOR_ID_PREFIX));
        let suffix = &row[PIN_ROW_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "pin row author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
        assert!(pin_remove_author_id("a/b").ends_with(".remove"));
        assert!(favorite_row_author_id("a:b").starts_with(FAVORITE_ROW_AUTHOR_ID_PREFIX));
        assert!(backlink_row_author_id("a b").starts_with(BACKLINK_ROW_AUTHOR_ID_PREFIX));
        assert!(unlinked_row_author_id("a.b").starts_with(UNLINKED_ROW_AUTHOR_ID_PREFIX));
        assert_eq!(breadcrumb_author_id(3), "sidebar.breadcrumb.3");
        assert_eq!(section_retry_author_id(SectionKind::Pins), "sidebar.pins.retry");
    }

    /// set_* installers clear the matching section's loading + error flags.
    #[test]
    fn set_section_clears_loading_and_error() {
        let mut panel = LoomSidebarPanel::new("ws");
        panel.set_loading(SectionKind::Pins);
        panel.set_error(SectionKind::Pins, "boom");
        assert!(panel.error_section.contains_key(&SectionKind::Pins));
        panel.set_pins(vec![SidebarBlock::new("p1", "P1", "note")]);
        assert!(!panel.loading_section.contains(&SectionKind::Pins));
        assert!(!panel.error_section.contains_key(&SectionKind::Pins));
        assert_eq!(panel.pins.len(), 1);
    }
}
