//! Operator-configurable keybinding overrides for the code editor (WP-KERNEL-012 — E1 MT-010).
//!
//! VS Code stores per-user keybindings in `keybindings.json`. The native editor mirrors that with a
//! portable, human-editable JSON file at `~/.handshake/keymap.json` (resolved via [`dirs::home_dir`],
//! NEVER a hardcoded path — GLOBAL-PORTABILITY-004 / AC-007). The file is the SAME shell settings home
//! the MT contract names; it carries ONLY override entries (action id + chord string), and the
//! resolver ([`crate::code_editor::keymap::Keymap::from_settings`]) layers them over the VS Code
//! defaults so an unspecified action keeps its default.
//!
//! ## Persistence-authority reconciliation (KERNEL_BUILDER gate)
//!
//! The contract flags two candidate homes: this portable file AND the existing PostgreSQL-backed
//! `/workspaces/:id/settings` surface MT-072 uses. To avoid a SECOND settings authority while MT-072 is
//! not yet landed, this module keeps a SINGLE local-file authority that is intentionally narrow (only
//! keybinding overrides) and portable. When MT-072 lands, the editor-keybindings settings section it
//! adds should read/write THIS same override list (it is plain serde JSON), so PG stays the durable
//! authority and this file is the VS-Code-parity local mirror — one logical authority, two transports.
//! NO SQLite is introduced; this is a flat JSON file plus the existing PG settings surface (future).
//!
//! ## No silent wrong binding (RISK-003 / MC-002)
//!
//! [`chord_from_str`] returns `Err` for an unknown key name or malformed chord; [`Keymap::from_settings`]
//! skips an unparseable override with a warning rather than binding the wrong chord.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::keymap::KeyChord;

/// The fixed sub-path under the user home for the editor keybinding overrides. Combined with
/// [`dirs::home_dir`] in [`keymap_settings_path`] — no hardcoded absolute path (AC-007 /
/// GLOBAL-PORTABILITY-004).
const KEYMAP_RELATIVE_PATH: &[&str] = &[".handshake", "keymap.json"];

/// One operator keybinding override: rebind `action` (a [`super::keymap::CodeEditorAction::name`]
/// snake_case id, e.g. `"open_find"`) to `chord` (a human-readable chord string parsed by
/// [`KeymapSettings::chord_from_str`], e.g. `"Ctrl+Shift+P"`). serde-derived so the file is plain JSON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeymapOverride {
    /// The action id to rebind (snake_case, see `CodeEditorAction::name`).
    pub action: String,
    /// The chord to bind it to (e.g. `"Ctrl+Shift+P"`, `"Mod+F"`, `"Alt+Up"`).
    pub chord: String,
}

/// The full keymap settings: just the override list. The default (no file / empty file) is no
/// overrides — the resolver then yields the pure VS Code defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeymapSettings {
    /// Operator overrides, layered over the defaults in declaration order (later wins).
    #[serde(default)]
    pub overrides: Vec<KeymapOverride>,
}

/// Errors loading/saving/parsing keymap settings. Explicit variants so a caller can distinguish a
/// missing file (benign — use defaults) from a malformed one (surface to the operator).
#[derive(Debug)]
pub enum KeymapSettingsError {
    /// The home directory could not be resolved (no `$HOME` / `%USERPROFILE%`).
    NoHomeDir,
    /// An I/O error reading/writing the file.
    Io(std::io::Error),
    /// The file content was not valid JSON for [`KeymapSettings`].
    Parse(serde_json::Error),
    /// A chord string could not be parsed (unknown key, empty, or malformed).
    BadChord(String),
}

impl std::fmt::Display for KeymapSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeymapSettingsError::NoHomeDir => {
                write!(
                    f,
                    "could not resolve the user home directory for ~/.handshake/keymap.json"
                )
            }
            KeymapSettingsError::Io(e) => write!(f, "keymap.json I/O error: {e}"),
            KeymapSettingsError::Parse(e) => write!(f, "keymap.json parse error: {e}"),
            KeymapSettingsError::BadChord(s) => write!(f, "unparseable key chord: {s:?}"),
        }
    }
}

impl std::error::Error for KeymapSettingsError {}

/// The portable path to the keymap override file: `{home}/.handshake/keymap.json`, resolved via
/// [`dirs::home_dir`] (AC-007 — NEVER a hardcoded path string). `Err(NoHomeDir)` when the home directory
/// is not resolvable (a headless/sandboxed environment), so the caller falls back to defaults.
pub fn keymap_settings_path() -> Result<PathBuf, KeymapSettingsError> {
    let mut path = dirs::home_dir().ok_or(KeymapSettingsError::NoHomeDir)?;
    for segment in KEYMAP_RELATIVE_PATH {
        path.push(segment);
    }
    Ok(path)
}

impl KeymapSettings {
    /// Load settings from `path` (a JSON file). A MISSING file yields the default (empty) settings —
    /// not an error — so a fresh install with no overrides resolves to the pure VS Code defaults. A
    /// present-but-malformed file is a [`KeymapSettingsError::Parse`] (surface it; do not silently
    /// discard the operator's intent).
    pub fn load_from_file(path: &Path) -> Result<Self, KeymapSettingsError> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(KeymapSettingsError::Parse),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(KeymapSettingsError::Io(e)),
        }
    }

    /// Load from the portable `~/.handshake/keymap.json` path. A missing file OR an unresolvable home
    /// directory both yield the default (empty) settings, so the editor always has a working keymap.
    pub fn load_default() -> Self {
        match keymap_settings_path() {
            Ok(path) => Self::load_from_file(&path).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "keymap.json load failed; using VS Code defaults");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Write `settings` to `path` as pretty JSON, creating the parent directory if needed (so a first
    /// save of `~/.handshake/keymap.json` creates `~/.handshake/`). The shell's "Configure keybindings"
    /// action calls this to materialize an editable file the operator then edits.
    pub fn save_to_file(path: &Path, settings: &Self) -> Result<(), KeymapSettingsError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(KeymapSettingsError::Io)?;
        }
        let json = serde_json::to_string_pretty(settings).map_err(KeymapSettingsError::Parse)?;
        std::fs::write(path, json).map_err(KeymapSettingsError::Io)
    }

    /// Parse a human-readable chord string into a [`KeyChord`] (implementation note 4). Splits on `+`,
    /// maps each part to a modifier flag or a key via [`key_from_str`]. `Mod` maps to Ctrl on
    /// Windows/Linux and Command on macOS. Modifiers may appear in any order; exactly one non-modifier
    /// token (the key) is required. Returns [`KeymapSettingsError::BadChord`] for an empty string, a
    /// missing key, multiple keys, or an unknown key name (RISK-003 — no silent wrong chord).
    ///
    /// Examples: `"Ctrl+Shift+P"`, `"Mod+F"`, `"Alt+Up"`, `"F12"`, `"Shift+F12"`, `"Escape"`.
    pub fn chord_from_str(s: &str) -> Result<KeyChord, KeymapSettingsError> {
        let bad = || KeymapSettingsError::BadChord(s.to_owned());
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(bad());
        }
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut mac_cmd = false;
        let mut key: Option<egui::Key> = None;
        for raw in trimmed.split('+') {
            let part = raw.trim();
            if part.is_empty() {
                return Err(bad()); // a trailing/leading/double `+` is malformed.
            }
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" | "option" => alt = true,
                "shift" => shift = true,
                "cmd" | "command" | "meta" | "super" | "win" => mac_cmd = true,
                "mod" => {
                    // Mod = Ctrl on Windows/Linux, Command on macOS (implementation note 4).
                    if super::keymap::mod_is_ctrl() {
                        ctrl = true;
                    } else {
                        mac_cmd = true;
                    }
                }
                _ => {
                    // A non-modifier token is the key. Exactly one is allowed.
                    if key.is_some() {
                        return Err(bad()); // two keys in one chord is malformed.
                    }
                    key = Some(key_from_str(part).ok_or_else(bad)?);
                }
            }
        }
        let key = key.ok_or_else(bad)?; // a chord with only modifiers is malformed.
        Ok(KeyChord {
            key,
            ctrl,
            alt,
            shift,
            mac_cmd,
        })
    }

    /// Format a [`KeyChord`] back to the canonical `"Ctrl+Shift+P"` string form (for writing an
    /// override file or a UI hint). Mirrors [`chord_from_str`] so a round trip is stable.
    pub fn chord_to_str(chord: &KeyChord) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if chord.ctrl {
            parts.push("Ctrl");
        }
        if chord.mac_cmd {
            parts.push("Cmd");
        }
        if chord.alt {
            parts.push("Alt");
        }
        if chord.shift {
            parts.push("Shift");
        }
        let key_name = key_to_str(chord.key);
        let mut out = parts.join("+");
        if out.is_empty() {
            key_name.to_owned()
        } else {
            out.push('+');
            out.push_str(key_name);
            out
        }
    }
}

/// Map a key NAME (the textual token in a chord string) to an [`egui::Key`]. Covers every key in the
/// default binding table plus common extras (F1-F12, arrows, page up/down, home, end, escape, tab,
/// enter, backspace, delete, digits, letters, and the punctuation the table uses). Returns `None` for
/// an unknown name so `chord_from_str` can return `Err` rather than guessing (RISK-003 / MC-002). Does
/// NOT rely on `egui::Key`'s own string names (implementation note 2 — an explicit lookup table).
pub fn key_from_str(name: &str) -> Option<egui::Key> {
    use egui::Key;
    let lower = name.to_ascii_lowercase();
    let key = match lower.as_str() {
        // Letters.
        "a" => Key::A,
        "b" => Key::B,
        "c" => Key::C,
        "d" => Key::D,
        "e" => Key::E,
        "f" => Key::F,
        "g" => Key::G,
        "h" => Key::H,
        "i" => Key::I,
        "j" => Key::J,
        "k" => Key::K,
        "l" => Key::L,
        "m" => Key::M,
        "n" => Key::N,
        "o" => Key::O,
        "p" => Key::P,
        "q" => Key::Q,
        "r" => Key::R,
        "s" => Key::S,
        "t" => Key::T,
        "u" => Key::U,
        "v" => Key::V,
        "w" => Key::W,
        "x" => Key::X,
        "y" => Key::Y,
        "z" => Key::Z,
        // Digits (top row).
        "0" | "num0" => Key::Num0,
        "1" | "num1" => Key::Num1,
        "2" | "num2" => Key::Num2,
        "3" | "num3" => Key::Num3,
        "4" | "num4" => Key::Num4,
        "5" | "num5" => Key::Num5,
        "6" | "num6" => Key::Num6,
        "7" | "num7" => Key::Num7,
        "8" | "num8" => Key::Num8,
        "9" | "num9" => Key::Num9,
        // Function keys.
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        // Arrows.
        "up" | "arrowup" => Key::ArrowUp,
        "down" | "arrowdown" => Key::ArrowDown,
        "left" | "arrowleft" => Key::ArrowLeft,
        "right" | "arrowright" => Key::ArrowRight,
        // Navigation / editing.
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" | "pgup" => Key::PageUp,
        "pagedown" | "pgdown" | "pgdn" => Key::PageDown,
        "escape" | "esc" => Key::Escape,
        "tab" => Key::Tab,
        "enter" | "return" => Key::Enter,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "space" | "spacebar" => Key::Space,
        "insert" | "ins" => Key::Insert,
        // Punctuation used by the binding table.
        "[" | "openbracket" | "leftbracket" => Key::OpenBracket,
        "]" | "closebracket" | "rightbracket" => Key::CloseBracket,
        "/" | "slash" => Key::Slash,
        "\\" | "backslash" => Key::Backslash,
        "," | "comma" => Key::Comma,
        "." | "period" | "dot" => Key::Period,
        ";" | "semicolon" => Key::Semicolon,
        "'" | "quote" => Key::Quote,
        "-" | "minus" => Key::Minus,
        "=" | "equals" | "plus" => Key::Equals,
        "`" | "backtick" | "grave" => Key::Backtick,
        _ => return None,
    };
    Some(key)
}

/// Map an [`egui::Key`] back to its canonical chord-string token (the inverse of [`key_from_str`] for
/// the names the table uses). Used by [`KeymapSettings::chord_to_str`].
pub fn key_to_str(key: egui::Key) -> &'static str {
    use egui::Key;
    match key {
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Escape => "Escape",
        Key::Tab => "Tab",
        Key::Enter => "Enter",
        Key::Backspace => "Backspace",
        Key::Delete => "Delete",
        Key::Space => "Space",
        Key::Insert => "Insert",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::Slash => "/",
        Key::Backslash => "\\",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Semicolon => ";",
        Key::Quote => "'",
        Key::Minus => "-",
        Key::Equals => "=",
        Key::Backtick => "`",
        // Any key not in the table renders as a debug name (still a stable, unique token).
        other => key_fallback_name(other),
    }
}

/// A stable fallback token for an `egui::Key` not in the explicit table (defensive — the table covers
/// every key the bindings + extras use, so this is only reached if egui adds keys). Uses a small static
/// set of leaked-free `&'static str`s for the handful of remaining keys.
fn key_fallback_name(key: egui::Key) -> &'static str {
    // egui::Key is non-exhaustive going forward; map the few remaining symbolic keys we know, else a
    // generic token. This branch is not reachable from the current binding table.
    match key {
        egui::Key::Plus => "Plus",
        egui::Key::Colon => "Colon",
        egui::Key::Questionmark => "?",
        egui::Key::Pipe => "|",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Key;

    #[test]
    fn chord_from_str_parses_ctrl_shift_p() {
        let chord = KeymapSettings::chord_from_str("Ctrl+Shift+P").expect("valid chord");
        assert_eq!(chord.key, Key::P);
        assert!(chord.ctrl && chord.shift);
        assert!(!chord.alt && !chord.mac_cmd);
    }

    #[test]
    fn chord_from_str_parses_alt_up() {
        let chord = KeymapSettings::chord_from_str("Alt+Up").expect("valid chord");
        assert_eq!(chord.key, Key::ArrowUp);
        assert!(chord.alt && !chord.ctrl && !chord.shift);
    }

    #[test]
    fn chord_from_str_invalid_returns_err() {
        // Unknown key name.
        assert!(KeymapSettings::chord_from_str("Ctrl+Nope").is_err());
        // Only modifiers, no key.
        assert!(KeymapSettings::chord_from_str("Ctrl+Shift").is_err());
        // Empty.
        assert!(KeymapSettings::chord_from_str("").is_err());
        // Malformed (double plus).
        assert!(KeymapSettings::chord_from_str("Ctrl++F").is_err());
        // Two keys.
        assert!(KeymapSettings::chord_from_str("F+G").is_err());
    }

    #[test]
    fn chord_round_trips() {
        for s in ["Ctrl+Shift+P", "Alt+Up", "F12", "Escape", "Ctrl+/"] {
            let chord = KeymapSettings::chord_from_str(s).expect("valid");
            let back = KeymapSettings::chord_to_str(&chord);
            let reparsed = KeymapSettings::chord_from_str(&back).expect("round-trip valid");
            assert_eq!(chord, reparsed, "round trip for {s:?} (-> {back:?})");
        }
    }

    #[test]
    fn load_missing_file_is_default_not_error() {
        let path = Path::new("definitely-does-not-exist-keymap-test.json");
        let settings = KeymapSettings::load_from_file(path).expect("missing file is default");
        assert!(settings.overrides.is_empty());
    }
}
