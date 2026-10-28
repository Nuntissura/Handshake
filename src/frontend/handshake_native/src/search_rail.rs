//! Persistent Handpick field with attached Notes and grouped graph/command results.
//! Shared Handpick owns request fences and picker state. This host owns authorization,
//! offset paging and the existing typed navigation bus; native Notes retains writing ownership.
//!
//! Public legacy controls retain NodeIds 22 (input), 23 (clear), 24 (Loom) and their
//! bottom-rail.* author IDs. The nine scope IDs remain reachable through the scope menu.
//! RailQuerySlot receives explicit Search/Enter/Loom intents and clears on Clear or context retirement.
//! Project/file graph providers run off-frame with the authenticated account, session/workspace
//! fences, cancellation and a post-body active check. Other providers report their missing binding.
//! Each page supplies loaded rows; the API supplies no canonical total or snapshot revision.
//! Errors retain the literal query; access denial discards result content. No writing/save is faked.
use crate::diagnostics::gui_events::{record_gui_observation, GuiDiagnosticOperation as GuiOp, GuiDiagnosticPhase as GuiPhase};
mod composition;

use std::fmt;
use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::accessibility::emit_interactive_node;

/// Fixed AccessKit/egui `NodeId` of the rail QUERY input (Role::TextInput). Fresh band slot 22.
pub const RAIL_INPUT_NODE_ID: u64 = 22;
/// Fixed AccessKit/egui `NodeId` of the rail CLEAR (`x`) button (Role::Button). Fresh band slot 23.
pub const RAIL_CLEAR_NODE_ID: u64 = 23;
/// Fixed AccessKit/egui `NodeId` of the rail LOOM shortcut button (Role::Button). Fresh band slot 24.
pub const RAIL_LOOM_NODE_ID: u64 = 24;

/// Stable out-of-process author_id for the rail query input.
pub const RAIL_INPUT_AUTHOR_ID: &str = "bottom-rail.input";
/// Stable out-of-process author_id for the rail clear button.
pub const RAIL_CLEAR_AUTHOR_ID: &str = "bottom-rail.clear";
/// Stable out-of-process author_id for the rail Loom shortcut button.
pub const RAIL_LOOM_AUTHOR_ID: &str = "bottom-rail.loom";

/// The author_id PREFIX for a scope pill: `bottom-rail.scope.{scope_name}` (Role::Button).
pub const SCOPE_AUTHOR_ID_PREFIX: &str = "bottom-rail.scope.";

/// Persistent header height in logical pixels.
pub const RAIL_HEIGHT: f32 = 44.0;

// ===========================================================================
// Scope enum (the nine fixed rail scopes).
// ===========================================================================

/// The nine fixed search scopes the rail's pill strip exposes (the contract's
/// `project: file: pane: window: stash: trace: terminal: stage: layout:`). The scope narrows WHAT a
/// query searches; the active scope is set by clicking a pill OR by typing its colon-prefix into the
/// input (`project:foo`). The scope is part of the emitted [`RailQuery`] intent â€” a downstream consumer
/// decides how to apply it when it executes the search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SearchScope {
    /// Whole-project Loom search (the default + the Loom shortcut).
    #[default]
    Project,
    /// File / annotated-file blocks.
    File,
    /// The active pane's content.
    Pane,
    /// The active window's surfaces.
    Window,
    /// The stash shelf (pinned/stashed blocks).
    Stash,
    /// Trace / flight-recorder entries.
    Trace,
    /// Terminal scrollback / sessions.
    Terminal,
    /// Stage entities.
    Stage,
    /// Saved layouts.
    Layout,
}

impl SearchScope {
    /// All nine scopes in the fixed left-to-right pill order.
    pub fn all() -> &'static [SearchScope] {
        &[
            SearchScope::Project,
            SearchScope::File,
            SearchScope::Pane,
            SearchScope::Window,
            SearchScope::Stash,
            SearchScope::Trace,
            SearchScope::Terminal,
            SearchScope::Stage,
            SearchScope::Layout,
        ]
    }

    /// The bare scope keyword (no colon): `"project"`, `"file"`, ... Used for the author_id suffix and
    /// the typed-prefix match.
    pub fn keyword(self) -> &'static str {
        match self {
            SearchScope::Project => "project",
            SearchScope::File => "file",
            SearchScope::Pane => "pane",
            SearchScope::Window => "window",
            SearchScope::Stash => "stash",
            SearchScope::Trace => "trace",
            SearchScope::Terminal => "terminal",
            SearchScope::Stage => "stage",
            SearchScope::Layout => "layout",
        }
    }

    /// Parse a bare scope keyword (no colon) back to a [`SearchScope`]. `None` for an unknown keyword.
    pub fn from_keyword(keyword: &str) -> Option<SearchScope> {
        SearchScope::all()
            .iter()
            .copied()
            .find(|s| s.keyword() == keyword)
    }

    /// The stable author_id for this scope's pill (`bottom-rail.scope.{keyword}`).
    pub fn author_id(self) -> String {
        format!("{SCOPE_AUTHOR_ID_PREFIX}{}", self.keyword())
    }

    /// The `content_type` facet this scope maps onto, if any (for the downstream consumer that will
    /// execute the search). A scope with no direct `content_type` (the cross-surface scopes â€”
    /// pane/window/trace/terminal/stage/layout, plus stash which has no distinct content_type) returns
    /// `None` and the consumer searches the whole graph with the typed query. `Project` is whole-project
    /// (no content_type filter). This is the rail's HONEST scope->facet mapping carried on the intent; it
    /// is NOT a backend request â€” the rail makes no backend call.
    pub fn content_type(self) -> Option<&'static str> {
        match self {
            // file / annotated_file blocks.
            SearchScope::File => Some("file"),
            // Project + stash + every cross-surface scope: no content_type facet (whole-graph query).
            _ => None,
        }
    }
}

impl fmt::Display for SearchScope {
    /// The colon-suffixed pill label (`"project:"`, `"file:"`, ...), per the contract.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:", self.keyword())
    }
}

// ===========================================================================
// Query parser (port of the React parseLoomSearchOperators).
// ===========================================================================

/// A parsed rail query: the residual free-text plus the resolved scope and the extracted operator
/// facets. Produced by [`parse_rail_query`]; this IS the search INTENT the rail emits and a downstream
/// pane/overlay consumes (the contract's `RailQuery`). The rail only PRODUCES this struct â€” it does not
/// execute a search.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RailQuery {
    /// The free-text remaining after every operator/scope prefix is stripped.
    pub free_text: String,
    /// The resolved scope: the typed colon-prefix wins over the pill selection (`project:foo` forces
    /// `Project` even if `File` was the active pill).
    pub scope: SearchScope,
    /// Tag-hub block ids from `tag:` / `mention:` operators (a downstream consumer sends as `tag_ids`).
    pub tag_ids: Vec<String>,
    /// Path filter from a `path:` / `folder:` operator (a downstream consumer decides how to apply it;
    /// the verified `search-v2` body has no dedicated path field, so the consumer folds it into the query
    /// â€” see [`effective_query`](Self::effective_query)).
    pub path: Option<String>,
    /// Content-kind filters from a `kind:` operator. A value that is a recognized `content_type`
    /// promotes to the resolved content_type (see [`resolved_content_type`](Self::resolved_content_type));
    /// an unrecognized value is captured in [`kind_errors`](Self::kind_errors) without panicking
    /// (PROOF-022-1e).
    pub kind_filters: Vec<String>,
    /// `kind:` values that did NOT match a known `content_type` (surfaced as a soft error, never a
    /// panic). Empty when every `kind:` value was valid.
    pub kind_errors: Vec<String>,
}

impl RailQuery {
    /// The `content_type` the emitted intent resolves to (for the downstream consumer): a valid `kind:`
    /// operator wins (most specific), else the active scope's mapping. `None` => whole-graph query. A
    /// pure helper over the parsed intent â€” no backend call.
    pub fn resolved_content_type(&self) -> Option<String> {
        // The first VALID kind: filter wins over the scope's mapping.
        for k in &self.kind_filters {
            if is_known_content_type(k) {
                return Some(k.clone());
            }
        }
        self.scope.content_type().map(str::to_owned)
    }

    /// The effective query string a downstream consumer would search with: the free-text plus any
    /// `path:` term folded in (the verified `search-v2` body has no path field, so the path narrows the
    /// FTS query honestly rather than a fabricated parameter). Empty path => just the free-text. A pure
    /// helper over the parsed intent â€” no backend call.
    pub fn effective_query(&self) -> String {
        match &self.path {
            Some(p) if !p.is_empty() => {
                if self.free_text.is_empty() {
                    p.clone()
                } else {
                    format!("{} {}", self.free_text, p)
                }
            }
            _ => self.free_text.clone(),
        }
    }
}

/// The lock-guarded shared slot the rail writes its emitted [`RailQuery`] intent into and a downstream
/// consumer / concurrent swarm reader clones-and-reads off the same lock (the contract's
/// `app_state.search_rail_query: Arc<Mutex<Option<RailQuery>>>`, HBR-SWARM). `None` until the first fire
/// (and reset to `None` by the clear button).
pub type RailQuerySlot = Arc<Mutex<Option<RailQuery>>>;

/// The recognized `content_type` keywords (mirror the backend `LoomBlockContentType` snake_case
/// serialization). Used to validate a `kind:` operator value and to map a scope.
pub fn is_known_content_type(value: &str) -> bool {
    matches!(
        value,
        "note" | "file" | "annotated_file" | "tag_hub" | "journal" | "canvas" | "view_def"
    )
}

/// Parse a raw rail query into a [`RailQuery`], resolving the scope and extracting operator facets.
///
/// A faithful port of the React `parseLoomSearchOperators` (`app/src/lib/loom_search_query.ts`) onto
/// Rust, plus the nine scope-prefix operators:
/// - tokenize on whitespace honoring double-quoted runs (so `path:"a b"` is one token);
/// - a token matching `^[A-Za-z_][A-Za-z0-9_]*:(.*)$` is an OPERATOR; dispatch by its lowercase name:
///   - `tag` / `mention` -> append the value to `tag_ids` (comma/space split);
///   - `path` / `folder`  -> set `path`;
///   - `kind`             -> append the value to `kind_filters` (and to `kind_errors` if unknown);
///   - one of the nine SCOPE keywords with an EMPTY value (`project:`) -> override the scope and drop
///     the token; with a NON-empty value (`project:foo`) -> override the scope AND keep `foo` as
///     free-text (so `project:hello world` -> scope=Project, free_text="hello world");
/// - any other token (non-operator, or an unrecognized operator) stays as free-text.
///
/// The `scope` argument is the pill-selected scope used UNLESS a typed scope-prefix overrides it
/// (typed prefix wins â€” AC-022-4). Never panics on malformed input (PROOF-022-1e).
pub fn parse_rail_query(raw: &str, scope: SearchScope) -> RailQuery {
    parse_rail_query_text(raw, scope, false)
}

fn parse_rail_query_text(raw: &str, scope: SearchScope, preserve_quotes: bool) -> RailQuery {
    let mut out = RailQuery {
        scope,
        ..RailQuery::default()
    };
    let mut free_parts: Vec<String> = Vec::new();

    for token in split_query_tokens(raw) {
        match split_operator(&token) {
            Some((op, value)) => {
                let op_lower = op.to_ascii_lowercase();
                // Scope-prefix operators take priority over the standard operator set so a scope
                // keyword is never misread as a content operator.
                if let Some(scope_override) = SearchScope::from_keyword(&op_lower) {
                    out.scope = scope_override;
                    // `project:foo` keeps `foo` as free-text; bare `project:` is just a scope selector.
                    let value = if preserve_quotes { value } else { value.replace('"', "") };
                    let v = value.trim();
                    if !v.is_empty() {
                        free_parts.push(v.to_owned());
                    }
                    continue;
                }
                match op_lower.as_str() {
                    "tag" | "mention" => {
                        for v in split_operator_values(&value.replace('"', "")) {
                            out.tag_ids.push(v);
                        }
                    }
                    "path" | "folder" => {
                        let value = value.replace('"', "");
                        let v = value.trim();
                        if !v.is_empty() {
                            out.path = Some(v.to_owned());
                        }
                    }
                    "kind" => {
                        for v in split_operator_values(&value.replace('"', "")) {
                            if !is_known_content_type(&v) {
                                out.kind_errors.push(v.clone());
                            }
                            out.kind_filters.push(v);
                        }
                    }
                    // An unrecognized operator (e.g. `foo:bar`) is NOT a known facet; keep the whole
                    // token as free-text (React parity: unknown operators fall through to the query).
                    _ => free_parts.push(if preserve_quotes { token } else { token.replace('"', "") }),
                }
            }
            // A plain token (no `name:` operator shape) is free-text.
            None => free_parts.push(if preserve_quotes { token } else { token.replace('"', "") }),
        }
    }

    out.free_text = free_parts.join(" ");
    out
}

/// Split a raw query into tokens on whitespace, honoring double-quoted runs (port of the React
/// `splitQueryTokens` state machine). Quotes remain available to the receiving parser;
/// whitespace inside quotes is preserved. Unterminated quotes consume to end-of-string. Never panics.
fn split_query_tokens(raw: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut have_token = false;

    for ch in raw.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
                have_token = true; // a `""` is an (empty) token, matching the TS source.
            }
            c if c.is_whitespace() && !in_quotes => {
                if have_token {
                    tokens.push(std::mem::take(&mut current));
                    have_token = false;
                }
            }
            c => {
                current.push(c);
                have_token = true;
            }
        }
    }
    if have_token {
        tokens.push(current);
    }
    tokens
}

/// Split a token into `(operator_name, value)` IFF it matches `^[A-Za-z_][A-Za-z0-9_]*:(.*)$` (the
/// React `OPERATOR_PATTERN`), without a regex dependency (implementation note: prefer no-regex). The
/// value may be empty (`project:`). Returns `None` for a non-operator token.
fn split_operator(token: &str) -> Option<(String, String)> {
    let (name, value) = token.split_once(':')?;
    if name.is_empty() {
        return None;
    }
    let mut chars = name.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some((name.to_owned(), value.to_owned()))
}

/// Split an operator VALUE into individual entries on commas and whitespace, dropping blanks (port of
/// the React `splitOperatorValues`). `tag:a,b c` -> `["a", "b", "c"]`.
fn split_operator_values(value: &str) -> Vec<String> {
    value
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

// ===========================================================================
// Rail rendering.
// ===========================================================================

/// What the rail wants the shell to do after a frame. The rail owns its small transient UI state
/// (query text, active pill, focus); the shell owns the emitted-intent slot. The rail signals
/// scope/clear/fire changes through this outcome so the shell writes/clears the slot.
#[derive(Debug, Clone, PartialEq)]
pub enum RailOutcome {
    /// No explicit action this frame; the header host observes live input.
    None,
    /// Search or Enter emits the selected/typed scope; Loom explicitly searches the whole project.
    /// The shell writes the intent into [`RailQuerySlot`]. The rail makes no backend call. Loom forces
    /// the `project:` scope.
    Fire(Box<RailQuery>),
    /// The `x` clear button was pressed: reset the rail (the shell writes `None` into the slot).
    Clear,
}

/// Inputs the shell hands the rail each frame.
pub struct RailView {
    /// Whether a workspace is selected (an empty/absent workspace disables firing + shows a hint).
    pub has_workspace: bool,
    /// Theme-token colors so the rail flips dark<->light with the rest of the shell.
    pub colors: RailVisuals,
}

/// The theme tokens the rail paints with (from the active palette).
#[derive(Debug, Clone, Copy)]
pub struct RailVisuals {
    /// The active pill's fill.
    pub active_pill_bg: egui::Color32,
    /// An inactive pill's (ghost) fill.
    pub inactive_pill_bg: egui::Color32,
    /// Primary text.
    pub text: egui::Color32,
    /// Accent (the Loom shortcut highlight).
    pub accent: egui::Color32,
}

/// The result of rendering a frame: the outcome PLUS the current parsed query + active scope, so the
/// shell can observe the live intent without owning the egui memory.
pub struct RailFrame {
    pub literal: String,
    pub input_changed: bool,
    pub anchor: egui::Rect,
    pub focused: bool,
    pub select_current: bool,
    pub composing: bool,
    /// What the shell should do (fire / clear / nothing).
    pub outcome: RailOutcome,
    /// The CURRENT parsed query this frame (free-text + scope + facets).
    pub query: RailQuery,
}

/// Per-frame transient UI state stored in egui memory keyed to the rail id. The query text + the
/// pill-selected scope live here so they survive between frames without the shell owning egui memory.
#[derive(Debug, Clone, Default)]
struct RailUiState {
    /// The current raw query text in the input.
    query: String,
    composing: bool,
    /// The pill-selected scope (overridden by a typed colon-prefix at parse time). Defaults to
    /// [`SearchScope::Project`] (the rail's default scope).
    active_scope: SearchScope,
    /// A Loom submission overrides typed scope until the next edit, scope choice or generic search.
    project_shortcut: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RailSubmission {
    Search,
    Activate,
    Loom,
}

/// The egui-memory key for the rail's transient UI state.
fn state_id() -> egui::Id {
    egui::Id::new("bottom-rail.state")
}

/// Render the persistent field and legacy controls inside the host header panel.
pub fn show(ui: &mut egui::Ui, view: RailView) -> RailFrame {
    show_with_composition(ui, view, false)
}

fn show_with_composition(ui: &mut egui::Ui, view: RailView, owner_composing: bool) -> RailFrame {
    let mut state: RailUiState = ui
        .ctx()
        .data_mut(|d| d.get_temp::<RailUiState>(state_id()))
        .unwrap_or_default();

    let mut anchor = egui::Rect::NOTHING;
    let mut focused = false;
    let mut composing_frame = false;
    let mut input_changed = false;
    let mut outcome = RailOutcome::None;
    let mut submission = None;
    let mut clear = false;
    let ime_frame = owner_composing
        || ui.input(|input| input.events.iter().any(|event| matches!(event, egui::Event::Ime(_))));

    let input_egui_id = unsafe { egui::Id::from_high_entropy_bits(RAIL_INPUT_NODE_ID) };
    let clear_egui_id = unsafe { egui::Id::from_high_entropy_bits(RAIL_CLEAR_NODE_ID) };
    let loom_egui_id = unsafe { egui::Id::from_high_entropy_bits(RAIL_LOOM_NODE_ID) };

    ui.horizontal_centered(|ui| {
        // Compact scope menu preserves the original scope identities.
        let menu = ui.menu_button(state.active_scope.to_string(), |ui| {
            for &scope in SearchScope::all() {
                if scope_pill(ui, scope, state.active_scope == scope, &view.colors).clicked() && !state.composing && !ime_frame {
                    input_changed |= state.active_scope != scope || state.project_shortcut;
                    state.active_scope = scope;
                    state.project_shortcut = false;
                    ui.close();
                }
            }
        });
        ui.ctx().accesskit_node_builder(menu.response.id, |node| node.set_author_id("handpick.scopes"));

        ui.separator();

        // â”€â”€ The Loom shortcut + clear button, right-aligned so the input fills the middle. â”€â”€
        // Laid out right-to-left so the input's `desired_width(INFINITY)` fills the gap to the left of
        // these fixed-size buttons (egui right_to_left reserves them first â€” so the input is ALWAYS
        // visible regardless of how many pills render to the left â€” RISK-022-B).
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if fixed_id_button(
                ui,
                egui::Id::new("handpick.search"),
                "handpick.search",
                "Search",
                "Search selected writing or the field in its selected or typed scope",
                view.colors.accent,
                view.colors.inactive_pill_bg,
            ) {
                submission = Some(RailSubmission::Search);
            }

            // Loom shortcut at its FIXED id (NodeId 24): forces project: scope + emits (one click from
            // any surface). Uses the codebase's fixed-id button pattern (allocate + interact at the fixed
            // id + manual paint + widget_info + node) so its AccessKit NodeId is stable across frames /
            // restarts (RISK-022-F), matching its DECLARED_IDENTITIES entry.
            if fixed_id_button(
                ui,
                loom_egui_id,
                RAIL_LOOM_AUTHOR_ID,
                "Loom",
                "Search the whole project (Loom)",
                view.colors.accent,
                view.colors.inactive_pill_bg,
            ) {
                submission = Some(RailSubmission::Loom);
            }

            // Clear (x) button at its FIXED id (NodeId 23).
            if fixed_id_button(
                ui,
                clear_egui_id,
                RAIL_CLEAR_AUTHOR_ID,
                "x",
                "Clear the search",
                view.colors.text,
                view.colors.inactive_pill_bg,
            ) {
                clear = true;
            }

            // â”€â”€ The free-text input fills the remaining middle width at its FIXED id (NodeId 22). â”€â”€
            let edit = egui::TextEdit::singleline(&mut state.query)
                .id(input_egui_id)
                .hint_text("Search your workspaceâ€¦")
                .desired_width(f32::INFINITY);
            let edit_response = ui.add(edit);
            if edit_response.changed() {
                input_changed = true;
                state.project_shortcut = false;
            }
            anchor = edit_response.rect;
            focused = edit_response.gained_focus();
            if edit_response.has_focus() || edit_response.lost_focus() {
                ui.input(|input| {
                    for event in &input.events {
                        if let egui::Event::Ime(ime) = event {
                            match ime {
                                egui::ImeEvent::Enabled | egui::ImeEvent::Preedit(_) => state.composing = true,
                                egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => state.composing = false,
                            }
                        }
                    }
                });
            }
            composing_frame = state.composing || ime_frame;
            if edit_response.lost_focus() { state.composing = false; }
            emit_interactive_node(ui.ctx(), input_egui_id, RAIL_INPUT_AUTHOR_ID);
            ui.ctx().accesskit_node_builder(input_egui_id, |node| {
                node.add_action(egui::accesskit::Action::SetValue);
            });
            if let Some(replacement) = crate::mcp::accesskit_string_set_value(ui, input_egui_id).filter(|_| !state.composing && !ime_frame) {
                input_changed |= state.query != replacement || state.project_shortcut;
                state.query = replacement;
                state.project_shortcut = false;
                ui.ctx().request_repaint();
            }
            if !composing_frame && edit_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submission = Some(RailSubmission::Activate);
            }
        });
    });

    let submission = submission.filter(|_| !composing_frame);
    let select_current = submission == Some(RailSubmission::Activate);
    if let Some(submission) = submission {
        let project_shortcut = submission == RailSubmission::Loom;
        input_changed |= state.project_shortcut != project_shortcut;
        state.project_shortcut = project_shortcut;
        if project_shortcut {
            input_changed |= state.active_scope != SearchScope::Project;
            state.active_scope = SearchScope::Project;
        }
    }

    let mut parsed = parse_rail_query(&state.query, state.active_scope);
    if state.project_shortcut {
        parsed.scope = SearchScope::Project;
    }

    // â”€â”€ Clear: empty the input, reset the scope to default, signal the shell to clear the slot. â”€â”€
    if clear && !composing_frame {
        input_changed |= !state.query.is_empty() || state.active_scope != SearchScope::default()
            || state.project_shortcut;
        state.query.clear();
        state.active_scope = SearchScope::default();
        state.project_shortcut = false;
        parsed = parse_rail_query(&state.query, state.active_scope);
        outcome = RailOutcome::Clear;
    } else if submission.is_some() {
        outcome = RailOutcome::Fire(Box::new(parsed.clone()));
    }

    // Persist the transient state.
    ui.ctx()
        .data_mut(|d| d.insert_temp(state_id(), state.clone()));

    RailFrame {
        literal: state.query.clone(),
        input_changed,
        anchor,
        focused,
        select_current,
        composing: composing_frame,
        outcome,
        query: parsed,
    }
}

/// Render one scope pill as a selectable button carrying its stable author_id (Role::Button). The
/// active pill is filled with the accent-soft background; inactive pills are ghost. The pill label is
/// the colon-suffixed scope string (`"project:"`).
fn scope_pill(
    ui: &mut egui::Ui,
    scope: SearchScope,
    selected: bool,
    colors: &RailVisuals,
) -> egui::Response {
    let fill = if selected {
        colors.active_pill_bg
    } else {
        colors.inactive_pill_bg
    };
    let label = egui::RichText::new(scope.to_string())
        .color(colors.text)
        .small();
    let resp = ui.add(
        egui::Button::new(label)
            .fill(fill)
            .small()
            .selected(selected),
    );
    // Stable author_id + Button role + selected state on the SAME live node egui built for the pill.
    let author_id = scope.author_id();
    let aria = format!("Scope {}", scope);
    ui.ctx().accesskit_node_builder(resp.id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author_id);
        node.set_label(aria);
        if selected {
            node.set_selected(true);
        }
    });
    resp
}

/// Render a small text button at a FIXED `egui::Id` so its AccessKit `NodeId` is stable across frames
/// and process restarts (the codebase's `left_rail::activity_button` pattern: allocate -> interact at
/// the fixed id -> manual paint -> widget_info -> node). Returns `true` if clicked this frame. Used for
/// the rail's two fixed-band controls (Loom shortcut, clear button) so they match their
/// `DECLARED_IDENTITIES` node_ids (RISK-022-F: no per-frame hashed ids for the declared controls).
fn fixed_id_button(
    ui: &mut egui::Ui,
    id: egui::Id,
    author_id: &str,
    glyph: &str,
    tooltip: &str,
    text_color: egui::Color32,
    bg: egui::Color32,
) -> bool {
    let g = ui.painter().layout_no_wrap(
        glyph.to_owned(),
        egui::FontId::proportional(13.0),
        text_color,
    );
    let size = egui::vec2(g.size().x + 12.0, (RAIL_HEIGHT - 8.0).max(16.0));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let resp = ui
        .interact(rect, id, egui::Sense::click())
        .on_hover_text(tooltip);
    if ui.is_rect_visible(rect) {
        let fill = if resp.hovered() {
            bg
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 3.0, fill);
        let g2 = ui.painter().layout_no_wrap(
            glyph.to_owned(),
            egui::FontId::proportional(13.0),
            text_color,
        );
        ui.painter().galley(
            egui::pos2(
                rect.center().x - g2.size().x * 0.5,
                rect.center().y - g2.size().y * 0.5,
            ),
            g2,
            text_color,
        );
    }
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tooltip)
    });
    let author_id = author_id.to_owned();
    let label = tooltip.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author_id);
        node.set_label(label);
    });
    resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    // â”€â”€ Fixed control ids in the fresh disjoint band â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn rail_control_ids_in_disjoint_fresh_band() {
        for id in [RAIL_INPUT_NODE_ID, RAIL_CLEAR_NODE_ID, RAIL_LOOM_NODE_ID] {
            assert!(
                (22..=24).contains(&id),
                "rail control id {id} in band 22..=24"
            );
            assert!(
                id < crate::accessibility::PANE_NODE_ID_BASE,
                "rail control id {id} below the pane id base"
            );
        }
        // The three ids are distinct.
        assert_ne!(RAIL_INPUT_NODE_ID, RAIL_CLEAR_NODE_ID);
        assert_ne!(RAIL_CLEAR_NODE_ID, RAIL_LOOM_NODE_ID);
    }

    // â”€â”€ Scope enum: Display, default, all(), keyword round-trip â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn scope_display_is_colon_suffixed() {
        assert_eq!(SearchScope::Project.to_string(), "project:");
        assert_eq!(SearchScope::File.to_string(), "file:");
        assert_eq!(SearchScope::Layout.to_string(), "layout:");
    }

    #[test]
    fn scope_default_is_project() {
        assert_eq!(SearchScope::default(), SearchScope::Project);
    }

    #[test]
    fn scope_all_has_nine_in_order() {
        let all = SearchScope::all();
        assert_eq!(all.len(), 9, "exactly nine scopes");
        assert_eq!(all[0], SearchScope::Project);
        assert_eq!(all[8], SearchScope::Layout);
        // keyword round-trips for every scope.
        for &s in all {
            assert_eq!(SearchScope::from_keyword(s.keyword()), Some(s));
        }
        assert_eq!(SearchScope::from_keyword("nope"), None);
    }

    // â”€â”€ parse_rail_query: PROOF-022-1 (a)..(e) + AC-022-4 â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn parse_free_text_no_operators() {
        // (a) plain free-text, no operators.
        let q = parse_rail_query("hello world", SearchScope::File);
        assert_eq!(q.free_text, "hello world");
        assert_eq!(
            q.scope,
            SearchScope::File,
            "pill scope retained when no prefix typed"
        );
        assert!(q.tag_ids.is_empty());
        assert!(q.path.is_none());
        assert!(q.kind_filters.is_empty());
        assert!(q.kind_errors.is_empty());
    }

    #[test]
    fn parse_tag_mention_path_kind_combined() {
        // (b) tag + mention + path + kind operators combined.
        let q = parse_rail_query(
            "foo tag:t1,t2 mention:m1 path:src/lib kind:note bar",
            SearchScope::Project,
        );
        assert_eq!(
            q.free_text, "foo bar",
            "operators stripped, free-text preserved in order"
        );
        assert_eq!(
            q.tag_ids,
            vec!["t1", "t2", "m1"],
            "tag + mention values collected"
        );
        assert_eq!(q.path.as_deref(), Some("src/lib"));
        assert_eq!(q.kind_filters, vec!["note"]);
        assert!(q.kind_errors.is_empty(), "note is a valid content_type");
        // note is a valid kind -> it becomes the resolved content_type.
        assert_eq!(q.resolved_content_type().as_deref(), Some("note"));
    }

    #[test]
    fn parse_scope_prefix_override_with_residual_free_text() {
        // (c) typed scope-prefix overrides the pill scope; residual free-text kept (AC-022-4).
        let q = parse_rail_query("project:hello world", SearchScope::File);
        assert_eq!(
            q.scope,
            SearchScope::Project,
            "typed project: overrides the File pill"
        );
        assert_eq!(
            q.free_text, "hello world",
            "the prefix is stripped from the free-text"
        );
    }

    #[test]
    fn parse_bare_scope_prefix_is_selector_only() {
        // A bare `stash:` with no value is a scope selector and contributes no free-text.
        let q = parse_rail_query("stash:", SearchScope::Project);
        assert_eq!(q.scope, SearchScope::Stash);
        assert_eq!(q.free_text, "");
    }

    #[test]
    fn parse_quoted_token_with_spaces() {
        // (d) a double-quoted run is one token (spaces preserved); a quoted path keeps its spaces.
        let q = parse_rail_query("\"hello world\" path:\"my folder\"", SearchScope::Project);
        assert_eq!(
            q.free_text, "hello world",
            "the quoted free-text run is one token"
        );
        assert_eq!(
            q.path.as_deref(),
            Some("my folder"),
            "the quoted path keeps its space"
        );
    }

    #[test]
    fn parse_invalid_kind_is_captured_not_panicked() {
        // (e) an invalid kind: value is captured in kind_errors without panicking.
        let q = parse_rail_query("kind:bogus thing", SearchScope::Project);
        assert_eq!(q.kind_filters, vec!["bogus"]);
        assert_eq!(
            q.kind_errors,
            vec!["bogus"],
            "unknown kind captured as a soft error"
        );
        assert_eq!(q.free_text, "thing");
        // No valid kind + Project scope -> no content_type filter.
        assert_eq!(q.resolved_content_type(), None);
    }

    #[test]
    fn parse_unknown_operator_stays_free_text() {
        // An unrecognized operator (`foo:bar`) is not a known facet -> kept verbatim as free-text.
        let q = parse_rail_query("foo:bar baz", SearchScope::Project);
        assert_eq!(q.free_text, "foo:bar baz");
        assert!(q.tag_ids.is_empty());
        assert!(q.kind_filters.is_empty());
    }

    #[test]
    fn parse_empty_is_empty() {
        let q = parse_rail_query("   ", SearchScope::Project);
        assert_eq!(q.free_text, "");
        assert!(q.tag_ids.is_empty());
    }

    // â”€â”€ content_type + effective_query mapping (intent-derived helpers, no backend call) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn file_scope_maps_to_file_content_type() {
        let q = parse_rail_query("readme", SearchScope::File);
        assert_eq!(q.resolved_content_type().as_deref(), Some("file"));
    }

    #[test]
    fn path_folds_into_effective_query() {
        let q = parse_rail_query("alpha path:docs", SearchScope::Project);
        assert_eq!(q.effective_query(), "alpha docs");
        let only_path = parse_rail_query("path:docs", SearchScope::Project);
        assert_eq!(only_path.effective_query(), "docs");
    }

    // â”€â”€ Emitted-intent slot: the fire path WRITES, a reader CLONES-and-READS (HBR-SWARM) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn rail_query_slot_write_then_read_clone() {
        // The contract's shared slot: the fire path writes the emitted RailQuery into the lock-guarded
        // Option, and a downstream consumer / swarm reader clones-and-reads it off the SAME lock. This
        // proves the lock-guarded write/read contract the rail's fire path relies on (CONTROL-022-D).
        let slot: RailQuerySlot = Arc::new(Mutex::new(None));

        // Reader sees None before any fire.
        assert!(slot.lock().unwrap().is_none(), "slot empty before a fire");

        // WRITE (the fire path): replace the slot with the emitted intent.
        let emitted = parse_rail_query("project:hello world", SearchScope::File);
        slot.lock().unwrap().replace(emitted.clone());

        // READ (a downstream consumer / concurrent swarm reader): clone the intent off the lock.
        let read_back = slot
            .lock()
            .unwrap()
            .clone()
            .expect("intent present after a fire");
        assert_eq!(read_back, emitted);
        assert_eq!(
            read_back.scope,
            SearchScope::Project,
            "typed prefix won in the emitted intent"
        );
        assert_eq!(read_back.free_text, "hello world");

        // CLEAR (the x button): the fire path writes None back, the reader sees an empty slot again.
        *slot.lock().unwrap() = None;
        assert!(
            slot.lock().unwrap().is_none(),
            "slot reset to None by clear"
        );
    }

    #[test]
    fn rail_query_slot_shared_across_clones() {
        // A second Arc clone (held by a downstream consumer / swarm thread) observes a write made
        // through the first handle â€” the slot is genuinely shared, not copied.
        let writer: RailQuerySlot = Arc::new(Mutex::new(None));
        let reader = writer.clone();
        let emitted = parse_rail_query("tag:t1 docs", SearchScope::Project);
        writer.lock().unwrap().replace(emitted.clone());
        assert_eq!(
            reader
                .lock()
                .unwrap()
                .clone()
                .expect("reader sees the write"),
            emitted
        );
    }
}


type Hit = crate::quick_switcher::LoomGraphSearchHit;
enum HeaderFetchError {
    Denied,
    Retired,
    Message(String),
}
impl From<String> for HeaderFetchError {
    fn from(message: String) -> Self { Self::Message(message) }
}
impl From<&str> for HeaderFetchError {
    fn from(message: &str) -> Self { Self::Message(message.to_owned()) }
}
type HeaderPage = (Arc<()>, handpick::Fence, Result<Vec<Hit>, HeaderFetchError>);

struct AdmittedHeaderQuery {
    parsed: RailQuery,
    draft_revision: u64,
}

/// Account-bound host state; shared Handpick owns query fences and grouped presentation.
#[derive(Default)]
pub struct HandpickHost {
    notes: composition::MountedNotes,
    account: Option<Arc<crate::local_account::AuthenticatedContext>>,
    workspace: String,
    backend: String,
    binding: Arc<()>,
    input_seen: bool,
    pub binding_changed: bool,
    session: Option<handpick::Session>,
    picker: handpick::Picker,
    rows: Vec<Hit>,
    commands: Vec<&'static crate::command_registry::AppCommand>,
    command_context: Option<crate::command_registry::EditorMenuEnableContext>,
    command_action: Option<&'static str>,
    delivery: Arc<Mutex<Option<HeaderPage>>>,
    task: Option<tokio::task::JoinHandle<()>>,
    query: Option<AdmittedHeaderQuery>,
    due: Option<std::time::Instant>,
    error: Option<String>,
    open: bool,
    exhausted: bool,
    skip_focus_open: bool,
    lookup_from_notes: bool,
    observations: bool,
}

impl Drop for HandpickHost {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Stale); }
        }
        if let Some(session) = &mut self.session { session.revoke(); }
    }
}

impl HandpickHost {
    pub fn set_command_context(&mut self, context: crate::command_registry::EditorMenuEnableContext) {
        self.command_context = Some(context);
    }

    pub fn take_command_action(&mut self) -> Option<&'static str> {
        self.command_action.take()
    }

    fn command_enabled(&self, command: &crate::command_registry::AppCommand) -> bool {
        self.account.as_ref().is_some_and(|account| account.is_active())
            && !crate::command_registry::effective_disabled(command,
                self.command_context.unwrap_or(crate::command_registry::EditorMenuEnableContext::unavailable()))
    }

    fn activate_command(&mut self, key: &handpick::ResultKey) -> bool {
        if key.provider.as_str() != "native.commands" { return false; }
        let command = self.commands.iter().copied().find(|command| command.id == key.result.as_str());
        let Some(command) = command.filter(|command| self.command_enabled(command)) else { return false; };
        if let Some(session) = &mut self.session {
            if let Err(error) = session.select(key.clone()) {
                self.error = Some(error.to_string());
                return false;
            }
        } else { return false; }
        self.command_action = Some(command.id);
        true
    }

    pub fn matches_context(&self, workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>, base: &str) -> bool {
        self.workspace == workspace && self.backend == base && match (&self.account, account) {
            (Some(previous), Some(current)) => Arc::ptr_eq(previous, current) && current.is_active(),
            (None, None) => true,
            _ => false,
        }
    }

    pub fn retained_note_bindings(&self) -> Vec<(String, String)> {
        self.notes.retained_bindings()
    }

    pub fn visible_note_binding(&self) -> Option<(String, String)> {
        self.notes.visible_binding()
    }

    pub fn focused_note_binding(&self) -> Option<(String, String)> {
        self.notes.focused_binding()
    }

    pub fn take_released_note_bindings(&mut self) -> Vec<(String, String)> {
        self.notes.take_released_bindings()
    }

    fn bind(&mut self, ctx: &egui::Context, workspace: &str,
        account: Option<Arc<crate::local_account::AuthenticatedContext>>, base: &str) {
        let same = self.matches_context(workspace, account.as_ref(), base);
        self.binding_changed = !same;
        if same { return; }
        let mut notes = std::mem::take(&mut self.notes);
        notes.retire_session();
        let command_context = self.command_context;
        *self = Self::default();
        self.command_context = command_context;
        self.notes = notes;
        self.binding_changed = true;
        ctx.data_mut(|data| data.remove::<RailUiState>(state_id()));
        let input = unsafe { egui::Id::from_high_entropy_bits(RAIL_INPUT_NODE_ID) };
        egui::TextEdit::store_state(ctx, input, egui::text_edit::TextEditState::default());
        ctx.memory_mut(|memory| memory.surrender_focus(input));
        self.workspace = workspace.to_owned();
        self.backend = base.to_owned();
        self.account = account.filter(|account| account.is_active());
        if let Some(account) = &self.account {
            match handpick::ContextId::new(account.session_id.clone()) {
                Ok(context) => self.session = Some(handpick::Session::new(context, handpick::Limits {
                    max_text_bytes: 65536, max_pins: 0, max_items: 1024,
                    max_item_bytes: 65536, max_actions_per_item: 1,
                })),
                Err(_) => self.error = Some("Invalid authenticated search context".into()),
            }
        }
    }

    fn replace_input(&mut self, frame: &RailFrame) {
        let changed = frame.input_changed || !self.input_seen;
        if !changed { return; }
        self.input_seen = true;
        // Retire all prior admission and delivery before attempting the exact new literal.
        self.query = None;
        if let Some(task) = self.task.take() {
            task.abort();
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Stale); }
        }
        self.delivery = Arc::new(Mutex::new(None));
        self.rows.clear();
        self.commands.clear();
        self.command_action = None;
        self.picker = handpick::Picker::new();
        self.error = None;
        self.exhausted = false;
        self.open = !frame.literal.trim().is_empty();
        self.due = None;
        if let Some(session) = &mut self.session {
            if let Err(error) = session.begin().and_then(|_| session.set_draft(frame.literal.clone())) {
                self.error = Some(error.to_string());
                if self.observations && frame.input_changed { record_gui_observation(GuiOp::HandpickInput, GuiPhase::Rejected); }
                return;
            }
            let mut parsed = parse_rail_query_text(&frame.literal, frame.query.scope, true);
            // The rail already resolved selected, typed and explicit Loom scope precedence.
            parsed.scope = frame.query.scope;
            self.query = Some(AdmittedHeaderQuery {
                parsed,
                draft_revision: session.draft_revision(),
            });
        }
        if self.observations && frame.input_changed { record_gui_observation(GuiOp::HandpickInput, GuiPhase::Accepted); }
        if self.open {
            self.due = Some(std::time::Instant::now() + std::time::Duration::from_millis(150));
        }
    }

    fn admitted_query(&self, literal: &str) -> Option<&AdmittedHeaderQuery> {
        self.query.as_ref().filter(|admitted| self.session.as_ref().is_some_and(|session|
            session.draft() == literal && session.draft_revision() == admitted.draft_revision))
    }

    fn fence_legacy_intent(&self, frame: &mut RailFrame) {
        if matches!(&frame.outcome, RailOutcome::Fire(_)) && self.admitted_query(&frame.literal).is_none() {
            frame.outcome = RailOutcome::None;
        }
    }

    fn request(&mut self, ctx: &egui::Context, runtime: Option<&tokio::runtime::Handle>, literal: &str) {
        self.due = None;
        if self.task.is_some() { return; }
        let Some(admitted) = self.admitted_query(literal) else {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Rejected); }
            self.error = Some("Current input has not been admitted; edit the query before searching".into());
            return;
        };
        let mut query = admitted.parsed.clone();
        if !query.kind_errors.is_empty() {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Rejected); }
            self.error = Some("Unsupported kind filter; query retained".into()); return;
        }
        use unicode_normalization::UnicodeNormalization;
        let normalized: String = query.free_text.nfc().collect();
        let parsed = match handpick::parse_query(&normalized) {
            Ok(parsed) => parsed,
            Err(error) => { self.error = Some(error.to_string()); return; }
        };
        if parsed.incomplete || !parsed.unsupported.is_empty() {
            self.error = Some("This query mode needs its native provider; query retained".into()); return;
        }
        if matches!(parsed.purpose, handpick::QueryPurpose::Commands | handpick::QueryPurpose::Settings) {
            if query.scope != SearchScope::Project || !query.tag_ids.is_empty() || query.path.is_some()
                || !query.kind_filters.is_empty() || has_legacy_filters(literal)
                || parsed.tag.is_some() || parsed.area.is_some() || parsed.task.is_some()
                || parsed.result_type.as_deref().is_some_and(|kind| !matches!(kind, "command" | "settings")) {
                self.error = Some("These filters are not supported by the command owner; query retained".into());
                return;
            }
            if let Err(error) = self.present_commands(&parsed) { self.error = Some(error); }
            return;
        }
        let source_kind = graph_source_kind(parsed.result_type.as_deref());
        if !matches!(query.scope, SearchScope::Project | SearchScope::File) {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Unavailable); }
            self.error = Some("This scope has no graph provider yet. Its existing search intent remains available.".into());
            return;
        }
        if parsed.words.is_empty() {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Rejected); }
            self.error = Some("Enter search text to query the graph".into()); return;
        }
        if (parsed.result_type.is_some() && source_kind.is_none()) || parsed.area.is_some() || parsed.task.is_some() {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Unavailable); }
            self.error = Some("This query mode needs its native provider; query retained".into()); return;
        }
        // Preserve token spelling and quoting. The type travels through source_kinds, never q.
        let tokens = split_query_tokens(&normalized);
        if tokens.iter().any(|token| token.contains('"') && token.chars().any(char::is_whitespace)) {
            // The current graph owner strips quotes then splits words; it cannot promise a phrase.
            self.error = Some("Exact phrase search needs its graph provider; query retained".into()); return;
        }
        query.free_text = tokens.into_iter().filter(|token|
            !split_operator(token).is_some_and(|(name, _)| name.eq_ignore_ascii_case("type")))
            .collect::<Vec<_>>().join(" ");
        let query = HeaderGraphQuery { intent: query, source_kind };
        let (Some(account), Some(runtime), Some(session)) = (&self.account, runtime, &mut self.session) else {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Unavailable); }
            self.error = Some("Connect an authenticated workspace to search".into()); return;
        };
        if !account.is_active() || self.workspace.is_empty() {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Rejected); }
            return;
        }
        let fence = match session.begin() {
            Ok(fence) => fence,
            Err(error) => {
                if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Rejected); }
                self.error = Some(error.to_string()); return;
            }
        };
        if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Accepted); }
        self.error = None;
        let offset = self.rows.len();
        let account = account.clone();
        let workspace = self.workspace.clone();
        let base = self.backend.clone();
        let binding = self.binding.clone();
        let delivery = self.delivery.clone();
        let repaint = ctx.clone();
        self.task = Some(runtime.spawn(async move {
            let result = fetch_header_page(&base, &workspace, account, &query, offset).await;
            if let Ok(mut slot) = delivery.lock() { *slot = Some((binding, fence, result)); }
            repaint.request_repaint();
        }));
        if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Pending); }
    }

    fn restart(&mut self, ctx: &egui::Context, runtime: Option<&tokio::runtime::Handle>, literal: &str) {
        if let Some(task) = self.task.take() {
            task.abort();
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Stale); }
        }
        self.delivery = Arc::new(Mutex::new(None));
        self.rows.clear();
        self.commands.clear();
        self.command_action = None;
        self.picker = handpick::Picker::new();
        self.exhausted = false;
        self.request(ctx, runtime, literal);
    }

    fn present_commands(&mut self, parsed: &handpick::ParsedQuery) -> Result<(), String> {
        if !self.account.as_ref().is_some_and(|account| account.is_active()) {
            return Err("Connect an authenticated workspace to use commands".into());
        }
        let commands: Vec<_> = crate::command_registry::filtered_commands(
            parsed.words.first().map_or("", String::as_str)).into_iter()
            .filter(|command| parsed.words.iter().skip(1).all(|term|
                crate::command_registry::matches_query(command, term)))
            .filter(|command| parsed.purpose != handpick::QueryPurpose::Settings || command.id.starts_with("settings."))
            .collect();
        let descriptors = commands.iter().map(|command| Ok(handpick::SearchItem {
            key: handpick::ResultKey {
                provider: handpick::ProviderId::new("native.commands").map_err(str::to_owned)?,
                result: handpick::ResultId::new(command.id).map_err(str::to_owned)?,
            },
            label: command.label.into(), detail: Some(command.description.into()),
            actions: vec![handpick::Action {
                id: handpick::ActionId::new(command.id).map_err(str::to_owned)?,
                kind: if parsed.purpose == handpick::QueryPurpose::Settings {
                    handpick::ActionKind::RevealSetting
                } else { handpick::ActionKind::RunCommand },
            }],
        })).collect::<Result<Vec<_>, String>>()?;
        let session = self.session.as_mut().ok_or("Search session unavailable")?;
        let fence = session.begin().map_err(|error| error.to_string())?;
        session.deliver(&fence, handpick::Channel::Search, handpick::DeliveryStatus::Complete, descriptors.clone())
            .map_err(|error| error.to_string())?;
        let loaded = descriptors.len() as u32;
        self.picker.set_results(descriptors.into_iter().map(|item| handpick::PickerItem {
            item, category: handpick::ResultCategory::Actions, location: None,
            highlights: Vec::new(), preview: None,
        }).collect(), vec![handpick::PickerCoverage {
            category: handpick::ResultCategory::Actions, status: handpick::DeliveryStatus::Complete,
            loaded, total: Some(loaded),
        }]).map_err(|error| error.to_string())?;
        self.rows.clear();
        self.commands = commands;
        self.exhausted = true;
        self.error = None;
        if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Completed); }
        Ok(())
    }

    fn drain(&mut self) {
        let delivery = self.delivery.lock().ok().and_then(|mut slot| slot.take());
        let Some((binding, fence, result)) = delivery else { return; };
        let Some(session) = &mut self.session else {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Stale); }
            return;
        };
        if !Arc::ptr_eq(&self.binding, &binding)
            || !self.account.as_ref().is_some_and(|account| account.is_active())
            || !self.query.as_ref().is_some_and(|admitted| admitted.draft_revision == session.draft_revision())
            || session.fence() != fence {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Stale); }
            return;
        }
        self.task = None;
        let page = match result {
            Ok(page) => page,
            Err(error) => {
                if self.observations {
                    let phase = match &error {
                        HeaderFetchError::Denied => GuiPhase::Rejected,
                        HeaderFetchError::Retired => GuiPhase::Stale,
                        HeaderFetchError::Message(_) => GuiPhase::Failed,
                    };
                    record_gui_observation(GuiOp::HandpickRetrieval, phase);
                }
                self.error = Some(match error {
                    HeaderFetchError::Denied | HeaderFetchError::Retired => {
                        self.rows.clear();
                        self.picker = handpick::Picker::new();
                        let _ = session.deliver(&fence, handpick::Channel::Search, handpick::DeliveryStatus::Failed, Vec::new());
                        "Search access denied or account retired; reconnect or change workspace".to_owned()
                    }
                    HeaderFetchError::Message(message) => message,
                });
                return;
            }
        };
        let exhausted = page.len() < 50;
        let mut rows = self.rows.clone();
        rows.extend(page);
        let descriptors = match rows.iter().map(header_item).collect::<Result<Vec<_>, _>>() {
            Ok(items) => items,
            Err(error) => {
                if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Rejected); }
                self.error = Some(error); return;
            }
        };
        let status = if exhausted { handpick::DeliveryStatus::Complete } else { handpick::DeliveryStatus::Partial };
        if let Err(error) = session.deliver(&fence, handpick::Channel::Search, status, descriptors.clone()) {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Rejected); }
            self.error = Some(format!("Search admission failed ({error}); restart the search or narrow the query"));
            return;
        }
        let items: Vec<_> = descriptors.into_iter().zip(&rows).map(|(item, hit)| handpick::PickerItem {
            item, category: header_category(hit), location: header_location(hit), highlights: Vec::new(), preview: None,
        }).collect();
        let coverage = handpick::ResultCategory::ALL.into_iter().filter_map(|category| {
            let loaded = items.iter().filter(|row| row.category == category).count() as u32;
            (loaded > 0).then_some(handpick::PickerCoverage { category, status, loaded,
                total: None })
        }).collect();
        if let Err(error) = self.picker.set_results(items, coverage) {
            if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Rejected); }
            self.error = Some(error.to_string()); return;
        }
        self.rows = rows;
        self.exhausted = exhausted;
        // Accepted transport/projection transition only; this says nothing about corpus coverage.
        if self.observations { record_gui_observation(GuiOp::HandpickRetrieval, GuiPhase::Completed); }
    }
}

/// Mount the persistent field and attached grouped dropdown without owning native Notes drafts.
pub fn show_header(ctx: &egui::Context, view: RailView, host: &mut HandpickHost,
    workspace: &str, account: Option<Arc<crate::local_account::AuthenticatedContext>>,
    runtime: Option<&tokio::runtime::Handle>, base: &str, observations: bool,
    current_note: Option<Arc<Mutex<crate::rich_editor::renderer::rich_editor_widget::RichEditorState>>>,
    notes_store: &crate::editor_pane_factories::RichEditorDocumentStore,
    notes_mount: &crate::editor_pane_factories::RichEditorPaneMount) -> (RailFrame, Option<Hit>) {
    host.observations = observations;
    host.bind(ctx, workspace, account, base);
    host.observations = observations;
    host.drain();
    host.notes.prepare(ctx, workspace, host.account.as_ref(), base, current_note,
        notes_store, runtime, observations);
    let input_id = unsafe { egui::Id::from_high_entropy_bits(RAIL_INPUT_NODE_ID) };
    let native_popup_open = egui::Popup::is_any_open(ctx);
    let notes_completion_open = host.notes.completion_open();
    let composing = ctx.data_mut(|data| data.get_temp::<RailUiState>(state_id()))
        .is_some_and(|state| state.composing)
        || host.notes.composing()
        || ctx.input(|input| input.events.iter().any(|event| matches!(event, egui::Event::Ime(_))));
    if host.open && !native_popup_open && !composing && ctx.memory(|memory| memory.has_focus(input_id)) {
        for (key, delta) in [(egui::Key::ArrowDown, 1), (egui::Key::ArrowUp, -1)] {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) { host.picker.move_selection(delta); }
        }
    }
    if let Some(text) = host.notes.take_selected_query(ctx, composing) {
        ctx.data_mut(|data| {
            let mut state = data.get_temp::<RailUiState>(state_id()).unwrap_or_default();
            state.query = text;
            state.project_shortcut = false;
            data.insert_temp(state_id(), state);
        });
        host.input_seen = false;
        host.open = true;
        host.lookup_from_notes = true;
        ctx.memory_mut(|memory| memory.request_focus(input_id));
    }
    host.notes.prepare_search_button(ctx, composing);
    let mut expand_id = None;
    let header = egui::TopBottomPanel::top("handshake_handpick_header")
        .resizable(false).show(ctx, |ui| {
            let field = egui::Frame::new().fill(view.colors.inactive_pill_bg)
                .inner_margin(4.0).show(ui, |ui| {
                    let frame = ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let expand = ui.add_enabled(host.account.as_ref().is_some_and(|account| account.is_active()),
                            egui::Button::new(if host.notes.expanded() { "Compact" } else { "Expand" }));
                        expand_id = Some(expand.id);
                        ctx.accesskit_node_builder(expand.id, |node| {
                            node.set_author_id("handpick.notes.open");
                            node.set_expanded(host.notes.expanded());
                        });
                        if expand.clicked() && !composing { host.notes.toggle(); }
                        expand.on_hover_text("Expand or compact the attached note. Right-click to attach the current Notes editor.")
                            .context_menu(|ui| {
                                let attach = ui.add_enabled(!composing && !host.notes.composing(), egui::Button::new("Attach current note"));
                                ctx.accesskit_node_builder(attach.id, |node| node.set_author_id("handpick.notes.attach-current"));
                                if attach.clicked() { host.notes.request_attach_current(); ui.close(); }
                            });
                        host.notes.controls(ui, workspace, host.account.as_ref(), base, composing);
                        show_with_composition(ui, view, composing)
                    }).inner;
                    host.notes.body(ui, notes_mount);
                    frame
                });
            if let Some(account) = host.account.as_ref() {
                let rim = egui::Rect::from_min_max(
                    egui::pos2(field.inner.anchor.left(), field.response.rect.top()),
                    egui::pos2(field.inner.anchor.right(), field.inner.anchor.top()),
                );
                host.notes.bookmarks_on_rim(ui, rim, workspace, base, account);
            }
            field.inner
        });
    let popup_anchor = egui::pos2(header.inner.anchor.left(), header.response.rect.bottom());
    let mut frame = header.inner;
    frame.composing |= composing;
    let mut writing_search_error = None;
    if ctx.read_response(egui::Id::new("handpick.search")).is_some_and(|response| response.clicked())
        && !frame.composing && matches!(&frame.outcome, RailOutcome::Fire(_)) {
        match host.notes.take_search_button_input(ctx, workspace, host.account.as_ref(), base, composing) {
            Ok(Some(text)) => {
                let scope = ctx.data_mut(|data| {
                    let mut state = data.get_temp::<RailUiState>(state_id()).unwrap_or_default();
                    state.query = text.clone();
                    state.project_shortcut = false;
                    let scope = state.active_scope;
                    data.insert_temp(state_id(), state);
                    scope
                });
                frame.literal = text;
                frame.query = parse_rail_query(&frame.literal, scope);
                frame.input_changed = true;
                frame.outcome = RailOutcome::Fire(Box::new(frame.query.clone()));
                host.lookup_from_notes = true;
                ctx.memory_mut(|memory| memory.request_focus(input_id));
            }
            Ok(None) => {}
            Err(error) => {
                frame.outcome = RailOutcome::None;
                frame.input_changed = true;
                writing_search_error = Some(error);
            }
        }
    }
    if host.notes.space_limited {
        let message = "The attached Notes editor needs more room. Compact Notes or enlarge the window; the editor is retained.";
        if let Some(id) = expand_id {
            ctx.accesskit_node_builder(id, |node| node.set_description(message));
            if let Some(response) = ctx.read_response(id) { response.on_hover_text(message); }
        }
    }
    host.replace_input(&frame);
    if let Some(error) = writing_search_error {
        host.due = None;
        host.error = Some(error);
        host.open = true;
    }
    if let Ok(parsed) = handpick::parse_query(&frame.literal) {
        host.notes.set_query_purpose(parsed.purpose);
    }
    ctx.accesskit_node_builder(input_id, |node| node.set_expanded(host.open));
    let skip_focus_open = std::mem::take(&mut host.skip_focus_open);
    if frame.focused && !skip_focus_open && !frame.literal.trim().is_empty() { host.open = true; }
    let mut picked = None;
    if host.open && !frame.composing && !native_popup_open && !notes_completion_open && !egui::Popup::is_any_open(ctx) {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            host.open = false;
            host.skip_focus_open = true;
            if std::mem::take(&mut host.lookup_from_notes) {
                host.notes.focus_editor();
                ctx.request_repaint();
            } else { ctx.memory_mut(|memory| memory.request_focus(input_id)); }
        }
    }
    if matches!(&frame.outcome, RailOutcome::Clear) {
        host.open = false;
    } else if matches!(&frame.outcome, RailOutcome::Fire(_)) && !frame.composing {
        if host.open && frame.select_current {
            if let Some(key) = host.picker.view().selected {
                if key.provider.as_str() == "native.commands" {
                    host.activate_command(&key);
                }
                picked = host.rows.iter().find(|hit| header_key(hit).ok().as_ref() == Some(&key)
                    && crate::quick_switcher::resolve_open_target(hit).enabled()).cloned();
            }
        }
        if picked.is_none() && host.command_action.is_none() { host.restart(ctx, runtime, &frame.literal); host.open = true; }
    }
    if !frame.composing && host.due.is_some_and(|due| std::time::Instant::now() >= due) {
        host.request(ctx, runtime, &frame.literal);
    }
    if let Some(due) = host.due { ctx.request_repaint_after(due.saturating_duration_since(std::time::Instant::now())); }
    let show_popup = host.open && !egui::Popup::is_any_open(ctx);
    let popup_frame = egui::Frame::popup(&ctx.style());
    let popup_height = results_height(ctx, popup_anchor.y, popup_frame.total_margin().sum().y);
    if show_popup && popup_height.is_none() {
        let message = "Search results need more room. Compact Notes or enlarge the window; the query and results are retained.";
        ctx.accesskit_node_builder(input_id, |node| node.set_description(message));
        if let Some(response) = ctx.read_response(input_id) { response.on_hover_text(message); }
    }
    if let (true, Some(popup_height)) = (show_popup, popup_height) {
        let popup = egui::Area::new(egui::Id::new("handpick.results"))
            .order(egui::Order::Foreground).fixed_pos(popup_anchor).constrain(false)
            .show(ctx, |ui| popup_frame.show(ui, |ui| {
                ui.set_width(frame.anchor.width().min(720.0));
                ctx.accesskit_node_builder(ui.id(), |node| {
                    node.set_role(accesskit::Role::ListBox);
                    node.set_author_id("handpick.results");
                    node.set_label("Workspace search results");
                });
                egui::ScrollArea::vertical().min_scrolled_height(0.0).max_height(popup_height)
                    .show(ui, |ui| {
                        if let Some(session) = &host.session {
                            if let Ok(handpick::InputState::Source { form, .. }) = session.input_state(&session.fence(), frame.composing) {
                                ui.label(match form { handpick::SourceForm::Prefix => "Source address prefix", handpick::SourceForm::Url => "Source address" });
                                let source = ui.add_enabled(false, egui::Button::new("Add source"));
                                ctx.accesskit_node_builder(source.id, |node| node.set_author_id("handpick.source.add"));
                                source.on_hover_text("The native source-management owner is not bound to this header");
                            }
                        }
                        if host.task.is_some() || host.due.is_some() { ui.label("Searchingâ€¦"); }
                        if let Some(error) = &host.error {
                            ui.colored_label(ui.visuals().error_fg_color, error);
                            if host.task.is_none() {
                                let retry = ui.button("Retry");
                                ctx.accesskit_node_builder(retry.id, |node| node.set_author_id("handpick.retry"));
                                if retry.clicked() { host.request(ctx, runtime, &frame.literal); }
                            }
                        }
                        let view = host.picker.view();
                        if host.exhausted && host.rows.is_empty() && host.commands.is_empty() { ui.label("No matches"); }
                        for group in view.groups {
                            let label = category_label(group.category);
                            let count = group.total.map_or_else(|| format!("{} loaded Â· total unknown", group.loaded), |total| total.to_string());
                            let heading = ui.button(format!("{label} Â· {count}"));
                            ctx.accesskit_node_builder(heading.id, |node| {
                                node.set_author_id(format!("handpick.group.{label}"));
                                node.set_expanded(!group.collapsed);
                            });
                            if heading.clicked() { host.picker.toggle(group.category); }
                            for row in group.items {
                                let selected = view.selected.as_ref() == Some(&row.item.key);
                                if let Some(command) = host.commands.iter().copied().find(|command|
                                    row.item.key.provider.as_str() == "native.commands" && command.id == row.item.key.result.as_str()) {
                                    let response = ui.push_id(command.stable_id, |ui| ui.add_enabled(
                                        host.command_enabled(command) && !frame.composing,
                                        egui::Button::new(command.label).selected(selected))).inner;
                                    ctx.accesskit_node_builder(response.id, |node| {
                                        node.set_role(accesskit::Role::ListBoxOption);
                                        node.set_author_id(format!("handpick.command.{}", command.stable_id));
                                        node.set_selected(selected);
                                    });
                                    if response.clicked() { host.activate_command(&row.item.key); }
                                    ui.small(command.description);
                                    continue;
                                }
                                if let Some(hit) = host.rows.iter().find(|hit| header_key(hit).ok().as_ref() == Some(&row.item.key)) {
                                    let supported = crate::quick_switcher::resolve_open_target(hit).enabled();
                                    let response = ui.push_id(
                                        (row.item.key.provider.as_str(), row.item.key.result.as_str()),
                                        |ui| ui.add_enabled(supported && !frame.composing,
                                            egui::Button::new(&row.item.label).selected(selected)),
                                    ).inner;
                                    ctx.accesskit_node_builder(response.id, |node| {
                                        node.set_role(accesskit::Role::ListBoxOption);
                                        node.set_author_id(format!("handpick.result.{}:{}", row.item.key.provider.as_str(), row.item.key.result.as_str()));
                                        node.set_selected(selected);
                                    });
                                    if response.clicked() { picked = Some(hit.clone()); }
                                    response.context_menu(|ui| {
                                        if ui.add_enabled(supported && !frame.composing, egui::Button::new("Open")).clicked() {
                                            picked = Some(hit.clone());
                                            ui.close();
                                        }
                                        if let Some(location) = &row.location { ui.small(location); }
                                        if !hit.excerpt.is_empty() { ui.label(&hit.excerpt); }
                                    });
                                    if let Some(location) = &row.location { ui.small(location); }
                                    if let Some(detail) = row.item.detail { ui.small(detail); }
                                }
                            }
                            if group.has_more {
                                let more = ui.button(format!("Show more {label}"));
                                ctx.accesskit_node_builder(more.id, |node| node.set_author_id(format!("handpick.more.{label}")));
                                if more.clicked() { host.picker.show_more(group.category); }
                            }
                        }
                        if !host.rows.is_empty() && !host.exhausted && host.task.is_none() {
                            let next = ui.button("Load next graph page");
                            ctx.accesskit_node_builder(next.id, |node| node.set_author_id("handpick.next-page"));
                            if next.clicked() { host.request(ctx, runtime, &frame.literal); }
                            ui.small("Offset pages have no snapshot revision. Concurrent changes can require a new search.");
                        }
                    });
            })).response;
        if ctx.input(|input| input.pointer.any_pressed()) && !popup.contains_pointer()
            && !frame.anchor.contains(ctx.input(|input| input.pointer.interact_pos().unwrap_or(egui::Pos2::ZERO))) {
            host.open = false;
            host.lookup_from_notes = false;
        }
    }
    if !host.account.as_ref().is_some_and(|account| account.is_active()) { picked = None; }
    if picked.is_some() || host.command_action.is_some() { host.open = false; }
    ctx.accesskit_node_builder(input_id, |node| node.set_expanded(host.open));
    host.fence_legacy_intent(&mut frame);
    (frame, picked)
}

fn results_height(ctx: &egui::Context, anchor_y: f32, margin: f32) -> Option<f32> {
    let remaining = ctx.content_rect().bottom() - anchor_y - margin;
    (remaining >= ctx.style().spacing.interact_size.y).then(|| remaining.min(480.0))
}

fn header_key(hit: &Hit) -> Result<handpick::ResultKey, String> {
    Ok(handpick::ResultKey {
        provider: handpick::ProviderId::new(hit.source_kind.clone()).map_err(str::to_owned)?,
        result: handpick::ResultId::new(hit.ref_id.clone()).map_err(str::to_owned)?,
    })
}
fn header_item(hit: &Hit) -> Result<handpick::SearchItem, String> {
    Ok(handpick::SearchItem { key: header_key(hit)?, label: hit.title.clone(),
        detail: (!hit.excerpt.is_empty()).then(|| hit.excerpt.clone()),
        actions: if crate::quick_switcher::resolve_open_target(hit).enabled() {
            vec![handpick::Action { id: handpick::ActionId::new("open").map_err(str::to_owned)?,
                kind: handpick::ActionKind::Open }]
        } else { Vec::new() } })
}
fn header_location(hit: &Hit) -> Option<String> {
    ["rich_document_id", "document_id", "page_slug", "work_packet_id", "entity_key"].into_iter()
        .find_map(|key| hit.metadata.get(key).and_then(serde_json::Value::as_str)
            .map(str::trim).filter(|value| !value.is_empty())
            .map(|value| value.chars().take(256).collect()))
}
fn has_legacy_filters(literal: &str) -> bool {
    split_query_tokens(literal).iter().any(|token| split_operator(token).is_some_and(|(name, _)|
        matches!(name.to_ascii_lowercase().as_str(), "tag" | "mention" | "path" | "folder" | "kind")))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeaderSourceKind { File, TagHub, LoomBlock, UserManualPage }
impl HeaderSourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::File => "file", Self::TagHub => "tag_hub",
            Self::LoomBlock => "loom_block", Self::UserManualPage => "user_manual_page",
        }
    }
}

struct HeaderGraphQuery {
    intent: RailQuery,
    source_kind: Option<HeaderSourceKind>,
}
impl HeaderGraphQuery {
    fn params(&self, offset: usize) -> Vec<(&'static str, String)> {
        let mut params = vec![("q", self.intent.free_text.clone()), ("limit", "50".into()), ("offset", offset.to_string())];
        if let Some(source_kind) = self.source_kind { params.push(("source_kinds", source_kind.as_str().into())); }
        if let Some(content_type) = self.intent.resolved_content_type() { params.push(("content_type", content_type)); }
        if !self.intent.tag_ids.is_empty() { params.push(("tag_ids", self.intent.tag_ids.join(","))); }
        if let Some(path) = &self.intent.path { params.push(("path", path.clone())); }
        params
    }
}

fn graph_source_kind(result_type: Option<&str>) -> Option<HeaderSourceKind> {
    match result_type {
        Some("file") => Some(HeaderSourceKind::File),
        Some("tag") => Some(HeaderSourceKind::TagHub),
        Some("block") => Some(HeaderSourceKind::LoomBlock),
        Some("manual") => Some(HeaderSourceKind::UserManualPage),
        _ => None,
    }
}
fn header_category(hit: &Hit) -> handpick::ResultCategory {
    match hit.source_kind.as_str() {
        "file" => handpick::ResultCategory::Files,
        "tag_hub" => handpick::ResultCategory::Tags,
        "document" | "loom_block" => handpick::ResultCategory::Notes,
        _ => handpick::ResultCategory::Actions,
    }
}
fn category_label(category: handpick::ResultCategory) -> &'static str {
    match category {
        handpick::ResultCategory::Notes => "Notes", handpick::ResultCategory::Files => "Files",
        handpick::ResultCategory::Folders => "Folders", handpick::ResultCategory::Tags => "Tags",
        handpick::ResultCategory::Actions => "Actions", handpick::ResultCategory::Tasks => "Tasks",
        handpick::ResultCategory::Feeds => "Feeds",
    }
}

async fn fetch_header_page(base: &str, workspace: &str,
    account: Arc<crate::local_account::AuthenticatedContext>, query: &HeaderGraphQuery, offset: usize)
    -> Result<Vec<Hit>, HeaderFetchError> {
    let mut url = reqwest::Url::parse(base).map_err(|_| "Invalid backend address".to_owned())?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| "Invalid backend address".to_owned())?;
        segments.pop_if_empty().push("workspaces").push(workspace).push("loom").push("graph-search");
    }
    let params = query.params(offset);
    let client = crate::backend_client::shared_http_client();
    let response = crate::local_account::AuthenticatedRequest::new(client.clone(), Some(account.clone()),
        client.get(url).query(&params).timeout(std::time::Duration::from_secs(5)))
        .send().await.map_err(|_| "Search connection failed; query retained".to_owned())?;
    if matches!(response.status().as_u16(), 401 | 403) { return Err(HeaderFetchError::Denied); }
    if !response.status().is_success() {
        return Err(format!("Search failed (HTTP {}); query retained", response.status().as_u16()).into());
    }
    let rows: Vec<Hit> = response.json().await.map_err(|_| "Invalid search response; query retained".to_owned())?;
    if !account.is_active() { return Err(HeaderFetchError::Retired); }
    if rows.len() > 50 { return Err("Search page exceeds the requested limit".into()); }
    Ok(rows)
}


#[cfg(test)]
mod handpick_boundary_tests {
    use super::*;

    fn rail_input_frame(ctx: &egui::Context, events: Vec<egui::Event>, owner_composing: bool) -> RailFrame {
        let mut frame = None;
        let _ = ctx.run(egui::RawInput { focused: true, events, screen_rect: Some(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 480.0))),
            ..Default::default() }, |ctx| {
            frame = Some(egui::CentralPanel::default().show(ctx, |ui| {
                show_with_composition(ui, RailView { has_workspace: true, colors: RailVisuals {
                    active_pill_bg: egui::Color32::BLUE, inactive_pill_bg: egui::Color32::GRAY,
                    text: egui::Color32::WHITE, accent: egui::Color32::LIGHT_BLUE,
                } }, owner_composing)
            }).inner);
        });
        frame.unwrap()
    }

    fn click_rail_action(ctx: &egui::Context, id: egui::Id, owner_composing: bool) -> RailFrame {
        rail_input_frame(ctx, Vec::new(), owner_composing);
        let pos = ctx.read_response(id).expect("the actual rail action is mounted").rect.center();
        for pressed in [true, false] {
            let frame = rail_input_frame(ctx, vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {
                pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE,
            }], owner_composing);
            if !pressed { return frame; }
        }
        unreachable!()
    }

    #[test]
    fn search_button_preserves_all_nine_selected_scopes_and_never_opens_a_selected_result() {
        for &scope in SearchScope::all() {
            let ctx = egui::Context::default();
            ctx.data_mut(|data| data.insert_temp(state_id(), RailUiState {
                query: "needle tag:tag-1".into(), active_scope: scope, ..Default::default()
            }));
            let frame = click_rail_action(&ctx, egui::Id::new("handpick.search"), false);
            assert!(!frame.select_current, "explicit Search must refresh, not activate a prior selection");
            assert_eq!(frame.literal, "needle tag:tag-1");
            assert_eq!(frame.query.scope, scope);
            assert_eq!(frame.query.tag_ids, vec!["tag-1"]);
            assert!(matches!(&frame.outcome, RailOutcome::Fire(query) if query.scope == scope));

            let base = "http://127.0.0.1:9911";
            let mut host = HandpickHost::default();
            host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
            host.replace_input(&frame);
            assert_eq!(host.admitted_query(&frame.literal).unwrap().parsed.scope, scope);
            assert!(host.task.is_none(), "intent admission does not start a provider");
        }
    }

    #[test]
    fn generic_search_honors_typed_scope_after_the_one_click_loom_override() {
        let ctx = egui::Context::default();
        ctx.data_mut(|data| data.insert_temp(state_id(), RailUiState {
            query: "file:needle tag:tag-1".into(), active_scope: SearchScope::Pane, ..Default::default()
        }));
        let loom_id = unsafe { egui::Id::from_high_entropy_bits(RAIL_LOOM_NODE_ID) };
        let loom = click_rail_action(&ctx, loom_id, false);
        assert_eq!(loom.literal, "file:needle tag:tag-1");
        assert_eq!(loom.query.scope, SearchScope::Project);
        assert!(matches!(&loom.outcome, RailOutcome::Fire(query) if query.scope == SearchScope::Project));
        assert_eq!(rail_input_frame(&ctx, Vec::new(), false).query.scope, SearchScope::Project);

        let base = "http://127.0.0.1:9911";
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
        host.replace_input(&loom);
        assert_eq!(host.admitted_query(&loom.literal).unwrap().parsed.scope, SearchScope::Project);
        ctx.data_mut(|data| data.insert_temp(state_id(), RailUiState {
            query: loom.literal.clone(), active_scope: SearchScope::Project, project_shortcut: true,
            ..Default::default()
        }));
        let search = click_rail_action(&ctx, egui::Id::new("handpick.search"), false);
        assert_eq!(search.literal, loom.literal);
        assert_eq!(search.query.scope, SearchScope::File);
        assert!(search.input_changed);
        assert!(!search.select_current);
        host.replace_input(&search);
        assert_eq!(host.admitted_query(&search.literal).unwrap().parsed.scope, SearchScope::File);
    }

    #[test]
    fn owner_ime_fences_search_and_loom_before_scope_or_admission_changes() {
        for id in [egui::Id::new("handpick.search"), unsafe {
            egui::Id::from_high_entropy_bits(RAIL_LOOM_NODE_ID) } ] {
            let ctx = egui::Context::default();
            ctx.data_mut(|data| data.insert_temp(state_id(), RailUiState {
                query: "needle".into(), active_scope: SearchScope::File, ..Default::default()
            }));
            let frame = click_rail_action(&ctx, id, true);
            assert!(frame.composing);
            assert_eq!(frame.outcome, RailOutcome::None);
            assert_eq!(frame.query.scope, SearchScope::File);
            assert!(!frame.input_changed);
        }
    }

    #[test]
    fn mounted_header_reserves_input_between_actions_at_native_minimum_and_normal_widths() {
        use crate::editor_pane_factories::{EditorSessionContext, RichEditorPaneMount, RichPaneEvents};
        use crate::rich_editor::{document_model::node::BlockNode, renderer::rich_editor_widget::RichEditorState};
        for width in [640.0, 1280.0] {
            let ctx = egui::Context::default();
            let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(Vec::new()))));
            let mount = RichEditorPaneMount::new(state, Arc::new(Mutex::new(EditorSessionContext::default())),
                RichPaneEvents::default(), "unbound-component-note");
            let store = mount.document_store();
            let base = "http://127.0.0.1:9911";
            let account = crate::local_account::mock_account_context(base);
            let mut host = HandpickHost::default();
            let mut input = egui::Rect::NOTHING;
            let _ = ctx.run(egui::RawInput { focused: true, screen_rect: Some(
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 480.0))),
                ..Default::default() }, |ctx| {
                input = show_header(ctx, RailView { has_workspace: true, colors: RailVisuals {
                    active_pill_bg: egui::Color32::BLUE, inactive_pill_bg: egui::Color32::GRAY,
                    text: egui::Color32::WHITE, accent: egui::Color32::LIGHT_BLUE,
                } }, &mut host, "workspace", Some(account.clone()), None, base, false,
                    None, &store, &mount).0.anchor;
            });
            let clear = ctx.read_response(unsafe { egui::Id::from_high_entropy_bits(RAIL_CLEAR_NODE_ID) }).unwrap().rect;
            let loom = ctx.read_response(unsafe { egui::Id::from_high_entropy_bits(RAIL_LOOM_NODE_ID) }).unwrap().rect;
            let search = ctx.read_response(egui::Id::new("handpick.search")).unwrap().rect;
            assert!(input.width() >= 64.0 && input.left() >= 0.0);
            assert!(input.right() <= clear.left() && clear.right() <= loom.left());
            assert!(loom.right() <= search.left() && search.right() <= width);
        }
    }

    #[test]
    fn short_viewport_withholds_results_and_zero_minimum_stays_within_scroll_budget() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput { focused: true, screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO, egui::vec2(240.0, 100.0))), ..Default::default() }, |ctx| {
            assert_eq!(results_height(ctx, ctx.content_rect().bottom(), 0.0), None);
            assert_eq!(results_height(ctx, ctx.content_rect().bottom() - 1.0, 0.0), None);
            let row = ctx.style().spacing.interact_size.y;
            assert_eq!(results_height(ctx, ctx.content_rect().bottom() - row, 0.0), Some(row));
            egui::Area::new(egui::Id::new("short-results-test")).fixed_pos(egui::Pos2::ZERO)
                .constrain(false).show(ctx, |ui| {
                    ui.set_width(200.0);
                    let scroll = egui::ScrollArea::vertical().min_scrolled_height(0.0).max_height(8.0)
                        .show(ui, |ui| { for _ in 0..100 { ui.label("Actual overflowing result row"); } });
                    assert!(scroll.inner_rect.height() <= 8.0, "the default64px floor must not defeat the viewport budget");
                });
        });
    }

    #[test]
    fn command_terms_are_anded_and_quoted_terms_remain_phrases() {
        let ctx = egui::Context::default();
        let base = "http://127.0.0.1:9911";
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
        host.replace_input(&frame("> open settings".into()));
        host.request(&ctx, None, "> open settings");
        assert!(host.error.is_none());
        assert!(host.commands.iter().any(|command| command.id == "settings.open"));
        assert!(host.commands.iter().all(|command|
            crate::command_registry::matches_query(command, "open")
                && crate::command_registry::matches_query(command, "settings")));

        host.replace_input(&frame("> \"open settings\"".into()));
        host.request(&ctx, None, "> \"open settings\"");
        assert!(host.error.is_none());
        assert_eq!(host.commands.iter().map(|command| command.id).collect::<Vec<_>>(),
            crate::command_registry::filtered_commands("open settings").iter().map(|command| command.id).collect::<Vec<_>>());
    }

    #[test]
    fn command_modes_never_discard_legacy_facets() {
        let ctx = egui::Context::default();
        let base = "http://127.0.0.1:9911";
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
        for literal in ["> save tag:t1", "> save mention:m1", "> save path:src", "> save folder:src",
            "> save kind:note", "> save kind:bogus", "type:settings tag:t1", "type:settings path:src",
            "> save tag:", "> save mention:", "> save path:", "> save kind:",
            "> save in:title", "> save is:todo", "> save type:file", "file: > save"] {
            host.replace_input(&frame(literal.into()));
            host.request(&ctx, None, literal);
            assert!(host.error.is_some(), "{literal}");
            assert!(host.commands.is_empty(), "{literal}");
            assert!(host.picker.view().groups.is_empty(), "{literal}");
            assert!(host.take_command_action().is_none(), "{literal}");
            assert!(host.task.is_none(), "{literal}");
            assert_eq!(host.session.as_ref().unwrap().draft(), literal);
        }
    }

    #[test]
    fn graph_type_is_a_wire_filter_and_quoted_operators_stay_literal() {
        let query = HeaderGraphQuery {
            intent: parse_rail_query_text("\"kind:wiki_page\"", SearchScope::Project, true),
            source_kind: graph_source_kind(Some("file")),
        };
        let request = reqwest::Client::new().get("http://127.0.0.1:9911/workspaces/workspace/loom/graph-search")
            .query(&query.params(50)).build().unwrap();
        let params: std::collections::HashMap<_, _> = request.url().query_pairs().into_owned().collect();
        assert_eq!(params.get("source_kinds").map(String::as_str), Some("file"));
        assert_eq!(params.get("q").map(String::as_str), Some("\"kind:wiki_page\""));
        assert_eq!(params.get("offset").map(String::as_str), Some("50"));
        assert!(!params.contains_key("content_type"));
    }

    #[test]
    fn graph_phrase_is_retained_and_declined_until_canonical_provider_supports_it() {
        let ctx = egui::Context::default();
        let base = "http://127.0.0.1:9911";
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
        let literal = "type:file \"alpha kind:wiki_page\"";
        host.replace_input(&frame(literal.into()));
        let admitted = &host.query.as_ref().unwrap().parsed;
        assert!(admitted.kind_filters.is_empty());
        assert_eq!(admitted.free_text, literal);
        host.request(&ctx, None, literal);
        assert_eq!(host.error.as_deref(), Some("Exact phrase search needs its graph provider; query retained"));
        assert!(host.task.is_none());
        assert_eq!(host.session.as_ref().unwrap().draft(), literal);
    }

    #[test]
    fn settings_query_routes_catalog_action_and_an_edit_retires_it() {
        let ctx = egui::Context::default();
        let base = "http://127.0.0.1:9911";
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
        host.replace_input(&frame("type:settings".into()));
        host.request(&ctx, None, "type:settings");
        assert!(host.error.is_none());
        assert!(host.task.is_none(), "local commands do not require a graph request");
        let group = host.picker.view().groups.into_iter().find(|group|
            group.category == handpick::ResultCategory::Actions).unwrap();
        assert_eq!(group.status, handpick::DeliveryStatus::Complete);
        assert_eq!(group.total, Some(group.loaded));
        assert!(!group.items.is_empty());
        assert!(group.items.iter().all(|row| row.item.key.result.as_str().starts_with("settings.")));
        let key = group.items[0].item.key.clone();
        assert_eq!(group.items[0].item.actions[0].kind, handpick::ActionKind::RevealSetting);
        assert!(host.activate_command(&key));
        assert_eq!(host.take_command_action(), Some(key.result.as_str()));
        host.replace_input(&frame("another query".into()));
        assert!(!host.activate_command(&key));
        assert!(host.take_command_action().is_none());
        assert!(host.picker.view().groups.is_empty());
    }

    #[test]
    fn unavailable_editor_command_is_visible_but_never_dispatched() {
        let ctx = egui::Context::default();
        let base = "http://127.0.0.1:9911";
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
        host.set_command_context(crate::command_registry::EditorMenuEnableContext::unavailable());
        host.replace_input(&frame("> save".into()));
        host.request(&ctx, None, "> save");
        assert!(host.error.is_none());
        let command = host.commands.iter().copied().find(|command|
            command.id == crate::command_registry::CMD_EDITOR_FILE_SAVE).unwrap();
        let key = handpick::ResultKey {
            provider: handpick::ProviderId::new("native.commands").unwrap(),
            result: handpick::ResultId::new(command.id).unwrap(),
        };
        assert!(!host.command_enabled(command));
        assert!(!host.activate_command(&key));
        assert!(host.take_command_action().is_none());
    }

    fn frame(literal: String) -> RailFrame {
        RailFrame {
            query: parse_rail_query(&literal, SearchScope::Project),
            literal,
            input_changed: true,
            anchor: egui::Rect::NOTHING,
            focused: false,
            select_current: false,
            composing: false,
            outcome: RailOutcome::None,
        }
    }

    fn hit(id: &str) -> Hit {
        Hit {
            result_kind: "loom_block".into(),
            source_kind: "loom_block".into(),
            ref_id: id.into(),
            title: "Authorized row".into(),
            excerpt: String::new(),
            block: serde_json::Value::Null,
            score: 1.0,
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn rejected_literal_blocks_enter_retry_and_recovers_on_valid_edit() {
        // The current-thread runtime is never driven: a mistakenly spawned HTTP task is observable
        // through the real dispatch handle without polling network work in this boundary test.
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let ctx = egui::Context::default();
        let base = "http://127.0.0.1:9911";
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
        host.replace_input(&frame("x".into()));
        host.request(&ctx, Some(runtime.handle()), "x");
        assert!(host.task.is_some(), "valid input reaches the actual dispatch boundary");

        let rejected = format!("x{}", " ".repeat(65536));
        let rejected_frame = frame(rejected.clone());
        assert_eq!(rejected_frame.query.free_text, "x", "the compacted query alone cannot prove literal admission");
        host.replace_input(&rejected_frame);
        assert!(host.task.is_none());
        assert!(host.query.is_none());
        assert!(host.rows.is_empty());
        assert!(host.due.is_none());
        assert_eq!(host.session.as_ref().unwrap().draft(), "x", "core rejection retains its previous literal");
        let rejected_fence = host.session.as_ref().unwrap().fence();

        host.restart(&ctx, Some(runtime.handle()), &rejected); // Enter/Loom path.
        assert!(host.task.is_none(), "Enter cannot dispatch rejected input under the old draft");
        host.request(&ctx, Some(runtime.handle()), &rejected); // Retry/page path.
        assert!(host.task.is_none(), "Retry cannot bypass literal admission");
        assert_eq!(host.session.as_ref().unwrap().fence(), rejected_fence);
        let mut rejected_intent = frame(rejected.clone());
        rejected_intent.outcome = RailOutcome::Fire(Box::new(rejected_intent.query.clone()));
        host.fence_legacy_intent(&mut rejected_intent);
        assert!(matches!(rejected_intent.outcome, RailOutcome::None), "legacy consumers cannot dispatch rejected input either");

        let mut unchanged = frame(rejected);
        unchanged.input_changed = false;
        host.replace_input(&unchanged);
        assert!(host.query.is_none());
        assert_eq!(host.session.as_ref().unwrap().fence(), rejected_fence, "a rejected unchanged frame does not retry admission");

        host.replace_input(&frame("recovered".into()));
        assert_eq!(host.session.as_ref().unwrap().draft(), "recovered");
        host.request(&ctx, Some(runtime.handle()), "recovered");
        assert!(host.task.is_some(), "a genuine valid edit can dispatch normally");
        let mut valid_intent = frame("recovered".into());
        valid_intent.outcome = RailOutcome::Fire(Box::new(valid_intent.query.clone()));
        host.fence_legacy_intent(&mut valid_intent);
        assert!(matches!(valid_intent.outcome, RailOutcome::Fire(_)));
    }

    #[test]
    fn dispatch_requires_exact_core_literal_and_admitted_revision() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let ctx = egui::Context::default();
        let base = "http://127.0.0.1:9911";
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(crate::local_account::mock_account_context(base)), base);
        host.replace_input(&frame("x".into()));
        let fence = host.session.as_ref().unwrap().fence();

        host.request(&ctx, Some(runtime.handle()), &format!("x{}", " ".repeat(65536)));
        assert!(host.task.is_none(), "removing the exact-literal guard would spawn this compactable request");
        assert_eq!(host.session.as_ref().unwrap().fence(), fence);

        host.session.as_mut().unwrap().set_draft("x".into()).unwrap();
        assert_ne!(host.query.as_ref().unwrap().draft_revision, host.session.as_ref().unwrap().draft_revision());
        host.request(&ctx, Some(runtime.handle()), "x");
        assert!(host.task.is_none(), "matching text with a replaced core revision is not the admitted query");

        host.replace_input(&frame("x".into()));
        host.request(&ctx, Some(runtime.handle()), "x");
        assert!(host.task.is_some());
    }

    #[test]
    fn backend_rebind_retires_private_state_and_rejects_old_delivery_identity() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let ctx = egui::Context::default();
        let base = "http://127.0.0.1:9911";
        let rebound = "http://127.0.0.1:9911/rebound";
        let account = crate::local_account::mock_account_context(base);
        let mut host = HandpickHost::default();
        host.bind(&ctx, "workspace", Some(account.clone()), base);
        host.replace_input(&frame("x".into()));
        let old_binding = host.binding.clone();
        let old_delivery = host.delivery.clone();
        let first_fence = host.session.as_ref().unwrap().fence();
        *host.delivery.lock().unwrap() = Some((old_binding.clone(), first_fence, Ok(vec![hit("old")])));
        host.drain();
        assert_eq!(host.rows.len(), 1);
        host.request(&ctx, Some(runtime.handle()), "x");
        assert!(host.task.is_some(), "the backend rebind must also retire a real pending dispatch");
        let old_fence = host.session.as_ref().unwrap().fence();

        ctx.data_mut(|data| data.insert_temp(state_id(), RailUiState {
            query: "private old input".into(), ..RailUiState::default()
        }));
        host.bind(&ctx, "workspace", Some(account.clone()), rebound);
        assert!(host.binding_changed);
        assert_eq!(host.backend, rebound);
        assert!(!Arc::ptr_eq(&host.binding, &old_binding));
        assert!(!Arc::ptr_eq(&host.delivery, &old_delivery));
        assert!(host.rows.is_empty());
        assert!(host.picker.view().groups.is_empty());
        assert!(host.query.is_none());
        assert!(host.due.is_none());
        assert!(host.task.is_none());
        assert!(host.session.as_ref().unwrap().draft().is_empty());
        assert!(ctx.data_mut(|data| data.get_temp::<RailUiState>(state_id())).is_none());

        host.replace_input(&frame("x".into()));
        host.request(&ctx, Some(runtime.handle()), "x");
        let fresh_fence = host.session.as_ref().unwrap().fence();
        assert_eq!(old_fence, fresh_fence, "same account/workspace/literal can recreate the shared-core fence");
        *old_delivery.lock().unwrap() = Some((old_binding.clone(), old_fence.clone(), Ok(vec![hit("late-isolated")])));
        host.drain();
        assert!(host.rows.is_empty(), "the retired response slot is disconnected");
        *host.delivery.lock().unwrap() = Some((old_binding, old_fence, Ok(vec![hit("stale-origin")])));
        host.drain();
        assert!(host.rows.is_empty(), "removing the binding-token check would admit the old backend's row");
        assert!(host.task.is_some(), "a rejected old delivery cannot clear the current pending dispatch");
        host.task.take().unwrap().abort();

        *host.delivery.lock().unwrap() = Some((host.binding.clone(), fresh_fence, Ok(vec![hit("current-origin")])));
        host.drain();
        assert_eq!(host.rows[0].ref_id, "current-origin");
        host.bind(&ctx, "workspace", Some(account), rebound);
        assert!(!host.binding_changed, "unchanged backend binding preserves the current result");
        assert_eq!(host.rows[0].ref_id, "current-origin");
    }
}
