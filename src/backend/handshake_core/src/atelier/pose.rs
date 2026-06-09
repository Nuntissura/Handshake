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
//! never faked). Storage authority is PostgreSQL + EventLedger only (MT-004).
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
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{
    event_ref_for_text, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
};

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
    ];
}

/// Re-export at module root so callers can write `pose::POSE_RIG_INGESTED`.
pub use pose_event_family::{
    IDENTITY_CROP_ARTIFACT_RECORDED, IDENTITY_PROFILE_APPENDED, POSE_CALIBRATION_SET,
    POSE_CONTEXT_STATE_SET, POSE_HEAD_POSE_RECORDED, POSE_RIG_INGESTED, POSE_SIDECAR_RECORDED,
    POSE_WORKSPACE_RIG_STATE_SET,
};

pub const IDENTITY_CROP_ARTIFACT_MANIFEST_SCHEMA: &str =
    "hsk.atelier.identity_crop_artifact_manifest@1";

/// OpenPose keypoint-array cardinalities (legacy source `rigToOpenposeJson`):
/// body-18, face-70, hand-21. Each keypoint is an `(x, y, confidence)` triple,
/// so flattened arrays have `count * 3` numbers.
pub const BODY_KEYPOINT_COUNT: usize = 18;
pub const FACE_KEYPOINT_COUNT: usize = 70;
pub const HAND_KEYPOINT_COUNT: usize = 21;

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
/// read from PostgreSQL and stored on the crop record.
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
    Ok(())
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

fn context_state_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<PoseContextState> {
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

fn workspace_rig_state_from_row(row: &sqlx::postgres::PgRow) -> PoseWorkspaceRigState {
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

fn rig_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<PoseRig> {
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

fn sidecar_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<PoseSidecar> {
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

fn source_image_strip_item_from_row(row: &sqlx::postgres::PgRow) -> PoseSourceImageStripItem {
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

fn head_pose_from_row(row: &sqlx::postgres::PgRow) -> HeadPose {
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

fn calibration_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<Calibration> {
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

fn identity_profile_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<IdentityProfile> {
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

fn identity_crop_artifact_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<IdentityCropArtifact> {
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

const RIG_COLUMNS: &str = "rig_id, character_internal_id, source_asset_id, source_ref, \
                           content_hash, canvas_width, canvas_height, detector_provider, \
                           detector_model, detector_model_version, source_asset_version_ref, \
                           source_asset_path_ref, confidence_available, detector_status, \
                           error_reason, keypoints_json, sidecar_ref, created_at_utc";

const CONTEXT_STATE_COLUMNS: &str = "context_id, state_seq, workspace_ref, kind, source_asset_id, \
                                     character_internal_id, collection_id, selected_rig_id, \
                                     requested_by, created_at_utc";

const WORKSPACE_RIG_STATE_COLUMNS: &str =
    "state.workspace_ref, state.session_ref, state.rig_id, rig.character_internal_id, rig.source_asset_id, \
     rig.source_ref, state.open, state.sort_order, state.active, state.dirty_calibration, \
     state.panel_state, state.requested_by, state.created_at_utc, state.updated_at_utc";

const SIDECAR_COLUMNS: &str = "sidecar_id, rig_id, source_asset_id, source_ref, kind, role, \
                               artifact_ref, manifest_ref, content_hash, byte_len, mime, width, height, \
                               status, error_message, created_at_utc";

const PROFILE_COLUMNS: &str = "profile_id, character_internal_id, seq, version, kind, name, description, \
                               reference_asset_id, reference_ref, source_ref, crop_ref, artifact_ref, \
                               provenance, created_at_utc, updated_at_utc";

const IDENTITY_CROP_COLUMNS: &str = "crop_id, profile_id, profile_version, character_internal_id, \
                                     source_ref, crop_box, landmarks, artifact_ref, manifest_ref, \
                                     content_hash, byte_len, mime, width, height, manifest, \
                                     created_by, created_at_utc";

const CALIBRATION_COLUMNS: &str =
    "rig_id, state, block_reason, head_pose_ref, marker_visibility, marker_colors, \
     hand_rows, history_refs, created_at_utc, updated_at_utc";

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

        let mut tx = self.pool().begin().await?;

        let character_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT internal_id FROM atelier_character WHERE internal_id = $1")
                .bind(new.character_internal_id)
                .fetch_optional(&mut *tx)
                .await?;
        if character_exists.is_none() {
            return Err(AtelierError::NotFound(format!(
                "atelier_character internal_id={}",
                new.character_internal_id
            )));
        }

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_pose_rig
                (character_internal_id, source_asset_id, source_ref, content_hash,
                  canvas_width, canvas_height, detector_provider, detector_status,
                  detector_model, detector_model_version, source_asset_version_ref,
                  source_asset_path_ref, confidence_available, error_reason, keypoints_json, sidecar_ref)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
               ON CONFLICT (character_internal_id, source_ref, content_hash)
                 DO UPDATE SET source_ref = EXCLUDED.source_ref
               RETURNING {RIG_COLUMNS}"#
        ))
        .bind(new.character_internal_id)
        .bind(new.source_asset_id)
        .bind(&new.source_ref)
        .bind(&new.content_hash)
        .bind(new.canvas.width)
        .bind(new.canvas.height)
        .bind(&new.detector_provider)
        .bind(new.detector_status.as_token())
        .bind(&new.detector_model)
        .bind(&new.detector_model_version)
        .bind(&new.source_asset_version_ref)
        .bind(&new.source_asset_path_ref)
        .bind(new.confidence_available)
        .bind(&new.error_reason)
        .bind(&new.keypoints_json)
        .bind(&new.sidecar_ref)
        .fetch_one(&mut *tx)
        .await?;

        let rig = rig_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            POSE_RIG_INGESTED,
            "atelier_pose_rig",
            &rig.rig_id.to_string(),
            serde_json::json!({
                "rig_id": rig.rig_id,
                "source_asset_id": rig.source_asset_id,
                "source_ref": rig.source_ref,
                "content_hash": rig.content_hash,
                "detector_provider": rig.detector_provider,
                "detector_model": rig.detector_model,
                "detector_model_version": rig.detector_model_version,
                "source_asset_version_ref": rig.source_asset_version_ref,
                "source_asset_path_ref": rig.source_asset_path_ref,
                "confidence_available": rig.confidence_available,
                "detector_status": rig.detector_status.as_token(),
                "error_reason": rig.error_reason,
                "canvas_width": rig.canvas.width,
                "canvas_height": rig.canvas.height,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(rig)
    }

    /// Fetch a pose rig by id.
    pub async fn get_pose_rig(&self, rig_id: Uuid) -> AtelierResult<PoseRig> {
        let row = sqlx::query(&format!(
            "SELECT {RIG_COLUMNS} FROM atelier_pose_rig WHERE rig_id = $1"
        ))
        .bind(rig_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("pose rig_id={rig_id}")))?;
        rig_from_row(&row)
    }

    /// List a character's pose rigs, newest first.
    pub async fn list_pose_rigs(
        &self,
        character_internal_id: Uuid,
        limit: i64,
    ) -> AtelierResult<Vec<PoseRig>> {
        let capped = limit.clamp(1, 1000);
        let rows = sqlx::query(&format!(
            r#"SELECT {RIG_COLUMNS} FROM atelier_pose_rig
               WHERE character_internal_id = $1
               ORDER BY created_at_utc DESC
               LIMIT $2"#
        ))
        .bind(character_internal_id)
        .bind(capped)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(rig_from_row).collect()
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
            let exists: Option<Uuid> =
                sqlx::query_scalar("SELECT asset_id FROM atelier_media_asset WHERE asset_id = $1")
                    .bind(source_asset_id)
                    .fetch_optional(self.pool())
                    .await?;
            if exists.is_none() {
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
            let exists: Option<Uuid> = sqlx::query_scalar(
                "SELECT internal_id FROM atelier_character WHERE internal_id = $1",
            )
            .bind(character_internal_id)
            .fetch_optional(self.pool())
            .await?;
            if exists.is_none() {
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

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_pose_context_state
                 (workspace_ref, kind, source_asset_id, character_internal_id,
                  collection_id, selected_rig_id, requested_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING {CONTEXT_STATE_COLUMNS}"#
        ))
        .bind(&new.workspace_ref)
        .bind(new.kind.as_token())
        .bind(new.source_asset_id)
        .bind(new.character_internal_id)
        .bind(new.collection_id)
        .bind(new.selected_rig_id)
        .bind(&new.requested_by)
        .fetch_one(&mut *tx)
        .await?;

        let state = context_state_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            POSE_CONTEXT_STATE_SET,
            "atelier_pose_context_state",
            &state.context_id.to_string(),
            serde_json::json!({
                "context_id": state.context_id,
                "workspace_ref": state.workspace_ref,
                "kind": state.kind.as_token(),
                "source_asset_id": state.source_asset_id,
                "character_internal_id": state.character_internal_id,
                "collection_id": state.collection_id,
                "selected_rig_id": state.selected_rig_id,
                "requested_by": state.requested_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(state)
    }

    /// Read the latest pose context state for a workspace.
    pub async fn current_pose_context_state(
        &self,
        workspace_ref: &str,
    ) -> AtelierResult<Option<PoseContextState>> {
        validate_pose_context_ref("workspace_ref", workspace_ref)?;
        let row = sqlx::query(&format!(
            r#"SELECT {CONTEXT_STATE_COLUMNS}
               FROM atelier_pose_context_state
               WHERE workspace_ref = $1
               ORDER BY state_seq DESC
               LIMIT 1"#
        ))
        .bind(workspace_ref)
        .fetch_optional(self.pool())
        .await?;
        row.as_ref().map(context_state_from_row).transpose()
    }

    /// List pose context state history oldest first for deterministic replay.
    pub async fn list_pose_context_history(
        &self,
        workspace_ref: &str,
    ) -> AtelierResult<Vec<PoseContextState>> {
        validate_pose_context_ref("workspace_ref", workspace_ref)?;
        let rows = sqlx::query(&format!(
            r#"SELECT {CONTEXT_STATE_COLUMNS}
               FROM atelier_pose_context_state
               WHERE workspace_ref = $1
               ORDER BY state_seq ASC"#
        ))
        .bind(workspace_ref)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(context_state_from_row).collect()
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

        let mut tx = self.pool().begin().await?;
        let lock_key = format!("{}:{}", new.workspace_ref, new.session_ref);
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 960096))")
            .bind(&lock_key)
            .execute(&mut *tx)
            .await?;

        if new.open {
            let duplicate_sort_order: Option<Uuid> = sqlx::query_scalar(
                r#"SELECT rig_id
                   FROM atelier_pose_workspace_rig_state
                   WHERE workspace_ref = $1
                     AND session_ref = $2
                     AND open = TRUE
                     AND rig_id <> $3
                     AND sort_order = $4
                   LIMIT 1"#,
            )
            .bind(&new.workspace_ref)
            .bind(&new.session_ref)
            .bind(new.rig_id)
            .bind(new.sort_order)
            .fetch_optional(&mut *tx)
            .await?;
            if duplicate_sort_order.is_some() {
                return Err(AtelierError::Validation(
                    "pose workspace sort_order must be unique among open rigs".into(),
                ));
            }
        }

        if new.active {
            let deactivated_rows = sqlx::query(&format!(
                r#"WITH deactivated AS (
                       UPDATE atelier_pose_workspace_rig_state
                       SET active = FALSE,
                           requested_by = $4,
                           updated_at_utc = NOW()
                       WHERE workspace_ref = $1
                         AND session_ref = $2
                         AND open = TRUE
                         AND active = TRUE
                         AND rig_id <> $3
                       RETURNING workspace_ref, session_ref, rig_id, open,
                                 sort_order, active, dirty_calibration,
                                 panel_state, requested_by,
                                 created_at_utc, updated_at_utc
                   )
                   SELECT {WORKSPACE_RIG_STATE_COLUMNS}
                   FROM deactivated state
                   JOIN atelier_pose_rig rig ON rig.rig_id = state.rig_id"#
            ))
            .bind(&new.workspace_ref)
            .bind(&new.session_ref)
            .bind(new.rig_id)
            .bind(&new.requested_by)
            .fetch_all(&mut *tx)
            .await?;
            for row in deactivated_rows {
                let state = workspace_rig_state_from_row(&row);
                let aggregate_id = format!(
                    "{}:{}:{}",
                    state.workspace_ref, state.session_ref, state.rig_id
                );
                self.record_event_in_tx(
                    &mut tx,
                    POSE_WORKSPACE_RIG_STATE_SET,
                    "atelier_pose_workspace_rig_state",
                    &aggregate_id,
                    serde_json::json!({
                        "workspace_ref": state.workspace_ref,
                        "session_ref": state.session_ref,
                        "rig_id": state.rig_id,
                        "character_internal_id": state.character_internal_id,
                        "open": state.open,
                        "sort_order": state.sort_order,
                        "active": state.active,
                        "dirty_calibration": state.dirty_calibration,
                        "panel_state_is_object": state.panel_state.is_object(),
                        "requested_by": state.requested_by,
                        "reason": "deactivated_by_active_rig_switch",
                    }),
                )
                .await?;
            }
        }

        let row = sqlx::query(&format!(
            r#"WITH upserted AS (
                   INSERT INTO atelier_pose_workspace_rig_state
                     (workspace_ref, session_ref, rig_id, open, sort_order, active,
                      dirty_calibration, panel_state, requested_by)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                   ON CONFLICT (workspace_ref, session_ref, rig_id) DO UPDATE SET
                     open = EXCLUDED.open,
                     sort_order = EXCLUDED.sort_order,
                     active = EXCLUDED.active,
                     dirty_calibration = EXCLUDED.dirty_calibration,
                     panel_state = EXCLUDED.panel_state,
                     requested_by = EXCLUDED.requested_by,
                     updated_at_utc = NOW()
                   RETURNING workspace_ref, session_ref, rig_id, open, sort_order, active,
                             dirty_calibration, panel_state, requested_by,
                             created_at_utc, updated_at_utc
               )
               SELECT {WORKSPACE_RIG_STATE_COLUMNS}
               FROM upserted state
               JOIN atelier_pose_rig rig ON rig.rig_id = state.rig_id"#
        ))
        .bind(&new.workspace_ref)
        .bind(&new.session_ref)
        .bind(new.rig_id)
        .bind(new.open)
        .bind(new.sort_order)
        .bind(new.active)
        .bind(new.dirty_calibration)
        .bind(&new.panel_state)
        .bind(&new.requested_by)
        .fetch_one(&mut *tx)
        .await?;

        let state = workspace_rig_state_from_row(&row);
        let aggregate_id = format!(
            "{}:{}:{}",
            state.workspace_ref, state.session_ref, state.rig_id
        );
        self.record_event_in_tx(
            &mut tx,
            POSE_WORKSPACE_RIG_STATE_SET,
            "atelier_pose_workspace_rig_state",
            &aggregate_id,
            serde_json::json!({
                "workspace_ref": state.workspace_ref,
                "session_ref": state.session_ref,
                "rig_id": state.rig_id,
                "character_internal_id": state.character_internal_id,
                "open": state.open,
                "sort_order": state.sort_order,
                "active": state.active,
                "dirty_calibration": state.dirty_calibration,
                "panel_state_is_object": state.panel_state.is_object(),
                "requested_by": state.requested_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(state)
    }

    /// List open rig tab state for a workspace in deterministic tab order.
    pub async fn list_pose_workspace_rig_state(
        &self,
        workspace_ref: &str,
        session_ref: &str,
    ) -> AtelierResult<Vec<PoseWorkspaceRigState>> {
        validate_pose_context_ref("workspace_ref", workspace_ref)?;
        validate_pose_context_ref("session_ref", session_ref)?;
        let rows = sqlx::query(&format!(
            r#"SELECT {WORKSPACE_RIG_STATE_COLUMNS}
               FROM atelier_pose_workspace_rig_state state
               JOIN atelier_pose_rig rig ON rig.rig_id = state.rig_id
               WHERE state.workspace_ref = $1
                 AND state.session_ref = $2
                 AND state.open = TRUE
               ORDER BY state.sort_order ASC, state.rig_id ASC"#
        ))
        .bind(workspace_ref)
        .bind(session_ref)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(workspace_rig_state_from_row).collect())
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

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_pose_sidecar
                 (rig_id, source_asset_id, source_ref, kind, role, artifact_ref,
                  manifest_ref, content_hash, byte_len, mime, width, height, status, error_message)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
               ON CONFLICT (rig_id, kind) DO UPDATE SET
                 source_asset_id = EXCLUDED.source_asset_id,
                 source_ref = EXCLUDED.source_ref,
                 role = EXCLUDED.role,
                 artifact_ref = EXCLUDED.artifact_ref,
                 manifest_ref = EXCLUDED.manifest_ref,
                 content_hash = EXCLUDED.content_hash,
                 byte_len = EXCLUDED.byte_len,
                 mime = EXCLUDED.mime,
                 width = EXCLUDED.width,
                 height = EXCLUDED.height,
                 status = EXCLUDED.status,
                 error_message = EXCLUDED.error_message,
                 created_at_utc = NOW()
               RETURNING {SIDECAR_COLUMNS}"#
        ))
        .bind(new.rig_id)
        .bind(rig.source_asset_id)
        .bind(&rig.source_ref)
        .bind(new.kind.as_token())
        .bind(new.kind.as_token())
        .bind(&new.artifact_ref)
        .bind(&new.manifest_ref)
        .bind(&new.content_hash)
        .bind(new.byte_len)
        .bind(&new.mime)
        .bind(new.width)
        .bind(new.height)
        .bind(new.status.as_token())
        .bind(&new.error_message)
        .fetch_one(&mut *tx)
        .await?;

        let sidecar = sidecar_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            POSE_SIDECAR_RECORDED,
            "atelier_pose_sidecar",
            &sidecar.sidecar_id.to_string(),
            serde_json::json!({
                "sidecar_id": sidecar.sidecar_id,
                "rig_id": sidecar.rig_id,
                "source_asset_id": sidecar.source_asset_id,
                "source_ref": sidecar.source_ref,
                "kind": sidecar.kind.as_token(),
                "role": sidecar.role,
                "artifact_ref": sidecar.artifact_ref,
                "manifest_ref": sidecar.manifest_ref,
                "content_hash": sidecar.content_hash,
                "byte_len": sidecar.byte_len,
                "mime": sidecar.mime,
                "width": sidecar.width,
                "height": sidecar.height,
                "status": sidecar.status.as_token(),
                "has_error_message": sidecar.error_message.is_some(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(sidecar)
    }

    /// List typed sidecars for a rig in deterministic OpenPose/PNG/conditioning order.
    pub async fn list_pose_sidecars(&self, rig_id: Uuid) -> AtelierResult<Vec<PoseSidecar>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {SIDECAR_COLUMNS}
               FROM atelier_pose_sidecar
               WHERE rig_id = $1
               ORDER BY CASE kind
                    WHEN 'openpose_json' THEN 0
                    WHEN 'openpose_png' THEN 1
                    WHEN 'conditioning_png' THEN 2
                    ELSE 99
               END,
               created_at_utc ASC,
               sidecar_id ASC"#
        ))
        .bind(rig_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(sidecar_from_row).collect()
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
        let rows = sqlx::query(&format!(
            r#"SELECT {SIDECAR_COLUMNS}
               FROM atelier_pose_sidecar
               WHERE source_ref = $1
                 AND ($2::UUID IS NULL OR rig_id = $2)
               ORDER BY CASE kind
                    WHEN 'openpose_json' THEN 0
                    WHEN 'openpose_png' THEN 1
                    WHEN 'conditioning_png' THEN 2
                    ELSE 99
               END,
               created_at_utc ASC,
               sidecar_id ASC"#
        ))
        .bind(&source_ref)
        .bind(rig_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(sidecar_from_row).collect()
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
        let rows = sqlx::query(
            r#"SELECT rig.rig_id,
                      rig.character_internal_id,
                      rig.source_asset_id,
                      rig.source_ref,
                      media.artifact_ref AS source_artifact_ref,
                      media.content_hash AS source_content_hash,
                      media.mime AS source_mime,
                      media.byte_len AS source_byte_len,
                      rig.created_at_utc
               FROM atelier_pose_rig rig
               LEFT JOIN atelier_media_asset media
                 ON media.asset_id = rig.source_asset_id
               WHERE rig.character_internal_id = $1
               ORDER BY rig.created_at_utc DESC, rig.rig_id ASC
               LIMIT $2"#,
        )
        .bind(character_internal_id)
        .bind(capped)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(source_image_strip_item_from_row).collect())
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
        let rows = sqlx::query(&format!(
            r#"SELECT {SIDECAR_COLUMNS}
               FROM atelier_pose_sidecar
               WHERE rig_id = $1
                 AND kind IN ('openpose_json','openpose_png')
               ORDER BY CASE kind
                    WHEN 'openpose_json' THEN 0
                    WHEN 'openpose_png' THEN 1
                    ELSE 99
               END,
               created_at_utc ASC,
               sidecar_id ASC"#
        ))
        .bind(rig_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter()
            .map(sidecar_from_row)
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

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_pose_head_pose
                 (rig_id, yaw_deg, pitch_deg, roll_deg, quat_x, quat_y, quat_z, quat_w)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (rig_id) DO UPDATE SET
                 yaw_deg   = EXCLUDED.yaw_deg,
                 pitch_deg = EXCLUDED.pitch_deg,
                 roll_deg  = EXCLUDED.roll_deg,
                 quat_x    = EXCLUDED.quat_x,
                 quat_y    = EXCLUDED.quat_y,
                 quat_z    = EXCLUDED.quat_z,
                 quat_w    = EXCLUDED.quat_w,
                 created_at_utc = NOW()
               RETURNING rig_id, yaw_deg, pitch_deg, roll_deg,
                         quat_x, quat_y, quat_z, quat_w, created_at_utc"#,
        )
        .bind(rig_id)
        .bind(yaw_deg)
        .bind(pitch_deg)
        .bind(roll_deg)
        .bind(q[0])
        .bind(q[1])
        .bind(q[2])
        .bind(q[3])
        .fetch_one(&mut *tx)
        .await?;

        let head_pose = head_pose_from_row(&row);
        self.record_event_in_tx(
            &mut tx,
            POSE_HEAD_POSE_RECORDED,
            "atelier_pose_rig",
            &rig_id.to_string(),
            serde_json::json!({
                "rig_id": head_pose.rig_id,
                "yaw_deg": head_pose.yaw_deg,
                "pitch_deg": head_pose.pitch_deg,
                "roll_deg": head_pose.roll_deg,
                "quaternion": head_pose.quaternion,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(head_pose)
    }

    /// Fetch the head pose for a rig, if recorded.
    pub async fn get_head_pose(&self, rig_id: Uuid) -> AtelierResult<Option<HeadPose>> {
        let row = sqlx::query(
            r#"SELECT rig_id, yaw_deg, pitch_deg, roll_deg,
                      quat_x, quat_y, quat_z, quat_w, created_at_utc
               FROM atelier_pose_head_pose WHERE rig_id = $1"#,
        )
        .bind(rig_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(head_pose_from_row))
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

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_pose_calibration
                 (rig_id, state, block_reason, head_pose_ref, marker_visibility,
                  marker_colors, hand_rows, history_refs)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (rig_id) DO UPDATE SET
                 state         = EXCLUDED.state,
                 block_reason  = EXCLUDED.block_reason,
                 head_pose_ref = EXCLUDED.head_pose_ref,
                 marker_visibility = EXCLUDED.marker_visibility,
                 marker_colors = EXCLUDED.marker_colors,
                 hand_rows = EXCLUDED.hand_rows,
                 history_refs = EXCLUDED.history_refs,
                 updated_at_utc = NOW()
               RETURNING rig_id, state, block_reason, head_pose_ref, marker_visibility,
                         marker_colors, hand_rows, history_refs, created_at_utc, updated_at_utc"#,
        )
        .bind(new.rig_id)
        .bind(new.state.as_token())
        .bind(&new.block_reason)
        .bind(&new.head_pose_ref)
        .bind(marker_visibility)
        .bind(marker_colors)
        .bind(hand_rows)
        .bind(history_refs)
        .fetch_one(&mut *tx)
        .await?;

        let calibration = calibration_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            POSE_CALIBRATION_SET,
            "atelier_pose_rig",
            &new.rig_id.to_string(),
            serde_json::json!({
                "rig_id": calibration.rig_id,
                "state": calibration.state.as_token(),
                "block_reason": calibration.block_reason,
                "head_pose_ref": calibration.head_pose_ref,
                "marker_visibility": calibration.marker_visibility,
                "marker_colors": calibration.marker_colors,
                "hand_rows": calibration.hand_rows,
                "history_refs": calibration.history_refs,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(calibration)
    }

    /// Fetch the calibration record for a rig, if any.
    pub async fn get_calibration(&self, rig_id: Uuid) -> AtelierResult<Option<Calibration>> {
        let row = sqlx::query(&format!(
            "SELECT {CALIBRATION_COLUMNS} FROM atelier_pose_calibration WHERE rig_id = $1"
        ))
        .bind(rig_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(calibration_from_row(&r)?)),
            None => Ok(None),
        }
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

        let mut tx = self.pool().begin().await?;

        let character_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT internal_id FROM atelier_character WHERE internal_id = $1")
                .bind(new.character_internal_id)
                .fetch_optional(&mut *tx)
                .await?;
        if character_exists.is_none() {
            return Err(AtelierError::NotFound(format!(
                "atelier_character internal_id={}",
                new.character_internal_id
            )));
        }

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_identity_profile \
             WHERE character_internal_id = $1",
        )
        .bind(new.character_internal_id)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_identity_profile
                 (character_internal_id, seq, version, kind, name, description,
                  reference_asset_id, reference_ref, source_ref, crop_ref,
                  artifact_ref, provenance)
               VALUES ($1, $2, 1, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               RETURNING {PROFILE_COLUMNS}"#
        ))
        .bind(new.character_internal_id)
        .bind(next_seq)
        .bind(new.kind.as_token())
        .bind(&name)
        .bind(&description)
        .bind(new.reference_asset_id)
        .bind(&new.reference_ref)
        .bind(&new.source_ref)
        .bind(&new.crop_ref)
        .bind(&new.artifact_ref)
        .bind(&provenance)
        .fetch_one(&mut *tx)
        .await?;

        let profile = identity_profile_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            IDENTITY_PROFILE_APPENDED,
            "atelier_identity_profile",
            &profile.profile_id.to_string(),
            serde_json::json!({
                "profile_id": profile.profile_id,
                "seq": profile.seq,
                "version": profile.version,
                "kind": profile.kind.as_token(),
                "reference_asset_id": profile.reference_asset_id,
                "name_ref": event_ref_for_text(&profile.name),
                "description_ref": event_ref_for_text(&profile.description),
                "reference_ref": profile.reference_ref,
                "source_ref": profile.source_ref,
                "crop_ref": profile.crop_ref,
                "artifact_ref": profile.artifact_ref,
                // Already redacted; never leak raw secrets into the ledger.
                "provenance": profile.provenance,
                "mutation": "append",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(profile)
    }

    /// Fetch one active identity profile by id.
    pub async fn get_identity_profile(
        &self,
        profile_id: Uuid,
    ) -> AtelierResult<Option<IdentityProfile>> {
        let row = sqlx::query(&format!(
            r#"SELECT {PROFILE_COLUMNS} FROM atelier_identity_profile
               WHERE profile_id = $1
                 AND deleted_at_utc IS NULL"#
        ))
        .bind(profile_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => Ok(Some(identity_profile_from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// Update mutable identity profile metadata while preserving character/seq identity.
    pub async fn update_identity_profile(
        &self,
        update: &UpdateIdentityProfile,
    ) -> AtelierResult<IdentityProfile> {
        validate_update_identity_profile(update)?;
        let name = redact_secrets(&update.name);
        let description = redact_secrets(&update.description);
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"UPDATE atelier_identity_profile
               SET name = $2,
                   description = $3,
                   source_ref = $4,
                   crop_ref = $5,
                   artifact_ref = $6,
                   version = version + 1,
                   updated_at_utc = NOW()
               WHERE profile_id = $1
                 AND deleted_at_utc IS NULL
               RETURNING {PROFILE_COLUMNS}"#
        ))
        .bind(update.profile_id)
        .bind(&name)
        .bind(&description)
        .bind(&update.source_ref)
        .bind(&update.crop_ref)
        .bind(&update.artifact_ref)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Err(AtelierError::NotFound(format!(
                "atelier_identity_profile profile_id={}",
                update.profile_id
            )));
        };
        let profile = identity_profile_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            IDENTITY_PROFILE_APPENDED,
            "atelier_identity_profile",
            &profile.profile_id.to_string(),
            serde_json::json!({
                "profile_id": profile.profile_id,
                "seq": profile.seq,
                "version": profile.version,
                "kind": profile.kind.as_token(),
                "name_ref": event_ref_for_text(&profile.name),
                "description_ref": event_ref_for_text(&profile.description),
                "source_ref": profile.source_ref,
                "crop_ref": profile.crop_ref,
                "artifact_ref": profile.artifact_ref,
                "requested_by": update.requested_by,
                "mutation": "update",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(profile)
    }

    /// Soft-delete an identity profile from normal CRUD/list projections.
    pub async fn delete_identity_profile(
        &self,
        profile_id: Uuid,
        requested_by: &str,
    ) -> AtelierResult<bool> {
        reject_legacy_runtime_ref("requested_by", requested_by)?;
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(&format!(
            r#"UPDATE atelier_identity_profile
               SET deleted_at_utc = NOW(),
                   version = version + 1,
                   updated_at_utc = NOW()
               WHERE profile_id = $1
                 AND deleted_at_utc IS NULL
               RETURNING {PROFILE_COLUMNS}"#
        ))
        .bind(profile_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Ok(false);
        };
        let profile = identity_profile_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            IDENTITY_PROFILE_APPENDED,
            "atelier_identity_profile",
            &profile.profile_id.to_string(),
            serde_json::json!({
                "profile_id": profile.profile_id,
                "seq": profile.seq,
                "version": profile.version,
                "kind": profile.kind.as_token(),
                "requested_by": requested_by,
                "mutation": "delete",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(true)
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

        let mut tx = self.pool().begin().await?;
        let profile_row = sqlx::query(
            r#"SELECT character_internal_id, version
               FROM atelier_identity_profile
               WHERE profile_id = $1
                 AND deleted_at_utc IS NULL"#,
        )
        .bind(new.profile_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(profile_row) = profile_row else {
            return Err(AtelierError::NotFound(format!(
                "atelier_identity_profile profile_id={}",
                new.profile_id
            )));
        };
        let character_internal_id: Uuid = profile_row.get("character_internal_id");
        let profile_version: i64 = profile_row.get("version");
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

        let inserted = sqlx::query(&format!(
            r#"INSERT INTO atelier_identity_crop_artifact
                 (crop_id, profile_id, profile_version, character_internal_id,
                  source_ref, crop_box, landmarks, artifact_ref, manifest_ref,
                  content_hash, byte_len, mime, width, height, manifest, created_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
               ON CONFLICT (profile_id, profile_version, content_hash) DO NOTHING
               RETURNING {IDENTITY_CROP_COLUMNS}"#
        ))
        .bind(crop_id)
        .bind(new.profile_id)
        .bind(profile_version)
        .bind(character_internal_id)
        .bind(&new.source_ref)
        .bind(&crop_box_json)
        .bind(&landmarks_json)
        .bind(&new.artifact_ref)
        .bind(&new.manifest_ref)
        .bind(&new.content_hash)
        .bind(new.byte_len)
        .bind(&new.mime)
        .bind(new.width)
        .bind(new.height)
        .bind(&manifest)
        .bind(&new.created_by)
        .fetch_optional(&mut *tx)
        .await?;

        let (crop, inserted_new) = match inserted {
            Some(row) => (identity_crop_artifact_from_row(&row)?, true),
            None => {
                let row = sqlx::query(&format!(
                    r#"SELECT {IDENTITY_CROP_COLUMNS}
                       FROM atelier_identity_crop_artifact
                       WHERE profile_id = $1
                         AND profile_version = $2
                         AND content_hash = $3"#
                ))
                .bind(new.profile_id)
                .bind(profile_version)
                .bind(&new.content_hash)
                .fetch_one(&mut *tx)
                .await?;
                (identity_crop_artifact_from_row(&row)?, false)
            }
        };

        if inserted_new {
            self.record_event_in_tx(
                &mut tx,
                IDENTITY_CROP_ARTIFACT_RECORDED,
                "atelier_identity_crop_artifact",
                &crop.crop_id.to_string(),
                serde_json::json!({
                    "crop_id": crop.crop_id,
                    "profile_id": crop.profile_id,
                    "profile_version": crop.profile_version,
                    "character_internal_id": crop.character_internal_id,
                    "source_ref": &crop.source_ref,
                    "crop_box": &crop.crop_box,
                    "landmark_count": crop.landmarks.len(),
                    "artifact_ref": &crop.artifact_ref,
                    "manifest_ref": &crop.manifest_ref,
                    "content_hash": &crop.content_hash,
                    "byte_len": crop.byte_len,
                    "mime": &crop.mime,
                    "width": crop.width,
                    "height": crop.height,
                    "manifest": &crop.manifest,
                    "created_by": &crop.created_by,
                }),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(crop)
    }

    /// Fetch one identity crop artifact by id.
    pub async fn get_identity_crop_artifact(
        &self,
        crop_id: Uuid,
    ) -> AtelierResult<Option<IdentityCropArtifact>> {
        let row = sqlx::query(&format!(
            r#"SELECT {IDENTITY_CROP_COLUMNS}
               FROM atelier_identity_crop_artifact
               WHERE crop_id = $1"#
        ))
        .bind(crop_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => Ok(Some(identity_crop_artifact_from_row(&row)?)),
            None => Ok(None),
        }
    }

    /// List identity crop artifacts for one profile in creation order.
    pub async fn list_identity_crop_artifacts(
        &self,
        profile_id: Uuid,
    ) -> AtelierResult<Vec<IdentityCropArtifact>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {IDENTITY_CROP_COLUMNS}
               FROM atelier_identity_crop_artifact
               WHERE profile_id = $1
               ORDER BY created_at_utc ASC, crop_id ASC"#
        ))
        .bind(profile_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(identity_crop_artifact_from_row).collect()
    }

    /// List a character's identity profiles in version order (ascending seq).
    pub async fn list_identity_profiles(
        &self,
        character_internal_id: Uuid,
        kind: Option<IdentityProfileKind>,
    ) -> AtelierResult<Vec<IdentityProfile>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {PROFILE_COLUMNS} FROM atelier_identity_profile
               WHERE character_internal_id = $1
                 AND ($2::TEXT IS NULL OR kind = $2)
                 AND deleted_at_utc IS NULL
               ORDER BY seq ASC"#
        ))
        .bind(character_internal_id)
        .bind(kind.map(|k| k.as_token()))
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(identity_profile_from_row).collect()
    }

    /// The latest (highest-seq) identity profile of a kind for a character.
    pub async fn latest_identity_profile(
        &self,
        character_internal_id: Uuid,
        kind: IdentityProfileKind,
    ) -> AtelierResult<Option<IdentityProfile>> {
        let row = sqlx::query(&format!(
            r#"SELECT {PROFILE_COLUMNS} FROM atelier_identity_profile
               WHERE character_internal_id = $1 AND kind = $2
                 AND deleted_at_utc IS NULL
               ORDER BY seq DESC LIMIT 1"#
        ))
        .bind(character_internal_id)
        .bind(kind.as_token())
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(identity_profile_from_row(&r)?)),
            None => Ok(None),
        }
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
