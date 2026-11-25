//! Moving a claim to the successor generation after a device loss. The charge moves in one step: it is
//! neither refunded by the loss nor counted twice (GPU-QUOTA-013).

use super::{
    bump, Claim, ClaimLedger, ClaimState, Located, Record, State, TOMB_LIVE, TOMB_SUPERSEDED,
};
use crate::ids::ClaimId;
use crate::outcome::RebindRefusal;
use crate::records::ClaimInfo;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

impl ClaimLedger {
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
        if record.in_flight > 0 {
            return Err(RebindRefusal::InFlight {
                in_flight: record.in_flight,
            });
        }
        let leases = record.leases();
        if leases > 0 {
            return Err(RebindRefusal::LeaseOutstanding { leases });
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
        if record.info.generation == device.generation {
            return Err(RebindRefusal::NotLost);
        }
        if record.info.device != device.key {
            return Err(RebindRefusal::DeviceChanged);
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
        let tomb = Arc::new(AtomicU8::new(TOMB_LIVE));
        // The charge moves with the claim: the quota total does not change. Nothing is committed yet: the
        // owner destroyed the lost device, and its memory with it, before it bound this generation.
        let successor = Record::open(info, old.charged_bytes, old.max_in_flight, &tomb);
        state.live.insert(new_id, successor);
        Ok(Claim {
            info,
            ledger_key: self.key,
            tomb,
        })
    }

    /// After a device loss: close `claim` as superseded and return its successor on the current
    /// generation, with the same attribution and the same charge. A later settle of the old claim is
    /// `LateRelease`. A refused rebind keeps the old claim open.
    pub fn rebind(&self, claim: &Claim) -> Result<Claim, RebindRefusal> {
        let mut state = self.lock();
        let result = self.rebind_locked(&mut state, claim);
        if result.is_ok() {
            bump(&mut state.counters.rebound);
        } else {
            bump(&mut state.counters.rebind_refused);
        }
        result
    }
}
