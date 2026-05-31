//! MT-117 — INF-9 Cross-session SSM state restore tests.
//!
//! Exercises three surfaces from candle/state_vector.rs:
//!
//! 1. In-process bytes-in / bytes-out: persist_to_bytes +
//!    load_from_bytes round-trip + integrity guards.
//! 2. Cross-handle re-mint bridge: load_into_handle resolves
//!    MT-114.cross_handle_finding by re-minting a KvPrefixHandle under
//!    a freshly-loaded handle's prefix_scope.
//! 3. ArtifactStore + KernelActionCatalogV1 disk integration (the MT-117
//!    contract proper): persist_to_artifact_store mints an artifact_id
//!    and writes the envelope under .handshake/artifacts/L3/{uuid}/,
//!    load_from_artifact_store reads it back with manifest+envelope
//!    integrity validation, and load_from_artifact_store_into_handle is
//!    the operator-facing one-shot that pairs the disk read with the
//!    cross-handle re-mint bridge.
//!
//! Validator focus coverage:
//! - "Process-restart simulation test (drop + reload + restore
//!   deterministic)" — covered by `process_restart_simulation_via_load_into_handle`
//!   for the in-process surface and
//!   `process_restart_simulation_via_artifact_store_round_trips_deterministic`
//!   for the disk path.
//! - "Sha256 + variant validation enforced" — covered by the
//!   `cross_session_restore_rejects_*` cluster.
//! - "Mutations through KernelActionCatalogV1" — covered by the
//!   `state_vector_persist_action_id_*` cluster which pins the
//!   catalog-bound action_id + write_box schema_id constants.

use std::fs;

use handshake_core::kernel::action_catalog::kernel002_action_catalog;
use handshake_core::model_runtime::{
    candle::{
        load_from_artifact_store, load_from_artifact_store_into_handle, load_from_bytes,
        load_into_handle, persist_to_artifact_store, persist_to_bytes, SSMStateSnapshot,
        SSMStateVariant, SSMTensorSnapshot, StateVectorHandle, StateVectorPersistEnvelope,
        StateVectorPersistRecord, STATE_VECTOR_PERSIST_ACTION_ID,
        STATE_VECTOR_PERSIST_ENVELOPE_VERSION, STATE_VECTOR_PERSIST_WRITE_BOX_SCHEMA_ID,
    },
    KvCacheOps, LicenseTag, ModelId, ModelRuntimeError, OperatorId,
};
use tempfile::TempDir;
use uuid::Uuid;

fn artifact_sha_a() -> String {
    "ab".repeat(32)
}

fn artifact_sha_b() -> String {
    "cd".repeat(32)
}

fn mamba2_snapshot(seed: u8) -> SSMStateSnapshot {
    SSMStateSnapshot::Mamba2 {
        conv_states: vec![SSMTensorSnapshot::new("f32", vec![2, 4], vec![seed; 32])
            .expect("conv_states tensor")],
        ssm_states: vec![SSMTensorSnapshot::new("f32", vec![4, 8], vec![seed; 128])
            .expect("ssm_states tensor")],
    }
}

fn rwkv_v5_snapshot(seed: u8) -> SSMStateSnapshot {
    SSMStateSnapshot::RwkvV5 {
        token_shift: vec![SSMTensorSnapshot::new("f32", vec![2, 4], vec![seed; 32])
            .expect("token_shift tensor")],
        ssm: vec![SSMTensorSnapshot::new("f32", vec![4, 8], vec![seed; 128])
            .expect("ssm tensor")],
    }
}

fn make_handle_with_sha(snapshot: SSMStateSnapshot, sha: String) -> StateVectorHandle {
    StateVectorHandle::new_in_memory(
        format!("test-sv-{}", uuid::Uuid::now_v7()),
        ModelId::new_v7(),
        sha,
        snapshot,
    )
    .expect("handle must construct")
}

fn operator() -> OperatorId {
    OperatorId::new("operator-ilja")
}

fn license() -> LicenseTag {
    LicenseTag::new("operator-private")
}

#[test]
fn persist_to_bytes_then_load_from_bytes_round_trips_envelope() {
    let handle = make_handle_with_sha(mamba2_snapshot(0xAA), artifact_sha_a());
    let prefix = handle
        .prefix_commit(&[1u32, 2, 3])
        .expect("commit must succeed");
    let bytes = persist_to_bytes(&handle, &prefix, operator(), license())
        .expect("persist_to_bytes must serialize the snapshot");
    assert!(!bytes.is_empty());

    let envelope: StateVectorPersistEnvelope =
        load_from_bytes(&bytes).expect("load_from_bytes must deserialize + validate");
    assert_eq!(
        envelope.envelope_version,
        STATE_VECTOR_PERSIST_ENVELOPE_VERSION
    );
    assert_eq!(envelope.metadata.persisted_by, operator());
    assert_eq!(envelope.metadata.license_tag, license());
    assert_eq!(envelope.metadata.n_tokens_advanced, 3);
    assert_eq!(envelope.metadata.variant_label, "mamba2");
    assert_eq!(envelope.record.snapshot.variant(), SSMStateVariant::Mamba2);
}

#[test]
fn load_from_bytes_rejects_envelope_version_mismatch() {
    let handle = make_handle_with_sha(mamba2_snapshot(0xBB), artifact_sha_a());
    let prefix = handle
        .prefix_commit(&[5u32])
        .expect("commit must succeed");
    let bytes = persist_to_bytes(&handle, &prefix, operator(), license())
        .expect("persist_to_bytes must serialize");

    // Re-parse, mutate version, re-serialize.
    let mut envelope: StateVectorPersistEnvelope =
        serde_json::from_slice(&bytes).expect("re-parse envelope");
    envelope.envelope_version = "hsk.subquad.state_vector.persist.v999".to_string();
    let tampered_bytes = serde_json::to_vec(&envelope).expect("re-serialize tampered envelope");

    let err = load_from_bytes(&tampered_bytes)
        .expect_err("version mismatch must be rejected");
    assert!(
        matches!(err, ModelRuntimeError::KvCacheError(ref message) if message.contains("envelope version mismatch")),
        "expected version-mismatch error; got {err:?}"
    );
}

#[test]
fn process_restart_simulation_via_load_into_handle() {
    // Phase 1: snapshot from a "session A" handle.
    let handle_a = make_handle_with_sha(mamba2_snapshot(0xCC), artifact_sha_a());
    let prefix_a = handle_a
        .prefix_commit(&[10u32, 20, 30])
        .expect("commit on handle A must succeed");
    let bytes = persist_to_bytes(&handle_a, &prefix_a, operator(), license())
        .expect("persist must succeed");

    // Phase 2: drop handle A (simulating Handshake process restart) and
    // mint a fresh "session B" handle on the SAME model + artifact_sha.
    drop(handle_a);
    drop(prefix_a);

    let handle_b = make_handle_with_sha(mamba2_snapshot(0xCC), artifact_sha_a());

    // Phase 3: cross-session restore via the re-mint bridge.
    let reminted_prefix = load_into_handle(&handle_b, &bytes, &[10u32, 20, 30])
        .expect("load_into_handle must re-mint a handle scoped to handle_b");

    // The re-minted handle is scoped to handle_b's prefix_scope, so its
    // content_hash differs from prefix_a's content_hash (different model_id).
    assert_eq!(reminted_prefix.token_count(), 3);

    // MT-095 regression guard: the re-minted cross-session handle is BOUND, so a
    // subsequent restore through the gated KvCacheHandle chokepoint
    // (subquadratic::state_restore / kv_cache_technique::prefix_restore) verifies
    // it instead of fail-closing as unbound. Without the load_into_handle binding
    // fix this would reject and break cross-session state recovery.
    reminted_prefix
        .verify_self_against(handle_b.model_id())
        .expect("re-minted cross-session handle must be bound for the gated restore path");

    // Subsequent prefix_restore via KvCacheOps must succeed on handle_b.
    handle_b
        .prefix_restore(&reminted_prefix)
        .expect("prefix_restore on the re-minted handle must succeed after restore");

    let occupancy = handle_b.occupancy();
    assert!(
        occupancy.prefix_cache_hit_count >= 1,
        "hit counter must advance on the post-restore prefix_restore call"
    );
}

#[test]
fn cross_session_restore_rejects_sha256_mismatch_with_operator_guidance() {
    let handle_a = make_handle_with_sha(mamba2_snapshot(0xDD), artifact_sha_a());
    let prefix = handle_a
        .prefix_commit(&[1u32, 2])
        .expect("commit must succeed");
    let bytes =
        persist_to_bytes(&handle_a, &prefix, operator(), license()).expect("persist must succeed");

    // Mint a handle with a DIFFERENT artifact_sha (operator loaded a
    // different model quantization).
    let handle_b = make_handle_with_sha(mamba2_snapshot(0xDD), artifact_sha_b());

    let err = load_into_handle(&handle_b, &bytes, &[1u32, 2])
        .expect_err("artifact_sha mismatch must reject");
    let message = match err {
        ModelRuntimeError::KvCacheError(message) => message,
        other => panic!("expected KvCacheError; got {other:?}"),
    };
    assert!(
        message.contains("different model artifact"),
        "expected operator guidance about the artifact mismatch; got: {message}"
    );
    assert!(
        message.contains("either load that artifact or recapture"),
        "expected operator guidance about the recovery path; got: {message}"
    );
}

#[test]
fn cross_session_restore_rejects_variant_mismatch() {
    let handle_a = make_handle_with_sha(mamba2_snapshot(0xEE), artifact_sha_a());
    let prefix = handle_a
        .prefix_commit(&[7u32])
        .expect("commit must succeed");
    let bytes =
        persist_to_bytes(&handle_a, &prefix, operator(), license()).expect("persist must succeed");

    // Mint an RWKV-v5 handle with the SAME artifact_sha — variant differs.
    let handle_b = make_handle_with_sha(rwkv_v5_snapshot(0xEE), artifact_sha_a());

    let err = load_into_handle(&handle_b, &bytes, &[7u32])
        .expect_err("variant mismatch must reject");
    let message = match err {
        ModelRuntimeError::KvCacheError(message) => message,
        other => panic!("expected KvCacheError; got {other:?}"),
    };
    assert!(
        message.contains("variant mismatch"),
        "expected variant mismatch error; got: {message}"
    );
}

#[test]
fn cross_session_restore_rejects_prefix_token_length_mismatch() {
    let handle_a = make_handle_with_sha(mamba2_snapshot(0xFF), artifact_sha_a());
    let prefix = handle_a
        .prefix_commit(&[1u32, 2, 3, 4])
        .expect("commit must succeed");
    let bytes =
        persist_to_bytes(&handle_a, &prefix, operator(), license()).expect("persist must succeed");

    let handle_b = make_handle_with_sha(mamba2_snapshot(0xFF), artifact_sha_a());
    // Different token count -> rejected before re-mint.
    let err = load_into_handle(&handle_b, &bytes, &[1u32, 2])
        .expect_err("token count mismatch must reject");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(ref message) if message.contains("prefix_tokens length mismatch")));
}

#[test]
fn cross_session_restore_rejects_corrupted_envelope_bytes() {
    let bytes = b"this is not a valid envelope";
    let err = load_from_bytes(bytes).expect_err("corrupted bytes must reject");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(ref message) if message.contains("deserialization failed")));
}

#[test]
fn cross_session_restore_rejects_envelope_with_tampered_record_hash() {
    let handle = make_handle_with_sha(mamba2_snapshot(0x11), artifact_sha_a());
    let prefix = handle
        .prefix_commit(&[1u32])
        .expect("commit must succeed");
    let bytes =
        persist_to_bytes(&handle, &prefix, operator(), license()).expect("persist must succeed");

    // Tamper: re-parse, mutate snapshot_hash to invalidate integrity.
    let mut envelope: StateVectorPersistEnvelope =
        serde_json::from_slice(&bytes).expect("re-parse envelope");
    envelope.record.snapshot_hash = [0u8; 32];
    let tampered_bytes = serde_json::to_vec(&envelope).expect("re-serialize tampered envelope");

    let err = load_from_bytes(&tampered_bytes)
        .expect_err("tampered snapshot_hash must reject via validate_integrity");
    assert!(matches!(err, ModelRuntimeError::KvCacheError(_)));
}

#[test]
fn persist_envelope_version_is_a_stable_string_constant() {
    // The wire format version is a public const that disk-integration
    // tooling can pin in tests. Bumping it requires a migration shim;
    // this test exists so the bump is intentional, never accidental.
    assert_eq!(
        STATE_VECTOR_PERSIST_ENVELOPE_VERSION,
        "hsk.subquad.state_vector.persist.v1"
    );
}

// ------------------------------------------------------------------
// MT-117 disk integration (ArtifactStore + KernelActionCatalogV1).
// ------------------------------------------------------------------

#[test]
fn state_vector_persist_action_id_is_stable() {
    // The catalog action_id is the wire string the IPC dispatcher binds
    // to. Bumping it is a contract break for any operator-facing
    // surface that names the action by string — this test pins it so
    // a future rename is intentional, never accidental.
    assert_eq!(
        STATE_VECTOR_PERSIST_ACTION_ID,
        "kernel.subquadratic.persist_state"
    );
    assert_eq!(
        STATE_VECTOR_PERSIST_WRITE_BOX_SCHEMA_ID,
        "hsk.write_box.state_vector_persist@1"
    );
}

#[test]
fn state_vector_persist_action_is_registered_in_kernel002_catalog() {
    let catalog = kernel002_action_catalog();
    let action = catalog
        .action(STATE_VECTOR_PERSIST_ACTION_ID)
        .expect("kernel.subquadratic.persist_state must be registered in the kernel catalog");
    assert_eq!(action.action_id, STATE_VECTOR_PERSIST_ACTION_ID);
    // The write_box schema id the engine produces must match the
    // catalog action's declared expected_write_boxes entry.
    let write_box = action
        .expected_write_boxes
        .iter()
        .find(|wb| wb.write_box_schema_id == STATE_VECTOR_PERSIST_WRITE_BOX_SCHEMA_ID)
        .expect("persist_state action must declare the state_vector_persist write_box");
    assert!(
        !write_box.write_box_kind.is_empty(),
        "write_box.kind must be set"
    );
    // KERNEL_BUILDER must be eligible to dispatch this action — the
    // engine is invoked from the kernel runtime path.
    assert!(
        action
            .role_eligibility
            .iter()
            .any(|role| role == "KERNEL_BUILDER"),
        "KERNEL_BUILDER must be in role_eligibility; got {:?}",
        action.role_eligibility
    );
}

#[test]
fn persist_to_artifact_store_round_trips_via_load_from_artifact_store() {
    let workspace = TempDir::new().expect("tempdir");
    let handle = make_handle_with_sha(mamba2_snapshot(0x21), artifact_sha_a());
    let prefix = handle
        .prefix_commit(&[1u32, 2, 3, 4, 5])
        .expect("commit must succeed");

    let record = persist_to_artifact_store(&handle, &prefix, operator(), license(), workspace.path())
        .expect("persist_to_artifact_store must write the envelope and return a record");

    assert_eq!(record.envelope_version, STATE_VECTOR_PERSIST_ENVELOPE_VERSION);
    assert_eq!(record.variant, SSMStateVariant::Mamba2);
    assert_eq!(record.persisted_by, operator());
    assert_eq!(record.license_tag, license());
    assert!(record.byte_len > 0);
    assert_eq!(record.content_hash.len(), 64);

    let envelope = load_from_artifact_store(record.artifact_id, workspace.path())
        .expect("load_from_artifact_store must return the persisted envelope");
    assert_eq!(envelope.envelope_version, STATE_VECTOR_PERSIST_ENVELOPE_VERSION);
    assert_eq!(envelope.record.state_vector_id, record.state_vector_id);
    assert_eq!(envelope.record.snapshot.variant(), SSMStateVariant::Mamba2);
    assert_eq!(envelope.metadata.persisted_by, record.persisted_by);
}

#[test]
fn persist_to_artifact_store_writes_into_l3_layer_under_workspace_root() {
    let workspace = TempDir::new().expect("tempdir");
    let handle = make_handle_with_sha(mamba2_snapshot(0x22), artifact_sha_a());
    let prefix = handle.prefix_commit(&[7u32]).expect("commit must succeed");

    let record = persist_to_artifact_store(&handle, &prefix, operator(), license(), workspace.path())
        .expect("persist must succeed");

    // The ArtifactStore lays out artifacts as
    // <workspace>/.handshake/artifacts/L3/<uuid>/{payload,artifact.json}
    let expected_root = workspace
        .path()
        .join(".handshake")
        .join("artifacts")
        .join("L3")
        .join(record.artifact_id.to_string());
    assert!(
        expected_root.exists() && expected_root.is_dir(),
        "expected artifact root {} to exist as a directory",
        expected_root.display()
    );
    assert!(expected_root.join("payload").is_file());
    assert!(expected_root.join("artifact.json").is_file());

    // Classification must be Medium (state vectors carry model behavior)
    // — read the manifest back to assert this without relying on a
    // private getter.
    let manifest_bytes =
        fs::read(expected_root.join("artifact.json")).expect("manifest must be readable");
    let manifest_value: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).expect("manifest must be JSON");
    assert_eq!(manifest_value["classification"], "medium");
    assert_eq!(manifest_value["layer"], "L3");
    assert_eq!(manifest_value["kind"], "bundle");
    assert_eq!(manifest_value["exportable"], true);
}

#[test]
fn process_restart_simulation_via_artifact_store_round_trips_deterministic() {
    let workspace = TempDir::new().expect("tempdir");

    // Phase 1: a "session A" handle commits a snapshot and persists it
    // to the ArtifactStore. The operator gets back an artifact_id.
    let handle_a = make_handle_with_sha(mamba2_snapshot(0xA1), artifact_sha_a());
    let prefix_a = handle_a
        .prefix_commit(&[10u32, 20, 30])
        .expect("commit on handle A must succeed");
    let record: StateVectorPersistRecord = persist_to_artifact_store(
        &handle_a,
        &prefix_a,
        operator(),
        license(),
        workspace.path(),
    )
    .expect("persist must succeed");
    let artifact_id = record.artifact_id;

    // Phase 2: simulate Handshake process restart by dropping the
    // in-memory state. The workspace tempdir survives the drops.
    drop(handle_a);
    drop(prefix_a);

    // Phase 3: a fresh "session B" handle is created on the SAME model
    // + artifact_sha (operator reloaded the same GGUF). It loads the
    // persisted snapshot back via load_from_artifact_store_into_handle,
    // which pairs the disk read with the cross-handle re-mint bridge.
    let handle_b = make_handle_with_sha(mamba2_snapshot(0xA1), artifact_sha_a());
    let reminted_prefix =
        load_from_artifact_store_into_handle(&handle_b, artifact_id, &[10u32, 20, 30], workspace.path())
            .expect("disk-restore must succeed on the fresh handle");
    assert_eq!(reminted_prefix.token_count(), 3);

    // Phase 4: KvCacheOps::prefix_restore on the re-minted handle must
    // succeed and advance handle_b's hit counter.
    handle_b
        .prefix_restore(&reminted_prefix)
        .expect("prefix_restore on the re-minted handle must succeed after disk restore");
    let occupancy = handle_b.occupancy();
    assert!(
        occupancy.prefix_cache_hit_count >= 1,
        "hit counter must advance after disk-restore + prefix_restore"
    );
}

#[test]
fn load_from_artifact_store_rejects_unknown_artifact_id() {
    let workspace = TempDir::new().expect("tempdir");
    // No persist happened; load with a bogus artifact_id must reject
    // with a typed error that names the missing root path.
    let bogus = Uuid::now_v7();
    let err = load_from_artifact_store(bogus, workspace.path())
        .expect_err("unknown artifact_id must reject");
    let message = match err {
        ModelRuntimeError::KvCacheError(m) => m,
        other => panic!("expected KvCacheError; got {other:?}"),
    };
    assert!(
        message.contains("not found in ArtifactStore"),
        "expected unknown-artifact guidance; got: {message}"
    );
    assert!(
        message.contains(&bogus.to_string()),
        "expected the unknown artifact_id in the message; got: {message}"
    );
}

#[test]
fn load_from_artifact_store_rejects_tampered_payload_after_persist() {
    let workspace = TempDir::new().expect("tempdir");
    let handle = make_handle_with_sha(mamba2_snapshot(0xB2), artifact_sha_a());
    let prefix = handle.prefix_commit(&[1u32]).expect("commit must succeed");
    let record = persist_to_artifact_store(&handle, &prefix, operator(), license(), workspace.path())
        .expect("persist must succeed");

    // Tamper the on-disk payload after persist completes. The
    // manifest's content_hash is stale relative to the new payload, so
    // load_from_artifact_store must reject before any envelope-side
    // logic runs.
    let payload_path = workspace
        .path()
        .join(".handshake")
        .join("artifacts")
        .join("L3")
        .join(record.artifact_id.to_string())
        .join("payload");
    fs::write(&payload_path, b"this is not the original envelope")
        .expect("tamper-write must succeed");

    let err = load_from_artifact_store(record.artifact_id, workspace.path())
        .expect_err("tampered payload must reject before envelope parse");
    let message = match err {
        ModelRuntimeError::KvCacheError(m) => m,
        other => panic!("expected KvCacheError; got {other:?}"),
    };
    assert!(
        message.contains("payload tampered after persist"),
        "expected tamper-detection guidance; got: {message}"
    );
    assert!(
        message.contains(&record.content_hash),
        "expected the expected sha256 in the message; got: {message}"
    );
}

#[test]
fn load_from_artifact_store_rejects_corrupted_manifest_json() {
    let workspace = TempDir::new().expect("tempdir");
    let handle = make_handle_with_sha(mamba2_snapshot(0xC3), artifact_sha_a());
    let prefix = handle.prefix_commit(&[1u32, 2]).expect("commit must succeed");
    let record = persist_to_artifact_store(&handle, &prefix, operator(), license(), workspace.path())
        .expect("persist must succeed");

    let manifest_path = workspace
        .path()
        .join(".handshake")
        .join("artifacts")
        .join("L3")
        .join(record.artifact_id.to_string())
        .join("artifact.json");
    fs::write(&manifest_path, b"{ not valid JSON }").expect("corrupt manifest write");

    let err = load_from_artifact_store(record.artifact_id, workspace.path())
        .expect_err("corrupt manifest must reject");
    let message = match err {
        ModelRuntimeError::KvCacheError(m) => m,
        other => panic!("expected KvCacheError; got {other:?}"),
    };
    assert!(
        message.contains("manifest deserialize failed"),
        "expected manifest-deserialize error; got: {message}"
    );
}

#[test]
fn load_from_artifact_store_into_handle_rejects_sha256_mismatch_on_target() {
    // Persist a snapshot against artifact_sha_a, then attempt to
    // load_from_artifact_store_into_handle on a handle bound to
    // artifact_sha_b (different model quantization). The cross-handle
    // bridge must reject with the operator-facing guidance.
    let workspace = TempDir::new().expect("tempdir");
    let handle_a = make_handle_with_sha(mamba2_snapshot(0xD4), artifact_sha_a());
    let prefix = handle_a
        .prefix_commit(&[3u32, 4])
        .expect("commit must succeed");
    let record = persist_to_artifact_store(&handle_a, &prefix, operator(), license(), workspace.path())
        .expect("persist must succeed");

    let handle_b = make_handle_with_sha(mamba2_snapshot(0xD4), artifact_sha_b());
    let err = load_from_artifact_store_into_handle(
        &handle_b,
        record.artifact_id,
        &[3u32, 4],
        workspace.path(),
    )
    .expect_err("sha mismatch must reject through the cross-handle bridge");
    let message = match err {
        ModelRuntimeError::KvCacheError(m) => m,
        other => panic!("expected KvCacheError; got {other:?}"),
    };
    assert!(
        message.contains("different model artifact"),
        "expected operator guidance about artifact mismatch; got: {message}"
    );
}

#[test]
fn load_from_artifact_store_into_handle_rejects_variant_mismatch_on_target() {
    let workspace = TempDir::new().expect("tempdir");
    let handle_a = make_handle_with_sha(mamba2_snapshot(0xE5), artifact_sha_a());
    let prefix = handle_a
        .prefix_commit(&[6u32])
        .expect("commit must succeed");
    let record = persist_to_artifact_store(&handle_a, &prefix, operator(), license(), workspace.path())
        .expect("persist must succeed");

    // Different variant on the target handle.
    let handle_b = make_handle_with_sha(rwkv_v5_snapshot(0xE5), artifact_sha_a());
    let err = load_from_artifact_store_into_handle(
        &handle_b,
        record.artifact_id,
        &[6u32],
        workspace.path(),
    )
    .expect_err("variant mismatch must reject through the cross-handle bridge");
    let message = match err {
        ModelRuntimeError::KvCacheError(m) => m,
        other => panic!("expected KvCacheError; got {other:?}"),
    };
    assert!(
        message.contains("variant mismatch"),
        "expected variant-mismatch guidance; got: {message}"
    );
}

#[test]
fn persist_to_artifact_store_does_not_double_write_under_concurrent_calls() {
    // Two threads call persist_to_artifact_store concurrently on the
    // same handle + prefix. Each call mints its own UUIDv7 artifact_id
    // (the kernel ArtifactStore is artifact-id-keyed, not handle-keyed),
    // so both writes must succeed and produce distinct artifact roots.
    // This proves the persistence layer is collision-free under
    // realistic concurrent operator action without smuggling in a hidden
    // "happens to work because nothing runs in parallel" assumption.
    use std::sync::Arc;
    use std::thread;

    let workspace = Arc::new(TempDir::new().expect("tempdir"));
    let handle = Arc::new(make_handle_with_sha(mamba2_snapshot(0xF6), artifact_sha_a()));
    let prefix = handle
        .prefix_commit(&[8u32, 9])
        .expect("commit must succeed");
    let prefix_arc = Arc::new(prefix);

    let mut handles = Vec::new();
    for _ in 0..4 {
        let workspace = Arc::clone(&workspace);
        let handle = Arc::clone(&handle);
        let prefix = Arc::clone(&prefix_arc);
        let op = operator();
        let lic = license();
        handles.push(thread::spawn(move || {
            persist_to_artifact_store(&handle, &prefix, op, lic, workspace.path())
                .expect("concurrent persist must succeed")
        }));
    }

    let mut artifact_ids = Vec::new();
    for h in handles {
        let record = h.join().expect("worker must not panic");
        artifact_ids.push(record.artifact_id);
    }

    let unique: std::collections::HashSet<Uuid> = artifact_ids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        artifact_ids.len(),
        "every concurrent persist must mint a unique artifact_id; got duplicates in {artifact_ids:?}"
    );

    // Each artifact_id must round-trip through load_from_artifact_store
    // — i.e. the concurrent writes did not corrupt each other.
    for id in artifact_ids {
        let env = load_from_artifact_store(id, workspace.path())
            .expect("each concurrent-written artifact must be loadable");
        assert_eq!(env.record.snapshot.variant(), SSMStateVariant::Mamba2);
    }
}

#[test]
fn persist_to_artifact_store_rejects_export_from_unknown_prefix_handle() {
    use handshake_core::model_runtime::KvPrefixHandle;

    let workspace = TempDir::new().expect("tempdir");
    let handle = make_handle_with_sha(mamba2_snapshot(0x07), artifact_sha_a());

    // Mint a handle but DON'T commit it — the StateVectorOps cache has
    // no entry for this handle, so the export step must fail closed.
    let unbound_prefix = KvPrefixHandle::from_scoped_tokens(b"foreign.scope", &[1u32, 2, 3])
        .expect("unbound prefix must construct");

    let err = persist_to_artifact_store(
        &handle,
        &unbound_prefix,
        operator(),
        license(),
        workspace.path(),
    )
    .expect_err("persist with unknown prefix must reject before any disk write");
    let message = match err {
        ModelRuntimeError::KvCacheError(m) => m,
        other => panic!("expected KvCacheError; got {other:?}"),
    };
    assert!(
        message.contains("unknown") || message.contains("not found") || message.contains("content_hash"),
        "expected typed export-side error; got: {message}"
    );

    // No artifact directory should have been created — the export
    // rejection happens before write_file_artifact runs.
    let l3_root = workspace.path().join(".handshake").join("artifacts").join("L3");
    if l3_root.exists() {
        let count = fs::read_dir(&l3_root)
            .expect("read l3")
            .filter_map(Result::ok)
            .count();
        assert_eq!(
            count, 0,
            "no L3 artifact directory should exist after a failed persist"
        );
    }
}
