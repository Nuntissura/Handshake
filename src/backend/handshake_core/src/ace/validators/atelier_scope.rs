use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SELECTION_SCHEMA_V1: &str = "hsk.selection_range@v1";
const DOC_PATCHSET_SCHEMA_V1: &str = "hsk.doc_patchset@v1";
const BOUNDARY_NORMALIZATION_DISABLED: &str = "disabled";

#[derive(Debug, thiserror::Error)]
pub enum AtelierScopeError {
    #[error("ATELIER-LENS-VAL-SCOPE-001: {0}")]
    ScopeViolation(String),
    #[error("invalid selection: {0}")]
    InvalidSelection(String),
    #[error("invalid patchset: {0}")]
    InvalidPatchset(String),
    #[error("hash mismatch: {0}")]
    HashMismatch(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRangeV1 {
    pub schema_version: String,
    pub surface: String,
    pub coordinate_space: String,
    pub start_utf8: usize,
    pub end_utf8: usize,
    pub doc_preimage_sha256: String,
    pub selection_preimage_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPatchsetV1 {
    pub schema_version: String,
    pub doc_id: String,
    pub selection: SelectionRangeV1,
    pub boundary_normalization: String,
    pub ops: Vec<PatchOpV1>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeUtf8 {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOpV1 {
    ReplaceRange {
        range_utf8: RangeUtf8,
        insert_text: String,
    },
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn is_sha256_hex(value: &str) -> bool {
    let value = value.trim();
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn require_char_boundary(value: &str, offset: usize, label: &str) -> Result<(), AtelierScopeError> {
    if offset > value.len() {
        return Err(AtelierScopeError::InvalidSelection(format!(
            "{label} out of bounds: {offset} > {}",
            value.len()
        )));
    }
    if !value.is_char_boundary(offset) {
        return Err(AtelierScopeError::InvalidSelection(format!(
            "{label} is not a UTF-8 char boundary"
        )));
    }
    Ok(())
}

fn validate_selection_contract(selection: &SelectionRangeV1) -> Result<(), AtelierScopeError> {
    if selection.schema_version != SELECTION_SCHEMA_V1 {
        return Err(AtelierScopeError::InvalidSelection(format!(
            "unsupported selection schema_version: {}",
            selection.schema_version
        )));
    }
    if selection.start_utf8 >= selection.end_utf8 {
        return Err(AtelierScopeError::InvalidSelection(
            "selection must be non-empty".to_string(),
        ));
    }
    if !is_sha256_hex(&selection.doc_preimage_sha256) {
        return Err(AtelierScopeError::InvalidSelection(
            "doc_preimage_sha256 must be a sha256 hex".to_string(),
        ));
    }
    if !is_sha256_hex(&selection.selection_preimage_sha256) {
        return Err(AtelierScopeError::InvalidSelection(
            "selection_preimage_sha256 must be a sha256 hex".to_string(),
        ));
    }
    Ok(())
}

fn validate_patchset_contract(patchset: &DocPatchsetV1) -> Result<(), AtelierScopeError> {
    if patchset.schema_version != DOC_PATCHSET_SCHEMA_V1 {
        return Err(AtelierScopeError::InvalidPatchset(format!(
            "unsupported patchset schema_version: {}",
            patchset.schema_version
        )));
    }
    if patchset.boundary_normalization != BOUNDARY_NORMALIZATION_DISABLED {
        return Err(AtelierScopeError::InvalidPatchset(
            "boundary_normalization must be disabled in v1".to_string(),
        ));
    }
    if patchset.ops.is_empty() {
        return Err(AtelierScopeError::InvalidPatchset(
            "ops must be non-empty".to_string(),
        ));
    }
    validate_selection_contract(&patchset.selection)?;
    Ok(())
}

fn selection_equivalent(a: &SelectionRangeV1, b: &SelectionRangeV1) -> bool {
    a.schema_version == b.schema_version
        && a.start_utf8 == b.start_utf8
        && a.end_utf8 == b.end_utf8
        && a.doc_preimage_sha256 == b.doc_preimage_sha256
        && a.selection_preimage_sha256 == b.selection_preimage_sha256
        && a.surface == b.surface
        && a.coordinate_space == b.coordinate_space
}

pub fn validate_selection_preimage(
    doc_text: &str,
    selection: &SelectionRangeV1,
) -> Result<String, AtelierScopeError> {
    validate_selection_contract(selection)?;
    require_char_boundary(doc_text, selection.start_utf8, "start_utf8")?;
    require_char_boundary(doc_text, selection.end_utf8, "end_utf8")?;

    let doc_hash = sha256_hex(doc_text.as_bytes());
    if doc_hash != selection.doc_preimage_sha256 {
        return Err(AtelierScopeError::HashMismatch(
            "doc_preimage_sha256 mismatch".to_string(),
        ));
    }

    let selected = &doc_text[selection.start_utf8..selection.end_utf8];
    let selected_hash = sha256_hex(selected.as_bytes());
    if selected_hash != selection.selection_preimage_sha256 {
        return Err(AtelierScopeError::HashMismatch(
            "selection_preimage_sha256 mismatch".to_string(),
        ));
    }

    Ok(selected.to_string())
}

fn apply_ops_to_buffer(buffer: &mut String, ops: &[PatchOpV1]) -> Result<(), AtelierScopeError> {
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    for op in ops {
        match op {
            PatchOpV1::ReplaceRange {
                range_utf8,
                insert_text,
            } => {
                let start = range_utf8.start;
                let end = range_utf8.end;
                if start > end {
                    return Err(AtelierScopeError::InvalidPatchset(
                        "replace_range start must be <= end".to_string(),
                    ));
                }
                if end > buffer.len() {
                    return Err(AtelierScopeError::InvalidPatchset(
                        "replace_range out of bounds".to_string(),
                    ));
                }
                if !buffer.is_char_boundary(start) || !buffer.is_char_boundary(end) {
                    return Err(AtelierScopeError::InvalidPatchset(
                        "replace_range must align to UTF-8 char boundaries".to_string(),
                    ));
                }
                edits.push((start, end, insert_text.as_str()));
            }
        }
    }

    // Apply edits from right to left to keep byte offsets stable.
    edits.sort_by(|a, b| b.0.cmp(&a.0));
    for (start, end, insert_text) in edits {
        let prefix = &buffer[..start];
        let suffix = &buffer[end..];
        let mut next = String::with_capacity(prefix.len() + insert_text.len() + suffix.len());
        next.push_str(prefix);
        next.push_str(insert_text);
        next.push_str(suffix);
        *buffer = next;
    }

    Ok(())
}

pub fn apply_selection_bounded_patchsets(
    doc_text_before: &str,
    selection: &SelectionRangeV1,
    patchsets: &[DocPatchsetV1],
) -> Result<String, AtelierScopeError> {
    if patchsets.is_empty() {
        return Err(AtelierScopeError::InvalidPatchset(
            "suggestions_to_apply must be non-empty".to_string(),
        ));
    }

    let selection_preimage = validate_selection_preimage(doc_text_before, selection)?;
    let prefix = &doc_text_before[..selection.start_utf8];
    let suffix = &doc_text_before[selection.end_utf8..];

    let mut buffer = selection_preimage;

    for patchset in patchsets {
        validate_patchset_contract(patchset)?;
        if !selection_equivalent(&patchset.selection, selection) {
            return Err(AtelierScopeError::InvalidPatchset(
                "patchset selection does not match request selection".to_string(),
            ));
        }

        apply_ops_to_buffer(&mut buffer, &patchset.ops)?;
    }

    let mut doc_text_after = String::with_capacity(prefix.len() + buffer.len() + suffix.len());
    doc_text_after.push_str(prefix);
    doc_text_after.push_str(&buffer);
    doc_text_after.push_str(suffix);

    if !doc_text_after.starts_with(prefix) || !doc_text_after.ends_with(suffix) {
        return Err(AtelierScopeError::ScopeViolation(
            "outside-selection bytes were modified".to_string(),
        ));
    }

    Ok(doc_text_after)
}
