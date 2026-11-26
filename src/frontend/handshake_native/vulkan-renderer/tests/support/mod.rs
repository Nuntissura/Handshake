//! Shared fixtures. Nothing here opens a window or leaves a file. `present_cases` holds the cases a window host runs
//! with its own windows; here they are only compiled.
#![allow(dead_code, unused_imports, unused_macros)]

pub mod gpu_env;
pub mod gpu_work;
pub mod present_cases;

use handshake_vulkan_renderer::{
    AuthorizationProvenance, Claim, ClaimInfo, ClaimLedger, ClaimRequest, ClientId, DeviceKey,
    LedgerClient, ModuleId, OperationId, OperationKind, OwnerDiagnostics, OwnerEvent,
    PolicyRevision, QuotaPolicy, Subject,
};
use std::sync::{Arc, Mutex, PoisonError};

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

pub fn request(reserve_bytes: u64) -> ClaimRequest {
    ClaimRequest {
        operation: OperationKind::Render,
        operation_id: OperationId(77),
        reserve_bytes,
        provenance: AuthorizationProvenance([9; 32]),
    }
}

/// The subject of events about one client and no claim.
pub fn client_subject(module: u32, client: u64) -> Subject {
    Subject {
        module: ModuleId(module),
        client: ClientId(client),
        claim: None,
        operation_id: None,
        generation: 0,
    }
}

/// The subject of events about a claim, by the claim's own client.
pub fn claim_subject(info: ClaimInfo) -> Subject {
    Subject {
        module: info.module,
        client: info.client,
        claim: Some(info.id),
        operation_id: Some(info.operation_id),
        generation: info.generation,
    }
}

/// A ledger with one bound generation, and the handle of module 1, client 1 on it.
pub struct Bench {
    pub ledger: ClaimLedger,
    pub client: LedgerClient,
    pub log: Arc<EventLog>,
}

pub fn bench(policy: QuotaPolicy) -> Bench {
    let log = Arc::new(EventLog::default());
    let ledger = ClaimLedger::new(policy, log.clone());
    assert_eq!(ledger.bind_device(KEY), 1);
    let client = attach(&ledger, 1, 1);
    Bench {
        ledger,
        client,
        log,
    }
}

impl Bench {
    pub fn claim(&self, reserve_bytes: u64) -> Claim {
        self.client
            .admit(&request(reserve_bytes))
            .expect("fits the test policy")
    }
}

pub fn attach(ledger: &ClaimLedger, module: u32, client: u64) -> LedgerClient {
    ledger
        .attach(ModuleId(module), ClientId(client))
        .expect("not attached yet")
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

    /// How many events with the code of `event` there are, whatever their numbers.
    pub fn count_like(&self, event: OwnerEvent) -> usize {
        self.events()
            .iter()
            .filter(|seen| seen.code() == event.code())
            .count()
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
