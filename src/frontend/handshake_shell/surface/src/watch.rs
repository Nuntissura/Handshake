//! Present progress for whoever watches the loop (MT-215 HB-4).
//!
//! A heartbeat proves that the loop dispatches wakes; it cannot tell a loop that wants frames and never presents one.
//! A [`PresentWatch`] can: it carries the owner's [`PresentProbe`] of the window's surface (what the queue took) and,
//! from the loop, since when a frame has been wanted and not presented. Both are read from any thread, under a lock
//! that is held for a copy only and never across a wait, with no call per frame. Wiring it into the heartbeat port or
//! the watcher is MT-215's; this crate only publishes it.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use handshake_vulkan_renderer::{PresentProbe, PresentProgress};

/// One window's present progress, for a watcher on any thread. Cheap to clone.
#[derive(Clone)]
pub struct PresentWatch {
    inner: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    /// The probe of the window's current surface; replaced when the surface is made again.
    probe: Option<PresentProbe>,
    /// Since when a frame is wanted and not presented.
    wanted_since: Option<Instant>,
}

impl std::fmt::Debug for PresentWatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresentWatch").finish_non_exhaustive()
    }
}

/// What a [`PresentWatch`] read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentReading {
    /// What the owner counted for the window's surface; `None` while it has none.
    pub present: Option<PresentProgress>,
    /// How long a frame has been wanted and not presented, as the loop last said. `None` when none is wanted, and while
    /// the window cannot show one (hidden, minimised, no size). It keeps growing when the loop stops saying anything.
    pub frame_wanted_for: Option<Duration>,
}

impl PresentReading {
    /// A frame has been wanted for at least `threshold` and none was presented since: the loop lives and shows nothing
    /// (HB-4), or it stopped altogether.
    pub fn never_presents(&self, threshold: Duration) -> bool {
        self.frame_wanted_for.is_some_and(|wanted| wanted >= threshold)
    }
}

impl PresentWatch {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(State::default())),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn set_probe(&self, probe: Option<PresentProbe>) {
        self.state().probe = probe;
    }

    /// The loop says how long a frame has been waiting now (`FrameProgress::waiting` of the window crate).
    pub(crate) fn publish(&self, waiting: Option<Duration>) {
        let since = waiting.map(|waiting| {
            let now = Instant::now();
            now.checked_sub(waiting).unwrap_or(now)
        });
        self.state().wanted_since = since;
    }

    /// Read it.
    pub fn read(&self) -> PresentReading {
        let (probe, since) = {
            let state = self.state();
            (state.probe.clone(), state.wanted_since)
        };
        PresentReading {
            present: probe.as_ref().map(PresentProbe::read),
            frame_wanted_for: since.map(|since| since.elapsed()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wanted_frame_keeps_waiting_until_the_loop_says_otherwise() {
        let watch = PresentWatch::new();
        let reading = watch.read();
        assert_eq!(reading.frame_wanted_for, None);
        assert!(!reading.never_presents(Duration::ZERO));
        assert_eq!(reading.present, None, "no surface yet");
        watch.publish(Some(Duration::from_millis(40)));
        let first = watch.read().frame_wanted_for.unwrap();
        assert!(first >= Duration::from_millis(40), "{first:?}");
        std::thread::sleep(Duration::from_millis(20));
        let later = watch.read();
        assert!(
            later.frame_wanted_for.unwrap() >= first + Duration::from_millis(20),
            "it grows while the loop says nothing"
        );
        assert!(later.never_presents(Duration::from_millis(60)));
        assert!(!later.never_presents(Duration::from_secs(60)));
        watch.publish(None);
        assert_eq!(watch.read().frame_wanted_for, None, "presented");
    }

    #[test]
    fn it_is_read_from_another_thread() {
        let watch = PresentWatch::new();
        watch.publish(Some(Duration::from_millis(5)));
        let other = watch.clone();
        let read = std::thread::spawn(move || other.read()).join().unwrap();
        assert!(read.never_presents(Duration::from_millis(5)));
    }
}
