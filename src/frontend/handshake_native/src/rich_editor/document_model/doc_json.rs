//! DocJson: serialize/deserialize the typed block model to/from the Tiptap
//! JSONContent shape used by the backend `knowledge/documents.content_json` field
//! (WP-KERNEL-012 MT-011).
//!
//! This is the LOAD-BEARING compatibility contract (MT impl note: anchored to the
//! REAL React/backend shape, read from `app/src/lib/editor/*`). The on-the-wire
//! shape is exactly what the React Tiptap editor produces:
//!
//! ```json
//! { "type": "doc", "content": [
//!     { "type": "heading", "attrs": { "level": 1 }, "content": [ { "type": "text", "text": "Title" } ] },
//!     { "type": "paragraph", "content": [
//!         { "type": "text", "text": "Bold ", "marks": [ { "type": "bold" } ] },
//!         { "type": "text", "text": "link", "marks": [ { "type": "link", "attrs": { "href": "https://x" } } ] }
//!     ] }
//! ] }
//! ```
//!
//! Round-trip rules (red-team RISK-3 — never silently drop attrs):
//! - `attrs` is serialized as an `"attrs"` object of `serde_json::Value` (NOT
//!   `#[serde(flatten)]`, which loses type info — MT impl note 4). Unknown attrs
//!   survive verbatim.
//! - `marks` is a `"marks"` array of `{ "type": ..., "attrs"?: {...} }`. `link`
//!   carries `attrs.href`; `wikilink` carries `attrs.{kind,value,label}`.
//! - The doc-level [`RichDocument`] envelope carries `schema_version`
//!   ([`RICH_DOCUMENT_SCHEMA_VERSION`] = `"rich_document_v1"`), matching the React
//!   `WP009_RICH_DOCUMENT_SCHEMA_VERSION` so the backend accepts the document on
//!   first save (RISK-5). A version bump is a one-line change to the const
//!   (MT impl note: import from a single const).
//!
//! The serde model uses an INTERMEDIATE typed JSON struct (`JsonNode`) rather than a
//! hand-rolled `serde_json::Value` walk, so both the write and read paths are the
//! same shape (CODER_RUBRIC dimension 4: end-to-end integrity — inspect both
//! write and read paths for serialized shapes).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use thiserror::Error;

use super::node::{BlockNode, Child, HeadingLevel, Mark, NodeKind, TextLeaf};

/// The single source of the rich-document schema version. A bump is a one-line edit
/// here (MT impl note). MUST equal the React `WP009_RICH_DOCUMENT_SCHEMA_VERSION`
/// (`"rich_document_v1"`) so the backend round-trips the document (RISK-5).
pub const RICH_DOCUMENT_SCHEMA_VERSION: &str = "rich_document_v1";

/// Why a DocJson (de)serialization failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DocJsonError {
    /// The top-level node was not a `doc`.
    #[error("expected a top-level 'doc' node, found {found:?}")]
    NotADoc { found: Option<String> },
    /// An unknown node `type` string with no [`NodeKind`] mapping.
    #[error("unknown node type {0:?}")]
    UnknownNodeType(String),
    /// An unknown mark `type` string with no [`Mark`] mapping.
    #[error("unknown mark type {0:?}")]
    UnknownMarkType(String),
    /// A link mark was missing its required `href` attr.
    #[error("link mark missing href attr")]
    LinkMissingHref,
    /// A wikilink mark was missing a required attr (`kind` or `value`).
    #[error("wikilink mark missing required attr: {0}")]
    WikilinkMissingAttr(&'static str),
    /// The JSON text could not be parsed at all.
    #[error("invalid JSON: {0}")]
    Parse(String),
}

/// The doc-level envelope: a `schema_version` plus the root `doc` node. This is the
/// shape stored in `RichDocument.content_json` (the backend stamps the version, but
/// the editor produces a matching one so a fresh save is accepted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RichDocument {
    /// The schema version (always [`RICH_DOCUMENT_SCHEMA_VERSION`] on serialize).
    pub schema_version: String,
    /// The root `doc` node and its content.
    #[serde(flatten)]
    pub doc: JsonNode,
}

/// The serde wire representation of one node (block or text). This is the Tiptap
/// JSONContent shape. `type`/`attrs`/`content`/`text`/`marks` are all optional so a
/// text node (no `content`/`attrs`) and a block node (no `text`/`marks`) share one
/// struct, matching the `JSONContentLike` type in `app/src/lib/api.ts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonNode {
    /// The node/mark `type` string (camelCase Tiptap name).
    #[serde(rename = "type")]
    pub ty: String,
    /// Node attributes (heading level, code-block language, task checked, …).
    /// Serialized as `"attrs"` object; omitted when empty (MT impl note 4).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub attrs: Option<Map<String, JsonValue>>,
    /// Block content (child nodes). Omitted on text nodes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content: Option<Vec<JsonNode>>,
    /// Text payload (present only on text nodes).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub text: Option<String>,
    /// Marks on a text node. Omitted when none.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub marks: Option<Vec<JsonMark>>,
}

/// The serde wire representation of one mark: `{ "type": ..., "attrs"?: {...} }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonMark {
    /// The mark `type` string.
    #[serde(rename = "type")]
    pub ty: String,
    /// Mark attributes (`href` for link; `kind`/`value`/`label` for wikilink).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub attrs: Option<Map<String, JsonValue>>,
}

// ---- model -> JSON ----------------------------------------------------------

/// Serialize a `doc` [`BlockNode`] to a [`RichDocument`] envelope (the round-trip
/// entry point used by tests + the backend bridge). The doc's `attrs` and structure
/// are preserved verbatim.
pub fn to_rich_document(doc: &BlockNode) -> RichDocument {
    RichDocument {
        schema_version: RICH_DOCUMENT_SCHEMA_VERSION.to_string(),
        doc: block_to_json(doc),
    }
}

/// Serialize a `doc` [`BlockNode`] to a pretty JSON `String` envelope.
pub fn to_json_string(doc: &BlockNode) -> Result<String, DocJsonError> {
    serde_json::to_string(&to_rich_document(doc)).map_err(|e| DocJsonError::Parse(e.to_string()))
}

/// Serialize a [`BlockNode`] to its bare [`JsonNode`] (no envelope) — used for
/// `serde_json::Value` shape assertions in the acceptance tests.
pub fn block_to_json(node: &BlockNode) -> JsonNode {
    let attrs = node_attrs_map(node);
    if node.kind.is_atom() {
        return JsonNode {
            ty: node.kind.to_json_type().to_string(),
            attrs,
            content: None,
            text: None,
            marks: None,
        };
    }
    let content: Vec<JsonNode> = node.children.iter().map(child_to_json).collect();
    JsonNode {
        ty: node.kind.to_json_type().to_string(),
        attrs,
        content: Some(content),
        text: None,
        marks: None,
    }
}

/// Build the `attrs` map for a node: the well-known typed attrs (`level` for a
/// heading) are stamped from the typed kind, then any free-form attrs are layered on
/// (free-form wins for forward-compat keys, except the typed `level` which the kind
/// owns). Returns `None` when empty so `attrs` is omitted from the JSON.
fn node_attrs_map(node: &BlockNode) -> Option<Map<String, JsonValue>> {
    let mut map = Map::new();
    // Free-form attrs first (so a forward-compat key survives — RISK-3)…
    for (k, v) in &node.attrs {
        map.insert(k.clone(), v.clone());
    }
    // …then stamp the typed level so a heading always carries the correct level
    // even if the free-form attrs lacked it.
    if let NodeKind::Heading(level) = node.kind {
        map.insert("level".to_string(), JsonValue::from(level.get()));
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

/// Serialize a [`Child`] (block or text leaf) to a [`JsonNode`].
fn child_to_json(child: &Child) -> JsonNode {
    match child {
        Child::Block(b) => block_to_json(b),
        Child::Text(leaf) => text_leaf_to_json(leaf),
    }
}

/// Serialize a [`TextLeaf`] to a `{type:"text", text:..., marks?:[...]}` node.
fn text_leaf_to_json(leaf: &TextLeaf) -> JsonNode {
    let marks: Vec<JsonMark> = leaf.marks.iter().map(mark_to_json).collect();
    JsonNode {
        ty: "text".to_string(),
        attrs: None,
        content: None,
        text: Some(leaf.text.to_string()),
        marks: if marks.is_empty() { None } else { Some(marks) },
    }
}

/// Serialize a [`Mark`] to a `{type, attrs?}` wire mark, carrying typed payloads.
fn mark_to_json(mark: &Mark) -> JsonMark {
    match mark {
        Mark::Link { href } => {
            let mut attrs = Map::new();
            attrs.insert("href".to_string(), JsonValue::from(href.clone()));
            JsonMark {
                ty: "link".to_string(),
                attrs: Some(attrs),
            }
        }
        Mark::Wikilink { kind, value, label } => {
            let mut attrs = Map::new();
            attrs.insert("kind".to_string(), JsonValue::from(kind.clone()));
            attrs.insert("value".to_string(), JsonValue::from(value.clone()));
            if let Some(label) = label {
                attrs.insert("label".to_string(), JsonValue::from(label.clone()));
            }
            JsonMark {
                ty: "wikilink".to_string(),
                attrs: Some(attrs),
            }
        }
        other => JsonMark {
            ty: other.json_type().to_string(),
            attrs: None,
        },
    }
}

// ---- JSON -> model ----------------------------------------------------------

/// Parse a JSON `String` envelope back into a `doc` [`BlockNode`]. Accepts the
/// [`RichDocument`] envelope (with `schema_version`); the bare-doc form is accepted
/// too (the `schema_version` is then assumed current).
pub fn from_json_string(json: &str) -> Result<BlockNode, DocJsonError> {
    let value: JsonValue =
        serde_json::from_str(json).map_err(|e| DocJsonError::Parse(e.to_string()))?;
    from_json_value(&value)
}

/// Parse a `serde_json::Value` envelope/bare-doc into a `doc` [`BlockNode`].
pub fn from_json_value(value: &JsonValue) -> Result<BlockNode, DocJsonError> {
    let node: JsonNode =
        serde_json::from_value(value.clone()).map_err(|e| DocJsonError::Parse(e.to_string()))?;
    let block = json_to_block(&node)?;
    if !matches!(block.kind, NodeKind::Doc) {
        return Err(DocJsonError::NotADoc {
            found: Some(node.ty.clone()),
        });
    }
    Ok(block)
}

/// Deserialize a [`JsonNode`] into a block [`BlockNode`]. The caller guarantees this
/// is a block node (a text node is handled in [`json_to_child`]).
fn json_to_block(node: &JsonNode) -> Result<BlockNode, DocJsonError> {
    let heading_level = node
        .attrs
        .as_ref()
        .and_then(|m| m.get("level"))
        .and_then(JsonValue::as_u64)
        .map(|l| l as u8)
        .unwrap_or(1);
    let kind = NodeKind::from_json_type(&node.ty, heading_level)
        .ok_or_else(|| DocJsonError::UnknownNodeType(node.ty.clone()))?;

    // Preserve free-form attrs verbatim (RISK-3) EXCEPT the typed `level`, which the
    // kind already owns (and `to_json` re-stamps), so we don't duplicate it.
    let mut attrs = std::collections::HashMap::new();
    if let Some(map) = &node.attrs {
        for (k, v) in map {
            if matches!(kind, NodeKind::Heading(_)) && k == "level" {
                continue;
            }
            attrs.insert(k.clone(), v.clone());
        }
    }

    let mut children = Vec::new();
    if let Some(content) = &node.content {
        for c in content {
            children.push(json_to_child(c)?);
        }
    }
    Ok(BlockNode {
        kind,
        attrs,
        children,
    })
}

/// Deserialize a [`JsonNode`] into a [`Child`]: a `"text"` node becomes a text leaf,
/// anything else a nested block.
fn json_to_child(node: &JsonNode) -> Result<Child, DocJsonError> {
    if node.ty == "text" {
        let text = node.text.clone().unwrap_or_default();
        let mut marks = Vec::new();
        if let Some(ms) = &node.marks {
            for m in ms {
                marks.push(json_to_mark(m)?);
            }
        }
        Ok(Child::Text(TextLeaf::with_marks(&text, marks)))
    } else {
        Ok(Child::Block(json_to_block(node)?))
    }
}

/// Deserialize a [`JsonMark`] into a [`Mark`], reading the typed payloads.
fn json_to_mark(mark: &JsonMark) -> Result<Mark, DocJsonError> {
    Ok(match mark.ty.as_str() {
        "bold" => Mark::Bold,
        "italic" => Mark::Italic,
        "underline" => Mark::Underline,
        "strike" => Mark::Strike,
        "code" => Mark::Code,
        "link" => {
            let href = mark
                .attrs
                .as_ref()
                .and_then(|m| m.get("href"))
                .and_then(JsonValue::as_str)
                .ok_or(DocJsonError::LinkMissingHref)?
                .to_string();
            Mark::Link { href }
        }
        "wikilink" => {
            let attrs = mark.attrs.as_ref();
            let kind = attrs
                .and_then(|m| m.get("kind"))
                .and_then(JsonValue::as_str)
                .ok_or(DocJsonError::WikilinkMissingAttr("kind"))?
                .to_string();
            let value = attrs
                .and_then(|m| m.get("value"))
                .and_then(JsonValue::as_str)
                .ok_or(DocJsonError::WikilinkMissingAttr("value"))?
                .to_string();
            let label = attrs
                .and_then(|m| m.get("label"))
                .and_then(JsonValue::as_str)
                .map(|s| s.to_string());
            Mark::Wikilink { kind, value, label }
        }
        other => return Err(DocJsonError::UnknownMarkType(other.to_string())),
    })
}

/// Keep [`HeadingLevel`] reachable for downstream MTs constructing headings from
/// JSON (re-exported via the module). Marker use to avoid an unused-import warning
/// while documenting the dependency.
#[allow(dead_code)]
fn _heading_level_is_used(level: u8) -> HeadingLevel {
    HeadingLevel::new(level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::node::{BlockNode, Child, Mark, NodeKind, TextLeaf};

    fn rich_doc() -> BlockNode {
        // doc > [ heading(1,"Title"), paragraph("Bold italic", with bold+italic) ]
        let mut para = BlockNode::new(NodeKind::Paragraph);
        para.children
            .push(Child::Text(TextLeaf::with_marks("Bold ", vec![Mark::Bold])));
        para.children
            .push(Child::Text(TextLeaf::with_marks("italic", vec![Mark::Italic])));
        BlockNode::doc(vec![BlockNode::heading(1, "Title"), para])
    }

    #[test]
    fn round_trip_equals_original() {
        let doc = rich_doc();
        let json = to_json_string(&doc).unwrap();
        let back = from_json_string(&json).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn schema_version_is_rich_document_v1() {
        assert_eq!(RICH_DOCUMENT_SCHEMA_VERSION, "rich_document_v1");
        let env = to_rich_document(&rich_doc());
        assert_eq!(env.schema_version, "rich_document_v1");
    }

    #[test]
    fn json_shape_matches_tiptap() {
        let doc = rich_doc();
        let v: JsonValue = serde_json::to_value(to_rich_document(&doc)).unwrap();
        assert_eq!(v["schema_version"], "rich_document_v1");
        assert_eq!(v["type"], "doc");
        // First child: heading with attrs.level == 1.
        assert_eq!(v["content"][0]["type"], "heading");
        assert_eq!(v["content"][0]["attrs"]["level"], 1);
        assert_eq!(v["content"][0]["content"][0]["type"], "text");
        assert_eq!(v["content"][0]["content"][0]["text"], "Title");
        // Second child: paragraph with a bold text run.
        assert_eq!(v["content"][1]["type"], "paragraph");
        assert_eq!(v["content"][1]["content"][0]["text"], "Bold ");
        assert_eq!(v["content"][1]["content"][0]["marks"][0]["type"], "bold");
    }

    #[test]
    fn link_href_survives_round_trip() {
        let mut para = BlockNode::new(NodeKind::Paragraph);
        para.children.push(Child::Text(TextLeaf::with_marks(
            "click",
            vec![Mark::Link {
                href: "https://example.com/docs".to_string(),
            }],
        )));
        let doc = BlockNode::doc(vec![para]);
        let json = to_json_string(&doc).unwrap();
        let back = from_json_string(&json).unwrap();
        assert_eq!(doc, back);
        // The href is present in the JSON.
        let v: JsonValue = serde_json::to_value(to_rich_document(&doc)).unwrap();
        assert_eq!(
            v["content"][0]["content"][0]["marks"][0]["attrs"]["href"],
            "https://example.com/docs"
        );
    }

    #[test]
    fn wikilink_payload_survives_round_trip() {
        let mut para = BlockNode::new(NodeKind::Paragraph);
        para.children.push(Child::Text(TextLeaf::with_marks(
            "the WP",
            vec![Mark::Wikilink {
                kind: "wp".to_string(),
                value: "WP-KERNEL-012".to_string(),
                label: Some("the WP".to_string()),
            }],
        )));
        let doc = BlockNode::doc(vec![para]);
        let json = to_json_string(&doc).unwrap();
        let back = from_json_string(&json).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn unknown_attr_survives_round_trip() {
        // A forward-compat attr the schema does not know must NOT be dropped (RISK-3).
        let mut para = BlockNode::new(NodeKind::Paragraph);
        para.attrs
            .insert("data-future".to_string(), JsonValue::from("keepme"));
        para.children.push(Child::Text(TextLeaf::new("x")));
        let doc = BlockNode::doc(vec![para]);
        let json = to_json_string(&doc).unwrap();
        let back = from_json_string(&json).unwrap();
        assert_eq!(
            back.children[0].as_block().unwrap().attrs.get("data-future"),
            Some(&JsonValue::from("keepme"))
        );
    }

    #[test]
    fn unknown_node_type_errors() {
        let json = r#"{"schema_version":"rich_document_v1","type":"doc","content":[{"type":"bogus"}]}"#;
        assert!(matches!(
            from_json_string(json),
            Err(DocJsonError::UnknownNodeType(_))
        ));
    }
}
