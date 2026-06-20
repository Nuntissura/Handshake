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
    ContextMenu::new("explorer")
        .item(ContextMenuItem::action(explorer_ids::OPEN, "Open"))
        .item(ContextMenuItem::action(explorer_ids::COPY_PATH, "Copy Path"))
        .separator()
        .item(rename)
        .item(
            ContextMenuItem::action(explorer_ids::REVEAL_IN_GRAPH, "Reveal in Graph")
                .disabled("No graph/loom view surface in this build (future)"),
        )
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
        _ => None,
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
}
