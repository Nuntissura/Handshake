//! WP-KERNEL-012 MT-088 (D2 — internal_diagnostics, Tier 2: backend-down graceful degradation,
//! Master Spec v02.196 §5.8.5 "Backend-down graceful degradation (HARD)") runtime proofs.
//!
//! THE motivating bug (2026-06-26 live session): launching Handshake with the backend
//! (`127.0.0.1:37501`) DOWN stalled the egui UI thread (Responding=false, CPU->0, frozen) because a
//! UI-thread backend call (`drive_layout_persistence` -> `load_layout` -> the layout-transport
//! `block_on(GET)`) blocked the frame loop for the connect attempt. §5.8.5 makes this a SPEC DEFECT:
//! "a UI path that can freeze the frame loop on an unreachable backend is a spec defect". This MT moves
//! every UI-thread-reachable backend interaction OFF the frame path (off-thread spawn + poll-if-finished,
//! modeled on the existing `health_handle`) and proves the app DEGRADES, NOT FREEZES, when the backend
//! is down.
//!
//! Each acceptance criterion maps to a REAL runtime proof (no mocked failure for the headline re-prove —
//! AC-008-1 launches the REAL `HandshakeApp` with NOTHING listening on 37501; the connection is genuinely
//! refused — Spec-Realism, RISK-008-2):
//!
//! - AC-008-1 / PT-008-A (`backend_down_responsive`, THE RE-PROVE): the real app with the
//!   backend down, stepped many times, stays RESPONSIVE — every frame completes within a tight bound far
//!   below the connect timeout (no frame stalls for the connect attempt) AND the MT-084 heartbeat counter
//!   advances by N across N frames (the UI thread is provably never stalled; the 2026-06-26 CPU->0
//!   symptom is gone).
//! - AC-008-2 / PT-008-B (`frame_path_has_no_ui_thread_block_on`): a source audit confirms the per-frame
//!   layout lifecycle uses the off-thread spawn+poll path (`spawn_layout_load` / `poll_layout_load`), NOT
//!   a UI-thread `load_layout`/`block_on` on the frame path.
//! - AC-008-3 / PT-008-C (`backend_down_records_event_and_degrades_surface` +
//!   `recovery_fires_recovered_event`): a
//!   `BackendUnreachable` typed event is recorded once on the down edge (debounced — not per frame) and a
//!   `BackendRecovered` once on the recovery edge.
//! - AC-008-4 / PT-008-C (`backend_down_records_event_and_degrades_surface`): the affected surface shows a
//!   DEGRADED/disconnected state (an explicit, finite indicator — not a perpetual spinner, not a hang).
//! - AC-008-5 (`reqwest_clients_carry_connect_and_request_timeouts`): the backend reqwest clients carry a
//!   short connect timeout + request timeout (defense in depth; `src/backend` untouched).
//! - AC-008-6 (`recovery_fires_recovered_event`): recovery works — the surface re-connects and
//!   `BackendRecovered` fires (no permanent stuck-disconnected state).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui_kittest::kittest::NodeT;
use sha2::{Digest, Sha256};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "backend_proof_support/mod.rs"]
mod backend_proof_support;

use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
use handshake_diag_ring::{DiagEventCode, DiagPhase, DiagRingReader, DiagRingWriter, DiagSeverity};
use handshake_native::app::{
    HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID, HEARTBEAT_IDLE_REPAINT_INTERVAL,
};
use handshake_native::backend_client::{
    fetch_health, HealthInfo, WorkbenchLayoutClient,
    BACKEND_CONNECT_TIMEOUT as CLIENT_CONNECT_TIMEOUT,
    BACKEND_REQUEST_TIMEOUT as CLIENT_REQUEST_TIMEOUT,
};
use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::diagnostics::{
    self, control_socket_name, launch_palmistry_at, ShutdownOutcome, BUFFER_CAP, ENV_PALMISTRY_EXE,
};
use handshake_native::layout_persistence::LayoutTransport;
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::split_layout::SplitWeights;
use handshake_native::tab_bar::TabState;

const BACKEND_STATUS_AUTHOR_ID: &str = "shell.chrome.status-bar";

struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_path(key: &'static str, value: &Path) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }

    fn set_value(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn find_palmistry_binary() -> PathBuf {
    let executable = if cfg!(windows) {
        "palmistry.exe"
    } else {
        "palmistry"
    };
    let required = Path::new("../../../../Handshake_Artifacts/handshake-cargo-target")
        .join("debug")
        .join(executable);
    let required = std::fs::canonicalize(&required).unwrap_or_else(|error| {
        panic!(
            "integrated MT-088 proof requires the canonical Palmistry binary at {}: {error}",
            required.display()
        )
    });
    if let Some(raw) = std::env::var_os(ENV_PALMISTRY_EXE) {
        let supplied = std::fs::canonicalize(PathBuf::from(raw))
            .unwrap_or_else(|error| panic!("canonicalize {ENV_PALMISTRY_EXE}: {error}"));
        assert_eq!(
            supplied, required,
            "{ENV_PALMISTRY_EXE} cannot redirect MT-088 away from the reserved canonical target"
        );
    }
    required
}

fn file_sha256(path: &Path) -> String {
    let mut file = std::fs::File::open(path)
        .unwrap_or_else(|error| panic!("open {} for SHA-256: {error}", path.display()));
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .unwrap_or_else(|error| panic!("read {} for SHA-256: {error}", path.display()));
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    format!("{:x}", hasher.finalize())
}

fn json_sha256(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serialize JSON for SHA-256");
    format!("{:x}", Sha256::digest(bytes))
}

fn repo_root() -> PathBuf {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("resolve product repo root for MT-088 provenance");
    assert!(
        output.status.success(),
        "git rev-parse --show-toplevel failed"
    );
    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("repo root is UTF-8")
            .trim(),
    )
}

fn current_binary_provenance(
    binary: &Path,
    tracked_source_pathspecs: &[&str],
) -> serde_json::Value {
    let binary = std::fs::canonicalize(binary)
        .unwrap_or_else(|error| panic!("canonicalize binary {}: {error}", binary.display()));
    let metadata = std::fs::metadata(&binary)
        .unwrap_or_else(|error| panic!("read binary metadata {}: {error}", binary.display()));
    let binary_modified = metadata
        .modified()
        .expect("binary modification time is available");
    let mut command = std::process::Command::new("git");
    command.args(["ls-files", "--full-name", "--"]);
    command.args(tracked_source_pathspecs);
    let output = command
        .output()
        .expect("list tracked current-source binary inputs");
    assert!(
        output.status.success(),
        "git ls-files failed for binary source pathspecs {tracked_source_pathspecs:?}"
    );
    let tracked = String::from_utf8(output.stdout).expect("tracked source paths are UTF-8");
    let root = std::fs::canonicalize(repo_root()).expect("canonicalize product repo root");
    let mut source_count = 0_usize;
    let mut newest_source: Option<(String, std::time::SystemTime)> = None;
    let mut tracked_input_manifest = serde_json::Map::new();
    for repo_path in tracked.lines().filter(|path| !path.is_empty()) {
        let source = root.join(repo_path);
        let modified = std::fs::metadata(&source)
            .unwrap_or_else(|error| {
                panic!("read tracked source metadata {}: {error}", source.display())
            })
            .modified()
            .expect("tracked source modification time is available");
        source_count += 1;
        let canonical_source = std::fs::canonicalize(&source).unwrap_or_else(|error| {
            panic!(
                "canonicalize tracked binary input {}: {error}",
                source.display()
            )
        });
        tracked_input_manifest.insert(
            repo_path.replace('\\', "/"),
            serde_json::json!({
                "repo_path": repo_path.replace('\\', "/"),
                "canonical_path": canonical_source,
                "sha256": file_sha256(&source),
            }),
        );
        if newest_source
            .as_ref()
            .map(|(_, newest)| modified > *newest)
            .unwrap_or(true)
        {
            newest_source = Some((repo_path.to_owned(), modified));
        }
    }
    let (newest_source_path, newest_source_modified) =
        newest_source.expect("binary provenance pathspecs resolve at least one tracked source");
    assert!(
        binary_modified >= newest_source_modified,
        "current-source binary {} ({binary_modified:?}) is older than tracked input \
         {newest_source_path} ({newest_source_modified:?})",
        binary.display()
    );
    let unix_millis = |time: std::time::SystemTime| {
        time.duration_since(std::time::UNIX_EPOCH)
            .expect("provenance timestamp is after epoch")
            .as_millis()
    };
    serde_json::json!({
        "canonical_path": binary.display().to_string(),
        "sha256": file_sha256(&binary),
        "size_bytes": metadata.len(),
        "modified_unix_millis": unix_millis(binary_modified),
        "tracked_source_count": source_count,
        "newest_tracked_source": newest_source_path,
        "newest_tracked_source_modified_unix_millis": unix_millis(newest_source_modified),
        "not_older_than_all_tracked_sources": true,
        "tracked_inputs": tracked_input_manifest,
    })
}

#[cfg(windows)]
struct SuspendedProcessGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
    resumed: bool,
}

#[cfg(windows)]
impl SuspendedProcessGuard {
    fn suspend(pid: u32) -> Self {
        use windows_sys::Win32::System::Threading::OpenProcess;
        const PROCESS_SUSPEND_RESUME: u32 = 0x0800;
        #[link(name = "ntdll")]
        extern "system" {
            fn NtSuspendProcess(process_handle: windows_sys::Win32::Foundation::HANDLE) -> i32;
        }
        // SAFETY: the fixture supplies the PID of its own exact child. The requested right is limited
        // to suspend/resume, the handle is checked, and Drop resumes then closes it exactly once.
        let handle = unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, 0, pid) };
        assert!(
            !handle.is_null(),
            "open fixture-owned backend pid {pid} for suspend: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: `handle` is a live process handle with PROCESS_SUSPEND_RESUME.
        let status = unsafe { NtSuspendProcess(handle) };
        if status != 0 {
            // SAFETY: suspension failed, but the handle was still opened successfully above.
            unsafe {
                let _ = windows_sys::Win32::Foundation::CloseHandle(handle);
            }
            panic!("NtSuspendProcess({pid}) failed with NTSTATUS {status:#x}");
        }
        Self {
            handle,
            resumed: false,
        }
    }

    fn resume(mut self) {
        self.try_resume()
            .unwrap_or_else(|status| panic!("NtResumeProcess failed with NTSTATUS {status:#x}"));
    }

    fn try_resume(&mut self) -> Result<(), i32> {
        if self.resumed {
            return Ok(());
        }
        #[link(name = "ntdll")]
        extern "system" {
            fn NtResumeProcess(process_handle: windows_sys::Win32::Foundation::HANDLE) -> i32;
        }
        // SAFETY: this is the same live fixture-owned process handle suspended above.
        let status = unsafe { NtResumeProcess(self.handle) };
        if status != 0 {
            return Err(status);
        }
        self.resumed = true;
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for SuspendedProcessGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        // Never panic from cleanup while unwinding another assertion. Explicit `resume()` above remains
        // fatal on failure; Drop makes the strongest best-effort recovery and always closes the handle.
        let _ = self.try_resume();
        // SAFETY: the handle was opened once by `suspend` and is closed once here.
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

#[cfg(not(windows))]
struct SuspendedProcessGuard {
    pid: u32,
    resumed: bool,
}

#[cfg(not(windows))]
impl SuspendedProcessGuard {
    fn suspend(pid: u32) -> Self {
        let status = std::process::Command::new("kill")
            .args(["-STOP", &pid.to_string()])
            .status()
            .expect("suspend fixture-owned backend");
        assert!(status.success(), "kill -STOP {pid} failed with {status}");
        Self {
            pid,
            resumed: false,
        }
    }

    fn resume(mut self) {
        self.try_resume()
            .unwrap_or_else(|error| panic!("resume fixture-owned backend failed: {error}"));
    }

    fn try_resume(&mut self) -> Result<(), String> {
        if self.resumed {
            return Ok(());
        }
        let status = std::process::Command::new("kill")
            .args(["-CONT", &self.pid.to_string()])
            .status()
            .map_err(|error| format!("kill -CONT {}: {error}", self.pid))?;
        if !status.success() {
            return Err(format!("kill -CONT {} failed with {status}", self.pid));
        }
        self.resumed = true;
        Ok(())
    }
}

#[cfg(not(windows))]
impl Drop for SuspendedProcessGuard {
    fn drop(&mut self) {
        // Avoid a double panic during cleanup; explicit `resume()` above still hard-fails.
        let _ = self.try_resume();
    }
}

#[cfg(windows)]
fn process_is_running(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    const SYNCHRONIZE_RIGHT: u32 = 0x0010_0000;
    // SAFETY: read-only liveness query for the exact Palmistry pid; checked and closed below.
    let handle = unsafe {
        OpenProcess(
            SYNCHRONIZE_RIGHT | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return false;
    }
    // SAFETY: zero-time wait on a valid process handle.
    let running = unsafe { WaitForSingleObject(handle, 0) } == WAIT_TIMEOUT;
    // SAFETY: the handle was opened once above.
    unsafe {
        let _ = CloseHandle(handle);
    }
    running
}

#[cfg(not(windows))]
fn process_is_running(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

/// A backend base URL whose port is reliably NOT listening, so every connection is refused — a
/// genuinely-down backend for the re-prove (NOT a mock; the TCP connect is really refused). Port 1 on
/// loopback has nothing listening, so the connect is refused immediately (and is in any case bounded by
/// the MT-088 connect timeout). The re-prove points the REAL `/health` + layout-transport code paths here.
const DEAD_BACKEND_URL: &str = "http://127.0.0.1:1";

/// The health and workbench-layout payloads served by [`TestBackend`] in its live mode. They are the
/// real HTTP shapes consumed by `fetch_health` and `WorkbenchLayoutClient`, not transport mocks.
const HEALTH_OK: &str = r#"{"status":"ok","db_status":"ok","migration_version":1}"#;
const HEALTH_UNKNOWN_STATUS: &str =
    r#"{"status":"starting","db_status":"ok","migration_version":1}"#;
const HEALTH_UNKNOWN_DB_STATUS: &str =
    r#"{"status":"error","db_status":"starting","migration_version":null}"#;
const HEALTH_INCONSISTENT: &str = r#"{"status":"ok","db_status":"error","migration_version":null}"#;
const LAYOUT_NONE: &str = r#"{"layout_state":null}"#;

fn expected_layout_path() -> String {
    format!("/workspaces/{DEFAULT_PROJECT_ID}/workbench/layout")
}

fn layout_response_body(layout_state: serde_json::Value) -> String {
    serde_json::json!({ "layout_state": layout_state }).to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackendMode {
    Live,
    MalformedHealth,
    UnknownHealthStatus,
    UnknownDbStatus,
    InconsistentHealth,
    MalformedLayout,
    Silent,
    ControlledLayout,
}

/// A bounded localhost HTTP peer used to exercise the production reqwest/App path. `Silent` accepts
/// and retains every real socket; `ControlledLayout` reads and counts the concrete route, answers health,
/// and retains only layout sockets until the test releases them. The server thread itself is non-blocking
/// and has a bounded stop acknowledgement, so proof cannot hide an unbounded helper `join()` behind the
/// client timeout being tested.
struct TestBackend {
    address: SocketAddr,
    stop_tx: std::sync::mpsc::SyncSender<()>,
    stopped_rx: std::sync::mpsc::Receiver<()>,
    thread: Option<std::thread::JoinHandle<()>>,
    accepted: Arc<AtomicUsize>,
    held: Arc<AtomicUsize>,
    running: Arc<AtomicBool>,
    health_requests: Arc<AtomicUsize>,
    layout_requests: Arc<AtomicUsize>,
    unclassified_requests: Arc<AtomicUsize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestPathFailure {
    DeadlineBeforeRequestLine,
    DeadlineBeforeHeaders,
    EofBeforeRequestLine,
    EofBeforeHeaders,
    RequestLineTooLarge,
    RequestHeadersTooLarge,
    ReadFailed,
    MalformedRequestLine,
}

#[derive(Debug, PartialEq, Eq)]
enum RequestPathRead {
    Classified(String),
    Unclassified(RequestPathFailure),
}

impl TestBackend {
    fn start(mode: BackendMode) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test backend");
        Self::from_listener(listener, mode, None, None)
    }

    fn start_at(address: SocketAddr, mode: BackendMode) -> Self {
        let listener = TcpListener::bind(address).expect("rebind test backend address");
        Self::from_listener(listener, mode, None, None)
    }

    fn start_with_layout_body(layout_body: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind layout test backend");
        Self::from_listener(listener, BackendMode::Live, Some(layout_body), None)
    }

    fn start_controlled_layout(layout_body: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind controlled layout backend");
        let release = Arc::new(AtomicBool::new(false));
        Self::from_listener(
            listener,
            BackendMode::ControlledLayout,
            Some(layout_body),
            Some(release),
        )
    }

    fn from_listener(
        listener: TcpListener,
        mode: BackendMode,
        layout_body: Option<String>,
        release_layouts: Option<Arc<AtomicBool>>,
    ) -> Self {
        listener
            .set_nonblocking(true)
            .expect("test backend listener is non-blocking");
        let address = listener.local_addr().expect("test backend address");
        let (stop_tx, stop_rx) = std::sync::mpsc::sync_channel(1);
        let (stopped_tx, stopped_rx) = std::sync::mpsc::sync_channel(1);
        let accepted = Arc::new(AtomicUsize::new(0));
        let held = Arc::new(AtomicUsize::new(0));
        let running = Arc::new(AtomicBool::new(true));
        let health_requests = Arc::new(AtomicUsize::new(0));
        let layout_requests = Arc::new(AtomicUsize::new(0));
        let unclassified_requests = Arc::new(AtomicUsize::new(0));
        let thread_accepted = Arc::clone(&accepted);
        let thread_held = Arc::clone(&held);
        let thread_running = Arc::clone(&running);
        let thread_health_requests = Arc::clone(&health_requests);
        let thread_layout_requests = Arc::clone(&layout_requests);
        let thread_unclassified_requests = Arc::clone(&unclassified_requests);
        let thread_release_layouts = release_layouts.clone();
        let thread = std::thread::spawn(move || {
            let mut held_streams = Vec::<(TcpStream, String)>::new();
            let layout_path = expected_layout_path();
            loop {
                if stop_rx.try_recv().is_ok() {
                    break;
                }
                if thread_release_layouts
                    .as_ref()
                    .is_some_and(|release| release.load(Ordering::SeqCst))
                    && !held_streams.is_empty()
                {
                    for (stream, path) in held_streams.drain(..) {
                        respond_to_path(stream, mode, &path, layout_body.as_deref());
                    }
                    thread_held.store(0, Ordering::SeqCst);
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        thread_accepted.fetch_add(1, Ordering::SeqCst);
                        let path = match read_request_path(&mut stream) {
                            RequestPathRead::Classified(path) => path,
                            RequestPathRead::Unclassified(failure) => {
                                thread_unclassified_requests.fetch_add(1, Ordering::SeqCst);
                                respond_to_unclassified_request(stream, failure);
                                continue;
                            }
                        };
                        if path == "/health" {
                            thread_health_requests.fetch_add(1, Ordering::SeqCst);
                        } else if path == layout_path.as_str() {
                            thread_layout_requests.fetch_add(1, Ordering::SeqCst);
                        }
                        let should_hold = mode == BackendMode::Silent
                            || (mode == BackendMode::ControlledLayout
                                && path == layout_path.as_str());
                        if should_hold {
                            held_streams.push((stream, path));
                            thread_held.store(held_streams.len(), Ordering::SeqCst);
                        } else {
                            respond_to_path(stream, mode, &path, layout_body.as_deref());
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("test backend accept failed: {error}"),
                }
            }
            drop(held_streams);
            thread_held.store(0, Ordering::SeqCst);
            thread_running.store(false, Ordering::SeqCst);
            let _ = stopped_tx.send(());
        });
        Self {
            address,
            stop_tx,
            stopped_rx,
            thread: Some(thread),
            accepted,
            held,
            running,
            health_requests,
            layout_requests,
            unclassified_requests,
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }

    fn address(&self) -> SocketAddr {
        self.address
    }

    fn accepted_count(&self) -> usize {
        self.accepted.load(Ordering::SeqCst)
    }

    fn held_count(&self) -> usize {
        self.held.load(Ordering::SeqCst)
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn health_request_count(&self) -> usize {
        self.health_requests.load(Ordering::SeqCst)
    }

    fn layout_request_count(&self) -> usize {
        self.layout_requests.load(Ordering::SeqCst)
    }

    fn unclassified_request_count(&self) -> usize {
        self.unclassified_requests.load(Ordering::SeqCst)
    }

    fn stop(mut self) {
        let _ = self.stop_tx.send(());
        self.stopped_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("test backend acknowledges shutdown within 2s");
        if let Some(thread) = self.thread.take() {
            thread.join().expect("completed test backend thread joins");
        }
    }
}

impl Drop for TestBackend {
    fn drop(&mut self) {
        if self.thread.is_none() {
            return;
        }
        let _ = self.stop_tx.try_send(());
        if self.stopped_rx.recv_timeout(Duration::from_secs(2)).is_ok() {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

const MAX_TEST_REQUEST_HEAD_BYTES: usize = 8192;
const MAX_TEST_REQUEST_HEAD_DRAIN_BYTES: usize = 1024;

fn request_head_end(request: &[u8]) -> Option<usize> {
    request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|start| start + 4)
}

fn read_request_path(stream: &mut TcpStream) -> RequestPathRead {
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("set request read timeout");
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let mut classified_path = None;
    let mut oversize_failure = None;
    let mut failure = RequestPathFailure::DeadlineBeforeRequestLine;
    while Instant::now() < deadline {
        let read_limit = oversize_failure
            .map(|_| {
                (MAX_TEST_REQUEST_HEAD_BYTES + MAX_TEST_REQUEST_HEAD_DRAIN_BYTES)
                    .saturating_sub(request.len())
            })
            .unwrap_or(chunk.len())
            .min(chunk.len());
        if read_limit == 0 {
            return RequestPathRead::Unclassified(
                oversize_failure.expect("zero read budget only follows oversize classification"),
            );
        }
        match stream.read(&mut chunk[..read_limit]) {
            Ok(0) => {
                failure = oversize_failure.unwrap_or_else(|| {
                    if classified_path.is_some() {
                        RequestPathFailure::EofBeforeHeaders
                    } else {
                        RequestPathFailure::EofBeforeRequestLine
                    }
                });
                break;
            }
            Ok(read) => {
                request.extend_from_slice(&chunk[..read]);

                // Once the positional maximum has been crossed, the request is already rejected. Drain
                // only through an already-arriving HTTP-head terminator (or one bounded extra chunk) so
                // the Windows peer can read the typed 400 without an unread-receive-buffer RST.
                if let Some(too_large) = oversize_failure {
                    if request_head_end(&request).is_some()
                        || request.len()
                            >= MAX_TEST_REQUEST_HEAD_BYTES + MAX_TEST_REQUEST_HEAD_DRAIN_BYTES
                    {
                        return RequestPathRead::Unclassified(too_large);
                    }
                    continue;
                }

                if classified_path.is_none() {
                    let Some(line_end) = request.iter().position(|byte| *byte == b'\n') else {
                        if request.len() >= MAX_TEST_REQUEST_HEAD_BYTES {
                            oversize_failure = Some(RequestPathFailure::RequestLineTooLarge);
                        }
                        continue;
                    };
                    let request_line_end = line_end + 1;
                    if request_line_end > MAX_TEST_REQUEST_HEAD_BYTES {
                        oversize_failure = Some(RequestPathFailure::RequestLineTooLarge);
                        if request_head_end(&request).is_some() {
                            return RequestPathRead::Unclassified(
                                RequestPathFailure::RequestLineTooLarge,
                            );
                        }
                        continue;
                    }
                    let Ok(request_line) = std::str::from_utf8(&request[..=line_end]) else {
                        return RequestPathRead::Unclassified(
                            RequestPathFailure::MalformedRequestLine,
                        );
                    };
                    let Some(path) = request_line
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .filter(|path| path.starts_with('/'))
                    else {
                        // A complete request line is enough to reject a malformed request. Do not wait
                        // for headers that an invalid client is not required to send.
                        return RequestPathRead::Unclassified(
                            RequestPathFailure::MalformedRequestLine,
                        );
                    };
                    classified_path = Some(path.to_owned());
                }
                // On Windows, closing a socket with unread request-header bytes produces WSAECONNRESET
                // for the client. Classify the line as soon as it arrives, but consume the bounded full
                // HTTP head before responding to a valid request so closure is graceful and deterministic.
                if let Some(head_end) = request_head_end(&request) {
                    if head_end <= MAX_TEST_REQUEST_HEAD_BYTES {
                        return RequestPathRead::Classified(
                            classified_path.expect("classified path exists"),
                        );
                    }
                    return RequestPathRead::Unclassified(
                        RequestPathFailure::RequestHeadersTooLarge,
                    );
                }
                if classified_path.is_some() && request.len() >= MAX_TEST_REQUEST_HEAD_BYTES {
                    oversize_failure = Some(RequestPathFailure::RequestHeadersTooLarge);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => {
                failure = RequestPathFailure::ReadFailed;
                break;
            }
        }
    }
    if let Some(too_large) = oversize_failure {
        failure = too_large;
    } else if classified_path.is_some() && failure == RequestPathFailure::DeadlineBeforeRequestLine
    {
        failure = RequestPathFailure::DeadlineBeforeHeaders;
    }
    RequestPathRead::Unclassified(failure)
}

fn respond_to_unclassified_request(mut stream: TcpStream, failure: RequestPathFailure) {
    let reason = match failure {
        RequestPathFailure::DeadlineBeforeRequestLine => "deadline-before-request-line",
        RequestPathFailure::DeadlineBeforeHeaders => "deadline-before-headers",
        RequestPathFailure::EofBeforeRequestLine => "eof-before-request-line",
        RequestPathFailure::EofBeforeHeaders => "eof-before-headers",
        RequestPathFailure::RequestLineTooLarge => "request-line-too-large",
        RequestPathFailure::RequestHeadersTooLarge => "request-headers-too-large",
        RequestPathFailure::ReadFailed => "read-failed",
        RequestPathFailure::MalformedRequestLine => "malformed-request-line",
    };
    let body = format!(r#"{{"error":"unclassified-request","reason":"{reason}"}}"#);
    let response = format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn layout_request_head_with_len(total_len: usize) -> Vec<u8> {
    let prefix = format!(
        "GET {} HTTP/1.1\r\nHost: bounded-peer\r\nX-Pad: ",
        expected_layout_path()
    );
    let suffix = b"\r\n\r\n";
    assert!(total_len >= prefix.len() + suffix.len());
    let mut request = prefix.into_bytes();
    request.resize(total_len - suffix.len(), b'x');
    request.extend_from_slice(suffix);
    assert_eq!(request.len(), total_len);
    request
}

fn respond_to_path(
    mut stream: TcpStream,
    mode: BackendMode,
    path: &str,
    layout_body: Option<&str>,
) {
    let layout_path = expected_layout_path();
    let (status, body) = if path == "/health" {
        (
            "200 OK",
            match mode {
                BackendMode::MalformedHealth => "{}",
                BackendMode::UnknownHealthStatus => HEALTH_UNKNOWN_STATUS,
                BackendMode::UnknownDbStatus => HEALTH_UNKNOWN_DB_STATUS,
                BackendMode::InconsistentHealth => HEALTH_INCONSISTENT,
                _ => HEALTH_OK,
            },
        )
    } else if path == layout_path.as_str() {
        (
            "200 OK",
            if mode == BackendMode::MalformedLayout {
                "{}"
            } else {
                layout_body.unwrap_or(LAYOUT_NONE)
            },
        )
    } else {
        ("404 Not Found", "{}")
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    // A production client may be cancelled while this peer is reading (endpoint replacement/App Drop).
    // That disconnect is expected lifecycle behaviour and must not kill the shared server thread, which
    // would manufacture later health failures and cross-test worker/probe leakage.
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn reserve_unused_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve unused loopback address");
    let address = listener.local_addr().expect("reserved address");
    drop(listener);
    address
}

#[test]
fn test_backend_requires_the_exact_workbench_layout_route() {
    let backend = TestBackend::start(BackendMode::Live);
    let invalid_routes = [
        format!("/bogus{}", expected_layout_path()),
        "/unrelated-non-health-route".to_owned(),
    ];
    for invalid in &invalid_routes {
        let mut stream = TcpStream::connect(backend.address()).expect("connect exact-route peer");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("bound exact-route response read");
        write!(
            stream,
            "GET {invalid} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            backend.address()
        )
        .expect("write invalid layout route");
        stream.flush().expect("flush invalid layout route");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("read invalid layout response");
        assert!(
            response.starts_with("HTTP/1.1 404 Not Found"),
            "non-exact route {invalid:?} must be rejected: {response:?}"
        );
    }
    assert_eq!(backend.layout_request_count(), 0);
    assert_eq!(backend.health_request_count(), 0);
    assert_eq!(backend.accepted_count(), invalid_routes.len());
    backend.stop();
}

#[test]
fn test_backend_classifies_a_fragmented_request_line_before_routing() {
    let backend = TestBackend::start(BackendMode::Live);
    let mut stream = TcpStream::connect(backend.address()).expect("connect fragmented-route peer");
    stream
        .set_nodelay(true)
        .expect("disable Nagle for fragmented request proof");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("bound fragmented-route response read");

    stream.write_all(b"GET ").expect("write request-line verb");
    stream.flush().expect("flush request-line verb fragment");
    std::thread::sleep(Duration::from_millis(20));
    stream
        .write_all(expected_layout_path().as_bytes())
        .expect("write request-line path");
    stream.flush().expect("flush request-line path fragment");
    std::thread::sleep(Duration::from_millis(20));
    write!(
        stream,
        " HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        backend.address()
    )
    .expect("finish fragmented request");
    stream.flush().expect("flush fragmented request");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read fragmented-route response");
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "a fragmented but complete exact route must be classified after accumulation: {response:?}"
    );
    assert_eq!(backend.accepted_count(), 1);
    assert_eq!(backend.layout_request_count(), 1);
    assert_eq!(backend.health_request_count(), 0);
    assert_eq!(
        backend.unclassified_request_count(),
        0,
        "TCP fragmentation is not an unclassified request"
    );
    backend.stop();
}

#[test]
fn test_backend_accepts_a_request_head_ending_exactly_at_the_limit() {
    let backend = TestBackend::start(BackendMode::Live);
    let mut stream = TcpStream::connect(backend.address()).expect("connect exact-limit peer");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("bound exact-limit response read");
    let request = layout_request_head_with_len(MAX_TEST_REQUEST_HEAD_BYTES);
    stream
        .write_all(&request)
        .expect("write exact-limit request head");
    stream.flush().expect("flush exact-limit request head");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read exact-limit response");
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "a header terminator ending exactly at the inclusive limit remains valid: {response:?}"
    );
    assert_eq!(backend.accepted_count(), 1);
    assert_eq!(backend.unclassified_request_count(), 0);
    assert_eq!(backend.health_request_count(), 0);
    assert_eq!(backend.layout_request_count(), 1);
    backend.stop();
}

#[test]
fn test_backend_rejects_fragmented_headers_ending_just_beyond_the_limit() {
    let backend = TestBackend::start(BackendMode::Live);
    let mut stream = TcpStream::connect(backend.address()).expect("connect oversized-header peer");
    stream
        .set_nodelay(true)
        .expect("disable Nagle for oversized-header fragmentation proof");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("bound oversized-header response read");
    let request = layout_request_head_with_len(MAX_TEST_REQUEST_HEAD_BYTES + 1);
    let split_at = MAX_TEST_REQUEST_HEAD_BYTES - 16;
    stream
        .write_all(&request[..split_at])
        .expect("write oversized-header first fragment");
    stream
        .flush()
        .expect("flush oversized-header first fragment");
    std::thread::sleep(Duration::from_millis(20));
    stream
        .write_all(&request[split_at..])
        .expect("write oversized-header terminal fragment");
    stream
        .flush()
        .expect("flush oversized-header terminal fragment");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read oversized-header response");
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request"),
        "a header terminator ending one byte beyond the limit must take the typed 400 path: {response:?}"
    );
    assert!(
        response.contains(r#""reason":"request-headers-too-large""#),
        "the oversized 400 must preserve its typed boundary diagnosis: {response:?}"
    );
    assert_eq!(backend.accepted_count(), 1);
    assert_eq!(backend.unclassified_request_count(), 1);
    assert_eq!(backend.health_request_count(), 0);
    assert_eq!(backend.layout_request_count(), 0);
    backend.stop();
}

#[test]
fn test_backend_rejects_a_complete_malformed_request_line_as_unclassified() {
    let backend = TestBackend::start(BackendMode::Live);
    let mut stream = TcpStream::connect(backend.address()).expect("connect malformed-line peer");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("bound malformed-line response read");
    stream
        .write_all(b"BROKEN\r\n")
        .expect("write complete malformed request line");
    stream
        .flush()
        .expect("flush complete malformed request line");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read malformed-line response");
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request"),
        "a complete malformed request line must take the explicit 400 path: {response:?}"
    );
    assert!(
        response.contains(r#""reason":"malformed-request-line""#),
        "the 400 response must preserve the typed parse diagnosis: {response:?}"
    );
    assert_eq!(backend.accepted_count(), 1);
    assert_eq!(backend.unclassified_request_count(), 1);
    assert_eq!(backend.health_request_count(), 0);
    assert_eq!(backend.layout_request_count(), 0);
    backend.stop();
}

/// Serializes the tests that EMIT or COUNT `BackendUnreachable`/`BackendRecovered` events. These events
/// are recorded into the PROCESS-GLOBAL diagnostics buffer (shared across the binary's tests), so two
/// such tests running concurrently (the default test threading) would interleave their event emissions
/// and make a before/after DELTA non-deterministic. Holding this lock for the whole test makes the
/// edge-count deltas deterministic WITHOUT weakening the debounce proof. (Tests in OTHER binaries under
/// `cargo test -j 2` are separate processes with their own global, so only same-binary tests matter.)
static BACKEND_EVENT_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Acquire [`BACKEND_EVENT_TEST_LOCK`], recovering from a poisoned lock (a panicking test must not wedge
/// the others). The returned guard is held for the test body so event deltas stay deterministic.
fn lock_backend_event_tests() -> std::sync::MutexGuard<'static, ()> {
    BACKEND_EVENT_TEST_LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

// ── artifact hygiene (CX-212E): no repo-local artifact dir may exist ───────────────────────────────

/// The external artifact root for any MT-088 test output. The proofs here are all in-memory (frame
/// timing + the in-process diagnostics buffer + a source scan); no screenshot/PNG is written, but the
/// guard is invoked uniformly so the hygiene contract is enforced and the helper is not dead.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

const MT088_INTEGRATED_PROOF_PATHS: &[&str] = &[
    "../../../justfile",
    "../../../.cargo/config.toml",
    ".cargo/config.toml",
    "Cargo.toml",
    "tests/test_backend_down_responsive.rs",
    "tests/backend_proof_support/mod.rs",
    "tests/native_gui_support/canonical_argus_driver.rs",
    "tests/native_gui_support/screenshot_harness.rs",
    "tests/native_gui_support/screenshot_marker.rs",
    "src/app.rs",
    "src/backend_client.rs",
    "src/settings_dialog.rs",
    "src/diagnostics/recorder.rs",
    "src/diagnostics/mod.rs",
    "src/diagnostics/palmistry_launch.rs",
    "diag_ring/Cargo.toml",
    "diag_ring/src/lib.rs",
    "diag_ring/src/ring.rs",
    "diag_ring/src/schema.rs",
    "src/mcp/server.rs",
    "src/mcp/binding.rs",
    "src/mcp/action.rs",
    "src/mcp/mod.rs",
    "src/mcp/argus.rs",
    "src/mcp/tools.rs",
    "src/mcp/session.rs",
    "src/mcp/attribution.rs",
    "src/top_menu_bar.rs",
    "src/pane_registry.rs",
    "src/tab_bar.rs",
    "src/layout_persistence.rs",
    "../palmistry/Cargo.toml",
    "../palmistry/src/main.rs",
    "../palmistry/src/lifecycle.rs",
    "../palmistry/src/freeze_detect.rs",
    "src/manual_content_editors.rs",
    "tests/test_manual_content.rs",
];

fn current_head_sha() -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("read current MT-088 source commit");
    assert!(output.status.success(), "git rev-parse HEAD failed");
    String::from_utf8(output.stdout)
        .expect("source SHA is UTF-8")
        .trim()
        .to_owned()
}

fn repo_relative_tracked_path(path: &str) -> String {
    let output = std::process::Command::new("git")
        .args(["ls-files", "--full-name", "--", path])
        .output()
        .unwrap_or_else(|error| panic!("resolve tracked MT-088 proof path {path}: {error}"));
    assert!(
        output.status.success(),
        "git ls-files failed for MT-088 proof path {path}"
    );
    let tracked = String::from_utf8(output.stdout).expect("tracked proof path is UTF-8");
    let paths = tracked.lines().collect::<Vec<_>>();
    assert_eq!(
        paths.len(),
        1,
        "MT-088 proof path {path} must resolve to exactly one tracked repo path; got {paths:?}"
    );
    paths[0].to_owned()
}

fn current_integrated_source_provenance() -> (String, serde_json::Value) {
    let head_sha = current_head_sha();
    let root = std::fs::canonicalize(repo_root()).expect("canonicalize product repo root");
    let mut files = serde_json::Map::new();
    for path in MT088_INTEGRATED_PROOF_PATHS {
        let repo_path = repo_relative_tracked_path(path).replace('\\', "/");
        let canonical_path = std::fs::canonicalize(path)
            .unwrap_or_else(|error| panic!("canonicalize MT-088 proof path {path}: {error}"));
        assert!(
            canonical_path.starts_with(&root),
            "proof input {} must remain inside the product repo {}",
            canonical_path.display(),
            root.display()
        );
        let sha256 = file_sha256(&canonical_path);
        files.insert(
            repo_path.clone(),
            serde_json::json!({
                "repo_path": repo_path,
                "canonical_path": canonical_path,
                "sha256": sha256,
            }),
        );
    }

    // Bind the proof to the entire current candidate, not only the proof-driving file manifest above.
    // `git diff HEAD` includes staged and unstaged tracked changes. Untracked files are added as sorted
    // path/content-hash records so a same-HEAD proof cannot be replayed against a different dirty tree.
    let tracked_diff = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["diff", "--binary", "HEAD", "--", "."])
        .output()
        .expect("read whole tracked MT-088 worktree diff");
    assert!(tracked_diff.status.success(), "git diff HEAD failed");
    let tracked_diff_sha256 = format!("{:x}", Sha256::digest(&tracked_diff.stdout));

    let untracked_output = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()
        .expect("list whole untracked MT-088 worktree candidate");
    assert!(
        untracked_output.status.success(),
        "git ls-files --others failed"
    );
    let mut untracked_paths = untracked_output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8(path.to_vec()).expect("untracked repo path is UTF-8"))
        .collect::<Vec<_>>();
    untracked_paths.sort();

    let mut candidate_material = Vec::new();
    candidate_material.extend_from_slice(b"tracked-diff\0");
    candidate_material.extend_from_slice(&tracked_diff.stdout);
    let mut untracked_files = serde_json::Map::new();
    for repo_path in untracked_paths {
        let path = root.join(&repo_path);
        let canonical_path = std::fs::canonicalize(&path).unwrap_or_else(|error| {
            panic!(
                "canonicalize untracked candidate {}: {error}",
                path.display()
            )
        });
        assert!(
            canonical_path.starts_with(&root),
            "untracked candidate {} must remain inside {}",
            canonical_path.display(),
            root.display()
        );
        let sha256 = file_sha256(&canonical_path);
        candidate_material.extend_from_slice(b"\0untracked\0");
        candidate_material.extend_from_slice(repo_path.as_bytes());
        candidate_material.push(0);
        candidate_material.extend_from_slice(sha256.as_bytes());
        untracked_files.insert(
            repo_path.clone(),
            serde_json::json!({
                "repo_path": repo_path,
                "canonical_path": canonical_path,
                "sha256": sha256,
            }),
        );
    }

    let mut hash_object = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["hash-object", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn git hash-object for whole worktree identity");
    hash_object
        .stdin
        .as_mut()
        .expect("git hash-object stdin is piped")
        .write_all(&candidate_material)
        .expect("write whole worktree identity material");
    let hash_output = hash_object
        .wait_with_output()
        .expect("wait for git hash-object");
    assert!(hash_output.status.success(), "git hash-object failed");
    let candidate_git_blob_oid = String::from_utf8(hash_output.stdout)
        .expect("candidate git object id is UTF-8")
        .trim()
        .to_owned();
    assert!(!candidate_git_blob_oid.is_empty());
    let candidate_sha256 = format!("{:x}", Sha256::digest(&candidate_material));
    let identity = format!("{head_sha}-worktree-{candidate_git_blob_oid}");
    (
        identity.clone(),
        serde_json::json!({
            "identity": identity,
            "head_sha": head_sha,
            "worktree_diff_git_blob_oid": candidate_git_blob_oid,
            "candidate_sha256": candidate_sha256,
            "tracked_diff_sha256": tracked_diff_sha256,
            "tracked_diff_bytes": tracked_diff.stdout.len(),
            "untracked_files": untracked_files,
            "proof_driving_files": files,
        }),
    )
}

fn running_test_binary_provenance() -> serde_json::Value {
    let executable = std::fs::canonicalize(
        std::env::current_exe().expect("resolve running MT-088 test executable"),
    )
    .expect("canonicalize running MT-088 test executable");
    let target = std::fs::canonicalize("../../../../Handshake_Artifacts/handshake-cargo-target")
        .expect("canonicalize reserved Cargo target for running MT-088 test executable");
    assert!(
        executable.starts_with(&target),
        "running MT-088 test executable {} must come from reserved target {}",
        executable.display(),
        target.display()
    );
    let mut provenance = current_binary_provenance(&executable, &["."]);
    provenance["reserved_target"] = serde_json::json!(target);
    provenance["process_id"] = serde_json::json!(std::process::id());
    provenance
}

fn json_author_value<'a>(
    value: &'a serde_json::Value,
    expected_author_id: &str,
) -> Option<&'a str> {
    match value {
        serde_json::Value::Object(object) => {
            if object.get("author_id").and_then(serde_json::Value::as_str)
                == Some(expected_author_id)
            {
                return object
                    .get("value")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| object.get("label").and_then(serde_json::Value::as_str));
            }
            object
                .values()
                .find_map(|value| json_author_value(value, expected_author_id))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| json_author_value(value, expected_author_id)),
        _ => None,
    }
}

fn json_author_ids(value: &serde_json::Value, ids: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(author_id) = object.get("author_id").and_then(serde_json::Value::as_str) {
                ids.push(author_id.to_owned());
            }
            for value in object.values() {
                json_author_ids(value, ids);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                json_author_ids(value, ids);
            }
        }
        _ => {}
    }
}

fn save_integrated_surface(
    harness: &mut Harness<'_, HandshakeApp>,
    artifact_dir: &Path,
    state: &str,
) -> PathBuf {
    let path = artifact_dir.join(format!("mt088-{state}.png"));
    harness
        .render()
        .expect("MT-088 integrated proof requires a material mounted-app render")
        .save(&path)
        .expect("save MT-088 integrated proof screenshot");
    assert!(
        path.is_file() && std::fs::metadata(&path).unwrap().len() > 0,
        "MT-088 {state} screenshot must be a non-empty external artifact"
    );
    path
}

fn validate_integrated_screenshot_markers(
    artifact_dir: &Path,
    run_id: &str,
    source_identity: &str,
    owner_session: &str,
    direct_frames: [(&Path, &str); 3],
    action_receipts: [Option<u64>; 3],
) -> (PathBuf, String, Vec<serde_json::Value>) {
    let marker_path = artifact_dir.join("screenshot_marker.jsonl");
    let marker_bytes = std::fs::read(&marker_path).unwrap_or_else(|error| {
        panic!(
            "read MT-088 screenshot marker {}: {error}",
            marker_path.display()
        )
    });
    let rows = String::from_utf8(marker_bytes.clone())
        .expect("MT-088 screenshot marker is UTF-8")
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .expect("decode MT-088 screenshot marker row")
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3, "exactly three MT-088 frames must be marked");

    let artifact_dir =
        std::fs::canonicalize(artifact_dir).expect("canonicalize MT-088 marker artifact directory");
    let mut previous_sequence = None;
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(row["schema_id"], "hsk.native_gui.screenshot_marker@5");
        assert_eq!(row["mt_id"], "MT-088");
        assert_eq!(row["run_id"], run_id);
        assert_eq!(row["source_sha"], source_identity);
        assert_eq!(row["process_correlation_id"], owner_session);
        assert_eq!(
            row["process_scenario_id"],
            "backend_down_responsive_real_surrealdb_palmistry_argus"
        );
        assert_eq!(row["process_id"], u64::from(std::process::id()));
        assert_eq!(row["status"], "CAPTURED");
        assert_eq!(row["action_receipt_id"].as_u64(), action_receipts[index]);

        let marker_frame = PathBuf::from(
            row["frame_path"]
                .as_str()
                .expect("CAPTURED marker has frame path"),
        );
        let marker_frame = std::fs::canonicalize(&marker_frame).unwrap_or_else(|error| {
            panic!(
                "canonicalize MT-088 marker frame {}: {error}",
                marker_frame.display()
            )
        });
        assert!(
            marker_frame.starts_with(&artifact_dir),
            "marker frame {} must remain under {}",
            marker_frame.display(),
            artifact_dir.display()
        );
        assert_eq!(
            file_sha256(&marker_frame),
            direct_frames[index].1,
            "marker frame {index} pixels must hash-match its operator-facing direct PNG {}",
            direct_frames[index].0.display()
        );
        let sequence = row["proof_event_sequence"]
            .as_u64()
            .expect("marker has monotonic proof sequence");
        if let Some(previous) = previous_sequence {
            assert!(
                sequence > previous,
                "MT-088 screenshot marker proof sequences must be strictly increasing"
            );
        }
        previous_sequence = Some(sequence);
    }
    assert!(
        rows[0]["proof_event_sequence"].as_u64() < rows[1]["proof_event_sequence"].as_u64(),
        "the connected pre-action marker must precede the first receipt-bound marker"
    );

    let marker_path = std::fs::canonicalize(&marker_path)
        .expect("canonicalize MT-088 screenshot marker artifact");
    (
        marker_path,
        format!("{:x}", Sha256::digest(marker_bytes)),
        rows,
    )
}

#[derive(Clone, Debug, serde::Serialize)]
struct LivePhaseSample {
    phase: String,
    step: usize,
    frame_elapsed_micros: u64,
    heartbeat_counter: u64,
}

fn step_until_with_ring_samples(
    harness: &mut Harness<'_, HandshakeApp>,
    ring: &DiagRingReader,
    phase: &str,
    timeout: Duration,
    predicate: impl Fn(&HandshakeApp) -> bool,
) -> Vec<LivePhaseSample> {
    let deadline = Instant::now() + timeout;
    let mut previous_heartbeat = ring
        .read_heartbeat()
        .unwrap_or_else(|| panic!("{phase}: ring heartbeat exists before stepping"))
        .counter;
    let mut samples = Vec::new();
    loop {
        let started = Instant::now();
        harness.step();
        let elapsed = started.elapsed();
        let heartbeat = ring
            .read_heartbeat()
            .unwrap_or_else(|| panic!("{phase}: ring heartbeat remains readable"))
            .counter;
        assert!(
            elapsed < Duration::from_millis(500),
            "{phase}: frame {} exceeded the MT-088 500ms responsiveness ceiling: {elapsed:?}",
            samples.len()
        );
        assert!(
            heartbeat > previous_heartbeat,
            "{phase}: heartbeat must strictly advance on every sampled frame (previous={previous_heartbeat}, current={heartbeat})"
        );
        previous_heartbeat = heartbeat;
        samples.push(LivePhaseSample {
            phase: phase.to_owned(),
            step: samples.len(),
            frame_elapsed_micros: u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX),
            heartbeat_counter: heartbeat,
        });
        if predicate(harness.state()) {
            return samples;
        }
        assert!(
            Instant::now() < deadline,
            "{phase}: terminal predicate did not resolve within {timeout:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn replace_unrelated_sibling_panes_with_manual(app: &mut HandshakeApp, workspace_id: &str) {
    let sibling_ids = [PaneId::from("pane-b"), PaneId::from("pane-c")];
    {
        let registry = app.pane_registry();
        let mut registry = registry.lock().expect("MT-088 sibling pane registry");
        for pane_id in &sibling_ids {
            registry.insert(PaneRecord::new(
                pane_id.clone(),
                PaneType::UserManual,
                workspace_id,
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ));
        }
    }
    for pane_id in sibling_ids {
        let bar = app
            .tab_bar_states_mut()
            .get_mut(&pane_id)
            .unwrap_or_else(|| panic!("MT-088 sibling tab bar {pane_id} exists"));
        bar.tabs = vec![TabState::new(PaneType::UserManual)];
        bar.active_index = 0;
    }
}

fn scroll_settings_diagnostics_to_events(harness: &mut Harness<'_, HandshakeApp>) {
    harness.event(egui::Event::PointerMoved(egui::pos2(720.0, 720.0)));
    harness.step();
    for _ in 0..12 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -4.0),
            modifiers: egui::Modifiers::default(),
        });
        harness.step();
    }
}

fn incident_survivor_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read survivor store {}: {error}", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("survivor-") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

/// Fail if a repo-local `test_output/` OR `tests/screenshots/` dir exists — artifacts must go to the
/// EXTERNAL `Handshake_Artifacts/handshake-test` root only (CX-212E). A tracked artifact under `src/`
/// is a hygiene FAILURE the reviewer also catches with `git ls-files "src/**/*.png"`.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "no repo-local {} dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local,
            p.display()
        );
    }
}

/// Count the `BackendUnreachable` / `BackendRecovered` typed events currently in the process-global
/// in-process diagnostics buffer. The global recorder is shared across tests in this binary, so the
/// proofs assert a DELTA (after - before) rather than an absolute count — robust to test ordering.
fn count_backend_events() -> (usize, usize) {
    let snap = diagnostics::snapshot_last_n(BUFFER_CAP);
    let unreachable = snap
        .iter()
        .filter(|e| e.event_code == DiagEventCode::BackendUnreachable.as_u16())
        .count();
    let recovered = snap
        .iter()
        .filter(|e| e.event_code == DiagEventCode::BackendRecovered.as_u16())
        .count();
    (unreachable, recovered)
}

// ── AC-008-1 / PT-008-A: THE RE-PROVE — real app, backend DOWN, stays responsive ──────────────────

/// THE deliverable proof. Launch the REAL production `HandshakeApp` with NOTHING listening on
/// `127.0.0.1:37501` (the production ctor points at that hardcoded backend URL; the test environment has
/// no backend running, so the connection is genuinely refused — this is the real bug, not a mock). Drive
/// the frame loop many times and assert the app stays RESPONSIVE:
///
/// (a) every frame completes within a tight per-frame time bound FAR below the connect timeout — no
///     frame blocks for the connect attempt (the old freeze blocked the frame for the full connect
///     timeout, or forever); and
/// (b) the MT-084 heartbeat counter advances by exactly N across N frames — the UI thread is provably
///     never stalled (the exact 2026-06-26 CPU->0 / Responding=false symptom is gone).
///
/// This directly contradicts the 2026-06-26 symptom: a frozen frame loop on a down backend.
#[test]
fn backend_down_responsive() {
    // Serialize with the other backend-event tests (this drives a real down backend, emitting events
    // into the shared process-global buffer the count-asserting tests read).
    let _guard = lock_backend_event_tests();
    // Drive the REAL production constructor via the eframe kittest harness, then point its
    // UI-thread-reachable backend interactions (the `/health` poll + the layout-persistence
    // `block_on(GET)` transport) at a GENUINELY connection-refusing endpoint (a dead port). This drives
    // the REAL production code paths against a backend that is really down — it does NOT mock the failure
    // (Spec-Realism, RISK-008-2). A dead port is used (rather than relying on nothing being on 37501)
    // because a real backend may be listening on 37501 in the build environment; the connection to the
    // dead port is unconditionally refused.
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    harness
        .state_mut()
        .set_backend_unreachable_for_test(DEAD_BACKEND_URL);

    // The per-frame bound: a frame must complete FAR below the connect timeout. If a UI-thread backend
    // call still blocked the frame, the frame would take ~the connect timeout (>=1.5s) or hang forever.
    // We assert each frame is well under that — a generous 1.0s (CI machines vary; the old freeze would
    // be >=1.5s connect or unbounded, so 1.0s cleanly separates responsive from frozen). The first frame
    // after construction is allowed a slightly larger budget for one-time wgpu/font setup.
    let frame_budget = Duration::from_millis(1000);
    assert!(
        frame_budget < CLIENT_CONNECT_TIMEOUT,
        "the per-frame responsiveness budget ({frame_budget:?}) must be below the backend connect \
         timeout ({CLIENT_CONNECT_TIMEOUT:?}) — a blocked frame would take at least the connect timeout"
    );

    let counter_before = harness.state().frame_counter();

    let n: u64 = 30;
    let mut worst_frame = Duration::ZERO;
    for i in 0..n {
        let t0 = Instant::now();
        harness.step();
        let dt = t0.elapsed();
        worst_frame = worst_frame.max(dt);
        assert!(
            dt < frame_budget,
            "frame {i} took {dt:?} — a responsive frame must complete well under the connect timeout \
             ({frame_budget:?}); a frame near/above the connect timeout means a UI-thread backend call \
             is still blocking the frame loop (the 2026-06-26 freeze)"
        );
    }

    // (b) The heartbeat oracle (MT-084): the in-app frame counter advanced by EXACTLY N over N frames.
    // A stalled UI thread would stop bumping it. This is the provable "CPU->0 freeze is gone" signal.
    let counter_after = harness.state().frame_counter();
    assert_eq!(
        counter_after - counter_before,
        n,
        "the MT-084 heartbeat (UI-thread frame counter) advanced by exactly N over N frames with the \
         backend DOWN — the UI thread is never stalled (the 2026-06-26 CPU->0 freeze is gone). Worst \
         frame was {worst_frame:?}."
    );

    assert_no_local_artifact_dir();
}

// ── AC-008-3 / AC-008-4 / PT-008-C: backend-down records a typed event + degrades the surface ──────

/// Drive the real app with the backend down until the `/health` poll resolves (connection refused), then
/// assert (AC-008-3) exactly ONE new `BackendUnreachable` typed event was recorded (debounced — not one
/// per frame) AND (AC-008-4) the affected surface shows a DEGRADED/disconnected state, not a perpetual
/// spinner and not a hang: `backend_is_down()` is true and the status-bar health indicator renders the
/// explicit "Disconnected" text (a finite indicator, not "Loading..." forever).
#[test]
fn backend_down_records_event_and_degrades_surface() {
    // Serialize the shared-global event count (see BACKEND_EVENT_TEST_LOCK) so the delta is deterministic.
    let _guard = lock_backend_event_tests();
    let (unreachable_before, _) = count_backend_events();

    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    // Point the real backend interactions at a genuinely-refusing dead port (see the re-prove above).
    harness
        .state_mut()
        .set_backend_unreachable_for_test(DEAD_BACKEND_URL);

    // Step frames until the spawned `/health` poll resolves to "unreachable" (connection refused is fast,
    // but the spawned task + the per-frame fold may take a few frames). Bounded wait so a (hypothetical)
    // backend actually being up would fail loudly rather than hang the test.
    let deadline = Instant::now() + Duration::from_secs(20);
    while !harness.state().backend_is_down() && Instant::now() < deadline {
        harness.step();
        std::thread::sleep(Duration::from_millis(20));
    }

    // AC-008-4: the debounced down state is set — the surface degraded (NOT a spinner, NOT a hang).
    assert!(
        harness.state().backend_is_down(),
        "with nothing listening at {DEAD_BACKEND_URL} the app must observe the backend as unreachable and \
         enter the degraded state (NOT spin forever / hang)"
    );

    // AC-008-3: exactly ONE new BackendUnreachable event (debounced to the down EDGE — not per frame).
    // Even though we stepped many frames in the down state, only ONE down-edge event was emitted.
    let (unreachable_after, _) = count_backend_events();
    assert_eq!(
        unreachable_after - unreachable_before,
        1,
        "exactly ONE BackendUnreachable event is recorded on the down EDGE (debounced — not one per \
         frame, RISK-008-4): saw {} new (before={unreachable_before}, after={unreachable_after})",
        unreachable_after - unreachable_before
    );
    let unreachable_event = diagnostics::snapshot_last_n(BUFFER_CAP)
        .into_iter()
        .rev()
        .find(|event| event.event_code == DiagEventCode::BackendUnreachable.as_u16())
        .expect("the new BackendUnreachable edge remains in the bounded diagnostics buffer");
    assert_eq!(
        unreachable_event.counter_a, 1,
        "the typed edge must carry the actual bound dead endpoint port, not a hardcoded default"
    );

    // AC-008-4 (corroboration): the status-bar health indicator text reflects the disconnected state.
    // `status_bar_health_text()` returns the exact live segment label the AccessKit status node carries.
    let indicator = harness.state().status_bar_health_text();
    assert!(
        indicator.contains("Disconnected"),
        "the status-bar health indicator shows the explicit finite Disconnected state (got {indicator:?}) \
         — not a perpetual spinner / Loading"
    );

    assert_no_local_artifact_dir();
}

// ── AC-008-2 / PT-008-B: the per-frame layout lifecycle has no UI-thread block_on ─────────────────

/// Source audit (code only — comments stripped) that the per-frame layout-persistence lifecycle uses the
/// OFF-thread spawn+poll path, NOT a UI-thread synchronous `load_layout`/`block_on`. The freeze was a
/// `block_on(GET)` reachable from `fn update` -> `ui()` -> `drive_layout_persistence` -> `load_layout`.
/// After the fix, `drive_layout_persistence` step 1 must call `poll_layout_load` + `spawn_layout_load`
/// and must NOT call the synchronous `load_layout` (which still exists for the off-UI-thread test path).
#[test]
fn frame_path_has_no_ui_thread_block_on() {
    let app_src = strip_line_comments(include_str!("../src/app.rs"));

    let drive_fn = extract_fn_body(&app_src, "fn drive_layout_persistence(&mut self")
        .expect("app.rs declares fn drive_layout_persistence(&mut self, ...)");

    // The frame-path lifecycle drains + kicks the OFF-thread load.
    assert!(
        drive_fn.contains("poll_layout_load"),
        "drive_layout_persistence must drain the OFF-thread layout-load result (poll_layout_load)"
    );
    assert!(
        drive_fn.contains("spawn_layout_load"),
        "drive_layout_persistence must spawn the layout load OFF the UI thread (spawn_layout_load)"
    );
    // It must NOT call the synchronous, UI-thread-blocking load_layout on the frame path anymore.
    assert!(
        !drive_fn.contains("self.load_layout("),
        "drive_layout_persistence must NOT call the synchronous self.load_layout(..) on the frame path \
         (that ran the transport block_on(GET) on the egui UI thread — the 2026-06-26 freeze)"
    );
    // And there is no raw block_on anywhere in the frame-path lifecycle body.
    assert!(
        !drive_fn.contains("block_on"),
        "drive_layout_persistence must contain no block_on on the frame path"
    );

    // The off-thread load worker must run on a spawned thread (not inline on the UI thread).
    let spawn_fn = extract_fn_body(&app_src, "fn spawn_layout_load(&mut self")
        .expect("app.rs declares fn spawn_layout_load(&mut self, ...)");
    assert!(
        spawn_fn.contains("std::thread::spawn"),
        "spawn_layout_load must run the load on a spawned OS worker thread (off the UI thread)"
    );
    assert!(
        spawn_fn.contains("layout_publication_gate")
            && spawn_fn.contains(".lock()")
            && spawn_fn.contains("layout_shutdown"),
        "layout-load publication must serialize its shutdown check, delivery, and wake through the publication gate"
    );

    let poll_load_fn = extract_fn_body(&app_src, "fn poll_layout_load(&mut self")
        .expect("app.rs declares fn poll_layout_load(&mut self, ...)");
    assert!(
        !poll_load_fn.contains("note_backend_reachability"),
        "layout delivery must not compete with the canonical /health reachability writer"
    );

    let poll_health_fn = extract_fn_body(&app_src, "fn poll_health(&mut self)")
        .expect("app.rs declares fn poll_health(&mut self)");
    assert!(
        poll_health_fn.contains("self.backend_health_url.clone()"),
        "every re-probe must retain the endpoint injected into the mounted app"
    );
    assert!(!poll_health_fn.contains("HEALTH_URL.to_owned()"), "re-probes must not silently jump back to the production URL after a test/operator endpoint is injected");

    let shutdown_fn = extract_fn_body(&app_src, "fn settle_layout_workers_with_timeout(&mut self")
        .expect("app.rs owns the bounded layout-worker settlement helper");
    assert!(
        shutdown_fn.contains("worker.is_finished()"),
        "shutdown may join only a worker whose handle reports finished"
    );
    assert!(
        shutdown_fn.contains("drop(worker)"),
        "an unfinished worker must be detached after the deadline rather than joined without a bound"
    );
    let begin_shutdown_fn = extract_fn_body(&app_src, "fn begin_layout_shutdown(&self)")
        .expect("app.rs owns the publication-serialized shutdown edge");
    assert!(
        begin_shutdown_fn.contains("layout_publication_gate")
            && begin_shutdown_fn.contains("layout_shutdown")
            && begin_shutdown_fn.contains("store(true"),
        "shutdown must raise its edge while holding the same gate used by layout publication"
    );

    // Both save-worker variants must use one audited completion primitive. That primitive serializes the
    // in-flight transition and repaint eligibility with shutdown, closing the check/detach/repaint race
    // for both the project-switch immediate save and the steady-state debounced save.
    let immediate_save_fn = extract_fn_body(&app_src, "fn spawn_layout_save_now(&mut self)")
        .expect("app.rs owns the immediate off-thread layout-save path");
    assert!(
        immediate_save_fn.contains("finish_layout_save_worker"),
        "the immediate layout-save worker must finish through the shutdown-serialized completion primitive"
    );
    assert!(
        drive_fn.contains("finish_layout_save_worker"),
        "the debounced layout-save worker must finish through the shutdown-serialized completion primitive"
    );
    let save_completion_fn = extract_fn_body(
        &app_src,
        "fn finish_layout_save_worker_with<BeforeEligibility, Publish>(",
    )
    .expect("app.rs owns the shared layout-save completion primitive");
    assert!(
        save_completion_fn.contains("publication_gate")
            && save_completion_fn.contains(".lock()")
            && save_completion_fn.contains("in_flight.store(false")
            && save_completion_fn.contains("shutdown.load"),
        "layout-save completion must clear in-flight state and decide publication while holding the shutdown gate"
    );
    let save_wake_fn = extract_fn_body(&app_src, "fn finish_layout_save_worker(")
        .expect("app.rs owns the production layout-save wake wrapper");
    assert!(
        save_wake_fn.contains("finish_layout_save_worker_with")
            && save_wake_fn.contains("request_repaint"),
        "the production layout-save repaint must be published only through the gated completion primitive"
    );

    // The eframe update body's UI render (`self.ui(ctx)`) is where the frame-path lifecycle runs; confirm
    // `ui` calls drive_layout_persistence (so the audited fn IS on the frame path) — but via the off-
    // thread helpers proven above, never a UI-thread block.
    let ui_fn = extract_fn_body(&app_src, "pub fn ui(&mut self, ctx: &egui::Context)")
        .expect("app.rs declares pub fn ui(&mut self, ctx: &egui::Context)");
    assert!(
        ui_fn.contains("drive_layout_persistence"),
        "ui() drives the layout persistence lifecycle (so the off-thread audit above covers the frame path)"
    );

    // MT-088 V4 inventory: name every production `Runtime::block_on` owner and prove its invocation
    // phase, thread exposure, awaited operation, guard, and bound. A newly introduced call fails this
    // audit until all five properties are reviewed explicitly; a numeric count alone is not evidence.
    let production_end = app_src
        .find("mod mt066_stage_route_admission_tests")
        .expect("app.rs production code precedes the trailing test modules");
    let production_app_src = &app_src[..production_end];
    assert_eq!(
        production_app_src.matches(".block_on(").count(),
        5,
        "every production app.rs block_on call must remain in the explicit named MT-088 frame-safety inventory"
    );
    let workspace_poll = extract_fn_body(&app_src, "fn poll_workspaces(&mut self)")
        .expect("app.rs declares poll_workspaces");
    let workspace_finished = workspace_poll
        .find("is_finished()")
        .expect("workspace poll checks terminal state");
    let workspace_take = workspace_poll
        .find("self.workspaces_handle.take()")
        .expect("workspace poll takes the terminal handle");
    let workspace_block_on = workspace_poll
        .find(".block_on(handle)")
        .expect("workspace poll drains the terminal handle");
    assert!(
        workspace_finished < workspace_take && workspace_take < workspace_block_on,
        "poll_workspaces: frame-path/UI-thread drain awaits the workspace JoinHandle only after the exact is_finished guard, so its bound is an already-terminal handle"
    );
    let health_finished = poll_health_fn
        .find("is_finished()")
        .expect("health poll checks terminal state");
    let health_take = poll_health_fn
        .find("self.health_handle.take()")
        .expect("health poll takes the terminal handle");
    let health_block_on = poll_health_fn
        .find(".block_on(handle)")
        .expect("health poll drains the terminal handle");
    assert!(
        health_finished < health_take && health_take < health_block_on,
        "poll_health: frame-path/UI-thread drain awaits the health JoinHandle only after the exact is_finished guard, so its bound is an already-terminal handle"
    );
    let abort_drain = extract_fn_body(&app_src, "fn abort_and_drain_runtime_task<T>(")
        .expect("app.rs declares abort_and_drain_runtime_task");
    let abort_position = abort_drain
        .find(".abort()")
        .expect("task-drain helper aborts the old task");
    let drain_position = abort_drain
        .find(".block_on(")
        .expect("task-drain helper drains the cancelled handle");
    assert!(
        abort_position < drain_position
            && abort_drain.contains("tokio::time::timeout(timeout, handle).await"),
        "abort_and_drain_runtime_task: shutdown aborts first and wraps the cancellation drain in the caller's remaining aggregate bound"
    );
    assert_eq!(
        abort_drain.matches(".block_on(").count(),
        1,
        "abort_and_drain_runtime_task owns exactly one inventoried cancellation drain"
    );

    let loss_sensitive_drain = extract_fn_body(&app_src, "fn drain_runtime_task<T>(")
        .expect("app.rs declares drain_runtime_task");
    assert!(
        loss_sensitive_drain.contains("tokio::time::timeout(timeout, handle).await"),
        "drain_runtime_task: shutdown-only await gives loss-sensitive settings/preference writes an explicit caller-supplied bound"
    );
    let shutdown_runtime_tasks = extract_fn_body(
        &app_src,
        "fn shutdown_background_runtime_tasks(&mut self, deadline: std::time::Instant)",
    )
    .expect("app.rs declares shutdown_background_runtime_tasks");
    assert_eq!(
        production_app_src
            .matches("Self::abort_and_drain_runtime_task")
            .count(),
        2,
        "abort+await is shutdown-only and covers exactly health + workspace tasks"
    );
    for (slot, intended, forbidden) in [
        (
            "if let Some(handle) = self.health_handle.take() {",
            "Self::abort_and_drain_runtime_task",
            "Self::drain_runtime_task",
        ),
        (
            "if let Some(handle) = self.workspaces_handle.take() {",
            "Self::abort_and_drain_runtime_task",
            "Self::drain_runtime_task",
        ),
        (
            "if let Some(handle) = self.settings_io_handle.take() {",
            "Self::drain_runtime_task",
            "Self::abort_and_drain_runtime_task",
        ),
        (
            "if let Some(handle) = self.preference_io_handle.take() {",
            "Self::drain_runtime_task",
            "Self::abort_and_drain_runtime_task",
        ),
    ] {
        let branch = extract_braced_region(shutdown_runtime_tasks, slot)
            .unwrap_or_else(|| panic!("shutdown owns exact branch {slot}"));
        assert_eq!(
            branch.matches(intended).count(),
            1,
            "shutdown maps {slot} to exactly one {intended}"
        );
        assert_eq!(
            branch.matches(forbidden).count(),
            0,
            "shutdown must not map {slot} to {forbidden}"
        );
    }
    assert_eq!(
        shutdown_runtime_tasks
            .matches("Self::abort_and_drain_runtime_task")
            .count(),
        2,
        "shutdown body contains exactly the health + workspace abort drains"
    );
    assert_eq!(
        shutdown_runtime_tasks
            .matches("Self::drain_runtime_task")
            .count(),
        2,
        "the bounded loss-sensitive drain is confined to explicit app shutdown and covers exactly settings + preference writes"
    );
    assert!(
        shutdown_runtime_tasks.contains("deadline.saturating_duration_since")
            && shutdown_runtime_tasks.matches("remaining").count() >= 4,
        "settings + preference drains must share one aggregate shutdown deadline, not receive a fresh per-handle allowance"
    );
    let layout_settlement = extract_fn_body(
        &app_src,
        "fn wait_for_layout_workers_to_settle(&mut self, deadline: std::time::Instant)",
    )
    .expect("layout settlement accepts the aggregate runtime deadline");
    assert!(
        layout_settlement.contains("deadline.saturating_duration_since")
            && layout_settlement.contains("remaining.min(BACKGROUND_WORKER_SHUTDOWN_TIMEOUT)"),
        "layout-worker settlement consumes only the aggregate deadline remainder"
    );
    let health_replacement = extract_fn_body(&app_src, "fn replace_health_handle(")
        .expect("app.rs declares replace_health_handle");
    assert!(
        health_replacement.contains("old.abort()")
            && health_replacement.contains("drop(old)")
            && !health_replacement.contains("block_on")
            && !health_replacement.contains("abort_and_drain_runtime_task"),
        "frame-reachable health replacement must abort+release without awaiting cancellation"
    );
    for (owner, signature) in [
        (
            "flush_preference_write_now",
            "fn flush_preference_write_now(&mut self)",
        ),
        (
            "flush_settings_save_now",
            "fn flush_settings_save_now(&mut self)",
        ),
        (
            "drive_settings_dialog",
            "fn drive_settings_dialog(&mut self, ctx: &egui::Context)",
        ),
    ] {
        let body = extract_fn_body(&app_src, signature)
            .unwrap_or_else(|| panic!("app.rs declares {owner}"));
        assert!(
            !body.contains("drain_runtime_task") && !body.contains("block_on"),
            "{owner}: the frame path must release an already-published predecessor without synchronously joining it"
        );
    }
    let mcp_startup = extract_fn_body(&app_src, "fn spawn_mcp_server(&mut self)")
        .expect("app.rs declares spawn_mcp_server");
    assert!(
        mcp_startup.contains("SwarmMcpServer::bind")
            && !mcp_startup.contains("fetch_health")
            && !mcp_startup.contains("load_layout"),
        "spawn_mcp_server: constructor-only/UI-construction-thread await is the local MCP bind, not backend I/O"
    );
    let constructor = extract_fn_body(&app_src, "pub fn new(cc: &eframe::CreationContext<'_>)")
        .expect("app.rs declares the production constructor");
    assert!(
        constructor.contains("app.spawn_mcp_server()"),
        "the MCP bind is invoked only during construction, before any eframe update/frame callback"
    );
    assert_eq!(
        production_app_src.matches(".spawn_mcp_server()").count(),
        1,
        "production invokes the startup MCP bind exactly once"
    );
    let mcp_server_src = strip_line_comments(include_str!("../src/mcp/server.rs"));
    let mcp_bind = extract_fn_body(&mcp_server_src, "async fn bind_with_safety_mode(")
        .expect("mcp/server.rs declares bind_with_safety_mode");
    assert!(
        mcp_bind.contains("TcpListener::bind((\"127.0.0.1\", 0)).await")
            && mcp_bind.contains("binding::write_binding(&binding)")
            && !mcp_bind.contains("reqwest")
            && !mcp_bind.contains("fetch_health")
            && !mcp_bind.contains("load_layout"),
        "startup MCP await is an ephemeral loopback listener bind plus local discovery publication and performs no remote/backend request"
    );
    let mcp_binding_src = strip_line_comments(include_str!("../src/mcp/binding.rs"));
    let binding_lock = extract_fn_body(&mcp_binding_src, "fn acquire_binding_lock(")
        .expect("mcp/binding.rs declares acquire_binding_lock");
    assert!(
        binding_lock.contains("const LOCK_TIMEOUT")
            && binding_lock.contains("Duration::from_secs(5)")
            && binding_lock.contains("file.try_lock()")
            && binding_lock.contains("Instant::now() < deadline"),
        "startup discovery publication coordinates through a try_lock loop with an explicit five-second deadline"
    );

    let handshake_drop_src = app_src
        .split_once("impl Drop for HandshakeApp")
        .map(|(_, tail)| tail)
        .expect("app.rs implements Drop for HandshakeApp");
    let app_drop = extract_fn_body(handshake_drop_src, "fn drop(&mut self)")
        .expect("HandshakeApp implements bounded Drop teardown");
    assert!(
        app_drop.contains("shutdown_runtime_once()"),
        "Drop must use the single idempotent bounded runtime shutdown path"
    );
    let shutdown_once = extract_fn_body(&app_src, "fn shutdown_runtime_once(&mut self)")
        .expect("app.rs owns one idempotent runtime shutdown path");
    assert!(
        shutdown_once.contains("if self.rt.is_none()")
            && shutdown_once.contains("background_runtime_shutdown_timeout")
            && shutdown_once.contains("shutdown_background_runtime_tasks(runtime_deadline)")
            && shutdown_once.contains("self.rt.take()")
            && shutdown_once.contains("rt.shutdown_timeout(remaining)"),
        "the single shutdown path must consume Tokio with the remaining aggregate deadline and become a no-op on a second call"
    );
    let on_exit = extract_fn_body(&app_src, "fn on_exit(&mut self)")
        .expect("eframe app owns an explicit on_exit path");
    assert!(
        on_exit.contains("shutdown_runtime_once()"),
        "eframe on_exit and Drop must share the same idempotent bounded runtime shutdown path"
    );

    for (owner, body) in [
        ("spawn_mcp_server", mcp_startup),
        ("poll_workspaces", workspace_poll),
        ("abort_and_drain_runtime_task", abort_drain),
        ("drain_runtime_task", loss_sensitive_drain),
        ("poll_health", poll_health_fn),
    ] {
        assert_eq!(
            body.matches(".block_on(").count(),
            1,
            "{owner} owns exactly one explicitly inventoried production block_on"
        );
    }

    // Runtime regression, not source inference: a loss-sensitive spawn_blocking transport is
    // deliberately wedged beyond all request-timeout assumptions. The real eframe on_exit path must
    // consume one injected aggregate budget; Drop must then be an idempotent no-op for the runtime.
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let exited = Arc::new(AtomicBool::new(false));
    let shutdown_bound = Duration::from_millis(20);
    let scheduler_slack = Duration::from_millis(100);
    let mut app = HandshakeApp::with_health(HealthDisplayState::Loading);
    app.set_background_runtime_shutdown_timeout_for_test(shutdown_bound);
    app.spawn_blocked_loss_sensitive_runtime_task_for_test(
        entered.clone(),
        release.clone(),
        exited.clone(),
    );
    let entered_deadline = Instant::now() + Duration::from_secs(2);
    while !entered.load(Ordering::SeqCst) {
        assert!(
            Instant::now() < entered_deadline,
            "deliberately blocked loss-sensitive worker starts within two seconds"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let shutdown_started = Instant::now();
    eframe::App::on_exit(&mut app);
    let on_exit_elapsed = shutdown_started.elapsed();
    assert!(
        on_exit_elapsed <= shutdown_bound + scheduler_slack,
        "blocked loss-sensitive task cannot freeze eframe on_exit: {on_exit_elapsed:?} must remain within {shutdown_bound:?} + {scheduler_slack:?} scheduler slack"
    );
    release.store(true, Ordering::SeqCst);
    let exit_deadline = Instant::now() + Duration::from_secs(2);
    while !exited.load(Ordering::SeqCst)
        || HandshakeApp::active_blocked_runtime_workers_for_test() != 0
    {
        assert!(
            Instant::now() < exit_deadline,
            "released blocked runtime worker reaps within two seconds"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let drop_started = Instant::now();
    drop(app);
    let drop_elapsed = drop_started.elapsed();
    assert!(
        drop_elapsed <= scheduler_slack,
        "Drop after on_exit must be runtime-idempotent, got {drop_elapsed:?}"
    );
    assert!(
        on_exit_elapsed + drop_elapsed <= shutdown_bound + scheduler_slack,
        "on_exit + idempotent Drop must consume one shutdown budget plus scheduler slack"
    );

    assert_no_local_artifact_dir();
}

// ── AC-008-5: the backend reqwest clients carry a connect + request timeout ────────────────────────

/// The backend reqwest clients carry a short connect timeout + a request timeout so a dead/half-open
/// backend cannot hang a worker indefinitely (defense in depth — the off-thread move already prevents a
/// UI stall; the timeout prevents a leaked worker on a half-open socket). `src/backend` is untouched —
/// this is reuse-config of the EXISTING client. Verified by the published constant + a source scan of the
/// client builder.
#[test]
fn reqwest_clients_carry_connect_and_request_timeouts() {
    // The published connect timeout is short and bounded (1-2s per the MT note).
    assert!(
        CLIENT_CONNECT_TIMEOUT >= Duration::from_millis(500)
            && CLIENT_CONNECT_TIMEOUT <= Duration::from_secs(2),
        "the backend connect timeout ({CLIENT_CONNECT_TIMEOUT:?}) is a short 0.5-2s bound so a half-open \
         backend cannot hang a worker for the OS default (tens of seconds)"
    );

    // The shared client builder applies BOTH connect_timeout and timeout. Source-scan the builder fn body
    // (code only) so the proof is robust to formatting.
    let client_src = strip_line_comments(include_str!("../src/backend_client.rs"));
    let build_fn = extract_fn_body(&client_src, "pub fn build_backend_client()")
        .expect("backend_client.rs declares pub fn build_backend_client()");
    assert!(
        build_fn.contains("build_backend_client_with_request_timeout"),
        "the public canonical builder must delegate to the timeout-configured builder"
    );
    let configured_builder = extract_fn_body(
        &client_src,
        "fn build_backend_client_with_request_timeout(request_timeout: Duration)",
    )
    .expect("backend_client.rs declares the delegated timeout-configured builder");
    assert!(
        configured_builder.contains("connect_timeout"),
        "the delegated canonical ClientBuilder must set connect_timeout"
    );
    assert!(
        configured_builder.contains(".timeout("),
        "the delegated canonical ClientBuilder must set an overall request timeout"
    );

    // The UI-thread-reachable transports use the timed client (not a bare reqwest::Client::new()).
    assert!(
        client_src.contains("client: build_backend_client()"),
        "the WorkbenchLayoutClient (the freeze-path transport) uses the timed build_backend_client()"
    );
    assert!(
        client_src.contains("let client = build_backend_client();"),
        "fetch_health uses the timed build_backend_client()"
    );

    // Inventory is deliberately exhaustive: if a new reqwest-owning UI client is added, this test fails
    // until its constructor is named and audited. Every ordinary client must clone the one shared pool;
    // only the layout transport and long-running model-session launch use named canonical variants.
    let expected_clients = [
        (
            "ModelSessionLaunchClient",
            "build_model_session_backend_client",
        ),
        ("WorkbenchLayoutClient", "build_backend_client"),
        ("LoomBlockClient", "shared_http_client"),
        ("SourceControlClient", "shared_http_client"),
        ("CanvasClient", "shared_http_client"),
        ("CanvasTitleClient", "shared_http_client"),
        ("DrawerDataClient", "shared_http_client"),
        ("DrawerActionClient", "shared_http_client"),
        ("LoomGraphClient", "shared_http_client"),
        ("CanvasBoardClient", "shared_http_client"),
        ("LoomFolderClient", "shared_http_client"),
        ("LoomTagClient", "shared_http_client"),
        ("LoomSidebarClient", "shared_http_client"),
        ("LoomWikiClient", "shared_http_client"),
        ("BlockViewClient", "shared_http_client"),
        ("LoomSearchV2Client", "shared_http_client"),
        ("WorkspaceSearchClient", "shared_http_client"),
        ("RichDocClient", "shared_http_client"),
        ("AtelierClient", "shared_http_client"),
    ];
    let discovered = reqwest_owning_client_names(&client_src);
    let expected_names: std::collections::BTreeSet<_> = expected_clients
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    assert_eq!(discovered, expected_names, "every reqwest-owning backend client must be explicitly inventoried by the MT-088 timeout audit");
    for (client, builder) in expected_clients {
        let constructor = extract_client_constructor(&client_src, client)
            .unwrap_or_else(|| panic!("{client}::new constructor must be extractable"));
        assert!(
            constructor.contains(builder),
            "{client}::new must use canonical {builder}; constructor was {constructor}"
        );
        assert!(
            !constructor.contains("reqwest::Client::new"),
            "{client} must not mint a bare unbounded pool"
        );
        assert!(
            !constructor.contains("reqwest::Client::builder"),
            "{client} must not fork timeout policy with a private ClientBuilder"
        );
    }
    let code_nav = extract_fn_body(&client_src, "pub async fn code_nav_get(")
        .expect("code_nav_get production helper exists");
    assert!(
        code_nav.contains("shared_http_client"),
        "the UI-mounted code-nav helper must use the shared canonical pool"
    );
    assert!(
        !code_nav.contains("reqwest::Client::new"),
        "code-nav must not retain a bare client outside the struct inventory"
    );

    // Frontend-wide audit: scan every Rust source file, not only backend_client.rs. Raw convenience GETs
    // are forbidden because they silently mint a private client with default timeout policy. The exact
    // remaining Client::new sites are explicit base-URL/test seams whose production constructors use the
    // shared pool; any new occurrence or count change fails this inventory.
    let frontend_sources = read_frontend_rust_sources();
    let mut raw_new_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut private_builder_counts = std::collections::BTreeMap::<String, usize>::new();
    for (path, source) in &frontend_sources {
        let code = strip_line_comments(source);
        assert!(
            !code.contains("reqwest::get("),
            "{path} must use the canonical shared bounded client, never reqwest::get"
        );
        let raw_new_count = code.matches("reqwest::Client::new()").count();
        if raw_new_count > 0 {
            raw_new_counts.insert(path.clone(), raw_new_count);
        }
        let builder_count = code.matches("reqwest::Client::builder()").count();
        if builder_count > 0 {
            private_builder_counts.insert(path.clone(), builder_count);
        }
    }
    let expected_raw_new_counts = std::collections::BTreeMap::from([
        ("backend/knowledge_code_nav.rs".to_owned(), 1),
        ("backend/knowledge_crdt.rs".to_owned(), 1),
        ("backend/knowledge_documents.rs".to_owned(), 1),
        ("backend/loom.rs".to_owned(), 1),
        ("backend_client.rs".to_owned(), 1),
        ("event_emitter.rs".to_owned(), 1),
        ("fems/memory_client.rs".to_owned(), 1),
        ("fems/memory_proposal.rs".to_owned(), 1),
        ("interop/calendar_interop.rs".to_owned(), 1),
        ("interop/locus_interop.rs".to_owned(), 1),
        ("loom_address.rs".to_owned(), 1),
        ("rich_editor/daily_notes/journal_store.rs".to_owned(), 1),
    ]);
    assert_eq!(
        raw_new_counts, expected_raw_new_counts,
        "frontend-wide raw-client inventory changed: only the explicitly audited isolated test/base-URL seams may mint a fresh pool"
    );
    assert_eq!(
        private_builder_counts,
        std::collections::BTreeMap::from([
            ("backend_client.rs".to_owned(), 1),
            ("rich_editor/embeds/asset_resolver.rs".to_owned(), 1),
        ]),
        "only the canonical backend builder and the explicitly bounded binary-asset resolver may own a ClientBuilder"
    );
    let asset_resolver = frontend_sources
        .get("rich_editor/embeds/asset_resolver.rs")
        .expect("asset resolver source is in the frontend-wide audit");
    let asset_builder = extract_fn_body(asset_resolver, "fn with_timeouts(")
        .expect("asset resolver private builder is isolated in with_timeouts");
    assert!(
        asset_builder.contains(".connect_timeout(") && asset_builder.contains(".timeout("),
        "the sole bounded-private non-shared HTTP exception must retain connect and request deadlines"
    );

    for (path, function) in [
        ("app.rs", "fn drive_flight_recorder_pane(&mut self"),
        ("project_tabs.rs", "pub async fn fetch_workspaces("),
        ("project_tree.rs", "pub async fn load_project_content("),
        ("interop/stage_interop.rs", "pub fn with_base_url("),
    ] {
        let source = frontend_sources
            .get(path)
            .unwrap_or_else(|| panic!("frontend-wide audit source missing {path}"));
        let body = extract_fn_body(source, function)
            .unwrap_or_else(|| panic!("frontend-wide audit cannot extract {function} from {path}"));
        assert!(
            body.contains("shared_http_client"),
            "live UI-mounted HTTP path {path}::{function} must use the canonical shared bounded pool"
        );
        assert!(
            !body.contains("reqwest::Client::new") && !body.contains("reqwest::get("),
            "live UI-mounted HTTP path {path}::{function} must not mint a private default client"
        );
    }
    for (path, client) in [
        ("workspace_settings.rs", "SettingsClient"),
        ("quick_switcher.rs", "LoomGraphSearchClient"),
        ("event_emitter.rs", "RuntimeChatLedgerTransport"),
        ("rich_editor/save/save_manager.rs", "ReqwestSaveBackend"),
        ("rich_editor/save/draft_manager.rs", "ReqwestDraftBackend"),
        ("interop/cross_ref.rs", "FindNotesHttp"),
        ("rich_editor/wikilinks/client.rs", "ReqwestWikilinkBackend"),
    ] {
        let source = frontend_sources
            .get(path)
            .unwrap_or_else(|| panic!("frontend-wide audit source missing {path}"));
        let body = extract_client_constructor(source, client).unwrap_or_else(|| {
            panic!("frontend-wide audit cannot extract {client}::new from {path}")
        });
        assert!(
            body.contains("shared_http_client"),
            "live UI-mounted HTTP path {path}::{client}::new must use the canonical shared bounded pool"
        );
        assert!(
            !body.contains("reqwest::Client::new") && !body.contains("reqwest::get("),
            "live UI-mounted HTTP path {path}::{client}::new must not mint a private default client"
        );
    }

    // `LoomBlockResolver` has an explicit injected-client seam for its in-module socket tests, but its
    // public and production constructors are live product paths and must retain the shared bounded pool.
    let loom_address = frontend_sources
        .get("loom_address.rs")
        .expect("loom-address source is in the frontend-wide audit");
    let resolver_impl = extract_braced_region(loom_address, "impl LoomBlockResolver {")
        .expect("loom_address.rs declares impl LoomBlockResolver");
    let resolver_new = extract_fn_body(resolver_impl, "pub fn new(")
        .expect("LoomBlockResolver::new is extractable");
    assert!(
        resolver_new.contains("shared_http_client") && resolver_new.contains("with_client"),
        "LoomBlockResolver::new must inject the canonical shared bounded pool into its explicit client seam"
    );
    let resolver_production = extract_fn_body(resolver_impl, "pub fn production(")
        .expect("LoomBlockResolver::production is extractable");
    assert!(
        resolver_production.contains("Self::new"),
        "LoomBlockResolver::production must delegate to the audited shared-pool constructor"
    );

    // Configurable base URLs are not automatically test seams. The mounted Notes editor constructs
    // these two adapters with `HandshakeApp::rich_doc_base_url`, so both must inject the canonical
    // bounded pool into the otherwise test-oriented `KnowledgeDocumentsClient::with_base_url` boundary.
    for client in ["RichDocSaveBackend", "RichDocDraftBackend"] {
        let implementation = extract_braced_region(&client_src, &format!("impl {client} {{"))
            .unwrap_or_else(|| panic!("backend_client.rs declares impl {client}"));
        let constructor = extract_fn_body(implementation, "pub fn new(")
            .unwrap_or_else(|| panic!("{client}::new constructor must be extractable"));
        assert!(
            constructor.contains("KnowledgeDocumentsClient::with_client")
                && constructor.contains("shared_http_client()"),
            "mounted {client}::new must inject the canonical shared bounded client; constructor was {constructor}"
        );
        assert!(
            !constructor.contains("KnowledgeDocumentsClient::with_base_url")
                && !constructor.contains("reqwest::Client::new"),
            "mounted {client}::new must not cross the isolated fresh-client test seam"
        );
    }

    assert_no_local_artifact_dir();
}

/// A real TCP peer accepts the connection and then deliberately never returns HTTP bytes. The
/// canonical client must terminate at its request deadline while an actual Handshake frame loop keeps
/// advancing. This covers the half-open/silent-peer boundary that a refused localhost port cannot.
#[test]
fn silent_half_open_peer_is_bounded_without_freezing_frames() {
    let _guard = lock_backend_event_tests();
    let server = TestBackend::start(BackendMode::Silent);
    let base_url = server.base_url();
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    harness
        .state_mut()
        .set_backend_unreachable_for_test(&base_url);

    let heartbeat_before = harness.state().frame_counter();
    let mut slowest_frame = Duration::ZERO;
    let started = Instant::now();
    let deadline = started + CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3);
    let mut input_written = false;
    while (!harness.state().backend_is_down()
        || harness.state().layout_workers_in_flight_for_test() != 0)
        && Instant::now() < deadline
    {
        let frame_started = Instant::now();
        harness.step();
        slowest_frame = slowest_frame.max(frame_started.elapsed());
        if !input_written && server.accepted_count() > 0 {
            set_mounted_code_value(&mut harness, "typed while backend peer stayed silent");
            input_written = true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let elapsed = started.elapsed();

    assert!(
        server.accepted_count() >= 2,
        "the mounted app must drive real /health and layout sockets against the silent peer"
    );
    assert!(server.held_count() >= 1 && server.is_running(), "the peer must still hold accepted sockets after the app-side deadline; the proof must not release the server to manufacture completion");
    assert!(harness.state().backend_is_down(), "a TCP-accepting peer that emits no HTTP bytes must resolve to the mounted app's degraded state within the request deadline");
    assert_eq!(harness.state().layout_workers_in_flight_for_test(), 0, "the app-owned layout worker must complete after its bounded request error; no worker may remain detached behind the degraded UI");
    assert!(
        harness.state().frame_counter() > heartbeat_before,
        "the real frame-loop heartbeat must advance while the mounted app waits on a silent peer"
    );
    assert!(
        input_written,
        "the live code editor input must be changed while backend requests are in flight"
    );
    assert_eq!(
        harness.state().mounted_code_panel().buffer().to_string(),
        "typed while backend peer stayed silent",
        "editor input remains usable while /health and layout are bounded off-thread"
    );
    assert!(
        slowest_frame < Duration::from_millis(500),
        "silent backend workers must not stall a UI frame (slowest={slowest_frame:?})"
    );
    assert!(
        elapsed <= CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3),
        "the mounted app must fold silent-peer failures and reclaim its workers within the configured outer request bound (elapsed={elapsed:?}, configured={CLIENT_REQUEST_TIMEOUT:?})"
    );
    server.stop();
    assert_no_local_artifact_dir();
}

// ── AC-008-3 + AC-008-6 / PT-008-C: recovery fires BackendRecovered, no stuck-disconnected ────────

/// Recovery works through the mounted production app and real sockets: establish a live health/layout
/// server, remove it so the next retained-URL re-probe gets a real connection refusal, edit the mounted
/// code buffer while disconnected, then rebind a live server at the same address. The canonical health
/// oracle must emit exactly one down edge and one recovery edge; successful layout traffic must not race
/// that state, and repeated successful probes must not duplicate recovery.
#[test]
fn recovery_fires_recovered_event() {
    let _guard = lock_backend_event_tests();
    let live = TestBackend::start(BackendMode::Live);
    let address = live.address();
    let base_url = live.base_url();
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    harness
        .state_mut()
        .set_backend_unreachable_for_test(&base_url);

    // Keep one-time egui/font initialization outside the recovery responsiveness oracle. The
    // production backend URL is already rebound to this fixture, so the warm-up cannot contact an
    // unrelated listener. All down/recovery transition frames below remain under the strict 500 ms
    // per-frame bound.
    harness.step();
    step_until_phase(
        &mut harness,
        "initial-live",
        Duration::from_secs(8),
        |app| {
            app.status_bar_health_text().contains("Backend: OK")
                && app.layout_workers_in_flight_for_test() == 0
        },
    );
    assert!(
        !harness.state().backend_is_down(),
        "live /health establishes reachable state"
    );
    let (unreachable0, recovered0) = count_backend_events();

    assert_eq!(
        live.unclassified_request_count(),
        0,
        "the initial live phase must not depend on an unclassified request"
    );
    live.stop();
    step_until_phase(&mut harness, "down-edge", Duration::from_secs(8), |app| {
        app.backend_is_down()
    });
    let (unreachable1, recovered1) = count_backend_events();
    assert_eq!(
        unreachable1 - unreachable0,
        1,
        "real server loss emits exactly one BackendUnreachable edge"
    );
    assert_eq!(
        recovered1 - recovered0,
        0,
        "server loss cannot emit recovery"
    );

    let heartbeat_before_edit = harness.state().frame_counter();
    set_mounted_code_value(&mut harness, "editing survives live backend loss");
    assert_eq!(
        harness.state().mounted_code_panel().buffer().to_string(),
        "editing survives live backend loss"
    );
    assert!(
        harness.state().frame_counter() > heartbeat_before_edit,
        "heartbeat and editor input advance while disconnected"
    );

    let recovered_server = TestBackend::start_at(address, BackendMode::Live);
    step_until_phase(
        &mut harness,
        "recovered-edge",
        Duration::from_secs(10),
        |app| !app.backend_is_down() && app.status_bar_health_text().contains("Backend: OK"),
    );
    let (unreachable2, recovered2) = count_backend_events();
    assert_eq!(unreachable2 - unreachable1, 0, "recovery adds no down edge");
    assert_eq!(
        recovered2 - recovered1,
        1,
        "retained injected URL re-probe emits exactly one BackendRecovered edge"
    );

    // Allow another canonical health re-probe and successful layout traffic. Neither may duplicate the
    // edge or act as a competing global reachability writer.
    for _ in 0..130 {
        harness.step();
        std::thread::sleep(Duration::from_millis(20));
    }
    let (unreachable3, recovered3) = count_backend_events();
    assert_eq!(
        (unreachable3, recovered3),
        (unreachable2, recovered2),
        "steady recovered probes are deduplicated"
    );
    assert_eq!(
        harness.state().mounted_code_panel().buffer().to_string(),
        "editing survives live backend loss",
        "recovery does not discard local editor input"
    );
    assert_eq!(
        recovered_server.unclassified_request_count(),
        0,
        "steady recovered probes must all have classified request lines"
    );
    // Reclaim the mounted app and drain its probe before removing the recovered endpoint. Stopping the
    // peer first creates a new real down edge during teardown and can leak that probe into the next test.
    drop(harness);
    recovered_server.stop();

    assert_no_local_artifact_dir();
}

/// V4 remediation gate: one exact current-source run binds every previously separate proof surface.
/// It starts the real managed-SurrealDB `handshake_core`, mounts the real `HandshakeApp`, launches the
/// real out-of-process Palmistry watcher on the app's exact diagnostics ring, then suspends ONLY the
/// fixture-owned backend process. Suspension leaves the real listener/sockets present but prevents the
/// backend from answering, exercising the half-open/slow-response request deadline without a stub.
/// The mounted app must remain responsive, emit one down edge, and expose the degraded status through
/// canonical localhost Argus. The process is resumed and replaced by the current-source backend on the
/// same listener; the same app must recover, emit one recovered edge, and expose the reconnected status.
fn delegate_integrated_proof_to_fresh_test_process() -> bool {
    const CHILD_ENV: &str = "HANDSHAKE_MT088_INTEGRATED_CHILD";
    if std::env::var(CHILD_ENV).ok().as_deref() == Some("1") {
        return false;
    }

    // The recorder is intentionally process-global and other tests exercise it before this ignored test
    // in an `--include-ignored` profile. Re-exec the exact same current-source test binary with an exact
    // single-test filter so the headline live proof always owns a fresh recorder/ring process. This also
    // makes the governed exact-live command and the 23-test full profile exercise the identical isolation
    // boundary instead of allowing test order to decide whether Palmistry can see the ring.
    let executable =
        std::env::current_exe().expect("resolve current MT-088 test executable for re-exec");
    let mut command = std::process::Command::new(&executable);
    command
        .arg("backend_down_responsive_real_surrealdb_palmistry_argus")
        .args(["--ignored", "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW (HBR-QUIET)
    }
    let status = command.status().unwrap_or_else(|error| {
        panic!(
            "launch exact isolated MT-088 test process {}: {error}",
            executable.display()
        )
    });
    assert!(
        status.success(),
        "exact isolated MT-088 test process failed: {status}"
    );
    true
}

#[test]
#[ignore = "LIVE MT-088 V4 proof: requires current-source handshake_core + palmistry binaries, isolated \
            real SurrealDB, and canonical Argus. Run the exact governed command documented in the \
            UserManual; missing live prerequisites hard-fail and never silently skip."]
fn backend_down_responsive_real_surrealdb_palmistry_argus() {
    if delegate_integrated_proof_to_fresh_test_process() {
        return;
    }
    let _guard = lock_backend_event_tests();
    assert_no_local_artifact_dir();
    let (source_identity, source_provenance) = current_integrated_source_provenance();
    let running_test_binary = running_test_binary_provenance();

    let run_id = uuid::Uuid::new_v4().simple().to_string();
    let run_started_unix_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("proof start is after epoch")
        .as_millis();
    let owner_session = format!("mt088-v4-{run_id}");
    let artifact_dir =
        external_artifact_dir(&format!("wp-kernel-012-mt-088/integrated/run-{run_id}"));
    std::fs::create_dir_all(&artifact_dir)
        .expect("create MT-088 integrated external artifact directory");
    let artifact_dir = std::fs::canonicalize(&artifact_dir)
        .expect("canonicalize MT-088 integrated external artifact directory");
    let survivor_dir = artifact_dir.join("survivors");
    std::fs::create_dir_all(&survivor_dir).expect("create scoped Palmistry survivor directory");
    let _survivor_env = EnvGuard::set_path(diagnostics::ENV_PALMISTRY_SURVIVOR_DIR, &survivor_dir);
    let matrix_run_id = format!("run-{run_id}");
    let _matrix_run = EnvGuard::set_value("HANDSHAKE_ARGUS_MATRIX_RUN_ID", &matrix_run_id);
    let _screenshot_run = EnvGuard::set_value("HANDSHAKE_SCREENSHOT_RUN_ID", &matrix_run_id);
    let _proof_mt = EnvGuard::set_value("HANDSHAKE_PROOF_MT_ID", "MT-088");
    let _proof_scenario = EnvGuard::set_value(
        "HANDSHAKE_PROOF_PROCESS_SCENARIO_ID",
        "backend_down_responsive_real_surrealdb_palmistry_argus",
    );
    let _clear_action_receipt = EnvGuard::set_value("HANDSHAKE_PROOF_ACTION_RECEIPT_ID", "");
    let _matrix_scenario = EnvGuard::set_value(
        "HANDSHAKE_ARGUS_MATRIX_SCENARIO_ID",
        "backend_down_responsive_real_surrealdb_palmistry_argus",
    );
    let _matrix_surface =
        EnvGuard::set_value("HANDSHAKE_ARGUS_MATRIX_SURFACE", "settings-diagnostics");
    let _matrix_edge = EnvGuard::set_value("HANDSHAKE_ARGUS_MATRIX_EDGE_STATE", "down-recovered");
    let _matrix_source = EnvGuard::set_value("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA", &source_identity);
    let _matrix_correlation =
        EnvGuard::set_value("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID", &owner_session);
    let _matrix_artifacts = EnvGuard::set_path(
        "HANDSHAKE_PROOF_ARTIFACT_DIR",
        artifact_dir
            .parent()
            .expect("integrated run directory has an artifact parent"),
    );

    let palmistry_exe = find_palmistry_binary();
    let palmistry_binary_provenance = current_binary_provenance(&palmistry_exe, &["../palmistry"]);
    let session_id = uuid::Uuid::new_v4().to_string();
    let ring_path = artifact_dir.join(format!("handshake-diag-{session_id}.ring"));
    let ring_writer = DiagRingWriter::create(&ring_path, handshake_diag_ring::DEFAULT_CAPACITY)
        .expect("create durable same-run MT-088 diagnostics ring");
    assert!(
        diagnostics::install(ring_writer),
        "MT-088 integrated proof must be run as the exact single test so it can install the real \
         diagnostics ring before any process-global recorder use"
    );
    let session = diagnostics::DiagSession {
        session_id,
        ring_path,
    };
    diagnostics::set_preinstalled_diag_session(session.clone());
    let control_socket = control_socket_name(&session.session_id);
    let mut palmistry = launch_palmistry_at(
        &palmistry_exe,
        &session,
        &session.ring_path,
        &control_socket,
    )
    .expect("launch current-source Palmistry for MT-088 integrated proof");
    assert!(
        palmistry.handshake_acked(),
        "Palmistry must acknowledge the exact MT-088 session/ring before backend fault injection; error={:?}",
        palmistry.handshake_error()
    );
    let palmistry_pid = palmistry.child_id();
    assert!(
        process_is_running(palmistry_pid),
        "Palmistry child {palmistry_pid} must be alive before the mounted scenario"
    );

    let mut backend = backend_proof_support::require_live_backend();
    let backend_binary_provenance = current_binary_provenance(
        backend.owned_binary_path(),
        &["../../backend/handshake_core"],
    );
    let initial_backend_binding_receipt = backend.owned_backend_binding_receipt();
    let original_backend_pid = backend.owned_process_id();
    let backend_base = backend.base.clone();
    let backend_workspace_id = backend.workspace_id.clone();
    assert!(
        !backend_workspace_id.is_empty(),
        "the integrated mounted proof requires a real SurrealDB-backed workspace"
    );
    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_size(egui::vec2(1600.0, 1100.0))
        .build_eframe(|cc| {
            let mut app = HandshakeApp::new(cc);
            app.bind_managed_backend_for_test(&backend_base);
            app.set_active_project_id_for_test(backend_workspace_id.clone());
            replace_unrelated_sibling_panes_with_manual(&mut app, &backend_workspace_id);
            app
        });
    assert_eq!(
        harness.state().diag_session(),
        Some(&session),
        "the mounted app must reuse the exact diagnostics ring Palmistry watches"
    );
    assert_eq!(
        harness.state().active_project_id(),
        backend_workspace_id,
        "mounted proof workspace remains bound to the managed SurrealDB fixture"
    );
    let ring =
        DiagRingReader::open(&session.ring_path).expect("Palmistry-shared MT-088 ring is readable");
    let connected_phase_samples = step_until_with_ring_samples(
        &mut harness,
        &ring,
        "connected-baseline",
        Duration::from_secs(20),
        |app| {
            !app.backend_is_down()
                && app.status_bar_health_text().contains("Backend: OK")
                && app.layout_workers_in_flight_for_test() == 0
        },
    );
    harness
        .state_mut()
        .clear_fems_overlay_for_integration_test();
    harness.step();

    let mut argus = CanonicalArgusDriver::bind(
        harness.state(),
        &format!("mt088-real-backend-loss-{run_id}"),
    );
    let live_inspect = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&live_inspect, BACKEND_STATUS_AUTHOR_ID),
        "canonical Argus must address the mounted backend status node"
    );
    let live_status = json_author_value(&live_inspect, BACKEND_STATUS_AUTHOR_ID)
        .expect("canonical Argus live status has text")
        .to_owned();
    assert!(
        live_status.contains("Backend: OK"),
        "canonical Argus must observe the real managed backend as connected: {live_status:?}"
    );
    let live_json =
        serde_json::to_string(&live_inspect).expect("serialize connected-before Argus tree");
    assert!(
        !live_json.contains(handshake_native::backend_client::BACKEND_BASE_URL),
        "a managed-backend proof cannot leave mounted consumers visibly bound to the production \
         default endpoint; tree={live_json}"
    );
    for unrelated in [
        handshake_native::fems::FEMS_PROPOSE_DIALOG_AUTHOR_ID,
        handshake_native::fems::FEMS_PROPOSE_STATUS_AUTHOR_ID,
        "wikilink-alias-local-only-banner",
        handshake_native::runtime_chat::RUNTIME_CHAT_STATUS_AUTHOR_ID,
    ] {
        assert!(
            !json_has_author_id(&live_inspect, unrelated),
            "connected baseline must be free of unrelated error surface {unrelated}"
        );
    }
    let live_screenshot = save_integrated_surface(&mut harness, &artifact_dir, "connected-before");
    let live_screenshot_sha256 = file_sha256(&live_screenshot);

    let heartbeat_live = ring
        .read_heartbeat()
        .expect("mounted app publishes a heartbeat before fault injection");
    let (unreachable_before, recovered_before) = count_backend_events();
    let frame_before_fault = harness.state().frame_counter();
    let backend_port = reqwest::Url::parse(&backend_base)
        .expect("fixture backend base is a URL")
        .port()
        .expect("fixture backend uses an explicit ephemeral port") as u64;

    // The exact current-source backend process is suspended, not replaced by a mock. Its listening
    // socket stays present, so real client connects become half-open/slow until the 10s request bound.
    let suspended = SuspendedProcessGuard::suspend(original_backend_pid);
    let fault_started_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos() as u64;
    // Rebind to the same real endpoint after suspension. This deliberately starts a fresh production
    // WorkbenchLayoutClient load as well as a health probe, so the integrated proof crosses the exact
    // layout-load path that originally froze the frame loop.
    harness
        .state_mut()
        .set_backend_endpoints_for_test(&backend_base, &backend_base);
    let fault_layout_generation = harness.state().layout_load_ownership_for_test().0;
    assert!(
        harness.state().layout_load_ownership_for_test().2,
        "real suspended-backend phase must begin with a fresh layout worker in flight"
    );
    let fault_phase_samples = step_until_with_ring_samples(
        &mut harness,
        &ring,
        "backend-suspended",
        CLIENT_REQUEST_TIMEOUT + Duration::from_secs(8),
        |app| app.backend_is_down() && app.layout_workers_in_flight_for_test() == 0,
    );
    let worst_fault_frame_micros = fault_phase_samples
        .iter()
        .map(|sample| sample.frame_elapsed_micros)
        .max()
        .expect("suspended phase samples at least one frame");
    assert!(
        harness.state().backend_is_down(),
        "suspended real backend must resolve to finite degraded state within the request deadline"
    );
    assert_eq!(
        harness.state().layout_workers_in_flight_for_test(),
        0,
        "suspended real backend must leave no app-owned layout worker after the request deadline"
    );
    assert!(
        harness.state().frame_counter() > frame_before_fault,
        "the mounted UI heartbeat must advance during real backend suspension"
    );
    assert!(
        process_is_running(palmistry_pid),
        "out-of-process Palmistry must survive and remain live during backend suspension"
    );

    let down_inspect = argus.inspect(&mut harness);
    let down_status = json_author_value(&down_inspect, BACKEND_STATUS_AUTHOR_ID)
        .expect("canonical Argus degraded status has text")
        .to_owned();
    assert!(
        down_status.contains("Disconnected") && down_status.contains("UI responsive"),
        "canonical Argus must observe the finite degraded status, got {down_status:?}"
    );
    let operator_menu = argus.click_and_reinspect(
        &mut harness,
        handshake_native::top_menu_bar::MenuId::Operator.author_id(),
    );
    assert!(
        json_has_author_id(&operator_menu.after, "menu.operator.settings"),
        "canonical Argus must expose OPERATOR -> Open Settings before navigating"
    );
    let operator_menu_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "operator-menu-open-exposes-settings",
        |tree| json_has_author_id(tree, "menu.operator.settings"),
    );
    let settings_open = argus.click_and_reinspect(&mut harness, "menu.operator.settings");
    assert!(
        harness.state().settings_open(),
        "canonical Argus OPERATOR -> Open Settings must open the real Settings overlay"
    );
    let settings_open_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "settings-open-exposes-search",
        |tree| {
            json_has_author_id(
                tree,
                handshake_native::settings_dialog::SETTINGS_SEARCH_AUTHOR_ID,
            )
        },
    );
    let down_search = argus.set_value_and_reinspect(
        &mut harness,
        handshake_native::settings_dialog::SETTINGS_SEARCH_AUTHOR_ID,
        "diagnostics",
    );
    let down_diagnostics_inspect = down_search.after.clone();
    assert_eq!(
        json_author_value(
            &down_diagnostics_inspect,
            handshake_native::settings_dialog::SETTINGS_SEARCH_AUTHOR_ID,
        ),
        Some("diagnostics"),
        "canonical Argus must filter the mounted Settings overlay to the Diagnostics section before capture"
    );
    let mut down_diagnostics_author_ids = Vec::new();
    json_author_ids(&down_diagnostics_inspect, &mut down_diagnostics_author_ids);
    for author_id in [
        diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID,
        diagnostics::DIAGNOSTICS_EVENTS_AUTHOR_ID,
        diagnostics::DIAGNOSTICS_PALMISTRY_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&down_diagnostics_inspect, author_id),
            "canonical Argus must observe affected Settings -> Diagnostics node {author_id} while down; \
             available Settings/diagnostics ids={:?}",
            down_diagnostics_author_ids
                .iter()
                .filter(|id| id.contains("settings") || id.contains("diagnostics"))
                .collect::<Vec<_>>()
        );
    }
    let down_diagnostics_json =
        serde_json::to_string(&down_diagnostics_inspect).expect("serialize down diagnostics tree");
    assert!(
        down_diagnostics_json.contains("BackendUnreachable"),
        "canonical Argus Settings diagnostics tree must project the typed down-edge label"
    );
    assert!(
        down_diagnostics_json.contains("Shared-memory ring active"),
        "canonical Argus Settings diagnostics tree must expose Tier-3 ring visibility"
    );
    let down_settings_phase_samples = step_until_with_ring_samples(
        &mut harness,
        &ring,
        "down-settings-terminal",
        Duration::from_secs(15),
        |app| app.settings_persist_error().is_some(),
    );
    let down_settings_error = harness
        .state()
        .settings_persist_error()
        .expect("the real Settings load must expose its bounded backend-down failure");
    assert!(
        down_settings_error.contains(&backend_base),
        "the mounted Settings failure must come from the same suspended managed backend, got \
         {down_settings_error:?}"
    );
    assert!(
        down_settings_error.contains(&backend_workspace_id),
        "the mounted Settings failure must identify the exact managed workspace, got {down_settings_error:?}"
    );
    assert!(
        !down_settings_error.contains(handshake_native::backend_client::BACKEND_BASE_URL),
        "the mounted Settings failure cannot come from the unrelated production default endpoint"
    );
    scroll_settings_diagnostics_to_events(&mut harness);
    let down_identity_bound = harness.state().diag_session() == Some(&session)
        && harness.state().active_project_id() == backend_workspace_id
        && harness.state().backend_is_down();
    let expected_down_endpoint = backend_base.clone();
    let expected_down_workspace = backend_workspace_id.clone();
    let expected_down_session = session.session_id.clone();
    let down_terminal_inspect = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "degraded-diagnostics-exact-session-endpoint-workspace",
        serde_json::json!({
            "endpoint": expected_down_endpoint,
            "workspace_id": expected_down_workspace,
            "diagnostics_session_id": expected_down_session,
            "expected_status": "Disconnected — UI responsive",
            "expected_event": "BackendUnreachable",
            "expected_ring": "Shared-memory ring active",
        }),
        |tree| {
            let json = serde_json::to_string(tree).expect("serialize terminal down tree");
            down_identity_bound
                && json_has_author_id(tree, BACKEND_STATUS_AUTHOR_ID)
                && json_has_author_id(tree, diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID)
                && json_has_author_id(tree, diagnostics::DIAGNOSTICS_EVENTS_AUTHOR_ID)
                && json_has_author_id(tree, diagnostics::DIAGNOSTICS_PALMISTRY_AUTHOR_ID)
                && json_has_author_id(
                    tree,
                    handshake_native::settings_dialog::SETTINGS_PERSIST_ERROR_AUTHOR_ID,
                )
                && json.contains("Disconnected")
                && json.contains("UI responsive")
                && json.contains("BackendUnreachable")
                && json.contains("Shared-memory ring active")
                && json.contains(&backend_base)
                && json.contains(&backend_workspace_id)
        },
    );
    let down_screenshot = {
        let _action_receipt = EnvGuard::set_value(
            "HANDSHAKE_PROOF_ACTION_RECEIPT_ID",
            down_search.receipt_id.to_string(),
        );
        save_integrated_surface(&mut harness, &artifact_dir, "disconnected")
    };
    let down_screenshot_sha256 = file_sha256(&down_screenshot);
    assert_ne!(
        down_screenshot_sha256, live_screenshot_sha256,
        "the mounted disconnected Diagnostics state must render differently from the connected surface"
    );
    let heartbeat_down = ring
        .read_heartbeat()
        .expect("Palmistry-shared heartbeat remains readable during backend suspension");
    assert!(
        heartbeat_down.counter > heartbeat_live.counter,
        "the exact Palmistry-shared heartbeat must advance during backend suspension"
    );
    let ring_down_events = ring.read_last_n(64);
    let ring_down_event = ring_down_events
        .iter()
        .filter(|event| {
            event.event_code == DiagEventCode::BackendUnreachable.as_u16()
                && event.counter_a == backend_port
                && event.phase_marker == DiagPhase::Degraded.as_u8()
                && event.severity == DiagSeverity::Error.as_u8()
                && event.timestamp_nanos >= fault_started_nanos
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ring_down_event.len(),
        1,
        "same-run ring must contain exactly one newly timestamped BackendUnreachable edge for real port \
         {backend_port}; events={ring_down_events:?}"
    );
    let (unreachable_down, recovered_down) = count_backend_events();
    assert_eq!(
        unreachable_down - unreachable_before,
        1,
        "real suspended-backend transition emits exactly one BackendUnreachable"
    );
    assert_eq!(
        recovered_down - recovered_before,
        0,
        "backend suspension cannot emit recovery"
    );

    // Resume the exact owned process only long enough to make termination safe, then restart the
    // current-source backend with the same SurrealDB authority and exact listener. The app retains its
    // URL throughout; no test seam fabricates the recovered state.
    suspended.resume();
    let recovery_started_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos() as u64;
    let (old_base, new_base) = backend.restart_owned();
    let restarted_backend_pid = backend.owned_process_id();
    assert_eq!(old_base, backend_base);
    assert_eq!(new_base, backend_base);
    assert_ne!(
        restarted_backend_pid, original_backend_pid,
        "real backend restart must replace the exact process"
    );
    let recovery_backend_phase_samples = step_until_with_ring_samples(
        &mut harness,
        &ring,
        "backend-restarted",
        Duration::from_secs(20),
        |app| !app.backend_is_down() && app.status_bar_health_text().contains("Backend: OK"),
    );
    assert!(
        process_is_running(palmistry_pid),
        "out-of-process Palmistry must survive through backend restart and reconnect"
    );
    let settings_retry = argus.click_and_reinspect(
        &mut harness,
        handshake_native::settings_dialog::SETTINGS_PERSIST_RETRY_AUTHOR_ID,
    );
    assert!(
        matches!(
            settings_retry.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "canonical Argus must dispatch the visible Settings recovery action, got {}",
        settings_retry.receipt_status
    );
    let recovery_settings_phase_samples = step_until_with_ring_samples(
        &mut harness,
        &ring,
        "recovered-settings-terminal",
        Duration::from_secs(15),
        |app| app.settings_persist_error().is_none(),
    );
    let ring_recovered_events = ring.read_last_n(64);
    let ring_recovered_event = ring_recovered_events
        .iter()
        .filter(|event| {
            event.event_code == DiagEventCode::BackendRecovered.as_u16()
                && event.counter_a == backend_port
                && event.phase_marker == DiagPhase::Recovered.as_u8()
                && event.severity == DiagSeverity::Info.as_u8()
                && event.timestamp_nanos >= recovery_started_nanos
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ring_recovered_event.len(),
        1,
        "same-run ring must contain exactly one newly timestamped BackendRecovered edge for real port \
         {backend_port}; events={ring_recovered_events:?}"
    );
    let recovered_identity_bound = harness.state().diag_session() == Some(&session)
        && harness.state().active_project_id() == backend_workspace_id
        && !harness.state().backend_is_down()
        && restarted_backend_pid != original_backend_pid;
    let recovered_inspect = argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "recovered-diagnostics-exact-session-endpoint-workspace",
        serde_json::json!({
            "endpoint": backend_base,
            "workspace_id": backend_workspace_id,
            "diagnostics_session_id": session.session_id,
            "restarted_backend_pid": restarted_backend_pid,
            "expected_status": "Backend: OK",
            "expected_event": "BackendRecovered",
            "expected_ring": "Shared-memory ring active",
            "settings_error_absent": true,
        }),
        |tree| {
            let json = serde_json::to_string(tree).expect("serialize terminal recovered tree");
            recovered_identity_bound
                && json_has_author_id(tree, BACKEND_STATUS_AUTHOR_ID)
                && json_has_author_id(tree, diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID)
                && json_has_author_id(tree, diagnostics::DIAGNOSTICS_EVENTS_AUTHOR_ID)
                && json_has_author_id(tree, diagnostics::DIAGNOSTICS_PALMISTRY_AUTHOR_ID)
                && !json_has_author_id(
                    tree,
                    handshake_native::settings_dialog::SETTINGS_PERSIST_ERROR_AUTHOR_ID,
                )
                && json.contains("Backend: OK")
                && json.contains("BackendRecovered")
                && json.contains("Shared-memory ring active")
        },
    );
    let recovered_status = json_author_value(&recovered_inspect, BACKEND_STATUS_AUTHOR_ID)
        .expect("canonical Argus recovered status has text")
        .to_owned();
    assert!(
        recovered_status.contains("Backend: OK"),
        "canonical Argus must re-observe the same mounted status as recovered: {recovered_status:?}"
    );
    for author_id in [
        diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID,
        diagnostics::DIAGNOSTICS_EVENTS_AUTHOR_ID,
        diagnostics::DIAGNOSTICS_PALMISTRY_AUTHOR_ID,
    ] {
        assert!(
            json_has_author_id(&recovered_inspect, author_id),
            "canonical Argus must retain affected Settings -> Diagnostics node {author_id} after recovery"
        );
    }
    let recovered_diagnostics_json =
        serde_json::to_string(&recovered_inspect).expect("serialize recovered diagnostics tree");
    assert!(
        recovered_diagnostics_json.contains("BackendRecovered"),
        "canonical Argus Settings diagnostics tree must project the typed recovery-edge label"
    );
    assert!(
        recovered_diagnostics_json.contains("Shared-memory ring active"),
        "canonical Argus Settings diagnostics tree must retain Tier-3 ring visibility after recovery"
    );
    assert!(
        !recovered_diagnostics_json
            .contains(handshake_native::settings_dialog::SETTINGS_PERSIST_ERROR_AUTHOR_ID),
        "the mounted Settings recovery action must clear its degraded-state error before the \
         reconnected capture"
    );
    let recovered_screenshot = {
        let _action_receipt = EnvGuard::set_value(
            "HANDSHAKE_PROOF_ACTION_RECEIPT_ID",
            settings_retry.receipt_id.to_string(),
        );
        save_integrated_surface(&mut harness, &artifact_dir, "reconnected")
    };
    let recovered_screenshot_sha256 = file_sha256(&recovered_screenshot);
    assert_ne!(
        recovered_screenshot_sha256, down_screenshot_sha256,
        "the mounted recovered Diagnostics state must render differently from the disconnected surface"
    );
    let heartbeat_recovered = ring
        .read_heartbeat()
        .expect("Palmistry-shared heartbeat remains readable after reconnect");
    assert!(
        heartbeat_recovered.counter > heartbeat_down.counter,
        "the exact Palmistry-shared heartbeat must advance after reconnect"
    );
    let (unreachable_after, recovered_after) = count_backend_events();
    assert_eq!(unreachable_after - unreachable_before, 1);
    assert_eq!(
        recovered_after - recovered_before,
        1,
        "real backend restart/reconnect emits exactly one BackendRecovered"
    );

    let incident_ring_events = ring
        .read_last_n(128)
        .into_iter()
        .filter(|event| {
            matches!(
                event.event_code,
                code if code == DiagEventCode::FreezeSuspected.as_u16()
                    || code == DiagEventCode::CrashDetected.as_u16()
            )
        })
        .collect::<Vec<_>>();
    assert!(
        incident_ring_events.is_empty(),
        "advancing heartbeat must not produce a false freeze/crash ring incident: {incident_ring_events:?}"
    );
    let receipt_ids = [
        operator_menu.receipt_id,
        settings_open.receipt_id,
        down_search.receipt_id,
        settings_retry.receipt_id,
    ];
    assert_eq!(
        receipt_ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4,
        "all four governed Argus actions must carry unique receipts"
    );
    assert_eq!(
        harness.state().layout_workers_in_flight_for_test(),
        0,
        "all app-owned layout workers must be reaped before Argus/app teardown"
    );
    let down_terminal_tree_sha256 = json_sha256(&down_terminal_inspect);
    let recovered_terminal_tree_sha256 = json_sha256(&recovered_inspect);
    let (screenshot_marker_path, screenshot_marker_sha256, screenshot_marker_rows) =
        validate_integrated_screenshot_markers(
            &artifact_dir,
            &matrix_run_id,
            &source_identity,
            &owner_session,
            [
                (&live_screenshot, &live_screenshot_sha256),
                (&down_screenshot, &down_screenshot_sha256),
                (&recovered_screenshot, &recovered_screenshot_sha256),
            ],
            [
                None,
                Some(down_search.receipt_id),
                Some(settings_retry.receipt_id),
            ],
        );
    argus.finish();
    let canonical_trace_path = artifact_dir.join("canonical-argus-matrix.jsonl");
    let canonical_trace_bytes = std::fs::read(&canonical_trace_path).unwrap_or_else(|error| {
        panic!(
            "read canonical Argus trace {} after finish: {error}",
            canonical_trace_path.display()
        )
    });
    let canonical_trace_rows = String::from_utf8(canonical_trace_bytes.clone())
        .expect("canonical Argus trace is UTF-8")
        .lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).expect("decode Argus trace row")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        canonical_trace_rows.len(),
        4,
        "trace contains all four actions"
    );
    for (index, row) in canonical_trace_rows.iter().enumerate() {
        assert_eq!(row["receipt_id"], receipt_ids[index]);
        assert_eq!(row["terminal_refreshed"], true);
        assert_eq!(row["run_id"], matrix_run_id);
        assert_eq!(row["source_sha"], source_identity);
        assert_eq!(row["process_correlation_id"], owner_session);
        assert!(
            row["terminal_predicates"]
                .as_array()
                .is_some_and(|predicates| {
                    !predicates.is_empty()
                        && predicates
                            .iter()
                            .all(|predicate| predicate["passed"] == true)
                }),
            "trace row {index} must carry a passing terminal predicate: {row}"
        );
    }
    let canonical_trace_path =
        std::fs::canonicalize(&canonical_trace_path).expect("canonicalize finished Argus trace");
    let canonical_trace_sha256 = format!("{:x}", Sha256::digest(&canonical_trace_bytes));
    drop(harness);
    let palmistry_shutdown = palmistry.request_shutdown_and_wait(Duration::from_secs(10));
    match palmistry_shutdown {
        ShutdownOutcome::ExitedCleanly(status) => {
            assert!(
                status.success(),
                "Palmistry clean shutdown failed: {status:?}"
            )
        }
        other => panic!("Palmistry must persist a clean same-run survivor receipt: {other:?}"),
    }
    assert!(
        !process_is_running(palmistry_pid),
        "clean Palmistry shutdown must reap the exact proof-owned child"
    );
    let palmistry_survivor =
        artifact_dir.join(format!("palmistry-survivor-{}.json", session.session_id));
    let survivor_json: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&palmistry_survivor)
            .unwrap_or_else(|error| panic!("read {}: {error}", palmistry_survivor.display())),
    )
    .expect("decode same-run Palmistry survivor receipt");
    assert_eq!(survivor_json["session_id"], session.session_id);
    assert_eq!(
        survivor_json["exit_reason"],
        serde_json::json!({ "reason": "CleanShutdown" })
    );
    assert_eq!(survivor_json["abnormal_parent_exit"], false);
    assert_eq!(survivor_json["shutdown_received"], true);
    assert_eq!(survivor_json["parent_exit_code"], serde_json::Value::Null);
    assert_eq!(
        survivor_json["parent_pid"],
        std::process::id(),
        "Palmistry lifecycle survivor must bind to this exact test parent"
    );
    let palmistry_survivor = std::fs::canonicalize(&palmistry_survivor)
        .expect("canonicalize Palmistry lifecycle survivor");
    let palmistry_survivor_sha256 = file_sha256(&palmistry_survivor);
    let incident_survivors = incident_survivor_files(&survivor_dir);
    assert!(
        incident_survivors.is_empty(),
        "healthy advancing heartbeat must produce zero Freeze/Crash/ChildStall incident survivors: {incident_survivors:?}"
    );
    let restarted_backend_binding_receipt = backend.owned_backend_binding_receipt();
    let backend_runtime_roots = backend.owned_runtime_roots_for_proof();
    backend.assert_cleanup();
    assert!(
        !process_is_running(restarted_backend_pid),
        "backend cleanup must reap the exact restarted proof-owned child"
    );
    drop(backend);
    for runtime_root in &backend_runtime_roots {
        assert!(
            !runtime_root.exists(),
            "backend runtime root must be removed before evidence publication: {}",
            runtime_root.display()
        );
    }
    let canonical_ring_path = std::fs::canonicalize(&session.ring_path)
        .expect("canonicalize same-run diagnostics ring after child teardown");

    let evidence_path = artifact_dir.join("mt088-integrated-canonical-evidence.json");
    let evidence = serde_json::json!({
        "schema_id": "handshake.mt088-integrated-proof.v4",
        "run_id": matrix_run_id,
        "argus_teardown_verified": true,
        "runtime_binding": {
            "workspace_id": backend_workspace_id,
            "diagnostics_session_id": session.session_id,
            "ring_canonical_path": canonical_ring_path,
            "control_socket": control_socket,
        },
        "ownership": {
            "owner_session": owner_session,
            "owner_wp": "WP-KERNEL-012",
            "owner_mt": "MT-088",
            "owner_role": "KERNEL_BUILDER",
            "started_at_unix_millis": run_started_unix_millis,
            "palmistry_child_reaped": true,
            "backend_child_reaped": true,
            "orphan_reclamation_verified": true,
        },
        "source_identity": source_identity,
        "source_provenance": source_provenance,
        "running_test_binary": running_test_binary,
        "managed_surrealdb": true,
        "current_source_backend": {
            "base_url": backend_base,
            "workspace_id": backend_workspace_id,
            "binary": backend_binary_provenance,
            "initial_binding_receipt": initial_backend_binding_receipt,
            "restarted_binding_receipt": restarted_backend_binding_receipt,
            "suspended_pid": original_backend_pid,
            "restarted_pid": restarted_backend_pid,
            "restart_reused_exact_listener": old_base == new_base,
            "cleanup_verified": true,
            "runtime_roots": backend_runtime_roots,
            "runtime_roots_deleted_before_evidence": true,
        },
        "fault": {
            "mechanism": "OS suspension of the fixture-owned current-source handshake_core process",
            "network_effect": "real listener remains present while accepted requests receive no application bytes until the bounded client deadline",
            "slowest_ui_frame_micros": worst_fault_frame_micros,
            "request_deadline_millis": CLIENT_REQUEST_TIMEOUT.as_millis(),
            "fresh_layout_generation": fault_layout_generation,
            "fresh_layout_worker_drained": true,
        },
        "internal_diagnostics": {
            "session_id": session.session_id,
            "ring_path": session.ring_path,
            "heartbeat_connected": heartbeat_live.counter,
            "heartbeat_disconnected": heartbeat_down.counter,
            "heartbeat_reconnected": heartbeat_recovered.counter,
            "backend_unreachable_delta": unreachable_after - unreachable_before,
            "backend_recovered_delta": recovered_after - recovered_before,
            "backend_unreachable_ring_records": ring_down_event,
            "backend_recovered_ring_records": ring_recovered_event,
            "false_freeze_crash_ring_events": incident_ring_events,
        },
        "palmistry": {
            "binary": palmistry_binary_provenance,
            "pid": palmistry_pid,
            "handshake_acked": true,
            "alive_during_backend_suspension": true,
            "alive_after_backend_restart": true,
            "parent_pid": std::process::id(),
            "survivor_receipt": {
                "canonical_path": palmistry_survivor,
                "sha256": palmistry_survivor_sha256,
            },
            "survivor": survivor_json,
            "incident_survivor_count": incident_survivors.len(),
            "incident_survivors": incident_survivors,
            "no_false_freeze_crash_child_stall": true,
        },
        "canonical_argus": {
            "transport": "SwarmMcpServer localhost JSON-RPC",
            "author_id": BACKEND_STATUS_AUTHOR_ID,
            "trace": {
                "canonical_path": canonical_trace_path,
                "sha256": canonical_trace_sha256,
                "row_count": canonical_trace_rows.len(),
            },
            "actions": [
                {
                    "ordinal": 1,
                    "target": handshake_native::top_menu_bar::MenuId::Operator.author_id(),
                    "receipt_id": operator_menu.receipt_id,
                    "receipt_status": operator_menu.receipt_status,
                    "terminal_tree_sha256": json_sha256(&operator_menu_terminal),
                    "terminal_predicate": "operator-menu-open-exposes-settings",
                },
                {
                    "ordinal": 2,
                    "target": "menu.operator.settings",
                    "receipt_id": settings_open.receipt_id,
                    "receipt_status": settings_open.receipt_status,
                    "terminal_tree_sha256": json_sha256(&settings_open_terminal),
                    "terminal_predicate": "settings-open-exposes-search",
                },
                {
                    "ordinal": 3,
                    "target": handshake_native::settings_dialog::SETTINGS_SEARCH_AUTHOR_ID,
                    "value": "diagnostics",
                    "receipt_id": down_search.receipt_id,
                    "receipt_status": down_search.receipt_status,
                    "terminal_tree_sha256": down_terminal_tree_sha256,
                    "terminal_predicate": "degraded-diagnostics-exact-session-endpoint-workspace",
                    "paired_frame": {
                        "path": down_screenshot,
                        "sha256": down_screenshot_sha256,
                    },
                },
                {
                    "ordinal": 4,
                    "target": handshake_native::settings_dialog::SETTINGS_PERSIST_RETRY_AUTHOR_ID,
                    "receipt_id": settings_retry.receipt_id,
                    "receipt_status": settings_retry.receipt_status,
                    "terminal_tree_sha256": recovered_terminal_tree_sha256,
                    "terminal_predicate": "recovered-diagnostics-exact-session-endpoint-workspace",
                    "paired_frame": {
                        "path": recovered_screenshot,
                        "sha256": recovered_screenshot_sha256,
                    },
                },
            ],
            "connected_before": {"status": live_status, "inspect": live_inspect},
            "disconnected": {
                "status": down_status,
                "status_inspect": down_inspect,
                "settings_diagnostics_inspect": down_diagnostics_inspect,
            },
            "reconnected": {"status": recovered_status, "inspect": recovered_inspect},
            "teardown_verified": true,
        },
        "frame_heartbeat_samples": {
            "per_frame_ceiling_micros": 500_000,
            "strict_heartbeat_advance": true,
            "connected_baseline": connected_phase_samples,
            "backend_suspended": fault_phase_samples,
            "down_settings_terminal": down_settings_phase_samples,
            "backend_restarted": recovery_backend_phase_samples,
            "recovered_settings_terminal": recovery_settings_phase_samples,
        },
        "mounted_surface_renders": [
            {"state": "connected-before", "path": live_screenshot, "sha256": live_screenshot_sha256},
            {"state": "disconnected-settings-diagnostics", "path": down_screenshot, "sha256": down_screenshot_sha256},
            {"state": "reconnected-settings-diagnostics", "path": recovered_screenshot, "sha256": recovered_screenshot_sha256},
        ],
        "screenshot_markers": {
            "canonical_path": screenshot_marker_path,
            "sha256": screenshot_marker_sha256,
            "row_count": screenshot_marker_rows.len(),
            "rows": screenshot_marker_rows,
            "pre_action_marker_has_no_receipt": true,
            "receipt_bound_markers": [down_search.receipt_id, settings_retry.receipt_id],
            "marker_frames_hash_match_operator_pngs": true,
        },
    });
    std::fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&evidence).expect("serialize MT-088 integrated evidence"),
    )
    .expect("write MT-088 integrated evidence");
    assert!(evidence_path.is_file());
    assert_no_local_artifact_dir();
    eprintln!(
        "MT-088 INTEGRATED PASS: real SurrealDB/backend suspension+restart, mounted heartbeat/events, \
         Palmistry survivor, canonical Argus down/recovered; evidence={}",
        evidence_path.display()
    );
}

/// A 2xx body is not healthy merely because it is valid JSON. Both `/health` and the layout route must
/// reject semantically incomplete objects, and the mounted app must treat malformed health as down.
#[test]
fn malformed_success_bodies_fail_closed() {
    let _guard = lock_backend_event_tests();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build decoder proof runtime");

    let health_server = TestBackend::start(BackendMode::MalformedHealth);
    let health_url = format!("{}/health", health_server.base_url());
    let health_error = runtime
        .block_on(fetch_health(&health_url))
        .expect_err("{} is not a semantically valid health response");
    assert!(
        health_error.to_string().contains("status"),
        "typed health decode error identifies the missing required status field: {health_error}"
    );

    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    harness
        .state_mut()
        .set_backend_unreachable_for_test(&health_server.base_url());
    step_until(&mut harness, Duration::from_secs(8), |app| {
        app.backend_is_down()
    });
    assert!(
        harness
            .state()
            .status_bar_health_text()
            .contains("Disconnected"),
        "mounted app degrades on malformed 2xx health instead of falsely recovering"
    );
    health_server.stop();

    for (mode, expected_field) in [
        (BackendMode::UnknownHealthStatus, "`status`"),
        (BackendMode::UnknownDbStatus, "`db_status`"),
        (BackendMode::InconsistentHealth, "inconsistent"),
    ] {
        let server = TestBackend::start(mode);
        let url = format!("{}/health", server.base_url());
        let error = runtime
            .block_on(fetch_health(&url))
            .expect_err("unknown or producer-inconsistent health enums must fail closed");
        assert!(
            error.to_string().contains(expected_field),
            "strict health decode identifies {expected_field}: {error}"
        );
        server.stop();
    }

    let layout_server = TestBackend::start(BackendMode::MalformedLayout);
    let layout_client =
        WorkbenchLayoutClient::new(layout_server.base_url(), runtime.handle().clone());
    let layout_error = layout_client
        .load("default-project")
        .expect_err("{} is not a semantically valid WorkbenchLayoutResponse");
    assert!(layout_error.to_string().contains("layout_state"), "typed layout decode error identifies the missing required layout_state field: {layout_error}");
    layout_server.stop();
    assert_no_local_artifact_dir();
}

/// `/health` is the single global reachability authority. A successful or failed layout request may
/// update layout-persistence status, but must never race the global BackendUnreachable/Recovered edges.
#[test]
fn split_health_and_layout_sources_do_not_compete_for_reachability() {
    let _guard = lock_backend_event_tests();
    let health_live = TestBackend::start(BackendMode::Live);
    let layout_silent = TestBackend::start(BackendMode::Silent);
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    harness
        .state_mut()
        .set_backend_endpoints_for_test(&health_live.base_url(), &layout_silent.base_url());
    step_until(
        &mut harness,
        CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3),
        |app| {
            app.status_bar_health_text().contains("Backend: OK")
                && app.layout_workers_in_flight_for_test() == 0
                && layout_silent.held_count() >= 1
        },
    );
    assert!(
        !harness.state().backend_is_down(),
        "layout timeout cannot overwrite the live canonical health result"
    );
    assert!(
        layout_silent.held_count() >= 1,
        "split proof drove the real layout socket"
    );

    let dead_health = reserve_unused_address();
    let layout_live = TestBackend::start(BackendMode::Live);
    harness
        .state_mut()
        .set_backend_endpoints_for_test(&format!("http://{dead_health}"), &layout_live.base_url());
    step_until(&mut harness, Duration::from_secs(8), |app| {
        app.backend_is_down()
            && app.layout_workers_in_flight_for_test() == 0
            && layout_live.accepted_count() >= 1
    });
    assert!(
        harness.state().backend_is_down(),
        "successful layout load cannot recover a failed canonical health source"
    );

    health_live.stop();
    layout_silent.stop();
    layout_live.stop();
    assert_no_local_artifact_dir();
}

/// Endpoint replacement must retire an old layout worker without waiting for it. The old worker is
/// deterministically paused AFTER its real HTTP result but BEFORE the generation-guarded publication;
/// the replacement endpoint then returns a visibly distinct valid snapshot and settles first. Releasing
/// the exact paused old generation afterward must neither overwrite that snapshot nor move the
/// ownership-clear marker backward. Removing either `owns_generation` guard makes this test fail.
#[test]
fn stale_layout_generation_cannot_publish_or_clear_replacement_ownership() {
    let _guard = lock_backend_event_tests();
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "generation-race proof begins without leaked layout workers"
    );
    let health_live = TestBackend::start(BackendMode::Live);
    let mut harness: Harness<HandshakeApp> = Harness::builder().build_eframe(|_| {
        HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_owned(),
            db_status: "ok".to_owned(),
            migration_version: Some(1),
        }))
    });

    let mut old_snapshot = harness.state().capture_layout_snapshot();
    old_snapshot.split_weights = SplitWeights {
        vertical: 0.27,
        horizontal: 0.33,
    };
    let old_layout =
        TestBackend::start_with_layout_body(layout_response_body(old_snapshot.to_layout_state()));
    harness
        .state()
        .pause_next_layout_load_before_publication_for_test();
    harness
        .state_mut()
        .set_backend_endpoints_for_test(&health_live.base_url(), &old_layout.base_url());
    step_until_phase_with_details(
        &mut harness,
        "old layout worker accepted and paused before publication",
        CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3),
        |app| {
            old_layout.layout_request_count() >= 1
                && app.paused_layout_load_generation_for_test().is_some()
        },
        |app| {
            format!(
                "old-layout accepted={}, layout={}, unclassified={}; ownership={:?}; paused={:?}; workers={}",
                old_layout.accepted_count(),
                old_layout.layout_request_count(),
                old_layout.unclassified_request_count(),
                app.layout_load_ownership_for_test(),
                app.paused_layout_load_generation_for_test(),
                app.layout_workers_in_flight_for_test(),
            )
        },
    );
    let old_generation = harness
        .state()
        .paused_layout_load_generation_for_test()
        .expect("old client worker is paused before publication");
    assert_eq!(
        harness.state().layout_load_ownership_for_test().0,
        old_generation,
        "the paused worker initially owns the current load generation"
    );
    assert!(
        harness.state().layout_load_ownership_for_test().2,
        "the paused old generation still owns the shared in-flight marker"
    );

    let mut replacement_snapshot = harness.state().capture_layout_snapshot();
    replacement_snapshot.split_weights = SplitWeights {
        vertical: 0.71,
        horizontal: 0.66,
    };
    let replacement_weights = replacement_snapshot.split_weights;
    let replacement_layout = TestBackend::start_with_layout_body(layout_response_body(
        replacement_snapshot.to_layout_state(),
    ));
    harness
        .state_mut()
        .set_backend_endpoints_for_test(&health_live.base_url(), &replacement_layout.base_url());
    step_until_phase(
        &mut harness,
        "replacement layout worker publishes while old generation remains paused",
        CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3),
        |app| {
            let (current, cleared, in_flight) = app.layout_load_ownership_for_test();
            replacement_layout.layout_request_count() >= 1
                && app.split_weights() == replacement_weights
                && current > old_generation
                && cleared == current
                && !in_flight
                && app.paused_layout_load_generation_for_test() == Some(old_generation)
        },
    );
    let replacement_ownership = harness.state().layout_load_ownership_for_test();
    assert_eq!(
        harness.state().paused_layout_load_generation_for_test(),
        Some(old_generation),
        "the exact old client worker remains active after replacement publication"
    );
    assert_eq!(
        harness.state().layout_workers_in_flight_for_test(),
        1,
        "replacement settled while only the deliberately paused old worker remains"
    );
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        1,
        "process-wide worker liveness agrees that the paused old generation is still active"
    );

    let old_release_started = Instant::now();
    harness
        .state()
        .release_paused_layout_load_publication_for_test();
    step_until(&mut harness, Duration::from_secs(4), |app| {
        app.paused_layout_load_generation_for_test().is_none()
            && app.layout_workers_in_flight_for_test() == 0
    });
    harness.run_steps(3);
    assert!(
        old_release_started.elapsed() < Duration::from_secs(4),
        "both real layout workers settle promptly after the old publication pause is released"
    );
    assert_eq!(
        harness.state().split_weights(),
        replacement_weights,
        "late old snapshot must not overwrite the already-applied replacement snapshot"
    );
    assert_eq!(
        harness.state().layout_load_ownership_for_test(),
        replacement_ownership,
        "late old owner must not clear or republish after the replacement generation settles"
    );
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "both generation workers are reclaimed"
    );
    assert!(health_live.health_request_count() >= 1);
    assert_eq!(health_live.layout_request_count(), 0);
    assert_eq!(old_layout.health_request_count(), 0);
    assert!(old_layout.layout_request_count() >= 1);
    assert_eq!(
        old_layout.accepted_count(),
        old_layout.layout_request_count()
    );
    assert_eq!(replacement_layout.health_request_count(), 0);
    assert!(replacement_layout.layout_request_count() >= 1);
    assert_eq!(
        replacement_layout.accepted_count(),
        replacement_layout.layout_request_count()
    );
    assert!(
        !harness.state().backend_is_down(),
        "layout generation outcomes never compete with the live canonical health route"
    );
    assert_eq!(health_live.unclassified_request_count(), 0);
    assert_eq!(old_layout.unclassified_request_count(), 0);
    assert_eq!(replacement_layout.unclassified_request_count(), 0);

    drop(harness);
    health_live.stop();
    old_layout.stop();
    replacement_layout.stop();
    assert_no_local_artifact_dir();
}

/// App teardown owns every layout generation, not only the newest one. Hold two real accepted layout
/// sockets across an endpoint rebind and drop the mounted app while both workers are active; production
/// request bounds must reclaim both without releasing either server to manufacture completion.
#[test]
fn app_drop_reclaims_two_active_layout_generations_within_bound() {
    let _guard = lock_backend_event_tests();
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "two-generation Drop proof begins without leaked layout workers"
    );
    let health_live = TestBackend::start(BackendMode::Live);
    let mut harness: Harness<HandshakeApp> = Harness::builder().build_eframe(|_| {
        HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
            status: "ok".to_owned(),
            db_status: "ok".to_owned(),
            migration_version: Some(1),
        }))
    });

    let mut first_snapshot = harness.state().capture_layout_snapshot();
    first_snapshot.split_weights = SplitWeights {
        vertical: 0.31,
        horizontal: 0.41,
    };
    let first_layout = TestBackend::start_controlled_layout(layout_response_body(
        first_snapshot.to_layout_state(),
    ));
    harness
        .state_mut()
        .set_backend_endpoints_for_test(&health_live.base_url(), &first_layout.base_url());
    step_until_phase(
        &mut harness,
        "first controlled layout worker accepted and held",
        CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3),
        |app| {
            first_layout.layout_request_count() >= 1
                && first_layout.held_count() >= 1
                && app.layout_workers_in_flight_for_test() >= 1
        },
    );

    let mut second_snapshot = harness.state().capture_layout_snapshot();
    second_snapshot.split_weights = SplitWeights {
        vertical: 0.63,
        horizontal: 0.73,
    };
    let second_layout = TestBackend::start_controlled_layout(layout_response_body(
        second_snapshot.to_layout_state(),
    ));
    harness
        .state_mut()
        .set_backend_endpoints_for_test(&health_live.base_url(), &second_layout.base_url());
    step_until_phase(
        &mut harness,
        "second controlled layout worker overlaps the first",
        CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3),
        |app| {
            second_layout.layout_request_count() >= 1
                && second_layout.held_count() >= 1
                && app.layout_workers_in_flight_for_test() >= 2
                && app.layout_load_ownership_for_test().2
        },
    );
    assert_eq!(health_live.layout_request_count(), 0);
    assert!(health_live.health_request_count() >= 1);
    assert_eq!(first_layout.health_request_count(), 0);
    assert_eq!(second_layout.health_request_count(), 0);
    assert_eq!(health_live.unclassified_request_count(), 0);
    assert_eq!(first_layout.unclassified_request_count(), 0);
    assert_eq!(second_layout.unclassified_request_count(), 0);

    let drop_started = Instant::now();
    drop(harness);
    let drop_elapsed = drop_started.elapsed();
    assert!(
        first_layout.is_running()
            && first_layout.held_count() >= 1
            && second_layout.is_running()
            && second_layout.held_count() >= 1,
        "both controlled peers remain silent and held throughout app teardown"
    );
    assert!(
        drop_elapsed <= CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3),
        "Drop must reclaim two overlapping layout generations within one request bound (elapsed={drop_elapsed:?})"
    );
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "Drop reclaims both active controlled generations"
    );
    assert!(first_layout.layout_request_count() >= 1);
    assert!(second_layout.layout_request_count() >= 1);
    assert_eq!(
        first_layout.accepted_count(),
        first_layout.layout_request_count()
    );
    assert_eq!(
        second_layout.accepted_count(),
        second_layout.layout_request_count()
    );

    health_live.stop();
    first_layout.stop();
    second_layout.stop();
    assert_no_local_artifact_dir();
}

/// Dropping a mounted app with a silent request in flight must reclaim and join every app-owned layout
/// worker within the same network bound. The peer remains silent and held throughout teardown, so only
/// production timeout/cancellation/ownership can make the worker finish.
#[test]
fn app_drop_reclaims_layout_workers_within_bound() {
    let _guard = lock_backend_event_tests();
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "test begins without leaked layout workers"
    );
    let server = TestBackend::start(BackendMode::Silent);
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    harness
        .state_mut()
        .set_backend_unreachable_for_test(&server.base_url());
    step_until(&mut harness, Duration::from_secs(3), |app| {
        app.layout_workers_in_flight_for_test() > 0 && server.held_count() >= 1
    });
    assert!(
        server.held_count() >= 1,
        "silent peer holds a real app layout request before teardown"
    );

    let drop_started = Instant::now();
    drop(harness);
    let drop_elapsed = drop_started.elapsed();
    assert!(
        server.is_running() && server.held_count() >= 1,
        "peer was not released to help app teardown complete"
    );
    assert!(drop_elapsed <= CLIENT_REQUEST_TIMEOUT + Duration::from_secs(3), "app drop must be bounded by configured backend timeout, not an unbounded join (elapsed={drop_elapsed:?})");
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "all app-owned layout workers are joined/reclaimed before drop returns"
    );
    server.stop();
    assert_no_local_artifact_dir();
}

/// Exercise the deadline-expired branch directly. A deliberately blocked worker cannot be joined within
/// the tiny deadline, so settlement must return promptly, detach its handle, suppress late actionable
/// state through the cooperative shutdown edge, and still let the owned lifecycle counter reap to zero.
#[test]
fn expired_shutdown_deadline_detaches_without_late_action_or_counter_leak() {
    let _guard = lock_backend_event_tests();
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "deadline-branch proof begins without leaked layout workers"
    );
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    let release = Arc::new(AtomicBool::new(false));
    let late_action = Arc::new(AtomicBool::new(false));
    app.spawn_blocked_layout_worker_for_test(Arc::clone(&release), Arc::clone(&late_action));

    let started = Instant::now();
    let detached = app.shutdown_layout_workers_with_timeout_for_test(Duration::from_millis(20));
    let elapsed = started.elapsed();
    assert_eq!(
        detached, 1,
        "the deliberately unfinished worker takes the detach branch"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "deadline-expired shutdown remains bounded instead of joining the unfinished worker: {elapsed:?}"
    );
    assert_eq!(
        app.layout_workers_in_flight_for_test(),
        0,
        "the app no longer owns an unfinished handle after bounded settlement"
    );

    release.store(true, Ordering::SeqCst);
    let reap_deadline = Instant::now() + Duration::from_secs(2);
    while HandshakeApp::global_layout_workers_in_flight_for_test() != 0
        && Instant::now() < reap_deadline
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "the detached worker retains and releases its owned lifecycle guard"
    );
    assert!(
        !late_action.load(Ordering::SeqCst),
        "a worker completing after shutdown must not publish actionable state"
    );
    drop(app);
    assert_no_local_artifact_dir();
}

/// Adversarially force the former check/publication race: the worker owns publication eligibility while
/// a concurrent shutdown edge tries to begin. Shutdown must not linearize or detach the worker between
/// its eligibility check and publication; after publication completes, shutdown may advance and bounded
/// settlement may detach the deliberately still-blocked tail. No action can then appear after shutdown.
#[test]
fn shutdown_edge_cannot_split_layout_publication_or_deliver_after_detach() {
    let _guard = lock_backend_event_tests();
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "publication-race proof begins without leaked layout workers"
    );
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    let gate_owned = Arc::new(AtomicBool::new(false));
    let allow_publication = Arc::new(AtomicBool::new(false));
    let release_worker = Arc::new(AtomicBool::new(false));
    let publication = Arc::new(AtomicBool::new(false));
    let post_detach_publication = Arc::new(AtomicBool::new(false));
    let edge_started = Arc::new(AtomicBool::new(false));
    let edge_set = Arc::new(AtomicBool::new(false));
    app.spawn_paused_layout_publication_for_test(
        Arc::clone(&gate_owned),
        Arc::clone(&allow_publication),
        Arc::clone(&release_worker),
        Arc::clone(&publication),
        Arc::clone(&post_detach_publication),
    );

    let gate_deadline = Instant::now() + Duration::from_secs(2);
    while !gate_owned.load(Ordering::SeqCst) && Instant::now() < gate_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        gate_owned.load(Ordering::SeqCst),
        "worker pauses while owning the publication eligibility gate"
    );

    let shutdown_edge =
        app.spawn_layout_shutdown_edge_for_test(Arc::clone(&edge_started), Arc::clone(&edge_set));
    let edge_deadline = Instant::now() + Duration::from_secs(2);
    while !edge_started.load(Ordering::SeqCst) && Instant::now() < edge_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        edge_started.load(Ordering::SeqCst),
        "shutdown edge is actively attempting to acquire the held publication gate"
    );
    assert!(
        !edge_set.load(Ordering::SeqCst),
        "shutdown cannot pass the publication gate and detach between eligibility and publication"
    );
    assert!(
        !publication.load(Ordering::SeqCst),
        "the worker remains paused before publication"
    );

    allow_publication.store(true, Ordering::SeqCst);
    shutdown_edge.join().expect("shutdown edge thread joins");
    assert!(
        publication.load(Ordering::SeqCst),
        "eligible publication linearizes before the waiting shutdown edge"
    );
    assert!(
        edge_set.load(Ordering::SeqCst),
        "shutdown edge advances immediately after publication releases the gate"
    );

    let detached = app.shutdown_layout_workers_with_timeout_for_test(Duration::from_millis(20));
    assert_eq!(
        detached, 1,
        "the deliberately blocked post-publication tail takes bounded detachment"
    );
    let publication_at_detach = publication.load(Ordering::SeqCst);
    release_worker.store(true, Ordering::SeqCst);
    let reap_deadline = Instant::now() + Duration::from_secs(2);
    while HandshakeApp::global_layout_workers_in_flight_for_test() != 0
        && Instant::now() < reap_deadline
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "detached post-publication worker tail releases its lifecycle guard"
    );
    assert_eq!(
        publication.load(Ordering::SeqCst),
        publication_at_detach,
        "detached completion cannot produce a later publication"
    );
    assert!(
        !post_detach_publication.load(Ordering::SeqCst),
        "the detached worker's explicit second publication attempt is rejected after shutdown"
    );
    drop(app);
    assert_no_local_artifact_dir();
}

/// Drive the exact shared save-completion primitive used by the named production worker path. The source
/// audit above proves both worker owners call this primitive; this adversarial runtime seam forces its
/// former eligibility-check/shutdown/repaint race and its post-detach rejection boundary.
fn assert_layout_save_completion_shutdown_race(worker_path: &str) {
    let _guard = lock_backend_event_tests();
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "{worker_path} save-completion race proof begins without leaked layout workers"
    );
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    let gate_owned = Arc::new(AtomicBool::new(false));
    let allow_publication = Arc::new(AtomicBool::new(false));
    let release_worker = Arc::new(AtomicBool::new(false));
    let publication = Arc::new(AtomicBool::new(false));
    let post_detach_publication = Arc::new(AtomicBool::new(false));
    let edge_started = Arc::new(AtomicBool::new(false));
    let edge_set = Arc::new(AtomicBool::new(false));
    app.spawn_paused_layout_save_completion_for_test(
        Arc::clone(&gate_owned),
        Arc::clone(&allow_publication),
        Arc::clone(&release_worker),
        Arc::clone(&publication),
        Arc::clone(&post_detach_publication),
    );

    let gate_deadline = Instant::now() + Duration::from_secs(2);
    while !gate_owned.load(Ordering::SeqCst) && Instant::now() < gate_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        gate_owned.load(Ordering::SeqCst),
        "{worker_path} save completion pauses while owning the publication gate"
    );

    let shutdown_edge =
        app.spawn_layout_shutdown_edge_for_test(Arc::clone(&edge_started), Arc::clone(&edge_set));
    let edge_deadline = Instant::now() + Duration::from_secs(2);
    while !edge_started.load(Ordering::SeqCst) && Instant::now() < edge_deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        edge_started.load(Ordering::SeqCst),
        "shutdown actively contends with {worker_path} save completion"
    );
    assert!(
        !edge_set.load(Ordering::SeqCst),
        "shutdown cannot split {worker_path} save completion eligibility from its wake publication"
    );
    assert!(
        !publication.load(Ordering::SeqCst),
        "{worker_path} save completion remains paused before publication"
    );

    allow_publication.store(true, Ordering::SeqCst);
    shutdown_edge.join().expect("shutdown edge thread joins");
    assert!(
        publication.load(Ordering::SeqCst),
        "{worker_path} eligible completion linearizes wholly before shutdown"
    );
    assert!(
        edge_set.load(Ordering::SeqCst),
        "shutdown advances after {worker_path} completion releases the gate"
    );

    let detached = app.shutdown_layout_workers_with_timeout_for_test(Duration::from_millis(20));
    assert_eq!(
        detached, 1,
        "the deliberately blocked {worker_path} post-completion tail is detached at the bound"
    );
    release_worker.store(true, Ordering::SeqCst);
    let reap_deadline = Instant::now() + Duration::from_secs(2);
    while HandshakeApp::global_layout_workers_in_flight_for_test() != 0
        && Instant::now() < reap_deadline
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        HandshakeApp::global_layout_workers_in_flight_for_test(),
        0,
        "detached {worker_path} save worker releases its lifecycle guard"
    );
    assert!(
        !post_detach_publication.load(Ordering::SeqCst),
        "{worker_path} cannot publish a second completion after shutdown/detachment"
    );
    drop(app);
    assert_no_local_artifact_dir();
}

#[test]
fn shutdown_edge_cannot_split_immediate_layout_save_completion() {
    assert_layout_save_completion_shutdown_race("immediate");
}

#[test]
fn shutdown_edge_cannot_split_debounced_layout_save_completion() {
    assert_layout_save_completion_shutdown_race("debounced");
}

/// Fixed backend safety deadlines are code policy, not operator preferences. The UserManual must give
/// no-context operators/models the exact runtime bounds and must not promise a nonexistent setting.
#[test]
fn backend_timeout_manual_matches_fixed_runtime_policy() {
    let manual = include_str!("../src/manual_content_editors.rs");
    assert_eq!(CLIENT_CONNECT_TIMEOUT, Duration::from_millis(1500));
    assert_eq!(CLIENT_REQUEST_TIMEOUT, Duration::from_secs(10));
    for required in [
        "bounds connection setup at 1.5 seconds",
        "a silent accepted request at 10 seconds",
        "fixed safety bounds, not operator preferences",
        "this WP adds no timeout setting",
    ] {
        assert!(manual.contains(required), "UserManual must remain code-truthful about fixed backend deadlines: missing {required:?}");
    }
    assert_no_local_artifact_dir();
}

// ── AC-008-1 corroboration: idle keep-alive keeps the heartbeat (responsiveness oracle) advancing ──

/// Corroborate that the responsiveness oracle (the heartbeat) keeps advancing even on an IDLE down
/// backend: the MT-084 idle repaint cadence (proven elsewhere) is within the window that keeps the frame
/// loop — and therefore the heartbeat — ticking, so a backend-down idle app is never mistaken for frozen.
/// (This just asserts the cadence constant is in the healthy window; the live advance is AC-008-1.)
#[test]
fn idle_keepalive_keeps_responsiveness_oracle_live() {
    assert!(
        HEARTBEAT_IDLE_REPAINT_INTERVAL <= Duration::from_millis(500),
        "the idle repaint cadence ({HEARTBEAT_IDLE_REPAINT_INTERVAL:?}) keeps the frame loop (and the \
         heartbeat responsiveness oracle) ticking so a backend-down idle app is not misread as frozen"
    );
    assert!(
        HEARTBEAT_IDLE_REPAINT_INTERVAL < Duration::from_secs(5),
        "the idle cadence is far below the ~5s freeze threshold"
    );
    assert_no_local_artifact_dir();
}

// ── source-scan helpers (code-only) ────────────────────────────────────────────────────────────────

fn read_frontend_rust_sources() -> std::collections::BTreeMap<String, String> {
    fn visit(
        root: &Path,
        directory: &Path,
        sources: &mut std::collections::BTreeMap<String, String>,
    ) {
        let mut entries: Vec<_> = std::fs::read_dir(directory)
            .unwrap_or_else(|error| {
                panic!(
                    "read frontend source directory {}: {error}",
                    directory.display()
                )
            })
            .map(|entry| entry.expect("read frontend source entry"))
            .collect();
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, sources);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("frontend source is below source root")
                    .to_string_lossy()
                    .replace('\\', "/");
                let source = std::fs::read_to_string(&path).unwrap_or_else(|error| {
                    panic!("read frontend source {}: {error}", path.display())
                });
                sources.insert(relative, source);
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = std::collections::BTreeMap::new();
    visit(&root, &root, &mut sources);
    sources
}

fn step_until(
    harness: &mut Harness<'_, HandshakeApp>,
    timeout: Duration,
    predicate: impl Fn(&HandshakeApp) -> bool,
) {
    step_until_phase(harness, "condition", timeout, predicate);
}

fn step_until_phase(
    harness: &mut Harness<'_, HandshakeApp>,
    phase: &str,
    timeout: Duration,
    predicate: impl Fn(&HandshakeApp) -> bool,
) {
    step_until_phase_with_details(harness, phase, timeout, predicate, |_| String::new());
}

fn step_until_phase_with_details(
    harness: &mut Harness<'_, HandshakeApp>,
    phase: &str,
    timeout: Duration,
    predicate: impl Fn(&HandshakeApp) -> bool,
    details: impl Fn(&HandshakeApp) -> String,
) {
    let deadline = Instant::now() + timeout;
    let mut slowest = Duration::ZERO;
    while !predicate(harness.state()) && Instant::now() < deadline {
        let started = Instant::now();
        harness.step();
        slowest = slowest.max(started.elapsed());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(predicate(harness.state()), "mounted app condition for {phase} was not reached within {timeout:?}; slowest frame={slowest:?}, status={:?}; {}", harness.state().status_bar_health_text(), details(harness.state()));
    assert!(
        slowest < Duration::from_millis(500),
        "waiting for backend state during {phase} must not freeze a frame; slowest={slowest:?}"
    );
}

fn set_mounted_code_value(harness: &mut Harness<'_, HandshakeApp>, value: &str) {
    harness.step();
    let node_id = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(CODE_EDITOR_TEXT_AUTHOR_ID))
        .expect("mounted code editor exposes editor.code.text while backend is unavailable")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::SetValue,
            target: node_id,
            data: Some(egui::accesskit::ActionData::Value(value.to_owned().into())),
        },
    ));
    harness.step();
}

fn reqwest_owning_client_names(src: &str) -> std::collections::BTreeSet<String> {
    let mut result = std::collections::BTreeSet::new();
    let lines: Vec<_> = src.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let Some(rest) = line.trim().strip_prefix("pub struct ") else {
            continue;
        };
        let Some(name) = rest.split_whitespace().next() else {
            continue;
        };
        if !name.ends_with("Client") {
            continue;
        }
        let owns_client = lines[index..lines.len().min(index + 12)]
            .iter()
            .any(|candidate| candidate.contains("client: reqwest::Client"));
        if owns_client {
            result.insert(name.to_owned());
        }
    }
    result
}

fn extract_client_constructor<'a>(src: &'a str, client: &str) -> Option<&'a str> {
    let implementation = extract_braced_region(src, &format!("impl {client} {{"))?;
    extract_fn_body(implementation, "pub fn new(")
}

fn extract_braced_region<'a>(src: &'a str, prefix: &str) -> Option<&'a str> {
    let start = src.find(prefix)?;
    let open = start + src[start..].find('{')?;
    let bytes = src.as_bytes();
    let mut depth = 0i32;
    for index in open..bytes.len() {
        match bytes[index] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[open..=index]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Strip `//` line comments so a source-review scan checks CODE, not explanatory prose that may
/// legitimately mention `block_on` / `load_layout` etc. Conservative: removes from the first `//` not
/// inside a string literal to end-of-line.
fn strip_line_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let mut in_str = false;
        let mut prev = '\0';
        let bytes: Vec<char> = line.chars().collect();
        let mut cut = bytes.len();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            if c == '"' && prev != '\\' {
                in_str = !in_str;
            }
            if !in_str && c == '/' && i + 1 < bytes.len() && bytes[i + 1] == '/' {
                cut = i;
                break;
            }
            prev = c;
            i += 1;
        }
        out.extend(bytes[..cut].iter());
        out.push('\n');
    }
    out
}

/// Extract the brace-balanced body text of the first `fn` whose signature starts with `sig_prefix`.
fn extract_fn_body<'a>(src: &'a str, sig_prefix: &str) -> Option<&'a str> {
    let start = src.find(sig_prefix)?;
    let open_rel = src[start..].find('{')?;
    let open = start + open_rel;
    let bytes = src.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[open..=i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
