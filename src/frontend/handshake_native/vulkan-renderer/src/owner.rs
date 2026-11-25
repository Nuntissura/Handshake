//! The GPU owner: the one Vulkan generation of the process together with the claim ledger.
//!
//! One owner per process (GUI-DESKTOP-027, HBR-HOST-003). Opening it either yields a real hardware Vulkan
//! device or a typed [`Unavailable`]; nothing in this unit substitutes another API or a CPU device.
//! Opening the device claims no positive quota: with `QuotaPolicy::Unavailable` every admission is denied
//! (GPU-QUOTA-012).
//!
//! The owner does nothing on its own: no thread, no timer and no call per frame. A host that draws only on
//! damage may leave it untouched for as long as it is idle.
//!
//! Not in this unit: the kernel issuer that authorizes the renderer-only shell and protected work
//! (NATIVE-RENDER-BOOT-002/007, GPU-QUOTA-011), the installation-wide persisted quota ledger
//! (GPU-QUOTA-006..010), windows, surfaces, swapchains and any drawing.

mod queue;

pub use queue::MAX_WAIT;

use crate::device::{DeviceIdentity, DeviceLease, MemorySnapshot};
use crate::diag::{OwnerDiagnostics, OwnerEvent, OwnerPhase};
use crate::ids::{ClientId, ModuleId};
use crate::ledger::Claim;
use crate::outcome::{
    AdmissionDenial, AuthorizeRefusal, ChargeRefusal, DeviceFault, DeviceRebindRefusal,
    RebindRefusal, SettleRefusal, Unavailable,
};
use crate::owner_core::{OwnerCore, ShutdownReport};
use crate::process::ProcessClaim;
use crate::records::{
    ChargeClass, ClaimRequest, DeviceBinding, LedgerCounters, LedgerUsage, ModuleUsage,
    QuotaPolicy, ReleaseReceipt, SettleReceipt, Settlement,
};
use crate::vulkan::{QueueFault, VulkanGeneration};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// The bound generation and its number.
struct Live {
    number: u64,
    vulkan: Arc<VulkanGeneration>,
}

struct Slot {
    live: Option<Live>,
    /// The adapter as its latest generation reported it.
    identity: DeviceIdentity,
    /// Released at close; a device that clients still hold keeps its own share.
    process: Option<Arc<ProcessClaim>>,
}

/// See the module documentation.
///
/// Lock order: `closed`, then `slot`, then the locks inside the ledger and the generation. The client's recording
/// call and bounded waits run with none of them held.
pub struct GpuOwner {
    core: OwnerCore,
    slot: Mutex<Slot>,
    closed: Mutex<Option<ShutdownReport>>,
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GpuOwner>();
    assert_send_sync::<DeviceLease>();
};

fn fault_of(fault: QueueFault, generation: u64) -> DeviceFault {
    match fault {
        QueueFault::Lost => DeviceFault::Lost { generation },
        QueueFault::OutOfMemory => DeviceFault::OutOfMemory,
        QueueFault::Driver(raw) => DeviceFault::Driver(raw),
    }
}

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
        let core = OwnerCore::new(policy, diagnostics);
        match VulkanGeneration::open(Arc::clone(&process), None) {
            Ok(generation) => {
                let identity = generation.identity().clone();
                let number = core.device_bound(identity.key);
                let live = Live {
                    number,
                    vulkan: Arc::new(generation),
                };
                Ok(Self {
                    core,
                    slot: Mutex::new(Slot {
                        live: Some(live),
                        identity,
                        process: Some(process),
                    }),
                    closed: Mutex::new(None),
                })
            }
            Err(reason) => {
                core.device_unavailable(reason);
                Err(reason)
            }
        }
    }

    /// A poisoned lock is taken over: what it guards stays consistent at every step that can panic.
    fn lock_slot(&self) -> MutexGuard<'_, Slot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_closed(&self) -> MutexGuard<'_, Option<ShutdownReport>> {
        self.closed.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The adapter this owner runs on, as its latest generation reported it. The adapter stays the same across
    /// generations; its driver version may not.
    pub fn identity(&self) -> DeviceIdentity {
        self.lock_slot().identity.clone()
    }

    /// The current device binding: generation number and whether it is lost.
    pub fn device(&self) -> Option<DeviceBinding> {
        self.core.device()
    }

    /// Replace the quota policy. Existing claims keep their charge, bounds and original revision.
    pub fn set_policy(&self, policy: QuotaPolicy) {
        self.core.set_policy(policy);
    }

    pub fn admit(&self, request: &ClaimRequest) -> Result<Claim, AdmissionDenial> {
        self.core.admit(request)
    }

    pub fn authorize(&self, claim: &Claim) -> Result<(), AuthorizeRefusal> {
        self.core.authorize(claim)
    }

    pub fn charge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        self.core.charge(claim, class, bytes)
    }

    pub fn discharge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        self.core.discharge(claim, class, bytes)
    }

    pub fn settle(
        &self,
        claim: &Claim,
        settlement: Settlement,
    ) -> Result<SettleReceipt, SettleRefusal> {
        self.core.settle(claim, settlement)
    }

    pub fn release_retained(&self, claim: &Claim) -> Result<ReleaseReceipt, SettleRefusal> {
        self.core.release_retained(claim)
    }

    /// Move `claim` to the current generation after a device loss.
    pub fn rebind_claim(&self, claim: &Claim) -> Result<Claim, RebindRefusal> {
        self.core.rebind(claim)
    }

    pub fn revoke_client(&self, module: ModuleId, client: ClientId) -> u64 {
        self.core.revoke_client(module, client)
    }

    pub fn usage(&self) -> LedgerUsage {
        self.core.usage()
    }

    pub fn usage_by_module(&self) -> Vec<ModuleUsage> {
        self.core.usage_by_module()
    }

    pub fn counters(&self) -> LedgerCounters {
        self.core.counters()
    }

    /// The device hand-over for an authorized claim. See [`DeviceLease`] for what its holder must keep to.
    pub fn lease(&self, claim: &Claim) -> Result<DeviceLease, AuthorizeRefusal> {
        // Locked before the claim is authorized, so the handles belong to the generation it was authorized on.
        let slot = self.lock_slot();
        let hold = self.core.hold(claim)?;
        match slot.live.as_ref() {
            Some(live) if live.number == claim.generation() => Ok(DeviceLease::new(
                live.vulkan.handles(live.number),
                claim.id(),
                Arc::clone(&live.vulkan),
                hold,
            )),
            _ => Err(AuthorizeRefusal::DeviceUnavailable),
        }
    }

    /// The device's own memory view (per-heap size, and budget and usage estimates when the device
    /// offers them).
    pub fn memory_snapshot(&self) -> Result<MemorySnapshot, DeviceFault> {
        match self.lock_slot().live.as_ref() {
            Some(live) => Ok(live.vulkan.memory(live.number)),
            None => Err(DeviceFault::NoDevice),
        }
    }

    /// The device of `live` answered with a fault. Only device loss marks the generation lost; it also ends the
    /// submissions of that generation once the device confirms that none of them is still in use.
    fn device_failed(&self, live: &Live, fault: QueueFault) -> DeviceFault {
        let typed = fault_of(fault, live.number);
        if fault == QueueFault::Lost {
            self.core.device_lost(live.number);
            if live.vulkan.quiesce().is_some() {
                self.core.resolve_lost(live.number);
            }
        } else {
            self.core.device_fault(live.number, typed);
        }
        typed
    }

    /// [`Self::device_failed`] for generation `number`, when that is still the bound one.
    fn generation_failed(&self, number: u64, fault: QueueFault) -> DeviceFault {
        let slot = self.lock_slot();
        match slot.live.as_ref() {
            Some(live) if live.number == number => self.device_failed(live, fault),
            _ => fault_of(fault, number),
        }
    }

    /// Wait for the device to go idle and report its health. This is the owner's own observation of
    /// device loss: a client that sees `VK_ERROR_DEVICE_LOST` asks for a probe, and the owner marks the
    /// generation lost only from what it observes itself. Loss is reported once per generation through
    /// the diagnostics port.
    pub fn probe(&self) -> Result<(), DeviceFault> {
        let slot = self.lock_slot();
        let Some(live) = slot.live.as_ref() else {
            return Err(DeviceFault::NoDevice);
        };
        if matches!(self.core.device(), Some(device) if device.lost) {
            // Known already; this only ends submissions an earlier observation could not.
            return Err(self.device_failed(live, QueueFault::Lost));
        }
        match live.vulkan.wait_idle().and_then(|()| live.vulkan.advance()) {
            Ok(value) => {
                self.core.resolve_through(live.number, value);
                Ok(())
            }
            Err(fault) => Err(self.device_failed(live, fault)),
        }
    }

    /// Replace a lost generation with a new one on the same adapter and return its number.
    ///
    /// The lost device is destroyed first, which is the owner's evidence that nothing submitted on it can
    /// still run. It is not destroyed under a client: while leases are out the rebind is refused. Claims are
    /// not touched: each keeps its charge and is moved by its holder with [`Self::rebind_claim`]. A different
    /// adapter is never taken instead; when the adapter is gone the owner stays without a device and this may
    /// be called again.
    pub fn rebind_device(&self) -> Result<u64, DeviceRebindRefusal> {
        let result = self.replace_device();
        if let Err(refusal) = &result {
            self.core.device_rebind_refused(*refusal);
        }
        result
    }

    fn replace_device(&self) -> Result<u64, DeviceRebindRefusal> {
        // Held for the whole rebind, so a concurrent `close` cannot finish between the check and the new device.
        let closed = self.lock_closed();
        if closed.is_some() {
            return Err(DeviceRebindRefusal::Closed);
        }
        let mut slot = self.lock_slot();
        if matches!(self.core.device(), Some(device) if !device.lost) {
            return Err(DeviceRebindRefusal::NotLost);
        }
        let Some(process) = slot.process.clone() else {
            return Err(DeviceRebindRefusal::Closed);
        };
        if let Some(Live { number, vulkan }) = slot.live.take() {
            // Nothing is submitted meanwhile: every submission needs the slot lock.
            let Some(evidence) = vulkan.quiesce() else {
                slot.live = Some(Live { number, vulkan });
                return Err(DeviceRebindRefusal::NotQuiescent);
            };
            match Arc::try_unwrap(vulkan) {
                Ok(generation) => {
                    self.core.device_rebinding(number);
                    generation.destroy(evidence);
                    self.core.generation_retired(number);
                }
                Err(vulkan) => {
                    let holders = Arc::strong_count(&vulkan).saturating_sub(1);
                    slot.live = Some(Live { number, vulkan });
                    return Err(DeviceRebindRefusal::DeviceHeld {
                        holders: u32::try_from(holders).unwrap_or(u32::MAX),
                    });
                }
            }
        }
        match VulkanGeneration::open(process, Some(slot.identity.key)) {
            Ok(generation) => {
                slot.identity = generation.identity().clone();
                let number = self.core.device_bound(slot.identity.key);
                slot.live = Some(Live {
                    number,
                    vulkan: Arc::new(generation),
                });
                Ok(number)
            }
            Err(reason) => {
                self.core.device_unavailable(reason);
                Err(DeviceRebindRefusal::Unavailable(reason))
            }
        }
    }

    /// Stop admitting, let the queue drain, destroy the device generation and report what was still held.
    /// Idempotent: later calls return the first report.
    ///
    /// Claims that were not settled stay recorded as held; closing never reports them as settled
    /// (GUI-DESKTOP-035). A device that clients still lease is not destroyed under them: the report says so, and
    /// it is destroyed when the last lease is dropped.
    pub fn close(&self) -> ShutdownReport {
        let mut closed = self.lock_closed();
        if let Some(report) = *closed {
            return report;
        }
        self.core.begin_shutdown();
        let mut slot = self.lock_slot();
        let retained_by = match slot.live.take() {
            None => None,
            Some(Live { number, vulkan }) => {
                let evidence = vulkan.quiesce();
                if evidence.is_some() {
                    match vulkan.advance() {
                        Ok(value) => {
                            self.core.resolve_through(number, value);
                        }
                        Err(QueueFault::Lost) => {
                            self.core.device_lost(number);
                            self.core.resolve_lost(number);
                        }
                        // Not observed: the submissions end with the device below, or stay in flight.
                        Err(_) => {}
                    }
                }
                match (Arc::try_unwrap(vulkan), evidence) {
                    (Ok(generation), Some(evidence)) => {
                        generation.destroy(evidence);
                        None
                    }
                    // Asks the device once more, and leaves it alive when it still cannot say.
                    (Ok(generation), None) => (!generation.dispose()).then_some(0),
                    (Err(vulkan), _) => {
                        let holders = Arc::strong_count(&vulkan).saturating_sub(1);
                        Some(u64::try_from(holders).unwrap_or(u64::MAX))
                    }
                }
            }
        };
        slot.process = None;
        let report = self.core.finish_shutdown(retained_by);
        *closed = Some(report);
        report
    }
}

impl Drop for GpuOwner {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
