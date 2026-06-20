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

/// Fixed `NodeId` base for the four per-card overflow buttons (MT-024, Role::Button). Fresh band
/// 44..=47 (Agenda=44, Mail=45, Lists=46, Notes=47) — disjoint from the drawer 32..=38 band and every
/// other declared identity, all open-only. The `...` overflow button opens the typed action menu.
pub const DRAWER_OVERFLOW_NODE_ID_BASE: u64 = 44;

/// The four overflow-button AccessKit node_ids in fixed logical order, as a const slice the collision
/// registry can enumerate.
pub const DRAWER_OVERFLOW_NODE_IDS: [u64; 4] = [
    DRAWER_OVERFLOW_NODE_ID_BASE,
    DRAWER_OVERFLOW_NODE_ID_BASE + 1,
    DRAWER_OVERFLOW_NODE_ID_BASE + 2,
    DRAWER_OVERFLOW_NODE_ID_BASE + 3,
];

/// The four overflow-button AccessKit author_ids in fixed logical order (`hsk.drawer.card.{kind}.overflow`).
pub const DRAWER_OVERFLOW_AUTHOR_IDS: [&str; 4] = [
    "hsk.drawer.card.agenda.overflow",
    "hsk.drawer.card.mail.overflow",
    "hsk.drawer.card.lists.overflow",
    "hsk.drawer.card.notes.overflow",
];

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

    /// The fixed logical index (0..=3) of this card kind (Agenda=0, Mail=1, Lists=2, Notes=3).
    fn logical_index(self) -> usize {
        match self {
            DrawerCardKind::Agenda => 0,
            DrawerCardKind::Mail => 1,
            DrawerCardKind::Lists => 2,
            DrawerCardKind::Notes => 3,
        }
    }

    /// The card's overflow-button stable AccessKit author_id (`hsk.drawer.card.{snake}.overflow`).
    pub fn overflow_author_id(self) -> &'static str {
        DRAWER_OVERFLOW_AUTHOR_IDS[self.logical_index()]
    }

    /// The card's overflow-button fixed AccessKit `NodeId` (DRAWER_OVERFLOW_NODE_ID_BASE + index).
    pub fn overflow_node_id(self) -> u64 {
        DRAWER_OVERFLOW_NODE_IDS[self.logical_index()]
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
// Card actions (MT-024, C6).
// ===========================================================================

/// The eight typed actions a drawer card's overflow menu offers (MT-024). The PERSISTING actions
/// (Stow/Pin/Discard/AttachEvidence) route through [`crate::backend_client::DrawerActionClient`]; the
/// LOCAL actions (Promote/SendToPane/CopyToPrompt) are pure AppState/clipboard mutations with NO backend
/// call. `ConvertArtifact` has NO backend surface (no content_type PATCH field / endpoint exists), so it
/// renders as a DISABLED V1 item and never produces an event (disclosed deviation; matches the MT-021
/// `convert_artifact` stub). The id snake-case suffix is the AccessKit action-id segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerCardAction {
    /// Archive the block by tagging it into the workspace `stash` TagHub (backend: POST /loom/edges).
    Stow,
    /// Bring the block to the top of the Pins grid (backend: PUT /loom/blocks/:id/pin-order).
    Pin,
    /// Promote the stashed block into the active pane (LOCAL: writes a PromoteIntent; no backend).
    Promote,
    /// Send the block to a chosen open pane (LOCAL: writes a SendToPaneIntent; no backend).
    SendToPane,
    /// Copy a coder-prompt string built from the card to the clipboard (LOCAL: egui clipboard).
    CopyToPrompt,
    /// Record the block as an evidence diagnostic on the active job (backend: POST /diagnostics).
    AttachEvidence,
    /// Convert the block into an artifact. NO backend surface exists — disabled V1 item.
    ConvertArtifact,
    /// Delete the block (backend: DELETE /loom/blocks/:id). Gated behind a confirm dialog (HBR-STOP).
    Discard,
}

impl DrawerCardAction {
    /// All eight actions in fixed menu order (Stow, Pin, Promote, Send to pane, Copy to prompt, Attach
    /// evidence, Convert to artifact, Discard).
    pub fn all() -> &'static [DrawerCardAction] {
        &[
            DrawerCardAction::Stow,
            DrawerCardAction::Pin,
            DrawerCardAction::Promote,
            DrawerCardAction::SendToPane,
            DrawerCardAction::CopyToPrompt,
            DrawerCardAction::AttachEvidence,
            DrawerCardAction::ConvertArtifact,
            DrawerCardAction::Discard,
        ]
    }

    /// The menu label (matches the contract's exact action labels).
    pub fn label(self) -> &'static str {
        match self {
            DrawerCardAction::Stow => "Stow",
            DrawerCardAction::Pin => "Pin",
            DrawerCardAction::Promote => "Promote",
            DrawerCardAction::SendToPane => "Send to pane",
            DrawerCardAction::CopyToPrompt => "Copy to prompt",
            DrawerCardAction::AttachEvidence => "Attach evidence",
            DrawerCardAction::ConvertArtifact => "Convert to artifact",
            DrawerCardAction::Discard => "Discard",
        }
    }

    /// The snake_case id segment used in the action menu item id + AccessKit author_id.
    pub fn snake(self) -> &'static str {
        match self {
            DrawerCardAction::Stow => "stow",
            DrawerCardAction::Pin => "pin",
            DrawerCardAction::Promote => "promote",
            DrawerCardAction::SendToPane => "send_to_pane",
            DrawerCardAction::CopyToPrompt => "copy_to_prompt",
            DrawerCardAction::AttachEvidence => "attach_evidence",
            DrawerCardAction::ConvertArtifact => "convert_artifact",
            DrawerCardAction::Discard => "discard",
        }
    }

    /// The stable context-menu item id this action uses (`drawer.action.{snake}`). The shared
    /// [`crate::context_menu::ContextMenu`] derives the AccessKit author_id `ctx-menu.{id}` from this, so
    /// the rendered item is `Role::MenuItem` with author_id `ctx-menu.drawer.action.{snake}` —
    /// DETERMINISTIC given the action (RISK-024-F: not random), discoverable + clickable out-of-process.
    pub fn menu_item_id(self) -> &'static str {
        match self {
            DrawerCardAction::Stow => "drawer.action.stow",
            DrawerCardAction::Pin => "drawer.action.pin",
            DrawerCardAction::Promote => "drawer.action.promote",
            DrawerCardAction::SendToPane => "drawer.action.send_to_pane",
            DrawerCardAction::CopyToPrompt => "drawer.action.copy_to_prompt",
            DrawerCardAction::AttachEvidence => "drawer.action.attach_evidence",
            DrawerCardAction::ConvertArtifact => "drawer.action.convert_artifact",
            DrawerCardAction::Discard => "drawer.action.discard",
        }
    }

    /// Map a confirmed context-menu item id back to its typed action (the inverse of [`menu_item_id`]).
    pub fn from_menu_item_id(id: &str) -> Option<DrawerCardAction> {
        Self::all().iter().copied().find(|a| a.menu_item_id() == id)
    }

    /// Whether this action requires a real backend block target. Local actions (Promote/SendToPane/
    /// CopyToPrompt) and the disabled ConvertArtifact do NOT; the persisting actions DO.
    pub fn needs_block_target(self) -> bool {
        matches!(
            self,
            DrawerCardAction::Stow
                | DrawerCardAction::Pin
                | DrawerCardAction::AttachEvidence
                | DrawerCardAction::Discard
        )
    }

    /// Whether this action is a DESTRUCTIVE op that MUST be gated behind a confirm dialog before
    /// dispatch (HBR-STOP / RISK-024-A). Only Discard (the irreversible DELETE) qualifies.
    pub fn needs_confirm(self) -> bool {
        matches!(self, DrawerCardAction::Discard)
    }
}

/// The real backend target a persisting card action acts on: the workspace + block ids plus the
/// display metadata the local actions (copy-to-prompt) need. A card only carries `Some(..)` when it is
/// bound to a concrete Loom block; the four MT-023 TYPE cards (Agenda/Mail/Lists/Notes) are category
/// summaries with NO single block, so they carry `None` and the block-requiring actions are correctly
/// rendered DISABLED (never faked against a nonexistent block id — rubric end-to-end integrity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawerActionTarget {
    pub workspace_id: String,
    pub block_id: String,
    pub title: String,
    pub content_type: String,
    pub excerpt: String,
}

impl DrawerActionTarget {
    /// Build the coder-prompt string copy-to-prompt writes to the clipboard (the contract's exact
    /// format, ported from the React `copy_as_coder_prompt` pattern in EvidenceDrawer.tsx):
    /// `"Block: {title}\nType: {content_type}\nID: {block_id}\n\n{excerpt}"`.
    pub fn coder_prompt(&self) -> String {
        format!(
            "Block: {}\nType: {}\nID: {}\n\n{}",
            self.title, self.content_type, self.block_id, self.excerpt
        )
    }
}

/// A confirmed drawer card action plus the card it targets. Returned by the drawer render so the host
/// ([`crate::app::HandshakeApp::apply_drawer_action`]) dispatches the backend call / local mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawerCardActionEvent {
    pub kind: DrawerCardKind,
    pub action: DrawerCardAction,
    /// The card's backend target, if it is bound to a concrete block. `None` for the TYPE cards.
    pub target: Option<DrawerActionTarget>,
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
    /// `Some(..)` when the card is bound to a concrete Loom block so its persisting actions
    /// (Stow/Pin/Discard/AttachEvidence) have a real target (MT-024). The four MT-023 TYPE cards carry
    /// `None` — they are category summaries, so block-requiring actions render DISABLED rather than
    /// dispatch against a nonexistent block id.
    pub action_target: Option<DrawerActionTarget>,
    /// MT-024 MAJOR FIX (AC-024-4/5): `true` briefly after a card action SUCCEEDS, so the card renders a
    /// success indicator. The contract's card-removal/reorder lifecycle assumes a per-block item list; the
    /// MT-023 TYPE-card drawer's success effect is this FEEDBACK + a count refresh, not removal/reorder
    /// (disclosed deviation). Cleared on the next fetch/apply_result so it does not persist indefinitely.
    pub action_succeeded: bool,
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
                action_target: None,
                action_succeeded: false,
            },
            _ => Self {
                kind,
                badge_count: 0,
                subtitle: "—".to_owned(),
                loading: false,
                error: None,
                action_target: None,
                action_succeeded: false,
            },
        }
    }

    /// Bind this card to a concrete Loom block so its persisting actions have a real backend target
    /// (MT-024). Builder-style so the host can attach a target when a card represents a single block.
    pub fn with_action_target(mut self, target: DrawerActionTarget) -> Self {
        self.action_target = Some(target);
        self
    }

    /// Fold a delivered fetch result into the card: clears the loading flag, sets badge + subtitle on
    /// success, or sets the error string on failure (badge/subtitle left as-is so the card degrades
    /// visibly rather than blanking).
    pub fn apply_result(&mut self, result: Result<DrawerCardData, String>) {
        self.loading = false;
        // A fresh data refresh supersedes the transient post-action success indicator (the refreshed
        // count IS the durable feedback; the success flag is the brief in-between signal).
        self.action_succeeded = false;
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

    /// MT-024 MAJOR FIX (AC-024-4/5): mark this card's last action as succeeded so it renders the success
    /// indicator, and clear any stale error. Called by the host's receipt drain on an `Ok` action receipt.
    pub fn mark_action_succeeded(&mut self) {
        self.action_succeeded = true;
        self.error = None;
    }

    /// The AccessKit label the card exposes (`"{title} ({badge_count})"` per the contract).
    pub fn access_label(&self) -> String {
        format!("{} ({})", self.kind.title(), self.badge_count)
    }

    /// Render the card as a fixed-width frame at its STABLE AccessKit id (so its NodeId is stable across
    /// frames/restarts). Returns a [`DrawerCardOutcome`] reporting whether the card body was clicked AND
    /// whether a typed action was confirmed from the overflow menu this frame (MT-024). Card-body click
    /// detection is a single `Sense::click()` on the card rect (CONTROL-023-E: never combined with a drag
    /// sense, so no double-fire), with an ADDITIONAL `Sense::click()` interacted on the overflow button
    /// rect and a right-click (`secondary_clicked`) sense that BOTH open the same action menu. The body
    /// shows the kind glyph + title, an always-visible `...` overflow button (AC-024-1), a badge chip, and
    /// the subtitle / loading / error line.
    pub fn show(&self, ui: &mut egui::Ui, colors: DrawerColors) -> DrawerCardOutcome {
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

        // ── Overflow `...` button (MT-024 AC-024-1): a fixed-id button anchored at the card's top-right,
        //    ALWAYS visible (not hover-gated). Rendered in its OWN foreground `Area` (the same isolation
        //    the affordance tab uses) so it sits on a DISTINCT interaction layer ABOVE the card body and
        //    therefore reliably receives the click instead of having it swallowed by the larger card
        //    `interact` rect underneath (proven necessary: an overlapping same-layer `ui.interact` lets
        //    the card claim the click). The Area itself is non-interactable so it adds no anonymous node.
        let overflow_id = unsafe { egui::Id::from_high_entropy_bits(self.kind.overflow_node_id()) };
        let overflow_author = self.kind.overflow_author_id();
        let overflow_size = egui::vec2(18.0, 18.0);
        let overflow_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - overflow_size.x - 6.0, rect.top() + 6.0),
            overflow_size,
        );
        let overflow_aria = format!("{} actions", self.kind.title());
        let overflow_resp = egui::Area::new(
            egui::Id::new(("hsk.drawer.overflow_area", self.kind.snake())),
        )
        .fixed_pos(overflow_rect.min)
        .order(egui::Order::Foreground)
        // interactable(false) on the AREA (the affordance-tab pattern): an interactable Area registers an
        // anonymous (role Unknown, Action::Click) node that trips the MT-025 gate. The explicit
        // `ui.interact` below is the ONE interactive node and carries the stable author_id.
        .interactable(false)
        .constrain(false)
        .show(ui.ctx(), |ui| {
            let (orect, _) = ui.allocate_exact_size(overflow_size, egui::Sense::hover());
            let oresp = ui.interact(orect, overflow_id, egui::Sense::click());
            let obg = if oresp.hovered() { colors.badge_bg } else { colors.card_bg };
            ui.painter().rect_filled(orect, 4.0, obg);
            let g = ui.painter().layout_no_wrap(
                "⋯".to_owned(),
                egui::FontId::proportional(14.0),
                colors.card_text,
            );
            ui.painter().galley(
                egui::pos2(
                    orect.center().x - g.size().x * 0.5,
                    orect.center().y - g.size().y * 0.5,
                ),
                g,
                colors.card_text,
            );
            let aria = overflow_aria.clone();
            oresp.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &aria)
            });
            let aria_node = overflow_aria.clone();
            ui.ctx().accesskit_node_builder(overflow_id, move |node| {
                node.set_role(accesskit::Role::Button);
                node.set_author_id(overflow_author.to_owned());
                node.set_label(aria_node);
            });
            oresp.on_hover_text("Card actions")
        })
        .inner;

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
            } else if self.action_succeeded {
                // MT-024 MAJOR FIX (AC-024-4/5): the success indicator for a TYPE card (feedback, since
                // there is no per-item card to remove/reorder). Shown briefly until the count refresh
                // resolves and `apply_result` clears the flag.
                ("✓ Done".to_owned(), colors.muted_text)
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

        // ── Action menu (MT-024 AC-024-2/3): ONE popup anchored on the overflow button, opened by EITHER
        //    trigger (AC-024-2): the overflow-button CLICK toggles it (egui's `Popup::menu` toggle-button
        //    semantics, the SAME native primitive the MT-015 menu bar uses), and a RIGHT-CLICK of the
        //    card body opens the SAME popup id (`Popup::open_id`). Built on the shared MT-019 ContextMenu
        //    item model rendered via `render_into`, so every item is a Role::MenuItem with a stable
        //    `ctx-menu.drawer.action.{snake}` author_id. `Popup::menu` is `CloseOnClickOutside` + Escape
        //    by default (AC-024-3) and anchored to the button rect (CONTROL-024-B: never a raw Window, so
        //    it can never open off-screen). The toggle path avoids egui's `context_menu` quirk where a
        //    same-frame click ALSO issues a close — the bug that made a card-anchored popup never open.
        let menu_popup_id = egui::Popup::default_response_id(&overflow_resp);
        // A card-body RIGHT-CLICK opens the SAME menu id as the overflow-button CLICK (AC-024-2). The
        // overflow click is handled by `Popup::menu`'s native toggle below (the overflow button is its own
        // foreground Area, so its `clicked()` reads reliably); the right-click opens the id explicitly.
        if resp.secondary_clicked() {
            egui::Popup::open_id(ui.ctx(), menu_popup_id);
        }
        let menu = self.action_menu();
        let confirmed = egui::Popup::menu(&overflow_resp)
            .show(|ui| {
                ui.set_min_width(180.0);
                menu.render_into(ui)
            })
            .and_then(|r| r.inner);
        let action = confirmed.and_then(DrawerCardAction::from_menu_item_id);

        DrawerCardOutcome {
            // The card body opens its pane only on a PRIMARY click (AC-023-12); a right-click opens the
            // action menu instead, so it never counts as a nav (suppress nav when secondary-clicked).
            clicked: resp.clicked() && !resp.secondary_clicked(),
            action,
        }
    }

    /// Build the typed eight-item action menu for this card (MT-024). ConvertArtifact is always disabled
    /// (no backend surface — disclosed). The four block-requiring actions (Stow/Pin/AttachEvidence/
    /// Discard) are disabled when this card has NO concrete block target (the TYPE cards), so they are
    /// never dispatched against a nonexistent block id (rubric end-to-end integrity / RISK-024-A safety).
    fn action_menu(&self) -> crate::context_menu::ContextMenu {
        use crate::context_menu::{ContextMenu, ContextMenuItem};
        let has_target = self.action_target.is_some();
        let mut menu = ContextMenu::new("drawer.action");
        for &action in DrawerCardAction::all() {
            let mut item = ContextMenuItem::action(action.menu_item_id(), action.label());
            if action == DrawerCardAction::ConvertArtifact {
                item = item.disabled("Artifact conversion has no backend surface yet (V1 stub)");
            } else if action.needs_block_target() && !has_target {
                item = item.disabled("This card has no single block to act on");
            }
            if action == DrawerCardAction::Discard {
                // Visual separation before the destructive action (parity with the MT-021 menu).
                menu = menu.separator();
            }
            menu = menu.item(item);
        }
        menu
    }
}

/// What one card render produced this frame (MT-024): whether the card body was clicked (the MT-023
/// open-pane / Mail-tooltip navigation) AND whether a typed action was confirmed from the overflow menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrawerCardOutcome {
    pub clicked: bool,
    pub action: Option<DrawerCardAction>,
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
    /// this codebase's registration-order convention — see the module docs). Returns a
    /// [`DrawerPanelOutcome`] carrying the first card NAV event (MT-023 open-pane / Mail-tooltip) AND the
    /// first confirmed card ACTION event (MT-024) produced this frame. The resize handle drag updates
    /// `self.height`.
    pub fn show_open_panel(&mut self, ui: &mut egui::Ui, colors: DrawerColors) -> DrawerPanelOutcome {
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

        let mut nav = None;
        let mut action = None;
        // Horizontal scroll shelf. CONTROL-023-F: cards render right-to-left for the right-aligned
        // visual (Agenda rightmost) WITHOUT reversing the Vec — the AccessKit tree keeps the logical
        // Agenda→Notes order, so swarm agents read a stable card sequence.
        egui::ScrollArea::horizontal()
            .id_salt("hsk.drawer.scroll")
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    for card in &self.cards {
                        let outcome = card.show(ui, colors);
                        if outcome.clicked && nav.is_none() {
                            nav = Some(match card.kind {
                                DrawerCardKind::Mail => DrawerEvent::MailTooltip,
                                other => DrawerEvent::OpenCard(other),
                            });
                        }
                        if let Some(act) = outcome.action {
                            if action.is_none() {
                                action = Some(DrawerCardActionEvent {
                                    kind: card.kind,
                                    action: act,
                                    target: card.action_target.clone(),
                                });
                            }
                        }
                        ui.add_space(8.0);
                    }
                });
            });
        DrawerPanelOutcome { nav, action }
    }
}

/// What one open-drawer frame produced (MT-024): the first card NAV event (MT-023 open-pane / Mail
/// tooltip) and the first confirmed card ACTION event. Both are `None` when nothing fired this frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrawerPanelOutcome {
    pub nav: Option<DrawerEvent>,
    pub action: Option<DrawerCardActionEvent>,
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
