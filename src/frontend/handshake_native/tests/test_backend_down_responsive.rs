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
//! - AC-008-1 / PT-008-A (`backend_down_app_stays_responsive`, THE RE-PROVE): the real app with the
//!   backend down, stepped many times, stays RESPONSIVE — every frame completes within a tight bound far
//!   below the connect timeout (no frame stalls for the connect attempt) AND the MT-084 heartbeat counter
//!   advances by N across N frames (the UI thread is provably never stalled; the 2026-06-26 CPU->0
//!   symptom is gone).
//! - AC-008-2 / PT-008-B (`frame_path_has_no_ui_thread_block_on`): a source audit confirms the per-frame
//!   layout lifecycle uses the off-thread spawn+poll path (`spawn_layout_load` / `poll_layout_load`), NOT
//!   a UI-thread `load_layout`/`block_on` on the frame path.
//! - AC-008-3 / PT-008-C (`backend_unreachable_event_recorded` + `recovery_fires_recovered_event`): a
//!   `BackendUnreachable` typed event is recorded once on the down edge (debounced — not per frame) and a
//!   `BackendRecovered` once on the recovery edge.
//! - AC-008-4 / PT-008-C (`backend_down_surface_is_degraded_not_spinner`): the affected surface shows a
//!   DEGRADED/disconnected state (an explicit, finite indicator — not a perpetual spinner, not a hang).
//! - AC-008-5 (`reqwest_clients_carry_connect_and_request_timeouts`): the backend reqwest clients carry a
//!   short connect timeout + request timeout (defense in depth; `src/backend` untouched).
//! - AC-008-6 (`recovery_fires_recovered_event`): recovery works — the surface re-connects and
//!   `BackendRecovered` fires (no permanent stuck-disconnected state).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use serde_json::Value;

use handshake_diag_ring::DiagEventCode;
use handshake_native::app::{HandshakeApp, HealthDisplayState, HEARTBEAT_IDLE_REPAINT_INTERVAL};
use handshake_native::backend_client::BACKEND_CONNECT_TIMEOUT as CLIENT_CONNECT_TIMEOUT;
use handshake_native::diagnostics::{self, BUFFER_CAP};
use handshake_native::layout_persistence::{
    LayoutError, LayoutPersistenceManager, LayoutTransport,
};

/// A backend base URL whose port is reliably NOT listening, so every connection is refused — a
/// genuinely-down backend for the re-prove (NOT a mock; the TCP connect is really refused). Port 1 on
/// loopback has nothing listening, so the connect is refused immediately (and is in any case bounded by
/// the MT-088 connect timeout). The re-prove points the REAL `/health` + layout-transport code paths here.
const DEAD_BACKEND_URL: &str = "http://127.0.0.1:1";

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
    BACKEND_EVENT_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

// ── artifact hygiene (CX-212E): no repo-local artifact dir may exist ───────────────────────────────

/// The external artifact root for any MT-088 test output. The proofs here are all in-memory (frame
/// timing + the in-process diagnostics buffer + a source scan); no screenshot/PNG is written, but the
/// guard is invoked uniformly so the hygiene contract is enforced and the helper is not dead.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
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
fn backend_down_app_stays_responsive() {
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
        "with nothing listening on 127.0.0.1:37501 the app must observe the backend as unreachable and \
         enter the degraded state (NOT spin forever / hang). If this fails a real backend may be running \
         on 37501 during the test."
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

    // The eframe update body's UI render (`self.ui(ctx)`) is where the frame-path lifecycle runs; confirm
    // `ui` calls drive_layout_persistence (so the audited fn IS on the frame path) — but via the off-
    // thread helpers proven above, never a UI-thread block.
    let ui_fn = extract_fn_body(&app_src, "pub fn ui(&mut self, ctx: &egui::Context)")
        .expect("app.rs declares pub fn ui(&mut self, ctx: &egui::Context)");
    assert!(
        ui_fn.contains("drive_layout_persistence"),
        "ui() drives the layout persistence lifecycle (so the off-thread audit above covers the frame path)"
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
        build_fn.contains("connect_timeout"),
        "build_backend_client must set a connect_timeout on the reqwest ClientBuilder"
    );
    assert!(
        build_fn.contains(".timeout("),
        "build_backend_client must set an overall request timeout on the reqwest ClientBuilder"
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

    assert_no_local_artifact_dir();
}

// ── AC-008-3 + AC-008-6 / PT-008-C: recovery fires BackendRecovered, no stuck-disconnected ────────

/// Recovery works — the down->up edge fires exactly one `BackendRecovered` event and clears the degraded
/// state (no permanent stuck-disconnected). This is proven DETERMINISTICALLY (no live backend race) by
/// driving the public load path with a scripted transport that returns a transport ERROR (backend down),
/// then a successful load (backend recovered): the first observation sets `backend_is_down()` + records
/// `BackendUnreachable`; the second clears it + records `BackendRecovered`. The edges are debounced (a
/// repeated same-state observation records nothing).
#[test]
fn recovery_fires_recovered_event() {
    // Serialize the shared-global event count (see BACKEND_EVENT_TEST_LOCK) so the edge deltas are
    // deterministic against the other backend-event tests in this binary.
    let _guard = lock_backend_event_tests();
    // A headless app (no network) whose layout manager we drive with a scripted transport.
    let mut app = HandshakeApp::with_health(HealthDisplayState::Loading);

    // Scripted reachability: Err (down), Err (still down — must NOT re-emit), Ok(None) (recovered),
    // Ok(None) (still up — must NOT re-emit).
    let transport = ScriptedTransport::new(vec![
        Err(LayoutError::Transport("connection refused (backend down)".into())),
        Err(LayoutError::Transport("connection refused (still down)".into())),
        Ok(None),
        Ok(None),
    ]);
    app.set_layout_manager(LayoutPersistenceManager::new(
        Box::new(transport),
        Duration::from_millis(500),
    ));

    let extent = app.monitor_extent();
    let (unreachable0, recovered0) = count_backend_events();

    // 1) Down edge: a transport error must set the degraded state + record ONE BackendUnreachable.
    app.load_layout("proj-x", extent);
    assert!(app.backend_is_down(), "a transport-error load enters the degraded (down) state");
    let (u1, r1) = count_backend_events();
    assert_eq!(u1 - unreachable0, 1, "exactly one BackendUnreachable on the down edge");
    assert_eq!(r1 - recovered0, 0, "no BackendRecovered yet (still down)");

    // 2) Still down: a second error observation must NOT re-emit (debounced to the edge).
    app.load_layout("proj-x", extent);
    assert!(app.backend_is_down(), "still down after a second error");
    let (u2, r2) = count_backend_events();
    assert_eq!(u2 - u1, 0, "no second BackendUnreachable while already down (debounced)");
    assert_eq!(r2 - r1, 0, "no BackendRecovered while still down");

    // 3) Recovery edge: a successful load must clear the degraded state + record ONE BackendRecovered.
    app.load_layout("proj-x", extent);
    assert!(
        !app.backend_is_down(),
        "a successful load clears the degraded state — no permanent stuck-disconnected (AC-008-6)"
    );
    let (u3, r3) = count_backend_events();
    assert_eq!(u3 - u2, 0, "no new BackendUnreachable on recovery");
    assert_eq!(r3 - r2, 1, "exactly one BackendRecovered on the recovery edge");

    // 4) Still up: a second successful observation must NOT re-emit (debounced to the edge).
    app.load_layout("proj-x", extent);
    assert!(!app.backend_is_down(), "still reachable after a second success");
    let (u4, r4) = count_backend_events();
    assert_eq!(u4 - u3, 0, "no BackendUnreachable while reachable");
    assert_eq!(r4 - r3, 0, "no second BackendRecovered while already reachable (debounced)");

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

// ── a scripted LayoutTransport for the deterministic recovery proof ────────────────────────────────

/// A test transport whose `load` returns a SCRIPTED sequence of results (front of the queue first), so
/// the recovery proof drives the exact reachable->unreachable->reachable edges with NO live backend and
/// NO timing race. `save` always succeeds (the recovery proof exercises load only).
struct ScriptedTransport {
    load_results: Mutex<VecDeque<Result<Option<Value>, LayoutError>>>,
}

impl ScriptedTransport {
    fn new(results: Vec<Result<Option<Value>, LayoutError>>) -> Self {
        Self {
            load_results: Mutex::new(results.into_iter().collect()),
        }
    }
}

impl LayoutTransport for ScriptedTransport {
    fn load(&self, _workspace_id: &str) -> Result<Option<Value>, LayoutError> {
        let mut q = self.load_results.lock().unwrap_or_else(|p| p.into_inner());
        // Once the script is exhausted, keep returning the last scripted state shape (Ok(None) = up).
        q.pop_front().unwrap_or(Ok(None))
    }
    fn save(&self, _workspace_id: &str, _layout_state: Value) -> Result<(), LayoutError> {
        Ok(())
    }
}

// ── source-scan helpers (code-only) ────────────────────────────────────────────────────────────────

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
