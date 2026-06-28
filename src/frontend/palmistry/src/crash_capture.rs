//! Palmistry CRASH CAPTURE + OUT-OF-PROCESS MINIDUMP (MT-092, Master Spec v02.196 §6.13.6).
//!
//! This is the CRASH half of Palmistry's job (the FREEZE half is MT-091). A crash is detected two ways:
//!
//! 1. **CrashContext-driven (the RICH case)** — Handshake's in-process crash-handler catches an
//!    unhandled OS exception (Windows SEH) and sends its [`crash_context::CrashContext`] (exception
//!    pointers + faulting thread/process id) to Palmistry over the Embark `minidumper` IPC. Palmistry,
//!    the SERVER, then writes a full MINIDUMP **out-of-process** via `minidump-writer`: it reads the
//!    crashing client's memory from OUTSIDE the dying process. §6.13.6: *"the crashing process cannot
//!    reliably dump itself; the external writer captures the dump from outside."*
//!
//! 2. **Process-handle-wait (the FLOOR case)** — MT-089's parent-handle wait observes an UNEXPECTED
//!    parent exit (no prior `Shutdown`). A hard kill (TerminateProcess) delivers NO CrashContext and no
//!    exception, so a full minidump is impossible post-mortem; Palmistry instead writes a best-effort
//!    typed crash RECORD (exit code + last heartbeat / last-N events read passively from the ring). This
//!    is the floor: a crash is NEVER missed, even when the rich path could not run.
//!
//! # The OUT-OF-PROCESS invariant (HARD, §6.13.6)
//!
//! The minidump is written by the Palmistry SERVER process, NOT by the crashing Handshake client. The
//! Embark pipeline is: `crash-handler` (client exception handler) -> `crash-context` (the typed context
//! carried across the process boundary) -> `minidumper` (client signals server) -> `minidump-writer`
//! (the server writes the dump, reading the client's memory cross-process). A self-dump (in-process)
//! would be unreliable precisely because the process is crashing (RISK-012-1). The client side lives in
//! `handshake-native` main.rs (MT-092 wiring); THIS module is the server/writer side.
//!
//! # CLEAN SHUTDOWN IS NOT A CRASH (HARD, §6.13)
//!
//! A `Shutdown` received before the parent exit produces NO crash record and NO minidump (RISK-012-2).
//! That distinction is owned by [`crate::lifecycle`] (the `abnormal_parent_exit` flag, classified at the
//! instant of death against the shutdown flag). The post-mortem record path here writes ONLY when the
//! lifecycle says the exit was abnormal; a `CleanShutdown` / clean `ParentExit` never reaches it.
//!
//! # LOCAL-ONLY + TYPED ALLOWLIST (HARD, §6.13.8)
//!
//! - The MINIDUMP is written to a LOCAL file path and NEVER uploaded. There is no network call anywhere
//!   in this module (RISK-012-4). Any future upload is consent-gated and out of scope here.
//! - The crash RECORD metadata ([`CrashRecord`]) is a TYPED ALLOWLIST: exit code, crash event code,
//!   thread/process ids, timestamps, and the local minidump file path. There is deliberately NO
//!   free-text / project-content field (RISK-012-6). The minidump binary itself contains thread
//!   stacks/registers/loaded modules (the OS process image — field-standard crash data), which §6.13.8
//!   classes as local-only by default.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use handshake_diag_ring::{DiagEvent, DiagEventCode, Heartbeat};

/// How a crash was detected — which of the two §6.13.6 paths produced the record. Typed so the survivor
/// store (MT-093) and a reviewer can tell a RICH (minidump-bearing) crash from a FLOOR (post-mortem-only)
/// crash without parsing prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "detection")]
pub enum CrashDetection {
    /// A `CrashContext` arrived over the IPC and a full minidump was written out-of-process (the rich
    /// case). The [`CrashRecord::minidump_path`] names the local dump file.
    CrashContextMinidump,
    /// The parent exited abnormally (hard kill / abrupt exit) with NO `CrashContext` — a best-effort
    /// post-mortem record only (the floor case). No minidump; the last heartbeat + events are the
    /// evidence. A full minidump would have needed the client's cooperation (the CrashContext path).
    PostMortemNoContext,
}

/// The TYPED-ALLOWLIST crash record Palmistry persists (§6.13.8). Every field is a fixed-width integer,
/// a typed enum, or a local file path — there is NO free-text / project-content field, so it cannot
/// become a content-smuggling channel (RISK-012-6). MT-093 reads this and forwards the typed subset to
/// the Tier-1 Flight Recorder at recovery (the FR forward is MT-093, not here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashRecord {
    /// The diagnostic session id (an opaque correlation token — NOT content).
    pub session_id: String,
    /// How the crash was detected (rich minidump vs floor post-mortem).
    pub detection: CrashDetection,
    /// The typed crash event code (`DiagEventCode::CrashDetected` as a u16), so the record's kind is the
    /// SAME shared numeric code space the ring uses.
    pub crash_event_code: u16,
    /// The watched parent process id.
    pub process_id: u32,
    /// The faulting thread id, if a `CrashContext` carried one (0 when unknown / post-mortem).
    pub faulting_thread_id: u64,
    /// The parent OS exit code, if the handle resolved one (post-mortem path); `None` for a
    /// CrashContext-driven dump (the process had not exited yet when the dump was taken).
    pub exit_code: Option<u32>,
    /// The last heartbeat COUNTER read passively from the ring before death (0 if none). A number, not
    /// content.
    pub last_heartbeat_counter: u64,
    /// The last heartbeat TIMESTAMP (nanos) read passively from the ring before death (0 if none).
    pub last_heartbeat_ts_nanos: u64,
    /// How many of the ring's last-N typed events were bundled as crash evidence (a count — the events
    /// themselves are POD integers, never text).
    pub last_event_count: u32,
    /// The LOCAL path of the out-of-process minidump file, if one was written (rich case). `None` for a
    /// post-mortem record. NEVER a URL — local filesystem only (§6.13.8 local-only).
    pub minidump_path: Option<PathBuf>,
    /// Wall-clock millis since the UNIX epoch when the record was written (a numeric timestamp).
    pub recorded_at_unix_ms: u128,
}

impl CrashRecord {
    /// Build a post-mortem (FLOOR) crash record from the abnormal-exit facts + the last ring readings.
    /// No minidump (the hard-kill path cannot produce one without the client's cooperation).
    pub fn post_mortem(
        session_id: &str,
        process_id: u32,
        exit_code: Option<u32>,
        last_heartbeat: Option<Heartbeat>,
        last_events: &[DiagEvent],
    ) -> Self {
        let (hb_counter, hb_ts) = match last_heartbeat {
            Some(hb) => (hb.counter, hb.timestamp_nanos),
            None => (0, 0),
        };
        Self {
            session_id: session_id.to_string(),
            detection: CrashDetection::PostMortemNoContext,
            crash_event_code: DiagEventCode::CrashDetected.as_u16(),
            process_id,
            faulting_thread_id: 0,
            exit_code,
            last_heartbeat_counter: hb_counter,
            last_heartbeat_ts_nanos: hb_ts,
            last_event_count: last_events.len() as u32,
            minidump_path: None,
            recorded_at_unix_ms: now_unix_ms(),
        }
    }

    /// Build a rich (CrashContext-driven) crash record naming the out-of-process minidump file.
    pub fn with_minidump(
        session_id: &str,
        process_id: u32,
        faulting_thread_id: u64,
        minidump_path: PathBuf,
        last_heartbeat: Option<Heartbeat>,
        last_events: &[DiagEvent],
    ) -> Self {
        let (hb_counter, hb_ts) = match last_heartbeat {
            Some(hb) => (hb.counter, hb.timestamp_nanos),
            None => (0, 0),
        };
        Self {
            session_id: session_id.to_string(),
            detection: CrashDetection::CrashContextMinidump,
            crash_event_code: DiagEventCode::CrashDetected.as_u16(),
            process_id,
            faulting_thread_id,
            exit_code: None,
            last_heartbeat_counter: hb_counter,
            last_heartbeat_ts_nanos: hb_ts,
            last_event_count: last_events.len() as u32,
            minidump_path: Some(minidump_path),
            recorded_at_unix_ms: now_unix_ms(),
        }
    }
}

/// Wall-clock millis since the UNIX epoch (best-effort; 0 on a clock error). A numeric timestamp, not
/// content.
fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Sanitize a session id into a filename-safe token (same rule as the MT-089 survivor record): keep
/// ASCII alphanumerics, `-`, `_`; map everything else to `_`. Keeps the minidump/record filenames stable
/// and space-free (CX-109A) regardless of the session id's source.
pub fn safe_session_token(session_id: &str) -> String {
    session_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The LOCAL minidump file path for a session, written as a sibling of the ring file:
/// `<ring_dir>/palmistry-crash-<session>.dmp`. A `.dmp` is the field-standard minidump extension the
/// `minidump` reader expects. LOCAL filesystem only — never a URL (§6.13.8).
pub fn minidump_path_for(ring_path: &Path, session_id: &str) -> PathBuf {
    let dir = ring_path.parent().unwrap_or_else(|| Path::new("."));
    dir.join(format!("palmistry-crash-{}.dmp", safe_session_token(session_id)))
}

/// The LOCAL crash-record JSON path for a session, a sibling of the ring file:
/// `<ring_dir>/palmistry-crash-<session>.json`. The TYPED-ALLOWLIST metadata MT-093 reads.
pub fn crash_record_path_for(ring_path: &Path, session_id: &str) -> PathBuf {
    let dir = ring_path.parent().unwrap_or_else(|| Path::new("."));
    dir.join(format!("palmistry-crash-{}.json", safe_session_token(session_id)))
}

/// Persist a [`CrashRecord`] as pretty JSON to its LOCAL sibling path. Best-effort durable evidence; the
/// caller treats an IO error as non-fatal (the lifecycle still ended correctly). NO network — a plain
/// local file write (§6.13.8).
pub fn persist_crash_record(ring_path: &Path, record: &CrashRecord) -> std::io::Result<PathBuf> {
    let out = crash_record_path_for(ring_path, &record.session_id);
    let json = serde_json::to_string_pretty(record).map_err(std::io::Error::other)?;
    std::fs::write(&out, json)?;
    Ok(out)
}

// ----------------------------------------------------------------------------------------------------
// The minidumper SERVER handler (the OUT-OF-PROCESS dump writer).
// ----------------------------------------------------------------------------------------------------

/// The Palmistry-side `minidumper::ServerHandler`. When the CLIENT (Handshake) signals a crash with a
/// `CrashContext`, the `minidumper::Server` calls [`create_minidump_file`] for a backing file, writes the
/// dump OUT-OF-PROCESS into it (reading the crashing client's memory cross-process via `minidump-writer`),
/// then calls [`on_minidump_created`] so we record the result. The handler holds the LOCAL output path and
/// a shared latch the run loop reads to learn a dump was captured.
///
/// [`create_minidump_file`]: minidumper::ServerHandler::create_minidump_file
/// [`on_minidump_created`]: minidumper::ServerHandler::on_minidump_created
pub struct CrashServerHandler {
    /// The LOCAL path the out-of-process minidump is written to.
    minidump_path: PathBuf,
    /// Latched true once a minidump has been fully written (the run loop / a test reads this).
    captured: Arc<AtomicBool>,
    /// The faulting thread id the client reported via a user message (so the typed record can name it).
    /// `minidumper` does not surface the CrashContext thread id to `on_minidump_created`, so the client
    /// sends it as a user message that lands in [`on_message`](minidumper::ServerHandler::on_message)
    /// BEFORE the dump request. 0 until one arrives.
    faulting_thread_id: Arc<AtomicU64>,
    /// Whether the last dump write FAILED (so the server can surface it without a panic). The
    /// `on_minidump_created` callback runs on the server loop; a panic there would abort the watcher.
    write_error: Arc<Mutex<Option<String>>>,
}

/// The user-message kind the CLIENT uses to send the faulting thread id ahead of the crash request, so
/// the typed [`CrashRecord`] can name the thread (minidumper does not surface it to the handler). A small
/// fixed `u32` kind — carries only an 8-byte little-endian thread id, never text.
pub const MSG_KIND_FAULTING_THREAD_ID: u32 = 1;

impl CrashServerHandler {
    /// Create a handler that writes the minidump to `minidump_path` (a LOCAL sibling of the ring) and
    /// latches `captured` when a dump is written.
    pub fn new(minidump_path: PathBuf) -> Self {
        Self {
            minidump_path,
            captured: Arc::new(AtomicBool::new(false)),
            faulting_thread_id: Arc::new(AtomicU64::new(0)),
            write_error: Arc::new(Mutex::new(None)),
        }
    }

    /// A handle the caller keeps to observe whether a minidump was captured (latched true on success).
    pub fn captured_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.captured)
    }

    /// A handle to the faulting thread id the client reported (0 until a crash with a reported id).
    pub fn faulting_thread_id_handle(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.faulting_thread_id)
    }

    /// A handle to the last write error (so the caller can surface a dump-write failure).
    pub fn write_error_handle(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.write_error)
    }
}

impl minidumper::ServerHandler for CrashServerHandler {
    fn create_minidump_file(&self) -> Result<(File, PathBuf), std::io::Error> {
        // Ensure the parent dir exists (the ring dir normally does, but be robust). LOCAL filesystem.
        if let Some(dir) = self.minidump_path.parent() {
            if !dir.as_os_str().is_empty() && !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
        }
        let file = File::create(&self.minidump_path)?;
        Ok((file, self.minidump_path.clone()))
    }

    fn on_minidump_created(
        &self,
        result: Result<minidumper::MinidumpBinary, minidumper::Error>,
    ) -> minidumper::LoopAction {
        match result {
            Ok(mut bin) => {
                // Flush the dump to disk so a reader sees a complete file (the dump is already written
                // out-of-process by minidump-writer; this just flushes the OS buffer). Best-effort.
                let _ = bin.file.flush();
                self.captured.store(true, Ordering::SeqCst);
            }
            Err(err) => {
                // Record the failure WITHOUT panicking (a panic on the server loop aborts the watcher).
                if let Ok(mut slot) = self.write_error.lock() {
                    *slot = Some(format!("minidump write failed: {err}"));
                }
            }
        }
        // One crash per watched session is the contract; exit the server loop after the first dump so the
        // run thread can join and the watcher proceeds to record + finalize.
        minidumper::LoopAction::Exit
    }

    fn on_message(&self, kind: u32, buffer: Vec<u8>) {
        // The client reports the faulting thread id ahead of the crash request (minidumper does not
        // surface the CrashContext thread id to the handler). Typed: an 8-byte LE u64, never text.
        if kind == MSG_KIND_FAULTING_THREAD_ID && buffer.len() == 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&buffer[..8]);
            self.faulting_thread_id
                .store(u64::from_le_bytes(bytes), Ordering::SeqCst);
        }
    }
}

/// Default `stale_timeout` for the crash server's accept loop. `minidumper::Server::run` uses this to
/// reap a client connection that has gone silent (e.g. the client crashed/exited without a clean
/// disconnect), so the server loop does not block forever waiting on a dead peer. The client sends a
/// ping/message frequently enough that a live client never trips it.
pub const CRASH_SERVER_STALE_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_token_strips_unsafe_chars_and_spaces() {
        // CX-109A: no spaces; only [A-Za-z0-9_-] survive.
        assert_eq!(safe_session_token("ab-CD_12"), "ab-CD_12");
        assert_eq!(safe_session_token("a b/c:d"), "a_b_c_d");
        assert_eq!(safe_session_token("session.id!"), "session_id_");
    }

    #[test]
    fn minidump_and_record_paths_are_local_siblings_of_the_ring() {
        let ring = Path::new("/tmp/diag/handshake-diag-sess.ring");
        let dmp = minidump_path_for(ring, "sess");
        let json = crash_record_path_for(ring, "sess");
        // Both land in the ring's LOCAL directory (not a URL, not elsewhere) — §6.13.8 local-only.
        assert_eq!(dmp, Path::new("/tmp/diag/palmistry-crash-sess.dmp"));
        assert_eq!(json, Path::new("/tmp/diag/palmistry-crash-sess.json"));
        // The dump uses the field-standard .dmp extension the `minidump` reader expects.
        assert_eq!(dmp.extension().and_then(|e| e.to_str()), Some("dmp"));
    }

    #[test]
    fn post_mortem_record_is_typed_allowlist_only_no_minidump() {
        // The FLOOR case (hard kill, no CrashContext): a best-effort record, no minidump, typed fields.
        let hb = Heartbeat {
            counter: 42,
            timestamp_nanos: 123_456,
        };
        let rec = CrashRecord::post_mortem("sess-x", 4242, Some(0xDEAD), Some(hb), &[]);
        assert_eq!(rec.detection, CrashDetection::PostMortemNoContext);
        assert_eq!(rec.process_id, 4242);
        assert_eq!(rec.exit_code, Some(0xDEAD));
        assert_eq!(rec.last_heartbeat_counter, 42);
        assert_eq!(rec.last_heartbeat_ts_nanos, 123_456);
        assert_eq!(rec.faulting_thread_id, 0, "no CrashContext => no thread id");
        assert!(rec.minidump_path.is_none(), "the floor case writes NO minidump");
        assert_eq!(rec.crash_event_code, DiagEventCode::CrashDetected.as_u16());

        // Serialize and assert the JSON carries NO free-text/project-content key — only the typed
        // allowlist (RISK-012-6). The only string-valued keys allowed are the opaque session token + the
        // typed `detection` tag.
        let json = serde_json::to_value(&rec).unwrap();
        let obj = json.as_object().unwrap();
        let allowed_keys = [
            "session_id",
            "detection",
            "crash_event_code",
            "process_id",
            "faulting_thread_id",
            "exit_code",
            "last_heartbeat_counter",
            "last_heartbeat_ts_nanos",
            "last_event_count",
            "minidump_path",
            "recorded_at_unix_ms",
        ];
        for key in obj.keys() {
            assert!(
                allowed_keys.contains(&key.as_str()),
                "crash record carried a non-allowlisted key '{key}' (typed-allowlist breach, RISK-012-6)"
            );
        }
    }

    #[test]
    fn with_minidump_record_names_the_local_dump_and_thread() {
        let path = PathBuf::from("/tmp/diag/palmistry-crash-sess.dmp");
        let rec = CrashRecord::with_minidump("sess", 7, 99, path.clone(), None, &[]);
        assert_eq!(rec.detection, CrashDetection::CrashContextMinidump);
        assert_eq!(rec.faulting_thread_id, 99);
        assert_eq!(rec.minidump_path.as_deref(), Some(path.as_path()));
        assert!(rec.exit_code.is_none(), "the process had not exited when dumped");
    }

    #[test]
    fn persist_writes_local_json_sibling() {
        let dir = std::env::temp_dir().join(format!("hsk-crash-persist-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ring = dir.join("handshake-diag-persist.ring");
        let rec = CrashRecord::post_mortem("persist-sess", 1, Some(1), None, &[]);
        let out = persist_crash_record(&ring, &rec).unwrap();
        assert!(out.exists(), "crash record JSON must be written locally");
        let back: CrashRecord =
            serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
        assert_eq!(back, rec, "round-trips through the local JSON");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The SHIPPED `CrashServerHandler` drives a REAL minidumper round-trip end-to-end (MT-092 hardening,
    /// must-fix #2). The previous suite only exercised INLINE reimplementations of the handler (struct `H`
    /// in tests/test_crash_capture.rs, `SelfTestHandler` in handshake-native), so the production handler's
    /// `create_minidump_file` / `on_minidump_created` / `on_message` (the 8-byte LE thread-id decode) and
    /// the `captured` + `faulting_thread_id` latches were never run at runtime. This constructs the REAL
    /// handler, runs `minidumper::Server::run` on it, connects a REAL crash-handler client, sends the typed
    /// thread-id message through the real `on_message`, fires a SIMULATED exception (a real captured
    /// context WITHOUT killing the test process), and asserts the shipped handler latched both a captured
    /// minidump AND the faulting thread id. (This is the in-crate, same-process complement to the
    /// cross-PROCESS binary proof in tests/test_crash_capture.rs::cross_process_*.)
    #[test]
    fn shipped_handler_captures_dump_and_thread_id_via_real_roundtrip() {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        // A short AF_UNIX filesystem-socket path under the OS temp dir (fits the 108-byte sun_path).
        let socket = std::env::temp_dir()
            .join(format!("hsk-shipped-{pid}-{nanos}.sock"))
            .to_string_lossy()
            .into_owned();
        let _ = std::fs::remove_file(&socket); // clear a stale socket from a prior run
        let dump_path = std::env::temp_dir().join(format!("hsk-shipped-{pid}-{nanos}.dmp"));

        // Construct the SHIPPED handler and grab its real latches.
        let handler = CrashServerHandler::new(dump_path.clone());
        let captured = handler.captured_flag();
        let thread_id = handler.faulting_thread_id_handle();

        let shutdown = Arc::new(AtomicBool::new(false));
        let mut server = minidumper::Server::with_name(minidumper::SocketName::path(&socket))
            .expect("bind crash server");
        let server_shutdown = Arc::clone(&shutdown);
        let server_loop = std::thread::spawn(move || {
            // The SHIPPED handler is moved in here — `Server::run` calls its real trait methods.
            let _ = server.run(Box::new(handler), &server_shutdown, Some(Duration::from_secs(5)));
        });

        // A REAL crash-handler client connects and reports the faulting thread id through the real
        // on_message path, then requests the out-of-process dump.
        let client = Arc::new(
            minidumper::Client::with_name(minidumper::SocketName::path(&socket))
                .expect("client connect"),
        );
        // A distinctive non-zero thread id so we can prove the 8-byte LE decode in on_message ran (not a
        // default 0). The value is arbitrary — it is the WIRE proof, not the real OS thread id.
        const PROBE_TID: u64 = 0x00C0_FFEE_1234_5678;
        let client_cb = Arc::clone(&client);
        #[allow(unsafe_code)]
        let attached = crash_handler::CrashHandler::attach(unsafe {
            crash_handler::make_crash_event(move |cc: &crash_handler::CrashContext| {
                let _ = client_cb.send_message(MSG_KIND_FAULTING_THREAD_ID, PROBE_TID.to_le_bytes());
                crash_handler::CrashEventResult::Handled(client_cb.request_dump(cc).is_ok())
            })
        })
        .expect("attach crash handler");

        // FIRE a REAL simulated exception (real captured context) WITHOUT crashing the test process.
        let _ = attached.simulate_exception(None);
        let _ = server_loop.join();
        drop(attached);

        // The SHIPPED handler latched a captured minidump (create_minidump_file + on_minidump_created ran)
        // and decoded the typed thread id through its real on_message (the 8-byte LE path).
        assert!(
            captured.load(Ordering::SeqCst),
            "the shipped CrashServerHandler must latch `captured` after writing the dump"
        );
        assert_eq!(
            thread_id.load(Ordering::SeqCst),
            PROBE_TID,
            "the shipped handler's on_message must decode the typed 8-byte LE faulting thread id"
        );
        assert!(dump_path.exists(), "the shipped handler must write the dump to its local path");
        let bytes = std::fs::read(&dump_path).expect("read dump");
        assert!(bytes.len() > 1024, "a real minidump is non-trivial, got {} bytes", bytes.len());

        let _ = std::fs::remove_file(&dump_path);
        let _ = std::fs::remove_file(&socket);
    }
}
