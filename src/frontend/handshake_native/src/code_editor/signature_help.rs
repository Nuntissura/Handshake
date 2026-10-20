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
//! - the Handshake backend code-nav symbol under the cursor -> [`from_code_nav`]
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

use egui::accesskit;

use super::code_nav::CodeSymbolNavProjection;

pub use handshake_signature_help::model::{
    signature_label_runs, ParamSpan, SignatureHelpState, SignatureInfo, SignatureSource,
};
pub use handshake_signature_help::scanner::{active_parameter_from_commas, top_level_paren_args};

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

/// Build the FALLBACK state from a Handshake backend code-nav symbol projection (AC-003). The label
/// and parameters are derived from the REAL backend fields (`display_name` and any parenthesized
/// parameter list it carries) — see [`signature_from_code_nav_symbol`]. `active_parameter` is the
/// locally-computed top-level comma count (the panel passes it). Returns `None` when the symbol
/// yields no usable signature label (the popup then shows nothing — never a panic, AC-008).
pub fn from_code_nav(
    symbol: &CodeSymbolNavProjection,
    anchor_byte: usize,
    active_parameter: usize,
) -> Option<SignatureHelpState> {
    let signature = signature_from_code_nav_symbol(symbol)?;
    let mut state = SignatureHelpState {
        signatures: vec![signature],
        active_signature: 0,
        active_parameter,
        anchor_byte,
        source: SignatureSource::CodeNavFallback,
    };
    state.clamp_active_parameter();
    Some(state)
}

/// Build a synthetic [`SignatureInfo`] from a backend code-nav symbol projection for the fallback path
/// (AC-003).
///
/// REAL-SHAPE DISCLOSURE (Spec-Realism Gate sub-rule 2 — verify-over-prose): the backend
/// `CodeSymbolNavProjection` has NO literal `params` field AND its `display_name` is a BARE identifier,
/// not a signature. The backend sets `display_name: symbol.name.clone()`
/// (`knowledge_code_index/engine.rs:781`) and `ExtractedSymbol.name` is documented as "Simple
/// identifier, e.g. `render`" (`knowledge_code_index/symbols.rs:79-80`); neither
/// `GET /knowledge/code/symbols/:entity_id` nor `.../spans` returns a parameter signature or the source
/// text to derive one. So against the CURRENT real backend this function ALWAYS takes the bare-name
/// branch: the fallback popup shows the call target with ZERO parameters (better than nothing — it still
/// names what is being called), and the parameter-hint behavior is the code-nav
/// `NEEDS_MANAGED_RESOURCE_PROOF` gap (a parameter-bearing fallback needs a NEW backend route, which is a
/// typed blocker, NOT a frontend-faked signature).
///
/// FORWARD-COMPAT (no-cost): if `display_name` ever DOES carry a parenthesized parameter list (e.g. a
/// future server/adapter that emits `add(a: i32, b: i32)` or `fn add(a: i32, b: i32) -> i32`), the
/// parameter spans are parsed from it via the same depth-aware argument split the comma scanner uses
/// (generics/nested types do not split a parameter). This branch is retained purely as a forward path;
/// it is NOT asserted to fire against the current real backend. Returns `None` only for an empty name.
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
        None => {
            return Some(SignatureInfo {
                label,
                parameters: Vec::new(),
                documentation: None,
            })
        }
    };
    Some(SignatureInfo::from_label_and_param_strings(label, params))
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
        Some(format!(
            "{}/{}",
            state.active_signature + 1,
            state.signatures.len()
        ))
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
    let line_h = ctx
        .style()
        .text_styles
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
                        egui::RichText::new(doc)
                            .small()
                            .color(ui.visuals().weak_text_color()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_nav_fallback_real_backend_shape_is_bare_name_zero_params() {
        // REAL-SHAPE PROOF (Spec-Realism sub-rule 2): the backend `symbol_to_json` projection's
        // `display_name` is a BARE identifier (`display_name = symbol.name.clone()`, engine.rs:781;
        // `ExtractedSymbol.name` = "Simple identifier, e.g. `render`", symbols.rs:79-80). This fixture
        // is that captured real shape — NOT a paren-bearing synthetic. Against it the fallback degrades
        // to a bare-name signature with ZERO parameters (the parameter-hint fallback is the
        // NEEDS_MANAGED_RESOURCE_PROOF code-nav gap; a param-bearing fallback needs a NEW backend route).
        let real_symbol = CodeSymbolNavProjection {
            symbol_entity_id: "ent-add".into(),
            symbol_key: "fn:src/math.rs#add".into(),
            display_name: "add".into(), // bare identifier — the real backend shape.
            symbol_kind: "function".into(),
            ..Default::default()
        };
        // Even with a non-zero local comma count (cursor on argument 1), the real symbol exposes no
        // parameters, so there is no active-parameter run — the popup just names the call target.
        let state = from_code_nav(&real_symbol, 10, 1).expect("Some signature");
        assert_eq!(state.source, SignatureSource::CodeNavFallback);
        let sig = state.active().unwrap();
        assert_eq!(
            sig.label, "add",
            "the real fallback label is the bare call-target name"
        );
        assert!(
            sig.parameters.is_empty(),
            "the REAL backend symbol carries no parameter signature -> zero params"
        );
        // The whole label is one inactive run: no emphasis against real data.
        let runs = signature_label_runs(sig, state.active_parameter);
        assert_eq!(runs, vec![("add".to_owned(), false)]);
    }

    #[test]
    fn code_nav_fallback_bare_name_has_no_parameters() {
        // A display name with no paren list -> a bare-name signature (still shown), zero params. This is
        // also the real backend shape for any non-callable symbol (constants, modules, ...).
        let symbol = CodeSymbolNavProjection {
            display_name: "MY_CONSTANT".into(),
            ..Default::default()
        };
        let state = from_code_nav(&symbol, 0, 0).expect("bare name -> Some");
        assert!(state.active().unwrap().parameters.is_empty());
        // The whole label is one inactive run.
        let runs = signature_label_runs(state.active().unwrap(), 0);
        assert_eq!(runs, vec![("MY_CONSTANT".to_owned(), false)]);
    }

    #[test]
    fn code_nav_fallback_parses_params_only_from_a_rich_label_forward_compat() {
        // FORWARD-COMPAT ONLY (NOT the current real backend): IF a future server/adapter ever emits a
        // parenthesized `display_name`, the paren-parser splits its params. This proves the retained
        // forward path; the current real backend emits a bare identifier (see the bare-name test above),
        // so this case does NOT fire in production today.
        let future_rich_symbol = CodeSymbolNavProjection {
            display_name: "add(a: i32, b: i32)".into(),
            symbol_kind: "function".into(),
            ..Default::default()
        };
        let state = from_code_nav(&future_rich_symbol, 10, 1).expect("Some");
        let sig = state.active().unwrap();
        assert_eq!(
            sig.parameters.len(),
            2,
            "a rich label's two params parse via the forward path"
        );
        assert_eq!(sig.parameters[0].label, "a: i32");
        assert_eq!(sig.parameters[1].label, "b: i32");
        let runs = signature_label_runs(sig, state.active_parameter);
        let active: String = runs
            .iter()
            .filter(|(_, a)| *a)
            .map(|(t, _)| t.clone())
            .collect();
        assert_eq!(active, "b: i32");
    }

    #[test]
    fn code_nav_fallback_empty_name_is_none() {
        let symbol = CodeSymbolNavProjection {
            display_name: "   ".into(),
            ..Default::default()
        };
        assert!(from_code_nav(&symbol, 0, 0).is_none());
    }

    #[test]
    fn code_nav_fallback_does_not_split_nested_generics() {
        // A param with a nested generic comma must stay ONE parameter (the depth-aware arg split).
        let symbol = CodeSymbolNavProjection {
            display_name: "f(map: HashMap<K, V>, n: usize)".into(),
            ..Default::default()
        };
        let state = from_code_nav(&symbol, 0, 0).unwrap();
        let sig = state.active().unwrap();
        assert_eq!(
            sig.parameters.len(),
            2,
            "the generic comma did not split the parameter"
        );
        assert_eq!(sig.parameters[0].label, "map: HashMap<K, V>");
        assert_eq!(sig.parameters[1].label, "n: usize");
    }
}
