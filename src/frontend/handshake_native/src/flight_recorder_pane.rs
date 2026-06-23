//! WP-KERNEL-012 MT-036 (E5 — Flight Recorder observability pane).
//!
//! [`FlightRecorderPane`] is the native port of the React `FlightRecorderView.tsx`: it lists the native
//! editor events the Flight Recorder ledger holds so a no-context model and the operator can SEE what the
//! editors are doing (HBR-VIS / HBR-SWARM). It reuses the existing accessibility live-emission path
//! (`crate::accessibility::emit_*`) for AccessKit wiring and the theme palette for all colors (NO
//! `Color32` literal — the no-hardcode invariant).
//!
//! ## No perpetual spinner (the MT-015 lesson)
//!
//! The pane drives a typed [`LoadState`]: it shows a one-shot "Loading…" ONLY while a load is genuinely
//! in flight, then transitions to `Loaded` (with rows or an honest empty state) or `Failed` (with the
//! reason). It NEVER shows an indefinite spinner: a load with no runtime / no backend resolves to a typed
//! empty/failed state, not a hang.
//!
//! ## Events shown + the backend-shape caveat (the TYPED BLOCKER, see `event_emitter.rs`)
//!
//! The pane queries the ledger through the [`FlightRecorderQuery`] seam and renders the rows it returns.
//! The native-editor → ledger ROUND-TRIP is currently a typed backend blocker (the real backend has no
//! ingestion endpoint that records a native-editor event with a custom actor/action — see
//! `event_emitter.rs`), so against a live backend today the native rows will be empty until that endpoint
//! lands. The pane ALSO surfaces the emitter's in-memory error ring so the empty state is EXPLAINED
//! (e.g. "emit dropped: backpressure") rather than silently blank. The widget itself is fully proven
//! standalone by injecting events through the query seam.

use crate::event_emitter::{EmitErrorEntry, ErrorRing};
use crate::theme::HsPalette;

/// AccessKit author_id for the pane root (Role::Region — the MT contract's exact id).
pub const FLIGHT_RECORDER_PANE_AUTHOR_ID: &str = "flight-recorder-pane";

/// AccessKit author_id PREFIX for one event row: `fr-event-{event_id}` (Role::ListItem).
pub const FR_EVENT_ROW_AUTHOR_PREFIX: &str = "fr-event-";

/// The stable AccessKit author_id for one event row (`fr-event-{event_id}`). The event id is sanitized
/// to `[A-Za-z0-9-]` so an arbitrary id yields a safe address.
pub fn fr_event_row_author_id(event_id: &str) -> String {
    let safe: String = event_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    format!("{FR_EVENT_ROW_AUTHOR_PREFIX}{safe}")
}

/// One native-editor flight-recorder row the pane renders. A reduced projection of the backend
/// `FlightEvent`: the fields the pane shows (the event id for the stable a11y address, the action label,
/// the actor, the timestamp). Built either from a live `FlightEvent` query response or from the
/// emitter-side projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlightRecorderRow {
    /// The backend event id (the `fr-event-{id}` address).
    pub event_id: String,
    /// The native-editor action (e.g. `document_saved`) for the row label.
    pub action: String,
    /// The acting actor id.
    pub actor_id: String,
    /// The RFC3339 timestamp string.
    pub ts_utc: String,
}

/// The load state of the pane (NO perpetual spinner — MT-015). Drives exactly one of: a one-shot
/// loading line, the loaded rows (or honest empty state), or a typed failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadState {
    /// Nothing requested yet.
    Idle,
    /// A load is genuinely in flight (a single frame's "Loading…", not an indefinite spinner).
    Loading,
    /// The load completed; carries the rows (possibly empty — an HONEST empty state).
    Loaded(Vec<FlightRecorderRow>),
    /// The load failed; carries the reason (shown, not hidden).
    Failed(String),
}

/// The query seam the pane reads through (the `GET /flight_recorder` consumer). A `Send + Sync` trait so
/// the production reqwest query and a unit mock are interchangeable, and so a kittest injects rows
/// without a live backend.
pub trait FlightRecorderQuery: Send + Sync {
    /// Synchronously return the current native-editor rows (the headless/test path injects directly;
    /// the production impl would resolve a completed async fetch's delivery cell here, never blocking
    /// the frame).
    fn rows(&self) -> Result<Vec<FlightRecorderRow>, String>;
}

/// The Flight Recorder pane state owned across frames. Holds the query seam, the current [`LoadState`],
/// and a handle to the emitter's [`ErrorRing`] so the empty state is explained.
pub struct FlightRecorderPane {
    query: std::sync::Arc<dyn FlightRecorderQuery>,
    state: LoadState,
    error_ring: ErrorRing,
}

impl FlightRecorderPane {
    /// Build a pane reading through `query`, surfacing `error_ring` failures. Starts [`LoadState::Idle`].
    pub fn new(query: std::sync::Arc<dyn FlightRecorderQuery>, error_ring: ErrorRing) -> Self {
        Self { query, state: LoadState::Idle, error_ring }
    }

    /// The current load state (tests / diagnostics).
    pub fn state(&self) -> &LoadState {
        &self.state
    }

    /// Run a load through the query seam, transitioning the state. This is the one-shot resolve (no
    /// perpetual spinner): `Idle`/`Loading` → `Loaded(rows)` or `Failed(reason)`. The production caller
    /// invokes this when a queued async fetch's delivery cell resolves; a test calls it directly.
    pub fn load_now(&mut self) {
        match self.query.rows() {
            Ok(rows) => self.state = LoadState::Loaded(rows),
            Err(e) => self.state = LoadState::Failed(e),
        }
    }

    /// Render the pane into `ui` with the active `palette`. Emits the `flight-recorder-pane` Region root
    /// and one `fr-event-{id}` ListItem per row through the existing accessibility live path. Returns the
    /// root response (so a host can size/position it). Theme tokens only — NO `Color32` literal.
    pub fn show(&self, ui: &mut egui::Ui, palette: &HsPalette) -> egui::Response {
        let resp = ui
            .scope(|ui| {
                ui.label(egui::RichText::new("Flight Recorder — Native Editor Events").color(palette.text));
                match &self.state {
                    LoadState::Idle => {
                        ui.label(egui::RichText::new("No load requested.").color(palette.text_subtle));
                    }
                    LoadState::Loading => {
                        ui.label(egui::RichText::new("Loading…").color(palette.text_subtle));
                    }
                    LoadState::Loaded(rows) if rows.is_empty() => {
                        ui.label(
                            egui::RichText::new("No native editor events yet.").color(palette.text_subtle),
                        );
                    }
                    LoadState::Loaded(rows) => {
                        for row in rows {
                            self.show_event_row(ui, palette, row);
                        }
                    }
                    LoadState::Failed(reason) => {
                        ui.label(
                            egui::RichText::new(format!("Load failed: {reason}"))
                                .color(palette.error_text),
                        );
                    }
                }
                self.show_error_ring(ui, palette);
            })
            .response;

        // Emit the pane root as a Region (the contract's flight-recorder-pane / Role::Region).
        crate::accessibility::emit_pane_node(
            ui.ctx(),
            resp.id,
            FLIGHT_RECORDER_PANE_AUTHOR_ID,
            egui::accesskit::Role::Region,
            "Flight Recorder native editor events",
        );
        resp
    }

    /// Render one event row + emit its `fr-event-{id}` ListItem AccessKit node.
    fn show_event_row(&self, ui: &mut egui::Ui, palette: &HsPalette, row: &FlightRecorderRow) {
        let text = format!("{}  ·  {}  ·  {}", row.action, row.actor_id, row.ts_utc);
        let resp = ui.label(egui::RichText::new(&text).color(palette.text));
        let author_id = fr_event_row_author_id(&row.event_id);
        let label = format!("Flight recorder event {}", row.action);
        let value = text.clone();
        ui.ctx().accesskit_node_builder(resp.id, move |node| {
            node.set_role(egui::accesskit::Role::ListItem);
            node.set_author_id(author_id.clone());
            node.set_label(label.clone());
            node.set_value(value.clone());
        });
    }

    /// Render the emitter error ring (so an empty event list is EXPLAINED, not silently blank).
    fn show_error_ring(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        let entries = self.error_ring.entries();
        if entries.is_empty() {
            return;
        }
        ui.label(
            egui::RichText::new(format!("Emit failures ({}):", entries.len())).color(palette.text_subtle),
        );
        for EmitErrorEntry { action, error } in &entries {
            ui.label(
                egui::RichText::new(format!("  {action}: {error}")).color(palette.error_text),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockQuery {
        rows: Vec<FlightRecorderRow>,
        fail: Option<String>,
    }
    impl FlightRecorderQuery for MockQuery {
        fn rows(&self) -> Result<Vec<FlightRecorderRow>, String> {
            match &self.fail {
                Some(e) => Err(e.clone()),
                None => Ok(self.rows.clone()),
            }
        }
    }

    fn row(id: &str, action: &str) -> FlightRecorderRow {
        FlightRecorderRow {
            event_id: id.to_owned(),
            action: action.to_owned(),
            actor_id: "hsk:native_editor:pane-rich".to_owned(),
            ts_utc: "2026-06-23T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn load_now_transitions_to_loaded_with_rows() {
        let query = Arc::new(MockQuery {
            rows: vec![row("FR-1", "document_saved")],
            fail: None,
        });
        let mut pane = FlightRecorderPane::new(query, ErrorRing::new());
        assert_eq!(*pane.state(), LoadState::Idle);
        pane.load_now();
        match pane.state() {
            LoadState::Loaded(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].action, "document_saved");
            }
            other => panic!("expected Loaded, got {other:?}"),
        }
    }

    #[test]
    fn load_failure_is_a_typed_failed_state_not_a_spinner() {
        let query = Arc::new(MockQuery { rows: vec![], fail: Some("backend unreachable".to_owned()) });
        let mut pane = FlightRecorderPane::new(query, ErrorRing::new());
        pane.load_now();
        assert_eq!(*pane.state(), LoadState::Failed("backend unreachable".to_owned()));
        // Crucially NOT Loading: there is no perpetual spinner.
        assert!(!matches!(pane.state(), LoadState::Loading));
    }

    #[test]
    fn empty_load_is_an_honest_empty_state() {
        let query = Arc::new(MockQuery { rows: vec![], fail: None });
        let mut pane = FlightRecorderPane::new(query, ErrorRing::new());
        pane.load_now();
        assert_eq!(*pane.state(), LoadState::Loaded(vec![]));
    }

    #[test]
    fn event_row_author_id_is_sanitized() {
        assert_eq!(fr_event_row_author_id("FR-EVT-001"), "fr-event-FR-EVT-001");
        assert_eq!(fr_event_row_author_id("a b/c"), "fr-event-a-b-c");
    }
}
