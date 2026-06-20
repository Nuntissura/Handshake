//! Bottom search rail for the native Handshake shell (WP-KERNEL-011 MT-022).
//!
//! ## What this provides (no-context model navigation — HBR-VIS / HBR-SWARM)
//!
//! A persistent, always-visible horizontal strip pinned to the BOTTOM of the shell window (a fixed
//! [`egui::TopBottomPanel::bottom`], 32px high, registered before the central panel). It is the shell's
//! one-click search-anywhere surface: a scope-selector pill strip, a free-text query input, a clear
//! (`x`) button, and a `Loom` shortcut that forces the `project:` scope. Its sole job is query input +
//! scope selection.
//!
//! ### The rail EMITS a search INTENT — it does NOT execute a search (AC-022-9)
//!
//! On an explicit fire (Enter OR the Loom button) the rail PARSES the input into a [`RailQuery`]
//! (`{ free_text, scope, tag_ids, path, kind_filters }`) and EMITS that intent — it makes NO HTTP or
//! IPC backend call and renders NO results. Search EXECUTION and results display are deliberately
//! deferred to a downstream consumer (a future search-results surface / pane overlay), exactly as the
//! contract defers (`binds_backend_api: None`, implementation_notes "Do NOT make backend search calls
//! inside this MT", "Backend wiring ... belongs to the pane or overlay that consumes the rail's
//! output"). The downstream consumer will call `POST /workspaces/:workspace_id/loom/search-v2` and/or
//! `GET /workspaces/:workspace_id/loom/graph-search`; this rail never does.
//!
//! ## Emitted-intent slot (HBR-SWARM — observable out-of-process)
//!
//! The emitted [`RailQuery`] is written into a lock-guarded shared slot on the app
//! (`app_state.search_rail_query: Arc<Mutex<Option<RailQuery>>>`, the alias [`RailQuerySlot`]). The
//! fire path WRITES the slot; a downstream consumer / concurrent swarm thread CLONES-and-READS it off
//! the same lock (the writer holds the lock only briefly, never across egui calls — no deadlock with a
//! concurrent reader). The `x` clear button writes `None` back into the slot (AC-022-6).
//!
//! ## Stable AccessKit ids (out-of-process steering — HBR-VIS)
//!
//! Three FIXED container/control nodes in the fresh 22..=24 band (disjoint from every other declared
//! identity, all `< PANE_NODE_ID_BASE`):
//! - the query input ([`RAIL_INPUT_NODE_ID`] = 22, Role::TextInput),
//! - the clear button ([`RAIL_CLEAR_NODE_ID`] = 23, Role::Button),
//! - the Loom shortcut ([`RAIL_LOOM_NODE_ID`] = 24, Role::Button).
//!
//! The nine scope pills are a FIXED-COUNT set but addressed by stable author_id STRINGS
//! (`bottom-rail.scope.{name}`, Role::Button) in egui's hashed id space, the same dynamic-author_id
//! pattern the MT-007 tab rows / MT-012-adjacent author_id-derived nodes use, so they are
//! discoverable/clickable out-of-process and pass the MT-025 interactive-naming gate without consuming a
//! fixed band. The three fixed control ids ARE enumerated in
//! [`crate::accessibility::DECLARED_IDENTITIES`].

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

/// The fixed pixel height of the bottom rail strip (the contract's 32px always-visible strip).
pub const RAIL_HEIGHT: f32 = 32.0;

// ===========================================================================
// Scope enum (the nine fixed rail scopes).
// ===========================================================================

/// The nine fixed search scopes the rail's pill strip exposes (the contract's
/// `project: file: pane: window: stash: trace: terminal: stage: layout:`). The scope narrows WHAT a
/// query searches; the active scope is set by clicking a pill OR by typing its colon-prefix into the
/// input (`project:foo`). The scope is part of the emitted [`RailQuery`] intent — a downstream consumer
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
    /// execute the search). A scope with no direct `content_type` (the cross-surface scopes —
    /// pane/window/trace/terminal/stage/layout, plus stash which has no distinct content_type) returns
    /// `None` and the consumer searches the whole graph with the typed query. `Project` is whole-project
    /// (no content_type filter). This is the rail's HONEST scope->facet mapping carried on the intent; it
    /// is NOT a backend request — the rail makes no backend call.
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
/// pane/overlay consumes (the contract's `RailQuery`). The rail only PRODUCES this struct — it does not
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
    /// — see [`effective_query`](Self::effective_query)).
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
    /// pure helper over the parsed intent — no backend call.
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
    /// helper over the parsed intent — no backend call.
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
        "note"
            | "file"
            | "annotated_file"
            | "tag_hub"
            | "journal"
            | "canvas"
            | "view_def"
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
/// (typed prefix wins — AC-022-4). Never panics on malformed input (PROOF-022-1e).
pub fn parse_rail_query(raw: &str, scope: SearchScope) -> RailQuery {
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
                    let v = value.trim();
                    if !v.is_empty() {
                        free_parts.push(v.to_owned());
                    }
                    continue;
                }
                match op_lower.as_str() {
                    "tag" | "mention" => {
                        for v in split_operator_values(&value) {
                            out.tag_ids.push(v);
                        }
                    }
                    "path" | "folder" => {
                        let v = value.trim();
                        if !v.is_empty() {
                            out.path = Some(v.to_owned());
                        }
                    }
                    "kind" => {
                        for v in split_operator_values(&value) {
                            if !is_known_content_type(&v) {
                                out.kind_errors.push(v.clone());
                            }
                            out.kind_filters.push(v);
                        }
                    }
                    // An unrecognized operator (e.g. `foo:bar`) is NOT a known facet; keep the whole
                    // token as free-text (React parity: unknown operators fall through to the query).
                    _ => free_parts.push(token.clone()),
                }
            }
            // A plain token (no `name:` operator shape) is free-text.
            None => free_parts.push(token.clone()),
        }
    }

    out.free_text = free_parts.join(" ");
    out
}

/// Split a raw query into tokens on whitespace, honoring double-quoted runs (port of the React
/// `splitQueryTokens` state machine). A `"`-quoted span is one token with the quotes removed;
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
    /// No actionable change this frame (typing is reflected in the returned query).
    None,
    /// The user pressed Enter or clicked Loom: EMIT this parsed query as the search intent (the shell
    /// writes it into the [`RailQuerySlot`]). The rail makes no backend call. The Loom shortcut forces
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
    /// The pill-selected scope (overridden by a typed colon-prefix at parse time). Defaults to
    /// [`SearchScope::Project`] (the rail's default scope).
    active_scope: SearchScope,
}

/// The egui-memory key for the rail's transient UI state.
fn state_id() -> egui::Id {
    egui::Id::new("bottom-rail.state")
}

/// Render the bottom search rail INSIDE an already-opened bottom panel `ui` (the shell registers the
/// `TopBottomPanel::bottom` so the panel claims its space before the central panel — see the module
/// docs). Returns the [`RailFrame`] for this frame.
///
/// Layout (left -> right): the nine scope pills, the free-text input (filling the remaining width), the
/// `x` clear button, and the `Loom` shortcut. The rail renders NO results — it only emits an intent
/// (AC-022-9). The strip height never changes (AC-022-1).
pub fn show(ui: &mut egui::Ui, view: RailView) -> RailFrame {
    let mut state: RailUiState = ui
        .ctx()
        .data_mut(|d| d.get_temp::<RailUiState>(state_id()))
        .unwrap_or_default();

    let mut outcome = RailOutcome::None;
    let mut fire = false;
    let mut clear = false;
    let mut loom = false;

    let input_egui_id = unsafe { egui::Id::from_high_entropy_bits(RAIL_INPUT_NODE_ID) };
    let clear_egui_id = unsafe { egui::Id::from_high_entropy_bits(RAIL_CLEAR_NODE_ID) };
    let loom_egui_id = unsafe { egui::Id::from_high_entropy_bits(RAIL_LOOM_NODE_ID) };

    ui.horizontal_centered(|ui| {
        // ── Scope pills (left), then the input + clear + Loom (right). ──
        // The pills render left-to-right; the input + buttons live in the right_to_left group below,
        // which egui lays out FIRST (reserving their width from the right edge), so the free-text input
        // is ALWAYS visible even when the nine pills are wide on a narrow window (RISK-022-B). A
        // horizontal egui ScrollArea was intentionally NOT used here: its viewport acquires an anonymous
        // (role Unknown, Action::Click) AccessKit node that trips the MT-025 interactive-naming gate;
        // the reserve-input-first layout meets the same "input never clipped" goal without that node.
        for &scope in SearchScope::all() {
            let selected = state.active_scope == scope;
            let resp = scope_pill(ui, scope, selected, &view.colors);
            if resp.clicked() {
                // Clicking a pill sets the scope; it does NOT fire a search (AC-022-3).
                state.active_scope = scope;
            }
        }

        ui.separator();

        // ── The Loom shortcut + clear button, right-aligned so the input fills the middle. ──
        // Laid out right-to-left so the input's `desired_width(INFINITY)` fills the gap to the left of
        // these fixed-size buttons (egui right_to_left reserves them first — so the input is ALWAYS
        // visible regardless of how many pills render to the left — RISK-022-B).
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                loom = true;
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

            // ── The free-text input fills the remaining middle width at its FIXED id (NodeId 22). ──
            let edit = egui::TextEdit::singleline(&mut state.query)
                .id(input_egui_id)
                .hint_text("Search…  (try project:foo, tag:x, kind:note)")
                .desired_width(f32::INFINITY);
            let edit_response = ui.add(edit);
            emit_interactive_node(ui.ctx(), input_egui_id, RAIL_INPUT_AUTHOR_ID);
            // Enter emits the search intent (AC-022-3: only Enter or Loom fires).
            if edit_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                fire = true;
            }
        });
    });

    // ── Apply the Loom shortcut: force project: scope, then fire. ──
    if loom {
        state.active_scope = SearchScope::Project;
        fire = true;
    }

    // Parse the live query against the active (pill) scope; a typed colon-prefix overrides it.
    let parsed = parse_rail_query(&state.query, state.active_scope);

    // ── Clear: empty the input, reset the scope to default, signal the shell to clear the slot. ──
    if clear {
        state.query.clear();
        state.active_scope = SearchScope::default();
        outcome = RailOutcome::Clear;
    } else if fire {
        outcome = RailOutcome::Fire(Box::new(parsed.clone()));
    }

    // Persist the transient state.
    ui.ctx()
        .data_mut(|d| d.insert_temp(state_id(), state.clone()));

    RailFrame {
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
    let label = egui::RichText::new(scope.to_string()).color(colors.text).small();
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
    let g = ui
        .painter()
        .layout_no_wrap(glyph.to_owned(), egui::FontId::proportional(13.0), text_color);
    let size = egui::vec2(g.size().x + 12.0, (RAIL_HEIGHT - 8.0).max(16.0));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let resp = ui.interact(rect, id, egui::Sense::click()).on_hover_text(tooltip);
    if ui.is_rect_visible(rect) {
        let fill = if resp.hovered() { bg } else { egui::Color32::TRANSPARENT };
        ui.painter().rect_filled(rect, 3.0, fill);
        let g2 = ui
            .painter()
            .layout_no_wrap(glyph.to_owned(), egui::FontId::proportional(13.0), text_color);
        ui.painter().galley(
            egui::pos2(rect.center().x - g2.size().x * 0.5, rect.center().y - g2.size().y * 0.5),
            g2,
            text_color,
        );
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tooltip));
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

    // ── Fixed control ids in the fresh disjoint band ──────────────────────────────────────────────

    #[test]
    fn rail_control_ids_in_disjoint_fresh_band() {
        for id in [RAIL_INPUT_NODE_ID, RAIL_CLEAR_NODE_ID, RAIL_LOOM_NODE_ID] {
            assert!((22..=24).contains(&id), "rail control id {id} in band 22..=24");
            assert!(
                id < crate::accessibility::PANE_NODE_ID_BASE,
                "rail control id {id} below the pane id base"
            );
        }
        // The three ids are distinct.
        assert_ne!(RAIL_INPUT_NODE_ID, RAIL_CLEAR_NODE_ID);
        assert_ne!(RAIL_CLEAR_NODE_ID, RAIL_LOOM_NODE_ID);
    }

    // ── Scope enum: Display, default, all(), keyword round-trip ────────────────────────────────────

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

    // ── parse_rail_query: PROOF-022-1 (a)..(e) + AC-022-4 ─────────────────────────────────────────

    #[test]
    fn parse_free_text_no_operators() {
        // (a) plain free-text, no operators.
        let q = parse_rail_query("hello world", SearchScope::File);
        assert_eq!(q.free_text, "hello world");
        assert_eq!(q.scope, SearchScope::File, "pill scope retained when no prefix typed");
        assert!(q.tag_ids.is_empty());
        assert!(q.path.is_none());
        assert!(q.kind_filters.is_empty());
        assert!(q.kind_errors.is_empty());
    }

    #[test]
    fn parse_tag_mention_path_kind_combined() {
        // (b) tag + mention + path + kind operators combined.
        let q = parse_rail_query("foo tag:t1,t2 mention:m1 path:src/lib kind:note bar", SearchScope::Project);
        assert_eq!(q.free_text, "foo bar", "operators stripped, free-text preserved in order");
        assert_eq!(q.tag_ids, vec!["t1", "t2", "m1"], "tag + mention values collected");
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
        assert_eq!(q.scope, SearchScope::Project, "typed project: overrides the File pill");
        assert_eq!(q.free_text, "hello world", "the prefix is stripped from the free-text");
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
        assert_eq!(q.free_text, "hello world", "the quoted free-text run is one token");
        assert_eq!(q.path.as_deref(), Some("my folder"), "the quoted path keeps its space");
    }

    #[test]
    fn parse_invalid_kind_is_captured_not_panicked() {
        // (e) an invalid kind: value is captured in kind_errors without panicking.
        let q = parse_rail_query("kind:bogus thing", SearchScope::Project);
        assert_eq!(q.kind_filters, vec!["bogus"]);
        assert_eq!(q.kind_errors, vec!["bogus"], "unknown kind captured as a soft error");
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

    // ── content_type + effective_query mapping (intent-derived helpers, no backend call) ───────────

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

    // ── Emitted-intent slot: the fire path WRITES, a reader CLONES-and-READS (HBR-SWARM) ──────────

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
        let read_back = slot.lock().unwrap().clone().expect("intent present after a fire");
        assert_eq!(read_back, emitted);
        assert_eq!(read_back.scope, SearchScope::Project, "typed prefix won in the emitted intent");
        assert_eq!(read_back.free_text, "hello world");

        // CLEAR (the x button): the fire path writes None back, the reader sees an empty slot again.
        *slot.lock().unwrap() = None;
        assert!(slot.lock().unwrap().is_none(), "slot reset to None by clear");
    }

    #[test]
    fn rail_query_slot_shared_across_clones() {
        // A second Arc clone (held by a downstream consumer / swarm thread) observes a write made
        // through the first handle — the slot is genuinely shared, not copied.
        let writer: RailQuerySlot = Arc::new(Mutex::new(None));
        let reader = writer.clone();
        let emitted = parse_rail_query("tag:t1 docs", SearchScope::Project);
        writer.lock().unwrap().replace(emitted.clone());
        assert_eq!(
            reader.lock().unwrap().clone().expect("reader sees the write"),
            emitted
        );
    }
}
