//! MT-146 RichBlockTreeModel + MT-147 RawDerivedDisplaySeparation + MT-148
//! BlockStableIdStrategy.
//!
//! The canonical structured block-tree model behind a RichDocument. A
//! RichDocument's authority is its ProseMirror/Tiptap `content_json` (spec
//! 2.3.13.11 / 7.1.1.8). This module is the typed, deterministic view OVER that
//! JSON:
//!
//! * [`BlockKind`] enumerates every supported block type (MT-146): paragraph,
//!   heading, list (bullet/ordered/task), quote, code, table, image, video,
//!   album, slideshow, and the typed link blocks file/folder/project/spec/wp/
//!   symbol.
//! * [`RawDerivedDisplay`] (MT-147, CX-100) carries the three separated layers
//!   on every block: RAW authority content, DERIVED (summaries/previews
//!   regenerated from raw), and DISPLAY (display-only UI hints). Only RAW is
//!   authority; derived and display are regenerable and never override raw.
//! * Stable block ids (MT-148): every block carries a stable `block_id`. When a
//!   block already has one (from a prior load / CRDT), it is preserved; when a
//!   block is new it is derived deterministically from its document id, its
//!   position path, and its raw content so a re-parse of the same document
//!   yields the same id (used by CRDT refs, citations, backlinks, and
//!   visual-debug selectors).
//!
//! Determinism contract: `BlockTree::from_document_json` followed by
//! `BlockTree::to_document_json` is a stable round-trip for the supported node
//! set, and stable block ids survive the round-trip. This underpins MT-149
//! deterministic save/load and MT-159 crash recovery.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::kernel::context_bundle::sha256_hex;

/// The WP-009 RichDocument schema-version token this model targets. Mirrors the
/// frontend `WP009_RICH_DOCUMENT_SCHEMA_VERSION` (`rich_document_v1`,
/// app/src/lib/tiptap/extension_set.ts). A document whose `schema_version`
/// differs is a schema-migration signal (spec 7.1.1.8).
pub const DOCUMENT_SCHEMA_VERSION: &str = "rich_document_v1";

/// Errors raised parsing or validating a document block tree.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BlockTreeError {
    #[error("document json root must be a ProseMirror doc node with type=\"doc\"")]
    NotADocNode,
    #[error("document json `content` must be an array of block nodes")]
    ContentNotArray,
    #[error("block node at {path} is missing a string `type`")]
    BlockMissingType { path: String },
    #[error("block node at {path} has unsupported type `{kind}`")]
    UnsupportedBlockType { path: String, kind: String },
    #[error("block node at {path} has a malformed `{field}` attribute")]
    MalformedAttr { path: String, field: String },
}

/// Every supported block type (MT-146).
///
/// The string token is the ProseMirror/Tiptap node `type` name (so the model is
/// a faithful view of the editor JSON). Typed link blocks are modeled as a
/// single `*_link` node type carrying a typed target attr — the editor renders
/// them, the backlink bridge (MT-155) reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Paragraph,
    Heading,
    BulletList,
    OrderedList,
    TaskList,
    Blockquote,
    CodeBlock,
    Table,
    Image,
    Video,
    Album,
    Slideshow,
    FileLink,
    FolderLink,
    ProjectLink,
    SpecLink,
    WpLink,
    SymbolLink,
}

impl BlockKind {
    /// The canonical ProseMirror node `type` token for this block kind.
    pub fn as_node_type(self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::Heading => "heading",
            Self::BulletList => "bulletList",
            Self::OrderedList => "orderedList",
            Self::TaskList => "taskList",
            Self::Blockquote => "blockquote",
            Self::CodeBlock => "codeBlock",
            Self::Table => "table",
            Self::Image => "image",
            Self::Video => "video",
            Self::Album => "album",
            Self::Slideshow => "slideshow",
            Self::FileLink => "fileLink",
            Self::FolderLink => "folderLink",
            Self::ProjectLink => "projectLink",
            Self::SpecLink => "specLink",
            Self::WpLink => "wpLink",
            Self::SymbolLink => "symbolLink",
        }
    }

    /// Parse a ProseMirror node `type` token into a known block kind.
    pub fn from_node_type(node_type: &str) -> Option<Self> {
        Some(match node_type {
            "paragraph" => Self::Paragraph,
            "heading" => Self::Heading,
            "bulletList" => Self::BulletList,
            "orderedList" => Self::OrderedList,
            "taskList" => Self::TaskList,
            "blockquote" => Self::Blockquote,
            "codeBlock" => Self::CodeBlock,
            "table" => Self::Table,
            "image" => Self::Image,
            "video" => Self::Video,
            "album" => Self::Album,
            "slideshow" => Self::Slideshow,
            "fileLink" => Self::FileLink,
            "folderLink" => Self::FolderLink,
            "projectLink" => Self::ProjectLink,
            "specLink" => Self::SpecLink,
            "wpLink" => Self::WpLink,
            "symbolLink" => Self::SymbolLink,
            _ => return None,
        })
    }

    /// Whether this block kind is one of the typed link blocks the backlink
    /// bridge (MT-155) reads.
    pub fn is_typed_link(self) -> bool {
        matches!(
            self,
            Self::FileLink
                | Self::FolderLink
                | Self::ProjectLink
                | Self::SpecLink
                | Self::WpLink
                | Self::SymbolLink
        )
    }

    /// Whether this block kind is an embed (carries an external/media/artifact
    /// target the embed-reference model owns, MT-152).
    pub fn is_embed(self) -> bool {
        matches!(
            self,
            Self::Image | Self::Video | Self::Album | Self::Slideshow
        )
    }
}

/// The Raw / Derived / Display separation carried on every block (MT-147,
/// CX-100).
///
/// * `raw` is the AUTHORITY: the canonical ProseMirror node JSON for this
///   block. Saving/loading/projecting all flow FROM raw.
/// * `derived` is a regenerable summary/preview (e.g. extracted plain text,
///   word count, first-line title) computed FROM raw. It must never be treated
///   as authority and is recomputed on parse.
/// * `display` is display-only UI hints (e.g. fold/collapse state, a render
///   class). It carries no authority and is preserved verbatim if present in
///   the node `attrs.display`, else empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawDerivedDisplay {
    /// AUTHORITY: the canonical ProseMirror node JSON for this block.
    pub raw: Value,
    /// REGENERABLE: derived summary/preview computed from `raw`.
    pub derived: DerivedBlockSummary,
    /// DISPLAY-ONLY: UI hints; no authority.
    pub display: Value,
}

/// A regenerable derived summary/preview of a block (never authority, MT-147).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedBlockSummary {
    /// Extracted plain text of the block (for previews, search, projections).
    pub plain_text: String,
    /// Word count over `plain_text`.
    pub word_count: usize,
    /// First non-empty line, capped — a one-line preview/title.
    pub preview: String,
}

impl DerivedBlockSummary {
    /// Derive the summary from a block's plain text (recomputed on every parse;
    /// never authority).
    fn from_plain_text(plain_text: String) -> Self {
        let word_count = plain_text.split_whitespace().count();
        let preview = plain_text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("")
            .chars()
            .take(120)
            .collect();
        Self {
            plain_text,
            word_count,
            preview,
        }
    }
}

/// A single block in the document block tree (MT-146/147/148).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Stable block id (MT-148). Preserved across loads/round-trips; used by
    /// CRDT refs, citations, backlinks, and visual-debug selectors.
    pub block_id: String,
    /// Typed block kind (MT-146).
    pub kind: BlockKind,
    /// Heading level for [`BlockKind::Heading`], else `None`.
    pub heading_level: Option<i64>,
    /// Zero-based position of this block among the document's top-level blocks.
    pub sequence: usize,
    /// Raw / Derived / Display separation (MT-147).
    pub content: RawDerivedDisplay,
}

/// The canonical structured block tree of a RichDocument (MT-146).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockTree {
    /// The rich document id this tree belongs to (KRD-...), used to seed stable
    /// block-id derivation (MT-148).
    pub rich_document_id: String,
    /// Schema version of the document JSON this tree was parsed from.
    pub schema_version: String,
    /// Top-level blocks in document order.
    pub blocks: Vec<Block>,
}

impl BlockTree {
    /// Parse a RichDocument's `content_json` (a ProseMirror doc node) into the
    /// typed block tree (MT-146), preserving Raw/Derived/Display separation
    /// (MT-147) and assigning/preserving stable block ids (MT-148).
    ///
    /// `rich_document_id` seeds deterministic id derivation for blocks that do
    /// not already carry a stable id.
    pub fn from_document_json(
        rich_document_id: &str,
        schema_version: &str,
        content_json: &Value,
    ) -> Result<Self, BlockTreeError> {
        let obj = content_json
            .as_object()
            .ok_or(BlockTreeError::NotADocNode)?;
        if obj.get("type").and_then(Value::as_str) != Some("doc") {
            return Err(BlockTreeError::NotADocNode);
        }
        let content = match obj.get("content") {
            None => &Value::Array(Vec::new()),
            Some(value) => value,
        };
        let nodes = content.as_array().ok_or(BlockTreeError::ContentNotArray)?;

        let mut blocks = Vec::with_capacity(nodes.len());
        for (index, node) in nodes.iter().enumerate() {
            blocks.push(parse_block(rich_document_id, index, node)?);
        }
        Ok(Self {
            rich_document_id: rich_document_id.to_string(),
            schema_version: schema_version.to_string(),
            blocks,
        })
    }

    /// Re-serialize the block tree back to a ProseMirror doc node JSON
    /// (MT-146). The raw block JSON is authority and is emitted verbatim, so
    /// `from_document_json` -> `to_document_json` is a stable round-trip for the
    /// supported node set (MT-149 deterministic save/load).
    pub fn to_document_json(&self) -> Value {
        let content: Vec<Value> = self
            .blocks
            .iter()
            .map(|block| block.content.raw.clone())
            .collect();
        json!({ "type": "doc", "content": content })
    }

    /// All stable block ids in document order (MT-148).
    pub fn block_ids(&self) -> Vec<String> {
        self.blocks.iter().map(|b| b.block_id.clone()).collect()
    }

    /// Concatenated plain text of every block (regenerable; never authority).
    /// Used by projections (MT-150) and the search-index bridge (MT-154).
    pub fn plain_text(&self) -> String {
        self.blocks
            .iter()
            .map(|b| b.content.derived.plain_text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Whether the parsed schema version matches the model's target version.
    /// A mismatch is a schema-migration signal (spec 7.1.1.8), surfaced to the
    /// caller rather than silently coerced.
    pub fn schema_matches(&self) -> bool {
        self.schema_version == DOCUMENT_SCHEMA_VERSION
    }
}

/// Derive a stable block id deterministically (MT-148).
///
/// `KBL-` + sha256_hex over a length-prefixed, injective preimage of the
/// document id, the block's position path, and its raw plain text. Length
/// prefixing makes the preimage injective so no separator inside any component
/// can alias two distinct blocks (same discipline as
/// `derive_knowledge_relationship_id`). Re-parsing the same document yields the
/// same id, so CRDT refs / citations / backlinks stay stable across loads.
pub fn derive_block_id(rich_document_id: &str, position_path: &str, plain_text: &str) -> String {
    use std::fmt::Write as _;
    let mut canonical = String::from("knowledge_block_id_v1");
    for component in [rich_document_id, position_path, plain_text] {
        let _ = write!(canonical, "|{}:{}", component.len(), component);
    }
    format!("KBL-{}", sha256_hex(canonical.as_bytes()))
}

/// Parse one top-level block node into a typed [`Block`].
fn parse_block(
    rich_document_id: &str,
    index: usize,
    node: &Value,
) -> Result<Block, BlockTreeError> {
    let path = format!("content[{index}]");
    let obj = node
        .as_object()
        .ok_or_else(|| BlockTreeError::BlockMissingType { path: path.clone() })?;
    let node_type = obj
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| BlockTreeError::BlockMissingType { path: path.clone() })?;
    let kind = BlockKind::from_node_type(node_type).ok_or_else(|| {
        BlockTreeError::UnsupportedBlockType {
            path: path.clone(),
            kind: node_type.to_string(),
        }
    })?;

    let attrs = obj.get("attrs").and_then(Value::as_object);
    let heading_level = if kind == BlockKind::Heading {
        Some(
            attrs
                .and_then(|a| a.get("level"))
                .and_then(Value::as_i64)
                .unwrap_or(1),
        )
    } else {
        None
    };

    // Display-only hints live in `attrs.display`; preserved verbatim, never
    // authority. Everything else in the node is the raw authority.
    let display = attrs
        .and_then(|a| a.get("display"))
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));

    let plain_text = extract_plain_text(node);
    let derived = DerivedBlockSummary::from_plain_text(plain_text.clone());

    // Stable id: preserve an existing `attrs.block_id`, else derive
    // deterministically from (doc id, position, raw text) (MT-148).
    let block_id = attrs
        .and_then(|a| a.get("block_id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| derive_block_id(rich_document_id, &path, &plain_text));

    Ok(Block {
        block_id,
        kind,
        heading_level,
        sequence: index,
        content: RawDerivedDisplay {
            raw: node.clone(),
            derived,
            display,
        },
    })
}

/// Recursively extract plain text from a ProseMirror node (regenerable derived
/// content, MT-147). Text nodes contribute their `text`; block separators join
/// with newlines; inline content joins with spaces.
pub fn extract_plain_text(node: &Value) -> String {
    let Some(obj) = node.as_object() else {
        return String::new();
    };
    if obj.get("type").and_then(Value::as_str) == Some("text") {
        return obj
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }
    let node_type = obj.get("type").and_then(Value::as_str).unwrap_or("");
    let Some(children) = obj.get("content").and_then(Value::as_array) else {
        return String::new();
    };
    let sep = match node_type {
        "listItem" | "bulletList" | "orderedList" | "taskList" | "taskItem" | "tableRow"
        | "table" => "\n",
        _ => " ",
    };
    children
        .iter()
        .map(extract_plain_text)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(sep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "Title" }] },
                { "type": "paragraph", "content": [{ "type": "text", "text": "Hello world" }] },
                { "type": "codeBlock", "content": [{ "type": "text", "text": "let x = 1;" }] },
                { "type": "fileLink", "attrs": { "target": "src/main.rs" } }
            ]
        })
    }

    #[test]
    fn parses_every_supported_block_kind() {
        for node_type in [
            "paragraph",
            "heading",
            "bulletList",
            "orderedList",
            "taskList",
            "blockquote",
            "codeBlock",
            "table",
            "image",
            "video",
            "album",
            "slideshow",
            "fileLink",
            "folderLink",
            "projectLink",
            "specLink",
            "wpLink",
            "symbolLink",
        ] {
            assert!(
                BlockKind::from_node_type(node_type).is_some(),
                "block kind `{node_type}` must be supported (MT-146)"
            );
        }
        // Round-trips node-type -> kind -> node-type.
        for kind in [
            BlockKind::Paragraph,
            BlockKind::Heading,
            BlockKind::Image,
            BlockKind::WpLink,
        ] {
            assert_eq!(BlockKind::from_node_type(kind.as_node_type()), Some(kind));
        }
    }

    #[test]
    fn from_to_document_json_is_a_stable_roundtrip() {
        // MT-146/149: parse then re-serialize reproduces the doc node exactly
        // (raw block JSON is authority, emitted verbatim).
        let doc = sample();
        let tree = BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        assert_eq!(tree.blocks.len(), 4);
        assert_eq!(tree.to_document_json(), doc);
    }

    #[test]
    fn raw_derived_display_are_separated() {
        // MT-147: raw is authority; derived is regenerable; display is hints.
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "paragraph",
                  "attrs": { "display": { "fold": true } },
                  "content": [{ "type": "text", "text": "one two three" }] }
            ]
        });
        let tree = BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        let block = &tree.blocks[0];
        // raw is the verbatim node.
        assert_eq!(block.content.raw, doc["content"][0]);
        // derived is recomputed (never authority).
        assert_eq!(block.content.derived.plain_text, "one two three");
        assert_eq!(block.content.derived.word_count, 3);
        // display hints preserved verbatim.
        assert_eq!(block.content.display, json!({ "fold": true }));
    }

    #[test]
    fn stable_block_id_is_deterministic_and_preserved() {
        // MT-148: same (doc, position, text) derives the same id; an existing
        // attrs.block_id is preserved over derivation.
        let doc = sample();
        let a = BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        let b = BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        assert_eq!(a.block_ids(), b.block_ids());
        assert!(a.block_ids().iter().all(|id| id.starts_with("KBL-")));
        // A different document id changes the derived ids.
        let c = BlockTree::from_document_json("KRD-y", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        assert_ne!(a.block_ids(), c.block_ids());

        // An explicit block_id is preserved.
        let doc_with_id = json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "attrs": { "block_id": "KBL-fixed" },
                  "content": [{ "type": "text", "text": "x" }] }
            ]
        });
        let tree =
            BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &doc_with_id).unwrap();
        assert_eq!(tree.blocks[0].block_id, "KBL-fixed");
    }

    #[test]
    fn malformed_documents_fail_closed() {
        // Not a doc node.
        assert!(matches!(
            BlockTree::from_document_json(
                "KRD-x",
                DOCUMENT_SCHEMA_VERSION,
                &json!({"type": "paragraph"})
            ),
            Err(BlockTreeError::NotADocNode)
        ));
        // Unsupported block type.
        let bad = json!({"type": "doc", "content": [{ "type": "mysteryBlock" }]});
        assert!(matches!(
            BlockTree::from_document_json("KRD-x", DOCUMENT_SCHEMA_VERSION, &bad),
            Err(BlockTreeError::UnsupportedBlockType { .. })
        ));
    }

    #[test]
    fn schema_mismatch_is_surfaced_not_coerced() {
        let doc = sample();
        let tree = BlockTree::from_document_json("KRD-x", "rich_document_v0", &doc).unwrap();
        assert!(
            !tree.schema_matches(),
            "a different schema version is a migration signal"
        );
    }
}
