use serde::{Deserialize, Serialize};

use super::context_slice::CrdtMaterializedFieldV1;
use super::identity::{validate_crdt_workspace_identity, CrdtWorkspaceIdentityV1};

pub const CRDT_SCHEMA_GUARD_CONTRACT_SCHEMA_ID: &str = "hsk.kernel.crdt_schema_guard@1";
pub const CRDT_PROMOTION_VALIDATION_REPORT_SCHEMA_ID: &str =
    "hsk.kernel.crdt_promotion_validation_report@1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtMaterializedStateV1 {
    pub identity: CrdtWorkspaceIdentityV1,
    pub document_schema_id: String,
    pub state_vector: String,
    pub latest_update_seq: u64,
    pub fields: Vec<CrdtMaterializedFieldV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtSchemaFieldRequirementV1 {
    pub field_id: String,
    pub field_path: String,
    pub required: bool,
    pub max_text_bytes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtSchemaGuardContractV1 {
    pub schema_id: String,
    pub document_schema_id: String,
    pub schema_version: String,
    pub required_fields: Vec<CrdtSchemaFieldRequirementV1>,
    pub authorized_actor_ids: Vec<String>,
    pub authorized_actor_kinds: Vec<String>,
    pub allowed_source_update_ids: Vec<String>,
    pub expected_state_vector: String,
    pub expected_latest_update_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtPromotionValidationDecision {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrdtStateValidationErrorCode {
    StructuralInvalid,
    MissingRequiredField,
    SourceCitationMissing,
    UnknownSourceUpdate,
    UnauthorizedActor,
    SchemaDrift,
    FreshnessDrift,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtStateValidationError {
    pub code: CrdtStateValidationErrorCode,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtPromotionValidationReportV1 {
    pub schema_id: String,
    pub document_schema_id: String,
    pub state_vector: String,
    pub latest_update_seq: u64,
    pub decision: CrdtPromotionValidationDecision,
    pub promotion_allowed: bool,
    pub validation_errors: Vec<CrdtStateValidationError>,
}

pub fn validate_crdt_state_for_promotion(
    state: &CrdtMaterializedStateV1,
    guard: &CrdtSchemaGuardContractV1,
) -> CrdtPromotionValidationReportV1 {
    let mut errors = Vec::new();

    if let Err(identity_errors) = validate_crdt_workspace_identity(&state.identity) {
        for identity_error in identity_errors {
            errors.push(error(
                CrdtStateValidationErrorCode::StructuralInvalid,
                identity_error.field,
                identity_error.message,
            ));
        }
    }

    validate_schema_guard(guard, &mut errors);
    validate_state_shape(state, &mut errors);
    validate_authorization(state, guard, &mut errors);
    validate_schema_freshness(state, guard, &mut errors);
    validate_required_fields(state, guard, &mut errors);
    validate_source_updates(state, guard, &mut errors);

    let promotion_allowed = errors.is_empty();
    CrdtPromotionValidationReportV1 {
        schema_id: CRDT_PROMOTION_VALIDATION_REPORT_SCHEMA_ID.to_string(),
        document_schema_id: state.document_schema_id.clone(),
        state_vector: state.state_vector.clone(),
        latest_update_seq: state.latest_update_seq,
        decision: if promotion_allowed {
            CrdtPromotionValidationDecision::Allowed
        } else {
            CrdtPromotionValidationDecision::Denied
        },
        promotion_allowed,
        validation_errors: errors,
    }
}

fn validate_schema_guard(
    guard: &CrdtSchemaGuardContractV1,
    errors: &mut Vec<CrdtStateValidationError>,
) {
    require_non_empty(
        errors,
        CrdtStateValidationErrorCode::StructuralInvalid,
        "guard.schema_id",
        &guard.schema_id,
    );
    require_non_empty(
        errors,
        CrdtStateValidationErrorCode::SchemaDrift,
        "guard.document_schema_id",
        &guard.document_schema_id,
    );
    require_non_empty(
        errors,
        CrdtStateValidationErrorCode::SchemaDrift,
        "guard.schema_version",
        &guard.schema_version,
    );
    require_non_empty(
        errors,
        CrdtStateValidationErrorCode::FreshnessDrift,
        "guard.expected_state_vector",
        &guard.expected_state_vector,
    );
    if guard.schema_id != CRDT_SCHEMA_GUARD_CONTRACT_SCHEMA_ID {
        errors.push(error(
            CrdtStateValidationErrorCode::SchemaDrift,
            "guard.schema_id",
            "unexpected CRDT schema guard schema",
        ));
    }
    if guard.required_fields.is_empty() {
        errors.push(error(
            CrdtStateValidationErrorCode::StructuralInvalid,
            "guard.required_fields",
            "at least one field requirement is required",
        ));
    }
}

fn validate_state_shape(
    state: &CrdtMaterializedStateV1,
    errors: &mut Vec<CrdtStateValidationError>,
) {
    require_non_empty(
        errors,
        CrdtStateValidationErrorCode::SchemaDrift,
        "state.document_schema_id",
        &state.document_schema_id,
    );
    require_non_empty(
        errors,
        CrdtStateValidationErrorCode::FreshnessDrift,
        "state.state_vector",
        &state.state_vector,
    );
    if state.latest_update_seq == 0 {
        errors.push(error(
            CrdtStateValidationErrorCode::FreshnessDrift,
            "state.latest_update_seq",
            "latest update sequence must be greater than zero",
        ));
    }
    if state.fields.is_empty() {
        errors.push(error(
            CrdtStateValidationErrorCode::StructuralInvalid,
            "state.fields",
            "materialized state requires at least one field",
        ));
    }
}

fn validate_authorization(
    state: &CrdtMaterializedStateV1,
    guard: &CrdtSchemaGuardContractV1,
    errors: &mut Vec<CrdtStateValidationError>,
) {
    if !guard.authorized_actor_ids.is_empty()
        && !guard
            .authorized_actor_ids
            .contains(&state.identity.actor_id)
    {
        errors.push(error(
            CrdtStateValidationErrorCode::UnauthorizedActor,
            "identity.actor_id",
            "actor is not authorized to promote this CRDT state",
        ));
    }
    if !guard.authorized_actor_kinds.is_empty()
        && !guard
            .authorized_actor_kinds
            .contains(&state.identity.actor_kind)
    {
        errors.push(error(
            CrdtStateValidationErrorCode::UnauthorizedActor,
            "identity.actor_kind",
            "actor kind is not authorized to promote this CRDT state",
        ));
    }
}

fn validate_schema_freshness(
    state: &CrdtMaterializedStateV1,
    guard: &CrdtSchemaGuardContractV1,
    errors: &mut Vec<CrdtStateValidationError>,
) {
    if state.document_schema_id != guard.document_schema_id
        || state.identity.document_schema_id != guard.document_schema_id
    {
        errors.push(error(
            CrdtStateValidationErrorCode::SchemaDrift,
            "document_schema_id",
            "materialized state schema must match the promotion guard schema",
        ));
    }
    if state.state_vector != guard.expected_state_vector
        || state.latest_update_seq != guard.expected_latest_update_seq
    {
        errors.push(error(
            CrdtStateValidationErrorCode::FreshnessDrift,
            "state_vector",
            "materialized state must match the expected CRDT version",
        ));
    }
}

fn validate_required_fields(
    state: &CrdtMaterializedStateV1,
    guard: &CrdtSchemaGuardContractV1,
    errors: &mut Vec<CrdtStateValidationError>,
) {
    for requirement in &guard.required_fields {
        let Some(field) = state
            .fields
            .iter()
            .find(|field| field.field_id == requirement.field_id)
        else {
            if requirement.required {
                errors.push(error(
                    CrdtStateValidationErrorCode::MissingRequiredField,
                    &requirement.field_id,
                    "required materialized field is missing",
                ));
            }
            continue;
        };

        if field.field_path != requirement.field_path {
            errors.push(error(
                CrdtStateValidationErrorCode::SchemaDrift,
                &field.field_id,
                "materialized field path does not match schema requirement",
            ));
        }
        if requirement.required && field.text.trim().is_empty() {
            errors.push(error(
                CrdtStateValidationErrorCode::StructuralInvalid,
                &field.field_id,
                "required materialized field text must not be empty",
            ));
        }
        if let Some(max_text_bytes) = requirement.max_text_bytes {
            if field.text.len() > max_text_bytes {
                errors.push(error(
                    CrdtStateValidationErrorCode::StructuralInvalid,
                    &field.field_id,
                    "materialized field exceeds schema text budget",
                ));
            }
        }
    }
}

fn validate_source_updates(
    state: &CrdtMaterializedStateV1,
    guard: &CrdtSchemaGuardContractV1,
    errors: &mut Vec<CrdtStateValidationError>,
) {
    for field in &state.fields {
        if field.source_update_ids.is_empty() {
            errors.push(error(
                CrdtStateValidationErrorCode::SourceCitationMissing,
                &field.field_id,
                "materialized field must cite CRDT source update ids",
            ));
            continue;
        }

        for source_update_id in &field.source_update_ids {
            if source_update_id.trim().is_empty() {
                errors.push(error(
                    CrdtStateValidationErrorCode::SourceCitationMissing,
                    &field.field_id,
                    "source update id must not be empty",
                ));
            } else if !guard.allowed_source_update_ids.is_empty()
                && !guard.allowed_source_update_ids.contains(source_update_id)
            {
                errors.push(error(
                    CrdtStateValidationErrorCode::UnknownSourceUpdate,
                    &field.field_id,
                    "source update id is outside the guarded update set",
                ));
            }
        }
    }
}

fn require_non_empty(
    errors: &mut Vec<CrdtStateValidationError>,
    code: CrdtStateValidationErrorCode,
    field: &str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(code, field, "value must not be empty"));
    }
}

fn error(
    code: CrdtStateValidationErrorCode,
    field: &str,
    message: &str,
) -> CrdtStateValidationError {
    CrdtStateValidationError {
        code,
        field: field.to_string(),
        message: message.to_string(),
    }
}
