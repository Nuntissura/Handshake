//! The end of windows and of the loop: closing a window, settling the claims of windows that are gone, and shutdown.
//!
//! Without `VK_EXT_swapchain_maintenance1` nothing but a surface's own images proves that its last presents ended, and a
//! surface released without that proof is kept, with its window, until the owner closes. So a closing window that
//! presented first gets that proof (MT-214 SF-2): a new swapchain, one present on it while the window is still shown,
//! then the presenter's looks ask the owner to acquire back its images until the image of that present came back, which
//! ends the old swapchain by its successor's present and the new one by its own image. Then the surface is released,
//! the owner ends it with its work, and the window is destroyed. Nothing waits for the device; after
//! [`decide::PROVE_WINDOW`] the surface is released all the same, and counted.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use handshake_shell_window::{Control, Refusal, WindowCommand, WindowKey};
use handshake_vulkan_renderer::{PresentRefusal, PresentsProof};

use super::Core;
use crate::decide::{self, Need, Swap, Tried};
use crate::error::PresentError;
use crate::slot::{Settled, Slot};

impl Core {
    /// Settle the claims of windows that are gone. Returns whether one is still waiting.
    pub(super) fn settle_ending(&self) -> bool {
        let mut ending = self.ending.borrow_mut();
        ending.retain_mut(|slot| match slot.settle() {
            Settled::Now => {
                self.count(|report| report.claims_settled += 1);
                false
            }
            Settled::Nothing => false,
            Settled::Waiting => true,
            Settled::Refused(refusal) => {
                self.error(slot, PresentError::Settle(refusal));
                false
            }
        });
        !ending.is_empty()
    }

    /// One step of closing a window: prove its presents ended and release its surface, settle its claim, and destroy
    /// it once its target is let go. Returns whether more is to come.
    pub(super) fn close_step(&self, control: &Control, slot: &mut Slot) -> bool {
        if self.prove_step(slot) {
            return true;
        }
        let mut more = false;
        match slot.settle() {
            Settled::Now => self.count(|report| report.claims_settled += 1),
            Settled::Nothing => {}
            Settled::Waiting => more = true,
            Settled::Refused(refusal) => self.error(slot, PresentError::Settle(refusal)),
        }
        if !slot.destroy_asked {
            let tokens = control
                .window_state(slot.key)
                .map_or(0, |state| state.native_tokens);
            if tokens == 0
                && control
                    .command(&self.source(), slot.key, WindowCommand::Destroy)
                    .is_ok()
            {
                slot.destroy_asked = true;
            } else {
                more = true;
            }
        }
        more
    }

    pub(crate) fn close_window(&self, control: &Control, key: WindowKey) -> Result<(), Refusal> {
        let Some(slot) = self.slot(key) else {
            return control.command(&self.source(), key, WindowCommand::Destroy);
        };
        let mut slot = slot.borrow_mut();
        if slot.closing {
            return Ok(());
        }
        slot.closing = true;
        self.schedule.borrow_mut().cancel(Need::Frame(key));
        if !self.present_fences && slot.has_surface() {
            // While the window is still shown: its one present is never into a hidden window.
            self.begin_proof(control, &mut slot);
        } else {
            self.release(&mut slot);
        }
        let hidden = control.command(&self.source(), key, WindowCommand::Hide);
        let _ = self.owner.poll();
        if self.close_step(control, &mut slot) {
            self.want_looks();
        }
        hidden
    }

    fn release(&self, slot: &mut Slot) {
        if slot.release_surface() {
            self.count(|report| report.surfaces_released += 1);
        }
    }

    /// Begin the proof of a closing window's presents (module documentation): a new swapchain when the current one
    /// presented, then the first step, which presents on it.
    fn begin_proof(&self, control: &Control, slot: &mut Slot) {
        slot.proving = Some(Instant::now() + decide::PROVE_WINDOW);
        if slot.presented_on_current() {
            let inner = control
                .window_state(slot.key)
                .map(|state| state.inner)
                .unwrap_or_default();
            // Without a size (minimised) or refused, there is nothing to present with.
            if self.configure(slot, inner).is_err() || slot.swap != Swap::Ready {
                self.end_proof(slot, false);
                return;
            }
        }
        self.prove_step(slot);
    }

    /// One step of the proof of a closing window's presents. Returns whether more is to come; releases the surface once
    /// the owner says that they are proven, or when it refuses, or when [`decide::PROVE_WINDOW`] is over.
    fn prove_step(&self, slot: &mut Slot) -> bool {
        let Some(until) = slot.proving else {
            return false;
        };
        let proven = match slot.prove() {
            Ok(PresentsProof::Proven) => Some(true),
            Ok(PresentsProof::Waiting) => None,
            Ok(PresentsProof::NeedsPresent) => match slot.draw(&self.config.frame) {
                Tried::Shown(_) => {
                    self.count(|report| report.retire_frames += 1);
                    None
                }
                Tried::Refused(PresentRefusal::NotReady) => None,
                Tried::Refused(_) | Tried::NotClearable => Some(false),
            },
            Err(_) => Some(false),
        };
        match proven {
            Some(proven) => {
                self.end_proof(slot, proven);
                false
            }
            None if Instant::now() >= until => {
                self.end_proof(slot, false);
                false
            }
            None => true,
        }
    }

    fn end_proof(&self, slot: &mut Slot, proven: bool) {
        if !proven {
            self.count(|report| report.closes_unproven += 1);
        }
        self.release(slot);
    }

    /// After the application heard `Event::ShutdownBegan`: release every surface, settle the claims, close the owner
    /// and end the loop, now or once the owner let go of the last window target.
    pub(super) fn tear_down(&self, control: &Control) {
        if self.torn_down.replace(true) {
            return;
        }
        let slots = self.all_slots();
        for slot in &slots {
            self.release(&mut slot.borrow_mut());
        }
        let _ = self.owner.poll();
        self.settle_all(&slots);
        self.settle_ending();
        // Bounded: the owner waits at most its own deadline for the queue (MT-203 `close`).
        let shutdown = self.owner.close();
        self.settle_all(&slots);
        self.settle_ending();
        let ending = std::mem::take(&mut *self.ending.borrow_mut());
        let abandoned = slots
            .iter()
            .filter(|slot| slot.borrow_mut().abandon_claim())
            .count()
            + ending.len();
        drop(ending);
        self.count(|report| report.claims_abandoned += abandoned as u64);
        self.count(|report| report.shutdown = Some(shutdown));
        // The clients go with the slots, before the loop ends.
        self.slots.borrow_mut().clear();
        drop(slots);
        self.schedule.borrow_mut().cancel(Need::Poll);
        self.try_finish(control);
    }

    pub(super) fn settle_all(&self, slots: &[Rc<RefCell<Slot>>]) {
        for slot in slots {
            let mut slot = slot.borrow_mut();
            match slot.settle() {
                Settled::Now => self.count(|report| report.claims_settled += 1),
                Settled::Refused(refusal) => self.error(&mut slot, PresentError::Settle(refusal)),
                Settled::Nothing | Settled::Waiting => {}
            }
        }
    }

    pub(super) fn try_finish(&self, control: &Control) {
        let code = self.exit_code.get().unwrap_or(0);
        match control.finish(code) {
            Ok(()) => {}
            Err(Refusal::NativeTokensHeld) => {
                self.count(|report| report.finish_refused += 1);
                // A device that was held or still ran work at the close is destroyed by a later look, with the
                // targets of its surfaces.
                let _ = self.owner.poll();
                let tries = self.finish_tries.get().saturating_add(1);
                self.finish_tries.set(tries);
                let delay = decide::poll_delay(tries).max(Duration::from_millis(10));
                self.schedule
                    .borrow_mut()
                    .arm(Need::Finish, Instant::now() + delay);
            }
            // Shutdown not begun, or the loop has ended: nothing to try again.
            Err(_) => {}
        }
    }
}
