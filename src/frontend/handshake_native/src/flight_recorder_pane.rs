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
//! ## Events shown
//!
//! The pane queries the real Flight Recorder route and renders both the closed
//! `event_family=native_editor` rows and canonical FEMS `FR-EVT-MEM-001..005` rows. It also surfaces
//! the emitter's in-memory error ring so a failed POST is explained rather than silently blank.

use crate::event_emitter::{EmitErrorEntry, ErrorRing};
use crate::theme::HsPalette;

/// AccessKit author_id for the pane root (Role::Region — the MT contract's exact id).
pub const FLIGHT_RECORDER_PANE_AUTHOR_ID: &str = "flight-recorder-pane";

/// Stable operator/model controls and status surfaces beneath the pane root.
pub const FLIGHT_RECORDER_REFRESH_AUTHOR_ID: &str = "flight-recorder.refresh";
pub const FLIGHT_RECORDER_RETRY_AUTHOR_ID: &str = "flight-recorder.retry";
pub const FLIGHT_RECORDER_ACTION_COMPLETION_AUTHOR_ID: &str = "flight-recorder.action-completion";
pub const FLIGHT_RECORDER_LOAD_FAILURE_AUTHOR_ID: &str = "flight-recorder.load-failure";
pub const FLIGHT_RECORDER_LOADING_STATUS_AUTHOR_ID: &str = "flight-recorder.loading-status";
pub const FLIGHT_RECORDER_QUARANTINE_STATUS_AUTHOR_ID: &str = "flight-recorder.quarantine-status";
pub const FLIGHT_RECORDER_ERROR_RING_AUTHOR_ID: &str = "flight-recorder.error-ring";
pub const FLIGHT_RECORDER_ERROR_ROW_AUTHOR_PREFIX: &str = "flight-recorder.emit-error-";

/// AccessKit author_id PREFIX for one event row: `fr-event-{event_id}` (Role::ListItem).
pub const FR_EVENT_ROW_AUTHOR_PREFIX: &str = "fr-event-";

/// MT-036-specific `DiagEventCode::Other.counter_a` lane for Flight Recorder pane load lifecycle.
/// The diagnostic substrate is a closed numeric allowlist, so the pane uses this exact counter lane
/// instead of inventing a new enum variant outside this MT's allowed path.
pub const MT036_FLIGHT_RECORDER_PANE_DIAG_COUNTER: u64 = 36_001;
/// `DiagEvent.sequence_id` for a real Flight Recorder load beginning.
pub const MT036_FLIGHT_RECORDER_LOAD_START_SEQ: u64 = 1;
/// `DiagEvent.sequence_id` for a Flight Recorder load resolving to visible rows or an honest empty state.
pub const MT036_FLIGHT_RECORDER_LOAD_RECOVERED_SEQ: u64 = 2;
/// `DiagEvent.sequence_id` for a Flight Recorder load resolving to a visible failed state.
pub const MT036_FLIGHT_RECORDER_LOAD_FAILED_SEQ: u64 = 3;

const FLIGHT_RECORDER_ACTION_EFFECT: &str = "mt036.flight-recorder-load";
const FLIGHT_RECORDER_ACTION_CONTEXT: &str = "wp-kernel-012-mt-036-v4";

#[derive(Debug, Clone)]
struct LoadActionCompletion {
    generation: u64,
    state: crate::mcp::action::ClickCompletionState,
    target: Option<String>,
    semantic: Option<String>,
    request_generation: Option<u64>,
    terminal_error: Option<String>,
    terminal_detail: Option<String>,
    retry_suppressed_snapshots_remaining: u8,
    refresh_terminal_ack_latched: bool,
}

impl Default for LoadActionCompletion {
    fn default() -> Self {
        Self {
            generation: 0,
            state: crate::mcp::action::ClickCompletionState::Ready,
            target: None,
            semantic: None,
            request_generation: None,
            terminal_error: None,
            terminal_detail: None,
            retry_suppressed_snapshots_remaining: 0,
            refresh_terminal_ack_latched: false,
        }
    }
}

impl LoadActionCompletion {
    fn target_declaration(&self, target: &str) -> Option<String> {
        let persistent = target == FLIGHT_RECORDER_REFRESH_AUTHOR_ID;
        if self.state == crate::mcp::action::ClickCompletionState::Pending
            || (persistent && self.refresh_terminal_ack_latched)
        {
            if !persistent || self.target.as_deref() != Some(target) {
                return None;
            }
            return crate::mcp::action::serialize_persistent_observer_click_target(
                FLIGHT_RECORDER_ACTION_EFFECT,
                FLIGHT_RECORDER_ACTION_CONTEXT,
                self.generation,
                FLIGHT_RECORDER_ACTION_COMPLETION_AUTHOR_ID,
                self.semantic.as_deref()?,
            );
        }
        let semantic = Self::semantic(target, self.generation.wrapping_add(1));
        if persistent {
            crate::mcp::action::serialize_persistent_observer_click_target(
                FLIGHT_RECORDER_ACTION_EFFECT,
                FLIGHT_RECORDER_ACTION_CONTEXT,
                self.generation,
                FLIGHT_RECORDER_ACTION_COMPLETION_AUTHOR_ID,
                &semantic,
            )
        } else {
            crate::mcp::action::serialize_observer_click_target(
                FLIGHT_RECORDER_ACTION_EFFECT,
                FLIGHT_RECORDER_ACTION_CONTEXT,
                self.generation,
                FLIGHT_RECORDER_ACTION_COMPLETION_AUTHOR_ID,
                &semantic,
            )
        }
    }

    fn semantic(target: &str, action_generation: u64) -> String {
        serde_json::json!({
            "action": if target == FLIGHT_RECORDER_RETRY_AUTHOR_ID { "retry" } else { "refresh" },
            "target": target,
            "next_action_generation": action_generation,
        })
        .to_string()
    }

    fn begin(&mut self, target: &str) {
        if self.state == crate::mcp::action::ClickCompletionState::Pending {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.state = crate::mcp::action::ClickCompletionState::Pending;
        self.target = Some(target.to_owned());
        self.semantic = Some(Self::semantic(target, self.generation));
        self.request_generation = None;
        self.terminal_error = None;
        self.terminal_detail = None;
        self.retry_suppressed_snapshots_remaining = 0;
        self.refresh_terminal_ack_latched = false;
    }

    fn bind_request_generation(&mut self, request_generation: u64) {
        if self.state == crate::mcp::action::ClickCompletionState::Pending {
            self.request_generation = Some(request_generation);
        }
    }

    fn complete_loaded(&mut self, result: &FlightRecorderQueryRows) {
        let Some(request_generation) = self.request_generation else {
            // A refresh clicked while an older request is in flight is queued. The older delivery
            // must not settle the newer click; only begin_loading of its own fetch binds a generation.
            return;
        };
        if self.state != crate::mcp::action::ClickCompletionState::Pending {
            return;
        }
        use sha2::Digest as _;
        let row_identity = result
            .rows
            .iter()
            .map(|row| row.event_id.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let row_ids_sha256 = format!("{:x}", sha2::Sha256::digest(row_identity.as_bytes()));
        self.state = crate::mcp::action::ClickCompletionState::Applied;
        self.refresh_terminal_ack_latched =
            self.target.as_deref() == Some(FLIGHT_RECORDER_REFRESH_AUTHOR_ID);
        self.terminal_detail = Some(
            serde_json::json!({
                "request_generation": request_generation,
                "row_count": result.rows.len(),
                "row_ids_sha256": row_ids_sha256,
                "quarantined_count": result.quarantined.len(),
                "load_state": "loaded",
            })
            .to_string(),
        );
    }

    fn complete_failed(&mut self, error: &str) {
        let Some(request_generation) = self.request_generation else {
            return;
        };
        if self.state != crate::mcp::action::ClickCompletionState::Pending {
            return;
        }
        self.state = crate::mcp::action::ClickCompletionState::Failed;
        self.retry_suppressed_snapshots_remaining =
            u8::from(self.target.as_deref() == Some(FLIGHT_RECORDER_RETRY_AUTHOR_ID));
        self.refresh_terminal_ack_latched =
            self.target.as_deref() == Some(FLIGHT_RECORDER_REFRESH_AUTHOR_ID);
        self.terminal_error = Some(error.to_owned());
        self.terminal_detail = Some(
            serde_json::json!({
                "request_generation": request_generation,
                "load_state": "failed",
                "error": error,
            })
            .to_string(),
        );
    }

    fn retry_terminal_ack_suppressed(&self) -> bool {
        self.retry_suppressed_snapshots_remaining > 0
    }

    fn observer_value(&self) -> Option<String> {
        match self.state {
            crate::mcp::action::ClickCompletionState::Ready
            | crate::mcp::action::ClickCompletionState::Pending => {
                crate::mcp::action::serialize_observer_click_state(
                    FLIGHT_RECORDER_ACTION_EFFECT,
                    FLIGHT_RECORDER_ACTION_CONTEXT,
                    self.generation,
                    self.state,
                    self.target.as_deref(),
                    self.semantic.as_deref(),
                )
            }
            crate::mcp::action::ClickCompletionState::Applied => {
                crate::mcp::action::serialize_observer_click_applied(
                    FLIGHT_RECORDER_ACTION_EFFECT,
                    FLIGHT_RECORDER_ACTION_CONTEXT,
                    self.generation,
                    self.target.as_deref()?,
                    self.semantic.as_deref()?,
                    self.terminal_detail.as_deref()?,
                )
            }
            crate::mcp::action::ClickCompletionState::Failed => {
                crate::mcp::action::serialize_observer_click_failure(
                    FLIGHT_RECORDER_ACTION_EFFECT,
                    FLIGHT_RECORDER_ACTION_CONTEXT,
                    self.generation,
                    self.target.as_deref()?,
                    self.semantic.as_deref()?,
                    self.terminal_error.as_deref()?,
                    self.terminal_detail.as_deref(),
                )
            }
        }
    }
}

/// The stable AccessKit author_id for one event row (`fr-event-{event_id}`). The event id is sanitized
/// to `[A-Za-z0-9-]` so an arbitrary id yields a safe address.
pub fn fr_event_row_author_id(event_id: &str) -> String {
    let safe: String = event_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
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
    /// Canonical FEMS event code when this is a memory lifecycle row. Native-editor rows have none.
    pub event_code: Option<String>,
    /// The acting actor id.
    pub actor_id: String,
    /// The RFC3339 timestamp string.
    pub ts_utc: String,
}

/// One completed backend projection plus operator-visible quarantine diagnostics. Malformed native
/// rows never poison valid neighbors, but they also never collapse into a misleading empty history.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlightRecorderQueryRows {
    pub rows: Vec<FlightRecorderRow>,
    pub quarantined: Vec<String>,
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
    Loaded(FlightRecorderQueryRows),
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
    fn rows(&self) -> Result<FlightRecorderQueryRows, String>;
}

/// The Flight Recorder pane state owned across frames. Holds the query seam, the current [`LoadState`],
/// and a handle to the emitter's [`ErrorRing`] so the empty state is explained.
pub struct FlightRecorderPane {
    query: std::sync::Arc<dyn FlightRecorderQuery>,
    state: LoadState,
    error_ring: ErrorRing,
    refresh_requested: std::sync::atomic::AtomicBool,
    action_completion: std::sync::Mutex<LoadActionCompletion>,
    active_request_generation: Option<u64>,
}

impl FlightRecorderPane {
    /// Build a pane reading through `query`, surfacing `error_ring` failures. Starts [`LoadState::Idle`].
    pub fn new(query: std::sync::Arc<dyn FlightRecorderQuery>, error_ring: ErrorRing) -> Self {
        Self {
            query,
            state: LoadState::Idle,
            error_ring,
            refresh_requested: std::sync::atomic::AtomicBool::new(false),
            action_completion: std::sync::Mutex::new(LoadActionCompletion::default()),
            active_request_generation: None,
        }
    }

    /// The current load state (tests / diagnostics).
    pub fn state(&self) -> &LoadState {
        &self.state
    }

    /// Mark a real query as in flight. The shell calls this only after it has atomically claimed the
    /// single fetch slot, so `Loading` always corresponds to an owned request and can never become a
    /// decorative or perpetual spinner.
    pub fn begin_loading(&mut self, request_generation: u64) {
        self.state = LoadState::Loading;
        self.active_request_generation = Some(request_generation);
        if let Ok(mut completion) = self.action_completion.lock() {
            completion.bind_request_generation(request_generation);
        }
        record_flight_recorder_pane_diagnostic(
            MT036_FLIGHT_RECORDER_LOAD_START_SEQ,
            handshake_diag_ring::DiagPhase::Start,
            handshake_diag_ring::DiagSeverity::Info,
            request_generation,
            0,
        );
    }

    /// Run a load through the query seam, transitioning the state. This is the one-shot resolve (no
    /// perpetual spinner): `Idle`/`Loading` → `Loaded(rows)` or `Failed(reason)`. The production caller
    /// invokes this when a queued async fetch's delivery cell resolves; a test calls it directly.
    pub fn load_now(&mut self) {
        match self.query.rows() {
            Ok(rows) => {
                let request_generation = self.active_request_generation.take().unwrap_or(0);
                let row_count = rows.rows.len() as u64;
                if let Ok(mut completion) = self.action_completion.lock() {
                    completion.complete_loaded(&rows);
                }
                self.state = LoadState::Loaded(rows);
                record_flight_recorder_pane_diagnostic(
                    MT036_FLIGHT_RECORDER_LOAD_RECOVERED_SEQ,
                    handshake_diag_ring::DiagPhase::Recovered,
                    handshake_diag_ring::DiagSeverity::Info,
                    request_generation,
                    row_count,
                );
            }
            Err(e) => {
                let request_generation = self.active_request_generation.take().unwrap_or(0);
                let reason_len = e.len() as u64;
                if let Ok(mut completion) = self.action_completion.lock() {
                    completion.complete_failed(&e);
                }
                self.state = LoadState::Failed(e);
                record_flight_recorder_pane_diagnostic(
                    MT036_FLIGHT_RECORDER_LOAD_FAILED_SEQ,
                    handshake_diag_ring::DiagPhase::Degraded,
                    handshake_diag_ring::DiagSeverity::Warn,
                    request_generation,
                    reason_len,
                );
            }
        }
    }

    /// Render the pane into `ui` with the active `palette`. Emits the `flight-recorder-pane` Region root
    /// and one `fr-event-{id}` ListItem per row through the existing accessibility live path. Returns the
    /// root response (so a host can size/position it). Theme tokens only — NO `Color32` literal.
    pub fn show(&self, ui: &mut egui::Ui, palette: &HsPalette) -> egui::Response {
        let resp = ui
            .scope(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Flight Recorder — Native Editor + FEMS Events")
                            .color(palette.text),
                    );
                    let refresh = ui.button("Refresh");
                    crate::accessibility::emit_interactive_node(
                        ui.ctx(),
                        refresh.id,
                        FLIGHT_RECORDER_REFRESH_AUTHOR_ID,
                    );
                    if refresh.clicked() {
                        if let Ok(mut completion) = self.action_completion.lock() {
                            completion.begin(FLIGHT_RECORDER_REFRESH_AUTHOR_ID);
                        }
                        self.refresh_requested
                            .store(true, std::sync::atomic::Ordering::Release);
                    }
                    if let Ok(completion) = self.action_completion.lock() {
                        if let Some(value) =
                            completion.target_declaration(FLIGHT_RECORDER_REFRESH_AUTHOR_ID)
                        {
                            ui.ctx().accesskit_node_builder(refresh.id, |node| {
                                node.set_value(value.clone());
                            });
                        }
                    }
                });
                match &self.state {
                    LoadState::Idle => {
                        ui.label(
                            egui::RichText::new("No load requested.").color(palette.text_subtle),
                        );
                    }
                    LoadState::Loading => {
                        let loading =
                            ui.label(egui::RichText::new("Loading…").color(palette.text_subtle));
                        ui.ctx().accesskit_node_builder(loading.id, |node| {
                            node.set_role(egui::accesskit::Role::Status);
                            node.set_author_id(FLIGHT_RECORDER_LOADING_STATUS_AUTHOR_ID.to_owned());
                            node.set_label("Flight Recorder loading".to_owned());
                            node.set_value(
                                serde_json::json!({
                                    "state": "loading",
                                    "active_request_generation": self.active_request_generation,
                                    "request": "one bounded workspace-scoped GET in flight",
                                })
                                .to_string(),
                            );
                        });
                    }
                    LoadState::Loaded(result) if result.rows.is_empty() => {
                        ui.label(
                            egui::RichText::new(if result.quarantined.is_empty() {
                                "No native editor or FEMS events yet.".to_owned()
                            } else {
                                format!(
                                    "No valid native editor or FEMS events. Rejected {} malformed row(s).",
                                    result.quarantined.len()
                                )
                            })
                            .color(palette.text_subtle),
                        );
                        if !result.quarantined.is_empty() {
                            self.show_quarantine_status(ui, palette, result);
                        }
                    }
                    LoadState::Loaded(result) => {
                        for row in &result.rows {
                            self.show_event_row(ui, palette, row);
                        }
                        if !result.quarantined.is_empty() {
                            self.show_quarantine_status(ui, palette, result);
                        }
                    }
                    LoadState::Failed(reason) => {
                        let failure = ui.label(
                            egui::RichText::new(format!("Load failed: {reason}"))
                                .color(palette.error_text),
                        );
                        ui.ctx().accesskit_node_builder(failure.id, |node| {
                            node.set_role(egui::accesskit::Role::Status);
                            node.set_author_id(FLIGHT_RECORDER_LOAD_FAILURE_AUTHOR_ID.to_owned());
                            node.set_label("Flight Recorder load failure".to_owned());
                            node.set_value(reason.clone());
                        });
                        // A failed Retry must first publish its durable Failed observer while the
                        // transient Retry target is absent. Otherwise ActionChannel sees a post-click
                        // node with the same author_id and can only classify the action Indeterminate.
                        // The following frame remounts a fresh Retry declaration for a new action.
                        let suppress_retry = self
                            .action_completion
                            .lock()
                            .map(|completion| completion.retry_terminal_ack_suppressed())
                            .unwrap_or(false);
                        if !suppress_retry {
                            let retry = ui.button("Retry");
                            crate::accessibility::emit_interactive_node(
                                ui.ctx(),
                                retry.id,
                                FLIGHT_RECORDER_RETRY_AUTHOR_ID,
                            );
                            if retry.clicked() {
                                if let Ok(mut completion) = self.action_completion.lock() {
                                    completion.begin(FLIGHT_RECORDER_RETRY_AUTHOR_ID);
                                }
                                self.refresh_requested
                                    .store(true, std::sync::atomic::Ordering::Release);
                            }
                            if let Ok(completion) = self.action_completion.lock() {
                                if let Some(value) =
                                    completion.target_declaration(FLIGHT_RECORDER_RETRY_AUTHOR_ID)
                                {
                                    ui.ctx().accesskit_node_builder(retry.id, |node| {
                                        node.set_value(value.clone());
                                    });
                                }
                            }
                        }
                    }
                }
                if let Ok(completion) = self.action_completion.lock() {
                    if let Some(value) = completion.observer_value() {
                        // This durable observer follows state-dependent row/failure widgets. It cannot
                        // use any response/auto ID: those are layout-derived and drift when Loading
                        // becomes Loaded/Failed. The explicit ID is its stable AccessKit identity.
                        let observer_id = egui::Id::new(FLIGHT_RECORDER_ACTION_COMPLETION_AUTHOR_ID);
                        let observer_rect = egui::Rect::from_min_size(
                            ui.next_widget_position(),
                            egui::Vec2::ZERO,
                        );
                        // Register the fixed id in egui's interaction/parent map before building its
                        // AccessKit node; an unattached builder is not a valid live-tree observer.
                        ui.interact(observer_rect, observer_id, egui::Sense::hover());
                        ui.ctx().accesskit_node_builder(observer_id, |node| {
                            node.set_role(egui::accesskit::Role::Status);
                            node.set_author_id(
                                FLIGHT_RECORDER_ACTION_COMPLETION_AUTHOR_ID.to_owned(),
                            );
                            node.set_label("Flight Recorder action completion".to_owned());
                            node.set_value(value.clone());
                        });
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
            "Flight Recorder native editor and FEMS events",
        );
        resp
    }

    /// Consume the operator's explicit refresh request.
    pub fn take_refresh_requested(&self) -> bool {
        self.refresh_requested
            .swap(false, std::sync::atomic::Ordering::AcqRel)
    }

    /// Release action terminal latches only after the app has projected, ActionChannel-acknowledged,
    /// and published that authoritative snapshot. This preserves the clicked Refresh semantic through
    /// Applied/Failed acknowledgement and keeps a failed Retry absent for its terminal snapshot.
    pub fn acknowledge_action_terminal_snapshot(&self) {
        if let Ok(mut completion) = self.action_completion.lock() {
            completion.retry_suppressed_snapshots_remaining = completion
                .retry_suppressed_snapshots_remaining
                .saturating_sub(1);
            completion.refresh_terminal_ack_latched = false;
        }
    }

    /// Render one event row + emit its `fr-event-{id}` ListItem AccessKit node.
    fn show_event_row(&self, ui: &mut egui::Ui, palette: &HsPalette, row: &FlightRecorderRow) {
        let action = row
            .event_code
            .as_deref()
            .map(|code| format!("{code} {}", row.action))
            .unwrap_or_else(|| row.action.clone());
        let text = format!("{}  ·  {}  ·  {}", action, row.actor_id, row.ts_utc);
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

    fn show_quarantine_status(
        &self,
        ui: &mut egui::Ui,
        palette: &HsPalette,
        result: &FlightRecorderQueryRows,
    ) {
        let value = format!(
            "Rejected {} malformed Flight Recorder row(s): {}",
            result.quarantined.len(),
            result.quarantined.join("; ")
        );
        let response = ui.label(egui::RichText::new(&value).color(palette.error_text));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Status);
            node.set_author_id(FLIGHT_RECORDER_QUARANTINE_STATUS_AUTHOR_ID.to_owned());
            node.set_label("Flight Recorder quarantined rows".to_owned());
            node.set_value(value.clone());
        });
    }

    /// Render the emitter error ring (so an empty event list is EXPLAINED, not silently blank).
    fn show_error_ring(&self, ui: &mut egui::Ui, palette: &HsPalette) {
        let entries = self.error_ring.entries();
        if entries.is_empty() {
            return;
        }
        let heading = ui.label(
            egui::RichText::new(format!("Emit failures ({}):", entries.len()))
                .color(palette.text_subtle),
        );
        ui.ctx().accesskit_node_builder(heading.id, |node| {
            node.set_role(egui::accesskit::Role::Status);
            node.set_author_id(FLIGHT_RECORDER_ERROR_RING_AUTHOR_ID.to_owned());
            node.set_label("Flight Recorder emit failures".to_owned());
        });
        for (index, EmitErrorEntry { action, error }) in entries.iter().enumerate() {
            let value = format!("{action}: {error}");
            let response =
                ui.label(egui::RichText::new(format!("  {value}")).color(palette.error_text));
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(egui::accesskit::Role::ListItem);
                node.set_author_id(format!("{FLIGHT_RECORDER_ERROR_ROW_AUTHOR_PREFIX}{index}"));
                node.set_label(format!("Flight Recorder emit failure {index}"));
                node.set_value(value.clone());
            });
        }
    }
}

fn record_flight_recorder_pane_diagnostic(
    sequence_id: u64,
    phase: handshake_diag_ring::DiagPhase,
    severity: handshake_diag_ring::DiagSeverity,
    counter_b: u64,
    metric_micros: u64,
) {
    let timestamp_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    crate::diagnostics::record_with(
        handshake_diag_ring::DiagEventCode::Other,
        phase,
        severity,
        0,
        sequence_id,
        MT036_FLIGHT_RECORDER_PANE_DIAG_COUNTER,
        counter_b,
        metric_micros,
        timestamp_nanos,
    );
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
        fn rows(&self) -> Result<FlightRecorderQueryRows, String> {
            match &self.fail {
                Some(e) => Err(e.clone()),
                None => Ok(FlightRecorderQueryRows {
                    rows: self.rows.clone(),
                    quarantined: Vec::new(),
                }),
            }
        }
    }

    fn row(id: &str, action: &str) -> FlightRecorderRow {
        FlightRecorderRow {
            event_id: id.to_owned(),
            action: action.to_owned(),
            event_code: None,
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
            LoadState::Loaded(result) => {
                assert_eq!(result.rows.len(), 1);
                assert_eq!(result.rows[0].action, "document_saved");
            }
            other => panic!("expected Loaded, got {other:?}"),
        }
    }

    #[test]
    fn load_failure_is_a_typed_failed_state_not_a_spinner() {
        let query = Arc::new(MockQuery {
            rows: vec![],
            fail: Some("backend unreachable".to_owned()),
        });
        let mut pane = FlightRecorderPane::new(query, ErrorRing::new());
        pane.load_now();
        assert_eq!(
            *pane.state(),
            LoadState::Failed("backend unreachable".to_owned())
        );
        // Crucially NOT Loading: there is no perpetual spinner.
        assert!(!matches!(pane.state(), LoadState::Loading));
    }

    #[test]
    fn empty_load_is_an_honest_empty_state() {
        let query = Arc::new(MockQuery {
            rows: vec![],
            fail: None,
        });
        let mut pane = FlightRecorderPane::new(query, ErrorRing::new());
        pane.load_now();
        assert_eq!(
            *pane.state(),
            LoadState::Loaded(FlightRecorderQueryRows::default())
        );
    }

    #[test]
    fn event_row_author_id_is_sanitized() {
        assert_eq!(fr_event_row_author_id("FR-EVT-001"), "fr-event-FR-EVT-001");
        assert_eq!(fr_event_row_author_id("a b/c"), "fr-event-a-b-c");
    }
}
