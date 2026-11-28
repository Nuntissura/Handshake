//! The shell's Palmistry adapter, without a watcher process, a window or a device (MT-214 S7 P3, AC214-11): what each
//! report of the launcher means to the program, that the watcher is asked once, and that an adapter dropped without having
//! been asked never tells the watcher anything (the launcher's handle would send a clean Shutdown when dropped).
#![cfg(windows)]

use std::io;
use std::os::windows::process::ExitStatusExt;
use std::process::ExitStatus;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use handshake_shell::{conclude, watcher_ack, ExitKind, OwnedWatcher, PalmistryWatcher, ShutdownWatcher, WatcherAck};
use palmistry_launch::{ShutdownAcknowledgement, ShutdownExit, ShutdownOutcome};

fn status(code: u32) -> ExitStatus {
    ExitStatus::from_raw(code)
}

fn confirmed() -> ShutdownAcknowledgement {
    ShutdownAcknowledgement::SignedConfirmed
}

fn unconfirmed() -> ShutdownAcknowledgement {
    ShutdownAcknowledgement::SignedUnconfirmed(io::Error::other("no reply"))
}

fn unsigned() -> ShutdownAcknowledgement {
    ShutdownAcknowledgement::UnsignedNotApplicable
}

fn cleanly(acknowledgement: ShutdownAcknowledgement, code: u32) -> ShutdownOutcome {
    ShutdownOutcome::ExitedCleanly(ShutdownExit {
        acknowledgement,
        status: status(code),
    })
}

/// What the fake saw: how often it was shut down and how often it was dropped.
#[derive(Default)]
struct Seen {
    shut_down: AtomicU32,
    dropped: AtomicU32,
}

struct Fake {
    seen: Arc<Seen>,
    outcome: Mutex<Option<ShutdownOutcome>>,
}

impl Fake {
    fn boxed(seen: &Arc<Seen>, outcome: ShutdownOutcome) -> Box<dyn OwnedWatcher> {
        Box::new(Self {
            seen: Arc::clone(seen),
            outcome: Mutex::new(Some(outcome)),
        })
    }
}

impl OwnedWatcher for Fake {
    fn shut_down(self: Box<Self>) -> ShutdownOutcome {
        self.seen.shut_down.fetch_add(1, Ordering::SeqCst);
        self.outcome.lock().unwrap().take().expect("shut down once")
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.seen.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

/// Every report of the launcher, with both kinds of status where it matters: only a clean owned end that confirmed its
/// signed Shutdown and succeeded is an acknowledgement.
#[test]
fn only_a_confirmed_successful_owned_end_is_an_acknowledgement() {
    let table: Vec<(&str, ShutdownOutcome, WatcherAck)> = vec![
        ("confirmed, success", cleanly(confirmed(), 0), WatcherAck::Acknowledged),
        ("confirmed, failing status", cleanly(confirmed(), 1), WatcherAck::NotAcknowledged),
        ("unsigned, success", cleanly(unsigned(), 0), WatcherAck::NotAcknowledged),
        ("unconfirmed, success", cleanly(unconfirmed(), 0), WatcherAck::NotAcknowledged),
        (
            "uncleanly, confirmed",
            ShutdownOutcome::ExitedUncleanly {
                acknowledgement: confirmed(),
                status: status(1),
            },
            WatcherAck::NotAcknowledged,
        ),
        (
            "uncleanly, success status, unconfirmed",
            ShutdownOutcome::ExitedUncleanly {
                acknowledgement: unconfirmed(),
                status: status(0),
            },
            WatcherAck::NotAcknowledged,
        ),
        (
            "backstop",
            ShutdownOutcome::Backstop {
                acknowledgement: confirmed(),
                initial_wait_error: None,
                kill_result: Ok(()),
                reap_result: Ok(Some(status(0))),
            },
            WatcherAck::NotAcknowledged,
        ),
        ("already done", ShutdownOutcome::AlreadyDone, WatcherAck::NotAcknowledged),
    ];
    for (name, outcome, expected) in table {
        assert_eq!(watcher_ack(&outcome), expected, "{name}");
    }
}

/// The watcher is asked once, on every branch, and a second question never reaches it again.
#[test]
// The adapter has no Drop of its own on purpose; `drop` here ends its life, which is the case under test.
#[allow(clippy::drop_non_drop)]
fn the_watcher_is_asked_once_whatever_it_answers() {
    let cases: Vec<(&str, bool, ShutdownOutcome, WatcherAck)> = vec![
        ("acknowledged", true, cleanly(confirmed(), 0), WatcherAck::Acknowledged),
        ("not acknowledged", true, cleanly(unconfirmed(), 0), WatcherAck::NotAcknowledged),
        ("unconfirmed handshake", false, cleanly(confirmed(), 0), WatcherAck::NotConnected),
    ];
    for (name, handshake_confirmed, outcome, expected) in cases {
        let seen = Arc::new(Seen::default());
        let watcher = PalmistryWatcher::over(Fake::boxed(&seen, outcome), handshake_confirmed);
        assert!(watcher.holds_watcher(), "{name}");
        assert_eq!(watcher.acknowledge_shutdown(), expected, "{name}");
        assert!(!watcher.holds_watcher(), "{name}: taken by the question");
        assert_eq!(watcher.acknowledge_shutdown(), WatcherAck::NotConnected, "{name}: a second question");
        assert_eq!(seen.shut_down.load(Ordering::SeqCst), 1, "{name}: shut down once");
        drop(watcher);
        assert_eq!(seen.shut_down.load(Ordering::SeqCst), 1, "{name}: nothing more at drop");
    }
}

/// No watcher: nothing is asked and the program may end clean.
#[test]
fn no_watcher_is_not_connected_and_the_end_stays_clean() {
    let watcher = PalmistryWatcher::none();
    assert!(!watcher.holds_watcher());
    let ack = watcher.acknowledge_shutdown();
    assert_eq!(ack, WatcherAck::NotConnected);
    assert_eq!(conclude(true, None, Some(ack)), ExitKind::Clean);
    assert_eq!(watcher.acknowledge_shutdown(), WatcherAck::NotConnected, "a second question");
}

/// What the program makes of each answer (the existing decision, over this adapter's answers).
#[test]
fn the_end_follows_the_answer() {
    assert_eq!(conclude(true, None, Some(WatcherAck::Acknowledged)), ExitKind::Clean);
    assert_eq!(conclude(true, None, Some(WatcherAck::NotAcknowledged)), ExitKind::WatcherNotAcknowledged);
    assert_eq!(conclude(true, None, Some(WatcherAck::NotConnected)), ExitKind::Clean);
}

/// An adapter dropped without having been asked (an unsettled end, a failed loop, a panic unwinding) never shuts the
/// watcher down and never drops it: the launcher's handle sends a clean Shutdown when dropped.
#[test]
// The adapter has no Drop of its own on purpose; `drop` here ends its life, which is the case under test.
#[allow(clippy::drop_non_drop)]
fn an_adapter_dropped_without_a_question_tells_the_watcher_nothing() {
    for handshake_confirmed in [true, false] {
        let seen = Arc::new(Seen::default());
        let watcher = PalmistryWatcher::over(Fake::boxed(&seen, cleanly(confirmed(), 0)), handshake_confirmed);
        drop(watcher);
        assert_eq!(seen.shut_down.load(Ordering::SeqCst), 0, "no Shutdown was sent");
        assert_eq!(seen.dropped.load(Ordering::SeqCst), 0, "the watcher was not dropped (a real handle would send one)");
    }
    // Positive control: a watcher that was asked is consumed, so the fake counts its drop.
    let seen = Arc::new(Seen::default());
    let watcher = PalmistryWatcher::over(Fake::boxed(&seen, cleanly(confirmed(), 0)), true);
    watcher.acknowledge_shutdown();
    assert_eq!(seen.dropped.load(Ordering::SeqCst), 1, "the counter can count");
}

/// The same through an unwinding panic.
#[test]
fn a_panic_unwinding_past_the_adapter_tells_the_watcher_nothing() {
    let seen = Arc::new(Seen::default());
    let inner = Arc::clone(&seen);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _watcher = PalmistryWatcher::over(Fake::boxed(&inner, cleanly(confirmed(), 0)), true);
        panic!("the loop failed");
    }));
    assert!(result.is_err());
    assert_eq!(seen.shut_down.load(Ordering::SeqCst), 0);
    assert_eq!(seen.dropped.load(Ordering::SeqCst), 0);
}
