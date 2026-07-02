//! MT-089 LIFECYCLE PROOFS (the deliverable, §6.13.3 — get the inversion exactly right).
//!
//! These are end-to-end tests that drive the REAL `palmistry` binary as a SEPARATE PROCESS against a
//! REAL hard-killed dummy parent — NOT a simulated/mocked parent exit (the MT note + Spec-Realism Gate
//! require a real hard kill). They prove, against the running binary:
//!
//! - **PT-009-A / AC-009-1**: launches with valid inputs and STAYS RUNNING with no Shutdown.
//! - **PT-009-B / AC-009-3**: a `Ping` does NOT cause exit; a `Shutdown` exits promptly + cleanly.
//! - **PT-009-C / AC-009-4**: HARD-KILL the dummy parent => Palmistry is STILL RUNNING after the parent
//!   is dead AND RECORDED the abnormal parent exit (the survivor record); it then exits on Shutdown /
//!   the bounded finalize — NOT at the instant of parent death.
//! - **AC-009-6**: a `Shutdown` that PRECEDES the parent exit records NO crash (clean-shutdown-is-not-a-
//!   crash).
//! - **AC-009-2**: a partial / malformed start exits non-zero with a clear error.
//!
//! The dummy parent is a long-lived OS process the test spawns and can TerminateProcess/kill at will
//! (here: the platform sleep command). Palmistry gets the dummy's pid and holds its handle.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use handshake_diag_ring::ring::DEFAULT_CAPACITY;
use handshake_diag_ring::DiagRingWriter;

// ---------------------------------------------------------------------------------------------------
// Harness helpers
// ---------------------------------------------------------------------------------------------------

/// Resolve the built `palmistry` binary from CARGO_BIN_EXE (cargo sets it for integration tests of a
/// binary crate). This is the REAL binary under test.
fn palmistry_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_palmistry"))
}

/// A unique token for this test invocation so parallel tests never collide on socket / ring / pid file.
fn unique_tag(label: &str) -> String {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{label}-{pid}-{nanos}")
}

/// Create a REAL MT-081 ring backing file under the OS temp dir and return its path (Palmistry
/// validates this on startup; the passive read loop is MT-090). The file is left on disk for the
/// watcher to open; the `RingGuard` removes it on drop.
fn make_ring(tag: &str) -> (PathBuf, RingGuard) {
    let path = std::env::temp_dir().join(format!("handshake-diag-{tag}.ring"));
    let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).expect("create ring");
    // Write one heartbeat so the ring is non-empty (not required, but realistic).
    writer.write_heartbeat(1, 1);
    drop(writer);
    (path.clone(), RingGuard(path))
}

struct RingGuard(PathBuf);
impl Drop for RingGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Spawn a long-lived DUMMY PARENT process the test can hard-kill. On Windows, a `ping` loopback with a
/// large count is a reliable, killable sleeper that needs no extra binary; elsewhere `sleep`. The point
/// is a real OS process with a real pid that Palmistry watches and that we TerminateProcess/kill.
fn spawn_dummy_parent() -> Child {
    #[cfg(windows)]
    {
        // `ping -n 100000 127.0.0.1` runs ~100000s; we kill it long before. CREATE_NO_WINDOW keeps it
        // headless (HBR-QUIET) — but std has no direct flag; a console child of a test is already
        // headless under the test harness. stdio nulled so it is silent.
        Command::new("cmd")
            .args(["/C", "ping", "-n", "100000", "127.0.0.1"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn dummy parent (ping)")
    }
    #[cfg(not(windows))]
    {
        Command::new("sleep")
            .arg("100000")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn dummy parent (sleep)")
    }
}

/// Spawn the REAL palmistry watcher binary with the four required inputs via ENV (the cleaner channel
/// for a spawned sibling). Returns the child handle.
fn spawn_palmistry(parent_pid: u32, session_id: &str, ring_path: &Path, socket: &str) -> Child {
    Command::new(palmistry_bin())
        .env("HANDSHAKE_PARENT_PID", parent_pid.to_string())
        .env("HANDSHAKE_SESSION_ID", session_id)
        .env("HANDSHAKE_RING_PATH", ring_path)
        .env("HANDSHAKE_CONTROL_SOCK", socket)
        .env("PALMISTRY_LOG", "warn")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn palmistry watcher")
}

/// Is the child still running? `try_wait` returns Ok(None) while alive. Does NOT reap a live child.
fn still_running(child: &mut Child) -> bool {
    matches!(child.try_wait(), Ok(None))
}

/// Wait up to `timeout` for the child to EXIT; returns the exit code if it did, else None (still alive).
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

/// Connect to Palmistry's control socket and send one newline-delimited JSON message, reading the reply
/// line. Retries the connect for up to `connect_timeout` because the watcher binds the socket slightly
/// after spawn. Returns the reply line (trimmed) on success.
fn send_control(
    socket: &str,
    message_json: &str,
    connect_timeout: Duration,
) -> std::io::Result<String> {
    use interprocess::local_socket::traits::Stream as _;
    use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};

    let name = socket.to_ns_name::<GenericNamespaced>()?;
    let deadline = Instant::now() + connect_timeout;
    let conn = loop {
        match Stream::connect(name.clone()) {
            Ok(c) => break c,
            Err(e) => {
                if Instant::now() >= deadline {
                    return Err(e);
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    };
    let mut reader = BufReader::new(conn);
    {
        let w = reader.get_mut();
        w.write_all(message_json.as_bytes())?;
        w.write_all(b"\n")?;
        w.flush()?;
    }
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line.trim_end().to_string())
}

/// The survivor record path Palmistry writes next to the ring.
fn survivor_path(ring_path: &Path, session_id: &str) -> PathBuf {
    let dir = ring_path.parent().unwrap_or_else(|| Path::new("."));
    let safe: String = session_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(format!("palmistry-survivor-{safe}.json"))
}

struct SurvivorGuard(PathBuf);
impl Drop for SurvivorGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Read + parse the survivor record JSON, retrying until it appears (Palmistry writes it on exit).
fn read_survivor(path: &Path, timeout: Duration) -> Option<serde_json::Value> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                return Some(v);
            }
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Best-effort kill + reap of a child (cleanup).
fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

// ---------------------------------------------------------------------------------------------------
// AC-009-1 / PT-009-A — launches with valid inputs and STAYS RUNNING with no Shutdown.
// ---------------------------------------------------------------------------------------------------

#[test]
fn launches_and_stays_running_without_shutdown() {
    let tag = unique_tag("stay-alive");
    let (ring_path, _ring) = make_ring(&tag);
    let socket = format!("hsk-palm-{tag}");
    let survivor = SurvivorGuard(survivor_path(&ring_path, &tag));

    let mut parent = spawn_dummy_parent();
    let mut watcher = spawn_palmistry(parent.id(), &tag, &ring_path, &socket);

    // Give it time to bind + arm, then assert it is STILL RUNNING (it must not self-exit, AC-009-1).
    std::thread::sleep(Duration::from_millis(700));
    let alive = still_running(&mut watcher);

    // Clean up: shutdown the watcher, kill the dummy parent.
    let _ = send_control(&socket, r#"{"type":"Shutdown"}"#, Duration::from_secs(3));
    let _ = wait_for_exit(&mut watcher, Duration::from_secs(3));
    kill_child(&mut parent);
    kill_child(&mut watcher);
    drop(survivor);

    assert!(
        alive,
        "palmistry must STAY RUNNING with no Shutdown (AC-009-1); it exited on its own"
    );
}

// ---------------------------------------------------------------------------------------------------
// AC-009-3 / PT-009-B — Ping does NOT exit; Shutdown exits promptly + cleanly.
// ---------------------------------------------------------------------------------------------------

#[test]
fn ping_does_not_exit_then_shutdown_exits() {
    let tag = unique_tag("ping-shutdown");
    let (ring_path, _ring) = make_ring(&tag);
    let socket = format!("hsk-palm-{tag}");
    let survivor = SurvivorGuard(survivor_path(&ring_path, &tag));

    let mut parent = spawn_dummy_parent();
    let mut watcher = spawn_palmistry(parent.id(), &tag, &ring_path, &socket);

    // Ping and confirm a Pong; the watcher must remain alive after it.
    let pong = send_control(&socket, r#"{"type":"Ping"}"#, Duration::from_secs(3))
        .expect("ping/pong over control socket");
    assert!(
        pong.contains("Pong"),
        "expected a Pong reply to Ping, got: {pong}"
    );
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        still_running(&mut watcher),
        "Ping must NOT cause Palmistry to exit (AC-009-3)"
    );

    // Now Shutdown: it must exit promptly + cleanly.
    let ack = send_control(&socket, r#"{"type":"Shutdown"}"#, Duration::from_secs(3))
        .expect("shutdown ack");
    assert!(
        ack.contains("Ack"),
        "expected an Ack to Shutdown, got: {ack}"
    );
    let code = wait_for_exit(&mut watcher, Duration::from_secs(5));
    assert_eq!(
        code,
        Some(0),
        "Shutdown must cause a prompt CLEAN exit (code 0) (AC-009-3)"
    );

    // Clean-shutdown-is-not-a-crash (AC-009-6): the survivor record records NO abnormal parent exit.
    let rec = read_survivor(&survivor.0, Duration::from_secs(3))
        .expect("survivor record written on clean shutdown");
    assert_eq!(
        rec["abnormal_parent_exit"],
        serde_json::Value::Bool(false),
        "a clean Shutdown must record NO abnormal/crash exit (AC-009-6): {rec}"
    );
    assert_eq!(rec["exit_reason"]["reason"], "CleanShutdown");

    kill_child(&mut parent);
    kill_child(&mut watcher);
    drop(survivor);
}

// ---------------------------------------------------------------------------------------------------
// AC-009-4 / PT-009-C — SURVIVES a hard-kill of the parent + RECORDS the abnormal exit; then exits.
// THE INVERSION: a normal child dies with its parent; Palmistry must outlive it and record its death.
// ---------------------------------------------------------------------------------------------------

#[test]
fn survives_hard_kill_of_parent_and_records_abnormal_exit() {
    let tag = unique_tag("survive-kill");
    let (ring_path, _ring) = make_ring(&tag);
    let socket = format!("hsk-palm-{tag}");
    let survivor = SurvivorGuard(survivor_path(&ring_path, &tag));

    let mut parent = spawn_dummy_parent();
    let parent_pid = parent.id();
    let mut watcher = spawn_palmistry(parent_pid, &tag, &ring_path, &socket);

    // Let the watcher fully arm its parent handle (OpenProcess + WaitForSingleObject).
    std::thread::sleep(Duration::from_millis(700));
    assert!(
        still_running(&mut watcher),
        "watcher should be alive before the parent is killed"
    );

    // HARD-KILL the dummy parent (TerminateProcess on Windows via Child::kill / SIGKILL elsewhere).
    parent.kill().expect("hard-kill dummy parent");
    parent.wait().expect("reap dummy parent");

    // THE INVERSION PROOF: shortly after the parent is DEAD, Palmistry must STILL BE RUNNING. We check
    // repeatedly across a window that comfortably exceeds the instant of death but is shorter than the
    // bounded post-death finalize, so the watcher is observed ALIVE while the parent is dead.
    let mut observed_alive_after_death = false;
    let check_deadline = Instant::now() + Duration::from_millis(300);
    while Instant::now() < check_deadline {
        if still_running(&mut watcher) {
            observed_alive_after_death = true;
        } else {
            observed_alive_after_death = false;
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    assert!(
        observed_alive_after_death,
        "INVERSION FAILED: Palmistry died with / immediately after its parent (AC-009-4) — a watcher \
         that dies with its parent cannot record the parent's death (\u{a7}6.13.3)"
    );

    // After the bounded finalize, Palmistry exits on its own (no Shutdown sent) with code 0.
    let code = wait_for_exit(&mut watcher, Duration::from_secs(5));
    assert_eq!(
        code,
        Some(0),
        "after the bounded post-death finalize, Palmistry should exit cleanly (code 0)"
    );

    // It RECORDED the abnormal parent exit (the typed parent-died signal MT-092 consumes).
    let rec = read_survivor(&survivor.0, Duration::from_secs(3))
        .expect("survivor record written after parent death");
    assert_eq!(
        rec["abnormal_parent_exit"],
        serde_json::Value::Bool(true),
        "Palmistry must RECORD the abnormal parent exit (AC-009-4): {rec}"
    );
    assert_eq!(
        rec["parent_pid"].as_u64(),
        Some(parent_pid as u64),
        "survivor record must name the watched parent pid"
    );
    assert_eq!(
        rec["exit_reason"]["reason"], "ParentDiedAbnormally",
        "exit reason must be ParentDiedAbnormally after a hard kill with no Shutdown: {rec}"
    );
    assert_eq!(
        rec["shutdown_received"],
        serde_json::Value::Bool(false),
        "no Shutdown was sent in this scenario"
    );

    kill_child(&mut watcher);
    drop(survivor);
}

// ---------------------------------------------------------------------------------------------------
// AC-009-4 variant — a Shutdown DURING the post-death finalize window yields ShutdownAfterParentDeath
// (still records the abnormal exit; exits on the explicit command).
// ---------------------------------------------------------------------------------------------------

#[test]
fn shutdown_after_parent_death_still_records_abnormal() {
    let tag = unique_tag("shutdown-after-death");
    let (ring_path, _ring) = make_ring(&tag);
    let socket = format!("hsk-palm-{tag}");
    let survivor = SurvivorGuard(survivor_path(&ring_path, &tag));

    let mut parent = spawn_dummy_parent();
    let parent_pid = parent.id();
    let mut watcher = spawn_palmistry(parent_pid, &tag, &ring_path, &socket);

    std::thread::sleep(Duration::from_millis(700));
    parent.kill().expect("hard-kill dummy parent");
    parent.wait().expect("reap dummy parent");

    // Send Shutdown promptly — it should land DURING the bounded finalize (default 500ms), so the
    // watcher exits on the explicit command but STILL records the abnormal exit it already observed.
    std::thread::sleep(Duration::from_millis(80));
    let _ = send_control(&socket, r#"{"type":"Shutdown"}"#, Duration::from_secs(3));
    let code = wait_for_exit(&mut watcher, Duration::from_secs(5));
    assert_eq!(
        code,
        Some(0),
        "should exit cleanly on the post-death Shutdown"
    );

    let rec = read_survivor(&survivor.0, Duration::from_secs(3)).expect("survivor record written");
    assert_eq!(
        rec["abnormal_parent_exit"], serde_json::Value::Bool(true),
        "the abnormal parent death is still recorded even though a Shutdown ended the watcher: {rec}"
    );
    // Either ShutdownAfterParentDeath (Shutdown landed in the window) or ParentDiedAbnormally (the
    // finalize elapsed first) — both record the abnormal exit, which is the contract. Assert it is one
    // of the two abnormal-recording reasons.
    let reason = rec["exit_reason"]["reason"].as_str().unwrap_or("");
    assert!(
        reason == "ShutdownAfterParentDeath" || reason == "ParentDiedAbnormally",
        "reason after a hard kill must record the abnormal exit, got {reason}: {rec}"
    );

    kill_child(&mut watcher);
    drop(survivor);
}

// ---------------------------------------------------------------------------------------------------
// AC-009-2 — refuses a partial / malformed start with a clear non-zero error.
// ---------------------------------------------------------------------------------------------------

#[test]
fn refuses_partial_start_nonzero_exit() {
    // Spawn with NO inputs at all (no env, no args) — must exit non-zero with a clear error, not a
    // silent half-start.
    let mut child = Command::new(palmistry_bin())
        .env_remove("HANDSHAKE_PARENT_PID")
        .env_remove("HANDSHAKE_SESSION_ID")
        .env_remove("HANDSHAKE_RING_PATH")
        .env_remove("HANDSHAKE_CONTROL_SOCK")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn palmistry with no config");

    let mut stderr = String::new();
    if let Some(err) = child.stderr.take() {
        let mut r = BufReader::new(err);
        let mut line = String::new();
        while r.read_line(&mut line).unwrap_or(0) > 0 {
            stderr.push_str(&line);
            line.clear();
        }
    }
    let code = wait_for_exit(&mut child, Duration::from_secs(5));
    assert_eq!(
        code,
        Some(2),
        "a partial/unconfigured start must exit non-zero (2) (AC-009-2); stderr was: {stderr}"
    );
    assert!(
        stderr.contains("refusing to start") && stderr.to_lowercase().contains("parent_pid"),
        "the refusal must clearly name the missing input; stderr was: {stderr}"
    );
}

// ---------------------------------------------------------------------------------------------------
// AC-009-2 — refuses a malformed PID specifically.
// ---------------------------------------------------------------------------------------------------

#[test]
fn refuses_malformed_pid_nonzero_exit() {
    let tag = unique_tag("bad-pid");
    let (ring_path, _ring) = make_ring(&tag);
    let mut child = Command::new(palmistry_bin())
        .env("HANDSHAKE_PARENT_PID", "not-a-pid")
        .env("HANDSHAKE_SESSION_ID", &tag)
        .env("HANDSHAKE_RING_PATH", &ring_path)
        .env("HANDSHAKE_CONTROL_SOCK", format!("hsk-palm-{tag}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn palmistry with bad pid");

    let code = wait_for_exit(&mut child, Duration::from_secs(5));
    assert_eq!(
        code,
        Some(2),
        "a malformed PID must exit non-zero (2) (AC-009-2)"
    );
    kill_child(&mut child);
}
