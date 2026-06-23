//! Per-surface context-menu item builders + typed dispatch (WP-KERNEL-011 MT-020, C5 part 1).
//!
//! ## What this module is
//!
//! MT-019 ([`crate::context_menu`]) built the SHARED right-click infrastructure: the typed
//! [`ContextMenuItem`] model, the [`ContextMenu`] builder, `show_on` (open-on-secondary-click +
//! dispatch the confirmed item's stable id + emit `Role::MenuItem` AccessKit nodes), keyboard nav,
//! and the memory open/dismiss helpers. MT-019 deliberately stopped at the primitive: it returns the
//! confirmed item id and leaves "which items does THIS surface show" + "what real action does this id
//! perform" to the per-surface caller (the contract's "Item ids are namespaced by the caller").
//!
//! This module is that per-surface layer for the four right-clickable surfaces that EXIST in the
//! native shell today and can dispatch into REAL existing app actions:
//!
//! 1. **pane tab** — the MT-007 per-pane tab chips ([`crate::tab_bar`]);
//! 2. **pane header** — the MT-013 pane binding header ([`crate::pane_header`]);
//! 3. **project tab** — the MT-011 top workspace tabs ([`crate::project_tabs`]);
//! 4. **explorer row** — the MT-014 project-tree document / canvas / bookmark rows
//!    ([`crate::project_tree`]). These ARE interactive rows that already carry a stable author_id and a
//!    `block_id`/`content_id`, so the menu dispatches into real existing actions (open the row, copy its
//!    id/path, and — for BOOKMARK rows only — rename its Loom block via the verified backend PATCH).
//!    Document and canvas rows carry a document/canvas id, NOT a Loom-block id, so their rename item is
//!    rendered DISABLED + disclosed (a document/canvas id PATCHed at `/loom/blocks/{id}` would 404 —
//!    different id space; document rename needs a future document endpoint).
//!
//! Each surface gets (a) a pure `*_context_items(...)` builder that returns the typed item list for a
//! given live state, and (b) a pure `*_action_for_id(...)` mapper that turns a confirmed stable id
//! into a typed [`TabMenuAction`] / [`PaneHeaderMenuAction`] / [`ProjectTabMenuAction`]. The builders
//! and mappers hold NO egui state and perform NO mutation — dispatch happens in the calling widget /
//! the app, exactly as the MT-020 implementation note requires ("Keep action dispatch OUT of the
//! builder functions … Dispatch happens in a root-level handler").
//!
//! ## Honest enable/disable (no fake-enable)
//!
//! Every item the surface conceptually offers is RENDERED, but an item whose target action does not
//! exist yet in this crate is rendered DISABLED with a disclosed reason (MT-019
//! [`ContextMenuItem::disabled`]) instead of being silently dropped or fake-enabled. The two examples
//! in scope:
//!
//! - **per-tab / per-pane split** (`*.split_right` / `*.split_down`): MT-006 ([`crate::split_layout`])
//!   models the work surface as a FIXED 2x2 grid driven by two `SplitWeights` dividers. There is no
//!   "split this pane into a child pane and move the tab into it" primitive in MT-006, so split is a
//!   future-target item — disclosed, not faked.
//! - **set pane type / close pane** (`pane.set_type_*` / `pane.close`): the registry has no
//!   change-pane-type or remove-pane-from-grid operation wired into the live shell yet (the 2x2 grid
//!   is fixed and seeded), so these are disclosed future-target items too.
//!
//! The items that DO map to a real action are enabled: tab close / close-others / close-all,
//! pin/unpin, pop-out; pane-header lock/unlock + pop-out; project-tab activate.
//!
//! ## Stable ids
//!
//! Item ids follow the MT-020 contract's stable id scheme (`tab.close`, `tab.close_others`,
//! `tab.pin`, `pane.lock`, …). They are `&'static str`, asserted against a reference list by a unit
//! test so a typo breaks at CI (red-team "menu item stable ids contain typos"). The MT-019 infra
//! turns each into the AccessKit author_id `ctx-menu.{id}` automatically.
//!
//! ## Scope note (MT-020 part 1, hardened)
//!
//! The MT-020 contract body enumerates two further surfaces:
//!
//! - **explorer row** — NOW WIRED here as Surface 4. A prior pass deferred it on the false premise that
//!   "no file-explorer-row widget exists" — but [`crate::project_tree`] already renders interactive
//!   document/canvas/bookmark rows that carry a stable author_id + a `content_id`/`block_id`. The menu
//!   reuses that EXISTING row identity, so no scaffolding is involved. `open` reuses the existing
//!   `OpenDocument`/`OpenCanvas`/`OpenBookmark` events; `copy_path` copies the row's stable id to the
//!   clipboard; `rename` PATCHes the Loom block title via the verified backend endpoint
//!   `PATCH /workspaces/:id/loom/blocks/:block_id` (body `{ "title": "…" }`) ONLY for BOOKMARK rows,
//!   whose id IS a genuine `LoomBlock.block_id` — document/canvas rows carry a different id space and
//!   render rename DISABLED + disclosed; `reveal_in_graph` is
//!   rendered DISABLED + disclosed because WP-011 has NO graph/loom-view pane surface to reveal into
//!   (no `PaneType::LoomGraph` / graph view exists — verified) — disclosed, not faked.
//! - **editor body** — still genuinely deferred: WP-011 has NO native editor widget (the editor is
//!   WP-012 scope), so there is no surface to attach an editor context menu to. Recorded as a WP-012
//!   deferral rather than scaffolded here.

use crate::context_menu::{ContextMenu, ContextMenuItem, MenuItemKind};

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 1: pane tab
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the pane-tab context menu. Public so the wiring site and the id-audit test reference
/// the SAME constants (a typo cannot drift between builder, dispatcher, and test).
pub mod tab_ids {
    pub const CLOSE: &str = "tab.close";
    pub const CLOSE_OTHERS: &str = "tab.close_others";
    pub const CLOSE_ALL: &str = "tab.close_all";
    pub const PIN: &str = "tab.pin";
    pub const SPLIT_RIGHT: &str = "tab.split_right";
    pub const SPLIT_DOWN: &str = "tab.split_down";
    pub const MOVE_TO_NEW_WINDOW: &str = "tab.move_to_new_window";
    pub const POP_OUT: &str = "tab.pop_out";
}

/// A typed action a confirmed pane-tab menu id maps to. The split/move-to-new-window ids are
/// future-target (their items render disabled), so they have NO action variant — confirming one is
/// impossible (egui ignores clicks on a disabled item). The variants here are exactly the ENABLED
/// actions, which keeps the dispatcher exhaustive over things that can actually fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabMenuAction {
    /// Close the right-clicked tab (`TabBarState::close_tab`).
    Close,
    /// Close every OTHER tab in the pane, keeping the right-clicked one.
    CloseOthers,
    /// Close every tab in the pane.
    CloseAll,
    /// Toggle the right-clicked tab's pinned state (`pin_tab` / `unpin_tab`).
    TogglePin,
    /// Pop the pane out into its own OS window (`HandshakeApp::request_pop_out`).
    PopOut,
}

/// Build the pane-tab context-menu item list for the tab at `tab_index` in a bar of `tab_count` tabs,
/// where `pinned` is that tab's current pinned state.
///
/// Enabled mapping:
/// - Close: always (pinned tabs are protected at the `close_tab` layer, which no-ops — the menu still
///   offers Close so the item set is uniform, and the protection is enforced by the model not the UI).
/// - Close Others: only when there is more than one tab (nothing to close otherwise).
/// - Close All: always.
/// - Pin/Unpin: always; the LABEL reflects the current state (`Unpin` when pinned, else `Pin`).
/// - Pop Out: always.
/// - Split Right / Split Down / Move to New Window: future-target (disabled + disclosed).
pub fn tab_context_items(tab_index: usize, tab_count: usize, pinned: bool) -> Vec<ContextMenuItem> {
    let _ = tab_index; // index is carried by the wiring site; the item set does not depend on it
    let close_others = if tab_count > 1 {
        ContextMenuItem::action(tab_ids::CLOSE_OTHERS, "Close Others")
    } else {
        ContextMenuItem::action(tab_ids::CLOSE_OTHERS, "Close Others")
            .disabled("Only one tab in this pane")
    };
    let pin_label = if pinned { "Unpin" } else { "Pin" };
    ContextMenu::new("tab")
        .item(ContextMenuItem::action(tab_ids::CLOSE, "Close").with_shortcut("Ctrl+W"))
        .item(close_others)
        .item(ContextMenuItem::action(tab_ids::CLOSE_ALL, "Close All"))
        .separator()
        .item(ContextMenuItem::action(tab_ids::PIN, pin_label))
        .separator()
        .item(
            ContextMenuItem::action(tab_ids::SPLIT_RIGHT, "Split Right")
                .disabled("Pane split is a future surface (fixed 2x2 grid)"),
        )
        .item(
            ContextMenuItem::action(tab_ids::SPLIT_DOWN, "Split Down")
                .disabled("Pane split is a future surface (fixed 2x2 grid)"),
        )
        .separator()
        .item(
            ContextMenuItem::action(tab_ids::MOVE_TO_NEW_WINDOW, "Move to New Window")
                .disabled("Use Pop Out (per-tab move-to-window is a future surface)"),
        )
        .item(ContextMenuItem::action(tab_ids::POP_OUT, "Pop Out"))
        .into_items()
}

/// Map a confirmed pane-tab menu id to its typed action, or `None` for an id that has no enabled
/// action (a future-target id, the separator filler, or an unknown id). A future-target item is
/// disabled in [`tab_context_items`], so it can never be confirmed — this mapping returning `None`
/// for it is the belt-and-braces second line of defence.
pub fn tab_action_for_id(id: &str) -> Option<TabMenuAction> {
    match id {
        tab_ids::CLOSE => Some(TabMenuAction::Close),
        tab_ids::CLOSE_OTHERS => Some(TabMenuAction::CloseOthers),
        tab_ids::CLOSE_ALL => Some(TabMenuAction::CloseAll),
        tab_ids::PIN => Some(TabMenuAction::TogglePin),
        tab_ids::POP_OUT => Some(TabMenuAction::PopOut),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 2: pane header
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the pane-header context menu.
pub mod pane_ids {
    pub const LOCK: &str = "pane.lock";
    pub const SPLIT_RIGHT: &str = "pane.split_right";
    pub const SPLIT_DOWN: &str = "pane.split_down";
    pub const SET_TYPE_EDITOR: &str = "pane.set_type_editor";
    pub const SET_TYPE_TERMINAL: &str = "pane.set_type_terminal";
    pub const SET_TYPE_CANVAS: &str = "pane.set_type_canvas";
    pub const SET_TYPE_BROWSER: &str = "pane.set_type_browser";
    pub const POP_OUT: &str = "pane.pop_out";
    pub const CLOSE: &str = "pane.close";
}

/// A typed action a confirmed pane-header menu id maps to. Only the ENABLED ids (lock toggle, pop out)
/// have variants; set-type / split / close are future-target (disabled), so they cannot fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneHeaderMenuAction {
    /// Toggle the pane's lock state (`LockState`), the same mutation the header Lock button performs.
    ToggleLock,
    /// Pop the pane out into its own OS window (`HandshakeApp::request_pop_out`).
    PopOut,
}

/// Build the pane-header context-menu item list. `locked` drives the lock item's label;
/// `is_last_pane` disables `pane.close` when this is the only pane (red-team: closing the last pane
/// would leave a blank window) — though `pane.close` is a future-target item regardless, the
/// last-pane guard is encoded so it stays correct when close-pane is wired.
pub fn pane_header_context_items(locked: bool, is_last_pane: bool) -> Vec<ContextMenuItem> {
    let lock_label = if locked { "Unlock Pane" } else { "Lock Pane" };
    let close_item = ContextMenuItem::action(pane_ids::CLOSE, "Close Pane").disabled(if is_last_pane {
        "Cannot close the only pane"
    } else {
        "Close pane is a future surface (fixed 2x2 grid)"
    });
    ContextMenu::new("pane")
        .item(ContextMenuItem::action(pane_ids::LOCK, lock_label))
        .separator()
        .item(
            ContextMenuItem::action(pane_ids::SPLIT_RIGHT, "Split Right")
                .disabled("Pane split is a future surface (fixed 2x2 grid)"),
        )
        .item(
            ContextMenuItem::action(pane_ids::SPLIT_DOWN, "Split Down")
                .disabled("Pane split is a future surface (fixed 2x2 grid)"),
        )
        .separator()
        .item(
            ContextMenuItem::action(pane_ids::SET_TYPE_EDITOR, "Set Type: Editor")
                .disabled("Set pane type is a future surface"),
        )
        .item(
            ContextMenuItem::action(pane_ids::SET_TYPE_TERMINAL, "Set Type: Terminal")
                .disabled("Set pane type is a future surface"),
        )
        .item(
            ContextMenuItem::action(pane_ids::SET_TYPE_CANVAS, "Set Type: Canvas")
                .disabled("Set pane type is a future surface"),
        )
        .item(
            ContextMenuItem::action(pane_ids::SET_TYPE_BROWSER, "Set Type: Browser")
                .disabled("Set pane type is a future surface"),
        )
        .separator()
        .item(ContextMenuItem::action(pane_ids::POP_OUT, "Pop Out Pane"))
        .item(close_item)
        .into_items()
}

/// Map a confirmed pane-header menu id to its typed action, or `None` for a future-target / unknown id.
pub fn pane_header_action_for_id(id: &str) -> Option<PaneHeaderMenuAction> {
    match id {
        pane_ids::LOCK => Some(PaneHeaderMenuAction::ToggleLock),
        pane_ids::POP_OUT => Some(PaneHeaderMenuAction::PopOut),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 3: project tab
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the project-tab context menu.
pub mod project_ids {
    pub const ACTIVATE: &str = "project.activate";
    pub const CLOSE: &str = "project.close";
    pub const NEW: &str = "project.new";
    pub const RENAME: &str = "project.rename";
}

/// A typed action a confirmed project-tab menu id maps to. Only `Activate` is wired today (it drives
/// the existing `active_project_id` switch the MT-009 lifecycle keys on); close/new/rename need
/// backend workspace CRUD that the native shell does not call yet, so they are future-target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTabMenuAction {
    /// Switch the shell to this project (the same action a left-click on a non-active tab performs).
    Activate,
}

/// Build the project-tab context-menu item list. `is_active` disables `project.activate` for the
/// already-active project (switching to it is a no-op). `project_count` would gate a future
/// "close project" item (you cannot close the last project); close/new/rename are future-target.
pub fn project_tab_context_items(is_active: bool, project_count: usize) -> Vec<ContextMenuItem> {
    let _ = project_count;
    let activate = if is_active {
        ContextMenuItem::action(project_ids::ACTIVATE, "Switch to Project")
            .disabled("Already the active project")
    } else {
        ContextMenuItem::action(project_ids::ACTIVATE, "Switch to Project")
    };
    ContextMenu::new("project")
        .item(activate)
        .separator()
        .item(
            ContextMenuItem::action(project_ids::NEW, "New Project")
                .disabled("Workspace create is a future surface"),
        )
        .item(
            ContextMenuItem::action(project_ids::RENAME, "Rename Project")
                .disabled("Workspace rename is a future surface"),
        )
        .item(
            ContextMenuItem::action(project_ids::CLOSE, "Close Project")
                .disabled("Workspace close is a future surface"),
        )
        .into_items()
}

/// Map a confirmed project-tab menu id to its typed action, or `None` for a future-target / unknown id.
pub fn project_tab_action_for_id(id: &str) -> Option<ProjectTabMenuAction> {
    match id {
        project_ids::ACTIVATE => Some(ProjectTabMenuAction::Activate),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 4: explorer row (project-tree document / canvas / bookmark rows)
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the explorer-row context menu (MT-020 part 1, FIX-A). Item ids follow the contract's
/// `ctx-menu.explorer.{id}` scheme (the MT-019 infra prefixes `ctx-menu.` automatically).
pub mod explorer_ids {
    pub const OPEN: &str = "explorer.open";
    pub const COPY_PATH: &str = "explorer.copy_path";
    pub const RENAME: &str = "explorer.rename";
    pub const REVEAL_IN_GRAPH: &str = "explorer.reveal_in_graph";
    /// WP-KERNEL-012 MT-033 (E5 — route-to-Stage): "Route to Stage" sends the row's document/canvas to
    /// the Stage pane via the shared MT-031 InteractionBus Route-to-Stage command. ENABLED only for a
    /// Document row (the Stage pane's `Document` content variant routes a rich document by id); a Canvas
    /// or Bookmark row's Route-to-Stage is disabled + disclosed (the Stage pane displays a routed
    /// DOCUMENT / selection / CKC item — a canvas/bookmark stage surface is a later MT).
    pub const ROUTE_TO_STAGE: &str = "explorer.route_to_stage";
}

/// The kind of project-tree row a context menu is being built for. Drives which items are ENABLED:
/// only a `Bookmark` row carries a genuine Loom-block id (`LoomBlock.block_id` from the pins view),
/// so it is the ONLY kind whose `rename` maps to the verified Loom-block PATCH target. A `Document`
/// row carries a DOCUMENT id (from `GET /workspaces/:id/documents`) and a `Canvas` row carries a
/// CANVAS id — neither is a Loom-block id, so both have rename disabled + disclosed (PATCHing
/// `/loom/blocks/{document_or_canvas_id}` would 404 at runtime — different id space). `open` is always
/// available (it dispatches the existing open events, which DO key on the document/canvas id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerRowKind {
    /// A Documents-group row (`OpenDocument`). Its id is a DOCUMENT id, NOT a Loom-block id, so rename
    /// is disabled + disclosed (renaming a document needs a document endpoint, a future MT).
    Document,
    /// A Canvases-group row (`OpenCanvas`). Its id is a CANVAS id, not a Loom block, so rename is
    /// disabled + disclosed.
    Canvas,
    /// A Bookmarks-group row (`OpenBookmark`). Backed by a genuine `LoomBlock.block_id`, so rename
    /// PATCHes that block via the verified endpoint.
    Bookmark,
}

/// A typed action a confirmed explorer-row menu id maps to. Only the ENABLED ids have variants:
/// `reveal_in_graph` is future-target (no graph view surface in WP-011), so it has NO variant and can
/// never fire. `Rename` is only produced for a Loom-block-backed BOOKMARK row; document and canvas rows
/// carry a non-Loom-block id, so their rename item is disabled and cannot be confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerMenuAction {
    /// Open the row's content on the active pane (reuses the existing open events).
    Open,
    /// Copy the row's stable id/path to the clipboard (no backend).
    CopyPath,
    /// Rename the row's Loom block via `PATCH /workspaces/:id/loom/blocks/:block_id`.
    Rename,
    /// MT-033: route the row's DOCUMENT to the Stage pane via the MT-031 Route-to-Stage command. Only
    /// produced for a Document row (the Stage pane displays a routed document by id); a Canvas/Bookmark
    /// row's Route-to-Stage item is disabled, so this never fires for them.
    RouteToStage,
}

/// Build the explorer-row context-menu item list for a row of `kind`. ONLY a `Bookmark` row carries a
/// genuine Loom-block id, so it is the only kind whose `rename` is ENABLED (it PATCHes that block via
/// the verified endpoint). A `Document` row's id is a DOCUMENT id and a `Canvas` row's id is a CANVAS
/// id — neither is a Loom-block id, so both have `rename` disabled + disclosed (PATCHing
/// `/loom/blocks/{document_or_canvas_id}` would 404 — different id space). `reveal_in_graph` is always
/// disabled + disclosed: WP-011 has no graph/loom-view pane to reveal into (verified — no such
/// `PaneType`).
pub fn explorer_context_items(kind: ExplorerRowKind) -> Vec<ContextMenuItem> {
    let rename = match kind {
        ExplorerRowKind::Bookmark => ContextMenuItem::action(explorer_ids::RENAME, "Rename"),
        ExplorerRowKind::Document => ContextMenuItem::action(explorer_ids::RENAME, "Rename")
            .disabled("Documents are not Loom blocks; document rename needs a document endpoint (future MT)"),
        ExplorerRowKind::Canvas => ContextMenuItem::action(explorer_ids::RENAME, "Rename")
            .disabled("Canvas rows are not Loom blocks (no rename endpoint)"),
    };
    // MT-033: "Route to Stage" — ENABLED only for a Document row (the Stage pane's `Document` content
    // variant routes a rich document by id). A Canvas/Bookmark row's Route-to-Stage is disabled +
    // disclosed (the Stage pane displays a routed document/selection/CKC item; a canvas/bookmark stage
    // route is a later MT) — honest enable/disable, never faked.
    let route_to_stage = match kind {
        ExplorerRowKind::Document => {
            ContextMenuItem::action(explorer_ids::ROUTE_TO_STAGE, "Route to Stage")
        }
        ExplorerRowKind::Canvas => {
            ContextMenuItem::action(explorer_ids::ROUTE_TO_STAGE, "Route to Stage")
                .disabled("Routing a canvas to the Stage pane is a later MT (Stage displays documents)")
        }
        ExplorerRowKind::Bookmark => {
            ContextMenuItem::action(explorer_ids::ROUTE_TO_STAGE, "Route to Stage")
                .disabled("Routing a bookmark to the Stage pane is a later MT (Stage displays documents)")
        }
    };
    ContextMenu::new("explorer")
        .item(ContextMenuItem::action(explorer_ids::OPEN, "Open"))
        .item(ContextMenuItem::action(explorer_ids::COPY_PATH, "Copy Path"))
        .separator()
        .item(rename)
        .item(
            ContextMenuItem::action(explorer_ids::REVEAL_IN_GRAPH, "Reveal in Graph")
                .disabled("No graph/loom view surface in this build (future)"),
        )
        .separator()
        .item(route_to_stage)
        .into_items()
}

/// Map a confirmed explorer-row menu id to its typed action, honoring the row `kind`: `rename` fires
/// ONLY for a `Bookmark` row (the only kind backed by a genuine Loom-block id). A `Document` or
/// `Canvas` row's `rename` id maps to `None` (its item is disabled, so it cannot be confirmed — this is
/// the belt-and-braces second line of defence that also guarantees a document/canvas id can never reach
/// the Loom-block PATCH). `reveal_in_graph` and unknown ids map to `None`.
pub fn explorer_action_for_id(id: &str, kind: ExplorerRowKind) -> Option<ExplorerMenuAction> {
    match id {
        explorer_ids::OPEN => Some(ExplorerMenuAction::Open),
        explorer_ids::COPY_PATH => Some(ExplorerMenuAction::CopyPath),
        explorer_ids::RENAME => match kind {
            ExplorerRowKind::Bookmark => Some(ExplorerMenuAction::Rename),
            ExplorerRowKind::Document | ExplorerRowKind::Canvas => None,
        },
        // MT-033: Route-to-Stage fires ONLY for a Document row (the Stage pane displays a routed
        // document); a Canvas/Bookmark row's item is disabled, so even a confirmed id maps to None
        // (belt-and-braces second line of defence, mirroring the rename gating).
        explorer_ids::ROUTE_TO_STAGE => match kind {
            ExplorerRowKind::Document => Some(ExplorerMenuAction::RouteToStage),
            ExplorerRowKind::Canvas | ExplorerRowKind::Bookmark => None,
        },
        _ => None,
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-021 (C5 part 2): the SIX additional per-surface menus.
//
// Same shape as the MT-020 surfaces above: a pure `*_context_items(...)` builder returning the typed
// item list for a given live state, plus a pure `*_action_for_id(...)` mapper turning a confirmed
// stable id into a typed action enum. Builders/mappers hold NO egui state and perform NO mutation —
// dispatch happens in the wiring widget. Item ids exactly match the MT-021 contract scope (asserted by
// the reference-list tests below). Honest enable/disable: an item whose target action does not exist
// yet (a V1 stub, or a surface owned by a later WP) renders DISABLED with a disclosed reason rather
// than being dropped or fake-enabled.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 5: Loom graph node
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the Loom-graph-node context menu (MT-021 scope, exact).
pub mod loom_ids {
    pub const OPEN: &str = "loom.open";
    pub const OPEN_TO_SIDE: &str = "loom.open_to_side";
    pub const RENAME: &str = "loom.rename";
    pub const PIN: &str = "loom.pin";
    pub const FAVORITE: &str = "loom.favorite";
    pub const CONNECT: &str = "loom.connect";
    pub const DISCONNECT: &str = "loom.disconnect";
    pub const COPY_BLOCK_ID: &str = "loom.copy_block_id";
    pub const REVEAL_IN_PANEL: &str = "loom.reveal_in_panel";
    pub const DELETE: &str = "loom.delete";
}

/// A typed action a confirmed Loom-node menu id maps to. Only ENABLED ids have variants; the V1 stubs
/// (`connect`/`disconnect`/`delete`) are disabled, so they have NO variant and can never fire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoomNodeMenuAction {
    /// Open the block in a new tab on the active pane.
    Open,
    /// Split right, then open the block (open-to-side).
    OpenToSide,
    /// Rename the block via `PATCH /workspaces/:id/loom/blocks/:block_id` `{title}`.
    Rename,
    /// Toggle `LoomBlock.pinned` via the same PATCH (`{pinned}`). `target` is the NEW value to send
    /// (computed from the freshest cached `pinned` so the toggle always flips the right way —
    /// red-team stale-state control).
    TogglePin { target: bool },
    /// Toggle `LoomBlock.favorite` via the same PATCH (`{favorite}`). `target` is the NEW value.
    ToggleFavorite { target: bool },
    /// Copy the block id to the clipboard (no backend).
    CopyBlockId,
    /// Open / focus the LoomBlock panel pane for the block.
    RevealInPanel,
}

/// The cached Loom-block state the menu builder reads. The pin/favorite LABELS and the toggle TARGET
/// are derived from this FRESH cached copy at build time (red-team: never a stale snapshot), and the
/// disconnect enable-check reads `has_edges` from the already-loaded edge list (no per-right-click
/// backend fetch — contract implementation note).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoomNodeState {
    pub block_id: String,
    pub pinned: bool,
    pub favorite: bool,
    /// True iff the loaded graph view shows at least one edge on this node (drives `disconnect`).
    pub has_edges: bool,
}

/// Build the Loom-graph-node context menu for `state`. Pin/Favorite labels + toggle targets come from
/// the FRESH cached `state` (red-team stale-state control). `disconnect` is enabled iff the node has
/// edges. `connect`/`disconnect`/`delete` are V1 stubs — disabled + disclosed (no fake-enable).
pub fn loom_node_context_items(state: &LoomNodeState) -> Vec<ContextMenuItem> {
    let pin_label = if state.pinned { "Unpin" } else { "Pin" };
    let fav_label = if state.favorite { "Unfavorite" } else { "Favorite" };
    let disconnect = if state.has_edges {
        ContextMenuItem::action(loom_ids::DISCONNECT, "Disconnect")
            .disabled("Disconnect is a future surface (no edge-edit endpoint wired)")
    } else {
        ContextMenuItem::action(loom_ids::DISCONNECT, "Disconnect")
            .disabled("No edges on this node")
    };
    ContextMenu::new("loom")
        .item(ContextMenuItem::action(loom_ids::OPEN, "Open Block"))
        .item(ContextMenuItem::action(loom_ids::OPEN_TO_SIDE, "Open to Side"))
        .separator()
        .item(ContextMenuItem::action(loom_ids::RENAME, "Rename..."))
        .item(ContextMenuItem::action(loom_ids::PIN, pin_label))
        .item(ContextMenuItem::action(loom_ids::FAVORITE, fav_label))
        .separator()
        .item(
            ContextMenuItem::action(loom_ids::CONNECT, "Connect to...")
                .disabled("Connection picker is a future surface (V1 stub)"),
        )
        .item(disconnect)
        .separator()
        .item(ContextMenuItem::action(loom_ids::COPY_BLOCK_ID, "Copy Block ID"))
        .item(ContextMenuItem::action(loom_ids::REVEAL_IN_PANEL, "Reveal in Block Panel"))
        .separator()
        .item(
            ContextMenuItem::action(loom_ids::DELETE, "Delete Block")
                .disabled("Delete needs a confirmation dialog (future surface)"),
        )
        .into_items()
}

/// Map a confirmed Loom-node menu id to its typed action, computing the pin/favorite toggle TARGET from
/// the FRESH cached `state` (so the action always flips the right way even if the menu was open while
/// another pane changed the block). A stub / unknown id maps to `None`.
pub fn loom_node_action_for_id(id: &str, state: &LoomNodeState) -> Option<LoomNodeMenuAction> {
    match id {
        loom_ids::OPEN => Some(LoomNodeMenuAction::Open),
        loom_ids::OPEN_TO_SIDE => Some(LoomNodeMenuAction::OpenToSide),
        loom_ids::RENAME => Some(LoomNodeMenuAction::Rename),
        loom_ids::PIN => Some(LoomNodeMenuAction::TogglePin { target: !state.pinned }),
        loom_ids::FAVORITE => Some(LoomNodeMenuAction::ToggleFavorite { target: !state.favorite }),
        loom_ids::COPY_BLOCK_ID => Some(LoomNodeMenuAction::CopyBlockId),
        loom_ids::REVEAL_IN_PANEL => Some(LoomNodeMenuAction::RevealInPanel),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 6: Canvas board node / card
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the canvas-node context menu (MT-021 scope, exact).
pub mod canvas_ids {
    pub const OPEN_BLOCK: &str = "canvas.open_block";
    pub const EDIT_CARD: &str = "canvas.edit_card";
    pub const CONNECT_TO: &str = "canvas.connect_to";
    pub const ADD_VISUAL_EDGE: &str = "canvas.add_visual_edge";
    pub const REMOVE_EDGES: &str = "canvas.remove_edges";
    pub const MOVE_TO_FRONT: &str = "canvas.move_to_front";
    pub const MOVE_TO_BACK: &str = "canvas.move_to_back";
    pub const COPY_BLOCK_ID: &str = "canvas.copy_block_id";
    pub const REMOVE: &str = "canvas.remove";
    pub const DELETE_BLOCK: &str = "canvas.delete_block";
}

/// Whether a canvas placement references a real LoomBlock or is a text-only card. Drives which items
/// are enabled: a block placement enables `open_block`/`copy_block_id`/`delete_block`; a card enables
/// `edit_card`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasNodeKind {
    /// A placement of a real LoomBlock (has a `block_id`).
    Block,
    /// A text-only canvas card (no `block_id`).
    Card,
}

/// The cached canvas-placement state the menu builder reads: its kind and whether it has any
/// visual-only edges (drives `remove_edges`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasNodeState {
    pub placement_id: String,
    pub kind: CanvasNodeKind,
    /// True iff this placement is an endpoint of at least one VISUAL-only edge on the board.
    pub has_visual_edges: bool,
}

/// A typed action a confirmed canvas-node menu id maps to. The V1 stubs
/// (`connect_to`/`add_visual_edge`/`delete_block`) are disabled, so they have no variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasNodeMenuAction {
    /// Open the placed block in a tab (block placements only).
    OpenBlock,
    /// Enter inline text-edit on the card (card placements only).
    EditCard,
    /// Remove all VISUAL-only edges connected to this placement (never semantic Loom edges).
    RemoveEdges,
    /// PATCH the placement's `z_index` to the front of the board.
    MoveToFront,
    /// PATCH the placement's `z_index` to the back of the board.
    MoveToBack,
    /// Copy the placed block id (block placements only).
    CopyBlockId,
    /// DELETE the placement (the canvas reference), NOT the underlying block.
    Remove,
}

/// Build the canvas-node context menu for `state`. `open_block`/`copy_block_id`/`delete_block` are
/// enabled only for a BLOCK placement; `edit_card` only for a CARD; `remove_edges` only when the
/// placement has visual edges. `connect_to`/`add_visual_edge`/`delete_block` are V1 stubs (disabled).
pub fn canvas_node_context_items(state: &CanvasNodeState) -> Vec<ContextMenuItem> {
    let is_block = matches!(state.kind, CanvasNodeKind::Block);
    let is_card = matches!(state.kind, CanvasNodeKind::Card);

    let open_block = enabled_or(
        ContextMenuItem::action(canvas_ids::OPEN_BLOCK, "Open Block"),
        is_block,
        "This placement is a card (no block to open)",
    );
    let edit_card = enabled_or(
        ContextMenuItem::action(canvas_ids::EDIT_CARD, "Edit Card Text"),
        is_card,
        "This placement is a block (use Open Block)",
    );
    let remove_edges = enabled_or(
        ContextMenuItem::action(canvas_ids::REMOVE_EDGES, "Remove All Connections"),
        state.has_visual_edges,
        "No visual connections on this node",
    );
    let copy_block_id = enabled_or(
        ContextMenuItem::action(canvas_ids::COPY_BLOCK_ID, "Copy Block ID"),
        is_block,
        "This placement is a card (no block id)",
    );

    ContextMenu::new("canvas")
        .item(open_block)
        .item(edit_card)
        .separator()
        .item(
            ContextMenuItem::action(canvas_ids::CONNECT_TO, "Connect to...")
                .disabled("Semantic edge create is a future surface (V1 stub)"),
        )
        .item(
            ContextMenuItem::action(canvas_ids::ADD_VISUAL_EDGE, "Add Visual Link")
                .disabled("Visual edge create is a future surface (V1 stub)"),
        )
        .item(remove_edges)
        .separator()
        .item(ContextMenuItem::action(canvas_ids::MOVE_TO_FRONT, "Bring to Front"))
        .item(ContextMenuItem::action(canvas_ids::MOVE_TO_BACK, "Send to Back"))
        .separator()
        .item(copy_block_id)
        .separator()
        .item(ContextMenuItem::action(canvas_ids::REMOVE, "Remove from Canvas"))
        .item(
            ContextMenuItem::action(canvas_ids::DELETE_BLOCK, "Delete Block")
                .disabled("Delete block needs a confirmation dialog (future surface)"),
        )
        .into_items()
}

/// Map a confirmed canvas-node menu id to its typed action, honoring `state.kind` (an `open_block` on a
/// card, or `edit_card` on a block, is disabled — so even a confirmed id maps to `None`, the
/// belt-and-braces second line of defence). Stubs / unknown ids map to `None`.
pub fn canvas_node_action_for_id(id: &str, state: &CanvasNodeState) -> Option<CanvasNodeMenuAction> {
    let is_block = matches!(state.kind, CanvasNodeKind::Block);
    let is_card = matches!(state.kind, CanvasNodeKind::Card);
    match id {
        canvas_ids::OPEN_BLOCK if is_block => Some(CanvasNodeMenuAction::OpenBlock),
        canvas_ids::EDIT_CARD if is_card => Some(CanvasNodeMenuAction::EditCard),
        canvas_ids::REMOVE_EDGES if state.has_visual_edges => {
            Some(CanvasNodeMenuAction::RemoveEdges)
        }
        canvas_ids::MOVE_TO_FRONT => Some(CanvasNodeMenuAction::MoveToFront),
        canvas_ids::MOVE_TO_BACK => Some(CanvasNodeMenuAction::MoveToBack),
        canvas_ids::COPY_BLOCK_ID if is_block => Some(CanvasNodeMenuAction::CopyBlockId),
        canvas_ids::REMOVE => Some(CanvasNodeMenuAction::Remove),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 7: Source-control change row
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the source-control change-row context menu (MT-021 scope, exact).
pub mod scm_ids {
    pub const STAGE: &str = "scm.stage";
    pub const UNSTAGE: &str = "scm.unstage";
    pub const DISCARD: &str = "scm.discard";
    pub const DIFF_WORKTREE: &str = "scm.diff_worktree";
    pub const DIFF_STAGED: &str = "scm.diff_staged";
    pub const BLAME: &str = "scm.blame";
    pub const COMMIT_THIS_FILE: &str = "scm.commit_this_file";
    pub const COPY_PATH: &str = "scm.copy_path";
}

/// The change-row state the menu builder reads — whether the entry has worktree (unstaged) changes
/// and/or index (staged) changes. Mirrors the verified backend `StatusEntry { index, worktree }`
/// (`Option<StatusCode>`): `has_worktree`/`has_index` are `true` iff the corresponding `Option` is
/// `Some`. Drives stage/unstage/discard enablement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmRowState {
    pub path: String,
    /// `true` iff `StatusEntry.worktree.is_some()` (unstaged changes exist → can stage/discard).
    pub has_worktree: bool,
    /// `true` iff `StatusEntry.index.is_some()` (staged changes exist → can unstage).
    pub has_index: bool,
}

/// A typed action a confirmed SCM-row menu id maps to. `discard`/`commit_this_file` are V1 stubs
/// (disabled), so they have no variant and can never fire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScmRowMenuAction {
    /// `POST /source-control/stage` `{repo_path, paths:[path]}`.
    Stage,
    /// `POST /source-control/unstage` `{repo_path, paths:[path]}`.
    Unstage,
    /// `GET /source-control/diff?scope=worktree` → display in the diff area.
    DiffWorktree,
    /// `GET /source-control/diff?scope=staged` → display in the diff area.
    DiffStaged,
    /// `GET /source-control/blame` → display in the detail area.
    Blame,
    /// Copy the entry path to the clipboard (no backend).
    CopyPath,
}

/// Build the source-control change-row context menu for `state`. Stage is enabled iff the entry has
/// worktree changes; Unstage iff it has index changes; Discard iff it has worktree changes BUT is a V1
/// stub (no confirm dialog yet) so it is rendered DISABLED + disclosed. Diff/Blame/Copy Path are always
/// enabled. Commit This File is a V1 stub (disabled).
pub fn source_control_context_items(state: &ScmRowState) -> Vec<ContextMenuItem> {
    let stage = enabled_or(
        ContextMenuItem::action(scm_ids::STAGE, "Stage"),
        state.has_worktree,
        "No unstaged changes on this file",
    );
    let unstage = enabled_or(
        ContextMenuItem::action(scm_ids::UNSTAGE, "Unstage"),
        state.has_index,
        "No staged changes on this file",
    );
    // Discard requires a confirm dialog (not yet built — V1). It is ALWAYS disabled here so an
    // accidental dispatch can never destroy local changes (red-team discard control). The disabled
    // reason carries the STUB_NO_CONFIRM marker a grep can detect if this is ever wrongly enabled.
    let discard = ContextMenuItem::action(scm_ids::DISCARD, "Discard Changes")
        .disabled("STUB_NO_CONFIRM: discard needs a confirmation dialog (future surface)");

    ContextMenu::new("scm")
        .item(stage)
        .item(unstage)
        .item(discard)
        .separator()
        .item(ContextMenuItem::action(scm_ids::DIFF_WORKTREE, "Show Worktree Diff"))
        .item(ContextMenuItem::action(scm_ids::DIFF_STAGED, "Show Staged Diff"))
        .item(ContextMenuItem::action(scm_ids::BLAME, "Show Line Blame"))
        .separator()
        .item(
            ContextMenuItem::action(scm_ids::COMMIT_THIS_FILE, "Commit This File...")
                .disabled("Single-file commit input is a future surface (V1 stub)"),
        )
        .item(ContextMenuItem::action(scm_ids::COPY_PATH, "Copy Path"))
        .into_items()
}

/// Map a confirmed SCM-row menu id to its typed action, honoring `state` (stage on a no-worktree entry,
/// or unstage on a no-index entry, is disabled → maps to `None`). `discard`/`commit_this_file`/unknown
/// map to `None`.
pub fn source_control_action_for_id(id: &str, state: &ScmRowState) -> Option<ScmRowMenuAction> {
    match id {
        scm_ids::STAGE if state.has_worktree => Some(ScmRowMenuAction::Stage),
        scm_ids::UNSTAGE if state.has_index => Some(ScmRowMenuAction::Unstage),
        scm_ids::DIFF_WORKTREE => Some(ScmRowMenuAction::DiffWorktree),
        scm_ids::DIFF_STAGED => Some(ScmRowMenuAction::DiffStaged),
        scm_ids::BLAME => Some(ScmRowMenuAction::Blame),
        scm_ids::COPY_PATH => Some(ScmRowMenuAction::CopyPath),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 8: Console / list rows (debug console + generic list)
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the debug-console-row context menu (MT-021 scope, exact). The filter children are a
/// one-level submenu under `console.filter_kind`.
pub mod console_ids {
    pub const COPY_LINE: &str = "console.copy_line";
    pub const COPY_ALL: &str = "console.copy_all";
    pub const FILTER_KIND: &str = "console.filter_kind";
    pub const FILTER_INPUT: &str = "console.filter.input";
    pub const FILTER_RESULT: &str = "console.filter.result";
    pub const FILTER_ERROR: &str = "console.filter.error";
    pub const FILTER_OUTPUT: &str = "console.filter.output";
    pub const FILTER_ALL: &str = "console.filter.all";
    pub const CLEAR: &str = "console.clear";
}

/// Stable ids for the GENERIC-list-row context menu (problems / timeline / job list / log tail). Shares
/// the filter submenu pattern with the console.
pub mod list_ids {
    pub const COPY_TEXT: &str = "list.copy_text";
    pub const COPY_ALL: &str = "list.copy_all";
    pub const GO_TO_SOURCE: &str = "list.go_to_source";
    pub const FILTER_KIND: &str = "list.filter_kind";
}

/// A debug-console entry kind (mirrors the React `DebugConsoleEntry.kind`). The filter submenu sets the
/// console display filter to one of these (or clears it via `All`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleEntryKind {
    Input,
    Result,
    Error,
    Output,
}

/// A typed action a confirmed debug-console-row menu id maps to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleRowMenuAction {
    /// Copy the right-clicked row's text to the clipboard.
    CopyLine,
    /// Copy every entry's text (joined) to the clipboard.
    CopyAll,
    /// Set the display filter to a single kind (`Some`) or clear it (`None` for "Show All").
    SetFilter(Option<ConsoleEntryKind>),
    /// Clear the IN-MEMORY display entries (NOT any persistent backend log — see action doc).
    Clear,
}

/// A typed action a confirmed generic-list-row menu id maps to. `go_to_source` is enabled only when the
/// row carries a source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListRowMenuAction {
    /// Copy the right-clicked row's text.
    CopyText,
    /// Copy every row's text (joined).
    CopyAll,
    /// Navigate to the row's file/line source.
    GoToSource,
    /// Set the display filter to a single kind (`Some`) or clear it (`None`).
    SetFilter(Option<ConsoleEntryKind>),
}

/// The shared filter submenu used by both the console and the generic list (`Input/Results/Errors/
/// Output/Show All`). The child ids are the surface's own filter-child ids so a confirmed child maps
/// back unambiguously.
fn filter_submenu(
    parent_id: &'static str,
    input: &'static str,
    result: &'static str,
    error: &'static str,
    output: &'static str,
    all: &'static str,
) -> ContextMenuItem {
    ContextMenuItem::submenu(
        parent_id,
        "Filter by Kind",
        vec![
            ContextMenuItem::action(input, "Input only"),
            ContextMenuItem::action(result, "Results only"),
            ContextMenuItem::action(error, "Errors only"),
            ContextMenuItem::action(output, "Output only"),
            ContextMenuItem::action(all, "Show All"),
        ],
    )
}

/// Build the debug-console-row context menu (Copy Line, Copy All, Filter by Kind submenu, Clear).
pub fn console_row_context_items() -> Vec<ContextMenuItem> {
    ContextMenu::new("console")
        .item(ContextMenuItem::action(console_ids::COPY_LINE, "Copy Line"))
        .item(ContextMenuItem::action(console_ids::COPY_ALL, "Copy All"))
        .separator()
        .item(filter_submenu(
            console_ids::FILTER_KIND,
            console_ids::FILTER_INPUT,
            console_ids::FILTER_RESULT,
            console_ids::FILTER_ERROR,
            console_ids::FILTER_OUTPUT,
            console_ids::FILTER_ALL,
        ))
        .separator()
        .item(ContextMenuItem::action(console_ids::CLEAR, "Clear Console"))
        .into_items()
}

/// Map a confirmed debug-console-row menu id to its typed action. The `filter_kind` submenu HEADER is
/// never confirmed (only its children are), so it maps to `None`; the filter CHILDREN map to a
/// `SetFilter`. Unknown ids map to `None`.
pub fn console_row_action_for_id(id: &str) -> Option<ConsoleRowMenuAction> {
    match id {
        console_ids::COPY_LINE => Some(ConsoleRowMenuAction::CopyLine),
        console_ids::COPY_ALL => Some(ConsoleRowMenuAction::CopyAll),
        console_ids::FILTER_INPUT => {
            Some(ConsoleRowMenuAction::SetFilter(Some(ConsoleEntryKind::Input)))
        }
        console_ids::FILTER_RESULT => {
            Some(ConsoleRowMenuAction::SetFilter(Some(ConsoleEntryKind::Result)))
        }
        console_ids::FILTER_ERROR => {
            Some(ConsoleRowMenuAction::SetFilter(Some(ConsoleEntryKind::Error)))
        }
        console_ids::FILTER_OUTPUT => {
            Some(ConsoleRowMenuAction::SetFilter(Some(ConsoleEntryKind::Output)))
        }
        console_ids::FILTER_ALL => Some(ConsoleRowMenuAction::SetFilter(None)),
        console_ids::CLEAR => Some(ConsoleRowMenuAction::Clear),
        _ => None,
    }
}

/// Build the generic-list-row context menu. `go_to_source` is enabled iff `has_source` (the row carries
/// a file/line reference); otherwise it is disabled + disclosed.
pub fn list_row_context_items(has_source: bool) -> Vec<ContextMenuItem> {
    let go_to_source = enabled_or(
        ContextMenuItem::action(list_ids::GO_TO_SOURCE, "Go to Source"),
        has_source,
        "This row has no file/line reference",
    );
    ContextMenu::new("list")
        .item(ContextMenuItem::action(list_ids::COPY_TEXT, "Copy"))
        .item(ContextMenuItem::action(list_ids::COPY_ALL, "Copy All"))
        .separator()
        .item(go_to_source)
        .item(filter_submenu(
            list_ids::FILTER_KIND,
            console_ids::FILTER_INPUT,
            console_ids::FILTER_RESULT,
            console_ids::FILTER_ERROR,
            console_ids::FILTER_OUTPUT,
            console_ids::FILTER_ALL,
        ))
        .into_items()
}

/// Map a confirmed generic-list-row menu id to its typed action, honoring `has_source` (`go_to_source`
/// on a row with no location is disabled → maps to `None`). The filter children reuse the console
/// filter ids.
pub fn list_row_action_for_id(id: &str, has_source: bool) -> Option<ListRowMenuAction> {
    match id {
        list_ids::COPY_TEXT => Some(ListRowMenuAction::CopyText),
        list_ids::COPY_ALL => Some(ListRowMenuAction::CopyAll),
        list_ids::GO_TO_SOURCE if has_source => Some(ListRowMenuAction::GoToSource),
        console_ids::FILTER_INPUT => {
            Some(ListRowMenuAction::SetFilter(Some(ConsoleEntryKind::Input)))
        }
        console_ids::FILTER_RESULT => {
            Some(ListRowMenuAction::SetFilter(Some(ConsoleEntryKind::Result)))
        }
        console_ids::FILTER_ERROR => {
            Some(ListRowMenuAction::SetFilter(Some(ConsoleEntryKind::Error)))
        }
        console_ids::FILTER_OUTPUT => {
            Some(ListRowMenuAction::SetFilter(Some(ConsoleEntryKind::Output)))
        }
        console_ids::FILTER_ALL => Some(ListRowMenuAction::SetFilter(None)),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 9: Drawer item (bottom stash shelf)
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the drawer-item context menu (MT-021 scope, exact). `send_to_pane` is a submenu whose
/// children are built DYNAMICALLY from the open panes (`drawer.send_to_pane.{pane_id}` — dynamic child
/// ids are allowed as long as the parent id is stable).
pub mod drawer_ids {
    pub const STOW: &str = "drawer.stow";
    pub const PIN: &str = "drawer.pin";
    pub const PROMOTE: &str = "drawer.promote";
    pub const SEND_TO_PANE: &str = "drawer.send_to_pane";
    pub const COPY_TO_PROMPT: &str = "drawer.copy_to_prompt";
    pub const ATTACH_EVIDENCE: &str = "drawer.attach_evidence";
    pub const CONVERT_ARTIFACT: &str = "drawer.convert_artifact";
    pub const DISCARD: &str = "drawer.discard";
}

/// The cached drawer-item state the menu builder reads: its pinned flag (drives the Pin/Unpin label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawerItemState {
    pub item_id: String,
    pub pinned: bool,
}

/// A typed action a confirmed drawer-item menu id maps to. The evidence/artifact stubs are disabled, so
/// they have no variant. `SendToPane` carries the chosen pane id (a dynamic submenu child).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawerItemMenuAction {
    /// Minimize the card to an icon in the shelf.
    Stow,
    /// Toggle the card's pinned flag (`target` is the NEW value, from the fresh cached `pinned`).
    TogglePin { target: bool },
    /// Open the card content as a full pane.
    Promote,
    /// Move the card content to the chosen open pane as a new tab.
    SendToPane { pane_id: String },
    /// Copy the card text into the focused prompt/editor.
    CopyToPrompt,
    /// Remove the card from the shelf.
    Discard,
}

/// Build the drawer-item context menu for `state`. `send_to_pane` is a one-level submenu whose children
/// are built from `open_panes` (`(pane_id, display_label)` pairs) with dynamic child ids
/// `drawer.send_to_pane.{pane_id}`; the parent id stays the stable `drawer.send_to_pane`. The dynamic
/// child ids are `String`s, so they are stored in a side table the caller keys on at confirm time (see
/// [`drawer_send_to_pane_target`]). The evidence/artifact items are V1 stubs (disabled).
pub fn drawer_item_context_items(
    state: &DrawerItemState,
    open_panes: &[(String, String)],
) -> Vec<ContextMenuItem> {
    let pin_label = if state.pinned { "Unpin" } else { "Pin" };
    // The submenu children are built with LEAKED &'static ids derived from the pane ids. egui needs
    // &'static str ids for the typed model; the caller maps the confirmed id back to a pane via
    // `drawer_send_to_pane_target` (string-prefix parse), so leaking is bounded by the open-pane count
    // at right-click time and the parent id stays the stable `drawer.send_to_pane`.
    let children: Vec<ContextMenuItem> = if open_panes.is_empty() {
        vec![ContextMenuItem::action("drawer.send_to_pane.__none", "No open panes")
            .disabled("No open panes to send to")]
    } else {
        open_panes
            .iter()
            .map(|(pane_id, label)| {
                let child_id: &'static str =
                    Box::leak(format!("drawer.send_to_pane.{pane_id}").into_boxed_str());
                let child_label: &'static str = Box::leak(label.clone().into_boxed_str());
                ContextMenuItem::action(child_id, child_label)
            })
            .collect()
    };
    ContextMenu::new("drawer")
        .item(ContextMenuItem::action(drawer_ids::STOW, "Stow"))
        .item(ContextMenuItem::action(drawer_ids::PIN, pin_label))
        .item(ContextMenuItem::action(drawer_ids::PROMOTE, "Promote to Pane"))
        .item(ContextMenuItem::submenu(drawer_ids::SEND_TO_PANE, "Send to Pane...", children))
        .separator()
        .item(ContextMenuItem::action(drawer_ids::COPY_TO_PROMPT, "Copy to Prompt"))
        .item(
            ContextMenuItem::action(drawer_ids::ATTACH_EVIDENCE, "Attach as Evidence")
                .disabled("Evidence-system link is a future surface (V1 stub)"),
        )
        .item(
            ContextMenuItem::action(drawer_ids::CONVERT_ARTIFACT, "Convert to Artifact...")
                .disabled("Artifact conversion is a future surface (V1 stub)"),
        )
        .separator()
        .item(ContextMenuItem::action(drawer_ids::DISCARD, "Discard"))
        .into_items()
}

/// If `id` is a dynamic `drawer.send_to_pane.{pane_id}` child id (and not the disabled `__none`
/// placeholder), return the `{pane_id}` it targets. Used by the caller to map a confirmed submenu child
/// to the pane to send the card to (red-team: snapshot/verify the pane still exists before sending).
pub fn drawer_send_to_pane_target(id: &str) -> Option<&str> {
    let suffix = id.strip_prefix("drawer.send_to_pane.")?;
    if suffix.is_empty() || suffix == "__none" {
        return None;
    }
    Some(suffix)
}

/// Map a confirmed drawer-item menu id to its typed action, computing the pin toggle TARGET from the
/// fresh cached `state`. A `send_to_pane.{pane_id}` child returns `SendToPane`. The submenu HEADER, the
/// evidence/artifact stubs, and unknown ids map to `None`.
pub fn drawer_item_action_for_id(id: &str, state: &DrawerItemState) -> Option<DrawerItemMenuAction> {
    match id {
        drawer_ids::STOW => Some(DrawerItemMenuAction::Stow),
        drawer_ids::PIN => Some(DrawerItemMenuAction::TogglePin { target: !state.pinned }),
        drawer_ids::PROMOTE => Some(DrawerItemMenuAction::Promote),
        drawer_ids::COPY_TO_PROMPT => Some(DrawerItemMenuAction::CopyToPrompt),
        drawer_ids::DISCARD => Some(DrawerItemMenuAction::Discard),
        other => drawer_send_to_pane_target(other).map(|pane_id| {
            DrawerItemMenuAction::SendToPane {
                pane_id: pane_id.to_owned(),
            }
        }),
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Surface 10: Status bar segment
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Stable ids for the status-bar-segment context menu (MT-021 scope, exact).
pub mod statusbar_ids {
    pub const COPY_SEGMENT: &str = "statusbar.copy_segment";
    pub const TOGGLE_VISIBILITY: &str = "statusbar.toggle_visibility";
    pub const OPEN_PANEL: &str = "statusbar.open_panel";
    pub const REFRESH: &str = "statusbar.refresh";
}

/// The cached status-bar-segment state the menu builder reads: the segment's stable id, its display
/// label, whether it is currently visible (drives the Hide/Show label), and the human name of the panel
/// `open_panel` opens (so the menu label reads e.g. "Open System Status").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusBarSegmentState {
    pub segment_id: String,
    pub segment_label: String,
    pub visible: bool,
    /// `Some(panel_name)` when this segment has a related panel to open; `None` disables `open_panel`.
    pub related_panel_name: Option<String>,
}

/// A typed action a confirmed status-bar-segment menu id maps to. `toggle_visibility` carries the NEW
/// visibility; `open_panel` is only produced when the segment has a related panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusBarMenuAction {
    /// Copy the segment's display text.
    CopySegment,
    /// Toggle the segment's visibility (`target` is the NEW value).
    ToggleVisibility { target: bool },
    /// Open the segment's related panel in a pane.
    OpenPanel,
    /// Re-fetch the segment's data.
    Refresh,
}

/// Build the status-bar-segment context menu for `state`. The Hide/Show label reflects the current
/// visibility; the Open label reads "Open {related_panel_name}" when a related panel exists (else the
/// item is disabled + disclosed). Copy + Refresh are always enabled.
pub fn status_bar_context_items(state: &StatusBarSegmentState) -> Vec<ContextMenuItem> {
    let visibility_label: &'static str = if state.visible {
        "Hide"
    } else {
        // A label that names the segment ("Show {name}") needs an owned String; the menu model takes
        // &'static str, so a non-visible segment uses the generic "Show Segment" label. (A segment is
        // normally visible when right-clicked — it must be on screen to click — so "Hide" is the live
        // path; "Show" is the round-trip-restore label for a future settings surface.)
        "Show Segment"
    };
    let open_panel = match &state.related_panel_name {
        Some(name) => {
            let label: &'static str = Box::leak(format!("Open {name}").into_boxed_str());
            ContextMenuItem::action(statusbar_ids::OPEN_PANEL, label)
        }
        None => ContextMenuItem::action(statusbar_ids::OPEN_PANEL, "Open Panel")
            .disabled("This segment has no related panel"),
    };
    ContextMenu::new("statusbar")
        .item(ContextMenuItem::action(statusbar_ids::COPY_SEGMENT, "Copy"))
        .separator()
        .item(ContextMenuItem::action(statusbar_ids::TOGGLE_VISIBILITY, visibility_label))
        .separator()
        .item(open_panel)
        .item(ContextMenuItem::action(statusbar_ids::REFRESH, "Refresh"))
        .into_items()
}

/// Map a confirmed status-bar-segment menu id to its typed action, computing the visibility toggle
/// TARGET from the fresh cached `state`. `open_panel` is only produced when the segment has a related
/// panel (else its item is disabled → maps to `None`). Unknown ids map to `None`.
pub fn status_bar_action_for_id(id: &str, state: &StatusBarSegmentState) -> Option<StatusBarMenuAction> {
    match id {
        statusbar_ids::COPY_SEGMENT => Some(StatusBarMenuAction::CopySegment),
        statusbar_ids::TOGGLE_VISIBILITY => {
            Some(StatusBarMenuAction::ToggleVisibility { target: !state.visible })
        }
        statusbar_ids::OPEN_PANEL if state.related_panel_name.is_some() => {
            Some(StatusBarMenuAction::OpenPanel)
        }
        statusbar_ids::REFRESH => Some(StatusBarMenuAction::Refresh),
        _ => None,
    }
}

/// Helper: keep `item` enabled, or disable it with `reason` when `enabled` is false. Centralizes the
/// "render-but-disable when the precondition fails" pattern the MT-021 surfaces share.
fn enabled_or(item: ContextMenuItem, enabled: bool, reason: &'static str) -> ContextMenuItem {
    if enabled {
        item
    } else {
        item.disabled(reason)
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Shared id-collection helper for the audit test
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Collect every item id from a built item list (skipping separators), recursing one level into any
/// submenu (V1 nesting depth). Used by the stable-id audit test.
pub fn collect_item_ids(items: &[ContextMenuItem]) -> Vec<&'static str> {
    let mut ids = Vec::new();
    for item in items {
        match &item.kind {
            MenuItemKind::Separator => {}
            MenuItemKind::Action => ids.push(item.id),
            MenuItemKind::Submenu(children) => {
                ids.push(item.id);
                for child in children {
                    if !matches!(child.kind, MenuItemKind::Separator) {
                        ids.push(child.id);
                    }
                }
            }
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC: "All menu item ids exactly match the stable ids listed in the scope (no typos — verified by
    /// a #[test] that asserts item ids against a reference list)." The reference list here is the exact
    /// id set each surface offers; a typo in any builder breaks this at CI.
    #[test]
    fn tab_item_ids_match_reference() {
        let items = tab_context_items(0, 3, false);
        assert_eq!(
            collect_item_ids(&items),
            vec![
                "tab.close",
                "tab.close_others",
                "tab.close_all",
                "tab.pin",
                "tab.split_right",
                "tab.split_down",
                "tab.move_to_new_window",
                "tab.pop_out",
            ],
        );
    }

    #[test]
    fn pane_item_ids_match_reference() {
        let items = pane_header_context_items(false, false);
        assert_eq!(
            collect_item_ids(&items),
            vec![
                "pane.lock",
                "pane.split_right",
                "pane.split_down",
                "pane.set_type_editor",
                "pane.set_type_terminal",
                "pane.set_type_canvas",
                "pane.set_type_browser",
                "pane.pop_out",
                "pane.close",
            ],
        );
    }

    #[test]
    fn project_item_ids_match_reference() {
        let items = project_tab_context_items(false, 2);
        assert_eq!(
            collect_item_ids(&items),
            vec!["project.activate", "project.new", "project.rename", "project.close"],
        );
    }

    /// Pin/Unpin label reflects the live pinned state (the contract's "label changes based on current
    /// state").
    #[test]
    fn pin_item_label_reflects_state() {
        let unpinned = tab_context_items(0, 1, false);
        let pin = unpinned.iter().find(|i| i.id == tab_ids::PIN).unwrap();
        assert_eq!(pin.label, "Pin");
        assert!(pin.enabled);

        let pinned = tab_context_items(0, 1, true);
        let unpin = pinned.iter().find(|i| i.id == tab_ids::PIN).unwrap();
        assert_eq!(unpin.label, "Unpin");
    }

    /// Close Others is disabled when there is only one tab (nothing else to close) and enabled when
    /// there is more than one (the contract's `enabled=if >1 tab in pane`).
    #[test]
    fn close_others_enabled_only_with_multiple_tabs() {
        let one = tab_context_items(0, 1, false);
        let co_one = one.iter().find(|i| i.id == tab_ids::CLOSE_OTHERS).unwrap();
        assert!(!co_one.enabled, "Close Others disabled with a single tab");
        assert_eq!(co_one.disabled_reason, Some("Only one tab in this pane"));

        let many = tab_context_items(1, 3, false);
        let co_many = many.iter().find(|i| i.id == tab_ids::CLOSE_OTHERS).unwrap();
        assert!(co_many.enabled, "Close Others enabled with multiple tabs");
    }

    /// Future-target items are rendered DISABLED (no fake-enable) AND map to no action.
    #[test]
    fn future_target_items_are_disabled_and_unmapped() {
        let items = tab_context_items(0, 2, false);
        for fid in [tab_ids::SPLIT_RIGHT, tab_ids::SPLIT_DOWN, tab_ids::MOVE_TO_NEW_WINDOW] {
            let item = items.iter().find(|i| i.id == fid).unwrap();
            assert!(!item.enabled, "{fid} is disabled (future target, no fake-enable)");
            assert!(item.disabled_reason.is_some(), "{fid} discloses why it is disabled");
            assert!(tab_action_for_id(fid).is_none(), "{fid} maps to no fireable action");
        }
        let pane = pane_header_context_items(false, false);
        for fid in [
            pane_ids::SPLIT_RIGHT,
            pane_ids::SPLIT_DOWN,
            pane_ids::SET_TYPE_EDITOR,
            pane_ids::SET_TYPE_TERMINAL,
            pane_ids::SET_TYPE_CANVAS,
            pane_ids::SET_TYPE_BROWSER,
            pane_ids::CLOSE,
        ] {
            let item = pane.iter().find(|i| i.id == fid).unwrap();
            assert!(!item.enabled, "{fid} is disabled (future target)");
            assert!(pane_header_action_for_id(fid).is_none(), "{fid} maps to no fireable action");
        }
    }

    /// Enabled ids map to their typed action; unknown / separator ids map to none.
    #[test]
    fn enabled_ids_map_to_actions() {
        assert_eq!(tab_action_for_id(tab_ids::CLOSE), Some(TabMenuAction::Close));
        assert_eq!(tab_action_for_id(tab_ids::CLOSE_OTHERS), Some(TabMenuAction::CloseOthers));
        assert_eq!(tab_action_for_id(tab_ids::CLOSE_ALL), Some(TabMenuAction::CloseAll));
        assert_eq!(tab_action_for_id(tab_ids::PIN), Some(TabMenuAction::TogglePin));
        assert_eq!(tab_action_for_id(tab_ids::POP_OUT), Some(TabMenuAction::PopOut));
        assert_eq!(tab_action_for_id("ctx-menu.separator"), None);
        assert_eq!(tab_action_for_id("bogus.id"), None);

        assert_eq!(pane_header_action_for_id(pane_ids::LOCK), Some(PaneHeaderMenuAction::ToggleLock));
        assert_eq!(pane_header_action_for_id(pane_ids::POP_OUT), Some(PaneHeaderMenuAction::PopOut));

        assert_eq!(
            project_tab_action_for_id(project_ids::ACTIVATE),
            Some(ProjectTabMenuAction::Activate)
        );
        assert_eq!(project_tab_action_for_id(project_ids::CLOSE), None);
    }

    /// Pane lock item label reflects the live lock state.
    #[test]
    fn pane_lock_label_reflects_state() {
        let unlocked = pane_header_context_items(false, false);
        assert_eq!(unlocked.iter().find(|i| i.id == pane_ids::LOCK).unwrap().label, "Lock Pane");
        let locked = pane_header_context_items(true, false);
        assert_eq!(locked.iter().find(|i| i.id == pane_ids::LOCK).unwrap().label, "Unlock Pane");
    }

    /// The last-pane guard disables `pane.close` with the distinct "only pane" reason.
    #[test]
    fn pane_close_last_pane_reason() {
        let last = pane_header_context_items(false, true);
        let close = last.iter().find(|i| i.id == pane_ids::CLOSE).unwrap();
        assert!(!close.enabled);
        assert_eq!(close.disabled_reason, Some("Cannot close the only pane"));
    }

    /// Project activate is disabled for the already-active project, enabled otherwise.
    #[test]
    fn project_activate_disabled_for_active() {
        let active = project_tab_context_items(true, 2);
        assert!(!active.iter().find(|i| i.id == project_ids::ACTIVATE).unwrap().enabled);
        let inactive = project_tab_context_items(false, 2);
        assert!(inactive.iter().find(|i| i.id == project_ids::ACTIVATE).unwrap().enabled);
    }

    /// Explorer-row item ids match the stable `ctx-menu.explorer.{id}` reference list (no typos), for
    /// each row kind. A document/bookmark renames; a canvas's rename is disabled but still present.
    #[test]
    fn explorer_item_ids_match_reference() {
        for kind in [
            ExplorerRowKind::Document,
            ExplorerRowKind::Canvas,
            ExplorerRowKind::Bookmark,
        ] {
            let items = explorer_context_items(kind);
            assert_eq!(
                collect_item_ids(&items),
                vec![
                    "explorer.open",
                    "explorer.copy_path",
                    "explorer.rename",
                    "explorer.reveal_in_graph",
                    "explorer.route_to_stage",
                ],
                "explorer ids for {kind:?}",
            );
        }
    }

    /// Open + Copy Path are always enabled; rename is enabled ONLY for a BOOKMARK row (the only kind
    /// whose id is a genuine Loom-block id). A Document or Canvas row's rename is disabled+disclosed (its
    /// id is a document/canvas id, NOT a Loom-block id — PATCHing `/loom/blocks/{id}` would 404).
    /// reveal_in_graph is always disabled.
    #[test]
    fn explorer_enable_state_matches_row_kind() {
        for (kind, rename_enabled) in [
            (ExplorerRowKind::Bookmark, true),
            (ExplorerRowKind::Document, false),
            (ExplorerRowKind::Canvas, false),
        ] {
            let items = explorer_context_items(kind);
            let find = |id: &str| items.iter().find(|i| i.id == id).unwrap();
            assert!(find(explorer_ids::OPEN).enabled, "open enabled for {kind:?}");
            assert!(find(explorer_ids::COPY_PATH).enabled, "copy_path enabled for {kind:?}");
            let rename = find(explorer_ids::RENAME);
            assert_eq!(rename.enabled, rename_enabled, "rename enable for {kind:?}");
            if !rename_enabled {
                assert!(rename.disabled_reason.is_some(), "disabled rename discloses why");
            }
            let reveal = find(explorer_ids::REVEAL_IN_GRAPH);
            assert!(!reveal.enabled, "reveal_in_graph disabled (no graph surface)");
            assert!(reveal.disabled_reason.is_some(), "reveal discloses why it is disabled");
        }
    }

    /// FIX (BLOCKER): a Document row's id is a DOCUMENT id, NOT a Loom-block id, so its Rename item must
    /// be present-but-DISABLED with a disclosed reason (mirrors the canvas-disabled assertion), and the
    /// id-to-action mapper must refuse it — so a document id can never reach the `/loom/blocks/{id}`
    /// PATCH (which would 404 at runtime). This is the regression guard for the adversarial finding.
    #[test]
    fn document_row_rename_is_present_but_disabled() {
        let items = explorer_context_items(ExplorerRowKind::Document);
        let rename = items
            .iter()
            .find(|i| i.id == explorer_ids::RENAME)
            .expect("Document row still RENDERS a Rename item (no fake-drop)");
        assert!(!rename.enabled, "Document Rename is disabled (document id is not a Loom-block id)");
        assert_eq!(
            rename.disabled_reason,
            Some(
                "Documents are not Loom blocks; document rename needs a document endpoint (future MT)"
            ),
            "disabled Document Rename discloses the real reason",
        );
        // Belt-and-braces: even a (impossible) confirmed Document rename maps to NO fireable action,
        // guaranteeing a document id is never routed to the Loom-block PATCH.
        assert_eq!(
            explorer_action_for_id(explorer_ids::RENAME, ExplorerRowKind::Document),
            None,
            "Document rename maps to no action (disabled, wrong id space)",
        );
    }

    /// FIX (BLOCKER): a Bookmark row's id IS a genuine `LoomBlock.block_id` (built in `project_tree.rs`
    /// from the pins view's `block_id`), so it is the ONLY explorer row whose Rename stays ENABLED and
    /// maps to the real `Rename` action that drives the verified PATCH. This pins the post-fix contract:
    /// rename is enabled iff the row carries a real block id.
    #[test]
    fn bookmark_row_rename_is_the_only_enabled_rename() {
        let bookmark = explorer_context_items(ExplorerRowKind::Bookmark);
        let rename = bookmark.iter().find(|i| i.id == explorer_ids::RENAME).unwrap();
        assert!(rename.enabled, "Bookmark Rename is enabled (block_id is a real Loom-block id)");
        assert_eq!(
            explorer_action_for_id(explorer_ids::RENAME, ExplorerRowKind::Bookmark),
            Some(ExplorerMenuAction::Rename),
            "Bookmark rename fires the real PATCH-driving action",
        );
        // Cross-check: the OTHER two kinds do NOT enable rename, so Bookmark is the sole enabled rename.
        for other in [ExplorerRowKind::Document, ExplorerRowKind::Canvas] {
            let r = explorer_context_items(other)
                .into_iter()
                .find(|i| i.id == explorer_ids::RENAME)
                .unwrap();
            assert!(!r.enabled, "{other:?} rename is disabled (not a Loom-block id)");
        }
    }

    /// Open + Copy Path map to their typed action for EVERY row kind (they involve no backend). Rename
    /// fires ONLY for a Bookmark row; reveal_in_graph + a document/canvas rename + unknown ids map to
    /// none (the future-target / disabled second line of defence).
    #[test]
    fn explorer_ids_map_to_actions() {
        for kind in [
            ExplorerRowKind::Document,
            ExplorerRowKind::Canvas,
            ExplorerRowKind::Bookmark,
        ] {
            assert_eq!(
                explorer_action_for_id(explorer_ids::OPEN, kind),
                Some(ExplorerMenuAction::Open)
            );
            assert_eq!(
                explorer_action_for_id(explorer_ids::COPY_PATH, kind),
                Some(ExplorerMenuAction::CopyPath)
            );
            assert_eq!(explorer_action_for_id(explorer_ids::REVEAL_IN_GRAPH, kind), None);
            assert_eq!(explorer_action_for_id("bogus.id", kind), None);
        }
        // Rename fires ONLY for a Bookmark row (the only Loom-block-backed id).
        assert_eq!(
            explorer_action_for_id(explorer_ids::RENAME, ExplorerRowKind::Bookmark),
            Some(ExplorerMenuAction::Rename),
            "bookmark rename fires the real PATCH action",
        );
        // A document or canvas rename is disabled (wrong id space), so even a confirmed id maps to no
        // fireable action — a document/canvas id can never reach the Loom-block PATCH.
        for kind in [ExplorerRowKind::Document, ExplorerRowKind::Canvas] {
            assert_eq!(
                explorer_action_for_id(explorer_ids::RENAME, kind),
                None,
                "{kind:?} rename maps to no action (disabled, not a Loom-block id)",
            );
        }
        // MT-033: Route-to-Stage fires ONLY for a Document row (the Stage pane displays a routed
        // document); a Canvas/Bookmark row's item is disabled, so even a confirmed id maps to None.
        assert_eq!(
            explorer_action_for_id(explorer_ids::ROUTE_TO_STAGE, ExplorerRowKind::Document),
            Some(ExplorerMenuAction::RouteToStage),
            "document Route-to-Stage fires the route-to-stage action",
        );
        for kind in [ExplorerRowKind::Canvas, ExplorerRowKind::Bookmark] {
            assert_eq!(
                explorer_action_for_id(explorer_ids::ROUTE_TO_STAGE, kind),
                None,
                "{kind:?} Route-to-Stage maps to no action (disabled)",
            );
        }
    }

    /// Every item id across all FOUR surfaces is unique within its own surface (so a confirmed id
    /// dispatches unambiguously). Cross-surface ids are namespaced by prefix
    /// (`tab.`/`pane.`/`project.`/`explorer.`).
    #[test]
    fn ids_unique_within_each_surface() {
        for items in [
            tab_context_items(0, 3, false),
            pane_header_context_items(false, false),
            project_tab_context_items(false, 2),
            explorer_context_items(ExplorerRowKind::Document),
        ] {
            let ids = collect_item_ids(&items);
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), ids.len(), "ids unique within surface: {ids:?}");
        }
    }

    // ── MT-021 (C5 part 2): the six additional surfaces ─────────────────────────────────────────────

    fn loom_state(pinned: bool, favorite: bool, has_edges: bool) -> LoomNodeState {
        LoomNodeState {
            block_id: "blk-1".to_owned(),
            pinned,
            favorite,
            has_edges,
        }
    }

    /// AC: Loom-node item ids exactly match the contract reference list (no typos).
    #[test]
    fn loom_node_item_ids_match_reference() {
        let items = loom_node_context_items(&loom_state(false, false, false));
        assert_eq!(
            collect_item_ids(&items),
            vec![
                "loom.open",
                "loom.open_to_side",
                "loom.rename",
                "loom.pin",
                "loom.favorite",
                "loom.connect",
                "loom.disconnect",
                "loom.copy_block_id",
                "loom.reveal_in_panel",
                "loom.delete",
            ],
        );
    }

    /// loom.pin label + toggle TARGET come from the FRESH cached state and always flip the right way
    /// (red-team stale-state control).
    #[test]
    fn loom_pin_favorite_reflect_fresh_state_and_flip() {
        let unpinned = loom_node_context_items(&loom_state(false, false, false));
        assert_eq!(unpinned.iter().find(|i| i.id == loom_ids::PIN).unwrap().label, "Pin");
        assert_eq!(
            loom_node_action_for_id(loom_ids::PIN, &loom_state(false, false, false)),
            Some(LoomNodeMenuAction::TogglePin { target: true }),
            "pin on an unpinned block sends pinned=true",
        );

        let pinned = loom_node_context_items(&loom_state(true, true, false));
        assert_eq!(pinned.iter().find(|i| i.id == loom_ids::PIN).unwrap().label, "Unpin");
        assert_eq!(pinned.iter().find(|i| i.id == loom_ids::FAVORITE).unwrap().label, "Unfavorite");
        assert_eq!(
            loom_node_action_for_id(loom_ids::PIN, &loom_state(true, false, false)),
            Some(LoomNodeMenuAction::TogglePin { target: false }),
            "pin on a pinned block sends pinned=false",
        );
        assert_eq!(
            loom_node_action_for_id(loom_ids::FAVORITE, &loom_state(false, true, false)),
            Some(LoomNodeMenuAction::ToggleFavorite { target: false }),
        );
    }

    /// loom stubs (connect/disconnect/delete) are disabled + disclosed and map to no action.
    #[test]
    fn loom_stubs_disabled_and_unmapped() {
        let items = loom_node_context_items(&loom_state(false, false, false));
        for sid in [loom_ids::CONNECT, loom_ids::DISCONNECT, loom_ids::DELETE] {
            let item = items.iter().find(|i| i.id == sid).unwrap();
            assert!(!item.enabled, "{sid} disabled (stub)");
            assert!(item.disabled_reason.is_some(), "{sid} discloses why");
            assert!(
                loom_node_action_for_id(sid, &loom_state(true, true, true)).is_none(),
                "{sid} maps to no fireable action",
            );
        }
    }

    fn canvas_state(kind: CanvasNodeKind, has_visual_edges: bool) -> CanvasNodeState {
        CanvasNodeState {
            placement_id: "pl-1".to_owned(),
            kind,
            has_visual_edges,
        }
    }

    /// AC: canvas-node item ids exactly match the contract reference list.
    #[test]
    fn canvas_node_item_ids_match_reference() {
        let items = canvas_node_context_items(&canvas_state(CanvasNodeKind::Block, false));
        assert_eq!(
            collect_item_ids(&items),
            vec![
                "canvas.open_block",
                "canvas.edit_card",
                "canvas.connect_to",
                "canvas.add_visual_edge",
                "canvas.remove_edges",
                "canvas.move_to_front",
                "canvas.move_to_back",
                "canvas.copy_block_id",
                "canvas.remove",
                "canvas.delete_block",
            ],
        );
    }

    /// open_block/copy_block_id enabled only for a BLOCK; edit_card only for a CARD; remove + move
    /// always; remove_edges only with visual edges.
    #[test]
    fn canvas_enable_state_matches_kind() {
        let block = canvas_node_context_items(&canvas_state(CanvasNodeKind::Block, true));
        let find = |items: &[ContextMenuItem], id: &str| {
            items.iter().find(|i| i.id == id).unwrap().enabled
        };
        assert!(find(&block, canvas_ids::OPEN_BLOCK), "block enables open_block");
        assert!(find(&block, canvas_ids::COPY_BLOCK_ID), "block enables copy_block_id");
        assert!(!find(&block, canvas_ids::EDIT_CARD), "block disables edit_card");
        assert!(find(&block, canvas_ids::REMOVE_EDGES), "has visual edges enables remove_edges");

        let card = canvas_node_context_items(&canvas_state(CanvasNodeKind::Card, false));
        assert!(find(&card, canvas_ids::EDIT_CARD), "card enables edit_card");
        assert!(!find(&card, canvas_ids::OPEN_BLOCK), "card disables open_block");
        assert!(!find(&card, canvas_ids::REMOVE_EDGES), "no visual edges disables remove_edges");
    }

    /// AC: canvas.remove maps to the Remove action (DELETE placement); stubs map to none even when
    /// confirmed; a wrong-kind id maps to none.
    #[test]
    fn canvas_actions_map_correctly() {
        let block = canvas_state(CanvasNodeKind::Block, true);
        assert_eq!(
            canvas_node_action_for_id(canvas_ids::REMOVE, &block),
            Some(CanvasNodeMenuAction::Remove),
        );
        assert_eq!(
            canvas_node_action_for_id(canvas_ids::OPEN_BLOCK, &block),
            Some(CanvasNodeMenuAction::OpenBlock),
        );
        assert_eq!(
            canvas_node_action_for_id(canvas_ids::MOVE_TO_FRONT, &block),
            Some(CanvasNodeMenuAction::MoveToFront),
        );
        assert_eq!(
            canvas_node_action_for_id(canvas_ids::REMOVE_EDGES, &block),
            Some(CanvasNodeMenuAction::RemoveEdges),
        );
        for stub in [canvas_ids::CONNECT_TO, canvas_ids::ADD_VISUAL_EDGE, canvas_ids::DELETE_BLOCK] {
            assert!(canvas_node_action_for_id(stub, &block).is_none(), "{stub} maps to none");
        }
        // edit_card on a block, open_block on a card → none (belt-and-braces second line of defence).
        assert!(canvas_node_action_for_id(canvas_ids::EDIT_CARD, &block).is_none());
        let card = canvas_state(CanvasNodeKind::Card, false);
        assert!(canvas_node_action_for_id(canvas_ids::OPEN_BLOCK, &card).is_none());
        // remove_edges with no visual edges → none.
        assert!(canvas_node_action_for_id(canvas_ids::REMOVE_EDGES, &card).is_none());
    }

    fn scm_state(has_worktree: bool, has_index: bool) -> ScmRowState {
        ScmRowState {
            path: "src/x.rs".to_owned(),
            has_worktree,
            has_index,
        }
    }

    /// AC: source-control item ids exactly match the contract reference list.
    #[test]
    fn scm_item_ids_match_reference() {
        let items = source_control_context_items(&scm_state(true, true));
        assert_eq!(
            collect_item_ids(&items),
            vec![
                "scm.stage",
                "scm.unstage",
                "scm.discard",
                "scm.diff_worktree",
                "scm.diff_staged",
                "scm.blame",
                "scm.commit_this_file",
                "scm.copy_path",
            ],
        );
    }

    /// Stage enabled iff worktree changes; Unstage iff index changes; discard is ALWAYS disabled
    /// (STUB_NO_CONFIRM marker present — red-team discard control).
    #[test]
    fn scm_enable_state_and_discard_stub() {
        fn enabled(items: &[ContextMenuItem], id: &str) -> bool {
            items.iter().find(|i| i.id == id).unwrap().enabled
        }
        let worktree_only = source_control_context_items(&scm_state(true, false));
        assert!(enabled(&worktree_only, scm_ids::STAGE), "worktree → stage enabled");
        assert!(!enabled(&worktree_only, scm_ids::UNSTAGE), "no index → unstage disabled");

        let index_only = source_control_context_items(&scm_state(false, true));
        assert!(enabled(&index_only, scm_ids::UNSTAGE), "index → unstage enabled");
        assert!(!enabled(&index_only, scm_ids::STAGE), "no worktree → stage disabled");

        // Discard is ALWAYS disabled and carries the STUB_NO_CONFIRM marker.
        for state in [scm_state(true, true), scm_state(false, false)] {
            let items = source_control_context_items(&state);
            let discard = items.iter().find(|i| i.id == scm_ids::DISCARD).unwrap();
            assert!(!discard.enabled, "discard always disabled (no confirm dialog yet)");
            assert!(
                discard.disabled_reason.unwrap().contains("STUB_NO_CONFIRM"),
                "discard disabled reason carries the STUB_NO_CONFIRM marker",
            );
            assert!(
                source_control_action_for_id(scm_ids::DISCARD, &state).is_none(),
                "discard maps to no fireable action (can never destroy changes by accident)",
            );
        }
    }

    /// AC: scm.stage maps to Stage (drives the verified POST), diff/blame/copy always map; stage on a
    /// no-worktree row maps to none.
    #[test]
    fn scm_actions_map_correctly() {
        let both = scm_state(true, true);
        assert_eq!(source_control_action_for_id(scm_ids::STAGE, &both), Some(ScmRowMenuAction::Stage));
        assert_eq!(
            source_control_action_for_id(scm_ids::UNSTAGE, &both),
            Some(ScmRowMenuAction::Unstage),
        );
        assert_eq!(
            source_control_action_for_id(scm_ids::DIFF_WORKTREE, &both),
            Some(ScmRowMenuAction::DiffWorktree),
        );
        assert_eq!(
            source_control_action_for_id(scm_ids::DIFF_STAGED, &both),
            Some(ScmRowMenuAction::DiffStaged),
        );
        assert_eq!(source_control_action_for_id(scm_ids::BLAME, &both), Some(ScmRowMenuAction::Blame));
        assert_eq!(
            source_control_action_for_id(scm_ids::COPY_PATH, &both),
            Some(ScmRowMenuAction::CopyPath),
        );
        // stage with no worktree changes → none; commit_this_file stub → none.
        assert!(source_control_action_for_id(scm_ids::STAGE, &scm_state(false, true)).is_none());
        assert!(source_control_action_for_id(scm_ids::COMMIT_THIS_FILE, &both).is_none());
    }

    /// AC: console-row item ids match the contract reference list (with the five filter children under
    /// the filter_kind submenu).
    #[test]
    fn console_row_item_ids_match_reference() {
        let items = console_row_context_items();
        assert_eq!(
            collect_item_ids(&items),
            vec![
                "console.copy_line",
                "console.copy_all",
                "console.filter_kind",
                "console.filter.input",
                "console.filter.result",
                "console.filter.error",
                "console.filter.output",
                "console.filter.all",
                "console.clear",
            ],
        );
    }

    /// console copy_line / clear / each filter child map to the right action; the filter_kind submenu
    /// HEADER is never confirmed → maps to none.
    #[test]
    fn console_actions_map_correctly() {
        assert_eq!(console_row_action_for_id(console_ids::COPY_LINE), Some(ConsoleRowMenuAction::CopyLine));
        assert_eq!(console_row_action_for_id(console_ids::COPY_ALL), Some(ConsoleRowMenuAction::CopyAll));
        assert_eq!(console_row_action_for_id(console_ids::CLEAR), Some(ConsoleRowMenuAction::Clear));
        assert_eq!(
            console_row_action_for_id(console_ids::FILTER_INPUT),
            Some(ConsoleRowMenuAction::SetFilter(Some(ConsoleEntryKind::Input))),
        );
        assert_eq!(
            console_row_action_for_id(console_ids::FILTER_ALL),
            Some(ConsoleRowMenuAction::SetFilter(None)),
        );
        // The submenu header itself maps to none (only its children are confirmable).
        assert!(console_row_action_for_id(console_ids::FILTER_KIND).is_none());
    }

    /// generic list go_to_source enabled iff the row has a source; the action mapper refuses it when
    /// the row has none.
    #[test]
    fn list_go_to_source_gated_on_source() {
        let with_src = list_row_context_items(true);
        assert!(with_src.iter().find(|i| i.id == list_ids::GO_TO_SOURCE).unwrap().enabled);
        assert_eq!(
            list_row_action_for_id(list_ids::GO_TO_SOURCE, true),
            Some(ListRowMenuAction::GoToSource),
        );

        let no_src = list_row_context_items(false);
        let g = no_src.iter().find(|i| i.id == list_ids::GO_TO_SOURCE).unwrap();
        assert!(!g.enabled, "no source → go_to_source disabled");
        assert!(list_row_action_for_id(list_ids::GO_TO_SOURCE, false).is_none());
    }

    fn drawer_state(pinned: bool) -> DrawerItemState {
        DrawerItemState {
            item_id: "card-1".to_owned(),
            pinned,
        }
    }

    /// AC: drawer-item item ids match the contract reference (with the dynamic send_to_pane children).
    #[test]
    fn drawer_item_ids_match_reference_with_dynamic_panes() {
        let panes = vec![
            ("pane-a".to_owned(), "Workspace".to_owned()),
            ("pane-b".to_owned(), "Inference Lab".to_owned()),
        ];
        let items = drawer_item_context_items(&drawer_state(false), &panes);
        assert_eq!(
            collect_item_ids(&items),
            vec![
                "drawer.stow",
                "drawer.pin",
                "drawer.promote",
                "drawer.send_to_pane",
                "drawer.send_to_pane.pane-a",
                "drawer.send_to_pane.pane-b",
                "drawer.copy_to_prompt",
                "drawer.attach_evidence",
                "drawer.convert_artifact",
                "drawer.discard",
            ],
        );
    }

    /// drawer pin label/target flips on the fresh state; a dynamic send_to_pane child maps to the
    /// targeted pane; the stubs map to none.
    #[test]
    fn drawer_actions_map_correctly() {
        assert_eq!(
            drawer_item_action_for_id(drawer_ids::PIN, &drawer_state(false)),
            Some(DrawerItemMenuAction::TogglePin { target: true }),
        );
        assert_eq!(
            drawer_item_action_for_id(drawer_ids::PIN, &drawer_state(true)),
            Some(DrawerItemMenuAction::TogglePin { target: false }),
        );
        assert_eq!(
            drawer_item_action_for_id(drawer_ids::STOW, &drawer_state(false)),
            Some(DrawerItemMenuAction::Stow),
        );
        assert_eq!(
            drawer_item_action_for_id("drawer.send_to_pane.pane-c", &drawer_state(false)),
            Some(DrawerItemMenuAction::SendToPane { pane_id: "pane-c".to_owned() }),
        );
        assert_eq!(drawer_send_to_pane_target("drawer.send_to_pane.pane-c"), Some("pane-c"));
        assert_eq!(drawer_send_to_pane_target("drawer.send_to_pane.__none"), None);
        for stub in [drawer_ids::ATTACH_EVIDENCE, drawer_ids::CONVERT_ARTIFACT, drawer_ids::SEND_TO_PANE]
        {
            assert!(
                drawer_item_action_for_id(stub, &drawer_state(false)).is_none(),
                "{stub} maps to none (stub or submenu header)",
            );
        }
    }

    fn statusbar_state(visible: bool, panel: Option<&str>) -> StatusBarSegmentState {
        StatusBarSegmentState {
            segment_id: "health".to_owned(),
            segment_label: "Backend: OK".to_owned(),
            visible,
            related_panel_name: panel.map(|s| s.to_owned()),
        }
    }

    /// AC: status-bar item ids match the contract reference list.
    #[test]
    fn status_bar_item_ids_match_reference() {
        let items = status_bar_context_items(&statusbar_state(true, Some("System Status")));
        assert_eq!(
            collect_item_ids(&items),
            vec![
                "statusbar.copy_segment",
                "statusbar.toggle_visibility",
                "statusbar.open_panel",
                "statusbar.refresh",
            ],
        );
    }

    /// open_panel label reads "Open {panel}" and is enabled when a related panel exists; disabled +
    /// disclosed otherwise. copy/toggle/refresh always map; toggle target flips visibility.
    #[test]
    fn status_bar_open_panel_and_actions() {
        let with_panel = status_bar_context_items(&statusbar_state(true, Some("System Status")));
        let open = with_panel.iter().find(|i| i.id == statusbar_ids::OPEN_PANEL).unwrap();
        assert!(open.enabled, "segment with a related panel enables open_panel");
        assert_eq!(open.label, "Open System Status", "label names the related panel");
        assert_eq!(
            status_bar_action_for_id(statusbar_ids::OPEN_PANEL, &statusbar_state(true, Some("x"))),
            Some(StatusBarMenuAction::OpenPanel),
        );

        let no_panel = status_bar_context_items(&statusbar_state(true, None));
        let open = no_panel.iter().find(|i| i.id == statusbar_ids::OPEN_PANEL).unwrap();
        assert!(!open.enabled, "no related panel → open_panel disabled");
        assert!(status_bar_action_for_id(statusbar_ids::OPEN_PANEL, &statusbar_state(true, None)).is_none());

        // toggle visibility flips, copy + refresh always map.
        assert_eq!(
            status_bar_action_for_id(statusbar_ids::TOGGLE_VISIBILITY, &statusbar_state(true, None)),
            Some(StatusBarMenuAction::ToggleVisibility { target: false }),
        );
        assert_eq!(
            status_bar_action_for_id(statusbar_ids::COPY_SEGMENT, &statusbar_state(true, None)),
            Some(StatusBarMenuAction::CopySegment),
        );
        assert_eq!(
            status_bar_action_for_id(statusbar_ids::REFRESH, &statusbar_state(true, None)),
            Some(StatusBarMenuAction::Refresh),
        );
    }

    /// Every MT-021 surface's ids are unique within its own surface (so a confirmed id dispatches
    /// unambiguously), including the submenu children.
    #[test]
    fn mt021_ids_unique_within_each_surface() {
        let panes = vec![("pane-a".to_owned(), "A".to_owned())];
        let surfaces = vec![
            loom_node_context_items(&loom_state(false, false, true)),
            canvas_node_context_items(&canvas_state(CanvasNodeKind::Block, true)),
            source_control_context_items(&scm_state(true, true)),
            console_row_context_items(),
            list_row_context_items(true),
            drawer_item_context_items(&drawer_state(false), &panes),
            status_bar_context_items(&statusbar_state(true, Some("System Status"))),
        ];
        for items in surfaces {
            let ids = collect_item_ids(&items);
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), ids.len(), "ids unique within surface: {ids:?}");
        }
    }
}
