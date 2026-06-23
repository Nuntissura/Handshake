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

use egui::accesskit;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

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

/// The stable AccessKit author_id for a placement card, sanitizing `placement_id` to `[a-z0-9-]` so a
/// raw id with slashes/colons can never break tree integrity (reuses the shell's slugger).
pub fn placement_author_id(placement_id: &str) -> String {
    format!("{PLACEMENT_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(placement_id))
}

/// The stable AccessKit author_id for a placement card's remove button.
pub fn placement_remove_author_id(placement_id: &str) -> String {
    format!("{}.remove", placement_author_id(placement_id))
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
        }
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
    /// Remove a placement reference (`DELETE .../canvas-placements/:id`). Source block is KEPT.
    RemovePlacement { placement_id: String },
    /// Create a real semantic Loom edge (`POST /loom/edges {edge_type:"mention"}`) between two BLOCKS.
    SemanticEdge { source_block_id: String, target_block_id: String },
    /// Create a board-local visual edge (`POST .../visual-edges`) between two PLACEMENTS.
    VisualEdgeAdded { from_placement_id: String, to_placement_id: String },
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
    /// The last canvas surface rect (screen space), recorded each frame so the MC-2 fallback can place
    /// a card at the centre of the currently-visible canvas. `None` until the board has rendered once.
    last_canvas_rect: Option<Rect>,
    /// Group-id counter so the `Group` event always gets a unique id even within one process run.
    group_seq: u64,
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
            last_canvas_rect: None,
            group_seq: 0,
        }
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
        ui.horizontal(|ui| {
            let pan_left = ui.button("◀ Pan");
            emit_button_node(ui, pan_left.id, PAN_LEFT_AUTHOR_ID, "Pan left");
            if pan_left.clicked() {
                self.pan.x -= PAN_STEP;
                event = Some(self.viewport_event());
            }
            let pan_right = ui.button("Pan ▶");
            emit_button_node(ui, pan_right.id, PAN_RIGHT_AUTHOR_ID, "Pan right");
            if pan_right.clicked() {
                self.pan.x += PAN_STEP;
                event = Some(self.viewport_event());
            }

            ui.separator();
            let zoom_out = ui.button("−");
            emit_button_node(ui, zoom_out.id, ZOOM_OUT_AUTHOR_ID, "Zoom out");
            if zoom_out.clicked() {
                self.step_zoom(-ZOOM_STEP);
                event = Some(self.viewport_event());
            }
            let zoom_label = format!("{:.2}x", self.zoom);
            let zlabel = ui.label(&zoom_label);
            emit_status_node(ui, zlabel.id, ZOOM_VALUE_AUTHOR_ID, &zoom_label);
            let zoom_in = ui.button("+");
            emit_button_node(ui, zoom_in.id, ZOOM_IN_AUTHOR_ID, "Zoom in");
            if zoom_in.clicked() {
                self.step_zoom(ZOOM_STEP);
                event = Some(self.viewport_event());
            }

            ui.separator();
            let add_card = ui.button("+ Text card");
            emit_button_node(ui, add_card.id, ADD_CARD_AUTHOR_ID, "Add text card");
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
            let place_btn = ui.add_enabled(can_place, egui::Button::new("Place"));
            emit_button_node(ui, place_btn.id, PLACE_BLOCK_AUTHOR_ID, "Place block by id");
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

        // Background fill + dotted grid (canvas is never blank/white — PROOF6).
        painter.rect_filled(rect, 0.0, palette.bg);
        self.draw_grid(&painter, rect, origin, palette);

        // ── AC4 / PROOF3: drop-to-place. A Loom block dragged from another panel via egui's
        // DragAndDrop channel (payload [`CanvasDragPayload`], the native peer of the React
        // CANVAS_DRAG_MIME `dataTransfer`) and RELEASED over the canvas places a REFERENCE card. The
        // drop position is computed in CANVAS space with the SAME screen_to_canvas inverse used by
        // hit-testing (RISK-1 / MC-1), exactly mirroring the React `(clientX-rect.left-pan.x)/zoom`.
        if let Some(payload) = canvas_resp.dnd_release_payload::<CanvasDragPayload>() {
            let drop_screen = canvas_resp
                .interact_pointer_pos()
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
                .unwrap_or_else(|| rect.center());
            let canvas_pos = self.screen_to_canvas(drop_screen, origin);
            self.status = format!("Placed {} (reference)", payload.block_id);
            event = Some(CanvasEvent::PlaceBlock {
                placed_block_id: payload.block_id.clone(),
                x: canvas_pos.x,
                y: canvas_pos.y,
            });
        }

        // Pointer input: zoom (scroll), pan (drag on empty area), card click (select / edge).
        if let Some(pointer) = canvas_resp.hover_pos() {
            let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_y != 0.0 && self.apply_zoom(scroll_y.signum(), pointer, origin) {
                // Persist on the gesture (the host debounces — RISK-3 / MC-3).
                event = Some(self.viewport_event());
            }
        }

        // Drag on empty canvas pans; a drag that began over a card is ignored (card drag is not in this
        // MT's scope). Persist the viewport on release.
        if canvas_resp.dragged() {
            let over_card = canvas_resp
                .interact_pointer_pos()
                .map(|p| self.placement_at_canvas(self.screen_to_canvas(p, origin)).is_some())
                .unwrap_or(false);
            if !over_card {
                self.pan += canvas_resp.drag_delta();
            }
        }
        if canvas_resp.drag_stopped() {
            event = Some(self.viewport_event());
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

        // Visual edges first (so cards render on top).
        self.draw_visual_edges(&painter, origin, palette);

        // Placement cards (+ remove button + AccessKit). Drawn in ascending z-order so the topmost card
        // paints last.
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

        event
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

    /// Draw one placement card + its remove button + AccessKit nodes. Returns a `RemovePlacement` event
    /// if the remove button was clicked this frame.
    fn draw_card(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        idx: usize,
        origin: Vec2,
        palette: &HsPalette,
    ) -> Option<CanvasEvent> {
        let card = &self.placements[idx];
        let canvas_rect = card.canvas_rect();
        let screen_min = self.canvas_to_screen(canvas_rect.min, origin);
        let screen_max = self.canvas_to_screen(canvas_rect.max, origin);
        let screen_rect = Rect::from_min_max(screen_min, screen_max);

        let selected = self.selected.contains(&card.placement_id);
        let border = if selected {
            Stroke::new(2.0, palette.accent)
        } else {
            Stroke::new(1.0, palette.border_strong)
        };
        // White card fill from the theme surface (no hardcoded hex — theme invariant).
        painter.rect_filled(screen_rect, 4.0, palette.surface);
        painter.rect_stroke(screen_rect, 4.0, border, egui::StrokeKind::Inside);

        // Title (bold) + content_type (muted). The title is the LIVE block title (reference, not copy).
        let title = card.display_title().to_owned();
        painter.text(
            Pos2::new(screen_rect.left() + 8.0, screen_rect.top() + 6.0),
            egui::Align2::LEFT_TOP,
            &title,
            egui::FontId::proportional(13.0),
            palette.text,
        );
        if let Some(ct) = &card.live_content_type {
            painter.text(
                Pos2::new(screen_rect.left() + 8.0, screen_rect.top() + 24.0),
                egui::Align2::LEFT_TOP,
                ct,
                egui::FontId::proportional(11.0),
                palette.text_subtle,
            );
        }

        // The card is an addressable Role::Group node (label = live title; group_id exposed as the
        // description so the AccessKit `data-group-id` is readable — AC6 / AC9).
        emit_placement_node(ui, card, &title);

        // Remove button ('x') at the card's top-right.
        let remove_size = Vec2::splat(18.0);
        let remove_rect = Rect::from_min_size(
            Pos2::new(screen_rect.right() - remove_size.x - 4.0, screen_rect.top() + 4.0),
            remove_size,
        );
        let remove_id = egui::Id::new(placement_remove_author_id(&card.placement_id));
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
        emit_remove_node(ui, &remove_resp, card);

        if remove_resp.clicked() {
            return Some(CanvasEvent::RemovePlacement { placement_id: card.placement_id.clone() });
        }
        None
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
/// card is painter-drawn (no egui widget), so it gets a stable `egui::Id` from its author_id.
fn emit_placement_node(ui: &egui::Ui, card: &CanvasPlacementCard, label: &str) {
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
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(group_value.clone());
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Origin used by the transform tests (the canvas rect's top-left).
    const ORIGIN: Vec2 = Vec2 { x: 12.0, y: 80.0 };

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
}
