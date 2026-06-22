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
        GetRequestSpec {
            method: HttpMethod::Get,
            url: self.graph_search_url(workspace_id),
            query: vec![
                ("q".to_owned(), title.to_owned()),
                ("backlink_depth".to_owned(), "2".to_owned()),
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
        let spec = self.local_request(workspace_id, focus_title);
        let client = self.client.clone();
        let focus = focus_block_id.to_owned();
        self.runtime.spawn(async move {
            let result = fetch_local_graph(&client, &spec.url, &spec.query, &focus).await;
            deliver_graph(&cell, result.map_err(|e| e.to_string()));
        });
    }
}

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
}
