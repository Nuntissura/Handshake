//! Built-in CKC character sheet templates.
//!
//! The v2.00 text is bundled from the original CastKit-Codex governance
//! template byte-for-byte so Handshake can create/import/export CKC sheets
//! without depending on the old app's runtime path.

use serde::{Deserialize, Serialize};

use super::sheet::sha256_hex;
use super::{AtelierError, AtelierResult};

pub const CHARACTER_SHEET_V2_TEMPLATE_ID: &str = "ckc-character-sheet";
pub const CHARACTER_SHEET_V2_TEMPLATE_VERSION: &str = "v2.00";
pub const CHARACTER_SHEET_V2_FILE_NAME: &str = "CHARACTER_SHEET__v2.00.txt";
pub const LLM_SAFE_SUBSET_V2_FILE_NAME: &str = "LLM_SAFE_SUBSET__v2.00.json";
pub const DEFAULT_SHEET_TOOL: &str = "handshake-core-ckc-template-v2";

pub const CHARACTER_SHEET_V2_TEXT: &str = include_str!("templates/CHARACTER_SHEET__v2.00.txt");
pub const LLM_SAFE_SUBSET_V2_JSON: &str = include_str!("templates/LLM_SAFE_SUBSET__v2.00.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuiltInSheetTemplate {
    pub template_id: String,
    pub template_version: String,
    pub file_name: String,
    pub template_hash: String,
    pub field_count: usize,
    pub section_count: usize,
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuiltInSafeSubset {
    pub template_id: String,
    pub template_version: String,
    pub file_name: String,
    pub field_count: usize,
    pub field_ids: Vec<String>,
    pub raw_json: String,
}

pub fn builtin_character_sheet_template() -> AtelierResult<BuiltInSheetTemplate> {
    Ok(BuiltInSheetTemplate {
        template_id: CHARACTER_SHEET_V2_TEMPLATE_ID.to_owned(),
        template_version: CHARACTER_SHEET_V2_TEMPLATE_VERSION.to_owned(),
        file_name: CHARACTER_SHEET_V2_FILE_NAME.to_owned(),
        template_hash: sha256_hex(CHARACTER_SHEET_V2_TEXT),
        field_count: count_template_field_lines(CHARACTER_SHEET_V2_TEXT),
        section_count: count_template_section_lines(CHARACTER_SHEET_V2_TEXT),
        raw_text: CHARACTER_SHEET_V2_TEXT.to_owned(),
    })
}

pub fn builtin_safe_subset() -> AtelierResult<BuiltInSafeSubset> {
    let field_ids = serde_json::from_str::<Vec<String>>(LLM_SAFE_SUBSET_V2_JSON)
        .map_err(|err| AtelierError::Validation(format!("invalid safe subset JSON: {err}")))?;
    Ok(BuiltInSafeSubset {
        template_id: CHARACTER_SHEET_V2_TEMPLATE_ID.to_owned(),
        template_version: CHARACTER_SHEET_V2_TEMPLATE_VERSION.to_owned(),
        file_name: LLM_SAFE_SUBSET_V2_FILE_NAME.to_owned(),
        field_count: field_ids.len(),
        field_ids,
        raw_json: LLM_SAFE_SUBSET_V2_JSON.to_owned(),
    })
}

pub fn default_character_sheet_text(public_id: &str, display_name: &str) -> String {
    let public_id = single_line_value(public_id);
    let display_name = single_line_value(display_name);
    CHARACTER_SHEET_V2_TEXT
        .replace(
            "CHAR-ID-001 \u{2014} Character_ID: <string>",
            &format!("CHAR-ID-001 \u{2014} Character_ID: {public_id}"),
        )
        .replace(
            "CHAR-ID-002 \u{2014} Name: <string>",
            &format!("CHAR-ID-002 \u{2014} Name: {display_name}"),
        )
}

pub fn text_hash(text: &str) -> String {
    sha256_hex(text)
}

fn single_line_value(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

fn count_template_field_lines(raw_text: &str) -> usize {
    raw_text.lines().filter(|line| field_id_line(line)).count()
}

fn count_template_section_lines(raw_text: &str) -> usize {
    raw_text
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.contains(':')
                && line
                    .chars()
                    .next()
                    .map(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
                    .unwrap_or(false)
        })
        .count()
}

fn field_id_line(line: &str) -> bool {
    let Some(colon) = line.find(':') else {
        return false;
    };
    let before_colon = line[..colon].trim();
    let separator = [" \u{2014} ", " \u{2013} ", " - "]
        .iter()
        .find_map(|separator| before_colon.find(separator));
    let Some(separator) = separator else {
        return false;
    };
    let field_id = before_colon[..separator].trim();
    let mut saw_dash = false;
    !field_id.is_empty()
        && field_id.chars().all(|ch| {
            if ch == '-' {
                saw_dash = true;
                true
            } else {
                ch.is_ascii_uppercase() || ch.is_ascii_digit()
            }
        })
        && saw_dash
}
