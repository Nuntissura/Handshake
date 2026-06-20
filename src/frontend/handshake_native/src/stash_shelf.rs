//! Bottom drawer stash shelf for the native Handshake shell (WP-KERNEL-011 MT-023, C6).
//!
//! ## What this is (no-context model navigation — HBR-VIS / HBR-SWARM)
//!
//! A collapsible panel that slides up from the bottom of the shell, ABOVE the always-visible MT-022
//! bottom search rail. A small affordance tab pinned to the bottom-right corner is ALWAYS visible (open
//! or collapsed); clicking it toggles the drawer. When open, the drawer shows a horizontally-scrolling
//! shelf of four typed cards — Agenda, Mail, Lists, Notes — each with a title, a badge count, and a
//! one-line subtitle. The drawer top edge is draggable to resize the height (100..=480px).
//!
//! ## The drawer is a downstream CONSUMER that CALLS the backend (the MT-022 contrast)
//!
//! Unlike the MT-022 rail (which only EMITS a search intent and makes no backend call), the MT-023
//! drawer's contract requires it to FETCH live card data from the real PostgreSQL/EventLedger backend.
//! On open it fires three one-shot off-thread GET/PUTs through [`DrawerDataClient`] (the MT-009/020/021
//! off-thread + delivery-cell pattern) and folds the results into the cards. Mail makes NO backend call
//! (no mail backend exists yet).
//!
//! ### Verified endpoints (the MT-022 lesson: the contract's `binds_backend_api` was STALE)
//!
//! The contract named `/loom/views/table?content_type=list` and `/loom/views/calendar` returning
//! `{blocks,total}`. NONE of that exists in `handshake_core` (verified read-only):
//! - `parse_view_type` accepts only {all,unlinked,sorted,pins,favorites} — table/calendar → HTTP 400.
//! - `LoomViewResponse` is `#[serde(tag="view_type")]` with NO `total` field (count = `blocks.len()`).
//! - `content_type=list` is invalid (`LoomBlockContentType` has no `list` variant).
//!
//! So the REAL surfaces are used (see [`crate::backend_client::DrawerDataKind`]): Notes →
//! `GET /loom/views/all?content_type=note`; Lists → the saved block-collection views
//! (`content_type=view_def`, MT-262); Agenda → `PUT /loom/journals/{today}` (the daily journal, exactly
//! the source the contract's own `ports_from_react` names). Disclosed as MT-023 deviations.
//!
//! ## Panel ordering (RISK-023-A) — egui-correct for THIS codebase
//!
//! The contract's "ORDER: drawer -> rail -> central" assumes egui stacks the FIRST-registered bottom
//! panel lowest. In THIS codebase the OPPOSITE is true and proven: the MT-022 rail registers AFTER the
//! status bar and sits ABOVE it (`app.rs`: "egui stacks bottom panels in registration order, so this
//! sits just above the status strip"). So to put the drawer ABOVE the rail (its required position), the
//! drawer panel must register AFTER the rail. The host calls [`DrawerStashShelf::show`] AFTER
//! `drive_search_rail` — see the `// ORDER:` comment in `app.rs::ui`. This is the egui-correct order for
//! this codebase, not a contract violation (disclosed deviation).
//!
//! ## Stable AccessKit ids (out-of-process steering — HBR-VIS)
//!
//! Seven FIXED nodes in the fresh 32..=38 band (disjoint from every other declared identity, all
//! `< PANE_NODE_ID_BASE`), all enumerated in [`crate::accessibility::DECLARED_IDENTITIES`]:
//! - affordance tab ([`DRAWER_AFFORDANCE_NODE_ID`] = 32, Role::Button) — ALWAYS in the live tree.
//! - shelf container ([`DRAWER_SHELF_NODE_ID`] = 33, Role::Group) — only when open.
//! - Agenda/Mail/Lists/Notes cards (34..=37, Role::Button) — only when open.
//! - resize handle ([`DRAWER_RESIZE_NODE_ID`] = 38, Role::Slider) — only when open.

use egui::accesskit;

use crate::backend_client::DrawerCardData;

/// Fixed AccessKit/egui `NodeId` of the affordance tab (Role::Button). Fresh band slot 32. ALWAYS
/// visible (open or collapsed), so it is in the default-frame live tree every paint.
pub const DRAWER_AFFORDANCE_NODE_ID: u64 = 32;
/// Fixed `NodeId` of the shelf container (Role::Group). Fresh band slot 33. Open-only.
pub const DRAWER_SHELF_NODE_ID: u64 = 33;
/// Fixed `NodeId` base for the four cards (Agenda=34, Mail=35, Lists=36, Notes=37, all Role::Button).
pub const DRAWER_CARD_NODE_ID_BASE: u64 = 34;
/// Fixed `NodeId` of the resize handle (Role::Slider). Fresh band slot 38. Open-only.
pub const DRAWER_RESIZE_NODE_ID: u64 = 38;

/// The four card AccessKit node_ids in fixed logical order (Agenda, Mail, Lists, Notes), as a const
/// slice the collision registry can enumerate (a const slice cannot call the `node_id()` method).
pub const DRAWER_CARD_NODE_IDS: [u64; 4] = [
    DRAWER_CARD_NODE_ID_BASE,
    DRAWER_CARD_NODE_ID_BASE + 1,
    DRAWER_CARD_NODE_ID_BASE + 2,
    DRAWER_CARD_NODE_ID_BASE + 3,
];

/// The four card AccessKit author_ids in fixed logical order, as a const slice the collision registry
/// can enumerate (the `author_id()` method returns an owned `String`, not a `&'static str`).
pub const DRAWER_CARD_AUTHOR_IDS: [&str; 4] = [
    "hsk.drawer.card.agenda",
    "hsk.drawer.card.mail",
    "hsk.drawer.card.lists",
    "hsk.drawer.card.notes",
];

/// Stable out-of-process author_id for the affordance tab.
pub const DRAWER_AFFORDANCE_AUTHOR_ID: &str = "hsk.drawer.affordance";
/// Stable out-of-process author_id for the shelf container.
pub const DRAWER_SHELF_AUTHOR_ID: &str = "hsk.drawer.shelf";
/// Stable out-of-process author_id for the resize handle.
pub const DRAWER_RESIZE_AUTHOR_ID: &str = "hsk.drawer.resize_handle";

/// The default open height of the drawer (the contract's 220px default).
pub const DRAWER_DEFAULT_HEIGHT: f32 = 220.0;
/// The minimum resizable drawer height (AC-023-8).
pub const DRAWER_MIN_HEIGHT: f32 = 100.0;
/// The maximum resizable drawer height (AC-023-8).
pub const DRAWER_MAX_HEIGHT: f32 = 480.0;
/// The fixed card width (the contract's 180px).
pub const DRAWER_CARD_WIDTH: f32 = 180.0;

// ===========================================================================
// Card kind.
// ===========================================================================

/// The four typed cards on the shelf, in their fixed logical order (CONTROL-023-F: stored + traversed
/// in this order — Agenda, Mail, Lists, Notes — for a stable AccessKit tree; the visual right-alignment
/// is a layout choice, never a Vec reversal that would confuse swarm agents).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerCardKind {
    Agenda,
    Mail,
    Lists,
    Notes,
}

impl DrawerCardKind {
    /// All four kinds in fixed logical order.
    pub fn all() -> &'static [DrawerCardKind] {
        &[
            DrawerCardKind::Agenda,
            DrawerCardKind::Mail,
            DrawerCardKind::Lists,
            DrawerCardKind::Notes,
        ]
    }

    /// The display title (PROOF-023-1b: `"Agenda"`, `"Mail"`, `"Lists"`, `"Notes"`).
    pub fn title(self) -> &'static str {
        match self {
            DrawerCardKind::Agenda => "Agenda",
            DrawerCardKind::Mail => "Mail",
            DrawerCardKind::Lists => "Lists",
            DrawerCardKind::Notes => "Notes",
        }
    }

    /// The snake_case suffix for the card's AccessKit author_id (`hsk.drawer.card.{snake}`).
    pub fn snake(self) -> &'static str {
        match self {
            DrawerCardKind::Agenda => "agenda",
            DrawerCardKind::Mail => "mail",
            DrawerCardKind::Lists => "lists",
            DrawerCardKind::Notes => "notes",
        }
    }

    /// The card's stable AccessKit author_id (`hsk.drawer.card.{snake}`).
    pub fn author_id(self) -> String {
        format!("hsk.drawer.card.{}", self.snake())
    }

    /// The card's fixed AccessKit `NodeId` (DRAWER_CARD_NODE_ID_BASE + logical index).
    pub fn node_id(self) -> u64 {
        let idx = match self {
            DrawerCardKind::Agenda => 0,
            DrawerCardKind::Mail => 1,
            DrawerCardKind::Lists => 2,
            DrawerCardKind::Notes => 3,
        };
        DRAWER_CARD_NODE_ID_BASE + idx
    }

    /// A small unicode glyph for the card icon (no icon font dependency yet — contract note).
    pub fn glyph(self) -> &'static str {
        match self {
            DrawerCardKind::Agenda => "📅",
            DrawerCardKind::Mail => "✉",
            DrawerCardKind::Lists => "▤",
            DrawerCardKind::Notes => "🗒",
        }
    }

    /// Whether this card fetches live data from the backend. Mail does NOT (no mail backend — AC-023-7).
    pub fn fetches_data(self) -> bool {
        !matches!(self, DrawerCardKind::Mail)
    }
}

impl std::fmt::Display for DrawerCardKind {
    /// PROOF-023-1b: the Display value is the bare title (`"Agenda"` etc.).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.title())
    }
}

// ===========================================================================
// Card.
// ===========================================================================

/// One shelf card: its kind plus the live data state (badge / subtitle / loading / error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawerCard {
    pub kind: DrawerCardKind,
    /// Live badge count (0 until data loads / for the Mail placeholder).
    pub badge_count: u32,
    /// One-line subtitle (a count summary, the journal title, or a placeholder).
    pub subtitle: String,
    /// True while the one-shot fetch is in flight (shows a loading indicator).
    pub loading: bool,
    /// `Some(msg)` when the fetch failed (the card shows an error state without crashing — AC-023-4).
    pub error: Option<String>,
}

impl DrawerCard {
    /// A fresh card in its initial (pre-fetch) state. Mail is a static placeholder (badge 0, "Coming
    /// soon", never loading); the data cards start at badge 0 / "—" until the first fetch resolves.
    pub fn new(kind: DrawerCardKind) -> Self {
        match kind {
            DrawerCardKind::Mail => Self {
                kind,
                badge_count: 0,
                subtitle: "Coming soon".to_owned(),
                loading: false,
                error: None,
            },
            _ => Self {
                kind,
                badge_count: 0,
                subtitle: "—".to_owned(),
                loading: false,
                error: None,
            },
        }
    }

    /// Fold a delivered fetch result into the card: clears the loading flag, sets badge + subtitle on
    /// success, or sets the error string on failure (badge/subtitle left as-is so the card degrades
    /// visibly rather than blanking).
    pub fn apply_result(&mut self, result: Result<DrawerCardData, String>) {
        self.loading = false;
        match result {
            Ok(data) => {
                self.badge_count = data.badge_count;
                self.subtitle = data.subtitle;
                self.error = None;
            }
            Err(msg) => {
                self.error = Some(msg);
            }
        }
    }

    /// The AccessKit label the card exposes (`"{title} ({badge_count})"` per the contract).
    pub fn access_label(&self) -> String {
        format!("{} ({})", self.kind.title(), self.badge_count)
    }

    /// Render the card as a fixed-width frame at its STABLE AccessKit id (so its NodeId is stable across
    /// frames/restarts). Returns `true` if the card was clicked this frame. Click detection is a single
    /// `Sense::click()` on the card rect (CONTROL-023-E: never combined with a drag sense, so no
    /// double-fire). The body shows the kind glyph + title, a badge chip, and the subtitle / loading /
    /// error line.
    pub fn show(&self, ui: &mut egui::Ui, colors: DrawerColors) -> bool {
        let id = unsafe { egui::Id::from_high_entropy_bits(self.kind.node_id()) };
        let author_id = self.kind.author_id();
        let label = self.access_label();

        // Allocate the fixed-size card rect, then SENSE the click on the FIXED id (the codebase's
        // fixed-id-button pattern: allocate -> interact at the fixed id -> manual paint -> widget_info ->
        // node). This guarantees the interactive node and the stable-id node are the SAME node (a single
        // Role::Button at NodeId 34..=37), so the card has exactly one addressable AccessKit node — never
        // a duplicate at an auto-allocated id. Single `Sense::click()` only (CONTROL-023-E: no drag sense
        // on the same rect, so no double-fire).
        let height = ui.available_height().max(48.0);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(DRAWER_CARD_WIDTH, height), egui::Sense::hover());
        let resp = ui.interact(rect, id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let bg = if resp.hovered() {
                colors.badge_bg
            } else {
                colors.card_bg
            };
            ui.painter().rect_filled(rect, 6.0, bg);
            // Paint the card content inside the rect via a child UI (label/badge/subtitle), which is
            // PRESENTATIONAL only — egui labels are non-interactive text, so they add no extra clickable
            // node. The one interactive node is the fixed-id `resp` above.
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink(8.0))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            content.horizontal(|ui| {
                ui.label(egui::RichText::new(self.kind.glyph()).color(colors.card_text));
                ui.label(
                    egui::RichText::new(self.kind.title())
                        .color(colors.card_text)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::new()
                        .fill(colors.badge_bg)
                        .inner_margin(egui::Margin::symmetric(6, 2))
                        .corner_radius(8.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(self.badge_count.to_string())
                                    .color(colors.badge_text)
                                    .small(),
                            );
                        });
                });
            });
            let (line, color) = if self.loading {
                ("Loading…".to_owned(), colors.muted_text)
            } else if let Some(err) = &self.error {
                (format!("Error: {err}"), colors.error_text)
            } else {
                (self.subtitle.clone(), colors.muted_text)
            };
            content.label(egui::RichText::new(line).color(color).small());
        }

        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
        });
        // Stable author_id + Button role on the SAME fixed-id node `resp` lives on.
        ui.ctx().accesskit_node_builder(id, move |node| {
            node.set_role(accesskit::Role::Button);
            node.set_author_id(author_id);
            node.set_label(label);
        });
        // Mail: a "Coming soon" tooltip on hover (AC-023-7). The host treats a Mail CLICK as a tooltip,
        // not a navigation (DrawerEvent::MailTooltip); the hover text keeps the affordance discoverable.
        let resp = if self.kind == DrawerCardKind::Mail {
            resp.on_hover_text("Mail is not available yet")
        } else {
            resp
        };
        resp.clicked()
    }
}

/// Theme tokens the drawer paints with (from the active palette), so it flips dark<->light with the rest
/// of the shell.
#[derive(Debug, Clone, Copy)]
pub struct DrawerColors {
    pub panel_bg: egui::Color32,
    pub card_bg: egui::Color32,
    pub card_text: egui::Color32,
    pub muted_text: egui::Color32,
    pub badge_bg: egui::Color32,
    pub badge_text: egui::Color32,
    pub error_text: egui::Color32,
    pub affordance_bg: egui::Color32,
    pub affordance_hover_bg: egui::Color32,
    pub affordance_text: egui::Color32,
    pub resize_idle: egui::Color32,
    pub resize_hover: egui::Color32,
}

// ===========================================================================
// The drawer.
// ===========================================================================

/// What the host should do after a drawer frame: a card was activated (open its pane), the Mail tooltip
/// fired (no nav), or the affordance toggled the open state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawerEvent {
    /// The affordance tab was clicked: toggle the drawer open/closed.
    ToggleOpen,
    /// A data card (Agenda / Lists / Notes) was clicked: the host opens the corresponding pane view.
    OpenCard(DrawerCardKind),
    /// The Mail card was clicked: show a "Coming soon" tooltip, do NOT navigate (AC-023-7 / AC-023-12).
    MailTooltip,
}

/// The bottom drawer stash shelf. Owns its open flag, resizable height, the four cards, and the resize
/// drag state. The host owns the persisted `bottom_drawer_open` flag (MT-014 `drawers.bottom`) and the
/// off-thread fetch wiring; this widget renders + reports events.
#[derive(Debug, Clone)]
pub struct DrawerStashShelf {
    /// Resizable open height in px (default 220, clamped 100..=480, and re-clamped to the available
    /// window height each frame — CONTROL-023-B).
    pub height: f32,
    /// The four cards in fixed logical order (CONTROL-023-F).
    pub cards: Vec<DrawerCard>,
    /// `Some(start_height)` while the top edge is being dragged to resize; `None` otherwise.
    drag_resize_start: Option<f32>,
}

impl Default for DrawerStashShelf {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawerStashShelf {
    /// A fresh drawer: default height, the four cards in fixed order, not dragging.
    pub fn new() -> Self {
        Self {
            height: DRAWER_DEFAULT_HEIGHT,
            cards: DrawerCardKind::all().iter().map(|&k| DrawerCard::new(k)).collect(),
            drag_resize_start: None,
        }
    }

    /// Mutable access to a card by kind (host folds a delivered fetch result into it).
    pub fn card_mut(&mut self, kind: DrawerCardKind) -> Option<&mut DrawerCard> {
        self.cards.iter_mut().find(|c| c.kind == kind)
    }

    /// Read access to a card by kind (tests / debug surface).
    pub fn card(&self, kind: DrawerCardKind) -> Option<&DrawerCard> {
        self.cards.iter().find(|c| c.kind == kind)
    }

    /// Mark all data-fetching cards as loading (called by the host when it fires the fetches on open).
    pub fn mark_data_cards_loading(&mut self) {
        for card in &mut self.cards {
            if card.kind.fetches_data() {
                card.loading = true;
                card.error = None;
            }
        }
    }

    /// Clamp the height to the resizable range AND to the available window height so the drawer + rail +
    /// status bar can never exceed the window and collapse the CentralPanel (RISK-023-B / CONTROL-023-B).
    /// `reserved_below` is the px already claimed by the rail + status bar below the drawer.
    pub fn clamp_height(&mut self, available_window_height: f32, reserved_below: f32) {
        let ceiling = (available_window_height - reserved_below - 40.0).max(DRAWER_MIN_HEIGHT);
        let max = DRAWER_MAX_HEIGHT.min(ceiling);
        self.height = self.height.clamp(DRAWER_MIN_HEIGHT, max.max(DRAWER_MIN_HEIGHT));
    }

    /// Render the affordance tab as a bottom-right anchored overlay button (ALWAYS visible, open or
    /// collapsed — AC-023-1). Uses the codebase's fixed-id button pattern so its AccessKit NodeId is
    /// stable. Returns `true` if clicked. `open` flips the glyph so a model can read the current state.
    pub fn show_affordance(&self, ctx: &egui::Context, open: bool, colors: DrawerColors) -> bool {
        let id = unsafe { egui::Id::from_high_entropy_bits(DRAWER_AFFORDANCE_NODE_ID) };
        let glyph = if open { "▼ stash" } else { "▲ stash" };
        let mut clicked = false;
        // Anchored above the 32px rail + ~24px status bar so it never overlaps them.
        egui::Area::new(egui::Id::new("hsk.drawer.affordance_area"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-8.0, -64.0))
            .order(egui::Order::Foreground)
            // The Area's own background must NOT be sensed: an interactable Area registers an anonymous
            // (role Unknown, Action::Click) AccessKit node that trips the MT-025 interactive-naming gate
            // (the same pitfall the MT-022 rail avoided with its ScrollArea). Only the explicit
            // `ui.interact` button below is interactive, and it carries the stable author_id.
            .interactable(false)
            .show(ctx, |ui| {
                let g = ui.painter().layout_no_wrap(
                    glyph.to_owned(),
                    egui::FontId::proportional(12.0),
                    colors.affordance_text,
                );
                let size = egui::vec2(g.size().x + 16.0, 24.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let resp = ui
                    .interact(rect, id, egui::Sense::click())
                    .on_hover_text(if open { "Close the stash drawer" } else { "Open the stash drawer" });
                if ui.is_rect_visible(rect) {
                    let bg = if resp.hovered() {
                        colors.affordance_hover_bg
                    } else {
                        colors.affordance_bg
                    };
                    ui.painter().rect_filled(rect, 4.0, bg);
                    let g2 = ui.painter().layout_no_wrap(
                        glyph.to_owned(),
                        egui::FontId::proportional(12.0),
                        colors.affordance_text,
                    );
                    ui.painter().galley(
                        egui::pos2(
                            rect.center().x - g2.size().x * 0.5,
                            rect.center().y - g2.size().y * 0.5,
                        ),
                        g2,
                        colors.affordance_text,
                    );
                }
                let aria = if open { "Close stash drawer" } else { "Open stash drawer" };
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), aria)
                });
                ui.ctx().accesskit_node_builder(id, |node| {
                    node.set_role(accesskit::Role::Button);
                    node.set_author_id(DRAWER_AFFORDANCE_AUTHOR_ID.to_owned());
                    node.set_label(aria.to_owned());
                });
                clicked = resp.clicked();
            });
        clicked
    }

    /// Render the open drawer panel INSIDE an already-opened bottom panel `ui` (the host registers the
    /// `TopBottomPanel::bottom("hsk.drawer")` AFTER the rail panel so the drawer stacks ABOVE the rail in
    /// this codebase's registration-order convention — see the module docs). Returns the first card event
    /// produced this frame (None if no card was clicked). The resize handle drag updates `self.height`.
    pub fn show_open_panel(&mut self, ui: &mut egui::Ui, colors: DrawerColors) -> Option<DrawerEvent> {
        // ── Resize handle: a 4px draggable strip at the very top (AC-023-8). ──
        let resize_id = unsafe { egui::Id::from_high_entropy_bits(DRAWER_RESIZE_NODE_ID) };
        let (resize_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 4.0), egui::Sense::hover());
        let resize_resp = ui.interact(resize_rect, resize_id, egui::Sense::drag());
        if resize_resp.drag_started() {
            self.drag_resize_start = Some(self.height);
        }
        if resize_resp.dragged() {
            // Dragging UP (negative dy) grows the drawer; clamp to the resizable range.
            self.height = (self.height - resize_resp.drag_delta().y)
                .clamp(DRAWER_MIN_HEIGHT, DRAWER_MAX_HEIGHT);
        }
        if resize_resp.drag_stopped() {
            self.drag_resize_start = None;
        }
        if ui.is_rect_visible(resize_rect) {
            let fill = if resize_resp.hovered() || resize_resp.dragged() {
                colors.resize_hover
            } else {
                colors.resize_idle
            };
            ui.painter().rect_filled(resize_rect, 0.0, fill);
        }
        if resize_resp.hovered() || resize_resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        // Resize handle as a stable Role::Slider node (the semantically-closest role for a resize edge).
        let height_now = self.height;
        resize_resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, ui.is_enabled(), "Resize stash drawer")
        });
        ui.ctx().accesskit_node_builder(resize_id, move |node| {
            node.set_role(accesskit::Role::Slider);
            node.set_author_id(DRAWER_RESIZE_AUTHOR_ID.to_owned());
            node.set_label("Resize stash drawer".to_owned());
            node.add_action(accesskit::Action::SetValue);
            node.set_numeric_value(height_now as f64);
            node.set_min_numeric_value(DRAWER_MIN_HEIGHT as f64);
            node.set_max_numeric_value(DRAWER_MAX_HEIGHT as f64);
        });

        // ── Shelf container node (Role::Group) at a stable id, wrapping the horizontal card row. ──
        let shelf_id = unsafe { egui::Id::from_high_entropy_bits(DRAWER_SHELF_NODE_ID) };
        ui.ctx().accesskit_node_builder(shelf_id, |node| {
            node.set_role(accesskit::Role::Group);
            node.set_author_id(DRAWER_SHELF_AUTHOR_ID.to_owned());
            node.set_label("Stash shelf".to_owned());
        });

        let mut event = None;
        // Horizontal scroll shelf. CONTROL-023-F: cards render right-to-left for the right-aligned
        // visual (Agenda rightmost) WITHOUT reversing the Vec — the AccessKit tree keeps the logical
        // Agenda→Notes order, so swarm agents read a stable card sequence.
        egui::ScrollArea::horizontal()
            .id_salt("hsk.drawer.scroll")
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    for card in &self.cards {
                        if card.show(ui, colors) {
                            event = Some(match card.kind {
                                DrawerCardKind::Mail => DrawerEvent::MailTooltip,
                                other => DrawerEvent::OpenCard(other),
                            });
                        }
                        ui.add_space(8.0);
                    }
                });
            });
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AccessKit ids: fresh disjoint band 32..=38, all below the pane base ────────────────────────

    #[test]
    fn drawer_ids_in_disjoint_fresh_band() {
        let ids = [
            DRAWER_AFFORDANCE_NODE_ID,
            DRAWER_SHELF_NODE_ID,
            DrawerCardKind::Agenda.node_id(),
            DrawerCardKind::Mail.node_id(),
            DrawerCardKind::Lists.node_id(),
            DrawerCardKind::Notes.node_id(),
            DRAWER_RESIZE_NODE_ID,
        ];
        for id in ids {
            assert!((32..=38).contains(&id), "drawer id {id} in band 32..=38");
            assert!(
                id < crate::accessibility::PANE_NODE_ID_BASE,
                "drawer id {id} below the pane id base"
            );
        }
        // All seven are distinct.
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            assert!(seen.insert(id), "drawer id {id} is unique");
        }
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn card_ids_are_sequential_in_logical_order() {
        assert_eq!(DrawerCardKind::Agenda.node_id(), 34);
        assert_eq!(DrawerCardKind::Mail.node_id(), 35);
        assert_eq!(DrawerCardKind::Lists.node_id(), 36);
        assert_eq!(DrawerCardKind::Notes.node_id(), 37);
    }

    // ── Card kind contract (PROOF-023-1b) ──────────────────────────────────────────────────────────

    #[test]
    fn card_kind_display_titles() {
        assert_eq!(DrawerCardKind::Agenda.to_string(), "Agenda");
        assert_eq!(DrawerCardKind::Mail.to_string(), "Mail");
        assert_eq!(DrawerCardKind::Lists.to_string(), "Lists");
        assert_eq!(DrawerCardKind::Notes.to_string(), "Notes");
    }

    #[test]
    fn card_kinds_in_fixed_order() {
        let all = DrawerCardKind::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], DrawerCardKind::Agenda);
        assert_eq!(all[1], DrawerCardKind::Mail);
        assert_eq!(all[2], DrawerCardKind::Lists);
        assert_eq!(all[3], DrawerCardKind::Notes);
    }

    #[test]
    fn const_card_arrays_match_method_values() {
        // The registry enumerates the const arrays (a const slice cannot call the methods); this proves
        // the arrays cannot silently drift from the per-kind methods.
        for (i, &kind) in DrawerCardKind::all().iter().enumerate() {
            assert_eq!(kind.node_id(), DRAWER_CARD_NODE_IDS[i], "{kind} node_id matches const");
            assert_eq!(kind.author_id(), DRAWER_CARD_AUTHOR_IDS[i], "{kind} author_id matches const");
        }
    }

    #[test]
    fn card_author_ids_are_snake_case() {
        assert_eq!(DrawerCardKind::Agenda.author_id(), "hsk.drawer.card.agenda");
        assert_eq!(DrawerCardKind::Mail.author_id(), "hsk.drawer.card.mail");
        assert_eq!(DrawerCardKind::Lists.author_id(), "hsk.drawer.card.lists");
        assert_eq!(DrawerCardKind::Notes.author_id(), "hsk.drawer.card.notes");
    }

    #[test]
    fn only_mail_skips_data_fetch() {
        assert!(!DrawerCardKind::Mail.fetches_data());
        assert!(DrawerCardKind::Agenda.fetches_data());
        assert!(DrawerCardKind::Lists.fetches_data());
        assert!(DrawerCardKind::Notes.fetches_data());
    }

    // ── Card state machine ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn fresh_mail_card_is_static_placeholder() {
        let c = DrawerCard::new(DrawerCardKind::Mail);
        assert_eq!(c.badge_count, 0);
        assert_eq!(c.subtitle, "Coming soon");
        assert!(!c.loading);
        assert!(c.error.is_none());
    }

    #[test]
    fn apply_ok_result_sets_badge_and_clears_loading() {
        let mut c = DrawerCard::new(DrawerCardKind::Notes);
        c.loading = true;
        c.apply_result(Ok(DrawerCardData { badge_count: 7, subtitle: "7 items".to_owned() }));
        assert_eq!(c.badge_count, 7);
        assert_eq!(c.subtitle, "7 items");
        assert!(!c.loading);
        assert!(c.error.is_none());
    }

    #[test]
    fn apply_err_result_sets_error_without_crashing() {
        let mut c = DrawerCard::new(DrawerCardKind::Lists);
        c.loading = true;
        c.apply_result(Err("backend down".to_owned()));
        assert!(!c.loading);
        assert_eq!(c.error.as_deref(), Some("backend down"));
    }

    #[test]
    fn access_label_is_title_with_badge() {
        let mut c = DrawerCard::new(DrawerCardKind::Agenda);
        c.badge_count = 3;
        assert_eq!(c.access_label(), "Agenda (3)");
    }

    // ── Shelf defaults + height clamping (RISK-023-B / CONTROL-023-B) ───────────────────────────────

    #[test]
    fn fresh_shelf_has_four_cards_in_order_at_default_height() {
        let shelf = DrawerStashShelf::new();
        assert_eq!(shelf.height, DRAWER_DEFAULT_HEIGHT);
        assert_eq!(shelf.cards.len(), 4);
        let kinds: Vec<_> = shelf.cards.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            vec![
                DrawerCardKind::Agenda,
                DrawerCardKind::Mail,
                DrawerCardKind::Lists,
                DrawerCardKind::Notes
            ]
        );
    }

    #[test]
    fn clamp_height_keeps_room_for_central_panel() {
        let mut shelf = DrawerStashShelf::new();
        shelf.height = 460.0;
        // Tiny window: 200px tall, 56px reserved below (rail+status). The drawer must shrink so the
        // CentralPanel never collapses (RISK-023-B).
        shelf.clamp_height(200.0, 56.0);
        assert!(shelf.height <= 200.0 - 56.0, "drawer fits within the window");
        assert!(shelf.height >= DRAWER_MIN_HEIGHT, "but never below the minimum");
    }

    #[test]
    fn clamp_height_respects_max_on_a_large_window() {
        let mut shelf = DrawerStashShelf::new();
        shelf.height = 9999.0;
        shelf.clamp_height(2000.0, 56.0);
        assert_eq!(shelf.height, DRAWER_MAX_HEIGHT, "clamped to the 480px max on a big window");
    }

    #[test]
    fn mark_data_cards_loading_skips_mail() {
        let mut shelf = DrawerStashShelf::new();
        shelf.mark_data_cards_loading();
        assert!(shelf.card(DrawerCardKind::Agenda).unwrap().loading);
        assert!(shelf.card(DrawerCardKind::Lists).unwrap().loading);
        assert!(shelf.card(DrawerCardKind::Notes).unwrap().loading);
        assert!(!shelf.card(DrawerCardKind::Mail).unwrap().loading, "mail never loads");
    }
}
