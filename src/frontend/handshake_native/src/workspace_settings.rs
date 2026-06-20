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

/// The full persisted workspace settings state. Port of the React `WorkspaceSettingsState`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSettingsState {
    /// Workspace-scoped shell theme (wired: drives `HsTheme`).
    pub theme: WorkspaceTheme,
    /// Per-action keybinding chords, in `APP_KEYBINDING_ACTIONS` order.
    pub keybindings: Vec<Keybinding>,
    /// Content-presentation mode (wired).
    pub view_mode: SettingsViewMode,
    /// Whether the Swarm Board opens on launch (wired).
    pub swarm_board_default_open: bool,
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
        if let Some(existing) = self.keybindings.iter_mut().find(|b| b.action_id == action_id) {
            existing.chord = chord;
        } else {
            self.keybindings.push(Keybinding {
                action_id: action_id.to_owned(),
                chord,
            });
        }
    }

    /// Serialize to the backend `settings_state` JSON shape (React `WorkspaceSettingsState` JSON).
    /// `keybindings` is emitted as a JSON object keyed by action id (React parity), `settings` is the
    /// nested object the React schema uses for view_mode + swarm_board_default_open.
    pub fn to_settings_state(&self) -> Value {
        let mut keybindings = serde_json::Map::new();
        for binding in &self.keybindings {
            keybindings.insert(binding.action_id.clone(), Value::String(binding.chord.clone()));
        }
        serde_json::json!({
            "schema_id": WORKSPACE_SETTINGS_SCHEMA_ID,
            "theme": self.theme.as_str(),
            "custom_theme_tokens": {},
            "keybindings": Value::Object(keybindings),
            "settings": {
                "view_mode": self.view_mode.as_str(),
                "swarm_board_default_open": self.swarm_board_default_open,
            },
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

    WorkspaceSettingsState {
        theme,
        keybindings,
        view_mode,
        swarm_board_default_open,
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
    fn save(&self, workspace_id: &str, settings_state: Value) -> Result<(), SettingsTransportError>;
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
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
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

    fn save(&self, workspace_id: &str, settings_state: Value) -> Result<(), SettingsTransportError> {
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
            keybinding_label_for_conflict(&[
                "A".to_owned(),
                "B".to_owned(),
                "C".to_owned()
            ]),
            "A, B and C",
        );
    }

    // Test 3 (contract): setting_matches_query matches the joined term list.
    #[test]
    fn setting_matches_query_matches_terms() {
        // Empty query matches everything.
        assert!(setting_matches_query("", &["anything"]));
        // Substring of a joined term matches.
        assert!(setting_matches_query("theme", &["appearance", "theme", "dark"]));
        assert!(setting_matches_query("dar", &["appearance", "theme", "dark"]));
        // A term not present does not match.
        assert!(!setting_matches_query("terminal", &["appearance", "theme", "dark"]));
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
        assert_eq!(back, settings, "settings round-trip through the backend JSON shape");
    }

    #[test]
    fn theme_maps_to_and_from_hs_theme() {
        assert_eq!(WorkspaceTheme::Dark.to_hs_theme(), HsTheme::Dark);
        assert_eq!(WorkspaceTheme::Light.to_hs_theme(), HsTheme::Light);
        assert_eq!(WorkspaceTheme::from_hs_theme(HsTheme::Dark), WorkspaceTheme::Dark);
        assert_eq!(WorkspaceTheme::from_hs_theme(HsTheme::Light), WorkspaceTheme::Light);
    }

    #[test]
    fn about_version_is_real_cargo_version_not_placeholder() {
        // AC11: the About version is the real package version, never the React "n/a" placeholder.
        assert_ne!(ABOUT_VERSION, "n/a");
        assert_eq!(ABOUT_VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(ABOUT_APP_NAME, "Handshake");
    }
}
