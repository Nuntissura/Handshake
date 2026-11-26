//! A desk without a window: a hand-driven clock, a hand-driven event loop, and the real Palmistry ring reader and
//! freeze decision watching the real ring file.
//!
//! Time here is virtual, so nothing sleeps and every run is the same.
//!
//! Ring files live in Cargo's temporary folder of the test target, inside the artifact root, and are removed however a
//! test ends; every test keeps a time limit of its own ([`limits`]; MT-215 F5, E5, E7, CX-GIT-003).

#![allow(dead_code)]

pub mod limits;

pub use limits::{RingFile, TimeLimit};

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagRingWriter, Heartbeat, DEFAULT_CAPACITY};
use palmistry::freeze_detect::{FreezeDetector, FreezeState, POLL_INTERVAL};
use palmistry::hung_window_probe::{FakeHungWindowProbe, HungWindowProbe, ProbeResult};
use palmistry::ring_reader::PalmistryRingReader;
use ui_heartbeat::{
    EventLoopWake, FrameWait, MonotonicClock, UiHeartbeat, WakeOutcome, WakeRefused, WakeTicket,
};

/// How far the virtual clock moves per simulated step.
pub const STEP: Duration = Duration::from_millis(5);

/// The time limit of a test on the virtual clock: it sleeps nowhere, so it ends in well under a second when it keeps
/// its bounds, also under emulation.
pub const VIRTUAL_LIMIT: Duration = Duration::from_secs(60);

/// The most wakes one call of [`Desk::dispatch_due`] dispatches. A port that keeps its contract has at most
/// `MAX_OUTSTANDING_WAKES` due at once, and each asks for the next one later, not now.
pub const MOST_DUE_AT_ONCE: u64 = 64;

pub fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).expect("test durations fit")
}

#[derive(Clone, Default)]
pub struct ManualClock(Rc<Cell<u64>>);

impl ManualClock {
    pub fn now(&self) -> u64 {
        self.0.get()
    }

    pub fn set(&self, nanos: u64) {
        self.0.set(nanos);
    }

    pub fn advance(&self, by: Duration) {
        self.0.set(self.0.get() + nanos(by));
    }
}

impl MonotonicClock for ManualClock {
    fn now_nanos(&self) -> u64 {
        self.0.get()
    }
}

/// An event loop whose wakes are timers on the virtual clock. Nothing is dispatched until a test takes it.
#[derive(Clone)]
pub struct ManualLoop(Rc<LoopState>);

struct LoopState {
    clock: ManualClock,
    latency: Cell<Duration>,
    pending: RefCell<VecDeque<(u64, WakeTicket)>>,
    refusal: Cell<Option<WakeRefused>>,
    lose_requests: Cell<bool>,
    requests: Cell<u64>,
    cancels: Cell<u64>,
    last_delay: Cell<Duration>,
}

impl ManualLoop {
    pub fn new(clock: ManualClock) -> Self {
        Self(Rc::new(LoopState {
            clock,
            latency: Cell::new(Duration::ZERO),
            pending: RefCell::new(VecDeque::with_capacity(64)),
            refusal: Cell::new(None),
            lose_requests: Cell::new(false),
            requests: Cell::new(0),
            cancels: Cell::new(0),
            last_delay: Cell::new(Duration::ZERO),
        }))
    }

    /// Dispatch wakes this long after they are due.
    pub fn set_latency(&self, latency: Duration) {
        self.0.latency.set(latency);
    }

    /// Refuse wake requests.
    pub fn refuse(&self, refusal: Option<WakeRefused>) {
        self.0.refusal.set(refusal);
    }

    /// Take wake requests and lose them.
    pub fn lose_requests(&self, lose: bool) {
        self.0.lose_requests.set(lose);
    }

    /// Lose the wakes in flight, as a destroyed timer does.
    pub fn lose_pending(&self) {
        self.0.pending.borrow_mut().clear();
    }

    pub fn pending(&self) -> usize {
        self.0.pending.borrow().len()
    }

    pub fn requests(&self) -> u64 {
        self.0.requests.get()
    }

    pub fn cancels(&self) -> u64 {
        self.0.cancels.get()
    }

    pub fn last_delay(&self) -> Duration {
        self.0.last_delay.get()
    }

    /// The oldest wake, if it is due.
    pub fn take_due(&self) -> Option<WakeTicket> {
        let mut pending = self.0.pending.borrow_mut();
        match pending.front() {
            Some(&(due, _)) if due <= self.0.clock.now() => pending.pop_front().map(|(_, t)| t),
            _ => None,
        }
    }

    /// The oldest wake, due or not: an operating-system timer does not read our clock.
    pub fn take_any(&self) -> Option<WakeTicket> {
        self.0.pending.borrow_mut().pop_front().map(|(_, t)| t)
    }
}

impl EventLoopWake for ManualLoop {
    type Dispatched<'a> = WakeTicket;

    fn dispatched_ticket(&self, wake: &WakeTicket) -> Option<WakeTicket> {
        Some(*wake)
    }

    fn on_loop_thread(&self) -> bool {
        true
    }

    fn request_wake(&self, ticket: WakeTicket, delay: Duration) -> Result<(), WakeRefused> {
        let state = &self.0;
        state.requests.set(state.requests.get() + 1);
        state.last_delay.set(delay);
        if let Some(refusal) = state.refusal.get() {
            return Err(refusal);
        }
        if !state.lose_requests.get() {
            let due = state.clock.now() + nanos(delay) + nanos(state.latency.get());
            state.pending.borrow_mut().push_back((due, ticket));
        }
        Ok(())
    }

    fn cancel_wakes(&self) {
        self.0.cancels.set(self.0.cancels.get() + 1);
        self.0.pending.borrow_mut().clear();
    }
}

/// Frames of a host on the virtual clock: one is wanted since a time, or none.
#[derive(Clone)]
pub struct ManualFrames {
    clock: ManualClock,
    since: Rc<Cell<Option<u64>>>,
}

impl ManualFrames {
    pub fn new(clock: ManualClock) -> Self {
        Self {
            clock,
            since: Rc::default(),
        }
    }

    /// A frame is wanted from now on, and none is presented.
    pub fn want(&self) {
        self.since.set(Some(self.clock.now()));
    }

    /// The wanted frame was presented.
    pub fn presented(&self) {
        self.since.set(None);
    }
}

impl FrameWait for ManualFrames {
    fn longest_wait(&self) -> Option<Duration> {
        self.since
            .get()
            .map(|since| Duration::from_nanos(self.clock.now().saturating_sub(since)))
    }
}

/// Palmistry's own reader and decision, on the virtual clock.
pub struct Watcher {
    reader: PalmistryRingReader,
    detector: FreezeDetector,
    base: Instant,
    next_poll: u64,
}

impl Watcher {
    pub fn open(path: &Path) -> Self {
        Self {
            reader: PalmistryRingReader::open(path).expect("the watcher maps the ring"),
            detector: FreezeDetector::new(),
            base: Instant::now(),
            next_poll: 0,
        }
    }

    pub fn heartbeat(&self) -> Heartbeat {
        self.reader
            .read_heartbeat()
            .expect("a ring with one writer reads cleanly")
    }

    pub fn poll(&mut self, now: u64, probe: &dyn HungWindowProbe) -> FreezeState {
        self.detector.poll(
            self.base + Duration::from_nanos(now),
            self.reader.read_heartbeat(),
            probe,
        )
    }

    fn poll_if_due(&mut self, now: u64, probe: &dyn HungWindowProbe) -> Option<FreezeState> {
        if now < self.next_poll {
            return None;
        }
        self.next_poll = now + nanos(POLL_INTERVAL);
        Some(self.poll(now, probe))
    }
}

/// What the UI thread does while time passes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ui {
    /// The loop dispatches its wakes.
    Dispatching,
    /// The thread is stuck in a handler: nothing is dispatched.
    Blocked,
    /// The thread draws and handles input but never takes a wake out of its queue.
    TurnsOnly,
}

/// One watcher verdict and when it was given.
#[derive(Clone, Copy, Debug)]
pub struct Verdict {
    pub at: u64,
    pub state: FreezeState,
}

pub type DeskHeartbeat = UiHeartbeat<ManualClock, ManualLoop, DiagRingWriter, ManualFrames>;

/// A UI thread, its event loop, its heartbeat and the watcher.
pub struct Desk {
    pub heartbeat: DeskHeartbeat,
    pub event_loop: ManualLoop,
    pub clock: ManualClock,
    pub frames: ManualFrames,
    pub watcher: Watcher,
    /// Longest time between two beats of the last run, in nanoseconds.
    pub longest_gap: u64,
    last_beat_at: u64,
    pub ring: RingFile,
}

impl Desk {
    pub fn new(label: &str) -> Self {
        let ring = RingFile::new(label);
        let writer =
            DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("the ring is created");
        let clock = ManualClock::default();
        let event_loop = ManualLoop::new(clock.clone());
        let frames = ManualFrames::new(clock.clone());
        Self {
            heartbeat: UiHeartbeat::with_frames(
                clock.clone(),
                event_loop.clone(),
                writer,
                frames.clone(),
            ),
            event_loop,
            clock,
            frames,
            watcher: Watcher::open(ring.path()),
            longest_gap: 0,
            last_beat_at: 0,
            ring,
        }
    }

    /// A desk whose loop has already served its first wakes.
    pub fn running(label: &str) -> Self {
        let mut desk = Self::new(label);
        desk.heartbeat.start().expect("starts on the loop thread");
        desk.run(
            Duration::from_secs(2),
            Ui::Dispatching,
            ProbeResult::Responding,
        );
        assert!(desk.heartbeat.last_beat().is_some(), "the desk is alive");
        desk
    }

    /// Dispatch every wake that is due, at most [`MOST_DUE_AT_ONCE`]. Returns the beats published.
    pub fn dispatch_due(&mut self) -> u64 {
        let mut beats = 0;
        let mut dispatched = 0;
        while let Some(ticket) = self.event_loop.take_due() {
            dispatched += 1;
            assert!(
                dispatched <= MOST_DUE_AT_ONCE,
                "wakes keep coming due at once: the port asks for each next wake now"
            );
            if let WakeOutcome::Beat(_) = self.heartbeat.on_wake(&ticket) {
                beats += 1;
                let now = self.clock.now();
                self.longest_gap = self.longest_gap.max(now.saturating_sub(self.last_beat_at));
                self.last_beat_at = now;
            }
        }
        beats
    }

    /// Let `span` pass. Returns every verdict the watcher gave.
    pub fn run(&mut self, span: Duration, ui: Ui, probe: ProbeResult) -> Vec<Verdict> {
        let probe = FakeHungWindowProbe::new(probe);
        let end = self.clock.now() + nanos(span);
        self.longest_gap = 0;
        self.last_beat_at = self.clock.now();
        let mut verdicts = Vec::new();
        while self.clock.now() < end {
            self.clock.advance(STEP);
            match ui {
                Ui::Dispatching => {
                    self.dispatch_due();
                }
                Ui::Blocked => {}
                Ui::TurnsOnly => {
                    self.heartbeat.on_loop_turn();
                }
            }
            let now = self.clock.now();
            if let Some(state) = self.watcher.poll_if_due(now, &probe) {
                verdicts.push(Verdict { at: now, state });
            }
        }
        verdicts
    }
}

pub fn all_healthy(verdicts: &[Verdict]) -> bool {
    !verdicts.is_empty()
        && verdicts
            .iter()
            .all(|verdict| verdict.state == FreezeState::Healthy)
}
