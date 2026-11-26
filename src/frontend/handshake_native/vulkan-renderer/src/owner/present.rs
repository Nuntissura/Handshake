//! Surfaces, swapchains, acquires and presents for clients: the owner's side of `crate::present`, over the Vulkan calls
//! of `vulkan/present.rs`.
//!
//! A present records the client's commands into the owner's command buffer, then, with the lane held, submits them
//! waiting for the frame's acquire and signalling its render semaphore, and presents behind them. No call on this path
//! waits for the device.

use super::outbox::Quiet;
use super::queue::fault_of;
use super::{Live, OwnerShared};
use crate::device::Recording;
use crate::diag::{OwnerEvent, Subject};
use crate::ledger::{Claim, LedgerClient};
use crate::outcome::{AuthorizeRefusal, DeviceFault, RecordAborted, SubmitRefusal};
use crate::present::{
    Frame, OwnerSurface, PresentCounters, PresentResult, Presented, SurfaceOrigin, SurfaceTarget,
    SwapchainInfo, SwapchainRequest, SwapchainStatus,
};
use crate::records::ChargeClass;
use crate::refusal::{PresentRefusal, ResourceRefusal};
use crate::resource::RenderAttachment;
use crate::vulkan::{FrameRef, Owned, PresentFault, Shown, SurfaceFault};
use std::sync::Arc;

impl OwnerShared {
    /// Make a surface on the target's window for `claim`.
    pub(super) fn create_surface(
        self: &Arc<Self>,
        client: &LedgerClient,
        claim: &Claim,
        target: Box<dyn SurfaceTarget>,
    ) -> Result<OwnerSurface, ResourceRefusal> {
        if !self.presentation {
            let refusal = ResourceRefusal::PresentationNotEnabled;
            return Err(self.resource_refused(client, claim, None, refusal));
        }
        let live = self
            .resource_device(client, claim)
            .map_err(|refusal| self.resource_refused(client, claim, None, refusal))?;
        let made = live.vulkan.make_surface(target).map_err(|fault| {
            let refusal = match fault {
                SurfaceFault::NotEnabled => ResourceRefusal::PresentationNotEnabled,
                SurfaceFault::WindowUnsupported => ResourceRefusal::WindowUnsupported,
                SurfaceFault::Device(fault) => {
                    ResourceRefusal::Device(fault_of(fault, live.number))
                }
            };
            self.resource_refused(client, claim, Some(&live), refusal)
        })?;
        if let Err(refusal) = client.own(claim, ChargeClass::Committed, 0) {
            // No lock of the owner is held here: the window's target may go at once.
            drop(live.vulkan.discard_surface(made));
            let refusal = ResourceRefusal::Charge(refusal);
            return Err(self.resource_refused(client, claim, None, refusal));
        }
        let counters = Arc::new(PresentCounters::new());
        let owned = Owned {
            claim: claim.id().get(),
            class: ChargeClass::Committed,
            bytes: 0,
        };
        let key = live
            .vulkan
            .keep_surface(made, owned, claim.info(), Arc::clone(&counters));
        Ok(OwnerSurface {
            owner: Arc::clone(self),
            origin: SurfaceOrigin {
                ledger: client.ledger_key(),
                seat: client.seat(),
                claim: claim.info(),
                key,
            },
            counters,
        })
    }

    /// The bound generation of a surface, when its claim may still use it.
    fn surface_device(&self, origin: &SurfaceOrigin) -> Result<Live, PresentRefusal> {
        let generation = origin.claim.generation;
        self.core
            .ledger()
            .standing_of(origin.claim.id.get())
            .map_err(PresentRefusal::NotAuthorized)?;
        match self.lock_slot().live.clone() {
            Some(live) if live.number == generation => Ok(live),
            _ => Err(PresentRefusal::NotAuthorized(AuthorizeRefusal::Lost {
                generation,
            })),
        }
    }

    /// A refusal for the client, reported, with what a device fault in it means for the generation.
    fn present_refused(
        &self,
        origin: &SurfaceOrigin,
        live: Option<&Live>,
        fault: PresentFault,
    ) -> PresentRefusal {
        let refusal = match (fault, live) {
            (PresentFault::Refused(refusal), _) => refusal,
            (PresentFault::Device(fault), Some(live)) => {
                PresentRefusal::Device(self.device_failed(live, fault))
            }
            (PresentFault::Device(fault), None) => {
                PresentRefusal::Device(fault_of(fault, origin.claim.generation))
            }
        };
        if refusal != PresentRefusal::NotReady {
            self.core.report(OwnerEvent::PresentRefused {
                subject: Subject::claim(&origin.claim),
                reason: refusal.number(),
            });
        }
        refusal
    }

    pub(crate) fn configure_surface(
        &self,
        origin: &SurfaceOrigin,
        request: &SwapchainRequest,
    ) -> Result<SwapchainStatus, PresentRefusal> {
        let live = self.surface_device(origin).map_err(|refusal| {
            self.present_refused(origin, None, PresentFault::Refused(refusal))
        })?;
        let configured = live.vulkan.configure(origin.key, request);
        let status =
            configured.map_err(|fault| self.present_refused(origin, Some(&live), fault))?;
        let (swapchain, width, height) = match status {
            SwapchainStatus::Ready(info) => (info.swapchain, info.extent.width, info.extent.height),
            SwapchainStatus::Suspended => (0, 0, 0),
        };
        self.core.report(OwnerEvent::SwapchainChanged {
            subject: Subject::claim(&origin.claim),
            swapchain,
            width,
            height,
        });
        // A swapchain retired just now may end at once.
        let _ = self.look(&live);
        Ok(status)
    }

    pub(crate) fn suspend_surface(&self, origin: &SurfaceOrigin) -> Result<(), PresentRefusal> {
        let live = self.surface_device(origin)?;
        live.vulkan
            .suspend(origin.key)
            .map_err(|fault| self.present_refused(origin, Some(&live), fault))?;
        self.core.report(OwnerEvent::SwapchainChanged {
            subject: Subject::claim(&origin.claim),
            swapchain: 0,
            width: 0,
            height: 0,
        });
        Ok(())
    }

    pub(crate) fn swapchain_info(&self, origin: &SurfaceOrigin) -> Option<SwapchainInfo> {
        let live = self
            .lock_slot()
            .generation(origin.claim.generation)
            .cloned()?;
        live.vulkan.swapchain_info(origin.key)
    }

    pub(crate) fn acquire_frame(
        self: &Arc<Self>,
        origin: &SurfaceOrigin,
        counters: &Arc<PresentCounters>,
    ) -> Result<Frame, PresentRefusal> {
        let live = self.surface_device(origin).map_err(|refusal| {
            self.present_refused(origin, None, PresentFault::Refused(refusal))
        })?;
        let acquired = live.vulkan.acquire(origin.key);
        // Fences of earlier acquires and presents may have signalled.
        let _ = self.look(&live);
        let acquired =
            acquired.map_err(|fault| self.present_refused(origin, Some(&live), fault))?;
        Ok(Frame {
            owner: Arc::clone(self),
            surface: *origin,
            swapchain: acquired.swapchain,
            index: acquired.index,
            semaphore: acquired.semaphore,
            attachment: RenderAttachment {
                image: acquired.image,
                view: acquired.view,
                format: acquired.format,
                extent: acquired.extent,
                usage: acquired.usage,
                generation: live.number,
            },
            suboptimal: acquired.suboptimal,
            counters: Arc::clone(counters),
            pending: true,
        })
    }

    /// A frame was dropped without a present.
    pub(crate) fn abandon_frame(&self, frame: &Frame) {
        let live = self
            .lock_slot()
            .generation(frame.surface.claim.generation)
            .cloned();
        if let Some(live) = live {
            live.vulkan.frame_dropped(frame_ref(frame));
        }
    }

    pub(crate) fn release_surface(&self, origin: &SurfaceOrigin) {
        let live = self
            .lock_slot()
            .generation(origin.claim.generation)
            .cloned();
        if let Some(live) = live {
            live.vulkan.release_surface(origin.key);
            let _ = self.look(&live);
        }
        self.reap();
    }

    /// Record into the frame, submit and present it; see the module documentation.
    pub(super) fn present<F>(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        mut frame: Frame,
        record: F,
    ) -> Result<Presented, PresentRefusal>
    where
        F: FnOnce(&Recording<'_>, &Frame) -> Result<(), RecordAborted>,
    {
        let result = self.present_frame(client, claim, &mut frame, record);
        // Dropped here: without a present that took its semaphore, as abandoned.
        drop(frame);
        self.reap();
        result
    }

    fn present_frame<F>(
        &self,
        client: &LedgerClient,
        claim: &Claim,
        frame: &mut Frame,
        record: F,
    ) -> Result<Presented, PresentRefusal>
    where
        F: FnOnce(&Recording<'_>, &Frame) -> Result<(), RecordAborted>,
    {
        let origin = frame.surface;
        if origin.ledger != client.ledger_key()
            || origin.seat != client.seat()
            || origin.claim.generation != claim.generation()
        {
            let refusal = PresentFault::Refused(PresentRefusal::Foreign);
            return Err(self.present_refused(&origin, None, refusal));
        }
        // The one recording in which the frame may be used: its submission waits for the frame's acquire.
        let key = frame.key();
        let work = self
            .record(client, claim, Some(key), |recording| {
                record(recording, &*frame)
            })
            .map_err(|refusal| match refusal {
                SubmitRefusal::RecordAborted => PresentRefusal::RecordAborted,
                SubmitRefusal::NotAuthorized(refusal) => PresentRefusal::NotAuthorized(refusal),
                SubmitRefusal::Device(fault) => PresentRefusal::Device(fault),
            })?;
        let generation = Arc::clone(work.generation());
        // Declared first, so it ends last: nothing reaches the sink while the lane is held.
        let _quiet = Quiet::begin(&self.outbox);
        let lane = generation.lane();
        let live = self
            .submitting_on(client, claim, &work)
            .map_err(|refusal| match refusal {
                SubmitRefusal::NotAuthorized(refusal) => PresentRefusal::NotAuthorized(refusal),
                _ => PresentRefusal::Foreign,
            })?;
        let submission = client
            .begin_submission(claim)
            .map_err(PresentRefusal::NotAuthorized)?;
        // From here on the owner disposes of the frame's semaphore, whatever happens.
        frame.pending = false;
        let shown = live
            .vulkan
            .present(&lane, work, submission.sequence(), frame_ref(frame));
        drop(lane);
        let result = match shown {
            Ok(Shown::Presented) => PresentResult::Presented,
            Ok(Shown::Suboptimal) => PresentResult::Suboptimal,
            Ok(Shown::OutOfDate) => PresentResult::OutOfDate,
            Ok(Shown::SurfaceLost) => PresentResult::SurfaceLost,
            Ok(Shown::Fault(fault)) => {
                // The submission was queued; the owner observes it like any other.
                let fault = PresentFault::Device(fault);
                return Err(self.present_refused(&origin, Some(&live), fault));
            }
            Err(Ok(fault)) => {
                let refusal = self.submit_failed(client, claim, &live, &submission, fault);
                return Err(match refusal {
                    SubmitRefusal::Device(fault) => PresentRefusal::Device(fault),
                    _ => PresentRefusal::Device(DeviceFault::NoDevice),
                });
            }
            Err(Err(fault)) => {
                // Nothing reached the queue. A device fault in it is reported once, by `present_refused`.
                let _ = self.core.ledger().submission_no_effect(&submission);
                return Err(self.present_refused(&origin, Some(&live), fault));
            }
        };
        if matches!(
            result,
            PresentResult::OutOfDate | PresentResult::SurfaceLost
        ) {
            let refusal = match result {
                PresentResult::OutOfDate => PresentRefusal::OutOfDate,
                _ => PresentRefusal::SurfaceLost,
            };
            self.core.report(OwnerEvent::PresentRefused {
                subject: Subject::claim(&origin.claim),
                reason: refusal.number(),
            });
        }
        let _ = self.look(&live);
        Ok(Presented { submission, result })
    }
}

fn frame_ref(frame: &Frame) -> FrameRef {
    FrameRef {
        surface: frame.surface.key,
        swapchain: frame.swapchain,
        index: frame.index,
        semaphore: frame.semaphore,
    }
}
