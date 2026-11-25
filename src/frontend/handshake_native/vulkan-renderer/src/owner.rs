//! The GPU owner: the one Vulkan generation of the process together with the claim ledger.
//!
//! One owner per process (GUI-DESKTOP-027, HBR-HOST-003). Opening it either yields a real hardware Vulkan
//! device or a typed [`Unavailable`]; nothing in this unit substitutes another API or a CPU device.
//! Opening the device claims no positive quota: with `QuotaPolicy::Unavailable` every admission is denied
//! (GPU-QUOTA-012).
//!
//! Two capabilities, never one handle for both. [`GpuOwner`] is the host's: policy, clients, recovery and shutdown.
//! [`GpuClient`] is what the host hands to one module client: it carries that client's identity and reaches only its
//! own claims. A client cannot raise the quota, act for another client, revoke one, rebind the device or close the
//! owner.
//!
//! The owner does nothing on its own: no thread, no timer and no call per frame. A host that draws only on
//! damage may leave it untouched for as long as it is idle. What ended meanwhile is noticed at the next call that
//! asks the device, [`GpuOwner::poll`] being the one that does only that.
//!
//! No wait for the device runs with a lock of the owner held that a client's call needs.
//!
//! Not in this unit: the kernel issuer that authorizes the renderer-only shell and protected work
//! (NATIVE-RENDER-BOOT-002/007, GPU-QUOTA-011), the installation-wide persisted quota ledger
//! (GPU-QUOTA-006..010), windows, surfaces, swapchains and any drawing.

mod client;
#[cfg(test)]
mod fault_tests;
mod lifecycle;
#[cfg(test)]
mod pending_tests;
mod queue;

pub use client::GpuClient;
pub use lifecycle::MAX_HELD_LOST_DEVICES;
pub use queue::MAX_WAIT;

use crate::device::{DeviceIdentity, DeviceLease, MemorySnapshot};
use crate::diag::{OwnerDiagnostics, OwnerEvent, OwnerPhase};
use crate::ids::{ClientId, ModuleId};
use crate::outcome::{AttachRefusal, DeviceFault, DeviceHealth, DeviceRebindRefusal, Unavailable};
use crate::owner_core::{OwnerCore, ShutdownReport};
use crate::process::ProcessClaim;
use crate::records::{DeviceBinding, LedgerCounters, LedgerUsage, ModuleUsage, QuotaPolicy};
use crate::vulkan::VulkanGeneration;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

/// The bound generation and its number.
struct Live {
    number: u64,
    vulkan: Arc<VulkanGeneration>,
}

struct Slot {
    live: Option<Live>,
    /// Lost devices that clients still hold.
    set_aside: Vec<Weak<VulkanGeneration>>,
    /// The adapter as its latest generation reported it.
    identity: DeviceIdentity,
    /// Released at close; a device that clients still hold keeps its own share.
    process: Option<Arc<ProcessClaim>>,
}

/// What the owner and the handles it issued share.
///
/// Lock order: `closed`, then `slot`, then the locks inside the ledger and the generation. The client's recording
/// call, every wait for the device and the creation of one run with neither `slot` nor a lock inside held. Only the
/// draining in a close, which is bounded, runs with `closed` held.
struct OwnerShared {
    core: Arc<OwnerCore>,
    slot: Mutex<Slot>,
    closed: Mutex<Option<ShutdownReport>>,
    rebinding: AtomicBool,
}

impl OwnerShared {
    /// A poisoned lock is taken over: what it guards stays consistent at every step that can panic.
    fn lock_slot(&self) -> MutexGuard<'_, Slot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_closed(&self) -> MutexGuard<'_, Option<ShutdownReport>> {
        self.closed.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn identity(&self) -> DeviceIdentity {
        self.lock_slot().identity.clone()
    }
}

/// The host's authority over the GPU owner. Not `Clone`: it stays with the host that opened it, and modules get
/// [`GpuClient`] handles from it.
///
/// Dropping it closes the owner.
pub struct GpuOwner {
    shared: Arc<OwnerShared>,
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GpuOwner>();
    assert_send_sync::<GpuClient>();
    assert_send_sync::<DeviceLease>();
};

impl GpuOwner {
    /// Open the process's one GPU owner on a real hardware Vulkan device.
    ///
    /// Refusals are typed: no loader, no instance, below the Vulkan baseline, no device, only software
    /// or Direct3D-layered devices, missing feature, device creation refused, or an owner already open.
    pub fn open(
        policy: QuotaPolicy,
        diagnostics: Arc<dyn OwnerDiagnostics>,
    ) -> Result<Self, Unavailable> {
        let Some(process) = ProcessClaim::take() else {
            let reason = Unavailable::OwnerAlreadyOpen;
            diagnostics.record(OwnerEvent::Unavailable {
                reason: reason.number(),
                vk_result: 0,
            });
            return Err(reason);
        };
        diagnostics.record(OwnerEvent::Phase {
            phase: OwnerPhase::Opening,
            generation: 0,
        });
        let core = Arc::new(OwnerCore::new(policy, diagnostics));
        match VulkanGeneration::open(Arc::clone(&process), None) {
            Ok(generation) => {
                let identity = generation.identity().clone();
                let number = core.device_bound(identity.key);
                let live = Live {
                    number,
                    vulkan: Arc::new(generation),
                };
                Ok(Self {
                    shared: Arc::new(OwnerShared {
                        core,
                        slot: Mutex::new(Slot {
                            live: Some(live),
                            set_aside: Vec::new(),
                            identity,
                            process: Some(process),
                        }),
                        closed: Mutex::new(None),
                        rebinding: AtomicBool::new(false),
                    }),
                })
            }
            Err(reason) => {
                core.device_unavailable(reason);
                Err(reason)
            }
        }
    }

    /// Issue the handle of one module client. Its identity is what the host says here, and the handle cannot change
    /// it. A client that already has a live handle gets no second one.
    pub fn attach_client(
        &self,
        module: ModuleId,
        client: ClientId,
    ) -> Result<GpuClient, AttachRefusal> {
        let ledger = self.shared.core.attach(module, client)?;
        Ok(GpuClient::new(Arc::clone(&self.shared), ledger))
    }

    /// Revoke a client (logout, lost grant, detach). Its handle is never admitted again and its claims stop
    /// submitting; they keep their charge and still settle once. Only [`Self::attach_client`] lets that client in
    /// again, with a new handle. Other clients are not touched. Returns how many claims were revoked.
    pub fn revoke_client(&self, module: ModuleId, client: ClientId) -> u64 {
        self.shared.core.ledger().revoke_client(module, client)
    }

    /// Replace the quota policy. Existing claims keep their charge, bounds and original revision.
    pub fn set_policy(&self, policy: QuotaPolicy) {
        self.shared.core.ledger().set_policy(policy);
    }

    /// The adapter this owner runs on, as its latest generation reported it. The adapter stays the same across
    /// generations; its driver version may not.
    pub fn identity(&self) -> DeviceIdentity {
        self.shared.identity()
    }

    /// The current device binding: generation number and whether it is lost.
    pub fn device(&self) -> Option<DeviceBinding> {
        self.shared.core.ledger().device()
    }

    pub fn usage(&self) -> LedgerUsage {
        self.shared.core.ledger().usage()
    }

    pub fn usage_by_module(&self) -> Vec<ModuleUsage> {
        self.shared.core.ledger().usage_by_module()
    }

    pub fn counters(&self) -> LedgerCounters {
        self.shared.core.ledger().counters()
    }

    /// The device's own memory view (per-heap size, and budget and usage estimates when the device
    /// offers them).
    pub fn memory_snapshot(&self) -> Result<MemorySnapshot, DeviceFault> {
        match self.shared.lock_slot().live.as_ref() {
            Some(live) => Ok(live.vulkan.memory(live.number)),
            None => Err(DeviceFault::NoDevice),
        }
    }

    /// Ask the device, without waiting, how far its work got: end what finished, reclaim what was dropped and
    /// finished, and notice a loss.
    pub fn poll(&self) -> Result<(), DeviceFault> {
        self.shared.poll()
    }

    /// [`Self::poll`], after waiting at most `timeout`, and never longer than [`MAX_WAIT`], for the device to go
    /// idle. Other calls are not blocked by the wait.
    ///
    /// This is the owner's own observation of device loss: a client that sees `VK_ERROR_DEVICE_LOST` says so with
    /// [`GpuClient::poll`], and the owner marks the generation lost only from what it observes itself. Loss is
    /// reported once per generation through the diagnostics port.
    pub fn probe(&self, timeout: Duration) -> Result<DeviceHealth, DeviceFault> {
        self.shared.probe(timeout)
    }

    /// Replace a lost generation with a new one on the same adapter and return its number.
    ///
    /// The lost device first has to confirm that its work has ended; it is asked for at most [`MAX_WAIT`], and then
    /// with the one wait that Vulkan promises to end on a lost device. Then it is destroyed. While clients still hold
    /// it, it is set aside instead and destroyed with its last holder: recovery does not wait for them, and their
    /// next call on the lease tells them of the loss. At most [`MAX_HELD_LOST_DEVICES`] devices are set aside.
    ///
    /// Claims are not touched: each keeps its charge and is moved by its holder with [`GpuClient::rebind_claim`]. A
    /// different adapter is never taken instead; when the adapter is gone the owner stays without a device and this
    /// may be called again.
    pub fn rebind_device(&self) -> Result<u64, DeviceRebindRefusal> {
        self.shared.rebind_device()
    }

    /// Stop admitting and attaching, wait at most [`MAX_WAIT`] for the queue to drain, destroy the device generation
    /// and report what was still held. Idempotent: later calls return the first report.
    ///
    /// Claims that were not settled stay recorded as held; closing never reports them as settled
    /// (GUI-DESKTOP-035). A device that clients still lease is not destroyed under them: the report says so, and
    /// it is destroyed, and reported, when the last lease is dropped. Nor is a device destroyed that did not confirm
    /// in time that its work has ended.
    pub fn close(&self) -> ShutdownReport {
        self.shared.close()
    }
}

impl Drop for GpuOwner {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
