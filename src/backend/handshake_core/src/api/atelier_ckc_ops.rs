//! WP-CKC-posekit-overhaul SurrealDB port — CKC `ops` lane router (MT-062).
//!
//! Routes owned here:
//! - `POST /atelier/posekit/openpose-export` — native Posekit OpenPose export (JSON + PNG + receipt)
//!   into the ArtifactStore, recorded as `atelier_pose_sidecar` rows when a rig is named.
//! - `GET  /atelier/posekit/openpose-png/bytes?artifact_ref=` — fail-closed byte route for a
//!   Posekit-generated OpenPose PNG payload (manifest shape AND a rendered sidecar binding required).
//! - `GET  /atelier/model-ops/state?thread_id=`, `/atelier/model-ops/leases` GET/POST,
//!   `/atelier/model-ops/leases/:claim_id` GET, `/renew` POST, `/release` POST,
//!   `/atelier/model-ops/action-receipts` POST, `/atelier/model-ops/action-receipts/:receipt_id` GET.
//! - `GET/PUT /atelier/preferences`, `POST /atelier/preferences/reset`.
//!
//! Storage authority is the embedded SurrealDB store through `AtelierStore`; blobs live in the
//! ArtifactStore (`storage::artifacts`). Shared helpers come from `super::atelier`. The model-ops
//! error envelope (`detail` / `recovery_hint` / `required_headers` / `state_route`) is a superset of
//! the shared `ErrorResponse` so every error body this router emits still carries the shared
//! `error` code.

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::atelier::{
    artifact_byte_read_error, atelier_error, atelier_store, calling_actor, header_str,
    internal_error, ErrorResponse as SharedErrorResponse, HSK_HEADER_ACTOR_ID,
};
use crate::ace::ArtifactHandle;
use crate::atelier::action_receipt::{ActionReceipt, ActionReceiptStatus, NewActionReceipt};
use crate::atelier::model_lease::{
    claim_mode_token, executor_kind_token, ModelLeaseRecord, NewModelLeaseClaim,
};
use crate::atelier::pose::{
    generate_posekit_openpose_export, generate_posekit_openpose_export_from_keypoints,
    parse_pose_sidecar_artifact_handle, NewPoseSidecar, PoseSidecar, PoseSidecarKind,
    PoseSidecarStatus, PosekitExportFraming, PosekitMarkerLayers, PosekitOpenPoseExport,
    PosekitOpenPoseExportRequest, POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID,
};
use crate::atelier::settings::{
    EffectivePreference, PreferenceScope, PreferenceType, SetPreference,
};
use crate::atelier::AtelierError;
use crate::kernel::role_mailbox_claim_lease::{
    ClaimLeaseState, RoleMailboxClaimMode, RoleMailboxExecutorKind,
};
use crate::storage::artifacts::{
    artifact_root_rel, read_artifact_manifest, read_file_artifact_with_manifest,
    resolve_workspace_root, sha256_hex, sha256_refs_match, validate_artifact_content_hash,
    write_file_artifact, ArtifactClassification, ArtifactLayer, ArtifactManifest,
    ArtifactPayloadKind,
};
use crate::storage::EntityRef;
use crate::AppState;

/// Request header naming the caller's actor kind; `operator` unlocks the operator path on
/// model-operation guarded mutations (only for `x-hsk-actor-id: operator`).
pub(crate) const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
/// Request header carrying the `atelier_model_coordination_lease` claim id a model mutates under.
pub(crate) const HSK_HEADER_MODEL_LEASE_ID: &str = "x-hsk-model-lease-id";
/// Request header carrying the session that holds the lease named by `x-hsk-model-lease-id`.
pub(crate) const HSK_HEADER_SESSION_ID: &str = "x-hsk-session-id";
/// Response header carrying the ArtifactStore payload ref the served bytes came from.
const HSK_HEADER_ARTIFACT_REF: &str = "x-hsk-artifact-ref";
/// Response header carrying the bare lowercase sha256 hex of the served payload.
const HSK_HEADER_CONTENT_SHA256: &str = "x-hsk-content-sha256";

const MODEL_OPS_STATE_ROUTE: &str = "/atelier/model-ops/state?thread_id={thread_id}";
const MODEL_OPS_RECOVERY_HINT: &str = "Read /atelier/model-ops/state?thread_id={thread_id}, then retry with matching x-hsk-actor-id, x-hsk-session-id, and x-hsk-model-lease-id for an active exclusive_lease or handoff reservation.";

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/atelier/posekit/openpose-export",
            post(export_posekit_openpose),
        )
        .route(
            "/atelier/posekit/openpose-png/bytes",
            get(get_posekit_openpose_png_bytes),
        )
        .route("/atelier/model-ops/state", get(model_ops_state))
        .route(
            "/atelier/model-ops/leases",
            get(list_model_operation_leases).post(claim_model_operation_lease),
        )
        .route(
            "/atelier/model-ops/leases/:claim_id",
            get(get_model_operation_lease),
        )
        .route(
            "/atelier/model-ops/leases/:claim_id/renew",
            post(renew_model_operation_lease),
        )
        .route(
            "/atelier/model-ops/leases/:claim_id/release",
            post(release_model_operation_lease),
        )
        .route(
            "/atelier/model-ops/action-receipts",
            post(record_model_operation_action_receipt),
        )
        .route(
            "/atelier/model-ops/action-receipts/:receipt_id",
            get(get_model_operation_action_receipt),
        )
        .route(
            "/atelier/preferences",
            get(list_atelier_preferences).put(set_atelier_preference),
        )
        .route("/atelier/preferences/reset", post(reset_atelier_preference))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Error envelope
// ---------------------------------------------------------------------------

/// Error body for this router: the shared `error` code plus the model-operation recovery
/// metadata a parallel agent needs to self-correct (all optional, omitted when absent so plain
/// errors serialize exactly like the shared `ErrorResponse`).
#[derive(Debug, Serialize)]
pub(crate) struct OpsErrorResponse {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_hint: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required_headers: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_route: Option<&'static str>,
}

type OpsError = (StatusCode, Json<OpsErrorResponse>);

fn plain_error(error: &'static str) -> OpsErrorResponse {
    OpsErrorResponse {
        error,
        detail: None,
        recovery_hint: None,
        required_headers: None,
        state_route: None,
    }
}

/// Lift a shared-helper error (`calling_actor`, `internal_error`, `artifact_byte_read_error`,
/// `atelier_error`) into this router's envelope without changing status or code.
fn lift((status, Json(shared)): (StatusCode, Json<SharedErrorResponse>)) -> OpsError {
    (status, Json(plain_error(shared.error)))
}

fn ops_internal_error(err: impl std::fmt::Display) -> OpsError {
    lift(internal_error(err))
}

fn model_ops_error_response(error: &'static str, detail: String) -> OpsErrorResponse {
    OpsErrorResponse {
        error,
        detail: Some(detail),
        recovery_hint: Some(MODEL_OPS_RECOVERY_HINT),
        required_headers: Some(vec![
            HSK_HEADER_ACTOR_ID,
            HSK_HEADER_SESSION_ID,
            HSK_HEADER_MODEL_LEASE_ID,
        ]),
        state_route: Some(MODEL_OPS_STATE_ROUTE),
    }
}

fn is_model_ops_error_detail(detail: &str) -> bool {
    detail.contains("model-operation")
        || detail.contains(HSK_HEADER_MODEL_LEASE_ID)
        || detail.contains(HSK_HEADER_SESSION_ID)
}

/// `AtelierError` -> HTTP, mirroring the shared `atelier_error` mapping (404 / 400 / 409 / 500)
/// but enriching model-operation validation and conflict failures with the recovery envelope.
fn ops_error(err: AtelierError) -> OpsError {
    match err {
        AtelierError::Validation(detail) if is_model_ops_error_detail(&detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "bad_request");
            (
                StatusCode::BAD_REQUEST,
                Json(model_ops_error_response("bad_request", detail)),
            )
        }
        AtelierError::Conflict(detail) if is_model_ops_error_detail(&detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "conflict");
            (
                StatusCode::CONFLICT,
                Json(model_ops_error_response("conflict", detail)),
            )
        }
        other => lift(atelier_error(other)),
    }
}

fn calling_model_ops_actor(headers: &HeaderMap) -> Result<String, OpsError> {
    header_str(headers, HSK_HEADER_ACTOR_ID)
        .map(ToOwned::to_owned)
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(model_ops_error_response(
                "missing_actor",
                format!("{HSK_HEADER_ACTOR_ID} is required for model-operation endpoints"),
            )),
        ))
}

// ---------------------------------------------------------------------------
// Model-operation lease guard
// ---------------------------------------------------------------------------

fn executor_kind_from_wire(token: &str) -> Result<RoleMailboxExecutorKind, OpsError> {
    match token {
        "local_small_model" => Ok(RoleMailboxExecutorKind::LocalSmallModel),
        "local_large_model" => Ok(RoleMailboxExecutorKind::LocalLargeModel),
        "cloud_model" => Ok(RoleMailboxExecutorKind::CloudModel),
        "reviewer" => Ok(RoleMailboxExecutorKind::Reviewer),
        "validator" => Ok(RoleMailboxExecutorKind::Validator),
        "operator" => Ok(RoleMailboxExecutorKind::Operator),
        "workflow_automation" => Ok(RoleMailboxExecutorKind::WorkflowAutomation),
        other => Err(ops_error(AtelierError::Validation(format!(
            "unknown model-operation executor_kind {other}"
        )))),
    }
}

fn claim_mode_from_wire(token: &str) -> Result<RoleMailboxClaimMode, OpsError> {
    match token {
        "exclusive_lease" => Ok(RoleMailboxClaimMode::ExclusiveLease),
        "shared_observer" => Ok(RoleMailboxClaimMode::SharedObserver),
        "broadcast_request" => Ok(RoleMailboxClaimMode::BroadcastRequest),
        "handoff_reservation" => Ok(RoleMailboxClaimMode::HandoffReservation),
        other => Err(ops_error(AtelierError::Validation(format!(
            "unknown model-operation claim_mode {other}"
        )))),
    }
}

fn lease_state_token(state: ClaimLeaseState) -> &'static str {
    match state {
        ClaimLeaseState::Unclaimed => "unclaimed",
        ClaimLeaseState::Active => "active",
        ClaimLeaseState::Released => "released",
        ClaimLeaseState::Expired => "expired",
        ClaimLeaseState::TakenOver => "taken_over",
    }
}

/// Guard for mutations a model may perform under a lease OR an operator may perform directly.
/// With `x-hsk-model-lease-id` present the lease is validated (holder, session, active, mutating
/// claim mode, expected thread). Without it, only the canonical operator path
/// (`x-hsk-actor-kind: operator` AND `x-hsk-actor-id: operator`) is accepted; a bare actor-only
/// call or a self-labelled "operator" model actor is a typed 400.
async fn validate_model_operation_lease_if_present(
    state: &AppState,
    headers: &HeaderMap,
    actor: &str,
    expected_thread_id: Option<&str>,
) -> Result<Option<ModelLeaseRecord>, OpsError> {
    if header_str(headers, HSK_HEADER_MODEL_LEASE_ID).is_some() {
        return validate_model_operation_lease_required(state, headers, actor, expected_thread_id)
            .await
            .map(Some);
    }
    match header_str(headers, HSK_HEADER_ACTOR_KIND) {
        Some("operator") if actor == "operator" => Ok(None),
        Some("operator") => Err(ops_error(AtelierError::Validation(format!(
            "{HSK_HEADER_ACTOR_KIND}=operator is reserved for {HSK_HEADER_ACTOR_ID}=operator"
        )))),
        Some(other) => Err(ops_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation guarded mutations unless {HSK_HEADER_ACTOR_KIND}=operator; got {other}"
        )))),
        None => Err(ops_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation guarded mutations unless {HSK_HEADER_ACTOR_KIND}=operator"
        )))),
    }
}

async fn validate_model_operation_lease_required(
    state: &AppState,
    headers: &HeaderMap,
    actor: &str,
    expected_thread_id: Option<&str>,
) -> Result<ModelLeaseRecord, OpsError> {
    let raw_claim_id = header_str(headers, HSK_HEADER_MODEL_LEASE_ID).ok_or_else(|| {
        ops_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation mutations"
        )))
    })?;
    let session_id = header_str(headers, HSK_HEADER_SESSION_ID).ok_or_else(|| {
        ops_error(AtelierError::Validation(format!(
            "{HSK_HEADER_SESSION_ID} is required with {HSK_HEADER_MODEL_LEASE_ID}"
        )))
    })?;
    let claim_id = Uuid::parse_str(raw_claim_id).map_err(|_| {
        ops_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} must be a UUID"
        )))
    })?;
    let store = atelier_store(state);
    let record = store.get_model_lease(claim_id).await.map_err(ops_error)?;
    if record.actor_id != actor || record.session_id != session_id {
        return Err(ops_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} is held by actor={} session={}",
            record.actor_id, record.session_id
        ))));
    }
    if record.effective_state != ClaimLeaseState::Active || record.lease_expired {
        return Err(ops_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} is not active: state={} expired={}",
            lease_state_token(record.effective_state),
            record.lease_expired
        ))));
    }
    if !matches!(
        record.claim_mode,
        RoleMailboxClaimMode::ExclusiveLease | RoleMailboxClaimMode::HandoffReservation
    ) {
        return Err(ops_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} has non-mutating claim_mode={}",
            claim_mode_token(record.claim_mode)
        ))));
    }
    if let Some(expected_thread_id) = expected_thread_id {
        if record.thread_id != expected_thread_id {
            return Err(ops_error(AtelierError::Conflict(format!(
                "model-operation lease {claim_id} targets thread_id={} but mutation requires thread_id={expected_thread_id}",
                record.thread_id
            ))));
        }
    }
    Ok(record)
}

fn preference_model_operation_thread_id(key: &str) -> String {
    format!("atelier.preferences.{key}")
}

/// Stable, path-safe segment for a free-form ref (bare lowercase sha256 hex).
fn stable_thread_segment(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

fn posekit_openpose_model_operation_thread_id(source_ref: &str, rig_id: Option<Uuid>) -> String {
    match rig_id {
        Some(rig_id) => format!("atelier.posekit.rig.{rig_id}.openpose"),
        None => format!(
            "atelier.posekit.source.{}.openpose",
            stable_thread_segment(source_ref)
        ),
    }
}

fn action_receipt_model_operation_thread_id(
    params: &serde_json::Value,
) -> Result<String, OpsError> {
    let thread_id = params
        .get("thread_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ops_error(AtelierError::Validation(
                "action receipt params.thread_id is required for model-operation receipts".into(),
            ))
        })?;
    Ok(thread_id.to_owned())
}

// ---------------------------------------------------------------------------
// Posekit OpenPose export + PNG byte route
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct PosekitOpenPosePngBytesQuery {
    artifact_ref: String,
}

#[derive(Debug, Serialize)]
struct PosekitArtifactResponse {
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
    byte_len: u64,
    mime: String,
    file_name: String,
}

#[derive(Debug, Serialize)]
struct PosekitSidecarResponse {
    sidecar_id: Uuid,
    rig_id: Uuid,
    kind: String,
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
}

#[derive(Debug, Serialize)]
struct PosekitOpenPoseExportResponse {
    schema_id: String,
    source_ref: String,
    rig_id: Option<Uuid>,
    yaw_deg: i32,
    pitch_deg: i32,
    zoom_percent: i32,
    framing: PosekitExportFraming,
    marker_layers: PosekitMarkerLayers,
    applied_marker_edit_count: usize,
    width: i32,
    height: i32,
    openpose_json: serde_json::Value,
    openpose_json_sha256: String,
    openpose_png_sha256: String,
    content_hash: String,
    receipt_ref: String,
    openpose_png_artifact: PosekitArtifactResponse,
    openpose_json_artifact: PosekitArtifactResponse,
    sidecars: Vec<PosekitSidecarResponse>,
}

/// POST /atelier/posekit/openpose-export — native Rust Posekit OpenPose export into the
/// ArtifactStore. With `rig_id` the rig's stored `keypoints_json` is projected through the requested
/// view; without it a procedural preview skeleton is generated. Both the PNG and the JSON are
/// written as verified L1 file artifacts plus a receipt artifact; with a rig, both are also
/// registered as `atelier_pose_sidecar` rows (openpose_json + openpose_png), which is what later
/// authorises the PNG byte route to serve them.
async fn export_posekit_openpose(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PosekitOpenPoseExportRequest>,
) -> Result<Json<PosekitOpenPoseExportResponse>, OpsError> {
    let actor = calling_actor(&headers).map_err(lift)?;
    let expected_thread_id =
        posekit_openpose_model_operation_thread_id(&payload.source_ref, payload.rig_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let store = atelier_store(&state);
    let rig = if let Some(rig_id) = payload.rig_id {
        let rig = store.get_pose_rig(rig_id).await.map_err(ops_error)?;
        if rig.source_ref != payload.source_ref {
            return Err(ops_error(AtelierError::Validation(format!(
                "Posekit OpenPose export source_ref must match rig source_ref: request={} rig={}",
                payload.source_ref, rig.source_ref
            ))));
        }
        Some(rig)
    } else {
        None
    };
    let export = if let Some(rig) = rig.as_ref() {
        generate_posekit_openpose_export_from_keypoints(&payload, &rig.keypoints_json)
    } else {
        generate_posekit_openpose_export(&payload)
    }
    .map_err(ops_error)?;
    let png_artifact = write_posekit_export_artifact(
        &export,
        &export.openpose_png_bytes,
        &export.openpose_png_sha256,
        "image/png",
        "posekit-openpose.png",
        &actor,
        payload.rig_id,
        &[],
    )?;
    let json_artifact = write_posekit_export_artifact(
        &export,
        &export.openpose_json_bytes,
        &export.openpose_json_sha256,
        "application/json",
        "posekit-openpose.json",
        &actor,
        payload.rig_id,
        &[],
    )?;
    let receipt_artifact = write_posekit_export_receipt_artifact(
        &export,
        &png_artifact,
        &json_artifact,
        &actor,
        payload.rig_id,
    )?;
    let sidecars = if let Some(rig_id) = payload.rig_id {
        let sidecars = store
            .record_pose_sidecars(&[
                NewPoseSidecar {
                    rig_id,
                    kind: PoseSidecarKind::OpenPoseJson,
                    artifact_ref: json_artifact.artifact_ref.clone(),
                    manifest_ref: json_artifact.manifest_ref.clone(),
                    content_hash: json_artifact.content_hash.clone(),
                    byte_len: json_artifact.byte_len as i64,
                    mime: json_artifact.mime.clone(),
                    width: export.width,
                    height: export.height,
                    status: PoseSidecarStatus::Rendered,
                    error_message: None,
                },
                NewPoseSidecar {
                    rig_id,
                    kind: PoseSidecarKind::OpenPosePng,
                    artifact_ref: png_artifact.artifact_ref.clone(),
                    manifest_ref: png_artifact.manifest_ref.clone(),
                    content_hash: png_artifact.content_hash.clone(),
                    byte_len: png_artifact.byte_len as i64,
                    mime: png_artifact.mime.clone(),
                    width: export.width,
                    height: export.height,
                    status: PoseSidecarStatus::Rendered,
                    error_message: None,
                },
            ])
            .await
            .map_err(ops_error)?;
        sidecars.into_iter().map(posekit_sidecar_response).collect()
    } else {
        Vec::new()
    };
    let response = PosekitOpenPoseExportResponse {
        schema_id: POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID.to_owned(),
        source_ref: export.source_ref.clone(),
        rig_id: payload.rig_id,
        yaw_deg: export.yaw_deg,
        pitch_deg: export.pitch_deg,
        zoom_percent: export.zoom_percent,
        framing: export.framing,
        marker_layers: export.marker_layers.clone(),
        applied_marker_edit_count: export.applied_marker_edit_count,
        width: export.width,
        height: export.height,
        openpose_json: export.openpose_json.clone(),
        openpose_json_sha256: export.openpose_json_sha256.clone(),
        openpose_png_sha256: export.openpose_png_sha256.clone(),
        content_hash: export.content_hash.clone(),
        receipt_ref: receipt_artifact.artifact_ref.clone(),
        openpose_png_artifact: png_artifact,
        openpose_json_artifact: json_artifact,
        sidecars,
    };
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/posekit/openpose-export",
        status = "ok",
        actor = %actor,
        source_ref = %response.source_ref,
        yaw_deg = response.yaw_deg,
        png_artifact_ref = %response.openpose_png_artifact.artifact_ref,
        json_artifact_ref = %response.openpose_json_artifact.artifact_ref,
        sidecar_count = response.sidecars.len(),
        receipt_ref = %response.receipt_ref,
        "export Posekit OpenPose"
    );
    Ok(Json(response))
}

/// The manifest shape the Posekit PNG byte route is willing to serve: an L1 single-file
/// `image/png` written by the Posekit OpenPose export contract (filename hint + `hash_basis`
/// stamped with the export schema id). Anything else is not a Posekit OpenPose PNG.
fn posekit_openpose_png_manifest_is_readable(manifest: &ArtifactManifest) -> bool {
    manifest.layer == ArtifactLayer::L1
        && manifest.kind == ArtifactPayloadKind::File
        && manifest.mime == "image/png"
        && manifest.filename_hint.as_deref() == Some("posekit-openpose.png")
        && manifest
            .hash_basis
            .as_deref()
            .is_some_and(|basis| basis.starts_with(POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID))
}

/// GET /atelier/posekit/openpose-png/bytes?artifact_ref=... — raw bytes of a Posekit OpenPose PNG
/// export payload. Deliberately NOT a generic ArtifactStore download: the manifest must be a
/// Posekit-generated L1 `image/png` AND a rendered `atelier_pose_sidecar` row (kind openpose_png)
/// must be bound to the ref and agree with the manifest on hash / size / mime. A Posekit-shaped
/// manifest without a sidecar binding is 404; sidecar-vs-manifest drift is a hard 500; a missing
/// manifest is 404 and a payload-vs-manifest hash drift is 500 (`artifact_byte_read_error`).
async fn get_posekit_openpose_png_bytes(
    State(state): State<AppState>,
    Query(query): Query<PosekitOpenPosePngBytesQuery>,
) -> Result<Response, OpsError> {
    let artifact_ref = query.artifact_ref.trim();
    let (layer, artifact_id) =
        parse_pose_sidecar_artifact_handle(artifact_ref).map_err(ops_error)?;
    let workspace_root = resolve_workspace_root().map_err(ops_internal_error)?;
    let manifest = read_artifact_manifest(&workspace_root, layer, artifact_id)
        .map_err(|err| lift(artifact_byte_read_error(err)))?;
    if !posekit_openpose_png_manifest_is_readable(&manifest) {
        return Err(ops_error(AtelierError::Validation(
            "posekit OpenPose PNG byte route only serves Posekit-generated image/png L1 payloads"
                .to_owned(),
        )));
    }

    let store = atelier_store(&state);
    let sidecar = store
        .get_pose_sidecar_by_artifact_ref(artifact_ref)
        .await
        .map_err(ops_error)?
        .ok_or_else(|| {
            ops_error(AtelierError::NotFound(
                "posekit OpenPose PNG sidecar binding".to_owned(),
            ))
        })?;
    if sidecar.kind != PoseSidecarKind::OpenPosePng
        || sidecar.status != PoseSidecarStatus::Rendered
        || sidecar.mime != "image/png"
        || !sha256_refs_match(&sidecar.content_hash, &manifest.content_hash)
        || sidecar.byte_len < 0
        || sidecar.byte_len as u64 != manifest.size_bytes
    {
        tracing::error!(
            target: "handshake_core::atelier",
            %artifact_ref,
            sidecar_id = %sidecar.sidecar_id,
            "posekit OpenPose PNG sidecar metadata mismatch against ArtifactStore manifest"
        );
        return Err(ops_internal_error(
            "posekit OpenPose PNG sidecar metadata mismatch",
        ));
    }

    let bytes = read_file_artifact_with_manifest(&workspace_root, &manifest)
        .map_err(|err| lift(artifact_byte_read_error(err)))?;
    let content_hash = manifest.content_hash.to_ascii_lowercase();
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    if let Ok(etag) = HeaderValue::from_str(&format!("\"sha256-{content_hash}\"")) {
        headers.insert(header::ETAG, etag);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, immutable"),
    );
    if let Ok(value) = HeaderValue::from_str(artifact_ref) {
        headers.insert(HSK_HEADER_ARTIFACT_REF, value);
    }
    if let Ok(value) = HeaderValue::from_str(&content_hash) {
        headers.insert(HSK_HEADER_CONTENT_SHA256, value);
    }
    Ok(response)
}

fn posekit_sidecar_response(sidecar: PoseSidecar) -> PosekitSidecarResponse {
    PosekitSidecarResponse {
        sidecar_id: sidecar.sidecar_id,
        rig_id: sidecar.rig_id,
        kind: sidecar.kind.as_token().to_owned(),
        artifact_ref: sidecar.artifact_ref,
        manifest_ref: sidecar.manifest_ref,
        content_hash: sidecar.content_hash,
    }
}

#[allow(clippy::too_many_arguments)]
fn write_posekit_export_artifact(
    export: &PosekitOpenPoseExport,
    payload_bytes: &[u8],
    content_hash: &str,
    mime: &str,
    file_name: &str,
    actor: &str,
    rig_id: Option<Uuid>,
    source_artifact_refs: &[ArtifactHandle],
) -> Result<PosekitArtifactResponse, OpsError> {
    let workspace_root = resolve_workspace_root().map_err(ops_internal_error)?;
    let artifact_id = Uuid::now_v7();
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: mime.to_owned(),
        filename_hint: Some(file_name.to_owned()),
        created_at: Utc::now(),
        created_by_job_id: None,
        source_entity_refs: posekit_export_source_entity_refs(export, actor, rig_id),
        source_artifact_refs: source_artifact_refs.to_vec(),
        content_hash: content_hash.to_owned(),
        size_bytes: payload_bytes.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(true),
        hash_basis: Some(format!(
            "{}|{}|yaw={}|pitch={}|zoom={}|{}",
            POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID,
            export.source_ref,
            export.yaw_deg,
            export.pitch_deg,
            export.zoom_percent,
            export.content_hash
        )),
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(&workspace_root, &manifest, payload_bytes).map_err(ops_internal_error)?;
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, artifact_id)
        .map_err(ops_internal_error)?;
    let root = artifact_root_rel(ArtifactLayer::L1, artifact_id);
    Ok(PosekitArtifactResponse {
        artifact_ref: format!("artifact://{root}/payload"),
        manifest_ref: format!("artifact://{root}/artifact.json"),
        content_hash: content_hash.to_owned(),
        byte_len: payload_bytes.len() as u64,
        mime: mime.to_owned(),
        file_name: file_name.to_owned(),
    })
}

fn write_posekit_export_receipt_artifact(
    export: &PosekitOpenPoseExport,
    png_artifact: &PosekitArtifactResponse,
    json_artifact: &PosekitArtifactResponse,
    actor: &str,
    rig_id: Option<Uuid>,
) -> Result<PosekitArtifactResponse, OpsError> {
    let receipt = serde_json::json!({
        "schema_id": "hsk.atelier.posekit.openpose_export_receipt@1",
        "export_schema_id": export.schema_id.clone(),
        "source_ref": export.source_ref.clone(),
        "rig_id": rig_id,
        "actor_ref": format!("actor://sha256/{}", sha256_hex(actor.as_bytes())),
        "yaw_deg": export.yaw_deg,
        "pitch_deg": export.pitch_deg,
        "zoom_percent": export.zoom_percent,
        "framing": export.framing,
        "marker_layers": export.marker_layers.clone(),
        "marker_edits": export.openpose_json
            .get("pose_state")
            .and_then(|pose_state| pose_state.get("marker_edits"))
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "applied_marker_edit_count": export.applied_marker_edit_count,
        "width": export.width,
        "height": export.height,
        "content_hash": export.content_hash.clone(),
        "openpose_png_artifact_ref": png_artifact.artifact_ref.clone(),
        "openpose_png_manifest_ref": png_artifact.manifest_ref.clone(),
        "openpose_png_sha256": export.openpose_png_sha256.clone(),
        "openpose_json_artifact_ref": json_artifact.artifact_ref.clone(),
        "openpose_json_manifest_ref": json_artifact.manifest_ref.clone(),
        "openpose_json_sha256": export.openpose_json_sha256.clone(),
    });
    let payload_bytes = serde_json::to_vec(&receipt).map_err(ops_internal_error)?;
    let content_hash = sha256_hex(&payload_bytes);
    write_posekit_export_artifact(
        export,
        &payload_bytes,
        &content_hash,
        "application/json",
        "posekit-openpose-export-receipt.json",
        actor,
        rig_id,
        &[
            posekit_artifact_handle(png_artifact)?,
            posekit_artifact_handle(json_artifact)?,
        ],
    )
}

fn posekit_export_source_entity_refs(
    export: &PosekitOpenPoseExport,
    actor: &str,
    rig_id: Option<Uuid>,
) -> Vec<EntityRef> {
    let mut refs = vec![
        EntityRef {
            entity_kind: "posekit_source_ref".to_owned(),
            entity_id: export.source_ref.clone(),
        },
        EntityRef {
            entity_kind: "actor_sha256".to_owned(),
            entity_id: sha256_hex(actor.as_bytes()),
        },
        EntityRef {
            entity_kind: "posekit_openpose_export".to_owned(),
            entity_id: export.content_hash.clone(),
        },
    ];
    if let Some(rig_id) = rig_id {
        refs.push(EntityRef {
            entity_kind: "pose_rig".to_owned(),
            entity_id: rig_id.to_string(),
        });
    }
    refs
}

/// The receipt manifest links its two payload artifacts by handle. The refs were minted by
/// `write_posekit_export_artifact` in this same request, so a ref that does not parse back to an
/// artifact id is an internal invariant violation, never something to paper over with a fresh id.
fn posekit_artifact_handle(artifact: &PosekitArtifactResponse) -> Result<ArtifactHandle, OpsError> {
    let (_, artifact_id) =
        parse_pose_sidecar_artifact_handle(&artifact.artifact_ref).map_err(|err| {
            ops_internal_error(format!(
                "posekit export artifact ref written by this request does not parse: {err}"
            ))
        })?;
    Ok(ArtifactHandle::new(
        artifact_id,
        artifact.artifact_ref.clone(),
    ))
}

// ---------------------------------------------------------------------------
// Atelier preferences (WP-CKC MT-042 operator defaults)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SetAtelierPreferenceRequest {
    key: String,
    value: String,
    /// Declared value type ("string" | "bool" | "integer" | "float" | "json" | "path").
    /// Validated against the registered definition by the store on the way in.
    value_type: PreferenceType,
}

#[derive(Debug, Deserialize)]
struct ResetAtelierPreferenceRequest {
    key: String,
}

/// GET /atelier/preferences — the effective, operator-safe projection of the Global-scope
/// preferences, including registry defaults for unset keys.
async fn list_atelier_preferences(
    State(state): State<AppState>,
) -> Result<Json<Vec<EffectivePreference>>, OpsError> {
    let store = atelier_store(&state);
    let preferences = store
        .list_preference_projection(PreferenceScope::Global, true)
        .await
        .map_err(ops_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/preferences",
        status = "ok",
        count = preferences.len(),
        "list atelier preferences"
    );
    Ok(Json(preferences))
}

/// PUT /atelier/preferences — set one Global-scope preference to an operator value and return the
/// resulting effective preference. Model-operation guarded: a lease bound to
/// `atelier.preferences.<key>` or the canonical operator path. Fail-closed: an unknown
/// namespace/key or an out-of-vocabulary enumerated value is a typed 400.
async fn set_atelier_preference(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SetAtelierPreferenceRequest>,
) -> Result<Json<EffectivePreference>, OpsError> {
    let actor = calling_actor(&headers).map_err(lift)?;
    let expected_thread_id = preference_model_operation_thread_id(&payload.key);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let store = atelier_store(&state);
    store
        .set_preference_with_receipt_as(
            &SetPreference {
                scope: PreferenceScope::Global,
                key: payload.key.clone(),
                value_type: payload.value_type,
                value: payload.value,
                redacted: false,
            },
            Some(actor.as_str()),
        )
        .await
        .map_err(ops_error)?;
    let effective = store
        .get_effective_preference(PreferenceScope::Global, &payload.key)
        .await
        .map_err(ops_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/preferences",
        status = "ok",
        actor = %actor,
        key = %effective.key,
        "set atelier preference"
    );
    Ok(Json(effective))
}

/// POST /atelier/preferences/reset — reset one Global-scope preference to its registered default
/// without deleting provenance (revision bump + reset event). Same guard as `PUT`.
async fn reset_atelier_preference(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ResetAtelierPreferenceRequest>,
) -> Result<Json<EffectivePreference>, OpsError> {
    let actor = calling_actor(&headers).map_err(lift)?;
    let expected_thread_id = preference_model_operation_thread_id(&payload.key);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let store = atelier_store(&state);
    store
        .reset_preference_to_default_as(PreferenceScope::Global, &payload.key, Some(actor.as_str()))
        .await
        .map_err(ops_error)?;
    let effective = store
        .get_effective_preference(PreferenceScope::Global, &payload.key)
        .await
        .map_err(ops_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/preferences/reset",
        status = "ok",
        actor = %actor,
        key = %effective.key,
        "reset atelier preference to default"
    );
    Ok(Json(effective))
}

// ---------------------------------------------------------------------------
// Model-operation leases + action receipts (WP-CKC MT-022)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ModelOperationLeaseListQuery {
    thread_id: String,
}

#[derive(Debug, Deserialize)]
struct ClaimModelOperationLeaseRequest {
    thread_id: String,
    executor_kind: String,
    session_id: String,
    claim_mode: String,
    ttl_seconds: i64,
    linked_work_packet_id: String,
    linked_micro_task_id: String,
}

#[derive(Debug, Deserialize)]
struct RenewModelOperationLeaseRequest {
    session_id: String,
    extend_seconds: i64,
}

#[derive(Debug, Deserialize)]
struct ReleaseModelOperationLeaseRequest {
    session_id: String,
}

#[derive(Debug, Serialize)]
struct ModelOperationLeaseResponse {
    schema_id: &'static str,
    claim_id: Uuid,
    thread_id: String,
    executor_kind: String,
    actor_id: String,
    session_id: String,
    claim_mode: String,
    stored_state: String,
    effective_state: String,
    claimed_at_utc: DateTime<Utc>,
    ttl_seconds: i64,
    lease_expires_at_utc: DateTime<Utc>,
    released_at_utc: Option<DateTime<Utc>>,
    taken_over_at_utc: Option<DateTime<Utc>>,
    takeover_reason: Option<String>,
    prior_claim_id: Option<Uuid>,
    linked_work_packet_id: String,
    linked_micro_task_id: String,
    lease_age_seconds: i64,
    lease_expired: bool,
}

#[derive(Debug, Serialize)]
struct ModelOperationStateResponse {
    schema_id: &'static str,
    thread_id: String,
    leases: Vec<ModelOperationLeaseResponse>,
    required_headers_for_mutation: Vec<&'static str>,
    recovery_hint: &'static str,
}

#[derive(Debug, Deserialize)]
struct RecordModelOperationActionReceiptRequest {
    action_id: String,
    /// Accepted for wire compatibility but never trusted: the persisted `actor_kind` is derived from
    /// the validated lease's executor kind.
    #[serde(default)]
    actor_kind: Option<String>,
    session_id: String,
    params: serde_json::Value,
    started_at_utc: DateTime<Utc>,
    completed_at_utc: DateTime<Utc>,
    status: String,
    target_refs: Vec<String>,
    evidence_refs: Vec<String>,
    result_refs: Vec<String>,
    error_class: Option<String>,
    recovery_hint: Option<String>,
}

#[derive(Debug, Serialize)]
struct ModelOperationActionReceiptResponse {
    schema_id: &'static str,
    receipt_id: Uuid,
    action_id: String,
    params_sha256: String,
    actor_kind: String,
    actor_id: String,
    session_id: String,
    thread_id: String,
    lease_claim_id: Uuid,
    started_at_utc: DateTime<Utc>,
    completed_at_utc: DateTime<Utc>,
    status: String,
    target_refs: Vec<String>,
    evidence_refs: Vec<String>,
    result_refs: Vec<String>,
    error_class: Option<String>,
    recovery_hint: Option<String>,
    created_at_utc: DateTime<Utc>,
}

fn model_operation_lease_response(record: ModelLeaseRecord) -> ModelOperationLeaseResponse {
    ModelOperationLeaseResponse {
        schema_id: "hsk.atelier.model_operation_lease@1",
        claim_id: record.claim_id,
        thread_id: record.thread_id,
        executor_kind: executor_kind_token(record.executor_kind).to_owned(),
        actor_id: record.actor_id,
        session_id: record.session_id,
        claim_mode: claim_mode_token(record.claim_mode).to_owned(),
        stored_state: lease_state_token(record.stored_state).to_owned(),
        effective_state: lease_state_token(record.effective_state).to_owned(),
        claimed_at_utc: record.claimed_at_utc,
        ttl_seconds: record.ttl_seconds,
        lease_expires_at_utc: record.lease_expires_at_utc,
        released_at_utc: record.released_at_utc,
        taken_over_at_utc: record.taken_over_at_utc,
        takeover_reason: record.takeover_reason,
        prior_claim_id: record.prior_claim_id,
        linked_work_packet_id: record.linked_work_packet_id,
        linked_micro_task_id: record.linked_micro_task_id,
        lease_age_seconds: record.lease_age_seconds,
        lease_expired: record.lease_expired,
    }
}

/// A receipt read through the model-ops surface must carry model-operation lineage; a legacy
/// receipt without `lease_claim_id`/`thread_id` is a typed 400 with the recovery envelope.
fn model_operation_action_receipt_response(
    receipt: ActionReceipt,
) -> Result<ModelOperationActionReceiptResponse, OpsError> {
    let Some(lease_claim_id) = receipt.lease_claim_id else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(model_ops_error_response(
                "bad_request",
                format!(
                    "model-operation action receipt {} is missing lease_claim_id lineage",
                    receipt.receipt_id
                ),
            )),
        ));
    };
    if receipt.thread_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(model_ops_error_response(
                "bad_request",
                format!(
                    "model-operation action receipt {} is missing thread_id lineage",
                    receipt.receipt_id
                ),
            )),
        ));
    }
    Ok(ModelOperationActionReceiptResponse {
        schema_id: "hsk.atelier.model_operation_action_receipt@1",
        receipt_id: receipt.receipt_id,
        action_id: receipt.action_id,
        params_sha256: receipt.params_sha256,
        actor_kind: receipt.actor_kind,
        actor_id: receipt.actor_id,
        session_id: receipt.session_id,
        thread_id: receipt.thread_id,
        lease_claim_id,
        started_at_utc: receipt.started_at_utc,
        completed_at_utc: receipt.completed_at_utc,
        status: receipt.status.as_token().to_owned(),
        target_refs: receipt.target_refs,
        evidence_refs: receipt.evidence_refs,
        result_refs: receipt.result_refs,
        error_class: receipt.error_class,
        recovery_hint: receipt.recovery_hint,
        created_at_utc: receipt.created_at_utc,
    })
}

/// POST /atelier/model-ops/leases — claim a coordination thread for the calling actor.
async fn claim_model_operation_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ClaimModelOperationLeaseRequest>,
) -> Result<(StatusCode, Json<ModelOperationLeaseResponse>), OpsError> {
    let actor = calling_model_ops_actor(&headers)?;
    let store = atelier_store(&state);
    let record = store
        .claim_model_lease(&NewModelLeaseClaim {
            thread_id: payload.thread_id,
            executor_kind: executor_kind_from_wire(&payload.executor_kind)?,
            actor_id: actor.clone(),
            session_id: payload.session_id,
            claim_mode: claim_mode_from_wire(&payload.claim_mode)?,
            ttl_seconds: payload.ttl_seconds,
            linked_work_packet_id: payload.linked_work_packet_id,
            linked_micro_task_id: payload.linked_micro_task_id,
        })
        .await
        .map_err(ops_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/model-ops/leases",
        status = "created",
        actor = %actor,
        claim_id = %record.claim_id,
        thread_id = %record.thread_id,
        "claim model-operation lease"
    );
    Ok((
        StatusCode::CREATED,
        Json(model_operation_lease_response(record)),
    ))
}

/// GET /atelier/model-ops/leases?thread_id= — all leases on a thread, newest first.
async fn list_model_operation_leases(
    State(state): State<AppState>,
    Query(query): Query<ModelOperationLeaseListQuery>,
) -> Result<Json<Vec<ModelOperationLeaseResponse>>, OpsError> {
    let store = atelier_store(&state);
    let rows = store
        .list_model_leases_for_thread(&query.thread_id)
        .await
        .map_err(ops_error)?;
    Ok(Json(
        rows.into_iter()
            .map(model_operation_lease_response)
            .collect(),
    ))
}

/// GET /atelier/model-ops/state?thread_id= — the recovery view a conflicting agent reads.
async fn model_ops_state(
    State(state): State<AppState>,
    Query(query): Query<ModelOperationLeaseListQuery>,
) -> Result<Json<ModelOperationStateResponse>, OpsError> {
    let store = atelier_store(&state);
    let rows = store
        .list_model_leases_for_thread(&query.thread_id)
        .await
        .map_err(ops_error)?;
    Ok(Json(ModelOperationStateResponse {
        schema_id: "hsk.atelier.model_operation_state@1",
        thread_id: query.thread_id,
        leases: rows
            .into_iter()
            .map(model_operation_lease_response)
            .collect(),
        required_headers_for_mutation: vec![
            HSK_HEADER_ACTOR_ID,
            HSK_HEADER_SESSION_ID,
            HSK_HEADER_MODEL_LEASE_ID,
        ],
        recovery_hint: "If a mutating request returns conflict, inspect this state route, release or wait for the active lease, then retry with a fresh exclusive_lease.",
    }))
}

/// GET /atelier/model-ops/leases/:claim_id
async fn get_model_operation_lease(
    State(state): State<AppState>,
    Path(claim_id): Path<Uuid>,
) -> Result<Json<ModelOperationLeaseResponse>, OpsError> {
    let store = atelier_store(&state);
    let record = store.get_model_lease(claim_id).await.map_err(ops_error)?;
    Ok(Json(model_operation_lease_response(record)))
}

fn require_session_header(headers: &HeaderMap, verb: &str) -> Result<String, OpsError> {
    header_str(headers, HSK_HEADER_SESSION_ID)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(model_ops_error_response(
                    "missing_session",
                    format!(
                        "{HSK_HEADER_SESSION_ID} is required to {verb} a model-operation lease"
                    ),
                )),
            )
        })
}

/// POST /atelier/model-ops/leases/:claim_id/renew
async fn renew_model_operation_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(claim_id): Path<Uuid>,
    Json(payload): Json<RenewModelOperationLeaseRequest>,
) -> Result<Json<ModelOperationLeaseResponse>, OpsError> {
    let actor = calling_model_ops_actor(&headers)?;
    let session_id = require_session_header(&headers, "renew")?;
    if session_id != payload.session_id {
        return Err(ops_error(AtelierError::Validation(format!(
            "session_id must match {HSK_HEADER_SESSION_ID}"
        ))));
    }
    let store = atelier_store(&state);
    let current = store.get_model_lease(claim_id).await.map_err(ops_error)?;
    if current.session_id != payload.session_id {
        return Err((
            StatusCode::CONFLICT,
            Json(model_ops_error_response(
                "conflict",
                format!(
                    "model-operation lease {claim_id} cannot be renewed from session {}; {HSK_HEADER_SESSION_ID} must match active lease session {}",
                    payload.session_id, current.session_id
                ),
            )),
        ));
    }
    let record = store
        .renew_model_lease(claim_id, &actor, payload.extend_seconds)
        .await
        .map_err(ops_error)?;
    Ok(Json(model_operation_lease_response(record)))
}

/// POST /atelier/model-ops/leases/:claim_id/release
async fn release_model_operation_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(claim_id): Path<Uuid>,
    Json(payload): Json<ReleaseModelOperationLeaseRequest>,
) -> Result<Json<ModelOperationLeaseResponse>, OpsError> {
    let actor = calling_model_ops_actor(&headers)?;
    let session_id = require_session_header(&headers, "release")?;
    if session_id != payload.session_id {
        return Err(ops_error(AtelierError::Validation(format!(
            "session_id must match {HSK_HEADER_SESSION_ID}"
        ))));
    }
    let store = atelier_store(&state);
    let current = store.get_model_lease(claim_id).await.map_err(ops_error)?;
    if current.session_id != payload.session_id {
        return Err((
            StatusCode::CONFLICT,
            Json(model_ops_error_response(
                "conflict",
                format!(
                    "model-operation lease {claim_id} cannot be released from session {}; {HSK_HEADER_SESSION_ID} must match active lease session {}",
                    payload.session_id, current.session_id
                ),
            )),
        ));
    }
    let record = store
        .release_model_lease(claim_id, &actor)
        .await
        .map_err(ops_error)?;
    Ok(Json(model_operation_lease_response(record)))
}

/// POST /atelier/model-ops/action-receipts — record a model-operation action receipt under the
/// lease named by the headers. `params.thread_id` must equal the lease thread; the persisted
/// `actor_kind` is the lease executor kind; `session_id` in the body must match the lease.
async fn record_model_operation_action_receipt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RecordModelOperationActionReceiptRequest>,
) -> Result<(StatusCode, Json<ModelOperationActionReceiptResponse>), OpsError> {
    let actor = calling_model_ops_actor(&headers)?;
    let expected_thread_id = action_receipt_model_operation_thread_id(&payload.params)?;
    let lease = validate_model_operation_lease_required(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    if payload.session_id != lease.session_id {
        return Err(ops_error(AtelierError::Validation(format!(
            "session_id must match {HSK_HEADER_SESSION_ID}"
        ))));
    }
    if let Some(supplied) = payload.actor_kind.as_deref() {
        tracing::debug!(
            target: "handshake_core::atelier",
            supplied_actor_kind = %supplied,
            lease_executor_kind = %executor_kind_token(lease.executor_kind),
            "model-operation receipt actor_kind is derived from the validated lease executor"
        );
    }
    let status = ActionReceiptStatus::from_token(&payload.status).map_err(ops_error)?;
    let store = atelier_store(&state);
    let receipt = store
        .record_action_receipt(&NewActionReceipt {
            action_id: payload.action_id,
            actor_kind: executor_kind_token(lease.executor_kind).to_owned(),
            actor_id: actor,
            session_id: lease.session_id,
            thread_id: lease.thread_id,
            lease_claim_id: Some(lease.claim_id),
            params: payload.params,
            started_at_utc: payload.started_at_utc,
            completed_at_utc: payload.completed_at_utc,
            status,
            target_refs: payload.target_refs,
            evidence_refs: payload.evidence_refs,
            result_refs: payload.result_refs,
            error_class: payload.error_class,
            recovery_hint: payload.recovery_hint,
        })
        .await
        .map_err(ops_error)?;
    Ok((
        StatusCode::CREATED,
        Json(model_operation_action_receipt_response(receipt)?),
    ))
}

/// GET /atelier/model-ops/action-receipts/:receipt_id
async fn get_model_operation_action_receipt(
    State(state): State<AppState>,
    Path(receipt_id): Path<Uuid>,
) -> Result<Json<ModelOperationActionReceiptResponse>, OpsError> {
    let store = atelier_store(&state);
    let receipt = store
        .get_action_receipt(receipt_id)
        .await
        .map_err(ops_error)?;
    Ok(Json(model_operation_action_receipt_response(receipt)?))
}
