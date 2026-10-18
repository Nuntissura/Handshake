use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    descriptor_inner, editable_roles_from_token, split_block_instance_field_line,
    split_descriptor_tokens, split_field_line, ParsedSheetFieldType, SheetTemplateAst,
    SheetTemplateField,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetFieldEdit {
    pub block_instance_id: Option<String>,
    pub field_id: String,
    pub replacement_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetFieldSelector {
    pub block_instance_id: Option<String>,
    pub field_id: String,
}

fn split_line_ending(segment: &str) -> (&str, &str) {
    if let Some(line) = segment.strip_suffix("\r\n") {
        (line, "\r\n")
    } else if let Some(line) = segment.strip_suffix('\n') {
        (line, "\n")
    } else if let Some(line) = segment.strip_suffix('\r') {
        (line, "\r")
    } else {
        (segment, "")
    }
}

fn replace_line_descriptor(line: &str, replacement_text: &str) -> String {
    let Some(colon) = line.find(':') else {
        return line.to_string();
    };
    let descriptor_start = colon + 1;
    let after_colon = &line[descriptor_start..];
    let whitespace_len = after_colon
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(after_colon.len());
    let spacing = if whitespace_len == 0 {
        " "
    } else {
        &after_colon[..whitespace_len]
    };
    if let Some(value_start) = after_colon.find('<') {
        let mut depth = 0i32;
        let mut value_end = None;
        for (idx, ch) in after_colon[value_start..].char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        value_end = Some(value_start + idx + ch.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(value_end) = value_end {
            let absolute_start = descriptor_start + value_start;
            let absolute_end = descriptor_start + value_end;
            return format!(
                "{}{}{}",
                &line[..absolute_start],
                replacement_text,
                &line[absolute_end..]
            );
        }
    }
    format!(
        "{}{}{}",
        &line[..descriptor_start],
        spacing,
        replacement_text
    )
}

pub fn sheet_edit_key(edit: &SheetFieldEdit) -> String {
    let field_id = edit.field_id.trim();
    match edit.block_instance_id.as_deref().map(str::trim) {
        Some(instance_id) if !instance_id.is_empty() => format!("{instance_id}.{field_id}"),
        _ => field_id.to_string(),
    }
}

fn sheet_selector_key(selector: &SheetFieldSelector) -> String {
    let field_id = selector.field_id.trim();
    match selector.block_instance_id.as_deref().map(str::trim) {
        Some(instance_id) if !instance_id.is_empty() => format!("{instance_id}.{field_id}"),
        _ => field_id.to_string(),
    }
}

pub fn validate_sheet_field_selectors(selectors: &[SheetFieldSelector]) -> Result<Vec<String>, String> {
    let mut keys = Vec::with_capacity(selectors.len());
    let mut seen = HashSet::new();
    for selector in selectors {
        let field_id = selector.field_id.trim();
        if field_id.is_empty() {
            return Err("sheet field selector field_id must not be empty".to_string());
        }
        if matches!(
            selector.block_instance_id.as_deref().map(str::trim),
            Some("")
        ) {
            return Err("sheet field selector block_instance_id must not be empty when present".to_string());
        }
        let key = sheet_selector_key(selector);
        if !seen.insert(key.clone()) {
            return Err(format!(
                "duplicate sheet field selector for field_id={key}"
            ));
        }
        keys.push(key);
    }
    Ok(keys)
}

pub fn validate_sheet_field_edits(edits: &[SheetFieldEdit]) -> Result<(), String> {
    if edits.is_empty() {
        return Err("at least one sheet field edit is required".to_string());
    }
    let mut seen = HashSet::new();
    for edit in edits {
        let field_id = edit.field_id.trim();
        if field_id.is_empty() {
            return Err("sheet field edit field_id must not be empty".to_string());
        }
        if matches!(edit.block_instance_id.as_deref().map(str::trim), Some("")) {
            return Err("sheet field edit block_instance_id must not be empty when present".to_string());
        }
        let edit_key = sheet_edit_key(edit);
        if !seen.insert(edit_key.clone()) {
            return Err(format!(
                "duplicate sheet field edit for field_id={edit_key}"
            ));
        }
        if edit.replacement_text.contains('\n') || edit.replacement_text.contains('\r') {
            return Err(format!(
                "sheet field edit replacement_text for field_id={field_id} must be single-line"
            ));
        }
    }
    Ok(())
}

fn score_text(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        let mut text = format!("{value:.2}");
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        text
    }
}

fn normalize_score10_replacement(
    field_id: &str,
    replacement_text: &str,
    allowed_special_values: &[String],
) -> Result<String, String> {
    let trimmed = replacement_text.trim();
    let inner = descriptor_inner(trimmed);
    let normalized = inner.trim().to_ascii_lowercase();
    if allowed_special_values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(&normalized))
    {
        return Ok(format!("<{}>", normalized));
    }
    let numeric_text = normalized
        .strip_suffix("/10")
        .map(str::trim)
        .unwrap_or(normalized.as_str());
    let score = numeric_text.parse::<f64>().map_err(|_| {
        format!("invalid_score_10: field_id={field_id} replacement_text must be 0..10 or an allowed special")
    })?;
    if !(0.0..=10.0).contains(&score) {
        return Err(format!(
            "invalid_score_10: field_id={field_id} score must be within 0..10"
        ));
    }
    Ok(format!("<{}/10>", score_text(score)))
}

fn descriptor_replacement_text(replacement_text: &str) -> String {
    let trimmed = replacement_text.trim();
    if trimmed.starts_with('<') && trimmed.ends_with('>') {
        trimmed.to_string()
    } else {
        format!("<{trimmed}>")
    }
}

fn normalize_integer_replacement(
    field_id: &str,
    replacement_text: &str,
    allowed_special_values: &[String],
) -> Result<String, String> {
    let inner = descriptor_inner(replacement_text)
        .trim()
        .to_ascii_lowercase();
    if allowed_special_values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(&inner))
    {
        return Ok(format!("<{inner}>"));
    }
    let parsed = inner.parse::<i64>().map_err(|_| {
        format!("invalid_integer: field_id={field_id} replacement_text must be an integer")
    })?;
    Ok(format!("<{parsed}>"))
}

fn normalize_number_replacement(
    field_id: &str,
    replacement_text: &str,
    allowed_special_values: &[String],
) -> Result<String, String> {
    let inner = descriptor_inner(replacement_text)
        .trim()
        .to_ascii_lowercase();
    if allowed_special_values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(&inner))
    {
        return Ok(format!("<{inner}>"));
    }
    let parsed = inner.parse::<f64>().map_err(|_| {
        format!("invalid_number: field_id={field_id} replacement_text must be numeric")
    })?;
    Ok(format!("<{}>", score_text(parsed)))
}

fn normalize_enum_replacement(
    field_id: &str,
    replacement_text: &str,
    values: &[String],
    allow_other_type: Option<&str>,
    allowed_special_values: &[String],
) -> Result<String, String> {
    let inner = descriptor_inner(replacement_text)
        .trim()
        .to_ascii_lowercase();
    if allowed_special_values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(&inner))
        || values
            .iter()
            .any(|value| value.eq_ignore_ascii_case(&inner))
    {
        return Ok(format!("<{inner}>"));
    }
    if let Some(other_type) = allow_other_type {
        return match other_type {
            "integer" | "int" => normalize_integer_replacement(field_id, replacement_text, &[]),
            "number" | "float" => normalize_number_replacement(field_id, replacement_text, &[]),
            _ => Ok(descriptor_replacement_text(replacement_text)),
        };
    }
    Err(format!(
        "invalid_enum: field_id={field_id} replacement_text must match an allowed enum value"
    ))
}

pub fn normalize_replacement_for_type(
    field_id: &str,
    field_type: &ParsedSheetFieldType,
    field_allowed_special_values: &[String],
    replacement_text: &str,
) -> Result<String, String> {
    match field_type {
        ParsedSheetFieldType::Integer => {
            normalize_integer_replacement(field_id, replacement_text, field_allowed_special_values)
        }
        ParsedSheetFieldType::Number => {
            normalize_number_replacement(field_id, replacement_text, field_allowed_special_values)
        }
        ParsedSheetFieldType::String
        | ParsedSheetFieldType::Paragraph
        | ParsedSheetFieldType::Descriptor
        | ParsedSheetFieldType::Rule
        | ParsedSheetFieldType::List { .. }
        | ParsedSheetFieldType::Block { .. }
        | ParsedSheetFieldType::BlockList { .. }
        | ParsedSheetFieldType::Unknown { .. } => Ok(descriptor_replacement_text(replacement_text)),
        ParsedSheetFieldType::Score10 {
            allowed_special_values,
        } => normalize_score10_replacement(field_id, replacement_text, allowed_special_values),
        ParsedSheetFieldType::Enum {
            values,
            allow_other_type,
            allowed_special_values,
        } => normalize_enum_replacement(
            field_id,
            replacement_text,
            values,
            allow_other_type.as_deref(),
            allowed_special_values,
        ),
        ParsedSheetFieldType::Union {
            variants,
            allowed_special_values,
        } => {
            let inner = descriptor_inner(replacement_text)
                .trim()
                .to_ascii_lowercase();
            if allowed_special_values
                .iter()
                .any(|value| value.eq_ignore_ascii_case(&inner))
            {
                return Ok(format!("<{inner}>"));
            }
            for variant in variants {
                if let Ok(normalized) =
                    normalize_replacement_for_type(field_id, variant, &[], replacement_text)
                {
                    return Ok(normalized);
                }
            }
            Err(format!(
                "invalid_union: field_id={field_id} replacement_text must match one union variant"
            ))
        }
    }
}

fn preserve_unscoped_field_guard_tokens(replacement: String, field: &SheetTemplateField) -> String {
    if field.editable_roles.is_empty() {
        return replacement;
    }
    let Some(inner) = replacement
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
    else {
        return replacement;
    };
    let mut tokens = split_descriptor_tokens(inner)
        .into_iter()
        .filter(|token| editable_roles_from_token(token).is_none())
        .collect::<Vec<_>>();
    tokens.push(format!("editable:{}", field.editable_roles.join(",")));
    format!("<{}>", tokens.join("|"))
}

pub fn apply_field_edits_to_raw_text(
    raw_text: &str,
    ast: &SheetTemplateAst,
    edits: &[SheetFieldEdit],
) -> Result<(String, Vec<String>), String> {
    validate_sheet_field_edits(edits)?;
    let field_by_id: HashMap<&str, &SheetTemplateField> = ast
        .fields
        .iter()
        .map(|field| (field.id.as_str(), field))
        .collect();
    let parsed_instance_field_ids: HashSet<String> =
        ast.block_instances
            .iter()
            .flat_map(|instance| {
                instance.fields.iter().map(move |field| {
                    format!("{}.{}", instance.instance_id, field.field_id.as_str())
                })
            })
            .collect();
    for edit in edits {
        let field_id = edit.field_id.trim();
        if let Some(instance_id) = edit.block_instance_id.as_deref().map(str::trim) {
            let edit_key = format!("{instance_id}.{field_id}");
            if !parsed_instance_field_ids.contains(&edit_key) {
                return Err(format!(
                    "sheet field edit references unknown parsed block field_id={edit_key}"
                ));
            }
        } else {
            let field = field_by_id.get(field_id).ok_or_else(|| {
                format!(
                    "sheet field edit references unknown parsed field_id={field_id}"
                )
            })?;
            if let Some(block_schema_name) = field.block_schema_name.as_deref() {
                return Err(format!(
                    "block_instance_required: field_id={field_id} belongs to block schema {block_schema_name}; edit requires block_instance_id"
                ));
            }
        }
    }

    let mut edit_by_id = HashMap::new();
    let mut edit_by_instance_id = HashMap::new();
    for edit in edits {
        let field_id = edit.field_id.trim();
        let field = field_by_id.get(field_id).ok_or_else(|| {
            format!(
                "sheet field edit references unknown parsed field_id={field_id}"
            )
        })?;
        let mut replacement = normalize_replacement_for_type(
            field_id,
            &field.field_type,
            &field.allowed_special_values,
            &edit.replacement_text,
        )
        ?;
        if let Some(instance_id) = edit
            .block_instance_id
            .as_deref()
            .map(str::trim)
            .filter(|instance_id| !instance_id.is_empty())
        {
            edit_by_instance_id.insert(format!("{instance_id}.{field_id}"), replacement);
        } else {
            replacement = preserve_unscoped_field_guard_tokens(replacement, field);
            edit_by_id.insert(field_id.to_string(), replacement);
        }
    }
    let edit_by_id: HashMap<&str, &str> = edit_by_id
        .iter()
        .map(|(field_id, replacement)| (field_id.as_str(), replacement.as_str()))
        .collect();
    let edit_by_instance_id: HashMap<String, &str> = edit_by_instance_id
        .iter()
        .map(|(field_id, replacement)| (field_id.clone(), replacement.as_str()))
        .collect();
    let mut applied_field_ids = Vec::new();
    let mut output = String::with_capacity(raw_text.len());

    for segment in raw_text.split_inclusive('\n') {
        let (line, ending) = split_line_ending(segment);
        let trimmed = line.trim();
        if let Some(instance_field) = split_block_instance_field_line(trimmed) {
            let edit_key = format!(
                "{}[{}].{}",
                instance_field.root_field_id, instance_field.ordinal, instance_field.field_id
            );
            if let Some(replacement_text) = edit_by_instance_id.get(&edit_key) {
                output.push_str(&replace_line_descriptor(line, replacement_text));
                output.push_str(ending);
                applied_field_ids.push(edit_key);
                continue;
            }
        }
        if let Some((field_id, _, _)) = split_field_line(trimmed) {
            if let Some(replacement_text) = edit_by_id.get(field_id.as_str()) {
                output.push_str(&replace_line_descriptor(line, replacement_text));
                output.push_str(ending);
                applied_field_ids.push(field_id);
                continue;
            }
        }
        output.push_str(segment);
    }

    if applied_field_ids.len() != edits.len() {
        return Err("one or more parsed sheet field edits were not applied to source text".to_string());
    }
    if output == raw_text {
        return Err("sheet field edits produced no text change".to_string());
    }
    Ok((output, applied_field_ids))
}
