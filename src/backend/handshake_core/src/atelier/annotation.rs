//! DAM-level media annotation overlays (MT-198): typed regions -- points,
//! boxes, and polygons -- layered over a [`super::MediaAsset`], each carrying a
//! free-text note. These overlays are decoupled from pose keypoints: they live
//! against media identity (`atelier_media_asset.asset_id`), not against a rig,
//! so they survive re-pose, re-import, and export.
//!
//! legacy source source: `app/backend/library.js` `getImageAnnotations` /
//! `setImageAnnotations` and `db.js` `ImageAnnotation` (MediaPane annotation
//! layers). legacy source stored a single `annotations_json` blob of point-pins
//! (`{x, y, text}` normalized 0..1) per image. This Handshake fold-in promotes
//! that blob into a normalized, append-and-query relational model so each typed
//! region is individually addressable, typed, and survives export. SQLite is
//! NOT carried over; storage authority is PostgreSQL only.
//!
//! Microtasks: MT-198 (annotation overlays), extends MT-005 (event coverage).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{AtelierError, AtelierResult, AtelierStore};

/// Annotation region geometry kind. Decoupled from pose keypoints: these are
/// operator/model overlays on the 2D media surface, not rig joints.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationKind {
    /// A single pin at a normalized (x, y) coordinate (the legacy source pin).
    Point,
    /// An axis-aligned rectangle: (x, y) top-left + (w, h) extent.
    Box,
    /// A free polygon described by `points` in `geometry`.
    Polygon,
}

impl AnnotationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AnnotationKind::Point => "point",
            AnnotationKind::Box => "box",
            AnnotationKind::Polygon => "polygon",
        }
    }

    pub fn parse(raw: &str) -> AtelierResult<Self> {
        match raw {
            "point" => Ok(AnnotationKind::Point),
            "box" => Ok(AnnotationKind::Box),
            "polygon" => Ok(AnnotationKind::Polygon),
            other => Err(AtelierError::Validation(format!(
                "unknown annotation kind: {other}"
            ))),
        }
    }
}

/// A single typed annotation region layered over a media asset.
///
/// Coordinates are stored inside `geometry` (JSONB) and are expected to be
/// normalized to the 0..1 image space so they survive resolution changes and
/// export. The repo validates the shape per [`AnnotationKind`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaAnnotation {
    pub annotation_id: Uuid,
    pub asset_id: Uuid,
    pub kind: AnnotationKind,
    /// Optional short typed label (e.g. "wardrobe", "blemish", "focus").
    pub label: Option<String>,
    /// Free-text operator/model note (the legacy source pin `text`).
    pub note: String,
    /// Normalized geometry payload validated per `kind`.
    pub geometry: serde_json::Value,
    /// Monotonic per-asset sequence; stable ordering for overlay paint + export.
    pub seq: i64,
    pub author: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input for creating a new annotation region on a media asset.
#[derive(Clone, Debug)]
pub struct NewMediaAnnotation {
    pub asset_id: Uuid,
    pub kind: AnnotationKind,
    pub label: Option<String>,
    pub note: String,
    pub geometry: serde_json::Value,
    pub author: String,
}

/// Atelier event families for media annotation overlays (extends MT-005).
pub mod annotation_event_family {
    pub const ANNOTATION_ADDED: &str = "atelier.annotation.added";
    pub const ANNOTATION_NOTE_UPDATED: &str = "atelier.annotation.note_updated";
    pub const ANNOTATION_REMOVED: &str = "atelier.annotation.removed";

    /// All annotation event families (used by parity/coverage checks).
    pub const ALL: &[&str] = &[
        ANNOTATION_ADDED,
        ANNOTATION_NOTE_UPDATED,
        ANNOTATION_REMOVED,
    ];
}

fn clamp01(n: f64) -> f64 {
    if !n.is_finite() {
        0.0
    } else if n < 0.0 {
        0.0
    } else if n > 1.0 {
        1.0
    } else {
        n
    }
}

fn require_num(obj: &serde_json::Value, key: &str) -> AtelierResult<f64> {
    obj.get(key)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| {
            AtelierError::Validation(format!("annotation geometry missing numeric '{key}'"))
        })
}

/// Validate + canonicalize geometry for a given kind, clamping coordinates to
/// the normalized 0..1 image space. Mirrors the legacy source `clamp01` discipline so
/// overlays never drift outside the asset and survive export intact.
fn canonical_geometry(
    kind: AnnotationKind,
    geometry: &serde_json::Value,
) -> AtelierResult<serde_json::Value> {
    match kind {
        AnnotationKind::Point => {
            let x = clamp01(require_num(geometry, "x")?);
            let y = clamp01(require_num(geometry, "y")?);
            Ok(serde_json::json!({ "x": x, "y": y }))
        }
        AnnotationKind::Box => {
            let x = clamp01(require_num(geometry, "x")?);
            let y = clamp01(require_num(geometry, "y")?);
            // Clamp extent so the box stays inside the unit square.
            let w = clamp01(require_num(geometry, "w")?).min(1.0 - x);
            let h = clamp01(require_num(geometry, "h")?).min(1.0 - y);
            if w <= 0.0 || h <= 0.0 {
                return Err(AtelierError::Validation(
                    "box annotation must have positive width and height".into(),
                ));
            }
            Ok(serde_json::json!({ "x": x, "y": y, "w": w, "h": h }))
        }
        AnnotationKind::Polygon => {
            let pts = geometry.get("points").and_then(serde_json::Value::as_array);
            let pts = match pts {
                Some(p) if p.len() >= 3 => p,
                _ => {
                    return Err(AtelierError::Validation(
                        "polygon annotation requires a 'points' array of >= 3 vertices".into(),
                    ));
                }
            };
            let mut out = Vec::with_capacity(pts.len());
            for p in pts {
                let x = clamp01(require_num(p, "x")?);
                let y = clamp01(require_num(p, "y")?);
                out.push(serde_json::json!({ "x": x, "y": y }));
            }
            Ok(serde_json::json!({ "points": out }))
        }
    }
}

fn annotation_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<MediaAnnotation> {
    let kind_raw: String = row.get("kind");
    Ok(MediaAnnotation {
        annotation_id: row.get("annotation_id"),
        asset_id: row.get("asset_id"),
        kind: AnnotationKind::parse(&kind_raw)?,
        label: row.get("label"),
        note: row.get("note"),
        geometry: row.get("geometry"),
        seq: row.get("seq"),
        author: row.get("author"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

impl AtelierStore {
    /// Add a typed annotation region to a media asset. Validates and clamps the
    /// geometry, assigns the next per-asset sequence, and emits
    /// [`annotation_event_family::ANNOTATION_ADDED`]. Append-style: existing
    /// overlays are never mutated by this call.
    pub async fn add_media_annotation(
        &self,
        new: &NewMediaAnnotation,
    ) -> AtelierResult<MediaAnnotation> {
        if new.author.trim().is_empty() {
            return Err(AtelierError::Validation("author must not be empty".into()));
        }
        let geometry = canonical_geometry(new.kind, &new.geometry)?;

        let mut tx = self.pool().begin().await?;

        // Asset must exist; FK also guards this but we want a clean domain error.
        let asset_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT asset_id FROM atelier_media_asset WHERE asset_id = $1")
                .bind(new.asset_id)
                .fetch_optional(&mut *tx)
                .await?;
        if asset_exists.is_none() {
            return Err(AtelierError::NotFound(format!(
                "media asset asset_id={}",
                new.asset_id
            )));
        }

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_media_annotation WHERE asset_id = $1",
        )
        .bind(new.asset_id)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_media_annotation
                 (asset_id, kind, label, note, geometry, seq, author)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING annotation_id, asset_id, kind, label, note, geometry, seq,
                         author, created_at_utc, updated_at_utc"#,
        )
        .bind(new.asset_id)
        .bind(new.kind.as_str())
        .bind(&new.label)
        .bind(&new.note)
        .bind(&geometry)
        .bind(next_seq)
        .bind(&new.author)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let annotation = annotation_from_row(&row)?;
        self.record_event(
            annotation_event_family::ANNOTATION_ADDED,
            "atelier_media_annotation",
            &annotation.asset_id.to_string(),
            serde_json::json!({
                "annotation_id": annotation.annotation_id,
                "kind": annotation.kind.as_str(),
                "label": annotation.label,
                "seq": annotation.seq,
            }),
        )
        .await?;
        Ok(annotation)
    }

    /// All annotation overlays for a media asset, in stable paint/export order
    /// (ascending sequence). This is the read path the MediaPane overlay uses.
    pub async fn list_media_annotations(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<Vec<MediaAnnotation>> {
        let rows = sqlx::query(
            r#"SELECT annotation_id, asset_id, kind, label, note, geometry, seq,
                      author, created_at_utc, updated_at_utc
               FROM atelier_media_annotation
               WHERE asset_id = $1
               ORDER BY seq ASC"#,
        )
        .bind(asset_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(annotation_from_row).collect()
    }

    /// Fetch a single annotation by id.
    pub async fn get_media_annotation(
        &self,
        annotation_id: Uuid,
    ) -> AtelierResult<MediaAnnotation> {
        let row = sqlx::query(
            r#"SELECT annotation_id, asset_id, kind, label, note, geometry, seq,
                      author, created_at_utc, updated_at_utc
               FROM atelier_media_annotation
               WHERE annotation_id = $1"#,
        )
        .bind(annotation_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| {
            AtelierError::NotFound(format!("media annotation annotation_id={annotation_id}"))
        })?;
        annotation_from_row(&row)
    }

    /// Update the free-text note (and optional label) on an existing annotation,
    /// bumping `updated_at_utc`. Geometry is immutable here so overlay position
    /// stays auditable; emits [`annotation_event_family::ANNOTATION_NOTE_UPDATED`].
    pub async fn update_media_annotation_note(
        &self,
        annotation_id: Uuid,
        note: &str,
        label: Option<&str>,
    ) -> AtelierResult<MediaAnnotation> {
        let row = sqlx::query(
            r#"UPDATE atelier_media_annotation
               SET note = $2,
                   label = COALESCE($3, label),
                   updated_at_utc = NOW()
               WHERE annotation_id = $1
               RETURNING annotation_id, asset_id, kind, label, note, geometry, seq,
                         author, created_at_utc, updated_at_utc"#,
        )
        .bind(annotation_id)
        .bind(note)
        .bind(label)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| {
            AtelierError::NotFound(format!("media annotation annotation_id={annotation_id}"))
        })?;

        let annotation = annotation_from_row(&row)?;
        self.record_event(
            annotation_event_family::ANNOTATION_NOTE_UPDATED,
            "atelier_media_annotation",
            &annotation.asset_id.to_string(),
            serde_json::json!({
                "annotation_id": annotation.annotation_id,
                "seq": annotation.seq,
            }),
        )
        .await?;
        Ok(annotation)
    }

    /// Remove an annotation overlay. Emits
    /// [`annotation_event_family::ANNOTATION_REMOVED`] for replay/audit. Returns
    /// the removed asset id so callers can refresh the overlay set.
    pub async fn remove_media_annotation(&self, annotation_id: Uuid) -> AtelierResult<Uuid> {
        let asset_id: Option<Uuid> = sqlx::query_scalar(
            "DELETE FROM atelier_media_annotation WHERE annotation_id = $1 RETURNING asset_id",
        )
        .bind(annotation_id)
        .fetch_optional(self.pool())
        .await?;
        let asset_id = asset_id.ok_or_else(|| {
            AtelierError::NotFound(format!("media annotation annotation_id={annotation_id}"))
        })?;

        self.record_event(
            annotation_event_family::ANNOTATION_REMOVED,
            "atelier_media_annotation",
            &asset_id.to_string(),
            serde_json::json!({ "annotation_id": annotation_id }),
        )
        .await?;
        Ok(asset_id)
    }

    /// Count annotation overlays on a media asset (used by export + tests).
    pub async fn count_media_annotations(&self, asset_id: Uuid) -> AtelierResult<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM atelier_media_annotation WHERE asset_id = $1")
                .bind(asset_id)
                .fetch_one(self.pool())
                .await?;
        Ok(count)
    }
}
