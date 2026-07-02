//! WP-KERNEL-012 MT-096 — the REOPENED REAL-HOST GATES (§6.13.5 + §6.13.6 proven on the REAL spawned
//! `palmistry.exe` against genuinely frozen / genuinely crashed HandshakeApp-SHAPED victim processes).
//!
//! The default MT-096 halves (`test_end_to_end_support.rs` here + `test_three_tier_end_to_end.rs` on the
//! handshake-native side) prove the ring-contract integration with library types and injected clocks.
//! What they deliberately do NOT prove — and what THIS file exists for — is the real spawned watcher
//! observing a real incident end-to-end:
//!
//! - **GATE 1 (FREEZE, AC-016-1)** [`live_palmistry_confirms_freeze_of_frozen_handshake_shaped_child`]:
//!   a re-exec'd VICTIM child process behaves exactly like a healthy HandshakeApp — it creates a REAL
//!   MT-081 ring, publishes advancing heartbeats at the MT-084 idle cadence, and hosts a REAL visible
//!   top-level Win32 window whose message pump it services — then it FREEZES for real: heartbeats stop
//!   AND the pump blocks on the same (UI) thread. The REAL spawned `palmistry.exe` (production
//!   `run_watcher`: MT-090 zero-coop reader + MT-091 `Win32HungWindowProbe` + the §6.13.5 double-signal
//!   gate) must CONFIRM the freeze and persist the durable freeze SURVIVOR RECORD (MT-093), which the
//!   test reads back through the production `SurvivorStore`. A clean Shutdown then reaps the watcher
//!   with NO crash record (the victim is frozen, not dead).
//! - **GATE 2 (CRASH, AC-016-2)** [`live_palmistry_captures_minidump_from_really_crashed_client`]: a
//!   re-exec'd VICTIM child connects the REAL `minidumper::Client` to the watcher's DERIVED crash socket
//!   (the §6.13.6 rendezvous), installs the REAL `crash-handler` OS-exception handler (the exact
//!   production client shape), and then REALLY CRASHES — an actual unhandled access violation, not
//!   `simulate_exception`; the victim process DIES with `STATUS_ACCESS_VIOLATION`. The REAL spawned
//!   `palmistry.exe`'s SHIPPED `CrashServerHandler` must write the minidump OUT-OF-PROCESS, and its
//!   lifecycle must classify the abnormal parent death + persist the RICH `CrashContextMinidump` crash
//!   record naming the dump. The test VALIDATES the dump with the `minidump` reader (thread list, module
//!   list, AND the exception stream carrying the access violation) — MT-092's reader-validation machinery
//!   reused against a genuinely dead client.
//!
//! Each gate writes a typed VERDICT artifact (`freeze_live_capture_verdict.json` /
//! `crash_live_capture_verdict.json`) into the external MT-096 `live/` artifact leaf — the
//! handshake-native capstone manifest (scenario5) DERIVES its AC-016-1/AC-016-2 verdicts from these
//! artifacts instead of hardcoding strings; a verdict file exists only if every assertion in the gate
//! held on this host.
//!
//! BOUNDED-TEST RULE (packet `palmistry_test_bound_policy`): both gates spawn children + do minidumper
//! IPC, so both are `#[ignore]`d with a typed reason and every wait is HARD-BOUNDED — child stdout reads
//! go through a dedicated reader thread + `recv_timeout` (never a blocking pipe read on the test
//! thread), child reaps are bounded `try_wait` loops, control replies use a bounded non-blocking read,
//! and record polls have explicit deadlines. Run with `--ignored` on a real interactive host.

#![cfg(windows)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use handshake_diag_ring::{DiagRingWriter, DEFAULT_CAPACITY};
use palmistry::survivor_store::{SurvivorProbeResult, SurvivorRecordKind, SurvivorStore};

// ── external artifact root (CX-212E) — same 4-up convention as test_end_to_end_support.rs ──────────────

/// The MT-096 live-gate artifact leaf under the disk-agnostic external root: the `palmistry` crate sits
/// at `<repo>/src/frontend/palmistry`, so four `..` reach `<repo>/..` where `Handshake_Artifacts` is a
/// sibling of the repo worktree. The ring, crash record, minidump, survivor records, and the typed
/// verdict files all land HERE (durable proof; never repo-local, never deleted on success).
fn live_artifact_dir() -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test")
        .join("wp-kernel-012-mt-096")
        .join("live")
}

/// CX-212E hygiene guard: NO repo-local artifact dir may exist under the crate.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

fn unique_tag(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{label}-{}-{nanos}", std::process::id())
}

fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn abs_display(p: &Path) -> String {
    std::fs::canonicalize(p)
        .unwrap_or_else(|_| p.to_path_buf())
        .display()
        .to_string()
}

// ── bounded child harness (the test_crash_capture.rs / MT-092 conventions) ─────────────────────────────

fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<i32> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.code().unwrap_or(-1)),
            Ok(None) => {
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return None,
        }
    }
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = wait_for_exit(child, Duration::from_secs(2));
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child }
    }
    fn id(&self) -> u32 {
        self.child.id()
    }
    fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        kill_child(&mut self.child);
    }
}

fn stderr_after_exit(child: &mut Child) -> String {
    if !matches!(child.try_wait(), Ok(Some(_))) {
        return "<stderr unavailable: child still running after bounded kill/reap>".to_owned();
    }
    let mut stderr = String::new();
    if let Some(mut se) = child.stderr.take() {
        let _ = se.read_to_string(&mut stderr);
    }
    stderr
}

/// Spawn a DEDICATED reader thread draining `reader` line-by-line into an mpsc channel — the HARD
/// external bound the packet `palmistry_test_bound_policy` mandates for child stdout reads (the blocking
/// `read_line` never runs on the test thread; the test waits only via `recv_timeout`).
fn spawn_bounded_line_reader<R: Read + Send + 'static>(reader: R) -> mpsc::Receiver<String> {
    use std::io::{BufRead, BufReader};
    let (tx, rx) = mpsc::channel::<String>();
    let _ = std::thread::Builder::new()
        .name("mt096-bounded-child-stdout-reader".to_string())
        .spawn(move || {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if tx.send(line.trim().to_string()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
    rx
}

/// Wait (HARD-BOUNDED via `recv_timeout`) for a relayed line equal to `marker`. `false` on timeout or
/// channel disconnect (the child exited / closed its pipe before printing the marker).
fn wait_marker_bounded(rx: &mpsc::Receiver<String>, marker: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        match rx.recv_timeout(deadline - now) {
            Ok(line) if line == marker => return true,
            Ok(_) => continue,
            Err(_) => return false,
        }
    }
}

/// Spawn the REAL palmistry watcher binary against a victim pid, with the survivor store scoped to
/// `survivor_dir` (the production `HANDSHAKE_PALMISTRY_SURVIVOR_DIR` override, inherited by the spawn).
fn spawn_palmistry_watcher(
    parent_pid: u32,
    session_id: &str,
    ring_path: &Path,
    control_socket: &str,
    survivor_dir: &Path,
) -> Child {
    Command::new(env!("CARGO_BIN_EXE_palmistry"))
        .env("HANDSHAKE_PARENT_PID", parent_pid.to_string())
        .env("HANDSHAKE_SESSION_ID", session_id)
        .env("HANDSHAKE_RING_PATH", ring_path)
        .env("HANDSHAKE_CONTROL_SOCK", control_socket)
        .env(
            palmistry::survivor_store::ENV_PALMISTRY_SURVIVOR_DIR,
            survivor_dir,
        )
        .env("PALMISTRY_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the REAL palmistry watcher binary")
}

/// Bounded, non-blocking read of one newline-delimited control reply (mirrors test_crash_capture.rs).
fn read_control_line_bounded(
    stream: &mut interprocess::local_socket::Stream,
    timeout: Duration,
) -> std::io::Result<String> {
    use interprocess::local_socket::traits::Stream as _;

    fn read_loop(
        stream: &mut interprocess::local_socket::Stream,
        timeout: Duration,
    ) -> std::io::Result<String> {
        let start = Instant::now();
        let mut bytes = Vec::with_capacity(64);
        let mut buf = [0u8; 128];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "control socket closed before newline reply",
                    ));
                }
                Ok(n) => {
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(pos) = bytes.iter().position(|b| *b == b'\n') {
                        bytes.truncate(pos + 1);
                        return String::from_utf8(bytes).map_err(|err| {
                            std::io::Error::new(std::io::ErrorKind::InvalidData, err)
                        });
                    }
                    if bytes.len() > 16 * 1024 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "control reply exceeded 16KiB before newline",
                        ));
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() >= timeout {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("no control reply within {timeout:?}"),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(err) => return Err(err),
            }
        }
    }

    stream.set_nonblocking(true)?;
    let result = read_loop(stream, timeout);
    let reset = stream.set_nonblocking(false);
    match (result, reset) {
        (Ok(line), Ok(())) => Ok(line),
        (Ok(_), Err(err)) => Err(err),
        (Err(err), _) => Err(err),
    }
}

/// Send one newline-delimited JSON control message to the watcher (bounded connect retry + bounded read).
fn send_control(
    socket: &str,
    message_json: &str,
    connect_timeout: Duration,
) -> std::io::Result<String> {
    use interprocess::local_socket::traits::Stream as _;
    use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};

    let name = socket.to_ns_name::<GenericNamespaced>()?;
    let deadline = Instant::now() + connect_timeout;
    let mut last_err = None;
    while Instant::now() < deadline {
        let mut conn = match Stream::connect(name.clone()) {
            Ok(conn) => conn,
            Err(err) => {
                last_err = Some(err);
                std::thread::sleep(Duration::from_millis(25));
                continue;
            }
        };
        if let Err(err) = conn
            .write_all(message_json.as_bytes())
            .and_then(|()| conn.write_all(b"\n"))
            .and_then(|()| conn.flush())
        {
            last_err = Some(err);
            std::thread::sleep(Duration::from_millis(25));
            continue;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        return read_control_line_bounded(&mut conn, remaining)
            .map(|line| line.trim_end().to_string());
    }
    Err(last_err.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("no control reply within {connect_timeout:?}"),
        )
    }))
}

// ── the FREEZE VICTIM (a HandshakeApp-SHAPED child: real ring heartbeats + a real pumped window) ───────

/// Env gate turning the re-exec'd child into the FREEZE VICTIM (never set in a normal run).
const FREEZE_VICTIM_ENV: &str = "HSK_MT096_FREEZE_VICTIM";
/// The ring path the victim creates + publishes heartbeats into.
const FREEZE_VICTIM_RING_ENV: &str = "HSK_MT096_FREEZE_RING";
/// How long the victim stays HEALTHY (pumping + heartbeating), in ms.
const FREEZE_VICTIM_HEALTHY_MS_ENV: &str = "HSK_MT096_FREEZE_HEALTHY_MS";
/// How long the victim stays FROZEN (no heartbeats, blocked pump), in ms.
const FREEZE_VICTIM_FROZEN_MS_ENV: &str = "HSK_MT096_FREEZE_FROZEN_MS";

/// The FREEZE-VICTIM host entry (the `test_freeze_detect.rs` re-exec pattern). In a normal run (env
/// unset) it is a no-op. Re-exec'd with [`FREEZE_VICTIM_ENV`] set, its body is a faithful
/// HandshakeApp-shaped process: it CREATES the REAL MT-081 ring at [`FREEZE_VICTIM_RING_ENV`], creates a
/// REAL visible top-level window (offscreen, WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW — quiet-operation
/// duty), prints `MT096_VICTIM_READY`, then for the healthy window publishes ADVANCING heartbeats at the
/// MT-084 idle cadence while SERVICING its message pump (a WM_NULL probe answers). Then it GENUINELY
/// FREEZES: prints `MT096_VICTIM_FROZEN` and sleeps on the SAME thread — heartbeats stop advancing AND
/// the pump stops being serviced, exactly the §6.13.5 double signal. Finally it destroys the window and
/// exits 0 (the parent test normally shuts the watcher down and kills the victim before that).
#[test]
fn mt096_freeze_victim_entry() {
    if std::env::var(FREEZE_VICTIM_ENV).ok().as_deref() != Some("1") {
        return; // Not re-exec'd as the victim: a no-op in a normal test run.
    }
    let ring_path = PathBuf::from(
        std::env::var_os(FREEZE_VICTIM_RING_ENV).expect("freeze victim needs the ring path env"),
    );
    let healthy_ms: u64 = std::env::var(FREEZE_VICTIM_HEALTHY_MS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6_000);
    let frozen_ms: u64 = std::env::var(FREEZE_VICTIM_FROZEN_MS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(45_000);

    // The REAL Tier-2 write side: the SAME `DiagRingWriter` Handshake uses (no reimplementation).
    let writer =
        DiagRingWriter::create(&ring_path, DEFAULT_CAPACITY).expect("victim creates the real ring");
    writer.write_heartbeat(1, now_nanos());

    let hwnd = create_victim_window().expect("victim creates a real top-level window");
    println!("MT096_VICTIM_READY");
    let _ = std::io::stdout().flush();

    // HEALTHY PHASE — a live HandshakeApp shape: advancing heartbeats at the ~250ms MT-084 idle cadence
    // + a serviced message pump on this same thread (the probe answers Responding).
    let healthy_deadline = Instant::now() + Duration::from_millis(healthy_ms);
    let mut counter: u64 = 1;
    let mut next_heartbeat = Instant::now();
    while Instant::now() < healthy_deadline {
        pump_messages_once();
        if Instant::now() >= next_heartbeat {
            counter += 1;
            writer.write_heartbeat(counter, now_nanos());
            next_heartbeat += Duration::from_millis(250);
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    println!("MT096_VICTIM_FROZEN");
    let _ = std::io::stdout().flush();

    // FROZEN PHASE — the GENUINE freeze: this (UI) thread sleeps, so the heartbeat counter stops
    // advancing in the ring AND the window's message pump stops being serviced. Palmistry's zero-coop
    // reader sees the stale heartbeat; its REAL Win32 probe times out on WM_NULL — the double signal.
    std::thread::sleep(Duration::from_millis(frozen_ms));

    destroy_victim_window(hwnd);
    println!("MT096_VICTIM_DONE");
    let _ = std::io::stdout().flush();
    drop(writer);
}

/// Register + create the victim's REAL top-level window (offscreen, never activates, no taskbar button —
/// quiet-operation duty; `SW_SHOWNOACTIVATE` sets WS_VISIBLE so `EnumWindows`+`IsWindowVisible` resolves
/// it). Returns the raw HWND as `isize`.
fn create_victim_window() -> Option<isize> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, RegisterClassW, ShowWindow, SW_SHOWNOACTIVATE, WNDCLASSW,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
    };

    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: plain passthrough window procedure; all args come from the OS dispatcher.
        unsafe { DefWindowProcW(hwnd, msg, w, l) }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let class_name = wide("HskMt096FreezeVictimWindow");
    // SAFETY: standard RegisterClassW + CreateWindowExW on this thread; the class name and window proc
    // outlive the window; a null parent/menu is valid for a top-level popup.
    let hwnd = unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        if RegisterClassW(&wc) == 0 {
            eprintln!("freeze victim: RegisterClassW failed");
            return None;
        }
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            wide("hsk-mt096-freeze-victim").as_ptr(),
            WS_POPUP,
            -32000,
            -32000,
            64,
            64,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        eprintln!("freeze victim: CreateWindowExW failed");
        return None;
    }
    // SAFETY: hwnd was just created on this thread; SW_SHOWNOACTIVATE sets WS_VISIBLE without focus.
    unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
    Some(hwnd as isize)
}

/// Service every pending message once (a live pump slice).
fn pump_messages_once() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };
    // SAFETY: standard PeekMessage/Translate/Dispatch loop on the window's owning thread.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn destroy_victim_window(hwnd: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;
    // SAFETY: hwnd is this thread's window handle (created above), destroyed exactly once before exit.
    unsafe { DestroyWindow(hwnd as windows_sys::Win32::Foundation::HWND) };
}

// ── the CRASH VICTIM (the production crash-client shape, then a REAL unhandled access violation) ───────

/// Env gate turning the re-exec'd child into the CRASH VICTIM (never set in a normal run).
const CRASH_VICTIM_ENV: &str = "HSK_MT096_CRASH_VICTIM";
/// The CONTROL socket base name the victim derives the crash socket from (the §6.13.6 rendezvous —
/// the SAME `crash_socket_path` derivation both production sides use).
const CRASH_VICTIM_CONTROL_SOCK_ENV: &str = "HSK_MT096_CRASH_CONTROL_SOCK";

/// The CRASH-VICTIM entry. In a normal run (env unset) it is a no-op. Re-exec'd with
/// [`CRASH_VICTIM_ENV`] set, it is the EXACT production Handshake crash-client shape: derive the crash
/// socket from the control socket (`palmistry::crash_capture::crash_socket_path` — the library
/// derivation pinned equal across crates), connect the REAL `minidumper::Client` with a bounded retry
/// (the launched watcher binds the server during startup), attach the REAL `crash-handler` OS-exception
/// handler whose callback reports the faulting thread id then requests the OUT-OF-PROCESS dump — and
/// then REALLY CRASH: an actual unhandled access violation (a write through a null pointer). This
/// process DIES abnormally (`STATUS_ACCESS_VIOLATION`); the dump is written by the WATCHER process from
/// outside (§6.13.6 — the crashing process never dumps itself).
#[test]
fn mt096_crash_victim_entry() {
    if std::env::var(CRASH_VICTIM_ENV).ok().as_deref() != Some("1") {
        return; // Not re-exec'd as the victim: a no-op in a normal test run.
    }
    let control_socket = std::env::var(CRASH_VICTIM_CONTROL_SOCK_ENV)
        .expect("crash victim needs the control socket env");
    let crash_socket = palmistry::crash_capture::crash_socket_path(&control_socket);

    // Bounded late-arm connect (the production `arm_crash_client_late` shape): the watcher binds the
    // derived socket during its startup, so a fresh spawn needs a short retry window.
    let connect_deadline = Instant::now() + Duration::from_secs(15);
    let client = loop {
        match minidumper::Client::with_name(minidumper::SocketName::path(&crash_socket)) {
            Ok(c) => break std::sync::Arc::new(c),
            Err(err) => {
                if Instant::now() >= connect_deadline {
                    panic!("crash victim: could not connect the crash client on '{crash_socket}': {err}");
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };

    // The EXACT production client callback: report the faulting thread id (typed 8-byte LE u64), then
    // request the OUT-OF-PROCESS dump.
    let client_cb = std::sync::Arc::clone(&client);
    #[allow(unsafe_code)]
    let _handler = crash_handler::CrashHandler::attach(unsafe {
        crash_handler::make_crash_event(move |cc: &crash_handler::CrashContext| {
            let tid: u64 = cc.thread_id as u64;
            let _ = client_cb.send_message(
                palmistry::crash_capture::MSG_KIND_FAULTING_THREAD_ID,
                tid.to_le_bytes(),
            );
            crash_handler::CrashEventResult::Handled(client_cb.request_dump(cc).is_ok())
        })
    })
    .expect("crash victim: attach the real crash handler");

    println!("MT096_CRASH_VICTIM_ARMED");
    let _ = std::io::stdout().flush();

    // REALLY CRASH: a genuine unhandled access violation on this thread. The OS delivers the exception
    // to the installed handler, whose callback requests the out-of-process dump from the WATCHER; the
    // handler then lets the process DIE (this is NOT simulate_exception — the process does not survive).
    let null: *mut u32 = std::ptr::null_mut();
    // SAFETY(deliberate crash): this is the crash trigger — an intentional invalid write so the process
    // raises a REAL STATUS_ACCESS_VIOLATION. It runs ONLY in the re-exec'd victim child (env-gated).
    unsafe { null.write_volatile(0xDEAD_BEEF) };
    unreachable!("the access violation must have terminated the victim process");
}

// ── GATE 1: the REAL spawned palmistry.exe confirms a GENUINE freeze of the victim ─────────────────────

#[test]
#[ignore = "LIVE real-host gate (AC-016-1): spawns the REAL palmistry.exe + a re-exec'd HandshakeApp-shaped \
            window/ring victim that genuinely freezes for ~30s; the production Win32HungWindowProbe + \
            zero-coop ring reader must confirm the freeze (double-signal) and persist the durable survivor \
            record. Every wait is hard-bounded (reader thread + recv_timeout, bounded reaps/polls). Run \
            with --ignored on a real interactive host."]
fn live_palmistry_confirms_freeze_of_frozen_handshake_shaped_child() {
    assert_no_local_artifact_dir();
    let dir = live_artifact_dir();
    std::fs::create_dir_all(&dir).expect("create the external MT-096 live artifact dir");

    let tag = unique_tag("mt096-freeze");
    let ring_path = dir.join(format!("ring-{tag}.ring"));
    let control_socket = format!("hsk-{tag}");
    let survivor_dir = dir.join(format!("survivors-{tag}"));
    std::fs::create_dir_all(&survivor_dir).expect("create the scoped survivor dir");

    // 1) Spawn the VICTIM (a HandshakeApp-shaped process: real ring writer + real pumped window). It
    //    creates the ring, so the watcher's bounded ring-open retry finds a valid ring once READY prints.
    let exe = std::env::current_exe().expect("current_exe for the integration-test binary");
    let mut victim = ChildGuard::new(
        Command::new(&exe)
            .env(FREEZE_VICTIM_ENV, "1")
            .env(FREEZE_VICTIM_RING_ENV, &ring_path)
            .env(FREEZE_VICTIM_HEALTHY_MS_ENV, "6000")
            .env(FREEZE_VICTIM_FROZEN_MS_ENV, "45000")
            .args(["--exact", "mt096_freeze_victim_entry", "--nocapture"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the freeze victim child"),
    );
    let victim_pid = victim.id();
    let victim_stdout = victim
        .child_mut()
        .stdout
        .take()
        .expect("victim stdout piped");
    let victim_rx = spawn_bounded_line_reader(victim_stdout);
    assert!(
        wait_marker_bounded(&victim_rx, "MT096_VICTIM_READY", Duration::from_secs(20)),
        "the freeze victim must report READY (ring created + real window shown)"
    );

    // 2) Spawn the REAL palmistry.exe watching the victim (production run_watcher: zero-coop reader +
    //    REAL Win32 probe + the §6.13.5 double-signal freeze detector + the MT-093 survivor store).
    let mut watcher = ChildGuard::new(spawn_palmistry_watcher(
        victim_pid,
        &tag,
        &ring_path,
        &control_socket,
        &survivor_dir,
    ));

    // 3) The victim freezes on its own schedule (~6s of healthy pumping first — the no-false-positive
    //    window: an advancing heartbeat + a responding pump must NOT trip the detector).
    assert!(
        wait_marker_bounded(&victim_rx, "MT096_VICTIM_FROZEN", Duration::from_secs(30)),
        "the freeze victim must report FROZEN (it stopped heartbeats + blocked its pump)"
    );

    // 4) BOUNDED poll for the durable FREEZE survivor record (read back through the PRODUCTION
    //    SurvivorStore). Confirmation needs staleness (~5s FREEZE_THRESHOLD) + the probe corroboration
    //    (+ the 300ms poll cadence), so allow a generous-but-bounded window.
    let deadline = Instant::now() + Duration::from_secs(30);
    let freeze_record = loop {
        let found = SurvivorStore::open(&survivor_dir)
            .ok()
            .and_then(|store| {
                store
                    .records()
                    .iter()
                    .find(|stored| stored.record.kind == SurvivorRecordKind::Freeze)
                    .map(|stored| (stored.record.clone(), stored.path.clone()))
            });
        if let Some(found) = found {
            break found;
        }
        if Instant::now() >= deadline {
            kill_child(watcher.child_mut());
            let stderr = stderr_after_exit(watcher.child_mut());
            panic!(
                "AC-016-1 LIVE: the REAL spawned palmistry.exe must persist a durable FREEZE survivor \
                 record for the genuinely frozen victim within the bounded window; palmistry stderr: \
                 {stderr}"
            );
        }
        std::thread::sleep(Duration::from_millis(500));
    };
    let (record, record_path) = freeze_record;

    // The record carries the REAL double-signal facts: confirmed via the REAL Win32 probe against the
    // victim's REAL window (NotResponding — never a fake probe), past the production 5s threshold, for
    // the watched victim pid, with the last heartbeat the victim actually published.
    assert_eq!(record.session_id, tag, "the record names this session");
    assert_eq!(
        record.process_id, victim_pid,
        "the record names the watched (frozen) victim pid"
    );
    assert_eq!(
        record.probe_result,
        SurvivorProbeResult::NotResponding,
        "AC-016-1 LIVE: the freeze was corroborated by the REAL Win32 hung-window probe (NotResponding)"
    );
    assert!(
        record.stale_ms >= 5_000,
        "the freeze confirmed past the production FREEZE_THRESHOLD (stale_ms={})",
        record.stale_ms
    );
    assert!(
        record.last_heartbeat_counter >= 2,
        "the record carries the victim's last published (advancing-then-stalled) heartbeat counter \
         (got {})",
        record.last_heartbeat_counter
    );

    // 5) Clean Shutdown reaps the watcher (exit 0) — the victim is FROZEN, not dead, so NO crash record.
    let ack = send_control(
        &control_socket,
        r#"{"type":"Shutdown"}"#,
        Duration::from_secs(10),
    );
    let watcher_code = match ack {
        Ok(reply) => {
            assert!(
                reply.contains("Ack"),
                "expected Ack to Shutdown, got: {reply}"
            );
            wait_for_exit(watcher.child_mut(), Duration::from_secs(10))
        }
        Err(err) => {
            let code = wait_for_exit(watcher.child_mut(), Duration::from_secs(5));
            assert_eq!(
                code,
                Some(0),
                "shutdown ack failed and the watcher did not exit 0: {err}"
            );
            code
        }
    };
    assert_eq!(
        watcher_code,
        Some(0),
        "a clean Shutdown must reap palmistry with exit 0"
    );
    let crash_json = dir.join(format!("palmistry-crash-{tag}.json"));
    assert!(
        !crash_json.exists(),
        "a frozen-but-alive victim + clean Shutdown must write NO crash record"
    );

    // 6) Durable typed VERDICT artifact — written ONLY after every assertion above held. The
    //    handshake-native capstone manifest (scenario5) derives its AC-016-1 verdict from this file.
    let verdict = serde_json::json!({
        "schema_version": "hsk.mt096.live_gate_verdict@0.1",
        "scenario": "AC-016-1",
        "gate": "freeze_live_capture",
        "pass": true,
        "watched_process_id": victim_pid,
        "stale_ms": record.stale_ms,
        "last_heartbeat_counter": record.last_heartbeat_counter,
        "probe_result": "NotResponding",
        "survivor_record": abs_display(&record_path),
        "palmistry_exit_code": 0,
        "generated_at": handshake_diag_ring::run_at_now(),
    });
    let verdict_path = dir.join("freeze_live_capture_verdict.json");
    std::fs::write(
        &verdict_path,
        format!("{}\n", serde_json::to_string_pretty(&verdict).unwrap()),
    )
    .expect("write the freeze live-gate verdict artifact");
    println!(
        "AC-016-1 LIVE verdict: {} (survivor record: {})",
        abs_display(&verdict_path),
        abs_display(&record_path)
    );

    // Reap the (still frozen) victim; the ring file stays as a durable artifact next to the records.
    kill_child(victim.child_mut());
    assert_no_local_artifact_dir();
}

// ── GATE 2: the REAL spawned palmistry.exe captures a validated minidump from a REALLY crashed client ──

#[test]
#[ignore = "LIVE real-host gate (AC-016-2): spawns the REAL palmistry.exe + a re-exec'd crash-client victim \
            that REALLY crashes (a genuine unhandled access violation — the process dies). The shipped \
            CrashServerHandler must write the minidump OUT-OF-PROCESS and the watcher must persist the \
            rich CrashContextMinidump record; the dump is validated with the minidump reader (threads, \
            modules, exception stream). minidumper IPC — every wait hard-bounded. Run with --ignored on a \
            real interactive host."]
fn live_palmistry_captures_minidump_from_really_crashed_client() {
    assert_no_local_artifact_dir();
    let dir = live_artifact_dir();
    std::fs::create_dir_all(&dir).expect("create the external MT-096 live artifact dir");

    let tag = unique_tag("mt096-crash");
    let ring_path = dir.join(format!("ring-{tag}.ring"));
    let control_socket = format!("hsk-{tag}");
    let survivor_dir = dir.join(format!("survivors-{tag}"));
    std::fs::create_dir_all(&survivor_dir).expect("create the scoped survivor dir");

    // The ring the watcher maps (the test is the Tier-2 writer here; the victim's job is to CRASH, and
    // the crash record bundles the last heartbeat read passively from this ring).
    let writer = DiagRingWriter::create(&ring_path, DEFAULT_CAPACITY).expect("create the ring");
    writer.write_heartbeat(7, now_nanos());

    // 1) Spawn the CRASH VICTIM first (it retries the crash-socket connect until the watcher binds it,
    //    then really crashes), so the watcher can be given the victim's pid at startup.
    let exe = std::env::current_exe().expect("current_exe for the integration-test binary");
    let mut victim = ChildGuard::new(
        Command::new(&exe)
            .env(CRASH_VICTIM_ENV, "1")
            .env(CRASH_VICTIM_CONTROL_SOCK_ENV, &control_socket)
            .args(["--exact", "mt096_crash_victim_entry", "--nocapture"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the crash victim child"),
    );
    let victim_pid = victim.id();
    let victim_stdout = victim
        .child_mut()
        .stdout
        .take()
        .expect("victim stdout piped");
    let victim_rx = spawn_bounded_line_reader(victim_stdout);

    // 2) Spawn the REAL palmistry.exe watching the victim. Its startup binds the crash server on the
    //    DERIVED socket (the §6.13.6 rendezvous) — the victim's bounded connect then succeeds.
    let mut watcher = ChildGuard::new(spawn_palmistry_watcher(
        victim_pid,
        &tag,
        &ring_path,
        &control_socket,
        &survivor_dir,
    ));

    // The victim reports ARMED (client connected + real crash handler attached) just before it crashes.
    assert!(
        wait_marker_bounded(
            &victim_rx,
            "MT096_CRASH_VICTIM_ARMED",
            Duration::from_secs(30)
        ),
        "the crash victim must arm its real crash client (connect + attach) before crashing"
    );

    // 3) The victim REALLY crashes: reap it (bounded) and assert the death was ABNORMAL — a genuine
    //    STATUS_ACCESS_VIOLATION exit, not a clean return (the 'really crashed' fact of this gate).
    let victim_code = wait_for_exit(victim.child_mut(), Duration::from_secs(30));
    let victim_code = victim_code
        .expect("the crash victim must die (bounded) after its deliberate access violation");
    assert_ne!(
        victim_code, 0,
        "the victim must die ABNORMALLY (a real crash), not exit cleanly"
    );

    // 4) The watcher observes the abnormal parent death, finishes the bounded post-death finalize, and
    //    exits 0 after persisting the RICH crash record (the dump was already written by its shipped
    //    CrashServerHandler during the victim's dying request_dump).
    let watcher_code = wait_for_exit(watcher.child_mut(), Duration::from_secs(30));
    if watcher_code != Some(0) {
        kill_child(watcher.child_mut());
        let stderr = stderr_after_exit(watcher.child_mut());
        panic!(
            "the watcher must exit 0 after capturing the crash; got {watcher_code:?}; stderr={stderr}"
        );
    }

    // 5) The RICH crash record: detection=CrashContextMinidump, naming the LOCAL out-of-process dump.
    let record_path = dir.join(format!("palmistry-crash-{tag}.json"));
    let record_bytes = std::fs::read(&record_path).unwrap_or_else(|err| {
        panic!(
            "AC-016-2 LIVE: the watcher must persist the rich crash record at {}: {err}",
            record_path.display()
        )
    });
    let record: serde_json::Value =
        serde_json::from_slice(&record_bytes).expect("crash record parses as JSON");
    assert_eq!(
        record["detection"]["detection"], "CrashContextMinidump",
        "AC-016-2 LIVE: a really-crashed client with a connected crash client must produce the RICH \
         CrashContextMinidump record: {record}"
    );
    assert_eq!(
        record["process_id"].as_u64(),
        Some(victim_pid as u64),
        "the crash record names the crashed victim pid: {record}"
    );
    let dump_named = record["minidump_path"].as_str().unwrap_or("");
    assert!(
        !dump_named.is_empty() && !dump_named.contains("://"),
        "the rich record must name the LOCAL dump file (never a URL): {record}"
    );

    // 6) VALIDATE the out-of-process dump with the `minidump` READER (the MT-092 machinery, reused): a
    //    well-formed dump of the DEAD victim process with the crashing thread, the loaded modules, AND
    //    the exception stream carrying the REAL access violation.
    let dump_path = PathBuf::from(dump_named);
    let dump_bytes = std::fs::read(&dump_path).expect("read the out-of-process minidump");
    assert!(
        dump_bytes.len() > 1024,
        "a real minidump is non-trivial, got {} bytes",
        dump_bytes.len()
    );
    let dump = minidump::Minidump::read(dump_bytes.as_slice())
        .expect("the out-of-process dump of the dead victim must parse as a well-formed minidump");
    let threads = dump
        .get_stream::<minidump::MinidumpThreadList>()
        .expect("the minidump must carry a thread list (the crashing thread)");
    assert!(
        !threads.threads.is_empty(),
        "the dump must contain at least one thread"
    );
    let modules = dump
        .get_stream::<minidump::MinidumpModuleList>()
        .expect("the minidump must carry a module list (loaded modules)");
    assert!(
        !modules.iter().collect::<Vec<_>>().is_empty(),
        "the dump must list at least one loaded module"
    );
    let exception = dump
        .get_stream::<minidump::MinidumpException>()
        .expect("a REALLY crashed client's dump must carry the exception stream");
    let exception_code = exception.raw.exception_record.exception_code;
    const STATUS_ACCESS_VIOLATION: u32 = 0xC000_0005;
    assert_eq!(
        exception_code, STATUS_ACCESS_VIOLATION,
        "the exception stream must carry the victim's REAL access violation"
    );

    // 7) The durable CRASH survivor record (MT-093) exists in the scoped store, carrying the rich facts.
    let store = SurvivorStore::open(&survivor_dir).expect("open the scoped survivor store");
    let crash_survivor = store
        .records()
        .iter()
        .find(|stored| stored.record.kind == SurvivorRecordKind::Crash)
        .expect("AC-016-2 LIVE: the watcher must persist a durable CRASH survivor record");
    assert_eq!(
        crash_survivor.record.minidump_path.as_deref(),
        Some(dump_path.as_path()),
        "the survivor record names the same local dump"
    );

    // 8) Durable typed VERDICT artifact — written ONLY after every assertion above held. The
    //    handshake-native capstone manifest (scenario5) derives its AC-016-2 verdict from this file.
    let verdict = serde_json::json!({
        "schema_version": "hsk.mt096.live_gate_verdict@0.1",
        "scenario": "AC-016-2",
        "gate": "crash_live_capture",
        "pass": true,
        "crashed_process_id": victim_pid,
        "victim_exit_code": victim_code,
        "exception_code": exception_code,
        "minidump": abs_display(&dump_path),
        "minidump_len": dump_bytes.len(),
        "minidump_validated": true,
        "crash_record": abs_display(&record_path),
        "survivor_record": abs_display(&crash_survivor.path),
        "detection": "CrashContextMinidump",
        "generated_at": handshake_diag_ring::run_at_now(),
    });
    let verdict_path = dir.join("crash_live_capture_verdict.json");
    std::fs::write(
        &verdict_path,
        format!("{}\n", serde_json::to_string_pretty(&verdict).unwrap()),
    )
    .expect("write the crash live-gate verdict artifact");
    println!(
        "AC-016-2 LIVE verdict: {} (minidump: {}, {} bytes)",
        abs_display(&verdict_path),
        abs_display(&dump_path),
        dump_bytes.len()
    );

    drop(writer);
    assert_no_local_artifact_dir();
}
