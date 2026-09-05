//! Pose / rig artifacts for the Photo Studio (WP-KERNEL-005, MT-PoseKit).
//!
//! Translates the legacy source `posekit` subsystem (`src/posekit/core.mjs`,
//! `src/posekit/poseDetection.worker.ts`) into the Handshake atelier domain.
//! legacy source computes OpenPose-style rigs in an Electron renderer / web worker and
//! serializes them to local JSON sidecars; Handshake forbids that model (no
//! SQLite, no Electron, no localhost authority, no in-module detector). The
//! actual keypoint detection (MediaPipe/OpenPose) runs OUT OF THIS MODULE as a
//! capability-gated Workflow-Engine job. This module is the GOVERNED DATA +
//! RECEIPT model the job writes through: it stores rig artifacts, OpenPose
//! keypoint arrays, head-pose quaternions, identity profiles, deterministic
//! sidecar refs, and BLOCKED calibration state, and emits an event per mutation.
//! It never opens a socket, spawns a process, or calls an external endpoint.
//!
//! Spec authority: master-spec-v02.189 module 10 (Photo Studio, 10.10; the
//! Calibration Panel 10.10.4.1.9 is recorded but kept BLOCKED/unresolved here,
//! never faked). Storage authority is the single Handshake store + EventLedger
//! only (MT-004), backed exclusively by embedded SurrealDB with no legacy
//! database fallback.
//!
//! legacy source source (intent only; SQLite/Electron/localhost/polling never copied):
//!   * `src/posekit/core.mjs` -- BODY_18 / HAND_21 / face-70 taxonomy,
//!     `createHeadPose` / `normalizeHeadPose` (YXZ Euler -> quaternion, deg
//!     limits yaw +-90 / pitch +-75 / roll +-45), `rigToOpenposeJson`
//!     (`pose_keypoints_2d` 18, `face_keypoints_2d` 70, `hand_*_keypoints_2d`
//!     21, zero-triple fill for absent face/hands), `createDefaultCalibration`.
//!   * `src/posekit/poseDetection.worker.ts` -- detector provider/status, the
//!     fallback ("deterministic body-18 fallback") detail string.
//!
//! Data contract (MT-PoseKit):
//!   * `atelier_pose_rig`        -- one detected/authored rig artifact for a
//!     (character, source media) pair, with canvas geometry, the OpenPose
//!     keypoint arrays as JSONB, the redacted detector descriptor, and the
//!     deterministic sidecar ref. Unique per `(character, source_media,
//!     content_hash)` for idempotent re-ingest.
//!   * `atelier_pose_head_pose`  -- the rig's head pose (yaw/pitch/roll degrees
//!     + normalized quaternion), one row per rig.
//!   * `atelier_pose_calibration`-- calibration state. Preserved as
//!     `unresolved` (BLOCKED) by default; the spec Calibration Panel is not yet
//!     implementable, so the row records the block reason instead of faking
//!     calibrated values.
//!   * `atelier_identity_profile`-- versioned face/reference identity profiles
//!     with provenance, append-only per character (mirrors legacy source portrait
//!     identity versioning). Unique per `(character, seq)`.

use chrono::{DateTime, Utc};
use image::{ImageEncoder, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::time::Duration;
use surrealdb::types::{RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use crate::storage::artifacts::{
    artifact_root_dir, read_artifact_manifest, resolve_workspace_root, sha256_hex,
    validate_artifact_content_hash, ArtifactLayer,
};

use super::{
    atelier_event_sql, event_ref_for_text, reject_legacy_runtime_ref, AtelierError, AtelierResult,
    AtelierStore,
};

struct PoseRow(serde_json::Map<String, serde_json::Value>);

impl PoseRow {
    fn get<T, I>(&self, field: I) -> T
    where
        T: serde::de::DeserializeOwned,
        I: AsRef<str>,
    {
        let field = field.as_ref();
        serde_json::from_value(
            self.0
                .get(field)
                .unwrap_or_else(|| panic!("missing persisted pose field {field}"))
                .clone(),
        )
        .unwrap_or_else(|err| panic!("invalid persisted pose field {field}: {err}"))
    }
}

fn pose_row(value: serde_json::Value) -> AtelierResult<PoseRow> {
    value
        .as_object()
        .cloned()
        .map(PoseRow)
        .ok_or_else(|| AtelierError::Internal("pose query returned a non-object row".to_owned()))
}

/// Pose / identity event families (extends the MT-005 coverage set). The parent
/// wires these into [`super::event_family::ALL`].
pub mod pose_event_family {
    /// A pose rig artifact was ingested for a character + source media.
    pub const POSE_RIG_INGESTED: &str = "atelier.pose.rig_ingested";
    /// A head pose (yaw/pitch/roll + quaternion) was recorded for a rig.
    pub const POSE_HEAD_POSE_RECORDED: &str = "atelier.pose.head_pose_recorded";
    /// A calibration record was set (typically BLOCKED/unresolved).
    pub const POSE_CALIBRATION_SET: &str = "atelier.pose.calibration_set";
    /// A typed OpenPose/conditioning sidecar artifact was registered.
    pub const POSE_SIDECAR_RECORDED: &str = "atelier.pose.sidecar_recorded";
    /// A pose workspace context state was appended.
    pub const POSE_CONTEXT_STATE_SET: &str = "atelier.pose.context_state_set";
    /// A multi-rig pose workspace tab/panel state was set.
    pub const POSE_WORKSPACE_RIG_STATE_SET: &str = "atelier.pose.workspace_rig_state_set";
    /// A versioned identity profile was appended for a character.
    pub const IDENTITY_PROFILE_APPENDED: &str = "atelier.pose.identity_profile_appended";
    /// A 512x512 identity crop artifact was registered for a profile version.
    pub const IDENTITY_CROP_ARTIFACT_RECORDED: &str =
        "atelier.pose.identity_crop_artifact_recorded";
    /// A planned/deferred/blocked pose feature was recorded (MT-115/116/117).
    pub const POSE_DEFERRED_FEATURE_RECORDED: &str = "atelier.pose.deferred_feature_recorded";

    /// All pose/identity event families, exported for parity/coverage proofs.
    pub const ALL: &[&str] = &[
        POSE_RIG_INGESTED,
        POSE_HEAD_POSE_RECORDED,
        POSE_CALIBRATION_SET,
        POSE_SIDECAR_RECORDED,
        POSE_CONTEXT_STATE_SET,
        POSE_WORKSPACE_RIG_STATE_SET,
        IDENTITY_PROFILE_APPENDED,
        IDENTITY_CROP_ARTIFACT_RECORDED,
        POSE_DEFERRED_FEATURE_RECORDED,
    ];
}

/// Re-export at module root so callers can write `pose::POSE_RIG_INGESTED`.
pub use pose_event_family::{
    IDENTITY_CROP_ARTIFACT_RECORDED, IDENTITY_PROFILE_APPENDED, POSE_CALIBRATION_SET,
    POSE_CONTEXT_STATE_SET, POSE_DEFERRED_FEATURE_RECORDED, POSE_HEAD_POSE_RECORDED,
    POSE_RIG_INGESTED, POSE_SIDECAR_RECORDED, POSE_WORKSPACE_RIG_STATE_SET,
};

pub const IDENTITY_CROP_ARTIFACT_MANIFEST_SCHEMA: &str =
    "hsk.atelier.identity_crop_artifact_manifest@1";

/// OpenPose keypoint-array cardinalities (legacy source `rigToOpenposeJson`):
/// body-18, face-70, hand-21. Each keypoint is an `(x, y, confidence)` triple,
/// so flattened arrays have `count * 3` numbers.
pub const BODY_KEYPOINT_COUNT: usize = 18;
pub const FACE_KEYPOINT_COUNT: usize = 70;
pub const HAND_KEYPOINT_COUNT: usize = 21;
/// WP-CKC-posekit-overhaul: schema id stamped into every native Posekit OpenPose
/// export (JSON payload, PNG manifest `hash_basis`, receipt).
pub const POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID: &str = "hsk.atelier.posekit.openpose_export@1";
pub const POSEKIT_OPENPOSE_EXPORT_WIDTH: i32 = 768;
pub const POSEKIT_OPENPOSE_EXPORT_HEIGHT: i32 = 768;

/// Provider/status of the detector that produced a rig. Mirrors legacy source
/// `detector.provider` / `detector.status` (e.g. `mediapipe.tasks-vision.pose`
/// detected, or the deterministic `fallback`). The detector runs out-of-module;
/// this only records which one the Workflow-Engine job reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectorStatus {
    /// Keypoints were detected by a model.
    Detected,
    /// legacy source "deterministic body-18 fallback" -- no model assets were used.
    Fallback,
    /// The detector failed; the rig is a placeholder pending re-run.
    Failed,
}

impl DetectorStatus {
    /// Stable lowercase DB token.
    pub fn as_token(self) -> &'static str {
        match self {
            DetectorStatus::Detected => "detected",
            DetectorStatus::Fallback => "fallback",
            DetectorStatus::Failed => "failed",
        }
    }

    /// Parse a stored token; unknown tokens are a validation error so a corrupt
    /// row never masquerades as a real detection.
    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "detected" => Ok(DetectorStatus::Detected),
            "fallback" => Ok(DetectorStatus::Fallback),
            "failed" => Ok(DetectorStatus::Failed),
            other => Err(AtelierError::Validation(format!(
                "unknown detector status token: {other}"
            ))),
        }
    }
}

/// Calibration state for a rig. legacy source `createDefaultCalibration` produces a live
/// calibration object, but the Handshake Calibration Panel (spec 10.10.4.1.9) is
/// not yet implementable. We therefore preserve calibration as an explicit
/// BLOCKED/unresolved record rather than fabricating calibrated values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationState {
    /// BLOCKED: calibration is intentionally not computed; values are absent and
    /// must not be faked. This is the default.
    Unresolved,
    /// Calibration was applied by a future capability-gated job (reserved; not
    /// produced by this module today).
    Resolved,
}

impl CalibrationState {
    pub fn as_token(self) -> &'static str {
        match self {
            CalibrationState::Unresolved => "unresolved",
            CalibrationState::Resolved => "resolved",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "unresolved" => Ok(CalibrationState::Unresolved),
            "resolved" => Ok(CalibrationState::Resolved),
            other => Err(AtelierError::Validation(format!(
                "unknown calibration state token: {other}"
            ))),
        }
    }
}

/// Kind of identity profile reference media (legacy source portrait/reference identity).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityProfileKind {
    /// A face identity profile (portrait crop / face embedding source).
    Face,
    /// A general reference identity profile (full-body / wardrobe reference).
    Reference,
}

impl IdentityProfileKind {
    pub fn as_token(self) -> &'static str {
        match self {
            IdentityProfileKind::Face => "face",
            IdentityProfileKind::Reference => "reference",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "face" => Ok(IdentityProfileKind::Face),
            "reference" => Ok(IdentityProfileKind::Reference),
            other => Err(AtelierError::Validation(format!(
                "unknown identity profile kind token: {other}"
            ))),
        }
    }
}

/// Canvas geometry for a rig (legacy source `canvas` / `image` width+height). Keypoint
/// coordinates are absolute pixels in this canvas space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasSize {
    pub width: i32,
    pub height: i32,
}

/// A detected/authored pose rig artifact (MT-PoseKit).
///
/// `keypoints_json` holds the OpenPose payload shape produced by legacy source
/// `rigToOpenposeJson`: `pose_keypoints_2d` (18 triples), `face_keypoints_2d`
/// (70 triples, zero-filled when absent), `hand_left_keypoints_2d` /
/// `hand_right_keypoints_2d` (21 triples each, zero-filled when a hand is
/// absent). It is stored verbatim as JSONB; structural validation happens on
/// the way in.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseRig {
    pub rig_id: Uuid,
    pub character_internal_id: Uuid,
    /// Optional FK to the source media asset (DAM, MT-015). The rig may also be
    /// authored without a stored asset (e.g. fallback rig), in which case this
    /// is None and `source_ref` carries the deterministic source identity.
    pub source_asset_id: Option<Uuid>,
    /// Stable source identity (legacy source `portraitImageId`); part of the idempotency
    /// key so re-detecting the same source never duplicates rigs.
    pub source_ref: String,
    /// Content hash of the rig payload for idempotent re-ingest and audit.
    pub content_hash: String,
    pub canvas: CanvasSize,
    pub detector_provider: String,
    pub detector_model: String,
    pub detector_model_version: String,
    pub source_asset_version_ref: Option<String>,
    pub source_asset_path_ref: Option<String>,
    pub confidence_available: bool,
    pub detector_status: DetectorStatus,
    pub error_reason: Option<String>,
    /// OpenPose keypoint arrays (body-18 / face-70 / hand-21), JSONB verbatim.
    pub keypoints_json: serde_json::Value,
    /// Deterministic sidecar ArtifactStore ref (legacy source wrote a JSON sidecar to
    /// disk; Handshake records an ArtifactStore ref instead of a local path).
    pub sidecar_ref: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to ingest a pose rig (the record a Workflow-Engine detection job
/// writes through). No detector execution happens here.
#[derive(Clone, Debug)]
pub struct NewPoseRig {
    pub character_internal_id: Uuid,
    pub source_asset_id: Option<Uuid>,
    pub source_ref: String,
    pub content_hash: String,
    pub canvas: CanvasSize,
    pub detector_provider: String,
    pub detector_model: String,
    pub detector_model_version: String,
    pub source_asset_version_ref: Option<String>,
    pub source_asset_path_ref: Option<String>,
    pub confidence_available: bool,
    pub detector_status: DetectorStatus,
    pub error_reason: Option<String>,
    pub keypoints_json: serde_json::Value,
    pub sidecar_ref: Option<String>,
}

/// Typed pose editor context mode. Stored append-only so switching modes never
/// deletes rigs, source media, or linked collections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoseContextKind {
    /// No source image, character, collection, or selected rig.
    Blank,
    /// One source media asset is active.
    SingleImage,
    /// A character is active.
    CharacterLinked,
    /// A collection is active.
    CollectionLinked,
}

impl PoseContextKind {
    pub fn as_token(self) -> &'static str {
        match self {
            PoseContextKind::Blank => "blank",
            PoseContextKind::SingleImage => "single_image",
            PoseContextKind::CharacterLinked => "character_linked",
            PoseContextKind::CollectionLinked => "collection_linked",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "blank" => Ok(PoseContextKind::Blank),
            "single_image" => Ok(PoseContextKind::SingleImage),
            "character_linked" => Ok(PoseContextKind::CharacterLinked),
            "collection_linked" => Ok(PoseContextKind::CollectionLinked),
            other => Err(AtelierError::Validation(format!(
                "unknown pose context kind token: {other}"
            ))),
        }
    }
}

/// Input to append a pose context state for a workspace.
#[derive(Clone, Debug)]
pub struct NewPoseContextState {
    pub workspace_ref: String,
    pub kind: PoseContextKind,
    pub source_asset_id: Option<Uuid>,
    pub character_internal_id: Option<Uuid>,
    pub collection_id: Option<Uuid>,
    pub selected_rig_id: Option<Uuid>,
    pub requested_by: String,
}

/// Persisted append-only pose context state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseContextState {
    pub context_id: Uuid,
    pub state_seq: i64,
    pub workspace_ref: String,
    pub kind: PoseContextKind,
    pub source_asset_id: Option<Uuid>,
    pub character_internal_id: Option<Uuid>,
    pub collection_id: Option<Uuid>,
    pub selected_rig_id: Option<Uuid>,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to set one rig tab's state inside a pose workspace.
#[derive(Clone, Debug)]
pub struct NewPoseWorkspaceRigState {
    pub workspace_ref: String,
    pub session_ref: String,
    pub rig_id: Uuid,
    pub open: bool,
    pub sort_order: i32,
    pub active: bool,
    pub dirty_calibration: bool,
    pub panel_state: serde_json::Value,
    pub requested_by: String,
}

/// Persisted multi-rig pose workspace tab/panel state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseWorkspaceRigState {
    pub workspace_ref: String,
    pub session_ref: String,
    pub rig_id: Uuid,
    pub character_internal_id: Uuid,
    pub source_asset_id: Option<Uuid>,
    pub source_ref: String,
    pub open: bool,
    pub sort_order: i32,
    pub active: bool,
    pub dirty_calibration: bool,
    pub panel_state: serde_json::Value,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to resolve a durable route to one rig inside a pose workspace.
#[derive(Clone, Debug)]
pub struct NewPoseWorkspaceRouteTarget {
    pub workspace_ref: String,
    pub session_ref: String,
    pub rig_id: Uuid,
    pub panel_id: String,
    pub requested_by: String,
}

/// Keyboard navigation actions supported by the multi-rig pose workspace.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PoseWorkspaceKeyboardAction {
    ActivateNextRig,
    ActivatePreviousRig,
}

impl PoseWorkspaceKeyboardAction {
    pub fn as_token(self) -> &'static str {
        match self {
            PoseWorkspaceKeyboardAction::ActivateNextRig => "activate_next_rig",
            PoseWorkspaceKeyboardAction::ActivatePreviousRig => "activate_previous_rig",
        }
    }
}

/// Input to apply keyboard navigation against the durable open-rig order.
#[derive(Clone, Debug)]
pub struct PoseWorkspaceKeyboardActionRequest {
    pub workspace_ref: String,
    pub session_ref: String,
    pub action: PoseWorkspaceKeyboardAction,
    pub panel_id: String,
    pub requested_by: String,
}

/// Resolved product route for a pose workspace rig.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseWorkspaceRouteResolution {
    pub route_ref: String,
    pub workspace_ref: String,
    pub session_ref: String,
    pub rig_id: Uuid,
    pub panel_id: String,
    pub active_sort_order: i32,
    pub open_rig_count: i32,
    pub keyboard_action: Option<PoseWorkspaceKeyboardAction>,
}

/// Typed sidecar artifact emitted by an out-of-module pose/OpenPose job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoseSidecarKind {
    /// OpenPose JSON payload sidecar.
    OpenPoseJson,
    /// Human-inspectable OpenPose PNG preview.
    OpenPosePng,
    /// PNG conditioning image for downstream generation workflows.
    ConditioningPng,
}

impl PoseSidecarKind {
    pub fn as_token(self) -> &'static str {
        match self {
            PoseSidecarKind::OpenPoseJson => "openpose_json",
            PoseSidecarKind::OpenPosePng => "openpose_png",
            PoseSidecarKind::ConditioningPng => "conditioning_png",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "openpose_json" => Ok(PoseSidecarKind::OpenPoseJson),
            "openpose_png" => Ok(PoseSidecarKind::OpenPosePng),
            "conditioning_png" => Ok(PoseSidecarKind::ConditioningPng),
            other => Err(AtelierError::Validation(format!(
                "unknown pose sidecar kind token: {other}"
            ))),
        }
    }

    fn expected_mime(self) -> &'static str {
        match self {
            PoseSidecarKind::OpenPoseJson => "application/json",
            PoseSidecarKind::OpenPosePng | PoseSidecarKind::ConditioningPng => "image/png",
        }
    }
}

/// Render lifecycle for a pose sidecar artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoseSidecarStatus {
    Rendered,
    Failed,
}

impl PoseSidecarStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            PoseSidecarStatus::Rendered => "rendered",
            PoseSidecarStatus::Failed => "failed",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "rendered" => Ok(PoseSidecarStatus::Rendered),
            "failed" => Ok(PoseSidecarStatus::Failed),
            other => Err(AtelierError::Validation(format!(
                "unknown pose sidecar status token: {other}"
            ))),
        }
    }
}

/// Input to register a typed pose sidecar artifact.
#[derive(Clone, Debug)]
pub struct NewPoseSidecar {
    pub rig_id: Uuid,
    pub kind: PoseSidecarKind,
    pub artifact_ref: String,
    pub manifest_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub mime: String,
    pub width: i32,
    pub height: i32,
    pub status: PoseSidecarStatus,
    pub error_message: Option<String>,
}

/// Persisted typed pose sidecar artifact.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseSidecar {
    pub sidecar_id: Uuid,
    pub rig_id: Uuid,
    pub source_asset_id: Option<Uuid>,
    pub source_ref: String,
    pub kind: PoseSidecarKind,
    pub role: String,
    pub artifact_ref: String,
    pub manifest_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub mime: String,
    pub width: i32,
    pub height: i32,
    pub status: PoseSidecarStatus,
    pub error_message: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

/// Gallery projection row for pose sidecars. These artifacts are discoverable
/// through pose-specific lookups, but hidden from normal media galleries.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseSidecarGalleryProjection {
    pub sidecar_id: Uuid,
    pub rig_id: Uuid,
    pub kind: PoseSidecarKind,
    pub artifact_ref: String,
    pub gallery_visible: bool,
    pub hidden_reason: String,
    pub jump_target: String,
}

/// Diagnostics-consumable strip row for source images backing pose rigs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseSourceImageStripItem {
    pub rig_id: Uuid,
    pub character_internal_id: Uuid,
    pub source_asset_id: Option<Uuid>,
    pub source_ref: String,
    pub artifact_ref: Option<String>,
    pub content_hash: Option<String>,
    pub mime: Option<String>,
    pub byte_len: Option<i64>,
    pub diagnostics_visible: bool,
    pub gallery_visible: bool,
    pub jump_target: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Diagnostics-consumable strip row for OpenPose sidecars.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseOpenPoseSidecarStripItem {
    pub sidecar_id: Uuid,
    pub rig_id: Uuid,
    pub source_asset_id: Option<Uuid>,
    pub source_ref: String,
    pub kind: PoseSidecarKind,
    pub role: String,
    pub artifact_ref: String,
    pub manifest_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub mime: String,
    pub width: i32,
    pub height: i32,
    pub status: PoseSidecarStatus,
    pub error_message: Option<String>,
    pub diagnostics_visible: bool,
    pub gallery_visible: bool,
    pub hidden_reason: String,
    pub jump_target: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Native Posekit view/export request. This is a render/view rotation, not
/// [`HeadPose`] calibration; yaw spans the operator 360 workflow as
/// -180..=180 degrees.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PosekitOpenPoseExportRequest {
    pub source_ref: String,
    #[serde(default)]
    pub rig_id: Option<Uuid>,
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub zoom: f32,
    pub include_face: bool,
    pub include_body: bool,
    pub include_hands: bool,
    #[serde(default)]
    pub marker_edits: Vec<PosekitMarkerEdit>,
    #[serde(default)]
    pub framing: PosekitExportFraming,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PosekitMarkerLayers {
    pub face: bool,
    pub body: bool,
    pub hands: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PosekitMarkerFamily {
    Body,
    Face,
    LeftHand,
    RightHand,
}

impl PosekitMarkerFamily {
    fn as_token(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Face => "face",
            Self::LeftHand => "left_hand",
            Self::RightHand => "right_hand",
        }
    }

    fn field_and_count(self) -> (&'static str, usize) {
        match self {
            Self::Body => ("pose_keypoints_2d", BODY_KEYPOINT_COUNT),
            Self::Face => ("face_keypoints_2d", FACE_KEYPOINT_COUNT),
            Self::LeftHand => ("hand_left_keypoints_2d", HAND_KEYPOINT_COUNT),
            Self::RightHand => ("hand_right_keypoints_2d", HAND_KEYPOINT_COUNT),
        }
    }

    fn layer_enabled(self, layers: &PosekitMarkerLayers) -> bool {
        match self {
            Self::Body => layers.body,
            Self::Face => layers.face,
            Self::LeftHand | Self::RightHand => layers.hands,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PosekitMarkerEditAction {
    Set,
    Add,
    Remove,
}

impl PosekitMarkerEditAction {
    fn as_token(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Add => "add",
            Self::Remove => "remove",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PosekitMarkerEdit {
    pub family: PosekitMarkerFamily,
    pub index: usize,
    pub action: PosekitMarkerEditAction,
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PosekitFramingPreset {
    #[default]
    Standard,
    FullBodyWithFeet,
    Portrait,
    Custom,
}

impl PosekitFramingPreset {
    fn as_token(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::FullBodyWithFeet => "full_body_with_feet",
            Self::Portrait => "portrait",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PosekitExportFraming {
    #[serde(default)]
    pub preset: PosekitFramingPreset,
    pub lens_mm: i32,
    pub padding_top_px: i32,
    pub padding_right_px: i32,
    pub padding_bottom_px: i32,
    pub padding_left_px: i32,
}

impl Default for PosekitExportFraming {
    fn default() -> Self {
        Self {
            preset: PosekitFramingPreset::Standard,
            lens_mm: 50,
            padding_top_px: 0,
            padding_right_px: 0,
            padding_bottom_px: 0,
            padding_left_px: 0,
        }
    }
}

/// A generated native Posekit OpenPose export: the OpenPose JSON, the rendered
/// PNG skeleton, and the content hashes the ArtifactStore manifests and
/// `atelier_pose_sidecar` rows are bound to.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PosekitOpenPoseExport {
    pub schema_id: String,
    pub source_ref: String,
    pub yaw_deg: i32,
    pub pitch_deg: i32,
    pub zoom_percent: i32,
    pub framing: PosekitExportFraming,
    pub marker_layers: PosekitMarkerLayers,
    pub applied_marker_edit_count: usize,
    pub width: i32,
    pub height: i32,
    pub openpose_json: serde_json::Value,
    pub openpose_json_bytes: Vec<u8>,
    pub openpose_png_bytes: Vec<u8>,
    pub openpose_json_sha256: String,
    pub openpose_png_sha256: String,
    pub content_hash: String,
    pub receipt_ref: String,
}

/// Head pose for a rig: yaw/pitch/roll in degrees plus the normalized
/// quaternion (legacy source `createHeadPose` / `normalizeHeadPose`, YXZ order).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HeadPose {
    pub rig_id: Uuid,
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    /// Normalized quaternion `[x, y, z, w]` (legacy source `quaternionToArray`).
    pub quaternion: [f64; 4],
    pub created_at_utc: DateTime<Utc>,
}

/// legacy source `HEAD_POSE_LIMITS`: yaw +-90, pitch +-75, roll +-45 (degrees).
const YAW_LIMIT_DEG: f64 = 90.0;
const PITCH_LIMIT_DEG: f64 = 75.0;
const ROLL_LIMIT_DEG: f64 = 45.0;

/// Convert legacy source YXZ Euler angles to `[x, y, z, w]` quaternion order.
fn quaternion_from_yxz_euler_degrees(yaw_deg: f64, pitch_deg: f64, roll_deg: f64) -> [f64; 4] {
    let x = pitch_deg.to_radians();
    let y = yaw_deg.to_radians();
    let z = roll_deg.to_radians();
    let c1 = (x / 2.0).cos();
    let c2 = (y / 2.0).cos();
    let c3 = (z / 2.0).cos();
    let s1 = (x / 2.0).sin();
    let s2 = (y / 2.0).sin();
    let s3 = (z / 2.0).sin();

    [
        s1 * c2 * c3 + c1 * s2 * s3,
        c1 * s2 * c3 - s1 * c2 * s3,
        c1 * c2 * s3 - s1 * s2 * c3,
        c1 * c2 * c3 + s1 * s2 * s3,
    ]
}

/// A calibration record for a rig, kept BLOCKED/unresolved by default.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Calibration {
    pub rig_id: Uuid,
    pub state: CalibrationState,
    /// Why calibration is unresolved (BLOCKED reason); preserved, never faked.
    pub block_reason: Option<String>,
    pub head_pose_ref: Option<String>,
    pub marker_visibility: CalibrationMarkerVisibility,
    pub marker_colors: CalibrationMarkerColors,
    pub hand_rows: Vec<CalibrationHandRow>,
    pub history_refs: Vec<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Visibility flags for marker families preserved by pose calibration.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibrationMarkerVisibility {
    pub body: bool,
    pub face: bool,
    pub left_hand: bool,
    pub right_hand: bool,
}

/// Marker color refs/names preserved by pose calibration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibrationMarkerColors {
    pub body: String,
    pub face: String,
    pub left_hand: String,
    pub right_hand: String,
}

impl Default for CalibrationMarkerColors {
    fn default() -> Self {
        Self {
            body: "unresolved".to_string(),
            face: "unresolved".to_string(),
            left_hand: "unresolved".to_string(),
            right_hand: "unresolved".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationHandKind {
    Left,
    Right,
}

/// One typed hand calibration row. Marker count stays explicit so flattening or
/// hand-side confusion is caught by validation instead of silently compressed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibrationHandRow {
    pub hand: CalibrationHandKind,
    pub visible: bool,
    pub marker_count: i32,
    pub confidence_available: bool,
}

/// Input to set a full typed pose calibration record.
#[derive(Clone, Debug)]
pub struct NewPoseCalibration {
    pub rig_id: Uuid,
    pub state: CalibrationState,
    pub block_reason: Option<String>,
    pub head_pose_ref: Option<String>,
    pub marker_visibility: CalibrationMarkerVisibility,
    pub marker_colors: CalibrationMarkerColors,
    pub hand_rows: Vec<CalibrationHandRow>,
    pub history_refs: Vec<String>,
}

/// A versioned identity profile for a character (append-only per character).
///
/// `provenance` is free-form lineage (where the reference came from). Secret
/// material in provenance is redacted before storage and before any event
/// payload (no raw cookies/tokens/auth ever persisted).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityProfile {
    pub profile_id: Uuid,
    pub character_internal_id: Uuid,
    pub seq: i64,
    pub version: i64,
    pub kind: IdentityProfileKind,
    pub name: String,
    pub description: String,
    /// Optional FK to the reference media asset (DAM, MT-015).
    pub reference_asset_id: Option<Uuid>,
    /// Stable reference id (legacy source portrait/reference image id).
    pub reference_ref: String,
    pub source_ref: Option<String>,
    pub crop_ref: Option<String>,
    pub artifact_ref: Option<String>,
    /// Free-form provenance/lineage (redacted of any secret material).
    pub provenance: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to append an identity profile version.
#[derive(Clone, Debug)]
pub struct NewIdentityProfile {
    pub character_internal_id: Uuid,
    pub kind: IdentityProfileKind,
    pub name: String,
    pub description: String,
    pub reference_asset_id: Option<Uuid>,
    pub reference_ref: String,
    pub source_ref: Option<String>,
    pub crop_ref: Option<String>,
    pub artifact_ref: Option<String>,
    pub provenance: String,
}

/// Input to update a mutable identity profile record without changing append sequence.
#[derive(Clone, Debug)]
pub struct UpdateIdentityProfile {
    pub profile_id: Uuid,
    pub name: String,
    pub description: String,
    pub source_ref: Option<String>,
    pub crop_ref: Option<String>,
    pub artifact_ref: Option<String>,
    pub requested_by: String,
}

/// Source-image crop box used to create a normalized 512x512 identity crop.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityCropBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Landmark captured inside the normalized identity crop.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IdentityCropLandmark {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub confidence: Option<f64>,
}

/// A persisted 512x512 face crop artifact linked to a concrete profile version.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IdentityCropArtifact {
    pub crop_id: Uuid,
    pub profile_id: Uuid,
    pub profile_version: i64,
    pub character_internal_id: Uuid,
    pub source_ref: String,
    pub crop_box: IdentityCropBox,
    pub landmarks: Vec<IdentityCropLandmark>,
    pub artifact_ref: String,
    pub manifest_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub mime: String,
    pub width: i32,
    pub height: i32,
    pub manifest: serde_json::Value,
    pub created_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to record an identity crop artifact. The current profile version is
/// read from the store and stored on the crop record.
#[derive(Clone, Debug)]
pub struct NewIdentityCropArtifact {
    pub profile_id: Uuid,
    pub source_ref: String,
    pub crop_box: IdentityCropBox,
    pub landmarks: Vec<IdentityCropLandmark>,
    pub artifact_ref: String,
    pub manifest_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub mime: String,
    pub width: i32,
    pub height: i32,
    pub created_by: String,
}

/// Redact secret-looking material (cookies/tokens/auth/keys) from a free-form
/// string before persistence and before any event payload. Mirrors the
/// settings.rs redaction stance: raw secrets are never stored. Conservative:
/// any line whose key looks secret has its value masked.
fn redact_secrets(raw: &str) -> String {
    const SECRET_HINTS: &[&str] = &[
        "cookie",
        "token",
        "secret",
        "password",
        "passwd",
        "authorization",
        "auth",
        "api_key",
        "apikey",
        "api-key",
        "bearer",
        "session",
        "x-api-key",
        "set-cookie",
        "private_key",
    ];
    const REDACTED: &str = "[REDACTED]";
    let mut out_lines: Vec<String> = Vec::new();
    for line in raw.lines() {
        let lower = line.to_ascii_lowercase();
        // Bearer tokens appear inline without a key=value shape.
        if lower.contains("bearer ") {
            out_lines.push(format!("{REDACTED} (bearer)"));
            continue;
        }
        // key: value / key = value shapes where the key looks secret.
        let sep = line.find(['=', ':']);
        if let Some(idx) = sep {
            let key = line[..idx].trim().to_ascii_lowercase();
            let key_is_secret = SECRET_HINTS.iter().any(|h| key.contains(h));
            if key_is_secret {
                let prefix = &line[..idx];
                let delim = &line[idx..=idx];
                out_lines.push(format!("{prefix}{delim} {REDACTED}"));
                continue;
            }
        }
        out_lines.push(line.to_string());
    }
    out_lines.join("\n")
}

fn validate_identity_profile_text(
    field: &str,
    value: &str,
    allow_empty: bool,
) -> AtelierResult<()> {
    if value.trim() != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be padded"
        )));
    }
    if !allow_empty && value.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn validate_optional_identity_ref(field: &str, value: Option<&String>) -> AtelierResult<()> {
    if let Some(value) = value {
        if value.trim().is_empty() || value.trim() != value {
            return Err(AtelierError::Validation(format!(
                "{field} must be non-empty and unpadded when present"
            )));
        }
        reject_legacy_runtime_ref(field, value)?;
    }
    Ok(())
}

fn validate_new_identity_profile(new: &NewIdentityProfile) -> AtelierResult<()> {
    validate_identity_profile_text("name", &new.name, false)?;
    validate_identity_profile_text("description", &new.description, true)?;
    if new.reference_ref.trim().is_empty() {
        return Err(AtelierError::Validation(
            "reference_ref must not be empty".into(),
        ));
    }
    reject_legacy_runtime_ref("reference_ref", &new.reference_ref)?;
    validate_optional_identity_ref("source_ref", new.source_ref.as_ref())?;
    validate_optional_identity_ref("crop_ref", new.crop_ref.as_ref())?;
    validate_optional_identity_ref("artifact_ref", new.artifact_ref.as_ref())?;
    Ok(())
}

fn validate_update_identity_profile(update: &UpdateIdentityProfile) -> AtelierResult<()> {
    validate_identity_profile_text("name", &update.name, false)?;
    validate_identity_profile_text("description", &update.description, true)?;
    validate_optional_identity_ref("source_ref", update.source_ref.as_ref())?;
    validate_optional_identity_ref("crop_ref", update.crop_ref.as_ref())?;
    validate_optional_identity_ref("artifact_ref", update.artifact_ref.as_ref())?;
    reject_legacy_runtime_ref("requested_by", &update.requested_by)?;
    Ok(())
}

fn validate_identity_crop_box(crop_box: &IdentityCropBox) -> AtelierResult<()> {
    if crop_box.x < 0 || crop_box.y < 0 || crop_box.width <= 0 || crop_box.height <= 0 {
        return Err(AtelierError::Validation(
            "identity crop_box must use non-negative origin and positive dimensions".into(),
        ));
    }
    Ok(())
}

fn validate_identity_crop_landmarks(landmarks: &[IdentityCropLandmark]) -> AtelierResult<()> {
    if landmarks.is_empty() {
        return Err(AtelierError::Validation(
            "identity crop landmarks must not be empty".into(),
        ));
    }
    for landmark in landmarks {
        validate_pose_workspace_panel_id(&landmark.name)?;
        if !landmark.x.is_finite()
            || !landmark.y.is_finite()
            || landmark.x < 0.0
            || landmark.y < 0.0
            || landmark.x > 512.0
            || landmark.y > 512.0
        {
            return Err(AtelierError::Validation(
                "identity crop landmark x/y must be finite and inside the 512x512 crop".into(),
            ));
        }
        if let Some(confidence) = landmark.confidence {
            if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
                return Err(AtelierError::Validation(
                    "identity crop landmark confidence must be between 0 and 1".into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_identity_crop_artifact(new: &NewIdentityCropArtifact) -> AtelierResult<()> {
    validate_pose_context_ref("source_ref", &new.source_ref)?;
    validate_identity_crop_box(&new.crop_box)?;
    validate_identity_crop_landmarks(&new.landmarks)?;
    validate_pose_sidecar_artifact_ref(&new.artifact_ref)?;
    validate_pose_sidecar_manifest_ref(&new.artifact_ref, &new.manifest_ref)?;
    validate_pose_content_hash(&new.content_hash)?;
    if new.byte_len <= 0 {
        return Err(AtelierError::Validation(
            "identity crop byte_len must be positive".into(),
        ));
    }
    if new.mime != "image/png" {
        return Err(AtelierError::Validation(
            "identity crop artifact mime must be image/png".into(),
        ));
    }
    if new.width != 512 || new.height != 512 {
        return Err(AtelierError::Validation(
            "identity crop artifact must be exactly 512x512".into(),
        ));
    }
    reject_legacy_runtime_ref("created_by", &new.created_by)?;
    Ok(())
}

fn validate_detector_error_reason(
    status: DetectorStatus,
    error_reason: Option<&str>,
) -> AtelierResult<()> {
    match status {
        DetectorStatus::Detected => {
            if error_reason.is_some() {
                return Err(AtelierError::Validation(
                    "error_reason must be absent when detector_status is detected".into(),
                ));
            }
            Ok(())
        }
        DetectorStatus::Fallback | DetectorStatus::Failed => {
            let reason = error_reason.ok_or_else(|| {
                AtelierError::Validation(
                    "error_reason is required when detector_status is fallback or failed".into(),
                )
            })?;
            reject_legacy_runtime_ref("error_reason", reason)?;
            Ok(())
        }
    }
}

/// Validate the OpenPose keypoint payload shape (legacy source `rigToOpenposeJson`).
/// Body must be 18 triples; when present, face must be 70 triples and each hand
/// 21 triples. Absent face/hands are allowed (legacy source zero-fills them). This is a
/// structural gate, not detector execution.
fn validate_keypoints(json: &serde_json::Value) -> AtelierResult<()> {
    let person = json
        .get("people")
        .and_then(|p| p.as_array())
        .and_then(|p| p.first())
        .ok_or_else(|| {
            AtelierError::Validation(
                "pose keypoints_json must contain a non-empty people[] array".into(),
            )
        })?;

    let check = |field: &str, expected: usize, required: bool| -> AtelierResult<()> {
        match person.get(field) {
            Some(serde_json::Value::Array(arr)) => {
                if arr.len() != expected * 3 {
                    return Err(AtelierError::Validation(format!(
                        "pose keypoints field {field} must have {} numbers ({} triples), got {}",
                        expected * 3,
                        expected,
                        arr.len()
                    )));
                }
                Ok(())
            }
            Some(serde_json::Value::Null) | None => {
                if required {
                    Err(AtelierError::Validation(format!(
                        "pose keypoints field {field} is required"
                    )))
                } else {
                    Ok(())
                }
            }
            Some(_) => Err(AtelierError::Validation(format!(
                "pose keypoints field {field} must be an array"
            ))),
        }
    };

    check("pose_keypoints_2d", BODY_KEYPOINT_COUNT, true)?;
    check("face_keypoints_2d", FACE_KEYPOINT_COUNT, false)?;
    check("hand_left_keypoints_2d", HAND_KEYPOINT_COUNT, false)?;
    check("hand_right_keypoints_2d", HAND_KEYPOINT_COUNT, false)?;
    Ok(())
}

/// Generate a procedural Posekit OpenPose export (no stored rig keypoints):
/// the skeleton is synthesized from yaw/pitch/zoom, framed, marker-edited,
/// validated, and rendered to a 768x768 PNG. Deterministic for equal input.
pub fn generate_posekit_openpose_export(
    request: &PosekitOpenPoseExportRequest,
) -> AtelierResult<PosekitOpenPoseExport> {
    generate_posekit_openpose_export_with_source_keypoints(request, None)
}

/// Generate a Posekit OpenPose export by projecting a rig's stored
/// `keypoints_json` through the requested view rotation instead of the
/// procedural skeleton.
pub fn generate_posekit_openpose_export_from_keypoints(
    request: &PosekitOpenPoseExportRequest,
    source_keypoints: &serde_json::Value,
) -> AtelierResult<PosekitOpenPoseExport> {
    generate_posekit_openpose_export_with_source_keypoints(request, Some(source_keypoints))
}

fn generate_posekit_openpose_export_with_source_keypoints(
    request: &PosekitOpenPoseExportRequest,
    source_keypoints: Option<&serde_json::Value>,
) -> AtelierResult<PosekitOpenPoseExport> {
    validate_posekit_openpose_export_request(request)?;
    let yaw = request.yaw_deg.round() as i32;
    let pitch = request.pitch_deg.round() as i32;
    let zoom_percent = (request.zoom.clamp(0.4, 2.2) * 100.0).round() as i32;
    let marker_layers = PosekitMarkerLayers {
        face: request.include_face,
        body: request.include_body,
        hands: request.include_hands,
    };
    let mut openpose_json = match source_keypoints {
        Some(source_keypoints) => posekit_openpose_json_from_source_keypoints(
            request,
            yaw,
            pitch,
            zoom_percent,
            source_keypoints,
        )?,
        None => posekit_openpose_json(request, yaw, pitch, zoom_percent),
    };
    apply_posekit_framing(&mut openpose_json, &request.framing)?;
    let applied_marker_edit_count =
        apply_posekit_marker_edits(&mut openpose_json, &request.marker_edits, &marker_layers)?;
    validate_keypoints(&openpose_json)?;
    validate_posekit_export_keypoints(&openpose_json, &marker_layers)?;
    let openpose_json_bytes = serde_json::to_vec(&openpose_json)
        .map_err(|err| AtelierError::Validation(err.to_string()))?;
    let openpose_png_bytes = render_posekit_openpose_png(&openpose_json)?;
    let openpose_json_sha256 = sha256_hex(&openpose_json_bytes);
    let openpose_png_sha256 = sha256_hex(&openpose_png_bytes);
    let content_hash = sha256_hex_joined(&[&openpose_json_bytes, &openpose_png_bytes]);
    let receipt_ref = format!("preview://atelier/posekit/openpose/{content_hash}/receipt");
    Ok(PosekitOpenPoseExport {
        schema_id: POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID.to_string(),
        source_ref: request.source_ref.trim().to_string(),
        yaw_deg: yaw,
        pitch_deg: pitch,
        zoom_percent,
        framing: request.framing,
        marker_layers,
        applied_marker_edit_count,
        width: POSEKIT_OPENPOSE_EXPORT_WIDTH,
        height: POSEKIT_OPENPOSE_EXPORT_HEIGHT,
        openpose_json,
        openpose_json_bytes,
        openpose_png_bytes,
        openpose_json_sha256,
        openpose_png_sha256,
        content_hash,
        receipt_ref,
    })
}

fn validate_posekit_openpose_export_request(
    request: &PosekitOpenPoseExportRequest,
) -> AtelierResult<()> {
    if request.source_ref.trim().is_empty() || request.source_ref.trim() != request.source_ref {
        return Err(AtelierError::Validation(
            "Posekit OpenPose export source_ref must be non-empty and unpadded".into(),
        ));
    }
    validate_pose_context_ref("source_ref", &request.source_ref)?;
    validate_finite_range("yaw_deg", request.yaw_deg, -180.0, 180.0)?;
    validate_finite_range("pitch_deg", request.pitch_deg, -45.0, 45.0)?;
    validate_finite_range("zoom", request.zoom, 0.4, 2.2)?;
    validate_posekit_framing(&request.framing)?;
    if !(request.include_face || request.include_body || request.include_hands) {
        return Err(AtelierError::Validation(
            "Posekit OpenPose export requires at least one marker layer".into(),
        ));
    }
    Ok(())
}

fn validate_finite_range(field: &str, value: f32, min: f32, max: f32) -> AtelierResult<()> {
    if !value.is_finite() {
        return Err(AtelierError::Validation(format!(
            "Posekit OpenPose export {field} must be finite"
        )));
    }
    if value < min || value > max {
        return Err(AtelierError::Validation(format!(
            "Posekit OpenPose export {field}={value} outside [{min}, {max}]"
        )));
    }
    Ok(())
}

fn posekit_openpose_json(
    request: &PosekitOpenPoseExportRequest,
    yaw_deg: i32,
    pitch_deg: i32,
    zoom_percent: i32,
) -> serde_json::Value {
    serde_json::json!({
        "version": 1.3,
        "handshake_schema": POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID,
        "source_ref": request.source_ref.trim(),
        "rig_id": request.rig_id.map(|rig_id| rig_id.to_string()),
        "canvas": {
            "width": POSEKIT_OPENPOSE_EXPORT_WIDTH,
            "height": POSEKIT_OPENPOSE_EXPORT_HEIGHT,
        },
        "pose_state": {
            "yaw_deg": yaw_deg,
            "pitch_deg": pitch_deg,
            "zoom_percent": zoom_percent,
            "openpose_generation": {
                "mode": "procedural-posekit-preview",
                "yaw_deg": yaw_deg,
                "pitch_deg": pitch_deg,
                "zoom_percent": zoom_percent,
            },
            "marker_layers": {
                "face": request.include_face,
                "body": request.include_body,
                "hands": request.include_hands,
            },
            "framing": posekit_framing_json(&request.framing),
            "marker_edits": posekit_marker_edits_json(&request.marker_edits),
        },
        "people": [{
            "pose_keypoints_2d": posekit_body_keypoints(
                request.yaw_deg,
                request.pitch_deg,
                request.zoom,
                request.include_body,
            ),
            "face_keypoints_2d": posekit_face_keypoints(
                request.yaw_deg,
                request.pitch_deg,
                request.zoom,
                request.include_face,
            ),
            "hand_left_keypoints_2d": posekit_hand_keypoints(
                request.yaw_deg,
                request.pitch_deg,
                request.zoom,
                request.include_hands,
                -1.0,
            ),
            "hand_right_keypoints_2d": posekit_hand_keypoints(
                request.yaw_deg,
                request.pitch_deg,
                request.zoom,
                request.include_hands,
                1.0,
            ),
        }],
    })
}

fn posekit_openpose_json_from_source_keypoints(
    request: &PosekitOpenPoseExportRequest,
    yaw_deg: i32,
    pitch_deg: i32,
    zoom_percent: i32,
    source_keypoints: &serde_json::Value,
) -> AtelierResult<serde_json::Value> {
    validate_keypoints(source_keypoints)?;
    validate_source_keypoints_for_posekit_projection(source_keypoints)?;
    Ok(serde_json::json!({
        "version": 1.3,
        "handshake_schema": POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID,
        "source_ref": request.source_ref.trim(),
        "rig_id": request.rig_id.map(|rig_id| rig_id.to_string()),
        "source_keypoints_ref": "atelier_pose_rig.keypoints_json",
        "canvas": {
            "width": POSEKIT_OPENPOSE_EXPORT_WIDTH,
            "height": POSEKIT_OPENPOSE_EXPORT_HEIGHT,
        },
        "pose_state": {
            "yaw_deg": yaw_deg,
            "pitch_deg": pitch_deg,
            "zoom_percent": zoom_percent,
            "source_keypoint_projection": {
                "mode": "native-rig-to-openpose",
                "yaw_deg": yaw_deg,
                "pitch_deg": pitch_deg,
                "zoom_percent": zoom_percent,
            },
            "marker_layers": {
                "face": request.include_face,
                "body": request.include_body,
                "hands": request.include_hands,
            },
            "framing": posekit_framing_json(&request.framing),
            "marker_edits": posekit_marker_edits_json(&request.marker_edits),
        },
        "people": [{
            "pose_keypoints_2d": if request.include_body {
                projected_source_keypoint_array(
                    request,
                    source_keypoints,
                    "pose_keypoints_2d",
                    BODY_KEYPOINT_COUNT,
                    true,
                )?
            } else {
                zero_keypoints(BODY_KEYPOINT_COUNT)
            },
            "face_keypoints_2d": if request.include_face {
                projected_source_keypoint_array(
                    request,
                    source_keypoints,
                    "face_keypoints_2d",
                    FACE_KEYPOINT_COUNT,
                    false,
                )?
            } else {
                zero_keypoints(FACE_KEYPOINT_COUNT)
            },
            "hand_left_keypoints_2d": if request.include_hands {
                projected_source_keypoint_array(
                    request,
                    source_keypoints,
                    "hand_left_keypoints_2d",
                    HAND_KEYPOINT_COUNT,
                    false,
                )?
            } else {
                zero_keypoints(HAND_KEYPOINT_COUNT)
            },
            "hand_right_keypoints_2d": if request.include_hands {
                projected_source_keypoint_array(
                    request,
                    source_keypoints,
                    "hand_right_keypoints_2d",
                    HAND_KEYPOINT_COUNT,
                    false,
                )?
            } else {
                zero_keypoints(HAND_KEYPOINT_COUNT)
            },
        }],
    }))
}

fn projected_source_keypoint_array(
    request: &PosekitOpenPoseExportRequest,
    source_keypoints: &serde_json::Value,
    field: &str,
    count: usize,
    required: bool,
) -> AtelierResult<Vec<f32>> {
    let mut points = source_keypoint_array(source_keypoints, field, count, required)?;
    apply_posekit_source_projection(&mut points, request);
    Ok(points)
}

fn source_keypoint_array(
    source_keypoints: &serde_json::Value,
    field: &str,
    count: usize,
    required: bool,
) -> AtelierResult<Vec<f32>> {
    let Some(person) = source_keypoints
        .get("people")
        .and_then(|people| people.as_array())
        .and_then(|people| people.first())
    else {
        return Err(AtelierError::Validation(
            "pose keypoints_json must contain a non-empty people[] array".into(),
        ));
    };
    let Some(value) = person.get(field) else {
        return if required {
            Err(AtelierError::Validation(format!(
                "pose keypoints_json missing required {field}"
            )))
        } else {
            Ok(zero_keypoints(count))
        };
    };
    let array = value.as_array().ok_or_else(|| {
        AtelierError::Validation(format!("pose keypoints field {field} must be an array"))
    })?;
    if array.len() != count * 3 {
        return Err(AtelierError::Validation(format!(
            "pose keypoints field {field} must have {} values",
            count * 3
        )));
    }
    array
        .iter()
        .map(|value| {
            value.as_f64().map(|value| value as f32).ok_or_else(|| {
                AtelierError::Validation(format!(
                    "pose keypoints field {field} contains a non-number"
                ))
            })
        })
        .collect()
}

fn validate_source_keypoints_for_posekit_projection(
    source_keypoints: &serde_json::Value,
) -> AtelierResult<()> {
    let Some(person) = source_keypoints
        .get("people")
        .and_then(|people| people.as_array())
        .and_then(|people| people.first())
    else {
        return Err(AtelierError::Validation(
            "pose keypoints_json must contain a non-empty people[] array".into(),
        ));
    };

    for (field, count, required) in [
        ("pose_keypoints_2d", BODY_KEYPOINT_COUNT, true),
        ("face_keypoints_2d", FACE_KEYPOINT_COUNT, false),
        ("hand_left_keypoints_2d", HAND_KEYPOINT_COUNT, false),
        ("hand_right_keypoints_2d", HAND_KEYPOINT_COUNT, false),
    ] {
        let Some(value) = person.get(field) else {
            if required {
                return Err(AtelierError::Validation(format!(
                    "pose keypoints_json missing required {field}"
                )));
            }
            continue;
        };
        if value.is_null() {
            if required {
                return Err(AtelierError::Validation(format!(
                    "pose keypoints_json missing required {field}"
                )));
            }
            continue;
        }
        let array = value.as_array().ok_or_else(|| {
            AtelierError::Validation(format!("pose keypoints field {field} must be an array"))
        })?;
        if array.len() != count * 3 {
            return Err(AtelierError::Validation(format!(
                "pose keypoints field {field} must have {} values",
                count * 3
            )));
        }
        for triple in array.chunks_exact(3) {
            let x = value_as_f32(&triple[0], field)?;
            let y = value_as_f32(&triple[1], field)?;
            let confidence = value_as_f32(&triple[2], field)?;
            if !(0.0..=1.0).contains(&confidence) {
                return Err(AtelierError::Validation(format!(
                    "pose source keypoints field {field} confidence must be in 0..=1 before projection"
                )));
            }
            if confidence <= 0.0 {
                continue;
            }
            if x < 0.0
                || y < 0.0
                || x > POSEKIT_OPENPOSE_EXPORT_WIDTH as f32
                || y > POSEKIT_OPENPOSE_EXPORT_HEIGHT as f32
            {
                return Err(AtelierError::Validation(format!(
                    "pose source keypoints field {field} has a visible point outside the export canvas before projection"
                )));
            }
        }
    }

    Ok(())
}

fn apply_posekit_source_projection(points: &mut [f32], request: &PosekitOpenPoseExportRequest) {
    let yaw_bias = request.yaw_deg.clamp(-180.0, 180.0) / 180.0;
    let yaw_squash = 1.0 - yaw_bias.abs() * 0.35;
    let pitch_shift = request.pitch_deg.clamp(-45.0, 45.0) / 45.0 * 42.0;
    let zoom = request.zoom.clamp(0.4, 2.2);
    let center_x = POSEKIT_OPENPOSE_EXPORT_WIDTH as f32 * 0.5;
    let center_y = POSEKIT_OPENPOSE_EXPORT_HEIGHT as f32 * 0.5;

    for triple in points.chunks_exact_mut(3) {
        let confidence = triple[2];
        if confidence <= 0.0 {
            continue;
        }

        let dx = (triple[0] - center_x) * zoom;
        let dy = (triple[1] - center_y) * zoom;
        let projected_x = center_x + dx * yaw_squash + yaw_bias * 72.0;
        let projected_y = center_y + dy + pitch_shift;

        triple[0] = round_posekit_coordinate(projected_x)
            .clamp(1.0, (POSEKIT_OPENPOSE_EXPORT_WIDTH - 1) as f64) as f32;
        triple[1] = round_posekit_coordinate(projected_y)
            .clamp(1.0, (POSEKIT_OPENPOSE_EXPORT_HEIGHT - 1) as f64) as f32;
        triple[2] = round_posekit_confidence(confidence.clamp(0.0, 1.0)) as f32;
    }
}

fn validate_posekit_framing(framing: &PosekitExportFraming) -> AtelierResult<()> {
    if !(18..=120).contains(&framing.lens_mm) {
        return Err(AtelierError::Validation(
            "Posekit OpenPose export lens_mm must be in 18..=120".into(),
        ));
    }
    for (field, value) in [
        ("padding_top_px", framing.padding_top_px),
        ("padding_right_px", framing.padding_right_px),
        ("padding_bottom_px", framing.padding_bottom_px),
        ("padding_left_px", framing.padding_left_px),
    ] {
        if !(0..=256).contains(&value) {
            return Err(AtelierError::Validation(format!(
                "Posekit OpenPose export {field} must be in 0..=256"
            )));
        }
    }
    let content_width =
        POSEKIT_OPENPOSE_EXPORT_WIDTH - framing.padding_left_px - framing.padding_right_px;
    let content_height =
        POSEKIT_OPENPOSE_EXPORT_HEIGHT - framing.padding_top_px - framing.padding_bottom_px;
    if content_width < 128 || content_height < 128 {
        return Err(AtelierError::Validation(
            "Posekit OpenPose export black-space padding leaves less than 128px content area"
                .into(),
        ));
    }
    Ok(())
}

fn posekit_framing_json(framing: &PosekitExportFraming) -> serde_json::Value {
    serde_json::json!({
        "preset": framing.preset.as_token(),
        "lens_mm": framing.lens_mm,
        "padding_top_px": framing.padding_top_px,
        "padding_right_px": framing.padding_right_px,
        "padding_bottom_px": framing.padding_bottom_px,
        "padding_left_px": framing.padding_left_px,
        "content_rect": {
            "x": framing.padding_left_px,
            "y": framing.padding_top_px,
            "width": POSEKIT_OPENPOSE_EXPORT_WIDTH - framing.padding_left_px - framing.padding_right_px,
            "height": POSEKIT_OPENPOSE_EXPORT_HEIGHT - framing.padding_top_px - framing.padding_bottom_px,
        },
    })
}

fn posekit_marker_edits_json(edits: &[PosekitMarkerEdit]) -> serde_json::Value {
    serde_json::Value::Array(
        edits
            .iter()
            .map(|edit| {
                serde_json::json!({
                    "family": edit.family.as_token(),
                    "index": edit.index,
                    "action": edit.action.as_token(),
                    "x": edit.x,
                    "y": edit.y,
                    "confidence": edit.confidence,
                })
            })
            .collect(),
    )
}

fn apply_posekit_framing(
    openpose_json: &mut serde_json::Value,
    framing: &PosekitExportFraming,
) -> AtelierResult<()> {
    validate_posekit_framing(framing)?;
    let scale = framing.lens_mm as f32 / 50.0;
    let source_center_x = POSEKIT_OPENPOSE_EXPORT_WIDTH as f32 * 0.5;
    let source_center_y = POSEKIT_OPENPOSE_EXPORT_HEIGHT as f32 * 0.5;
    let content_width =
        (POSEKIT_OPENPOSE_EXPORT_WIDTH - framing.padding_left_px - framing.padding_right_px) as f32;
    let content_height = (POSEKIT_OPENPOSE_EXPORT_HEIGHT
        - framing.padding_top_px
        - framing.padding_bottom_px) as f32;
    let content_center_x = framing.padding_left_px as f32 + content_width * 0.5;
    let content_center_y = framing.padding_top_px as f32 + content_height * 0.5;

    for field in [
        "pose_keypoints_2d",
        "face_keypoints_2d",
        "hand_left_keypoints_2d",
        "hand_right_keypoints_2d",
    ] {
        let values = openpose_keypoint_array_mut(openpose_json, field)?;
        for triple in values.chunks_exact_mut(3) {
            let confidence = value_as_f32(&triple[2], field)?;
            if confidence <= 0.0 {
                continue;
            }
            let x = value_as_f32(&triple[0], field)?;
            let y = value_as_f32(&triple[1], field)?;
            let framed_x = content_center_x + (x - source_center_x) * scale;
            let framed_y = content_center_y + (y - source_center_y) * scale;
            triple[0] = serde_json::json!(round_posekit_coordinate(framed_x));
            triple[1] = serde_json::json!(round_posekit_coordinate(framed_y));
        }
    }
    if let Some(pose_state) = openpose_json
        .get_mut("pose_state")
        .and_then(|value| value.as_object_mut())
    {
        pose_state.insert("framing".to_owned(), posekit_framing_json(framing));
    }
    Ok(())
}

fn apply_posekit_marker_edits(
    openpose_json: &mut serde_json::Value,
    edits: &[PosekitMarkerEdit],
    layers: &PosekitMarkerLayers,
) -> AtelierResult<usize> {
    for edit in edits {
        let (field, expected_count) = edit.family.field_and_count();
        if edit.index >= expected_count {
            return Err(AtelierError::Validation(format!(
                "Posekit marker edit index {} is outside {} marker count {}",
                edit.index,
                edit.family.as_token(),
                expected_count
            )));
        }
        if !edit.family.layer_enabled(layers) {
            return Err(AtelierError::Validation(format!(
                "Posekit marker edit family {} is disabled by marker layers",
                edit.family.as_token()
            )));
        }
        let values = openpose_keypoint_array_mut(openpose_json, field)?;
        let offset = edit.index * 3;
        match edit.action {
            PosekitMarkerEditAction::Remove => {
                values[offset] = serde_json::json!(0.0);
                values[offset + 1] = serde_json::json!(0.0);
                values[offset + 2] = serde_json::json!(0.0);
            }
            PosekitMarkerEditAction::Set | PosekitMarkerEditAction::Add => {
                let x = edit.x.ok_or_else(|| {
                    AtelierError::Validation("Posekit marker edit x is required".into())
                })?;
                let y = edit.y.ok_or_else(|| {
                    AtelierError::Validation("Posekit marker edit y is required".into())
                })?;
                let confidence = edit.confidence.ok_or_else(|| {
                    AtelierError::Validation("Posekit marker edit confidence is required".into())
                })?;
                validate_posekit_marker_coordinate("x", x)?;
                validate_posekit_marker_coordinate("y", y)?;
                validate_posekit_marker_confidence(confidence)?;
                if matches!(edit.action, PosekitMarkerEditAction::Add)
                    && !is_zero_marker_slot(&values[offset..offset + 3])?
                {
                    return Err(AtelierError::Validation(
                        "Posekit marker add can only fill an empty zero-confidence slot".into(),
                    ));
                }
                values[offset] = serde_json::json!(round_posekit_coordinate(x));
                values[offset + 1] = serde_json::json!(round_posekit_coordinate(y));
                values[offset + 2] = serde_json::json!(round_posekit_confidence(confidence));
            }
        }
    }
    Ok(edits.len())
}

fn openpose_keypoint_array_mut<'a>(
    openpose_json: &'a mut serde_json::Value,
    field: &str,
) -> AtelierResult<&'a mut Vec<serde_json::Value>> {
    openpose_json
        .get_mut("people")
        .and_then(|people| people.as_array_mut())
        .and_then(|people| people.first_mut())
        .and_then(|person| person.get_mut(field))
        .and_then(|value| value.as_array_mut())
        .ok_or_else(|| {
            AtelierError::Validation(format!("Posekit OpenPose JSON missing array {field}"))
        })
}

fn value_as_f32(value: &serde_json::Value, field: &str) -> AtelierResult<f32> {
    let number = value.as_f64().ok_or_else(|| {
        AtelierError::Validation(format!(
            "pose keypoints field {field} contains a non-number"
        ))
    })?;
    if !number.is_finite() {
        return Err(AtelierError::Validation(format!(
            "pose keypoints field {field} contains a non-finite number"
        )));
    }
    Ok(number as f32)
}

fn is_zero_marker_slot(values: &[serde_json::Value]) -> AtelierResult<bool> {
    Ok(value_as_f32(&values[0], "marker")? == 0.0
        && value_as_f32(&values[1], "marker")? == 0.0
        && value_as_f32(&values[2], "marker")? == 0.0)
}

fn validate_posekit_marker_coordinate(field: &str, value: f32) -> AtelierResult<()> {
    if !value.is_finite() {
        return Err(AtelierError::Validation(format!(
            "Posekit marker edit {field} must be finite"
        )));
    }
    let max = if field == "x" {
        POSEKIT_OPENPOSE_EXPORT_WIDTH as f32
    } else {
        POSEKIT_OPENPOSE_EXPORT_HEIGHT as f32
    };
    if value < 0.0 || value > max {
        return Err(AtelierError::Validation(format!(
            "Posekit marker edit {field} must be inside the export canvas"
        )));
    }
    Ok(())
}

fn validate_posekit_marker_confidence(value: f32) -> AtelierResult<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(AtelierError::Validation(
            "Posekit marker edit confidence must be finite and in 0..=1".into(),
        ));
    }
    Ok(())
}

fn validate_posekit_export_keypoints(
    openpose_json: &serde_json::Value,
    layers: &PosekitMarkerLayers,
) -> AtelierResult<()> {
    let mut visible = 0usize;
    let body_visible = validate_posekit_export_keypoint_field(
        openpose_json,
        "pose_keypoints_2d",
        BODY_KEYPOINT_COUNT,
    )?;
    visible += body_visible;
    if layers.body && body_visible == 0 {
        return Err(AtelierError::Validation(
            "Posekit body export cannot be all-zero after marker edits and framing".into(),
        ));
    }
    visible += validate_posekit_export_keypoint_field(
        openpose_json,
        "face_keypoints_2d",
        FACE_KEYPOINT_COUNT,
    )?;
    visible += validate_posekit_export_keypoint_field(
        openpose_json,
        "hand_left_keypoints_2d",
        HAND_KEYPOINT_COUNT,
    )?;
    visible += validate_posekit_export_keypoint_field(
        openpose_json,
        "hand_right_keypoints_2d",
        HAND_KEYPOINT_COUNT,
    )?;
    if visible == 0 {
        return Err(AtelierError::Validation(
            "Posekit OpenPose export would be blank after marker edits and framing".into(),
        ));
    }
    Ok(())
}

fn validate_posekit_export_keypoint_field(
    openpose_json: &serde_json::Value,
    field: &str,
    expected_count: usize,
) -> AtelierResult<usize> {
    let points = openpose_points(openpose_json, field, expected_count)?;
    let mut visible = 0usize;
    for (x, y, confidence) in points {
        if !x.is_finite() || !y.is_finite() || !confidence.is_finite() {
            return Err(AtelierError::Validation(format!(
                "Posekit OpenPose field {field} contains non-finite values"
            )));
        }
        if !(0.0..=1.0).contains(&confidence) {
            return Err(AtelierError::Validation(format!(
                "Posekit OpenPose field {field} confidence must be in 0..=1"
            )));
        }
        if confidence <= 0.0 {
            continue;
        }
        if x < 0.0
            || y < 0.0
            || x > POSEKIT_OPENPOSE_EXPORT_WIDTH as f32
            || y > POSEKIT_OPENPOSE_EXPORT_HEIGHT as f32
        {
            return Err(AtelierError::Validation(format!(
                "Posekit OpenPose field {field} has a visible point outside the export canvas"
            )));
        }
        visible += 1;
    }
    Ok(visible)
}

fn round_posekit_coordinate(value: f32) -> f64 {
    ((value as f64) * 10.0).round() / 10.0
}

fn round_posekit_confidence(value: f32) -> f64 {
    ((value as f64) * 100.0).round() / 100.0
}

fn posekit_pose_center(yaw_deg: f32, pitch_deg: f32) -> (f32, f32) {
    (
        POSEKIT_OPENPOSE_EXPORT_WIDTH as f32 * 0.5 + yaw_deg / 180.0 * 72.0,
        POSEKIT_OPENPOSE_EXPORT_HEIGHT as f32 * 0.51 + pitch_deg / 45.0 * 42.0,
    )
}

fn posekit_body_keypoints(yaw_deg: f32, pitch_deg: f32, zoom: f32, visible: bool) -> Vec<f32> {
    if !visible {
        return zero_keypoints(BODY_KEYPOINT_COUNT);
    }
    let (center_x, center_y) = posekit_pose_center(yaw_deg, pitch_deg);
    let scale = zoom.clamp(0.4, 2.2);
    let yaw_bias = yaw_deg / 180.0;
    let shoulder = 86.0 * scale * (1.0 - yaw_bias.abs() * 0.28);
    let hip = 52.0 * scale * (1.0 - yaw_bias.abs() * 0.18);
    let points = [
        (center_x, center_y - 170.0 * scale, 0.95),
        (center_x, center_y - 102.0 * scale, 0.94),
        (center_x - shoulder, center_y - 92.0 * scale, 0.91),
        (
            center_x - shoulder - 54.0 * scale,
            center_y - 34.0 * scale,
            0.86,
        ),
        (
            center_x - shoulder - 70.0 * scale,
            center_y + 34.0 * scale,
            0.82,
        ),
        (center_x + shoulder, center_y - 92.0 * scale, 0.91),
        (
            center_x + shoulder + 54.0 * scale,
            center_y - 34.0 * scale,
            0.86,
        ),
        (
            center_x + shoulder + 70.0 * scale,
            center_y + 34.0 * scale,
            0.82,
        ),
        (center_x - hip, center_y + 46.0 * scale, 0.90),
        (
            center_x - hip - 22.0 * scale,
            center_y + 142.0 * scale,
            0.86,
        ),
        (
            center_x - hip - 18.0 * scale,
            center_y + 238.0 * scale,
            0.82,
        ),
        (center_x + hip, center_y + 46.0 * scale, 0.90),
        (
            center_x + hip + 22.0 * scale,
            center_y + 142.0 * scale,
            0.86,
        ),
        (
            center_x + hip + 18.0 * scale,
            center_y + 238.0 * scale,
            0.82,
        ),
        (
            center_x - 18.0 * scale - yaw_bias * 8.0,
            center_y - 180.0 * scale,
            0.80,
        ),
        (
            center_x + 18.0 * scale - yaw_bias * 8.0,
            center_y - 180.0 * scale,
            0.80,
        ),
        (
            center_x - 42.0 * scale - yaw_bias * 10.0,
            center_y - 164.0 * scale,
            0.76,
        ),
        (
            center_x + 42.0 * scale - yaw_bias * 10.0,
            center_y - 164.0 * scale,
            0.76,
        ),
    ];
    flatten_posekit_keypoints(&points)
}

fn posekit_face_keypoints(yaw_deg: f32, pitch_deg: f32, zoom: f32, visible: bool) -> Vec<f32> {
    if !visible {
        return zero_keypoints(FACE_KEYPOINT_COUNT);
    }
    let (center_x, center_y) = posekit_pose_center(yaw_deg, pitch_deg);
    let scale = zoom.clamp(0.4, 2.2);
    let yaw_bias = yaw_deg / 180.0;
    let mut points = Vec::with_capacity(FACE_KEYPOINT_COUNT);
    for index in 0..FACE_KEYPOINT_COUNT {
        let theta = index as f32 / FACE_KEYPOINT_COUNT as f32 * std::f32::consts::TAU;
        let x = center_x
            + theta.cos() * 34.0 * scale * (1.0 - yaw_bias.abs() * 0.32)
            + yaw_bias * 14.0 * scale;
        let y = center_y - 170.0 * scale + theta.sin() * 45.0 * scale;
        points.push((x, y, 0.78));
    }
    flatten_posekit_keypoints(&points)
}

fn posekit_hand_keypoints(
    yaw_deg: f32,
    pitch_deg: f32,
    zoom: f32,
    visible: bool,
    side: f32,
) -> Vec<f32> {
    if !visible {
        return zero_keypoints(HAND_KEYPOINT_COUNT);
    }
    let (center_x, center_y) = posekit_pose_center(yaw_deg, pitch_deg);
    let scale = zoom.clamp(0.4, 2.2);
    let wrist_x = center_x + side * 158.0 * scale;
    let wrist_y = center_y + 34.0 * scale;
    let mut points = Vec::with_capacity(HAND_KEYPOINT_COUNT);
    for index in 0..HAND_KEYPOINT_COUNT {
        let finger = (index / 4) as f32;
        let joint = (index % 4) as f32;
        points.push((
            wrist_x + side * (finger - 2.0) * 8.0 * scale,
            wrist_y - joint * 13.0 * scale - finger * 2.0 * scale,
            0.70,
        ));
    }
    flatten_posekit_keypoints(&points)
}

fn flatten_posekit_keypoints(points: &[(f32, f32, f32)]) -> Vec<f32> {
    let mut flattened = Vec::with_capacity(points.len() * 3);
    for (x, y, confidence) in points {
        flattened.push((x * 10.0).round() / 10.0);
        flattened.push((y * 10.0).round() / 10.0);
        flattened.push((confidence * 100.0).round() / 100.0);
    }
    flattened
}

fn zero_keypoints(count: usize) -> Vec<f32> {
    vec![0.0; count * 3]
}

fn render_posekit_openpose_png(openpose_json: &serde_json::Value) -> AtelierResult<Vec<u8>> {
    let mut image = RgbaImage::from_pixel(
        POSEKIT_OPENPOSE_EXPORT_WIDTH as u32,
        POSEKIT_OPENPOSE_EXPORT_HEIGHT as u32,
        Rgba([0, 0, 0, 255]),
    );
    let cyan = Rgba([70, 220, 255, 255]);
    let amber = Rgba([255, 190, 80, 255]);
    let green = Rgba([120, 255, 150, 255]);

    let body = openpose_points(openpose_json, "pose_keypoints_2d", BODY_KEYPOINT_COUNT)?;
    let face = openpose_points(openpose_json, "face_keypoints_2d", FACE_KEYPOINT_COUNT)?;
    let left_hand = openpose_points(openpose_json, "hand_left_keypoints_2d", HAND_KEYPOINT_COUNT)?;
    let right_hand = openpose_points(
        openpose_json,
        "hand_right_keypoints_2d",
        HAND_KEYPOINT_COUNT,
    )?;

    for (from, to) in [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 4),
        (1, 5),
        (5, 6),
        (6, 7),
        (1, 8),
        (8, 9),
        (9, 10),
        (1, 11),
        (11, 12),
        (12, 13),
        (0, 14),
        (14, 16),
        (0, 15),
        (15, 17),
    ] {
        draw_openpose_segment_if_visible(&mut image, body[from], body[to], cyan);
    }
    for point in &body {
        if visible_openpose_point(*point) {
            draw_disc(&mut image, point.0, point.1, 4.0, amber);
        }
    }

    for point in &face {
        if visible_openpose_point(*point) {
            draw_disc(&mut image, point.0, point.1, 3.0, amber);
        }
    }
    draw_hand_keypoints(&mut image, &left_hand, green);
    draw_hand_keypoints(&mut image, &right_hand, green);

    encode_posekit_png(&image)
}

fn openpose_points(
    openpose_json: &serde_json::Value,
    field: &str,
    expected_count: usize,
) -> AtelierResult<Vec<(f32, f32, f32)>> {
    let person = openpose_json
        .get("people")
        .and_then(|people| people.as_array())
        .and_then(|people| people.first())
        .ok_or_else(|| {
            AtelierError::Validation("Posekit OpenPose JSON must contain people[0]".into())
        })?;
    let values = person
        .get(field)
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            AtelierError::Validation(format!("Posekit OpenPose JSON missing array {field}"))
        })?;
    if values.len() != expected_count * 3 {
        return Err(AtelierError::Validation(format!(
            "Posekit OpenPose JSON field {field} must have {} values",
            expected_count * 3
        )));
    }
    values
        .chunks_exact(3)
        .map(|chunk| {
            let x = chunk[0].as_f64().ok_or_else(|| {
                AtelierError::Validation(format!(
                    "Posekit OpenPose JSON field {field} contains non-number x"
                ))
            })?;
            let y = chunk[1].as_f64().ok_or_else(|| {
                AtelierError::Validation(format!(
                    "Posekit OpenPose JSON field {field} contains non-number y"
                ))
            })?;
            let confidence = chunk[2].as_f64().ok_or_else(|| {
                AtelierError::Validation(format!(
                    "Posekit OpenPose JSON field {field} contains non-number confidence"
                ))
            })?;
            Ok((x as f32, y as f32, confidence as f32))
        })
        .collect()
}

fn draw_hand_keypoints(image: &mut RgbaImage, points: &[(f32, f32, f32)], color: Rgba<u8>) {
    for finger in 0..5 {
        let base = 1 + finger * 4;
        draw_openpose_segment_if_visible(image, points[0], points[base], color);
        for offset in 0..3 {
            draw_openpose_segment_if_visible(
                image,
                points[base + offset],
                points[base + offset + 1],
                color,
            );
        }
    }
    for point in points {
        if visible_openpose_point(*point) {
            draw_disc(image, point.0, point.1, 2.8, color);
        }
    }
}

fn draw_openpose_segment_if_visible(
    image: &mut RgbaImage,
    from: (f32, f32, f32),
    to: (f32, f32, f32),
    color: Rgba<u8>,
) {
    if visible_openpose_point(from) && visible_openpose_point(to) {
        draw_line(image, from.0, from.1, to.0, to.1, color);
    }
}

fn visible_openpose_point(point: (f32, f32, f32)) -> bool {
    point.2 > 0.0 && point.0 > 0.0 && point.1 > 0.0
}

fn encode_posekit_png(image: &RgbaImage) -> AtelierResult<Vec<u8>> {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|err| AtelierError::Validation(format!("Posekit PNG encode failed: {err}")))?;
    Ok(png)
}

fn draw_line(image: &mut RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgba<u8>) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        draw_disc(image, x0 + dx * t, y0 + dy * t, 2.0, color);
    }
}

fn draw_disc(image: &mut RgbaImage, cx: f32, cy: f32, radius: f32, color: Rgba<u8>) {
    let min_x = (cx - radius).floor() as i32;
    let max_x = (cx + radius).ceil() as i32;
    let min_y = (cy - radius).floor() as i32;
    let max_y = (cy + radius).ceil() as i32;
    let r2 = radius * radius;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x < 0
                || y < 0
                || x >= POSEKIT_OPENPOSE_EXPORT_WIDTH
                || y >= POSEKIT_OPENPOSE_EXPORT_HEIGHT
            {
                continue;
            }
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if dx * dx + dy * dy <= r2 {
                image.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

/// sha256 over several buffers in order (the export `content_hash` binds the
/// JSON and PNG payloads together). Single-buffer hashing goes through the
/// crate-level `storage::artifacts::sha256_hex`.
fn sha256_hex_joined(chunks: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hex::encode(hasher.finalize())
}

fn validate_pose_content_hash(content_hash: &str) -> AtelierResult<()> {
    let trimmed = content_hash.trim();
    if trimmed.is_empty()
        || trimmed != content_hash
        || content_hash.chars().any(char::is_whitespace)
    {
        return Err(AtelierError::Validation(
            "content_hash must be non-empty, unpadded, and no-space".into(),
        ));
    }
    Ok(())
}

fn validate_pose_sidecar_artifact_ref(artifact_ref: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref("artifact_ref", artifact_ref)?;
    if !artifact_ref.starts_with("artifact://.handshake/artifacts/") {
        return Err(AtelierError::Validation(
            "artifact_ref must be a native ArtifactStore payload handle".into(),
        ));
    }
    if artifact_ref.chars().any(char::is_whitespace) || artifact_ref.contains('\\') {
        return Err(AtelierError::Validation(
            "artifact_ref must be a portable no-space ArtifactStore handle".into(),
        ));
    }
    Ok(())
}

fn validate_pose_sidecar_manifest_ref(artifact_ref: &str, manifest_ref: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref("manifest_ref", manifest_ref)?;
    if !manifest_ref.starts_with("artifact://.handshake/artifacts/")
        || !manifest_ref.ends_with("/artifact.json")
    {
        return Err(AtelierError::Validation(
            "manifest_ref must be a native ArtifactStore artifact manifest handle".into(),
        ));
    }
    if manifest_ref.chars().any(char::is_whitespace) || manifest_ref.contains('\\') {
        return Err(AtelierError::Validation(
            "manifest_ref must be a portable no-space ArtifactStore handle".into(),
        ));
    }
    let expected = artifact_ref
        .strip_suffix("/payload")
        .map(|artifact_root| format!("{artifact_root}/artifact.json"))
        .ok_or_else(|| {
            AtelierError::Validation(
                "artifact_ref must be a native ArtifactStore payload handle ending in /payload"
                    .into(),
            )
        })?;
    if manifest_ref != expected {
        return Err(AtelierError::Validation(format!(
            "manifest_ref must point to the same ArtifactStore artifact manifest: expected {expected}"
        )));
    }
    Ok(())
}

fn validate_pose_source_ref_for_lookup(source_ref: &str) -> AtelierResult<String> {
    let trimmed = source_ref.trim();
    if trimmed.is_empty() || trimmed != source_ref {
        return Err(AtelierError::Validation(
            "source_ref lookup must be non-empty and unpadded".into(),
        ));
    }
    reject_legacy_runtime_ref("source_ref", source_ref)?;
    Ok(trimmed.to_string())
}

fn validate_pose_context_ref(field: &str, value: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref(field, value)?;
    if value.chars().any(char::is_whitespace) || value.contains('\\') {
        return Err(AtelierError::Validation(format!(
            "{field} must be a portable no-space ref"
        )));
    }
    Ok(())
}

fn validate_pose_context_request(new: &NewPoseContextState) -> AtelierResult<()> {
    validate_pose_context_ref("workspace_ref", &new.workspace_ref)?;
    reject_legacy_runtime_ref("requested_by", &new.requested_by)?;

    match new.kind {
        PoseContextKind::Blank => {
            if new.source_asset_id.is_some()
                || new.character_internal_id.is_some()
                || new.collection_id.is_some()
                || new.selected_rig_id.is_some()
            {
                return Err(AtelierError::Validation(
                    "blank pose context must not carry image, character, collection, or rig links"
                        .into(),
                ));
            }
        }
        PoseContextKind::SingleImage => {
            if new.source_asset_id.is_none()
                || new.character_internal_id.is_some()
                || new.collection_id.is_some()
            {
                return Err(AtelierError::Validation(
                    "single_image pose context requires source_asset_id and no character/collection links"
                        .into(),
                ));
            }
        }
        PoseContextKind::CharacterLinked => {
            if new.character_internal_id.is_none() || new.collection_id.is_some() {
                return Err(AtelierError::Validation(
                    "character_linked pose context requires character_internal_id and no collection link"
                        .into(),
                ));
            }
        }
        PoseContextKind::CollectionLinked => {
            if new.collection_id.is_none() {
                return Err(AtelierError::Validation(
                    "collection_linked pose context requires collection_id".into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_pose_workspace_rig_state_request(new: &NewPoseWorkspaceRigState) -> AtelierResult<()> {
    validate_pose_context_ref("workspace_ref", &new.workspace_ref)?;
    validate_pose_context_ref("session_ref", &new.session_ref)?;
    reject_legacy_runtime_ref("requested_by", &new.requested_by)?;
    if new.active && !new.open {
        return Err(AtelierError::Validation(
            "closed pose workspace rig state cannot be active".into(),
        ));
    }
    if new.sort_order < 0 {
        return Err(AtelierError::Validation(
            "pose workspace sort_order must be non-negative".into(),
        ));
    }
    if !new.panel_state.is_object() {
        return Err(AtelierError::Validation(
            "pose workspace panel_state must be a JSON object".into(),
        ));
    }
    Ok(())
}

fn validate_pose_workspace_panel_id(panel_id: &str) -> AtelierResult<()> {
    if panel_id.trim().is_empty() || panel_id.trim() != panel_id {
        return Err(AtelierError::Validation(
            "pose workspace panel_id must be non-empty and unpadded".into(),
        ));
    }
    if !panel_id
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        return Err(AtelierError::Validation(
            "pose workspace panel_id must be a stable lowercase token".into(),
        ));
    }
    Ok(())
}

fn validate_pose_workspace_route_target(new: &NewPoseWorkspaceRouteTarget) -> AtelierResult<()> {
    validate_pose_context_ref("workspace_ref", &new.workspace_ref)?;
    validate_pose_context_ref("session_ref", &new.session_ref)?;
    validate_pose_workspace_panel_id(&new.panel_id)?;
    reject_legacy_runtime_ref("requested_by", &new.requested_by)?;
    Ok(())
}

fn validate_pose_workspace_keyboard_action(
    request: &PoseWorkspaceKeyboardActionRequest,
) -> AtelierResult<()> {
    validate_pose_context_ref("workspace_ref", &request.workspace_ref)?;
    validate_pose_context_ref("session_ref", &request.session_ref)?;
    validate_pose_workspace_panel_id(&request.panel_id)?;
    reject_legacy_runtime_ref("requested_by", &request.requested_by)?;
    Ok(())
}

fn pose_workspace_route_ref(
    workspace_ref: &str,
    session_ref: &str,
    rig_id: Uuid,
    panel_id: &str,
) -> String {
    let workspace_key = event_ref_for_text(workspace_ref).replace(':', "-");
    let session_key = event_ref_for_text(session_ref).replace(':', "-");
    format!("pose-workspace-route://{workspace_key}/{session_key}/{rig_id}/{panel_id}")
}

fn validate_pose_sidecar(new: &NewPoseSidecar) -> AtelierResult<()> {
    validate_pose_sidecar_artifact_ref(&new.artifact_ref)?;
    validate_pose_sidecar_manifest_ref(&new.artifact_ref, &new.manifest_ref)?;
    validate_pose_content_hash(&new.content_hash)?;
    if new.byte_len <= 0 {
        return Err(AtelierError::Validation("byte_len must be positive".into()));
    }
    if new.width <= 0 || new.height <= 0 {
        return Err(AtelierError::Validation(
            "pose sidecar width/height must be positive".into(),
        ));
    }
    let mime = new.mime.trim();
    if mime != new.mime || mime != new.kind.expected_mime() {
        return Err(AtelierError::Validation(format!(
            "mime for {} must be {}",
            new.kind.as_token(),
            new.kind.expected_mime()
        )));
    }
    match new.status {
        PoseSidecarStatus::Rendered if new.error_message.is_some() => {
            return Err(AtelierError::Validation(
                "rendered pose sidecar error_message must be empty".into(),
            ));
        }
        PoseSidecarStatus::Failed => {
            let Some(error_message) = &new.error_message else {
                return Err(AtelierError::Validation(
                    "failed pose sidecar must include error_message".into(),
                ));
            };
            if error_message.trim().is_empty() || error_message.trim() != error_message {
                return Err(AtelierError::Validation(
                    "failed pose sidecar error_message must be non-empty and unpadded".into(),
                ));
            }
        }
        _ => {}
    }
    validate_pose_sidecar_artifact_payload(new)?;
    Ok(())
}

/// A sidecar row is only as trustworthy as the ArtifactStore payload it points
/// at: the manifest must exist, the payload must re-hash to the manifest, the
/// row's hash/size/mime must equal the manifest, and the payload must decode
/// as what the `kind` claims (OpenPose JSON with valid keypoints, or a PNG whose
/// dimensions match `width`/`height`). This is what lets the Posekit byte route
/// serve sidecar-bound bytes without re-deriving trust at read time.
fn validate_pose_sidecar_artifact_payload(new: &NewPoseSidecar) -> AtelierResult<()> {
    let (layer, artifact_id) = parse_pose_sidecar_artifact_handle(&new.artifact_ref)?;
    let workspace_root = resolve_workspace_root().map_err(|err| {
        AtelierError::Validation(format!("ArtifactStore root unavailable: {err}"))
    })?;
    let manifest = read_artifact_manifest(&workspace_root, layer, artifact_id).map_err(|err| {
        AtelierError::Validation(format!("ArtifactStore manifest validation failed: {err}"))
    })?;
    validate_artifact_content_hash(&workspace_root, layer, artifact_id).map_err(|err| {
        AtelierError::Validation(format!(
            "ArtifactStore content hash validation failed: {err}"
        ))
    })?;
    if manifest.content_hash != new.content_hash {
        return Err(AtelierError::Validation(
            "pose sidecar content_hash does not match ArtifactStore manifest".into(),
        ));
    }
    if manifest.size_bytes != new.byte_len as u64 {
        return Err(AtelierError::Validation(
            "pose sidecar byte_len does not match ArtifactStore manifest".into(),
        ));
    }
    if manifest.mime != new.mime {
        return Err(AtelierError::Validation(
            "pose sidecar mime does not match ArtifactStore manifest".into(),
        ));
    }
    let payload_path = artifact_root_dir(&workspace_root, layer, artifact_id).join("payload");
    let payload = fs::read(&payload_path).map_err(|err| {
        AtelierError::Validation(format!(
            "pose sidecar ArtifactStore payload could not be read: {err}"
        ))
    })?;
    match new.kind {
        PoseSidecarKind::OpenPoseJson => {
            let payload_json: serde_json::Value =
                serde_json::from_slice(&payload).map_err(|err| {
                    AtelierError::Validation(format!(
                        "openpose_json sidecar payload is not valid JSON: {err}"
                    ))
                })?;
            validate_keypoints(&payload_json)?;
        }
        PoseSidecarKind::OpenPosePng | PoseSidecarKind::ConditioningPng => {
            let image = image::load_from_memory(&payload).map_err(|err| {
                AtelierError::Validation(format!(
                    "pose PNG sidecar payload is not a decodable image: {err}"
                ))
            })?;
            if image.width() as i32 != new.width || image.height() as i32 != new.height {
                return Err(AtelierError::Validation(
                    "pose PNG sidecar dimensions do not match decoded payload".into(),
                ));
            }
        }
    }
    Ok(())
}

fn parse_pose_sidecar_artifact_handle(artifact_ref: &str) -> AtelierResult<(ArtifactLayer, Uuid)> {
    let path = artifact_ref
        .strip_prefix("artifact://.handshake/artifacts/")
        .and_then(|value| value.strip_suffix("/payload"))
        .ok_or_else(|| {
            AtelierError::Validation(
                "artifact_ref must be artifact://.handshake/artifacts/<layer>/<uuid>/payload"
                    .into(),
            )
        })?;
    let mut parts = path.split('/');
    let layer = match parts.next() {
        Some("L1") => ArtifactLayer::L1,
        Some("L2") => ArtifactLayer::L2,
        Some("L3") => ArtifactLayer::L3,
        Some("L4") => ArtifactLayer::L4,
        Some(other) => {
            return Err(AtelierError::Validation(format!(
                "unsupported ArtifactStore layer in pose sidecar ref: {other}"
            )));
        }
        None => {
            return Err(AtelierError::Validation(
                "missing ArtifactStore layer in pose sidecar ref".into(),
            ));
        }
    };
    let artifact_id = parts
        .next()
        .ok_or_else(|| {
            AtelierError::Validation("missing ArtifactStore artifact id in pose sidecar ref".into())
        })
        .and_then(|value| {
            Uuid::parse_str(value).map_err(|err| {
                AtelierError::Validation(format!("invalid pose sidecar artifact id: {err}"))
            })
        })?;
    if parts.next().is_some() {
        return Err(AtelierError::Validation(
            "pose sidecar artifact ref has unexpected path suffix".into(),
        ));
    }
    Ok((layer, artifact_id))
}

fn validate_pose_calibration(new: &NewPoseCalibration) -> AtelierResult<()> {
    match new.state {
        CalibrationState::Unresolved => {
            let Some(reason) = &new.block_reason else {
                return Err(AtelierError::Validation(
                    "unresolved calibration must record a block_reason".into(),
                ));
            };
            if reason.trim().is_empty() || reason.trim() != reason {
                return Err(AtelierError::Validation(
                    "unresolved calibration block_reason must be non-empty and unpadded".into(),
                ));
            }
        }
        CalibrationState::Resolved => {
            if new.block_reason.is_some() {
                return Err(AtelierError::Validation(
                    "resolved calibration must not carry a block_reason".into(),
                ));
            }
        }
    }
    if let Some(head_pose_ref) = &new.head_pose_ref {
        reject_legacy_runtime_ref("head_pose_ref", head_pose_ref)?;
    }
    for (field, color) in [
        ("marker_colors.body", &new.marker_colors.body),
        ("marker_colors.face", &new.marker_colors.face),
        ("marker_colors.left_hand", &new.marker_colors.left_hand),
        ("marker_colors.right_hand", &new.marker_colors.right_hand),
    ] {
        reject_legacy_runtime_ref(field, color)?;
    }
    let mut seen_left = false;
    let mut seen_right = false;
    for hand_row in &new.hand_rows {
        if hand_row.marker_count != HAND_KEYPOINT_COUNT as i32 {
            return Err(AtelierError::Validation(format!(
                "calibration hand row marker_count must be {HAND_KEYPOINT_COUNT}"
            )));
        }
        match hand_row.hand {
            CalibrationHandKind::Left if seen_left => {
                return Err(AtelierError::Validation(
                    "duplicate left hand calibration row".into(),
                ));
            }
            CalibrationHandKind::Right if seen_right => {
                return Err(AtelierError::Validation(
                    "duplicate right hand calibration row".into(),
                ));
            }
            CalibrationHandKind::Left => seen_left = true,
            CalibrationHandKind::Right => seen_right = true,
        }
    }
    for history_ref in &new.history_refs {
        reject_legacy_runtime_ref("history_ref", history_ref)?;
    }
    Ok(())
}

fn to_json_value<T: Serialize>(label: &str, value: &T) -> AtelierResult<serde_json::Value> {
    serde_json::to_value(value)
        .map_err(|err| AtelierError::Validation(format!("{label} serialization failed: {err}")))
}

fn context_state_from_row(row: &PoseRow) -> AtelierResult<PoseContextState> {
    let kind: String = row.get("kind");
    Ok(PoseContextState {
        context_id: row.get("context_id"),
        state_seq: row.get("state_seq"),
        workspace_ref: row.get("workspace_ref"),
        kind: PoseContextKind::from_token(&kind)?,
        source_asset_id: row.get("source_asset_id"),
        character_internal_id: row.get("character_internal_id"),
        collection_id: row.get("collection_id"),
        selected_rig_id: row.get("selected_rig_id"),
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn workspace_rig_state_from_row(row: &PoseRow) -> PoseWorkspaceRigState {
    PoseWorkspaceRigState {
        workspace_ref: row.get("workspace_ref"),
        session_ref: row.get("session_ref"),
        rig_id: row.get("rig_id"),
        character_internal_id: row.get("character_internal_id"),
        source_asset_id: row.get("source_asset_id"),
        source_ref: row.get("source_ref"),
        open: row.get("open"),
        sort_order: row.get("sort_order"),
        active: row.get("active"),
        dirty_calibration: row.get("dirty_calibration"),
        panel_state: row.get("panel_state"),
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn rig_from_row(row: &PoseRow) -> AtelierResult<PoseRig> {
    let detector_status: String = row.get("detector_status");
    Ok(PoseRig {
        rig_id: row.get("rig_id"),
        character_internal_id: row.get("character_internal_id"),
        source_asset_id: row.get("source_asset_id"),
        source_ref: row.get("source_ref"),
        content_hash: row.get("content_hash"),
        canvas: CanvasSize {
            width: row.get("canvas_width"),
            height: row.get("canvas_height"),
        },
        detector_provider: row.get("detector_provider"),
        detector_model: row.get("detector_model"),
        detector_model_version: row.get("detector_model_version"),
        source_asset_version_ref: row.get("source_asset_version_ref"),
        source_asset_path_ref: row.get("source_asset_path_ref"),
        confidence_available: row.get("confidence_available"),
        detector_status: DetectorStatus::from_token(&detector_status)?,
        error_reason: row.get("error_reason"),
        keypoints_json: row.get("keypoints_json"),
        sidecar_ref: row.get("sidecar_ref"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn sidecar_from_row(row: &PoseRow) -> AtelierResult<PoseSidecar> {
    let kind: String = row.get("kind");
    let status: String = row.get("status");
    Ok(PoseSidecar {
        sidecar_id: row.get("sidecar_id"),
        rig_id: row.get("rig_id"),
        source_asset_id: row.get("source_asset_id"),
        source_ref: row.get("source_ref"),
        kind: PoseSidecarKind::from_token(&kind)?,
        role: row.get("role"),
        artifact_ref: row.get("artifact_ref"),
        manifest_ref: row.get("manifest_ref"),
        content_hash: row.get("content_hash"),
        byte_len: row.get("byte_len"),
        mime: row.get("mime"),
        width: row.get("width"),
        height: row.get("height"),
        status: PoseSidecarStatus::from_token(&status)?,
        error_message: row.get("error_message"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn source_image_strip_item_from_row(row: &PoseRow) -> PoseSourceImageStripItem {
    let rig_id: Uuid = row.get("rig_id");
    let source_asset_id: Option<Uuid> = row.get("source_asset_id");
    PoseSourceImageStripItem {
        rig_id,
        character_internal_id: row.get("character_internal_id"),
        source_asset_id,
        source_ref: row.get("source_ref"),
        artifact_ref: row.get("source_artifact_ref"),
        content_hash: row.get("source_content_hash"),
        mime: row.get("source_mime"),
        byte_len: row.get("source_byte_len"),
        diagnostics_visible: true,
        gallery_visible: source_asset_id.is_some(),
        jump_target: format!("atelier://pose-rig/{rig_id}/source"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn openpose_sidecar_strip_item_from_sidecar(sidecar: PoseSidecar) -> PoseOpenPoseSidecarStripItem {
    PoseOpenPoseSidecarStripItem {
        sidecar_id: sidecar.sidecar_id,
        rig_id: sidecar.rig_id,
        source_asset_id: sidecar.source_asset_id,
        source_ref: sidecar.source_ref,
        kind: sidecar.kind,
        role: sidecar.role,
        artifact_ref: sidecar.artifact_ref,
        manifest_ref: sidecar.manifest_ref,
        content_hash: sidecar.content_hash,
        byte_len: sidecar.byte_len,
        mime: sidecar.mime,
        width: sidecar.width,
        height: sidecar.height,
        status: sidecar.status,
        error_message: sidecar.error_message,
        diagnostics_visible: true,
        gallery_visible: false,
        hidden_reason: "pose_sidecar".to_string(),
        jump_target: format!("atelier://pose-rig/{}/sidecars", sidecar.rig_id),
        created_at_utc: sidecar.created_at_utc,
    }
}

fn head_pose_from_row(row: &PoseRow) -> HeadPose {
    HeadPose {
        rig_id: row.get("rig_id"),
        yaw_deg: row.get("yaw_deg"),
        pitch_deg: row.get("pitch_deg"),
        roll_deg: row.get("roll_deg"),
        quaternion: [
            row.get("quat_x"),
            row.get("quat_y"),
            row.get("quat_z"),
            row.get("quat_w"),
        ],
        created_at_utc: row.get("created_at_utc"),
    }
}

fn calibration_from_row(row: &PoseRow) -> AtelierResult<Calibration> {
    let state: String = row.get("state");
    let marker_visibility_json: serde_json::Value = row.get("marker_visibility");
    let marker_colors_json: serde_json::Value = row.get("marker_colors");
    let hand_rows_json: serde_json::Value = row.get("hand_rows");
    let history_refs_json: serde_json::Value = row.get("history_refs");
    Ok(Calibration {
        rig_id: row.get("rig_id"),
        state: CalibrationState::from_token(&state)?,
        block_reason: row.get("block_reason"),
        head_pose_ref: row.get("head_pose_ref"),
        marker_visibility: serde_json::from_value(marker_visibility_json).map_err(|err| {
            AtelierError::Validation(format!("invalid marker_visibility JSON: {err}"))
        })?,
        marker_colors: serde_json::from_value(marker_colors_json).map_err(|err| {
            AtelierError::Validation(format!("invalid marker_colors JSON: {err}"))
        })?,
        hand_rows: serde_json::from_value(hand_rows_json)
            .map_err(|err| AtelierError::Validation(format!("invalid hand_rows JSON: {err}")))?,
        history_refs: serde_json::from_value(history_refs_json)
            .map_err(|err| AtelierError::Validation(format!("invalid history_refs JSON: {err}")))?,
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn identity_profile_from_row(row: &PoseRow) -> AtelierResult<IdentityProfile> {
    let kind: String = row.get("kind");
    Ok(IdentityProfile {
        profile_id: row.get("profile_id"),
        character_internal_id: row.get("character_internal_id"),
        seq: row.get("seq"),
        version: row.get("version"),
        kind: IdentityProfileKind::from_token(&kind)?,
        name: row.get("name"),
        description: row.get("description"),
        reference_asset_id: row.get("reference_asset_id"),
        reference_ref: row.get("reference_ref"),
        source_ref: row.get("source_ref"),
        crop_ref: row.get("crop_ref"),
        artifact_ref: row.get("artifact_ref"),
        provenance: row.get("provenance"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn identity_crop_artifact_from_row(row: &PoseRow) -> AtelierResult<IdentityCropArtifact> {
    let crop_box_json: serde_json::Value = row.get("crop_box");
    let landmarks_json: serde_json::Value = row.get("landmarks");
    Ok(IdentityCropArtifact {
        crop_id: row.get("crop_id"),
        profile_id: row.get("profile_id"),
        profile_version: row.get("profile_version"),
        character_internal_id: row.get("character_internal_id"),
        source_ref: row.get("source_ref"),
        crop_box: serde_json::from_value(crop_box_json)
            .map_err(|err| AtelierError::Validation(format!("invalid crop_box JSON: {err}")))?,
        landmarks: serde_json::from_value(landmarks_json)
            .map_err(|err| AtelierError::Validation(format!("invalid landmarks JSON: {err}")))?,
        artifact_ref: row.get("artifact_ref"),
        manifest_ref: row.get("manifest_ref"),
        content_hash: row.get("content_hash"),
        byte_len: row.get("byte_len"),
        mime: row.get("mime"),
        width: row.get("width"),
        height: row.get("height"),
        manifest: row.get("manifest"),
        created_by: row.get("created_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

#[derive(SurrealValue)]
struct PoseRecordBinding {
    record_id: RecordId,
}

#[derive(Clone, SurrealValue)]
struct PoseRigWriteBindings {
    record_id: RecordId,
    character_ref: RecordId,
    source_asset_ref: Option<RecordId>,
    source_ref: String,
    content_hash: String,
    canvas_width: i32,
    canvas_height: i32,
    detector_provider: String,
    detector_model: String,
    detector_model_version: String,
    source_asset_version_ref: Option<String>,
    source_asset_path_ref: Option<String>,
    confidence_available: bool,
    detector_status: String,
    error_reason: Option<String>,
    keypoints_json: serde_json::Value,
    sidecar_ref: Option<String>,
}

#[derive(SurrealValue)]
struct PoseRigIdBinding {
    rig_ref: RecordId,
}

#[derive(SurrealValue)]
struct PoseRigIdentityBinding {
    character_ref: RecordId,
    source_ref: String,
    content_hash: String,
}

#[derive(SurrealValue)]
struct PoseRigListBindings {
    character_ref: RecordId,
    limit: i64,
}

#[derive(Clone, SurrealValue)]
struct PoseContextWriteBindings {
    record_id: RecordId,
    context_id: SurrealUuid,
    workspace_ref: String,
    kind: String,
    source_asset_ref: Option<RecordId>,
    character_ref: Option<RecordId>,
    collection_ref: Option<RecordId>,
    selected_rig_ref: Option<RecordId>,
    requested_by: String,
}

#[derive(SurrealValue)]
struct PoseWorkspaceBinding {
    workspace_ref: String,
}

#[derive(Clone, SurrealValue)]
struct PoseWorkspaceRigWriteBindings {
    record_id: RecordId,
    workspace_ref: String,
    session_ref: String,
    rig_ref: RecordId,
    open: bool,
    sort_order: i32,
    active: bool,
    dirty_calibration: bool,
    panel_state: serde_json::Value,
    requested_by: String,
}

#[derive(SurrealValue)]
struct PoseWorkspaceSessionBinding {
    workspace_ref: String,
    session_ref: String,
}

#[derive(Clone, SurrealValue)]
struct PoseSidecarWriteBindings {
    record_id: RecordId,
    rig_ref: RecordId,
    source_asset_ref: Option<RecordId>,
    source_ref: String,
    kind: String,
    role: String,
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
    byte_len: i64,
    mime: String,
    width: i32,
    height: i32,
    status: String,
    error_message: Option<String>,
}

#[derive(SurrealValue)]
struct PoseSidecarSourceBinding {
    source_ref: String,
    rig_ref: Option<RecordId>,
}

#[derive(SurrealValue)]
struct PoseSidecarIdentityBinding {
    rig_ref: RecordId,
    kind: String,
}

#[derive(SurrealValue)]
struct PoseSidecarArtifactRefBinding {
    artifact_ref: String,
}

#[derive(SurrealValue)]
struct PoseSourceStripBindings {
    character_ref: RecordId,
    limit: i64,
}

macro_rules! rig_columns {
    () => {
        "rig_id, record::id(character_internal_id) AS character_internal_id, \
         IF source_asset_id = NONE { NONE } ELSE { record::id(source_asset_id) } AS source_asset_id, \
         source_ref, content_hash, canvas_width, canvas_height, detector_provider, \
         detector_model, detector_model_version, source_asset_version_ref, \
         source_asset_path_ref, confidence_available, detector_status, error_reason, \
         keypoints_json, sidecar_ref, created_at_utc"
    };
}

macro_rules! context_state_columns {
    () => {
        "context_id, state_seq, workspace_ref, kind, \
         IF source_asset_id = NONE { NONE } ELSE { record::id(source_asset_id) } AS source_asset_id, \
         IF character_internal_id = NONE { NONE } ELSE { record::id(character_internal_id) } AS character_internal_id, \
         IF collection_id = NONE { NONE } ELSE { record::id(collection_id) } AS collection_id, \
         IF selected_rig_id = NONE { NONE } ELSE { record::id(selected_rig_id) } AS selected_rig_id, \
         requested_by, created_at_utc"
    };
}

macro_rules! workspace_rig_state_columns {
    () => {
        "workspace_ref, session_ref, record::id(rig_id) AS rig_id, \
         record::id(rig_id.character_internal_id) AS character_internal_id, \
         IF rig_id.source_asset_id = NONE { NONE } ELSE { record::id(rig_id.source_asset_id) } AS source_asset_id, \
         rig_id.source_ref AS source_ref, open, sort_order, active, dirty_calibration, \
         panel_state, requested_by, created_at_utc, updated_at_utc"
    };
}

macro_rules! sidecar_columns {
    () => {
        "sidecar_id, record::id(rig_id) AS rig_id, \
         IF source_asset_id = NONE { NONE } ELSE { record::id(source_asset_id) } AS source_asset_id, \
         source_ref, kind, role, artifact_ref, manifest_ref, content_hash, byte_len, \
         mime, width, height, status, error_message, created_at_utc"
    };
}

const POSE_RECORD_EXISTS_STATEMENT: &str = "RETURN record::exists($record_id);";

const WRITE_POSE_RIG_STATEMENT: &str = concat!(
    "RETURN { IF !record::exists($domain.character_ref) { RETURN NONE; }; ",
    "LET $existing = (SELECT VALUE id FROM atelier_pose_rig \
       WHERE character_internal_id = $domain.character_ref \
         AND source_ref = $domain.source_ref AND content_hash = $domain.content_hash LIMIT 1)[0]; ",
    "LET $row = IF $existing = NONE { \
       (CREATE $domain.record_id CONTENT { rig_id: record::id($domain.record_id), \
         character_internal_id: $domain.character_ref, source_asset_id: $domain.source_asset_ref, \
         source_ref: $domain.source_ref, content_hash: $domain.content_hash, \
         canvas_width: $domain.canvas_width, canvas_height: $domain.canvas_height, \
         detector_provider: $domain.detector_provider, detector_model: $domain.detector_model, \
         detector_model_version: $domain.detector_model_version, \
         source_asset_version_ref: $domain.source_asset_version_ref, \
         source_asset_path_ref: $domain.source_asset_path_ref, \
         confidence_available: $domain.confidence_available, \
         detector_status: $domain.detector_status, error_reason: $domain.error_reason, \
         keypoints_json: $domain.keypoints_json, sidecar_ref: $domain.sidecar_ref } RETURN AFTER)[0] \
       } ELSE { (SELECT ",
    rig_columns!(),
    " FROM ONLY $existing) }; ",
    atelier_event_sql!(),
    " RETURN IF $existing = NONE { (SELECT ",
    rig_columns!(),
    " FROM ONLY $domain.record_id) } ELSE { $row }; };"
);

const GET_POSE_RIG_STATEMENT: &str = concat!("SELECT ", rig_columns!(), " FROM $rig_ref LIMIT 1;");

const FIND_POSE_RIG_ID_STATEMENT: &str =
    "SELECT VALUE rig_id FROM atelier_pose_rig WHERE character_internal_id = $character_ref \
     AND source_ref = $source_ref AND content_hash = $content_hash LIMIT 1;";

const LIST_POSE_RIGS_STATEMENT: &str = concat!(
    "SELECT ",
    rig_columns!(),
    " FROM atelier_pose_rig WHERE character_internal_id = $character_ref \
     ORDER BY created_at_utc DESC LIMIT $limit;"
);

const WRITE_POSE_CONTEXT_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " RETURN (CREATE $domain.record_id CONTENT { context_id: $domain.context_id, \
       workspace_ref: $domain.workspace_ref, kind: $domain.kind, \
       source_asset_id: $domain.source_asset_ref, character_internal_id: $domain.character_ref, \
       collection_id: $domain.collection_ref, selected_rig_id: $domain.selected_rig_ref, \
       requested_by: $domain.requested_by } RETURN ",
    context_state_columns!(),
    ")[0]; };"
);

const CURRENT_POSE_CONTEXT_STATEMENT: &str = concat!(
    "SELECT ",
    context_state_columns!(),
    " FROM atelier_pose_context_state WHERE workspace_ref = $workspace_ref \
     ORDER BY state_seq DESC LIMIT 1;"
);

const LIST_POSE_CONTEXT_STATEMENT: &str = concat!(
    "SELECT ",
    context_state_columns!(),
    " FROM atelier_pose_context_state WHERE workspace_ref = $workspace_ref \
     ORDER BY state_seq ASC;"
);

const WRITE_POSE_WORKSPACE_RIG_STATEMENT: &str = concat!(
    "RETURN { IF !record::exists($domain.rig_ref) { RETURN NONE; }; ",
    "LET $duplicate = IF $domain.open { (SELECT VALUE id FROM atelier_pose_workspace_rig_state \
       WHERE workspace_ref = $domain.workspace_ref AND session_ref = $domain.session_ref \
         AND open = true AND rig_id != $domain.rig_ref AND sort_order = $domain.sort_order LIMIT 1)[0] \
       } ELSE { NONE }; ",
    "IF $duplicate != NONE { RETURN NONE; }; ",
    "LET $existing = (SELECT VALUE id FROM atelier_pose_workspace_rig_state \
       WHERE workspace_ref = $domain.workspace_ref AND session_ref = $domain.session_ref \
         AND rig_id = $domain.rig_ref LIMIT 1)[0]; ",
    "LET $rid = IF $existing = NONE { $domain.record_id } ELSE { $existing }; ",
    "IF $domain.active { UPDATE atelier_pose_workspace_rig_state SET active = false, \
       requested_by = $domain.requested_by, updated_at_utc = time::now() \
       WHERE workspace_ref = $domain.workspace_ref AND session_ref = $domain.session_ref \
         AND open = true AND active = true AND rig_id != $domain.rig_ref; }; ",
    atelier_event_sql!(),
    " UPSERT $rid MERGE { workspace_ref: $domain.workspace_ref, session_ref: $domain.session_ref, \
       rig_id: $domain.rig_ref, open: $domain.open, sort_order: $domain.sort_order, \
       active: $domain.active, dirty_calibration: $domain.dirty_calibration, \
       panel_state: $domain.panel_state, requested_by: $domain.requested_by, \
       updated_at_utc: time::now() }; ",
    "RETURN (SELECT ",
    workspace_rig_state_columns!(),
    " FROM ONLY $rid); };"
);

const LIST_POSE_WORKSPACE_RIG_STATEMENT: &str = concat!(
    "SELECT ",
    workspace_rig_state_columns!(),
    " FROM atelier_pose_workspace_rig_state \
     WHERE workspace_ref = $workspace_ref AND session_ref = $session_ref AND open = true \
     ORDER BY sort_order ASC, rig_id ASC;"
);

const WRITE_POSE_SIDECAR_STATEMENT: &str = concat!(
    "RETURN { LET $existing = (SELECT VALUE id FROM atelier_pose_sidecar \
       WHERE rig_id = $domain.rig_ref AND kind = $domain.kind LIMIT 1)[0]; ",
    "LET $rid = IF $existing = NONE { $domain.record_id } ELSE { $existing }; ",
    atelier_event_sql!(),
    " UPSERT $rid MERGE { sidecar_id: record::id($rid), rig_id: $domain.rig_ref, \
       source_asset_id: $domain.source_asset_ref, source_ref: $domain.source_ref, \
       kind: $domain.kind, role: $domain.role, artifact_ref: $domain.artifact_ref, \
       manifest_ref: $domain.manifest_ref, content_hash: $domain.content_hash, \
       byte_len: $domain.byte_len, mime: $domain.mime, width: $domain.width, \
       height: $domain.height, status: $domain.status, error_message: $domain.error_message, \
       created_at_utc: time::now() }; ",
    "RETURN (SELECT ",
    sidecar_columns!(),
    " FROM ONLY $rid); };"
);

const LIST_POSE_SIDECARS_STATEMENT: &str = concat!(
    "SELECT ",
    sidecar_columns!(),
    " FROM atelier_pose_sidecar WHERE rig_id = $rig_ref \
     ORDER BY created_at_utc ASC, sidecar_id ASC;"
);

const FIND_POSE_SIDECAR_ID_STATEMENT: &str = "SELECT VALUE sidecar_id FROM atelier_pose_sidecar \
     WHERE rig_id = $rig_ref AND kind = $kind LIMIT 1;";

/// Reverse lookup for the Posekit byte route: the ONE sidecar row bound to an
/// ArtifactStore payload ref. `artifact_ref` is not unique in the schema, so
/// the newest row wins deterministically if a ref were ever re-bound.
const GET_POSE_SIDECAR_BY_ARTIFACT_REF_STATEMENT: &str = concat!(
    "SELECT ",
    sidecar_columns!(),
    " FROM atelier_pose_sidecar WHERE artifact_ref = $artifact_ref \
     ORDER BY created_at_utc DESC, sidecar_id DESC LIMIT 1;"
);

const LIST_POSE_SIDECARS_FOR_SOURCE_STATEMENT: &str = concat!(
    "SELECT ",
    sidecar_columns!(),
    " FROM atelier_pose_sidecar WHERE source_ref = $source_ref \
       AND ($rig_ref = NONE OR rig_id = $rig_ref) \
     ORDER BY created_at_utc ASC, sidecar_id ASC;"
);

const LIST_POSE_SOURCE_STRIP_STATEMENT: &str =
    "SELECT rig_id, record::id(character_internal_id) AS character_internal_id, \
            IF source_asset_id = NONE { NONE } ELSE { record::id(source_asset_id) } AS source_asset_id, \
            source_ref, source_asset_id.artifact_ref AS source_artifact_ref, \
            source_asset_id.content_hash AS source_content_hash, \
            source_asset_id.mime AS source_mime, source_asset_id.byte_len AS source_byte_len, \
            created_at_utc \
     FROM atelier_pose_rig WHERE character_internal_id = $character_ref \
     ORDER BY created_at_utc DESC, rig_id ASC LIMIT $limit;";

const LIST_OPENPOSE_SIDECARS_STATEMENT: &str = concat!(
    "SELECT ",
    sidecar_columns!(),
    " FROM atelier_pose_sidecar WHERE rig_id = $rig_ref \
       AND kind IN ['openpose_json', 'openpose_png'] \
     ORDER BY kind ASC, created_at_utc ASC, sidecar_id ASC;"
);

async fn pose_record_exists(store: &AtelierStore, record_id: RecordId) -> AtelierResult<bool> {
    let binding = PoseRecordBinding { record_id };
    let exists: Option<bool> = store
        .with_data(move |ctx| {
            Box::pin(async move { ctx.query_first(POSE_RECORD_EXISTS_STATEMENT, binding).await })
        })
        .await?;
    Ok(exists.unwrap_or(false))
}

fn pose_sidecar_kind_order(kind: PoseSidecarKind) -> u8 {
    match kind {
        PoseSidecarKind::OpenPoseJson => 0,
        PoseSidecarKind::OpenPosePng => 1,
        PoseSidecarKind::ConditioningPng => 2,
    }
}

#[derive(Clone, SurrealValue)]
struct PoseHeadPoseBindings {
    record_id: RecordId,
    rig_ref: RecordId,
    yaw_deg: f64,
    pitch_deg: f64,
    roll_deg: f64,
    quat_x: f64,
    quat_y: f64,
    quat_z: f64,
    quat_w: f64,
}

#[derive(Clone, SurrealValue)]
struct PoseCalibrationBindings {
    record_id: RecordId,
    rig_ref: RecordId,
    state: String,
    block_reason: Option<String>,
    head_pose_ref: Option<String>,
    marker_visibility: serde_json::Value,
    marker_colors: serde_json::Value,
    hand_rows: serde_json::Value,
    history_refs: serde_json::Value,
}

#[derive(Clone, SurrealValue)]
struct IdentityProfileAppendBindings {
    record_id: RecordId,
    profile_id: SurrealUuid,
    character_ref: RecordId,
    kind: String,
    name: String,
    description: String,
    reference_asset_ref: Option<RecordId>,
    reference_ref: String,
    source_ref: Option<String>,
    crop_ref: Option<String>,
    artifact_ref: Option<String>,
    provenance: String,
}

#[derive(Clone, SurrealValue)]
struct IdentityProfileUpdateBindings {
    profile_ref: RecordId,
    name: String,
    description: String,
    source_ref: Option<String>,
    crop_ref: Option<String>,
    artifact_ref: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct IdentityProfileDeleteBindings {
    profile_ref: RecordId,
}

#[derive(SurrealValue)]
struct IdentityProfileListBindings {
    character_ref: RecordId,
    kind: Option<String>,
}

#[derive(SurrealValue)]
struct IdentityProfileKindBinding {
    character_ref: RecordId,
    kind: String,
}

#[derive(Clone, SurrealValue)]
struct IdentityCropBindings {
    record_id: RecordId,
    crop_id: SurrealUuid,
    profile_ref: RecordId,
    profile_version: i64,
    character_ref: RecordId,
    source_ref: String,
    crop_box: serde_json::Value,
    landmarks: serde_json::Value,
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
    byte_len: i64,
    mime: String,
    width: i32,
    height: i32,
    manifest: serde_json::Value,
    created_by: String,
}

#[derive(SurrealValue)]
struct IdentityCropIdentityBinding {
    profile_ref: RecordId,
    profile_version: i64,
    content_hash: String,
}

macro_rules! head_pose_columns {
    () => {
        "record::id(rig_id) AS rig_id, yaw_deg, pitch_deg, roll_deg, \
         quat_x, quat_y, quat_z, quat_w, created_at_utc"
    };
}

macro_rules! calibration_columns {
    () => {
        "record::id(rig_id) AS rig_id, state, block_reason, head_pose_ref, \
         marker_visibility, marker_colors, hand_rows, history_refs, \
         created_at_utc, updated_at_utc"
    };
}

macro_rules! profile_columns {
    () => {
        "profile_id, record::id(character_internal_id) AS character_internal_id, \
         seq, version, kind, name, description, \
         IF reference_asset_id = NONE { NONE } ELSE { record::id(reference_asset_id) } AS reference_asset_id, \
         reference_ref, source_ref, crop_ref, artifact_ref, provenance, \
         created_at_utc, updated_at_utc"
    };
}

macro_rules! identity_crop_columns {
    () => {
        "crop_id, record::id(profile_id) AS profile_id, profile_version, \
         record::id(character_internal_id) AS character_internal_id, \
         source_ref, crop_box, landmarks, artifact_ref, manifest_ref, content_hash, \
         byte_len, mime, width, height, manifest, created_by, created_at_utc"
    };
}

const WRITE_HEAD_POSE_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " UPSERT $domain.record_id MERGE { rig_id: $domain.rig_ref, yaw_deg: $domain.yaw_deg, \
       pitch_deg: $domain.pitch_deg, roll_deg: $domain.roll_deg, quat_x: $domain.quat_x, \
       quat_y: $domain.quat_y, quat_z: $domain.quat_z, quat_w: $domain.quat_w, \
       created_at_utc: time::now() }; RETURN (SELECT ",
    head_pose_columns!(),
    " FROM ONLY $domain.record_id); };"
);

const GET_HEAD_POSE_STATEMENT: &str =
    concat!("SELECT ", head_pose_columns!(), " FROM $rig_ref LIMIT 1;");

const WRITE_CALIBRATION_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " UPSERT $domain.record_id MERGE { rig_id: $domain.rig_ref, state: $domain.state, \
       block_reason: $domain.block_reason, head_pose_ref: $domain.head_pose_ref, \
       marker_visibility: $domain.marker_visibility, marker_colors: $domain.marker_colors, \
       hand_rows: $domain.hand_rows, history_refs: $domain.history_refs, \
       updated_at_utc: time::now() }; RETURN (SELECT ",
    calibration_columns!(),
    " FROM ONLY $domain.record_id); };"
);

const GET_CALIBRATION_STATEMENT: &str =
    concat!("SELECT ", calibration_columns!(), " FROM $rig_ref LIMIT 1;");

const APPEND_IDENTITY_PROFILE_STATEMENT: &str = concat!(
    "RETURN { IF !record::exists($domain.character_ref) { RETURN NONE; }; ",
    "LET $next_seq = ((SELECT VALUE seq FROM atelier_identity_profile \
       WHERE character_internal_id = $domain.character_ref ORDER BY seq DESC LIMIT 1)[0] ?? 0) + 1; ",
    atelier_event_sql!(),
    " RETURN (CREATE $domain.record_id CONTENT { profile_id: $domain.profile_id, \
       character_internal_id: $domain.character_ref, seq: $next_seq, version: 1, \
       kind: $domain.kind, name: $domain.name, description: $domain.description, \
       reference_asset_id: $domain.reference_asset_ref, reference_ref: $domain.reference_ref, \
       source_ref: $domain.source_ref, crop_ref: $domain.crop_ref, artifact_ref: $domain.artifact_ref, \
       provenance: $domain.provenance } RETURN ",
    profile_columns!(),
    ")[0]; };"
);

const GET_IDENTITY_PROFILE_STATEMENT: &str = concat!(
    "SELECT ",
    profile_columns!(),
    " FROM $record_id WHERE deleted_at_utc = NONE LIMIT 1;"
);

const UPDATE_IDENTITY_PROFILE_STATEMENT: &str = concat!(
    "RETURN { LET $current = (SELECT VALUE id FROM ONLY $domain.profile_ref \
       WHERE deleted_at_utc = NONE); IF $current = NONE { RETURN NONE; }; ",
    atelier_event_sql!(),
    " UPDATE $domain.profile_ref SET name = $domain.name, description = $domain.description, \
       source_ref = $domain.source_ref, crop_ref = $domain.crop_ref, \
       artifact_ref = $domain.artifact_ref, version += 1, updated_at_utc = time::now(); \
       RETURN (SELECT ",
    profile_columns!(),
    " FROM ONLY $domain.profile_ref); };"
);

const DELETE_IDENTITY_PROFILE_STATEMENT: &str = concat!(
    "RETURN { LET $current = (SELECT VALUE id FROM ONLY $domain.profile_ref \
       WHERE deleted_at_utc = NONE); IF $current = NONE { RETURN NONE; }; ",
    atelier_event_sql!(),
    " UPDATE $domain.profile_ref SET deleted_at_utc = time::now(), version += 1, \
       updated_at_utc = time::now(); RETURN (SELECT ",
    profile_columns!(),
    " FROM ONLY $domain.profile_ref); };"
);

const LIST_IDENTITY_PROFILES_STATEMENT: &str = concat!(
    "SELECT ",
    profile_columns!(),
    " FROM atelier_identity_profile WHERE character_internal_id = $character_ref \
       AND ($kind = NONE OR kind = $kind) AND deleted_at_utc = NONE ORDER BY seq ASC;"
);

const LATEST_IDENTITY_PROFILE_STATEMENT: &str = concat!(
    "SELECT ",
    profile_columns!(),
    " FROM atelier_identity_profile WHERE character_internal_id = $character_ref \
       AND kind = $kind AND deleted_at_utc = NONE ORDER BY seq DESC LIMIT 1;"
);

const GET_IDENTITY_CROP_PROFILE_STATEMENT: &str =
    "SELECT record::id(character_internal_id) AS character_internal_id, version \
     FROM $record_id WHERE deleted_at_utc = NONE LIMIT 1;";

const FIND_IDENTITY_CROP_STATEMENT: &str = concat!(
    "SELECT ",
    identity_crop_columns!(),
    " FROM atelier_identity_crop_artifact WHERE profile_id = $profile_ref \
       AND profile_version = $profile_version AND content_hash = $content_hash LIMIT 1;"
);

const WRITE_IDENTITY_CROP_STATEMENT: &str = concat!(
    "RETURN { LET $created = (CREATE $domain.record_id CONTENT { crop_id: $domain.crop_id, \
       profile_id: $domain.profile_ref, profile_version: $domain.profile_version, \
       character_internal_id: $domain.character_ref, source_ref: $domain.source_ref, \
       crop_box: $domain.crop_box, landmarks: $domain.landmarks, \
       artifact_ref: $domain.artifact_ref, manifest_ref: $domain.manifest_ref, \
       content_hash: $domain.content_hash, byte_len: $domain.byte_len, mime: $domain.mime, \
       width: $domain.width, height: $domain.height, manifest: $domain.manifest, \
       created_by: $domain.created_by } RETURN ",
    identity_crop_columns!(),
    ")[0]; ",
    atelier_event_sql!(),
    " RETURN $created; };"
);

fn is_identity_crop_unique_conflict(error: &AtelierError) -> bool {
    let text = error.to_string();
    text.contains("Database index")
        && text.contains("uq_atelier_identity_crop_artifact_profile_version_hash")
        && text.contains("already contains")
}

const IDENTITY_CROP_TRANSACTION_MAX_ATTEMPTS: usize = 10;
const IDENTITY_CROP_TRANSACTION_BACKOFF_CAP_MS: u64 = 32;

fn is_identity_crop_retryable_transaction_conflict(error: &AtelierError) -> bool {
    matches!(
        error,
        AtelierError::Database(crate::storage::surreal::SurrealStorageError::Database(source))
            if source
                .to_string()
                .contains("Transaction conflict: Resource busy. This transaction can be retried")
    )
}

fn identity_crop_transaction_retry_delay(seed: Uuid, failed_attempt: usize) -> Duration {
    let exponential_cap = 1_u64
        .checked_shl(failed_attempt.min(5) as u32)
        .unwrap_or(IDENTITY_CROP_TRANSACTION_BACKOFF_CAP_MS)
        .min(IDENTITY_CROP_TRANSACTION_BACKOFF_CAP_MS);
    let seed = seed.as_u128();
    let mut mixed = (seed as u64)
        ^ ((seed >> 64) as u64)
        ^ (failed_attempt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mixed ^= mixed >> 30;
    mixed = mixed.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed ^= mixed >> 27;
    mixed = mixed.wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^= mixed >> 31;
    Duration::from_millis(mixed % (exponential_cap + 1))
}

async fn wait_before_identity_crop_transaction_retry(seed: Uuid, failed_attempt: usize) {
    tokio::time::sleep(identity_crop_transaction_retry_delay(seed, failed_attempt)).await;
}

const GET_IDENTITY_CROP_STATEMENT: &str = concat!(
    "SELECT ",
    identity_crop_columns!(),
    " FROM $record_id LIMIT 1;"
);

const LIST_IDENTITY_CROPS_STATEMENT: &str = concat!(
    "SELECT ",
    identity_crop_columns!(),
    " FROM atelier_identity_crop_artifact WHERE profile_id = $record_id \
     ORDER BY created_at_utc ASC, crop_id ASC;"
);

fn identity_crop_artifact_manifest(
    crop_id: Uuid,
    profile_id: Uuid,
    profile_version: i64,
    source_ref: &str,
    crop_box: &IdentityCropBox,
    landmarks: &[IdentityCropLandmark],
    artifact_ref: &str,
    manifest_ref: &str,
    content_hash: &str,
    byte_len: i64,
    mime: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema": IDENTITY_CROP_ARTIFACT_MANIFEST_SCHEMA,
        "crop_id": crop_id,
        "profile_id": profile_id,
        "profile_version": profile_version,
        "source_ref": source_ref,
        "crop_box": crop_box,
        "landmarks": landmarks,
        "artifact_store": {
            "handle": artifact_ref,
            "manifest": manifest_ref,
            "content_hash": content_hash,
            "size_bytes": byte_len,
            "mime": mime,
            "width": 512,
            "height": 512,
        }
    })
}

impl AtelierStore {
    async fn find_identity_crop_artifact_by_identity(
        &self,
        profile_id: Uuid,
        profile_version: i64,
        content_hash: &str,
    ) -> AtelierResult<Option<IdentityCropArtifact>> {
        let identity = IdentityCropIdentityBinding {
            profile_ref: RecordId::new("atelier_identity_profile", SurrealUuid::from(profile_id)),
            profile_version,
            content_hash: content_hash.to_owned(),
        };
        let existing: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(FIND_IDENTITY_CROP_STATEMENT, identity)
                        .await
                })
            })
            .await?;
        existing
            .map(|row| identity_crop_artifact_from_row(&pose_row(row)?))
            .transpose()
    }

    /// Record a head pose by deriving the normalized quaternion from legacy
    /// source YXZ Euler degrees (pitch=X, yaw=Y, roll=Z).
    pub async fn record_head_pose_from_yxz_euler(
        &self,
        rig_id: Uuid,
        yaw_deg: f64,
        pitch_deg: f64,
        roll_deg: f64,
    ) -> AtelierResult<HeadPose> {
        let quaternion = quaternion_from_yxz_euler_degrees(yaw_deg, pitch_deg, roll_deg);
        self.record_head_pose(rig_id, yaw_deg, pitch_deg, roll_deg, quaternion)
            .await
    }

    /// Import a legacy yaw-only head-pose value by preserving yaw and deriving
    /// the equivalent YXZ quaternion with zero pitch/roll.
    pub async fn import_legacy_yaw_head_pose(
        &self,
        rig_id: Uuid,
        yaw_deg: f64,
    ) -> AtelierResult<HeadPose> {
        self.record_head_pose_from_yxz_euler(rig_id, yaw_deg, 0.0, 0.0)
            .await
    }

    /// Ingest a pose rig artifact written through by a Workflow-Engine detection
    /// job (MT-PoseKit). No detector runs here; this stores the governed record.
    ///
    /// Idempotent on `(character_internal_id, source_ref, content_hash)`:
    /// re-ingesting an identical rig returns the existing row instead of
    /// duplicating it (mirrors the DAM content-hash dedup, MT-015). The
    /// OpenPose keypoint payload is structurally validated (body-18 required;
    /// face-70 and hand-21 optional and zero-fillable, per legacy source
    /// `rigToOpenposeJson`). The character FK is guarded explicitly so a bad id
    /// is a clean not-found rather than a raw constraint violation. Emits
    /// `POSE_RIG_INGESTED`.
    pub async fn ingest_pose_rig(&self, new: &NewPoseRig) -> AtelierResult<PoseRig> {
        if new.source_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "source_ref must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("source_ref", &new.source_ref)?;
        if let Some(sidecar_ref) = &new.sidecar_ref {
            reject_legacy_runtime_ref("sidecar_ref", sidecar_ref)?;
        }
        if new.content_hash.trim().is_empty() {
            return Err(AtelierError::Validation(
                "content_hash must not be empty".into(),
            ));
        }
        if new.canvas.width <= 0 || new.canvas.height <= 0 {
            return Err(AtelierError::Validation(
                "canvas width/height must be positive".into(),
            ));
        }
        if new.detector_provider.trim().is_empty()
            || new.detector_provider.trim() != new.detector_provider
        {
            return Err(AtelierError::Validation(
                "detector_provider must not be empty or padded".into(),
            ));
        }
        if new.detector_model.trim().is_empty() || new.detector_model.trim() != new.detector_model {
            return Err(AtelierError::Validation(
                "detector_model must not be empty or padded".into(),
            ));
        }
        if new.detector_model_version.trim().is_empty()
            || new.detector_model_version.trim() != new.detector_model_version
        {
            return Err(AtelierError::Validation(
                "detector_model_version must not be empty or padded".into(),
            ));
        }
        if let Some(source_asset_version_ref) = &new.source_asset_version_ref {
            reject_legacy_runtime_ref("source_asset_version_ref", source_asset_version_ref)?;
        }
        if let Some(source_asset_path_ref) = &new.source_asset_path_ref {
            reject_legacy_runtime_ref("source_asset_path_ref", source_asset_path_ref)?;
        }
        validate_detector_error_reason(new.detector_status, new.error_reason.as_deref())?;
        validate_keypoints(&new.keypoints_json)?;

        let identity = PoseRigIdentityBinding {
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.character_internal_id),
            ),
            source_ref: new.source_ref.clone(),
            content_hash: new.content_hash.clone(),
        };
        let existing_id: Option<SurrealUuid> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(FIND_POSE_RIG_ID_STATEMENT, identity).await })
            })
            .await?;
        let rig_id = existing_id.map(Into::into).unwrap_or_else(Uuid::now_v7);
        let bindings = PoseRigWriteBindings {
            record_id: RecordId::new("atelier_pose_rig", SurrealUuid::from(rig_id)),
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.character_internal_id),
            ),
            source_asset_ref: new
                .source_asset_id
                .map(|asset_id| RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id))),
            source_ref: new.source_ref.clone(),
            content_hash: new.content_hash.clone(),
            canvas_width: new.canvas.width,
            canvas_height: new.canvas.height,
            detector_provider: new.detector_provider.clone(),
            detector_model: new.detector_model.clone(),
            detector_model_version: new.detector_model_version.clone(),
            source_asset_version_ref: new.source_asset_version_ref.clone(),
            source_asset_path_ref: new.source_asset_path_ref.clone(),
            confidence_available: new.confidence_available,
            detector_status: new.detector_status.as_token().to_owned(),
            error_reason: new.error_reason.clone(),
            keypoints_json: new.keypoints_json.clone(),
            sidecar_ref: new.sidecar_ref.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_POSE_RIG_STATEMENT,
                bindings,
                POSE_RIG_INGESTED,
                "atelier_pose_rig",
                &rig_id.to_string(),
                serde_json::json!({
                    "rig_id": rig_id,
                    "source_asset_id": new.source_asset_id,
                    "source_ref": new.source_ref,
                    "content_hash": new.content_hash,
                    "detector_provider": new.detector_provider,
                    "detector_model": new.detector_model,
                    "detector_model_version": new.detector_model_version,
                    "source_asset_version_ref": new.source_asset_version_ref,
                    "source_asset_path_ref": new.source_asset_path_ref,
                    "confidence_available": new.confidence_available,
                    "detector_status": new.detector_status.as_token(),
                    "error_reason": new.error_reason,
                    "canvas_width": new.canvas.width,
                    "canvas_height": new.canvas.height,
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::NotFound(format!(
                "atelier_character internal_id={}",
                new.character_internal_id
            ))
        })?;
        rig_from_row(&pose_row(row)?)
    }

    /// Fetch a pose rig by id.
    pub async fn get_pose_rig(&self, rig_id: Uuid) -> AtelierResult<PoseRig> {
        let binding = PoseRigIdBinding {
            rig_ref: RecordId::new("atelier_pose_rig", SurrealUuid::from(rig_id)),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_POSE_RIG_STATEMENT, binding).await })
            })
            .await?;
        let row = row.ok_or_else(|| AtelierError::NotFound(format!("pose rig_id={rig_id}")))?;
        rig_from_row(&pose_row(row)?)
    }

    /// List a character's pose rigs, newest first.
    pub async fn list_pose_rigs(
        &self,
        character_internal_id: Uuid,
        limit: i64,
    ) -> AtelierResult<Vec<PoseRig>> {
        let capped = limit.clamp(1, 1000);
        let bindings = PoseRigListBindings {
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(character_internal_id),
            ),
            limit: capped,
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_POSE_RIGS_STATEMENT, bindings).await })
            })
            .await?;
        rows.into_iter()
            .map(|row| rig_from_row(&pose_row(row)?))
            .collect()
    }

    /// Append a typed pose context state for a workspace (MT-094).
    ///
    /// This is a history table, not a mutable singleton. Switching between
    /// blank, single-image, character-linked, and collection-linked contexts
    /// inserts a new row and preserves existing rigs, source media, and linked
    /// collections. Emits `POSE_CONTEXT_STATE_SET`.
    pub async fn set_pose_context_state(
        &self,
        new: &NewPoseContextState,
    ) -> AtelierResult<PoseContextState> {
        validate_pose_context_request(new)?;

        let selected_rig = match new.selected_rig_id {
            Some(rig_id) => Some(self.get_pose_rig(rig_id).await?),
            None => None,
        };

        if let Some(source_asset_id) = new.source_asset_id {
            if !pose_record_exists(
                self,
                RecordId::new("atelier_media_asset", SurrealUuid::from(source_asset_id)),
            )
            .await?
            {
                return Err(AtelierError::NotFound(format!(
                    "atelier_media_asset asset_id={source_asset_id}"
                )));
            }
            if let Some(rig) = &selected_rig {
                if rig.source_asset_id != Some(source_asset_id) {
                    return Err(AtelierError::Validation(
                        "selected_rig_id must belong to the same source_asset_id".into(),
                    ));
                }
            }
        }

        if let Some(character_internal_id) = new.character_internal_id {
            if !pose_record_exists(
                self,
                RecordId::new(
                    "atelier_character",
                    SurrealUuid::from(character_internal_id),
                ),
            )
            .await?
            {
                return Err(AtelierError::NotFound(format!(
                    "atelier_character internal_id={character_internal_id}"
                )));
            }
            if let Some(rig) = &selected_rig {
                if rig.character_internal_id != character_internal_id {
                    return Err(AtelierError::Validation(
                        "selected_rig_id belongs to a different character_internal_id".into(),
                    ));
                }
            }
        }

        if let Some(collection_id) = new.collection_id {
            let collection = self.get_collection(collection_id).await?;
            if let Some(collection_character) = collection.character_internal_id {
                if let Some(context_character) = new.character_internal_id {
                    if context_character != collection_character {
                        return Err(AtelierError::Validation(
                            "collection_id belongs to a different character_internal_id".into(),
                        ));
                    }
                }
                if let Some(rig) = &selected_rig {
                    if rig.character_internal_id != collection_character {
                        return Err(AtelierError::Validation(
                            "selected_rig_id does not belong to collection_id character".into(),
                        ));
                    }
                }
            } else if let (Some(context_character), Some(rig)) =
                (new.character_internal_id, &selected_rig)
            {
                if rig.character_internal_id != context_character {
                    return Err(AtelierError::Validation(
                        "selected_rig_id belongs to a different character_internal_id".into(),
                    ));
                }
            }
        }

        let context_id = Uuid::now_v7();
        let bindings = PoseContextWriteBindings {
            record_id: RecordId::new("atelier_pose_context_state", SurrealUuid::from(context_id)),
            context_id: SurrealUuid::from(context_id),
            workspace_ref: new.workspace_ref.clone(),
            kind: new.kind.as_token().to_owned(),
            source_asset_ref: new
                .source_asset_id
                .map(|asset_id| RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id))),
            character_ref: new.character_internal_id.map(|character_id| {
                RecordId::new("atelier_character", SurrealUuid::from(character_id))
            }),
            collection_ref: new.collection_id.map(|collection_id| {
                RecordId::new("atelier_collection", SurrealUuid::from(collection_id))
            }),
            selected_rig_ref: new
                .selected_rig_id
                .map(|rig_id| RecordId::new("atelier_pose_rig", SurrealUuid::from(rig_id))),
            requested_by: new.requested_by.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_POSE_CONTEXT_STATEMENT,
                bindings,
                POSE_CONTEXT_STATE_SET,
                "atelier_pose_context_state",
                &context_id.to_string(),
                serde_json::json!({
                    "context_id": context_id,
                    "workspace_ref": new.workspace_ref,
                    "kind": new.kind.as_token(),
                    "source_asset_id": new.source_asset_id,
                    "character_internal_id": new.character_internal_id,
                    "collection_id": new.collection_id,
                    "selected_rig_id": new.selected_rig_id,
                    "requested_by": new.requested_by,
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::Internal("recording pose context state returned no row".to_owned())
        })?;
        context_state_from_row(&pose_row(row)?)
    }

    /// Read the latest pose context state for a workspace.
    pub async fn current_pose_context_state(
        &self,
        workspace_ref: &str,
    ) -> AtelierResult<Option<PoseContextState>> {
        validate_pose_context_ref("workspace_ref", workspace_ref)?;
        let binding = PoseWorkspaceBinding {
            workspace_ref: workspace_ref.to_owned(),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(CURRENT_POSE_CONTEXT_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        row.map(|row| context_state_from_row(&pose_row(row)?))
            .transpose()
    }

    /// List pose context state history oldest first for deterministic replay.
    pub async fn list_pose_context_history(
        &self,
        workspace_ref: &str,
    ) -> AtelierResult<Vec<PoseContextState>> {
        validate_pose_context_ref("workspace_ref", workspace_ref)?;
        let binding = PoseWorkspaceBinding {
            workspace_ref: workspace_ref.to_owned(),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(
                    async move { ctx.query_values(LIST_POSE_CONTEXT_STATEMENT, binding).await },
                )
            })
            .await?;
        rows.into_iter()
            .map(|row| context_state_from_row(&pose_row(row)?))
            .collect()
    }

    /// Set one open rig tab's workspace state (MT-096).
    ///
    /// The workspace can hold multiple rigs. `sort_order` controls tab order,
    /// `active=true` makes this rig the sole active tab for the workspace, and
    /// `dirty_calibration`/`panel_state` are persisted as structured state for
    /// Diagnostics and multi-model recovery.
    pub async fn upsert_pose_workspace_rig_state(
        &self,
        new: &NewPoseWorkspaceRigState,
    ) -> AtelierResult<PoseWorkspaceRigState> {
        validate_pose_workspace_rig_state_request(new)?;
        let _ = self.get_pose_rig(new.rig_id).await?;
        let aggregate_id = format!("{}:{}:{}", new.workspace_ref, new.session_ref, new.rig_id);
        let state_record_id = Uuid::now_v7();
        let bindings = PoseWorkspaceRigWriteBindings {
            record_id: RecordId::new(
                "atelier_pose_workspace_rig_state",
                SurrealUuid::from(state_record_id),
            ),
            workspace_ref: new.workspace_ref.clone(),
            session_ref: new.session_ref.clone(),
            rig_ref: RecordId::new("atelier_pose_rig", SurrealUuid::from(new.rig_id)),
            open: new.open,
            sort_order: new.sort_order,
            active: new.active,
            dirty_calibration: new.dirty_calibration,
            panel_state: new.panel_state.clone(),
            requested_by: new.requested_by.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_POSE_WORKSPACE_RIG_STATEMENT,
                bindings,
                POSE_WORKSPACE_RIG_STATE_SET,
                "atelier_pose_workspace_rig_state",
                &aggregate_id,
                serde_json::json!({
                    "workspace_ref": new.workspace_ref,
                    "session_ref": new.session_ref,
                    "rig_id": new.rig_id,
                    "open": new.open,
                    "sort_order": new.sort_order,
                    "active": new.active,
                    "dirty_calibration": new.dirty_calibration,
                    "panel_state_is_object": new.panel_state.is_object(),
                    "requested_by": new.requested_by,
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::Validation(
                "pose workspace sort_order must be unique among open rigs".into(),
            )
        })?;
        Ok(workspace_rig_state_from_row(&pose_row(row)?))
    }

    /// List open rig tab state for a workspace in deterministic tab order.
    pub async fn list_pose_workspace_rig_state(
        &self,
        workspace_ref: &str,
        session_ref: &str,
    ) -> AtelierResult<Vec<PoseWorkspaceRigState>> {
        validate_pose_context_ref("workspace_ref", workspace_ref)?;
        validate_pose_context_ref("session_ref", session_ref)?;
        let binding = PoseWorkspaceSessionBinding {
            workspace_ref: workspace_ref.to_owned(),
            session_ref: session_ref.to_owned(),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_POSE_WORKSPACE_RIG_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(|row| Ok(workspace_rig_state_from_row(&pose_row(row)?)))
            .collect()
    }

    /// Resolve and persist a durable product route to one rig in a multi-rig pose workspace (MT-097).
    pub async fn route_pose_workspace_to_rig(
        &self,
        new: &NewPoseWorkspaceRouteTarget,
    ) -> AtelierResult<PoseWorkspaceRouteResolution> {
        validate_pose_workspace_route_target(new)?;
        let states = self
            .list_pose_workspace_rig_state(&new.workspace_ref, &new.session_ref)
            .await?;
        let Some(state) = states.iter().find(|state| state.rig_id == new.rig_id) else {
            return Err(AtelierError::Validation(
                "pose workspace route rig_id must be an open rig in the workspace session".into(),
            ));
        };
        let route_ref = pose_workspace_route_ref(
            &new.workspace_ref,
            &new.session_ref,
            new.rig_id,
            &new.panel_id,
        );
        let mut panel_state = state
            .panel_state
            .as_object()
            .cloned()
            .unwrap_or_else(serde_json::Map::new);
        panel_state.insert("panel".to_string(), serde_json::json!(new.panel_id));
        panel_state.insert("route_ref".to_string(), serde_json::json!(route_ref));
        panel_state.insert(
            "route_semantics_schema".to_string(),
            serde_json::json!("hsk.atelier.pose_workspace_route@1"),
        );
        let updated = self
            .upsert_pose_workspace_rig_state(&NewPoseWorkspaceRigState {
                workspace_ref: new.workspace_ref.clone(),
                session_ref: new.session_ref.clone(),
                rig_id: new.rig_id,
                open: true,
                sort_order: state.sort_order,
                active: true,
                dirty_calibration: state.dirty_calibration,
                panel_state: serde_json::Value::Object(panel_state),
                requested_by: new.requested_by.clone(),
            })
            .await?;

        Ok(PoseWorkspaceRouteResolution {
            route_ref: pose_workspace_route_ref(
                &new.workspace_ref,
                &new.session_ref,
                new.rig_id,
                &new.panel_id,
            ),
            workspace_ref: updated.workspace_ref,
            session_ref: updated.session_ref,
            rig_id: updated.rig_id,
            panel_id: new.panel_id.clone(),
            active_sort_order: updated.sort_order,
            open_rig_count: states.len() as i32,
            keyboard_action: None,
        })
    }

    /// Apply a keyboard navigation action using the durable open-rig tab order (MT-097).
    pub async fn apply_pose_workspace_keyboard_action(
        &self,
        request: &PoseWorkspaceKeyboardActionRequest,
    ) -> AtelierResult<PoseWorkspaceRouteResolution> {
        validate_pose_workspace_keyboard_action(request)?;
        let states = self
            .list_pose_workspace_rig_state(&request.workspace_ref, &request.session_ref)
            .await?;
        if states.is_empty() {
            return Err(AtelierError::Validation(
                "pose workspace keyboard action requires at least one open rig".into(),
            ));
        }
        let active_index = states.iter().position(|state| state.active).unwrap_or(0);
        let target_index = match request.action {
            PoseWorkspaceKeyboardAction::ActivateNextRig => (active_index + 1) % states.len(),
            PoseWorkspaceKeyboardAction::ActivatePreviousRig => {
                if active_index == 0 {
                    states.len() - 1
                } else {
                    active_index - 1
                }
            }
        };
        let target = &states[target_index];
        let mut route = self
            .route_pose_workspace_to_rig(&NewPoseWorkspaceRouteTarget {
                workspace_ref: request.workspace_ref.clone(),
                session_ref: request.session_ref.clone(),
                rig_id: target.rig_id,
                panel_id: request.panel_id.clone(),
                requested_by: format!("{}:{}", request.requested_by, request.action.as_token()),
            })
            .await?;
        route.keyboard_action = Some(request.action);
        Ok(route)
    }

    /// Register a typed pose sidecar artifact for a rig (MT-092).
    ///
    /// The detector/renderer runs out-of-module. This method only records the
    /// governed artifact refs for OpenPose JSON, OpenPose PNG previews, and
    /// conditioning PNGs. Idempotent on `(rig_id, kind)` so reruns replace the
    /// current artifact for the same role without creating duplicate sidecar
    /// roles. Emits `POSE_SIDECAR_RECORDED`.
    pub async fn record_pose_sidecar(&self, new: &NewPoseSidecar) -> AtelierResult<PoseSidecar> {
        validate_pose_sidecar(new)?;
        let rig = self.get_pose_rig(new.rig_id).await?;
        self.write_pose_sidecar_row(new, &rig).await
    }

    /// Register multiple typed pose sidecars for one rig (the OpenPose JSON +
    /// PNG pair a Posekit export produces). Every sidecar is validated against
    /// its ArtifactStore payload BEFORE the first write, so a bad member fails
    /// the whole batch without leaving a partial pair behind. Each row is then
    /// written together with its own `POSE_SIDECAR_RECORDED` event in one
    /// atomic statement (the `write_with_event` contract), preserving the
    /// one-event-per-sidecar ledger shape of the reference implementation.
    pub async fn record_pose_sidecars(
        &self,
        sidecars: &[NewPoseSidecar],
    ) -> AtelierResult<Vec<PoseSidecar>> {
        let Some(first) = sidecars.first() else {
            return Ok(Vec::new());
        };
        let rig_id = first.rig_id;
        for sidecar in sidecars {
            if sidecar.rig_id != rig_id {
                return Err(AtelierError::Validation(
                    "pose sidecar batch must target one rig_id".into(),
                ));
            }
            validate_pose_sidecar(sidecar)?;
        }
        let rig = self.get_pose_rig(rig_id).await?;
        let mut recorded = Vec::with_capacity(sidecars.len());
        for new in sidecars {
            recorded.push(self.write_pose_sidecar_row(new, &rig).await?);
        }
        Ok(recorded)
    }

    /// Resolve the sidecar row bound to an ArtifactStore payload ref, if any.
    /// This is the authority check the Posekit PNG byte route performs before
    /// serving bytes: a Posekit-shaped manifest alone is not enough.
    pub async fn get_pose_sidecar_by_artifact_ref(
        &self,
        artifact_ref: &str,
    ) -> AtelierResult<Option<PoseSidecar>> {
        let trimmed = artifact_ref.trim();
        if trimmed.is_empty() {
            return Err(AtelierError::Validation(
                "artifact_ref lookup must be non-empty".into(),
            ));
        }
        let binding = PoseSidecarArtifactRefBinding {
            artifact_ref: trimmed.to_owned(),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_POSE_SIDECAR_BY_ARTIFACT_REF_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        row.map(|row| sidecar_from_row(&pose_row(row)?)).transpose()
    }

    async fn write_pose_sidecar_row(
        &self,
        new: &NewPoseSidecar,
        rig: &PoseRig,
    ) -> AtelierResult<PoseSidecar> {
        let identity = PoseSidecarIdentityBinding {
            rig_ref: RecordId::new("atelier_pose_rig", SurrealUuid::from(new.rig_id)),
            kind: new.kind.as_token().to_owned(),
        };
        let existing_id: Option<SurrealUuid> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(FIND_POSE_SIDECAR_ID_STATEMENT, identity)
                        .await
                })
            })
            .await?;
        let sidecar_id = existing_id.map(Into::into).unwrap_or_else(Uuid::now_v7);
        let bindings = PoseSidecarWriteBindings {
            record_id: RecordId::new("atelier_pose_sidecar", SurrealUuid::from(sidecar_id)),
            rig_ref: RecordId::new("atelier_pose_rig", SurrealUuid::from(new.rig_id)),
            source_asset_ref: rig
                .source_asset_id
                .map(|asset_id| RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id))),
            source_ref: rig.source_ref.clone(),
            kind: new.kind.as_token().to_owned(),
            role: new.kind.as_token().to_owned(),
            artifact_ref: new.artifact_ref.clone(),
            manifest_ref: new.manifest_ref.clone(),
            content_hash: new.content_hash.clone(),
            byte_len: new.byte_len,
            mime: new.mime.clone(),
            width: new.width,
            height: new.height,
            status: new.status.as_token().to_owned(),
            error_message: new.error_message.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_POSE_SIDECAR_STATEMENT,
                bindings,
                POSE_SIDECAR_RECORDED,
                "atelier_pose_sidecar",
                &sidecar_id.to_string(),
                serde_json::json!({
                    "sidecar_id": sidecar_id,
                    "rig_id": new.rig_id,
                    "source_asset_id": rig.source_asset_id,
                    "source_ref": rig.source_ref,
                    "kind": new.kind.as_token(),
                    "role": new.kind.as_token(),
                    "artifact_ref": new.artifact_ref,
                    "manifest_ref": new.manifest_ref,
                    "content_hash": new.content_hash,
                    "byte_len": new.byte_len,
                    "mime": new.mime,
                    "width": new.width,
                    "height": new.height,
                    "status": new.status.as_token(),
                    "has_error_message": new.error_message.is_some(),
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::Internal("recording pose sidecar returned no row".to_owned())
        })?;
        sidecar_from_row(&pose_row(row)?)
    }

    /// List typed sidecars for a rig in deterministic OpenPose/PNG/conditioning order.
    pub async fn list_pose_sidecars(&self, rig_id: Uuid) -> AtelierResult<Vec<PoseSidecar>> {
        let binding = PoseRigIdBinding {
            rig_ref: RecordId::new("atelier_pose_rig", SurrealUuid::from(rig_id)),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_POSE_SIDECARS_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        let mut sidecars = rows
            .into_iter()
            .map(|row| sidecar_from_row(&pose_row(row)?))
            .collect::<AtelierResult<Vec<_>>>()?;
        sidecars.sort_by_key(|sidecar| {
            (
                pose_sidecar_kind_order(sidecar.kind),
                sidecar.created_at_utc.clone(),
                sidecar.sidecar_id,
            )
        });
        Ok(sidecars)
    }

    /// Lookup typed pose sidecars by source image identity, optionally scoped to
    /// a specific rig. This is the pose-specific discovery path; sidecars are
    /// still hidden from normal galleries by [`pose_sidecar_gallery_projection`].
    pub async fn list_pose_sidecars_for_source(
        &self,
        source_ref: &str,
        rig_id: Option<Uuid>,
    ) -> AtelierResult<Vec<PoseSidecar>> {
        let source_ref = validate_pose_source_ref_for_lookup(source_ref)?;
        let binding = PoseSidecarSourceBinding {
            source_ref,
            rig_ref: rig_id.map(|id| RecordId::new("atelier_pose_rig", SurrealUuid::from(id))),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_POSE_SIDECARS_FOR_SOURCE_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        let mut sidecars = rows
            .into_iter()
            .map(|row| sidecar_from_row(&pose_row(row)?))
            .collect::<AtelierResult<Vec<_>>>()?;
        sidecars.sort_by_key(|sidecar| {
            (
                pose_sidecar_kind_order(sidecar.kind),
                sidecar.created_at_utc.clone(),
                sidecar.sidecar_id,
            )
        });
        Ok(sidecars)
    }

    /// Projection contract for normal galleries: pose sidecars are hidden from
    /// gallery-visible media surfaces while remaining traceable by pose routes.
    pub async fn pose_sidecar_gallery_projection(
        &self,
        rig_id: Uuid,
    ) -> AtelierResult<Vec<PoseSidecarGalleryProjection>> {
        let sidecars = self.list_pose_sidecars(rig_id).await?;
        Ok(sidecars
            .into_iter()
            .map(|sidecar| PoseSidecarGalleryProjection {
                sidecar_id: sidecar.sidecar_id,
                rig_id: sidecar.rig_id,
                kind: sidecar.kind,
                artifact_ref: sidecar.artifact_ref,
                gallery_visible: false,
                hidden_reason: "pose_sidecar".to_string(),
                jump_target: format!("atelier://pose-rig/{}/sidecars", sidecar.rig_id),
            })
            .collect())
    }

    /// Source-image strip projection for Diagnostics (MT-095).
    ///
    /// Returns structured source-image state for pose rigs without requiring a
    /// model or UI test to scrape rendered galleries. Rigs without a persisted
    /// media asset remain visible to Diagnostics through `source_ref`; media
    /// artifact fields are populated when a DAM asset exists.
    pub async fn pose_source_image_strip_state(
        &self,
        character_internal_id: Uuid,
        limit: i64,
    ) -> AtelierResult<Vec<PoseSourceImageStripItem>> {
        let capped = limit.clamp(1, 1000);
        let bindings = PoseSourceStripBindings {
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(character_internal_id),
            ),
            limit: capped,
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_POSE_SOURCE_STRIP_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(|row| Ok(source_image_strip_item_from_row(&pose_row(row)?)))
            .collect()
    }

    /// OpenPose sidecar strip projection for Diagnostics (MT-095).
    ///
    /// Exposes OpenPose JSON and OpenPose PNG sidecars as structured rows while
    /// preserving the normal-gallery hidden contract. Conditioning PNGs remain
    /// available through `list_pose_sidecars` but are not OpenPose strip items.
    pub async fn pose_openpose_sidecar_strip_state(
        &self,
        rig_id: Uuid,
    ) -> AtelierResult<Vec<PoseOpenPoseSidecarStripItem>> {
        let _ = self.get_pose_rig(rig_id).await?;
        let binding = PoseRigIdBinding {
            rig_ref: RecordId::new("atelier_pose_rig", SurrealUuid::from(rig_id)),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_OPENPOSE_SIDECARS_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(|row| sidecar_from_row(&pose_row(row)?))
            .map(|result| result.map(openpose_sidecar_strip_item_from_sidecar))
            .collect()
    }

    /// Record (or replace) the head pose for a rig (MT-PoseKit).
    ///
    /// Validates the legacy source degree limits (yaw +-90 / pitch +-75 / roll +-45) and
    /// that the quaternion is finite and non-degenerate (lengthSq > 0), then
    /// stores the normalized quaternion. One head pose per rig: re-recording
    /// updates in place (upsert on `rig_id`). Emits `POSE_HEAD_POSE_RECORDED`.
    pub async fn record_head_pose(
        &self,
        rig_id: Uuid,
        yaw_deg: f64,
        pitch_deg: f64,
        roll_deg: f64,
        quaternion: [f64; 4],
    ) -> AtelierResult<HeadPose> {
        // Guard the rig FK explicitly for a clean not-found.
        let _ = self.get_pose_rig(rig_id).await?;

        for (name, value, limit) in [
            ("yaw", yaw_deg, YAW_LIMIT_DEG),
            ("pitch", pitch_deg, PITCH_LIMIT_DEG),
            ("roll", roll_deg, ROLL_LIMIT_DEG),
        ] {
            if !value.is_finite() {
                return Err(AtelierError::Validation(format!(
                    "head pose {name} must be a finite number"
                )));
            }
            if value < -limit || value > limit {
                return Err(AtelierError::Validation(format!(
                    "head pose {name}={value} out of range [-{limit}, {limit}]"
                )));
            }
        }

        if quaternion.iter().any(|c| !c.is_finite()) {
            return Err(AtelierError::Validation(
                "head pose quaternion components must be finite".into(),
            ));
        }
        let len_sq: f64 = quaternion.iter().map(|c| c * c).sum();
        if len_sq <= 0.0 {
            return Err(AtelierError::Validation(
                "head pose quaternion is degenerate (lengthSq <= 0)".into(),
            ));
        }
        // Normalize (legacy source `Quaternion.normalize`).
        let inv = 1.0 / len_sq.sqrt();
        let q = [
            quaternion[0] * inv,
            quaternion[1] * inv,
            quaternion[2] * inv,
            quaternion[3] * inv,
        ];

        let rig_ref = RecordId::new("atelier_pose_rig", SurrealUuid::from(rig_id));
        let bindings = PoseHeadPoseBindings {
            record_id: RecordId::new("atelier_pose_head_pose", SurrealUuid::from(rig_id)),
            rig_ref,
            yaw_deg,
            pitch_deg,
            roll_deg,
            quat_x: q[0],
            quat_y: q[1],
            quat_z: q[2],
            quat_w: q[3],
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_HEAD_POSE_STATEMENT,
                bindings,
                POSE_HEAD_POSE_RECORDED,
                "atelier_pose_rig",
                &rig_id.to_string(),
                serde_json::json!({
                    "rig_id": rig_id,
                    "yaw_deg": yaw_deg,
                    "pitch_deg": pitch_deg,
                    "roll_deg": roll_deg,
                    "quaternion": q,
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::Internal("recording head pose returned no row".to_owned())
        })?;
        Ok(head_pose_from_row(&pose_row(row)?))
    }

    /// Fetch the head pose for a rig, if recorded.
    pub async fn get_head_pose(&self, rig_id: Uuid) -> AtelierResult<Option<HeadPose>> {
        let binding = PoseRigIdBinding {
            rig_ref: RecordId::new("atelier_pose_head_pose", SurrealUuid::from(rig_id)),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_HEAD_POSE_STATEMENT, binding).await })
            })
            .await?;
        row.map(|row| Ok(head_pose_from_row(&pose_row(row)?)))
            .transpose()
    }

    /// Set the calibration record for a rig, preserved as BLOCKED/unresolved by
    /// default (MT-PoseKit). The spec Calibration Panel (10.10.4.1.9) is not yet
    /// implementable, so calibrated values are NOT fabricated: the row records
    /// the state and a block reason. Upsert on `rig_id`. Emits
    /// `POSE_CALIBRATION_SET`.
    pub async fn set_calibration(
        &self,
        rig_id: Uuid,
        state: CalibrationState,
        block_reason: Option<&str>,
    ) -> AtelierResult<Calibration> {
        self.set_pose_calibration(&NewPoseCalibration {
            rig_id,
            state,
            block_reason: block_reason.map(str::to_string),
            head_pose_ref: None,
            marker_visibility: CalibrationMarkerVisibility::default(),
            marker_colors: CalibrationMarkerColors::default(),
            hand_rows: Vec::new(),
            history_refs: Vec::new(),
        })
        .await
    }

    /// Set the full typed calibration record for a rig (MT-090).
    ///
    /// State/blocking fields are preserved together with typed head-pose refs,
    /// marker visibility/colors, hand rows, and history refs so calibration
    /// detail is not flattened into a status string.
    pub async fn set_pose_calibration(
        &self,
        new: &NewPoseCalibration,
    ) -> AtelierResult<Calibration> {
        // Guard the rig FK explicitly for a clean not-found.
        let _ = self.get_pose_rig(new.rig_id).await?;
        validate_pose_calibration(new)?;
        let marker_visibility = to_json_value("marker_visibility", &new.marker_visibility)?;
        let marker_colors = to_json_value("marker_colors", &new.marker_colors)?;
        let hand_rows = to_json_value("hand_rows", &new.hand_rows)?;
        let history_refs = to_json_value("history_refs", &new.history_refs)?;

        let bindings = PoseCalibrationBindings {
            record_id: RecordId::new("atelier_pose_calibration", SurrealUuid::from(new.rig_id)),
            rig_ref: RecordId::new("atelier_pose_rig", SurrealUuid::from(new.rig_id)),
            state: new.state.as_token().to_owned(),
            block_reason: new.block_reason.clone(),
            head_pose_ref: new.head_pose_ref.clone(),
            marker_visibility,
            marker_colors,
            hand_rows,
            history_refs,
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_CALIBRATION_STATEMENT,
                bindings,
                POSE_CALIBRATION_SET,
                "atelier_pose_rig",
                &new.rig_id.to_string(),
                serde_json::json!({
                    "rig_id": new.rig_id,
                    "state": new.state.as_token(),
                    "block_reason": new.block_reason,
                    "head_pose_ref": new.head_pose_ref,
                    "marker_visibility": new.marker_visibility,
                    "marker_colors": new.marker_colors,
                    "hand_rows": new.hand_rows,
                    "history_refs": new.history_refs,
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::Internal("recording pose calibration returned no row".to_owned())
        })?;
        calibration_from_row(&pose_row(row)?)
    }

    /// Fetch the calibration record for a rig, if any.
    pub async fn get_calibration(&self, rig_id: Uuid) -> AtelierResult<Option<Calibration>> {
        let binding = PoseRigIdBinding {
            rig_ref: RecordId::new("atelier_pose_calibration", SurrealUuid::from(rig_id)),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_CALIBRATION_STATEMENT, binding).await })
            })
            .await?;
        row.map(|row| calibration_from_row(&pose_row(row)?))
            .transpose()
    }

    /// Append a versioned identity profile for a character (MT-PoseKit).
    ///
    /// Append-only per character: the next `seq` is computed inside a
    /// transaction so concurrent appends cannot collide on `(character, seq)`.
    /// Provenance is redacted of any secret-looking material before storage and
    /// before the event payload (no raw cookies/tokens/auth ever persisted).
    /// The character FK is guarded explicitly. Emits `IDENTITY_PROFILE_APPENDED`.
    pub async fn append_identity_profile(
        &self,
        new: &NewIdentityProfile,
    ) -> AtelierResult<IdentityProfile> {
        validate_new_identity_profile(new)?;
        let name = redact_secrets(&new.name);
        let description = redact_secrets(&new.description);
        let provenance = redact_secrets(&new.provenance);

        let profile_id = Uuid::now_v7();
        let bindings = IdentityProfileAppendBindings {
            record_id: RecordId::new("atelier_identity_profile", SurrealUuid::from(profile_id)),
            profile_id: SurrealUuid::from(profile_id),
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.character_internal_id),
            ),
            kind: new.kind.as_token().to_owned(),
            name: name.clone(),
            description: description.clone(),
            reference_asset_ref: new
                .reference_asset_id
                .map(|asset_id| RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id))),
            reference_ref: new.reference_ref.clone(),
            source_ref: new.source_ref.clone(),
            crop_ref: new.crop_ref.clone(),
            artifact_ref: new.artifact_ref.clone(),
            provenance: provenance.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                APPEND_IDENTITY_PROFILE_STATEMENT,
                bindings,
                IDENTITY_PROFILE_APPENDED,
                "atelier_identity_profile",
                &profile_id.to_string(),
                serde_json::json!({
                    "profile_id": profile_id,
                    "version": 1,
                    "kind": new.kind.as_token(),
                    "reference_asset_id": new.reference_asset_id,
                    "name_ref": event_ref_for_text(&name),
                    "description_ref": event_ref_for_text(&description),
                    "reference_ref": new.reference_ref,
                    "source_ref": new.source_ref,
                    "crop_ref": new.crop_ref,
                    "artifact_ref": new.artifact_ref,
                    "provenance": provenance,
                    "mutation": "append",
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::NotFound(format!(
                "atelier_character internal_id={}",
                new.character_internal_id
            ))
        })?;
        identity_profile_from_row(&pose_row(row)?)
    }

    /// Fetch one active identity profile by id.
    pub async fn get_identity_profile(
        &self,
        profile_id: Uuid,
    ) -> AtelierResult<Option<IdentityProfile>> {
        let binding = PoseRecordBinding {
            record_id: RecordId::new("atelier_identity_profile", SurrealUuid::from(profile_id)),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_IDENTITY_PROFILE_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        row.map(|row| identity_profile_from_row(&pose_row(row)?))
            .transpose()
    }

    /// Update mutable identity profile metadata while preserving character/seq identity.
    pub async fn update_identity_profile(
        &self,
        update: &UpdateIdentityProfile,
    ) -> AtelierResult<IdentityProfile> {
        validate_update_identity_profile(update)?;
        let name = redact_secrets(&update.name);
        let description = redact_secrets(&update.description);
        let existing = self.get_identity_profile(update.profile_id).await?;
        let Some(existing) = existing else {
            return Err(AtelierError::NotFound(format!(
                "atelier_identity_profile profile_id={}",
                update.profile_id
            )));
        };
        let bindings = IdentityProfileUpdateBindings {
            profile_ref: RecordId::new(
                "atelier_identity_profile",
                SurrealUuid::from(update.profile_id),
            ),
            name: name.clone(),
            description: description.clone(),
            source_ref: update.source_ref.clone(),
            crop_ref: update.crop_ref.clone(),
            artifact_ref: update.artifact_ref.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                UPDATE_IDENTITY_PROFILE_STATEMENT,
                bindings,
                IDENTITY_PROFILE_APPENDED,
                "atelier_identity_profile",
                &update.profile_id.to_string(),
                serde_json::json!({
                    "profile_id": update.profile_id,
                    "seq": existing.seq,
                    "version": existing.version + 1,
                    "kind": existing.kind.as_token(),
                    "name_ref": event_ref_for_text(&name),
                    "description_ref": event_ref_for_text(&description),
                    "source_ref": update.source_ref,
                    "crop_ref": update.crop_ref,
                    "artifact_ref": update.artifact_ref,
                    "requested_by": update.requested_by,
                    "mutation": "update",
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::NotFound(format!(
                "atelier_identity_profile profile_id={}",
                update.profile_id
            ))
        })?;
        identity_profile_from_row(&pose_row(row)?)
    }

    /// Soft-delete an identity profile from normal CRUD/list projections.
    pub async fn delete_identity_profile(
        &self,
        profile_id: Uuid,
        requested_by: &str,
    ) -> AtelierResult<bool> {
        reject_legacy_runtime_ref("requested_by", requested_by)?;
        let existing = self.get_identity_profile(profile_id).await?;
        let Some(existing) = existing else {
            return Ok(false);
        };
        let bindings = IdentityProfileDeleteBindings {
            profile_ref: RecordId::new("atelier_identity_profile", SurrealUuid::from(profile_id)),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                DELETE_IDENTITY_PROFILE_STATEMENT,
                bindings,
                IDENTITY_PROFILE_APPENDED,
                "atelier_identity_profile",
                &profile_id.to_string(),
                serde_json::json!({
                    "profile_id": profile_id,
                    "seq": existing.seq,
                    "version": existing.version + 1,
                    "kind": existing.kind.as_token(),
                    "requested_by": requested_by,
                    "mutation": "delete",
                }),
            )
            .await?;
        Ok(row.is_some())
    }

    /// Record a normalized 512x512 face crop artifact linked to the current
    /// identity profile version. Idempotent on
    /// `(profile_id, profile_version, content_hash)` so a workflow retry returns
    /// the original crop row without duplicating events.
    pub async fn record_identity_crop_artifact(
        &self,
        new: &NewIdentityCropArtifact,
    ) -> AtelierResult<IdentityCropArtifact> {
        validate_identity_crop_artifact(new)?;
        let crop_box_json = to_json_value("identity crop_box", &new.crop_box)?;
        let landmarks_json = to_json_value("identity crop landmarks", &new.landmarks)?;

        let profile_ref = RecordId::new(
            "atelier_identity_profile",
            SurrealUuid::from(new.profile_id),
        );
        let profile_binding = PoseRecordBinding {
            record_id: profile_ref.clone(),
        };
        let profile_row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_IDENTITY_CROP_PROFILE_STATEMENT, profile_binding)
                        .await
                })
            })
            .await?;
        let Some(profile_row) = profile_row else {
            return Err(AtelierError::NotFound(format!(
                "atelier_identity_profile profile_id={}",
                new.profile_id
            )));
        };
        let profile_row = pose_row(profile_row)?;
        let character_internal_id: Uuid = profile_row.get("character_internal_id");
        let profile_version: i64 = profile_row.get("version");

        if let Some(existing) = self
            .find_identity_crop_artifact_by_identity(
                new.profile_id,
                profile_version,
                &new.content_hash,
            )
            .await?
        {
            return Ok(existing);
        }

        let crop_id = Uuid::now_v7();
        let manifest = identity_crop_artifact_manifest(
            crop_id,
            new.profile_id,
            profile_version,
            &new.source_ref,
            &new.crop_box,
            &new.landmarks,
            &new.artifact_ref,
            &new.manifest_ref,
            &new.content_hash,
            new.byte_len,
            &new.mime,
        );

        let bindings = IdentityCropBindings {
            record_id: RecordId::new("atelier_identity_crop_artifact", SurrealUuid::from(crop_id)),
            crop_id: SurrealUuid::from(crop_id),
            profile_ref,
            profile_version,
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(character_internal_id),
            ),
            source_ref: new.source_ref.clone(),
            crop_box: crop_box_json,
            landmarks: landmarks_json,
            artifact_ref: new.artifact_ref.clone(),
            manifest_ref: new.manifest_ref.clone(),
            content_hash: new.content_hash.clone(),
            byte_len: new.byte_len,
            mime: new.mime.clone(),
            width: new.width,
            height: new.height,
            manifest: manifest.clone(),
            created_by: new.created_by.clone(),
        };
        let payload = serde_json::json!({
            "crop_id": crop_id,
            "profile_id": new.profile_id,
            "profile_version": profile_version,
            "character_internal_id": character_internal_id,
            "source_ref": new.source_ref,
            "crop_box": new.crop_box,
            "landmark_count": new.landmarks.len(),
            "artifact_ref": new.artifact_ref,
            "manifest_ref": new.manifest_ref,
            "content_hash": new.content_hash,
            "byte_len": new.byte_len,
            "mime": new.mime,
            "width": new.width,
            "height": new.height,
            "manifest": manifest,
            "created_by": new.created_by,
        });
        let aggregate_id = crop_id.to_string();
        let mut attempt = 1;
        let row = loop {
            let row_result: AtelierResult<Option<serde_json::Value>> = self
                .write_with_event(
                    WRITE_IDENTITY_CROP_STATEMENT,
                    bindings.clone(),
                    IDENTITY_CROP_ARTIFACT_RECORDED,
                    "atelier_identity_crop_artifact",
                    &aggregate_id,
                    payload.clone(),
                )
                .await;
            match row_result {
                Ok(Some(row)) => break row,
                Ok(None) => {
                    return Err(AtelierError::Internal(
                        "recording identity crop returned no row".to_owned(),
                    ));
                }
                Err(error)
                    if is_identity_crop_unique_conflict(&error)
                        || is_identity_crop_retryable_transaction_conflict(&error) =>
                {
                    match self
                        .find_identity_crop_artifact_by_identity(
                            new.profile_id,
                            profile_version,
                            &new.content_hash,
                        )
                        .await
                    {
                        Ok(Some(existing)) => return Ok(existing),
                        Ok(None) => {}
                        Err(read_error)
                            if is_identity_crop_retryable_transaction_conflict(&read_error) => {}
                        Err(read_error) => {
                            return Err(AtelierError::Internal(format!(
                                "recording identity crop failed: {error}; canonical idempotency reread also failed: {read_error}"
                            )));
                        }
                    }
                    if attempt >= IDENTITY_CROP_TRANSACTION_MAX_ATTEMPTS {
                        return Err(error);
                    }
                    wait_before_identity_crop_transaction_retry(crop_id, attempt).await;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        };
        identity_crop_artifact_from_row(&pose_row(row)?)
    }

    /// Fetch one identity crop artifact by id.
    pub async fn get_identity_crop_artifact(
        &self,
        crop_id: Uuid,
    ) -> AtelierResult<Option<IdentityCropArtifact>> {
        let binding = PoseRecordBinding {
            record_id: RecordId::new("atelier_identity_crop_artifact", SurrealUuid::from(crop_id)),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_IDENTITY_CROP_STATEMENT, binding).await })
            })
            .await?;
        row.map(|row| identity_crop_artifact_from_row(&pose_row(row)?))
            .transpose()
    }

    /// List identity crop artifacts for one profile in creation order.
    pub async fn list_identity_crop_artifacts(
        &self,
        profile_id: Uuid,
    ) -> AtelierResult<Vec<IdentityCropArtifact>> {
        let binding = PoseRecordBinding {
            record_id: RecordId::new("atelier_identity_profile", SurrealUuid::from(profile_id)),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_IDENTITY_CROPS_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(|row| identity_crop_artifact_from_row(&pose_row(row)?))
            .collect()
    }

    /// List a character's identity profiles in version order (ascending seq).
    pub async fn list_identity_profiles(
        &self,
        character_internal_id: Uuid,
        kind: Option<IdentityProfileKind>,
    ) -> AtelierResult<Vec<IdentityProfile>> {
        let bindings = IdentityProfileListBindings {
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(character_internal_id),
            ),
            kind: kind.map(|value| value.as_token().to_owned()),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_IDENTITY_PROFILES_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(|row| identity_profile_from_row(&pose_row(row)?))
            .collect()
    }

    /// The latest (highest-seq) identity profile of a kind for a character.
    pub async fn latest_identity_profile(
        &self,
        character_internal_id: Uuid,
        kind: IdentityProfileKind,
    ) -> AtelierResult<Option<IdentityProfile>> {
        let binding = IdentityProfileKindBinding {
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(character_internal_id),
            ),
            kind: kind.as_token().to_owned(),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(LATEST_IDENTITY_PROFILE_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        row.map(|row| identity_profile_from_row(&pose_row(row)?))
            .transpose()
    }
}

// ---------------------------------------------------------------------------
// Pose deferred-feature registry (WP-KERNEL-005 MT-115 / MT-116 / MT-117).
//
// A typed runtime surface that records pose-workspace and Pose-tab features the
// CKC WP-0133 parity work intentionally does NOT implement yet. Each deferred or
// blocked feature is a real embedded SurrealDB row + EventLedger event with a mandatory
// machine-readable reason, so "deferred/blocked" can never be a false parity
// claim hidden in governance prose. Detection/render still happens out of module;
// this only records the deferral decision.
// ---------------------------------------------------------------------------

/// Lifecycle of a recorded pose feature that is intentionally not built yet.
///
/// * `Planned`  -- design is understood; carried forward for a future WP.
/// * `Deferred` -- explicitly postponed; carried forward without early build.
/// * `Blocked`  -- cannot be implemented now (missing capability / spec gate);
///   preserved as unresolved instead of faked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PoseDeferredStatus {
    Planned,
    Deferred,
    Blocked,
}

impl PoseDeferredStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            PoseDeferredStatus::Planned => "PLANNED",
            PoseDeferredStatus::Deferred => "DEFERRED",
            PoseDeferredStatus::Blocked => "BLOCKED",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "PLANNED" => Ok(PoseDeferredStatus::Planned),
            "DEFERRED" => Ok(PoseDeferredStatus::Deferred),
            "BLOCKED" => Ok(PoseDeferredStatus::Blocked),
            other => Err(AtelierError::Validation(format!(
                "unknown pose deferred status token: {other}"
            ))),
        }
    }
}

/// Input to record one deferred/blocked pose feature.
#[derive(Clone, Debug)]
pub struct NewPoseDeferredFeature {
    /// Stable kebab-case feature id, unique in the registry (the PK).
    pub feature_id: String,
    /// Grouping kind (e.g. the originating MT / parity area).
    pub feature_kind: String,
    pub status: PoseDeferredStatus,
    /// Human-readable feature name from the contract.
    pub feature_label: String,
    /// Why the feature is deferred/blocked. MUST be non-empty.
    pub deferral_reason: String,
    /// Whether this is carried forward to a future work packet.
    pub carry_forward: bool,
    /// Optional source/spec anchor (no .GOV / no local path).
    pub source_ref: Option<String>,
}

/// Persisted deferred/blocked pose feature record.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoseDeferredFeature {
    pub feature_id: String,
    pub feature_kind: String,
    pub status: PoseDeferredStatus,
    pub feature_label: String,
    pub deferral_reason: String,
    pub carry_forward: bool,
    pub source_ref: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, SurrealValue)]
struct PoseDeferredFeatureBindings {
    record_id: RecordId,
    feature_id: String,
    feature_kind: String,
    status: String,
    feature_label: String,
    deferral_reason: String,
    carry_forward: bool,
    source_ref: Option<String>,
}

#[derive(SurrealValue)]
struct PoseFeatureKindBinding {
    feature_kind: String,
}

#[derive(SurrealValue)]
struct PoseEmptyBindings {}

macro_rules! pose_deferred_feature_columns {
    () => {
        "feature_id, feature_kind, status, feature_label, deferral_reason, \
         carry_forward, source_ref, created_at_utc"
    };
}

const WRITE_POSE_DEFERRED_FEATURE_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " UPSERT $domain.record_id MERGE { feature_id: $domain.feature_id, \
       feature_kind: $domain.feature_kind, status: $domain.status, \
       feature_label: $domain.feature_label, deferral_reason: $domain.deferral_reason, \
       carry_forward: $domain.carry_forward, source_ref: $domain.source_ref }; \
       RETURN (SELECT ",
    pose_deferred_feature_columns!(),
    " FROM ONLY $domain.record_id); };"
);

const LIST_POSE_DEFERRED_FEATURES_STATEMENT: &str = concat!(
    "SELECT ",
    pose_deferred_feature_columns!(),
    " FROM atelier_pose_deferred_feature ORDER BY feature_id ASC;"
);

const LIST_POSE_DEFERRED_FEATURES_BY_KIND_STATEMENT: &str = concat!(
    "SELECT ",
    pose_deferred_feature_columns!(),
    " FROM atelier_pose_deferred_feature WHERE feature_kind = $feature_kind \
     ORDER BY feature_id ASC;"
);

fn pose_deferred_feature_from_row(row: &PoseRow) -> AtelierResult<PoseDeferredFeature> {
    let status: String = row.get("status");
    Ok(PoseDeferredFeature {
        feature_id: row.get("feature_id"),
        feature_kind: row.get("feature_kind"),
        status: PoseDeferredStatus::from_token(&status)?,
        feature_label: row.get("feature_label"),
        deferral_reason: row.get("deferral_reason"),
        carry_forward: row.get("carry_forward"),
        source_ref: row.get("source_ref"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn validate_pose_deferred_feature(new: &NewPoseDeferredFeature) -> AtelierResult<()> {
    for (field, value) in [
        ("feature_id", &new.feature_id),
        ("feature_kind", &new.feature_kind),
        ("feature_label", &new.feature_label),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(AtelierError::Validation(format!(
                "pose deferred feature {field} must be non-empty and unpadded"
            )));
        }
    }
    // Hard gate: a blank deferral_reason is rejected so a deferral is never silent.
    if new.deferral_reason.trim().is_empty() || new.deferral_reason.trim() != new.deferral_reason {
        return Err(AtelierError::Validation(
            "pose deferred feature deferral_reason must be non-empty and unpadded".into(),
        ));
    }
    if let Some(source_ref) = &new.source_ref {
        validate_pose_context_ref("source_ref", source_ref)?;
    }
    Ok(())
}

/// Catalog of every deferred/blocked pose feature recorded by MT-115/116/117.
///
/// Const-style data (mirrors `source_evidence::core_data_source_evidence_matrix`):
/// the records carry the real feature names + reasons from the MT contracts. A
/// test persists this catalog and reloads it to prove the runtime surface.
pub fn pose_deferred_feature_catalog() -> Vec<NewPoseDeferredFeature> {
    vec![
        // ---- MT-115: five non-calibration WP-0133 pose-workspace items, BLOCKED.
        NewPoseDeferredFeature {
            feature_id: "mt-115.pose-workspace.draggable-overlay".to_string(),
            feature_kind: "MT-115.pose-workspace-blocked".to_string(),
            status: PoseDeferredStatus::Blocked,
            feature_label: "Draggable keypoint overlay".to_string(),
            deferral_reason:
                "CKC WP-0133 draggable overlay editing is not implementable yet; preserved as \
                 blocked so no false parity claim is made for interactive overlay drag."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/workspace/draggable-overlay".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-115.pose-workspace.missing-marker-placement".to_string(),
            feature_kind: "MT-115.pose-workspace-blocked".to_string(),
            status: PoseDeferredStatus::Blocked,
            feature_label: "Missing-marker placement".to_string(),
            deferral_reason:
                "CKC WP-0133 missing-marker placement is not implementable yet; preserved as \
                 blocked rather than fabricating placed markers."
                    .to_string(),
            carry_forward: true,
            source_ref: Some(
                "source://legacy/posekit/workspace/missing-marker-placement".to_string(),
            ),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-115.pose-workspace.3d-live-split".to_string(),
            feature_kind: "MT-115.pose-workspace-blocked".to_string(),
            status: PoseDeferredStatus::Blocked,
            feature_label: "3D / live split view".to_string(),
            deferral_reason:
                "CKC WP-0133 3D/live split view is not implementable yet; preserved as blocked \
                 so the split-view parity gap stays visible."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/workspace/3d-live-split".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-115.pose-workspace.forked-history".to_string(),
            feature_kind: "MT-115.pose-workspace-blocked".to_string(),
            status: PoseDeferredStatus::Blocked,
            feature_label: "Forked history".to_string(),
            deferral_reason:
                "CKC WP-0133 forked (branching) history is not implementable yet; preserved as \
                 blocked so branch/merge history parity is not falsely claimed."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/workspace/forked-history".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-115.pose-workspace.history-tab".to_string(),
            feature_kind: "MT-115.pose-workspace-blocked".to_string(),
            status: PoseDeferredStatus::Blocked,
            feature_label: "History tab".to_string(),
            deferral_reason:
                "CKC WP-0133 History tab UI is not implementable yet; preserved as blocked so the \
                 history-tab parity gap stays explicit."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/workspace/history-tab".to_string()),
        },
        // ---- MT-116: RigData v2 multi-subject, carry-forward DEFERRED.
        NewPoseDeferredFeature {
            feature_id: "mt-116.rigdata-v2.multi-subject".to_string(),
            feature_kind: "MT-116.multi-subject-rig-carry-forward".to_string(),
            status: PoseDeferredStatus::Deferred,
            feature_label:
                "RigData v2 multi-subject (people[], per-subject calibration/head-pose/masks)"
                    .to_string(),
            deferral_reason:
                "Planned multi-subject scenes (RigData v2 people[] with per-subject calibration, \
                 head pose, and masks) are deferred and carried forward; not implemented early to \
                 avoid premature multi-subject schema commitments."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/rigdata-v2/multi-subject".to_string()),
        },
        // ---- MT-117: Pose tab polish features, PLANNED/RESEARCH deferred.
        NewPoseDeferredFeature {
            feature_id: "mt-117.pose-tab-polish.multi-file-dnd".to_string(),
            feature_kind: "MT-117.pose-tab-polish-carry-forward".to_string(),
            status: PoseDeferredStatus::Planned,
            feature_label: "Multi-file drag-and-drop".to_string(),
            deferral_reason:
                "Pose tab polish: multi-file drag-and-drop import is PLANNED/RESEARCH deferred; \
                 carried forward without early build."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/pose-tab/multi-file-dnd".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-117.pose-tab-polish.multi-angle-export".to_string(),
            feature_kind: "MT-117.pose-tab-polish-carry-forward".to_string(),
            status: PoseDeferredStatus::Planned,
            feature_label: "Multi-angle export".to_string(),
            deferral_reason:
                "Pose tab polish: multi-angle export is PLANNED/RESEARCH deferred; carried forward \
                 without early build."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/pose-tab/multi-angle-export".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-117.pose-tab-polish.clear-workspace".to_string(),
            feature_kind: "MT-117.pose-tab-polish-carry-forward".to_string(),
            status: PoseDeferredStatus::Planned,
            feature_label: "Clear workspace".to_string(),
            deferral_reason:
                "Pose tab polish: clear-workspace action is PLANNED/RESEARCH deferred; carried \
                 forward without early build."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/pose-tab/clear-workspace".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-117.pose-tab-polish.sync-zoom".to_string(),
            feature_kind: "MT-117.pose-tab-polish-carry-forward".to_string(),
            status: PoseDeferredStatus::Planned,
            feature_label: "Sync zoom".to_string(),
            deferral_reason:
                "Pose tab polish: synchronized zoom across views is PLANNED/RESEARCH deferred; \
                 carried forward without early build."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/pose-tab/sync-zoom".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-117.pose-tab-polish.import-openpose-json".to_string(),
            feature_kind: "MT-117.pose-tab-polish-carry-forward".to_string(),
            status: PoseDeferredStatus::Planned,
            feature_label: "Import OpenPose JSON".to_string(),
            deferral_reason:
                "Pose tab polish: import OpenPose JSON into the workspace is PLANNED/RESEARCH \
                 deferred; carried forward without early build."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/pose-tab/import-openpose-json".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-117.pose-tab-polish.shortcuts".to_string(),
            feature_kind: "MT-117.pose-tab-polish-carry-forward".to_string(),
            status: PoseDeferredStatus::Planned,
            feature_label: "Keyboard shortcuts".to_string(),
            deferral_reason:
                "Pose tab polish: keyboard shortcuts are PLANNED/RESEARCH deferred; carried \
                 forward without early build."
                    .to_string(),
            carry_forward: true,
            source_ref: Some("source://legacy/posekit/pose-tab/shortcuts".to_string()),
        },
        NewPoseDeferredFeature {
            feature_id: "mt-117.pose-tab-polish.stylized-landmark-router".to_string(),
            feature_kind: "MT-117.pose-tab-polish-carry-forward".to_string(),
            status: PoseDeferredStatus::Planned,
            feature_label: "Stylized-landmark router".to_string(),
            deferral_reason:
                "Pose tab polish: stylized-landmark router is PLANNED/RESEARCH deferred; carried \
                 forward without early build."
                    .to_string(),
            carry_forward: true,
            source_ref: Some(
                "source://legacy/posekit/pose-tab/stylized-landmark-router".to_string(),
            ),
        },
    ]
}

impl AtelierStore {
    /// Record one deferred/blocked pose feature (MT-115/116/117).
    ///
    /// Idempotent on `feature_id`: re-recording the same feature updates the
    /// mutable fields and returns the row instead of erroring, so seeding the
    /// catalog twice is safe. A blank `deferral_reason` is rejected with a
    /// `Validation` error so a deferral is never silent. Emits
    /// `POSE_DEFERRED_FEATURE_RECORDED`.
    pub async fn record_pose_deferred_feature(
        &self,
        new: &NewPoseDeferredFeature,
    ) -> AtelierResult<PoseDeferredFeature> {
        validate_pose_deferred_feature(new)?;

        let bindings = PoseDeferredFeatureBindings {
            record_id: RecordId::new("atelier_pose_deferred_feature", new.feature_id.clone()),
            feature_id: new.feature_id.clone(),
            feature_kind: new.feature_kind.clone(),
            status: new.status.as_token().to_owned(),
            feature_label: new.feature_label.clone(),
            deferral_reason: new.deferral_reason.clone(),
            carry_forward: new.carry_forward,
            source_ref: new.source_ref.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_POSE_DEFERRED_FEATURE_STATEMENT,
                bindings,
                POSE_DEFERRED_FEATURE_RECORDED,
                "atelier_pose_deferred_feature",
                &new.feature_id,
                serde_json::json!({
                    "feature_id": new.feature_id,
                    "feature_kind": new.feature_kind,
                    "status": new.status.as_token(),
                    "feature_label": new.feature_label,
                    "deferral_reason": new.deferral_reason,
                    "carry_forward": new.carry_forward,
                    "source_ref": new.source_ref,
                }),
            )
            .await?;
        let row = row.ok_or_else(|| {
            AtelierError::Internal("recording pose deferred feature returned no row".to_owned())
        })?;
        pose_deferred_feature_from_row(&pose_row(row)?)
    }

    /// List every recorded deferred/blocked pose feature, by `feature_id`.
    pub async fn list_pose_deferred_features(&self) -> AtelierResult<Vec<PoseDeferredFeature>> {
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_POSE_DEFERRED_FEATURES_STATEMENT, PoseEmptyBindings {})
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(|row| pose_deferred_feature_from_row(&pose_row(row)?))
            .collect()
    }

    /// List recorded deferred/blocked pose features for one `feature_kind`.
    pub async fn list_pose_deferred_features_by_kind(
        &self,
        feature_kind: &str,
    ) -> AtelierResult<Vec<PoseDeferredFeature>> {
        let binding = PoseFeatureKindBinding {
            feature_kind: feature_kind.to_owned(),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_POSE_DEFERRED_FEATURES_BY_KIND_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(|row| pose_deferred_feature_from_row(&pose_row(row)?))
            .collect()
    }
}

#[cfg(test)]
mod redaction_tests {
    use super::*;

    #[test]
    fn redacts_secret_keyed_lines() {
        let raw = "source: civitai\ncookie: abc123\nAuthorization: Bearer xyz\nnote: ok";
        let red = redact_secrets(raw);
        assert!(red.contains("source: civitai"));
        assert!(red.contains("note: ok"));
        assert!(!red.contains("abc123"));
        assert!(!red.contains("xyz"));
        assert!(red.contains("[REDACTED]"));
    }

    #[test]
    fn validates_keypoint_cardinality() {
        let ok = serde_json::json!({
            "people": [{
                "pose_keypoints_2d": vec![0.0; BODY_KEYPOINT_COUNT * 3],
                "face_keypoints_2d": vec![0.0; FACE_KEYPOINT_COUNT * 3],
                "hand_left_keypoints_2d": vec![0.0; HAND_KEYPOINT_COUNT * 3],
                "hand_right_keypoints_2d": vec![0.0; HAND_KEYPOINT_COUNT * 3],
            }]
        });
        assert!(validate_keypoints(&ok).is_ok());

        // Absent face/hands are allowed (legacy source zero-fills, so optional here).
        let body_only = serde_json::json!({
            "people": [{ "pose_keypoints_2d": vec![0.0; BODY_KEYPOINT_COUNT * 3] }]
        });
        assert!(validate_keypoints(&body_only).is_ok());

        // Wrong body cardinality is rejected.
        let bad = serde_json::json!({
            "people": [{ "pose_keypoints_2d": vec![0.0; 10] }]
        });
        assert!(validate_keypoints(&bad).is_err());
    }
}
