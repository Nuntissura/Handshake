use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::identity::{CrdtWorkspaceIdentityV1, validate_crdt_workspace_identity};
use super::persistence::{CrdtUpdateRecordV1, sha256_hex, validate_crdt_update_record};

pub const CRDT_CONTEXT_SLICE_SCHEMA_ID: &str = "hsk.kernel.crdt_context_slice@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtContextSliceKind {
    Summary,
    SelectedRange,
    FieldDigest,
    OperationDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtContextVersionRefV1 {
    pub state_vector: String,
    pub latest_update_seq: u64,
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtSelectionRangeV1 {
    pub field_id: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtContextSliceRequestV1 {
    pub identity: CrdtWorkspaceIdentityV1,
    pub requested_kinds: Vec<CrdtContextSliceKind>,
    pub selected_range: Option<CrdtSelectionRangeV1>,
    pub max_text_bytes: usize,
    pub max_operation_deltas: usize,
    pub version: CrdtContextVersionRefV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtMaterializedFieldV1 {
    pub field_id: String,
    pub field_path: String,
    pub text: String,
    pub source_update_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtContextSourceKind {
    Summary,
    SelectedRange,
    FieldDigest,
    OperationDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtContextSourceCitationV1 {
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub state_vector: String,
    pub latest_update_seq: u64,
    pub snapshot_id: Option<String>,
    pub source_kind: CrdtContextSourceKind,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtSummaryContextSliceV1 {
    pub text: String,
    pub byte_len: usize,
    pub truncated: bool,
    pub citation: CrdtContextSourceCitationV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtSelectedRangeContextSliceV1 {
    pub field_id: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub text: String,
    pub truncated: bool,
    pub citation: CrdtContextSourceCitationV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtFieldDigestV1 {
    pub field_id: String,
    pub field_path: String,
    pub content_sha256: String,
    pub byte_len: usize,
    pub citation: CrdtContextSourceCitationV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtOperationDeltaV1 {
    pub update_id: String,
    pub update_seq: u64,
    pub actor_id: String,
    pub session_id: String,
    pub delta_summary: String,
    pub citation: CrdtContextSourceCitationV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtContextSliceV1 {
    pub schema_id: String,
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,
    pub version: CrdtContextVersionRefV1,
    pub max_text_bytes: usize,
    pub max_operation_deltas: usize,
    pub summary: Option<CrdtSummaryContextSliceV1>,
    pub selected_ranges: Vec<CrdtSelectedRangeContextSliceV1>,
    pub field_digests: Vec<CrdtFieldDigestV1>,
    pub operation_deltas: Vec<CrdtOperationDeltaV1>,
    pub citations: Vec<CrdtContextSourceCitationV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtContextSliceValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_crdt_context_slice(
    request: &CrdtContextSliceRequestV1,
    fields: &[CrdtMaterializedFieldV1],
    updates: &[CrdtUpdateRecordV1],
) -> Result<CrdtContextSliceV1, Vec<CrdtContextSliceValidationError>> {
    validate_context_inputs(request, fields, updates)?;

    let mut citations = Vec::new();
    let summary = if request
        .requested_kinds
        .contains(&CrdtContextSliceKind::Summary)
    {
        let summary = build_summary_slice(request, fields);
        citations.push(summary.citation.clone());
        Some(summary)
    } else {
        None
    };

    let selected_ranges = if request
        .requested_kinds
        .contains(&CrdtContextSliceKind::SelectedRange)
    {
        build_selected_ranges(request, fields, &mut citations)?
    } else {
        Vec::new()
    };

    let field_digests = if request
        .requested_kinds
        .contains(&CrdtContextSliceKind::FieldDigest)
    {
        let digests = build_field_digests(request, fields);
        citations.extend(digests.iter().map(|digest| digest.citation.clone()));
        digests
    } else {
        Vec::new()
    };

    let operation_deltas = if request
        .requested_kinds
        .contains(&CrdtContextSliceKind::OperationDelta)
    {
        let deltas = build_operation_deltas(request, updates);
        citations.extend(deltas.iter().map(|delta| delta.citation.clone()));
        deltas
    } else {
        Vec::new()
    };

    let slice = CrdtContextSliceV1 {
        schema_id: CRDT_CONTEXT_SLICE_SCHEMA_ID.to_string(),
        workspace_id: request.identity.workspace_id.clone(),
        document_id: request.identity.document_id.clone(),
        crdt_document_id: request.identity.crdt_document_id.clone(),
        version: request.version.clone(),
        max_text_bytes: request.max_text_bytes,
        max_operation_deltas: request.max_operation_deltas,
        summary,
        selected_ranges,
        field_digests,
        operation_deltas,
        citations,
    };

    validate_crdt_context_slice(&slice)?;
    Ok(slice)
}

pub fn validate_crdt_context_slice(
    slice: &CrdtContextSliceV1,
) -> Result<(), Vec<CrdtContextSliceValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &slice.schema_id);
    require_non_empty(&mut errors, "workspace_id", &slice.workspace_id);
    require_non_empty(&mut errors, "document_id", &slice.document_id);
    require_non_empty(&mut errors, "crdt_document_id", &slice.crdt_document_id);
    require_non_empty(
        &mut errors,
        "version.state_vector",
        &slice.version.state_vector,
    );

    if slice.citations.is_empty() {
        errors.push(CrdtContextSliceValidationError {
            field: "citations",
            message: "at least one citation is required",
        });
    }

    for citation in &slice.citations {
        validate_citation(&mut errors, citation);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_context_inputs(
    request: &CrdtContextSliceRequestV1,
    fields: &[CrdtMaterializedFieldV1],
    updates: &[CrdtUpdateRecordV1],
) -> Result<(), Vec<CrdtContextSliceValidationError>> {
    let mut errors = Vec::new();

    if let Err(identity_errors) = validate_crdt_workspace_identity(&request.identity) {
        for identity_error in identity_errors {
            errors.push(CrdtContextSliceValidationError {
                field: identity_error.field,
                message: identity_error.message,
            });
        }
    }
    if request.requested_kinds.is_empty() {
        errors.push(CrdtContextSliceValidationError {
            field: "requested_kinds",
            message: "at least one context slice kind is required",
        });
    }
    if request.max_text_bytes == 0 {
        errors.push(CrdtContextSliceValidationError {
            field: "max_text_bytes",
            message: "text budget must be greater than zero",
        });
    }
    if request.max_operation_deltas == 0
        && request
            .requested_kinds
            .contains(&CrdtContextSliceKind::OperationDelta)
    {
        errors.push(CrdtContextSliceValidationError {
            field: "max_operation_deltas",
            message: "operation delta budget must be greater than zero",
        });
    }
    require_non_empty(
        &mut errors,
        "version.state_vector",
        &request.version.state_vector,
    );
    if request.version.latest_update_seq == 0 {
        errors.push(CrdtContextSliceValidationError {
            field: "version.latest_update_seq",
            message: "latest update sequence must be greater than zero",
        });
    }

    for field in fields {
        require_non_empty(&mut errors, "fields.field_id", &field.field_id);
        require_non_empty(&mut errors, "fields.field_path", &field.field_path);
        if field.source_update_ids.is_empty() {
            errors.push(CrdtContextSliceValidationError {
                field: "fields.source_update_ids",
                message: "field outputs must cite at least one source update id",
            });
        }
    }

    for update in updates {
        if let Err(update_errors) = validate_crdt_update_record(update) {
            for update_error in update_errors {
                errors.push(CrdtContextSliceValidationError {
                    field: update_error.field,
                    message: update_error.message,
                });
            }
        }
        if update.workspace_id != request.identity.workspace_id
            || update.document_id != request.identity.document_id
            || update.crdt_document_id != request.identity.crdt_document_id
        {
            errors.push(CrdtContextSliceValidationError {
                field: "updates.identity",
                message: "update identity must match context request identity",
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn build_summary_slice(
    request: &CrdtContextSliceRequestV1,
    fields: &[CrdtMaterializedFieldV1],
) -> CrdtSummaryContextSliceV1 {
    let mut ordered_fields = fields.to_vec();
    ordered_fields.sort_by(|left, right| left.field_path.cmp(&right.field_path));

    let mut text = String::new();
    let mut truncated = false;
    let mut source_ids = Vec::new();
    for field in &ordered_fields {
        source_ids.extend(field.source_update_ids.clone());
        let fragment = format!("{}: {}\n", field.field_path, field.text);
        if append_bounded(&mut text, &fragment, request.max_text_bytes) {
            truncated = true;
            break;
        }
    }

    CrdtSummaryContextSliceV1 {
        byte_len: text.len(),
        text,
        truncated,
        citation: citation(
            request,
            CrdtContextSourceKind::Summary,
            dedup_source_ids(source_ids),
        ),
    }
}

fn build_selected_ranges(
    request: &CrdtContextSliceRequestV1,
    fields: &[CrdtMaterializedFieldV1],
    citations: &mut Vec<CrdtContextSourceCitationV1>,
) -> Result<Vec<CrdtSelectedRangeContextSliceV1>, Vec<CrdtContextSliceValidationError>> {
    let Some(selection) = &request.selected_range else {
        return Ok(Vec::new());
    };
    let Some(field) = fields
        .iter()
        .find(|field| field.field_id == selection.field_id)
    else {
        return Err(vec![CrdtContextSliceValidationError {
            field: "selected_range.field_id",
            message: "selected field id was not found",
        }]);
    };

    let Some(raw_text) =
        utf8_slice_by_byte_range(&field.text, selection.start_byte, selection.end_byte)
    else {
        return Err(vec![CrdtContextSliceValidationError {
            field: "selected_range",
            message: "selected range must fit UTF-8 text boundaries",
        }]);
    };
    let (text, truncated) = bounded_prefix(&raw_text, request.max_text_bytes);
    let citation = citation(
        request,
        CrdtContextSourceKind::SelectedRange,
        field.source_update_ids.clone(),
    );
    citations.push(citation.clone());

    Ok(vec![CrdtSelectedRangeContextSliceV1 {
        field_id: field.field_id.clone(),
        start_byte: selection.start_byte,
        end_byte: selection.end_byte,
        text,
        truncated,
        citation,
    }])
}

fn build_field_digests(
    request: &CrdtContextSliceRequestV1,
    fields: &[CrdtMaterializedFieldV1],
) -> Vec<CrdtFieldDigestV1> {
    fields
        .iter()
        .map(|field| CrdtFieldDigestV1 {
            field_id: field.field_id.clone(),
            field_path: field.field_path.clone(),
            content_sha256: sha256_hex(field.text.as_bytes()),
            byte_len: field.text.len(),
            citation: citation(
                request,
                CrdtContextSourceKind::FieldDigest,
                field.source_update_ids.clone(),
            ),
        })
        .collect()
}

fn build_operation_deltas(
    request: &CrdtContextSliceRequestV1,
    updates: &[CrdtUpdateRecordV1],
) -> Vec<CrdtOperationDeltaV1> {
    let mut ordered_updates = updates.to_vec();
    ordered_updates.sort_by_key(|update| update.update_seq);
    let skip_count = ordered_updates
        .len()
        .saturating_sub(request.max_operation_deltas);

    ordered_updates
        .into_iter()
        .skip(skip_count)
        .map(|update| CrdtOperationDeltaV1 {
            update_id: update.update_id.clone(),
            update_seq: update.update_seq,
            actor_id: update.actor_id.clone(),
            session_id: update.session_id.clone(),
            delta_summary: format!(
                "update_seq={} actor={} session={}",
                update.update_seq, update.actor_id, update.session_id
            ),
            citation: citation(
                request,
                CrdtContextSourceKind::OperationDelta,
                vec![update.update_id],
            ),
        })
        .collect()
}

fn citation(
    request: &CrdtContextSliceRequestV1,
    source_kind: CrdtContextSourceKind,
    source_ids: Vec<String>,
) -> CrdtContextSourceCitationV1 {
    CrdtContextSourceCitationV1 {
        workspace_id: request.identity.workspace_id.clone(),
        document_id: request.identity.document_id.clone(),
        crdt_document_id: request.identity.crdt_document_id.clone(),
        state_vector: request.version.state_vector.clone(),
        latest_update_seq: request.version.latest_update_seq,
        snapshot_id: request.version.snapshot_id.clone(),
        source_kind,
        source_ids: dedup_source_ids(source_ids),
    }
}

fn validate_citation(
    errors: &mut Vec<CrdtContextSliceValidationError>,
    citation: &CrdtContextSourceCitationV1,
) {
    require_non_empty(errors, "citation.workspace_id", &citation.workspace_id);
    require_non_empty(errors, "citation.document_id", &citation.document_id);
    require_non_empty(
        errors,
        "citation.crdt_document_id",
        &citation.crdt_document_id,
    );
    require_non_empty(errors, "citation.state_vector", &citation.state_vector);
    if citation.latest_update_seq == 0 {
        errors.push(CrdtContextSliceValidationError {
            field: "citation.latest_update_seq",
            message: "citation latest update sequence must be greater than zero",
        });
    }
    if citation.source_ids.is_empty() {
        errors.push(CrdtContextSliceValidationError {
            field: "citation.source_ids",
            message: "citation must include at least one source id",
        });
    }
    if citation
        .source_ids
        .iter()
        .any(|source_id| source_id.trim().is_empty())
    {
        errors.push(CrdtContextSliceValidationError {
            field: "citation.source_ids",
            message: "citation source ids must not be empty",
        });
    }
}

fn append_bounded(target: &mut String, fragment: &str, max_bytes: usize) -> bool {
    let remaining = max_bytes.saturating_sub(target.len());
    if remaining == 0 {
        return true;
    }
    if fragment.len() <= remaining {
        target.push_str(fragment);
        false
    } else {
        let (prefix, _) = bounded_prefix(fragment, remaining);
        target.push_str(&prefix);
        true
    }
}

fn bounded_prefix(value: &str, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value.to_string(), false);
    }

    let mut end = 0;
    for (index, character) in value.char_indices() {
        let candidate_end = index + character.len_utf8();
        if candidate_end > max_bytes {
            break;
        }
        end = candidate_end;
    }
    (value[..end].to_string(), true)
}

fn utf8_slice_by_byte_range(value: &str, start: usize, end: usize) -> Option<String> {
    if start > end
        || end > value.len()
        || !value.is_char_boundary(start)
        || !value.is_char_boundary(end)
    {
        return None;
    }
    Some(value[start..end].to_string())
}

fn dedup_source_ids(source_ids: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for source_id in source_ids {
        if seen.insert(source_id.clone()) {
            deduped.push(source_id);
        }
    }
    deduped
}

fn require_non_empty(
    errors: &mut Vec<CrdtContextSliceValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(CrdtContextSliceValidationError {
            field,
            message: "value must not be empty",
        });
    }
}
