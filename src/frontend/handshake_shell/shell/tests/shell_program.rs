//! The shell program on this machine's real device, as a test binary, quiet, on a private desktop that receives no input,
//! in a process of its own (MT-214 S5, AC214-10, AC214-11, AC214-12): the five crates run as one process, the window is
//! created and shown without activation, its tree is given to the accessibility hub, the heartbeat beats into a ring, and
//! the program ends by itself, drains, settles every claim, marks the ring and asks the watcher exactly once.
//!
//! Never on the Operator's desktop: `on_private_desktop` runs the loop on a thread bound to a desktop of its own and
//! watches the Operator's focus, Z order and input for changes. A machine without a device skips, with the owner's reason,
//! unless `HANDSHAKE_REQUIRE_VULKAN_GPU` says a device is required.
#![cfg(windows)]

#[path = "../../surface/tests/support/mod.rs"]
mod support;

#[path = "../../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs"]
mod limits;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use handshake_diag_ring::ring::{DiagRingReaderV2, DiagRingWriterV2};
use handshake_diag_ring::{DiagEventCode, DEFAULT_CAPACITY};
use handshake_shell::{
    run_program, DeviceAuthority, EndAfter, ExitKind, ProgramConfig, ShutdownWatcher, TestBinary, WatcherAck,
};
use handshake_shell_window::{LogicalSize, PhysicalPosition, Placement};

use limits::{RingFile, TimeLimit};
use support::desktop::on_private_desktop;
use support::gpu_env::{in_own_process, is_required, ran_in_its_own_process, REQUIRE_GPU};

const LIMIT: Duration = Duration::from_secs(100);
/// Ends the test before the private desktop's watchdog and the parent process's deadline do, so its ring file is
/// removed first.
const TIME_LIMIT: Duration = Duration::from_secs(90);
/// Long enough for several beats (the port asks for one every 100 ms) and for the window's first frames.
const RUN_FOR: Duration = Duration::from_millis(800);
/// The ring's launch nonce: the program's ring is the launcher's version 2 ring.
const NONCE: [u8; 16] = [0x5a; 16];

/// A watcher that counts how often it was asked.
struct Counting(Arc<AtomicU32>);

impl ShutdownWatcher for Counting {
    fn acknowledge_shutdown(&self) -> WatcherAck {
        self.0.fetch_add(1, Ordering::SeqCst);
        WatcherAck::Acknowledged
    }
}

/// A watcher that never answers.
struct Silent(Arc<AtomicU32>);

impl ShutdownWatcher for Silent {
    fn acknowledge_shutdown(&self) -> WatcherAck {
        self.0.fetch_add(1, Ordering::SeqCst);
        WatcherAck::NotAcknowledged
    }
}

fn config(ring: Option<DiagRingWriterV2>) -> ProgramConfig {
    let mut config = ProgramConfig::new(ring);
    config.title = "shell program test".to_owned();
    config.size = LogicalSize::new(240.0, 140.0);
    config.placement = Some(Placement::At(PhysicalPosition::new(40, 40)));
    config.end_after = EndAfter {
        time: Some(RUN_FOR),
        frames_presented: None,
    };
    config
}

/// `true` after saying why there is no device to run on; fails when one is required.
fn no_device(outcome: &handshake_shell::ProgramOutcome) -> bool {
    if outcome.kind != ExitKind::DeviceUnavailable {
        return false;
    }
    assert!(
        !is_required(std::env::var_os(REQUIRE_GPU).as_deref()),
        "no device to run on ({:?}), and {REQUIRE_GPU} is not 0",
        outcome.detail
    );
    // AC214-9: no device is a typed, readable state with the owner's own code, ending with the typed code.
    let state = outcome.unavailable.expect("a missing device is a typed, readable state");
    assert_eq!(state.code(), outcome.detail.expect("the owner's key"));
    assert_eq!(state.end().code(), ExitKind::DeviceUnavailable.code());
    assert!(outcome.report.is_none(), "no loop ran without a device");
    println!("HSK_GPU_SKIP reason={}", outcome.detail.unwrap_or("none"));
    true
}

#[test]
fn the_program_runs_drains_marks_the_ring_and_asks_the_watcher_once() {
    in_own_process!();
    let _limit = TimeLimit::new(TIME_LIMIT);
    // Out here, so a failure on the loop's thread still unwinds to it and removes the file.
    let ring = RingFile::new("program");
    let path = ring.path().to_owned();
    let asked = Arc::new(AtomicU32::new(0));
    let for_watcher = Arc::clone(&asked);
    let outcome = on_private_desktop(LIMIT, move || {
        let writer = DiagRingWriterV2::create(&path, DEFAULT_CAPACITY, NONCE).expect("the ring");
        let authority = DeviceAuthority::Test(TestBinary::declare("shell_program_test"));
        assert!(authority.receipt().contains("product=false"));
        run_program(config(Some(writer)), authority, Box::new(Counting(for_watcher)))
    });
    if no_device(&outcome) {
        return;
    }
    assert_eq!(outcome.kind, ExitKind::Clean, "{}", outcome.line());
    let report = outcome.report.expect("the loop ran");
    assert_eq!(report.run.windows_created, 1);
    assert_eq!(report.run.windows_destroyed, 1, "the window is gone");
    assert_eq!(report.run.quiet_broken, 0, "the window never took the foreground");
    assert!(report.present.frames_presented >= 1, "a frame was presented: {:?}", report.present);
    assert_eq!(report.present.errors, 0, "{:?}", report.present.last_error);
    assert!(report.settled, "{report:?}");
    assert!(
        report.present.shutdown.is_some_and(|owner| owner.is_settled()),
        "the owner closed with nothing held: {:?}",
        report.present.shutdown
    );
    assert_eq!(report.present.claims_abandoned, 0);
    assert_eq!(report.present.claims_settled, report.present.surfaces_made);
    // E9-F1: the one window's presents were proven ended before its surface was released.
    support::assert_shutdown_proved(&report.present, 1);
    // The accessibility hub: the tree was attached before the first show and the shell frame given, whole, once.
    assert_eq!(report.attach_refused, 0, "{:?}", report.access_code);
    assert_eq!(report.access.frames, 1, "{:?}", report.access);
    assert_eq!(report.access.frames_refused, 0);
    // The heartbeat beat while the loop ran and watched the window's presents.
    assert!(report.beats >= 3, "{report:?}");
    assert_eq!(report.watch_refused, 0);
    // The ring holds the shutdown marker, and the watcher was asked once, after the shutdown settled.
    assert!(report.ring_marked);
    assert_eq!(report.watcher, Some(WatcherAck::Acknowledged));
    assert_eq!(asked.load(Ordering::SeqCst), 1, "asked exactly once");
    let reader = DiagRingReaderV2::open(ring.path(), NONCE).expect("the reader");
    let marked = reader
        .read_last_n(32)
        .iter()
        .filter(|event| event.event.event_code == DiagEventCode::Shutdown.as_u16())
        .count();
    assert_eq!(marked, 1, "one shutdown marker in the ring");
}

#[test]
fn a_watcher_that_does_not_answer_is_a_typed_ending_not_a_clean_one() {
    in_own_process!();
    let _limit = TimeLimit::new(TIME_LIMIT);
    let asked = Arc::new(AtomicU32::new(0));
    let for_watcher = Arc::clone(&asked);
    let outcome = on_private_desktop(LIMIT, move || {
        let authority = DeviceAuthority::Test(TestBinary::declare("shell_program_test"));
        run_program(config(None), authority, Box::new(Silent(for_watcher)))
    });
    if no_device(&outcome) {
        return;
    }
    assert_eq!(outcome.kind, ExitKind::WatcherNotAcknowledged, "{}", outcome.line());
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    let report = outcome.report.expect("the loop ran");
    assert!(report.settled, "it was the watcher's silence, not unsettled work");
    support::assert_shutdown_proved(&report.present, 1);
    assert!(!report.ring_marked, "no ring was given, so none was marked");
}
