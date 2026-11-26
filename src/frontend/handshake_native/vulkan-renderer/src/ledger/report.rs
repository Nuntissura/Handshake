//! Read-only views of the ledger: totals, per-module attribution and counters (GUI-DESKTOP-039). They are the
//! owner's: a client handle has none of them (GPU-QUOTA-015).

use super::{ClaimLedger, ClaimState};
use crate::ids::ModuleId;
use crate::records::{LedgerCounters, LedgerUsage, ModuleUsage};
use std::collections::BTreeMap;

impl ClaimLedger {
    pub fn usage(&self) -> LedgerUsage {
        let state = self.lock();
        let mut usage = LedgerUsage {
            policy: state.policy,
            charged_bytes: state.charged_bytes,
            reserved_bytes: 0,
            retained_bytes: 0,
            committed_bytes: 0,
            staging_bytes: 0,
            open_claims: 0,
            retained_claims: 0,
            revoked_claims: 0,
            in_flight: 0,
            leases: 0,
            attached_clients: u64::try_from(state.seats.len()).unwrap_or(u64::MAX),
            abandoned_claims: 0,
            leaked_claims: 0,
            owned_resources: 0,
            owned_bytes: 0,
            device: state.device,
            admission_open: state.admission_open,
        };
        for record in state.live.values() {
            usage.leases = usage.leases.saturating_add(u64::from(record.leases));
            usage.owned_resources = usage
                .owned_resources
                .saturating_add(u64::from(record.owned));
            usage.owned_bytes = usage.owned_bytes.saturating_add(record.owned_bytes());
            usage.abandoned_claims = usage
                .abandoned_claims
                .saturating_add(u64::from(record.abandoned));
            usage.leaked_claims = usage
                .leaked_claims
                .saturating_add(u64::from(record.leak_reported));
            match record.state {
                ClaimState::Open => {
                    usage.open_claims = usage.open_claims.saturating_add(1);
                    usage.reserved_bytes =
                        usage.reserved_bytes.saturating_add(record.charged_bytes);
                    usage.committed_bytes =
                        usage.committed_bytes.saturating_add(record.committed_bytes);
                    usage.staging_bytes = usage.staging_bytes.saturating_add(record.staging_bytes);
                    if record.revoked {
                        usage.revoked_claims = usage.revoked_claims.saturating_add(1);
                    }
                    usage.in_flight = usage.in_flight.saturating_add(u64::from(record.in_flight));
                }
                ClaimState::Retained => {
                    usage.retained_claims = usage.retained_claims.saturating_add(1);
                    usage.retained_bytes =
                        usage.retained_bytes.saturating_add(record.charged_bytes);
                }
            }
        }
        usage
    }

    /// Usage per module, in module order.
    pub fn usage_by_module(&self) -> Vec<ModuleUsage> {
        let state = self.lock();
        let mut by_module: BTreeMap<ModuleId, ModuleUsage> = BTreeMap::new();
        for record in state.live.values() {
            let entry = by_module.entry(record.info.module).or_insert(ModuleUsage {
                module: record.info.module,
                charged_bytes: 0,
                open_claims: 0,
                retained_claims: 0,
            });
            entry.charged_bytes = entry.charged_bytes.saturating_add(record.charged_bytes);
            match record.state {
                ClaimState::Open => entry.open_claims = entry.open_claims.saturating_add(1),
                ClaimState::Retained => {
                    entry.retained_claims = entry.retained_claims.saturating_add(1)
                }
            }
        }
        by_module.into_values().collect()
    }

    pub fn counters(&self) -> LedgerCounters {
        self.lock().counters
    }
}
