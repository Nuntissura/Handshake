//! Command Palette overlay for the native Handshake shell (WP-KERNEL-011 MT-016).
//!
//! ## What this provides (no-context model navigation — HBR-VIS)
//!
//! A modal, centred, always-on-top floating panel (the classic Ctrl+Shift+P command palette): a search
//! text input, a filtered scrollable list of commands, keyboard navigation (ArrowUp/ArrowDown/Enter/
//! Escape), and execute-on-Enter or click. It is a direct port of `app/src/components/CommandPalette.tsx`
//! over the static [`crate::command_registry`] catalog.
//!
//! ## Ownership split (mirrors the MT-015 menu bar)
//!
//! The palette widget owns ONLY its small transient UI state (the query string + the selected row
//! index), stored in egui persistent memory keyed to the palette id and RESET each time the palette
//! re-opens (red-team MC1 — keyed by a monotonic `open_count`). It NEVER mutates app state: [`show`]
//! returns a [`PaletteOutcome`] and the shell ([`crate::app`]) matches on it and routes the command id
//! into the existing state-mutation paths (the same pattern the menu bar uses with `MenuBarAction`).
//! The shell owns the `command_palette_open` flag; the palette only REQUESTS close via the outcome.
//!
//! ## Stable AccessKit ids (out-of-process steering — HBR-VIS)
//!
//! The palette has three FIXED container nodes in a fresh disjoint band (11..=13, below the pane id
//! base 100 and disjoint from every other declared identity — the registry collision test proves it):
//! - the dialog root ([`PALETTE_DIALOG_NODE_ID`] = 11, Role::Dialog, modal),
//! - the search box ([`PALETTE_SEARCH_NODE_ID`] = 12, Role::TextInput),
//! - the list container ([`PALETTE_LIST_NODE_ID`] = 13, Role::ListBox).
//!
//! Each command ROW is DYNAMIC (the count varies with the filter), so — like the per-tab and menu-leaf
//! nodes — it lives in egui's hashed id space, addressed by a stable author_id STRING
//! (`command-palette.option.{stable_id}`, Role::ListBoxOption), not a fixed-band NodeId. Every row
//! carries its author_id so it is discoverable/clickable out-of-process and never trips the MT-025
//! interactive-naming gate. The three fixed container ids ARE enumerated in `DECLARED_IDENTITIES`.

use egui::accesskit;

use crate::accessibility::{emit_interactive_node, PALETTE_AUTHOR_IDS};
use crate::command_registry::{filtered_commands, AppCommand};

/// Fixed AccessKit/egui `NodeId` of the palette DIALOG root (Role::Dialog, modal). Fresh band slot 11:
/// above the theme toggle (10), below the chrome title bar (20) and the pane id base (100). A
/// fixed-value `egui::Id` (`from_high_entropy_bits`) yields a fixed `NodeId` across frames + restarts —
/// the same convention every other fixed-band node in this crate uses.
pub const PALETTE_DIALOG_NODE_ID: u64 = 11;
/// Fixed AccessKit/egui `NodeId` of the palette SEARCH box (Role::TextInput). Fresh band slot 12.
pub const PALETTE_SEARCH_NODE_ID: u64 = 12;
/// Fixed AccessKit/egui `NodeId` of the palette LIST container (Role::ListBox). Fresh band slot 13.
pub const PALETTE_LIST_NODE_ID: u64 = 13;

/// Stable out-of-process author_id for the palette dialog root.
pub const PALETTE_DIALOG_AUTHOR_ID: &str = "command-palette.dialog";
/// Stable out-of-process author_id for the palette search box.
pub const PALETTE_SEARCH_AUTHOR_ID: &str = "command-palette.search";
/// Stable out-of-process author_id for the palette list container.
pub const PALETTE_LIST_AUTHOR_ID: &str = "command-palette.list";
/// Stable out-of-process author_id for the palette header Close button. The button lives in egui's
/// hashed id space (it has no fixed `DeclaredIdentity` slot, like the command rows and the settings
/// Close button), so it is addressed by this author_id via `emit_interactive_node` rather than a fixed
/// registry NodeId. Without it the Close button is an interactive control with no stable address — the
/// gap the MT-029 overlay accessibility-invariant proof surfaces.
pub const PALETTE_CLOSE_AUTHOR_ID: &str = "command-palette.close";

/// The author_id prefix for a command ROW (a `ListBoxOption`). Each row's full author_id is
/// `{ROW_AUTHOR_ID_PREFIX}{cmd.stable_id}`, in egui's hashed id space (dynamic count).
pub const ROW_AUTHOR_ID_PREFIX: &str = "command-palette.option.";

/// What the palette wants the shell to do after a frame.
///
/// Returned by [`show`]. The shell matches on it: `Run` dispatches the command id into the existing
/// state-mutation paths AND closes the palette; `Close` just clears the open flag; `None` leaves the
/// palette open for the next frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteOutcome {
    /// Nothing happened this frame; keep the palette open.
    None,
    /// The user picked an enabled command (Enter on the selection or a click). Carries the command id
    /// the shell dispatches; the shell also closes the palette.
    Run(String),
    /// The user dismissed the palette (Escape, the Close button, or a backdrop click) without running
    /// a command. The shell clears the open flag.
    Close,
}

/// Transient per-open palette UI state: the query text and the selected row index. Stored in egui
/// persistent memory keyed to the palette id, and RESET when [`PaletteState::open_count`] changes (each
/// time the palette re-opens) so a re-open never shows the previous session's query/selection (R1/MC1).
#[derive(Debug, Clone, Default)]
struct PaletteState {
    /// The open generation this state was initialized for. When the shell's `open_count` differs, the
    /// state is reset (query cleared, selection back to 0) before use.
    open_count: u64,
    /// The current search query.
    query: String,
    /// The selected row index into the CURRENT filtered list (clamped before use each frame).
    selected_index: usize,
    /// Set once after a re-open so the search box is focused on the first frame only (so the focus
    /// request does not fight the user clicking a row on later frames).
    focus_requested: bool,
}

/// Render the command palette overlay and return the [`PaletteOutcome`] for this frame.
///
/// `open_count` is a monotonic counter the shell increments each time `command_palette_open` flips to
/// `true`; the palette resets its transient state whenever it sees a new value (R1/MC1). The palette is
/// rendered as a backdrop [`egui::Area`] (full-screen, behind the panel, catches click-to-dismiss) plus
/// a centred [`egui::Window`] with the title bar hidden — both on the `Foreground` order so the palette
/// sits above the workspace (AC10) but below a higher overlay the shell renders later (settings).
pub fn show(ctx: &egui::Context, open_count: u64, editor_available: bool) -> PaletteOutcome {
    let state_id = egui::Id::new("command-palette.state");
    let mut state: PaletteState = ctx
        .data_mut(|d| d.get_temp::<PaletteState>(state_id))
        .unwrap_or_default();

    // Reset transient state on (re-)open: a new open generation clears the query + selection so a
    // re-open never shows the previous session's text (red-team R1 / MC1).
    if state.open_count != open_count {
        state = PaletteState {
            open_count,
            query: String::new(),
            selected_index: 0,
            focus_requested: false,
        };
    }

    // ── Keyboard navigation: read key events BEFORE rendering the list so Enter/arrows act on the
    //    selection computed from THIS frame's query (the contract's "capture Key events first"). The
    //    text input still receives the typed characters egui routes to the focused widget. ──
    let mut outcome = PaletteOutcome::None;
    let mut nav_down = 0i64;
    let mut escape = false;
    let mut enter = false;
    ctx.input(|i| {
        for event in &i.events {
            if let egui::Event::Key {
                key, pressed: true, ..
            } = event
            {
                match key {
                    egui::Key::ArrowDown => nav_down += 1,
                    egui::Key::ArrowUp => nav_down -= 1,
                    egui::Key::Escape => escape = true,
                    egui::Key::Enter => enter = true,
                    _ => {}
                }
            }
        }
    });

    // Compute the filtered list for this frame's query (catalog order).
    let filtered: Vec<&'static AppCommand> = filtered_commands(&state.query);

    // Apply arrow navigation, clamped to the current filtered range.
    if !filtered.is_empty() {
        let max = filtered.len() - 1;
        let cur = state.selected_index.min(max) as i64;
        let next = (cur + nav_down).clamp(0, max as i64);
        state.selected_index = next as usize;
    } else {
        state.selected_index = 0;
    }

    if escape {
        // Escape dismisses without running (AC5). Persist nothing — the shell will close + the next
        // open resets via open_count anyway.
        persist(ctx, state_id, &state);
        return PaletteOutcome::Close;
    }

    // Enter runs the selected ENABLED command (AC4 / AC7: disabled rows are not runnable). MT-069: an
    // EditorMenu command is enabled only when an editor pane is the focusable target (the live predicate),
    // so the effective-disabled gate — not the static flag — decides runnability.
    if enter {
        if let Some(cmd) = filtered.get(state.selected_index) {
            if !crate::command_registry::effective_disabled(cmd, editor_available) {
                outcome = PaletteOutcome::Run(cmd.id.to_owned());
            }
        }
    }

    // ── Backdrop: a full-screen interactable Area BEHIND the window. A click on the backdrop (i.e. not
    //    on the window) dismisses the palette (AC: backdrop click closes). Registered first so it sits
    //    below the window in z-order (red-team R3). ──
    let screen = ctx.content_rect();
    let backdrop = egui::Area::new(egui::Id::new("command-palette.backdrop"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(96));
            response
        });
    if backdrop.inner.clicked() {
        persist(ctx, state_id, &state);
        return PaletteOutcome::Close;
    }

    // ── The palette window: centred, fixed size, no title bar, always-on-top (above the backdrop). ──
    let search_egui_id = unsafe { egui::Id::from_high_entropy_bits(PALETTE_SEARCH_NODE_ID) };
    let dialog_egui_id = unsafe { egui::Id::from_high_entropy_bits(PALETTE_DIALOG_NODE_ID) };
    let list_egui_id = unsafe { egui::Id::from_high_entropy_bits(PALETTE_LIST_NODE_ID) };

    let mut close_clicked = false;
    let mut clicked_command: Option<String> = None;
    let mut hovered_index: Option<usize> = None;

    egui::Window::new("command_palette")
        .id(egui::Id::new("command-palette.window"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([580.0, 400.0])
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // Header row: eyebrow + title on the left, Close button on the right.
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("AI ACTIONS").small().weak());
                    ui.label(egui::RichText::new("Command Palette").heading());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.button("Close");
                    // Tag the Close button with its stable author_id so it is a NAMED interactive
                    // control out-of-process (egui already derived Role::Button + Click/Focus actions).
                    emit_interactive_node(ui.ctx(), close.id, PALETTE_CLOSE_AUTHOR_ID);
                    if close.clicked() {
                        close_clicked = true;
                    }
                });
            });
            ui.add_space(6.0);

            // Search input, pinned to the fixed search id so its AccessKit NodeId is stable. Request
            // focus on the first frame after (re-)open only.
            let edit = egui::TextEdit::singleline(&mut state.query)
                .id(search_egui_id)
                .hint_text("Search actions...")
                .desired_width(f32::INFINITY);
            let edit_response = ui.add(edit);
            if edit_response.changed() {
                // Reset the selection to the top whenever the query changes (React parity: onChange ->
                // setSelectedIndex(0)).
                state.selected_index = 0;
            }
            // Keep requesting until focus is actually observed. Marking the latch before focus settles
            // can lose the first-frame request in egui/kittest and leaves the palette open without a
            // focused search box.
            if !state.focus_requested || !edit_response.has_focus() {
                edit_response.request_focus();
            }
            if edit_response.has_focus() {
                state.focus_requested = true;
            }
            // Tag the search node with its stable author_id + a TextInput role (egui already derived the
            // interactive role/actions for the TextEdit; this only adds the address — like the toggle).
            emit_search_node(ui.ctx(), search_egui_id);

            ui.add_space(6.0);

            // Recompute the filtered list AFTER the text edit so a character typed this frame is
            // reflected in the rows + the Enter/selection math the shell reads next frame.
            let rows: Vec<&'static AppCommand> = filtered_commands(&state.query);
            let sel = state.selected_index.min(rows.len().saturating_sub(1));

            // List container node (Role::ListBox) reserved at the fixed list id so it carries a stable
            // address; rendered as a vertical scroll area of selectable rows.
            ui.push_id(list_egui_id, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if rows.is_empty() {
                            ui.label(egui::RichText::new("No matching actions.").weak());
                        }
                        for (idx, cmd) in rows.iter().enumerate() {
                            let is_selected = idx == sel && !rows.is_empty();
                            // MT-069: the row's runnable/disabled state is the LIVE effective predicate
                            // (an EditorMenu command needs an editor pane available), not just the static
                            // catalog flag — so a stale-state editor row is honestly greyed (RISK-006).
                            let row_disabled =
                                crate::command_registry::effective_disabled(cmd, editor_available);
                            let resp = command_row(ui, cmd, is_selected, row_disabled);
                            if resp.hovered() {
                                hovered_index = Some(idx);
                            }
                            if resp.clicked() && !row_disabled {
                                clicked_command = Some(cmd.id.to_owned());
                            }
                        }
                    });
            });
            // Emit the ListBox container node (own role + author_id) — it has no egui-derived widget
            // info, so we set role + author_id + label fully.
            emit_list_node(ui.ctx(), list_egui_id);
        });

    // The dialog root container node (Role::Dialog, modal). The Window's own id is internal; we attach
    // the modal dialog identity to the fixed dialog id so an out-of-process model finds the modal by
    // `command-palette.dialog`. Emitted unconditionally each open frame.
    emit_dialog_node(ctx, dialog_egui_id);

    // Hovering a row updates the selection (React parity: onMouseEnter -> setSelectedIndex(idx)).
    if let Some(h) = hovered_index {
        state.selected_index = h;
    }

    // Resolve the frame's outcome precedence: an explicit click/Enter Run wins, then Close, then None.
    if outcome == PaletteOutcome::None {
        if let Some(id) = clicked_command {
            outcome = PaletteOutcome::Run(id);
        } else if close_clicked {
            outcome = PaletteOutcome::Close;
        }
    }

    persist(ctx, state_id, &state);
    outcome
}

/// Persist the transient palette state back into egui memory.
fn persist(ctx: &egui::Context, state_id: egui::Id, state: &PaletteState) {
    ctx.data_mut(|d| d.insert_temp(state_id, state.clone()));
}

/// Render one command row as a full-width selectable button. The selected row uses egui's selection
/// fill; a disabled row renders grayed and is added via `add_enabled(false, ..)` so it cannot be
/// clicked into a run (AC7 — no fake-enable). Bold label on the left, muted description on the right.
fn command_row(
    ui: &mut egui::Ui,
    cmd: &AppCommand,
    is_selected: bool,
    disabled: bool,
) -> egui::Response {
    let author_id = format!("{ROW_AUTHOR_ID_PREFIX}{}", cmd.stable_id);
    let full_width = ui.available_width();

    // Build the row text: bold label + muted description, laid out as one button so the whole row is a
    // single addressable ListBoxOption. `disabled` is the LIVE effective state (MT-069 predicate), not the
    // raw catalog flag.
    let mut job = egui::text::LayoutJob::default();
    let strong = ui.visuals().strong_text_color();
    let weak = ui.visuals().weak_text_color();
    job.append(
        cmd.label,
        0.0,
        egui::TextFormat {
            color: if disabled { weak } else { strong },
            ..Default::default()
        },
    );
    job.append(
        &format!("   {}", cmd.description),
        0.0,
        egui::TextFormat {
            color: weak,
            italics: disabled,
            ..Default::default()
        },
    );

    let response = ui.add_enabled(
        !disabled,
        egui::Button::selectable(is_selected, job)
            .truncate()
            .min_size(egui::vec2(full_width, 0.0)),
    );

    // Attach the stable author_id + ListBoxOption role + selected/disabled state to the SAME live node
    // egui built for this row (SelectableLabel derives Role + Action::Click from its Sense). This adds
    // the out-of-process address while leaving egui's interactive role/actions intact.
    let label = cmd.label.to_owned();
    ui.ctx().accesskit_node_builder(response.id, move |node| {
        node.set_role(accesskit::Role::ListBoxOption);
        node.set_author_id(author_id);
        node.set_label(label);
        if is_selected {
            node.set_selected(true);
        }
        if disabled {
            node.set_disabled();
        }
    });

    response
}

/// Emit the palette DIALOG root node (Role::Dialog, modal=true, label="Command Palette").
fn emit_dialog_node(ctx: &egui::Context, dialog_id: egui::Id) {
    ctx.accesskit_node_builder(dialog_id, |node| {
        node.set_role(accesskit::Role::Dialog);
        node.set_author_id(PALETTE_DIALOG_AUTHOR_ID.to_owned());
        node.set_label("Command Palette".to_owned());
        node.set_modal();
    });
}

/// Emit the palette SEARCH box address. egui already derived `Role::TextInput` + actions for the
/// `TextEdit`; this only adds the stable author_id (mirrors [`emit_interactive_node`] for the toggle).
fn emit_search_node(ctx: &egui::Context, search_id: egui::Id) {
    emit_interactive_node(ctx, search_id, PALETTE_SEARCH_AUTHOR_ID);
}

/// Emit the palette LIST container node (Role::ListBox, label="Actions").
fn emit_list_node(ctx: &egui::Context, list_id: egui::Id) {
    ctx.accesskit_node_builder(list_id, |node| {
        node.set_role(accesskit::Role::ListBox);
        node.set_author_id(PALETTE_LIST_AUTHOR_ID.to_owned());
        node.set_label("Actions".to_owned());
    });
}

/// Reference the declared-identity list so this module is tied to the registry's collision proof.
#[allow(dead_code)]
const _: &[&str] = PALETTE_AUTHOR_IDS;

#[cfg(test)]
mod tests {
    use super::*;

    /// The three fixed container ids sit in the fresh 11..=13 band, strictly below the pane id base.
    #[test]
    fn palette_container_ids_in_disjoint_fresh_band() {
        for id in [
            PALETTE_DIALOG_NODE_ID,
            PALETTE_SEARCH_NODE_ID,
            PALETTE_LIST_NODE_ID,
        ] {
            assert!((11..=13).contains(&id), "palette id {id} in band 11..=13");
            assert!(
                id < crate::accessibility::PANE_NODE_ID_BASE,
                "palette id {id} below pane base {}",
                crate::accessibility::PANE_NODE_ID_BASE
            );
        }
        // The three ids are distinct.
        assert_ne!(PALETTE_DIALOG_NODE_ID, PALETTE_SEARCH_NODE_ID);
        assert_ne!(PALETTE_SEARCH_NODE_ID, PALETTE_LIST_NODE_ID);
        assert_ne!(PALETTE_DIALOG_NODE_ID, PALETTE_LIST_NODE_ID);
    }

    /// The author_ids are stable kebab-case keys.
    #[test]
    fn palette_author_ids_are_stable() {
        assert_eq!(PALETTE_DIALOG_AUTHOR_ID, "command-palette.dialog");
        assert_eq!(PALETTE_SEARCH_AUTHOR_ID, "command-palette.search");
        assert_eq!(PALETTE_LIST_AUTHOR_ID, "command-palette.list");
        assert_eq!(ROW_AUTHOR_ID_PREFIX, "command-palette.option.");
    }
}
