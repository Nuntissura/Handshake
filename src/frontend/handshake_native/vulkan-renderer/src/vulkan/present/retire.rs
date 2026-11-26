//! The end of presentation objects: released surfaces, retired swapchains, and everything at the destruction of the
//! generation.
//!
//! # When a retired swapchain ends
//!
//! It is destroyed once, when all uses of its images have completed (VUID-vkDestroySwapchainKHR-swapchain-01282): no
//! frame of it is out with a client, the fences of the submissions that rendered into it were observed, the fences of
//! its acquires have signalled, and its presents are known to have ended. What proves the presents is the swapchain's
//! own, never another window's:
//! - With `VK_EXT_swapchain_maintenance1` ([`Retirement::PresentFences`]) every present carries a fence, and the swapchain
//!   may be destroyed "once the fences for all past presents to it have signaled" (VkSwapchainPresentFenceInfoKHR).
//! - Without it ([`Retirement::SuccessorPresent`]) by the rule of the Khronos `swapchain_recreation` sample: the old
//!   swapchain is destroyed when "the first present operation of the new swapchain is done, which means the old swapchain
//!   is no longer being presented". A present of a later swapchain of the same surface is known done when an acquire of
//!   that swapchain hands back the same image and its fence has signalled ("Swapchain Semaphore Reuse": acquiring the
//!   image index again and waiting on its fence establishes that the earlier present has finished).
//!
//! A swapchain that was never presented waits for nothing but its work. One that was presented and has no successor
//! (its surface was released) has nothing that proves its presents without the extension: the Vulkan guide says that
//! unextended Vulkan has no good solution for it, so it is kept, with its surface and the window's target, until the
//! device is destroyed with its queue drained. A surface ends once it is released and none of its swapchains, fences or
//! acquire semaphores is left.

use super::super::queue::QueueView;
use super::super::VulkanGeneration;
use super::{Note, PresentEnd, PresentFns, Proof, ProofKind, SurfaceEntry, Swapchain, Targets};
use crate::present::SurfaceTarget;
use std::sync::PoisonError;

/// How the presents of a retired swapchain are proven ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Retirement {
    /// By its own present fences (`VK_EXT_swapchain_maintenance1`).
    PresentFences,
    /// By a proven present of a later swapchain of the same surface.
    SuccessorPresent,
}

/// What the end of one retired swapchain waits for, as plain numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Standing {
    pub(super) number: u64,
    pub(super) presents: u64,
    pub(super) unproven: u32,
    pub(super) acquiring: u32,
    pub(super) out: u32,
    pub(super) last_use: u64,
}

impl Swapchain {
    fn standing(&self) -> Standing {
        Standing {
            number: self.number,
            presents: self.presents,
            unproven: self.unproven,
            acquiring: self.acquiring,
            out: self.out,
            last_use: self.last_use,
        }
    }
}

/// Whether a retired swapchain may be destroyed on a device that is not lost, where submissions up to `observed` were
/// seen completed and `proven_present` is the highest swapchain of its own surface with a present proven ended.
pub(super) fn may_end(
    standing: Standing,
    rule: Retirement,
    observed: u64,
    proven_present: u64,
) -> bool {
    let used = standing.out == 0 && standing.acquiring == 0 && standing.last_use <= observed;
    let shown = standing.presents == 0
        || match rule {
            Retirement::PresentFences => standing.unproven == 0,
            Retirement::SuccessorPresent => proven_present > standing.number,
        };
    used && shown
}

/// The swapchain a signalled proof shows a present of ended, if any.
pub(super) const fn proves_present(kind: ProofKind, swapchain: u64) -> Option<u64> {
    match kind {
        ProofKind::Acquire { previous } if previous > 0 => Some(swapchain),
        _ => None,
    }
}

impl SurfaceEntry {
    /// The fence of `proof` has signalled on a device that is not lost (`ended`), or is destroyed without a proof on a
    /// lost and drained one.
    fn settle(&mut self, proof: &Proof, ended: bool) {
        if let Some(swapchain) = self.swapchain_mut(proof.swapchain) {
            match proof.kind {
                ProofKind::Acquire { .. } => {
                    swapchain.acquiring = swapchain.acquiring.saturating_sub(1)
                }
                ProofKind::Present => swapchain.unproven = swapchain.unproven.saturating_sub(1),
            }
        }
        if let Some(number) = proves_present(proof.kind, proof.swapchain).filter(|_| ended) {
            self.proven_present = self.proven_present.max(number);
        }
    }
}

impl VulkanGeneration {
    /// The client let go of surface `key`: its swapchains and the surface end once their work and presents did.
    pub(crate) fn release_surface(&self, key: u64) {
        if let Some(slot) = self.surfaces.slot(key) {
            slot.note(Note::Released, &self.surfaces);
        }
    }

    /// Look at the fences of acquires and presents without waiting, and destroy the swapchains and released surfaces
    /// whose work and presents are known to have ended. A surface whose lock is held now (an acquire, a present) is
    /// looked at next time; its lock is the only one held while its swapchains and the surface are destroyed.
    pub(crate) fn retire_presentation(&self) -> Vec<PresentEnd> {
        let Some(fns) = &self.presentation else {
            return Vec::new();
        };
        let view = self.view();
        // A lost device's fences may say anything; only a drained one says that nothing runs.
        let gone = view.lost && view.drained;
        let rule = fns.retirement();
        let mut ends = Vec::new();
        for (key, slot) in self.surfaces.all() {
            let Some(mut entry) = slot.try_lock(&self.surfaces) else {
                continue;
            };
            self.settle_proofs(&mut entry, view, gone);
            let proven = entry.proven_present;
            let mut kept = Vec::with_capacity(entry.retired.len());
            for swapchain in std::mem::take(&mut entry.retired) {
                if !(gone
                    || (!view.lost && may_end(swapchain.standing(), rule, view.observed, proven)))
                {
                    kept.push(swapchain);
                    continue;
                }
                let number = swapchain.number;
                // SAFETY: no frame of it is out, every submission that rendered into it was observed, its acquires and
                // presents are proven ended, or its device is lost and drained.
                unsafe { self.destroy_swapchain(fns, swapchain) };
                ends.push(PresentEnd::Swapchain {
                    claim: entry.claim,
                    swapchain: number,
                });
            }
            entry.retired = kept;
            let done = entry.released
                && !entry.ended
                && entry.current.is_none()
                && entry.retired.is_empty()
                && entry.proofs.is_empty()
                && entry.waiting.is_empty();
            if done {
                // SAFETY: none of its swapchains exists any more and none of its semaphores is pending.
                let target = unsafe { self.end_surface(fns, &mut entry) };
                ends.push(PresentEnd::Surface {
                    owned: entry.owned,
                    target,
                });
                drop(entry);
                self.surfaces.remove(key);
            }
        }
        ends
    }

    /// Settle the proofs whose fences signalled, and free the acquire semaphores whose waits were observed.
    fn settle_proofs(&self, entry: &mut SurfaceEntry, view: QueueView, gone: bool) {
        let mut kept = Vec::with_capacity(entry.proofs.len());
        for proof in std::mem::take(&mut entry.proofs) {
            // SAFETY: a fence of this device that lives until destroyed here; the query does not wait.
            let signalled = unsafe { self.device.get_fence_status(proof.fence) } == Ok(true);
            let ended = signalled && !view.lost;
            if !(ended || gone) {
                kept.push(proof);
                continue;
            }
            entry.settle(&proof, ended);
            // SAFETY: signalled, or its device is drained: the acquire or present it was given has ended.
            unsafe { self.device.destroy_fence(proof.fence, None) };
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

    /// Destroy the surface of `entry` and hand back its window's target, which the caller drops with no lock held.
    ///
    /// # Safety
    /// None of its swapchains exists, and none of its semaphores is pending.
    unsafe fn end_surface(
        &self,
        fns: &PresentFns,
        entry: &mut SurfaceEntry,
    ) -> Option<Box<dyn SurfaceTarget>> {
        // SAFETY: the caller's (VUID-vkDestroySurfaceKHR-surface-01266, VUID-vkDestroySemaphore-semaphore-05149).
        unsafe {
            for semaphore in std::mem::take(&mut entry.free) {
                self.device.destroy_semaphore(semaphore, None);
            }
            fns.surface.destroy_surface(entry.surface, None);
        }
        entry.ended = true;
        entry.target.take()
    }

    /// Everything of the presentation, at the destruction of the generation: swapchains before the device, surfaces
    /// before the instance. The windows' targets are returned, for the owner to drop with no lock held.
    ///
    /// # Safety
    /// `&mut self` at the destruction of the generation, with the evidence that nothing it took is still in use.
    pub(in crate::vulkan) unsafe fn destroy_presentation(&mut self) -> Targets {
        let slots = std::mem::take(
            self.surfaces
                .slots
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        let unknown = std::mem::take(
            self.surfaces
                .unknown
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        let mut targets = Targets::new();
        let Some(fns) = &self.presentation else {
            return targets;
        };
        // SAFETY: the caller's: the queue is drained, so no acquire, render or present of these is still in use.
        unsafe {
            for slot in slots.into_values() {
                let mut entry = slot.lock(&self.surfaces);
                for proof in std::mem::take(&mut entry.proofs) {
                    self.device.destroy_fence(proof.fence, None);
                }
                let swapchains: Vec<Swapchain> = entry
                    .current
                    .take()
                    .into_iter()
                    .chain(std::mem::take(&mut entry.retired))
                    .collect();
                for swapchain in swapchains {
                    self.destroy_swapchain(fns, swapchain);
                }
                for (semaphore, _) in std::mem::take(&mut entry.waiting) {
                    self.device.destroy_semaphore(semaphore, None);
                }
                if !entry.ended {
                    targets.extend(self.end_surface(fns, &mut entry));
                }
            }
            for semaphore in unknown.semaphores {
                self.device.destroy_semaphore(semaphore, None);
            }
            for fence in unknown.fences {
                self.device.destroy_fence(fence, None);
            }
        }
        targets
    }

    /// Leave everything of the presentation alive with a device that is left alive, windows included.
    pub(in crate::vulkan) fn abandon_presentation(&mut self) {
        let slots = std::mem::take(
            self.surfaces
                .slots
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        );
        std::mem::forget(slots);
    }
}

#[cfg(test)]
mod tests {
    use super::super::ProofKind;
    use super::{may_end, proves_present, Retirement, Standing};

    /// A surface's own books, as the owner keeps them: its highest swapchain with a proven present, and a swapchain it
    /// retired.
    struct Window {
        proven_present: u64,
        retired: Standing,
    }

    impl Window {
        fn proof(&mut self, kind: ProofKind, swapchain: u64) {
            if let Some(number) = proves_present(kind, swapchain) {
                self.proven_present = self.proven_present.max(number);
            }
        }

        fn may_end(&self, rule: Retirement) -> bool {
            may_end(self.retired, rule, 100, self.proven_present)
        }
    }

    fn presented(number: u64) -> Standing {
        Standing {
            number,
            presents: 3,
            last_use: 7,
            ..Standing::default()
        }
    }

    #[test]
    fn a_retired_swapchain_ends_by_its_own_surface_and_never_by_another_window() {
        let rule = Retirement::SuccessorPresent;
        // Window A retired swapchain 1 for 3; window B presents with swapchain 2.
        let mut a = Window {
            proven_present: 0,
            retired: presented(1),
        };
        let mut b = Window {
            proven_present: 0,
            retired: presented(4),
        };
        // B's acquire hands back an image B presented: that proves B's present, and nothing of A.
        b.proof(ProofKind::Acquire { previous: 5 }, 2);
        assert!(
            !a.may_end(rule),
            "another window's present ends nothing of this one"
        );
        // A's new swapchain acquires an image it never presented: no present is proven.
        a.proof(ProofKind::Acquire { previous: 0 }, 3);
        assert!(!a.may_end(rule));
        // A maintenance1 present fence of the successor says nothing about the old one's presents either.
        a.proof(ProofKind::Present, 3);
        assert!(!a.may_end(rule));
        // A's successor gets back an image it presented: its first present is done, the old swapchain is not shown.
        a.proof(ProofKind::Acquire { previous: 1 }, 3);
        assert!(a.may_end(rule));
        // A present proven on the retired swapchain itself is no successor's.
        let mut c = Window {
            proven_present: 0,
            retired: presented(6),
        };
        c.proof(ProofKind::Acquire { previous: 2 }, 6);
        assert!(!c.may_end(rule));
    }

    #[test]
    fn a_retired_swapchain_waits_for_its_work_frames_acquires_and_presents() {
        let done = presented(1);
        for rule in [Retirement::SuccessorPresent, Retirement::PresentFences] {
            assert!(may_end(done, rule, 7, 2), "{rule:?}");
            assert!(
                !may_end(done, rule, 6, 2),
                "a submission that rendered into it runs"
            );
            let out = Standing { out: 1, ..done };
            assert!(!may_end(out, rule, 7, 2), "a frame of it is with a client");
            let acquiring = Standing {
                acquiring: 1,
                ..done
            };
            assert!(!may_end(acquiring, rule, 7, 2), "an acquire has not ended");
            let never = Standing {
                presents: 0,
                ..done
            };
            assert!(
                may_end(never, rule, 7, 0),
                "never presented: only its work counts"
            );
        }
        let unproven = Standing {
            unproven: 1,
            ..done
        };
        assert!(!may_end(unproven, Retirement::PresentFences, 7, 2));
        assert!(
            may_end(done, Retirement::PresentFences, 7, 0),
            "its own fences are enough"
        );
        assert!(
            !may_end(done, Retirement::SuccessorPresent, 7, 1),
            "no later swapchain was proven"
        );
        assert!(
            !may_end(done, Retirement::SuccessorPresent, 7, 0),
            "released, nothing proves its presents"
        );
    }
}
