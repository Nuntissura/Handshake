//! The owner's device-independent half: the claim ledger together with the diagnostics port and the
//! lifecycle transitions that do not need Vulkan. [`crate::GpuOwner`] adds the Vulkan generation on top.
//!
//! Every operation asks the ledger first and reports afterwards, with the ledger lock released, so a
//! diagnostics sink can never deadlock or delay an admission decision.

use crate::diag::{OwnerDiagnostics, OwnerEvent, OwnerPhase};
use crate::ids::{ClientId, DeviceKey, ModuleId};
use crate::ledger::{Claim, ClaimHold, ClaimLedger, Submission};
use crate::outcome::{
    AdmissionDenial, AuthorizeRefusal, ChargeRefusal, DeviceFault, DeviceRebindRefusal,
    RebindRefusal, SettleRefusal, SubmissionRefusal, SubmitRefusal, Unavailable,
};
use crate::records::{
    ChargeClass, ClaimDisposition, ClaimRequest, DeviceBinding, LedgerCounters, LedgerUsage,
    ModuleUsage, QuotaPolicy, ReleaseReceipt, SettleReceipt, Settlement,
};
use std::sync::Arc;

/// What the owner still held when it closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownReport {
    pub open_claims: u64,
    pub retained_claims: u64,
    pub charged_bytes: u64,
    pub in_flight: u64,
    pub leases: u64,
    /// The device was not destroyed: clients still hold it, or it did not confirm that its work has ended.
    pub device_retained: bool,
}

impl ShutdownReport {
    /// Whether everything was settled, released and destroyed.
    pub const fn is_settled(self) -> bool {
        self.open_claims == 0
            && self.retained_claims == 0
            && self.charged_bytes == 0
            && self.in_flight == 0
            && self.leases == 0
            && !self.device_retained
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
            ledger: ClaimLedger::new(policy),
            diagnostics,
        }
    }

    fn emit(&self, event: OwnerEvent) {
        self.diagnostics.record(event);
    }

    /// Replace the quota policy. Existing claims keep their charge, bounds and original revision.
    pub fn set_policy(&self, policy: QuotaPolicy) {
        self.ledger.set_policy(policy);
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
    /// current generation changes state and is reported. Claims keep their charge.
    pub fn device_lost(&self, generation: u64) -> bool {
        let first = self.ledger.mark_device_lost(generation);
        if first {
            self.emit(OwnerEvent::VulkanDeviceLost { generation });
            self.emit(OwnerEvent::Phase {
                phase: OwnerPhase::DeviceLost,
                generation,
            });
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

    pub fn device(&self) -> Option<DeviceBinding> {
        self.ledger.device()
    }

    pub fn admit(&self, request: &ClaimRequest) -> Result<Claim, AdmissionDenial> {
        let result = self.ledger.admit(request);
        match &result {
            Ok(claim) => self.emit(OwnerEvent::Admitted {
                generation: claim.generation(),
                reserved_bytes: claim.reserved_bytes(),
            }),
            Err(denial) => self.emit(OwnerEvent::AdmissionDenied {
                reason: denial.number(),
            }),
        }
        result
    }

    fn report_authorize<T>(
        &self,
        result: Result<T, AuthorizeRefusal>,
    ) -> Result<T, AuthorizeRefusal> {
        if let Err(refusal) = &result {
            self.emit(OwnerEvent::AuthorizeRefused {
                reason: refusal.number(),
            });
        }
        result
    }

    pub fn authorize(&self, claim: &Claim) -> Result<(), AuthorizeRefusal> {
        self.report_authorize(self.ledger.authorize(claim))
    }

    /// Authorize `claim` and take a hold on its device; see [`ClaimLedger::hold`].
    pub fn hold(&self, claim: &Claim) -> Result<ClaimHold, AuthorizeRefusal> {
        self.report_authorize(self.ledger.hold(claim))
    }

    fn report_charge(&self, result: Result<(), ChargeRefusal>) -> Result<(), ChargeRefusal> {
        if let Err(refusal) = &result {
            self.emit(OwnerEvent::ChargeRefused {
                reason: refusal.number(),
            });
        }
        result
    }

    pub fn charge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        self.report_charge(self.ledger.charge(claim, class, bytes))
    }

    pub fn discharge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        self.report_charge(self.ledger.discharge(claim, class, bytes))
    }

    /// Authorize one submission and count it in flight until the owner observes the end of it.
    pub fn begin_submission(&self, claim: &Claim) -> Result<Submission, AuthorizeRefusal> {
        self.report_authorize(self.ledger.begin_submission(claim))
    }

    /// Nothing was submitted after an authorization, for a reason other than authorization itself.
    pub fn submit_refused(&self, refusal: SubmitRefusal) {
        self.emit(OwnerEvent::SubmitRefused {
            reason: refusal.number(),
        });
    }

    /// Whether this owner issued `submission`.
    pub fn issued(&self, submission: &Submission) -> bool {
        self.ledger.issued(submission)
    }

    /// A submission is not observed.
    pub fn submission_refused(&self, refusal: SubmissionRefusal) {
        self.emit(OwnerEvent::SubmissionRefused {
            reason: refusal.number(),
        });
    }

    /// `submission` never reached the queue.
    pub fn submission_no_effect(&self, submission: &Submission) -> Result<(), SubmissionRefusal> {
        let result = self.ledger.submission_no_effect(submission);
        if let Err(refusal) = &result {
            self.submission_refused(*refusal);
        }
        result
    }

    /// The owner observed the fence of `generation` at `value`; see [`ClaimLedger::resolve_through`].
    pub fn resolve_through(&self, generation: u64, value: u64) -> u64 {
        self.ledger.resolve_through(generation, value)
    }

    /// The lost device of `generation` is quiescent; see [`ClaimLedger::resolve_lost`].
    pub fn resolve_lost(&self, generation: u64) -> u64 {
        self.ledger.resolve_lost(generation)
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

    pub fn settle(
        &self,
        claim: &Claim,
        settlement: Settlement,
    ) -> Result<SettleReceipt, SettleRefusal> {
        let result = self.ledger.settle(claim, settlement);
        match &result {
            Ok(receipt) => {
                let retained_bytes = match receipt.disposition {
                    ClaimDisposition::Closed => 0,
                    ClaimDisposition::Retained { charged_bytes } => charged_bytes,
                };
                self.emit(OwnerEvent::Settled {
                    outcome: receipt.settlement.outcome.number(),
                    released_bytes: receipt.released_bytes,
                    retained_bytes,
                });
            }
            Err(refusal) => self.emit(OwnerEvent::SettleRefused {
                reason: refusal.number(),
            }),
        }
        result
    }

    pub fn release_retained(&self, claim: &Claim) -> Result<ReleaseReceipt, SettleRefusal> {
        let result = self.ledger.release_retained(claim);
        match &result {
            Ok(receipt) => self.emit(OwnerEvent::RetainedReleased {
                released_bytes: receipt.released_bytes,
            }),
            Err(refusal) => self.emit(OwnerEvent::SettleRefused {
                reason: refusal.number(),
            }),
        }
        result
    }

    pub fn rebind(&self, claim: &Claim) -> Result<Claim, RebindRefusal> {
        let result = self.ledger.rebind(claim);
        match &result {
            Ok(successor) => self.emit(OwnerEvent::ClaimRebound {
                generation: successor.generation(),
            }),
            Err(refusal) => self.emit(OwnerEvent::ClaimRebindRefused {
                reason: refusal.number(),
            }),
        }
        result
    }

    pub fn revoke_client(&self, module: ModuleId, client: ClientId) -> u64 {
        let claims = self.ledger.revoke_client(module, client);
        self.emit(OwnerEvent::ClientRevoked { claims });
        claims
    }

    /// Stop admitting. Claims stay settleable.
    pub fn begin_shutdown(&self) {
        self.ledger.close_admission();
        let generation = self.ledger.device().map_or(0, |device| device.generation);
        self.emit(OwnerEvent::Phase {
            phase: OwnerPhase::ShuttingDown,
            generation,
        });
    }

    /// The owner let go of its device for good. `retained_by` is `None` when the device was destroyed (or never
    /// existed), or the number of clients that still hold it (0: it did not confirm that its work has ended).
    ///
    /// Outstanding claims are not cleared: they stay settleable, and the report says truthfully what remained
    /// (GUI-DESKTOP-035).
    pub fn finish_shutdown(&self, retained_by: Option<u64>) -> ShutdownReport {
        let generation = self.ledger.device().map_or(0, |device| device.generation);
        self.ledger.unbind_device();
        match retained_by {
            Some(holders) => self.emit(OwnerEvent::DeviceRetainedAtClose {
                generation,
                holders,
            }),
            None if generation > 0 => {
                self.generation_retired(generation);
            }
            None => {}
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
            device_retained: retained_by.is_some(),
        }
    }

    pub fn usage(&self) -> LedgerUsage {
        self.ledger.usage()
    }

    pub fn usage_by_module(&self) -> Vec<ModuleUsage> {
        self.ledger.usage_by_module()
    }

    pub fn counters(&self) -> LedgerCounters {
        self.ledger.counters()
    }
}
