//! Settlement and the release of retained memory: each reservation ends exactly once, and only on evidence.

use super::{bump, Claim, ClaimState, LedgerClient, Located, Record, State, TOMB_CLOSED};
use crate::diag::{OwnerEvent, Subject};
use crate::outcome::SettleRefusal;
use crate::records::{
    ByteQuantity, ClaimDisposition, ReleaseReceipt, SettleOutcome, SettleReceipt, Settlement,
    WorkObserved,
};
use std::sync::atomic::Ordering;

/// Whether `outcome` says something else than the owner observed of the work under `record`.
///
/// `Canceled`, `Failed` and `Abandoned` say how the client's operation ended, which the owner does not observe; the
/// receipt carries what it did observe.
fn contradicts(record: &Record, outcome: SettleOutcome) -> bool {
    let work = record.work;
    let total = record.prior.plus(work);
    match outcome {
        SettleOutcome::Completed => {
            // Memory work with no submission at all may complete: it took a lease or charged memory, and its
            // device is still there.
            let without_submission = total == WorkObserved::default()
                && record.touched
                && !record.device_lost
                && !record.rebound;
            work.lost > 0
                || (record.prior.lost > 0 && work.completed == 0)
                || (total.completed == 0 && !without_submission)
        }
        SettleOutcome::NoEffect => total.completed > 0 || total.lost > 0,
        SettleOutcome::Lost => total.lost == 0 && !record.device_lost && !record.rebound,
        SettleOutcome::Canceled | SettleOutcome::Failed | SettleOutcome::Abandoned => false,
    }
}

impl LedgerClient {
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
        // So does its own observation: no receipt says something else than what happened on the device
        // (GPU-QUOTA-014).
        let observed = record.prior.plus(record.work);
        if contradicts(record, settlement.outcome) {
            return Err(SettleRefusal::OutcomeContradicted { observed });
        }
        // Closing says nothing is left on the device; objects made from a lease may be. A retaining
        // settlement keeps its leases until the retained memory is released.
        let leases = record.leases;
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
            observed,
            disposition,
            released_bytes,
        })
    }

    /// Settle the work of `claim` exactly once. Refused while anything is in flight, when the outcome contradicts
    /// what the owner observed, and as closing while a device lease is out.
    pub fn settle(
        &self,
        claim: &Claim,
        settlement: Settlement,
    ) -> Result<SettleReceipt, SettleRefusal> {
        let result = {
            let mut guard = self.shared.lock();
            let state = &mut *guard;
            let result = self.settle_locked(state, claim, settlement);
            match &result {
                Ok(receipt) => {
                    bump(&mut state.counters.settled);
                    if matches!(receipt.disposition, ClaimDisposition::Retained { .. }) {
                        bump(&mut state.counters.settled_retaining);
                    }
                }
                Err(refusal) => count_refusal(state, *refusal),
            }
            result
        };
        let event = match &result {
            Ok(receipt) => OwnerEvent::Settled {
                subject: Subject::claim(&receipt.claim),
                outcome: receipt.settlement.outcome.number(),
                released_bytes: receipt.released_bytes,
                retained_bytes: match receipt.disposition {
                    ClaimDisposition::Closed => 0,
                    ClaimDisposition::Retained { charged_bytes } => charged_bytes,
                },
            },
            Err(refusal) => self.settle_refused(claim, *refusal),
        };
        self.shared.emit(event);
        result
    }

    fn settle_refused(&self, claim: &Claim, refusal: SettleRefusal) -> OwnerEvent {
        OwnerEvent::SettleRefused {
            subject: self.subject(&claim.info),
            reason: refusal.number(),
        }
    }

    /// Release the retained memory of a settled claim exactly once. Refused before settlement and while a
    /// device lease is out.
    pub fn release_retained(&self, claim: &Claim) -> Result<ReleaseReceipt, SettleRefusal> {
        let result = {
            let mut guard = self.shared.lock();
            let state = &mut *guard;
            let result = self.release_locked(state, claim);
            match &result {
                Ok(_) => bump(&mut state.counters.retained_released),
                Err(refusal) => count_refusal(state, *refusal),
            }
            result
        };
        let event = match &result {
            Ok(receipt) => OwnerEvent::RetainedReleased {
                subject: Subject::claim(&receipt.claim),
                released_bytes: receipt.released_bytes,
            },
            Err(refusal) => self.settle_refused(claim, *refusal),
        };
        self.shared.emit(event);
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
            Some(record) if record.leases > 0 => {
                return Err(SettleRefusal::LeaseOutstanding {
                    leases: record.leases,
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
