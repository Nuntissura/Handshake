//! Append-only character sheet versions (MT-012): updates never mutate prior
//! versions; each change is a new version with parent linkage and provenance,
//! preventing silent data loss when models or imports edit a sheet.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{
    AtelierResult, AtelierStore, BulkOperationReceipt, character_ref, event_family,
    reject_legacy_runtime_ref, sheet_version_ref,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetVersion {
    pub version_id: Uuid,
    pub character_internal_id: Uuid,
    pub parent_version_id: Option<Uuid>,
    pub seq: i64,
    pub raw_text: String,
    pub author: String,
    pub tool: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewSheetVersion {
    pub character_internal_id: Uuid,
    pub raw_text: String,
    pub author: String,
    pub tool: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetFieldSuggestion {
    pub field_id: String,
    pub value: String,
    pub occurrences: i64,
    pub latest_version_id: Uuid,
    pub latest_character_internal_id: Uuid,
    pub latest_sheet_version_ref: String,
    pub latest_character_ref: String,
}

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetFieldEditRequest {
    pub version_id: Uuid,
    pub template_id: String,
    pub source_path: Option<String>,
    pub expected_template_hash: Option<String>,
    pub actor_role: String,
    pub edits: Vec<SheetFieldEdit>,
    pub author: String,
    pub tool: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetFieldEditResult {
    pub version: SheetVersion,
    pub source_version_id: Uuid,
    pub applied_field_ids: Vec<String>,
    pub preserved_unmapped_lines: Vec<SheetUnmappedLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BulkSheetFieldEditResult {
    pub receipt: BulkOperationReceipt,
    pub results: Vec<SheetFieldEditResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetVersionRevertRequest {
    pub character_internal_id: Uuid,
    pub target_version_id: Uuid,
    pub template_id: String,
    pub source_path: Option<String>,
    pub expected_template_hash: Option<String>,
    pub actor_role: String,
    pub field_selectors: Vec<SheetFieldSelector>,
    pub author: String,
    pub tool: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetVersionRevertResult {
    pub version: SheetVersion,
    pub reverted_to_version_id: Uuid,
    pub previous_head_version_id: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedSheetTemplate {
    pub parse_id: Uuid,
    pub version_id: Uuid,
    pub template_id: String,
    pub source_path: Option<String>,
    pub ast: SheetTemplateAst,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetTemplateAst {
    pub template_id: String,
    pub template_version: Option<String>,
    pub template_hash: String,
    pub source_path: Option<String>,
    pub sections: Vec<SheetTemplateSection>,
    pub block_schemas: Vec<SheetBlockSchema>,
    pub block_instances: Vec<SheetBlockInstance>,
    pub fields: Vec<SheetTemplateField>,
    pub unmapped_lines: Vec<SheetUnmappedLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetTemplateSection {
    pub name: String,
    pub line_number: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetBlockSchema {
    pub name: String,
    pub line_number: usize,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetBlockInstance {
    pub instance_id: String,
    pub root_field_id: String,
    pub block_schema_name: String,
    pub ordinal: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub fields: Vec<SheetBlockInstanceField>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetBlockInstanceField {
    pub field_id: String,
    pub label: String,
    pub line_number: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub raw: String,
    pub template_descriptor: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetTemplateField {
    pub id: String,
    pub label: String,
    pub field_type: ParsedSheetFieldType,
    pub optional: bool,
    pub allowed_special_values: Vec<String>,
    pub section: Option<String>,
    pub block_schema_name: Option<String>,
    pub line_number: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub raw: String,
    pub template_descriptor: String,
    pub protected: bool,
    pub editable_roles: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetUnmappedLine {
    pub line_number: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub raw: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParsedSheetFieldType {
    String,
    Integer,
    Number,
    Paragraph,
    Descriptor,
    Rule,
    Score10 {
        allowed_special_values: Vec<String>,
    },
    List {
        item_type: String,
    },
    Block {
        block_schema_name: String,
    },
    BlockList {
        block_schema_name: String,
    },
    Enum {
        values: Vec<String>,
        allow_other_type: Option<String>,
        allowed_special_values: Vec<String>,
    },
    Union {
        variants: Vec<ParsedSheetFieldType>,
        allowed_special_values: Vec<String>,
    },
    Unknown {
        descriptor: String,
    },
}

fn version_from_row(row: &sqlx::postgres::PgRow) -> SheetVersion {
    SheetVersion {
        version_id: row.get("version_id"),
        character_internal_id: row.get("character_internal_id"),
        parent_version_id: row.get("parent_version_id"),
        seq: row.get("seq"),
        raw_text: row.get("raw_text"),
        author: row.get("author"),
        tool: row.get("tool"),
        created_at_utc: row.get("created_at_utc"),
    }
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

fn sheet_edit_key(edit: &SheetFieldEdit) -> String {
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

fn validate_sheet_field_selectors(selectors: &[SheetFieldSelector]) -> AtelierResult<Vec<String>> {
    let mut keys = Vec::with_capacity(selectors.len());
    let mut seen = HashSet::new();
    for selector in selectors {
        let field_id = selector.field_id.trim();
        if field_id.is_empty() {
            return Err(super::AtelierError::Validation(
                "sheet field selector field_id must not be empty".to_string(),
            ));
        }
        if matches!(
            selector.block_instance_id.as_deref().map(str::trim),
            Some("")
        ) {
            return Err(super::AtelierError::Validation(
                "sheet field selector block_instance_id must not be empty when present".to_string(),
            ));
        }
        let key = sheet_selector_key(selector);
        if !seen.insert(key.clone()) {
            return Err(super::AtelierError::Validation(format!(
                "duplicate sheet field selector for field_id={key}"
            )));
        }
        keys.push(key);
    }
    Ok(keys)
}

fn validate_sheet_field_edits(edits: &[SheetFieldEdit]) -> AtelierResult<()> {
    if edits.is_empty() {
        return Err(super::AtelierError::Validation(
            "at least one sheet field edit is required".to_string(),
        ));
    }
    let mut seen = HashSet::new();
    for edit in edits {
        let field_id = edit.field_id.trim();
        if field_id.is_empty() {
            return Err(super::AtelierError::Validation(
                "sheet field edit field_id must not be empty".to_string(),
            ));
        }
        if matches!(edit.block_instance_id.as_deref().map(str::trim), Some("")) {
            return Err(super::AtelierError::Validation(
                "sheet field edit block_instance_id must not be empty when present".to_string(),
            ));
        }
        let edit_key = sheet_edit_key(edit);
        if !seen.insert(edit_key.clone()) {
            return Err(super::AtelierError::Validation(format!(
                "duplicate sheet field edit for field_id={edit_key}"
            )));
        }
        if edit.replacement_text.contains('\n') || edit.replacement_text.contains('\r') {
            return Err(super::AtelierError::Validation(format!(
                "sheet field edit replacement_text for field_id={field_id} must be single-line"
            )));
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

fn normalize_replacement_for_type(
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

fn apply_field_edits_to_raw_text(
    raw_text: &str,
    ast: &SheetTemplateAst,
    edits: &[SheetFieldEdit],
) -> AtelierResult<(String, Vec<String>)> {
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
                return Err(super::AtelierError::Validation(format!(
                    "sheet field edit references unknown parsed block field_id={edit_key}"
                )));
            }
        } else {
            let field = field_by_id.get(field_id).ok_or_else(|| {
                super::AtelierError::Validation(format!(
                    "sheet field edit references unknown parsed field_id={field_id}"
                ))
            })?;
            if let Some(block_schema_name) = field.block_schema_name.as_deref() {
                return Err(super::AtelierError::Validation(format!(
                    "block_instance_required: field_id={field_id} belongs to block schema {block_schema_name}; edit requires block_instance_id"
                )));
            }
        }
    }

    let mut edit_by_id = HashMap::new();
    let mut edit_by_instance_id = HashMap::new();
    for edit in edits {
        let field_id = edit.field_id.trim();
        let field = field_by_id.get(field_id).ok_or_else(|| {
            super::AtelierError::Validation(format!(
                "sheet field edit references unknown parsed field_id={field_id}"
            ))
        })?;
        let mut replacement = normalize_replacement_for_type(
            field_id,
            &field.field_type,
            &field.allowed_special_values,
            &edit.replacement_text,
        )
        .map_err(super::AtelierError::Validation)?;
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
        return Err(super::AtelierError::Validation(
            "one or more parsed sheet field edits were not applied to source text".to_string(),
        ));
    }
    if output == raw_text {
        return Err(super::AtelierError::Validation(
            "sheet field edits produced no text change".to_string(),
        ));
    }
    Ok((output, applied_field_ids))
}

struct SheetFieldEditDenial {
    reason_code: &'static str,
    field_id: Option<String>,
    message: String,
}

fn deny_sheet_field_edit(
    reason_code: &'static str,
    field_id: Option<&str>,
    message: String,
) -> SheetFieldEditDenial {
    SheetFieldEditDenial {
        reason_code,
        field_id: field_id.map(ToOwned::to_owned),
        message,
    }
}

fn validate_sheet_field_edit_guard(
    ast: &SheetTemplateAst,
    request: &SheetFieldEditRequest,
) -> Result<(), SheetFieldEditDenial> {
    let actor_role = request.actor_role.trim().to_ascii_lowercase();
    if actor_role.is_empty() {
        return Err(deny_sheet_field_edit(
            "actor_role_required",
            None,
            "sheet field edit requires a non-empty actor_role".to_string(),
        ));
    }

    let Some(expected_hash) = request.expected_template_hash.as_deref() else {
        return Err(deny_sheet_field_edit(
            "stale_selection",
            None,
            "stale_selection: expected_template_hash is required for bounded sheet field apply"
                .to_string(),
        ));
    };
    if expected_hash.trim() != ast.template_hash {
        return Err(deny_sheet_field_edit(
            "stale_selection",
            None,
            format!(
                "stale_selection: expected template hash {} but source is {}",
                expected_hash.trim(),
                ast.template_hash
            ),
        ));
    }

    let field_by_id: HashMap<&str, &SheetTemplateField> = ast
        .fields
        .iter()
        .map(|field| (field.id.as_str(), field))
        .collect();
    for edit in &request.edits {
        let field_id = edit.field_id.trim();
        let Some(field) = field_by_id.get(field_id) else {
            return Err(deny_sheet_field_edit(
                "unknown_field",
                Some(field_id),
                format!("unknown_field: parsed sheet has no field_id={field_id}"),
            ));
        };
        if field.protected {
            return Err(deny_sheet_field_edit(
                "protected_field",
                Some(field_id),
                format!("protected_field: field_id={field_id} cannot be edited"),
            ));
        }
        if !field.editable_roles.is_empty()
            && !field
                .editable_roles
                .iter()
                .any(|role| role.eq_ignore_ascii_case(&actor_role))
        {
            return Err(deny_sheet_field_edit(
                "role_scope_denied",
                Some(field_id),
                format!(
                    "role_scope_denied: actor_role={} cannot edit field_id={} allowed_roles={}",
                    actor_role,
                    field_id,
                    field.editable_roles.join(",")
                ),
            ));
        }
        if let Err(message) = normalize_replacement_for_type(
            field_id,
            &field.field_type,
            &field.allowed_special_values,
            &edit.replacement_text,
        ) {
            return Err(deny_sheet_field_edit(
                "invalid_typed_value",
                Some(field_id),
                message,
            ));
        }
    }

    Ok(())
}

fn sheet_guard_field_map(ast: &SheetTemplateAst) -> HashMap<String, &SheetTemplateField> {
    let field_by_id: HashMap<&str, &SheetTemplateField> = ast
        .fields
        .iter()
        .map(|field| (field.id.as_str(), field))
        .collect();
    let mut fields = HashMap::new();
    for field in &ast.fields {
        fields.insert(field.id.clone(), field);
    }
    for instance in &ast.block_instances {
        for instance_field in &instance.fields {
            if let Some(field) = field_by_id.get(instance_field.field_id.as_str()) {
                fields.insert(
                    format!("{}.{}", instance.instance_id, instance_field.field_id),
                    *field,
                );
            }
        }
    }
    fields
}

fn validate_sheet_guard_key(
    actor_role: &str,
    field_key: &str,
    field: &SheetTemplateField,
) -> Result<(), SheetFieldEditDenial> {
    if field.protected {
        return Err(deny_sheet_field_edit(
            "protected_field",
            Some(field_key),
            format!("protected_field: field_id={field_key} cannot be edited"),
        ));
    }
    if !field.editable_roles.is_empty()
        && !field
            .editable_roles
            .iter()
            .any(|role| role.eq_ignore_ascii_case(actor_role))
    {
        return Err(deny_sheet_field_edit(
            "role_scope_denied",
            Some(field_key),
            format!(
                "role_scope_denied: actor_role={} cannot edit field_id={} allowed_roles={}",
                actor_role,
                field_key,
                field.editable_roles.join(",")
            ),
        ));
    }
    Ok(())
}

fn validate_sheet_revert_guard(
    current_ast: &SheetTemplateAst,
    target_ast: &SheetTemplateAst,
    request: &SheetVersionRevertRequest,
    changed_field_keys: &[String],
) -> Result<(), SheetFieldEditDenial> {
    let actor_role = request.actor_role.trim().to_ascii_lowercase();
    if actor_role.is_empty() {
        return Err(deny_sheet_field_edit(
            "actor_role_required",
            None,
            "sheet revert requires a non-empty actor_role".to_string(),
        ));
    }

    let Some(expected_hash) = request.expected_template_hash.as_deref() else {
        return Err(deny_sheet_field_edit(
            "stale_selection",
            None,
            "stale_selection: expected_template_hash is required for bounded sheet revert"
                .to_string(),
        ));
    };
    if expected_hash.trim() != current_ast.template_hash {
        return Err(deny_sheet_field_edit(
            "stale_selection",
            None,
            format!(
                "stale_selection: expected template hash {} but current head is {}",
                expected_hash.trim(),
                current_ast.template_hash
            ),
        ));
    }

    let current_fields = sheet_guard_field_map(current_ast);
    let target_fields = sheet_guard_field_map(target_ast);
    for key in changed_field_keys {
        let Some(field) = current_fields.get(key).or_else(|| target_fields.get(key)) else {
            return Err(deny_sheet_field_edit(
                "unknown_field",
                Some(key),
                format!("unknown_field: parsed sheet has no field_id={key}"),
            ));
        };
        validate_sheet_guard_key(&actor_role, key, field)?;
    }
    Ok(())
}

fn full_revert_changed_field_keys(current_raw_text: &str, target_raw_text: &str) -> Vec<String> {
    let current_lines = sheet_field_line_map(current_raw_text);
    let target_lines = sheet_field_line_map(target_raw_text);
    let mut keys = current_lines
        .keys()
        .chain(target_lines.keys())
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .filter(|key| current_lines.get(key) != target_lines.get(key))
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

struct FieldTypeInference {
    field_type: ParsedSheetFieldType,
    optional: bool,
    allowed_special_values: Vec<String>,
    protected: bool,
    editable_roles: Vec<String>,
}

pub(crate) fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn source_path_fingerprint(source_path: Option<&str>) -> Option<String> {
    source_path.map(|path| format!("sha256:{}", sha256_hex(path)))
}

fn first_template_version(text: &str) -> Option<String> {
    let Some(line) = text.lines().map(str::trim).find(|line| !line.is_empty()) else {
        return Some("unknown".to_string());
    };
    let lower = line.to_ascii_lowercase();
    let Some(start) = lower.find("(v") else {
        return Some("unknown".to_string());
    };
    let version_start = start + 2;
    let Some(end) = line[version_start..].find(')') else {
        return Some("unknown".to_string());
    };
    let version = &line[version_start..version_start + end];
    if version.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
        && version.chars().any(|ch| ch.is_ascii_digit())
    {
        Some(version.to_string())
    } else {
        Some("unknown".to_string())
    }
}

fn source_lines(text: &str) -> Vec<(usize, usize, usize, String)> {
    let mut lines = Vec::new();
    let mut byte_start = 0usize;
    for (idx, raw_line) in text.split_inclusive('\n').enumerate() {
        let byte_end = byte_start + raw_line.len();
        let content = raw_line.trim_end_matches(&['\r', '\n'][..]).to_string();
        lines.push((idx + 1, byte_start, byte_start + content.len(), content));
        byte_start = byte_end;
    }
    if !text.is_empty() && !text.ends_with('\n') && lines.is_empty() {
        lines.push((1, 0, text.len(), text.to_string()));
    }
    lines
}

fn trimmed_span(byte_start: usize, byte_end: usize, line: &str) -> (usize, usize, String) {
    let trimmed = line.trim();
    let leading = line.find(trimmed).unwrap_or(0);
    let trailing = line.len().saturating_sub(leading + trimmed.len());
    (
        byte_start + leading,
        byte_end.saturating_sub(trailing),
        trimmed.to_string(),
    )
}

fn is_block_schema_header(line: &str) -> bool {
    line.ends_with("_Block")
        && line
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_section_header(line: &str) -> bool {
    let mut chars = line.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_uppercase() || first.is_ascii_digit())
        && !line.contains(':')
        && !is_block_schema_header(line)
        && line.chars().all(|ch| {
            ch.is_ascii_uppercase()
                || ch.is_ascii_digit()
                || ch.is_ascii_whitespace()
                || ch == '_'
                || ch == '-'
                || ch == '&'
                || ch == '/'
                || ch == '('
                || ch == ')'
                || ch == ','
                || ch == '.'
                || ch == ';'
                || ch == '\u{2014}'
                || ch == '\u{2013}'
        })
}

fn field_id_end(before_colon: &str) -> Option<usize> {
    let mut idx = 0usize;
    for segment_idx in 0..3 {
        let segment_start = idx;
        while let Some(ch) = before_colon[idx..].chars().next() {
            if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
                idx += ch.len_utf8();
            } else {
                break;
            }
        }
        if idx == segment_start {
            return None;
        }
        if segment_idx < 2 {
            if before_colon[idx..].starts_with('-') {
                idx += 1;
            } else {
                return None;
            }
        }
    }
    Some(idx)
}

fn split_field_line(line: &str) -> Option<(String, String, String)> {
    let colon = line.find(':')?;
    let before_colon = line[..colon].trim();
    let descriptor = line[colon + 1..].trim().to_string();
    let id_end = field_id_end(before_colon)?;
    let id = before_colon[..id_end].trim();
    let after_id = before_colon[id_end..].trim_start();
    let separator = after_id.chars().next()?;
    if !matches!(separator, '\u{2014}' | '\u{2013}' | '-') {
        return None;
    }
    let label = after_id[separator.len_utf8()..].trim();
    if !label.is_empty() {
        Some((id.to_string(), label.to_string(), descriptor))
    } else {
        None
    }
}

fn field_value_for_suggestion(raw_text: &str, target_field_id: &str) -> Option<String> {
    for line in raw_text.lines().map(str::trim) {
        let Some((field_id, _, value)) = split_field_line(line) else {
            continue;
        };
        if field_id.eq_ignore_ascii_case(target_field_id) {
            return normalize_field_suggestion_value(&value);
        }
    }
    None
}

fn normalize_field_suggestion_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 500 {
        return None;
    }
    if value.starts_with('<') && value.ends_with('>') {
        return None;
    }
    Some(value.to_owned())
}

fn validate_template_descriptor(
    field_id: &str,
    line_number: usize,
    descriptor: &str,
) -> AtelierResult<()> {
    let trimmed = descriptor.trim();
    if trimmed.is_empty() {
        return Err(super::AtelierError::Validation(format!(
            "structured_parse_error: line_number={line_number} field_id={field_id} reason=empty_descriptor"
        )));
    }
    if !trimmed.starts_with('<') {
        return Err(super::AtelierError::Validation(format!(
            "structured_parse_error: line_number={line_number} field_id={field_id} reason=descriptor_must_start_with_angle"
        )));
    }
    if !trimmed.contains('>') {
        return Err(super::AtelierError::Validation(format!(
            "structured_parse_error: line_number={line_number} field_id={field_id} reason=unclosed_descriptor"
        )));
    }
    if trimmed.matches('<').count() != trimmed.matches('>').count() {
        return Err(super::AtelierError::Validation(format!(
            "structured_parse_error: line_number={line_number} field_id={field_id} reason=unbalanced_descriptor_angles"
        )));
    }
    Ok(())
}

fn descriptor_inner(descriptor: &str) -> String {
    descriptor
        .trim()
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
        .unwrap_or(descriptor.trim())
        .trim()
        .to_string()
}

fn split_descriptor_tokens(inner: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in inner.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            '|' if depth == 0 => {
                let token = inner[start..idx].trim();
                if !token.is_empty() {
                    tokens.push(token.to_string());
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    let token = inner[start..].trim();
    if !token.is_empty() {
        tokens.push(token.to_string());
    }
    tokens
}

fn normalize_other_type(token: &str) -> Option<String> {
    let raw = token.trim();
    if !raw.to_ascii_lowercase().starts_with("other:") {
        return None;
    }
    let other = descriptor_inner(raw[6..].trim()).to_ascii_lowercase();
    if is_primary_type_keyword(&other) {
        Some(other)
    } else {
        Some("descriptor".to_string())
    }
}

fn is_special_value(token: &str) -> bool {
    matches!(token, "optional" | "unset" | "unknown" | "none")
}

fn editable_roles_from_token(token: &str) -> Option<Vec<String>> {
    let trimmed = token.trim();
    let lower = trimmed.to_ascii_lowercase();
    let role_spec = lower
        .strip_prefix("editable:")
        .or_else(|| lower.strip_prefix("editable="))
        .or_else(|| lower.strip_prefix("role:"))
        .or_else(|| lower.strip_prefix("roles:"))?;
    let roles = role_spec
        .split([',', '+', ' '])
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    Some(roles)
}

fn is_primary_type_keyword(token: &str) -> bool {
    matches!(
        token,
        "string" | "integer" | "number" | "paragraph" | "descriptor" | "score_10" | "list" | "rule"
    )
}

fn primary_field_type(token: &str, allowed_special_values: Vec<String>) -> ParsedSheetFieldType {
    match token {
        "integer" | "int" => ParsedSheetFieldType::Integer,
        "number" | "float" => ParsedSheetFieldType::Number,
        "paragraph" | "text" => ParsedSheetFieldType::Paragraph,
        "descriptor" => ParsedSheetFieldType::Descriptor,
        "rule" => ParsedSheetFieldType::Rule,
        "score_10" => ParsedSheetFieldType::Score10 {
            allowed_special_values,
        },
        "list" => ParsedSheetFieldType::List {
            item_type: "string".to_string(),
        },
        _ => ParsedSheetFieldType::String,
    }
}

fn parse_single_type(token: &str, allowed_special_values: Vec<String>) -> ParsedSheetFieldType {
    let normalized = token.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "string" | "integer" | "int" | "number" | "float" | "paragraph" | "text" | "descriptor"
        | "score_10" | "list" | "rule" => primary_field_type(&normalized, allowed_special_values),
        value if value.starts_with("list of ") => {
            let block_schema_name = token.trim()[8..].trim().to_string();
            if block_schema_name.ends_with("_Block") {
                ParsedSheetFieldType::BlockList { block_schema_name }
            } else {
                ParsedSheetFieldType::List {
                    item_type: block_schema_name,
                }
            }
        }
        value if value.starts_with("block:") => ParsedSheetFieldType::Block {
            block_schema_name: token.trim()[6..].trim().to_string(),
        },
        value if value.ends_with("_block") => ParsedSheetFieldType::Block {
            block_schema_name: token.trim().to_string(),
        },
        _ => ParsedSheetFieldType::Unknown {
            descriptor: token.trim().to_string(),
        },
    }
}

struct ParsedBlockInstanceFieldLine {
    root_field_id: String,
    instance_id: String,
    ordinal: usize,
    field_id: String,
    label: String,
    descriptor: String,
}

fn block_list_field_id_from_root_path(root_path: &str) -> Option<&str> {
    let terminal = root_path.rsplit('.').next()?.trim();
    let id_end = field_id_end(terminal)?;
    if id_end == terminal.len() {
        Some(terminal)
    } else {
        None
    }
}

fn parent_instance_id_from_root_path(root_path: &str) -> Option<&str> {
    let parent_end = root_path.rfind('.')?;
    Some(root_path[..parent_end].trim())
}

fn split_block_instance_field_line(line: &str) -> Option<ParsedBlockInstanceFieldLine> {
    let colon = line.find(':')?;
    let before_colon = line[..colon].trim();
    let descriptor = line[colon + 1..].trim().to_string();
    let close = before_colon.rfind(']')?;
    let open = before_colon[..close].rfind('[')?;
    let root_field_id = before_colon[..open].trim();
    block_list_field_id_from_root_path(root_field_id)?;
    let ordinal = before_colon[open + 1..close].parse::<usize>().ok()?;
    let member = before_colon[close + 1..]
        .trim_start()
        .strip_prefix('.')?
        .trim_start();
    let id_end = field_id_end(member)?;
    let field_id = member[..id_end].trim();
    let after_id = member[id_end..].trim_start();
    let separator = after_id.chars().next()?;
    if !matches!(separator, '\u{2014}' | '\u{2013}' | '-') {
        return None;
    }
    let label = after_id[separator.len_utf8()..].trim();
    if label.is_empty() {
        return None;
    }
    let instance_id = format!("{root_field_id}[{ordinal}]");
    Some(ParsedBlockInstanceFieldLine {
        root_field_id: root_field_id.to_string(),
        instance_id,
        ordinal,
        field_id: field_id.to_string(),
        label: label.to_string(),
        descriptor,
    })
}

fn resolve_block_list_schema_for_instance_root(
    fields: &[SheetTemplateField],
    block_instances: &[SheetBlockInstance],
    root_field_id: &str,
) -> Option<String> {
    let list_field_id = block_list_field_id_from_root_path(root_field_id)?;
    let parent_schema = parent_instance_id_from_root_path(root_field_id).and_then(|parent_id| {
        block_instances
            .iter()
            .find(|instance| instance.instance_id == parent_id)
            .map(|instance| instance.block_schema_name.as_str())
    });
    fields.iter().rev().find_map(|field| {
        if field.id != list_field_id {
            return None;
        }
        if parent_schema.is_some_and(|schema| field.block_schema_name.as_deref() != Some(schema)) {
            return None;
        }
        match &field.field_type {
            ParsedSheetFieldType::BlockList { block_schema_name } => {
                Some(block_schema_name.clone())
            }
            _ => None,
        }
    })
}

fn sheet_field_line_key(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(instance_field) = split_block_instance_field_line(trimmed) {
        return Some(format!(
            "{}.{}",
            instance_field.instance_id, instance_field.field_id
        ));
    }
    split_field_line(trimmed).map(|(field_id, _, _)| field_id)
}

fn sheet_field_line_map(raw_text: &str) -> HashMap<String, String> {
    let mut lines = HashMap::new();
    for segment in raw_text.split_inclusive('\n') {
        let (line, _) = split_line_ending(segment);
        if let Some(key) = sheet_field_line_key(line) {
            lines.insert(key, line.to_string());
        }
    }
    lines
}

fn apply_selected_revert_to_raw_text(
    current_raw_text: &str,
    target_raw_text: &str,
    selectors: &[SheetFieldSelector],
) -> AtelierResult<(String, Vec<String>)> {
    let selected_keys = validate_sheet_field_selectors(selectors)?;
    let current_lines = sheet_field_line_map(current_raw_text);
    let target_lines = sheet_field_line_map(target_raw_text);
    for key in &selected_keys {
        if !current_lines.contains_key(key) {
            return Err(super::AtelierError::Validation(format!(
                "selected sheet revert references unknown current field_id={key}"
            )));
        }
        if !target_lines.contains_key(key) {
            return Err(super::AtelierError::Validation(format!(
                "selected sheet revert references unknown target field_id={key}"
            )));
        }
    }

    let selected: HashSet<&str> = selected_keys.iter().map(String::as_str).collect();
    let mut applied = Vec::with_capacity(selected_keys.len());
    let mut output = String::with_capacity(current_raw_text.len());
    for segment in current_raw_text.split_inclusive('\n') {
        let (line, ending) = split_line_ending(segment);
        if let Some(key) = sheet_field_line_key(line) {
            if selected.contains(key.as_str()) {
                let target_line = target_lines.get(&key).ok_or_else(|| {
                    super::AtelierError::Validation(format!(
                        "selected sheet revert references unknown target field_id={key}"
                    ))
                })?;
                output.push_str(target_line);
                output.push_str(ending);
                applied.push(key);
                continue;
            }
        }
        output.push_str(segment);
    }

    if applied.len() != selected_keys.len() {
        return Err(super::AtelierError::Validation(
            "one or more selected sheet revert fields were not applied".to_string(),
        ));
    }
    if output == current_raw_text {
        return Err(super::AtelierError::Validation(
            "selected sheet revert produced no text change".to_string(),
        ));
    }
    Ok((output, applied))
}

fn is_optional_descriptor(descriptor: &str) -> bool {
    let lower = descriptor.to_ascii_lowercase();
    lower.contains("optional") || lower.contains("unset")
}

fn parse_field_type(descriptor: &str) -> FieldTypeInference {
    let optional = is_optional_descriptor(descriptor);
    let inner = descriptor_inner(descriptor);
    let raw_tokens = split_descriptor_tokens(&inner);
    let mut protected = false;
    let mut editable_roles = Vec::new();
    let mut tokens = Vec::new();
    for token in raw_tokens {
        let normalized = token.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "protected" | "read_only" | "readonly") {
            protected = true;
            continue;
        }
        if let Some(roles) = editable_roles_from_token(&token) {
            for role in roles {
                if !editable_roles.contains(&role) {
                    editable_roles.push(role);
                }
            }
            continue;
        }
        tokens.push(token);
    }

    if descriptor.trim().to_ascii_lowercase().starts_with("<rule>") {
        return FieldTypeInference {
            field_type: ParsedSheetFieldType::Rule,
            optional,
            allowed_special_values: Vec::new(),
            protected,
            editable_roles,
        };
    }

    if tokens.is_empty() {
        return FieldTypeInference {
            field_type: ParsedSheetFieldType::String,
            optional,
            allowed_special_values: Vec::new(),
            protected,
            editable_roles,
        };
    }
    if tokens.len() == 1 {
        let token = tokens[0].trim();
        let normalized = token.to_ascii_lowercase();
        if is_special_value(&normalized) {
            return FieldTypeInference {
                field_type: ParsedSheetFieldType::String,
                optional,
                allowed_special_values: vec![token.to_string()],
                protected,
                editable_roles,
            };
        }
        let field_type = match parse_single_type(token, Vec::new()) {
            ParsedSheetFieldType::Unknown { .. } => ParsedSheetFieldType::String,
            other => other,
        };
        return FieldTypeInference {
            field_type,
            optional,
            allowed_special_values: Vec::new(),
            protected,
            editable_roles,
        };
    }

    let mut allowed_special_values = Vec::new();
    let mut allow_other_type = None;
    let mut enum_values = Vec::new();
    let mut primary_variants: Vec<ParsedSheetFieldType> = Vec::new();
    let mut primary_keywords = HashSet::new();
    let mut block_type = None;

    for token in tokens {
        let normalized = token.trim().to_ascii_lowercase();
        if is_special_value(&normalized) {
            allowed_special_values.push(token.trim().to_string());
            continue;
        }
        if let Some(other_type) = normalize_other_type(token.trim()) {
            allow_other_type = Some(other_type);
            continue;
        }

        let parsed = parse_single_type(token.trim(), Vec::new());
        match parsed {
            ParsedSheetFieldType::Unknown { descriptor } => enum_values.push(descriptor),
            ParsedSheetFieldType::Block { block_schema_name } => {
                block_type = Some(ParsedSheetFieldType::Block { block_schema_name });
            }
            ParsedSheetFieldType::BlockList { block_schema_name } => {
                block_type = Some(ParsedSheetFieldType::BlockList { block_schema_name });
            }
            _ if is_primary_type_keyword(&normalized) => {
                if primary_keywords.insert(normalized.clone()) {
                    primary_variants.push(primary_field_type(&normalized, Vec::new()));
                }
            }
            other => {
                block_type = Some(other);
            }
        }
    }

    if let Some(field_type) = block_type {
        return FieldTypeInference {
            field_type,
            optional,
            allowed_special_values,
            protected,
            editable_roles,
        };
    }

    if primary_variants.len() > 1 {
        return FieldTypeInference {
            field_type: ParsedSheetFieldType::Union {
                variants: primary_variants,
                allowed_special_values: allowed_special_values.clone(),
            },
            optional,
            allowed_special_values,
            protected,
            editable_roles,
        };
    }

    if let Some(field_type) = primary_variants.into_iter().next() {
        let mut primary_allowed = allowed_special_values.clone();
        primary_allowed.extend(enum_values);
        let field_type = match field_type {
            ParsedSheetFieldType::Score10 { .. } => ParsedSheetFieldType::Score10 {
                allowed_special_values: primary_allowed.clone(),
            },
            ParsedSheetFieldType::List { .. } => ParsedSheetFieldType::List {
                item_type: "string".to_string(),
            },
            other => other,
        };
        return FieldTypeInference {
            field_type,
            optional,
            allowed_special_values: primary_allowed,
            protected,
            editable_roles,
        };
    }

    if !enum_values.is_empty() || allow_other_type.is_some() {
        return FieldTypeInference {
            field_type: ParsedSheetFieldType::Enum {
                values: enum_values,
                allow_other_type,
                allowed_special_values: allowed_special_values.clone(),
            },
            optional,
            allowed_special_values,
            protected,
            editable_roles,
        };
    }

    FieldTypeInference {
        field_type: ParsedSheetFieldType::String,
        optional,
        allowed_special_values,
        protected,
        editable_roles,
    }
}

pub(crate) fn parse_sheet_template_ast(
    raw_text: &str,
    template_id: &str,
    source_path: Option<&str>,
) -> AtelierResult<SheetTemplateAst> {
    let template_version = first_template_version(raw_text);
    let template_hash = sha256_hex(raw_text);
    let mut sections = Vec::new();
    let mut block_schemas: Vec<SheetBlockSchema> = Vec::new();
    let mut block_instances: Vec<SheetBlockInstance> = Vec::new();
    let mut fields: Vec<SheetTemplateField> = Vec::new();
    let mut unmapped_lines = Vec::new();
    let mut current_section: Option<String> = None;
    let mut current_block_schema: Option<String> = None;
    let mut header_consumed = false;

    for (line_number, byte_start, byte_end, line) in source_lines(raw_text) {
        let (_, _, trimmed) = trimmed_span(byte_start, byte_end, &line);
        if trimmed.is_empty() {
            unmapped_lines.push(SheetUnmappedLine {
                line_number,
                byte_start,
                byte_end,
                raw: line,
            });
            continue;
        }
        if !header_consumed && template_version.is_some() && trimmed.contains("(v") {
            header_consumed = true;
            continue;
        }
        header_consumed = true;

        if is_block_schema_header(&trimmed) {
            current_block_schema = Some(trimmed.clone());
            block_schemas.push(SheetBlockSchema {
                name: trimmed,
                line_number,
                fields: Vec::new(),
            });
            continue;
        }

        if is_section_header(&trimmed) {
            current_section = Some(trimmed.clone());
            current_block_schema = None;
            sections.push(SheetTemplateSection {
                name: trimmed,
                line_number,
            });
            continue;
        }

        if let Some(instance_field) = split_block_instance_field_line(&trimmed) {
            let block_schema_name = resolve_block_list_schema_for_instance_root(
                &fields,
                &block_instances,
                &instance_field.root_field_id,
            );
            if let Some(block_schema_name) = block_schema_name {
                let belongs_to_schema = block_schemas
                    .iter()
                    .find(|schema| schema.name == block_schema_name)
                    .map(|schema| schema.fields.contains(&instance_field.field_id))
                    .unwrap_or(false);
                if belongs_to_schema {
                    validate_template_descriptor(
                        &instance_field.field_id,
                        line_number,
                        &instance_field.descriptor,
                    )?;
                    let field = SheetBlockInstanceField {
                        field_id: instance_field.field_id,
                        label: instance_field.label,
                        line_number,
                        byte_start,
                        byte_end,
                        raw: line,
                        template_descriptor: instance_field.descriptor,
                    };
                    if let Some(instance) = block_instances
                        .iter_mut()
                        .find(|instance| instance.instance_id == instance_field.instance_id)
                    {
                        instance.line_end = line_number;
                        instance.fields.push(field);
                    } else {
                        block_instances.push(SheetBlockInstance {
                            instance_id: instance_field.instance_id,
                            root_field_id: instance_field.root_field_id,
                            block_schema_name,
                            ordinal: instance_field.ordinal,
                            line_start: line_number,
                            line_end: line_number,
                            fields: vec![field],
                        });
                    }
                    continue;
                }
            }
        }

        if let Some((id, label, descriptor)) = split_field_line(&trimmed) {
            validate_template_descriptor(&id, line_number, &descriptor)?;
            let inferred = parse_field_type(&descriptor);
            if let Some(block_schema_name) = &current_block_schema {
                if let Some(schema) = block_schemas
                    .iter_mut()
                    .find(|schema| &schema.name == block_schema_name)
                {
                    schema.fields.push(id.clone());
                }
            }
            fields.push(SheetTemplateField {
                id,
                label,
                field_type: inferred.field_type,
                optional: inferred.optional,
                allowed_special_values: inferred.allowed_special_values,
                section: current_section.clone(),
                block_schema_name: current_block_schema.clone(),
                line_number,
                byte_start,
                byte_end,
                raw: line,
                template_descriptor: descriptor,
                protected: inferred.protected,
                editable_roles: inferred.editable_roles,
            });
            continue;
        }

        unmapped_lines.push(SheetUnmappedLine {
            line_number,
            byte_start,
            byte_end,
            raw: line,
        });
    }

    Ok(SheetTemplateAst {
        template_id: template_id.to_string(),
        template_version,
        template_hash,
        source_path: source_path.map(ToOwned::to_owned),
        sections,
        block_schemas,
        block_instances,
        fields,
        unmapped_lines,
    })
}

impl AtelierStore {
    async fn record_sheet_field_edit_rejection(
        &self,
        source_version_id: Uuid,
        ast: &SheetTemplateAst,
        request: &SheetFieldEditRequest,
        denial: &SheetFieldEditDenial,
    ) -> AtelierResult<()> {
        let attempted_field_ids = request.edits.iter().map(sheet_edit_key).collect::<Vec<_>>();
        self.record_event(
            event_family::SHEET_FIELD_EDIT_REJECTED,
            "atelier_sheet_version",
            &source_version_id.to_string(),
            serde_json::json!({
                "source_version_id": source_version_id,
                "template_id": &request.template_id,
                "template_hash": &ast.template_hash,
                "expected_template_hash": &request.expected_template_hash,
                "source_path_ref": source_path_fingerprint(request.source_path.as_deref()),
                "actor_role": &request.actor_role,
                "reason_code": denial.reason_code,
                "field_id": &denial.field_id,
                "attempted_field_ids": attempted_field_ids,
            }),
        )
        .await
    }

    async fn record_sheet_revert_rejection(
        &self,
        source_version_id: Uuid,
        ast: &SheetTemplateAst,
        request: &SheetVersionRevertRequest,
        denial: &SheetFieldEditDenial,
        attempted_field_ids: &[String],
    ) -> AtelierResult<()> {
        self.record_event(
            event_family::SHEET_FIELD_EDIT_REJECTED,
            "atelier_sheet_version",
            &source_version_id.to_string(),
            serde_json::json!({
                "operation": "sheet_revert",
                "source_version_id": source_version_id,
                "target_version_id": request.target_version_id,
                "template_id": &request.template_id,
                "template_hash": &ast.template_hash,
                "expected_template_hash": &request.expected_template_hash,
                "source_path_ref": source_path_fingerprint(request.source_path.as_deref()),
                "actor_role": &request.actor_role,
                "reason_code": denial.reason_code,
                "field_id": &denial.field_id,
                "attempted_field_ids": attempted_field_ids,
            }),
        )
        .await
    }

    async fn record_sheet_version_conflict(
        &self,
        new: &NewSheetVersion,
        expected_parent_version_id: Option<Uuid>,
        current_parent_version_id: Option<Uuid>,
    ) -> AtelierResult<()> {
        self.record_event(
            event_family::SHEET_VERSION_CONFLICT,
            "atelier_sheet_version",
            &format!("{}:conflict", character_ref(new.character_internal_id)),
            serde_json::json!({
                "reason_code": "stale_sheet_version",
                "character_internal_id": new.character_internal_id,
                "character_ref": character_ref(new.character_internal_id),
                "expected_parent_version_id": expected_parent_version_id,
                "expected_parent_sheet_version_ref": expected_parent_version_id
                    .map(|id| sheet_version_ref(new.character_internal_id, id)),
                "expected_sheet_version_ref": expected_parent_version_id
                    .map(|id| sheet_version_ref(new.character_internal_id, id)),
                "current_head_version_id": current_parent_version_id,
                "current_head_sheet_version_ref": current_parent_version_id
                    .map(|id| sheet_version_ref(new.character_internal_id, id)),
                "current_parent_version_id": current_parent_version_id,
                "current_sheet_version_ref": current_parent_version_id
                    .map(|id| sheet_version_ref(new.character_internal_id, id)),
                "author": &new.author,
                "tool": &new.tool,
            }),
        )
        .await
    }

    /// Append a new sheet version. Computes the next sequence number and links
    /// to the previous head as parent; never overwrites an existing version.
    pub async fn append_sheet_version(&self, new: &NewSheetVersion) -> AtelierResult<SheetVersion> {
        self.append_sheet_version_with_guard(new, None).await
    }

    /// Append a new sheet version only if the caller's selected base version is
    /// still the current head. `expected_parent_version_id = None` means the
    /// caller expects this to be the first sheet version for the character.
    pub async fn append_sheet_version_if_current(
        &self,
        new: &NewSheetVersion,
        expected_parent_version_id: Option<Uuid>,
    ) -> AtelierResult<SheetVersion> {
        self.append_sheet_version_with_guard(new, Some(expected_parent_version_id))
            .await
    }

    async fn append_sheet_version_with_guard(
        &self,
        new: &NewSheetVersion,
        expected_parent_version_id: Option<Option<Uuid>>,
    ) -> AtelierResult<SheetVersion> {
        let mut tx = self.pool().begin().await?;
        let seq_lock_key = format!("atelier_sheet_version_seq:{}", new.character_internal_id);
        sqlx::query(
            "SELECT pg_advisory_xact_lock(('x' || substr(md5($1), 1, 16))::bit(64)::bigint)",
        )
        .bind(&seq_lock_key)
        .execute(&mut *tx)
        .await?;

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_sheet_version WHERE character_internal_id = $1",
        )
        .bind(new.character_internal_id)
        .fetch_one(&mut *tx)
        .await?;

        let parent_version_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT version_id FROM atelier_sheet_version WHERE character_internal_id = $1 ORDER BY seq DESC LIMIT 1",
        )
        .bind(new.character_internal_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(expected_parent_version_id) = expected_parent_version_id {
            if parent_version_id != expected_parent_version_id {
                tx.rollback().await?;
                self.record_sheet_version_conflict(
                    new,
                    expected_parent_version_id,
                    parent_version_id,
                )
                .await?;
                return Err(super::AtelierError::Conflict(format!(
                    "stale_sheet_version: expected parent {:?}, current head {:?}",
                    expected_parent_version_id, parent_version_id
                )));
            }
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_sheet_version
                 (character_internal_id, parent_version_id, seq, raw_text, author, tool)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING version_id, character_internal_id, parent_version_id, seq,
                         raw_text, author, tool, created_at_utc"#,
        )
        .bind(new.character_internal_id)
        .bind(parent_version_id)
        .bind(next_seq)
        .bind(&new.raw_text)
        .bind(&new.author)
        .bind(&new.tool)
        .fetch_one(&mut *tx)
        .await?;

        let version = version_from_row(&row);
        if let Err(err) = self
            .record_event_in_tx(
                &mut tx,
                event_family::SHEET_VERSION_APPENDED,
                "atelier_sheet_version",
                &version.version_id.to_string(),
                serde_json::json!({
                    "version_id": version.version_id,
                    "seq": version.seq,
                }),
            )
            .await
        {
            tx.rollback().await?;
            return Err(err);
        }
        tx.commit().await?;
        Ok(version)
    }

    /// Fetch one sheet version by id.
    pub async fn get_sheet_version(&self, version_id: Uuid) -> AtelierResult<SheetVersion> {
        let row = sqlx::query(
            r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                      raw_text, author, tool, created_at_utc
               FROM atelier_sheet_version
               WHERE version_id = $1"#,
        )
        .bind(version_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| {
            super::AtelierError::NotFound(format!("sheet version version_id={version_id}"))
        })?;
        Ok(version_from_row(&row))
    }

    /// The current (highest-seq) sheet version for a character, if any.
    pub async fn latest_sheet_version(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Option<SheetVersion>> {
        let row = sqlx::query(
            r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                      raw_text, author, tool, created_at_utc
               FROM atelier_sheet_version
               WHERE character_internal_id = $1
               ORDER BY seq DESC LIMIT 1"#,
        )
        .bind(character_internal_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(version_from_row))
    }

    /// Full append-only version history (ascending sequence).
    pub async fn sheet_version_history(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Vec<SheetVersion>> {
        let rows = sqlx::query(
            r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                      raw_text, author, tool, created_at_utc
               FROM atelier_sheet_version
               WHERE character_internal_id = $1
               ORDER BY seq ASC"#,
        )
        .bind(character_internal_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(version_from_row).collect())
    }

    /// Recent distinct values for one parsed Field ID. This mirrors original
    /// CKC field memory: suggestions are exact-field scoped, never auto-filled,
    /// and ignore template placeholder descriptors such as `<string>`.
    pub async fn sheet_field_suggestions(
        &self,
        field_id: &str,
        limit: i64,
    ) -> AtelierResult<Vec<SheetFieldSuggestion>> {
        let field_id = field_id.trim();
        if field_id.is_empty() {
            return Err(super::AtelierError::Validation(
                "field_id must not be empty".to_string(),
            ));
        }
        let limit = limit.clamp(1, 50) as usize;
        let rows = sqlx::query(
            r#"SELECT version_id, character_internal_id, raw_text
               FROM atelier_sheet_version
               ORDER BY created_at_utc DESC, seq DESC
               LIMIT 2000"#,
        )
        .fetch_all(self.pool())
        .await?;

        let mut suggestions = Vec::<SheetFieldSuggestion>::new();
        let mut value_index = HashMap::<String, usize>::new();
        for row in rows {
            let raw_text: String = row.get("raw_text");
            let Some(value) = field_value_for_suggestion(&raw_text, field_id) else {
                continue;
            };
            if let Some(index) = value_index.get(&value).copied() {
                suggestions[index].occurrences += 1;
                continue;
            }
            let version_id: Uuid = row.get("version_id");
            let character_internal_id: Uuid = row.get("character_internal_id");
            value_index.insert(value.clone(), suggestions.len());
            suggestions.push(SheetFieldSuggestion {
                field_id: field_id.to_string(),
                value,
                occurrences: 1,
                latest_version_id: version_id,
                latest_character_internal_id: character_internal_id,
                latest_sheet_version_ref: sheet_version_ref(character_internal_id, version_id),
                latest_character_ref: character_ref(character_internal_id),
            });
        }
        suggestions.truncate(limit);
        Ok(suggestions)
    }

    /// Parse one append-only sheet version into a typed template AST snapshot.
    ///
    /// The raw sheet remains in `atelier_sheet_version`; parse output is a
    /// separate durable snapshot so later selective apply/revert and block-list
    /// work can reference typed fields without mutating the original version.
    pub async fn parse_sheet_template_version(
        &self,
        version_id: Uuid,
        template_id: &str,
        source_path: Option<&str>,
    ) -> AtelierResult<ParsedSheetTemplate> {
        if let Some(source_path) = source_path {
            reject_legacy_runtime_ref("source_path", source_path)?;
        }
        let raw_text: String =
            sqlx::query_scalar("SELECT raw_text FROM atelier_sheet_version WHERE version_id = $1")
                .bind(version_id)
                .fetch_optional(self.pool())
                .await?
                .ok_or_else(|| {
                    super::AtelierError::NotFound(format!(
                        "sheet version not found for parse: {version_id}"
                    ))
                })?;
        let ast = parse_sheet_template_ast(&raw_text, template_id, source_path)?;
        let ast_json = serde_json::to_value(&ast)
            .map_err(|err| super::AtelierError::Validation(err.to_string()))?;
        let unmapped_json = serde_json::to_value(&ast.unmapped_lines)
            .map_err(|err| super::AtelierError::Validation(err.to_string()))?;

        let mut tx = self.pool().begin().await?;
        let parse_lock_key = format!(
            "atelier_sheet_parse_snapshot:{version_id}:{template_id}:{}",
            ast.template_hash
        );
        sqlx::query(
            "SELECT pg_advisory_xact_lock(('x' || substr(md5($1), 1, 16))::bit(64)::bigint)",
        )
        .bind(&parse_lock_key)
        .execute(&mut *tx)
        .await?;

        let existing_row = sqlx::query(
            r#"SELECT parse_id, created_at_utc
               FROM atelier_sheet_parse_snapshot
               WHERE version_id = $1
                 AND template_id = $2
                 AND template_hash = $3"#,
        )
        .bind(version_id)
        .bind(template_id)
        .bind(&ast.template_hash)
        .fetch_optional(&mut *tx)
        .await?;
        let row = match existing_row {
            Some(row) => row,
            None => {
                sqlx::query(
                    r#"INSERT INTO atelier_sheet_parse_snapshot
                           (parse_id, version_id, template_id, source_path, template_version,
                            template_hash, ast, unmapped_lines)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                       RETURNING parse_id, created_at_utc"#,
                )
                .bind(Uuid::now_v7())
                .bind(version_id)
                .bind(template_id)
                .bind(source_path)
                .bind(&ast.template_version)
                .bind(&ast.template_hash)
                .bind(ast_json)
                .bind(unmapped_json)
                .fetch_one(&mut *tx)
                .await?
            }
        };
        let parse_id: Uuid = row.get("parse_id");
        let created_at_utc: DateTime<Utc> = row.get("created_at_utc");

        let event_result = self
            .record_event_in_tx(
                &mut tx,
                event_family::SHEET_TEMPLATE_PARSED,
                "atelier_sheet_version",
                &version_id.to_string(),
                serde_json::json!({
                    "parse_id": parse_id,
                    "version_id": version_id,
                    "template_id": template_id,
                    "template_version": &ast.template_version,
                    "template_hash": &ast.template_hash,
                    "field_count": ast.fields.len(),
                    "block_schema_count": ast.block_schemas.len(),
                    "block_instance_count": ast.block_instances.len(),
                    "section_count": ast.sections.len(),
                    "unmapped_count": ast.unmapped_lines.len(),
                }),
            )
            .await;
        if let Err(err) = event_result {
            let _ = tx.rollback().await;
            return Err(err);
        }
        tx.commit().await?;

        Ok(ParsedSheetTemplate {
            parse_id,
            version_id,
            template_id: template_id.to_string(),
            source_path: source_path.map(ToOwned::to_owned),
            ast,
            created_at_utc,
        })
    }

    /// Apply one or more parsed field-level edits as a new append-only sheet
    /// version. The source version remains immutable; unmapped source text is
    /// copied through byte-for-byte because only matched field descriptor lines
    /// are rewritten.
    pub async fn apply_sheet_field_edits(
        &self,
        request: &SheetFieldEditRequest,
    ) -> AtelierResult<SheetFieldEditResult> {
        let source_row = sqlx::query(
            r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                      raw_text, author, tool, created_at_utc
               FROM atelier_sheet_version
               WHERE version_id = $1"#,
        )
        .bind(request.version_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| {
            super::AtelierError::NotFound(format!(
                "source sheet version not found for selective apply: {}",
                request.version_id
            ))
        })?;
        let source = version_from_row(&source_row);
        if let Some(source_path) = request.source_path.as_deref() {
            reject_legacy_runtime_ref("source_path", source_path)?;
        }
        let ast = parse_sheet_template_ast(
            &source.raw_text,
            &request.template_id,
            request.source_path.as_deref(),
        )?;
        if let Err(err) = validate_sheet_field_edits(&request.edits) {
            if let super::AtelierError::Validation(message) = err {
                let denial = deny_sheet_field_edit("bounded_apply_denied", None, message);
                self.record_sheet_field_edit_rejection(source.version_id, &ast, request, &denial)
                    .await?;
                return Err(super::AtelierError::Validation(denial.message));
            }
            return Err(err);
        }
        if let Err(denial) = validate_sheet_field_edit_guard(&ast, request) {
            self.record_sheet_field_edit_rejection(source.version_id, &ast, request, &denial)
                .await?;
            return Err(super::AtelierError::Validation(denial.message));
        }
        let (updated_raw_text, applied_field_ids) =
            match apply_field_edits_to_raw_text(&source.raw_text, &ast, &request.edits) {
                Ok(applied) => applied,
                Err(super::AtelierError::Validation(message)) => {
                    let denial = deny_sheet_field_edit("bounded_apply_denied", None, message);
                    self.record_sheet_field_edit_rejection(
                        source.version_id,
                        &ast,
                        request,
                        &denial,
                    )
                    .await?;
                    return Err(super::AtelierError::Validation(denial.message));
                }
                Err(err) => return Err(err),
            };

        let mut tx = self.pool().begin().await?;
        let seq_lock_key = format!("atelier_sheet_version_seq:{}", source.character_internal_id);
        sqlx::query(
            "SELECT pg_advisory_xact_lock(('x' || substr(md5($1), 1, 16))::bit(64)::bigint)",
        )
        .bind(&seq_lock_key)
        .execute(&mut *tx)
        .await?;

        let current_head_version_id: Uuid = sqlx::query_scalar(
            "SELECT version_id FROM atelier_sheet_version WHERE character_internal_id = $1 ORDER BY seq DESC LIMIT 1",
        )
        .bind(source.character_internal_id)
        .fetch_one(&mut *tx)
        .await?;
        if current_head_version_id != source.version_id {
            tx.rollback().await?;
            let denial = deny_sheet_field_edit(
                "stale_selection",
                None,
                format!(
                    "stale_selection: source version {} is not the current head {}",
                    source.version_id, current_head_version_id
                ),
            );
            let message = denial.message.clone();
            self.record_sheet_field_edit_rejection(source.version_id, &ast, request, &denial)
                .await?;
            return Err(super::AtelierError::Validation(message));
        }

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_sheet_version WHERE character_internal_id = $1",
        )
        .bind(source.character_internal_id)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_sheet_version
                 (version_id, character_internal_id, parent_version_id, seq, raw_text, author, tool)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING version_id, character_internal_id, parent_version_id, seq,
                         raw_text, author, tool, created_at_utc"#,
        )
        .bind(Uuid::now_v7())
        .bind(source.character_internal_id)
        .bind(source.version_id)
        .bind(next_seq)
        .bind(&updated_raw_text)
        .bind(&request.author)
        .bind(&request.tool)
        .fetch_one(&mut *tx)
        .await?;
        let version = version_from_row(&row);

        let event_result = self
            .record_event_in_tx(
                &mut tx,
                event_family::SHEET_FIELD_EDITS_APPLIED,
                "atelier_sheet_version",
                &version.version_id.to_string(),
                serde_json::json!({
                    "new_version_id": version.version_id,
                    "source_version_id": source.version_id,
                    "template_id": &request.template_id,
                    "template_hash": &ast.template_hash,
                    "actor_role": &request.actor_role,
                    "applied_field_ids": &applied_field_ids,
                    "preserved_unmapped_count": ast.unmapped_lines.len(),
                }),
            )
            .await;
        if let Err(err) = event_result {
            let _ = tx.rollback().await;
            return Err(err);
        }
        tx.commit().await?;

        Ok(SheetFieldEditResult {
            version,
            source_version_id: source.version_id,
            applied_field_ids,
            preserved_unmapped_lines: ast.unmapped_lines,
        })
    }

    /// Apply multiple parsed field-edit requests as one all-or-nothing batch.
    ///
    /// Every source version, template hash, field id, and role guard is checked
    /// before the transaction begins to append new sheet versions. If any
    /// target is invalid, no version, receipt, or EventLedger row is written.
    pub async fn bulk_apply_sheet_field_edits(
        &self,
        requests: &[SheetFieldEditRequest],
        requested_by: &str,
    ) -> AtelierResult<BulkSheetFieldEditResult> {
        let requested_by = requested_by.trim();
        if requested_by.is_empty() {
            return Err(super::AtelierError::Validation(
                "requested_by must not be empty".into(),
            ));
        }
        if requests.is_empty() {
            return Err(super::AtelierError::Validation(
                "bulk sheet field edits require at least one request".into(),
            ));
        }

        struct FieldEditPlan {
            source: SheetVersion,
            request: SheetFieldEditRequest,
            updated_raw_text: String,
            applied_field_ids: Vec<String>,
            preserved_unmapped_lines: Vec<SheetUnmappedLine>,
            template_hash: String,
        }

        let mut plans = Vec::with_capacity(requests.len());
        for request in requests {
            let source_row = sqlx::query(
                r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                          raw_text, author, tool, created_at_utc
                   FROM atelier_sheet_version
                   WHERE version_id = $1"#,
            )
            .bind(request.version_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or_else(|| {
                super::AtelierError::NotFound(format!(
                    "source sheet version not found for bulk selective apply: {}",
                    request.version_id
                ))
            })?;
            let source = version_from_row(&source_row);
            if let Some(source_path) = request.source_path.as_deref() {
                reject_legacy_runtime_ref("source_path", source_path)?;
            }
            let ast = parse_sheet_template_ast(
                &source.raw_text,
                &request.template_id,
                request.source_path.as_deref(),
            )?;
            if let Err(err) = validate_sheet_field_edits(&request.edits) {
                if let super::AtelierError::Validation(message) = err {
                    let denial = deny_sheet_field_edit("bounded_apply_denied", None, message);
                    return Err(super::AtelierError::Validation(denial.message));
                }
                return Err(err);
            }
            if let Err(denial) = validate_sheet_field_edit_guard(&ast, request) {
                return Err(super::AtelierError::Validation(denial.message));
            }
            let (updated_raw_text, applied_field_ids) =
                match apply_field_edits_to_raw_text(&source.raw_text, &ast, &request.edits) {
                    Ok(applied) => applied,
                    Err(super::AtelierError::Validation(message)) => {
                        let denial = deny_sheet_field_edit("bounded_apply_denied", None, message);
                        return Err(super::AtelierError::Validation(denial.message));
                    }
                    Err(err) => return Err(err),
                };
            plans.push(FieldEditPlan {
                source,
                request: request.clone(),
                updated_raw_text,
                applied_field_ids,
                preserved_unmapped_lines: ast.unmapped_lines,
                template_hash: ast.template_hash,
            });
        }

        let mut tx = self.pool().begin().await?;
        let mut character_ids: Vec<Uuid> = plans
            .iter()
            .map(|plan| plan.source.character_internal_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        character_ids.sort();
        for character_id in &character_ids {
            let seq_lock_key = format!("atelier_sheet_version_seq:{character_id}");
            sqlx::query(
                "SELECT pg_advisory_xact_lock(('x' || substr(md5($1), 1, 16))::bit(64)::bigint)",
            )
            .bind(&seq_lock_key)
            .execute(&mut *tx)
            .await?;
        }

        let mut seen_bulk_characters = HashSet::new();
        for plan in &plans {
            if !seen_bulk_characters.insert(plan.source.character_internal_id) {
                return Err(super::AtelierError::Validation(format!(
                    "stale_selection: bulk sheet field edits require one current-head request per character; duplicate character_internal_id={}",
                    plan.source.character_internal_id
                )));
            }

            let current_head_version_id: Uuid = sqlx::query_scalar(
                "SELECT version_id FROM atelier_sheet_version WHERE character_internal_id = $1 ORDER BY seq DESC LIMIT 1",
            )
            .bind(plan.source.character_internal_id)
            .fetch_one(&mut *tx)
            .await?;
            if current_head_version_id != plan.source.version_id {
                return Err(super::AtelierError::Validation(format!(
                    "stale_selection: source version {} is not the current head {}",
                    plan.source.version_id, current_head_version_id
                )));
            }
        }

        let mut results = Vec::with_capacity(plans.len());
        for plan in plans {
            let next_seq: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_sheet_version WHERE character_internal_id = $1",
            )
            .bind(plan.source.character_internal_id)
            .fetch_one(&mut *tx)
            .await?;

            let row = sqlx::query(
                r#"INSERT INTO atelier_sheet_version
                     (version_id, character_internal_id, parent_version_id, seq, raw_text, author, tool)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)
                   RETURNING version_id, character_internal_id, parent_version_id, seq,
                             raw_text, author, tool, created_at_utc"#,
            )
            .bind(Uuid::now_v7())
            .bind(plan.source.character_internal_id)
            .bind(plan.source.version_id)
            .bind(next_seq)
            .bind(&plan.updated_raw_text)
            .bind(&plan.request.author)
            .bind(&plan.request.tool)
            .fetch_one(&mut *tx)
            .await?;
            let version = version_from_row(&row);
            self.record_event_in_tx(
                &mut tx,
                event_family::SHEET_FIELD_EDITS_APPLIED,
                "atelier_sheet_version",
                &version.version_id.to_string(),
                serde_json::json!({
                    "new_version_id": version.version_id,
                    "source_version_id": plan.source.version_id,
                    "template_id": &plan.request.template_id,
                    "template_hash": &plan.template_hash,
                    "actor_role": &plan.request.actor_role,
                    "applied_field_ids": &plan.applied_field_ids,
                    "preserved_unmapped_count": plan.preserved_unmapped_lines.len(),
                    "bulk": true,
                }),
            )
            .await?;

            results.push(SheetFieldEditResult {
                version,
                source_version_id: plan.source.version_id,
                applied_field_ids: plan.applied_field_ids,
                preserved_unmapped_lines: plan.preserved_unmapped_lines,
            });
        }

        let receipt = self
            .record_bulk_operation_receipt_in_tx(
                &mut tx,
                "bulk_apply_sheet_field_edits",
                requested_by,
                requests.len() as i64,
                results.len() as i64,
                serde_json::json!({
                    "source_version_ids": results
                        .iter()
                        .map(|result| result.source_version_id)
                        .collect::<Vec<_>>(),
                    "new_version_ids": results
                        .iter()
                        .map(|result| result.version.version_id)
                        .collect::<Vec<_>>(),
                    "character_count": character_ids.len(),
                }),
            )
            .await?;
        tx.commit().await?;

        Ok(BulkSheetFieldEditResult { receipt, results })
    }

    /// Revert a prior sheet version by copying its raw text into a new
    /// append-only version. This never rewrites or deletes the versions between
    /// the current head and the chosen target.
    pub async fn revert_sheet_version_as_new(
        &self,
        request: &SheetVersionRevertRequest,
    ) -> AtelierResult<SheetVersionRevertResult> {
        if let Some(source_path) = request.source_path.as_deref() {
            reject_legacy_runtime_ref("source_path", source_path)?;
        }
        let mut tx = self.pool().begin().await?;
        let seq_lock_key = format!(
            "atelier_sheet_version_seq:{}",
            request.character_internal_id
        );
        sqlx::query(
            "SELECT pg_advisory_xact_lock(('x' || substr(md5($1), 1, 16))::bit(64)::bigint)",
        )
        .bind(&seq_lock_key)
        .execute(&mut *tx)
        .await?;

        let target_row = sqlx::query(
            r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                      raw_text, author, tool, created_at_utc
               FROM atelier_sheet_version
               WHERE version_id = $1
                 AND character_internal_id = $2"#,
        )
        .bind(request.target_version_id)
        .bind(request.character_internal_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            super::AtelierError::NotFound(format!(
                "target sheet version not found for revert: {}",
                request.target_version_id
            ))
        })?;
        let target = version_from_row(&target_row);

        let previous_head_row = sqlx::query(
            r#"SELECT version_id, character_internal_id, parent_version_id, seq,
                      raw_text, author, tool, created_at_utc
               FROM atelier_sheet_version
               WHERE character_internal_id = $1
               ORDER BY seq DESC
               LIMIT 1"#,
        )
        .bind(request.character_internal_id)
        .fetch_one(&mut *tx)
        .await?;
        let previous_head = version_from_row(&previous_head_row);
        let previous_head_version_id = previous_head.version_id;

        let current_ast = parse_sheet_template_ast(
            &previous_head.raw_text,
            &request.template_id,
            request.source_path.as_deref(),
        )?;
        let target_ast = parse_sheet_template_ast(
            &target.raw_text,
            &request.template_id,
            request.source_path.as_deref(),
        )?;
        let guarded_revert_field_ids = if request.field_selectors.is_empty() {
            full_revert_changed_field_keys(&previous_head.raw_text, &target.raw_text)
        } else {
            validate_sheet_field_selectors(&request.field_selectors)?
        };
        if let Err(denial) = validate_sheet_revert_guard(
            &current_ast,
            &target_ast,
            request,
            &guarded_revert_field_ids,
        ) {
            let _ = tx.rollback().await;
            self.record_sheet_revert_rejection(
                previous_head_version_id,
                &current_ast,
                request,
                &denial,
                &guarded_revert_field_ids,
            )
            .await?;
            return Err(super::AtelierError::Validation(denial.message));
        }

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_sheet_version WHERE character_internal_id = $1",
        )
        .bind(request.character_internal_id)
        .fetch_one(&mut *tx)
        .await?;

        let (reverted_raw_text, reverted_field_ids, revert_scope) =
            if request.field_selectors.is_empty() {
                (target.raw_text.clone(), Vec::new(), "full_version")
            } else {
                let (raw_text, field_ids) = apply_selected_revert_to_raw_text(
                    &previous_head.raw_text,
                    &target.raw_text,
                    &request.field_selectors,
                )?;
                (raw_text, field_ids, "selected_fields")
            };

        let row = sqlx::query(
            r#"INSERT INTO atelier_sheet_version
                 (version_id, character_internal_id, parent_version_id, seq, raw_text, author, tool)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING version_id, character_internal_id, parent_version_id, seq,
                         raw_text, author, tool, created_at_utc"#,
        )
        .bind(Uuid::now_v7())
        .bind(request.character_internal_id)
        .bind(previous_head_version_id)
        .bind(next_seq)
        .bind(&reverted_raw_text)
        .bind(&request.author)
        .bind(&request.tool)
        .fetch_one(&mut *tx)
        .await?;
        let version = version_from_row(&row);

        let event_result = self
            .record_event_in_tx(
                &mut tx,
                event_family::SHEET_VERSION_REVERTED,
                "atelier_sheet_version",
                &version.version_id.to_string(),
                serde_json::json!({
                    "new_version_id": version.version_id,
                    "previous_head_version_id": previous_head_version_id,
                    "reverted_to_version_id": target.version_id,
                    "revert_scope": revert_scope,
                    "reverted_field_ids": &reverted_field_ids,
                    "target_seq": target.seq,
                    "new_seq": version.seq,
                }),
            )
            .await;
        if let Err(err) = event_result {
            let _ = tx.rollback().await;
            return Err(err);
        }
        tx.commit().await?;

        Ok(SheetVersionRevertResult {
            version,
            reverted_to_version_id: target.version_id,
            previous_head_version_id,
        })
    }
}
