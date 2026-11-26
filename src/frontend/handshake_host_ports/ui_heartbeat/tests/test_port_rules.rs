//! What the port accepts, ignores, recovers and publishes, and in which order it ends.

mod support;

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use handshake_diag_ring::{
    DiagEvent, DiagEventCode, DiagPhase, DiagRingReader, DiagRingWriter, DiagSeverity, Heartbeat,
    DEFAULT_CAPACITY,
};
use support::{
    nanos, ManualClock, ManualFrames, ManualLoop, RingFile, TimeLimit, STEP, VIRTUAL_LIMIT,
};
use ui_heartbeat::{
    EventLoopWake, HeartbeatSink, IgnoredWake, PortToken, ProcessClock, RecorderHeartbeat,
    StartRefused, UiHeartbeat, WakeOutcome, WakeRefused, WakeTicket, HANDLER_ALLOWANCE,
    LOST_WAKE_GRACE, MAX_BEAT_GAP, MAX_OUTSTANDING_WAKES, PRESENT_STALL, REFUSED_WAKE_RETRY,
    TIMER_GRANULARITY, WAKE_DELAY,
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
    type Dispatched<'a> = WakeTicket;
    fn dispatched_ticket(&self, wake: &WakeTicket) -> Option<WakeTicket> {
        Some(*wake)
    }
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

/// Keeps what was published, and the events recorded.
#[derive(Clone, Default)]
struct Published(Rc<RefCell<Vec<Heartbeat>>>, Rc<RefCell<Vec<DiagEvent>>>);

impl Published {
    fn count(&self) -> usize {
        self.0.borrow().len()
    }

    fn events(&self) -> Vec<DiagEvent> {
        self.1.borrow().clone()
    }
}

impl HeartbeatSink for Published {
    fn publish(&self, beat: Heartbeat, _: &PortToken) {
        self.0.borrow_mut().push(beat);
    }

    fn record(&self, event: DiagEvent, _: &PortToken) {
        self.1.borrow_mut().push(event);
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
        match self.heartbeat.on_wake(&ticket) {
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
    type Dispatched<'a> = WakeTicket;
    fn dispatched_ticket(&self, wake: &WakeTicket) -> Option<WakeTicket> {
        Some(*wake)
    }
    fn on_loop_thread(&self) -> bool {
        true
    }
    fn request_wake(&self, ticket: WakeTicket, _: Duration) -> Result<(), WakeRefused> {
        if let Some(port) = self.0.port.borrow().upgrade() {
            if self.0.stop_inside.get() {
                port.stop();
            } else {
                self.0.outcome.set(Some(port.on_wake(&ticket)));
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let bench = Bench::running();
    let (ticket, _) = bench.beat();
    let published = bench.published.count();

    assert_eq!(
        bench.heartbeat.on_wake(&ticket),
        WakeOutcome::Ignored(IgnoredWake::AlreadyRedeemed)
    );
    for forged in [0, ticket.to_raw() + 2, u64::MAX] {
        assert!(matches!(
            bench.heartbeat.on_wake(&WakeTicket::from_raw(forged)),
            WakeOutcome::Ignored(_)
        ));
    }
    assert_eq!(bench.published.count(), published);
    assert_eq!(bench.heartbeat.counters().ignored_wakes, 4);
    bench.beat();
}

#[test]
fn posting_one_wake_many_times_gives_one_beat() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let bench = Bench::running();
    bench.clock.advance(WAKE_DELAY);
    let ticket = bench.event_loop.take_due().expect("due");
    let before = bench.published.count();
    for _ in 0..10_000 {
        bench.heartbeat.on_wake(&ticket);
    }
    assert_eq!(bench.published.count(), before + 1);
    assert_eq!(bench.event_loop.pending(), 1, "one chain, not one per post");
}

#[test]
fn a_loop_that_dispatches_inside_the_request_does_not_beat() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let (port, looped, published) = inline_port();
    port.start().expect("starts");
    assert_eq!(
        looped.outcome.get(),
        Some(WakeOutcome::Ignored(IgnoredWake::InsidePortCall))
    );
    assert_eq!(published.count(), 0);
    // The same ticket, out of the queue this time.
    assert!(matches!(
        port.on_wake(&WakeTicket::from_raw(1)),
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let (port, looped, published) = inline_port();
    looped.stop_inside.set(true);
    port.start().expect("the loop took the wake");
    assert!(!port.is_running());
    assert!(looped.cancels.get() >= 2, "once by stop, once for the wake");
    assert_eq!(
        port.on_wake(&WakeTicket::from_raw(1)),
        WakeOutcome::Ignored(IgnoredWake::NotRunning)
    );
    assert_eq!(published.count(), 0);
}

#[test]
fn a_lost_wake_is_replaced_by_a_turn_that_publishes_nothing() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let bench = Bench::running();
    let late = bench.event_loop.take_any().expect("in flight");
    bench.clock.advance(WAKE_DELAY + LOST_WAKE_GRACE);
    assert!(bench.heartbeat.on_loop_turn(), "a replacement is asked for");

    assert!(matches!(
        bench.heartbeat.on_wake(&late),
        WakeOutcome::Beat(_)
    ));
    let replacement = bench.event_loop.take_due().expect("the replacement");
    assert!(matches!(
        bench.heartbeat.on_wake(&replacement),
        WakeOutcome::Beat(_)
    ));
    assert_eq!(bench.event_loop.pending(), 1, "one wake in flight again");
    bench.beat();
}

/// A loop so slow that every wake is overdue when it is dispatched is still being serviced: replacing wakes must not
/// make the ones in flight worthless.
#[test]
fn a_slow_loop_keeps_beating() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let bench = Bench::running();
    let turn = Duration::from_millis(900);
    let before = bench.published.count();
    for _ in 0..40 {
        bench.clock.advance(turn);
        // Each turn dispatches another event first, then one wake.
        bench.heartbeat.on_loop_turn();
        if let Some(ticket) = bench.event_loop.take_due() {
            bench.heartbeat.on_wake(&ticket);
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let ring = RingFile::new("shutdown");
    let reader;
    {
        let writer = DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("ring");
        reader = DiagRingReader::open(ring.path()).expect("reader");
        let clock = ManualClock::default();
        let event_loop = ManualLoop::new(clock.clone());
        let heartbeat = UiHeartbeat::new(clock.clone(), event_loop.clone(), writer);
        heartbeat.start().expect("starts");
        heartbeat.on_wake(&event_loop.take_due().expect("due"));
        clock.advance(WAKE_DELAY);
        let late = event_loop.take_due().expect("dispatched after the stop");
        let last = heartbeat.last_beat();

        heartbeat.stop();
        heartbeat.stop();
        assert_eq!(event_loop.cancels(), 1, "withdrawn once");
        assert!(!heartbeat.is_running());
        assert_eq!(
            heartbeat.on_wake(&late),
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let clock = ManualClock::default();
    let event_loop = ManualLoop::new(clock.clone());
    let heartbeat = UiHeartbeat::new(clock.clone(), event_loop.clone(), None::<DiagRingWriter>);
    heartbeat.start().expect("starts");
    for counter in 1..=10 {
        clock.advance(WAKE_DELAY);
        let ticket = event_loop.take_due().expect("due");
        assert!(matches!(
            heartbeat.on_wake(&ticket),
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
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let clock = ManualClock::default();
    clock.set(777);
    let event_loop = ManualLoop::new(clock.clone());
    let heartbeat = UiHeartbeat::new(
        clock,
        event_loop.clone(),
        RecorderHeartbeat::new(recorder_heartbeat, || true),
    );
    heartbeat.start().expect("starts");
    heartbeat.on_wake(&event_loop.take_due().expect("due"));
    assert_eq!(RECORDED_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(RECORDED_NANOS.load(Ordering::SeqCst), 777);
}

/// A loop that hands out a proof with each dispatch and says whether it dispatched that wake for the port.
#[derive(Clone)]
struct Vouching(ManualLoop);

/// A dispatched wake as this loop hands it out: the ticket it carries, and whether the loop dispatched it for the port.
struct Proof {
    ticket: WakeTicket,
    for_the_port: bool,
}

impl EventLoopWake for Vouching {
    type Dispatched<'a> = Proof;
    fn dispatched_ticket(&self, wake: &Proof) -> Option<WakeTicket> {
        wake.for_the_port.then_some(wake.ticket)
    }
    fn on_loop_thread(&self) -> bool {
        true
    }
    fn request_wake(&self, ticket: WakeTicket, delay: Duration) -> Result<(), WakeRefused> {
        self.0.request_wake(ticket, delay)
    }
    fn cancel_wakes(&self) {
        self.0.cancel_wakes();
    }
}

/// MT-215 HB-7: the port takes the loop's proof of a dispatch, not a ticket. A wake that carries the very ticket the
/// port waits for, but that the loop did not dispatch for it, publishes nothing and leaves the ticket redeemable.
#[test]
fn a_wake_the_loop_does_not_vouch_for_publishes_nothing() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let clock = ManualClock::default();
    let event_loop = ManualLoop::new(clock.clone());
    let published = Published::default();
    let heartbeat = UiHeartbeat::new(
        clock.clone(),
        Vouching(event_loop.clone()),
        published.clone(),
    );
    heartbeat.start().expect("starts");
    let ticket = event_loop.take_due().expect("due");

    let foreign = Proof {
        ticket,
        for_the_port: false,
    };
    for _ in 0..3 {
        assert_eq!(
            heartbeat.on_wake(&foreign),
            WakeOutcome::Ignored(IgnoredWake::ForeignWake)
        );
    }
    assert_eq!(published.count(), 0);
    assert_eq!(heartbeat.counters().ignored_wakes, 3);
    let own = Proof {
        ticket,
        for_the_port: true,
    };
    assert!(matches!(
        heartbeat.on_wake(&own),
        WakeOutcome::Beat(beat) if beat.counter == 1
    ));
}

/// MT-215 F4: a loop that has ended is asked nothing more, however many turns and resumes follow.
#[test]
fn a_closed_loop_ends_the_heartbeat_and_is_asked_nothing_more() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let bench = Bench::running();
    bench.event_loop.refuse(Some(WakeRefused::LoopClosed));
    bench.beat();
    let requests = bench.event_loop.requests();
    assert!(!bench.heartbeat.is_running(), "the port stopped");
    assert!(bench.heartbeat.counters().loop_closed);
    for _ in 0..10_000 {
        bench.clock.advance(Duration::from_secs(1));
        assert!(!bench.heartbeat.on_loop_turn());
        assert!(!bench.heartbeat.on_system_resume());
    }
    assert_eq!(
        bench.event_loop.requests(),
        requests,
        "a closed loop was asked again"
    );
    assert_eq!(bench.heartbeat.start(), Err(StartRefused::Stopped));

    // A loop that has ended before the first wake: refused once, then stopped.
    let bench = Bench::new();
    bench.event_loop.refuse(Some(WakeRefused::LoopClosed));
    assert_eq!(
        bench.heartbeat.start(),
        Err(StartRefused::Wake(WakeRefused::LoopClosed))
    );
    assert_eq!(bench.heartbeat.start(), Err(StartRefused::Stopped));
    assert_eq!(bench.event_loop.requests(), 1);
}

/// MT-215 F4: a loop that takes nothing out of its queue for an hour holds at most `MAX_OUTSTANDING_WAKES` of the
/// port's wakes, however many turns and resumes come meanwhile; once it dispatches one, the chain goes on.
#[test]
fn outstanding_wakes_are_bounded_however_long_the_loop_takes_nothing_out() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let bench = Bench::running();
    assert_eq!(bench.event_loop.pending(), 1);
    for turn in 0..72_000 {
        bench.clock.advance(Duration::from_millis(50));
        bench.heartbeat.on_loop_turn();
        if turn % 72 == 0 {
            bench.heartbeat.on_system_resume();
        }
    }
    assert_eq!(
        bench.event_loop.pending() as u64,
        MAX_OUTSTANDING_WAKES,
        "wakes outstanding after an hour"
    );
    let counters = bench.heartbeat.counters();
    assert_eq!(counters.replaced_wakes, MAX_OUTSTANDING_WAKES - 1);
    assert!(counters.held_back_wakes > 1_000, "{counters:?}");

    let oldest = bench.event_loop.take_any().expect("in flight");
    assert!(matches!(
        bench.heartbeat.on_wake(&oldest),
        WakeOutcome::Beat(_)
    ));
    let newest = std::iter::from_fn(|| bench.event_loop.take_any())
        .last()
        .expect("in flight");
    assert!(matches!(
        bench.heartbeat.on_wake(&newest),
        WakeOutcome::Beat(_)
    ));
    bench.beat();
    assert_eq!(bench.event_loop.pending(), 1, "one chain again");
}

/// AC215-6 with MT-215 HB-4: while a frame has waited `PRESENT_STALL` without a present, every dispatched wake still
/// beats; the stall is a typed event beside the beats, recorded once when it begins and once when it ends, with only
/// the fields `SlowFrame` documents (E4).
#[test]
fn a_present_stall_keeps_every_beat_and_is_recorded_once_each_way() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let clock = ManualClock::default();
    clock.set(nanos(Duration::from_secs(1)));
    let event_loop = ManualLoop::new(clock.clone());
    let published = Published::default();
    let frames = ManualFrames::new(clock.clone());
    let heartbeat = UiHeartbeat::with_frames(
        clock.clone(),
        event_loop.clone(),
        published.clone(),
        frames.clone(),
    );
    heartbeat.start().expect("starts");
    let dispatch = || {
        clock.advance(WAKE_DELAY);
        heartbeat.on_wake(&event_loop.take_due().expect("a wake is due"))
    };
    assert!(matches!(dispatch(), WakeOutcome::Beat(_)));

    frames.want();
    // Bounded (MT-215 E5): the stall begins within `PRESENT_STALL` of the want, a wake every `WAKE_DELAY`.
    let most = PRESENT_STALL.as_millis() / WAKE_DELAY.as_millis() + 2;
    let mut beats_before_the_stall = 0;
    for _ in 0..most {
        let outcome = dispatch();
        assert!(
            matches!(outcome, WakeOutcome::Beat(_)),
            "every dispatched wake beats (AC215-6): {outcome:?}"
        );
        if heartbeat.present_stalled() {
            break;
        }
        beats_before_the_stall += 1;
    }
    assert!(heartbeat.present_stalled(), "the stall began within {most} wakes");
    assert!(
        beats_before_the_stall >= 18,
        "a stall only after PRESENT_STALL: {beats_before_the_stall}"
    );
    let events = published.events();
    assert_eq!(events.len(), 1, "the stall is recorded when it begins");
    let stalled = events[0];
    assert_eq!(stalled.event_code, DiagEventCode::SlowFrame.as_u16());
    assert_eq!(stalled.phase_marker, DiagPhase::Degraded.as_u8());
    assert_eq!(stalled.severity, DiagSeverity::Warn.as_u8());
    assert!(stalled.metric_micros >= nanos(PRESENT_STALL) / 1_000);
    assert!(stalled.metric_micros < nanos(PRESENT_STALL + WAKE_DELAY + WAKE_DELAY) / 1_000);
    assert_eq!(
        (stalled.counter_a, stalled.counter_b),
        (0, 0),
        "SlowFrame's counter_a is a frame index, which the port does not know (MT-215 E4)"
    );

    let last = heartbeat.last_beat().expect("a beat at the stall's start");
    let published_before = published.count();
    for _ in 0..30 {
        let outcome = dispatch();
        assert!(
            matches!(outcome, WakeOutcome::Beat(_)),
            "a stalled present keeps the heartbeat fresh (AC215-6): {outcome:?}"
        );
    }
    assert_eq!(published.count(), published_before + 30, "a beat for every dispatched wake");
    assert_eq!(heartbeat.last_beat().map(|beat| beat.counter), Some(last.counter + 30));
    assert!(heartbeat.present_stalled());
    assert_eq!(event_loop.pending(), 1, "the chain goes on");
    assert_eq!(published.events().len(), 1, "the stall is recorded once");

    frames.presented();
    let WakeOutcome::Beat(beat) = dispatch() else {
        panic!("a beat once the frame was presented");
    };
    assert_eq!(beat.counter, last.counter + 31);
    let events = published.events();
    assert_eq!(events.len(), 2, "and once when it ends");
    assert_eq!(events[1].event_code, DiagEventCode::SlowFrame.as_u16());
    assert_eq!(events[1].phase_marker, DiagPhase::Recovered.as_u8());
    assert_eq!(events[1].severity, DiagSeverity::Info.as_u8());
    assert_eq!((events[1].counter_a, events[1].counter_b), (0, 0));
    assert!(events[1].metric_micros >= stalled.metric_micros + 3_000_000, "the longest wait seen");
    assert!(events[1].sequence_id > stalled.sequence_id);
    let counters = heartbeat.counters();
    assert_eq!(counters.present_stalls, 1);
    assert_eq!(counters.beats, published.count() as u64);
    assert_eq!(counters.overdue_beats, 0, "no gap: beats never stopped");
    assert!(!heartbeat.present_stalled());
}

/// MT-215 F3: a beat stays within `MAX_BEAT_GAP` when the wake is rounded up by the timer and a handler of up to
/// `HANDLER_ALLOWANCE` runs when it comes due; with the earlier 200 ms delay it did not beyond about 35 ms.
#[test]
fn the_next_wake_leaves_room_for_the_timer_and_a_handler() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    assert!(HANDLER_ALLOWANCE >= Duration::from_millis(100));
    let bench = Bench::running();
    assert_eq!(bench.event_loop.last_delay(), WAKE_DELAY);
    bench
        .event_loop
        .set_latency(TIMER_GRANULARITY + HANDLER_ALLOWANCE);
    let mut at = bench.clock.now();
    // Bounded (MT-215 E5): a wake that is not due a step past `MAX_BEAT_GAP` fails the gap below anyway.
    let steps = MAX_BEAT_GAP.as_millis() / STEP.as_millis() + 2;
    for _ in 0..100 {
        let mut beat = false;
        for _ in 0..steps {
            bench.clock.advance(STEP);
            if let Some(ticket) = bench.event_loop.take_due() {
                assert!(matches!(
                    bench.heartbeat.on_wake(&ticket),
                    WakeOutcome::Beat(_)
                ));
                beat = true;
                break;
            }
        }
        assert!(beat, "no wake came due within {steps} steps");
        let gap = Duration::from_nanos(bench.clock.now() - at);
        assert!(gap <= MAX_BEAT_GAP, "{gap:?}");
        at = bench.clock.now();
    }
    assert_eq!(bench.heartbeat.counters().overdue_beats, 0);
}
