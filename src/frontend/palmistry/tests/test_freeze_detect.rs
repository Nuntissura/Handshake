//! MT-091 FREEZE-DETECTION PROOFS (the deliverable, §6.13.5 — double-signal gate, NO false positives).
//!
//! These tests prove the §6.13.5 freeze detector against the EXACT primitives the watcher uses: the
//! MT-090 zero-cooperation ring reader (a real MT-081 ring written by a real `DiagRingWriter`) feeding
//! the PRODUCTION `palmistry::freeze_detect::FreezeDetector`, gated by the PRODUCTION
//! `palmistry::hung_window_probe::HungWindowProbe` seam. They cover the four deliverable scenarios
//! the MT names, mapped to the acceptance criteria:
//!
//! - **AC-011-1 / PT-011-A** ([`healthy_advancing_heartbeat_never_freezes`] +
//!   [`idle_cadence_heartbeat_over_real_ring_never_freezes`]): a heartbeat that keeps ADVANCING — even at
//!   the MT-084 ~250ms idle cadence, even across many poll ticks past the ~5s wall-time threshold — NEVER
//!   trips a freeze. THE GATE: a detector that cries wolf on a healthy/idle heartbeat is worse than none.
//! - **AC-011-2 / PT-011-B** ([`stalled_writer_over_real_ring_confirms_freeze`]): a real ring writer
//!   STOPS bumping the heartbeat (a frozen UI thread), staleness crosses the threshold, the injected
//!   hung-window probe corroborates (not-responding), and the detector declares Frozen with the correct
//!   stale duration + last-heartbeat snapshot read from the actual ring.
//! - **AC-011-3 / PT-011-C** ([`stale_but_responding_window_is_suspected_only`]): the heartbeat is stale
//!   BUT the hung-window probe says responding (a borderline long frame) — the detector declares
//!   SUSPECTED, NOT a confirmed hard freeze (the double-signal gate prevents the false positive).
//! - **AC-011-4** ([`freeze_recovers_when_writer_resumes_over_real_ring`]): after a confirmed freeze, the
//!   ring writer resumes bumping the heartbeat and the detector clears back to Healthy (a freeze can
//!   recover; it does not latch Frozen forever).
//! - **LIVE REAL-PROBE PROOF** ([`live_win32_probe_detects_pumping_and_blocked_window`], `#[ignore]`,
//!   opt-in `--ignored` on a real interactive host): the REAL `Win32HungWindowProbe`
//!   (SendMessageTimeoutW WM_NULL SMTO_ABORTIFHUNG) is exercised against a REAL top-level window hosted
//!   by a re-exec'd child process that first PUMPS its message loop (probe must say Responding) and then
//!   BLOCKS the pump (probe must say NotResponding), and finally against the DESTROYED window after the
//!   child dies (probe must say WindowNotFound via the GetLastError/ERROR_INVALID_WINDOW_HANDLE path —
//!   the MT-091 remediation: a destroyed window must never corroborate Frozen).
//!
//! # These tests import the PRODUCTION types (MT-091 remediation)
//!
//! Historically this file carried a faithful re-statement of the detector because `palmistry` was a
//! `[bin]`-only crate whose items `tests/` could not import. MT-093 added the `[lib]` target, so the
//! copy is GONE: every test below drives `palmistry::freeze_detect::{FreezeDetector, FreezeState}` and
//! `palmistry::hung_window_probe::{HungWindowProbe, ProbeResult, FakeHungWindowProbe}` — the binary and
//! these tests exercise the SAME code, over genuine cross-component ring reads (real `DiagRingWriter`
//! -> real `DiagRingReader` -> the production detector) with a controlled `Instant` clock.

use std::time::{Duration, Instant};

use handshake_diag_ring::ring::DEFAULT_CAPACITY;
use handshake_diag_ring::{DiagRingReader, DiagRingWriter, Heartbeat};
use palmistry::freeze_detect::{FreezeDetector, FreezeState};
use palmistry::hung_window_probe::{FakeHungWindowProbe, HungWindowProbe, ProbeResult};

// ---------------------------------------------------------------------------------------------------
// Real-ring harness.
// ---------------------------------------------------------------------------------------------------

fn temp_ring(label: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hsk-mt091-{label}-{}-{nanos}.ring",
        std::process::id()
    ))
}

struct PathGuard(std::path::PathBuf);
impl Drop for PathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ---------------------------------------------------------------------------------------------------
// AC-011-1 / PT-011-A — NO FALSE POSITIVE: a healthy ADVANCING heartbeat never trips a freeze.
// ---------------------------------------------------------------------------------------------------

/// Pure decision-rule proof over the PRODUCTION detector: an advancing counter resets the monotonic
/// clock every tick, so even with a NOT-responding probe and 30s of wall time past the 5s threshold, a
/// freeze is impossible. THE GATE.
#[test]
fn healthy_advancing_heartbeat_never_freezes() {
    let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
    let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
    let base = Instant::now();
    for i in 0..100u64 {
        let now = base + Duration::from_millis(300 * i);
        let hb = Heartbeat {
            counter: i + 1,
            timestamp_nanos: (i + 1) * 1000,
        };
        assert_eq!(
            det.poll(now, Some(hb), &not_responding),
            FreezeState::Healthy,
            "an advancing heartbeat must never freeze (tick {i}, wall {}ms)",
            300 * i
        );
    }
}

/// End-to-end over a REAL ring at the MT-084 ~250ms idle cadence: a writer bumps the heartbeat every
/// ~250ms; the reader reads it; the PRODUCTION detector polls at the ~300ms cadence with a NOT-responding
/// probe; it NEVER freezes across the full window even though wall time exceeds the threshold — because
/// the counter keeps advancing (AC-011-1: a healthy idle app whose heartbeat advances every ~250ms never
/// goes stale).
#[test]
fn idle_cadence_heartbeat_over_real_ring_never_freezes() {
    let ring = temp_ring("idle-cadence");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let reader = DiagRingReader::open(&ring).expect("open ring");

    // Use a SMALL threshold (500ms) and a virtual clock so the test is fast but the RELATIONSHIP is the
    // same as production (idle cadence < poll < threshold << total wall). The writer advances the counter
    // at the idle cadence (every 250ms of virtual time); the detector polls every 300ms. The total window
    // is 5000ms — 10x the threshold — yet a freeze never fires because the counter advances throughout.
    let idle_cadence = Duration::from_millis(250);
    let poll = Duration::from_millis(300);
    let mut det = FreezeDetector::with_threshold(Duration::from_millis(500));
    let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);

    let base = Instant::now();
    let mut counter = 0u64;
    let mut next_hb = Duration::ZERO;
    let total = Duration::from_millis(5000);
    let mut t = Duration::ZERO;
    while t <= total {
        // The writer bumps the heartbeat whenever the idle cadence elapses (real ring write).
        while next_hb <= t {
            counter += 1;
            writer.write_heartbeat(counter, counter * 1000);
            next_hb += idle_cadence;
        }
        // The detector polls: read the REAL heartbeat through the reader, decide.
        let hb = reader.read_heartbeat();
        let state = det.poll(base + t, hb, &not_responding);
        assert_eq!(
            state,
            FreezeState::Healthy,
            "an idle-cadence heartbeat over a real ring must never freeze (t={}ms, counter={counter})",
            t.as_millis()
        );
        t += poll;
    }
    assert!(
        counter >= 19,
        "the writer should have advanced the counter ~20x over 5s at 250ms cadence (was {counter})"
    );
}

// ---------------------------------------------------------------------------------------------------
// AC-011-2 / PT-011-B — FREEZE DETECTED: a stalled writer + corroborating probe => Frozen.
// ---------------------------------------------------------------------------------------------------

/// A REAL ring writer writes a heartbeat then STOPS bumping it (a frozen UI thread). The reader keeps
/// reading the last good (stuck) heartbeat from shared memory; once staleness crosses the threshold and
/// the hung-window probe corroborates (not-responding), the PRODUCTION detector declares Frozen with the
/// correct stale duration + the last-heartbeat snapshot READ FROM THE RING (AC-011-2).
#[test]
fn stalled_writer_over_real_ring_confirms_freeze() {
    let ring = temp_ring("freeze");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let reader = DiagRingReader::open(&ring).expect("open ring");

    let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
    let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
    let base = Instant::now();

    // The writer publishes one heartbeat (counter 77, ts 77_000), then FREEZES — it never writes again.
    writer.write_heartbeat(77, 77_000);
    // Baseline poll: the reader reads counter 77 -> Healthy (and anchors the clock).
    assert_eq!(
        det.poll(base, reader.read_heartbeat(), &not_responding),
        FreezeState::Healthy
    );

    // 6 virtual seconds later the writer is STILL frozen; the reader still reads the stuck counter 77
    // (proving the frozen-writer last-good-state read), staleness is ~6s (> 5s), the probe corroborates.
    let now = base + Duration::from_secs(6);
    let hb = reader.read_heartbeat();
    assert_eq!(
        hb,
        Some(Heartbeat {
            counter: 77,
            timestamp_nanos: 77_000
        }),
        "frozen writer's last heartbeat stays readable"
    );
    let state = det.poll(now, hb, &not_responding);
    match state {
        FreezeState::Frozen(report) => {
            assert!(report.stale_ms >= 6000, "stale ~6s, got {}ms", report.stale_ms);
            assert_eq!(
                report.last_heartbeat_counter, 77,
                "the frozen counter read from the real ring"
            );
            assert_eq!(
                report.last_heartbeat_ts_nanos, 77_000,
                "the frozen heartbeat ts read from the real ring"
            );
        }
        other => panic!("expected a confirmed Frozen, got {other:?}"),
    }
    drop(writer);
}

// ---------------------------------------------------------------------------------------------------
// AC-011-3 / PT-011-C — CORROBORATION / double-signal: stale but RESPONDING window => Suspected, not
// a confirmed hard freeze (the gate that prevents a false positive on a legitimate long frame).
// ---------------------------------------------------------------------------------------------------

#[test]
fn stale_but_responding_window_is_suspected_only() {
    let ring = temp_ring("suspected");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let reader = DiagRingReader::open(&ring).expect("open ring");

    let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
    let responding = FakeHungWindowProbe::new(ProbeResult::Responding);
    let base = Instant::now();

    writer.write_heartbeat(9, 9_000);
    assert_eq!(
        det.poll(base, reader.read_heartbeat(), &responding),
        FreezeState::Healthy
    );

    // Stale for 6s, but the window still pumps messages (a long frame, not a freeze). The reader reads the
    // same stuck heartbeat — staleness crosses the threshold — but the probe says responding, so the
    // detector SUSPECTS only and does NOT confirm a hard freeze (AC-011-3 double-signal gate).
    let state = det.poll(
        base + Duration::from_secs(6),
        reader.read_heartbeat(),
        &responding,
    );
    assert!(
        state.is_suspected(),
        "stale + responding must be SUSPECTED, got {state:?}"
    );
    assert!(
        !state.is_frozen(),
        "a legitimate long frame must NOT confirm a hard freeze"
    );

    // And a window that cannot be resolved likewise cannot corroborate (RISK-011-5): Suspected, not Frozen.
    let mut det2 = FreezeDetector::with_threshold(Duration::from_secs(5));
    let no_window = FakeHungWindowProbe::new(ProbeResult::WindowNotFound);
    det2.poll(base, reader.read_heartbeat(), &no_window);
    let s2 = det2.poll(
        base + Duration::from_secs(6),
        reader.read_heartbeat(),
        &no_window,
    );
    assert!(
        s2.is_suspected(),
        "a missing window cannot confirm a freeze, got {s2:?}"
    );
    drop(writer);
}

// ---------------------------------------------------------------------------------------------------
// AC-011-4 — RECOVERY: after a confirmed freeze, the writer resumes and the detector clears to Healthy.
// ---------------------------------------------------------------------------------------------------

#[test]
fn freeze_recovers_when_writer_resumes_over_real_ring() {
    let ring = temp_ring("recover");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let reader = DiagRingReader::open(&ring).expect("open ring");

    let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
    let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
    let base = Instant::now();

    // Freeze: write once (counter 100), then stop; at +6s the detector confirms Frozen.
    writer.write_heartbeat(100, 100_000);
    det.poll(base, reader.read_heartbeat(), &not_responding);
    let frozen = det.poll(
        base + Duration::from_secs(6),
        reader.read_heartbeat(),
        &not_responding,
    );
    assert!(
        frozen.is_frozen(),
        "the freeze must be confirmed first, got {frozen:?}"
    );

    // The app UNFREEZES: the writer resumes bumping the heartbeat (counter 101). The reader reads the new
    // counter; the detector clears back to Healthy (recovery — it does not latch Frozen forever).
    writer.write_heartbeat(101, 101_000);
    let recovered = det.poll(
        base + Duration::from_millis(6300),
        reader.read_heartbeat(),
        &not_responding,
    );
    assert_eq!(
        recovered,
        FreezeState::Healthy,
        "an advancing counter must clear the freeze (AC-011-4 recovery)"
    );

    // It stays healthy as the writer keeps advancing.
    writer.write_heartbeat(102, 102_000);
    let still = det.poll(
        base + Duration::from_millis(6600),
        reader.read_heartbeat(),
        &not_responding,
    );
    assert_eq!(
        still,
        FreezeState::Healthy,
        "recovery is durable as long as the heartbeat keeps advancing"
    );
    drop(writer);
}

// ---------------------------------------------------------------------------------------------------
// AC-011-5 — staleness is measured against a MONOTONIC reference, never a wall clock that can jump.
// ---------------------------------------------------------------------------------------------------

/// The detector measures staleness with an injected `Instant` (a monotonic clock). This test feeds a
/// monotonic clock that only ever moves FORWARD by the poll cadence and asserts the staleness math is
/// purely `now - last_advance` — independent of any wall-clock value. (A wall-clock-based detector could
/// be fooled by an NTP step / DST jump; this one cannot, because `Instant` cannot go backwards and is not
/// the system wall clock.)
#[test]
fn staleness_uses_monotonic_reference_only() {
    let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
    let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
    let base = Instant::now();
    // Establish baseline.
    det.poll(
        base,
        Some(Heartbeat {
            counter: 1,
            timestamp_nanos: 1,
        }),
        &not_responding,
    );
    // The heartbeat's EMBEDDED timestamp is wildly inconsistent (as if the writer's clock jumped), but the
    // COUNTER does not advance — staleness must come purely from the monotonic `now`, not the embedded ts.
    let weird_hb = Heartbeat {
        counter: 1,
        timestamp_nanos: u64::MAX,
    };
    let state = det.poll(
        base + Duration::from_secs(6),
        Some(weird_hb),
        &not_responding,
    );
    // Stale by the monotonic clock (6s) -> Frozen regardless of the embedded ts being garbage.
    assert!(
        state.is_frozen(),
        "staleness must derive from the monotonic clock, not the embedded ts: {state:?}"
    );
    if let FreezeState::Frozen(report) = state {
        assert!(
            (6000..=6100).contains(&report.stale_ms),
            "stale_ms must be the monotonic delta ~6000, got {}",
            report.stale_ms
        );
    }
}

// ---------------------------------------------------------------------------------------------------
// LIVE REAL-PROBE PROOF (MT-091 remediation) — the REAL Win32HungWindowProbe against a REAL window
// whose message pump first PUMPS and then BLOCKS, hosted in a re-exec'd child process.
// ---------------------------------------------------------------------------------------------------

/// Env gate that turns the re-exec'd child into the WINDOW HOST (never set in a normal test run).
#[cfg(windows)]
const WINDOW_CHILD_ENV: &str = "HSK_MT091_WINDOW_CHILD";
/// How long the child PUMPS its message loop before deliberately blocking, in ms.
#[cfg(windows)]
const WINDOW_CHILD_PUMP_MS_ENV: &str = "HSK_MT091_PUMP_MS";
/// How long the child BLOCKS its message pump (sleeps without pumping) before exiting, in ms.
#[cfg(windows)]
const WINDOW_CHILD_BLOCK_MS_ENV: &str = "HSK_MT091_BLOCK_MS";

/// The dedicated WINDOW-CHILD HOST test (the `test_ring_reader_zero_coop.rs` re-exec pattern). In a
/// NORMAL run (`HSK_MT091_WINDOW_CHILD` unset) it is a no-op. When re-exec'd by the live probe test with
/// the env set, its body: registers + creates a REAL top-level window (offscreen, WS_EX_NOACTIVATE |
/// WS_EX_TOOLWINDOW so it never steals focus or appears in the taskbar — quiet-operation duty), prints
/// `PUMPING`, services its message loop for the pump window, prints `BLOCKING`, then SLEEPS without
/// pumping (a deliberately hung pump), destroys the window, prints `DONE`, and exits.
#[test]
fn mt091_window_child_entry() {
    #[cfg(windows)]
    {
        let _ = maybe_run_as_window_child();
    }
    // On non-Windows (and in a normal run) this is a no-op; nothing to assert.
}

#[cfg(windows)]
fn maybe_run_as_window_child() -> bool {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW,
        RegisterClassW, ShowWindow, TranslateMessage, MSG, PM_REMOVE, SW_SHOWNOACTIVATE, WNDCLASSW,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
    };

    if std::env::var(WINDOW_CHILD_ENV).ok().as_deref() != Some("1") {
        return false;
    }
    let pump_ms: u64 = std::env::var(WINDOW_CHILD_PUMP_MS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    let block_ms: u64 = std::env::var(WINDOW_CHILD_BLOCK_MS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8000);

    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: plain passthrough window procedure; all args come from the OS dispatcher.
        unsafe { DefWindowProcW(hwnd, msg, w, l) }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let class_name = wide("HskMt091ProbeHostWindow");
    // SAFETY: standard RegisterClassW + CreateWindowExW sequence on this thread; the class name and
    // window procedure outlive the window; a null parent/menu is valid for a top-level popup.
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
            eprintln!("RegisterClassW failed");
            return true;
        }
        // Offscreen + never-activate + no taskbar button: the probe only needs WS_VISIBLE (the style
        // IsWindowVisible checks), not an on-screen focus-stealing window (quiet-operation duty).
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            wide("hsk-mt091-probe-host").as_ptr(),
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
        eprintln!("CreateWindowExW failed");
        return true;
    }
    // SAFETY: hwnd was just created on this thread. SW_SHOWNOACTIVATE sets WS_VISIBLE without focus.
    unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };

    use std::io::Write as _;
    println!("PUMPING");
    let _ = std::io::stdout().flush();

    // PHASE A — a LIVE message pump for `pump_ms`: service the queue so a WM_NULL probe is answered.
    let pump_deadline = Instant::now() + Duration::from_millis(pump_ms);
    while Instant::now() < pump_deadline {
        // SAFETY: standard PeekMessage/Translate/Dispatch loop on the window's owning thread.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    println!("BLOCKING");
    let _ = std::io::stdout().flush();

    // PHASE B — a DELIBERATELY HUNG pump: sleep without servicing the queue. A WM_NULL sent with a
    // bounded timeout can now only time out (the real not-responding condition, no fake).
    std::thread::sleep(Duration::from_millis(block_ms));

    // SAFETY: hwnd is still this thread's window; destroy it before exit.
    unsafe { DestroyWindow(hwnd) };
    println!("DONE");
    let _ = std::io::stdout().flush();
    true
}

/// THE LIVE REAL-PROBE TEST (MT-091 remediation — the Responding/NotResponding branches of the REAL
/// `Win32HungWindowProbe` were previously exercised by NO test anywhere; a flag/return-code inversion
/// would have passed every fake-probe test).
///
/// Flow: re-exec this test binary as a WINDOW HOST child (`mt091_window_child_entry` +
/// `HSK_MT091_WINDOW_CHILD=1`); wait for its `PUMPING` marker; assert the REAL probe reports
/// `Responding` while the child pumps; wait for `BLOCKING`; assert the REAL probe reports
/// `NotResponding` once the pump is stuck (SendMessageTimeoutW WM_NULL times out); then kill the child
/// and assert the probe reports `WindowNotFound` for the DESTROYED window (the
/// GetLastError/ERROR_INVALID_WINDOW_HANDLE remediation path — a dead window must NOT corroborate
/// Frozen).
#[cfg(windows)]
#[test]
#[ignore = "LIVE Win32 hung-window probe: spawns a re-exec'd child process hosting a REAL top-level \
            window (offscreen, WS_EX_NOACTIVATE — no focus steal) that pumps, then BLOCKS its message \
            pump for ~8s; opt-in real-interactive-host proof — run with --ignored. Asserts the REAL \
            Win32HungWindowProbe returns Responding (pumping), NotResponding (blocked pump), and \
            WindowNotFound (destroyed window, the ERROR_INVALID_WINDOW_HANDLE path)."]
fn live_win32_probe_detects_pumping_and_blocked_window() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    use palmistry::hung_window_probe::Win32HungWindowProbe;

    let exe = std::env::current_exe().expect("current_exe for the integration-test binary");
    let mut child = Command::new(exe)
        .env(WINDOW_CHILD_ENV, "1")
        .env(WINDOW_CHILD_PUMP_MS_ENV, "6000")
        .env(WINDOW_CHILD_BLOCK_MS_ENV, "10000")
        .args(["--exact", "mt091_window_child_entry", "--nocapture"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the window-host child");
    let child_pid = child.id();

    let stdout = child.stdout.take().expect("child stdout piped");
    let mut reader = BufReader::new(stdout);

    // Read child stdout lines until `marker` appears (libtest banner lines are interleaved), bounded.
    let mut wait_marker = |marker: &str, timeout: Duration| -> bool {
        let deadline = Instant::now() + timeout;
        let mut line = String::new();
        while Instant::now() < deadline {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => return false, // EOF — child exited early.
                Ok(_) => {
                    if line.trim() == marker {
                        return true;
                    }
                }
                Err(_) => return false,
            }
        }
        false
    };

    // PHASE A — the child pumps: the REAL probe must observe Responding.
    assert!(
        wait_marker("PUMPING", Duration::from_secs(20)),
        "the window-host child must report PUMPING (did the child fail to create its window?)"
    );
    let probe = Win32HungWindowProbe::new(child_pid);
    let mut saw_responding = false;
    let responding_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < responding_deadline {
        match probe.probe() {
            ProbeResult::Responding => {
                saw_responding = true;
                break;
            }
            // WindowNotFound can race window creation for a moment; NotResponding must not appear
            // while the pump is live, but a single early snapshot is tolerated inside the retry loop.
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    assert!(
        saw_responding,
        "the REAL Win32 probe must report Responding while the child's message pump is live"
    );

    // PHASE B — the child blocks its pump: the REAL probe must observe NotResponding.
    assert!(
        wait_marker("BLOCKING", Duration::from_secs(20)),
        "the window-host child must report BLOCKING"
    );
    // Give the child a moment to actually enter the blocked sleep after printing the marker.
    std::thread::sleep(Duration::from_millis(300));
    let mut saw_not_responding = false;
    let blocked_deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < blocked_deadline {
        if probe.probe() == ProbeResult::NotResponding {
            saw_not_responding = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        saw_not_responding,
        "the REAL Win32 probe must report NotResponding once the child's message pump is blocked \
         (SendMessageTimeoutW WM_NULL timeout)"
    );

    // PHASE C — the window is DESTROYED (kill the child): the probe's cached HWND is now invalid and
    // the ret==0 + ERROR_INVALID_WINDOW_HANDLE path must report WindowNotFound (NOT NotResponding —
    // a destroyed window must never corroborate a freeze; the MT-091 remediation under test).
    child.kill().expect("kill the window-host child");
    let _ = child.wait();
    std::thread::sleep(Duration::from_millis(200));
    let mut saw_window_not_found = false;
    let gone_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < gone_deadline {
        if probe.probe() == ProbeResult::WindowNotFound {
            saw_window_not_found = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        saw_window_not_found,
        "after the child dies, the probe must resolve the destroyed cached HWND to WindowNotFound \
         (ERROR_INVALID_WINDOW_HANDLE), never a freeze corroboration"
    );
}
