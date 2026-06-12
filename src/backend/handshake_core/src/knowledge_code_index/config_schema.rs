//! WP-KERNEL-009 MT-101 ConfigAndSchemaExtractor.
//!
//! Master Spec anchor: 2.3.13.11 KnowledgeEntity (`schema`, `command`,
//! `config`-class facts) + KnowledgeSpan. Extracts package/Cargo/JSON/YAML/
//! migration/schema facts with SOURCE SPANS so config keys are navigable and
//! citeable just like code symbols.
//!
//! Pure data; no DB. The engine maps each [`ConfigFact`] to a `schema`-kind
//! span + an entity (kind `schema` for JSON-schema definitions, else `command`
//! for npm scripts / `concept` for plain config keys — chosen by the engine
//! from `fact_kind`).
//!
//! Parser strategy:
//! * JSON / package.json: parsed with `serde_json` (already a dependency),
//!   walking the object tree into dotted key paths with line spans. A JSON file
//!   that declares `"$schema"` or top-level `"properties"`/`"definitions"` is
//!   treated as a JSON-Schema and its property/definition names become `schema`
//!   facts.
//! * YAML: parsed with `serde_yaml` (already a dependency) into the same dotted
//!   key model.
//! * TOML (Cargo.toml and friends): there is NO `toml` crate in the lockfile
//!   and the code-index MT must not add one, so we use a small, deterministic
//!   line scanner that recognises `[table]` / `[[array.table]]` headers and
//!   `key = value` assignments at the current table scope. This is sufficient
//!   for navigation facts (table + key paths with line spans); it does not
//!   evaluate TOML values.
//!
//! Determinism: keys are emitted in document order with their 1-based line.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// What a config fact represents (drives the entity kind the engine writes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFactKind {
    /// A plain configuration key (json/yaml/toml object key path).
    ConfigKey,
    /// A JSON-Schema property or definition name.
    SchemaProperty,
    /// An npm `scripts.<name>` entry (a runnable command).
    PackageScript,
    /// A TOML `[table]` / `[[array]]` table header.
    TomlTable,
}

impl ConfigFactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConfigKey => "config_key",
            Self::SchemaProperty => "schema_property",
            Self::PackageScript => "package_script",
            Self::TomlTable => "toml_table",
        }
    }
}

/// The config file format detected from the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Yaml,
    Toml,
}

/// Detect the config format from a repo-relative path, or `None` if it is not
/// a config file this extractor handles.
pub fn detect_config_format(relative_path: &str) -> Option<ConfigFormat> {
    let lower = relative_path.to_ascii_lowercase();
    if lower.ends_with(".json") {
        Some(ConfigFormat::Json)
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        Some(ConfigFormat::Yaml)
    } else if lower.ends_with(".toml") {
        Some(ConfigFormat::Toml)
    } else {
        None
    }
}

/// One extracted config fact (key path + line span). Pure data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFact {
    pub fact_kind: ConfigFactKind,
    /// Dotted key path, e.g. `dependencies.serde` or `properties.title`.
    pub key_path: String,
    /// 1-based line where the key appears.
    pub line: u32,
    /// Byte offset of the key line start (for the span range).
    pub byte_start: usize,
    pub byte_end: usize,
}

impl ConfigFact {
    pub fn entity_key(&self, relative_path: &str) -> String {
        format!("config:{relative_path}#{}", self.key_path)
    }
}

/// Extract config facts from a config file's text. Returns `Err(reason)` when
/// the file cannot be parsed (the engine records a typed failure receipt).
pub fn extract_config_facts(
    format: ConfigFormat,
    relative_path: &str,
    text: &str,
) -> Result<Vec<ConfigFact>, String> {
    match format {
        ConfigFormat::Json => extract_json(relative_path, text),
        ConfigFormat::Yaml => extract_yaml(text),
        ConfigFormat::Toml => Ok(extract_toml(text)),
    }
}

/// Line offsets for mapping byte positions to 1-based lines.
fn line_index(text: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(idx + 1);
        }
    }
    offsets
}

/// What kind of key token to locate, so the locator matches a KEY position
/// rather than any literal substring (a literal `text.find` would happily match
/// the key string inside a value, or a shorter key inside a longer one — e.g.
/// `"name"` inside `"display_name"`). MT-101 hardening.
#[derive(Clone, Copy)]
enum KeyToken<'a> {
    /// JSON object key: a quoted `"leaf"` immediately followed (after optional
    /// whitespace) by `:`.
    Json(&'a str),
    /// YAML mapping key: `leaf` that begins a (trimmed) line and is followed by
    /// `:` then end-of-line or whitespace.
    Yaml(&'a str),
}

/// Structured key locator: returns the 1-based line + that line's byte range for
/// the next occurrence of `token` AT A KEY POSITION at or after `from_byte`.
/// Falls back to `from_byte` if no structured match is found (degrades to the
/// previous behaviour rather than panicking), so a quirky document still yields
/// a span instead of aborting the index.
fn locate_key(
    text: &str,
    offsets: &[usize],
    token: KeyToken<'_>,
    from_byte: usize,
) -> (u32, usize, usize) {
    let start = from_byte.min(text.len());
    let abs = match token {
        KeyToken::Json(leaf) => find_json_key(text, leaf, start),
        KeyToken::Yaml(leaf) => find_yaml_key(text, leaf, start),
    }
    .unwrap_or(start);
    let line = match offsets.binary_search(&abs) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let line_start = offsets.get(line).copied().unwrap_or(0);
    let line_end = offsets.get(line + 1).copied().unwrap_or(text.len());
    ((line + 1) as u32, line_start, line_end)
}

/// Find the absolute byte offset of `"leaf"` used as a JSON object key (quoted,
/// then `:` after optional whitespace) at or after `from`.
fn find_json_key(text: &str, leaf: &str, from: usize) -> Option<usize> {
    let quoted = format!("\"{leaf}\"");
    let mut search_from = from;
    while let Some(rel) = text[search_from.min(text.len())..].find(&quoted) {
        let abs = search_from + rel;
        let after = abs + quoted.len();
        // A key is followed (after whitespace) by ':'.
        if text[after..].trim_start().starts_with(':') {
            return Some(abs);
        }
        search_from = after;
    }
    None
}

/// Find the absolute byte offset of `leaf` used as a YAML mapping key (the leaf
/// begins a trimmed line and is followed by `:` then EOL/space) at or after
/// `from`.
fn find_yaml_key(text: &str, leaf: &str, from: usize) -> Option<usize> {
    let needle = format!("{leaf}:");
    let mut search_from = from;
    while let Some(rel) = text[search_from.min(text.len())..].find(&needle) {
        let abs = search_from + rel;
        // The text from the line start up to `abs` must be only indentation /
        // list markers (so we matched a key, not a substring inside a value).
        let line_start = text[..abs].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prefix = &text[line_start..abs];
        let prefix_is_keyish = prefix.chars().all(|c| c == ' ' || c == '\t' || c == '-');
        // What follows the colon must be EOL or whitespace (a value), not more
        // identifier characters (which would mean we hit `keyfoo:` for `key`).
        let after = abs + needle.len();
        let next_ok = text[after..]
            .chars()
            .next()
            .map(|c| c.is_whitespace())
            .unwrap_or(true);
        if prefix_is_keyish && next_ok {
            return Some(abs);
        }
        search_from = abs + needle.len();
    }
    None
}

fn extract_json(relative_path: &str, text: &str) -> Result<Vec<ConfigFact>, String> {
    let value: JsonValue =
        serde_json::from_str(text).map_err(|err| format!("invalid JSON: {err}"))?;
    let offsets = line_index(text);
    let mut facts = Vec::new();

    let is_package = relative_path.to_ascii_lowercase().ends_with("package.json");
    let is_schema = json_is_schema(&value);

    let mut cursor = 0usize;
    walk_json(
        &value,
        "",
        text,
        &offsets,
        &mut cursor,
        is_package,
        is_schema,
        &mut facts,
        0,
    );
    Ok(facts)
}

/// Heuristic: a JSON-Schema document declares `$schema` or has top-level
/// `properties`/`definitions`/`$defs`.
fn json_is_schema(value: &JsonValue) -> bool {
    let JsonValue::Object(map) = value else {
        return false;
    };
    map.contains_key("$schema")
        || map.contains_key("properties")
        || map.contains_key("definitions")
        || map.contains_key("$defs")
}

#[allow(clippy::too_many_arguments)]
fn walk_json(
    value: &JsonValue,
    prefix: &str,
    text: &str,
    offsets: &[usize],
    cursor: &mut usize,
    is_package: bool,
    is_schema: bool,
    out: &mut Vec<ConfigFact>,
    depth: usize,
) {
    // Cap recursion depth so a pathological document cannot blow the stack.
    if depth > 24 {
        return;
    }
    let JsonValue::Object(map) = value else {
        return;
    };
    for (key, child) in map {
        let key_path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        let (line, byte_start, byte_end) = locate_key(text, offsets, KeyToken::Json(key), *cursor);
        *cursor = byte_end;

        let fact_kind = if is_package && key_path.starts_with("scripts.") {
            ConfigFactKind::PackageScript
        } else if is_schema && is_schema_member(prefix) {
            ConfigFactKind::SchemaProperty
        } else {
            ConfigFactKind::ConfigKey
        };

        out.push(ConfigFact {
            fact_kind,
            key_path: key_path.clone(),
            line,
            byte_start,
            byte_end,
        });

        walk_json(
            child,
            &key_path,
            text,
            offsets,
            cursor,
            is_package,
            is_schema,
            out,
            depth + 1,
        );
    }
}

/// Members directly under `properties`/`definitions`/`$defs` are schema
/// properties.
fn is_schema_member(prefix: &str) -> bool {
    let last = prefix.rsplit('.').next().unwrap_or(prefix);
    matches!(last, "properties" | "definitions" | "$defs")
}

fn extract_yaml(text: &str) -> Result<Vec<ConfigFact>, String> {
    let value: serde_yaml::Value =
        serde_yaml::from_str(text).map_err(|err| format!("invalid YAML: {err}"))?;
    let offsets = line_index(text);
    let mut facts = Vec::new();
    let mut cursor = 0usize;
    walk_yaml(&value, "", text, &offsets, &mut cursor, &mut facts, 0);
    Ok(facts)
}

fn walk_yaml(
    value: &serde_yaml::Value,
    prefix: &str,
    text: &str,
    offsets: &[usize],
    cursor: &mut usize,
    out: &mut Vec<ConfigFact>,
    depth: usize,
) {
    if depth > 24 {
        return;
    }
    let serde_yaml::Value::Mapping(map) = value else {
        return;
    };
    for (key, child) in map {
        let Some(key) = key.as_str() else { continue };
        let key_path = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        };
        let (line, byte_start, byte_end) = locate_key(text, offsets, KeyToken::Yaml(key), *cursor);
        *cursor = byte_start; // YAML siblings share indentation; do not over-advance.
        out.push(ConfigFact {
            fact_kind: ConfigFactKind::ConfigKey,
            key_path: key_path.clone(),
            line,
            byte_start,
            byte_end,
        });
        walk_yaml(child, &key_path, text, offsets, cursor, out, depth + 1);
    }
}

/// Minimal deterministic TOML scanner: `[table]`, `[[array.table]]`, and
/// `key = value` at the current table scope. Does not evaluate values.
///
/// MT-101 hardening: it also (a) skips continuation lines of a MULTILINE array
/// value so an `=` inside an array element (e.g. `"foo = 1"`) is not misread as
/// a key, (b) expands INLINE TABLES (`pt = { x = 1, y = 2 }`) into dotted inner
/// keys (`pt.x`, `pt.y`), and (c) keeps dotted bare keys (`a.b.c = 1`) as a
/// single dotted path under the current table.
fn extract_toml(text: &str) -> Vec<ConfigFact> {
    let mut facts = Vec::new();
    let mut current_table = String::new();
    let mut byte_cursor = 0usize;
    // Net unclosed '[' brackets carried from prior lines (multiline arrays).
    let mut open_array_depth: i32 = 0;
    for (lineno0, raw_line) in text.split_inclusive('\n').enumerate() {
        let line_start = byte_cursor;
        byte_cursor += raw_line.len();
        let line_end = byte_cursor;
        let line = raw_line.trim_end_matches(['\n', '\r']);
        let trimmed = line.trim_start();
        let lineno = (lineno0 + 1) as u32;

        // Inside an open multiline array: this line is value continuation, not a
        // key. Just update bracket depth and move on.
        if open_array_depth > 0 {
            open_array_depth += array_bracket_delta(line);
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("[[") {
            if let Some(name) = rest.split_once("]]").map(|(n, _)| n.trim()) {
                current_table = name.to_string();
                facts.push(ConfigFact {
                    fact_kind: ConfigFactKind::TomlTable,
                    key_path: name.to_string(),
                    line: lineno,
                    byte_start: line_start,
                    byte_end: line_end,
                });
                continue;
            }
        }
        if let Some(rest) = trimmed.strip_prefix('[') {
            if let Some(name) = rest.split_once(']').map(|(n, _)| n.trim()) {
                current_table = name.to_string();
                facts.push(ConfigFact {
                    fact_kind: ConfigFactKind::TomlTable,
                    key_path: name.to_string(),
                    line: lineno,
                    byte_start: line_start,
                    byte_end: line_end,
                });
                continue;
            }
        }
        // key = value (key may be quoted or bare; stop at first top-level '=').
        if let Some((key_part, value)) = split_toml_assignment(trimmed) {
            let key = normalize_toml_key(key_part);
            if key.is_empty() {
                continue;
            }
            let key_path = if current_table.is_empty() {
                key.clone()
            } else {
                format!("{current_table}.{key}")
            };
            facts.push(ConfigFact {
                fact_kind: ConfigFactKind::ConfigKey,
                key_path: key_path.clone(),
                line: lineno,
                byte_start: line_start,
                byte_end: line_end,
            });
            // Inline table: expand inner `inner = ...` assignments into dotted
            // keys on the same line.
            let value_trimmed = value.trim();
            if let Some(inner) = value_trimmed
                .strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
            {
                for piece in split_top_level_commas(inner) {
                    if let Some((ik, _)) = split_toml_assignment(piece.trim()) {
                        let inner_key = normalize_toml_key(ik);
                        if !inner_key.is_empty() {
                            facts.push(ConfigFact {
                                fact_kind: ConfigFactKind::ConfigKey,
                                key_path: format!("{key_path}.{inner_key}"),
                                line: lineno,
                                byte_start: line_start,
                                byte_end: line_end,
                            });
                        }
                    }
                }
            } else {
                // A value that opens an array but does not close it on this line
                // begins a multiline array; skip following continuation lines.
                open_array_depth += array_bracket_delta(value);
                if open_array_depth < 0 {
                    open_array_depth = 0;
                }
            }
        }
    }
    facts
}

/// Net `[` minus `]` brackets that are NOT inside a quoted string, used to track
/// multiline-array nesting.
fn array_bracket_delta(line: &str) -> i32 {
    let mut depth = 0i32;
    let mut in_quote = false;
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_quote = !in_quote,
            b'#' if !in_quote => break, // comment to EOL
            b'[' if !in_quote => depth += 1,
            b']' if !in_quote => depth -= 1,
            _ => {}
        }
        let _ = i;
    }
    depth
}

/// Normalize a TOML key part: trim, strip surrounding quotes on each dotted
/// segment, re-join with dots so `a."b c".d` becomes `a.b c.d`.
fn normalize_toml_key(key_part: &str) -> String {
    key_part
        .split('.')
        .map(|seg| seg.trim().trim_matches('"').trim())
        .filter(|seg| !seg.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}

/// Split on top-level commas (ignoring commas inside quotes or nested braces),
/// used to break an inline table body into its assignments.
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut start = 0usize;
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_quote = !in_quote,
            b'{' | b'[' if !in_quote => depth += 1,
            b'}' | b']' if !in_quote => depth -= 1,
            b',' if !in_quote && depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

/// Split a TOML line at the first top-level `=` (ignores `=` inside quotes).
fn split_toml_assignment(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    let mut in_quote = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_quote = !in_quote,
            b'=' if !in_quote => return Some((&line[..i], &line[i + 1..])),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_formats() {
        assert_eq!(detect_config_format("a/b.json"), Some(ConfigFormat::Json));
        assert_eq!(detect_config_format("a/b.yaml"), Some(ConfigFormat::Yaml));
        assert_eq!(detect_config_format("a/b.yml"), Some(ConfigFormat::Yaml));
        assert_eq!(detect_config_format("Cargo.toml"), Some(ConfigFormat::Toml));
        assert_eq!(detect_config_format("a/b.rs"), None);
    }

    #[test]
    fn json_package_scripts_and_keys() {
        let text = r#"{
  "name": "demo",
  "scripts": {
    "build": "tsc",
    "test": "vitest"
  }
}"#;
        let facts = extract_config_facts(ConfigFormat::Json, "package.json", text).unwrap();
        let scripts: Vec<&ConfigFact> = facts
            .iter()
            .filter(|f| f.fact_kind == ConfigFactKind::PackageScript)
            .collect();
        assert_eq!(scripts.len(), 2, "{facts:?}");
        assert!(scripts.iter().any(|f| f.key_path == "scripts.build"));
        assert!(facts.iter().any(|f| f.key_path == "name"));
    }

    #[test]
    fn json_schema_properties() {
        let text = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "properties": {
    "title": { "type": "string" },
    "count": { "type": "number" }
  }
}"#;
        let facts = extract_config_facts(ConfigFormat::Json, "thing.schema.json", text).unwrap();
        let props: Vec<&str> = facts
            .iter()
            .filter(|f| f.fact_kind == ConfigFactKind::SchemaProperty)
            .map(|f| f.key_path.as_str())
            .collect();
        assert!(props.contains(&"properties.title"), "{facts:?}");
        assert!(props.contains(&"properties.count"), "{facts:?}");
    }

    #[test]
    fn yaml_keys_with_lines() {
        let text = "name: demo\nspec:\n  replicas: 3\n";
        let facts = extract_config_facts(ConfigFormat::Yaml, "deploy.yaml", text).unwrap();
        assert!(facts.iter().any(|f| f.key_path == "name" && f.line == 1));
        assert!(facts.iter().any(|f| f.key_path == "spec.replicas"));
    }

    #[test]
    fn toml_tables_and_keys() {
        let text = "name = \"demo\"\n\n[dependencies]\nserde = \"1\"\n\n[[bin]]\nname = \"x\"\n";
        let facts = extract_config_facts(ConfigFormat::Toml, "Cargo.toml", text).unwrap();
        assert!(facts
            .iter()
            .any(|f| f.fact_kind == ConfigFactKind::TomlTable && f.key_path == "dependencies"));
        assert!(facts.iter().any(|f| f.key_path == "dependencies.serde"));
        assert!(facts
            .iter()
            .any(|f| f.fact_kind == ConfigFactKind::TomlTable && f.key_path == "bin"));
        // top-level key before any table.
        assert!(facts.iter().any(|f| f.key_path == "name" && f.line == 1));
    }

    #[test]
    fn malformed_json_is_typed_error() {
        let err = extract_config_facts(ConfigFormat::Json, "x.json", "{ not json").unwrap_err();
        assert!(err.contains("invalid JSON"), "{err}");
    }

    // ---- MT-101 hardening: structured locator + TOML edge cases ----

    #[test]
    fn json_locator_does_not_match_key_string_inside_a_value() {
        // The value of "a" literally contains the token `"b"`; the structured
        // locator must put key `b` on its real key line (3), not line 2.
        let text = "{\n  \"a\": \"contains \\\"b\\\" inside\",\n  \"b\": 1\n}";
        let facts = extract_config_facts(ConfigFormat::Json, "x.json", text).unwrap();
        let b = facts.iter().find(|f| f.key_path == "b").expect("key b");
        assert_eq!(b.line, 3, "key b must resolve to its key line, got {b:?}");
    }

    #[test]
    fn json_locator_distinguishes_substring_keys() {
        // `name` must not bind to the `display_name` line.
        let text = "{\n  \"display_name\": \"x\",\n  \"name\": \"y\"\n}";
        let facts = extract_config_facts(ConfigFormat::Json, "x.json", text).unwrap();
        let name = facts
            .iter()
            .find(|f| f.key_path == "name")
            .expect("key name");
        assert_eq!(name.line, 3, "got {name:?}");
        let dn = facts
            .iter()
            .find(|f| f.key_path == "display_name")
            .expect("display_name");
        assert_eq!(dn.line, 2, "got {dn:?}");
    }

    #[test]
    fn toml_multiline_array_does_not_emit_spurious_keys_from_values() {
        // The `=` inside the array element string must NOT be read as a key.
        let text = "[deps]\nmembers = [\n  \"foo = 1\",\n  \"bar = 2\",\n]\nname = \"x\"\n";
        let facts = extract_config_facts(ConfigFormat::Toml, "Cargo.toml", text).unwrap();
        assert!(facts.iter().any(|f| f.key_path == "deps.members"));
        // `name` after the array still parses.
        assert!(facts.iter().any(|f| f.key_path == "deps.name"));
        // No spurious `deps.foo` / `deps.bar` from the array element strings.
        assert!(
            !facts
                .iter()
                .any(|f| f.key_path.ends_with(".foo") || f.key_path.ends_with(".bar")),
            "multiline array values leaked as keys: {facts:?}"
        );
    }

    #[test]
    fn toml_inline_table_expands_inner_keys() {
        let text = "[pkg]\npoint = { x = 1, y = 2 }\n";
        let facts = extract_config_facts(ConfigFormat::Toml, "Cargo.toml", text).unwrap();
        assert!(facts.iter().any(|f| f.key_path == "pkg.point"));
        assert!(
            facts.iter().any(|f| f.key_path == "pkg.point.x"),
            "{facts:?}"
        );
        assert!(
            facts.iter().any(|f| f.key_path == "pkg.point.y"),
            "{facts:?}"
        );
    }

    #[test]
    fn toml_dotted_bare_key_is_kept_as_dotted_path() {
        let text = "[a]\nb.c.d = 1\n";
        let facts = extract_config_facts(ConfigFormat::Toml, "Cargo.toml", text).unwrap();
        assert!(facts.iter().any(|f| f.key_path == "a.b.c.d"), "{facts:?}");
    }

    #[test]
    fn toml_quoted_dotted_key_segments_are_unquoted() {
        let text = "\"weird.key\" = 1\n";
        let facts = extract_config_facts(ConfigFormat::Toml, "Cargo.toml", text).unwrap();
        // Quotes stripped per segment; the literal dot inside the quotes is kept
        // as a path separator (deterministic, navigation-grade).
        assert!(facts.iter().any(|f| f.key_path == "weird.key"), "{facts:?}");
    }
}
