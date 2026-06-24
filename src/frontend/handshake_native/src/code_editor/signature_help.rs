//! Signature help (parameter hints) for the native code editor (WP-KERNEL-012 MT-047 — E1 VS Code
//! parity).
//!
//! This is a PURE ADDITION layered on top of MT-008's LSP JSON-RPC transport ([`super::lsp_client`])
//! and the Handshake backend code-nav client ([`super::code_nav`]). While the user types a function
//! call, signature help is requested automatically on an open-paren `(` or a comma `,` trigger (or
//! manually via Ctrl+Shift+Space), and the active parameter is rendered emphasized in a small floating
//! popup anchored just ABOVE the cursor line (VS Code parity).
//!
//! ## Transport-agnostic state ([`SignatureHelpState`])
//!
//! Two sources both produce the SAME [`SignatureHelpState`] so the renderer never branches on origin:
//! - LSP `textDocument/signatureHelp` -> [`SignatureHelpState::from_lsp`] (the field-correct
//!   `lsp_types::SignatureHelp` shape: `signatures`, `active_signature`, `active_parameter`, and each
//!   `ParameterInformation.label` is a [`lsp_types::ParameterLabel`] — `Simple(String)` OR
//!   `LabelOffsets([u32; 2])`; BOTH are handled — RISK-005 / MC-005).
//! - the Handshake backend code-nav symbol under the cursor -> [`SignatureHelpState::from_code_nav`]
//!   (the FALLBACK when no language server is attached or the server returns an empty/None response).
//!   The fallback signature label is built from the REAL backend fields the code-nav client returns
//!   (`display_name` + any parenthesized parameter list it carries) — NOT an assumed `params` field,
//!   which the backend `CodeSymbolNavProjection` does not have (the verify-over-prose discipline).
//!
//! ## The active-parameter comma scanner ([`active_parameter_from_commas`])
//!
//! On the fallback path the active parameter is computed LOCALLY by counting the top-level commas
//! between the call's open-paren and the cursor. It is a DEPTH-AWARE scanner: commas nested inside
//! `()`, `[]`, `{}`, generic `<>` angle brackets, string literals, and char literals are SKIPPED
//! (RISK-001 / MC-001 / AC-007). This is the same kind of scanner used to find the active call site.
//!
//! ## Non-focus-stealing popup (RISK-003 / MC-003 / HBR-QUIET)
//!
//! The popup renders as a NON-modal [`egui::Area`] on the [`egui::Order::Tooltip`] order. It does NOT
//! request focus and does NOT consume the editor's keyboard input on the frame it opens, so the editor
//! processes keystrokes FIRST (the panel input handler runs before this is drawn) and the character
//! that triggered the popup still lands — the same class of focus bug MT-008's completion popup had to
//! avoid. AccessKit emits a `Role::Tooltip` node with `author_id = "code_editor_signature_help"` whose
//! value carries the active signature label so a no-context swarm agent reads the hint without pixels
//! (AC-005 / MC-006).

use std::ops::Range;

use egui::accesskit;

use super::code_nav::CodeSymbolNavProjection;

/// The trigger characters that initiate / retrigger a signature-help request while typing a call. The
/// open-paren OPENS the popup at a new call site; the comma UPDATES the active parameter of the popup
/// already open at that call site (keyed by the open-paren byte — see the panel). Named + documented as
/// the MT contract requires; the server's declared `triggerCharacters` override these when present.
pub const SIGNATURE_HELP_TRIGGER_CHARS: [char; 2] = ['(', ','];

/// The stable AccessKit author_id for the signature-help popup root node (a `Role::Tooltip`, mirroring
/// the MT-008 hover-tooltip convention). A swarm agent addresses the hint by this id and reads the
/// active signature label from the node value WITHOUT pixels (AC-005 / MC-006).
pub const CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID: &str = "code_editor_signature_help";

/// Fixed AccessKit/egui `NodeId` for the signature-help popup root. A fresh slot (700) ABOVE the
/// MT-008 overlay band (completion popup 600.., completion items 601..665, hover 680/681) so the
/// signature popup never collides with another overlay node id.
const SIGNATURE_HELP_NODE_ID: u64 = 700;

/// Where a [`SignatureHelpState`] came from. The renderer does not branch on this, but the panel uses
/// it to decide whether a comma should RE-REQUEST from the LSP server (it owns the active parameter) or
/// RECOMPUTE locally (the code-nav fallback computes the active parameter from commas).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureSource {
    /// The signature came from an attached language server's `textDocument/signatureHelp`.
    Lsp,
    /// The signature was synthesized from the Handshake backend code-nav symbol (the fallback).
    CodeNavFallback,
}

/// One parameter of a signature: its display text plus the byte range it occupies WITHIN the signature
/// label, so the renderer can emphasize exactly that run. `range_in_label` is resolved from the LSP
/// `LabelOffsets([start, end])` form when present, otherwise by a substring search of the simple label
/// within the signature label (left-to-right, consuming earlier matched ranges so a repeated parameter
/// name maps to a DISTINCT span — RISK-005 / MC-005). `None` when the parameter text could not be
/// located in the label (the renderer then emphasizes nothing for it rather than guessing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSpan {
    /// The parameter's display text (e.g. `a: i32`).
    pub label: String,
    /// The byte range this parameter occupies in the signature label, when locatable.
    pub range_in_label: Option<Range<usize>>,
}

/// One callable signature: the full label (e.g. `fn add(a: i32, b: i32) -> i32`), its parameters (with
/// their spans in the label), and an optional documentation string (the first line is shown).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInfo {
    /// The full signature label shown in the popup.
    pub label: String,
    /// The parameters, in order, with their spans into `label`.
    pub parameters: Vec<ParamSpan>,
    /// Optional documentation (the popup shows the first non-empty line).
    pub documentation: Option<String>,
}

impl SignatureInfo {
    /// Build a signature from a label + an ORDERED list of parameter display strings, resolving each
    /// parameter's span by a left-to-right substring search of the label (the `ParameterLabel::Simple`
    /// path and the code-nav fallback path). Earlier matches are consumed so a repeated parameter name
    /// (`fn f(x: i32, x: i32)`) maps each occurrence to a DISTINCT span (RISK-005 / MC-005).
    pub fn from_label_and_param_strings(label: impl Into<String>, params: Vec<String>) -> Self {
        let label = label.into();
        let mut parameters = Vec::with_capacity(params.len());
        // `search_from` advances past each matched run so repeated names resolve to distinct spans.
        let mut search_from = 0usize;
        for p in params {
            let range = substring_range_from(&label, &p, search_from);
            if let Some(r) = &range {
                search_from = r.end;
            }
            parameters.push(ParamSpan { label: p, range_in_label: range });
        }
        Self { label, parameters, documentation: None }
    }
}

/// The live signature-help popup state, owned by the editor pane. Present only while the popup is open.
/// `anchor_byte` is the call's open-paren byte offset — the KEY that distinguishes call sites so a
/// comma updates the open popup (same `anchor_byte`) rather than opening a second one (RISK-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpState {
    /// The available signatures (overloads). At least one when the popup is open.
    pub signatures: Vec<SignatureInfo>,
    /// The active signature index (overload selection; Up/Down cycles it). Clamped before use.
    pub active_signature: usize,
    /// The active parameter index within the active signature (the emphasized run). Clamped before use.
    pub active_parameter: usize,
    /// The call's open-paren byte offset — the call-site key (RISK-002) + the dismissal anchor.
    pub anchor_byte: usize,
    /// Where this state came from (LSP vs the code-nav fallback).
    pub source: SignatureSource,
}

impl SignatureHelpState {
    /// The currently-active signature (clamped), or `None` when there are no signatures.
    pub fn active(&self) -> Option<&SignatureInfo> {
        if self.signatures.is_empty() {
            return None;
        }
        let idx = self.active_signature.min(self.signatures.len() - 1);
        self.signatures.get(idx)
    }

    /// Cycle to the NEXT overload (Down arrow), wrapping. No-op with 0/1 signatures.
    pub fn select_next_signature(&mut self) {
        if self.signatures.len() > 1 {
            self.active_signature = (self.active_signature + 1) % self.signatures.len();
            // A new overload may have fewer parameters; clamp the active parameter.
            self.clamp_active_parameter();
        }
    }

    /// Cycle to the PREVIOUS overload (Up arrow), wrapping. No-op with 0/1 signatures.
    pub fn select_prev_signature(&mut self) {
        let n = self.signatures.len();
        if n > 1 {
            self.active_signature = (self.active_signature + n - 1) % n;
            self.clamp_active_parameter();
        }
    }

    /// Clamp `active_parameter` to the active signature's parameter count (so an overload switch / a
    /// recomputed comma count past the last parameter never indexes out of range).
    pub fn clamp_active_parameter(&mut self) {
        let count = self.active().map(|s| s.parameters.len()).unwrap_or(0);
        if count == 0 {
            self.active_parameter = 0;
        } else if self.active_parameter >= count {
            self.active_parameter = count - 1;
        }
    }

    /// Convert an `lsp_types::SignatureHelp` into the transport-agnostic state, resolving each
    /// parameter's span (BOTH `ParameterLabel` forms — MC-005) and reading the active indices from the
    /// response (`active_signature` / `active_parameter`, with the per-signature `active_parameter`
    /// override the LSP 3.16 spec allows). `anchor_byte` is the call's open-paren offset (the panel
    /// passes it; the LSP response does not carry it). Returns `None` when the server sent no signatures
    /// (the caller then falls back to the code-nav path — AC-003). NEVER panics on a malformed field
    /// (AC-008): missing/out-of-range indices clamp to 0.
    pub fn from_lsp(help: &lsp_types::SignatureHelp, anchor_byte: usize) -> Option<Self> {
        if help.signatures.is_empty() {
            return None;
        }
        let active_signature = help.active_signature.unwrap_or(0) as usize;
        let signatures: Vec<SignatureInfo> = help
            .signatures
            .iter()
            .map(signature_info_from_lsp)
            .collect();
        // Per LSP 3.16, a signature's own `active_parameter` (if present) takes precedence over the
        // top-level one for THAT signature; we read the active signature's override when set.
        let sig_idx = active_signature.min(signatures.len() - 1);
        let active_parameter = help
            .signatures
            .get(sig_idx)
            .and_then(|s| s.active_parameter)
            .or(help.active_parameter)
            .unwrap_or(0) as usize;
        let mut state = Self {
            signatures,
            active_signature,
            active_parameter,
            anchor_byte,
            source: SignatureSource::Lsp,
        };
        state.clamp_active_parameter();
        // Clamp the signature index too (a server could send active_signature past the list).
        if state.active_signature >= state.signatures.len() {
            state.active_signature = 0;
        }
        Some(state)
    }

    /// Build the FALLBACK state from a Handshake backend code-nav symbol projection (AC-003). The label
    /// and parameters are derived from the REAL backend fields (`display_name` and any parenthesized
    /// parameter list it carries) — see [`signature_from_code_nav_symbol`]. `active_parameter` is the
    /// locally-computed top-level comma count (the panel passes it). Returns `None` when the symbol
    /// yields no usable signature label (the popup then shows nothing — never a panic, AC-008).
    pub fn from_code_nav(
        symbol: &CodeSymbolNavProjection,
        anchor_byte: usize,
        active_parameter: usize,
    ) -> Option<Self> {
        let signature = signature_from_code_nav_symbol(symbol)?;
        let mut state = Self {
            signatures: vec![signature],
            active_signature: 0,
            active_parameter,
            anchor_byte,
            source: SignatureSource::CodeNavFallback,
        };
        state.clamp_active_parameter();
        Some(state)
    }
}

/// Convert one `lsp_types::SignatureInformation` to a [`SignatureInfo`], resolving each parameter's span
/// from BOTH `ParameterLabel` forms (MC-005). A `LabelOffsets([start, end])` is taken directly (clamped
/// to the label length so a server's bad offset never panics — AC-008); a `Simple(name)` is resolved by
/// a left-to-right substring search consuming earlier matches (RISK-005). Documentation flattens to its
/// string form.
fn signature_info_from_lsp(sig: &lsp_types::SignatureInformation) -> SignatureInfo {
    let label = sig.label.clone();
    let mut parameters = Vec::new();
    let mut search_from = 0usize; // advances for the Simple-form left-to-right search.
    if let Some(params) = &sig.parameters {
        for p in params {
            let (text, range) = match &p.label {
                lsp_types::ParameterLabel::LabelOffsets([start, end]) => {
                    let s = (*start as usize).min(label.len());
                    let e = (*end as usize).min(label.len());
                    let (s, e) = if s <= e { (s, e) } else { (e, s) };
                    // Snap to char boundaries so slicing the label is always valid (AC-008).
                    let s = snap_to_char_boundary(&label, s);
                    let e = snap_to_char_boundary(&label, e);
                    let text = label.get(s..e).unwrap_or("").to_owned();
                    (text, Some(s..e))
                }
                lsp_types::ParameterLabel::Simple(name) => {
                    let range = substring_range_from(&label, name, search_from);
                    if let Some(r) = &range {
                        search_from = r.end;
                    }
                    (name.clone(), range)
                }
            };
            parameters.push(ParamSpan { label: text, range_in_label: range });
        }
    }
    let documentation = sig.documentation.as_ref().map(documentation_to_string);
    SignatureInfo { label, parameters, documentation }
}

/// Flatten an `lsp_types::Documentation` (plain string or markup) to a display string.
fn documentation_to_string(doc: &lsp_types::Documentation) -> String {
    match doc {
        lsp_types::Documentation::String(s) => s.clone(),
        lsp_types::Documentation::MarkupContent(m) => m.value.clone(),
    }
}

/// Build a synthetic [`SignatureInfo`] from a backend code-nav symbol projection for the fallback path
/// (AC-003). The backend `CodeSymbolNavProjection` has NO literal `params` field (verified — MT-039 /
/// the KERNEL_BUILDER gate); the only signature-bearing field is `display_name`. When `display_name`
/// already carries a parenthesized parameter list (e.g. `add(a: i32, b: i32)` or
/// `fn add(a: i32, b: i32) -> i32`), the parameter spans are parsed from it (the same depth-aware
/// argument split the comma scanner uses, so generics/nested types do not split a parameter). When it
/// carries no parameter list, the signature is the bare name with zero parameters (the popup still
/// shows the call target — better than nothing). Returns `None` only for an empty display name.
pub fn signature_from_code_nav_symbol(symbol: &CodeSymbolNavProjection) -> Option<SignatureInfo> {
    let name = symbol.display_name.trim();
    if name.is_empty() {
        return None;
    }
    let label = name.to_owned();
    // Find the FIRST top-level '(' and its matching ')' in the display name; the content between them
    // is the parameter list (split at top-level commas). Absent -> a bare-name signature.
    let params = match top_level_paren_args(&label) {
        Some(args) => args,
        None => return Some(SignatureInfo { label, parameters: Vec::new(), documentation: None }),
    };
    Some(SignatureInfo::from_label_and_param_strings(label, params))
}

/// Split the parameter list inside the FIRST top-level `(...)` of `label` into individual parameter
/// strings (trimmed), splitting at TOP-LEVEL commas only (commas nested in `()`/`[]`/`{}`/`<>`/strings
/// are kept inside their parameter). Returns `None` when there is no top-level paren pair (so the
/// caller renders a bare-name signature). An empty arg list (`f()`) returns `Some(vec![])`.
fn top_level_paren_args(label: &str) -> Option<Vec<String>> {
    let bytes = label.as_bytes();
    // Locate the first top-level '(' (depth 0 in the OTHER bracket kinds, ignoring string/char state).
    let mut scanner = DepthScanner::new();
    let mut open: Option<usize> = None;
    for (i, &b) in bytes.iter().enumerate() {
        let c = b as char;
        if !scanner.in_literal() && scanner.paren_depth() == 0 && c == '(' {
            open = Some(i);
            break;
        }
        scanner.step(c);
    }
    let open = open?;
    // Walk from just after '(' collecting args, splitting at depth-1 commas, until the matching ')'.
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut inner = DepthScanner::new();
    inner.step('('); // enter the call: paren_depth now 1.
    let mut closed = false;
    for &b in &bytes[open + 1..] {
        let c = b as char;
        // The matching close-paren is the first ')' at the call's own paren depth (1), outside a
        // literal — a nested ')' (depth > 1) just decrements and stays inside the arg.
        if !inner.in_literal() && inner.paren_depth() == 1 && c == ')' {
            closed = true;
            break;
        }
        // A top-level argument separator is a ',' at the call's top level (no nested []/{}/<>) — a comma
        // inside a generic/array/map/nested-call stays part of the current arg.
        if c == ',' && inner.at_call_top_level() {
            let trimmed = current.trim().to_owned();
            if !trimmed.is_empty() {
                args.push(trimmed);
            }
            current.clear();
            inner.step(c);
            continue;
        }
        current.push(c);
        inner.step(c);
    }
    if !closed {
        // Unbalanced display name; still treat what we have as a best-effort arg list.
    }
    let trimmed = current.trim().to_owned();
    if !trimmed.is_empty() {
        args.push(trimmed);
    }
    Some(args)
}

/// Count the TOP-LEVEL commas between the call's open-paren and the cursor, returning the active
/// parameter index (commas-before-cursor at the call's depth). `buffer_slice` is the text from
/// `open_paren_byte` (INCLUSIVE — its first char should be the `(`) to `cursor_byte`. Commas nested in
/// inner `()`/`[]`/`{}`/generic `<>` brackets, string literals, and char literals are SKIPPED
/// (RISK-001 / MC-001 / AC-007). The result is 0 right after the open-paren and increments at each
/// top-level comma the cursor has passed.
///
/// `open_paren_byte` and `cursor_byte` are absolute buffer offsets; the slice must be exactly
/// `buffer[open_paren_byte..cursor_byte]` (the panel slices it). When the slice does not start with `(`
/// the scanner still works (it just never enters the call depth and returns 0), so a defensive caller
/// never panics.
pub fn active_parameter_from_commas(
    buffer_slice: &str,
    open_paren_byte: usize,
    cursor_byte: usize,
) -> usize {
    // `open_paren_byte` / `cursor_byte` are documented for the caller's contract; the count is computed
    // purely from the slice (the offsets let the caller assert the slice bounds).
    debug_assert!(open_paren_byte <= cursor_byte);
    let mut scanner = DepthScanner::new();
    let mut commas = 0usize;
    // The slice begins at the '(': enter the call so its arguments are at depth 1.
    let mut chars = buffer_slice.chars();
    // Consume the leading '(' if present so the call's base depth is 1.
    if let Some(first) = chars.clone().next() {
        if first == '(' {
            scanner.step('(');
            chars.next();
        }
    }
    for c in chars {
        // A comma at the call's top argument level (paren depth 1, no nested []/{}/<> and not inside a
        // literal) is a top-level argument separator (RISK-001 / AC-007).
        if c == ',' && scanner.at_call_top_level() {
            commas += 1;
        }
        scanner.step(c);
    }
    commas
}

/// Split the signature label into `(text, is_active)` runs so the renderer can draw the active
/// parameter emphasized (bold). The active run is the `range_in_label` of `parameters[active_param]`
/// in the active signature; everything before/after it is inactive. When the active parameter has no
/// resolvable range (or the index is out of bounds) the whole label is one inactive run (the popup
/// still shows the signature — AC-008, no panic). Runs are returned in label order with no gaps.
pub fn signature_label_runs(sig: &SignatureInfo, active_param: usize) -> Vec<(String, bool)> {
    let active_range = sig
        .parameters
        .get(active_param)
        .and_then(|p| p.range_in_label.clone());
    let Some(range) = active_range else {
        return vec![(sig.label.clone(), false)];
    };
    // Defensive clamp so a stale range never slices out of bounds (AC-008).
    let start = snap_to_char_boundary(&sig.label, range.start.min(sig.label.len()));
    let end = snap_to_char_boundary(&sig.label, range.end.min(sig.label.len()));
    if start >= end {
        return vec![(sig.label.clone(), false)];
    }
    let mut runs = Vec::with_capacity(3);
    if start > 0 {
        runs.push((sig.label[..start].to_owned(), false));
    }
    runs.push((sig.label[start..end].to_owned(), true));
    if end < sig.label.len() {
        runs.push((sig.label[end..].to_owned(), false));
    }
    runs
}

/// Render the floating signature-help popup at the cursor and register its AccessKit metadata.
///
/// The popup is a NON-focus-stealing [`egui::Area`] on the [`egui::Order::Tooltip`] order (RISK-003 /
/// MC-003): it never requests focus, so the editor keeps keyboard input and the trigger character still
/// lands. It is anchored ABOVE the cursor line (offset up by an estimated popup height) so it does not
/// occlude the text being typed; if there is no room above (it would clip the top of the editor) it
/// falls back to BELOW the cursor line (RISK-006). The active parameter renders `.strong()` in an
/// emphasis color from `ui.visuals()` (AC-004). When there are multiple overloads a `1/N` indicator is
/// shown. A `Role::Tooltip` AccessKit node `code_editor_signature_help` carries the active signature
/// label so a swarm agent reads the hint without pixels (AC-005 / MC-006).
///
/// `instance` is the panel's AccessKit instance suffix (empty for the default panel) so a diff view's
/// two editors do not collide on the popup author_id (the same scheme the MT-008 overlays use).
pub fn render_signature_popup(
    ctx: &egui::Context,
    state: &SignatureHelpState,
    cursor_screen_pos: egui::Pos2,
    instance: &str,
) {
    let Some(sig) = state.active() else {
        return; // no signatures -> nothing to show (defensive).
    };
    let runs = signature_label_runs(sig, state.active_parameter);
    let overload_indicator = if state.signatures.len() > 1 {
        Some(format!("{}/{}", state.active_signature + 1, state.signatures.len()))
    } else {
        None
    };
    let doc_first_line = sig
        .documentation
        .as_deref()
        .and_then(|d| d.lines().find(|l| !l.trim().is_empty()))
        .map(|l| l.trim().to_owned());

    // Estimate the popup height to anchor it ABOVE the cursor line (one signature line + optional
    // overload/doc lines). A modest fixed estimate is enough; egui clamps the Area into the screen.
    let line_h = ctx.style().text_styles
        .get(&egui::TextStyle::Body)
        .map(|f| f.size)
        .unwrap_or(14.0);
    let mut estimated_lines = 1.0_f32;
    if overload_indicator.is_some() {
        estimated_lines += 1.0;
    }
    if doc_first_line.is_some() {
        estimated_lines += 1.0;
    }
    let estimated_height = estimated_lines * (line_h + 4.0) + 12.0;
    // Anchor above the cursor line; if that would clip above the editor top, fall back to below.
    let above_y = cursor_screen_pos.y - estimated_height - 4.0;
    let anchor = if above_y >= ctx.content_rect().top() + 2.0 {
        egui::pos2(cursor_screen_pos.x, above_y)
    } else {
        // No room above: anchor a touch below the cursor line.
        egui::pos2(cursor_screen_pos.x, cursor_screen_pos.y + 4.0)
    };

    let area_id = egui::Id::new(("code-editor-signature-help-area", instance));
    egui::Area::new(area_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(anchor)
        .interactable(false) // never takes pointer/keyboard (RISK-003 / MC-003).
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(520.0);
                // The signature label as a sequence of runs; the active parameter run is emphasized.
                let emphasis = ui.visuals().selection.stroke.color;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (text, is_active) in &runs {
                        let mut rich = egui::RichText::new(text).monospace();
                        if *is_active {
                            rich = rich.strong().color(emphasis);
                        }
                        ui.label(rich);
                    }
                    if let Some(ind) = &overload_indicator {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!("({ind})"))
                                .small()
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                });
                if let Some(doc) = &doc_first_line {
                    ui.label(
                        egui::RichText::new(doc).small().color(ui.visuals().weak_text_color()),
                    );
                }

                // Emit the Tooltip AccessKit node carrying the full active signature label + the active
                // parameter, so a swarm agent reads the hint without pixels (AC-005 / MC-006).
                let author = if instance.is_empty() {
                    CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID.to_owned()
                } else {
                    format!("{CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID}#{instance}")
                };
                let active_label = sig
                    .parameters
                    .get(state.active_parameter)
                    .map(|p| p.label.clone())
                    .unwrap_or_default();
                let mut value = sig.label.clone();
                if !active_label.is_empty() {
                    value.push_str(&format!("  [active: {active_label}]"));
                }
                if let Some(ind) = &overload_indicator {
                    value.push_str(&format!("  ({ind})"));
                }
                let node_id = signature_help_node_id(instance);
                ctx.accesskit_node_builder(node_id, move |node| {
                    node.set_role(accesskit::Role::Tooltip);
                    node.set_author_id(author.clone());
                    node.set_label("Code editor signature help".to_owned());
                    node.set_value(value.clone());
                });
            });
        });
}

/// The fixed `egui::Id` backing the signature-help popup's AccessKit node (default panel; instances
/// hash the suffixed author_id so two editors do not collide — RISK-004).
fn signature_help_node_id(instance: &str) -> egui::Id {
    if instance.is_empty() {
        // SAFETY: a single hand-assigned fixed id in the disjoint overlay band (700, above the MT-008
        // overlay band's top at 681); never reused, so it cannot self-collide.
        unsafe { egui::Id::from_high_entropy_bits(SIGNATURE_HELP_NODE_ID) }
    } else {
        egui::Id::new(format!("{CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID}#{instance}"))
    }
}

/// Find the byte range of the first occurrence of `needle` in `haystack` at or after `from`, or `None`.
/// Used to resolve a `ParameterLabel::Simple` / code-nav parameter string to a span in the signature
/// label, advancing `from` across calls so a repeated name resolves to distinct spans (RISK-005).
fn substring_range_from(haystack: &str, needle: &str, from: usize) -> Option<Range<usize>> {
    if needle.is_empty() || from > haystack.len() {
        return None;
    }
    let from = snap_to_char_boundary(haystack, from.min(haystack.len()));
    let rel = haystack.get(from..)?.find(needle)?;
    let start = from + rel;
    Some(start..start + needle.len())
}

/// Snap a byte index to the nearest char boundary at or before it (so slicing the label is always
/// valid even when an LSP `LabelOffsets` lands mid-codepoint — AC-008, never panics).
fn snap_to_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// A small depth-aware scanner over source-like text that tracks bracket nesting and string/char
/// literal state, so a comma scan skips commas nested inside `()`/`[]`/`{}`/generic `<>` brackets and
/// inside string/char literals (RISK-001 / MC-001). It is intentionally lightweight (no full lexer):
/// it tracks paren/bracket/brace/angle depth and a string/char literal flag with backslash escaping.
/// `<` only increments angle depth in a generic-looking context (preceded by an identifier char or
/// `>`); otherwise (e.g. a `<` comparison) it is treated as plain text so an expression like `a < b`
/// does not wrongly enter a generic depth.
#[derive(Debug, Clone)]
struct DepthScanner {
    paren: i32,
    bracket: i32,
    brace: i32,
    angle: i32,
    in_string: bool,
    in_char: bool,
    escaped: bool,
    prev: Option<char>,
}

impl DepthScanner {
    fn new() -> Self {
        Self {
            paren: 0,
            bracket: 0,
            brace: 0,
            angle: 0,
            in_string: false,
            in_char: false,
            escaped: false,
            prev: None,
        }
    }

    /// Whether the scanner is currently inside a string or char literal.
    fn in_literal(&self) -> bool {
        self.in_string || self.in_char
    }

    /// The current `()` nesting depth (the call-argument depth the comma scanner counts at).
    fn paren_depth(&self) -> i32 {
        self.paren
    }

    /// Whether the scanner is exactly at the call's top argument level: paren depth 1 and NO open
    /// `[]`/`{}`/`<>` nesting and not inside a literal. A comma here is a top-level argument separator;
    /// a comma anywhere else (a nested call/array/map/generic/literal) is NOT (RISK-001 / AC-007).
    fn at_call_top_level(&self) -> bool {
        !self.in_literal()
            && self.paren == 1
            && self.bracket == 0
            && self.brace == 0
            && self.angle == 0
    }

    /// Advance the scanner over one character, updating literal + depth state.
    fn step(&mut self, c: char) {
        // Inside a string/char literal: only the matching unescaped quote (or end-escape) matters.
        if self.in_string {
            if self.escaped {
                self.escaped = false;
            } else if c == '\\' {
                self.escaped = true;
            } else if c == '"' {
                self.in_string = false;
            }
            self.prev = Some(c);
            return;
        }
        if self.in_char {
            if self.escaped {
                self.escaped = false;
            } else if c == '\\' {
                self.escaped = true;
            } else if c == '\'' {
                self.in_char = false;
            }
            self.prev = Some(c);
            return;
        }
        match c {
            '"' => self.in_string = true,
            '\'' => self.in_char = true,
            '(' => self.paren += 1,
            ')' => self.paren = (self.paren - 1).max(0),
            '[' => self.bracket += 1,
            ']' => self.bracket = (self.bracket - 1).max(0),
            '{' => self.brace += 1,
            '}' => self.brace = (self.brace - 1).max(0),
            '<' => {
                // Only treat '<' as a generic open when it looks like a generic (preceded by an
                // identifier char or a closing '>'), so a comparison `a < b` is not misread.
                if matches!(self.prev, Some(p) if p.is_alphanumeric() || p == '_' || p == '>') {
                    self.angle += 1;
                }
            }
            '>' => {
                // Close a generic only if one is open (so `->` / `=>` / a comparison does not underflow).
                if self.angle > 0 {
                    self.angle -= 1;
                }
            }
            _ => {}
        }
        self.prev = Some(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_parameter_counts_top_level_commas() {
        // `(a, b, c)` — cursor after the second top-level comma -> active parameter 2.
        let s = "(a, b, c";
        assert_eq!(active_parameter_from_commas(s, 0, s.len()), 2);
        // Right after the open-paren -> parameter 0.
        assert_eq!(active_parameter_from_commas("(", 0, 1), 0);
        // After the first comma -> parameter 1.
        assert_eq!(active_parameter_from_commas("(a,", 0, 3), 1);
    }

    #[test]
    fn active_parameter_ignores_nested_calls() {
        // The comma inside `inner(x, y)` must NOT advance the outer active parameter (RISK-001).
        let s = "(a, inner(x, y), ";
        // Outer commas the cursor has passed: after `a,` (1) and after `inner(...),` (2) -> param 2.
        assert_eq!(active_parameter_from_commas(s, 0, s.len()), 2);
        // Cursor INSIDE the inner call, after its first comma: outer param is still 1 (the inner comma
        // is at depth 2, skipped).
        let inside = "(a, inner(x, ";
        assert_eq!(active_parameter_from_commas(inside, 0, inside.len()), 1);
    }

    #[test]
    fn active_parameter_ignores_brackets_braces_and_generics() {
        // Commas inside `[]`, `{}`, and generic `<>` are skipped (AC-007).
        let arr = "(a, [1, 2, 3], ";
        assert_eq!(active_parameter_from_commas(arr, 0, arr.len()), 2);
        let map = "(a, Foo { x: 1, y: 2 }, ";
        assert_eq!(active_parameter_from_commas(map, 0, map.len()), 2);
        let generic = "(a, HashMap<K, V>, ";
        assert_eq!(active_parameter_from_commas(generic, 0, generic.len()), 2);
    }

    #[test]
    fn active_parameter_ignores_string_and_char_literals() {
        // Commas inside a string literal and a char literal must be skipped (AC-007).
        let strs = "(a, \"x, y, z\", ";
        assert_eq!(active_parameter_from_commas(strs, 0, strs.len()), 2);
        let chr = "(a, ',', ";
        assert_eq!(active_parameter_from_commas(chr, 0, chr.len()), 2);
        // An escaped quote inside a string does not end the literal early.
        let esc = "(a, \"he said \\\"hi, there\\\"\", ";
        assert_eq!(active_parameter_from_commas(esc, 0, esc.len()), 2);
    }

    #[test]
    fn comparison_less_than_is_not_a_generic() {
        // `a < b` is a comparison, not a generic open, so the following comma stays top-level.
        let s = "(a < b, c";
        assert_eq!(active_parameter_from_commas(s, 0, s.len()), 1);
    }

    #[test]
    fn label_runs_emphasize_the_active_parameter() {
        // `LabelOffsets`-style explicit spans: active parameter 1 ('b: i32') is the emphasized run.
        let sig = SignatureInfo {
            label: "fn add(a: i32, b: i32) -> i32".to_owned(),
            parameters: vec![
                ParamSpan { label: "a: i32".into(), range_in_label: Some(7..13) },
                ParamSpan { label: "b: i32".into(), range_in_label: Some(15..21) },
            ],
            documentation: None,
        };
        let runs = signature_label_runs(&sig, 1);
        // The emphasized run is exactly 'b: i32'.
        let active: Vec<&String> = runs.iter().filter(|(_, a)| *a).map(|(t, _)| t).collect();
        assert_eq!(active, vec![&"b: i32".to_owned()]);
        // Reassembling the runs reproduces the whole label (no gaps/overlaps).
        let joined: String = runs.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(joined, sig.label);
    }

    #[test]
    fn simple_label_form_resolves_repeated_names_to_distinct_spans() {
        // RISK-005 / MC-005: two params named 'x' must map to DISTINCT spans (left-to-right consume).
        let sig = SignatureInfo::from_label_and_param_strings(
            "f(x: i32, x: i32)".to_owned(),
            vec!["x: i32".to_owned(), "x: i32".to_owned()],
        );
        let r0 = sig.parameters[0].range_in_label.clone().unwrap();
        let r1 = sig.parameters[1].range_in_label.clone().unwrap();
        assert_ne!(r0, r1, "repeated param names resolve to distinct spans");
        assert!(r0.end <= r1.start, "the second match is after the first");
        // The active run for param 1 is the SECOND 'x: i32'.
        let runs = signature_label_runs(&sig, 1);
        let active_start = sig.label.find("x: i32").map(|i| i + "x: i32".len()).unwrap();
        let active_text: String =
            runs.iter().filter(|(_, a)| *a).map(|(t, _)| t.clone()).collect();
        assert_eq!(active_text, "x: i32");
        assert!(r1.start >= active_start, "param 1 maps to the second occurrence");
    }

    #[test]
    fn from_lsp_parses_active_parameter_and_both_label_forms() {
        // A SignatureHelp with one signature, active_parameter = 1, mixing both ParameterLabel forms.
        let help = lsp_types::SignatureHelp {
            signatures: vec![lsp_types::SignatureInformation {
                label: "fn add(a: i32, b: i32) -> i32".to_owned(),
                documentation: Some(lsp_types::Documentation::String("Adds two numbers".into())),
                parameters: Some(vec![
                    lsp_types::ParameterInformation {
                        // Offsets form for 'a: i32'.
                        label: lsp_types::ParameterLabel::LabelOffsets([7, 13]),
                        documentation: None,
                    },
                    lsp_types::ParameterInformation {
                        // Simple form for 'b: i32'.
                        label: lsp_types::ParameterLabel::Simple("b: i32".into()),
                        documentation: None,
                    },
                ]),
                active_parameter: None,
            }],
            active_signature: Some(0),
            active_parameter: Some(1),
        };
        let state = SignatureHelpState::from_lsp(&help, 42).expect("one signature -> Some");
        assert_eq!(state.active_parameter, 1, "active parameter parsed from the response");
        assert_eq!(state.source, SignatureSource::Lsp);
        assert_eq!(state.anchor_byte, 42);
        let sig = state.active().unwrap();
        // Both forms resolved to a non-empty span.
        assert_eq!(sig.parameters[0].range_in_label, Some(7..13));
        assert!(sig.parameters[1].range_in_label.is_some());
        // The active run is 'b: i32'.
        let runs = signature_label_runs(sig, state.active_parameter);
        let active: String = runs.iter().filter(|(_, a)| *a).map(|(t, _)| t.clone()).collect();
        assert_eq!(active, "b: i32");
        assert_eq!(sig.documentation.as_deref(), Some("Adds two numbers"));
    }

    #[test]
    fn from_lsp_empty_signatures_is_none() {
        // An empty SignatureHelp -> None so the caller falls back to code-nav (AC-003).
        let help = lsp_types::SignatureHelp {
            signatures: vec![],
            active_signature: None,
            active_parameter: None,
        };
        assert!(SignatureHelpState::from_lsp(&help, 0).is_none());
    }

    #[test]
    fn from_lsp_clamps_out_of_range_indices_without_panic() {
        // AC-008: out-of-range active indices clamp instead of panicking.
        let help = lsp_types::SignatureHelp {
            signatures: vec![lsp_types::SignatureInformation {
                label: "f(a)".to_owned(),
                documentation: None,
                parameters: Some(vec![lsp_types::ParameterInformation {
                    label: lsp_types::ParameterLabel::Simple("a".into()),
                    documentation: None,
                }]),
                active_parameter: None,
            }],
            active_signature: Some(9), // out of range
            active_parameter: Some(9), // out of range
        };
        let state = SignatureHelpState::from_lsp(&help, 0).unwrap();
        assert_eq!(state.active_signature, 0, "out-of-range signature index clamped");
        assert_eq!(state.active_parameter, 0, "out-of-range parameter index clamped");
    }

    #[test]
    fn label_offsets_out_of_bounds_does_not_panic() {
        // A server sending offsets past the label length must clamp (AC-008).
        let sig = lsp_types::SignatureInformation {
            label: "f(a)".to_owned(),
            documentation: None,
            parameters: Some(vec![lsp_types::ParameterInformation {
                label: lsp_types::ParameterLabel::LabelOffsets([2, 999]),
                documentation: None,
            }]),
            active_parameter: None,
        };
        let info = signature_info_from_lsp(&sig);
        // The span is clamped to the label length; slicing never panicked.
        let range = info.parameters[0].range_in_label.clone().unwrap();
        assert!(range.end <= info.label.len());
    }

    #[test]
    fn code_nav_fallback_builds_signature_from_display_name() {
        // AC-003: the fallback derives params from the REAL display_name (no assumed 'params' field).
        let symbol = CodeSymbolNavProjection {
            display_name: "add(a: i32, b: i32)".into(),
            symbol_kind: "function".into(),
            ..Default::default()
        };
        let state = SignatureHelpState::from_code_nav(&symbol, 10, 1).expect("Some signature");
        assert_eq!(state.source, SignatureSource::CodeNavFallback);
        let sig = state.active().unwrap();
        assert_eq!(sig.parameters.len(), 2, "two params parsed from the display name");
        assert_eq!(sig.parameters[0].label, "a: i32");
        assert_eq!(sig.parameters[1].label, "b: i32");
        // The active run for param 1 is 'b: i32'.
        let runs = signature_label_runs(sig, state.active_parameter);
        let active: String = runs.iter().filter(|(_, a)| *a).map(|(t, _)| t.clone()).collect();
        assert_eq!(active, "b: i32");
    }

    #[test]
    fn code_nav_fallback_bare_name_has_no_parameters() {
        // A display name with no paren list -> a bare-name signature (still shown), zero params.
        let symbol = CodeSymbolNavProjection {
            display_name: "MY_CONSTANT".into(),
            ..Default::default()
        };
        let state = SignatureHelpState::from_code_nav(&symbol, 0, 0).expect("bare name -> Some");
        assert!(state.active().unwrap().parameters.is_empty());
        // The whole label is one inactive run.
        let runs = signature_label_runs(state.active().unwrap(), 0);
        assert_eq!(runs, vec![("MY_CONSTANT".to_owned(), false)]);
    }

    #[test]
    fn code_nav_fallback_empty_name_is_none() {
        let symbol = CodeSymbolNavProjection { display_name: "   ".into(), ..Default::default() };
        assert!(SignatureHelpState::from_code_nav(&symbol, 0, 0).is_none());
    }

    #[test]
    fn code_nav_fallback_does_not_split_nested_generics() {
        // A param with a nested generic comma must stay ONE parameter (the depth-aware arg split).
        let symbol = CodeSymbolNavProjection {
            display_name: "f(map: HashMap<K, V>, n: usize)".into(),
            ..Default::default()
        };
        let state = SignatureHelpState::from_code_nav(&symbol, 0, 0).unwrap();
        let sig = state.active().unwrap();
        assert_eq!(sig.parameters.len(), 2, "the generic comma did not split the parameter");
        assert_eq!(sig.parameters[0].label, "map: HashMap<K, V>");
        assert_eq!(sig.parameters[1].label, "n: usize");
    }

    #[test]
    fn overload_cycling_wraps_and_clamps_active_parameter() {
        let mut state = SignatureHelpState {
            signatures: vec![
                SignatureInfo::from_label_and_param_strings("f(a, b)", vec!["a".into(), "b".into()]),
                SignatureInfo::from_label_and_param_strings("f(a)", vec!["a".into()]),
            ],
            active_signature: 0,
            active_parameter: 1,
            anchor_byte: 0,
            source: SignatureSource::Lsp,
        };
        // Switch to the second overload (1 param): the active parameter clamps from 1 to 0.
        state.select_next_signature();
        assert_eq!(state.active_signature, 1);
        assert_eq!(state.active_parameter, 0, "active param clamped to the smaller overload");
        // Wrap forward back to the first.
        state.select_next_signature();
        assert_eq!(state.active_signature, 0);
        // Wrap backward to the last.
        state.select_prev_signature();
        assert_eq!(state.active_signature, 1);
    }
}
