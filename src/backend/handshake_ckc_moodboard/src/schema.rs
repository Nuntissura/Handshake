use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

use crate::{MOODBOARD_SCHEMA_ID, MOODBOARD_SCHEMA_VERSION};

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

pub(crate) fn validate_against_schema(document: &Value) -> Result<(), String> {
    let schema = moodboard_json_schema();
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .map_err(|err| {
            format!(
                "invalid moodboard Draft 2020-12 JSON Schema: {err}"
            )
        })?;

    if let Err(errors) = compiled.validate(document) {
        let details = errors
            .take(5)
            .map(|err| format!("{} at {}", err, err.instance_path))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "moodboard JSON failed Draft 2020-12 validation: {details}"
        ));
    }
    Ok(())
}
