//! Wakes: how something asks the event loop to run the host's handler on the loop's thread.

use std::fmt;
use std::marker::PhantomData;
use std::rc::Weak;
use std::time::Duration;

use crate::platform::{Core, Lane};

/// One source of wakes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LaneId(pub(crate) u32);

/// Why a wake was not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WakeRefused {
    /// The loop has ended or the lane was dropped; no wake will be dispatched.
    Closed,
    /// No more lanes in this loop.
    LaneLimit,
}

impl WakeRefused {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::Closed => "wake.closed",
            Self::LaneLimit => "wake.lane_limit",
        }
    }
}

/// A lane as the loop's own thread holds it.
///
/// At most one wake of a lane is under way: a newer request replaces one that has not been dispatched. A wake that was
/// accepted is posted until the loop dispatches it, so neither a full message queue nor a thrown away message loses
/// it. Dropping the lane withdraws its wake.
pub struct WakeLane {
    lane: Lane,
    core: Weak<Core>,
}

impl WakeLane {
    pub(crate) fn new(lane: Lane, core: Weak<Core>) -> Self {
        Self { lane, core }
    }

    /// Which lane.
    pub fn id(&self) -> LaneId {
        self.lane.id()
    }

    /// Whether the calling thread is the one that dispatches the loop's events and owns its windows.
    pub fn on_loop_thread(&self) -> bool {
        self.lane.on_loop_thread()
    }

    /// Have the loop call [`crate::ShellHost::on_wake`] with `value` once `delay` has passed. Never calls it from
    /// here, never waits for the loop and allocates nothing.
    ///
    /// # Errors
    ///
    /// [`WakeRefused::Closed`].
    pub fn request(&self, value: u64, delay: Duration) -> Result<(), WakeRefused> {
        self.lane.request(value, delay)
    }

    /// Withdraw a wake that has not been dispatched.
    pub fn cancel(&self) {
        self.lane.cancel();
    }

    /// A handle for other threads.
    pub fn waker(&self) -> LaneWaker {
        LaneWaker {
            lane: self.lane.clone(),
        }
    }
}

impl Drop for WakeLane {
    fn drop(&mut self) {
        self.lane.close();
        if let Some(core) = self.core.upgrade() {
            core.forget_lane(self.lane.id());
        }
    }
}

impl fmt::Debug for WakeLane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WakeLane({})", self.lane.id().0)
    }
}

/// A lane as any thread holds it. It can ask for a wake and nothing else: what the wake does runs on the loop's thread.
#[derive(Clone)]
pub struct LaneWaker {
    lane: Lane,
}

impl LaneWaker {
    /// Have the loop call [`crate::ShellHost::on_wake`] with `value` as soon as it can. Wakes asked for before the
    /// loop dispatched one become one, with the newest value.
    ///
    /// # Errors
    ///
    /// [`WakeRefused::Closed`].
    pub fn wake(&self, value: u64) -> Result<(), WakeRefused> {
        self.lane.request(value, Duration::ZERO)
    }
}

impl fmt::Debug for LaneWaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LaneWaker({})", self.lane.id().0)
    }
}

/// Proof that the event loop took a wake from its own queue and is dispatching it on its thread, now.
///
/// Only the loop makes one. It cannot be copied, kept or sent to another thread, so code that takes it by reference
/// cannot be run from anywhere else (MT-215 HB-7).
pub struct DispatchedWake<'a> {
    lane: LaneId,
    value: u64,
    late: Duration,
    inside_host: bool,
    here: PhantomData<(&'a (), *const ())>,
}

impl DispatchedWake<'_> {
    pub(crate) fn new(lane: LaneId, value: u64, late: Duration, inside_host: bool) -> Self {
        Self {
            lane,
            value,
            late,
            inside_host,
            here: PhantomData,
        }
    }

    /// The lane it came through.
    pub fn lane(&self) -> LaneId {
        self.lane
    }

    /// The value it was asked for with.
    pub fn value(&self) -> u64 {
        self.value
    }

    /// How long after it was due.
    pub fn late(&self) -> Duration {
        self.late
    }

    /// Dispatched by a loop that runs inside a call of the host's own (a dialog or menu the host opened), so that call
    /// has not returned.
    pub fn inside_host_call(&self) -> bool {
        self.inside_host
    }
}

impl fmt::Debug for DispatchedWake<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DispatchedWake")
            .field("lane", &self.lane)
            .field("value", &self.value)
            .field("late", &self.late)
            .field("inside_host", &self.inside_host)
            .finish()
    }
}
