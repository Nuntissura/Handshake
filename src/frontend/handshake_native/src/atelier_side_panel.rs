//! CKC / Atelier side panel (WP-KERNEL-012 MT-033, cluster E5 — CKC embeds / drag-in).
//!
//! ## What this is
//!
//! [`AtelierSidePanel`] is the native egui side panel that lists CKC/Atelier intake batches (expandable
//! into draggable media/character/moodboard item rows) and the command corpus, fetched LIVE from the
//! EXISTING WP-KERNEL-005 atelier backend via [`crate::backend_client::AtelierClient`] (NO mocks — the
//! list rows come from real `GET /atelier/intake/batches` + `.../items` + `/atelier/command-corpus`).
//! It is the native peer of the React `app/src/components/AtelierPanel.tsx`.
//!
//! Each item row is an egui `dnd_drag_source` whose payload is a
//! [`crate::interop::DragPayload::AtelierRef`]: dragging a row and releasing it over the rich-text editor
//! inserts a CKC `hsLink` embed atom, or over the canvas places a `loom://` block reference. The panel
//! itself performs NO embed/placement — it only EMITS the drag payload; the drop targets (rich editor,
//! canvas) consume it.
//!
//! ## Backend reuse only (verified, typed-blocker on a gap)
//!
//! The three reads were VERIFIED READ-ONLY against `src/backend/handshake_core/src/api/atelier.rs`
//! (WP-KERNEL-005). The atelier backend EXISTS, so this is real wiring, not a typed blocker. If the
//! backend is DOWN/unreachable the panel shows a typed error (never a blank panel, never faked items —
//! RISK-5 / MC-5: a spinner only while a fetch is genuinely in flight, then either rows or an error).
//!
//! ## AccessKit (HBR-SWARM)
//!
//! - the panel container: author_id [`PANEL_AUTHOR_ID`] (`atelier-side-panel`), Role::List.
//! - each item row: author_id `atelier-item-{item_id}` (Role::ListItem), with a description carrying
//!   `draggable` + the resolved ref so an out-of-process agent can read the draggable reference by stable
//!   id. AccessKit 0.21.1 has no `StartDrag` action, so the row advertises `Click` as an executable
//!   model fallback that inserts the same item through the active editor route.
//!   The refresh button: author_id [`REFRESH_AUTHOR_ID`], Role::Button.

use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::backend_client::{
    AtelierClient, AtelierCorpusRow, AtelierItemRow, AtelierItemsCell, AtelierSidePanelCell,
};
use crate::interop::{AtelierItemKind, AtelierRef, DragPayload};
use crate::theme::HsPalette;

/// Stable AccessKit author_id for the panel container (Role::List).
pub const PANEL_AUTHOR_ID: &str = "atelier-side-panel";
/// Stable AccessKit author_id for the panel's refresh button (Role::Button).
pub const REFRESH_AUTHOR_ID: &str = "atelier-side-panel.refresh";
/// Author_id prefix for one draggable item row. The full id is `atelier-item-{hex_item_id}`.
pub const ITEM_AUTHOR_ID_PREFIX: &str = "atelier-item-";
pub const ITEM_INSERT_AUTHOR_ID_PREFIX: &str = "atelier-item-insert-";
pub const ITEM_CANVAS_AUTHOR_ID_PREFIX: &str = "atelier-item-canvas-";
pub const BATCH_AUTHOR_ID_PREFIX: &str = "atelier-batch-";
pub const CORPUS_AUTHOR_ID_PREFIX: &str = "atelier-corpus-";
pub const ITEMS_RETRY_AUTHOR_ID_PREFIX: &str = "atelier-items-retry-";
pub const CHARACTER_BLOCKER_AUTHOR_ID: &str = "atelier-character-list-blocker";
pub const MOODBOARD_BLOCKER_AUTHOR_ID: &str = "atelier-moodboard-list-blocker";

pub const CHARACTER_LIST_BLOCKER: &str =
    "Characters unavailable: the Atelier backend exposes no character-list route.";
pub const MOODBOARD_LIST_BLOCKER: &str =
    "Moodboards unavailable: the Atelier backend exposes no moodboard-list route.";

/// The stable AccessKit author_id for one draggable item row. Hex-encoding the raw UTF-8 bytes keeps the
/// suffix in `[0-9a-f]` while remaining injective (`a/b` and `a:b` can never collapse to one node id).
pub fn item_author_id(item_id: &str) -> String {
    format!("{ITEM_AUTHOR_ID_PREFIX}{}", stable_hex(item_id))
}

pub fn item_insert_author_id(item_id: &str) -> String {
    format!("{ITEM_INSERT_AUTHOR_ID_PREFIX}{}", stable_hex(item_id))
}

pub fn item_canvas_author_id(item_id: &str) -> String {
    format!("{ITEM_CANVAS_AUTHOR_ID_PREFIX}{}", stable_hex(item_id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtelierPanelAction {
    InsertIntoActiveEditor(AtelierRef),
    PlaceOnActiveCanvas(AtelierRef),
}

pub fn batch_author_id(batch_id: &str) -> String {
    format!("{BATCH_AUTHOR_ID_PREFIX}{}", stable_hex(batch_id))
}

pub fn corpus_author_id(entry_id: &str) -> String {
    format!("{CORPUS_AUTHOR_ID_PREFIX}{}", stable_hex(entry_id))
}

fn stable_hex(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

/// Whether a section is expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadState {
    /// No load has been requested yet (initial state — the panel triggers a load on first render).
    Idle,
    /// A side-panel load is in flight (spinner shown ONLY in this state — RISK-5 / MC-5).
    Loading,
    /// The load resolved (rows present, possibly empty).
    Loaded,
    /// The load failed; the panel shows the error text.
    Failed,
}

/// The CKC / Atelier side panel state. Held by the host pane; mutated in place by [`Self::show`]. The
/// backend reads run off the UI thread through [`AtelierClient`]; this state holds only the resolved
/// projection + ephemeral UI state (which batch is expanded, the in-flight cells).
pub struct AtelierSidePanel {
    /// The atelier read client (off-thread fetches). `None` in a headless test that injects rows
    /// directly via [`Self::with_rows`] (so a kittest never needs a live backend / runtime).
    client: Option<AtelierClient>,
    /// The batch rows (top-level "Media / Characters / Moodboards" section source).
    batches: Vec<crate::backend_client::AtelierBatchRow>,
    /// The command-corpus rows (the "Command Corpus" section).
    corpus: Vec<AtelierCorpusRow>,
    /// The currently-expanded batch id + its loaded item rows (one batch expanded at a time, keeping the
    /// panel compact). `None` => no batch expanded.
    expanded: Option<(String, Vec<AtelierItemRow>)>,
    /// The side-panel load state (drives the spinner/error/rows — no perpetual spinner).
    state: LoadState,
    /// The per-batch items-load state for the currently-expanding batch (a spinner while in flight).
    items_loading: Option<String>,
    /// Error text from a failed load (shown instead of rows; never a blank panel — RISK-5 / MC-5).
    error: Option<String>,
    /// Error for the expanded batch's item request. Kept separate from the top-level load so an item
    /// failure renders an explicit Retry surface instead of masquerading as a valid empty batch.
    items_error: Option<(String, String)>,
    /// In-flight side-panel load delivery cell (drained at the top of `show`).
    panel_cell: AtelierSidePanelCell,
    /// In-flight per-batch items delivery cell (drained at the top of `show`).
    items_cell: AtelierItemsCell,
    /// Monotonic identity of the newest top-level load. Older completions are discarded.
    load_generation: u64,
    /// Monotonic identity of the newest per-batch items load. Older completions are discarded.
    items_generation: u64,
    pending_actions: std::collections::VecDeque<AtelierPanelAction>,
}

impl AtelierSidePanel {
    /// A fresh panel bound to the production atelier client (the shell's wiring point). The first
    /// [`Self::show`] triggers the initial load.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::with_client(Some(AtelierClient::production(runtime)))
    }

    /// A panel with an explicit (or no) client. `None` is the headless test path — the panel renders no
    /// rows until [`Self::with_rows`] injects them, and never touches the network.
    pub fn with_client(client: Option<AtelierClient>) -> Self {
        Self {
            client,
            batches: Vec::new(),
            corpus: Vec::new(),
            expanded: None,
            state: LoadState::Idle,
            items_loading: None,
            error: None,
            items_error: None,
            panel_cell: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            items_cell: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            load_generation: 0,
            items_generation: 0,
            pending_actions: std::collections::VecDeque::new(),
        }
    }

    /// TEST SEAM: a panel pre-seeded with batches + corpus + an expanded batch's items, in the `Loaded`
    /// state (no client, no network). Used by the kittest drag + AccessKit proofs so a headless render
    /// shows real rows without a backend.
    pub fn with_rows(
        batches: Vec<crate::backend_client::AtelierBatchRow>,
        corpus: Vec<AtelierCorpusRow>,
        expanded: Option<(String, Vec<AtelierItemRow>)>,
    ) -> Self {
        let mut p = Self::with_client(None);
        p.batches = batches;
        p.corpus = corpus;
        p.expanded = expanded;
        p.state = LoadState::Loaded;
        p
    }

    /// TEST SEAM: seed rows into an ALREADY-constructed panel (e.g. the one the live `HandshakeApp`
    /// mounts), moving it to the `Loaded` state so a live-shell render shows real draggable item nodes
    /// without a backend. Mirrors [`Self::with_rows`] but mutates in place rather than constructing.
    pub fn seed_rows(
        &mut self,
        batches: Vec<crate::backend_client::AtelierBatchRow>,
        corpus: Vec<AtelierCorpusRow>,
        expanded: Option<(String, Vec<AtelierItemRow>)>,
    ) {
        self.batches = batches;
        self.corpus = corpus;
        self.expanded = expanded;
        self.state = LoadState::Loaded;
        self.error = None;
        self.items_error = None;
    }

    /// Rebind a headless or stale shell panel to a live backend client. In-flight generations and
    /// projections are invalidated so the next mounted frame loads exclusively from the new endpoint.
    pub fn bind_client(&mut self, client: AtelierClient) {
        self.client = Some(client);
        self.batches.clear();
        self.corpus.clear();
        self.expanded = None;
        self.state = LoadState::Idle;
        self.items_loading = None;
        self.error = None;
        self.items_error = None;
        self.load_generation = self.load_generation.wrapping_add(1);
        self.items_generation = self.items_generation.wrapping_add(1);
        if let Ok(mut queue) = self.panel_cell.lock() {
            queue.clear();
        }
        if let Ok(mut queue) = self.items_cell.lock() {
            queue.clear();
        }
    }

    /// Trigger a side-panel load (batches + corpus) if a client is present. Sets [`LoadState::Loading`]
    /// so the spinner shows ONLY while the fetch is genuinely in flight.
    pub fn request_load(&mut self) {
        if let Some(client) = &self.client {
            self.load_generation = self.load_generation.wrapping_add(1);
            self.state = LoadState::Loading;
            self.error = None;
            client.fetch_side_panel(self.load_generation, Arc::clone(&self.panel_cell));
        }
    }

    /// Expand a batch: collapse any other, then trigger its items load (or no-op in the headless path).
    fn expand_batch(&mut self, batch_id: &str) {
        // Toggle: clicking the expanded batch collapses it.
        if self.expanded.as_ref().map(|(id, _)| id.as_str()) == Some(batch_id) {
            self.expanded = None;
            self.items_loading = None;
            self.items_error = None;
            return;
        }
        self.expanded = Some((batch_id.to_owned(), Vec::new()));
        self.items_error = None;
        if let Some(client) = &self.client {
            self.items_generation = self.items_generation.wrapping_add(1);
            self.items_loading = Some(batch_id.to_owned());
            client.fetch_items(
                self.items_generation,
                batch_id,
                Arc::clone(&self.items_cell),
            );
        }
    }

    /// Drain the off-thread delivery cells into the panel state (called at the top of `show`).
    fn drain_cells(&mut self) {
        if let Ok(mut slot) = self.panel_cell.lock() {
            while let Some((generation, result)) = slot.pop_front() {
                if generation == self.load_generation {
                    match result {
                        Ok(data) => {
                            self.batches = data.batches;
                            self.corpus = data.corpus;
                            self.state = LoadState::Loaded;
                            self.error = None;
                        }
                        Err(msg) => {
                            self.state = LoadState::Failed;
                            self.error = Some(msg);
                        }
                    }
                }
            }
        }
        if let Ok(mut slot) = self.items_cell.lock() {
            while let Some((generation, batch_id, result)) = slot.pop_front() {
                if generation == self.items_generation {
                    // Only apply if this is still the expanded batch (a stale response for a
                    // since-collapsed batch is discarded — no dangling item rows).
                    if self.expanded.as_ref().map(|(id, _)| id.as_str()) == Some(batch_id.as_str())
                    {
                        match result {
                            Ok(items) => {
                                self.expanded = Some((batch_id.clone(), items));
                                self.items_error = None;
                            }
                            Err(msg) => {
                                self.items_error = Some((batch_id.clone(), msg));
                            }
                        }
                    }
                    if self.items_loading.as_deref() == Some(batch_id.as_str()) {
                        self.items_loading = None;
                    }
                }
            }
        }
    }

    /// True when a load is in flight (the spinner shows only here — RISK-5 / MC-5).
    pub fn is_loading(&self) -> bool {
        self.state == LoadState::Loading
    }

    /// The currently-expanded batch id + its item rows (test/peek accessor).
    pub fn expanded(&self) -> Option<&(String, Vec<AtelierItemRow>)> {
        self.expanded.as_ref()
    }

    /// Drain stable executable actions emitted by item-row buttons. Drag remains available for humans;
    /// these actions are the deterministic model/operator path when AccessKit cannot express StartDrag.
    pub fn take_action(&mut self) -> Option<AtelierPanelAction> {
        self.pending_actions.pop_front()
    }

    /// Render the panel into `ui`. The panel:
    /// - triggers the initial load on the first render (Idle -> Loading) when a client is present,
    /// - shows a spinner ONLY while loading, an error chip on failure, else the two sections,
    /// - makes each item row a `dnd_drag_source` whose payload is a `DragPayload::AtelierRef`,
    /// - emits the AccessKit List container + per-row ListItem nodes (HBR-SWARM).
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) {
        self.drain_cells();

        // First-render load trigger (Idle -> Loading) when a client is present (the headless path stays
        // Loaded with injected rows and never enters Loading).
        if self.state == LoadState::Idle && self.client.is_some() {
            self.request_load();
        }

        // ── Header strip: title + refresh ───────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Atelier / CKC")
                    .strong()
                    .color(palette.text),
            );
            let refresh = ui.button("⟳");
            emit_button_node(ui, refresh.id, REFRESH_AUTHOR_ID, "Refresh atelier");
            if refresh.clicked() {
                self.request_load();
            }
        });
        ui.separator();

        // ── Panel container: a Role::List node so a swarm agent reads the whole panel by stable id ────
        let panel_id = egui::Id::new(PANEL_AUTHOR_ID);
        let panel_resp = ui
            .scope_builder(egui::UiBuilder::new().id_salt(panel_id), |ui| {
                self.show_body(ui, palette);
            })
            .response;
        emit_list_container_node(ui, panel_resp.id, PANEL_AUTHOR_ID, "Atelier / CKC items");
    }

    /// The panel body (inside the List container scope): spinner / error / the two sections.
    fn show_body(&mut self, ui: &mut egui::Ui, palette: &HsPalette) {
        match self.state {
            LoadState::Loading => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(egui::RichText::new("Loading atelier…").color(palette.text_subtle));
                });
                // A genuine in-flight fetch animates; the spinner stops as soon as the cell delivers.
                ui.ctx().request_repaint();
                return;
            }
            LoadState::Failed => {
                let msg = self.error.as_deref().unwrap_or("atelier load failed");
                ui.colored_label(palette.error_text, format!("Atelier unavailable: {msg}"));
                ui.label(
                    egui::RichText::new("Is the Handshake backend running? Click ⟳ to retry.")
                        .color(palette.text_subtle),
                );
                return;
            }
            LoadState::Idle | LoadState::Loaded => {}
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ── Section 1: Media / Characters / Moodboards (intake batches -> draggable item rows) ────
            ui.label(
                egui::RichText::new("Media / Characters / Moodboards")
                    .strong()
                    .color(palette.text),
            );
            ui.label(
                egui::RichText::new("Media available through intake batches.")
                    .color(palette.text_subtle),
            );
            let character = ui.colored_label(palette.error_text, CHARACTER_LIST_BLOCKER);
            emit_status_node(
                ui,
                character.id,
                CHARACTER_BLOCKER_AUTHOR_ID,
                CHARACTER_LIST_BLOCKER,
            );
            let moodboard = ui.colored_label(palette.error_text, MOODBOARD_LIST_BLOCKER);
            emit_status_node(
                ui,
                moodboard.id,
                MOODBOARD_BLOCKER_AUTHOR_ID,
                MOODBOARD_LIST_BLOCKER,
            );
            if self.batches.is_empty() {
                ui.label(egui::RichText::new("No intake batches yet.").color(palette.text_subtle));
            }
            // Clone the batch list so we can mutate `self.expanded` while iterating (small list — the
            // backend caps at 200; a clone of {id,label,status} strings is cheap and avoids a borrow
            // conflict with the per-row expand/drag handlers).
            let batches = self.batches.clone();
            for batch in &batches {
                let expanded_here = self.expanded.as_ref().map(|(id, _)| id.as_str())
                    == Some(batch.batch_id.as_str());
                let marker = if expanded_here { "▼" } else { "▶" };
                let label = format!("{marker} {}  ({})", batch.source_label, batch.status);
                let batch_button = ui.add(egui::Button::new(&label).frame(false));
                emit_button_node(
                    ui,
                    batch_button.id,
                    &batch_author_id(&batch.batch_id),
                    &label,
                );
                if batch_button.clicked() || accesskit_click_requested(ui, batch_button.id) {
                    self.expand_batch(&batch.batch_id);
                }
                if expanded_here {
                    self.show_expanded_items(ui, palette);
                }
            }

            ui.separator();

            // ── Section 2: Command Corpus ─────────────────────────────────────────────────────────────
            ui.label(
                egui::RichText::new("Command Corpus")
                    .strong()
                    .color(palette.text),
            );
            if self.corpus.is_empty() {
                ui.label(
                    egui::RichText::new("No command-corpus entries.").color(palette.text_subtle),
                );
            }
            for entry in &self.corpus {
                let response = ui.label(
                    egui::RichText::new(format!(
                        "• {}  [{} · {}]",
                        entry.action_id, entry.execution_class, entry.owner
                    ))
                    .color(palette.text_subtle),
                );
                emit_list_item_plain_node(
                    ui,
                    response.id,
                    &corpus_author_id(&entry.entry_id),
                    &entry.action_id,
                    "Atelier command-corpus entry",
                );
            }
        });
    }

    /// Render the expanded batch's item rows as draggable `dnd_drag_source`s.
    fn show_expanded_items(&mut self, ui: &mut egui::Ui, palette: &HsPalette) {
        if self.items_loading.is_some() {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.spinner();
                ui.label(egui::RichText::new("Loading items…").color(palette.text_subtle));
            });
            ui.ctx().request_repaint();
            return;
        }
        let expanded_batch_id = self.expanded.as_ref().map(|(id, _)| id.clone());
        if let (Some(batch_id), Some((failed_batch, message))) =
            (expanded_batch_id.as_ref(), self.items_error.as_ref())
        {
            if batch_id == failed_batch {
                let message = message.clone();
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.colored_label(
                            palette.error_text,
                            format!("Items unavailable: {message}"),
                        );
                        let retry = ui.button("Retry items");
                        emit_button_node(
                            ui,
                            retry.id,
                            &format!("{ITEMS_RETRY_AUTHOR_ID_PREFIX}{}", stable_hex(batch_id)),
                            "Retry items",
                        );
                        if retry.clicked() {
                            self.retry_items(batch_id);
                        }
                    });
                });
                return;
            }
        }
        let items = match &self.expanded {
            Some((_, items)) => items.clone(),
            None => return,
        };
        if items.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("(no items in this batch)").color(palette.text_subtle),
                );
            });
            return;
        }
        for item in &items {
            ui.indent(("atelier-item", item.item_id.as_str()), |ui| {
                if let Some(action) = draw_item_row(ui, item, palette) {
                    self.pending_actions.push_back(action);
                }
            });
        }
    }

    fn retry_items(&mut self, batch_id: &str) {
        self.items_error = None;
        self.items_generation = self.items_generation.wrapping_add(1);
        self.items_loading = Some(batch_id.to_owned());
        if let Some(client) = &self.client {
            client.fetch_items(
                self.items_generation,
                batch_id,
                Arc::clone(&self.items_cell),
            );
        } else {
            self.items_loading = None;
            self.items_error = Some((batch_id.to_owned(), "backend client unavailable".to_owned()));
        }
    }
}

/// The drag payload an item row carries. Each intake item is dragged as a `DragPayload::AtelierRef` with
/// `item_kind = Media` (intake items are media assets; characters/moodboards arrive via their own rows in
/// a richer panel — the intake-items list IS the media source per the verified backend). The label is the
/// file name; `loom_block_id` is `None` because the panel must not fabricate backend resolution. The
/// canvas host resolves the intake item through the canonical Loom block API before placement; the
/// rich-text target inserts the unresolved Atelier identity as an `hsLink` chip.
fn item_drag_payload(item: &AtelierItemRow) -> DragPayload {
    let reference = match &item.loom_block_id {
        Some(block_id) => AtelierRef::with_loom_block(
            item.item_id.clone(),
            AtelierItemKind::Media,
            item.file_name.clone(),
            block_id.clone(),
        ),
        None => AtelierRef::new(
            item.item_id.clone(),
            AtelierItemKind::Media,
            item.file_name.clone(),
        ),
    };
    DragPayload::AtelierRef(reference)
}

/// Draw one draggable item row + its ListItem AccessKit node.
fn draw_item_row(
    ui: &mut egui::Ui,
    item: &AtelierItemRow,
    palette: &HsPalette,
) -> Option<AtelierPanelAction> {
    let payload = item_drag_payload(item);
    let body_payload = payload.clone();
    let atelier_ref = match &payload {
        DragPayload::AtelierRef(reference) => reference.clone(),
        _ => unreachable!("item_drag_payload always returns AtelierRef"),
    };
    let mut action = None;
    let drag_id = egui::Id::new(item_author_id(&item.item_id));
    // The row is the drag SOURCE: dragging it produces the AtelierRef the rich-text / canvas drop zones
    // consume (egui::DragAndDrop, egui 0.33 — the same `dnd_drag_source` the tab bar / canvas use).
    let inner = ui
        .dnd_drag_source(drag_id, payload, |ui| {
            let body = ui.vertical(|ui| {
                let badge = AtelierItemKind::Media.badge();
                ui.label(
                    egui::RichText::new(format!("[{badge}] {}", item.file_name))
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} · source metadata only (no preview loaded)",
                        item.source_path
                    ))
                    .small()
                    .color(palette.text_subtle),
                );
            });
            // The nested labels own their AccessKit bounds. Give that exact body a click+drag
            // interaction and stage the same typed payload when it starts dragging; otherwise a
            // pointer at the model-addressable ListItem can be claimed by the child scope while the
            // outer dnd_drag_source never sees the gesture.
            let response = ui.interact(
                body.response.rect,
                drag_id.with("addressable-body"),
                egui::Sense::click_and_drag(),
            );
            response.dnd_set_drag_payload(body_payload.clone());
            response
        })
        .inner;
    // Emit the ListItem AccessKit node (the dynamic per-row address). AccessKit 0.21.1 has no
    // StartDrag action, so Click is the executable model fallback and inserts the same item while
    // pointer users retain typed drag-and-drop.
    emit_list_item_node(ui, inner.id, &item.item_id, &item.file_name);
    let accesskit_clicked = accesskit_click_requested(ui, inner.id);
    if inner.clicked() || accesskit_clicked {
        action = Some(AtelierPanelAction::InsertIntoActiveEditor(
            atelier_ref.clone(),
        ));
    }
    ui.horizontal_wrapped(|ui| {
        let insert = ui.button("Insert in editor");
        emit_button_node(
            ui,
            insert.id,
            &item_insert_author_id(&item.item_id),
            "Insert Atelier item into active rich editor",
        );
        if insert.clicked() || accesskit_click_requested(ui, insert.id) {
            action = Some(AtelierPanelAction::InsertIntoActiveEditor(
                atelier_ref.clone(),
            ));
        }
        let canvas = ui.button("Place on Canvas");
        emit_button_node(
            ui,
            canvas.id,
            &item_canvas_author_id(&item.item_id),
            "Place Atelier item on active Canvas",
        );
        if canvas.clicked() || accesskit_click_requested(ui, canvas.id) {
            action = Some(AtelierPanelAction::PlaceOnActiveCanvas(atelier_ref.clone()));
        }
    });
    action
}

fn accesskit_click_requested(ui: &egui::Ui, id: egui::Id) -> bool {
    ui.input(|input| {
        input
            .accesskit_action_requests(id, accesskit::Action::Click)
            .next()
            .is_some()
    })
}

/// Emit the panel container's Role::List AccessKit node (author_id `atelier-side-panel`).
fn emit_list_container_node(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::List);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
    });
}

/// Emit one item row's Role::ListItem AccessKit node with a `draggable` description carrying the
/// Atelier ref so an out-of-process agent reads the draggable reference by stable id.
fn emit_list_item_node(ui: &egui::Ui, id: egui::Id, item_id: &str, file_name: &str) {
    let author = item_author_id(item_id);
    let label = file_name.to_owned();
    let description = format!("draggable; atelier-ref media:{item_id}");
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::ListItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_description(description.clone());
        node.add_action(accesskit::Action::Click);
    });
}

fn emit_list_item_plain_node(
    ui: &egui::Ui,
    id: egui::Id,
    author_id: &str,
    label: &str,
    description: &str,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let description = description.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::ListItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_description(description.clone());
    });
}

fn emit_status_node(ui: &egui::Ui, id: egui::Id, author_id: &str, value: &str) {
    let author = author_id.to_owned();
    let value = value.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Status);
        node.set_author_id(author.clone());
        node.set_value(value.clone());
    });
}

/// Emit a button's AccessKit node (Role::Button + Action::Click + author_id).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_client::AtelierBatchRow;
    use egui_kittest::kittest::{NodeT, Queryable};
    use egui_kittest::Harness;

    fn item(id: &str, name: &str) -> AtelierItemRow {
        AtelierItemRow {
            item_id: id.to_owned(),
            file_name: name.to_owned(),
            source_path: format!("/intake/{name}"),
            lane: "accept".to_owned(),
            loom_block_id: None,
        }
    }

    /// The item author_id matches the contract's `atelier-item-{id}` shape with an injective hex suffix.
    #[test]
    fn item_author_id_matches_contract_shape() {
        assert_eq!(item_author_id("abc-123"), "atelier-item-6162632d313233");
        let id = item_author_id("ws:1/item 7#x");
        assert!(id.starts_with(ITEM_AUTHOR_ID_PREFIX));
        let suffix = &id[ITEM_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "author_id suffix must be lowercase hex; got '{suffix}'"
        );
        assert_ne!(item_author_id("a/b"), item_author_id("a:b"));
    }

    /// An item row drags as a `DragPayload::AtelierRef` whose `refValue` is the item id (the embed
    /// `refValue`) and whose label is the file name.
    #[test]
    fn item_row_drags_as_atelier_ref() {
        let payload = item_drag_payload(&item("item-9", "sunset.png"));
        match payload {
            DragPayload::AtelierRef(r) => {
                assert_eq!(r.item_id, "item-9");
                assert_eq!(r.item_kind, AtelierItemKind::Media);
                assert_eq!(r.label, "sunset.png");
                assert_eq!(r.ref_kind(), "media");
                // An intake item remains unresolved in the panel; the canvas host performs resolution.
                assert!(r.loom_block_id.is_none());
            }
            other => panic!("expected AtelierRef, got {other:?}"),
        }
    }

    /// A panel built `with_rows` is in the Loaded state (no client, no spinner) and exposes the rows.
    #[test]
    fn with_rows_is_loaded_not_loading() {
        let panel = AtelierSidePanel::with_rows(
            vec![AtelierBatchRow {
                batch_id: "b-1".to_owned(),
                source_label: "Batch One".to_owned(),
                status: "open".to_owned(),
            }],
            vec![],
            Some(("b-1".to_owned(), vec![item("i-1", "a.png")])),
        );
        assert!(
            !panel.is_loading(),
            "a with_rows panel is Loaded, never Loading (no perpetual spinner)"
        );
        assert_eq!(
            panel
                .expanded()
                .map(|(id, items)| (id.as_str(), items.len())),
            Some(("b-1", 1))
        );
    }

    #[test]
    fn fifo_drain_keeps_newest_generation_when_old_response_arrives_last() {
        let mut panel = AtelierSidePanel::with_client(None);
        panel.load_generation = 2;
        {
            let mut queue = panel.panel_cell.lock().unwrap();
            queue.push_back((
                2,
                Ok(crate::backend_client::AtelierSidePanelData {
                    batches: vec![AtelierBatchRow {
                        batch_id: "01900000-0000-7000-8000-000000000002".to_owned(),
                        source_label: "new".to_owned(),
                        status: "open".to_owned(),
                    }],
                    corpus: vec![],
                }),
            ));
            queue.push_back((1, Err("stale failure".to_owned())));
        }
        panel.drain_cells();
        assert_eq!(panel.batches[0].source_label, "new");
        assert_eq!(panel.state, LoadState::Loaded);
        assert!(panel.error.is_none());

        panel.items_generation = 4;
        panel.expanded = Some(("batch-new".to_owned(), vec![]));
        panel.items_loading = Some("batch-new".to_owned());
        {
            let mut queue = panel.items_cell.lock().unwrap();
            queue.push_back((
                4,
                "batch-new".to_owned(),
                Ok(vec![item(
                    "01900000-0000-7000-8000-000000000004",
                    "new.png",
                )]),
            ));
            queue.push_back((
                3,
                "batch-new".to_owned(),
                Ok(vec![item(
                    "01900000-0000-7000-8000-000000000003",
                    "old.png",
                )]),
            ));
        }
        panel.drain_cells();
        let (_, rows) = panel.expanded().expect("new batch stays expanded");
        assert_eq!(rows[0].file_name, "new.png");
        assert!(panel.items_loading.is_none());
    }

    #[test]
    fn item_http_failure_is_not_empty_state_and_newer_delivery_recovers() {
        let batch_id = "01900000-0000-7000-8000-000000000033";
        let mut panel = AtelierSidePanel::with_client(None);
        panel.state = LoadState::Loaded;
        panel.expanded = Some((batch_id.to_owned(), vec![]));
        panel.items_generation = 1;
        panel.items_loading = Some(batch_id.to_owned());
        panel.items_cell.lock().unwrap().push_back((
            1,
            batch_id.to_owned(),
            Err("HTTP 503 Service Unavailable".to_owned()),
        ));
        panel.drain_cells();
        assert_eq!(
            panel
                .items_error
                .as_ref()
                .map(|(_, message)| message.as_str()),
            Some("HTTP 503 Service Unavailable")
        );
        assert!(
            panel.expanded().is_some_and(|(_, rows)| rows.is_empty()),
            "the row projection stays empty but the separate error prevents an empty-state lie"
        );

        panel.items_generation = 2;
        panel.items_loading = Some(batch_id.to_owned());
        panel.items_cell.lock().unwrap().push_back((
            2,
            batch_id.to_owned(),
            Ok(vec![item(
                "01900000-0000-7000-8000-000000000034",
                "recovered.png",
            )]),
        ));
        panel.drain_cells();
        assert!(panel.items_error.is_none());
        assert_eq!(panel.expanded().unwrap().1[0].file_name, "recovered.png");
    }

    #[test]
    fn item_http_failure_renders_retry_not_no_items() {
        let batch_id = "01900000-0000-7000-8000-000000000033";
        let mut state = AtelierSidePanel::with_client(None);
        state.state = LoadState::Loaded;
        state.batches.push(AtelierBatchRow {
            batch_id: batch_id.to_owned(),
            source_label: "Failure batch".to_owned(),
            status: "open".to_owned(),
        });
        state.expanded = Some((batch_id.to_owned(), vec![]));
        state.items_generation = 1;
        state.items_cell.lock().unwrap().push_back((
            1,
            batch_id.to_owned(),
            Err("HTTP 503 Service Unavailable".to_owned()),
        ));
        let panel = Arc::new(Mutex::new(state));
        let mut harness = Harness::builder().build_ui(move |ui| {
            panel
                .lock()
                .unwrap()
                .show(ui, &crate::theme::HsTheme::Dark.palette());
        });
        harness.run();
        assert!(harness
            .get_by_label("Retry items")
            .accesskit_node()
            .data()
            .supports_action(accesskit::Action::Click));
        assert!(
            harness.root().children_recursive().any(|node| node
                .accesskit_node()
                .author_id()
                .is_some_and(|id| { id.starts_with(ITEMS_RETRY_AUTHOR_ID_PREFIX) })),
            "failed item load must expose a stable Retry node"
        );
        assert!(
            !harness.root().children_recursive().any(|node| node
                .accesskit_node()
                .label()
                .as_deref()
                == Some("(no items in this batch)")),
            "transport failure must not render the valid empty-state"
        );
    }

    /// `request_load` on a no-client panel is a benign no-op (stays Loaded, no panic).
    #[test]
    fn request_load_without_client_is_benign() {
        let mut panel = AtelierSidePanel::with_rows(vec![], vec![], None);
        panel.request_load();
        assert!(
            !panel.is_loading(),
            "no client -> request_load does not enter Loading"
        );
    }
}
