//! The rich-text editor toolbar widget (WP-KERNEL-012 MT-013).
//!
//! [`EditorToolbar`] renders a SINGLE horizontal row of glyph-labelled buttons grouped
//! by category (history | format | block | list | table), matching the React
//! `RichTextEditor.tsx` toolbar. Each button:
//! - shows a text glyph (`B`, `I`, `H1`, …) — NOT an icon asset (MT scope: text glyphs
//!   are sufficient; icon assets are a later polish MT);
//! - highlights (egui selection background) when its command is ACTIVE at the caret
//!   (e.g. bold when the cursor is inside bold text) via [`super::commands::is_mark_active`]
//!   and node-kind inspection;
//! - carries a stable AccessKit `author_id = format!("toolbar-btn-{command_id}")` with
//!   `Role::Button` + `Action::Click` (egui derives those from the button's `Sense`; we
//!   attach the author_id through the SAME `crate::accessibility::emit_interactive_node`
//!   hook the shell uses) so a swarm agent activates a button by stable id;
//! - dispatches its [`FormattingCommand`] through [`super::commands::dispatch`] STANDALONE
//!   on the local editor state (the WP-011 command_registry/event_bus host Sender is NOT
//!   wired into the editor until E11/MT-069 — routing through the bus is additive, never
//!   the only path; COMMAND DISPATCH REALITY gate).
//!
//! ## Overflow (red-team RISK-5 / MC-005)
//!
//! When the available width is narrower than the full button row, a trailing `…`
//! overflow button opens an `egui::popup_below_widget` keyed by a STABLE id
//! (`Id::new("toolbar-overflow")`) so the popup does not close on the next frame from a
//! fresh id (the close-on-next-frame bug MC-005 warns about). The popup lists the
//! remaining commands as full-width buttons.
//!
//! ## Theme (CONTROL-4: no hardcoded hex)
//!
//! The active-button highlight uses egui's resolved selection color
//! (`ui.visuals().selection.bg_fill`), itself seeded from the Handshake palette, so the
//! toolbar carries no literal color. The button glyphs use the ambient text color.

use egui::Id;

use crate::accessibility;
use crate::rich_editor::document_model::node::{BlockNode, Mark, NodeKind};
use crate::rich_editor::document_model::selection::Selection;

use super::commands::{self, CommandContext, FormattingCommand};

/// The stable egui/AccessKit id base for the overflow popup (MC-005: a FIXED id so the
/// popup survives across frames instead of closing on a fresh-id frame).
const OVERFLOW_POPUP_ID: &str = "toolbar-overflow";

/// The author_id prefix for a toolbar button (`toolbar-btn-{command_id}`), per the MT
/// scope. The AC asserts `toolbar-btn-toggle_bold` is present in the AccessKit tree.
pub const TOOLBAR_BTN_AUTHOR_PREFIX: &str = "toolbar-btn-";

/// The author_id of a toolbar button for `cmd` (`toolbar-btn-{command_id}`).
pub fn toolbar_button_author_id(cmd: &FormattingCommand) -> String {
    format!("{TOOLBAR_BTN_AUTHOR_PREFIX}{}", cmd.command_id())
}

/// One toolbar button spec: its command, the glyph label shown on the button, and a
/// human/model label for tooltips + AccessKit.
#[derive(Clone)]
struct ButtonSpec {
    cmd: FormattingCommand,
    glyph: &'static str,
    label: &'static str,
}

impl ButtonSpec {
    fn new(cmd: FormattingCommand, glyph: &'static str, label: &'static str) -> Self {
        Self { cmd, glyph, label }
    }
}

/// The ordered toolbar groups (history | format | block | list | table), each a list of
/// button specs. This is the single source the row + the overflow popup both render, so
/// the two surfaces cannot drift. Glyphs are the MT scope's suggested text labels.
fn toolbar_groups() -> Vec<Vec<ButtonSpec>> {
    vec![
        // history. Use ASCII-safe glyphs that the bundled Inter font always has (the
        // fancier ↶/↷ arrows render as tofu boxes in Inter — a legibility regression, not
        // a functional one, but easily avoided). Icon assets are a later polish MT.
        vec![
            ButtonSpec::new(FormattingCommand::Undo, "Undo", "Undo"),
            ButtonSpec::new(FormattingCommand::Redo, "Redo", "Redo"),
        ],
        // format (inline marks)
        vec![
            ButtonSpec::new(FormattingCommand::ToggleBold, "B", "Bold"),
            ButtonSpec::new(FormattingCommand::ToggleItalic, "I", "Italic"),
            ButtonSpec::new(FormattingCommand::ToggleUnderline, "U", "Underline"),
            ButtonSpec::new(FormattingCommand::ToggleStrike, "S", "Strikethrough"),
            ButtonSpec::new(FormattingCommand::ToggleCode, "`", "Inline code"),
        ],
        // block
        vec![
            ButtonSpec::new(FormattingCommand::SetHeading(1), "H1", "Heading 1"),
            ButtonSpec::new(FormattingCommand::SetHeading(2), "H2", "Heading 2"),
            ButtonSpec::new(FormattingCommand::SetHeading(3), "H3", "Heading 3"),
            ButtonSpec::new(FormattingCommand::SetParagraph, "¶", "Paragraph"),
            ButtonSpec::new(FormattingCommand::SetBlockquote, "\"", "Block quote"),
            ButtonSpec::new(FormattingCommand::SetCodeBlock(None), "</>", "Code block"),
            ButtonSpec::new(FormattingCommand::InsertHorizontalRule, "—", "Horizontal rule"),
        ],
        // list
        vec![
            ButtonSpec::new(FormattingCommand::ToggleBulletList, "•", "Bullet list"),
            ButtonSpec::new(FormattingCommand::ToggleOrderedList, "1.", "Numbered list"),
            ButtonSpec::new(FormattingCommand::ToggleTaskList, "☑", "Task list"),
            ButtonSpec::new(FormattingCommand::SinkListItem, "→", "Indent"),
            ButtonSpec::new(FormattingCommand::LiftListItem, "←", "Outdent"),
        ],
        // table
        vec![
            ButtonSpec::new(FormattingCommand::InsertTable { rows: 3, cols: 3 }, "Table", "Insert table"),
            ButtonSpec::new(FormattingCommand::AddRowAfter, "+R", "Add row"),
            ButtonSpec::new(FormattingCommand::DeleteRow, "-R", "Delete row"),
            ButtonSpec::new(FormattingCommand::AddColAfter, "+C", "Add column"),
            ButtonSpec::new(FormattingCommand::DeleteCol, "-C", "Delete column"),
            ButtonSpec::new(FormattingCommand::ToggleHeaderRow, "TH", "Toggle header row"),
            ButtonSpec::new(FormattingCommand::DeleteTable, "✕T", "Delete table"),
        ],
    ]
}

/// The flat list of every toolbar command (group order), used by AccessKit-tree tests
/// and the overflow popup enumeration.
pub fn all_toolbar_commands() -> Vec<FormattingCommand> {
    toolbar_groups()
        .into_iter()
        .flat_map(|g| g.into_iter().map(|b| b.cmd))
        .collect()
}

/// The toolbar widget. It borrows the live editor state by `&mut` (doc, undo, selection)
/// so a button click dispatches a command directly on it. The widget does NOT own the
/// document (MT impl note 4).
pub struct EditorToolbar<'a> {
    ctx: CommandContext<'a>,
    /// Optional forced max width for the row (kittest narrow-window simulation). When
    /// `Some`, the toolbar uses this instead of `ui.available_width()` to decide overflow
    /// (so the overflow button appears deterministically in the test).
    forced_max_width: Option<f32>,
}

impl<'a> EditorToolbar<'a> {
    /// Build the toolbar over the borrowed editor state.
    pub fn new(ctx: CommandContext<'a>) -> Self {
        Self {
            ctx,
            forced_max_width: None,
        }
    }

    /// Force the row's max width (kittest narrow-window simulation). Below the full
    /// button-row width this forces the overflow `…` button to appear (AC-11).
    pub fn with_forced_max_width(mut self, width: f32) -> Self {
        self.forced_max_width = Some(width);
        self
    }

    /// Render the toolbar row into `ui`. Returns `true` when a command was dispatched
    /// this frame (so the caller can request a repaint / mark the doc dirty).
    pub fn show(mut self, ui: &mut egui::Ui) -> bool {
        let mut dispatched = false;
        let groups = toolbar_groups();
        let max_width = self.forced_max_width.unwrap_or_else(|| ui.available_width());

        // Estimate the row width: a rough per-button width budget. egui sizes buttons by
        // their galley; we use a conservative fixed estimate so the overflow decision is
        // deterministic in a kittest (where the real galley width is font-dependent).
        let approx_btn_w = 34.0_f32;
        let group_gap = 10.0_f32;
        let total_buttons: usize = groups.iter().map(|g| g.len()).sum();
        let approx_row_w =
            total_buttons as f32 * approx_btn_w + groups.len() as f32 * group_gap + 40.0;
        let overflow = approx_row_w > max_width;

        // How many leading buttons fit before overflow (a simple greedy budget). When
        // overflowing, the first `fit_count` buttons render inline and the rest go to the
        // popup; when not overflowing, every button renders inline.
        let fit_count = if overflow {
            ((max_width - 40.0).max(0.0) / approx_btn_w) as usize
        } else {
            total_buttons
        };

        ui.horizontal(|ui| {
            let mut rendered = 0usize;
            for (gi, group) in groups.iter().enumerate() {
                if gi > 0 && rendered < fit_count {
                    ui.add_space(group_gap);
                    ui.separator();
                }
                for spec in group {
                    if rendered >= fit_count {
                        break;
                    }
                    if self.button(ui, spec) {
                        dispatched = true;
                    }
                    rendered += 1;
                }
                if rendered >= fit_count {
                    break;
                }
            }

            // Overflow `…` button + popup with the remaining commands. The popup is built
            // with egui 0.33's `Popup::menu`, whose id derives deterministically from the
            // button's response id (`Popup::default_response_id`, i.e. `resp.id.with("popup")`)
            // — a STABLE id across frames, so the popup survives the next-frame repaint
            // instead of closing on a fresh id (MC-005). (The MT contract names the older
            // `popup_below_widget` + `Id::new("toolbar-overflow")`, but that API is deprecated
            // in egui 0.33 and the rest of this crate has migrated to `egui::Popup`; the
            // stable-id requirement is preserved by the response-derived popup id. The
            // OVERFLOW_POPUP_ID salt below keeps the `…` button's own egui id stable too.)
            if overflow {
                let resp = ui
                    .add(egui::Button::new("…"))
                    .on_hover_text("More formatting commands");
                // Stable author_id for the overflow control so a swarm agent can open it.
                accessibility::emit_interactive_node(ui.ctx(), resp.id, "toolbar-btn-overflow");
                egui::Popup::menu(&resp)
                    .id(Id::new(OVERFLOW_POPUP_ID))
                    .show(|ui| {
                        ui.set_min_width(160.0);
                        // The remaining (overflowed) commands as full-width rows.
                        let flat = all_toolbar_commands();
                        for cmd in flat.into_iter().skip(rendered) {
                            let label = command_menu_label(&cmd);
                            let btn = ui.button(label);
                            accessibility::emit_interactive_node(
                                ui.ctx(),
                                btn.id,
                                &format!("toolbar-overflow-{}", cmd.command_id()),
                            );
                            if btn.clicked() {
                                let _ = commands::dispatch(&mut self.ctx, &cmd);
                                dispatched = true;
                            }
                        }
                    });
            }
        });

        dispatched
    }

    /// Render one toolbar button: glyph label, active-state highlight, AccessKit
    /// author_id, and dispatch-on-click. Returns `true` when clicked + dispatched.
    fn button(&mut self, ui: &mut egui::Ui, spec: &ButtonSpec) -> bool {
        let active = is_command_active(self.ctx.doc, self.ctx.selection, &spec.cmd);
        let btn = egui::Button::new(spec.glyph).selected(active);
        let resp = ui.add(btn).on_hover_text(spec.label);
        // Attach the stable author_id to the (already interactive) button node — egui
        // derives Role::Button + Action::Click from the Button's Sense; we add only the
        // address, through the SAME hook the shell uses (HBR-SWARM).
        let author = toolbar_button_author_id(&spec.cmd);
        accessibility::emit_interactive_node(ui.ctx(), resp.id, &author);
        if resp.clicked() {
            // STANDALONE dispatch on the local editor state (COMMAND DISPATCH REALITY
            // gate: the host bus Sender is E11/MT-069 — routing through it is additive).
            let _ = commands::dispatch(&mut self.ctx, &spec.cmd);
            true
        } else {
            false
        }
    }
}

/// The label shown for a command in the overflow popup (the human label + glyph).
fn command_menu_label(cmd: &FormattingCommand) -> String {
    for group in toolbar_groups() {
        for spec in group {
            if &spec.cmd == cmd {
                return format!("{}  {}", spec.glyph, spec.label);
            }
        }
    }
    cmd.command_id().to_string()
}

/// True when a command's mark/node should render as ACTIVE (selected) at the current
/// caret. Inline marks use the full-range active check (RISK-1 / MC-001); block-kind
/// commands inspect the caret block's kind; list/table commands check the enclosing
/// container kind. Commands with no meaningful active state (undo/redo, structural)
/// return `false`.
pub fn is_command_active(doc: &BlockNode, selection: &Selection, cmd: &FormattingCommand) -> bool {
    match cmd {
        FormattingCommand::ToggleBold => commands::is_mark_active(doc, selection, &Mark::Bold),
        FormattingCommand::ToggleItalic => commands::is_mark_active(doc, selection, &Mark::Italic),
        FormattingCommand::ToggleUnderline => {
            commands::is_mark_active(doc, selection, &Mark::Underline)
        }
        FormattingCommand::ToggleStrike => commands::is_mark_active(doc, selection, &Mark::Strike),
        FormattingCommand::ToggleCode => commands::is_mark_active(doc, selection, &Mark::Code),
        FormattingCommand::SetParagraph => caret_block_kind_is(doc, selection, |k| matches!(k, NodeKind::Paragraph)),
        FormattingCommand::SetHeading(level) => caret_block_kind_is(doc, selection, |k| {
            matches!(k, NodeKind::Heading(l) if l.get() == *level)
        }),
        FormattingCommand::SetBlockquote => caret_is_inside_kind(doc, selection, NodeKind::Blockquote),
        FormattingCommand::SetCodeBlock(_) => caret_block_kind_is(doc, selection, |k| matches!(k, NodeKind::CodeBlock)),
        FormattingCommand::ToggleBulletList => caret_is_inside_kind(doc, selection, NodeKind::BulletList),
        FormattingCommand::ToggleOrderedList => caret_is_inside_kind(doc, selection, NodeKind::OrderedList),
        FormattingCommand::ToggleTaskList => caret_is_inside_kind(doc, selection, NodeKind::TaskItem),
        _ => false,
    }
}

/// True when the caret's immediate block kind satisfies `pred`.
fn caret_block_kind_is(
    doc: &BlockNode,
    selection: &Selection,
    pred: impl Fn(NodeKind) -> bool,
) -> bool {
    let Some(block_path) = caret_block_path(selection) else {
        return false;
    };
    block_at(doc, &block_path).map(|b| pred(b.kind)).unwrap_or(false)
}

/// True when any ancestor of the caret block is of `kind`.
fn caret_is_inside_kind(doc: &BlockNode, selection: &Selection, kind: NodeKind) -> bool {
    let Some(block_path) = caret_block_path(selection) else {
        return false;
    };
    let mut node = doc;
    if node.kind == kind {
        return true;
    }
    for &idx in &block_path {
        let Some(next) = node.children.get(idx).and_then(|c| c.as_block()) else {
            return false;
        };
        node = next;
        if node.kind == kind {
            return true;
        }
    }
    false
}

/// The caret's block path (head leaf path minus the final leaf index).
fn caret_block_path(selection: &Selection) -> Option<Vec<usize>> {
    match selection {
        Selection::Text { head, .. } if !head.path.is_empty() => {
            Some(head.path[..head.path.len() - 1].to_vec())
        }
        _ => None,
    }
}

/// Resolve a block path to a shared block reference.
fn block_at<'a>(doc: &'a BlockNode, path: &[usize]) -> Option<&'a BlockNode> {
    let mut node = doc;
    for &idx in path {
        node = node.children.get(idx)?.as_block()?;
    }
    Some(node)
}
