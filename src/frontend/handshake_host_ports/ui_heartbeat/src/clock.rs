//! The time source of the port.

use std::time::Instant;

/// A monotonic clock. Reading it must not block, allocate or panic.
///
/// The port never relies on it for liveness: a beat is a dispatched wake, whatever the clock says. The clock only
/// stamps beats and spaces replacement wakes, and the port tolerates a reading that jumps in either direction.
pub trait MonotonicClock {
    /// Nanoseconds since an origin the implementation chose.
    fn now_nanos(&self) -> u64;
}

/// The production clock: [`Instant`], measured from an origin the host chooses (normally process start).
///
/// `Instant` is not a wall clock, so it is unaffected by time-of-day changes. On Windows it counts time spent asleep;
/// nothing here depends on whether it does.
#[derive(Clone, Copy, Debug)]
pub struct ProcessClock {
    origin: Instant,
}

impl ProcessClock {
    /// A clock whose zero is `origin`.
    pub fn since(origin: Instant) -> Self {
        Self { origin }
    }
}

impl MonotonicClock for ProcessClock {
    fn now_nanos(&self) -> u64 {
        // `checked` because `Instant` only promises not to panic on a reading before the origin in its checked form.
        let elapsed = Instant::now()
            .checked_duration_since(self.origin)
            .unwrap_or_default();
        u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
    }
}
