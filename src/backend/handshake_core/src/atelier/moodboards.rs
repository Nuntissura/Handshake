//! Moodboard JSON schema and layer model snapshots (MT-042).
//!
//! Moodboards are character documents with an attached schema-versioned JSON
//! snapshot. The exact source JSON is preserved for round-trip export, while a
//! JSONB projection and typed Rust model provide validation/query surfaces.

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};
use jsonschema::{Draft, JSONSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::documents::CharacterDocumentType;
use super::{
    atelier_event_sql, event_ref_for_text, uuid_from_record_link, AtelierError, AtelierResult,
    AtelierStore,
};

pub const MOODBOARD_SCHEMA_ID: &str = "hsk.atelier.moodboard@1";
const MOODBOARD_SCHEMA_VERSION: i64 = 1;
const MOODBOARD_OPERATION_RECEIPT_SCHEMA_ID: &str = "hsk.atelier.moodboard_operation_receipt@1";
const MOODBOARD_EXPORT_MANIFEST_SCHEMA_ID: &str = "hsk.atelier.moodboard_export_manifest@1";
const MOODBOARD_EXPORT_RECEIPT_SCHEMA_ID: &str = "hsk.atelier.moodboard_export_receipt@1";
const MOODBOARD_EXPORT_DEFERRED_REASON: &str = "deferred: PNG/PDF moodboard export is planned; no renderer is implemented and no output artifact was produced";

pub mod moodboard_event_family {
    pub const MOODBOARD_SNAPSHOT_RECORDED: &str = "atelier.moodboard.snapshot_recorded";
    pub const MOODBOARD_OPERATION_RECORDED: &str = "atelier.moodboard.operation_recorded";
    pub const MOODBOARD_EXPORT_REQUESTED: &str = "atelier.moodboard.export_requested";

    pub const ALL: &[&str] = &[
        MOODBOARD_SNAPSHOT_RECORDED,
        MOODBOARD_OPERATION_RECORDED,
        MOODBOARD_EXPORT_REQUESTED,
    ];
}

#[derive(Clone, Debug)]
pub struct NewMoodboardSnapshot {
    pub document_id: Uuid,
    pub raw_json_text: String,
    /// WP-CKC MT-012 optimistic concurrency: when set, the snapshot is refused
    /// with [`AtelierError::Conflict`] unless the moodboard document's current
    /// version is exactly this id. `None` skips the check.
    pub expected_document_version_id: Option<Uuid>,
    pub author: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardSnapshot {
    pub snapshot_id: Uuid,
    pub document_id: Uuid,
    pub document_version_id: Uuid,
    pub schema_id: String,
    pub schema_version: i64,
    pub raw_json_text: String,
    pub moodboard_json: Value,
    pub moodboard: MoodboardDocument,
    pub content_sha256: String,
    pub author: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoodboardOperationKind {
    LayerReordered,
    ElementMoved,
    ElementUpdated,
    LayerVisibilityChanged,
    StyleUpdated,
    HistoryAnnotated,
}

impl MoodboardOperationKind {
    pub fn as_token(self) -> &'static str {
        match self {
            MoodboardOperationKind::LayerReordered => "layer_reordered",
            MoodboardOperationKind::ElementMoved => "element_moved",
            MoodboardOperationKind::ElementUpdated => "element_updated",
            MoodboardOperationKind::LayerVisibilityChanged => "layer_visibility_changed",
            MoodboardOperationKind::StyleUpdated => "style_updated",
            MoodboardOperationKind::HistoryAnnotated => "history_annotated",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "layer_reordered" => Ok(MoodboardOperationKind::LayerReordered),
            "element_moved" => Ok(MoodboardOperationKind::ElementMoved),
            "element_updated" => Ok(MoodboardOperationKind::ElementUpdated),
            "layer_visibility_changed" => Ok(MoodboardOperationKind::LayerVisibilityChanged),
            "style_updated" => Ok(MoodboardOperationKind::StyleUpdated),
            "history_annotated" => Ok(MoodboardOperationKind::HistoryAnnotated),
            other => Err(AtelierError::Validation(format!(
                "unknown moodboard operation kind token: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewMoodboardOperation {
    pub snapshot_id: Uuid,
    pub operation_kind: MoodboardOperationKind,
    pub operation_payload: Value,
    pub actor: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardOperationReceipt {
    pub operation_id: Uuid,
    pub snapshot_id: Uuid,
    pub document_id: Uuid,
    pub document_version_id: Uuid,
    pub operation_kind: MoodboardOperationKind,
    pub operation_payload: Value,
    pub operation_payload_sha256: String,
    pub receipt_json: Value,
    pub actor: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoodboardExportFormat {
    Png,
    Pdf,
}

impl MoodboardExportFormat {
    pub fn as_token(self) -> &'static str {
        match self {
            MoodboardExportFormat::Png => "png",
            MoodboardExportFormat::Pdf => "pdf",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "png" => Ok(MoodboardExportFormat::Png),
            "pdf" => Ok(MoodboardExportFormat::Pdf),
            other => Err(AtelierError::Validation(format!(
                "unknown moodboard export format token: {other}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoodboardExportStatus {
    Planned,
}

impl MoodboardExportStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            MoodboardExportStatus::Planned => "planned",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "planned" => Ok(MoodboardExportStatus::Planned),
            other => Err(AtelierError::Validation(format!(
                "unknown moodboard export status token: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewMoodboardExportRequest {
    pub snapshot_id: Uuid,
    pub format: MoodboardExportFormat,
    pub label: Option<String>,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardExportRequest {
    pub export_id: Uuid,
    pub snapshot_id: Uuid,
    pub document_id: Uuid,
    pub document_version_id: Uuid,
    pub format: MoodboardExportFormat,
    pub status: MoodboardExportStatus,
    pub label: Option<String>,
    pub manifest_json: Value,
    pub receipt_json: Value,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardDocument {
    pub schema_id: String,
    pub schema_version: i64,
    pub moodboard_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub canvas: MoodboardCanvas,
    pub layers: Vec<MoodboardLayer>,
    pub images: Vec<MoodboardImageElement>,
    #[serde(rename = "text")]
    pub text: Vec<MoodboardTextElement>,
    pub shapes: Vec<MoodboardShapeElement>,
    pub connectors: Vec<MoodboardConnector>,
    pub folders: Vec<MoodboardFolder>,
    pub guides: Vec<MoodboardGuide>,
    pub flags: MoodboardFlags,
    pub style: MoodboardStyle,
    pub history: Vec<MoodboardHistoryEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardCanvas {
    pub width: f64,
    pub height: f64,
    pub background_color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardLayer {
    pub layer_id: Uuid,
    pub name: String,
    pub order: i64,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f64,
    pub parent_layer_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardImageElement {
    pub element_id: Uuid,
    pub layer_id: Uuid,
    pub asset_id: Option<Uuid>,
    pub source: String,
    pub url: Option<String>,
    pub position: MoodboardPoint,
    pub size: MoodboardSize,
    pub rotation: f64,
    pub opacity: f64,
    pub flags: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardTextElement {
    pub element_id: Uuid,
    pub layer_id: Uuid,
    pub content: String,
    pub font: String,
    pub font_size: f64,
    pub color: String,
    pub position: MoodboardPoint,
    pub rotation: f64,
    pub flags: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardShapeElement {
    pub element_id: Uuid,
    pub layer_id: Uuid,
    pub shape_type: String,
    pub position: MoodboardPoint,
    pub size: MoodboardSize,
    pub rotation: f64,
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
    pub flags: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardConnector {
    pub connector_id: Uuid,
    pub layer_id: Uuid,
    pub from_element_id: Uuid,
    pub to_element_id: Uuid,
    pub points: Vec<MoodboardPoint>,
    pub style: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardFolder {
    pub folder_id: Uuid,
    pub name: String,
    pub collapsed: bool,
    pub children: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardGuide {
    pub guide_id: Uuid,
    pub axis: String,
    pub position: f64,
    pub locked: bool,
    pub label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardFlags {
    pub locked: bool,
    pub archived: bool,
    pub operator_reviewed: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardStyle {
    pub dominant_colors: Vec<String>,
    pub mood_keywords: Vec<String>,
    pub style_description: String,
    pub suggested_presets: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoodboardHistoryEntry {
    pub history_id: Uuid,
    pub at: DateTime<Utc>,
    pub actor: String,
    pub operation: String,
    pub summary: String,
}

fn require_non_empty_trimmed(field: &str, value: &str) -> AtelierResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn uuid_schema() -> Value {
    serde_json::json!({
        "type": "string",
        "pattern": "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
    })
}

fn point_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "required": ["x", "y"],
        "additionalProperties": false,
        "properties": {
            "x": { "type": "number" },
            "y": { "type": "number" }
        }
    })
}

fn size_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "required": ["width", "height"],
        "additionalProperties": false,
        "properties": {
            "width": { "type": "number", "exclusiveMinimum": 0 },
            "height": { "type": "number", "exclusiveMinimum": 0 }
        }
    })
}

fn flags_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": { "type": "boolean" }
    })
}

fn style_map_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": {
            "type": ["string", "number", "integer", "boolean", "null"]
        }
    })
}

fn moodboard_json_schema() -> Value {
    let uuid = uuid_schema();
    let nullable_uuid = serde_json::json!({ "anyOf": [uuid.clone(), { "type": "null" }] });
    let point = point_schema();
    let size = size_schema();
    let element_flags = flags_schema();
    let style_map = style_map_schema();

    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": [
            "schema_id",
            "schema_version",
            "moodboard_id",
            "name",
            "canvas",
            "layers",
            "images",
            "text",
            "shapes",
            "connectors",
            "folders",
            "guides",
            "flags",
            "style",
            "history"
        ],
        "additionalProperties": false,
        "properties": {
            "schema_id": { "const": MOODBOARD_SCHEMA_ID },
            "schema_version": { "const": MOODBOARD_SCHEMA_VERSION },
            "moodboard_id": uuid.clone(),
            "name": { "type": "string", "minLength": 1 },
            "description": { "type": ["string", "null"] },
            "canvas": {
                "type": "object",
                "required": ["width", "height", "background_color"],
                "additionalProperties": false,
                "properties": {
                    "width": { "type": "number", "exclusiveMinimum": 0 },
                    "height": { "type": "number", "exclusiveMinimum": 0 },
                    "background_color": { "type": "string", "minLength": 1 }
                }
            },
            "layers": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": [
                        "layer_id",
                        "name",
                        "order",
                        "visible",
                        "locked",
                        "opacity",
                        "parent_layer_id"
                    ],
                    "additionalProperties": false,
                    "properties": {
                        "layer_id": uuid.clone(),
                        "name": { "type": "string", "minLength": 1 },
                        "order": { "type": "integer" },
                        "visible": { "type": "boolean" },
                        "locked": { "type": "boolean" },
                        "opacity": { "type": "number", "minimum": 0, "maximum": 1 },
                        "parent_layer_id": nullable_uuid.clone()
                    }
                }
            },
            "images": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": [
                        "element_id",
                        "layer_id",
                        "asset_id",
                        "source",
                        "url",
                        "position",
                        "size",
                        "rotation",
                        "opacity",
                        "flags"
                    ],
                    "additionalProperties": false,
                    "properties": {
                        "element_id": uuid.clone(),
                        "layer_id": uuid.clone(),
                        "asset_id": nullable_uuid.clone(),
                        "source": { "type": "string", "enum": ["local", "web", "generated"] },
                        "url": { "type": ["string", "null"] },
                        "position": point.clone(),
                        "size": size.clone(),
                        "rotation": { "type": "number" },
                        "opacity": { "type": "number", "minimum": 0, "maximum": 1 },
                        "flags": element_flags.clone()
                    }
                }
            },
            "text": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": [
                        "element_id",
                        "layer_id",
                        "content",
                        "font",
                        "font_size",
                        "color",
                        "position",
                        "rotation",
                        "flags"
                    ],
                    "additionalProperties": false,
                    "properties": {
                        "element_id": uuid.clone(),
                        "layer_id": uuid.clone(),
                        "content": { "type": "string" },
                        "font": { "type": "string", "minLength": 1 },
                        "font_size": { "type": "number", "exclusiveMinimum": 0 },
                        "color": { "type": "string", "minLength": 1 },
                        "position": point.clone(),
                        "rotation": { "type": "number" },
                        "flags": element_flags.clone()
                    }
                }
            },
            "shapes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": [
                        "element_id",
                        "layer_id",
                        "shape_type",
                        "position",
                        "size",
                        "rotation",
                        "fill",
                        "stroke",
                        "stroke_width",
                        "flags"
                    ],
                    "additionalProperties": false,
                    "properties": {
                        "element_id": uuid.clone(),
                        "layer_id": uuid.clone(),
                        "shape_type": { "type": "string", "minLength": 1 },
                        "position": point.clone(),
                        "size": size.clone(),
                        "rotation": { "type": "number" },
                        "fill": { "type": "string", "minLength": 1 },
                        "stroke": { "type": "string", "minLength": 1 },
                        "stroke_width": { "type": "number", "minimum": 0 },
                        "flags": element_flags.clone()
                    }
                }
            },
            "connectors": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": [
                        "connector_id",
                        "layer_id",
                        "from_element_id",
                        "to_element_id",
                        "points",
                        "style"
                    ],
                    "additionalProperties": false,
                    "properties": {
                        "connector_id": uuid.clone(),
                        "layer_id": uuid.clone(),
                        "from_element_id": uuid.clone(),
                        "to_element_id": uuid.clone(),
                        "points": {
                            "type": "array",
                            "minItems": 1,
                            "items": point.clone()
                        },
                        "style": style_map.clone()
                    }
                }
            },
            "folders": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["folder_id", "name", "collapsed", "children"],
                    "additionalProperties": false,
                    "properties": {
                        "folder_id": uuid.clone(),
                        "name": { "type": "string", "minLength": 1 },
                        "collapsed": { "type": "boolean" },
                        "children": {
                            "type": "array",
                            "items": uuid.clone(),
                            "uniqueItems": true
                        }
                    }
                }
            },
            "guides": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["guide_id", "axis", "position", "locked", "label"],
                    "additionalProperties": false,
                    "properties": {
                        "guide_id": uuid.clone(),
                        "axis": { "type": "string", "enum": ["horizontal", "vertical"] },
                        "position": { "type": "number" },
                        "locked": { "type": "boolean" },
                        "label": { "type": ["string", "null"] }
                    }
                }
            },
            "flags": {
                "type": "object",
                "required": ["locked", "archived", "operator_reviewed"],
                "additionalProperties": { "type": "boolean" },
                "properties": {
                    "locked": { "type": "boolean" },
                    "archived": { "type": "boolean" },
                    "operator_reviewed": { "type": "boolean" }
                }
            },
            "style": {
                "type": "object",
                "required": [
                    "dominant_colors",
                    "mood_keywords",
                    "style_description",
                    "suggested_presets"
                ],
                "additionalProperties": false,
                "properties": {
                    "dominant_colors": {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1 },
                        "uniqueItems": true
                    },
                    "mood_keywords": {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1 },
                        "uniqueItems": true
                    },
                    "style_description": { "type": "string" },
                    "suggested_presets": {
                        "type": "array",
                        "items": uuid.clone(),
                        "uniqueItems": true
                    }
                }
            },
            "history": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["history_id", "at", "actor", "operation", "summary"],
                    "additionalProperties": false,
                    "properties": {
                        "history_id": uuid,
                        "at": { "type": "string", "minLength": 1 },
                        "actor": { "type": "string", "minLength": 1 },
                        "operation": { "type": "string", "minLength": 1 },
                        "summary": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn validate_against_schema(document: &Value) -> AtelierResult<()> {
    let schema = moodboard_json_schema();
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .map_err(|err| {
            AtelierError::Validation(format!(
                "invalid moodboard Draft 2020-12 JSON Schema: {err}"
            ))
        })?;

    if let Err(errors) = compiled.validate(document) {
        let details = errors
            .take(5)
            .map(|err| format!("{} at {}", err, err.instance_path))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(AtelierError::Validation(format!(
            "moodboard JSON failed Draft 2020-12 validation: {details}"
        )));
    }
    Ok(())
}

fn parse_and_validate_moodboard(raw_json_text: &str) -> AtelierResult<(Value, MoodboardDocument)> {
    require_non_empty_trimmed("raw_json_text", raw_json_text)?;
    let moodboard_json: Value = serde_json::from_str(raw_json_text).map_err(|err| {
        AtelierError::Validation(format!("moodboard JSON could not be parsed: {err}"))
    })?;
    validate_against_schema(&moodboard_json)?;
    let moodboard: MoodboardDocument =
        serde_json::from_value(moodboard_json.clone()).map_err(|err| {
            AtelierError::Validation(format!("moodboard JSON did not match typed model: {err}"))
        })?;
    validate_layer_model(&moodboard)?;
    Ok((moodboard_json, moodboard))
}

fn insert_uuid(
    ids: &mut HashSet<Uuid>,
    kind: &str,
    id: Uuid,
    parent: Option<Uuid>,
) -> AtelierResult<()> {
    if !ids.insert(id) {
        let parent_text = parent
            .map(|value| format!(" in {value}"))
            .unwrap_or_default();
        return Err(AtelierError::Validation(format!(
            "duplicate moodboard {kind} id {id}{parent_text}"
        )));
    }
    Ok(())
}

fn validate_layer_model(moodboard: &MoodboardDocument) -> AtelierResult<()> {
    let mut layer_ids = HashSet::new();
    for layer in &moodboard.layers {
        insert_uuid(&mut layer_ids, "layer", layer.layer_id, None)?;
    }
    for layer in &moodboard.layers {
        if let Some(parent_layer_id) = layer.parent_layer_id {
            if parent_layer_id == layer.layer_id {
                return Err(AtelierError::Validation(format!(
                    "moodboard layer {} cannot parent itself",
                    layer.layer_id
                )));
            }
            if !layer_ids.contains(&parent_layer_id) {
                return Err(AtelierError::Validation(format!(
                    "moodboard layer {} references missing parent layer {parent_layer_id}",
                    layer.layer_id
                )));
            }
        }
    }

    let mut element_ids = HashSet::new();
    for image in &moodboard.images {
        require_layer(&layer_ids, image.layer_id, "image", image.element_id)?;
        insert_uuid(&mut element_ids, "element", image.element_id, None)?;
    }
    for text in &moodboard.text {
        require_layer(&layer_ids, text.layer_id, "text", text.element_id)?;
        insert_uuid(&mut element_ids, "element", text.element_id, None)?;
    }
    for shape in &moodboard.shapes {
        require_layer(&layer_ids, shape.layer_id, "shape", shape.element_id)?;
        insert_uuid(&mut element_ids, "element", shape.element_id, None)?;
    }

    let mut connector_ids = HashSet::new();
    for connector in &moodboard.connectors {
        require_layer(
            &layer_ids,
            connector.layer_id,
            "connector",
            connector.connector_id,
        )?;
        insert_uuid(
            &mut connector_ids,
            "connector",
            connector.connector_id,
            None,
        )?;
        if !element_ids.contains(&connector.from_element_id) {
            return Err(AtelierError::Validation(format!(
                "moodboard connector {} references missing from_element_id {}",
                connector.connector_id, connector.from_element_id
            )));
        }
        if !element_ids.contains(&connector.to_element_id) {
            return Err(AtelierError::Validation(format!(
                "moodboard connector {} references missing to_element_id {}",
                connector.connector_id, connector.to_element_id
            )));
        }
    }

    let mut folder_ids = HashSet::new();
    let mut folder_child_ids = element_ids.clone();
    folder_child_ids.extend(connector_ids.iter().copied());
    folder_child_ids.extend(layer_ids.iter().copied());
    for folder in &moodboard.folders {
        insert_uuid(&mut folder_ids, "folder", folder.folder_id, None)?;
        for child in &folder.children {
            if !folder_child_ids.contains(child) {
                return Err(AtelierError::Validation(format!(
                    "moodboard folder {} references missing child {child}",
                    folder.folder_id
                )));
            }
        }
    }

    let mut guide_ids = HashSet::new();
    for guide in &moodboard.guides {
        insert_uuid(&mut guide_ids, "guide", guide.guide_id, None)?;
    }
    let mut history_ids = HashSet::new();
    for entry in &moodboard.history {
        insert_uuid(&mut history_ids, "history", entry.history_id, None)?;
    }

    Ok(())
}

fn require_layer(
    layer_ids: &HashSet<Uuid>,
    layer_id: Uuid,
    kind: &str,
    element_id: Uuid,
) -> AtelierResult<()> {
    if !layer_ids.contains(&layer_id) {
        return Err(AtelierError::Validation(format!(
            "moodboard {kind} {element_id} references missing layer {layer_id}"
        )));
    }
    Ok(())
}

#[derive(SurrealValue)]
struct MoodboardSnapshotRow {
    snapshot_id: SurrealUuid,
    document_id: SurrealUuid,
    document_version_id: SurrealUuid,
    schema_id: String,
    schema_version: i64,
    raw_json_text: String,
    moodboard_json: Value,
    content_sha256: String,
    author: String,
    created_at_utc: Datetime,
    doc_type: Option<String>,
    version_document_id: Option<SurrealUuid>,
}

impl TryFrom<MoodboardSnapshotRow> for MoodboardSnapshot {
    type Error = AtelierError;

    fn try_from(row: MoodboardSnapshotRow) -> AtelierResult<Self> {
        let moodboard_json = row.moodboard_json;
        let moodboard: MoodboardDocument =
            serde_json::from_value(moodboard_json.clone()).map_err(|err| {
                AtelierError::Validation(format!(
                    "stored moodboard JSON did not match model: {err}"
                ))
            })?;
        Ok(MoodboardSnapshot {
            snapshot_id: row.snapshot_id.into(),
            document_id: row.document_id.into(),
            document_version_id: row.document_version_id.into(),
            schema_id: row.schema_id,
            schema_version: row.schema_version,
            raw_json_text: row.raw_json_text,
            moodboard_json,
            moodboard,
            content_sha256: row.content_sha256,
            author: row.author,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct MoodboardOperationReceiptRow {
    operation_id: SurrealUuid,
    snapshot_id: SurrealUuid,
    document_id: SurrealUuid,
    document_version_id: SurrealUuid,
    operation_kind: String,
    operation_payload: Value,
    operation_payload_sha256: String,
    receipt_json: Value,
    actor: String,
    created_at_utc: Datetime,
}

impl TryFrom<MoodboardOperationReceiptRow> for MoodboardOperationReceipt {
    type Error = AtelierError;

    fn try_from(row: MoodboardOperationReceiptRow) -> AtelierResult<Self> {
        Ok(MoodboardOperationReceipt {
            operation_id: row.operation_id.into(),
            snapshot_id: row.snapshot_id.into(),
            document_id: row.document_id.into(),
            document_version_id: row.document_version_id.into(),
            operation_kind: MoodboardOperationKind::from_token(&row.operation_kind)?,
            operation_payload: row.operation_payload,
            operation_payload_sha256: row.operation_payload_sha256,
            receipt_json: row.receipt_json,
            actor: row.actor,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct MoodboardExportRequestRow {
    export_id: SurrealUuid,
    snapshot_id: SurrealUuid,
    document_id: SurrealUuid,
    document_version_id: SurrealUuid,
    format: String,
    status: String,
    label: Option<String>,
    manifest_json: Value,
    receipt_json: Value,
    requested_by: String,
    created_at_utc: Datetime,
    inserted: Option<bool>,
}

impl TryFrom<MoodboardExportRequestRow> for MoodboardExportRequest {
    type Error = AtelierError;

    fn try_from(row: MoodboardExportRequestRow) -> AtelierResult<Self> {
        Ok(MoodboardExportRequest {
            export_id: row.export_id.into(),
            snapshot_id: row.snapshot_id.into(),
            document_id: row.document_id.into(),
            document_version_id: row.document_version_id.into(),
            format: MoodboardExportFormat::from_token(&row.format)?,
            status: MoodboardExportStatus::from_token(&row.status)?,
            label: row.label,
            manifest_json: row.manifest_json,
            receipt_json: row.receipt_json,
            requested_by: row.requested_by,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

fn moodboard_counts_json(snapshot: &MoodboardSnapshot) -> Value {
    serde_json::json!({
        "layers": snapshot.moodboard.layers.len(),
        "images": snapshot.moodboard.images.len(),
        "text": snapshot.moodboard.text.len(),
        "shapes": snapshot.moodboard.shapes.len(),
        "connectors": snapshot.moodboard.connectors.len(),
        "folders": snapshot.moodboard.folders.len(),
        "guides": snapshot.moodboard.guides.len(),
        "history": snapshot.moodboard.history.len(),
    })
}

fn build_operation_receipt_json(
    operation_id: Uuid,
    snapshot: &MoodboardSnapshot,
    operation_kind: MoodboardOperationKind,
    operation_payload_sha256: &str,
    actor: &str,
) -> Value {
    serde_json::json!({
        "schema": MOODBOARD_OPERATION_RECEIPT_SCHEMA_ID,
        "operation_id": operation_id,
        "snapshot_id": snapshot.snapshot_id,
        "document_id": snapshot.document_id,
        "document_version_id": snapshot.document_version_id,
        "operation_kind": operation_kind.as_token(),
        "operation_payload_sha256": operation_payload_sha256,
        "source_schema_id": snapshot.schema_id,
        "source_schema_version": snapshot.schema_version,
        "source_content_sha256": snapshot.content_sha256,
        "actor": actor,
        "status": "recorded"
    })
}

fn build_export_manifest_json(
    export_id: Uuid,
    snapshot: &MoodboardSnapshot,
    format: MoodboardExportFormat,
) -> Value {
    serde_json::json!({
        "schema": MOODBOARD_EXPORT_MANIFEST_SCHEMA_ID,
        "export_id": export_id,
        "snapshot_id": snapshot.snapshot_id,
        "document_id": snapshot.document_id,
        "document_version_id": snapshot.document_version_id,
        "format": format.as_token(),
        "status": MoodboardExportStatus::Planned.as_token(),
        "source_schema_id": snapshot.schema_id,
        "source_schema_version": snapshot.schema_version,
        "source_content_sha256": snapshot.content_sha256,
        "counts": moodboard_counts_json(snapshot),
        "output": {
            "artifact": "not_produced",
            "reason": MOODBOARD_EXPORT_DEFERRED_REASON
        }
    })
}

fn build_export_receipt_json(
    export_id: Uuid,
    snapshot: &MoodboardSnapshot,
    format: MoodboardExportFormat,
    requested_by: &str,
) -> Value {
    serde_json::json!({
        "schema": MOODBOARD_EXPORT_RECEIPT_SCHEMA_ID,
        "export_id": export_id,
        "snapshot_id": snapshot.snapshot_id,
        "document_id": snapshot.document_id,
        "document_version_id": snapshot.document_version_id,
        "format": format.as_token(),
        "status": MoodboardExportStatus::Planned.as_token(),
        "requested_by": requested_by,
        "source_content_sha256": snapshot.content_sha256,
        "output_artifact": "not_produced",
        "deferred_reason": MOODBOARD_EXPORT_DEFERRED_REASON
    })
}

fn validate_snapshot_join_invariants(
    snapshot: &MoodboardSnapshot,
    doc_type_token: &str,
    version_document_id: Uuid,
) -> AtelierResult<()> {
    if CharacterDocumentType::from_token(doc_type_token)? != CharacterDocumentType::Moodboard {
        return Err(AtelierError::Validation(format!(
            "moodboard snapshot {} is attached to non-moodboard document {}",
            snapshot.snapshot_id, snapshot.document_id
        )));
    }
    if version_document_id != snapshot.document_id {
        return Err(AtelierError::Validation(format!(
            "moodboard snapshot {} version {} belongs to document {}, not {}",
            snapshot.snapshot_id,
            snapshot.document_version_id,
            version_document_id,
            snapshot.document_id
        )));
    }
    Ok(())
}

#[derive(SurrealValue)]
struct UuidBinding {
    id: SurrealUuid,
}

#[derive(SurrealValue)]
struct DocumentIdBinding {
    document_id: RecordId,
}

#[derive(SurrealValue)]
struct SnapshotIdBinding {
    snapshot_id: RecordId,
}

#[derive(SurrealValue)]
struct ExportLookupBinding {
    snapshot_id: RecordId,
    format: String,
}

#[derive(SurrealValue)]
struct DocumentHeadRow {
    doc_type: String,
    current_version_id: Option<RecordId>,
}

#[derive(Clone, SurrealValue)]
struct MoodboardSnapshotBindings {
    record_id: RecordId,
    snapshot_id: SurrealUuid,
    document_id: RecordId,
    document_version_id: RecordId,
    schema_id: String,
    schema_version: i64,
    raw_json_text: String,
    moodboard_json: Value,
    content_sha256: String,
    author: String,
}

#[derive(Clone, SurrealValue)]
struct MoodboardOperationBindings {
    record_id: RecordId,
    operation_id: SurrealUuid,
    snapshot_id: RecordId,
    document_id: RecordId,
    document_version_id: RecordId,
    operation_kind: String,
    operation_payload: Value,
    operation_payload_sha256: String,
    receipt_json: Value,
    actor: String,
}

#[derive(Clone, SurrealValue)]
struct MoodboardExportBindings {
    record_id: RecordId,
    export_id: SurrealUuid,
    snapshot_id: RecordId,
    document_id: RecordId,
    document_version_id: RecordId,
    format: String,
    status: String,
    label: Option<String>,
    manifest_json: Value,
    receipt_json: Value,
    requested_by: String,
}

macro_rules! snapshot_columns {
    () => {
        "snapshot_id, record::id(document_id) AS document_id, \
         record::id(document_version_id) AS document_version_id, schema_id, schema_version, \
         raw_json_text, moodboard_json, content_sha256, author, created_at_utc"
    };
}
macro_rules! operation_columns {
    () => {
        "operation_id, record::id(snapshot_id) AS snapshot_id, \
         record::id(document_id) AS document_id, \
         record::id(document_version_id) AS document_version_id, operation_kind, \
         operation_payload, operation_payload_sha256, receipt_json, actor, created_at_utc"
    };
}
macro_rules! export_columns {
    () => {
        "export_id, record::id(snapshot_id) AS snapshot_id, \
         record::id(document_id) AS document_id, \
         record::id(document_version_id) AS document_version_id, format, status, label, \
         manifest_json, receipt_json, requested_by, created_at_utc"
    };
}

const GET_SNAPSHOT_STATEMENT: &str = concat!(
    "SELECT ",
    snapshot_columns!(),
    ", document_id.doc_type AS doc_type, \
       record::id(document_version_id.document_id) AS version_document_id \
     FROM atelier_moodboard WHERE snapshot_id = $id LIMIT 1;"
);
const LATEST_SNAPSHOT_STATEMENT: &str = concat!(
    "SELECT ",
    snapshot_columns!(),
    ", document_id.doc_type AS doc_type, \
       record::id(document_version_id.document_id) AS version_document_id \
     FROM atelier_moodboard WHERE document_id = $document_id \
     ORDER BY created_at_utc DESC, snapshot_id DESC LIMIT 1;"
);
const FIND_SNAPSHOT_STATEMENT: &str = concat!(
    "SELECT ",
    snapshot_columns!(),
    " FROM atelier_moodboard WHERE document_id = $document_id \
      AND document_version_id = $document_version_id \
      AND content_sha256 = $content_sha256 LIMIT 1;"
);
const RECORD_SNAPSHOT_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " RETURN (CREATE $rid CONTENT { \
       snapshot_id: $domain.snapshot_id, document_id: $domain.document_id, \
       document_version_id: $domain.document_version_id, schema_id: $domain.schema_id, \
       schema_version: $domain.schema_version, raw_json_text: $domain.raw_json_text, \
       moodboard_json: $domain.moodboard_json, content_sha256: $domain.content_sha256, \
       author: $domain.author })[0]; };"
);
const RECORD_OPERATION_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " RETURN (CREATE $rid CONTENT { \
       operation_id: $domain.operation_id, snapshot_id: $domain.snapshot_id, \
       document_id: $domain.document_id, document_version_id: $domain.document_version_id, \
       operation_kind: $domain.operation_kind, operation_payload: $domain.operation_payload, \
       operation_payload_sha256: $domain.operation_payload_sha256, \
       receipt_json: $domain.receipt_json, actor: $domain.actor })[0]; };"
);
const LIST_OPERATIONS_STATEMENT: &str = concat!(
    "SELECT ",
    operation_columns!(),
    " FROM atelier_moodboard_operation_receipt WHERE snapshot_id = $snapshot_id \
      ORDER BY created_at_utc ASC, operation_id ASC;"
);
const RECORD_EXPORT_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " LET $created = (CREATE $rid CONTENT { \
       export_id: $domain.export_id, snapshot_id: $domain.snapshot_id, \
       document_id: $domain.document_id, document_version_id: $domain.document_version_id, \
       format: $domain.format, status: $domain.status, label: $domain.label, \
       manifest_json: $domain.manifest_json, receipt_json: $domain.receipt_json, \
       requested_by: $domain.requested_by })[0]; RETURN $created; };"
);
const FIND_EXPORT_STATEMENT: &str = concat!(
    "SELECT ",
    export_columns!(),
    " FROM atelier_moodboard_export_request WHERE snapshot_id = $snapshot_id \
      AND format = $format LIMIT 1;"
);
const LIST_EXPORTS_STATEMENT: &str = concat!(
    "SELECT ",
    export_columns!(),
    " FROM atelier_moodboard_export_request WHERE snapshot_id = $snapshot_id \
      ORDER BY created_at_utc ASC, format ASC;"
);

impl AtelierStore {
    async fn jsonb_text_sha256(&self, value: &Value) -> AtelierResult<String> {
        let bytes = serde_json::to_vec(value)
            .map_err(|err| AtelierError::Validation(format!("invalid JSON payload: {err}")))?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }

    async fn moodboard_snapshot_by_id(
        &self,
        snapshot_id: Uuid,
    ) -> AtelierResult<MoodboardSnapshot> {
        let binding = UuidBinding {
            id: SurrealUuid::from(snapshot_id),
        };
        let row: MoodboardSnapshotRow = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_SNAPSHOT_STATEMENT, binding).await })
            })
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("moodboard snapshot {snapshot_id}")))?;
        let doc_type = row.doc_type.clone().ok_or_else(|| {
            AtelierError::Internal(format!("moodboard snapshot {snapshot_id} has no document"))
        })?;
        let version_document_id = row.version_document_id.map(Into::into).ok_or_else(|| {
            AtelierError::Internal(format!(
                "moodboard snapshot {snapshot_id} has no version document"
            ))
        })?;
        let snapshot: MoodboardSnapshot = row.try_into()?;
        validate_snapshot_join_invariants(&snapshot, &doc_type, version_document_id)?;
        Ok(snapshot)
    }

    pub async fn record_moodboard_snapshot(
        &self,
        new: &NewMoodboardSnapshot,
    ) -> AtelierResult<MoodboardSnapshot> {
        let author = require_non_empty_trimmed("author", &new.author)?;
        let (moodboard_json, moodboard) = parse_and_validate_moodboard(&new.raw_json_text)?;
        let content_sha256 = sha256_hex(new.raw_json_text.as_bytes());

        let document_id = RecordId::new(
            "atelier_character_document",
            SurrealUuid::from(new.document_id),
        );
        let doc_binding = DocumentIdBinding {
            document_id: document_id.clone(),
        };
        let doc_row: DocumentHeadRow = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "SELECT doc_type, current_version_id FROM $document_id LIMIT 1;",
                        doc_binding,
                    )
                    .await
                })
            })
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("character document {}", new.document_id))
            })?;
        if CharacterDocumentType::from_token(&doc_row.doc_type)? != CharacterDocumentType::Moodboard
        {
            return Err(AtelierError::Validation(format!(
                "document {} must be a moodboard document",
                new.document_id
            )));
        }
        let document_version_link = doc_row.current_version_id.ok_or_else(|| {
            AtelierError::Validation(format!(
                "moodboard document {} has no current version",
                new.document_id
            ))
        })?;
        let document_version_id = uuid_from_record_link(
            "atelier_character_document.current_version_id",
            &document_version_link,
        )?;
        if let Some(expected_document_version_id) = new.expected_document_version_id {
            if document_version_id != expected_document_version_id {
                return Err(AtelierError::Conflict(format!(
                    "stale_moodboard_document_version: expected document version {:?}, current head {:?}",
                    expected_document_version_id, document_version_id
                )));
            }
        }
        #[derive(SurrealValue)]
        struct ExistingBinding {
            document_id: RecordId,
            document_version_id: RecordId,
            content_sha256: String,
        }
        let existing_binding = ExistingBinding {
            document_id: document_id.clone(),
            document_version_id: document_version_link.clone(),
            content_sha256: content_sha256.clone(),
        };
        let existing: Option<MoodboardSnapshotRow> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(FIND_SNAPSHOT_STATEMENT, existing_binding)
                        .await
                })
            })
            .await?;
        if let Some(existing) = existing {
            return existing.try_into();
        }

        let snapshot_id = Uuid::now_v7();
        let bindings = MoodboardSnapshotBindings {
            record_id: RecordId::new("atelier_moodboard", SurrealUuid::from(snapshot_id)),
            snapshot_id: SurrealUuid::from(snapshot_id),
            document_id,
            document_version_id: document_version_link,
            schema_id: MOODBOARD_SCHEMA_ID.to_owned(),
            schema_version: MOODBOARD_SCHEMA_VERSION,
            raw_json_text: new.raw_json_text.clone(),
            moodboard_json,
            content_sha256: content_sha256.clone(),
            author,
        };
        let row: Option<MoodboardSnapshotRow> = self
            .write_with_event(
                RECORD_SNAPSHOT_STATEMENT,
                bindings,
                moodboard_event_family::MOODBOARD_SNAPSHOT_RECORDED,
                "atelier_character_document",
                &new.document_id.to_string(),
                serde_json::json!({
                    "schema_id": MOODBOARD_SCHEMA_ID,
                    "schema_version": MOODBOARD_SCHEMA_VERSION,
                    "document_version_id_ref": event_ref_for_text(&document_version_id.to_string()),
                    "content_sha256": content_sha256,
                    "layer_count": moodboard.layers.len(),
                    "image_count": moodboard.images.len(),
                    "text_count": moodboard.text.len(),
                    "shape_count": moodboard.shapes.len(),
                    "connector_count": moodboard.connectors.len(),
                    "folder_count": moodboard.folders.len(),
                    "guide_count": moodboard.guides.len(),
                    "history_count": moodboard.history.len(),
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("recording a moodboard snapshot returned no row".to_owned())
        })?
        .try_into()
    }

    pub async fn latest_moodboard_snapshot(
        &self,
        document_id: Uuid,
    ) -> AtelierResult<Option<MoodboardSnapshot>> {
        let binding = DocumentIdBinding {
            document_id: RecordId::new(
                "atelier_character_document",
                SurrealUuid::from(document_id),
            ),
        };
        let row: Option<MoodboardSnapshotRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(LATEST_SNAPSHOT_STATEMENT, binding).await })
            })
            .await?;
        row.map(|row| {
            let doc_type = row.doc_type.clone().ok_or_else(|| {
                AtelierError::Internal("moodboard snapshot has no document".to_owned())
            })?;
            let version_document_id = row.version_document_id.map(Into::into).ok_or_else(|| {
                AtelierError::Internal("moodboard snapshot has no version document".to_owned())
            })?;
            let snapshot: MoodboardSnapshot = row.try_into()?;
            validate_snapshot_join_invariants(&snapshot, &doc_type, version_document_id)?;
            Ok(snapshot)
        })
        .transpose()
    }

    pub async fn record_moodboard_operation(
        &self,
        new: &NewMoodboardOperation,
    ) -> AtelierResult<MoodboardOperationReceipt> {
        let actor = require_non_empty_trimmed("actor", &new.actor)?;
        if !new.operation_payload.is_object() {
            return Err(AtelierError::Validation(
                "operation_payload must be a JSON object".into(),
            ));
        }
        let snapshot = self.moodboard_snapshot_by_id(new.snapshot_id).await?;
        let operation_id = Uuid::new_v4();
        let operation_payload_sha256 = self.jsonb_text_sha256(&new.operation_payload).await?;
        let receipt_json = build_operation_receipt_json(
            operation_id,
            &snapshot,
            new.operation_kind,
            &operation_payload_sha256,
            &actor,
        );

        let bindings = MoodboardOperationBindings {
            record_id: RecordId::new(
                "atelier_moodboard_operation_receipt",
                SurrealUuid::from(operation_id),
            ),
            operation_id: SurrealUuid::from(operation_id),
            snapshot_id: RecordId::new(
                "atelier_moodboard",
                SurrealUuid::from(snapshot.snapshot_id),
            ),
            document_id: RecordId::new(
                "atelier_character_document",
                SurrealUuid::from(snapshot.document_id),
            ),
            document_version_id: RecordId::new(
                "atelier_character_document_version",
                SurrealUuid::from(snapshot.document_version_id),
            ),
            operation_kind: new.operation_kind.as_token().to_owned(),
            operation_payload: new.operation_payload.clone(),
            operation_payload_sha256: operation_payload_sha256.clone(),
            receipt_json,
            actor,
        };
        let row: Option<MoodboardOperationReceiptRow> = self
            .write_with_event(
                RECORD_OPERATION_STATEMENT,
                bindings,
                moodboard_event_family::MOODBOARD_OPERATION_RECORDED,
                "atelier_moodboard",
                &snapshot.snapshot_id.to_string(),
                serde_json::json!({
                    "operation_id": operation_id,
                    "operation_kind": new.operation_kind.as_token(),
                    "operation_payload_sha256": operation_payload_sha256,
                    "document_id_ref": event_ref_for_text(&snapshot.document_id.to_string()),
                    "document_version_id_ref": event_ref_for_text(
                        &snapshot.document_version_id.to_string()
                    ),
                    "source_content_sha256": snapshot.content_sha256,
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("recording a moodboard operation returned no row".to_owned())
        })?
        .try_into()
    }

    pub async fn list_moodboard_operations(
        &self,
        snapshot_id: Uuid,
    ) -> AtelierResult<Vec<MoodboardOperationReceipt>> {
        let _ = self.moodboard_snapshot_by_id(snapshot_id).await?;
        let binding = SnapshotIdBinding {
            snapshot_id: RecordId::new("atelier_moodboard", SurrealUuid::from(snapshot_id)),
        };
        let rows: Vec<MoodboardOperationReceiptRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_OPERATIONS_STATEMENT, binding).await })
            })
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn request_moodboard_export(
        &self,
        new: &NewMoodboardExportRequest,
    ) -> AtelierResult<MoodboardExportRequest> {
        let requested_by = require_non_empty_trimmed("requested_by", &new.requested_by)?;
        let label = new
            .label
            .as_ref()
            .map(|value| require_non_empty_trimmed("label", value))
            .transpose()?;
        if let Some(label) = &label {
            if label.to_ascii_lowercase().contains(".gov") {
                return Err(AtelierError::Validation(
                    "moodboard export label must not reference .GOV outputs".into(),
                ));
            }
        }
        let snapshot = self.moodboard_snapshot_by_id(new.snapshot_id).await?;
        let export_id = Uuid::new_v4();
        let status = MoodboardExportStatus::Planned;
        let manifest_json = build_export_manifest_json(export_id, &snapshot, new.format);
        let receipt_json =
            build_export_receipt_json(export_id, &snapshot, new.format, &requested_by);
        let snapshot_link =
            RecordId::new("atelier_moodboard", SurrealUuid::from(snapshot.snapshot_id));
        let lookup = ExportLookupBinding {
            snapshot_id: snapshot_link.clone(),
            format: new.format.as_token().to_owned(),
        };
        let existing: Option<MoodboardExportRequestRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(FIND_EXPORT_STATEMENT, lookup).await })
            })
            .await?;
        if let Some(existing) = existing {
            return existing.try_into();
        }

        let bindings = MoodboardExportBindings {
            record_id: RecordId::new(
                "atelier_moodboard_export_request",
                SurrealUuid::from(export_id),
            ),
            export_id: SurrealUuid::from(export_id),
            snapshot_id: snapshot_link,
            document_id: RecordId::new(
                "atelier_character_document",
                SurrealUuid::from(snapshot.document_id),
            ),
            document_version_id: RecordId::new(
                "atelier_character_document_version",
                SurrealUuid::from(snapshot.document_version_id),
            ),
            format: new.format.as_token().to_owned(),
            status: status.as_token().to_owned(),
            label,
            manifest_json,
            receipt_json,
            requested_by,
        };
        let row: Option<MoodboardExportRequestRow> = self
            .write_with_event(
                RECORD_EXPORT_STATEMENT,
                bindings,
                moodboard_event_family::MOODBOARD_EXPORT_REQUESTED,
                "atelier_moodboard",
                &snapshot.snapshot_id.to_string(),
                serde_json::json!({
                    "export_id": export_id,
                    "format": new.format.as_token(),
                    "status": status.as_token(),
                    "document_id_ref": event_ref_for_text(&snapshot.document_id.to_string()),
                    "document_version_id_ref": event_ref_for_text(
                        &snapshot.document_version_id.to_string()
                    ),
                    "source_content_sha256": snapshot.content_sha256,
                    "output_artifact": "not_produced",
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("recording a moodboard export returned no row".to_owned())
        })?
        .try_into()
    }

    pub async fn list_moodboard_export_requests(
        &self,
        snapshot_id: Uuid,
    ) -> AtelierResult<Vec<MoodboardExportRequest>> {
        let _ = self.moodboard_snapshot_by_id(snapshot_id).await?;
        let binding = SnapshotIdBinding {
            snapshot_id: RecordId::new("atelier_moodboard", SurrealUuid::from(snapshot_id)),
        };
        let rows: Vec<MoodboardExportRequestRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_EXPORTS_STATEMENT, binding).await })
            })
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}
