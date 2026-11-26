//! The UI heartbeat on the shell's loop and present path (MT-215 slice 2).
//!
//! The port (`ui_heartbeat`) asks a loop for wakes and publishes a beat for each one the loop dispatches. Here the loop
//! is one wake lane of the window crate, which keeps the port's contract by construction: an accepted wake is posted
//! again until the loop dispatches it, with no more than four posts while the loop takes nothing out (MT-215 F1, F4);
//! a newer request replaces one not dispatched; a message that anybody else posts or sends dispatches nothing; and the
//! proof of a dispatch, [`DispatchedWake`], only the loop can make, so no handler can stand in for it (HB-7). The frames
//! are the surface crate's present watches (HB-4): while a watched window has wanted a frame for
//! `ui_heartbeat::PRESENT_STALL` without a present, beats are withheld.
//!
//! What reads the result: the standalone Palmistry watcher reads the ring's heartbeat slot (a frozen UI thread or a
//! present stall goes stale; only the first is confirmed as a freeze, because the window of a live loop still
//! answers), and the internal diagnostics and Palmistry's survivor record read the typed stall events in the same
//! ring (`ui_heartbeat::HeartbeatEvent`).
//!
//! # How a host uses it
//!
//! 1. On `Event::LoopStarted`, after the ring writer or the recorder exists: [`ShellHeartbeat::open`] with the sink,
//!    then [`ShellHeartbeat::start`].
//! 2. In `ShellApp::on_event` call [`ShellHeartbeat::on_shell_event`] (with the window crate alone,
//!    [`ShellHeartbeat::on_event`] and [`ShellHeartbeat::watch`]): it watches every window that presents, forgets it
//!    when it is destroyed, takes resumes and stops at `Event::LoopEnding`.
//! 3. In `ShellApp::on_wake` call [`ShellHeartbeat::on_wake`] first: `Some` means the wake was the heartbeat's, `None`
//!    that it belongs to another lane.
//! 4. End the watcher while the loop still runs (`Event::ShutdownBegan`; MT-215 HB-3), then let the loop end.
//!
//! The sink is the port's for good: one publisher per ring. Nothing else in the shell writes the heartbeat slot
//! (`tests/source_scan.rs`).

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
/// Lives on the loop's thread, as the port does: neither `Send` nor `Sync`.
pub struct ShellHeartbeat<S: HeartbeatSink> {
    port: UiHeartbeat<ProcessClock, LaneWake, S, Watches<PresentWatch>>,
    lane: LaneId,
}

impl<S: HeartbeatSink> ShellHeartbeat<S> {
    /// Open the heartbeat's wake lane. Nothing is asked for or published yet. `origin` is the zero of the beats'
    /// monotonic time, normally the process's start.
    ///
    /// # Errors
    ///
    /// The window crate's refusal of a lane, with the sink handed back.
    pub fn open(control: &Control, origin: Instant, sink: S) -> Result<Self, (LaneRefused, S)> {
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
                })
            }
            Err(refused) => Err((refused, sink)),
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

    /// Whether beats are withheld for a present stall now.
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
