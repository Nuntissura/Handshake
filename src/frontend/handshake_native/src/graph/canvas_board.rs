//! Loom canvas board (WP-KERNEL-012 MT-026, cluster E3) — the native Obsidian-Canvas-class surface.
//!
//! ## What this is
//!
//! [`LoomCanvasBoard`] is the native peer of the React `app/src/components/LoomCanvasBoard.tsx`
//! (MT-261 parity). The board IS a typed `LoomBlock(content_type=canvas)`; placed items are block-id
//! REFERENCES rendered as LIVE previews (proving reference-not-copy — a placement shows the live block
//! title fetched once on load, never a copied content string, and a missing block shows
//! `(stale reference)`). Semantic connections become real `loom_edges`; visual-only connections are
//! board-local decoration. Pan/zoom is hand-rolled with [`egui::Painter`] (no new canvas library), with
//! ONE canvas-to-screen / screen-to-canvas transform pair used by BOTH the draw pass and the hit-test
//! pass (RISK-1 / MC-1).
//!
//! Backend authority is PostgreSQL + EventLedger; this widget is a projection. Every mutating action is
//! a typed [`CanvasEvent`] the host applies through [`crate::backend_client::CanvasBoardClient`]
//! (off the UI thread), then re-fetches the board + re-resolves live titles — never a per-frame fetch.
//!
//! ## Backend reality (verified read-only — the MT-022/023/024 lesson)
//!
//! The MT contract's `binds_backend_api` URLs were STALE (the `.../loom/canvas/{cb}/...` shape does not
//! exist). Verified against `src/backend/handshake_core/src/api/loom.rs` + `storage/loom.rs`, the REAL
//! routes the host drives through [`crate::backend_client::CanvasBoardClient`] are:
//!   - `GET    /workspaces/:ws/loom/canvas-boards/:block_id`            getCanvasBoard -> LoomCanvasBoardView
//!   - `PUT    /workspaces/:ws/loom/canvas-boards/:block_id/viewport`   updateViewport  body `{board_state:{pan_x,pan_y,zoom,schema_id}}`
//!   - `POST   /workspaces/:ws/loom/canvas-boards/:block_id/placements` placeBlock      body `{placed_block_id,x,y,w,h}`
//!   - `POST   /workspaces/:ws/loom/canvas-boards/:block_id/cards`      createCard      body `{title,body,x,y,w,h}`
//!   - `PATCH  /workspaces/:ws/loom/canvas-placements/:placement_id`    updatePlacement body `{group_id}` (NOT `.../canvas/{cb}/placements/{p}`)
//!   - `DELETE /workspaces/:ws/loom/canvas-placements/:placement_id`    removePlacement
//!   - `POST   /workspaces/:ws/loom/edges`                              createLoomEdge  body `{source_block_id,target_block_id,edge_type:"mention",created_by:"user"}`
//!   - `POST   /workspaces/:ws/loom/canvas-boards/:block_id/visual-edges` addVisualEdge body `{from_placement_id,to_placement_id}`
//!   - `GET    /workspaces/:ws/loom/blocks/:block_id`                   getLoomBlock -> LoomBlock (live title)
//!
//! Placement `x/y/w/h` are `f64` on the wire; the widget keeps them as `f32` for egui math and the host
//! round-trips through the f64 client. The board viewport persists under `board_state.{pan_x,pan_y,zoom}`
//! (NOT a top-level `{pan_x,pan_y,zoom}` body — that was the contract's stale shape).
//!
//! ## Repaint discipline (the MT-015 idle-repaint lesson)
//!
//! The board NEVER requests a perpetual repaint. The pan-drag and loading indicator animate only while
//! a genuine interaction or in-flight fetch is happening; a headless / no-runtime render is neutral and
//! non-animating. Viewport persistence fires on pan/zoom RELEASE (the host applies the typed
//! [`CanvasEvent::ViewportChanged`]), never every frame (RISK-3 / MC-3 — the host debounces).
//!
//! ## AccessKit (HBR-SWARM)
//!
//! Every toolbar control (`canvas.pan-left`, `canvas.pan-right`, `canvas.zoom-in`, `canvas.zoom-out`,
//! `canvas.zoom-value`, `canvas.add-card`, `canvas.group`, `canvas.edge-mode`, `canvas.start-edge`),
//! the status bar (`canvas.status`, Role::Status), each placement card
//! (`canvas.placement.{placement_id}`, Role::Group, with the `group_id` exposed as a description) and
//! its remove button (`canvas.placement.{placement_id}.remove`) emits a live AccessKit node through
//! egui's own [`egui::Context::accesskit_node_builder`] hook so an out-of-process swarm agent can read
//! the board and drive it by stable id. Placement ids are sanitized to `[a-z0-9-]` via
//! [`crate::project_tree::stable_part`] before forming the author_id suffix (RISK / id-integrity).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use egui::accesskit;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::accessibility::knowledge_action_registry::{
    self, AddEdgePayload, AxRole as KAxRole, EdgeIdPayload, KnowledgeActionRegistry,
    KnowledgeNodeState, PlaceBlockPayload, PlacementIdPayload, CANVAS_CONTROL_CATALOG,
};
use crate::theme::HsPalette;

/// Default card dimensions on a drop / new card (React `DEFAULT_CARD_W` / `DEFAULT_CARD_H`).
pub const DEFAULT_CARD_W: f32 = 200.0;
pub const DEFAULT_CARD_H: f32 = 120.0;

/// MIME the inter-panel drag payload carries (React `CANVAS_DRAG_MIME`). The drag payload is JSON
/// `{blockId, title?}`; the host accepts a drop with this MIME and computes the canvas-space position.
pub const CANVAS_DRAG_MIME: &str = "application/x-handshake-loom-block";

/// Pan step (px) for the pan-left/right buttons (React `± 40`).
pub const PAN_STEP: f32 = 40.0;

/// Zoom step for the zoom-in/out buttons (React `± 0.25`).
pub const ZOOM_STEP: f32 = 0.25;

/// Zoom clamp (React `[0.25, 4.0]`).
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 4.0;

/// Dotted background grid spacing (px, in canvas space) and dot radius.
const GRID_STEP: f32 = 24.0;
const GRID_DOT_RADIUS: f32 = 1.0;

/// A short visual edge (< this many px) is drawn as a single solid line, not segmented (RISK-5 / MC-5).
const MIN_DASH_LEN: f32 = 10.0;
/// Dash / gap lengths for the manual dashed visual edge (React stroke pattern 6 / 4).
const DASH_SOLID: f32 = 6.0;
const DASH_GAP: f32 = 4.0;

// ── Toolbar / status AccessKit author_ids (stable strings) ───────────────────────────────────────
pub const PAN_LEFT_AUTHOR_ID: &str = "canvas.pan-left";
pub const PAN_RIGHT_AUTHOR_ID: &str = "canvas.pan-right";
pub const ZOOM_OUT_AUTHOR_ID: &str = "canvas.zoom-out";
pub const ZOOM_IN_AUTHOR_ID: &str = "canvas.zoom-in";
pub const ZOOM_VALUE_AUTHOR_ID: &str = "canvas.zoom-value";
pub const ADD_CARD_AUTHOR_ID: &str = "canvas.add-card";
pub const GROUP_AUTHOR_ID: &str = "canvas.group";
pub const EDGE_MODE_AUTHOR_ID: &str = "canvas.edge-mode";
pub const START_EDGE_AUTHOR_ID: &str = "canvas.start-edge";
pub const STATUS_AUTHOR_ID: &str = "canvas.status";
/// MC-2 fallback: the block-id text field and the `Place` button that place a reference without OS drag.
pub const PLACE_BLOCK_INPUT_AUTHOR_ID: &str = "canvas.place-block-input";
pub const PLACE_BLOCK_AUTHOR_ID: &str = "canvas.place-block";

/// Author_id prefix for a placement card. The full id is `canvas.placement.{sanitized_placement_id}`.
pub const PLACEMENT_AUTHOR_ID_PREFIX: &str = "canvas.placement.";

/// Author_id SUFFIX for a placement card's bottom-right resize handle (WP-KERNEL-012 MT-061). The full
/// id is `canvas.placement.{sanitized_placement_id}.resize`.
pub const RESIZE_HANDLE_AUTHOR_ID_SUFFIX: &str = ".resize";

/// Minimum card width in CANVAS units (WP-KERNEL-012 MT-061 / IN: clamp to a sensible minimum so a resize
/// can never collapse a card to an unusable size).
pub const MIN_CARD_W: f32 = 80.0;
/// Minimum card height in CANVAS units (WP-KERNEL-012 MT-061).
pub const MIN_CARD_H: f32 = 48.0;

/// The on-screen size (logical px BEFORE zoom) of the bottom-right resize grab handle (MT-061).
const RESIZE_HANDLE_PX: f32 = 12.0;

/// The stable AccessKit author_id for a placement card, sanitizing `placement_id` to `[a-z0-9-]` so a
/// raw id with slashes/colons can never break tree integrity (reuses the shell's slugger).
pub fn placement_author_id(placement_id: &str) -> String {
    format!("{PLACEMENT_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(placement_id))
}

/// The stable AccessKit author_id for a placement card's remove button.
pub fn placement_remove_author_id(placement_id: &str) -> String {
    format!("{}.remove", placement_author_id(placement_id))
}

/// The stable AccessKit author_id for a placement card's resize handle (WP-KERNEL-012 MT-061):
/// `canvas.placement.{sanitized_placement_id}.resize`. Extends (does not replace) the MT-026 card ids.
pub fn placement_resize_author_id(placement_id: &str) -> String {
    format!("{}{}", placement_author_id(placement_id), RESIZE_HANDLE_AUTHOR_ID_SUFFIX)
}

/// What backs a placement card, deciding whether it is INLINE-EDITABLE on the canvas (WP-KERNEL-012
/// MT-061, the load-bearing reference-not-copy gate, AC-061-5).
///
/// - [`CanvasCardKind::TextCard`]: a FREE-TEXT card created via the cards endpoint (a `note` LoomBlock the
///   canvas owns the editing surface for). Double-click enters in-place editing; committing persists
///   title/body. Editing a text card mutates only THAT card's own content.
/// - [`CanvasCardKind::BlockRef`]: a placed REFERENCE to an existing Loom block (the MT-026 default).
///   NEVER inline-editable — double-clicking navigates to the block. This preserves reference-not-copy:
///   a canvas edit can never fork or copy the underlying block.
///
/// The host sets `TextCard` for placements it created through `createCard` (it holds the
/// `CreateCanvasCardResponse`); every other placement defaults to `BlockRef`. The widget NEVER guesses
/// from content_type — the kind is an explicit, testable flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CanvasCardKind {
    /// A free-text note card the canvas can inline-edit (its content is the card's own RichDocument).
    TextCard,
    /// A reference to an existing Loom block — navigate on double-click, never inline-edit (default).
    #[default]
    BlockRef,
}

impl CanvasCardKind {
    /// `true` only for a free-text card (the inline editor gate — AC-061-5 reference-not-copy).
    pub fn is_text_card(self) -> bool {
        matches!(self, CanvasCardKind::TextCard)
    }
}

/// Which kind of edge `Draw edge` creates. `Semantic` calls `createLoomEdge` (a real, graph-authority
/// `loom_edge`); `Visual` calls `addCanvasVisualEdge` (board-local decoration, never graph authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeMode {
    Semantic,
    Visual,
}

impl EdgeMode {
    /// The label shown on the edge-mode toggle.
    pub fn label(self) -> &'static str {
        match self {
            EdgeMode::Semantic => "Semantic",
            EdgeMode::Visual => "Visual",
        }
    }

    /// Toggle to the other mode.
    fn toggled(self) -> Self {
        match self {
            EdgeMode::Semantic => EdgeMode::Visual,
            EdgeMode::Visual => EdgeMode::Semantic,
        }
    }
}

/// One placement card rendered by the board: a block-id REFERENCE positioned on the canvas with its
/// resolved-once live title + content_type (NEVER copied content). `live_title == None` means the
/// referenced block could not be resolved -> the card shows `(stale reference)`.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasPlacementCard {
    pub placement_id: String,
    pub placed_block_id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub z_index: i32,
    pub group_id: Option<String>,
    /// Live block title resolved ONCE on load via `getLoomBlock` (reference, not copy). `None` => the
    /// block is missing -> "(stale reference)".
    pub live_title: Option<String>,
    /// Live block content_type (muted subtitle). `None` when unresolved.
    pub live_content_type: Option<String>,
    /// WP-KERNEL-012 MT-032 (E5 "everything is a Loom block"): the backend-computed content hash read
    /// from the resolved `LoomBlock` (getLoomBlock carries `content_hash`). `None` when unresolved or
    /// the backend block had no hash. Shown as a short suffix on the `loom://` chip; READ-ONLY (the
    /// canvas never writes a hash — the backend computes it).
    pub loom_content_hash: Option<String>,
    /// WP-KERNEL-012 MT-061: what backs this card, deciding inline-editability (reference-not-copy gate,
    /// AC-061-5). Defaults to [`CanvasCardKind::BlockRef`] (the MT-026 reference semantics). The host sets
    /// [`CanvasCardKind::TextCard`] for free-text cards it created via `createCard`.
    pub card_kind: CanvasCardKind,
    /// WP-KERNEL-012 MT-061: the free-text card's body (markdown), resolved on load for a [`TextCard`].
    /// Seeds the in-place editor's buffer so a double-click opens the card with its existing content.
    /// `None`/empty for a block reference (which is never inline-edited). The board never copies a block's
    /// content here — this is only ever a text card's OWN body.
    ///
    /// [`TextCard`]: CanvasCardKind::TextCard
    pub live_body: Option<String>,
}

impl CanvasPlacementCard {
    /// A placement with no resolved live block yet (the host fills `live_title`/`live_content_type`
    /// after the `getLoomBlock` resolve cycle).
    pub fn new(
        placement_id: impl Into<String>,
        placed_block_id: impl Into<String>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Self {
        Self {
            placement_id: placement_id.into(),
            placed_block_id: placed_block_id.into(),
            x,
            y,
            w,
            h,
            z_index: 0,
            group_id: None,
            live_title: None,
            live_content_type: None,
            loom_content_hash: None,
            card_kind: CanvasCardKind::BlockRef,
            live_body: None,
        }
    }

    /// Mark this placement as a free-text card (the inline-editable kind), seeding its editor body. The
    /// host calls this for a placement it created via `createCard` so a double-click opens the in-place
    /// editor (AC-061-4). A block reference is left as the default [`CanvasCardKind::BlockRef`].
    pub fn as_text_card(mut self, body: impl Into<String>) -> Self {
        self.card_kind = CanvasCardKind::TextCard;
        self.live_body = Some(body.into());
        self
    }

    /// The `loom://{workspace_id}/{placed_block_id}` address of the block this card references, when
    /// `placed_block_id` is non-empty (MT-032). `None` for a placement with no block id (RISK-3: the
    /// chip is then skipped — no panic, no fabricated `loom://` URI). `workspace_id` is the board's.
    pub fn loom_addr(&self, workspace_id: &str) -> Option<crate::loom_address::LoomBlockAddr> {
        let addr = crate::loom_address::LoomBlockAddr::new(workspace_id, &self.placed_block_id);
        addr.is_addressable().then_some(addr)
    }

    /// The display title: the live block title, or `(stale reference)` when the referenced block could
    /// not be resolved (AC1 / AC4 — never a copied content string).
    pub fn display_title(&self) -> &str {
        match &self.live_title {
            Some(t) if !t.trim().is_empty() => t.as_str(),
            _ => "(stale reference)",
        }
    }

    /// This card's rect in CANVAS space (`[x, y, x+w, y+h]`).
    fn canvas_rect(&self) -> Rect {
        Rect::from_min_size(Pos2::new(self.x, self.y), Vec2::new(self.w, self.h))
    }

    /// This card's centre in CANVAS space (visual-edge endpoint).
    fn canvas_center(&self) -> Pos2 {
        Pos2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

/// A visual-only edge between two placements (board decoration; NOT a `loom_edge`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualEdge {
    pub visual_edge_id: String,
    pub from_placement_id: String,
    pub to_placement_id: String,
}

/// The typed inter-panel drag payload a Loom block source (folder tree, graph view, search result)
/// hands to the canvas via egui's [`egui::DragAndDrop`] channel. It carries the block id (and an
/// optional title hint) under the logical MIME [`CANVAS_DRAG_MIME`] — the React reference passes the
/// equivalent JSON `{blockId, title?}` through `dataTransfer.getData(CANVAS_DRAG_MIME)`
/// (`LoomCanvasBoard.tsx` `onDrop`). The canvas reads it with
/// [`egui::Response::dnd_release_payload`] when a drag is released over the surface, computes the
/// canvas-space drop position with [`LoomCanvasBoard::screen_to_canvas`], and emits
/// [`CanvasEvent::PlaceBlock`]. Must be `Send + Sync + 'static` for the egui DragAndDrop store
/// (compile-gated by `canvas_drag_payload_is_send_sync_static`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasDragPayload {
    /// The Loom block id to place as a REFERENCE (never a copy).
    pub block_id: String,
    /// An optional title hint from the drag source (the live title is still re-resolved on refresh).
    pub title: Option<String>,
}

impl CanvasDragPayload {
    /// A payload carrying just the block id (the common inter-panel case).
    pub fn new(block_id: impl Into<String>) -> Self {
        Self { block_id: block_id.into(), title: None }
    }
}

/// The typed event a board interaction produces this frame, for the host to apply through the backend
/// client and then re-fetch. Every variant maps 1:1 to a verified backend route; the widget itself
/// performs NO network IO (HBR-QUIET — the host spawns the request off the UI thread).
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasEvent {
    /// Persist the viewport (`PUT .../viewport` with `board_state.{pan_x,pan_y,zoom}`). Fired on pan/zoom
    /// RELEASE only (RISK-3 / MC-3), never per frame.
    ViewportChanged { pan_x: f32, pan_y: f32, zoom: f32 },
    /// Place a dropped block as a reference (`POST .../placements`). `x`/`y` are in CANVAS space.
    PlaceBlock { placed_block_id: String, x: f32, y: f32 },
    /// Create a free-text note card (`POST .../cards`); `title` is the React timestamp title.
    AddCard { title: String, x: f32, y: f32 },
    /// Group the given placements under a new `group_id` (`PATCH .../canvas-placements/:id {group_id}`).
    Group { placement_ids: Vec<String>, group_id: String },
    /// WP-KERNEL-012 MT-061: persist a card's resized geometry (`PATCH .../canvas-placements/:id {w,h}`).
    /// Fired ONCE per resize gesture, on drag-STOP only (debounced — RISK-061-1 / MC-061-1); the in-flight
    /// size renders optimistically during the drag and reconciles via `getCanvasBoard` after the PATCH.
    /// `w`/`h` are in CANVAS units, already clamped to [`MIN_CARD_W`]x[`MIN_CARD_H`].
    ResizePlacement { placement_id: String, w: f32, h: f32 },
    /// WP-KERNEL-012 MT-061: assign (or clear) a card's section/group via the SAME placement PATCH route
    /// (`PATCH .../canvas-placements/:id {group_id}` / `{clear_group:true}`). `group_id == Some(id)`
    /// assigns the dropped card to that section; `group_id == None` CLEARS the assignment (drop outside
    /// all frames). Mutates ONLY the placement record — never the underlying block (reference-not-copy).
    AssignSection { placement_id: String, group_id: Option<String> },
    /// WP-KERNEL-012 MT-061: persist an in-place edit of a free-text card's title/body. The host applies
    /// it through the existing knowledge-document save path for the card's backing note block (the cards
    /// surface), then re-fetches via `getCanvasBoard`. Emitted ONLY for a [`CanvasCardKind::TextCard`]
    /// (the inline editor is gated to text cards — AC-061-4/AC-061-5). `block_id` is the card's backing
    /// note block so the host knows which document to save.
    EditTextCard { placement_id: String, block_id: String, title: String, body: String },
    /// Remove a placement reference (`DELETE .../canvas-placements/:id`). Source block is KEPT.
    RemovePlacement { placement_id: String },
    /// Create a real semantic Loom edge (`POST /loom/edges {edge_type:"mention"}`) between two BLOCKS.
    SemanticEdge { source_block_id: String, target_block_id: String },
    /// Create a board-local visual edge (`POST .../visual-edges`) between two PLACEMENTS.
    VisualEdgeAdded { from_placement_id: String, to_placement_id: String },
    /// WP-KERNEL-012 MT-042: remove an edge by id — a swarm `canvas.remove-edge` dispatch. The host
    /// routes it through the E6 loom client (semantic edge) or removes the board-local visual edge.
    RemoveEdge { edge_id: String },
}

/// The board widget state. Held by the host pane, mutated in place by [`LoomCanvasBoard::show`]. Pan,
/// zoom, selection, and edge_from are ephemeral UI state; placements + visual_edges are the projection
/// of authoritative backend state the host loads via `getCanvasBoard`.
#[derive(Debug, Clone)]
pub struct LoomCanvasBoard {
    pub workspace_id: String,
    pub canvas_block_id: String,
    pub placements: Vec<CanvasPlacementCard>,
    pub visual_edges: Vec<VisualEdge>,
    pub pan: Vec2,
    pub zoom: f32,
    pub selected: HashSet<String>,
    pub edge_mode: EdgeMode,
    /// The placement a `Draw edge from selected` started from; the next card click completes the edge.
    pub edge_from: Option<String>,
    pub status: String,
    pub loading: bool,
    pub error: Option<String>,
    /// MC-2 fallback input: a block id typed/pasted into the toolbar text field for backends where
    /// OS / inter-panel drag is unavailable. The `Place` button emits the SAME
    /// [`CanvasEvent::PlaceBlock`] the drop path produces, so the place behavior is always reachable.
    pub place_block_input: String,
    /// WP-KERNEL-012 MT-061: the placement id + in-flight CANVAS-space size of an ACTIVE resize gesture.
    /// `Some((id, w, h))` while a resize handle is being dragged; the card renders at this optimistic
    /// size every frame (immediate visual feedback). Cleared on drag-stop after the single
    /// [`CanvasEvent::ResizePlacement`] fires. Never persisted per-frame (debounce — RISK-061-1).
    resizing: Option<(String, f32, f32)>,
    /// WP-KERNEL-012 MT-061: the placement id of a card being MOVED by dragging its body. `Some(id)` while
    /// a card-move drag is active; on drag-stop the drop position resolves a section assignment via
    /// [`crate::graph::canvas_sections::SectionLayer::which_section`]. Cleared on drag-stop.
    moving: Option<String>,
    /// WP-KERNEL-012 MT-061: the in-flight CANVAS-space top-left of the card being moved, so the card
    /// renders at the optimistic drag position and the drop point is exact. Paired with [`Self::moving`].
    moving_pos: Option<Pos2>,
    /// WP-KERNEL-012 MT-061: the placement id of the free-text card currently in IN-PLACE edit mode, or
    /// `None`. Only ONE card edits at a time. Set on double-click of a [`CanvasCardKind::TextCard`].
    editing_card_id: Option<String>,
    /// WP-KERNEL-012 MT-061: the live edit buffer for the inline text-card editor (the multiline
    /// `TextEdit`'s backing string). Seeded from the card's `live_body` on entering edit mode; discarded
    /// on Escape; committed (-> [`CanvasEvent::EditTextCard`]) on focus-loss / Ctrl+Enter.
    editing_buffer: String,
    /// WP-KERNEL-012 MT-061: optional per-`group_id` section labels from the board's section/group
    /// metadata. Used by the derived [`crate::graph::canvas_sections::SectionLayer`] for frame titles;
    /// absent entries fall back to the `group_id` string. Set by the host via [`Self::set_section_labels`]
    /// alongside `set_board`.
    section_labels: std::collections::BTreeMap<String, String>,
    /// The last canvas surface rect (screen space), recorded each frame so the MC-2 fallback can place
    /// a card at the centre of the currently-visible canvas. `None` until the board has rendered once.
    last_canvas_rect: Option<Rect>,
    /// Group-id counter so the `Group` event always gets a unique id even within one process run.
    group_seq: u64,
    /// WP-KERNEL-012 MT-042 (E7): the shared knowledge AccessKit action registry. `None` until the host
    /// installs it. An `Arc` handle (cheap shared clone, never deep-copied) so the board stays `Clone`.
    knowledge_registry: Option<Arc<Mutex<KnowledgeActionRegistry>>>,
    /// MT-042 (IN-042-08): the live `egui::Id` of each TOOLBAR-owned control whose author_id collides
    /// with the MT-042 canonical catalog (pan/zoom/add-card/place-block). Recorded by `show` each frame so
    /// `take_knowledge_dispatched` can consume a swarm `Click` (incl. a parameterized JSON payload) at the
    /// SAME node the toolbar emitted — avoiding a second parallel registry node. Not part of `Clone`
    /// equality semantics (a transient per-frame map).
    toolbar_control_ids: std::collections::HashMap<&'static str, egui::Id>,
    /// MT-042: swarm AccessKit dispatches the in-render sync/emit/take loop consumed THIS frame but that
    /// the single-`Option` `show` return cannot carry. The host drains them via
    /// [`Self::drain_knowledge_events`] after `show`. This is the must-fix anti-scaffolding wiring: `show`
    /// itself drives the registry, so any host that renders the board gets a populated AccessKit tree +
    /// consumed dispatch with no extra calls.
    pending_knowledge_events: Vec<CanvasEvent>,
}

impl LoomCanvasBoard {
    /// A fresh board for `workspace_id` + `canvas_block_id` (no placements yet — the host loads them).
    pub fn new(workspace_id: impl Into<String>, canvas_block_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            canvas_block_id: canvas_block_id.into(),
            placements: Vec::new(),
            visual_edges: Vec::new(),
            pan: Vec2::ZERO,
            zoom: 1.0,
            selected: HashSet::new(),
            edge_mode: EdgeMode::Semantic,
            edge_from: None,
            status: String::new(),
            loading: false,
            error: None,
            place_block_input: String::new(),
            resizing: None,
            moving: None,
            moving_pos: None,
            editing_card_id: None,
            editing_buffer: String::new(),
            section_labels: std::collections::BTreeMap::new(),
            last_canvas_rect: None,
            group_seq: 0,
            knowledge_registry: None,
            toolbar_control_ids: std::collections::HashMap::new(),
            pending_knowledge_events: Vec::new(),
        }
    }

    /// WP-KERNEL-012 MT-061: install the per-`group_id` section labels (board section/group metadata) the
    /// derived section frames title themselves with. The host calls this alongside `set_board` when the
    /// board payload carries section metadata; absent labels fall back to the `group_id` string.
    pub fn set_section_labels(&mut self, labels: std::collections::BTreeMap<String, String>) {
        self.section_labels = labels;
    }

    /// WP-KERNEL-012 MT-061: the placement id of the free-text card currently in in-place edit mode (for
    /// the host / proof tests to observe edit state). `None` when no card is being edited.
    pub fn editing_card_id(&self) -> Option<&str> {
        self.editing_card_id.as_deref()
    }

    /// WP-KERNEL-012 MT-061: the current inline-editor buffer contents (for the host / proof tests to
    /// observe what has been typed before commit). Empty when no card is being edited.
    pub fn editing_buffer(&self) -> &str {
        &self.editing_buffer
    }

    /// WP-KERNEL-012 MT-061: the last rendered canvas surface rect (screen space), recorded each frame.
    /// `None` until the board has rendered once. Exposed so a host / proof harness can convert a
    /// canvas-space point to the exact screen coordinate of the LIVE widget (no hard-coded layout offset).
    pub fn last_canvas_rect(&self) -> Option<Rect> {
        self.last_canvas_rect
    }

    /// WP-KERNEL-012 MT-061: convert a CANVAS-space point to its exact SCREEN coordinate using the last
    /// rendered canvas origin (so a proof harness can click the real handle/card without guessing the
    /// layout offset). Returns `None` before the first render. Mirrors [`Self::canvas_to_screen`] with the
    /// recorded origin.
    pub fn canvas_point_to_screen(&self, canvas: Pos2) -> Option<Pos2> {
        self.last_canvas_rect.map(|rect| self.canvas_to_screen(canvas, rect.min.to_vec2()))
    }

    /// WP-KERNEL-012 MT-061: the derived section layer for the CURRENT placements + section labels (the
    /// frames drawn behind the cards). Re-derived on demand so a removed/cleared group's frame disappears.
    pub fn section_layer(&self) -> crate::graph::canvas_sections::SectionLayer {
        crate::graph::canvas_sections::SectionLayer::derive(&self.placements, &self.section_labels)
    }

    /// WP-KERNEL-012 MT-061 (MC-061-2 rollback): restore a placement's geometry to the last
    /// server-confirmed value after a resize/move PATCH FAILS, so the canvas never shows geometry the
    /// backend never stored. The host calls this with the placement id + the pre-edit `(x, y, w, h)` it
    /// snapshotted when it dispatched the PATCH; on failure the optimistic geometry is reverted and the
    /// transient resize/move state is cleared. Returns `true` if a matching placement was rolled back.
    pub fn rollback_placement_geometry(&mut self, placement_id: &str, x: f32, y: f32, w: f32, h: f32) -> bool {
        // Clear any transient in-flight state for this placement so the optimistic value can't re-apply.
        if self.resizing.as_ref().map(|(id, _, _)| id.as_str()) == Some(placement_id) {
            self.resizing = None;
        }
        if self.moving.as_deref() == Some(placement_id) {
            self.moving = None;
            self.moving_pos = None;
        }
        if let Some(card) = self.placements.iter_mut().find(|p| p.placement_id == placement_id) {
            card.x = x;
            card.y = y;
            card.w = w;
            card.h = h;
            self.status = format!("Reverted {placement_id} (save failed)");
            true
        } else {
            false
        }
    }

    /// MT-042: record a toolbar-owned control's live `egui::Id` (called by `show` for each colliding
    /// control), so `take_knowledge_dispatched` consumes a swarm `Click` at the toolbar node rather than
    /// a duplicate registry node (IN-042-08).
    fn record_toolbar_id(&mut self, author_id: &'static str, id: egui::Id) {
        self.toolbar_control_ids.insert(author_id, id);
    }

    /// The default canvas-space position for a fallback `Place` (MC-2): the centre of the
    /// currently-visible canvas. Falls back to `(40, 40)` (the React default) before the first render
    /// records a canvas rect, so a headless place still lands on a deterministic spot.
    fn default_place_pos(&self) -> Pos2 {
        match self.last_canvas_rect {
            Some(rect) => self.screen_to_canvas(rect.center(), rect.min.to_vec2()),
            None => Pos2::new(40.0, 40.0),
        }
    }

    /// Replace the placement + visual-edge set (after a `getCanvasBoard` fetch resolves) and set the
    /// viewport from the board state. Clears the transient selection/edge-draw state so a reload never
    /// leaves a dangling edge-from referencing a removed placement.
    pub fn set_board(
        &mut self,
        placements: Vec<CanvasPlacementCard>,
        visual_edges: Vec<VisualEdge>,
        pan: Vec2,
        zoom: f32,
    ) {
        self.placements = placements;
        self.visual_edges = visual_edges;
        self.pan = pan;
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        // Drop selection / edge-from ids that no longer exist after the reload.
        let present: HashSet<&str> = self.placements.iter().map(|p| p.placement_id.as_str()).collect();
        self.selected.retain(|id| present.contains(id.as_str()));
        if let Some(from) = &self.edge_from {
            if !present.contains(from.as_str()) {
                self.edge_from = None;
            }
        }
        // WP-KERNEL-012 MT-061: a reload reconciles server truth, so any in-flight resize/move optimistic
        // state is now stale — drop it. Drop an in-place editor whose card no longer exists; keep editing
        // a card that survived the reload (the host applies an edit, refreshes, and may keep editing).
        self.resizing = None;
        self.moving = None;
        self.moving_pos = None;
        if let Some(editing) = &self.editing_card_id {
            if !present.contains(editing.as_str()) {
                self.editing_card_id = None;
                self.editing_buffer.clear();
            }
        }
        self.loading = false;
        self.error = None;
    }

    /// Canvas-space -> screen-space transform (THE single pair, RISK-1 / MC-1):
    /// `screen = origin + pan + canvas * zoom`. `origin` is the canvas rect's top-left.
    pub fn canvas_to_screen(&self, canvas: Pos2, origin: Vec2) -> Pos2 {
        Pos2::new(
            origin.x + self.pan.x + canvas.x * self.zoom,
            origin.y + self.pan.y + canvas.y * self.zoom,
        )
    }

    /// Screen-space -> canvas-space inverse (THE single pair, RISK-1 / MC-1):
    /// `canvas = (screen - origin - pan) / zoom`.
    pub fn screen_to_canvas(&self, screen: Pos2, origin: Vec2) -> Pos2 {
        Pos2::new(
            (screen.x - origin.x - self.pan.x) / self.zoom,
            (screen.y - origin.y - self.pan.y) / self.zoom,
        )
    }

    /// The placement whose canvas rect contains `canvas_pos`, in reverse z-order (topmost wins). Used by
    /// card-click hit testing and pan-vs-card detection.
    fn placement_at_canvas(&self, canvas_pos: Pos2) -> Option<usize> {
        // Iterate by descending z_index so the visually-top card wins the hit; ties keep list order.
        let mut indices: Vec<usize> = (0..self.placements.len()).collect();
        indices.sort_by(|&a, &b| self.placements[b].z_index.cmp(&self.placements[a].z_index));
        indices
            .into_iter()
            .find(|&i| self.placements[i].canvas_rect().contains(canvas_pos))
    }

    /// Apply a scroll-wheel zoom around `pointer` (zoom-to-pointer): keep the canvas point under the
    /// cursor fixed while scaling. `scroll_y` is the wheel delta sign (+1 = zoom in). Returns true if the
    /// zoom actually changed (so the host can persist on the gesture).
    pub fn apply_zoom(&mut self, scroll_y: f32, pointer: Pos2, origin: Vec2) -> bool {
        if scroll_y == 0.0 {
            return false;
        }
        let canvas_before = self.screen_to_canvas(pointer, origin);
        let old = self.zoom;
        let factor = 1.15f32.powf(scroll_y);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if (self.zoom - old).abs() < f32::EPSILON {
            return false;
        }
        // Re-derive pan so `canvas_before` maps back to the same screen `pointer` after the scale.
        let screen_after = self.canvas_to_screen(canvas_before, origin);
        self.pan.x += pointer.x - screen_after.x;
        self.pan.y += pointer.y - screen_after.y;
        true
    }

    /// Step the zoom by a button (`+ZOOM_STEP` / `-ZOOM_STEP`), clamped, rounded to 2dp to match the
    /// React label exactly. Returns the new zoom.
    fn step_zoom(&mut self, delta: f32) -> f32 {
        let raw = self.zoom + delta;
        self.zoom = ((raw * 100.0).round() / 100.0).clamp(MIN_ZOOM, MAX_ZOOM);
        self.zoom
    }

    /// Build the typed edge event for the current `edge_mode` between `from`/`to` placement ids,
    /// resolving block ids for the semantic case. `None` if either placement is gone (defensive).
    fn edge_event(&self, from: &str, to: &str) -> Option<CanvasEvent> {
        let from_p = self.placements.iter().find(|p| p.placement_id == from)?;
        let to_p = self.placements.iter().find(|p| p.placement_id == to)?;
        Some(match self.edge_mode {
            EdgeMode::Semantic => CanvasEvent::SemanticEdge {
                source_block_id: from_p.placed_block_id.clone(),
                target_block_id: to_p.placed_block_id.clone(),
            },
            EdgeMode::Visual => CanvasEvent::VisualEdgeAdded {
                from_placement_id: from.to_owned(),
                to_placement_id: to.to_owned(),
            },
        })
    }

    /// Render the board + return the typed event (if any) this frame produced. The host applies the
    /// event (mutate via the backend client, then re-fetch). The widget performs NO network IO.
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<CanvasEvent> {
        let mut event: Option<CanvasEvent> = None;

        // ── Toolbar strip ─────────────────────────────────────────────────────────────────────────
        // WP-KERNEL-012 MT-042 (IN-042-08 no-duplicate-node): the control author_ids the MT-026 toolbar
        // shares with the MT-042 KnowledgeActionRegistry canonical catalog (pan-left/right, zoom-in/out,
        // add-card, place-block) stay OWNED by the toolbar button (emitted on the button's own id so the
        // node keeps its layout rect — kittest interaction depends on it). The registry does NOT emit a
        // second parallel node for these (see `sync_knowledge_registry`, which skips the toolbar-owned
        // ids); instead the pane RECORDS each toolbar button's NodeId here so `take_knowledge_dispatched`
        // can consume a swarm `Click` (incl. a parameterized place-block/add-card JSON payload) targeting
        // that one node. A plain (no-payload) swarm Click on a toolbar button also triggers egui's own
        // synthetic `.clicked()` so pan/zoom apply through the existing handler — never double-applied.
        ui.horizontal(|ui| {
            let pan_left = ui.button("◀ Pan");
            emit_button_node(ui, pan_left.id, PAN_LEFT_AUTHOR_ID, "Pan left");
            self.record_toolbar_id(PAN_LEFT_AUTHOR_ID, pan_left.id);
            if pan_left.clicked() {
                self.pan.x -= PAN_STEP;
                event = Some(self.viewport_event());
            }
            let pan_right = ui.button("Pan ▶");
            emit_button_node(ui, pan_right.id, PAN_RIGHT_AUTHOR_ID, "Pan right");
            self.record_toolbar_id(PAN_RIGHT_AUTHOR_ID, pan_right.id);
            if pan_right.clicked() {
                self.pan.x += PAN_STEP;
                event = Some(self.viewport_event());
            }

            ui.separator();
            let zoom_out = ui.button("−");
            emit_button_node(ui, zoom_out.id, ZOOM_OUT_AUTHOR_ID, "Zoom out");
            self.record_toolbar_id(ZOOM_OUT_AUTHOR_ID, zoom_out.id);
            if zoom_out.clicked() {
                self.step_zoom(-ZOOM_STEP);
                event = Some(self.viewport_event());
            }
            let zoom_label = format!("{:.2}x", self.zoom);
            let zlabel = ui.label(&zoom_label);
            emit_status_node(ui, zlabel.id, ZOOM_VALUE_AUTHOR_ID, &zoom_label);
            let zoom_in = ui.button("+");
            emit_button_node(ui, zoom_in.id, ZOOM_IN_AUTHOR_ID, "Zoom in");
            self.record_toolbar_id(ZOOM_IN_AUTHOR_ID, zoom_in.id);
            if zoom_in.clicked() {
                self.step_zoom(ZOOM_STEP);
                event = Some(self.viewport_event());
            }

            ui.separator();
            let add_card = ui.button("+ Text card");
            emit_button_node(ui, add_card.id, ADD_CARD_AUTHOR_ID, "Add text card");
            self.record_toolbar_id(ADD_CARD_AUTHOR_ID, add_card.id);
            if add_card.clicked() {
                // React: title = `Card ${new Date().toISOString()}`; default position (40, 40).
                let title = format!("Card {}", now_iso8601());
                event = Some(CanvasEvent::AddCard { title, x: 40.0, y: 40.0 });
            }

            let can_group = self.selected.len() >= 2;
            let group_label = format!("Group ({})", self.selected.len());
            let group_btn = ui.add_enabled(can_group, egui::Button::new(&group_label));
            emit_button_node(ui, group_btn.id, GROUP_AUTHOR_ID, &group_label);
            if group_btn.clicked() && can_group {
                self.group_seq += 1;
                let group_id = format!("grp-{}", self.group_seq);
                let placement_ids: Vec<String> = self.selected.iter().cloned().collect();
                // Reflect the group locally so the AccessKit data-group-id is visible THIS frame (AC6);
                // the host persists each via updateCanvasPlacement and the next refresh confirms it.
                for p in self.placements.iter_mut() {
                    if self.selected.contains(&p.placement_id) {
                        p.group_id = Some(group_id.clone());
                    }
                }
                self.status = format!("Grouped {} placements as {group_id}", placement_ids.len());
                event = Some(CanvasEvent::Group { placement_ids, group_id });
            }

            ui.separator();
            let mode_label = format!("Edge: {}", self.edge_mode.label());
            let mode_btn = ui.button(&mode_label);
            emit_button_node(ui, mode_btn.id, EDGE_MODE_AUTHOR_ID, &mode_label);
            if mode_btn.clicked() {
                self.edge_mode = self.edge_mode.toggled();
            }
            let can_start = self.selected.len() == 1 && self.edge_from.is_none();
            let start_edge = ui.add_enabled(can_start, egui::Button::new("Draw edge from selected"));
            emit_button_node(ui, start_edge.id, START_EDGE_AUTHOR_ID, "Draw edge from selected");
            if start_edge.clicked() && can_start {
                if let Some(first) = self.selected.iter().next().cloned() {
                    self.edge_from = Some(first);
                    self.status = "Click a second card to draw the edge".to_owned();
                }
            }

            // ── MC-2 / RISK-2 fallback: place a block by id when OS / inter-panel drag is unavailable.
            // A small text field + 'Place' button emit the SAME PlaceBlock event the drop path produces,
            // so the place behavior is reachable on every backend (the contract's documented fallback).
            ui.separator();
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.place_block_input)
                    .desired_width(120.0)
                    .hint_text("block id"),
            );
            emit_text_field_node(ui, field.id, PLACE_BLOCK_INPUT_AUTHOR_ID, &self.place_block_input);
            let block_id = self.place_block_input.trim().to_owned();
            let can_place = !block_id.is_empty();
            // MT-042: the MC-2 fallback button stays `add_enabled(can_place, ..)` for the MOUSE path
            // (disabled while the text field is empty — unchanged MT-026 behavior). The AccessKit node is
            // emitted via `emit_button_node`, which does NOT propagate the widget's disabled state, so the
            // node stays discoverable + dispatchable for the registry's parameterized
            // `place-block {block_id,x,y}` swarm path (IN-042-08: the toolbar owns the id; the registry
            // does not re-mint it, it consumes the dispatch at the recorded button NodeId).
            let place_btn = ui.add_enabled(can_place, egui::Button::new("Place"));
            emit_button_node(ui, place_btn.id, PLACE_BLOCK_AUTHOR_ID, "Place block by id");
            self.record_toolbar_id(PLACE_BLOCK_AUTHOR_ID, place_btn.id);
            if place_btn.clicked() && can_place {
                // Default canvas position: the centre of the currently-visible canvas, in canvas space,
                // so the placed card lands where the user is looking regardless of pan/zoom.
                let pos = self.default_place_pos();
                self.place_block_input.clear();
                self.status = format!("Placed {block_id} (reference)");
                event = Some(CanvasEvent::PlaceBlock {
                    placed_block_id: block_id,
                    x: pos.x,
                    y: pos.y,
                });
            }
        });

        // ── Status bar (Role::Status) ─────────────────────────────────────────────────────────────
        let status_text = if let Some(err) = &self.error {
            format!("Canvas error: {err}")
        } else if self.status.is_empty() {
            format!("{} placements", self.placements.len())
        } else {
            self.status.clone()
        };
        let status_resp = ui.label(&status_text);
        emit_status_node(ui, status_resp.id, STATUS_AUTHOR_ID, &status_text);

        // ── Canvas surface (fills the remaining rect) ───────────────────────────────────────────────
        let (rect, canvas_resp) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let origin = rect.min.to_vec2();
        // Record the canvas rect so the MC-2 fallback can place at the visible centre (toolbar runs
        // before this allocation, so it needs last frame's rect).
        self.last_canvas_rect = Some(rect);

        // WP-KERNEL-012 MT-061: derive the section/group frame layer ONCE per frame from the current
        // placements' group_id + the board section labels. Reused by BOTH the drag-stop drop hit-test
        // (which_section) and the behind-cards frame draw pass — one derivation, one source of truth.
        let section_layer =
            crate::graph::canvas_sections::SectionLayer::derive(&self.placements, &self.section_labels);

        // Background fill + dotted grid (canvas is never blank/white — PROOF6).
        painter.rect_filled(rect, 0.0, palette.bg);
        self.draw_grid(&painter, rect, origin, palette);

        // ── AC4 / PROOF3: drop-to-place. A Loom block dragged from another panel via egui's
        // DragAndDrop channel (payload [`CanvasDragPayload`], the native peer of the React
        // CANVAS_DRAG_MIME `dataTransfer`) and RELEASED over the canvas places a REFERENCE card. The
        // drop position is computed in CANVAS space with the SAME screen_to_canvas inverse used by
        // hit-testing (RISK-1 / MC-1), exactly mirroring the React `(clientX-rect.left-pan.x)/zoom`.
        // DROP TYPE DISPATCH (egui take-payload hazard): `dnd_release_payload::<T>()` calls
        // `DragAndDrop::take_payload::<T>()`, which UNCONDITIONALLY `take()`s the in-flight payload and
        // only THEN downcasts — so calling it for the WRONG type silently DISCARDS the other type's
        // payload. The canvas accepts two payload types (the native `CanvasDragPayload` and the
        // cross-surface MT-033 `DragPayload`), so we must guard each take with `has_payload_of_type` so a
        // `DragPayload` drop is never swallowed by the `CanvasDragPayload` take (RISK / MT-033 drop bug).
        let drop_canvas_pos = || {
            let drop_screen = canvas_resp
                .interact_pointer_pos()
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
                .unwrap_or_else(|| rect.center());
            self.screen_to_canvas(drop_screen, origin)
        };
        if egui::DragAndDrop::has_payload_of_type::<CanvasDragPayload>(ui.ctx()) {
            if let Some(payload) = canvas_resp.dnd_release_payload::<CanvasDragPayload>() {
                let canvas_pos = drop_canvas_pos();
                self.status = format!("Placed {} (reference)", payload.block_id);
                event = Some(CanvasEvent::PlaceBlock {
                    placed_block_id: payload.block_id.clone(),
                    x: canvas_pos.x,
                    y: canvas_pos.y,
                });
            }
        } else if egui::DragAndDrop::has_payload_of_type::<crate::interop::DragPayload>(ui.ctx()) {
            // WP-KERNEL-012 MT-033 (E5 — CKC drag-in): a CKC/Atelier item (or a Loom block) dragged from
            // the atelier side panel via the cross-surface [`crate::interop::DragPayload`] channel and
            // RELEASED over the canvas places a block REFERENCE — IFF the payload resolves to a
            // `placed_block_id` (MT-026: the placement body takes a block id, never an `atelier_item_id`).
            // An UNRESOLVED atelier item (no `loom_block_id`) is a typed no-op with a visible status — NOT
            // a fake POST (RISK-3 / MC-3). Reuses the SAME `screen_to_canvas` inverse + `PlaceBlock` event.
            if let Some(payload) = canvas_resp.dnd_release_payload::<crate::interop::DragPayload>() {
                match payload.canvas_drag_payload() {
                    Some(cdp) => {
                        let canvas_pos = drop_canvas_pos();
                        self.status = format!("Placed {} (reference)", cdp.block_id);
                        event = Some(CanvasEvent::PlaceBlock {
                            placed_block_id: cdp.block_id.clone(),
                            x: canvas_pos.x,
                            y: canvas_pos.y,
                        });
                    }
                    None => {
                        // A CKC item not yet resolved to a Loom block id cannot be placed (no fake field).
                        self.status =
                            "Dropped CKC item needs a loom block id before it can be placed on the canvas"
                                .to_owned();
                    }
                }
            }
        }

        // Pointer input: zoom (scroll), pan (drag on empty area), card click (select / edge).
        if let Some(pointer) = canvas_resp.hover_pos() {
            let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_y != 0.0 && self.apply_zoom(scroll_y.signum(), pointer, origin) {
                // Persist on the gesture (the host debounces — RISK-3 / MC-3).
                event = Some(self.viewport_event());
            }
        }

        // WP-KERNEL-012 MT-061: drag dispatch on the canvas surface.
        //   - A drag that BEGINS over a card body MOVES that card (optimistic), and on release resolves a
        //     section assignment from the drop position (AC-061-3). The card-RESIZE drag is owned by the
        //     handle's own `ui.interact` inside `draw_card` (consumed before this parent response), so a
        //     resize never reaches here.
        //   - A drag on EMPTY canvas pans (unchanged MT-026 behavior).
        // A card-move drag SUPPRESSES the pan + the viewport-persist on release (it persists a section
        // assignment instead, not a viewport). The `resizing` guard keeps a resize from also moving a card.
        if canvas_resp.drag_started() && self.resizing.is_none() && self.moving.is_none() {
            // Use the PRESS origin (where the gesture began), not the current pointer — on the frame
            // drag_started() fires the pointer may already have moved off the card toward the drop point.
            let press = ui
                .input(|i| i.pointer.press_origin())
                .or_else(|| canvas_resp.interact_pointer_pos());
            if let Some(screen) = press {
                let canvas_pos = self.screen_to_canvas(screen, origin);
                if let Some(idx) = self.placement_at_canvas(canvas_pos) {
                    let card = &self.placements[idx];
                    self.moving = Some(card.placement_id.clone());
                    self.moving_pos = Some(Pos2::new(card.x, card.y));
                }
            }
        }
        if canvas_resp.dragged() {
            if let Some(moving_id) = self.moving.clone() {
                // Move the card optimistically by the canvas-space delta (screen delta / zoom).
                let delta = canvas_resp.drag_delta() / self.zoom;
                if let Some(card) = self.placements.iter_mut().find(|p| p.placement_id == moving_id) {
                    card.x += delta.x;
                    card.y += delta.y;
                    self.moving_pos = Some(Pos2::new(card.x, card.y));
                }
            } else if self.resizing.is_none() {
                // Empty-canvas pan (no card under the gesture, no active resize).
                self.pan += canvas_resp.drag_delta();
            }
        }
        if canvas_resp.drag_stopped() {
            if let Some(moving_id) = self.moving.take() {
                // AC-061-3: resolve the drop position to a section. The drop point is the card's CENTRE in
                // canvas space (where the user released it), so a card landing inside a frame is assigned,
                // and one released outside all frames clears its group (Some(None) -> clear_group).
                let drop_pos = self
                    .placements
                    .iter()
                    .find(|p| p.placement_id == moving_id)
                    .map(|c| Pos2::new(c.x + c.w * 0.5, c.y + c.h * 0.5))
                    .or(self.moving_pos)
                    .unwrap_or(Pos2::ZERO);
                // Derive the drop-test layer EXCLUDING the moving card, so a card cannot anchor the frame
                // it is trying to LEAVE (a member dragged out of its own section must be able to clear it —
                // otherwise its own bounds keep the frame open under it). RISK-061-4 hit-test is computed
                // against the OTHER cards' frames only.
                let drop_layer = crate::graph::canvas_sections::SectionLayer::derive(
                    &self
                        .placements
                        .iter()
                        .filter(|p| p.placement_id != moving_id)
                        .cloned()
                        .collect::<Vec<_>>(),
                    &self.section_labels,
                );
                let target = drop_layer.which_section(drop_pos).map(ToOwned::to_owned);
                // Reflect the assignment locally so the AccessKit data-group-id updates THIS frame; the
                // host persists via updateCanvasPlacement and the next getCanvasBoard refresh confirms it.
                if let Some(card) = self.placements.iter_mut().find(|p| p.placement_id == moving_id) {
                    card.group_id = target.clone();
                }
                self.status = match &target {
                    Some(g) => format!("Assigned {moving_id} to section {g}"),
                    None => format!("Cleared {moving_id} section"),
                };
                self.moving_pos = None;
                event = Some(CanvasEvent::AssignSection { placement_id: moving_id, group_id: target });
            } else if self.resizing.is_none() {
                // Pan release: persist the viewport (RISK-3 / MC-3 — host debounces).
                event = Some(self.viewport_event());
            }
        }

        // WP-KERNEL-012 MT-061 (AC-061-4): double-click a card. A FREE-TEXT card enters in-place edit
        // mode (its body seeds the editor buffer); a BLOCK-backed card NAVIGATES to its block instead of
        // becoming editable (reference-not-copy gate — the inline editor is strictly text-card-only).
        if canvas_resp.double_clicked() {
            if let Some(screen) = canvas_resp.interact_pointer_pos() {
                let canvas_pos = self.screen_to_canvas(screen, origin);
                if let Some(idx) = self.placement_at_canvas(canvas_pos) {
                    let card = &self.placements[idx];
                    if card.card_kind.is_text_card() {
                        // Enter edit mode: seed the buffer from the card's own body (never a block copy).
                        self.editing_card_id = Some(card.placement_id.clone());
                        self.editing_buffer = card.live_body.clone().unwrap_or_default();
                        self.status = format!("Editing {} (text card)", card.placement_id);
                    } else {
                        // A block reference navigates to its block — it is NEVER turned into an editor.
                        self.status =
                            format!("Open block {} (reference — not inline-editable)", card.placed_block_id);
                    }
                }
            }
        }

        // Click: card hit -> toggle selection (shift = additive) and complete a pending edge; empty ->
        // deselect all.
        if canvas_resp.clicked() {
            if let Some(screen) = canvas_resp.interact_pointer_pos() {
                let canvas_pos = self.screen_to_canvas(screen, origin);
                let shift = ui.input(|i| i.modifiers.shift);
                if let Some(idx) = self.placement_at_canvas(canvas_pos) {
                    let pid = self.placements[idx].placement_id.clone();
                    // Complete a pending edge BEFORE mutating selection (edge_from cleared immediately —
                    // RISK-6 / MC: no double-mutate).
                    if let Some(from) = self.edge_from.take() {
                        if from != pid {
                            if let Some(ev) = self.edge_event(&from, &pid) {
                                self.status = match &ev {
                                    CanvasEvent::SemanticEdge { .. } => {
                                        "Semantic edge created (real loom edge)".to_owned()
                                    }
                                    _ => "Visual-only edge added (NOT graph authority)".to_owned(),
                                };
                                event = Some(ev);
                            }
                        }
                    } else {
                        if !shift {
                            self.selected.clear();
                        }
                        if self.selected.contains(&pid) {
                            self.selected.remove(&pid);
                        } else {
                            self.selected.insert(pid);
                        }
                    }
                } else {
                    // Empty-area click: deselect all.
                    self.selected.clear();
                    self.edge_from = None;
                }
            }
        }

        // ── WP-KERNEL-012 MT-061: section/group FRAMES, drawn FIRST so they read as containers BEHIND
        // their member cards. Each frame emits a `canvas.section.{id}` AccessKit node (HBR-SWARM). The
        // `section_layer` was derived once at the top of the canvas pass (reused by the drop hit-test).
        self.draw_section_frames(ui, &painter, &section_layer, origin);

        // Visual edges next (so cards render on top of edges, but on top of the section frames too).
        self.draw_visual_edges(&painter, origin, palette);

        // Placement cards (+ remove button + resize handle + inline editor + AccessKit). Drawn in
        // ascending z-order so the topmost card paints last.
        let mut order: Vec<usize> = (0..self.placements.len()).collect();
        order.sort_by(|&a, &b| self.placements[a].z_index.cmp(&self.placements[b].z_index));
        for idx in order {
            if let Some(ev) = self.draw_card(ui, &painter, idx, origin, palette) {
                event = Some(ev);
            }
        }

        // Loading overlay animates ONLY during a genuine in-flight fetch (host sets `loading=true` only
        // when a runtime-backed request is dispatched). A headless render is neutral (no perpetual
        // spinner — MT-015 lesson). Empty board (AC10): no overlay, no "(stale reference)" text.
        if self.loading {
            draw_overlay_label(&painter, rect, "Loading canvas…", palette.text_subtle, palette);
            ui.ctx().request_repaint();
        }

        // ── WP-KERNEL-012 MT-042 (E7): drive the knowledge AccessKit surface FROM the render path. ───
        // The must-fix anti-scaffolding wiring (the MT-041 pattern). Runs AFTER the toolbar so
        // `take_knowledge_dispatched` sees the toolbar-owned NodeIds recorded this frame (IN-042-08).
        // Gated on an installed registry so a bare `board.show(ui, &palette)` stays a pure no-op.
        if self.knowledge_registry.is_some() {
            self.sync_knowledge_registry();
            self.emit_knowledge_accesskit(ui);
            let dispatched = self.take_knowledge_dispatched(ui);
            self.pending_knowledge_events.extend(dispatched);
        }

        event
    }

    /// MT-042: drain the swarm AccessKit dispatches the in-render sync/emit/take loop consumed since the
    /// last drain. The host calls this AFTER [`Self::show`] to route each dispatched [`CanvasEvent`] to the
    /// E6 loom client (the same way it applies `show`'s `Option` return).
    pub fn drain_knowledge_events(&mut self) -> Vec<CanvasEvent> {
        std::mem::take(&mut self.pending_knowledge_events)
    }

    /// MT-031 (E5 melt-together): the canvas board's selected placement, as the referenced Loom block
    /// id, for the shared [`crate::interop::InteractionBus`] selection model. Returns the
    /// `placed_block_id` of the single selected placement (the first when several are selected — the
    /// canvas multi-selects for grouping, but the shared selection is a single focus reference), or
    /// `None` when nothing is selected. The host publishes this to the bus via
    /// `graph::interop_adapter::canvas_node_selection` so a cross-pane Copy / backlink can address the
    /// canvas's selected block by `loom://{block_id}` (the contract's "canvas node selection feeds
    /// SharedSelection"). Reuses the existing `selected` set + `placements` projection — no new state.
    pub fn shared_selection_block_id(&self) -> Option<String> {
        let placement_id = self.selected.iter().next()?;
        self.placements
            .iter()
            .find(|p| &p.placement_id == placement_id)
            .map(|p| p.placed_block_id.clone())
    }

    /// The current viewport-persist event (pan/zoom snapshot the host PUTs to `.../viewport`).
    fn viewport_event(&self) -> CanvasEvent {
        CanvasEvent::ViewportChanged {
            pan_x: self.pan.x,
            pan_y: self.pan.y,
            zoom: self.zoom,
        }
    }

    // ── WP-KERNEL-012 MT-042 (E7): knowledge AccessKit action surface ─────────────────────────────

    /// Install the shared knowledge AccessKit action registry (the MT-041 `install_*` pattern).
    pub fn install_knowledge_action_registry(&mut self, registry: Arc<Mutex<KnowledgeActionRegistry>>) {
        self.knowledge_registry = Some(registry);
    }

    /// The canvas control author_ids the MT-026 TOOLBAR already emits (so the MT-042 registry must NOT
    /// re-mint them — IN-042-08). The registry owns the rest of `CANVAS_CONTROL_CATALOG` + the per-card
    /// identities; the toolbar owns these and the pane consumes their dispatch at the recorded button id.
    const TOOLBAR_OWNED: &'static [&'static str] = &[
        PAN_LEFT_AUTHOR_ID,
        PAN_RIGHT_AUTHOR_ID,
        ZOOM_IN_AUTHOR_ID,
        ZOOM_OUT_AUTHOR_ID,
        ADD_CARD_AUTHOR_ID,
        PLACE_BLOCK_AUTHOR_ID,
    ];

    /// Populate the knowledge registry with the canvas GLOBAL controls NOT already owned by the toolbar
    /// (zoom-reset, deselect-all, remove-placement, add-edge, remove-edge, select-card — fixed Button
    /// nodes regardless of content, AC-042-08) and one `canvas.card.<placement_id>` Group identity per
    /// LIVE placement (AC-042-03). Each card node carries `block_id=<placed_block_id>` in its AccessKit
    /// `value` so a swarm agent correlates a placement to its source block (IN-042-02). The toolbar-owned
    /// ids (pan/zoom/add-card/place-block) are SKIPPED here to avoid a second parallel node — the toolbar
    /// emits them and the pane consumes their dispatch at the recorded button id (IN-042-08). Fully
    /// re-derived each frame so a removed placement DISAPPEARS (deletion-by-absence — IN-042-10).
    /// placement_ids are real UUIDs minted by the backend (RISK-042-02 / CTRL-042-02), surfaced verbatim.
    pub fn sync_knowledge_registry(&self) {
        let Some(registry) = &self.knowledge_registry else { return };
        let mut reg = registry.lock().unwrap_or_else(|e| e.into_inner());
        reg.clear_nodes();
        for entry in CANVAS_CONTROL_CATALOG {
            if Self::TOOLBAR_OWNED.contains(&entry.author_id) {
                continue; // toolbar owns this id (IN-042-08 no-duplicate)
            }
            reg.upsert_control(entry.author_id, entry.label, KnowledgeNodeState::present());
        }
        for card in &self.placements {
            let author = knowledge_action_registry::canvas_card_author_id(&card.placement_id);
            // AC-042-03: the card carries its source block_id (IN-042-02) + advertises the delete route so
            // a swarm reads how to delete this exact placement.
            let value = Some(format!(
                "block_id={};group_id={};delete=canvas.remove-placement",
                card.placed_block_id,
                card.group_id.as_deref().unwrap_or("none")
            ));
            // AC-042-03: a card declares BOTH 'activate' (Click) AND 'delete' (a real AccessKit custom
            // action on the node), so the swarm-readable action set on canvas.card.<id> is {Click, Focus,
            // delete}. A 'delete' custom-action dispatch on the card maps to RemovePlacement.
            reg.upsert_identity_with_actions(
                author,
                KAxRole::Group,
                card.display_title().to_owned(),
                value,
                &["delete"],
                KnowledgeNodeState::present(),
            );
        }
    }

    /// Emit the knowledge registry's nodes into the live AccessKit tree (call inside the host's `show`,
    /// after [`Self::sync_knowledge_registry`]). No-op if no registry is installed.
    pub fn emit_knowledge_accesskit(&self, ui: &egui::Ui) {
        if let Some(registry) = &self.knowledge_registry {
            registry.lock().unwrap_or_else(|e| e.into_inner()).emit_into_tree(ui);
        }
    }

    /// Consume this frame's swarm AccessKit `Click` dispatches targeting the canvas knowledge nodes and
    /// MAP each to a typed [`CanvasEvent`] (RISK-042-04). Parameterized actions parse JSON via the
    /// no-unwrap [`knowledge_action_registry::parse_payload`] seam (RISK-042-03 / CTRL-042-03). Reads BOTH
    /// the registry-owned nodes (zoom-reset/deselect/remove-placement/add-edge/remove-edge/select-card +
    /// per-card identities) AND the toolbar-owned ids (pan/zoom/add-card/place-block) at their recorded
    /// button NodeIds (IN-042-08 — one node per id, the toolbar's). A `canvas.card.<id>` click maps to a
    /// select of that placement (the swarm select-by-identity path).
    pub fn take_knowledge_dispatched(&mut self, ui: &egui::Ui) -> Vec<CanvasEvent> {
        if self.knowledge_registry.is_none() {
            return Vec::new();
        }
        // Registry-owned node dispatches (non-toolbar controls + per-card identities).
        let registry = self.knowledge_registry.as_ref().unwrap();
        let dispatched = registry.lock().unwrap_or_else(|e| e.into_inner()).take_dispatched(ui);
        // Toolbar-owned dispatches: read the raw AccessKit Click at each recorded button id, but ONLY
        // forward the PARAMETERIZED (payload-carrying) ones to `apply_knowledge_action`.
        //
        // DOUBLE-APPLY GUARD (the diff comment at the toolbar emit asserts "never double-applied"; this is
        // the mechanism): a PLAIN swarm Click on a toolbar-owned NodeId is ALSO consumed by egui's own
        // synthetic `.clicked()` in `show` above, which already applied the pan/zoom/add-card effect once.
        // Re-applying it here via `apply_knowledge_action` would move pan/zoom by 2x (the latent
        // toolbar-double-apply bug the must-fix wiring would otherwise expose). So a plain (no-payload)
        // toolbar Click is DROPPED here — the toolbar's own handler owns it. A PARAMETERIZED Click (a
        // place-block/add-card JSON payload a swarm sends, which the toolbar's `.clicked()` either does not
        // fire for — `Place` needs the text field — or would build from the wrong source) IS forwarded so
        // the swarm's parameterized path produces the right `PlaceBlock`/`AddCard` event exactly once.
        let toolbar = self.toolbar_control_ids.clone();
        let mut toolbar_dispatched: Vec<(String, String)> = Vec::new();
        ui.input(|input| {
            for (author_id, id) in &toolbar {
                for request in input.accesskit_action_requests(*id, accesskit::Action::Click) {
                    if let Some(accesskit::ActionData::Value(v)) = &request.data {
                        // Parameterized only: a JSON payload the swarm supplied (place-block/add-card).
                        toolbar_dispatched.push(((*author_id).to_owned(), v.to_string()));
                    }
                    // A plain toolbar Click (no payload) is intentionally NOT collected — egui's
                    // synthetic `.clicked()` already applied it (never double-applied).
                }
            }
        });

        let mut events = Vec::new();
        for (author_id, payload) in dispatched {
            if let Some(ev) = self.apply_knowledge_action(&author_id, payload.as_deref()) {
                events.push(ev);
            }
        }
        for (author_id, payload) in toolbar_dispatched {
            if let Some(ev) = self.apply_knowledge_action(&author_id, Some(payload.as_str())) {
                events.push(ev);
            }
        }
        events
    }

    /// Map ONE canonical knowledge action (+ optional JSON payload) to a typed [`CanvasEvent`], applying
    /// any in-pane state change (pan/zoom/select). Returns `Some` for an action that produces a
    /// host-applied event, `None` for a purely in-pane action (pan/zoom/select) or a dropped malformed
    /// payload (RISK-042-03 / CTRL-042-03 — never a panic).
    fn apply_knowledge_action(&mut self, author_id: &str, payload: Option<&str>) -> Option<CanvasEvent> {
        match author_id {
            "canvas.pan-left" => { self.pan.x -= PAN_STEP; None }
            "canvas.pan-right" => { self.pan.x += PAN_STEP; None }
            "canvas.zoom-in" => { self.step_zoom(ZOOM_STEP); None }
            "canvas.zoom-out" => { self.step_zoom(-ZOOM_STEP); None }
            "canvas.zoom-reset" => { self.zoom = 1.0; None }
            "canvas.deselect-all" => {
                self.selected.clear();
                self.edge_from = None;
                None
            }
            "canvas.add-card" => {
                // The payload may carry a title; fall back to the timestamp title (React parity).
                let title = knowledge_action_registry::parse_payload::<serde_json::Value>(payload)
                    .and_then(|v| v.get("title").and_then(|t| t.as_str().map(ToOwned::to_owned)))
                    .unwrap_or_else(|| format!("Card {}", now_iso8601()));
                let pos = self.default_place_pos();
                Some(CanvasEvent::AddCard { title, x: pos.x, y: pos.y })
            }
            "canvas.place-block" => {
                let p = knowledge_action_registry::parse_payload::<PlaceBlockPayload>(payload)?;
                self.status = format!("Placed {} (reference)", p.block_id);
                Some(CanvasEvent::PlaceBlock { placed_block_id: p.block_id, x: p.x, y: p.y })
            }
            "canvas.remove-placement" => {
                let p = knowledge_action_registry::parse_payload::<PlacementIdPayload>(payload)?;
                Some(CanvasEvent::RemovePlacement { placement_id: p.placement_id })
            }
            "canvas.add-edge" => {
                let p = knowledge_action_registry::parse_payload::<AddEdgePayload>(payload)?;
                // edge_mode=visual creates a board-local visual edge between PLACEMENTS; otherwise a real
                // semantic loom edge between BLOCKS (the canvas edge_event semantics).
                if p.edge_mode.as_deref() == Some("visual") {
                    Some(CanvasEvent::VisualEdgeAdded {
                        from_placement_id: p.source_id,
                        to_placement_id: p.target_id,
                    })
                } else {
                    Some(CanvasEvent::SemanticEdge {
                        source_block_id: p.source_id,
                        target_block_id: p.target_id,
                    })
                }
            }
            "canvas.remove-edge" => {
                let p = knowledge_action_registry::parse_payload::<EdgeIdPayload>(payload)?;
                Some(CanvasEvent::RemoveEdge { edge_id: p.edge_id })
            }
            "canvas.select-card" => {
                let p = knowledge_action_registry::parse_payload::<PlacementIdPayload>(payload)?;
                if self.placements.iter().any(|c| c.placement_id == p.placement_id) {
                    self.selected.clear();
                    self.selected.insert(p.placement_id);
                }
                None
            }
            // AC-042-03: a `delete` custom-action dispatch on a card -> RemovePlacement for that card.
            other if other.starts_with(knowledge_action_registry::CANVAS_CARD_AUTHOR_ID_PREFIX)
                && other.ends_with("#delete") =>
            {
                let sanitized = other
                    .trim_start_matches(knowledge_action_registry::CANVAS_CARD_AUTHOR_ID_PREFIX)
                    .trim_end_matches("#delete");
                self.placements
                    .iter()
                    .find(|c| crate::project_tree::stable_part(&c.placement_id) == sanitized)
                    .map(|c| CanvasEvent::RemovePlacement { placement_id: c.placement_id.clone() })
            }
            other => {
                // A per-identity `canvas.card.<sanitized_placement_id>` click -> select that card.
                if let Some(stripped) = other.strip_prefix(knowledge_action_registry::CANVAS_CARD_AUTHOR_ID_PREFIX) {
                    if let Some(card) = self
                        .placements
                        .iter()
                        .find(|c| crate::project_tree::stable_part(&c.placement_id) == stripped)
                    {
                        let pid = card.placement_id.clone();
                        self.selected.clear();
                        self.selected.insert(pid);
                    }
                }
                None
            }
        }
    }

    /// Draw the dotted background grid in canvas space so it pans/zooms with the board.
    fn draw_grid(&self, painter: &egui::Painter, rect: Rect, origin: Vec2, palette: &HsPalette) {
        let dot = palette.border.gamma_multiply(0.6);
        // Canvas-space bounds of the visible rect, so the grid is finite regardless of pan/zoom.
        let top_left = self.screen_to_canvas(rect.min, origin);
        let bottom_right = self.screen_to_canvas(rect.max, origin);
        let start_x = (top_left.x / GRID_STEP).floor() * GRID_STEP;
        let start_y = (top_left.y / GRID_STEP).floor() * GRID_STEP;
        let mut y = start_y;
        // Bound the loop so a degenerate zoom can never spin forever.
        let mut guard = 0u32;
        while y <= bottom_right.y && guard < 100_000 {
            let mut x = start_x;
            while x <= bottom_right.x && guard < 100_000 {
                let screen = self.canvas_to_screen(Pos2::new(x, y), origin);
                if rect.contains(screen) {
                    painter.circle_filled(screen, GRID_DOT_RADIUS, dot);
                }
                x += GRID_STEP;
                guard += 1;
            }
            y += GRID_STEP;
            guard += 1;
        }
    }

    /// Draw every visual edge as a dashed line between card centres (RISK-5 / MC-5: a short edge is a
    /// single solid line).
    fn draw_visual_edges(&self, painter: &egui::Painter, origin: Vec2, palette: &HsPalette) {
        let stroke = Stroke::new(2.0, palette.text_subtle.gamma_multiply(0.7));
        for edge in &self.visual_edges {
            let from = self.placements.iter().find(|p| p.placement_id == edge.from_placement_id);
            let to = self.placements.iter().find(|p| p.placement_id == edge.to_placement_id);
            if let (Some(from), Some(to)) = (from, to) {
                let a = self.canvas_to_screen(from.canvas_center(), origin);
                let b = self.canvas_to_screen(to.canvas_center(), origin);
                draw_dashed_line(painter, a, b, stroke);
            }
        }
    }

    /// Draw one placement card + its remove button + resize handle + (for a text card in edit mode) the
    /// inline editor + AccessKit nodes. Returns the typed event this card produced this frame: a
    /// `RemovePlacement` (remove click), a `ResizePlacement` (resize drag-stop, debounced to ONE per
    /// gesture — RISK-061-1), or an `EditTextCard` (inline-edit commit). `&mut self` because the resize
    /// handle accumulates the in-flight size and the inline editor owns a live buffer.
    fn draw_card(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        idx: usize,
        origin: Vec2,
        palette: &HsPalette,
    ) -> Option<CanvasEvent> {
        // Snapshot the immutable card geometry / identity up front so the rest of the body can take
        // `&mut self` for the resize accumulator + inline editor without overlapping borrows.
        let placement_id = self.placements[idx].placement_id.clone();
        let placed_block_id = self.placements[idx].placed_block_id.clone();
        let canvas_rect = self.placements[idx].canvas_rect();
        let screen_min = self.canvas_to_screen(canvas_rect.min, origin);
        let screen_max = self.canvas_to_screen(canvas_rect.max, origin);
        let screen_rect = Rect::from_min_max(screen_min, screen_max);

        let selected = self.selected.contains(&placement_id);
        let border = if selected {
            Stroke::new(2.0, palette.accent)
        } else {
            Stroke::new(1.0, palette.border_strong)
        };
        // White card fill from the theme surface (no hardcoded hex — theme invariant).
        painter.rect_filled(screen_rect, 4.0, palette.surface);
        painter.rect_stroke(screen_rect, 4.0, border, egui::StrokeKind::Inside);

        // Title (bold) + content_type (muted). The title is the LIVE block title (reference, not copy).
        let title = self.placements[idx].display_title().to_owned();
        painter.text(
            Pos2::new(screen_rect.left() + 8.0, screen_rect.top() + 6.0),
            egui::Align2::LEFT_TOP,
            &title,
            egui::FontId::proportional(13.0),
            palette.text,
        );
        if let Some(ct) = &self.placements[idx].live_content_type {
            painter.text(
                Pos2::new(screen_rect.left() + 8.0, screen_rect.top() + 24.0),
                egui::Align2::LEFT_TOP,
                ct,
                egui::FontId::proportional(11.0),
                palette.text_subtle,
            );
        }

        // WP-KERNEL-012 MT-032 (E5 "everything is a Loom block"): a `loom://` chip in the card footer
        // for any placement that maps to a real Loom block (RISK-3: skipped — no panic, no fabricated
        // URI — when `placed_block_id` is empty). The chip shows the full loom:// address; a resolved
        // content_hash adds a short ` #<8hex>` suffix (READ from the backend block, never written).
        let chip_text = self.placements[idx].loom_addr(&self.workspace_id).map(|addr| {
            let mut s = addr.to_uri();
            if let Some(hash) =
                self.placements[idx].loom_content_hash.as_deref().filter(|h| !h.trim().is_empty())
            {
                // CHAR-BOUNDARY SAFE: route the short prefix through ContentHash::short() rather than a
                // raw byte slice `&hash[..8]` — the backend hash is untrusted (from_backend does not
                // validate hex), so a multi-byte first char would otherwise panic the egui render thread.
                let short = crate::loom_address::ContentHash(hash.to_owned());
                s.push_str(&format!(" #{}", short.short()));
            }
            s
        });
        if let Some(chip) = &chip_text {
            painter.text(
                Pos2::new(screen_rect.left() + 8.0, screen_rect.bottom() - 16.0),
                egui::Align2::LEFT_TOP,
                chip,
                egui::FontId::monospace(10.0),
                palette.accent,
            );
        }

        // The card is an addressable Role::Group node (label = live title; group_id exposed as the
        // description so the AccessKit `data-group-id` is readable — AC6 / AC9). MT-032 also exposes the
        // `loom://` chip text as the node's DESCRIPTION so an out-of-process agent reads the placement's
        // loom address by stable id (HBR-SWARM); a non-addressable placement has no chip description.
        emit_placement_node(ui, &self.placements[idx], &title, chip_text.as_deref());

        // ── WP-KERNEL-012 MT-061: inline TEXT-CARD editor (AC-061-4). When THIS card is the one being
        // edited, render an egui TextEdit::multiline over the card body instead of waiting for a click;
        // commit on focus-loss / Ctrl+Enter, discard on Escape. Gated to text cards only (a block card is
        // never `editing_card_id`, so it never reaches here — reference-not-copy).
        let mut card_event: Option<CanvasEvent> = None;
        if self.editing_card_id.as_deref() == Some(placement_id.as_str()) {
            card_event = self.draw_inline_editor(ui, screen_rect, &placement_id, &placed_block_id, &title);
        }

        // ── WP-KERNEL-012 MT-061: bottom-right RESIZE handle (AC-061-1). A ~12px grab handle scaled by
        // the current zoom; dragging it updates the card's w/h live (optimistic, screen-delta / zoom), and
        // on drag-STOP fires ONE debounced ResizePlacement carrying the final clamped geometry. The handle
        // is suppressed while this card is being inline-edited (the editor owns the card body).
        if self.editing_card_id.as_deref() != Some(placement_id.as_str()) {
            if let Some(ev) = self.draw_resize_handle(ui, painter, idx, screen_rect, palette) {
                card_event = Some(ev);
            }
        }

        // Remove button ('x') at the card's top-right.
        let remove_size = Vec2::splat(18.0);
        let remove_rect = Rect::from_min_size(
            Pos2::new(screen_rect.right() - remove_size.x - 4.0, screen_rect.top() + 4.0),
            remove_size,
        );
        let remove_id = egui::Id::new(placement_remove_author_id(&placement_id));
        let remove_resp = ui.interact(remove_rect, remove_id, Sense::click());
        let remove_bg = if remove_resp.hovered() {
            palette.error_text.gamma_multiply(0.25)
        } else {
            palette.surface_strong
        };
        if ui.is_rect_visible(remove_rect) {
            painter.rect_filled(remove_rect, 3.0, remove_bg);
            painter.text(
                remove_rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(13.0),
                palette.text,
            );
        }
        emit_remove_node(ui, &remove_resp, &self.placements[idx]);

        if remove_resp.clicked() {
            return Some(CanvasEvent::RemovePlacement { placement_id });
        }
        card_event
    }

    /// WP-KERNEL-012 MT-061 (AC-061-1): the bottom-right resize handle for the card at `idx`. Returns ONE
    /// debounced [`CanvasEvent::ResizePlacement`] on drag-STOP (never per dragged() frame — RISK-061-1).
    /// During the drag, the card's w/h are updated optimistically (screen drag-delta / zoom, clamped to
    /// [`MIN_CARD_W`]x[`MIN_CARD_H`]) and tracked in `self.resizing` so the card renders live at the
    /// in-flight size. The handle always emits its `canvas.placement.{id}.resize` AccessKit node.
    fn draw_resize_handle(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        idx: usize,
        screen_rect: Rect,
        palette: &HsPalette,
    ) -> Option<CanvasEvent> {
        let placement_id = self.placements[idx].placement_id.clone();
        // The handle is a small square at the card's bottom-right, scaled by zoom so it tracks the card.
        let handle_px = RESIZE_HANDLE_PX * self.zoom;
        let handle_rect = Rect::from_min_max(
            Pos2::new(screen_rect.right() - handle_px, screen_rect.bottom() - handle_px),
            Pos2::new(screen_rect.right(), screen_rect.bottom()),
        );
        let handle_id = egui::Id::new(("canvas.placement.resize", &placement_id));
        let resp = ui.interact(handle_rect, handle_id, Sense::drag());

        // Hover cursor affordance (NW-SE diagonal resize).
        if resp.hovered() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeNwSe);
        }

        // Paint the grab affordance (a small accent triangle/handle) so the resize point is discoverable.
        if ui.is_rect_visible(handle_rect) {
            let grab = if resp.hovered() || resp.dragged() {
                palette.accent
            } else {
                palette.border_strong.gamma_multiply(0.8)
            };
            painter.rect_filled(handle_rect, 2.0, grab);
        }

        // Always emit the resize-handle AccessKit node (AC-061-6 / HBR-SWARM) so a swarm agent can drive
        // the resize by stable id even without a pointer.
        emit_resize_node(ui, &resp, &placement_id);

        let mut event = None;
        if resp.dragged() {
            // Optimistic in-flight resize: convert the SCREEN drag-delta to CANVAS units (/ zoom), add to
            // the card's current w/h, clamp to the minimum. Track it in `self.resizing` (debounce state).
            let delta = resp.drag_delta() / self.zoom;
            if let Some(card) = self.placements.get_mut(idx) {
                card.w = (card.w + delta.x).max(MIN_CARD_W);
                card.h = (card.h + delta.y).max(MIN_CARD_H);
                self.resizing = Some((placement_id.clone(), card.w, card.h));
                self.status = format!("Resizing {placement_id} to {:.0}x{:.0}", card.w, card.h);
            }
        }
        if resp.drag_stopped() {
            // DEBOUNCE (RISK-061-1 / MC-061-1): fire EXACTLY ONE ResizePlacement for the whole gesture,
            // with the final clamped geometry. The host PATCHes {w,h}, then getCanvasBoard reconciles.
            if let Some((id, w, h)) = self.resizing.take() {
                if id == placement_id {
                    self.status = format!("Resized {placement_id} to {w:.0}x{h:.0}");
                    event = Some(CanvasEvent::ResizePlacement { placement_id, w, h });
                }
            }
        }
        event
    }

    /// WP-KERNEL-012 MT-061 (AC-061-4): the in-place editor for a free-text card. Renders an
    /// `egui::TextEdit::multiline` over the card body bound to `self.editing_buffer`. Commits on
    /// focus-loss OR Ctrl+Enter (-> [`CanvasEvent::EditTextCard`] carrying the new title/body); discards
    /// on Escape (no server call). Returns the commit event, if any, this frame.
    fn draw_inline_editor(
        &mut self,
        ui: &mut egui::Ui,
        screen_rect: Rect,
        placement_id: &str,
        block_id: &str,
        title: &str,
    ) -> Option<CanvasEvent> {
        // The editor occupies the card body (below the title line), inset slightly from the border.
        let editor_rect = Rect::from_min_max(
            Pos2::new(screen_rect.left() + 6.0, screen_rect.top() + 22.0),
            Pos2::new(screen_rect.right() - 6.0, screen_rect.bottom() - 6.0),
        );
        let editor_id = egui::Id::new(("canvas.placement.editor", placement_id));

        // Escape discards the buffer and exits edit mode WITHOUT a server call (checked + CONSUMED before
        // building the widget so the editor never sees the key and the discard wins this frame).
        let escape = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if escape {
            self.editing_card_id = None;
            self.editing_buffer.clear();
            self.status = format!("Discarded edit of {placement_id}");
            return None;
        }

        // Ctrl/Cmd+Enter COMMITS. Consume it BEFORE the multiline TextEdit (which would otherwise insert a
        // newline), so the keyboard commit is deterministic.
        let ctrl_enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));

        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(editor_rect).layout(*ui.layout()),
        );
        let output = egui::TextEdit::multiline(&mut self.editing_buffer)
            .id(editor_id)
            .desired_width(editor_rect.width())
            .frame(false)
            .show(&mut child);
        let resp = output.response;
        // Focus the editor on the first frame it appears so typing lands without an extra click.
        if !resp.has_focus() && ui.memory(|m| m.focused().is_none()) {
            resp.request_focus();
        }

        // Commit triggers: Ctrl/Cmd+Enter (consumed above), OR the editor LOSES focus (clicked elsewhere).
        let lost_focus = resp.lost_focus();
        if ctrl_enter || lost_focus {
            let body = std::mem::take(&mut self.editing_buffer);
            self.editing_card_id = None;
            self.status = format!("Saved {placement_id}");
            return Some(CanvasEvent::EditTextCard {
                placement_id: placement_id.to_owned(),
                block_id: block_id.to_owned(),
                title: title.to_owned(),
                body,
            });
        }
        None
    }

    /// WP-KERNEL-012 MT-061: draw the derived section/group FRAMES behind the cards. Each frame is a
    /// translucent rounded rectangle (theme-token fill) with an opaque border + a title label at its
    /// top-left, plus a `canvas.section.{id}` AccessKit node (Role::Group, label = section title). Drawn
    /// FIRST in the canvas pass so cards paint on top (the frame reads as a container).
    fn draw_section_frames(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        layer: &crate::graph::canvas_sections::SectionLayer,
        origin: Vec2,
    ) {
        for frame in &layer.frames {
            let screen_min = self.canvas_to_screen(frame.rect.min, origin);
            let screen_max = self.canvas_to_screen(frame.rect.max, origin);
            let screen_rect = Rect::from_min_max(screen_min, screen_max);
            // Translucent fill (the section hue at low alpha) + an opaque border in the same hue.
            let fill = frame.color.gamma_multiply(0.12);
            painter.rect_filled(screen_rect, 6.0, fill);
            painter.rect_stroke(
                screen_rect,
                6.0,
                Stroke::new(1.5, frame.color),
                egui::StrokeKind::Inside,
            );
            // Title at the frame's top-left, inside the reserved title band.
            painter.text(
                Pos2::new(screen_rect.left() + 8.0, screen_rect.top() + 4.0),
                egui::Align2::LEFT_TOP,
                &frame.label,
                egui::FontId::proportional(12.0),
                frame.color,
            );
            emit_section_node(ui, frame);
        }
    }
}

/// Draw a manually-segmented dashed line A->B (egui has no native dash API). A line shorter than
/// [`MIN_DASH_LEN`] is drawn solid so the segment count never degenerates (RISK-5 / MC-5).
fn draw_dashed_line(painter: &egui::Painter, a: Pos2, b: Pos2, stroke: Stroke) {
    let delta = b - a;
    let len = delta.length();
    if len < MIN_DASH_LEN || len < f32::EPSILON {
        painter.line_segment([a, b], stroke);
        return;
    }
    let dir = delta / len;
    let mut pos = 0.0f32;
    while pos < len {
        let seg_start = a + dir * pos;
        let seg_end = a + dir * (pos + DASH_SOLID).min(len);
        painter.line_segment([seg_start, seg_end], stroke);
        pos += DASH_SOLID + DASH_GAP;
    }
}

/// Draw a centred overlay label (loading) over the canvas.
fn draw_overlay_label(painter: &egui::Painter, rect: Rect, text: &str, color: Color32, palette: &HsPalette) {
    let galley = painter.layout_no_wrap(text.to_owned(), egui::FontId::proportional(15.0), color);
    let pos = Pos2::new(
        rect.center().x - galley.size().x * 0.5,
        rect.center().y - galley.size().y * 0.5,
    );
    let pad = Vec2::new(8.0, 4.0);
    let bg_rect = Rect::from_min_size(pos - pad, galley.size() + pad * 2.0);
    painter.rect_filled(bg_rect, 4.0, palette.surface);
    painter.galley(pos, galley, color);
}

/// Emit a toolbar button's live AccessKit node (Role::Button + Action::Click + author_id).
fn emit_button_node(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
    });
}

/// Emit a Role::Status AccessKit node (zoom value label, status bar).
fn emit_status_node(ui: &egui::Ui, id: egui::Id, author_id: &str, value: &str) {
    let author = author_id.to_owned();
    let value = value.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Status);
        node.set_author_id(author.clone());
        node.set_value(value.clone());
        node.set_label(value.clone());
    });
}

/// Emit the MC-2 fallback text field's AccessKit node (Role::TextInput + author_id + current value) so
/// a swarm agent can type a block id and drive the `Place` button without OS drag.
fn emit_text_field_node(ui: &egui::Ui, id: egui::Id, author_id: &str, value: &str) {
    let author = author_id.to_owned();
    let value = value.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::TextInput);
        node.set_author_id(author.clone());
        node.set_value(value.clone());
    });
}

/// Emit a placement card's AccessKit node: Role::Group, label = live title, DefaultAction = select,
/// plus a `value` carrying the `group_id` (so the AccessKit `data-group-id` is readable — AC6). The
/// card is painter-drawn (no egui widget), so it gets a stable `egui::Id` from its author_id. When the
/// placement maps to a real Loom block (MT-032), its `loom://` chip text is exposed as the node's
/// DESCRIPTION so an out-of-process agent reads the placement's loom address (HBR-SWARM); a
/// non-addressable placement passes `None` and gets no description.
fn emit_placement_node(ui: &egui::Ui, card: &CanvasPlacementCard, label: &str, loom_chip: Option<&str>) {
    let author = placement_author_id(&card.placement_id);
    let id = egui::Id::new(&author);
    let label = label.to_owned();
    // Expose the group id (or "ungrouped") so a swarm agent / test can read the placement's
    // data-group-id via the AccessKit value field (AC6) — `set_value` is the crate's proven extra-data
    // channel (code_editor/editor_view.rs).
    let group_value = match &card.group_id {
        Some(g) => format!("group_id={g}"),
        None => "group_id=none".to_owned(),
    };
    let loom_chip = loom_chip.map(ToOwned::to_owned);
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(group_value.clone());
        if let Some(chip) = &loom_chip {
            node.set_description(chip.clone());
        }
        node.add_action(accesskit::Action::Click);
    });
}

/// An RFC3339-ish UTC timestamp from `SystemTime` (no chrono dependency — the crate's convention in
/// `app.rs` / `mcp/attribution.rs`). Used for the React-parity timestamp card title.
fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Days since epoch -> civil date (Howard Hinnant's algorithm), then HH:MM:SS for the time-of-day.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Emit a placement remove button's AccessKit node (Role::Button + Action::Click + author_id).
fn emit_remove_node(ui: &egui::Ui, resp: &egui::Response, card: &CanvasPlacementCard) {
    let author = placement_remove_author_id(&card.placement_id);
    let label = format!("Remove {}", card.display_title());
    ui.ctx().accesskit_node_builder(resp.id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
    });
}

/// WP-KERNEL-012 MT-061: emit a placement RESIZE handle's AccessKit node (Role::Button — a draggable grab
/// affordance — + Action::Click so a swarm agent can address it, + the `canvas.placement.{id}.resize`
/// author_id). AC-061-6 / HBR-SWARM.
fn emit_resize_node(ui: &egui::Ui, resp: &egui::Response, placement_id: &str) {
    let author = placement_resize_author_id(placement_id);
    let label = format!("Resize {placement_id}");
    ui.ctx().accesskit_node_builder(resp.id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
    });
}

/// WP-KERNEL-012 MT-061: emit a section/group FRAME's AccessKit node (Role::Group + the
/// `canvas.section.{id}` author_id + the section label as the accessible name + the group_id in `value`).
/// The frame is painter-drawn (no egui widget), so it gets a stable `egui::Id` from its author_id (the
/// same pattern as the placement card node). AC-061-2 / AC-061-6 / HBR-SWARM.
fn emit_section_node(ui: &egui::Ui, frame: &crate::graph::canvas_sections::SectionFrame) {
    let author = crate::graph::canvas_sections::section_author_id(&frame.id);
    let id = egui::Id::new(&author);
    let label = frame.label.clone();
    let value = format!("group_id={}", frame.id);
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(value.clone());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Origin used by the transform tests (the canvas rect's top-left).
    const ORIGIN: Vec2 = Vec2 { x: 12.0, y: 80.0 };

    /// MT-032 AC-5: a placement with a real `placed_block_id` is addressable as a `loom://` block; the
    /// chip uses the BOARD's workspace id + the placement's block id.
    #[test]
    fn placed_card_has_loom_addr_chip() {
        let card = CanvasPlacementCard::new("p-1", "blk-7", 0.0, 0.0, 200.0, 120.0);
        let addr = card.loom_addr("ws-9").expect("a placed block is addressable");
        assert_eq!(addr.to_uri(), "loom://ws-9/blk-7");
    }

    /// MT-032 RISK-3: a placement with an EMPTY `placed_block_id` is NOT addressable -> no chip, no
    /// fabricated loom:// URI, no panic.
    #[test]
    fn empty_placed_block_id_has_no_loom_chip() {
        let card = CanvasPlacementCard::new("p-1", "", 0.0, 0.0, 200.0, 120.0);
        assert_eq!(card.loom_addr("ws-9"), None, "no chip for an empty placed_block_id (RISK-3)");
        // An empty workspace also yields no chip (the board has no workspace yet).
        let card2 = CanvasPlacementCard::new("p-2", "blk-7", 0.0, 0.0, 200.0, 120.0);
        assert_eq!(card2.loom_addr(""), None, "no chip without a workspace");
    }

    fn board_with(n: usize) -> LoomCanvasBoard {
        let mut b = LoomCanvasBoard::new("ws-test", "canvas-1");
        let placements = (0..n)
            .map(|i| {
                let mut c = CanvasPlacementCard::new(
                    format!("p-{:03}", i + 1),
                    format!("block-{:03}", i + 1),
                    (i as f32) * 250.0 + 20.0,
                    40.0,
                    DEFAULT_CARD_W,
                    DEFAULT_CARD_H,
                );
                c.live_title = Some(format!("Block {}", i + 1));
                c.live_content_type = Some("note".to_owned());
                c
            })
            .collect();
        b.set_board(placements, vec![], Vec2::ZERO, 1.0);
        b
    }

    /// PROOF1 / MC-1: canvas_to_screen and screen_to_canvas are exact inverses (< 1px round-trip) across
    /// pan + non-unit zoom.
    #[test]
    fn transform_round_trips_under_1px() {
        let mut b = board_with(0);
        b.pan = Vec2::new(37.0, -19.0);
        b.zoom = 1.75;
        for &(cx, cy) in &[(0.0, 0.0), (100.0, 100.0), (-250.5, 640.0), (1920.0, 1080.0)] {
            let canvas = Pos2::new(cx, cy);
            let screen = b.canvas_to_screen(canvas, ORIGIN);
            let back = b.screen_to_canvas(screen, ORIGIN);
            assert!(
                (back.x - canvas.x).abs() < 1.0 && (back.y - canvas.y).abs() < 1.0,
                "PROOF1: round-trip must be < 1px (canvas {canvas:?} -> {back:?})"
            );
        }
    }

    /// MC-1: the drop position uses screen_to_canvas, the inverse used by hit-testing — a point dropped
    /// at a screen pos maps to a canvas pos that hit-tests back to the same screen pos region.
    #[test]
    fn drop_position_is_canvas_space() {
        let mut b = board_with(0);
        b.pan = Vec2::new(40.0, 40.0);
        b.zoom = 2.0;
        // A drop at screen (240, 240) with origin (12,80) maps to canvas ((240-12-40)/2, (240-80-40)/2).
        let canvas = b.screen_to_canvas(Pos2::new(240.0, 240.0), ORIGIN);
        assert!((canvas.x - 94.0).abs() < 0.01, "canvas x {} != 94", canvas.x);
        assert!((canvas.y - 60.0).abs() < 0.01, "canvas y {} != 60", canvas.y);
    }

    /// Card hit-test picks the topmost (highest z_index) overlapping card.
    #[test]
    fn hit_test_respects_z_order() {
        let mut b = board_with(0);
        let mut a = CanvasPlacementCard::new("p-low", "blk-a", 0.0, 0.0, 100.0, 100.0);
        a.z_index = 1;
        let mut top = CanvasPlacementCard::new("p-high", "blk-b", 0.0, 0.0, 100.0, 100.0);
        top.z_index = 5;
        b.set_board(vec![a, top], vec![], Vec2::ZERO, 1.0);
        let hit = b.placement_at_canvas(Pos2::new(50.0, 50.0)).unwrap();
        assert_eq!(b.placements[hit].placement_id, "p-high", "topmost z_index card wins the hit");
    }

    /// AC3: zoom buttons step ±0.25, clamp to [0.25, 4.0], and round to 2dp (matching the React label).
    #[test]
    fn zoom_steps_clamp_and_round() {
        let mut b = board_with(0);
        assert!((b.step_zoom(ZOOM_STEP) - 1.25).abs() < 1e-6);
        assert!((b.step_zoom(-ZOOM_STEP) - 1.0).abs() < 1e-6);
        // Clamp low.
        for _ in 0..20 {
            b.step_zoom(-ZOOM_STEP);
        }
        assert!((b.zoom - MIN_ZOOM).abs() < 1e-6, "clamped to MIN_ZOOM (got {})", b.zoom);
        // Clamp high.
        for _ in 0..40 {
            b.step_zoom(ZOOM_STEP);
        }
        assert!((b.zoom - MAX_ZOOM).abs() < 1e-6, "clamped to MAX_ZOOM (got {})", b.zoom);
    }

    /// Zoom-to-pointer keeps the canvas point under the cursor fixed.
    #[test]
    fn zoom_to_pointer_keeps_point_fixed() {
        let mut b = board_with(0);
        b.pan = Vec2::new(15.0, -8.0);
        let pointer = Pos2::new(300.0, 220.0);
        let canvas_before = b.screen_to_canvas(pointer, ORIGIN);
        assert!(b.apply_zoom(1.0, pointer, ORIGIN), "zoom must change");
        let screen_after = b.canvas_to_screen(canvas_before, ORIGIN);
        assert!(
            (screen_after.x - pointer.x).abs() < 0.5 && (screen_after.y - pointer.y).abs() < 0.5,
            "zoom-to-pointer must keep the canvas point under the cursor fixed (got {screen_after:?})"
        );
        // Clamp on repeated zoom-in.
        for _ in 0..50 {
            b.apply_zoom(1.0, pointer, ORIGIN);
        }
        assert!(b.zoom <= MAX_ZOOM + 1e-3, "clamped to MAX_ZOOM (got {})", b.zoom);
    }

    /// Reference-not-copy: a placement with no resolved live block shows "(stale reference)", never a
    /// content copy (AC1 / AC4).
    #[test]
    fn unresolved_placement_shows_stale_reference() {
        let stale = CanvasPlacementCard::new("p-x", "missing-block", 0.0, 0.0, 10.0, 10.0);
        assert_eq!(stale.display_title(), "(stale reference)");
        let mut resolved = stale.clone();
        resolved.live_title = Some("Real Title".to_owned());
        assert_eq!(resolved.display_title(), "Real Title");
    }

    /// AC7: edge_event builds a Semantic edge with the two BLOCK ids, and a Visual edge with the two
    /// PLACEMENT ids, per the active mode.
    #[test]
    fn edge_event_maps_mode_to_ids() {
        let mut b = board_with(2); // p-001/block-001, p-002/block-002
        b.edge_mode = EdgeMode::Semantic;
        assert_eq!(
            b.edge_event("p-001", "p-002"),
            Some(CanvasEvent::SemanticEdge {
                source_block_id: "block-001".to_owned(),
                target_block_id: "block-002".to_owned(),
            })
        );
        b.edge_mode = EdgeMode::Visual;
        assert_eq!(
            b.edge_event("p-001", "p-002"),
            Some(CanvasEvent::VisualEdgeAdded {
                from_placement_id: "p-001".to_owned(),
                to_placement_id: "p-002".to_owned(),
            })
        );
        // A missing placement yields None (defensive — no panic, no fabricated edge).
        assert_eq!(b.edge_event("p-001", "p-gone"), None);
    }

    /// set_board drops a selection / edge_from that references a removed placement (no dangling id after
    /// a reload).
    #[test]
    fn set_board_prunes_stale_selection() {
        let mut b = board_with(2);
        b.selected.insert("p-001".to_owned());
        b.selected.insert("p-removed".to_owned());
        b.edge_from = Some("p-removed".to_owned());
        // Reload with only p-001 present.
        let keep = b.placements[0].clone();
        b.set_board(vec![keep], vec![], Vec2::ZERO, 1.0);
        assert!(b.selected.contains("p-001"), "live id kept");
        assert!(!b.selected.contains("p-removed"), "stale id pruned");
        assert_eq!(b.edge_from, None, "stale edge_from cleared");
    }

    /// AC9 / author-id sanitization: a placement id with slashes/colons sanitizes to a `[a-z0-9-]`
    /// author_id suffix; the remove id is the placement id + ".remove".
    #[test]
    fn placement_author_ids_are_sanitized() {
        let id = placement_author_id("ws:1/p 7#x");
        assert!(id.starts_with(PLACEMENT_AUTHOR_ID_PREFIX));
        let suffix = &id[PLACEMENT_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
        assert_eq!(placement_remove_author_id("p-001"), "canvas.placement.p-001.remove");
    }

    /// AC10: an empty board is a no-op to render-prep — no placements, no panic on hit-test, no stale
    /// text.
    #[test]
    fn empty_board_has_no_cards() {
        let b = board_with(0);
        assert!(b.placements.is_empty());
        assert_eq!(b.placement_at_canvas(Pos2::new(10.0, 10.0)), None);
    }

    /// RED-TEAM CONTROL: the inter-panel drag payload MUST be `Send + Sync + 'static` for egui's
    /// DragAndDrop store (a compile error here is the gate — same control as `tab_bar`'s
    /// `TabDragPayload`).
    #[test]
    fn canvas_drag_payload_is_send_sync_static() {
        fn assert_send_sync_static<T: Send + Sync + 'static>() {}
        assert_send_sync_static::<CanvasDragPayload>();
    }

    /// MC-2 fallback math: `default_place_pos` returns the canvas-space centre of the visible rect
    /// (transform-correct), and the pre-render fallback is the React default `(40, 40)`.
    #[test]
    fn default_place_pos_is_visible_centre() {
        let mut b = board_with(0);
        // Before any render, the fallback is the React default (40, 40).
        assert_eq!(b.default_place_pos(), Pos2::new(40.0, 40.0));
        // After a render records the canvas rect, the default place is the rect centre in canvas space.
        b.pan = Vec2::new(20.0, -10.0);
        b.zoom = 2.0;
        let rect = Rect::from_min_size(Pos2::new(10.0, 80.0), Vec2::new(800.0, 500.0));
        b.last_canvas_rect = Some(rect);
        let pos = b.default_place_pos();
        // The returned canvas point must map back to the rect centre under the same transform (MC-1).
        let back = b.canvas_to_screen(pos, rect.min.to_vec2());
        assert!(
            (back.x - rect.center().x).abs() < 0.5 && (back.y - rect.center().y).abs() < 0.5,
            "default_place_pos must round-trip to the visible centre (got {pos:?} -> {back:?})"
        );
    }

    // ── WP-KERNEL-012 MT-061 lib-unit proofs (the widget-internal logic the kittest harness drives) ──

    /// AC-061-5 (reference-not-copy): a block-backed card's `card_kind` defaults to `BlockRef`, so it is
    /// NOT inline-editable; only a card explicitly marked `as_text_card` is a `TextCard`.
    #[test]
    fn card_kind_default_is_block_ref_and_gates_inline_edit() {
        let block_card = CanvasPlacementCard::new("p-1", "blk-7", 0.0, 0.0, 200.0, 120.0);
        assert_eq!(block_card.card_kind, CanvasCardKind::BlockRef, "default is a block reference");
        assert!(!block_card.card_kind.is_text_card(), "a block ref is NOT inline-editable");
        let text_card =
            CanvasPlacementCard::new("p-2", "blk-8", 0.0, 0.0, 200.0, 120.0).as_text_card("hello body");
        assert_eq!(text_card.card_kind, CanvasCardKind::TextCard, "as_text_card marks a TextCard");
        assert!(text_card.card_kind.is_text_card(), "a text card IS inline-editable");
        assert_eq!(text_card.live_body.as_deref(), Some("hello body"), "text card seeds its body");
    }

    /// AC-061-1 clamp: the resize handle clamps the card to the minimum size — w/h can never go below
    /// MIN_CARD_W x MIN_CARD_H regardless of how far the handle is dragged inward. (Drives the live resize
    /// math the kittest harness exercises with a real pointer drag.)
    #[test]
    fn resize_clamps_to_minimum() {
        // Simulate the in-flight resize math: a large negative delta cannot shrink below the minimum.
        let mut w: f32 = 200.0;
        let mut h: f32 = 120.0;
        // Apply a huge inward delta (as draw_resize_handle does: card.w = (card.w + delta).max(MIN)).
        w = (w + -1000.0).max(MIN_CARD_W);
        h = (h + -1000.0).max(MIN_CARD_H);
        assert_eq!(w, MIN_CARD_W, "resize clamps width to MIN_CARD_W");
        assert_eq!(h, MIN_CARD_H, "resize clamps height to MIN_CARD_H");
        // The minimums match the contract's 80x48 canvas-unit floor.
        assert_eq!((MIN_CARD_W, MIN_CARD_H), (80.0, 48.0), "minimums match the contract (80x48)");
    }

    /// AC-061-5 (the load-bearing invariant): a resize + section-assign cycle on a BLOCK-backed card never
    /// changes the set of underlying block_ids on the board (no block is duplicated/forked). We mutate the
    /// placement geometry + group like the widget does and assert the placed_block_id set is invariant.
    #[test]
    fn reference_not_copy_block_id_set_invariant_across_resize_and_section() {
        let mut b = board_with(2); // p-001/block-001, p-002/block-002 (both BlockRef by default)
        let before: std::collections::BTreeSet<String> =
            b.placements.iter().map(|p| p.placed_block_id.clone()).collect();
        // RESIZE p-001 (mutate ONLY the placement record's w/h).
        {
            let card = b.placements.iter_mut().find(|p| p.placement_id == "p-001").unwrap();
            card.w = 320.0;
            card.h = 200.0;
        }
        // SECTION-ASSIGN p-001 (mutate ONLY the placement record's group_id).
        {
            let card = b.placements.iter_mut().find(|p| p.placement_id == "p-001").unwrap();
            card.group_id = Some("g-research".to_owned());
        }
        let after: std::collections::BTreeSet<String> =
            b.placements.iter().map(|p| p.placed_block_id.clone()).collect();
        assert_eq!(before, after, "AC-061-5: the underlying block_id set is INVARIANT (no copy/fork)");
        // The count of placements is unchanged too (no duplicate placement created).
        assert_eq!(b.placements.len(), 2, "no placement duplicated by resize/section");
    }

    /// MC-061-2 rollback: after a resize PATCH FAILS, the host calls `rollback_placement_geometry` with
    /// the last server-confirmed geometry, reverting the optimistic w/h and clearing the in-flight state.
    #[test]
    fn rollback_restores_server_geometry_on_patch_failure() {
        let mut b = board_with(1); // p-001/block-001 at (20,40) 200x120
        // Optimistically resize p-001 (as a drag would) and mark it in-flight.
        {
            let card = b.placements.iter_mut().find(|p| p.placement_id == "p-001").unwrap();
            card.w = 400.0;
            card.h = 300.0;
        }
        b.resizing = Some(("p-001".to_owned(), 400.0, 300.0));
        // The PATCH fails -> roll back to the last server-confirmed geometry (200x120 at 20,40).
        let rolled = b.rollback_placement_geometry("p-001", 20.0, 40.0, 200.0, 120.0);
        assert!(rolled, "rollback found and reverted the placement");
        let card = b.placements.iter().find(|p| p.placement_id == "p-001").unwrap();
        assert_eq!((card.x, card.y, card.w, card.h), (20.0, 40.0, 200.0, 120.0), "geometry reverted");
        assert!(b.resizing.is_none(), "in-flight resize state cleared on rollback");
    }

    /// AssignSection event shape: clearing (drop outside all frames) carries `group_id: None`; assigning
    /// carries `Some(id)`. (The event the widget emits on a card-move drag-stop.)
    #[test]
    fn assign_section_event_shape() {
        let assign = CanvasEvent::AssignSection {
            placement_id: "p-1".to_owned(),
            group_id: Some("g-a".to_owned()),
        };
        let clear = CanvasEvent::AssignSection { placement_id: "p-1".to_owned(), group_id: None };
        match assign {
            CanvasEvent::AssignSection { group_id: Some(g), .. } => assert_eq!(g, "g-a"),
            _ => panic!("assign must carry Some(group_id)"),
        }
        match clear {
            CanvasEvent::AssignSection { group_id: None, .. } => {}
            _ => panic!("clear must carry None group_id"),
        }
    }

    /// AC-061-6: the resize-handle author_id extends (does not collide with) the placement card id.
    #[test]
    fn resize_author_id_extends_placement_id() {
        let card_id = placement_author_id("p-001");
        let resize_id = placement_resize_author_id("p-001");
        assert_eq!(resize_id, "canvas.placement.p-001.resize");
        assert!(resize_id.starts_with(&card_id), "resize id extends the card id (no collision)");
        assert_ne!(resize_id, card_id, "resize id is distinct from the card id");
        assert_ne!(resize_id, placement_remove_author_id("p-001"), "resize id != remove id");
    }
}
