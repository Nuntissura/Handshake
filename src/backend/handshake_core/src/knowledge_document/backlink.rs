//! MT-154 DocumentSearchIndexBridge + MT-155 DocumentBacklinkBridge (model
//! inputs).
//!
//! A RichDocument emits link references: typed link blocks (file/folder/
//! project/spec/wp/symbol, MT-146) and inline links / wikilinks / mentions /
//! tags inside text. This module extracts those references FROM the canonical
//! block tree as typed [`DocumentLinkReference`]s carrying a stable
//! relationship id, so the API/service layer (MT-155) can persist a backlink
//! edge whose id is stable across re-extraction runs (same determinism
//! discipline as `derive_knowledge_relationship_id`), and so the search-index
//! bridge (MT-154) can index titles / blocks / links / tags / embeds.
//!
//! This is the PURE extraction model; the persistence into `knowledge_edges`
//! (with the stable `relationship_id`) lives in the storage/api layer.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::kernel::context_bundle::sha256_hex;

use super::block_tree::{Block, BlockKind, BlockTree};

/// The kind of a link reference a document emits (MT-155).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentLinkKind {
    /// A typed file-link block.
    File,
    /// A typed folder-link block.
    Folder,
    /// A typed project-link block.
    Project,
    /// A typed spec-link block.
    Spec,
    /// A typed work-packet-link block.
    Wp,
    /// A typed symbol-link block.
    Symbol,
    /// An inline `[[wikilink]]` inside text.
    Wikilink,
    /// An `@mention` inside text.
    Mention,
    /// A `#tag` inside text.
    Tag,
}

impl DocumentLinkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
            Self::Project => "project",
            Self::Spec => "spec",
            Self::Wp => "wp",
            Self::Symbol => "symbol",
            Self::Wikilink => "wikilink",
            Self::Mention => "mention",
            Self::Tag => "tag",
        }
    }

    fn from_typed_link_block(kind: BlockKind) -> Option<Self> {
        Some(match kind {
            BlockKind::FileLink => Self::File,
            BlockKind::FolderLink => Self::Folder,
            BlockKind::ProjectLink => Self::Project,
            BlockKind::SpecLink => Self::Spec,
            BlockKind::WpLink => Self::Wp,
            BlockKind::SymbolLink => Self::Symbol,
            _ => return None,
        })
    }

    fn from_hs_link_ref_kind(ref_kind: &str) -> Self {
        match ref_kind.trim().to_ascii_lowercase().as_str() {
            "file" => Self::File,
            "folder" => Self::Folder,
            "project" => Self::Project,
            "spec" => Self::Spec,
            "wp" => Self::Wp,
            "symbol" => Self::Symbol,
            _ => Self::Wikilink,
        }
    }
}

/// A single typed link reference emitted by a document (MT-155).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentLinkReference {
    /// Stable relationship id (`KDLNK-...`), deterministic across re-extraction
    /// runs (MT-155). Hashes the source document, the link kind, the target,
    /// and the originating block id with a length-prefixed injective preimage.
    pub relationship_id: String,
    /// The source rich document id (`KRD-...`).
    pub source_document_id: String,
    pub kind: DocumentLinkKind,
    /// The link target: a typed id (file path token, KRD/KWP/spec anchor,
    /// symbol fqn), a wikilink title, a mention handle, or a tag name.
    pub target: String,
    /// Stable block id of the block this reference came from (MT-148).
    pub block_id: String,
}

impl DocumentLinkReference {
    fn new(source_document_id: &str, kind: DocumentLinkKind, target: &str, block_id: &str) -> Self {
        Self {
            relationship_id: derive_document_link_relationship_id(
                source_document_id,
                kind,
                target,
                block_id,
            ),
            source_document_id: source_document_id.to_string(),
            kind,
            target: target.to_string(),
            block_id: block_id.to_string(),
        }
    }
}

/// All link references a document emits, deduplicated by relationship id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentLinkReferences {
    pub source_document_id: String,
    pub references: Vec<DocumentLinkReference>,
}

impl DocumentLinkReferences {
    /// Extract every typed link reference from a parsed block tree (MT-155):
    /// typed link blocks plus inline wikilinks/mentions/tags inside text.
    /// Deduplicated by stable relationship id (a link referenced twice yields
    /// one edge).
    pub fn extract(tree: &BlockTree) -> Self {
        let mut references: Vec<DocumentLinkReference> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for block in &tree.blocks {
            for reference in extract_from_block(&tree.rich_document_id, block) {
                if seen.insert(reference.relationship_id.clone()) {
                    references.push(reference);
                }
            }
        }
        Self {
            source_document_id: tree.rich_document_id.clone(),
            references,
        }
    }

    /// The distinct `#tag` targets, for the search-index tag bridge (MT-154).
    pub fn tags(&self) -> Vec<String> {
        self.references
            .iter()
            .filter(|r| r.kind == DocumentLinkKind::Tag)
            .map(|r| r.target.clone())
            .collect()
    }
}

/// Derive a stable, deterministic relationship id for a document link edge
/// (MT-155). Length-prefixed injective preimage (same discipline as
/// `derive_knowledge_relationship_id`) so no separator inside a target/handle
/// can alias two distinct links. Stable across re-extraction because it hashes
/// only natural identities, never row ids or run ids.
pub fn derive_document_link_relationship_id(
    source_document_id: &str,
    kind: DocumentLinkKind,
    target: &str,
    block_id: &str,
) -> String {
    use std::fmt::Write as _;
    let mut canonical = String::from("knowledge_document_link_v1");
    for component in [source_document_id, kind.as_str(), target, block_id] {
        let _ = write!(canonical, "|{}:{}", component.len(), component);
    }
    format!("KDLNK-{}", sha256_hex(canonical.as_bytes()))
}

/// Extract the link references from a single block.
fn extract_from_block(source_document_id: &str, block: &Block) -> Vec<DocumentLinkReference> {
    let mut out = Vec::new();

    // Typed link blocks carry their target in `attrs.target`.
    if let Some(kind) = DocumentLinkKind::from_typed_link_block(block.kind) {
        if let Some(target) = typed_link_target(&block.content.raw) {
            out.push(DocumentLinkReference::new(
                source_document_id,
                kind,
                &target,
                &block.block_id,
            ));
        }
    }

    extract_structured_inline_references(
        source_document_id,
        &block.content.raw,
        &block.block_id,
        &mut out,
    );

    // Inline wikilinks / mentions / tags inside the block's plain text.
    let text = &block.content.derived.plain_text;
    for target in scan_wikilinks(text) {
        out.push(DocumentLinkReference::new(
            source_document_id,
            DocumentLinkKind::Wikilink,
            &target,
            &block.block_id,
        ));
    }
    for target in scan_prefixed(text, '@') {
        out.push(DocumentLinkReference::new(
            source_document_id,
            DocumentLinkKind::Mention,
            &target,
            &block.block_id,
        ));
    }
    for target in scan_prefixed(text, '#') {
        out.push(DocumentLinkReference::new(
            source_document_id,
            DocumentLinkKind::Tag,
            &target,
            &block.block_id,
        ));
    }
    out
}

/// Read `attrs.target` (a string) from a typed link block's raw node.
fn typed_link_target(raw: &Value) -> Option<String> {
    raw.as_object()
        .and_then(|o| o.get("attrs"))
        .and_then(Value::as_object)
        .and_then(|a| a.get("target"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_structured_inline_references(
    source_document_id: &str,
    node: &Value,
    block_id: &str,
    out: &mut Vec<DocumentLinkReference>,
) {
    let Some(obj) = node.as_object() else {
        return;
    };
    match obj.get("type").and_then(Value::as_str) {
        Some("hsLink") => {
            if let Some((kind, target)) = hs_link_reference(obj) {
                out.push(DocumentLinkReference::new(
                    source_document_id,
                    kind,
                    &target,
                    block_id,
                ));
            }
        }
        Some("mention") => {
            if let Some(target) = inline_attr_target(obj, &["id", "label"]) {
                out.push(DocumentLinkReference::new(
                    source_document_id,
                    DocumentLinkKind::Mention,
                    &target,
                    block_id,
                ));
            }
        }
        Some("tagMention") => {
            if let Some(target) = inline_attr_target(obj, &["id", "label"]) {
                out.push(DocumentLinkReference::new(
                    source_document_id,
                    DocumentLinkKind::Tag,
                    &target,
                    block_id,
                ));
            }
        }
        _ => {}
    }

    if let Some(children) = obj.get("content").and_then(Value::as_array) {
        for child in children {
            extract_structured_inline_references(source_document_id, child, block_id, out);
        }
    }
}

fn hs_link_reference(obj: &serde_json::Map<String, Value>) -> Option<(DocumentLinkKind, String)> {
    let attrs = obj.get("attrs").and_then(Value::as_object)?;
    let ref_kind = attrs
        .get("refKind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("wikilink");
    let ref_value = attrs
        .get("refValue")
        .and_then(Value::as_str)
        .or_else(|| attrs.get("target").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let kind = DocumentLinkKind::from_hs_link_ref_kind(ref_kind);
    let target = if kind == DocumentLinkKind::Wikilink
        && !matches!(
            ref_kind.to_ascii_lowercase().as_str(),
            "note" | "wikilink" | "wiki"
        ) {
        format!("{ref_kind}:{ref_value}")
    } else {
        ref_value.to_string()
    };
    Some((kind, target))
}

fn inline_attr_target(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    let attrs = obj.get("attrs").and_then(Value::as_object)?;
    keys.iter()
        .find_map(|key| attrs.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// Scan `[[wikilink]]` targets out of plain text.
fn scan_wikilinks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("[[") {
        let after = &rest[open + 2..];
        if let Some(close) = after.find("]]") {
            let inner = after[..close].trim();
            if !inner.is_empty() {
                out.push(inner.to_string());
            }
            rest = &after[close + 2..];
        } else {
            break;
        }
    }
    out
}

/// Scan `@handle` / `#tag` tokens out of plain text. A token runs from the
/// prefix char up to the next whitespace or punctuation that is not part of a
/// handle (`-`, `_`, `/`, `.`, `:` are allowed inside, so spec anchors and
/// paths survive).
fn scan_prefixed(text: &str, prefix: char) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // A prefix only starts a token at string start or after whitespace, so
        // `email@host` is not a mention but ` @host` is.
        let at_boundary = i == 0 || chars[i - 1].is_whitespace();
        if chars[i] == prefix && at_boundary {
            let mut j = i + 1;
            while j < chars.len()
                && (chars[j].is_alphanumeric() || matches!(chars[j], '-' | '_' | '/' | '.' | ':'))
            {
                j += 1;
            }
            let token: String = chars[i + 1..j].iter().collect();
            let token = token.trim_end_matches([':', '.']);
            if !token.is_empty() {
                out.push(token.to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_document::block_tree::{BlockTree, DOCUMENT_SCHEMA_VERSION};
    use serde_json::json;

    fn tree() -> BlockTree {
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "See [[Guide]] ping @alice tag #ops" }] },
                { "type": "wpLink", "attrs": { "target": "WP-1" } },
                { "type": "specLink", "attrs": { "target": "2.3.13.11" } },
                { "type": "fileLink", "attrs": { "target": "src/main.rs" } }
            ]
        });
        BlockTree::from_document_json("KRD-doc", DOCUMENT_SCHEMA_VERSION, &doc).unwrap()
    }

    #[test]
    fn extracts_typed_links_and_inline_references() {
        let refs = DocumentLinkReferences::extract(&tree());
        let has = |kind: DocumentLinkKind, target: &str| {
            refs.references
                .iter()
                .any(|r| r.kind == kind && r.target == target)
        };
        assert!(has(DocumentLinkKind::Wikilink, "Guide"));
        assert!(has(DocumentLinkKind::Mention, "alice"));
        assert!(has(DocumentLinkKind::Tag, "ops"));
        assert!(has(DocumentLinkKind::Wp, "WP-1"));
        assert!(has(DocumentLinkKind::Spec, "2.3.13.11"));
        assert!(has(DocumentLinkKind::File, "src/main.rs"));
    }

    #[test]
    fn extracts_inline_tiptap_node_references_from_attrs() {
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [
                    { "type": "text", "text": "Structured inline refs " },
                    { "type": "hsLink", "attrs": { "refKind": "file", "refValue": "src/lib/editor.ts", "label": "editor.ts" } },
                    { "type": "text", "text": " " },
                    { "type": "hsLink", "attrs": { "refKind": "wp", "refValue": "WP-KERNEL-009", "label": "WP-KERNEL-009" } },
                    { "type": "text", "text": " " },
                    { "type": "hsLink", "attrs": { "refKind": "video", "refValue": "KVID-fixture", "label": "video" } },
                    { "type": "text", "text": " " },
                    { "type": "mention", "attrs": { "id": "operator-1", "label": "Operator One" } },
                    { "type": "text", "text": " " },
                    { "type": "tagMention", "attrs": { "id": "tag-fixture", "label": "fixture" } }
                ] }
            ]
        });
        let tiptap_tree =
            BlockTree::from_document_json("KRD-doc", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        let refs = DocumentLinkReferences::extract(&tiptap_tree);
        let has = |kind: DocumentLinkKind, target: &str| {
            refs.references
                .iter()
                .any(|r| r.kind == kind && r.target == target)
        };
        assert!(has(DocumentLinkKind::File, "src/lib/editor.ts"));
        assert!(has(DocumentLinkKind::Wp, "WP-KERNEL-009"));
        assert!(has(DocumentLinkKind::Wikilink, "video:KVID-fixture"));
        assert!(has(DocumentLinkKind::Mention, "operator-1"));
        assert!(has(DocumentLinkKind::Tag, "tag-fixture"));
    }

    #[test]
    fn relationship_id_is_deterministic_and_namespaced() {
        let id_a =
            derive_document_link_relationship_id("KRD-doc", DocumentLinkKind::Wp, "WP-1", "KBL-1");
        let id_b =
            derive_document_link_relationship_id("KRD-doc", DocumentLinkKind::Wp, "WP-1", "KBL-1");
        assert_eq!(id_a, id_b);
        assert!(id_a.starts_with("KDLNK-"));
        // A different target derives a different id.
        let id_c =
            derive_document_link_relationship_id("KRD-doc", DocumentLinkKind::Wp, "WP-2", "KBL-1");
        assert_ne!(id_a, id_c);
    }

    #[test]
    fn injective_preimage_distinguishes_separator_collisions() {
        // The length-prefixed preimage must not let `|`/`:` inside a target
        // alias two distinct links (same discipline as knowledge_edges).
        let a =
            derive_document_link_relationship_id("KRD-doc", DocumentLinkKind::File, "a:b", "KBL-1");
        let b =
            derive_document_link_relationship_id("KRD-doc", DocumentLinkKind::File, "a", "b:KBL-1");
        assert_ne!(a, b, "separator-injective preimage must not alias");
    }

    #[test]
    fn references_are_deduplicated_by_relationship_id() {
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "#ops #ops" }] }
            ]
        });
        let t = BlockTree::from_document_json("KRD-doc", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        let refs = DocumentLinkReferences::extract(&t);
        let tags: Vec<_> = refs
            .references
            .iter()
            .filter(|r| r.kind == DocumentLinkKind::Tag)
            .collect();
        assert_eq!(
            tags.len(),
            1,
            "a tag referenced twice in one block yields one edge"
        );
    }

    #[test]
    fn mention_only_at_word_boundary() {
        // `email@host` is NOT a mention; ` @alice` is.
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "mail me at a@host then ping @alice" }] }
            ]
        });
        let t = BlockTree::from_document_json("KRD-doc", DOCUMENT_SCHEMA_VERSION, &doc).unwrap();
        let refs = DocumentLinkReferences::extract(&t);
        let mentions: Vec<_> = refs
            .references
            .iter()
            .filter(|r| r.kind == DocumentLinkKind::Mention)
            .map(|r| r.target.as_str())
            .collect();
        assert_eq!(mentions, vec!["alice"]);
    }
}
