//! What becomes of a claim whose handle is gone.
//!
//! A dropped handle proves nothing, so it releases nothing (GPU-QUOTA-013). The owner closes such a claim itself, and
//! only when nothing under it can still use the device: every submission was observed finished, every lease was
//! returned, and the memory it reported is gone, because the client reported it freed or because the device was
//! destroyed. An abandoned claim that does not get there stays charged and is reported as leaked.

use super::{bump, ClaimLedger, ClaimState, Shared, State, TOMB_CLOSED};
use crate::diag::{OwnerEvent, Subject};
use std::sync::atomic::Ordering;

impl Shared {
    /// The holder of claim `id` dropped it without settling it.
    pub(super) fn abandon(&self, id: u64) {
        let mut events = Vec::new();
        {
            let mut guard = self.lock();
            let state = &mut *guard;
            let Some(record) = state.live.get_mut(&id) else {
                return;
            };
            record.abandoned = true;
            events.push(OwnerEvent::ClaimAbandoned {
                subject: Subject::claim(&record.info),
                in_flight: u64::from(record.in_flight),
                charged_bytes: record.charged_bytes,
            });
            bump(&mut state.counters.abandoned);
            reclaim(state, id, &mut events);
        }
        self.emit_all(events);
    }

    /// A hold on the device under claim `id` was dropped.
    pub(super) fn release_hold(&self, id: u64) {
        let mut events = Vec::new();
        {
            let mut guard = self.lock();
            let state = &mut *guard;
            let Some(record) = state.live.get_mut(&id) else {
                return;
            };
            record.leases = record.leases.saturating_sub(1);
            reclaim(state, id, &mut events);
        }
        self.emit_all(events);
    }
}

/// Close claim `id` when it is abandoned and nothing under it can still use the device. Reports it as leaked, once,
/// when only the memory it reported stands in the way.
pub(super) fn reclaim(state: &mut State, id: u64, events: &mut Vec<OwnerEvent>) {
    let Some(record) = state.live.get_mut(&id) else {
        return;
    };
    if !record.abandoned || record.in_flight > 0 || record.leases > 0 || record.owned > 0 {
        return;
    }
    // A settlement that retained memory said it is there; how much may be unknown, and unknown is never zero.
    let holds_memory = match record.state {
        ClaimState::Open => record.held_bytes() > 0,
        ClaimState::Retained => true,
    };
    if holds_memory && !record.device_gone {
        report_leak(state, id, events);
        return;
    }
    let Some(record) = state.live.remove(&id) else {
        return;
    };
    record.tomb.store(TOMB_CLOSED, Ordering::Release);
    state.charged_bytes = state.charged_bytes.saturating_sub(record.charged_bytes);
    bump(&mut state.counters.reclaimed);
    events.push(OwnerEvent::ClaimReclaimed {
        subject: Subject::claim(&record.info),
        released_bytes: record.charged_bytes,
    });
}

fn report_leak(state: &mut State, id: u64, events: &mut Vec<OwnerEvent>) {
    let Some(record) = state.live.get_mut(&id) else {
        return;
    };
    if record.leak_reported {
        return;
    }
    record.leak_reported = true;
    events.push(OwnerEvent::ClaimLeaked {
        subject: Subject::claim(&record.info),
        leaked_bytes: record.charged_bytes,
    });
    bump(&mut state.counters.leaked);
}

impl ClaimLedger {
    /// The owner will observe nothing more by itself: report every abandoned claim it still holds as leaked. Claims
    /// whose device is destroyed later are reclaimed then.
    pub fn report_abandoned(&self) {
        let mut events = Vec::new();
        {
            let mut guard = self.shared.lock();
            let state = &mut *guard;
            let ids: Vec<u64> = state
                .live
                .iter()
                .filter(|(_, record)| record.abandoned)
                .map(|(id, _)| *id)
                .collect();
            for id in ids {
                report_leak(state, id, &mut events);
            }
        }
        self.shared.emit_all(events);
    }
}
