//! WP-CKC-posekit-overhaul SurrealDB port — CKC `intake_facial` lane router (MT-061).
//!
//! Routes ported from the reference `api/atelier.rs` (feat/WP-CKC-posekit-overhaul):
//!
//! - `POST /atelier/intake/batches/:batch_id/classifications` (MT-017 batch triage plan with
//!   dataset-mining metadata; the backend enumerates the canonical item set)
//! - `POST /atelier/intake/items/:item_id/classification` (single-item triage decision)
//! - `POST /atelier/intake/batches/:batch_id/facial/analyze` (MT-019 native Facial analysis)
//! - `POST /atelier/intake/batches/:batch_id/facial/review/session` (MT-028 review session)
//! - `GET  /atelier/facial/features` (native Facial feature registry + command route map)
//! - `GET  /atelier/facial/artifacts/read?artifact_ref=` (Facial-owned JSON artifact read-back)
//! - `POST /atelier/facial/review/{claims,decisions,status,montage,export}` (MT-029/031/055
//!   durable command-outcome envelopes)
//! - `POST /atelier/contact-sheets/export` (MT-018 contact sheet SVG + receipt)
//!
//! Storage authority is the embedded SurrealDB store through `AtelierStore`; the ArtifactStore
//! (`storage::artifacts`) is the only blob tier. Facial review claims/decisions/sessions are
//! artifact-backed exactly as in the reference: the session-scoped commands rediscover persisted
//! claim and decision receipts from the L1 layer so a caller that supplies no refs still recovers
//! server-authoritative state. Shared helpers (store constructor, error envelope, actor header)
//! come from `super::atelier`; the model-operation lease guard is a lane-local copy of the
//! reference helper because `api/atelier.rs` does not export one on this tree.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::sync::OnceLock;
use uuid::Uuid;

use super::atelier::{
    artifact_byte_read_error, atelier_error, atelier_store, calling_actor, header_str,
    internal_error, ErrorResponse, HSK_HEADER_ACTOR_ID,
};
use crate::ace::ArtifactHandle;
use crate::atelier::contact_sheet::{
    generate_contact_sheet_export, ContactSheetExport, ContactSheetExportItem, ContactSheetLayout,
    GenerateContactSheetRequest, CONTACT_SHEET_EXPORT_SCHEMA_ID,
};
use crate::atelier::facial::{
    generate_facial_ingest_analysis, FacialIngestAnalysisExport, FacialIngestAnalysisItem,
    FacialIngestAnalysisSummary, GenerateFacialIngestAnalysisRequest,
    FACIAL_INGEST_ANALYSIS_RECEIPT_SCHEMA_ID, FACIAL_INGEST_ANALYSIS_SCHEMA_ID,
};
use crate::atelier::facial_native::review::{
    build_review_export_manifest, build_review_montage, build_review_session, build_review_status,
    claim_review_shard, record_review_decision, BuildFacialReviewExportRequest,
    BuildFacialReviewMontageRequest, BuildFacialReviewSessionRequest, FacialReviewClaimReceipt,
    FacialReviewClaimRequest, FacialReviewDecisionReceipt, FacialReviewDecisionRequest,
    FacialReviewExportManifest, FacialReviewSessionArtifact, FACIAL_REVIEW_CLAIM_SCHEMA_ID,
    FACIAL_REVIEW_DECISION_SCHEMA_ID, FACIAL_REVIEW_EXPORT_SCHEMA_ID,
    FACIAL_REVIEW_MONTAGE_SCHEMA_ID, FACIAL_REVIEW_SESSION_SCHEMA_ID,
    FACIAL_REVIEW_STATUS_SCHEMA_ID,
};
use crate::atelier::facial_native::{
    facial_feature_registry, FacialNativeFeature, FACIAL_NATIVE_REGISTRY_SCHEMA_ID,
};
use crate::atelier::intake::{
    ApplyIntakeBatchClassificationOverride, ApplyIntakeBatchClassificationsRequest,
    ApplyIntakeClassificationRequest, IntakeBatchClassificationFailure,
    IntakeClassificationApplyResult, IntakeClassificationMetadata, IntakeItem, IntakeLane,
};
use crate::atelier::model_lease::ModelLeaseRecord;
use crate::atelier::refs::{collection_ref, media_asset_ref};
use crate::atelier::AtelierError;
use crate::kernel::role_mailbox_claim_lease::{ClaimLeaseState, RoleMailboxClaimMode};
use crate::storage::artifacts::{
    artifact_root_rel, artifact_store_root, read_artifact_manifest,
    read_file_artifact_with_manifest, resolve_workspace_root, sha256_hex,
    validate_artifact_content_hash, write_file_artifact, ArtifactClassification, ArtifactLayer,
    ArtifactManifest, ArtifactPayloadKind,
};
use crate::storage::EntityRef;
use crate::AppState;

/// Request header naming the caller kind; `operator` bypasses the model-operation lease guard.
const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
/// Request header carrying the model-operation lease claim id (MT-022 guard).
const HSK_HEADER_MODEL_LEASE_ID: &str = "x-hsk-model-lease-id";
/// Request header carrying the lease-holder session id; required with the lease id.
const HSK_HEADER_SESSION_ID: &str = "x-hsk-session-id";

const FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID: &str = "hsk.atelier.facial_api.command_response@1";
const FACIAL_API_COMMAND_RECEIPT_SCHEMA_ID: &str = "hsk.atelier.facial_api.command_receipt@1";
const FACIAL_FEATURES_SCHEMA_ID: &str = "hsk.atelier.facial.features@1";
const FACIAL_ARTIFACT_READ_SCHEMA_ID: &str = "hsk.atelier.facial.artifact_read@1";
const DEFAULT_FACIAL_REVIEW_PROFILE: &str = "quality+dedupe+identity+review";
/// Cap on the per-row preview echoed by the batch classification response.
const INTAKE_BATCH_APPLY_PREVIEW_LIMIT: usize = 64;

static FACIAL_REVIEW_CLAIM_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/atelier/contact-sheets/export", post(export_contact_sheet))
        .route(
            "/atelier/intake/batches/:batch_id/classifications",
            post(apply_intake_batch_classifications),
        )
        .route(
            "/atelier/intake/items/:item_id/classification",
            post(apply_intake_item_classification),
        )
        .route(
            "/atelier/intake/batches/:batch_id/facial/analyze",
            post(analyze_intake_batch_facial),
        )
        .route(
            "/atelier/intake/batches/:batch_id/facial/review/session",
            post(create_facial_review_session),
        )
        .route("/atelier/facial/features", get(list_facial_features))
        .route("/atelier/facial/artifacts/read", get(read_facial_artifact))
        .route(
            "/atelier/facial/review/claims",
            post(claim_facial_review_shard),
        )
        .route(
            "/atelier/facial/review/decisions",
            post(record_facial_review_decision),
        )
        .route(
            "/atelier/facial/review/status",
            post(build_facial_review_status),
        )
        .route(
            "/atelier/facial/review/montage",
            post(build_facial_review_montage),
        )
        .route(
            "/atelier/facial/review/export",
            post(build_facial_review_export),
        )
        .with_state(state)
}

type ApiError = (StatusCode, Json<ErrorResponse>);

/// Stable hash of caller-controlled text; the actor ref and thread segments never carry the raw
/// value into a manifest or thread id.
fn text_hash(text: &str) -> String {
    sha256_hex(text.as_bytes())
}

// ---------------------------------------------------------------------------
// Model-operation lease guard (MT-022), lane-local copy of the reference helper.
// ---------------------------------------------------------------------------

fn lease_state_token(state: ClaimLeaseState) -> &'static str {
    match state {
        ClaimLeaseState::Unclaimed => "unclaimed",
        ClaimLeaseState::Active => "active",
        ClaimLeaseState::Released => "released",
        ClaimLeaseState::Expired => "expired",
        ClaimLeaseState::TakenOver => "taken_over",
    }
}

fn claim_mode_token(mode: RoleMailboxClaimMode) -> &'static str {
    match mode {
        RoleMailboxClaimMode::ExclusiveLease => "exclusive_lease",
        RoleMailboxClaimMode::SharedObserver => "shared_observer",
        RoleMailboxClaimMode::BroadcastRequest => "broadcast_request",
        RoleMailboxClaimMode::HandoffReservation => "handoff_reservation",
    }
}

/// Guard a model-operation mutation: an agent caller must present an active exclusive (or
/// handoff-reservation) lease bound to `expected_thread_id`; `x-hsk-actor-kind: operator` with
/// `x-hsk-actor-id: operator` bypasses the lease.
async fn validate_model_operation_lease_if_present(
    state: &AppState,
    headers: &HeaderMap,
    actor: &str,
    expected_thread_id: Option<&str>,
) -> Result<Option<ModelLeaseRecord>, ApiError> {
    if header_str(headers, HSK_HEADER_MODEL_LEASE_ID).is_some() {
        return validate_model_operation_lease_required(state, headers, actor, expected_thread_id)
            .await
            .map(Some);
    }
    match header_str(headers, HSK_HEADER_ACTOR_KIND) {
        Some("operator") if actor == "operator" => Ok(None),
        Some("operator") => Err(atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_ACTOR_KIND}=operator is reserved for {HSK_HEADER_ACTOR_ID}=operator"
        )))),
        Some(other) => Err(atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation guarded mutations unless {HSK_HEADER_ACTOR_KIND}=operator; got {other}"
        )))),
        None => Err(atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation guarded mutations unless {HSK_HEADER_ACTOR_KIND}=operator"
        )))),
    }
}

async fn validate_model_operation_lease_required(
    state: &AppState,
    headers: &HeaderMap,
    actor: &str,
    expected_thread_id: Option<&str>,
) -> Result<ModelLeaseRecord, ApiError> {
    let raw_claim_id = header_str(headers, HSK_HEADER_MODEL_LEASE_ID).ok_or_else(|| {
        atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation mutations"
        )))
    })?;
    let session_id = header_str(headers, HSK_HEADER_SESSION_ID).ok_or_else(|| {
        atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_SESSION_ID} is required with {HSK_HEADER_MODEL_LEASE_ID}"
        )))
    })?;
    let claim_id = Uuid::parse_str(raw_claim_id).map_err(|_| {
        atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} must be a UUID"
        )))
    })?;
    let store = atelier_store(state);
    let record = store
        .get_model_lease(claim_id)
        .await
        .map_err(atelier_error)?;
    if record.actor_id != actor || record.session_id != session_id {
        return Err(atelier_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} is held by actor={} session={}",
            record.actor_id, record.session_id
        ))));
    }
    if record.effective_state != ClaimLeaseState::Active || record.lease_expired {
        return Err(atelier_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} is not active: state={} expired={}",
            lease_state_token(record.effective_state),
            record.lease_expired
        ))));
    }
    if !matches!(
        record.claim_mode,
        RoleMailboxClaimMode::ExclusiveLease | RoleMailboxClaimMode::HandoffReservation
    ) {
        return Err(atelier_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} has non-mutating claim_mode={}",
            claim_mode_token(record.claim_mode)
        ))));
    }
    if let Some(expected_thread_id) = expected_thread_id {
        if record.thread_id != expected_thread_id {
            return Err(atelier_error(AtelierError::Conflict(format!(
                "model-operation lease {claim_id} targets thread_id={} but mutation requires thread_id={expected_thread_id}",
                record.thread_id
            ))));
        }
    }
    Ok(record)
}

fn stable_thread_segment(value: &str) -> String {
    text_hash(value)
}

fn contact_sheet_model_operation_thread_id(source_kind: &str, source_ref: &str) -> String {
    format!(
        "atelier.contact-sheet.{}.{}",
        stable_thread_segment(source_kind),
        stable_thread_segment(source_ref)
    )
}

fn intake_batch_model_operation_thread_id(batch_id: Uuid) -> String {
    format!("atelier.intake.batch.{batch_id}")
}

fn facial_review_session_model_operation_thread_id(session_id: &str) -> String {
    format!("atelier.facial.review.session.{session_id}")
}

// ---------------------------------------------------------------------------
// Intake classification (MT-017 / MT-031).
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct IntakeItemResponse {
    item_id: Uuid,
    source_path: String,
    file_name: String,
    lane: String,
    byte_len: i64,
}

fn intake_item_response(item: IntakeItem) -> IntakeItemResponse {
    IntakeItemResponse {
        item_id: item.item_id,
        source_path: item.source_path,
        file_name: item.file_name,
        lane: item.lane.as_str().to_owned(),
        byte_len: item.byte_len,
    }
}

#[derive(Debug, Deserialize)]
struct ApplyIntakeItemClassificationRequest {
    lane: String,
    reason: Option<String>,
    metadata: Option<IntakeClassificationMetadata>,
}

#[derive(Debug, Serialize)]
struct IntakeClassificationApplyResponse {
    item: IntakeItemResponse,
    asset_id: Option<Uuid>,
    media_ref: Option<String>,
    collection_id: Option<Uuid>,
    collection_ref: Option<String>,
    collection_inserted: bool,
    requested_by: String,
}

#[derive(Debug, Deserialize)]
struct ApplyIntakeBatchClassificationOverrideRequest {
    item_id: Uuid,
    lane: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApplyIntakeBatchClassificationsApiRequest {
    default_lane: String,
    default_reason: Option<String>,
    metadata: Option<IntakeClassificationMetadata>,
    #[serde(default)]
    overrides: Vec<ApplyIntakeBatchClassificationOverrideRequest>,
}

#[derive(Debug, Serialize)]
struct IntakeBatchClassificationFailureResponse {
    item_id: Uuid,
    index: usize,
    error: String,
}

#[derive(Debug, Serialize)]
struct IntakeBatchClassificationApplyResponse {
    batch_id: Uuid,
    total_item_count: usize,
    applied_count: usize,
    applied: Vec<IntakeClassificationApplyResponse>,
    failed: Option<IntakeBatchClassificationFailureResponse>,
    requested_by: String,
}

fn intake_classification_apply_response(
    applied: IntakeClassificationApplyResult,
    requested_by: &str,
) -> IntakeClassificationApplyResponse {
    IntakeClassificationApplyResponse {
        item: intake_item_response(applied.item),
        asset_id: applied.asset_id,
        media_ref: applied.asset_id.map(media_asset_ref),
        collection_id: applied.collection_id,
        collection_ref: applied.collection_id.map(collection_ref),
        collection_inserted: applied.collection_inserted,
        requested_by: requested_by.to_owned(),
    }
}

fn intake_batch_classification_failure_response(
    failed: IntakeBatchClassificationFailure,
) -> IntakeBatchClassificationFailureResponse {
    IntakeBatchClassificationFailureResponse {
        item_id: failed.item_id,
        index: failed.index,
        error: failed.error,
    }
}

async fn intake_batch_id_for_item(state: &AppState, item_id: Uuid) -> Result<Uuid, ApiError> {
    atelier_store(state)
        .get_intake_item_by_id(item_id)
        .await
        .map_err(atelier_error)?
        .map(|item| item.batch_id)
        .ok_or_else(|| atelier_error(AtelierError::NotFound(format!("intake item_id={item_id}"))))
}

/// POST /atelier/intake/batches/:batch_id/classifications — persist a full-batch triage plan.
async fn apply_intake_batch_classifications(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch_id): Path<Uuid>,
    Json(payload): Json<ApplyIntakeBatchClassificationsApiRequest>,
) -> Result<Json<IntakeBatchClassificationApplyResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = intake_batch_model_operation_thread_id(batch_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let default_lane = IntakeLane::parse(&payload.default_lane).map_err(atelier_error)?;
    let mut overrides = Vec::with_capacity(payload.overrides.len());
    for override_row in payload.overrides {
        overrides.push(ApplyIntakeBatchClassificationOverride {
            item_id: override_row.item_id,
            lane: IntakeLane::parse(&override_row.lane).map_err(atelier_error)?,
            reason: override_row.reason,
        });
    }
    let store = atelier_store(&state);
    let applied = store
        .apply_intake_batch_classifications(&ApplyIntakeBatchClassificationsRequest {
            batch_id,
            default_lane,
            default_reason: payload.default_reason,
            requested_by: actor.clone(),
            metadata: payload.metadata,
            overrides,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/intake/batches/:batch_id/classifications",
        status = "ok",
        actor = %actor,
        batch_id = %batch_id,
        applied_count = applied.applied.len(),
        total_item_count = applied.total_item_count,
        failed = applied.failed.is_some(),
        "apply intake batch classifications"
    );

    let applied_count = applied.applied.len();
    Ok(Json(IntakeBatchClassificationApplyResponse {
        batch_id: applied.batch_id,
        total_item_count: applied.total_item_count,
        applied_count,
        applied: applied
            .applied
            .into_iter()
            .take(INTAKE_BATCH_APPLY_PREVIEW_LIMIT)
            .map(|row| intake_classification_apply_response(row, &actor))
            .collect(),
        failed: applied
            .failed
            .map(intake_batch_classification_failure_response),
        requested_by: actor,
    }))
}

/// POST /atelier/intake/items/:item_id/classification — persist one item triage decision.
async fn apply_intake_item_classification(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
    Json(payload): Json<ApplyIntakeItemClassificationRequest>,
) -> Result<Json<IntakeClassificationApplyResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let batch_id = intake_batch_id_for_item(&state, item_id).await?;
    let expected_thread_id = intake_batch_model_operation_thread_id(batch_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let lane = IntakeLane::parse(&payload.lane).map_err(atelier_error)?;
    let store = atelier_store(&state);
    let applied = store
        .apply_intake_classification(&ApplyIntakeClassificationRequest {
            item_id,
            lane,
            reason: payload.reason,
            requested_by: Some(actor.clone()),
            metadata: payload.metadata,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/intake/items/:item_id/classification",
        status = "ok",
        actor = %actor,
        item_id = %applied.item.item_id,
        lane = applied.item.lane.as_str(),
        "apply intake item classification"
    );

    Ok(Json(intake_classification_apply_response(applied, &actor)))
}

// ---------------------------------------------------------------------------
// Contact sheets (MT-018).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ContactSheetExportApiRequest {
    source_kind: String,
    source_ref: String,
    rows: usize,
    columns: usize,
    dpi: usize,
    include_labels: Option<bool>,
    thumbnail_fit: Option<String>,
    output_path: Option<String>,
    items: Vec<ContactSheetExportItem>,
}

#[derive(Debug, Serialize)]
struct ContactSheetArtifactResponse {
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
    byte_len: u64,
    mime: String,
    file_name: String,
}

#[derive(Debug, Serialize)]
struct ContactSheetExportResponse {
    schema_id: String,
    source_kind: String,
    source_ref: String,
    thumbnail_fit: String,
    output_path: Option<String>,
    layout: ContactSheetLayout,
    source_items: Vec<ContactSheetExportItem>,
    item_count: usize,
    rendered_item_count: usize,
    omitted_item_count: usize,
    include_labels: bool,
    svg_sha256: String,
    receipt_sha256: String,
    content_hash: String,
    receipt_ref: String,
    svg_artifact: ContactSheetArtifactResponse,
    receipt_artifact: ContactSheetArtifactResponse,
}

/// POST /atelier/contact-sheets/export — native Rust contact sheet SVG + receipt into ArtifactStore.
async fn export_contact_sheet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ContactSheetExportApiRequest>,
) -> Result<Json<ContactSheetExportResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id =
        contact_sheet_model_operation_thread_id(&payload.source_kind, &payload.source_ref);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let validated_items = validate_contact_sheet_source_items(&state, &payload).await?;
    let export = generate_contact_sheet_export(GenerateContactSheetRequest {
        source_kind: payload.source_kind,
        source_ref: payload.source_ref,
        rows: payload.rows,
        columns: payload.columns,
        dpi: payload.dpi,
        include_labels: payload.include_labels.unwrap_or(true),
        thumbnail_fit: payload
            .thumbnail_fit
            .unwrap_or_else(|| "contain".to_owned()),
        output_path: payload.output_path,
        items: validated_items,
    })
    .map_err(|err| atelier_error(AtelierError::Validation(err)))?;
    let response_content_hash = export.content_hash.clone();
    let svg_artifact = write_contact_sheet_artifact(
        &export,
        export.svg.as_bytes(),
        &export.svg_sha256,
        "image/svg+xml",
        "atelier-contact-sheet.svg",
        &actor,
        &response_content_hash,
        &[],
    )?;
    let receipt_artifact = write_contact_sheet_receipt_artifact(
        &export,
        &svg_artifact,
        &actor,
        &response_content_hash,
    )?;
    let source_items = export
        .receipt_json
        .get("source_items")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<ContactSheetExportItem>>(value).ok())
        .unwrap_or_default();
    let receipt_sha256 = receipt_artifact.content_hash.clone();
    let response = ContactSheetExportResponse {
        schema_id: CONTACT_SHEET_EXPORT_SCHEMA_ID.to_owned(),
        source_kind: export.source_kind.clone(),
        source_ref: export.source_ref.clone(),
        thumbnail_fit: export.thumbnail_fit.clone(),
        output_path: export.output_path.clone(),
        layout: export.layout.clone(),
        source_items,
        item_count: export.item_count,
        rendered_item_count: export.rendered_item_count,
        omitted_item_count: export.omitted_item_count,
        include_labels: export.include_labels,
        svg_sha256: export.svg_sha256.clone(),
        receipt_sha256,
        content_hash: response_content_hash,
        receipt_ref: receipt_artifact.artifact_ref.clone(),
        svg_artifact,
        receipt_artifact,
    };
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/contact-sheets/export",
        status = "ok",
        actor = %actor,
        source_kind = %response.source_kind,
        source_ref = %response.source_ref,
        item_count = response.item_count,
        rendered_item_count = response.rendered_item_count,
        omitted_item_count = response.omitted_item_count,
        svg_artifact_ref = %response.svg_artifact.artifact_ref,
        receipt_ref = %response.receipt_ref,
        "export Atelier contact sheet"
    );
    Ok(Json(response))
}

/// The contact sheet lineage is grounded in the canonical intake item set: every requested item
/// must belong to the ingest batch AND carry the stored `source_path`, so a receipt can never
/// record a falsified source ref.
async fn validate_contact_sheet_source_items(
    state: &AppState,
    payload: &ContactSheetExportApiRequest,
) -> Result<Vec<ContactSheetExportItem>, ApiError> {
    if payload.source_kind.trim() != "ingest_batch" {
        return Err(atelier_error(AtelierError::Validation(format!(
            "contact sheet source_kind {:?} is not supported yet; supported source_kind=ingest_batch",
            payload.source_kind
        ))));
    }
    let batch_id = Uuid::parse_str(payload.source_ref.trim()).map_err(|_| {
        atelier_error(AtelierError::Validation(
            "contact sheet ingest_batch source_ref must be a backend batch UUID".to_owned(),
        ))
    })?;
    let store = atelier_store(state);
    let canonical_items = store
        .list_intake_items(batch_id, None)
        .await
        .map_err(atelier_error)?;
    let canonical = canonical_items
        .into_iter()
        .map(|item| (item.item_id.to_string(), item.source_path))
        .collect::<std::collections::HashMap<_, _>>();
    if canonical.is_empty() {
        return Err(atelier_error(AtelierError::Validation(format!(
            "contact sheet ingest_batch source_ref has no items: {batch_id}"
        ))));
    }
    for item in &payload.items {
        match canonical.get(&item.item_id) {
            Some(source_path) if source_path == &item.source_ref => {}
            Some(source_path) => {
                return Err(atelier_error(AtelierError::Validation(format!(
                    "contact sheet item source_ref mismatch for item_id={}: expected {}, got {}",
                    item.item_id, source_path, item.source_ref
                ))));
            }
            None => {
                return Err(atelier_error(AtelierError::Validation(format!(
                    "contact sheet item_id={} does not belong to ingest batch {}",
                    item.item_id, batch_id
                ))));
            }
        }
    }
    Ok(payload
        .items
        .iter()
        .map(|item| ContactSheetExportItem {
            item_id: item.item_id.clone(),
            label: item.label.clone(),
            source_ref: item.source_ref.clone(),
            media_ref: None,
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
fn write_contact_sheet_artifact(
    export: &ContactSheetExport,
    payload_bytes: &[u8],
    content_hash: &str,
    mime: &str,
    file_name: &str,
    actor: &str,
    export_entity_id: &str,
    source_artifact_refs: &[ArtifactHandle],
) -> Result<ContactSheetArtifactResponse, ApiError> {
    let workspace_root = resolve_workspace_root().map_err(internal_error)?;
    let artifact_id = Uuid::now_v7();
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: mime.to_owned(),
        filename_hint: Some(file_name.to_owned()),
        created_at: Utc::now(),
        created_by_job_id: None,
        source_entity_refs: contact_sheet_source_entity_refs(export, actor, export_entity_id),
        source_artifact_refs: source_artifact_refs.to_vec(),
        content_hash: content_hash.to_owned(),
        size_bytes: payload_bytes.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(true),
        hash_basis: Some(format!(
            "{}|{}|{}|{}|payload={}|export={}",
            CONTACT_SHEET_EXPORT_SCHEMA_ID,
            export.source_kind,
            export.source_ref,
            export.layout.cell_count,
            content_hash,
            export.content_hash
        )),
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(&workspace_root, &manifest, payload_bytes).map_err(internal_error)?;
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, artifact_id)
        .map_err(internal_error)?;
    let root = artifact_root_rel(ArtifactLayer::L1, artifact_id);
    Ok(ContactSheetArtifactResponse {
        artifact_ref: format!("artifact://{root}/payload"),
        manifest_ref: format!("artifact://{root}/artifact.json"),
        content_hash: content_hash.to_owned(),
        byte_len: payload_bytes.len() as u64,
        mime: mime.to_owned(),
        file_name: file_name.to_owned(),
    })
}

fn write_contact_sheet_receipt_artifact(
    export: &ContactSheetExport,
    svg_artifact: &ContactSheetArtifactResponse,
    actor: &str,
    export_entity_id: &str,
) -> Result<ContactSheetArtifactResponse, ApiError> {
    let mut receipt = export.receipt_json.clone();
    if let Some(receipt) = receipt.as_object_mut() {
        receipt.insert(
            "actor_ref".to_owned(),
            serde_json::json!(format!("actor://sha256/{}", text_hash(actor))),
        );
        receipt.insert(
            "svg_artifact_ref".to_owned(),
            serde_json::json!(svg_artifact.artifact_ref.clone()),
        );
        receipt.insert(
            "svg_manifest_ref".to_owned(),
            serde_json::json!(svg_artifact.manifest_ref.clone()),
        );
    }
    let payload_bytes = serde_json::to_vec(&receipt).map_err(internal_error)?;
    let content_hash = sha256_hex(&payload_bytes);
    write_contact_sheet_artifact(
        export,
        &payload_bytes,
        &content_hash,
        "application/json",
        "atelier-contact-sheet-receipt.json",
        actor,
        export_entity_id,
        &[contact_sheet_artifact_handle(svg_artifact)],
    )
}

fn contact_sheet_source_entity_refs(
    export: &ContactSheetExport,
    actor: &str,
    export_entity_id: &str,
) -> Vec<EntityRef> {
    vec![
        EntityRef {
            entity_kind: "contact_sheet_source_kind".to_owned(),
            entity_id: export.source_kind.clone(),
        },
        EntityRef {
            entity_kind: "contact_sheet_source_ref".to_owned(),
            entity_id: export.source_ref.clone(),
        },
        EntityRef {
            entity_kind: "actor_sha256".to_owned(),
            entity_id: text_hash(actor),
        },
        EntityRef {
            entity_kind: "contact_sheet_export".to_owned(),
            entity_id: export_entity_id.to_owned(),
        },
    ]
}

fn contact_sheet_artifact_handle(artifact: &ContactSheetArtifactResponse) -> ArtifactHandle {
    artifact_handle_from_ref(&artifact.artifact_ref)
        .unwrap_or_else(|| ArtifactHandle::new(Uuid::now_v7(), artifact.artifact_ref.clone()))
}

// ---------------------------------------------------------------------------
// Facial ingest analysis (MT-019).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct FacialIngestAnalysisApiRequest {
    profile: String,
}

#[derive(Debug, Serialize)]
struct FacialIngestArtifactResponse {
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
    byte_len: u64,
    mime: String,
    file_name: String,
}

#[derive(Debug, Serialize)]
struct FacialIngestAnalysisResponse {
    schema_id: String,
    batch_id: Uuid,
    profile: String,
    profile_tokens: Vec<String>,
    item_count: usize,
    summary: FacialIngestAnalysisSummary,
    analysis_sha256: String,
    receipt_sha256: String,
    content_hash: String,
    receipt_ref: String,
    analysis_artifact: FacialIngestArtifactResponse,
    receipt_artifact: FacialIngestArtifactResponse,
}

/// Resolve an `artifact://.handshake/...` source ref to the on-disk payload so the analysis can
/// probe image dimensions without copying bytes; any other ref stays external and undecoded.
fn resolve_facial_ingest_local_path(source_ref: &str) -> Option<String> {
    let relative = source_ref.trim().strip_prefix("artifact://.handshake/")?;
    if relative.is_empty()
        || relative
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return None;
    }
    let workspace_root = resolve_workspace_root().ok()?;
    let mut path = workspace_root.join(".handshake");
    for segment in relative.split('/') {
        path.push(segment);
    }
    Some(path.to_string_lossy().into_owned())
}

fn facial_analysis_item(item: IntakeItem) -> FacialIngestAnalysisItem {
    let source_path = item.source_path;
    let local_path_hint = resolve_facial_ingest_local_path(&source_path);
    FacialIngestAnalysisItem {
        item_id: item.item_id.to_string(),
        source_ref: source_path,
        local_path_hint,
        file_name: item.file_name,
        byte_len: item.byte_len,
        content_hash: item.content_hash,
        lane: item.lane.as_str().to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn write_facial_ingest_artifact(
    export: &FacialIngestAnalysisExport,
    payload_bytes: &[u8],
    content_hash: &str,
    mime: &str,
    file_name: &str,
    actor: &str,
    export_entity_id: &str,
    source_artifact_refs: &[ArtifactHandle],
) -> Result<FacialIngestArtifactResponse, ApiError> {
    let workspace_root = resolve_workspace_root().map_err(internal_error)?;
    let artifact_id = Uuid::now_v7();
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: mime.to_owned(),
        filename_hint: Some(file_name.to_owned()),
        created_at: Utc::now(),
        created_by_job_id: None,
        source_entity_refs: facial_ingest_source_entity_refs(export, actor, export_entity_id),
        source_artifact_refs: source_artifact_refs.to_vec(),
        content_hash: content_hash.to_owned(),
        size_bytes: payload_bytes.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(true),
        hash_basis: Some(format!(
            "{}|{}|{}|payload={}|export={}",
            FACIAL_INGEST_ANALYSIS_SCHEMA_ID,
            export.batch_id,
            export.profile,
            content_hash,
            export.content_hash
        )),
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(&workspace_root, &manifest, payload_bytes).map_err(internal_error)?;
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, artifact_id)
        .map_err(internal_error)?;
    let root = artifact_root_rel(ArtifactLayer::L1, artifact_id);
    Ok(FacialIngestArtifactResponse {
        artifact_ref: format!("artifact://{root}/payload"),
        manifest_ref: format!("artifact://{root}/artifact.json"),
        content_hash: content_hash.to_owned(),
        byte_len: payload_bytes.len() as u64,
        mime: mime.to_owned(),
        file_name: file_name.to_owned(),
    })
}

fn write_facial_ingest_receipt_artifact(
    export: &FacialIngestAnalysisExport,
    analysis_artifact: &FacialIngestArtifactResponse,
    actor: &str,
) -> Result<FacialIngestArtifactResponse, ApiError> {
    let mut receipt = export.receipt_json.clone();
    if let Some(receipt) = receipt.as_object_mut() {
        receipt.insert(
            "actor_ref".to_owned(),
            serde_json::json!(format!("actor://sha256/{}", text_hash(actor))),
        );
        receipt.insert(
            "analysis_artifact_ref".to_owned(),
            serde_json::json!(analysis_artifact.artifact_ref.clone()),
        );
        receipt.insert(
            "analysis_manifest_ref".to_owned(),
            serde_json::json!(analysis_artifact.manifest_ref.clone()),
        );
        if let Some(native_run) = receipt
            .get_mut("native_run")
            .and_then(|value| value.as_object_mut())
        {
            native_run.insert(
                "artifact_refs".to_owned(),
                serde_json::json!([analysis_artifact.artifact_ref.clone()]),
            );
            native_run.insert(
                "manifest_refs".to_owned(),
                serde_json::json!([analysis_artifact.manifest_ref.clone()]),
            );
        }
    }
    let payload_bytes = serde_json::to_vec(&receipt).map_err(internal_error)?;
    let content_hash = sha256_hex(&payload_bytes);
    write_facial_ingest_artifact(
        export,
        &payload_bytes,
        &content_hash,
        "application/json",
        "atelier-facial-ingest-analysis-receipt.json",
        actor,
        &export.content_hash,
        &[facial_ingest_artifact_handle(analysis_artifact)],
    )
}

fn facial_ingest_source_entity_refs(
    export: &FacialIngestAnalysisExport,
    actor: &str,
    export_entity_id: &str,
) -> Vec<EntityRef> {
    vec![
        EntityRef {
            entity_kind: "intake_batch".to_owned(),
            entity_id: export.batch_id.clone(),
        },
        EntityRef {
            entity_kind: "facial_profile".to_owned(),
            entity_id: export.profile.clone(),
        },
        EntityRef {
            entity_kind: "actor_sha256".to_owned(),
            entity_id: text_hash(actor),
        },
        EntityRef {
            entity_kind: "facial_ingest_analysis".to_owned(),
            entity_id: export_entity_id.to_owned(),
        },
    ]
}

fn facial_ingest_artifact_handle(artifact: &FacialIngestArtifactResponse) -> ArtifactHandle {
    artifact_handle_from_ref(&artifact.artifact_ref)
        .unwrap_or_else(|| ArtifactHandle::new(Uuid::now_v7(), artifact.artifact_ref.clone()))
}

/// POST /atelier/intake/batches/:batch_id/facial/analyze — native Facial-derived Ingest analysis.
async fn analyze_intake_batch_facial(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch_id): Path<Uuid>,
    Json(payload): Json<FacialIngestAnalysisApiRequest>,
) -> Result<Json<FacialIngestAnalysisResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = intake_batch_model_operation_thread_id(batch_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let store = atelier_store(&state);
    let analysis_items = store
        .list_intake_items(batch_id, None)
        .await
        .map_err(atelier_error)?
        .into_iter()
        .map(facial_analysis_item)
        .collect::<Vec<_>>();
    let export = generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
        batch_id: batch_id.to_string(),
        profile: payload.profile,
        requested_by: actor.clone(),
        items: analysis_items,
    })
    .map_err(|err| atelier_error(AtelierError::Validation(err)))?;

    let analysis_payload = serde_json::to_vec(&export.analysis_json).map_err(internal_error)?;
    let analysis_artifact = write_facial_ingest_artifact(
        &export,
        &analysis_payload,
        &export.analysis_sha256,
        "application/json",
        "atelier-facial-ingest-analysis.json",
        &actor,
        &export.content_hash,
        &[],
    )?;
    let receipt_artifact =
        write_facial_ingest_receipt_artifact(&export, &analysis_artifact, &actor)?;
    let receipt_sha256 = receipt_artifact.content_hash.clone();
    let mut response_summary = export.summary.clone();
    response_summary.native_run.artifact_refs = vec![
        analysis_artifact.artifact_ref.clone(),
        receipt_artifact.artifact_ref.clone(),
    ];
    response_summary.native_run.manifest_refs = vec![
        analysis_artifact.manifest_ref.clone(),
        receipt_artifact.manifest_ref.clone(),
    ];
    let response = FacialIngestAnalysisResponse {
        schema_id: FACIAL_INGEST_ANALYSIS_SCHEMA_ID.to_owned(),
        batch_id,
        profile: export.profile.clone(),
        profile_tokens: export.profile_tokens.clone(),
        item_count: export.item_count,
        summary: response_summary,
        analysis_sha256: export.analysis_sha256.clone(),
        receipt_sha256,
        content_hash: export.content_hash.clone(),
        receipt_ref: receipt_artifact.artifact_ref.clone(),
        analysis_artifact,
        receipt_artifact,
    };

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/intake/batches/:batch_id/facial/analyze",
        status = "ok",
        actor = %actor,
        batch_id = %batch_id,
        profile = %response.profile,
        item_count = response.item_count,
        analysis_artifact_ref = %response.analysis_artifact.artifact_ref,
        receipt_ref = %response.receipt_ref,
        "analyze intake batch with native Facial bridge"
    );
    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// Facial feature registry + artifact read-back.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct FacialArtifactReadQuery {
    artifact_ref: String,
}

#[derive(Debug, Serialize)]
struct FacialArtifactReadResponse {
    schema_id: String,
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
    byte_len: u64,
    mime: String,
    file_name: Option<String>,
    payload_schema_id: Option<String>,
    payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct FacialFeatureListResponse {
    schema_id: String,
    registry_schema_id: String,
    feature_count: usize,
    features: Vec<FacialNativeFeature>,
    command_routes: Vec<FacialCommandRoute>,
}

#[derive(Debug, Serialize)]
struct FacialCommandRoute {
    command: &'static str,
    method: &'static str,
    path: &'static str,
    response_schema_id: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    result_schema_id: Option<&'static str>,
    output_schema_id: &'static str,
}

/// GET /atelier/facial/features — native Facial feature registry plus route map for agents.
async fn list_facial_features() -> Json<FacialFeatureListResponse> {
    let features = facial_feature_registry();
    Json(FacialFeatureListResponse {
        schema_id: FACIAL_FEATURES_SCHEMA_ID.to_owned(),
        registry_schema_id: FACIAL_NATIVE_REGISTRY_SCHEMA_ID.to_owned(),
        feature_count: features.len(),
        features,
        command_routes: facial_command_routes(),
    })
}

/// GET /atelier/facial/artifacts/read?artifact_ref=... — read one Facial-owned JSON ArtifactStore
/// payload. Bytes are served through `read_artifact_manifest` + `read_file_artifact_with_manifest`
/// (hash- and size-verified); a manifest that is not on disk is 404, a non-Facial manifest is 400.
async fn read_facial_artifact(
    Query(query): Query<FacialArtifactReadQuery>,
) -> Result<Json<FacialArtifactReadResponse>, ApiError> {
    read_facial_json_artifact_value(&query.artifact_ref).map(Json)
}

fn facial_command_routes() -> Vec<FacialCommandRoute> {
    vec![
        FacialCommandRoute {
            command: "atelier.facial.features.list",
            method: "GET",
            path: "/atelier/facial/features",
            response_schema_id: FACIAL_FEATURES_SCHEMA_ID,
            result_schema_id: None,
            output_schema_id: FACIAL_FEATURES_SCHEMA_ID,
        },
        FacialCommandRoute {
            command: "atelier.facial.artifacts.read",
            method: "GET",
            path: "/atelier/facial/artifacts/read?artifact_ref=...",
            response_schema_id: FACIAL_ARTIFACT_READ_SCHEMA_ID,
            result_schema_id: None,
            output_schema_id: FACIAL_ARTIFACT_READ_SCHEMA_ID,
        },
        FacialCommandRoute {
            command: "atelier.facial.review.session.create",
            method: "POST",
            path: "/atelier/intake/batches/:batch_id/facial/review/session",
            response_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
            result_schema_id: Some(FACIAL_REVIEW_SESSION_SCHEMA_ID),
            output_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
        },
        FacialCommandRoute {
            command: "atelier.facial.review.claim",
            method: "POST",
            path: "/atelier/facial/review/claims",
            response_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
            result_schema_id: Some(FACIAL_REVIEW_CLAIM_SCHEMA_ID),
            output_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
        },
        FacialCommandRoute {
            command: "atelier.facial.review.decision",
            method: "POST",
            path: "/atelier/facial/review/decisions",
            response_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
            result_schema_id: Some(FACIAL_REVIEW_DECISION_SCHEMA_ID),
            output_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
        },
        FacialCommandRoute {
            command: "atelier.facial.review.status",
            method: "POST",
            path: "/atelier/facial/review/status",
            response_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
            result_schema_id: Some(FACIAL_REVIEW_STATUS_SCHEMA_ID),
            output_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
        },
        FacialCommandRoute {
            command: "atelier.facial.review.montage",
            method: "POST",
            path: "/atelier/facial/review/montage",
            response_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
            result_schema_id: Some(FACIAL_REVIEW_MONTAGE_SCHEMA_ID),
            output_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
        },
        FacialCommandRoute {
            command: "atelier.facial.review.export",
            method: "POST",
            path: "/atelier/facial/review/export",
            response_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
            result_schema_id: Some(FACIAL_REVIEW_EXPORT_SCHEMA_ID),
            output_schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID,
        },
    ]
}

fn parse_artifact_ref(artifact_ref: &str) -> Result<(ArtifactLayer, Uuid), ApiError> {
    let trimmed = artifact_ref.trim();
    let Some(rest) = trimmed.strip_prefix("artifact://.handshake/artifacts/") else {
        return Err(atelier_error(AtelierError::Validation(
            "facial artifact_ref must be an ArtifactStore payload ref".to_owned(),
        )));
    };
    let parts = rest.split('/').collect::<Vec<_>>();
    if parts.len() != 3 || parts[2] != "payload" {
        return Err(atelier_error(AtelierError::Validation(
            "facial artifact_ref must point to an ArtifactStore payload".to_owned(),
        )));
    }
    let layer = match parts[0] {
        "L1" => ArtifactLayer::L1,
        other => {
            return Err(atelier_error(AtelierError::Validation(format!(
                "facial artifact reads only support L1 command artifacts, got {other}"
            ))));
        }
    };
    let artifact_id = Uuid::parse_str(parts[1]).map_err(|err| {
        atelier_error(AtelierError::Validation(format!(
            "invalid facial artifact UUID: {err}"
        )))
    })?;
    Ok((layer, artifact_id))
}

fn read_facial_json_artifact_value(
    artifact_ref: &str,
) -> Result<FacialArtifactReadResponse, ApiError> {
    let (layer, artifact_id) = parse_artifact_ref(artifact_ref)?;
    let workspace_root = resolve_workspace_root().map_err(internal_error)?;
    let manifest = read_artifact_manifest(&workspace_root, layer, artifact_id)
        .map_err(artifact_byte_read_error)?;
    if !facial_artifact_manifest_is_readable(&manifest) {
        return Err(atelier_error(AtelierError::Validation(
            "facial artifact read only accepts Facial-owned L1 JSON artifacts".to_owned(),
        )));
    }
    if manifest.mime != "application/json" {
        return Err(atelier_error(AtelierError::Validation(format!(
            "facial artifact read expected application/json, got {}",
            manifest.mime
        ))));
    }
    let payload_bytes = read_file_artifact_with_manifest(&workspace_root, &manifest)
        .map_err(artifact_byte_read_error)?;
    let payload: serde_json::Value =
        serde_json::from_slice(&payload_bytes).map_err(internal_error)?;
    let payload_schema_id = payload
        .get("schema_id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            atelier_error(AtelierError::Validation(
                "facial artifact payload missing schema_id".to_owned(),
            ))
        })?;
    if !facial_schema_id_is_readable(&payload_schema_id) {
        return Err(atelier_error(AtelierError::Validation(format!(
            "facial artifact schema is not readable through this endpoint: {payload_schema_id}"
        ))));
    }
    let root = artifact_root_rel(layer, artifact_id);
    Ok(FacialArtifactReadResponse {
        schema_id: FACIAL_ARTIFACT_READ_SCHEMA_ID.to_owned(),
        artifact_ref: format!("artifact://{root}/payload"),
        manifest_ref: format!("artifact://{root}/artifact.json"),
        content_hash: manifest.content_hash,
        byte_len: manifest.size_bytes,
        mime: manifest.mime,
        file_name: manifest.filename_hint,
        payload_schema_id: Some(payload_schema_id),
        payload,
    })
}

fn read_facial_json_artifact_as<T: serde::de::DeserializeOwned>(
    artifact_ref: &str,
    expected_schema_id: &str,
) -> Result<T, ApiError> {
    let response = read_facial_json_artifact_value(artifact_ref)?;
    let actual_schema = response
        .payload
        .get("schema_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if actual_schema != expected_schema_id {
        return Err(atelier_error(AtelierError::Validation(format!(
            "facial artifact schema mismatch: expected {expected_schema_id}, got {actual_schema}"
        ))));
    }
    serde_json::from_value(response.payload).map_err(internal_error)
}

fn read_facial_json_artifact_refs_as<T: serde::de::DeserializeOwned>(
    artifact_refs: &[String],
    expected_schema_id: &str,
) -> Result<Vec<T>, ApiError> {
    artifact_refs
        .iter()
        .map(|artifact_ref| read_facial_json_artifact_as(artifact_ref, expected_schema_id))
        .collect()
}

fn facial_artifact_manifest_is_readable(manifest: &ArtifactManifest) -> bool {
    manifest.layer == ArtifactLayer::L1
        && manifest.kind == ArtifactPayloadKind::File
        && manifest.mime == "application/json"
        && manifest.classification == ArtifactClassification::Low
        && manifest.exportable
        && manifest.source_entity_refs.iter().any(|entity| {
            matches!(
                entity.entity_kind.as_str(),
                "facial_command" | "facial_schema" | "facial_ingest_analysis"
            )
        })
}

fn facial_schema_id_is_readable(schema_id: &str) -> bool {
    matches!(
        schema_id,
        FACIAL_INGEST_ANALYSIS_SCHEMA_ID
            | FACIAL_INGEST_ANALYSIS_RECEIPT_SCHEMA_ID
            | FACIAL_REVIEW_SESSION_SCHEMA_ID
            | FACIAL_REVIEW_CLAIM_SCHEMA_ID
            | FACIAL_REVIEW_DECISION_SCHEMA_ID
            | FACIAL_REVIEW_STATUS_SCHEMA_ID
            | FACIAL_REVIEW_MONTAGE_SCHEMA_ID
            | FACIAL_REVIEW_EXPORT_SCHEMA_ID
            | FACIAL_API_COMMAND_RECEIPT_SCHEMA_ID
    )
}

fn artifact_handle_from_ref(artifact_ref: &str) -> Option<ArtifactHandle> {
    parse_artifact_ref(artifact_ref)
        .ok()
        .map(|(_, artifact_id)| ArtifactHandle::new(artifact_id, artifact_ref.to_owned()))
}

fn artifact_handles_from_refs<S: AsRef<str>>(artifact_refs: &[S]) -> Vec<ArtifactHandle> {
    artifact_refs
        .iter()
        .filter_map(|artifact_ref| artifact_handle_from_ref(artifact_ref.as_ref()))
        .collect()
}

// ---------------------------------------------------------------------------
// Facial review commands (MT-028 / MT-029 / MT-031 / MT-055).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
struct FacialCommandArtifactResponse {
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
    byte_len: u64,
    mime: String,
    file_name: String,
}

/// MT-031/MT-055: durable Facial command *outcome* envelope. Models every post-context outcome
/// of a Facial command (`succeeded`, `degraded`, `blocked`, `error`) and is always returned at
/// HTTP 200 so models can recover from command-level failures without scraping generic
/// transport error bodies. For `blocked`/`error` the `result` is JSON null and
/// `result_artifact` is omitted, but a durable receipt is ALWAYS written.
#[derive(Debug, Serialize)]
struct FacialCommandOutcomeResponse {
    schema_id: String,
    command: String,
    status: String,
    actor: String,
    result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result_artifact: Option<FacialCommandArtifactResponse>,
    receipt_ref: String,
    receipt_artifact: FacialCommandArtifactResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    degraded_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_hint: Option<String>,
}

#[derive(Debug, Serialize)]
struct FacialReviewSessionResult {
    session: FacialReviewSessionArtifact,
    analysis_artifact: FacialIngestArtifactResponse,
    analysis_receipt_artifact: FacialIngestArtifactResponse,
}

#[derive(Debug, Deserialize)]
struct FacialReviewSessionApiRequest {
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    shard_count: Option<usize>,
    #[serde(default)]
    claim_ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct FacialReviewClaimApiRequest {
    session_artifact_ref: String,
    #[serde(default)]
    existing_claim_artifact_refs: Vec<String>,
    #[serde(default)]
    decision_artifact_refs: Vec<String>,
    #[serde(default)]
    shard: Option<usize>,
    #[serde(default)]
    claim_ttl_seconds: Option<u64>,
    #[serde(default)]
    steal_expired: Option<bool>,
    #[serde(default)]
    claimed_at_utc: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FacialReviewDecisionApiRequest {
    session_artifact_ref: String,
    claim_artifact_ref: String,
    item_id: String,
    decision: String,
    reason: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    decided_at_utc: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FacialReviewStatusApiRequest {
    session_artifact_ref: String,
    #[serde(default)]
    claim_artifact_refs: Vec<String>,
    #[serde(default)]
    decision_artifact_refs: Vec<String>,
    #[serde(default)]
    now_utc: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FacialReviewMontageApiRequest {
    session_artifact_ref: String,
    #[serde(default)]
    decision_artifact_refs: Vec<String>,
    #[serde(default)]
    page: Option<usize>,
    #[serde(default)]
    columns: Option<usize>,
    #[serde(default)]
    rows: Option<usize>,
    #[serde(default)]
    tile_width: Option<u32>,
    #[serde(default)]
    tile_height: Option<u32>,
    #[serde(default)]
    decision_filter: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FacialReviewExportApiRequest {
    session_artifact_ref: String,
    #[serde(default)]
    decision_artifact_refs: Vec<String>,
    dataset_name: String,
    #[serde(default)]
    repeats: Option<u32>,
    #[serde(default)]
    allow_partial: Option<bool>,
    output_root_ref: String,
    #[serde(default)]
    exported_at_utc: Option<String>,
}

/// The canonical item set for a review session: the batch must exist (404 otherwise) even when
/// it has no items, so an unknown batch is a pre-context transport error and an empty batch is a
/// post-context command outcome.
async fn list_facial_analysis_items_for_batch(
    state: &AppState,
    batch_id: Uuid,
) -> Result<Vec<FacialIngestAnalysisItem>, ApiError> {
    let store = atelier_store(state);
    store
        .get_intake_batch_by_id(batch_id)
        .await
        .map_err(atelier_error)?
        .ok_or_else(|| atelier_error(AtelierError::NotFound(format!("intake batch {batch_id}"))))?;
    Ok(store
        .list_intake_items(batch_id, None)
        .await
        .map_err(atelier_error)?
        .into_iter()
        .map(facial_analysis_item)
        .collect())
}

/// POST /atelier/intake/batches/:batch_id/facial/review/session — create a review session over
/// the canonical batch item set.
async fn create_facial_review_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch_id): Path<Uuid>,
    Json(payload): Json<FacialReviewSessionApiRequest>,
) -> Result<Json<FacialCommandOutcomeResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = intake_batch_model_operation_thread_id(batch_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let profile = payload
        .profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_FACIAL_REVIEW_PROFILE)
        .to_owned();
    let analysis_items = list_facial_analysis_items_for_batch(&state, batch_id).await?;
    let export = match generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
        batch_id: batch_id.to_string(),
        profile,
        requested_by: actor.clone(),
        items: analysis_items,
    }) {
        Ok(export) => export,
        Err(_domain_detail) => {
            return facial_review_command_outcome(
                "atelier.facial.review.session.create",
                &actor,
                "error",
                None,
                None,
                None,
                Some("session_command_failed".to_owned()),
                Some(
                    "Session creation could not generate review analysis from the batch. \
                     Verify the intake batch has canonical items and rerun session creation."
                        .to_owned(),
                ),
                Vec::new(),
                &[],
            );
        }
    };

    let analysis_payload = serde_json::to_vec(&export.analysis_json).map_err(internal_error)?;
    let analysis_artifact = write_facial_ingest_artifact(
        &export,
        &analysis_payload,
        &export.analysis_sha256,
        "application/json",
        "atelier-facial-ingest-analysis.json",
        &actor,
        &export.content_hash,
        &[],
    )?;
    let analysis_receipt_artifact =
        write_facial_ingest_receipt_artifact(&export, &analysis_artifact, &actor)?;

    let session = match build_review_session(BuildFacialReviewSessionRequest {
        batch_id: batch_id.to_string(),
        analysis_sha256: export.analysis_sha256.clone(),
        receipt_sha256: analysis_receipt_artifact.content_hash.clone(),
        created_by: actor.clone(),
        created_at_utc: Utc::now().to_rfc3339(),
        shard_count: payload.shard_count.unwrap_or(4),
        claim_ttl_seconds: payload.claim_ttl_seconds,
        rows: export.rows.clone(),
    }) {
        Ok(session) => session,
        Err(_domain_detail) => {
            return facial_review_command_outcome(
                "atelier.facial.review.session.create",
                &actor,
                "error",
                None,
                None,
                None,
                Some("session_command_failed".to_owned()),
                Some(
                    "Session creation could not build review rows from the batch analysis. \
                     Verify the intake batch has canonical items and rerun session creation."
                        .to_owned(),
                ),
                Vec::new(),
                &[],
            );
        }
    };

    let result = FacialReviewSessionResult {
        session,
        analysis_artifact,
        analysis_receipt_artifact,
    };
    let session_artifact_payload = result.session.clone();
    facial_review_command_response(
        "atelier.facial.review.session.create",
        &actor,
        FACIAL_REVIEW_SESSION_SCHEMA_ID,
        "atelier-facial-review-session.json",
        &session_artifact_payload,
        result,
        &[],
    )
}

/// POST /atelier/facial/review/claims — claim one review shard using persisted session/decision
/// refs. Claims serialise on a process-wide lock so two agents racing for the same shard observe
/// each other's persisted receipts.
async fn claim_facial_review_shard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FacialReviewClaimApiRequest>,
) -> Result<Json<FacialCommandOutcomeResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let session: FacialReviewSessionArtifact = read_facial_json_artifact_as(
        &payload.session_artifact_ref,
        FACIAL_REVIEW_SESSION_SCHEMA_ID,
    )?;
    let expected_thread_id = facial_review_session_model_operation_thread_id(&session.session_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let _claim_guard = facial_review_claim_lock().lock().await;
    let provided_claims: Vec<FacialReviewClaimReceipt> = read_facial_json_artifact_refs_as(
        &payload.existing_claim_artifact_refs,
        FACIAL_REVIEW_CLAIM_SCHEMA_ID,
    )?;
    let discovered_claims = discover_facial_review_claims(&session.session_id)?;
    let claims = merge_facial_review_claims(discovered_claims, provided_claims);
    let provided_decisions: Vec<FacialReviewDecisionReceipt> = read_facial_json_artifact_refs_as(
        &payload.decision_artifact_refs,
        FACIAL_REVIEW_DECISION_SCHEMA_ID,
    )?;
    let discovered_decisions = discover_facial_review_decisions(&session.session_id)?;
    let decisions = merge_facial_review_decisions(discovered_decisions, provided_decisions);
    let sources = artifact_handles_from_refs(
        &[
            std::slice::from_ref(&payload.session_artifact_ref),
            payload.existing_claim_artifact_refs.as_slice(),
            payload.decision_artifact_refs.as_slice(),
        ]
        .concat(),
    );
    let claim = match claim_review_shard(
        &session,
        &claims,
        &decisions,
        FacialReviewClaimRequest {
            actor: actor.clone(),
            claimed_at_utc: payload
                .claimed_at_utc
                .unwrap_or_else(|| Utc::now().to_rfc3339()),
            shard: payload.shard,
            claim_ttl_seconds: payload.claim_ttl_seconds,
            steal_expired: payload.steal_expired.unwrap_or(false),
        },
    ) {
        Ok(claim) => claim,
        Err(domain_detail) => {
            let error_code = facial_claim_error_code(&domain_detail);
            return facial_review_command_outcome(
                "atelier.facial.review.claim",
                &actor,
                "blocked",
                None,
                None,
                None,
                Some(error_code.to_owned()),
                Some(
                    "Claim was not granted. Refresh review status, inspect active and expired \
                     claims, adjust shard or steal_expired when appropriate, then retry."
                        .to_owned(),
                ),
                Vec::new(),
                &sources,
            );
        }
    };
    let claim_artifact_payload = claim.clone();
    facial_review_command_response(
        "atelier.facial.review.claim",
        &actor,
        FACIAL_REVIEW_CLAIM_SCHEMA_ID,
        "atelier-facial-review-claim.json",
        &claim_artifact_payload,
        claim,
        &sources,
    )
}

/// POST /atelier/facial/review/decisions — persist one actor decision receipt from a claim.
async fn record_facial_review_decision(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FacialReviewDecisionApiRequest>,
) -> Result<Json<FacialCommandOutcomeResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let session: FacialReviewSessionArtifact = read_facial_json_artifact_as(
        &payload.session_artifact_ref,
        FACIAL_REVIEW_SESSION_SCHEMA_ID,
    )?;
    let expected_thread_id = facial_review_session_model_operation_thread_id(&session.session_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let claim: FacialReviewClaimReceipt =
        read_facial_json_artifact_as(&payload.claim_artifact_ref, FACIAL_REVIEW_CLAIM_SCHEMA_ID)?;
    let sources = artifact_handles_from_refs(&[
        payload.session_artifact_ref.as_str(),
        payload.claim_artifact_ref.as_str(),
    ]);
    let decision = match record_review_decision(
        &session,
        &claim,
        FacialReviewDecisionRequest {
            actor: actor.clone(),
            decided_at_utc: payload
                .decided_at_utc
                .unwrap_or_else(|| Utc::now().to_rfc3339()),
            item_id: payload.item_id,
            claim_id: claim.claim_id.clone(),
            decision: payload.decision,
            reason: payload.reason,
            tags: payload.tags,
            notes: payload.notes,
        },
    ) {
        Ok(decision) => decision,
        Err(domain_detail) => {
            let error_code = facial_decision_error_code(&domain_detail);
            return facial_review_command_outcome(
                "atelier.facial.review.decision",
                &actor,
                "error",
                None,
                None,
                None,
                Some(error_code.to_owned()),
                Some(
                    "Decision was not recorded. Verify the claim owner, claim freshness, item id, \
                     verdict, and reason, then retry the decision command."
                        .to_owned(),
                ),
                Vec::new(),
                &sources,
            );
        }
    };
    let decision_artifact_payload = decision.clone();
    facial_review_command_response(
        "atelier.facial.review.decision",
        &actor,
        FACIAL_REVIEW_DECISION_SCHEMA_ID,
        "atelier-facial-review-decision.json",
        &decision_artifact_payload,
        decision,
        &sources,
    )
}

/// POST /atelier/facial/review/status — replay claim/decision refs into recoverable status.
async fn build_facial_review_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FacialReviewStatusApiRequest>,
) -> Result<Json<FacialCommandOutcomeResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let session: FacialReviewSessionArtifact = read_facial_json_artifact_as(
        &payload.session_artifact_ref,
        FACIAL_REVIEW_SESSION_SCHEMA_ID,
    )?;
    let expected_thread_id = facial_review_session_model_operation_thread_id(&session.session_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let provided_claims: Vec<FacialReviewClaimReceipt> = read_facial_json_artifact_refs_as(
        &payload.claim_artifact_refs,
        FACIAL_REVIEW_CLAIM_SCHEMA_ID,
    )?;
    let discovered_claims = discover_facial_review_claims(&session.session_id)?;
    let claims = merge_facial_review_claims(discovered_claims, provided_claims);
    let provided_decisions: Vec<FacialReviewDecisionReceipt> = read_facial_json_artifact_refs_as(
        &payload.decision_artifact_refs,
        FACIAL_REVIEW_DECISION_SCHEMA_ID,
    )?;
    let discovered_decisions = discover_facial_review_decisions(&session.session_id)?;
    let decisions = merge_facial_review_decisions(discovered_decisions, provided_decisions);
    let sources = artifact_handles_from_refs(
        &[
            std::slice::from_ref(&payload.session_artifact_ref),
            payload.claim_artifact_refs.as_slice(),
            payload.decision_artifact_refs.as_slice(),
        ]
        .concat(),
    );
    let status = match build_review_status(
        &session,
        &claims,
        &decisions,
        &payload.now_utc.unwrap_or_else(|| Utc::now().to_rfc3339()),
    ) {
        Ok(status) => status,
        Err(_domain_detail) => {
            return facial_review_command_outcome(
                "atelier.facial.review.status",
                &actor,
                "error",
                None,
                None,
                None,
                Some("status_command_failed".to_owned()),
                Some(
                    "Status replay could not be built. Verify now_utc and the supplied claim or \
                     decision refs, then rerun status replay."
                        .to_owned(),
                ),
                Vec::new(),
                &sources,
            );
        }
    };
    let status_artifact_payload = status.clone();
    facial_review_command_response(
        "atelier.facial.review.status",
        &actor,
        FACIAL_REVIEW_STATUS_SCHEMA_ID,
        "atelier-facial-review-status.json",
        &status_artifact_payload,
        status,
        &sources,
    )
}

/// POST /atelier/facial/review/montage — build an Argus-addressable review tile-map artifact.
async fn build_facial_review_montage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FacialReviewMontageApiRequest>,
) -> Result<Json<FacialCommandOutcomeResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let session: FacialReviewSessionArtifact = read_facial_json_artifact_as(
        &payload.session_artifact_ref,
        FACIAL_REVIEW_SESSION_SCHEMA_ID,
    )?;
    let expected_thread_id = facial_review_session_model_operation_thread_id(&session.session_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let provided_decisions: Vec<FacialReviewDecisionReceipt> = read_facial_json_artifact_refs_as(
        &payload.decision_artifact_refs,
        FACIAL_REVIEW_DECISION_SCHEMA_ID,
    )?;
    let discovered_decisions = discover_facial_review_decisions(&session.session_id)?;
    let decisions = merge_facial_review_decisions(discovered_decisions, provided_decisions);
    let sources = artifact_handles_from_refs(
        &[
            std::slice::from_ref(&payload.session_artifact_ref),
            payload.decision_artifact_refs.as_slice(),
        ]
        .concat(),
    );
    let montage = match build_review_montage(
        &session,
        &decisions,
        BuildFacialReviewMontageRequest {
            requested_by: actor.clone(),
            page: payload.page.unwrap_or(0),
            columns: payload.columns.unwrap_or(5),
            rows: payload.rows.unwrap_or(4),
            tile_width: payload.tile_width.unwrap_or(256),
            tile_height: payload.tile_height.unwrap_or(256),
            decision_filter: payload.decision_filter,
        },
    ) {
        Ok(montage) => montage,
        Err(domain_detail) => {
            let error_code = facial_montage_error_code(&domain_detail);
            return facial_review_command_outcome(
                "atelier.facial.review.montage",
                &actor,
                "error",
                None,
                None,
                None,
                Some(error_code.to_owned()),
                Some(
                    "Montage could not be built. Verify grid size, tile dimensions, page, and \
                     decision filter, then rerun montage build."
                        .to_owned(),
                ),
                Vec::new(),
                &sources,
            );
        }
    };
    let montage_artifact_payload = montage.clone();
    facial_review_command_response(
        "atelier.facial.review.montage",
        &actor,
        FACIAL_REVIEW_MONTAGE_SCHEMA_ID,
        "atelier-facial-review-montage.json",
        &montage_artifact_payload,
        montage,
        &sources,
    )
}

/// POST /atelier/facial/review/export — build a non-destructive LoRA dataset export manifest,
/// returning a durable Facial command *outcome* envelope (MT-031). Every post-context outcome
/// (`blocked`/`degraded`/`error`/`succeeded`) is HTTP 200 with a persisted receipt; pre-context
/// failures (missing actor header, unresolvable session/decision refs) stay bare HTTP 4xx.
async fn build_facial_review_export(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FacialReviewExportApiRequest>,
) -> Result<Json<FacialCommandOutcomeResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let session: FacialReviewSessionArtifact = read_facial_json_artifact_as(
        &payload.session_artifact_ref,
        FACIAL_REVIEW_SESSION_SCHEMA_ID,
    )?;
    let expected_thread_id = facial_review_session_model_operation_thread_id(&session.session_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let provided_decisions: Vec<FacialReviewDecisionReceipt> = read_facial_json_artifact_refs_as(
        &payload.decision_artifact_refs,
        FACIAL_REVIEW_DECISION_SCHEMA_ID,
    )?;
    let discovered_decisions = discover_facial_review_decisions(&session.session_id)?;
    let decisions = merge_facial_review_decisions(discovered_decisions, provided_decisions);
    let sources = artifact_handles_from_refs(
        &[
            std::slice::from_ref(&payload.session_artifact_ref),
            payload.decision_artifact_refs.as_slice(),
        ]
        .concat(),
    );
    let allow_partial = payload.allow_partial.unwrap_or(false);

    // Deterministic BLOCKED pre-check: mirror the domain's undecided guard so a refusal becomes a
    // durable, parser-visible command envelope instead of the domain's opaque Err string.
    let decided_stable_ids: HashSet<&str> = decisions
        .iter()
        .map(|decision| decision.stable_image_id.as_str())
        .collect();
    let undecided = session
        .items
        .iter()
        .any(|item| !decided_stable_ids.contains(item.stable_image_id.as_str()));
    if !allow_partial && undecided {
        return facial_review_command_outcome(
            "atelier.facial.review.export",
            &actor,
            "blocked",
            None,
            None,
            None,
            Some("undecided_items_block_export".to_owned()),
            Some(
                "Undecided items remain: record a decision for every item via \
                 POST /atelier/facial/review/decisions, or re-run export with \
                 allow_partial=true to intentionally export a partial dataset."
                    .to_owned(),
            ),
            Vec::new(),
            &sources,
        );
    }

    let manifest: FacialReviewExportManifest = match build_review_export_manifest(
        &session,
        &decisions,
        BuildFacialReviewExportRequest {
            requested_by: actor.clone(),
            exported_at_utc: payload
                .exported_at_utc
                .unwrap_or_else(|| Utc::now().to_rfc3339()),
            dataset_name: payload.dataset_name,
            repeats: payload.repeats.unwrap_or(10),
            allow_partial,
            output_root_ref: payload.output_root_ref,
        },
    ) {
        Ok(manifest) => manifest,
        Err(_domain_detail) => {
            return facial_review_command_outcome(
                "atelier.facial.review.export",
                &actor,
                "error",
                None,
                None,
                None,
                Some("export_command_failed".to_owned()),
                Some(
                    "Export could not be built. Verify dataset_name (no whitespace or \
                     slashes), repeats, output_root_ref, and the session/decision refs, \
                     then re-run export. Read the receipt for the persisted failure."
                        .to_owned(),
                ),
                Vec::new(),
                &sources,
            );
        }
    };

    let undecided_count = manifest
        .funnel
        .get("undecided")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let is_degraded = undecided_count > 0 || !manifest.problems.is_empty();
    let manifest_value = serde_json::to_value(&manifest).map_err(internal_error)?;
    if is_degraded {
        let mut degraded_reasons = Vec::new();
        if undecided_count > 0 {
            degraded_reasons.push(format!("undecided_items_skipped:{undecided_count}"));
        }
        let mut problem_counts: Vec<(String, usize)> = Vec::new();
        for problem in &manifest.problems {
            if problem.problem == "undecided_skipped" {
                continue;
            }
            match problem_counts
                .iter_mut()
                .find(|(name, _)| name == &problem.problem)
            {
                Some((_, count)) => *count += 1,
                None => problem_counts.push((problem.problem.clone(), 1)),
            }
        }
        for (problem, count) in problem_counts {
            degraded_reasons.push(format!("{problem}:{count}"));
        }
        if degraded_reasons.is_empty() {
            degraded_reasons.push("export_degraded".to_owned());
        }
        facial_review_command_outcome(
            "atelier.facial.review.export",
            &actor,
            "degraded",
            Some(FACIAL_REVIEW_EXPORT_SCHEMA_ID),
            Some("atelier-facial-review-export.json"),
            Some(manifest_value),
            None,
            Some(
                "Export succeeded but some items were skipped (undecided/rejected/hold/\
                 duplicate). Review the funnel and problems, adjust decisions, and re-run \
                 export if a complete dataset is required."
                    .to_owned(),
            ),
            degraded_reasons,
            &sources,
        )
    } else {
        facial_review_command_outcome(
            "atelier.facial.review.export",
            &actor,
            "succeeded",
            Some(FACIAL_REVIEW_EXPORT_SCHEMA_ID),
            Some("atelier-facial-review-export.json"),
            Some(manifest_value),
            None,
            None,
            Vec::new(),
            &sources,
        )
    }
}

fn facial_review_command_response<T, P>(
    command: &str,
    actor: &str,
    result_schema_id: &str,
    result_file_name: &str,
    artifact_payload: &P,
    result: T,
    source_artifact_refs: &[ArtifactHandle],
) -> Result<Json<FacialCommandOutcomeResponse>, ApiError>
where
    T: Serialize,
    P: Serialize,
{
    let result_artifact = write_facial_json_artifact(
        artifact_payload,
        result_schema_id,
        result_file_name,
        actor,
        command,
        source_artifact_refs,
    )?;
    let result_value = serde_json::to_value(&result).map_err(internal_error)?;
    let mut receipt_sources = source_artifact_refs.to_vec();
    receipt_sources.push(facial_command_artifact_handle(&result_artifact));
    let receipt_payload = serde_json::json!({
        "schema_id": FACIAL_API_COMMAND_RECEIPT_SCHEMA_ID,
        "command": command,
        "status": "succeeded",
        "actor": actor,
        "actor_ref": format!("actor://sha256/{}", text_hash(actor)),
        "created_at_utc": Utc::now().to_rfc3339(),
        "result_schema_id": result_schema_id,
        "result_artifact_ref": result_artifact.artifact_ref.clone(),
        "result_manifest_ref": result_artifact.manifest_ref.clone(),
        "result_content_hash": result_artifact.content_hash.clone(),
        "result": result_value.clone(),
    });
    let receipt_artifact = write_facial_json_artifact(
        &receipt_payload,
        FACIAL_API_COMMAND_RECEIPT_SCHEMA_ID,
        "atelier-facial-command-receipt.json",
        actor,
        command,
        &receipt_sources,
    )?;
    Ok(Json(FacialCommandOutcomeResponse {
        schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID.to_owned(),
        command: command.to_owned(),
        status: "succeeded".to_owned(),
        actor: actor.to_owned(),
        result: result_value,
        result_artifact: Some(result_artifact),
        receipt_ref: receipt_artifact.artifact_ref.clone(),
        receipt_artifact,
        error: None,
        degraded_reasons: Vec::new(),
        recovery_hint: None,
    }))
}

fn facial_claim_error_code(detail: &str) -> &'static str {
    if detail.contains("actively claimed") {
        "claim_shard_already_active"
    } else if detail.contains("no claimable facial review shards remain") {
        "claim_no_claimable_shards"
    } else if detail.contains("out of range") {
        "claim_shard_out_of_range"
    } else if detail.contains("expired claim") {
        "claim_expired_requires_steal"
    } else if detail.contains("no undecided work") {
        "claim_shard_has_no_work"
    } else {
        "claim_command_failed"
    }
}

fn facial_decision_error_code(detail: &str) -> &'static str {
    if detail.contains("different session") {
        "decision_claim_session_mismatch"
    } else if detail.contains("actor does not own") {
        "decision_actor_not_claim_owner"
    } else if detail.contains("claim_id does not match") {
        "decision_claim_id_mismatch"
    } else if detail.contains("expired claim") {
        "decision_claim_expired"
    } else if detail.contains("not in the claimed shard") {
        "decision_item_not_in_claim"
    } else if detail.contains("unknown facial review item") {
        "decision_unknown_item"
    } else if detail.contains("unsupported facial review decision") {
        "decision_unsupported_value"
    } else {
        "decision_command_failed"
    }
}

fn facial_montage_error_code(detail: &str) -> &'static str {
    if detail.contains("grid must be") {
        "montage_invalid_grid"
    } else if detail.contains("tile dimensions") {
        "montage_invalid_tile_dimensions"
    } else if detail.contains("no matching items") {
        "montage_no_matching_items"
    } else if detail.contains("page") && detail.contains("out of range") {
        "montage_page_out_of_range"
    } else if detail.contains("unsupported facial review decision") {
        "montage_unsupported_filter"
    } else {
        "montage_command_failed"
    }
}

/// MT-031: build a durable Facial command *outcome* envelope for any post-context status at
/// HTTP 200. A result artifact is written only when a result payload is present
/// (succeeded/degraded); a durable receipt is ALWAYS written through the ArtifactStore so a
/// model can read the failure back through `GET /atelier/facial/artifacts/read`.
#[allow(clippy::too_many_arguments)]
fn facial_review_command_outcome(
    command: &str,
    actor: &str,
    status: &str,
    result_schema_id: Option<&str>,
    result_file_name: Option<&str>,
    result_payload: Option<serde_json::Value>,
    error: Option<String>,
    recovery_hint: Option<String>,
    degraded_reasons: Vec<String>,
    source_artifact_refs: &[ArtifactHandle],
) -> Result<Json<FacialCommandOutcomeResponse>, ApiError> {
    let result_artifact = match (&result_payload, result_schema_id, result_file_name) {
        (Some(payload), Some(schema_id), Some(file_name)) => Some(write_facial_json_artifact(
            payload,
            schema_id,
            file_name,
            actor,
            command,
            source_artifact_refs,
        )?),
        _ => None,
    };
    let mut receipt_map = serde_json::Map::new();
    receipt_map.insert(
        "schema_id".to_owned(),
        serde_json::json!(FACIAL_API_COMMAND_RECEIPT_SCHEMA_ID),
    );
    receipt_map.insert("command".to_owned(), serde_json::json!(command));
    receipt_map.insert("status".to_owned(), serde_json::json!(status));
    receipt_map.insert("actor".to_owned(), serde_json::json!(actor));
    receipt_map.insert(
        "actor_ref".to_owned(),
        serde_json::json!(format!("actor://sha256/{}", text_hash(actor))),
    );
    receipt_map.insert(
        "created_at_utc".to_owned(),
        serde_json::json!(Utc::now().to_rfc3339()),
    );
    if let Some(schema_id) = result_schema_id {
        receipt_map.insert("result_schema_id".to_owned(), serde_json::json!(schema_id));
    }
    if let Some(artifact) = &result_artifact {
        receipt_map.insert(
            "result_artifact_ref".to_owned(),
            serde_json::json!(artifact.artifact_ref),
        );
        receipt_map.insert(
            "result_manifest_ref".to_owned(),
            serde_json::json!(artifact.manifest_ref),
        );
        receipt_map.insert(
            "result_content_hash".to_owned(),
            serde_json::json!(artifact.content_hash),
        );
    }
    if let Some(error) = &error {
        receipt_map.insert("error".to_owned(), serde_json::json!(error));
    }
    if let Some(recovery_hint) = &recovery_hint {
        receipt_map.insert("recovery_hint".to_owned(), serde_json::json!(recovery_hint));
    }
    if !degraded_reasons.is_empty() {
        receipt_map.insert(
            "degraded_reasons".to_owned(),
            serde_json::json!(degraded_reasons),
        );
    }
    receipt_map.insert(
        "result".to_owned(),
        result_payload.clone().unwrap_or(serde_json::Value::Null),
    );
    let receipt_payload = serde_json::Value::Object(receipt_map);
    let mut receipt_sources = source_artifact_refs.to_vec();
    if let Some(artifact) = &result_artifact {
        receipt_sources.push(facial_command_artifact_handle(artifact));
    }
    let receipt_artifact = write_facial_json_artifact(
        &receipt_payload,
        FACIAL_API_COMMAND_RECEIPT_SCHEMA_ID,
        "atelier-facial-command-receipt.json",
        actor,
        command,
        &receipt_sources,
    )?;
    Ok(Json(FacialCommandOutcomeResponse {
        schema_id: FACIAL_API_COMMAND_RESPONSE_SCHEMA_ID.to_owned(),
        command: command.to_owned(),
        status: status.to_owned(),
        actor: actor.to_owned(),
        result: result_payload.unwrap_or(serde_json::Value::Null),
        result_artifact,
        receipt_ref: receipt_artifact.artifact_ref.clone(),
        receipt_artifact,
        error,
        degraded_reasons,
        recovery_hint,
    }))
}

fn write_facial_json_artifact<T: Serialize>(
    payload: &T,
    schema_id: &str,
    file_name: &str,
    actor: &str,
    command: &str,
    source_artifact_refs: &[ArtifactHandle],
) -> Result<FacialCommandArtifactResponse, ApiError> {
    let workspace_root = resolve_workspace_root().map_err(internal_error)?;
    let payload_value = serde_json::to_value(payload).map_err(internal_error)?;
    let payload_bytes = serde_json::to_vec(payload).map_err(internal_error)?;
    let content_hash = sha256_hex(&payload_bytes);
    let artifact_id = Uuid::now_v7();
    let mut source_entity_refs = facial_command_source_entity_refs(actor, command, schema_id);
    source_entity_refs.extend(facial_payload_source_entity_refs(&payload_value));
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: "application/json".to_owned(),
        filename_hint: Some(file_name.to_owned()),
        created_at: Utc::now(),
        created_by_job_id: None,
        source_entity_refs,
        source_artifact_refs: source_artifact_refs.to_vec(),
        content_hash: content_hash.clone(),
        size_bytes: payload_bytes.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(true),
        hash_basis: Some(format!(
            "{}|command={}|actor={}|payload={}",
            schema_id,
            command,
            text_hash(actor),
            content_hash
        )),
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(&workspace_root, &manifest, &payload_bytes).map_err(internal_error)?;
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, artifact_id)
        .map_err(internal_error)?;
    let root = artifact_root_rel(ArtifactLayer::L1, artifact_id);
    Ok(FacialCommandArtifactResponse {
        artifact_ref: format!("artifact://{root}/payload"),
        manifest_ref: format!("artifact://{root}/artifact.json"),
        content_hash,
        byte_len: payload_bytes.len() as u64,
        mime: "application/json".to_owned(),
        file_name: file_name.to_owned(),
    })
}

fn facial_review_claim_lock() -> &'static tokio::sync::Mutex<()> {
    FACIAL_REVIEW_CLAIM_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn discover_facial_review_claims(
    session_id: &str,
) -> Result<Vec<FacialReviewClaimReceipt>, ApiError> {
    discover_facial_review_artifacts(session_id, FACIAL_REVIEW_CLAIM_SCHEMA_ID)
}

fn discover_facial_review_decisions(
    session_id: &str,
) -> Result<Vec<FacialReviewDecisionReceipt>, ApiError> {
    discover_facial_review_artifacts(session_id, FACIAL_REVIEW_DECISION_SCHEMA_ID)
}

/// Server-authoritative recovery: enumerate the L1 layer for Facial receipts of one schema that
/// belong to `session_id`, so a caller that supplies no refs still sees every persisted claim or
/// decision. Non-Facial and unreadable artifacts are skipped; hard read failures propagate.
fn discover_facial_review_artifacts<T: serde::de::DeserializeOwned>(
    session_id: &str,
    expected_schema_id: &str,
) -> Result<Vec<T>, ApiError> {
    let workspace_root = resolve_workspace_root().map_err(internal_error)?;
    let layer = ArtifactLayer::L1;
    let layer_root = artifact_store_root(&workspace_root).join(layer.as_str());
    if !layer_root.exists() {
        return Ok(Vec::new());
    }

    let mut artifact_ids = Vec::new();
    for entry in fs::read_dir(&layer_root).map_err(internal_error)? {
        let entry = entry.map_err(internal_error)?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(artifact_id) = Uuid::parse_str(name) else {
            continue;
        };
        artifact_ids.push(artifact_id);
    }
    artifact_ids.sort();

    let mut artifacts = Vec::new();
    for artifact_id in artifact_ids {
        let artifact_ref = format!(
            "artifact://{}/payload",
            artifact_root_rel(layer, artifact_id)
        );
        let response = match read_facial_json_artifact_value(&artifact_ref) {
            Ok(response) => response,
            Err((StatusCode::BAD_REQUEST, _)) | Err((StatusCode::NOT_FOUND, _)) => continue,
            Err(err) => return Err(err),
        };
        if response.payload_schema_id.as_deref() != Some(expected_schema_id) {
            continue;
        }
        if response
            .payload
            .get("session_id")
            .and_then(|value| value.as_str())
            != Some(session_id)
        {
            continue;
        }
        artifacts.push(serde_json::from_value(response.payload).map_err(internal_error)?);
    }
    Ok(artifacts)
}

fn merge_facial_review_claims(
    discovered: Vec<FacialReviewClaimReceipt>,
    provided: Vec<FacialReviewClaimReceipt>,
) -> Vec<FacialReviewClaimReceipt> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for claim in discovered.into_iter().chain(provided) {
        if seen.insert(claim.claim_id.clone()) {
            merged.push(claim);
        }
    }
    merged
}

fn merge_facial_review_decisions(
    discovered: Vec<FacialReviewDecisionReceipt>,
    provided: Vec<FacialReviewDecisionReceipt>,
) -> Vec<FacialReviewDecisionReceipt> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();
    for decision in discovered.into_iter().chain(provided) {
        if seen.insert(decision.decision_id.clone()) {
            merged.push(decision);
        }
    }
    merged
}

fn facial_command_artifact_handle(artifact: &FacialCommandArtifactResponse) -> ArtifactHandle {
    artifact_handle_from_ref(&artifact.artifact_ref)
        .unwrap_or_else(|| ArtifactHandle::new(Uuid::now_v7(), artifact.artifact_ref.clone()))
}

fn facial_command_source_entity_refs(
    actor: &str,
    command: &str,
    schema_id: &str,
) -> Vec<EntityRef> {
    vec![
        EntityRef {
            entity_kind: "facial_command".to_owned(),
            entity_id: command.to_owned(),
        },
        EntityRef {
            entity_kind: "facial_schema".to_owned(),
            entity_id: schema_id.to_owned(),
        },
        EntityRef {
            entity_kind: "actor_sha256".to_owned(),
            entity_id: text_hash(actor),
        },
    ]
}

fn facial_payload_source_entity_refs(payload: &serde_json::Value) -> Vec<EntityRef> {
    let mut refs = Vec::new();
    if let Some(schema_id) = payload.get("schema_id").and_then(|value| value.as_str()) {
        refs.push(EntityRef {
            entity_kind: "facial_schema".to_owned(),
            entity_id: schema_id.to_owned(),
        });
    }
    let session_id = payload
        .get("session_id")
        .and_then(|value| value.as_str())
        .or_else(|| {
            payload
                .pointer("/session/session_id")
                .and_then(|value| value.as_str())
        })
        .or_else(|| {
            payload
                .pointer("/result/session_id")
                .and_then(|value| value.as_str())
        })
        .or_else(|| {
            payload
                .pointer("/result/session/session_id")
                .and_then(|value| value.as_str())
        });
    if let Some(session_id) = session_id {
        refs.push(EntityRef {
            entity_kind: "facial_review_session".to_owned(),
            entity_id: session_id.to_owned(),
        });
    }
    let batch_id = payload
        .get("batch_id")
        .and_then(|value| value.as_str())
        .or_else(|| {
            payload
                .pointer("/session/batch_id")
                .and_then(|value| value.as_str())
        })
        .or_else(|| {
            payload
                .pointer("/result/batch_id")
                .and_then(|value| value.as_str())
        })
        .or_else(|| {
            payload
                .pointer("/result/session/batch_id")
                .and_then(|value| value.as_str())
        });
    if let Some(batch_id) = batch_id {
        refs.push(EntityRef {
            entity_kind: "intake_batch".to_owned(),
            entity_id: batch_id.to_owned(),
        });
    }
    refs
}
