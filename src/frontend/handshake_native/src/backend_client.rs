//! Backend HTTP client. Reuses the EXISTING handshake_core backend over its real HTTP API
//! (GET /health, GET/PUT /workspaces/:id/workbench/layout) — the native app never starts or embeds
//! the backend; it assumes it is running. Deserializes via serde_json::Value to avoid a build
//! dependency on the handshake_core crate.

use crate::error::AppError;
use crate::layout_persistence::{LayoutError, LayoutTransport};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// handshake_core listens here (hardcoded in handshake_core/src/main.rs).
pub const BACKEND_BASE_URL: &str = "http://127.0.0.1:37501";
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
}
