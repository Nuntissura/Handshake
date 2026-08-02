//! Real managed-PostgreSQL + handshake_core fixture shared by native-editor integration proofs.
//!
//! The frontend/backend Cargo graphs intentionally stay separate (their tree-sitter ABI lines differ).
//! The fixture attaches to `HSK_TEST_BASE` when a healthy root-managed product backend is present, or
//! starts the explicitly-built product executable named by `HSK_TEST_BACKEND_BIN`. Stage proofs always
//! use an owned process so the backend inherits their private discovery-binding root. It never invokes Cargo.
//! An owned process is killed on drop; an attached process is never touched. Every proof creates its own
//! workspace through production HTTP and deletes that workspace before releasing the fixture lock.

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::error::Error as StdError;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
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
    owned_backend: RefCell<Option<Child>>,
    owned_binary: Option<PathBuf>,
    owned_data_dir: Option<PathBuf>,
    owned_runtime_roots: Vec<PathBuf>,
    retained_failure_receipt: RefCell<Option<PathBuf>>,
    preserve_runtime_roots: Cell<bool>,
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
        owned_backend: RefCell::new(owned_backend),
        owned_binary,
        owned_data_dir,
        owned_runtime_roots,
        retained_failure_receipt: RefCell::new(None),
        preserve_runtime_roots: Cell::new(false),
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
        if backend.owned_backend.borrow().is_some() {
            let runtime_root = backend
                .owned_runtime_roots
                .last()
                .expect("owned workspace fixture has an active runtime root");
            let marker_path = runtime_root.join("workspace-identity.json");
            let marker = serde_json::json!({
                "schema_id": "hsk.wp_kernel_012.mt045_workspace_identity@1",
                "run_id": std::env::var("HSK_MT045_RUN_ID").unwrap_or_else(|_| "standalone-run".to_owned()),
                "scenario_identity": thread::current().name().unwrap_or("unnamed-test-thread"),
                "workspace_id": backend.workspace_id.clone(),
                "owned_backend_pid": backend.owned_backend.borrow().as_ref().map(Child::id),
            });
            write_new_atomic(
                &marker_path,
                &serde_json::to_vec_pretty(&marker)
                    .expect("serialize owned workspace identity marker"),
            )
            .unwrap_or_else(|error| panic!("publish owned workspace identity marker: {error}"));
        }
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
    let run_id = safe_artifact_component(
        &std::env::var("HSK_MT045_RUN_ID").unwrap_or_else(|_| "standalone-run".to_owned()),
        "standalone-run",
    );
    let scenario = safe_artifact_component(
        thread::current().name().unwrap_or("unnamed-test-thread"),
        "unnamed-test-thread",
    );
    let run_root = external_artifact_root()
        .join("backend-runtime")
        .join(run_id)
        .join(scenario)
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&run_root).expect("create backend runtime artifact directory");
    restrict_runtime_directory(&run_root)
        .unwrap_or_else(|error| panic!("restrict backend runtime directory: {error}"));
    let data_dir = existing_data_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| run_root.join("data"));
    if existing_data_dir.is_none() {
        std::fs::create_dir(&data_dir).expect("create owned backend data directory");
        restrict_runtime_directory(&data_dir)
            .unwrap_or_else(|error| panic!("restrict backend data directory: {error}"));
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

fn restrict_runtime_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("chmod 0700 {}: {error}", path.display()))?;
    }
    #[cfg(windows)]
    {
        let user = match (
            std::env::var("USERDOMAIN").ok(),
            std::env::var("USERNAME").ok(),
        ) {
            (Some(domain), Some(user)) if !domain.is_empty() && !user.is_empty() => {
                format!(r"{domain}\{user}")
            }
            (_, Some(user)) if !user.is_empty() => user,
            _ => return Err("USERNAME is unavailable for restrictive Windows ACL".to_owned()),
        };
        let grant = format!(r"{user}:(OI)(CI)F");
        let mut command = Command::new("icacls");
        let output = no_window(&mut command)
            .arg(path)
            .args(["/inheritance:r", "/grant:r", &grant, "/Q"])
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("run icacls for {}: {error}", path.display()))?;
        if !output.status.success() {
            return Err(format!(
                "icacls failed for {} with {}: {}",
                path.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    Ok(())
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
    pub fn owned_runtime_roots_for_proof(&self) -> Vec<PathBuf> {
        self.owned_runtime_roots
            .iter()
            .map(|path| {
                std::fs::canonicalize(path).unwrap_or_else(|error| {
                    panic!(
                        "canonicalize owned backend runtime root {}: {error}",
                        path.display()
                    )
                })
            })
            .collect()
    }

    pub fn owned_backend_binding_receipt(&self) -> serde_json::Value {
        let owned_backend = self.owned_backend.borrow();
        let child = owned_backend
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
            .borrow()
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
            .get_mut()
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
        *self.owned_backend.get_mut() = None;
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
        *self.owned_backend.get_mut() = Some(pending.take());
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
        if let Some(child) = self.owned_backend.get_mut().as_mut() {
            kill_and_reap(child, "clean up fixture-owned backend");
            *self.owned_backend.get_mut() = None;
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

    /// Complete the normal proof teardown and atomically publish the one active backend runtime set.
    /// The HTTP workspace delete happens while the fixture-owned backend is alive; `assert_cleanup`
    /// then reaps only that owned child. Publication starts only after reap. A partial publication keeps
    /// every source runtime root for recovery, while a complete publication removes the UUID leaves and
    /// their now-empty run/scenario parents.
    pub fn assert_cleanup_and_publish_runtime_diagnostics(
        &mut self,
        scenario_id: &str,
    ) -> Result<serde_json::Value, String> {
        // Preserve all owned runtime evidence before the first fallible lookup or request. Only a
        // complete publication plus verified source cleanup clears this guard.
        self.preserve_runtime_roots.set(true);
        let workspace_cleanup = if !self.workspace_id.is_empty() {
            let workspace_id = self.workspace_id.clone();
            let status = self.delete_workspace(&workspace_id);
            if !(200..300).contains(&status) && status != 404 {
                return Err(format!(
                    "managed fixture workspace cleanup {workspace_id} returned {status}"
                ));
            }
            self.workspace_id.clear();
            serde_json::json!({
                "status": "http_delete_completed_before_owned_reap",
                "workspace_id": workspace_id,
                "http_status": status,
            })
        } else {
            serde_json::json!({
                "status": "no_workspace",
                "workspace_id": null,
                "http_status": null,
            })
        };
        let active_runtime_root = self.owned_runtime_roots.last().cloned().ok_or_else(|| {
            "success runtime publication requires an active runtime root".to_owned()
        })?;
        let child = self.owned_backend.get_mut().as_mut().ok_or_else(|| {
            "success runtime publication requires a fixture-owned backend".to_owned()
        })?;
        let owned_pid = child.id();
        force_kill_tree_and_reap(child, "publish successful fixture runtime diagnostics")?;
        let exit_status = child
            .try_wait()
            .map_err(|error| format!("poll reaped fixture-owned backend pid {owned_pid}: {error}"))?
            .ok_or_else(|| {
                format!("fixture-owned backend pid {owned_pid} was not reaped after termination")
            })?;
        *self.owned_backend.get_mut() = None;

        let outcome = publish_success_runtime_diagnostics(
            &active_runtime_root,
            scenario_id,
            serde_json::json!({
                "owned": true,
                "pid": owned_pid,
                "try_wait": "reaped_by_assert_cleanup",
                "termination": "terminated_and_reaped",
                "exit_code": exit_status.code(),
                "success": exit_status.success(),
            }),
            workspace_cleanup,
        )?;
        if outcome.complete {
            self.owned_data_dir.take();
            for runtime_root in self.owned_runtime_roots.drain(..).rev() {
                remove_runtime_root_and_empty_parents(&runtime_root)?;
            }
            self.preserve_runtime_roots.set(false);
        }
        Ok(outcome.receipt)
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

    /// GET counterpart of [`Self::post_json_response`]: preserves a typed non-success response so
    /// negative route validation proofs can assert the exact status and error contract.
    pub fn get_json_response(&self, path: &str) -> (u16, serde_json::Value) {
        let label = format!("GET {path}");
        let (status, text) = self.request_text_response(
            self.ident(self.client.get(format!("{}{path}", self.base))),
            &label,
        );
        let value = if text.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        };
        (status, value)
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
        let result = self.rt.block_on(async {
            let response = request
                .timeout(timeout)
                .send()
                .await
                .map_err(|error| ("request_send", error))?;
            let status = response.status().as_u16();
            let text = response
                .text()
                .await
                .map_err(|error| ("response_body", error))?;
            Ok::<_, (&'static str, reqwest::Error)>((status, text))
        });
        match result {
            Ok(response) => response,
            Err((stage, error)) => {
                let structured_error = reqwest_error_receipt(&error);
                let retained =
                    self.retain_failure_diagnostics("request_failure", stage, label, Some(&error));
                match retained {
                    Ok(receipt) => panic!(
                        "{label} failed at {stage}: {error}; failure_diagnostics={}; reqwest={structured_error}",
                        receipt.display()
                    ),
                    Err(retention_error) => panic!(
                        "{label} failed at {stage}: {error}; failure_diagnostics_error={retention_error}; reqwest={structured_error}"
                    ),
                }
            }
        }
    }

    fn retain_failure_diagnostics(
        &self,
        trigger: &str,
        stage: &str,
        label: &str,
        request_error: Option<&reqwest::Error>,
    ) -> Result<PathBuf, String> {
        if let Some(receipt) = self.retained_failure_receipt.borrow().as_ref() {
            return Ok(receipt.clone());
        }

        let health = self.immediate_health_snapshot();
        let mut stable_logs = false;
        let process = {
            let mut child_slot = self.owned_backend.borrow_mut();
            let mut clear_child = false;
            let receipt = match child_slot.as_mut() {
                Some(child) => {
                    let pid = child.id();
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            clear_child = true;
                            stable_logs = true;
                            serde_json::json!({
                                "owned": true,
                                "pid": pid,
                                "try_wait": "exited",
                                "exit_code": status.code(),
                                "success": status.success(),
                                "termination": "already_exited_and_reaped",
                                "termination_error": null,
                            })
                        }
                        Ok(None) => match force_kill_tree_and_reap(
                            child,
                            "stabilize fixture-owned backend failure diagnostics",
                        ) {
                            Ok(()) => match child.try_wait() {
                                Ok(Some(status)) => {
                                    clear_child = true;
                                    stable_logs = true;
                                    serde_json::json!({
                                        "owned": true,
                                        "pid": pid,
                                        "try_wait": "running",
                                        "exit_code": status.code(),
                                        "success": status.success(),
                                        "termination": "terminated_and_reaped",
                                        "termination_error": null,
                                    })
                                }
                                Ok(None) => serde_json::json!({
                                    "owned": true,
                                    "pid": pid,
                                    "try_wait": "running",
                                    "exit_code": null,
                                    "success": null,
                                    "termination": "termination_returned_without_reap",
                                    "termination_error": "owned backend still running after termination helper",
                                }),
                                Err(error) => serde_json::json!({
                                    "owned": true,
                                    "pid": pid,
                                    "try_wait": "running",
                                    "exit_code": null,
                                    "success": null,
                                    "termination": "post_termination_poll_failed",
                                    "termination_error": error.to_string(),
                                }),
                            },
                            Err(error) => serde_json::json!({
                                "owned": true,
                                "pid": pid,
                                "try_wait": "running",
                                "exit_code": null,
                                "success": null,
                                "termination": "termination_failed",
                                "termination_error": error,
                            }),
                        },
                        Err(error) => serde_json::json!({
                            "owned": true,
                            "pid": pid,
                            "try_wait": "error",
                            "exit_code": null,
                            "success": null,
                            "termination": "not_attempted_after_poll_error",
                            "termination_error": null,
                            "error": error.to_string(),
                        }),
                    }
                }
                None => serde_json::json!({
                    "owned": false,
                    "pid": null,
                    "try_wait": "not_owned_or_already_reaped",
                    "exit_code": null,
                    "success": null,
                    "termination": "not_applicable",
                    "termination_error": null,
                }),
            };
            if clear_child {
                *child_slot = None;
            }
            receipt
        };
        let request_error = request_error.map(reqwest_error_receipt);
        let active_runtime_root = self
            .owned_runtime_roots
            .last()
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();
        let workspace_cleanup = if stable_logs && process["owned"] == true {
            match active_runtime_root.first() {
                Some(runtime_root) => {
                    cleanup_workspace_after_backend_reap(&self.workspace_id, runtime_root)
                }
                None => serde_json::json!({
                    "status": "failed",
                    "error": "owned backend has no active runtime root for bounded psql cleanup",
                }),
            }
        } else if process["owned"] == false {
            serde_json::json!({
                "status": "deferred_attached_backend",
                "error": null,
            })
        } else {
            serde_json::json!({
                "status": "failed",
                "error": "workspace cleanup cannot run until the fixture-owned backend is confirmed reaped",
            })
        };
        let outcome = retain_backend_failure_files(
            &active_runtime_root,
            trigger,
            stage,
            label,
            process,
            health,
            request_error,
            stable_logs,
            workspace_cleanup,
        );
        match outcome {
            Ok(outcome) => {
                if !outcome.complete {
                    self.preserve_runtime_roots.set(true);
                }
                *self.retained_failure_receipt.borrow_mut() = Some(outcome.receipt_path.clone());
                Ok(outcome.receipt_path)
            }
            Err(error) => {
                self.preserve_runtime_roots.set(true);
                Err(error)
            }
        }
    }

    fn immediate_health_snapshot(&self) -> serde_json::Value {
        let url = format!("{}/health", self.base);
        let result = self.rt.block_on(async {
            let response = self
                .client
                .get(&url)
                .timeout(Duration::from_secs(2))
                .send()
                .await?;
            let status = response.status().as_u16();
            let text = response.text().await?;
            Ok::<_, reqwest::Error>((status, text))
        });
        match result {
            Ok((status, text)) => serde_json::json!({
                "url": url,
                "reachable": true,
                "http_status": status,
                "body": serde_json::from_str::<serde_json::Value>(&text)
                    .unwrap_or_else(|_| serde_json::Value::String(text)),
            }),
            Err(error) => serde_json::json!({
                "url": url,
                "reachable": false,
                "http_status": null,
                "error": reqwest_error_receipt(&error),
            }),
        }
    }
}

fn cleanup_workspace_after_backend_reap(
    workspace_id: &str,
    runtime_root: &Path,
) -> serde_json::Value {
    if workspace_id.is_empty() {
        return serde_json::json!({
            "status": "no_workspace",
            "workspace_id": null,
            "verified_absent": true,
            "remaining_workspace_count": 0,
            "error": null,
        });
    }
    let Some(database_url) = std::env::var("HANDSHAKE_TEST_PG_DSN")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return serde_json::json!({
            "status": "failed",
            "workspace_id": workspace_id,
            "error": "HANDSHAKE_TEST_PG_DSN is unavailable for post-reap workspace cleanup",
        });
    };
    let psql = std::env::var_os("HSK_PSQL_BIN").unwrap_or_else(|| "psql".into());
    let stdout_path = runtime_root.join("workspace-cleanup.stdout.log");
    let stderr_path = runtime_root.join("workspace-cleanup.stderr.log");
    let stdout = match File::create(&stdout_path) {
        Ok(file) => file,
        Err(error) => {
            return serde_json::json!({
                "status": "failed",
                "workspace_id": workspace_id,
                "error": format!("create post-reap cleanup stdout: {error}"),
            });
        }
    };
    let stderr = match File::create(&stderr_path) {
        Ok(file) => file,
        Err(error) => {
            return serde_json::json!({
                "status": "failed",
                "workspace_id": workspace_id,
                "error": format!("create post-reap cleanup stderr: {error}"),
            });
        }
    };
    let literal = workspace_id.replace('\'', "''");
    let sql = format!(
        "DELETE FROM workspaces WHERE id = '{literal}'; SELECT COUNT(*) FROM workspaces WHERE id = '{literal}';"
    );
    let mut command = Command::new(&psql);
    command
        .current_dir(runtime_root)
        .arg("--no-psqlrc")
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--quiet")
        .arg("--tuples-only")
        .arg("--no-align")
        .arg("--dbname")
        .arg(&database_url)
        .arg("--command")
        .arg(sql)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .env("PGCONNECT_TIMEOUT", "5");
    let child = match no_window(&mut command).spawn() {
        Ok(child) => child,
        Err(error) => {
            return serde_json::json!({
                "status": "failed",
                "workspace_id": workspace_id,
                "psql": psql,
                "error": format!("start bounded post-reap psql cleanup: {error}"),
            });
        }
    };
    let pid = child.id();
    let mut pending = PendingChild::new(child);
    let deadline = Instant::now() + Duration::from_secs(30);
    let exit_status = loop {
        match pending.child_mut().try_wait() {
            Ok(Some(status)) => {
                let _ = pending.take();
                break Ok(status);
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let cleanup = force_kill_tree_and_reap(
                    pending.child_mut(),
                    "time out post-reap workspace cleanup psql",
                );
                let _ = pending.take();
                break Err(format!(
                    "post-reap workspace cleanup psql timed out; reap={cleanup:?}"
                ));
            }
            Err(error) => {
                let cleanup = force_kill_tree_and_reap(
                    pending.child_mut(),
                    "poll failed post-reap workspace cleanup psql",
                );
                let _ = pending.take();
                break Err(format!(
                    "poll post-reap workspace cleanup psql: {error}; reap={cleanup:?}"
                ));
            }
        }
    };
    let stdout_text = std::fs::read_to_string(&stdout_path).unwrap_or_default();
    let stderr_text = std::fs::read_to_string(&stderr_path).unwrap_or_default();
    let verified_absent =
        exit_status.as_ref().is_ok_and(|status| status.success()) && stdout_text.trim() == "0";
    serde_json::json!({
        "status": if verified_absent { "deleted_and_verified_absent" } else { "failed" },
        "workspace_id": workspace_id,
        "psql": psql,
        "psql_pid": pid,
        "exit_code": exit_status.as_ref().ok().and_then(|status| status.code()),
        "success": exit_status.as_ref().ok().map(std::process::ExitStatus::success),
        "verified_absent": verified_absent,
        "remaining_workspace_count": if verified_absent { Some(0_u64) } else { None },
        "stdout_path": stdout_path,
        "stdout_sha256": sha256_file(&stdout_path).ok(),
        "stdout_tail": stdout_text.lines().rev().take(8).collect::<Vec<_>>(),
        "stderr_path": stderr_path,
        "stderr_sha256": sha256_file(&stderr_path).ok(),
        "stderr_tail": stderr_text.lines().rev().take(8).collect::<Vec<_>>(),
        "error": exit_status.err(),
    })
}

fn reqwest_error_receipt(error: &reqwest::Error) -> serde_json::Value {
    let mut source_chain = Vec::new();
    let mut current: Option<&(dyn StdError + 'static)> = Some(error);
    while let Some(source) = current {
        source_chain.push(source.to_string());
        if source_chain.len() == 32 {
            source_chain.push("source_chain_truncated_after_32_entries".to_owned());
            break;
        }
        current = source.source();
    }
    serde_json::json!({
        "display": error.to_string(),
        "debug": format!("{error:?}"),
        "is_builder": error.is_builder(),
        "is_request": error.is_request(),
        "is_body": error.is_body(),
        "is_decode": error.is_decode(),
        "is_connect": error.is_connect(),
        "is_timeout": error.is_timeout(),
        "is_status": error.is_status(),
        "status": error.status().map(|status| status.as_u16()),
        "url": error.url().map(reqwest::Url::as_str),
        "source_chain": source_chain,
    })
}

fn safe_artifact_component(value: &str, fallback: &str) -> String {
    let mut safe = String::with_capacity(value.len().min(96));
    for character in value.chars().take(96) {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            safe.push(character);
        } else {
            safe.push('-');
        }
    }
    let safe = safe.trim_matches(['-', '.']).to_owned();
    if safe.is_empty() {
        fallback.to_owned()
    } else {
        safe
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("open retained artifact {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("hash retained artifact {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write_new_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("artifact path has no parent: {}", path.display()))?;
    let temporary = parent.join(format!(
        ".{}-{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("mt045-artifact"),
        uuid::Uuid::new_v4().simple()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("create atomic artifact {}: {error}", temporary.display()))?;
        file.write_all(bytes)
            .map_err(|error| format!("write atomic artifact {}: {error}", temporary.display()))?;
        file.sync_all()
            .map_err(|error| format!("sync atomic artifact {}: {error}", temporary.display()))?;
        std::fs::rename(&temporary, path).map_err(|error| {
            format!(
                "publish atomic artifact {} -> {}: {error}",
                temporary.display(),
                path.display()
            )
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn path_is_reparse_or_symlink(path: &Path) -> Result<bool, String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("inspect path type {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        return Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0);
    }
    #[cfg(not(windows))]
    Ok(false)
}

fn reject_reparse_chain(path: &Path, boundary: &Path) -> Result<(), String> {
    if !path.starts_with(boundary) {
        return Err(format!(
            "path {} is outside expected boundary {}",
            path.display(),
            boundary.display()
        ));
    }
    let mut current = path;
    loop {
        if path_is_reparse_or_symlink(current)? {
            return Err(format!(
                "reparse point or symlink is forbidden in retained diagnostic path: {}",
                current.display()
            ));
        }
        if current == boundary {
            return Ok(());
        }
        current = current.parent().ok_or_else(|| {
            format!(
                "path {} reached no parent before boundary {}",
                path.display(),
                boundary.display()
            )
        })?;
    }
}

fn copy_hash_atomic(source: &Path, destination: &Path) -> Result<(u64, String), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("retained path has no parent: {}", destination.display()))?;
    let temporary = parent.join(format!(
        ".{}-{}.tmp",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("backend-diagnostic"),
        uuid::Uuid::new_v4().simple()
    ));
    let result = (|| {
        let mut input = File::open(source)
            .map_err(|error| format!("open source diagnostic {}: {error}", source.display()))?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                format!(
                    "create retained diagnostic temporary {}: {error}",
                    temporary.display()
                )
            })?;
        let bytes = std::io::copy(&mut input, &mut output).map_err(|error| {
            format!(
                "copy diagnostic {} -> {}: {error}",
                source.display(),
                temporary.display()
            )
        })?;
        output.sync_all().map_err(|error| {
            format!(
                "sync retained diagnostic temporary {}: {error}",
                temporary.display()
            )
        })?;
        drop(output);
        let sha256 = sha256_file(&temporary)?;
        std::fs::rename(&temporary, destination).map_err(|error| {
            format!(
                "publish retained diagnostic {} -> {}: {error}",
                temporary.display(),
                destination.display()
            )
        })?;
        Ok((bytes, sha256))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

struct FailureRetentionOutcome {
    receipt_path: PathBuf,
    complete: bool,
}

struct SuccessRetentionOutcome {
    receipt: serde_json::Value,
    complete: bool,
}

fn validate_owned_runtime_root(runtime_root: &Path) -> Result<PathBuf, String> {
    let backend_runtime_root = external_artifact_root().join("backend-runtime");
    if !runtime_root.starts_with(&backend_runtime_root) {
        return Err(format!(
            "runtime root {} is outside owned backend runtime root {}",
            runtime_root.display(),
            backend_runtime_root.display()
        ));
    }
    reject_reparse_chain(runtime_root, &backend_runtime_root)?;
    let canonical_boundary = std::fs::canonicalize(&backend_runtime_root).map_err(|error| {
        format!(
            "canonicalize owned backend runtime boundary {}: {error}",
            backend_runtime_root.display()
        )
    })?;
    let canonical_root = std::fs::canonicalize(runtime_root).map_err(|error| {
        format!(
            "canonicalize owned backend runtime root {}: {error}",
            runtime_root.display()
        )
    })?;
    if canonical_root == canonical_boundary || !canonical_root.starts_with(&canonical_boundary) {
        return Err(format!(
            "runtime root {} escaped owned boundary {}",
            canonical_root.display(),
            canonical_boundary.display()
        ));
    }
    Ok(canonical_root)
}

fn remove_runtime_root_and_empty_parents(runtime_root: &Path) -> Result<(), String> {
    if !runtime_root.exists() {
        return Ok(());
    }
    validate_owned_runtime_root(runtime_root)?;
    std::fs::remove_dir_all(runtime_root).map_err(|error| {
        format!(
            "remove fixture-owned runtime root {}: {error}",
            runtime_root.display()
        )
    })?;
    let boundary = external_artifact_root().join("backend-runtime");
    let mut current = runtime_root.parent();
    while let Some(directory) = current {
        if directory == boundary {
            break;
        }
        match std::fs::remove_dir(directory) {
            Ok(()) => current = directory.parent(),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::DirectoryNotEmpty | std::io::ErrorKind::NotFound
                ) =>
            {
                break;
            }
            Err(error) => {
                return Err(format!(
                    "remove empty backend runtime parent {}: {error}",
                    directory.display()
                ));
            }
        }
    }
    Ok(())
}

fn publish_success_runtime_diagnostics(
    runtime_root: &Path,
    scenario_id: &str,
    process: serde_json::Value,
    workspace_cleanup: serde_json::Value,
) -> Result<SuccessRetentionOutcome, String> {
    let canonical_source_root = validate_owned_runtime_root(runtime_root)?;
    #[cfg(test)]
    if canonical_source_root
        .join(".force-success-publication-error")
        .is_file()
    {
        return Err("injected success runtime publication failure".to_owned());
    }
    let wp_root = external_artifact_root();
    let run_id = safe_artifact_component(
        &std::env::var("HSK_MT045_RUN_ID").unwrap_or_else(|_| "standalone-run".to_owned()),
        "standalone-run",
    );
    let scenario = safe_artifact_component(scenario_id, "unnamed-scenario");
    let destination = wp_root
        .join("mt-045")
        .join("success-runtime")
        .join(&run_id)
        .join(&scenario)
        .join(uuid::Uuid::new_v4().simple().to_string());
    std::fs::create_dir_all(&destination).map_err(|error| {
        format!(
            "create success runtime publication directory {}: {error}",
            destination.display()
        )
    })?;
    restrict_runtime_directory(&destination)?;
    let canonical_destination = std::fs::canonicalize(&destination).map_err(|error| {
        format!(
            "canonicalize success runtime publication {}: {error}",
            destination.display()
        )
    })?;
    let canonical_wp_root = std::fs::canonicalize(&wp_root).map_err(|error| {
        format!(
            "canonicalize WP artifact root {}: {error}",
            wp_root.display()
        )
    })?;
    reject_reparse_chain(&canonical_destination, &canonical_wp_root)?;

    let expected = [
        "listen-report.json",
        "backend.stdout.log",
        "backend.stderr.log",
    ];
    let mut files = Vec::new();
    for name in expected {
        let source = canonical_source_root.join(name);
        let retained = canonical_destination.join(name);
        let binding = match std::fs::symlink_metadata(&source) {
            Ok(metadata)
                if metadata.is_file() && !path_is_reparse_or_symlink(&source).unwrap_or(true) =>
            {
                match copy_hash_atomic(&source, &retained) {
                    Ok((bytes, sha256)) => serde_json::json!({
                        "name": name,
                        "status": "retained",
                        "source": source,
                        "path": retained,
                        "bytes": bytes,
                        "sha256": sha256,
                        "error": null,
                    }),
                    Err(error) => serde_json::json!({
                        "name": name,
                        "status": "copy_or_hash_error",
                        "source": source,
                        "path": null,
                        "bytes": null,
                        "sha256": null,
                        "error": error,
                    }),
                }
            }
            Ok(_) => serde_json::json!({
                "name": name,
                "status": "source_type_error",
                "source": source,
                "path": null,
                "bytes": null,
                "sha256": null,
                "error": "expected a regular non-reparse file",
            }),
            Err(error) => serde_json::json!({
                "name": name,
                "status": if error.kind() == std::io::ErrorKind::NotFound { "missing" } else { "source_metadata_error" },
                "source": source,
                "path": null,
                "bytes": null,
                "sha256": null,
                "error": error.to_string(),
            }),
        };
        files.push(binding);
    }
    let complete = files.len() == 3 && files.iter().all(|binding| binding["status"] == "retained");
    let receipt_path = canonical_destination.join("runtime-diagnostics.json");
    let receipt = serde_json::json!({
        "schema_id": "hsk.wp_kernel_012.mt045_success_runtime@1",
        "work_packet_id": "WP-KERNEL-012",
        "micro_task_id": "MT-045",
        "run_id": run_id,
        "scenario_identity": scenario,
        "status": if complete { "complete" } else { "partial" },
        "receipt_path": receipt_path,
        "process": process,
        "workspace_cleanup": workspace_cleanup,
        "command_binding": {
            "label": std::env::var("HSK_MT045_COMMAND_LABEL").ok(),
            "test_binary": std::env::var("HSK_MT045_TEST_BINARY").ok(),
            "test_name": std::env::var("HSK_MT045_TEST_NAME").ok(),
        },
        "files": files,
    });
    write_new_atomic(
        &receipt_path,
        &serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize success runtime receipt: {error}"))?,
    )?;
    let receipt_sha256 = sha256_file(&receipt_path)?;
    write_new_atomic(
        &canonical_destination.join("runtime-diagnostics.json.sha256"),
        format!("{receipt_sha256}  runtime-diagnostics.json\n").as_bytes(),
    )?;
    Ok(SuccessRetentionOutcome { receipt, complete })
}

fn retain_backend_failure_files(
    runtime_roots: &[PathBuf],
    trigger: &str,
    stage: &str,
    label: &str,
    process: serde_json::Value,
    health: serde_json::Value,
    request_error: Option<serde_json::Value>,
    stable_logs: bool,
    mut workspace_cleanup: serde_json::Value,
) -> Result<FailureRetentionOutcome, String> {
    let wp_artifact_root = external_artifact_root();
    std::fs::create_dir_all(&wp_artifact_root).map_err(|error| {
        format!(
            "create WP-012 external artifact root {}: {error}",
            wp_artifact_root.display()
        )
    })?;
    let canonical_wp_root = std::fs::canonicalize(&wp_artifact_root).map_err(|error| {
        format!(
            "canonicalize WP-012 external artifact root {}: {error}",
            wp_artifact_root.display()
        )
    })?;
    let canonical_artifact_root = canonical_wp_root.parent().ok_or_else(|| {
        format!(
            "WP-012 external artifact root has no Handshake_Artifacts parent: {}",
            canonical_wp_root.display()
        )
    })?;
    if canonical_wp_root.file_name().and_then(|name| name.to_str()) != Some("wp-kernel-012")
        || canonical_artifact_root
            .file_name()
            .and_then(|name| name.to_str())
            != Some("Handshake_Artifacts")
    {
        return Err(format!(
            "MT-045 failure diagnostics escaped the canonical Handshake_Artifacts/wp-kernel-012 root: {}",
            canonical_wp_root.display()
        ));
    }
    reject_reparse_chain(&canonical_wp_root, canonical_artifact_root)?;
    let backend_runtime_root = wp_artifact_root.join("backend-runtime");
    std::fs::create_dir_all(&backend_runtime_root).map_err(|error| {
        format!(
            "create canonical backend runtime root {}: {error}",
            backend_runtime_root.display()
        )
    })?;
    let canonical_backend_runtime_root =
        std::fs::canonicalize(&backend_runtime_root).map_err(|error| {
            format!(
                "canonicalize backend runtime root {}: {error}",
                backend_runtime_root.display()
            )
        })?;
    reject_reparse_chain(&canonical_backend_runtime_root, &canonical_wp_root)?;

    let run_id = safe_artifact_component(
        &std::env::var("HSK_MT045_RUN_ID").unwrap_or_else(|_| "standalone-run".to_owned()),
        "standalone-run",
    );
    let scenario = safe_artifact_component(
        thread::current().name().unwrap_or("unnamed-test-thread"),
        "unnamed-test-thread",
    );
    let label_component = safe_artifact_component(label, "request");
    let unique = format!(
        "{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        uuid::Uuid::new_v4().simple()
    );
    let destination = canonical_wp_root
        .join("mt-045")
        .join("failure-diagnostics")
        .join(&run_id)
        .join(&scenario)
        .join(format!("{label_component}-{unique}"));
    std::fs::create_dir_all(&destination).map_err(|error| {
        format!(
            "create MT-045 failure diagnostic directory {}: {error}",
            destination.display()
        )
    })?;
    let canonical_destination = std::fs::canonicalize(&destination).map_err(|error| {
        format!(
            "canonicalize MT-045 failure diagnostic directory {}: {error}",
            destination.display()
        )
    })?;
    if !canonical_destination.starts_with(&canonical_wp_root) {
        return Err(format!(
            "MT-045 failure diagnostic directory escaped external artifact root: {}",
            canonical_destination.display()
        ));
    }
    reject_reparse_chain(&canonical_destination, &canonical_wp_root)?;
    restrict_runtime_directory(&canonical_destination)?;

    let expected_files = [
        "listen-report.json",
        "backend.stdout.log",
        "backend.stderr.log",
    ];
    let mut retained_files = Vec::new();
    for (root_index, runtime_root) in runtime_roots.iter().enumerate() {
        let retained_root = canonical_destination.join(format!("backend-{root_index:02}"));
        std::fs::create_dir(&retained_root).map_err(|error| {
            format!(
                "create retained backend directory {}: {error}",
                retained_root.display()
            )
        })?;
        restrict_runtime_directory(&retained_root)?;
        let source_root_result = (|| {
            if !runtime_root.starts_with(&backend_runtime_root) {
                return Err(format!(
                    "runtime root {} is not lexically under owned backend runtime root {}",
                    runtime_root.display(),
                    backend_runtime_root.display()
                ));
            }
            reject_reparse_chain(runtime_root, &backend_runtime_root)?;
            let canonical = std::fs::canonicalize(runtime_root).map_err(|error| {
                format!(
                    "canonicalize owned backend runtime root {}: {error}",
                    runtime_root.display()
                )
            })?;
            if canonical == canonical_backend_runtime_root
                || !canonical.starts_with(&canonical_backend_runtime_root)
            {
                return Err(format!(
                    "runtime root {} escaped owned backend runtime root {}",
                    canonical.display(),
                    canonical_backend_runtime_root.display()
                ));
            }
            Ok(canonical)
        })();
        for file_name in expected_files {
            let source = runtime_root.join(file_name);
            let retained = retained_root.join(file_name);
            let canonical_source_root = match &source_root_result {
                Ok(root) => root,
                Err(error) => {
                    retained_files.push(serde_json::json!({
                        "runtime_root_index": root_index,
                        "name": file_name,
                        "source": source,
                        "retained_path": null,
                        "status": "source_root_error",
                        "error": error,
                        "bytes": null,
                        "sha256": null,
                    }));
                    continue;
                }
            };
            if !stable_logs {
                retained_files.push(serde_json::json!({
                    "runtime_root_index": root_index,
                    "name": file_name,
                    "source": source,
                    "retained_path": null,
                    "status": "unstable_process",
                    "error": "fixture-owned backend was not confirmed reaped before capture",
                    "bytes": null,
                    "sha256": null,
                }));
                continue;
            }
            let source_metadata = match std::fs::symlink_metadata(&source) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    retained_files.push(serde_json::json!({
                        "runtime_root_index": root_index,
                        "name": file_name,
                        "source": source,
                        "retained_path": null,
                        "status": "missing",
                        "error": null,
                        "bytes": null,
                        "sha256": null,
                    }));
                    continue;
                }
                Err(error) => {
                    retained_files.push(serde_json::json!({
                        "runtime_root_index": root_index,
                        "name": file_name,
                        "source": source,
                        "retained_path": null,
                        "status": "source_metadata_error",
                        "error": error.to_string(),
                        "bytes": null,
                        "sha256": null,
                    }));
                    continue;
                }
            };
            if !source_metadata.is_file() {
                retained_files.push(serde_json::json!({
                    "runtime_root_index": root_index,
                    "name": file_name,
                    "source": source,
                    "retained_path": null,
                    "status": "source_type_error",
                    "error": "expected a regular non-reparse file",
                    "bytes": null,
                    "sha256": null,
                }));
                continue;
            }
            if path_is_reparse_or_symlink(&source).unwrap_or(true) {
                retained_files.push(serde_json::json!({
                    "runtime_root_index": root_index,
                    "name": file_name,
                    "source": source,
                    "retained_path": null,
                    "status": "source_reparse_error",
                    "error": "source diagnostic is a symlink or reparse point",
                    "bytes": null,
                    "sha256": null,
                }));
                continue;
            }
            let canonical_source = match std::fs::canonicalize(&source) {
                Ok(path) if path.parent() == Some(canonical_source_root.as_path()) => path,
                Ok(path) => {
                    retained_files.push(serde_json::json!({
                        "runtime_root_index": root_index,
                        "name": file_name,
                        "source": source,
                        "retained_path": null,
                        "status": "source_escape_error",
                        "error": format!("canonical source escaped runtime root: {}", path.display()),
                        "bytes": null,
                        "sha256": null,
                    }));
                    continue;
                }
                Err(error) => {
                    retained_files.push(serde_json::json!({
                        "runtime_root_index": root_index,
                        "name": file_name,
                        "source": source,
                        "retained_path": null,
                        "status": "source_canonicalization_error",
                        "error": error.to_string(),
                        "bytes": null,
                        "sha256": null,
                    }));
                    continue;
                }
            };
            match copy_hash_atomic(&canonical_source, &retained) {
                Ok((bytes, sha256)) => retained_files.push(serde_json::json!({
                    "runtime_root_index": root_index,
                    "name": file_name,
                    "source": canonical_source,
                    "retained_path": retained,
                    "status": "retained",
                    "error": null,
                    "bytes": bytes,
                    "sha256": sha256,
                })),
                Err(error) => retained_files.push(serde_json::json!({
                    "runtime_root_index": root_index,
                    "name": file_name,
                    "source": canonical_source,
                    "retained_path": null,
                    "status": "copy_or_hash_error",
                    "error": error,
                    "bytes": null,
                    "sha256": null,
                })),
            }
        }
    }

    if workspace_cleanup["status"] == "deleted_and_verified_absent" {
        let retain_cleanup_file = |source_field: &str,
                                   expected_name: &str,
                                   retained_name: &str|
         -> Result<(PathBuf, u64, String), String> {
            if runtime_roots.len() != 1 {
                return Err(
                    "post-reap workspace cleanup proof requires exactly one runtime root"
                        .to_owned(),
                );
            }
            let source = workspace_cleanup[source_field]
                .as_str()
                .map(PathBuf::from)
                .ok_or_else(|| format!("workspace cleanup receipt has no {source_field}"))?;
            let canonical_runtime_root = validate_owned_runtime_root(&runtime_roots[0])?;
            let expected_source = canonical_runtime_root.join(expected_name);
            let canonical_source = std::fs::canonicalize(&source).map_err(|error| {
                format!(
                    "canonicalize workspace cleanup proof {}: {error}",
                    source.display()
                )
            })?;
            if canonical_source != expected_source {
                return Err(format!(
                    "workspace cleanup proof {} is not the expected owned runtime file {}",
                    canonical_source.display(),
                    expected_source.display()
                ));
            }
            let metadata = std::fs::symlink_metadata(&canonical_source).map_err(|error| {
                format!(
                    "inspect workspace cleanup proof {}: {error}",
                    canonical_source.display()
                )
            })?;
            if !metadata.is_file() || path_is_reparse_or_symlink(&canonical_source).unwrap_or(true)
            {
                return Err(format!(
                    "workspace cleanup proof {} is not a regular non-reparse file",
                    canonical_source.display()
                ));
            }
            let retained = canonical_destination.join("backend-00").join(retained_name);
            let (bytes, sha256) = copy_hash_atomic(&canonical_source, &retained)?;
            Ok((retained, bytes, sha256))
        };
        let retained_cleanup = retain_cleanup_file(
            "stdout_path",
            "workspace-cleanup.stdout.log",
            "workspace-cleanup.stdout.log",
        )
        .and_then(|stdout| {
            retain_cleanup_file(
                "stderr_path",
                "workspace-cleanup.stderr.log",
                "workspace-cleanup.stderr.log",
            )
            .map(|stderr| (stdout, stderr))
        });
        match retained_cleanup {
            Ok((
                (stdout_path, stdout_bytes, stdout_sha256),
                (stderr_path, stderr_bytes, stderr_sha256),
            )) => {
                let cleanup = workspace_cleanup
                    .as_object_mut()
                    .expect("workspace cleanup receipt is an object");
                cleanup.insert(
                    "retained_stdout_path".to_owned(),
                    serde_json::json!(stdout_path),
                );
                cleanup.insert(
                    "retained_stdout_bytes".to_owned(),
                    serde_json::json!(stdout_bytes),
                );
                cleanup.insert(
                    "retained_stdout_sha256".to_owned(),
                    serde_json::json!(stdout_sha256),
                );
                cleanup.insert(
                    "retained_stderr_path".to_owned(),
                    serde_json::json!(stderr_path),
                );
                cleanup.insert(
                    "retained_stderr_bytes".to_owned(),
                    serde_json::json!(stderr_bytes),
                );
                cleanup.insert(
                    "retained_stderr_sha256".to_owned(),
                    serde_json::json!(stderr_sha256),
                );
            }
            Err(error) => {
                let cleanup = workspace_cleanup
                    .as_object_mut()
                    .expect("workspace cleanup receipt is an object");
                cleanup.insert("status".to_owned(), serde_json::json!("failed"));
                cleanup.insert("publication_error".to_owned(), serde_json::json!(error));
            }
        }
    }
    let cleanup_complete = match workspace_cleanup["status"].as_str() {
        Some("no_workspace") => {
            workspace_cleanup["verified_absent"] == true
                && workspace_cleanup["remaining_workspace_count"] == 0
        }
        Some("deleted_and_verified_absent") => {
            workspace_cleanup["verified_absent"] == true
                && workspace_cleanup["remaining_workspace_count"] == 0
                && workspace_cleanup["retained_stdout_path"].is_string()
                && workspace_cleanup["retained_stdout_sha256"].is_string()
                && workspace_cleanup["retained_stdout_bytes"].is_u64()
                && workspace_cleanup["retained_stderr_path"].is_string()
                && workspace_cleanup["retained_stderr_sha256"].is_string()
                && workspace_cleanup["retained_stderr_bytes"].is_u64()
        }
        _ => false,
    };
    let complete = cleanup_complete
        && runtime_roots.len() == 1
        && retained_files.len() == expected_files.len()
        && retained_files
            .iter()
            .all(|binding| binding["status"] == "retained");

    let receipt_path = canonical_destination.join("failure-diagnostics.json");
    let receipt = serde_json::json!({
        "schema_id": "hsk.wp_kernel_012.mt045_failure_diagnostics@1",
        "work_packet_id": "WP-KERNEL-012",
        "micro_task_id": "MT-045",
        "run_id": run_id,
        "scenario_identity": scenario,
        "trigger": trigger,
        "stage": stage,
        "label": label,
        "retention_status": if complete { "complete" } else { "partial" },
        "command_binding": {
            "label": std::env::var("HSK_MT045_COMMAND_LABEL").ok(),
            "test_binary": std::env::var("HSK_MT045_TEST_BINARY").ok(),
            "test_name": std::env::var("HSK_MT045_TEST_NAME").ok(),
        },
        "captured_at_unix_ms": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        "process": process,
        "immediate_health": health,
        "reqwest_error": request_error,
        "workspace_cleanup": workspace_cleanup,
        "runtime_roots": runtime_roots,
        "retained_files": retained_files,
    });
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("serialize MT-045 failure diagnostics: {error}"))?;
    write_new_atomic(&receipt_path, &receipt_bytes)?;
    let receipt_sha256 = sha256_file(&receipt_path)?;
    let sidecar_path = canonical_destination.join("failure-diagnostics.json.sha256");
    write_new_atomic(
        &sidecar_path,
        format!("{receipt_sha256}  failure-diagnostics.json\n").as_bytes(),
    )?;
    Ok(FailureRetentionOutcome {
        receipt_path,
        complete,
    })
}

#[cfg(test)]
mod failure_diagnostic_tests {
    use super::*;

    fn unique_runtime_root(label: &str) -> PathBuf {
        external_artifact_root()
            .join("backend-runtime")
            .join("standalone-run")
            .join(label)
            .join(uuid::Uuid::new_v4().simple().to_string())
    }

    fn clean_current_unit_receipt_scenario() {
        let scenario = safe_artifact_component(
            thread::current().name().unwrap_or("unnamed-test-thread"),
            "unnamed-test-thread",
        );
        let scenario_root = external_artifact_root()
            .join("mt-045")
            .join("failure-diagnostics")
            .join("standalone-run")
            .join(scenario);
        let _ = std::fs::remove_dir_all(scenario_root);
    }

    struct ExactTestDirectory(PathBuf);

    impl Drop for ExactTestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn retained_failure_receipt_is_contained_and_hash_bound() {
        clean_current_unit_receipt_scenario();
        let wp_root = external_artifact_root();
        std::fs::create_dir_all(&wp_root).expect("create external WP-012 artifact root");
        let canonical_wp_root =
            std::fs::canonicalize(&wp_root).expect("canonicalize external WP-012 artifact root");
        let input_root = unique_runtime_root("diagnostic-unit-test");
        let complete_runtime_root = input_root.clone();
        std::fs::create_dir_all(&complete_runtime_root).expect("create complete runtime root");
        restrict_runtime_directory(&complete_runtime_root).expect("restrict complete runtime root");
        let input_cleanup = ExactTestDirectory(input_root.clone());

        for (name, contents) in [
            ("listen-report.json", br#"{"port":37501}"#.as_slice()),
            ("backend.stdout.log", b"stdout-proof\n".as_slice()),
            ("backend.stderr.log", b"stderr-proof\n".as_slice()),
        ] {
            std::fs::write(complete_runtime_root.join(name), contents)
                .expect("write complete runtime diagnostic");
        }
        let cleanup_stdout = complete_runtime_root.join("workspace-cleanup.stdout.log");
        let cleanup_stderr = complete_runtime_root.join("workspace-cleanup.stderr.log");
        std::fs::write(&cleanup_stdout, b"0\n").expect("write workspace cleanup stdout");
        std::fs::write(&cleanup_stderr, b"").expect("write workspace cleanup stderr");
        let outcome = retain_backend_failure_files(
            &[complete_runtime_root],
            "unit_test_failure",
            "request_send",
            "../../path escape proof",
            serde_json::json!({"ownership": "fixture_owned", "pid": 42}),
            serde_json::json!({"reachable": false}),
            Some(serde_json::json!({"is_connect": true})),
            true,
            serde_json::json!({
                "status": "deleted_and_verified_absent",
                "workspace_id": "unit-workspace",
                "verified_absent": true,
                "remaining_workspace_count": 0,
                "stdout_path": cleanup_stdout,
                "stderr_path": cleanup_stderr,
            }),
        )
        .expect("retain test failure diagnostics");
        let receipt_path = outcome.receipt_path;
        let receipt_directory = receipt_path
            .parent()
            .expect("receipt has parent")
            .to_path_buf();
        let receipt_cleanup = ExactTestDirectory(receipt_directory.clone());
        assert!(outcome.complete);
        let canonical_receipt_directory = std::fs::canonicalize(&receipt_directory)
            .expect("canonicalize retained receipt directory");
        assert!(canonical_receipt_directory.starts_with(&canonical_wp_root));
        let leaf = canonical_receipt_directory
            .file_name()
            .and_then(|name| name.to_str())
            .expect("UTF-8 receipt directory name");
        assert!(leaf.starts_with("path-escape-proof-"));
        assert!(!leaf.contains(".."));

        let receipt_bytes = std::fs::read(&receipt_path).expect("read retained receipt");
        let receipt: serde_json::Value =
            serde_json::from_slice(&receipt_bytes).expect("parse retained receipt");
        assert_eq!(
            receipt["schema_id"],
            "hsk.wp_kernel_012.mt045_failure_diagnostics@1"
        );
        assert_eq!(receipt["trigger"], "unit_test_failure");
        assert_eq!(receipt["stage"], "request_send");
        assert_eq!(
            receipt["workspace_cleanup"]["status"],
            "deleted_and_verified_absent"
        );
        for stream in ["stdout", "stderr"] {
            let path = PathBuf::from(
                receipt["workspace_cleanup"][format!("retained_{stream}_path")]
                    .as_str()
                    .expect("retained cleanup proof path"),
            );
            assert!(path.starts_with(&canonical_receipt_directory));
            assert_eq!(
                receipt["workspace_cleanup"][format!("retained_{stream}_sha256")],
                sha256_file(&path).expect("hash retained cleanup proof")
            );
            assert_eq!(
                receipt["workspace_cleanup"][format!("retained_{stream}_bytes")],
                std::fs::metadata(&path)
                    .expect("inspect retained cleanup proof")
                    .len()
            );
        }

        let retained_files = receipt["retained_files"]
            .as_array()
            .expect("retained file bindings");
        assert_eq!(retained_files.len(), 3);
        assert_eq!(
            retained_files
                .iter()
                .filter(|entry| entry["status"] == "retained")
                .count(),
            3
        );
        assert_eq!(
            retained_files
                .iter()
                .filter(|entry| entry["status"] == "missing")
                .count(),
            0
        );
        for binding in retained_files
            .iter()
            .filter(|entry| entry["status"] == "retained")
        {
            let retained_path = PathBuf::from(
                binding["retained_path"]
                    .as_str()
                    .expect("retained path string"),
            );
            let canonical_retained =
                std::fs::canonicalize(&retained_path).expect("canonicalize retained diagnostic");
            assert!(canonical_retained.starts_with(&canonical_receipt_directory));
            assert_eq!(
                binding["sha256"].as_str().expect("recorded file hash"),
                sha256_file(&canonical_retained).expect("recompute retained file hash")
            );
        }

        let receipt_hash = sha256_file(&receipt_path).expect("hash receipt");
        let sidecar =
            std::fs::read_to_string(receipt_directory.join("failure-diagnostics.json.sha256"))
                .expect("read receipt hash sidecar");
        assert_eq!(
            sidecar,
            format!("{receipt_hash}  failure-diagnostics.json\n")
        );

        let input_parent = input_root.parent().expect("input parent").to_path_buf();
        let scenario_parent = receipt_directory
            .parent()
            .expect("receipt scenario parent")
            .to_path_buf();
        let run_parent = scenario_parent
            .parent()
            .expect("receipt run parent")
            .to_path_buf();
        drop(receipt_cleanup);
        drop(input_cleanup);
        let _ = std::fs::remove_dir(input_parent);
        let _ = std::fs::remove_dir(scenario_parent);
        let _ = std::fs::remove_dir(run_parent);
    }

    #[test]
    fn secure_mt045_evidence_publication_is_atomic_hash_bound_and_contained() {
        let category = format!("evidence-api-test-{}", uuid::Uuid::new_v4().simple());
        let run = uuid::Uuid::new_v4().simple().to_string();
        let binding =
            publish_mt045_evidence_bytes(&category, &run, "fixture-counts.txt", b"members=10000\n")
                .expect("publish secure MT-045 evidence");
        let path = PathBuf::from(binding["path"].as_str().expect("evidence path"));
        let canonical_mt045_root = std::fs::canonicalize(external_artifact_root().join("mt-045"))
            .expect("canonical MT-045 evidence root");
        assert!(path.starts_with(canonical_mt045_root));
        assert_eq!(binding["bytes"], 14);
        assert_eq!(
            binding["sha256"],
            sha256_file(&path).expect("hash published evidence")
        );
        assert!(
            publish_mt045_evidence_bytes("../escape", &run, "bad.txt", b"bad").is_err(),
            "unsafe evidence category rejected"
        );
        let category_root = path
            .parent()
            .expect("evidence run")
            .parent()
            .expect("evidence category")
            .to_path_buf();
        std::fs::remove_dir_all(category_root).expect("remove evidence API test category");
    }

    #[cfg(windows)]
    #[test]
    fn unreadable_file_yields_typed_partial_receipt_and_preserves_originals() {
        use std::os::windows::fs::OpenOptionsExt;

        clean_current_unit_receipt_scenario();
        let runtime_root = unique_runtime_root("diagnostic-partial-test");
        std::fs::create_dir_all(&runtime_root).expect("create partial runtime root");
        restrict_runtime_directory(&runtime_root).expect("restrict partial runtime root");
        let runtime_cleanup = ExactTestDirectory(runtime_root.clone());
        for (name, contents) in [
            ("listen-report.json", br#"{"port":37501}"#.as_slice()),
            ("backend.stdout.log", b"stable-last-line\n".as_slice()),
            ("backend.stderr.log", b"locked-error-line\n".as_slice()),
        ] {
            std::fs::write(runtime_root.join(name), contents).expect("write partial-test source");
        }
        let locked_stderr = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(runtime_root.join("backend.stderr.log"))
            .expect("exclusively lock stderr source");

        let outcome = retain_backend_failure_files(
            std::slice::from_ref(&runtime_root),
            "unit_test_failure",
            "request_send",
            "diagnostic-partial-test",
            serde_json::json!({
                "owned": true,
                "pid": 42,
                "try_wait": "exited",
                "termination": "already_exited_and_reaped"
            }),
            serde_json::json!({"reachable": false}),
            Some(serde_json::json!({"is_request": true})),
            true,
            serde_json::json!({
                "status": "no_workspace",
                "verified_absent": true,
                "remaining_workspace_count": 0,
            }),
        )
        .expect("publish typed partial receipt");
        let receipt_cleanup = ExactTestDirectory(
            outcome
                .receipt_path
                .parent()
                .expect("partial receipt directory")
                .to_path_buf(),
        );
        assert!(!outcome.complete);
        let receipt: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&outcome.receipt_path).expect("read partial receipt"),
        )
        .expect("parse partial receipt");
        assert_eq!(receipt["retention_status"], "partial");
        let files = receipt["retained_files"]
            .as_array()
            .expect("partial retained file bindings");
        assert_eq!(files.len(), 3);
        assert_eq!(
            files
                .iter()
                .find(|file| file["name"] == "backend.stderr.log")
                .expect("stderr binding")["status"],
            "copy_or_hash_error"
        );
        assert_eq!(
            files
                .iter()
                .filter(|file| file["status"] == "retained")
                .count(),
            2
        );
        for name in [
            "listen-report.json",
            "backend.stdout.log",
            "backend.stderr.log",
        ] {
            assert!(
                runtime_root.join(name).is_file(),
                "original {name} retained"
            );
        }

        drop(locked_stderr);
        drop(receipt_cleanup);
        let scenario_parent = outcome
            .receipt_path
            .parent()
            .expect("receipt directory")
            .parent()
            .expect("receipt scenario parent");
        let run_parent = scenario_parent.parent().expect("receipt run parent");
        let _ = std::fs::remove_dir(scenario_parent);
        let _ = std::fs::remove_dir(run_parent);
        drop(runtime_cleanup);
        let scenario_parent = runtime_root.parent().expect("runtime scenario parent");
        let run_parent = scenario_parent.parent().expect("runtime run parent");
        let _ = std::fs::remove_dir(scenario_parent);
        let _ = std::fs::remove_dir(run_parent);
    }

    #[test]
    fn complete_success_publication_removes_runtime_run_and_scenario_hierarchy() {
        let runtime_root = external_artifact_root()
            .join("backend-runtime")
            .join("success-cleanup-unit-run")
            .join("success-cleanup-unit-scenario")
            .join(uuid::Uuid::new_v4().simple().to_string());
        std::fs::create_dir_all(&runtime_root).expect("create success runtime source");
        restrict_runtime_directory(&runtime_root).expect("restrict success runtime source");
        for (name, contents) in [
            ("listen-report.json", br#"{"pid":42}"#.as_slice()),
            ("backend.stdout.log", b"success-final-line\n".as_slice()),
            ("backend.stderr.log", b"success-stderr-line\n".as_slice()),
        ] {
            std::fs::write(runtime_root.join(name), contents).expect("write success source");
        }
        let scenario_parent = runtime_root
            .parent()
            .expect("success scenario")
            .to_path_buf();
        let run_parent = scenario_parent.parent().expect("success run").to_path_buf();
        let outcome = publish_success_runtime_diagnostics(
            &runtime_root,
            "success-cleanup-unit-scenario",
            serde_json::json!({
                "owned": true,
                "pid": 42,
                "try_wait": "reaped_by_assert_cleanup",
                "termination": "terminated_and_reaped",
                "exit_code": 1,
                "success": false,
            }),
            serde_json::json!({
                "status": "http_delete_completed_before_owned_reap",
                "workspace_id": "unit-workspace",
                "http_status": 204,
            }),
        )
        .expect("publish success runtime diagnostics");
        assert!(outcome.complete);
        assert_eq!(outcome.receipt["status"], "complete");
        let success_receipt = PathBuf::from(
            outcome.receipt["receipt_path"]
                .as_str()
                .expect("success receipt path"),
        );
        let success_directory = success_receipt
            .parent()
            .expect("success receipt directory")
            .to_path_buf();
        let success_cleanup = ExactTestDirectory(success_directory.clone());

        remove_runtime_root_and_empty_parents(&runtime_root)
            .expect("remove runtime root and empty parents");
        assert!(!runtime_root.exists());
        assert!(!scenario_parent.exists());
        assert!(!run_parent.exists());

        drop(success_cleanup);
        let success_scenario = success_directory.parent().expect("success scenario parent");
        let success_run = success_scenario.parent().expect("success run parent");
        let _ = std::fs::remove_dir(success_scenario);
        let _ = std::fs::remove_dir(success_run);
    }

    #[cfg(windows)]
    fn live_backend_for_success_publication_test(
        runtime_root: &Path,
        base: String,
        workspace_id: String,
        force_publication_error: bool,
    ) -> (LiveBackend, PathBuf, u32) {
        std::fs::create_dir_all(runtime_root).expect("create live success runtime source");
        restrict_runtime_directory(runtime_root).expect("restrict live success runtime source");
        let stdout = File::create(runtime_root.join("backend.stdout.log"))
            .expect("create live success stdout");
        let stderr = File::create(runtime_root.join("backend.stderr.log"))
            .expect("create live success stderr");
        let mut command = Command::new("powershell.exe");
        let child = no_window(&mut command)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Write-Output 'live-success-child'; Start-Sleep -Seconds 30",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .expect("spawn live success child");
        let pid = child.id();
        std::fs::write(
            runtime_root.join("listen-report.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema_id": "handshake.backend-listen-report.v1",
                "pid": pid,
                "listen_addr": base.trim_start_matches("http://"),
            }))
            .expect("serialize live success listen report"),
        )
        .expect("write live success listen report");
        if force_publication_error {
            std::fs::write(
                runtime_root.join(".force-success-publication-error"),
                b"injected\n",
            )
            .expect("write success publication error sentinel");
        }
        let lock_path = external_artifact_root().join(format!(
            "success-public-api-test-{}.lock",
            uuid::Uuid::new_v4().simple()
        ));
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .expect("create live success test lock");
        (
            LiveBackend {
                base,
                workspace_id,
                client: build_backend_client(),
                rt: tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("build live success test runtime"),
                owned_backend: RefCell::new(Some(child)),
                owned_binary: None,
                owned_data_dir: None,
                owned_runtime_roots: vec![runtime_root.to_path_buf()],
                retained_failure_receipt: RefCell::new(None),
                preserve_runtime_roots: Cell::new(false),
                _fixture_lock: FileLock { file: lock_file },
            },
            lock_path,
            pid,
        )
    }

    #[cfg(windows)]
    #[test]
    fn public_success_api_deletes_workspace_then_reaps_and_publishes_actual_exit() {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind live success cleanup server");
        let address = listener.local_addr().expect("live success server address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept workspace delete");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set workspace delete read timeout");
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).expect("read workspace delete");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("DELETE /workspaces/success-workspace "));
            stream
                .write_all(
                    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .expect("respond to workspace delete");
        });
        let scenario = format!("public-success-api-{}", uuid::Uuid::new_v4().simple());
        let runtime_root = external_artifact_root()
            .join("backend-runtime")
            .join(format!("{scenario}-run"))
            .join(format!("{scenario}-scenario"))
            .join(uuid::Uuid::new_v4().simple().to_string());
        let scenario_parent = runtime_root
            .parent()
            .expect("success scenario")
            .to_path_buf();
        let run_parent = scenario_parent.parent().expect("success run").to_path_buf();
        let (mut backend, lock_path, pid) = live_backend_for_success_publication_test(
            &runtime_root,
            format!("http://{address}"),
            "success-workspace".to_owned(),
            false,
        );
        let receipt = backend
            .assert_cleanup_and_publish_runtime_diagnostics(&scenario)
            .expect("public success publication");
        server.join().expect("workspace delete server");
        assert_eq!(receipt["status"], "complete");
        assert_eq!(receipt["process"]["pid"], pid);
        assert_eq!(receipt["process"]["termination"], "terminated_and_reaped");
        assert!(receipt["process"]["exit_code"].is_number());
        assert_eq!(receipt["process"]["success"], false);
        assert_eq!(
            receipt["workspace_cleanup"]["status"],
            "http_delete_completed_before_owned_reap"
        );
        assert!(!runtime_root.exists());
        assert!(!scenario_parent.exists());
        assert!(!run_parent.exists());
        let receipt_path = PathBuf::from(
            receipt["receipt_path"]
                .as_str()
                .expect("public success receipt path"),
        );
        let success_directory = receipt_path
            .parent()
            .expect("success directory")
            .to_path_buf();
        drop(backend);
        std::fs::remove_file(lock_path).expect("remove public success test lock");
        std::fs::remove_dir_all(&success_directory).expect("remove public success receipt");
        let success_scenario = success_directory.parent().expect("success scenario parent");
        let success_run = success_scenario.parent().expect("success run parent");
        let _ = std::fs::remove_dir(success_scenario);
        let _ = std::fs::remove_dir(success_run);
    }

    #[cfg(windows)]
    #[test]
    fn public_success_api_preserves_sources_when_publication_errors() {
        let scenario = format!("public-success-error-{}", uuid::Uuid::new_v4().simple());
        let runtime_root = external_artifact_root()
            .join("backend-runtime")
            .join(format!("{scenario}-run"))
            .join(format!("{scenario}-scenario"))
            .join(uuid::Uuid::new_v4().simple().to_string());
        let (mut backend, lock_path, _) = live_backend_for_success_publication_test(
            &runtime_root,
            "http://127.0.0.1:9".to_owned(),
            String::new(),
            true,
        );
        let error = backend
            .assert_cleanup_and_publish_runtime_diagnostics(&scenario)
            .expect_err("injected public success publication error");
        assert!(error.contains("injected success runtime publication failure"));
        drop(backend);
        assert!(runtime_root.exists(), "Drop preserved source evidence");
        std::fs::remove_file(lock_path).expect("remove publication error test lock");
        remove_runtime_root_and_empty_parents(&runtime_root)
            .expect("remove preserved publication error runtime root");
    }

    #[cfg(windows)]
    #[test]
    fn public_success_api_retains_workspace_identity_and_retries_delete_after_non_success() {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind non-success cleanup server");
        let address = listener.local_addr().expect("non-success server address");
        let server = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("accept cleanup retry");
                let mut request = [0_u8; 4096];
                let read = stream.read(&mut request).expect("read cleanup retry");
                assert!(String::from_utf8_lossy(&request[..read])
                    .starts_with("DELETE /workspaces/retry-workspace "));
                stream
                    .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .expect("respond cleanup retry");
            }
        });
        let scenario = format!("public-delete-error-{}", uuid::Uuid::new_v4().simple());
        let runtime_root = unique_runtime_root(&scenario);
        let (mut backend, lock_path, _) = live_backend_for_success_publication_test(
            &runtime_root,
            format!("http://{address}"),
            "retry-workspace".to_owned(),
            false,
        );
        let error = backend
            .assert_cleanup_and_publish_runtime_diagnostics(&scenario)
            .expect_err("non-success workspace cleanup rejects publication");
        assert!(error.contains("returned 503"));
        assert_eq!(backend.workspace_id, "retry-workspace");
        assert!(backend.preserve_runtime_roots.get());
        assert!(backend.owned_backend.borrow().is_some());
        drop(backend);
        server.join().expect("cleanup retry server");
        assert!(
            runtime_root.exists(),
            "failed cleanup preserves source root"
        );
        std::fs::remove_file(lock_path).expect("remove cleanup retry lock");
        remove_runtime_root_and_empty_parents(&runtime_root)
            .expect("remove cleanup retry runtime root");
    }

    #[cfg(windows)]
    #[test]
    fn public_success_api_preserves_before_missing_runtime_root_error() {
        let scenario = format!("public-missing-root-{}", uuid::Uuid::new_v4().simple());
        let runtime_root = unique_runtime_root(&scenario);
        let (mut backend, lock_path, _) = live_backend_for_success_publication_test(
            &runtime_root,
            "http://127.0.0.1:9".to_owned(),
            String::new(),
            false,
        );
        backend.owned_runtime_roots.clear();
        let error = backend
            .assert_cleanup_and_publish_runtime_diagnostics(&scenario)
            .expect_err("missing runtime root rejects publication");
        assert!(error.contains("active runtime root"));
        assert!(backend.preserve_runtime_roots.get());
        drop(backend);
        assert!(runtime_root.exists());
        std::fs::remove_file(lock_path).expect("remove missing-root lock");
        remove_runtime_root_and_empty_parents(&runtime_root)
            .expect("remove missing-root runtime root");
    }

    #[cfg(windows)]
    #[test]
    fn public_success_api_preserves_before_missing_owned_child_error() {
        let scenario = format!("public-missing-child-{}", uuid::Uuid::new_v4().simple());
        let runtime_root = unique_runtime_root(&scenario);
        let (mut backend, lock_path, _) = live_backend_for_success_publication_test(
            &runtime_root,
            "http://127.0.0.1:9".to_owned(),
            String::new(),
            false,
        );
        let mut child = backend
            .owned_backend
            .get_mut()
            .take()
            .expect("take owned child for missing-child test");
        force_kill_tree_and_reap(&mut child, "prepare missing-child publication test")
            .expect("reap missing-child test process");
        let error = backend
            .assert_cleanup_and_publish_runtime_diagnostics(&scenario)
            .expect_err("missing owned child rejects publication");
        assert!(error.contains("fixture-owned backend"));
        assert!(backend.preserve_runtime_roots.get());
        drop(backend);
        assert!(runtime_root.exists());
        std::fs::remove_file(lock_path).expect("remove missing-child lock");
        remove_runtime_root_and_empty_parents(&runtime_root)
            .expect("remove missing-child runtime root");
    }

    #[test]
    fn post_reap_psql_cleanup_deletes_and_verifies_exact_workspace() {
        let Some(workspace_id) = std::env::var("HSK_WORKSPACE_CLEANUP_PROOF_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
        else {
            return;
        };
        let runtime_root = unique_runtime_root("post-reap-workspace-cleanup-test");
        std::fs::create_dir_all(&runtime_root).expect("create psql cleanup runtime root");
        restrict_runtime_directory(&runtime_root).expect("restrict psql cleanup runtime root");
        let runtime_cleanup = ExactTestDirectory(runtime_root.clone());
        let cleanup = cleanup_workspace_after_backend_reap(&workspace_id, &runtime_root);
        assert_eq!(
            cleanup["status"], "deleted_and_verified_absent",
            "post-reap psql cleanup receipt: {cleanup}"
        );
        assert_eq!(cleanup["verified_absent"], true);
        assert!(cleanup["stdout_sha256"].as_str().is_some());
        assert!(cleanup["stderr_sha256"].as_str().is_some());
        drop(runtime_cleanup);
        let scenario_parent = runtime_root.parent().expect("cleanup scenario parent");
        let run_parent = scenario_parent.parent().expect("cleanup run parent");
        let _ = std::fs::remove_dir(scenario_parent);
        let _ = std::fs::remove_dir(run_parent);
    }

    #[test]
    fn attached_backend_failure_is_never_represented_as_owned_or_terminated() {
        clean_current_unit_receipt_scenario();
        let lock_path = external_artifact_root().join(format!(
            "attached-boundary-test-{}.lock",
            uuid::Uuid::new_v4().simple()
        ));
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .expect("create attached-boundary test lock");
        let backend = LiveBackend {
            base: "http://127.0.0.1:9".to_owned(),
            workspace_id: String::new(),
            client: build_backend_client(),
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build attached-boundary runtime"),
            owned_backend: RefCell::new(None),
            owned_binary: None,
            owned_data_dir: None,
            owned_runtime_roots: Vec::new(),
            retained_failure_receipt: RefCell::new(None),
            preserve_runtime_roots: Cell::new(false),
            _fixture_lock: FileLock { file: lock_file },
        };
        let receipt_path = backend
            .retain_failure_diagnostics("panic_drop", "unwind", "attached-boundary-test", None)
            .expect("publish attached-boundary partial receipt");
        let receipt: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&receipt_path).expect("read attached-boundary receipt"),
        )
        .expect("parse attached-boundary receipt");
        assert_eq!(receipt["retention_status"], "partial");
        assert_eq!(receipt["process"]["owned"], false);
        assert_eq!(receipt["process"]["pid"], serde_json::Value::Null);
        assert_eq!(receipt["process"]["termination"], "not_applicable");
        assert_eq!(
            receipt["workspace_cleanup"]["status"],
            "deferred_attached_backend"
        );

        drop(backend);
        let receipt_directory = receipt_path.parent().expect("attached receipt directory");
        let receipt_scenario = receipt_directory
            .parent()
            .expect("attached receipt scenario");
        let receipt_run = receipt_scenario.parent().expect("attached receipt run");
        std::fs::remove_dir_all(receipt_directory).expect("remove attached receipt directory");
        let _ = std::fs::remove_dir(receipt_scenario);
        let _ = std::fs::remove_dir(receipt_run);
        std::fs::remove_file(lock_path).expect("remove attached-boundary lock");
    }
}

impl Drop for LiveBackend {
    fn drop(&mut self) {
        if thread::panicking() && self.retained_failure_receipt.get_mut().is_none() {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.retain_failure_diagnostics("panic_drop", "unwind", "live-backend-drop", None)
            })) {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    self.preserve_runtime_roots.set(true);
                    eprintln!("WARN: retain MT-045 backend failure diagnostics: {error}");
                }
                Err(_) => {
                    self.preserve_runtime_roots.set(true);
                    eprintln!(
                        "WARN: MT-045 backend failure diagnostic capture panicked; preserving original runtime roots"
                    );
                }
            }
        }
        if !self.workspace_id.is_empty() {
            let workspace_id = self.workspace_id.clone();
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.delete_workspace(&workspace_id)
            })) {
                Ok(status) if (200..300).contains(&status) || status == 404 => {
                    self.workspace_id.clear();
                }
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
        let mut owned_backend_reaped = false;
        if let Some(child) = self.owned_backend.get_mut().as_mut() {
            if let Err(error) = force_kill_tree_and_reap(child, "drop fixture-owned backend") {
                eprintln!("FATAL: {error}");
                std::process::abort();
            }
            *self.owned_backend.get_mut() = None;
            owned_backend_reaped = true;
        }
        if owned_backend_reaped && !self.workspace_id.is_empty() {
            match self.owned_runtime_roots.last() {
                Some(runtime_root) => {
                    let cleanup =
                        cleanup_workspace_after_backend_reap(&self.workspace_id, runtime_root);
                    if cleanup["status"] == "deleted_and_verified_absent" {
                        self.workspace_id.clear();
                    } else {
                        self.preserve_runtime_roots.set(true);
                        eprintln!(
                            "WARN: post-reap Drop workspace cleanup failed and source evidence is preserved: {cleanup}"
                        );
                    }
                }
                None => {
                    self.preserve_runtime_roots.set(true);
                    eprintln!(
                        "WARN: owned backend reaped with workspace cleanup pending but no runtime root"
                    );
                }
            }
        }
        self.owned_data_dir.take();
        if self.preserve_runtime_roots.get() {
            for runtime_root in &self.owned_runtime_roots {
                eprintln!(
                    "INFO: preserving incomplete MT-045 failure diagnostic source root {}",
                    runtime_root.display()
                );
            }
        } else {
            for runtime_root in self.owned_runtime_roots.drain(..).rev() {
                if let Err(error) = remove_runtime_root_and_empty_parents(&runtime_root) {
                    eprintln!("FATAL: {error}");
                    std::process::abort();
                }
            }
        }
    }
}

/// Atomically publish an immutable MT-045 evidence file beneath the canonical external artifact
/// root. Every caller-controlled path segment must already be a safe single component; the helper
/// rejects reparse chains, applies the proof directory ACL, and returns the exact path/hash/byte
/// binding that receipts must embed.
pub fn publish_mt045_evidence_bytes(
    category: &str,
    run_component: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<serde_json::Value, String> {
    for (label, component) in [
        ("category", category),
        ("run component", run_component),
        ("filename", filename),
    ] {
        if component.is_empty()
            || component == "."
            || component == ".."
            || safe_artifact_component(component, "") != component
        {
            return Err(format!(
                "MT-045 evidence {label} is not a safe single path component: {component:?}"
            ));
        }
    }

    let wp_root = external_artifact_root();
    std::fs::create_dir_all(&wp_root).map_err(|error| {
        format!(
            "create WP-012 external artifact root {}: {error}",
            wp_root.display()
        )
    })?;
    let canonical_wp_root = std::fs::canonicalize(&wp_root).map_err(|error| {
        format!(
            "canonicalize WP-012 external artifact root {}: {error}",
            wp_root.display()
        )
    })?;
    let artifact_boundary = canonical_wp_root
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "WP-012 external artifact root has no artifact boundary: {}",
                canonical_wp_root.display()
            )
        })?;
    if canonical_wp_root.file_name().and_then(|name| name.to_str()) != Some("wp-kernel-012")
        || artifact_boundary.file_name().and_then(|name| name.to_str())
            != Some("Handshake_Artifacts")
    {
        return Err(format!(
            "MT-045 evidence root is not canonical Handshake_Artifacts/wp-kernel-012: {}",
            canonical_wp_root.display()
        ));
    }
    reject_reparse_chain(&canonical_wp_root, &artifact_boundary)?;

    let mut directory = canonical_wp_root;
    for component in ["mt-045", category, run_component] {
        directory = directory.join(component);
        match std::fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(format!(
                    "create MT-045 evidence directory {}: {error}",
                    directory.display()
                ));
            }
        }
        reject_reparse_chain(&directory, &artifact_boundary)?;
        let metadata = std::fs::symlink_metadata(&directory).map_err(|error| {
            format!(
                "inspect MT-045 evidence directory {}: {error}",
                directory.display()
            )
        })?;
        if !metadata.is_dir() {
            return Err(format!(
                "MT-045 evidence directory path is not a directory: {}",
                directory.display()
            ));
        }
        restrict_runtime_directory(&directory)?;
    }
    let canonical_directory = std::fs::canonicalize(&directory).map_err(|error| {
        format!(
            "canonicalize MT-045 evidence directory {}: {error}",
            directory.display()
        )
    })?;
    let canonical_mt045_root = std::fs::canonicalize(external_artifact_root().join("mt-045"))
        .map_err(|error| format!("canonicalize MT-045 evidence root: {error}"))?;
    if !canonical_directory.starts_with(&canonical_mt045_root) {
        return Err(format!(
            "MT-045 evidence directory escaped canonical root: {}",
            canonical_directory.display()
        ));
    }

    let path = canonical_directory.join(filename);
    write_new_atomic(&path, bytes)?;
    let sha256 = sha256_file(&path)?;
    Ok(serde_json::json!({
        "path": path,
        "bytes": bytes.len(),
        "sha256": sha256,
    }))
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
