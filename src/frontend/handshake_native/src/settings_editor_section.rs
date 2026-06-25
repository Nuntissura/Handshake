//! Editor-specific Settings sections for the native Handshake shell (WP-KERNEL-012 MT-072).
//!
//! ## What this module owns
//!
//! The three editor-focused Settings sections the Handshake GUI look-and-behavior doc defines and that
//! WP-KERNEL-011 only stubbed, mounted INTO the existing WP-011 [`crate::settings_dialog`] dialog (this
//! module is NOT a new dialog / persistence system — it renders inside the existing one, and its values
//! ride the SAME PostgreSQL-backed `GET`/`PUT /workspaces/:id/settings` payload WP-011 already stores):
//!
//! 1. **Editor** — [`render_editor_prefs`]: `editor_font_size` (a `DragValue`, clamped 6..=48),
//!    `tab_size` (a `DragValue`, clamped 1..=16), `insert_spaces` (a `Checkbox`), `word_wrap` (a
//!    `ComboBox`: Off | On | Bounded column), `render_whitespace` (a `ComboBox`: None | Boundary | All),
//!    plus a read-only surface of the already-wired `auto_save_interval` so the editor prefs read
//!    together (NOT re-added — surfaced).
//! 2. **Syntax** — [`render_syntax_palette`]: a Muted | Standard | Custom selector; in Custom mode one
//!    `color_edit_button_srgba` swatch per [`HighlightScope`], feeding
//!    [`crate::code_editor::resolve_scope_color`] LIVE (a swatch edit changes the resolved color in the
//!    same frame).
//! 3. **Keybindings (editor extension)** — [`render_editor_keybindings`]: ALL editor-specific actions
//!    (MT-010 code chords + rich-editor commands), each row showing the action id/label, the current
//!    binding (an editable text input), and the built-in default; a custom binding overrides the default
//!    for that action and persists into the SEPARATE `editor_keybindings` list (NOT the WP-011
//!    `keybindings` map — the backend deny-unknown-validates that map's keys; see the persistence note
//!    below).
//!
//! ## Ownership split (mirrors [`crate::settings_dialog`])
//!
//! Like the WP-011 dialog, this section is RENDER-ONLY over a read-only [`EditorSettingsView`] and
//! returns an [`EditorSectionOutcome`] the shell applies + persists; it never borrows `&mut` app state.
//! Transient per-row input drafts live in egui memory keyed to the dialog open generation (the same
//! reset-on-reopen pattern the keybindings rows already use). The shell's debounced `PUT` carries the
//! new fields because they are part of the SAME serialized [`crate::workspace_settings::WorkspaceSettingsState`].
//!
//! ## Persistence-authority note (RISK-001 / the load-bearing extensibility question)
//!
//! The backend `validate_workspace_settings_state_shape` accepts EXTRA top-level keys but
//! deny-unknown-validates the `keybindings` map (its keys must be exactly the two WP-011 app actions).
//! So `editor_prefs` + `syntax_palette` are NEW TOP-LEVEL keys (stored verbatim), but editor keybinding
//! overrides live in a SEPARATE top-level `editor_keybindings` list — NOT the shared map. Writing editor
//! bindings into the shared map would hard-fail every PUT. No SQLite, no new endpoint, no new save code.

use egui::accesskit;

use crate::code_editor::keymap::CodeEditorAction;
use crate::code_editor::{resolve_scope_color, HighlightScope, Keymap};
use crate::rich_editor::formatting::FormattingCommand;
use crate::workspace_settings::{
    EditorPrefs, RenderWhitespaceMode, SyntaxPalette, SyntaxPaletteMode, WordWrapMode,
    WorkspaceSettingsState,
};

// ── Stable AccessKit author_ids (HBR-VIS / HBR-SWARM — out-of-process steering) ───────────────────
//
// Every new control carries a stable author_id in egui's hashed id space (the same convention the
// WP-011 settings form controls use — NOT the fixed 17..=19 container band; see settings_dialog.rs).
// These are NOT enumerated in accessibility::registry::DECLARED_IDENTITIES because, like the existing
// settings form controls / palette command rows, they are addressed by author_id STRING in the hashed
// id space, not by a fixed NodeId.

/// AccessKit author_id for the editor font-size `DragValue`.
pub const EDITOR_FONT_SIZE_AUTHOR_ID: &str = "settings-editor-font-size";
/// AccessKit author_id for the tab-size `DragValue`.
pub const EDITOR_TAB_SIZE_AUTHOR_ID: &str = "settings-editor-tab-size";
/// AccessKit author_id for the insert-spaces (tabs-vs-spaces) `Checkbox`.
pub const EDITOR_INSERT_SPACES_AUTHOR_ID: &str = "settings-editor-insert-spaces";
/// AccessKit author_id for the word-wrap mode `ComboBox`.
pub const EDITOR_WORD_WRAP_AUTHOR_ID: &str = "settings-editor-word-wrap";
/// AccessKit author_id for the render-whitespace mode `ComboBox`.
pub const EDITOR_RENDER_WHITESPACE_AUTHOR_ID: &str = "settings-editor-render-whitespace";
/// AccessKit author_id for the bounded-wrap-column `DragValue` (shown only when word_wrap = Bounded).
pub const EDITOR_WRAP_COLUMN_AUTHOR_ID: &str = "settings-editor-wrap-column";
/// AccessKit author_id for the syntax-palette mode `ComboBox`.
pub const SYNTAX_PALETTE_MODE_AUTHOR_ID: &str = "settings-syntax-palette-mode";
/// Author_id prefix for a per-scope Custom swatch (`settings-syntax-swatch-{scope}`).
pub const SYNTAX_SWATCH_AUTHOR_ID_PREFIX: &str = "settings-syntax-swatch-";
/// Author_id prefix for a per-action editor keybinding row input (`settings-keybind-row-{action_id}`).
pub const EDITOR_KEYBIND_ROW_AUTHOR_ID_PREFIX: &str = "settings-keybind-row-";
/// Author_id prefix for a per-action editor keybinding Reset button.
pub const EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX: &str = "settings-keybind-reset-";

/// The stable author_id for a Custom syntax swatch for `scope`.
pub fn syntax_swatch_author_id(scope: HighlightScope) -> String {
    format!("{SYNTAX_SWATCH_AUTHOR_ID_PREFIX}{}", scope.scope_key())
}

/// The stable author_id for an editor keybinding row input for `action_id`.
pub fn editor_keybind_row_author_id(action_id: &str) -> String {
    format!("{EDITOR_KEYBIND_ROW_AUTHOR_ID_PREFIX}{action_id}")
}

// ── Editor action catalog (the keybindings table source — AC-005) ─────────────────────────────────

/// The surface an editor keybinding action belongs to (so the table can group + label code vs rich).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorActionSurface {
    /// A code-editor (Monaco-parity) action (MT-010 chords + the MT-047..053 expansion).
    Code,
    /// A rich-text (Obsidian/Notion-parity) editor command (MT-011+ formatting commands).
    Rich,
}

/// One editor keybinding action row descriptor: a stable id, a display label, the built-in default
/// chord (a human-readable string), and the surface it belongs to. The id is a
/// [`CodeEditorAction::name`] (snake_case) for code actions or a [`FormattingCommand::command_id`] for
/// rich commands; the two id spaces are kept disjoint by the `code.`/`rich.` prefix so a code action and
/// a rich command with the same bare name (e.g. `undo`) never collide in the override map.
#[derive(Debug, Clone)]
pub struct EditorAction {
    /// The stable, prefix-namespaced action id used as the override-map key (`code.open_find`,
    /// `rich.toggle_bold`).
    pub id: String,
    /// The bare (un-prefixed) action id, for display.
    pub bare_id: &'static str,
    /// Display label.
    pub label: String,
    /// The built-in default chord (human-readable), or `""` when the action has no default chord.
    pub default_chord: String,
    /// Which editor surface this action belongs to.
    pub surface: EditorActionSurface,
}

/// The prefix that namespaces a code-editor action id in the override map.
pub const CODE_ACTION_ID_PREFIX: &str = "code.";
/// The prefix that namespaces a rich-editor command id in the override map.
pub const RICH_ACTION_ID_PREFIX: &str = "rich.";

/// Build the FULL editor-specific action catalog (AC-005): every code-editor action (from
/// [`CodeEditorAction::all`], with its default chord read from the live VS Code default [`Keymap`]) and
/// every rich-editor formatting command (from [`rich_editor_commands`]). The list is the keybindings
/// table's source of truth; the shell reads the same list to resolve default-vs-custom.
///
/// Default chords come from the SAME default keymap the live editor uses, so the "Default" column is
/// honest (it is what the action does today with no override), not a re-listed guess.
pub fn editor_action_catalog() -> Vec<EditorAction> {
    let default_keymap = Keymap::default_vscode();
    let mut catalog: Vec<EditorAction> = Vec::new();

    // Code-editor actions (MT-010 chords + the MT-047..053 expansion), each with its default chord.
    for action in CodeEditorAction::all() {
        let bare_id = action.name();
        let default_chord = default_keymap
            .bindings_for_action(*action)
            .first()
            .map(|b| crate::code_editor::keymap_settings::KeymapSettings::chord_to_str(&b.chord))
            .unwrap_or_default();
        catalog.push(EditorAction {
            id: format!("{CODE_ACTION_ID_PREFIX}{bare_id}"),
            bare_id,
            label: action.description().to_owned(),
            default_chord,
            surface: EditorActionSurface::Code,
        });
    }

    // Rich-editor commands (MT-011+ formatting commands). These are dispatched through the rich editor
    // keymap/toolbar; the default chord column shows the canonical shortcut where one exists (a blank
    // default means "toolbar/slash-command only — no default chord").
    for (command, label, default_chord) in rich_editor_commands() {
        catalog.push(EditorAction {
            id: format!("{RICH_ACTION_ID_PREFIX}{}", command.command_id()),
            bare_id: command.command_id(),
            label: label.to_owned(),
            default_chord: default_chord.to_owned(),
            surface: EditorActionSurface::Rich,
        });
    }

    catalog
}

/// The rich-editor command catalog: each [`FormattingCommand`] paired with a display label and its
/// default chord (the canonical formatting shortcut where one exists, else `""` for a toolbar/slash-only
/// command). A small fixed table (the rich command set is fixed) so the keybindings section lists the
/// rich commands alongside the code chords (AC-005).
fn rich_editor_commands() -> Vec<(FormattingCommand, &'static str, &'static str)> {
    use FormattingCommand as F;
    vec![
        (F::Undo, "Undo (rich)", "Mod+Z"),
        (F::Redo, "Redo (rich)", "Mod+Shift+Z"),
        (F::ToggleBold, "Bold", "Mod+B"),
        (F::ToggleItalic, "Italic", "Mod+I"),
        (F::ToggleUnderline, "Underline", "Mod+U"),
        (F::ToggleStrike, "Strikethrough", ""),
        (F::ToggleCode, "Inline code", "Mod+E"),
        (F::SetParagraph, "Paragraph", ""),
        (F::SetHeading(1), "Heading 1", ""),
        (F::SetHeading(2), "Heading 2", ""),
        (F::SetHeading(3), "Heading 3", ""),
        (F::SetBlockquote, "Blockquote", ""),
        (F::SetCodeBlock(None), "Code block", ""),
        (F::InsertHorizontalRule, "Horizontal rule", ""),
        (F::ToggleBulletList, "Bullet list", ""),
        (F::ToggleOrderedList, "Ordered list", ""),
        (F::ToggleTaskList, "Task list", ""),
        (F::SinkListItem, "Indent list item", "Tab"),
        (F::LiftListItem, "Outdent list item", "Shift+Tab"),
        (F::InsertTable { rows: 2, cols: 2 }, "Insert table", ""),
        (F::ToggleHeaderRow, "Toggle table header row", ""),
    ]
}

// ── View + outcome (read-only render, shell applies the outcome) ──────────────────────────────────

/// Read-only inputs the editor sections render from: the live settings (the dialog already holds them)
/// plus the already-wired auto-save interval label to surface (AC: surface, do not re-add).
pub struct EditorSettingsView<'a> {
    /// The live workspace settings (editor prefs, syntax palette, editor keybinding overrides).
    pub settings: &'a WorkspaceSettingsState,
    /// The already-present auto-save interval display string (e.g. `"30s"`). Surfaced read-only inside
    /// the Editor section so the editor prefs read together WITHOUT this MT re-adding the field.
    pub auto_save_interval_label: &'a str,
}

/// What an editor-section control wants the shell to apply (and then persist via the existing debounced
/// `PUT`). At most one per frame (a single control interaction). The shell mutates the corresponding
/// field on [`WorkspaceSettingsState`] and schedules a save — the SAME path the WP-011 outcomes use, no
/// new save code (AC-009).
#[derive(Debug, Clone, PartialEq)]
pub enum EditorSectionOutcome {
    /// Nothing changed this frame.
    None,
    /// The whole editor-prefs group changed to this value (font/tab/insert-spaces/wrap/whitespace).
    EditorPrefsChanged(EditorPrefs),
    /// The syntax palette changed (mode and/or a Custom swatch).
    SyntaxPaletteChanged(SyntaxPalette),
    /// An editor keybinding override changed to `chord` (already trimmed; empty `chord` is ignored by
    /// the renderer, so this always carries a non-empty chord).
    EditorKeybindingChanged { action_id: String, chord: String },
    /// An editor keybinding override was reset to its built-in default (the override is removed).
    EditorKeybindingReset { action_id: String },
}

/// The transient per-open state for the editor keybinding rows: the in-progress draft chord text per
/// action id. Seeded from the live overrides on (re-)open and reset when the dialog reopens (keyed by
/// `open_count`, mirroring [`crate::settings_dialog`]'s keybinding drafts).
#[derive(Debug, Clone, Default)]
pub struct EditorSectionState {
    /// The open generation this state was initialized for.
    pub open_count: u64,
    /// In-progress draft chord text per editor action id (`(action_id, draft)`).
    drafts: Vec<(String, String)>,
}

impl EditorSectionState {
    /// (Re-)seed the drafts for `open_count` from the live overrides (the catalog default when an action
    /// has no override), if the open generation changed. Idempotent within one open generation.
    pub fn reseed_if_reopened(
        &mut self,
        open_count: u64,
        settings: &WorkspaceSettingsState,
        catalog: &[EditorAction],
    ) {
        if self.open_count == open_count && !self.drafts.is_empty() {
            return;
        }
        self.open_count = open_count;
        self.drafts = catalog
            .iter()
            .map(|action| {
                let current = settings
                    .editor_chord_override(&action.id)
                    .map(str::to_owned)
                    .unwrap_or_else(|| action.default_chord.clone());
                (action.id.clone(), current)
            })
            .collect();
    }

    /// The draft chord for `action_id`, if seeded/edited.
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
}

/// The editor settings section widget. Holds the action catalog (built once) + the transient row state;
/// renders the three sections from a read-only [`EditorSettingsView`] and returns an
/// [`EditorSectionOutcome`].
///
/// `Clone` so the dialog can stash it in egui temp memory (load -> render -> store) across frames.
#[derive(Clone)]
pub struct EditorSettingsSection {
    /// The full editor action catalog (code chords + rich commands) — the keybindings table source.
    pub catalog: Vec<EditorAction>,
    /// Transient per-open keybinding-row draft state.
    pub state: EditorSectionState,
}

impl Default for EditorSettingsSection {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorSettingsSection {
    /// Build the section with the full editor action catalog.
    pub fn new() -> Self {
        Self {
            catalog: editor_action_catalog(),
            state: EditorSectionState::default(),
        }
    }

    /// Render the **Editor** preferences group and return an outcome if a control changed. Renders the
    /// font-size + tab-size `DragValue`s, the insert-spaces `Checkbox`, the word-wrap + render-whitespace
    /// `ComboBox`es (plus the bounded-column `DragValue` when word_wrap = Bounded), and a read-only
    /// surface of the already-wired auto-save interval (AC: surfaced, not re-added).
    pub fn render_editor_prefs(
        &mut self,
        ui: &mut egui::Ui,
        view: &EditorSettingsView<'_>,
    ) -> EditorSectionOutcome {
        let mut prefs = view.settings.editor_prefs;
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Editor font size");
            let dv = ui.add(
                egui::DragValue::new(&mut prefs.editor_font_size)
                    .speed(0.5)
                    .range(crate::workspace_settings::EDITOR_FONT_SIZE_RANGE)
                    .suffix(" pt"),
            );
            set_author_id_and_label(ui, dv.id, EDITOR_FONT_SIZE_AUTHOR_ID, "Editor font size");
            changed |= dv.changed();
        });

        ui.horizontal(|ui| {
            ui.label("Tab size");
            let mut tab = prefs.tab_size as u32;
            let dv = ui.add(
                egui::DragValue::new(&mut tab)
                    .speed(1.0)
                    .range(
                        (*crate::workspace_settings::TAB_SIZE_RANGE.start() as u32)
                            ..=(*crate::workspace_settings::TAB_SIZE_RANGE.end() as u32),
                    ),
            );
            set_author_id_and_label(ui, dv.id, EDITOR_TAB_SIZE_AUTHOR_ID, "Tab size");
            if dv.changed() {
                prefs.tab_size = tab as u8;
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            let mut insert = prefs.insert_spaces;
            let label = if insert { "Insert spaces" } else { "Keep hard tabs" };
            let cb = ui.checkbox(&mut insert, label);
            set_author_id_and_label(
                ui,
                cb.id,
                EDITOR_INSERT_SPACES_AUTHOR_ID,
                "Insert spaces instead of tabs",
            );
            if cb.changed() {
                prefs.insert_spaces = insert;
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Word wrap");
            let mut selected = wrap_discriminant(prefs.word_wrap);
            let combo = egui::ComboBox::from_id_salt("settings.editor.word-wrap.combo")
                .selected_text(wrap_label(prefs.word_wrap))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, WrapKind::Off, "Off");
                    ui.selectable_value(&mut selected, WrapKind::On, "On (viewport)");
                    ui.selectable_value(&mut selected, WrapKind::Bounded, "Bounded column");
                });
            set_author_id_and_label(ui, combo.response.id, EDITOR_WORD_WRAP_AUTHOR_ID, "Word wrap mode");
            let new_wrap = match selected {
                WrapKind::Off => WordWrapMode::Off,
                WrapKind::On => WordWrapMode::On,
                WrapKind::Bounded => match prefs.word_wrap {
                    // Preserve an existing bounded column; default to 80 when switching INTO Bounded.
                    WordWrapMode::BoundedColumn(n) => WordWrapMode::BoundedColumn(n),
                    _ => WordWrapMode::BoundedColumn(80),
                },
            };
            if new_wrap != prefs.word_wrap {
                prefs.word_wrap = new_wrap;
                changed = true;
            }
        });

        // The bounded-column DragValue is shown only when word_wrap = Bounded.
        if let WordWrapMode::BoundedColumn(col) = prefs.word_wrap {
            ui.horizontal(|ui| {
                ui.label("Wrap column");
                let mut c = col as u32;
                let dv = ui.add(egui::DragValue::new(&mut c).speed(1.0).range(20u32..=400u32));
                set_author_id_and_label(ui, dv.id, EDITOR_WRAP_COLUMN_AUTHOR_ID, "Wrap column");
                if dv.changed() {
                    prefs.word_wrap = WordWrapMode::BoundedColumn(c.min(u16::MAX as u32) as u16);
                    changed = true;
                }
            });
        }

        ui.horizontal(|ui| {
            ui.label("Render whitespace");
            let mut selected = prefs.render_whitespace;
            let combo = egui::ComboBox::from_id_salt("settings.editor.render-whitespace.combo")
                .selected_text(whitespace_label(prefs.render_whitespace))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, RenderWhitespaceMode::None, "None");
                    ui.selectable_value(&mut selected, RenderWhitespaceMode::Boundary, "Boundary");
                    ui.selectable_value(&mut selected, RenderWhitespaceMode::All, "All");
                });
            set_author_id_and_label(
                ui,
                combo.response.id,
                EDITOR_RENDER_WHITESPACE_AUTHOR_ID,
                "Render whitespace mode",
            );
            if selected != prefs.render_whitespace {
                prefs.render_whitespace = selected;
                changed = true;
            }
        });

        // Surface (read-only) the already-present auto-save interval so the editor-prefs group reads as a
        // complete set WITHOUT this MT re-adding the field (it is owned elsewhere — surfaced, not owned).
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Auto-save interval").weak());
            ui.label(
                egui::RichText::new(format!("{} (configured elsewhere)", view.auto_save_interval_label))
                    .small()
                    .weak(),
            );
        });

        if changed {
            EditorSectionOutcome::EditorPrefsChanged(prefs)
        } else {
            EditorSectionOutcome::None
        }
    }

    /// Render the **Syntax** color-scheme palette editor and return an outcome if the mode or a Custom
    /// swatch changed. In Custom mode, one [`egui::Ui::color_edit_button_srgba`] swatch per
    /// [`HighlightScope`] writes into the palette's `custom` map; the live editor's
    /// [`resolve_scope_color`] picks the change up in the SAME frame (AC-003). A small live-preview swatch
    /// per scope shows the CURRENTLY-RESOLVED color (so the section visibly reflects the resolution).
    pub fn render_syntax_palette(
        &mut self,
        ui: &mut egui::Ui,
        view: &EditorSettingsView<'_>,
    ) -> EditorSectionOutcome {
        let mut palette = view.settings.syntax_palette.clone();
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Syntax color scheme");
            let mut selected = palette.mode;
            let combo = egui::ComboBox::from_id_salt("settings.syntax.palette-mode.combo")
                .selected_text(palette_mode_label(palette.mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, SyntaxPaletteMode::Muted, "Muted");
                    ui.selectable_value(&mut selected, SyntaxPaletteMode::Standard, "Standard");
                    ui.selectable_value(&mut selected, SyntaxPaletteMode::Custom, "Custom");
                });
            set_author_id_and_label(
                ui,
                combo.response.id,
                SYNTAX_PALETTE_MODE_AUTHOR_ID,
                "Syntax palette mode",
            );
            if selected != palette.mode {
                palette.mode = selected;
                changed = true;
            }
        });

        // One row per scope: a label, the live-resolved preview swatch, and (in Custom mode) an editable
        // swatch button. Every scope is always shown (no gap — AC-004).
        for scope in HighlightScope::ALL.iter().copied() {
            ui.horizontal(|ui| {
                ui.label(scope_label(scope));
                let resolved = resolve_scope_color(scope, &palette);
                // A small non-interactive preview of the currently-resolved color.
                let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 3.0, resolved);

                if palette.mode == SyntaxPaletteMode::Custom {
                    // The editable swatch: seed from the resolved color so the user starts from what they
                    // see; on change, store the sRGBA into the Custom map (live).
                    let mut color = resolved;
                    let sw = ui.color_edit_button_srgba(&mut color);
                    set_author_id_and_label(
                        ui,
                        sw.id,
                        &syntax_swatch_author_id(scope),
                        &format!("{} color", scope_label(scope)),
                    );
                    if sw.changed() {
                        palette.set_custom(scope.scope_key(), color.to_array());
                        changed = true;
                    }
                }
            });
        }

        if changed {
            EditorSectionOutcome::SyntaxPaletteChanged(palette)
        } else {
            EditorSectionOutcome::None
        }
    }

    /// Render the **Keybindings (editor extension)** table and return an outcome if a binding changed.
    /// Lists EVERY editor-specific action (code chords + rich commands — AC-005), each row with the
    /// label, an editable chord input (the current override or the default), a Reset button, and the
    /// built-in default shown as a hint. A captured custom binding overrides the default for that action
    /// (persisted into the SEPARATE `editor_keybindings` list, not the WP-011 map — RISK-001).
    ///
    /// `query` is the dialog's lowercased search query (so the editor rows participate in the existing
    /// settings search). `open_count` re-seeds the row drafts on (re-)open.
    pub fn render_editor_keybindings(
        &mut self,
        ui: &mut egui::Ui,
        view: &EditorSettingsView<'_>,
        query: &str,
        open_count: u64,
    ) -> EditorSectionOutcome {
        self.state
            .reseed_if_reopened(open_count, view.settings, &self.catalog);

        let mut outcome = EditorSectionOutcome::None;

        // Group the table by surface for legibility (Code first, then Rich), preserving catalog order.
        for surface in [EditorActionSurface::Code, EditorActionSurface::Rich] {
            let surface_actions: Vec<&EditorAction> = self
                .catalog
                .iter()
                .filter(|a| a.surface == surface)
                .filter(|a| action_matches_query(a, query))
                .collect();
            if surface_actions.is_empty() {
                continue;
            }
            ui.label(
                egui::RichText::new(match surface {
                    EditorActionSurface::Code => "Code editor",
                    EditorActionSurface::Rich => "Rich-text editor",
                })
                .small()
                .weak(),
            );
            for action in surface_actions {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(&action.label);
                        let default_hint = if action.default_chord.is_empty() {
                            "no default".to_owned()
                        } else {
                            format!("default: {}", action.default_chord)
                        };
                        ui.label(egui::RichText::new(default_hint).small().weak());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Reset clears the override (revert to the built-in default).
                        let reset = ui.button("Reset");
                        set_author_id(
                            ui,
                            reset.id,
                            &format!("{EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX}{}", action.id),
                        );
                        if reset.clicked() && outcome == EditorSectionOutcome::None {
                            self.state.set_draft(&action.id, action.default_chord.clone());
                            outcome = EditorSectionOutcome::EditorKeybindingReset {
                                action_id: action.id.clone(),
                            };
                        }

                        // Editable chord input bound to the draft.
                        let mut draft = self
                            .state
                            .draft_for(&action.id)
                            .map(str::to_owned)
                            .unwrap_or_else(|| action.default_chord.clone());
                        let input = ui.add(
                            egui::TextEdit::singleline(&mut draft)
                                .desired_width(150.0)
                                .hint_text(if action.default_chord.is_empty() {
                                    "unbound"
                                } else {
                                    action.default_chord.as_str()
                                }),
                        );
                        set_author_id_and_label(
                            ui,
                            input.id,
                            &editor_keybind_row_author_id(&action.id),
                            &format!("{} keybinding", action.label),
                        );
                        if input.changed() {
                            self.state.set_draft(&action.id, draft.clone());
                            let trimmed = draft.trim().to_owned();
                            // Only emit a change for a NON-empty chord (an empty input is treated as "in
                            // progress", not a binding); the shell normalizes + validates on apply.
                            if !trimmed.is_empty()
                                && trimmed != action.default_chord
                                && outcome == EditorSectionOutcome::None
                            {
                                outcome = EditorSectionOutcome::EditorKeybindingChanged {
                                    action_id: action.id.clone(),
                                    chord: trimmed,
                                };
                            }
                        }
                    });
                });
            }
        }

        outcome
    }
}

// ── ComboBox discriminants + labels (keep the closures simple/`Copy`-able) ────────────────────────

/// A `Copy` discriminant for the word-wrap ComboBox (`WordWrapMode::BoundedColumn` carries data, so the
/// selectable value must be a `Copy` tag, not the full enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WrapKind {
    Off,
    On,
    Bounded,
}

fn wrap_discriminant(mode: WordWrapMode) -> WrapKind {
    match mode {
        WordWrapMode::Off => WrapKind::Off,
        WordWrapMode::On => WrapKind::On,
        WordWrapMode::BoundedColumn(_) => WrapKind::Bounded,
    }
}

fn wrap_label(mode: WordWrapMode) -> String {
    match mode {
        WordWrapMode::Off => "Off".to_owned(),
        WordWrapMode::On => "On (viewport)".to_owned(),
        WordWrapMode::BoundedColumn(n) => format!("Bounded ({n})"),
    }
}

fn whitespace_label(mode: RenderWhitespaceMode) -> &'static str {
    match mode {
        RenderWhitespaceMode::None => "None",
        RenderWhitespaceMode::Boundary => "Boundary",
        RenderWhitespaceMode::All => "All",
    }
}

fn palette_mode_label(mode: SyntaxPaletteMode) -> &'static str {
    match mode {
        SyntaxPaletteMode::Muted => "Muted",
        SyntaxPaletteMode::Standard => "Standard",
        SyntaxPaletteMode::Custom => "Custom",
    }
}

/// The display label for a [`HighlightScope`] in the Custom swatch list.
fn scope_label(scope: HighlightScope) -> &'static str {
    match scope {
        HighlightScope::Keyword => "Keyword",
        HighlightScope::String => "String",
        HighlightScope::Comment => "Comment",
        HighlightScope::Number => "Number",
        HighlightScope::Function => "Function",
        HighlightScope::Type => "Type",
        HighlightScope::Operator => "Operator",
        HighlightScope::Other => "Other",
    }
}

/// True when an editor action matches the dialog's (already lowercased) search query: an empty query
/// matches everything; otherwise the label / bare id / default chord must contain the query.
fn action_matches_query(action: &EditorAction, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let haystack = format!(
        "keybinding shortcut {} {} {}",
        action.label, action.bare_id, action.default_chord
    )
    .to_lowercase();
    haystack.contains(query)
}

// ── AccessKit helpers (mirror settings_dialog.rs) ─────────────────────────────────────────────────

/// Attach a stable author_id to an already-interactive live node (the same helper shape
/// `settings_dialog.rs` uses).
fn set_author_id(ui: &egui::Ui, widget_id: egui::Id, author_id: &str) {
    let author_id = author_id.to_owned();
    ui.ctx()
        .accesskit_node_builder(widget_id, move |node| node.set_author_id(author_id));
}

/// Attach a stable author_id AND an accessible label to an already-interactive live node (for controls
/// whose accessible name is not derivable from rendered text — DragValue / ComboBox / TextEdit / swatch).
fn set_author_id_and_label(ui: &egui::Ui, widget_id: egui::Id, author_id: &str, label: &str) {
    let author_id = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(widget_id, move |node| {
        node.set_author_id(author_id);
        node.set_label(label);
    });
}

/// True when `role` is one AccessKit assigns to an interactive control (so the MT-025 interactive-naming
/// gate must see an author_id on it). Exposed for the section's own AccessKit tests.
pub fn is_control_role(role: accesskit::Role) -> bool {
    crate::accessibility::INTERACTIVE_ROLES.contains(&role)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_code_and_rich_actions_disjoint_ids() {
        let catalog = editor_action_catalog();
        assert!(!catalog.is_empty(), "catalog is non-empty");
        let code = catalog
            .iter()
            .filter(|a| a.surface == EditorActionSurface::Code)
            .count();
        let rich = catalog
            .iter()
            .filter(|a| a.surface == EditorActionSurface::Rich)
            .count();
        assert!(code >= 50, "all code-editor actions present (got {code})");
        assert!(rich >= 15, "rich-editor commands present (got {rich})");

        // Every id is prefix-namespaced + unique (no code/rich id collision — e.g. both have `undo`).
        let mut ids = std::collections::HashSet::new();
        for action in &catalog {
            assert!(
                action.id.starts_with(CODE_ACTION_ID_PREFIX)
                    || action.id.starts_with(RICH_ACTION_ID_PREFIX),
                "action id '{}' is namespaced",
                action.id
            );
            assert!(ids.insert(action.id.clone()), "duplicate action id '{}'", action.id);
        }
        // The `undo` collision is resolved by the prefix.
        assert!(ids.contains("code.undo"));
        assert!(ids.contains("rich.undo"));
    }

    #[test]
    fn author_ids_are_stable_kebab_case() {
        assert_eq!(EDITOR_FONT_SIZE_AUTHOR_ID, "settings-editor-font-size");
        assert_eq!(EDITOR_TAB_SIZE_AUTHOR_ID, "settings-editor-tab-size");
        assert_eq!(EDITOR_INSERT_SPACES_AUTHOR_ID, "settings-editor-insert-spaces");
        assert_eq!(EDITOR_WORD_WRAP_AUTHOR_ID, "settings-editor-word-wrap");
        assert_eq!(EDITOR_RENDER_WHITESPACE_AUTHOR_ID, "settings-editor-render-whitespace");
        assert_eq!(SYNTAX_PALETTE_MODE_AUTHOR_ID, "settings-syntax-palette-mode");
        assert_eq!(syntax_swatch_author_id(HighlightScope::Keyword), "settings-syntax-swatch-keyword");
        assert_eq!(
            editor_keybind_row_author_id("code.open_find"),
            "settings-keybind-row-code.open_find"
        );
    }

    #[test]
    fn reseed_resets_drafts_on_reopen() {
        let mut settings = crate::workspace_settings::default_workspace_settings_state();
        settings.set_editor_chord("code.open_find", "Mod+Alt+F".to_owned());
        let catalog = editor_action_catalog();
        let mut state = EditorSectionState::default();
        state.reseed_if_reopened(1, &settings, &catalog);
        assert_eq!(state.draft_for("code.open_find"), Some("Mod+Alt+F"));

        // A re-open with the override removed reseeds the draft back to the default.
        settings.clear_editor_chord("code.open_find");
        state.reseed_if_reopened(2, &settings, &catalog);
        let default_find = catalog
            .iter()
            .find(|a| a.id == "code.open_find")
            .unwrap()
            .default_chord
            .clone();
        assert_eq!(state.draft_for("code.open_find"), Some(default_find.as_str()));
    }
}
