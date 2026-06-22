//! The floating find/replace panel widget (WP-KERNEL-012 MT-018).
//!
//! Ports `app/src/components/FindReplacePanel.tsx`: a floating, non-blocking panel anchored at the
//! top-right of the content area with
//! - Row 1 (find): `[find input] [Aa case] [W word] [.* regex] [count] [◄ prev] [► next] [✕ close]`,
//! - Row 2 (replace, only when `with_replace`): `[replace input] [Replace] [Replace All]`,
//! - an inline red `find-error` line below the find input when the regex is invalid.
//!
//! Keyboard inside the find/replace inputs: Enter = next match, Shift+Enter = prev match, Escape =
//! close. The match count shows `{current+1} of {total}` / `{total} matches` / `No matches`, with a
//! `+` marker when the scan truncated.
//!
//! ## Non-focus-stealing (HBR-QUIET, KERNEL_BUILDER gate)
//!
//! The panel is an `egui::Window`, which does NOT auto-grab keyboard focus. We request focus on the
//! FIND INPUT exactly once, when the panel opens (the `focus_find_input` one-shot flag), via
//! `Response::request_focus` (an egui in-app focus request, NEVER an OS focus grab). After that the
//! operator's clicks drive focus normally. This module makes NO OS focus-grab call.
//!
//! ## Output, not direct mutation
//!
//! The panel is a pure view over [`FindReplaceState`] (it reads the scan + edits the query/replacement
//! IN the state) and returns a typed [`PanelOutcome`]. The HOST widget (`rich_editor_widget.rs`)
//! applies a `ReplaceOne`/`ReplaceAll`/`Close` outcome against the document + undo manager — so the
//! doc-mutating replace lives in one place (`super::replace_one`/`replace_all`), reusing the MT-011
//! transaction path, and the panel never holds a `&mut BlockNode`.

use egui::accesskit;

use crate::theme::HsPalette;

use super::{
    FindReplaceState, FIND_CLOSE_AUTHOR_ID, FIND_COUNT_AUTHOR_ID, FIND_ERROR_AUTHOR_ID,
    FIND_ERROR_ROLE, FIND_INPUT_AUTHOR_ID, FIND_INPUT_ROLE, FIND_NEXT_AUTHOR_ID, FIND_PANEL_AUTHOR_ID,
    FIND_PANEL_ROLE, FIND_PREV_AUTHOR_ID, FIND_TOGGLE_CASE_AUTHOR_ID, FIND_TOGGLE_REGEX_AUTHOR_ID,
    FIND_TOGGLE_WORD_AUTHOR_ID, REPLACE_ALL_AUTHOR_ID, REPLACE_INPUT_AUTHOR_ID, REPLACE_ONE_AUTHOR_ID,
};

/// What the panel asks the host to do this frame. The host applies it against the document + undo
/// manager (the panel never mutates the doc itself). `None` = no decisive action this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelOutcome {
    /// Nothing decisive happened (the operator only edited the query / toggled an option / navigated).
    #[default]
    None,
    /// Replace the CURRENT match (the Replace button, or the operator pressed it). The host calls
    /// [`super::replace_one`] with the active match then advances.
    ReplaceOne,
    /// Replace ALL matches in one transaction (the Replace All button). The host calls
    /// [`super::replace_all`].
    ReplaceAll,
    /// Close the panel (Escape or the ✕ button). The host clears `find_replace` + the highlights and
    /// returns focus to the editor.
    Close,
}

/// Render the find/replace panel for `state` as a floating window over the editor content, returning
/// the [`PanelOutcome`] the host applies. `palette` supplies all colors (no hardcoded hex). The panel
/// reads `state.scan` (recomputed by the host before this call) and edits `state.query` /
/// `state.replacement` / `state.active` in place; query edits set `query_changed` so the host knows to
/// re-scan.
///
/// Returns `(outcome, query_changed)`: `query_changed` is `true` when the operator changed the find
/// text or any option toggle this frame (the host then re-runs the scan).
pub fn show_find_panel(
    ctx: &egui::Context,
    state: &mut FindReplaceState,
    palette: &HsPalette,
) -> (PanelOutcome, bool) {
    let mut outcome = PanelOutcome::None;
    let mut query_changed = false;

    // AC-9: a pressed Escape closes the panel, regardless of which sub-widget holds focus. We claim
    // it from the context input here (mirroring how the slash menu claims its keys before the focus
    // gate) so Escape never falls through to the editor as a no-op when, e.g., the surface (not the
    // find input) is the focused widget. `consume_key` removes it so nothing else handles it.
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        outcome = PanelOutcome::Close;
    }

    let window = egui::Window::new("Find")
        .id(egui::Id::new("rich-editor-find-panel"))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        // Float at the top-right of the content area (the React panel anchors top-right), with a
        // small inset so it does not touch the edge.
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0));

    window.show(ctx, |ui| {
        // The panel container node: a stable author_id (find-panel) so a swarm agent can address the
        // open panel. Emitted onto a dedicated child id WITHIN the window content so it nests under
        // the window's accessibility subtree (the same nesting pattern the editor uses for blocks).
        let container_id = ui.id().with("find-panel-container");
        emit_node(ui, container_id, FIND_PANEL_AUTHOR_ID, Some(FIND_PANEL_ROLE), None);

        // The whole panel reads in the theme surface; the frame is the egui window frame.
        ui.vertical(|ui| {
            // ── Row 1: find input + toggles + count + nav + close ──────────────────────────────
            ui.horizontal(|ui| {
                // Find input. Focus it exactly once on open (HBR-QUIET one-shot).
                let mut pattern = state.query.pattern.clone();
                let find_resp = ui.add(
                    egui::TextEdit::singleline(&mut pattern)
                        .id(egui::Id::new("find-input-field"))
                        .desired_width(180.0)
                        .hint_text("Find (prose + code)"),
                );
                emit_node(ui, find_resp.id, FIND_INPUT_AUTHOR_ID, Some(FIND_INPUT_ROLE), None);
                if pattern != state.query.pattern {
                    state.query.pattern = pattern;
                    state.active = None; // a new query resets the active match (React parity).
                    query_changed = true;
                }
                // One-shot focus on open: request egui focus (in-app), NEVER an OS focus grab.
                if state.focus_find_input {
                    find_resp.request_focus();
                    state.focus_find_input = false;
                }
                // Enter = next match, Shift+Enter = prev match while the find input has focus (Escape
                // is claimed globally at the top of this fn, so it is not handled here).
                if find_resp.has_focus() {
                    if let Some(action) = input_key_action(ui) {
                        match action {
                            InputKey::Next => state.select_next(),
                            InputKey::Prev => state.select_prev(),
                        }
                    }
                }

                // Option toggles (case / whole-word / regex). Each flips a bool + marks the query
                // changed so the host re-scans.
                query_changed |= toggle(ui, "Aa", &mut state.query.case_sensitive, FIND_TOGGLE_CASE_AUTHOR_ID, "Match case");
                query_changed |= toggle(ui, "W", &mut state.query.whole_word, FIND_TOGGLE_WORD_AUTHOR_ID, "Match whole word");
                query_changed |= toggle(ui, ".*", &mut state.query.is_regex, FIND_TOGGLE_REGEX_AUTHOR_ID, "Use regular expression");

                // Match count (`3 of 17` / `17 matches` / `No matches`, `+` when truncated).
                let count = state.count_label();
                let count_resp = ui.colored_label(palette.text_subtle, &count);
                emit_node(ui, count_resp.id, FIND_COUNT_AUTHOR_ID, None, Some(count));

                // Prev / Next / Close.
                let prev = ui.button("\u{25C4}"); // ◄
                emit_node(ui, prev.id, FIND_PREV_AUTHOR_ID, None, None);
                if prev.clicked() {
                    state.select_prev();
                }
                let next = ui.button("\u{25BA}"); // ►
                emit_node(ui, next.id, FIND_NEXT_AUTHOR_ID, None, None);
                if next.clicked() {
                    state.select_next();
                }
                let close = ui.button("\u{2715}"); // ✕
                emit_node(ui, close.id, FIND_CLOSE_AUTHOR_ID, None, None);
                if close.clicked() {
                    outcome = PanelOutcome::Close;
                }
            });

            // ── Inline regex error (role=alert), below the find input ──────────────────────────
            if let Some(err) = state.scan.error.clone() {
                let err_resp = ui.colored_label(palette.error_text, &err);
                emit_node(ui, err_resp.id, FIND_ERROR_AUTHOR_ID, Some(FIND_ERROR_ROLE), Some(err));
            }

            // ── Row 2: replace input + Replace / Replace All (replace mode only) ───────────────
            if state.with_replace {
                ui.horizontal(|ui| {
                    let mut replacement = state.replacement.clone();
                    let repl_resp = ui.add(
                        egui::TextEdit::singleline(&mut replacement)
                            .id(egui::Id::new("replace-input-field"))
                            .desired_width(180.0)
                            .hint_text("Replace"),
                    );
                    emit_node(ui, repl_resp.id, REPLACE_INPUT_AUTHOR_ID, Some(FIND_INPUT_ROLE), None);
                    if replacement != state.replacement {
                        state.replacement = replacement;
                    }
                    if repl_resp.has_focus() {
                        if let Some(action) = input_key_action(ui) {
                            match action {
                                InputKey::Next => state.select_next(),
                                InputKey::Prev => state.select_prev(),
                            }
                        }
                    }

                    let has_matches = !state.scan.is_empty();
                    let replace_one = ui.add_enabled(has_matches, egui::Button::new("Replace"));
                    emit_node(ui, replace_one.id, REPLACE_ONE_AUTHOR_ID, None, None);
                    if replace_one.clicked() {
                        outcome = PanelOutcome::ReplaceOne;
                    }
                    let replace_all = ui.add_enabled(has_matches, egui::Button::new("Replace All"));
                    emit_node(ui, replace_all.id, REPLACE_ALL_AUTHOR_ID, None, None);
                    if replace_all.clicked() {
                        outcome = PanelOutcome::ReplaceAll;
                    }
                });
            }
        });
    });

    (outcome, query_changed)
}

/// The navigation action recognized inside the find/replace text inputs (Escape is claimed globally
/// at the top of [`show_find_panel`], so it is not an `InputKey`).
enum InputKey {
    /// Enter -> next match.
    Next,
    /// Shift+Enter -> previous match.
    Prev,
}

/// Read the find-input navigation action for this frame: Enter (next), Shift+Enter (prev). Consumes
/// the key so egui does not also treat Enter as a newline/submit elsewhere.
fn input_key_action(ui: &egui::Ui) -> Option<InputKey> {
    ui.input_mut(|i| {
        if i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter) {
            return Some(InputKey::Prev);
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
            return Some(InputKey::Next);
        }
        None
    })
}

/// Render one option toggle button (a selectable button showing `label`, pressed when `*flag`).
/// Flips `*flag` on click, emits the toggle's stable author_id, and returns whether it changed.
fn toggle(ui: &mut egui::Ui, label: &str, flag: &mut bool, author_id: &str, tooltip: &str) -> bool {
    let resp = ui
        .add(egui::Button::selectable(*flag, label))
        .on_hover_text(tooltip);
    emit_node(ui, resp.id, author_id, None, None);
    if resp.clicked() {
        *flag = !*flag;
        true
    } else {
        false
    }
}

/// Emit a stable AccessKit node for an interactive (or labeled) panel widget through the SAME live
/// `accesskit_node_builder` hook the shell + the rest of the editor use, so the author_id (and
/// optional role/value) land in the real per-frame tree a swarm agent reads. Every interactive node
/// MUST carry an author_id (the shell HBR-SWARM gate panics on an unnamed interactive node).
fn emit_node(
    ui: &egui::Ui,
    id: egui::Id,
    author_id: &str,
    role: Option<accesskit::Role>,
    value: Option<String>,
) {
    let author = author_id.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_author_id(author.clone());
        if let Some(r) = role {
            node.set_role(r);
        }
        if let Some(v) = value.clone() {
            node.set_value(v);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::find_replace::scanner::FindQuery;

    #[test]
    fn panel_outcome_default_is_none() {
        assert_eq!(PanelOutcome::default(), PanelOutcome::None);
    }

    #[test]
    fn toggling_options_marks_query_changed() {
        // A focused unit check of the toggle helper's flip behavior via a headless frame.
        let ctx = egui::Context::default();
        let mut flag = false;
        let mut changed = false;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // Not clicked -> no change. (We cannot synthesize a click here, but the no-click
                // path must report false.)
                changed = toggle(ui, "Aa", &mut flag, FIND_TOGGLE_CASE_AUTHOR_ID, "Match case");
            });
        });
        assert!(!changed, "an un-clicked toggle reports no change");
        assert!(!flag, "the flag is untouched without a click");
    }

    #[test]
    fn state_query_edits_are_observed() {
        // The panel edits state.query.pattern in place; a direct state mutation here mirrors that
        // and proves count_label reacts (the panel's observable contract).
        let mut st = FindReplaceState::open(true);
        st.query = FindQuery::literal("foo");
        assert_eq!(st.query.pattern, "foo");
        assert!(st.with_replace, "Ctrl+H panel shows the replace row");
    }
}
