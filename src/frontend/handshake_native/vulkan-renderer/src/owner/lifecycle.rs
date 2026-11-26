//! Recovery from a device loss, and shutdown: when a device generation ends, and what its holders have to do with it.

use super::queue::{bounded, MAX_WAIT};
use super::{Live, OwnerShared, Slot};
use crate::ids::DeviceKey;
use crate::outcome::DeviceRebindRefusal;
use crate::owner_core::ShutdownReport;
use crate::process::ProcessClaim;
use crate::records::DeviceFate;
use crate::vulkan::VulkanGeneration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Lost devices that are kept for clients which have not returned them, while recovery goes on. One client that never
/// returns its lease uses up one of them, however often the device is lost.
pub const MAX_HELD_LOST_DEVICES: usize = 2;

/// One rebind at a time, without a lock that a close would have to wait for.
struct Rebinding<'a>(&'a AtomicBool);

impl<'a> Rebinding<'a> {
    fn begin(running: &'a AtomicBool) -> Option<Self> {
        (!running.swap(true, Ordering::AcqRel)).then(|| Self(running))
    }
}

impl Drop for Rebinding<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// While it exists, [`Slot::creating`] is set.
struct Creating<'a>(&'a OwnerShared);

impl Drop for Creating<'_> {
    fn drop(&mut self) {
        self.0.lock_slot().creating = false;
    }
}

impl OwnerShared {
    pub(super) fn rebind_device(&self) -> Result<u64, DeviceRebindRefusal> {
        let result = self.replace_device();
        self.reap();
        if let Err(refusal) = &result {
            self.core.device_rebind_refused(*refusal);
        }
        result
    }

    /// The bound generation, when it is lost.
    fn lost_generation(&self) -> Result<Option<Live>, DeviceRebindRefusal> {
        let slot = self.lock_slot();
        if matches!(self.core.ledger().device(), Some(device) if !device.lost) {
            return Err(DeviceRebindRefusal::NotLost);
        }
        Ok(slot.live.clone())
    }

    fn replace_device(&self) -> Result<u64, DeviceRebindRefusal> {
        let Some(_rebinding) = Rebinding::begin(&self.rebinding) else {
            return Err(DeviceRebindRefusal::InProgress);
        };
        if self.lock_closed().is_some() {
            return Err(DeviceRebindRefusal::Closed);
        }
        // Asked with no lock held: whatever the lost device does, it keeps no client and no close waiting.
        if let Some(lost) = self.lost_generation()? {
            lost.vulkan.drain(&mut bounded(MAX_WAIT));
        }
        let (process, key, last) = self.let_go_of_lost()?;
        let creating = Creating(self);
        // Nor do its destruction and the creation of the new one.
        if let Some(live) = last {
            self.end_lost(live);
        }
        let opened = VulkanGeneration::open(process, Some(key), self.presentation);
        let closed = self.lock_closed();
        drop(creating);
        match opened {
            Ok(generation) if closed.is_some() => {
                // Closed meanwhile, and said so. It was never bound and never had work, so it ends here.
                drop(closed);
                let (_, targets) = generation.dispose();
                self.outbox.bury(targets);
                Err(DeviceRebindRefusal::Closed)
            }
            Ok(generation) => {
                let mut slot = self.lock_slot();
                slot.identity = generation.identity().clone();
                let number = self.core.device_bound(slot.identity.key);
                slot.live = Some(Live {
                    number,
                    vulkan: Arc::new(generation),
                });
                Ok(number)
            }
            Err(_) if closed.is_some() => Err(DeviceRebindRefusal::Closed),
            Err(reason) => {
                self.core.device_unavailable(reason);
                Err(DeviceRebindRefusal::Unavailable(reason))
            }
        }
    }

    /// Let go of the lost device, as one step with respect to a close, and begin to create its successor. Returns what
    /// that one needs, and the lost device when nobody else holds it.
    fn let_go_of_lost(
        &self,
    ) -> Result<(Arc<ProcessClaim>, DeviceKey, Option<Live>), DeviceRebindRefusal> {
        let closed = self.lock_closed();
        if closed.is_some() {
            return Err(DeviceRebindRefusal::Closed);
        }
        let (process, key, lost) = {
            let mut slot = self.lock_slot();
            if matches!(self.core.ledger().device(), Some(device) if !device.lost) {
                return Err(DeviceRebindRefusal::NotLost);
            }
            let Some(process) = slot.process.clone() else {
                return Err(DeviceRebindRefusal::Closed);
            };
            let lost = self.take_lost(&mut slot)?;
            slot.creating = true;
            (process, slot.identity.key, lost)
        };
        let last = match lost {
            None => None,
            Some((live, 0)) => {
                self.core.device_rebinding(live.number);
                Some(live)
            }
            Some((live, holders)) => {
                self.core.device_rebinding(live.number);
                let holders = u64::try_from(holders).unwrap_or(u64::MAX);
                self.core.lost_device_set_aside(live.number, holders);
                // Its holders may have left meanwhile: then it ends in the reap that follows.
                None
            }
        };
        Ok((process, key, last))
    }

    /// Destroy the lost device, which nobody else holds.
    fn end_lost(&self, live: Live) {
        let number = live.number;
        if self.destroy(live) {
            self.core.generation_retired(number);
        } else {
            // It was drained when it was taken, and that is for good.
            self.core.retained_device_ended(number, false);
        }
    }

    /// Take the lost device out of the slot, with the number of its holders. With holders it is kept among the
    /// retained ones; without, the one returned is the last of it.
    fn take_lost(&self, slot: &mut Slot) -> Result<Option<(Live, usize)>, DeviceRebindRefusal> {
        let Some(live) = slot.live.as_ref() else {
            return Ok(None);
        };
        if !live.vulkan.is_drained() {
            return Err(DeviceRebindRefusal::NotQuiescent);
        }
        self.core.ledger().resolve_lost(live.number);
        // Nobody gets a new hold on it without the slot lock, so from here on they only become fewer.
        let holders = Arc::strong_count(&live.vulkan).saturating_sub(1);
        if holders > 0 && slot.retained.len() >= MAX_HELD_LOST_DEVICES {
            return Err(DeviceRebindRefusal::DeviceHeld {
                holders: u32::try_from(holders).unwrap_or(u32::MAX),
            });
        }
        let taken = slot.live.take();
        if holders > 0 {
            slot.retained.extend(taken.clone());
        }
        Ok(taken.map(|live| (live, holders)))
    }

    pub(super) fn close(&self) -> ShutdownReport {
        let mut closed = self.lock_closed();
        if let Some(report) = *closed {
            drop(closed);
            self.reap();
            return report;
        }
        self.core.begin_shutdown();
        // From here on nothing is leased, recorded or submitted. What was submitted is still observed, and the slot is
        // free while the queue drains.
        let live = {
            let mut slot = self.lock_slot();
            slot.process = None;
            let live = slot.live.take();
            slot.retained.extend(live.clone());
            live
        };
        if let Some(live) = &live {
            live.vulkan.seal();
            live.vulkan.drain(&mut bounded(MAX_WAIT));
            // What the device says now ends what it ends; the rest stays in flight.
            let _ = self.look(live);
        }
        let (device, lost_devices_held, kept) = self.end_at_close(live);
        let lost_devices_held = u64::try_from(lost_devices_held).unwrap_or(u64::MAX);
        let report = self.core.finish_shutdown(device, lost_devices_held);
        *closed = Some(report);
        drop(closed);
        // Its holders may have left meanwhile: then it ends here, reported after the close.
        drop(kept);
        self.reap();
        report
    }

    /// Destroy the device of `live`, which is among the retained ones, when nobody else holds it and it confirmed that
    /// its work has ended. Returns what became of it, how many lost devices are kept besides, and `live` when it is.
    fn end_at_close(&self, live: Option<Live>) -> (DeviceFate, usize, Option<Live>) {
        let mut slot = self.lock_slot();
        let Some(live) = live else {
            let fate = if slot.creating {
                DeviceFate::Creating
            } else {
                DeviceFate::NoDevice
            };
            return (fate, slot.retained.len(), None);
        };
        let others = slot.retained.len().saturating_sub(1);
        // One share is in the list and one is here.
        let holders = Arc::strong_count(&live.vulkan).saturating_sub(2);
        if holders > 0 {
            let holders = u64::try_from(holders).unwrap_or(u64::MAX);
            return (DeviceFate::Held { holders }, others, Some(live));
        }
        if !live.vulkan.is_drained() {
            return (DeviceFate::Unconfirmed, others, Some(live));
        }
        slot.retained.retain(|kept| kept.number != live.number);
        drop(slot);
        let fate = if self.destroy(live) {
            DeviceFate::Destroyed
        } else {
            // It was drained, and that is for good.
            DeviceFate::Unconfirmed
        };
        (fate, others, None)
    }
}

impl OwnerShared {
    /// Destroy the device of `live`, the last share of it. `false` when it is not the last or did not confirm that its
    /// work has ended. The targets of its windows are dropped once this thread holds no lock of the owner.
    pub(super) fn destroy(&self, live: Live) -> bool {
        let Some(generation) = Arc::into_inner(live.vulkan) else {
            return false;
        };
        let (destroyed, targets) = generation.dispose();
        self.outbox.bury(targets);
        destroyed
    }
}
