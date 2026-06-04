use handshake_core::model_runtime::{
    candle::{
        SSMStateSnapshot, SSMStateVariant, SSMTensorSnapshot, StateVectorHandle, StateVectorId,
        StateVectorSnapshotRecord,
    },
    KvCacheOps, KvPrefixHandle, KvQuantSupport, ModelId,
};

#[test]
fn candle_state_vector_tests_ids_are_uuid_v7_and_reject_other_versions() {
    let id = StateVectorId::new_v7();
    assert_eq!(id.as_uuid().get_version_num(), 7);

    assert!(StateVectorId::from_uuid(uuid::Uuid::nil()).is_err());
    let v4_shaped_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    assert!(StateVectorId::from_uuid(v4_shaped_id).is_err());
}

#[test]
fn candle_state_vector_tests_commit_restore_evict_and_occupancy_follow_kv_cache_ops() {
    let handle = mamba_handle();
    assert_eq!(handle.variant(), SSMStateVariant::Mamba2);
    assert_eq!(handle.quantization(), KvQuantSupport::None);
    assert!(handle.set_quantization(KvQuantSupport::Q4).is_err());

    let committed = handle.prefix_commit(&[10, 20, 30]).unwrap();
    assert_eq!(committed.prefix_id().get_version_num(), 7);
    assert_eq!(committed.token_count(), 3);

    let stats = handle.occupancy();
    assert_eq!(stats.prefix_cache_entries, 1);
    assert!(stats.bytes_used > 0, "state-vector bytes must be reported");
    assert!(stats.bytes_capacity >= stats.bytes_used);
    assert_eq!(stats.quant_level_current, KvQuantSupport::None);

    handle.prefix_restore(&committed).unwrap();

    let mut tampered_hash = *committed.content_hash();
    tampered_hash[0] ^= 0xff;
    let tampered = KvPrefixHandle::from_parts(
        committed.prefix_id(),
        tampered_hash,
        committed.token_count(),
    )
    .unwrap();
    let error = handle.prefix_restore(&tampered).unwrap_err();
    assert!(error.to_string().contains("content_hash"), "{error}");

    handle.prefix_evict(committed).unwrap();
    assert_eq!(handle.occupancy().prefix_cache_entries, 0);
    handle.evict_all().unwrap();
    assert_eq!(handle.occupancy().prefix_cache_entries, 0);
}

#[test]
fn candle_state_vector_tests_variant_mismatch_and_artifact_hash_binding_are_rejected() {
    let mamba = mamba_handle();
    let committed = mamba.prefix_commit(&[1, 2, 3]).unwrap();
    let record = mamba.export_snapshot(&committed).unwrap();

    let rwkv = StateVectorHandle::new_in_memory(
        "test-rwkv-v6",
        record.model_id,
        record.artifact_sha256.clone(),
        SSMStateSnapshot::RwkvV6 {
            token_shift: vec![tensor("f32", &[1, 4], &[9, 10, 11, 12])],
            ssm: vec![tensor("f32", &[1, 4], &[13, 14, 15, 16])],
        },
    )
    .unwrap();
    let variant_error = rwkv
        .restore_snapshot_record(&committed, record.clone())
        .unwrap_err();
    assert!(
        variant_error.to_string().contains("variant mismatch"),
        "{variant_error}"
    );

    let wrong_artifact = StateVectorHandle::new_in_memory(
        "test-mamba2-wrong-artifact",
        record.model_id,
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        mamba_snapshot(),
    )
    .unwrap();
    let artifact_error = wrong_artifact
        .restore_snapshot_record(&committed, record)
        .unwrap_err();
    assert!(
        artifact_error.to_string().contains("artifact_sha256"),
        "{artifact_error}"
    );
}

#[test]
fn candle_state_vector_tests_snapshot_records_round_trip_and_detect_payload_tampering() {
    let handle = mamba_handle();
    let committed = handle.prefix_commit(&[42, 43]).unwrap();
    let record = handle.export_snapshot(&committed).unwrap();

    let json = serde_json::to_string(&record).unwrap();
    let round_tripped: StateVectorSnapshotRecord = serde_json::from_str(&json).unwrap();
    round_tripped.validate_integrity().unwrap();
    handle
        .restore_snapshot_record(&committed, round_tripped.clone())
        .unwrap();

    let mut tampered = round_tripped;
    if let SSMStateSnapshot::Mamba2 { conv_states, .. } = &mut tampered.snapshot {
        conv_states[0].bytes[0] ^= 0xff;
    }
    let error = handle
        .restore_snapshot_record(&committed, tampered)
        .unwrap_err();
    assert!(error.to_string().contains("snapshot_hash"), "{error}");
}

fn mamba_handle() -> StateVectorHandle {
    StateVectorHandle::new_in_memory(
        "test-mamba2",
        ModelId::new_v7(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        mamba_snapshot(),
    )
    .unwrap()
}

fn mamba_snapshot() -> SSMStateSnapshot {
    SSMStateSnapshot::Mamba2 {
        conv_states: vec![tensor("f32", &[1, 2, 2], &[1, 2, 3, 4])],
        ssm_states: vec![tensor("f32", &[1, 2, 2], &[5, 6, 7, 8])],
    }
}

fn tensor(dtype: &str, shape: &[usize], bytes: &[u8]) -> SSMTensorSnapshot {
    SSMTensorSnapshot::new(dtype, shape.to_vec(), bytes.to_vec()).unwrap()
}

// ---------------------------------------------------------------------------
// CRIT-1 / MT-088 — live SSM source round trip
//
// Pre-remediation, InMemoryStateVectorOps stored a placeholder snapshot
// that was never updated from the live model. The test below proves the
// new contract:
//
//   1. prefix_commit captures the snapshot the live source returns NOW
//      (not a placeholder).
//   2. prefix_restore writes the captured snapshot back into the live
//      source so a subsequent extract sees it.
//
// A `MockSsmSource` stands in for `CandleMamba2Model` so the test can
// avoid loading a real GGUF artifact while still exercising the wiring
// the adapter sets up at load time.
// ---------------------------------------------------------------------------
#[cfg(feature = "candle-runtime-engine")]
#[test]
fn candle_state_vector_tests_live_source_commit_then_restore_round_trips_through_source() {
    use std::sync::{Arc, Mutex};

    use handshake_core::model_runtime::candle::SsmStateSource;
    use handshake_core::model_runtime::ModelRuntimeError;

    struct MockSsmSource {
        snapshot: Mutex<SSMStateSnapshot>,
    }
    impl SsmStateSource for MockSsmSource {
        fn extract(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
            Ok(self.snapshot.lock().unwrap().clone())
        }
        fn restore(&self, snapshot: &SSMStateSnapshot) -> Result<(), ModelRuntimeError> {
            *self.snapshot.lock().unwrap() = snapshot.clone();
            Ok(())
        }
    }

    let initial_state = SSMStateSnapshot::Mamba2 {
        conv_states: vec![tensor("f32", &[1, 4], &[1, 2, 3, 4])],
        ssm_states: vec![tensor("f32", &[1, 4], &[5, 6, 7, 8])],
    };
    let source = Arc::new(MockSsmSource {
        snapshot: Mutex::new(initial_state.clone()),
    });
    let handle = StateVectorHandle::new_in_memory_with_source(
        "test-mamba2-live-source",
        ModelId::new_v7(),
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        SSMStateVariant::Mamba2,
        Arc::clone(&source) as Arc<dyn SsmStateSource>,
    )
    .unwrap();

    // Simulate `forward(4 tokens)` advancing the model — the live
    // source's snapshot now differs from the initial state.
    let post_forward_state = SSMStateSnapshot::Mamba2 {
        conv_states: vec![tensor("f32", &[1, 4], &[10, 20, 30, 40])],
        ssm_states: vec![tensor("f32", &[1, 4], &[50, 60, 70, 80])],
    };
    *source.snapshot.lock().unwrap() = post_forward_state.clone();

    // prefix_commit must pull from the live source. Pre-fix, this
    // captured the placeholder; the assertion below would have failed.
    let committed = handle.prefix_commit(&[1, 2, 3, 4]).unwrap();
    let exported = handle.export_snapshot(&committed).unwrap();
    assert_eq!(
        exported.snapshot, post_forward_state,
        "prefix_commit must snapshot live source state, not the placeholder"
    );

    // Simulate `reset_generation_state()` — the live source goes back to
    // initial, so prefix_restore must re-seat post_forward_state into it.
    *source.snapshot.lock().unwrap() = initial_state.clone();
    handle.prefix_restore(&committed).unwrap();
    let live_after_restore = source.snapshot.lock().unwrap().clone();
    assert_eq!(
        live_after_restore, post_forward_state,
        "prefix_restore must write the captured snapshot back into the live source"
    );

    // A second extract through the handle round-trips again from the
    // now-restored live source.
    let second_commit = handle.prefix_commit(&[1, 2, 3, 4]).unwrap();
    let second_exported = handle.export_snapshot(&second_commit).unwrap();
    assert_eq!(
        second_exported.snapshot, post_forward_state,
        "post-restore re-commit must re-capture the same live state"
    );
}

// CRIT-1 / MT-088: when the declared variant disagrees with the live
// source's first extraction, the constructor must reject up-front so the
// adapter never seats a handle that would silently mis-tag prefix
// commits.
#[cfg(feature = "candle-runtime-engine")]
#[test]
fn candle_state_vector_tests_new_in_memory_with_source_rejects_variant_mismatch() {
    use std::sync::{Arc, Mutex};

    use handshake_core::model_runtime::candle::SsmStateSource;
    use handshake_core::model_runtime::ModelRuntimeError;

    struct MockSsmSource {
        snapshot: Mutex<SSMStateSnapshot>,
    }
    impl SsmStateSource for MockSsmSource {
        fn extract(&self) -> Result<SSMStateSnapshot, ModelRuntimeError> {
            Ok(self.snapshot.lock().unwrap().clone())
        }
        fn restore(&self, snapshot: &SSMStateSnapshot) -> Result<(), ModelRuntimeError> {
            *self.snapshot.lock().unwrap() = snapshot.clone();
            Ok(())
        }
    }

    let source = Arc::new(MockSsmSource {
        snapshot: Mutex::new(SSMStateSnapshot::Mamba2 {
            conv_states: vec![tensor("f32", &[1, 2], &[1, 2])],
            ssm_states: vec![tensor("f32", &[1, 2], &[3, 4])],
        }),
    });

    // Declare RwkvV6 but the source produces Mamba2 → must error.
    let result = StateVectorHandle::new_in_memory_with_source(
        "test-variant-mismatch",
        ModelId::new_v7(),
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        SSMStateVariant::RwkvV6,
        Arc::clone(&source) as Arc<dyn SsmStateSource>,
    );
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("source variant mismatch"),
        "expected variant-mismatch error, got: {error}"
    );
}
