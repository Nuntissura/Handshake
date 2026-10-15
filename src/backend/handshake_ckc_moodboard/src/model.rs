use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::schema::validate_against_schema;

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

pub fn require_non_empty_trimmed(field: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "{field} must not be empty"
        ));
    }
    Ok(trimmed.to_string())
}


pub fn parse_and_validate_moodboard(raw_json_text: &str) -> Result<(Value, MoodboardDocument), String> {
    require_non_empty_trimmed("raw_json_text", raw_json_text)?;
    let moodboard_json: Value = serde_json::from_str(raw_json_text).map_err(|err| {
        format!("moodboard JSON could not be parsed: {err}")
    })?;
    validate_against_schema(&moodboard_json)?;
    let moodboard: MoodboardDocument =
        serde_json::from_value(moodboard_json.clone()).map_err(|err| {
            format!("moodboard JSON did not match typed model: {err}")
        })?;
    validate_layer_model(&moodboard)?;
    Ok((moodboard_json, moodboard))
}

fn insert_uuid(
    ids: &mut HashSet<Uuid>,
    kind: &str,
    id: Uuid,
    parent: Option<Uuid>,
) -> Result<(), String> {
    if !ids.insert(id) {
        let parent_text = parent
            .map(|value| format!(" in {value}"))
            .unwrap_or_default();
        return Err(format!(
            "duplicate moodboard {kind} id {id}{parent_text}"
        ));
    }
    Ok(())
}

fn validate_layer_model(moodboard: &MoodboardDocument) -> Result<(), String> {
    let mut layer_ids = HashSet::new();
    for layer in &moodboard.layers {
        insert_uuid(&mut layer_ids, "layer", layer.layer_id, None)?;
    }
    for layer in &moodboard.layers {
        if let Some(parent_layer_id) = layer.parent_layer_id {
            if parent_layer_id == layer.layer_id {
                return Err(format!(
                    "moodboard layer {} cannot parent itself",
                    layer.layer_id
                ));
            }
            if !layer_ids.contains(&parent_layer_id) {
                return Err(format!(
                    "moodboard layer {} references missing parent layer {parent_layer_id}",
                    layer.layer_id
                ));
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
            return Err(format!(
                "moodboard connector {} references missing from_element_id {}",
                connector.connector_id, connector.from_element_id
            ));
        }
        if !element_ids.contains(&connector.to_element_id) {
            return Err(format!(
                "moodboard connector {} references missing to_element_id {}",
                connector.connector_id, connector.to_element_id
            ));
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
                return Err(format!(
                    "moodboard folder {} references missing child {child}",
                    folder.folder_id
                ));
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
) -> Result<(), String> {
    if !layer_ids.contains(&layer_id) {
        return Err(format!(
            "moodboard {kind} {element_id} references missing layer {layer_id}"
        ));
    }
    Ok(())
}
