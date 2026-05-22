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

use handshake_core::model_runtime::{KvPrefixHandle, ModelId, ModelRuntimeError};

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
