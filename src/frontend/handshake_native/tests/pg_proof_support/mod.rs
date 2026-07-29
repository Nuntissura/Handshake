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
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use handshake_native::backend_client::build_backend_client;
use sha2::{Digest, Sha256};

const FIXTURE_LOCK_TIMEOUT: Duration = Duration::from_secs(60);
// Every owned backend replays the complete production startup path before it can publish its listen
// report. On a shared PostgreSQL cluster, another worktree can hold the migration advisory lock and
// serialize schema/corpus work for more than five minutes. Keep one aggregate startup deadline across
// listen-report + health readiness, separate from every measured route budget and capped by the
// supervisor-injected command-wide deadline.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(1200);
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(1620);
const COMMAND_DEADLINE_ENV: &str = "HSK_MT045_COMMAND_DEADLINE_UNIX_MS";
const COMMAND_DEADLINE_QPC_ENV: &str = "HSK_MT045_COMMAND_DEADLINE_QPC_TICKS";
const COMMAND_BUDGET_ENV: &str = "HSK_MT045_COMMAND_BUDGET_MS";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const SHUTDOWN_GRACE_TIMEOUT: Duration = Duration::from_secs(10);
const HELPER_REAP_RESERVE: Duration = Duration::from_secs(2);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);
// Contract-sized 5k-row corpora traverse public validation, PostgreSQL, EventLedger, projections, and
// search refresh at the product's measured ~12-writes/s write ceiling, so large corpora (LK-03's 10k
// rows) legitimately need the full bounded setup window to SEED. Setup is explicitly OUTSIDE measured
// query time (it never affects a budget). The default and maximum are identical for the canonical proof;
// an optional lower value can fail faster locally but can never widen the canonical setup allowance.
const DEFAULT_SETUP_TIMEOUT: Duration = Duration::from_secs(1200);
const MAX_SETUP_TIMEOUT: Duration = Duration::from_secs(1200);
static PROOF_COMMAND_DEADLINE: OnceLock<Instant> = OnceLock::new();

pub const DEFAULT_BASE: &str = "http://127.0.0.1:37501";

pub struct LiveBackend {
    pub base: String,
    pub workspace_id: String,
    client: reqwest::Client,
    rt: tokio::runtime::Runtime,
    owned_backend: Option<Child>,
    owned_binary: Option<PathBuf>,
    owned_data_dir: Option<PathBuf>,
    owned_runtime_roots: Vec<PathBuf>,
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
        let configured_timeout = std::env::var("HSK_PROOF_SETUP_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_SETUP_TIMEOUT)
            .min(MAX_SETUP_TIMEOUT);
        let started = Instant::now();
        let timeout = configured_timeout.min(command_time_remaining(started));
        assert!(
            !timeout.is_zero(),
            "fixture setup cannot start after the command-wide proof deadline"
        );
        Self {
            label: label.into(),
            started,
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

fn proof_command_deadline() -> Instant {
    *PROOF_COMMAND_DEADLINE.get_or_init(|| {
        let now = Instant::now();
        let authorized_budget_ms = std::env::var(COMMAND_BUDGET_ENV)
            .ok()
            .map(|value| {
                value.parse::<u64>().unwrap_or_else(|error| {
                    panic!("{COMMAND_BUDGET_ENV} must be an unsigned millisecond duration: {error}")
                })
            })
            .unwrap_or_else(|| {
                u64::try_from(DEFAULT_COMMAND_TIMEOUT.as_millis())
                    .expect("default command timeout milliseconds fit u64")
            })
            .min(
                u64::try_from(DEFAULT_COMMAND_TIMEOUT.as_millis())
                    .expect("default command timeout milliseconds fit u64"),
            );
        if let Some(monotonic_remaining) = windows_qpc_remaining() {
            return now + monotonic_remaining.min(Duration::from_millis(authorized_budget_ms));
        }
        let Some(raw_deadline) = std::env::var(COMMAND_DEADLINE_ENV).ok() else {
            return now + Duration::from_millis(authorized_budget_ms);
        };
        let deadline_unix_ms = raw_deadline.parse::<u128>().unwrap_or_else(|error| {
            panic!("{COMMAND_DEADLINE_ENV} must be an unsigned Unix millisecond timestamp: {error}")
        });
        let now_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after Unix epoch")
            .as_millis();
        assert!(
            deadline_unix_ms > now_unix_ms,
            "{COMMAND_DEADLINE_ENV} expired before the proof process started"
        );
        let remaining_ms = u64::try_from(deadline_unix_ms - now_unix_ms)
            .expect("command deadline remaining milliseconds fit u64");
        // The supervisor normally injects a shorter exact remainder. The clamp is a fail-closed guard
        // against an inherited far-future value or a backward wall-clock adjustment before this process
        // converts the absolute timestamp to its monotonic deadline.
        now + Duration::from_millis(remaining_ms.min(authorized_budget_ms))
    })
}

#[cfg(windows)]
fn windows_qpc_remaining() -> Option<Duration> {
    #[link(name = "Kernel32")]
    extern "system" {
        fn QueryPerformanceCounter(value: *mut i64) -> i32;
        fn QueryPerformanceFrequency(value: *mut i64) -> i32;
    }

    let raw_deadline = std::env::var(COMMAND_DEADLINE_QPC_ENV).ok()?;
    let deadline_ticks = raw_deadline.parse::<i128>().unwrap_or_else(|error| {
        panic!(
            "{COMMAND_DEADLINE_QPC_ENV} must be a signed QueryPerformanceCounter tick value: {error}"
        )
    });
    let mut current_ticks = 0_i64;
    let mut frequency = 0_i64;
    // SAFETY: both Windows APIs write one i64 into the provided valid stack pointer.
    let counter_ok = unsafe { QueryPerformanceCounter(&mut current_ticks) };
    let frequency_ok = unsafe { QueryPerformanceFrequency(&mut frequency) };
    assert!(
        counter_ok != 0 && frequency_ok != 0 && frequency > 0,
        "Windows QueryPerformanceCounter/Frequency must be available"
    );
    let remaining_ticks = deadline_ticks.saturating_sub(i128::from(current_ticks));
    if remaining_ticks <= 0 {
        return Some(Duration::ZERO);
    }
    let remaining_nanos = (u128::try_from(remaining_ticks).expect("positive QPC ticks fit u128")
        * 1_000_000_000_u128)
        / u128::try_from(frequency).expect("positive QPC frequency fits u128");
    let remaining_nanos = u64::try_from(remaining_nanos).unwrap_or(u64::MAX);
    Some(Duration::from_nanos(remaining_nanos))
}

#[cfg(not(windows))]
fn windows_qpc_remaining() -> Option<Duration> {
    None
}

fn command_time_remaining(now: Instant) -> Duration {
    proof_command_deadline()
        .checked_duration_since(now)
        .unwrap_or(Duration::ZERO)
}

fn bounded_command_deadline(phase_timeout: Duration) -> Instant {
    let now = Instant::now();
    (now + phase_timeout).min(proof_command_deadline())
}

fn proof_request_timeout(maximum: Duration) -> Option<Duration> {
    let remaining = command_time_remaining(Instant::now());
    (!remaining.is_zero()).then(|| maximum.min(remaining))
}

struct PendingChild(Option<Child>);

struct RemoveFileOnDrop(PathBuf);

impl Drop for RemoveFileOnDrop {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.0) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => eprintln!(
                "WARN: failed to remove transient fixture input {}: {error}",
                self.0.display()
            ),
        }
    }
}

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
            if let Err(error) = force_kill_tree_and_reap(child, "drop pending fixture backend") {
                eprintln!("FATAL: {error}");
                std::process::abort();
            }
        }
    }
}

fn kill_and_reap(child: &mut Child, operation: &str) {
    force_kill_tree_and_reap(child, operation).unwrap_or_else(|error| panic!("{error}"));
}

fn force_kill_tree_and_reap(child: &mut Child, operation: &str) -> Result<(), String> {
    force_kill_tree_and_reap_before(child, operation, Instant::now() + SHUTDOWN_TIMEOUT)
}

fn force_kill_tree_and_reap_before(
    child: &mut Child,
    operation: &str,
    cleanup_deadline: Instant,
) -> Result<(), String> {
    let pid = child.id();
    if child
        .try_wait()
        .map_err(|error| format!("{operation}: poll owned backend pid {pid}: {error}"))?
        .is_some()
    {
        return Ok(());
    }
    child
        .kill()
        .map_err(|error| format!("{operation}: kill owned backend pid {pid}: {error}"))?;
    let graceful_deadline = (Instant::now() + SHUTDOWN_GRACE_TIMEOUT).min(cleanup_deadline);
    if wait_for_owned_exit_before(child, graceful_deadline)? {
        return Ok(());
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        let mut taskkill = no_window(&mut command)
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("{operation}: force-kill tree for pid {pid}: {error}"))?;
        let taskkill_status = wait_for_process_before(&mut taskkill, cleanup_deadline)?
            .ok_or_else(|| format!("{operation}: taskkill for owned pid {pid} timed out"))?;
        if !taskkill_status.success() {
            return Err(format!(
                "{operation}: taskkill for owned pid {pid} failed with {taskkill_status}"
            ));
        }
        if wait_for_owned_exit_before(child, cleanup_deadline)? {
            return Ok(());
        }
    }
    Err(format!(
        "{operation}: owned backend pid {pid} remained alive after bounded force termination"
    ))
}

fn wait_for_process_before(
    child: &mut Child,
    deadline: Instant,
) -> Result<Option<std::process::ExitStatus>, String> {
    let run_deadline = deadline
        .checked_sub(HELPER_REAP_RESERVE)
        .filter(|candidate| *candidate > Instant::now())
        .unwrap_or_else(Instant::now);
    loop {
        match child
            .try_wait()
            .map_err(|error| format!("poll helper process: {error}"))?
        {
            Some(status) => return Ok(Some(status)),
            None if Instant::now() < run_deadline => thread::sleep(Duration::from_millis(50)),
            None => {
                child
                    .kill()
                    .map_err(|error| format!("kill timed-out helper process: {error}"))?;
                loop {
                    match child
                        .try_wait()
                        .map_err(|error| format!("reap killed helper process: {error}"))?
                    {
                        Some(_) => return Ok(None),
                        None if Instant::now() < deadline => {
                            thread::sleep(Duration::from_millis(25))
                        }
                        None => {
                            return Err(
                                "killed helper process did not exit before its reap deadline"
                                    .to_owned(),
                            )
                        }
                    }
                }
            }
        }
    }
}

fn wait_for_owned_exit_before(child: &mut Child, deadline: Instant) -> Result<bool, String> {
    let pid = child.id();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(true),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => return Ok(false),
            Err(error) => return Err(format!("reap owned backend pid {pid}: {error}")),
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
    let lock_timeout = FIXTURE_LOCK_TIMEOUT.min(command_time_remaining(Instant::now()));
    assert!(
        !lock_timeout.is_zero(),
        "managed backend fixture lock cannot start after the command-wide proof deadline"
    );
    let lock = FileLock::acquire(
        &external_artifact_root().join("managed-backend-fixture.lock"),
        lock_timeout,
    )
    .unwrap_or_else(|| {
        panic!(
            "managed backend fixture lock timed out after {}s",
            lock_timeout.as_secs()
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
    let mut owned_runtime_roots = Vec::new();
    if force_owned || !healthy(&rt, &client, &configured_base, proof_command_deadline()) {
        if !force_owned {
            assert_eq!(
                configured_base, DEFAULT_BASE,
                "HSK_TEST_BASE={configured_base} is attach-only and cannot be replaced by an owned backend"
            );
        }
        let binary = resolve_backend_binary();
        let (child, report_path, data_dir) = spawn_backend(&binary);
        let mut pending = PendingChild::new(child);
        let startup_deadline = bounded_command_deadline(STARTUP_TIMEOUT);
        base = wait_for_listen_report(pending.child_mut(), &report_path, startup_deadline);
        wait_for_health(&rt, &client, &base, pending.child_mut(), startup_deadline);
        owned_backend = Some(pending.take());
        owned_binary = Some(binary);
        owned_runtime_roots.push(
            data_dir
                .parent()
                .expect("owned backend data directory has a runtime root")
                .to_path_buf(),
        );
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
        owned_runtime_roots,
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

fn wait_for_listen_report(
    child: &mut Child,
    report_path: &Path,
    startup_deadline: Instant,
) -> String {
    loop {
        match std::fs::read(report_path) {
            Ok(bytes) => {
                let report: serde_json::Value = match serde_json::from_slice(&bytes) {
                    Ok(report) => report,
                    Err(error)
                        if error.classify() == serde_json::error::Category::Eof
                            && Instant::now() < startup_deadline =>
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
            Instant::now() < startup_deadline,
            "owned handshake_core did not publish listen report before its shared startup/command deadline (maximum {}s)",
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

fn healthy(
    rt: &tokio::runtime::Runtime,
    client: &reqwest::Client,
    base: &str,
    deadline: Instant,
) -> bool {
    let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
        return false;
    };
    if remaining.is_zero() {
        return false;
    }
    let url = format!("{base}/health");
    rt.block_on(async {
        match client
            .get(url)
            .timeout(Duration::from_secs(2).min(remaining))
            .send()
            .await
        {
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
    startup_deadline: Instant,
) {
    loop {
        if healthy(rt, client, base, startup_deadline) {
            return;
        }
        if let Some(status) = child.try_wait().expect("poll owned backend") {
            panic!("owned handshake_core exited before health with {status}");
        }
        assert!(
            Instant::now() < startup_deadline,
            "owned handshake_core did not become healthy before its shared startup/command deadline (maximum {}s)",
            STARTUP_TIMEOUT.as_secs()
        );
        thread::sleep(Duration::from_millis(200));
    }
}

impl LiveBackend {
    pub fn owned_backend_binding_receipt(&self) -> serde_json::Value {
        let child = self
            .owned_backend
            .as_ref()
            .expect("canonical proof requires a fixture-owned backend");
        let binary = self
            .owned_binary
            .as_ref()
            .expect("owned backend records its exact binary");
        let bytes = std::fs::read(binary)
            .unwrap_or_else(|error| panic!("hash owned backend {}: {error}", binary.display()));
        let dsn = std::env::var("HANDSHAKE_TEST_PG_DSN")
            .expect("owned backend proof requires HANDSHAKE_TEST_PG_DSN");
        let parsed = reqwest::Url::parse(&dsn).expect("parse HANDSHAKE_TEST_PG_DSN");
        serde_json::json!({
            "owned": true,
            "base_url": self.base,
            "backend_pid": child.id(),
            "backend_binary": binary,
            "backend_binary_sha256": format!("{:x}", Sha256::digest(bytes)),
            "database_host": parsed.host_str(),
            "database_port": parsed.port_or_known_default(),
            "database_name": parsed.path().trim_start_matches('/'),
            "runtime_data_dir": self.owned_data_dir,
        })
    }

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
        let restart_deadline = bounded_command_deadline(STARTUP_TIMEOUT);
        assert!(
            Instant::now() < restart_deadline,
            "owned backend restart cannot begin after the command-wide proof deadline"
        );
        let child = self
            .owned_backend
            .as_mut()
            .expect("restart_owned requires a fixture-owned backend");
        let old_base = self.base.clone();
        let old_child_deadline = (Instant::now() + SHUTDOWN_TIMEOUT).min(restart_deadline);
        force_kill_tree_and_reap_before(
            child,
            "restart exact fixture-owned backend",
            old_child_deadline,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        self.owned_backend = None;
        assert!(
            Instant::now() < restart_deadline,
            "owned backend restart exhausted its command-wide deadline during shutdown"
        );

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
        self.owned_runtime_roots.push(
            report_path
                .parent()
                .expect("replacement listen report has a runtime root")
                .to_path_buf(),
        );
        let mut pending = PendingChild::new(replacement);
        let new_base = wait_for_listen_report(pending.child_mut(), &report_path, restart_deadline);
        wait_for_health(
            &self.rt,
            &self.client,
            &new_base,
            pending.child_mut(),
            restart_deadline,
        );
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
            healthy(&self.rt, &self.client, &self.base, proof_command_deadline()),
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
        let sql_path = log_dir.join(format!("{label}-{run_id}.sql"));
        let _sql_input_guard = RemoveFileOnDrop(sql_path.clone());
        let mut sql_file = File::create(&sql_path).expect("create fixture SQL input");
        sql_file
            .write_all(sql.as_bytes())
            .unwrap_or_else(|error| panic!("{label}: write psql fixture file: {error}"));
        sql_file
            .flush()
            .unwrap_or_else(|error| panic!("{label}: flush psql fixture file: {error}"));
        drop(sql_file);
        setup_deadline.check();
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
            .arg("--file")
            .arg(&sql_path)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("PGCONNECT_TIMEOUT", "5");
        let child = no_window(&mut command)
            .spawn()
            .unwrap_or_else(|error| panic!("{label}: start psql batch fixture: {error}"));
        let mut pending = PendingChild::new(child);

        let deadline = Instant::now() + setup_deadline.remaining();
        loop {
            match pending.child_mut().try_wait() {
                Ok(Some(status)) => {
                    let _ = pending.take();
                    assert!(
                        status.success(),
                        "{label}: psql batch fixture failed with {status}; stderr={}",
                        stderr_path.display()
                    );
                    break;
                }
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
                Ok(None) => {
                    force_kill_tree_and_reap(pending.child_mut(), "time out psql batch fixture")
                        .unwrap_or_else(|error| {
                            let _ = std::fs::remove_file(&sql_path);
                            eprintln!("FATAL: {error}");
                            std::process::abort();
                        });
                    let _ = pending.take();
                    panic!(
                        "{label}: psql batch fixture exceeded hard {}s deadline and was killed/reaped; stderr={}",
                        setup_deadline.timeout_secs(),
                        stderr_path.display()
                    );
                }
                Err(error) => {
                    force_kill_tree_and_reap(pending.child_mut(), "poll failed psql batch fixture")
                        .unwrap_or_else(|cleanup_error| {
                            let _ = std::fs::remove_file(&sql_path);
                            eprintln!("FATAL: {cleanup_error}");
                            std::process::abort();
                        });
                    let _ = pending.take();
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
        let workspace_cleanup = if !self.workspace_id.is_empty() {
            let workspace_id = std::mem::take(&mut self.workspace_id);
            Some((
                workspace_id.clone(),
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.delete_workspace(&workspace_id)
                })),
            ))
        } else {
            None
        };
        if let Some(child) = self.owned_backend.as_mut() {
            kill_and_reap(child, "clean up fixture-owned backend");
            self.owned_backend = None;
        }
        if let Some((workspace_id, result)) = workspace_cleanup {
            match result {
                Ok(status) => assert!(
                    (200..300).contains(&status) || status == 404,
                    "managed fixture workspace cleanup {workspace_id} returned {status}"
                ),
                Err(payload) => std::panic::resume_unwind(payload),
            }
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
        let timeout = proof_request_timeout(REQUEST_TIMEOUT).unwrap_or_else(|| {
            panic!("GET {url} cannot start after the command-wide proof deadline")
        });
        let (status, bytes) = self.rt.block_on(async {
            let response = self
                .ident(self.client.get(&url))
                .timeout(timeout)
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
        let deadline = bounded_command_deadline(Duration::from_secs(10));
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
        let deadline = bounded_command_deadline(Duration::from_secs(10));
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
        let deadline = bounded_command_deadline(Duration::from_secs(10));
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
        let Some(timeout) = proof_request_timeout(REQUEST_TIMEOUT) else {
            return 0;
        };
        self.rt.block_on(async {
            request
                .timeout(timeout)
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
        let timeout = proof_request_timeout(REQUEST_TIMEOUT).unwrap_or_else(|| {
            panic!("{label} cannot start after the command-wide proof deadline")
        });
        self.rt.block_on(async {
            let response = request
                .timeout(timeout)
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
            let workspace_id = std::mem::take(&mut self.workspace_id);
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.delete_workspace(&workspace_id)
            })) {
                Ok(status) if (200..300).contains(&status) || status == 404 => {}
                Ok(status) => {
                    eprintln!(
                        "WARN: managed fixture workspace cleanup {workspace_id} returned {status}"
                    );
                }
                Err(_) => {
                    eprintln!(
                        "WARN: managed fixture workspace cleanup {workspace_id} panicked during Drop"
                    );
                }
            }
        }
        if let Some(child) = self.owned_backend.as_mut() {
            if let Err(error) = force_kill_tree_and_reap(child, "drop fixture-owned backend") {
                eprintln!("FATAL: {error}");
                std::process::abort();
            }
            self.owned_backend = None;
        }
        self.owned_data_dir.take();
        for runtime_root in self.owned_runtime_roots.drain(..).rev() {
            if let Err(error) = std::fs::remove_dir_all(&runtime_root) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!(
                        "FATAL: remove fixture-owned runtime root {}: {error}",
                        runtime_root.display()
                    );
                    std::process::abort();
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
