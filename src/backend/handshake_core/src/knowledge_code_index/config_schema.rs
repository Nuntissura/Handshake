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

/// 1-based line + that line's byte range for a key, found by literal search.
/// Used for JSON/YAML where the parsed model loses positions. Searches for
/// `"<leaf>"` (JSON) or `<leaf>:` (YAML) starting at `from_byte`.
fn locate_key(
    text: &str,
    offsets: &[usize],
    needle: &str,
    from_byte: usize,
) -> (u32, usize, usize) {
    let hay = &text[from_byte.min(text.len())..];
    let rel = hay.find(needle).unwrap_or(0);
    let abs = from_byte + rel;
    let line = match offsets.binary_search(&abs) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let line_start = offsets.get(line).copied().unwrap_or(0);
    let line_end = offsets.get(line + 1).copied().unwrap_or(text.len());
    ((line + 1) as u32, line_start, line_end)
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
        let (line, byte_start, byte_end) =
            locate_key(text, offsets, &format!("\"{key}\""), *cursor);
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
        let (line, byte_start, byte_end) = locate_key(text, offsets, &format!("{key}:"), *cursor);
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
fn extract_toml(text: &str) -> Vec<ConfigFact> {
    let mut facts = Vec::new();
    let mut current_table = String::new();
    let mut byte_cursor = 0usize;
    for (lineno0, raw_line) in text.split_inclusive('\n').enumerate() {
        let line_start = byte_cursor;
        byte_cursor += raw_line.len();
        let line_end = byte_cursor;
        let line = raw_line.trim_end_matches(['\n', '\r']);
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let lineno = (lineno0 + 1) as u32;

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
        // key = value (key may be quoted or bare; stop at first '=').
        if let Some((key_part, _value)) = split_toml_assignment(trimmed) {
            let key = key_part.trim().trim_matches('"').trim();
            if key.is_empty() {
                continue;
            }
            let key_path = if current_table.is_empty() {
                key.to_string()
            } else {
                format!("{current_table}.{key}")
            };
            facts.push(ConfigFact {
                fact_kind: ConfigFactKind::ConfigKey,
                key_path,
                line: lineno,
                byte_start: line_start,
                byte_end: line_end,
            });
        }
    }
    facts
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
}
