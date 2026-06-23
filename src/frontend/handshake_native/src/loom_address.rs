//! The "everything is a Loom block" addressing layer (WP-KERNEL-012 MT-032, cluster E5).
//!
//! ## What this is (the UNIQUE MT-032 deliverable)
//!
//! Every document, rich-text block, canvas node, and graph node in the native surface is addressable as
//! a Loom block by a stable [`LoomBlockAddr`] (`workspace_id` + `block_id`), rendered as a `loom://`
//! URI ([`loom_uri`] / [`parse_loom_uri`]). [`LoomBlockRecord`] pairs that address with the block's
//! type, title, and content hash so a single typed value flows across the four editor surfaces (the
//! interconnection-contract "everything is a Loom block" invariant). [`LoomBlockResolver`] reads a
//! block's record from the EXISTING backend Loom surface (`GET /workspaces/{ws}/loom/blocks/{id}` —
//! `getLoomBlock`) off the UI thread.
//!
//! ## CONTENT HASH IS BACKEND-COMPUTED — this layer READS it, it never client-PATCHes a hash
//!
//! The MT-032 contract body proposed `PATCH /loom/blocks/{id} { content_hash }`. The KERNEL_BUILDER
//! gate (and MT-022's verified `LoomBlockUpdate` shape) established that the backend `LoomBlockUpdate`
//! exposes ONLY `pinned` / `favorite` / `title` (see [`crate::backend_client::LoomBlockClient`]);
//! there is NO writable `content_hash` field. The backend computes the document hash server-side
//! (`knowledge_canonical_json_sha256`, ported byte-for-byte in
//! [`crate::rich_editor::save::canonical_hash`]). So this layer:
//!   - READS the authoritative `content_hash` from the loaded `LoomBlock` (getLoomBlock), and
//!   - can RECOMPUTE the same canonical-JSON SHA-256 LOCALLY ([`ContentHash::of_content_json`], reusing
//!     `canonical_hash`) for display + a client-side integrity check that the local edit buffer matches
//!     the persisted block — NEVER as a fake write.
//!
//! There is therefore NO unsupported PATCH here. A real content-hash write route does not exist; if one
//! is added later, bind it then. This is the "verify the real backend, never trust the contract wording"
//! rule the whole crate already follows (the MT-011 hsLink / MT-020 canonical-hash lessons).
//!
//! ## Reuse, not reinvent
//!
//! - The canonical-JSON SHA-256 reuses [`crate::rich_editor::save::canonical_hash`] (MT-020) — NO new
//!   `indexmap`, NO duplicate `sha2`, NO second canonical serializer.
//! - The resolver reuses the existing `reqwest` + tokio off-thread + delivery-cell pattern
//!   (`backend_client`), speaking `serde_json::Value` so it never depends on the `handshake_core` crate.

use std::sync::{Arc, Mutex};

use serde_json::Value as JsonValue;

use crate::rich_editor::save::canonical_hash::canonical_content_sha256;

/// The URI scheme every Loom block is addressable under (`loom://{workspace_id}/{block_id}`).
pub const LOOM_URI_SCHEME: &str = "loom://";

/// A stable address for any Loom block: the workspace it lives in + the block's id. Stable across
/// renames (the `block_id` is the backend's UUID; the `workspace_id` is the app-level workspace UUID),
/// so a `loom://` reference never breaks when a block's title changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LoomBlockAddr {
    /// The workspace UUID the block lives in.
    pub workspace_id: String,
    /// The backend block UUID (stable across renames).
    pub block_id: String,
}

impl LoomBlockAddr {
    /// Construct an address from a workspace id + block id.
    pub fn new(workspace_id: impl Into<String>, block_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            block_id: block_id.into(),
        }
    }

    /// This address as its `loom://{workspace_id}/{block_id}` URI (convenience for [`loom_uri`]).
    pub fn to_uri(&self) -> String {
        loom_uri(self)
    }

    /// True when neither id is empty — a usable, addressable block (a placement with an empty
    /// `placed_block_id`, or a pane with no workspace yet, is NOT addressable; the chip/tooltip is
    /// skipped, never faked — red-team RISK-3).
    pub fn is_addressable(&self) -> bool {
        !self.workspace_id.trim().is_empty() && !self.block_id.trim().is_empty()
    }
}

/// A content-addressing hash of a Loom block's `content_json`: the canonical-JSON SHA-256 (lowercase
/// hex, 64 chars), byte-identical to the backend's `knowledge_canonical_json_sha256`. The value either
/// comes FROM the backend (read off a loaded `LoomBlock.content_hash`) or is RECOMPUTED locally from a
/// `content_json` value via [`ContentHash::of_content_json`] (reusing the MT-020 canonical writer) for
/// display + a client-side match check. It is never written back to the backend (the backend computes
/// it server-side; there is no writable field — see the module docs).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash(pub String);

impl ContentHash {
    /// Compute the canonical-JSON SHA-256 of a `content_json` value, byte-identical to the backend's
    /// server-side hash (reuses [`canonical_content_sha256`] — the MT-020 canonical writer; NO new
    /// serializer, NO duplicate sha2). Deterministic: the same value hashes identically across calls
    /// and platforms (red-team RISK-1 / MC-1 — the canonical writer sorts keys + emits no whitespace).
    pub fn of_content_json(content_json: &JsonValue) -> Self {
        ContentHash(canonical_content_sha256(content_json))
    }

    /// Build a [`ContentHash`] from a hash string already produced by the backend (a `LoomBlock`'s
    /// `content_hash` field). Empty/whitespace -> `None` (an absent backend hash is honestly absent,
    /// never a fabricated zero).
    pub fn from_backend(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(ContentHash(trimmed.to_owned()))
        }
    }

    /// The hash as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A short prefix (first 8 hex chars) for compact display in a chip/tooltip; the full hash stays in
    /// [`ContentHash::as_str`].
    pub fn short(&self) -> &str {
        let n = self.0.len().min(8);
        &self.0[..n]
    }
}

/// The kind of Loom block an address points at. Mirrors the four native surfaces that are each
/// addressable as a block (the interconnection contract's "everything is a Loom block").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoomBlockType {
    /// A rich-text document (the Obsidian/Notion-class editor).
    RichDocument,
    /// A node placed on the canvas board (a placement REFERENCE to a block).
    CanvasNode,
    /// A code-file node addressable from the code editor.
    CodeFileNode,
    /// A node in the Loom knowledge graph.
    GraphNode,
}

impl LoomBlockType {
    /// Map the backend `content_type` string (`note`/`document`/`canvas`/`file`/...) to a block type.
    /// An unknown/absent type degrades to [`LoomBlockType::GraphNode`] (a generic addressable block) —
    /// never a panic, never a fabricated specific type.
    pub fn from_content_type(content_type: &str) -> Self {
        match content_type {
            "canvas" => LoomBlockType::CanvasNode,
            "file" | "annotated_file" => LoomBlockType::CodeFileNode,
            "note" | "document" | "journal" => LoomBlockType::RichDocument,
            _ => LoomBlockType::GraphNode,
        }
    }

    /// A stable kebab-case token for the type (display + test vocabulary).
    pub fn kind_str(&self) -> &'static str {
        match self {
            LoomBlockType::RichDocument => "rich-document",
            LoomBlockType::CanvasNode => "canvas-node",
            LoomBlockType::CodeFileNode => "code-file-node",
            LoomBlockType::GraphNode => "graph-node",
        }
    }
}

/// A resolved Loom block: its stable address, its type, its display title, and its backend-computed
/// content hash (when the backend carries one). This is the one typed value that flows across the four
/// editor surfaces — a graph node, a canvas placement, a rich document, and a code-file node are all
/// `LoomBlockRecord`s, so the surfaces "melt together" around one address scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoomBlockRecord {
    /// The stable address (`loom://{workspace_id}/{block_id}`).
    pub addr: LoomBlockAddr,
    /// The block's display title (may be empty for an untitled block).
    pub title: String,
    /// The backend-computed content hash, when the loaded block carried one. `None` when the backend
    /// did not include a `content_hash` (honestly absent — not a fabricated value).
    pub content_hash: Option<ContentHash>,
    /// The block type (which surface it primarily belongs to).
    pub block_type: LoomBlockType,
}

impl LoomBlockRecord {
    /// This record's `loom://` URI.
    pub fn uri(&self) -> String {
        self.addr.to_uri()
    }

    /// Parse a verified `LoomBlock` JSON value (the `getLoomBlock` response) into a record for
    /// `workspace_id`. Returns `None` when the body has no `block_id` (a malformed row is skipped, not
    /// faked). Reads the backend `content_hash` if present; an absent hash stays `None`.
    pub fn from_loom_block_json(workspace_id: &str, v: &JsonValue) -> Option<Self> {
        let block_id = v.get("block_id").and_then(|x| x.as_str())?.to_owned();
        let title = v
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_owned();
        let content_type = v.get("content_type").and_then(|x| x.as_str()).unwrap_or("");
        let content_hash = v
            .get("content_hash")
            .and_then(|x| x.as_str())
            .and_then(ContentHash::from_backend);
        Some(LoomBlockRecord {
            addr: LoomBlockAddr::new(workspace_id, block_id),
            title,
            content_hash,
            block_type: LoomBlockType::from_content_type(content_type),
        })
    }
}

/// Format a [`LoomBlockAddr`] as a `loom://{workspace_id}/{block_id}` URI.
pub fn loom_uri(addr: &LoomBlockAddr) -> String {
    format!("{LOOM_URI_SCHEME}{}/{}", addr.workspace_id, addr.block_id)
}

/// Parse a `loom://{workspace_id}/{block_id}` URI back into a [`LoomBlockAddr`]. Returns `None` for any
/// string that is not a well-formed loom URI (wrong scheme, missing the `/` separator, or an empty
/// workspace/block segment). The round-trip `parse_loom_uri(loom_uri(&a)) == Some(a)` holds for every
/// addressable `a` (AC-1). The `block_id` segment keeps everything after the FIRST `/` so a backend id
/// that itself contains a `/` survives (the workspace id never does).
pub fn parse_loom_uri(s: &str) -> Option<LoomBlockAddr> {
    let rest = s.strip_prefix(LOOM_URI_SCHEME)?;
    let (workspace_id, block_id) = rest.split_once('/')?;
    if workspace_id.is_empty() || block_id.is_empty() {
        return None;
    }
    Some(LoomBlockAddr::new(workspace_id, block_id))
}

/// One-slot delivery cell for an off-thread [`LoomBlockResolver::resolve`]: `(block_id, result)`. The
/// spawned task writes the resolved [`LoomBlockRecord`] (or an error string) here; the egui UI thread
/// drains it next frame (the crate's standard `Arc<Mutex<Option<..>>>` off-thread pattern, HBR-QUIET —
/// the resolve HTTP call is NEVER on the UI thread).
pub type LoomBlockRecordCell = Arc<Mutex<Option<(String, Result<LoomBlockRecord, String>)>>>;

/// Resolves a [`LoomBlockAddr`] to its full [`LoomBlockRecord`] by reading the EXISTING backend Loom
/// surface (`GET /workspaces/{ws}/loom/blocks/{id}` — `getLoomBlock`). It READS the block's
/// backend-computed `content_hash`; it never client-PATCHes a hash (the backend has no writable
/// content-hash field — see the module docs). Off-thread + delivery-cell, mirroring the
/// `CanvasBoardClient::resolve_block` shape. Speaks `serde_json::Value` so it never depends on the
/// `handshake_core` crate.
#[derive(Clone)]
pub struct LoomBlockResolver {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomBlockResolver {
    /// Build a resolver against `base_url` (e.g. [`crate::backend_client::BACKEND_BASE_URL`]) bridging
    /// onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production resolver: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL, runtime)
    }

    /// The verified `getLoomBlock` URL for `addr` (`GET /workspaces/{ws}/loom/blocks/{id}`). Split out so
    /// a unit test asserts the EXACT URL without a live backend (the spawn path routes through this same
    /// builder, so the test proves the production request construction).
    pub fn resolve_url(&self, addr: &LoomBlockAddr) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}",
            self.base_url, addr.workspace_id, addr.block_id
        )
    }

    /// Resolve `addr` to its [`LoomBlockRecord`] off the UI thread, delivering
    /// `(block_id, Ok(record))` / `(block_id, Err(msg))` into `cell`. A 404 / non-success status is an
    /// `Err` (the caller shows "(unresolved)" — never a fabricated record).
    pub fn resolve(&self, addr: &LoomBlockAddr, cell: LoomBlockRecordCell) {
        let url = self.resolve_url(addr);
        let client = self.client.clone();
        let workspace_id = addr.workspace_id.clone();
        let block_id = addr.block_id.clone();
        self.runtime.spawn(async move {
            let result = fetch_loom_block_record(&client, &url, &workspace_id).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((block_id, result));
            }
        });
    }
}

/// `GET {url}` and parse the verified `LoomBlock` body into a [`LoomBlockRecord`] for `workspace_id`. A
/// non-success status or a body missing `block_id` is an error (never a fabricated record).
async fn fetch_loom_block_record(
    client: &reqwest::Client,
    url: &str,
    workspace_id: &str,
) -> Result<LoomBlockRecord, String> {
    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("getLoomBlock failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("getLoomBlock returned HTTP {status}"));
    }
    let v: JsonValue = resp
        .json()
        .await
        .map_err(|e| format!("getLoomBlock body invalid: {e}"))?;
    LoomBlockRecord::from_loom_block_json(workspace_id, &v)
        .ok_or_else(|| "getLoomBlock body had no block_id".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_loom_uri_round_trips() {
        // AC-1: parse_loom_uri(loom_uri(&addr)) == Some(addr).
        let addr = LoomBlockAddr::new("ws1", "block1");
        assert_eq!(loom_uri(&addr), "loom://ws1/block1");
        assert_eq!(parse_loom_uri(&loom_uri(&addr)), Some(addr.clone()));
        // The literal the contract names parses to the exact address.
        assert_eq!(
            parse_loom_uri("loom://ws1/block1"),
            Some(LoomBlockAddr::new("ws1", "block1"))
        );
    }

    #[test]
    fn loom_uri_round_trips_for_uuid_shaped_ids() {
        let addr = LoomBlockAddr::new(
            "11111111-1111-1111-1111-111111111111",
            "22222222-2222-2222-2222-222222222222",
        );
        assert_eq!(parse_loom_uri(&addr.to_uri()), Some(addr));
    }

    #[test]
    fn parse_loom_uri_rejects_malformed() {
        assert_eq!(parse_loom_uri("https://ws1/block1"), None, "wrong scheme");
        assert_eq!(parse_loom_uri("loom://ws1"), None, "no block segment");
        assert_eq!(parse_loom_uri("loom://"), None, "empty");
        assert_eq!(parse_loom_uri("loom:///block1"), None, "empty workspace");
        assert_eq!(parse_loom_uri("loom://ws1/"), None, "empty block");
        assert_eq!(parse_loom_uri("block1"), None, "no scheme");
    }

    #[test]
    fn parse_keeps_block_id_with_internal_slash() {
        // The workspace id is the segment up to the FIRST '/'; a block id containing a '/' survives.
        let parsed = parse_loom_uri("loom://ws1/a/b/c").unwrap();
        assert_eq!(parsed.workspace_id, "ws1");
        assert_eq!(parsed.block_id, "a/b/c");
    }

    #[test]
    fn is_addressable_guards_empty_ids() {
        assert!(LoomBlockAddr::new("ws", "blk").is_addressable());
        assert!(!LoomBlockAddr::new("", "blk").is_addressable(), "empty workspace");
        assert!(!LoomBlockAddr::new("ws", "").is_addressable(), "empty block (RISK-3 chip skip)");
        assert!(!LoomBlockAddr::new("  ", "blk").is_addressable(), "whitespace workspace");
    }

    #[test]
    fn content_hash_is_deterministic_and_matches_backend_writer() {
        // RISK-1 / MC-1: the same content_json hashes identically across calls (no key-order drift).
        let v = json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"hi"}]}]});
        let h1 = ContentHash::of_content_json(&v);
        let h2 = ContentHash::of_content_json(&v);
        assert_eq!(h1, h2, "deterministic");
        // It is byte-identical to the backend canonical writer (the shared MT-020 helper).
        assert_eq!(h1.as_str(), canonical_content_sha256(&v));
        // 64-char lowercase hex.
        assert_eq!(h1.as_str().len(), 64);
        assert!(h1.as_str().chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(h1.short().len(), 8, "short prefix is 8 chars");
    }

    #[test]
    fn key_order_does_not_change_the_hash() {
        // Two structurally-identical docs with different key insertion order hash the SAME (the canonical
        // writer sorts keys) — the determinism the write-back relied on.
        let a = json!({ "a": 1, "b": 2 });
        let b = json!({ "b": 2, "a": 1 });
        assert_eq!(ContentHash::of_content_json(&a), ContentHash::of_content_json(&b));
    }

    #[test]
    fn content_hash_from_backend_rejects_empty() {
        assert_eq!(ContentHash::from_backend(""), None);
        assert_eq!(ContentHash::from_backend("   "), None);
        assert_eq!(
            ContentHash::from_backend("deadbeef"),
            Some(ContentHash("deadbeef".to_owned()))
        );
    }

    #[test]
    fn block_type_maps_content_type() {
        assert_eq!(LoomBlockType::from_content_type("canvas"), LoomBlockType::CanvasNode);
        assert_eq!(LoomBlockType::from_content_type("file"), LoomBlockType::CodeFileNode);
        assert_eq!(LoomBlockType::from_content_type("note"), LoomBlockType::RichDocument);
        assert_eq!(LoomBlockType::from_content_type("document"), LoomBlockType::RichDocument);
        // Unknown degrades to a generic GraphNode (no panic, no fabricated specific type).
        assert_eq!(LoomBlockType::from_content_type("tag_hub"), LoomBlockType::GraphNode);
        assert_eq!(LoomBlockType::from_content_type(""), LoomBlockType::GraphNode);
    }

    #[test]
    fn record_reads_backend_content_hash_not_fabricates_it() {
        // The resolver READS the backend's content_hash (it does NOT recompute or PATCH it).
        let body = json!({
            "block_id": "blk-7",
            "title": "My Note",
            "content_type": "note",
            "content_hash": "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        });
        let rec = LoomBlockRecord::from_loom_block_json("ws1", &body).unwrap();
        assert_eq!(rec.addr, LoomBlockAddr::new("ws1", "blk-7"));
        assert_eq!(rec.title, "My Note");
        assert_eq!(rec.block_type, LoomBlockType::RichDocument);
        assert_eq!(
            rec.content_hash.as_ref().map(|h| h.as_str()),
            Some("44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a")
        );
        assert_eq!(rec.uri(), "loom://ws1/blk-7");
    }

    #[test]
    fn record_absent_content_hash_stays_none() {
        // A LoomBlock with no content_hash field -> the record's hash is honestly None (not faked).
        let body = json!({ "block_id": "blk-9", "title": "", "content_type": "canvas" });
        let rec = LoomBlockRecord::from_loom_block_json("ws1", &body).unwrap();
        assert_eq!(rec.content_hash, None);
        assert_eq!(rec.block_type, LoomBlockType::CanvasNode);
        assert_eq!(rec.title, "");
    }

    #[test]
    fn record_without_block_id_is_skipped_not_faked() {
        let body = json!({ "title": "orphan", "content_type": "note" });
        assert_eq!(LoomBlockRecord::from_loom_block_json("ws1", &body), None);
    }

    #[test]
    fn resolve_url_is_the_verified_get_loom_block_route() {
        let resolver = LoomBlockResolver::new("http://127.0.0.1:37501", dummy_handle());
        let addr = LoomBlockAddr::new("ws1", "blk-7");
        assert_eq!(
            resolver.resolve_url(&addr),
            "http://127.0.0.1:37501/workspaces/ws1/loom/blocks/blk-7"
        );
    }

    /// A tokio runtime handle for the URL-builder test (the builder is pure; no task is spawned).
    fn dummy_handle() -> tokio::runtime::Handle {
        use std::sync::OnceLock;
        static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
        RT.get_or_init(|| tokio::runtime::Builder::new_current_thread().build().unwrap())
            .handle()
            .clone()
    }
}
