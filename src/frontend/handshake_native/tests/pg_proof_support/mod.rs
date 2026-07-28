//! Real managed-PostgreSQL + handshake_core fixture shared by native-editor integration proofs.
//!
//! The frontend/backend Cargo graphs intentionally stay separate (their tree-sitter ABI lines differ).
//! The fixture attaches to `HSK_TEST_BASE` when a healthy root-managed product backend is present, or
//! starts the explicitly-built product executable named by `HSK_TEST_BACKEND_BIN`. Stage proofs always
//! use an owned process so the backend inherits their private discovery-binding root. It never invokes Cargo.
//! An owned process is killed on drop; an attached process is never touched. Every proof creates its own
//! workspace through production HTTP and deletes that workspace before releasing the fixture lock.

#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use handshake_native::backend_client::build_backend_client;

const FIXTURE_LOCK_TIMEOUT: Duration = Duration::from_secs(60);
// A fresh current-source PostgreSQL database must apply the complete managed
// migration set before it can publish the listen report. Keep this setup bound
// separate from the measured LC-06 route budget; under concurrent local builds
// the first migration pass can legitimately exceed one minute.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(300);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);
// Contract-sized 5k-row corpora traverse public validation, PostgreSQL, EventLedger, projections, and
// search refresh at the product's measured ~12-writes/s write ceiling, so large corpora (LK-03's 10k
// rows) legitimately need the full bounded setup window to SEED. Setup is explicitly OUTSIDE measured
// query time (it never affects a budget). The default and maximum are identical for the canonical proof;
// an optional lower value can fail faster locally but can never widen the canonical setup allowance.
const DEFAULT_SETUP_TIMEOUT: Duration = Duration::from_secs(1200);
const MAX_SETUP_TIMEOUT: Duration = Duration::from_secs(1200);

pub const DEFAULT_BASE: &str = "http://127.0.0.1:37501";

pub struct LiveBackend {
    pub base: String,
    pub workspace_id: String,
    client: reqwest::Client,
    rt: tokio::runtime::Runtime,
    owned_backend: Option<Child>,
    owned_binary: Option<PathBuf>,
    owned_data_dir: Option<PathBuf>,
    _fixture_lock: FileLock,
}

/// Hard aggregate wall-clock deadline for fixture creation. Per-request timeouts alone do not bound a
/// large loop of individually successful requests, so large-corpus proofs check this guard on every
/// iteration and once after setup.
pub struct SetupDeadline {
    label: String,
    started: Instant,
    timeout: Duration,
}

impl SetupDeadline {
    pub fn begin(label: impl Into<String>) -> Self {
        let timeout = std::env::var("HSK_PROOF_SETUP_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_SETUP_TIMEOUT)
            .min(MAX_SETUP_TIMEOUT);
        Self {
            label: label.into(),
            started: Instant::now(),
            timeout,
        }
    }

    pub fn check(&self) {
        assert!(
            self.started.elapsed() <= self.timeout,
            "{} fixture setup exceeded hard aggregate deadline of {}s",
            self.label,
            self.timeout.as_secs()
        );
    }

    fn remaining(&self) -> Duration {
        self.timeout
            .checked_sub(self.started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .unwrap_or_else(|| {
                panic!(
                    "{} fixture setup exceeded hard aggregate deadline of {}s",
                    self.label,
                    self.timeout.as_secs()
                )
            })
    }

    pub fn timeout_secs(&self) -> u64 {
        self.timeout.as_secs()
    }
}

struct PendingChild(Option<Child>);

impl PendingChild {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn child_mut(&mut self) -> &mut Child {
        self.0.as_mut().expect("pending child exists")
    }

    fn take(mut self) -> Child {
        self.0.take().expect("pending child exists")
    }
}

impl Drop for PendingChild {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
            while Instant::now() < deadline {
                match child.try_wait() {
                    Ok(Some(_)) | Err(_) => return,
                    Ok(None) => thread::sleep(Duration::from_millis(50)),
                }
            }
        }
    }
}

fn kill_and_reap(child: &mut Child, operation: &str) {
    let pid = child.id();
    child
        .kill()
        .unwrap_or_else(|error| panic!("{operation}: kill owned backend pid {pid}: {error}"));
    let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => panic!(
                "{operation}: owned backend pid {pid} did not exit within {}s",
                SHUTDOWN_TIMEOUT.as_secs()
            ),
            Err(error) => panic!("{operation}: failed to reap owned backend pid {pid}: {error}"),
        }
    }
}

pub fn require_live_backend() -> LiveBackend {
    start_product_backend(true)
}

pub fn require_reachable_backend() -> LiveBackend {
    start_product_backend(false)
}

fn start_product_backend(create_workspace: bool) -> LiveBackend {
    let lock = FileLock::acquire(
        &external_artifact_root().join("managed-backend-fixture.lock"),
        FIXTURE_LOCK_TIMEOUT,
    )
    .unwrap_or_else(|| {
        panic!(
            "managed backend fixture lock timed out after {}s",
            FIXTURE_LOCK_TIMEOUT.as_secs()
        )
    });
    let configured_base =
        std::env::var("HSK_TEST_BASE").unwrap_or_else(|_| DEFAULT_BASE.to_owned());
    let force_owned = std::env::var_os("HANDSHAKE_TEST_STAGE_BINDING_ROOT").is_some();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build native proof client runtime");
    let client = build_backend_client();

    let mut base = configured_base.clone();
    let mut owned_backend = None;
    let mut owned_binary = None;
    let mut owned_data_dir = None;
    if force_owned || !healthy(&rt, &client, &configured_base) {
        if !force_owned {
            assert_eq!(
                configured_base, DEFAULT_BASE,
                "HSK_TEST_BASE={configured_base} is attach-only and cannot be replaced by an owned backend"
            );
        }
        let binary = resolve_backend_binary();
        let (child, report_path, data_dir) = spawn_backend(&binary);
        let mut pending = PendingChild::new(child);
        base = wait_for_listen_report(pending.child_mut(), &report_path);
        wait_for_health(&rt, &client, &base, pending.child_mut());
        owned_backend = Some(pending.take());
        owned_binary = Some(binary);
        owned_data_dir = Some(data_dir);
    }

    let mut backend = LiveBackend {
        base,
        workspace_id: String::new(),
        client,
        rt,
        owned_backend,
        owned_binary,
        owned_data_dir,
        _fixture_lock: lock,
    };
    backend.assert_healthy();
    if create_workspace {
        let workspace = backend.create_workspace(&format!(
            "wp-kernel-012-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        backend.workspace_id = workspace
            .get("id")
            .or_else(|| workspace.pointer("/workspace/id"))
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| panic!("POST /workspaces response lacks id: {workspace}"))
            .to_owned();
    }
    backend
}

fn resolve_backend_binary() -> PathBuf {
    let executable_name = if cfg!(windows) {
        "handshake_core.exe"
    } else {
        "handshake_core"
    };
    let configured = std::env::var_os("HSK_TEST_BACKEND_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            panic!(
                "owned backend requires explicit HSK_TEST_BACKEND_BIN from the isolated current-source cargo build"
            )
        });
    let binary = configured.canonicalize().unwrap_or_else(|error| {
        panic!(
            "canonicalize HSK_TEST_BACKEND_BIN {}: {error}",
            configured.display()
        )
    });
    assert_eq!(
        binary.file_name().and_then(|name| name.to_str()),
        Some(executable_name),
        "HSK_TEST_BACKEND_BIN must name the handshake_core product executable"
    );
    let target = Path::new("../../../../Handshake_Artifacts/handshake-cargo-target")
        .canonicalize()
        .expect("canonicalize configured canonical Cargo target");
    assert!(
        binary.starts_with(&target),
        "HSK_TEST_BACKEND_BIN {} is outside configured canonical Cargo target {}",
        binary.display(),
        target.display()
    );
    assert_backend_binary_is_current_source(&binary);
    binary
}

fn assert_backend_binary_is_current_source(binary: &Path) {
    let binary_modified = binary
        .metadata()
        .and_then(|metadata| metadata.modified())
        .expect("inspect HSK_TEST_BACKEND_BIN modification time");
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate must live at repo/src/frontend/handshake_native");
    let backend_root = repo_root.join("src/backend/handshake_core");
    let mut inputs = vec![
        backend_root.join("Cargo.toml"),
        backend_root.join("Cargo.lock"),
        backend_root.join("build.rs"),
        backend_root.join("mechanical_engines.json"),
        backend_root.join("src"),
        backend_root.join("migrations"),
        backend_root.join("schemas"),
    ];
    while let Some(input) = inputs.pop() {
        let metadata = input.metadata().unwrap_or_else(|error| {
            panic!("inspect current-source input {}: {error}", input.display())
        });
        if metadata.is_dir() {
            inputs.extend(
                std::fs::read_dir(&input)
                    .unwrap_or_else(|error| {
                        panic!("read current-source input {}: {error}", input.display())
                    })
                    .map(|entry| entry.expect("read current-source directory entry").path()),
            );
            continue;
        }
        let input_modified = metadata.modified().unwrap_or_else(|error| {
            panic!("inspect current-source mtime {}: {error}", input.display())
        });
        assert!(
            input_modified <= binary_modified,
            "HSK_TEST_BACKEND_BIN {} predates current source input {}; rerun the explicit isolated `cargo build --bin handshake_core --features app-runtime`",
            binary.display(),
            input.display()
        );
    }
}

fn spawn_backend(binary: &Path) -> (Child, PathBuf, PathBuf) {
    spawn_backend_at(binary, "127.0.0.1:0", None)
}

fn spawn_backend_at(
    binary: &Path,
    listen_addr: &str,
    existing_data_dir: Option<&Path>,
) -> (Child, PathBuf, PathBuf) {
    let run_id = uuid::Uuid::new_v4();
    let run_root = external_artifact_root()
        .join("backend-runtime")
        .join(run_id.to_string());
    std::fs::create_dir_all(&run_root).expect("create backend runtime artifact directory");
    restrict_runtime_directory(&run_root);
    let data_dir = existing_data_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| run_root.join("data"));
    if existing_data_dir.is_none() {
        std::fs::create_dir(&data_dir).expect("create owned backend data directory");
        restrict_runtime_directory(&data_dir);
    } else {
        assert!(
            data_dir.is_dir(),
            "restarted owned backend must reuse its existing data directory"
        );
    }
    let report_path = run_root.join("listen-report.json");
    let database_url = std::env::var("HANDSHAKE_TEST_PG_DSN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("owned backend requires explicit HANDSHAKE_TEST_PG_DSN"));
    let stdout =
        File::create(run_root.join("backend.stdout.log")).expect("create backend stdout log");
    let stderr =
        File::create(run_root.join("backend.stderr.log")).expect("create backend stderr log");
    let mut command = Command::new(binary);
    command
        // Adversarial review B4: pin the child CWD to its external runtime dir so any relative-path write
        // by the backend lands under Handshake_Artifacts, never inside the repo worktree (under `cargo
        // test` the inherited CWD is the in-repo crate dir).
        .current_dir(&run_root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .env("HANDSHAKE_BACKEND_LISTEN_ADDR", listen_addr)
        .env("HANDSHAKE_BACKEND_LISTEN_REPORT_FILE", &report_path)
        .env("HANDSHAKE_DATA_DIR", &data_dir)
        .env("DATABASE_URL", database_url)
        .env("HANDSHAKE_STORAGE_MODE", "postgres_primary")
        .env("HANDSHAKE_CONTROL_PLANE_REQUIRES_POSTGRES", "true")
        .env("HANDSHAKE_MANAGED_PG_ENABLED", "false");
    let child = no_window(&mut command)
        .spawn()
        .unwrap_or_else(|error| panic!("start {}: {error}", binary.display()));
    (child, report_path, data_dir)
}

fn wait_for_listen_report(child: &mut Child, report_path: &Path) -> String {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        match std::fs::read(report_path) {
            Ok(bytes) => {
                let report: serde_json::Value = match serde_json::from_slice(&bytes) {
                    Ok(report) => report,
                    Err(error)
                        if error.classify() == serde_json::error::Category::Eof
                            && Instant::now() < deadline =>
                    {
                        if let Some(status) = child.try_wait().expect("poll owned backend") {
                            panic!(
                                "owned handshake_core exited while publishing listen report with \
                                 {status}: {error}"
                            );
                        }
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    Err(error) => panic!("parse {}: {error}", report_path.display()),
                };
                assert_eq!(
                    report["schema_id"], "handshake.backend-listen-report.v1",
                    "owned backend listen report schema drifted"
                );
                assert_eq!(
                    report["pid"].as_u64(),
                    Some(u64::from(child.id())),
                    "owned backend listen report pid drifted"
                );
                let addr = report["listen_addr"]
                    .as_str()
                    .expect("owned backend listen report address")
                    .parse::<std::net::SocketAddr>()
                    .expect("parse owned backend listen address");
                assert!(addr.ip().is_loopback(), "owned backend must bind loopback");
                assert_ne!(
                    addr.port(),
                    0,
                    "owned backend must report its assigned port"
                );
                return format!("http://{addr}");
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("read {}: {error}", report_path.display()),
        }
        if let Some(status) = child.try_wait().expect("poll owned backend") {
            panic!("owned handshake_core exited before listen report with {status}");
        }
        assert!(
            Instant::now() < deadline,
            "owned handshake_core did not publish listen report within {}s",
            STARTUP_TIMEOUT.as_secs()
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn restrict_runtime_directory(_path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(_path, std::fs::Permissions::from_mode(0o700)).unwrap_or_else(
            |error| panic!("restrict runtime directory {}: {error}", _path.display()),
        );
    }
}

#[cfg(windows)]
fn no_window(command: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(windows))]
fn no_window(command: &mut Command) -> &mut Command {
    command
}

fn healthy(rt: &tokio::runtime::Runtime, client: &reqwest::Client, base: &str) -> bool {
    let url = format!("{base}/health");
    rt.block_on(async {
        match client.get(url).timeout(Duration::from_secs(2)).send().await {
            Ok(response) if response.status().is_success() => response
                .json::<serde_json::Value>()
                .await
                .ok()
                .is_some_and(|value| value["status"] == "ok" && value["db_status"] == "ok"),
            _ => false,
        }
    })
}

fn wait_for_health(
    rt: &tokio::runtime::Runtime,
    client: &reqwest::Client,
    base: &str,
    child: &mut Child,
) {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if healthy(rt, client, base) {
            return;
        }
        if let Some(status) = child.try_wait().expect("poll owned backend") {
            panic!("owned handshake_core exited before health with {status}");
        }
        assert!(
            Instant::now() < deadline,
            "owned handshake_core did not become healthy within {}s",
            STARTUP_TIMEOUT.as_secs()
        );
        thread::sleep(Duration::from_millis(200));
    }
}

impl LiveBackend {
    /// OS process id of the exact current-source backend owned by this fixture. Live fault-injection
    /// proofs use this identity to suspend only their own backend process; attached/root-managed
    /// backends are never eligible.
    pub fn owned_process_id(&self) -> u32 {
        self.owned_backend
            .as_ref()
            .expect("owned_process_id requires a fixture-owned backend")
            .id()
    }

    /// Exact executable used to spawn this fixture-owned backend. The live proof hashes this file and
    /// `restart_owned` reuses it rather than re-resolving mutable environment state mid-scenario.
    pub fn owned_binary_path(&self) -> &Path {
        self.owned_binary
            .as_deref()
            .expect("owned_binary_path requires a fixture-owned backend")
    }

    /// Restart the exact backend process owned by this fixture while preserving its PostgreSQL
    /// authority. The replacement is spawned from the same current-source executable and private
    /// binding root, then health-gated before the new ephemeral base URL is returned.
    pub fn restart_owned(&mut self) -> (String, String) {
        let child = self
            .owned_backend
            .as_mut()
            .expect("restart_owned requires a fixture-owned backend");
        let old_base = self.base.clone();
        kill_and_reap(child, "restart exact fixture-owned backend");
        self.owned_backend = None;

        let binary = self
            .owned_binary
            .clone()
            .expect("restart_owned requires the fixture's original backend executable");
        let listen_addr = old_base
            .strip_prefix("http://")
            .expect("owned backend base is an HTTP listener");
        let data_dir = self
            .owned_data_dir
            .clone()
            .expect("restart_owned requires the fixture's persistent data directory");
        let (replacement, report_path, replacement_data_dir) =
            spawn_backend_at(&binary, listen_addr, Some(&data_dir));
        let mut pending = PendingChild::new(replacement);
        let new_base = wait_for_listen_report(pending.child_mut(), &report_path);
        wait_for_health(&self.rt, &self.client, &new_base, pending.child_mut());
        self.base = new_base.clone();
        self.owned_backend = Some(pending.take());
        self.owned_data_dir = Some(replacement_data_dir);
        self.assert_healthy();
        assert_eq!(
            old_base, new_base,
            "owned restart must reclaim the exact app-bound listener"
        );
        (old_base, new_base)
    }

    fn assert_healthy(&self) {
        assert!(
            healthy(&self.rt, &self.client, &self.base),
            "product backend is not PostgreSQL-healthy at {}",
            self.base
        );
    }

    fn ident(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header("x-hsk-actor-id", "wp-kernel-012-native-proof")
            .header("x-hsk-kernel-task-run-id", "wp-kernel-012-native-proof")
            .header("x-hsk-session-run-id", "wp-kernel-012-native-proof-session")
            .header("x-hsk-actor-kind", "operator")
    }

    fn workspace_ident(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header("x-hsk-actor-id", "wp-kernel-012-native-proof")
            .header("x-hsk-actor-kind", "human")
    }

    pub fn create_workspace(&self, name: &str) -> serde_json::Value {
        self.request_json(
            self.workspace_ident(self.client.post(format!("{}/workspaces", self.base)))
                .json(&serde_json::json!({"name": name})),
            "POST /workspaces",
        )
    }

    /// Seed a large fixture in one PostgreSQL transaction through the canonical Loom tables. This is
    /// fixture-only setup: the measured query still enters through the production HTTP route. The psql
    /// child is hard-bounded to 60 seconds and is always killed/reaped on timeout.
    pub fn run_fixture_sql(&self, label: &str, sql: &str) {
        let setup_deadline = SetupDeadline::begin(label);
        let database_url = [
            "HANDSHAKE_TEST_PG_DSN",
            "HSK_PROOF_DATABASE_URL",
            "POSTGRES_TEST_URL",
            "DATABASE_URL",
        ]
            .into_iter()
            .find_map(|name| std::env::var(name).ok().filter(|value| !value.trim().is_empty()))
            .unwrap_or_else(|| {
                panic!(
                    "{label}: batch fixture needs HANDSHAKE_TEST_PG_DSN, HSK_PROOF_DATABASE_URL, POSTGRES_TEST_URL, or DATABASE_URL"
                )
            });
        let psql = std::env::var_os("HSK_PSQL_BIN").unwrap_or_else(|| "psql".into());
        let log_dir = external_artifact_root().join("fixture-runtime");
        std::fs::create_dir_all(&log_dir).expect("create fixture runtime artifact directory");
        let run_id = uuid::Uuid::new_v4();
        let stdout_path = log_dir.join(format!("{label}-{run_id}.stdout.log"));
        let stderr_path = log_dir.join(format!("{label}-{run_id}.stderr.log"));
        let stdout = File::create(&stdout_path).expect("create fixture stdout log");
        let stderr = File::create(&stderr_path).expect("create fixture stderr log");
        let mut command = Command::new(psql);
        command
            // Adversarial review B4: pin psql CWD to the external fixture log dir, never the repo worktree.
            .current_dir(&log_dir)
            .arg("--no-psqlrc")
            .arg("--set")
            .arg("ON_ERROR_STOP=1")
            .arg("--dbname")
            .arg(database_url)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("PGCONNECT_TIMEOUT", "5");
        let mut child = no_window(&mut command)
            .spawn()
            .unwrap_or_else(|error| panic!("{label}: start psql batch fixture: {error}"));
        child
            .stdin
            .take()
            .expect("psql fixture stdin")
            .write_all(sql.as_bytes())
            .unwrap_or_else(|error| panic!("{label}: write psql fixture transaction: {error}"));

        let deadline = Instant::now() + Duration::from_secs(setup_deadline.timeout_secs());
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    assert!(
                        status.success(),
                        "{label}: psql batch fixture failed with {status}; stderr={}",
                        stderr_path.display()
                    );
                    break;
                }
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "{label}: psql batch fixture exceeded hard {}s deadline and was killed/reaped; stderr={}",
                        setup_deadline.timeout_secs(),
                        stderr_path.display()
                    );
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("{label}: poll psql batch fixture: {error}");
                }
            }
        }
    }

    pub fn delete_workspace(&self, workspace_id: &str) -> u16 {
        self.request_status(
            self.workspace_ident(
                self.client
                    .delete(format!("{}/workspaces/{workspace_id}", self.base)),
            ),
        )
    }

    /// Explicit, asserted teardown used by proof tests before they write PASS. A workspace delete must
    /// succeed (or report already absent), and an owned backend must be killed and reaped within the hard
    /// shutdown deadline. Attached root-managed processes are never touched.
    pub fn assert_cleanup(&mut self) {
        if !self.workspace_id.is_empty() {
            let workspace_id = std::mem::take(&mut self.workspace_id);
            let status = self.delete_workspace(&workspace_id);
            assert!(
                (200..300).contains(&status) || status == 404,
                "managed fixture workspace cleanup {workspace_id} returned {status}"
            );
        }
        if let Some(child) = self.owned_backend.as_mut() {
            kill_and_reap(child, "clean up fixture-owned backend");
            self.owned_backend = None;
        }
    }

    pub fn require_block_id(&self) -> String {
        panic!(
            "managed fixture self-seeds block ids; operator-supplied HSK_TEST_BLOCK_ID is forbidden"
        )
    }

    pub fn post_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        self.request_json(
            self.ident(self.client.post(format!("{}{path}", self.base)))
                .json(body),
            &format!("POST {path}"),
        )
    }

    /// Drive a contract-sized fixture through the product's public mutation routes without serializing
    /// thousands of independent round trips. The concurrency bound protects the backend pool while every
    /// row still traverses route validation, Loom authority mutation, ProjectKnowledgeIndex projection,
    /// EventLedger emission, and search-index refresh. Results preserve request order.
    pub fn post_json_batch_bounded(
        &self,
        requests: Vec<(String, serde_json::Value)>,
        concurrency: usize,
        setup_deadline: &SetupDeadline,
    ) -> Vec<serde_json::Value> {
        self.request_json_batch_bounded(
            reqwest::Method::POST,
            requests,
            concurrency,
            setup_deadline,
        )
    }

    /// Bounded counterpart for product PUT mutations (for example folder memberships).
    pub fn put_json_batch_bounded(
        &self,
        requests: Vec<(String, serde_json::Value)>,
        concurrency: usize,
        setup_deadline: &SetupDeadline,
    ) -> Vec<serde_json::Value> {
        self.request_json_batch_bounded(reqwest::Method::PUT, requests, concurrency, setup_deadline)
    }

    fn request_json_batch_bounded(
        &self,
        method: reqwest::Method,
        requests: Vec<(String, serde_json::Value)>,
        concurrency: usize,
        setup_deadline: &SetupDeadline,
    ) -> Vec<serde_json::Value> {
        assert!(
            concurrency > 0,
            "bounded product mutation concurrency must be non-zero"
        );
        let total = requests.len();
        let mut ordered = vec![serde_json::Value::Null; total];
        for (chunk_number, chunk) in requests.chunks(concurrency).enumerate() {
            let request_timeout = REQUEST_TIMEOUT.min(setup_deadline.remaining());
            let chunk_results = self.rt.block_on(async {
                let mut pending = tokio::task::JoinSet::new();
                for (offset, (path, body)) in chunk.iter().cloned().enumerate() {
                    let client = self.client.clone();
                    let base = self.base.clone();
                    let method = method.clone();
                    pending.spawn(async move {
                        let url = format!("{base}{path}");
                        // A send failure has commit-unknown semantics: the server may have accepted the
                        // mutation before the transport failed. Blindly replaying it can duplicate
                        // non-idempotent writes or hide a product error, so the canonical proof aborts
                        // this run and requires a fresh workspace/run identity.
                        let response = client
                            .request(method.clone(), &url)
                            .header("x-hsk-actor-id", "wp-kernel-012-native-proof")
                            .header("x-hsk-kernel-task-run-id", "wp-kernel-012-native-proof")
                            .header("x-hsk-session-run-id", "wp-kernel-012-native-proof-session")
                            .header("x-hsk-actor-kind", "operator")
                            .json(&body)
                            .timeout(request_timeout)
                            .send()
                            .await
                            .map_err(|error| {
                                format!(
                                    "{method} {path} send failed with commit-unknown semantics; \
                                     abandon this workspace and start a fresh suite run: {error}"
                                )
                            })?;
                        let status = response.status();
                        let text = response
                            .text()
                            .await
                            .map_err(|error| format!("{method} {path} body failed: {error}"))?;
                        if !status.is_success() {
                            return Err(format!("{method} {path} -> {status}: {text}"));
                        }
                        let value = if text.trim().is_empty() {
                            serde_json::Value::Null
                        } else {
                            serde_json::from_str(&text).map_err(|error| {
                                format!("{method} {path} response not JSON ({error}): {text}")
                            })?
                        };
                        Ok::<_, String>((offset, value))
                    });
                }
                let mut results = Vec::with_capacity(chunk.len());
                while let Some(joined) = pending.join_next().await {
                    results
                        .push(joined.map_err(|error| format!("bounded mutation task: {error}"))??);
                }
                Ok::<_, String>(results)
            });
            let chunk_results = chunk_results.unwrap_or_else(|error| {
                panic!(
                    "bounded product mutation batch {chunk_number} of {} failed: {error}",
                    total.div_ceil(concurrency)
                )
            });
            let base_index = chunk_number * concurrency;
            for (offset, value) in chunk_results {
                ordered[base_index + offset] = value;
            }
            setup_deadline.check();
        }
        ordered
    }

    /// Issue a JSON POST without converting a typed non-success response into
    /// an assertion failure. Scenarios must inspect both the status and body;
    /// this is intentionally separate from `post_json`, whose success-only
    /// contract remains the default for all ordinary proof requests.
    pub fn post_json_response(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> (u16, serde_json::Value) {
        let label = format!("POST {path}");
        let (status, text) = self.request_text_response(
            self.ident(self.client.post(format!("{}{path}", self.base)))
                .json(body),
            &label,
        );
        let value = if text.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        };
        (status, value)
    }

    /// PUT counterpart of [`Self::post_json_response`]: returns the status + parsed body without
    /// asserting success, so a proof can exercise a typed non-2xx rejection (e.g. a 400 preference
    /// validation error) directly.
    pub fn put_json_response(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> (u16, serde_json::Value) {
        let label = format!("PUT {path}");
        let (status, text) = self.request_text_response(
            self.ident(self.client.put(format!("{}{path}", self.base)))
                .json(body),
            &label,
        );
        let value = if text.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        };
        (status, value)
    }

    pub fn put_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        self.request_json_allow_empty(
            self.ident(self.client.put(format!("{}{path}", self.base)))
                .json(body),
            &format!("PUT {path}"),
        )
    }

    pub fn patch_json(&self, path: &str, body: &serde_json::Value) -> serde_json::Value {
        self.request_json_allow_empty(
            self.ident(self.client.patch(format!("{}{path}", self.base)))
                .json(body),
            &format!("PATCH {path}"),
        )
    }

    pub fn get_json(&self, path: &str) -> serde_json::Value {
        let text = self.get_text(path);
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("GET {path} response not JSON ({error}): {text}"))
    }

    pub fn get_text(&self, path: &str) -> String {
        self.request_text(
            self.ident(self.client.get(format!("{}{path}", self.base))),
            &format!("GET {path}"),
        )
    }

    pub fn get_bytes(&self, path: &str) -> Vec<u8> {
        let url = format!("{}{path}", self.base);
        let (status, bytes) = self.rt.block_on(async {
            let response = self
                .ident(self.client.get(&url))
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .unwrap_or_else(|error| panic!("GET {url} failed: {error}"));
            let status = response.status();
            let bytes = response.bytes().await.unwrap_or_default().to_vec();
            (status, bytes)
        });
        assert!(status.is_success(), "GET {path} -> {status}");
        bytes
    }

    pub fn get_status(&self, path: &str) -> u16 {
        self.request_status(self.ident(self.client.get(format!("{}{path}", self.base))))
    }

    pub fn delete(&self, path: &str) -> u16 {
        self.request_status(self.ident(self.client.delete(format!("{}{path}", self.base))))
    }

    /// Read back the exact durable Flight Recorder/EventLedger row correlated by one payload id.
    /// Product write routes await their recorder append before responding, but a short bounded poll also
    /// tolerates deployments whose recorder projection becomes visible just after the authority commit.
    pub fn poll_event_by_payload(&self, field: &str, value: &str) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let events = self.get_json(&format!("/events?wsid={}", self.workspace_id));
            if let Some(event) = events.as_array().and_then(|rows| {
                rows.iter().find(|event| {
                    event
                        .get("payload")
                        .and_then(|payload| payload.get(field))
                        .and_then(serde_json::Value::as_str)
                        == Some(value)
                })
            }) {
                assert!(
                    event
                        .get("event_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|id| !id.is_empty()),
                    "correlated event must expose a durable event_id: {event}"
                );
                return event.clone();
            }
            assert!(
                Instant::now() < deadline,
                "no durable /events row correlated by payload.{field}={value} within 10s"
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Correlate the durable `PreferenceRecord` EventLedger row through the canonical kernel
    /// event-ledger aggregate endpoint that the change receipt's `event_ledger_event_id` (a `KE-` id)
    /// actually points at. The Flight Recorder `/events` HTTP projection is a curated business-event
    /// stream (capability/system/diagnostic surfaces) that does not carry preference records and cannot
    /// parse `KE-` ids, so the kernel aggregate is the authoritative recoverability surface for a
    /// preference change receipt. `aggregate_id` is the single path segment
    /// `workspace:{workspace_id}:{preference_id}` — colons are valid path characters (RFC 3986 pchar).
    pub fn poll_preference_event(&self, preference_id: &str, revision: i64) -> serde_json::Value {
        let aggregate_id = format!("workspace:{}:{preference_id}", self.workspace_id);
        let path = format!("/kernel/events/aggregates/preference_record/{aggregate_id}");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let events = self.get_json(&path);
            if let Some(event) = events.as_array().and_then(|rows| {
                rows.iter().find(|event| {
                    let payload = event.get("payload");
                    payload
                        .and_then(|p| p.get("preference_id"))
                        .and_then(serde_json::Value::as_str)
                        == Some(preference_id)
                        && payload
                            .and_then(|p| p.get("revision"))
                            .and_then(serde_json::Value::as_i64)
                            == Some(revision)
                })
            }) {
                assert!(
                    event
                        .get("event_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|id| id.starts_with("KE-")),
                    "correlated preference event must expose a durable kernel EventLedger id: {event}"
                );
                return event.clone();
            }
            assert!(
                Instant::now() < deadline,
                "no durable kernel EventLedger row for preference {preference_id} revision {revision} within 10s"
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn poll_event_by_id(&self, event_id: &str) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let events = self.get_json(&format!("/events?event_id={event_id}"));
            if let Some(event) = events.as_array().and_then(|rows| {
                rows.iter().find(|event| {
                    event.get("event_id").and_then(serde_json::Value::as_str) == Some(event_id)
                        && event
                            .get("wsids")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|ids| {
                                ids.iter()
                                    .any(|id| id.as_str() == Some(self.workspace_id.as_str()))
                            })
                })
            }) {
                return event.clone();
            }
            assert!(
                Instant::now() < deadline,
                "durable /events row {event_id} was not visible for workspace {} within 10s",
                self.workspace_id
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn request_status(&self, request: reqwest::RequestBuilder) -> u16 {
        self.rt.block_on(async {
            request
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map(|response| response.status().as_u16())
                .unwrap_or(0)
        })
    }

    fn request_json(&self, request: reqwest::RequestBuilder, label: &str) -> serde_json::Value {
        let text = self.request_text(request, label);
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("{label} response not JSON ({error}): {text}"))
    }

    fn request_json_allow_empty(
        &self,
        request: reqwest::RequestBuilder,
        label: &str,
    ) -> serde_json::Value {
        let text = self.request_text(request, label);
        if text.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        }
    }

    fn request_text(&self, request: reqwest::RequestBuilder, label: &str) -> String {
        let (status, text) = self.request_text_response(request, label);
        assert!((200..300).contains(&status), "{label} -> {status}: {text}");
        text
    }

    fn request_text_response(
        &self,
        request: reqwest::RequestBuilder,
        label: &str,
    ) -> (u16, String) {
        self.rt.block_on(async {
            let response = request
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .unwrap_or_else(|error| panic!("{label} failed: {error}"));
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            (status, text)
        })
    }
}

impl Drop for LiveBackend {
    fn drop(&mut self) {
        if !self.workspace_id.is_empty() {
            let status = self.delete_workspace(&self.workspace_id);
            if !(200..300).contains(&status) && status != 404 {
                eprintln!(
                    "WARN: managed fixture workspace cleanup {} returned {status}",
                    self.workspace_id
                );
            }
        }
        if let Some(mut child) = self.owned_backend.take() {
            let _ = child.kill();
            let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Ok(None) => {
                        eprintln!(
                            "WARN: owned backend pid {} did not exit within {}s",
                            child.id(),
                            SHUTDOWN_TIMEOUT.as_secs()
                        );
                        break;
                    }
                    Err(error) => {
                        eprintln!(
                            "WARN: failed to reap owned backend pid {}: {error}",
                            child.id()
                        );
                        break;
                    }
                }
            }
        }
    }
}

pub fn external_artifact_root() -> PathBuf {
    if let Some(root) = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT") {
        return PathBuf::from(root).join("wp-kernel-012");
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("native crate must live under a worktree root")
        .join("Handshake_Artifacts")
        .join("wp-kernel-012")
}

struct FileLock {
    file: File,
}

impl FileLock {
    fn acquire(path: &Path, timeout: Duration) -> Option<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .ok()?;
        let started = Instant::now();
        loop {
            match try_lock_file(&file) {
                Ok(true) => return Some(Self { file }),
                Ok(false) if started.elapsed() < timeout => {
                    thread::sleep(Duration::from_millis(100))
                }
                Ok(false) => return None,
                Err(_) => return None,
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = unlock_file(&self.file);
    }
}

#[cfg(unix)]
fn try_lock_file(file: &File) -> std::io::Result<bool> {
    use std::os::fd::AsRawFd;
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } == 0 {
        Ok(true)
    } else {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::WouldBlock {
            Ok(false)
        } else {
            Err(error)
        }
    }
}

#[cfg(unix)]
fn unlock_file(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    const LOCK_UN: i32 = 8;
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    if unsafe { flock(file.as_raw_fd(), LOCK_UN) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
#[repr(C)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset_or_pointer: usize,
    event: *mut std::ffi::c_void,
}

#[cfg(windows)]
fn try_lock_file(file: &File) -> std::io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    const LOCKFILE_FAIL_IMMEDIATELY: u32 = 0x0000_0001;
    const LOCKFILE_EXCLUSIVE_LOCK: u32 = 0x0000_0002;
    const ERROR_LOCK_VIOLATION: i32 = 33;
    unsafe extern "system" {
        fn LockFileEx(
            file: *mut std::ffi::c_void,
            flags: u32,
            reserved: u32,
            bytes_low: u32,
            bytes_high: u32,
            overlapped: *mut Overlapped,
        ) -> i32;
    }
    let mut overlapped = Overlapped {
        internal: 0,
        internal_high: 0,
        offset_or_pointer: 0,
        event: std::ptr::null_mut(),
    };
    let locked = unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_FAIL_IMMEDIATELY | LOCKFILE_EXCLUSIVE_LOCK,
            0,
            1,
            0,
            &mut overlapped,
        )
    };
    if locked != 0 {
        Ok(true)
    } else {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_LOCK_VIOLATION) {
            Ok(false)
        } else {
            Err(error)
        }
    }
}

#[cfg(windows)]
fn unlock_file(file: &File) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    unsafe extern "system" {
        fn UnlockFileEx(
            file: *mut std::ffi::c_void,
            reserved: u32,
            bytes_low: u32,
            bytes_high: u32,
            overlapped: *mut Overlapped,
        ) -> i32;
    }
    let mut overlapped = Overlapped {
        internal: 0,
        internal_high: 0,
        offset_or_pointer: 0,
        event: std::ptr::null_mut(),
    };
    if unsafe { UnlockFileEx(file.as_raw_handle(), 0, 1, 0, &mut overlapped) } != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
