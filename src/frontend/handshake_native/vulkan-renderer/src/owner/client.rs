//! What one module client holds: its own claims, and nothing of the owner's authority.

use super::OwnerShared;
use crate::device::{DeviceIdentity, DeviceLease, Recording};
use crate::ids::{ClientId, ModuleId};
use crate::ledger::{Claim, LedgerClient, Submission};
use crate::outcome::{
    AdmissionDenial, AuthorizeRefusal, ChargeRefusal, DeviceFault, RebindRefusal, RecordAborted,
    SettleRefusal, SubmissionRefusal, SubmitRefusal,
};
use crate::present::{Frame, OwnerSurface, Presented, SurfaceTarget};
use crate::records::{
    ChargeClass, ClaimRequest, DeviceBinding, ReleaseReceipt, SettleReceipt, Settlement,
    SubmissionState,
};
use crate::refusal::{PresentRefusal, ResourceRefusal};
use crate::resource::{BufferRequest, ImageRequest, OwnerBuffer, OwnerImage};
use std::sync::Arc;
use std::time::Duration;

/// One module client's handle on the GPU owner, issued by the host with [`crate::GpuOwner::attach_client`].
///
/// Its identity is fixed at issue, so every byte and every event is attributed to the client the host meant. It
/// reaches only claims and submissions of its own: another client's are unknown to it. Not `Clone`; dropping it
/// detaches the client. A revoked handle admits nothing any more and still settles what it holds; claims that outlive
/// the handle are discharged, settled and released through [`Claim`] itself.
pub struct GpuClient {
    owner: Arc<OwnerShared>,
    ledger: LedgerClient,
}

impl std::fmt::Debug for GpuClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuClient")
            .field("module", &self.ledger.module())
            .field("client", &self.ledger.client())
            .finish()
    }
}

impl GpuClient {
    pub(super) fn new(owner: Arc<OwnerShared>, ledger: LedgerClient) -> Self {
        Self { owner, ledger }
    }

    pub fn module(&self) -> ModuleId {
        self.ledger.module()
    }

    pub fn client(&self) -> ClientId {
        self.ledger.client()
    }

    /// Whether the host still admits this handle.
    pub fn is_attached(&self) -> bool {
        self.ledger.is_attached()
    }

    /// The adapter the owner runs on.
    pub fn identity(&self) -> DeviceIdentity {
        self.owner.identity()
    }

    /// The current device binding: generation number and whether it is lost.
    pub fn device(&self) -> Option<DeviceBinding> {
        self.owner.core.ledger().device()
    }

    /// Admit work of this client: reserve its bytes against the quota, or a typed denial.
    pub fn admit(&self, request: &ClaimRequest) -> Result<Claim, AdmissionDenial> {
        if !self.ledger.has_room(request) {
            // The work of dropped claims may have ended since somebody last asked the device.
            let _ = self.owner.poll();
        }
        self.ledger.admit(request)
    }

    pub fn authorize(&self, claim: &Claim) -> Result<(), AuthorizeRefusal> {
        self.ledger.authorize(claim)
    }

    /// Report memory the claim now holds inside its reservation.
    pub fn charge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        self.ledger.charge(claim, class, bytes)
    }

    /// Report memory the claim freed.
    pub fn discharge(
        &self,
        claim: &Claim,
        class: ChargeClass,
        bytes: u64,
    ) -> Result<(), ChargeRefusal> {
        self.ledger.discharge(claim, class, bytes)
    }

    /// The device hand-over for an authorized claim. See [`DeviceLease`] for what its holder must keep to.
    pub fn lease(&self, claim: &Claim) -> Result<DeviceLease, AuthorizeRefusal> {
        // Locked before the claim is authorized, so the handles belong to the generation it was authorized on.
        let slot = self.owner.lock_slot();
        let hold = self.ledger.hold(claim)?;
        match slot.live.as_ref() {
            Some(live) if live.number == claim.generation() => Ok(DeviceLease::new(
                live.vulkan.handles(live.number),
                claim.id(),
                Arc::clone(&live.vulkan),
                hold,
                Arc::clone(&self.owner),
            )),
            _ => Err(AuthorizeRefusal::DeviceUnavailable),
        }
    }

    /// Record work under `claim` and submit it. Returns the submission without waiting for it.
    ///
    /// `record` runs on this thread, before the owner's queue is touched and with no lock of the owner held, and
    /// only when the claim may submit and has room for one more submission. The claim is authorized again when the
    /// work is submitted, so a claim that is revoked, lost or settled while recording submits nothing.
    pub fn submit<F>(&self, claim: &Claim, record: F) -> Result<Submission, SubmitRefusal>
    where
        F: FnOnce(&Recording<'_>) -> Result<(), RecordAborted>,
    {
        self.owner.submit(&self.ledger, claim, record)
    }

    /// Make a render-attachment image for `claim`: created, placed and owned by the owner, charged to the claim inside
    /// its reservation, in the `UNDEFINED` layout. Dropping it releases it (see [`OwnerImage`]).
    pub fn create_image(
        &self,
        claim: &Claim,
        request: &ImageRequest,
    ) -> Result<OwnerImage, ResourceRefusal> {
        let made = self.owner.create_image(&self.ledger, claim, request);
        self.owner.reap();
        made
    }

    /// Make a buffer for `claim`, charged like an image: device memory as committed, upload and readback memory as
    /// staging.
    pub fn create_buffer(
        &self,
        claim: &Claim,
        request: &BufferRequest,
    ) -> Result<OwnerBuffer, ResourceRefusal> {
        let made = self.owner.create_buffer(&self.ledger, claim, request);
        self.owner.reap();
        made
    }

    /// Make a surface on the window `target` names, for `claim` (a [`crate::OperationKind::Display`] claim, by
    /// convention). The owner keeps `target`, and with it the window, until it destroyed the surface. Refused when the
    /// owner was opened without presentation.
    pub fn create_surface(
        &self,
        claim: &Claim,
        target: Box<dyn SurfaceTarget>,
    ) -> Result<OwnerSurface, ResourceRefusal> {
        self.owner.create_surface(&self.ledger, claim, target)
    }

    /// Record into `frame` under `claim`, submit the work waiting for the frame's acquire, and present it behind the
    /// work. Returns the submission at once, with how the present went; nothing on this path waits for the device.
    /// `record` gets the frame to record against ([`Recording::transition`], [`Recording::clear`]) and must leave its
    /// image in [`crate::ImageLayout::Present`].
    pub fn present<F>(
        &self,
        claim: &Claim,
        frame: Frame,
        record: F,
    ) -> Result<Presented, PresentRefusal>
    where
        F: FnOnce(&Recording<'_>, &Frame) -> Result<(), RecordAborted>,
    {
        self.owner.present(&self.ledger, claim, frame, record)
    }

    /// Where `submission` stands, asking the device when the owner does not know yet. Does not wait for work.
    ///
    /// A submission that was already observed answers from what was observed, also after its generation is gone. One
    /// that is pending is observed as long as its device exists, also after a close. On a refusal it stays pending.
    pub fn observe(&self, submission: &Submission) -> Result<SubmissionState, SubmissionRefusal> {
        self.owner.observe(&self.ledger, submission)
    }

    /// Wait at most `timeout`, and never longer than [`crate::MAX_WAIT`], for `submission`, then answer as
    /// [`Self::observe`] does. Other calls are not blocked by the wait.
    pub fn wait(
        &self,
        submission: &Submission,
        timeout: Duration,
    ) -> Result<SubmissionState, SubmissionRefusal> {
        self.owner.wait(&self.ledger, submission, timeout)
    }

    /// Settle the work of `claim` exactly once, with an outcome that does not contradict what the owner observed.
    pub fn settle(
        &self,
        claim: &Claim,
        settlement: Settlement,
    ) -> Result<SettleReceipt, SettleRefusal> {
        self.ledger.settle(claim, settlement)
    }

    /// Release the retained memory of a settled claim exactly once.
    pub fn release_retained(&self, claim: &Claim) -> Result<ReleaseReceipt, SettleRefusal> {
        self.ledger.release_retained(claim)
    }

    /// Move `claim` to the current generation after a device loss.
    pub fn rebind_claim(&self, claim: &Claim) -> Result<Claim, RebindRefusal> {
        self.ledger.rebind(claim)
    }

    /// Let the owner look at its device, without waiting. A client that saw `VK_ERROR_DEVICE_LOST` calls this; the
    /// owner marks the generation lost only from what it observes itself. See [`crate::GpuOwner::poll`].
    pub fn poll(&self) -> Result<(), DeviceFault> {
        self.owner.poll()
    }
}
