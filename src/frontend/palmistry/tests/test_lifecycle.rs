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

#[test]
fn explicit_v2_binary_uses_real_default_reader_and_rejects_wrong_nonce() {
    use handshake_diag_ring::ring::DiagRingWriterV2;
    use handshake_diag_ring::schema::DiagEventV2;
    let tag = unique_tag("v2-default");
    let path = std::env::temp_dir().join(format!("handshake-diag-{tag}.ring"));
    let _ring = RingGuard(path.clone());
    let nonce = [0x55; 16];
    let writer = DiagRingWriterV2::create(&path, DEFAULT_CAPACITY, nonce).unwrap();
    writer.write_heartbeat(12, 34);
    assert!(writer.write(DiagEventV2::behavior_observation(
        1, 12, 56, nonce, [0xaa; 32]
    )));
    let _survivor = SurvivorGuard(survivor_path(&path, &tag));
    let socket = format!("hsk-palm-{tag}");
    let spawn = |expected_nonce: &str, socket: &str| {
        let mut command = Command::new(palmistry_bin());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        command
            .args([
                "--parent-pid",
                &std::process::id().to_string(),
                "--session-id",
                &tag,
                "--ring-path",
                path.to_str().unwrap(),
                "--control-socket",
                socket,
                "--ring-version",
                "2",
                "--ring-nonce",
                expected_nonce,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    };
    struct OwnedWatcher(Child);
    impl Drop for OwnedWatcher {
        fn drop(&mut self) {
            kill_child(&mut self.0);
        }
    }
    let mut watcher = OwnedWatcher(spawn("55555555555555555555555555555555", &socket));
    let pong = send_v2_control_with_complete_deadline(
        &socket,
        r#"{"type":"Ping"}"#,
        Duration::from_secs(3),
    )
    .unwrap();
    assert!(pong.contains("Pong"));
    assert!(still_running(&mut watcher.0));
    let ack = send_v2_control_with_complete_deadline(
        &socket,
        r#"{"type":"Shutdown"}"#,
        Duration::from_secs(3),
    )
    .unwrap();
    assert!(ack.contains("Ack"));
    assert_eq!(
        wait_for_exit(&mut watcher.0, Duration::from_secs(5)),
        Some(0)
    );
    let mut wrong = OwnedWatcher(spawn(
        "56565656565656565656565656565656",
        &format!("{socket}-wrong"),
    ));
    assert_eq!(wait_for_exit(&mut wrong.0, Duration::from_secs(5)), Some(2));
    drop(wrong);
    drop(watcher);
    drop(writer);
}

/// Only the new v2 fixture uses this complete exchange deadline; legacy helpers stay unchanged.
fn send_v2_control_with_complete_deadline(
    socket: &str,
    message_json: &str,
    timeout: Duration,
) -> std::io::Result<String> {
    use interprocess::local_socket::traits::Stream as _;
    use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};
    use std::io::{self, Read};
    const MAX_FRAME: usize = 16 * 1024;
    if message_json.len() >= MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "v2 control request exceeds bound",
        ));
    }
    let deadline = Instant::now() + timeout;
    let timeout_error = || {
        io::Error::new(
            io::ErrorKind::TimedOut,
            "v2 control exchange deadline elapsed",
        )
    };
    #[cfg(unix)]
    let name = socket.to_ns_name::<GenericNamespaced>()?;
    #[cfg(windows)]
    let _validated_name = socket.to_ns_name::<GenericNamespaced>()?;
    let stream = loop {
        if Instant::now() >= deadline {
            return Err(timeout_error());
        }
        #[cfg(windows)]
        let attempt = {
            use interprocess::os::windows::named_pipe::local_socket::Stream as WindowsStream;
            use interprocess::os::windows::named_pipe::{pipe_mode::Bytes, DuplexPipeStream};
            use interprocess::ConnectWaitMode;
            use std::os::windows::io::OwnedHandle;
            // GenericNamespaced maps this exact socket to the local \\.\pipe\ prefix.
            // The ordinary local Stream::connect does not forward wait_mode on Windows.
            let path = format!(r"\\.\pipe\{socket}");
            DuplexPipeStream::<Bytes>::connect_by_path_with_wait_mode(
                path.as_str(),
                ConnectWaitMode::Timeout(Duration::ZERO),
            )
            .and_then(|pipe| {
                let handle: OwnedHandle = pipe.try_into().map_err(|_| {
                    io::Error::other(
                        "v2 control acquisition unexpectedly lost exclusive pipe ownership",
                    )
                })?;
                WindowsStream::try_from(handle)
                    .map(Stream::NamedPipe)
                    .map_err(|error| io::Error::other(error.to_string()))
            })
        };
        #[cfg(unix)]
        let attempt = {
            use interprocess::{local_socket::ConnectOptions, ConnectWaitMode};
            // Verified Unix implementation creates the nonblocking fd BEFORE connect for Timeout,
            // then waits at most the specified duration. Zero plus the outer deadline never blocks.
            ConnectOptions::new()
                .name(name.clone())
                .wait_mode(ConnectWaitMode::Timeout(Duration::ZERO))
                .nonblocking_stream(true)
                .connect_sync()
        };
        match attempt {
            Ok(stream) => break stream,
            Err(error) => {
                if Instant::now() >= deadline {
                    return Err(error);
                }
                std::thread::sleep(
                    Duration::from_millis(5)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
        }
    };
    // Windows pipe-drop normally sends dirty streams to a background flushing limbo. This owned
    // fixture exchange instead cancels/releases its actual handle on every return, including timeout.
    struct OwnedConnection(Stream);
    impl Drop for OwnedConnection {
        fn drop(&mut self) {
            #[cfg(windows)]
            {
                let Stream::NamedPipe(stream) = &self.0;
                stream.inner().assume_flushed();
            }
        }
    }
    let mut connection = OwnedConnection(stream);
    connection.0.set_nonblocking(true)?;
    let mut request = message_json.as_bytes().to_vec();
    request.push(b'\n');
    let mut written = 0;
    while written < request.len() {
        if Instant::now() >= deadline {
            return Err(timeout_error());
        }
        match connection.0.write(&request[written..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "v2 control write made no progress",
                ))
            }
            Ok(count) => written += count,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                std::thread::sleep(
                    Duration::from_millis(5)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Err(error) => return Err(error),
        }
    }
    // interprocess::local_socket::Stream::flush is an unconditional no-op on both platforms.
    // Do not use the underlying named-pipe FlushFileBuffers, which waits for the peer to consume.
    connection.0.flush()?;
    let mut reply = Vec::with_capacity(64);
    let mut byte = [0; 1];
    loop {
        if Instant::now() >= deadline {
            return Err(timeout_error());
        }
        match connection.0.read(&mut byte) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "v2 control peer closed before reply",
                ))
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    return String::from_utf8(reply)
                        .map(|line| line.trim_end_matches('\r').to_string())
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
                }
                if reply.len() == MAX_FRAME {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "v2 control reply exceeds bound",
                    ));
                }
                reply.push(byte[0]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                std::thread::sleep(
                    Duration::from_millis(5)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Err(error) => return Err(error),
        }
    }
}

#[test]
#[cfg(windows)]
fn current_owned_v2_child_private_seed_signs_exact_identity_and_keeps_control() {
    use handshake_diag_ring::ring::DiagRingWriterV2;
    use palmistry::control::{
        watcher_challenge_bytes, ControlMessage, ControlReply, WatcherSigningIdentity,
    };
    let session = uuid::Uuid::now_v7();
    let instance = uuid::Uuid::now_v7();
    let native_lease = std::net::UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let native_endpoint = native_lease.local_addr().unwrap();
    assert!(std::net::UdpSocket::bind(native_endpoint).is_err());
    let native_host_scope = unique_tag("signed-native-host");
    let parent_os_creation_time_100ns = {
        use windows_sys::Win32::Foundation::FILETIME;
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
        let mut creation = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut exit = creation;
        let mut kernel = creation;
        let mut user = creation;
        // SAFETY: query only this test process using its pseudo-handle and valid output pointers.
        assert_ne!(
            unsafe {
                GetProcessTimes(
                    GetCurrentProcess(),
                    &mut creation,
                    &mut exit,
                    &mut kernel,
                    &mut user,
                )
            },
            0
        );
        let generation =
            (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime);
        assert_ne!(generation, 0);
        generation
    };
    let nonce = [0x66; 16];
    let path = std::env::temp_dir().join(format!("handshake-diag-signed-{session}.ring"));
    let _ring_guard = RingGuard(path.clone());
    let writer = DiagRingWriterV2::create(&path, DEFAULT_CAPACITY, nonce).unwrap();
    writer.write_heartbeat(1, 2);
    let _survivor = SurvivorGuard(survivor_path(&path, &session.to_string()));
    struct OwnedWatcher(Child);
    impl Drop for OwnedWatcher {
        fn drop(&mut self) {
            kill_child(&mut self.0);
        }
    }
    let spawn = |suffix: &str| {
        let socket = format!("hsk-palm-signed-{session}-{suffix}");
        let mut command = Command::new(palmistry_bin());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let child = command
            .args([
                "--parent-pid",
                &std::process::id().to_string(),
                "--session-id",
                &session.to_string(),
                "--ring-path",
                path.to_str().unwrap(),
                "--control-socket",
                &socket,
                "--ring-version",
                "2",
                "--ring-nonce",
                "66666666666666666666666666666666",
                "--signing-process-instance",
                &instance.to_string(),
                "--runtime-instance-schema-id",
                palmistry::cli::RUNTIME_INSTANCE_SCHEMA_ID,
                "--runtime-instance-id",
                &session.to_string(),
                "--runtime-host-scope-id",
                &native_host_scope,
                "--runtime-lease-protocol",
                palmistry::cli::RUNTIME_LEASE_PROTOCOL,
                "--runtime-lease-address",
                &native_endpoint.ip().to_string(),
                "--runtime-lease-port",
                &native_endpoint.port().to_string(),
                "--parent-os-creation-time-100ns",
                &parent_os_creation_time_100ns.to_string(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        (OwnedWatcher(child), socket)
    };
    let seed = zeroize::Zeroizing::new(vec![0x77; 32]);
    let (mut watcher, socket) = spawn("valid");
    let mut stdin = watcher.0.stdin.take().unwrap();
    stdin.write_all(&seed).unwrap();
    drop(stdin);
    let identity = WatcherSigningIdentity {
        protocol_version: 1,
        session_id: session,
        process_instance_id: instance,
        parent_pid: std::process::id(),
        watcher_pid: watcher.0.id(),
        ring_version: 2,
        record_size: 120,
        launch_nonce: nonce,
        runtime_instance_schema_id: palmistry::cli::RUNTIME_INSTANCE_SCHEMA_ID.into(),
        runtime_instance_id: session,
        runtime_host_scope_id: native_host_scope.clone(),
        runtime_lease_protocol: palmistry::cli::RUNTIME_LEASE_PROTOCOL.into(),
        runtime_lease_address: native_endpoint.ip(),
        runtime_lease_port: native_endpoint.port(),
        parent_os_creation_time_100ns,
    };
    let request = ControlMessage::ProvenanceChallengeV1 {
        identity: identity.clone(),
        challenge: [8; 32],
        transport_public_key: [9; 32],
    };
    let response = send_v2_control_with_complete_deadline(
        &socket,
        &serde_json::to_string(&request).unwrap(),
        Duration::from_secs(3),
    )
    .unwrap();
    let ControlReply::ProvenanceReadyV1 {
        identity: signed_identity,
        challenge,
        transport_public_key,
        public_verifier,
        signature,
    } = serde_json::from_str(&response).unwrap()
    else {
        panic!("typed signed readiness required");
    };
    assert_eq!(signed_identity, identity);
    assert_eq!(challenge, [8; 32]);
    assert_eq!(transport_public_key, [9; 32]);
    assert_eq!(
        public_verifier,
        ed25519_dalek::SigningKey::from_bytes(&<[u8; 32]>::try_from(seed.as_slice()).unwrap())
            .verifying_key()
            .to_bytes()
    );
    let verifier = ed25519_dalek::VerifyingKey::from_bytes(&public_verifier).unwrap();
    let signature = ed25519_dalek::Signature::from_slice(&signature).unwrap();
    verifier
        .verify_strict(
            &watcher_challenge_bytes(&identity, challenge, transport_public_key).unwrap(),
            &signature,
        )
        .unwrap();
    assert!(verifier
        .verify_strict(
            &watcher_challenge_bytes(&identity, [10; 32], transport_public_key).unwrap(),
            &signature
        )
        .is_err());
    let mut foreign = identity.clone();
    foreign.watcher_pid += 1;
    let forged = ControlMessage::ProvenanceChallengeV1 {
        identity: foreign,
        challenge: [8; 32],
        transport_public_key: [9; 32],
    };
    let denied = send_v2_control_with_complete_deadline(
        &socket,
        &serde_json::to_string(&forged).unwrap(),
        Duration::from_secs(3),
    )
    .unwrap();
    assert!(matches!(
        serde_json::from_str::<ControlReply>(&denied).unwrap(),
        ControlReply::Error {
            code: palmistry::control::ControlErrorCode::SigningIdentityMismatch
        }
    ));
    for attempt in 0..64 {
        let mut rejected = serde_json::to_value(&request).unwrap();
        if attempt % 2 == 0 {
            rejected["identity"]
                .as_object_mut()
                .unwrap()
                .remove("parent_os_creation_time_100ns");
        } else {
            rejected["identity"]["unknown_signed_field"] = serde_json::json!(1);
        }
        assert!(
            send_v2_control_with_complete_deadline(
                &socket,
                &serde_json::to_string(&rejected).unwrap(),
                Duration::from_secs(3),
            )
            .is_err(),
            "rejected signed frame {attempt} must close without a reply"
        );
    }
    assert!(send_v2_control_with_complete_deadline(
        &socket,
        r#"{"type":"Ping"}"#,
        Duration::from_secs(3)
    )
    .unwrap()
    .contains("Pong"));
    assert!(still_running(&mut watcher.0));
    assert!(send_v2_control_with_complete_deadline(
        &socket,
        r#"{"type":"Shutdown"}"#,
        Duration::from_secs(3)
    )
    .unwrap()
    .contains("Ack"));
    assert_eq!(
        wait_for_exit(&mut watcher.0, Duration::from_secs(5)),
        Some(0)
    );
    for size in [31, 33] {
        let (mut invalid, _) = spawn(&format!("invalid-{size}"));
        let mut stdin = invalid.0.stdin.take().unwrap();
        let invalid_seed = zeroize::Zeroizing::new(vec![0x77; size]);
        stdin.write_all(&invalid_seed).unwrap();
        drop(stdin);
        assert_eq!(
            wait_for_exit(&mut invalid.0, Duration::from_secs(5)),
            Some(2)
        );
    }
    let (mut quiet, _) = spawn("quiet");
    let held_pipe = quiet.0.stdin.take().unwrap();
    assert_eq!(wait_for_exit(&mut quiet.0, Duration::from_secs(5)), Some(2));
    drop(held_pipe);
    drop(writer);
    assert!(std::net::UdpSocket::bind(native_endpoint).is_err());
    drop(native_lease);
}
