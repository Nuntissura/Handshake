//! Typed pane registry (WP-KERNEL-011 MT-005).
//!
//! The central authority for every pane in the native Handshake work surface. All later C2 MTs
//! (splits MT-006, tabs MT-007, pop-out MT-008, persistence MT-009) read and write this registry
//! rather than inventing their own pane state. Keeping a single source of truth is what lets the
//! layout serialize (MT-009) and lets concurrent agent + operator activity converge on one set of
//! pane records (MT-028) instead of fragmenting.
//!
//! Design choices pinned by the MT-005 contract and the MT-001 toolkit spike verdict:
//! - `PaneId = Arc<str>`: stable kebab-case keys, cheap to clone across the hot render path.
//! - `BTreeMap` keys: stable alphabetical iteration order so layout snapshots and AccessKit trees
//!   are deterministic frame-to-frame and run-to-run.
//! - AccessKit `NodeId`s come from a monotonic `u64` counter owned by the registry, NOT a frame
//!   counter. Frame-counter ids would change every repaint and break out-of-process steering
//!   (RISK-1 / CONTROL-1). The `author_id` on each node is the kebab-case `pane_id` string so a
//!   model can match a pane by a stable, human-meaningful identifier.
//! - `accesskit` is reached through egui's re-export (`egui::accesskit`); the native crate adds no
//!   direct accesskit dependency (it is already a transitive dep at the spike-pinned 0.21.1).

use std::collections::BTreeMap;
use std::sync::Arc;

use egui::accesskit;
use serde::{Deserialize, Serialize};

/// Stable, kebab-case pane key (e.g. `"pane-a"`). `Arc<str>` so registry keys and lookups clone
/// cheaply without per-frame `String` allocations (RISK-4 / CONTROL-4).
pub type PaneId = Arc<str>;

/// The kind of surface a pane hosts. Ported from the React `PaneTabId` union in
/// `app/src/App.tsx`; surfaces not yet built in this WP render through `PlaceholderPaneFactory`.
/// `Placeholder(String)` carries a free-form label for future surfaces that have no dedicated
/// variant yet, so an unknown surface degrades to a labeled placeholder instead of a panic
/// (RISK-3 / CONTROL-3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaneType {
    Workspace,
    LoomDailyJournal,
    LoomBlock,
    LoomWikiPage,
    AtelierEditor,
    KernelDcc,
    InferenceLab,
    ModelRuntime,
    Swarm,
    Problems,
    Jobs,
    Timeline,
    UserManual,
    CodeSymbol,
    SourceControl,
    MediaDownloader,
    FontManager,
    FlightRecorder,
    VisualDebugger,
    /// WP-KERNEL-012 MT-028: the native LoomSearchV2 hybrid-search surface (E4 Search).
    LoomSearchV2,
    /// WP-KERNEL-012 MT-029: the native Find-in-Files + Replace-in-Files surface (E4 Search).
    FindInFiles,
    /// A surface with no dedicated variant yet; the carried string is the display label.
    Placeholder(String),
}

impl PaneType {
    /// Stable human/model-readable label for the surface. Used by the placeholder factory and by
    /// AccessKit so a model can read what a pane is.
    pub fn label(&self) -> String {
        match self {
            PaneType::Workspace => "Workspace".to_owned(),
            PaneType::LoomDailyJournal => "Loom Daily Journal".to_owned(),
            PaneType::LoomBlock => "Loom Block".to_owned(),
            PaneType::LoomWikiPage => "Loom Wiki Page".to_owned(),
            PaneType::AtelierEditor => "Atelier Editor".to_owned(),
            PaneType::KernelDcc => "Kernel DCC".to_owned(),
            PaneType::InferenceLab => "Inference Lab".to_owned(),
            PaneType::ModelRuntime => "Model Runtime".to_owned(),
            PaneType::Swarm => "Swarm".to_owned(),
            PaneType::Problems => "Problems".to_owned(),
            PaneType::Jobs => "Jobs".to_owned(),
            PaneType::Timeline => "Timeline".to_owned(),
            PaneType::UserManual => "User Manual".to_owned(),
            PaneType::CodeSymbol => "Code Symbol".to_owned(),
            PaneType::SourceControl => "Source Control".to_owned(),
            PaneType::MediaDownloader => "Media Downloader".to_owned(),
            PaneType::FontManager => "Font Manager".to_owned(),
            PaneType::FlightRecorder => "Flight Recorder".to_owned(),
            PaneType::VisualDebugger => "Visual Debugger".to_owned(),
            PaneType::LoomSearchV2 => "Loom Search".to_owned(),
            PaneType::FindInFiles => "Find in Files".to_owned(),
            PaneType::Placeholder(name) => name.clone(),
        }
    }

    /// The TAB label for this surface — the React `TAB_LABEL_BY_ID` mapping (`app/src/App.tsx`).
    /// Deliberately distinct from [`PaneType::label`]: tabs use the SHORT React tab labels
    /// (e.g. `Fonts`, `Journal`, `Atelier`) where the pane container uses the longer descriptive
    /// label (`Font Manager`, `Loom Daily Journal`, `Atelier Editor`). MT-007's tab bar renders the
    /// tab label; the pane AccessKit container keeps using `label()`. Returns a `&'static str` so the
    /// non-placeholder variants are zero-allocation.
    pub fn default_label(&self) -> &str {
        match self {
            PaneType::Workspace => "Workspace",
            PaneType::MediaDownloader => "Media Downloader",
            PaneType::FontManager => "Fonts",
            PaneType::FlightRecorder => "Flight Recorder",
            PaneType::KernelDcc => "Kernel DCC",
            PaneType::InferenceLab => "Inference Lab",
            PaneType::ModelRuntime => "Model Runtime",
            PaneType::Swarm => "Swarm",
            PaneType::Problems => "Problems",
            PaneType::Jobs => "Jobs",
            PaneType::Timeline => "Timeline",
            PaneType::UserManual => "User Manual",
            PaneType::CodeSymbol => "Code Symbol",
            PaneType::SourceControl => "Source Control",
            PaneType::LoomDailyJournal => "Journal",
            PaneType::LoomBlock => "Loom Block",
            PaneType::LoomWikiPage => "Wiki Page",
            PaneType::AtelierEditor => "Atelier",
            PaneType::VisualDebugger => "Visual Debugger",
            PaneType::LoomSearchV2 => "Loom Search",
            PaneType::FindInFiles => "Find in Files",
            PaneType::Placeholder(name) => name.as_str(),
        }
    }
}

/// Whether a pane is locked against edits. Ports the React `locked` boolean as an explicit enum so
/// the state cannot be accidentally truthy/falsy at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockState {
    Unlocked,
    Locked,
}

/// Whether a pane has unsaved changes. Added by this MT (no React equivalent yet) so later MTs can
/// gate close/pop-out on unsaved work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirtyState {
    Clean,
    Dirty,
}

/// Who last modified this pane record. Lets swarm/operator co-work attribute pane changes
/// (HBR-SWARM). `Agent(String)` carries the agent/session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneAuthority {
    Human,
    Agent(String),
    System,
}

/// One pane's authoritative record. `last_update` is in-memory only and excluded from serde so the
/// struct round-trips cleanly; persisted layout snapshots (MT-009) carry a separate
/// `chrono::DateTime<Utc>` field rather than conflating wall-clock time with `Instant`
/// (RISK-2 / CONTROL-2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneRecord {
    pub pane_id: PaneId,
    pub pane_type: PaneType,
    /// Workspace/project identifier this pane belongs to.
    pub project_id: String,
    /// The content this pane shows (document_id / canvas_id / block_id, etc.), when applicable.
    pub content_id: Option<String>,
    pub lock_state: LockState,
    pub dirty: DirtyState,
    pub authority: PaneAuthority,
    /// In-memory last-touch marker; never serialized (see struct docs).
    #[serde(skip, default = "PaneRecord::now")]
    pub last_update: std::time::Instant,
}

impl PaneRecord {
    fn now() -> std::time::Instant {
        std::time::Instant::now()
    }

    /// Build a pane record with `last_update` set to now. Keeps the common construction path from
    /// repeating the `Instant::now()` boilerplate.
    pub fn new(
        pane_id: PaneId,
        pane_type: PaneType,
        project_id: impl Into<String>,
        content_id: Option<String>,
        lock_state: LockState,
        dirty: DirtyState,
        authority: PaneAuthority,
    ) -> Self {
        Self {
            pane_id,
            pane_type,
            project_id: project_id.into(),
            content_id,
            lock_state,
            dirty,
            authority,
            last_update: Self::now(),
        }
    }
}

/// Central registry of every pane plus its stable AccessKit node id. Iteration order is stable
/// (alphabetical by `pane_id`) because the backing map is a `BTreeMap`.
#[derive(Debug)]
pub struct PaneRegistry {
    records: BTreeMap<PaneId, PaneRecord>,
    /// AccessKit node ids keyed by pane id, for out-of-process model visibility/steering.
    accesskit_ids: BTreeMap<PaneId, accesskit::NodeId>,
    /// Monotonic source of AccessKit node ids. Starts at 100 so pane ids never collide with the
    /// low fixed ids hand-assigned to chrome widgets (e.g. the theme toggle at id 10 in `app.rs`).
    /// Incremented on every `insert`; never reuses a value within a registry's lifetime.
    next_accesskit_id: u64,
}

impl PaneRegistry {
    /// AccessKit node id allocation starts here so pane ids stay clear of the small hand-assigned
    /// chrome ids (theme toggle = 10).
    const ACCESSKIT_ID_BASE: u64 = 100;

    pub fn new() -> Self {
        Self {
            records: BTreeMap::new(),
            accesskit_ids: BTreeMap::new(),
            next_accesskit_id: Self::ACCESSKIT_ID_BASE,
        }
    }
}

impl Default for PaneRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneRegistry {

    /// Insert (or replace) a pane record and assign it a fresh, stable AccessKit node id from the
    /// monotonic counter. Re-inserting an existing `pane_id` keeps the already-assigned node id so
    /// a model that targeted the pane does not lose its handle across an in-place update.
    pub fn insert(&mut self, record: PaneRecord) {
        let pane_id = record.pane_id.clone();
        if !self.accesskit_ids.contains_key(&pane_id) {
            let node_id = accesskit::NodeId(self.next_accesskit_id);
            self.next_accesskit_id += 1;
            self.accesskit_ids.insert(pane_id.clone(), node_id);
        }
        self.records.insert(pane_id, record);
    }

    pub fn get(&self, id: &PaneId) -> Option<&PaneRecord> {
        self.records.get(id)
    }

    pub fn get_mut(&mut self, id: &PaneId) -> Option<&mut PaneRecord> {
        self.records.get_mut(id)
    }

    /// Remove a pane and its AccessKit id. Returns the removed record if it existed.
    pub fn remove(&mut self, id: &PaneId) -> Option<PaneRecord> {
        self.accesskit_ids.remove(id);
        self.records.remove(id)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Iterate panes in stable alphabetical-by-id order.
    pub fn iter(&self) -> impl Iterator<Item = (&PaneId, &PaneRecord)> {
        self.records.iter()
    }

    /// Pane ids in stable alphabetical order.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        self.records.keys().cloned().collect()
    }

    /// Override the AccessKit node id for a pane. Normally `insert` assigns ids automatically; this
    /// exists for callers (and tests) that need to pin a specific node id.
    pub fn assign_accesskit_id(&mut self, pane_id: &PaneId, node_id: accesskit::NodeId) {
        self.accesskit_ids.insert(pane_id.clone(), node_id);
    }

    pub fn accesskit_id(&self, pane_id: &PaneId) -> Option<accesskit::NodeId> {
        self.accesskit_ids.get(pane_id).copied()
    }

    /// Build the AccessKit node for a pane: `Role::Group` (or the factory's role) carrying the
    /// pane's label and, critically, an `author_id` equal to the kebab-case `pane_id` so an
    /// out-of-process client can match the pane by a stable identifier (RISK-1 / CONTROL-1). The
    /// id is the registry-assigned monotonic `NodeId`.
    ///
    /// Returns `None` if the pane has no record or no assigned node id.
    pub fn build_accesskit_node(
        &self,
        pane_id: &PaneId,
        role: accesskit::Role,
    ) -> Option<(accesskit::NodeId, accesskit::Node)> {
        let record = self.records.get(pane_id)?;
        let node_id = self.accesskit_id(pane_id)?;
        let mut node = accesskit::Node::new(role);
        node.set_label(record.pane_type.label());
        // author_id is the stable, model-meaningful match key for out-of-process steering.
        node.set_author_id(pane_id.as_ref().to_owned());
        Some((node_id, node))
    }
}

/// WP-KERNEL-012 MT-062: the stable pane id under which the Outgoing Links pane
/// ([`crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel`]) docks in the shell. A
/// fixed kebab/dotted key (NOT a per-instance `pane-a`-style id) so the shell can `dock`/`show` the
/// pane and a swarm agent can address it deterministically (AC-007 / PT-005). The pane is a Loom
/// knowledge surface attached to the active block/document, so its record uses the existing
/// [`PaneType::LoomBlock`] variant — no new `PaneType` variant is forked (that would ripple through
/// every exhaustive `PaneType` match across the shell); the STABLE ADDRESS is this pane id string.
pub const OUTGOING_LINKS_PANE_ID: &str = "loom.outgoing_links";

/// Register the WP-KERNEL-012 MT-062 Outgoing Links pane into `registry` under the stable
/// [`OUTGOING_LINKS_PANE_ID`] (`"loom.outgoing_links"`), bound to the active `document_id` content (the
/// source of outgoing links) for `project_id`. Re-registering keeps the already-assigned AccessKit node
/// id (see [`PaneRegistry::insert`]), so an agent that targeted the pane does not lose its handle when
/// the active document changes. Returns the pane id for convenience.
///
/// This is the E3 registration the MT requires NOW; the host wiring of the pane's `on_open` closure to
/// the real MT-030 navigation bus + the live dock placement land at E11 (MT-069), like the other panes.
pub fn register_outgoing_links_pane(
    registry: &mut PaneRegistry,
    project_id: impl Into<String>,
    document_id: Option<String>,
) -> PaneId {
    let pane_id: PaneId = Arc::from(OUTGOING_LINKS_PANE_ID);
    registry.insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomBlock,
        project_id,
        document_id,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    pane_id
}

/// Context handed to a `PaneFactory::render`. Carries the egui id base for the pane and the project
/// id; a real `BackendClient` handle will be threaded through here once concrete surfaces are built
/// (MT-006+). Kept as an explicit struct now so adding the backend handle later is a field add, not
/// a signature break across every factory.
pub struct PaneRenderContext<'a> {
    /// The pane being rendered. Factories read its type/content/lock state.
    pub record: &'a PaneRecord,
    /// Stable egui id base for this pane, derived from its AccessKit node id, so any interactive
    /// child widgets a factory builds can derive their own stable ids from it.
    pub egui_id: egui::Id,
}

/// A renderer for one `PaneType`. The host widget owns no factories; it borrows `&dyn PaneFactory`
/// references at render time so factory ownership stays in `HandshakeApp`.
pub trait PaneFactory: Send + Sync {
    /// Which surface this factory renders. Used to route a record to its factory.
    fn pane_type(&self) -> PaneType;

    /// Render the pane body into `ui`. Must never panic on any valid record.
    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext);

    /// The AccessKit role for this pane's container node. Defaults to `Group`.
    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Group
    }
}

/// Fallback factory for every surface not yet built in this WP. Renders a centered label with the
/// pane type and id so the pane is visibly present (and never blank/panicking) until a real factory
/// replaces it. Registered for every `PaneType` variant at startup (RISK-3 / CONTROL-3).
#[derive(Debug, Default)]
pub struct PlaceholderPaneFactory {
    pane_type: PaneType,
}

impl Default for PaneType {
    fn default() -> Self {
        PaneType::Placeholder("placeholder".to_owned())
    }
}

impl PlaceholderPaneFactory {
    pub fn new(pane_type: PaneType) -> Self {
        Self { pane_type }
    }
}

impl PaneFactory for PlaceholderPaneFactory {
    fn pane_type(&self) -> PaneType {
        self.pane_type.clone()
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            ui.label(ctx.record.pane_type.label());
            ui.small(ctx.record.pane_id.as_ref());
        });
    }
}

/// Renders every registered pane through its matching factory. Stateless: it borrows the registry
/// and a factory lookup at `show` time and owns nothing, so it is safe to construct per frame.
pub struct PaneHostWidget;

impl PaneHostWidget {
    /// Render all panes (stable order). `factory_for` returns the factory to use for a given
    /// pane type; callers wire it to their `HashMap<PaneType, Box<dyn PaneFactory>>`, falling back
    /// to a placeholder factory for any type with no dedicated entry. Each pane is wrapped in an
    /// egui scope with a stable id derived from its AccessKit node id so child widget ids are
    /// stable across frames.
    pub fn show<'f, F>(ui: &mut egui::Ui, registry: &PaneRegistry, mut factory_for: F)
    where
        F: FnMut(&PaneType) -> &'f dyn PaneFactory,
    {
        Self::show_with_accesskit(ui, registry, &mut factory_for, |_, _, _, _, _| {});
    }

    /// Like [`show`](Self::show) but also invokes `emit_accesskit` once per pane so the caller can
    /// push a LIVE AccessKit node into the frame's accessibility tree (MT-025). The callback runs
    /// inside the pane's own egui scope, after the pane's stable `egui_id` has been registered in
    /// egui's interaction/parent map, so a node emitted at that id attaches under the correct
    /// accessibility parent rather than floating at the root.
    ///
    /// `emit_accesskit` receives:
    /// - the `egui::Context` (for `accesskit_node_builder`),
    /// - the pane's stable `egui::Id` (== `PaneRenderContext::egui_id`),
    /// - the pane's kebab-case `author_id` string,
    /// - the factory's `accesskit_role()`,
    /// - the pane's `PaneType::label()`.
    ///
    /// Splitting emission out as a callback keeps `pane_registry` free of any dependency on the
    /// `accessibility` module (the app wires them together), so the registry stays a pure data +
    /// layout surface.
    pub fn show_with_accesskit<'f, F, A>(
        ui: &mut egui::Ui,
        registry: &PaneRegistry,
        mut factory_for: F,
        mut emit_accesskit: A,
    ) where
        F: FnMut(&PaneType) -> &'f dyn PaneFactory,
        A: FnMut(&egui::Context, egui::Id, &str, accesskit::Role, &str),
    {
        for (pane_id, record) in registry.iter() {
            let node_id = registry
                .accesskit_id(pane_id)
                .map(|n| n.0)
                // A pane is always assigned a node id at insert; if a custom id override left it
                // unset, fall back to a hash of the pane id rather than panic in the render path.
                .unwrap_or_else(|| hash_pane_id(pane_id));
            // from_high_entropy_bits (NOT Id::new, which hashes its argument) so the LIVE AccessKit
            // NodeId equals the registry node_id (100..103) — the same convention chrome (10/20/21)
            // and the theme toggle (10) use. This makes the DECLARED_IDENTITIES collision space
            // reflect the REAL live tree rather than an unused 100..103 space. Safe for fixed ids:
            // a single fixed id per pane cannot self-collide; entropy only affects child IdMap
            // distribution. (Custom-override fallback ids from hash_pane_id are already high-entropy.)
            let egui_id = unsafe { egui::Id::from_high_entropy_bits(node_id) };
            let factory = factory_for(&record.pane_type);
            let role = factory.accesskit_role();
            let label = record.pane_type.label();
            let ctx = PaneRenderContext { record, egui_id };
            // Render the pane inside a scope so its body has a stable id namespace.
            let scope = ui.scope_builder(egui::UiBuilder::new().id_salt(node_id), |ui| {
                factory.render(ui, &ctx);
                // Register the pane's stable egui_id in the interaction/parent map on the pane's
                // content rect, so the live AccessKit node attaches under this scope rather than the
                // tree root. Sense::hover() keeps the pane container non-interactive (a model
                // steers child widgets, not the container itself).
                let rect = ui.min_rect();
                ui.interact(rect, egui_id, egui::Sense::hover());
            });
            // Emit the live node now that egui_id is in the parent map for this frame.
            emit_accesskit(ui.ctx(), egui_id, pane_id.as_ref(), role, &label);
            let _ = scope;
        }
    }
}

/// Stable fallback id derived from the pane id string. Only used if a pane somehow lacks a
/// registry-assigned AccessKit id; deterministic so it is still stable across frames.
fn hash_pane_id(pane_id: &PaneId) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pane_id.as_ref().hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four default panes from the contract (mirrors the React DEFAULT_PANES four-pane shape).
    fn default_records() -> Vec<PaneRecord> {
        vec![
            PaneRecord::new(
                Arc::from("pane-a"),
                PaneType::Workspace,
                "project-1",
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ),
            PaneRecord::new(
                Arc::from("pane-b"),
                PaneType::InferenceLab,
                "project-1",
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ),
            PaneRecord::new(
                Arc::from("pane-c"),
                PaneType::MediaDownloader,
                "project-1",
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ),
            PaneRecord::new(
                Arc::from("pane-d"),
                PaneType::FontManager,
                "project-1",
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ),
        ]
    }

    fn seeded_registry() -> PaneRegistry {
        let mut reg = PaneRegistry::new();
        for r in default_records() {
            reg.insert(r);
        }
        reg
    }

    #[test]
    fn inserts_and_retrieves_four_default_panes() {
        let reg = seeded_registry();
        assert_eq!(reg.len(), 4);

        let a: PaneId = Arc::from("pane-a");
        let rec = reg.get(&a).expect("pane-a present");
        assert_eq!(rec.pane_type, PaneType::Workspace);
        assert_eq!(rec.lock_state, LockState::Unlocked);
        assert_eq!(rec.dirty, DirtyState::Clean);
        assert_eq!(rec.authority, PaneAuthority::System);
        assert_eq!(rec.project_id, "project-1");

        for id in ["pane-a", "pane-b", "pane-c", "pane-d"] {
            let pid: PaneId = Arc::from(id);
            assert!(reg.get(&pid).is_some(), "{id} retrievable");
            let node = reg.accesskit_id(&pid).expect("accesskit id assigned");
            assert_ne!(node.0, 0, "{id} accesskit id is non-zero");
        }
    }

    #[test]
    fn accesskit_ids_are_unique_and_follow_monotonic_counter_not_frame_counter() {
        let reg = seeded_registry();
        // Stable order: pane-a..pane-d -> ids 100..103 from the monotonic base.
        let ids: Vec<u64> = reg
            .pane_ids()
            .iter()
            .map(|p| reg.accesskit_id(p).unwrap().0)
            .collect();
        assert_eq!(ids, vec![100, 101, 102, 103], "ids follow the monotonic counter, not frame counters");

        // Uniqueness.
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "all accesskit ids unique");
    }

    #[test]
    fn iter_returns_stable_btreemap_order() {
        let mut reg = PaneRegistry::new();
        // Insert out of order; iteration must still be alphabetical.
        for id in ["pane-d", "pane-b", "pane-a", "pane-c"] {
            reg.insert(PaneRecord::new(
                Arc::from(id),
                PaneType::Workspace,
                "p",
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ));
        }
        let order: Vec<String> = reg.iter().map(|(id, _)| id.as_ref().to_owned()).collect();
        assert_eq!(order, vec!["pane-a", "pane-b", "pane-c", "pane-d"]);
    }

    #[test]
    fn reinsert_keeps_stable_accesskit_id() {
        let mut reg = seeded_registry();
        let a: PaneId = Arc::from("pane-a");
        let before = reg.accesskit_id(&a).unwrap();
        // In-place update of the same pane id (e.g. an agent dirties it).
        reg.insert(PaneRecord::new(
            a.clone(),
            PaneType::Workspace,
            "project-1",
            Some("doc-99".to_owned()),
            LockState::Locked,
            DirtyState::Dirty,
            PaneAuthority::Agent("agent-7".to_owned()),
        ));
        let after = reg.accesskit_id(&a).unwrap();
        assert_eq!(before, after, "node id stable across in-place update");
        assert_eq!(reg.get(&a).unwrap().dirty, DirtyState::Dirty);
    }

    #[test]
    fn remove_drops_record_and_accesskit_id() {
        let mut reg = seeded_registry();
        let a: PaneId = Arc::from("pane-a");
        assert!(reg.remove(&a).is_some());
        assert!(reg.get(&a).is_none());
        assert!(reg.accesskit_id(&a).is_none());
        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn pane_record_round_trips_through_serde_except_last_update() {
        let rec = PaneRecord::new(
            Arc::from("pane-a"),
            PaneType::Placeholder("future-surface".to_owned()),
            "project-1",
            Some("doc-1".to_owned()),
            LockState::Locked,
            DirtyState::Dirty,
            PaneAuthority::Agent("agent-3".to_owned()),
        );
        let json = serde_json::to_string(&rec).expect("serialize");
        // last_update must not appear in the serialized form.
        assert!(!json.contains("last_update"), "last_update excluded from serde");
        let back: PaneRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.pane_id, rec.pane_id);
        assert_eq!(back.pane_type, rec.pane_type);
        assert_eq!(back.project_id, rec.project_id);
        assert_eq!(back.content_id, rec.content_id);
        assert_eq!(back.lock_state, rec.lock_state);
        assert_eq!(back.dirty, rec.dirty);
        assert_eq!(back.authority, rec.authority);
    }

    #[test]
    fn accesskit_node_author_id_equals_pane_id_string() {
        let reg = seeded_registry();
        for id in ["pane-a", "pane-b", "pane-c", "pane-d"] {
            let pid: PaneId = Arc::from(id);
            let (node_id, node) = reg
                .build_accesskit_node(&pid, accesskit::Role::Group)
                .expect("node built");
            assert_eq!(node_id, reg.accesskit_id(&pid).unwrap());
            assert_eq!(node.author_id(), Some(id), "author_id equals pane id string");
            assert_eq!(node.role(), accesskit::Role::Group);
        }
    }

    #[test]
    fn placeholder_factory_renders_every_pane_type_without_panic() {
        // Exercise every variant, including Placeholder, through the placeholder factory.
        let variants = vec![
            PaneType::Workspace,
            PaneType::LoomDailyJournal,
            PaneType::LoomBlock,
            PaneType::LoomWikiPage,
            PaneType::AtelierEditor,
            PaneType::KernelDcc,
            PaneType::InferenceLab,
            PaneType::ModelRuntime,
            PaneType::Swarm,
            PaneType::Problems,
            PaneType::Jobs,
            PaneType::Timeline,
            PaneType::UserManual,
            PaneType::CodeSymbol,
            PaneType::SourceControl,
            PaneType::MediaDownloader,
            PaneType::FontManager,
            PaneType::FlightRecorder,
            PaneType::VisualDebugger,
            PaneType::Placeholder("unknown-surface".to_owned()),
        ];
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for (i, variant) in variants.iter().enumerate() {
                    let factory = PlaceholderPaneFactory::new(variant.clone());
                    let record = PaneRecord::new(
                        Arc::from(format!("pane-{i}").as_str()),
                        variant.clone(),
                        "p",
                        None,
                        LockState::Unlocked,
                        DirtyState::Clean,
                        PaneAuthority::System,
                    );
                    let render_ctx = PaneRenderContext {
                        record: &record,
                        egui_id: egui::Id::new(i as u64),
                    };
                    ui.push_id(i as u64, |ui| {
                        factory.render(ui, &render_ctx);
                    });
                }
            });
        });
    }

    #[test]
    fn host_widget_renders_all_panes_via_factory() {
        let reg = seeded_registry();
        let factory = PlaceholderPaneFactory::new(PaneType::Workspace);
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Every type routes to the same placeholder factory for this test.
                PaneHostWidget::show(ui, &reg, |_t| &factory as &dyn PaneFactory);
            });
        });
    }

    /// WP-KERNEL-012 MT-062 (PT-005 / AC-007 pane half): the Outgoing Links pane registers under the
    /// stable `loom.outgoing_links` id, is retrievable by that id, carries a stable AccessKit node id,
    /// and binds the active document as its content. Re-registering keeps the same AccessKit id.
    #[test]
    fn registers_outgoing_links_pane_under_stable_id() {
        let mut reg = PaneRegistry::new();
        let id = register_outgoing_links_pane(&mut reg, "project-1", Some("DOC-active".to_owned()));
        assert_eq!(id.as_ref(), OUTGOING_LINKS_PANE_ID);
        assert_eq!(OUTGOING_LINKS_PANE_ID, "loom.outgoing_links");

        let pid: PaneId = Arc::from(OUTGOING_LINKS_PANE_ID);
        let rec = reg.get(&pid).expect("loom.outgoing_links pane registered");
        assert_eq!(rec.content_id.as_deref(), Some("DOC-active"), "active document bound as pane content");
        let node = reg.accesskit_id(&pid).expect("accesskit id assigned");
        assert!(node.0 >= PaneRegistry::ACCESSKIT_ID_BASE, "pane id sits in the pane id space (>= 100)");

        // The pane's AccessKit node carries the stable pane id as its author_id (the swarm address).
        let (_nid, akn) = reg
            .build_accesskit_node(&pid, accesskit::Role::Group)
            .expect("node built");
        assert_eq!(akn.author_id(), Some("loom.outgoing_links"), "author_id equals the stable pane id");

        // Re-register on a document change: the AccessKit id is stable across the in-place update.
        let before = reg.accesskit_id(&pid).unwrap();
        register_outgoing_links_pane(&mut reg, "project-1", Some("DOC-other".to_owned()));
        assert_eq!(reg.accesskit_id(&pid).unwrap(), before, "AccessKit id stable across re-register");
        assert_eq!(reg.get(&pid).unwrap().content_id.as_deref(), Some("DOC-other"));
    }
}
