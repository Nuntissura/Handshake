//! What the port accepts, ignores, recovers and publishes, and in which order it ends.

mod support;

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use handshake_diag_ring::{DiagRingReader, DiagRingWriter, Heartbeat, DEFAULT_CAPACITY};
use support::{nanos, ManualClock, ManualLoop, RingFile, STEP};
use ui_heartbeat::{
    EventLoopWake, HeartbeatSink, IgnoredWake, ProcessClock, RecorderHeartbeat, StartRefused,
    UiHeartbeat, WakeOutcome, WakeRefused, WakeTicket, LOST_WAKE_GRACE, REFUSED_WAKE_RETRY,
    WAKE_DELAY,
};

/// Compiles only while `$type` does not implement `$trait`.
macro_rules! assert_not_implemented {
    ($type:ty, $trait:path) => {
        const _: fn() = || {
            trait AmbiguousIfImplemented<A> {
                fn some_item() {}
            }
            impl<T: ?Sized> AmbiguousIfImplemented<()> for T {}
            #[allow(dead_code)]
            struct Implemented;
            impl<T: ?Sized + $trait> AmbiguousIfImplemented<Implemented> for T {}
            let _ = <$type as AmbiguousIfImplemented<_>>::some_item;
        };
    };
}

/// A loop that may be used from any thread, so only the port itself can keep another thread out.
#[allow(dead_code)]
struct AnywhereLoop;

impl EventLoopWake for AnywhereLoop {
    fn on_loop_thread(&self) -> bool {
        true
    }
    fn request_wake(&self, _: WakeTicket, _: Duration) -> Result<(), WakeRefused> {
        Ok(())
    }
    fn cancel_wakes(&self) {}
}

// AC215-6: no other thread can publish. Every part below may cross threads; the port built from them may not.
const _: fn() = || {
    fn may_cross_threads<T: Send + Sync>() {}
    may_cross_threads::<ProcessClock>();
    may_cross_threads::<AnywhereLoop>();
    may_cross_threads::<Option<DiagRingWriter>>();
    may_cross_threads::<WakeTicket>();
};
assert_not_implemented!(UiHeartbeat<ProcessClock, AnywhereLoop, Option<DiagRingWriter>>, Send);
assert_not_implemented!(UiHeartbeat<ProcessClock, AnywhereLoop, Option<DiagRingWriter>>, Sync);

/// Keeps what was published.
#[derive(Clone, Default)]
struct Published(Rc<RefCell<Vec<Heartbeat>>>);

impl Published {
    fn count(&self) -> usize {
        self.0.borrow().len()
    }
}

impl HeartbeatSink for Published {
    fn publish(&self, beat: Heartbeat) {
        self.0.borrow_mut().push(beat);
    }
}

struct Bench {
    heartbeat: UiHeartbeat<ManualClock, ManualLoop, Published>,
    event_loop: ManualLoop,
    clock: ManualClock,
    published: Published,
}

impl Bench {
    fn new() -> Self {
        let clock = ManualClock::default();
        clock.set(nanos(Duration::from_secs(1)));
        let event_loop = ManualLoop::new(clock.clone());
        let published = Published::default();
        Self {
            heartbeat: UiHeartbeat::new(clock.clone(), event_loop.clone(), published.clone()),
            event_loop,
            clock,
            published,
        }
    }

    fn running() -> Self {
        let bench = Self::new();
        bench.heartbeat.start().expect("starts");
        bench.beat();
        bench
    }

    /// Dispatch the next wake at its time and expect a beat.
    fn beat(&self) -> (WakeTicket, Heartbeat) {
        self.clock.advance(WAKE_DELAY);
        let ticket = self.event_loop.take_due().expect("a wake is due");
        match self.heartbeat.on_wake(ticket) {
            WakeOutcome::Beat(beat) => (ticket, beat),
            other => panic!("a due wake beats, got {other:?}"),
        }
    }
}

/// A loop whose `request_wake` calls back into the port, as a loop that dispatches inline would.
struct InlineLoop {
    port: RefCell<Weak<UiHeartbeat<ManualClock, InlineRef, Published>>>,
    outcome: Cell<Option<WakeOutcome>>,
    stop_inside: Cell<bool>,
    cancels: Cell<u64>,
}

#[derive(Clone)]
struct InlineRef(Rc<InlineLoop>);

impl EventLoopWake for InlineRef {
    fn on_loop_thread(&self) -> bool {
        true
    }
    fn request_wake(&self, ticket: WakeTicket, _: Duration) -> Result<(), WakeRefused> {
        if let Some(port) = self.0.port.borrow().upgrade() {
            if self.0.stop_inside.get() {
                port.stop();
            } else {
                self.0.outcome.set(Some(port.on_wake(ticket)));
                port.on_loop_turn();
            }
        }
        Ok(())
    }
    fn cancel_wakes(&self) {
        self.0.cancels.set(self.0.cancels.get() + 1);
    }
}

fn inline_port() -> (
    Rc<UiHeartbeat<ManualClock, InlineRef, Published>>,
    Rc<InlineLoop>,
    Published,
) {
    let looped = Rc::new(InlineLoop {
        port: RefCell::new(Weak::new()),
        outcome: Cell::new(None),
        stop_inside: Cell::new(false),
        cancels: Cell::new(0),
    });
    let published = Published::default();
    let port = Rc::new(UiHeartbeat::new(
        ManualClock::default(),
        InlineRef(Rc::clone(&looped)),
        published.clone(),
    ));
    *looped.port.borrow_mut() = Rc::downgrade(&port);
    (port, looped, published)
}

#[test]
fn nothing_is_published_before_the_loop_dispatches_the_first_wake() {
    let bench = Bench::new();
    assert_eq!(bench.heartbeat.last_beat(), None);
    assert!(!bench.heartbeat.is_running());
    bench.heartbeat.start().expect("starts");
    assert!(bench.heartbeat.is_running());
    assert_eq!(bench.event_loop.last_delay(), Duration::ZERO);
    assert_eq!(bench.published.count(), 0);
    let (_, beat) = bench.beat();
    assert_eq!(beat.counter, 1);
    assert_eq!(bench.heartbeat.last_beat(), Some(beat));
}

#[test]
fn start_is_refused_when_it_would_prove_nothing_or_again() {
    let bench = Bench::new();
    bench.event_loop.refuse(Some(WakeRefused::Unavailable));
    assert_eq!(
        bench.heartbeat.start(),
        Err(StartRefused::Wake(WakeRefused::Unavailable))
    );
    assert!(!bench.heartbeat.is_running());

    bench.event_loop.refuse(None);
    assert_eq!(bench.heartbeat.start(), Ok(()));
    assert_eq!(bench.heartbeat.start(), Err(StartRefused::AlreadyRunning));
    bench.heartbeat.stop();
    assert_eq!(bench.heartbeat.start(), Err(StartRefused::Stopped));
}

#[test]
fn only_a_wake_the_port_asked_for_and_has_not_seen_beats() {
    let bench = Bench::running();
    let (ticket, _) = bench.beat();
    let published = bench.published.count();

    assert_eq!(
        bench.heartbeat.on_wake(ticket),
        WakeOutcome::Ignored(IgnoredWake::AlreadyRedeemed)
    );
    for forged in [0, ticket.to_raw() + 2, u64::MAX] {
        assert!(matches!(
            bench.heartbeat.on_wake(WakeTicket::from_raw(forged)),
            WakeOutcome::Ignored(_)
        ));
    }
    assert_eq!(bench.published.count(), published);
    assert_eq!(bench.heartbeat.counters().ignored_wakes, 4);
    bench.beat();
}

#[test]
fn posting_one_wake_many_times_gives_one_beat() {
    let bench = Bench::running();
    bench.clock.advance(WAKE_DELAY);
    let ticket = bench.event_loop.take_due().expect("due");
    let before = bench.published.count();
    for _ in 0..10_000 {
        bench.heartbeat.on_wake(ticket);
    }
    assert_eq!(bench.published.count(), before + 1);
    assert_eq!(bench.event_loop.pending(), 1, "one chain, not one per post");
}

#[test]
fn a_loop_that_dispatches_inside_the_request_does_not_beat() {
    let (port, looped, published) = inline_port();
    port.start().expect("starts");
    assert_eq!(
        looped.outcome.get(),
        Some(WakeOutcome::Ignored(IgnoredWake::InsidePortCall))
    );
    assert_eq!(published.count(), 0);
    // The same ticket, out of the queue this time.
    assert!(matches!(
        port.on_wake(WakeTicket::from_raw(1)),
        WakeOutcome::Beat(_)
    ));
    assert_eq!(
        published.count(),
        1,
        "the wake it asks for next is inline again"
    );
    assert_eq!(
        port.counters().ignored_wakes,
        2,
        "every inline dispatch was ignored, the second as well as the first"
    );
}

#[test]
fn stopping_inside_a_request_withdraws_the_wake_just_taken() {
    let (port, looped, published) = inline_port();
    looped.stop_inside.set(true);
    port.start().expect("the loop took the wake");
    assert!(!port.is_running());
    assert!(looped.cancels.get() >= 2, "once by stop, once for the wake");
    assert_eq!(
        port.on_wake(WakeTicket::from_raw(1)),
        WakeOutcome::Ignored(IgnoredWake::NotRunning)
    );
    assert_eq!(published.count(), 0);
}

#[test]
fn a_lost_wake_is_replaced_by_a_turn_that_publishes_nothing() {
    let bench = Bench::running();
    bench.event_loop.lose_pending();
    let published = bench.published.count();

    assert!(!bench.heartbeat.on_loop_turn(), "not due yet");
    bench.clock.advance(WAKE_DELAY + LOST_WAKE_GRACE - STEP);
    assert!(!bench.heartbeat.on_loop_turn(), "late is not lost");
    bench.clock.advance(STEP);
    assert!(bench.heartbeat.on_loop_turn(), "replaced");
    assert!(!bench.heartbeat.on_loop_turn(), "once");
    assert_eq!(bench.published.count(), published);
    assert_eq!(bench.heartbeat.counters().replaced_wakes, 1);

    bench.beat();
    assert_eq!(bench.event_loop.pending(), 1);
}

#[test]
fn a_wake_that_was_only_late_still_counts_and_the_chains_merge() {
    let bench = Bench::running();
    let late = bench.event_loop.take_any().expect("in flight");
    bench.clock.advance(WAKE_DELAY + LOST_WAKE_GRACE);
    assert!(bench.heartbeat.on_loop_turn(), "a replacement is asked for");

    assert!(matches!(
        bench.heartbeat.on_wake(late),
        WakeOutcome::Beat(_)
    ));
    let replacement = bench.event_loop.take_due().expect("the replacement");
    assert!(matches!(
        bench.heartbeat.on_wake(replacement),
        WakeOutcome::Beat(_)
    ));
    assert_eq!(bench.event_loop.pending(), 1, "one wake in flight again");
    bench.beat();
}

/// A loop so slow that every wake is overdue when it is dispatched is still being serviced: replacing wakes must not
/// make the ones in flight worthless.
#[test]
fn a_slow_loop_keeps_beating() {
    let bench = Bench::running();
    let turn = Duration::from_millis(900);
    let before = bench.published.count();
    for _ in 0..40 {
        bench.clock.advance(turn);
        // Each turn dispatches another event first, then one wake.
        bench.heartbeat.on_loop_turn();
        if let Some(ticket) = bench.event_loop.take_due() {
            bench.heartbeat.on_wake(ticket);
        }
    }
    assert_eq!(bench.published.count(), before + 40, "one beat per turn");
    assert!(
        bench.event_loop.pending() <= 2,
        "wakes do not pile up: {}",
        bench.event_loop.pending()
    );
}

#[test]
fn a_refused_wake_costs_no_beat_and_is_asked_for_again() {
    let bench = Bench::running();
    bench.event_loop.refuse(Some(WakeRefused::Unavailable));
    bench.beat();
    let requests = bench.event_loop.requests();
    assert_eq!(bench.heartbeat.counters().refused_wakes, 1);

    for _ in 0..100 {
        assert!(!bench.heartbeat.on_loop_turn(), "spaced, not once per turn");
    }
    assert_eq!(bench.event_loop.requests(), requests);
    bench.clock.advance(REFUSED_WAKE_RETRY);
    assert!(!bench.heartbeat.on_loop_turn(), "refused again");
    assert_eq!(bench.event_loop.requests(), requests + 1);

    bench.event_loop.refuse(None);
    bench.clock.advance(REFUSED_WAKE_RETRY);
    assert!(bench.heartbeat.on_loop_turn());
    bench.beat();
}

#[test]
fn turns_and_resumes_publish_nothing() {
    let bench = Bench::running();
    let before = bench.published.count();
    for _ in 0..1000 {
        bench.clock.advance(STEP);
        bench.heartbeat.on_loop_turn();
    }
    bench.heartbeat.on_system_resume();
    assert_eq!(bench.published.count(), before);
}

/// Shutdown: wakes are withdrawn, a late one is ignored, no last beat is invented, and the sink comes back to be
/// dropped by the host after the last beat.
#[test]
fn stop_publishes_nothing_and_the_ring_outlives_the_last_beat() {
    let ring = RingFile::new("shutdown");
    let reader;
    {
        let writer = DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("ring");
        reader = DiagRingReader::open(ring.path()).expect("reader");
        let clock = ManualClock::default();
        let event_loop = ManualLoop::new(clock.clone());
        let heartbeat = UiHeartbeat::new(clock.clone(), event_loop.clone(), writer);
        heartbeat.start().expect("starts");
        heartbeat.on_wake(event_loop.take_due().expect("due"));
        clock.advance(WAKE_DELAY);
        let late = event_loop.take_due().expect("dispatched after the stop");
        let last = heartbeat.last_beat();

        heartbeat.stop();
        heartbeat.stop();
        assert_eq!(event_loop.cancels(), 1, "withdrawn once");
        assert!(!heartbeat.is_running());
        assert_eq!(
            heartbeat.on_wake(late),
            WakeOutcome::Ignored(IgnoredWake::NotRunning)
        );
        assert!(!heartbeat.on_loop_turn());
        assert!(!heartbeat.on_system_resume());
        assert_eq!(event_loop.pending(), 0);
        assert_eq!(heartbeat.last_beat(), last);

        let writer = heartbeat.into_sink();
        assert_eq!(reader.read_heartbeat(), last);
        drop(writer);
    }
    assert_eq!(
        reader.read_heartbeat().map(|beat| beat.counter),
        Some(1),
        "the last beat stays readable after the writer is gone"
    );
    drop(reader);
}

#[test]
fn into_sink_stops_first_and_dropping_publishes_nothing() {
    let bench = Bench::running();
    let before = bench.published.count();
    let returned = bench.heartbeat.into_sink();
    assert_eq!(bench.event_loop.cancels(), 1);
    assert_eq!(returned.count(), before);

    let bench = Bench::running();
    let before = bench.published.count();
    let published = bench.published.clone();
    drop(bench);
    assert_eq!(published.count(), before);
}

/// AC215-6, MT-084 AC-004-5: without a ring writer the port runs and writes nowhere.
#[test]
fn without_a_ring_writer_it_runs_and_stays_silent() {
    let clock = ManualClock::default();
    let event_loop = ManualLoop::new(clock.clone());
    let heartbeat = UiHeartbeat::new(clock.clone(), event_loop.clone(), None::<DiagRingWriter>);
    heartbeat.start().expect("starts");
    for counter in 1..=10 {
        clock.advance(WAKE_DELAY);
        let ticket = event_loop.take_due().expect("due");
        assert!(matches!(
            heartbeat.on_wake(ticket),
            WakeOutcome::Beat(beat) if beat.counter == counter
        ));
    }
    assert_eq!(heartbeat.counters().beats, 10);
}

static RECORDED_COUNTER: AtomicU64 = AtomicU64::new(0);
static RECORDED_NANOS: AtomicU64 = AtomicU64::new(0);

/// The shape of the diagnostics recorder's `heartbeat`.
fn recorder_heartbeat(counter: u64, timestamp_nanos: u64) {
    RECORDED_COUNTER.store(counter, Ordering::SeqCst);
    RECORDED_NANOS.store(timestamp_nanos, Ordering::SeqCst);
}

#[test]
fn the_recorder_gets_the_counter_first_and_the_time_second() {
    let clock = ManualClock::default();
    clock.set(777);
    let event_loop = ManualLoop::new(clock.clone());
    let heartbeat = UiHeartbeat::new(
        clock,
        event_loop.clone(),
        RecorderHeartbeat::new(recorder_heartbeat),
    );
    heartbeat.start().expect("starts");
    heartbeat.on_wake(event_loop.take_due().expect("due"));
    assert_eq!(RECORDED_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(RECORDED_NANOS.load(Ordering::SeqCst), 777);
}
