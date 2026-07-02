//! MT-090 RING-READER PROOFS (the deliverable, §6.13.4 — ZERO COOPERATION + FROZEN WRITER).
//!
//! These tests prove the Palmistry RING READER reads Handshake's heartbeat + last-N diagnostic events
//! from the SAME MT-081 shared-memory ring with ZERO COOPERATION from the writer — including while the
//! writer is artificially STALLED (the frozen-UI-thread case). They exercise the read side that the
//! freeze probe (MT-091) and the survivor store (MT-093) build on.
//!
//! What each test proves, mapped to the MT contract:
//!
//! - **AC-010-1 / PT-010-A** ([`cross_process_writer_reader_reads_heartbeat_and_events`]): a SEPARATE
//!   OS PROCESS (a real "Handshake" writer the test spawns) creates + writes the ring; the Palmistry
//!   reader in THIS process opens the SAME backing file and reads back the heartbeat + last-N events.
//!   This is the genuine cross-process shared-memory read, not a same-process shortcut.
//! - **AC-010-2 / PT-010-B** ([`frozen_writer_reader_still_reads_last_good_state`]): a writer thread
//!   writes a heartbeat + N events, then STALLS (parks/sleeps — a frozen UI thread that stopped
//!   writing). The reader STILL reads the last good heartbeat + last-N events while the writer is
//!   stalled — a frozen writer does NOT block or break the reader. This is the §6.13.4 deliverable.
//! - **AC-010-2 cross-process** ([`frozen_writer_process_still_readable`]): the same frozen-writer proof
//!   across a REAL process boundary — the writer PROCESS writes then sleeps (frozen), and the reader in
//!   this process reads its last good state while that process is stalled and still alive.
//! - **AC-010-5 / PT-010-C** ([`read_is_wait_free_under_a_hot_concurrent_writer`]): a hot concurrent
//!   writer hammers the ring while the reader reads in a tight loop; every read RETURNS within a tight
//!   bound (the MT-081 bounded seqlock retry) — no unbounded block.
//! - **AC-010-4 / PT-010-C** ([`reader_exposes_only_typed_values_no_free_text`]): the reader returns
//!   ONLY typed `Heartbeat` / `DiagEvent` (integers) and a source scan confirms there is no
//!   string-content read API (typed-allowlist on the read side, §6.13.8).
//!
//! Cross-process mechanism: the test binary RE-EXECS ITSELF as the writer child via a gated env var
//! (`MT090_WRITER_MODE`), so no extra fixture binary is needed — the child maps the SAME backing file
//! and writes into it, proving two distinct processes share the ring pages.
//!
//! NOTE on the read side under test: `palmistry` is a BINARY crate (no `[lib]`), so an integration test
//! in `tests/` cannot import `palmistry::ring_reader::PalmistryRingReader` directly (the same constraint
//! `test_lifecycle.rs` works under — it drives the compiled binary + reuses `handshake_diag_ring`). The
//! `PalmistryRingReader` wrapper is a THIN delegate over the MT-081 `DiagRingReader` — its open-retry,
//! reopen, and delegation logic are proven by the in-crate `#[cfg(test)]` unit tests in `ring_reader.rs`
//! (run by `cargo test -p palmistry`). THESE integration tests prove the underlying cross-process,
//! frozen-writer, and wait-free read GUARANTEES the wrapper inherits, against the exact `DiagRingReader`
//! reads the wrapper calls — so the end-to-end behavior the wrapper promises is proven across a real
//! process boundary, not asserted.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use handshake_diag_ring::ring::DEFAULT_CAPACITY;
use handshake_diag_ring::{DiagEvent, DiagRingReader, DiagRingWriter};

// ---------------------------------------------------------------------------------------------------
// Cross-process writer child (re-exec of THIS test binary, gated by an env var).
// ---------------------------------------------------------------------------------------------------

const WRITER_MODE_ENV: &str = "MT090_WRITER_MODE";
const WRITER_RING_ENV: &str = "MT090_WRITER_RING";
const WRITER_HB_COUNTER_ENV: &str = "MT090_WRITER_HB";
const WRITER_EVENTS_ENV: &str = "MT090_WRITER_EVENTS";
const WRITER_STALL_MS_ENV: &str = "MT090_WRITER_STALL_MS";

/// When the test binary is re-exec'd with `MT090_WRITER_MODE=1`, it acts as the "Handshake" WRITER:
/// it CREATES the ring at `MT090_WRITER_RING`, writes a heartbeat (counter = `MT090_WRITER_HB`) + N
/// events (`MT090_WRITER_EVENTS`), prints "READY" to stdout, then SLEEPS `MT090_WRITER_STALL_MS` ms
/// (simulating a frozen UI thread that stopped writing) before exiting. The parent reads the ring
/// while this child is alive + stalled. Returns true if it ran as the writer (the caller then exits).
fn maybe_run_as_writer_child() -> bool {
    if std::env::var(WRITER_MODE_ENV).ok().as_deref() != Some("1") {
        return false;
    }
    let ring = std::env::var(WRITER_RING_ENV).expect("writer child needs a ring path");
    let hb: u64 = std::env::var(WRITER_HB_COUNTER_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let n_events: u64 = std::env::var(WRITER_EVENTS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let stall_ms: u64 = std::env::var(WRITER_STALL_MS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let writer =
        DiagRingWriter::create(Path::new(&ring), DEFAULT_CAPACITY).expect("child creates ring");
    writer.write_heartbeat(hb, hb.wrapping_mul(1000));
    for i in 0..n_events {
        writer.write(DiagEvent::resource_sample(
            99,
            i + 1,
            100 + i,
            2048,
            0,
            i + 1,
        ));
    }
    // Signal the parent the ring is populated, then STALL (frozen writer) while still mapped + alive.
    println!("READY");
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
    std::thread::sleep(Duration::from_millis(stall_ms));
    // Keep the map alive until here; drop happens on return/exit.
    drop(writer);
    true
}

/// The path to THIS compiled test binary (cargo sets `CARGO_BIN_EXE`? No — for an integration test the
/// running binary is `std::env::current_exe()`). We re-exec it with the writer-mode env to act as the
/// cross-process writer child.
fn self_exe() -> PathBuf {
    std::env::current_exe().expect("current_exe for the integration-test binary")
}

/// Build a unique temp ring path (no collisions across parallel tests).
fn temp_ring(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hsk-mt090-{label}-{}-{nanos}.ring",
        std::process::id()
    ))
}

struct PathGuard(PathBuf);
impl Drop for PathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Spawn the writer child (re-exec of this binary). The child runs ONE dedicated host test
/// (`mt090_writer_child_entry`) selected with `--exact`, whose body does nothing but invoke the
/// writer-mode gate. libtest selects tests by name AFTER `main`, so the gate must run inside a SELECTED
/// test — `mt090_writer_child_entry` is that selected test, and `MT090_WRITER_MODE=1` turns its body
/// into the writer. `--nocapture` lets the child's "READY" line reach our piped stdout.
fn spawn_writer_child(ring: &Path, hb: u64, n_events: u64, stall_ms: u64) -> Child {
    Command::new(self_exe())
        .env(WRITER_MODE_ENV, "1")
        .env(WRITER_RING_ENV, ring)
        .env(WRITER_HB_COUNTER_ENV, hb.to_string())
        .env(WRITER_EVENTS_ENV, n_events.to_string())
        .env(WRITER_STALL_MS_ENV, stall_ms.to_string())
        // Select EXACTLY the writer host test so only it runs in the child, and uncapture stdout so the
        // child's "READY" marker reaches the pipe.
        .args(["--exact", "mt090_writer_child_entry", "--nocapture"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn writer child")
}

/// The dedicated WRITER-CHILD HOST test. In a NORMAL run (`MT090_WRITER_MODE` unset) it is a no-op. When
/// re-exec'd by [`spawn_writer_child`] with `MT090_WRITER_MODE=1`, its body runs the writer: create the
/// ring, write a heartbeat + N events, print READY, then stall. It is selected with `--exact` in the
/// child so it is the ONLY test that runs there.
#[test]
fn mt090_writer_child_entry() {
    let _ = maybe_run_as_writer_child();
    // In a normal run this is a no-op (the env gate returns false); nothing to assert.
}

/// Wait for the child to print "READY" on stdout (the ring is populated), up to `timeout`. Returns the
/// child's stdout reader kept alive so the pipe is not closed.
fn wait_ready(child: &mut Child, timeout: Duration) -> bool {
    use std::io::{BufRead, BufReader};
    let stdout = child.stdout.take().expect("child stdout piped");
    let mut reader = BufReader::new(stdout);
    let deadline = Instant::now() + timeout;
    let mut line = String::new();
    // read_line blocks; rely on the child printing READY quickly. If the child died, read_line returns 0.
    while Instant::now() < deadline {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return false, // EOF: child exited without READY
            Ok(_) => {
                if line.trim() == "READY" {
                    return true;
                }
            }
            Err(_) => return false,
        }
    }
    false
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

// EVERY test path must first honor the writer-mode gate so a re-exec'd child acts as the writer and
// NEVER runs the test body. Each test calls `if maybe_run_as_writer_child() { return; }` at its top.

// ---------------------------------------------------------------------------------------------------
// AC-010-1 / PT-010-A — cross-process: a separate writer PROCESS wrote the ring; the reader reads it.
// ---------------------------------------------------------------------------------------------------

#[test]
fn cross_process_writer_reader_reads_heartbeat_and_events() {
    if maybe_run_as_writer_child() {
        return;
    }
    let ring = temp_ring("xproc");
    let _g = PathGuard(ring.clone());

    // A SEPARATE PROCESS creates + writes the ring, then stalls briefly so it is still alive while we
    // read (proving two distinct processes share the same shared-memory pages).
    let mut child = spawn_writer_child(&ring, 7, 4, 600);
    assert!(
        wait_ready(&mut child, Duration::from_secs(10)),
        "writer child must populate the ring and signal READY"
    );

    // Reader in THIS process opens the SAME backing file the OTHER process created. (This is the exact
    // `DiagRingReader::open` + `read_heartbeat` + `read_last_n` that `PalmistryRingReader` delegates to.)
    let reader = DiagRingReader::open(&ring).expect("open the ring a separate process created");
    let hb = reader
        .read_heartbeat()
        .expect("cross-process heartbeat readable");
    assert_eq!(
        hb.counter, 7,
        "reads the heartbeat counter the OTHER process wrote"
    );
    assert_eq!(hb.timestamp_nanos, 7000);

    let events = reader.read_last_n(8);
    assert_eq!(
        events.len(),
        4,
        "reads the last-N events the OTHER process wrote"
    );
    // Newest first; the writer wrote sequence_id 1..=4.
    assert_eq!(events[0].sequence_id, 4);
    assert_eq!(events[3].sequence_id, 1);

    kill_child(&mut child);
}

// ---------------------------------------------------------------------------------------------------
// AC-010-2 / PT-010-B — FROZEN WRITER (same-process thread): reader still reads the last good state
// while the writer thread is STALLED. THE DELIVERABLE.
// ---------------------------------------------------------------------------------------------------

#[test]
fn frozen_writer_reader_still_reads_last_good_state() {
    if maybe_run_as_writer_child() {
        return;
    }
    let ring = temp_ring("frozen-thread");
    let _g = PathGuard(ring.clone());

    // A writer THREAD writes a heartbeat + N events, then PARKS (a frozen UI thread that stopped
    // writing). The reader must STILL read the last good heartbeat + last-N events while it is stalled.
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let last_hb_counter = 55u64;
    writer.write_heartbeat(last_hb_counter, 55_000);
    for i in 0..5u64 {
        writer.write(DiagEvent::slow_frame(1, i + 1, i, 1_000 + i, i + 1));
    }

    // STALL the writer: keep the DiagRingWriter alive (the map stays valid) but DO NOTHING with it — the
    // writer thread is frozen. We model the freeze by simply not writing again and parking the writer in
    // an Arc held by a thread that sleeps. The heartbeat counter is now STUCK at last_hb_counter — which
    // is exactly the staleness MT-091 keys on.
    let writer = Arc::new(writer);
    let frozen = Arc::clone(&writer);
    let stalled = Arc::new(AtomicBool::new(true));
    let stalled_flag = Arc::clone(&stalled);
    let frozen_thread = std::thread::spawn(move || {
        // Hold the writer alive (map stays mapped) and FREEZE: sleep without writing.
        let _hold = frozen;
        while stalled_flag.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    // Now read REPEATEDLY across a window while the writer is frozen. Every read must return the last
    // good state immediately (no block), and the heartbeat must NOT advance (it is frozen).
    let reader = DiagRingReader::open(&ring).expect("open ring");
    let mut readings = Vec::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(300) {
        let read_start = Instant::now();
        let hb = reader.read_heartbeat();
        let read_elapsed = read_start.elapsed();
        // Each read returns promptly even though the writer is frozen (zero cooperation, wait-free).
        assert!(
            read_elapsed < Duration::from_millis(50),
            "a read must not block on a frozen writer (took {read_elapsed:?})"
        );
        readings.push(hb);
        std::thread::sleep(Duration::from_millis(20));
    }

    // The reader read the LAST GOOD heartbeat throughout the freeze, and it never advanced.
    assert!(
        readings.iter().all(|hb| hb == &Some(handshake_diag_ring::Heartbeat {
            counter: last_hb_counter,
            timestamp_nanos: 55_000
        })),
        "the reader must keep reading the LAST GOOD frozen heartbeat (it never advances): {readings:?}"
    );

    // The last-N events written before the freeze are STILL readable while the writer is frozen.
    let events = reader.read_last_n(8);
    assert_eq!(
        events.len(),
        5,
        "all events written before the freeze stay readable from shared memory"
    );
    assert_eq!(
        events[0].sequence_id, 5,
        "newest-first; last event before freeze"
    );

    // Release the frozen writer thread and join.
    stalled.store(false, Ordering::SeqCst);
    frozen_thread.join().expect("join frozen writer thread");
}

// ---------------------------------------------------------------------------------------------------
// AC-010-2 (cross-process) — FROZEN WRITER PROCESS: the writer PROCESS writes then sleeps (frozen);
// the reader reads its last good state while that process is stalled + alive.
// ---------------------------------------------------------------------------------------------------

#[test]
fn frozen_writer_process_still_readable() {
    if maybe_run_as_writer_child() {
        return;
    }
    let ring = temp_ring("frozen-proc");
    let _g = PathGuard(ring.clone());

    // The writer PROCESS writes a heartbeat + 3 events, signals READY, then SLEEPS 800ms (frozen,
    // still alive, NOT writing).
    let mut child = spawn_writer_child(&ring, 88, 3, 800);
    assert!(
        wait_ready(&mut child, Duration::from_secs(10)),
        "frozen writer process must populate the ring and signal READY"
    );

    // While the writer process is STALLED (alive but not writing), the reader reads its last good state.
    let reader = DiagRingReader::open(&ring).expect("open ring");
    // Read across a window; the frozen process never advances the heartbeat.
    let mut last = None;
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(400) {
        let read_start = Instant::now();
        last = reader.read_heartbeat();
        assert!(
            read_start.elapsed() < Duration::from_millis(50),
            "a read must not block on a frozen writer PROCESS"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        last,
        Some(handshake_diag_ring::Heartbeat {
            counter: 88,
            timestamp_nanos: 88_000
        }),
        "the frozen writer process's LAST heartbeat stays readable and never advances"
    );
    let events = reader.read_last_n(8);
    assert_eq!(
        events.len(),
        3,
        "the frozen process's last-N events stay readable"
    );

    kill_child(&mut child);
}

// ---------------------------------------------------------------------------------------------------
// AC-010-5 / PT-010-C — the read path is WAIT-FREE: every read returns within a tight bound even under
// a HOT concurrent writer (no unbounded block).
// ---------------------------------------------------------------------------------------------------

#[test]
fn read_is_wait_free_under_a_hot_concurrent_writer() {
    if maybe_run_as_writer_child() {
        return;
    }
    let ring = temp_ring("waitfree");
    let _g = PathGuard(ring.clone());

    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let writer = Arc::new(writer);
    let stop = Arc::new(AtomicBool::new(false));

    // A HOT writer thread hammers the ring as fast as it can (heartbeat + records), maximizing the
    // chance a reader observes a mid-write seqlock state and has to retry.
    let hot_writer = Arc::clone(&writer);
    let hot_stop = Arc::clone(&stop);
    let hot = std::thread::spawn(move || {
        let mut c = 0u64;
        while !hot_stop.load(Ordering::SeqCst) {
            c = c.wrapping_add(1);
            hot_writer.write_heartbeat(c, c.wrapping_mul(7));
            hot_writer.write(DiagEvent::heartbeat(1, c, c, c));
        }
    });

    // The reader reads in a tight loop; EVERY read must return within a tight bound (the bounded seqlock
    // retry), proving it never blocks unboundedly on the hot writer.
    let reader = DiagRingReader::open(&ring).expect("open ring");
    let mut worst = Duration::ZERO;
    for _ in 0..50_000 {
        let t = Instant::now();
        let _ = reader.read_heartbeat();
        let _ = reader.read_last_n(4);
        let e = t.elapsed();
        if e > worst {
            worst = e;
        }
    }
    stop.store(true, Ordering::SeqCst);
    hot.join().expect("join hot writer");

    // A wait-free bounded-retry read must complete fast. A generous ceiling (the bound is 16 spin
    // retries) — if the reader ever blocked on the writer this would be orders of magnitude larger.
    assert!(
        worst < Duration::from_millis(50),
        "worst-case read under a hot writer must be tightly bounded (was {worst:?}); a blocking read \
         would be far larger"
    );
}

// ---------------------------------------------------------------------------------------------------
// AC-010-4 / PT-010-C — typed-allowlist on the READ side: the reader returns only typed integer values,
// and a SOURCE SCAN confirms there is no string-content read API.
// ---------------------------------------------------------------------------------------------------

#[test]
fn reader_exposes_only_typed_values_no_free_text() {
    if maybe_run_as_writer_child() {
        return;
    }
    // Behavioral: every value the reader returns is a fixed-width integer field — there is structurally
    // no String/&str/[u8] to read out. We assert the returned types are the typed Heartbeat / DiagEvent
    // (all-integer) records and that DiagEvent carries no text field by exercising every accessor.
    let ring = temp_ring("typed");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    writer.write_heartbeat(3, 30_000);
    writer.write(DiagEvent::resource_sample(1, 1, 250, 4096, 12, 7));

    let reader = DiagRingReader::open(&ring).expect("open ring");
    let hb = reader.read_heartbeat().unwrap();
    // The heartbeat is two u64s; consuming them as integers is the ONLY thing possible.
    let _: u64 = hb.counter;
    let _: u64 = hb.timestamp_nanos;
    let ev = &reader.read_last_n(1)[0];
    // Every DiagEvent field is a fixed-width integer (the typed-allowlist). Touch them all to prove
    // there is no text accessor (this would not compile if a String field existed and we tried `: u64`).
    let _: u16 = ev.event_code;
    let _: u8 = ev.phase_marker;
    let _: u8 = ev.severity;
    let _: u64 = ev.thread_id;
    let _: u64 = ev.sequence_id;
    let _: u64 = ev.counter_a;
    let _: u64 = ev.counter_b;
    let _: u64 = ev.metric_micros;
    let _: u64 = ev.timestamp_nanos;

    // SOURCE SCAN (the MT requires it): the reader module exposes NO string-content read API. Scan the
    // ring_reader source for any return type / signature that yields text content. The only `String` in
    // the file is the `path` field (a filesystem path, not ring content) — assert no read method returns
    // a String/&str.
    let src = include_str!("../src/ring_reader.rs");
    // The public read methods are `read_heartbeat` and `read_last_events`; assert neither returns text.
    assert!(
        src.contains("pub fn read_heartbeat(&self) -> Option<Heartbeat>"),
        "read_heartbeat must return the typed Heartbeat, not text"
    );
    assert!(
        src.contains("pub fn read_last_events(&self, n: u32) -> Vec<DiagEvent>"),
        "read_last_events must return typed DiagEvents, not text"
    );
    // No read API returns a String / &str / bytes-as-text. (The `path()` accessor returns &Path, which is
    // a filesystem path, not ring content — explicitly allowed.)
    assert!(
        !src.contains("-> String") && !src.contains("-> &str") && !src.contains("-> Vec<u8>"),
        "the reader must expose NO free-text / raw-bytes read API (typed-allowlist, §6.13.8)"
    );
}
