//! Backend HTTP client. Reuses the EXISTING handshake_core backend over its real HTTP API
//! (GET /health, GET/PUT /workspaces/:id/workbench/layout) — the native app never starts or embeds
//! the backend; it assumes it is running. Deserializes via serde_json::Value to avoid a build
//! dependency on the handshake_core crate.

use crate::error::AppError;
use crate::layout_persistence::{LayoutError, LayoutTransport};
use serde_json::Value;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// handshake_core listens here (hardcoded in handshake_core/src/main.rs).
pub const BACKEND_BASE_URL: &str = "http://127.0.0.1:37501";

/// The process-wide shared backend [`reqwest::Client`]. `reqwest::Client` owns a connection pool and is
/// cheaply cloneable (an `Arc` internally), so the whole native app should share ONE pool rather than
/// minting an independent pool/TLS stack per sub-client. New `/knowledge/documents/*` transport (the
/// MT-037 consolidated client + the MT-029 find/replace `RichDocClient`, which now delegates to it)
/// resolves its client from here so there is exactly ONE document-transport pool. Lazily initialized on
/// first use; the build is infallible (`reqwest::Client::new()` with the crate's default rustls config).
static SHARED_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Return a clone of the process-wide shared backend [`reqwest::Client`] (one connection pool for the
/// whole app). Cloning is cheap (the pool is shared behind an `Arc`); callers that need to own a client
/// should clone this rather than calling `reqwest::Client::new()` so they share the single pool.
pub fn shared_http_client() -> reqwest::Client {
    SHARED_HTTP_CLIENT
        .get_or_init(reqwest::Client::new)
        .clone()
}
/// Health probe (CONTROL-2). Kept as a full URL for the existing MT-002 health wiring.
pub const HEALTH_URL: &str = "http://127.0.0.1:37501/health";

/// Per-request timeout for the layout endpoint. A save must not hang the UI thread; on timeout the
/// transport returns a TRANSIENT [`LayoutError::Transport`] the persistence manager retries.
const LAYOUT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct HealthInfo {
    pub status: String,
    pub db_status: String,
    pub migration_version: Option<i64>,
}

/// GET /health with a 5s timeout (CONTROL-2). Non-success status or a parse failure is an error,
/// never a panic.
pub async fn fetch_health(url: &str) -> Result<HealthInfo, AppError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("non-success status {}", resp.status())));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    Ok(HealthInfo {
        status: v.get("status").and_then(|x| x.as_str()).unwrap_or("unknown").to_string(),
        db_status: v.get("db_status").and_then(|x| x.as_str()).unwrap_or("unknown").to_string(),
        migration_version: v.get("migration_version").and_then(|x| x.as_i64()),
    })
}

/// REST client for the backend's PostgreSQL-authoritative workbench-layout surface
/// (`GET`/`PUT /workspaces/:workspace_id/workbench/layout`, migration `0323_workbench_layout_state`).
///
/// This is the REAL [`LayoutTransport`] the app wires into its [`LayoutPersistenceManager`]: the
/// native layout persists THROUGH this REST endpoint into PostgreSQL/EventLedger — there is no local
/// file authority (CX-503S / Data Posture). The endpoint stores the snapshot as an opaque JSONB
/// `layout_state` blob, so this client speaks `serde_json::Value` directly and never depends on the
/// `handshake_core` crate's types.
///
/// ## Why a blocking transport over an async client
///
/// reqwest is async, but [`LayoutTransport`] is synchronous so the persistence manager stays a pure,
/// directly-unit-testable state machine. This client holds a tokio runtime [`Handle`] and bridges by
/// `Handle::block_on`. The app calls the transport from a short-lived tokio worker (NOT the egui UI
/// thread — see `HandshakeApp`'s save wiring), so the UI thread is never blocked on the network
/// (HBR-QUIET: background work must not stall the operator).
///
/// [`Handle`]: tokio::runtime::Handle
#[derive(Clone)]
pub struct WorkbenchLayoutClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl WorkbenchLayoutClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn layout_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/workbench/layout", self.base_url, workspace_id)
    }
}

/// Which single flag a [`LoomBlockClient::set_flag`] call PATCHes (MT-021 AC#73). Exactly one flag is
/// sent per request, mapping to the verified flattened `LoomBlockUpdate` field, so a typo can never
/// reach the wrong backend field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoomBlockFlag {
    /// `{ "pinned": <bool> }`.
    Pinned,
    /// `{ "favorite": <bool> }`.
    Favorite,
}

/// The HTTP method a [`RequestSpec`] carries. Kept as a tiny typed enum (not a `reqwest::Method`) so a
/// unit test can assert the method without depending on reqwest internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Post,
    Patch,
    Delete,
    Get,
    /// MT-023: the daily-journal Agenda fetch is a get-or-create PUT (`open_daily_journal`).
    Put,
}

/// The fully-resolved `(method, url, body)` a client method is about to send. Returned by the pure
/// `*_request` builders so a unit test asserts the EXACT verified URL + JSON body (MT-021 MAJOR #1/#2/#3
/// proof) without a live backend. The real spawn paths route through these SAME builders, so the test
/// proves the production request construction, not a parallel reimplementation. `body` is `None` for a
/// bodyless request (DELETE / GET) and `query` carries GET query params.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSpec {
    pub method: HttpMethod,
    pub url: String,
    pub body: Option<serde_json::Value>,
}

/// A `(method, url, query)` spec for a GET request (diff/blame), where the params live in the query
/// string rather than a JSON body. Separate from [`RequestSpec`] so the query is asserted as typed
/// pairs (order-stable) instead of being smuggled into the URL string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetRequestSpec {
    pub method: HttpMethod,
    pub url: String,
    pub query: Vec<(String, String)>,
}

/// One-slot delivery cell for an off-thread Loom-block rename result (MT-020 explorer-row rename).
/// The spawned tokio task writes the PATCH outcome here; the egui UI thread drains it next frame
/// (the same `Arc<Mutex<Option<Result<..>>>>` pattern the settings save/load cells use). `Ok(title)`
/// carries the renamed block's new title (the externally-meaningful result), `Err(msg)` the failure.
pub type RenameDeliveryCell = Arc<Mutex<Option<Result<String, String>>>>;

/// REST client for the Loom-block surface this shell mutates today: the rename PATCH on the VERIFIED
/// backend endpoint `PATCH /workspaces/:workspace_id/loom/blocks/:block_id` (handler
/// `handshake_core::api::loom::patch_loom_block`, body `LoomBlockPatchRequest` whose flattened
/// `LoomBlockUpdate.title` is the rename field). The body sent is `{ "title": "<new title>" }`.
///
/// ## Off-thread (HBR-QUIET)
///
/// The egui UI thread must never block on the network, so [`rename_block`](Self::rename_block) spawns
/// the PATCH on the app's tokio runtime and delivers the result into a [`RenameDeliveryCell`] the UI
/// drains next frame — the MT-009 off-thread + delivery-cell pattern (the same shape
/// `WorkbenchLayoutClient` + the settings cells use). It speaks `serde_json::Value` so it never depends
/// on the `handshake_core` crate's types.
#[derive(Clone)]
pub struct LoomBlockClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomBlockClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!("{}/workspaces/{}/loom/blocks/{}", self.base_url, workspace_id, block_id)
    }

    /// PATCH a single Loom-block FLAG (`pinned` or `favorite`) off the UI thread, delivering the result
    /// into `cell` (MT-021 AC#73). The body is `{ "pinned": <bool> }` or `{ "favorite": <bool> }` —
    /// exactly ONE flag per request, flattened into the verified `LoomBlockUpdate` (the same PATCH
    /// endpoint `rename_block` uses). `Ok(())` on a 2xx; `Err(msg)` on failure. This is what the
    /// `loom.pin` / `loom.favorite` menu actions invoke, so the toggled flag actually persists.
    pub fn set_flag(
        &self,
        workspace_id: &str,
        block_id: &str,
        flag: LoomBlockFlag,
        value: bool,
        cell: ScmReceiptCell,
    ) {
        let spec = self.set_flag_request(workspace_id, block_id, flag, value);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = patch_expect_success(&client, &spec.url, &body).await;
            let delivered = result.map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(delivered);
            }
        });
    }

    /// Pure request builder for [`set_flag`](Self::set_flag): the `(PATCH, url, body)` it sends. Split
    /// out so a unit test asserts the EXACT verified URL + single-flag JSON body without a live backend
    /// (the spawn path above routes through this same builder, so the test proves the production path).
    pub fn set_flag_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        flag: LoomBlockFlag,
        value: bool,
    ) -> RequestSpec {
        let body = match flag {
            LoomBlockFlag::Pinned => serde_json::json!({ "pinned": value }),
            LoomBlockFlag::Favorite => serde_json::json!({ "favorite": value }),
        };
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(body),
        }
    }

    /// Pure request builder for [`rename_block`](Self::rename_block): the `(PATCH, url, body)` it sends.
    pub fn rename_request(&self, workspace_id: &str, block_id: &str, new_title: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(serde_json::json!({ "title": new_title })),
        }
    }

    /// PATCH the block's title off the UI thread, delivering the result into `cell`. The egui UI thread
    /// returns immediately; the spawned task writes `Ok(new_title)` / `Err(msg)` into `cell` and the UI
    /// drains it next frame. The repaint is requested by the caller's normal frame loop (the cell is
    /// drained at the top of `update`).
    pub fn rename_block(
        &self,
        workspace_id: &str,
        block_id: &str,
        new_title: &str,
        cell: RenameDeliveryCell,
    ) {
        let url = self.block_url(workspace_id, block_id);
        let client = self.client.clone();
        let new_title = new_title.to_owned();
        self.runtime.spawn(async move {
            let result = patch_block_title(&client, &url, &new_title).await;
            let delivered = match result {
                Ok(title) => Ok(title),
                Err(e) => Err(e.to_string()),
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(delivered);
            }
        });
    }
}

/// Send `PATCH {url}` with body `{ "title": <new_title> }` and return the renamed block's title from
/// the response (`LoomBlock.title`), falling back to the sent title if the response omits it. A
/// non-success status or a parse failure is an [`AppError`], never a panic.
async fn patch_block_title(
    client: &reqwest::Client,
    url: &str,
    new_title: &str,
) -> Result<String, AppError> {
    let body = serde_json::json!({ "title": new_title });
    let resp = client
        .patch(url)
        .timeout(Duration::from_secs(5))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("PATCH block non-success status {}", resp.status())));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or(new_title)
        .to_owned();
    Ok(title)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// MT-021 (C5 part 2) off-thread clients for the source-control + canvas surfaces.
//
// Every endpoint here was VERIFIED READ-ONLY against `src/backend/handshake_core` (the real running
// backend), NOT assumed from the contract body (whose URLs were partly stale):
//   - source-control routes are mounted at `/source-control/{status,diff,stage,unstage,discard,blame}`
//     with NO `/api` prefix (handshake_core::api::source_control::routes_with_event_recorder). The base
//     URL has no `/api` either (BACKEND_BASE_URL + "/source-control/..."), matching the existing health
//     + workbench-layout clients. `stage`/`unstage` POST `{repo_path, paths}` (PathsRequest); `discard`
//     POSTs `{repo_path, paths, confirmed}` (DiscardRequest — the field is `confirmed`, NOT the
//     contract's `force`, and `confirmed:false` returns HTTP 409 with NO mutation); `diff` is a GET with
//     query `repo_path`,`path`,`scope` (scope ∈ {worktree,staged}); `blame` is a GET with
//     `repo_path`,`path`.
//   - canvas placement routes are `PATCH`/`DELETE /workspaces/:ws/loom/canvas-placements/:placement_id`
//     (handshake_core::api::loom — NOT the contract's `.../loom/canvas/{cb}/placements/{p}`). The
//     placement body supports a real `z_index` field (migration 0334 `loom_canvas_placements.z_index`),
//     so canvas bring-to-front / send-to-back PERSIST (not local-only).
//   - canvas visual-edge delete is `DELETE /workspaces/:ws/loom/canvas-visual-edges/:visual_edge_id`
//     (visual-only edges; semantic Loom edges are a different surface and are NEVER touched here — the
//     red-team `remove_edges` control).
//
// All follow the MT-020 `LoomBlockClient` shape: spawn the request on the app's tokio runtime and
// deliver the outcome into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame, so
// the render thread is NEVER blocked on the network (HBR-QUIET).

/// One-slot delivery cell for an off-thread source-control receipt result. `Ok(())` on a successful
/// stage/unstage/discard (the receipt body is not needed by the menu — the panel re-fetches status),
/// `Err(msg)` on failure (surfaced on the panel status row).
pub type ScmReceiptCell = Arc<Mutex<Option<Result<(), String>>>>;

/// One-slot delivery cell for an off-thread source-control diff/blame text result. `Ok(text)` carries
/// the patch (diff) or rendered blame the panel displays; `Err(msg)` the failure.
pub type ScmTextCell = Arc<Mutex<Option<Result<String, String>>>>;

/// One-slot delivery cell for an off-thread canvas placement mutation result. `Ok(())` on a successful
/// placement update/remove (the canvas re-fetches its board), `Err(msg)` the failure.
pub type CanvasOpCell = Arc<Mutex<Option<Result<(), String>>>>;

/// REST client for the VERIFIED Handshake-native source-control surface (MT-253 backend). Drives the
/// stage/unstage/discard write ops and the diff/blame read ops the MT-021 source-control change-row
/// context menu dispatches. Speaks `serde_json::Value` so it never depends on the `handshake_core`
/// crate's types.
#[derive(Clone)]
pub struct SourceControlClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl SourceControlClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    /// `POST /source-control/{stage|unstage}` with `{repo_path, paths:[path]}`, off the UI thread.
    /// `op` is `"stage"` or `"unstage"` — the SAME path segment the verified backend route uses.
    pub fn stage_paths(
        &self,
        op: ScmWriteOp,
        repo_path: &str,
        path: &str,
        cell: ScmReceiptCell,
    ) {
        let spec = self.stage_request(op, repo_path, path);
        self.spawn_receipt(spec.url, spec.body.unwrap_or_default(), cell);
    }

    /// Pure request builder for [`stage_paths`](Self::stage_paths).
    pub fn stage_request(&self, op: ScmWriteOp, repo_path: &str, path: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/source-control/{}", self.base_url, op.path_segment()),
            body: Some(serde_json::json!({ "repo_path": repo_path, "paths": [path] })),
        }
    }

    /// `POST /source-control/discard` with `{repo_path, paths:[path], confirmed}`, off the UI thread.
    /// `confirmed` MUST be true to mutate: the verified backend returns HTTP 409 (no mutation) when
    /// `confirmed:false`. The MT-021 menu item is a STUB in V1 (no confirm dialog yet), so the panel
    /// passes `confirmed:false` until a real confirm dialog exists — making an accidental dispatch a
    /// safe 409 no-op, never a destructive discard (red-team discard control).
    pub fn discard_paths(
        &self,
        repo_path: &str,
        path: &str,
        confirmed: bool,
        cell: ScmReceiptCell,
    ) {
        let spec = self.discard_request(repo_path, path, confirmed);
        self.spawn_receipt(spec.url, spec.body.unwrap_or_default(), cell);
    }

    /// Pure request builder for [`discard_paths`](Self::discard_paths).
    pub fn discard_request(&self, repo_path: &str, path: &str, confirmed: bool) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/source-control/discard", self.base_url),
            body: Some(serde_json::json!({
                "repo_path": repo_path,
                "paths": [path],
                "confirmed": confirmed
            })),
        }
    }

    /// `GET /source-control/diff?repo_path&path&scope`, off the UI thread, delivering the patch text.
    pub fn diff(&self, repo_path: &str, path: &str, scope: ScmDiffScope, cell: ScmTextCell) {
        let spec = self.diff_request(repo_path, path, scope);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = get_json_field(&client, &spec.url, &spec.query, "patch").await;
            deliver_text(&cell, result);
        });
    }

    /// Pure request builder for [`diff`](Self::diff).
    pub fn diff_request(&self, repo_path: &str, path: &str, scope: ScmDiffScope) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/source-control/diff", self.base_url),
            query: vec![
                ("repo_path".to_owned(), repo_path.to_owned()),
                ("path".to_owned(), path.to_owned()),
                ("scope".to_owned(), scope.query_value().to_owned()),
            ],
        }
    }

    /// `GET /source-control/blame?repo_path&path`, off the UI thread, delivering a rendered blame text
    /// (each line `"{short_commit}  {content}"`) for the V1 monospace blame display.
    pub fn blame(&self, repo_path: &str, path: &str, cell: ScmTextCell) {
        let spec = self.blame_request(repo_path, path);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_blame_text(&client, &spec.url, &spec.query).await;
            deliver_text(&cell, result);
        });
    }

    /// Pure request builder for [`blame`](Self::blame).
    pub fn blame_request(&self, repo_path: &str, path: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/source-control/blame", self.base_url),
            query: vec![
                ("repo_path".to_owned(), repo_path.to_owned()),
                ("path".to_owned(), path.to_owned()),
            ],
        }
    }

    /// Shared spawn for a write op (stage/unstage/discard): POST the body, deliver `Ok(())`/`Err(msg)`.
    fn spawn_receipt(&self, url: String, body: serde_json::Value, cell: ScmReceiptCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &url, &body).await;
            let delivered = result.map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(delivered);
            }
        });
    }
}

/// Which source-control write op a [`SourceControlClient::stage_paths`] call performs. The variant maps
/// to the verified backend route's path segment, so a typo can never reach a wrong endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScmWriteOp {
    Stage,
    Unstage,
}

impl ScmWriteOp {
    fn path_segment(self) -> &'static str {
        match self {
            ScmWriteOp::Stage => "stage",
            ScmWriteOp::Unstage => "unstage",
        }
    }
}

/// The diff scope a [`SourceControlClient::diff`] call requests — the verified backend `DiffScope`
/// enum's two lowercase query values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScmDiffScope {
    Worktree,
    Staged,
}

impl ScmDiffScope {
    fn query_value(self) -> &'static str {
        match self {
            ScmDiffScope::Worktree => "worktree",
            ScmDiffScope::Staged => "staged",
        }
    }
}

/// REST client for the VERIFIED canvas placement + visual-edge surface (MT-261 backend). Drives the
/// canvas-node context menu's `move_to_front`/`move_to_back` (PATCH `z_index`), `remove` (DELETE
/// placement), and `remove_edges` (DELETE visual edges) off the UI thread.
#[derive(Clone)]
pub struct CanvasClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl CanvasClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn placement_url(&self, workspace_id: &str, placement_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/canvas-placements/{}",
            self.base_url, workspace_id, placement_id
        )
    }

    fn visual_edge_url(&self, workspace_id: &str, visual_edge_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/canvas-visual-edges/{}",
            self.base_url, workspace_id, visual_edge_id
        )
    }

    /// `PATCH /workspaces/:ws/loom/canvas-placements/:placement_id` with `{z_index}`, off the UI thread.
    /// The verified backend persists `z_index`, so bring-to-front / send-to-back survives a reload (the
    /// red-team z-order-persistence concern is resolved by the real backend field, not a local list).
    pub fn set_z_index(
        &self,
        workspace_id: &str,
        placement_id: &str,
        z_index: i32,
        cell: CanvasOpCell,
    ) {
        let spec = self.set_z_index_request(workspace_id, placement_id, z_index);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = patch_expect_success(&client, &spec.url, &body).await;
            deliver_op(&cell, result);
        });
    }

    /// Pure request builder for [`set_z_index`](Self::set_z_index).
    pub fn set_z_index_request(
        &self,
        workspace_id: &str,
        placement_id: &str,
        z_index: i32,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(serde_json::json!({ "z_index": z_index })),
        }
    }

    /// `DELETE /workspaces/:ws/loom/canvas-placements/:placement_id`, off the UI thread. Removes the
    /// placement (the canvas reference), NOT the underlying LoomBlock (the contract's "Remove from
    /// Canvas, NOT the block").
    pub fn remove_placement(&self, workspace_id: &str, placement_id: &str, cell: CanvasOpCell) {
        let spec = self.remove_placement_request(workspace_id, placement_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = delete_expect_success(&client, &spec.url).await;
            deliver_op(&cell, result);
        });
    }

    /// Pure request builder for [`remove_placement`](Self::remove_placement).
    pub fn remove_placement_request(&self, workspace_id: &str, placement_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.placement_url(workspace_id, placement_id),
            body: None,
        }
    }

    /// `DELETE /workspaces/:ws/loom/canvas-visual-edges/:visual_edge_id`, off the UI thread. ONLY a
    /// VISUAL-only edge is ever passed here — the canvas-node menu's `remove_edges` enumerates the
    /// board's `visual_edges` and never touches a semantic Loom edge (red-team `remove_edges` control).
    pub fn remove_visual_edge(&self, workspace_id: &str, visual_edge_id: &str, cell: CanvasOpCell) {
        let spec = self.remove_visual_edge_request(workspace_id, visual_edge_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = delete_expect_success(&client, &spec.url).await;
            deliver_op(&cell, result);
        });
    }

    /// Pure request builder for [`remove_visual_edge`](Self::remove_visual_edge).
    pub fn remove_visual_edge_request(
        &self,
        workspace_id: &str,
        visual_edge_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.visual_edge_url(workspace_id, visual_edge_id),
            body: None,
        }
    }
}

/// Write the receipt result into a [`ScmReceiptCell`]/[`CanvasOpCell`]-shaped cell.
fn deliver_op(cell: &CanvasOpCell, result: Result<(), AppError>) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(result.map_err(|e| e.to_string()));
    }
}

/// Write a text result into a [`ScmTextCell`].
fn deliver_text(cell: &ScmTextCell, result: Result<String, AppError>) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(result.map_err(|e| e.to_string()));
    }
}

/// POST `body` and treat any 2xx as success (the receipt body is not needed by the menu). A
/// non-success status (e.g. discard's 409 when not confirmed) is an [`AppError`], never a panic.
async fn post_expect_success(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), AppError> {
    let resp = client
        .post(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("POST non-success status {}", resp.status())));
    }
    Ok(())
}

/// PATCH `body` and treat any 2xx as success.
async fn patch_expect_success(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), AppError> {
    let resp = client
        .patch(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("PATCH non-success status {}", resp.status())));
    }
    Ok(())
}

/// DELETE `url` and treat any 2xx as success.
async fn delete_expect_success(client: &reqwest::Client, url: &str) -> Result<(), AppError> {
    let resp = client
        .delete(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("DELETE non-success status {}", resp.status())));
    }
    Ok(())
}

/// GET `url?query` and return the string at top-level JSON `field` (e.g. the diff's `patch`).
async fn get_json_field(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
    field: &str,
) -> Result<String, AppError> {
    let v = get_json(client, url, query).await?;
    let text = v
        .get(field)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_owned();
    Ok(text)
}

/// GET `url?query` and render the verified `SourceControlBlame.lines[]` (`{commit_id, content}`) into a
/// monospace `"{short_commit}  {content}"` block for the V1 blame display.
async fn fetch_blame_text(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<String, AppError> {
    let v = get_json(client, url, query).await?;
    let mut out = String::new();
    if let Some(lines) = v.get("lines").and_then(|x| x.as_array()) {
        for line in lines {
            let commit = line.get("commit_id").and_then(|x| x.as_str()).unwrap_or("");
            let short = commit.chars().take(8).collect::<String>();
            let content = line.get("content").and_then(|x| x.as_str()).unwrap_or("");
            out.push_str(&format!("{short}  {content}\n"));
        }
    }
    Ok(out)
}

/// GET `url?query` and parse the JSON body. A non-success status or parse failure is an [`AppError`].
async fn get_json(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<serde_json::Value, AppError> {
    let resp = client
        .get(url)
        .query(query)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("GET non-success status {}", resp.status())));
    }
    resp.json().await.map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-023 (C6) off-thread client for the bottom-drawer stash-shelf card data.
//
// Every endpoint here was VERIFIED READ-ONLY against `src/backend/handshake_core` (the real running
// backend), NOT taken from the MT-023 contract body (whose `binds_backend_api` was STALE, the MT-022
// lesson):
//   - The contract named `GET /workspaces/:ws/loom/views/table?content_type=list` and
//     `GET /workspaces/:ws/loom/views/calendar` returning `{ "blocks": [...], "total": N }`. NONE of
//     that exists. `parse_view_type` (handshake_core::api::loom) accepts ONLY
//     {all,unlinked,sorted,pins,favorites} — `table`/`calendar` return HTTP 400 HSK-400-LOOM-VIEW-TYPE.
//     `LoomViewResponse` is `#[serde(tag="view_type")]` with NO `total` field; the count is
//     `blocks.len()`. And `content_type=list` is invalid — `LoomBlockContentType` has no `list`
//     variant (valid: note,file,annotated_file,tag_hub,journal,canvas,view_def → HSK-400 otherwise).
//   - The REAL countable surface is `GET /workspaces/:ws/loom/views/all?content_type=<ct>` (handler
//     `query_loom_view`, `LoomViewQuery.content_type`), response `{ "view_type":"all","blocks":[...] }`.
//     Notes card → content_type=note (exists); the contract's "Lists" maps to the saved
//     block-collection views, whose real content_type is `view_def` (MT-262 BlockCollectionViews).
//   - Agenda has no calendar view to read; the contract's own `ports_from_react` directs the daily
//     journal as the data source: `PUT /workspaces/:ws/loom/journals/:date` (handler
//     `open_daily_journal`, returns a single `LoomBlock`). Badge = 1 if today's journal block has a
//     title/content, else 0; subtitle = its title.
//
// All follow the MT-020/021 off-thread shape: spawn on the app's tokio runtime, deliver the outcome
// into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET — the render
// thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends on the
// `handshake_core` crate's types.

/// The four drawer card kinds whose badge data this client fetches. Mail makes NO backend call (the
/// contract: no mail backend exists yet), so it has no variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerDataKind {
    /// Today's daily journal (`PUT /loom/journals/{today}`): badge = has-content, subtitle = title.
    Agenda,
    /// Saved block-collection views (`GET /loom/views/all?content_type=view_def`): badge = count.
    Lists,
    /// Note blocks (`GET /loom/views/all?content_type=note`): badge = count.
    Notes,
}

impl DrawerDataKind {
    /// The verified `content_type` query value for the count fetch. `Agenda` uses the journal endpoint
    /// (not a view count), so it has no content_type and returns `None`.
    pub fn content_type(self) -> Option<&'static str> {
        match self {
            DrawerDataKind::Agenda => None,
            // The contract's "Lists" = saved block-collection views; their real content_type is
            // `view_def` (no `list` content_type exists — disclosed MT-023 deviation).
            DrawerDataKind::Lists => Some("view_def"),
            DrawerDataKind::Notes => Some("note"),
        }
    }
}

/// The externally-meaningful result of one drawer card fetch: the badge count plus a one-line subtitle.
/// `Ok` carries the live data; `Err(msg)` a failure the card surfaces without crashing the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawerCardData {
    /// Badge count (CONTROL-023-D: a missing/empty result defaults to 0, never an error).
    pub badge_count: u32,
    /// One-line subtitle (e.g. the journal title, or a "N items" summary).
    pub subtitle: String,
}

/// One-slot delivery cell for an off-thread drawer-card fetch result, keyed by which card it is. The
/// spawned task writes `(kind, Ok(data))` / `(kind, Err(msg))`; the egui UI thread drains it next frame
/// and folds it into the matching card (same `Arc<Mutex<Option<..>>>` pattern as the SCM/rename cells).
pub type DrawerDataCell = Arc<Mutex<Option<(DrawerDataKind, Result<DrawerCardData, String>)>>>;

/// REST client for the VERIFIED Loom view-count + daily-journal surfaces the MT-023 bottom drawer reads.
#[derive(Clone)]
pub struct DrawerDataClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl DrawerDataClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn views_all_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/views/all", self.base_url, workspace_id)
    }

    fn journal_url(&self, workspace_id: &str, journal_date: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/journals/{}",
            self.base_url, workspace_id, journal_date
        )
    }

    /// Pure request builder for a view-count fetch: `GET /loom/views/all?content_type=<ct>`. Split out so
    /// a unit test asserts the EXACT verified URL + query without a live backend (the spawn path routes
    /// through this same builder). `kind` must be `Lists` or `Notes` (Agenda has no content_type).
    pub fn count_request(&self, workspace_id: &str, kind: DrawerDataKind) -> GetRequestSpec {
        let content_type = kind
            .content_type()
            .expect("count_request requires a content_type kind (Lists/Notes), not Agenda");
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.views_all_url(workspace_id),
            query: vec![("content_type".to_owned(), content_type.to_owned())],
        }
    }

    /// Pure request builder for the Agenda fetch: `PUT /loom/journals/{today}` (the journal endpoint is a
    /// PUT — `open_daily_journal` get-or-creates today's journal block). No body.
    pub fn journal_request(&self, workspace_id: &str, journal_date: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: self.journal_url(workspace_id, journal_date),
            body: None,
        }
    }

    /// Fetch the Lists/Notes badge count off the UI thread, delivering `(kind, Ok/Err)` into `cell`. The
    /// count is `blocks.len()` from the verified `{ "view_type":"all","blocks":[...] }` response
    /// (CONTROL-023-D: an absent/empty `blocks` array yields 0, never an error).
    pub fn fetch_count(&self, workspace_id: &str, kind: DrawerDataKind, cell: DrawerDataCell) {
        let spec = self.count_request(workspace_id, kind);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_view_count(&client, &spec.url, &spec.query).await;
            deliver_drawer(&cell, kind, result.map_err(|e| e.to_string()));
        });
    }

    /// Fetch today's Agenda data off the UI thread, delivering `(Agenda, Ok/Err)` into `cell`. Badge = 1
    /// if today's journal block has a non-empty title, else 0; subtitle = the title (or "No agenda today").
    pub fn fetch_agenda(&self, workspace_id: &str, journal_date: &str, cell: DrawerDataCell) {
        let spec = self.journal_request(workspace_id, journal_date);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_daily_journal(&client, &spec.url).await;
            deliver_drawer(&cell, DrawerDataKind::Agenda, result.map_err(|e| e.to_string()));
        });
    }
}

/// Write a drawer fetch result into a [`DrawerDataCell`].
fn deliver_drawer(cell: &DrawerDataCell, kind: DrawerDataKind, result: Result<DrawerCardData, String>) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some((kind, result));
    }
}

/// `GET {url}?{query}` and count the `blocks` array length from the verified `LoomViewResponse::All`
/// shape `{ "view_type":"all", "blocks":[...] }`. A missing/null `blocks` field counts as 0
/// (CONTROL-023-D — never an error). A non-success status or parse failure is an [`AppError`].
async fn fetch_view_count(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<DrawerCardData, AppError> {
    let v = get_json(client, url, query).await?;
    let count = v
        .get("blocks")
        .and_then(|b| b.as_array())
        .map(|a| a.len())
        .unwrap_or(0) as u32;
    let subtitle = if count == 1 {
        "1 item".to_owned()
    } else {
        format!("{count} items")
    };
    Ok(DrawerCardData { badge_count: count, subtitle })
}

/// `PUT {url}` (no body) and read today's daily-journal block (the verified `open_daily_journal`
/// response is a single `LoomBlock`). Badge = 1 if the block has a non-empty `title`, else 0; subtitle
/// is the title (or a "No agenda today" fallback). A non-success status or parse failure is an
/// [`AppError`], never a panic.
async fn fetch_daily_journal(client: &reqwest::Client, url: &str) -> Result<DrawerCardData, AppError> {
    let resp = client
        .put(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("PUT journal non-success status {}", resp.status())));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty());
    match title {
        Some(t) => Ok(DrawerCardData { badge_count: 1, subtitle: t.to_owned() }),
        None => Ok(DrawerCardData { badge_count: 0, subtitle: "No agenda today".to_owned() }),
    }
}

impl LayoutTransport for WorkbenchLayoutClient {
    /// `GET /workspaces/:id/workbench/layout`. The backend's `WorkbenchLayoutResponse` carries
    /// `layout_state: Option<Value>` — `null`/absent means no layout stored yet (first run -> `Ok(None)`).
    /// A non-success status or a transport error is a TRANSIENT [`LayoutError::Transport`].
    fn load(&self, workspace_id: &str) -> Result<Option<Value>, LayoutError> {
        let url = self.layout_url(workspace_id);
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .get(&url)
                .timeout(LAYOUT_REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(LayoutError::Transport(format!(
                    "GET layout non-success status {}",
                    resp.status()
                )));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            // WorkbenchLayoutResponse.layout_state is Option<Value>; null/absent => first run.
            match body.get("layout_state") {
                Some(Value::Null) | None => Ok(None),
                Some(v) => Ok(Some(v.clone())),
            }
        })
    }

    /// `PUT /workspaces/:id/workbench/layout` with `SaveWorkbenchLayoutRequest { layout_state }`.
    /// A non-success status or a transport error is a TRANSIENT [`LayoutError::Transport`] the
    /// manager retries; the in-memory layout is unaffected by a save failure.
    fn save(&self, workspace_id: &str, layout_state: Value) -> Result<(), LayoutError> {
        let url = self.layout_url(workspace_id);
        let client = self.client.clone();
        let request_body = serde_json::json!({ "layout_state": layout_state });
        self.runtime.block_on(async move {
            let resp = client
                .put(&url)
                .timeout(LAYOUT_REQUEST_TIMEOUT)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| LayoutError::Transport(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(LayoutError::Transport(format!(
                    "PUT layout non-success status {}",
                    resp.status()
                )));
            }
            Ok(())
        })
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-024 (C6) off-thread client for the bottom-drawer CARD ACTION backend mutations.
//
// The MT-023 drawer renders four TYPE cards (Agenda/Mail/Lists/Notes). MT-024 wires the typed action
// menu the cards expose. The PERSISTING actions route through THIS client; every endpoint + body was
// VERIFIED READ-ONLY against `src/backend/handshake_core` (the real running backend), NOT taken from
// the MT-024 contract body (whose `binds_backend_api` was partly STALE — the MT-022/MT-023 lesson):
//
//   - PIN: `PUT /workspaces/:ws/loom/blocks/:block_id/pin-order` (handler `set_loom_block_pin_order`,
//     MT-183). VERIFIED body field is `{ "pin_order": <i32|null> }` (struct `SetPinOrderRequest`) —
//     NOT the contract's `{ "ordinal": 0 }`. To bring a card to the top we send `pin_order: 0`.
//   - DISCARD: `DELETE /workspaces/:ws/loom/blocks/:block_id` (handler `delete_loom_block`). Bodyless.
//   - STOW: the contract proposed `PATCH ... { metadata: { stash_state } }` OR `{ content_type }`.
//     NEITHER is patchable: the VERIFIED `LoomBlockUpdate` (storage/loom.rs) has ONLY
//     {title,pinned,favorite,journal_date,pin_order} — there is NO `metadata`, `stash_state`, or
//     `content_type` field on the PATCH. The contract's OWN implementation_note names the fallback:
//     the tag-edge approach. So STOW = `POST /workspaces/:ws/loom/edges` with
//     `{ source_block_id, target_block_id:<stash hub>, edge_type:"tag", created_by:"user",
//        target_title:"stash" }` (VERIFIED `CreateLoomEdgeRequest`; `ensure_edge_target_exists`
//     get-or-creates the `stash` TagHub on first use, so no separate hub-creation call is needed).
//   - ATTACH-EVIDENCE: `POST /diagnostics` (handler `create_diagnostic`, body `DiagnosticInput`).
//     VERIFIED enum values: `source` ∈ {lsp,terminal,validator,engine,connector,system,plugin:*,
//     matcher:*} and `surface` ∈ {monaco,canvas,sheet,terminal,connector,system} — the contract's
//     `source:"user"` + `surface:"drawer"` do NOT exist and would HTTP-400. We send the honest closest
//     valid values `source:"system"`, `surface:"system"`, severity `"info"`, and carry the stashed
//     block id in `evidence_refs.artifact_hashes` (the VERIFIED `EvidenceRefs.artifact_hashes` field).
//   - CONVERT-TO-ARTIFACT: there is NO backend surface to change a block's content_type (no PATCH
//     field, no dedicated endpoint). It therefore CANNOT be wired honestly and remains a disabled V1
//     menu item (same treatment as the existing MT-021 `convert_artifact` stub) — disclosed deviation.
//
// All follow the MT-020/021/023 off-thread shape: spawn on the app's tokio runtime, deliver the
// outcome into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET —
// the render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends on
// the `handshake_core` crate's types.

/// One-slot delivery cell for an off-thread drawer-card action result. `Ok(())` on a 2xx (the card
/// removes/refreshes optimistically AFTER the backend confirms), `Err(msg)` on failure (the card stays
/// and the drawer surfaces the error). Same shape as the SCM/canvas receipt cells.
pub type DrawerActionCell = Arc<Mutex<Option<Result<(), String>>>>;

/// The well-known title of the per-workspace "stash" TagHub a Stow action tags blocks into. The
/// backend `ensure_edge_target_exists` get-or-creates a TagHub with this title on first tag-edge
/// creation, so Stow never needs a separate hub-creation round-trip.
pub const STASH_TAG_TITLE: &str = "stash";

/// The deterministic block id of the per-workspace stash TagHub. A stable, content-addressable id
/// (`stash` hub is a singleton per workspace) so repeated Stows tag the SAME hub and a swarm reader can
/// address it. `ensure_edge_target_exists` creates it with `content_type=tag_hub` + `target_title` on
/// the first Stow if absent.
pub const STASH_TAG_HUB_BLOCK_ID: &str = "tag-hub-stash";

/// REST client for the VERIFIED Loom-block + diagnostics surfaces the MT-024 drawer card ACTIONS
/// mutate: pin-order (Pin), block delete (Discard), tag-edge (Stow), and diagnostic create
/// (Attach-evidence). Mirrors the `LoomBlockClient`/`CanvasClient` shape exactly.
#[derive(Clone)]
pub struct DrawerActionClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl DrawerActionClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn pin_order_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}/pin-order",
            self.base_url, workspace_id, block_id
        )
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!("{}/workspaces/{}/loom/blocks/{}", self.base_url, workspace_id, block_id)
    }

    fn edges_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/edges", self.base_url, workspace_id)
    }

    fn diagnostics_url(&self) -> String {
        format!("{}/diagnostics", self.base_url)
    }

    // ── Pin ─────────────────────────────────────────────────────────────────────────────────────────

    /// Pure request builder for [`pin_to_top`](Self::pin_to_top): `PUT /loom/blocks/:id/pin-order` with
    /// `{ "pin_order": <ordinal> }`. The field is `pin_order` (VERIFIED `SetPinOrderRequest`), NOT the
    /// contract's `ordinal`. Bring-to-top sends ordinal 0.
    pub fn pin_order_request(&self, workspace_id: &str, block_id: &str, ordinal: i32) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: self.pin_order_url(workspace_id, block_id),
            body: Some(serde_json::json!({ "pin_order": ordinal })),
        }
    }

    /// Bring a card's block to the top of the Pins grid (ordinal 0) off the UI thread, delivering the
    /// result into `cell`.
    pub fn pin_to_top(&self, workspace_id: &str, block_id: &str, cell: DrawerActionCell) {
        let spec = self.pin_order_request(workspace_id, block_id, 0);
        let body = spec.body.unwrap_or_default();
        self.spawn_put_receipt(spec.url, body, cell);
    }

    // ── Discard ─────────────────────────────────────────────────────────────────────────────────────

    /// Pure request builder for [`discard`](Self::discard): `DELETE /loom/blocks/:id`, bodyless.
    pub fn discard_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.block_url(workspace_id, block_id),
            body: None,
        }
    }

    /// DELETE the card's block off the UI thread, delivering the result into `cell`. DESTRUCTIVE: the
    /// caller MUST only invoke this after the confirm-discard guard is `true` (HBR-STOP / RISK-024-A).
    pub fn discard(&self, workspace_id: &str, block_id: &str, cell: DrawerActionCell) {
        let spec = self.discard_request(workspace_id, block_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = delete_expect_success(&client, &spec.url).await;
            deliver_drawer_action(&cell, result);
        });
    }

    // ── Stow (tag-edge to the stash TagHub) ───────────────────────────────────────────────────────────

    /// Pure request builder for [`stow`](Self::stow): `POST /loom/edges` with a VERIFIED
    /// `CreateLoomEdgeRequest` that tags the card's block into the per-workspace `stash` TagHub. The
    /// `target_title` lets `ensure_edge_target_exists` get-or-create the hub on first use.
    pub fn stow_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.edges_url(workspace_id),
            body: Some(serde_json::json!({
                "source_block_id": block_id,
                "target_block_id": STASH_TAG_HUB_BLOCK_ID,
                "edge_type": "tag",
                "created_by": "user",
                "target_title": STASH_TAG_TITLE,
            })),
        }
    }

    /// Tag the card's block into the `stash` TagHub off the UI thread, delivering the result into `cell`.
    pub fn stow(&self, workspace_id: &str, block_id: &str, cell: DrawerActionCell) {
        let spec = self.stow_request(workspace_id, block_id);
        let body = spec.body.unwrap_or_default();
        self.spawn_post_receipt(spec.url, body, cell);
    }

    // ── Attach evidence (diagnostic create) ──────────────────────────────────────────────────────────

    /// Pure request builder for [`attach_evidence`](Self::attach_evidence): `POST /diagnostics` with a
    /// VERIFIED `DiagnosticInput`. `source:"system"` + `surface:"system"` are the honest closest valid
    /// enum values (the contract's `user`/`drawer` do not exist); the stashed block id is carried in
    /// `evidence_refs.artifact_hashes`. `job_id` is the active job when present (AC-024-9).
    pub fn attach_evidence_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        block_title: &str,
        job_id: Option<&str>,
    ) -> RequestSpec {
        let mut body = serde_json::json!({
            "title": format!("Evidence: {block_title}"),
            "message": "Attached from drawer stash shelf",
            "severity": "info",
            "source": "system",
            "surface": "system",
            "wsid": workspace_id,
            "evidence_refs": { "artifact_hashes": { block_id: block_id } },
        });
        if let Some(job_id) = job_id {
            body["job_id"] = serde_json::Value::String(job_id.to_owned());
        }
        RequestSpec {
            method: HttpMethod::Post,
            url: self.diagnostics_url(),
            body: Some(body),
        }
    }

    /// Record the card's block as an evidence diagnostic off the UI thread, delivering the result into
    /// `cell`. The caller is responsible for the AC-024-9 "no active job" pre-check (it shows a tooltip
    /// and makes NO call when there is no active job).
    pub fn attach_evidence(
        &self,
        workspace_id: &str,
        block_id: &str,
        block_title: &str,
        job_id: Option<&str>,
        cell: DrawerActionCell,
    ) {
        let spec = self.attach_evidence_request(workspace_id, block_id, block_title, job_id);
        let body = spec.body.unwrap_or_default();
        self.spawn_post_receipt(spec.url, body, cell);
    }

    // ── Shared spawns ────────────────────────────────────────────────────────────────────────────────

    fn spawn_post_receipt(&self, url: String, body: serde_json::Value, cell: DrawerActionCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &url, &body).await;
            deliver_drawer_action(&cell, result);
        });
    }

    fn spawn_put_receipt(&self, url: String, body: serde_json::Value, cell: DrawerActionCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = put_expect_success(&client, &url, &body).await;
            deliver_drawer_action(&cell, result);
        });
    }
}

/// Write a drawer-action receipt result into a [`DrawerActionCell`].
fn deliver_drawer_action(cell: &DrawerActionCell, result: Result<(), AppError>) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(result.map_err(|e| e.to_string()));
    }
}

/// PUT `body` and treat any 2xx as success (the pin-order receipt body is not needed by the card).
async fn put_expect_success(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<(), AppError> {
    let resp = client
        .put(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("PUT non-success status {}", resp.status())));
    }
    Ok(())
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-021 Loom-graph transport (REUSE the existing reqwest/timeout/error shape).
//
// The native Loom GRAPH VIEW (`graph::graph_view::LoomGraphView`) binds the EXISTING handshake_core
// Loom read APIs through THIS client (NOT Tauri — the contract's "Tauri command" reference is the
// legacy React/webview stack; this is the native egui app, so it uses the same HTTP client every other
// MT-008/014/015 surface uses). Two modes, both VERIFIED READ-ONLY against `src/backend/handshake_core`:
//   - GLOBAL: `GET /workspaces/:ws/loom/views/all` -> `LoomViewResponse::All { blocks }` (the SAME
//     endpoint DrawerDataClient counts). Each block becomes a graph node; the global view returns NO
//     edges (the flat enumeration has no edge payload), so the global graph is node-only until a node
//     is focused. content_type drives the node colour.
//   - LOCAL: `GET /workspaces/:ws/loom/graph-search?q={title}&backlink_depth=2&limit=200` ->
//     `Vec<LoomGraphSearchResult>` (handler `search_loom_graph`). VERIFIED: the backend REJECTS an empty
//     `q` with HTTP 400 HSK-400-LOOM-QUERY-REQUIRED, so Local mode MUST pass the focused block's title
//     as `q` (the contract's "empty q to enumerate" is stale for graph-search; global enumeration is
//     `views/all`, not `graph-search?q=`). Each result with a `block` becomes a node; an edge is
//     synthesized from the focused block to every neighbour (the neighbourhood star), since the
//     graph-search result list is the focused block's neighbourhood, not an edge list.
//
// Follows the MT-020/021/023 off-thread shape: spawn on the app's tokio runtime, deliver the parsed
// graph into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET — the
// render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends on the
// `handshake_core` crate's types; the parsed node/edge shapes are the widget's own
// `graph::graph_view::{GraphNode, GraphEdge}` (the field-correct reuse of the verified backend shapes).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::graph_view::{GraphEdge, GraphNode};

/// The externally-meaningful result of a Loom-graph fetch: the node + edge lists the
/// [`crate::graph::graph_view::LoomGraphView`] renders. `Ok` carries the live graph; `Err(msg)` a
/// failure the view surfaces as an error label (AC8) instead of crashing.
pub type LoomGraphCell = Arc<Mutex<Option<Result<LoomGraphData, String>>>>;

/// A parsed Loom graph (nodes + edges) plus the focus block id the fetch was for (so a stale delivery
/// for a previous mode/block can be detected by the host).
#[derive(Debug, Clone, PartialEq)]
pub struct LoomGraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// REST client for the VERIFIED Loom graph read surfaces the MT-021 graph view binds: `views/all`
/// (global) and `graph-search` (local neighbourhood). Mirrors the `DrawerDataClient` shape exactly.
#[derive(Clone)]
pub struct LoomGraphClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomGraphClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn views_all_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/views/all", self.base_url, workspace_id)
    }

    fn graph_search_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/graph-search", self.base_url, workspace_id)
    }

    /// Pure request builder for the GLOBAL graph fetch: `GET /loom/views/all` (no query). Split out so a
    /// unit test asserts the EXACT verified URL without a live backend (the spawn path routes through
    /// this same builder).
    pub fn global_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.views_all_url(workspace_id),
            query: vec![],
        }
    }

    /// Pure request builder for the LOCAL neighbourhood fetch: `GET /loom/graph-search?q={title}&
    /// backlink_depth=2&limit=200`. `q` is the focused block's TITLE (the backend rejects an empty `q`).
    pub fn local_request(&self, workspace_id: &str, title: &str) -> GetRequestSpec {
        self.local_request_with_depth(workspace_id, title, DEFAULT_BACKLINK_DEPTH)
    }

    /// WP-KERNEL-012 MT-080 (E11 host-mount, AC-080-3 / MT-060 deep wiring): the DEPTH-parameterized
    /// variant of [`local_request`](Self::local_request). The graph view's MT-060 link-depth slider fires
    /// `GraphEvent::DepthChanged { depth }`; the host re-fires the EXISTING `graph-search` endpoint with the
    /// new `backlink_depth` (NO new endpoint — only the verified query parameter changes). `depth` is
    /// clamped to `[MIN..=MAX]_BACKLINK_DEPTH` so a slider/agent value can never send an out-of-range or
    /// abusive depth to the backend. `local_request` delegates here with the default depth, so the two stay
    /// one builder (no second URL surface to drift).
    pub fn local_request_with_depth(
        &self,
        workspace_id: &str,
        title: &str,
        depth: u32,
    ) -> GetRequestSpec {
        let depth = depth.clamp(MIN_BACKLINK_DEPTH, MAX_BACKLINK_DEPTH);
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.graph_search_url(workspace_id),
            query: vec![
                ("q".to_owned(), title.to_owned()),
                ("backlink_depth".to_owned(), depth.to_string()),
                ("limit".to_owned(), "200".to_owned()),
            ],
        }
    }

    /// Fetch the GLOBAL graph (all blocks) off the UI thread, delivering the parsed graph into `cell`.
    pub fn fetch_global(&self, workspace_id: &str, cell: LoomGraphCell) {
        let spec = self.global_request(workspace_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_global_graph(&client, &spec.url).await;
            deliver_graph(&cell, result.map_err(|e| e.to_string()));
        });
    }

    /// Fetch the LOCAL neighbourhood of the focused block off the UI thread, delivering the parsed graph
    /// into `cell`. `focus_block_id` is the block whose neighbourhood is shown (the star centre);
    /// `focus_title` is the graph-search `q`.
    pub fn fetch_local(
        &self,
        workspace_id: &str,
        focus_block_id: &str,
        focus_title: &str,
        cell: LoomGraphCell,
    ) {
        self.fetch_local_with_depth(
            workspace_id,
            focus_block_id,
            focus_title,
            DEFAULT_BACKLINK_DEPTH,
            cell,
        );
    }

    /// WP-KERNEL-012 MT-080 (AC-080-3 / MT-060): fetch the LOCAL neighbourhood at a specific
    /// `backlink_depth`, the re-query the host fires on `GraphEvent::DepthChanged`. Same off-thread spawn +
    /// parse path as [`fetch_local`](Self::fetch_local); only the query `backlink_depth` differs (the
    /// EXISTING endpoint, NO new route). `fetch_local` delegates here with the default depth.
    pub fn fetch_local_with_depth(
        &self,
        workspace_id: &str,
        focus_block_id: &str,
        focus_title: &str,
        depth: u32,
        cell: LoomGraphCell,
    ) {
        let spec = self.local_request_with_depth(workspace_id, focus_title, depth);
        let client = self.client.clone();
        let focus = focus_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_local_graph(&client, &spec.url, &spec.query, &focus).await;
            deliver_graph(&cell, result.map_err(|e| e.to_string()));
        });
    }
}

/// WP-KERNEL-012 MT-080 (MT-060 link-depth): the default `backlink_depth` the local neighbourhood fetch
/// uses (the value `local_request` carried before the depth parameter was threaded — unchanged behavior
/// for the non-depth path).
pub const DEFAULT_BACKLINK_DEPTH: u32 = 2;
/// The minimum `backlink_depth` the depth-parameterized graph re-query will send. A depth of 1 is the
/// focused block plus its direct neighbours (the shallowest useful local view).
pub const MIN_BACKLINK_DEPTH: u32 = 1;
/// The maximum `backlink_depth` the depth-parameterized graph re-query will send. Clamps a slider/agent
/// value so an out-of-range depth can never reach the backend as an abusive traversal (RISK-080-3 — the
/// re-query stays inside the verified endpoint's safe envelope).
pub const MAX_BACKLINK_DEPTH: u32 = 5;

/// Write a graph fetch result into a [`LoomGraphCell`].
fn deliver_graph(cell: &LoomGraphCell, result: Result<LoomGraphData, String>) {
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(result);
    }
}

/// Parse one verified `LoomBlock` JSON object into a [`GraphNode`]. `title` falls back to the block id
/// when null/empty so a node is never label-less. `content_type` defaults to "other" (slate) when
/// absent. Returns `None` only when the block has no `block_id` (a malformed row is skipped, not faked).
fn block_to_node(block: &serde_json::Value) -> Option<GraphNode> {
    let block_id = block.get("block_id").and_then(|x| x.as_str())?.to_owned();
    let title = block
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&block_id)
        .to_owned();
    let content_type = block
        .get("content_type")
        .and_then(|x| x.as_str())
        .unwrap_or("other")
        .to_owned();
    Some(GraphNode::new(block_id, title, content_type))
}

/// `GET {url}` and parse the verified `LoomViewResponse::All { blocks }` into a node-only graph (the
/// global enumeration carries no edge payload). A missing/empty `blocks` array yields an EMPTY graph
/// (0 nodes), never an error (AC7). A non-success status or parse failure is an [`AppError`] (AC8).
async fn fetch_global_graph(client: &reqwest::Client, url: &str) -> Result<LoomGraphData, AppError> {
    let v = get_json(client, url, &[]).await?;
    let nodes = v
        .get("blocks")
        .and_then(|b| b.as_array())
        .map(|arr| arr.iter().filter_map(block_to_node).collect())
        .unwrap_or_default();
    Ok(LoomGraphData { nodes, edges: vec![] })
}

/// `GET {url}?{query}` and parse the verified `Vec<LoomGraphSearchResult>` into the focused block's
/// neighbourhood graph: every result that carries a `block` becomes a node, and a star edge links the
/// focused block to each neighbour. The focused block itself is added as a node if the backend did not
/// return it among the results (so the centre is always present). A non-success status or parse failure
/// is an [`AppError`] (AC8) — including the backend's HTTP 400 for an empty `q`.
async fn fetch_local_graph(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
    focus_block_id: &str,
) -> Result<LoomGraphData, AppError> {
    let v = get_json(client, url, query).await?;
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(arr) = v.as_array() {
        for result in arr {
            // A graph-search result references a block via the optional `block` object; the flat
            // `ref_id`/`title` carry the addressing when `block` is absent.
            if let Some(block) = result.get("block") {
                if let Some(node) = block_to_node(block) {
                    if seen.insert(node.block_id.clone()) {
                        nodes.push(node);
                    }
                    continue;
                }
            }
            // Fall back to ref_id + title when no embedded block object.
            if let Some(ref_id) = result.get("ref_id").and_then(|x| x.as_str()) {
                let title = result
                    .get("title")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(ref_id)
                    .to_owned();
                if seen.insert(ref_id.to_owned()) {
                    nodes.push(GraphNode::new(ref_id.to_owned(), title, "other"));
                }
            }
        }
    }
    // Ensure the focus block is present as the star centre.
    if !seen.contains(focus_block_id) {
        nodes.insert(0, GraphNode::new(focus_block_id.to_owned(), focus_block_id.to_owned(), "note"));
        seen.insert(focus_block_id.to_owned());
    }
    // Star edges from the focus to each neighbour (the neighbourhood is the focus's local graph).
    let edges = nodes
        .iter()
        .filter(|n| n.block_id != focus_block_id)
        .map(|n| GraphEdge::new(focus_block_id.to_owned(), n.block_id.clone(), "mention"))
        .collect();
    Ok(LoomGraphData { nodes, edges })
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-026 Loom CANVAS-BOARD transport (E3 — the Obsidian-Canvas-class surface).
//
// VERIFIED READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the running
// backend), NOT taken from the MT-026 contract body — whose `binds_backend_api` URLs were STALE
// (the contract named `.../loom/canvas/{cb}/...`; the REAL routes are `.../loom/canvas-boards/...`
// + the placement/visual-edge routes under `.../loom/canvas-placements` / `.../loom/canvas-visual-edges`,
// matching the existing CanvasClient that already verified the placement DELETE/PATCH shape). The
// MT-022/023/024 "verify, don't trust the contract" lesson. The nine routes this client binds:
//   - GET    /workspaces/:ws/loom/canvas-boards/:block_id              get_canvas_board -> LoomCanvasBoardView
//                                                                       { board{board_state{pan_x,pan_y,zoom}},
//                                                                         placements[], visual_edges[] }
//   - PUT    /workspaces/:ws/loom/canvas-boards/:block_id/viewport     update_canvas_board_state
//                                                                       body { board_state:{schema_id,pan_x,pan_y,zoom} }
//   - POST   /workspaces/:ws/loom/canvas-boards/:block_id/placements   place_block_on_canvas
//                                                                       body { placed_block_id,x,y,w,h }
//   - POST   /workspaces/:ws/loom/canvas-boards/:block_id/cards        create_canvas_card
//                                                                       body { title,body,x,y,w,h }
//   - PATCH  /workspaces/:ws/loom/canvas-placements/:placement_id      update_canvas_placement
//                                                                       body { group_id } (NOT `.../canvas/{cb}/placements/{p}`)
//   - DELETE /workspaces/:ws/loom/canvas-placements/:placement_id      remove_canvas_placement (source block kept)
//   - POST   /workspaces/:ws/loom/edges                                create_loom_edge
//                                                                       body { source_block_id,target_block_id,
//                                                                              edge_type:"mention",created_by:"user" }
//   - POST   /workspaces/:ws/loom/canvas-boards/:block_id/visual-edges add_canvas_visual_edge
//                                                                       body { from_placement_id,to_placement_id }
//   - GET    /workspaces/:ws/loom/blocks/:block_id                     get_loom_block -> LoomBlock (live title resolve)
//
// Placement x/y/w/h are `f64` on the wire (the storage struct), so the request builders take f64.
// All follow the MT-020/021 off-thread shape: spawn on the app's tokio runtime, deliver the outcome
// into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET). Speaks
// `serde_json::Value` so it never depends on the `handshake_core` crate's types; the parsed board
// shape is the widget's own `graph::canvas_board::{CanvasPlacementCard, VisualEdge}`.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::canvas_board::{CanvasPlacementCard, VisualEdge};

/// The parsed canvas board: placements + visual edges + viewport (pan/zoom), plus the live-title resolve
/// map keyed by `placed_block_id` (filled by a follow-up `getLoomBlock` per distinct block). `Ok` carries
/// the projection; `Err(msg)` a failure the board surfaces as an error label instead of crashing.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasBoardData {
    pub placements: Vec<CanvasPlacementCard>,
    pub visual_edges: Vec<VisualEdge>,
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

/// One-slot delivery cell for an off-thread `getCanvasBoard` fetch result.
pub type CanvasBoardCell = Arc<Mutex<Option<Result<CanvasBoardData, String>>>>;

/// One-slot delivery cell for an off-thread canvas MUTATION (place/card/viewport/group/remove/edge)
/// result. `Ok(())` on a 2xx (the board re-fetches), `Err(msg)` the failure. Same shape as
/// [`CanvasOpCell`].
pub type CanvasBoardOpCell = Arc<Mutex<Option<Result<(), String>>>>;

/// The resolved fields a `getLoomBlock` live-resolve carries: `(title, content_type, content_hash)`.
/// `title` is `Option<String>` (a block can be untitled); `content_hash` is `Option<String>`, the
/// backend-computed canonical-JSON SHA-256 the block carries (WP-KERNEL-012 MT-032 — READ-only, the
/// canvas never writes a hash). `None` content_hash means the backend block omitted it (honestly
/// absent, never fabricated).
pub type LiveBlock = (Option<String>, String, Option<String>);

/// One-slot delivery cell for an off-thread `getLoomBlock` live-resolve. Delivers
/// `(placed_block_id, Ok((title, content_type, content_hash)))` / `(placed_block_id, Err(msg))`. A
/// missing block (HTTP 404) is delivered as `Err` so the host shows `(stale reference)` — never a
/// fabricated title.
pub type LiveBlockCell = Arc<Mutex<Option<(String, Result<LiveBlock, String>)>>>;

/// REST client for the VERIFIED Loom canvas-board surface (MT-261 backend). Drives the board read +
/// all canvas mutations off the UI thread. Mirrors the `CanvasClient`/`LoomGraphClient` shape exactly.
#[derive(Clone)]
pub struct CanvasBoardClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl CanvasBoardClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn board_url(&self, workspace_id: &str, canvas_block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/canvas-boards/{}",
            self.base_url, workspace_id, canvas_block_id
        )
    }

    fn placement_url(&self, workspace_id: &str, placement_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/canvas-placements/{}",
            self.base_url, workspace_id, placement_id
        )
    }

    fn edges_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/edges", self.base_url, workspace_id)
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!("{}/workspaces/{}/loom/blocks/{}", self.base_url, workspace_id, block_id)
    }

    /// Pure request builder for `GET .../canvas-boards/:block_id` (getCanvasBoard).
    pub fn get_board_request(&self, workspace_id: &str, canvas_block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.board_url(workspace_id, canvas_block_id),
            query: vec![],
        }
    }

    /// Pure request builder for `PUT .../canvas-boards/:block_id/viewport` (updateCanvasBoardViewport).
    /// The verified body is `{ board_state:{schema_id,pan_x,pan_y,zoom} }` — NOT a top-level
    /// `{pan_x,pan_y,zoom}` (the contract's stale shape).
    pub fn viewport_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        pan_x: f32,
        pan_y: f32,
        zoom: f32,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: format!("{}/viewport", self.board_url(workspace_id, canvas_block_id)),
            body: Some(serde_json::json!({
                "board_state": {
                    "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
                    "pan_x": pan_x,
                    "pan_y": pan_y,
                    "zoom": zoom,
                }
            })),
        }
    }

    /// Pure request builder for `POST .../canvas-boards/:block_id/placements` (placeBlockOnCanvas).
    #[allow(clippy::too_many_arguments)] // x/y/w/h geometry + ids — the verified placement body shape.
    pub fn place_block_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        placed_block_id: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/placements", self.board_url(workspace_id, canvas_block_id)),
            body: Some(serde_json::json!({
                "placed_block_id": placed_block_id,
                "x": x, "y": y, "w": w, "h": h,
            })),
        }
    }

    /// Pure request builder for `POST .../canvas-boards/:block_id/cards` (createCanvasCard).
    #[allow(clippy::too_many_arguments)] // x/y/w/h geometry + title + ids — the verified card body shape.
    pub fn create_card_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        title: &str,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/cards", self.board_url(workspace_id, canvas_block_id)),
            body: Some(serde_json::json!({
                "title": title,
                "body": "",
                "x": x, "y": y, "w": w, "h": h,
            })),
        }
    }

    /// Pure request builder for `PATCH .../canvas-placements/:placement_id` (updateCanvasPlacement)
    /// with a `group_id` (grouping). The verified body uses `group_id`.
    pub fn group_request(
        &self,
        workspace_id: &str,
        placement_id: &str,
        group_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(serde_json::json!({ "group_id": group_id })),
        }
    }

    /// WP-KERNEL-012 MT-080 (AC-080-2 / MT-061): pure request builder for
    /// `PATCH .../canvas-placements/:placement_id` (updateCanvasPlacement) with the new card `{w, h}`. The
    /// canvas `CanvasEvent::ResizePlacement { placement_id, w, h }` fires ONCE on resize drag-stop
    /// (debounced in the widget); the host maps it to this builder, sends it via [`dispatch`](Self::dispatch),
    /// then re-fetches the board so the persisted geometry replaces the optimistic in-flight size. Same
    /// placement URL + PATCH verb as [`group_request`](Self::group_request); only the body fields differ
    /// (`w`/`h` are the verified placement geometry fields — see [`placement_from_json`]).
    pub fn resize_request(
        &self,
        workspace_id: &str,
        placement_id: &str,
        w: f64,
        h: f64,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(serde_json::json!({ "w": w, "h": h })),
        }
    }

    /// WP-KERNEL-012 MT-080 (AC-080-2 / MT-061): pure request builder for
    /// `PATCH .../canvas-placements/:placement_id` clearing the `group_id` (a card dropped OUTSIDE all
    /// section frames). The canvas `CanvasEvent::AssignSection { placement_id, group_id: None }` fires on a
    /// move drag-stop outside any section; the host maps the `None` arm here, the `Some` arm to
    /// [`group_request`](Self::group_request).
    ///
    /// Backend-shape note (verified against `update_canvas_placement` /
    /// `UpdatePlacementRequest` in `src/backend/handshake_core/src/api/loom.rs`): the handler clears the
    /// group ONLY when the separate boolean `clear_group: true` is present
    /// (`let group_id = if payload.clear_group { Some(None) } else { payload.group_id.map(Some) };`). A
    /// `{"group_id": null}` body deserializes to `group_id: None` (serde default) and the storage layer's
    /// `CASE WHEN $8 ...` (with `$8 = update.group_id.is_some() = false`) leaves the group UNCHANGED — i.e.
    /// `{"group_id": null}` is a silent no-op and the card re-snaps into its old section on the next board
    /// refresh. Sending `{"clear_group": true}` is the only body that actually clears the assignment. This
    /// matches the shape the widget already documents at `graph/canvas_board.rs` (`{clear_group:true}`).
    pub fn clear_group_request(&self, workspace_id: &str, placement_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.placement_url(workspace_id, placement_id),
            body: Some(serde_json::json!({ "clear_group": true })),
        }
    }

    /// Pure request builder for `DELETE .../canvas-placements/:placement_id` (removeCanvasPlacement).
    /// Removes the placement REFERENCE; the source block is KEPT (MC-4).
    pub fn remove_placement_request(&self, workspace_id: &str, placement_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Delete,
            url: self.placement_url(workspace_id, placement_id),
            body: None,
        }
    }

    /// Pure request builder for `POST /loom/edges` (createLoomEdge) — a real semantic `mention` edge.
    pub fn semantic_edge_request(
        &self,
        workspace_id: &str,
        source_block_id: &str,
        target_block_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.edges_url(workspace_id),
            body: Some(serde_json::json!({
                "source_block_id": source_block_id,
                "target_block_id": target_block_id,
                "edge_type": "mention",
                "created_by": "user",
            })),
        }
    }

    /// Pure request builder for `POST .../canvas-boards/:block_id/visual-edges` (addCanvasVisualEdge).
    pub fn visual_edge_request(
        &self,
        workspace_id: &str,
        canvas_block_id: &str,
        from_placement_id: &str,
        to_placement_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/visual-edges", self.board_url(workspace_id, canvas_block_id)),
            body: Some(serde_json::json!({
                "from_placement_id": from_placement_id,
                "to_placement_id": to_placement_id,
            })),
        }
    }

    /// Pure request builder for `GET /loom/blocks/:block_id` (getLoomBlock) — live-title resolve.
    pub fn get_block_request(&self, workspace_id: &str, block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.block_url(workspace_id, block_id),
            query: vec![],
        }
    }

    /// Fetch the board off the UI thread, delivering the parsed projection into `cell`.
    pub fn fetch_board(&self, workspace_id: &str, canvas_block_id: &str, cell: CanvasBoardCell) {
        let spec = self.get_board_request(workspace_id, canvas_block_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_canvas_board(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Resolve a block's live title + content_type off the UI thread (`getLoomBlock`), delivering
    /// `(placed_block_id, Ok((title, content_type)))` / `(placed_block_id, Err))` into `cell`.
    pub fn resolve_block(&self, workspace_id: &str, placed_block_id: &str, cell: LiveBlockCell) {
        let spec = self.get_block_request(workspace_id, placed_block_id);
        let client = self.client.clone();
        let id = placed_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_live_block(&client, &spec.url).await.map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((id, result));
            }
        });
    }

    /// Send a prebuilt mutation [`RequestSpec`] (place/card/viewport/group/remove/edge) off the UI
    /// thread, delivering `Ok(())`/`Err(msg)` into `cell`. The host re-fetches the board after a 2xx.
    pub fn dispatch(&self, spec: RequestSpec, cell: CanvasBoardOpCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = send_canvas_mutation(&client, &spec).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }
}

/// The board-state schema id the backend stamps on the canvas viewport JSONB (mirrors
/// `handshake_core::storage::LOOM_CANVAS_BOARD_SCHEMA_ID`). Kept as a const here so the native client
/// never depends on the backend crate.
pub const LOOM_CANVAS_BOARD_SCHEMA_ID: &str = "hsk.loom_canvas_board@1";

/// Send one canvas mutation by method, treating any 2xx as success (the board re-fetches for the body).
async fn send_canvas_mutation(client: &reqwest::Client, spec: &RequestSpec) -> Result<(), AppError> {
    let empty = serde_json::json!({});
    let body = spec.body.as_ref().unwrap_or(&empty);
    match spec.method {
        HttpMethod::Post => post_expect_success(client, &spec.url, body).await,
        HttpMethod::Patch => patch_expect_success(client, &spec.url, body).await,
        HttpMethod::Put => put_expect_success(client, &spec.url, body).await,
        HttpMethod::Delete => delete_expect_success(client, &spec.url).await,
        HttpMethod::Get => Err(AppError::Http("GET is not a mutation".to_owned())),
    }
}

/// `GET {url}` and parse the verified `LoomCanvasBoardView` into a [`CanvasBoardData`]. Placements
/// arrive WITHOUT live titles (the host resolves each via `getLoomBlock` after this returns — reference,
/// not copy). A missing/empty board yields an EMPTY projection (0 placements), never an error (AC10).
async fn fetch_canvas_board(client: &reqwest::Client, url: &str) -> Result<CanvasBoardData, AppError> {
    let v = get_json(client, url, &[]).await?;
    let placements = v
        .get("placements")
        .and_then(|p| p.as_array())
        .map(|arr| arr.iter().filter_map(placement_from_json).collect())
        .unwrap_or_default();
    let visual_edges = v
        .get("visual_edges")
        .and_then(|e| e.as_array())
        .map(|arr| arr.iter().filter_map(visual_edge_from_json).collect())
        .unwrap_or_default();
    let board_state = v.get("board").and_then(|b| b.get("board_state"));
    let pan_x = board_state.and_then(|s| s.get("pan_x")).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let pan_y = board_state.and_then(|s| s.get("pan_y")).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let zoom = board_state.and_then(|s| s.get("zoom")).and_then(|x| x.as_f64()).unwrap_or(1.0) as f32;
    Ok(CanvasBoardData { placements, visual_edges, pan_x, pan_y, zoom })
}

/// Parse one verified `LoomCanvasPlacement` JSON object into a [`CanvasPlacementCard`] (no live title
/// yet). Returns `None` only when `placement_id` or `placed_block_id` is missing (a malformed row is
/// skipped, not faked).
fn placement_from_json(p: &serde_json::Value) -> Option<CanvasPlacementCard> {
    let placement_id = p.get("placement_id").and_then(|x| x.as_str())?.to_owned();
    let placed_block_id = p.get("placed_block_id").and_then(|x| x.as_str())?.to_owned();
    let x = p.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let y = p.get("y").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let w = p.get("w").and_then(|x| x.as_f64()).unwrap_or(200.0) as f32;
    let h = p.get("h").and_then(|x| x.as_f64()).unwrap_or(120.0) as f32;
    let mut card = CanvasPlacementCard::new(placement_id, placed_block_id, x, y, w, h);
    card.z_index = p.get("z_index").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    card.group_id = p.get("group_id").and_then(|x| x.as_str()).map(ToOwned::to_owned);
    Some(card)
}

/// Parse one verified `LoomCanvasVisualEdge` JSON object into a [`VisualEdge`]. Returns `None` when any
/// required id is missing.
fn visual_edge_from_json(e: &serde_json::Value) -> Option<VisualEdge> {
    Some(VisualEdge {
        visual_edge_id: e.get("visual_edge_id").and_then(|x| x.as_str())?.to_owned(),
        from_placement_id: e.get("from_placement_id").and_then(|x| x.as_str())?.to_owned(),
        to_placement_id: e.get("to_placement_id").and_then(|x| x.as_str())?.to_owned(),
    })
}

/// `GET {url}` and read a verified `LoomBlock`'s `(title, content_type, content_hash)` for the
/// live-resolve. `title` is `Option<String>` (a block can be untitled); `content_type` defaults to
/// "note"; `content_hash` is the backend-computed canonical-JSON hash when present (MT-032, READ-only —
/// `Option<String>`, honestly `None` when the backend omits it). A 404 (the block was deleted) is an
/// [`AppError`] so the host shows "(stale reference)" — never a fabricated title.
async fn fetch_live_block(
    client: &reqwest::Client,
    url: &str,
) -> Result<LiveBlock, AppError> {
    let v = get_json(client, url, &[]).await?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned);
    let content_type = v
        .get("content_type")
        .and_then(|x| x.as_str())
        .unwrap_or("note")
        .to_owned();
    let content_hash = v
        .get("content_hash")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToOwned::to_owned);
    Ok((title, content_type, content_hash))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-022 Loom FOLDER-TREE transport (REUSE — extends the MT-021 Loom read surface).
//
// VERIFIED READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the running
// backend), NOT taken from the MT-022 contract body — whose assumed surface (content_type='folder'
// LoomBlocks, color in content_json.metadata.color_label, children via views/sorted?tag_ids=) does NOT
// exist (the MT-022/023 "verify, don't trust the contract" lesson the LoomGraphClient + DrawerDataClient
// already embody). The REAL folder authority is the dedicated `loom_folders` subsystem (MT-181
// FolderTreeAndColorLabels, Master Spec §7.1.4.3), an organizational overlay over LoomBlocks with a
// first-class `color` column. The three routes this client binds (mounted in `loom::routes`):
//   - GET   /workspaces/:ws/loom/folders                    -> Vec<LoomFolder> (the tree rows; the
//     parent/child shape is `parent_folder_id`, so the tree is built CLIENT-side from the flat list).
//   - GET   /workspaces/:ws/loom/folders/:folder_id/blocks  -> Vec<LoomBlock> (the lazy child-block
//     load on expand; supports `limit`/`offset`, default limit 100, capped 500).
//   - PATCH /workspaces/:ws/loom/folders/:folder_id  body { "color": "#rrggbb" } -> LoomFolder (recolor).
//     `LoomFolderUpdate.color` is `Option<Option<String>>` server-side, i.e. a TRUE JSON merge-patch:
//     sending ONLY `color` leaves name/sort/parent untouched (RISK-2/MC-2: no whole-record clobber).
//
// Follows the MT-020/021/023 off-thread shape exactly: spawn on the app's tokio runtime, deliver the
// parsed result into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET
// — the render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends
// on the `handshake_core` crate's types; the parsed shapes are the widget's own
// `graph::folder_tree::{FolderRow, LeafBlock}` (the field-correct reuse of the verified backend shapes).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::folder_tree::{FolderRow, LeafBlock};

/// The externally-meaningful result of a folder-list fetch: the flat [`FolderRow`] list the
/// [`crate::graph::folder_tree::LoomFolderTree`] builds its forest from. `Ok` carries the rows (possibly
/// empty -> the "No folders" empty state, AC7); `Err(msg)` a failure the view surfaces as an error
/// banner + Retry (AC8) instead of crashing.
pub type FolderListCell = Arc<Mutex<Option<Result<Vec<FolderRow>, String>>>>;

/// The externally-meaningful result of a folder-children fetch: the leaf [`LeafBlock`] list for one
/// expanded folder. `Ok` carries the blocks (possibly empty); `Err(msg)` a failure the node surfaces.
/// The host clears the node's `loading` flag when this delivers (the bounded-spinner rule).
pub type FolderChildrenCell = Arc<Mutex<Option<Result<Vec<LeafBlock>, String>>>>;

/// REST client for the VERIFIED Loom folder-tree surface (MT-181 backend) the MT-022 folder tree binds:
/// list folders, list a folder's child blocks, and recolor a folder. Mirrors the `LoomGraphClient` /
/// `DrawerDataClient` shape exactly (off-thread + delivery cell). Speaks `serde_json::Value` so it never
/// depends on the `handshake_core` crate's types.
#[derive(Clone)]
pub struct LoomFolderClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomFolderClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn folders_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/folders", self.base_url, workspace_id)
    }

    fn folder_url(&self, workspace_id: &str, folder_id: &str) -> String {
        format!("{}/workspaces/{}/loom/folders/{}", self.base_url, workspace_id, folder_id)
    }

    fn folder_blocks_url(&self, workspace_id: &str, folder_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/folders/{}/blocks",
            self.base_url, workspace_id, folder_id
        )
    }

    /// Pure request builder for the folder-list fetch: `GET /loom/folders` (no query). Split out so a
    /// unit test asserts the EXACT verified URL without a live backend (the spawn path routes through
    /// this same builder, so the test proves the production request construction).
    pub fn list_folders_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.folders_url(workspace_id),
            query: vec![],
        }
    }

    /// Pure request builder for the child-block fetch: `GET /loom/folders/{id}/blocks?limit=100`.
    pub fn list_folder_blocks_request(&self, workspace_id: &str, folder_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.folder_blocks_url(workspace_id, folder_id),
            query: vec![("limit".to_owned(), "100".to_owned())],
        }
    }

    /// Pure request builder for the recolor PATCH: `PATCH /loom/folders/{id}` body `{ "color": "#hex" }`.
    /// The body carries ONLY the `color` key — a true JSON merge-patch against the verified
    /// `LoomFolderUpdate` (whose `color: Option<Option<String>>` means "set color, leave everything
    /// else"), so a recolor can NEVER clobber the folder's name/sort/parent (RISK-2 / MC-2 / AC4).
    pub fn recolor_request(&self, workspace_id: &str, folder_id: &str, hex: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.folder_url(workspace_id, folder_id),
            body: Some(serde_json::json!({ "color": hex })),
        }
    }

    /// Fetch the workspace's folder list off the UI thread, delivering the parsed rows into `cell` (the
    /// initial AC1 tree load). The host sets `loading=true` before calling and clears it on delivery.
    pub fn fetch_folders(&self, workspace_id: &str, cell: FolderListCell) {
        let spec = self.list_folders_request(workspace_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_folder_rows(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Fetch one folder's child blocks off the UI thread, delivering the parsed leaves into `cell` (the
    /// AC2 lazy child load on expand). The host sets the node's `loading=true` before calling (so the
    /// spinner animates ONLY during this genuine in-flight fetch) and clears it on delivery.
    pub fn fetch_folder_blocks(&self, workspace_id: &str, folder_id: &str, cell: FolderChildrenCell) {
        let spec = self.list_folder_blocks_request(workspace_id, folder_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_folder_leaves(&client, &spec.url, &spec.query).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Recolor a folder off the UI thread (AC4), delivering the outcome into `cell`. The PATCH body is
    /// the single-`color`-key merge-patch from [`recolor_request`](Self::recolor_request). `Ok(())` on a
    /// 2xx; `Err(msg)` on failure. The host updates the node swatch optimistically and reconciles on the
    /// delivered result.
    pub fn recolor_folder(&self, workspace_id: &str, folder_id: &str, hex: &str, cell: ScmReceiptCell) {
        let spec = self.recolor_request(workspace_id, folder_id, hex);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = patch_expect_success(&client, &spec.url, &body).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }
}

/// Parse one verified `LoomFolder` JSON object into a [`FolderRow`]. `name` falls back to the folder id
/// when null/empty. Returns `None` only when the row has no `folder_id` (a malformed row is skipped, not
/// faked).
fn folder_to_row(folder: &serde_json::Value) -> Option<FolderRow> {
    let folder_id = folder.get("folder_id").and_then(|x| x.as_str())?.to_owned();
    let parent_folder_id = folder
        .get("parent_folder_id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_owned());
    let name = folder
        .get("name")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&folder_id)
        .to_owned();
    let color = folder
        .get("color")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_owned());
    Some(FolderRow::new(folder_id, parent_folder_id, name, color))
}

/// Parse one verified `LoomBlock` JSON object into a folder-tree [`LeafBlock`]. Mirrors the graph
/// view's `block_to_node` field reads (`block_id`/`title`/`content_type`) so the two surfaces agree on
/// the verified block shape. `title` falls back to the block id; `content_type` defaults to "other".
fn block_to_leaf(block: &serde_json::Value) -> Option<LeafBlock> {
    let block_id = block.get("block_id").and_then(|x| x.as_str())?.to_owned();
    let title = block
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&block_id)
        .to_owned();
    let content_type = block
        .get("content_type")
        .and_then(|x| x.as_str())
        .unwrap_or("other")
        .to_owned();
    Some(LeafBlock::new(block_id, title, content_type))
}

/// `GET {url}` and parse the verified `Vec<LoomFolder>` into [`FolderRow`]s. A missing/empty array
/// yields an EMPTY list (0 folders -> the "No folders" empty state, AC7), never an error. A non-success
/// status or parse failure is an [`AppError`] (the AC8 error banner).
async fn fetch_folder_rows(client: &reqwest::Client, url: &str) -> Result<Vec<FolderRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    let rows = v
        .as_array()
        .map(|arr| arr.iter().filter_map(folder_to_row).collect())
        .unwrap_or_default();
    Ok(rows)
}

/// `GET {url}?{query}` and parse the verified `Vec<LoomBlock>` into folder-tree [`LeafBlock`]s. An
/// empty array yields an empty leaf list (the folder renders "(empty)"), never an error. A non-success
/// status or parse failure is an [`AppError`].
async fn fetch_folder_leaves(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<LeafBlock>, AppError> {
    let v = get_json(client, url, query).await?;
    let leaves = v
        .as_array()
        .map(|arr| arr.iter().filter_map(block_to_leaf).collect())
        .unwrap_or_default();
    Ok(leaves)
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-023 Loom TAG-HUB transport (REUSE — extends the MT-021/022 Loom read surface).
//
// VERIFIED READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the running
// backend), NOT taken from the MT-023 contract body — whose assumed surface (`views/all?content_type=
// tag_hub` list filter, `views/all?tag_ids={id}` member query, a `content_json` hub description) does
// NOT exist (the MT-022/023 "verify, don't trust the contract" lesson). The REAL tag authority is the
// dedicated tag-hub API (MT-182 "tags as first-class blocks"):
//   - GET  /workspaces/:ws/loom/tags                       -> Vec<LoomBlock> (every `tag_hub` block; the
//     flat list the panel renders. Because this route ALREADY returns only tag hubs, RISK-5's
//     client-side content_type fallback is unnecessary — there is no `content_type` filter to fall back
//     from). Supports `limit`/`offset` (default 100, capped 500).
//   - GET  /workspaces/:ws/loom/tags/:tag_block_id         -> LoomTagHub { block, sub_tags,
//     tagged_blocks, backlink_count } (the hub page: title from block.title, members from tagged_blocks).
//   - GET  /workspaces/:ws/loom/tags/:tag_block_id/blocks  -> Vec<LoomBlock> (members; supports
//     `include_subtags`/`limit`/`offset`). The exact member-count + member-list source.
//   - POST /workspaces/:ws/loom/edges  body { source_block_id, target_block_id, edge_type:"tag",
//     created_by:"user" } -> LoomEdge (tag a block with a hub). The backend HARD-rejects a non-tag_hub
//     target with HSK-400-LOOM-TAG-TARGET-MUST-BE-TAG_HUB, so the hub is ALWAYS the edge TARGET and the
//     tagged block the SOURCE (verified `create_loom_edge`). `created_by` is the verified
//     `LoomEdgeCreatedBy` enum ("user"/"ai"); "user" is the operator-initiated tag.
//
// Follows the MT-020/021/022 off-thread shape exactly: spawn on the app's tokio runtime, deliver the
// parsed result into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET
// — the render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends
// on the handshake_core crate; the parsed shapes are the widget's own graph::tags_panel types.
//
// AC6 / RISK-2 / MC-2 (the no-fixed-sleep correction): `tag_block` spawns the POST and delivers the
// outcome into a `ScmReceiptCell`; the HOST awaits THAT delivery and only THEN re-queries the members
// via `fetch_members`. There is NO 100ms sleep — the re-query is gated on the edge-create RESPONSE.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::tags_panel::{AddTagCandidate, HubMember, TagEntry};

/// The externally-meaningful result of a tag-list fetch: the flat [`TagEntry`] list the
/// [`crate::graph::tags_panel::LoomTagsPanel`] renders. `Ok` carries the entries (possibly empty -> the
/// "No tags" empty state, AC8); `Err(msg)` a failure the panel surfaces as an error banner + Retry.
pub type TagListCell = Arc<Mutex<Option<Result<Vec<TagEntry>, String>>>>;

/// The externally-meaningful result of a hub-detail fetch: `(title, members)` for the hub page. `Ok`
/// carries the resolved title + member list; `Err(msg)` a failure the hub page surfaces.
pub type TagHubDetailCell = Arc<Mutex<Option<Result<(String, Vec<HubMember>), String>>>>;

/// The externally-meaningful result of an add-tag candidate search: the candidate blocks to tag. `Ok`
/// carries the candidates (possibly empty); `Err(msg)` a failure (the popup shows nothing rather than
/// crashing).
pub type AddTagCandidatesCell = Arc<Mutex<Option<Result<Vec<AddTagCandidate>, String>>>>;

/// REST client for the VERIFIED Loom tag-hub surface (MT-182 backend) the MT-023 tags panel binds: list
/// tag hubs, load a hub's detail + members, search for taggable blocks, and create a `tag` edge. Mirrors
/// the `LoomFolderClient` / `LoomGraphClient` shape exactly (off-thread + delivery cell). Speaks
/// `serde_json::Value` so it never depends on the handshake_core crate's types.
#[derive(Clone)]
pub struct LoomTagClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomTagClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn tags_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/tags", self.base_url, workspace_id)
    }

    fn tag_url(&self, workspace_id: &str, tag_block_id: &str) -> String {
        format!("{}/workspaces/{}/loom/tags/{}", self.base_url, workspace_id, tag_block_id)
    }

    fn tag_blocks_url(&self, workspace_id: &str, tag_block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/tags/{}/blocks",
            self.base_url, workspace_id, tag_block_id
        )
    }

    fn edges_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/edges", self.base_url, workspace_id)
    }

    fn search_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/search", self.base_url, workspace_id)
    }

    /// Pure request builder for the tag-list fetch: `GET /loom/tags` (no query — default limit 100).
    /// Split out so a unit test asserts the EXACT verified URL without a live backend (the spawn path
    /// routes through this same builder, so the test proves the production request construction).
    pub fn list_tags_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.tags_url(workspace_id),
            query: vec![],
        }
    }

    /// Pure request builder for the hub-detail fetch: `GET /loom/tags/{id}`.
    pub fn tag_detail_request(&self, workspace_id: &str, tag_block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.tag_url(workspace_id, tag_block_id),
            query: vec![],
        }
    }

    /// Pure request builder for the member-list fetch: `GET /loom/tags/{id}/blocks?limit=100`.
    pub fn list_members_request(&self, workspace_id: &str, tag_block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.tag_blocks_url(workspace_id, tag_block_id),
            query: vec![("limit".to_owned(), "100".to_owned())],
        }
    }

    /// Pure request builder for the add-tag candidate search: `GET /loom/search?q={q}&limit=20`. The
    /// verified workspace search route returns blocks matching `q` (the candidate blocks to tag).
    pub fn search_blocks_request(&self, workspace_id: &str, q: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.search_url(workspace_id),
            query: vec![
                ("q".to_owned(), q.to_owned()),
                ("limit".to_owned(), "20".to_owned()),
            ],
        }
    }

    /// Pure request builder for the tag-edge create: `POST /loom/edges` with the verified
    /// `CreateLoomEdgeRequest` body. The tagged block is the edge SOURCE; the hub is the TARGET (the
    /// backend rejects a non-tag_hub target). `created_by:"user"` is the operator-initiated tag (AC6).
    pub fn tag_block_request(
        &self,
        workspace_id: &str,
        source_block_id: &str,
        hub_block_id: &str,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.edges_url(workspace_id),
            body: Some(serde_json::json!({
                "source_block_id": source_block_id,
                "target_block_id": hub_block_id,
                "edge_type": "tag",
                "created_by": "user",
            })),
        }
    }

    /// Fetch the workspace's tag-hub list off the UI thread, delivering the parsed entries into `cell`
    /// (the initial AC1 load). The host sets `loading=true` before calling and clears it on delivery.
    pub fn fetch_tags(&self, workspace_id: &str, cell: TagListCell) {
        let spec = self.list_tags_request(workspace_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_tag_entries(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Fetch one hub's detail (title + members) off the UI thread, delivering into `cell` (AC4). Parses
    /// the verified `LoomTagHub` shape: title from `block.title`, members from `tagged_blocks`.
    pub fn fetch_hub_detail(&self, workspace_id: &str, tag_block_id: &str, cell: TagHubDetailCell) {
        let spec = self.tag_detail_request(workspace_id, tag_block_id);
        let client = self.client.clone();
        let fallback_id = tag_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_tag_hub_detail(&client, &spec.url, &fallback_id).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Fetch a hub's members off the UI thread, delivering into `cell`. Used for the lazy list
    /// member-count resolution AND the AC6 post-tag member refresh (the host calls this AFTER the
    /// tag-edge POST response resolves — no fixed sleep).
    pub fn fetch_members(&self, workspace_id: &str, tag_block_id: &str, cell: TagHubDetailCell) {
        let spec = self.list_members_request(workspace_id, tag_block_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_tag_members(&client, &spec.url, &spec.query).await;
            if let Ok(mut slot) = cell.lock() {
                // The member-list route carries no hub title; deliver an empty title so the host keeps
                // its existing title and replaces only the members.
                *slot = Some(result.map(|m| (String::new(), m)).map_err(|e| e.to_string()));
            }
        });
    }

    /// Search for candidate blocks to tag off the UI thread, delivering into `cell` (the add-tag popup).
    pub fn search_blocks(&self, workspace_id: &str, q: &str, cell: AddTagCandidatesCell) {
        let spec = self.search_blocks_request(workspace_id, q);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_add_tag_candidates(&client, &spec.url, &spec.query).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Create a `tag` edge (tag `source_block_id` with the hub `hub_block_id`) off the UI thread,
    /// delivering the outcome into `cell` (AC6). The body is the verified `CreateLoomEdgeRequest`. The
    /// HOST awaits this delivery and only THEN re-queries the members (no fixed sleep — RISK-2/MC-2).
    pub fn tag_block(
        &self,
        workspace_id: &str,
        source_block_id: &str,
        hub_block_id: &str,
        cell: ScmReceiptCell,
    ) {
        let spec = self.tag_block_request(workspace_id, source_block_id, hub_block_id);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &spec.url, &body).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }
}

/// Parse one verified `tag_hub` `LoomBlock` JSON object into a [`TagEntry`]. `title` falls back to the
/// block id when null/empty. `member_count` starts as `None` (the list never blocks on per-tag fetches;
/// the host resolves the exact count lazily on hub open). Returns `None` only when the block has no
/// `block_id` (a malformed row is skipped, not faked).
fn block_to_tag_entry(block: &serde_json::Value) -> Option<TagEntry> {
    let block_id = block.get("block_id").and_then(|x| x.as_str())?.to_owned();
    let title = block
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&block_id)
        .to_owned();
    Some(TagEntry::new(block_id, title, None))
}

/// Parse one verified `LoomBlock` JSON object into a hub-page [`HubMember`]. Mirrors the folder-tree
/// `block_to_leaf` field reads so the surfaces agree on the verified block shape.
fn block_to_hub_member(block: &serde_json::Value) -> Option<HubMember> {
    let block_id = block.get("block_id").and_then(|x| x.as_str())?.to_owned();
    let title = block
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&block_id)
        .to_owned();
    let content_type = block
        .get("content_type")
        .and_then(|x| x.as_str())
        .unwrap_or("other")
        .to_owned();
    Some(HubMember::new(block_id, title, content_type))
}

/// `GET {url}` and parse the verified `Vec<LoomBlock>` (tag hubs) into [`TagEntry`]s. An empty array
/// yields an empty list (the "No tags" empty state, AC8), never an error. A non-success status / parse
/// failure is an [`AppError`] (the error banner).
async fn fetch_tag_entries(client: &reqwest::Client, url: &str) -> Result<Vec<TagEntry>, AppError> {
    let v = get_json(client, url, &[]).await?;
    let entries = v
        .as_array()
        .map(|arr| arr.iter().filter_map(block_to_tag_entry).collect())
        .unwrap_or_default();
    Ok(entries)
}

/// `GET {url}` and parse the verified `LoomTagHub` `{ block, tagged_blocks, .. }` into `(title,
/// members)`. The hub title is `block.title` (falling back to the block id); the members are the
/// `tagged_blocks` array. A non-success status / parse failure is an [`AppError`].
async fn fetch_tag_hub_detail(
    client: &reqwest::Client,
    url: &str,
    fallback_id: &str,
) -> Result<(String, Vec<HubMember>), AppError> {
    let v = get_json(client, url, &[]).await?;
    let title = v
        .get("block")
        .and_then(|b| b.get("title"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(fallback_id)
        .to_owned();
    let members = v
        .get("tagged_blocks")
        .and_then(|b| b.as_array())
        .map(|arr| arr.iter().filter_map(block_to_hub_member).collect())
        .unwrap_or_default();
    Ok((title, members))
}

/// `GET {url}?{query}` and parse the verified `Vec<LoomBlock>` (a hub's members) into [`HubMember`]s. An
/// empty array yields an empty member list, never an error.
async fn fetch_tag_members(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<HubMember>, AppError> {
    let v = get_json(client, url, query).await?;
    let members = v
        .as_array()
        .map(|arr| arr.iter().filter_map(block_to_hub_member).collect())
        .unwrap_or_default();
    Ok(members)
}

/// Pure parser: turn a verified `/loom/search` JSON response into add-tag [`AddTagCandidate`]s. Split out
/// (pure over `serde_json::Value`, no I/O) so the VERIFIED response shape is unit-testable without a live
/// backend — the gap that hid the wrong-shape bug (the only candidate-producing widget test injected
/// candidates directly, never exercising this parse).
///
/// VERIFIED shape (`api::loom::search_loom_blocks` -> `Json<Vec<LoomBlockSearchResult>>`, and
/// `storage::loom::LoomBlockSearchResult { block: LoomBlock, score: f64 }` — NO `#[serde(flatten)]`): each
/// array entry is `{ "block": { "block_id", "title", .. }, "score": f64 }`, so `block_id`/`title` live
/// UNDER the `block` key, NOT at the entry's top level. We read the nested `block` object first (the real
/// route), then fall back to a top-level `block_id`/`ref_id`/`title` read for a bare-block array (defensive
/// — other search-shaped routes). The outer collection tolerates a bare array OR an object wrapping a
/// `results`/`blocks`/`hits` array. An empty result yields no candidates, never an error.
fn parse_add_tag_candidates(v: &serde_json::Value) -> Vec<AddTagCandidate> {
    let arr = v
        .as_array()
        .cloned()
        .or_else(|| v.get("results").and_then(|x| x.as_array()).cloned())
        .or_else(|| v.get("blocks").and_then(|x| x.as_array()).cloned())
        .or_else(|| v.get("hits").and_then(|x| x.as_array()).cloned())
        .unwrap_or_default();
    arr.iter()
        .filter_map(|entry| {
            // The verified LoomBlockSearchResult nests the block under `block`; prefer that, then fall
            // back to the entry itself for a bare-block array.
            let block = entry.get("block").unwrap_or(entry);
            let id = block
                .get("block_id")
                .and_then(|x| x.as_str())
                .or_else(|| block.get("ref_id").and_then(|x| x.as_str()))?
                .to_owned();
            let title = block
                .get("title")
                .and_then(|x| x.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(&id)
                .to_owned();
            Some(AddTagCandidate::new(id, title))
        })
        .collect()
}

/// `GET {url}?{query}` against the verified workspace search route and parse the result blocks into
/// add-tag [`AddTagCandidate`]s via [`parse_add_tag_candidates`]. An empty result yields no candidates,
/// never an error.
async fn fetch_add_tag_candidates(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<AddTagCandidate>, AppError> {
    let v = get_json(client, url, query).await?;
    Ok(parse_add_tag_candidates(&v))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-024 Loom SIDEBAR transport (REUSE — extends the MT-021/022/023 Loom read surface).
//
// The native pins/favorites/backlinks/unlinked sidebar (`graph::sidebar_panel::LoomSidebarPanel`) binds
// the EXISTING handshake_core Loom read + mutation APIs through THIS client (NO Tauri — the same HTTP
// client every MT-008/014/021/022/023 surface uses). Every endpoint + body was VERIFIED READ-ONLY
// against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the running backend), NOT taken from
// the MT-024 contract body (the MT-022/023 "verify, don't trust the contract" lesson):
//
//   - PINS:      `GET /workspaces/:ws/loom/views/pins?limit=100`      -> `LoomViewResponse::Pins
//     { blocks }`. `parse_view_type` accepts `pins` (CONFIRMED real this time, unlike the MT-022/023
//     stale view types). The count is `blocks.len()`; there is NO `total` field.
//   - FAVORITES: `GET /workspaces/:ws/loom/views/favorites?limit=100` -> `LoomViewResponse::Favorites
//     { blocks }` (`parse_view_type` accepts `favorites`).
//   - BACKLINKS (contract correction, disclosed): the contract named `graph-search?mention_ids={id}`.
//     That param IS real, but the DEDICATED `GET /workspaces/:ws/loom/blocks/:id/backlinks` ->
//     `Vec<LoomBacklink>` (MT-178 `get_backlinks_with_context`) is the field-correct surface: each
//     backlink carries the incoming `edge` (with `edge_type`) + the `source_block`. That is exactly the
//     AC4 "source block title + edge_type label", so this client binds THAT route (verified), not a
//     synthesized graph-search star.
//   - UNLINKED (contract correction, disclosed): the contract named `GET /loom/views/unlinked` (a
//     WORKSPACE unlinked view). For the *per-active-block* Unlinked section the correct verified surface
//     is the DEDICATED `GET /workspaces/:ws/loom/blocks/:id/unlinked-mentions` ->
//     `Vec<LoomUnlinkedMention>` (MT-178 `scan_loom_block_unlinked_mentions`): blocks whose text mentions
//     the active block's title with NO edge — exactly the AC5 semantics. The workspace `/views/unlinked`
//     is NOT scoped to the active block, so it would be the wrong list.
//   - REMOVE PIN (two-call, RISK-1 / MC-1): `PUT /workspaces/:ws/loom/blocks/:id/pin-order` body
//     `{ "pin_order": null }` (`SetPinOrderRequest`, MT-183 — the field is `pin_order`, NOT the
//     contract's `ordinal`) THEN `PATCH /workspaces/:ws/loom/blocks/:id` body `{ "pinned": false }`
//     (`LoomBlockUpdate`, MT-022 confirmed `pinned` is an `Option<bool>` PATCH field). Both are issued in
//     sequence (the React WorkspaceSidebar.tsx lines 297-298 flow); on the SECOND failure the host
//     re-fetches Pins to determine true state (RISK-1 recovery).
//   - REMOVE FAVORITE: `PATCH /workspaces/:ws/loom/blocks/:id` body `{ "favorite": false }`.
//
// All follow the MT-020/021/023 off-thread shape: spawn on the app's tokio runtime, deliver the parsed
// result into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next frame (HBR-QUIET — the
// render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it never depends on the
// `handshake_core` crate's types; the parsed shapes are the widget's own
// `graph::sidebar_panel::{SidebarBlock, BacklinkRow, UnlinkedRow}` (field-correct reuse of the verified
// backend shapes).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::sidebar_panel::{BacklinkRow, SidebarBlock, UnlinkedRow};

/// The externally-meaningful result of a Pins/Favorites fetch: the [`SidebarBlock`] list the
/// [`crate::graph::sidebar_panel::LoomSidebarPanel`] renders. `Ok` carries the blocks (possibly empty ->
/// the section empty state); `Err(msg)` a failure the section surfaces as an inline banner + Retry (AC9).
pub type SidebarBlockListCell = Arc<Mutex<Option<Result<Vec<SidebarBlock>, String>>>>;

/// The externally-meaningful result of a backlinks fetch: the [`BacklinkRow`] list (source block + edge
/// type). Stamped with the generation the host bumped on dispatch so a stale delivery is dropped (RISK-2).
pub type SidebarBacklinksCell = Arc<Mutex<Option<(u64, Result<Vec<BacklinkRow>, String>)>>>;

/// The externally-meaningful result of an unlinked-mentions fetch: the [`UnlinkedRow`] list. Stamped with
/// the dispatch generation so a stale delivery is dropped (RISK-2).
pub type SidebarUnlinkedCell = Arc<Mutex<Option<(u64, Result<Vec<UnlinkedRow>, String>)>>>;

/// REST client for the VERIFIED Loom sidebar surfaces the MT-024 sidebar panel binds: pins/favorites
/// view lists, per-block backlinks + unlinked-mentions, and the two-call pin removal + favorite removal.
/// Mirrors the `LoomTagClient` / `LoomFolderClient` / `LoomGraphClient` shape exactly.
#[derive(Clone)]
pub struct LoomSidebarClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomSidebarClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn view_url(&self, workspace_id: &str, view_type: &str) -> String {
        format!("{}/workspaces/{}/loom/views/{}", self.base_url, workspace_id, view_type)
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!("{}/workspaces/{}/loom/blocks/{}", self.base_url, workspace_id, block_id)
    }

    fn pin_order_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}/pin-order",
            self.base_url, workspace_id, block_id
        )
    }

    fn backlinks_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}/backlinks",
            self.base_url, workspace_id, block_id
        )
    }

    fn unlinked_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/blocks/{}/unlinked-mentions",
            self.base_url, workspace_id, block_id
        )
    }

    /// Pure request builder for the Pins fetch: `GET /loom/views/pins?limit=100`. Split out so a unit
    /// test asserts the EXACT verified URL + query without a live backend (the spawn path routes through
    /// this same builder, so the test proves the production request construction).
    pub fn pins_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.view_url(workspace_id, "pins"),
            query: vec![("limit".to_owned(), "100".to_owned())],
        }
    }

    /// Pure request builder for the Favorites fetch: `GET /loom/views/favorites?limit=100`.
    pub fn favorites_request(&self, workspace_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.view_url(workspace_id, "favorites"),
            query: vec![("limit".to_owned(), "100".to_owned())],
        }
    }

    /// Pure request builder for the per-block Backlinks fetch: `GET /loom/blocks/{id}/backlinks` (the
    /// verified dedicated MT-178 route — see the module comment for the contract correction).
    pub fn backlinks_request(&self, workspace_id: &str, block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.backlinks_url(workspace_id, block_id),
            query: vec![],
        }
    }

    /// Pure request builder for the per-block Unlinked-mentions fetch: `GET
    /// /loom/blocks/{id}/unlinked-mentions` (the verified dedicated MT-178 route).
    pub fn unlinked_request(&self, workspace_id: &str, block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.unlinked_url(workspace_id, block_id),
            query: vec![],
        }
    }

    /// Pure request builder for the pin-order CLEAR (the FIRST of the two-call pin removal):
    /// `PUT /loom/blocks/{id}/pin-order` body `{ "pin_order": null }` (RISK-1 / MC-1).
    pub fn clear_pin_order_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: self.pin_order_url(workspace_id, block_id),
            body: Some(serde_json::json!({ "pin_order": serde_json::Value::Null })),
        }
    }

    /// Pure request builder for the unpin PATCH (the SECOND of the two-call pin removal):
    /// `PATCH /loom/blocks/{id}` body `{ "pinned": false }`.
    pub fn unpin_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(serde_json::json!({ "pinned": false })),
        }
    }

    /// Pure request builder for the un-favorite PATCH: `PATCH /loom/blocks/{id}` body
    /// `{ "favorite": false }` (AC3).
    pub fn unfavorite_request(&self, workspace_id: &str, block_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(serde_json::json!({ "favorite": false })),
        }
    }

    /// Fetch the Pins list off the UI thread, delivering the parsed blocks into `cell` (AC1). The host
    /// sets the section loading flag before calling and clears it on delivery.
    pub fn fetch_pins(&self, workspace_id: &str, cell: SidebarBlockListCell) {
        let spec = self.pins_request(workspace_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_view_blocks(&client, &spec.url, &spec.query).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Fetch the Favorites list off the UI thread, delivering into `cell` (AC3 load).
    pub fn fetch_favorites(&self, workspace_id: &str, cell: SidebarBlockListCell) {
        let spec = self.favorites_request(workspace_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_view_blocks(&client, &spec.url, &spec.query).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Fetch the active block's Backlinks off the UI thread, delivering `(generation, Ok/Err)` into
    /// `cell` (AC4). `generation` is the value the host bumped on dispatch; the host drops the delivery
    /// if its generation has since advanced (RISK-2 stale-response guard).
    pub fn fetch_backlinks(
        &self,
        workspace_id: &str,
        block_id: &str,
        generation: u64,
        cell: SidebarBacklinksCell,
    ) {
        let spec = self.backlinks_request(workspace_id, block_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_backlink_rows(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((generation, result.map_err(|e| e.to_string())));
            }
        });
    }

    /// Fetch the active block's Unlinked mentions off the UI thread, delivering `(generation, Ok/Err)`
    /// into `cell` (AC5). Generation-stamped for the same RISK-2 stale-drop guard.
    pub fn fetch_unlinked(
        &self,
        workspace_id: &str,
        block_id: &str,
        generation: u64,
        cell: SidebarUnlinkedCell,
    ) {
        let spec = self.unlinked_request(workspace_id, block_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = fetch_unlinked_rows(&client, &spec.url).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((generation, result.map_err(|e| e.to_string())));
            }
        });
    }

    /// Remove a pin off the UI thread with the TWO-CALL flow (RISK-1 / MC-1 / AC2): `PUT /pin-order
    /// {pin_order:null}` THEN `PATCH {pinned:false}`. Delivers `Ok(())` only when BOTH succeed; if either
    /// fails, `Err(msg)` (the host rolls the optimistic removal back and re-fetches to find true state).
    /// Both calls are always issued in sequence (the React WorkspaceSidebar.tsx lines 297-298 flow); the
    /// pin-order clear is never skipped.
    pub fn remove_pin(&self, workspace_id: &str, block_id: &str, cell: DrawerActionCell) {
        let clear = self.clear_pin_order_request(workspace_id, block_id);
        let unpin = self.unpin_request(workspace_id, block_id);
        let clear_body = clear.body.unwrap_or_default();
        let unpin_body = unpin.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            // Call 1: clear the pin order. A failure here aborts the unpin (true partial state -> the host
            // re-fetches), exactly the RISK-1 recovery.
            let result = match put_expect_success(&client, &clear.url, &clear_body).await {
                Ok(()) => patch_expect_success(&client, &unpin.url, &unpin_body).await,
                Err(e) => Err(e),
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Remove a favorite off the UI thread: `PATCH {favorite:false}` (AC3). Single call.
    pub fn remove_favorite(&self, workspace_id: &str, block_id: &str, cell: DrawerActionCell) {
        let unfav = self.unfavorite_request(workspace_id, block_id);
        let body = unfav.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = patch_expect_success(&client, &unfav.url, &body).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }
}

/// Parse one verified `LoomBlock` JSON object into a [`SidebarBlock`]. `title` falls back to the block id
/// when null/empty; `content_type` defaults to "other". Returns `None` only when the block has no
/// `block_id` (a malformed row is skipped, not faked). Mirrors the `block_to_node`/`block_to_hub_member`
/// field reads so every Loom surface agrees on the verified block shape.
fn block_to_sidebar_block(block: &serde_json::Value) -> Option<SidebarBlock> {
    let block_id = block.get("block_id").and_then(|x| x.as_str())?.to_owned();
    let title = block
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&block_id)
        .to_owned();
    let content_type = block
        .get("content_type")
        .and_then(|x| x.as_str())
        .unwrap_or("other")
        .to_owned();
    Some(SidebarBlock::new(block_id, title, content_type))
}

/// `GET {url}?{query}` and parse the verified `LoomViewResponse::{Pins,Favorites} { blocks }` shape into
/// [`SidebarBlock`]s. A missing/empty `blocks` array yields an EMPTY list (the section empty state),
/// never an error. A non-success status or parse failure is an [`AppError`] (the AC9 error banner).
async fn fetch_view_blocks(
    client: &reqwest::Client,
    url: &str,
    query: &[(String, String)],
) -> Result<Vec<SidebarBlock>, AppError> {
    let v = get_json(client, url, query).await?;
    let blocks = v
        .get("blocks")
        .and_then(|b| b.as_array())
        .map(|arr| arr.iter().filter_map(block_to_sidebar_block).collect())
        .unwrap_or_default();
    Ok(blocks)
}

/// `GET {url}` and parse the verified `Vec<LoomBacklink>` shape into [`BacklinkRow`]s. Each backlink is
/// `{ edge:{ edge_type, source_block_id, .. }, source_block:{ block_id, title, .. }, context_snippet }`.
/// The row's open key + title come from `source_block`; the label is `edge.edge_type` (AC4). A backlink
/// missing its `source_block.block_id` is skipped. An empty array yields an empty list, never an error.
async fn fetch_backlink_rows(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<BacklinkRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    let rows = v
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|bl| {
                    let source = bl.get("source_block")?;
                    let block_id = source.get("block_id").and_then(|x| x.as_str())?.to_owned();
                    let title = source
                        .get("title")
                        .and_then(|x| x.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or(&block_id)
                        .to_owned();
                    let edge_type = bl
                        .get("edge")
                        .and_then(|e| e.get("edge_type"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("mention")
                        .to_owned();
                    Some(BacklinkRow::new(block_id, title, edge_type))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(rows)
}

/// `GET {url}` and parse the verified `Vec<LoomUnlinkedMention>` shape into [`UnlinkedRow`]s. Each mention
/// is `{ source_block:{ block_id, title, .. }, matched_term, snippet, match_offset }`; the row's open key
/// + title come from `source_block` (AC5). An empty array yields an empty list, never an error.
async fn fetch_unlinked_rows(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<UnlinkedRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    let rows = v
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let source = m.get("source_block")?;
                    let block_id = source.get("block_id").and_then(|x| x.as_str())?.to_owned();
                    let title = source
                        .get("title")
                        .and_then(|x| x.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or(&block_id)
                        .to_owned();
                    Some(UnlinkedRow::new(block_id, title))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(rows)
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-008 code-navigation transport (REUSE — not a second HTTP stack).
//
// The native code-editor's CodeNavClient (`code_editor::code_nav`) binds the EXISTING handshake_core
// code-nav GET routes. Those routes (`api::knowledge_code_nav`) require four backend-navigation
// identity headers on EVERY request, verified READ-ONLY against the running backend:
//   x-hsk-actor-id, x-hsk-kernel-task-run-id, x-hsk-session-run-id, x-hsk-actor-kind.
// A missing header is a deterministic HTTP 400 ("<header> header is required"), so the transport must
// attach them or the bind silently 400s. `actor-kind: system` is the verified valid kind for an
// automated UI nav (the same kind the backend's own quiet-nav lane uses). The shared `code_nav_get`
// helper below adds the headers + parses the JSON body via `serde_json::Value`, reusing the SAME
// `reqwest`/timeout/error shape as every other client in this module (NO new HTTP stack, NO dependency
// on the handshake_core crate). CodeNavClient calls THIS helper for all four routes.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// The backend-navigation identity headers required on every code-nav GET (verified against
/// `handshake_core::api::knowledge_code_nav::nav_context`) AND on every knowledge-document request
/// (verified against `handshake_core::api::knowledge_documents::doc_context`). A missing header is a
/// hard 400 ("<header> header is required"). `pub` so the rich-editor save/draft transport reuses the
/// SAME canonical header names rather than re-deriving the strings (the MT-020 missing-headers fix).
pub const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
pub const HSK_HEADER_KERNEL_TASK_RUN_ID: &str = "x-hsk-kernel-task-run-id";
pub const HSK_HEADER_SESSION_RUN_ID: &str = "x-hsk-session-run-id";
pub const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";

/// The stable actor identity the native editor presents to the backend code-nav API. `system` is the
/// verified-valid `x-hsk-actor-kind` for an automated UI navigation (the backend maps it to
/// `KernelActor::System`); the actor id names the native editor surface so the nav receipts are
/// attributable to it (HBR-SWARM attribution).
pub const CODE_NAV_ACTOR_ID: &str = "handshake-native-editor";
pub const CODE_NAV_ACTOR_KIND: &str = "system";

/// The stable identity the native editor presents to the backend KNOWLEDGE-DOCUMENT API (save +
/// draft). `operator` is the verified-valid `x-hsk-actor-kind` for an operator-initiated document
/// edit: the MT-158 permission matrix (`knowledge_document::permission`) grants `operator` the
/// `Write` action, so a save (`PUT /save`), a draft upsert (`PUT /draft`), and a draft clear
/// (`DELETE /draft`) are permitted. A MISSING `x-hsk-actor-kind` defaults to the least-privileged
/// (read-only) kind server-side and a write then 403s — so the kind MUST be asserted. The actor id
/// names the native editor surface so the document receipts are attributable to it (HBR-SWARM).
pub const DOC_ACTOR_ID: &str = "handshake-native-editor";
pub const DOC_ACTOR_KIND: &str = "operator";

/// `GET {url}?{query}` against the code-nav API with the four required backend-nav identity headers
/// attached, returning the parsed JSON body. `run_id` is folded into the per-request run ids so each
/// editor nav action is individually traceable (it never reaches the wrong field — the headers are
/// fixed names). A non-success status or a parse failure is an [`AppError`], never a panic — the
/// CodeNavClient turns that into graceful empty results (no completion / no hover), so the editor keeps
/// working when the backend is down (AC-004 graceful-degradation analog for the code-nav path).
///
/// REUSE: a fresh short-lived `reqwest::Client` + the same 5s timeout the other clients use. The
/// editor calls this from a spawned tokio task (HBR-QUIET — never the egui UI thread), so a slow
/// request never stalls the operator.
pub async fn code_nav_get(
    url: &str,
    query: &[(String, String)],
    run_id: &str,
) -> Result<serde_json::Value, AppError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .query(query)
        .header(HSK_HEADER_ACTOR_ID, CODE_NAV_ACTOR_ID)
        .header(HSK_HEADER_ACTOR_KIND, CODE_NAV_ACTOR_KIND)
        .header(HSK_HEADER_KERNEL_TASK_RUN_ID, format!("native-editor-{run_id}"))
        .header(HSK_HEADER_SESSION_RUN_ID, format!("native-editor-session-{run_id}"))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "GET code-nav non-success status {}",
            resp.status()
        )));
    }
    resp.json().await.map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-025 Loom WIKI-PROJECTION transport (REUSE — extends the MT-021/022/024 Loom read
// surface; mirrors the `LoomGraphClient` / `LoomFolderClient` / `LoomSidebarClient` shape exactly:
// off-thread spawn + one-slot delivery cell, speaks `serde_json::Value`, NEVER depends on the
// `handshake_core` crate types).
//
// SPEC-REALISM GATE (the MT-025 KERNEL_BUILDER gate + the MT-008/021/022/023/024 "verify, don't trust
// the contract" rule). VERIFIED READ-ONLY against the running backend
// `src/backend/handshake_core/src/{api,storage}/loom.rs`:
//   - `GET    /workspaces/{ws}/loom/wiki/{projection_id}`            -> `ServedWikiPage` =
//       `LoomWikiProjection { projection_id, workspace_id, title, source_block_ids[], rendered_content,
//       staleness_hash, rebuild_status, page_type?, compile_stamp?, page_links, created_at, updated_at }`
//       FLATTENED with `staleness_verdict` (the MT-242 LM-PWIKI-008 fail-closed verdict). CONFIRMED real
//       — the field shape the MT assumed EXISTS (handler `get_loom_wiki_projection` -> `ServedWikiPage`).
//   - `POST   /workspaces/{ws}/loom/wiki/{projection_id}/regenerate` -> `ServedWikiPage` (handler
//       `regenerate_loom_wiki_projection`). This is the REAL "rebuild" route — NOT the contract's assumed
//       `.../rebuild`. A regenerate recompiles `rendered_content` FROM `source_block_ids` and re-stamps.
//   - `POST   /workspaces/{ws}/loom/wiki/{projection_id}/overlays`   body `{ "annotation", "anchor"? }`
//       -> `LoomWikiOverlay { overlay_id, projection_id, workspace_id, annotation, anchor?, .. }` (handler
//       `add_loom_wiki_overlay`). This is the REAL, PERSISTED, CANONICAL write surface for a wiki page.
//
// THE CRITICAL FINDING (MC-1 / RISK-1, the contract's own doubt confirmed): there is **NO PATCH or PUT
// route that edits `rendered_content`**. The backend storage comment is explicit — `rendered_content` is
// "The rendered wiki markdown (regenerable; never authority)"; it is a DERIVED projection compiled FROM
// `source_block_ids` and is OVERWRITTEN on every regenerate. The ONLY canonical write is an OVERLAY
// annotation, stored in its OWN authority row precisely so "editing it never makes the projection
// canonical" (storage::LoomWikiOverlay doc). Therefore the native panel ships the "Edit overlay" as the
// REAL overlay-annotation write (POST .../overlays) and keeps `rendered_content` READ-ONLY — never a fake
// PATCH that would 404 or be silently clobbered on the next rebuild (Spec-Realism: no silently-broken
// write). The contract's PATCH/PUT-on-rendered_content path is a TYPED LIMITATION, surfaced in the widget.

/// A parsed Loom wiki projection (the `ServedWikiPage` GET/regenerate body), holding ONLY the fields the
/// native panel reads. `staleness_verdict` is the raw flattened verdict object (`serde_json::Value`,
/// typed `unknown` in the React API + `serde_json::Value` here per the MT note) so the "stale" display
/// logic can treat any non-null/non-`{"state":"fresh"}` value as stale without coupling to the verdict
/// schema. Parsing is total: a malformed/absent field falls back (never a panic, never a fabricated
/// value); `projection_id`/`title` fall back to the requested id / a placeholder so the panel is never
/// label-less.
#[derive(Debug, Clone, PartialEq)]
pub struct WikiProjection {
    pub projection_id: String,
    pub workspace_id: String,
    pub title: String,
    pub source_block_ids: Vec<String>,
    pub rendered_content: String,
    pub staleness_hash: String,
    pub rebuild_status: String,
    pub page_type: Option<String>,
    /// The raw flattened `staleness_verdict` object (or `Null` when absent). The display treats any
    /// non-null value whose `state` is not `"fresh"` as STALE (the MT RISK-5/MC-5 "treat any non-null
    /// non-fresh verdict as stale" rule; the React type is `unknown`).
    pub staleness_verdict: serde_json::Value,
}

impl WikiProjection {
    /// Parse one `ServedWikiPage` JSON object. `requested_id` is the projection id the GET was for, used
    /// as the `projection_id` fallback so a row is never id-less. Total: every field defaults safely.
    fn from_json(v: &serde_json::Value, requested_id: &str) -> Self {
        let str_field = |key: &str, fallback: &str| -> String {
            v.get(key)
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(fallback)
                .to_owned()
        };
        let projection_id = str_field("projection_id", requested_id);
        let source_block_ids = v
            .get("source_block_ids")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.as_str().map(|s| s.to_owned()))
                    .collect()
            })
            .unwrap_or_default();
        WikiProjection {
            projection_id,
            workspace_id: str_field("workspace_id", ""),
            // An empty title is a legitimate (if unusual) page; fall back to the projection id so the
            // heading is never blank, matching the graph/sidebar "never label-less" convention.
            title: str_field("title", requested_id),
            source_block_ids,
            // `rendered_content` may legitimately be empty (a freshly-compiled page with no sources);
            // keep it as-is (empty string), the panel shows a "No rendered wiki content." placeholder.
            rendered_content: v
                .get("rendered_content")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_owned(),
            staleness_hash: str_field("staleness_hash", ""),
            rebuild_status: str_field("rebuild_status", "unknown"),
            page_type: v
                .get("page_type")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned()),
            staleness_verdict: v
                .get("staleness_verdict")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        }
    }
}

/// One-slot delivery cell for an off-thread wiki-projection GET/regenerate result. `Ok(projection)`
/// carries the parsed page the panel renders; `Err(msg)` the failure the panel surfaces (AC8).
pub type WikiProjectionCell = Arc<Mutex<Option<Result<WikiProjection, String>>>>;

/// REST client for the VERIFIED Loom wiki-projection surface the MT-025 wiki page panel binds:
/// `GET /loom/wiki/{id}` (load), `POST /loom/wiki/{id}/regenerate` (rebuild), and
/// `POST /loom/wiki/{id}/overlays` (the REAL persisted overlay-annotation write — the "Edit overlay"
/// mechanism, since `rendered_content` itself has NO edit route). Mirrors the `LoomGraphClient` shape.
#[derive(Clone)]
pub struct LoomWikiClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomWikiClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn wiki_url(&self, workspace_id: &str, projection_id: &str) -> String {
        format!(
            "{}/workspaces/{}/loom/wiki/{}",
            self.base_url, workspace_id, projection_id
        )
    }

    /// Pure request builder for the wiki-page LOAD: `GET /loom/wiki/{id}` (no query). Split out so a unit
    /// test asserts the EXACT verified URL without a live backend (the spawn path routes through this same
    /// builder, so the test proves the production request construction — PROOF2 request-shape layer).
    pub fn load_request(&self, workspace_id: &str, projection_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.wiki_url(workspace_id, projection_id),
            query: vec![],
        }
    }

    /// Pure request builder for the REBUILD: `POST /loom/wiki/{id}/regenerate` (no body). This is the REAL
    /// route (`regenerate_loom_wiki_projection`) — the contract's assumed `.../rebuild` does NOT exist.
    pub fn regenerate_request(&self, workspace_id: &str, projection_id: &str) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/regenerate", self.wiki_url(workspace_id, projection_id)),
            body: None,
        }
    }

    /// Pure request builder for the OVERLAY-ANNOTATION write (the REAL "Edit overlay" persistence):
    /// `POST /loom/wiki/{id}/overlays` body `{ "annotation": <text> }` (+ optional `anchor`). This is the
    /// ONLY canonical wiki-page write (`add_loom_wiki_overlay`); `rendered_content` itself is read-only.
    /// PROOF3 asserts this exact `(POST, url, body)` is what the Save spawn path sends.
    pub fn add_overlay_request(
        &self,
        workspace_id: &str,
        projection_id: &str,
        annotation: &str,
        anchor: Option<&str>,
    ) -> RequestSpec {
        let mut body = serde_json::Map::new();
        body.insert(
            "annotation".to_owned(),
            serde_json::Value::String(annotation.to_owned()),
        );
        if let Some(anchor) = anchor.filter(|a| !a.is_empty()) {
            body.insert(
                "anchor".to_owned(),
                serde_json::Value::String(anchor.to_owned()),
            );
        }
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/overlays", self.wiki_url(workspace_id, projection_id)),
            body: Some(serde_json::Value::Object(body)),
        }
    }

    /// Fetch one wiki projection off the UI thread, delivering the parsed page into `cell` (AC1 load). The
    /// host sets `loading=true` before calling (so the spinner animates ONLY during this genuine in-flight
    /// fetch — the MT-015 idle-repaint rule) and clears it on delivery.
    pub fn fetch_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
        cell: WikiProjectionCell,
    ) {
        let spec = self.load_request(workspace_id, projection_id);
        let client = self.client.clone();
        let pid = projection_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_wiki_projection(&client, &spec.url, &pid).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Regenerate (rebuild) the projection off the UI thread, delivering the REBUILT page into `cell`
    /// (the optional Rebuild button). The POST returns the fresh `ServedWikiPage`, parsed like the GET.
    pub fn regenerate_projection(
        &self,
        workspace_id: &str,
        projection_id: &str,
        cell: WikiProjectionCell,
    ) {
        let spec = self.regenerate_request(workspace_id, projection_id);
        let client = self.client.clone();
        let pid = projection_id.to_owned();
        self.runtime.spawn(async move {
            let result = post_wiki_regenerate(&client, &spec.url, &pid).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Add an overlay annotation off the UI thread (the REAL "Save" of the Edit overlay), delivering the
    /// outcome into `cell`. `Ok(())` on a 2xx; `Err(msg)` on failure (AC5/PROOF5 — the host keeps the edit
    /// buffer and shows the error inline). The host re-fetches the projection on success (AC3).
    pub fn add_overlay(
        &self,
        workspace_id: &str,
        projection_id: &str,
        annotation: &str,
        anchor: Option<&str>,
        cell: ScmReceiptCell,
    ) {
        let spec = self.add_overlay_request(workspace_id, projection_id, annotation, anchor);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_expect_success(&client, &spec.url, &body).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }
}

/// `GET {url}` and parse the verified `ServedWikiPage` into a [`WikiProjection`]. A non-success status or
/// parse failure is an [`AppError`] (AC8). `requested_id` is the GET's projection id (the parse fallback).
async fn fetch_wiki_projection(
    client: &reqwest::Client,
    url: &str,
    requested_id: &str,
) -> Result<WikiProjection, AppError> {
    let v = get_json(client, url, &[]).await?;
    Ok(WikiProjection::from_json(&v, requested_id))
}

/// `POST {url}` (no body) for the regenerate route and parse the rebuilt `ServedWikiPage`. A non-success
/// status or parse failure is an [`AppError`].
async fn post_wiki_regenerate(
    client: &reqwest::Client,
    url: &str,
    requested_id: &str,
) -> Result<WikiProjection, AppError> {
    let resp = client
        .post(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!(
            "POST regenerate non-success status {}",
            resp.status()
        )));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    Ok(WikiProjection::from_json(&v, requested_id))
}

#[cfg(test)]
mod wiki_client_tests {
    use super::*;

    fn client() -> LoomWikiClient {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        // Leak the runtime handle for the test's lifetime (the builders are pure; no task is spawned).
        let handle = rt.handle().clone();
        std::mem::forget(rt);
        LoomWikiClient::new("http://test.local:1234", handle)
    }

    /// PROOF2 (request layer) / AC1: the LOAD hits the verified `GET /loom/wiki/{id}` route.
    #[test]
    fn load_request_hits_verified_get_route() {
        let spec = client().load_request("ws1", "proj-001");
        assert_eq!(spec.method, HttpMethod::Get);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/wiki/proj-001"
        );
        assert!(spec.query.is_empty());
    }

    /// AC: the REBUILD hits the verified `POST /loom/wiki/{id}/regenerate` (NOT the contract's
    /// non-existent `.../rebuild`), bodyless.
    #[test]
    fn regenerate_request_hits_verified_regenerate_route() {
        let spec = client().regenerate_request("ws1", "proj-001");
        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/wiki/proj-001/regenerate"
        );
        assert_eq!(spec.body, None);
    }

    /// PROOF3 (request layer) / AC3: the overlay-annotation SAVE hits the verified
    /// `POST /loom/wiki/{id}/overlays` route with the verified `{ "annotation": <text> }` body — the REAL
    /// persisted wiki-page write (NOT a fake PATCH on rendered_content).
    #[test]
    fn add_overlay_request_hits_verified_overlays_route() {
        let spec = client().add_overlay_request("ws1", "proj-001", "NEW CONTENT", None);
        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(
            spec.url,
            "http://test.local:1234/workspaces/ws1/loom/wiki/proj-001/overlays"
        );
        assert_eq!(spec.body, Some(serde_json::json!({ "annotation": "NEW CONTENT" })));
    }

    /// The optional anchor is included only when non-empty (a true merge — never sends `anchor:""`).
    #[test]
    fn add_overlay_request_includes_anchor_when_present() {
        let spec = client().add_overlay_request("ws1", "proj-001", "note", Some("block-7"));
        assert_eq!(
            spec.body,
            Some(serde_json::json!({ "annotation": "note", "anchor": "block-7" }))
        );
        let spec_empty = client().add_overlay_request("ws1", "proj-001", "note", Some(""));
        assert_eq!(spec_empty.body, Some(serde_json::json!({ "annotation": "note" })));
    }

    /// AC1 parse: the verified `ServedWikiPage` shape parses totally into [`WikiProjection`], including the
    /// flattened `staleness_verdict`, with safe fallbacks (the MT-022/023/024 "verify the field shape"
    /// rule — this asserts the shape the GET handler actually returns).
    #[test]
    fn parses_served_wiki_page_shape() {
        let body = serde_json::json!({
            "projection_id": "proj-001",
            "workspace_id": "ws1",
            "title": "Ownership model",
            "source_block_ids": ["blk-1", "blk-2", "blk-3"],
            "rendered_content": "# Ownership\nBorrow checker notes.",
            "staleness_hash": "abc123",
            "rebuild_status": "fresh",
            "page_type": "concept",
            "page_links": [],
            "created_at": "2026-06-19T00:00:00Z",
            "updated_at": "2026-06-19T00:00:00Z",
            "staleness_verdict": { "state": "fresh", "stamp_ledger_version": 7 }
        });
        let p = WikiProjection::from_json(&body, "proj-001");
        assert_eq!(p.projection_id, "proj-001");
        assert_eq!(p.title, "Ownership model");
        assert_eq!(p.source_block_ids.len(), 3);
        assert_eq!(p.rendered_content, "# Ownership\nBorrow checker notes.");
        assert_eq!(p.rebuild_status, "fresh");
        assert_eq!(p.page_type.as_deref(), Some("concept"));
        assert_eq!(p.staleness_verdict["state"], "fresh");
    }

    /// Parse is total on a degenerate body: missing fields fall back, `projection_id`/`title` to the
    /// requested id, and it NEVER panics (AC8 robustness).
    #[test]
    fn parse_is_total_on_missing_fields() {
        let p = WikiProjection::from_json(&serde_json::json!({}), "proj-xyz");
        assert_eq!(p.projection_id, "proj-xyz");
        assert_eq!(p.title, "proj-xyz");
        assert!(p.source_block_ids.is_empty());
        assert_eq!(p.rendered_content, "");
        assert_eq!(p.rebuild_status, "unknown");
        assert!(p.page_type.is_none());
        assert_eq!(p.staleness_verdict, serde_json::Value::Null);
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-027 BlockCollectionViews client: the VERIFIED saved-view surface (table / Kanban / calendar).
//
// Verified READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` + `app/src/lib/
// api.ts` (the MT-022/023/024/026 lesson — bind only confirmed shapes):
//   - GET    /workspaces/:ws/loom/views/definitions/:block_id          getBlockView -> BlockViewRecord
//   - POST   /workspaces/:ws/loom/views/definitions/:block_id/results  queryBlockViewResults
//       body {limit,offset} -> BlockViewResults{kind,blocks,groups,total_returned}  (POST, RISK-1)
//   - PATCH  /workspaces/:ws/loom/views/definitions/:block_id          updateBlockView body {definition}
//   - PATCH  /workspaces/:ws/loom/blocks/:block_id                     updateLoomBlock body
//       {add_tags,remove_tags} (top-level alongside the flattened update) — Kanban lane move
//   - POST   /workspaces/:ws/loom/views/definitions                    createBlockView body
//       {block_id?,title?,definition}
//
// Mirrors the `CanvasBoardClient` shape: pure `*_request` builders return a [`RequestSpec`] /
// [`GetRequestSpec`] (unit-testable WITHOUT a backend), and the off-thread fetch/dispatch methods
// deliver into `Arc<Mutex<Option<..>>>` cells the egui UI drains next frame (HBR-QUIET). Speaks
// `serde_json::Value` so it never depends on the `handshake_core` crate.
//
// FIELD-TYPE VERIFICATION (adversarial-review hardening, must-fix #1/#2/#3 — the route+method match
// was not enough; the query field VALUE TYPES and the group_by lane dependency had drifted):
//   - `BlockViewQuery.date_from/date_to` are backend type `Option<DateTime<Utc>>` with the DEFAULT
//     chrono serde (RFC3339, full timestamp). `definition_to_json` EXPANDS the calendar `YYYY-MM-DD`
//     to `<date>T00:00:00Z` / `<date>T23:59:59Z` so the PATCH body actually deserializes (a bare date
//     would 400/422). `date_serializes_as_rfc3339_*` prove the produced strings parse as `DateTime<Utc>`
//     (the SAME type+serde the backend field uses) — an adapter-boundary check, not a self-tautology.
//   - The native `BlockViewDefinition`/`BlockViewQuery` now model the FULL backend query
//     (content_type/mime/tag_ids/mention_ids) + `group_by` ({"kind":"tag"} | {"kind":"field","field"}),
//     so a sort/kind/date `updateBlockView` — which the backend persists as a FULL overwrite of
//     `view_definition_json` — never silently drops a server-side filter or a Kanban lane grouping.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::graph::block_collection_view::{
    BlockViewDefinition, BlockViewField, BlockViewGroupBy, BlockViewKind, BlockViewLane,
    BlockViewQuery, BlockViewResults, BlockViewSort, BlockViewSortDirection, LoomBlockRow,
};

/// The parsed result of a `getBlockView` fetch: the loaded definition + the view block id (so the host
/// can confirm identity). The block's own fields are not modeled here — only the definition the
/// sub-views need (the full `LoomBlock` is not required by the MT-027 surfaces).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockViewRecordData {
    pub view_block_id: String,
    pub definition: BlockViewDefinition,
}

/// One-slot delivery cell for an off-thread `getBlockView` result.
pub type BlockViewRecordCell = Arc<Mutex<Option<Result<BlockViewRecordData, String>>>>;

/// One-slot delivery cell for an off-thread `queryBlockViewResults` result.
pub type BlockViewResultsCell = Arc<Mutex<Option<Result<BlockViewResults, String>>>>;

/// One-slot delivery cell for an off-thread view MUTATION (updateBlockView / updateLoomBlock /
/// createBlockView). `Ok(view_block_id)` carries the (possibly new) view block id the host should be
/// on after the mutation; `Err(msg)` the failure. For a create, this is the NEW block id (so the host
/// switches to it); for update/card-move it echoes the current id.
pub type BlockViewOpCell = Arc<Mutex<Option<Result<String, String>>>>;

/// REST client for the VERIFIED MT-262 block-collection-view surface. Drives the definition read, the
/// query (POST!), the sort/kind/date persist, the Kanban card-move tag mutation, and view creation off
/// the UI thread.
#[derive(Clone)]
pub struct BlockViewClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl BlockViewClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn definitions_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/views/definitions", self.base_url, workspace_id)
    }

    fn definition_url(&self, workspace_id: &str, view_block_id: &str) -> String {
        format!("{}/{}", self.definitions_url(workspace_id), view_block_id)
    }

    fn block_url(&self, workspace_id: &str, block_id: &str) -> String {
        format!("{}/workspaces/{}/loom/blocks/{}", self.base_url, workspace_id, block_id)
    }

    /// Pure request builder for `GET .../views/definitions/:block_id` (getBlockView).
    pub fn get_view_request(&self, workspace_id: &str, view_block_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.definition_url(workspace_id, view_block_id),
            query: vec![],
        }
    }

    /// Pure request builder for `POST .../views/definitions/:block_id/results` (queryBlockViewResults).
    /// The VERIFIED method is POST with a JSON body `{limit, offset}` — NOT a GET with query params
    /// (RISK-1 / MC-1: a GET would 405 or silently send params as a query string).
    pub fn query_results_request(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        limit: u32,
        offset: u32,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: format!("{}/results", self.definition_url(workspace_id, view_block_id)),
            body: Some(serde_json::json!({ "limit": limit, "offset": offset })),
        }
    }

    /// Pure request builder for `PATCH .../views/definitions/:block_id` (updateBlockView). The VERIFIED
    /// body is `{definition: <BlockViewDefinition JSON>}` (NOT the bare definition at top level).
    pub fn update_view_request(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        definition: &BlockViewDefinition,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.definition_url(workspace_id, view_block_id),
            body: Some(serde_json::json!({ "definition": definition_to_json(definition) })),
        }
    }

    /// Pure request builder for `PATCH .../loom/blocks/:block_id` (updateLoomBlock) carrying the Kanban
    /// lane-move tag mutation. The VERIFIED body has `add_tags`/`remove_tags` at the TOP level (the
    /// backend `LoomBlockPatchRequest` reads them alongside the flattened update).
    pub fn card_move_request(
        &self,
        workspace_id: &str,
        block_id: &str,
        add_tags: &[String],
        remove_tags: &[String],
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Patch,
            url: self.block_url(workspace_id, block_id),
            body: Some(serde_json::json!({
                "add_tags": add_tags,
                "remove_tags": remove_tags,
            })),
        }
    }

    /// Pure request builder for `POST .../views/definitions` (createBlockView). The VERIFIED body is
    /// `{block_id?, title?, definition}`; the `block_id` is omitted so the backend mints a new one.
    pub fn create_view_request(
        &self,
        workspace_id: &str,
        title: &str,
        definition: &BlockViewDefinition,
    ) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.definitions_url(workspace_id),
            body: Some(serde_json::json!({
                "title": title,
                "definition": definition_to_json(definition),
            })),
        }
    }

    /// Fetch the view definition off the UI thread, delivering the parsed [`BlockViewRecordData`] into
    /// `cell`.
    pub fn fetch_view(&self, workspace_id: &str, view_block_id: &str, cell: BlockViewRecordCell) {
        let spec = self.get_view_request(workspace_id, view_block_id);
        let client = self.client.clone();
        let id = view_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_block_view(&client, &spec.url, &id).await.map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }

    /// Run the view query (POST!) off the UI thread, delivering the parsed [`BlockViewResults`] into
    /// `cell`.
    pub fn query_results(
        &self,
        workspace_id: &str,
        view_block_id: &str,
        limit: u32,
        offset: u32,
        cell: BlockViewResultsCell,
    ) {
        let spec = self.query_results_request(workspace_id, view_block_id, limit, offset);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = post_block_view_results(&client, &spec.url, &body)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }

    /// Send a prebuilt update/card-move [`RequestSpec`] off the UI thread, delivering `Ok(echo_id)` /
    /// `Err(msg)` into `cell`. `echo_id` is the view block id the host stays on (the host passes its
    /// current id). The host re-queries after a 2xx.
    pub fn dispatch(&self, spec: RequestSpec, echo_id: String, cell: BlockViewOpCell) {
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = send_block_view_mutation(&client, &spec).await.map(|_| echo_id);
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Create a new view off the UI thread, delivering the NEW view block id into `cell` (so the host
    /// switches to it). The body is `createBlockView`'s `{title, definition}`.
    pub fn create_view(
        &self,
        workspace_id: &str,
        title: &str,
        definition: &BlockViewDefinition,
        cell: BlockViewOpCell,
    ) {
        let spec = self.create_view_request(workspace_id, title, definition);
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        let url = spec.url.clone();
        self.runtime.spawn(async move {
            let result = post_create_block_view(&client, &url, &body)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }
}

/// Expand a calendar `YYYY-MM-DD` bound to the full RFC3339 instant the backend's
/// `BlockViewQuery.date_from/date_to: Option<DateTime<Utc>>` (default chrono serde) ACCEPTS. The backend
/// REJECTS a bare date-only string (must-fix #1 / backend-shape #4 — `updateBlockView` would 400/422),
/// so `date_from` becomes the start-of-day `<date>T00:00:00Z` and `date_to` the INCLUSIVE end-of-day
/// `<date>T23:59:59Z`. A value already carrying a time component (`T`) is passed through unchanged (so a
/// future full-timestamp input still round-trips). `end_of_day=false` => 00:00:00, `true` => 23:59:59.
fn expand_iso_date_to_rfc3339(date: &str, end_of_day: bool) -> String {
    let trimmed = date.trim();
    if trimmed.contains('T') {
        // Already a full timestamp — leave it (the read path slices, but a caller may pass full).
        return trimmed.to_owned();
    }
    let time = if end_of_day { "23:59:59" } else { "00:00:00" };
    format!("{trimmed}T{time}Z")
}

/// Serialize a [`BlockViewDefinition`] to the VERIFIED wire JSON the backend `BlockViewDefinition`
/// deserializes (snake_case kind/field/direction strings). The FULL query (date window expanded to
/// RFC3339, content_type, mime, tag_ids, mention_ids) and `group_by` are written so a sort/kind/date
/// `updateBlockView` round-trip — which the backend persists as a FULL overwrite of
/// `view_definition_json` — never silently drops a server-side filter or a Kanban grouping (must-fix
/// #1/#2/#3). The backend defaults only genuinely-absent fields (serde `#[serde(default)]`).
fn definition_to_json(def: &BlockViewDefinition) -> serde_json::Value {
    let q = &def.query;
    let mut query = serde_json::Map::new();
    // date_from/date_to: EXPAND the calendar `YYYY-MM-DD` to a full RFC3339 instant — the backend field
    // is `Option<DateTime<Utc>>` and rejects a bare date (must-fix #1).
    if let Some(from) = &q.date_from {
        query.insert(
            "date_from".to_owned(),
            serde_json::Value::String(expand_iso_date_to_rfc3339(from, false)),
        );
    }
    if let Some(to) = &q.date_to {
        query.insert(
            "date_to".to_owned(),
            serde_json::Value::String(expand_iso_date_to_rfc3339(to, true)),
        );
    }
    if let Some(ct) = &q.content_type {
        query.insert("content_type".to_owned(), serde_json::Value::String(ct.clone()));
    }
    if let Some(mime) = &q.mime {
        query.insert("mime".to_owned(), serde_json::Value::String(mime.clone()));
    }
    if !q.tag_ids.is_empty() {
        query.insert(
            "tag_ids".to_owned(),
            serde_json::Value::Array(
                q.tag_ids.iter().map(|t| serde_json::Value::String(t.clone())).collect(),
            ),
        );
    }
    if !q.mention_ids.is_empty() {
        query.insert(
            "mention_ids".to_owned(),
            serde_json::Value::Array(
                q.mention_ids.iter().map(|m| serde_json::Value::String(m.clone())).collect(),
            ),
        );
    }
    let mut obj = serde_json::Map::new();
    obj.insert("kind".to_owned(), serde_json::Value::String(def.kind.as_str().to_owned()));
    if !query.is_empty() {
        obj.insert("query".to_owned(), serde_json::Value::Object(query));
    }
    if !def.columns.is_empty() {
        obj.insert(
            "columns".to_owned(),
            serde_json::Value::Array(
                def.columns.iter().map(|f| serde_json::Value::String(f.as_str().to_owned())).collect(),
            ),
        );
    }
    // group_by: serialize the verified tagged-enum shape ({"kind":"tag"} | {"kind":"field","field":..})
    // so a Kanban view's lane grouping survives the full-overwrite persist (must-fix #3).
    if let Some(group_by) = &def.group_by {
        obj.insert("group_by".to_owned(), group_by_to_json(group_by));
    }
    if let Some(sort) = def.sort {
        obj.insert(
            "sort".to_owned(),
            serde_json::json!({
                "field": sort.field.as_str(),
                "direction": sort.direction.as_str(),
            }),
        );
    }
    if let Some(field) = def.calendar_date_field {
        obj.insert(
            "calendar_date_field".to_owned(),
            serde_json::Value::String(field.as_str().to_owned()),
        );
    }
    serde_json::Value::Object(obj)
}

/// Serialize the verified `BlockViewGroupBy` tagged-enum wire shape (`#[serde(tag="kind",
/// rename_all="snake_case")]`): `{"kind":"tag"}` or `{"kind":"field","field":"<field>"}`.
fn group_by_to_json(group_by: &BlockViewGroupBy) -> serde_json::Value {
    match group_by {
        BlockViewGroupBy::Tag => serde_json::json!({ "kind": "tag" }),
        BlockViewGroupBy::Field { field } => {
            serde_json::json!({ "kind": "field", "field": field.as_str() })
        }
    }
}

/// Parse the verified `BlockViewGroupBy` tagged-enum JSON. An unknown/missing `kind` (or a `field`
/// variant with an unparseable field) yields `None` — a malformed grouping is dropped, never faked.
fn group_by_from_json(v: &serde_json::Value) -> Option<BlockViewGroupBy> {
    match v.get("kind").and_then(|x| x.as_str())? {
        "tag" => Some(BlockViewGroupBy::Tag),
        "field" => {
            let field = BlockViewField::parse_str(v.get("field").and_then(|x| x.as_str())?)?;
            Some(BlockViewGroupBy::Field { field })
        }
        _ => None,
    }
}

/// Parse the VERIFIED `BlockViewDefinition` JSON into the native projection. Unknown kinds default to
/// table; unknown fields are dropped (never faked).
pub fn definition_from_json(v: &serde_json::Value) -> BlockViewDefinition {
    let kind = BlockViewKind::parse_str(v.get("kind").and_then(|x| x.as_str()).unwrap_or("table"));
    let columns = v
        .get("columns")
        .and_then(|c| c.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str()).filter_map(BlockViewField::parse_str).collect())
        .unwrap_or_default();
    let sort = v.get("sort").and_then(|s| {
        let field = BlockViewField::parse_str(s.get("field").and_then(|x| x.as_str())?)?;
        let direction = match s.get("direction").and_then(|x| x.as_str()) {
            Some("asc") => BlockViewSortDirection::Asc,
            _ => BlockViewSortDirection::Desc,
        };
        Some(BlockViewSort { field, direction })
    });
    let group_by = v.get("group_by").and_then(group_by_from_json);
    let calendar_date_field = v
        .get("calendar_date_field")
        .and_then(|x| x.as_str())
        .and_then(BlockViewField::parse_str);
    let query = v.get("query").map(parse_block_view_query).unwrap_or_default();
    BlockViewDefinition { kind, query, columns, group_by, sort, calendar_date_field }
}

/// Parse the FULL VERIFIED `BlockViewQuery` JSON into the native projection. The backend stores
/// `date_from`/`date_to` as ISO datetimes; the calendar surface only needs the `YYYY-MM-DD` prefix, so
/// the native projection slices it (the write path re-expands it to RFC3339). `content_type`/`mime`/
/// `tag_ids`/`mention_ids` are carried verbatim so a later `updateBlockView` round-trip never drops the
/// user's server-side filters (must-fix #2).
fn parse_block_view_query(v: &serde_json::Value) -> BlockViewQuery {
    let slice_date = |key: &str| {
        v.get(key)
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.chars().take(10).collect::<String>())
    };
    let string_array = |key: &str| {
        v.get(key)
            .and_then(|x| x.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(ToOwned::to_owned)).collect())
            .unwrap_or_default()
    };
    BlockViewQuery {
        date_from: slice_date("date_from"),
        date_to: slice_date("date_to"),
        content_type: v.get("content_type").and_then(|x| x.as_str()).map(ToOwned::to_owned),
        mime: v.get("mime").and_then(|x| x.as_str()).map(ToOwned::to_owned),
        tag_ids: string_array("tag_ids"),
        mention_ids: string_array("mention_ids"),
    }
}

/// Parse one VERIFIED `LoomBlock` JSON object into a [`LoomBlockRow`] (the cell-value + bucket-key +
/// title fields the sub-views read). `derived.{backlink,mention,tag}_count` live under the nested
/// `derived` object; `title`/`journal_date`/`original_filename` are optional. Returns `None` only when
/// `block_id` is missing (a malformed row is skipped, not faked).
pub fn loom_block_row_from_json(b: &serde_json::Value) -> Option<LoomBlockRow> {
    let block_id = b.get("block_id").and_then(|x| x.as_str())?.to_owned();
    let derived = b.get("derived");
    let count = |key: &str| {
        derived
            .and_then(|d| d.get(key))
            .and_then(|x| x.as_i64())
            .or_else(|| b.get(key).and_then(|x| x.as_i64()))
            .unwrap_or(0)
    };
    Some(LoomBlockRow {
        title: b.get("title").and_then(|x| x.as_str()).map(ToOwned::to_owned),
        original_filename: b.get("original_filename").and_then(|x| x.as_str()).map(ToOwned::to_owned),
        content_type: b.get("content_type").and_then(|x| x.as_str()).unwrap_or("note").to_owned(),
        journal_date: b.get("journal_date").and_then(|x| x.as_str()).map(ToOwned::to_owned),
        created_at: b.get("created_at").and_then(|x| x.as_str()).unwrap_or("").to_owned(),
        updated_at: b.get("updated_at").and_then(|x| x.as_str()).unwrap_or("").to_owned(),
        pinned: b.get("pinned").and_then(|x| x.as_bool()).unwrap_or(false),
        favorite: b.get("favorite").and_then(|x| x.as_bool()).unwrap_or(false),
        backlink_count: count("backlink_count"),
        mention_count: count("mention_count"),
        tag_count: count("tag_count"),
        block_id,
    })
}

/// Parse the VERIFIED `BlockViewResults` JSON (`{kind, blocks, groups?, total_returned}`) into the
/// native projection. A missing/empty `blocks`/`groups` is an EMPTY result, never an error (AC10).
pub fn results_from_json(v: &serde_json::Value) -> BlockViewResults {
    let blocks = v
        .get("blocks")
        .and_then(|b| b.as_array())
        .map(|arr| arr.iter().filter_map(loom_block_row_from_json).collect())
        .unwrap_or_default();
    let groups = v
        .get("groups")
        .and_then(|g| g.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|lane| {
                    let key = lane.get("key").and_then(|x| x.as_str())?.to_owned();
                    let blocks = lane
                        .get("blocks")
                        .and_then(|b| b.as_array())
                        .map(|a| a.iter().filter_map(loom_block_row_from_json).collect())
                        .unwrap_or_default();
                    Some(BlockViewLane { key, blocks })
                })
                .collect()
        })
        .unwrap_or_default();
    BlockViewResults {
        kind_str: v.get("kind").and_then(|x| x.as_str()).unwrap_or("table").to_owned(),
        blocks,
        groups,
        total_returned: v.get("total_returned").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
    }
}

/// `GET {url}` and parse the verified `BlockViewRecord` (`{block, definition}`) into a
/// [`BlockViewRecordData`]. The `block.block_id` (or the requested id) identifies the view.
async fn fetch_block_view(
    client: &reqwest::Client,
    url: &str,
    requested_id: &str,
) -> Result<BlockViewRecordData, AppError> {
    let v = get_json(client, url, &[]).await?;
    let definition = v
        .get("definition")
        .map(definition_from_json)
        .unwrap_or_else(|| BlockViewDefinition::of_kind(BlockViewKind::Table));
    let view_block_id = v
        .get("block")
        .and_then(|b| b.get("block_id"))
        .and_then(|x| x.as_str())
        .unwrap_or(requested_id)
        .to_owned();
    Ok(BlockViewRecordData { view_block_id, definition })
}

/// `POST {url}` (body `{limit,offset}`) and parse the verified `BlockViewResults` (RISK-1: POST not GET).
async fn post_block_view_results(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<BlockViewResults, AppError> {
    let v = post_json(client, url, body).await?;
    Ok(results_from_json(&v))
}

/// `POST {url}` (createBlockView body) and read the NEW view block id from the returned `BlockViewRecord`.
async fn post_create_block_view(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<String, AppError> {
    let v = post_json(client, url, body).await?;
    let id = v
        .get("block")
        .and_then(|b| b.get("block_id"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| AppError::Parse("createBlockView response missing block.block_id".to_owned()))?
        .to_owned();
    Ok(id)
}

/// Send one block-view mutation (update/card-move) by method, treating any 2xx as success (the host
/// re-queries for the body).
async fn send_block_view_mutation(
    client: &reqwest::Client,
    spec: &RequestSpec,
) -> Result<(), AppError> {
    let empty = serde_json::json!({});
    let body = spec.body.as_ref().unwrap_or(&empty);
    match spec.method {
        HttpMethod::Post => post_expect_success(client, &spec.url, body).await,
        HttpMethod::Patch => patch_expect_success(client, &spec.url, body).await,
        _ => Err(AppError::Http("block-view mutation must be POST or PATCH".to_owned())),
    }
}

/// `POST {url}` with a JSON body and return the parsed JSON response. A non-success status or a parse
/// failure is an [`AppError`]. Used by the block-view query + create (both POST + read a body).
async fn post_json(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let resp = client
        .post(url)
        .timeout(Duration::from_secs(5))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("POST non-success status {}", resp.status())));
    }
    resp.json().await.map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-028 LoomSearchV2 client: the VERIFIED hybrid-search + save-as-view surface.
//
// Verified READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs` (the MT-022..027
// lesson — bind only confirmed shapes; do NOT guess a route):
//   - POST /workspaces/:ws/loom/search-v2  (api/loom.rs route table line 294 + handler `loom_search_v2`)
//       body  `LoomSearchV2Body {query, content_type?, tag_ids?, graph_boost, limit, offset?}`
//       reply `LoomSearchV2Response {hits:[LoomSearchV2Hit{block,score,fts_rank,trgm_sim,vector_sim,
//              edge_degree,highlight}], content_type_facets: {ct->count}, semantic_available, total}`
//       (storage/loom.rs 637-712; LoomBlockContentType serializes `snake_case`, e.g. `tag_hub`).
//   - SAVE-AS-VIEW reuses the MT-027 VERIFIED createBlockView route:
//       POST /workspaces/:ws/loom/views/definitions  body `{title, definition}` -> `{block:{block_id}}`
//     The MT-028 contract's bare `/loom/views` is STALE (RISK-3 / MC-3); MT-027 proved the real route is
//     `/loom/views/definitions` and the body is `{title, definition}` (NOT `{kind, query, columns}` at
//     top level — the React `createBlockView(ws, definition, {title})` flattens to the SAME wire shape).
//     The save-as-view `definition` carries `{kind:"table", query:{content_type?}, columns:[...]}`.
//
// Mirrors the `BlockViewClient` shape: pure `*_request` builders return a [`RequestSpec`] (unit-testable
// WITHOUT a backend), and the off-thread methods deliver into `Arc<Mutex<Option<..>>>` cells the egui UI
// drains next frame (HBR-QUIET: the search HTTP call is NEVER on the UI thread). The deserialized result
// types are local serde structs whose field names match the snake_case backend JSON EXACTLY (RISK-6: a
// rename would silently null a field), modelling only the fields the panel displays.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// A Loom block REFERENCE returned in a search hit (open-in-place, never a content copy). Only the
/// display fields the panel renders are modelled; the backend `LoomBlock` carries more (workspace_id,
/// timestamps, derived counts) that the search surface does not need. `content_type` is a plain
/// `String` (the backend serializes the `LoomBlockContentType` enum as a `snake_case` string, e.g.
/// `"note"`, `"tag_hub"`), so the native struct never re-encodes the enum and an unknown future
/// content_type degrades to its raw string instead of a deserialize error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct LoomSearchBlock {
    pub block_id: String,
    pub content_type: String,
    #[serde(default)]
    pub title: Option<String>,
}

impl LoomSearchBlock {
    /// The display title: the block's own `title`, or the `block_id` as a fallback (the React parity
    /// reference renders `hit.block.title ?? hit.block.block_id`).
    pub fn display_title(&self) -> &str {
        self.title.as_deref().filter(|t| !t.is_empty()).unwrap_or(&self.block_id)
    }
}

/// One hybrid-search hit. Field names match the backend `storage::LoomSearchV2Hit` snake_case JSON
/// EXACTLY. The per-modality sub-scores (fts_rank/trgm_sim/vector_sim/edge_degree) are retained so a
/// later MT or test can prove which modality matched; the panel itself renders `score` + `highlight`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct LoomSearchV2Hit {
    pub block: LoomSearchBlock,
    pub score: f64,
    #[serde(default)]
    pub fts_rank: f64,
    #[serde(default)]
    pub trgm_sim: f64,
    #[serde(default)]
    pub vector_sim: f64,
    #[serde(default)]
    pub edge_degree: i64,
    /// ts_headline highlight with literal `<mark>…</mark>` markers; rendered as colored runs (NOT raw
    /// HTML) by [`crate::loom_search_v2::parse_highlight_segments`].
    #[serde(default)]
    pub highlight: String,
}

/// A faceted, ranked LoomSearchV2 result set. Field names match the backend `storage::LoomSearchV2Response`
/// snake_case JSON EXACTLY. `content_type_facets` keeps the backend's `BTreeMap<String,i64>` shape so the
/// facet order is deterministic before the panel re-sorts by count.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct LoomSearchV2Response {
    pub hits: Vec<LoomSearchV2Hit>,
    #[serde(default)]
    pub content_type_facets: std::collections::BTreeMap<String, i64>,
    /// `true` => the semantic (pgvector kNN) modality contributed; `false` => typed keyword/trigram
    /// fallback (no embedding model configured). The status line reads this to show `(semantic on)`
    /// vs `(keyword/fuzzy only)` HONESTLY (RISK-7: never claim semantic when it is off).
    #[serde(default)]
    pub semantic_available: bool,
    #[serde(default)]
    pub total: i64,
}

/// The request body for `POST /loom/search-v2`. Matches the backend `LoomSearchV2Body` (snake_case).
/// `graph_boost` is always `1.0` and `limit` `25` for the MT-028 baseline (the React parity reference
/// sends exactly these); `content_type` is the active facet filter (omitted via `skip_serializing_if`
/// when `None`, exactly as the backend's `#[serde(default)] Option` accepts).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LoomSearchV2Body {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub graph_boost: f64,
    pub limit: u32,
}

impl LoomSearchV2Body {
    /// The MT-028 baseline body: graph_boost 1.0, limit 25 (the React parity defaults), with the active
    /// facet (if any) as the `content_type` filter.
    pub fn baseline(query: impl Into<String>, content_type: Option<String>) -> Self {
        Self {
            query: query.into(),
            content_type,
            graph_boost: 1.0,
            limit: 25,
        }
    }
}

/// One-slot delivery cell for an off-thread LoomSearchV2 result (the egui UI drains it next frame).
pub type LoomSearchCell = Arc<Mutex<Option<Result<LoomSearchV2Response, String>>>>;

/// One-slot delivery cell for an off-thread save-as-view result. `Ok(block_id)` is the NEW view block
/// id (shown in the panel's view-status label); `Err(msg)` the failure string.
pub type SaveViewCell = Arc<Mutex<Option<Result<String, String>>>>;

/// REST client for the VERIFIED MT-264 LoomSearchV2 surface: the hybrid search POST and the save-results
/// -as-view POST (reusing the MT-027 createBlockView route). Drives both off the UI thread, delivering
/// into the delivery cells the egui panel drains. Speaks `serde_json` so it never depends on the
/// `handshake_core` crate.
#[derive(Clone)]
pub struct LoomSearchV2Client {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl LoomSearchV2Client {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn search_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/search-v2", self.base_url, workspace_id)
    }

    fn views_definitions_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/views/definitions", self.base_url, workspace_id)
    }

    /// Pure request builder for `POST .../loom/search-v2`. The VERIFIED body is the snake_case
    /// `LoomSearchV2Body`; asserting it proves the production request construction (the spawn path below
    /// routes through this SAME builder) without a live backend.
    pub fn search_request(&self, workspace_id: &str, body: &LoomSearchV2Body) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Post,
            url: self.search_url(workspace_id),
            body: Some(serde_json::to_value(body).unwrap_or_default()),
        }
    }

    /// Pure request builder for the save-as-view `POST .../loom/views/definitions` (createBlockView).
    /// The VERIFIED body is `{title, definition}` where `definition = {kind:"table", query, columns}`
    /// (MT-027's proven shape; the MT-028 contract's bare `/loom/views` is stale). `active_content_type`
    /// becomes the saved view's `query.content_type` filter (omitted when `None`).
    pub fn save_view_request(
        &self,
        workspace_id: &str,
        query_text: &str,
        active_content_type: Option<&str>,
    ) -> RequestSpec {
        let query = match active_content_type {
            Some(ct) => serde_json::json!({ "content_type": ct }),
            None => serde_json::json!({}),
        };
        let definition = serde_json::json!({
            "kind": "table",
            "query": query,
            "columns": ["title", "content_type", "updated"],
        });
        RequestSpec {
            method: HttpMethod::Post,
            url: self.views_definitions_url(workspace_id),
            body: Some(serde_json::json!({
                "title": format!("Search: {}", query_text.trim()),
                "definition": definition,
            })),
        }
    }

    /// Run the hybrid search (POST) off the UI thread, delivering the parsed [`LoomSearchV2Response`]
    /// into `cell`. The egui UI thread returns immediately; the spawned tokio task does the network I/O
    /// and writes the result, which the UI drains next frame (HBR-QUIET — no UI-thread network block).
    pub fn search(&self, workspace_id: &str, body: &LoomSearchV2Body, cell: LoomSearchCell) {
        let spec = self.search_request(workspace_id, body);
        let req_body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        let url = spec.url;
        self.runtime.spawn(async move {
            let result = post_loom_search_v2(&client, &url, &req_body)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }

    /// Create the saved view (POST) off the UI thread, delivering the NEW view block id into `cell`.
    pub fn save_view(
        &self,
        workspace_id: &str,
        query_text: &str,
        active_content_type: Option<&str>,
        cell: SaveViewCell,
    ) {
        let spec = self.save_view_request(workspace_id, query_text, active_content_type);
        let req_body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        let url = spec.url;
        self.runtime.spawn(async move {
            let result = post_create_block_view(&client, &url, &req_body)
                .await
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }
}

/// `POST {url}` (LoomSearchV2 body) and parse the verified [`LoomSearchV2Response`]. A non-success
/// status or a parse failure is an [`AppError`] (NEVER a panic; the panel shows the error string).
async fn post_loom_search_v2(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<LoomSearchV2Response, AppError> {
    let v = post_json(client, url, body).await?;
    serde_json::from_value(v).map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-029 Find-in-Files + Replace-in-Files transport (E4 Search).
//
// Three clients drive the native WorkspaceSearchPanel port, each binding a backend route VERIFIED
// READ-ONLY against the running `src/backend/handshake_core` + the React reference (api.ts / loom.rs /
// workspaces.rs / knowledge_documents.rs), NOT the MT-029 contract body (whose route names were partly
// stale — the recurring backend-shape lesson):
//
//   - SEARCH binds `GET /workspaces/{ws}/loom/graph-search` (handler `search_loom_graph` ->
//     `Vec<LoomGraphSearchResult>` carrying source_kind/result_kind/ref_id/title/excerpt/metadata/block).
//     This is what the React `searchLoomGraph()` actually calls (api.ts:1320-1341) — NOT the plain
//     `/loom/search` (handler `search_loom_blocks` -> `Vec<{block,score}>` with NO source_kind/ref_id,
//     so it cannot satisfy documentIdFromHit). Verified params (loom.rs `LoomSearchQueryParams` +
//     api.test.ts:771): q, source_kinds (comma-joined), tag_ids, mention_ids, case_sensitive,
//     whole_word, `regex` (NOT isRegex), path, limit (server-capped at 500), offset.
//   - BOOKMARKS bind `GET/PUT /workspaces/{ws}/search-bookmarks` (api/workspaces.rs:61-62,806-869). GET
//     returns `{workspace_id, bookmark_state:Option<Value>, ..}`; PUT body is `{bookmark_state:Value}`.
//     The bookmark blob (carried INSIDE bookmark_state) is `{schema_id:"hsk.workspace_search_bookmark_
//     state@1", bookmarks:[..]}` — the schema_id lives in the blob (RISK-6).
//   - RICH-DOC load binds `GET /knowledge/documents/{id}` -> `{document:{rich_document_id,doc_version,
//     title,content_json,crdt_document_id,..}, tree, code_nodes}`; save binds `PUT /knowledge/documents/
//     {id}/save` `{expected_version, content_json}` -> `{document:{doc_version,..}, save_receipt_event_id}`;
//     409 = optimistic-concurrency conflict (the MT-017/020 VERIFIED routes — NOT the contract's
//     /knowledge/rich-documents PATCH). The four `x-hsk-*` document identity headers are REQUIRED (a
//     missing one is a hard 400 / read-only 403), reusing the canonical DOC_* constants above.
//
// All follow the MT-020/028 off-thread shape: spawn on the app runtime, deliver into an
// `Arc<Mutex<Option<..>>>` cell the egui UI drains next frame (HBR-QUIET). Speaks `serde_json::Value`
// so it never depends on the `handshake_core` crate types.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// One graph-search hit, the native projection of the backend `LoomGraphSearchResult`. Field names
/// match the snake_case JSON EXACTLY. `metadata`/`block` are raw `serde_json::Value` so the
/// documentId-from-hit logic can read whatever keys the backend attaches without coupling to a schema.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct LoomGraphSearchHit {
    pub source_kind: String,
    pub result_kind: String,
    pub ref_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub excerpt: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub block: Option<serde_json::Value>,
}

/// One-slot delivery cell for an off-thread paginated search: `Ok((hits, result_set_key))` carries the
/// fully-paginated hit set tagged with the search-plan key it was fetched under (the stale-result
/// guard); `Err(msg)` the failure.
pub type GraphSearchCell = Arc<Mutex<Option<Result<(Vec<LoomGraphSearchHit>, String), String>>>>;

/// One-slot delivery cell for an off-thread bookmark op: `Ok((bookmark_state_blob, status?))` carries
/// the saved/loaded `bookmark_state` blob (re-parsed by the panel) and an optional status string;
/// `Err(msg)` the failure.
pub type BookmarkStateCell = Arc<Mutex<Option<Result<(serde_json::Value, Option<String>), String>>>>;

/// The match options the search transport forwards as query params (a copy of the panel's toggles, kept
/// here so backend_client does not depend on the find_in_files module).
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchMatchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
}

/// The page size for the paginated workspace search (the React `SEARCH_PAGE_SIZE`). The backend caps a
/// single page at 500, so requesting 500 and looping until a short page is the find-all contract.
pub const SEARCH_PAGE_SIZE: u32 = 500;

/// REST client for the VERIFIED workspace search + search-bookmark surfaces the MT-029 Find-in-Files
/// panel binds: `GET /loom/graph-search` (paginated) and `GET/PUT /search-bookmarks`.
#[derive(Clone)]
pub struct WorkspaceSearchClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl WorkspaceSearchClient {
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    fn graph_search_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/loom/graph-search", self.base_url, workspace_id)
    }

    fn bookmarks_url(&self, workspace_id: &str) -> String {
        format!("{}/workspaces/{}/search-bookmarks", self.base_url, workspace_id)
    }

    /// Build the query params for ONE search page (the VERIFIED `LoomSearchQueryParams` names; note the
    /// regex param is `regex`, NOT `isRegex`). `source_kind` is omitted for the All filter (AC-4); empty
    /// tag/path filters are omitted. Split out so a unit test asserts the EXACT wire params without a
    /// backend (the spawn path routes through this same builder).
    #[allow(clippy::too_many_arguments)]
    pub fn search_page_query(
        &self,
        query: &str,
        source_kind: Option<&str>,
        tag_filter: &str,
        path_filter: &str,
        opts: SearchMatchOptions,
        offset: u32,
    ) -> Vec<(String, String)> {
        let mut params: Vec<(String, String)> = vec![
            ("q".to_owned(), query.to_owned()),
            ("limit".to_owned(), SEARCH_PAGE_SIZE.to_string()),
            ("offset".to_owned(), offset.to_string()),
        ];
        if let Some(sk) = source_kind {
            params.push(("source_kinds".to_owned(), sk.to_owned()));
        }
        let tags: Vec<&str> = tag_filter.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
        if !tags.is_empty() {
            params.push(("tag_ids".to_owned(), tags.join(",")));
        }
        if !path_filter.trim().is_empty() {
            params.push(("path".to_owned(), path_filter.trim().to_owned()));
        }
        if opts.case_sensitive {
            params.push(("case_sensitive".to_owned(), "true".to_owned()));
        }
        if opts.whole_word {
            params.push(("whole_word".to_owned(), "true".to_owned()));
        }
        if opts.is_regex {
            params.push(("regex".to_owned(), "true".to_owned()));
        }
        params
    }

    /// Run the paginated workspace search off the UI thread, delivering `(all_hits, result_set_key)`
    /// into `cell`. Loops `offset += 500` until a page returns `< 500` hits (the find-all contract), so
    /// a large workspace returns the WHOLE result set (the React pagination — a partial first page would
    /// silently truncate replace-all, RISK-7).
    #[allow(clippy::too_many_arguments)]
    pub fn search_paginated(
        &self,
        workspace_id: &str,
        query: &str,
        source_kind: Option<&str>,
        tag_filter: &str,
        path_filter: &str,
        opts: SearchMatchOptions,
        result_set_key: String,
        cell: GraphSearchCell,
    ) {
        let url = self.graph_search_url(workspace_id);
        let client = self.client.clone();
        let query = query.to_owned();
        let source_kind = source_kind.map(str::to_owned);
        let tag_filter = tag_filter.to_owned();
        let path_filter = path_filter.to_owned();
        let this = self.clone();
        self.runtime.spawn(async move {
            let mut all: Vec<LoomGraphSearchHit> = Vec::new();
            let mut offset = 0u32;
            let result = loop {
                let params = this.search_page_query(
                    &query,
                    source_kind.as_deref(),
                    &tag_filter,
                    &path_filter,
                    opts,
                    offset,
                );
                match get_json(&client, &url, &params).await {
                    Ok(v) => {
                        let page: Vec<LoomGraphSearchHit> = match v.as_array() {
                            Some(arr) => arr
                                .iter()
                                .filter_map(|h| serde_json::from_value(h.clone()).ok())
                                .collect(),
                            None => Vec::new(),
                        };
                        let page_len = page.len();
                        all.extend(page);
                        if page_len < SEARCH_PAGE_SIZE as usize {
                            break Ok((all, result_set_key));
                        }
                        offset += SEARCH_PAGE_SIZE;
                    }
                    Err(e) => break Err(e.to_string()),
                }
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }

    /// Load the saved-search bookmark state off the UI thread, delivering `(bookmark_state_blob, None)`
    /// into `cell`. An absent `bookmark_state` (no bookmarks saved yet) yields an empty blob, never an
    /// error.
    pub fn load_bookmarks(&self, workspace_id: &str, cell: BookmarkStateCell) {
        let url = self.bookmarks_url(workspace_id);
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = get_json(&client, &url, &[])
                .await
                .map(|v| {
                    let blob = v
                        .get("bookmark_state")
                        .filter(|b| !b.is_null())
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!({}));
                    (blob, None)
                })
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }

    /// Build the bookmark-save request (`PUT /search-bookmarks` with `{bookmark_state: <blob>}`). Split
    /// out so a unit test asserts the EXACT wrapper without a backend.
    pub fn save_bookmarks_request(&self, workspace_id: &str, bookmark_state: serde_json::Value) -> RequestSpec {
        RequestSpec {
            method: HttpMethod::Put,
            url: self.bookmarks_url(workspace_id),
            body: Some(serde_json::json!({ "bookmark_state": bookmark_state })),
        }
    }

    /// Save the bookmark state off the UI thread, delivering `(saved_blob, Some(status))` into `cell`.
    /// The saved blob is re-read from the PUT response's `bookmark_state` so the panel renders the
    /// canonical persisted list (not the optimistic local copy).
    pub fn save_bookmarks(
        &self,
        workspace_id: &str,
        bookmark_state: serde_json::Value,
        status: String,
        cell: BookmarkStateCell,
    ) {
        let spec = self.save_bookmarks_request(workspace_id, bookmark_state.clone());
        let body = spec.body.unwrap_or_default();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = put_json(&client, &spec.url, &body)
                .await
                .map(|v| {
                    let blob = v
                        .get("bookmark_state")
                        .filter(|b| !b.is_null())
                        .cloned()
                        .unwrap_or(bookmark_state);
                    (blob, Some(status))
                })
                .map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result);
            }
        });
    }
}

/// `PUT {url}` with a JSON body, returning the parsed response body. A non-success status or parse
/// failure is an [`AppError`]. Mirrors [`post_json`] for the bookmark-save path.
async fn put_json(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let resp = client
        .put(url)
        .timeout(Duration::from_secs(10))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("PUT non-success status {}", resp.status())));
    }
    resp.json().await.map_err(|e| AppError::Parse(e.to_string()))
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-029 rich-document load + save transport for the replace pipeline. Reuses the MT-020 VERIFIED
// `/knowledge/documents/{id}` + `/save` routes + the four required identity headers. The preview +
// apply orchestration (load each doc, walk content_json, save with expected_version, 409-no-overwrite,
// partial-failure receipts) is owned by the find_in_files module; this client provides the raw load +
// save primitives the module's off-thread pipeline calls.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// A loaded rich document's verified fields the replace pipeline consumes.
#[derive(Debug, Clone, PartialEq)]
pub struct RichDocBody {
    pub document_id: String,
    pub doc_version: u64,
    pub title: String,
    pub content_json: serde_json::Value,
    pub crdt_document_id: Option<String>,
}

/// The outcome of one document save: the receipt event id, or a typed conflict / error.
#[derive(Debug, Clone, PartialEq)]
pub enum DocSaveOutcome {
    /// 200: the save committed; carries the `save_receipt_event_id`.
    Saved(String),
    /// 409: the document changed since preview — NOT overwritten (RISK-2 data-loss control).
    Conflict,
    /// A non-409 failure (network / server / schema).
    Failed(String),
}

/// Fold the per-document `(document_id, outcome)` results of the apply pipeline into the typed
/// [`crate::find_in_files::ReplaceDelivery`] (RISK-1/MC-1). PURE so the partial-failure receipt-
/// preservation control is unit-provable without a live backend: every `Saved` receipt seen BEFORE the
/// first `Conflict`/`Failed` is preserved in `AppliedPartial`; an all-`Saved` run yields `Applied`.
/// `plan_count` is the original number of plans (carried for the success status line).
pub fn fold_apply_outcomes(
    outcomes: &[(String, DocSaveOutcome)],
    plan_count: usize,
) -> crate::find_in_files::ReplaceDelivery {
    let mut receipts: Vec<String> = Vec::new();
    let mut failure: Option<String> = None;
    for (document_id, outcome) in outcomes {
        match outcome {
            DocSaveOutcome::Saved(receipt) => receipts.push(receipt.clone()),
            DocSaveOutcome::Conflict => {
                failure = Some(format!(
                    "Document {document_id} changed since preview (version conflict); not overwritten."
                ));
                break;
            }
            DocSaveOutcome::Failed(msg) => {
                failure = Some(format!("Save of {document_id} failed: {msg}"));
                break;
            }
        }
    }
    match failure {
        // RISK-1/MC-1: a partial failure preserves the receipts already collected.
        Some(error) => crate::find_in_files::ReplaceDelivery::AppliedPartial { receipts, error },
        None => crate::find_in_files::ReplaceDelivery::Applied { receipts, plan_count },
    }
}

/// The typed delivery the find_in_files replace pipeline emits. Kept as a `serde_json::Value`-free
/// enum so the module's `ReplaceDelivery` can be built from it without backend_client depending on the
/// module. (The module defines its own `ReplaceDelivery`; this is the transport-level result feeding it
/// via the closure the module passes — see `RichDocClient::preview_replace`/`apply_plans`.)
pub type FindReplaceCell = Arc<Mutex<Option<crate::find_in_files::ReplaceDelivery>>>;

/// REST client for the VERIFIED rich-document load + save routes the MT-029 replace pipeline drives.
#[derive(Clone)]
pub struct RichDocClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
    session_run_id: String,
}

impl RichDocClient {
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            // Share the ONE process-wide pool rather than minting an independent connection pool/TLS
            // stack: load/save now delegate to the consolidated MT-037 client (see `load_document` /
            // `save_document`), so the find/replace pipeline and the editor client share one transport.
            client: shared_http_client(),
            base_url: base_url.into(),
            runtime,
            session_run_id: format!("native-editor-fif-{}", std::process::id()),
        }
    }

    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    /// The consolidated MT-037 client bound to the SAME shared pool + base URL. load/save delegate
    /// through this so there is exactly ONE document load/save wire path with ONE conflict semantic
    /// (the REUSE-NOT-DUPLICATE gate): `RichDocClient` no longer forks its own load/save transport.
    fn consolidated(&self) -> crate::backend::knowledge_documents::KnowledgeDocumentsClient {
        crate::backend::knowledge_documents::KnowledgeDocumentsClient::with_client(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    /// Run the PREVIEW pipeline off the UI thread: for each `document_id`, load the doc, walk its
    /// content_json with `regex`/`replacement`/`opts`, and accumulate a [`crate::find_in_files::
    /// ReplacementPlan`] for every doc with >= 1 match. Delivers a `ReplaceDelivery::Preview{plans,key}`
    /// (or `PreviewError`) into `cell`. A load failure aborts with `PreviewError` (no partial preview).
    #[allow(clippy::too_many_arguments)]
    pub fn preview_replace(
        &self,
        workspace_id: &str,
        document_ids: Vec<String>,
        regex: regex::Regex,
        replacement: String,
        opts: crate::find_in_files::MatchOptions,
        key: String,
        cell: FindReplaceCell,
    ) {
        let _ = workspace_id; // documents are addressed by global KRD- id, not workspace-scoped.
        let this = self.clone();
        self.runtime.spawn(async move {
            let mut plans = Vec::new();
            let mut error: Option<String> = None;
            for document_id in &document_ids {
                match this.load_document(document_id).await {
                    Ok(doc) => {
                        let replaced = crate::find_in_files::replace_in_content(
                            &doc.content_json,
                            &regex,
                            &replacement,
                            opts,
                        );
                        if replaced.count == 0 {
                            continue;
                        }
                        plans.push(crate::find_in_files::ReplacementPlan {
                            document_id: doc.document_id,
                            title: doc.title,
                            expected_version: doc.doc_version,
                            content_json_after: replaced.content,
                            crdt_document_id: doc.crdt_document_id,
                            match_count: replaced.count,
                            before_preview: replaced.before_preview,
                            after_preview: replaced.after_preview,
                            match_previews: replaced.match_previews,
                        });
                    }
                    Err(e) => {
                        error = Some(format!("Replace preview failed: {e}"));
                        break;
                    }
                }
            }
            let delivery = match error {
                Some(msg) => crate::find_in_files::ReplaceDelivery::PreviewError(msg),
                None => crate::find_in_files::ReplaceDelivery::Preview { plans, key },
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(delivery);
            }
        });
    }

    /// Run the APPLY pipeline off the UI thread: save each plan with its captured `expected_version`
    /// (optimistic concurrency). On a 409 or error, STOP but PRESERVE the receipts already collected
    /// (RISK-1/MC-1 — never a silent partial loss): delivers `AppliedPartial{receipts,error}`. On full
    /// success delivers `Applied{receipts,plan_count}`. The fold from per-document outcomes to the typed
    /// delivery lives in the pure [`fold_apply_outcomes`] so MC-1 is unit-provable without a backend.
    pub fn apply_plans(
        &self,
        workspace_id: &str,
        plans: Vec<crate::find_in_files::ReplacementPlan>,
        cell: FindReplaceCell,
    ) {
        let _ = workspace_id;
        let this = self.clone();
        let plan_count = plans.len();
        self.runtime.spawn(async move {
            // Save sequentially, capturing each (document_id, outcome) and STOPPING at the first
            // non-success so a since-edited later doc is never overwritten on a stale plan. The break is
            // realized inside the pure fold below by feeding outcomes only up to (and including) the first
            // failure.
            let mut outcomes: Vec<(String, DocSaveOutcome)> = Vec::with_capacity(plans.len());
            for plan in &plans {
                let outcome = this
                    .save_document(&plan.document_id, &plan.content_json_after, plan.expected_version)
                    .await;
                let is_terminal = !matches!(outcome, DocSaveOutcome::Saved(_));
                outcomes.push((plan.document_id.clone(), outcome));
                if is_terminal {
                    break;
                }
            }
            let delivery = fold_apply_outcomes(&outcomes, plan_count);
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(delivery);
            }
        });
    }

    /// `GET /knowledge/documents/{id}` -> the verified `{document:{..}}` body, narrowed into a
    /// [`RichDocBody`] for the replace pipeline. DELEGATES to the consolidated MT-037 client (ONE wire
    /// load path — the REUSE-NOT-DUPLICATE gate); the rich [`crate::backend::knowledge_documents::
    /// DocumentLoadResponse`] is narrowed here to the four fields the pipeline reads. A non-success
    /// status or parse failure is an [`AppError`].
    pub async fn load_document(&self, document_id: &str) -> Result<RichDocBody, AppError> {
        let headers = crate::backend::knowledge_documents::HskDocumentHeaders::for_operator(
            self.session_run_id.clone(),
            document_id,
        );
        let resp = self
            .consolidated()
            .load_document(&headers, document_id)
            .await
            .map_err(|e| AppError::Http(e.to_string()))?;
        let doc = &resp.document;
        Ok(RichDocBody {
            document_id: doc
                .get("rich_document_id")
                .and_then(|x| x.as_str())
                .unwrap_or(document_id)
                .to_owned(),
            doc_version: doc.get("doc_version").and_then(|x| x.as_u64()).unwrap_or(0),
            title: doc.get("title").and_then(|x| x.as_str()).unwrap_or("").to_owned(),
            content_json: doc
                .get("content_json")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({ "type": "doc", "content": [] })),
            crdt_document_id: doc
                .get("crdt_document_id")
                .and_then(|x| x.as_str())
                .map(str::to_owned),
        })
    }

    /// `PUT /knowledge/documents/{id}/save` with `{expected_version, content_json}` -> the
    /// [`DocSaveOutcome`]. DELEGATES to the consolidated MT-037 client (ONE wire save path with ONE
    /// conflict semantic — the REUSE-NOT-DUPLICATE gate): a 200 returns the `save_receipt_event_id`;
    /// the consolidated [`crate::backend::knowledge_documents::KnowledgeDocumentsError::SaveConflict`]
    /// (409) maps to [`DocSaveOutcome::Conflict`] (NEVER an overwrite — so the find/replace pipeline's
    /// `AppliedPartial` / [`fold_apply_outcomes`] receipt-preservation control is unchanged); any other
    /// failure is [`DocSaveOutcome::Failed`].
    pub async fn save_document(
        &self,
        document_id: &str,
        content_json: &serde_json::Value,
        expected_version: u64,
    ) -> DocSaveOutcome {
        let headers = crate::backend::knowledge_documents::HskDocumentHeaders::for_operator(
            self.session_run_id.clone(),
            document_id,
        );
        let body = crate::backend::knowledge_documents::SaveDocumentRequest {
            // The replace pipeline carries `expected_version` as a u64; the backend optimistic-
            // concurrency token is an i64. Versions are small non-negative monotone counters, so the
            // saturating cast is lossless in practice and never produces a negative token.
            expected_version: i64::try_from(expected_version).unwrap_or(i64::MAX),
            content_json: content_json.clone(),
            crdt_document_id: None,
            crdt_snapshot_id: None,
            promotion_receipt_event_id: None,
        };
        match self.consolidated().save_document(&headers, document_id, &body).await {
            Ok(saved) => DocSaveOutcome::Saved(saved.save_receipt_event_id.unwrap_or_default()),
            Err(crate::backend::knowledge_documents::KnowledgeDocumentsError::SaveConflict { .. }) => {
                DocSaveOutcome::Conflict
            }
            Err(e) => DocSaveOutcome::Failed(e.to_string()),
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-033 (E5 — CKC embeds / drag-in): the atelier read surface the AtelierSidePanel loads.
//
// VERIFIED READ-ONLY against the REAL running backend (`src/backend/handshake_core/src/api/atelier.rs`,
// WP-KERNEL-005), NOT taken from the MT contract body:
//   - GET /atelier/intake/batches                 -> Vec<{batch_id(uuid), source_label, source_ref,
//                                                          mode, status, created_at_utc, ...}>
//   - GET /atelier/intake/batches/{batch_id}/items -> { lane_counts, items:[{item_id(uuid), file_name,
//                                                          source_path, lane, byte_len}] }
//   - GET /atelier/command-corpus                  -> Vec<{entry_id(uuid), action_id, owner,
//                                                          execution_class, foreground_flag,
//                                                          manual_anchor}>
// These three reads are the ONLY atelier endpoints the side panel needs (the contract's two list reads +
// the per-batch items read used to expand a batch into draggable item rows). No backend edit; a gap is a
// typed blocker. Follows the MT-020/021/023/026 off-thread shape: spawn on the app's tokio runtime,
// deliver the parsed projection into an `Arc<Mutex<Option<Result<..>>>>` the egui UI thread drains next
// frame (HBR-QUIET — the render thread is NEVER blocked on the network). Speaks `serde_json::Value` so it
// never depends on the `handshake_core` crate's types.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// One intake batch row the AtelierSidePanel lists (the verified subset of `IntakeBatchResponse`). The
/// `batch_id` is the path arg for the items read; `source_label` is the row label; `status` is a muted
/// chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierBatchRow {
    pub batch_id: String,
    pub source_label: String,
    pub status: String,
}

/// One intake item row inside an expanded batch (the verified subset of `IntakeItemResponse`). The
/// `item_id` is the atelier item id used as the embed `refValue`; `file_name` is the draggable row label;
/// `source_path` is the thumbnail/path hint the contract asks the row to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierItemRow {
    pub item_id: String,
    pub file_name: String,
    pub source_path: String,
    pub lane: String,
}

/// One command-corpus entry row (the verified subset of `CommandCorpusEntryResponse`). `action_id` is the
/// row label; `owner` + `execution_class` are muted detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierCorpusRow {
    pub entry_id: String,
    pub action_id: String,
    pub owner: String,
    pub execution_class: String,
}

/// The externally-meaningful result of one atelier side-panel load: the batches + the command corpus
/// (the two top-level sections). Items are loaded per-batch on demand via [`AtelierClient::fetch_items`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtelierSidePanelData {
    pub batches: Vec<AtelierBatchRow>,
    pub corpus: Vec<AtelierCorpusRow>,
}

/// One-slot delivery cell for the off-thread side-panel load (`Ok(data)` / `Err(msg)`), drained by the
/// egui UI thread next frame.
pub type AtelierSidePanelCell = Arc<Mutex<Option<Result<AtelierSidePanelData, String>>>>;

/// One-slot delivery cell for an off-thread per-batch items load, keyed by the batch id so a stale
/// response for a previously-expanded batch is discardable.
pub type AtelierItemsCell = Arc<Mutex<Option<(String, Result<Vec<AtelierItemRow>, String>)>>>;

/// REST client for the VERIFIED atelier read surface the MT-033 AtelierSidePanel consumes.
#[derive(Clone)]
pub struct AtelierClient {
    client: reqwest::Client,
    base_url: String,
    runtime: tokio::runtime::Handle,
}

impl AtelierClient {
    /// Build a client against `base_url` (e.g. [`BACKEND_BASE_URL`]) bridging onto `runtime`.
    pub fn new(base_url: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, bridging onto the app's runtime handle.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(BACKEND_BASE_URL, runtime)
    }

    /// Pure request builder for `GET /atelier/intake/batches`.
    pub fn batches_request(&self) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/atelier/intake/batches", self.base_url),
            query: vec![],
        }
    }

    /// Pure request builder for `GET /atelier/command-corpus`.
    pub fn corpus_request(&self) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/atelier/command-corpus", self.base_url),
            query: vec![],
        }
    }

    /// Pure request builder for `GET /atelier/intake/batches/{batch_id}/items`.
    pub fn items_request(&self, batch_id: &str) -> GetRequestSpec {
        GetRequestSpec {
            method: HttpMethod::Get,
            url: format!("{}/atelier/intake/batches/{}/items", self.base_url, batch_id),
            query: vec![],
        }
    }

    /// Load the batches + command corpus off the UI thread, delivering the parsed projection into `cell`.
    /// A failure of EITHER read fails the whole load (the panel surfaces the error text, never a blank
    /// half-loaded panel). The two reads run concurrently on the runtime.
    pub fn fetch_side_panel(&self, cell: AtelierSidePanelCell) {
        let batches_url = self.batches_request().url;
        let corpus_url = self.corpus_request().url;
        let client = self.client.clone();
        self.runtime.spawn(async move {
            let result = load_atelier_side_panel(&client, &batches_url, &corpus_url).await;
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(result.map_err(|e| e.to_string()));
            }
        });
    }

    /// Load one batch's items off the UI thread, delivering `(batch_id, Ok(items))` / `(batch_id, Err)`
    /// into `cell`. The host matches the delivered `batch_id` against the currently-expanded batch so a
    /// stale response for a since-collapsed batch is discarded.
    pub fn fetch_items(&self, batch_id: &str, cell: AtelierItemsCell) {
        let url = self.items_request(batch_id).url;
        let client = self.client.clone();
        let id = batch_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_atelier_items(&client, &url).await.map_err(|e| e.to_string());
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((id, result));
            }
        });
    }
}

/// GET the two side-panel reads and assemble the projection. Either read failing fails the whole load.
async fn load_atelier_side_panel(
    client: &reqwest::Client,
    batches_url: &str,
    corpus_url: &str,
) -> Result<AtelierSidePanelData, AppError> {
    let batches = fetch_atelier_batches(client, batches_url).await?;
    let corpus = fetch_atelier_corpus(client, corpus_url).await?;
    Ok(AtelierSidePanelData { batches, corpus })
}

/// `GET {url}` and parse the `Vec<IntakeBatchResponse>` into [`AtelierBatchRow`]s. A row missing a
/// `batch_id` is skipped (defensive — never a panic, never a fabricated id).
async fn fetch_atelier_batches(client: &reqwest::Client, url: &str) -> Result<Vec<AtelierBatchRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let rows = arr
        .iter()
        .filter_map(|row| {
            let batch_id = row.get("batch_id").and_then(|x| x.as_str())?.to_owned();
            if batch_id.is_empty() {
                return None;
            }
            Some(AtelierBatchRow {
                batch_id,
                source_label: row
                    .get("source_label")
                    .and_then(|x| x.as_str())
                    .unwrap_or("(unnamed batch)")
                    .to_owned(),
                status: row.get("status").and_then(|x| x.as_str()).unwrap_or("").to_owned(),
            })
        })
        .collect();
    Ok(rows)
}

/// `GET {url}` and parse the `Vec<CommandCorpusEntryResponse>` into [`AtelierCorpusRow`]s.
async fn fetch_atelier_corpus(client: &reqwest::Client, url: &str) -> Result<Vec<AtelierCorpusRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let rows = arr
        .iter()
        .filter_map(|row| {
            let entry_id = row.get("entry_id").and_then(|x| x.as_str())?.to_owned();
            let action_id = row.get("action_id").and_then(|x| x.as_str()).unwrap_or("").to_owned();
            if entry_id.is_empty() {
                return None;
            }
            Some(AtelierCorpusRow {
                entry_id,
                action_id,
                owner: row.get("owner").and_then(|x| x.as_str()).unwrap_or("").to_owned(),
                execution_class: row
                    .get("execution_class")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_owned(),
            })
        })
        .collect();
    Ok(rows)
}

/// `GET {url}` and parse the `IntakeBatchItemsResponse.items[]` into [`AtelierItemRow`]s.
async fn fetch_atelier_items(client: &reqwest::Client, url: &str) -> Result<Vec<AtelierItemRow>, AppError> {
    let v = get_json(client, url, &[]).await?;
    let items = v.get("items").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    let rows = items
        .iter()
        .filter_map(|row| {
            let item_id = row.get("item_id").and_then(|x| x.as_str())?.to_owned();
            if item_id.is_empty() {
                return None;
            }
            Some(AtelierItemRow {
                item_id,
                file_name: row
                    .get("file_name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("(unnamed item)")
                    .to_owned(),
                source_path: row.get("source_path").and_then(|x| x.as_str()).unwrap_or("").to_owned(),
                lane: row.get("lane").and_then(|x| x.as_str()).unwrap_or("").to_owned(),
            })
        })
        .collect();
    Ok(rows)
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// MT-021 hardening tests (MAJOR #1/#2/#3): prove every menu-action backend call constructs the EXACT
// verified URL + JSON body. Two layers:
//   1. Pure request-builder assertions (`*_request`) — deterministic, no port flakiness. Because the
//      real spawn methods (`stage_paths`, `set_z_index`, `set_flag`, …) route through these SAME
//      builders, asserting the builder asserts the production request construction.
//   2. A live in-process HTTP CAPTURE server (std::net::TcpListener, no new deps) that the REAL spawn
//      path of one representative write op (stage) actually sends to — proving the client is genuinely
//      CONSUMED end-to-end (the dispatch -> client -> reqwest -> wire path is real, not just arithmetic).
// ═════════════════════════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    /// A current-thread runtime whose handle the clients bridge onto. Kept alive for the test scope.
    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build current-thread runtime")
    }

    const BASE: &str = "http://test.local:1234";

    // ── SourceControlClient: stage / unstage / discard / diff / blame ────────────────────────────────

    #[test]
    fn scm_stage_request_url_and_body() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        let spec = c.stage_request(ScmWriteOp::Stage, "/repo", "src/x.rs");
        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(spec.url, "http://test.local:1234/source-control/stage");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({ "repo_path": "/repo", "paths": ["src/x.rs"] })
        );
    }

    #[test]
    fn scm_unstage_request_uses_unstage_segment() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        let spec = c.stage_request(ScmWriteOp::Unstage, "/repo", "src/x.rs");
        assert_eq!(spec.url, "http://test.local:1234/source-control/unstage");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({ "repo_path": "/repo", "paths": ["src/x.rs"] })
        );
    }

    #[test]
    fn scm_discard_request_carries_confirmed_flag() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        // confirmed:false is the V1 stub default — a safe 409 no-op, never a destructive discard.
        let spec = c.discard_request("/repo", "src/x.rs", false);
        assert_eq!(spec.url, "http://test.local:1234/source-control/discard");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({ "repo_path": "/repo", "paths": ["src/x.rs"], "confirmed": false })
        );
    }

    #[test]
    fn scm_diff_request_query_carries_scope() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        let worktree = c.diff_request("/repo", "src/x.rs", ScmDiffScope::Worktree);
        assert_eq!(worktree.method, HttpMethod::Get);
        assert_eq!(worktree.url, "http://test.local:1234/source-control/diff");
        assert_eq!(
            worktree.query,
            vec![
                ("repo_path".to_owned(), "/repo".to_owned()),
                ("path".to_owned(), "src/x.rs".to_owned()),
                ("scope".to_owned(), "worktree".to_owned()),
            ]
        );
        let staged = c.diff_request("/repo", "src/x.rs", ScmDiffScope::Staged);
        assert_eq!(staged.query.last().unwrap().1, "staged");
    }

    #[test]
    fn scm_blame_request_url_and_query() {
        let rt = rt();
        let c = SourceControlClient::new(BASE, rt.handle().clone());
        let spec = c.blame_request("/repo", "src/x.rs");
        assert_eq!(spec.url, "http://test.local:1234/source-control/blame");
        assert_eq!(
            spec.query,
            vec![
                ("repo_path".to_owned(), "/repo".to_owned()),
                ("path".to_owned(), "src/x.rs".to_owned()),
            ]
        );
    }

    // ── CanvasClient: set_z_index (front/back) / remove placement / remove visual edge ───────────────

    #[test]
    fn canvas_set_z_index_request_url_and_body() {
        let rt = rt();
        let c = CanvasClient::new(BASE, rt.handle().clone());
        let spec = c.set_z_index_request("ws1", "p9", 1_000_000);
        assert_eq!(spec.method, HttpMethod::Patch);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/canvas-placements/p9");
        assert_eq!(spec.body.unwrap(), serde_json::json!({ "z_index": 1_000_000 }));
    }

    #[test]
    fn canvas_remove_placement_request_is_delete_no_body() {
        let rt = rt();
        let c = CanvasClient::new(BASE, rt.handle().clone());
        let spec = c.remove_placement_request("ws1", "p9");
        assert_eq!(spec.method, HttpMethod::Delete);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/canvas-placements/p9");
        assert!(spec.body.is_none());
    }

    #[test]
    fn canvas_remove_visual_edge_request_targets_visual_edge_endpoint() {
        let rt = rt();
        let c = CanvasClient::new(BASE, rt.handle().clone());
        let spec = c.remove_visual_edge_request("ws1", "ve7");
        assert_eq!(spec.method, HttpMethod::Delete);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/canvas-visual-edges/ve7");
        assert!(spec.body.is_none());
    }

    // ── LoomBlockClient: set_flag (AC#73) + rename ───────────────────────────────────────────────────

    #[test]
    fn loom_set_flag_pinned_body_contains_pinned() {
        let rt = rt();
        let c = LoomBlockClient::new(BASE, rt.handle().clone());
        let spec = c.set_flag_request("ws1", "b3", LoomBlockFlag::Pinned, true);
        assert_eq!(spec.method, HttpMethod::Patch);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/blocks/b3");
        let body = spec.body.unwrap();
        // AC#73: the serialized body contains the `pinned` flag, and ONLY that flag (not favorite).
        assert_eq!(body, serde_json::json!({ "pinned": true }));
        assert!(body.get("favorite").is_none());
    }

    #[test]
    fn loom_set_flag_favorite_body_contains_favorite() {
        let rt = rt();
        let c = LoomBlockClient::new(BASE, rt.handle().clone());
        let spec = c.set_flag_request("ws1", "b3", LoomBlockFlag::Favorite, false);
        let body = spec.body.unwrap();
        assert_eq!(body, serde_json::json!({ "favorite": false }));
        assert!(body.get("pinned").is_none());
    }

    #[test]
    fn loom_rename_request_body_contains_title() {
        let rt = rt();
        let c = LoomBlockClient::new(BASE, rt.handle().clone());
        let spec = c.rename_request("ws1", "b3", "New Title");
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/blocks/b3");
        assert_eq!(spec.body.unwrap(), serde_json::json!({ "title": "New Title" }));
    }

    // ── MT-023 DrawerDataClient: verified view-count + daily-journal requests ────────────────────────

    #[test]
    fn drawer_notes_count_request_targets_views_all_with_note_content_type() {
        let rt = rt();
        let c = DrawerDataClient::new(BASE, rt.handle().clone());
        let spec = c.count_request("ws1", DrawerDataKind::Notes);
        assert_eq!(spec.method, HttpMethod::Get);
        // VERIFIED endpoint: /loom/views/all (NOT the contract's stale /loom/views/table).
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/views/all");
        // VERIFIED content_type: note (the contract's `list` does not exist as a content_type).
        assert_eq!(spec.query, vec![("content_type".to_owned(), "note".to_owned())]);
    }

    #[test]
    fn drawer_lists_count_request_maps_to_view_def_content_type() {
        let rt = rt();
        let c = DrawerDataClient::new(BASE, rt.handle().clone());
        let spec = c.count_request("ws1", DrawerDataKind::Lists);
        // The contract's "Lists" maps to saved block-collection views → content_type=view_def (the
        // real, countable surface; `list` is not a valid LoomBlockContentType — disclosed deviation).
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/views/all");
        assert_eq!(spec.query, vec![("content_type".to_owned(), "view_def".to_owned())]);
    }

    #[test]
    fn drawer_agenda_request_is_put_to_daily_journal() {
        let rt = rt();
        let c = DrawerDataClient::new(BASE, rt.handle().clone());
        let spec = c.journal_request("ws1", "2026-06-20");
        // VERIFIED endpoint: PUT /loom/journals/{date} (open_daily_journal, get-or-create, no body).
        assert_eq!(spec.method, HttpMethod::Put);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/journals/2026-06-20");
        assert!(spec.body.is_none());
    }

    #[test]
    #[should_panic(expected = "count_request requires a content_type")]
    fn drawer_count_request_rejects_agenda_kind() {
        let rt = rt();
        let c = DrawerDataClient::new(BASE, rt.handle().clone());
        // Agenda has no content_type (it uses the journal PUT); building a count request for it is a
        // programmer error, caught loudly rather than silently sending a malformed query.
        let _ = c.count_request("ws1", DrawerDataKind::Agenda);
    }

    // ── End-to-end live capture: the REAL spawn path sends the real request on the wire ─────────────

    /// Captured raw HTTP request: the request line (`METHOD path HTTP/1.1`) + the body after the blank
    /// line. Proves the client genuinely CONSTRUCTED and SENT the request (not just built a spec).
    struct Captured {
        request_line: String,
        body: String,
    }

    /// Bind an ephemeral localhost port, accept ONE connection, read the request, reply `200 {}`, and
    /// return the captured request line + body. No new deps — raw std::net + a tiny manual HTTP read.
    fn capture_one_request(listener: std::net::TcpListener) -> Captured {
        use std::io::{Read, Write};
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut buf = [0u8; 8192];
        let mut data = Vec::new();
        // Read until we have headers + the (small) JSON body. One read is enough for these tiny bodies,
        // but loop until a blank line is seen and the declared Content-Length is satisfied.
        loop {
            let n = stream.read(&mut buf).expect("read request");
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&data);
            if let Some(hdr_end) = text.find("\r\n\r\n") {
                let header = &text[..hdr_end];
                let body_so_far = &text[hdr_end + 4..];
                let content_len = header
                    .lines()
                    .find_map(|l| {
                        let l = l.to_ascii_lowercase();
                        l.strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().ok())
                    })
                    .flatten()
                    .unwrap_or(0);
                if body_so_far.len() >= content_len {
                    break;
                }
            }
        }
        let text = String::from_utf8_lossy(&data).into_owned();
        let request_line = text.lines().next().unwrap_or("").to_owned();
        let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_owned();
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
        let _ = stream.flush();
        Captured { request_line, body }
    }

    #[test]
    fn scm_stage_spawn_sends_real_request_on_the_wire() {
        // Real multi-thread runtime so the spawned task actually runs while the test thread blocks on
        // the capture server (the production off-thread path).
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = SourceControlClient::new(base, rt.handle().clone());
        let cell: ScmReceiptCell = Arc::new(Mutex::new(None));
        // Drive the REAL spawn path (the same call apply_source_control_event makes).
        client.stage_paths(ScmWriteOp::Stage, "/repo", "src/x.rs", cell.clone());

        // Capture the request the spawned task sends on the wire.
        let captured = capture_one_request(listener);
        assert_eq!(captured.request_line, "POST /source-control/stage HTTP/1.1");
        let body: serde_json::Value = serde_json::from_str(captured.body.trim()).expect("json body");
        assert_eq!(body, serde_json::json!({ "repo_path": "/repo", "paths": ["src/x.rs"] }));

        // The delivery cell receives Ok(()) after the 200 — proving the full round-trip is consumed.
        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivered = cell.lock().unwrap().take();
        assert_eq!(delivered, Some(Ok(())), "stage round-trip delivered Ok(())");
    }

    /// Accept ONE connection, read the request, reply `200` with `reply_body`, and return the captured
    /// request line. Variant of [`capture_one_request`] that lets the test control the response body so
    /// the client's parse path (e.g. counting `blocks`) is proven end-to-end, not just the request line.
    fn capture_one_request_reply(listener: std::net::TcpListener, reply_body: &str) -> String {
        use std::io::{Read, Write};
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut buf = [0u8; 8192];
        let mut data = Vec::new();
        loop {
            let n = stream.read(&mut buf).expect("read request");
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&data);
            if let Some(hdr_end) = text.find("\r\n\r\n") {
                let header = &text[..hdr_end];
                let body_so_far = &text[hdr_end + 4..];
                let content_len = header
                    .lines()
                    .find_map(|l| {
                        let l = l.to_ascii_lowercase();
                        l.strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().ok())
                    })
                    .flatten()
                    .unwrap_or(0);
                if body_so_far.len() >= content_len {
                    break;
                }
            }
        }
        let text = String::from_utf8_lossy(&data).into_owned();
        let request_line = text.lines().next().unwrap_or("").to_owned();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            reply_body.len(),
            reply_body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        request_line
    }

    #[test]
    fn drawer_count_spawn_sends_real_get_and_parses_blocks_len() {
        // The REAL fetch_count spawn path: it must GET /loom/views/all?content_type=note on the wire,
        // parse `blocks.len()` from the verified LoomViewResponse::All shape, and deliver the count. This
        // proves the client is genuinely CONSUMED end-to-end (dispatch → spawn → reqwest → parse → cell),
        // not just that the request builder is correct.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = DrawerDataClient::new(base, rt.handle().clone());
        let cell: DrawerDataCell = Arc::new(Mutex::new(None));
        client.fetch_count("ws1", DrawerDataKind::Notes, cell.clone());

        // The verified response shape: two note blocks → badge_count 2.
        let reply = r#"{"view_type":"all","blocks":[{"block_id":"b1"},{"block_id":"b2"}]}"#;
        let request_line = capture_one_request_reply(listener, reply);
        assert_eq!(
            request_line, "GET /workspaces/ws1/loom/views/all?content_type=note HTTP/1.1",
            "REAL spawn path hits the VERIFIED /loom/views/all endpoint with content_type=note"
        );

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivered = cell.lock().unwrap().take().expect("drawer count delivered");
        assert_eq!(
            delivered,
            (DrawerDataKind::Notes, Ok(DrawerCardData { badge_count: 2, subtitle: "2 items".to_owned() })),
            "blocks.len() parsed as the badge count from the verified response shape"
        );
    }

    // ── MT-024 DrawerActionClient: verified pin / discard / stow / attach-evidence requests ──────────

    #[test]
    fn drawer_action_pin_request_uses_pin_order_field_not_ordinal() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.pin_order_request("ws1", "b3", 0);
        assert_eq!(spec.method, HttpMethod::Put);
        // VERIFIED endpoint: /pin-order (MT-183 set_loom_block_pin_order).
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/blocks/b3/pin-order");
        // VERIFIED body field: pin_order (NOT the contract's `ordinal`).
        assert_eq!(spec.body.unwrap(), serde_json::json!({ "pin_order": 0 }));
    }

    #[test]
    fn drawer_action_discard_request_is_delete_no_body() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.discard_request("ws1", "b3");
        assert_eq!(spec.method, HttpMethod::Delete);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/blocks/b3");
        assert!(spec.body.is_none(), "DELETE carries no body");
    }

    #[test]
    fn drawer_action_stow_request_posts_a_tag_edge_to_the_stash_hub() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.stow_request("ws1", "b3");
        assert_eq!(spec.method, HttpMethod::Post);
        // VERIFIED endpoint: /loom/edges (the contract's metadata/content_type PATCH does not exist).
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws1/loom/edges");
        assert_eq!(
            spec.body.unwrap(),
            serde_json::json!({
                "source_block_id": "b3",
                "target_block_id": STASH_TAG_HUB_BLOCK_ID,
                "edge_type": "tag",
                "created_by": "user",
                "target_title": STASH_TAG_TITLE,
            })
        );
    }

    #[test]
    fn drawer_action_attach_evidence_request_uses_valid_enums_and_carries_block_id() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.attach_evidence_request("ws1", "b3", "My Note", Some("job-9"));
        assert_eq!(spec.method, HttpMethod::Post);
        assert_eq!(spec.url, "http://test.local:1234/diagnostics");
        let body = spec.body.unwrap();
        assert_eq!(body["title"], serde_json::json!("Evidence: My Note"));
        // VERIFIED enums: source/surface "user"/"drawer" do not exist; "system" is the honest valid value.
        assert_eq!(body["source"], serde_json::json!("system"));
        assert_eq!(body["surface"], serde_json::json!("system"));
        assert_eq!(body["severity"], serde_json::json!("info"));
        assert_eq!(body["job_id"], serde_json::json!("job-9"));
        // The stashed block id is carried in the VERIFIED evidence_refs.artifact_hashes map.
        assert_eq!(body["evidence_refs"]["artifact_hashes"]["b3"], serde_json::json!("b3"));
    }

    #[test]
    fn drawer_action_attach_evidence_omits_job_id_when_none() {
        let rt = rt();
        let c = DrawerActionClient::new(BASE, rt.handle().clone());
        let spec = c.attach_evidence_request("ws1", "b3", "My Note", None);
        let body = spec.body.unwrap();
        assert!(body.get("job_id").is_none(), "no job_id key when there is no active job");
    }

    #[test]
    fn drawer_action_discard_spawn_sends_real_delete_on_the_wire() {
        // The REAL discard spawn path: it must DELETE /workspaces/ws1/loom/blocks/b3 on the wire and
        // deliver Ok(()) after the 200. Proves the client is genuinely CONSUMED end-to-end (the
        // dispatch -> spawn -> reqwest -> wire -> cell path is real), the MT-021 capture pattern.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = DrawerActionClient::new(base, rt.handle().clone());
        let cell: DrawerActionCell = Arc::new(Mutex::new(None));
        client.discard("ws1", "b3", cell.clone());

        let captured = capture_one_request(listener);
        assert_eq!(captured.request_line, "DELETE /workspaces/ws1/loom/blocks/b3 HTTP/1.1");

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        assert_eq!(cell.lock().unwrap().take(), Some(Ok(())), "discard round-trip delivered Ok(())");
    }

    #[test]
    fn drawer_action_stow_spawn_sends_real_tag_edge_post_on_the_wire() {
        // The REAL stow spawn path: POST /workspaces/ws1/loom/edges with the verified tag-edge body.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = DrawerActionClient::new(base, rt.handle().clone());
        let cell: DrawerActionCell = Arc::new(Mutex::new(None));
        client.stow("ws1", "b3", cell.clone());

        let captured = capture_one_request(listener);
        assert_eq!(captured.request_line, "POST /workspaces/ws1/loom/edges HTTP/1.1");
        let body: serde_json::Value = serde_json::from_str(captured.body.trim()).expect("json body");
        assert_eq!(body["source_block_id"], serde_json::json!("b3"));
        assert_eq!(body["edge_type"], serde_json::json!("tag"));
        assert_eq!(body["target_block_id"], serde_json::json!(STASH_TAG_HUB_BLOCK_ID));

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        assert_eq!(cell.lock().unwrap().take(), Some(Ok(())), "stow round-trip delivered Ok(())");
    }

    #[test]
    fn drawer_count_missing_blocks_field_defaults_to_zero() {
        // CONTROL-023-D: a response that omits `blocks` (or has it null) must default the count to 0
        // without erroring the card. Proven through the REAL spawn + parse path.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = DrawerDataClient::new(base, rt.handle().clone());
        let cell: DrawerDataCell = Arc::new(Mutex::new(None));
        client.fetch_count("ws1", DrawerDataKind::Lists, cell.clone());

        // A response with NO blocks field (the red-team "API omits the field" case).
        let _ = capture_one_request_reply(listener, r#"{"view_type":"all"}"#);

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivered = cell.lock().unwrap().take().expect("drawer count delivered");
        assert_eq!(
            delivered,
            (DrawerDataKind::Lists, Ok(DrawerCardData { badge_count: 0, subtitle: "0 items".to_owned() })),
            "missing blocks field defaults to 0 (CONTROL-023-D), never an error"
        );
    }

    // ── MT-021 LoomGraphClient: verified URL/query builders + a live round-trip parse ────────────────

    #[test]
    fn loom_graph_global_request_url() {
        let rt = rt();
        let c = LoomGraphClient::new(BASE, rt.handle().clone());
        let spec = c.global_request("ws-7");
        assert_eq!(spec.method, HttpMethod::Get);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws-7/loom/views/all");
        assert!(spec.query.is_empty(), "global enumeration carries no query");
    }

    #[test]
    fn loom_graph_local_request_url_and_query() {
        let rt = rt();
        let c = LoomGraphClient::new(BASE, rt.handle().clone());
        let spec = c.local_request("ws-7", "My Note");
        assert_eq!(spec.method, HttpMethod::Get);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws-7/loom/graph-search");
        // The focused block's TITLE is the graph-search `q` (the backend 400s on an empty q), plus the
        // verified backlink_depth + limit caps.
        assert_eq!(
            spec.query,
            vec![
                ("q".to_owned(), "My Note".to_owned()),
                ("backlink_depth".to_owned(), "2".to_owned()),
                ("limit".to_owned(), "200".to_owned()),
            ]
        );
    }

    /// WP-KERNEL-012 MT-080 (AC-080-3 / MT-060): the depth-parameterized builder carries the NEW
    /// `backlink_depth` (the re-query the host fires on `GraphEvent::DepthChanged`) on the SAME verified
    /// endpoint, and clamps an out-of-range depth into `[MIN..=MAX]_BACKLINK_DEPTH` (RISK-080-3).
    #[test]
    fn loom_graph_local_request_with_depth_carries_and_clamps_backlink_depth() {
        let rt = rt();
        let c = LoomGraphClient::new(BASE, rt.handle().clone());

        // A valid in-range depth is carried verbatim on the SAME graph-search URL (NO new endpoint).
        let spec = c.local_request_with_depth("ws-7", "My Note", 4);
        assert_eq!(spec.url, "http://test.local:1234/workspaces/ws-7/loom/graph-search");
        assert_eq!(
            spec.query,
            vec![
                ("q".to_owned(), "My Note".to_owned()),
                ("backlink_depth".to_owned(), "4".to_owned()),
                ("limit".to_owned(), "200".to_owned()),
            ],
            "the new depth replaces the default backlink_depth on the verified endpoint"
        );

        // An abusive over-range depth clamps DOWN to MAX (never reaches the backend as an abusive
        // traversal); a zero/under-range depth clamps UP to MIN.
        let too_deep = c.local_request_with_depth("ws-7", "T", 99);
        assert_eq!(too_deep.query[1], ("backlink_depth".to_owned(), MAX_BACKLINK_DEPTH.to_string()));
        let too_shallow = c.local_request_with_depth("ws-7", "T", 0);
        assert_eq!(too_shallow.query[1], ("backlink_depth".to_owned(), MIN_BACKLINK_DEPTH.to_string()));

        // The non-depth `local_request` still equals the default-depth path (one builder, no drift).
        assert_eq!(c.local_request("ws-7", "X").query, c.local_request_with_depth("ws-7", "X", DEFAULT_BACKLINK_DEPTH).query);
    }

    /// WP-KERNEL-012 MT-080 (AC-080-2 / MT-061): the canvas resize + clear-group request builders PATCH the
    /// SAME verified placement URL the `group_request` uses; only the body differs (`{w,h}` for a resize,
    /// `{clear_group: true}` for a clear). The clear body is asserted against the REAL backend's accepted
    /// contract (`UpdatePlacementRequest.clear_group` in `src/backend/handshake_core/src/api/loom.rs`),
    /// NOT the serializer's own historical output: `{"group_id": null}` is a verified backend no-op (it
    /// deserializes to `group_id: None` and leaves the group unchanged), so only `{"clear_group": true}`
    /// actually clears the section assignment end-to-end.
    #[test]
    fn canvas_board_resize_and_clear_group_requests() {
        let rt = rt();
        let c = CanvasBoardClient::new(BASE, rt.handle().clone());

        let resize = c.resize_request("ws-7", "p-9", 320.0, 180.0);
        assert_eq!(resize.method, HttpMethod::Patch);
        assert_eq!(resize.url, "http://test.local:1234/workspaces/ws-7/loom/canvas-placements/p-9");
        assert_eq!(resize.body, Some(serde_json::json!({ "w": 320.0, "h": 180.0 })));

        let clear = c.clear_group_request("ws-7", "p-9");
        assert_eq!(clear.method, HttpMethod::Patch);
        assert_eq!(clear.url, "http://test.local:1234/workspaces/ws-7/loom/canvas-placements/p-9");
        // The backend clears the group ONLY on `clear_group: true`; `{"group_id": null}` is a no-op.
        assert_eq!(clear.body, Some(serde_json::json!({ "clear_group": true })));

        // The assign (Some group) arm reuses the existing verified group_request (same URL + verb).
        let assign = c.group_request("ws-7", "p-9", "section-2");
        assert_eq!(assign.url, resize.url, "assign-section reuses the same placement PATCH URL");
        assert_eq!(assign.body, Some(serde_json::json!({ "group_id": "section-2" })));
    }

    /// End-to-end: the REAL `fetch_global` spawn path hits a live capture server and parses the verified
    /// `LoomViewResponse::All { blocks }` payload into graph nodes (proves the client is genuinely
    /// CONSUMED — dispatch -> reqwest -> wire -> parse — not just arithmetic). 3 seeded blocks => 3 nodes,
    /// content_type preserved.
    #[test]
    fn loom_graph_global_fetch_parses_blocks() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = LoomGraphClient::new(base, rt.handle().clone());
        let cell: LoomGraphCell = Arc::new(Mutex::new(None));
        client.fetch_global("ws1", cell.clone());

        let body = r#"{"view_type":"all","blocks":[
            {"block_id":"b1","title":"Alpha","content_type":"note"},
            {"block_id":"b2","title":"Beta","content_type":"file"},
            {"block_id":"b3","title":null,"content_type":"tag_hub"}
        ]}"#;
        let request_line = capture_one_request_reply(listener, body);
        assert!(
            request_line.contains("GET /workspaces/ws1/loom/views/all"),
            "global fetch hits views/all (got '{request_line}')"
        );

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let data = cell.lock().unwrap().take().expect("graph delivered").expect("parse ok");
        assert_eq!(data.nodes.len(), 3, "3 seeded blocks -> 3 nodes");
        assert_eq!(data.nodes[0].block_id, "b1");
        assert_eq!(data.nodes[0].content_type, "note");
        // A null title falls back to the block id (never label-less).
        assert_eq!(data.nodes[2].title, "b3", "null title falls back to block_id");
        assert!(data.edges.is_empty(), "global enumeration has no edges");
    }

    /// AC8 binding: a backend 5xx (non-success status) delivers Err, NOT a panic and NOT a fake graph.
    #[test]
    fn loom_graph_global_fetch_error_on_5xx() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build multi-thread runtime");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");

        let client = LoomGraphClient::new(base, rt.handle().clone());
        let cell: LoomGraphCell = Arc::new(Mutex::new(None));
        client.fetch_global("ws1", cell.clone());

        // Reply 503 (backend unreachable analog).
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let resp = "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        });

        rt.block_on(async {
            for _ in 0..50 {
                if cell.lock().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });
        let delivered = cell.lock().unwrap().take().expect("graph delivered");
        assert!(delivered.is_err(), "AC8: a 5xx must deliver Err (got {delivered:?}), never a fake graph");
    }

    // ── LoomTagClient: add-tag candidate parser (verified /loom/search response shape) ────────────────

    /// AC6 / Spec-Realism Gate: the add-tag candidate parser MUST read the VERIFIED `/loom/search` shape
    /// `Vec<LoomBlockSearchResult>` = `[{ "block": { "block_id", "title" }, "score" }]` — `block_id`/`title`
    /// are nested UNDER `block`, not at the entry top level. This feeds the exact wrapper shape and asserts
    /// a candidate is produced; a top-level-only read (the prior bug) would yield ZERO candidates and an
    /// always-empty add-tag popup against the real backend. This exercises the parse directly (NOT the
    /// widget's own `AddTagCandidate::new`, which the existing PROOF5 test injects).
    #[test]
    fn add_tag_candidates_parse_verified_search_result_shape() {
        // The verified LoomBlockSearchResult wrapper: { block: {...}, score }.
        let response = serde_json::json!([
            { "block": { "block_id": "blk-1", "title": "Rust notes" }, "score": 0.9 },
            { "block": { "block_id": "blk-2", "title": "" }, "score": 0.4 },
        ]);
        let candidates = parse_add_tag_candidates(&response);
        assert_eq!(
            candidates.len(),
            2,
            "verified [{{block,score}}] shape must yield one candidate per entry (got {candidates:?})"
        );
        assert_eq!(candidates[0].block_id, "blk-1");
        assert_eq!(candidates[0].title, "Rust notes");
        // Empty title falls back to the block id (never a fabricated label).
        assert_eq!(candidates[1].block_id, "blk-2");
        assert_eq!(candidates[1].title, "blk-2");
    }

    /// The bare-block fallback (defensive, for other search-shaped routes) still reads a top-level
    /// `block_id`/`title` when there is no `block` wrapper, so the parser handles both shapes.
    #[test]
    fn add_tag_candidates_parse_bare_block_fallback() {
        let bare = serde_json::json!([{ "block_id": "b9", "title": "Bare" }]);
        let candidates = parse_add_tag_candidates(&bare);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].block_id, "b9");
        assert_eq!(candidates[0].title, "Bare");

        // An object wrapper exposing a `results` array is also unwrapped.
        let wrapped = serde_json::json!({
            "results": [{ "block": { "block_id": "b10", "title": "Wrapped" }, "score": 1.0 }]
        });
        let wrapped_candidates = parse_add_tag_candidates(&wrapped);
        assert_eq!(wrapped_candidates.len(), 1);
        assert_eq!(wrapped_candidates[0].block_id, "b10");
        assert_eq!(wrapped_candidates[0].title, "Wrapped");

        // Empty array => no candidates, never an error.
        assert!(parse_add_tag_candidates(&serde_json::json!([])).is_empty());
    }

    // ── Apply pipeline partial-failure fold (RISK-1/MC-1) ─────────────────────────────────────────────

    /// MC-1: a SECOND document save returning 409 (Conflict) must PRESERVE the first document's receipt
    /// and STOP — `AppliedPartial{receipts:[r1], ..}`, never a silent loss of the first receipt. This is
    /// the red_team-named control that previously had no standalone (un-ignored) coverage.
    #[test]
    fn apply_fold_preserves_first_receipt_on_second_doc_conflict() {
        let outcomes = vec![
            ("KRD-1".to_owned(), DocSaveOutcome::Saved("evt-1".to_owned())),
            ("KRD-2".to_owned(), DocSaveOutcome::Conflict),
        ];
        match fold_apply_outcomes(&outcomes, 2) {
            crate::find_in_files::ReplaceDelivery::AppliedPartial { receipts, error } => {
                assert_eq!(receipts, vec!["evt-1".to_owned()], "first receipt must survive the conflict");
                assert!(error.contains("KRD-2"), "error names the conflicting doc: {error}");
                assert!(error.contains("conflict"), "error states the version conflict: {error}");
            }
            other => panic!("expected AppliedPartial preserving the first receipt, got {other:?}"),
        }
    }

    /// MC-1 (Failed variant): a non-409 failure on the second doc also preserves the first receipt.
    #[test]
    fn apply_fold_preserves_first_receipt_on_second_doc_failure() {
        let outcomes = vec![
            ("KRD-1".to_owned(), DocSaveOutcome::Saved("evt-1".to_owned())),
            ("KRD-2".to_owned(), DocSaveOutcome::Failed("status 500".to_owned())),
        ];
        match fold_apply_outcomes(&outcomes, 2) {
            crate::find_in_files::ReplaceDelivery::AppliedPartial { receipts, error } => {
                assert_eq!(receipts, vec!["evt-1".to_owned()]);
                assert!(error.contains("status 500"), "error carries the failure detail: {error}");
            }
            other => panic!("expected AppliedPartial, got {other:?}"),
        }
    }

    /// An all-success run folds to `Applied{receipts, plan_count}` with every receipt.
    #[test]
    fn apply_fold_all_success_yields_applied_with_all_receipts() {
        let outcomes = vec![
            ("KRD-1".to_owned(), DocSaveOutcome::Saved("evt-1".to_owned())),
            ("KRD-2".to_owned(), DocSaveOutcome::Saved("evt-2".to_owned())),
        ];
        match fold_apply_outcomes(&outcomes, 2) {
            crate::find_in_files::ReplaceDelivery::Applied { receipts, plan_count } => {
                assert_eq!(receipts, vec!["evt-1".to_owned(), "evt-2".to_owned()]);
                assert_eq!(plan_count, 2);
            }
            other => panic!("expected Applied, got {other:?}"),
        }
    }
}
