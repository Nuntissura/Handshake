//! Recovery from a device loss, and shutdown: when a device generation ends, and what its holders have to do with it.

use super::queue::{bounded_ns, MAX_WAIT};
use super::{Live, OwnerShared, Slot};
use crate::ids::DeviceKey;
use crate::outcome::DeviceRebindRefusal;
use crate::owner_core::ShutdownReport;
use crate::process::ProcessClaim;
use crate::records::DeviceFate;
use crate::vulkan::{Epilogue, QueueFault, VulkanGeneration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Lost devices that are kept for clients which have not returned them, while recovery goes on. One client that never
/// returns its lease uses up one of them, however often the device is lost.
pub const MAX_HELD_LOST_DEVICES: usize = 2;

/// What became of a device its owner let go of.
enum LetGo {
    Destroyed,
    /// Left alive: it did not confirm that its work has ended.
    Unconfirmed,
    /// Still held by `holders` others, never 0. It ends when they and this reference are gone.
    Held {
        holders: usize,
        vulkan: Arc<VulkanGeneration>,
    },
}

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

impl OwnerShared {
    pub(super) fn rebind_device(&self) -> Result<u64, DeviceRebindRefusal> {
        let result = self.replace_device();
        if let Err(refusal) = &result {
            self.core.device_rebind_refused(*refusal);
        }
        result
    }

    /// The bound generation, when it is lost.
    fn lost_generation(&self) -> Result<Option<Arc<VulkanGeneration>>, DeviceRebindRefusal> {
        let slot = self.lock_slot();
        if matches!(self.core.ledger().device(), Some(device) if !device.lost) {
            return Err(DeviceRebindRefusal::NotLost);
        }
        Ok(slot.live.as_ref().map(|live| Arc::clone(&live.vulkan)))
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
            lost.drain_lost(bounded_ns(MAX_WAIT));
        }
        let (process, key) = self.end_lost()?;
        // Nor does the creation of the new one.
        let opened = VulkanGeneration::open(process, Some(key));
        let closed = self.lock_closed();
        match opened {
            Ok(generation) if closed.is_some() => {
                // Closed meanwhile. It was never bound and never had work, so it ends here.
                drop(closed);
                drop(generation);
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

    /// Let go of the lost device, as one step with respect to a close. Returns what its successor needs.
    fn end_lost(&self) -> Result<(Arc<ProcessClaim>, DeviceKey), DeviceRebindRefusal> {
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
            (process, slot.identity.key, take_lost(&mut slot)?)
        };
        if let Some(Live { number, vulkan }) = lost {
            self.core.device_rebinding(number);
            self.core.ledger().resolve_lost(number);
            match self.let_go(number, vulkan) {
                LetGo::Destroyed => {
                    self.core.generation_retired(number);
                }
                LetGo::Held { holders, vulkan } => {
                    let holders = u64::try_from(holders).unwrap_or(u64::MAX);
                    self.core.lost_device_set_aside(number, holders);
                    // Its holders may have left meanwhile: then it ends here.
                    drop(vulkan);
                }
                // It was drained when it was taken, and that is for good.
                LetGo::Unconfirmed => self.core.retained_device_ended(number, false),
            }
        }
        Ok((process, key))
    }

    /// Destroy the device of `number` when nobody else holds it and it confirms that its work has ended. The caller
    /// took it out of the slot, so nobody gets a new hold on it.
    fn let_go(&self, number: u64, mut vulkan: Arc<VulkanGeneration>) -> LetGo {
        loop {
            vulkan = match Arc::try_unwrap(vulkan) {
                Ok(generation) => {
                    return if generation.dispose() {
                        LetGo::Destroyed
                    } else {
                        LetGo::Unconfirmed
                    }
                }
                Err(vulkan) => vulkan,
            };
            let holders = Arc::strong_count(&vulkan).saturating_sub(1);
            if holders > 0 {
                vulkan.retain_for_holders(Epilogue {
                    core: Arc::clone(&self.core),
                    generation: number,
                });
                return LetGo::Held { holders, vulkan };
            }
            // The last of them left between the two looks: nobody holds it now.
        }
    }

    pub(super) fn close(&self) -> ShutdownReport {
        let mut closed = self.lock_closed();
        if let Some(report) = *closed {
            return report;
        }
        self.core.begin_shutdown();
        // From here on nothing is leased, recorded or submitted. The slot is free while the queue drains.
        let live = self.lock_slot().live.take();
        let let_go = live.map(|live| {
            self.drain(&live);
            self.let_go(live.number, live.vulkan)
        });
        let lost_devices_held = {
            let mut slot = self.lock_slot();
            slot.process = None;
            held_lost_devices(&mut slot)
        };
        let (fate, held) = match let_go {
            None => (DeviceFate::NoDevice, None),
            Some(LetGo::Destroyed) => (DeviceFate::Destroyed, None),
            Some(LetGo::Unconfirmed) => (DeviceFate::Unconfirmed, None),
            Some(LetGo::Held { holders, vulkan }) => {
                let holders = u64::try_from(holders).unwrap_or(u64::MAX);
                (DeviceFate::Held { holders }, Some(vulkan))
            }
        };
        let lost_devices_held = u64::try_from(lost_devices_held).unwrap_or(u64::MAX);
        let report = self.core.finish_shutdown(fate, lost_devices_held);
        *closed = Some(report);
        drop(closed);
        // Its holders may have left meanwhile: then it ends here, reported after the close.
        drop(held);
        report
    }

    /// Seal the queue of `live`, wait within the bound for what it took, and end what the device then says.
    fn drain(&self, live: &Live) {
        live.vulkan.seal();
        let value = match live.vulkan.drain(bounded_ns(MAX_WAIT)) {
            Ok(true) => live.vulkan.advance().map(Some),
            Ok(false) => Ok(None),
            Err(fault) => Err(fault),
        };
        match value {
            Ok(Some(value)) => {
                self.core.ledger().resolve_through(live.number, value);
            }
            Err(QueueFault::Lost) => {
                self.core.device_lost(live.number);
                self.core.ledger().resolve_lost(live.number);
            }
            // Not observed: the submissions end with the device, or stay in flight.
            Ok(None) | Err(_) => {}
        }
    }
}

/// Prune the set-aside devices that are gone and count the rest.
fn held_lost_devices(slot: &mut Slot) -> usize {
    slot.set_aside.retain(|device| device.strong_count() > 0);
    slot.set_aside.len()
}

/// Take the lost device out of the slot, setting it aside when clients hold it.
fn take_lost(slot: &mut Slot) -> Result<Option<Live>, DeviceRebindRefusal> {
    let held_before = held_lost_devices(slot);
    let Some(live) = slot.live.as_ref() else {
        return Ok(None);
    };
    if !live.vulkan.is_drained() {
        return Err(DeviceRebindRefusal::NotQuiescent);
    }
    // Nobody gets a new hold on it without the slot lock, so from here on they only become fewer.
    let holders = Arc::strong_count(&live.vulkan).saturating_sub(1);
    if holders > 0 {
        if held_before >= MAX_HELD_LOST_DEVICES {
            return Err(DeviceRebindRefusal::DeviceHeld {
                holders: u32::try_from(holders).unwrap_or(u32::MAX),
            });
        }
        let device = Arc::downgrade(&live.vulkan);
        slot.set_aside.push(device);
    }
    Ok(slot.live.take())
}
