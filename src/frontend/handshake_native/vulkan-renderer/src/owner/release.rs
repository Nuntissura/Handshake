//! When a device the owner let go of ends, and how its holders learn that they may let go as well.
//!
//! Such a device stays with the owner ([`super::Slot`]): a holder that leaves early takes nothing from under running
//! work, and nobody has to leave last. It is destroyed by the first look after its last holder and its work are gone.
//! Only with the last handle of all (owner, clients and leases) is there nobody left to look.
//!
//! The slot lock is held here only to copy or take entries of the list: the looks at the devices, which may destroy
//! resources, swapchains and surfaces, and the destruction of a device run with no lock of the owner held (MT-203 E3-1).

use super::queue::bounded;
use super::{Live, OwnerShared};
use crate::ledger::ClaimHold;
use crate::outcome::ReleaseRefusal;
use std::sync::{Arc, PoisonError};
use std::time::Duration;

impl OwnerShared {
    /// Look at the devices the owner let go of, without waiting, and destroy those that nobody holds and whose work has
    /// ended. Called by everything that may have ended a hold or a submission, with no lock held.
    pub(crate) fn reap(&self) {
        let retained: Vec<Live> = self.lock_slot().retained.clone();
        if retained.is_empty() {
            return;
        }
        for live in &retained {
            // What it answers is in the ledger and in its drained mark.
            let _ = self.look(live);
        }
        // This thread's shares go before the count: a device is taken out only while the list holds its last share.
        drop(retained);
        let ended: Vec<Live> = {
            let mut slot = self.lock_slot();
            let mut ended = Vec::new();
            let mut index = 0;
            while let Some(live) = slot.retained.get(index) {
                // Nobody gets a new hold on it without the slot lock.
                if Arc::strong_count(&live.vulkan) == 1 && live.vulkan.is_drained() {
                    ended.push(slot.retained.remove(index));
                } else {
                    index += 1;
                }
            }
            ended
        };
        for live in ended {
            let number = live.number;
            let destroyed = self.destroy(live);
            self.core.retained_device_ended(number, destroyed);
        }
    }

    /// Whether the client of `hold` has no work left on the device of `generation`, after asking the device.
    pub(crate) fn may_release(
        &self,
        generation: u64,
        hold: &ClaimHold,
    ) -> Result<(), ReleaseRefusal> {
        if hold.in_flight() > 0 {
            let live = self.lock_slot().generation(generation).cloned();
            if let Some(live) = live {
                // What it answers is in the ledger.
                let _ = self.look(&live);
            }
        }
        match hold.in_flight() {
            0 => Ok(()),
            in_flight => Err(ReleaseRefusal::InFlight { in_flight }),
        }
    }

    pub(crate) fn wait_release(
        &self,
        generation: u64,
        hold: &ClaimHold,
        timeout: Duration,
    ) -> Result<(), ReleaseRefusal> {
        if let Some(value) = hold.last_in_flight() {
            let live = self.lock_slot().generation(generation).cloned();
            if let Some(live) = live {
                self.wait_for(&live, value, &mut bounded(timeout));
            }
        }
        self.may_release(generation, hold)
    }
}

impl Drop for OwnerShared {
    /// The last handle of all: nobody can look at the retained devices again.
    fn drop(&mut self) {
        let slot = self.slot.get_mut().unwrap_or_else(PoisonError::into_inner);
        let retained = std::mem::take(&mut slot.retained);
        for live in retained {
            let _ = self.look(&live);
            let number = live.number;
            let destroyed = self.destroy(live);
            self.core.retained_device_ended(number, destroyed);
        }
    }
}
