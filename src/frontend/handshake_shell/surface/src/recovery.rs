//! How the presenter recovers from a lost device (MT-214 AC214-9), decided without a window or a device.
//!
//! The GPU owner reports a loss ([`handshake_vulkan_renderer::DeviceFault::Lost`], or a claim it refuses as lost); the
//! host then replaces the generation ([`handshake_vulkan_renderer::GpuOwner::rebind_device`]), every client moves its
//! claim ([`handshake_vulkan_renderer::GpuClient::rebind_claim`]) and each window makes a surface and a swapchain on the
//! new generation and draws again. This module holds only the bookkeeping: which report starts a recovery and which is
//! the same loss again or an old one, how often the presenter looks at the owner while a refusal passes, and when it
//! gives up, so that no loss, however often repeated, makes the presenter loop without end. No other renderer is tried
//! when it gives up: the program ends with a typed outcome.

use std::time::Duration;

use crate::decide::poll_delay;

/// Losses in a row, with no frame presented between them, before the presenter gives up.
pub(crate) const MAX_LOSSES_IN_A_ROW: u32 = 3;

/// Looks at the owner, in one recovery, while the owner or a claim still refuses to move (about 10 s).
pub(crate) const MAX_REBIND_LOOKS: u32 = 24;

/// Why the presenter stopped trying to recover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GaveUp {
    /// The device was lost again and again, with no frame presented in between.
    RepeatedLoss,
    /// The owner or a claim kept refusing to move, or refused for good; the owner's own key is inside.
    Rebind(&'static str),
}

impl GaveUp {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::RepeatedLoss => "device.repeated_loss",
            Self::Rebind(key) => key,
        }
    }
}

/// What the presenter tells the application about the device. Delivered on the loop's thread, after the event or wake
/// that caused it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceEvent {
    /// The owner reported the device lost. Every window's surface is released; the generation, when the report named
    /// one. Work that cannot survive ended: a pending capture ends with `ReadbackError::Ended`.
    Lost {
        /// The lost generation, when the report named it.
        generation: Option<u64>,
    },
    /// The owner replaced the device and the windows were given surfaces on it; each draws again.
    Rebound {
        /// The new generation.
        generation: u64,
        /// Windows that were given a surface on it.
        windows: u64,
    },
    /// The presenter will not try again: no device is available. No other renderer is tried.
    GaveUp(GaveUp),
}

/// Where the device stands, for the presenter's reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceState {
    /// No loss is being recovered.
    Healthy,
    /// A loss is being recovered.
    Recovering,
    /// Recovery ended without a device.
    Failed(GaveUp),
}

/// What a report of a loss means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Loss {
    /// A recovery begins: release every surface.
    Begin,
    /// The same loss, already being recovered (or recovery ended).
    Already,
    /// A report about a generation older than the one the device was already replaced with.
    Stale,
    /// Too many losses in a row: recovery ends here.
    GaveUp(GaveUp),
}

/// What to do after the owner or a claim refused to move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Next {
    /// Look again after this long.
    Retry(Duration),
    /// Stop.
    GaveUp(GaveUp),
}

/// The presenter's recovery bookkeeping.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Recovery {
    state: DeviceState,
    in_a_row: u32,
    looks: u32,
    /// The owner replaced the device in the recovery under way.
    replaced: bool,
    rebound_generation: Option<u64>,
}

impl Default for Recovery {
    fn default() -> Self {
        Self::new()
    }
}

impl Recovery {
    pub(crate) const fn new() -> Self {
        Self {
            state: DeviceState::Healthy,
            in_a_row: 0,
            looks: 0,
            replaced: false,
            rebound_generation: None,
        }
    }

    pub(crate) fn state(&self) -> DeviceState {
        self.state
    }

    /// A report that the device is lost: `generation` when the report named it.
    pub(crate) fn lost(&mut self, generation: Option<u64>) -> Loss {
        if matches!(self.state, DeviceState::Failed(_)) {
            return Loss::Already;
        }
        if self.state == DeviceState::Recovering && !self.replaced {
            return Loss::Already;
        }
        // Healthy, or recovering with the device replaced: a loss of the generation we moved to is a new loss; one of
        // an older generation is a late report of the one already recovered.
        if let (Some(reported), Some(moved_to)) = (generation, self.rebound_generation) {
            if reported < moved_to {
                return Loss::Stale;
            }
        }
        self.in_a_row = self.in_a_row.saturating_add(1);
        if self.in_a_row > MAX_LOSSES_IN_A_ROW {
            let gave_up = GaveUp::RepeatedLoss;
            self.state = DeviceState::Failed(gave_up);
            return Loss::GaveUp(gave_up);
        }
        self.state = DeviceState::Recovering;
        self.replaced = false;
        self.looks = 0;
        Loss::Begin
    }

    /// The owner or a claim refused to move, for the reason `key`, and may later: look again, a bounded number of times.
    pub(crate) fn refused(&mut self, key: &'static str) -> Next {
        let look = self.looks;
        self.looks = self.looks.saturating_add(1);
        if self.looks > MAX_REBIND_LOOKS {
            return self.give_up(GaveUp::Rebind(key));
        }
        Next::Retry(poll_delay(look))
    }

    /// The owner or a claim refused for good, or too often: recovery ends.
    pub(crate) fn give_up(&mut self, why: GaveUp) -> Next {
        self.state = DeviceState::Failed(why);
        Next::GaveUp(why)
    }

    /// The owner replaced the device with `generation`.
    pub(crate) fn device_replaced(&mut self, generation: u64) {
        self.replaced = true;
        self.rebound_generation = Some(generation);
    }

    /// Every window that could be given a surface has one.
    pub(crate) fn recovered(&mut self) {
        if self.state == DeviceState::Recovering {
            self.state = DeviceState::Healthy;
        }
        self.looks = 0;
    }

    /// A frame was presented: the losses in a row start again from none.
    pub(crate) fn presented(&mut self) {
        if self.state == DeviceState::Healthy {
            self.in_a_row = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loss_begins_one_recovery_and_its_repeats_are_the_same_loss() {
        let mut recovery = Recovery::new();
        assert_eq!(recovery.lost(Some(1)), Loss::Begin);
        assert_eq!(recovery.state(), DeviceState::Recovering);
        // Every window's frame reports the same loss: one recovery, not one per window.
        assert_eq!(recovery.lost(Some(1)), Loss::Already);
        assert_eq!(recovery.lost(None), Loss::Already);
    }

    #[test]
    fn a_late_report_of_an_old_generation_after_the_rebind_starts_nothing() {
        let mut recovery = Recovery::new();
        assert_eq!(recovery.lost(Some(1)), Loss::Begin);
        recovery.device_replaced(2);
        recovery.recovered();
        assert_eq!(recovery.state(), DeviceState::Healthy);
        assert_eq!(recovery.lost(Some(1)), Loss::Stale, "the old generation is already recovered");
        assert_eq!(recovery.state(), DeviceState::Healthy);
        assert_eq!(recovery.lost(Some(2)), Loss::Begin, "the new generation was lost");
    }

    #[test]
    fn a_loss_of_the_new_generation_while_moving_claims_is_a_new_loss() {
        let mut recovery = Recovery::new();
        assert_eq!(recovery.lost(Some(1)), Loss::Begin);
        recovery.device_replaced(2);
        assert_eq!(recovery.lost(Some(2)), Loss::Begin);
        assert_eq!(recovery.state(), DeviceState::Recovering);
        assert_eq!(recovery.lost(Some(2)), Loss::Already);
    }

    #[test]
    fn repeated_loss_is_bounded_and_ends_without_a_device() {
        let mut recovery = Recovery::new();
        let mut generation = 1;
        for _ in 0..MAX_LOSSES_IN_A_ROW {
            assert_eq!(recovery.lost(Some(generation)), Loss::Begin);
            generation += 1;
            recovery.device_replaced(generation);
            recovery.recovered();
        }
        assert_eq!(
            recovery.lost(Some(generation)),
            Loss::GaveUp(GaveUp::RepeatedLoss),
            "the loss after the bound is the last"
        );
        assert_eq!(recovery.state(), DeviceState::Failed(GaveUp::RepeatedLoss));
        assert_eq!(recovery.lost(Some(generation + 1)), Loss::Already, "nothing starts again");
    }

    #[test]
    fn a_presented_frame_between_losses_starts_the_count_again() {
        let mut recovery = Recovery::new();
        let mut generation = 1;
        for _ in 0..(MAX_LOSSES_IN_A_ROW * 4) {
            assert_eq!(recovery.lost(Some(generation)), Loss::Begin);
            generation += 1;
            recovery.device_replaced(generation);
            recovery.recovered();
            recovery.presented();
        }
        assert_eq!(recovery.state(), DeviceState::Healthy);
    }

    #[test]
    fn a_frame_presented_during_recovery_does_not_hide_a_loss_in_a_row() {
        let mut recovery = Recovery::new();
        assert_eq!(recovery.lost(Some(1)), Loss::Begin);
        recovery.presented();
        recovery.device_replaced(2);
        assert_eq!(recovery.lost(Some(2)), Loss::Begin);
        assert_eq!(recovery.in_a_row, 2);
    }

    #[test]
    fn refusals_are_retried_with_growing_delays_and_then_end_with_the_owners_key() {
        let mut recovery = Recovery::new();
        assert_eq!(recovery.lost(Some(1)), Loss::Begin);
        let mut previous = Duration::ZERO;
        for _ in 0..MAX_REBIND_LOOKS {
            match recovery.refused("device_not_quiescent") {
                Next::Retry(delay) => {
                    assert!(delay >= previous, "{delay:?} after {previous:?}");
                    previous = delay;
                }
                Next::GaveUp(why) => panic!("gave up too early: {why:?}"),
            }
        }
        assert_eq!(
            recovery.refused("device_not_quiescent"),
            Next::GaveUp(GaveUp::Rebind("device_not_quiescent"))
        );
        assert_eq!(
            recovery.state(),
            DeviceState::Failed(GaveUp::Rebind("device_not_quiescent"))
        );
        assert_eq!(GaveUp::Rebind("device_not_quiescent").code(), "device_not_quiescent");
        assert_eq!(GaveUp::RepeatedLoss.code(), "device.repeated_loss");
    }

    #[test]
    fn the_looks_of_one_recovery_do_not_count_against_the_next() {
        let mut recovery = Recovery::new();
        assert_eq!(recovery.lost(Some(1)), Loss::Begin);
        for _ in 0..MAX_REBIND_LOOKS {
            assert!(matches!(recovery.refused("device_held"), Next::Retry(_)));
        }
        recovery.device_replaced(2);
        recovery.recovered();
        recovery.presented();
        assert_eq!(recovery.lost(Some(2)), Loss::Begin);
        assert!(matches!(recovery.refused("device_held"), Next::Retry(_)));
    }

    #[test]
    fn a_refusal_for_good_ends_recovery_at_once() {
        let mut recovery = Recovery::new();
        assert_eq!(recovery.lost(None), Loss::Begin);
        assert_eq!(
            recovery.give_up(GaveUp::Rebind("owner_closed")),
            Next::GaveUp(GaveUp::Rebind("owner_closed"))
        );
        assert_eq!(recovery.lost(None), Loss::Already);
    }
}
