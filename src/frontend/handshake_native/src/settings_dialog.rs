//! Settings / Options dialog for the native Handshake shell (WP-KERNEL-011 MT-018).
//!
//! ## What this provides (no-context model navigation — HBR-VIS / HBR-SWARM)
//!
//! A modal settings dialog (a centred, always-on-top floating panel with a hidden title bar) opened
//! from HELP > Open Settings… (MT-015 menu), the command palette action `settings.open` (MT-016), or a
//! test/agent setting `app_state.settings_open = true`. It is a port of
//! `app/src/components/SettingsMenu.tsx` over the [`crate::workspace_settings`] schema + helpers, with
//! these sections in order: Appearance (theme + view mode — both WIRED), Keybindings (editable, with
//! live conflict detection), Swarm (a wired board-default-open checkbox + not-yet-wired interval rows),
//! Terminal (not-yet-wired rows), Layout (a wired Reset panes & drawers button), and About (app name +
//! the real Cargo version).
//!
//! ## Swarm interaction contract (HBR-SWARM)
//!
//! A swarm agent drives this dialog out-of-process without screen scraping:
//! 1. set `app_state.settings_open = true` (HELP menu / palette `settings.open` / direct flag) to open;
//! 2. read the current theme from `app_state.workspace_settings().theme` (or the `Theme / appearance`
//!    ComboBox node, author_id [`THEME_COMBO_AUTHOR_ID`]);
//! 3. toggle it via the ComboBox (the dialog returns [`SettingsOutcome::ThemeChanged`], which the shell
//!    applies to `current_theme` + persists via `PUT /workspaces/{id}/settings`);
//! 4. verify the change propagated by re-reading `app_state.current_theme()` /
//!    `app_state.workspace_settings().theme`.
//!
//! Every interactive control carries a stable AccessKit role + author_id (see the consts below and the
//! per-control `author_id` strings) so the agent addresses them deterministically.
//!
//! ## Ownership split (mirrors the MT-016 command palette + MT-015 menu bar)
//!
//! The dialog owns ONLY its transient UI state (the search query + the in-progress *draft* keybinding
//! text per action), stored in egui persistent memory keyed to the dialog id and RESET when the dialog
//! re-opens (keyed by a monotonic `open_count`, like the palette). It NEVER mutates app state: [`show`]
//! takes a read-only [`SettingsView`] (the live settings + open generation) and returns a
//! [`SettingsOutcome`] the shell ([`crate::app`]) matches on to mutate `workspace_settings` /
//! `current_theme` / `view_mode` and persist. The shell owns the `settings_open` flag; the dialog only
//! REQUESTS close via [`SettingsOutcome::Close`].
//!
//! ## Stable AccessKit ids (out-of-process steering — HBR-VIS)
//!
//! Three FIXED container nodes in a fresh disjoint band (17..=19, directly above the quick-switcher
//! band 14..=16, below the chrome title bar 20 and the pane id base 100):
//! - the dialog root ([`SETTINGS_DIALOG_NODE_ID`] = 17, Role::Dialog, modal),
//! - the search box ([`SETTINGS_SEARCH_NODE_ID`] = 18, Role::TextInput),
//! - the body/list region ([`SETTINGS_LIST_NODE_ID`] = 19, Role::Group).
//!
//! Every interactive CONTROL (theme combo, view-mode combo, per-action keybinding inputs + reset
//! buttons, the swarm-board checkbox, the reset-layout button, the close button) is rendered with a
//! stable author_id STRING (in egui's hashed id space, the same convention as the palette command rows
//! and the per-tab nodes), so the count can vary with the search filter without bloating the fixed
//! band, while every control stays discoverable/clickable out-of-process and never trips the MT-025
//! interactive-naming gate. The three fixed container ids ARE enumerated in `DECLARED_IDENTITIES`.
//!
//! The dialog renders ONLY while `settings_open` is true (closed by default), so the default-seed live
//! tree never contains any of these nodes — exactly like the palette / switcher overlays.

use egui::accesskit;

use crate::workspace_settings::{
    find_keybinding_conflicts, keybinding_label_for_conflict, normalize_chord_input,
    setting_matches_query, ABOUT_APP_NAME, ABOUT_VERSION, APP_KEYBINDING_ACTIONS, Keybinding,
    NotYetWiredSetting, SettingsViewMode, WorkspaceSettingsState, WorkspaceTheme,
    SWARM_RECONCILE_INTERVAL_SETTING, SWARM_RESOURCE_POLL_INTERVAL_SETTING,
    TERMINAL_DEFAULT_SHELL_SETTING, TERMINAL_MAX_SCROLLBACK_SETTING,
    TERMINAL_OUTPUT_LOGGING_SETTING,
};

/// Fixed AccessKit/egui `NodeId` of the settings DIALOG root (Role::Dialog, modal). Fresh band slot 17:
/// directly above the quick-switcher band (14..=16), below the chrome title bar (20) and the pane id
/// base (100). A fixed-value `egui::Id` (`from_high_entropy_bits`) yields a fixed `NodeId` across frames
/// + restarts — the same convention every other fixed-band node in this crate uses.
pub const SETTINGS_DIALOG_NODE_ID: u64 = 17;
/// Fixed AccessKit/egui `NodeId` of the settings SEARCH box (Role::TextInput). Fresh band slot 18.
pub const SETTINGS_SEARCH_NODE_ID: u64 = 18;
/// Fixed AccessKit/egui `NodeId` of the settings BODY/list region (Role::Group). Fresh band slot 19.
pub const SETTINGS_LIST_NODE_ID: u64 = 19;

/// Stable out-of-process author_id for the settings dialog root.
pub const SETTINGS_DIALOG_AUTHOR_ID: &str = "settings.dialog";
/// Stable out-of-process author_id for the settings search box.
pub const SETTINGS_SEARCH_AUTHOR_ID: &str = "settings.search";
/// Stable out-of-process author_id for the settings body/list region.
pub const SETTINGS_LIST_AUTHOR_ID: &str = "settings.list";

/// Stable author_id for the Theme / appearance ComboBox.
pub const THEME_COMBO_AUTHOR_ID: &str = "settings.theme";
/// Stable author_id for the View Mode ComboBox.
pub const VIEW_MODE_COMBO_AUTHOR_ID: &str = "settings.view-mode";
/// Stable author_id for the Swarm board default-open checkbox.
pub const SWARM_BOARD_CHECKBOX_AUTHOR_ID: &str = "settings.swarm-board-default-open";
/// Stable author_id for the Reset panes & drawers button.
pub const RESET_LAYOUT_AUTHOR_ID: &str = "settings.reset-layout";
/// Stable author_id for the Close button.
pub const CLOSE_AUTHOR_ID: &str = "settings.close";
/// Author_id prefix for a per-action keybinding text input (`{prefix}{action_id}`).
pub const KEYBINDING_INPUT_AUTHOR_ID_PREFIX: &str = "settings.keybinding.";
/// Author_id prefix for a per-action keybinding Reset button (`{prefix}{action_id}`).
pub const KEYBINDING_RESET_AUTHOR_ID_PREFIX: &str = "settings.keybinding-reset.";
/// Author_id prefix for a not-yet-wired row's disabled control (`{prefix}{setting_id}`).
pub const NOT_WIRED_AUTHOR_ID_PREFIX: &str = "settings.not-wired.";

/// What the dialog wants the shell to do after a frame.
///
/// Returned by [`show`]. The shell matches on it: a wired change variant updates
/// `workspace_settings` / `current_theme` / `view_mode` and persists via `PUT /workspaces/{id}/settings`;
/// `Close` clears the open flag; `None` leaves the dialog open. At most one outcome per frame (a single
/// control interaction), so the shell never has to reconcile two simultaneous changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOutcome {
    /// Nothing happened this frame; keep the dialog open.
    None,
    /// The Theme / appearance ComboBox selected a (different) theme. WIRED.
    ThemeChanged(WorkspaceTheme),
    /// The View Mode ComboBox selected a (different) mode. WIRED.
    ViewModeChanged(SettingsViewMode),
    /// A keybinding's chord changed to a NON-conflicting value (already normalized). WIRED — the shell
    /// persists it. A conflicting draft does NOT emit this (it only shows the banner), so a conflicting
    /// binding is never saved (AC6).
    KeybindingChanged { action_id: String, chord: String },
    /// A keybinding Reset button was clicked; restore the action's default chord. WIRED.
    KeybindingReset { action_id: String },
    /// The Swarm board default-open checkbox was toggled. WIRED.
    SwarmBoardDefaultOpenChanged(bool),
    /// The Reset panes & drawers button was clicked (same action as VIEW > Reset Layout). WIRED.
    ResetLayout,
    /// The user dismissed the dialog (Escape, the Close button, or a backdrop click). The shell clears
    /// the open flag.
    Close,
}

/// Read-only inputs the dialog renders from (the live settings + the open generation). The dialog never
/// borrows `&mut HandshakeApp`; the shell applies the returned [`SettingsOutcome`].
pub struct SettingsView<'a> {
    /// Monotonic open generation; a new value resets the dialog's transient state.
    pub open_count: u64,
    /// The live workspace settings (theme, keybindings, view mode, swarm board flag).
    pub settings: &'a WorkspaceSettingsState,
    /// The last transient persistence error, if any, surfaced on the status row.
    pub persist_error: Option<&'a str>,
}

/// Transient per-open dialog UI state: the search query + the in-progress draft keybinding text per
/// action. Stored in egui persistent memory keyed to the dialog id, and RESET when [`open_count`]
/// changes so a re-open never shows the previous session's text/drafts.
///
/// [`open_count`]: DialogState::open_count
#[derive(Debug, Clone, Default)]
struct DialogState {
    /// The open generation this state was initialized for.
    open_count: u64,
    /// The current settings-search query.
    query: String,
    /// In-progress draft chord text per action (`(action_id, draft)`). The draft is what the text input
    /// shows; it is normalized for conflict detection + only persisted (via the outcome) when conflict-
    /// free (red-team R3/MC3: normalize the draft before comparing). Seeded from the live settings on
    /// (re-)open.
    drafts: Vec<(String, String)>,
    /// Set once after a (re-)open so the search box is focused on the first frame only.
    focus_requested: bool,
}

impl DialogState {
    /// The draft chord for `action_id`, if a draft has been seeded/edited.
    fn draft_for(&self, action_id: &str) -> Option<&str> {
        self.drafts
            .iter()
            .find(|(id, _)| id == action_id)
            .map(|(_, d)| d.as_str())
    }

    /// Set the draft chord for `action_id`.
    fn set_draft(&mut self, action_id: &str, draft: String) {
        if let Some(slot) = self.drafts.iter_mut().find(|(id, _)| id == action_id) {
            slot.1 = draft;
        } else {
            self.drafts.push((action_id.to_owned(), draft));
        }
    }

    /// A settings snapshot used to compute conflicts from the CURRENT drafts (red-team R3/MC3): each
    /// draft is normalized; an action with no draft uses the live chord.
    fn draft_settings(&self, live: &WorkspaceSettingsState) -> WorkspaceSettingsState {
        let keybindings: Vec<Keybinding> = APP_KEYBINDING_ACTIONS
            .iter()
            .map(|action| {
                let chord = self
                    .draft_for(action.id)
                    .map(normalize_chord_input)
                    .unwrap_or_else(|| {
                        normalize_chord_input(live.chord_for(action.id).unwrap_or(action.default_chord))
                    });
                Keybinding {
                    action_id: action.id.to_owned(),
                    chord,
                }
            })
            .collect();
        WorkspaceSettingsState {
            theme: live.theme,
            keybindings,
            view_mode: live.view_mode,
            swarm_board_default_open: live.swarm_board_default_open,
        }
    }
}

/// Render the settings dialog overlay and return the [`SettingsOutcome`] for this frame.
///
/// `view.open_count` is a monotonic counter the shell increments each time `settings_open` flips to
/// `true`; the dialog resets its transient state whenever it sees a new value. The dialog is rendered as
/// a backdrop [`egui::Area`] (full-screen, behind the panel, catches click-to-dismiss) plus a centred
/// [`egui::Window`] with the title bar hidden — both on the `Foreground` order so the dialog sits above
/// the whole workspace (and above the palette/switcher overlays the shell renders earlier).
///
/// Layout choice (contract note): a CENTRED modal with a scroll body is used rather than a right-edge
/// side-drawer. egui's `Window` right-edge anchoring with a full-height fixed panel is awkward (it
/// fights egui's auto-sizing + the existing top/bottom panels), and the contract explicitly allows the
/// centred-modal fallback; the centred modal matches the palette/switcher overlay convention already in
/// this crate, so the three overlays are visually + structurally consistent.
pub fn show(ctx: &egui::Context, view: SettingsView<'_>) -> SettingsOutcome {
    let state_id = egui::Id::new("settings.state");
    let mut state: DialogState = ctx
        .data_mut(|d| d.get_temp::<DialogState>(state_id))
        .unwrap_or_default();

    // Reset transient state on (re-)open: a new open generation clears the query + reseeds drafts from
    // the live settings so a re-open never shows the previous session's text/drafts.
    if state.open_count != view.open_count {
        state = DialogState {
            open_count: view.open_count,
            query: String::new(),
            drafts: APP_KEYBINDING_ACTIONS
                .iter()
                .map(|action| {
                    (
                        action.id.to_owned(),
                        view.settings
                            .chord_for(action.id)
                            .unwrap_or(action.default_chord)
                            .to_owned(),
                    )
                })
                .collect(),
            focus_requested: false,
        };
    }

    // ── Escape (AC12) — popup-aware (FIX-C). ────────────────────────────────────────────────────────
    // Escape has two jobs in this dialog: close an OPEN ComboBox popup (Theme / View Mode), or — when
    // nothing else is open — close the whole dialog. egui's ComboBox popup closes itself on Escape by
    // PEEKING `i.key_pressed(Key::Escape)` (it does not consume the event), and so does this handler
    // (`i.events.iter()` is a peek too). So on a single Escape both fire in the same frame: the popup
    // would close AND the dialog would close. That is the bug — one Escape collapsing the popup should
    // NOT also tear down the dialog.
    //
    // Fix: read whether ANY egui popup/combo is open coming into this frame (`Popup::is_any_open` reads
    // the memory the popup wrote LAST frame). If a popup is open, this Escape is "owned" by the popup —
    // egui closes it when the combo renders below this frame — so we suppress the dialog-close and keep
    // the dialog open. Only when no popup is open does Escape request a dialog close. (We do not call
    // `Popup::close_all` ourselves: egui's own Escape handling closes the combo, and double-closing
    // could swallow a second nested popup the same frame.)
    let escape = ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(
                e,
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                }
            )
        })
    });
    if escape {
        let popup_open = egui::Popup::is_any_open(ctx);
        if !popup_open {
            persist(ctx, state_id, &state);
            return SettingsOutcome::Close;
        }
        // A popup/combo is open: let egui's own Escape handling close just the popup this frame; the
        // dialog stays open. Fall through to render so the combo gets its frame to close.
    }

    // ── Backdrop: a full-screen interactable Area BEHIND the window; a click on it dismisses. ──
    let screen = ctx.content_rect();
    let backdrop = egui::Area::new(egui::Id::new("settings.backdrop"))
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
        return SettingsOutcome::Close;
    }

    let search_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_SEARCH_NODE_ID) };
    let dialog_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_DIALOG_NODE_ID) };
    let list_egui_id = unsafe { egui::Id::from_high_entropy_bits(SETTINGS_LIST_NODE_ID) };

    let mut outcome = SettingsOutcome::None;

    egui::Window::new("settings")
        .id(egui::Id::new("settings.window"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([480.0, 560.0])
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // Header: eyebrow + title on the left, Close button on the right.
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("GLOBAL").small().weak());
                    ui.label(egui::RichText::new("Settings").heading());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.button("Close");
                    set_author_id(ui, close.id, CLOSE_AUTHOR_ID);
                    if close.clicked() {
                        outcome = SettingsOutcome::Close;
                    }
                });
            });
            ui.add_space(6.0);

            // Search input, pinned to the fixed search id so its AccessKit NodeId is stable.
            ui.label(egui::RichText::new("Search settings").small().weak());
            let edit = egui::TextEdit::singleline(&mut state.query)
                .id(search_egui_id)
                .hint_text("Theme, quick switcher, terminal...")
                .desired_width(f32::INFINITY);
            let _edit_response = ui.add(edit);
            if !state.focus_requested {
                _edit_response.request_focus();
                state.focus_requested = true;
            }
            emit_search_node(ui.ctx(), search_egui_id);

            // Persistence error row (HBR: important state visible; surfaces a save/load failure).
            if let Some(err) = view.persist_error {
                ui.add_space(4.0);
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("Settings sync error: {err}"),
                );
            }

            ui.add_space(6.0);

            let query = state.query.trim().to_lowercase();

            // The scrollable body region (Role::Group container at the fixed list id).
            ui.push_id(list_egui_id, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(440.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        outcome = render_sections(ui, &query, &mut state, &view, outcome.clone());
                    });
            });
            emit_list_node(ui.ctx(), list_egui_id);
        });

    // The dialog root container node (Role::Dialog, modal) attached to the fixed dialog id so an
    // out-of-process model finds the modal by `settings.dialog`. Emitted each open frame.
    emit_dialog_node(ctx, dialog_egui_id);

    persist(ctx, state_id, &state);
    outcome
}

/// Render every settings section in order, threading `outcome` so the first interaction this frame wins
/// (we never overwrite a Close already chosen in the header). Returns the (possibly updated) outcome.
fn render_sections(
    ui: &mut egui::Ui,
    query: &str,
    state: &mut DialogState,
    view: &SettingsView<'_>,
    mut outcome: SettingsOutcome,
) -> SettingsOutcome {
    let settings = view.settings;

    // ── [1] Appearance (theme + view mode — both WIRED) ────────────────────────────────────────────
    let show_appearance = setting_matches_query(
        query,
        &["appearance", "theme", "light", "dark", "view", "mode", "sfw", "nsfw"],
    );
    let show_theme_row = setting_matches_query(query, &["appearance", "theme", "light", "dark"]);
    let show_view_mode_row =
        setting_matches_query(query, &["appearance", "view", "mode", "sfw", "nsfw"]);
    if show_appearance {
        egui::CollapsingHeader::new("Appearance")
            .default_open(true)
            .show(ui, |ui| {
                if show_view_mode_row {
                    ui.horizontal(|ui| {
                        ui.label("View Mode");
                        // UPGRADED: NSFW/SFW content visibility is wired + persisted in the native shell.
                        let current = settings.view_mode;
                        let mut selected = current;
                        let combo = egui::ComboBox::from_id_salt("settings.view-mode.combo")
                            .selected_text(match current {
                                SettingsViewMode::Nsfw => "NSFW",
                                SettingsViewMode::Sfw => "SFW",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut selected, SettingsViewMode::Nsfw, "NSFW");
                                ui.selectable_value(&mut selected, SettingsViewMode::Sfw, "SFW");
                            });
                        // The visible "View Mode" row label (above) provides the accessible name; the
                        // combo carries only the stable author_id so there is exactly ONE node labeled
                        // "View Mode" in the tree (unambiguous for out-of-process lookup-by-label).
                        set_author_id(ui, combo.response.id, VIEW_MODE_COMBO_AUTHOR_ID);
                        if selected != current && outcome == SettingsOutcome::None {
                            outcome = SettingsOutcome::ViewModeChanged(selected);
                        }
                    });
                }
                if show_theme_row {
                    ui.horizontal(|ui| {
                        ui.label("Theme / appearance");
                        // UPGRADED from NotYetWiredRow: theme is now wired in the native shell.
                        let current = settings.theme;
                        let mut selected = current;
                        let combo = egui::ComboBox::from_id_salt("settings.theme.combo")
                            .selected_text(match current {
                                WorkspaceTheme::Light => "Light",
                                WorkspaceTheme::Dark => "Dark",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut selected, WorkspaceTheme::Light, "Light");
                                ui.selectable_value(&mut selected, WorkspaceTheme::Dark, "Dark");
                            });
                        // The visible "Theme / appearance" row label (above) provides the accessible
                        // name; the combo carries only the stable author_id so there is exactly ONE node
                        // labeled "Theme / appearance" in the tree (unambiguous lookup-by-label).
                        set_author_id(ui, combo.response.id, THEME_COMBO_AUTHOR_ID);
                        if selected != current && outcome == SettingsOutcome::None {
                            outcome = SettingsOutcome::ThemeChanged(selected);
                        }
                    });
                }
            });
    }

    // ── [2] Keybindings (editable + live conflict detection) ───────────────────────────────────────
    let visible_actions: Vec<&'static crate::workspace_settings::AppKeybindingAction> =
        APP_KEYBINDING_ACTIONS
            .iter()
            .filter(|action| {
                let mut terms: Vec<&str> = vec!["keybinding", "shortcut", action.label, action.description];
                terms.extend_from_slice(action.keywords);
                setting_matches_query(query, &terms)
            })
            .collect();
    let show_keybindings = !visible_actions.is_empty()
        || setting_matches_query(query, &["keybinding", "keybindings", "shortcut", "shortcuts"]);
    if show_keybindings {
        egui::CollapsingHeader::new("Keybindings")
            .default_open(true)
            .show(ui, |ui| {
                // Conflict banner computed from the CURRENT drafts (normalized — red-team R3/MC3).
                let draft_settings = state.draft_settings(settings);
                let conflicts = find_keybinding_conflicts(&draft_settings);
                if !conflicts.is_empty() {
                    let text = conflicts
                        .iter()
                        .map(|c| {
                            format!(
                                "{} both use {}.",
                                keybinding_label_for_conflict(&c.action_labels),
                                c.chord
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let banner = ui.colored_label(ui.visuals().error_fg_color, &text);
                    // An addressable alert node so a swarm agent reads the conflict out-of-process.
                    ui.ctx().accesskit_node_builder(banner.id, |node| {
                        node.set_role(accesskit::Role::Alert);
                        node.set_author_id("settings.keybinding-conflict".to_owned());
                        node.set_label(text.clone());
                    });
                }
                for action in &visible_actions {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(action.label);
                            ui.label(egui::RichText::new(action.description).small().weak());
                        });
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let reset = ui.button("Reset");
                                set_author_id(
                                    ui,
                                    reset.id,
                                    &format!("{KEYBINDING_RESET_AUTHOR_ID_PREFIX}{}", action.id),
                                );
                                if reset.clicked() && outcome == SettingsOutcome::None {
                                    // Reflect the default in the draft immediately, then emit the reset.
                                    state.set_draft(action.id, action.default_chord.to_owned());
                                    outcome = SettingsOutcome::KeybindingReset {
                                        action_id: action.id.to_owned(),
                                    };
                                }

                                // Editable chord input bound to the draft.
                                let mut draft = state
                                    .draft_for(action.id)
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| action.default_chord.to_owned());
                                let input = ui.add(
                                    egui::TextEdit::singleline(&mut draft)
                                        .desired_width(140.0)
                                        .hint_text(action.default_chord),
                                );
                                set_author_id_and_label(
                                    ui,
                                    input.id,
                                    &format!("{KEYBINDING_INPUT_AUTHOR_ID_PREFIX}{}", action.id),
                                    &format!("{} keybinding", action.label),
                                );
                                if input.changed() {
                                    state.set_draft(action.id, draft.clone());
                                    // Persist ONLY when the new draft is conflict-free (AC6): build the
                                    // would-be settings, normalize, check conflicts; emit on clean.
                                    //
                                    // FIX-D — conflict basis (DELIBERATE): the conflict check runs over
                                    // `draft_settings` (every action's CURRENT draft text, normalized),
                                    // NOT over the persisted `settings.keybindings`. This is intentional:
                                    // an editor must see a conflict against what the user is TYPING right
                                    // now across all rows, not against the last-saved chords (which may
                                    // already be stale relative to two in-progress edits). The persisted
                                    // keybindings only seed the drafts on (re-)open; from then on the
                                    // visible drafts are authoritative for conflict detection, so a
                                    // `KeybindingChanged` is emitted (and later persisted) only when the
                                    // DRAFT set — the user's visible intent — is conflict-free.
                                    let normalized = normalize_chord_input(&draft);
                                    if !normalized.is_empty() && outcome == SettingsOutcome::None {
                                        let mut probe = state.draft_settings(settings);
                                        probe.set_chord(action.id, normalized.clone());
                                        if find_keybinding_conflicts(&probe).is_empty() {
                                            outcome = SettingsOutcome::KeybindingChanged {
                                                action_id: action.id.to_owned(),
                                                chord: normalized,
                                            };
                                        }
                                    }
                                }
                            },
                        );
                    });
                }
            });
    }

    // ── [3] Swarm (wired checkbox + not-yet-wired interval rows) ───────────────────────────────────
    let show_swarm =
        setting_matches_query(query, &["swarm", "board", "reconcile", "resource", "poll"]);
    if show_swarm {
        egui::CollapsingHeader::new("Swarm")
            .default_open(true)
            .show(ui, |ui| {
                not_yet_wired_row(ui, &SWARM_RECONCILE_INTERVAL_SETTING);
                not_yet_wired_row(ui, &SWARM_RESOURCE_POLL_INTERVAL_SETTING);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Open Swarm Board on launch");
                        ui.label(
                            egui::RichText::new(
                                "Persisted. Board stays collapsed by default; enable to open it at startup.",
                            )
                            .small()
                            .weak(),
                        );
                    });
                    let mut checked = settings.swarm_board_default_open;
                    let cb_label = if checked { "Open" } else { "Collapsed" };
                    let cb = ui.checkbox(&mut checked, cb_label);
                    set_author_id(ui, cb.id, SWARM_BOARD_CHECKBOX_AUTHOR_ID);
                    if cb.changed() && outcome == SettingsOutcome::None {
                        outcome = SettingsOutcome::SwarmBoardDefaultOpenChanged(checked);
                    }
                });
            });
    }

    // ── [4] Terminal (not-yet-wired rows) ──────────────────────────────────────────────────────────
    let show_terminal =
        setting_matches_query(query, &["terminal", "shell", "scrollback", "logging"]);
    if show_terminal {
        egui::CollapsingHeader::new("Terminal")
            .default_open(true)
            .show(ui, |ui| {
                not_yet_wired_row(ui, &TERMINAL_DEFAULT_SHELL_SETTING);
                not_yet_wired_row(ui, &TERMINAL_MAX_SCROLLBACK_SETTING);
                not_yet_wired_row(ui, &TERMINAL_OUTPUT_LOGGING_SETTING);
            });
    }

    // ── [5] Layout (wired Reset panes & drawers button) ────────────────────────────────────────────
    let show_layout = setting_matches_query(query, &["layout", "reset", "panes", "drawers"]);
    if show_layout {
        egui::CollapsingHeader::new("Layout")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Reset layout");
                        ui.label(
                            egui::RichText::new("Restore panes & drawers to their defaults.")
                                .small()
                                .weak(),
                        );
                    });
                    let btn = ui.button("Reset panes & drawers");
                    set_author_id(ui, btn.id, RESET_LAYOUT_AUTHOR_ID);
                    if btn.clicked() && outcome == SettingsOutcome::None {
                        outcome = SettingsOutcome::ResetLayout;
                    }
                });
            });
    }

    // ── [6] About (app name + REAL Cargo version) ──────────────────────────────────────────────────
    let show_about = setting_matches_query(query, &["about", "app", "version"]);
    if show_about {
        egui::CollapsingHeader::new("About")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("App").small().weak());
                    ui.label(ABOUT_APP_NAME);
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Version").small().weak());
                    ui.label(ABOUT_VERSION);
                });
            });
        // TODO MT-0XX: CLI Bridge config panel - see app/src/components/CliBridgeConfigPanel.tsx
    }

    outcome
}

/// Render one not-yet-wired row: label + note on the left, a DISABLED read-only text input pinned to the
/// fixed value on the right. The control uses `add_enabled(false, ..)` so it is visually grayed AND
/// cannot receive Tab focus (red-team R5/MC5), and carries a stable author_id derived from the setting
/// id so it is addressable out-of-process while clearly non-editable.
fn not_yet_wired_row(ui: &mut egui::Ui, setting: &NotYetWiredSetting) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(setting.label);
            ui.label(egui::RichText::new(setting.note).small().weak());
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut value = setting.fixed_value.to_owned();
            let resp = ui.add_enabled(
                false,
                egui::TextEdit::singleline(&mut value).desired_width(180.0),
            );
            set_author_id(
                ui,
                resp.id,
                &format!("{NOT_WIRED_AUTHOR_ID_PREFIX}{}", setting.id),
            );
        });
    });
}

/// Persist the transient dialog state back into egui memory.
fn persist(ctx: &egui::Context, state_id: egui::Id, state: &DialogState) {
    ctx.data_mut(|d| d.insert_temp(state_id, state.clone()));
}

/// Attach a stable author_id to an already-interactive live node (egui derived its role + actions from
/// the widget's `Sense`/`widget_info`). Mirrors `accessibility::emit_interactive_node`, but takes a
/// `&Ui` for the egui-version ergonomics inside the closures.
fn set_author_id(ui: &egui::Ui, widget_id: egui::Id, author_id: &str) {
    let author_id = author_id.to_owned();
    ui.ctx()
        .accesskit_node_builder(widget_id, move |node| node.set_author_id(author_id));
}

/// Attach a stable author_id AND an accessible label to an already-interactive live node. Used for
/// controls whose accessible name is not derivable from rendered text (e.g. a `TextEdit`, or a ComboBox
/// whose visible label is a sibling), so an out-of-process model + kittest can resolve them by label.
fn set_author_id_and_label(ui: &egui::Ui, widget_id: egui::Id, author_id: &str, label: &str) {
    let author_id = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(widget_id, move |node| {
        node.set_author_id(author_id);
        node.set_label(label);
    });
}

/// Emit the dialog ROOT node (Role::Dialog, modal=true, label="Settings").
fn emit_dialog_node(ctx: &egui::Context, dialog_id: egui::Id) {
    ctx.accesskit_node_builder(dialog_id, |node| {
        node.set_role(accesskit::Role::Dialog);
        node.set_author_id(SETTINGS_DIALOG_AUTHOR_ID.to_owned());
        node.set_label("Settings".to_owned());
        node.set_modal();
    });
}

/// Emit the search box address. egui already derived `Role::TextInput`; this adds the stable author_id.
fn emit_search_node(ctx: &egui::Context, search_id: egui::Id) {
    crate::accessibility::emit_interactive_node(ctx, search_id, SETTINGS_SEARCH_AUTHOR_ID);
}

/// Emit the body/list region node (Role::Group, label="Settings sections").
fn emit_list_node(ctx: &egui::Context, list_id: egui::Id) {
    ctx.accesskit_node_builder(list_id, |node| {
        node.set_role(accesskit::Role::Group);
        node.set_author_id(SETTINGS_LIST_AUTHOR_ID.to_owned());
        node.set_label("Settings sections".to_owned());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three fixed container ids sit in the fresh 17..=19 band, strictly below the pane id base, and
    /// are distinct.
    #[test]
    fn settings_container_ids_in_disjoint_fresh_band() {
        for id in [
            SETTINGS_DIALOG_NODE_ID,
            SETTINGS_SEARCH_NODE_ID,
            SETTINGS_LIST_NODE_ID,
        ] {
            assert!((17..=19).contains(&id), "settings id {id} in band 17..=19");
            assert!(
                id < crate::accessibility::PANE_NODE_ID_BASE,
                "settings id {id} below pane base {}",
                crate::accessibility::PANE_NODE_ID_BASE
            );
        }
        assert_ne!(SETTINGS_DIALOG_NODE_ID, SETTINGS_SEARCH_NODE_ID);
        assert_ne!(SETTINGS_SEARCH_NODE_ID, SETTINGS_LIST_NODE_ID);
        assert_ne!(SETTINGS_DIALOG_NODE_ID, SETTINGS_LIST_NODE_ID);
    }

    /// The author_ids are stable kebab-case keys.
    #[test]
    fn settings_author_ids_are_stable() {
        assert_eq!(SETTINGS_DIALOG_AUTHOR_ID, "settings.dialog");
        assert_eq!(SETTINGS_SEARCH_AUTHOR_ID, "settings.search");
        assert_eq!(SETTINGS_LIST_AUTHOR_ID, "settings.list");
        assert_eq!(THEME_COMBO_AUTHOR_ID, "settings.theme");
        assert_eq!(VIEW_MODE_COMBO_AUTHOR_ID, "settings.view-mode");
        assert_eq!(SWARM_BOARD_CHECKBOX_AUTHOR_ID, "settings.swarm-board-default-open");
        assert_eq!(RESET_LAYOUT_AUTHOR_ID, "settings.reset-layout");
        assert_eq!(CLOSE_AUTHOR_ID, "settings.close");
    }
}
