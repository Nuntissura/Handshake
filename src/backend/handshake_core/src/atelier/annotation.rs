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
//! that blob into a normalized, append-and-query model in the embedded
//! SurrealDB store so each typed region is individually addressable, typed,
//! and survives export. SQLite is NOT carried over; storage authority is the
//! single Handshake store only.
//!
//! Microtasks: MT-198 (annotation overlays), extends MT-005 (event coverage).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{uuid_from_record_link, AtelierError, AtelierResult, AtelierStore};

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
/// Coordinates are stored inside `geometry` (a flexible object field) and are
/// expected to be normalized to the 0..1 image space so they survive
/// resolution changes and export. The store validates the shape per
/// [`AnnotationKind`].
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

/// One `atelier_media_annotation` row as the store returns it.
#[derive(SurrealValue)]
struct MediaAnnotationRow {
    annotation_id: SurrealUuid,
    asset_id: SurrealUuid,
    kind: String,
    label: Option<String>,
    note: String,
    geometry: serde_json::Value,
    seq: i64,
    author: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl TryFrom<MediaAnnotationRow> for MediaAnnotation {
    type Error = AtelierError;

    fn try_from(row: MediaAnnotationRow) -> AtelierResult<Self> {
        Ok(MediaAnnotation {
            annotation_id: row.annotation_id.into(),
            asset_id: row.asset_id.into(),
            kind: AnnotationKind::parse(&row.kind)?,
            label: row.label,
            note: row.note,
            geometry: row.geometry,
            seq: row.seq,
            author: row.author,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

/// The annotation select list, with the asset link projected back to its uuid.
const MEDIA_ANNOTATION_FIELDS: &str =
    "annotation_id, record::id(asset_id) AS asset_id, kind, label, note, geometry, seq, \
     author, created_at_utc, updated_at_utc";

#[derive(SurrealValue)]
struct AddAnnotationBindings {
    record_id: RecordId,
    annotation_id: SurrealUuid,
    asset_ref: RecordId,
    kind: String,
    label: Option<String>,
    note: String,
    geometry: serde_json::Value,
    author: String,
}

#[derive(SurrealValue)]
struct AssetRefBinding {
    asset_ref: RecordId,
}

#[derive(SurrealValue)]
struct AnnotationIdBinding {
    annotation_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct UpdateAnnotationNoteBindings {
    annotation_id: SurrealUuid,
    note: String,
    label: Option<String>,
}

/// Assign the next per-asset sequence and create the annotation atomically, so
/// two concurrent adds cannot both observe the same maximum. The unique
/// `(asset_id, seq)` index remains the last line of defence.
const ADD_MEDIA_ANNOTATION_STATEMENT: &str = concat!(
    "RETURN { \
       LET $next = (array::max((SELECT VALUE seq FROM atelier_media_annotation \
                                WHERE asset_id = $asset_ref)) ?? 0) + 1; \
       CREATE $record_id CONTENT { \
         annotation_id: $annotation_id, \
         asset_id: $asset_ref, \
         kind: $kind, \
         label: $label, \
         note: $note, \
         geometry: $geometry, \
         seq: $next, \
         author: $author \
       }; \
       RETURN (SELECT ",
    "annotation_id, record::id(asset_id) AS asset_id, kind, label, note, geometry, seq, \
     author, created_at_utc, updated_at_utc",
    " FROM ONLY $record_id); };"
);

const LIST_MEDIA_ANNOTATIONS_STATEMENT: &str = concat!(
    "SELECT ",
    "annotation_id, record::id(asset_id) AS asset_id, kind, label, note, geometry, seq, \
     author, created_at_utc, updated_at_utc",
    " FROM atelier_media_annotation WHERE asset_id = $asset_ref ORDER BY seq ASC;"
);

const GET_MEDIA_ANNOTATION_STATEMENT: &str = concat!(
    "SELECT ",
    "annotation_id, record::id(asset_id) AS asset_id, kind, label, note, geometry, seq, \
     author, created_at_utc, updated_at_utc",
    " FROM atelier_media_annotation WHERE annotation_id = $annotation_id LIMIT 1;"
);

const UPDATE_MEDIA_ANNOTATION_NOTE_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_media_annotation', $annotation_id); \
       UPDATE $rid SET \
         note = $note, \
         label = $label ?? label, \
         updated_at_utc = time::now(); \
       RETURN (SELECT ",
    "annotation_id, record::id(asset_id) AS asset_id, kind, label, note, geometry, seq, \
     author, created_at_utc, updated_at_utc",
    " FROM $rid); };"
);

const REMOVE_MEDIA_ANNOTATION_STATEMENT: &str =
    "RETURN (DELETE type::record('atelier_media_annotation', $annotation_id) RETURN BEFORE);";

/// The record shape [`REMOVE_MEDIA_ANNOTATION_STATEMENT`] returns: the raw
/// asset link of the deleted row.
#[derive(SurrealValue)]
struct RemovedAnnotationRow {
    asset_id: RecordId,
}

const COUNT_MEDIA_ANNOTATIONS_STATEMENT: &str =
    "RETURN count(SELECT id FROM atelier_media_annotation WHERE asset_id = $asset_ref);";

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

        // Asset must exist; the schema link assertion also guards this but we
        // want a clean domain error.
        let asset_ref = RecordId::new("atelier_media_asset", SurrealUuid::from(new.asset_id));
        let exists_bindings = AssetRefBinding {
            asset_ref: asset_ref.clone(),
        };
        let asset_exists: Option<bool> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first("RETURN record::exists($asset_ref);", exists_bindings)
                        .await
                })
            })
            .await?;
        if !asset_exists.unwrap_or(false) {
            return Err(AtelierError::NotFound(format!(
                "media asset asset_id={}",
                new.asset_id
            )));
        }

        let annotation_id = Uuid::now_v7();
        let bindings = AddAnnotationBindings {
            record_id: RecordId::new("atelier_media_annotation", SurrealUuid::from(annotation_id)),
            annotation_id: SurrealUuid::from(annotation_id),
            asset_ref,
            kind: new.kind.as_str().to_owned(),
            label: new.label.clone(),
            note: new.note.clone(),
            geometry,
            author: new.author.clone(),
        };
        let row: Option<MediaAnnotationRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(ADD_MEDIA_ANNOTATION_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        let annotation: MediaAnnotation = row
            .ok_or_else(|| {
                AtelierError::Internal("adding a media annotation returned no row".to_owned())
            })?
            .try_into()?;

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
        let bindings = AssetRefBinding {
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
        };
        let rows: Vec<MediaAnnotationRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_MEDIA_ANNOTATIONS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter().map(MediaAnnotation::try_from).collect()
    }

    /// Fetch a single annotation by id.
    pub async fn get_media_annotation(
        &self,
        annotation_id: Uuid,
    ) -> AtelierResult<MediaAnnotation> {
        let bindings = AnnotationIdBinding {
            annotation_id: SurrealUuid::from(annotation_id),
        };
        let row: Option<MediaAnnotationRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_MEDIA_ANNOTATION_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        row.ok_or_else(|| {
            AtelierError::NotFound(format!("media annotation annotation_id={annotation_id}"))
        })?
        .try_into()
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
        let bindings = UpdateAnnotationNoteBindings {
            annotation_id: SurrealUuid::from(annotation_id),
            note: note.to_owned(),
            label: label.map(ToOwned::to_owned),
        };
        let row: Option<MediaAnnotationRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(UPDATE_MEDIA_ANNOTATION_NOTE_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        let annotation: MediaAnnotation = row
            .ok_or_else(|| {
                AtelierError::NotFound(format!("media annotation annotation_id={annotation_id}"))
            })?
            .try_into()?;

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
        let bindings = AnnotationIdBinding {
            annotation_id: SurrealUuid::from(annotation_id),
        };
        let removed: Option<RemovedAnnotationRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(REMOVE_MEDIA_ANNOTATION_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        let removed = removed.ok_or_else(|| {
            AtelierError::NotFound(format!("media annotation annotation_id={annotation_id}"))
        })?;
        let asset_id = uuid_from_record_link("asset_id", &removed.asset_id)?;

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
        let bindings = AssetRefBinding {
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
        };
        let count: Option<i64> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(COUNT_MEDIA_ANNOTATIONS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(count.unwrap_or_default())
    }
}
