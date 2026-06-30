//! WP-KERNEL-012 MT-105 generalized operation stall watchdog proofs.

use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use handshake_diag_ring::{DiagEvent, DiagEventCode};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::diagnostics::{
    self, OperationCode, OperationWatchdog, BUFFER_CAP, OPERATION_WATCHDOG_POLL_INTERVAL,
};

static WATCHDOG_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_watchdog_tests() -> std::sync::MutexGuard<'static, ()> {
    WATCHDOG_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn stalled_events() -> Vec<DiagEvent> {
    diagnostics::snapshot_last_n(BUFFER_CAP)
        .into_iter()
        .filter(|event| event.event_code == DiagEventCode::StalledOperation.as_u16())
        .collect()
}

fn wait_for_stalled_delta(before: usize, timeout: Duration) -> DiagEvent {
    let deadline = Instant::now() + timeout;
    loop {
        let events = stalled_events();
        if events.len() > before {
            return *events.last().expect("stalled events non-empty after delta");
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for a StalledOperation event"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn stalled_operation_emits_one_typed_event_within_bound() {
    let _guard = lock_watchdog_tests();
    let before = stalled_events().len();
    let poll_interval = Duration::from_millis(20);
    let deadline = Duration::from_millis(80);
    let watchdog = OperationWatchdog::new(poll_interval);
    let _thread = watchdog
        .start_poll_thread()
        .expect("fresh watchdog starts a poll thread");
    let handle = watchdog.register(OperationCode::BackendCall, deadline, None);
    let operation_id = handle.operation_id();

    let event = wait_for_stalled_delta(before, Duration::from_secs(3));

    assert_eq!(event.event_code, DiagEventCode::StalledOperation.as_u16());
    assert_eq!(event.sequence_id, operation_id);
    assert_eq!(event.counter_a, OperationCode::BackendCall.as_u64());
    assert!(
        event.counter_b >= deadline.as_millis() as u64,
        "last_progress_ms should be at least the configured deadline"
    );
    assert_eq!(event.metric_micros % 1000, 0);
    let elapsed_ms = event.metric_micros / 1000;
    assert!(
        elapsed_ms <= (deadline + poll_interval + Duration::from_millis(200)).as_millis() as u64,
        "event elapsed_ms={elapsed_ms} should be bounded by deadline + one poll with CI slack"
    );

    drop(handle);
}

#[test]
fn ticking_and_completed_operations_do_not_false_flag() {
    let _guard = lock_watchdog_tests();
    let before = stalled_events().len();
    let watchdog = OperationWatchdog::new(OPERATION_WATCHDOG_POLL_INTERVAL);
    let handle = watchdog.register(
        OperationCode::BackendCall,
        Duration::from_millis(250),
        Some(Duration::from_millis(250)),
    );

    for _ in 0..8 {
        std::thread::sleep(Duration::from_millis(60));
        handle.tick();
        assert_eq!(
            watchdog.poll_once(),
            0,
            "a ticking operation must not emit StalledOperation"
        );
    }
    handle.complete();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(watchdog.poll_once(), 0);

    assert_eq!(
        stalled_events().len(),
        before,
        "ticking/completed operations must not add stalled events"
    );
}

#[test]
fn stalled_operation_is_debounced_to_one_event() {
    let _guard = lock_watchdog_tests();
    let before = stalled_events().len();
    let watchdog = OperationWatchdog::new(OPERATION_WATCHDOG_POLL_INTERVAL);
    let _handle = watchdog.register(OperationCode::BackendCall, Duration::from_millis(30), None);

    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(watchdog.poll_once(), 1, "first stalled edge emits once");
    for _ in 0..6 {
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(watchdog.poll_once(), 0, "still-stalled op is debounced");
    }

    assert_eq!(
        stalled_events().len() - before,
        1,
        "exactly one StalledOperation event is recorded for one stalled operation"
    );
}

#[test]
fn register_tick_complete_stress_has_no_deadlock_or_spurious_emit() {
    let _guard = lock_watchdog_tests();
    let before = stalled_events().len();
    let watchdog = OperationWatchdog::new(OPERATION_WATCHDOG_POLL_INTERVAL);
    let _thread = watchdog
        .start_poll_thread()
        .expect("fresh watchdog starts a poll thread");
    let mut joins = Vec::new();

    for _ in 0..4 {
        let watchdog = watchdog.clone();
        joins.push(std::thread::spawn(move || {
            for _ in 0..500 {
                let handle =
                    watchdog.register(OperationCode::BackendCall, Duration::from_secs(60), None);
                handle.tick();
                handle.complete();
            }
        }));
    }

    for join in joins {
        join.join().expect("stress worker did not panic");
    }
    assert_eq!(watchdog.active_stalled_count(), 0);
    assert_eq!(
        stalled_events().len(),
        before,
        "register/tick/complete do not emit; only the poll thread emits stalled events"
    );
}

#[test]
fn completed_after_deadline_but_before_poll_does_not_emit() {
    let _guard = lock_watchdog_tests();
    let before = stalled_events().len();
    let watchdog = OperationWatchdog::new(OPERATION_WATCHDOG_POLL_INTERVAL);
    let handle = watchdog.register(OperationCode::BackendCall, Duration::from_millis(20), None);

    std::thread::sleep(Duration::from_millis(40));
    handle.complete();

    assert_eq!(
        watchdog.poll_once(),
        0,
        "an operation completed before the watchdog poll must not be reported as stalled"
    );
    assert_eq!(watchdog.active_stalled_count(), 0);
    assert_eq!(stalled_events().len(), before);
}

#[test]
fn shipped_backend_hang_path_emits_stalled_operation_at_runtime() {
    let _guard = lock_watchdog_tests();
    let before = stalled_events().len();
    let server = HangingBackend::start();
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));

    harness
        .state_mut()
        .set_backend_unreachable_for_test(&server.base_url);

    let deadline = Instant::now()
        + diagnostics::BACKEND_OPERATION_STALL_DEADLINE
        + OPERATION_WATCHDOG_POLL_INTERVAL
        + Duration::from_secs(4);
    let mut saw_status_indicator = false;
    while stalled_events().len() == before && Instant::now() < deadline {
        harness.step();
        saw_status_indicator |= harness
            .state()
            .status_bar_health_text()
            .contains("Stalled ops:");
        std::thread::sleep(Duration::from_millis(40));
    }

    let events = stalled_events();
    let event = events
        .get(before)
        .expect("shipped backend hang path should emit a StalledOperation event");
    assert_eq!(event.counter_a, OperationCode::BackendCall.as_u64());
    assert!(
        event.counter_b >= diagnostics::BACKEND_OPERATION_STALL_DEADLINE.as_millis() as u64,
        "backend-call stall should carry last_progress_ms past the backend deadline"
    );
    saw_status_indicator |= harness
        .state()
        .status_bar_health_text()
        .contains("Stalled ops:");
    assert!(
        saw_status_indicator,
        "the runtime backend stall must surface in the status bar while active"
    );

    drop(server);
    let clear_deadline = Instant::now() + Duration::from_secs(8);
    while diagnostics::active_stalled_operation_count() > 0 && Instant::now() < clear_deadline {
        harness.step();
        std::thread::sleep(Duration::from_millis(40));
    }
    assert_eq!(
        diagnostics::active_stalled_operation_count(),
        0,
        "active stalled-operation projection clears after the hung backend sockets close"
    );
}

#[test]
fn typed_constructor_and_schema_size_stay_fixed() {
    let event =
        DiagEvent::stalled_operation(0, 42, OperationCode::BackendCall.as_u64(), 123, 77, 999);
    assert_eq!(std::mem::size_of::<DiagEvent>(), 56);
    assert_eq!(event.event_code, DiagEventCode::StalledOperation.as_u16());
    assert_eq!(event.sequence_id, 42);
    assert_eq!(event.counter_a, OperationCode::BackendCall.as_u64());
    assert_eq!(event.counter_b, 77);
    assert_eq!(event.metric_micros, 123_000);
    assert_eq!(event.timestamp_nanos, 999);
}

#[test]
fn watchdog_source_exposes_no_free_text_payload_or_allocating_emit_path() {
    let src = strip_line_comments(include_str!("../src/diagnostics/operation_watchdog.rs"));
    for forbidden in ["String", "&str", "Vec<", "format!", "std::process::Command"] {
        assert!(
            !src.contains(forbidden),
            "operation watchdog source must not expose {forbidden} on the typed emit path"
        );
    }
    assert!(
        src.contains("#[repr(u16)]") && src.contains("pub enum OperationCode"),
        "OperationCode must be a closed numeric enum"
    );
    assert!(
        src.contains("DiagEvent::stalled_operation"),
        "the watchdog emits through the typed StalledOperation constructor"
    );
    assert!(
        src.contains("crate::diagnostics::record("),
        "the watchdog reuses the MT-082 recorder API"
    );
    let tick_fn =
        extract_fn_body(&src, "pub fn tick(&self)").expect("OperationHandle::tick exists");
    assert!(
        !tick_fn.contains("lock_operations"),
        "tick must not contend on the registry map"
    );
    let complete_fn =
        extract_fn_body(&src, "pub fn complete(&self)").expect("OperationHandle::complete exists");
    assert!(
        !complete_fn.contains("lock_operations"),
        "complete must not contend on the registry map"
    );
    let active_count_fn = extract_fn_body(&src, "pub fn active_stalled_count(&self)")
        .expect("OperationWatchdog::active_stalled_count exists");
    assert!(
        !active_count_fn.contains("lock_operations"),
        "status-bar active stalled count must be an atomic projection, not a UI-thread registry scan"
    );
    assert!(
        src.contains("AtomicU8")
            && src.contains("OPERATION_STATE_STALLING")
            && src.contains("compare_exchange"),
        "operation lifecycle must use atomic active/stalling/stalled/completed state transitions"
    );
}

#[test]
fn shipped_backend_health_and_layout_paths_register_operations() {
    let app_src = strip_line_comments(include_str!("../src/app.rs"));
    let health_probe = extract_fn_body(&app_src, "fn spawn_health_probe(")
        .expect("app.rs declares spawn_health_probe");
    assert!(health_probe.contains("global_operation_watchdog().register"));
    assert!(health_probe.contains("OperationCode::BackendCall"));
    assert!(health_probe.contains("backend_client::fetch_health"));
    assert!(health_probe.contains("operation_handle.complete()"));

    let app_new = extract_fn_body(&app_src, "pub fn new(cc: &eframe::CreationContext<'_>)")
        .expect("app.rs declares production constructor");
    assert!(app_new.contains("start_global_operation_watchdog"));
    assert!(app_new.contains("Self::spawn_health_probe"));
    let install_pos = app_new
        .find("install_diagnostics_ring")
        .expect("production constructor installs diagnostics");
    let watchdog_pos = app_new
        .find("start_global_operation_watchdog")
        .expect("production constructor starts watchdog");
    let health_pos = app_new
        .find("Self::spawn_health_probe")
        .expect("production constructor spawns health probe");
    assert!(
        install_pos < watchdog_pos && watchdog_pos < health_pos,
        "diagnostics ring must install before the watchdog can emit, and before initial health registers"
    );

    let layout_load = extract_fn_body(&app_src, "fn spawn_layout_load(&mut self")
        .expect("app.rs declares spawn_layout_load");
    assert!(layout_load.contains("global_operation_watchdog().register"));
    assert!(layout_load.contains("OperationCode::BackendCall"));
    assert!(layout_load.contains("operation_handle.complete()"));

    let poll_health =
        extract_fn_body(&app_src, "fn poll_health(&mut self").expect("app.rs declares poll_health");
    assert!(
        poll_health.contains("Self::spawn_health_probe"),
        "health re-probes use the shipped watchdog-wrapped health helper"
    );
}

#[test]
fn diagnostics_panel_and_status_bar_surface_stalled_operation() {
    let _guard = lock_watchdog_tests();
    let handle = diagnostics::global_operation_watchdog().register(
        OperationCode::BackendCall,
        Duration::from_millis(1),
        None,
    );
    std::thread::sleep(Duration::from_millis(5));
    assert_eq!(diagnostics::global_operation_watchdog().poll_once(), 1);

    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    assert!(
        app.status_bar_health_text().contains("Stalled ops:"),
        "status bar health segment appends a minimal stalled-operation indicator"
    );
    handle.complete();
    assert!(
        !app.status_bar_health_text().contains("Stalled ops:"),
        "status bar stalled-operation indicator clears when the active stalled operation completes"
    );

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.set_size(egui::Vec2::new(1200.0, 800.0));
    harness.state_mut().open_settings();
    harness.step();
    harness.get_by_label("Search settings").focus();
    harness.step();
    harness
        .get_by_label("Search settings")
        .type_text("diagnostics");
    harness.run_steps(3);

    let labels = live_text_values(&harness);
    assert!(
        labels.iter().any(|label| label == "StalledOperation"),
        "diagnostics panel renders at least one StalledOperation row"
    );
    assert!(
        labels.iter().any(|label| label.contains("Stalled ops:")),
        "diagnostics panel renders the stalled-operation count"
    );
}

fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_fn_body<'a>(src: &'a str, signature: &str) -> Option<&'a str> {
    let start = src.find(signature)?;
    let body_start = src[start..].find('{').map(|offset| start + offset)?;
    let mut depth = 0usize;
    for (offset, ch) in src[body_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(&src[body_start..=body_start + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

fn live_text_values(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().value())
        .collect()
}

struct HangingBackend {
    base_url: String,
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl HangingBackend {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind hanging backend");
        listener
            .set_nonblocking(true)
            .expect("make hanging backend listener nonblocking");
        let addr = listener.local_addr().expect("read hanging backend addr");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let join = std::thread::spawn(move || {
            let mut held_streams: Vec<TcpStream> = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(10);
            while !stop_thread.load(Ordering::Acquire) && Instant::now() < deadline {
                match listener.accept() {
                    Ok((stream, _)) => held_streams.push(stream),
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            base_url: format!("http://{addr}"),
            stop,
            join: Some(join),
        }
    }
}

impl Drop for HangingBackend {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
