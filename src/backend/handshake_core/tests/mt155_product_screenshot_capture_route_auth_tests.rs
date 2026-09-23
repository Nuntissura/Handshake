#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! WP-KERNEL-012 MT-155: `POST /kernel/product_screenshot_capture/execute` authorization and
//! process-safety proofs against the real `api::kernel` router (loopback, quiet) and the real
//! embedded account-session store.
//!
//! Spec basis: 02-system-architecture:2758 "Authorization is deny-by-default and evaluated on every
//! executable backend boundary." and :2776 "Application authorization remains required for action
//! semantics and capability checks".
//!
//! "Spawns nothing" is proven by sentinels, never by status code alone: the server-side adapter hook
//! points at a node script that writes a marker file under this test's TMP sandbox, a caller-supplied
//! script would write another marker, and the adapter-output directory is only created after the
//! `node --version` pre-flight. A denied request must leave all three absent.
//!
//! Requires `node` on PATH (the authorized and timeout proofs run a real child process).

#[path = "knowledge_ingestion_support/mod.rs"]
mod embedded_knowledge_support;

#[path = "account_session_support/mod.rs"]
mod account_session_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use account_session_support::{
    AccountFixture, OwnerSession, CHANNEL_BINDING_TOKEN_HEADER, SESSION_TOKEN_HEADER,
};
use async_trait::async_trait;
use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::product_screenshot_capture::{
    set_product_screenshot_route_test_hooks, ProductScreenshotRouteTestHooks,
    PRODUCT_SCREENSHOT_CAPTURE_EXECUTE_CAPABILITY,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};

const ROUTE: &str = "/kernel/product_screenshot_capture/execute";

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }
    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }
    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }
    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
    async fn get_diagnostic(
        &self,
        _id: uuid::Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }
    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }
    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn test_state(store: &EmbeddedKnowledgeStore) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: Arc::new(store.db.clone()),
        surreal: store.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt155-route-auth-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

/// The real `api::kernel` router on a loopback listener; aborted and joined on drop.
struct KernelServer {
    base: String,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl KernelServer {
    async fn start(store: &EmbeddedKnowledgeStore) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let addr = listener.local_addr().expect("local addr");
        let app = handshake_core::api::kernel::routes(test_state(store));
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("kernel api server");
        });
        Self {
            base: format!("http://{addr}"),
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("{}{ROUTE}", self.base)
    }
}

impl Drop for KernelServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

/// Installs the server-side route hooks for one test body and clears them on drop (also on panic).
struct RouteHooksGuard;

impl RouteHooksGuard {
    fn install(hooks: ProductScreenshotRouteTestHooks) -> Self {
        set_product_screenshot_route_test_hooks(Some(hooks));
        Self
    }
}

impl Drop for RouteHooksGuard {
    fn drop(&mut self) {
        set_product_screenshot_route_test_hooks(None);
    }
}

/// A per-test sandbox under the owner TMP dir, laid out like the product app directory
/// (`app/scripts/*.mjs` + `app/node_modules/playwright/package.json`) so the route's pre-flight
/// accepts a test adapter script. Removed on drop.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!("mt155-{label}-{}", uuid::Uuid::now_v7()));
        let playwright = root.join("app").join("node_modules").join("playwright");
        std::fs::create_dir_all(&playwright).expect("create sandbox playwright dir");
        std::fs::create_dir_all(root.join("app").join("scripts")).expect("create scripts dir");
        std::fs::write(playwright.join("package.json"), "{}").expect("write playwright probe");
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn artifact_root(&self) -> PathBuf {
        self.root.join("artifacts")
    }

    fn script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.join("app").join("scripts").join(name);
        std::fs::write(&path, body).expect("write sandbox script");
        path
    }

    /// A node script that only records that it ran.
    fn sentinel_script(&self, name: &str, marker: &Path) -> PathBuf {
        self.script(
            name,
            &format!(
                "import {{ writeFileSync }} from \"node:fs\";\nwriteFileSync({}, String(process.pid));\n",
                js_string(marker)
            ),
        )
    }

    /// A node script that behaves like the capture adapter: it records that it ran and writes a
    /// real PNG to `--output`.
    fn fake_adapter_script(&self, marker: &Path) -> PathBuf {
        let png = self.path("source.png");
        std::fs::write(&png, tiny_png_bytes()).expect("write source png");
        self.script(
            "fake-adapter.mjs",
            &format!(
                "import {{ copyFileSync, writeFileSync }} from \"node:fs\";\n\
                 const args = process.argv.slice(2);\n\
                 const output = args[args.indexOf(\"--output\") + 1];\n\
                 writeFileSync({marker}, String(process.pid));\n\
                 copyFileSync({png}, output);\n",
                marker = js_string(marker),
                png = js_string(&png)
            ),
        )
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn js_string(path: &Path) -> String {
    serde_json::to_string(&path.to_string_lossy()).expect("path as JS string literal")
}

fn tiny_png_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        4,
        3,
        image::Rgba([12, 34, 56, 255]),
    ))
    .write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .expect("tiny png writes");
    bytes
}

fn capture_body(label: &str) -> Value {
    json!({
        "request": {
            "request_id": format!("request.mt155.{label}.{}", uuid::Uuid::now_v7()),
            "scope": "Module",
            "target_ref": "module://mt155-route-auth",
            "requested_by_role": "KERNEL_BUILDER",
            "trigger_kind": "DccApi",
            "window_title": "Handshake",
            "width": 4,
            "height": 3,
            "capture_adapter_ref": "capture-adapter://app/playwright-dom-screenshot",
            "flight_recorder_ref": "FR-EVT-VISUAL-CAPTURE-MT155",
            "execution_surface": "GovernedAdapterApi",
            "workdir_ref": "repo-root://"
        },
        "source_url": "http://127.0.0.1:9/"
    })
}

async fn status_and_json(response: reqwest::Response) -> (reqwest::StatusCode, Value) {
    let status = response.status();
    let text = response.text().await.expect("response body");
    let body = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (status, body)
}

async fn open_store() -> EmbeddedKnowledgeStore {
    open_embedded_store()
        .await
        .expect("MT-155 requires an isolated embedded store")
}

/// Assert that nothing ran: no server-adapter marker, no caller marker, and no artifact root (the
/// route creates `<artifact_root>/adapter-output` only after the `node --version` pre-flight).
fn assert_nothing_spawned(sandbox: &Sandbox, markers: &[&Path], context: &str) {
    for marker in markers {
        assert!(
            !marker.exists(),
            "{context}: sentinel {} must never be written",
            marker.display()
        );
    }
    assert!(
        !sandbox.artifact_root().exists(),
        "{context}: no artifact/adapter-output directory may be created"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt155_anonymous_capture_is_401_and_spawns_nothing() {
    let store = open_store().await;
    let fixture = AccountFixture::install(&store.storage).await;
    let sandbox = Sandbox::new("anonymous");
    let server_marker = sandbox.path("server-adapter.marker");
    let server_script = sandbox.sentinel_script("server-sentinel.mjs", &server_marker);
    let _hooks = RouteHooksGuard::install(ProductScreenshotRouteTestHooks {
        timeout: Some(Duration::from_secs(60)),
        adapter_script_path: Some(server_script),
        artifact_root: Some(sandbox.artifact_root()),
    });
    let server = KernelServer::start(&store).await;
    let anonymous = reqwest::Client::new();

    // No credentials at all: a valid body that would otherwise run the server-side sentinel.
    let (status, body) = status_and_json(
        anonymous
            .post(server.url())
            .json(&capture_body("anonymous"))
            .send()
            .await
            .expect("anonymous request"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body, json!({"error": "HSK-401-SESSION-REQUIRED"}));

    // A syntactically valid but unknown session token bound to the live channel: still 401.
    let (status, body) = status_and_json(
        anonymous
            .post(server.url())
            .header(SESSION_TOKEN_HEADER, "ab".repeat(32))
            .header(CHANNEL_BINDING_TOKEN_HEADER, fixture.binding_token())
            .json(&capture_body("forged-session"))
            .send()
            .await
            .expect("forged-session request"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body, json!({"error": "HSK-401-SESSION-REQUIRED"}));

    // A real session token without the live channel binding: 401.
    let (status, body) = status_and_json(
        anonymous
            .post(server.url())
            .header(SESSION_TOKEN_HEADER, &fixture.session_token)
            .json(&capture_body("unbound-session"))
            .send()
            .await
            .expect("unbound-session request"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body, json!({"error": "HSK-401-SESSION-REQUIRED"}));

    // Authentication precedes body parsing: a malformed body is 401, not a parse error.
    let (status, body) = status_and_json(
        anonymous
            .post(server.url())
            .header("content-type", "application/json")
            .body("{not json")
            .send()
            .await
            .expect("anonymous malformed request"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED, "{body}");

    assert_nothing_spawned(&sandbox, &[&server_marker], "anonymous");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt155_account_without_capability_is_403_and_spawns_nothing() {
    let store = open_store().await;
    // The default Owner capability set (fs.*, fr.*, memory.*) does NOT include the capture
    // capability: deny by default.
    let fixture = AccountFixture::install(&store.storage).await;
    // A principal delegated the `*` wildcard and a prefix pattern must not pass either: the route
    // accepts only the exact capability id.
    let wildcard = OwnerSession::provision_with_capabilities(
        &store.storage,
        fixture.binding_token(),
        &["*", "kernel.product_screenshot_capture.*"],
    )
    .await;
    let sandbox = Sandbox::new("no-capability");
    let server_marker = sandbox.path("server-adapter.marker");
    let server_script = sandbox.sentinel_script("server-sentinel.mjs", &server_marker);
    let _hooks = RouteHooksGuard::install(ProductScreenshotRouteTestHooks {
        timeout: Some(Duration::from_secs(60)),
        adapter_script_path: Some(server_script),
        artifact_root: Some(sandbox.artifact_root()),
    });
    let server = KernelServer::start(&store).await;

    for (label, session) in [("owner", &fixture.owner), ("wildcard", &wildcard)] {
        let (status, body) = status_and_json(
            session
                .client()
                .post(server.url())
                .json(&capture_body(label))
                .send()
                .await
                .expect("authenticated request without capability"),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::FORBIDDEN, "{label}: {body}");
        assert_eq!(
            body,
            json!({"error": "HSK-403-PROTECTED-RESOURCE"}),
            "{label}: constant denial body"
        );
    }

    assert_nothing_spawned(&sandbox, &[&server_marker], "account without capability");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt155_authorized_capture_passes_authorization_and_is_attributed() {
    let store = open_store().await;
    let fixture = AccountFixture::install(&store.storage).await;
    let capturer = OwnerSession::provision_with_capabilities(
        &store.storage,
        fixture.binding_token(),
        &[PRODUCT_SCREENSHOT_CAPTURE_EXECUTE_CAPABILITY],
    )
    .await;
    let sandbox = Sandbox::new("authorized");
    let adapter_marker = sandbox.path("adapter-ran.marker");
    let adapter_script = sandbox.fake_adapter_script(&adapter_marker);
    let _hooks = RouteHooksGuard::install(ProductScreenshotRouteTestHooks {
        timeout: Some(Duration::from_secs(60)),
        adapter_script_path: Some(adapter_script),
        artifact_root: Some(sandbox.artifact_root()),
    });
    let server = KernelServer::start(&store).await;

    // The server defaults may still be echoed by older callers; they are accepted, not executed
    // from the body (the server-side script runs).
    let mut body = capture_body("authorized");
    body["node_binary"] = json!("node");
    body["adapter_script_path"] = json!("app/scripts/handshake-screenshot-capture.mjs");
    let (status, result) = status_and_json(
        capturer
            .client()
            .post(server.url())
            .json(&body)
            .send()
            .await
            .expect("authorized capture request"),
    )
    .await;
    assert_ne!(status, reqwest::StatusCode::UNAUTHORIZED, "{result}");
    assert_ne!(status, reqwest::StatusCode::FORBIDDEN, "{result}");
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "authorized capture must run (requires node on PATH): {result}"
    );
    assert!(
        adapter_marker.exists(),
        "the server-side adapter really ran"
    );

    // Response shape unchanged.
    assert_eq!(
        result["schema_id"],
        "hsk.kernel.product_screenshot_capture_execute_result@1"
    );
    for key in ["artifact", "durable_receipt", "proof", "receipt"] {
        assert!(result[key].is_object(), "response keeps `{key}`: {result}");
    }
    assert_eq!(
        result["receipt"]["command_or_api_ref"],
        "api://kernel.product_screenshot_capture.execute"
    );

    // The initiator is the session principal, never a constant or a header value.
    let expected = json!({"actor_id": capturer.actor_id, "session_id": capturer.session_id});
    assert_eq!(result["receipt"]["initiated_by"], expected, "{result}");
    assert_eq!(result["proof"]["initiated_by"], expected, "{result}");
    assert_ne!(
        capturer.session_id, fixture.owner.session_id,
        "the capturer is a distinct session"
    );

    // Canonical re-read: the durable receipt on disk carries the same initiator.
    let receipt_path = result["receipt"]["receipt_path"]
        .as_str()
        .expect("receipt_path");
    assert!(
        Path::new(receipt_path).starts_with(sandbox.artifact_root()),
        "receipt written under the server-side artifact root: {receipt_path}"
    );
    let durable: Value = serde_json::from_slice(
        &std::fs::read(receipt_path).expect("read durable receipt from disk"),
    )
    .expect("durable receipt JSON");
    assert_eq!(durable["initiated_by"], expected, "{durable}");
    let screenshot = std::fs::read(
        result["artifact"]["screenshot_path"]
            .as_str()
            .expect("screenshot_path"),
    )
    .expect("screenshot on disk");
    assert_eq!(screenshot, tiny_png_bytes(), "the adapter PNG was recorded");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt155_caller_supplied_binary_is_never_executed() {
    let store = open_store().await;
    let fixture = AccountFixture::install(&store.storage).await;
    let capturer = OwnerSession::provision_with_capabilities(
        &store.storage,
        fixture.binding_token(),
        &[PRODUCT_SCREENSHOT_CAPTURE_EXECUTE_CAPABILITY],
    )
    .await;
    let sandbox = Sandbox::new("caller-path");
    let server_marker = sandbox.path("server-adapter.marker");
    let server_script = sandbox.sentinel_script("server-sentinel.mjs", &server_marker);
    let caller_marker = sandbox.path("caller-script.marker");
    let caller_script = sandbox.sentinel_script("caller-sentinel.mjs", &caller_marker);
    let caller_binary_marker = sandbox.path("caller-binary.marker");
    let caller_binary = sandbox.path(if cfg!(windows) {
        "caller-node.cmd"
    } else {
        "caller-node.sh"
    });
    std::fs::write(
        &caller_binary,
        format!(
            "echo ran > \"{}\"\n",
            caller_binary_marker.to_string_lossy()
        ),
    )
    .expect("write caller binary sentinel");
    let _hooks = RouteHooksGuard::install(ProductScreenshotRouteTestHooks {
        timeout: Some(Duration::from_secs(60)),
        adapter_script_path: Some(server_script),
        artifact_root: Some(sandbox.artifact_root()),
    });
    let server = KernelServer::start(&store).await;

    let cases = [
        (
            "caller-script",
            json!(null),
            json!(caller_script.to_string_lossy()),
        ),
        (
            "caller-binary",
            json!(caller_binary.to_string_lossy()),
            json!(null),
        ),
        (
            "caller-both",
            json!(caller_binary.to_string_lossy()),
            json!(caller_script.to_string_lossy()),
        ),
    ];
    for (label, node_binary, adapter_script_path) in cases {
        let mut body = capture_body(label);
        body["node_binary"] = node_binary;
        body["adapter_script_path"] = adapter_script_path;
        let (status, result) = status_and_json(
            capturer
                .client()
                .post(server.url())
                .json(&body)
                .send()
                .await
                .expect("caller-path request"),
        )
        .await;
        assert_eq!(
            status,
            reqwest::StatusCode::BAD_REQUEST,
            "{label}: {result}"
        );
        assert_eq!(
            result["code"], "kernel_product_screenshot_capture_caller_path_rejected",
            "{label}: {result}"
        );
    }

    assert_nothing_spawned(
        &sandbox,
        &[&server_marker, &caller_marker, &caller_binary_marker],
        "caller-supplied executable or script",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt155_hung_adapter_is_killed_and_reaped_within_timeout() {
    let store = open_store().await;
    let fixture = AccountFixture::install(&store.storage).await;
    let capturer = OwnerSession::provision_with_capabilities(
        &store.storage,
        fixture.binding_token(),
        &[PRODUCT_SCREENSHOT_CAPTURE_EXECUTE_CAPABILITY],
    )
    .await;
    let sandbox = Sandbox::new("hung-adapter");
    let pid_file = sandbox.path("hung-adapter.pid");
    let survived = sandbox.path("hung-adapter-survived.marker");
    let hang_script = sandbox.script(
        "hang-adapter.mjs",
        &format!(
            "import {{ writeFileSync }} from \"node:fs\";\n\
             writeFileSync({pid}, String(process.pid));\n\
             setTimeout(() => writeFileSync({survived}, \"survived\"), 8000);\n\
             setInterval(() => {{}}, 1000);\n",
            pid = js_string(&pid_file),
            survived = js_string(&survived)
        ),
    );
    let timeout = Duration::from_secs(3);
    let _hooks = RouteHooksGuard::install(ProductScreenshotRouteTestHooks {
        timeout: Some(timeout),
        adapter_script_path: Some(hang_script),
        artifact_root: Some(sandbox.artifact_root()),
    });
    let server = KernelServer::start(&store).await;

    let started = Instant::now();
    let (status, result) = status_and_json(
        capturer
            .client()
            .post(server.url())
            .json(&capture_body("hung"))
            .send()
            .await
            .expect("hung-adapter request"),
    )
    .await;
    let elapsed = started.elapsed();
    assert_eq!(
        status,
        reqwest::StatusCode::GATEWAY_TIMEOUT,
        "a hung adapter returns the typed timeout (requires node on PATH): {result}"
    );
    assert_eq!(result["code"], "kernel_product_screenshot_capture_timeout");
    let message = result["message"].as_str().expect("timeout message");
    assert!(message.contains("AdapterTimedOut"), "{message}");
    assert!(message.contains("stage: \"adapter\""), "{message}");
    assert!(message.contains("reaped: true"), "{message}");
    assert!(
        elapsed < timeout * 2 + Duration::from_secs(5),
        "the route returned within its bound (pre-flight + adapter), took {elapsed:?}"
    );

    let pid: u32 = std::fs::read_to_string(&pid_file)
        .expect("the hung adapter started and wrote its pid")
        .trim()
        .parse()
        .expect("pid");
    assert!(message.contains(&format!("pid: {pid}")), "{message}");

    // Past the point where a surviving adapter would have written its marker.
    tokio::time::sleep(Duration::from_secs(7)).await;
    assert!(
        !survived.exists(),
        "the timed-out adapter (pid {pid}) must be killed, not left running"
    );
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let listing = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .expect("run tasklist");
        let listing = String::from_utf8_lossy(&listing.stdout);
        assert!(
            !listing.contains(&format!("\"{pid}\"")),
            "the timed-out adapter pid {pid} must no longer exist: {listing}"
        );
    }
}
