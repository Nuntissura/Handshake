//! The wake lanes of one loop, by number, and the signs that the loop runs, which they share.

use std::cell::{OnceCell, RefCell};
use std::sync::Arc;
use std::time::Instant;

use windows_sys::Win32::Foundation::{HWND, LPARAM};

use super::wake::{Lane, Pulse};
use crate::wake::{LaneId, WakeRefused};

/// The lane that hands events to the host.
pub(super) const EVENT_LANE: u32 = 0;

/// The most lanes open at once, the events' own included. The number of a dropped lane is used again (MT-214 R2-N4):
/// a post of the dropped lane that is still in the queue carries that lane's mask, which the new one does not have,
/// so it dispatches nothing (`wake.rs`). The bound also bounds the posts a hung loop gets, `wake_slot::BLIND_POSTS`
/// per lane.
const MAX_LANES: usize = 64;

#[derive(Default)]
pub(super) struct Lanes {
    open: RefCell<Vec<Option<Lane>>>,
    pulse: OnceCell<Arc<Pulse>>,
}

impl Lanes {
    /// The service window exists: the lane of the host's events opens. `None` when the system gives no timer.
    pub(super) fn attach(&self, service: HWND, thread: u32, epoch: Instant) -> Option<()> {
        let pulse = self
            .pulse
            .get_or_init(|| Pulse::new(service, thread, epoch));
        // The host's events are input; this lane does not wait for more.
        let lane = Lane::open(EVENT_LANE, pulse, false)?;
        self.open.borrow_mut().push(Some(lane));
        Some(())
    }

    pub(super) fn get(&self, id: u32) -> Option<Lane> {
        self.open.borrow().get(id as usize).cloned().flatten()
    }

    /// A lane under the first number that is free, after the events' own.
    pub(super) fn open(&self) -> Result<Lane, WakeRefused> {
        let pulse = self.pulse.get().ok_or(WakeRefused::Closed)?;
        let mut open = self.open.borrow_mut();
        let id = open
            .iter()
            .skip(1)
            .position(Option::is_none)
            .map_or(open.len(), |at| at + 1);
        if id >= MAX_LANES {
            return Err(WakeRefused::LaneLimit);
        }
        let lane = u32::try_from(id)
            .ok()
            .and_then(|number| Lane::open(number, pulse, true))
            .ok_or(WakeRefused::LaneLimit)?;
        match open.get_mut(id) {
            Some(slot) => *slot = Some(lane.clone()),
            None => open.push(Some(lane.clone())),
        }
        Ok(lane)
    }

    /// The lane was dropped; its number is free.
    pub(super) fn forget(&self, id: LaneId) {
        let forgotten = self
            .open
            .borrow_mut()
            .get_mut(id.0 as usize)
            .and_then(Option::take);
        // Dropped outside the borrow: closing its timer waits for a callback that is running.
        drop(forgotten);
    }

    /// The service window got a probe. One of the pulse's: the loop runs, so the lanes look again now whether to
    /// post, instead of when their timers say. Inside a call that holds the lanes nothing is nudged; their timers
    /// remain.
    pub(super) fn probe_answered(&self, token: LPARAM) {
        let Some(pulse) = self.pulse.get() else {
            return;
        };
        if !pulse.answered(token) {
            return;
        }
        let lanes: Vec<Lane> = match self.open.try_borrow() {
            Ok(open) => open.iter().flatten().cloned().collect(),
            Err(_) => return,
        };
        for lane in &lanes {
            lane.nudge();
        }
    }

    /// No more wakes, by any thread.
    pub(super) fn close_all(&self) {
        let lanes = std::mem::take(&mut *self.open.borrow_mut());
        for lane in lanes.iter().flatten() {
            lane.close();
        }
    }
}
