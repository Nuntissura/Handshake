//! The end of presentation objects: released surfaces, retired swapchains, and everything at the destruction of the
//! generation. A swapchain is destroyed once the fences of the submissions that rendered into it were observed and its
//! presents are proven ended; a surface once it is released and none of its swapchains, fences or semaphores is left.

use super::super::VulkanGeneration;
use super::{Note, PresentEnd, PresentFns, SurfaceEntry};
use std::sync::{PoisonError, TryLockError};

impl VulkanGeneration {
    /// The client let go of surface `key`: its swapchains and the surface end once their work and presents did.
    pub(crate) fn release_surface(&self, key: u64) {
        self.note(Note::Released(key));
    }

    /// Look at the fences of acquires and presents without waiting, and destroy the swapchains and released surfaces
    /// whose work and presents are known to have ended. Does nothing while a present or an acquire holds the lock.
    pub(crate) fn retire_presentation(&self) -> Vec<PresentEnd> {
        let Some(fns) = &self.presentation else {
            return Vec::new();
        };
        let view = self.view();
        let mut guard = match self.surfaces.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return Vec::new(),
        };
        let surfaces = &mut *guard;
        surfaces.apply(self.notes.take());
        // A lost device's fences may say anything; only a drained one says that nothing runs.
        let gone = view.lost && view.drained;
        let mut ends = Vec::new();
        for entry in surfaces.entries.values_mut() {
            let mut kept = Vec::with_capacity(entry.proofs.len());
            for proof in std::mem::take(&mut entry.proofs) {
                // SAFETY: a fence of this device that lives until destroyed here; the query does not wait.
                let signalled = unsafe { self.device.get_fence_status(proof.fence) } == Ok(true);
                let ended = signalled && !view.lost;
                if !(ended || gone) {
                    kept.push(proof);
                    continue;
                }
                if ended {
                    surfaces.proven = surfaces.proven.max(proof.proves);
                }
                // SAFETY: signalled, or its device is drained: the acquire or present it was given has ended.
                unsafe { self.device.destroy_fence(proof.fence, None) };
                let owner = entry
                    .current
                    .iter_mut()
                    .chain(entry.retired.iter_mut())
                    .find(|swapchain| swapchain.number == proof.swapchain);
                if let Some(swapchain) = owner {
                    swapchain.pending_proofs = swapchain.pending_proofs.saturating_sub(1);
                }
            }
            entry.proofs = kept;
            let free = &mut entry.free;
            entry.waiting.retain(|(semaphore, value)| {
                let ended = gone || (!view.lost && *value <= view.observed);
                if ended {
                    free.push(*semaphore);
                }
                !ended
            });
        }
        let proven = surfaces.proven;
        for entry in surfaces.entries.values_mut() {
            let mut kept = Vec::with_capacity(entry.retired.len());
            for swapchain in std::mem::take(&mut entry.retired) {
                let ended = gone
                    || (!view.lost
                        && swapchain.last_use <= view.observed
                        && swapchain.pending_proofs == 0
                        && proven >= swapchain.last_present);
                if !ended {
                    kept.push(swapchain);
                    continue;
                }
                let number = swapchain.number;
                // SAFETY: every submission that rendered into it was observed, its presents are proven ended, and no
                // fence of it is pending (or its device is lost and drained).
                unsafe { self.destroy_swapchain(fns, swapchain) };
                ends.push(PresentEnd::Swapchain {
                    claim: entry.claim,
                    swapchain: number,
                });
            }
            entry.retired = kept;
        }
        let done: Vec<u64> = surfaces
            .entries
            .iter()
            .filter(|(_, entry)| {
                entry.released
                    && entry.current.is_none()
                    && entry.retired.is_empty()
                    && entry.proofs.is_empty()
                    && entry.waiting.is_empty()
            })
            .map(|(key, _)| *key)
            .collect();
        for key in done {
            if let Some(entry) = surfaces.entries.remove(&key) {
                let owned = entry.owned;
                // SAFETY: none of its swapchains exists any more (VUID-vkDestroySurfaceKHR-surface-01266).
                unsafe { self.destroy_surface(fns, entry) };
                ends.push(PresentEnd::Surface(owned));
            }
        }
        ends
    }

    /// # Safety
    /// None of its swapchains exists, and none of its semaphores is pending.
    unsafe fn destroy_surface(&self, fns: &PresentFns, entry: SurfaceEntry) {
        // SAFETY: the caller's.
        unsafe {
            for semaphore in entry.free {
                self.device.destroy_semaphore(semaphore, None);
            }
            fns.surface.destroy_surface(entry.surface, None);
        }
        // The window may go now.
        drop(entry.target);
    }

    /// Everything of the presentation, at the destruction of the generation: swapchains before the device, surfaces
    /// before the instance, and each window's target after its surface.
    ///
    /// # Safety
    /// `&mut self` at the destruction of the generation, with the evidence that nothing it took is still in use.
    pub(in crate::vulkan) unsafe fn destroy_presentation(&mut self) {
        let Some(fns) = &self.presentation else {
            return;
        };
        let notes = self.notes.take();
        let mut surfaces = std::mem::take(
            self.surfaces
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        surfaces.apply(notes);
        // SAFETY: the caller's: the queue is drained, so no acquire, render or present of these is still in use.
        unsafe {
            for mut entry in surfaces.entries.into_values() {
                for proof in std::mem::take(&mut entry.proofs) {
                    self.device.destroy_fence(proof.fence, None);
                }
                for swapchain in entry
                    .current
                    .take()
                    .into_iter()
                    .chain(std::mem::take(&mut entry.retired))
                {
                    self.destroy_swapchain(fns, swapchain);
                }
                for (semaphore, _) in std::mem::take(&mut entry.waiting) {
                    self.device.destroy_semaphore(semaphore, None);
                }
                self.destroy_surface(fns, entry);
            }
            for semaphore in surfaces.stranded {
                self.device.destroy_semaphore(semaphore, None);
            }
            for fence in surfaces.stranded_fences {
                self.device.destroy_fence(fence, None);
            }
        }
    }

    /// Leave everything of the presentation alive with a device that is left alive, windows included.
    pub(in crate::vulkan) fn abandon_presentation(&mut self) {
        let surfaces = std::mem::take(
            self.surfaces
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        std::mem::forget(surfaces);
    }
}
