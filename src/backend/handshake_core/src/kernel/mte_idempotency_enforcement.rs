//! MT-055: Idempotency Key Enforcement (MTE layer).
//!
//! Acceptance (MT-055.json): "prevent duplicate promotion effects. Acceptance:
//! duplicate accept returns prior receipt or typed duplicate rejection without
//! second mutation."
//!
//! Idempotency layering:
//!
//! - `kb003_promotion::gate` (Batch E) enforces idempotency at the **promotion
//!   receipt insert** level by keying `(idempotency_key, payload_hash)` in
//!   `Kb003Storage::insert_promotion_receipt`.
//! - This module enforces idempotency one level **earlier**, at the **MTE
//!   sandbox→validation→promotion chain** level. It rejects a chained
//!   re-attempt for the same `(wp_id, mt_id, sandbox_run_id, idempotency_key)`
//!   before any sandbox or validation work re-runs, returning the prior chain
//!   receipt id when the payload matches and a typed duplicate rejection when
//!   the payload differs.
//!
//! The result is two-layer defence in depth: a runaway scheduler that
//! re-issues the same chain twice will be caught here without ever invoking
//! the gate; a runaway *promoter* that bypasses MTE is still caught by the
//! gate. Both layers fail closed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Composite key for an MTE chain attempt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MteChainKey {
    pub wp_id: String,
    pub mt_id: String,
    pub sandbox_run_id: String,
    pub idempotency_key: String,
}

impl MteChainKey {
    pub fn new(
        wp_id: impl Into<String>,
        mt_id: impl Into<String>,
        sandbox_run_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Self {
        Self {
            wp_id: wp_id.into(),
            mt_id: mt_id.into(),
            sandbox_run_id: sandbox_run_id.into(),
            idempotency_key: idempotency_key.into(),
        }
    }
}

/// Result of asking the enforcer whether a new chain attempt may run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteIdempotencyVerdict {
    /// First time we have seen this chain key; caller proceeds.
    FreshAttempt,
    /// Same key, same payload hash: caller must short-circuit and reuse the
    /// prior chain receipt id; no second mutation.
    DuplicateMatchingReturnPrior {
        prior_chain_receipt_id: String,
        first_attempt_at_utc: DateTime<Utc>,
    },
    /// Same key, different payload: typed duplicate rejection — caller must
    /// surface this to the operator as a hard error.
    DuplicateMismatch {
        existing_payload_hash: String,
        new_payload_hash: String,
        first_attempt_at_utc: DateTime<Utc>,
    },
}

impl MteIdempotencyVerdict {
    pub fn allows_new_mutation(&self) -> bool {
        matches!(self, Self::FreshAttempt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChainRecord {
    chain_receipt_id: String,
    payload_hash: String,
    first_attempt_at_utc: DateTime<Utc>,
}

/// In-process enforcer. Production deployments back this by Postgres
/// (mirroring the `kb003_promotion_receipts.idempotency_key` UNIQUE index),
/// but the contract and tests live here so the chain-level invariant is
/// verifiable without a database.
#[derive(Debug, Default)]
pub struct MteIdempotencyEnforcer {
    seen: HashMap<MteChainKey, ChainRecord>,
}

impl MteIdempotencyEnforcer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ask whether a chain attempt may proceed.
    ///
    /// Side-effect on `FreshAttempt`: the enforcer records the chain so the
    /// next call with the same key short-circuits per the gate's idempotency
    /// rule.
    pub fn check_and_register(
        &mut self,
        key: MteChainKey,
        payload_hash: impl Into<String>,
        new_chain_receipt_id: impl Into<String>,
    ) -> MteIdempotencyVerdict {
        let payload_hash = payload_hash.into();
        let new_chain_receipt_id = new_chain_receipt_id.into();
        if let Some(existing) = self.seen.get(&key) {
            if existing.payload_hash == payload_hash {
                return MteIdempotencyVerdict::DuplicateMatchingReturnPrior {
                    prior_chain_receipt_id: existing.chain_receipt_id.clone(),
                    first_attempt_at_utc: existing.first_attempt_at_utc,
                };
            }
            return MteIdempotencyVerdict::DuplicateMismatch {
                existing_payload_hash: existing.payload_hash.clone(),
                new_payload_hash: payload_hash,
                first_attempt_at_utc: existing.first_attempt_at_utc,
            };
        }
        self.seen.insert(
            key,
            ChainRecord {
                chain_receipt_id: new_chain_receipt_id,
                payload_hash,
                first_attempt_at_utc: Utc::now(),
            },
        );
        MteIdempotencyVerdict::FreshAttempt
    }

    /// Count of distinct chain keys tracked.
    pub fn distinct_chains(&self) -> usize {
        self.seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> MteChainKey {
        MteChainKey::new("WP-X", "MT-1", "SBX-1", "IK-1")
    }

    // MT-055 acceptance: first attempt proceeds and registers the chain.
    #[test]
    fn fresh_attempt_proceeds() {
        let mut enf = MteIdempotencyEnforcer::new();
        let v = enf.check_and_register(key(), "h-a", "CR-1");
        assert_eq!(v, MteIdempotencyVerdict::FreshAttempt);
        assert!(v.allows_new_mutation());
        assert_eq!(enf.distinct_chains(), 1);
    }

    // MT-055 acceptance: duplicate accept returns prior receipt; no second
    // mutation.
    #[test]
    fn duplicate_matching_payload_returns_prior() {
        let mut enf = MteIdempotencyEnforcer::new();
        let _ = enf.check_and_register(key(), "h-a", "CR-1");
        let v = enf.check_and_register(key(), "h-a", "CR-2");
        match v {
            MteIdempotencyVerdict::DuplicateMatchingReturnPrior {
                prior_chain_receipt_id,
                ..
            } => assert_eq!(prior_chain_receipt_id, "CR-1"),
            other => panic!("expected DuplicateMatchingReturnPrior, got {other:?}"),
        }
        // Important: distinct chain count did not increase (no second
        // registration) and second-mutation is refused.
        assert_eq!(enf.distinct_chains(), 1);
    }

    // MT-055 acceptance: duplicate key with different payload surfaces as a
    // typed duplicate rejection.
    #[test]
    fn duplicate_mismatch_payload_surfaces_typed_rejection() {
        let mut enf = MteIdempotencyEnforcer::new();
        let _ = enf.check_and_register(key(), "h-a", "CR-1");
        let v = enf.check_and_register(key(), "h-different", "CR-2");
        match v {
            MteIdempotencyVerdict::DuplicateMismatch {
                ref existing_payload_hash,
                ref new_payload_hash,
                ..
            } => {
                assert_eq!(existing_payload_hash, "h-a");
                assert_eq!(new_payload_hash, "h-different");
            }
            other => panic!("expected DuplicateMismatch, got {other:?}"),
        }
        assert!(!v.allows_new_mutation());
    }

    #[test]
    fn different_keys_do_not_collide() {
        let mut enf = MteIdempotencyEnforcer::new();
        let a = MteChainKey::new("WP-X", "MT-1", "SBX-1", "IK-A");
        let b = MteChainKey::new("WP-X", "MT-1", "SBX-1", "IK-B");
        assert!(matches!(
            enf.check_and_register(a, "h", "CR-A"),
            MteIdempotencyVerdict::FreshAttempt
        ));
        assert!(matches!(
            enf.check_and_register(b, "h", "CR-B"),
            MteIdempotencyVerdict::FreshAttempt
        ));
        assert_eq!(enf.distinct_chains(), 2);
    }

    #[test]
    fn key_components_all_matter() {
        let mut enf = MteIdempotencyEnforcer::new();
        let k1 = MteChainKey::new("WP-X", "MT-1", "SBX-1", "IK");
        let k2 = MteChainKey::new("WP-X", "MT-2", "SBX-1", "IK"); // different mt_id
        let k3 = MteChainKey::new("WP-X", "MT-1", "SBX-2", "IK"); // different sandbox_run_id
        let k4 = MteChainKey::new("WP-Y", "MT-1", "SBX-1", "IK"); // different wp_id
        for k in [k1, k2, k3, k4] {
            assert!(matches!(
                enf.check_and_register(k, "h", "CR"),
                MteIdempotencyVerdict::FreshAttempt
            ));
        }
        assert_eq!(enf.distinct_chains(), 4);
    }
}
