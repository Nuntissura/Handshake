//! The heartbeat port itself.

use std::cell::Cell;
use std::marker::PhantomData;
use std::time::Duration;

use handshake_diag_ring::Heartbeat;

use crate::clock::MonotonicClock;
use crate::sink::HeartbeatSink;
use crate::wake::{EventLoopWake, WakeRefused, WakeTicket};
use crate::{LOST_WAKE_GRACE, MAX_BEAT_GAP, REFUSED_WAKE_RETRY, WAKE_DELAY};

/// Why [`UiHeartbeat::start`] did not start the heartbeat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartRefused {
    /// The calling thread does not dispatch the event loop. A heartbeat from here would prove nothing about the UI.
    NotLoopThread,
    /// Already running.
    AlreadyRunning,
    /// Already stopped; a stopped heartbeat stays silent.
    Stopped,
    /// The loop did not take the first wake. Nothing was published; starting again is allowed.
    Wake(WakeRefused),
}

/// What became of one dispatched wake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeOutcome {
    /// The wake proved the loop is being serviced and this beat was published.
    Beat(Heartbeat),
    /// Nothing was published.
    Ignored(IgnoredWake),
}

/// Why a dispatched wake published nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IgnoredWake {
    /// The port has not started or has stopped.
    NotRunning,
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
    /// Wake requests the loop did not take.
    pub refused_wakes: u64,
    /// Dispatched wakes that published nothing.
    pub ignored_wakes: u64,
    /// Beats for which the clock read earlier than at the beat before.
    pub clock_regressions: u64,
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

/// The UI-thread heartbeat.
///
/// Lives on the UI thread: it is neither `Send` nor `Sync`, so no other thread can publish through it, and
/// [`UiHeartbeat::start`] refuses a thread that does not dispatch the loop. See the crate documentation for what a
/// beat proves.
///
/// Every method takes `&self`, because a nested (modal) loop may call in again while an outer handler is still running.
/// Each is bounded and, with a clock, loop and sink that keep their contracts, neither blocks, allocates nor panics.
pub struct UiHeartbeat<C, L, S> {
    clock: C,
    event_loop: L,
    sink: S,
    phase: Cell<Phase>,
    inside_call: Cell<bool>,
    wakes: Cell<Wakes>,
    last_beat: Cell<Heartbeat>,
    last_beat_read_nanos: Cell<u64>,
    counters: Cell<HeartbeatCounters>,
    ui_thread_only: PhantomData<*const ()>,
}

impl<C, L, S> UiHeartbeat<C, L, S>
where
    C: MonotonicClock,
    L: EventLoopWake,
    S: HeartbeatSink,
{
    /// A heartbeat that has not started. Publishes nothing and requests nothing yet.
    pub fn new(clock: C, event_loop: L, sink: S) -> Self {
        Self {
            clock,
            event_loop,
            sink,
            phase: Cell::new(Phase::Created),
            inside_call: Cell::new(false),
            wakes: Cell::new(Wakes::default()),
            last_beat: Cell::new(Heartbeat {
                counter: 0,
                timestamp_nanos: 0,
            }),
            last_beat_read_nanos: Cell::new(0),
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

    /// The loop dispatched `ticket`: publish a beat and ask for the next wake.
    pub fn on_wake(&self, ticket: WakeTicket) -> WakeOutcome {
        let Some(_call) = PortCall::enter(&self.inside_call) else {
            return self.ignored(IgnoredWake::InsidePortCall);
        };
        if self.phase.get() != Phase::Running {
            return self.ignored(IgnoredWake::NotRunning);
        }
        let mut wakes = self.wakes.get();
        if ticket.number() > wakes.requested {
            return self.ignored(IgnoredWake::NeverRequested);
        }
        if ticket.number() <= wakes.redeemed {
            return self.ignored(IgnoredWake::AlreadyRedeemed);
        }

        // The beat comes before the next request, so a loop that refuses the next wake cannot cost this one.
        let now = self.clock.now_nanos();
        let beat = self.publish(now, wakes.resumed);

        wakes.redeemed = ticket.number();
        wakes.resumed = false;
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
    /// Recovers a wake that was refused, or lost with the timer or window that carried it. Returns whether a
    /// replacement was taken by the loop.
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
        self.sink.publish(beat);
        beat
    }

    fn replace_wake(&self, now: u64) -> bool {
        let mut counters = self.counters.get();
        counters.replaced_wakes = counters.replaced_wakes.saturating_add(1);
        self.counters.set(counters);
        self.request_wake(now, Duration::ZERO).is_ok()
    }

    /// Ask the loop for one more wake. Earlier tickets stay redeemable: a slow loop that dispatches them late is still
    /// being serviced.
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
                let mut counters = self.counters.get();
                counters.refused_wakes = counters.refused_wakes.saturating_add(1);
                self.counters.set(counters);
                REFUSED_WAKE_RETRY
            }
        };
        wakes.replace_at_nanos = now.saturating_add(nanos(replace_after));
        self.wakes.set(wakes);
        if self.phase.get() == Phase::Stopped {
            // Stopped from inside the request; the wake it just took must not outlive the stop.
            self.event_loop.cancel_wakes();
        }
        taken
    }

    fn ignored(&self, reason: IgnoredWake) -> WakeOutcome {
        let mut counters = self.counters.get();
        counters.ignored_wakes = counters.ignored_wakes.saturating_add(1);
        self.counters.set(counters);
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
