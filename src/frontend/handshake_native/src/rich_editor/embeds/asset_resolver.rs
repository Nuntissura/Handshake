//! Fail-closed media-embed asset resolution (WP-KERNEL-012 MT-014).
//!
//! This is the native Rust port of the React `app/src/lib/editor/embed_assets.ts`
//! validation + resolution pipeline. It is the standalone, backend-free CORE of the
//! embed cluster: the ref-shape validation, the album/slideshow ref-list parse, the
//! asset-metadata URL construction, the typed error vocabulary, the resolution state
//! machine, and the per-asset resolution cache all live here and are FULLY unit-testable
//! with NO backend (a counted, in-memory mock fetcher stands in for HTTP).
//!
//! ## What an "embed" is (NODE-SHAPE RECONCILIATION — the MT-014 critical gate)
//!
//! Media embeds are NOT a new `NodeKind::Embed` block. MT-011 already established
//! (`document_model::node::HsLinkNode` / `Child::HsLink`) that a Handshake typed link is
//! the inline atom `hsLink` carrying `{ ref_kind, ref_value, label, resolved }`, matching
//! the REAL backend `content_json` shape (`app/src/lib/tiptap/hs_link_node.ts`). The React
//! NodeView `HsLinkView.tsx` renders image/video/album/slideshow embeds AND ordinary
//! wikilinks through that SAME `hsLink` node, discriminated by `refKind`. So this MT renders
//! embeds from the EXISTING [`crate::rich_editor::document_model::node::HsLinkNode`], where
//! `ref_kind ∈ {images, video, album, slideshow}` (the [`MEDIA_EMBED_REF_KINDS`] set). No
//! invented node is added — inventing one would repeat the MT-011 wikilink-mark mistake.
//!
//! ## Fail-closed (red-team + the React EmbedErrorKind contract)
//!
//! Every failure is a TYPED [`EmbedError`] that the view renders as a VISIBLE chip — never
//! a blank, never a panic, never substituted mock data. The validation rejects (in this
//! exact order, matching `validateAssetRef`):
//!   - empty/whitespace ref            -> [`EmbedError::EmptyRef`]
//!   - a `:` (drive letter `C:\`/`C:/`) -> [`EmbedError::AbsolutePathRejected`]
//!   - a `:` (any other scheme)         -> [`EmbedError::SchemeRejected`]
//!   - a leading `/` or `\`             -> [`EmbedError::AbsolutePathRejected`]
//!   - a `/`, `\`, or `..` ANYWHERE     -> [`EmbedError::TraversalRejected`] (MC-003)
//!   - an over-long / non-pattern id    -> [`EmbedError::InvalidRef`]
//!
//! The `..` substring check is deliberately substring-ANYWHERE (not just a path-component
//! prefix), so `..hidden/secret` and `a..b` are both rejected (red-team RISK-3 / MC-003).

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use thiserror::Error;

/// The `hsLink` ref kinds that render as media embeds (mirrors the React
/// `MEDIA_EMBED_REF_KINDS`). Note `images` (plural) is the IMAGE kind — the backend
/// `RichDocEmbed.ref_kind` value is `"images"`, NOT `"image"` (a frequent transcription
/// slip the contract's scope text makes; the REAL backend shape, verified against
/// `embed_assets.ts`, is `images`).
pub const MEDIA_EMBED_REF_KINDS: [&str; 4] = ["images", "video", "album", "slideshow"];

/// DoS guard (mirrors the React `MAX_SEQUENCE_ITEMS`): an album/slideshow ref-list caps at
/// this many members. A hostile/corrupt document could otherwise carry thousands of
/// comma-separated ids, fanning out one metadata request each.
pub const MAX_SEQUENCE_ITEMS: usize = 100;

/// Concurrency cap for album/slideshow sequence resolution (red-team RISK-2 / MC-002): at
/// most this many member metadata fetches run at once, via a [`tokio::sync::Semaphore`], so
/// a 50-thumbnail album never opens 50 simultaneous backend connections.
pub const MAX_CONCURRENT_RESOLUTIONS: usize = 6;

/// The longest an asset id may be (mirrors `ASSET_ID_MAX_LENGTH`).
const ASSET_ID_MAX_LENGTH: usize = 256;

/// The media family an embed kind expects, so a kind/mime mismatch is fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaEmbedKind {
    /// `images`: a single still image.
    Images,
    /// `video`: a video asset (poster + play affordance; no in-process decode).
    Video,
    /// `album`: an ordered grid of images.
    Album,
    /// `slideshow`: an ordered one-at-a-time image sequence.
    Slideshow,
}

impl MediaEmbedKind {
    /// Parse a backend `ref_kind` string into a media kind, or `None` for a non-media kind
    /// (ordinary wikilink kinds keep the MT-015 chip rendering, not handled here).
    pub fn from_ref_kind(ref_kind: &str) -> Option<Self> {
        match ref_kind {
            "images" => Some(Self::Images),
            "video" => Some(Self::Video),
            "album" => Some(Self::Album),
            "slideshow" => Some(Self::Slideshow),
            _ => None,
        }
    }

    /// The backend `ref_kind` string for this kind (the inverse of [`Self::from_ref_kind`]).
    pub fn ref_kind(self) -> &'static str {
        match self {
            Self::Images => "images",
            Self::Video => "video",
            Self::Album => "album",
            Self::Slideshow => "slideshow",
        }
    }

    /// True when this kind resolves an ORDERED SEQUENCE (album/slideshow) rather than a single
    /// asset; such a `ref_value` is a comma-separated asset-id list.
    pub fn is_sequence(self) -> bool {
        matches!(self, Self::Album | Self::Slideshow)
    }

    /// True when `mime` matches the media family this kind expects (mirrors
    /// `mimeMatchesEmbedKind`). Fail-closed: a video asset inside an `images` embed is a
    /// [`EmbedError::KindMismatch`].
    pub fn mime_matches(self, mime: &str) -> bool {
        let normalized = mime.to_ascii_lowercase();
        match self {
            Self::Video => normalized.starts_with("video/"),
            Self::Images | Self::Album | Self::Slideshow => normalized.starts_with("image/"),
        }
    }
}

/// The typed reasons an embed cannot resolve. Every variant renders as a VISIBLE chip
/// (fail-closed). The kebab-case [`Self::kind_str`] matches the React `EmbedErrorKind`
/// vocabulary verbatim, so the native chip text is identical to the web app's.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EmbedError {
    /// The editor has no workspace bound; an embed cannot resolve a workspace asset.
    #[error("no workspace context: media embeds resolve workspace assets and need a workspace id")]
    NoWorkspace,
    /// The ref value is empty / whitespace only.
    #[error("embed reference is empty")]
    EmptyRef,
    /// An absolute / UNC / drive-letter path was supplied where an opaque asset id is required.
    #[error("absolute path '{0}' is forbidden: embeds are asset ids, never machine-local paths")]
    AbsolutePathRejected(String),
    /// A `..` traversal or path separator appeared in the asset id.
    #[error("'{0}' contains path separators or traversal; embeds are opaque asset ids")]
    TraversalRejected(String),
    /// An http(s)/file/javascript/data scheme appeared in the asset ref.
    #[error("'{0}' carries a scheme; media embeds resolve workspace asset ids only")]
    SchemeRejected(String),
    /// The asset id is otherwise malformed (too long, illegal characters).
    #[error("'{0}' is not a valid asset id")]
    InvalidRef(String),
    /// The backend returned 404 for the asset.
    #[error("asset not found: {0}")]
    NotFound(String),
    /// The backend returned 401/403 for the asset.
    #[error("asset is not accessible: {0}")]
    Forbidden(String),
    /// The backend returned 5xx or a malformed metadata body.
    #[error("server error: {0}")]
    ServerError(String),
    /// The fetch itself failed (backend unreachable / transport error).
    #[error("network error: {0}")]
    NetworkError(String),
    /// The asset mime does not match the embed kind (e.g. a video asset in an `images` embed).
    #[error("kind mismatch: {0}")]
    KindMismatch(String),
    /// The decoder could not decode/play the resolved bytes (corrupt/unsupported media).
    #[error("media load failed: {0}")]
    MediaLoadFailed(String),
}

impl EmbedError {
    /// The kebab-case kind string (verbatim from the React `EmbedErrorKind`). This is the
    /// stable text the error chip shows and the AccessKit label carries, so an out-of-process
    /// agent reads the SAME error vocabulary the web app used.
    pub fn kind_str(&self) -> &'static str {
        match self {
            EmbedError::NoWorkspace => "no_workspace",
            EmbedError::EmptyRef => "empty_ref",
            EmbedError::AbsolutePathRejected(_) => "absolute_path_rejected",
            EmbedError::TraversalRejected(_) => "traversal_rejected",
            EmbedError::SchemeRejected(_) => "scheme_rejected",
            EmbedError::InvalidRef(_) => "invalid_ref",
            EmbedError::NotFound(_) => "not_found",
            EmbedError::Forbidden(_) => "forbidden",
            EmbedError::ServerError(_) => "server_error",
            EmbedError::NetworkError(_) => "network_error",
            EmbedError::KindMismatch(_) => "kind_mismatch",
            EmbedError::MediaLoadFailed(_) => "media_load_failed",
        }
    }
}

/// Backend asset metadata (the native mirror of the React `EmbedAssetMetadata`, which itself
/// mirrors the backend `storage/loom.rs` `Asset` row). Only the fields the views need are
/// modeled; unknown fields are ignored by serde so a forward-compatible backend body still
/// deserializes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct EmbedAssetMetadata {
    pub asset_id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub kind: String,
    pub mime: String,
    #[serde(default)]
    pub original_filename: Option<String>,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

/// A fully-resolved single asset: its metadata plus the content/thumbnail URLs the view
/// loads (mirrors the React `EmbedResolution` ok branch). The URLs are constructed from the
/// verified backend endpoint patterns ([`asset_content_url`] / [`asset_thumbnail_url`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAsset {
    /// The backend asset metadata.
    pub asset: EmbedAssetMetadata,
    /// `GET /workspaces/{ws}/assets/{id}/content` — full-res original bytes.
    pub content_url: String,
    /// `GET /workspaces/{ws}/assets/{id}/thumbnail` — thumbnail bytes (grid/sequence first load).
    pub thumbnail_url: String,
}

/// The resolution state of ONE embed target, cached per asset id so a repeated render does
/// not re-fetch (AC-9). This is the value stored in the per-editor resolution cache.
#[derive(Debug, Clone)]
pub enum EmbedResolutionState {
    /// The fetch is in flight (the view shows an `egui::Spinner`).
    Resolving,
    /// Resolved OK — the view renders the media.
    Ok(ResolvedAsset),
    /// Resolution failed with a typed error — the view renders the error chip (never blank).
    Err(EmbedError),
}

impl EmbedResolutionState {
    /// True when this state is terminal (Ok or Err) — a terminal state is NOT re-fetched
    /// (AC-9 caching: the resolver skips an asset that already resolved or failed).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            EmbedResolutionState::Ok(_) | EmbedResolutionState::Err(_)
        )
    }
}

/// Validate ONE asset-ref fail-closed (the native mirror of the React `validateAssetRef`,
/// byte-for-byte in CHECK ORDER). Returns `Ok(trimmed_id)` when valid, else the typed
/// [`EmbedError`]. The order matters: a `:` is classified as a drive letter vs a scheme
/// BEFORE the separator/traversal check, exactly as the React code does, so `C:\x` is an
/// `AbsolutePathRejected` and `http://x` is a `SchemeRejected` (not a generic traversal).
pub fn validate_asset_ref(ref_value: &str) -> Result<String, EmbedError> {
    let value = ref_value.trim();
    if value.is_empty() {
        return Err(EmbedError::EmptyRef);
    }
    // A `:` carries either a drive letter (`C:\`, `C:/`) or a scheme (`http://`, `file:`,
    // `javascript:`); a real asset id never contains `:`.
    if value.contains(':') {
        if looks_like_drive_letter(value) {
            return Err(EmbedError::AbsolutePathRejected(value.to_owned()));
        }
        return Err(EmbedError::SchemeRejected(value.to_owned()));
    }
    if value.starts_with('/') || value.starts_with('\\') {
        return Err(EmbedError::AbsolutePathRejected(value.to_owned()));
    }
    // MC-003: reject `..` (traversal) as a SUBSTRING ANYWHERE, plus any path separator. This
    // catches `..hidden/secret`, `a..b`, and `../../etc/passwd` — not just a leading `..`.
    if value.contains('/') || value.contains('\\') || value.contains("..") {
        return Err(EmbedError::TraversalRejected(value.to_owned()));
    }
    if value.len() > ASSET_ID_MAX_LENGTH || !is_valid_asset_id_pattern(value) {
        return Err(EmbedError::InvalidRef(value.to_owned()));
    }
    Ok(value.to_owned())
}

/// True for a `C:\…` / `C:/…` drive-letter prefix (mirrors the React `/^[A-Za-z]:[\\/]/`).
fn looks_like_drive_letter(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// True when `value` matches the asset-id pattern `^[A-Za-z0-9][A-Za-z0-9._-]*$` (mirrors the
/// React `ASSET_ID_PATTERN`): an opaque id with no separators/colons/spaces.
fn is_valid_asset_id_pattern(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// Parse an album/slideshow `ref_value` into its ordered asset-id sequence (mirrors the React
/// `parseAssetRefList`): split on `,`, trim each, drop empties. The returned entries are the
/// RAW (un-validated) ids; the resolver validates each individually so one bad member becomes
/// a per-item error chip while the rest of the sequence still renders.
pub fn parse_asset_ref_list(ref_value: &str) -> Vec<String> {
    ref_value
        .split(',')
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty())
        .collect()
}

/// `GET /workspaces/{ws}/assets/{id}` — the asset metadata endpoint (verified backend pattern
/// `api/loom.rs get_asset_metadata`). Path components are NOT percent-encoded here because the
/// validated asset id is already constrained to `[A-Za-z0-9._-]` (no path/percent chars), and
/// the workspace id is a backend-owned opaque id; the React code encodes defensively, but the
/// native side has already fail-closed-rejected any id that would need encoding.
pub fn asset_metadata_url(base_url: &str, workspace_id: &str, asset_id: &str) -> String {
    format!("{base_url}/workspaces/{workspace_id}/assets/{asset_id}")
}

/// `GET /workspaces/{ws}/assets/{id}/content` — full-res content bytes.
pub fn asset_content_url(base_url: &str, workspace_id: &str, asset_id: &str) -> String {
    format!(
        "{}/content",
        asset_metadata_url(base_url, workspace_id, asset_id)
    )
}

/// `GET /workspaces/{ws}/assets/{id}/thumbnail` — thumbnail bytes.
pub fn asset_thumbnail_url(base_url: &str, workspace_id: &str, asset_id: &str) -> String {
    format!(
        "{}/thumbnail",
        asset_metadata_url(base_url, workspace_id, asset_id)
    )
}

/// A boxed, `Send` future yielding fetched asset metadata, returned by
/// [`AssetMetadataFetcher::fetch_metadata`]. Spelled out (rather than the `async-trait` macro)
/// so this module adds ZERO new dependency families — the resolution path stays on the crate's
/// existing `tokio` + `reqwest` graph only.
pub type MetadataFuture<'a> =
    Pin<Box<dyn Future<Output = Result<EmbedAssetMetadata, EmbedError>> + Send + 'a>>;

/// A boxed, `Send` future yielding fetched asset CONTENT bytes (the raw image/video bytes from
/// `GET /workspaces/{ws}/assets/{id}/content`), returned by [`AssetMetadataFetcher::fetch_content`].
/// These are the bytes the image-embed pipeline decodes off-thread (MC-001) into a texture.
pub type ContentFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<u8>, EmbedError>> + Send + 'a>>;

/// The transport an async resolution uses to fetch asset metadata AND content bytes. A trait
/// (rather than a hard `reqwest` call) so the FULL resolution + content-fetch + decode path —
/// the kind/mime check, the caching skip, the typed error mapping, the concurrency cap, the
/// content GET that feeds the off-thread decode — is unit-testable with a COUNTED in-memory mock
/// (AC-9 second-render-no-refetch, MC-002 concurrency, the kind-mismatch test, the
/// content->decode->texture wiring) WITHOUT a backend. The production implementation
/// ([`ReqwestAssetFetcher`]) wraps the existing `handshake_native::backend_client` reqwest client
/// (no new HTTP crate — MT scope).
pub trait AssetMetadataFetcher: Send + Sync {
    /// Fetch the asset metadata for `(workspace_id, asset_id)`. The id is ALREADY validated by
    /// [`validate_asset_ref`] before this is called. Returns the typed metadata or a typed
    /// [`EmbedError`] (NotFound / Forbidden / ServerError / NetworkError).
    fn fetch_metadata<'a>(&'a self, workspace_id: &'a str, asset_id: &'a str)
        -> MetadataFuture<'a>;

    /// Fetch the raw CONTENT bytes for `(workspace_id, asset_id)` (`GET .../content`). These feed
    /// the off-thread image decode (MC-001). The id is ALREADY validated before this is called.
    ///
    /// The DEFAULT impl returns a typed [`EmbedError::MediaLoadFailed`] so a metadata-only mock
    /// (one that only proves the resolution/validation path) does not have to implement content
    /// fetching; the image-content pipeline (and its dedicated mock) overrides this. The
    /// production [`ReqwestAssetFetcher`] overrides it with a real GET.
    fn fetch_content<'a>(&'a self, _workspace_id: &'a str, asset_id: &'a str) -> ContentFuture<'a> {
        let asset_id = asset_id.to_owned();
        Box::pin(async move {
            Err(EmbedError::MediaLoadFailed(format!(
                "fetcher does not provide content bytes for asset '{asset_id}'"
            )))
        })
    }
}

/// Resolve ONE media asset fail-closed: validate the ref, fetch metadata through `fetcher`,
/// check the mime family against `kind`, and build the content/thumbnail URLs. Every failure
/// is a typed [`EmbedError`] — never a panic. This is the native mirror of `resolveEmbedAsset`.
///
/// `base_url` is the REST base the content/thumbnail URLs are built against (the same base the
/// `fetcher` talks to). It is a pure string-format step here so the URLs are deterministic and
/// unit-asserted without a backend.
pub async fn resolve_one(
    kind: MediaEmbedKind,
    workspace_id: &str,
    ref_value: &str,
    base_url: &str,
    fetcher: &dyn AssetMetadataFetcher,
) -> Result<ResolvedAsset, EmbedError> {
    if workspace_id.trim().is_empty() {
        return Err(EmbedError::NoWorkspace);
    }
    let asset_id = validate_asset_ref(ref_value)?;
    let metadata = fetcher.fetch_metadata(workspace_id, &asset_id).await?;
    if !kind.mime_matches(&metadata.mime) {
        return Err(EmbedError::KindMismatch(format!(
            "asset '{asset_id}' is '{}', which does not match the '{}' embed kind",
            metadata.mime,
            kind.ref_kind()
        )));
    }
    Ok(ResolvedAsset {
        content_url: asset_content_url(base_url, workspace_id, &asset_id),
        thumbnail_url: asset_thumbnail_url(base_url, workspace_id, &asset_id),
        asset: metadata,
    })
}

/// The production [`AssetMetadataFetcher`]: a thin wrapper over a `reqwest::Client` that GETs
/// the verified backend metadata endpoint and maps the HTTP status to the typed
/// [`EmbedError`] vocabulary (mirrors the React `resolveEmbedAsset` status handling). It REUSES
/// the existing `handshake_native::backend_client` REST stack (reqwest 0.12, rustls) — NO new
/// HTTP crate is introduced (MT scope). Backend access is read-only GET; no backend code is
/// touched (consume-via-API-only).
#[derive(Clone)]
pub struct ReqwestAssetFetcher {
    client: reqwest::Client,
    base_url: String,
}

impl ReqwestAssetFetcher {
    /// Build a fetcher against `base_url` (e.g. `backend_client::BACKEND_BASE_URL`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// The production fetcher against the hardcoded backend base URL.
    pub fn production() -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL)
    }

    /// The REST base this fetcher resolves content/thumbnail URLs against (so the resolver and
    /// the fetcher agree on the base).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl AssetMetadataFetcher for ReqwestAssetFetcher {
    fn fetch_metadata<'a>(
        &'a self,
        workspace_id: &'a str,
        asset_id: &'a str,
    ) -> MetadataFuture<'a> {
        let url = asset_metadata_url(&self.base_url, workspace_id, asset_id);
        let client = self.client.clone();
        let asset_id = asset_id.to_owned();
        Box::pin(async move {
            let response = client.get(&url).send().await.map_err(|e| {
                EmbedError::NetworkError(format!("asset metadata request failed: {e}"))
            })?;
            let status = response.status();
            if status.as_u16() == 404 {
                return Err(EmbedError::NotFound(format!(
                    "asset '{asset_id}' not found"
                )));
            }
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(EmbedError::Forbidden(format!(
                    "asset '{asset_id}' is not accessible (HTTP {status})"
                )));
            }
            if !status.is_success() {
                return Err(EmbedError::ServerError(format!(
                    "asset metadata request returned HTTP {status}"
                )));
            }
            let metadata: EmbedAssetMetadata = response.json().await.map_err(|e| {
                EmbedError::ServerError(format!("asset metadata body is invalid: {e}"))
            })?;
            Ok(metadata)
        })
    }

    fn fetch_content<'a>(&'a self, workspace_id: &'a str, asset_id: &'a str) -> ContentFuture<'a> {
        let url = asset_content_url(&self.base_url, workspace_id, asset_id);
        let client = self.client.clone();
        let asset_id = asset_id.to_owned();
        Box::pin(async move {
            let response = client.get(&url).send().await.map_err(|e| {
                EmbedError::NetworkError(format!("asset content request failed: {e}"))
            })?;
            let status = response.status();
            if status.as_u16() == 404 {
                return Err(EmbedError::NotFound(format!(
                    "asset content '{asset_id}' not found"
                )));
            }
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(EmbedError::Forbidden(format!(
                    "asset content '{asset_id}' is not accessible (HTTP {status})"
                )));
            }
            if !status.is_success() {
                return Err(EmbedError::ServerError(format!(
                    "asset content request returned HTTP {status}"
                )));
            }
            let bytes = response.bytes().await.map_err(|e| {
                EmbedError::NetworkError(format!("asset content body read failed: {e}"))
            })?;
            Ok(bytes.to_vec())
        })
    }
}

/// One member of a resolved album/slideshow sequence: its (validated-or-raw) ref and its
/// resolution state. A broken member is a per-item `Err` so the rest of the sequence still
/// renders (fail-closed per item, not all-or-nothing blanking) — mirrors `EmbedSequenceItem`.
#[derive(Debug, Clone)]
pub struct SequenceItem {
    /// The member ref as it appeared in the comma list.
    pub ref_value: String,
    /// This member's resolution outcome.
    pub resolution: Result<ResolvedAsset, EmbedError>,
}

/// Resolve an album/slideshow ordered sequence with a BOUNDED concurrency of
/// [`MAX_CONCURRENT_RESOLUTIONS`] (MC-002): the members resolve in parallel but at most six
/// metadata fetches are in flight at once, gated by a [`tokio::sync::Semaphore`]. An empty
/// list and an oversized list (`> MAX_SEQUENCE_ITEMS`) are themselves typed errors (the
/// caller renders the whole-sequence error chip). Mirrors `resolveEmbedSequence`.
pub async fn resolve_sequence(
    kind: MediaEmbedKind,
    workspace_id: &str,
    ref_value: &str,
    base_url: &str,
    fetcher: Arc<dyn AssetMetadataFetcher>,
) -> Result<Vec<SequenceItem>, EmbedError> {
    if workspace_id.trim().is_empty() {
        return Err(EmbedError::NoWorkspace);
    }
    let refs = parse_asset_ref_list(ref_value);
    if refs.is_empty() {
        return Err(EmbedError::EmptyRef);
    }
    if refs.len() > MAX_SEQUENCE_ITEMS {
        return Err(EmbedError::InvalidRef(format!(
            "sequence has {} members; the maximum is {MAX_SEQUENCE_ITEMS}",
            refs.len()
        )));
    }

    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_RESOLUTIONS));
    let mut handles = Vec::with_capacity(refs.len());
    for member in refs {
        let sem = Arc::clone(&semaphore);
        let fetcher = Arc::clone(&fetcher);
        let workspace_id = workspace_id.to_owned();
        let base_url = base_url.to_owned();
        handles.push(tokio::spawn(async move {
            // Hold a permit for the whole member resolution so at most six run concurrently.
            let _permit = sem
                .acquire()
                .await
                .expect("embed-sequence semaphore is never closed before all permits return");
            let resolution =
                resolve_one(kind, &workspace_id, &member, &base_url, fetcher.as_ref()).await;
            SequenceItem {
                ref_value: member,
                resolution,
            }
        }));
    }

    let mut items = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(item) => items.push(item),
            // A spawned member task panicked (should not happen — resolve_one never panics);
            // surface it as a typed server error for that member rather than aborting the set.
            Err(join_err) => items.push(SequenceItem {
                ref_value: String::new(),
                resolution: Err(EmbedError::ServerError(format!(
                    "embed member task failed: {join_err}"
                ))),
            }),
        }
    }
    Ok(items)
}

/// Per-editor resolution cache (AC-9): keyed by asset id, so a second render of the same
/// embed reuses the terminal state instead of issuing a second fetch. Stored in
/// `RichEditorState` (owned by the shell frame) so it persists across frames — NOT inside a
/// renderer function. The renderer calls [`Self::needs_fetch`] before spawning; once a state
/// is terminal ([`EmbedResolutionState::is_terminal`]) the asset is never re-fetched.
#[derive(Debug, Default)]
pub struct EmbedResolutionCache {
    states: HashMap<String, EmbedResolutionState>,
}

impl EmbedResolutionCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// The cached state for `asset_id`, if any.
    pub fn get(&self, asset_id: &str) -> Option<&EmbedResolutionState> {
        self.states.get(asset_id)
    }

    /// Insert / replace the state for `asset_id`.
    pub fn insert(&mut self, asset_id: impl Into<String>, state: EmbedResolutionState) {
        self.states.insert(asset_id.into(), state);
    }

    /// True when `asset_id` has NO cached state yet, OR its cached state is still `Resolving`
    /// (i.e. a fetch has not completed). A TERMINAL state (Ok/Err) returns `false` — the AC-9
    /// caching invariant: a resolved/failed asset is never re-fetched. The renderer marks the
    /// asset `Resolving` before spawning so a re-render mid-flight does not double-fetch.
    pub fn needs_fetch(&self, asset_id: &str) -> bool {
        match self.states.get(asset_id) {
            None => true,
            Some(state) => {
                !state.is_terminal() && !matches!(state, EmbedResolutionState::Resolving)
            }
        }
    }

    /// True when `asset_id` is currently marked `Resolving` (a fetch is in flight).
    pub fn is_resolving(&self, asset_id: &str) -> bool {
        matches!(
            self.states.get(asset_id),
            Some(EmbedResolutionState::Resolving)
        )
    }

    /// Number of cached entries (test/diagnostic helper).
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// True when the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // ── AC-3 / AC-4 / MC-003: fail-closed ref validation (no backend) ────────────────────────

    #[test]
    fn empty_ref_is_rejected() {
        assert_eq!(validate_asset_ref(""), Err(EmbedError::EmptyRef));
        assert_eq!(validate_asset_ref("   "), Err(EmbedError::EmptyRef));
        assert_eq!(EmbedError::EmptyRef.kind_str(), "empty_ref");
    }

    #[test]
    fn traversal_dotdot_rejected_anywhere_mc003() {
        // MC-003: `..` ANYWHERE in the string is rejected, not just a leading path component.
        for bad in [
            "..",
            "../../etc/passwd",
            "..hidden",
            "a..b",
            "secret..",
            "foo/../bar",
        ] {
            let err = validate_asset_ref(bad).unwrap_err();
            assert_eq!(
                err.kind_str(),
                "traversal_rejected",
                "ref '{bad}' must be traversal_rejected, got {err:?}"
            );
        }
    }

    #[test]
    fn path_separators_rejected() {
        for bad in ["dir/asset", "dir\\asset", "a/b/c"] {
            assert_eq!(
                validate_asset_ref(bad).unwrap_err().kind_str(),
                "traversal_rejected"
            );
        }
    }

    #[test]
    fn absolute_paths_rejected() {
        // Leading slash / backslash and drive letters.
        assert_eq!(
            validate_asset_ref("/etc/passwd").unwrap_err().kind_str(),
            "absolute_path_rejected"
        );
        assert_eq!(
            validate_asset_ref("\\\\unc\\share").unwrap_err().kind_str(),
            "absolute_path_rejected"
        );
        assert_eq!(
            validate_asset_ref("C:\\Windows").unwrap_err().kind_str(),
            "absolute_path_rejected"
        );
        assert_eq!(
            validate_asset_ref("D:/x").unwrap_err().kind_str(),
            "absolute_path_rejected"
        );
    }

    #[test]
    fn schemes_rejected() {
        for bad in [
            "http://evil.test/x",
            "https://x",
            "file:///etc",
            "javascript:alert(1)",
            "data:text/html",
        ] {
            assert_eq!(
                validate_asset_ref(bad).unwrap_err().kind_str(),
                "scheme_rejected",
                "ref '{bad}' must be scheme_rejected"
            );
        }
    }

    #[test]
    fn valid_asset_ids_accept_and_trim() {
        assert_eq!(validate_asset_ref("asset123").unwrap(), "asset123");
        assert_eq!(validate_asset_ref("  a-b_c.d  ").unwrap(), "a-b_c.d");
        // Over-length is invalid_ref.
        let long = "a".repeat(ASSET_ID_MAX_LENGTH + 1);
        assert_eq!(
            validate_asset_ref(&long).unwrap_err().kind_str(),
            "invalid_ref"
        );
        // Illegal characters (space) -> invalid_ref.
        assert_eq!(
            validate_asset_ref("a b").unwrap_err().kind_str(),
            "invalid_ref"
        );
    }

    #[test]
    fn parse_ref_list_splits_trims_and_drops_empties() {
        assert_eq!(parse_asset_ref_list("a, b ,c"), vec!["a", "b", "c"]);
        assert_eq!(parse_asset_ref_list(" , a , , b , "), vec!["a", "b"]);
        assert!(parse_asset_ref_list("   ").is_empty());
    }

    #[test]
    fn urls_match_backend_pattern() {
        let base = "http://127.0.0.1:37501";
        assert_eq!(
            asset_metadata_url(base, "ws1", "a1"),
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1"
        );
        assert_eq!(
            asset_content_url(base, "ws1", "a1"),
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1/content"
        );
        assert_eq!(
            asset_thumbnail_url(base, "ws1", "a1"),
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1/thumbnail"
        );
    }

    #[test]
    fn mime_matches_kind() {
        assert!(MediaEmbedKind::Images.mime_matches("image/png"));
        assert!(MediaEmbedKind::Images.mime_matches("IMAGE/JPEG"));
        assert!(!MediaEmbedKind::Images.mime_matches("video/mp4"));
        assert!(MediaEmbedKind::Video.mime_matches("video/webm"));
        assert!(!MediaEmbedKind::Video.mime_matches("image/png"));
        assert!(MediaEmbedKind::Album.mime_matches("image/gif"));
        assert!(MediaEmbedKind::Slideshow.mime_matches("image/webp"));
    }

    #[test]
    fn ref_kind_round_trips() {
        for k in [
            MediaEmbedKind::Images,
            MediaEmbedKind::Video,
            MediaEmbedKind::Album,
            MediaEmbedKind::Slideshow,
        ] {
            assert_eq!(MediaEmbedKind::from_ref_kind(k.ref_kind()), Some(k));
        }
        assert_eq!(MediaEmbedKind::from_ref_kind("wp"), None);
        assert_eq!(MediaEmbedKind::from_ref_kind("note"), None);
        // The contract scope text's "image" (singular) is NOT a media kind — only "images".
        assert_eq!(MediaEmbedKind::from_ref_kind("image"), None);
    }

    // ── A counted mock fetcher (no backend) used by the resolution / caching / concurrency tests ─

    /// A mock metadata fetcher that COUNTS calls (AC-9) and can simulate slow fetches +
    /// concurrency tracking (MC-002). It NEVER touches the network.
    struct MockFetcher {
        calls: AtomicUsize,
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
        delay_ms: u64,
        mime: String,
    }

    impl MockFetcher {
        fn new(mime: &str, delay_ms: u64) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                in_flight: AtomicUsize::new(0),
                max_in_flight: AtomicUsize::new(0),
                delay_ms,
                mime: mime.to_owned(),
            }
        }
        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
        fn observed_max_in_flight(&self) -> usize {
            self.max_in_flight.load(Ordering::SeqCst)
        }
    }

    impl AssetMetadataFetcher for MockFetcher {
        fn fetch_metadata<'a>(
            &'a self,
            workspace_id: &'a str,
            asset_id: &'a str,
        ) -> MetadataFuture<'a> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                // Track the high-water mark of simultaneous in-flight fetches (MC-002 proof).
                self.max_in_flight.fetch_max(now, Ordering::SeqCst);
                if self.delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
                }
                self.in_flight.fetch_sub(1, Ordering::SeqCst);
                Ok(EmbedAssetMetadata {
                    asset_id: asset_id.to_owned(),
                    workspace_id: workspace_id.to_owned(),
                    kind: "image".to_owned(),
                    mime: self.mime.clone(),
                    original_filename: Some(format!("{asset_id}.png")),
                    content_hash: "deadbeef".to_owned(),
                    size_bytes: 1024,
                    width: Some(640),
                    height: Some(480),
                })
            })
        }
    }

    #[tokio::test]
    async fn resolve_one_validates_before_any_fetch_ac3() {
        // AC-3: a `..` ref is rejected with TraversalRejected BEFORE any fetch is issued.
        let fetcher = MockFetcher::new("image/png", 0);
        let err = resolve_one(
            MediaEmbedKind::Images,
            "ws",
            "../secret",
            "http://b",
            &fetcher,
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind_str(), "traversal_rejected");
        assert_eq!(
            fetcher.call_count(),
            0,
            "AC-3: NO HTTP call may be made for a rejected ref"
        );
    }

    #[tokio::test]
    async fn resolve_one_rejects_scheme_before_fetch_ac4() {
        // AC-4: an http:// ref is SchemeRejected with no fetch.
        let fetcher = MockFetcher::new("image/png", 0);
        let err = resolve_one(
            MediaEmbedKind::Images,
            "ws",
            "http://evil/x",
            "http://b",
            &fetcher,
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind_str(), "scheme_rejected");
        assert_eq!(fetcher.call_count(), 0);
    }

    #[tokio::test]
    async fn resolve_one_ok_builds_urls() {
        let fetcher = MockFetcher::new("image/png", 0);
        let resolved = resolve_one(
            MediaEmbedKind::Images,
            "ws1",
            "a1",
            "http://127.0.0.1:37501",
            &fetcher,
        )
        .await
        .unwrap();
        assert_eq!(resolved.asset.asset_id, "a1");
        assert_eq!(
            resolved.content_url,
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1/content"
        );
        assert_eq!(
            resolved.thumbnail_url,
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1/thumbnail"
        );
        assert_eq!(fetcher.call_count(), 1);
    }

    #[tokio::test]
    async fn resolve_one_kind_mismatch_is_fail_closed() {
        // A video asset inside an `images` embed -> kind_mismatch (fetch happened, mime checked).
        let fetcher = MockFetcher::new("video/mp4", 0);
        let err = resolve_one(MediaEmbedKind::Images, "ws", "a1", "http://b", &fetcher)
            .await
            .unwrap_err();
        assert_eq!(err.kind_str(), "kind_mismatch");
    }

    #[tokio::test]
    async fn no_workspace_is_rejected_before_fetch() {
        let fetcher = MockFetcher::new("image/png", 0);
        let err = resolve_one(MediaEmbedKind::Images, "  ", "a1", "http://b", &fetcher)
            .await
            .unwrap_err();
        assert_eq!(err.kind_str(), "no_workspace");
        assert_eq!(fetcher.call_count(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn sequence_concurrency_capped_at_six_mc002() {
        // MC-002: 12 members, each fetch sleeps 30ms; at most MAX_CONCURRENT_RESOLUTIONS (6) may
        // be in flight at any instant. The mock tracks the high-water in-flight count.
        let refs: Vec<String> = (0..12).map(|i| format!("a{i}")).collect();
        let ref_value = refs.join(",");
        let fetcher: Arc<MockFetcher> = Arc::new(MockFetcher::new("image/png", 30));
        let fetcher_dyn: Arc<dyn AssetMetadataFetcher> = fetcher.clone();
        let items = resolve_sequence(
            MediaEmbedKind::Album,
            "ws",
            &ref_value,
            "http://b",
            fetcher_dyn,
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 12, "all 12 members resolved");
        assert_eq!(fetcher.call_count(), 12);
        assert!(
            fetcher.observed_max_in_flight() <= MAX_CONCURRENT_RESOLUTIONS,
            "MC-002: at most {MAX_CONCURRENT_RESOLUTIONS} concurrent fetches; observed max {}",
            fetcher.observed_max_in_flight()
        );
    }

    #[tokio::test]
    async fn sequence_per_item_failclosed() {
        // A 3-member sequence where the middle ref is a traversal: the bad member is a per-item
        // Err while the other two resolve OK (not all-or-nothing).
        let fetcher: Arc<dyn AssetMetadataFetcher> = Arc::new(MockFetcher::new("image/png", 0));
        let items = resolve_sequence(
            MediaEmbedKind::Slideshow,
            "ws",
            "a1, ../bad , a3",
            "http://b",
            fetcher,
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 3);
        assert!(items[0].resolution.is_ok());
        assert_eq!(
            items[1].resolution.as_ref().unwrap_err().kind_str(),
            "traversal_rejected"
        );
        assert!(items[2].resolution.is_ok());
    }

    #[tokio::test]
    async fn empty_and_oversized_sequences_are_typed_errors() {
        let fetcher: Arc<dyn AssetMetadataFetcher> = Arc::new(MockFetcher::new("image/png", 0));
        assert_eq!(
            resolve_sequence(
                MediaEmbedKind::Album,
                "ws",
                "  ,  ",
                "http://b",
                Arc::clone(&fetcher)
            )
            .await
            .unwrap_err()
            .kind_str(),
            "empty_ref"
        );
        let huge: String = (0..(MAX_SEQUENCE_ITEMS + 1))
            .map(|i| format!("a{i}"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            resolve_sequence(MediaEmbedKind::Album, "ws", &huge, "http://b", fetcher)
                .await
                .unwrap_err()
                .kind_str(),
            "invalid_ref"
        );
    }

    // ── AC-9: the resolution cache skips a second fetch for a terminal asset ──────────────────

    #[test]
    fn cache_needs_fetch_only_when_absent() {
        let mut cache = EmbedResolutionCache::new();
        assert!(cache.needs_fetch("a1"), "absent -> needs fetch");
        cache.insert("a1", EmbedResolutionState::Resolving);
        assert!(
            !cache.needs_fetch("a1"),
            "resolving -> in flight, do not re-spawn"
        );
        assert!(cache.is_resolving("a1"));
        cache.insert(
            "a1",
            EmbedResolutionState::Ok(ResolvedAsset {
                asset: EmbedAssetMetadata {
                    asset_id: "a1".into(),
                    workspace_id: "ws".into(),
                    kind: "image".into(),
                    mime: "image/png".into(),
                    original_filename: None,
                    content_hash: String::new(),
                    size_bytes: 0,
                    width: None,
                    height: None,
                },
                content_url: "u".into(),
                thumbnail_url: "t".into(),
            }),
        );
        assert!(
            !cache.needs_fetch("a1"),
            "AC-9: a resolved (Ok) asset is NEVER re-fetched"
        );
        cache.insert(
            "a2",
            EmbedResolutionState::Err(EmbedError::NotFound("a2".into())),
        );
        assert!(
            !cache.needs_fetch("a2"),
            "AC-9: a failed (Err) asset is NEVER re-fetched"
        );
        assert_eq!(cache.len(), 2);
    }

    /// AC-9 end-to-end with the COUNTED mock: the renderer-shaped "fetch-once" loop spawns a
    /// fetch only when `needs_fetch`, marks `Resolving`, then stores the terminal state — a
    /// second pass issues NO second call.
    #[tokio::test]
    async fn second_render_issues_no_second_fetch_ac9() {
        let fetcher = MockFetcher::new("image/png", 0);
        let mut cache = EmbedResolutionCache::new();

        // Pass 1: not cached -> fetch once, store Ok.
        async fn render_pass(cache: &mut EmbedResolutionCache, fetcher: &MockFetcher) {
            let asset_id = "a1";
            if cache.needs_fetch(asset_id) {
                cache.insert(asset_id, EmbedResolutionState::Resolving);
                let res =
                    resolve_one(MediaEmbedKind::Images, "ws", asset_id, "http://b", fetcher).await;
                cache.insert(
                    asset_id,
                    match res {
                        Ok(r) => EmbedResolutionState::Ok(r),
                        Err(e) => EmbedResolutionState::Err(e),
                    },
                );
            }
        }
        render_pass(&mut cache, &fetcher).await;
        assert_eq!(fetcher.call_count(), 1, "first render fetches once");
        // Pass 2: cached terminal -> NO fetch.
        render_pass(&mut cache, &fetcher).await;
        assert_eq!(
            fetcher.call_count(),
            1,
            "AC-9: second render issues NO second fetch"
        );
    }
}
