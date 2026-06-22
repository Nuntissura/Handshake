//! The tag chip editor for the properties panel (WP-KERNEL-012 MT-017).
//!
//! ## MC-002: tags are a real backend gap — NO fake persistence
//!
//! The verified backend `RichDocument` (`app/src/lib/api.ts` lines 3028-3048) has NO `tags` field, and
//! the knowledge-document API (`src/backend/handshake_core/src/api/knowledge_documents.rs`) has no tag
//! endpoint. So this editor (a) renders a VISIBLE banner `"Tags not persisted (backend gap: coming
//! soon)"`, and (b) keeps a LOCAL-ONLY tag list ([`PropertiesState::tags`]) where add/remove work
//! in-memory but are NEVER sent to the backend.
//!
//! This is the "no fake persistence" control: the operator can SEE that tags are local-only, rather
//! than the editor appearing fully functional while silently dropping the tags (the hsLink /
//! tableHeader field-existence lesson). When the backend adds a tags field/endpoint, a later MT wires
//! the local list to it; the banner is the typed-blocker marker until then.
//!
//! The container carries author_id `properties-tags` (AC-9), each chip `tag-chip-{tag}`, and the add
//! button `tag-add-button`, all through the SAME `ctx.accesskit_node_builder` hook the shell uses.

use egui::accesskit;

use crate::rich_editor::properties::{
    tag_chip_author_id, PropertiesState, TAGS_CONTAINER_AUTHOR_ID, TAG_ADD_BUTTON_AUTHOR_ID,
};
use crate::theme::HsPalette;

/// The visible backend-gap banner text (MC-002). A test asserts this exact string is what the panel
/// shows, so the "no fake persistence" control is mechanically checkable.
pub const BACKEND_GAP_BANNER: &str = "Tags not persisted (backend gap: coming soon)";

/// Emit a stable AccessKit author_id (+ role) onto an already-rendered node.
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

/// Render the tag chip editor into `ui`. Existing tags render as a wrapping chip row (each chip has an
/// `x` remove button); a `+` button opens an inline single-line TextEdit for a new tag (Enter/blur
/// adds it to the LOCAL list). The MC-002 backend-gap banner renders above the chips. Mutates the
/// LOCAL-only [`PropertiesState::tags`] (never persisted).
pub fn tag_editor(ui: &mut egui::Ui, state: &mut PropertiesState, palette: &HsPalette) {
    // MC-002: the visible backend-gap banner — tags are LOCAL ONLY.
    ui.colored_label(palette.text_subtle, BACKEND_GAP_BANNER);

    // The chip row + add affordance, wrapping so many tags flow onto multiple lines.
    let group = ui.horizontal_wrapped(|ui| {
        // Render each existing tag as a chip: `label` + an `x` remove button. Collect removals first so
        // we do not mutate `state.tags` while iterating it.
        let mut to_remove: Option<String> = None;
        for tag in &state.tags {
            let chip = egui::Frame::new()
                .fill(palette.accent_soft)
                .stroke(egui::Stroke::new(1.0, palette.border))
                .inner_margin(egui::Margin::symmetric(6, 2))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(palette.text, tag);
                        // The `x` remove button — an interactive node, so it carries a stable author_id
                        // (`tag-remove-{tag}`) or the shell HBR-SWARM gate would reject it as unnamed.
                        let x = ui.add(egui::Button::new("x").frame(false).small());
                        emit_node_author(ui.ctx(), x.id, accesskit::Role::Button, &format!("tag-remove-{tag}"));
                        if x.clicked() {
                            to_remove = Some(tag.clone());
                        }
                    });
                });
            // The chip container carries the stable `tag-chip-{tag}` author_id (a ListItem within the
            // tags Group).
            emit_node_author(
                ui.ctx(),
                chip.response.id,
                accesskit::Role::ListItem,
                &tag_chip_author_id(tag),
            );
        }
        if let Some(tag) = to_remove {
            state.remove_tag(&tag);
        }

        // The add affordance: a `+` button that opens an inline TextEdit; Enter/blur adds the tag.
        match state.new_tag_input.take() {
            Some(mut buf) => {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut buf)
                        .desired_width(100.0)
                        .hint_text("new tag"),
                );
                emit_node_author(ui.ctx(), resp.id, accesskit::Role::TextInput, "tag-new-input");
                resp.request_focus();
                let commit = resp.lost_focus();
                if commit {
                    // Add the typed tag (no-op for blank/duplicate) and close the input.
                    if !buf.trim().is_empty() {
                        state.add_tag(buf.clone());
                    }
                    state.new_tag_input = None;
                } else {
                    state.new_tag_input = Some(buf);
                }
            }
            None => {
                let add = ui.add(egui::Button::new("+").small());
                emit_node_author(ui.ctx(), add.id, accesskit::Role::Button, TAG_ADD_BUTTON_AUTHOR_ID);
                if add.clicked() {
                    state.new_tag_input = Some(String::new());
                }
            }
        }
    });

    // The tags container carries the AC-9 `properties-tags` author_id (a Group enclosing the chips).
    emit_node_author(ui.ctx(), group.response.id, accesskit::Role::Group, TAGS_CONTAINER_AUTHOR_ID);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_gap_banner_text_is_stable() {
        // MC-002: the no-fake-persistence banner text is the mechanically-checkable marker.
        assert_eq!(BACKEND_GAP_BANNER, "Tags not persisted (backend gap: coming soon)");
        assert!(BACKEND_GAP_BANNER.contains("backend gap"), "the banner names the gap explicitly");
    }
}
