//! Waiting without a Vulkan wait: look, sleep, look again, until a deadline of one's own.
//!
//! What it costs: one look per pause, and only while a caller waits. Pauses double from [`SHORTEST`] to [`LONGEST`], so
//! short work is seen within about its own length and long work within [`LONGEST`], at no more than 250 looks a second.

use std::time::{Duration, Instant};

const SHORTEST: Duration = Duration::from_micros(100);
const LONGEST: Duration = Duration::from_millis(4);

/// The deadline of one wait, and the pause before its next look.
pub(crate) struct Backoff {
    end: Instant,
    pause: Duration,
}

impl Backoff {
    /// A wait that ends `timeout` from now. With zero it looks once.
    pub(crate) fn new(timeout: Duration) -> Self {
        let now = Instant::now();
        Self {
            end: now.checked_add(timeout).unwrap_or(now),
            pause: SHORTEST,
        }
    }

    /// Sleep until the next look is due. `false`, at once, when the deadline has passed.
    pub(crate) fn pause(&mut self) -> bool {
        let left = self.end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        std::thread::sleep(self.pause.min(left));
        self.pause = (self.pause * 2).min(LONGEST);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{Backoff, LONGEST, SHORTEST};
    use std::time::{Duration, Instant};

    #[test]
    fn a_wait_without_time_never_sleeps() {
        assert!(!Backoff::new(Duration::ZERO).pause());
        // A deadline that cannot be represented is none.
        assert!(!Backoff::new(Duration::MAX).pause());
    }

    #[test]
    fn a_wait_ends_at_its_deadline_and_its_pauses_stay_short() {
        let timeout = Duration::from_millis(60);
        let start = Instant::now();
        let mut backoff = Backoff::new(timeout);
        assert_eq!(backoff.pause, SHORTEST);
        let mut looks = 0u32;
        while backoff.pause() {
            looks += 1;
            assert!(backoff.pause <= LONGEST);
        }
        assert!(looks >= 2, "{looks}");
        assert_eq!(backoff.pause, LONGEST.min(SHORTEST * (1 << looks.min(8))));
        let took = start.elapsed();
        assert!(took >= timeout && took < timeout * 50, "{took:?}");
        assert!(!backoff.pause(), "past its deadline it sleeps no more");
    }
}
