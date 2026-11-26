//! The owner's device-independent half: the claim ledger together with the diagnostics port and the
//! lifecycle transitions that do not need Vulkan. [`crate::GpuOwner`] adds the Vulkan generation on top.
//!
//! The ledger is asked first and the report comes afterwards, with the ledger lock released, so a diagnostics sink
//! can never deadlock or delay an admission decision.

use crate::diag::{OwnerDiagnostics, OwnerEvent, OwnerPhase};
use crate::ids::{DeviceKey, ModuleId};
use crate::ledger::{ClaimLedger, LedgerClient};
use crate::outcome::{AttachRefusal, DeviceFault, DeviceRebindRefusal, Unavailable};
use crate::records::{DeviceFate, QuotaPolicy};
use crate::ClientId;
use std::sync::Arc;

/// What the owner still held when it closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownReport {
    pub open_claims: u64,
    pub retained_claims: u64,
    pub charged_bytes: u64,
    pub in_flight: u64,
    pub leases: u64,
    /// What became of the device.
    pub device: DeviceFate,
    /// Lost devices of earlier generations that clients still hold.
    pub lost_devices_held: u64,
}

impl ShutdownReport {
    /// Whether everything was settled, released and destroyed.
    pub const fn is_settled(self) -> bool {
        self.open_claims == 0
            && self.retained_claims == 0
            && self.charged_bytes == 0
            && self.in_flight == 0
            && self.leases == 0
            && self.lost_devices_held == 0
            && matches!(self.device, DeviceFate::NoDevice | DeviceFate::Destroyed)
    }
}

/// See the module documentation.
pub struct OwnerCore {
    ledger: ClaimLedger,
    diagnostics: Arc<dyn OwnerDiagnostics>,
}

impl OwnerCore {
    pub fn new(policy: QuotaPolicy, diagnostics: Arc<dyn OwnerDiagnostics>) -> Self {
        Self {
            ledger: ClaimLedger::new(policy, Arc::clone(&diagnostics)),
            diagnostics,
        }
    }

    fn emit(&self, event: OwnerEvent) {
        self.diagnostics.record(event);
    }

    /// Report an event of the owner's own making (resources, presentation).
    pub(crate) fn report(&self, event: OwnerEvent) {
        self.emit(event);
    }

    /// The owner's side of the ledger.
    pub fn ledger(&self) -> &ClaimLedger {
        &self.ledger
    }

    /// Issue the handle of one client; see [`ClaimLedger::attach`].
    pub fn attach(
        &self,
        module: ModuleId,
        client: ClientId,
    ) -> Result<LedgerClient, AttachRefusal> {
        self.ledger.attach(module, client)
    }

    /// A device generation exists: bind it and open admission on it. Returns the generation.
    pub fn device_bound(&self, key: DeviceKey) -> u64 {
        let generation = self.ledger.bind_device(key);
        self.emit(OwnerEvent::Phase {
            phase: OwnerPhase::Open,
            generation,
        });
        generation
    }

    /// No device generation could be created.
    pub fn device_unavailable(&self, reason: Unavailable) {
        self.ledger.unbind_device();
        let vk_result = match reason {
            Unavailable::Driver(raw) => raw,
            _ => 0,
        };
        self.emit(OwnerEvent::Unavailable {
            reason: reason.number(),
            vk_result,
        });
        self.emit(OwnerEvent::Phase {
            phase: OwnerPhase::Unavailable,
            generation: 0,
        });
    }

    /// `VK_ERROR_DEVICE_LOST` was observed on `generation`. Idempotent: only the first report of the
    /// latest generation changes state and is reported. Claims keep their charge.
    pub fn device_lost(&self, generation: u64) -> bool {
        let first = self.ledger.mark_device_lost(generation);
        if first {
            self.emit(OwnerEvent::VulkanDeviceLost { generation });
            // A closed owner that keeps the device for its holders stays closed.
            if self.ledger.device().is_some() {
                self.emit(OwnerEvent::Phase {
                    phase: OwnerPhase::DeviceLost,
                    generation,
                });
            }
        }
        first
    }

    /// The device answered with a fault that is not device loss.
    pub fn device_fault(&self, generation: u64, fault: DeviceFault) {
        let vk_result = match fault {
            DeviceFault::Driver(raw) => raw,
            _ => 0,
        };
        self.emit(OwnerEvent::DeviceFault {
            generation,
            fault: fault.number(),
            vk_result,
        });
    }

    /// The owner starts replacing a lost generation.
    pub fn device_rebinding(&self, generation: u64) {
        self.emit(OwnerEvent::Phase {
            phase: OwnerPhase::Rebinding,
            generation,
        });
    }

    /// The owner did not replace its generation.
    pub fn device_rebind_refused(&self, refusal: DeviceRebindRefusal) {
        self.emit(OwnerEvent::DeviceRebindRefused {
            reason: refusal.number(),
        });
    }

    /// The lost device of `generation` stays alive for its holders, and the owner goes on without it.
    pub fn lost_device_set_aside(&self, generation: u64, holders: u64) {
        self.emit(OwnerEvent::LostDeviceSetAside {
            generation,
            holders,
        });
    }

    /// A device its owner could not destroy at once ended: `destroyed`, or left alive with the last handle of the owner
    /// because it did not confirm that its work has ended.
    pub fn retained_device_ended(&self, generation: u64, destroyed: bool) {
        if destroyed {
            let resolved = self.ledger.generation_retired(generation);
            self.emit(OwnerEvent::RetainedDeviceDestroyed {
                generation,
                resolved,
            });
        } else {
            self.ledger.report_abandoned();
            self.emit(OwnerEvent::DeviceLeaked { generation });
        }
    }

    /// The owner destroyed the device of `generation`; see [`ClaimLedger::generation_retired`].
    pub fn generation_retired(&self, generation: u64) -> u64 {
        let resolved = self.ledger.generation_retired(generation);
        self.emit(OwnerEvent::GenerationRetired {
            generation,
            resolved,
        });
        resolved
    }

    /// Stop admitting and attaching. Claims stay settleable.
    pub fn begin_shutdown(&self) {
        self.ledger.close_admission();
        let generation = self.ledger.device().map_or(0, |device| device.generation);
        self.emit(OwnerEvent::Phase {
            phase: OwnerPhase::ShuttingDown,
            generation,
        });
    }

    /// The owner let go of its device for good: `fate` says what became of it, and `lost_devices_held` how many
    /// earlier lost ones are still held by clients.
    ///
    /// Outstanding claims are not cleared: they stay settleable, and the report says truthfully what remained
    /// (GUI-DESKTOP-035).
    pub fn finish_shutdown(&self, fate: DeviceFate, lost_devices_held: u64) -> ShutdownReport {
        let generation = self.ledger.device().map_or(0, |device| device.generation);
        self.ledger.unbind_device();
        match fate {
            DeviceFate::NoDevice => {}
            DeviceFate::Destroyed => {
                self.generation_retired(generation);
            }
            DeviceFate::Held { holders } => self.emit(OwnerEvent::DeviceRetainedAtClose {
                generation,
                holders,
            }),
            DeviceFate::Unconfirmed => self.emit(OwnerEvent::DeviceRetainedAtClose {
                generation,
                holders: 0,
            }),
            DeviceFate::Creating => self.emit(OwnerEvent::DeviceCreatingAtClose),
        }
        let usage = self.ledger.usage();
        self.emit(OwnerEvent::ShutdownOutstanding {
            open_claims: usage.open_claims,
            retained_claims: usage.retained_claims,
            charged_bytes: usage.charged_bytes,
        });
        self.emit(OwnerEvent::Phase {
            phase: OwnerPhase::Closed,
            generation,
        });
        ShutdownReport {
            open_claims: usage.open_claims,
            retained_claims: usage.retained_claims,
            charged_bytes: usage.charged_bytes,
            in_flight: usage.in_flight,
            leases: usage.leases,
            device: fate,
            lost_devices_held,
        }
    }
}
