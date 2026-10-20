//! Append-only character sheet versions (MT-012): updates never mutate prior
//! versions; each change is a new version with parent linkage and provenance,
//! preventing silent data loss when models or imports edit a sheet.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

pub use handshake_ckc_sheet_parser::{
    SheetTemplateAst, SheetTemplateSection, SheetBlockSchema, SheetBlockInstance,
    SheetBlockInstanceField, SheetTemplateField, SheetUnmappedLine, ParsedSheetFieldType,
    SheetFieldEdit, SheetFieldSelector,
};
pub(crate) use handshake_ckc_sheet_parser::{
    parse_sheet_template_ast, sha256_hex, sheet_field_id_from_line, sheet_line_looks_like_field,
};
use handshake_ckc_sheet_parser::{
    normalize_replacement_for_type, sheet_edit_key, split_field_line,
    split_block_instance_field_line,
};

use super::refs::{character_ref, sheet_version_ref};
use super::{
    atelier_event_sql, event_family, reject_legacy_runtime_ref, AtelierError, AtelierResult,
    AtelierStore, BulkOperationReceipt, RecordEventBindings, RecordedLedgerRow,
};

/// Event families owned by the sheet module that the WP-CKC port adds on top of
/// the MT-005 set in [`super::event_family`]. Listed here (pattern:
/// `collections_event_family`) so `event_family::ALL` can be extended by the
/// module owner without this file editing `mod.rs`.
pub mod sheet_event_family {
    /// A guarded sheet append was refused because the caller's expected head
    /// was no longer the current head (WP-CKC MT-009).
    pub const SHEET_VERSION_CONFLICT: &str = "atelier.sheet.version_conflict";

    pub const ALL: &[&str] = &[SHEET_VERSION_CONFLICT];
}

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

/// One remembered value for a CKC Field ID, aggregated over every sheet version
/// that carried it (MT-009 field memory). `latest_*` point at the newest
/// version that used the value.
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

#[derive(SurrealValue)]
struct SheetVersionRow {
    version_id: SurrealUuid,
    character_internal_id: SurrealUuid,
    parent_version_id: Option<SurrealUuid>,
    seq: i64,
    raw_text: String,
    author: String,
    tool: Option<String>,
    created_at_utc: Datetime,
}

fn version_from_row(row: SheetVersionRow) -> SheetVersion {
    SheetVersion {
        version_id: row.version_id.into(),
        character_internal_id: row.character_internal_id.into(),
        parent_version_id: row.parent_version_id.map(Into::into),
        seq: row.seq,
        raw_text: row.raw_text,
        author: row.author,
        tool: row.tool,
        created_at_utc: row.created_at_utc.into(),
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

fn validate_sheet_field_selectors(selectors: &[SheetFieldSelector]) -> AtelierResult<Vec<String>> {
    handshake_ckc_sheet_parser::validate_sheet_field_selectors(selectors)
        .map_err(AtelierError::Validation)
}

fn validate_sheet_field_edits(edits: &[SheetFieldEdit]) -> AtelierResult<()> {
    handshake_ckc_sheet_parser::validate_sheet_field_edits(edits)
        .map_err(AtelierError::Validation)
}

fn apply_field_edits_to_raw_text(
    raw_text: &str,
    ast: &SheetTemplateAst,
    edits: &[SheetFieldEdit],
) -> AtelierResult<(String, Vec<String>)> {
    handshake_ckc_sheet_parser::apply_field_edits_to_raw_text(raw_text, ast, edits)
        .map_err(AtelierError::Validation)
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

fn source_path_fingerprint(source_path: Option<&str>) -> Option<String> {
    source_path.map(|path| format!("sha256:{}", sha256_hex(path)))
}

/// The first normalised value written for `target_field_id` in a raw sheet.
pub(crate) fn sheet_field_value(raw_text: &str, target_field_id: &str) -> Option<String> {
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

/// Every normalised value written for `target_field_id` in a raw sheet, in
/// line order (duplicates kept so ownership checks can count them).
pub(crate) fn sheet_field_values(raw_text: &str, target_field_id: &str) -> Vec<String> {
    let mut values = Vec::new();
    for line in raw_text.lines().map(str::trim) {
        let Some((field_id, _, value)) = split_field_line(line) else {
            continue;
        };
        if !field_id.eq_ignore_ascii_case(target_field_id) {
            continue;
        }
        if let Some(value) = normalize_field_suggestion_value(&value) {
            values.push(value);
        }
    }
    values
}

/// Distinct `(FIELD_ID, value)` pairs of one raw sheet, in first-seen order:
/// the rows the field-value projection stores for a new sheet version.
fn sheet_field_values_for_projection(raw_text: &str) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut values = Vec::new();
    for line in raw_text.lines().map(str::trim) {
        let Some((field_id, _, value)) = split_field_line(line) else {
            continue;
        };
        let Some(value) = normalize_field_suggestion_value(&value) else {
            continue;
        };
        let field_id = field_id.to_ascii_uppercase();
        if seen.insert((field_id.clone(), value.clone())) {
            values.push((field_id, value));
        }
    }
    values
}

/// Suggestion values are trimmed, bounded, and never a template placeholder
/// such as `<string>`.
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

#[derive(Clone, SurrealValue)]
struct CharacterSheetBinding {
    character: RecordId,
}

#[derive(Clone, SurrealValue)]
struct SheetVersionBinding {
    version_id: SurrealUuid,
}

/// One `atelier_sheet_field_value_projection` row written alongside a new
/// sheet version (WP-CKC MT-009). The version and character links are taken
/// from the enclosing append bindings, so a row cannot point at another sheet.
#[derive(Clone, SurrealValue)]
struct SheetFieldProjectionRow {
    record: RecordId,
    projection_id: SurrealUuid,
    field_id: String,
    value: String,
}

fn sheet_field_projection_rows(raw_text: &str) -> Vec<SheetFieldProjectionRow> {
    sheet_field_values_for_projection(raw_text)
        .into_iter()
        .map(|(field_id, value)| {
            let projection_id = Uuid::now_v7();
            SheetFieldProjectionRow {
                record: RecordId::new(
                    "atelier_sheet_field_value_projection",
                    SurrealUuid::from(projection_id),
                ),
                projection_id: projection_id.into(),
                field_id,
                value,
            }
        })
        .collect()
}

#[derive(Clone, SurrealValue)]
struct SheetAppendBindings {
    character: RecordId,
    version_record: RecordId,
    version_id: SurrealUuid,
    expected_head: Option<RecordId>,
    enforce_head: bool,
    raw_text: String,
    author: String,
    tool: Option<String>,
    projection_rows: Vec<SheetFieldProjectionRow>,
}

#[derive(SurrealValue)]
struct FieldIdBindings {
    field_id: String,
}

/// One `atelier_sheet_field_value_projection` row as the suggestion read
/// returns it (record links already reduced to their uuid keys).
#[derive(SurrealValue)]
struct SheetFieldProjectionReadRow {
    value: String,
    character_internal_id: SurrealUuid,
    sheet_version_id: SurrealUuid,
}

#[derive(Clone, SurrealValue)]
struct ParseLookupBindings {
    version: RecordId,
    template_id: String,
    template_hash: String,
}

#[derive(Clone, SurrealValue)]
struct ParseWriteBindings {
    record: RecordId,
    parse_id: SurrealUuid,
    version: RecordId,
    template_id: String,
    source_path: Option<String>,
    template_version: Option<String>,
    template_hash: String,
    ast: serde_json::Value,
    unmapped_lines: serde_json::Value,
}

#[derive(SurrealValue)]
struct ParseSnapshotRow {
    parse_id: SurrealUuid,
    created_at_utc: Datetime,
}

#[derive(Clone, SurrealValue)]
struct BulkSheetAppendRow {
    character: RecordId,
    expected_head: RecordId,
    version_record: RecordId,
    version_id: SurrealUuid,
    seq: i64,
    raw_text: String,
    author: String,
    tool: Option<String>,
    projection_rows: Vec<SheetFieldProjectionRow>,
}

#[derive(Clone, SurrealValue)]
struct BulkSheetAppendBindings {
    rows: Vec<BulkSheetAppendRow>,
    version_ids: Vec<SurrealUuid>,
    receipt_record: RecordId,
    receipt_id: SurrealUuid,
    operation: String,
    requested_by: String,
    target_count: i64,
    mutation_count: i64,
    receipt_payload: serde_json::Value,
    events: Vec<RecordEventBindings>,
    event_idempotency_keys: Vec<String>,
}

#[derive(SurrealValue)]
struct BulkSheetReceiptRow {
    receipt_id: SurrealUuid,
    operation: String,
    requested_by: String,
    target_count: i64,
    mutation_count: i64,
    status: String,
    payload: serde_json::Value,
    created_at_utc: Datetime,
}

#[derive(SurrealValue)]
struct BulkSheetLedgerRow {
    idempotency_key: String,
    event_id: String,
    event_sequence: i64,
}

#[derive(SurrealValue)]
struct BulkSheetCommitRow {
    versions: Vec<SheetVersionRow>,
    receipt: BulkSheetReceiptRow,
    ledgers: Vec<BulkSheetLedgerRow>,
}

/// The `THROW` token the append statement raises when the caller's expected
/// head is no longer the current head. Matched on the error text by the
/// guarded append path and mapped to [`AtelierError::Conflict`].
const SHEET_STALE_HEAD_THROW: &str = "HSK-SHEET-STALE-HEAD";

/// Append one sheet version, its field-value projection rows, and its event in
/// one statement. The projection rows (`FOR $row IN $domain.projection_rows`)
/// are the CKC field-memory table; PostgreSQL filled it with an
/// `ON CONFLICT DO NOTHING` insert per value inside the same transaction, and
/// the rows are already distinct per version here, so no conflict can arise.
const APPEND_SHEET_STATEMENT: &str = concat!(
    "RETURN { LET $head = (SELECT id, seq FROM atelier_sheet_version WHERE character_internal_id = $domain.character ORDER BY seq DESC LIMIT 1)[0]; LET $head_id = IF $head = NONE { NONE } ELSE { $head.id }; IF $domain.enforce_head AND $head_id != $domain.expected_head { THROW 'HSK-SHEET-STALE-HEAD'; }; LET $next_seq = IF $head = NONE { 1 } ELSE { $head.seq + 1 }; CREATE $domain.version_record CONTENT { version_id: $domain.version_id, character_internal_id: $domain.character, parent_version_id: $head_id, seq: $next_seq, raw_text: $domain.raw_text, author: $domain.author, tool: $domain.tool } RETURN NONE; FOR $row IN $domain.projection_rows { CREATE $row.record CONTENT { projection_id: $row.projection_id, field_id: $row.field_id, value: $row.value, character_internal_id: $domain.character, sheet_version_id: $domain.version_record } RETURN NONE; }; ",
    atelier_event_sql!(),
    " RETURN (SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM $domain.version_record)[0]; };"
);

const SELECT_SHEET_VERSION_BY_ID: &str = "SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM atelier_sheet_version WHERE version_id = $version_id LIMIT 1;";

/// Newest-first projection rows for one Field ID. `sheet_version_id` keys are
/// UUID v7, so the secondary sort is time-ordered within one instant.
const SELECT_FIELD_PROJECTION_BY_FIELD: &str = "SELECT `value`, record::id(character_internal_id) AS character_internal_id, record::id(sheet_version_id) AS sheet_version_id, created_at_utc FROM atelier_sheet_field_value_projection WHERE field_id = $field_id ORDER BY created_at_utc DESC, sheet_version_id DESC;";

const WRITE_PARSE_SNAPSHOT_STATEMENT: &str = concat!(
    "RETURN { LET $row = (UPSERT $domain.record SET parse_id = $domain.parse_id, version_id = $domain.version, template_id = $domain.template_id, source_path = $domain.source_path, template_version = $domain.template_version, template_hash = $domain.template_hash, ast = $domain.ast, unmapped_lines = $domain.unmapped_lines RETURN AFTER)[0]; ",
    atelier_event_sql!(),
    " RETURN $row; };"
);

const BULK_APPEND_SHEETS_STATEMENT: &str = "BEGIN TRANSACTION; RETURN { FOR $item IN $rows { LET $head = (SELECT VALUE id FROM atelier_sheet_version WHERE character_internal_id = $item.character ORDER BY seq DESC LIMIT 1)[0]; IF $head != $item.expected_head { THROW 'HSK-SHEET-BULK-STALE-HEAD'; }; }; FOR $item IN $rows { CREATE $item.version_record CONTENT { version_id: $item.version_id, character_internal_id: $item.character, parent_version_id: $item.expected_head, seq: $item.seq, raw_text: $item.raw_text, author: $item.author, tool: $item.tool } RETURN NONE; FOR $row IN $item.projection_rows { CREATE $row.record CONTENT { projection_id: $row.projection_id, field_id: $row.field_id, value: $row.value, character_internal_id: $item.character, sheet_version_id: $item.version_record } RETURN NONE; }; }; FOR $event IN $events { LET $existing = (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0]; IF $existing IS NONE { CREATE $event.ledger_id CONTENT { event_id: $event.kernel_event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.kernel_aggregate_type, aggregate_id: $event.kernel_aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.ledger_payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, created_at: $event.created_at } RETURN NONE; }; LET $ledger = (SELECT event_id, event_sequence FROM kernel_event_ledger WHERE idempotency_key = $event.idempotency_key LIMIT 1)[0]; CREATE $event.atelier_id CONTENT { event_id: $event.atelier_event_uuid, event_family: $event.event_family, aggregate_type: $event.kernel_aggregate_type, aggregate_id: $event.kernel_aggregate_id, kernel_event_id: $ledger.event_id, kernel_event_sequence: $ledger.event_sequence, payload: $event.atelier_payload } RETURN NONE; }; CREATE $receipt_record CONTENT { receipt_id: $receipt_id, operation: $operation, requested_by: $requested_by, target_count: $target_count, mutation_count: $mutation_count, status: 'applied', payload: $receipt_payload } RETURN NONE; RETURN { versions: (SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM atelier_sheet_version WHERE version_id IN $version_ids), receipt: (SELECT receipt_id, operation, requested_by, target_count, mutation_count, status, payload, created_at_utc FROM $receipt_record)[0], ledgers: (SELECT idempotency_key, event_id, event_sequence FROM kernel_event_ledger WHERE idempotency_key IN $event_idempotency_keys) }; }; COMMIT TRANSACTION;";

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

    /// Durable record of a refused guarded append (WP-CKC MT-009), keyed on
    /// `<character_ref>:conflict` so a no-context reader can find every stale
    /// write against one character.
    async fn record_sheet_version_conflict(
        &self,
        new: &NewSheetVersion,
        expected_parent_version_id: Option<Uuid>,
        current_parent_version_id: Option<Uuid>,
    ) -> AtelierResult<()> {
        self.record_event(
            sheet_event_family::SHEET_VERSION_CONFLICT,
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
    /// A stale expectation records a `SHEET_VERSION_CONFLICT` event and returns
    /// [`AtelierError::Conflict`]; nothing is appended.
    pub async fn append_sheet_version_if_current(
        &self,
        new: &NewSheetVersion,
        expected_parent_version_id: Option<Uuid>,
    ) -> AtelierResult<SheetVersion> {
        self.append_sheet_version_with_guard(new, Some(expected_parent_version_id))
            .await
    }

    /// The optimistic-concurrency check runs INSIDE the append statement
    /// (`IF $domain.enforce_head AND $head_id != $domain.expected_head THROW`),
    /// so the head cannot move between the check and the CREATE. PostgreSQL got
    /// the same guarantee from an advisory lock on the character; here it is the
    /// single-statement transaction.
    async fn append_sheet_version_with_guard(
        &self,
        new: &NewSheetVersion,
        expected_parent_version_id: Option<Option<Uuid>>,
    ) -> AtelierResult<SheetVersion> {
        let version_id = Uuid::now_v7();
        let enforce_head = expected_parent_version_id.is_some();
        let expected_head = expected_parent_version_id.flatten().map(|parent| {
            RecordId::new("atelier_sheet_version", SurrealUuid::from(parent))
        });
        let written = self
            .write_with_event(
                APPEND_SHEET_STATEMENT,
                SheetAppendBindings {
                    character: RecordId::new(
                        "atelier_character",
                        SurrealUuid::from(new.character_internal_id),
                    ),
                    version_record: RecordId::new(
                        "atelier_sheet_version",
                        SurrealUuid::from(version_id),
                    ),
                    version_id: version_id.into(),
                    expected_head,
                    enforce_head,
                    raw_text: new.raw_text.clone(),
                    author: new.author.clone(),
                    tool: new.tool.clone(),
                    projection_rows: sheet_field_projection_rows(&new.raw_text),
                },
                event_family::SHEET_VERSION_APPENDED,
                "atelier_sheet_version",
                &version_id.to_string(),
                serde_json::json!({
                    "version_id": version_id,
                }),
            )
            .await;
        let row: Option<SheetVersionRow> = match written {
            Ok(row) => row,
            Err(error) if error.to_string().contains(SHEET_STALE_HEAD_THROW) => {
                let expected = expected_parent_version_id.flatten();
                let current = self
                    .latest_sheet_version(new.character_internal_id)
                    .await?
                    .map(|version| version.version_id);
                self.record_sheet_version_conflict(new, expected, current)
                    .await?;
                return Err(AtelierError::Conflict(format!(
                    "stale_sheet_version: expected parent {expected:?}, current head {current:?}"
                )));
            }
            Err(error) => return Err(error),
        };
        row.map(version_from_row).ok_or_else(|| {
            AtelierError::Internal("sheet version append returned no row".to_owned())
        })
    }

    /// Fetch one sheet version by id.
    pub async fn get_sheet_version(&self, version_id: Uuid) -> AtelierResult<SheetVersion> {
        let row: Option<SheetVersionRow> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        SELECT_SHEET_VERSION_BY_ID,
                        SheetVersionBinding {
                            version_id: version_id.into(),
                        },
                    )
                    .await
                })
            })
            .await?;
        row.map(version_from_row).ok_or_else(|| {
            AtelierError::NotFound(format!("sheet version version_id={version_id}"))
        })
    }

    /// Recent distinct values for one parsed Field ID. This mirrors original
    /// CKC field memory: suggestions are exact-field scoped, never auto-filled,
    /// and ignore template placeholder descriptors such as `<string>`.
    ///
    /// Reads `atelier_sheet_field_value_projection`, the rows the append path
    /// writes in the same statement as each version, newest version first; the
    /// first row for a value names the latest version that used it and every
    /// later row adds to `occurrences`.
    pub async fn sheet_field_suggestions(
        &self,
        field_id: &str,
        limit: i64,
    ) -> AtelierResult<Vec<SheetFieldSuggestion>> {
        let field_id = field_id.trim();
        if field_id.is_empty() {
            return Err(AtelierError::Validation(
                "field_id must not be empty".to_string(),
            ));
        }
        let field_id = field_id.to_ascii_uppercase();
        let limit = limit.clamp(1, 50);
        let rows: Vec<SheetFieldProjectionReadRow> = self
            .with_data({
                let field_id = field_id.clone();
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_values(
                            SELECT_FIELD_PROJECTION_BY_FIELD,
                            FieldIdBindings { field_id },
                        )
                        .await
                    })
                }
            })
            .await?;
        let mut suggestions = Vec::<SheetFieldSuggestion>::new();
        let mut value_index = HashMap::<String, usize>::new();
        for row in rows {
            if let Some(index) = value_index.get(&row.value).copied() {
                suggestions[index].occurrences += 1;
                continue;
            }
            let version_id: Uuid = row.sheet_version_id.into();
            let character_internal_id: Uuid = row.character_internal_id.into();
            value_index.insert(row.value.clone(), suggestions.len());
            suggestions.push(SheetFieldSuggestion {
                field_id: field_id.clone(),
                value: row.value,
                occurrences: 1,
                latest_version_id: version_id,
                latest_character_internal_id: character_internal_id,
                latest_sheet_version_ref: sheet_version_ref(character_internal_id, version_id),
                latest_character_ref: character_ref(character_internal_id),
            });
        }
        suggestions.truncate(limit as usize);
        Ok(suggestions)
    }

    /// The current (highest-seq) sheet version for a character, if any.
    pub async fn latest_sheet_version(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Option<SheetVersion>> {
        let character = RecordId::new(
            "atelier_character",
            SurrealUuid::from(character_internal_id),
        );
        let row: Option<SheetVersionRow> = self
            .with_data(move |ctx| Box::pin(async move {
                ctx.query_first("SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM atelier_sheet_version WHERE character_internal_id = $character ORDER BY seq DESC LIMIT 1;", CharacterSheetBinding { character }).await
            }))
            .await?;
        Ok(row.map(version_from_row))
    }

    /// Full append-only version history (ascending sequence).
    pub async fn sheet_version_history(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Vec<SheetVersion>> {
        let character = RecordId::new(
            "atelier_character",
            SurrealUuid::from(character_internal_id),
        );
        let rows: Vec<SheetVersionRow> = self
            .with_data(move |ctx| Box::pin(async move {
                ctx.query_values("SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM atelier_sheet_version WHERE character_internal_id = $character ORDER BY seq ASC;", CharacterSheetBinding { character }).await
            }))
            .await?;
        Ok(rows.into_iter().map(version_from_row).collect())
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
        let source: Option<SheetVersionRow> = self
            .with_data(move |ctx| Box::pin(async move {
                ctx.query_first("SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM atelier_sheet_version WHERE version_id = $version_id LIMIT 1;", SheetVersionBinding { version_id: version_id.into() }).await
            }))
            .await?;
        let raw_text = source
            .map(version_from_row)
            .ok_or_else(|| {
                AtelierError::NotFound(format!("sheet version not found for parse: {version_id}"))
            })?
            .raw_text;
        let ast = parse_sheet_template_ast(&raw_text, template_id, source_path).map_err(AtelierError::Validation)?;
        let ast_json = serde_json::to_value(&ast)
            .map_err(|err| super::AtelierError::Validation(err.to_string()))?;
        let unmapped_json = serde_json::to_value(&ast.unmapped_lines)
            .map_err(|err| super::AtelierError::Validation(err.to_string()))?;

        let lookup = ParseLookupBindings {
            version: RecordId::new("atelier_sheet_version", SurrealUuid::from(version_id)),
            template_id: template_id.to_owned(),
            template_hash: ast.template_hash.clone(),
        };
        let existing: Option<ParseSnapshotRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT parse_id, created_at_utc FROM atelier_sheet_parse_snapshot WHERE version_id = $version AND template_id = $template_id AND template_hash = $template_hash LIMIT 1;", lookup).await })).await?;
        let parse_id: Uuid = existing
            .as_ref()
            .map(|row| Uuid::from(row.parse_id.clone()))
            .unwrap_or_else(Uuid::now_v7);
        let row: Option<ParseSnapshotRow> = self
            .write_with_event(
                WRITE_PARSE_SNAPSHOT_STATEMENT,
                ParseWriteBindings {
                    record: RecordId::new(
                        "atelier_sheet_parse_snapshot",
                        SurrealUuid::from(parse_id),
                    ),
                    parse_id: parse_id.into(),
                    version: RecordId::new("atelier_sheet_version", SurrealUuid::from(version_id)),
                    template_id: template_id.to_owned(),
                    source_path: source_path.map(ToOwned::to_owned),
                    template_version: ast.template_version.clone(),
                    template_hash: ast.template_hash.clone(),
                    ast: ast_json,
                    unmapped_lines: unmapped_json,
                },
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
            .await?;
        let created_at_utc: DateTime<Utc> = row
            .ok_or_else(|| {
                AtelierError::Internal("sheet parse snapshot write returned no row".to_owned())
            })?
            .created_at_utc
            .into();

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
        let source_row: Option<SheetVersionRow> = self
            .with_data({ let version_id = request.version_id; move |ctx| Box::pin(async move { ctx.query_first("SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM atelier_sheet_version WHERE version_id = $version_id LIMIT 1;", SheetVersionBinding { version_id: version_id.into() }).await }) })
            .await?;
        let source_row = source_row.ok_or_else(|| {
            super::AtelierError::NotFound(format!(
                "source sheet version not found for selective apply: {}",
                request.version_id
            ))
        })?;
        let source = version_from_row(source_row);
        if let Some(source_path) = request.source_path.as_deref() {
            reject_legacy_runtime_ref("source_path", source_path)?;
        }
        let ast = parse_sheet_template_ast(
            &source.raw_text,
            &request.template_id,
            request.source_path.as_deref(),
        ).map_err(AtelierError::Validation)?;
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

        let current_head_version_id = self
            .latest_sheet_version(source.character_internal_id)
            .await?
            .map(|row| row.version_id)
            .ok_or_else(|| {
                AtelierError::NotFound(format!(
                    "sheet head for character {}",
                    source.character_internal_id
                ))
            })?;
        if current_head_version_id != source.version_id {
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

        let version_id = Uuid::now_v7();
        let row: Option<SheetVersionRow> = self
            .write_with_event(
                APPEND_SHEET_STATEMENT,
                SheetAppendBindings {
                    character: RecordId::new(
                        "atelier_character",
                        SurrealUuid::from(source.character_internal_id),
                    ),
                    version_record: RecordId::new(
                        "atelier_sheet_version",
                        SurrealUuid::from(version_id),
                    ),
                    version_id: version_id.into(),
                    expected_head: Some(RecordId::new(
                        "atelier_sheet_version",
                        SurrealUuid::from(source.version_id),
                    )),
                    enforce_head: true,
                    projection_rows: sheet_field_projection_rows(&updated_raw_text),
                    raw_text: updated_raw_text,
                    author: request.author.clone(),
                    tool: request.tool.clone(),
                },
                event_family::SHEET_FIELD_EDITS_APPLIED,
                "atelier_sheet_version",
                &version_id.to_string(),
                serde_json::json!({
                    "new_version_id": version_id,
                    "source_version_id": source.version_id,
                    "template_id": &request.template_id,
                    "template_hash": &ast.template_hash,
                    "actor_role": &request.actor_role,
                    "applied_field_ids": &applied_field_ids,
                    "preserved_unmapped_count": ast.unmapped_lines.len(),
                }),
            )
            .await?;
        let version = row.map(version_from_row).ok_or_else(|| {
            AtelierError::Internal("sheet field edit append returned no row".to_owned())
        })?;

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
            let version_id = request.version_id;
            let source_row: Option<SheetVersionRow> = self
                .with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM atelier_sheet_version WHERE version_id = $version_id LIMIT 1;", SheetVersionBinding { version_id: version_id.into() }).await }))
                .await?;
            let source_row = source_row.ok_or_else(|| {
                super::AtelierError::NotFound(format!(
                    "source sheet version not found for bulk selective apply: {}",
                    request.version_id
                ))
            })?;
            let source = version_from_row(source_row);
            if let Some(source_path) = request.source_path.as_deref() {
                reject_legacy_runtime_ref("source_path", source_path)?;
            }
            let ast = parse_sheet_template_ast(
                &source.raw_text,
                &request.template_id,
                request.source_path.as_deref(),
            ).map_err(AtelierError::Validation)?;
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

        let mut character_ids: Vec<Uuid> = plans
            .iter()
            .map(|plan| plan.source.character_internal_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        character_ids.sort();
        let mut seen_bulk_characters = HashSet::new();
        for plan in &plans {
            if !seen_bulk_characters.insert(plan.source.character_internal_id) {
                return Err(super::AtelierError::Validation(format!(
                    "stale_selection: bulk sheet field edits require one current-head request per character; duplicate character_internal_id={}",
                    plan.source.character_internal_id
                )));
            }

            let current_head_version_id = self
                .latest_sheet_version(plan.source.character_internal_id)
                .await?
                .map(|row| row.version_id)
                .ok_or_else(|| {
                    AtelierError::NotFound(format!(
                        "sheet head for character {}",
                        plan.source.character_internal_id
                    ))
                })?;
            if current_head_version_id != plan.source.version_id {
                return Err(super::AtelierError::Validation(format!(
                    "stale_selection: source version {} is not the current head {}",
                    plan.source.version_id, current_head_version_id
                )));
            }
        }
        let new_version_ids = (0..plans.len()).map(|_| Uuid::now_v7()).collect::<Vec<_>>();
        let rows = plans
            .iter()
            .zip(&new_version_ids)
            .map(|(plan, version_id)| BulkSheetAppendRow {
                character: RecordId::new(
                    "atelier_character",
                    SurrealUuid::from(plan.source.character_internal_id),
                ),
                expected_head: RecordId::new(
                    "atelier_sheet_version",
                    SurrealUuid::from(plan.source.version_id),
                ),
                version_record: RecordId::new(
                    "atelier_sheet_version",
                    SurrealUuid::from(*version_id),
                ),
                version_id: (*version_id).into(),
                seq: plan.source.seq + 1,
                raw_text: plan.updated_raw_text.clone(),
                author: plan.request.author.clone(),
                tool: plan.request.tool.clone(),
                projection_rows: sheet_field_projection_rows(&plan.updated_raw_text),
            })
            .collect::<Vec<_>>();
        let receipt_id = Uuid::now_v7();
        let receipt_payload = serde_json::json!({
            "source_version_ids": plans
                .iter()
                .map(|plan| plan.source.version_id)
                .collect::<Vec<_>>(),
            "new_version_ids": &new_version_ids,
            "character_count": character_ids.len(),
        });
        let mut prepared_events = Vec::with_capacity(plans.len() + 1);
        for (plan, version_id) in plans.iter().zip(&new_version_ids) {
            prepared_events.push(self.prepare_event(
                event_family::SHEET_FIELD_EDITS_APPLIED,
                "atelier_sheet_version",
                &version_id.to_string(),
                serde_json::json!({
                    "new_version_id": version_id,
                    "source_version_id": plan.source.version_id,
                    "template_id": &plan.request.template_id,
                    "template_hash": &plan.template_hash,
                    "actor_role": &plan.request.actor_role,
                    "applied_field_ids": &plan.applied_field_ids,
                    "preserved_unmapped_count": plan.preserved_unmapped_lines.len(),
                    "bulk": true,
                }),
            )?);
        }
        prepared_events.push(self.prepare_event(
            event_family::BULK_OPERATION_APPLIED,
            "atelier_bulk_operation_receipt",
            &receipt_id.to_string(),
            serde_json::json!({
                "receipt_id": receipt_id,
                "operation": "bulk_apply_sheet_field_edits",
                "requested_by": requested_by,
                "target_count": requests.len(),
                "mutation_count": plans.len(),
                "status": "applied",
                "receipt_payload": &receipt_payload,
            }),
        )?);
        let bindings = BulkSheetAppendBindings {
            rows,
            version_ids: new_version_ids.iter().copied().map(Into::into).collect(),
            receipt_record: RecordId::new(
                "atelier_bulk_operation_receipt",
                SurrealUuid::from(receipt_id),
            ),
            receipt_id: receipt_id.into(),
            operation: "bulk_apply_sheet_field_edits".to_owned(),
            requested_by: requested_by.to_owned(),
            target_count: requests.len() as i64,
            mutation_count: plans.len() as i64,
            receipt_payload: receipt_payload.clone(),
            events: prepared_events
                .iter()
                .map(|event| event.bindings.clone())
                .collect(),
            event_idempotency_keys: prepared_events
                .iter()
                .map(|event| event.bindings.idempotency_key.clone())
                .collect(),
        };
        let committed: BulkSheetCommitRow = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values_at(BULK_APPEND_SHEETS_STATEMENT, bindings, 1)
                        .await
                        .map(|rows: Vec<BulkSheetCommitRow>| rows.into_iter().next())
                })
            })
            .await?
            .ok_or_else(|| {
                AtelierError::Internal("bulk sheet transaction returned no commit row".to_owned())
            })?;
        let mut recorded_by_key = committed
            .ledgers
            .into_iter()
            .map(|row| (row.idempotency_key.clone(), row))
            .collect::<HashMap<_, _>>();
        for prepared in prepared_events {
            let recorded = recorded_by_key
                .remove(&prepared.bindings.idempotency_key)
                .map(|row| RecordedLedgerRow {
                    event_id: row.event_id,
                    event_sequence: row.event_sequence,
                });
            self.finish_event(prepared, recorded).await?;
        }

        let mut by_id = committed
            .versions
            .into_iter()
            .map(version_from_row)
            .map(|version| (version.version_id, version))
            .collect::<HashMap<_, _>>();
        let mut results = Vec::with_capacity(plans.len());
        for (plan, version_id) in plans.into_iter().zip(new_version_ids) {
            let version = by_id.remove(&version_id).ok_or_else(|| {
                AtelierError::Internal(format!(
                    "bulk sheet append returned no row for {version_id}"
                ))
            })?;
            results.push(SheetFieldEditResult {
                version,
                source_version_id: plan.source.version_id,
                applied_field_ids: plan.applied_field_ids,
                preserved_unmapped_lines: plan.preserved_unmapped_lines,
            });
        }
        let receipt_row = committed.receipt;
        let receipt = BulkOperationReceipt {
            receipt_id: receipt_row.receipt_id.into(),
            operation: receipt_row.operation,
            requested_by: receipt_row.requested_by,
            target_count: receipt_row.target_count,
            mutation_count: receipt_row.mutation_count,
            status: receipt_row.status,
            payload: receipt_row.payload,
            created_at_utc: receipt_row.created_at_utc.into(),
        };

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
        let target_version_id = request.target_version_id;
        let target_row: Option<SheetVersionRow> = self
            .with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT version_id, record::id(character_internal_id) AS character_internal_id, IF parent_version_id = NONE { NONE } ELSE { record::id(parent_version_id) } AS parent_version_id, seq, raw_text, author, tool, created_at_utc FROM atelier_sheet_version WHERE version_id = $version_id LIMIT 1;", SheetVersionBinding { version_id: target_version_id.into() }).await }))
            .await?;
        let target = target_row
            .map(version_from_row)
            .filter(|row| row.character_internal_id == request.character_internal_id)
            .ok_or_else(|| {
                super::AtelierError::NotFound(format!(
                    "target sheet version not found for revert: {}",
                    request.target_version_id
                ))
            })?;
        let previous_head = self
            .latest_sheet_version(request.character_internal_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!(
                    "sheet head for character {}",
                    request.character_internal_id
                ))
            })?;
        let previous_head_version_id = previous_head.version_id;

        let current_ast = parse_sheet_template_ast(
            &previous_head.raw_text,
            &request.template_id,
            request.source_path.as_deref(),
        ).map_err(AtelierError::Validation)?;
        let target_ast = parse_sheet_template_ast(
            &target.raw_text,
            &request.template_id,
            request.source_path.as_deref(),
        ).map_err(AtelierError::Validation)?;
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

        let version_id = Uuid::now_v7();
        let row: Option<SheetVersionRow> = self
            .write_with_event(
                APPEND_SHEET_STATEMENT,
                SheetAppendBindings {
                    character: RecordId::new(
                        "atelier_character",
                        SurrealUuid::from(request.character_internal_id),
                    ),
                    version_record: RecordId::new(
                        "atelier_sheet_version",
                        SurrealUuid::from(version_id),
                    ),
                    version_id: version_id.into(),
                    expected_head: Some(RecordId::new(
                        "atelier_sheet_version",
                        SurrealUuid::from(previous_head_version_id),
                    )),
                    enforce_head: true,
                    projection_rows: sheet_field_projection_rows(&reverted_raw_text),
                    raw_text: reverted_raw_text,
                    author: request.author.clone(),
                    tool: request.tool.clone(),
                },
                event_family::SHEET_VERSION_REVERTED,
                "atelier_sheet_version",
                &version_id.to_string(),
                serde_json::json!({
                    "new_version_id": version_id,
                    "previous_head_version_id": previous_head_version_id,
                    "reverted_to_version_id": target.version_id,
                    "revert_scope": revert_scope,
                    "reverted_field_ids": &reverted_field_ids,
                    "target_seq": target.seq,
                    "new_seq": previous_head.seq + 1,
                }),
            )
            .await?;
        let version = row.map(version_from_row).ok_or_else(|| {
            AtelierError::Internal("sheet revert append returned no row".to_owned())
        })?;

        Ok(SheetVersionRevertResult {
            version,
            reverted_to_version_id: target.version_id,
            previous_head_version_id,
        })
    }
}
