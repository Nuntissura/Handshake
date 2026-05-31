//! MT-095 — KV prefix handle replay-resistance security tests.
//!
//! Covers the five adversarial paths in the MT-095 contract:
//!   1. tamper content_hash    -> KvCacheError
//!   2. tamper model_id        -> KvCacheError
//!   3. cross-process key      -> KvCacheError (simulated)
//!   4. cross-model            -> KvCacheError
//!   5. stale-snapshot expiry  -> KvCacheError (TTL drift)
//! Plus collateral: round-trip Ok, missing binding rejected, per-process
//! key is non-zero entropy.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use handshake_core::model_runtime::{
    KvCacheHandle, KvCacheOps, KvCacheStats, KvPrefixHandle, KvQuantSupport, ModelId,
    ModelRuntimeError,
};

const CONTENT_A: [u8; 32] = [0xAA; 32];
const CONTENT_B: [u8; 32] = [0xBB; 32];
const REGISTERED_T0: i64 = 1_700_000_000_000_000; // micros — arbitrary
const REGISTERED_T1: i64 = 1_700_000_000_000_001; // micros — 1us later

#[test]
fn kv_security_round_trip_passes_when_components_match() {
    let model_id = ModelId::new_v7();
    let handle = KvPrefixHandle::from_derived(model_id, CONTENT_A, REGISTERED_T0, 4);
    assert!(handle.derived_id().is_some());
    handle
        .verify_derived_against(model_id, &CONTENT_A, REGISTERED_T0)
        .expect("matching components must verify");
}

#[test]
fn kv_security_tampered_content_hash_is_rejected() {
    let model_id = ModelId::new_v7();
    let handle = KvPrefixHandle::from_derived(model_id, CONTENT_A, REGISTERED_T0, 4);
    let err = handle
        .verify_derived_against(model_id, &CONTENT_B, REGISTERED_T0)
        .expect_err("tampered content_hash must reject");
    assert!(
        matches!(err, ModelRuntimeError::KvCacheError(_)),
        "expected KvCacheError, got {err:?}"
    );
}

#[test]
fn kv_security_cross_model_rejection() {
    let model_a = ModelId::new_v7();
    let model_b = ModelId::new_v7();
    assert_ne!(model_a, model_b);

    let handle = KvPrefixHandle::from_derived(model_a, CONTENT_A, REGISTERED_T0, 4);
    let err = handle
        .verify_derived_against(model_b, &CONTENT_A, REGISTERED_T0)
        .expect_err("cross-model handle replay must reject");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)));
}

#[test]
fn kv_security_stale_registered_at_utc_rejection() {
    let model_id = ModelId::new_v7();
    let handle = KvPrefixHandle::from_derived(model_id, CONTENT_A, REGISTERED_T0, 4);
    // The cache stack would record `registered_at_utc=T0`; if a later
    // operator presents the same handle but the cache entry has been
    // refreshed to `T1`, verification must fail. This catches both
    // stale-snapshot replay and accidental ABA collisions on a reused
    // (model_id, content_hash) pair.
    let err = handle
        .verify_derived_against(model_id, &CONTENT_A, REGISTERED_T1)
        .expect_err("stale registered_at_utc must reject");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)));
}

#[test]
fn kv_security_cross_process_key_isolation_simulated() {
    // Real cross-process replay is detected by: process P1's
    // KV_PREFIX_KEY differs from P2's. A handle minted in P1 has
    // derived_id = blake3_keyed(P1_KEY, ...). When P2 calls
    // verify_derived_against on that handle, it re-derives under
    // P2_KEY and compares — they won't match.
    //
    // We can't fork a process inside a unit test, so we simulate by
    // taking a legitimate handle's derived_id, flipping a bit, and
    // re-injecting it. That mimics what a foreign process would
    // produce under its own key.
    let model_id = ModelId::new_v7();
    let legit = KvPrefixHandle::from_derived(model_id, CONTENT_A, REGISTERED_T0, 4);
    let mut tampered_bytes = *legit.derived_id().expect("from_derived sets derived_id");
    tampered_bytes[0] ^= 0xFF;

    // Re-serialize via JSON to inject the tampered derived_id without
    // exposing a `with_derived` mutator on the public API.
    let mut value = serde_json::to_value(&legit).expect("serialize legit handle");
    value["derived_id"] = serde_json::Value::Array(
        tampered_bytes
            .iter()
            .map(|b| serde_json::json!(*b as u64))
            .collect(),
    );
    let tampered: KvPrefixHandle =
        serde_json::from_value(value).expect("deserialize tampered handle");

    let err = tampered
        .verify_derived_against(model_id, &CONTENT_A, REGISTERED_T0)
        .expect_err("derived_id minted under a foreign key must reject");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)));
}

#[test]
fn kv_security_handle_without_derived_id_binding_is_rejected() {
    // Legacy KvPrefixHandle paths (`from_tokens`, `from_scoped_tokens`,
    // `from_parts`) do not bind a derived_id. A cache stack hardened
    // with MT-095 must refuse to restore from such a handle.
    let handle = KvPrefixHandle::from_tokens(&[1, 2, 3]).expect("legacy from_tokens");
    assert!(handle.derived_id().is_none());

    let model_id = ModelId::new_v7();
    let err = handle
        .verify_derived_against(model_id, handle.content_hash(), REGISTERED_T0)
        .expect_err("missing derived_id binding must reject");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)));
}

#[test]
fn kv_security_per_process_key_is_non_zero_entropy() {
    // Sanity that the process key is at least non-zero and has variety
    // across bytes. We don't claim cryptographic randomness assertions
    // (entropy proof is the OS-provided uuid v4 derivation), only that
    // the OnceLock initialised correctly under normal conditions.
    let key = handshake_core::model_runtime::kv_cache::__kv_prefix_key_for_tests();
    assert!(
        key.iter().any(|&b| b != 0),
        "process key must not be all zeros"
    );
    let unique = key
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert!(
        unique.len() >= 8,
        "process key should have at least 8 distinct byte values (got {})",
        unique.len()
    );
}

#[test]
fn kv_security_derive_id_is_deterministic_under_same_inputs() {
    // Same inputs under the same process key MUST produce the same
    // derived_id — otherwise the cache stack would never get a hit.
    let model_id = ModelId::new_v7();
    let a = KvPrefixHandle::derive_id(model_id, &CONTENT_A, REGISTERED_T0);
    let b = KvPrefixHandle::derive_id(model_id, &CONTENT_A, REGISTERED_T0);
    assert_eq!(a, b);
    // Different content_hash must produce a different id.
    let c = KvPrefixHandle::derive_id(model_id, &CONTENT_B, REGISTERED_T0);
    assert_ne!(a, c);
}

#[test]
fn kv_security_bind_derived_then_verify_self_round_trips() {
    // MT-095: the technique surface binds an unbound (from_scoped_tokens)
    // handle at commit via bind_derived and self-verifies at restore.
    let model_id = ModelId::new_v7();
    let unbound = KvPrefixHandle::from_scoped_tokens(b"scope", &[1, 2, 3]).expect("unbound handle");
    assert!(unbound.derived_id().is_none());
    let bound = unbound.bind_derived(model_id, REGISTERED_T0);
    assert!(bound.derived_id().is_some());
    bound
        .verify_self_against(model_id)
        .expect("a freshly bound handle verifies against itself");
}

#[test]
fn kv_security_verify_self_against_rejects_unbound_handle() {
    // A handle the commit path never bound (legacy/adapter-direct) must fail
    // closed at the restore gate.
    let model_id = ModelId::new_v7();
    let unbound = KvPrefixHandle::from_scoped_tokens(b"scope", &[1, 2, 3]).expect("unbound handle");
    let err = unbound
        .verify_self_against(model_id)
        .expect_err("unbound handle must fail closed");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)), "{err:?}");
}

#[test]
fn kv_security_verify_self_against_rejects_cross_model_binding() {
    // A handle bound under model A, replayed against model B, must reject:
    // verify re-derives from the handle's own content_hash + registered_at
    // but under the live model_id, so the keyed hash no longer matches.
    let model_a = ModelId::new_v7();
    let model_b = ModelId::new_v7();
    let bound = KvPrefixHandle::from_scoped_tokens(b"scope", &[1, 2, 3])
        .expect("unbound handle")
        .bind_derived(model_a, REGISTERED_T0);
    let err = bound
        .verify_self_against(model_b)
        .expect_err("cross-model replay of a bound handle must reject");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)), "{err:?}");
}

// ----------------------------------------------------------------------------
// MT-095 restore CHOKEPOINT tests: the binding + TTL gate lives in
// `KvCacheHandle::prefix_restore`, so EVERY caller (technique surface,
// subquadratic, or any in-crate handle holder) is gated in one place — closing
// the facade-bypass and enforcing the prefix-cache TTL with a typed error.
// ----------------------------------------------------------------------------

/// Test `KvCacheOps` that records whether the adapter layer was reached and
/// reports a configurable `prefix_cache_ttl_seconds`. Lets the facade gate be
/// exercised in default CI without a real model or feature build.
struct TtlGateKvCacheOps {
    ttl_seconds: u64,
    restored: AtomicBool,
}

impl TtlGateKvCacheOps {
    fn new(ttl_seconds: u64) -> Self {
        Self {
            ttl_seconds,
            restored: AtomicBool::new(false),
        }
    }

    fn was_restored(&self) -> bool {
        self.restored.load(Ordering::SeqCst)
    }
}

impl KvCacheOps for TtlGateKvCacheOps {
    fn quantization(&self) -> KvQuantSupport {
        KvQuantSupport::None
    }

    fn set_quantization(&self, _level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn occupancy(&self) -> KvCacheStats {
        KvCacheStats {
            bytes_used: 0,
            bytes_capacity: 0,
            prefix_cache_entries: 0,
            prefix_cache_hit_count: 0,
            prefix_cache_miss_count: 0,
            quant_level_current: KvQuantSupport::None,
        }
    }

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError> {
        KvPrefixHandle::from_tokens(prefix_tokens)
    }

    fn prefix_restore(&self, _handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        self.restored.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn prefix_evict(&self, _handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn prefix_cache_ttl_seconds(&self) -> u64 {
        self.ttl_seconds
    }
}

fn now_micros() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_micros() as i64
}

#[test]
fn kv_security_facade_restore_rejects_unbound_handle_before_adapter() {
    // MT-095.2: a direct facade caller holding a KvCacheHandle cannot bypass the
    // binding gate. An unbound handle fails closed BEFORE the adapter ops layer.
    let ops = Arc::new(TtlGateKvCacheOps::new(0));
    let cache = KvCacheHandle::with_ops("facade-gate", ops.clone());
    let model_id = ModelId::new_v7();
    let unbound = KvPrefixHandle::from_scoped_tokens(b"scope", &[1, 2, 3]).expect("unbound");

    let err = cache
        .prefix_restore(model_id, &unbound)
        .expect_err("unbound handle must fail closed at the facade chokepoint");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)), "{err:?}");
    assert!(
        !ops.was_restored(),
        "adapter ops must NOT be reached when the binding gate rejects"
    );
}

#[test]
fn kv_security_facade_restore_rejects_cross_model_bound_handle() {
    // MT-095.2: a handle bound under model A, replayed via the facade against
    // model B, is rejected at the chokepoint.
    let ops = Arc::new(TtlGateKvCacheOps::new(0));
    let cache = KvCacheHandle::with_ops("facade-gate", ops.clone());
    let model_a = ModelId::new_v7();
    let model_b = ModelId::new_v7();
    let ch = KvPrefixHandle::content_hash_for_tokens(&[1, 2, 3]);
    let bound_a = KvPrefixHandle::from_derived(model_a, ch, now_micros(), 3);

    let err = cache
        .prefix_restore(model_b, &bound_a)
        .expect_err("cross-model replay via facade must reject");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)), "{err:?}");
    assert!(!ops.was_restored());
}

#[test]
fn kv_security_facade_restore_enforces_prefix_cache_ttl() {
    // MT-095.1: "wait past TTL -> Err". A correctly-bound handle whose commit
    // timestamp is older than prefix_cache_ttl_seconds is rejected with a typed
    // expiry error BEFORE reaching the adapter; a fresh handle within the TTL
    // passes. No sleep needed — the expired handle is minted with an old, but
    // self-consistent, registered_at timestamp.
    let ops = Arc::new(TtlGateKvCacheOps::new(1)); // 1-second TTL
    let cache = KvCacheHandle::with_ops("ttl-gate", ops.clone());
    let model_id = ModelId::new_v7();
    let ch = KvPrefixHandle::content_hash_for_tokens(&[7, 8, 9]);

    let expired = KvPrefixHandle::from_derived(model_id, ch, now_micros() - 5_000_000, 3);
    // The binding self-verifies (derived from its own old timestamp) — only the
    // TTL gate should reject it, proving expiry is enforced independently.
    expired
        .verify_self_against(model_id)
        .expect("expired handle is still a valid binding; only TTL should reject it");
    let err = cache
        .prefix_restore(model_id, &expired)
        .expect_err("handle older than the TTL must reject");
    assert!(
        err.to_string().contains("expired"),
        "expected a typed expiry error, got: {err}"
    );
    assert!(
        !ops.was_restored(),
        "expired handle must not reach the adapter"
    );

    let fresh = KvPrefixHandle::from_derived(model_id, ch, now_micros(), 3);
    cache
        .prefix_restore(model_id, &fresh)
        .expect("a fresh handle within the TTL restores");
    assert!(ops.was_restored(), "fresh handle reaches the adapter");
}

#[test]
fn kv_security_facade_restore_ttl_zero_means_no_expiry() {
    // ttl_seconds == 0 is the historical "no expiry" default: even an ancient
    // (but correctly-bound) handle restores.
    let ops = Arc::new(TtlGateKvCacheOps::new(0));
    let cache = KvCacheHandle::with_ops("ttl-zero", ops.clone());
    let model_id = ModelId::new_v7();
    let ch = KvPrefixHandle::content_hash_for_tokens(&[1]);
    let ancient = KvPrefixHandle::from_derived(model_id, ch, now_micros() - 1_000_000_000, 1);
    cache
        .prefix_restore(model_id, &ancient)
        .expect("ttl=0 means no expiry");
    assert!(ops.was_restored());
}
