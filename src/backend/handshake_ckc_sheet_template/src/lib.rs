mod syntax;
mod template;

use serde::{Deserialize, Serialize};

pub use template::{
    builtin_character_sheet_template, builtin_safe_subset, default_character_sheet_text,
};

pub const CHARACTER_SHEET_V2_TEMPLATE_ID: &str = "ckc-character-sheet";
pub const CHARACTER_SHEET_V2_TEMPLATE_VERSION: &str = "v2.00";
pub const CHARACTER_SHEET_V2_FILE_NAME: &str = "CHARACTER_SHEET__v2.00.txt";
pub const LLM_SAFE_SUBSET_V2_FILE_NAME: &str = "LLM_SAFE_SUBSET__v2.00.json";
pub const DEFAULT_SHEET_TOOL: &str = "handshake-core-ckc-template-v2";

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
