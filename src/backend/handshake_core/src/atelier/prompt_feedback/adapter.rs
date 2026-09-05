//! WP-CKC-posekit-overhaul MT-020: Leeseo CUIPP import + JSONL export adapter.
//!
//! The import side turns Leeseo/CUIPP recipe rows and prompt-stress scene rows
//! into normalized [`NewPromptCase`] values. It NEVER stores a raw machine path:
//! image references are normalized to portable `dataset://` / `artifact://` refs
//! so they survive the atelier ref validators (no localhost, no drive letters).
//!
//! The export side is a pure function that renders corrected prompt rows as
//! CUI/Leeseo-compatible JSONL carrying the source case id, rule-pack id +
//! version, rewrite trace, and the original prompt hash. The bytes it produces
//! are materialized as a hashed ArtifactStore artifact by `super` -- the JSONL is
//! an artifact, never product authority.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::engine::RewriteOutcome;
use super::model::{NewPromptCase, PromptCaseAxes};
use super::PromptFeedbackError;

/// One CUIPP recipe / prompt-stress scene row from a Leeseo eval suite. Fields
/// mirror the real i76 `*.cuipp-recipes.jsonl` / `*_scene_manifest.jsonl` schema;
/// everything is optional so the adapter is tolerant of schema drift and the
/// caller can also pass fully explicit rows.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CuippRow {
    pub case_id: String,
    #[serde(default)]
    pub cell: Option<String>,
    #[serde(default)]
    pub segment: Option<String>,
    #[serde(default)]
    pub render_stack: Option<String>,
    #[serde(default)]
    pub render_key: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub framing: Option<String>,
    #[serde(default)]
    pub clothing_state: Option<String>,
    #[serde(default)]
    pub identity_judgement_allowed: Option<bool>,
    #[serde(default)]
    pub prompt_quality_review_allowed: Option<bool>,
    /// Fully assembled positive prompt if the caller has one.
    #[serde(default)]
    pub positive_prompt: Option<String>,
    /// Fully assembled negative prompt if the caller has one.
    #[serde(default)]
    pub negative_prompt: Option<String>,
    /// CUIPP-contributed positive tail (the leak vector on standard rows).
    #[serde(default)]
    pub positive_tail: Option<String>,
    /// CUIPP-contributed negative extra.
    #[serde(default)]
    pub negative_extra: Option<String>,
    #[serde(default)]
    pub contact_level: Option<String>,
    #[serde(default)]
    pub outfit: Option<String>,
    #[serde(default)]
    pub outfit_access: Option<String>,
    #[serde(default)]
    pub setting_family: Option<String>,
    #[serde(default)]
    pub scene: Option<String>,
    #[serde(default)]
    pub body_target_terms: Option<String>,
    #[serde(default)]
    pub micro_gate: Option<String>,
    #[serde(default)]
    pub expected_failure: Option<String>,
    #[serde(default)]
    pub recipe_id: Option<String>,
    /// Already-portable image ref (`artifact://` or `dataset://`). Preferred.
    #[serde(default)]
    pub image_ref: Option<String>,
    /// Bare image file name; the adapter synthesizes a portable `dataset://` ref.
    #[serde(default)]
    pub image_name: Option<String>,
    #[serde(default)]
    pub sheet_ref: Option<String>,
    /// Any additional hardcore/free-form fields to preserve verbatim.
    #[serde(default)]
    pub hardcore_fields: Option<serde_json::Value>,
}

/// A batch import request for one Leeseo eval suite (e.g. i76).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeeseoImportRequest {
    pub project_id: String,
    pub source_system: String,
    pub adapter_id: String,
    #[serde(default)]
    pub source_iteration_id: Option<String>,
    pub imported_by: String,
    pub rows: Vec<CuippRow>,
}

/// A prompt-stress CSV manifest import request. This stays pure adapter logic:
/// parsing produces [`NewPromptCase`] rows; persistence remains
/// [`super::AtelierStore::import_prompt_cases`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptStressCsvImportRequest {
    pub project_id: String,
    pub source_system: String,
    pub adapter_id: String,
    #[serde(default)]
    pub source_iteration_id: Option<String>,
    #[serde(default)]
    pub source_manifest_ref: Option<String>,
    pub imported_by: String,
    pub csv: String,
}

fn trimmed_opt(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|inner| !inner.is_empty())
        .map(ToOwned::to_owned)
}

/// Lowercase slug safe for a `dataset://` path segment (no spaces, drive
/// letters, or reserved ref characters).
fn slug(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed
    }
}

fn sha256_prefixed(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn is_portable_ref(value: &str) -> bool {
    (value.starts_with("artifact://") || value.starts_with("dataset://"))
        && super::super::reject_legacy_runtime_ref("image_ref", value).is_ok()
}

fn validate_source_manifest_ref(req: &PromptStressCsvImportRequest) -> Result<(), PromptFeedbackError> {
    let Some(source_manifest_ref) = req.source_manifest_ref.as_deref() else {
        return Ok(());
    };
    if source_manifest_ref.is_empty()
        || !is_portable_ref(source_manifest_ref)
        || super::super::reject_legacy_runtime_ref("source_manifest_ref", source_manifest_ref)
            .is_err()
    {
        return Err(PromptFeedbackError::Validation(
            "prompt-stress csv source_manifest_ref must be a portable Handshake artifact:// or dataset:// ref without machine-local path".to_string(),
        ));
    }
    Ok(())
}

fn looks_like_machine_path(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.contains('\\')
        || trimmed.starts_with('/')
        || trimmed.starts_with('~')
        || trimmed
            .as_bytes()
            .get(1)
            .is_some_and(|ch| *ch == b':')
}

fn basename_from_possible_path(value: &str) -> String {
    value
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(value)
        .trim()
        .to_string()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CsvRecord {
    row_number: usize,
    fields: Vec<String>,
}

fn finish_csv_record(
    records: &mut Vec<CsvRecord>,
    row_number: usize,
    row: &mut Vec<String>,
    field: &mut String,
) {
    row.push(std::mem::take(field));
    let is_blank = row.iter().all(|value| value.trim().is_empty());
    if !is_blank {
        records.push(CsvRecord {
            row_number,
            fields: std::mem::take(row),
        });
    } else {
        row.clear();
    }
}

/// Strict RFC-4180-style CSV parser for the prompt-stress manifest. It is small
/// on purpose because MT-052 cannot widen `Cargo.toml` to add a new parser
/// dependency, but it still handles quoted commas/newlines and escaped quotes.
fn parse_csv_records(input: &str) -> Result<Vec<CsvRecord>, PromptFeedbackError> {
    let mut records = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;
    let mut after_closing_quote = false;
    let mut line_number = 1usize;
    let mut row_start_line = 1usize;
    while let Some(ch) = chars.next() {
        let column_number = row.len() + 1;
        if in_quotes {
            match ch {
                '"' => {
                    if matches!(chars.peek(), Some('"')) {
                        chars.next();
                        field.push('"');
                    } else {
                        in_quotes = false;
                        after_closing_quote = true;
                    }
                }
                '\r' => {
                    if matches!(chars.peek(), Some('\n')) {
                        chars.next();
                    }
                    line_number += 1;
                    field.push('\n');
                }
                '\n' => {
                    line_number += 1;
                    field.push('\n');
                }
                _ => field.push(ch),
            }
            continue;
        }
        if after_closing_quote {
            match ch {
                ',' => {
                    row.push(std::mem::take(&mut field));
                    after_closing_quote = false;
                }
                '\r' => {
                    if matches!(chars.peek(), Some('\n')) {
                        chars.next();
                    }
                    finish_csv_record(&mut records, row_start_line, &mut row, &mut field);
                    after_closing_quote = false;
                    line_number += 1;
                    row_start_line = line_number;
                }
                '\n' => {
                    finish_csv_record(&mut records, row_start_line, &mut row, &mut field);
                    after_closing_quote = false;
                    line_number += 1;
                    row_start_line = line_number;
                }
                _ => {
                    return Err(PromptFeedbackError::Validation(format!(
                        "prompt-stress csv row {row_start_line} column {column_number}: expected delimiter or line ending after quoted field"
                    )));
                }
            }
            continue;
        }
        match ch {
            '"' if field.is_empty() => {
                in_quotes = true;
            }
            '"' => {
                return Err(PromptFeedbackError::Validation(format!(
                    "prompt-stress csv row {row_start_line} column {column_number}: unexpected quote inside unquoted field"
                )));
            }
            ',' => {
                row.push(std::mem::take(&mut field));
            }
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                finish_csv_record(&mut records, row_start_line, &mut row, &mut field);
                line_number += 1;
                row_start_line = line_number;
            }
            '\n' => {
                finish_csv_record(&mut records, row_start_line, &mut row, &mut field);
                line_number += 1;
                row_start_line = line_number;
            }
            _ => field.push(ch),
        }
    }
    if in_quotes {
        let column_number = row.len() + 1;
        return Err(PromptFeedbackError::Validation(format!(
            "prompt-stress csv row {row_start_line} column {column_number}: unterminated quoted field"
        )));
    }
    if !field.is_empty() || !row.is_empty() {
        finish_csv_record(&mut records, row_start_line, &mut row, &mut field);
    }
    Ok(records)
}

fn normalize_csv_header(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('\u{feff}')
        .chars()
        .map(|ch| match ch {
            'A'..='Z' => ch.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn canonical_prompt_stress_header(normalized: &str) -> Option<&'static str> {
    match normalized {
        "case_id" | "source_case_id" => Some("case_id"),
        "segment" | "segment_name" => Some("segment"),
        "cell" | "cell_id" | "grid_cell" => Some("cell"),
        "framing" => Some("framing"),
        "clothing_state" => Some("clothing_state"),
        "render_stack" | "renderstack" => Some("render_stack"),
        "render_key" | "cuipp_render_key" | "render" => Some("render_key"),
        "positive_prompt" | "prompt_positive" | "positive" | "prompt" => Some("positive_prompt"),
        "negative_prompt" | "prompt_negative" | "negative" | "negative_extra" => {
            Some("negative_prompt")
        }
        "contact_level" => Some("contact_level"),
        "outfit" => Some("outfit"),
        "outfit_access" => Some("outfit_access"),
        "setting_family" => Some("setting_family"),
        "scene" => Some("scene"),
        "body_target_terms" => Some("body_target_terms"),
        "micro_gate" => Some("micro_gate"),
        "expected_failure" => Some("expected_failure"),
        "recipe_id" | "cuipp_recipe_id" => Some("recipe_id"),
        "image_ref" | "artifact_ref" => Some("image_ref"),
        "image_name" | "image_file" | "filename" | "image" => Some("image_name"),
        "sheet_ref" => Some("sheet_ref"),
        _ => None,
    }
}

fn required_csv_value<'a>(
    row_number: usize,
    values: &'a HashMap<&'static str, String>,
    key: &'static str,
) -> Result<&'a str, String> {
    values
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("row {row_number} column {key}: required value is empty"))
}

fn optional_csv_value(values: &HashMap<&'static str, String>, key: &'static str) -> Option<String> {
    values
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_prompt_stress_segment(row_number: usize, segment: Option<String>) -> Result<String, String> {
    let Some(segment) = segment else {
        return Ok(super::engine::SEGMENT_PROMPT_STRESS.to_string());
    };
    let normalized = normalize_csv_header(&segment);
    match normalized.as_str() {
        "prompt_stress" => Ok(super::engine::SEGMENT_PROMPT_STRESS.to_string()),
        _ => Err(format!(
            "row {row_number} column segment: expected prompt_stress, got {segment}"
        )),
    }
}

fn build_prompt_stress_row(
    req: &PromptStressCsvImportRequest,
    row_number: usize,
    original_headers: &[String],
    raw_fields: &[String],
    values: &HashMap<&'static str, String>,
    unmapped: BTreeMap<String, String>,
) -> Result<CuippRow, String> {
    let case_id = required_csv_value(row_number, values, "case_id")?.to_string();
    if case_id.chars().any(char::is_control) {
        return Err(format!(
            "row {row_number} column case_id: control characters are not allowed"
        ));
    }
    let cell = required_csv_value(row_number, values, "cell")?.to_string();
    let positive_prompt = required_csv_value(row_number, values, "positive_prompt")?.to_string();
    let render_stack = optional_csv_value(values, "render_stack");
    let render_key = optional_csv_value(values, "render_key");
    if render_stack.is_none() && render_key.is_none() {
        return Err(format!(
            "row {row_number} column render_stack: render_stack or render_key is required"
        ));
    }
    let segment = normalize_prompt_stress_segment(row_number, optional_csv_value(values, "segment"))?;

    let image_ref = optional_csv_value(values, "image_ref");
    if let Some(ref image_ref) = image_ref {
        if !is_portable_ref(image_ref) {
            return Err(format!(
                "row {row_number} column image_ref: expected portable Handshake artifact:// or dataset:// ref without machine-local path"
            ));
        }
    }
    let image_name = optional_csv_value(values, "image_name").map(|value| {
        if looks_like_machine_path(&value) {
            basename_from_possible_path(&value)
        } else {
            value
        }
    });

    let row_hash = sha256_prefixed(&serde_json::json!({
        "headers": original_headers,
        "fields": raw_fields,
    }));
    let hardcore_fields = serde_json::json!({
        "csv": {
            "source_format": "prompt_stress_manifest.csv",
            "source_manifest_ref": req.source_manifest_ref.clone().unwrap_or_default(),
            "row_number": row_number,
            "row_hash": row_hash,
            "headers": original_headers,
            "unmapped": unmapped,
        }
    });

    Ok(CuippRow {
        case_id,
        cell: Some(cell),
        segment: Some(segment),
        render_stack,
        render_key,
        framing: optional_csv_value(values, "framing"),
        clothing_state: optional_csv_value(values, "clothing_state"),
        identity_judgement_allowed: Some(false),
        prompt_quality_review_allowed: Some(true),
        positive_prompt: Some(positive_prompt),
        negative_prompt: optional_csv_value(values, "negative_prompt"),
        contact_level: optional_csv_value(values, "contact_level"),
        outfit: optional_csv_value(values, "outfit"),
        outfit_access: optional_csv_value(values, "outfit_access"),
        setting_family: optional_csv_value(values, "setting_family"),
        scene: optional_csv_value(values, "scene"),
        body_target_terms: optional_csv_value(values, "body_target_terms"),
        micro_gate: optional_csv_value(values, "micro_gate"),
        expected_failure: optional_csv_value(values, "expected_failure"),
        recipe_id: optional_csv_value(values, "recipe_id"),
        image_ref,
        image_name,
        sheet_ref: optional_csv_value(values, "sheet_ref"),
        hardcore_fields: Some(hardcore_fields),
        ..Default::default()
    })
}

/// Parse a prompt_stress_manifest.csv into prompt-feedback cases. The parser is
/// strict and all-or-nothing: malformed rows produce a row/column-specific error
/// before any persistence can occur.
pub fn import_prompt_stress_csv_manifest(
    req: &PromptStressCsvImportRequest,
) -> Result<Vec<NewPromptCase>, PromptFeedbackError> {
    validate_source_manifest_ref(req)?;
    let records = parse_csv_records(&req.csv)?;
    let Some(header_record) = records.first() else {
        return Err(PromptFeedbackError::Validation(
            "prompt-stress csv is empty".to_string(),
        ));
    };
    let original_headers: Vec<String> = header_record
        .fields
        .iter()
        .map(|header| header.trim().trim_start_matches('\u{feff}').to_string())
        .collect();
    let mut canonical_headers: Vec<Option<&'static str>> = Vec::with_capacity(original_headers.len());
    let mut seen_headers = HashSet::new();
    for header in &original_headers {
        let normalized = normalize_csv_header(header);
        if normalized.is_empty() {
            return Err(PromptFeedbackError::Validation(
                "prompt-stress csv header contains an empty column name".to_string(),
            ));
        }
        let canonical = canonical_prompt_stress_header(&normalized);
        let duplicate_key = canonical
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("unmapped:{normalized}"));
        if !seen_headers.insert(duplicate_key.clone()) {
            return Err(PromptFeedbackError::Validation(format!(
                "prompt-stress csv duplicate header: {header}"
            )));
        }
        canonical_headers.push(canonical);
    }
    for required in ["case_id", "cell", "positive_prompt"] {
        if !canonical_headers.iter().any(|entry| *entry == Some(required)) {
            return Err(PromptFeedbackError::Validation(format!(
                "prompt-stress csv missing required header: {required}"
            )));
        }
    }

    let mut errors = Vec::new();
    let mut rows = Vec::new();
    let mut seen_case_ids: HashMap<String, usize> = HashMap::new();
    for record in records.iter().skip(1) {
        if record.fields.len() != original_headers.len() {
            errors.push(format!(
                "row {}: expected {} fields from header, got {}",
                record.row_number,
                original_headers.len(),
                record.fields.len()
            ));
            continue;
        }
        let mut values: HashMap<&'static str, String> = HashMap::new();
        let mut unmapped = BTreeMap::new();
        for (index, value) in record.fields.iter().enumerate() {
            let trimmed = value.trim().to_string();
            if let Some(canonical) = canonical_headers[index] {
                values.insert(canonical, trimmed);
            } else if !trimmed.is_empty() {
                unmapped.insert(original_headers[index].clone(), trimmed);
            }
        }
        match build_prompt_stress_row(
            req,
            record.row_number,
            &original_headers,
            &record.fields,
            &values,
            unmapped,
        ) {
            Ok(row) => {
                let case_id_key = row.case_id.clone();
                if let Some(first_row) = seen_case_ids.insert(case_id_key.clone(), record.row_number)
                {
                    errors.push(format!(
                        "row {} column case_id: duplicate case_id {case_id_key} first seen on row {first_row}",
                        record.row_number
                    ));
                } else {
                    rows.push(row);
                }
            }
            Err(err) => errors.push(err),
        }
    }
    if !errors.is_empty() {
        return Err(PromptFeedbackError::Validation(format!(
            "prompt-stress csv import failed: {}",
            errors.join("; ")
        )));
    }
    if rows.is_empty() {
        return Err(PromptFeedbackError::Validation(
            "prompt-stress csv contains no data rows".to_string(),
        ));
    }
    let leeseo = LeeseoImportRequest {
        project_id: req.project_id.clone(),
        source_system: req.source_system.clone(),
        adapter_id: req.adapter_id.clone(),
        source_iteration_id: req.source_iteration_id.clone(),
        imported_by: req.imported_by.clone(),
        rows,
    };
    import_leeseo(&leeseo)
}

fn derive_render_stack(row: &CuippRow) -> String {
    if let Some(explicit) = trimmed_opt(&row.render_stack) {
        return explicit;
    }
    let probe = format!(
        "{} {} {}",
        row.render_key.clone().unwrap_or_default(),
        row.mode.clone().unwrap_or_default(),
        row.case_id
    )
    .to_ascii_lowercase();
    // Order matters: check FaceID first, then the explicit no-detail marker BEFORE
    // the bare "detail" substring (otherwise "no_detail" wrongly matches "detail").
    if probe.contains("faceid") {
        "FaceDetailer+FaceID".to_string()
    } else if probe.contains("no_detail") || probe.contains("no-detail") {
        "no_detail".to_string()
    } else if probe.contains("facedetailer") || probe.contains("detail") {
        "FaceDetailer".to_string()
    } else {
        "no_detail".to_string()
    }
}

fn derive_framing(row: &CuippRow, cell: &str) -> String {
    if let Some(explicit) = trimmed_opt(&row.framing) {
        return explicit;
    }
    let lower = cell.to_ascii_lowercase();
    if lower.contains("closeup") || lower.starts_with("0_") {
        "close-up".to_string()
    } else if lower.contains("full") {
        "full".to_string()
    } else if lower.contains("tq") || lower.contains("3q") {
        "3/4".to_string()
    } else {
        "unknown".to_string()
    }
}

fn derive_clothing_state(row: &CuippRow, cell: &str) -> String {
    if let Some(explicit) = trimmed_opt(&row.clothing_state) {
        return explicit;
    }
    if cell.to_ascii_lowercase().contains("naked") {
        "naked".to_string()
    } else {
        "clothed".to_string()
    }
}

fn portable_image_ref(row: &CuippRow, source_system: &str, iteration: &str) -> Option<String> {
    if let Some(explicit) = trimmed_opt(&row.image_ref) {
        if explicit.starts_with("artifact://") || explicit.starts_with("dataset://") {
            return Some(explicit);
        }
        // A non-portable explicit ref (e.g. a raw machine path) is dropped, not
        // stored, so the atelier ref validators never reject the whole import.
    }
    if let Some(name) = trimmed_opt(&row.image_name) {
        return Some(format!(
            "dataset://{}/{}/{}",
            slug(source_system),
            slug(iteration),
            slug(&name)
        ));
    }
    None
}

/// Normalize one CUIPP row into a [`NewPromptCase`].
pub fn normalize_row(
    req: &LeeseoImportRequest,
    row: &CuippRow,
) -> Result<NewPromptCase, PromptFeedbackError> {
    let source_case_id = row.case_id.trim().to_string();
    if source_case_id.is_empty() {
        return Err(PromptFeedbackError::Validation(
            "cuipp row is missing case_id".to_string(),
        ));
    }
    let segment = trimmed_opt(&row.segment).unwrap_or_else(|| "standard".to_string());
    let cell = trimmed_opt(&row.cell).unwrap_or_else(|| "unknown".to_string());
    let render_stack = derive_render_stack(row);
    let framing = derive_framing(row, &cell);
    let clothing_state = derive_clothing_state(row, &cell);

    let is_prompt_stress = segment == super::engine::SEGMENT_PROMPT_STRESS;
    // Core invariant (NON-OVERRIDABLE): a prompt-stress case is NEVER
    // identity-success evidence and IS prompt-quality/porn-readiness evidence. A
    // client-supplied override is ignored for the prompt-stress segment so the
    // invariant can never be flipped at import time.
    let (identity_judgement_allowed, prompt_quality_review_allowed) = if is_prompt_stress {
        (false, true)
    } else {
        (
            row.identity_judgement_allowed.unwrap_or(true),
            row.prompt_quality_review_allowed.unwrap_or(true),
        )
    };

    let tail = trimmed_opt(&row.positive_tail);
    let positive_prompt = trimmed_opt(&row.positive_prompt)
        .or_else(|| tail.clone())
        .unwrap_or_default();
    let negative_prompt = trimmed_opt(&row.negative_prompt)
        .or_else(|| trimmed_opt(&row.negative_extra))
        .unwrap_or_default();

    // On a `standard` row a prompt-stress positive tail is protected-runner
    // leakage; flag it so the engine can strip it. On a prompt-stress row the
    // tail is legitimate and is not flagged as a leak.
    let prompt_stress_positive_tail = if segment == super::engine::SEGMENT_STANDARD {
        tail
    } else {
        None
    };

    let iteration = req
        .source_iteration_id
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let image_artifact_ref = portable_image_ref(row, &req.source_system, &iteration);
    let sheet_artifact_ref = trimmed_opt(&row.sheet_ref).filter(|value| {
        value.starts_with("artifact://")
            || value.starts_with("dataset://")
            || value.starts_with("atelier://")
    });

    let axes = PromptCaseAxes {
        contact_level: trimmed_opt(&row.contact_level),
        outfit: trimmed_opt(&row.outfit),
        outfit_access: trimmed_opt(&row.outfit_access),
        setting_family: trimmed_opt(&row.setting_family),
        scene: trimmed_opt(&row.scene),
        body_target_terms: trimmed_opt(&row.body_target_terms),
        prompt_stress_positive_tail,
    };

    Ok(NewPromptCase {
        project_id: req.project_id.clone(),
        source_system: req.source_system.clone(),
        adapter_id: req.adapter_id.clone(),
        source_iteration_id: req.source_iteration_id.clone(),
        source_case_id,
        source_recipe_id: trimmed_opt(&row.recipe_id),
        segment,
        cell,
        framing,
        clothing_state,
        render_stack,
        identity_judgement_allowed,
        prompt_quality_review_allowed,
        positive_prompt,
        negative_prompt,
        micro_gate: trimmed_opt(&row.micro_gate),
        expected_failure: trimmed_opt(&row.expected_failure),
        image_artifact_ref,
        sheet_artifact_ref,
        axes,
        hardcore_fields: row
            .hardcore_fields
            .clone()
            .unwrap_or_else(|| serde_json::json!({})),
        imported_by: req.imported_by.clone(),
    })
}

/// Normalize a whole import request.
pub fn import_leeseo(req: &LeeseoImportRequest) -> Result<Vec<NewPromptCase>, PromptFeedbackError> {
    req.rows.iter().map(|row| normalize_row(req, row)).collect()
}

/// One corrected prompt row in the JSONL export.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportRow {
    pub schema_id: String,
    pub source_case_id: String,
    pub segment: String,
    pub cell: String,
    pub render_stack: String,
    pub rule_pack_id: String,
    pub rule_pack_version: i32,
    pub original_prompt_hash: String,
    pub rewritten_prompt_hash: String,
    pub positive_prompt: String,
    pub negative_prompt: String,
    pub changed_fields: Vec<String>,
    pub rule_trace: RewriteOutcome,
}

/// The bytes + provenance of a JSONL export, before it is written to the
/// ArtifactStore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportBundle {
    pub jsonl: String,
    pub row_count: usize,
    pub source_case_ids: Vec<String>,
}

/// Render corrected prompt rows as deterministic JSONL. Rows are sorted by
/// `source_case_id` so the byte output is stable regardless of input ordering.
pub fn export_jsonl(mut rows: Vec<ExportRow>) -> Result<ExportBundle, PromptFeedbackError> {
    rows.sort_by(|a, b| a.source_case_id.cmp(&b.source_case_id));
    let mut jsonl = String::new();
    let mut source_case_ids = Vec::with_capacity(rows.len());
    for row in &rows {
        let line = serde_json::to_string(row).map_err(|err| {
            PromptFeedbackError::Validation(format!("failed to serialize export row: {err}"))
        })?;
        jsonl.push_str(&line);
        jsonl.push('\n');
        source_case_ids.push(row.source_case_id.clone());
    }
    Ok(ExportBundle {
        row_count: rows.len(),
        source_case_ids,
        jsonl,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request(rows: Vec<CuippRow>) -> LeeseoImportRequest {
        LeeseoImportRequest {
            project_id: "leeseo".to_string(),
            source_system: "leeseo".to_string(),
            adapter_id: "leeseo.cuipp.v1".to_string(),
            source_iteration_id: Some("i76".to_string()),
            imported_by: "test".to_string(),
            rows,
        }
    }

    #[test]
    fn prompt_stress_row_is_not_identity_evidence() {
        let req = base_request(vec![CuippRow {
            case_id: "with_detail_faceid:0_closeup:1".to_string(),
            segment: Some("prompt_stress".to_string()),
            cell: Some("0_closeup".to_string()),
            render_key: Some("FaceDetailer+FaceID".to_string()),
            positive_tail: Some("open blouse no bra".to_string()),
            ..Default::default()
        }]);
        let cases = import_leeseo(&req).expect("import");
        assert_eq!(cases.len(), 1);
        assert!(!cases[0].identity_judgement_allowed);
        assert_eq!(cases[0].render_stack, "FaceDetailer+FaceID");
        // Legitimate tail on a prompt-stress row is not flagged as a leak.
        assert!(cases[0].axes.prompt_stress_positive_tail.is_none());
    }

    #[test]
    fn prompt_stress_identity_override_is_ignored() {
        // A client tries to force identity_judgement_allowed=true on a
        // prompt-stress row; the adapter must ignore the override.
        let req = base_request(vec![CuippRow {
            case_id: "with_detail_faceid:0_closeup:2".to_string(),
            segment: Some("prompt_stress".to_string()),
            cell: Some("0_closeup".to_string()),
            identity_judgement_allowed: Some(true),
            prompt_quality_review_allowed: Some(false),
            ..Default::default()
        }]);
        let cases = import_leeseo(&req).expect("import");
        assert!(!cases[0].identity_judgement_allowed);
        assert!(cases[0].prompt_quality_review_allowed);
    }

    #[test]
    fn standard_row_flags_leaked_tail() {
        let req = base_request(vec![CuippRow {
            case_id: "no_detail:0_closeup:1".to_string(),
            segment: Some("standard".to_string()),
            cell: Some("0_closeup".to_string()),
            positive_tail: Some("prompt-stress wardrobe tail".to_string()),
            ..Default::default()
        }]);
        let cases = import_leeseo(&req).expect("import");
        assert!(cases[0].identity_judgement_allowed);
        assert_eq!(
            cases[0].axes.prompt_stress_positive_tail.as_deref(),
            Some("prompt-stress wardrobe tail")
        );
    }

    #[test]
    fn raw_machine_path_is_not_stored_as_image_ref() {
        let req = base_request(vec![CuippRow {
            case_id: "no_detail:0_closeup:1".to_string(),
            image_ref: Some("D:\\Projects\\leeseo\\i76\\img.png".to_string()),
            ..Default::default()
        }]);
        let cases = import_leeseo(&req).expect("import");
        assert!(cases[0].image_artifact_ref.is_none());
    }

    #[test]
    fn image_name_becomes_portable_dataset_ref() {
        let req = base_request(vec![CuippRow {
            case_id: "no_detail:0_closeup:1".to_string(),
            image_name: Some("closeup 01.png".to_string()),
            ..Default::default()
        }]);
        let cases = import_leeseo(&req).expect("import");
        assert_eq!(
            cases[0].image_artifact_ref.as_deref(),
            Some("dataset://leeseo/i76/closeup-01.png")
        );
    }
}
