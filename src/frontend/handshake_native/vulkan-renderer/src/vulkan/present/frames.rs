//! Acquire and present: the frame path. Nothing here waits for the device: the acquire has timeout 0 and the present
//! returns behind its submission. Each takes only its own surface's lock, and a present the lane before it.

use super::super::queue::{
    create_fence, LaneGuard, QueueFault, RecordedWork, SubmitFault, SubmitSync,
};
use super::super::VulkanGeneration;
use super::{
    fault, refused, Acquired, FrameRef, Note, PresentFault, Proof, ProofKind, Shown, SurfaceEntry,
};
use crate::present::PresentCounters;
use crate::refusal::PresentRefusal;
use ash::vk::{self, Handle};

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
        let semaphore = match free.pop() {
            Some(semaphore) => semaphore,
            // SAFETY: valid device; a binary semaphore.
            None => unsafe {
                self.device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }
            .map_err(fault)?,
        };
        let fence = match create_fence(&self.device) {
            Ok(fence) => fence,
            Err(result) => {
                free.push(semaphore);
                return Err(fault(result));
            }
        };
        // SAFETY: a non-retired swapchain of this device (VUID-vkAcquireNextImageKHR-swapchain-01285); the semaphore is
        // binary, unsignalled and has nothing pending (VUID-...-semaphore-01286/01779/03265); the fence is new
        // (VUID-...-fence-01287/10066); timeout 0 waits for nothing; the surface's lock synchronizes swapchain, semaphore
        // and fence.
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
                    },
                });
                swapchain.acquiring += 1;
                swapchain.acquired[at] = true;
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
                match result {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => *stale = true,
                    vk::Result::ERROR_SURFACE_LOST_KHR => *lost = true,
                    _ => PresentCounters::bump(&counters.not_ready),
                }
                Err(match result {
                    vk::Result::NOT_READY | vk::Result::TIMEOUT => {
                        refused(PresentRefusal::NotReady)
                    }
                    other => fault(other),
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
                *stale = true;
                Err(match other {
                    Err(result) => fault(result),
                    Ok(_) => {
                        PresentFault::Device(QueueFault::Driver(vk::Result::ERROR_UNKNOWN.as_raw()))
                    }
                })
            }
        }
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
