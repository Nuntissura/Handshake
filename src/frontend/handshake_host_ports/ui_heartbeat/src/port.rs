//! The heartbeat port itself.

use std::cell::Cell;
use std::marker::PhantomData;
use std::time::Duration;

use handshake_diag_ring::Heartbeat;

use crate::clock::MonotonicClock;
use crate::events::HeartbeatEvent;
use crate::frames::{FrameWait, NoFrames};
use crate::sink::{HeartbeatSink, PortToken};
use crate::wake::{EventLoopWake, WakeRefused, WakeTicket};
use crate::{
    LOST_WAKE_GRACE, MAX_BEAT_GAP, MAX_OUTSTANDING_WAKES, PRESENT_STALL, REFUSED_WAKE_RETRY,
    WAKE_DELAY,
};

/// Why [`UiHeartbeat::start`] did not start the heartbeat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartRefused {
    /// The calling thread does not dispatch the event loop. A heartbeat from here would prove nothing about the UI.
    NotLoopThread,
    /// Already running.
    AlreadyRunning,
    /// Already stopped, or the loop has ended; a stopped heartbeat stays silent.
    Stopped,
    /// The loop did not take the first wake. Nothing was published. Starting again is allowed after
    /// [`WakeRefused::Unavailable`]; after [`WakeRefused::LoopClosed`] the heartbeat is stopped.
    Wake(WakeRefused),
}

/// What became of one dispatched wake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeOutcome {
    /// The wake proved the loop is being serviced and this beat was published: every wake the port asked for and the
    /// loop dispatched, also while a present stalls (AC215-6; the stall is a typed event beside the beat, MT-215 HB-4).
    Beat(Heartbeat),
    /// Nothing was published.
    Ignored(IgnoredWake),
}

/// Why a dispatched wake published nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IgnoredWake {
    /// The port has not started or has stopped.
    NotRunning,
    /// The loop says this is not a wake it dispatched for the port (MT-215 HB-7).
    ForeignWake,
    /// The port never asked for this ticket.
    NeverRequested,
    /// This ticket, or a later one, was already dispatched.
    AlreadyRedeemed,
    /// The loop dispatched it inside one of the port's own calls instead of from its queue.
    InsidePortCall,
}

/// Counters since construction. Mechanical numbers only, for the host's diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeartbeatCounters {
    /// Beats published.
    pub beats: u64,
    /// Beats that came more than [`MAX_BEAT_GAP`] after the one before and were not explained by a system resume.
    pub overdue_beats: u64,
    /// The longest such gap, in nanoseconds.
    pub longest_gap_nanos: u64,
    /// First beats after a reported system resume.
    pub resumes: u64,
    /// Wakes requested again because one was lost or a resume was reported.
    pub replaced_wakes: u64,
    /// Replacements not asked for because [`MAX_OUTSTANDING_WAKES`] were already outstanding (MT-215 F4).
    pub held_back_wakes: u64,
    /// Wake requests the loop did not take.
    pub refused_wakes: u64,
    /// Dispatched wakes that published nothing.
    pub ignored_wakes: u64,
    /// Present stalls that began (MT-215 HB-4). Each is one typed event when it begins and one when it ends.
    pub present_stalls: u64,
    /// Beats for which the clock read earlier than at the beat before.
    pub clock_regressions: u64,
    /// The loop said it has ended; the port asks it for nothing more (MT-215 F4).
    pub loop_closed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Created,
    Running,
    Stopped,
}

/// The wakes in flight. Ticket numbers rise by one; two watermarks are enough to know every ticket's state.
#[derive(Clone, Copy, Default)]
struct Wakes {
    /// Highest ticket the loop has taken.
    requested: u64,
    /// Highest ticket the loop has dispatched back.
    redeemed: u64,
    /// Earliest time a loop turn may ask for a replacement.
    replace_at_nanos: u64,
    /// The next beat follows a reported system resume.
    resumed: bool,
}

/// A present stall under way.
#[derive(Clone, Copy)]
struct Stall {
    /// The longest wait seen.
    longest: Duration,
}

/// The UI-thread heartbeat.
///
/// Lives on the UI thread: it is neither `Send` nor `Sync`, so no other thread can publish through it, and
/// [`UiHeartbeat::start`] refuses a thread that does not dispatch the loop. See the crate documentation for what a
/// beat proves.
///
/// Every method takes `&self`, because a nested (modal) loop may call in again while an outer handler is still running.
/// Each is bounded and, with a clock, loop, sink and frames that keep their contracts, neither blocks, allocates nor
/// panics.
pub struct UiHeartbeat<C, L, S, F = NoFrames> {
    clock: C,
    event_loop: L,
    sink: S,
    frames: F,
    phase: Cell<Phase>,
    inside_call: Cell<bool>,
    wakes: Cell<Wakes>,
    last_beat: Cell<Heartbeat>,
    last_beat_read_nanos: Cell<u64>,
    stall: Cell<Option<Stall>>,
    events: Cell<u64>,
    counters: Cell<HeartbeatCounters>,
    ui_thread_only: PhantomData<*const ()>,
}

impl<C, L, S> UiHeartbeat<C, L, S, NoFrames>
where
    C: MonotonicClock,
    L: EventLoopWake,
    S: HeartbeatSink,
{
    /// A heartbeat that has not started and watches no frames. Publishes nothing and requests nothing yet.
    pub fn new(clock: C, event_loop: L, sink: S) -> Self {
        Self::with_frames(clock, event_loop, sink, NoFrames)
    }
}

impl<C, L, S, F> UiHeartbeat<C, L, S, F>
where
    C: MonotonicClock,
    L: EventLoopWake,
    S: HeartbeatSink,
    F: FrameWait,
{
    /// A heartbeat that has not started and records a typed event when `frames` reports a present stall, and another
    /// when it ends. Its beats do not depend on `frames` (AC215-6).
    pub fn with_frames(clock: C, event_loop: L, sink: S, frames: F) -> Self {
        Self {
            clock,
            event_loop,
            sink,
            frames,
            phase: Cell::new(Phase::Created),
            inside_call: Cell::new(false),
            wakes: Cell::new(Wakes::default()),
            last_beat: Cell::new(Heartbeat {
                counter: 0,
                timestamp_nanos: 0,
            }),
            last_beat_read_nanos: Cell::new(0),
            stall: Cell::new(None),
            events: Cell::new(0),
            counters: Cell::new(HeartbeatCounters::default()),
            ui_thread_only: PhantomData,
        }
    }

    /// Ask the loop for the first wake. The first beat is published when the loop dispatches it, not here.
    ///
    /// # Errors
    ///
    /// [`StartRefused`].
    pub fn start(&self) -> Result<(), StartRefused> {
        match self.phase.get() {
            Phase::Created => {}
            Phase::Running => return Err(StartRefused::AlreadyRunning),
            Phase::Stopped => return Err(StartRefused::Stopped),
        }
        if !self.event_loop.on_loop_thread() {
            return Err(StartRefused::NotLoopThread);
        }
        let Some(_call) = PortCall::enter(&self.inside_call) else {
            return Err(StartRefused::AlreadyRunning);
        };
        self.phase.set(Phase::Running);
        let requested = self.request_wake(self.clock.now_nanos(), Duration::ZERO);
        if requested.is_err() && self.phase.get() == Phase::Running {
            self.phase.set(Phase::Created);
        }
        requested.map_err(StartRefused::Wake)
    }

    /// The loop dispatched `wake`: publish a beat and ask for the next wake. A present stall that begins or ends here is
    /// recorded as a typed event beside the beat, never instead of it (AC215-6, MT-215 HB-4).
    ///
    /// `wake` is the loop's own proof of the dispatch ([`EventLoopWake::Dispatched`]); the loop says which ticket it
    /// carries, and a wake it did not dispatch for the port publishes nothing (MT-215 HB-7).
    pub fn on_wake(&self, wake: &L::Dispatched<'_>) -> WakeOutcome {
        let Some(_call) = PortCall::enter(&self.inside_call) else {
            return self.ignored(IgnoredWake::InsidePortCall);
        };
        if self.phase.get() != Phase::Running {
            return self.ignored(IgnoredWake::NotRunning);
        }
        let Some(ticket) = self.event_loop.dispatched_ticket(wake) else {
            return self.ignored(IgnoredWake::ForeignWake);
        };
        let mut wakes = self.wakes.get();
        if ticket.number() > wakes.requested {
            return self.ignored(IgnoredWake::NeverRequested);
        }
        if ticket.number() <= wakes.redeemed {
            return self.ignored(IgnoredWake::AlreadyRedeemed);
        }

        // The beat comes first: before the next request, so a loop that refuses the next wake cannot cost this one, and
        // before the frames are read, so nothing the frames wait on can delay it (MT-215 N2, the wait-free beat path:
        // the clock, the counters and the sink are all this thread touches before the beat is out). A present stall
        // changes what is recorded beside it, not whether it is published: a frozen UI thread goes stale, a stalled
        // present shows as a fresh heartbeat plus its typed event, and the watcher can tell the two apart.
        let now = self.clock.now_nanos();
        let beat = self.publish(now, wakes.resumed);
        self.track_present_stall(now);
        wakes.resumed = false;

        wakes.redeemed = ticket.number();
        wakes.replace_at_nanos = now.saturating_add(nanos(LOST_WAKE_GRACE));
        self.wakes.set(wakes);
        // A replacement still in flight carries the chain on; otherwise ask for the next wake. A refusal is counted
        // and retried by the next loop turn.
        if wakes.redeemed == wakes.requested {
            let _ = self.request_wake(now, WAKE_DELAY);
        }
        WakeOutcome::Beat(beat)
    }

    /// The loop dispatched something else (input, resize, redraw, a user event). Publishes nothing.
    ///
    /// Recovers a wake that was refused, or lost by a loop that broke its contract. Returns whether a replacement was
    /// taken by the loop.
    pub fn on_loop_turn(&self) -> bool {
        let Some(_call) = PortCall::enter(&self.inside_call) else {
            return false;
        };
        if self.phase.get() != Phase::Running {
            return false;
        }
        let now = self.clock.now_nanos();
        let mut wakes = self.wakes.get();
        // A clock that ran backward must not postpone recovery by the size of its jump.
        let latest = now.saturating_add(nanos(WAKE_DELAY.saturating_add(LOST_WAKE_GRACE)));
        if wakes.replace_at_nanos > latest {
            wakes.replace_at_nanos = latest;
            self.wakes.set(wakes);
        }
        now >= wakes.replace_at_nanos && self.replace_wake(now)
    }

    /// The system resumed from sleep. Publishes nothing: asks the loop for a wake at once, because the timer behind
    /// the previous one may not have survived, and marks the coming gap as a resume. Returns whether the loop took it.
    ///
    /// There is nothing to call before a sleep: the process and the watcher stop together.
    pub fn on_system_resume(&self) -> bool {
        let Some(_call) = PortCall::enter(&self.inside_call) else {
            return false;
        };
        if self.phase.get() != Phase::Running {
            return false;
        }
        let mut wakes = self.wakes.get();
        wakes.resumed = true;
        self.wakes.set(wakes);
        self.replace_wake(self.clock.now_nanos())
    }

    /// End the heartbeat for good and withdraw its wakes. Publishes nothing. A wake that still arrives is ignored.
    ///
    /// The watcher reads a heartbeat that ended as one that went stale, so end the watcher (or have its acknowledgement)
    /// within its threshold of this call.
    pub fn stop(&self) {
        let was_running = self.phase.replace(Phase::Stopped) == Phase::Running;
        if was_running {
            self.event_loop.cancel_wakes();
        }
    }

    /// Stop, then hand back the sink, so the ring writer inside it outlives the last beat and the host decides when it
    /// is dropped.
    pub fn into_sink(self) -> S {
        self.stop();
        self.sink
    }

    /// Whether beats are published for dispatched wakes.
    pub fn is_running(&self) -> bool {
        self.phase.get() == Phase::Running
    }

    /// The last published beat.
    pub fn last_beat(&self) -> Option<Heartbeat> {
        let beat = self.last_beat.get();
        (beat.counter != 0).then_some(beat)
    }

    /// The counters so far.
    pub fn counters(&self) -> HeartbeatCounters {
        self.counters.get()
    }

    /// Whether a present stall is under way, as of the last dispatched wake. Beats go on during one.
    pub fn present_stalled(&self) -> bool {
        self.stall.get().is_some()
    }

    /// The frames the port watches.
    pub fn frames(&self) -> &F {
        &self.frames
    }

    fn count(&self, change: impl FnOnce(&mut HeartbeatCounters)) {
        let mut counters = self.counters.get();
        change(&mut counters);
        self.counters.set(counters);
    }

    /// Records a present stall once when it begins and once when it ends. Decides nothing about the beat.
    fn track_present_stall(&self, now: u64) {
        let stalled = self
            .frames
            .longest_wait()
            .filter(|wait| *wait >= PRESENT_STALL);
        match (self.stall.get(), stalled) {
            (None, None) => {}
            (None, Some(waited)) => {
                self.stall.set(Some(Stall { longest: waited }));
                self.count(|counters| counters.present_stalls = counters.present_stalls.saturating_add(1));
                self.record(now, HeartbeatEvent::PresentStalled { waited });
            }
            (Some(stall), Some(waited)) => {
                self.stall.set(Some(Stall {
                    longest: stall.longest.max(waited),
                }));
            }
            (Some(stall), None) => {
                self.stall.set(None);
                self.record(
                    now,
                    HeartbeatEvent::PresentRecovered {
                        waited: stall.longest,
                    },
                );
            }
        }
    }

    fn record(&self, now: u64, event: HeartbeatEvent) {
        let sequence = self.events.get().saturating_add(1);
        self.events.set(sequence);
        self.sink
            .record(event.to_diag_event(sequence, now), &PortToken::new());
    }

    fn publish(&self, now: u64, resumed: bool) -> Heartbeat {
        let last = self.last_beat.get();
        let mut counters = self.counters.get();
        if self.last_beat().is_some() {
            let previous = self.last_beat_read_nanos.get();
            let gap = now.saturating_sub(previous);
            if resumed {
                counters.resumes = counters.resumes.saturating_add(1);
            } else if gap > nanos(MAX_BEAT_GAP) {
                counters.overdue_beats = counters.overdue_beats.saturating_add(1);
                counters.longest_gap_nanos = counters.longest_gap_nanos.max(gap);
            }
            if now < previous {
                counters.clock_regressions = counters.clock_regressions.saturating_add(1);
            }
        }
        counters.beats = counters.beats.saturating_add(1);

        let beat = Heartbeat {
            counter: next_counter(last.counter),
            // Strictly rising, also when the clock repeats a reading or runs backward.
            timestamp_nanos: now.max(last.timestamp_nanos.saturating_add(1)),
        };
        self.last_beat.set(beat);
        self.last_beat_read_nanos.set(now);
        self.counters.set(counters);
        self.sink.publish(beat, &PortToken::new());
        beat
    }

    /// Ask for a replacement wake, unless [`MAX_OUTSTANDING_WAKES`] are outstanding already (MT-215 F4): a loop that
    /// keeps its contract holds one and delivers it, so more would only pile up in a loop that takes nothing out.
    fn replace_wake(&self, now: u64) -> bool {
        let mut wakes = self.wakes.get();
        if wakes.requested.saturating_sub(wakes.redeemed) >= MAX_OUTSTANDING_WAKES {
            self.count(|counters| counters.held_back_wakes = counters.held_back_wakes.saturating_add(1));
            wakes.replace_at_nanos = now.saturating_add(nanos(LOST_WAKE_GRACE));
            self.wakes.set(wakes);
            return false;
        }
        self.count(|counters| counters.replaced_wakes = counters.replaced_wakes.saturating_add(1));
        self.request_wake(now, Duration::ZERO).is_ok()
    }

    /// Ask the loop for one more wake. Earlier tickets stay redeemable: a slow loop that dispatches them late is still
    /// being serviced. A loop that has ended stops the port for good.
    fn request_wake(&self, now: u64, delay: Duration) -> Result<(), WakeRefused> {
        let mut wakes = self.wakes.get();
        let ticket = wakes.requested.saturating_add(1);
        let taken = self.event_loop.request_wake(WakeTicket::new(ticket), delay);
        let replace_after = match taken {
            Ok(()) => {
                wakes.requested = ticket;
                delay.saturating_add(LOST_WAKE_GRACE)
            }
            Err(_) => {
                self.count(|counters| counters.refused_wakes = counters.refused_wakes.saturating_add(1));
                REFUSED_WAKE_RETRY
            }
        };
        wakes.replace_at_nanos = now.saturating_add(nanos(replace_after));
        self.wakes.set(wakes);
        if taken == Err(WakeRefused::LoopClosed) {
            // No wake will ever be dispatched: nothing to withdraw, and nothing to ask for again.
            self.phase.set(Phase::Stopped);
            self.count(|counters| counters.loop_closed = true);
        } else if self.phase.get() == Phase::Stopped {
            // Stopped from inside the request; the wake it just took must not outlive the stop.
            self.event_loop.cancel_wakes();
        }
        taken
    }

    fn ignored(&self, reason: IgnoredWake) -> WakeOutcome {
        self.count(|counters| counters.ignored_wakes = counters.ignored_wakes.saturating_add(1));
        WakeOutcome::Ignored(reason)
    }
}

/// Marks a port call in progress, and unmarks it however the call ends.
struct PortCall<'a>(&'a Cell<bool>);

impl<'a> PortCall<'a> {
    /// `None` while another call is in progress. Built lazily: dropping one marks the call as ended.
    fn enter(inside_call: &'a Cell<bool>) -> Option<Self> {
        (!inside_call.replace(true)).then(|| Self(inside_call))
    }
}

impl Drop for PortCall<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// The next counter: always different from the last one, and never 0, which the ring holds before any beat.
fn next_counter(last: u64) -> u64 {
    last.checked_add(1).unwrap_or(1)
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_wraps_to_one_so_it_keeps_changing_and_is_never_zero() {
        assert_eq!(next_counter(0), 1);
        assert_eq!(next_counter(41), 42);
        assert_eq!(next_counter(u64::MAX), 1);
    }

    #[test]
    fn a_duration_beyond_u64_nanoseconds_saturates() {
        assert_eq!(nanos(Duration::MAX), u64::MAX);
        assert_eq!(nanos(Duration::from_millis(250)), 250_000_000);
    }
}
