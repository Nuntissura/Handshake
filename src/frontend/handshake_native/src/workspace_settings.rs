//! Workspace settings state + persistence for the native Handshake shell (WP-KERNEL-011 MT-018).
//!
//! ## What this module owns
//!
//! The Rust port of `app/src/lib/workspaceSettings.ts` (+ the genuinely-wired descriptors from
//! `app/src/lib/globalSettings.ts`): the [`WorkspaceSettingsState`] schema, the
//! [`APP_KEYBINDING_ACTIONS`] catalog, the chord-normalization / conflict-detection helpers, the
//! settings-search term matcher, and the `not yet wired` descriptors the settings dialog renders.
//!
//! ## Persistence (CX-503S / Data Posture — PostgreSQL-authoritative, NO local-file authority)
//!
//! Settings persist THROUGH the running `handshake_core` backend's PostgreSQL-authoritative REST
//! surface `GET`/`PUT /workspaces/:workspace_id/settings` (verified live in
//! `src/backend/handshake_core/src/api/workspaces.rs`). The HTTP transport is abstracted behind the
//! synchronous [`SettingsTransport`] seam so the load/save logic stays directly unit-testable with a
//! stub; the production [`SettingsClient`] bridges async reqwest onto the app's tokio runtime handle
//! (the MT-009 `WorkbenchLayoutClient` pattern). There is NO local JSON file, NO SQLite, and NO
//! alternate on-disk authority.
//!
//! ## Theme mapping
//!
//! [`WorkspaceTheme`] (`Light`/`Dark`) is the persisted, workspace-scoped theme. The in-memory shell
//! base theme [`crate::theme::HsTheme`] maps 1:1 to it via [`WorkspaceTheme::to_hs_theme`] /
//! [`WorkspaceTheme::from_hs_theme`], so the existing MT-003 theme flag is now BACKED by persisted
//! settings (the contract's "back the existing in-memory theme/view_mode flags with persisted
//! settings where they map").

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

use crate::theme::HsTheme;

/// Schema id stamped into every persisted settings blob, mirroring the React
/// `WORKSPACE_SETTINGS_SCHEMA_ID`. A blob whose `schema_id` differs is treated as foreign and the
/// fallback ([`default_workspace_settings_state`]) is used on load (red-team R6 / MC6).
pub const WORKSPACE_SETTINGS_SCHEMA_ID: &str = "hsk.workspace_settings_state@1";

/// Per-request timeout for the settings endpoint. A save/load must surface a slow/absent backend as a
/// transient error, never hang the worker thread. Matches the layout transport's 5s bound.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// The workspace-scoped shell theme. Port of the React `WorkspaceTheme` (`"light" | "dark"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTheme {
    /// Light shell theme.
    Light,
    /// Dark shell theme (the desktop production default).
    Dark,
}

impl WorkspaceTheme {
    /// The persisted string form (React parity: `"light"` / `"dark"`).
    pub fn as_str(self) -> &'static str {
        match self {
            WorkspaceTheme::Light => "light",
            WorkspaceTheme::Dark => "dark",
        }
    }

    /// Parse the persisted string form; `None` for any other value (so the caller can fall back).
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "light" => Some(WorkspaceTheme::Light),
            "dark" => Some(WorkspaceTheme::Dark),
            _ => None,
        }
    }

    /// Map to the in-memory shell base theme ([`HsTheme`]). The two enums are 1:1; this is the bridge
    /// that lets the persisted theme drive the MT-003 `current_theme` flag.
    pub fn to_hs_theme(self) -> HsTheme {
        match self {
            WorkspaceTheme::Light => HsTheme::Light,
            WorkspaceTheme::Dark => HsTheme::Dark,
        }
    }

    /// Map from the in-memory shell base theme ([`HsTheme`]) to the persisted theme.
    pub fn from_hs_theme(theme: HsTheme) -> Self {
        match theme {
            HsTheme::Light => WorkspaceTheme::Light,
            HsTheme::Dark => WorkspaceTheme::Dark,
        }
    }
}

/// The persisted content-presentation mode. Mirrors the React `ViewMode` (`"NSFW" | "SFW"`). Kept as
/// a distinct persisted enum (rather than reusing `crate::app::ViewMode`) so this module has no
/// dependency cycle back into `app`; the dialog maps between the two at the seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsViewMode {
    /// Adult content surfaces shown (the production default).
    Nsfw,
    /// Adult content surfaces hidden.
    Sfw,
}

impl SettingsViewMode {
    /// The persisted string form (React parity: `"NSFW"` / `"SFW"`).
    pub fn as_str(self) -> &'static str {
        match self {
            SettingsViewMode::Nsfw => "NSFW",
            SettingsViewMode::Sfw => "SFW",
        }
    }

    /// Parse the persisted string form; `None` for any other value (so the caller can fall back).
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "NSFW" => Some(SettingsViewMode::Nsfw),
            "SFW" => Some(SettingsViewMode::Sfw),
            _ => None,
        }
    }
}

/// A keybinding action descriptor. Port of the React `AppKeybindingDescriptor`.
#[derive(Debug, Clone, Copy)]
pub struct AppKeybindingAction {
    /// Stable id (`app.quick_switcher.open` / `app.command_palette.open`).
    pub id: &'static str,
    /// Display label shown in the keybindings section.
    pub label: &'static str,
    /// One-line description shown under the label.
    pub description: &'static str,
    /// The default chord, in canonical form.
    pub default_chord: &'static str,
    /// Search keywords for the settings search filter.
    pub keywords: &'static [&'static str],
}

/// The app-level keybinding actions, in the React `APP_KEYBINDING_ACTIONS` order. Currently two:
/// the quick switcher (`Mod-p`) and the command palette (`Mod-Shift-p`).
pub const APP_KEYBINDING_ACTIONS: &[AppKeybindingAction] = &[
    AppKeybindingAction {
        id: "app.quick_switcher.open",
        label: "Quick Switcher",
        description: "Open the workspace-wide quick switcher.",
        default_chord: "Mod-p",
        keywords: &["quick", "switcher", "open", "workspace", "search"],
    },
    AppKeybindingAction {
        id: "app.command_palette.open",
        label: "Command Palette",
        description: "Open app-level commands.",
        default_chord: "Mod-Shift-p",
        keywords: &["command", "palette", "commands", "open"],
    },
];

/// One keybinding chord assignment (action id -> chord). The settings state stores one per action in
/// `APP_KEYBINDING_ACTIONS` order. A `Vec` (not a map) keeps deterministic order for serialization +
/// rendering; lookups are by the small fixed action set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keybinding {
    /// The action id this binding is for (one of `APP_KEYBINDING_ACTIONS[*].id`).
    pub action_id: String,
    /// The currently-assigned chord (canonical form once normalized).
    pub chord: String,
}

// ===========================================================================
// WP-KERNEL-012 MT-072 — editor-specific settings (EditorPrefs + SyntaxPalette
// + editor keybinding overrides). These are NEW NESTED FIELDS appended into the
// SAME WorkspaceSettingsState the WP-011 dialog already serializes through the
// existing PostgreSQL-backed GET/PUT /workspaces/:id/settings surface. No new
// persistence system, no SQLite, no new endpoint (AC-009).
//
// PERSISTENCE-AUTHORITY NOTE (the load-bearing extensibility question — RISK-001):
// the backend `validate_workspace_settings_state_shape`
// (src/backend/handshake_core/src/storage/postgres.rs) inspects ONLY the known
// top-level keys (`theme`, `custom_theme_tokens`, `keybindings`, `settings`) and
// NEVER rejects EXTRA top-level keys. So `editor_prefs` and `syntax_palette`,
// emitted as NEW TOP-LEVEL keys, ride the opaque settings_state JSON through PUT
// and are stored verbatim in PostgreSQL (verified read-only against the backend
// validator). The `keybindings` map, however, IS deny-unknown on the backend
// (its keys MUST equal exactly ["app.quick_switcher.open", "app.command_palette.open"]),
// so editor keybinding OVERRIDES are kept in a SEPARATE top-level `editor_keybindings`
// list, NOT written into the shared `keybindings` map — writing them there would
// hard-fail every PUT for the workspace (RISK-001 realized). See the MT handoff
// typed blocker for the editor-keybindings-into-shared-map path.
// ===========================================================================

/// The editor word-wrap mode. Port of the VS Code `editor.wordWrap` setting
/// (`off` | `on` | `wordWrapColumn`). `BoundedColumn(n)` wraps at a fixed column `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordWrapMode {
    /// No wrapping (long lines scroll horizontally). Default.
    Off,
    /// Wrap at the viewport edge.
    On,
    /// Wrap at a fixed column.
    BoundedColumn(u16),
}

impl WordWrapMode {
    /// The default editor wrap mode (Off — VS Code parity).
    pub const fn default_mode() -> Self {
        WordWrapMode::Off
    }

    /// Serialize to the persisted JSON value. `Off`/`On` are strings; `BoundedColumn` is
    /// `{"boundedColumn": n}` so the column round-trips.
    pub fn to_json(self) -> Value {
        match self {
            WordWrapMode::Off => Value::String("off".to_owned()),
            WordWrapMode::On => Value::String("on".to_owned()),
            WordWrapMode::BoundedColumn(n) => {
                serde_json::json!({ "boundedColumn": n })
            }
        }
    }

    /// Parse from the persisted JSON value; `None` for an unrecognized shape so the caller falls back.
    pub fn from_json(value: &Value) -> Option<Self> {
        match value {
            Value::String(s) => match s.as_str() {
                "off" => Some(WordWrapMode::Off),
                "on" => Some(WordWrapMode::On),
                _ => None,
            },
            Value::Object(map) => map
                .get("boundedColumn")
                .and_then(Value::as_u64)
                .map(|n| WordWrapMode::BoundedColumn(n.min(u16::MAX as u64) as u16)),
            _ => None,
        }
    }
}

/// Whether the code editor draws whitespace glyphs. Port of VS Code `editor.renderWhitespace`
/// (`none` | `boundary` | `all`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderWhitespaceMode {
    /// Never draw whitespace glyphs. Default.
    None,
    /// Draw whitespace at word boundaries (between words, not inside indentation runs).
    Boundary,
    /// Draw every whitespace glyph.
    All,
}

impl RenderWhitespaceMode {
    /// The default render-whitespace mode (None).
    pub const fn default_mode() -> Self {
        RenderWhitespaceMode::None
    }

    /// The persisted string form.
    pub fn as_str(self) -> &'static str {
        match self {
            RenderWhitespaceMode::None => "none",
            RenderWhitespaceMode::Boundary => "boundary",
            RenderWhitespaceMode::All => "all",
        }
    }

    /// Parse the persisted string form; `None` for an unrecognized value.
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "none" => Some(RenderWhitespaceMode::None),
            "boundary" => Some(RenderWhitespaceMode::Boundary),
            "all" => Some(RenderWhitespaceMode::All),
            _ => None,
        }
    }

    /// Whether this mode draws ANY whitespace glyphs (the boolean the code editor's existing
    /// `set_render_whitespace` slot consumes — Boundary and All both enable drawing).
    pub fn draws_whitespace(self) -> bool {
        !matches!(self, RenderWhitespaceMode::None)
    }
}

/// User-configurable editor text preferences, distinct from the UI/chrome appearance the
/// Appearance section manages. Editor text surfaces read these; the app chrome does NOT.
///
/// `editor_font_size` is DELIBERATELY a separate field from the WP-011 chrome/UI font size
/// (RISK-002 / AC-002): mutating the editor font size MUST NOT resize the app chrome and vice
/// versa. (The WP-011 shell has no persisted chrome-font field today — chrome uses egui's default
/// text styles — so "separate field" is satisfied structurally: `editor_font_size` lives ONLY
/// under `editor_prefs` and feeds ONLY the editor text surfaces.)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorPrefs {
    /// Point size used ONLY by editor text surfaces. Clamped to 6.0..=48.0 by the UI.
    pub editor_font_size: f32,
    /// Columns per tab (indent unit). Clamped to 1..=16 by the UI. Default 4.
    pub tab_size: u8,
    /// Tabs-vs-spaces: `true` inserts `tab_size` spaces, `false` keeps hard tabs. Default `true`.
    pub insert_spaces: bool,
    /// The default editor word-wrap mode. Default `Off`.
    pub word_wrap: WordWrapMode,
    /// Whether the code editor draws whitespace glyphs. Default `None`.
    pub render_whitespace: RenderWhitespaceMode,
}

impl Default for EditorPrefs {
    fn default() -> Self {
        Self {
            editor_font_size: DEFAULT_EDITOR_FONT_SIZE,
            tab_size: 4,
            insert_spaces: true,
            word_wrap: WordWrapMode::default_mode(),
            render_whitespace: RenderWhitespaceMode::default_mode(),
        }
    }
}

/// The default editor font point size. Matches the code editor's current `MONO_FONT_SIZE` (13.0)
/// so a fresh workspace renders editor text at the same size as before this MT.
pub const DEFAULT_EDITOR_FONT_SIZE: f32 = 13.0;

/// The inclusive editor-font-size clamp range (the UI `DragValue` enforces it; the loader clamps a
/// stored value too, so a hand-edited out-of-range PostgreSQL row cannot smuggle an invalid size).
pub const EDITOR_FONT_SIZE_RANGE: std::ops::RangeInclusive<f32> = 6.0..=48.0;

/// The inclusive tab-size clamp range.
pub const TAB_SIZE_RANGE: std::ops::RangeInclusive<u8> = 1..=16;

impl EditorPrefs {
    /// Serialize to the persisted JSON object.
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "editor_font_size": self.editor_font_size,
            "tab_size": self.tab_size,
            "insert_spaces": self.insert_spaces,
            "word_wrap": self.word_wrap.to_json(),
            "render_whitespace": self.render_whitespace.as_str(),
        })
    }

    /// Parse from the persisted JSON value, using `fallback` for any missing/invalid field. A non-object
    /// or absent `editor_prefs` key yields `fallback` wholesale (AC-006 legacy compat — a WP-011-era row
    /// has no `editor_prefs` key, so the defaults are used and the dialog opens normally).
    pub fn from_json(value: Option<&Value>, fallback: &EditorPrefs) -> EditorPrefs {
        let Some(obj) = value.and_then(Value::as_object) else {
            return *fallback;
        };
        let editor_font_size = obj
            .get("editor_font_size")
            .and_then(Value::as_f64)
            .map(|v| {
                (v as f32).clamp(
                    *EDITOR_FONT_SIZE_RANGE.start(),
                    *EDITOR_FONT_SIZE_RANGE.end(),
                )
            })
            .unwrap_or(fallback.editor_font_size);
        let tab_size = obj
            .get("tab_size")
            .and_then(Value::as_u64)
            .map(|v| (v as u8).clamp(*TAB_SIZE_RANGE.start(), *TAB_SIZE_RANGE.end()))
            .unwrap_or(fallback.tab_size);
        let insert_spaces = obj
            .get("insert_spaces")
            .and_then(Value::as_bool)
            .unwrap_or(fallback.insert_spaces);
        let word_wrap = obj
            .get("word_wrap")
            .and_then(WordWrapMode::from_json)
            .unwrap_or(fallback.word_wrap);
        let render_whitespace = obj
            .get("render_whitespace")
            .and_then(Value::as_str)
            .and_then(RenderWhitespaceMode::from_str_opt)
            .unwrap_or(fallback.render_whitespace);
        EditorPrefs {
            editor_font_size,
            tab_size,
            insert_spaces,
            word_wrap,
            render_whitespace,
        }
    }
}

/// The syntax color-scheme palette mode. `Custom` exposes a per-scope swatch editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxPaletteMode {
    /// The muted built-in palette.
    Muted,
    /// The standard (VS-Code-like) built-in palette. Default.
    Standard,
    /// User-edited per-scope colors (falling back to Standard for any un-overridden scope).
    Custom,
}

impl SyntaxPaletteMode {
    /// The persisted string form.
    pub fn as_str(self) -> &'static str {
        match self {
            SyntaxPaletteMode::Muted => "muted",
            SyntaxPaletteMode::Standard => "standard",
            SyntaxPaletteMode::Custom => "custom",
        }
    }

    /// Parse the persisted string form; `None` for an unrecognized value.
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "muted" => Some(SyntaxPaletteMode::Muted),
            "standard" => Some(SyntaxPaletteMode::Standard),
            "custom" => Some(SyntaxPaletteMode::Custom),
            _ => None,
        }
    }
}

/// The stable string key for a [`crate::code_editor::HighlightScope`] used in the persisted
/// `syntax_palette.custom` map. A small fixed vocabulary (one per HighlightScope variant) so the
/// Custom swatch map round-trips through JSON object keys. Mirrors the variant names lowercased.
pub const SYNTAX_SCOPE_KEYS: &[&str] = &[
    "keyword", "string", "comment", "number", "function", "type", "operator", "other",
];

/// The syntax color-scheme palette: the mode plus, for `Custom`, a per-scope sRGBA override map keyed
/// by [`SYNTAX_SCOPE_KEYS`]. Stored as `[u8; 4]` sRGBA so it round-trips through JSON as a 4-element
/// array and converts to `egui::Color32` via `Color32::from_rgba_unmultiplied` for live use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxPalette {
    /// The active palette mode.
    pub mode: SyntaxPaletteMode,
    /// Per-scope sRGBA overrides (only consulted when `mode == Custom`). Keyed by [`SYNTAX_SCOPE_KEYS`].
    /// Absent scopes fall back to the Standard built-in palette (no missing scope — AC-004).
    pub custom: std::collections::HashMap<String, [u8; 4]>,
}

impl Default for SyntaxPalette {
    fn default() -> Self {
        Self {
            mode: SyntaxPaletteMode::Standard,
            custom: std::collections::HashMap::new(),
        }
    }
}

impl SyntaxPalette {
    /// The sRGBA override for `scope_key` if the user has set one in Custom mode (always `None` when
    /// the key is absent — the caller falls back to a built-in palette table).
    pub fn custom_for(&self, scope_key: &str) -> Option<[u8; 4]> {
        self.custom.get(scope_key).copied()
    }

    /// Set (or replace) the Custom override for `scope_key`.
    pub fn set_custom(&mut self, scope_key: &str, rgba: [u8; 4]) {
        self.custom.insert(scope_key.to_owned(), rgba);
    }

    /// Serialize to the persisted JSON object: `{ "mode": "...", "custom": { "keyword": [r,g,b,a], ... } }`.
    /// Custom entries are emitted in [`SYNTAX_SCOPE_KEYS`] order for deterministic output.
    pub fn to_json(&self) -> Value {
        let mut custom = serde_json::Map::new();
        for key in SYNTAX_SCOPE_KEYS {
            if let Some(rgba) = self.custom.get(*key) {
                custom.insert(
                    (*key).to_owned(),
                    Value::Array(rgba.iter().map(|c| Value::from(*c)).collect()),
                );
            }
        }
        serde_json::json!({
            "mode": self.mode.as_str(),
            "custom": Value::Object(custom),
        })
    }

    /// Parse from the persisted JSON value, using `fallback` for a missing/invalid mode. A non-object or
    /// absent `syntax_palette` key yields `fallback` wholesale (AC-006 legacy compat). Only well-formed
    /// `[u8;4]` entries keyed by a known [`SYNTAX_SCOPE_KEYS`] scope are taken into `custom`.
    pub fn from_json(value: Option<&Value>, fallback: &SyntaxPalette) -> SyntaxPalette {
        let Some(obj) = value.and_then(Value::as_object) else {
            return fallback.clone();
        };
        let mode = obj
            .get("mode")
            .and_then(Value::as_str)
            .and_then(SyntaxPaletteMode::from_str_opt)
            .unwrap_or(fallback.mode);
        let mut custom = std::collections::HashMap::new();
        if let Some(custom_obj) = obj.get("custom").and_then(Value::as_object) {
            for key in SYNTAX_SCOPE_KEYS {
                if let Some(arr) = custom_obj.get(*key).and_then(Value::as_array) {
                    if arr.len() == 4 {
                        let mut rgba = [0u8; 4];
                        let mut ok = true;
                        for (i, c) in arr.iter().enumerate() {
                            match c.as_u64() {
                                Some(v) if v <= 255 => rgba[i] = v as u8,
                                _ => {
                                    ok = false;
                                    break;
                                }
                            }
                        }
                        if ok {
                            custom.insert((*key).to_owned(), rgba);
                        }
                    }
                }
            }
        }
        SyntaxPalette { mode, custom }
    }
}

/// One editor keybinding OVERRIDE (action id -> chord). Stored in a SEPARATE top-level
/// `editor_keybindings` list, NOT in the WP-011 `keybindings` map (which the backend validates with a
/// fixed action-id allowlist — RISK-001). The action id is a code-editor action `name()`
/// (snake_case, e.g. `"open_find"`) or a rich-editor `command_id()` (e.g. `"toggle_bold"`); the chord
/// is the human-readable form (`"Ctrl+Shift+P"`). Default-vs-custom is resolved by
/// "custom if present in this list else the built-in default" (AC-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorKeybinding {
    /// The editor action id this override is for.
    pub action_id: String,
    /// The chord the user bound it to.
    pub chord: String,
}

/// The full persisted workspace settings state. Port of the React `WorkspaceSettingsState`.
///
/// `PartialEq` only (not `Eq`) since MT-072's [`EditorPrefs::editor_font_size`] is an `f32`.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSettingsState {
    /// Workspace-scoped shell theme (wired: drives `HsTheme`).
    pub theme: WorkspaceTheme,
    /// Per-action keybinding chords, in `APP_KEYBINDING_ACTIONS` order.
    pub keybindings: Vec<Keybinding>,
    /// Content-presentation mode (wired).
    pub view_mode: SettingsViewMode,
    /// Whether the Swarm Board opens on launch (wired).
    pub swarm_board_default_open: bool,
    /// MT-072 editor text preferences (font size, tab size, tabs-vs-spaces, word wrap, whitespace).
    /// A NEW nested field: rides the SAME serialized struct through the existing PUT/GET surface.
    pub editor_prefs: EditorPrefs,
    /// MT-072 syntax color-scheme palette (Muted | Standard | Custom + per-scope overrides).
    pub syntax_palette: SyntaxPalette,
    /// MT-072 editor keybinding OVERRIDES (kept SEPARATE from the WP-011 `keybindings` map — the
    /// backend deny-unknown-validates that map's keys, so editor bindings live here; RISK-001).
    pub editor_keybindings: Vec<EditorKeybinding>,
}

impl WorkspaceSettingsState {
    /// The chord currently assigned to `action_id`, if present.
    pub fn chord_for(&self, action_id: &str) -> Option<&str> {
        self.keybindings
            .iter()
            .find(|b| b.action_id == action_id)
            .map(|b| b.chord.as_str())
    }

    /// Set (or insert) the chord for `action_id`, preserving `APP_KEYBINDING_ACTIONS` order.
    pub fn set_chord(&mut self, action_id: &str, chord: String) {
        if let Some(existing) = self
            .keybindings
            .iter_mut()
            .find(|b| b.action_id == action_id)
        {
            existing.chord = chord;
        } else {
            self.keybindings.push(Keybinding {
                action_id: action_id.to_owned(),
                chord,
            });
        }
    }

    /// MT-072: the custom editor-keybinding override for `action_id`, if the user has set one. `None`
    /// means "use the action's built-in default" (default-vs-custom resolution — AC-005).
    pub fn editor_chord_override(&self, action_id: &str) -> Option<&str> {
        self.editor_keybindings
            .iter()
            .find(|b| b.action_id == action_id)
            .map(|b| b.chord.as_str())
    }

    /// MT-072: set (or replace) the editor-keybinding override for `action_id`. Writes into the
    /// SEPARATE `editor_keybindings` list (NOT the WP-011 `keybindings` map — RISK-001), so the custom
    /// binding overrides the default for that action without touching the backend-validated map.
    pub fn set_editor_chord(&mut self, action_id: &str, chord: String) {
        if let Some(existing) = self
            .editor_keybindings
            .iter_mut()
            .find(|b| b.action_id == action_id)
        {
            existing.chord = chord;
        } else {
            self.editor_keybindings.push(EditorKeybinding {
                action_id: action_id.to_owned(),
                chord,
            });
        }
    }

    /// MT-072: remove the editor-keybinding override for `action_id` (revert to the built-in default).
    /// Returns `true` if an override was removed.
    pub fn clear_editor_chord(&mut self, action_id: &str) -> bool {
        let before = self.editor_keybindings.len();
        self.editor_keybindings.retain(|b| b.action_id != action_id);
        self.editor_keybindings.len() != before
    }

    /// Serialize to the backend `settings_state` JSON shape (React `WorkspaceSettingsState` JSON).
    /// `keybindings` is emitted as a JSON object keyed by action id (React parity), `settings` is the
    /// nested object the React schema uses for view_mode + swarm_board_default_open.
    pub fn to_settings_state(&self) -> Value {
        let mut keybindings = serde_json::Map::new();
        for binding in &self.keybindings {
            keybindings.insert(
                binding.action_id.clone(),
                Value::String(binding.chord.clone()),
            );
        }
        // MT-072: editor keybinding overrides emit as a separate top-level array (NOT into the
        // backend-validated `keybindings` map — RISK-001). Each entry is `{action, chord}`.
        let editor_keybindings: Vec<Value> = self
            .editor_keybindings
            .iter()
            .map(|b| serde_json::json!({ "action": b.action_id, "chord": b.chord }))
            .collect();
        serde_json::json!({
            "schema_id": WORKSPACE_SETTINGS_SCHEMA_ID,
            "theme": self.theme.as_str(),
            "custom_theme_tokens": {},
            "keybindings": Value::Object(keybindings),
            "settings": {
                "view_mode": self.view_mode.as_str(),
                "swarm_board_default_open": self.swarm_board_default_open,
            },
            // MT-072 NEW top-level keys (the backend stores settings_state as an opaque JSON object and
            // does NOT reject unknown top-level keys — verified read-only against the backend validator).
            "editor_prefs": self.editor_prefs.to_json(),
            "syntax_palette": self.syntax_palette.to_json(),
            "editor_keybindings": Value::Array(editor_keybindings),
        })
    }
}

/// The default settings state for a fresh workspace. Port of `defaultWorkspaceSettingsState`. The
/// React default theme is `"light"`; the native desktop default is Dark (mirroring the shell's
/// `HsTheme::Dark` default and the React app's dark desktop default), so the native default theme is
/// Dark. The keybindings default to each action's `default_chord`.
pub fn default_workspace_settings_state() -> WorkspaceSettingsState {
    WorkspaceSettingsState {
        theme: WorkspaceTheme::Dark,
        keybindings: APP_KEYBINDING_ACTIONS
            .iter()
            .map(|action| Keybinding {
                action_id: action.id.to_owned(),
                chord: action.default_chord.to_owned(),
            })
            .collect(),
        view_mode: SettingsViewMode::Nsfw,
        swarm_board_default_open: false,
        // MT-072 editor settings default to the built-in editor prefs + Standard syntax palette + no
        // editor keybinding overrides (every editor action keeps its built-in default chord).
        editor_prefs: EditorPrefs::default(),
        syntax_palette: SyntaxPalette::default(),
        editor_keybindings: Vec::new(),
    }
}

/// Normalize a keybinding chord to canonical form. Verbatim port of the React `normalizeChordInput`:
/// split on `-`, trim + drop empties, classify each modifier (Mod/Cmd/Command/Meta/Ctrl/Control ->
/// `Mod`, Shift -> `Shift`, Alt/Option -> `Alt`), reassemble in the canonical `Mod-Alt-Shift-key`
/// order, lowercasing a single-character key. An empty/blank input normalizes to `""`.
pub fn normalize_chord_input(chord: &str) -> String {
    let parts: Vec<&str> = chord
        .split('-')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return String::new();
    }

    let key = parts[parts.len() - 1];
    let mut has_mod = false;
    let mut has_shift = false;
    let mut has_alt = false;
    for part in &parts[..parts.len() - 1] {
        match part.to_lowercase().as_str() {
            "mod" | "cmd" | "command" | "meta" | "ctrl" | "control" => has_mod = true,
            "shift" => has_shift = true,
            "alt" | "option" => has_alt = true,
            _ => {}
        }
    }

    let mut ordered: Vec<String> = Vec::new();
    if has_mod {
        ordered.push("Mod".to_owned());
    }
    if has_alt {
        ordered.push("Alt".to_owned());
    }
    if has_shift {
        ordered.push("Shift".to_owned());
    }
    let normalized_key = if key.chars().count() == 1 {
        key.to_lowercase()
    } else {
        key.to_owned()
    };
    ordered.push(normalized_key);
    ordered.join("-")
}

/// One keybinding conflict: a chord shared by two or more actions. Port of the React
/// `KeybindingConflict`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindingConflict {
    /// The shared (normalized) chord.
    pub chord: String,
    /// The labels of the actions that share it, in `APP_KEYBINDING_ACTIONS` order.
    pub action_labels: Vec<String>,
}

/// Find every chord shared by two or more keybindings. Port of the React `findKeybindingConflicts`:
/// normalize each action's chord, group by the normalized chord (skipping empties), and return the
/// groups with more than one action. Iterates `APP_KEYBINDING_ACTIONS` so the action-label order in a
/// conflict is deterministic.
pub fn find_keybinding_conflicts(settings: &WorkspaceSettingsState) -> Vec<KeybindingConflict> {
    // Preserve first-seen chord order so the conflict list is deterministic.
    let mut order: Vec<String> = Vec::new();
    let mut labels_by_chord: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for action in APP_KEYBINDING_ACTIONS {
        let chord = normalize_chord_input(settings.chord_for(action.id).unwrap_or(""));
        if chord.is_empty() {
            continue;
        }
        if !labels_by_chord.contains_key(&chord) {
            order.push(chord.clone());
        }
        labels_by_chord
            .entry(chord)
            .or_default()
            .push(action.label.to_owned());
    }
    order
        .into_iter()
        .filter_map(|chord| {
            let labels = labels_by_chord.remove(&chord).unwrap_or_default();
            if labels.len() > 1 {
                Some(KeybindingConflict {
                    chord,
                    action_labels: labels,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Format the action labels in a conflict for display. Port of the React `keybindingLabelForConflict`:
/// `"A"` for one, `"A and B"` for two, `"A, B and C"` for three+ (note the Oxford-comma-free join with
/// a trailing " and " before the last label — matching the React implementation exactly).
pub fn keybinding_label_for_conflict(labels: &[String]) -> String {
    match labels.len() {
        0 => String::new(),
        1 => labels[0].clone(),
        _ => {
            let head = labels[..labels.len() - 1].join(", ");
            format!("{head} and {}", labels[labels.len() - 1])
        }
    }
}

/// True when `query` (already lowercased + trimmed by the caller) matches the joined `terms`. Port of
/// the React `settingMatchesQuery`: an empty query matches everything; otherwise the space-joined,
/// lowercased terms must contain the query substring.
pub fn setting_matches_query(query: &str, terms: &[&str]) -> bool {
    if query.is_empty() {
        return true;
    }
    terms.join(" ").to_lowercase().contains(query)
}

/// Parse a backend `settings_state` blob into a [`WorkspaceSettingsState`], using `fallback` for any
/// missing/invalid field. Port of the React `normalizeWorkspaceSettingsState`:
/// - a non-object, or a `schema_id` mismatch, returns the fallback wholesale (red-team R6 / MC6);
/// - each keybinding is taken from the blob only when it normalizes to a non-empty chord, else the
///   fallback's chord for that action is kept;
/// - theme / view_mode / swarm_board_default_open fall back per-field on an invalid value.
pub fn normalize_workspace_settings_state(
    value: &Value,
    fallback: &WorkspaceSettingsState,
) -> WorkspaceSettingsState {
    let Some(obj) = value.as_object() else {
        return fallback.clone();
    };
    if obj.get("schema_id").and_then(Value::as_str) != Some(WORKSPACE_SETTINGS_SCHEMA_ID) {
        return fallback.clone();
    }

    let raw_keybindings = obj.get("keybindings").and_then(Value::as_object);
    let mut keybindings: Vec<Keybinding> = Vec::with_capacity(APP_KEYBINDING_ACTIONS.len());
    for action in APP_KEYBINDING_ACTIONS {
        let fallback_chord = fallback
            .chord_for(action.id)
            .unwrap_or(action.default_chord)
            .to_owned();
        let chord = raw_keybindings
            .and_then(|m| m.get(action.id))
            .and_then(Value::as_str)
            .map(normalize_chord_input)
            .filter(|c| !c.is_empty())
            .unwrap_or(fallback_chord);
        keybindings.push(Keybinding {
            action_id: action.id.to_owned(),
            chord,
        });
    }

    let theme = obj
        .get("theme")
        .and_then(Value::as_str)
        .and_then(WorkspaceTheme::from_str_opt)
        .unwrap_or(fallback.theme);

    let raw_settings = obj.get("settings").and_then(Value::as_object);
    let view_mode = raw_settings
        .and_then(|m| m.get("view_mode"))
        .and_then(Value::as_str)
        .and_then(SettingsViewMode::from_str_opt)
        .unwrap_or(fallback.view_mode);
    let swarm_board_default_open = raw_settings
        .and_then(|m| m.get("swarm_board_default_open"))
        .and_then(Value::as_bool)
        .unwrap_or(fallback.swarm_board_default_open);

    // MT-072: the three new top-level keys parse with a per-field fallback so a WP-011-era stored
    // document (which lacks them entirely) deserializes cleanly to the defaults (AC-006 legacy compat —
    // this is the manual-round-trip analogue of #[serde(default)]; the struct uses an explicit
    // normalize, not serde-derive, so "absent => fallback" is implemented here per field).
    let editor_prefs = EditorPrefs::from_json(obj.get("editor_prefs"), &fallback.editor_prefs);
    let syntax_palette =
        SyntaxPalette::from_json(obj.get("syntax_palette"), &fallback.syntax_palette);
    let editor_keybindings = match obj.get("editor_keybindings").and_then(Value::as_array) {
        Some(arr) => arr
            .iter()
            .filter_map(|entry| {
                let o = entry.as_object()?;
                let action_id = o.get("action").and_then(Value::as_str)?.to_owned();
                let chord = o.get("chord").and_then(Value::as_str)?.to_owned();
                if action_id.is_empty() || chord.is_empty() {
                    return None;
                }
                Some(EditorKeybinding { action_id, chord })
            })
            .collect(),
        None => fallback.editor_keybindings.clone(),
    };

    WorkspaceSettingsState {
        theme,
        keybindings,
        view_mode,
        swarm_board_default_open,
        editor_prefs,
        syntax_palette,
        editor_keybindings,
    }
}

/// A setting that exists in the UI but has no backing setter yet. Port of the React
/// `NotYetWiredSetting`. Rendered as a disabled/read-only row with a visible note.
#[derive(Debug, Clone, Copy)]
pub struct NotYetWiredSetting {
    /// Stable id used to derive the row's stable author_id.
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
    /// The fixed value the control is pinned to.
    pub fixed_value: &'static str,
    /// The "not yet wired" note shown under the label.
    pub note: &'static str,
}

/// Swarm board auto-reconcile cadence — port of `SWARM_RECONCILE_INTERVAL_SETTING`.
pub const SWARM_RECONCILE_INTERVAL_SETTING: NotYetWiredSetting = NotYetWiredSetting {
    id: "swarm-reconcile-interval",
    label: "Swarm board auto-reconcile interval",
    fixed_value: "10s",
    note: "Not yet wired - fixed at 10s",
};

/// Swarm resource poll cadence — port of `SWARM_RESOURCE_POLL_INTERVAL_SETTING`.
pub const SWARM_RESOURCE_POLL_INTERVAL_SETTING: NotYetWiredSetting = NotYetWiredSetting {
    id: "swarm-resource-poll-interval",
    label: "Swarm resource poll interval",
    fixed_value: "1.5s",
    note: "Not yet wired - fixed at 1.5s",
};

/// Terminal default shell — port of `TERMINAL_DEFAULT_SHELL_SETTING`.
pub const TERMINAL_DEFAULT_SHELL_SETTING: NotYetWiredSetting = NotYetWiredSetting {
    id: "terminal-default-shell",
    label: "Terminal default shell",
    fixed_value: "System default (backend-chosen)",
    note: "Not yet wired - backend picks the shell",
};

/// Terminal max scrollback — port of `TERMINAL_MAX_SCROLLBACK_SETTING`.
pub const TERMINAL_MAX_SCROLLBACK_SETTING: NotYetWiredSetting = NotYetWiredSetting {
    id: "terminal-max-scrollback",
    label: "Terminal max scrollback",
    fixed_value: "5000 lines",
    note: "Not yet wired - fixed at 5000 lines",
};

/// Terminal output-logging policy — port of `TERMINAL_OUTPUT_LOGGING_SETTING`.
pub const TERMINAL_OUTPUT_LOGGING_SETTING: NotYetWiredSetting = NotYetWiredSetting {
    id: "terminal-output-logging",
    label: "Terminal output logging policy",
    fixed_value: "Capture to Flight Recorder (redacted)",
    note: "Not yet wired - backend redacts + records captured output",
};

/// Model-session default provider. MT-101 keeps provider per-launch; durable defaults are not wired yet.
pub const MODEL_SESSION_DEFAULT_PROVIDER_SETTING: NotYetWiredSetting = NotYetWiredSetting {
    id: "model-session-default-provider",
    label: "Model-session default provider",
    fixed_value: "Per launch",
    note: "Not yet wired - choose Local or Cloud in the launch dialog",
};

/// Model-session default wrapper. The MT-101 dialog seeds this value but does not persist it yet.
pub const MODEL_SESSION_DEFAULT_WRAPPER_SETTING: NotYetWiredSetting = NotYetWiredSetting {
    id: "model-session-default-wrapper",
    label: "Model-session default wrapper",
    fixed_value: "repo-folder-wrapper-v1",
    note: "Not yet wired - edit the wrapper in the launch dialog",
};

/// Model-session local model root. Native launch records model_id/path in /jobs but does not own model
/// inventory discovery yet.
pub const MODEL_SESSION_LOCAL_MODEL_ROOT_SETTING: NotYetWiredSetting = NotYetWiredSetting {
    id: "model-session-local-model-root",
    label: "Model-session local model root",
    fixed_value: "Configured outside native settings",
    note: "Not yet wired - provide the model id/path in the launch dialog",
};

/// The app name shown in the About section. Port of `ABOUT_INFO.appName`.
pub const ABOUT_APP_NAME: &str = "Handshake";

/// The app version shown in the About section. UPGRADED from the React `ABOUT_INFO.version` ("n/a"):
/// the native shell reports the real Cargo package version (compile-time `CARGO_PKG_VERSION`), not a
/// placeholder, per the MT contract (AC11).
pub const ABOUT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ===========================================================================
// Async transport seam (MT-009 pattern): a SYNCHRONOUS trait for unit-testing,
// with a reqwest implementation that bridges async onto the app's tokio runtime.
// ===========================================================================

/// The async delivery cell a spawned settings-LOAD task writes into (MT-018): `Ok(Some(blob))` /
/// `Ok(None)` (first run) / `Err(message)`. Drained (try_lock) on the egui frame thread. A type alias so
/// the field type stays legible (clippy type_complexity).
pub type SettingsLoadCell = Arc<Mutex<Option<Result<Option<Value>, String>>>>;

/// The async delivery cell a spawned settings-SAVE task writes into (MT-018): `Ok(())` / `Err(message)`.
pub type SettingsSaveCell = Arc<Mutex<Option<Result<(), String>>>>;

/// A transient settings-transport failure (network down, non-success status, parse error). Surfaced on
/// the dialog's status row so a save/load failure degrades visibly without a crash. Distinct from
/// "no settings stored yet" (which is `Ok(None)` on load).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTransportError(pub String);

impl std::fmt::Display for SettingsTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The synchronous settings transport seam the app's background tasks drive. Synchronous so the
/// load/save bridge stays a pure, directly-unit-testable seam with a stub (no live server). The
/// production [`SettingsClient`] bridges this onto reqwest + the app's tokio runtime (HBR-QUIET: the
/// egui thread never calls these directly — the app spawns them off-thread).
pub trait SettingsTransport: Send + Sync {
    /// `GET /workspaces/{workspace_id}/settings` → the stored `settings_state` blob, or `None` when no
    /// settings have been saved yet (first run).
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, SettingsTransportError>;

    /// `PUT /workspaces/{workspace_id}/settings` with `{ settings_state }` → the persisted blob.
    fn save(&self, workspace_id: &str, settings_state: Value)
        -> Result<(), SettingsTransportError>;
}

/// Production transport: the backend's PostgreSQL-authoritative workspace-settings REST surface
/// (`GET`/`PUT /workspaces/:workspace_id/settings`), bridged onto the app's tokio runtime handle (the
/// MT-009 `WorkbenchLayoutClient` pattern). reqwest is async; this holds a runtime [`Handle`] and
/// bridges with `Handle::block_on` so the transport stays a synchronous seam, and the app calls it
/// ONLY from a short-lived tokio task off the egui UI thread.
///
/// [`Handle`]: tokio::runtime::Handle
#[derive(Clone)]
pub struct SettingsClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl SettingsClient {
    /// Build a client against `base_url` (e.g. [`crate::backend_client::BACKEND_BASE_URL`]) bridging
    /// onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL, runtime)
    }

    fn settings_url(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/settings",
            self.base_url,
            urlencode(workspace_id)
        )
    }
}

/// Minimal percent-encoding for a path segment (defensive: a stray space/slash cannot break the URL).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl SettingsTransport for SettingsClient {
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, SettingsTransportError> {
        let url = self.settings_url(workspace_id);
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .get(&url)
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| SettingsTransportError(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(SettingsTransportError(format!(
                    "GET settings non-success status {}",
                    resp.status()
                )));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|e| SettingsTransportError(e.to_string()))?;
            // WorkspaceSettingsResponse.settings_state is Option<Value>; null/absent => first run.
            match body.get("settings_state") {
                Some(Value::Null) | None => Ok(None),
                Some(v) => Ok(Some(v.clone())),
            }
        })
    }

    fn save(
        &self,
        workspace_id: &str,
        settings_state: Value,
    ) -> Result<(), SettingsTransportError> {
        let url = self.settings_url(workspace_id);
        let client = self.client.clone();
        let request_body = serde_json::json!({ "settings_state": settings_state });
        self.runtime.block_on(async move {
            let resp = client
                .put(&url)
                .timeout(REQUEST_TIMEOUT)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| SettingsTransportError(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(SettingsTransportError(format!(
                    "PUT settings non-success status {}",
                    resp.status()
                )));
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1 (contract): normalize_chord_input canonicalizes modifier names + order.
    #[test]
    fn normalize_chord_input_canonicalizes_modifiers_and_order() {
        assert_eq!(normalize_chord_input("Ctrl-Shift-P"), "Mod-Shift-p");
        assert_eq!(normalize_chord_input("Cmd-p"), "Mod-p");
        // Modifier order is normalized to Mod-Alt-Shift regardless of input order.
        assert_eq!(normalize_chord_input("Shift-Alt-Mod-k"), "Mod-Alt-Shift-k");
        // Command/Control/Meta all collapse to Mod; Option collapses to Alt.
        assert_eq!(normalize_chord_input("Command-Option-x"), "Mod-Alt-x");
        // A multi-char key (e.g. a named key) is NOT lowercased.
        assert_eq!(normalize_chord_input("Mod-Enter"), "Mod-Enter");
        // Blank / empty -> "".
        assert_eq!(normalize_chord_input(""), "");
        assert_eq!(normalize_chord_input("  -  "), "");
    }

    // Test 2 (contract): find_keybinding_conflicts returns a conflict when two actions share a chord.
    #[test]
    fn find_keybinding_conflicts_detects_shared_chord() {
        let mut settings = default_workspace_settings_state();
        // Defaults are distinct (Mod-p vs Mod-Shift-p) -> no conflict.
        assert!(find_keybinding_conflicts(&settings).is_empty());

        // Point both actions at the same chord -> one conflict naming both labels.
        settings.set_chord("app.quick_switcher.open", "Mod-Alt-p".to_owned());
        settings.set_chord("app.command_palette.open", "Mod-Alt-p".to_owned());
        let conflicts = find_keybinding_conflicts(&settings);
        assert_eq!(conflicts.len(), 1, "exactly one shared-chord conflict");
        assert_eq!(conflicts[0].chord, "Mod-Alt-p");
        assert_eq!(
            conflicts[0].action_labels,
            vec!["Quick Switcher".to_owned(), "Command Palette".to_owned()],
        );

        // The conflict label format ("A and B").
        assert_eq!(
            keybinding_label_for_conflict(&conflicts[0].action_labels),
            "Quick Switcher and Command Palette",
        );
        // Three-label format ("A, B and C").
        assert_eq!(
            keybinding_label_for_conflict(&["A".to_owned(), "B".to_owned(), "C".to_owned()]),
            "A, B and C",
        );
    }

    // Test 3 (contract): setting_matches_query matches the joined term list.
    #[test]
    fn setting_matches_query_matches_terms() {
        // Empty query matches everything.
        assert!(setting_matches_query("", &["anything"]));
        // Substring of a joined term matches.
        assert!(setting_matches_query(
            "theme",
            &["appearance", "theme", "dark"]
        ));
        assert!(setting_matches_query(
            "dar",
            &["appearance", "theme", "dark"]
        ));
        // A term not present does not match.
        assert!(!setting_matches_query(
            "terminal",
            &["appearance", "theme", "dark"]
        ));
        // Case-insensitive (caller lowercases the query; terms are lowercased here).
        assert!(setting_matches_query("view", &["View", "Mode", "NSFW"]));
    }

    // Red-team MC6: an empty / foreign backend response falls back to defaults (never empty chords).
    #[test]
    fn normalize_falls_back_on_empty_or_foreign_blob() {
        let fallback = default_workspace_settings_state();

        // Empty object (no schema_id) -> whole fallback.
        let got = normalize_workspace_settings_state(&serde_json::json!({}), &fallback);
        assert_eq!(got, fallback, "empty object falls back wholesale");

        // Non-object -> whole fallback.
        let got = normalize_workspace_settings_state(&serde_json::json!("nope"), &fallback);
        assert_eq!(got, fallback, "non-object falls back wholesale");

        // Foreign schema_id -> whole fallback.
        let got = normalize_workspace_settings_state(
            &serde_json::json!({ "schema_id": "something.else@1", "theme": "light" }),
            &fallback,
        );
        assert_eq!(got, fallback, "foreign schema_id falls back wholesale");

        // Right schema, but missing keybindings -> defaults kept (NOT empty chords).
        let got = normalize_workspace_settings_state(
            &serde_json::json!({ "schema_id": WORKSPACE_SETTINGS_SCHEMA_ID, "theme": "light" }),
            &fallback,
        );
        assert_eq!(got.theme, WorkspaceTheme::Light, "valid theme parsed");
        assert_eq!(
            got.chord_for("app.quick_switcher.open"),
            Some("Mod-p"),
            "missing keybinding falls back to default, never empty"
        );
        assert_eq!(
            got.chord_for("app.command_palette.open"),
            Some("Mod-Shift-p"),
        );
    }

    #[test]
    fn round_trips_through_settings_state_json() {
        let mut settings = default_workspace_settings_state();
        settings.theme = WorkspaceTheme::Light;
        settings.view_mode = SettingsViewMode::Sfw;
        settings.swarm_board_default_open = true;
        settings.set_chord("app.quick_switcher.open", "Mod-Alt-q".to_owned());

        let json = settings.to_settings_state();
        let back = normalize_workspace_settings_state(&json, &default_workspace_settings_state());
        assert_eq!(
            back, settings,
            "settings round-trip through the backend JSON shape"
        );
    }

    #[test]
    fn theme_maps_to_and_from_hs_theme() {
        assert_eq!(WorkspaceTheme::Dark.to_hs_theme(), HsTheme::Dark);
        assert_eq!(WorkspaceTheme::Light.to_hs_theme(), HsTheme::Light);
        assert_eq!(
            WorkspaceTheme::from_hs_theme(HsTheme::Dark),
            WorkspaceTheme::Dark
        );
        assert_eq!(
            WorkspaceTheme::from_hs_theme(HsTheme::Light),
            WorkspaceTheme::Light
        );
    }

    #[test]
    fn about_version_is_real_cargo_version_not_placeholder() {
        // AC11: the About version is the real package version, never the React "n/a" placeholder.
        assert_ne!(ABOUT_VERSION, "n/a");
        assert_eq!(ABOUT_VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(ABOUT_APP_NAME, "Handshake");
    }

    // ── MT-072 editor settings serde round-trip + legacy compat (AC-001/002/006/009) ─────────────────

    /// AC-001 (shape side): editor prefs + syntax palette + editor keybindings round-trip through the
    /// SAME `to_settings_state` / `normalize_workspace_settings_state` path the existing PUT/GET uses.
    #[test]
    fn editor_settings_round_trip_through_settings_state_json() {
        let mut settings = default_workspace_settings_state();
        settings.editor_prefs = EditorPrefs {
            editor_font_size: 17.5,
            tab_size: 8,
            insert_spaces: false,
            word_wrap: WordWrapMode::BoundedColumn(100),
            render_whitespace: RenderWhitespaceMode::All,
        };
        settings.syntax_palette = SyntaxPalette {
            mode: SyntaxPaletteMode::Custom,
            custom: std::collections::HashMap::from([("keyword".to_owned(), [10, 20, 30, 255])]),
        };
        settings.set_editor_chord("code.open_find", "Mod+Alt+F".to_owned());
        settings.set_editor_chord("rich.toggle_bold", "Mod+Shift+B".to_owned());

        let json = settings.to_settings_state();
        let back = normalize_workspace_settings_state(&json, &default_workspace_settings_state());
        assert_eq!(
            back, settings,
            "editor settings round-trip through the backend JSON shape"
        );

        // The new fields are NEW TOP-LEVEL keys (NOT inside the backend-validated `keybindings` map).
        let obj = json.as_object().unwrap();
        assert!(
            obj.contains_key("editor_prefs"),
            "editor_prefs is a top-level key"
        );
        assert!(
            obj.contains_key("syntax_palette"),
            "syntax_palette is a top-level key"
        );
        assert!(
            obj.contains_key("editor_keybindings"),
            "editor_keybindings is a top-level key"
        );
        // RISK-001 guard: the editor bindings did NOT leak into the backend-validated `keybindings` map.
        let kb = obj.get("keybindings").and_then(Value::as_object).unwrap();
        assert!(
            kb.keys().all(|k| k == "app.quick_switcher.open" || k == "app.command_palette.open"),
            "the WP-011 keybindings map keeps ONLY the two app actions the backend allows (RISK-001); \
             editor bindings live in the separate editor_keybindings list, got {:?}",
            kb.keys().collect::<Vec<_>>()
        );
    }

    /// AC-002: `editor_font_size` is a SEPARATE field from the chrome/UI appearance — it lives ONLY under
    /// `editor_prefs`, and mutating it does not touch the theme (the chrome appearance field). The
    /// serialized payload proves the two are distinct keys.
    #[test]
    fn editor_font_size_is_separate_from_chrome_appearance() {
        let mut settings = default_workspace_settings_state();
        let theme_before = settings.theme;
        settings.editor_prefs.editor_font_size = 28.0;

        // Mutating editor font size left the chrome theme untouched.
        assert_eq!(
            settings.theme, theme_before,
            "editor font size change must not alter chrome theme"
        );

        let json = settings.to_settings_state();
        let obj = json.as_object().unwrap();
        // editor_font_size is under editor_prefs, NOT a top-level theme/appearance key.
        let editor_prefs = obj.get("editor_prefs").and_then(Value::as_object).unwrap();
        assert!(editor_prefs.contains_key("editor_font_size"));
        // The top-level appearance key (`theme`) carries NO font size — they are distinct surfaces.
        assert!(
            !obj.contains_key("editor_font_size"),
            "editor font size is NOT a top-level chrome key"
        );
        assert!(
            obj.contains_key("theme"),
            "chrome appearance (theme) is its own top-level key"
        );
    }

    /// AC-006: a legacy WP-011-era settings document WITHOUT any of the new keys deserializes cleanly to
    /// the defaults (the manual-normalize analogue of #[serde(default)] — absent => fallback per field).
    #[test]
    fn legacy_settings_doc_without_editor_fields_deserializes_to_defaults() {
        // A WP-011-era blob: valid schema_id + theme + keybindings + settings, but NO editor_prefs /
        // syntax_palette / editor_keybindings keys at all (exactly what a pre-MT-072 PostgreSQL row holds).
        let legacy = serde_json::json!({
            "schema_id": WORKSPACE_SETTINGS_SCHEMA_ID,
            "theme": "dark",
            "custom_theme_tokens": {},
            "keybindings": {
                "app.quick_switcher.open": "Mod-p",
                "app.command_palette.open": "Mod-Shift-p",
            },
            "settings": { "view_mode": "NSFW", "swarm_board_default_open": false },
        });
        let fallback = default_workspace_settings_state();
        let got = normalize_workspace_settings_state(&legacy, &fallback);

        // It deserialized without error and the new fields are the DEFAULTS (no panic, no missing key).
        assert_eq!(
            got.editor_prefs,
            EditorPrefs::default(),
            "legacy doc -> default editor prefs"
        );
        assert_eq!(
            got.syntax_palette,
            SyntaxPalette::default(),
            "legacy doc -> default syntax palette"
        );
        assert!(
            got.editor_keybindings.is_empty(),
            "legacy doc -> no editor keybinding overrides"
        );
        // And the existing fields still parsed correctly.
        assert_eq!(got.theme, WorkspaceTheme::Dark);
        assert_eq!(got.chord_for("app.quick_switcher.open"), Some("Mod-p"));
    }

    /// A stored value out of the clamp ranges is clamped on load (a hand-edited PostgreSQL row cannot
    /// smuggle an invalid font size / tab size into the live editor).
    #[test]
    fn out_of_range_stored_editor_prefs_are_clamped_on_load() {
        let blob = serde_json::json!({
            "editor_font_size": 999.0,
            "tab_size": 99,
            "insert_spaces": true,
            "word_wrap": "off",
            "render_whitespace": "none",
        });
        let got = EditorPrefs::from_json(Some(&blob), &EditorPrefs::default());
        assert_eq!(
            got.editor_font_size,
            *EDITOR_FONT_SIZE_RANGE.end(),
            "font size clamped to max"
        );
        assert_eq!(
            got.tab_size,
            *TAB_SIZE_RANGE.end(),
            "tab size clamped to max"
        );
    }

    /// `WordWrapMode::BoundedColumn` round-trips its column through JSON.
    #[test]
    fn word_wrap_bounded_column_round_trips() {
        for mode in [
            WordWrapMode::Off,
            WordWrapMode::On,
            WordWrapMode::BoundedColumn(72),
        ] {
            let j = mode.to_json();
            assert_eq!(
                WordWrapMode::from_json(&j),
                Some(mode),
                "wrap mode {mode:?} round-trips"
            );
        }
    }

    /// The editor keybinding override is custom-if-present-else-default (AC-005 resolution).
    #[test]
    fn editor_chord_override_is_custom_if_present_else_none() {
        let mut settings = default_workspace_settings_state();
        assert_eq!(
            settings.editor_chord_override("code.open_find"),
            None,
            "no override => None (default)"
        );
        settings.set_editor_chord("code.open_find", "Mod+Alt+F".to_owned());
        assert_eq!(
            settings.editor_chord_override("code.open_find"),
            Some("Mod+Alt+F")
        );
        assert!(
            settings.clear_editor_chord("code.open_find"),
            "clear removes the override"
        );
        assert_eq!(
            settings.editor_chord_override("code.open_find"),
            None,
            "cleared => back to default"
        );
    }
}
