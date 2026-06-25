//! Local + global Loom graph view (WP-KERNEL-012 MT-021, cluster E3).
//!
//! ## What this is
//!
//! [`LoomGraphView`] is a native, force-directed node-link diagram of Loom blocks (nodes) and the
//! edges between them, rendered entirely with [`egui::Painter`] (no third-party graph library — the
//! MT `implementation_notes` constraint). It is the primary wayfinding surface for the Obsidian-class
//! knowledge layer: every other E3 MT (folder tree, tags, breadcrumbs, canvas) hangs navigation off
//! this graph.
//!
//! It binds the REAL PostgreSQL/EventLedger backend through the WP-011
//! [`crate::backend_client::LoomGraphClient`] (added by this MT alongside the widget): Global mode
//! enumerates `GET /workspaces/{id}/loom/views/all`; Local mode fetches the focused block's
//! neighbourhood via `GET /workspaces/{id}/loom/graph-search?q={title}&backlink_depth=2&limit=200`.
//! There is NO Tauri command anywhere (the contract's step-3 "Tauri" reference is the LEGACY
//! React/webview stack; the KERNEL_BUILDER gate corrected it to backend_client.rs — the same client
//! MT-008/014/015/017 used).
//!
//! ## Repaint discipline (the MT-015 idle-repaint lesson applied to the animation)
//!
//! The spring/force layout requests `ctx.request_repaint()` ONLY while it has NOT converged (per-node
//! step < [`CONVERGENCE_EPS`] px) AND the iteration count is below [`MAX_LAYOUT_ITERS`]. Once either
//! stop condition holds, layout STOPS requesting repaint — a layout that animated every frame forever
//! would burn idle CPU and make a kittest `harness.run()` exceed its step cap (the backlinks-spinner
//! regression class). The loading indicator likewise animates ONLY during a genuine in-flight backend
//! fetch (runtime present + a request dispatched); a headless / no-runtime render shows a neutral,
//! non-animating "no backend" state, never a perpetual spinner.
//!
//! ## AccessKit (HBR-SWARM)
//!
//! Every toolbar control (`graph.mode.local`, `graph.mode.global`, `graph.zoom.in`, `graph.zoom.out`,
//! `graph.relayout`) and every rendered node (`graph.node.{sanitized_block_id}`, Role::Button, label =
//! title, Action::Click) emits a live AccessKit node through egui's own
//! [`egui::Context::accesskit_node_builder`] hook so an out-of-process swarm agent can read the graph
//! and click a node by stable id. Block ids are sanitized to `[a-z0-9-]` via
//! [`crate::project_tree::stable_part`] before forming the author_id suffix (RISK-3 / MC-3): a raw id
//! with slashes or colons can never break AccessKit-tree integrity.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use egui::accesskit;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::accessibility::knowledge_action_registry::{
    self, AddEdgePayload, AxRole as KAxRole, BlockIdPayload, EdgeIdPayload, KnowledgeActionRegistry,
    KnowledgeNodeState, GRAPH_CONTROL_CATALOG, VIEWPORT_LOOKAHEAD,
};
// WP-KERNEL-012 MT-060: the Obsidian-class control panel + its pure filter/group/sizing fns. The view
// OWNS a `GraphControls`, renders it each frame, and CONSUMES the pure results in the live painter pass.
use crate::graph::graph_controls::{
    assign_group_color, compute_visibility, node_degree, node_radius, GraphControls,
    GraphControlsEvent, NodeVisibility, DIM_ALPHA,
};
use crate::theme::HsPalette;

/// Default node circle radius in WORLD space (px before zoom). Click detection uses this same radius
/// after inverse-transforming the pointer (RISK-4).
pub const NODE_RADIUS: f32 = 18.0;

/// Hard cap on loaded nodes (RISK-5 / MC-2). A naive O(n^2) repulsion is fine up to a couple hundred
/// nodes; beyond this the graph clamps and shows a "showing N of M" truncation notice.
pub const NODE_CAP: usize = 200;

/// Total force-layout iteration budget across all frames (PROOF1 convergence ceiling). Once reached,
/// layout stops regardless of convergence so it can never animate forever (idle-repaint discipline).
pub const MAX_LAYOUT_ITERS: usize = 300;

/// Per-frame iteration cap (RISK-1 / MC-1): never run more than this many force steps in one frame so
/// a big graph cannot stall egui at 60fps. The remaining budget is consumed over subsequent frames.
pub const ITERS_PER_FRAME: usize = 10;

/// Convergence epsilon (px): when the largest single-node displacement in an iteration drops below
/// this, the layout is "stable" and stops requesting repaint (PROOF1 asserts < 1px after the budget).
pub const CONVERGENCE_EPS: f32 = 1.0;

/// Min / max zoom (AC4 clamp).
pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 4.0;

/// Toolbar AccessKit author_ids (stable strings; live in egui's hashed id space — the dynamic-id
/// pattern the shell registry documents for non-fixed-band controls).
pub const MODE_LOCAL_AUTHOR_ID: &str = "graph.mode.local";
pub const MODE_GLOBAL_AUTHOR_ID: &str = "graph.mode.global";
pub const ZOOM_IN_AUTHOR_ID: &str = "graph.zoom.in";
pub const ZOOM_OUT_AUTHOR_ID: &str = "graph.zoom.out";
pub const RELAYOUT_AUTHOR_ID: &str = "graph.relayout";

/// Author_id prefix for a graph node. The full id is `graph.node.{sanitized_block_id}`.
pub const NODE_AUTHOR_ID_PREFIX: &str = "graph.node.";

/// The stable AccessKit author_id for a graph node, sanitizing `block_id` to `[a-z0-9-]` (RISK-3 /
/// MC-3). Reuses the shell's [`crate::project_tree::stable_part`] slugger so a block_id with slashes
/// or colons can never inject an unsafe author_id.
pub fn node_author_id(block_id: &str) -> String {
    format!("{NODE_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(block_id))
}

/// Which graph the view is showing. `Local` is the neighbourhood of a focused block (graph-search);
/// `Global` is the full workspace (views/all). Switching modes triggers a re-fetch + re-layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphMode {
    /// Neighbourhood of one focused block. `title` is the graph-search query term (`q=`); `block_id`
    /// is the focused block whose neighbourhood is shown.
    Local { block_id: String, title: String },
    /// The full workspace graph (all blocks).
    Global,
}

impl GraphMode {
    fn is_local(&self) -> bool {
        matches!(self, GraphMode::Local { .. })
    }
}

/// One graph node: a Loom block placed in WORLD space. Positions are EPHEMERAL UI state (re-run on
/// open) and never persisted to the backend (the MT "do not store node positions in backend" rule).
///
/// ## Group-identity fields (WP-KERNEL-012 MT-060)
///
/// `tags` and `folder_path` are the OPTIONAL group-identity the MT-060 control panel matches against
/// for tag/folder GROUP coloring. They default EMPTY: the `views/all` / `graph-search` payload the
/// MT-021 backend client parses (`backend_client::block_to_node`) carries ONLY `block_id` / `title` /
/// `content_type` — it does NOT yet expose per-node tag identity or folder-path identity. So the node's
/// group identity is host-populated from the SAME identity surfaces the trees use (MT-023 tag identity =
/// a tag-hub's `title`; MT-022 folder identity = the `loom_folders` folder-path string sanitized by
/// [`crate::project_tree::stable_part`]) via [`GraphNode::with_tags`] / [`GraphNode::with_folder_path`],
/// NEVER re-derived from raw strings inside the widget (RISK-1 / MC-1). When the host has no tag/folder
/// identity for a node (the current backend gap — disclosed as a TYPED BLOCKER in the MT handoff), the
/// node simply matches no group and falls back to its `content_type` colour, exactly as before. This is
/// a CLIENT-SIDE field on the already-loaded vec — it adds NO backend endpoint and NO network call.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub block_id: String,
    pub title: String,
    /// Loom `content_type` string (note/file/tag_hub/journal/canvas/view_def/...). Drives the node
    /// colour via [`content_type_color`].
    pub content_type: String,
    /// The tag-hub identities this node carries (MT-023 tag identity = the hub title). Empty by default;
    /// host-populated. Used by [`graph_controls::assign_group_color`] for tag GROUP matching.
    pub tags: Vec<String>,
    /// The node's folder-path identity (MT-022 folder identity, the `loom_folders` path string). `None`
    /// by default; host-populated. Used by [`graph_controls::assign_group_color`] for folder GROUP
    /// matching (a folder group matches when this path starts with the folder key).
    pub folder_path: Option<String>,
    pub x: f32,
    pub y: f32,
}

impl GraphNode {
    pub fn new(block_id: impl Into<String>, title: impl Into<String>, content_type: impl Into<String>) -> Self {
        Self {
            block_id: block_id.into(),
            title: title.into(),
            content_type: content_type.into(),
            tags: Vec::new(),
            folder_path: None,
            x: 0.0,
            y: 0.0,
        }
    }

    /// Builder: attach the node's tag-hub identities (MT-023 identity surface). The host calls this when
    /// it knows a node's tags so a tag GROUP can colour it (RISK-1 / MC-1 — same identity the tag tree
    /// uses). Chainable; replaces any prior tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: attach the node's folder-path identity (MT-022 identity surface). The host calls this
    /// when it knows a node's folder so a folder GROUP can colour it (RISK-1 / MC-1 — same path identity
    /// the folder tree uses). Chainable.
    pub fn with_folder_path(mut self, folder_path: impl Into<String>) -> Self {
        self.folder_path = Some(folder_path.into());
        self
    }

    fn pos(&self) -> Pos2 {
        Pos2::new(self.x, self.y)
    }
}

/// One graph edge between two block ids. `edge_type` is the Loom edge type string (mention/tag/...);
/// kept for future colour-by-edge-type but not yet rendered distinctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,
}

impl GraphEdge {
    pub fn new(source: impl Into<String>, target: impl Into<String>, edge_type: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            edge_type: edge_type.into(),
        }
    }
}

/// Map a Loom `content_type` to a node colour DERIVED FROM the live theme palette (no hardcoded hex in
/// this widget — the theme/syntax no-hardcode invariant). The MT colour intent
/// (note=blue, file=gray, tag_hub=green, journal=orange, canvas=purple, other=slate) is realised by
/// picking the closest existing semantic token rather than inventing literals:
///   - note -> `syntax.keyword` (the theme's blue)
///   - file -> `text_subtle` (gray)
///   - tag_hub -> `success_text` (green)
///   - journal -> `diagnostics.warning` (the theme's amber/yellow — closest to "orange")
///   - canvas -> `graph_canvas`, a derived violet/plum token (accent blended with the breakpoint red);
///     the blend is computed inside `palette.rs` so this widget holds no `Color32` literal
///   - other -> `border_strong` (slate)
pub fn content_type_color(content_type: &str, palette: &HsPalette) -> Color32 {
    match content_type {
        "note" => palette.syntax.keyword,
        "file" | "annotated_file" => palette.text_subtle,
        "tag_hub" => palette.success_text,
        "journal" => palette.diagnostics.warning,
        // "purple" for canvas: a derived theme token (accent blended with the breakpoint red) so the
        // result leans violet on either theme without this widget constructing a Color32. The blend
        // lives in palette.rs (the sanctioned home); the graph widget only reads the token.
        "canvas" => palette.graph_canvas,
        _ => palette.border_strong,
    }
}

/// The typed event a graph interaction produces this frame, for the host to apply. `OpenNode` is the
/// AC5 click-to-open; `ModeChanged`/`Relayout` let the host re-fetch when the toolbar drives a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphEvent {
    /// A node was clicked: open the block in the active pane (the cross-pane open the MT names).
    OpenNode { block_id: String },
    /// The Local/Global toggle changed; host should re-fetch for the new mode.
    ModeChanged { to_global: bool },
    /// The Re-layout button was pressed; positions were reset and layout restarts.
    Relayout,
    /// WP-KERNEL-012 MT-060: the link-depth slider was released at a new value in Local mode. The host
    /// re-fires the EXISTING `GET /loom/graph-search?q={focused_title}&limit=&backlink_depth={depth}` and
    /// replaces the node/edge set (then `set_graph` re-runs the force layout). NO new endpoint. In Global
    /// mode the slider is disabled and this event never fires.
    DepthChanged { depth: u32 },
    /// WP-KERNEL-012 MT-042: a node was selected (not opened) — a swarm `graph.select-node` dispatch or
    /// the host's selection sync. The host publishes the selection to the shared bus (E5).
    SelectNode { block_id: String },
    /// MT-042: create a real semantic Loom edge (`POST /loom/edges`) between two BLOCKS — a swarm
    /// `graph.add-edge` dispatch. The host runs it through the E6 loom client (NEEDS_MANAGED_RESOURCE_PROOF
    /// for the DB round-trip).
    AddEdge { source_block_id: String, target_block_id: String },
    /// MT-042: remove a Loom edge by id — a swarm `graph.remove-edge` dispatch. Host runs it via the E6
    /// loom client.
    RemoveEdge { edge_id: String },
}

/// The widget's full state. Held by the host (the pane), mutated in place by [`LoomGraphView::show`].
/// Layout positions, pan, zoom, selection, and loading/error are ephemeral UI state.
#[derive(Debug, Clone)]
pub struct LoomGraphView {
    pub workspace_id: String,
    pub mode: GraphMode,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    /// Total nodes the backend reported (>= `nodes.len()` when truncated to [`NODE_CAP`]). Drives the
    /// "showing N of M" notice (MC-2).
    pub total_available: usize,
    pub pan: Vec2,
    pub zoom: f32,
    pub selected: Option<String>,
    pub loading: bool,
    pub error: Option<String>,
    /// Force-iterations consumed so far (across frames). Capped at [`MAX_LAYOUT_ITERS`].
    pub iters_done: usize,
    /// Largest single-node displacement (px) in the most recent iteration; `< CONVERGENCE_EPS` => stable.
    pub last_max_step: f32,
    /// True once the layout positions have been seeded (a circle) for the current node set.
    seeded: bool,
    /// WP-KERNEL-012 MT-060 (E3): the Obsidian-class control panel state (search / groups / link-depth /
    /// orphans / size-by-degree / collapsed). Rendered each frame by [`Self::show`]; its pure results are
    /// applied in the painter pass via the overlays below.
    pub controls: GraphControls,
    /// MT-060: the cached visibility overlay (`block_id -> NodeVisibility`), recomputed on a
    /// [`GraphControlsEvent::FiltersChanged`] (and on load). A SEPARATE map — it NEVER mutates `nodes` /
    /// `edges`, so click/open + pan/zoom keep using the canonical vecs (RISK-6 / MC-6). Empty => every node
    /// fully visible.
    visibility: HashMap<String, NodeVisibility>,
    /// MT-060: the cached per-node group colour overlay (`block_id -> Color32`), recomputed alongside
    /// `visibility`. A node absent from this map falls back to its `content_type` colour.
    group_colors: HashMap<String, Color32>,
    /// MT-060: true once [`Self::controls`].`groups` have been discovered for the current node set, so the
    /// idempotent discovery runs once per load (re-running is still safe — discovery is idempotent).
    groups_discovered: bool,
    /// WP-KERNEL-012 MT-042 (E7): the shared knowledge AccessKit action registry. `None` until the host
    /// installs it via [`LoomGraphView::install_knowledge_action_registry`]. Skipped from `Clone`/`Debug`
    /// equality by being an `Arc` handle (cheap clone of the shared registry, never deep-copied).
    knowledge_registry: Option<Arc<Mutex<KnowledgeActionRegistry>>>,
    /// MT-042: the last canvas rect [`Self::show`] allocated, recorded so the in-`show` knowledge sync
    /// drives the SAME viewport-visible set the frame rendered (CTRL-042-06). `None` before the first
    /// render (the whole capped set is visible then). Transient per-frame state (not `Clone` semantics).
    last_canvas_rect: Option<Rect>,
    /// MT-042: swarm AccessKit dispatches the in-render sync/emit/take loop consumed THIS frame but that
    /// the single-`Option` `show` return cannot carry. The host drains them via
    /// [`Self::drain_knowledge_events`] after `show`. This is the wiring that makes the swarm surface LIVE
    /// from the render path (the must-fix anti-scaffolding fix): `show` itself drives the registry, so any
    /// host that renders the view gets a populated tree + consumed dispatch with no extra calls.
    pending_knowledge_events: Vec<GraphEvent>,
}

impl Default for LoomGraphView {
    fn default() -> Self {
        Self {
            workspace_id: String::new(),
            mode: GraphMode::Global,
            nodes: Vec::new(),
            edges: Vec::new(),
            total_available: 0,
            pan: Vec2::ZERO,
            zoom: 1.0,
            selected: None,
            loading: false,
            error: None,
            iters_done: 0,
            last_max_step: f32::INFINITY,
            seeded: false,
            controls: GraphControls::default(),
            visibility: HashMap::new(),
            group_colors: HashMap::new(),
            groups_discovered: false,
            knowledge_registry: None,
            last_canvas_rect: None,
            pending_knowledge_events: Vec::new(),
        }
    }
}

impl LoomGraphView {
    /// A fresh Global-mode view for `workspace_id`.
    pub fn global(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            mode: GraphMode::Global,
            ..Self::default()
        }
    }

    /// Replace the node/edge set (e.g. after a backend fetch resolves), clamping to [`NODE_CAP`]
    /// (MC-2) and recording the true total for the truncation notice. Resets layout so the new set
    /// re-seeds + re-converges.
    pub fn set_graph(&mut self, mut nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) {
        self.total_available = nodes.len();
        if nodes.len() > NODE_CAP {
            nodes.truncate(NODE_CAP);
        }
        // Drop edges that reference a clamped-away node so rendering never dereferences a missing node.
        let present: std::collections::HashSet<&str> = nodes.iter().map(|n| n.block_id.as_str()).collect();
        let edges = edges
            .into_iter()
            .filter(|e| present.contains(e.source.as_str()) && present.contains(e.target.as_str()))
            .collect();
        self.nodes = nodes;
        self.edges = edges;
        self.reset_layout();
        self.loading = false;
        self.error = None;
        // MT-060: discover groups from the freshly-loaded nodes (idempotent — user state survives a
        // depth-change reload, RISK-7 / MC-7) and recompute the visibility/colour overlay over the new
        // vecs. Discovery is keyed on the stable group key so it never duplicates or resets enabled/colour.
        self.controls.discover_groups(&self.nodes);
        self.groups_discovered = true;
        self.recompute_overlays();
    }

    /// MT-060: recompute the cached visibility + group-colour overlays from the CURRENT controls + the
    /// loaded vecs. Called on load and on each [`GraphControlsEvent::FiltersChanged`]. A SEPARATE overlay
    /// keyed by `block_id` — it NEVER mutates `nodes`/`edges` (RISK-6 / MC-6). O(nodes + edges) (MC-4).
    pub fn recompute_overlays(&mut self) {
        self.visibility = compute_visibility(
            &self.nodes,
            &self.edges,
            &self.controls.search,
            self.controls.show_orphans,
        );
        let enabled = self.controls.enabled_groups();
        self.group_colors.clear();
        if !enabled.is_empty() {
            for node in &self.nodes {
                if let Some(color) = assign_group_color(node, &enabled) {
                    self.group_colors.insert(node.block_id.clone(), color);
                }
            }
        }
    }

    /// The canvas [`Rect`] the last [`Self::show`] allocated (the area the nodes/edges paint into, AFTER
    /// the MT-060 control panel consumed its left strip). `None` before the first render. A host/test uses
    /// this to map a node's world position to its screen position with the SAME transform the widget uses,
    /// rather than guessing the canvas centre (which moved when the control panel took left space).
    pub fn canvas_rect(&self) -> Option<Rect> {
        self.last_canvas_rect
    }

    /// MT-060: the group colour the painter will use for `block_id`, or `None` when no enabled group
    /// matches (the painter then falls back to the `content_type` colour). Reads the SAME cached overlay
    /// the painter reads, so a test asserting this matches exactly what renders.
    pub fn group_color_for(&self, block_id: &str) -> Option<Color32> {
        self.group_colors.get(block_id).copied()
    }

    /// MT-060: the visibility overlay value the painter will use for `block_id` (test/host visibility into
    /// the same map the painter reads). `None` => the node is fully visible (not in the overlay).
    pub fn node_visibility(&self, block_id: &str) -> Option<NodeVisibility> {
        self.visibility.get(block_id).copied()
    }

    /// MT-060: is this node hidden by the current visibility overlay (the orphan filter)? A hidden node is
    /// not drawn and is NOT selectable (RISK-6 / MC-6 — click detection skips it).
    fn is_hidden(&self, block_id: &str) -> bool {
        self.visibility.get(block_id).map(|v| v.hidden).unwrap_or(false)
    }

    /// MT-060: is this node dimmed by the current visibility overlay (a search non-match)? A dimmed node
    /// renders at reduced alpha but stays on the canvas (spatial context — Obsidian behaviour).
    fn is_dimmed(&self, block_id: &str) -> bool {
        self.visibility.get(block_id).map(|v| v.dimmed).unwrap_or(false)
    }

    /// Reset the force layout so it re-seeds positions and re-converges from scratch (Re-layout button,
    /// or after a new graph is loaded).
    pub fn reset_layout(&mut self) {
        self.seeded = false;
        self.iters_done = 0;
        self.last_max_step = f32::INFINITY;
    }

    /// True when the layout has reached a stop condition (converged OR budget exhausted) and so must
    /// NOT request another repaint (the idle-repaint discipline).
    pub fn layout_stable(&self) -> bool {
        self.iters_done >= MAX_LAYOUT_ITERS || self.last_max_step < CONVERGENCE_EPS
    }

    /// Seed initial positions on a circle around the origin (deterministic; not random, so tests are
    /// reproducible). A single isolated node sits at the origin.
    fn seed_positions(&mut self) {
        let n = self.nodes.len();
        if n == 0 {
            self.seeded = true;
            return;
        }
        let radius = 60.0 + (n as f32) * 6.0;
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let theta = (i as f32) / (n as f32) * std::f32::consts::TAU;
            node.x = radius * theta.cos();
            node.y = radius * theta.sin();
        }
        self.seeded = true;
    }

    /// Run up to [`ITERS_PER_FRAME`] spring/force iterations (RISK-1 / MC-1), stopping early if the
    /// budget is exhausted or the layout converged. Returns the largest single-node displacement of
    /// the LAST iteration run this frame (used for the convergence test + repaint decision).
    ///
    /// Forces (the MT step-4 model):
    ///   - repulsion: every node pair pushes apart with Coulomb k=1000/d^2 (capped at small d).
    ///   - attraction: connected pairs pull toward a 150px rest length with spring k=0.05.
    pub fn step_layout(&mut self) -> f32 {
        if !self.seeded {
            self.seed_positions();
        }
        if self.nodes.is_empty() {
            self.last_max_step = 0.0;
            self.iters_done = MAX_LAYOUT_ITERS; // nothing to lay out; treat as immediately stable.
            return 0.0;
        }

        // Build an index for edge lookups.
        let index: HashMap<&str, usize> =
            self.nodes.iter().enumerate().map(|(i, n)| (n.block_id.as_str(), i)).collect();
        let edge_pairs: Vec<(usize, usize)> = self
            .edges
            .iter()
            .filter_map(|e| Some((*index.get(e.source.as_str())?, *index.get(e.target.as_str())?)))
            .filter(|(a, b)| a != b)
            .collect();

        let mut max_step = 0.0f32;
        let budget = ITERS_PER_FRAME.min(MAX_LAYOUT_ITERS.saturating_sub(self.iters_done));
        for _ in 0..budget {
            let n = self.nodes.len();
            let mut disp = vec![Vec2::ZERO; n];

            // Repulsion (Coulomb): O(n^2).
            for i in 0..n {
                for j in (i + 1)..n {
                    let mut delta = self.nodes[i].pos() - self.nodes[j].pos();
                    let mut dist = delta.length();
                    if dist < 0.01 {
                        // Coincident: nudge deterministically so the pair separates.
                        delta = Vec2::new(0.01 * (i as f32 + 1.0), 0.01 * (j as f32 + 1.0));
                        dist = delta.length();
                    }
                    let force = 1000.0 / (dist * dist);
                    let dir = delta / dist;
                    disp[i] += dir * force;
                    disp[j] -= dir * force;
                }
            }

            // Attraction (spring toward 150px rest length) for connected pairs.
            for &(a, b) in &edge_pairs {
                let delta = self.nodes[a].pos() - self.nodes[b].pos();
                let dist = delta.length().max(0.01);
                let dir = delta / dist;
                let force = 0.05 * (dist - 150.0);
                disp[a] -= dir * force;
                disp[b] += dir * force;
            }

            // Apply, clamping a single step so the explosion of 1/d^2 at tiny d cannot fling a node to
            // infinity (numerical-stability guard; keeps positions finite for the screenshot/AC tests).
            max_step = 0.0;
            for (i, node) in self.nodes.iter_mut().enumerate() {
                let mut step = disp[i];
                let len = step.length();
                let max_len = 50.0;
                if len > max_len {
                    step = step / len * max_len;
                }
                node.x += step.x;
                node.y += step.y;
                max_step = max_step.max(step.length());
            }

            self.iters_done += 1;
            if max_step < CONVERGENCE_EPS {
                break;
            }
        }
        self.last_max_step = max_step;
        max_step
    }

    /// World-space -> screen-space transform: `screen = center + pan + world * zoom`.
    fn to_screen(&self, world: Pos2, center: Vec2) -> Pos2 {
        Pos2::new(
            center.x + self.pan.x + world.x * self.zoom,
            center.y + self.pan.y + world.y * self.zoom,
        )
    }

    /// Screen-space -> world-space inverse (RISK-4 click detection): `world = (screen - center - pan) / zoom`.
    fn to_world(&self, screen: Pos2, center: Vec2) -> Pos2 {
        Pos2::new(
            (screen.x - center.x - self.pan.x) / self.zoom,
            (screen.y - center.y - self.pan.y) / self.zoom,
        )
    }

    /// Find the node whose circle contains `screen_pos` (topmost / last drawn wins). Used by click
    /// detection and pan-vs-node hit testing. MT-060: a HIDDEN node (orphan filter) is NOT drawn and so is
    /// NOT hit-testable — click detection skips it so a hidden node can never be selected (RISK-6 / MC-6).
    fn node_at_screen(&self, screen_pos: Pos2, center: Vec2) -> Option<usize> {
        let world = self.to_world(screen_pos, center);
        // Radius in WORLD space is constant; compare world distances so zoom does not skew hit area.
        self.nodes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, n)| !self.is_hidden(&n.block_id) && (n.pos() - world).length() <= NODE_RADIUS)
            .map(|(i, _)| i)
    }

    /// Apply a scroll-wheel zoom around `pointer` (RISK-4 zoom-to-pointer): keep the world point under
    /// the cursor fixed while scaling. `scroll_y` is the wheel delta (positive = zoom in).
    pub fn apply_zoom(&mut self, scroll_y: f32, pointer: Pos2, center: Vec2) {
        if scroll_y == 0.0 {
            return;
        }
        let world_before = self.to_world(pointer, center);
        let factor = 1.15f32.powf(scroll_y);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        // Re-derive pan so `world_before` maps back to the same screen `pointer` after the scale.
        let screen_after = Pos2::new(
            center.x + self.pan.x + world_before.x * self.zoom,
            center.y + self.pan.y + world_before.y * self.zoom,
        );
        self.pan.x += pointer.x - screen_after.x;
        self.pan.y += pointer.y - screen_after.y;
    }

    /// Render the graph and return the typed event (if any) this frame produced. The host applies the
    /// event (re-fetch on mode change, open block on node click). Drives one layout step + requests a
    /// repaint ONLY while not yet stable (idle-repaint discipline).
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<GraphEvent> {
        let mut event = None;

        // ── Toolbar strip ────────────────────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            let is_local = self.mode.is_local();
            // Mode toggle (two SelectableLabel widgets with stable author_ids).
            let local = ui.selectable_label(is_local, "Local");
            emit_toolbar_node(ui, local.id, MODE_LOCAL_AUTHOR_ID, "Local graph mode");
            if local.clicked() && !is_local {
                // Cannot enter Local without a focused block; the host supplies one. If none is set,
                // stay Global (no-op) — the host re-fetches on ModeChanged{to_global:false}.
                event = Some(GraphEvent::ModeChanged { to_global: false });
            }
            let global = ui.selectable_label(!is_local, "Global");
            emit_toolbar_node(ui, global.id, MODE_GLOBAL_AUTHOR_ID, "Global graph mode");
            if global.clicked() && is_local {
                self.mode = GraphMode::Global;
                event = Some(GraphEvent::ModeChanged { to_global: true });
            }

            ui.separator();
            let zin = ui.button("+");
            emit_toolbar_node(ui, zin.id, ZOOM_IN_AUTHOR_ID, "Zoom in");
            if zin.clicked() {
                self.zoom = (self.zoom * 1.15).clamp(MIN_ZOOM, MAX_ZOOM);
            }
            let zout = ui.button("-");
            emit_toolbar_node(ui, zout.id, ZOOM_OUT_AUTHOR_ID, "Zoom out");
            if zout.clicked() {
                self.zoom = (self.zoom / 1.15).clamp(MIN_ZOOM, MAX_ZOOM);
            }
            let relayout = ui.button("Re-layout");
            emit_toolbar_node(ui, relayout.id, RELAYOUT_AUTHOR_ID, "Re-run graph layout");
            if relayout.clicked() {
                self.reset_layout();
                event = Some(GraphEvent::Relayout);
            }

            ui.separator();
            // Node count label (AC1: matches the loaded block count; MC-2 truncation notice).
            let count_label = if self.total_available > self.nodes.len() {
                format!("showing {} of {} nodes", self.nodes.len(), self.total_available)
            } else {
                format!("{} nodes", self.nodes.len())
            };
            ui.label(count_label);
        });

        // ── MT-060 control panel (left strip alongside the canvas) ─────────────────────────────────
        // A late group-discovery safety net: if the host populated nodes WITHOUT calling set_graph (rare),
        // discover once so groups still appear. Idempotent, so calling again after set_graph is harmless.
        if !self.groups_discovered && !self.nodes.is_empty() {
            self.controls.discover_groups(&self.nodes);
            self.groups_discovered = true;
            self.recompute_overlays();
        }
        let is_local_mode = self.mode.is_local();
        // Render the control panel as a left SidePanel scoped to THIS ui, so it sits beside the canvas and
        // the canvas takes the remaining width. When collapsed (panel_open=false) the panel renders only
        // its expand toggle, so it does not steal canvas space.
        let controls_event = egui::SidePanel::left(ui.id().with("graph-controls"))
            .resizable(false)
            .min_width(if self.controls.panel_open { 160.0 } else { 0.0 })
            .frame(egui::Frame::default().fill(palette.surface).inner_margin(6.0))
            .show_inside(ui, |ui| self.controls.show(ui, is_local_mode))
            .inner;
        // Apply the control event: a DepthChanged is a backend re-query (Local only); a FiltersChanged is a
        // pure client-side overlay recompute (NO network — AC7 / AC8).
        match controls_event {
            GraphControlsEvent::DepthChanged(depth) if is_local_mode => {
                // Re-query SIGNAL only: the host re-fires the existing graph-search endpoint with the new
                // depth and calls set_graph with the result. The host sets `loading=true` when it ACTUALLY
                // dispatches the runtime-backed request (the MT-021 idle-repaint discipline: the spinner
                // animates ONLY during a genuine in-flight fetch, never merely because a control changed) —
                // the view does NOT set loading here, so a headless/no-host render never spins forever.
                event = Some(GraphEvent::DepthChanged { depth });
            }
            GraphControlsEvent::DepthChanged(_) => { /* Global: slider is disabled; unreachable no-op. */ }
            GraphControlsEvent::FiltersChanged => {
                self.recompute_overlays();
            }
            GraphControlsEvent::None => {}
        }

        // ── Canvas ───────────────────────────────────────────────────────────────────────────────
        let (rect, canvas_resp) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        // Record the canvas rect so the in-`show` knowledge sync derives the SAME viewport-visible node
        // set this frame rendered (CTRL-042-06 / MT-042 in-render wiring).
        self.last_canvas_rect = Some(rect);
        let painter = ui.painter_at(rect);
        let center = rect.center().to_vec2();

        // Background fill + dotted grid (so the canvas is never blank/white — AC7 + PROOF4).
        painter.rect_filled(rect, 0.0, palette.bg);
        draw_grid(&painter, rect, palette);

        // Drive one layout step; request repaint ONLY while still animating (idle-repaint discipline).
        let max_step = self.step_layout();
        if !self.layout_stable() {
            ui.ctx().request_repaint();
        }
        let _ = max_step;

        // Pointer input: zoom (scroll), pan (drag on empty area), click node (open).
        if let Some(pointer) = canvas_resp.hover_pos() {
            let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_y != 0.0 {
                self.apply_zoom(scroll_y.signum(), pointer, center);
            }
        }
        // Drag: if it started over empty canvas (no node under the press), pan; otherwise ignore (a
        // node drag is not in scope this MT). We pan on any canvas drag that is not over a node.
        if canvas_resp.dragged() {
            let over_node = canvas_resp
                .interact_pointer_pos()
                .and_then(|p| self.node_at_screen(p, center))
                .is_some();
            if !over_node {
                self.pan += canvas_resp.drag_delta();
            }
        }
        // Click: open the node under the pointer (AC5).
        if canvas_resp.clicked() {
            if let Some(pos) = canvas_resp.interact_pointer_pos() {
                if let Some(idx) = self.node_at_screen(pos, center) {
                    let block_id = self.nodes[idx].block_id.clone();
                    self.selected = Some(block_id.clone());
                    event = Some(GraphEvent::OpenNode { block_id });
                }
            }
        }

        // Edges first (so nodes render on top — MT implementation_notes). MT-060: an edge with a HIDDEN
        // endpoint is skipped entirely (the orphan filter removed that node); an edge with a DIMMED
        // endpoint draws at reduced alpha (the node it connects to is a search non-match).
        let edge_stroke_full = Stroke::new(1.5, palette.text_subtle.gamma_multiply(0.6));
        let edge_stroke_dim =
            Stroke::new(1.5, palette.text_subtle.gamma_multiply(0.6).gamma_multiply(0.35));
        let pos_by_id: HashMap<&str, Pos2> =
            self.nodes.iter().map(|n| (n.block_id.as_str(), n.pos())).collect();
        for e in &self.edges {
            // Skip any edge touching a hidden node (RISK-6 / MC-6: its node is off the canvas).
            if self.is_hidden(&e.source) || self.is_hidden(&e.target) {
                continue;
            }
            if let (Some(&s), Some(&t)) = (pos_by_id.get(e.source.as_str()), pos_by_id.get(e.target.as_str())) {
                let dimmed = self.is_dimmed(&e.source) || self.is_dimmed(&e.target);
                let stroke = if dimmed { edge_stroke_dim } else { edge_stroke_full };
                painter.line_segment(
                    [self.to_screen(s, center), self.to_screen(t, center)],
                    stroke,
                );
            }
        }

        // Nodes + labels + AccessKit. Each node is an addressable Role::Button (Action::Click) the
        // swarm can drive by `graph.node.{id}` (AC6 / HBR-SWARM). MT-060 applies the overlays HERE: a
        // hidden node is skipped (and not addressable); a dimmed node renders at reduced alpha; a node in
        // an enabled group uses the group colour (else the content_type colour); size-by-degree scales the
        // radius by the node's edge degree.
        for node in &self.nodes {
            // Skip hidden nodes entirely — not drawn, not labelled, not AccessKit-addressable, not
            // selectable (the hit test already skips them). RISK-6 / MC-6.
            if self.is_hidden(&node.block_id) {
                continue;
            }
            let screen = self.to_screen(node.pos(), center);
            let dimmed = self.is_dimmed(&node.block_id);
            // Group colour wins over content_type; fall back to content_type when no enabled group matches.
            let base_color = self
                .group_colors
                .get(&node.block_id)
                .copied()
                .unwrap_or_else(|| content_type_color(&node.content_type, palette));
            let color = if dimmed { dim_color(base_color) } else { base_color };
            // Size-by-degree: radius scales with the node's edge degree (clamped to 3x base). World-space
            // base radius is NODE_RADIUS; the screen radius multiplies by zoom (as before).
            let degree = node_degree(&node.block_id, &self.edges);
            let world_r = node_radius(NODE_RADIUS, degree, self.controls.size_by_degree);
            let r = world_r * self.zoom;
            painter.circle_filled(screen, r, color);
            if self.selected.as_deref() == Some(node.block_id.as_str()) {
                painter.circle_stroke(screen, r + 2.0, Stroke::new(2.0, palette.accent));
            }
            // Title label beneath the node (dimmed too when the node is a search non-match).
            let label_color = if dimmed { dim_color(palette.text) } else { palette.text };
            painter.text(
                Pos2::new(screen.x, screen.y + r + 2.0),
                egui::Align2::CENTER_TOP,
                &node.title,
                egui::FontId::proportional(11.0),
                label_color,
            );
            emit_node_accesskit(ui, node);
        }

        // Loading / error overlay. Loading animates ONLY during a genuine in-flight fetch (the host
        // sets `loading=true` only when a runtime-backed request is dispatched). Error is a static label.
        if let Some(err) = &self.error {
            draw_overlay_label(&painter, rect, &format!("Graph error: {err}"), palette.error_text, palette);
        } else if self.loading {
            draw_overlay_label(&painter, rect, "Loading graph…", palette.text_subtle, palette);
            // A real in-flight fetch is the ONE case we keep animating, so the spinner text can update;
            // bounded because the host clears `loading` when the fetch resolves/fails.
            ui.ctx().request_repaint();
        } else if self.nodes.is_empty() {
            // AC7: empty canvas shows a "0 nodes" hint and never panics. No repaint requested (idle).
            draw_overlay_label(&painter, rect, "0 nodes", palette.text_subtle, palette);
        }

        // ── WP-KERNEL-012 MT-042 (E7): drive the knowledge AccessKit surface FROM the render path. ───
        // This is the must-fix anti-scaffolding wiring (the MT-041 pattern: `CodeEditorPanel::show` calls
        // `sync_editor_actions`). A swarm agent must DISCOVER + INVOKE the graph actions purely via the
        // AccessKit channel; that only works if EVERY frame the host renders re-derives the registry,
        // emits its nodes into the live tree, and consumes this frame's dispatch. Gated on an installed
        // registry, so a bare `view.show(ui, &palette)` with no registry stays a pure no-op.
        if self.knowledge_registry.is_some() {
            self.sync_knowledge_registry(self.last_canvas_rect);
            self.emit_knowledge_accesskit(ui);
            let dispatched = self.take_knowledge_dispatched(ui);
            self.pending_knowledge_events.extend(dispatched);
        }

        event
    }

    /// MT-042: drain the swarm AccessKit dispatches the in-render sync/emit/take loop consumed since the
    /// last drain. The host calls this AFTER [`Self::show`] to route each dispatched [`GraphEvent`] to the
    /// E6 loom client (the same way it applies `show`'s `Option` return). Empty when no swarm dispatch
    /// arrived (or no registry is installed).
    pub fn drain_knowledge_events(&mut self) -> Vec<GraphEvent> {
        std::mem::take(&mut self.pending_knowledge_events)
    }

    // ── WP-KERNEL-012 MT-042 (E7): knowledge AccessKit action surface ─────────────────────────────

    /// Install the shared knowledge AccessKit action registry (the MT-041 `install_*` pattern). After
    /// this, [`Self::sync_knowledge_registry`] populates the registry each frame and
    /// [`Self::take_knowledge_dispatched`] consumes swarm `Click` dispatches.
    pub fn install_knowledge_action_registry(&mut self, registry: Arc<Mutex<KnowledgeActionRegistry>>) {
        self.knowledge_registry = Some(registry);
    }

    /// The viewport-visible node set plus a [`VIEWPORT_LOOKAHEAD`] lookahead (CTRL-042-06 / RISK-042-06):
    /// returns the indices of `self.nodes` whose screen position falls within `rect`, plus up to
    /// `VIEWPORT_LOOKAHEAD` additional nodes nearest the viewport, so a swarm agent has a small
    /// off-screen margin without registering the whole (capped) graph. When `rect` is `None` (no render
    /// yet) the whole capped set is visible (it is already bounded to `NODE_CAP`).
    fn visible_node_indices(&self, rect: Option<Rect>, center: Vec2) -> Vec<usize> {
        let Some(rect) = rect else {
            return (0..self.nodes.len()).collect();
        };
        let mut visible = Vec::new();
        let mut offscreen: Vec<(f32, usize)> = Vec::new();
        let view_center = rect.center();
        for (i, node) in self.nodes.iter().enumerate() {
            let screen = self.to_screen(node.pos(), center);
            if rect.contains(screen) {
                visible.push(i);
            } else {
                let d = (screen - view_center).length();
                offscreen.push((d, i));
            }
        }
        // Lookahead buffer: the nearest off-screen nodes (CTRL-042-06).
        offscreen.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, i) in offscreen.into_iter().take(VIEWPORT_LOOKAHEAD) {
            visible.push(i);
        }
        visible
    }

    /// Populate the knowledge registry with the graph's GLOBAL controls (registered every frame as fixed
    /// Button nodes regardless of content — AC-042-08) and the per-node `graph.node.<block_id>` TreeItem
    /// identities for the viewport-visible set (CTRL-042-06). Re-derives the node set fully each frame so
    /// a deleted block's node DISAPPEARS from the tree (deletion-by-absence — IN-042-10). HBR-QUIET: the
    /// host calls [`KnowledgeActionRegistry::state_changed_since_last_push`] to decide whether to notify.
    /// `last_rect` is the canvas rect recorded by a prior `show`; pass `None` before the first render.
    pub fn sync_knowledge_registry(&self, last_rect: Option<Rect>) {
        let Some(registry) = &self.knowledge_registry else { return };
        let mut reg = registry.lock().unwrap_or_else(|e| e.into_inner());
        // Fully re-derive: clear, then re-register controls + visible identities (deletion-by-absence).
        reg.clear_nodes();
        // Global controls — ALWAYS present, content-independent (AC-042-08). add/remove-edge are
        // dispatch-only seams the host routes to the E6 loom client; they are enabled (discoverable +
        // dispatchable) but carry a JSON payload.
        for entry in GRAPH_CONTROL_CATALOG {
            reg.upsert_control(entry.author_id, entry.label, KnowledgeNodeState::present());
        }
        // Per-node identities for the viewport-visible set + lookahead.
        let center = last_rect.map(|r| r.center().to_vec2()).unwrap_or(Vec2::ZERO);
        for i in self.visible_node_indices(last_rect, center) {
            let node = &self.nodes[i];
            let author = knowledge_action_registry::graph_node_author_id(&node.block_id);
            // value carries the raw block_id so a swarm agent correlates the sanitized author_id to the
            // real Loom id (IN-042-02 pattern). content_type is included for filtering context.
            let value = Some(format!("block_id={};content_type={}", node.block_id, node.content_type));
            reg.upsert_identity(author, KAxRole::TreeItem, node.title.clone(), value, KnowledgeNodeState::present());
        }
    }

    /// Emit the knowledge registry's nodes into the live AccessKit tree (call inside the host's `show`,
    /// after [`Self::sync_knowledge_registry`]). No-op if no registry is installed.
    pub fn emit_knowledge_accesskit(&self, ui: &egui::Ui) {
        if let Some(registry) = &self.knowledge_registry {
            registry.lock().unwrap_or_else(|e| e.into_inner()).emit_into_tree(ui);
        }
    }

    /// Consume this frame's swarm AccessKit `Click` dispatches targeting the graph's knowledge nodes and
    /// MAP each to a typed [`GraphEvent`] (RISK-042-04 — the dispatch REACHES the pane). Returns the
    /// events in dispatch order. Parameterized actions parse their JSON payload via the no-unwrap
    /// [`knowledge_action_registry::parse_payload`] seam (RISK-042-03 / CTRL-042-03 — a malformed payload
    /// is logged + dropped, never a panic). A `graph.node.<id>` click maps to `OpenNode` (the swarm
    /// open-by-identity path).
    pub fn take_knowledge_dispatched(&mut self, ui: &egui::Ui) -> Vec<GraphEvent> {
        let Some(registry) = &self.knowledge_registry else { return Vec::new() };
        let dispatched = registry.lock().unwrap_or_else(|e| e.into_inner()).take_dispatched(ui);
        let mut events = Vec::new();
        for (author_id, payload) in dispatched {
            match author_id.as_str() {
                "graph.pan-left" => self.pan.x -= 40.0,
                "graph.pan-right" => self.pan.x += 40.0,
                "graph.zoom-in" => self.zoom = (self.zoom * 1.15).clamp(MIN_ZOOM, MAX_ZOOM),
                "graph.zoom-out" => self.zoom = (self.zoom / 1.15).clamp(MIN_ZOOM, MAX_ZOOM),
                "graph.zoom-reset" => self.zoom = 1.0,
                "graph.deselect-all" => self.selected = None,
                "graph.open-node" => {
                    if let Some(p) = knowledge_action_registry::parse_payload::<BlockIdPayload>(payload.as_deref()) {
                        self.selected = Some(p.block_id.clone());
                        events.push(GraphEvent::OpenNode { block_id: p.block_id });
                    }
                }
                "graph.select-node" => {
                    if let Some(p) = knowledge_action_registry::parse_payload::<BlockIdPayload>(payload.as_deref()) {
                        self.selected = Some(p.block_id.clone());
                        events.push(GraphEvent::SelectNode { block_id: p.block_id });
                    }
                }
                "graph.add-edge" => {
                    if let Some(p) = knowledge_action_registry::parse_payload::<AddEdgePayload>(payload.as_deref()) {
                        events.push(GraphEvent::AddEdge {
                            source_block_id: p.source_id,
                            target_block_id: p.target_id,
                        });
                    }
                }
                "graph.remove-edge" => {
                    if let Some(p) = knowledge_action_registry::parse_payload::<EdgeIdPayload>(payload.as_deref()) {
                        events.push(GraphEvent::RemoveEdge { edge_id: p.edge_id });
                    }
                }
                other => {
                    // A per-identity node click: `graph.node.<sanitized_block_id>` -> open that node. We
                    // resolve the sanitized author_id back to the real block_id by scanning the live node
                    // set (the author_id is a sanitized projection, so a reverse map is needed).
                    if let Some(stripped) = other.strip_prefix(knowledge_action_registry::GRAPH_NODE_AUTHOR_ID_PREFIX) {
                        if let Some(node) = self
                            .nodes
                            .iter()
                            .find(|n| crate::project_tree::stable_part(&n.block_id) == stripped)
                        {
                            let block_id = node.block_id.clone();
                            self.selected = Some(block_id.clone());
                            events.push(GraphEvent::OpenNode { block_id });
                        }
                    }
                }
            }
        }
        events
    }
}

/// MT-060: dim a colour to [`DIM_ALPHA`] for a search non-match (reduced alpha, kept on the canvas for
/// spatial context). Reuses the colour's RGB and lowers its alpha via `from_rgba_unmultiplied` (the
/// sanctioned DYNAMIC form the no-hardcoded-colour guard does NOT flag — it is data, not a palette
/// literal). Obsidian dims non-matches rather than removing them.
fn dim_color(color: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), DIM_ALPHA)
}

/// Draw a dotted grid background across `rect` (so the canvas is visibly non-blank for PROOF4).
fn draw_grid(painter: &egui::Painter, rect: Rect, palette: &HsPalette) {
    let step = 40.0;
    let dot = palette.border.gamma_multiply(0.5);
    let mut y = rect.top();
    while y <= rect.bottom() {
        let mut x = rect.left();
        while x <= rect.right() {
            painter.circle_filled(Pos2::new(x, y), 1.0, dot);
            x += step;
        }
        y += step;
    }
}

/// Draw a centered overlay label (loading / error / empty) over the canvas.
fn draw_overlay_label(painter: &egui::Painter, rect: Rect, text: &str, color: Color32, palette: &HsPalette) {
    let galley = painter.layout_no_wrap(text.to_owned(), egui::FontId::proportional(15.0), color);
    let pos = Pos2::new(
        rect.center().x - galley.size().x * 0.5,
        rect.center().y - galley.size().y * 0.5,
    );
    // A faint backing panel so the label reads over the grid.
    let pad = Vec2::new(8.0, 4.0);
    let bg_rect = Rect::from_min_size(pos - pad, galley.size() + pad * 2.0);
    painter.rect_filled(bg_rect, 4.0, palette.surface);
    painter.galley(pos, galley, color);
}

/// Emit a toolbar control's live AccessKit node (Role::Button + Action::Click + author_id) so a swarm
/// agent can address it by stable id (AC6 / HBR-SWARM).
fn emit_toolbar_node(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
    });
}

/// Emit a graph node's live AccessKit node: Role::Button, label = title, Action::Click (DefaultAction),
/// author_id = `graph.node.{sanitized_block_id}` (AC6 / HBR-SWARM). The node has no egui widget of its
/// own (it is painter-drawn), so we allocate a stable `egui::Id` from its author_id string — the
/// dynamic-author_id pattern the shell uses for non-fixed-band addressable nodes.
fn emit_node_accesskit(ui: &egui::Ui, node: &GraphNode) {
    let author = node_author_id(&node.block_id);
    let id = egui::Id::new(&author);
    let label = node.title.clone();
    ui.ctx().accesskit_node_builder(id, move |n| {
        n.set_role(accesskit::Role::Button);
        n.set_author_id(author.clone());
        n.set_label(label.clone());
        n.add_action(accesskit::Action::Click);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring_graph(n: usize) -> LoomGraphView {
        let mut v = LoomGraphView::global("ws-1");
        let nodes: Vec<GraphNode> = (0..n)
            .map(|i| GraphNode::new(format!("block-{i:03}"), format!("Block {i}"), "note"))
            .collect();
        let edges: Vec<GraphEdge> = (0..n)
            .map(|i| GraphEdge::new(format!("block-{i:03}"), format!("block-{:03}", (i + 1) % n), "mention"))
            .collect();
        v.set_graph(nodes, edges);
        v
    }

    /// PROOF1: a 5-node graph converges to < 1px per-node step within the 300-iteration budget.
    #[test]
    fn force_layout_converges_under_budget() {
        let mut v = ring_graph(5);
        let mut last = f32::INFINITY;
        // Drive frames until stable or the budget is exhausted.
        while !v.layout_stable() {
            last = v.step_layout();
        }
        assert!(
            v.iters_done <= MAX_LAYOUT_ITERS,
            "layout must stop within the {MAX_LAYOUT_ITERS}-iteration budget (did {})",
            v.iters_done
        );
        assert!(
            v.last_max_step < CONVERGENCE_EPS,
            "PROOF1: 5-node graph must converge to < {CONVERGENCE_EPS}px (last step {})",
            v.last_max_step
        );
        // Positions must be finite (the step clamp guards 1/d^2 blow-up).
        for node in &v.nodes {
            assert!(node.x.is_finite() && node.y.is_finite(), "node position must stay finite");
        }
        let _ = last;
    }

    /// Stable layout must NOT request more iterations (idle-repaint discipline at the data level): once
    /// converged, `step_layout` is a no-op-ish call that keeps `iters_done`/`last_max_step` stable.
    #[test]
    fn stable_layout_is_idempotent() {
        let mut v = ring_graph(5);
        while !v.layout_stable() {
            v.step_layout();
        }
        let iters = v.iters_done;
        // Calling step again past stability does not blow the budget or destabilize.
        v.step_layout();
        assert!(v.layout_stable(), "must remain stable");
        assert!(v.iters_done >= iters, "iters only ever grow, capped at the budget");
        assert!(v.iters_done <= MAX_LAYOUT_ITERS + ITERS_PER_FRAME);
    }

    /// MC-3 / RISK-3: block ids with slashes/colons sanitize to `[a-z0-9-]` author_id suffixes.
    #[test]
    fn node_author_id_is_sanitized() {
        let id = node_author_id("ws:1/block 7#x");
        assert!(id.starts_with(NODE_AUTHOR_ID_PREFIX));
        let suffix = &id[NODE_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
        assert!(!suffix.is_empty(), "non-empty suffix");
    }

    /// MC-2 / RISK-5: loading more than NODE_CAP nodes clamps to the cap and records the true total.
    #[test]
    fn node_cap_clamps_and_records_total() {
        let mut v = LoomGraphView::global("ws-1");
        let nodes: Vec<GraphNode> = (0..(NODE_CAP + 50))
            .map(|i| GraphNode::new(format!("b{i}"), format!("B{i}"), "note"))
            .collect();
        v.set_graph(nodes, vec![]);
        assert_eq!(v.nodes.len(), NODE_CAP, "clamped to the node cap");
        assert_eq!(v.total_available, NODE_CAP + 50, "true total recorded for the notice");
    }

    /// RISK-4: zoom is clamped to [0.1, 4.0] and zoom-to-pointer keeps the world point under the cursor
    /// fixed (no jump after pan).
    #[test]
    fn zoom_clamps_and_preserves_pointer_world_point() {
        let mut v = ring_graph(3);
        v.pan = Vec2::new(20.0, -15.0);
        let center = Vec2::new(300.0, 200.0);
        let pointer = Pos2::new(350.0, 250.0);
        let world_before = v.to_world(pointer, center);
        v.apply_zoom(1.0, pointer, center); // one zoom-in step
        let world_after_screen = v.to_screen(world_before, center);
        assert!(
            (world_after_screen.x - pointer.x).abs() < 0.5 && (world_after_screen.y - pointer.y).abs() < 0.5,
            "zoom-to-pointer must keep the world point under the cursor fixed (got {world_after_screen:?})"
        );
        // Clamp: scrolling up many times never exceeds MAX_ZOOM.
        for _ in 0..50 {
            v.apply_zoom(1.0, pointer, center);
        }
        assert!(v.zoom <= MAX_ZOOM + 1e-3, "zoom clamped to MAX_ZOOM (got {})", v.zoom);
        for _ in 0..100 {
            v.apply_zoom(-1.0, pointer, center);
        }
        assert!(v.zoom >= MIN_ZOOM - 1e-3, "zoom clamped to MIN_ZOOM (got {})", v.zoom);
    }

    /// AC7: an empty graph is stable, has 0 nodes, and never panics on layout.
    #[test]
    fn empty_graph_is_stable_zero_nodes() {
        let mut v = LoomGraphView::global("ws-1");
        v.set_graph(vec![], vec![]);
        assert_eq!(v.nodes.len(), 0);
        let step = v.step_layout();
        assert_eq!(step, 0.0, "empty layout has zero displacement");
        assert!(v.layout_stable(), "empty layout is immediately stable (no perpetual repaint)");
    }

    /// AC8: an error string is preserved on the view (the host sets it on a backend failure) and does
    /// not get cleared by a layout step.
    #[test]
    fn error_state_survives_layout() {
        let mut v = ring_graph(3);
        v.error = Some("backend unreachable".to_owned());
        v.step_layout();
        assert_eq!(v.error.as_deref(), Some("backend unreachable"));
    }

    /// content_type colours come from the live theme (no hardcoded hex) and differ across types so the
    /// graph is legible.
    #[test]
    fn content_type_colors_are_distinct_theme_tokens() {
        let pal = crate::theme::HsTheme::Dark.palette();
        let note = content_type_color("note", &pal);
        let file = content_type_color("file", &pal);
        let tag = content_type_color("tag_hub", &pal);
        let other = content_type_color("zzz_unknown", &pal);
        assert_eq!(note, pal.syntax.keyword);
        assert_eq!(file, pal.text_subtle);
        assert_eq!(tag, pal.success_text);
        assert_eq!(other, pal.border_strong);
        // At least three of the mapped colours are visually distinct.
        let set: std::collections::HashSet<[u8; 4]> =
            [note, file, tag, other].iter().map(|c| c.to_array()).collect();
        assert!(set.len() >= 3, "content-type colours must be distinguishable (got {})", set.len());
    }
}
