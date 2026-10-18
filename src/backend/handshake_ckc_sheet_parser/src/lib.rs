mod parser;
mod types;

pub use parser::{
    parse_sheet_template_ast, sha256_hex, sheet_field_id_from_line, sheet_line_looks_like_field,
    split_field_line, split_block_instance_field_line, ParsedBlockInstanceFieldLine,
};
pub use types::{
    SheetTemplateAst, SheetTemplateSection, SheetBlockSchema, SheetBlockInstance,
    SheetBlockInstanceField, SheetTemplateField, SheetUnmappedLine, ParsedSheetFieldType,
    descriptor_inner, split_descriptor_tokens, editable_roles_from_token,
};

mod editing;

pub use editing::{
    apply_field_edits_to_raw_text, normalize_replacement_for_type, sheet_edit_key,
    validate_sheet_field_edits, validate_sheet_field_selectors, SheetFieldEdit, SheetFieldSelector,
};

#[cfg(test)]
mod editing_tests;
