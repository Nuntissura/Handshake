//! The present watches the heartbeat reads at each of its wakes (MT-215 HB-4): a fixed number of windows, no
//! allocation, no lock and no wait: each watch is read with `PresentWatch::frame_wanted_for`, one atomic load and the
//! clock (MT-215 N2, AC215-6).

use std::cell::RefCell;
use std::time::Duration;

use handshake_shell_surface::PresentWatch;
use ui_heartbeat::FrameWait;

/// The most windows whose presents the heartbeat watches (AC215-9). A window beyond it is refused, typed.
pub const MAX_WATCHED_WINDOWS: usize = 16;

/// Why a window's present watch was not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchRefused {
    /// [`MAX_WATCHED_WINDOWS`] are watched already.
    Full,
    /// Asked from inside a read of the watches; ask again from the handler.
    Busy,
}

impl WatchRefused {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::Full => "heartbeat.watches_full",
            Self::Busy => "heartbeat.watches_busy",
        }
    }
}

/// What one watch says: how long a frame of its window has been wanted and not presented.
pub(crate) trait Waits {
    fn frame_wanted_for(&self) -> Option<Duration>;
}

impl Waits for PresentWatch {
    fn frame_wanted_for(&self) -> Option<Duration> {
        PresentWatch::frame_wanted_for(self)
    }
}

/// Watches by window number.
pub(crate) struct Watches<W> {
    slots: RefCell<[Option<(u64, W)>; MAX_WATCHED_WINDOWS]>,
}

impl<W> Watches<W> {
    pub(crate) fn new() -> Self {
        Self {
            slots: RefCell::new(std::array::from_fn(|_| None)),
        }
    }

    /// Watch `window`, in place of what watched it before.
    pub(crate) fn watch(&self, window: u64, watch: W) -> Result<(), WatchRefused> {
        let mut slots = self.slots.try_borrow_mut().map_err(|_| WatchRefused::Busy)?;
        let held = slots
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|(number, _)| *number == window));
        let at = held
            .or_else(|| slots.iter().position(Option::is_none))
            .ok_or(WatchRefused::Full)?;
        if let Some(slot) = slots.get_mut(at) {
            *slot = Some((window, watch));
        }
        Ok(())
    }

    /// Watch `window` no more. Returns whether it was watched.
    pub(crate) fn forget(&self, window: u64) -> bool {
        let Ok(mut slots) = self.slots.try_borrow_mut() else {
            return false;
        };
        let mut forgot = false;
        for slot in slots.iter_mut() {
            if slot.as_ref().is_some_and(|(number, _)| *number == window) {
                *slot = None;
                forgot = true;
            }
        }
        forgot
    }

    pub(crate) fn count(&self) -> usize {
        self.slots
            .try_borrow()
            .map_or(0, |slots| slots.iter().flatten().count())
    }
}

impl<W: Waits> FrameWait for Watches<W> {
    /// The longest wait of any watched window. While the watches are being changed, `None`: no registration runs a
    /// loop, so a wake cannot come in during one.
    fn longest_wait(&self) -> Option<Duration> {
        let slots = self.slots.try_borrow().ok()?;
        slots
            .iter()
            .flatten()
            .filter_map(|(_, watch)| watch.frame_wanted_for())
            .max()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[derive(Clone, Default)]
    struct Fake(Rc<Cell<Option<Duration>>>);

    impl Waits for Fake {
        fn frame_wanted_for(&self) -> Option<Duration> {
            self.0.get()
        }
    }

    fn ms(millis: u64) -> Option<Duration> {
        Some(Duration::from_millis(millis))
    }

    #[test]
    fn the_longest_wait_of_any_window_counts_and_none_waits_without_one() {
        let watches = Watches::new();
        assert_eq!(watches.longest_wait(), None, "no window");
        let (a, b) = (Fake::default(), Fake::default());
        watches.watch(1, a.clone()).unwrap();
        watches.watch(2, b.clone()).unwrap();
        assert_eq!(watches.longest_wait(), None, "nothing wanted");
        a.0.set(ms(30));
        b.0.set(ms(2_500));
        assert_eq!(watches.longest_wait(), ms(2_500));
        assert!(watches.forget(2));
        assert!(!watches.forget(2));
        assert_eq!(watches.longest_wait(), ms(30), "a forgotten window waits no more");
    }

    #[test]
    fn a_window_is_watched_once_and_the_number_is_bounded() {
        let watches = Watches::new();
        let stale = Fake::default();
        stale.0.set(ms(9_000));
        watches.watch(7, stale).unwrap();
        watches.watch(7, Fake::default()).unwrap();
        assert_eq!(watches.count(), 1);
        assert_eq!(watches.longest_wait(), None, "the new watch replaced the old");
        for window in 100..100 + MAX_WATCHED_WINDOWS as u64 - 1 {
            watches.watch(window, Fake::default()).unwrap();
        }
        assert_eq!(watches.count(), MAX_WATCHED_WINDOWS);
        assert_eq!(watches.watch(999, Fake::default()), Err(WatchRefused::Full));
        assert!(watches.forget(100));
        assert_eq!(watches.watch(999, Fake::default()), Ok(()));
    }

    #[test]
    fn a_change_during_a_read_is_refused_and_a_read_during_a_change_says_nothing() {
        let watches = Watches::new();
        let waits = Fake::default();
        waits.0.set(ms(5_000));
        watches.watch(1, waits).unwrap();
        {
            let _reading = watches.slots.borrow();
            assert_eq!(watches.watch(2, Fake::default()), Err(WatchRefused::Busy));
            assert!(!watches.forget(1));
        }
        {
            let _changing = watches.slots.borrow_mut();
            assert_eq!(watches.longest_wait(), None);
        }
        assert_eq!(watches.longest_wait(), ms(5_000));
    }
}
