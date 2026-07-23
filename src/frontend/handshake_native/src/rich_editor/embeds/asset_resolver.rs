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
use std::time::Duration;

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

/// Bounded transport/decode limits. These reject oversized bodies and decompression bombs with a
/// typed visible error before an allocation can grow without bound.
pub const MAX_METADATA_BYTES: usize = 1024 * 1024;
pub const MAX_THUMBNAIL_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_POSTER_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PREVIEW_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_FULL_IMAGE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: u32 = 16_384;
pub const MAX_IMAGE_PIXELS: u64 = 80_000_000;

/// Hard transport/operation deadlines for every production embed request. The request timeout
/// covers response-body streaming as well as the initial response, while the shorter connect
/// timeout prevents an unreachable host from consuming the entire operation budget.
pub const EMBED_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const EMBED_OPERATION_TIMEOUT: Duration = Duration::from_secs(15);

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

/// The byte tier a view requests. Grid cells use `Thumbnail`, video uses `Poster`, and only an
/// active slideshow frame or open modal requests `Full`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaTier {
    Thumbnail,
    Preview,
    Poster,
    Full,
}

impl MediaTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Thumbnail => "thumb",
            Self::Preview => "preview",
            Self::Poster => "poster",
            Self::Full => "full",
        }
    }

    pub fn max_body_bytes(self) -> usize {
        match self {
            Self::Thumbnail => MAX_THUMBNAIL_BYTES,
            Self::Preview => MAX_PREVIEW_BYTES,
            Self::Poster => MAX_POSTER_BYTES,
            Self::Full => MAX_FULL_IMAGE_BYTES,
        }
    }
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
    /// A backend fetch or decode did not finish inside the bounded embed deadline.
    #[error("embed operation timed out: {0}")]
    TimedOut(String),
    /// A response body or decoded image exceeded the explicit media safety bounds.
    #[error("embed resource limit exceeded: {0}")]
    ResourceLimit(String),
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
            EmbedError::TimedOut(_) => "timed_out",
            EmbedError::ResourceLimit(_) => "resource_limit",
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
    /// Mid-resolution tier used by an enlarged preview before/while full pixels are available.
    pub preview_url: String,
    /// Video poster tier; this is fetched without loading the full video body.
    pub poster_url: String,
}

/// Backend-owned ordered album/slideshow membership (`collection:<id>` parity).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct EmbedCollection {
    pub collection_id: String,
    #[serde(default)]
    pub title: Option<String>,
    pub members: Vec<String>,
}

pub const COLLECTION_REF_PREFIX: &str = "collection:";

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

/// Tier-aware content URL used by the current React parity surface.
pub fn asset_tier_url(
    base_url: &str,
    workspace_id: &str,
    asset_id: &str,
    tier: MediaTier,
) -> String {
    if tier == MediaTier::Full {
        asset_content_url(base_url, workspace_id, asset_id)
    } else {
        format!(
            "{}?tier={}",
            asset_content_url(base_url, workspace_id, asset_id),
            tier.as_str()
        )
    }
}

pub fn collection_url(base_url: &str, workspace_id: &str, collection_id: &str) -> String {
    format!("{base_url}/workspaces/{workspace_id}/loom/collections/{collection_id}")
}

pub fn collection_ref_id(ref_value: &str) -> Option<&str> {
    ref_value
        .trim()
        .strip_prefix(COLLECTION_REF_PREFIX)
        .map(str::trim)
        .filter(|value| !value.is_empty())
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

pub type CollectionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<EmbedCollection, EmbedError>> + Send + 'a>>;

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

    /// Fetch a bounded presentation tier for a validated media kind. `kind` is part of the
    /// transport contract so production fallback policy cannot accidentally treat a video poster
    /// or an album-grid thumbnail like a single-image thumbnail. The default preserves
    /// compatibility for focused metadata mocks.
    fn fetch_tier<'a>(
        &'a self,
        workspace_id: &'a str,
        asset_id: &'a str,
        _kind: MediaEmbedKind,
        _tier: MediaTier,
    ) -> ContentFuture<'a> {
        self.fetch_content(workspace_id, asset_id)
    }

    /// Resolve backend-owned album/slideshow membership.
    fn fetch_collection<'a>(
        &'a self,
        _workspace_id: &'a str,
        collection_id: &'a str,
    ) -> CollectionFuture<'a> {
        let collection_id = collection_id.to_owned();
        Box::pin(async move {
            Err(EmbedError::ServerError(format!(
                "fetcher does not provide collection '{collection_id}'"
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
    resolve_one_with_timeout(
        kind,
        workspace_id,
        ref_value,
        base_url,
        fetcher,
        EMBED_OPERATION_TIMEOUT,
    )
    .await
}

async fn resolve_one_with_timeout(
    kind: MediaEmbedKind,
    workspace_id: &str,
    ref_value: &str,
    base_url: &str,
    fetcher: &dyn AssetMetadataFetcher,
    operation_timeout: Duration,
) -> Result<ResolvedAsset, EmbedError> {
    if workspace_id.trim().is_empty() {
        return Err(EmbedError::NoWorkspace);
    }
    let asset_id = validate_asset_ref(ref_value)?;
    let metadata = run_with_deadline(
        operation_timeout,
        format!("resolving metadata for asset '{asset_id}'"),
        fetcher.fetch_metadata(workspace_id, &asset_id),
    )
    .await?;
    if metadata.asset_id != asset_id {
        return Err(EmbedError::ServerError(format!(
            "asset metadata id '{}' does not match requested '{asset_id}'",
            metadata.asset_id
        )));
    }
    if metadata.workspace_id != workspace_id {
        return Err(EmbedError::ServerError(format!(
            "asset metadata workspace '{}' does not match requested '{workspace_id}'",
            metadata.workspace_id
        )));
    }
    if metadata.mime.trim().is_empty() {
        return Err(EmbedError::ServerError(format!(
            "asset '{asset_id}' metadata has an empty mime"
        )));
    }
    if !kind.mime_matches(&metadata.mime) {
        return Err(EmbedError::KindMismatch(format!(
            "asset '{asset_id}' is '{}', which does not match the '{}' embed kind",
            metadata.mime,
            kind.ref_kind()
        )));
    }
    Ok(ResolvedAsset {
        content_url: asset_content_url(base_url, workspace_id, &asset_id),
        thumbnail_url: asset_tier_url(base_url, workspace_id, &asset_id, MediaTier::Thumbnail),
        preview_url: asset_tier_url(base_url, workspace_id, &asset_id, MediaTier::Preview),
        poster_url: asset_tier_url(base_url, workspace_id, &asset_id, MediaTier::Poster),
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
    client: Option<reqwest::Client>,
    client_init_error: Option<Arc<str>>,
    base_url: String,
    request_timeout: Duration,
}

impl ReqwestAssetFetcher {
    /// Build a fetcher against `base_url` (e.g. `backend_client::BACKEND_BASE_URL`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_timeouts(
            base_url,
            EMBED_HTTP_CONNECT_TIMEOUT,
            EMBED_OPERATION_TIMEOUT,
        )
    }

    fn with_timeouts(
        base_url: impl Into<String>,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Self {
        let base_url = base_url.into();
        let client = reqwest::Client::builder()
            .connect_timeout(connect_timeout)
            .timeout(request_timeout)
            .build();
        match client {
            Ok(client) => Self {
                client: Some(client),
                client_init_error: None,
                base_url,
                request_timeout,
            },
            Err(error) => Self {
                client: None,
                client_init_error: Some(Arc::from(error.to_string())),
                base_url,
                request_timeout,
            },
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

    fn cloned_client(&self) -> Result<reqwest::Client, EmbedError> {
        self.client.clone().ok_or_else(|| {
            EmbedError::NetworkError(format!(
                "bounded embed HTTP client could not initialize: {}",
                self.client_init_error.as_deref().unwrap_or("unknown error")
            ))
        })
    }
}

async fn run_with_deadline<T, F>(
    deadline: Duration,
    label: String,
    future: F,
) -> Result<T, EmbedError>
where
    F: Future<Output = Result<T, EmbedError>>,
{
    tokio::time::timeout(deadline, future)
        .await
        .unwrap_or_else(|_| {
            Err(EmbedError::TimedOut(format!(
                "{label} exceeded {deadline:?}"
            )))
        })
}

fn map_reqwest_error(label: &str, error: reqwest::Error) -> EmbedError {
    if error.is_timeout() {
        EmbedError::TimedOut(format!("{label} exceeded its transport deadline: {error}"))
    } else {
        EmbedError::NetworkError(format!("{label} failed: {error}"))
    }
}

async fn read_bounded_response(
    mut response: reqwest::Response,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, EmbedError> {
    if let Some(length) = response.content_length() {
        if length > max_bytes as u64 {
            return Err(EmbedError::ResourceLimit(format!(
                "{label} content-length {length} exceeds {max_bytes} bytes"
            )));
        }
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| map_reqwest_error(&format!("{label} body read"), error))?
    {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(EmbedError::ResourceLimit(format!(
                "{label} streamed body exceeds {max_bytes} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn map_response_status(status: reqwest::StatusCode, label: &str) -> Result<(), EmbedError> {
    match status.as_u16() {
        404 => Err(EmbedError::NotFound(format!("{label} not found"))),
        401 | 403 => Err(EmbedError::Forbidden(format!(
            "{label} is not accessible (HTTP {status})"
        ))),
        _ if !status.is_success() => Err(EmbedError::ServerError(format!(
            "{label} request returned HTTP {status}"
        ))),
        _ => Ok(()),
    }
}

fn tier_body_limit(tier: MediaTier, used_original_fallback: bool) -> usize {
    if used_original_fallback {
        MAX_FULL_IMAGE_BYTES
    } else {
        tier.max_body_bytes()
    }
}

impl AssetMetadataFetcher for ReqwestAssetFetcher {
    fn fetch_metadata<'a>(
        &'a self,
        workspace_id: &'a str,
        asset_id: &'a str,
    ) -> MetadataFuture<'a> {
        let url = asset_metadata_url(&self.base_url, workspace_id, asset_id);
        let client = self.cloned_client();
        let asset_id = asset_id.to_owned();
        let request_timeout = self.request_timeout;
        Box::pin(async move {
            run_with_deadline(
                request_timeout,
                format!("asset '{asset_id}' metadata request"),
                async move {
                    let client = client?;
                    let response = client
                        .get(&url)
                        .send()
                        .await
                        .map_err(|error| map_reqwest_error("asset metadata request", error))?;
                    let status = response.status();
                    map_response_status(status, &format!("asset '{asset_id}' metadata"))?;
                    let body =
                        read_bounded_response(response, MAX_METADATA_BYTES, "asset metadata")
                            .await?;
                    serde_json::from_slice(&body).map_err(|error| {
                        EmbedError::ServerError(format!("asset metadata body is invalid: {error}"))
                    })
                },
            )
            .await
        })
    }

    fn fetch_content<'a>(&'a self, workspace_id: &'a str, asset_id: &'a str) -> ContentFuture<'a> {
        let url = asset_content_url(&self.base_url, workspace_id, asset_id);
        let client = self.cloned_client();
        let asset_id = asset_id.to_owned();
        let request_timeout = self.request_timeout;
        Box::pin(async move {
            run_with_deadline(
                request_timeout,
                format!("asset '{asset_id}' content request"),
                async move {
                    let client = client?;
                    let response = client
                        .get(&url)
                        .send()
                        .await
                        .map_err(|error| map_reqwest_error("asset content request", error))?;
                    let status = response.status();
                    map_response_status(status, &format!("asset content '{asset_id}'"))?;
                    read_bounded_response(response, MAX_FULL_IMAGE_BYTES, "asset content").await
                },
            )
            .await
        })
    }

    fn fetch_tier<'a>(
        &'a self,
        workspace_id: &'a str,
        asset_id: &'a str,
        kind: MediaEmbedKind,
        tier: MediaTier,
    ) -> ContentFuture<'a> {
        let url = asset_tier_url(&self.base_url, workspace_id, asset_id, tier);
        let original_url = asset_content_url(&self.base_url, workspace_id, asset_id);
        let client = self.cloned_client();
        let asset_id = asset_id.to_owned();
        let request_timeout = self.request_timeout;
        Box::pin(async move {
            run_with_deadline(
                request_timeout,
                format!("{} tier request for asset '{asset_id}'", tier.as_str()),
                async move {
                    let client = client?;
                    let mut response = client.get(&url).send().await.map_err(|error| {
                        map_reqwest_error(&format!("{} request", tier.as_str()), error)
                    })?;

                    // React parity only falls a single-image thumbnail back to its original. Album and
                    // slideshow grids keep a missing thumbnail typed/visible, and a missing video poster
                    // must never trigger a full video-body download.
                    let used_original_fallback = kind == MediaEmbedKind::Images
                        && tier == MediaTier::Thumbnail
                        && response.status() == reqwest::StatusCode::NOT_FOUND;
                    if used_original_fallback {
                        response = client.get(&original_url).send().await.map_err(|error| {
                            map_reqwest_error(
                                &format!("{} fallback content request", tier.as_str()),
                                error,
                            )
                        })?;
                    }
                    map_response_status(
                        response.status(),
                        &format!("{} tier for asset '{asset_id}'", tier.as_str()),
                    )?;
                    read_bounded_response(
                        response,
                        tier_body_limit(tier, used_original_fallback),
                        tier.as_str(),
                    )
                    .await
                },
            )
            .await
        })
    }

    fn fetch_collection<'a>(
        &'a self,
        workspace_id: &'a str,
        collection_id: &'a str,
    ) -> CollectionFuture<'a> {
        let url = collection_url(&self.base_url, workspace_id, collection_id);
        let client = self.cloned_client();
        let collection_id = collection_id.to_owned();
        let request_timeout = self.request_timeout;
        Box::pin(async move {
            run_with_deadline(
                request_timeout,
                format!("collection '{collection_id}' request"),
                async move {
                    let client = client?;
                    let response = client
                        .get(&url)
                        .send()
                        .await
                        .map_err(|error| map_reqwest_error("collection request", error))?;
                    map_response_status(
                        response.status(),
                        &format!("collection '{collection_id}'"),
                    )?;
                    let body =
                        read_bounded_response(response, MAX_METADATA_BYTES, "collection").await?;
                    serde_json::from_slice(&body).map_err(|error| {
                        EmbedError::ServerError(format!("collection body is invalid: {error}"))
                    })
                },
            )
            .await
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

/// Owns the per-member tasks spawned by [`resolve_sequence`]. Dropping the parent resolution
/// future (for example on workspace rebind or timeout) aborts every still-running member instead
/// of detaching background HTTP work from the editor that requested it.
struct SequenceTaskSet(Vec<tokio::task::JoinHandle<SequenceItem>>);

impl Drop for SequenceTaskSet {
    fn drop(&mut self) {
        for task in self.0.drain(..) {
            task.abort();
        }
    }
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
    resolve_sequence_with_budget(
        kind,
        workspace_id,
        ref_value,
        base_url,
        fetcher,
        Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_RESOLUTIONS)),
    )
    .await
}

/// Runtime variant using the editor's shared media budget. The same six permits are also used by
/// single metadata, tier-body, and decode work, so multiple embeds cannot each create a private
/// six-request fan-out.
pub async fn resolve_sequence_with_budget(
    kind: MediaEmbedKind,
    workspace_id: &str,
    ref_value: &str,
    base_url: &str,
    fetcher: Arc<dyn AssetMetadataFetcher>,
    semaphore: Arc<tokio::sync::Semaphore>,
) -> Result<Vec<SequenceItem>, EmbedError> {
    resolve_sequence_with_budget_and_timeout(
        kind,
        workspace_id,
        ref_value,
        base_url,
        fetcher,
        semaphore,
        EMBED_OPERATION_TIMEOUT,
    )
    .await
}

async fn resolve_sequence_with_budget_and_timeout(
    kind: MediaEmbedKind,
    workspace_id: &str,
    ref_value: &str,
    base_url: &str,
    fetcher: Arc<dyn AssetMetadataFetcher>,
    semaphore: Arc<tokio::sync::Semaphore>,
    operation_timeout: Duration,
) -> Result<Vec<SequenceItem>, EmbedError> {
    run_with_deadline(
        operation_timeout,
        format!("resolving {} sequence", kind.ref_kind()),
        resolve_sequence_with_budget_inner(
            kind,
            workspace_id,
            ref_value,
            base_url,
            fetcher,
            semaphore,
        ),
    )
    .await
}

async fn resolve_sequence_with_budget_inner(
    kind: MediaEmbedKind,
    workspace_id: &str,
    ref_value: &str,
    base_url: &str,
    fetcher: Arc<dyn AssetMetadataFetcher>,
    semaphore: Arc<tokio::sync::Semaphore>,
) -> Result<Vec<SequenceItem>, EmbedError> {
    if workspace_id.trim().is_empty() {
        return Err(EmbedError::NoWorkspace);
    }
    let refs = if let Some(collection_id) = collection_ref_id(ref_value) {
        let collection_id = validate_asset_ref(collection_id)?;
        let _permit = Arc::clone(&semaphore)
            .acquire_owned()
            .await
            .map_err(|_| EmbedError::ServerError("embed work budget closed".to_owned()))?;
        let collection = fetcher
            .fetch_collection(workspace_id, &collection_id)
            .await?;
        if collection.collection_id != collection_id {
            return Err(EmbedError::ServerError(format!(
                "collection response id '{}' does not match requested '{collection_id}'",
                collection.collection_id
            )));
        }
        collection.members
    } else {
        parse_asset_ref_list(ref_value)
    };
    if refs.is_empty() {
        return Err(EmbedError::EmptyRef);
    }
    if refs.len() > MAX_SEQUENCE_ITEMS {
        return Err(EmbedError::InvalidRef(format!(
            "sequence has {} members; the maximum is {MAX_SEQUENCE_ITEMS}",
            refs.len()
        )));
    }

    let mut handles = SequenceTaskSet(Vec::with_capacity(refs.len()));
    for member in refs {
        let sem = Arc::clone(&semaphore);
        let fetcher = Arc::clone(&fetcher);
        let workspace_id = workspace_id.to_owned();
        let base_url = base_url.to_owned();
        handles.0.push(tokio::spawn(async move {
            // Hold a permit for the whole member resolution so at most six run concurrently.
            let _permit = match sem.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    return SequenceItem {
                        ref_value: member,
                        resolution: Err(EmbedError::ServerError(
                            "embed work budget closed".to_owned(),
                        )),
                    };
                }
            };
            let resolution =
                resolve_one(kind, &workspace_id, &member, &base_url, fetcher.as_ref()).await;
            SequenceItem {
                ref_value: member,
                resolution,
            }
        }));
    }

    let mut items = Vec::with_capacity(handles.0.len());
    // Await by mutable reference so every handle remains owned by `SequenceTaskSet`; if this
    // parent future is cancelled while one member is awaited, Drop still aborts that member too.
    for handle in &mut handles.0 {
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

    pub fn remove(&mut self, key: &str) -> Option<EmbedResolutionState> {
        self.states.remove(key)
    }

    /// Remove every workspace-local resolution when the owning editor changes workspace.
    pub fn clear(&mut self) {
        self.states.clear();
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
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn scripted_http_server(
        responses: Vec<(&'static str, u16, Vec<u8>)>,
    ) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind scripted HTTP server");
        let address = listener.local_addr().expect("scripted server address");
        listener
            .set_nonblocking(true)
            .expect("bound scripted accept");
        let handle = std::thread::spawn(move || {
            let mut observed = Vec::new();
            for (expected_path, status, body) in responses {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "timed out waiting for scripted request {expected_path}"
                            );
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(error) => panic!("accept scripted request failed: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .expect("bound scripted request read");
                let mut request = [0_u8; 4096];
                let read = stream.read(&mut request).expect("read scripted request");
                let request = String::from_utf8_lossy(&request[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .expect("HTTP request path")
                    .to_owned();
                assert_eq!(path, expected_path);
                observed.push(path);
                let reason = match status {
                    200 => "OK",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    _ => "Test Status",
                };
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .expect("write scripted response headers");
                stream
                    .write_all(&body)
                    .expect("write scripted response body");
            }
            observed
        });
        (format!("http://{address}"), handle)
    }

    fn raw_http_server(
        response: Vec<u8>,
        delay_before_response: Duration,
    ) -> (String, std::thread::JoinHandle<String>) {
        raw_http_server_parts(Vec::new(), delay_before_response, response)
    }

    fn raw_http_server_parts(
        response_prefix: Vec<u8>,
        delay_before_suffix: Duration,
        response_suffix: Vec<u8>,
    ) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind raw HTTP server");
        let address = listener.local_addr().expect("raw server address");
        listener.set_nonblocking(true).expect("bound raw accept");
        let handle = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "timed out waiting for raw HTTP request"
                        );
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept raw request failed: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("bound raw request read");
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .expect("bound raw response write");
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).expect("read raw request");
            let request = String::from_utf8_lossy(&request[..read]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .expect("raw HTTP request path")
                .to_owned();
            // A timeout or an oversized Content-Length may make the client close before the
            // test server writes. That is the expected negative path, so BrokenPipe is not a
            // server-test failure.
            let _ = stream.write_all(&response_prefix);
            let _ = stream.flush();
            if !delay_before_suffix.is_zero() {
                std::thread::sleep(delay_before_suffix);
            }
            let _ = stream.write_all(&response_suffix);
            path
        });
        (format!("http://{address}"), handle)
    }

    fn metadata_body(workspace_id: &str, asset_id: &str, mime: &str) -> Vec<u8> {
        serde_json::to_vec(&EmbedAssetMetadata {
            asset_id: asset_id.to_owned(),
            workspace_id: workspace_id.to_owned(),
            kind: "image".to_owned(),
            mime: mime.to_owned(),
            original_filename: Some(format!("{asset_id}.png")),
            content_hash: "test-hash".to_owned(),
            size_bytes: 16,
            width: Some(1),
            height: Some(1),
        })
        .expect("serialize metadata fixture")
    }

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

    #[tokio::test]
    async fn single_image_thumbnail_404_falls_back_to_bounded_original() {
        let tier_path = "/workspaces/ws/assets/img/content?tier=thumb";
        let original_path = "/workspaces/ws/assets/img/content";
        let fallback_len = MAX_THUMBNAIL_BYTES + 1;
        let (base_url, server) = scripted_http_server(vec![
            (tier_path, 404, Vec::new()),
            (original_path, 200, vec![0x5a; fallback_len]),
        ]);
        let fetcher = ReqwestAssetFetcher::new(base_url);
        let bytes = fetcher
            .fetch_tier("ws", "img", MediaEmbedKind::Images, MediaTier::Thumbnail)
            .await
            .expect("single-image thumbnail fallback succeeds");
        assert_eq!(
            bytes.len(),
            fallback_len,
            "fallback accepts an original larger than the thumbnail limit but within the full-image limit"
        );
        assert!(bytes.iter().all(|byte| *byte == 0x5a));
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![tier_path.to_owned(), original_path.to_owned()]
        );
        assert_eq!(
            tier_body_limit(MediaTier::Thumbnail, true),
            MAX_FULL_IMAGE_BYTES,
            "a selected original fallback must use the full-image byte bound"
        );
        assert_eq!(
            tier_body_limit(MediaTier::Thumbnail, false),
            MAX_THUMBNAIL_BYTES,
            "a real thumbnail response keeps the tighter thumbnail bound"
        );
    }

    #[tokio::test]
    async fn video_poster_404_never_fetches_full_video_body() {
        let poster_path = "/workspaces/ws/assets/video/content?tier=poster";
        let (base_url, server) = scripted_http_server(vec![(poster_path, 404, Vec::new())]);
        let fetcher = ReqwestAssetFetcher::new(base_url);
        let error = fetcher
            .fetch_tier("ws", "video", MediaEmbedKind::Video, MediaTier::Poster)
            .await
            .expect_err("missing poster remains a typed error");
        assert_eq!(error.kind_str(), "not_found");
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![poster_path.to_owned()],
            "no request may reach the canonical full video content route"
        );
    }

    #[tokio::test]
    async fn album_thumbnail_404_does_not_expand_into_original_body_fanout() {
        let thumb_path = "/workspaces/ws/assets/cell/content?tier=thumb";
        let (base_url, server) = scripted_http_server(vec![(thumb_path, 404, Vec::new())]);
        let fetcher = ReqwestAssetFetcher::new(base_url);
        let error = fetcher
            .fetch_tier("ws", "cell", MediaEmbedKind::Album, MediaTier::Thumbnail)
            .await
            .expect_err("missing album thumbnail remains typed/visible");
        assert_eq!(error.kind_str(), "not_found");
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![thumb_path.to_owned()]
        );
    }

    #[tokio::test]
    async fn derived_tier_server_error_never_falls_back_to_original() {
        let thumb_path = "/workspaces/ws/assets/img/content?tier=thumb";
        let (base_url, server) = scripted_http_server(vec![(thumb_path, 500, Vec::new())]);
        let fetcher = ReqwestAssetFetcher::new(base_url);
        let error = fetcher
            .fetch_tier("ws", "img", MediaEmbedKind::Images, MediaTier::Thumbnail)
            .await
            .expect_err("only a 404 may enter the single-image fallback path");
        assert_eq!(error.kind_str(), "server_error");
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![thumb_path.to_owned()]
        );
    }

    #[tokio::test]
    async fn malformed_and_missing_required_metadata_are_typed_server_errors() {
        let path = "/workspaces/ws/assets/img";
        for body in [
            br#"{"asset_id":"img","workspace_id":"ws","mime": }"#.to_vec(),
            br#"{"asset_id":"img","workspace_id":"ws"}"#.to_vec(),
        ] {
            let (base_url, server) = scripted_http_server(vec![(path, 200, body)]);
            let fetcher = ReqwestAssetFetcher::new(base_url);
            let error = fetcher
                .fetch_metadata("ws", "img")
                .await
                .expect_err("invalid metadata must fail closed");
            assert_eq!(error.kind_str(), "server_error");
            assert_eq!(
                server.join().expect("scripted server completed"),
                vec![path.to_owned()]
            );
        }
    }

    #[tokio::test]
    async fn metadata_identity_workspace_and_mime_bindings_fail_closed() {
        let cases = [
            ("other", "ws", "image/png", "id"),
            ("img", "other-workspace", "image/png", "workspace"),
            ("img", "ws", "   ", "mime"),
        ];
        for (returned_id, returned_workspace, mime, expected_message) in cases {
            let path = "/workspaces/ws/assets/img";
            let body = metadata_body(returned_workspace, returned_id, mime);
            let (base_url, server) = scripted_http_server(vec![(path, 200, body)]);
            let fetcher = ReqwestAssetFetcher::new(base_url.clone());
            let error = resolve_one(MediaEmbedKind::Images, "ws", "img", &base_url, &fetcher)
                .await
                .expect_err("unbound metadata must fail closed");
            assert_eq!(error.kind_str(), "server_error");
            assert!(error.to_string().contains(expected_message));
            assert_eq!(
                server.join().expect("scripted server completed"),
                vec![path.to_owned()]
            );
        }
    }

    #[tokio::test]
    async fn missing_asset_metadata_is_typed_not_found() {
        let path = "/workspaces/ws/assets/missing";
        let (base_url, server) = scripted_http_server(vec![(path, 404, Vec::new())]);
        let fetcher = ReqwestAssetFetcher::new(base_url.clone());
        let error = resolve_one(MediaEmbedKind::Images, "ws", "missing", &base_url, &fetcher)
            .await
            .expect_err("missing asset must remain visible and typed");
        assert_eq!(error.kind_str(), "not_found");
        assert_eq!(
            server.join().expect("scripted server completed"),
            vec![path.to_owned()]
        );
    }

    #[tokio::test]
    async fn oversized_declared_content_length_is_rejected_before_body_read() {
        let declared = MAX_METADATA_BYTES + 1;
        let response =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n")
                .into_bytes();
        let (base_url, server) = raw_http_server(response, Duration::ZERO);
        let fetcher = ReqwestAssetFetcher::new(base_url);
        let error = fetcher
            .fetch_metadata("ws", "oversized")
            .await
            .expect_err("oversized Content-Length must fail before allocation/body read");
        assert_eq!(error.kind_str(), "resource_limit");
        assert!(error.to_string().contains("content-length"));
        assert_eq!(
            server.join().expect("raw server completed"),
            "/workspaces/ws/assets/oversized"
        );
    }

    #[tokio::test]
    async fn oversized_chunked_stream_is_rejected_while_reading() {
        let body = vec![b'x'; MAX_METADATA_BYTES + 1];
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:X}\r\n",
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(&body);
        response.extend_from_slice(b"\r\n0\r\n\r\n");
        let (base_url, server) = raw_http_server(response, Duration::ZERO);
        let fetcher = ReqwestAssetFetcher::new(base_url);
        let error = fetcher
            .fetch_metadata("ws", "streamed")
            .await
            .expect_err("streamed body must be bounded even without Content-Length");
        assert_eq!(error.kind_str(), "resource_limit");
        assert!(error.to_string().contains("streamed body"));
        assert_eq!(
            server.join().expect("raw server completed"),
            "/workspaces/ws/assets/streamed"
        );
    }

    #[tokio::test]
    async fn reqwest_request_deadline_bounds_headers_and_body() {
        let response =
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec();
        let (base_url, server) = raw_http_server(response, Duration::from_millis(150));
        let fetcher = ReqwestAssetFetcher::with_timeouts(
            base_url,
            Duration::from_millis(25),
            Duration::from_millis(40),
        );
        let started = std::time::Instant::now();
        let error = fetcher
            .fetch_metadata("ws", "slow")
            .await
            .expect_err("slow headers must hit the request deadline");
        assert_eq!(error.kind_str(), "timed_out");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "the short test deadline must not inherit the production 15-second timeout"
        );
        assert_eq!(
            server.join().expect("raw server completed"),
            "/workspaces/ws/assets/slow"
        );
    }

    #[tokio::test]
    async fn reqwest_request_deadline_bounds_a_stalled_streamed_body() {
        let headers = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n".to_vec();
        let (base_url, server) =
            raw_http_server_parts(headers, Duration::from_millis(150), b"{}".to_vec());
        let fetcher = ReqwestAssetFetcher::with_timeouts(
            base_url,
            Duration::from_millis(25),
            Duration::from_millis(40),
        );
        let started = std::time::Instant::now();
        let error = fetcher
            .fetch_metadata("ws", "slow-body")
            .await
            .expect_err("a stalled response body must hit the request deadline");
        assert_eq!(error.kind_str(), "timed_out");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            server.join().expect("raw server completed"),
            "/workspaces/ws/assets/slow-body"
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

    struct PendingMetadataFetcher;

    impl AssetMetadataFetcher for PendingMetadataFetcher {
        fn fetch_metadata<'a>(
            &'a self,
            _workspace_id: &'a str,
            _asset_id: &'a str,
        ) -> MetadataFuture<'a> {
            Box::pin(std::future::pending())
        }
    }

    #[tokio::test]
    async fn direct_resolve_one_path_has_a_hard_deadline() {
        let started = std::time::Instant::now();
        let error = resolve_one_with_timeout(
            MediaEmbedKind::Images,
            "ws",
            "hung",
            "http://b",
            &PendingMetadataFetcher,
            Duration::from_millis(5),
        )
        .await
        .expect_err("a direct resolver call may not wait forever");
        assert_eq!(error.kind_str(), "timed_out");
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn direct_sequence_budget_wait_has_a_hard_deadline() {
        let fetcher: Arc<dyn AssetMetadataFetcher> = Arc::new(PendingMetadataFetcher);
        let started = std::time::Instant::now();
        let error = resolve_sequence_with_budget_and_timeout(
            MediaEmbedKind::Album,
            "ws",
            "one",
            "http://b",
            fetcher,
            Arc::new(tokio::sync::Semaphore::new(0)),
            Duration::from_millis(5),
        )
        .await
        .expect_err("a direct sequence semaphore wait may not outlive the operation budget");
        assert_eq!(error.kind_str(), "timed_out");
        assert!(started.elapsed() < Duration::from_secs(1));
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
            "http://127.0.0.1:37501/workspaces/ws1/assets/a1/content?tier=thumb"
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
    async fn resolve_one_does_not_reject_large_original_before_bounded_tier_fetch() {
        struct OversizedFetcher;
        impl AssetMetadataFetcher for OversizedFetcher {
            fn fetch_metadata<'a>(
                &'a self,
                workspace_id: &'a str,
                asset_id: &'a str,
            ) -> MetadataFuture<'a> {
                Box::pin(async move {
                    Ok(EmbedAssetMetadata {
                        asset_id: asset_id.to_owned(),
                        workspace_id: workspace_id.to_owned(),
                        kind: "image".to_owned(),
                        mime: "image/png".to_owned(),
                        original_filename: None,
                        content_hash: String::new(),
                        size_bytes: (MAX_FULL_IMAGE_BYTES + 1) as u64,
                        width: Some(1),
                        height: Some(1),
                    })
                })
            }
        }

        let resolved = resolve_one(
            MediaEmbedKind::Images,
            "ws",
            "too-large",
            "http://b",
            &OversizedFetcher,
        )
        .await
        .expect("metadata size must not block a separately bounded thumbnail fetch");
        assert_eq!(resolved.asset.size_bytes, (MAX_FULL_IMAGE_BYTES + 1) as u64);
        assert!(resolved.thumbnail_url.ends_with("/content?tier=thumb"));
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
    async fn backend_collection_owns_sequence_membership_and_order() {
        struct CollectionFetcher {
            metadata_calls: std::sync::Mutex<Vec<String>>,
        }
        impl AssetMetadataFetcher for CollectionFetcher {
            fn fetch_metadata<'a>(
                &'a self,
                workspace_id: &'a str,
                asset_id: &'a str,
            ) -> MetadataFuture<'a> {
                Box::pin(async move {
                    self.metadata_calls
                        .lock()
                        .unwrap()
                        .push(asset_id.to_owned());
                    Ok(EmbedAssetMetadata {
                        asset_id: asset_id.to_owned(),
                        workspace_id: workspace_id.to_owned(),
                        kind: "image".to_owned(),
                        mime: "image/png".to_owned(),
                        original_filename: None,
                        content_hash: String::new(),
                        size_bytes: 16,
                        width: Some(1),
                        height: Some(1),
                    })
                })
            }

            fn fetch_collection<'a>(
                &'a self,
                _workspace_id: &'a str,
                collection_id: &'a str,
            ) -> CollectionFuture<'a> {
                Box::pin(async move {
                    Ok(EmbedCollection {
                        collection_id: collection_id.to_owned(),
                        title: Some("Backend order".to_owned()),
                        members: vec!["third".to_owned(), "first".to_owned(), "second".to_owned()],
                    })
                })
            }
        }

        let fetcher = Arc::new(CollectionFetcher {
            metadata_calls: std::sync::Mutex::new(Vec::new()),
        });
        let fetcher_dyn: Arc<dyn AssetMetadataFetcher> = fetcher.clone();
        let items = resolve_sequence(
            MediaEmbedKind::Album,
            "ws",
            "collection:ordered-set",
            "http://b",
            fetcher_dyn,
        )
        .await
        .unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item.ref_value.as_str())
                .collect::<Vec<_>>(),
            vec!["third", "first", "second"]
        );
        let mut calls = fetcher.metadata_calls.lock().unwrap().clone();
        calls.sort();
        assert_eq!(calls, vec!["first", "second", "third"]);
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
                preview_url: "p".into(),
                poster_url: "v".into(),
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
