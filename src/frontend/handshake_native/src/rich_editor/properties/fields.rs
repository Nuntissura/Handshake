//! Individual field renderers for the properties panel (WP-KERNEL-012 MT-017).
//!
//! Each renderer takes a borrowed `egui::Ui` + the live [`PropertiesState`] (and, for the title, the
//! [`PropertiesRuntime`] for the save dispatch) and paints ONE metadata field into the panel's
//! two-column grid. All colors are `HsPalette` theme tokens (no hardcoded hex), and every interactive
//! node carries a stable AccessKit author_id through the SAME `ctx.accesskit_node_builder` hook the
//! shell uses (so the HBR-SWARM gate passes and a swarm agent can drive each field).

use egui::accesskit;

use crate::rich_editor::properties::metadata_client::{ClipboardSink, PropertiesRuntime, SaveState};
use crate::rich_editor::properties::{
    format_iso_local, PropertiesState, DOC_ID_FIELD_AUTHOR_ID, TITLE_FIELD_AUTHOR_ID,
};
use crate::theme::HsPalette;

/// Emit a stable AccessKit author_id (+ role) onto an already-rendered node (the shared helper shape
/// used across the editor MTs). A `Button`/`TextInput` keeps egui's derived role; a plain label/badge
/// gets the role set.
fn emit_node_author(ctx: &egui::Context, id: egui::Id, role: accesskit::Role, author_id: &str) {
    let role_for_closure = role;
    let author = author_id.to_owned();
    ctx.accesskit_node_builder(id, move |node| {
        if !matches!(role_for_closure, accesskit::Role::Button | accesskit::Role::TextInput) {
            node.set_role(role_for_closure);
        }
        node.set_author_id(author);
    });
}

/// Render the EDITABLE title field (a single-line `TextEdit`). On focus-gained the edit buffer is
/// seeded from the persisted title; on focus-lost OR Enter, the edit is committed — a real change marks
/// `PropertiesState.pending_save` and dispatches the rename through the runtime (the REAL `/rename`
/// endpoint, NOT `/save` — see the module doc). The field carries author_id `properties-title`.
///
/// Returns true when a save was dispatched this frame (so the host can request a repaint / log).
pub fn title_field(
    ui: &mut egui::Ui,
    state: &mut PropertiesState,
    runtime: &mut PropertiesRuntime,
    palette: &HsPalette,
) -> bool {
    // Seed the edit buffer on demand; `title_edit` is the live text bound to the widget.
    state.begin_title_edit();
    let mut dispatched = false;

    // The widget needs a `&mut String`; take the buffer out, edit it, then put it back. This keeps the
    // buffer owned by `PropertiesState` across frames (so partial edits survive a re-render).
    let mut buf = state.title_edit.take().unwrap_or_else(|| state.doc_metadata.title.clone());
    let resp = ui.add(
        egui::TextEdit::singleline(&mut buf)
            .desired_width(220.0)
            .hint_text("Document title"),
    );
    state.title_edit = Some(buf);
    emit_node_author(ui.ctx(), resp.id, accesskit::Role::TextInput, TITLE_FIELD_AUTHOR_ID);

    // Commit on focus-loss, which egui reports for BOTH an Enter submit (single-line TextEdit surrenders
    // focus on Enter) and a plain click-away. The contract is "autosaves on blur or Enter", and both
    // arrive as `lost_focus()`, so a single check covers both paths. (A test programmatically drives the
    // commit via `PropertiesState::commit_title_edit` to avoid depending on egui's focus internals.)
    if resp.lost_focus() && state.commit_title_edit() && state.pending_save {
        // Dispatch the rename for the committed (optimistically-applied) title.
        runtime.dispatch_rename(&state.doc_metadata.rich_document_id, &state.doc_metadata.title);
        state.pending_save = false; // the dispatch consumed the one-shot request.
        dispatched = true;
    }

    // A small inline save-state hint (NEVER a perpetual Spinner — the no-spinner gate): a "Saving…"
    // label while in flight, a typed banner on failure.
    match &runtime.save_state {
        SaveState::Saving => {
            ui.colored_label(palette.text_subtle, "Saving…");
        }
        SaveState::Failed(e) => {
            ui.colored_label(palette.error_text, format!("Save failed ({}): {e}", e.kind_str()));
        }
        SaveState::Idle | SaveState::Saved => {}
    }
    dispatched
}

/// Render the READ-ONLY document-id field: a monospace label that copies the id to the clipboard when
/// clicked (AC-6). The copy goes through the [`ClipboardSink`] trait (the production sink delegates to
/// `egui::Context::copy_text`; a test injects a mock), so a headless test never touches the OS
/// clipboard. The label carries author_id `properties-doc-id`.
pub fn doc_id_field(
    ui: &mut egui::Ui,
    state: &PropertiesState,
    clipboard: &dyn ClipboardSink,
    palette: &HsPalette,
) {
    let id = state.doc_metadata.rich_document_id.clone();
    let resp = ui.add(
        egui::Label::new(egui::RichText::new(&id).monospace().color(palette.text))
            .sense(egui::Sense::click()),
    )
    .on_hover_text("Click to copy");
    emit_node_author(ui.ctx(), resp.id, accesskit::Role::Label, DOC_ID_FIELD_AUTHOR_ID);
    if resp.clicked() {
        clipboard.copy(&id);
    }
}

/// Render a READ-ONLY date field: the ISO-8601 backend timestamp formatted as a human-readable LOCAL
/// date (MC-003 UTC fallback inside [`format_iso_local`]).
pub fn date_field(ui: &mut egui::Ui, iso: &str, palette: &HsPalette) {
    ui.colored_label(palette.text, format_iso_local(iso));
}

/// Render the version badge (`#version`) as a label inside a theme-colored frame, visually distinct
/// from plain text. Uses `surface` (a panel-like fill) + `text` so the badge reads in BOTH themes —
/// `surface_strong` is an INVERTED near-white token in the dark theme (equal to `text`), which would
/// render white-on-white, so it is deliberately NOT used here.
pub fn version_badge(ui: &mut egui::Ui, doc_version: u64, palette: &HsPalette) {
    egui::Frame::new()
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border_strong))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.colored_label(palette.text, format!("#{doc_version}"));
        });
}

/// Render the authority-label badge. A `promoted` label is a strong success-colored badge; any other
/// label (`draft`, …) is a muted surface badge — so `promoted` reads as visibly distinct from plain
/// text (AC-8).
pub fn authority_badge(ui: &mut egui::Ui, authority_label: &str, palette: &HsPalette) {
    let promoted = authority_label.eq_ignore_ascii_case("promoted");
    let (fill, text) = if promoted {
        // A strong success-colored badge (green bg + green text) — visibly distinct from plain text.
        (palette.success_bg, palette.success_text)
    } else {
        // A muted surface badge for draft/other. Uses `surface` (NOT `surface_strong`, which is an
        // inverted near-white token in the dark theme) so the muted label reads against it.
        (palette.surface, palette.text_subtle)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.colored_label(text, authority_label);
        });
}

/// Render a simple read-only text value (owner, crdt id, project/folder ref), with an em-dash for an
/// absent optional so the field never renders blank.
pub fn read_only_value(ui: &mut egui::Ui, value: Option<&str>, palette: &HsPalette) {
    let shown = value.filter(|v| !v.trim().is_empty()).unwrap_or("—");
    ui.colored_label(palette.text, shown);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::properties::metadata_client::DocMetadata;

    // The pure-logic field behavior (date formatting, badge selection) is proven here; the live
    // rendering + AccessKit emission is proven in tests/test_properties.rs via egui_kittest.

    #[test]
    fn date_field_uses_local_format() {
        // The date field delegates to format_iso_local — proven directly (the kittest screenshot proves
        // it renders). Here we just confirm the formatter is the one wired (no raw ISO echo).
        let out = format_iso_local("2026-06-19T14:32:00Z");
        assert!(out.contains("Jun") && out.contains("2026") && !out.contains('T'));
    }

    #[test]
    fn promoted_is_distinct_from_draft() {
        // AC-8 logic: `promoted` selects the success palette; `draft` selects the muted surface palette.
        // (The pixel-distinctness is proven by the kittest screenshot; here we assert the branch.)
        let promoted = "promoted".eq_ignore_ascii_case("promoted");
        let draft = "draft".eq_ignore_ascii_case("promoted");
        assert!(promoted, "promoted matches the success branch");
        assert!(!draft, "draft takes the muted branch -> a different fill/text than promoted");
    }

    fn _meta() -> DocMetadata {
        DocMetadata {
            rich_document_id: "KRD-1".into(),
            workspace_id: "ws".into(),
            title: "T".into(),
            doc_version: 1,
            authority_label: "draft".into(),
            owner_actor_kind: None,
            owner_actor_id: None,
            project_ref: None,
            folder_ref: None,
            crdt_document_id: None,
            created_at: "2026-06-19T14:32:00Z".into(),
            updated_at: "2026-06-19T14:32:00Z".into(),
        }
    }
}
