//! The UI heartbeat on the shell's loop and present path (MT-215 slice 2).
//!
//! The port (`ui_heartbeat`) asks a loop for wakes and publishes a beat for each one the loop dispatches. Here the loop
//! is one wake lane of the window crate, which keeps the port's contract by construction: an accepted wake is posted
//! again until the loop dispatches it, with no more than four posts while the loop takes nothing out (MT-215 F1, F4);
//! a newer request replaces one not dispatched; a message that anybody else posts or sends dispatches nothing; and the
//! proof of a dispatch, [`DispatchedWake`], only the loop can make, so no handler can stand in for it (HB-7). The frames
//! are the surface crate's present watches (HB-4): once a watched window has wanted a frame for
//! `ui_heartbeat::PRESENT_STALL` without a present, the port records a typed stall event, and another when it ends.
//! Beats go on meanwhile: every dispatched wake of the heartbeat's lane is a beat while the loop is alive (AC215-6).
//!
//! What reads the result: the standalone Palmistry watcher reads the ring's heartbeat slot, which goes stale when the UI
//! thread freezes (a confirmed freeze once the window does not answer either); the internal diagnostics and
//! Palmistry's survivor record read the typed stall events in the same ring (`ui_heartbeat::HeartbeatEvent`). A stale
//! heartbeat is a frozen UI thread; a fresh heartbeat beside a stall event is a stalled present.
//!
//! # How a host uses it
//!
//! 1. On `Event::LoopStarted`, after the ring writer or the recorder exists: [`ShellHeartbeat::open`] with the sink,
//!    then [`ShellHeartbeat::start`]. One per loop.
//! 2. In `ShellApp::on_event` call [`ShellHeartbeat::on_shell_event`] (with the window crate alone,
//!    [`ShellHeartbeat::on_event`] and [`ShellHeartbeat::watch`]): it watches every window that presents, forgets it
//!    when it is destroyed, takes resumes and stops at `Event::LoopEnding`.
//! 3. In `ShellApp::on_wake` call [`ShellHeartbeat::on_wake`] first: `Some` means the wake was the heartbeat's, `None`
//!    that it belongs to another lane.
//! 4. End the watcher while the loop still runs (`Event::ShutdownBegan`; MT-215 HB-3), then let the loop end.
//!
//! # One heartbeat per loop (MT-215 E3)
//!
//! The ring's heartbeat slot takes one publisher, and the sink is the port's for good. [`ShellHeartbeat::open`]
//! refuses a second heartbeat on a loop while the first lives ([`OpenRefused::AlreadyOpen`]); the port reaches its sink
//! only with a token only the port makes (`ui_heartbeat::PortToken`); and `tests/source_scan.rs` reads every workspace
//! member from the manifests and fails on any other writer of the slot, a second port or a second ring writer in
//! product code.
//!
//! # What a beat waits for (MT-215 E6, N2)
//!
//! Nothing. The beat path allocates nothing (`tests/heartbeat_loop.rs` and `tests/heartbeat_present.rs` count it) and
//! takes no lock (`tests/beat_path_locks.rs` scans it), so it is wait-free in the sense AC215-6 asks for: the port
//! publishes the beat before it reads the frames; it reads each watched window's wait with one atomic load
//! (`PresentWatch::frame_wanted_for`); and it asks the wake lane for the next wake with `try_lock` and an atomic
//! hand-off, so a pool thread that holds the lane's slot, which it does for arithmetic only and never across a call into
//! the system, costs the beat nothing: what the beat wanted is left for that thread, which does it before it lets go.
//! The one thing a held slot can delay is a dispatch (the loop's thread posts the message again and takes it a moment
//! later), never a beat that was dispatched.

use std::cell::Cell;
use std::marker::PhantomData;
use std::time::Instant;

use handshake_shell_surface::{PresentWatch, Shell};
use handshake_shell_window::{
    Control, DispatchedWake, Event, LaneId, WakeLane, WakeRefused as LaneRefused, WindowEvent,
    WindowKey,
};
use ui_heartbeat::{
    EventLoopWake, Heartbeat, HeartbeatCounters, HeartbeatSink, ProcessClock, StartRefused,
    UiHeartbeat, WakeOutcome, WakeRefused, WakeTicket,
};

use crate::watches::{WatchRefused, Watches};

/// Why [`ShellHeartbeat::open`] opened nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenRefused {
    /// A heartbeat of this loop lives already: the ring's heartbeat slot takes one publisher (MT-215 E3).
    AlreadyOpen,
    /// The window crate refused a wake lane: the loop has ended, or has no lane left.
    Lane(LaneRefused),
}

impl OpenRefused {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::AlreadyOpen => "heartbeat.already_open",
            Self::Lane(refused) => refused.code(),
        }
    }
}

thread_local! {
    /// Whether a [`ShellHeartbeat`] lives on this thread. A loop runs on one thread and its `Control` cannot leave it,
    /// so this is one heartbeat per loop.
    static OPEN_ON_THIS_LOOP: Cell<bool> = const { Cell::new(false) };
}

/// This loop's one heartbeat, held as long as the [`ShellHeartbeat`] lives. Neither `Send` nor `Sync`: released on the
/// thread that took it.
struct LoopClaim(PhantomData<*const ()>);

impl LoopClaim {
    /// `None` while another heartbeat of this loop lives.
    fn take() -> Option<Self> {
        let free = OPEN_ON_THIS_LOOP
            .try_with(|open| !open.replace(true))
            .unwrap_or(false);
        // Not `then_some`: a claim made and dropped when the loop is taken would free it.
        if free {
            Some(Self(PhantomData))
        } else {
            None
        }
    }
}

impl Drop for LoopClaim {
    fn drop(&mut self) {
        let _ = OPEN_ON_THIS_LOOP.try_with(|open| open.set(false));
    }
}

/// The port's loop: one wake lane of the window crate.
struct LaneWake {
    lane: WakeLane,
}

impl EventLoopWake for LaneWake {
    type Dispatched<'a> = DispatchedWake<'a>;

    fn on_loop_thread(&self) -> bool {
        self.lane.on_loop_thread()
    }

    fn request_wake(
        &self,
        ticket: WakeTicket,
        delay: std::time::Duration,
    ) -> Result<(), WakeRefused> {
        self.lane
            .request(ticket.to_raw(), delay)
            .map_err(|refused| match refused {
                LaneRefused::Closed => WakeRefused::LoopClosed,
                _ => WakeRefused::Unavailable,
            })
    }

    fn cancel_wakes(&self) {
        self.lane.cancel();
    }

    /// Only a wake of this lane carries a ticket; any other lane's, whatever its value, carries none.
    fn dispatched_ticket(&self, wake: &DispatchedWake<'_>) -> Option<WakeTicket> {
        (wake.lane() == self.lane.id()).then(|| WakeTicket::from_raw(wake.value()))
    }
}

/// The UI heartbeat of the shell: the port on one wake lane, watching the presents of the shell's windows.
///
/// Lives on the loop's thread, as the port does: neither `Send` nor `Sync`. One per loop.
pub struct ShellHeartbeat<S: HeartbeatSink> {
    port: UiHeartbeat<ProcessClock, LaneWake, S, Watches<PresentWatch>>,
    lane: LaneId,
    /// Dropped last, once the lane is gone.
    _one_per_loop: LoopClaim,
}

impl<S: HeartbeatSink> ShellHeartbeat<S> {
    /// Open the heartbeat's wake lane. Nothing is asked for or published yet. `origin` is the zero of the beats'
    /// monotonic time, normally the process's start.
    ///
    /// # Errors
    ///
    /// [`OpenRefused`], with the sink handed back: a heartbeat of this loop lives already (then no lane is taken), or
    /// the window crate refused a lane.
    pub fn open(control: &Control, origin: Instant, sink: S) -> Result<Self, (OpenRefused, S)> {
        let Some(claim) = LoopClaim::take() else {
            return Err((OpenRefused::AlreadyOpen, sink));
        };
        match control.open_wake_lane() {
            Ok(lane) => {
                let id = lane.id();
                Ok(Self {
                    port: UiHeartbeat::with_frames(
                        ProcessClock::since(origin),
                        LaneWake { lane },
                        sink,
                        Watches::new(),
                    ),
                    lane: id,
                    _one_per_loop: claim,
                })
            }
            Err(refused) => Err((OpenRefused::Lane(refused), sink)),
        }
    }

    /// Ask the loop for the first wake.
    ///
    /// # Errors
    ///
    /// The port's [`StartRefused`]: off the loop's thread, already started or stopped, or the loop has ended.
    pub fn start(&self) -> Result<(), StartRefused> {
        self.port.start()
    }

    /// Whether `wake` came through the heartbeat's lane.
    pub fn owns(&self, wake: &DispatchedWake<'_>) -> bool {
        wake.lane() == self.lane
    }

    /// A wake the loop dispatched. `None` when it is another lane's: route it on. `Some` with what the port made of it.
    pub fn on_wake(&self, wake: &DispatchedWake<'_>) -> Option<WakeOutcome> {
        self.owns(wake).then(|| self.port.on_wake(wake))
    }

    /// Every event of the loop: a turn for the port, which publishes nothing. A resume asks for a wake at once; a
    /// destroyed window is watched no more; `Event::LoopEnding` stops the heartbeat.
    pub fn on_event(&self, event: &Event) {
        match event {
            Event::LoopEnding => {
                self.port.stop();
                return;
            }
            Event::SystemResumed => {
                self.port.on_system_resume();
            }
            Event::Window {
                window,
                event: WindowEvent::Destroyed,
            } => {
                self.port.frames().forget(window.number());
            }
            _ => {}
        }
        self.port.on_loop_turn();
    }

    /// [`Self::on_event`], and on `WindowEvent::Created` of a window that presents, watch it.
    ///
    /// # Errors
    ///
    /// [`WatchRefused`] for a window beyond [`crate::MAX_WATCHED_WINDOWS`]: its presents are not watched.
    pub fn on_shell_event(&self, shell: &Shell<'_>, event: &Event) -> Result<(), WatchRefused> {
        let watched = match event {
            Event::Window {
                window,
                event: WindowEvent::Created { .. },
            } => shell
                .watch(*window)
                .map_or(Ok(()), |watch| self.watch(*window, watch)),
            _ => Ok(()),
        };
        self.on_event(event);
        watched
    }

    /// Watch the presents of `window`.
    ///
    /// # Errors
    ///
    /// [`WatchRefused`].
    pub fn watch(&self, window: WindowKey, watch: PresentWatch) -> Result<(), WatchRefused> {
        self.port.frames().watch(window.number(), watch)
    }

    /// Watch `window` no more. Returns whether it was watched.
    pub fn forget(&self, window: WindowKey) -> bool {
        self.port.frames().forget(window.number())
    }

    /// How many windows are watched.
    pub fn watched(&self) -> usize {
        self.port.frames().count()
    }

    /// The port's counters.
    pub fn counters(&self) -> HeartbeatCounters {
        self.port.counters()
    }

    /// The last published beat.
    pub fn last_beat(&self) -> Option<Heartbeat> {
        self.port.last_beat()
    }

    /// Whether beats are published for dispatched wakes.
    pub fn is_running(&self) -> bool {
        self.port.is_running()
    }

    /// Whether a present stall is under way, as of the heartbeat's last wake. Beats go on during one.
    pub fn present_stalled(&self) -> bool {
        self.port.present_stalled()
    }

    /// End the heartbeat and withdraw its wake.
    pub fn stop(&self) {
        self.port.stop();
    }

    /// Stop, and hand back the sink, so the ring writer outlives the last beat.
    pub fn into_sink(self) -> S {
        self.port.into_sink()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_heartbeat_per_loop_until_it_is_dropped() {
        let first = LoopClaim::take().expect("the first heartbeat of this loop");
        assert!(
            LoopClaim::take().is_none(),
            "a second heartbeat on the same loop"
        );
        let elsewhere = std::thread::spawn(|| LoopClaim::take().is_some())
            .join()
            .expect("the other thread");
        assert!(elsewhere, "a loop on another thread has its own");
        drop(first);
        assert!(LoopClaim::take().is_some(), "free once the first is gone");
    }

    #[test]
    fn refusals_have_stable_names() {
        assert_eq!(OpenRefused::AlreadyOpen.code(), "heartbeat.already_open");
        assert_eq!(
            OpenRefused::Lane(LaneRefused::Closed).code(),
            "wake.closed"
        );
    }
}
