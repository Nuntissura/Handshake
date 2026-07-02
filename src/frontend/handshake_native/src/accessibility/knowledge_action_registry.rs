//! WP-KERNEL-012 MT-042 (E7 model-vision parity): the **KnowledgeActionRegistry** — the single,
//! consolidated AccessKit action surface for the native knowledge-surface views (the E3 graph
//! [`crate::graph::graph_view`], the canvas board [`crate::graph::canvas_board`], and the
//! block-collection views [`crate::graph::block_collection_view`]: table / Kanban / calendar), so an
//! out-of-process swarm agent can DISCOVER and INVOKE every interactive knowledge action purely through
//! the WP-011 AccessKit channel — no screen-scraping, no keyboard simulation.
//!
//! ## Why this is a CONSOLIDATION layer, not a second parallel node set (the anti-duplication rule)
//!
//! Each E3 microtask already emitted per-widget AccessKit `author_id`s: MT-021 graph node ids
//! (`graph.node.<sanitized_block_id>`, [`crate::graph::graph_view::node_author_id`]) + toolbar ids
//! (`graph.mode.*`/`graph.zoom.*`/`graph.relayout`); MT-026 canvas placement card ids
//! (`canvas.placement.<placement_id>`, [`crate::graph::canvas_board::placement_author_id`]) + toolbar
//! ids; MT-027 collection ids (`bcv.table.*`/`bcv.kanban.*`/`bcv.calendar.*`/`bcv.kind.*`). MT-042 does
//! NOT re-mint a second set of nodes for those widgets — that would create duplicate AccessKit nodes a
//! swarm agent cannot disambiguate (the MT-041 discipline, IN-042-08 + the "REUSE EXISTING author_ids"
//! gate). Instead this registry:
//!
//! 1. Defines the canonical, surface-namespaced swarm-action vocabulary
//!    `graph.*` / `canvas.*` / `collection.*` (the contract's IN-042-01/07 catalog) as the ONE
//!    swarm-facing action vocabulary, and adds the per-identity keyed addressable nodes the contract
//!    names: `graph.node.<block_id>` (Role::TreeItem), `canvas.card.<placement_id>` (Role::Group),
//!    `collection.row.<block_id>` (Role::Row), `collection.lane.<tag>` (Role::Group). The `graph.node.`
//!    prefix is the SAME prefix MT-021 already emits, so this registry ALIASES the existing graph node
//!    id rather than minting a parallel one; `canvas.card.` is the canonical MT-042 alias of MT-026's
//!    `canvas.placement.` (the contract names `canvas.card.<placement_id>` — both stay live, keyed by
//!    the same placement id, never two parallel nodes for one widget).
//! 2. Maps each canonical action id to a REAL dispatch target ([`KnowledgeDispatch`]) the host runs
//!    against the real widget state ([`crate::graph::graph_view::LoomGraphView`] / `LoomCanvasBoard` /
//!    `BlockCollectionView`). A canonical node is never a mock — it aliases an already-wired event path
//!    (`GraphEvent` / `CanvasEvent` / `BlockViewEvent`). NO-MOCK-NODES: a per-identity node is emitted
//!    ONLY for a block / placement / row the host actually rendered this frame (the MT-041 AC-08
//!    anti-scaffolding gate); a node for a non-drawn item is a FAILURE.
//! 3. Emits ONE node per action / per identity through the SAME `ctx.accesskit_node_builder` hook the
//!    rest of the shell uses ([`crate::accessibility::live`]), with a deterministic stable id derived
//!    from the canonical `author_id` STRING (`egui::Id::new(author_id)`, never an insertion-order id —
//!    HBR-SWARM stability), so a stored swarm reference survives a layout change / frame churn.
//! 4. CONSUMES the AccessKit `Action::Click` request targeted at a canonical / per-identity node within
//!    the same frame ([`KnowledgeActionRegistry::take_dispatched`]), including the parameterized JSON
//!    payload (IN-042-04: `ActionData::Value(json)`), so a swarm `dispatch(graph.open-node, {block_id})`
//!    actually REACHES the pane before the next frame — the registry is the swarm-agent invocation path,
//!    not just a discovery surface.
//!
//! ## HBR-QUIET + viewport separation (CTRL-042-01 / RISK-042-01 / IN-042-01)
//!
//! VIEWPORT state (pan / zoom) is SEPARATE from the per-identity NODE SET. The global control nodes
//! (`graph.pan-*` / `graph.zoom-*`) are registered ONCE as fixed controls regardless of content
//! (AC-042-08), so a pan/zoom never re-registers the per-node set. The per-identity nodes are
//! re-registered ONLY when the node set changes (block add/remove/rename), detected by a per-node
//! content-hash [`std::collections::BTreeMap`] (IN-042-01): the registry hashes the present node set
//! before pushing into the live tree, and [`Self::state_changed_since_last_push`] reports `false` on a
//! steady-state frame so the host schedules no AccessKit diff churn (HBR-QUIET).
//!
//! ## Stable-id contract (HBR-SWARM / IN-042-10)
//!
//! `graph.node.<block_id>` is stable for the lifetime of the block; `canvas.card.<placement_id>` is
//! stable for the lifetime of the placement; `collection.row.<block_id>` is stable for the lifetime of
//! the row's block. Deletion is signalled by ABSENCE from the AccessKit tree, NOT by a tombstone. ids
//! carry NO positional or frame-count segment (HBR-SWARM). placement_ids are real UUIDs (RISK-042-02 /
//! CTRL-042-02 — the backend mints `Uuid::new_v4()` at placement creation).
//!
//! ## Pagination / large-graph contract (CTRL-042-06 / RISK-042-06)
//!
//! A workspace with thousands of blocks must not register thousands of AccessKit nodes at once. The
//! graph pane registers per-node AccessKit nodes ONLY for blocks currently in the graph viewport plus a
//! [`VIEWPORT_LOOKAHEAD`]-node lookahead buffer; the host re-derives the visible set as the viewport
//! pans (MT-021 already clamps the loaded set to `NODE_CAP`). A swarm agent that needs an off-screen
//! node must first dispatch `graph.pan-*` or `graph.select-node` to bring it into the viewport. This
//! contract is documented here so a swarm agent and the host agree on the visible-set boundary.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use egui::accesskit;

/// The always-present canary node id (mirrors MT-041's [`super::editor_action_registry::HEALTH_CANARY_AUTHOR_ID`]).
/// A kittest asserts this node is in the live tree so an empty / false-green tree (AccessKit never
/// initialized) cannot pass silently (RISK-042-06's adjacent false-green guard).
pub const HEALTH_CANARY_AUTHOR_ID: &str = "knowledge.accesskit.health";

/// The per-node lookahead buffer (CTRL-042-06): the graph pane registers AccessKit nodes for the
/// viewport-visible blocks PLUS this many lookahead nodes, so a swarm agent has a small off-screen
/// margin without forcing the whole (capped) graph into the tree at once.
pub const VIEWPORT_LOOKAHEAD: usize = 50;

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// author_id prefixes for the per-identity keyed nodes (IN-042-01/02/03). These deliberately REUSE the
// MT-021 graph-node prefix (`graph.node.`) so MT-042 aliases the existing node; `canvas.card.` is the
// canonical MT-042 name for a placement (MT-026 also emits `canvas.placement.<id>`, kept live by the
// canvas pane — both key off the same placement id, so they are the SAME widget addressed by two
// documented ids, never two parallel nodes). `collection.row.` / `collection.lane.` are MT-042's
// canonical collection identity prefixes.
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Prefix for a graph node identity node: `graph.node.<sanitized_block_id>` (Role::TreeItem). This is
/// the SAME prefix [`crate::graph::graph_view::NODE_AUTHOR_ID_PREFIX`] uses, so MT-042 reuses it.
pub const GRAPH_NODE_AUTHOR_ID_PREFIX: &str = "graph.node.";

/// Prefix for a canvas placement identity node: `canvas.card.<sanitized_placement_id>` (Role::Group).
pub const CANVAS_CARD_AUTHOR_ID_PREFIX: &str = "canvas.card.";

/// Prefix for a collection row identity node: `collection.row.<sanitized_block_id>` (Role::Row).
pub const COLLECTION_ROW_AUTHOR_ID_PREFIX: &str = "collection.row.";

/// Prefix for a collection Kanban lane container node: `collection.lane.<sanitized_tag>` (Role::Group).
pub const COLLECTION_LANE_AUTHOR_ID_PREFIX: &str = "collection.lane.";

/// The stable AccessKit author_id for a graph node identity (`graph.node.<block_id>`), block id
/// sanitized to `[a-z0-9-]` (RISK / id-integrity), reusing the shell's [`crate::project_tree::stable_part`].
pub fn graph_node_author_id(block_id: &str) -> String {
    format!(
        "{GRAPH_NODE_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(block_id)
    )
}

/// The stable AccessKit author_id for a canvas placement identity (`canvas.card.<placement_id>`).
pub fn canvas_card_author_id(placement_id: &str) -> String {
    format!(
        "{CANVAS_CARD_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(placement_id)
    )
}

/// The stable AccessKit author_id for a collection row identity (`collection.row.<block_id>`).
pub fn collection_row_author_id(block_id: &str) -> String {
    format!(
        "{COLLECTION_ROW_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(block_id)
    )
}

/// The stable AccessKit author_id for a collection Kanban lane container (`collection.lane.<tag>`).
pub fn collection_lane_author_id(lane_tag: &str) -> String {
    format!(
        "{COLLECTION_LANE_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(lane_tag)
    )
}

/// Which knowledge surface a registered node belongs to — the `<surface>` segment of the canonical
/// `<surface>.<action>` author_id (IN-042-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KnowledgeSurface {
    /// The E3 Loom graph view ([`crate::graph::graph_view::LoomGraphView`]).
    Graph,
    /// The E3 Loom canvas board ([`crate::graph::canvas_board::LoomCanvasBoard`]).
    Canvas,
    /// The E3 block-collection views ([`crate::graph::block_collection_view::BlockCollectionView`]).
    Collection,
}

impl KnowledgeSurface {
    /// The stable `<surface>` segment string.
    pub fn as_str(self) -> &'static str {
        match self {
            KnowledgeSurface::Graph => "graph",
            KnowledgeSurface::Canvas => "canvas",
            KnowledgeSurface::Collection => "collection",
        }
    }
}

/// The AccessKit role MT-042 declares for a knowledge node, kept as a small closed enum so the contract
/// vocabulary is explicit at the call site. All map to field-correct `accesskit::Role` values in
/// accesskit 0.21.1 (verified against the crate source — `TreeItem` / `Group` / `Row` / `Button` all
/// exist).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxRole {
    /// A momentary action button (global controls — pan/zoom/sort/etc.).
    Button,
    /// A graph node identity node (`graph.node.<block_id>`).
    TreeItem,
    /// A canvas placement / Kanban lane container identity node.
    Group,
    /// A collection table/calendar row identity node.
    Row,
}

impl AxRole {
    /// The field-correct `accesskit::Role` for this contract role.
    pub fn accesskit_role(self) -> accesskit::Role {
        match self {
            AxRole::Button => accesskit::Role::Button,
            AxRole::TreeItem => accesskit::Role::TreeItem,
            AxRole::Group => accesskit::Role::Group,
            AxRole::Row => accesskit::Role::Row,
        }
    }

    /// The stable debug-name string the snapshot reports for this role.
    pub fn role_str(self) -> &'static str {
        match self {
            AxRole::Button => "Button",
            AxRole::TreeItem => "TreeItem",
            AxRole::Group => "Group",
            AxRole::Row => "Row",
        }
    }
}

/// The mutable, per-frame state of one knowledge node. Pushed into the registry by the pane each frame
/// so the AccessKit node reflects the live surface.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct KnowledgeNodeState {
    /// Whether the backing widget / item is actually rendered this frame. A node whose backing widget
    /// is not drawn (e.g. a graph node outside the viewport+lookahead window, or a kanban action while
    /// the view is in table mode) is marked `present=false` and is NOT emitted into the live tree
    /// (AC-042-08 anti-scaffolding — never a node for a widget not on screen / a deleted item; IN-042-10
    /// deletion-by-absence).
    pub present: bool,
    /// Whether the action is currently enabled (a disabled node is emitted but a swarm dispatch on it is
    /// rejected by the MCP action channel — see [`crate::mcp::action::resolve_target`]).
    pub enabled: bool,
}

impl KnowledgeNodeState {
    /// A present, enabled state.
    pub fn present() -> Self {
        Self {
            present: true,
            enabled: true,
        }
    }

    /// A present-but-disabled state (a discoverable typed-gap node a dispatch is rejected on, never a
    /// silent no-op).
    pub fn present_disabled() -> Self {
        Self {
            present: true,
            enabled: false,
        }
    }

    /// An absent state — the backing widget is not rendered this frame, so the node is suppressed.
    pub fn absent() -> Self {
        Self {
            present: false,
            enabled: false,
        }
    }
}

/// One registered knowledge node — the discovery record a swarm agent reads, and the dispatch alias the
/// registry invokes. Keyed in the registry by its canonical [`Self::author_id`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeActionNode {
    /// The canonical, deterministic AccessKit address (`graph.pan-left`, `graph.node.<block_id>`,
    /// `canvas.card.<placement_id>`, `collection.row.<block_id>`, …). Stable across frames + restarts
    /// (HBR-SWARM / RISK-042-01).
    pub author_id: String,
    /// The contract role for this node.
    pub role: AxRole,
    /// A human / model-readable label.
    pub label: String,
    /// Optional extra-data the node exposes via the AccessKit `value` field (e.g. a canvas card's
    /// `block_id=<uuid>` so a swarm agent correlates a placement to its source block — IN-042-02; or a
    /// collection row's lane). `None` for a plain control.
    pub value: Option<String>,
    /// The AccessKit actions this node declares (debug-name strings, e.g. `"Click"`, `"Focus"`). Every
    /// node declares at least one (AC-042-02/03). A swarm agent dispatches one of these.
    pub actions: Vec<String>,
    /// The live per-frame state (present / enabled).
    pub state: KnowledgeNodeState,
}

impl KnowledgeActionNode {
    /// The fixed `egui::Id` (and thus AccessKit `NodeId`) backing this node, derived from its STABLE
    /// canonical author_id STRING — NOT an insertion-order id (RISK-042-01). The same author_id always
    /// yields the same id across frames + after a layout change.
    pub fn egui_id(&self) -> egui::Id {
        egui::Id::new(&self.author_id)
    }
}

/// A thin hashing view so the registry can hash the full node set without requiring `Hash` on the
/// public node type (which carries `String` fields).
struct KnowledgeNodeHashView<'a>(&'a KnowledgeActionNode);

impl Hash for KnowledgeNodeHashView<'_> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.0.author_id.hash(h);
        self.0.role.hash(h);
        self.0.label.hash(h);
        self.0.value.hash(h);
        self.0.actions.hash(h);
        self.0.state.hash(h);
    }
}

/// The single source of truth for knowledge-surface AccessKit node identity (IN-042-01/08).
///
/// Holds a stable `BTreeMap<author_id, KnowledgeActionNode>` (sorted, so the snapshot / dump is
/// deterministic) and gates the AccessKit push on a content hash that EXCLUDES viewport state
/// (HBR-QUIET / CTRL-042-01 / IN-042-01). The registry is wrapped by the host in
/// `Arc<Mutex<KnowledgeActionRegistry>>` (the MT-041 pattern) so the render path and the AccessKit poll
/// can both reach it; this type itself is plain (no interior locking) so its unit tests are deterministic.
#[derive(Debug, Default)]
pub struct KnowledgeActionRegistry {
    nodes: BTreeMap<String, KnowledgeActionNode>,
    /// Per-node content hash, keyed by author_id (IN-042-01: "keep a `BTreeMap<.., u64>` of per-node
    /// content hashes to detect changes cheaply"). Lets the host ask which identity nodes actually
    /// changed without re-hashing the whole set.
    per_node_hash: BTreeMap<String, u64>,
    /// The content hash of the last node set pushed to the AccessKit surface, for the unchanged-skip
    /// decision (HBR-QUIET). `None` until the first push.
    last_push_hash: Option<u64>,
}

impl KnowledgeActionRegistry {
    /// A fresh, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update one node by its canonical author_id. Called by the pane each frame for every
    /// control / identity it renders, with the live [`KnowledgeNodeState`]. Panics in debug builds if a
    /// DIFFERENT caller claims an author_id already owned with a different role (duplicate-id guard,
    /// IN-042-08 no-cross-registry-collision); in release it logs and keeps the first registration.
    pub fn upsert(
        &mut self,
        author_id: impl Into<String>,
        role: AxRole,
        label: impl Into<String>,
        value: Option<String>,
        actions: Vec<String>,
        state: KnowledgeNodeState,
    ) {
        let author_id = author_id.into();
        if let Some(existing) = self.nodes.get(&author_id) {
            if existing.role != role {
                debug_assert!(
                    false,
                    "KnowledgeActionRegistry: author_id '{author_id}' re-registered with a different \
                     role ({:?} vs {:?}) — duplicate/colliding node (IN-042-08)",
                    existing.role, role
                );
                tracing::error!(
                    author_id = %author_id,
                    "knowledge action node re-registered with a conflicting role; keeping the first"
                );
                return;
            }
        }
        let node = KnowledgeActionNode {
            author_id: author_id.clone(),
            role,
            label: label.into(),
            value,
            actions,
            state,
        };
        // Update the per-node hash so the host can detect which identities changed (IN-042-01).
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        KnowledgeNodeHashView(&node).hash(&mut hasher);
        self.per_node_hash
            .insert(author_id.clone(), hasher.finish());
        self.nodes.insert(author_id, node);
    }

    /// Register a global control button (a `<surface>.<action>` node) with the standard
    /// `["Click","Focus"]` action set. Convenience over [`Self::upsert`] for the fixed controls.
    pub fn upsert_control(
        &mut self,
        author_id: impl Into<String>,
        label: impl Into<String>,
        state: KnowledgeNodeState,
    ) {
        self.upsert(
            author_id,
            AxRole::Button,
            label,
            None,
            vec!["Click".to_owned(), "Focus".to_owned()],
            state,
        );
    }

    /// Register a per-identity node (graph node / canvas card / collection row / lane). Declares the
    /// `["Click","Focus"]` action set (a swarm agent activates the identity by Click — open / select);
    /// `value` carries the extra-data (e.g. a canvas card's `block_id=`).
    pub fn upsert_identity(
        &mut self,
        author_id: impl Into<String>,
        role: AxRole,
        label: impl Into<String>,
        value: Option<String>,
        state: KnowledgeNodeState,
    ) {
        self.upsert(
            author_id,
            role,
            label,
            value,
            vec!["Click".to_owned(), "Focus".to_owned()],
            state,
        );
    }

    /// Register a per-identity node declaring EXTRA contract actions beyond `["Click","Focus"]`. Used by
    /// the canvas card, which AC-042-03 requires to declare both `activate` (Click) and `delete`: the
    /// `delete` is a discoverable, swarm-readable capability on the card node itself, dispatched (via
    /// `canvas.remove-placement {placement_id}`) by a swarm agent that read it on the card. The extra
    /// action strings are debug-name capability labels (e.g. `"delete"`) a swarm reads from the node's
    /// declared action set — they are NOT egui Action variants, so they are advertised here and routed
    /// by the pane, never silently no-op.
    pub fn upsert_identity_with_actions(
        &mut self,
        author_id: impl Into<String>,
        role: AxRole,
        label: impl Into<String>,
        value: Option<String>,
        extra_actions: &[&str],
        state: KnowledgeNodeState,
    ) {
        let mut actions = vec!["Click".to_owned(), "Focus".to_owned()];
        actions.extend(extra_actions.iter().map(|a| (*a).to_owned()));
        self.upsert(author_id, role, label, value, actions, state);
    }

    /// Look up a node by its canonical author_id.
    pub fn node(&self, author_id: &str) -> Option<&KnowledgeActionNode> {
        self.nodes.get(author_id)
    }

    /// The per-node content hash recorded at the last [`Self::upsert`] for this author_id (IN-042-01),
    /// so the host can cheaply tell whether one identity node changed since a prior frame.
    pub fn node_hash(&self, author_id: &str) -> Option<u64> {
        self.per_node_hash.get(author_id).copied()
    }

    /// All registered nodes, in deterministic (author_id-sorted) order.
    pub fn nodes(&self) -> impl Iterator<Item = &KnowledgeActionNode> {
        self.nodes.values()
    }

    /// The number of registered nodes (excludes the always-emitted health canary, emitted directly).
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True when no nodes are registered.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// All present per-identity author_ids for a surface that match the given prefix (e.g. every live
    /// `graph.node.` id), so the host / a test can enumerate the current identity set. Excludes absent
    /// nodes (deletion-by-absence, IN-042-10).
    pub fn present_ids_with_prefix(&self, prefix: &str) -> Vec<&str> {
        self.nodes
            .values()
            .filter(|n| n.state.present && n.author_id.starts_with(prefix))
            .map(|n| n.author_id.as_str())
            .collect()
    }

    /// Clear all registered nodes. Called by a host that fully re-derives the node set each frame, so an
    /// identity whose backing item disappeared is DROPPED from the tree rather than going stale
    /// (deletion-by-absence, IN-042-10). The per-node-hash map is cleared in lock-step.
    pub fn clear_nodes(&mut self) {
        self.nodes.clear();
        self.per_node_hash.clear();
    }

    /// Content hash of the full PRESENT node set (author_id + role + label + value + actions + state),
    /// for the HBR-QUIET unchanged-skip decision (IN-042-01 / CTRL-042-01). VIEWPORT state (pan/zoom) is
    /// NOT part of any node here — the global pan/zoom controls are fixed Button nodes whose state never
    /// changes on a pan — so a pan/zoom frame does NOT change this hash (the viewport-separation
    /// requirement). Absent nodes are excluded because they are not emitted.
    fn content_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for node in self.nodes.values().filter(|n| n.state.present) {
            KnowledgeNodeHashView(node).hash(&mut hasher);
        }
        hasher.finish()
    }

    /// The HBR-QUIET gate (IN-042-01 / CTRL-042-01): returns `true` and records the new hash when the
    /// PRESENT node set changed since the last call, `false` when it is unchanged (so the host skips
    /// scheduling a repaint / AccessKit notify on a steady-state — or pan/zoom-only — frame). This is the
    /// "push-on-change only" hook; the per-frame node emission ([`Self::emit_into_tree`]) still runs
    /// every frame (egui needs the node present), but the diff-notify decision is gated here.
    pub fn state_changed_since_last_push(&mut self) -> bool {
        let hash = self.content_hash();
        let changed = self.last_push_hash != Some(hash);
        if changed {
            self.last_push_hash = Some(hash);
        }
        changed
    }

    /// Emit every PRESENT registered node into the live AccessKit tree through the shell's own
    /// `ctx.accesskit_node_builder` hook ([`crate::accessibility::live`] uses the same path), plus the
    /// always-present health canary. Each node is keyed by its STABLE `egui::Id::new(author_id)`
    /// (RISK-042-01), so the id is identical every frame + after a layout change. Absent nodes are
    /// skipped (AC-042-08 / IN-042-10 deletion-by-absence). A node's `value` (when set) is exposed via
    /// the AccessKit `value` field (IN-042-02 — e.g. a canvas card's source `block_id`).
    ///
    /// No-op for the node body when AccessKit is not active this frame (`accesskit_node_builder`
    /// returns `None`), matching egui's graceful-degradation contract.
    pub fn emit_into_tree(&self, ui: &egui::Ui) {
        let ctx = ui.ctx();
        // The always-present canary: a non-empty-tree witness so a false-green empty tree cannot pass.
        let canary_id = egui::Id::new(HEALTH_CANARY_AUTHOR_ID);
        ctx.accesskit_node_builder(canary_id, |node| {
            node.set_role(accesskit::Role::Status);
            node.set_author_id(HEALTH_CANARY_AUTHOR_ID.to_owned());
            node.set_label("knowledge accesskit surface live".to_owned());
        });

        for node in self.nodes.values() {
            if !node.state.present {
                continue;
            }
            let role = node.role.accesskit_role();
            let author_id = node.author_id.clone();
            let label = node.label.clone();
            let value = node.value.clone();
            let enabled = node.state.enabled;
            // Extra contract capabilities beyond Click/Focus (e.g. a canvas card's `delete` — AC-042-03)
            // are surfaced as real AccessKit CUSTOM actions so a swarm agent genuinely READS them on the
            // node (not a faked string): each gets a stable id (its index) + the capability name as the
            // description. The pane routes a custom-action dispatch to the matching event.
            let custom: Vec<accesskit::CustomAction> = node
                .actions
                .iter()
                .filter(|a| a.as_str() != "Click" && a.as_str() != "Focus")
                .enumerate()
                .map(|(i, name)| accesskit::CustomAction {
                    id: i as i32,
                    description: name.clone().into(),
                })
                .collect();
            ctx.accesskit_node_builder(node.egui_id(), move |n| {
                n.set_role(role);
                n.set_author_id(author_id.clone());
                n.set_label(label.clone());
                if let Some(v) = &value {
                    n.set_value(v.clone());
                }
                // Click is the activation a swarm agent dispatches; Focus lets it move to the node first
                // (the AccessKit default-action contract). A parameterized action carries its JSON in the
                // request's `ActionData::Value` payload (IN-042-04); the action itself is still Click.
                n.add_action(accesskit::Action::Click);
                n.add_action(accesskit::Action::Focus);
                if !custom.is_empty() {
                    // Declare the CustomAction capability set + the CustomAction action so a swarm sees it.
                    n.add_action(accesskit::Action::CustomAction);
                    n.set_custom_actions(custom.clone());
                }
                if !enabled {
                    n.set_disabled();
                }
            });
        }
    }

    /// Drain this frame's AccessKit `Action::Click` requests targeted at the registered nodes and return
    /// the activated `(author_id, payload)` pairs in dispatch order. The host calls this in its `show`
    /// (BEFORE it would render the next frame) and routes each returned author_id + JSON payload to the
    /// real pane action it aliases (RISK-042-04 — the dispatch REACHES the pane, consumed within the
    /// frame). `payload` is the [`accesskit::ActionData::Value`] string carried by a parameterized
    /// dispatch (IN-042-04), or `None` for a plain control.
    ///
    /// Reuses egui's own `input.accesskit_action_requests(node_id, action)` consumer (the same hook
    /// MT-041 uses), so a swarm agent's `egui::Event::AccessKitActionRequest` (built by
    /// [`crate::mcp::action`]) drives a canonical node exactly like a real click.
    pub fn take_dispatched(&self, ui: &egui::Ui) -> Vec<(String, Option<String>)> {
        let mut activated = Vec::new();
        ui.input(|input| {
            for node in self.nodes.values() {
                if !node.state.present || !node.state.enabled {
                    continue;
                }
                let id = node.egui_id();
                let mut payload: Option<String> = None;
                let mut clicked = false;
                for request in input.accesskit_action_requests(id, accesskit::Action::Click) {
                    clicked = true;
                    // Parameterized action payload (IN-042-04): the JSON string travels in
                    // `ActionData::Value`. Last request of the frame wins (a swarm normally sends one).
                    if let Some(accesskit::ActionData::Value(v)) = &request.data {
                        payload = Some(v.to_string());
                    }
                }
                if clicked {
                    activated.push((node.author_id.clone(), payload));
                }
                // CustomAction dispatch (AC-042-03 card `delete`): a swarm CustomAction request carries the
                // capability index in `ActionData::CustomAction(i)`; map it back to the node's extra-action
                // name (the i-th non-Click/Focus action) and surface it as a synthetic
                // `<author_id>#<capability>` dispatch the pane routes (e.g. a card delete -> RemovePlacement).
                let extras: Vec<&str> = node
                    .actions
                    .iter()
                    .filter(|a| a.as_str() != "Click" && a.as_str() != "Focus")
                    .map(|a| a.as_str())
                    .collect();
                if !extras.is_empty() {
                    for request in
                        input.accesskit_action_requests(id, accesskit::Action::CustomAction)
                    {
                        if let Some(accesskit::ActionData::CustomAction(i)) = &request.data {
                            if let Some(name) = extras.get(*i as usize) {
                                activated.push((format!("{}#{name}", node.author_id), None));
                            }
                        }
                    }
                }
            }
        });
        activated
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Canonical action catalogs (IN-042-01/07) + dispatch mapping + parameterized-payload parsing.
//
// These are the ONE swarm-facing global-control vocabulary for each surface. Each entry maps a
// canonical `<surface>.<action>` id to (a) its label and (b) whether it carries a JSON payload. The
// per-identity nodes (graph.node.*, canvas.card.*, collection.row.*, collection.lane.*) are NOT in
// these catalogs — they are emitted dynamically by the pane from the live item set.
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// One canonical global-control entry: the full `author_id`, a label, and whether a swarm dispatch on
/// it carries a JSON payload (IN-042-04 parameterized action).
#[derive(Debug, Clone, Copy)]
pub struct ControlEntry {
    /// The full canonical `<surface>.<action>` author_id.
    pub author_id: &'static str,
    /// Human / model-readable label.
    pub label: &'static str,
    /// True when a swarm dispatch on this control carries a JSON `ActionData::Value` payload (IN-042-04).
    pub parameterized: bool,
}

/// The GRAPH global controls (IN-042-07 + the contract scope.summary graph action list). pan/zoom/reset/
/// deselect are plain; open-node / select-node / add-edge / remove-edge carry a JSON payload.
pub const GRAPH_CONTROL_CATALOG: &[ControlEntry] = &[
    ControlEntry {
        author_id: "graph.pan-left",
        label: "Pan graph left",
        parameterized: false,
    },
    ControlEntry {
        author_id: "graph.pan-right",
        label: "Pan graph right",
        parameterized: false,
    },
    ControlEntry {
        author_id: "graph.zoom-in",
        label: "Zoom graph in",
        parameterized: false,
    },
    ControlEntry {
        author_id: "graph.zoom-out",
        label: "Zoom graph out",
        parameterized: false,
    },
    ControlEntry {
        author_id: "graph.zoom-reset",
        label: "Reset graph zoom",
        parameterized: false,
    },
    ControlEntry {
        author_id: "graph.deselect-all",
        label: "Deselect all graph nodes",
        parameterized: false,
    },
    ControlEntry {
        author_id: "graph.open-node",
        label: "Open graph node by block id",
        parameterized: true,
    },
    ControlEntry {
        author_id: "graph.select-node",
        label: "Select graph node by block id",
        parameterized: true,
    },
    ControlEntry {
        author_id: "graph.add-edge",
        label: "Add graph edge",
        parameterized: true,
    },
    ControlEntry {
        author_id: "graph.remove-edge",
        label: "Remove graph edge",
        parameterized: true,
    },
];

/// The CANVAS global controls (scope.summary canvas action list).
pub const CANVAS_CONTROL_CATALOG: &[ControlEntry] = &[
    ControlEntry {
        author_id: "canvas.pan-left",
        label: "Pan canvas left",
        parameterized: false,
    },
    ControlEntry {
        author_id: "canvas.pan-right",
        label: "Pan canvas right",
        parameterized: false,
    },
    ControlEntry {
        author_id: "canvas.zoom-in",
        label: "Zoom canvas in",
        parameterized: false,
    },
    ControlEntry {
        author_id: "canvas.zoom-out",
        label: "Zoom canvas out",
        parameterized: false,
    },
    ControlEntry {
        author_id: "canvas.zoom-reset",
        label: "Reset canvas zoom",
        parameterized: false,
    },
    ControlEntry {
        author_id: "canvas.deselect-all",
        label: "Deselect all canvas cards",
        parameterized: false,
    },
    ControlEntry {
        author_id: "canvas.add-card",
        label: "Add canvas text card",
        parameterized: true,
    },
    ControlEntry {
        author_id: "canvas.place-block",
        label: "Place block on canvas",
        parameterized: true,
    },
    ControlEntry {
        author_id: "canvas.remove-placement",
        label: "Remove canvas placement",
        parameterized: true,
    },
    ControlEntry {
        author_id: "canvas.add-edge",
        label: "Add canvas edge",
        parameterized: true,
    },
    ControlEntry {
        author_id: "canvas.remove-edge",
        label: "Remove canvas edge",
        parameterized: true,
    },
    ControlEntry {
        author_id: "canvas.select-card",
        label: "Select canvas card by placement id",
        parameterized: true,
    },
];

/// The COLLECTION global controls (scope.summary collection action list).
pub const COLLECTION_CONTROL_CATALOG: &[ControlEntry] = &[
    ControlEntry {
        author_id: "collection.sort",
        label: "Sort collection column",
        parameterized: true,
    },
    ControlEntry {
        author_id: "collection.filter",
        label: "Filter collection",
        parameterized: true,
    },
    ControlEntry {
        author_id: "collection.kanban-move",
        label: "Move kanban card between lanes",
        parameterized: true,
    },
    ControlEntry {
        author_id: "collection.calendar-next",
        label: "Calendar next period",
        parameterized: false,
    },
    ControlEntry {
        author_id: "collection.calendar-prev",
        label: "Calendar previous period",
        parameterized: false,
    },
    ControlEntry {
        author_id: "collection.calendar-today",
        label: "Calendar today",
        parameterized: false,
    },
    ControlEntry {
        author_id: "collection.open-block",
        label: "Open collection block by block id",
        parameterized: true,
    },
];

// ── Parameterized-action payload types (IN-042-04). Parsed via serde_json::from_str MATCH (NEVER
//    unwrap — RISK-042-03 / CTRL-042-03 no panic on a malformed payload). ───────────────────────────

/// `graph.open-node` / `graph.select-node` / `collection.open-block` payload: `{"block_id":"<uuid>"}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct BlockIdPayload {
    pub block_id: String,
}

/// `graph.add-edge` / `canvas.add-edge` payload: `{"source_id":"..","target_id":".."}` (+ optional
/// `edge_mode` for the canvas semantic/visual distinction).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct AddEdgePayload {
    pub source_id: String,
    pub target_id: String,
    /// Optional `"semantic"` / `"visual"` (canvas only). Absent => semantic (the default loom edge).
    #[serde(default)]
    pub edge_mode: Option<String>,
}

/// `graph.remove-edge` / `canvas.remove-edge` payload: `{"edge_id":".."}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct EdgeIdPayload {
    pub edge_id: String,
}

/// `canvas.place-block` payload: `{"block_id":"<uuid>","x":100,"y":100}`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlaceBlockPayload {
    pub block_id: String,
    pub x: f32,
    pub y: f32,
}

/// `canvas.remove-placement` / `canvas.select-card` payload: `{"placement_id":".."}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PlacementIdPayload {
    pub placement_id: String,
}

/// `collection.kanban-move` payload: `{"block_id":"..","from_lane":"..","to_lane":".."}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct KanbanMovePayload {
    pub block_id: String,
    pub from_lane: String,
    pub to_lane: String,
}

/// `collection.sort` payload: `{"field":"title","direction":"asc"}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct SortPayload {
    pub field: String,
    #[serde(default)]
    pub direction: Option<String>,
}

/// Parse a parameterized-action JSON payload string into `T`, MATCHING the result (NEVER unwrap —
/// RISK-042-03 / CTRL-042-03). Logs and returns `None` on a malformed / missing payload so the action
/// handler early-returns (no panic, no fake dispatch). This is the single parse seam every pane uses.
pub fn parse_payload<T: serde::de::DeserializeOwned>(payload: Option<&str>) -> Option<T> {
    let raw = match payload {
        Some(p) => p,
        None => {
            tracing::warn!("knowledge action: parameterized dispatch carried no payload; ignored");
            return None;
        }
    };
    match serde_json::from_str::<T>(raw) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!(error = %e, payload = %raw, "knowledge action: malformed JSON payload; ignored");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_author_ids_sanitize_and_prefix() {
        // graph.node reuses the MT-021 prefix; canvas.card / collection.row / collection.lane are MT-042.
        assert_eq!(graph_node_author_id("blk-7").as_str(), "graph.node.blk-7");
        assert_eq!(canvas_card_author_id("p-1").as_str(), "canvas.card.p-1");
        assert_eq!(
            collection_row_author_id("blk-7").as_str(),
            "collection.row.blk-7"
        );
        assert_eq!(
            collection_lane_author_id("todo").as_str(),
            "collection.lane.todo"
        );
        // A raw id with slashes/colons/spaces sanitizes to [a-z0-9-] (id-integrity, RISK).
        let id = graph_node_author_id("ws:1/Block 7#x");
        let suffix = &id[GRAPH_NODE_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
    }

    #[test]
    fn role_maps_to_field_correct_accesskit_role() {
        assert_eq!(AxRole::Button.accesskit_role(), accesskit::Role::Button);
        assert_eq!(AxRole::TreeItem.accesskit_role(), accesskit::Role::TreeItem);
        assert_eq!(AxRole::Group.accesskit_role(), accesskit::Role::Group);
        assert_eq!(AxRole::Row.accesskit_role(), accesskit::Role::Row);
    }

    #[test]
    fn upsert_and_lookup_roundtrip_with_per_node_hash() {
        let mut reg = KnowledgeActionRegistry::new();
        reg.upsert_control(
            "graph.pan-left",
            "Pan graph left",
            KnowledgeNodeState::present(),
        );
        let node = reg.node("graph.pan-left").expect("control present");
        assert_eq!(node.role, AxRole::Button);
        assert!(
            node.actions.iter().any(|a| a == "Click"),
            "declares >=1 (Click) action"
        );
        assert!(
            reg.node_hash("graph.pan-left").is_some(),
            "per-node hash recorded (IN-042-01)"
        );
    }

    #[test]
    fn hbr_quiet_skips_unchanged_and_viewport_only_frames() {
        let mut reg = KnowledgeActionRegistry::new();
        // Global pan/zoom controls registered ONCE (AC-042-08) — their state never changes on a pan.
        for entry in GRAPH_CONTROL_CATALOG {
            reg.upsert_control(entry.author_id, entry.label, KnowledgeNodeState::present());
        }
        // A graph node identity.
        reg.upsert_identity(
            graph_node_author_id("blk-1"),
            AxRole::TreeItem,
            "Note one",
            None,
            KnowledgeNodeState::present(),
        );
        // First push reports changed.
        assert!(
            reg.state_changed_since_last_push(),
            "first push is a change"
        );
        // Re-registering the SAME controls + node (a pan/zoom-only frame re-runs registration but the
        // node SET is identical) reports unchanged — viewport state is not in the hash (CTRL-042-01).
        for entry in GRAPH_CONTROL_CATALOG {
            reg.upsert_control(entry.author_id, entry.label, KnowledgeNodeState::present());
        }
        reg.upsert_identity(
            graph_node_author_id("blk-1"),
            AxRole::TreeItem,
            "Note one",
            None,
            KnowledgeNodeState::present(),
        );
        assert!(
            !reg.state_changed_since_last_push(),
            "pan/zoom-only (unchanged node set) reports unchanged (HBR-QUIET)"
        );
        // Adding a NEW node IS a change (the node set actually changed).
        reg.upsert_identity(
            graph_node_author_id("blk-2"),
            AxRole::TreeItem,
            "Note two",
            None,
            KnowledgeNodeState::present(),
        );
        assert!(
            reg.state_changed_since_last_push(),
            "adding a node is a change"
        );
    }

    #[test]
    fn absent_identity_excluded_from_present_set_and_hash() {
        let mut reg = KnowledgeActionRegistry::new();
        reg.upsert_identity(
            graph_node_author_id("blk-1"),
            AxRole::TreeItem,
            "n1",
            None,
            KnowledgeNodeState::present(),
        );
        assert!(reg.state_changed_since_last_push());
        // A deleted node marked absent (deletion-by-absence, IN-042-10) does NOT change the present hash.
        reg.upsert_identity(
            graph_node_author_id("blk-2"),
            AxRole::TreeItem,
            "n2",
            None,
            KnowledgeNodeState::absent(),
        );
        assert!(
            !reg.state_changed_since_last_push(),
            "absent node does not change the push hash"
        );
        // present_ids_with_prefix excludes the absent node.
        let present = reg.present_ids_with_prefix(GRAPH_NODE_AUTHOR_ID_PREFIX);
        assert_eq!(
            present,
            vec!["graph.node.blk-1"],
            "only the present node is enumerated"
        );
    }

    #[test]
    fn parse_payload_matches_never_unwraps() {
        // Valid payloads parse.
        let p: BlockIdPayload =
            parse_payload(Some(r#"{"block_id":"blk-7"}"#)).expect("valid block id");
        assert_eq!(p.block_id, "blk-7");
        let pb: PlaceBlockPayload =
            parse_payload(Some(r#"{"block_id":"blk-7","x":100,"y":100}"#)).expect("valid place");
        assert_eq!((pb.x, pb.y), (100.0, 100.0));
        let km: KanbanMovePayload = parse_payload(Some(
            r#"{"block_id":"b","from_lane":"todo","to_lane":"done"}"#,
        ))
        .expect("valid move");
        assert_eq!(
            (km.from_lane.as_str(), km.to_lane.as_str()),
            ("todo", "done")
        );
        // Malformed JSON -> None, no panic (RISK-042-03 / CTRL-042-03).
        let bad: Option<BlockIdPayload> = parse_payload(Some("not json {{"));
        assert!(bad.is_none(), "malformed JSON yields None, never a panic");
        // Missing required field -> None.
        let missing: Option<PlaceBlockPayload> = parse_payload(Some(r#"{"block_id":"x"}"#));
        assert!(missing.is_none(), "missing required field yields None");
        // No payload at all -> None.
        let none: Option<BlockIdPayload> = parse_payload(None);
        assert!(none.is_none(), "no payload yields None");
    }

    #[test]
    fn control_catalogs_cover_the_contract_action_lists() {
        // The exact global-control ids the MT scope.summary + IN-042-07 list, per surface.
        let graph: Vec<&str> = GRAPH_CONTROL_CATALOG.iter().map(|e| e.author_id).collect();
        for want in [
            "graph.pan-left",
            "graph.pan-right",
            "graph.zoom-in",
            "graph.zoom-out",
            "graph.zoom-reset",
            "graph.open-node",
            "graph.add-edge",
            "graph.remove-edge",
            "graph.select-node",
            "graph.deselect-all",
        ] {
            assert!(graph.contains(&want), "graph catalog missing '{want}'");
        }
        let canvas: Vec<&str> = CANVAS_CONTROL_CATALOG.iter().map(|e| e.author_id).collect();
        for want in [
            "canvas.pan-left",
            "canvas.pan-right",
            "canvas.zoom-in",
            "canvas.zoom-out",
            "canvas.zoom-reset",
            "canvas.add-card",
            "canvas.place-block",
            "canvas.remove-placement",
            "canvas.add-edge",
            "canvas.remove-edge",
            "canvas.select-card",
            "canvas.deselect-all",
        ] {
            assert!(canvas.contains(&want), "canvas catalog missing '{want}'");
        }
        let coll: Vec<&str> = COLLECTION_CONTROL_CATALOG
            .iter()
            .map(|e| e.author_id)
            .collect();
        for want in [
            "collection.sort",
            "collection.filter",
            "collection.kanban-move",
            "collection.calendar-next",
            "collection.calendar-prev",
            "collection.calendar-today",
            "collection.open-block",
        ] {
            assert!(coll.contains(&want), "collection catalog missing '{want}'");
        }
    }

    #[test]
    fn no_cross_surface_author_id_collision() {
        // IN-042-08: the graph./canvas./collection. prefixes disambiguate, and within MT-042 no two
        // catalog entries share an id; also confirm none collide with the MT-041 editor. prefix.
        let mut all: Vec<&str> = Vec::new();
        all.extend(GRAPH_CONTROL_CATALOG.iter().map(|e| e.author_id));
        all.extend(CANVAS_CONTROL_CATALOG.iter().map(|e| e.author_id));
        all.extend(COLLECTION_CONTROL_CATALOG.iter().map(|e| e.author_id));
        let mut seen = std::collections::HashSet::new();
        for id in &all {
            assert!(seen.insert(*id), "duplicate control author_id '{id}'");
            assert!(
                !id.starts_with("editor."),
                "knowledge id '{id}' must not collide with the MT-041 editor. prefix"
            );
        }
    }
}
