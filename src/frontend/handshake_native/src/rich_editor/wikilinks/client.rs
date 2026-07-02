//! Async backend transport for wikilink autocomplete search, transclusion resolution, and backlink
//! listing (WP-KERNEL-012 MT-015).
//!
//! ## Why a fetcher TRAIT (reuse the proven MT-014 pattern, do not fork it)
//!
//! MT-014's `embeds::asset_resolver::AssetMetadataFetcher` proved the right shape for this crate:
//! a `Send + Sync` trait returning boxed futures (NOT `async-trait`, so ZERO new dependency
//! families), a production [`reqwest`] impl wrapping the existing `backend_client` REST stack, and a
//! COUNTED in-memory mock for the unit tests. This module reuses that exact pattern for the THREE
//! MT-015 backend bindings:
//!   - autocomplete search: `POST /workspaces/{ws}/loom/search-v2` -> [`LoomSearchV2Response`]
//!     (verified backend shape from `app/src/lib/api.ts loomSearchV2`),
//!   - transclusion resolve: `GET /workspaces/{ws}/loom/blocks/{ref_value}/transclusion` ->
//!     [`LoomBlockTransclusion`] (verified shape from `getLoomBlockTransclusion`),
//!   - backlinks: `GET /knowledge/documents/{doc_id}/backlinks` -> [`BacklinksResponse`]
//!     (verified shape from `listRichDocumentBacklinks`).
//!
//! The debounce (MC-002), generation-counter cancellation (MC-004), 404->remove-embed (MC-003), and
//! the typed-error vocabulary are ALL unit-testable here with a counted mock and NO backend.
//!
//! ## Backend reuse only (no backend edits — typed blocker if a gap)
//!
//! Every endpoint above already exists in `handshake_core` (proven by the React `api.ts` client that
//! calls them). This module only CONSUMES them read-only over the existing reqwest client; a missing
//! endpoint is a typed [`WikilinkError`] (a visible empty-state / error chip), never a backend edit.

use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;
use thiserror::Error;

/// The typed reasons a wikilink backend interaction failed. Every variant renders as a VISIBLE chip
/// / empty-state (fail-closed, never blank, never a panic). `kind_str` is a stable kebab-case token
/// the error UI + AccessKit label carry, so an out-of-process agent reads a stable failure
/// vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WikilinkError {
    /// No workspace bound — autocomplete/transclusion/backlinks resolve workspace state and need a
    /// workspace id.
    #[error("no workspace context: wikilink resolution needs a workspace id")]
    NoWorkspace,
    /// The target document/block was not found (HTTP 404). Drives the transclusion "Remove embed"
    /// affordance (MC-003).
    #[error("not found: {0}")]
    NotFound(String),
    /// The target is not accessible (HTTP 401/403).
    #[error("not accessible: {0}")]
    Forbidden(String),
    /// The backend returned 5xx or a malformed body.
    #[error("server error: {0}")]
    ServerError(String),
    /// The fetch itself failed (backend unreachable / transport error).
    #[error("network error: {0}")]
    NetworkError(String),
}

impl WikilinkError {
    /// Stable kebab-case kind token (the chip text + AccessKit label vocabulary).
    pub fn kind_str(&self) -> &'static str {
        match self {
            WikilinkError::NoWorkspace => "no_workspace",
            WikilinkError::NotFound(_) => "not_found",
            WikilinkError::Forbidden(_) => "forbidden",
            WikilinkError::ServerError(_) => "server_error",
            WikilinkError::NetworkError(_) => "network_error",
        }
    }

    /// True for a 404 (NotFound). The transclusion view shows a "Remove embed" action only on a 404
    /// of a deleted block (MC-003), not on a transient network error (which should retry, not delete).
    pub fn is_not_found(&self) -> bool {
        matches!(self, WikilinkError::NotFound(_))
    }
}

/// One autocomplete result the popup lists (mirrors the fields the React autocomplete shows). Built
/// from a [`LoomSearchV2Hit`]'s block, so each result carries the block id (the `ref_value` an inserted
/// wikilink targets), a title, the content type, and a search highlight snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikilinkResult {
    /// The LoomBlock id — the `ref_value` an inserted wikilink/transclusion targets.
    pub block_id: String,
    /// Display title (or the block id when the block has no title).
    pub title: String,
    /// The backend content type (`note`, `document`, …) — drives the result's prefix hint.
    pub content_type: String,
    /// A search-highlight snippet (may be empty).
    pub highlight: String,
}

/// The transclusion resolution result (the native mirror of the backend `LoomBlockTransclusion`,
/// verified shape from `app/src/lib/api.ts`). The host stores only the reference; this carries the
/// resolved SOURCE document content the read-through preview renders.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LoomBlockTransclusion {
    /// The transcluded block id.
    pub block_id: String,
    /// The workspace the block lives in.
    pub workspace_id: String,
    /// The source rich-document id edits route to (`null` when unresolved).
    #[serde(default)]
    pub source_document_id: Option<String>,
    /// The source document version (`null` when unresolved).
    #[serde(default)]
    pub source_doc_version: Option<i64>,
    /// The resolved source `content_json` (the read-through body). Kept as an opaque JSON value
    /// (the preview renders a plain-text projection of it); `null` when unresolved.
    #[serde(default)]
    pub content_json: Option<serde_json::Value>,
    /// Whether the block resolved to a live source document.
    pub resolved: bool,
    /// A typed reason the block did not resolve (e.g. `"source_unresolved"`), when `resolved=false`.
    #[serde(default)]
    pub unresolved_reason: Option<String>,
}

/// One backlink entry (the native mirror of the backend `RichDocBacklink`, verified shape from
/// `app/src/lib/api.ts`). NOTE: the backend record carries NO `source_title` (the MT contract assumed
/// one); the panel labels each entry by `source_document_id` + `link_kind` (the real fields).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RichDocBacklink {
    /// Stable backlink id.
    pub backlink_id: String,
    /// The workspace the backlink lives in.
    pub workspace_id: String,
    /// The relationship id.
    pub relationship_id: String,
    /// The document that LINKS TO the current document (the navigation target on click).
    pub source_document_id: String,
    /// The link kind (`note`, `wp`, …) — the backend ref kind that created the backlink.
    pub link_kind: String,
    /// The link target value.
    pub target: String,
    /// The source block id within the source document.
    pub block_id: String,
}

/// The backlinks list response (`{ source_document_id, backlinks: [...] }` — verified backend shape).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BacklinksResponse {
    /// The document the backlinks point AT (the queried document).
    pub source_document_id: String,
    /// Every document that links to it.
    pub backlinks: Vec<RichDocBacklink>,
}

/// One loom search-v2 hit (the subset of the backend `LoomSearchV2Hit` the autocomplete needs). The
/// block is the searched LoomBlock; `highlight` is the FTS snippet.
#[derive(Debug, Clone, Deserialize)]
pub struct LoomSearchV2Hit {
    /// The matched block.
    pub block: SearchLoomBlock,
    /// The FTS highlight snippet (may be empty).
    #[serde(default)]
    pub highlight: String,
}

/// The subset of the backend `LoomBlock` the autocomplete result needs (block id, title, type).
/// `#[serde(default)]` on the optionals so a forward-compatible backend body still deserializes.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchLoomBlock {
    /// The block id (the `ref_value` an inserted wikilink targets).
    pub block_id: String,
    /// The backend content type (`note`, `document`, …).
    pub content_type: String,
    /// The block title (absent for untitled blocks).
    #[serde(default)]
    pub title: Option<String>,
}

/// The loom search-v2 response (`{ hits, ... }` — the autocomplete only needs the hits).
#[derive(Debug, Clone, Deserialize)]
pub struct LoomSearchV2Response {
    /// The ranked hits.
    pub hits: Vec<LoomSearchV2Hit>,
}

impl WikilinkResult {
    /// Build a result row from a search hit (block id + title fallback + type + highlight).
    pub fn from_hit(hit: LoomSearchV2Hit) -> Self {
        let title = hit
            .block
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| hit.block.block_id.clone());
        Self {
            block_id: hit.block.block_id,
            title,
            content_type: hit.block.content_type,
            highlight: hit.highlight,
        }
    }
}

/// A boxed `Send` future yielding a Result, returned by the [`WikilinkBackend`] methods. Spelled out
/// (not `async-trait`) so this module adds ZERO new dependency families — the same boxed-future
/// pattern MT-014's `AssetMetadataFetcher` uses.
pub type WikilinkFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, WikilinkError>> + Send + 'a>>;

/// The backend transport for the three MT-015 bindings. A trait (not hard reqwest calls) so the
/// debounce / cancellation / error-mapping / 404-remove logic is unit-testable with a counted mock
/// and NO backend. The production impl ([`ReqwestWikilinkBackend`]) wraps the existing reqwest stack.
pub trait WikilinkBackend: Send + Sync {
    /// Autocomplete search: `POST /workspaces/{ws}/loom/search-v2` with `{query, limit}`. Returns the
    /// ranked block hits the popup lists (mapped to [`WikilinkResult`] by the caller).
    fn search<'a>(
        &'a self,
        workspace_id: &'a str,
        query: &'a str,
        limit: usize,
    ) -> WikilinkFuture<'a, Vec<WikilinkResult>>;

    /// Resolve a transclusion: `GET /workspaces/{ws}/loom/blocks/{ref_value}/transclusion`. Returns
    /// the resolved source content + metadata. A 404 maps to [`WikilinkError::NotFound`] (drives the
    /// "Remove embed" affordance, MC-003).
    fn resolve_transclusion<'a>(
        &'a self,
        workspace_id: &'a str,
        ref_value: &'a str,
    ) -> WikilinkFuture<'a, LoomBlockTransclusion>;

    /// List backlinks: `GET /knowledge/documents/{doc_id}/backlinks`. Returns every document linking
    /// to `document_id`.
    fn list_backlinks<'a>(&'a self, document_id: &'a str) -> WikilinkFuture<'a, BacklinksResponse>;
}

/// The production [`WikilinkBackend`]: a thin wrapper over a `reqwest::Client` against the verified
/// backend endpoints, mapping HTTP status to the typed [`WikilinkError`] vocabulary. REUSES the
/// existing reqwest 0.12 + rustls stack from `backend_client` — NO new HTTP crate. Read-only; no
/// backend code is touched.
#[derive(Clone)]
pub struct ReqwestWikilinkBackend {
    client: reqwest::Client,
    base_url: String,
}

impl ReqwestWikilinkBackend {
    /// Build a backend client against `base_url` (e.g. `backend_client::BACKEND_BASE_URL`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// The production client against the hardcoded backend base URL.
    pub fn production() -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL)
    }

    /// The REST base this client talks to.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Map a non-success HTTP status to the typed [`WikilinkError`] (shared by all three bindings).
fn map_status(status: u16, what: &str) -> WikilinkError {
    match status {
        404 => WikilinkError::NotFound(what.to_owned()),
        401 | 403 => WikilinkError::Forbidden(format!("{what} (HTTP {status})")),
        _ => WikilinkError::ServerError(format!("{what} returned HTTP {status}")),
    }
}

impl WikilinkBackend for ReqwestWikilinkBackend {
    fn search<'a>(
        &'a self,
        workspace_id: &'a str,
        query: &'a str,
        limit: usize,
    ) -> WikilinkFuture<'a, Vec<WikilinkResult>> {
        let url = format!(
            "{}/workspaces/{}/loom/search-v2",
            self.base_url, workspace_id
        );
        let client = self.client.clone();
        let query = query.to_owned();
        let ws_empty = workspace_id.trim().is_empty();
        Box::pin(async move {
            if ws_empty {
                return Err(WikilinkError::NoWorkspace);
            }
            let body = serde_json::json!({
                "query": query,
                "tag_ids": [],
                "graph_boost": 0,
                "limit": limit,
                "offset": 0,
            });
            let response = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| WikilinkError::NetworkError(format!("loom search failed: {e}")))?;
            let status = response.status();
            if !status.is_success() {
                return Err(map_status(status.as_u16(), "loom search-v2"));
            }
            let parsed: LoomSearchV2Response = response.json().await.map_err(|e| {
                WikilinkError::ServerError(format!("loom search body invalid: {e}"))
            })?;
            Ok(parsed
                .hits
                .into_iter()
                .map(WikilinkResult::from_hit)
                .collect())
        })
    }

    fn resolve_transclusion<'a>(
        &'a self,
        workspace_id: &'a str,
        ref_value: &'a str,
    ) -> WikilinkFuture<'a, LoomBlockTransclusion> {
        let url = format!(
            "{}/workspaces/{}/loom/blocks/{}/transclusion",
            self.base_url, workspace_id, ref_value
        );
        let client = self.client.clone();
        let ws_empty = workspace_id.trim().is_empty();
        let ref_value = ref_value.to_owned();
        Box::pin(async move {
            if ws_empty {
                return Err(WikilinkError::NoWorkspace);
            }
            let response = client.get(&url).send().await.map_err(|e| {
                WikilinkError::NetworkError(format!("transclusion fetch failed: {e}"))
            })?;
            let status = response.status();
            if !status.is_success() {
                return Err(map_status(
                    status.as_u16(),
                    &format!("transclusion '{ref_value}'"),
                ));
            }
            let parsed: LoomBlockTransclusion = response.json().await.map_err(|e| {
                WikilinkError::ServerError(format!("transclusion body invalid: {e}"))
            })?;
            Ok(parsed)
        })
    }

    fn list_backlinks<'a>(&'a self, document_id: &'a str) -> WikilinkFuture<'a, BacklinksResponse> {
        let url = format!(
            "{}/knowledge/documents/{}/backlinks",
            self.base_url, document_id
        );
        let client = self.client.clone();
        let document_id = document_id.to_owned();
        Box::pin(async move {
            let response =
                client.get(&url).send().await.map_err(|e| {
                    WikilinkError::NetworkError(format!("backlinks fetch failed: {e}"))
                })?;
            let status = response.status();
            if !status.is_success() {
                return Err(map_status(
                    status.as_u16(),
                    &format!("backlinks for '{document_id}'"),
                ));
            }
            let parsed: BacklinksResponse = response
                .json()
                .await
                .map_err(|e| WikilinkError::ServerError(format!("backlinks body invalid: {e}")))?;
            Ok(parsed)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_kind_strings_are_stable() {
        assert_eq!(WikilinkError::NoWorkspace.kind_str(), "no_workspace");
        assert_eq!(WikilinkError::NotFound("x".into()).kind_str(), "not_found");
        assert_eq!(WikilinkError::Forbidden("x".into()).kind_str(), "forbidden");
        assert_eq!(
            WikilinkError::ServerError("x".into()).kind_str(),
            "server_error"
        );
        assert_eq!(
            WikilinkError::NetworkError("x".into()).kind_str(),
            "network_error"
        );
    }

    #[test]
    fn is_not_found_only_for_404() {
        assert!(WikilinkError::NotFound("blk".into()).is_not_found());
        assert!(!WikilinkError::NetworkError("down".into()).is_not_found());
        assert!(!WikilinkError::ServerError("500".into()).is_not_found());
    }

    #[test]
    fn map_status_maps_http_codes() {
        assert_eq!(map_status(404, "x"), WikilinkError::NotFound("x".into()));
        assert_eq!(map_status(401, "x").kind_str(), "forbidden");
        assert_eq!(map_status(403, "x").kind_str(), "forbidden");
        assert_eq!(map_status(500, "x").kind_str(), "server_error");
        assert_eq!(map_status(502, "x").kind_str(), "server_error");
    }

    #[test]
    fn result_from_hit_falls_back_to_block_id_when_untitled() {
        let hit = LoomSearchV2Hit {
            block: SearchLoomBlock {
                block_id: "BLK-1".into(),
                content_type: "note".into(),
                title: None,
            },
            highlight: "match".into(),
        };
        let r = WikilinkResult::from_hit(hit);
        assert_eq!(r.title, "BLK-1", "untitled block falls back to its id");
        assert_eq!(r.block_id, "BLK-1");
        assert_eq!(r.content_type, "note");
        assert_eq!(r.highlight, "match");

        let titled = LoomSearchV2Hit {
            block: SearchLoomBlock {
                block_id: "BLK-2".into(),
                content_type: "document".into(),
                title: Some("My Doc".into()),
            },
            highlight: String::new(),
        };
        assert_eq!(WikilinkResult::from_hit(titled).title, "My Doc");
    }

    #[test]
    fn transclusion_deserializes_real_backend_shape() {
        // The verified backend LoomBlockTransclusion shape round-trips into the native type.
        let json = serde_json::json!({
            "block_id": "BLK-9",
            "workspace_id": "ws1",
            "source_document_id": "DOC-3",
            "source_doc_version": 4,
            "content_json": { "type": "doc", "content": [] },
            "resolved": true,
            "unresolved_reason": null
        });
        let t: LoomBlockTransclusion = serde_json::from_value(json).unwrap();
        assert_eq!(t.block_id, "BLK-9");
        assert_eq!(t.source_document_id.as_deref(), Some("DOC-3"));
        assert_eq!(t.source_doc_version, Some(4));
        assert!(t.resolved);
        assert!(t.content_json.is_some());
    }

    #[test]
    fn backlinks_deserialize_real_backend_shape() {
        let json = serde_json::json!({
            "source_document_id": "DOC-1",
            "backlinks": [
                {
                    "backlink_id": "BL-1",
                    "workspace_id": "ws1",
                    "relationship_id": "REL-1",
                    "source_document_id": "DOC-2",
                    "link_kind": "note",
                    "target": "DOC-1",
                    "block_id": "BLK-7"
                }
            ]
        });
        let r: BacklinksResponse = serde_json::from_value(json).unwrap();
        assert_eq!(r.source_document_id, "DOC-1");
        assert_eq!(r.backlinks.len(), 1);
        assert_eq!(r.backlinks[0].source_document_id, "DOC-2");
        assert_eq!(r.backlinks[0].link_kind, "note");
    }
}
