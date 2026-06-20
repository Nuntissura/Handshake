//! Single source of truth for the shell's declared stable AccessKit identities, plus the
//! live-tree interactive-naming gate (WP-KERNEL-011 MT-025).
//!
//! ## Why a declared registry
//!
//! Stable `author_id`s and fixed `NodeId`s are scattered across the codebase by necessity — the
//! theme toggle pins its id in `app.rs`, chrome pins its ids in `live.rs`, and panes derive theirs
//! from `pane_registry.rs`'s monotonic counter (base 100). Nothing previously asserted that those
//! independently-chosen numbers do not COLLIDE. A collision is silent and dangerous: AccessKit keys
//! its tree by `NodeId`, so two widgets sharing an id means one becomes invisible to an
//! out-of-process model (RISK: "Hash/id collision" in the MT red-team).
//!
//! [`DECLARED_IDENTITIES`] gathers every hand-assigned identity into one list so a single unit test
//! (in this module's `tests`) can prove the full set is collision-free across both `author_id` and
//! `NodeId`.
//! It is intentionally a flat const slice (not a runtime map) so the proof is compile-time-visible
//! and a new identity added anywhere must be registered here or the collision/coverage test will not
//! cover it.
//!
//! ## Why the gate lives here too
//!
//! [`assert_no_unnamed_interactive`] walks a LIVE `accesskit::TreeUpdate` and panics if any
//! interactive (clickable/focusable) node lacks an `author_id`. It reads the same declared-identity
//! vocabulary conceptually (an interactive widget MUST carry a stable address), so co-locating it
//! with the registry keeps the "what is a stable identity" and "every interactive node must have
//! one" rules in one module.

use egui::accesskit;

use super::live::{STATUS_BAR_NODE_ID, TITLE_BAR_NODE_ID};
use crate::left_rail::LEFT_RAIL_BUTTONS;
use crate::split_layout::{
    DIVIDER_H_AUTHOR_ID, DIVIDER_H_NODE_ID, DIVIDER_V_AUTHOR_ID, DIVIDER_V_NODE_ID,
};
use crate::module_switcher::{MODULE_DEFINITIONS, MODULE_NODE_ID_BASE};
use crate::pane_header::{PANE_LOCK_SLOTS, PANE_TITLE_SLOTS};
use crate::popout_window::MERGE_BACK_SLOTS;
use crate::project_tabs::{PROJECT_TABS_AUTHOR_ID, PROJECT_TABS_NODE_ID};
use crate::project_tree::{
    BOOKMARKS_AUTHOR_ID, BOOKMARKS_NODE_ID, PROJECT_TREE_AUTHOR_ID, PROJECT_TREE_NODE_ID,
};
use crate::quick_links::{QUICK_LINKS_AUTHOR_ID, QUICK_LINKS_NODE_ID};
use crate::rails::SCROLLBAR_V_NODE_IDS;
use crate::tab_bar::TABBAR_SLOTS;

/// Fixed AccessKit/egui id for the theme-toggle button (mirrors the private `THEME_TOGGLE_NODE_ID`
/// in `app.rs`). Re-declared here as the registry's copy so the collision test can see it without
/// `app.rs` exposing its private const. The live-frame integration test proves the toggle actually
/// emits this exact id + author_id into the real tree, so the two copies cannot silently drift.
pub const THEME_TOGGLE_NODE_ID: u64 = 10;

/// Fixed AccessKit/egui id for the theme-toggle button's stable author_id.
pub const THEME_TOGGLE_AUTHOR_ID: &str = "shell.chrome.theme-toggle";

/// The base AccessKit `NodeId` from which `PaneRegistry` allocates pane ids (mirrors
/// `PaneRegistry::ACCESSKIT_ID_BASE`). The first seeded pane (`pane-a`) gets exactly this id; later
/// panes increment from here. Declared so the collision check proves chrome/toggle ids stay strictly
/// below the pane id space and can never overlap it.
pub const PANE_NODE_ID_BASE: u64 = 100;

/// One declared stable identity: a hand-assigned `author_id` paired with its fixed `NodeId`.
/// Pane ids are NOT listed individually (they are allocated dynamically from `PANE_NODE_ID_BASE`);
/// instead the collision check asserts every fixed chrome/toggle id is strictly below the pane base
/// so the two id spaces are disjoint by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclaredIdentity {
    /// Stable kebab-case match key an out-of-process model addresses the widget by.
    pub author_id: &'static str,
    /// Fixed `NodeId` u64 backing the widget's `egui::Id` (and thus its AccessKit `NodeId`).
    pub node_id: u64,
}

/// Every hand-assigned stable identity in the shell chrome. The single source of truth: a new fixed
/// identity added anywhere in the shell MUST be added here so the collision test covers it.
///
/// - theme toggle      -> id 10  (interactive Button, `app::theme_toggle`)
/// - title bar         -> id 20  (`ChromeWidget::TitleBar`, `live::emit_chrome_node`)
/// - status bar        -> id 21  (`ChromeWidget::StatusBar`, `live::emit_chrome_node`)
/// - divider horizontal -> id 30 (`Role::Splitter`, `split_layout::SplitLayoutWidget`)
/// - divider vertical   -> id 31 (`Role::Splitter`, `split_layout::SplitLayoutWidget`)
/// - tab bar pane-a..d  -> id 60..63 (`Role::TabList`, `tab_bar::TabBar`)
/// - merge-back pane-a..d -> id 64..67 (`Role::Button`, `popout_window::PopOutPlaceholder`)
///
/// Panes occupy id >= 100 (see [`PANE_NODE_ID_BASE`]); they are validated by the disjointness
/// assertion in the collision test rather than enumerated here. Individual tab + tab-close nodes
/// (MT-007) are DYNAMIC (count varies as tabs open/close) and are addressed by an `egui::Id` derived
/// from their author_id STRING (`tab-{pane_id}-{index}`), so they live in egui's hashed id space —
/// NOT the small fixed band — and are not enumerated here. Only the fixed per-pane tab-bar CONTAINER
/// ids are declared.
pub const DECLARED_IDENTITIES: &[DeclaredIdentity] = &[
    DeclaredIdentity {
        author_id: THEME_TOGGLE_AUTHOR_ID,
        node_id: THEME_TOGGLE_NODE_ID,
    },
    DeclaredIdentity {
        author_id: "shell.chrome.title-bar",
        node_id: TITLE_BAR_NODE_ID,
    },
    DeclaredIdentity {
        author_id: "shell.chrome.status-bar",
        node_id: STATUS_BAR_NODE_ID,
    },
    DeclaredIdentity {
        author_id: DIVIDER_H_AUTHOR_ID,
        node_id: DIVIDER_H_NODE_ID,
    },
    DeclaredIdentity {
        author_id: DIVIDER_V_AUTHOR_ID,
        node_id: DIVIDER_V_NODE_ID,
    },
    // MT-007 per-pane tab-bar containers (Role::TabList), fixed 60..63 band.
    DeclaredIdentity {
        author_id: "tabbar-pane-a",
        node_id: TABBAR_SLOTS[0].1,
    },
    DeclaredIdentity {
        author_id: "tabbar-pane-b",
        node_id: TABBAR_SLOTS[1].1,
    },
    DeclaredIdentity {
        author_id: "tabbar-pane-c",
        node_id: TABBAR_SLOTS[2].1,
    },
    DeclaredIdentity {
        author_id: "tabbar-pane-d",
        node_id: TABBAR_SLOTS[3].1,
    },
    // MT-008 per-pane "Merge Back" placeholder buttons (Role::Button), fixed 64..67 band. These
    // render ONLY while a pane is popped out, so the default-seed live tree never contains them; the
    // collision test still covers their fixed ids here so they can never overlap chrome/tab/pane ids.
    DeclaredIdentity {
        author_id: "merge-back-pane-a",
        node_id: MERGE_BACK_SLOTS[0].1,
    },
    DeclaredIdentity {
        author_id: "merge-back-pane-b",
        node_id: MERGE_BACK_SLOTS[1].1,
    },
    DeclaredIdentity {
        author_id: "merge-back-pane-c",
        node_id: MERGE_BACK_SLOTS[2].1,
    },
    DeclaredIdentity {
        author_id: "merge-back-pane-d",
        node_id: MERGE_BACK_SLOTS[3].1,
    },
    // MT-010 per-pane vertical scrollbar rails (Role::ScrollBar), fresh 40..43 band. These render
    // ONLY when a pane's content overflows its viewport, so the default-seed live tree (placeholder
    // panes that fit) never contains them; the collision test still covers their fixed ids here so
    // they can never overlap chrome / divider / tab / merge-back / pane ids.
    DeclaredIdentity {
        author_id: SCROLLBAR_V_NODE_IDS[0].0,
        node_id: SCROLLBAR_V_NODE_IDS[0].1,
    },
    DeclaredIdentity {
        author_id: SCROLLBAR_V_NODE_IDS[1].0,
        node_id: SCROLLBAR_V_NODE_IDS[1].1,
    },
    DeclaredIdentity {
        author_id: SCROLLBAR_V_NODE_IDS[2].0,
        node_id: SCROLLBAR_V_NODE_IDS[2].1,
    },
    DeclaredIdentity {
        author_id: SCROLLBAR_V_NODE_IDS[3].0,
        node_id: SCROLLBAR_V_NODE_IDS[3].1,
    },
    // MT-011 top project-tab strip CONTAINER (Role::TabList), fresh band slot 50: above the scrollbar
    // rails (40..43), below the per-pane tab-bar containers (60..63), strictly below the pane id base
    // (100). Individual project TABS (Role::Tab) are dynamic (count varies as projects open/close) and
    // are addressed by an egui::Id derived from their author_id STRING (`project-tab-{id}`), so they
    // live in egui's hashed id space — NOT this fixed band — and are not enumerated here (same pattern
    // as the MT-007 per-tab nodes). Only the fixed container id is declared.
    DeclaredIdentity {
        author_id: PROJECT_TABS_AUTHOR_ID,
        node_id: PROJECT_TABS_NODE_ID,
    },
    // MT-012 top-right module switcher buttons (Role::Button), fresh band 51..=56: above the
    // project-tab strip container (50), below the per-pane tab-bar containers (60..63), strictly below
    // the pane id base (100). The module count is FIXED at six, so — unlike the dynamic project tabs —
    // each button gets a fixed id (MODULE_NODE_ID_BASE + index) and a fixed author_id (its data_id),
    // both enumerated here so the collision test proves the six ids are disjoint from every other
    // declared identity. (Const-context: the indices are spelled out because a const slice cannot
    // iterate; the `module_node_ids_sit_in_a_disjoint_fresh_band` unit test in `module_switcher` plus
    // this collision test together pin the band.)
    DeclaredIdentity {
        author_id: MODULE_DEFINITIONS[0].data_id,
        node_id: MODULE_NODE_ID_BASE,
    },
    DeclaredIdentity {
        author_id: MODULE_DEFINITIONS[1].data_id,
        node_id: MODULE_NODE_ID_BASE + 1,
    },
    DeclaredIdentity {
        author_id: MODULE_DEFINITIONS[2].data_id,
        node_id: MODULE_NODE_ID_BASE + 2,
    },
    DeclaredIdentity {
        author_id: MODULE_DEFINITIONS[3].data_id,
        node_id: MODULE_NODE_ID_BASE + 3,
    },
    DeclaredIdentity {
        author_id: MODULE_DEFINITIONS[4].data_id,
        node_id: MODULE_NODE_ID_BASE + 4,
    },
    DeclaredIdentity {
        author_id: MODULE_DEFINITIONS[5].data_id,
        node_id: MODULE_NODE_ID_BASE + 5,
    },
    // MT-013 per-pane LOCK buttons (Role::Button), fresh 70..73 band: above the MT-008 merge-back band
    // (64..67), below the pane id base (100). The lock button is the pane header's one interactive
    // control; the four fixed grid panes each get a fixed id (PANE_LOCK_NODE_ID_BASE + slot) and a
    // fixed author_id (`pane-{pane_id}-lock`), both enumerated here so the collision test proves the
    // four ids are disjoint from every other declared identity. The MT-013 pane-header TITLE is a
    // presentational Role::Label (no author_id, like the pane body label), so it is not declared here.
    // Individual TAB nodes keep their MT-007 dynamic `tab-{pane}-{index}` ids (egui hashed id space);
    // the MT-013 pane-a User-Manual override (`hs-usermanual-diagnostics-tab`) is likewise a dynamic
    // per-tab node, not a fixed-band id, so it is not enumerated here either.
    DeclaredIdentity {
        author_id: "pane-pane-a-lock",
        node_id: PANE_LOCK_SLOTS[0].1,
    },
    DeclaredIdentity {
        author_id: "pane-pane-b-lock",
        node_id: PANE_LOCK_SLOTS[1].1,
    },
    DeclaredIdentity {
        author_id: "pane-pane-c-lock",
        node_id: PANE_LOCK_SLOTS[2].1,
    },
    DeclaredIdentity {
        author_id: "pane-pane-d-lock",
        node_id: PANE_LOCK_SLOTS[3].1,
    },
    // MT-013 per-pane header TITLE labels (Role::Label), fresh 74..77 band: above the lock band
    // (70..73), below the pane id base (100). The title is non-interactive (a Label) but is emitted as
    // an ADDRESSABLE node so an out-of-process model reads a pane's active-tab binding by the stable
    // `pane-{pane_id}-title` id. Declared here so the collision test proves the four ids + author_ids
    // are disjoint from every other declared identity. (A Label carrying an author_id is allowed; the
    // MT-025 interactive gate only flags clickable/focusable nodes that LACK an author_id.)
    DeclaredIdentity {
        author_id: "pane-pane-a-title",
        node_id: PANE_TITLE_SLOTS[0].1,
    },
    DeclaredIdentity {
        author_id: "pane-pane-b-title",
        node_id: PANE_TITLE_SLOTS[1].1,
    },
    DeclaredIdentity {
        author_id: "pane-pane-c-title",
        node_id: PANE_TITLE_SLOTS[2].1,
    },
    DeclaredIdentity {
        author_id: "pane-pane-d-title",
        node_id: PANE_TITLE_SLOTS[3].1,
    },
    // MT-014 left activity rail controls (Role::Button), fresh 80..=88 band: above the pane-title band
    // (74..77), below the project-tree container (89) / quick-links container (90), strictly below the
    // pane id base (100). The nine controls are the four activity icons (files/agenda/mail/notes), the
    // stash toggle, the three bottom affordances (agenda/mail/notes), and the rail collapse toggle.
    // Each has a fixed id (LEFT_RAIL_NODE_ID_BASE + slot) and a fixed author_id, both enumerated here
    // (const slice cannot iterate) so the collision test proves the nine ids are disjoint from every
    // other declared identity. The `left_rail_band_is_disjoint_and_sequential` unit test in `left_rail`
    // pins the band shape; this declares them for the cross-module collision proof.
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[0].0,
        node_id: LEFT_RAIL_BUTTONS[0].1,
    },
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[1].0,
        node_id: LEFT_RAIL_BUTTONS[1].1,
    },
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[2].0,
        node_id: LEFT_RAIL_BUTTONS[2].1,
    },
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[3].0,
        node_id: LEFT_RAIL_BUTTONS[3].1,
    },
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[4].0,
        node_id: LEFT_RAIL_BUTTONS[4].1,
    },
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[5].0,
        node_id: LEFT_RAIL_BUTTONS[5].1,
    },
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[6].0,
        node_id: LEFT_RAIL_BUTTONS[6].1,
    },
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[7].0,
        node_id: LEFT_RAIL_BUTTONS[7].1,
    },
    DeclaredIdentity {
        author_id: LEFT_RAIL_BUTTONS[8].0,
        node_id: LEFT_RAIL_BUTTONS[8].1,
    },
    // MT-014 project-tree CONTAINER (Role::Tree), fixed slot 89: above the left-rail band (80..88),
    // below the quick-links container (90) and the pane id base (100). Individual tree group headers and
    // leaf rows are DYNAMIC (their count varies with the project's content) and live in egui's hashed id
    // space (author_id-derived), so they are not enumerated here — only the fixed container id is.
    DeclaredIdentity {
        author_id: PROJECT_TREE_AUTHOR_ID,
        node_id: PROJECT_TREE_NODE_ID,
    },
    // MT-014 quick-links CONTAINER (Role::List), fixed slot 90: above the project-tree container (89),
    // below the pane id base (100). Individual quick-link rows + the disclosure toggle are DYNAMIC /
    // author_id-derived, so they are not enumerated here — only the fixed container id is.
    DeclaredIdentity {
        author_id: QUICK_LINKS_AUTHOR_ID,
        node_id: QUICK_LINKS_NODE_ID,
    },
    // MT-014 FIX-A bookmarks-group CONTAINER (Role::Tree), fixed slot 91: above the quick-links
    // container (90), below the pane id base (100). The Bookmarks group renders pinned Loom blocks
    // below the Documents/Canvases groups inside the Files panel. Individual bookmark ROWS are DYNAMIC
    // (count varies with pins) and live in egui's hashed id space (`project-tree.bookmark.{slug}`), so
    // they are not enumerated here — only the fixed container id is.
    DeclaredIdentity {
        author_id: BOOKMARKS_AUTHOR_ID,
        node_id: BOOKMARKS_NODE_ID,
    },
];

/// AccessKit roles that denote an interactive widget a model is expected to be able to address and
/// drive. Mirrors egui's own `WidgetType -> Role` mapping for interactive widgets
/// (`egui::Response::fill_accesskit_node_from_widget_info`): Button family, text input, toggles,
/// selectors, sliders, spin buttons, and links. A node with one of these roles MUST carry a stable
/// `author_id` or the [`assert_no_unnamed_interactive`] gate fails.
pub const INTERACTIVE_ROLES: &[accesskit::Role] = &[
    accesskit::Role::Button,
    accesskit::Role::TextInput,
    accesskit::Role::CheckBox,
    accesskit::Role::RadioButton,
    accesskit::Role::ComboBox,
    accesskit::Role::Slider,
    accesskit::Role::SpinButton,
    accesskit::Role::Link,
    accesskit::Role::MenuItem,
    accesskit::Role::Tab,
    // MT-010 integrated scrollbar rails are driveable out-of-process (SetValue / ScrollUp /
    // ScrollDown), so a ScrollBar node MUST carry a stable author_id or the gate fails.
    accesskit::Role::ScrollBar,
];

/// True when a live AccessKit node is an interactive *control* a model is expected to drive.
///
/// egui marks interactive widgets with `Action::Click` (clickable) and/or `Action::Focus`
/// (focusable) on the node (`Response::fill_accesskit_node_common`), and gives them an interactive
/// `Role`. We treat a node as an interactive control if it carries an interactive role (the strong
/// signal), OR it supports `Action::Click` while NOT being a presentational text node.
///
/// The `Role::Label` exclusion is load-bearing and intentional: egui's `selectable_labels` style
/// option (default ON) gives EVERY `ui.label(...)` a `Sense::click()` so the user can click-drag to
/// select its text — but it explicitly strips `Sense::FOCUSABLE` from that select-sense
/// (`Label::layout_in_ui`: `select_sense -= Sense::FOCUSABLE`). Such a label is clickable text, not a
/// control to steer; counting it would flood the gate with false positives (every pane label) and
/// force meaningless author_ids onto static text. A real interactive label-like control reports a
/// control role (Button/Link/MenuItem/etc.), which `INTERACTIVE_ROLES` still catches.
fn is_interactive(node: &accesskit::Node) -> bool {
    if INTERACTIVE_ROLES.contains(&node.role()) {
        return true;
    }
    node.supports_action(accesskit::Action::Click) && node.role() != accesskit::Role::Label
}

/// Walk a LIVE `accesskit::TreeUpdate` and panic if any interactive (clickable/focusable) node lacks
/// a stable `author_id`.
///
/// This is the enforcement gate for the MT contract clause "every interactive widget sets a stable
/// AccessKit author_id". It runs against the real per-frame tree egui produced (the same value the
/// out-of-process UIA adapter receives), so it cannot be satisfied by in-memory-only nodes. The
/// panic message names the offending `NodeId` and `Role` so a developer adding an un-named
/// interactive widget gets an actionable failure.
///
/// Returns the count of interactive nodes inspected (all of which passed) so callers can assert the
/// gate actually examined widgets rather than trivially passing on an empty tree.
///
/// # Panics
/// Panics if any interactive node has no `author_id`. A negative test (in
/// `tests/test_accesskit_ids.rs`) proves the panic fires via `catch_unwind` so the gate cannot be
/// silently removed, and a positive test proves the real shell passes it.
pub fn assert_no_unnamed_interactive(update: &accesskit::TreeUpdate) -> usize {
    let mut interactive_count = 0usize;
    for (node_id, node) in &update.nodes {
        if !is_interactive(node) {
            continue;
        }
        interactive_count += 1;
        if node.author_id().is_none() {
            panic!(
                "AccessKit: interactive widget NodeId({}) role {:?} has no stable author_id -- \
                 every clickable/focusable widget must set one (see accessibility::emit_* helpers)",
                node_id.0,
                node.role(),
            );
        }
    }
    interactive_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Item-2 proof: NO NodeId or author_id collision across the full declared identity set, AND the
    /// fixed chrome/toggle id space is strictly disjoint from the pane id space (>= 100). Iterating
    /// `DECLARED_IDENTITIES` means a new fixed identity added to that list is automatically covered.
    #[test]
    fn declared_identities_have_no_node_id_or_author_id_collision() {
        let mut seen_ids: HashSet<u64> = HashSet::new();
        let mut seen_authors: HashSet<&'static str> = HashSet::new();

        for ident in DECLARED_IDENTITIES {
            assert!(
                seen_ids.insert(ident.node_id),
                "duplicate NodeId {} for author_id '{}'",
                ident.node_id,
                ident.author_id,
            );
            assert!(
                seen_authors.insert(ident.author_id),
                "duplicate author_id '{}'",
                ident.author_id,
            );
            // Fixed chrome/toggle ids must stay strictly below the pane id base so the two id spaces
            // can never overlap as panes are allocated upward from PANE_NODE_ID_BASE.
            assert!(
                ident.node_id < PANE_NODE_ID_BASE,
                "fixed identity '{}' id {} must stay below the pane id base {}",
                ident.author_id,
                ident.node_id,
                PANE_NODE_ID_BASE,
            );
        }

        // Sanity: the set is non-empty and counts match (no silent dedup hid a collision).
        assert_eq!(seen_ids.len(), DECLARED_IDENTITIES.len());
        assert_eq!(seen_authors.len(), DECLARED_IDENTITIES.len());
    }

    /// The gate flags a clickable node with no author_id, and accepts one once an author_id is set.
    #[test]
    fn gate_flags_unnamed_clickable_and_accepts_named() {
        // Unnamed clickable -> panics.
        let mut update = accesskit::TreeUpdate {
            nodes: Vec::new(),
            tree: Some(accesskit::Tree::new(accesskit::NodeId(1))),
            focus: accesskit::NodeId(1),
        };
        let mut btn = accesskit::Node::new(accesskit::Role::Button);
        btn.add_action(accesskit::Action::Click);
        update.nodes.push((accesskit::NodeId(2), btn));

        let unnamed = std::panic::catch_unwind(|| assert_no_unnamed_interactive(&update));
        assert!(unnamed.is_err(), "gate must panic on an unnamed clickable node");

        // Same node, now named -> passes, and reports 1 interactive node inspected.
        let mut named_update = accesskit::TreeUpdate {
            nodes: Vec::new(),
            tree: Some(accesskit::Tree::new(accesskit::NodeId(1))),
            focus: accesskit::NodeId(1),
        };
        let mut named = accesskit::Node::new(accesskit::Role::Button);
        named.add_action(accesskit::Action::Click);
        named.set_author_id("shell.chrome.theme-toggle".to_owned());
        named_update.nodes.push((accesskit::NodeId(2), named));
        let count = assert_no_unnamed_interactive(&named_update);
        assert_eq!(count, 1, "one interactive node inspected and passed");
    }

    /// Non-interactive nodes (e.g. a plain Label / Group container) are ignored by the gate.
    #[test]
    fn gate_ignores_non_interactive_nodes() {
        let mut update = accesskit::TreeUpdate {
            nodes: Vec::new(),
            tree: Some(accesskit::Tree::new(accesskit::NodeId(1))),
            focus: accesskit::NodeId(1),
        };
        // A label with no actions and a group container -- neither carries an author_id, and neither
        // should trip the gate.
        update
            .nodes
            .push((accesskit::NodeId(2), accesskit::Node::new(accesskit::Role::Label)));
        update
            .nodes
            .push((accesskit::NodeId(3), accesskit::Node::new(accesskit::Role::Group)));
        let count = assert_no_unnamed_interactive(&update);
        assert_eq!(count, 0, "no interactive nodes present");
    }
}
