//! Shared fixtures. Nothing here opens a window or writes a file.
#![allow(dead_code)]

use handshake_vulkan_renderer::{
    AuthorizationProvenance, Claim, ClaimLedger, ClaimRequest, ClientId, DeviceKey, ModuleId,
    OperationId, OperationKind, OwnerDiagnostics, OwnerEvent, PolicyRevision, QuotaPolicy,
};
use std::sync::{Mutex, PoisonError};

pub const KEY: DeviceKey = DeviceKey {
    vendor_id: 1,
    device_id: 2,
    device_uuid: [3; 16],
};
pub const OTHER_KEY: DeviceKey = DeviceKey {
    vendor_id: 1,
    device_id: 2,
    device_uuid: [4; 16],
};

/// A policy with `limit_bytes` and room for 8 claims with 4 submissions each.
pub fn policy(limit_bytes: u64) -> QuotaPolicy {
    bounded_policy(limit_bytes, 8, 4)
}

pub fn bounded_policy(limit_bytes: u64, max_claims: u32, max_in_flight: u32) -> QuotaPolicy {
    QuotaPolicy::Configured {
        revision: PolicyRevision(7),
        limit_bytes,
        max_claims,
        max_in_flight_per_claim: max_in_flight,
    }
}

pub fn request(module: u32, client: u64, reserve_bytes: u64) -> ClaimRequest {
    ClaimRequest {
        module: ModuleId(module),
        client: ClientId(client),
        operation: OperationKind::Render,
        operation_id: OperationId(u128::from(client)),
        reserve_bytes,
        provenance: AuthorizationProvenance([0; 32]),
    }
}

/// A ledger with one bound generation.
pub fn ledger(policy: QuotaPolicy) -> ClaimLedger {
    let ledger = ClaimLedger::new(policy);
    assert_eq!(ledger.bind_device(KEY), 1);
    ledger
}

pub fn claim(ledger: &ClaimLedger, reserve_bytes: u64) -> Claim {
    ledger
        .admit(&request(1, 1, reserve_bytes))
        .expect("fits the test policy")
}

/// Keeps every event, in order.
#[derive(Default)]
pub struct EventLog {
    events: Mutex<Vec<OwnerEvent>>,
}

impl EventLog {
    pub fn events(&self) -> Vec<OwnerEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn count(&self, event: OwnerEvent) -> usize {
        self.events().iter().filter(|seen| **seen == event).count()
    }
}

impl OwnerDiagnostics for EventLog {
    fn record(&self, event: OwnerEvent) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}
