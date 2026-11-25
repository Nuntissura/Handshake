//! The heartbeat against Palmistry's own ring reader, freeze decision and constants.

mod support;

use std::time::Duration;

use handshake_diag_ring::ring::DiagRingWriterV2;
use handshake_diag_ring::DEFAULT_CAPACITY;
use palmistry::freeze_detect::{
    FreezeState, FREEZE_THRESHOLD, MT084_IDLE_HEARTBEAT_CADENCE, POLL_INTERVAL,
};
use palmistry::hung_window_probe::{FakeHungWindowProbe, ProbeResult};
use palmistry::ring_reader::PalmistryRingReaderV2;
use support::{all_healthy, nanos, Desk, ManualClock, ManualLoop, RingFile, Ui, Verdict, STEP};
use ui_heartbeat::{
    UiHeartbeat, WakeOutcome, LOST_WAKE_GRACE, MAX_BEAT_GAP, REFUSED_WAKE_RETRY, WAKE_DELAY,
};

fn first_unhealthy(verdicts: &[Verdict]) -> Option<&Verdict> {
    verdicts
        .iter()
        .find(|verdict| verdict.state != FreezeState::Healthy)
}

#[test]
fn cadence_agrees_with_the_watchers_constants() {
    assert_eq!(
        MAX_BEAT_GAP, MT084_IDLE_HEARTBEAT_CADENCE,
        "the watcher's idle cadence"
    );
    assert!(WAKE_DELAY < MAX_BEAT_GAP);
    assert!(
        WAKE_DELAY + LOST_WAKE_GRACE + POLL_INTERVAL < FREEZE_THRESHOLD,
        "a lost wake is replaced long before it can look like a freeze"
    );
    assert!(REFUSED_WAKE_RETRY < MAX_BEAT_GAP);
}

/// AC215-6, MT-084 RISK-004-1: an idle UI that draws nothing stays alive.
#[test]
fn idle_ui_without_frames_never_looks_frozen() {
    let mut desk = Desk::new("idle");
    desk.event_loop
        .set_latency(MAX_BEAT_GAP - WAKE_DELAY - STEP);
    desk.heartbeat.start().expect("starts");

    let verdicts = desk.run(
        Duration::from_secs(600),
        Ui::Dispatching,
        ProbeResult::NotResponding,
    );

    assert!(all_healthy(&verdicts), "{:?}", first_unhealthy(&verdicts));
    assert!(
        desk.longest_gap <= nanos(MAX_BEAT_GAP),
        "a beat at least every {MAX_BEAT_GAP:?}, longest gap {} ns",
        desk.longest_gap
    );
    let counters = desk.heartbeat.counters();
    assert_eq!(counters.overdue_beats, 0);
    assert_eq!(counters.replaced_wakes, 0);
    assert_eq!(desk.watcher.heartbeat().counter, counters.beats);
}

/// AC215-6, HBR-HOST-006: a hung UI thread stops the heartbeat, and the watcher confirms it in its own time.
#[test]
fn blocked_ui_thread_is_confirmed_frozen_within_the_watchers_threshold() {
    let mut desk = Desk::running("blocked");
    let blocked_at = desk.clock.now();
    let last = desk.heartbeat.last_beat().expect("alive before the block");

    let verdicts = desk.run(
        Duration::from_secs(7),
        Ui::Blocked,
        ProbeResult::NotResponding,
    );

    assert_eq!(
        desk.watcher.heartbeat(),
        last,
        "nothing is published while blocked"
    );
    let first = first_unhealthy(&verdicts).expect("the block is noticed");
    let after = Duration::from_nanos(first.at - blocked_at);
    assert!(
        after > FREEZE_THRESHOLD - MAX_BEAT_GAP - POLL_INTERVAL,
        "{after:?}"
    );
    assert!(
        after <= FREEZE_THRESHOLD + POLL_INTERVAL + POLL_INTERVAL,
        "{after:?}"
    );
    match first.state {
        FreezeState::Frozen(report) => {
            assert_eq!(report.last_heartbeat_counter, last.counter);
            assert_eq!(report.last_heartbeat_ts_nanos, last.timestamp_nanos);
        }
        other => panic!("stale and not responding is a confirmed freeze, got {other:?}"),
    }
}

/// The watcher's second signal still decides: stale alone is suspected, never confirmed.
#[test]
fn stale_heartbeat_alone_is_only_suspected() {
    for probe in [ProbeResult::Responding, ProbeResult::WindowNotFound] {
        let mut desk = Desk::running("suspected");
        let verdicts = desk.run(Duration::from_secs(7), Ui::Blocked, probe);
        let last = verdicts.last().expect("polled").state;
        assert!(last.is_suspected(), "{probe:?}: {last:?}");
        assert!(verdicts.iter().all(|verdict| !verdict.state.is_frozen()));
    }
}

/// A thread that comes back is healthy at once, with one beat for the wake it missed and no catch-up burst.
#[test]
fn ui_thread_that_returns_is_healthy_with_one_beat() {
    let mut desk = Desk::running("returns");
    let before = desk.heartbeat.counters();
    let frozen = desk.run(
        Duration::from_secs(7),
        Ui::Blocked,
        ProbeResult::NotResponding,
    );
    assert!(frozen.last().expect("polled").state.is_frozen());

    assert_eq!(desk.dispatch_due(), 1, "one beat, however long the block");
    let healthy = desk.run(
        Duration::from_secs(5),
        Ui::Dispatching,
        ProbeResult::NotResponding,
    );
    assert!(all_healthy(&healthy));

    let after = desk.heartbeat.counters();
    assert_eq!(after.overdue_beats, before.overdue_beats + 1);
    assert!(after.longest_gap_nanos >= nanos(Duration::from_secs(7)));
}

/// Drawing and input are not liveness: a loop that never takes its wakes goes stale whatever else it does.
#[test]
fn turns_without_dispatched_wakes_do_not_keep_it_alive() {
    let mut desk = Desk::running("turns-only");
    let last = desk.heartbeat.last_beat().expect("alive before");

    let verdicts = desk.run(
        Duration::from_secs(7),
        Ui::TurnsOnly,
        ProbeResult::NotResponding,
    );

    assert_eq!(desk.watcher.heartbeat(), last);
    assert!(verdicts.last().expect("polled").state.is_frozen());
    let replaced = desk.heartbeat.counters().replaced_wakes;
    assert!(
        (1..=14).contains(&replaced),
        "replacements are spaced, not one per turn: {replaced}"
    );
}

/// A nested modal loop that dispatches window messages keeps the beat; one that swallows the wakes does not, and the
/// outer loop recovers.
#[test]
fn modal_loops() {
    let mut desk = Desk::running("modal");
    // The outer handler has not returned: these dispatches are nested in it.
    let nested = desk.run(
        Duration::from_secs(30),
        Ui::Dispatching,
        ProbeResult::Responding,
    );
    assert!(all_healthy(&nested));

    let swallowing = desk.run(Duration::from_secs(7), Ui::Blocked, ProbeResult::Responding);
    assert!(swallowing.last().expect("polled").state.is_suspected());
    desk.event_loop.lose_pending();

    // Back in the outer loop: any event replaces the lost wake, and the replacement beats.
    let counter = desk.watcher.heartbeat().counter;
    assert!(desk.heartbeat.on_loop_turn(), "a replacement is taken");
    assert_eq!(
        desk.watcher.heartbeat().counter,
        counter,
        "a turn publishes nothing"
    );
    let recovered = desk.run(
        Duration::from_secs(2),
        Ui::Dispatching,
        ProbeResult::Responding,
    );
    assert!(all_healthy(&recovered));
}

/// Sleep: both processes stop together and the timer may not survive. After the resume notification the next dispatch
/// beats, once, and the gap is recorded as a resume, not as a stall.
#[test]
fn system_resume_beats_once_and_is_not_a_stall() {
    let mut desk = Desk::running("resume");
    let before = desk.heartbeat.counters();
    desk.clock.advance(Duration::from_secs(8 * 3600));
    desk.event_loop.lose_pending();

    assert!(desk.heartbeat.on_system_resume());
    assert_eq!(desk.event_loop.last_delay(), Duration::ZERO);
    assert_eq!(desk.dispatch_due(), 1);
    let verdicts = desk.run(
        Duration::from_secs(10),
        Ui::Dispatching,
        ProbeResult::NotResponding,
    );
    assert!(all_healthy(&verdicts), "{:?}", first_unhealthy(&verdicts));

    let after = desk.heartbeat.counters();
    assert_eq!(after.resumes, before.resumes + 1);
    assert_eq!(after.overdue_beats, before.overdue_beats);
}

/// The watcher may poll between the resume and the UI thread's first dispatch. Both clocks jumped, so that one poll
/// sees a stale counter: with the window answering it only suspects, and the next poll after the dispatch is healthy.
#[test]
fn poll_between_resume_and_first_dispatch_is_suspected_at_most_until_the_dispatch() {
    let mut desk = Desk::running("resume-race");
    desk.clock.advance(Duration::from_secs(3600));
    let responding = FakeHungWindowProbe::new(ProbeResult::Responding);
    let now = desk.clock.now();
    assert!(desk.watcher.poll(now, &responding).is_suspected());

    desk.heartbeat.on_system_resume();
    assert!(desk.dispatch_due() >= 1);
    let now = desk.clock.now();
    assert_eq!(desk.watcher.poll(now, &responding), FreezeState::Healthy);
}

/// MT-084 AC-004-2: published time strictly rises and never goes back, and the beat does not depend on it.
#[test]
fn clock_running_backward_neither_stops_the_beat_nor_turns_time_back() {
    let desk = Desk::running("backward");
    let mut last = desk.heartbeat.last_beat().expect("alive");
    desk.clock.set(1);

    for _ in 0..20 {
        // The operating system's timer fires whatever our clock reads.
        let ticket = desk.event_loop.take_any().expect("a wake is in flight");
        match desk.heartbeat.on_wake(ticket) {
            WakeOutcome::Beat(beat) => {
                assert_eq!(beat.counter, last.counter + 1);
                assert!(beat.timestamp_nanos > last.timestamp_nanos);
                last = beat;
            }
            other => panic!("a dispatched wake beats, got {other:?}"),
        }
        desk.clock.advance(WAKE_DELAY);
    }
    assert_eq!(desk.heartbeat.counters().clock_regressions, 1);
}

/// A clock that ran backward must not postpone the recovery of a lost wake by the size of its jump.
#[test]
fn lost_wake_is_replaced_in_bounded_time_after_the_clock_ran_backward() {
    let mut desk = Desk::running("backward-lost");
    desk.clock.set(1);
    desk.event_loop.lose_pending();
    let give_up = desk.clock.now() + nanos(WAKE_DELAY + LOST_WAKE_GRACE) + nanos(STEP);
    while !desk.heartbeat.on_loop_turn() {
        assert!(
            desk.clock.now() <= give_up,
            "recovery waited for the old time"
        );
        desk.clock.advance(STEP);
    }
    assert_eq!(desk.dispatch_due(), 1);
}

#[test]
fn clock_jumping_forward_gives_one_beat_not_a_burst() {
    let mut desk = Desk::running("forward");
    desk.clock.advance(Duration::from_secs(3600));
    assert_eq!(desk.dispatch_due(), 1);
    assert_eq!(desk.dispatch_due(), 0, "the next wake is due later");
    desk.clock.advance(WAKE_DELAY);
    assert_eq!(desk.dispatch_due(), 1);
}

/// A new ring reads as counter 0, not as unwritten, so the first beat must differ from 0.
#[test]
fn first_beat_is_an_advance_over_a_new_ring() {
    let mut desk = Desk::new("first");
    let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
    assert_eq!(desk.watcher.heartbeat().counter, 0);
    assert_eq!(desk.watcher.poll(0, &not_responding), FreezeState::Healthy);

    desk.heartbeat.start().expect("starts");
    assert_eq!(
        desk.watcher.heartbeat().counter,
        0,
        "starting is not a beat: the loop has not dispatched anything"
    );
    desk.clock.advance(Duration::from_secs(4));
    assert_eq!(desk.dispatch_due(), 1);
    assert_eq!(desk.watcher.heartbeat().counter, 1);
    desk.clock.advance(Duration::from_secs(2));
    let now = desk.clock.now();
    assert_eq!(
        desk.watcher.poll(now, &not_responding),
        FreezeState::Healthy,
        "6 s after the first poll, but the counter moved"
    );
}

/// The production ring is version 2, with a launch nonce: the same port, read by the watcher's version 2 reader.
#[test]
fn version_2_ring_carries_the_beat() {
    let ring = RingFile::new("v2");
    let nonce = [0x5a; 16];
    let writer = DiagRingWriterV2::create(ring.path(), DEFAULT_CAPACITY, nonce).expect("v2 ring");
    let reader = PalmistryRingReaderV2::open(ring.path(), nonce).expect("the watcher maps it");
    let clock = ManualClock::default();
    let event_loop = ManualLoop::new(clock.clone());
    let heartbeat = UiHeartbeat::new(clock.clone(), event_loop.clone(), writer);
    heartbeat.start().expect("starts");

    for counter in 1..=5 {
        clock.advance(WAKE_DELAY);
        let ticket = event_loop.take_due().expect("due");
        let WakeOutcome::Beat(beat) = heartbeat.on_wake(ticket) else {
            panic!("a due wake beats");
        };
        assert_eq!(beat.counter, counter);
        assert_eq!(reader.read_heartbeat(), Some(beat));
    }
    assert!(
        reader.read_last_events(16).is_empty(),
        "the heartbeat slot only: no record is written"
    );
}
