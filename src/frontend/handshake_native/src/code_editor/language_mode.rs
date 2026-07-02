//! Language-mode detection + per-document override model (WP-KERNEL-012 MT-071, E11).
//!
//! ## What this is
//!
//! MT-001 ([`super::highlight`]) resolves a document's language from its FILE EXTENSION only
//! ([`super::highlight::language_id_for_extension`]). That is correct for `foo.rs` -> `rust`, but it
//! cannot tell that an extension-less `#!/usr/bin/env python` script is Python, and it gives the user
//! no way to OVERRIDE a wrong guess. This module is the VS-Code-parity upgrade the status-bar
//! "language mode" segment (MT-071) drives:
//!
//! - [`detect_language`] resolves the language with a STRICT precedence —
//!   `UserOverride > Shebang > Content > Extension` (the MT-071 contract order, RISK-003/MC-003) —
//!   layering shebang + a small content heuristic ABOVE the MT-001 extension fallback, NEVER letting
//!   the extension beat an explicit override or shebang.
//! - [`available_languages`] returns the picker list, sourced from the SAME MT-001
//!   [`LanguageRegistry`](super::highlight::LanguageRegistry) bundled grammar set so the picker set
//!   matches the highlighter's supported languages exactly (AC-001).
//! - [`LanguageId`] is a thin owned newtype over the registry's stable family-id string (`"rust"` /
//!   `"javascript"`), so the override the user picks is the SAME id space the highlighter +
//!   [`super::highlight::language_id_for_extension`] speak.
//!
//! ## Where the override LIVES (RISK-004/MC-004)
//!
//! The per-document override is NOT stored here. It hangs off the MT-010 document model
//! ([`super::panel::CodeEditorPanel`] — `language_override`), so it survives re-render + re-focus and
//! the MT-001 highlighter reads the resolved language from it. This module is the pure detection +
//! enumeration logic the panel and the status-bar segment call; the panel owns the state slot. That is
//! the "no parallel document store" discipline the contract names.

use super::highlight::{language_id_for_extension, LanguageRegistry};

/// A stable language-family id (the MT-001 registry id space: `"rust"`, `"javascript"`, ...). A thin
/// owned newtype over the `&'static str` family id [`language_id_for_extension`] returns, so the
/// user's picked override is the SAME id the highlighter selects its grammar by (AC-001 — the picker
/// set matches the highlighter). "Plain Text" is represented by the empty string family id, the same
/// convention the highlighter uses for an unmapped extension.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageId(String);

impl LanguageId {
    /// Wrap a family-id string (already in the registry id space).
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The plain-text language (empty family id) — the fallback when no grammar matches.
    pub fn plain_text() -> Self {
        Self(String::new())
    }

    /// The stable family-id string a consumer (the highlighter, the override slot) reads.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True for the empty (plain-text) family id.
    pub fn is_plain_text(&self) -> bool {
        self.0.is_empty()
    }

    /// A human-readable display label for the status-bar segment + picker rows (e.g. `Rust`,
    /// `JavaScript`, `Plain Text`). Derived from the family id so a new registered language gets a
    /// sensible label without a second table to maintain.
    pub fn display_label(&self) -> String {
        match self.0.as_str() {
            "" => "Plain Text".to_owned(),
            "rust" => "Rust".to_owned(),
            "javascript" => "JavaScript".to_owned(),
            "python" => "Python".to_owned(),
            "shell" => "Shell Script".to_owned(),
            "go" => "Go".to_owned(),
            "php" => "PHP".to_owned(),
            "html" => "HTML".to_owned(),
            "json" => "JSON".to_owned(),
            // Title-case the raw family id for any other registered language.
            other => {
                let mut chars = other.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            }
        }
    }
}

/// Where a resolved language came from — the precedence layer that WON. The status-bar segment shows
/// this (e.g. a small "(auto)" vs "(override)" hint) and the AC-001 test asserts which layer resolved
/// a given document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionSource {
    /// The user explicitly picked the language from the status-bar picker (highest precedence).
    UserOverride,
    /// A `#!` shebang on the first line named an interpreter (e.g. `#!/usr/bin/env python`).
    Shebang,
    /// A content heuristic matched (e.g. leading `<?php`, `<!DOCTYPE html`, `package main`).
    Content,
    /// Fell back to the MT-001 file-extension lookup (lowest precedence).
    Extension,
}

/// The result of [`detect_language`]: the resolved family id + the layer that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageDetection {
    pub detected: LanguageId,
    pub source: DetectionSource,
}

/// Process-cached bundled+heuristic picker list. The set is CONSTANT for the life of the process (the
/// bundled grammars + the heuristic families never change at runtime), so it is computed ONCE on first
/// use and cloned thereafter. This is the perf-lens fix (adversarial-review must-fix #4): the status bar
/// renders every frame, and `available_languages()` previously rebuilt a whole
/// [`LanguageRegistry::with_bundled_languages`] (which constructs tree-sitter grammar objects +
/// highlight queries) on EVERY call — a per-frame grammar construction once the cluster is live. The
/// cache reduces that to a one-time build + a cheap `Vec<LanguageId>` clone per frame.
static AVAILABLE_LANGUAGES: std::sync::OnceLock<Vec<LanguageId>> = std::sync::OnceLock::new();

/// The bundled language family ids (the MT-001 registry's `with_bundled_languages` set) plus the
/// heuristic-only families the content/shebang layers can resolve to. Plain Text is always offered.
/// The first element is always plain text so the picker leads with "Plain Text". Sourced from the
/// registry's bundled grammar coverage so the picker set tracks the highlighter (AC-001).
///
/// The constant result is process-cached (see [`AVAILABLE_LANGUAGES`]); each call after the first clones
/// the cached list instead of rebuilding the tree-sitter registry (per-frame status-bar safe).
pub fn available_languages() -> Vec<LanguageId> {
    AVAILABLE_LANGUAGES
        .get_or_init(compute_available_languages)
        .clone()
}

/// Build the bundled+heuristic picker list ONCE (called by the [`AVAILABLE_LANGUAGES`] cache). Anchors
/// the bundled grammar set against the live registry so the two never silently drift: the registry
/// highlights exactly the families `language_id_for_extension` maps, and we enumerate the bundled
/// extensions through it. (A future `register` call adds a grammar; extend the seed list +
/// `language_id_for_extension` together and this picker grows with it.)
fn compute_available_languages() -> Vec<LanguageId> {
    let registry = LanguageRegistry::with_bundled_languages();
    let mut out = vec![LanguageId::plain_text()];
    // Bundled grammar families (verified registered: rust via `.rs`, javascript via `.js`).
    for ext in ["rs", "js"] {
        if registry.highlighter_for_extension(ext).is_some() {
            if let Some(family) = language_id_for_extension(ext) {
                let id = LanguageId::new(family);
                if !out.contains(&id) {
                    out.push(id);
                }
            }
        }
    }
    // Heuristic-only families the shebang/content layers resolve to. They have no bundled grammar yet
    // (so picking them yields plain-text highlighting), but they MUST be selectable so a user can
    // confirm the detected language a shebang/content layer reported (and so the picker label matches
    // what the segment shows). This is the honest "detected but un-highlighted" surface, not a fake.
    for family in ["python", "shell", "go", "php", "html", "json"] {
        let id = LanguageId::new(family);
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Resolve a document's language with the STRICT precedence `UserOverride > Shebang > Content >
/// Extension` (RISK-003/MC-003). The extension fallback REUSES the MT-001
/// [`language_id_for_extension`] so the extension layer agrees with the highlighter exactly.
///
/// - `override_id`: the per-document user override read off the MT-010 model (`Some` wins outright).
/// - `path`: the document path/name, for the extension fallback (`None` for an in-memory buffer).
/// - `first_bytes`: the first bytes of the document, for the shebang sniff (the first line).
/// - `full_text`: the document text, for the content heuristic.
///
/// NEVER lets the extension beat an override or a shebang — the layers are checked top-down and the
/// FIRST that resolves wins (AC-001).
pub fn detect_language(
    override_id: Option<&LanguageId>,
    path: Option<&str>,
    first_bytes: &[u8],
    full_text: &str,
) -> LanguageDetection {
    // 1. User override — highest precedence, ends the search (RISK-003).
    if let Some(id) = override_id {
        return LanguageDetection {
            detected: id.clone(),
            source: DetectionSource::UserOverride,
        };
    }
    // 2. Shebang — `#!` on the first line names an interpreter.
    if let Some(family) = detect_shebang(first_bytes) {
        return LanguageDetection {
            detected: LanguageId::new(family),
            source: DetectionSource::Shebang,
        };
    }
    // 3. Content heuristic — a small leading-marker table.
    if let Some(family) = detect_content(full_text) {
        return LanguageDetection {
            detected: LanguageId::new(family),
            source: DetectionSource::Content,
        };
    }
    // 4. Extension fallback — the MT-001 path (REUSED, not re-derived).
    let family = path
        .and_then(extension_of)
        .and_then(|ext| language_id_for_extension(&ext))
        .unwrap_or("");
    LanguageDetection {
        detected: LanguageId::new(family),
        source: DetectionSource::Extension,
    }
}

/// The lowercased extension of a path (the segment after the last `.`), or `None`. Pure string work;
/// no filesystem access.
fn extension_of(path: &str) -> Option<String> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let dot = name.rfind('.')?;
    // A leading-dot file (`.bashrc`) has no extension before the dot.
    if dot == 0 {
        return None;
    }
    let ext = &name[dot + 1..];
    if ext.is_empty() {
        None
    } else {
        Some(ext.to_ascii_lowercase())
    }
}

/// Sniff a `#!` shebang on the first line and map the interpreter to a family id, or `None` when the
/// document does not start with `#!`. Handles both the direct (`#!/bin/bash`) and `env`
/// (`#!/usr/bin/env python3`) forms. Reads only the first line of `first_bytes`.
fn detect_shebang(first_bytes: &[u8]) -> Option<&'static str> {
    let text = std::str::from_utf8(first_bytes).ok()?;
    let first_line = text.lines().next()?;
    let rest = first_line.strip_prefix("#!")?.trim();
    // The interpreter is the basename of the program (or the arg after `env`).
    let mut tokens = rest.split_whitespace();
    let program = tokens.next()?;
    let basename = program.rsplit(['/', '\\']).next().unwrap_or(program);
    // `env <prog>`: the real interpreter is the next token (strip a leading `-S`/flag if present).
    let interp = if basename == "env" {
        tokens
            .find(|t| !t.starts_with('-'))
            .map(|t| t.rsplit(['/', '\\']).next().unwrap_or(t))
            .unwrap_or(basename)
    } else {
        basename
    };
    interpreter_to_family(interp)
}

/// Map an interpreter basename (possibly version-suffixed like `python3`) to a family id.
fn interpreter_to_family(interp: &str) -> Option<&'static str> {
    let lower = interp.to_ascii_lowercase();
    // Strip a trailing version number (`python3` -> `python`, `node18` -> `node`).
    let base: String = lower
        .trim_end_matches(|c: char| c.is_ascii_digit() || c == '.')
        .to_owned();
    match base.as_str() {
        "python" => Some("python"),
        "bash" | "sh" | "zsh" | "dash" | "ksh" => Some("shell"),
        "node" | "nodejs" => Some("javascript"),
        "php" => Some("php"),
        _ => None,
    }
}

/// A small content heuristic table layered ABOVE the extension fallback (the MT-071 examples:
/// leading `<?php`, `<!DOCTYPE html`, a JSON object/array shape, `package main` -> go). Checks the
/// FIRST non-blank, trimmed content so a leading blank line / BOM does not defeat it. Returns `None`
/// when no marker matches (so the extension layer then decides).
fn detect_content(full_text: &str) -> Option<&'static str> {
    let trimmed = full_text.trim_start_matches(['\u{feff}', ' ', '\t', '\r', '\n']);
    let lower_head: String = trimmed
        .chars()
        .take(64)
        .collect::<String>()
        .to_ascii_lowercase();

    if lower_head.starts_with("<?php") {
        return Some("php");
    }
    if lower_head.starts_with("<!doctype html") || lower_head.starts_with("<html") {
        return Some("html");
    }
    // `package main` (Go) — the first non-blank line begins with `package `.
    if let Some(first_line) = trimmed.lines().find(|l| !l.trim().is_empty()) {
        if first_line.trim_start().starts_with("package ") {
            return Some("go");
        }
    }
    // JSON shape: starts with `{` or `[` AND the trimmed tail closes it. A bare `{` of a Rust/JS block
    // is NOT JSON, so require a closing brace/bracket at the end of the trimmed document (a cheap shape
    // check that avoids misclassifying a code file whose first char is `{`).
    let trimmed_end = trimmed.trim_end();
    let starts_obj = trimmed.starts_with('{') && trimmed_end.ends_with('}');
    let starts_arr = trimmed.starts_with('[') && trimmed_end.ends_with(']');
    if (starts_obj || starts_arr) && looks_like_json(trimmed_end) {
        return Some("json");
    }
    None
}

/// A cheap JSON-shape confirmation: the document contains a quoted key or is an array/object of
/// primitives, and does NOT contain obvious code tokens (`fn `, `function `, `;` at line end). This
/// keeps a `{ let x = 1; }` code block from being mis-read as JSON (RISK-007 spirit — wrong detection
/// flipping editor behavior). Conservative: when unsure, returns false so the extension layer decides.
fn looks_like_json(s: &str) -> bool {
    // Must contain a `:` (object) or be a pure array; reject obvious code markers.
    let has_code_marker = s.contains("fn ")
        || s.contains("function ")
        || s.contains("=>")
        || s.contains("println!")
        || s.contains("//");
    if has_code_marker {
        return false;
    }
    // An object needs a `"key":` pair; an array is accepted on its bracket shape alone.
    s.starts_with('[') || s.contains("\":") || s.contains("\" :")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_beats_everything() {
        // Extension says rust, shebang says python, but the user override says javascript -> override wins.
        let ov = LanguageId::new("javascript");
        let d = detect_language(
            Some(&ov),
            Some("script.rs"),
            b"#!/usr/bin/env python\n",
            "#!/usr/bin/env python\nprint('hi')\n",
        );
        assert_eq!(d.source, DetectionSource::UserOverride);
        assert_eq!(d.detected.as_str(), "javascript");
    }

    #[test]
    fn shebang_beats_extension() {
        // A file named `.txt` (no grammar) whose shebang is python resolves to python via the shebang
        // layer — NOT the extension fallback (RISK-003).
        let d = detect_language(
            None,
            Some("noext.txt"),
            b"#!/usr/bin/env python3\n",
            "print(1)\n",
        );
        assert_eq!(d.source, DetectionSource::Shebang);
        assert_eq!(d.detected.as_str(), "python");

        let d2 = detect_language(None, Some("run"), b"#!/bin/bash\necho hi\n", "echo hi\n");
        assert_eq!(d2.source, DetectionSource::Shebang);
        assert_eq!(d2.detected.as_str(), "shell");
    }

    #[test]
    fn content_beats_extension() {
        // A `.txt` file whose content is `<?php` resolves to php via the content layer.
        let d = detect_language(None, Some("page.txt"), b"<?php echo 1;", "<?php echo 1;\n");
        assert_eq!(d.source, DetectionSource::Content);
        assert_eq!(d.detected.as_str(), "php");

        let go = detect_language(
            None,
            Some("main.txt"),
            b"package main\n",
            "package main\n\nfunc main() {}\n",
        );
        assert_eq!(go.source, DetectionSource::Content);
        assert_eq!(go.detected.as_str(), "go");

        let json = detect_language(None, Some("data.txt"), b"{\n", "{\n  \"a\": 1\n}\n");
        assert_eq!(json.source, DetectionSource::Content);
        assert_eq!(json.detected.as_str(), "json");
    }

    #[test]
    fn extension_is_the_fallback() {
        // No override, no shebang, no content marker -> the extension decides (REUSES MT-001).
        let d = detect_language(None, Some("lib.rs"), b"fn main() {}\n", "fn main() {}\n");
        assert_eq!(d.source, DetectionSource::Extension);
        assert_eq!(d.detected.as_str(), "rust");

        // Unknown extension + no markers -> plain text (empty family), still via the Extension layer.
        let plain = detect_language(None, Some("notes.unknownext"), b"hello\n", "hello world\n");
        assert_eq!(plain.source, DetectionSource::Extension);
        assert!(plain.detected.is_plain_text());
    }

    #[test]
    fn code_brace_is_not_json() {
        // A JS block starting with `{` and ending with `}` must NOT be mis-detected as JSON; it falls
        // through to the extension (RISK-007: wrong detection flipping behavior).
        let d = detect_language(
            None,
            Some("snippet.js"),
            b"{ function f() {} }",
            "{ function f() {} }",
        );
        assert_eq!(d.source, DetectionSource::Extension);
        assert_eq!(d.detected.as_str(), "javascript");
    }

    #[test]
    fn available_languages_sourced_from_registry() {
        let langs = available_languages();
        // Plain Text leads.
        assert!(langs[0].is_plain_text());
        // The bundled grammars (rust, javascript) are present — matching the highlighter (AC-001).
        assert!(langs.iter().any(|l| l.as_str() == "rust"), "rust in picker");
        assert!(
            langs.iter().any(|l| l.as_str() == "javascript"),
            "javascript in picker"
        );
        // The heuristic-only families a shebang/content layer can report are selectable.
        for fam in ["python", "shell", "go", "php", "html", "json"] {
            assert!(
                langs.iter().any(|l| l.as_str() == fam),
                "{fam} selectable in picker"
            );
        }
        // No duplicates.
        let mut seen = std::collections::HashSet::new();
        for l in &langs {
            assert!(
                seen.insert(l.as_str().to_owned()),
                "no duplicate language {}",
                l.as_str()
            );
        }
    }

    #[test]
    fn display_labels_are_human_readable() {
        assert_eq!(LanguageId::plain_text().display_label(), "Plain Text");
        assert_eq!(LanguageId::new("rust").display_label(), "Rust");
        assert_eq!(LanguageId::new("javascript").display_label(), "JavaScript");
        assert_eq!(LanguageId::new("shell").display_label(), "Shell Script");
    }
}
