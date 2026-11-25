//! Settlement and the release of retained memory: each reservation ends exactly once, and only on evidence.

use super::{bump, Claim, ClaimLedger, ClaimState, Located, State, TOMB_CLOSED};
use crate::outcome::SettleRefusal;
use crate::records::{ByteQuantity, ClaimDisposition, ReleaseReceipt, SettleReceipt, Settlement};
use std::sync::atomic::Ordering;

impl ClaimLedger {
    fn settle_locked(
        &self,
        state: &mut State,
        claim: &Claim,
        settlement: Settlement,
    ) -> Result<SettleReceipt, SettleRefusal> {
        match self.locate(state, claim) {
            Located::Foreign => return Err(SettleRefusal::StaleClaim),
            Located::Closed => return Err(SettleRefusal::DoubleRelease),
            Located::Superseded => return Err(SettleRefusal::LateRelease),
            Located::Live => {}
        }
        let id = claim.info.id.0;
        let Some(record) = state.live.get(&id) else {
            return Err(SettleRefusal::StaleClaim);
        };
        if record.state == ClaimState::Retained {
            return Err(SettleRefusal::AlreadySettled);
        }
        // The owner's own count wins over what the client reports: a submission the owner has not seen
        // finish keeps the claim unsettled (GUI-DESKTOP-033).
        let in_flight = record.in_flight.max(settlement.in_flight);
        if in_flight > 0 {
            return Err(SettleRefusal::InFlight { in_flight });
        }
        // Closing says nothing is left on the device; objects made from a lease may be. A retaining
        // settlement keeps its leases until the retained memory is released.
        let leases = record.leases();
        if leases > 0 && settlement.retained == ByteQuantity::Measured(0) {
            return Err(SettleRefusal::LeaseOutstanding { leases });
        }
        let info = record.info;
        let reserved_bytes = record.charged_bytes;
        let held_bytes = record.held_bytes();
        let keep_bytes = match settlement.retained {
            ByteQuantity::Measured(retained_bytes) if retained_bytes > reserved_bytes => {
                return Err(SettleRefusal::RetainedExceedsReservation {
                    reserved_bytes,
                    retained_bytes,
                })
            }
            // The claim's own charges say it still holds more than it reports here.
            ByteQuantity::Measured(retained_bytes) if retained_bytes < held_bytes => {
                return Err(SettleRefusal::RetainedBelowCharged {
                    charged_bytes: held_bytes,
                    retained_bytes,
                })
            }
            ByteQuantity::Measured(retained_bytes) => retained_bytes,
            // Not measured: the whole reservation stays charged. Unknown never becomes zero.
            ByteQuantity::Estimated(_) | ByteQuantity::Unknown => reserved_bytes,
        };
        let released_bytes = reserved_bytes.saturating_sub(keep_bytes);
        let disposition = if settlement.retained == ByteQuantity::Measured(0) {
            if let Some(record) = state.live.remove(&id) {
                record.tomb.store(TOMB_CLOSED, Ordering::Release);
            }
            ClaimDisposition::Closed
        } else {
            if let Some(record) = state.live.get_mut(&id) {
                record.state = ClaimState::Retained;
                record.charged_bytes = keep_bytes;
                record.committed_bytes = 0;
                record.staging_bytes = 0;
            }
            ClaimDisposition::Retained {
                charged_bytes: keep_bytes,
            }
        };
        state.charged_bytes = state.charged_bytes.saturating_sub(released_bytes);
        Ok(SettleReceipt {
            claim: info,
            settlement,
            disposition,
            released_bytes,
        })
    }

    /// Settle the work of `claim` exactly once. Refused while anything is in flight, and as closing while a
    /// device lease is out.
    pub fn settle(
        &self,
        claim: &Claim,
        settlement: Settlement,
    ) -> Result<SettleReceipt, SettleRefusal> {
        let mut state = self.lock();
        let result = self.settle_locked(&mut state, claim, settlement);
        match &result {
            Ok(receipt) => {
                bump(&mut state.counters.settled);
                if matches!(receipt.disposition, ClaimDisposition::Retained { .. }) {
                    bump(&mut state.counters.settled_retaining);
                }
            }
            Err(refusal) => count_refusal(&mut state, *refusal),
        }
        result
    }

    /// Release the retained memory of a settled claim exactly once. Refused before settlement and while a
    /// device lease is out.
    pub fn release_retained(&self, claim: &Claim) -> Result<ReleaseReceipt, SettleRefusal> {
        let mut state = self.lock();
        let result = self.release_locked(&mut state, claim);
        match &result {
            Ok(_) => bump(&mut state.counters.retained_released),
            Err(refusal) => count_refusal(&mut state, *refusal),
        }
        result
    }

    fn release_locked(
        &self,
        state: &mut State,
        claim: &Claim,
    ) -> Result<ReleaseReceipt, SettleRefusal> {
        match self.locate(state, claim) {
            Located::Foreign => return Err(SettleRefusal::StaleClaim),
            Located::Closed => return Err(SettleRefusal::DoubleRelease),
            Located::Superseded => return Err(SettleRefusal::LateRelease),
            Located::Live => {}
        }
        let id = claim.info.id.0;
        match state.live.get(&id) {
            None => return Err(SettleRefusal::StaleClaim),
            Some(record) if record.state == ClaimState::Open => {
                return Err(SettleRefusal::NotSettled)
            }
            Some(record) if record.leases() > 0 => {
                return Err(SettleRefusal::LeaseOutstanding {
                    leases: record.leases(),
                })
            }
            Some(_) => {}
        }
        let Some(record) = state.live.remove(&id) else {
            return Err(SettleRefusal::StaleClaim);
        };
        record.tomb.store(TOMB_CLOSED, Ordering::Release);
        state.charged_bytes = state.charged_bytes.saturating_sub(record.charged_bytes);
        Ok(ReleaseReceipt {
            claim: record.info,
            released_bytes: record.charged_bytes,
        })
    }
}

fn count_refusal(state: &mut State, refusal: SettleRefusal) {
    bump(&mut state.counters.settle_refused);
    if refusal == SettleRefusal::DoubleRelease {
        bump(&mut state.counters.double_release);
    }
}
