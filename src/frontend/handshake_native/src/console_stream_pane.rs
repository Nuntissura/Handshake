//! WP-1 live orchestration debug console pane.
//!
//! Tails the backend SSE endpoint `GET /wp1/diagnostics/console/stream`
//! (`hsk.wp1_console_entry@1`) and renders the streamed entries live, REUSING the
//! existing [`crate::debug_console::DebugConsole`] widget (stable
//! `console_row_{index}` AccessKit author_ids + a display filter). Live entries
//! are appended as they arrive (live-append) and the scroll area sticks to the
//! bottom (auto-follow). The console is a DISPLAY buffer only — the durable
//! authority for every event is the backend's PostgreSQL/EventLedger + Flight
//! Recorder, mirrored by the backend console tee; nothing here is authority.
//!
//! Off the UI thread: the SSE tail runs on the app's tokio runtime and appends
//! into a shared delivery buffer the egui frame thread drains each frame (the
//! same off-thread + delivery-cell shape the other native backend clients use),
//! so the render thread is never blocked on the network (HBR-QUIET).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use egui::accesskit;
use serde::Deserialize;

use crate::context_menu_surfaces::ConsoleEntryKind;
use crate::debug_console::{ConsoleEntry as DebugConsoleRow, DebugConsole, DebugConsoleColors};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

/// One entry as delivered by the backend SSE console
/// (`hsk.wp1_console_entry@1`). Deserialized as plain strings so the native
/// crate never depends on `handshake_core`'s types.
#[derive(Debug, Clone, Deserialize)]
pub struct ConsoleStreamEntry {
    pub seq: u64,
    #[serde(default)]
    pub ts_unix_ms: u64,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// Shared, bounded delivery buffer the off-thread SSE tail appends into and the
/// egui frame thread drains each frame.
pub type ConsoleStreamBuffer = Arc<Mutex<VecDeque<ConsoleStreamEntry>>>;

/// Begins (once) a live SSE tail of the backend console into a delivery buffer.
/// Production wires [`crate::backend_client::ConsoleStreamClient`]; tests can push
/// entries into the buffer directly (no transport needed).
pub trait ConsoleStreamTransport: Send + Sync {
    fn start_tail(&self, buffer: ConsoleStreamBuffer);
}

struct ConsolePaneUiState {
    console: DebugConsole,
    /// Highest backend `seq` already appended — the auto-follow/dedupe cursor so
    /// the connect-time replay + live tail never double-append.
    last_seq: Option<u64>,
    tail_started: bool,
}

impl Default for ConsolePaneUiState {
    fn default() -> Self {
        Self {
            console: DebugConsole::default(),
            last_seq: None,
            tail_started: false,
        }
    }
}

/// The pane factory: owns the reused [`DebugConsole`], the shared delivery
/// buffer, and the optional SSE transport.
pub struct ConsoleStreamPaneFactory {
    state: Arc<Mutex<ConsolePaneUiState>>,
    buffer: ConsoleStreamBuffer,
    transport: Option<Arc<dyn ConsoleStreamTransport>>,
}

impl ConsoleStreamPaneFactory {
    /// No transport (no live tail). Renders whatever is pushed into its buffer.
    pub fn offline() -> Self {
        Self {
            state: Arc::new(Mutex::new(ConsolePaneUiState::default())),
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            transport: None,
        }
    }

    /// Production: tail the live SSE endpoint through `transport`.
    pub fn with_transport(transport: Arc<dyn ConsoleStreamTransport>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ConsolePaneUiState::default())),
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            transport: Some(transport),
        }
    }

    /// Test seam: seed the delivery buffer with entries (rendered on the next
    /// frame through the SAME live-append path as the SSE tail).
    pub fn with_entries(entries: Vec<ConsoleStreamEntry>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ConsolePaneUiState::default())),
            buffer: Arc::new(Mutex::new(entries.into_iter().collect())),
            transport: None,
        }
    }

    /// The shared delivery buffer (production wires the SSE client into it; tests
    /// can push directly).
    pub fn buffer(&self) -> ConsoleStreamBuffer {
        self.buffer.clone()
    }

    /// Drain any newly-delivered entries into the reused DebugConsole (live-append
    /// + monotonic-seq dedupe), then render a category filter row + the console.
    fn show(&self, ui: &mut egui::Ui) {
        // Start the live tail once (idempotent).
        if let Some(transport) = &self.transport {
            let start = {
                let mut state = self.state.lock().expect("console pane state poisoned");
                if state.tail_started {
                    false
                } else {
                    state.tail_started = true;
                    true
                }
            };
            if start {
                transport.start_tail(self.buffer.clone());
            }
        }

        let mut state = self.state.lock().expect("console pane state poisoned");

        // Drain the delivery buffer into the display console (live-append).
        let drained: Vec<ConsoleStreamEntry> = {
            let mut buffer = self.buffer.lock().expect("console buffer poisoned");
            buffer.drain(..).collect()
        };
        for entry in drained {
            if state.last_seq.is_none_or(|last| entry.seq > last) {
                state.last_seq = Some(entry.seq);
                let kind = kind_for_severity(&entry.severity);
                let text = row_text(&entry);
                state.console.entries.push(DebugConsoleRow::new(kind, text));
            }
        }

        ui.heading("WP-1 Live Orchestration Console");
        ui.horizontal(|ui| {
            ui.label("Filter:");
            let mut set_filter: Option<Option<ConsoleEntryKind>> = None;
            if filter_button(ui, "Show All", FILTER_ALL_AUTHOR_ID) {
                set_filter = Some(None);
            }
            if filter_button(ui, "Errors", FILTER_ERRORS_AUTHOR_ID) {
                set_filter = Some(Some(ConsoleEntryKind::Error));
            }
            if filter_button(ui, "Info", FILTER_INFO_AUTHOR_ID) {
                set_filter = Some(Some(ConsoleEntryKind::Output));
            }
            if filter_button(ui, "Debug", FILTER_DEBUG_AUTHOR_ID) {
                set_filter = Some(Some(ConsoleEntryKind::Input));
            }
            if let Some(filter) = set_filter {
                state.console.filter = filter;
            }
        });

        let colors = DebugConsoleColors {
            row_bg: ui.visuals().extreme_bg_color,
            row_hover_bg: ui.visuals().faint_bg_color,
            row_text: ui.visuals().text_color(),
        };
        // Auto-follow: stick to the bottom so the newest live entry stays visible.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                state.console.show(ui, colors);
            });
    }
}

impl PaneFactory for ConsoleStreamPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::Wp1OrchestrationConsole
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        self.show(ui);
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Pane
    }
}

/// Stable AccessKit author_ids for the category-filter buttons, so a headless
/// kittest can address and click them (a plain egui button is not reliably
/// pointer-clickable headless; attaching an explicit `Action::Click` accesskit
/// node makes `click_accesskit()` fire the button — the same pattern the swarm
/// diagnostics pane uses for its Refresh control).
pub const FILTER_ALL_AUTHOR_ID: &str = "wp1-console.filter.all";
pub const FILTER_ERRORS_AUTHOR_ID: &str = "wp1-console.filter.errors";
pub const FILTER_INFO_AUTHOR_ID: &str = "wp1-console.filter.info";
pub const FILTER_DEBUG_AUTHOR_ID: &str = "wp1-console.filter.debug";

/// A filter button that is reliably clickable headlessly: it renders a standard
/// egui button AND publishes an AccessKit node (Role::Button + Action::Click +
/// stable author_id) on the button's response id. Returns whether it was clicked.
fn filter_button(ui: &mut egui::Ui, label: &str, author_id: &str) -> bool {
    let response = ui.button(label);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(accesskit::Role::Button);
        node.add_action(accesskit::Action::Click);
        node.set_author_id(author_id.to_owned());
        node.set_label(label.to_owned());
    });
    response.clicked()
}

/// Map backend severity to the reused DebugConsole entry kind so the display
/// filter can narrow by class. `warn`/`error` -> Error, `debug` -> Input,
/// `info`/unknown -> Output.
fn kind_for_severity(severity: &str) -> ConsoleEntryKind {
    match severity {
        "error" | "warn" => ConsoleEntryKind::Error,
        "debug" => ConsoleEntryKind::Input,
        _ => ConsoleEntryKind::Output,
    }
}

/// The one-line text rendered for a console row: `[category] subject — detail`,
/// with the trace id appended when present.
fn row_text(entry: &ConsoleStreamEntry) -> String {
    let mut text = format!("[{}] {} — {}", entry.category, entry.subject, entry.detail);
    if let Some(trace_id) = entry.trace_id.as_deref().filter(|t| !t.is_empty()) {
        text.push_str(&format!(" (trace {trace_id})"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(seq: u64, severity: &str, subject: &str) -> ConsoleStreamEntry {
        ConsoleStreamEntry {
            seq,
            ts_unix_ms: 0,
            severity: severity.to_string(),
            category: "model_lane_status".to_string(),
            subject: subject.to_string(),
            detail: "detail".to_string(),
            trace_id: None,
        }
    }

    #[test]
    fn severity_maps_to_console_kind() {
        assert_eq!(kind_for_severity("error"), ConsoleEntryKind::Error);
        assert_eq!(kind_for_severity("warn"), ConsoleEntryKind::Error);
        assert_eq!(kind_for_severity("debug"), ConsoleEntryKind::Input);
        assert_eq!(kind_for_severity("info"), ConsoleEntryKind::Output);
        assert_eq!(kind_for_severity("unknown"), ConsoleEntryKind::Output);
    }

    #[test]
    fn row_text_includes_category_subject_detail_and_trace() {
        let mut e = entry(1, "info", "lane-1");
        e.trace_id = Some("trace-9".to_string());
        let text = row_text(&e);
        assert!(text.contains("model_lane_status"));
        assert!(text.contains("lane-1"));
        assert!(text.contains("detail"));
        assert!(text.contains("trace trace-9"));
    }
}
