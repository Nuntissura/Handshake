//! WP-KERNEL-012 MT-028 — native LoomSearchV2 surface (E4 Search).
//!
//! The native Rust/egui counterpart to the React `LoomSearchV2Panel`
//! (`app/src/components/LoomSearchV2Panel.tsx`). It is a UI-only port: the hybrid-search backend
//! (`POST /workspaces/:ws/loom/search-v2`, KERNEL-009 MT-264) and the save-as-view route
//! (`POST /workspaces/:ws/loom/views/definitions`, the MT-027 VERIFIED createBlockView) are reused
//! THROUGH [`crate::backend_client::LoomSearchV2Client`]; this module never re-implements transport.
//!
//! ## What it renders (React parity)
//!
//! - QUERY BAR: a single-line `TextEdit` bound to [`LoomSearchV2PanelState::query`]; Enter or the
//!   `Search` button fires the search (off the UI thread).
//! - STATUS LINE: `Searching…` while a request is in flight, then `{N} results (semantic on)` /
//!   `{N} results (keyword/fuzzy only)` per `semantic_available`, or the error string.
//! - CONTENT-TYPE FACETS: one toggle per `content_type_facets` entry, sorted by count desc; clicking
//!   a facet re-runs the search filtered to that type; clicking the active facet clears it.
//! - RESULTS LIST: a `ScrollArea` of clickable rows; each shows the block title (or block_id
//!   fallback), a content_type badge, the score to 3 decimals, and the `<mark>`-highlighted excerpt
//!   rendered as COLORED runs (a `LayoutJob`, never raw HTML tags). Clicking a row invokes the
//!   `on_open_block` callback with the block id (open-in-place, a REFERENCE — never a copy).
//! - SAVE AS VIEW: a button enabled only when there are results; it POSTs the createBlockView body and
//!   shows the returned view block id (or the error) in the view-status label.
//!
//! ## Async / HBR-QUIET
//!
//! egui is single-threaded. The search + save-view HTTP calls run on the app's tokio runtime (via the
//! client's `spawn`) and deliver into `Arc<Mutex<Option<..>>>` cells the panel drains each frame
//! ([`LoomSearchV2PanelState::poll`]). The `Searching…` spinner-state animates ONLY while a request is
//! genuinely in flight (a `loading` flag the delivery clears) — idle/headless is neutral, never a
//! perpetual spinner (the MT-015 idle-repaint lesson). A repaint is requested while loading so the
//! delivered result is drained promptly without busy-looping when idle.
//!
//! ## AccessKit (HBR-SWARM / HBR-VIS)
//!
//! Every primary search widget carries a stable `author_id` under the canonical `search.`
//! namespace, attached through [`crate::accessibility::emit_interactive_node`] (the SAME hook the
//! shell chrome + toolbar use), so an out-of-process swarm agent drives the panel by id.

use std::sync::{Arc, Mutex};

use crate::accessibility;
use crate::backend_client::{
    LoomSearchCell, LoomSearchV2Body, LoomSearchV2Client, LoomSearchV2Response, SaveViewCell,
};
use crate::pane_registry::{PaneFactory, PaneId, PaneRenderContext, PaneType};
use crate::theme::HsPalette;

// ── Stable AccessKit author_ids (the MT-028 naming contract) ────────────────────────────────────────

/// The query `TextEdit`.
pub const QUERY_AUTHOR_ID: &str = "search.query";
/// The `Search` button.
pub const SEARCH_AUTHOR_ID: &str = "search.run";
/// The `Save as view` button.
pub const SAVE_VIEW_AUTHOR_ID: &str = "loom-search-v2.save-view";
/// The status line label.
pub const STATUS_AUTHOR_ID: &str = "loom-search-v2.status";
/// Prefix for a per-content_type facet toggle (`loom-search-v2.facet.{content_type}`).
pub const FACET_AUTHOR_ID_PREFIX: &str = "loom-search-v2.facet.";
/// Prefix for a per-result row (`search.result.{block_id}`).
pub const RESULT_AUTHOR_ID_PREFIX: &str = "search.result.";

/// The facet toggle author_id for a `content_type` (e.g. `loom-search-v2.facet.note`).
pub fn facet_author_id(content_type: &str) -> String {
    format!("{FACET_AUTHOR_ID_PREFIX}{content_type}")
}

/// The result-row author_id for a `block_id` (e.g. `search.result.blk-1`).
pub fn result_author_id(block_id: &str) -> String {
    format!("{RESULT_AUTHOR_ID_PREFIX}{block_id}")
}

/// Scope one canonical search author id to a secondary pane instance. The complete pane-id UTF-8
/// bytes are hex encoded, making the suffix injective and stable without relying on a short hash.
pub fn pane_scoped_author_id(author_id: &str, pane_id: Option<&str>) -> String {
    let Some(pane_id) = pane_id else {
        return author_id.to_owned();
    };
    use std::fmt::Write as _;
    let mut encoded = String::with_capacity(pane_id.len() * 2);
    for byte in pane_id.as_bytes() {
        let _ = write!(encoded, "{byte:02x}");
    }
    format!("{author_id}--pane-{encoded}")
}

/// Scoped result-row author id for a secondary Loom Search pane.
pub fn pane_scoped_result_author_id(block_id: &str, pane_id: Option<&str>) -> String {
    pane_scoped_author_id(&result_author_id(block_id), pane_id)
}

// ── Highlight parsing (RISK-1 / MC-1) ───────────────────────────────────────────────────────────────

/// One run of the ts_headline highlight: a text slice and whether it sat inside a `<mark>…</mark>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSegment {
    pub text: String,
    pub marked: bool,
}

/// Parse a ts_headline highlight string into alternating normal / marked segments by splitting on the
/// LITERAL `<mark>` and `</mark>` tokens (NOT an HTML parser — the backend only ever emits these two
/// markers). Empty slices are dropped, so `<mark>foo</mark> bar <mark>baz</mark>` yields exactly three
/// segments — `foo` (marked), ` bar ` (normal), `baz` (marked) — the MC-1 contract case. A string with
/// no markers yields a single normal segment; an empty string yields no segments.
///
/// Rendering this as a [`egui::text::LayoutJob`] (marked runs get a background fill) is what makes the
/// highlight show as COLORED text rather than literal `<mark>` tags (RISK-1: the React reference splits
/// on the same tokens; we never `dangerouslySetInnerHTML`).
pub fn parse_highlight_segments(highlight: &str) -> Vec<HighlightSegment> {
    let mut segments = Vec::new();
    let mut rest = highlight;
    let mut marked = false;
    // Walk the string, switching the `marked` flag at each literal marker and pushing the text
    // between markers. Using `find` on the two fixed tokens keeps this a pure string scan.
    while !rest.is_empty() {
        let open = rest.find("<mark>");
        let close = rest.find("</mark>");
        // The next marker is whichever of open/close appears first.
        let next = match (open, close) {
            (Some(o), Some(c)) => Some((o.min(c), if o < c { 6 } else { 7 }, o < c)),
            (Some(o), None) => Some((o, 6, true)),
            (None, Some(c)) => Some((c, 7, false)),
            (None, None) => None,
        };
        match next {
            Some((pos, token_len, is_open)) => {
                if pos > 0 {
                    segments.push(HighlightSegment {
                        text: rest[..pos].to_owned(),
                        marked,
                    });
                }
                // An `<mark>` turns marking on; a `</mark>` turns it off. A malformed/duplicated marker
                // just re-sets the same state (never panics — CONTROL-3 graceful degradation).
                marked = is_open;
                rest = &rest[pos + token_len..];
            }
            None => {
                segments.push(HighlightSegment {
                    text: rest.to_owned(),
                    marked,
                });
                break;
            }
        }
    }
    segments
}

/// Build a [`egui::text::LayoutJob`] from a highlight string: marked runs get the palette's
/// `search_highlight_bg` background (the theme token — NO `Color32` literal here, CONTROL-4), normal
/// runs the default text color. Used both by the row renderer and (re-exported for) the test surface.
pub fn highlight_layout_job(
    highlight: &str,
    palette: &HsPalette,
    text_color: egui::Color32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for seg in parse_highlight_segments(highlight) {
        let format = if seg.marked {
            egui::TextFormat {
                color: text_color,
                background: palette.search_highlight_bg,
                ..Default::default()
            }
        } else {
            egui::TextFormat {
                color: text_color,
                ..Default::default()
            }
        };
        job.append(&seg.text, 0.0, format);
    }
    job
}

// ── Facet ordering ──────────────────────────────────────────────────────────────────────────────────

/// The facet entries sorted by count DESCENDING, then by content_type ASCENDING for stable ties
/// (the React reference sorts `[content_type, count]` by `b[1] - a[1]`; the secondary key makes the
/// order deterministic across equal counts so the AccessKit tree + screenshot are reproducible).
pub fn sorted_facets(response: &LoomSearchV2Response) -> Vec<(String, i64)> {
    let mut entries: Vec<(String, i64)> = response
        .content_type_facets
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    entries
}

// ── Panel state machine ─────────────────────────────────────────────────────────────────────────────

/// All LoomSearchV2 panel state (the React component's `useState` hooks, as one struct). The delivery
/// cells are the off-thread async bridge: the search/save spawn writes into them and the panel drains
/// them each frame in [`poll`](Self::poll).
pub struct LoomSearchV2PanelState {
    /// The current query text (bound to the query `TextEdit`).
    pub query: String,
    /// The active content_type facet filter, or `None`. Reset to `None` whenever the query text
    /// changes (MC-4: a stale facet must never silently narrow a fresh query).
    pub active_content_type: Option<String>,
    /// The latest search response, or `None` before the first search / after an error.
    pub response: Option<LoomSearchV2Response>,
    /// `true` while a search request is genuinely in flight (drives the `Searching…` status ONLY while
    /// pending — never a perpetual spinner).
    pub loading: bool,
    /// The last search error string, or `None`.
    pub error: Option<String>,
    /// The save-as-view status string (the new view block id on success, or the error), or `None`.
    pub view_status: Option<String>,
    /// The query text the LAST in-flight search was fired with — used to detect a query edit so every
    /// query-derived response, pending delivery, error, save receipt, and facet can be invalidated.
    last_searched_query: Option<String>,
    /// Workspace identity currently owning every visible result and in-flight delivery. A workspace
    /// switch replaces both delivery cells, so a late completion from the prior workspace can only
    /// write into an orphaned cell that this panel will never drain.
    bound_workspace_id: Option<String>,
    /// Off-thread search delivery cell.
    search_cell: LoomSearchCell,
    /// Off-thread save-view delivery cell.
    save_cell: SaveViewCell,
    /// Stable id for the current save intent. It survives an ambiguous
    /// response/error and is retired only on success or a query/workspace
    /// identity change.
    save_attempt_id: Option<String>,
}

impl Default for LoomSearchV2PanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl LoomSearchV2PanelState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            active_content_type: None,
            response: None,
            loading: false,
            error: None,
            view_status: None,
            last_searched_query: None,
            bound_workspace_id: None,
            search_cell: Arc::new(Mutex::new(None)),
            save_cell: Arc::new(Mutex::new(None)),
            save_attempt_id: None,
        }
    }

    /// Bind the panel to the shell's current workspace. Returns `true` when the identity changed.
    /// Visible results, filters, errors, save receipts, and pending deliveries never cross this
    /// boundary. Replacing the cells also rejects late completions from the old workspace without
    /// cancellation races or sequence comparisons on the UI thread.
    pub fn bind_workspace(&mut self, workspace_id: Option<&str>) -> bool {
        if self.bound_workspace_id.as_deref() == workspace_id {
            return false;
        }
        self.bound_workspace_id = workspace_id.map(ToOwned::to_owned);
        self.active_content_type = None;
        self.response = None;
        self.loading = false;
        self.error = None;
        self.view_status = None;
        self.last_searched_query = None;
        self.search_cell = Arc::new(Mutex::new(None));
        self.save_cell = Arc::new(Mutex::new(None));
        self.save_attempt_id = None;
        true
    }

    /// `true` when there is at least one hit (gates the `Save as view` button: disabled with no
    /// results, enabled with results — AC-7).
    pub fn has_results(&self) -> bool {
        self.response.as_ref().is_some_and(|r| !r.hits.is_empty())
    }

    /// The honest status text for the current state. `Searching…` while pending; else the error; else
    /// the `{N} results (semantic …)` summary; else the neutral idle hint. The semantic suffix reads
    /// `semantic_available` so the operator is never told `(semantic on)` in keyword-only mode (RISK-7).
    pub fn status_text(&self) -> String {
        if self.loading {
            return "Searching…".to_owned();
        }
        if let Some(err) = &self.error {
            return err.clone();
        }
        if let Some(resp) = &self.response {
            let plural = if resp.total == 1 { "" } else { "s" };
            let modality = if resp.semantic_available {
                "semantic on"
            } else {
                "keyword/fuzzy only"
            };
            return format!("{} result{} ({})", resp.total, plural, modality);
        }
        "Enter a query".to_owned()
    }

    /// Drain the off-thread delivery cells, updating state for any arrived result. Called once at the
    /// top of [`show`]; returns `true` if anything was delivered (so the caller may request a repaint).
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Ok(mut slot) = self.search_cell.lock() {
            if let Some(result) = slot.take() {
                self.loading = false;
                match result {
                    Ok(resp) => {
                        self.response = Some(resp);
                        self.error = None;
                    }
                    Err(msg) => {
                        self.response = None;
                        self.error = Some(msg);
                    }
                }
                changed = true;
            }
        }
        if let Ok(mut slot) = self.save_cell.lock() {
            if let Some(result) = slot.take() {
                self.view_status = Some(match result {
                    Ok(block_id) => {
                        self.save_attempt_id = None;
                        format!("Saved search as Loom view {block_id}")
                    }
                    Err(msg) => format!("Save view failed: {msg}"),
                });
                changed = true;
            }
        }
        changed
    }

    /// Fire a search against `workspace_id` using the current query + active facet. Guards the
    /// no-workspace case (MC-7: show an error, NO HTTP call) and the empty-query case (the backend
    /// requires a non-empty query). On a real fire, sets `loading` and clears the previous error.
    pub fn run_search(&mut self, client: &LoomSearchV2Client, workspace_id: Option<&str>) {
        self.bind_workspace(workspace_id);
        let Some(ws) = workspace_id else {
            self.error = Some("No workspace selected".to_owned());
            return;
        };
        let trimmed = self.query.trim();
        if trimmed.is_empty() {
            self.error = Some("Search query is required".to_owned());
            return;
        }
        let body = LoomSearchV2Body::baseline(trimmed.to_owned(), self.active_content_type.clone());
        self.loading = true;
        self.error = None;
        self.view_status = None;
        self.last_searched_query = Some(self.query.clone());
        // Give every request its own delivery cell. A superseded request can finish, but it writes to
        // the old cell and therefore cannot replace the newer query/facet result. A search dispatch is
        // also a new save identity: its query/facet may differ from the view an earlier in-flight save
        // captured, so orphan that receipt instead of attributing it to the current result set.
        self.search_cell = Arc::new(Mutex::new(None));
        self.save_cell = Arc::new(Mutex::new(None));
        self.save_attempt_id = None;
        client.search(ws, &body, Arc::clone(&self.search_cell));
    }

    /// Toggle a content_type facet and immediately re-run the search. Clicking the active facet again
    /// CLEARS it (back to unfiltered); clicking a different facet switches to it (AC-4).
    pub fn toggle_facet(
        &mut self,
        content_type: &str,
        client: &LoomSearchV2Client,
        workspace_id: Option<&str>,
    ) {
        if self.active_content_type.as_deref() == Some(content_type) {
            self.active_content_type = None;
        } else {
            self.active_content_type = Some(content_type.to_owned());
        }
        self.run_search(client, workspace_id);
    }

    /// Save the current results as a Loom view (createBlockView). No-op when there are no results
    /// (the button is also disabled in that state) or no workspace.
    pub fn save_as_view(&mut self, client: &LoomSearchV2Client, workspace_id: Option<&str>) {
        self.bind_workspace(workspace_id);
        let Some(ws) = workspace_id else {
            self.view_status = Some("Save view failed: No workspace selected".to_owned());
            return;
        };
        if !self.has_results() {
            return;
        }
        self.view_status = None;
        let block_id = self
            .save_attempt_id
            .get_or_insert_with(|| uuid::Uuid::new_v4().to_string())
            .clone();
        // As with search, a superseded save owns an orphaned cell and cannot publish a stale receipt.
        self.save_cell = Arc::new(Mutex::new(None));
        client.save_view(
            ws,
            &block_id,
            self.query.trim(),
            self.active_content_type.as_deref(),
            Arc::clone(&self.save_cell),
        );
    }

    /// React to a query-text edit. Every search response, error, and save receipt is derived from the
    /// exact query that produced it, so editing query A into query B invalidates all of that state
    /// immediately. Replacing both delivery cells is the cancellation boundary: already-running HTTP
    /// work may finish off-thread, but it can only publish into an orphaned A cell that this panel will
    /// never drain. This also clears `loading` and the visible response synchronously, so B can neither
    /// display nor save A's results while the operator is still typing.
    ///
    /// The active facet is cleared at the same boundary (MC-4 — a stale facet must not silently narrow
    /// a fresh query). Called only when the `TextEdit` reports an actual change.
    fn on_query_edited(&mut self) {
        if self.last_searched_query.as_deref() != Some(self.query.as_str()) {
            self.active_content_type = None;
            self.response = None;
            self.loading = false;
            self.error = None;
            self.view_status = None;
            self.last_searched_query = None;
            self.search_cell = Arc::new(Mutex::new(None));
            self.save_cell = Arc::new(Mutex::new(None));
            self.save_attempt_id = None;
        }
    }
}

/// Callbacks the host wires into the panel. Kept as a struct (not bare closures in the signature) so a
/// later MT can add callbacks without breaking every call site.
pub struct LoomSearchV2Callbacks<'a> {
    /// Open the REAL block in place (a REFERENCE, not a copy) — routed to the Loom block viewer by the
    /// shell (the full open-target dispatch is MT-030; this MT covers the loom_block case).
    pub on_open_block: &'a mut dyn FnMut(&str),
}

/// Render the panel. Drains the async cells, draws the query bar / status / facets / results, and
/// dispatches search/save/facet/open actions through `client` + `callbacks`. `workspace_id` is the
/// active workspace (the no-workspace guard shows an error rather than 404ing — MC-7). Returns nothing;
/// all effects flow through the state machine + callbacks.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut LoomSearchV2PanelState,
    palette: &HsPalette,
    client: &LoomSearchV2Client,
    workspace_id: Option<&str>,
    callbacks: &mut LoomSearchV2Callbacks<'_>,
) {
    show_with_author_scope(ui, state, palette, client, workspace_id, callbacks, None);
}

/// Render with an optional secondary-pane author-id scope. `None` is the primary pane and preserves
/// canonical `search.*` ids; a secondary pane appends its injectively encoded stable pane id.
#[allow(clippy::too_many_arguments)]
fn show_with_author_scope(
    ui: &mut egui::Ui,
    state: &mut LoomSearchV2PanelState,
    palette: &HsPalette,
    client: &LoomSearchV2Client,
    workspace_id: Option<&str>,
    callbacks: &mut LoomSearchV2Callbacks<'_>,
    secondary_pane_id: Option<&str>,
) {
    state.bind_workspace(workspace_id);
    // Drain any delivered async result first, then keep repainting while a request is in flight so the
    // result is drained promptly (and the `Searching…` state is live) WITHOUT busy-looping when idle.
    state.poll();
    if state.loading {
        ui.ctx().request_repaint();
    }

    ui.heading("Loom Search");
    ui.label(egui::RichText::new("Hybrid: full-text + fuzzy + semantic").weak());
    ui.add_space(4.0);

    // ── Query bar: TextEdit + Search + Save-as-view ──
    let mut fire_search = false;
    let mut fire_save = false;
    let query_author_id = pane_scoped_author_id(QUERY_AUTHOR_ID, secondary_pane_id);
    let search_author_id = pane_scoped_author_id(SEARCH_AUTHOR_ID, secondary_pane_id);
    let save_view_author_id = pane_scoped_author_id(SAVE_VIEW_AUTHOR_ID, secondary_pane_id);
    let status_author_id = pane_scoped_author_id(STATUS_AUTHOR_ID, secondary_pane_id);
    ui.horizontal(|ui| {
        let edit = egui::TextEdit::singleline(&mut state.query)
            .hint_text("Search the Loom")
            .desired_width(220.0);
        let resp = ui.add(edit);
        accessibility::emit_interactive_node(ui.ctx(), resp.id, &query_author_id);
        ui.ctx().accesskit_node_builder(resp.id, |node| {
            node.set_role(egui::accesskit::Role::TextInput);
            node.set_value(state.query.clone());
            node.add_action(egui::accesskit::Action::Focus);
            node.add_action(egui::accesskit::Action::SetValue);
            node.add_action(egui::accesskit::Action::ReplaceSelectedText);
        });
        let mut swarm_query: Option<String> = None;
        let mut swarm_append = Vec::new();
        ui.input(|input| {
            for request in
                input.accesskit_action_requests(resp.id, egui::accesskit::Action::SetValue)
            {
                if let Some(egui::accesskit::ActionData::Value(value)) = &request.data {
                    swarm_query = Some(value.to_string());
                }
            }
            for request in input
                .accesskit_action_requests(resp.id, egui::accesskit::Action::ReplaceSelectedText)
            {
                if let Some(egui::accesskit::ActionData::Value(value)) = &request.data {
                    swarm_append.push(value.to_string());
                }
            }
        });
        if let Some(query) = swarm_query {
            state.query = query;
            state.on_query_edited();
        }
        if !swarm_append.is_empty() {
            state.query.push_str(&swarm_append.concat());
            state.on_query_edited();
        }
        if resp.changed() {
            state.on_query_edited();
        }
        // Enter in the focused field fires the search (React parity: onKeyDown Enter).
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            fire_search = true;
        }

        let search_btn = ui.button("Search");
        accessibility::emit_interactive_node(ui.ctx(), search_btn.id, &search_author_id);
        if search_btn.clicked() {
            fire_search = true;
        }

        let save_btn = ui.add_enabled(state.has_results(), egui::Button::new("Save as view"));
        accessibility::emit_interactive_node(ui.ctx(), save_btn.id, &save_view_author_id);
        if save_btn.clicked() {
            fire_save = true;
        }
    });

    // ── Status line ──
    let status_resp = ui.label(state.status_text());
    accessibility::emit_interactive_node(ui.ctx(), status_resp.id, &status_author_id);

    // ── View-status line (save-as-view result) ──
    if let Some(view_status) = &state.view_status {
        ui.label(egui::RichText::new(view_status).weak());
    }

    // ── Facets ──
    let mut toggle_facet: Option<String> = None;
    if let Some(response) = &state.response {
        let facets = sorted_facets(response);
        if !facets.is_empty() {
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                for (content_type, count) in &facets {
                    let active =
                        state.active_content_type.as_deref() == Some(content_type.as_str());
                    let label = format!("{content_type} ({count})");
                    let btn = ui.add(egui::Button::new(label).selected(active));
                    accessibility::emit_interactive_node(
                        ui.ctx(),
                        btn.id,
                        &pane_scoped_author_id(&facet_author_id(content_type), secondary_pane_id),
                    );
                    if btn.clicked() {
                        toggle_facet = Some(content_type.clone());
                    }
                }
            });
        }
    }

    // ── Results ──
    let mut open_block: Option<String> = None;
    if let Some(response) = &state.response {
        let text_color = ui.visuals().text_color();
        let badge_color = ui.visuals().weak_text_color();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for hit in &response.hits {
                    let block_id = hit.block.block_id.clone();
                    let frame = egui::Frame::group(ui.style());
                    let inner = frame.show(ui, |ui| {
                        ui.vertical(|ui| {
                            // Title + content_type badge + score on one line.
                            ui.horizontal(|ui| {
                                ui.strong(hit.block.display_title());
                                ui.label(
                                    egui::RichText::new(format!("[{}]", hit.block.content_type))
                                        .color(badge_color)
                                        .small(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("score {:.3}", hit.score))
                                        .color(badge_color)
                                        .small(),
                                );
                            });
                            // Highlight: <mark> runs as colored LayoutJob (NOT raw HTML).
                            if !hit.highlight.is_empty() {
                                let job = highlight_layout_job(&hit.highlight, palette, text_color);
                                ui.label(job);
                            }
                        });
                    });
                    // Make the whole row clickable; attach the stable result author_id.
                    let row = inner.response.interact(egui::Sense::click());
                    accessibility::emit_interactive_node(
                        ui.ctx(),
                        row.id,
                        &pane_scoped_result_author_id(&block_id, secondary_pane_id),
                    );
                    if row.clicked() {
                        open_block = Some(block_id);
                    }
                }
            });
    }

    // ── Dispatch deferred actions (after the borrows of `state.response` end) ──
    if let Some(content_type) = toggle_facet {
        state.toggle_facet(&content_type, client, workspace_id);
    }
    if let Some(block_id) = open_block {
        (callbacks.on_open_block)(&block_id);
    }
    if fire_search {
        state.run_search(client, workspace_id);
    }
    if fire_save {
        state.save_as_view(client, workspace_id);
    }
}

// ── Pane factory (the in-product render path — AC-9) ──────────────────────────────────────────────────

/// Per-frame inputs the shell pushes to the [`LoomSearchV2PaneFactory`] before the pane host renders,
/// plus the open-block requests the factory pushes back. `PaneFactory::render` takes `&self`, so this
/// shared cell (behind a `Mutex`) is how the live app threads the active workspace id + theme palette
/// IN and drains result-row clicks OUT — without `&mut self` on the factory map. The shell updates
/// `workspace_id`/`palette` at the top of each frame and drains `open_requests` after the pane host runs
/// (routing each block id to the Loom block viewer via the same open path the bookmark rail uses).
pub struct LoomSearchV2PaneShared {
    /// The active workspace id, or `None` when no workspace is selected (the panel's no-workspace guard
    /// then shows an error and fires NO HTTP call — MC-7).
    pub workspace_id: Option<String>,
    /// The live theme palette, so the `<mark>` highlight reads the themed `search_highlight_bg` and the
    /// pane flips dark<->light with the rest of the shell.
    pub palette: HsPalette,
    /// Block ids the operator/agent clicked this frame, drained by the shell into the Loom block-open
    /// path (FIFO). The factory's `on_open_block` callback pushes here.
    pub open_requests: Vec<String>,
}

impl LoomSearchV2PaneShared {
    /// Seed with no workspace and the given palette (the shell overwrites both each frame).
    pub fn new(palette: HsPalette) -> Self {
        Self {
            workspace_id: None,
            palette,
            open_requests: Vec::new(),
        }
    }
}

/// The CONCRETE `PaneFactory` for [`PaneType::LoomSearchV2`] — the in-product render path that makes the
/// "Loom Search" pane render the REAL panel ([`show`]) instead of the centered placeholder (AC-9). The
/// shell inserts ONE of these into its factory map AFTER the placeholder-fill loop, overriding the
/// `PlaceholderPaneFactory` for this variant (the same override mechanism a concrete pane uses).
///
/// `PaneFactory::render` is `&self`, so the mutable panel state lives behind a `Mutex` (Send + Sync, as
/// the trait requires) and the per-frame workspace id + palette + open-block drain flow through the
/// shared [`LoomSearchV2PaneShared`] cell. The HTTP transport reuses the real
/// [`LoomSearchV2Client`] (the SAME verified routes the request-builder + live-PG proofs bind).
pub struct LoomSearchV2PaneFactory {
    state: Mutex<LoomSearchV2PanelState>,
    client: LoomSearchV2Client,
    shared: Arc<Mutex<LoomSearchV2PaneShared>>,
    /// First pane rendered by this factory retains the canonical ids. Every additional pane is
    /// deterministically scoped by its stable pane id, preventing full-tree author-id overlap.
    primary_pane_id: Mutex<Option<PaneId>>,
}

impl LoomSearchV2PaneFactory {
    /// Build the factory + return the shared cell the shell keeps a clone of (to push workspace id +
    /// palette in and drain open-block requests out each frame). `client` is the real
    /// [`LoomSearchV2Client::production`] bridged onto the app runtime.
    pub fn new(client: LoomSearchV2Client, shared: Arc<Mutex<LoomSearchV2PaneShared>>) -> Self {
        Self::with_state(client, shared, LoomSearchV2PanelState::new())
    }

    /// Like [`new`](Self::new) but seeds the panel state. Lets a proof open the pane THROUGH the registry
    /// with a pre-populated response (so the registry-dispatched render shows real facets/rows/highlight),
    /// rather than calling [`show`] out-of-band — the AC-9 in-product render path.
    pub fn with_state(
        client: LoomSearchV2Client,
        shared: Arc<Mutex<LoomSearchV2PaneShared>>,
        state: LoomSearchV2PanelState,
    ) -> Self {
        Self {
            state: Mutex::new(state),
            client,
            shared,
            primary_pane_id: Mutex::new(None),
        }
    }
}

impl PaneFactory for LoomSearchV2PaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::LoomSearchV2
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        // Read the per-frame inputs (workspace id + palette) under a short lock, so the long-lived
        // `show` borrow does not hold the shared mutex while the panel renders.
        let (workspace_id, palette) = {
            let guard = self.shared.lock().unwrap_or_else(|p| p.into_inner());
            (guard.workspace_id.clone(), guard.palette.clone())
        };
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let shared_for_open = Arc::clone(&self.shared);
        // The open-block callback pushes the clicked id into the shared cell; the shell drains it into
        // the Loom block-open path after the pane host runs (open-in-place, a REFERENCE — never a copy).
        let mut on_open = move |block_id: &str| {
            if let Ok(mut guard) = shared_for_open.lock() {
                guard.open_requests.push(block_id.to_owned());
            }
        };
        let mut callbacks = LoomSearchV2Callbacks {
            on_open_block: &mut on_open,
        };
        let secondary_pane_id = {
            let mut primary = self
                .primary_pane_id
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            match primary.as_ref() {
                Some(primary_id) if primary_id == &ctx.record.pane_id => None,
                Some(_) => Some(ctx.record.pane_id.as_ref()),
                None => {
                    *primary = Some(ctx.record.pane_id.clone());
                    None
                }
            }
        };
        show_with_author_scope(
            ui,
            &mut state,
            &palette,
            &self.client,
            workspace_id.as_deref(),
            &mut callbacks,
            secondary_pane_id,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::NodeT;
    use egui_kittest::Harness;
    use std::collections::BTreeMap;

    #[test]
    fn primary_live_search_ids_use_canonical_namespace() {
        assert_eq!(QUERY_AUTHOR_ID, "search.query");
        assert_eq!(SEARCH_AUTHOR_ID, "search.run");
        assert_eq!(result_author_id("BLOCK-9"), "search.result.BLOCK-9");
        assert_eq!(pane_scoped_author_id(QUERY_AUTHOR_ID, None), "search.query");
        assert_eq!(
            pane_scoped_author_id(QUERY_AUTHOR_ID, Some("pane-two")),
            "search.query--pane-70616e652d74776f"
        );
        assert_eq!(
            pane_scoped_result_author_id("BLOCK-9", Some("pane-two")),
            "search.result.BLOCK-9--pane-70616e652d74776f"
        );
    }

    #[test]
    fn two_search_panes_have_unique_full_tree_author_ids() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let client = LoomSearchV2Client::new("http://127.0.0.1:9", runtime.handle().clone());
        let palette = crate::theme::HsTheme::Dark.palette();
        let mut primary = LoomSearchV2PanelState::new();
        let mut secondary = LoomSearchV2PanelState::new();
        let response = LoomSearchV2Response {
            hits: vec![crate::backend_client::LoomSearchV2Hit {
                block: crate::backend_client::LoomSearchBlock {
                    block_id: "shared-block".to_owned(),
                    content_type: "note".to_owned(),
                    document_id: Some("doc-shared".to_owned()),
                    title: Some("Shared result".to_owned()),
                },
                score: 1.0,
                fts_rank: 1.0,
                trgm_sim: 0.0,
                vector_sim: 0.0,
                edge_degree: 0,
                highlight: "<mark>shared</mark> result".to_owned(),
            }],
            content_type_facets: BTreeMap::from([("note".to_owned(), 1)]),
            semantic_available: true,
            total: 1,
        };
        primary.response = Some(response.clone());
        secondary.response = Some(response);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(900.0, 700.0))
            .build_ui(move |ui| {
                let _keep_runtime_alive = &runtime;
                ui.push_id("primary-search-pane", |ui| {
                    let mut open = |_block_id: &str| {};
                    let mut callbacks = LoomSearchV2Callbacks {
                        on_open_block: &mut open,
                    };
                    show_with_author_scope(
                        ui,
                        &mut primary,
                        &palette,
                        &client,
                        None,
                        &mut callbacks,
                        None,
                    );
                });
                ui.push_id("secondary-search-pane", |ui| {
                    let mut open = |_block_id: &str| {};
                    let mut callbacks = LoomSearchV2Callbacks {
                        on_open_block: &mut open,
                    };
                    show_with_author_scope(
                        ui,
                        &mut secondary,
                        &palette,
                        &client,
                        None,
                        &mut callbacks,
                        Some("pane-two"),
                    );
                });
            });
        harness.run();
        harness.run();

        let authors = harness
            .root()
            .children_recursive()
            .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
            .collect::<Vec<_>>();
        let unique = authors.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(
            unique.len(),
            authors.len(),
            "multiple Loom Search panes must have a globally unique author-id tree: {authors:?}"
        );
        assert!(authors.iter().any(|author| author == QUERY_AUTHOR_ID));
        assert!(authors
            .iter()
            .any(|author| { author == &pane_scoped_author_id(QUERY_AUTHOR_ID, Some("pane-two")) }));
        assert!(authors
            .iter()
            .any(|author| author == &facet_author_id("note")));
        assert!(authors.iter().any(|author| {
            author == &pane_scoped_author_id(&facet_author_id("note"), Some("pane-two"))
        }));
        assert!(authors
            .iter()
            .any(|author| author == &result_author_id("shared-block")));
        assert!(authors.iter().any(|author| {
            author == &pane_scoped_result_author_id("shared-block", Some("pane-two"))
        }));
    }

    fn response_with(facets: &[(&str, i64)], semantic: bool, total: i64) -> LoomSearchV2Response {
        let mut map = BTreeMap::new();
        for (k, v) in facets {
            map.insert((*k).to_owned(), *v);
        }
        LoomSearchV2Response {
            hits: vec![],
            content_type_facets: map,
            semantic_available: semantic,
            total,
        }
    }

    #[test]
    fn highlight_three_segments_mid_and_last_marked() {
        // The MC-1 contract case.
        let segs = parse_highlight_segments("<mark>foo</mark> bar <mark>baz</mark>");
        assert_eq!(
            segs,
            vec![
                HighlightSegment {
                    text: "foo".to_owned(),
                    marked: true
                },
                HighlightSegment {
                    text: " bar ".to_owned(),
                    marked: false
                },
                HighlightSegment {
                    text: "baz".to_owned(),
                    marked: true
                },
            ]
        );
    }

    #[test]
    fn highlight_no_markers_is_one_normal_segment() {
        let segs = parse_highlight_segments("plain text");
        assert_eq!(
            segs,
            vec![HighlightSegment {
                text: "plain text".to_owned(),
                marked: false
            }]
        );
    }

    #[test]
    fn highlight_empty_is_no_segments() {
        assert!(parse_highlight_segments("").is_empty());
    }

    #[test]
    fn highlight_leading_mark() {
        let segs = parse_highlight_segments("<mark>hit</mark> rest");
        assert_eq!(
            segs,
            vec![
                HighlightSegment {
                    text: "hit".to_owned(),
                    marked: true
                },
                HighlightSegment {
                    text: " rest".to_owned(),
                    marked: false
                },
            ]
        );
    }

    #[test]
    fn facets_sort_by_count_desc_then_name_asc() {
        let resp = response_with(&[("note", 2), ("code", 5), ("image", 2)], true, 9);
        let sorted = sorted_facets(&resp);
        assert_eq!(
            sorted,
            vec![
                ("code".to_owned(), 5),
                ("image".to_owned(), 2),
                ("note".to_owned(), 2)
            ]
        );
    }

    #[test]
    fn status_semantic_on_vs_keyword_only() {
        let mut state = LoomSearchV2PanelState::new();
        state.response = Some(response_with(&[("note", 1)], true, 3));
        assert_eq!(state.status_text(), "3 results (semantic on)");
        state.response = Some(response_with(&[("note", 1)], false, 1));
        assert_eq!(state.status_text(), "1 result (keyword/fuzzy only)");
    }

    #[test]
    fn status_loading_takes_precedence() {
        let mut state = LoomSearchV2PanelState::new();
        state.loading = true;
        state.response = Some(response_with(&[], true, 5));
        assert_eq!(state.status_text(), "Searching…");
    }

    #[test]
    fn status_error_when_set_and_idle() {
        let mut state = LoomSearchV2PanelState::new();
        state.error = Some("No workspace selected".to_owned());
        assert_eq!(state.status_text(), "No workspace selected");
    }

    #[test]
    fn query_edit_clears_active_facet() {
        let mut state = LoomSearchV2PanelState::new();
        state.last_searched_query = Some("cats".to_owned());
        state.active_content_type = Some("note".to_owned());
        state.response = Some(response_with(&[("note", 1)], false, 1));
        state.error = Some("stale search error".to_owned());
        state.view_status = Some("Saved search as Loom view stale-a".to_owned());
        state.query = "cats and dogs".to_owned(); // changed
        state.on_query_edited();
        assert!(state.active_content_type.is_none());
        assert!(state.response.is_none(), "query B cannot display query A");
        assert!(
            !state.has_results(),
            "query B cannot save query A's results"
        );
        assert!(state.error.is_none(), "query A's error is stale under B");
        assert!(
            state.view_status.is_none(),
            "query A's save receipt is stale under B"
        );
    }

    #[test]
    fn query_unchanged_keeps_facet() {
        let mut state = LoomSearchV2PanelState::new();
        state.last_searched_query = Some("cats".to_owned());
        state.active_content_type = Some("note".to_owned());
        state.query = "cats".to_owned(); // unchanged
        state.on_query_edited();
        assert_eq!(state.active_content_type, Some("note".to_owned()));
    }

    #[test]
    fn has_results_gates_save_button() {
        let mut state = LoomSearchV2PanelState::new();
        assert!(!state.has_results(), "no response => no results");
        state.response = Some(response_with(&[], true, 0)); // empty hits
        assert!(!state.has_results(), "empty hits => no results");
    }

    #[test]
    fn empty_query_is_rejected_without_starting_a_request() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let client = LoomSearchV2Client::new("http://127.0.0.1:9", runtime.handle().clone());
        let mut state = LoomSearchV2PanelState::new();
        state.query = "  \t".to_owned();

        state.run_search(&client, Some("workspace-a"));

        assert_eq!(state.error.as_deref(), Some("Search query is required"));
        assert!(!state.loading);
        assert!(state.response.is_none());
    }

    #[test]
    fn workspace_rebind_clears_state_and_rejects_stale_delivery() {
        let mut state = LoomSearchV2PanelState::new();
        state.bind_workspace(Some("workspace-a"));
        let stale_cell = Arc::clone(&state.search_cell);
        state.active_content_type = Some("note".to_owned());
        state.response = Some(response_with(&[("note", 1)], false, 1));
        state.loading = true;
        state.error = Some("old error".to_owned());
        state.view_status = Some("old save receipt".to_owned());

        assert!(state.bind_workspace(Some("workspace-b")));
        *stale_cell.lock().expect("stale delivery cell") =
            Some(Ok(response_with(&[("file", 9)], true, 9)));

        assert!(
            !state.poll(),
            "the orphaned workspace-a cell is never drained"
        );
        assert_eq!(state.bound_workspace_id.as_deref(), Some("workspace-b"));
        assert!(state.active_content_type.is_none());
        assert!(state.response.is_none());
        assert!(!state.loading);
        assert!(state.error.is_none());
        assert!(state.view_status.is_none());
    }

    #[test]
    fn superseded_search_delivery_cannot_replace_the_newer_result() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let client = LoomSearchV2Client::new("http://127.0.0.1:9", runtime.handle().clone());
        let mut state = LoomSearchV2PanelState::new();
        state.query = "first".to_owned();
        state.run_search(&client, Some("workspace-a"));
        let superseded_cell = Arc::clone(&state.search_cell);

        state.query = "second".to_owned();
        state.run_search(&client, Some("workspace-a"));
        let current_cell = Arc::clone(&state.search_cell);
        assert!(
            !Arc::ptr_eq(&superseded_cell, &current_cell),
            "each query owns a distinct delivery cell"
        );

        // Deliver the CURRENT request first and apply it.
        *current_cell.lock().expect("current cell") =
            Some(Ok(response_with(&[("file", 2)], false, 2)));
        assert!(state.poll());
        let response = state.response.as_ref().expect("current response delivered");
        assert_eq!(response.total, 2);
        assert_eq!(response.content_type_facets.get("file"), Some(&2));
        assert!(!response.content_type_facets.contains_key("note"));

        // Now deliver the stale request LAST. Polling the panel must not even observe that orphaned
        // cell, and the already-displayed current result must remain byte-for-byte equivalent.
        *superseded_cell.lock().expect("superseded cell") =
            Some(Ok(response_with(&[("note", 1)], false, 1)));
        assert!(!state.poll(), "late stale delivery is not consumed");
        let response = state.response.as_ref().expect("current result retained");
        assert_eq!(response.total, 2);
        assert_eq!(response.content_type_facets.get("file"), Some(&2));
        assert!(!response.content_type_facets.contains_key("note"));
    }

    #[test]
    fn query_edit_during_flight_orphans_search_and_save_deliveries() {
        let mut state = LoomSearchV2PanelState::new();
        state.bind_workspace(Some("workspace-a"));
        state.query = "query-a".to_owned();
        state.last_searched_query = Some("query-a".to_owned());
        state.loading = true;
        state.active_content_type = Some("note".to_owned());
        state.response = Some(response_with(&[("note", 1)], false, 1));
        let stale_search_cell = Arc::clone(&state.search_cell);
        let stale_save_cell = Arc::clone(&state.save_cell);

        state.query = "query-b".to_owned();
        state.on_query_edited();

        assert!(
            !state.loading,
            "editing cancels A's visible in-flight state"
        );
        assert!(state.response.is_none(), "A disappears immediately under B");
        assert!(
            !state.has_results(),
            "save is immediately ineligible under B"
        );
        assert!(state.active_content_type.is_none());
        assert!(state.error.is_none());
        assert!(state.view_status.is_none());
        assert!(!Arc::ptr_eq(&stale_search_cell, &state.search_cell));
        assert!(!Arc::ptr_eq(&stale_save_cell, &state.save_cell));

        *stale_search_cell.lock().expect("stale search cell") =
            Some(Ok(response_with(&[("note", 9)], true, 9)));
        *stale_save_cell.lock().expect("stale save cell") = Some(Ok("view-for-query-a".to_owned()));
        assert!(
            !state.poll(),
            "A's late search/save deliveries stay orphaned"
        );
        assert!(state.response.is_none());
        assert!(state.view_status.is_none());
    }

    #[test]
    fn facet_transition_keeps_current_result_and_orphans_stale_save_delivered_last() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let client = LoomSearchV2Client::new("http://127.0.0.1:9", runtime.handle().clone());
        let mut state = LoomSearchV2PanelState::new();
        state.bind_workspace(Some("workspace-a"));
        state.query = "shared-query".to_owned();
        state.last_searched_query = Some(state.query.clone());
        state.active_content_type = Some("note".to_owned());
        state.response = Some(response_with(&[("note", 1), ("file", 1)], false, 2));

        state.save_as_view(&client, Some("workspace-a"));
        let stale_note_save_cell = Arc::clone(&state.save_cell);

        // Switch to a different logical search identity. The current file result is delivered and
        // applied FIRST; the prior note-view save receipt is deliberately delivered LAST.
        state.toggle_facet("file", &client, Some("workspace-a"));
        let current_file_search_cell = Arc::clone(&state.search_cell);
        assert_eq!(state.active_content_type.as_deref(), Some("file"));
        assert!(
            !Arc::ptr_eq(&stale_note_save_cell, &state.save_cell),
            "a facet transition must orphan the prior facet's save delivery"
        );

        *current_file_search_cell
            .lock()
            .expect("current search cell") = Some(Ok(response_with(&[("file", 1)], false, 1)));
        assert!(state.poll(), "the current facet result is consumed first");
        assert_eq!(
            state.response.as_ref().map(|response| response.total),
            Some(1)
        );
        assert_eq!(
            state
                .response
                .as_ref()
                .and_then(|response| response.content_type_facets.get("file")),
            Some(&1)
        );

        *stale_note_save_cell.lock().expect("stale save cell") =
            Some(Ok("view-for-note-facet".to_owned()));
        assert!(
            !state.poll(),
            "the stale note-facet save delivered last is never consumed"
        );
        assert!(state.view_status.is_none());
        assert_eq!(state.active_content_type.as_deref(), Some("file"));
        assert_eq!(
            state.response.as_ref().map(|response| response.total),
            Some(1)
        );
    }

    #[test]
    fn ambiguous_save_retry_reuses_block_id_until_success() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let client = LoomSearchV2Client::new("http://127.0.0.1:9", runtime.handle().clone());
        let mut state = LoomSearchV2PanelState::new();
        state.bind_workspace(Some("workspace-a"));
        state.query = "stable retry".to_owned();
        state.response = Some(response_with(&[("note", 1)], false, 1));

        state.save_as_view(&client, Some("workspace-a"));
        let first_id = state
            .save_attempt_id
            .clone()
            .expect("save dispatch owns a stable id");
        *state.save_cell.lock().expect("save cell") = Some(Err("response lost".to_owned()));
        assert!(state.poll());
        assert_eq!(state.save_attempt_id.as_deref(), Some(first_id.as_str()));

        state.save_as_view(&client, Some("workspace-a"));
        assert_eq!(
            state.save_attempt_id.as_deref(),
            Some(first_id.as_str()),
            "ambiguous retry must not mint a second saved view id"
        );
        *state.save_cell.lock().expect("retry save cell") = Some(Ok(first_id.clone()));
        assert!(state.poll());
        assert!(
            state.save_attempt_id.is_none(),
            "successful authority response retires the retry identity"
        );
    }
}
