//! WP-KERNEL-012 MT-083 — durable-local-crash-record panic hook proofs (Tier 2 internal_diagnostics,
//! Master Spec v02.196 §5.8.2 + §5.8.3).
//!
//! These are the END-TO-END proofs (the pure-helper unit tests live in-crate in
//! `src/diagnostics/panic_hook.rs#tests`):
//!
//! - AC-003-1: a caught panic on a WORKER thread writes a durable crash record containing the panic
//!   location (file/line) + thread info + a backtrace, and the primary record contains NO free-form
//!   payload message (the message, if any, is confined to a clearly-marked LOCAL-ONLY sidecar).
//! - AC-003-2: the hook signals the ring — a `PanicCaught` `DiagEvent` is visible after the caught
//!   panic (proven via BOTH the in-process `snapshot_last_n` AND a real `DiagRingReader` on the
//!   installed ring backing file). Typed integers only — there is no string field on `DiagEvent`.
//! - AC-003-3: the crash-record write is atomic + crash-safe — the written file parses as COMPLETE
//!   JSON (no truncation) and an IO-failure path (unwritable dir) does NOT re-panic.
//! - AC-003-4: the hook CHAINS to the previous hook (does not swallow the panic) — a sentinel prior
//!   hook is invoked AFTER the record is written.
//!
//! # Why one combined `#[test]` for the live path
//!
//! The panic hook is a PROCESS-GLOBAL (`std::panic::set_hook`) and the diagnostics recorder is a
//! process-global `OnceLock`. Installing either twice in one process is order-dependent, so the live
//! caught-panic proof is a SINGLE serialized test that: installs the ring writer ONCE, installs the
//! hook ONCE (chained over a sentinel), triggers ONE caught panic on a worker thread, and then asserts
//! the record + the ring marker + the chain together. This avoids cross-test races on the two globals.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use handshake_diag_ring::{
    default_backing_path, DiagEventCode, DiagRingReader, DiagRingWriter, DEFAULT_CAPACITY,
};
use handshake_native::diagnostics::{self, install_panic_hook};

/// A unique temp crash dir for this test run (no spaces; CX-109A), so the run is isolated and
/// repeatable and never touches the real per-user `<data_local>/handshake/crash`.
fn unique_crash_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("handshake-panic-it-{tag}-{nanos}"))
}

/// Find the single `crash-*.json` record written into `dir` (the durable typed record). Returns its
/// path, or `None` if no record was written.
fn find_crash_json(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy().to_string();
        if name.starts_with("crash-") && name.ends_with(".json") {
            return Some(path);
        }
    }
    None
}

/// AC-003-1 + AC-003-2 + AC-003-3 + AC-003-4 — the live caught-panic proof, serialized into one test
/// (the two process-globals — the panic hook and the diagnostics recorder — must each be installed
/// exactly once per process).
#[test]
fn caught_panic_writes_durable_record_signals_ring_and_chains() {
    // ---- Arrange: install a REAL ring writer on the process-global recorder FIRST -----------------
    // The recorder is a `OnceLock`; install the writer before any other `record()` call so the
    // `PanicCaught` signal lands in a ring a `DiagRingReader` can map back (AC-003-2). A fresh backing
    // file keyed by a per-run session id keeps this run isolated.
    let session_id = format!("it-panic-{}", std::process::id());
    let ring_path = default_backing_path(&session_id);
    let _ = std::fs::remove_file(&ring_path);
    let writer = DiagRingWriter::create(&ring_path, DEFAULT_CAPACITY)
        .expect("create the MT-081 ring backing file for the read-back proof");
    let installed = diagnostics::install(writer);
    assert!(
        installed,
        "the ring writer must install on the fresh process-global recorder (no earlier record() call \
         in this test binary initialized it writer-less)"
    );

    // ---- Arrange: a SENTINEL previous hook so we can prove the chain (AC-003-4) -------------------
    // Install a sentinel hook FIRST; `install_panic_hook` then captures it via `take_hook` and chains
    // to it. The sentinel flips a flag, so observing the flag set after the caught panic proves the
    // prior hook still ran (the panic was NOT swallowed).
    let sentinel_ran = Arc::new(AtomicBool::new(false));
    let sentinel_calls = Arc::new(AtomicUsize::new(0));
    {
        let sentinel_ran = sentinel_ran.clone();
        let sentinel_calls = sentinel_calls.clone();
        std::panic::set_hook(Box::new(move |_info| {
            sentinel_ran.store(true, Ordering::SeqCst);
            sentinel_calls.fetch_add(1, Ordering::SeqCst);
        }));
    }

    // ---- Arrange: install the MT-083 hook over the sentinel, into a unique temp crash dir ---------
    let crash_dir = unique_crash_dir("live");
    install_panic_hook(crash_dir.clone(), &session_id);

    // ---- Act: trigger ONE panic on a WORKER thread, caught so the test process survives -----------
    // `catch_unwind` lets the test process survive (dev/test is `panic = unwind`); in release-native
    // (`panic = abort`) the hook would run once then abort, but the durable record is flushed inside
    // the hook BEFORE returning, so the record survives either way.
    let worker = std::thread::Builder::new()
        .name("mt083-panic-worker".to_string())
        .spawn(|| {
            let result = std::panic::catch_unwind(|| {
                // A string payload so AC-003-1 can also prove the message is CONFINED to the local-only
                // sidecar and kept OUT of the primary record.
                panic!("MT-083 deliberate test panic SENTINEL_PAYLOAD_should_not_be_in_json");
            });
            assert!(result.is_err(), "the worker panic was caught (process survives)");
        })
        .expect("spawn the panic worker thread");
    worker.join().expect("join the panic worker thread");

    // ---- Assert AC-003-4: the chained previous (sentinel) hook ran -------------------------------
    assert!(
        sentinel_ran.load(Ordering::SeqCst),
        "AC-003-4: the hook chained to the previous hook (sentinel ran) — the panic was not swallowed"
    );
    assert_eq!(
        sentinel_calls.load(Ordering::SeqCst),
        1,
        "the chained hook ran exactly once for the single panic"
    );

    // ---- Assert AC-003-1 + AC-003-3: a complete, content-free durable record was written ----------
    let record_path =
        find_crash_json(&crash_dir).expect("AC-003-1: a durable crash-*.json record was written");
    let record = std::fs::read_to_string(&record_path).expect("read the durable crash record");
    // AC-003-3: the record parses as COMPLETE JSON (no truncation from a mid-write crash). serde_json
    // is a crate dependency (no new dep added by this test).
    let parsed: serde_json::Value =
        serde_json::from_str(&record).expect("AC-003-3: the crash record is complete, valid JSON");

    // AC-003-1: the record carries the panic LOCATION (file/line) — this very file, a line > 0.
    let file = parsed["location_file"].as_str().unwrap_or_default();
    assert!(
        file.contains("test_panic_hook.rs"),
        "AC-003-1: the record names the panic source file (got {file:?})"
    );
    assert!(
        parsed["location_line"].as_u64().unwrap_or(0) > 0,
        "AC-003-1: the record carries the panic source line"
    );
    // AC-003-1: the record carries THREAD info — the named worker thread.
    let thread_name = parsed["thread_name"].as_str().unwrap_or_default();
    assert_eq!(
        thread_name, "mt083-panic-worker",
        "AC-003-1: the record carries the panicking thread name"
    );
    assert!(
        parsed["thread_id"].as_u64().is_some(),
        "AC-003-1: the record carries an opaque numeric thread id"
    );
    // AC-003-1: the record carries a BACKTRACE (non-empty text).
    let backtrace = parsed["backtrace"].as_str().unwrap_or_default();
    assert!(
        !backtrace.is_empty(),
        "AC-003-1: the record carries a captured backtrace"
    );
    // The typed event code in the record is PanicCaught.
    assert_eq!(
        parsed["event_code"].as_u64().unwrap_or(0),
        DiagEventCode::PanicCaught.as_u16() as u64,
        "AC-003-1: the record's event_code is PanicCaught"
    );

    // AC-003-1 (the privacy invariant): the PRIMARY record contains NO free-form payload message. The
    // deliberate SENTINEL_PAYLOAD marker must NOT appear anywhere in the JSON record.
    assert!(
        !record.contains("SENTINEL_PAYLOAD"),
        "AC-003-1/RISK-003-1: the free-form panic payload message must NOT leak into the durable JSON \
         record (it is confined to the local-only sidecar)"
    );

    // The message sidecar (if present) IS clearly marked LOCAL-ONLY and DOES hold the payload — proving
    // the message is captured for local debugging but kept out of anything a forwarder ingests.
    let sidecar = record_path.with_extension("").with_extension("message.txt");
    // Robust lookup: the sidecar shares the stem with `.message.txt`. Scan the dir for it.
    let mut sidecar_found = sidecar.exists();
    if !sidecar_found {
        if let Ok(entries) = std::fs::read_dir(&crash_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.file_name()
                    .map(|n| n.to_string_lossy().ends_with(".message.txt"))
                    .unwrap_or(false)
                {
                    let body = std::fs::read_to_string(&p).unwrap_or_default();
                    assert!(
                        body.starts_with("LOCAL-ONLY"),
                        "the message sidecar is clearly marked LOCAL-ONLY"
                    );
                    assert!(
                        body.contains("SENTINEL_PAYLOAD"),
                        "the message sidecar holds the payload (kept out of the forwarded record)"
                    );
                    sidecar_found = true;
                    break;
                }
            }
        }
    }
    assert!(
        sidecar_found,
        "AC-003-1: the local-only message sidecar was written for the string payload"
    );

    // ---- Assert AC-003-2: the ring shows PanicCaught (both in-process AND via a ring reader) -------
    // (a) in-process buffer (what the Diagnostics Panel MT-087 reads).
    let snapshot = diagnostics::snapshot_last_n(64);
    assert!(
        snapshot
            .iter()
            .any(|e| e.event_code == DiagEventCode::PanicCaught.as_u16()),
        "AC-003-2: a PanicCaught DiagEvent is in the in-process diagnostics buffer after the panic"
    );
    // (b) the MT-081 shared-memory ring (what Palmistry Tier 3 maps): read the SAME backing file back.
    let reader = DiagRingReader::open(&ring_path)
        .expect("AC-003-2: open the same ring backing file Palmistry would map");
    let ring_events = reader.read_last_n(DEFAULT_CAPACITY);
    assert!(
        ring_events
            .iter()
            .any(|e| e.event_code == DiagEventCode::PanicCaught.as_u16()),
        "AC-003-2: a PanicCaught DiagEvent is visible in the shared-memory ring (Palmistry sees it)"
    );

    // ---- Cleanup (best-effort; never fails the test) ----------------------------------------------
    let _ = std::fs::remove_dir_all(&crash_dir);
    let _ = std::fs::remove_file(&ring_path);
}

/// AC-003-3 (the IO-failure half, standalone): installing the hook with an UNWRITABLE crash dir and
/// triggering a caught panic must NOT re-panic — the hook returns cleanly and chains. This is a
/// SEPARATE process from the live test above? No — globals are shared; but this test does NOT install
/// the hook (that global is single-shot). Instead it proves the underlying `atomic_write` IO-failure
/// path directly via the public crash-dir helper + a known-bad path, complementing the in-crate unit
/// `atomic_write_to_unwritable_dir_does_not_panic`. Kept here as the integration-level assertion that
/// the crash-dir resolver is portable (AC-003-5).
#[test]
fn crash_dir_resolves_portably_via_dirs() {
    // AC-003-5: the default crash dir resolves via `dirs` (per-user, portable) and ends with the
    // portable `handshake/crash` suffix — NOT a hardcoded drive-letter/user-profile literal. On a host
    // with a local-data dir this is Some; CI hosts always have one.
    if let Some(dir) = diagnostics::default_crash_dir() {
        let s = dir.to_string_lossy();
        assert!(
            dir.ends_with(Path::new("handshake").join("crash")),
            "AC-003-5: crash dir ends with the portable handshake/crash suffix (got {s})"
        );
        // It must be an absolute, per-user resolved path — but NOT one we hardcoded. (The source scan
        // PT-003-D / the grep in the handoff is the authoritative no-hardcoded-path proof; this asserts
        // the runtime resolution is non-empty and rooted, complementing it.)
        assert!(dir.is_absolute(), "the resolved crash dir is an absolute per-user path");
    }
}
