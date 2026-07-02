//! Per-pane tabbed regions for the native work surface (WP-KERNEL-011 MT-007).
//!
//! ## What this provides
//!
//! Each pane in the MT-005 [`crate::pane_registry::PaneRegistry`] gets a horizontal **tab bar**
//! rendered above its content area (MT-006 split rects). A tab bar lets the operator (or an
//! out-of-process agent) open / close / reorder / pin tabs, shows a dirty-dot on tabs with unsaved
//! state, and supports **dragging a tab from one pane region into another**.
//!
//! ## State model (single source of truth)
//!
//! - [`TabState`]   — one open tab (a [`PaneType`] + optional content id + pinned/dirty flags).
//! - [`TabBarState`] — the ordered list of tabs for ONE pane region plus its `active_index`.
//! - [`TabDragPayload`] / [`TabDropTarget`] — the cross-pane drag/drop contract.
//!
//! `TabBarState` is `Serialize`/`Deserialize` because MT-009 persists per-pane tab state inside the
//! layout snapshot (`tabs` / `activeTab` in the React schema). Inter-pane drag state lives at the
//! [`crate::app::HandshakeApp`] level (not inside `TabBarState`) because a drop crosses pane
//! boundaries — see [`apply_drop`].
//!
//! ## Why hand-rolled tabs over `egui_tiles`
//!
//! `egui_tiles` owns its own tab containers, but it does NOT expose the contract's explicit
//! `TabState { pinned, dirty, content_id }` model, the React `TAB_LABEL_BY_ID` labels, the
//! pin-stabilization ordering, or the per-tab `Role::Tab` AccessKit nodes with the exact
//! `tab-{pane_id}-{index}` author_ids the MT-007 acceptance criteria assert by string. The tab
//! surface is a focused widget over the existing registry; `egui_tiles` remains available for
//! free-form docking where its tree model is the right tool.
//!
//! ## Stable AccessKit ids (out-of-process steering)
//!
//! Per-pane tab-bar **containers** get fixed `NodeId`s in the dedicated 60-63 band (declared in
//! [`crate::accessibility::registry::DECLARED_IDENTITIES`]), strictly below the pane id base (100)
//! and disjoint from chrome (10/20/21) and dividers (30/31). Individual **Tab** and **close-button**
//! nodes are dynamic (their count changes as tabs open/close), so their `egui::Id`s are derived from
//! their stable author_id STRING (`tab-{pane_id}-{index}` / `tab-close-{pane_id}-{index}`) via
//! `egui::Id::new`. That keeps a tab's id stable for a given (pane, index) across frames — the
//! contract's accepted index-based identity — while egui's hashing keeps them clear of the small
//! fixed id band. A model looks up the current tab ids from the live AccessKit snapshot before
//! dispatching an action (indices are recalculated each frame after reorders/closes).

use egui::accesskit;
use serde::{Deserialize, Serialize};

use crate::context_menu::ContextMenu;
use crate::context_menu_surfaces::{tab_action_for_id, tab_context_items, TabMenuAction};
use crate::module_switcher::ModuleId;
use crate::pane_header::module_label_for_tab;
use crate::pane_registry::{PaneId, PaneType};

/// Tab bar height in logical pixels. Matches the React CSS tab bar (`app/src/App.css`).
pub const TAB_BAR_HEIGHT: f32 = 32.0;

/// Radius of the unsaved-state dirty-dot painted to the left of a dirty tab's label.
const DIRTY_DOT_RADIUS: f32 = 2.5;

/// Glyph drawn before a pinned tab's label. `'●'` is used (not the 📌 emoji) for renderer
/// reliability: color-emoji rendering with wgpu is complex and platform-variable, while a filled
/// circle renders consistently everywhere (contract implementation note).
const PIN_GLYPH: &str = "\u{25CF}"; // ●
/// Close-button glyph.
const CLOSE_GLYPH: &str = "\u{00D7}"; // ×

/// AccessKit `NodeId` band base for per-pane tab-bar CONTAINER nodes (`Role::TabList`). The four
/// fixed-position panes (pane-a..pane-d) map to 60..63 by their spatial slot. Strictly below the
/// pane id base (100) and disjoint from chrome (10/20/21) + dividers (30/31). Declared in
/// [`crate::accessibility::registry::DECLARED_IDENTITIES`] so the collision test covers them.
pub const TABBAR_NODE_ID_BASE: u64 = 60;

/// The four fixed pane slots, in stable spatial order, paired with their tab-bar container NodeId.
/// Mirrors the `split_layout` 2x2 grid order so a tab-bar id maps 1:1 to its pane slot.
pub const TABBAR_SLOTS: [(&str, u64); 4] = [
    ("pane-a", TABBAR_NODE_ID_BASE),     // 60
    ("pane-b", TABBAR_NODE_ID_BASE + 1), // 61
    ("pane-c", TABBAR_NODE_ID_BASE + 2), // 62
    ("pane-d", TABBAR_NODE_ID_BASE + 3), // 63
];

/// The fixed tab-bar container NodeId for a pane slot, if it is one of the four grid panes.
/// Dynamic panes added later have no fixed slot and fall back to a hashed id (see [`tabbar_egui_id`]).
pub fn tabbar_node_id(pane_id: &str) -> Option<u64> {
    TABBAR_SLOTS
        .iter()
        .find(|(slot, _)| *slot == pane_id)
        .map(|(_, id)| *id)
}

/// Stable out-of-process author_id for a pane's tab-bar container (`tabbar-{pane_id}`).
pub fn tabbar_author_id(pane_id: &str) -> String {
    format!("tabbar-{pane_id}")
}

/// Stable out-of-process author_id for a single tab (`tab-{pane_id}-{index}`).
pub fn tab_author_id(pane_id: &str, index: usize) -> String {
    format!("tab-{pane_id}-{index}")
}

/// The special stable id the React app gives the pane-a User Manual tab so the diagnostics/UserManual
/// surface keeps ONE agent-stable handle regardless of its tab index (`app/src/App.tsx` lines
/// 1847-1850 `USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID`). Adopted verbatim as the AccessKit author_id for
/// that one tab (MT-013). Every other tab uses the index-based [`tab_author_id`].
pub const USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID: &str = "hs-usermanual-diagnostics-tab";

/// The author_id a tab should carry, honoring the MT-013 pane-a User-Manual override: pane-a's
/// `UserManual` tab gets [`USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID`] instead of the index-based
/// `tab-pane-a-{index}` id; every other (pane, tab) uses [`tab_author_id`]. This keeps the
/// diagnostics surface addressable by ONE stable handle even as it moves index (the React contract).
pub fn tab_author_id_for(pane_id: &str, index: usize, pane_type: &PaneType) -> String {
    if pane_id == "pane-a" && *pane_type == PaneType::UserManual {
        USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID.to_owned()
    } else {
        tab_author_id(pane_id, index)
    }
}

/// Stable out-of-process author_id for a tab's close button (`tab-close-{pane_id}-{index}`).
pub fn tab_close_author_id(pane_id: &str, index: usize) -> String {
    format!("tab-close-{pane_id}-{index}")
}

/// Stable `egui::Id` for a pane's tab-bar container. For the four fixed grid panes this is the
/// fixed-value id (so its AccessKit `NodeId` equals [`tabbar_node_id`]); for any other pane it is
/// derived from the author_id string so it is still stable across frames.
pub fn tabbar_egui_id(pane_id: &str) -> egui::Id {
    match tabbar_node_id(pane_id) {
        // # Safety: a single hand-assigned, never-reused fixed id (60..63) cannot self-collide;
        // entropy only affects egui's child IdMap distribution. The band is disjoint from all other
        // declared ids by construction (see TABBAR_NODE_ID_BASE doc).
        Some(node_id) => unsafe { egui::Id::from_high_entropy_bits(node_id) },
        None => egui::Id::new(tabbar_author_id(pane_id)),
    }
}

/// A single open tab within a pane region. Ported from the React `OpenDocumentTab`
/// (`app/src/App.tsx`): `documentId` is generalized to `content_id` so a tab can address any pane
/// content (document / block / canvas), and `pinned` / `dirty` carry over directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabState {
    /// The surface this tab hosts. Drives the default label via [`PaneType::default_label`].
    pub pane_type: PaneType,
    /// The content this tab shows (document_id / block_id), when the surface has content. `None`
    /// for content-less surfaces (e.g. a Problems panel).
    pub content_id: Option<String>,
    /// Pinned tabs render first (see [`TabBarState::stabilize_pins`]) and cannot be closed.
    pub pinned: bool,
    /// Unsaved-state flag; shows the dirty-dot indicator.
    pub dirty: bool,
    /// Display-label override. `None` uses [`PaneType::default_label`].
    pub label_override: Option<String>,
}

impl TabState {
    /// Construct a clean, unpinned tab for a surface with no content id.
    pub fn new(pane_type: PaneType) -> Self {
        Self {
            pane_type,
            content_id: None,
            pinned: false,
            dirty: false,
            label_override: None,
        }
    }

    /// The display label: the override if set, else the surface's default label.
    pub fn label(&self) -> String {
        self.label_override
            .clone()
            .unwrap_or_else(|| self.pane_type.default_label().to_owned())
    }

    /// The de-duplication key: a tab is uniquely identified by its `(pane_type, content_id)` pair,
    /// matching React `uniqueOpenDocumentTabs()`. Two tabs with the same surface + content are the
    /// same logical tab and must not both exist in one bar.
    fn dedup_key(&self) -> (PaneType, Option<String>) {
        (self.pane_type.clone(), self.content_id.clone())
    }
}

/// The tab bar state for one pane region: an ordered list of tabs plus the active index.
///
/// Invariants maintained by every mutating method:
/// - `active_index < tabs.len()` whenever `tabs` is non-empty; `active_index == 0` when empty.
/// - pinned tabs always occupy the front of `tabs` (enforced by [`stabilize_pins`]).
/// - no two tabs share a `(pane_type, content_id)` key (enforced by [`new`] + [`insert_tab`]).
///
/// [`stabilize_pins`]: TabBarState::stabilize_pins
/// [`new`]: TabBarState::new
/// [`insert_tab`]: TabBarState::insert_tab
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabBarState {
    pub pane_id: PaneId,
    pub tabs: Vec<TabState>,
    pub active_index: usize,
}

impl TabBarState {
    /// Build a tab bar from an initial tab list, de-duplicated by `(pane_type, content_id)`
    /// (React `uniqueTabs()`), pin-stabilized so pinned tabs lead, and with `active_index` clamped
    /// into range (0 when empty).
    pub fn new(pane_id: PaneId, initial_tabs: Vec<TabState>) -> Self {
        let mut state = Self {
            pane_id,
            tabs: dedup_tabs(initial_tabs),
            active_index: 0,
        };
        state.stabilize_pins();
        state.clamp_active();
        state
    }

    /// The currently active tab, if any.
    pub fn active(&self) -> Option<&TabState> {
        self.tabs.get(self.active_index)
    }

    /// Activate the tab at `index` if it exists. Out-of-range indices are ignored (no panic).
    pub fn activate(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = index;
        }
    }

    /// Move keyboard/active focus to the previous tab (wraps to the last). No-op when empty.
    pub fn activate_prev(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_index = (self.active_index + self.tabs.len() - 1) % self.tabs.len();
    }

    /// Move keyboard/active focus to the next tab (wraps to the first). No-op when empty.
    pub fn activate_next(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.active_index = (self.active_index + 1) % self.tabs.len();
    }

    /// Close the tab at `index`.
    ///
    /// - **Pinned tabs are protected**: closing a pinned tab is a no-op (returns `false`) so a tab
    ///   cannot be lost by accident. The contract requires this to never panic.
    /// - When the closed tab WAS the active one, the nearest remaining tab is activated: the tab that
    ///   slides into the freed slot (the former RIGHT neighbour) stays active at the SAME index, so
    ///   `close_tab(1)` on `[A, B, C]` active=1 yields `[A, C]` active=1 (C is now at index 1, per the
    ///   MT-007 acceptance criterion). When the closed active tab was the LAST tab there is no right
    ///   neighbour, so the new last tab (the left neighbour) becomes active.
    /// - When a tab BEFORE the active one is closed, `active_index` shifts left to keep pointing at
    ///   the same logical tab.
    ///
    /// Returns `true` if a tab was removed.
    pub fn close_tab(&mut self, index: usize) -> bool {
        let Some(tab) = self.tabs.get(index) else {
            return false;
        };
        if tab.pinned {
            return false; // protected: pinned tabs cannot be closed
        }

        let was_active = index == self.active_index;
        self.tabs.remove(index);
        self.fix_active_after_remove(index, was_active);
        true
    }

    /// Recompute `active_index` after the tab at `removed_index` was removed. Shared by [`close_tab`]
    /// and [`take_tab`] so the "stay on the slid-in right neighbour, fall back to the new last tab"
    /// rule is defined once.
    ///
    /// [`close_tab`]: TabBarState::close_tab
    /// [`take_tab`]: TabBarState::take_tab
    fn fix_active_after_remove(&mut self, removed_index: usize, removed_was_active: bool) {
        if self.tabs.is_empty() {
            self.active_index = 0;
            return;
        }
        if removed_was_active {
            // The former right neighbour slid into `removed_index`; keep it active by staying at the
            // same index, clamped to the new last tab (covers closing the last tab -> left neighbour).
            self.active_index = removed_index.min(self.tabs.len() - 1);
        } else if removed_index < self.active_index {
            // A tab before the active one was removed; shift the active pointer left to follow it.
            self.active_index -= 1;
        }
    }

    /// Reorder the tab at `from` to position `to`, following the moved tab with `active_index`.
    /// Out-of-range `from`/`to` are ignored (no panic). `to` is clamped to the last valid slot.
    pub fn reorder_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || from == to {
            return;
        }
        let to = to.min(self.tabs.len() - 1);
        let moved_is_active = from == self.active_index;
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);

        if moved_is_active {
            self.active_index = to;
        } else {
            // The active tab did not move, but its index may shift because another tab was pulled
            // out from before/after it and reinserted elsewhere.
            self.active_index = reindex_after_move(self.active_index, from, to);
        }
    }

    /// Pin the tab at `index`, then re-stabilize so pinned tabs lead. The previously-active tab
    /// stays active (its index follows the reorder). Out-of-range indices are ignored.
    pub fn pin_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs[index].pinned = true;
        self.stabilize_pins();
    }

    /// Unpin the tab at `index`, then re-stabilize. Out-of-range indices are ignored.
    pub fn unpin_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs[index].pinned = false;
        self.stabilize_pins();
    }

    /// Toggle the dirty flag on the tab at `index`. Out-of-range indices are ignored.
    pub fn set_dirty(&mut self, index: usize, dirty: bool) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.dirty = dirty;
        }
    }

    /// Move all pinned tabs to the front, preserving the relative order WITHIN the pinned group and
    /// WITHIN the unpinned group (a stable partition). `active_index` is updated to keep pointing at
    /// the same logical tab after the partition.
    pub fn stabilize_pins(&mut self) {
        if self.tabs.is_empty() {
            self.active_index = 0;
            return;
        }
        // Identity-track the active tab so we can recover its new index after the stable partition.
        // We tag each tab with its original index, partition, then find where the active one landed.
        let active = self.active_index.min(self.tabs.len() - 1);
        let mut tagged: Vec<(usize, TabState)> = std::mem::take(&mut self.tabs)
            .into_iter()
            .enumerate()
            .collect();
        // Stable partition: pinned first. `sort_by_key` is stable in std, so relative order within
        // each group is preserved.
        tagged.sort_by_key(|(_, t)| !t.pinned); // false(pinned)=0 sorts before true(unpinned)=1
        let new_active = tagged
            .iter()
            .position(|(orig, _)| *orig == active)
            .unwrap_or(0);
        self.tabs = tagged.into_iter().map(|(_, t)| t).collect();
        self.active_index = new_active;
    }

    /// Insert a tab, de-duplicated by `(pane_type, content_id)`: if a tab with the same key already
    /// exists it is activated instead of duplicated (React `uniqueOpenDocumentTabs` semantics).
    /// Returns the index the tab occupies (existing or newly appended), after pin stabilization.
    pub fn insert_tab(&mut self, tab: TabState) -> usize {
        let key = tab.dedup_key();
        if let Some(existing) = self.tabs.iter().position(|t| t.dedup_key() == key) {
            self.active_index = existing;
            return existing;
        }
        self.tabs.push(tab);
        let appended = self.tabs.len() - 1;
        self.active_index = appended;
        self.stabilize_pins();
        // After stabilization the appended tab may have a new index; find it by key.
        self.tabs
            .iter()
            .position(|t| t.dedup_key() == key)
            .unwrap_or(appended)
    }

    /// Insert a tab at a SPECIFIC position (used by inter-pane drop). De-duplicates by key; if the
    /// tab already exists it is activated in place. `insert_before` clamps to the end
    /// ([`usize::MAX`] appends). Pin stabilization runs after insertion so pinned tabs stay leading.
    pub fn insert_tab_at(&mut self, tab: TabState, insert_before: usize) -> usize {
        let key = tab.dedup_key();
        if let Some(existing) = self.tabs.iter().position(|t| t.dedup_key() == key) {
            self.active_index = existing;
            return existing;
        }
        let pos = insert_before.min(self.tabs.len());
        self.tabs.insert(pos, tab);
        self.active_index = pos;
        self.stabilize_pins();
        self.tabs
            .iter()
            .position(|t| t.dedup_key() == key)
            .unwrap_or(pos)
    }

    /// Remove and return the tab at `index` WITHOUT the pinned-tab protection (used by inter-pane
    /// drag, which moves a tab rather than closing it). Adjusts `active_index` like [`close_tab`].
    /// Returns `None` for an out-of-range index.
    ///
    /// [`close_tab`]: TabBarState::close_tab
    pub fn take_tab(&mut self, index: usize) -> Option<TabState> {
        if index >= self.tabs.len() {
            return None;
        }
        let was_active = index == self.active_index;
        let tab = self.tabs.remove(index);
        self.fix_active_after_remove(index, was_active);
        Some(tab)
    }

    fn clamp_active(&mut self) {
        if self.tabs.is_empty() {
            self.active_index = 0;
        } else if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        }
    }
}

/// De-duplicate a tab list by `(pane_type, content_id)`, preserving first-seen order
/// (React `uniqueTabs()` / `uniqueOpenDocumentTabs()`).
fn dedup_tabs(tabs: Vec<TabState>) -> Vec<TabState> {
    let mut seen: Vec<(PaneType, Option<String>)> = Vec::new();
    let mut out = Vec::with_capacity(tabs.len());
    for tab in tabs {
        let key = tab.dedup_key();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(tab);
    }
    out
}

/// Recompute an index that did NOT itself move, after the tab at `from` was removed and reinserted
/// at `to`. Used to keep `active_index` pointing at the same logical tab through a reorder.
fn reindex_after_move(idx: usize, from: usize, to: usize) -> usize {
    if from < to {
        // Tab pulled from the left and reinserted to the right: everything in (from, to] shifts left.
        if idx > from && idx <= to {
            idx - 1
        } else {
            idx
        }
    } else {
        // Tab pulled from the right and reinserted to the left: everything in [to, from) shifts right.
        if idx >= to && idx < from {
            idx + 1
        } else {
            idx
        }
    }
}

/// Drag payload carried while a tab is being dragged. Must be `Clone + Send + Sync + 'static` for
/// egui's type-safe `DragAndDrop` API (red-team CONTROL: a non-`Send`/`Sync` payload is a compile
/// error, which is the gate). Carries the SOURCE pane + tab index plus a snapshot of the tab so the
/// drop handler can reconstruct the moved [`TabState`] without re-locking the source bar.
#[derive(Debug, Clone, PartialEq)]
pub struct TabDragPayload {
    pub source_pane_id: PaneId,
    pub tab_index: usize,
    pub pane_type: PaneType,
    pub content_id: Option<String>,
    pub pinned: bool,
    pub dirty: bool,
    pub label_override: Option<String>,
}

impl TabDragPayload {
    /// Snapshot a tab into a drag payload.
    pub fn from_tab(source_pane_id: PaneId, tab_index: usize, tab: &TabState) -> Self {
        Self {
            source_pane_id,
            tab_index,
            pane_type: tab.pane_type.clone(),
            content_id: tab.content_id.clone(),
            pinned: tab.pinned,
            dirty: tab.dirty,
            label_override: tab.label_override.clone(),
        }
    }

    /// Reconstruct the [`TabState`] the payload represents.
    pub fn to_tab(&self) -> TabState {
        TabState {
            pane_type: self.pane_type.clone(),
            content_id: self.content_id.clone(),
            pinned: self.pinned,
            dirty: self.dirty,
            label_override: self.label_override.clone(),
        }
    }
}

/// Where a dragged tab should be dropped: into `target_pane_id` at position `insert_before_index`
/// ([`usize::MAX`] = append).
#[derive(Debug, Clone, PartialEq)]
pub struct TabDropTarget {
    pub target_pane_id: PaneId,
    pub insert_before_index: usize,
}

/// Apply a completed inter-pane drag/drop to a pair of tab bars: remove the dragged tab from the
/// SOURCE bar and insert it into the TARGET bar at `insert_before_index`. This is the single mutation
/// point for a cross-pane move so the source-remove and target-insert happen atomically together —
/// preventing the red-team "duplicated or lost tab" failure where two independent handlers mutate in
/// the same frame.
///
/// Same-pane drop (`source == target`) is treated as a reorder within that bar.
///
/// Returns `true` if the move was applied.
pub fn apply_drop(
    payload: &TabDragPayload,
    target: &TabDropTarget,
    source_bar: &mut TabBarState,
    target_bar: &mut TabBarState,
) -> bool {
    debug_assert_eq!(
        source_bar.pane_id, payload.source_pane_id,
        "source bar must match payload"
    );
    debug_assert_eq!(
        target_bar.pane_id, target.target_pane_id,
        "target bar must match target"
    );

    if payload.source_pane_id == target.target_pane_id {
        // Same pane: a reorder, not a cross-pane move. (Callers normally pass the SAME bar twice via
        // `apply_drop_same_pane`; this guard keeps `apply_drop` correct if they don't.)
        source_bar.reorder_tab(payload.tab_index, target.insert_before_index);
        return true;
    }

    let Some(tab) = source_bar.take_tab(payload.tab_index) else {
        return false; // stale index (source changed mid-drag); drop is a no-op rather than a panic
    };
    target_bar.insert_tab_at(tab, target.insert_before_index);
    true
}

/// Apply a same-pane reorder drop (source and target are the one bar). Separate entry point because
/// [`apply_drop`] needs two distinct `&mut` borrows, which Rust forbids for the same bar.
pub fn apply_drop_same_pane(
    payload: &TabDragPayload,
    target: &TabDropTarget,
    bar: &mut TabBarState,
) {
    bar.reorder_tab(payload.tab_index, target.insert_before_index);
}

/// The interactions a single frame of a tab bar produced, surfaced to the caller so the app can wire
/// them to registry / cross-pane state. All `Option`/`Vec` fields are `None`/empty when nothing
/// happened that frame.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TabBarResponse {
    /// A tab was clicked to activate it (index AFTER any same-frame reorder).
    pub activated_index: Option<usize>,
    /// A tab's close button was clicked (index into the bar at render time).
    pub closed_index: Option<usize>,
    /// A pin/unpin was requested (index, new pinned state).
    pub pin_toggled: Option<(usize, bool)>,
    /// A drag of a tab STARTED this frame (source index).
    pub drag_started: Option<usize>,
    /// A drop COMPLETED on this bar this frame: (payload, drop target).
    pub drop_completed: Option<(TabDragPayload, TabDropTarget)>,
    /// MT-020 context menu: "Close Others" was chosen for this tab (keep `index`, close the rest).
    pub close_others_index: Option<usize>,
    /// MT-020 context menu: "Close All" was chosen (close every tab in this bar).
    pub close_all: bool,
    /// MT-020 context menu: "Pop Out" was chosen for the pane hosting this bar (the app pops the pane
    /// into its own OS window). Carries the right-clicked tab index for symmetry; the pop-out acts on
    /// the whole pane (MT-008 pops a pane, not a single tab — see the MT-020 deviation note).
    pub pop_out_requested: bool,
}

impl TabBarResponse {
    fn any(&self) -> bool {
        self.activated_index.is_some()
            || self.closed_index.is_some()
            || self.pin_toggled.is_some()
            || self.drag_started.is_some()
            || self.drop_completed.is_some()
            || self.close_others_index.is_some()
            || self.close_all
            || self.pop_out_requested
    }
}

/// Colors the tab bar paints with, sourced from the active theme tokens by the caller so the tab bar
/// never reads egui's generic visuals for its themed surfaces (mirrors `split_layout::DividerColors`).
#[derive(Debug, Clone, Copy)]
pub struct TabBarColors {
    /// Background fill of the active tab.
    pub active_bg: egui::Color32,
    /// Background fill of an inactive tab (and the bar itself).
    pub inactive_bg: egui::Color32,
    /// Label text color.
    pub text: egui::Color32,
    /// Dirty-dot + pin glyph accent color.
    pub accent: egui::Color32,
    /// Drop-zone highlight while a tab is hovering over this bar.
    pub drop_highlight: egui::Color32,
}

/// Stateless renderer for one pane's tab bar. Borrows the [`TabBarState`] at `show` time and owns
/// nothing (mirrors `split_layout::SplitLayoutWidget`).
pub struct TabBar;

impl TabBar {
    /// Render `state`'s tab bar into `ui` and return the interactions it produced.
    ///
    /// Rendering:
    /// - a horizontal scroll area of tab buttons, each showing (optional) pin glyph, (optional)
    ///   dirty-dot, the label, and a close button (`×`) that is hidden for pinned tabs;
    /// - the active tab uses `colors.active_bg`;
    /// - left/right arrow keys cycle the active tab while the tab bar (or one of its tabs) is focused;
    /// - each tab is a `dnd_drag_source` (payload [`TabDragPayload`]) and the whole bar is a
    ///   `dnd_drop_zone` (so a tab from ANOTHER pane can be dropped here).
    ///
    /// AccessKit:
    /// - the bar is a `Role::TabList` node with author_id `tabbar-{pane_id}`;
    /// - each tab is a `Role::Tab` node with author_id `tab-{pane_id}-{index}` + `Action::Click`/
    ///   `Action::Focus`; the active tab is marked selected;
    /// - each close button is a `Role::Button` node with author_id `tab-close-{pane_id}-{index}`.
    ///
    /// `active_module` (MT-013) is the work-surface MODULE the pane is currently showing; it drives
    /// the per-tab module/type BADGE suffix (e.g. `Inference Lab (LAB)`) via
    /// [`crate::pane_header::module_label_for_tab`], and the badge text is also written to the tab
    /// node's AccessKit `description` so a model reads the module without pixels.
    pub fn show(
        ui: &mut egui::Ui,
        state: &TabBarState,
        colors: TabBarColors,
        active_module: ModuleId,
    ) -> TabBarResponse {
        let mut response = TabBarResponse::default();
        let pane_id = state.pane_id.as_ref().to_owned();
        let bar_egui_id = tabbar_egui_id(&pane_id);

        // Paint the bar background.
        let bar_rect = ui.available_rect_before_wrap();
        if ui.is_rect_visible(bar_rect) {
            ui.painter().rect_filled(bar_rect, 0.0, colors.inactive_bg);
        }

        // ── Drop zone: a tab dragged from ANY pane can be dropped onto this bar ──────────────────
        // dnd_drop_zone draws the inner content and reports whether a payload was released over it.
        // The drop zone's accept rect is the rect its CONTENT allocates, so the content must span the
        // FULL tab-bar strip — otherwise releasing a tab over the empty part of the strip (past the
        // last tab) would miss the zone. We force the content to claim the full bar width + height.
        let frame = egui::Frame::default().inner_margin(egui::Margin::symmetric(4, 2));
        let bar_width = bar_rect.width();
        let bar_height = bar_rect.height();
        let (_, dropped) = ui.dnd_drop_zone::<TabDragPayload, _>(frame, |ui| {
            ui.set_min_width((bar_width - 8.0).max(0.0)); // minus inner margin so it fits the strip
            ui.set_min_height((bar_height - 4.0).max(0.0));
            ui.horizontal(|ui| {
                egui::ScrollArea::horizontal()
                    .id_salt(("tab-bar-scroll", &pane_id))
                    .show(ui, |ui| {
                        Self::render_tabs(
                            ui,
                            state,
                            &pane_id,
                            colors,
                            active_module,
                            &mut response,
                        );
                    });
            });
        });

        // A payload released over this bar: a cross-pane (or same-pane) move COMPLETED. Default drop
        // position is append (usize::MAX); a future MT can refine to an insertion gap from the
        // pointer x. Recording the completed drop here is the single place the app reconciles the
        // source/target bars (see `apply_drop`), avoiding the duplicated/lost-tab race.
        if let Some(payload) = dropped {
            let target = TabDropTarget {
                target_pane_id: state.pane_id.clone(),
                insert_before_index: usize::MAX,
            };
            response.drop_completed = Some(((*payload).clone(), target));
        }

        // ── Keyboard arrow-key tab cycling (only while the bar or a tab is focused) ──────────────
        // Gated on focus so arrow keys never steal input from a focused widget inside the pane body
        // (mirrors the divider keyboard gate in split_layout).
        let bar_focused = ui.memory(|m| {
            m.focused().is_some_and(|f| {
                f == bar_egui_id
                    || state
                        .tabs
                        .iter()
                        .enumerate()
                        .any(|(i, t)| f == Self::tab_egui_id(&pane_id, i, &t.pane_type))
            })
        });
        if bar_focused {
            let (mut left, mut right) = (false, false);
            ui.input(|i| {
                left = i.key_pressed(egui::Key::ArrowLeft);
                right = i.key_pressed(egui::Key::ArrowRight);
            });
            if left {
                let mut next = state.active_index;
                if !state.tabs.is_empty() {
                    next = (state.active_index + state.tabs.len() - 1) % state.tabs.len();
                }
                response.activated_index = Some(next);
            } else if right {
                let mut next = state.active_index;
                if !state.tabs.is_empty() {
                    next = (state.active_index + 1) % state.tabs.len();
                }
                response.activated_index = Some(next);
            }
        }

        // ── Live AccessKit: the bar container is a TabList ───────────────────────────────────────
        // Register the bar id on its rect first so the node attaches under the correct parent, then
        // enrich it (Role::TabList + author_id). Sense::focusable so keyboard nav can land on the bar.
        ui.interact(
            bar_rect,
            bar_egui_id,
            egui::Sense::focusable_noninteractive(),
        );
        ui.ctx().accesskit_node_builder(bar_egui_id, |node| {
            node.set_role(accesskit::Role::TabList);
            node.set_author_id(tabbar_author_id(&pane_id));
            node.set_label(format!("Tab bar for {pane_id}"));
        });

        let _ = response.any(); // response is returned; `any` is a test/debug helper.
        response
    }

    /// Stable `egui::Id` for a single tab. Derived from the (override-aware) author_id string so the
    /// live egui/AccessKit node carries the SAME id the author_id implies — including the MT-013
    /// pane-a User-Manual override ([`USERMANUAL_DIAGNOSTICS_TAB_STABLE_ID`]). Dynamic count keeps it
    /// clear of the fixed id band used by chrome/dividers/tab-bar-containers.
    fn tab_egui_id(pane_id: &str, index: usize, pane_type: &PaneType) -> egui::Id {
        egui::Id::new(tab_author_id_for(pane_id, index, pane_type))
    }

    /// Stable `egui::Id` for a tab's close button.
    fn close_egui_id(pane_id: &str, index: usize) -> egui::Id {
        egui::Id::new(tab_close_author_id(pane_id, index))
    }

    /// Render every tab button into the (already horizontal) `ui`, recording interactions into
    /// `response`.
    fn render_tabs(
        ui: &mut egui::Ui,
        state: &TabBarState,
        pane_id: &str,
        colors: TabBarColors,
        active_module: ModuleId,
        response: &mut TabBarResponse,
    ) {
        for (index, tab) in state.tabs.iter().enumerate() {
            let is_active = index == state.active_index;
            // The MT-013 module/type badge for this tab: derived from the tab's PaneType + the pane's
            // active module. Rendered as a suffix `(LAB)` after the label and mirrored into the tab
            // node's AccessKit description. Empty for a tab in no module (only Placeholder).
            let module_badge = module_label_for_tab(&tab.pane_type, active_module);
            // Each tab is a horizontal group: [drag-source body] [close button]. The close button is
            // rendered as a SIBLING widget OUTSIDE the drag source so a click on it is never swallowed
            // by the body's drag/click sense (the two rects do not overlap).
            ui.horizontal(|ui| {
                Self::render_tab_body(
                    ui,
                    state,
                    tab,
                    index,
                    is_active,
                    pane_id,
                    colors,
                    module_badge,
                    response,
                );
                if !tab.pinned {
                    Self::render_close_button(ui, &tab.label(), index, pane_id, colors, response);
                }
            });
        }
    }

    /// Render the close button (`×`) for an unpinned tab as a standalone interactive widget and emit
    /// its `Role::Button` AccessKit node (author_id `tab-close-{pane_id}-{index}`). Records a close
    /// request into `response`.
    fn render_close_button(
        ui: &mut egui::Ui,
        label: &str,
        index: usize,
        pane_id: &str,
        colors: TabBarColors,
        response: &mut TabBarResponse,
    ) {
        let close_w = 16.0;
        let height = TAB_BAR_HEIGHT - 6.0;
        let close_id = Self::close_egui_id(pane_id, index);
        // Reserve the space, then interact at the FIXED close_id so the Response, its widget_info
        // (Role::Button + label + Action::Click), the AccessKit bounding box, and the author_id all
        // land on the SAME node — otherwise `allocate_exact_size`'s auto-generated id would carry the
        // label+bbox while a separate empty node carried the author_id (and the close button would be
        // unaddressable / lack a bounding box for pointer clicks).
        let (close_rect, _) =
            ui.allocate_exact_size(egui::vec2(close_w, height), egui::Sense::hover());
        let close_resp = ui.interact(close_rect, close_id, egui::Sense::click());

        if ui.is_rect_visible(close_rect) {
            let close_color = if close_resp.hovered() {
                colors.accent
            } else {
                colors.text
            };
            let cg = ui.painter().layout_no_wrap(
                CLOSE_GLYPH.to_owned(),
                egui::FontId::proportional(14.0),
                close_color,
            );
            ui.painter().galley(
                egui::pos2(
                    close_rect.center().x - cg.size().x * 0.5,
                    close_rect.center().y - cg.size().y * 0.5,
                ),
                cg,
                close_color,
            );
        }

        if close_resp.clicked() {
            response.closed_index = Some(index);
        }
        // AccessKit: the close button is a Role::Button (from widget_info) addressable out-of-process.
        close_resp.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                format!("Close {label}"),
            )
        });
        ui.ctx().accesskit_node_builder(close_id, |node| {
            node.set_author_id(tab_close_author_id(pane_id, index));
        });
    }

    /// Render the tab body (pin glyph, dirty-dot, label) as a drag source and emit its `Role::Tab`
    /// AccessKit node. Records activation / pin-toggle / drag-start into `response`.
    #[allow(clippy::too_many_arguments)]
    fn render_tab_body(
        ui: &mut egui::Ui,
        state: &TabBarState,
        tab: &TabState,
        index: usize,
        is_active: bool,
        pane_id: &str,
        colors: TabBarColors,
        module_badge: &str,
        response: &mut TabBarResponse,
    ) {
        let tab_id = Self::tab_egui_id(pane_id, index, &tab.pane_type);
        let label = tab.label();

        // Width of the body content = [pin glyph] [dirty dot] [label] [module badge suffix].
        let font = egui::FontId::proportional(13.0);
        let label_galley = ui
            .painter()
            .layout_no_wrap(label.clone(), font, colors.text);
        // MT-013 module/type badge, painted as a smaller, subtler `(LAB)` suffix after the label
        // (the contract's "shorter suffix format for space efficiency"). Empty -> no badge, no width.
        let badge_text = if module_badge.is_empty() {
            String::new()
        } else {
            format!(" ({module_badge})")
        };
        let badge_galley = if badge_text.is_empty() {
            None
        } else {
            Some(ui.painter().layout_no_wrap(
                badge_text.clone(),
                egui::FontId::proportional(10.0),
                colors.accent,
            ))
        };
        let badge_w = badge_galley.as_ref().map(|g| g.size().x).unwrap_or(0.0);
        let pad = 6.0;
        let glyph_w = if tab.pinned { 12.0 } else { 0.0 };
        let dot_w = if tab.dirty {
            DIRTY_DOT_RADIUS * 2.0 + 4.0
        } else {
            0.0
        };
        let content_w = pad + glyph_w + dot_w + label_galley.size().x + badge_w + pad;
        let height = TAB_BAR_HEIGHT - 6.0;

        // The body is the drag SOURCE: dragging it produces a TabDragPayload the drop zone consumes.
        let payload = TabDragPayload::from_tab(state.pane_id.clone(), index, tab);
        let inner = ui.dnd_drag_source(tab_id, payload, |ui| {
            // Allocate the body rect for painting only (Sense::hover, NOT click): the dnd_drag_source
            // wrapper adds drag sense at `tab_id`, and the explicit `ui.interact(.., tab_id,
            // click_and_drag)` below adds click sense + the Tab role/author_id. Using Sense::click here
            // would create an extra anonymous clickable node (role Unknown, no author_id) that trips
            // the MT-025 `assert_no_unnamed_interactive` gate.
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(content_w, height), egui::Sense::hover());
            if ui.is_rect_visible(rect) {
                let bg = if is_active {
                    colors.active_bg
                } else {
                    colors.inactive_bg
                };
                ui.painter().rect_filled(rect, 3.0, bg);

                let mut cursor_x = rect.left() + pad;
                let mid_y = rect.center().y;

                // Pin glyph (●) in the accent color, before the label.
                if tab.pinned {
                    let g = ui.painter().layout_no_wrap(
                        PIN_GLYPH.to_owned(),
                        egui::FontId::proportional(10.0),
                        colors.accent,
                    );
                    ui.painter().galley(
                        egui::pos2(cursor_x, mid_y - g.size().y * 0.5),
                        g,
                        colors.accent,
                    );
                    cursor_x += glyph_w;
                }
                // Dirty-dot: a filled circle in the accent color, left of the label.
                if tab.dirty {
                    let dot_x = cursor_x + DIRTY_DOT_RADIUS;
                    ui.painter().circle_filled(
                        egui::pos2(dot_x, mid_y),
                        DIRTY_DOT_RADIUS,
                        colors.accent,
                    );
                    cursor_x += dot_w;
                }
                // Label.
                ui.painter().galley(
                    egui::pos2(cursor_x, mid_y - label_galley.size().y * 0.5),
                    label_galley.clone(),
                    colors.text,
                );
                cursor_x += label_galley.size().x;
                // MT-013 module/type badge suffix `(LAB)` in the accent color, after the label.
                if let Some(bg) = &badge_galley {
                    ui.painter().galley(
                        egui::pos2(cursor_x, mid_y - bg.size().y * 0.5),
                        bg.clone(),
                        colors.accent,
                    );
                }
            }
        });
        let drag_resp = inner.response;
        if drag_resp.drag_started() {
            response.drag_started = Some(index);
        }

        // `dnd_drag_source` senses ONLY drag, so its response never reports `clicked()`. Re-interact
        // the SAME rect and id with click_and_drag so a press-release that did NOT become a drag
        // activates the tab, AND egui derives `Action::Click`/`Action::Focus` on the Tab node for
        // out-of-process steering — WITHOUT dropping the drag sense (click_and_drag keeps both, so the
        // dnd_drag_source drag path above still engages).
        let tab_resp = ui.interact(drag_resp.rect, tab_id, egui::Sense::click_and_drag());
        if tab_resp.clicked() {
            response.activated_index = Some(index);
        }
        // ── MT-020 right-click context menu (replaces the MT-019 ad-hoc stub) ────────────────────────
        // Built from the SHARED MT-019 infra (`context_menu_surfaces::tab_context_items` +
        // `ContextMenu::show_on`) so the menu paints via egui's hardened popup, emits `Role::MenuItem`
        // AccessKit nodes (`ctx-menu.tab.*`) for out-of-process steering, supports keyboard nav, and is
        // CLOSED by default (so the MT-025 default-frame snapshot is unchanged). `show_on` opens on
        // `tab_resp.secondary_clicked()` and reuses the SAME `tab_id` response that already carries the
        // named Tab node, so no new unnamed interactive node is created (MT-025 gate stays green).
        //
        // (pane_id, index) are captured by VALUE into the action mapping at the moment of confirm
        // (red-team control: capture the right-clicked target as owned values, not a live-state ref),
        // and the chosen action is recorded into `response` for the app/split-layout to apply this frame.
        let tab_count = state.tabs.len();
        let menu = ContextMenu::new("tab").items(tab_context_items(index, tab_count, tab.pinned));
        if let Some(confirmed_id) = menu.show_on(&tab_resp) {
            if let Some(action) = tab_action_for_id(confirmed_id) {
                match action {
                    TabMenuAction::Close => response.closed_index = Some(index),
                    TabMenuAction::CloseOthers => response.close_others_index = Some(index),
                    TabMenuAction::CloseAll => response.close_all = true,
                    TabMenuAction::TogglePin => response.pin_toggled = Some((index, !tab.pinned)),
                    TabMenuAction::PopOut => response.pop_out_requested = true,
                }
            }
        }

        // Shift+F10 opens the same context menu when the tab is focused (keyboard parity per the MT-020
        // contract; egui 0.33 has no dedicated Menu/ContextMenu key, so Shift+F10 is the keyboard
        // trigger). Red-team control: gate on `has_focus()` so the key never fires for an unfocused
        // surface and never leaks to global shortcuts. `request_open` uses the SAME popup id `show_on`
        // reads, so the keyboard-opened menu is the identical popup the right-click opens.
        if tab_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift)
        {
            crate::context_menu::request_open(ui.ctx(), tab_resp.id, tab_resp.rect.left_bottom());
        }

        // AccessKit: emit the Tab node enriched with role + author_id + selected state. egui already
        // derived Action::Click/Action::Focus from Sense::click(), so we only ADD identity here
        // (mirrors `emit_interactive_node` for the theme toggle).
        tab_resp.widget_info(|| {
            let mut info = egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                ui.is_enabled(),
                is_active,
                &label,
            );
            info.label = Some(label.clone());
            info
        });
        ui.ctx().accesskit_node_builder(tab_id, |node| {
            node.set_role(accesskit::Role::Tab);
            node.set_author_id(tab_author_id_for(pane_id, index, &tab.pane_type));
            node.set_label(label.clone());
            if is_active {
                node.set_selected(true);
            }
            // Description carries machine-readable, pixel-free metadata: the MT-013 module/type badge
            // and the MT-007 dirty indicator. Combined into one description so a model reads both
            // (e.g. "module: LAB; dirty") without scraping the suffix glyph or the dot.
            let mut desc_parts: Vec<String> = Vec::new();
            if !module_badge.is_empty() {
                desc_parts.push(format!("module: {module_badge}"));
            }
            if tab.dirty {
                desc_parts.push("dirty".to_owned());
            }
            if !desc_parts.is_empty() {
                node.set_description(desc_parts.join("; "));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn pid() -> PaneId {
        Arc::from("pane-a")
    }

    fn tab(pt: PaneType) -> TabState {
        TabState::new(pt)
    }

    fn named_bar(types: &[PaneType]) -> TabBarState {
        TabBarState::new(pid(), types.iter().cloned().map(TabState::new).collect())
    }

    #[test]
    fn new_clamps_active_and_dedups() {
        // Duplicate (Workspace, None) collapses to one tab.
        let bar = TabBarState::new(
            pid(),
            vec![
                tab(PaneType::Workspace),
                tab(PaneType::InferenceLab),
                tab(PaneType::Workspace), // dup of #0
            ],
        );
        assert_eq!(
            bar.tabs.len(),
            2,
            "duplicate (pane_type, content_id) removed"
        );
        assert_eq!(bar.active_index, 0);
    }

    #[test]
    fn close_non_pinned_activates_nearest_remaining() {
        // [A, B, C] active=1; close(1) -> [A, C] with active=1 (C now at index 1).
        let mut bar = named_bar(&[
            PaneType::Workspace,
            PaneType::InferenceLab,
            PaneType::AtelierEditor,
        ]);
        bar.activate(1);
        assert!(bar.close_tab(1));
        assert_eq!(bar.tabs.len(), 2);
        assert_eq!(bar.tabs[0].pane_type, PaneType::Workspace);
        assert_eq!(bar.tabs[1].pane_type, PaneType::AtelierEditor);
        assert_eq!(bar.active_index, 1, "C (now index 1) is active");
    }

    #[test]
    fn close_active_last_tab_falls_back_left() {
        // [A, B, C] active=2; close(2) -> [A, B] active=1 (left neighbour).
        let mut bar = named_bar(&[
            PaneType::Workspace,
            PaneType::InferenceLab,
            PaneType::AtelierEditor,
        ]);
        bar.activate(2);
        assert!(bar.close_tab(2));
        assert_eq!(bar.active_index, 1);
    }

    #[test]
    fn close_before_active_shifts_active_left() {
        // [A, B, C] active=2; close(0) -> [B, C] active=1.
        let mut bar = named_bar(&[
            PaneType::Workspace,
            PaneType::InferenceLab,
            PaneType::AtelierEditor,
        ]);
        bar.activate(2);
        assert!(bar.close_tab(0));
        assert_eq!(bar.active_index, 1);
        assert_eq!(bar.tabs[1].pane_type, PaneType::AtelierEditor);
    }

    #[test]
    fn close_pinned_is_noop() {
        let mut bar = named_bar(&[PaneType::Workspace, PaneType::InferenceLab]);
        bar.pin_tab(1); // pins InferenceLab; stabilize moves it to front
                        // After pin, InferenceLab is index 0. Closing it must be a no-op.
        let pinned_idx = bar.tabs.iter().position(|t| t.pinned).unwrap();
        assert!(
            !bar.close_tab(pinned_idx),
            "closing a pinned tab is a no-op"
        );
        assert_eq!(bar.tabs.len(), 2, "no tab removed");
    }

    #[test]
    fn close_only_tab_empties_without_panic() {
        // Red-team CONTROL: close_tab on a 1-tab pane (active=0) -> empty list, active_index=0.
        let mut bar = named_bar(&[PaneType::Workspace]);
        assert!(bar.close_tab(0));
        assert!(bar.tabs.is_empty());
        assert_eq!(bar.active_index, 0);
    }

    #[test]
    fn pin_moves_to_front_and_follows_active() {
        // [A, B, C(unpinned)] active points at C; pin(2) -> [C(pinned), A, B], active follows C to 0.
        let mut bar = named_bar(&[
            PaneType::Workspace,
            PaneType::InferenceLab,
            PaneType::AtelierEditor,
        ]);
        bar.activate(2); // active = C
        bar.pin_tab(2);
        assert_eq!(bar.tabs[0].pane_type, PaneType::AtelierEditor);
        assert!(bar.tabs[0].pinned);
        assert_eq!(bar.tabs[1].pane_type, PaneType::Workspace);
        assert_eq!(bar.tabs[2].pane_type, PaneType::InferenceLab);
        assert_eq!(
            bar.active_index, 0,
            "active follows the previously-active tab (C) to its new slot"
        );
    }

    #[test]
    fn reorder_zero_to_two_moves_tab() {
        // [A, B, C] reorder(0 -> 2) -> [B, C, A].
        let mut bar = named_bar(&[
            PaneType::Workspace,
            PaneType::InferenceLab,
            PaneType::AtelierEditor,
        ]);
        bar.reorder_tab(0, 2);
        assert_eq!(bar.tabs[0].pane_type, PaneType::InferenceLab);
        assert_eq!(bar.tabs[1].pane_type, PaneType::AtelierEditor);
        assert_eq!(bar.tabs[2].pane_type, PaneType::Workspace);
    }

    #[test]
    fn reorder_active_follows_moved_tab() {
        let mut bar = named_bar(&[
            PaneType::Workspace,
            PaneType::InferenceLab,
            PaneType::AtelierEditor,
        ]);
        bar.activate(0); // A active
        bar.reorder_tab(0, 2); // move A to the end
        assert_eq!(bar.active_index, 2, "active follows the moved tab");
    }

    #[test]
    fn reorder_non_active_reindexes() {
        let mut bar = named_bar(&[
            PaneType::Workspace,
            PaneType::InferenceLab,
            PaneType::AtelierEditor,
        ]);
        bar.activate(2); // C active (index 2)
        bar.reorder_tab(0, 1); // move A between B and C: [B, A, C], C now index 2 still
        assert_eq!(bar.active_index, 2);
        bar.activate(0); // now B at 0
        bar.reorder_tab(2, 0); // move C to front: [C, B, A]; B (was active idx0) -> idx1
        assert_eq!(bar.active_index, 1);
    }

    #[test]
    fn stabilize_preserves_relative_order_within_groups() {
        // [A(pin), B, C(pin), D] -> pinned group [A, C] then unpinned [B, D].
        let bar = TabBarState::new(
            pid(),
            vec![
                {
                    let mut t = tab(PaneType::Workspace);
                    t.pinned = true;
                    t
                },
                tab(PaneType::InferenceLab),
                {
                    let mut t = tab(PaneType::AtelierEditor);
                    t.pinned = true;
                    t
                },
                tab(PaneType::Swarm),
            ],
        );
        // new() already stabilizes.
        let order: Vec<PaneType> = bar.tabs.iter().map(|t| t.pane_type.clone()).collect();
        assert_eq!(
            order,
            vec![
                PaneType::Workspace,     // pinned, first-seen
                PaneType::AtelierEditor, // pinned, second-seen
                PaneType::InferenceLab,  // unpinned, first-seen
                PaneType::Swarm,         // unpinned, second-seen
            ]
        );
    }

    #[test]
    fn drag_drop_moves_tab_across_panes_exactly_once() {
        // Red-team CONTROL: drop of {source: pane-a, index: 1} onto pane-b -> pane-a loses that tab,
        // pane-b gains it exactly once (no duplication, no loss).
        let mut source = TabBarState::new(
            Arc::from("pane-a"),
            vec![
                tab(PaneType::Workspace),
                tab(PaneType::InferenceLab),
                tab(PaneType::AtelierEditor),
            ],
        );
        let mut target = TabBarState::new(Arc::from("pane-b"), vec![tab(PaneType::Problems)]);

        let payload = TabDragPayload::from_tab(source.pane_id.clone(), 1, &source.tabs[1]);
        let drop = TabDropTarget {
            target_pane_id: target.pane_id.clone(),
            insert_before_index: usize::MAX,
        };
        assert!(apply_drop(&payload, &drop, &mut source, &mut target));

        assert_eq!(source.tabs.len(), 2, "source lost exactly one tab");
        assert!(
            !source
                .tabs
                .iter()
                .any(|t| t.pane_type == PaneType::InferenceLab),
            "moved tab no longer in source"
        );
        assert_eq!(target.tabs.len(), 2, "target gained exactly one tab");
        assert_eq!(
            target
                .tabs
                .iter()
                .filter(|t| t.pane_type == PaneType::InferenceLab)
                .count(),
            1,
            "moved tab appears exactly once in target (no duplication)"
        );
    }

    #[test]
    fn drop_with_stale_index_is_noop() {
        // The source changed mid-drag and the payload index is now out of range: a no-op, not a panic.
        let mut source = TabBarState::new(Arc::from("pane-a"), vec![tab(PaneType::Workspace)]);
        let mut target = TabBarState::new(Arc::from("pane-b"), vec![tab(PaneType::Problems)]);
        let payload = TabDragPayload {
            source_pane_id: Arc::from("pane-a"),
            tab_index: 99, // stale
            pane_type: PaneType::Workspace,
            content_id: None,
            pinned: false,
            dirty: false,
            label_override: None,
        };
        let drop = TabDropTarget {
            target_pane_id: target.pane_id.clone(),
            insert_before_index: usize::MAX,
        };
        assert!(!apply_drop(&payload, &drop, &mut source, &mut target));
        assert_eq!(source.tabs.len(), 1);
        assert_eq!(target.tabs.len(), 1);
    }

    #[test]
    fn same_pane_drop_is_a_reorder() {
        let mut bar = named_bar(&[
            PaneType::Workspace,
            PaneType::InferenceLab,
            PaneType::AtelierEditor,
        ]);
        let payload = TabDragPayload::from_tab(bar.pane_id.clone(), 0, &bar.tabs[0]);
        let drop = TabDropTarget {
            target_pane_id: bar.pane_id.clone(),
            insert_before_index: 2,
        };
        apply_drop_same_pane(&payload, &drop, &mut bar);
        assert_eq!(
            bar.tabs[2].pane_type,
            PaneType::Workspace,
            "tab moved to the end"
        );
    }

    #[test]
    fn default_label_matches_react_tab_label_map() {
        // Spot-check the React TAB_LABEL_BY_ID strings the contract enumerates.
        assert_eq!(PaneType::Workspace.default_label(), "Workspace");
        assert_eq!(
            PaneType::MediaDownloader.default_label(),
            "Media Downloader"
        );
        assert_eq!(PaneType::FontManager.default_label(), "Fonts");
        assert_eq!(PaneType::FlightRecorder.default_label(), "Flight Recorder");
        assert_eq!(PaneType::KernelDcc.default_label(), "Kernel DCC");
        assert_eq!(PaneType::InferenceLab.default_label(), "Inference Lab");
        assert_eq!(PaneType::ModelRuntime.default_label(), "Model Runtime");
        assert_eq!(PaneType::Swarm.default_label(), "Swarm");
        assert_eq!(PaneType::UserManual.default_label(), "User Manual");
        assert_eq!(PaneType::SourceControl.default_label(), "Source Control");
        assert_eq!(PaneType::LoomDailyJournal.default_label(), "Journal");
        assert_eq!(PaneType::LoomBlock.default_label(), "Loom Block");
        assert_eq!(PaneType::LoomWikiPage.default_label(), "Wiki Page");
        assert_eq!(PaneType::AtelierEditor.default_label(), "Atelier");
        assert_eq!(PaneType::VisualDebugger.default_label(), "Visual Debugger");
        assert_eq!(
            PaneType::Placeholder("Custom".to_owned()).default_label(),
            "Custom"
        );
    }

    #[test]
    fn label_override_takes_precedence() {
        let mut t = tab(PaneType::Workspace);
        t.label_override = Some("My Project".to_owned());
        assert_eq!(t.label(), "My Project");
    }

    #[test]
    fn serde_round_trips_with_field_names() {
        let bar = named_bar(&[PaneType::Workspace, PaneType::InferenceLab]);
        let json = serde_json::to_string(&bar).expect("serialize");
        assert!(json.contains("\"tabs\""), "has tabs field: {json}");
        assert!(json.contains("\"active_index\""), "has active_index field");
        assert!(json.contains("\"pinned\""), "tab has pinned field");
        assert!(json.contains("\"dirty\""), "tab has dirty field");
        let back: TabBarState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, bar);
    }

    #[test]
    fn tab_payload_is_send_sync_static() {
        // Red-team CONTROL: TabDragPayload MUST be Send + Sync + 'static for egui DragAndDrop.
        // A compile error here is the gate.
        fn assert_send_sync_static<T: Send + Sync + 'static>() {}
        assert_send_sync_static::<TabDragPayload>();
    }

    #[test]
    fn tabbar_node_ids_are_collision_safe() {
        // Tab-bar container ids occupy the fixed 60-63 band, below the pane base and disjoint from
        // chrome (10/20/21) and dividers (30/31).
        let pane_base = crate::accessibility::PANE_NODE_ID_BASE;
        for (slot, id) in TABBAR_SLOTS {
            assert!(
                id < pane_base,
                "tabbar id {id} for {slot} below pane base {pane_base}"
            );
            for fixed in [10_u64, 20, 21, 30, 31] {
                assert_ne!(
                    id, fixed,
                    "tabbar id {id} collides with fixed chrome/divider id {fixed}"
                );
            }
        }
        // The four ids are distinct.
        let ids: Vec<u64> = TABBAR_SLOTS.iter().map(|(_, id)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "tabbar ids unique");
    }

    #[test]
    fn author_id_formats_match_contract() {
        assert_eq!(tabbar_author_id("pane-a"), "tabbar-pane-a");
        assert_eq!(tab_author_id("pane-a", 0), "tab-pane-a-0");
        assert_eq!(tab_close_author_id("pane-a", 2), "tab-close-pane-a-2");
    }
}
