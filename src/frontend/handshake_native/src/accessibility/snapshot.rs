//! Contract snapshot / verification endpoint for the live AccessKit tree.
//!
//! `collect_tree_snapshot` projects the live `accesskit::TreeUpdate` (the exact value egui hands the
//! platform adapter each frame) into a stable, machine-readable list of every node that carries a
//! stable `author_id`. This is the verification surface for MT-025's acceptance criterion ("panes +
//! chrome are findable in the live tree"): a test (or an out-of-process tool) can assert that every
//! expected `author_id` is present with the right role, without scraping pixels or relying on
//! display text.
//!
//! The snapshot is intentionally derived from the *live* `TreeUpdate`, not from `PaneRegistry`'s
//! in-memory builder, so it proves the LIVE emission actually happened — closing the recurring gap
//! where nodes existed in memory but never reached the frame's accessibility tree.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use egui::accesskit;
use serde::{Deserialize, Serialize};

/// One stable-id node from the live AccessKit tree, reduced to the fields a model uses to address it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessNodeSnapshot {
    /// Stable, model-meaningful match key (kebab-case). Only nodes that set this are included.
    pub author_id: String,
    /// Semantic role, as the debug name of the `accesskit::Role` (stable across the egui 0.33
    /// family; avoids leaking the non-serializable enum into the snapshot contract).
    pub role: String,
    /// Human/model-readable label, when set.
    pub label: Option<String>,
    /// The raw AccessKit node id (u64), for callers that address by id rather than author_id.
    pub node_id: u64,
}

/// A deterministic projection of the live AccessKit tree: every node that carries a stable
/// `author_id`, sorted by `author_id` so the snapshot is stable run-to-run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessTreeSnapshot {
    pub nodes: Vec<AccessNodeSnapshot>,
}

impl AccessTreeSnapshot {
    /// Look up a node by its stable `author_id`.
    pub fn by_author_id(&self, author_id: &str) -> Option<&AccessNodeSnapshot> {
        self.nodes.iter().find(|n| n.author_id == author_id)
    }

    /// True when every supplied `author_id` is present in the live tree.
    pub fn contains_all(&self, author_ids: &[&str]) -> bool {
        author_ids
            .iter()
            .all(|id| self.by_author_id(id).is_some())
    }

    /// All `author_id`s present, in snapshot (sorted) order.
    pub fn author_ids(&self) -> Vec<&str> {
        self.nodes.iter().map(|n| n.author_id.as_str()).collect()
    }
}

/// Project a live `accesskit::TreeUpdate` into a stable-id snapshot.
///
/// The `TreeUpdate` is obtained from `egui::FullOutput::platform_output.accesskit_update` after a
/// frame runs with AccessKit enabled (kittest enables it automatically; eframe enables it on the
/// real window). Only nodes with an `author_id` are included — egui emits many anonymous layout
/// nodes that are not part of the stable-id contract.
pub fn collect_tree_snapshot(update: &accesskit::TreeUpdate) -> AccessTreeSnapshot {
    let mut nodes: Vec<AccessNodeSnapshot> = update
        .nodes
        .iter()
        .filter_map(|(node_id, node)| {
            node.author_id().map(|author_id| AccessNodeSnapshot {
                author_id: author_id.to_owned(),
                role: format!("{:?}", node.role()),
                label: node.label().map(|l| l.to_owned()),
                node_id: node_id.0,
            })
        })
        .collect();
    nodes.sort_by(|a, b| a.author_id.cmp(&b.author_id));
    AccessTreeSnapshot { nodes }
}

// ── MT-026: in-process full UI-tree JSON snapshot (out-of-process model-vision surface) ─────────────
//
// `collect_tree_snapshot` (above) is MT-025's verification surface: a FLAT, author_id-only projection
// for asserting "the expected stable ids are live". MT-026 needs the OTHER half a no-context model
// needs to *operate* the UI: the full nested widget tree — every node (not only author_id-bearing
// ones), each with its role, label, value, disabled state, supported actions, layout bounds, and its
// children — serialized as a stable, deterministic JSON document. This is the native equivalent of a
// DOM snapshot (`querySelectorAll('*')` + the accessibility tree), the data source MT-027's action
// channel reads to enumerate steerable targets and MT-028's swarm-safety layer reads to reason about
// what is on screen, WITHOUT any OS accessibility (UIA) round-trip.
//
// ## Why this walks egui's own `TreeUpdate` and adds NO `accesskit_consumer` dependency
//
// The MT-026 contract body sketched an `accesskit_consumer::Tree` fed `TreeUpdate`s. That is the
// right shape for a host that only has the serialized updates and must rebuild tree topology itself.
// Here we are IN-PROCESS and already hold the exact `accesskit::TreeUpdate` egui produced this frame
// (the same value `collect_tree_snapshot` consumes and the same value eframe hands the UIA adapter).
// That `TreeUpdate` already carries the full node set plus each node's `children()` topology, so we
// can walk it directly from `tree.root`. Pulling in `accesskit_consumer` to re-derive topology we
// already have would add a dependency for zero benefit and fork the snapshot away from the live tree
// MT-025 already proves — so we reuse the live `TreeUpdate`, honoring the contract's intent (a
// stable, machine-readable, in-process full-tree consumer) without the redundant crate.
//
// ## Determinism + safety controls (MT-026 red-team)
//
// - Children are emitted in the node's own `children()` order (the visual/logical order egui built),
//   so the JSON is deterministic frame-to-frame for an unchanged UI.
// - The walk is ITERATIVE (an explicit stack), never recursive, so a deeply nested pane tree cannot
//   overflow the call stack (red-team RISK "deep tree stack overflow").
// - A hard node cap (`MAX_SNAPSHOT_NODES`) bounds output size; if exceeded, the offending parent gets
//   one synthetic `Overflow` child so a reader knows the tree was truncated rather than silently cut
//   (red-team RISK "JSON size explosion").
// - A visited-set guards against a malformed update whose `children()` form a cycle, so the walk
//   always terminates.

/// Hard cap on the number of nodes a single [`UiTreeSnapshot`] will contain. A real shell frame is
/// well under this (the MT-025 live frame has ~80 stable-id nodes plus anonymous layout nodes); the
/// cap exists only to bound a pathological tree so the JSON cannot explode without a visible marker.
pub const MAX_SNAPSHOT_NODES: usize = 4000;

/// Axis-aligned layout bounds of a node, in AccessKit logical pixels (top-left origin). This is the
/// same coordinate space egui/AccessKit use; no DPI scaling is applied here (the platform adapter
/// owns physical-pixel translation). `w`/`h` are derived from the AccessKit `Rect` corners so a model
/// gets a width/height it can target directly.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UiNodeBounds {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// One node in the full UI-tree snapshot — everything a no-context model needs to identify, reason
/// about, and (via MT-027) drive a single widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiTreeNode {
    /// The model-facing address for this node. Prefers the stable kebab-case `author_id` (what a
    /// model should steer by); falls back to `node:<raw_u64>` for anonymous layout nodes that carry
    /// no author_id, so every node still has a non-empty, debuggable id (never panics on a missing
    /// id — see red-team RISK "unknown ids in snapshot").
    pub id: String,
    /// The raw stable `author_id` when present (None for anonymous layout nodes). Kept distinct from
    /// `id` so a consumer can tell a real stable address from the synthetic `node:<u64>` fallback.
    pub author_id: Option<String>,
    /// The raw AccessKit `NodeId` u64, for callers addressing by id rather than author_id.
    pub node_id: u64,
    /// Semantic role as the debug name of the `accesskit::Role` (stable string across the egui 0.33
    /// family; avoids leaking the non-serializable enum into the JSON contract — same convention as
    /// [`AccessNodeSnapshot::role`]).
    pub role: String,
    /// Human/model-readable label, when set.
    pub label: Option<String>,
    /// Current displayed value (e.g. a text input's contents), when set.
    pub value: Option<String>,
    /// Whether the widget is disabled (not interactable). A model must not attempt to drive a
    /// disabled control; surfacing this lets MT-027/MT-028 skip it.
    pub disabled: bool,
    /// The AccessKit actions this node supports, as debug-name strings (e.g. `"Click"`, `"Focus"`,
    /// `"SetValue"`). This is the steerable-capability list MT-027's action channel consumes.
    pub actions: Vec<String>,
    /// Layout bounds, when the node carries them (anonymous root/window nodes often do not).
    pub bounds: Option<UiNodeBounds>,
    /// Child nodes, in the node's own (visual/logical) child order.
    pub children: Vec<UiTreeNode>,
}

/// A complete, machine-readable snapshot of the live UI widget tree — the in-process model-vision
/// surface (MT-026). Round-trips through serde JSON via [`UiTreeSnapshot::to_json`] /
/// [`serde_json::from_str`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiTreeSnapshot {
    /// The tree root (the egui window node). Walk `children` for the full UI.
    pub root: UiTreeNode,
    /// RFC3339-ish UTC capture timestamp (`<unix_seconds>.<nanos>Z` form via std time — no chrono
    /// dependency). Lets a reader tell two snapshots apart in time without parsing the tree.
    pub captured_at_utc: String,
    /// Total number of nodes in `root` (inclusive), so a reader can size the tree without walking it.
    pub widget_count: usize,
}

impl UiTreeSnapshot {
    /// Serialize the snapshot to pretty JSON. Infallible in practice (the types are plain serde
    /// data); falls back to `{}` rather than panicking so a snapshot call on the render path can
    /// never bring the UI down (red-team: never panic on the model-vision path).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Depth-first iterator over every node in the tree (root first, then children in order).
    pub fn iter_nodes(&self) -> impl Iterator<Item = &UiTreeNode> {
        let mut stack = vec![&self.root];
        std::iter::from_fn(move || {
            let node = stack.pop()?;
            // Push children in reverse so they pop in natural order.
            for child in node.children.iter().rev() {
                stack.push(child);
            }
            Some(node)
        })
    }

    /// Find the first node addressed by the given stable `author_id`, anywhere in the tree.
    pub fn find_by_author_id(&self, author_id: &str) -> Option<&UiTreeNode> {
        self.iter_nodes()
            .find(|n| n.author_id.as_deref() == Some(author_id))
    }
}

/// The full set of AccessKit actions, used to enumerate which ones a node supports. AccessKit exposes
/// `supports_action(Action) -> bool` but no iterator over a node's supported actions, so we probe each
/// known action. Listing them explicitly (rather than relying on a private discriminant range) keeps
/// the snapshot stable if the upstream enum grows: a new action simply will not be reported until
/// added here, which is safe (no panic, no wrong data).
const ALL_ACTIONS: &[accesskit::Action] = &[
    accesskit::Action::Click,
    accesskit::Action::Focus,
    accesskit::Action::Blur,
    accesskit::Action::Collapse,
    accesskit::Action::Expand,
    accesskit::Action::CustomAction,
    accesskit::Action::Decrement,
    accesskit::Action::Increment,
    accesskit::Action::HideTooltip,
    accesskit::Action::ShowTooltip,
    accesskit::Action::ReplaceSelectedText,
    accesskit::Action::ScrollDown,
    accesskit::Action::ScrollLeft,
    accesskit::Action::ScrollRight,
    accesskit::Action::ScrollUp,
    accesskit::Action::ScrollIntoView,
    accesskit::Action::ScrollToPoint,
    accesskit::Action::SetScrollOffset,
    accesskit::Action::SetTextSelection,
    accesskit::Action::SetSequentialFocusNavigationStartingPoint,
    accesskit::Action::SetValue,
    accesskit::Action::ShowContextMenu,
];

/// The supported-action debug-name strings for a single node, in [`ALL_ACTIONS`] order.
fn node_actions(node: &accesskit::Node) -> Vec<String> {
    ALL_ACTIONS
        .iter()
        .filter(|a| node.supports_action(**a))
        .map(|a| format!("{a:?}"))
        .collect()
}

/// Build a leaf [`UiTreeNode`] (no children yet) from a live AccessKit node + its id.
fn leaf_node(node_id: accesskit::NodeId, node: &accesskit::Node) -> UiTreeNode {
    let author_id = node.author_id().map(|a| a.to_owned());
    let id = author_id
        .clone()
        .unwrap_or_else(|| format!("node:{}", node_id.0));
    // Bounds: egui/AccessKit can hand back a `Rect` with non-finite corners (NaN/inf) for nodes that
    // have no resolved layout this frame. serde_json serializes a non-finite f32 as JSON `null`, which
    // then fails to deserialize back into an f32 — breaking the round-trip contract. Drop bounds unless
    // ALL four derived coords are finite, so the JSON always round-trips and a model never reads a
    // garbage rect (red-team: deterministic, lossless JSON).
    let bounds = node.bounds().and_then(|r| {
        let b = UiNodeBounds {
            x: r.x0 as f32,
            y: r.y0 as f32,
            w: (r.x1 - r.x0) as f32,
            h: (r.y1 - r.y0) as f32,
        };
        if b.x.is_finite() && b.y.is_finite() && b.w.is_finite() && b.h.is_finite() {
            Some(b)
        } else {
            None
        }
    });
    UiTreeNode {
        id,
        author_id,
        node_id: node_id.0,
        role: format!("{:?}", node.role()),
        label: node.label().map(|l| l.to_owned()),
        value: node.value().map(|v| v.to_owned()),
        disabled: node.is_disabled(),
        actions: node_actions(node),
        bounds,
        children: Vec::new(),
    }
}

/// One synthetic marker child appended when the node cap is hit, so a reader sees the tree was cut
/// rather than silently truncated.
fn overflow_marker() -> UiTreeNode {
    UiTreeNode {
        id: "snapshot:overflow".to_owned(),
        author_id: None,
        node_id: 0,
        role: "Unknown".to_owned(),
        label: Some("snapshot truncated (node cap reached)".to_owned()),
        value: None,
        disabled: false,
        actions: Vec::new(),
        bounds: None,
        children: Vec::new(),
    }
}

/// RFC3339-ish UTC timestamp from `SystemTime` without a chrono dependency:
/// `<unix_seconds>.<9-digit-nanos>Z`. Monotonic enough to distinguish snapshots; the exact calendar
/// rendering is a projection concern, not part of the stable tree contract.
fn now_utc_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => format!("{}.{:09}Z", d.as_secs(), d.subsec_nanos()),
        // Clock before the epoch is implausible on a real host; surface it rather than panic.
        Err(_) => "0.000000000Z".to_owned(),
    }
}

/// Project a live `accesskit::TreeUpdate` into a full, nested, JSON-serializable UI tree snapshot.
///
/// Walks from `update.tree.root` (or, if the update omits the tree header, the first node) over the
/// node set, attaching each node's children in their declared order. The walk is iterative and
/// cycle-guarded, and bounded by [`MAX_SNAPSHOT_NODES`].
///
/// Returns an empty-root snapshot (role `"Unknown"`, `widget_count` 1) only if the update has no
/// nodes at all — a degenerate case that does not occur on a real rendered frame but is handled
/// without panicking so callers never have to guard the model-vision path.
pub fn collect_ui_tree_snapshot(update: &accesskit::TreeUpdate) -> UiTreeSnapshot {
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        update.nodes.iter().map(|(id, node)| (*id, node)).collect();

    let root_id = update
        .tree
        .as_ref()
        .map(|t| t.root)
        .or_else(|| update.nodes.first().map(|(id, _)| *id));

    let root = match root_id.and_then(|id| by_id.get(&id).map(|n| (id, *n))) {
        Some((id, node)) => build_tree(id, node, &by_id),
        None => UiTreeNode {
            id: "node:empty".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Unknown".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children: Vec::new(),
        },
    };

    let widget_count = count_nodes(&root);
    UiTreeSnapshot {
        root,
        captured_at_utc: now_utc_string(),
        widget_count,
    }
}

/// One slot in the build arena: the (not-yet-assembled) node, plus the arena indices of its children
/// in declared (visual/logical) order, and whether its child list was truncated by the node cap.
struct ArenaSlot {
    node: UiTreeNode,
    child_slots: Vec<usize>,
    truncated: bool,
}

/// Iterative, cycle-guarded, cap-bounded tree build from a root node — fully safe (no recursion, no
/// raw pointers).
///
/// Phase 1 (top-down DFS): allocate an arena slot per reachable node, recording each node's child
/// slot indices in declared order. The work stack carries `(accesskit id, parent slot)`; children are
/// pushed in declared order so they are allocated in that order, and each parent records the new slot
/// in its `child_slots`.
///
/// Phase 2 (bottom-up assembly): because a child always gets a HIGHER arena index than its parent
/// (it is allocated after the parent), iterating slots high→low and `take()`-ing each finished
/// `UiTreeNode` into its parent's `children` (looked up by the parent's recorded `child_slots`)
/// assembles the whole tree in one pass, in declared order, with no recursion. Slot 0 (the root) is
/// the last one assembled and is returned.
fn build_tree(
    root_id: accesskit::NodeId,
    root_node: &accesskit::Node,
    by_id: &HashMap<accesskit::NodeId, &accesskit::Node>,
) -> UiTreeNode {
    use std::collections::HashSet;

    let mut arena: Vec<ArenaSlot> = vec![ArenaSlot {
        node: leaf_node(root_id, root_node),
        child_slots: Vec::new(),
        truncated: false,
    }];
    let mut visited: HashSet<accesskit::NodeId> = HashSet::new();
    visited.insert(root_id);

    // Phase 1: work stack of (accesskit id, parent arena slot). Push the root's children in declared
    // order; the stack pops LIFO, so to allocate children in declared order we reverse on push.
    let mut stack: Vec<(accesskit::NodeId, usize)> = root_node
        .children()
        .iter()
        .rev()
        .map(|c| (*c, 0usize))
        .collect();

    while let Some((node_id, parent_slot)) = stack.pop() {
        // Cap: once the arena is full, stop allocating real nodes and flag the parent so a single
        // overflow marker is appended during assembly (red-team: visible truncation, no silent cut).
        if arena.len() >= MAX_SNAPSHOT_NODES {
            arena[parent_slot].truncated = true;
            continue;
        }
        // Cycle guard: never expand a node twice (a well-formed egui tree is acyclic; this defends a
        // malformed update so the walk always terminates).
        if !visited.insert(node_id) {
            continue;
        }
        let Some(node) = by_id.get(&node_id).copied() else {
            // Child id with no matching node in the update (possible mid-update). Skip, do not invent.
            continue;
        };
        let slot = arena.len();
        arena.push(ArenaSlot {
            node: leaf_node(node_id, node),
            child_slots: Vec::new(),
            truncated: false,
        });
        arena[parent_slot].child_slots.push(slot);
        for child_id in node.children().iter().rev() {
            stack.push((*child_id, slot));
        }
    }

    // Phase 2: assemble bottom-up. Each node's children all have higher slot indices, so by the time
    // we reach slot N every slot > N is already a finished node we can move out via Option::take.
    let mut built: Vec<Option<UiTreeNode>> = Vec::with_capacity(arena.len());
    let mut child_slots: Vec<Vec<usize>> = Vec::with_capacity(arena.len());
    let mut truncated_flags: Vec<bool> = Vec::with_capacity(arena.len());
    for slot in arena {
        built.push(Some(slot.node));
        child_slots.push(slot.child_slots);
        truncated_flags.push(slot.truncated);
    }
    for slot in (0..built.len()).rev() {
        // Pull this node's finished children (declared order) out of their slots.
        let mut children: Vec<UiTreeNode> = child_slots[slot]
            .iter()
            .filter_map(|&cs| built[cs].take())
            .collect();
        if truncated_flags[slot] {
            children.push(overflow_marker());
        }
        if let Some(node) = built[slot].as_mut() {
            node.children = children;
        }
    }

    built[0]
        .take()
        .unwrap_or_else(|| leaf_node(root_id, root_node))
}

/// Total node count of a built subtree (inclusive), computed iteratively.
fn count_nodes(root: &UiTreeNode) -> usize {
    let mut count = 0usize;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        count += 1;
        for child in &node.children {
            stack.push(child);
        }
    }
    count
}

#[cfg(test)]
mod ui_tree_tests {
    use super::*;

    /// Build a `TreeUpdate` with `root` plus `child_count` direct children of the root, optionally
    /// giving the root extra deeply-nested structure. Each node carries a Button role + Click action so
    /// actions/serialization paths are exercised.
    fn node(role: accesskit::Role) -> accesskit::Node {
        accesskit::Node::new(role)
    }

    /// A small explicit tree: root(Window) -> tabbar(TabList, author "tabbar") -> tab(Tab, author "tab")
    /// and a sibling button(Button, author "btn", Click + bounds). Proves declared-order nesting,
    /// author_id vs synthetic id, actions, bounds, and round-trip — all on a controlled input.
    #[test]
    fn builds_nested_tree_with_actions_bounds_and_ids() {
        let mut update = accesskit::TreeUpdate {
            nodes: Vec::new(),
            tree: Some(accesskit::Tree::new(accesskit::NodeId(1))),
            focus: accesskit::NodeId(1),
        };
        let mut root = node(accesskit::Role::Window);
        root.set_children([accesskit::NodeId(2), accesskit::NodeId(4)]);
        update.nodes.push((accesskit::NodeId(1), root));

        let mut tabbar = node(accesskit::Role::TabList);
        tabbar.set_author_id("tabbar".to_owned());
        tabbar.set_children([accesskit::NodeId(3)]);
        update.nodes.push((accesskit::NodeId(2), tabbar));

        let mut tab = node(accesskit::Role::Tab);
        tab.set_author_id("tab".to_owned());
        tab.set_label("First".to_owned());
        update.nodes.push((accesskit::NodeId(3), tab));

        let mut btn = node(accesskit::Role::Button);
        btn.set_author_id("btn".to_owned());
        btn.add_action(accesskit::Action::Click);
        btn.set_bounds(accesskit::Rect {
            x0: 10.0,
            y0: 20.0,
            x1: 40.0,
            y1: 60.0,
        });
        update.nodes.push((accesskit::NodeId(4), btn));

        let snap = collect_ui_tree_snapshot(&update);
        assert_eq!(snap.widget_count, 4, "root + tabbar + tab + button");
        // Root is anonymous -> synthetic id, not empty.
        assert_eq!(snap.root.id, "node:1");
        assert_eq!(snap.root.author_id, None);
        // Declared order preserved: tabbar first, button second.
        assert_eq!(snap.root.children.len(), 2);
        assert_eq!(snap.root.children[0].author_id.as_deref(), Some("tabbar"));
        assert_eq!(snap.root.children[1].author_id.as_deref(), Some("btn"));
        // Tab is nested under the tabbar.
        let tabbar = &snap.root.children[0];
        assert_eq!(tabbar.children.len(), 1);
        assert_eq!(tabbar.children[0].author_id.as_deref(), Some("tab"));
        assert_eq!(tabbar.children[0].label.as_deref(), Some("First"));
        // Button carries Click and finite bounds.
        let btn = &snap.root.children[1];
        assert!(btn.actions.iter().any(|a| a == "Click"));
        let b = btn.bounds.expect("button has bounds");
        assert_eq!((b.x, b.y, b.w, b.h), (10.0, 20.0, 30.0, 40.0));
        // Round-trips losslessly.
        let restored: UiTreeSnapshot = serde_json::from_str(&snap.to_json()).unwrap();
        assert_eq!(restored, snap);
    }

    /// Non-finite bounds are dropped (so JSON round-trips) rather than serialized as `null` f32.
    #[test]
    fn non_finite_bounds_are_dropped() {
        let mut update = accesskit::TreeUpdate {
            nodes: Vec::new(),
            tree: Some(accesskit::Tree::new(accesskit::NodeId(1))),
            focus: accesskit::NodeId(1),
        };
        let mut root = node(accesskit::Role::Window);
        root.set_bounds(accesskit::Rect {
            x0: f64::NAN,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        });
        update.nodes.push((accesskit::NodeId(1), root));
        let snap = collect_ui_tree_snapshot(&update);
        assert_eq!(snap.root.bounds, None, "NaN bounds dropped");
        // Round-trips (would fail if NaN serialized to null and back to f32).
        let restored: UiTreeSnapshot = serde_json::from_str(&snap.to_json()).unwrap();
        assert_eq!(restored, snap);
    }

    /// The node cap bounds output and appends a single visible overflow marker (no silent truncation,
    /// no stack overflow). A flat root with more than `MAX_SNAPSHOT_NODES` children is truncated.
    #[test]
    fn node_cap_truncates_with_visible_marker() {
        let mut update = accesskit::TreeUpdate {
            nodes: Vec::new(),
            tree: Some(accesskit::Tree::new(accesskit::NodeId(1))),
            focus: accesskit::NodeId(1),
        };
        let total_children = MAX_SNAPSHOT_NODES + 50;
        let mut root = node(accesskit::Role::Window);
        let child_ids: Vec<accesskit::NodeId> =
            (2..(2 + total_children as u64)).map(accesskit::NodeId).collect();
        root.set_children(child_ids.clone());
        update.nodes.push((accesskit::NodeId(1), root));
        for id in &child_ids {
            update.nodes.push((*id, node(accesskit::Role::Button)));
        }

        let snap = collect_ui_tree_snapshot(&update);
        // Total nodes are bounded by the cap + the one synthetic overflow marker.
        assert!(
            snap.widget_count <= MAX_SNAPSHOT_NODES + 1,
            "widget_count {} must be bounded by the cap+marker",
            snap.widget_count
        );
        // Exactly one visible overflow marker was appended.
        let markers = snap
            .iter_nodes()
            .filter(|n| n.id == "snapshot:overflow")
            .count();
        assert_eq!(markers, 1, "one overflow marker appended");
    }

    /// A malformed update whose children form a cycle terminates (cycle guard) and never duplicates a
    /// node. root -> a -> b -> a (cycle back to a).
    #[test]
    fn cyclic_update_terminates_without_duplication() {
        let mut update = accesskit::TreeUpdate {
            nodes: Vec::new(),
            tree: Some(accesskit::Tree::new(accesskit::NodeId(1))),
            focus: accesskit::NodeId(1),
        };
        let mut root = node(accesskit::Role::Window);
        root.set_children([accesskit::NodeId(2)]);
        update.nodes.push((accesskit::NodeId(1), root));
        let mut a = node(accesskit::Role::Group);
        a.set_author_id("a".to_owned());
        a.set_children([accesskit::NodeId(3)]);
        update.nodes.push((accesskit::NodeId(2), a));
        let mut b = node(accesskit::Role::Group);
        b.set_author_id("b".to_owned());
        b.set_children([accesskit::NodeId(2)]); // cycle back to a
        update.nodes.push((accesskit::NodeId(3), b));

        let snap = collect_ui_tree_snapshot(&update);
        // root + a + b, each exactly once (the cycle edge is not re-expanded).
        assert_eq!(snap.widget_count, 3);
        assert_eq!(
            snap.iter_nodes()
                .filter(|n| n.author_id.as_deref() == Some("a"))
                .count(),
            1,
            "node 'a' appears exactly once despite the cycle"
        );
    }

    /// An empty update yields a degenerate but valid, round-trippable snapshot (no panic).
    #[test]
    fn empty_update_yields_valid_snapshot() {
        let update = accesskit::TreeUpdate {
            nodes: Vec::new(),
            tree: None,
            focus: accesskit::NodeId(0),
        };
        let snap = collect_ui_tree_snapshot(&update);
        assert_eq!(snap.widget_count, 1);
        let restored: UiTreeSnapshot = serde_json::from_str(&snap.to_json()).unwrap();
        assert_eq!(restored, snap);
    }
}
