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
//! - each item row: author_id `atelier-item-{item_id}` (Role::ListItem), with Action::Click and a
//!   description carrying `draggable` + the resolved ref so an out-of-process agent can read the
//!   draggable reference by stable id. NOTE on actions: the contract names `actions=[Press, StartDrag]`,
//!   but accesskit 0.21.1 (the locked version) has NEITHER a `Press` NOR a `StartDrag` action variant —
//!   the field-correct mapping is `Action::Click` (the closest representable action) plus a `draggable`
//!   description, exactly as `tab_bar.rs` exposes its drag-source nodes. Inventing a non-existent variant
//!   would not compile. The refresh button: author_id [`REFRESH_AUTHOR_ID`], Role::Button.

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
/// Author_id prefix for one draggable item row. The full id is `atelier-item-{sanitized_item_id}`.
pub const ITEM_AUTHOR_ID_PREFIX: &str = "atelier-item-";

/// The stable AccessKit author_id for one draggable item row, sanitizing `item_id` to `[a-z0-9-]` so a
/// raw id with slashes/colons (e.g. a UUID is already safe, but a fabricated id might not be) can never
/// break AccessKit tree integrity (reuses the shell's slugger, the canvas-board pattern).
pub fn item_author_id(item_id: &str) -> String {
    format!("{ITEM_AUTHOR_ID_PREFIX}{}", crate::project_tree::stable_part(item_id))
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
    /// In-flight side-panel load delivery cell (drained at the top of `show`).
    panel_cell: AtelierSidePanelCell,
    /// In-flight per-batch items delivery cell (drained at the top of `show`).
    items_cell: AtelierItemsCell,
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
            panel_cell: Arc::new(Mutex::new(None)),
            items_cell: Arc::new(Mutex::new(None)),
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
    }

    /// Trigger a side-panel load (batches + corpus) if a client is present. Sets [`LoadState::Loading`]
    /// so the spinner shows ONLY while the fetch is genuinely in flight.
    pub fn request_load(&mut self) {
        if let Some(client) = &self.client {
            self.state = LoadState::Loading;
            self.error = None;
            client.fetch_side_panel(Arc::clone(&self.panel_cell));
        }
    }

    /// Expand a batch: collapse any other, then trigger its items load (or no-op in the headless path).
    fn expand_batch(&mut self, batch_id: &str) {
        // Toggle: clicking the expanded batch collapses it.
        if self.expanded.as_ref().map(|(id, _)| id.as_str()) == Some(batch_id) {
            self.expanded = None;
            self.items_loading = None;
            return;
        }
        self.expanded = Some((batch_id.to_owned(), Vec::new()));
        if let Some(client) = &self.client {
            self.items_loading = Some(batch_id.to_owned());
            client.fetch_items(batch_id, Arc::clone(&self.items_cell));
        }
    }

    /// Drain the off-thread delivery cells into the panel state (called at the top of `show`).
    fn drain_cells(&mut self) {
        if let Ok(mut slot) = self.panel_cell.lock() {
            if let Some(result) = slot.take() {
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
        if let Ok(mut slot) = self.items_cell.lock() {
            if let Some((batch_id, result)) = slot.take() {
                // Only apply if this is still the expanded batch (a stale response for a since-collapsed
                // batch is discarded — RISK / MC: no dangling item rows).
                if self.expanded.as_ref().map(|(id, _)| id.as_str()) == Some(batch_id.as_str()) {
                    match result {
                        Ok(items) => {
                            self.expanded = Some((batch_id.clone(), items));
                        }
                        Err(msg) => {
                            self.error = Some(format!("items load failed: {msg}"));
                        }
                    }
                }
                if self.items_loading.as_deref() == Some(batch_id.as_str()) {
                    self.items_loading = None;
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
            ui.label(egui::RichText::new("Atelier / CKC").strong().color(palette.text));
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
            ui.label(egui::RichText::new("Media / Characters / Moodboards").strong().color(palette.text));
            if self.batches.is_empty() {
                ui.label(egui::RichText::new("No intake batches yet.").color(palette.text_subtle));
            }
            // Clone the batch list so we can mutate `self.expanded` while iterating (small list — the
            // backend caps at 200; a clone of {id,label,status} strings is cheap and avoids a borrow
            // conflict with the per-row expand/drag handlers).
            let batches = self.batches.clone();
            for batch in &batches {
                let expanded_here =
                    self.expanded.as_ref().map(|(id, _)| id.as_str()) == Some(batch.batch_id.as_str());
                let marker = if expanded_here { "▼" } else { "▶" };
                let label = format!("{marker} {}  ({})", batch.source_label, batch.status);
                if ui.add(egui::Button::new(label).frame(false)).clicked() {
                    self.expand_batch(&batch.batch_id);
                }
                if expanded_here {
                    self.show_expanded_items(ui, palette);
                }
            }

            ui.separator();

            // ── Section 2: Command Corpus ─────────────────────────────────────────────────────────────
            ui.label(egui::RichText::new("Command Corpus").strong().color(palette.text));
            if self.corpus.is_empty() {
                ui.label(egui::RichText::new("No command-corpus entries.").color(palette.text_subtle));
            }
            for entry in &self.corpus {
                ui.label(
                    egui::RichText::new(format!("• {}  [{}]", entry.action_id, entry.execution_class))
                        .color(palette.text_subtle),
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
        let items = match &self.expanded {
            Some((_, items)) => items.clone(),
            None => return,
        };
        if items.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(egui::RichText::new("(no items in this batch)").color(palette.text_subtle));
            });
            return;
        }
        for item in &items {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                draw_item_row(ui, item, palette);
            });
        }
    }
}

/// The drag payload an item row carries. Each intake item is dragged as a `DragPayload::AtelierRef` with
/// `item_kind = Media` (intake items are media assets; characters/moodboards arrive via their own rows in
/// a richer panel — the intake-items list IS the media source per the verified backend). The label is the
/// file name; `loom_block_id` is `None` (an intake item is not yet a Loom block — the canvas drop then
/// shows the typed "needs a loom block id" state; the rich-text embed still works, an `hsLink` chip).
fn item_drag_payload(item: &AtelierItemRow) -> DragPayload {
    DragPayload::AtelierRef(AtelierRef::new(
        item.item_id.clone(),
        AtelierItemKind::Media,
        item.file_name.clone(),
    ))
}

/// Draw one draggable item row + its ListItem AccessKit node.
fn draw_item_row(ui: &mut egui::Ui, item: &AtelierItemRow, palette: &HsPalette) {
    let payload = item_drag_payload(item);
    let drag_id = egui::Id::new(item_author_id(&item.item_id));
    // The row is the drag SOURCE: dragging it produces the AtelierRef the rich-text / canvas drop zones
    // consume (egui::DragAndDrop, egui 0.33 — the same `dnd_drag_source` the tab bar / canvas use).
    let inner = ui
        .dnd_drag_source(drag_id, payload, |ui| {
            // Path/thumbnail hint (muted) + the file-name label + a kind badge.
            let badge = AtelierItemKind::Media.badge();
            ui.label(
                egui::RichText::new(format!("[{badge}] {}", item.file_name)).color(palette.text),
            );
        })
        .response;
    // Emit the ListItem AccessKit node (the dynamic per-row address). The contract names
    // actions=[Press, StartDrag]; accesskit 0.21.1 has neither variant, so the field-correct mapping is
    // Action::Click + a `draggable` description carrying the ref (the tab_bar drag-source pattern).
    emit_list_item_node(ui, inner.id, &item.item_id, &item.file_name);
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

/// Emit one item row's Role::ListItem AccessKit node (author_id `atelier-item-{item_id}`), with
/// Action::Click and a `draggable` description carrying the atelier ref so an out-of-process agent reads
/// the draggable reference by stable id.
fn emit_list_item_node(ui: &egui::Ui, id: egui::Id, item_id: &str, file_name: &str) {
    let author = item_author_id(item_id);
    let label = file_name.to_owned();
    // The description encodes the draggable affordance + the atelier ref (refKind:item_id) — the
    // field-correct stand-in for the non-existent `StartDrag` action (tab_bar pattern).
    let description = format!("draggable; atelier-ref media:{item_id}");
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::ListItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_description(description.clone());
        node.add_action(accesskit::Action::Click);
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

    fn item(id: &str, name: &str) -> AtelierItemRow {
        AtelierItemRow {
            item_id: id.to_owned(),
            file_name: name.to_owned(),
            source_path: format!("/intake/{name}"),
            lane: "accept".to_owned(),
        }
    }

    /// The item author_id matches the contract's `atelier-item-{id}` shape and sanitizes to `[a-z0-9-]`.
    #[test]
    fn item_author_id_matches_contract_shape() {
        assert_eq!(item_author_id("abc-123"), "atelier-item-abc-123");
        let id = item_author_id("ws:1/item 7#x");
        assert!(id.starts_with(ITEM_AUTHOR_ID_PREFIX));
        let suffix = &id[ITEM_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
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
                // An intake item is not yet a Loom block, so a canvas drop is gated (no fake POST).
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
        assert!(!panel.is_loading(), "a with_rows panel is Loaded, never Loading (no perpetual spinner)");
        assert_eq!(panel.expanded().map(|(id, items)| (id.as_str(), items.len())), Some(("b-1", 1)));
    }

    /// `request_load` on a no-client panel is a benign no-op (stays Loaded, no panic).
    #[test]
    fn request_load_without_client_is_benign() {
        let mut panel = AtelierSidePanel::with_rows(vec![], vec![], None);
        panel.request_load();
        assert!(!panel.is_loading(), "no client -> request_load does not enter Loading");
    }
}
