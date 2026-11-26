//! Acquire and present: the frame path. Nothing here waits for the device: the acquire has timeout 0 and the present
//! returns behind its submission.

use super::super::queue::{
    create_fence, LaneGuard, QueueFault, RecordedWork, SubmitFault, SubmitSync,
};
use super::super::VulkanGeneration;
use super::{fault, refused, Acquired, FrameRef, Note, PresentFault, Proof, Shown, SurfaceEntry};
use crate::present::PresentCounters;
use crate::refusal::PresentRefusal;
use ash::vk::{self, Handle};

impl VulkanGeneration {
    /// Acquire the next image of surface `key` without waiting (`timeout` 0), with a semaphore for the submission that
    /// renders it and a fence that proves, once signalled, that the image's earlier present has ended.
    pub(crate) fn acquire(&self, key: u64) -> Result<Acquired, PresentFault> {
        let fns = self.fns()?;
        let mut guard = self.lock_surfaces();
        let surfaces = &mut *guard;
        let entry = match surfaces.entries.get_mut(&key) {
            Some(entry) if !entry.released => entry,
            _ => return Err(refused(PresentRefusal::Released)),
        };
        if entry.lost {
            return Err(refused(PresentRefusal::SurfaceLost));
        }
        if entry.stale {
            return Err(refused(PresentRefusal::OutOfDate));
        }
        let Some(swapchain) = entry.current.as_mut() else {
            return Err(refused(PresentRefusal::NoSwapchain));
        };
        if swapchain.suspended {
            return Err(refused(PresentRefusal::Suspended));
        }
        let semaphore = match entry.free.pop() {
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
                entry.free.push(semaphore);
                return Err(fault(result));
            }
        };
        // SAFETY: a non-retired swapchain of this device (VUID-vkAcquireNextImageKHR-swapchain-01285); the semaphore is
        // binary, unsignalled and has nothing pending (VUID-...-semaphore-01286/01779/03265); the fence is new
        // (VUID-...-fence-01287/10066); timeout 0 waits for nothing; the surfaces lock synchronizes swapchain, semaphore
        // and fence.
        let acquired = unsafe {
            fns.swapchain
                .acquire_next_image(swapchain.handle, 0, semaphore, fence)
        };
        match acquired {
            Ok((index, suboptimal)) if (index as usize) < swapchain.images.len() => {
                let at = index as usize;
                entry.proofs.push(Proof {
                    fence,
                    proves: swapchain.last_present_of[at],
                    swapchain: swapchain.number,
                });
                swapchain.pending_proofs += 1;
                swapchain.acquired[at] = true;
                PresentCounters::bump(&entry.counters.acquired);
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
                entry.free.push(semaphore);
                // SAFETY: the fence was never associated with an operation that runs.
                unsafe { self.device.destroy_fence(fence, None) };
                match result {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => entry.stale = true,
                    vk::Result::ERROR_SURFACE_LOST_KHR => entry.lost = true,
                    _ => PresentCounters::bump(&entry.counters.not_ready),
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
                surfaces.stranded.push(semaphore);
                surfaces.stranded_fences.push(fence);
                entry.stale = true;
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
    /// has to be configured again and the semaphore goes with the device.
    pub(crate) fn frame_dropped(&self, frame: FrameRef) {
        self.note(Note::Dropped(frame));
    }

    /// Submit `work`, which rendered the frame, waiting for its acquire and signalling its render semaphore, and present
    /// the frame behind it. The caller holds the lane and hands out `value` in order.
    pub(crate) fn present(
        &self,
        lane: &LaneGuard<'_>,
        work: RecordedWork,
        value: u64,
        frame: FrameRef,
    ) -> Result<Shown, Result<SubmitFault, PresentFault>> {
        let fns = self.fns().map_err(Err)?;
        let mut guard = self.lock_surfaces();
        let surfaces = &mut *guard;
        let acquire = vk::Semaphore::from_raw(frame.semaphore);
        let found = surfaces.entries.get_mut(&frame.surface).and_then(|entry| {
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
            } = entry;
            let swapchain = current
                .iter_mut()
                .chain(retired.iter_mut())
                .find(|swapchain| swapchain.number == frame.swapchain)
                .filter(|swapchain| swapchain.acquired.get(at) == Some(&true))?;
            Some((swapchain, proofs, waiting, stale, lost, counters))
        });
        let Some((swapchain, proofs, waiting, stale, lost, counters)) = found else {
            // Its swapchain is gone: nothing waits for the acquire any more.
            surfaces.stranded.push(acquire);
            return Err(Err(refused(PresentRefusal::Foreign)));
        };
        let at = frame.index as usize;
        let rendered = swapchain.rendered[at];
        let sync = SubmitSync {
            waits: &[acquire],
            signals: &[rendered],
        };
        if let Err(submit) = work.submit(lane, value, sync) {
            // The acquire's signal stays unwaited, and the image acquired.
            surfaces.stranded.push(acquire);
            *stale = true;
            PresentCounters::bump(&counters.failed);
            return Err(Ok(submit));
        }
        waiting.push((acquire, value));
        swapchain.last_use = swapchain.last_use.max(value);
        surfaces.queued += 1;
        let number = surfaces.queued;
        let fence = if fns.present_fences {
            create_fence(&self.device).ok()
        } else {
            None
        };
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
        // SAFETY: the lane synchronizes the queue and the surfaces lock the swapchain; the image index was acquired and
        // not presented since; the render semaphore's signal was submitted just before and nothing else waits for it
        // (VUID-vkQueuePresentKHR-pWaitSemaphores-01294/03268); the present fence is new and unsignalled
        // (VUID-VkSwapchainPresentFenceInfoEXT-pFences-07758/07759); all arrays outlive the call.
        let result = unsafe { fns.swapchain.queue_present(lane.guard.queue, &info) };
        swapchain.acquired[at] = false;
        swapchain.last_present_of[at] = number;
        swapchain.last_present = number;
        if let Some(fence) = fence {
            proofs.push(Proof {
                fence,
                proves: number,
                swapchain: swapchain.number,
            });
            swapchain.pending_proofs += 1;
        }
        let shown = match result {
            Ok(false) => Shown::Presented,
            Ok(true) => Shown::Suboptimal,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                *stale = true;
                Shown::OutOfDate
            }
            Err(vk::Result::ERROR_SURFACE_LOST_KHR) => {
                *lost = true;
                Shown::SurfaceLost
            }
            Err(other) => Shown::Fault(QueueFault::of(other)),
        };
        match shown {
            Shown::Presented => counters.mark_presented(),
            Shown::Suboptimal => {
                counters.mark_presented();
                PresentCounters::bump(&counters.suboptimal);
            }
            _ => PresentCounters::bump(&counters.failed),
        }
        Ok(shown)
    }
}
