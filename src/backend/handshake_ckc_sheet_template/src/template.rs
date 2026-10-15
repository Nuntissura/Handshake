use crate::syntax::{
    count_template_field_lines, count_template_section_lines, single_line_value,
};
use crate::{
    BuiltInSafeSubset, BuiltInSheetTemplate, CHARACTER_SHEET_V2_FILE_NAME,
    CHARACTER_SHEET_V2_TEMPLATE_ID, CHARACTER_SHEET_V2_TEMPLATE_VERSION, LLM_SAFE_SUBSET_V2_FILE_NAME,
};

pub fn builtin_character_sheet_template(
    template_text: &str,
    hash: fn(&str) -> String,
) -> BuiltInSheetTemplate {
    BuiltInSheetTemplate {
        template_id: CHARACTER_SHEET_V2_TEMPLATE_ID.to_owned(),
        template_version: CHARACTER_SHEET_V2_TEMPLATE_VERSION.to_owned(),
        file_name: CHARACTER_SHEET_V2_FILE_NAME.to_owned(),
        template_hash: hash(template_text),
        field_count: count_template_field_lines(template_text),
        section_count: count_template_section_lines(template_text),
        raw_text: template_text.to_owned(),
    }
}

pub fn builtin_safe_subset(raw_json: &str) -> Result<BuiltInSafeSubset, String> {
    let field_ids = serde_json::from_str::<Vec<String>>(raw_json)
        .map_err(|err| format!("invalid safe subset JSON: {err}"))?;
    Ok(BuiltInSafeSubset {
        template_id: CHARACTER_SHEET_V2_TEMPLATE_ID.to_owned(),
        template_version: CHARACTER_SHEET_V2_TEMPLATE_VERSION.to_owned(),
        file_name: LLM_SAFE_SUBSET_V2_FILE_NAME.to_owned(),
        field_count: field_ids.len(),
        field_ids,
        raw_json: raw_json.to_owned(),
    })
}

pub fn default_character_sheet_text(
    template_text: &str,
    public_id: &str,
    display_name: &str,
) -> String {
    let public_id = single_line_value(public_id);
    let display_name = single_line_value(display_name);
    template_text
        .replace(
            "CHAR-ID-001 \u{2014} Character_ID: <string>",
            &format!("CHAR-ID-001 \u{2014} Character_ID: {public_id}"),
        )
        .replace(
            "CHAR-ID-002 \u{2014} Name: <string>",
            &format!("CHAR-ID-002 \u{2014} Name: {display_name}"),
        )
}
