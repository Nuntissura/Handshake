//! Acquire and present: the frame path. Nothing here waits for the device: the acquire has timeout 0 and the present
//! returns behind its submission. Each takes only its own surface's lock, and a present the lane before it.

use super::super::queue::{
    create_fence, LaneGuard, QueueFault, RecordedWork, SubmitFault, SubmitSync,
};
use super::super::VulkanGeneration;
use super::retire::{presents_ended, Retirement};
use super::{
    fault, refused, Acquired, FrameRef, Note, PresentFault, PresentFns, Proof, ProofKind, Shown,
    SurfaceEntry, Swapchain,
};
use crate::present::{PresentCounters, PresentsProof};
use crate::refusal::PresentRefusal;
use ash::vk::{self, Handle};

/// Why one acquire gave no image.
enum NoImage {
    /// None is free now.
    NotReady,
    OutOfDate,
    SurfaceLost,
    /// No semaphore or fence could be made: nothing was asked of the swapchain.
    NotMade(vk::Result),
    /// A lost device, or an index the driver should not give: the semaphore and the fence went to the unknown ones.
    Unknown(PresentFault),
}

impl VulkanGeneration {
    /// Acquire the next image of surface `key` without waiting (`timeout` 0), with a semaphore for the submission that
    /// renders it and a fence that proves, once signalled, that the acquire and the image's earlier present have ended.
    pub(crate) fn acquire(&self, key: u64) -> Result<Acquired, PresentFault> {
        let fns = self.fns()?;
        let slot = self.surface_slot(key)?;
        let mut guard = slot.lock(&self.surfaces);
        let SurfaceEntry {
            current,
            free,
            proofs,
            counters,
            stale,
            lost,
            released,
            ended,
            ..
        } = &mut *guard;
        if *released || *ended {
            return Err(refused(PresentRefusal::Released));
        }
        if *lost {
            return Err(refused(PresentRefusal::SurfaceLost));
        }
        if *stale {
            return Err(refused(PresentRefusal::OutOfDate));
        }
        let Some(swapchain) = current.as_mut() else {
            return Err(refused(PresentRefusal::NoSwapchain));
        };
        if swapchain.suspended {
            return Err(refused(PresentRefusal::Suspended));
        }
        match self.acquire_one(fns, swapchain, free, proofs) {
            Ok((index, semaphore, suboptimal)) => {
                let at = index as usize;
                swapchain.out += 1;
                PresentCounters::bump(&counters.acquired);
                Ok(Acquired {
                    swapchain: swapchain.number,
                    index,
                    image: swapchain.images[at].as_raw(),
                    view: swapchain.views[at].as_raw(),
                    semaphore: semaphore.as_raw(),
                    format: swapchain.format,
                    extent: swapchain.extent,
                    usage: swapchain.usage,
                    suboptimal,
                })
            }
            Err(NoImage::NotReady) => {
                PresentCounters::bump(&counters.not_ready);
                Err(refused(PresentRefusal::NotReady))
            }
            Err(NoImage::OutOfDate) => {
                *stale = true;
                Err(fault(vk::Result::ERROR_OUT_OF_DATE_KHR))
            }
            Err(NoImage::SurfaceLost) => {
                *lost = true;
                Err(fault(vk::Result::ERROR_SURFACE_LOST_KHR))
            }
            Err(NoImage::NotMade(result)) => Err(fault(result)),
            Err(NoImage::Unknown(fault)) => {
                *stale = true;
                Err(fault)
            }
        }
    }

    /// One acquire of `swapchain` without waiting (`timeout` 0), with a semaphore and a fence that proves, once
    /// signalled, that the acquire and the image's earlier present have ended. Returns the image's index, its acquire
    /// semaphore and whether the swapchain is suboptimal. The caller holds the surface's lock.
    fn acquire_one(
        &self,
        fns: &PresentFns,
        swapchain: &mut Swapchain,
        free: &mut Vec<vk::Semaphore>,
        proofs: &mut Vec<Proof>,
    ) -> Result<(u32, vk::Semaphore, bool), NoImage> {
        let semaphore = match free.pop() {
            Some(semaphore) => semaphore,
            // SAFETY: valid device; a binary semaphore.
            None => unsafe {
                self.device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }
            .map_err(NoImage::NotMade)?,
        };
        let fence = match create_fence(&self.device) {
            Ok(fence) => fence,
            Err(result) => {
                free.push(semaphore);
                return Err(NoImage::NotMade(result));
            }
        };
        // SAFETY: a non-retired swapchain of this device (VUID-vkAcquireNextImageKHR-swapchain-01285); the semaphore is
        // binary, unsignalled and has nothing pending (VUID-...-semaphore-01286/01779/03265); the fence is new
        // (VUID-...-fence-01287/10066); timeout 0 waits for nothing, also with more images acquired than the surface
        // guarantees to give (VUID-vkAcquireNextImageKHR-surface-07783); the surface's lock synchronizes swapchain,
        // semaphore and fence.
        let acquired = unsafe {
            fns.swapchain
                .acquire_next_image(swapchain.handle, 0, semaphore, fence)
        };
        match acquired {
            Ok((index, suboptimal)) if (index as usize) < swapchain.images.len() => {
                let at = index as usize;
                proofs.push(Proof {
                    fence,
                    swapchain: swapchain.number,
                    kind: ProofKind::Acquire {
                        previous: swapchain.last_present_of[at],
                        index,
                    },
                });
                swapchain.acquiring += 1;
                swapchain.acquired[at] = true;
                Ok((index, semaphore, suboptimal))
            }
            Err(
                result @ (vk::Result::NOT_READY
                | vk::Result::TIMEOUT
                | vk::Result::ERROR_OUT_OF_DATE_KHR
                | vk::Result::ERROR_SURFACE_LOST_KHR),
            ) => {
                // No image was acquired, so neither the semaphore nor the fence was given a signal.
                free.push(semaphore);
                // SAFETY: the fence was never associated with an operation that runs.
                unsafe { self.device.destroy_fence(fence, None) };
                Err(match result {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => NoImage::OutOfDate,
                    vk::Result::ERROR_SURFACE_LOST_KHR => NoImage::SurfaceLost,
                    _ => NoImage::NotReady,
                })
            }
            other => {
                // A lost device, or an index the driver should not give: what the semaphore and fence carry now is not
                // known, so they go with the device.
                {
                    let mut unknown = self.surfaces.lock_unknown();
                    unknown.semaphores.push(semaphore);
                    unknown.fences.push(fence);
                }
                Err(NoImage::Unknown(match other {
                    Err(result) => fault(result),
                    Ok(_) => {
                        PresentFault::Device(QueueFault::Driver(vk::Result::ERROR_UNKNOWN.as_raw()))
                    }
                }))
            }
        }
    }

    /// Acquire back, without waiting, images of surface `key`'s current swapchain, so that their presents are proven
    /// ended by the fences of these acquires (MT-214 SF-2, [`super::Retirement`]), and say how far the presents of the
    /// surface's swapchains are proven. An image acquired here is handed to nobody: it stays acquired, and its semaphore
    /// keeps the acquire's signal, until the swapchain is destroyed. The swapchain is out of date from then on.
    pub(crate) fn prove_presents(&self, key: u64) -> Result<PresentsProof, PresentFault> {
        let fns = self.fns()?;
        let slot = self.surface_slot(key)?;
        let mut guard = slot.lock(&self.surfaces);
        let SurfaceEntry {
            current,
            retired,
            free,
            proofs,
            proven_present,
            stale,
            lost,
            released,
            ended,
            ..
        } = &mut *guard;
        if *released || *ended {
            return Err(refused(PresentRefusal::Released));
        }
        if *lost {
            return Err(refused(PresentRefusal::SurfaceLost));
        }
        let rule = fns.retirement();
        let proven = *proven_present;
        let older_wait = retired
            .iter()
            .any(|swapchain| !presents_ended(swapchain.standing(), rule, proven));
        let Some(swapchain) = current.as_mut() else {
            return Ok(if older_wait {
                PresentsProof::NeedsPresent
            } else {
                PresentsProof::Proven
            });
        };
        let standing = swapchain.standing();
        let current_ended = presents_ended(standing, rule, proven);
        if current_ended && !older_wait {
            return Ok(PresentsProof::Proven);
        }
        if standing.presents == 0 {
            // Its first present, once its image came back, proves the older ones no longer shown.
            return Ok(PresentsProof::NeedsPresent);
        }
        if current_ended || rule == Retirement::PresentFences {
            // The older ones end by the next look, or by their own present fences.
            return Ok(PresentsProof::Waiting);
        }
        let mut taken = false;
        for _ in 0..swapchain.images.len() {
            if swapchain.acquired.iter().all(|acquired| *acquired) {
                break;
            }
            match self.acquire_one(fns, swapchain, free, proofs) {
                Ok((_, semaphore, _)) => {
                    // Waited for by nothing; destroyed with the swapchain once its acquire has ended.
                    swapchain.stranded.push(semaphore);
                    taken = true;
                }
                Err(NoImage::NotReady) => break,
                Err(NoImage::OutOfDate) => {
                    *stale = true;
                    break;
                }
                Err(NoImage::SurfaceLost) => {
                    *lost = true;
                    return Err(fault(vk::Result::ERROR_SURFACE_LOST_KHR));
                }
                Err(NoImage::NotMade(result)) => return Err(fault(result)),
                Err(NoImage::Unknown(fault)) => {
                    *stale = true;
                    return Err(fault);
                }
            }
        }
        *stale |= taken;
        Ok(PresentsProof::Waiting)
    }

    /// A frame was dropped without a present: its image stays acquired and its semaphore signalled, so the swapchain
    /// has to be configured again and the semaphore ends with it.
    pub(crate) fn frame_dropped(&self, frame: FrameRef) {
        match self.surfaces.slot(frame.surface) {
            Some(slot) => slot.note(Note::Dropped(frame), &self.surfaces),
            None => self
                .surfaces
                .strand(vk::Semaphore::from_raw(frame.semaphore)),
        }
    }

    /// Submit `work`, which rendered the frame, waiting for its acquire and signalling its render semaphore, and present
    /// the frame behind it. The caller holds the lane and hands out `value` in order; this takes the frame's surface's
    /// lock and nothing else.
    ///
    /// No present is made without its proof: with `VK_EXT_swapchain_maintenance1` a present whose fence cannot be made
    /// is not made; without it the proof is the acquire fence of the image's next acquire, and a retired swapchain ends
    /// by the rule of [`super::Retirement::SuccessorPresent`].
    pub(crate) fn present(
        &self,
        lane: &LaneGuard<'_>,
        work: RecordedWork,
        value: u64,
        frame: FrameRef,
    ) -> Result<Shown, Result<SubmitFault, PresentFault>> {
        let fns = self.fns().map_err(Err)?;
        let acquire = vk::Semaphore::from_raw(frame.semaphore);
        let Some(slot) = self.surfaces.slot(frame.surface) else {
            // Its surface is gone: nothing waits for the acquire any more.
            self.surfaces.strand(acquire);
            return Err(Err(refused(PresentRefusal::Foreign)));
        };
        let mut guard = slot.lock(&self.surfaces);
        let at = frame.index as usize;
        let SurfaceEntry {
            current,
            retired,
            proofs,
            waiting,
            stale,
            lost,
            counters,
            ..
        } = &mut *guard;
        let is_current = current
            .as_ref()
            .is_some_and(|current| current.number == frame.swapchain);
        let found = current
            .iter_mut()
            .chain(retired.iter_mut())
            .find(|swapchain| swapchain.number == frame.swapchain)
            .filter(|swapchain| swapchain.acquired.get(at) == Some(&true));
        let Some(swapchain) = found else {
            self.surfaces.strand(acquire);
            return Err(Err(refused(PresentRefusal::Foreign)));
        };
        // From here on the frame is the owner's: presented, or its semaphore ends with its swapchain.
        swapchain.out = swapchain.out.saturating_sub(1);
        let fence = if fns.present_fences {
            match create_fence(&self.device) {
                Ok(fence) => Some(fence),
                Err(result) => {
                    swapchain.stranded.push(acquire);
                    *stale |= is_current;
                    PresentCounters::bump(&counters.failed);
                    return Err(Err(fault(result)));
                }
            }
        } else {
            None
        };
        let rendered = swapchain.rendered[at];
        let sync = SubmitSync {
            waits: &[acquire],
            signals: &[rendered],
        };
        if let Err(submit) = work.submit(lane, value, sync) {
            // The acquire's signal stays unwaited and the image acquired.
            swapchain.stranded.push(acquire);
            if let Some(fence) = fence {
                // SAFETY: never associated with an operation.
                unsafe { self.device.destroy_fence(fence, None) };
            }
            *stale |= is_current;
            PresentCounters::bump(&counters.failed);
            return Err(Ok(submit));
        }
        waiting.push((acquire, value));
        swapchain.last_use = swapchain.last_use.max(value);
        let waits = [rendered];
        let swapchains = [swapchain.handle];
        let indices = [frame.index];
        let fences = [fence.unwrap_or_else(vk::Fence::null)];
        let mut fenced = vk::SwapchainPresentFenceInfoEXT::default().fences(&fences);
        let mut info = vk::PresentInfoKHR::default()
            .wait_semaphores(&waits)
            .swapchains(&swapchains)
            .image_indices(&indices);
        if fence.is_some() {
            info = info.push_next(&mut fenced);
        }
        // SAFETY: the lane synchronizes the queue and the surface's lock the swapchain; the image index was acquired and
        // not presented since (VUID-VkPresentInfoKHR-pImageIndices-01430); the render semaphore's signal was submitted
        // just before and nothing else waits for it (VUID-vkQueuePresentKHR-pWaitSemaphores-01294/03268); the present
        // fence is new and unsignalled (VUID-VkSwapchainPresentFenceInfoEXT-pFences-07758/07759); all arrays outlive the
        // call.
        let result = unsafe { fns.swapchain.queue_present(lane.guard.queue, &info) };
        let shown = match result {
            Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY | vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => {
                // Not enqueued, and every resource and synchronization primitive it names is unaffected
                // (vkQueuePresentKHR): the image stays acquired, the fence was never given a signal, and the render
                // semaphore's signal is waited for by nothing, so the image is not used again from this swapchain.
                if let Some(fence) = fence {
                    // SAFETY: unaffected by the failed call, so never associated with an operation.
                    unsafe { self.device.destroy_fence(fence, None) };
                }
                *stale |= is_current;
                Shown::Fault(QueueFault::OutOfMemory)
            }
            result => {
                // Taken by the queue: also out of date and surface lost are enqueued and release the acquisition
                // (vkQueuePresentKHR). Another fault is counted as taken too, so the swapchain waits for its proof.
                swapchain.acquired[at] = false;
                swapchain.presents += 1;
                swapchain.last_present_of[at] = swapchain.presents;
                if let Some(fence) = fence {
                    proofs.push(Proof {
                        fence,
                        swapchain: swapchain.number,
                        kind: ProofKind::Present,
                    });
                    swapchain.unproven += 1;
                }
                match result {
                    Ok(false) => Shown::Presented,
                    Ok(true) => Shown::Suboptimal,
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                        *stale |= is_current;
                        Shown::OutOfDate
                    }
                    Err(vk::Result::ERROR_SURFACE_LOST_KHR) => {
                        *lost = true;
                        Shown::SurfaceLost
                    }
                    Err(other) => {
                        *stale |= is_current;
                        Shown::Fault(QueueFault::of(other))
                    }
                }
            }
        };
        shown.count(counters);
        Ok(shown)
    }
}
