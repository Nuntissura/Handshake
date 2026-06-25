//! Obsidian-class graph control panel (WP-KERNEL-012 MT-060, cluster E3).
//!
//! ## What this is
//!
//! [`GraphControls`] is the collapsible side control panel rendered alongside the MT-021
//! [`crate::graph::graph_view::LoomGraphView`] canvas. It adds the controls the bare Local/Global graph
//! lacked, bringing the surface to Obsidian graph parity:
//!
//! - a **search filter** that DIMS (never removes) non-matching nodes so spatial context is kept;
//! - **tag / folder GROUPS** that colour matching nodes and form a legend;
//! - a **link-depth slider** (Local mode only) that re-fires the SAME existing graph-search endpoint
//!   with a new `backlink_depth`;
//! - an **orphan toggle** that hides degree-0 nodes;
//! - a **size-by-degree toggle** that grows hub nodes.
//!
//! ## The pure / impure split (so the AC math is unit-testable without a render harness)
//!
//! Every filter/group/size DECISION is a pure function with NO `egui` types in its signature:
//! [`compute_visibility`], [`node_degree`], [`assign_group_color`], [`node_radius`]. These are what the
//! AC unit tests prove. [`GraphControls::show`] is the only `egui`-typed surface; it renders the panel,
//! mutates the controls' own state, and returns a [`GraphControlsEvent`] so the live
//! [`crate::graph::graph_view::LoomGraphView`] painter can apply the pure results (skip hidden nodes,
//! dim non-matches, colour by group, size by degree) and re-fire the backend ONLY on a depth change.
//!
//! ## No backend, no network (AC7 / AC8 / RISK-3 / MC-3)
//!
//! The panel performs NO network call and binds NO new endpoint. The ONLY backend interaction the whole
//! MT adds is the depth slider re-firing the EXISTING `GET /loom/graph-search?...&backlink_depth=` the
//! MT-021 client already owns — and only on slider RELEASE (debounced — RISK-2 / MC-2), only in Local
//! mode. Search / group / orphan / size are all CLIENT-SIDE over the already-loaded node/edge vecs.
//!
//! ## Group identity (RISK-1 / MC-1)
//!
//! A tag GROUP matches a node by the SAME tag identity the MT-023 tag tree uses (a tag-hub `title`,
//! carried on [`crate::graph::graph_view::GraphNode::tags`]); a folder GROUP matches by the SAME folder
//! path identity the MT-022 folder tree uses (the `loom_folders` path string on
//! [`crate::graph::graph_view::GraphNode::folder_path`], prefix-matched). Group keys are sanitized to
//! `[a-z0-9-]` via [`crate::project_tree::stable_part`] — the SAME slugger the graph nodes and trees use
//! — so a raw tag/folder string with slashes, colons, or spaces can never inject an unsafe AccessKit
//! author_id (RISK-5 / MC-5).
//!
//! ## Theme tokens only (CONTROL-4)
//!
//! Every group default colour comes from [`crate::theme::palette::graph_group_palette`] (the sanctioned
//! home for `Color32` literals); this module constructs NO `Color32::from_rgb` literal.

use std::collections::HashMap;

use egui::accesskit;
use egui::Color32;

use crate::graph::graph_view::{GraphEdge, GraphNode};

/// AccessKit author_id for the panel collapse/expand toggle button.
pub const TOGGLE_AUTHOR_ID: &str = "graph.controls.toggle";

/// AccessKit author_id for the search filter text field.
pub const SEARCH_AUTHOR_ID: &str = "graph.filter.search";

/// AccessKit author_id for the link-depth slider.
pub const DEPTH_AUTHOR_ID: &str = "graph.depth.slider";

/// AccessKit author_id for the show-orphans toggle.
pub const ORPHAN_AUTHOR_ID: &str = "graph.orphan.toggle";

/// AccessKit author_id for the size-by-degree toggle.
pub const SIZE_DEGREE_AUTHOR_ID: &str = "graph.size.degree";

/// AccessKit author_id prefix for a group enable toggle: `graph.group.{key}`.
pub const GROUP_AUTHOR_ID_PREFIX: &str = "graph.group.";

/// The minimum and maximum link depth the slider allows (the contract's `1..=5`).
pub const MIN_LINK_DEPTH: u32 = 1;
pub const MAX_LINK_DEPTH: u32 = 5;

/// The default link depth — matches the MT-021 `backlink_depth=2` default so the first Local fetch is
/// identical to the pre-MT-060 behaviour.
pub const DEFAULT_LINK_DEPTH: u32 = 2;

/// The alpha applied to a DIMMED node's fill/label (search non-matches, dimmed group members). Low enough
/// that the matched nodes clearly stand out, high enough that the dimmed node is still visible (Obsidian
/// keeps the spatial context — the node is dimmed, never removed).
pub const DIM_ALPHA: u8 = 70;

/// The stable AccessKit author_id for a group toggle: `graph.group.{sanitized_key}`. The key is sanitized
/// to `[a-z0-9-]` via [`crate::project_tree::stable_part`] (RISK-5 / MC-5) so a raw tag/folder string can
/// never inject an unsafe author_id.
pub fn group_author_id(key: &str) -> String {
    format!("{GROUP_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(key))
}

/// What KIND of group a [`GraphGroup`] is: a tag (MT-023 identity = the hub title) or a folder (MT-022
/// identity = the `loom_folders` path). The inner string is the RAW identity value the node carries, used
/// for matching; the [`GraphGroup::key`] is the sanitized stable identity used for the author_id + legend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupKind {
    /// A tag group; the inner string is the tag-hub title a node must carry in
    /// [`GraphNode::tags`](crate::graph::graph_view::GraphNode::tags) to match.
    Tag(String),
    /// A folder group; the inner string is the folder path a node's
    /// [`GraphNode::folder_path`](crate::graph::graph_view::GraphNode::folder_path) must START WITH to
    /// match (so a parent folder group colours its whole subtree).
    Folder(String),
}

impl GroupKind {
    /// The RAW identity value used for matching (the tag title or folder path).
    pub fn raw_value(&self) -> &str {
        match self {
            GroupKind::Tag(v) | GroupKind::Folder(v) => v,
        }
    }

    /// The stable kebab-case key prefix that distinguishes a tag group key from a folder group key so two
    /// different identities can never collide on the same sanitized key (`tag-` vs `folder-`).
    fn key_prefix(&self) -> &'static str {
        match self {
            GroupKind::Tag(_) => "tag",
            GroupKind::Folder(_) => "folder",
        }
    }
}

/// One colour GROUP in the control panel: a tag or folder identity, its assigned colour, its enabled
/// state, and its STABLE kebab-case key (used directly as the AccessKit author_id suffix and the legend
/// label identity). The key is derived once at discovery and is idempotent across re-loads (RISK-7 /
/// MC-7) so the user's enabled/colour choices survive a depth-change reload.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphGroup {
    /// Stable kebab-case identity, e.g. `tag-research`, `folder-src-frontend`. Used as the AccessKit
    /// author_id suffix (`graph.group.{key}`) and the legend label.
    pub key: String,
    pub kind: GroupKind,
    pub color: Color32,
    pub enabled: bool,
}

impl GraphGroup {
    /// Build a group from its kind + a default colour, deriving the stable kebab-case key from the kind's
    /// raw identity (sanitized via [`crate::project_tree::stable_part`], RISK-5 / MC-5). New groups are
    /// disabled by default so colouring is opt-in (an empty legend until the user enables a group).
    pub fn new(kind: GroupKind, color: Color32) -> Self {
        let key = format!(
            "{}-{}",
            kind.key_prefix(),
            crate::project_tree::stable_part(kind.raw_value())
        );
        Self {
            key,
            kind,
            color,
            enabled: false,
        }
    }

    /// A human-readable legend label: the raw tag/folder identity (the title or last path segment), with a
    /// leading `#` for tags and a trailing `/` cue for folders so the legend reads naturally.
    pub fn label(&self) -> String {
        match &self.kind {
            GroupKind::Tag(title) => format!("#{title}"),
            GroupKind::Folder(path) => {
                let last = path.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(path);
                format!("{last}/")
            }
        }
    }
}

/// The visibility decision for ONE node, the output of [`compute_visibility`]. A SEPARATE overlay value
/// keyed by `block_id` — it NEVER mutates the canonical loaded node/edge vecs the click/open + pan/zoom
/// math uses (RISK-6 / MC-6). `hidden` removes the node from the canvas entirely (orphan filter);
/// `dimmed` keeps it on the canvas at [`DIM_ALPHA`] (search non-match).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeVisibility {
    /// The node is removed from the canvas (and its incident edges skipped). Orphan filter only.
    pub hidden: bool,
    /// The node renders at reduced alpha (a search non-match) but stays on the canvas for spatial context.
    pub dimmed: bool,
}

/// The typed event [`GraphControls::show`] returns so the host can distinguish a network re-query from a
/// cheap client-side recompute. `DepthChanged` is the ONLY variant that touches the backend (a re-fire of
/// the existing graph-search endpoint with a new `backlink_depth`); `FiltersChanged` is a pure client-side
/// recompute of the visibility/colour/size overlay with ZERO network (AC7 / AC8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphControlsEvent {
    /// Nothing changed this frame.
    None,
    /// The link-depth slider was released at a new value (Local mode only). The host re-fires the existing
    /// `GET /loom/graph-search?...&backlink_depth={0}`.
    DepthChanged(u32),
    /// A client-side filter/group/orphan/size control changed. The host recomputes the visibility/colour/
    /// size overlay over the already-loaded vecs. NO network.
    FiltersChanged,
}

/// The control-panel state. Held by [`crate::graph::graph_view::LoomGraphView`] (a `controls` field) and
/// mutated in place by [`Self::show`]. All fields are ephemeral UI state — never persisted to the backend.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphControls {
    /// The case-insensitive substring search. Empty => nothing dimmed.
    pub search: String,
    /// The discovered tag/folder groups (populated lazily from the loaded nodes, idempotent across loads).
    pub groups: Vec<GraphGroup>,
    /// The current link depth (the graph-search `backlink_depth`). 1..=5; default 2.
    pub link_depth: u32,
    /// When false, degree-0 (orphan) nodes are hidden from the canvas.
    pub show_orphans: bool,
    /// When true, node radius scales with edge degree (hub emphasis).
    pub size_by_degree: bool,
    /// When false, the panel is collapsed to a single expand toggle (so it does not steal canvas space).
    pub panel_open: bool,
    /// The link_depth value the LAST `DepthChanged` was emitted for, so a depth re-query fires AT MOST once
    /// per committed depth (RISK-2 / MC-2 debounce): the slider only emits `DepthChanged` when its
    /// drag-released value differs from this.
    last_committed_depth: u32,
}

impl Default for GraphControls {
    fn default() -> Self {
        Self {
            search: String::new(),
            groups: Vec::new(),
            link_depth: DEFAULT_LINK_DEPTH,
            show_orphans: true,
            size_by_degree: false,
            panel_open: true,
            last_committed_depth: DEFAULT_LINK_DEPTH,
        }
    }
}

impl GraphControls {
    /// Discover candidate groups from the loaded nodes, IDEMPOTENTLY (RISK-7 / MC-7): a group whose stable
    /// key already exists is left untouched (its user-set `enabled`/`color` survive), and a group is added
    /// only for a NEW key. So re-loading the graph after a depth change never resets the user's configured
    /// view. Candidate tag groups come from the distinct tag identities present on the loaded nodes
    /// (MT-023 identity); candidate folder groups from the distinct folder-path identities (MT-022
    /// identity). Each new group gets a stable default colour from the theme palette by discovery order.
    pub fn discover_groups(&mut self, nodes: &[GraphNode]) {
        let palette = crate::theme::palette::graph_group_palette();
        // Collect distinct raw identities in a stable (sorted) order so the colour-by-discovery-order
        // assignment is deterministic across runs.
        let mut tags: Vec<&str> = nodes.iter().flat_map(|n| n.tags.iter().map(|s| s.as_str())).collect();
        tags.sort_unstable();
        tags.dedup();
        let mut folders: Vec<&str> =
            nodes.iter().filter_map(|n| n.folder_path.as_deref()).collect();
        folders.sort_unstable();
        folders.dedup();

        let candidate_kinds = tags
            .into_iter()
            .map(|t| GroupKind::Tag(t.to_owned()))
            .chain(folders.into_iter().map(|f| GroupKind::Folder(f.to_owned())));

        for kind in candidate_kinds {
            let color = palette[self.groups.len() % palette.len()];
            let candidate = GraphGroup::new(kind, color);
            // Idempotent insert keyed on the stable kebab-case key (RISK-7 / MC-7).
            if !self.groups.iter().any(|g| g.key == candidate.key) {
                self.groups.push(candidate);
            }
        }
    }

    /// Render the control panel into `ui` and return the typed [`GraphControlsEvent`] describing what
    /// changed this frame. Pure UI: performs NO network and mutates NO graph data directly — the host
    /// applies the returned event. Every interactive control emits a stable AccessKit author_id (AC6).
    ///
    /// `is_local` gates the depth slider: it is interactive ONLY in Local mode (Global already enumerates
    /// every node via `views/all`, so changing depth there is meaningless — it renders disabled).
    pub fn show(&mut self, ui: &mut egui::Ui, is_local: bool) -> GraphControlsEvent {
        let mut event = GraphControlsEvent::None;

        // Collapse / expand toggle — ALWAYS present (so a collapsed panel can be re-opened). When the
        // panel is closed, only this toggle renders, so it never steals canvas space.
        let toggle_label = if self.panel_open { "Controls ◂" } else { "Controls ▸" };
        let toggle = ui.button(toggle_label);
        emit_control_node(ui, toggle.id, TOGGLE_AUTHOR_ID, accesskit::Role::Button, "Toggle graph controls");
        if toggle.clicked() {
            self.panel_open = !self.panel_open;
        }
        if !self.panel_open {
            return event;
        }

        ui.separator();

        // ── Search filter ────────────────────────────────────────────────────────────────────────────
        ui.label("Filter");
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("search nodes…")
                .desired_width(f32::INFINITY),
        );
        emit_control_node(ui, search.id, SEARCH_AUTHOR_ID, accesskit::Role::TextInput, "Graph search filter");
        if search.changed() {
            event = GraphControlsEvent::FiltersChanged;
        }

        ui.separator();

        // ── Link-depth slider (Local mode only) ────────────────────────────────────────────────────────
        ui.label("Link depth");
        let mut depth = self.link_depth;
        let slider = ui.add_enabled(
            is_local,
            egui::Slider::new(&mut depth, MIN_LINK_DEPTH..=MAX_LINK_DEPTH).integer(),
        );
        emit_control_node(ui, slider.id, DEPTH_AUTHOR_ID, accesskit::Role::Slider, "Graph link depth");
        // The live value follows the drag so the UI feels responsive, but the BACKEND re-query fires only
        // on RELEASE (debounce — RISK-2 / MC-2): `drag_stopped()` is the commit, and we only emit when the
        // committed value actually differs from the last committed depth.
        self.link_depth = depth;
        if is_local && slider.drag_stopped() && self.link_depth != self.last_committed_depth {
            self.last_committed_depth = self.link_depth;
            event = GraphControlsEvent::DepthChanged(self.link_depth);
        }
        // A keyboard/scroll change that lands without a drag (the AccessKit `SetValue` / arrow-key path a
        // swarm agent uses) also commits on the frame it lands, so an out-of-process driver is not forced
        // to synthesize a drag-release. `changed()` fires once when the value settles for those inputs.
        if is_local
            && slider.changed()
            && !slider.dragged()
            && self.link_depth != self.last_committed_depth
        {
            self.last_committed_depth = self.link_depth;
            event = GraphControlsEvent::DepthChanged(self.link_depth);
        }

        ui.separator();

        // ── Orphan + size toggles ────────────────────────────────────────────────────────────────────
        let orphan = ui.checkbox(&mut self.show_orphans, "Show orphans");
        emit_control_node(ui, orphan.id, ORPHAN_AUTHOR_ID, accesskit::Role::CheckBox, "Show orphan nodes");
        if orphan.changed() {
            event = GraphControlsEvent::FiltersChanged;
        }

        let size = ui.checkbox(&mut self.size_by_degree, "Size by degree");
        emit_control_node(ui, size.id, SIZE_DEGREE_AUTHOR_ID, accesskit::Role::CheckBox, "Size nodes by degree");
        if size.changed() {
            event = GraphControlsEvent::FiltersChanged;
        }

        ui.separator();

        // ── Groups + legend ──────────────────────────────────────────────────────────────────────────
        ui.label("Groups");
        if self.groups.is_empty() {
            ui.weak("(no tags or folders on loaded nodes)");
        }
        // Iterate by index so we can mutate `enabled`/`color` in place while reading the stable key.
        for i in 0..self.groups.len() {
            let (key, label) = {
                let g = &self.groups[i];
                (g.key.clone(), g.label())
            };
            ui.horizontal(|ui| {
                let mut enabled = self.groups[i].enabled;
                let toggle = ui.checkbox(&mut enabled, "");
                emit_control_node(
                    ui,
                    toggle.id,
                    &group_author_id(&key),
                    accesskit::Role::CheckBox,
                    &format!("Group {label}"),
                );
                if toggle.changed() {
                    self.groups[i].enabled = enabled;
                    event = GraphControlsEvent::FiltersChanged;
                }
                // Colour swatch (an egui colour-edit button) + the legend label.
                let mut color = self.groups[i].color;
                if ui.color_edit_button_srgba(&mut color).changed() {
                    self.groups[i].color = color;
                    event = GraphControlsEvent::FiltersChanged;
                }
                ui.label(label);
            });
        }

        event
    }

    /// The set of currently-enabled groups, in vec order (the deterministic first-match-wins resolution
    /// order for [`assign_group_color`]). Cheap clone of the small group set; used by the host to build the
    /// per-node colour overlay each `FiltersChanged`.
    pub fn enabled_groups(&self) -> Vec<GraphGroup> {
        self.groups.iter().filter(|g| g.enabled).cloned().collect()
    }
}

/// Count the edges incident to `node_id` (an edge whose `source` OR `target` equals `node_id`). A self-
/// loop (source == target == node_id) counts ONCE — it is one incident edge. Pure; no egui.
pub fn node_degree(node_id: &str, edges: &[GraphEdge]) -> usize {
    edges
        .iter()
        .filter(|e| e.source == node_id || e.target == node_id)
        .count()
}

/// Compute the visibility overlay for every node, the pure core of AC1 (search dim) + AC2 (orphan hide).
/// Returns a map `block_id -> NodeVisibility`. A SEPARATE overlay — it NEVER mutates the node/edge vecs
/// (RISK-6 / MC-6).
///
/// Rules:
/// - `search` (case-insensitive substring over the node TITLE): when non-empty, every node whose title
///   does NOT contain the substring is `dimmed` (reduced alpha, kept on the canvas for spatial context).
///   When `search` is empty, nothing is dimmed.
/// - `show_orphans = false`: any node whose edge degree is 0 is `hidden` (removed from the canvas). When
///   true, no node is hidden by the orphan rule.
///
/// A node can be both hidden (orphan) and would-be-dimmed; `hidden` dominates at render (a hidden node is
/// not drawn at all).
pub fn compute_visibility(
    nodes: &[GraphNode],
    edges: &[GraphEdge],
    search: &str,
    show_orphans: bool,
) -> HashMap<String, NodeVisibility> {
    let needle = search.trim().to_lowercase();
    let dim_active = !needle.is_empty();
    let mut map = HashMap::with_capacity(nodes.len());
    for node in nodes {
        let mut vis = NodeVisibility::default();
        if dim_active && !node.title.to_lowercase().contains(&needle) {
            vis.dimmed = true;
        }
        if !show_orphans && node_degree(&node.block_id, edges) == 0 {
            vis.hidden = true;
        }
        map.insert(node.block_id.clone(), vis);
    }
    map
}

/// The colour for a node from the FIRST enabled matching group, or `None` if no enabled group matches
/// (the caller then falls back to the MT-021 `content_type` colour). Pure; no egui beyond [`Color32`].
///
/// Matching (RISK-1 / MC-1 — reuses the SAME identity the trees use, NOT re-derived from raw strings):
/// - a [`GroupKind::Tag`] group matches when the node carries that tag identity in
///   [`GraphNode::tags`](crate::graph::graph_view::GraphNode::tags) (exact tag-title match, the MT-023
///   identity the tag tree uses);
/// - a [`GroupKind::Folder`] group matches when the node's
///   [`GraphNode::folder_path`](crate::graph::graph_view::GraphNode::folder_path) equals the folder key OR
///   lies inside its subtree (the MT-022 identity the folder tree uses), so a parent-folder group colours
///   its whole subtree — but a sibling folder that merely shares a string prefix does NOT bleed in. The
///   match is at a PATH-SEGMENT boundary: `src/front` colours `src/front` and `src/front/x` but NEVER
///   `src/frontend` (the prefix-boundary guard — the raw `starts_with` bleed the trees would not produce).
///
/// Resolution order is the slice order of `groups` (first enabled match wins — the deterministic order the
/// legend renders in), so the caller passes the enabled groups in vec order.
pub fn assign_group_color(node: &GraphNode, groups: &[GraphGroup]) -> Option<Color32> {
    for group in groups.iter().filter(|g| g.enabled) {
        let matches = match &group.kind {
            GroupKind::Tag(title) => node.tags.iter().any(|t| t == title),
            GroupKind::Folder(path) => node
                .folder_path
                .as_deref()
                .is_some_and(|fp| folder_path_in_subtree(fp, path)),
        };
        if matches {
            return Some(group.color);
        }
    }
    None
}

/// Does folder path `fp` lie at or under the folder-group key `path`, matched at a PATH-SEGMENT boundary?
/// `true` when `fp == path` (the folder itself) or `fp` starts with `path` followed by a `/` (a descendant
/// in the subtree). A bare string `starts_with` would wrongly colour a SIBLING that merely shares a string
/// prefix (`src/front` matching `src/frontend`); this boundary guard rejects that (RISK-1 / MC-1 — the same
/// subtree semantics the MT-022 folder tree uses, not a raw string prefix). Pure; no egui.
pub fn folder_path_in_subtree(fp: &str, path: &str) -> bool {
    fp == path || fp.starts_with(&format!("{path}/"))
}

/// The node circle radius for the given `base` radius and edge `degree`. When `size_by_degree` is false,
/// returns `base` (the MT-021 18px default). When true, scales by `sqrt(degree)` so hub nodes read larger,
/// CLAMPED to `[base, base * 3.0]` (AC5). Pure; no egui.
///
/// The `sqrt` curve (not linear) keeps a very high-degree hub from dominating the canvas while still making
/// degree differences legible: `base * (1 + sqrt(degree) * 0.35)`, clamped to 3x base.
pub fn node_radius(base: f32, degree: usize, size_by_degree: bool) -> f32 {
    if !size_by_degree {
        return base;
    }
    let scaled = base * (1.0 + (degree as f32).sqrt() * 0.35);
    scaled.clamp(base, base * 3.0)
}

/// Emit a control's live AccessKit node (role + author_id + Click/Focus actions) so a swarm agent can
/// address it by stable id (AC6 / HBR-SWARM). Mirrors the MT-021 `emit_toolbar_node` pattern: the control
/// already has an `egui::Id` from its `Response`, so we attach the AccessKit identity to that exact id.
fn emit_control_node(ui: &egui::Ui, id: egui::Id, author_id: &str, role: accesskit::Role, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(role);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
        node.add_action(accesskit::Action::Focus);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::graph_view::{GraphEdge, GraphNode};
    use crate::theme::palette::graph_group_palette;

    fn n(id: &str, title: &str) -> GraphNode {
        GraphNode::new(id, title, "note")
    }

    // ── node_degree ───────────────────────────────────────────────────────────────────────────────

    #[test]
    fn node_degree_counts_incident_edges() {
        let edges = vec![
            GraphEdge::new("a", "b", "mention"),
            GraphEdge::new("b", "c", "mention"),
            GraphEdge::new("c", "a", "mention"),
        ];
        assert_eq!(node_degree("a", &edges), 2, "a is in (a,b) and (c,a)");
        assert_eq!(node_degree("b", &edges), 2);
        assert_eq!(node_degree("z", &edges), 0, "an orphan has degree 0");
    }

    #[test]
    fn node_degree_self_loop_counts_once() {
        let edges = vec![GraphEdge::new("a", "a", "mention")];
        assert_eq!(node_degree("a", &edges), 1, "a self-loop is one incident edge");
    }

    // ── compute_visibility: AC1 search dim ──────────────────────────────────────────────────────────

    #[test]
    fn search_dims_non_matching_keeps_matching_full() {
        let nodes = vec![n("a", "Research notes"), n("b", "Daily log"), n("c", "research plan")];
        let edges = vec![GraphEdge::new("a", "b", "mention")];
        // Case-insensitive substring "research" matches a + c, not b.
        let vis = compute_visibility(&nodes, &edges, "research", true);
        assert!(!vis["a"].dimmed, "AC1: a matches -> full opacity");
        assert!(!vis["c"].dimmed, "AC1: c matches (case-insensitive) -> full opacity");
        assert!(vis["b"].dimmed, "AC1: b does not match -> dimmed");
        // Nobody hidden (orphans shown).
        assert!(vis.values().all(|v| !v.hidden));
    }

    #[test]
    fn empty_search_dims_nothing() {
        let nodes = vec![n("a", "Alpha"), n("b", "Beta")];
        let vis = compute_visibility(&nodes, &[], "", true);
        assert!(vis.values().all(|v| !v.dimmed), "AC1: empty search dims nothing");
        let vis2 = compute_visibility(&nodes, &[], "   ", true);
        assert!(vis2.values().all(|v| !v.dimmed), "AC1: whitespace-only search dims nothing");
    }

    // ── compute_visibility: AC2 orphan hide ─────────────────────────────────────────────────────────

    #[test]
    fn orphan_off_hides_degree_zero_nodes() {
        // a-b connected; c is an orphan (degree 0).
        let nodes = vec![n("a", "A"), n("b", "B"), n("c", "C")];
        let edges = vec![GraphEdge::new("a", "b", "mention")];
        let off = compute_visibility(&nodes, &edges, "", false);
        assert!(!off["a"].hidden, "AC2: connected node visible");
        assert!(!off["b"].hidden, "AC2: connected node visible");
        assert!(off["c"].hidden, "AC2: orphan hidden when show_orphans=false");
        // With orphans ON, nothing hidden.
        let on = compute_visibility(&nodes, &edges, "", true);
        assert!(on.values().all(|v| !v.hidden), "AC2: all visible when show_orphans=true");
    }

    // ── assign_group_color: AC3 tag + folder + no-match fallback ────────────────────────────────────

    #[test]
    fn assign_group_color_tag_match() {
        let palette = graph_group_palette();
        let mut group = GraphGroup::new(GroupKind::Tag("research".to_owned()), palette[0]);
        group.enabled = true;
        let tagged = n("a", "A").with_tags(vec!["research".to_owned(), "ml".to_owned()]);
        let untagged = n("b", "B").with_tags(vec!["ops".to_owned()]);
        assert_eq!(assign_group_color(&tagged, std::slice::from_ref(&group)), Some(palette[0]), "AC3: tag match -> group color");
        assert_eq!(assign_group_color(&untagged, std::slice::from_ref(&group)), None, "AC3: no tag match -> None (content_type fallback)");
    }

    #[test]
    fn assign_group_color_folder_prefix_match() {
        let palette = graph_group_palette();
        let mut group = GraphGroup::new(GroupKind::Folder("src/frontend".to_owned()), palette[1]);
        group.enabled = true;
        let inside = n("a", "A").with_folder_path("src/frontend/handshake_native");
        let exact = n("b", "B").with_folder_path("src/frontend");
        let outside = n("c", "C").with_folder_path("src/backend");
        let no_folder = n("d", "D");
        assert_eq!(assign_group_color(&inside, std::slice::from_ref(&group)), Some(palette[1]), "AC3: folder subtree match");
        assert_eq!(assign_group_color(&exact, std::slice::from_ref(&group)), Some(palette[1]), "AC3: exact folder match");
        assert_eq!(assign_group_color(&outside, std::slice::from_ref(&group)), None, "AC3: sibling folder no match");
        assert_eq!(assign_group_color(&no_folder, std::slice::from_ref(&group)), None, "AC3: no folder -> None");
    }

    #[test]
    fn assign_group_color_folder_prefix_no_false_positive() {
        // RISK-1 / MC-1 boundary guard: a folder group keyed "src/front" must NOT colour a SIBLING folder
        // "src/frontend" that merely shares a raw string prefix. The match is at a PATH-SEGMENT boundary
        // (the same subtree semantics the MT-022 folder tree uses), so a string-prefix bleed cannot occur.
        let palette = graph_group_palette();
        let mut group = GraphGroup::new(GroupKind::Folder("src/front".to_owned()), palette[2]);
        group.enabled = true;
        let sibling = n("a", "A").with_folder_path("src/frontend");
        assert_eq!(
            assign_group_color(&sibling, std::slice::from_ref(&group)),
            None,
            "AC3/RISK-1: 'src/frontend' is a SIBLING of the 'src/front' group, not a descendant — the \
             path-segment boundary guard must reject the raw-prefix bleed"
        );
        // The folder itself and a genuine descendant DO match (subtree colouring still works).
        let exact = n("b", "B").with_folder_path("src/front");
        let child = n("c", "C").with_folder_path("src/front/widgets");
        assert_eq!(assign_group_color(&exact, std::slice::from_ref(&group)), Some(palette[2]), "AC3: the folder itself matches");
        assert_eq!(assign_group_color(&child, std::slice::from_ref(&group)), Some(palette[2]), "AC3: a real descendant matches");
    }

    #[test]
    fn folder_path_in_subtree_boundary() {
        // Direct unit coverage of the boundary predicate (RISK-1 / MC-1).
        assert!(folder_path_in_subtree("src/front", "src/front"), "the folder itself");
        assert!(folder_path_in_subtree("src/front/a", "src/front"), "a descendant");
        assert!(folder_path_in_subtree("src/front/a/b", "src/front"), "a deep descendant");
        assert!(!folder_path_in_subtree("src/frontend", "src/front"), "a sibling sharing a string prefix must NOT match");
        assert!(!folder_path_in_subtree("src", "src/front"), "an ancestor must NOT match");
        assert!(!folder_path_in_subtree("docs/front", "src/front"), "an unrelated path must NOT match");
    }

    #[test]
    fn assign_group_color_disabled_group_does_not_match() {
        let palette = graph_group_palette();
        let group = GraphGroup::new(GroupKind::Tag("research".to_owned()), palette[0]); // enabled=false
        let tagged = n("a", "A").with_tags(vec!["research".to_owned()]);
        assert_eq!(assign_group_color(&tagged, std::slice::from_ref(&group)), None, "a disabled group never colours");
    }

    #[test]
    fn assign_group_color_first_enabled_match_wins() {
        let palette = graph_group_palette();
        let mut g0 = GraphGroup::new(GroupKind::Tag("research".to_owned()), palette[0]);
        let mut g1 = GraphGroup::new(GroupKind::Tag("ml".to_owned()), palette[3]);
        g0.enabled = true;
        g1.enabled = true;
        let node = n("a", "A").with_tags(vec!["ml".to_owned(), "research".to_owned()]);
        // node carries both tags; the FIRST enabled group in slice order wins (research = palette[0]).
        assert_eq!(assign_group_color(&node, &[g0.clone(), g1.clone()]), Some(palette[0]));
        // Reversing the order changes the winner (deterministic resolution by slice order).
        assert_eq!(assign_group_color(&node, &[g1, g0]), Some(palette[3]));
    }

    // ── node_radius: AC5 degree scaling + clamp ─────────────────────────────────────────────────────

    #[test]
    fn node_radius_off_is_base() {
        assert_eq!(node_radius(18.0, 0, false), 18.0);
        assert_eq!(node_radius(18.0, 50, false), 18.0, "size_by_degree off ignores degree");
    }

    #[test]
    fn node_radius_scales_with_degree_and_clamps() {
        let base = 18.0;
        let r0 = node_radius(base, 0, true);
        let r4 = node_radius(base, 4, true);
        let r25 = node_radius(base, 25, true);
        assert_eq!(r0, base, "AC5: degree-0 node uses base radius even when size_by_degree on");
        assert!(r4 > r0, "AC5: higher degree -> strictly larger radius ({r4} > {r0})");
        assert!(r25 > r4, "AC5: monotonic growth ({r25} > {r4})");
        // Clamp: a huge degree never exceeds 3x base.
        let r_huge = node_radius(base, 100_000, true);
        assert!(r_huge <= base * 3.0 + 1e-3, "AC5: radius clamped to <= 3x base (got {r_huge})");
        assert!(r_huge >= base, "AC5: radius never below base");
    }

    #[test]
    fn node_radius_higher_degree_strictly_larger_than_orphan() {
        // The exact AC5 assertion: a higher-degree node's radius is strictly larger than a degree-0 node.
        let base = 18.0;
        let orphan = node_radius(base, 0, true);
        let hub = node_radius(base, 9, true);
        assert!(hub > orphan, "AC5: hub radius {hub} must be strictly > orphan radius {orphan}");
    }

    // ── group key sanitization: RISK-5 / MC-5 ───────────────────────────────────────────────────────

    #[test]
    fn group_key_and_author_id_are_sanitized() {
        let g = GraphGroup::new(GroupKind::Folder("src/Frontend: Native!".to_owned()), graph_group_palette()[0]);
        // The key has the folder- prefix + a sanitized [a-z0-9-] body.
        assert!(g.key.starts_with("folder-"), "folder key prefix");
        let body = &g.key["folder-".len()..];
        assert!(
            body.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "RISK-5: group key body must be [a-z0-9-], got '{body}'"
        );
        let author = group_author_id(&g.key);
        assert!(author.starts_with(GROUP_AUTHOR_ID_PREFIX));
        let suffix = &author[GROUP_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "RISK-5: author_id suffix must be [a-z0-9-], got '{suffix}'"
        );
    }

    #[test]
    fn tag_and_folder_keys_never_collide_on_same_string() {
        // A tag named "research" and a folder named "research" must yield DISTINCT keys (tag- vs folder-)
        // so they cannot collide on the same AccessKit author_id.
        let tag = GraphGroup::new(GroupKind::Tag("research".to_owned()), graph_group_palette()[0]);
        let folder = GraphGroup::new(GroupKind::Folder("research".to_owned()), graph_group_palette()[1]);
        assert_ne!(tag.key, folder.key, "tag and folder keys must not collide");
        assert_eq!(tag.key, "tag-research");
        assert_eq!(folder.key, "folder-research");
    }

    // ── discover_groups: idempotency RISK-7 / MC-7 ──────────────────────────────────────────────────

    #[test]
    fn discover_groups_is_idempotent_and_preserves_user_state() {
        let nodes = vec![
            n("a", "A").with_tags(vec!["research".to_owned()]).with_folder_path("src/frontend"),
            n("b", "B").with_tags(vec!["ops".to_owned()]),
        ];
        let mut controls = GraphControls::default();
        controls.discover_groups(&nodes);
        // research, ops (tags) + src/frontend (folder) = 3 groups.
        assert_eq!(controls.groups.len(), 3, "distinct tag + folder identities discovered");
        // User enables + recolours the research group.
        let custom = graph_group_palette()[5];
        {
            let g = controls.groups.iter_mut().find(|g| g.key == "tag-research").unwrap();
            g.enabled = true;
            g.color = custom;
        }
        // Re-discover (simulating a depth-change reload): NO duplicates, user state survives (RISK-7/MC-7).
        controls.discover_groups(&nodes);
        assert_eq!(controls.groups.len(), 3, "re-discovery does not duplicate groups (idempotent)");
        let g = controls.groups.iter().find(|g| g.key == "tag-research").unwrap();
        assert!(g.enabled, "MC-7: user-enabled state survives reload");
        assert_eq!(g.color, custom, "MC-7: user color survives reload");
    }

    #[test]
    fn discover_groups_empty_when_no_identity() {
        // Nodes with no tags + no folder path -> no groups (the empty-legend case).
        let nodes = vec![n("a", "A"), n("b", "B")];
        let mut controls = GraphControls::default();
        controls.discover_groups(&nodes);
        assert!(controls.groups.is_empty(), "no tag/folder identity -> no candidate groups");
    }

    // ── defaults match the MT contract ──────────────────────────────────────────────────────────────

    #[test]
    fn defaults_match_contract() {
        let c = GraphControls::default();
        assert_eq!(c.search, "");
        assert!(c.groups.is_empty());
        assert_eq!(c.link_depth, DEFAULT_LINK_DEPTH, "default depth 2 matches MT-021 backlink_depth");
        assert_eq!(c.link_depth, 2);
        assert!(c.show_orphans, "orphans shown by default");
        assert!(!c.size_by_degree, "size-by-degree off by default");
        assert!(c.panel_open, "panel open by default");
    }
}
