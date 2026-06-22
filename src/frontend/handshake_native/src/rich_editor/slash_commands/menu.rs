//! The slash-command popup widget + the prompt modal (WP-KERNEL-012 MT-016).
//!
//! [`render_slash_menu`] paints the floating popup at the caret pixel: a scrollable list of
//! the filtered catalog, GROUPED by [`super::registry::SlashCategory`] with a section header
//! per group, keyboard-navigable (Up/Down move the selection, Enter executes, Escape cancels),
//! and click-to-execute. Each item row is an AccessKit `slash-item-{id}` node (Role::MenuItem)
//! and the popup container is the `slash-menu` node (Role::Menu) — the AC-6/AC-7 ids a swarm
//! agent drives.
//!
//! [`render_slash_prompt`] paints the embed/transclusion/manual-insert modal: a single
//! `egui::Window` with one text input + Ok/Cancel (the `context_menu_surfaces.rs` dialog
//! pattern), NOT a file picker (MT impl note 4). It is AccessKit-addressable
//! (`slash-prompt-dialog`/`-input`/`-ok`/`-cancel`).
//!
//! ## Ownership split (mirrors command_palette.rs)
//!
//! The widget owns NO editor state. It reads the live [`super::SlashMenuState`] + the filtered
//! catalog and returns a [`SlashMenuOutcome`]; the host (`rich_editor_widget`) matches on it and
//! routes selection/execution through the [`super::executor`] (the same "widget returns an
//! outcome, shell mutates state" pattern the command palette uses). Keyboard NAV is decided in
//! the widget (it owns the transient `selected` index in the passed-in state); the host drains
//! the key events out of the editor input path while the menu is open (the editor's chords are
//! claimed by the menu first), exactly as the wikilink autocomplete popup does.

use egui::accesskit;

use crate::theme::HsPalette;

use super::registry::{filter_slash_commands, SlashCategory, SlashCommand, SLASH_MENU_MAX_VISIBLE};
use super::{
    slash_item_author_id, SlashMenuState, SLASH_ITEM_ROLE, SLASH_MENU_AUTHOR_ID, SLASH_MENU_ROLE,
    SLASH_PROMPT_CANCEL_AUTHOR_ID, SLASH_PROMPT_DIALOG_AUTHOR_ID, SLASH_PROMPT_INPUT_AUTHOR_ID,
    SLASH_PROMPT_OK_AUTHOR_ID,
};

/// What the slash menu wants the host to do after a frame. The host (`rich_editor_widget`)
/// matches on it: `Execute` runs the command at the carried catalog index via the executor;
/// `Cancel` closes the menu leaving the `/` in the text (AC-5); `None` keeps it open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashMenuOutcome {
    /// Nothing decisive this frame; keep the menu open.
    None,
    /// Execute the command at this index into the CURRENTLY FILTERED list (Enter or a click).
    Execute(usize),
    /// Dismiss the menu without executing (Escape, backdrop click, or focus loss). The `/`
    /// stays in the text at the caret (AC-5).
    Cancel,
}

/// What the prompt modal wants the host to do. `Confirm` commits the typed input via the
/// executor's `confirm_prompt`; `Cancel` closes the modal (and the menu) without inserting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashPromptOutcome {
    /// Keep the modal open.
    None,
    /// Confirm the modal's current input (Ok button or Enter).
    Confirm,
    /// Cancel the modal (Cancel button or Escape).
    Cancel,
}

/// Read the menu's keyboard navigation keys (Up/Down/Enter/Escape) from this frame's input and
/// apply Up/Down to `state.selected` (clamped to the filtered length). Returns the decisive
/// outcome (Execute/Cancel) when Enter/Escape was pressed, else `None`. The host calls this
/// BEFORE rendering so Enter/Escape act on the selection computed from the current filter, and
/// so the editor never also sees these keys while the menu is open (the host removes them from
/// the editor input path — the same claim the wikilink autocomplete does).
pub fn handle_menu_keys(ctx: &egui::Context, state: &mut SlashMenuState) -> SlashMenuOutcome {
    let filtered = filter_slash_commands(&state.filter);
    let mut nav = 0i64;
    let mut enter = false;
    let mut escape = false;
    ctx.input(|i| {
        for ev in &i.events {
            if let egui::Event::Key { key, pressed: true, .. } = ev {
                match key {
                    egui::Key::ArrowDown => nav += 1,
                    egui::Key::ArrowUp => nav -= 1,
                    egui::Key::Enter => enter = true,
                    egui::Key::Escape => escape = true,
                    _ => {}
                }
            }
        }
    });

    if filtered.is_empty() {
        state.selected = 0;
    } else {
        let max = filtered.len() as i64 - 1;
        let cur = (state.selected as i64).min(max);
        state.selected = (cur + nav).clamp(0, max) as usize;
    }

    if escape {
        return SlashMenuOutcome::Cancel;
    }
    if enter && !filtered.is_empty() {
        return SlashMenuOutcome::Execute(state.selected);
    }
    SlashMenuOutcome::None
}

/// Render the slash-command popup at `caret_pixel` (a screen position just below the caret;
/// defaults to the editor's bottom-left when unavailable, e.g. an empty doc). Returns the
/// [`SlashMenuOutcome`] for a click; keyboard outcomes come from [`handle_menu_keys`] which the
/// host calls first. The popup is grouped by category with a section header per group; the
/// visible list is capped at [`SLASH_MENU_MAX_VISIBLE`] inside a scroll area.
pub fn render_slash_menu(
    ui: &mut egui::Ui,
    state: &SlashMenuState,
    palette: &HsPalette,
    caret_pixel: Option<egui::Pos2>,
) -> SlashMenuOutcome {
    let filtered = filter_slash_commands(&state.filter);
    let anchor = caret_pixel.unwrap_or_else(|| ui.max_rect().left_bottom());

    let mut clicked_index: Option<usize> = None;

    egui::Area::new(ui.id().with("slash-menu-area"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor + egui::vec2(0.0, 4.0))
        .show(ui.ctx(), |ui| {
            let frame = egui::Frame::popup(ui.style());
            let resp = frame
                .show(ui, |ui| {
                    ui.set_min_width(280.0);
                    ui.set_max_width(360.0);
                    if filtered.is_empty() {
                        ui.colored_label(palette.text_subtle, "No matching commands");
                        return;
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("slash-menu-scroll")
                        .max_height(row_height(ui) * SLASH_MENU_MAX_VISIBLE as f32)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut last_category: Option<SlashCategory> = None;
                            for (idx, cmd) in filtered.iter().enumerate() {
                                // Render a section header when the category changes (the list is
                                // already in catalog order, which is grouped by category, so a
                                // header appears once per contiguous category run).
                                if last_category != Some(cmd.category) {
                                    if last_category.is_some() {
                                        ui.add_space(2.0);
                                    }
                                    ui.label(
                                        egui::RichText::new(cmd.category.header())
                                            .small()
                                            .color(palette.text_subtle),
                                    );
                                    last_category = Some(cmd.category);
                                }
                                let selected = idx == state.selected;
                                let resp = slash_item_row(ui, cmd, selected, palette);
                                if resp.clicked() {
                                    clicked_index = Some(idx);
                                }
                            }
                        });
                })
                .response;

            // The popup container node (AC-6: `slash-menu`, Role::Menu). Emitted unconditionally
            // each open frame so a swarm agent always finds the modal by `slash-menu`.
            let menu_id = resp.id;
            ui.ctx().accesskit_node_builder(menu_id, |node| {
                node.set_role(SLASH_MENU_ROLE);
                node.set_author_id(SLASH_MENU_AUTHOR_ID.to_owned());
                node.set_label("Slash commands".to_owned());
            });
        });

    match clicked_index {
        Some(i) => SlashMenuOutcome::Execute(i),
        None => SlashMenuOutcome::None,
    }
}

/// Render one slash-menu item row: a full-width selectable button carrying the glyph + label +
/// (muted) description, tagged with its stable AccessKit author_id `slash-item-{id}` and
/// `Role::MenuItem` (AC-7). The selected row uses egui's selection fill.
fn slash_item_row(
    ui: &mut egui::Ui,
    cmd: &SlashCommand,
    selected: bool,
    palette: &HsPalette,
) -> egui::Response {
    let full_width = ui.available_width();
    let mut job = egui::text::LayoutJob::default();
    // Glyph (fixed-ish width) + bold label + muted description, one button -> one MenuItem node.
    job.append(
        &format!("{}  ", cmd.glyph),
        0.0,
        egui::TextFormat {
            color: palette.text_subtle,
            ..Default::default()
        },
    );
    job.append(
        cmd.label,
        0.0,
        egui::TextFormat {
            color: palette.text,
            ..Default::default()
        },
    );
    job.append(
        &format!("   {}", cmd.description),
        0.0,
        egui::TextFormat {
            color: palette.text_subtle,
            ..Default::default()
        },
    );

    let response = ui.add(
        egui::Button::selectable(selected, job)
            .truncate()
            .min_size(egui::vec2(full_width, 0.0)),
    );

    // Attach the stable author_id + MenuItem role to the SAME live node egui built for the row
    // (egui derived Role + Action::Click from the Button's Sense). This adds the out-of-process
    // address while keeping egui's interactive role/actions intact — the command_palette pattern.
    let author = slash_item_author_id(cmd.id);
    let label = cmd.label.to_owned();
    ui.ctx().accesskit_node_builder(response.id, move |node| {
        node.set_role(SLASH_ITEM_ROLE);
        node.set_author_id(author);
        node.set_label(label);
        if selected {
            node.set_selected(true);
        }
    });

    response
}

/// The approximate per-row height used to cap the scroll area at [`SLASH_MENU_MAX_VISIBLE`]
/// rows (a soft bound; the scroll area handles overflow).
fn row_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Body)) + 4.0
}

/// Render the embed/transclusion/manual-insert prompt modal (a centred `egui::Window` with one
/// text input + Ok/Cancel). Mutates `input` in place (the operator's typed value) and returns
/// the [`SlashPromptOutcome`]. The dialog + its three interactive controls are AccessKit-named.
pub fn render_slash_prompt(
    ctx: &egui::Context,
    title: &str,
    hint: &str,
    input: &mut String,
) -> SlashPromptOutcome {
    let mut outcome = SlashPromptOutcome::None;

    // Enter confirms / Escape cancels (read before the window so they act this frame).
    ctx.input(|i| {
        for ev in &i.events {
            if let egui::Event::Key { key, pressed: true, .. } = ev {
                match key {
                    egui::Key::Enter => outcome = SlashPromptOutcome::Confirm,
                    egui::Key::Escape => outcome = SlashPromptOutcome::Cancel,
                    _ => {}
                }
            }
        }
    });

    let dialog_egui_id = egui::Id::new("slash-prompt.window");
    egui::Window::new("slash_prompt")
        .id(dialog_egui_id)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new(title).heading());
            ui.add_space(6.0);
            let edit = egui::TextEdit::singleline(input)
                .hint_text(hint)
                .desired_width(320.0)
                .id(egui::Id::new("slash-prompt.input"));
            let edit_resp = ui.add(edit);
            // Name the input field (egui derived Role::TextInput + actions; add the address).
            crate::accessibility::emit_interactive_node(
                ui.ctx(),
                edit_resp.id,
                SLASH_PROMPT_INPUT_AUTHOR_ID,
            );
            // Focus the input on first show so the operator can type immediately.
            if edit_resp.lost_focus() {
                // no-op; keep focus handling minimal (the field requests focus below once).
            }
            edit_resp.request_focus();

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ok = ui.button("Insert");
                crate::accessibility::emit_interactive_node(ui.ctx(), ok.id, SLASH_PROMPT_OK_AUTHOR_ID);
                if ok.clicked() {
                    outcome = SlashPromptOutcome::Confirm;
                }
                let cancel = ui.button("Cancel");
                crate::accessibility::emit_interactive_node(
                    ui.ctx(),
                    cancel.id,
                    SLASH_PROMPT_CANCEL_AUTHOR_ID,
                );
                if cancel.clicked() {
                    outcome = SlashPromptOutcome::Cancel;
                }
            });
        });

    // The dialog root container node (Role::Dialog, modal) addressable by `slash-prompt-dialog`.
    ctx.accesskit_node_builder(dialog_egui_id, |node| {
        node.set_role(accesskit::Role::Dialog);
        node.set_author_id(SLASH_PROMPT_DIALOG_AUTHOR_ID.to_owned());
        node.set_modal();
    });

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_variants_are_comparable() {
        assert_eq!(SlashMenuOutcome::Execute(2), SlashMenuOutcome::Execute(2));
        assert_ne!(SlashMenuOutcome::Execute(2), SlashMenuOutcome::Cancel);
        assert_eq!(SlashPromptOutcome::Confirm, SlashPromptOutcome::Confirm);
    }
}
