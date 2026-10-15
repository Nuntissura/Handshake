//! Built-in CKC character sheet templates.
//!
//! The v2.00 text is bundled from the original CastKit-Codex governance
//! template byte-for-byte so Handshake can create/import/export CKC sheets
//! without depending on the old app's runtime path. Storage-agnostic: nothing
//! here touches the embedded store.


use super::sheet::sha256_hex;
use super::{AtelierError, AtelierResult};

pub use handshake_ckc_sheet_template::{
    BuiltInSafeSubset, BuiltInSheetTemplate, CHARACTER_SHEET_V2_FILE_NAME,
    CHARACTER_SHEET_V2_TEMPLATE_ID, CHARACTER_SHEET_V2_TEMPLATE_VERSION, DEFAULT_SHEET_TOOL,
    LLM_SAFE_SUBSET_V2_FILE_NAME,
};

pub const CHARACTER_SHEET_V2_TEXT: &str = include_str!("templates/CHARACTER_SHEET__v2.00.txt");
pub const LLM_SAFE_SUBSET_V2_JSON: &str = include_str!("templates/LLM_SAFE_SUBSET__v2.00.json");


pub fn builtin_character_sheet_template() -> AtelierResult<BuiltInSheetTemplate> {
    Ok(handshake_ckc_sheet_template::builtin_character_sheet_template(
        CHARACTER_SHEET_V2_TEXT,
        sha256_hex,
    ))
}

pub fn builtin_safe_subset() -> AtelierResult<BuiltInSafeSubset> {
    handshake_ckc_sheet_template::builtin_safe_subset(LLM_SAFE_SUBSET_V2_JSON)
        .map_err(AtelierError::Validation)
}

pub fn default_character_sheet_text(public_id: &str, display_name: &str) -> String {
    handshake_ckc_sheet_template::default_character_sheet_text(
        CHARACTER_SHEET_V2_TEXT,
        public_id,
        display_name,
    )
}

pub fn text_hash(text: &str) -> String {
    sha256_hex(text)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_template_is_v2_with_owner_placeholders() {
        let template = builtin_character_sheet_template().expect("bundled template parses");
        assert_eq!(template.template_version, "v2.00");
        assert!(template.field_count > 100);
        assert!(template
            .raw_text
            .contains("CHAR-ID-001 \u{2014} Character_ID: <string>"));
        assert_eq!(template.template_hash.len(), 64);
    }

    #[test]
    fn default_sheet_text_normalizes_owner_values_to_one_line() {
        let text = default_character_sheet_text("  char-x\n", "Multi\tWord   Name ");
        assert!(text.contains("CHAR-ID-001 \u{2014} Character_ID: char-x"));
        assert!(text.contains("CHAR-ID-002 \u{2014} Name: Multi Word Name"));
    }

    #[test]
    fn safe_subset_lists_short_field_ids() {
        let subset = builtin_safe_subset().expect("bundled safe subset parses");
        assert_eq!(subset.field_count, subset.field_ids.len());
        assert!(subset.field_ids.iter().any(|id| id == "CHAR-ID-006"));
    }
}
