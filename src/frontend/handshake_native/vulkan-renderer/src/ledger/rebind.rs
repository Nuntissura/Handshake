//! Moving a claim to the successor generation after a device loss. The charge moves in one step: it is
//! neither refunded by the loss nor counted twice (GPU-QUOTA-013).

use super::{bump, Claim, ClaimState, LedgerClient, Located, Record, State, TOMB_SUPERSEDED};
use crate::diag::{OwnerEvent, Subject};
use crate::ids::ClaimId;
use crate::outcome::RebindRefusal;
use crate::records::ClaimInfo;
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl LedgerClient {
    fn rebind_locked(&self, state: &mut State, claim: &Claim) -> Result<Claim, RebindRefusal> {
        match self.locate(state, claim) {
            Located::Foreign => return Err(RebindRefusal::Unknown),
            Located::Closed | Located::Superseded => return Err(RebindRefusal::NotOpen),
            Located::Live => {}
        }
        let id = claim.info.id.0;
        let Some(record) = state.live.get(&id) else {
            return Err(RebindRefusal::Unknown);
        };
        if record.state == ClaimState::Retained || record.revoked {
            return Err(RebindRefusal::NotOpen);
        }
        // Before everything that asks the client to wait or to let go: on a healthy device there is no loss to
        // recover from.
        if state.is_running(record.info.generation) {
            return Err(RebindRefusal::NotLost);
        }
        if record.in_flight > 0 {
            return Err(RebindRefusal::InFlight {
                in_flight: record.in_flight,
            });
        }
        if record.leases > 0 {
            return Err(RebindRefusal::LeaseOutstanding {
                leases: record.leases,
            });
        }
        let device = match state.device {
            None => return Err(RebindRefusal::DeviceUnavailable),
            Some(device) if device.lost => {
                return Err(RebindRefusal::Lost {
                    generation: device.generation,
                })
            }
            Some(device) => device,
        };
        if record.info.device != device.key {
            return Err(RebindRefusal::DeviceChanged);
        }
        // The lost device may still exist, set aside for a client that holds it. What this claim reported on it is
        // not gone by the loss, and the successor starts with nothing.
        if record.held_bytes() > 0 {
            return Err(RebindRefusal::MemoryHeld {
                bytes: record.held_bytes(),
            });
        }
        let Some(old) = state.live.remove(&id) else {
            return Err(RebindRefusal::Unknown);
        };
        old.tomb.store(TOMB_SUPERSEDED, Ordering::Release);
        let new_id = state.next_claim;
        state.next_claim = new_id.saturating_add(1);
        let info = ClaimInfo {
            id: ClaimId(new_id),
            generation: device.generation,
            ..old.info
        };
        // The charge moves with the claim: the quota total does not change.
        let mut successor = Record::open(info, self.seat, old.charged_bytes, old.max_in_flight);
        successor.rebound = true;
        successor.prior = old.prior.plus(old.work);
        let tomb = Arc::clone(&successor.tomb);
        state.live.insert(new_id, successor);
        Ok(Claim {
            info,
            seat: self.seat,
            ledger: Arc::clone(&self.shared),
            tomb,
        })
    }

    /// After a device loss: close `claim` as superseded and return its successor on the current
    /// generation, with the same attribution and the same charge. A later settle of the old claim is
    /// `LateRelease`. A refused rebind keeps the old claim open.
    pub fn rebind(&self, claim: &Claim) -> Result<Claim, RebindRefusal> {
        let result = {
            let mut state = self.shared.lock();
            let result = self.rebind_locked(&mut state, claim);
            if result.is_ok() {
                bump(&mut state.counters.rebound);
            } else {
                bump(&mut state.counters.rebind_refused);
            }
            result
        };
        let event = match &result {
            Ok(successor) => OwnerEvent::ClaimRebound {
                subject: Subject::claim(&successor.info),
                superseded: claim.info.id,
            },
            Err(refusal) => OwnerEvent::ClaimRebindRefused {
                subject: self.subject(&claim.info),
                reason: refusal.number(),
            },
        };
        self.shared.emit(event);
        result
    }
}
