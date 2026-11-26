//! Present progress for whoever watches the loop (MT-215 HB-4).
//!
//! A heartbeat proves that the loop dispatches wakes; it cannot tell a loop that wants frames and never presents one.
//! A [`PresentWatch`] can: it carries the owner's [`PresentProbe`] of the window's surface (what the queue took) and,
//! from the loop, since when a frame has been wanted and not presented. Both are read from any thread, with no call per
//! frame. Since when a frame is wanted is one atomic, and [`PresentWatch::frame_wanted_for`] reads it and the clock and
//! nothing else: no lock, so the heartbeat that reads it at each beat never waits for the loop's own writes (MT-215 N2,
//! AC215-6). The probe sits under a lock that is held for a copy only; [`PresentWatch::read`] takes it, the beat does
//! not. Wiring it into the heartbeat port or the watcher is MT-215's; this crate only publishes it.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use handshake_vulkan_renderer::{PresentProbe, PresentProgress};

/// One window's present progress, for a watcher on any thread. Cheap to clone.
#[derive(Clone)]
pub struct PresentWatch {
    inner: Arc<Shared>,
}

/// No frame is wanted.
const NOT_WANTED: i64 = i64::MIN;

struct Shared {
    /// The probe of the window's current surface; replaced when the surface is made again.
    probe: Mutex<Option<PresentProbe>>,
    /// Since when a frame is wanted and not presented: nanoseconds after `origin` (negative before it), or
    /// [`NOT_WANTED`]. One atomic, so a reader never waits.
    wanted_since: AtomicI64,
    /// The zero of `wanted_since`.
    origin: Instant,
}

impl Shared {
    /// Nanoseconds after `origin`, saturating.
    fn elapsed(&self) -> i64 {
        i64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(i64::MAX)
    }
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
            inner: Arc::new(Shared {
                probe: Mutex::new(None),
                wanted_since: AtomicI64::new(NOT_WANTED),
                origin: Instant::now(),
            }),
        }
    }

    pub(crate) fn set_probe(&self, probe: Option<PresentProbe>) {
        *self.inner.probe.lock().unwrap_or_else(PoisonError::into_inner) = probe;
    }

    /// The loop says how long a frame has been waiting now (`FrameProgress::waiting` of the window crate). A store.
    pub(crate) fn publish(&self, waiting: Option<Duration>) {
        let since = waiting.map_or(NOT_WANTED, |waiting| {
            let waiting = i64::try_from(waiting.as_nanos()).unwrap_or(i64::MAX);
            // Never `NOT_WANTED` itself, however long it waited.
            self.inner.elapsed().saturating_sub(waiting).max(NOT_WANTED + 1)
        });
        self.inner.wanted_since.store(since, Ordering::Release);
    }

    /// How long a frame has been wanted and not presented, as the loop last said; the same as
    /// [`PresentReading::frame_wanted_for`] of [`PresentWatch::read`]. Takes no lock and waits for nobody: one atomic
    /// load and the clock. This is what the heartbeat reads at each beat (MT-215 N2).
    pub fn frame_wanted_for(&self) -> Option<Duration> {
        let since = self.inner.wanted_since.load(Ordering::Acquire);
        if since == NOT_WANTED {
            return None;
        }
        let waited = self.inner.elapsed().saturating_sub(since).max(0);
        Some(Duration::from_nanos(u64::try_from(waited).unwrap_or(0)))
    }

    /// Read it.
    pub fn read(&self) -> PresentReading {
        let probe = self
            .inner
            .probe
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        PresentReading {
            present: probe.as_ref().map(PresentProbe::read),
            frame_wanted_for: self.frame_wanted_for(),
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

    /// MT-215 N2: the wait the heartbeat reads is the same number as the reading's, and needs no lock: it is read here
    /// while another thread holds the probe's lock.
    #[test]
    fn the_wait_is_read_without_the_probes_lock() {
        let watch = PresentWatch::new();
        watch.publish(Some(Duration::from_millis(7)));
        let held = watch.inner.probe.lock().unwrap();
        let waited = watch.frame_wanted_for().expect("a frame is wanted");
        assert!(waited >= Duration::from_millis(7), "{waited:?}");
        drop(held);
        watch.publish(None);
        assert_eq!(watch.frame_wanted_for(), None);
        // A wait older than the watch itself, and one the clock cannot hold.
        watch.publish(Some(Duration::from_secs(3_600)));
        assert!(watch.frame_wanted_for().unwrap() >= Duration::from_secs(3_600));
        watch.publish(Some(Duration::MAX));
        assert!(watch.frame_wanted_for().is_some(), "the longest wait is a wait, not none");
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
