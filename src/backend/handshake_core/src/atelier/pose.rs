//! Pose / rig artifacts for the Photo Studio (WP-KERNEL-005, MT-PoseKit).
//!
//! Translates the CastKit-Codex `posekit` subsystem (`src/posekit/core.mjs`,
//! `src/posekit/poseDetection.worker.ts`) into the Handshake atelier domain.
//! CKC computes OpenPose-style rigs in an Electron renderer / web worker and
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
//! CKC source (intent only; SQLite/Electron/localhost/polling never copied):
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
//!     with provenance, append-only per character (mirrors CKC portrait
//!     identity versioning). Unique per `(character, seq)`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

/// Pose / identity event families (extends the MT-005 coverage set). The parent
/// wires these into [`super::event_family::ALL`].
pub mod pose_event_family {
    /// A pose rig artifact was ingested for a character + source media.
    pub const POSE_RIG_INGESTED: &str = "atelier.pose.rig_ingested";
    /// A head pose (yaw/pitch/roll + quaternion) was recorded for a rig.
    pub const POSE_HEAD_POSE_RECORDED: &str = "atelier.pose.head_pose_recorded";
    /// A calibration record was set (typically BLOCKED/unresolved).
    pub const POSE_CALIBRATION_SET: &str = "atelier.pose.calibration_set";
    /// A versioned identity profile was appended for a character.
    pub const IDENTITY_PROFILE_APPENDED: &str = "atelier.pose.identity_profile_appended";

    /// All pose/identity event families, exported for parity/coverage proofs.
    pub const ALL: &[&str] = &[
        POSE_RIG_INGESTED,
        POSE_HEAD_POSE_RECORDED,
        POSE_CALIBRATION_SET,
        IDENTITY_PROFILE_APPENDED,
    ];
}

/// Re-export at module root so callers can write `pose::POSE_RIG_INGESTED`.
pub use pose_event_family::{
    IDENTITY_PROFILE_APPENDED, POSE_CALIBRATION_SET, POSE_HEAD_POSE_RECORDED, POSE_RIG_INGESTED,
};

/// OpenPose keypoint-array cardinalities (CKC `rigToOpenposeJson`):
/// body-18, face-70, hand-21. Each keypoint is an `(x, y, confidence)` triple,
/// so flattened arrays have `count * 3` numbers.
pub const BODY_KEYPOINT_COUNT: usize = 18;
pub const FACE_KEYPOINT_COUNT: usize = 70;
pub const HAND_KEYPOINT_COUNT: usize = 21;

/// Provider/status of the detector that produced a rig. Mirrors CKC
/// `detector.provider` / `detector.status` (e.g. `mediapipe.tasks-vision.pose`
/// detected, or the deterministic `fallback`). The detector runs out-of-module;
/// this only records which one the Workflow-Engine job reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectorStatus {
    /// Keypoints were detected by a model.
    Detected,
    /// CKC "deterministic body-18 fallback" -- no model assets were used.
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

/// Calibration state for a rig. CKC `createDefaultCalibration` produces a live
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

/// Kind of identity profile reference media (CKC portrait/reference identity).
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

/// Canvas geometry for a rig (CKC `canvas` / `image` width+height). Keypoint
/// coordinates are absolute pixels in this canvas space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasSize {
    pub width: i32,
    pub height: i32,
}

/// A detected/authored pose rig artifact (MT-PoseKit).
///
/// `keypoints_json` holds the OpenPose payload shape produced by CKC
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
    /// Stable source identity (CKC `portraitImageId`); part of the idempotency
    /// key so re-detecting the same source never duplicates rigs.
    pub source_ref: String,
    /// Content hash of the rig payload for idempotent re-ingest and audit.
    pub content_hash: String,
    pub canvas: CanvasSize,
    pub detector_provider: String,
    pub detector_status: DetectorStatus,
    /// OpenPose keypoint arrays (body-18 / face-70 / hand-21), JSONB verbatim.
    pub keypoints_json: serde_json::Value,
    /// Deterministic sidecar ArtifactStore ref (CKC wrote a JSON sidecar to
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
    pub detector_status: DetectorStatus,
    pub keypoints_json: serde_json::Value,
    pub sidecar_ref: Option<String>,
}

/// Head pose for a rig: yaw/pitch/roll in degrees plus the normalized
/// quaternion (CKC `createHeadPose` / `normalizeHeadPose`, YXZ order).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HeadPose {
    pub rig_id: Uuid,
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    /// Normalized quaternion `[x, y, z, w]` (CKC `quaternionToArray`).
    pub quaternion: [f64; 4],
    pub created_at_utc: DateTime<Utc>,
}

/// CKC `HEAD_POSE_LIMITS`: yaw +-90, pitch +-75, roll +-45 (degrees).
const YAW_LIMIT_DEG: f64 = 90.0;
const PITCH_LIMIT_DEG: f64 = 75.0;
const ROLL_LIMIT_DEG: f64 = 45.0;

/// A calibration record for a rig, kept BLOCKED/unresolved by default.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Calibration {
    pub rig_id: Uuid,
    pub state: CalibrationState,
    /// Why calibration is unresolved (BLOCKED reason); preserved, never faked.
    pub block_reason: Option<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
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
    pub kind: IdentityProfileKind,
    /// Optional FK to the reference media asset (DAM, MT-015).
    pub reference_asset_id: Option<Uuid>,
    /// Stable reference id (CKC portrait/reference image id).
    pub reference_ref: String,
    /// Free-form provenance/lineage (redacted of any secret material).
    pub provenance: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to append an identity profile version.
#[derive(Clone, Debug)]
pub struct NewIdentityProfile {
    pub character_internal_id: Uuid,
    pub kind: IdentityProfileKind,
    pub reference_asset_id: Option<Uuid>,
    pub reference_ref: String,
    pub provenance: String,
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

/// Validate the OpenPose keypoint payload shape (CKC `rigToOpenposeJson`).
/// Body must be 18 triples; when present, face must be 70 triples and each hand
/// 21 triples. Absent face/hands are allowed (CKC zero-fills them). This is a
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
        detector_status: DetectorStatus::from_token(&detector_status)?,
        keypoints_json: row.get("keypoints_json"),
        sidecar_ref: row.get("sidecar_ref"),
        created_at_utc: row.get("created_at_utc"),
    })
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
    Ok(Calibration {
        rig_id: row.get("rig_id"),
        state: CalibrationState::from_token(&state)?,
        block_reason: row.get("block_reason"),
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
        kind: IdentityProfileKind::from_token(&kind)?,
        reference_asset_id: row.get("reference_asset_id"),
        reference_ref: row.get("reference_ref"),
        provenance: row.get("provenance"),
        created_at_utc: row.get("created_at_utc"),
    })
}

const RIG_COLUMNS: &str = "rig_id, character_internal_id, source_asset_id, source_ref, \
                           content_hash, canvas_width, canvas_height, detector_provider, \
                           detector_status, keypoints_json, sidecar_ref, created_at_utc";

const PROFILE_COLUMNS: &str = "profile_id, character_internal_id, seq, kind, reference_asset_id, \
                               reference_ref, provenance, created_at_utc";

impl AtelierStore {
    /// Ingest a pose rig artifact written through by a Workflow-Engine detection
    /// job (MT-PoseKit). No detector runs here; this stores the governed record.
    ///
    /// Idempotent on `(character_internal_id, source_ref, content_hash)`:
    /// re-ingesting an identical rig returns the existing row instead of
    /// duplicating it (mirrors the DAM content-hash dedup, MT-015). The
    /// OpenPose keypoint payload is structurally validated (body-18 required;
    /// face-70 and hand-21 optional and zero-fillable, per CKC
    /// `rigToOpenposeJson`). The character FK is guarded explicitly so a bad id
    /// is a clean not-found rather than a raw constraint violation. Emits
    /// `POSE_RIG_INGESTED`.
    pub async fn ingest_pose_rig(&self, new: &NewPoseRig) -> AtelierResult<PoseRig> {
        if new.source_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "source_ref must not be empty".into(),
            ));
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
                  keypoints_json, sidecar_ref)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
        .bind(&new.keypoints_json)
        .bind(&new.sidecar_ref)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let rig = rig_from_row(&row)?;
        self.record_event(
            POSE_RIG_INGESTED,
            "atelier_pose_rig",
            &rig.rig_id.to_string(),
            serde_json::json!({
                "rig_id": rig.rig_id,
                "character_internal_id": rig.character_internal_id,
                "source_asset_id": rig.source_asset_id,
                "source_ref": rig.source_ref,
                "content_hash": rig.content_hash,
                "detector_provider": rig.detector_provider,
                "detector_status": rig.detector_status.as_token(),
                "canvas_width": rig.canvas.width,
                "canvas_height": rig.canvas.height,
            }),
        )
        .await?;
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

    /// Record (or replace) the head pose for a rig (MT-PoseKit).
    ///
    /// Validates the CKC degree limits (yaw +-90 / pitch +-75 / roll +-45) and
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
        // Normalize (CKC `Quaternion.normalize`).
        let inv = 1.0 / len_sq.sqrt();
        let q = [
            quaternion[0] * inv,
            quaternion[1] * inv,
            quaternion[2] * inv,
            quaternion[3] * inv,
        ];

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
        .fetch_one(self.pool())
        .await?;

        let head_pose = head_pose_from_row(&row);
        self.record_event(
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
        // Guard the rig FK explicitly for a clean not-found.
        let _ = self.get_pose_rig(rig_id).await?;

        // An unresolved calibration must carry a block reason so the BLOCKED
        // status is auditable and never silently empty.
        if matches!(state, CalibrationState::Unresolved)
            && block_reason.map(|r| r.trim().is_empty()).unwrap_or(true)
        {
            return Err(AtelierError::Validation(
                "unresolved calibration must record a block_reason".into(),
            ));
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_pose_calibration (rig_id, state, block_reason)
               VALUES ($1, $2, $3)
               ON CONFLICT (rig_id) DO UPDATE SET
                 state         = EXCLUDED.state,
                 block_reason  = EXCLUDED.block_reason,
                 updated_at_utc = NOW()
               RETURNING rig_id, state, block_reason, created_at_utc, updated_at_utc"#,
        )
        .bind(rig_id)
        .bind(state.as_token())
        .bind(block_reason)
        .fetch_one(self.pool())
        .await?;

        let calibration = calibration_from_row(&row)?;
        self.record_event(
            POSE_CALIBRATION_SET,
            "atelier_pose_rig",
            &rig_id.to_string(),
            serde_json::json!({
                "rig_id": calibration.rig_id,
                "state": calibration.state.as_token(),
                "block_reason": calibration.block_reason,
            }),
        )
        .await?;
        Ok(calibration)
    }

    /// Fetch the calibration record for a rig, if any.
    pub async fn get_calibration(&self, rig_id: Uuid) -> AtelierResult<Option<Calibration>> {
        let row = sqlx::query(
            r#"SELECT rig_id, state, block_reason, created_at_utc, updated_at_utc
               FROM atelier_pose_calibration WHERE rig_id = $1"#,
        )
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
        if new.reference_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "reference_ref must not be empty".into(),
            ));
        }
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
                 (character_internal_id, seq, kind, reference_asset_id,
                  reference_ref, provenance)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING {PROFILE_COLUMNS}"#
        ))
        .bind(new.character_internal_id)
        .bind(next_seq)
        .bind(new.kind.as_token())
        .bind(new.reference_asset_id)
        .bind(&new.reference_ref)
        .bind(&provenance)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let profile = identity_profile_from_row(&row)?;
        self.record_event(
            IDENTITY_PROFILE_APPENDED,
            "atelier_identity_profile",
            &profile.profile_id.to_string(),
            serde_json::json!({
                "profile_id": profile.profile_id,
                "character_internal_id": profile.character_internal_id,
                "seq": profile.seq,
                "kind": profile.kind.as_token(),
                "reference_asset_id": profile.reference_asset_id,
                "reference_ref": profile.reference_ref,
                // Already redacted; never leak raw secrets into the ledger.
                "provenance": profile.provenance,
            }),
        )
        .await?;
        Ok(profile)
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

        // Absent face/hands are allowed (CKC zero-fills, so optional here).
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
