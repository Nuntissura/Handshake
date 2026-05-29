//! WP-KERNEL-004 cluster X.1 MT-183 Handoff bundle + announce-back provenance
//! integration tests.
//!
//! Spec authority:
//!   - .GOV/spec/master-spec-v02.186/spec-modules/02-system-architecture.md
//!     role-mailbox-handoff-bundle subsection [ADD v02.177] line 6160.
//!
//! Contract: MT-183 owns `src/backend/handshake_core/src/role_mailbox_v1/handoff.rs`
//! and this integration-test surface. Pure-Rust assertions are always-on;
//! Postgres-backed assertions are `#[ignore]`-gated on `POSTGRES_TEST_URL`
//! per the cluster X.1 Spec-Realism Gate convention used by
//! role_mailbox_repo_tests and role_mailbox_lease_tests.
//!
//! Adversarial coverage (per MT-183 `red_team.minimum_controls` and the
//! KERNEL_BUILDER subagent brief):
//!   (a) HandoffBundle round-trip (serde + provenance correlation).
//!   (b) AnnounceBack pairs with the right HandoffBundle by correlation id.
//!   (c) Announce-back without prior handoff returns a typed error
//!       (`ChainError::MissingBundleReference`).
//!   (d) Provenance chain depth limit (`MAX_PROVENANCE_CHAIN_DEPTH`) is
//!       enforced with `ChainError::DepthExceeded`.
//!   (e) Dangling correlation: announce-back references a bundle id absent
//!       from the verifier's bundle map (`ChainError::DanglingBundleCorrelation`).
//!   (f) Tampered bundle insert via `RoleMailboxRepository::insert_handoff_bundle`
//!       returns `MailboxError::HashMismatch` (Postgres-gated).
//!   (g) `get_handoff_bundle` round-trip preserves all fields and the
//!       reloaded bundle's `verify_hash` returns true (Postgres-gated).
//!   (h) Canonical-JSON hashing is order-stable: permuting `linked_artifacts`
//!       changes the hash, but reserializing a clone of the bundle produces
//!       the same hash (deterministic-hash golden-fixture).

use chrono::{Duration, Utc};
use handshake_core::role_mailbox::RoleId;
use handshake_core::role_mailbox_v1::{
    families::{
        AnnounceBackBody, ArtifactPointer, CapabilityGrant, CompletionState, MessageFamily,
    },
    handoff::{
        recompute_message_hash, AnnounceBackComposer, ChainError, HandoffBundleBuilder,
        MailboxHandoffBundleV1, ProvenanceLink, TranscriptPointer, MAX_PROVENANCE_CHAIN_DEPTH,
    },
    message::{MessageType, RoleMailboxMessage, RoleMailboxMessageId},
    repo::{MailboxError, RoleMailboxRepository},
    router::ExecutorKind,
    thread::{
        ClaimMode, LinkedRecordKind, ResponseAuthorityScope, RoleMailboxThread, RoleMailboxThreadId,
    },
    TakeoverPolicy,
};
use std::collections::HashMap;
use uuid::Uuid;

// ====================================================================
//                       Pure-Rust assertions
// ====================================================================

#[test]
fn mt_183_handoff_bundle_round_trip_serde() {
    // (a) HandoffBundle round-trip: full serde encode/decode preserves
    // every field including the content_hash, and the decoded bundle's
    // verify_hash() still returns true.
    let bundle = sample_bundle();
    assert!(bundle.verify_hash(), "freshly built bundle must verify");

    let encoded = serde_json::to_string(&bundle).expect("encode");
    let decoded: MailboxHandoffBundleV1 = serde_json::from_str(&encoded).expect("decode");
    assert_eq!(bundle, decoded, "serde round-trip preserves identity");
    assert!(
        decoded.verify_hash(),
        "round-tripped bundle still verifies — content_hash is part of the wire shape"
    );
}

#[test]
fn mt_183_announce_back_pairs_by_correlation_id() {
    // (b) AnnounceBackComposer::compose stamps bundle_id + provenance_chain.
    // verify_announce_back_pairing must accept the matched pair.
    let bundle = sample_bundle();
    let body = AnnounceBackComposer::compose(
        &bundle,
        "completed cleanly",
        vec![sample_artifact("doc-1")],
        CompletionState::Completed,
        vec![],
    );
    assert_eq!(
        body.bundle_id,
        Some(bundle.bundle_id),
        "announce-back must carry the originating bundle_id"
    );
    assert_eq!(
        body.sub_session_id,
        Some(bundle.created_by_session),
        "announce-back must carry the originating sub_session_id"
    );

    let mut bundles = HashMap::new();
    bundles.insert(bundle.bundle_id, bundle.clone());
    AnnounceBackComposer::verify_announce_back_pairing(&body, &bundles)
        .expect("matched bundle must pair");
}

#[test]
fn mt_183_announce_back_without_prior_handoff_typed_error() {
    // (c) Announce-back composed manually (no bundle_id) -> typed error.
    let orphan_body = AnnounceBackBody {
        sub_session_id: Some(Uuid::now_v7()),
        summary: "orphan".to_string(),
        artifacts: vec![],
        completion_state: CompletionState::Completed,
        provenance_chain: vec![],
        bundle_id: None,
    };
    let bundles = HashMap::new();
    let err = AnnounceBackComposer::verify_announce_back_pairing(&orphan_body, &bundles);
    assert!(
        matches!(err, Err(ChainError::MissingBundleReference)),
        "announce-back without bundle_id must fail with MissingBundleReference, got {err:?}"
    );
}

#[test]
fn mt_183_dangling_correlation_typed_error() {
    // (e) Announce-back references a bundle that the verifier doesn't know.
    let bundle = sample_bundle();
    let body = AnnounceBackComposer::compose(
        &bundle,
        "ok",
        vec![],
        CompletionState::Completed,
        vec![],
    );
    // Empty map -> dangling correlation.
    let bundles = HashMap::new();
    let err = AnnounceBackComposer::verify_announce_back_pairing(&body, &bundles);
    assert!(
        matches!(
            err,
            Err(ChainError::DanglingBundleCorrelation { bundle_id }) if bundle_id == bundle.bundle_id
        ),
        "missing bundle in verifier map must fail with DanglingBundleCorrelation, got {err:?}"
    );
}

#[test]
fn mt_183_provenance_chain_depth_limit_enforced() {
    // (d) A chain of length MAX_PROVENANCE_CHAIN_DEPTH + 1 must fail.
    let chain: Vec<ProvenanceLink> = (0..=MAX_PROVENANCE_CHAIN_DEPTH)
        .map(|_| ProvenanceLink {
            predecessor_message_id: Uuid::now_v7(),
            content_hash: "deadbeef".to_string(),
        })
        .collect();
    let messages = HashMap::new();
    let err = AnnounceBackComposer::verify_chain(&messages, &chain);
    match err {
        Err(ChainError::DepthExceeded { depth, limit }) => {
            assert_eq!(depth, MAX_PROVENANCE_CHAIN_DEPTH + 1);
            assert_eq!(limit, MAX_PROVENANCE_CHAIN_DEPTH);
        }
        other => panic!(
            "expected DepthExceeded for chain over MAX_PROVENANCE_CHAIN_DEPTH, got {other:?}"
        ),
    }
}

#[test]
fn mt_183_provenance_chain_walks_back_to_delegate_work() {
    // Build a 3-step chain: DelegateWork -> intermediate Blocker -> AnnounceBack.
    let thread_id = RoleMailboxThreadId::new_v7();
    let delegate = RoleMailboxMessage::new(
        thread_id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"task": "build mt183 surface"}),
    );
    let blocker = RoleMailboxMessage::new(
        thread_id,
        MessageType::Blocker,
        RoleId::Coder,
        vec![RoleId::Orchestrator],
        serde_json::json!({"why": "needs more context"}),
    );
    let mut messages = HashMap::new();
    let delegate_hash = recompute_message_hash(&delegate);
    let blocker_hash = recompute_message_hash(&blocker);
    let delegate_id = delegate.message_id.as_uuid();
    let blocker_id = blocker.message_id.as_uuid();
    messages.insert(delegate_id, delegate);
    messages.insert(blocker_id, blocker);
    let chain = vec![
        ProvenanceLink {
            predecessor_message_id: delegate_id,
            content_hash: delegate_hash,
        },
        ProvenanceLink {
            predecessor_message_id: blocker_id,
            content_hash: blocker_hash,
        },
    ];
    AnnounceBackComposer::verify_chain(&messages, &chain).expect("clean chain verifies");
}

#[test]
fn mt_183_empty_provenance_chain_typed_error() {
    let messages = HashMap::new();
    let err = AnnounceBackComposer::verify_chain(&messages, &[]);
    assert!(
        matches!(err, Err(ChainError::Empty)),
        "empty chain must fail with ChainError::Empty, got {err:?}"
    );
}

#[test]
fn mt_183_missing_predecessor_typed_error() {
    let messages = HashMap::new();
    let dangling_id = Uuid::now_v7();
    let chain = vec![ProvenanceLink {
        predecessor_message_id: dangling_id,
        content_hash: "deadbeef".to_string(),
    }];
    let err = AnnounceBackComposer::verify_chain(&messages, &chain);
    assert!(
        matches!(err, Err(ChainError::MissingPredecessor { message_id }) if message_id == dangling_id),
        "missing predecessor must produce ChainError::MissingPredecessor, got {err:?}"
    );
}

#[test]
fn mt_183_tampered_chain_link_typed_error() {
    let thread_id = RoleMailboxThreadId::new_v7();
    let msg = RoleMailboxMessage::new(
        thread_id,
        MessageType::DelegateWork,
        RoleId::Orchestrator,
        vec![RoleId::Coder],
        serde_json::json!({"task": "x"}),
    );
    let msg_id = msg.message_id.as_uuid();
    let mut messages = HashMap::new();
    messages.insert(msg_id, msg);
    let chain = vec![ProvenanceLink {
        predecessor_message_id: msg_id,
        content_hash: "0".repeat(64), // intentional wrong hash
    }];
    let err = AnnounceBackComposer::verify_chain(&messages, &chain);
    assert!(
        matches!(err, Err(ChainError::HashMismatch { message_id }) if message_id == msg_id),
        "tampered hash must produce ChainError::HashMismatch, got {err:?}"
    );
}

#[test]
fn mt_183_tampered_bundle_pairing_fails() {
    // verify_announce_back_pairing also re-verifies the bundle's stored
    // hash. If the verifier's bundle map contains a tampered bundle, the
    // pairing must fail with HashMismatch.
    let mut bundle = sample_bundle();
    let body = AnnounceBackComposer::compose(
        &bundle,
        "ok",
        vec![],
        CompletionState::Completed,
        vec![],
    );
    bundle.context_summary = "TAMPERED".to_string();
    let mut bundles = HashMap::new();
    bundles.insert(bundle.bundle_id, bundle.clone());
    let err = AnnounceBackComposer::verify_announce_back_pairing(&body, &bundles);
    assert!(
        matches!(err, Err(ChainError::HashMismatch { .. })),
        "tampered bundle must fail pairing with HashMismatch, got {err:?}"
    );
}

#[test]
fn mt_183_handoff_bundle_id_is_v7() {
    // HBR-INT-008: every Uuid mint site must use Uuid::now_v7().
    for _ in 0..16 {
        let b = sample_bundle();
        assert_eq!(
            b.bundle_id.get_version_num(),
            7,
            "bundle_id must be Uuid v7 per HBR-INT-008"
        );
    }
}

#[test]
fn mt_183_canonical_hash_is_deterministic_golden_fixture() {
    // (h) Canonical-JSON hashing is order-stable. Two bundles with the
    // same field values produce the same hash; cloning a bundle and
    // re-running recompute_hash must produce the same digest.
    let bundle = sample_bundle();
    let hash_a = bundle.recompute_hash();
    let bundle_clone = bundle.clone();
    let hash_b = bundle_clone.recompute_hash();
    assert_eq!(hash_a, hash_b, "recompute must be order-stable");

    // Hash is 32-byte sha256 hex (64 hex chars).
    assert_eq!(hash_a.len(), 64, "hash must be sha256 hex (64 chars)");
    assert!(
        hash_a.chars().all(|c| c.is_ascii_hexdigit()),
        "hash must be lowercase hex"
    );

    // Permuting artifact order changes the hash (proves linked_artifacts
    // is part of the canonical input — no field is silently dropped).
    let mut permuted = bundle.clone();
    permuted.linked_artifacts.reverse();
    if permuted.linked_artifacts.len() >= 2 {
        let hash_perm = permuted.recompute_hash();
        assert_ne!(
            hash_a, hash_perm,
            "permuting linked_artifacts must alter the canonical hash"
        );
    }
}

#[test]
fn mt_183_handoff_bundle_serializes_into_announce_back_via_message_family() {
    // Announce-back body is itself one of the 10 Phase-1 MessageFamily
    // variants — confirm it survives a round-trip through the family
    // wrapper while carrying the bundle_id and provenance chain.
    let bundle = sample_bundle();
    let provenance = vec![ProvenanceLink {
        predecessor_message_id: Uuid::now_v7(),
        content_hash: "ff".repeat(32),
    }];
    let body = AnnounceBackComposer::compose(
        &bundle,
        "done",
        vec![sample_artifact("art-x")],
        CompletionState::Completed,
        provenance.clone(),
    );
    let family = MessageFamily::AnnounceBack(body.clone());
    let encoded = family.encode_bounded().expect("encode under bound");
    let decoded: MessageFamily = serde_json::from_slice(&encoded).expect("decode");
    match decoded {
        MessageFamily::AnnounceBack(decoded_body) => {
            assert_eq!(decoded_body.bundle_id, Some(bundle.bundle_id));
            assert_eq!(decoded_body.provenance_chain, provenance);
            assert_eq!(decoded_body.summary, "done");
        }
        other => panic!("expected AnnounceBack family, got {other:?}"),
    }
}

#[test]
fn mt_183_chain_error_display_variants_are_distinct() {
    let e_empty = ChainError::Empty;
    let e_missing = ChainError::MissingPredecessor {
        message_id: Uuid::now_v7(),
    };
    let e_mismatch = ChainError::HashMismatch {
        message_id: Uuid::now_v7(),
    };
    let e_depth = ChainError::DepthExceeded {
        depth: 1024,
        limit: MAX_PROVENANCE_CHAIN_DEPTH,
    };
    let e_dangling = ChainError::DanglingBundleCorrelation {
        bundle_id: Uuid::now_v7(),
    };
    let e_missing_ref = ChainError::MissingBundleReference;
    assert_eq!(format!("{e_empty}"), "empty chain");
    assert!(format!("{e_missing}").contains("missing predecessor"));
    assert!(format!("{e_mismatch}").contains("chain broken"));
    assert!(format!("{e_depth}").contains("exceeds limit"));
    assert!(format!("{e_dangling}").contains("dangling bundle correlation"));
    assert!(format!("{e_missing_ref}").contains("without prior handoff"));
}

#[test]
fn mt_183_repo_handoff_bundle_methods_have_pgpool_only_constructor() {
    // CX-503R surface check: insert_handoff_bundle / get_handoff_bundle /
    // list_handoff_bundles_for_thread all live on
    // RoleMailboxRepository which is PgPool-only by construction. This
    // type-pin proves they cannot accept a SqliteConnection at compile
    // time (mirrors the pattern in role_mailbox_repo_tests.rs).
    let _ctor: fn(sqlx::PgPool) -> RoleMailboxRepository = RoleMailboxRepository::new;
}

#[test]
fn mt_183_mailbox_error_hash_mismatch_display_contains_both_hashes() {
    let err = MailboxError::HashMismatch {
        expected: "abc".to_string(),
        got: "def".to_string(),
    };
    let s = format!("{err}");
    assert!(s.contains("abc"), "Display must surface expected hash: {s}");
    assert!(s.contains("def"), "Display must surface got hash: {s}");
}

#[test]
fn mt_183_handoff_bundle_with_transcript_pointer_round_trip() {
    let bundle = HandoffBundleBuilder::new()
        .source_thread(RoleMailboxThreadId::new_v7())
        .source_message(RoleMailboxMessageId::new_v7())
        .target_role(RoleId::Validator)
        .target_executor_kind(ExecutorKind::Validator)
        .context_summary("review needed")
        .transcript(TranscriptPointer {
            transcript_id: "t-1".to_string(),
            uri: "file:///x".to_string(),
        })
        .expires_at(Utc::now() + Duration::hours(1))
        .build();
    assert!(bundle.verify_hash());
    let s = serde_json::to_string(&bundle).expect("encode");
    let back: MailboxHandoffBundleV1 = serde_json::from_str(&s).expect("decode");
    assert!(back.verify_hash());
    assert!(back.transcript_pointer.is_some());
}

// ====================================================================
//                  Postgres-gated integration tests
// ====================================================================

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_183_insert_handoff_bundle_recomputes_hash_and_rejects_tampered_input() {
    // (f) Direct insert with a tampered content_hash -> MailboxError::HashMismatch.
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let thread_id = thread.thread_id;
    repo.create_thread(thread).await.expect("create thread");

    // Build a clean bundle bound to the created thread + a message id.
    let source_msg_id = RoleMailboxMessageId::new_v7();
    let mut bundle = HandoffBundleBuilder::new()
        .source_thread(thread_id)
        .source_message(source_msg_id)
        .target_role(RoleId::Coder)
        .target_executor_kind(ExecutorKind::LocalSmallModel)
        .context_summary("hard-coded fixture")
        .linked_artifacts(vec![sample_artifact("art-1")])
        .capability_grants(vec![CapabilityGrant {
            capability_id: "cap.code".to_string(),
            granted_at_utc: Utc::now(),
            granted_by: RoleId::Orchestrator,
        }])
        .build();

    // Clean insert succeeds.
    repo.insert_handoff_bundle(&bundle)
        .await
        .expect("clean insert");

    // Now tamper content_hash on a fresh bundle and confirm typed error.
    bundle.bundle_id = Uuid::now_v7(); // avoid PK conflict; use a fresh id
    bundle.content_hash = "0".repeat(64);
    let err = repo.insert_handoff_bundle(&bundle).await;
    assert!(
        matches!(err, Err(MailboxError::HashMismatch { .. })),
        "tampered hash must return MailboxError::HashMismatch, got {err:?}"
    );
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_183_get_handoff_bundle_round_trip_preserves_fields_and_verify_hash() {
    // (g) Insert -> get -> verify all fields round-trip and verify_hash() passes.
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let thread_id = thread.thread_id;
    repo.create_thread(thread).await.expect("create thread");

    let msg_id = RoleMailboxMessageId::new_v7();
    let expires = Utc::now() + Duration::hours(2);
    let bundle = HandoffBundleBuilder::new()
        .source_thread(thread_id)
        .source_message(msg_id)
        .target_role(RoleId::Validator)
        .target_executor_kind(ExecutorKind::Validator)
        .context_summary("review handoff")
        .linked_artifacts(vec![
            sample_artifact("art-1"),
            sample_artifact("art-2"),
        ])
        .transcript(TranscriptPointer {
            transcript_id: "t-99".to_string(),
            uri: "s3://bucket/t-99".to_string(),
        })
        .capability_grants(vec![CapabilityGrant {
            capability_id: "cap.validate".to_string(),
            granted_at_utc: Utc::now(),
            granted_by: RoleId::Orchestrator,
        }])
        .expires_at(expires)
        .build();
    let bundle_id = bundle.bundle_id;
    repo.insert_handoff_bundle(&bundle)
        .await
        .expect("clean insert");

    let got = repo
        .get_handoff_bundle(bundle_id)
        .await
        .expect("get bundle")
        .expect("bundle row present");
    assert_eq!(got.bundle_id, bundle_id);
    assert_eq!(got.source_thread_id, thread_id.as_uuid());
    assert_eq!(got.source_message_id, msg_id.as_uuid());
    assert_eq!(got.target_role, RoleId::Validator);
    assert_eq!(got.target_executor_kind, ExecutorKind::Validator);
    assert_eq!(got.context_summary, "review handoff");
    assert_eq!(got.linked_artifacts.len(), 2);
    assert!(got.transcript_pointer.is_some());
    assert_eq!(got.capability_grants.len(), 1);
    assert!(
        got.verify_hash(),
        "round-tripped bundle from Postgres must verify hash"
    );
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_183_list_handoff_bundles_for_thread_chronological() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let thread_id = thread.thread_id;
    repo.create_thread(thread).await.expect("create thread");

    for _ in 0..3 {
        let msg_id = RoleMailboxMessageId::new_v7();
        let bundle = HandoffBundleBuilder::new()
            .source_thread(thread_id)
            .source_message(msg_id)
            .target_role(RoleId::Coder)
            .target_executor_kind(ExecutorKind::LocalSmallModel)
            .context_summary("chronological-test")
            .build();
        repo.insert_handoff_bundle(&bundle).await.expect("insert");
    }
    let bundles = repo
        .list_handoff_bundles_for_thread(thread_id)
        .await
        .expect("list bundles");
    assert_eq!(bundles.len(), 3, "all three bundles must be listed");
    for w in bundles.windows(2) {
        assert!(
            w[0].created_at_utc <= w[1].created_at_utc,
            "chronological ordering must hold"
        );
    }
    // All round-tripped bundles verify hash.
    for b in &bundles {
        assert!(b.verify_hash());
    }
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_183_get_handoff_bundle_unknown_returns_none() {
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");
    let unknown = Uuid::now_v7();
    let got = repo
        .get_handoff_bundle(unknown)
        .await
        .expect("get unknown does not error");
    assert!(got.is_none(), "unknown bundle_id must return Ok(None)");
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_183_insert_with_clean_hash_then_get_then_verify_chain_end_to_end() {
    // End-to-end: insert bundle, compose AnnounceBack with a provenance
    // chain referring to a stored message, verify chain + bundle pairing.
    let pool = postgres_pool().await;
    let repo = RoleMailboxRepository::new(pool);
    repo.ensure_schema().await.expect("schema");

    let thread = sample_open_thread();
    let thread_id = thread.thread_id;
    repo.create_thread(thread).await.expect("create thread");

    // Append a DelegateWork message to use as the chain anchor.
    let delegate = repo
        .append_message(
            thread_id,
            MessageType::DelegateWork,
            RoleId::Orchestrator,
            vec![RoleId::Coder],
            serde_json::json!({"task": "MT-183 e2e"}),
        )
        .await
        .expect("append delegate work");

    // Build + insert the handoff bundle bound to the delegate message id.
    let bundle = HandoffBundleBuilder::new()
        .source_thread(thread_id)
        .source_message(delegate.message_id)
        .target_role(RoleId::Coder)
        .target_executor_kind(ExecutorKind::LocalSmallModel)
        .context_summary("end-to-end fixture")
        .build();
    repo.insert_handoff_bundle(&bundle).await.expect("insert");

    // Compose AnnounceBack with a provenance chain referencing the delegate.
    let delegate_hash = recompute_message_hash(&delegate);
    let chain = vec![ProvenanceLink {
        predecessor_message_id: delegate.message_id.as_uuid(),
        content_hash: delegate_hash,
    }];
    let body = AnnounceBackComposer::compose(
        &bundle,
        "e2e done",
        vec![],
        CompletionState::Completed,
        chain.clone(),
    );

    // verify_chain over a single-message HashMap built from the message we
    // just delivered.
    let mut messages = HashMap::new();
    messages.insert(delegate.message_id.as_uuid(), delegate);
    AnnounceBackComposer::verify_chain(&messages, &body.provenance_chain).expect("chain verifies");

    // verify_announce_back_pairing over a single-bundle HashMap.
    let mut bundles = HashMap::new();
    let reloaded = repo
        .get_handoff_bundle(bundle.bundle_id)
        .await
        .expect("get bundle")
        .expect("bundle present");
    bundles.insert(reloaded.bundle_id, reloaded);
    AnnounceBackComposer::verify_announce_back_pairing(&body, &bundles).expect("pairing verifies");
}

// ====================================================================
//                            Test helpers
// ====================================================================

fn sample_bundle() -> MailboxHandoffBundleV1 {
    HandoffBundleBuilder::new()
        .source_thread(RoleMailboxThreadId::new_v7())
        .source_message(RoleMailboxMessageId::new_v7())
        .target_role(RoleId::Coder)
        .target_executor_kind(ExecutorKind::LocalSmallModel)
        .context_summary("sample handoff")
        .linked_artifacts(vec![sample_artifact("a-1"), sample_artifact("a-2")])
        .build()
}

fn sample_artifact(id: &str) -> ArtifactPointer {
    ArtifactPointer {
        artifact_id: id.to_string(),
        uri: format!("artifact://{id}"),
        content_hash: Some(format!("{id}-hash")),
    }
}

fn sample_open_thread() -> RoleMailboxThread {
    RoleMailboxThread::open(
        "mt-183 handoff fixture",
        LinkedRecordKind::Mt,
        Some("MT-183".to_string()),
        vec![ExecutorKind::LocalSmallModel, ExecutorKind::Validator],
        ClaimMode::Handoff,
        TakeoverPolicy::Never,
        ResponseAuthorityScope::LeaseHolder,
    )
}

async fn postgres_pool() -> sqlx::PgPool {
    let url = std::env::var("POSTGRES_TEST_URL")
        .expect("ENVIRONMENT_BLOCKED: POSTGRES_TEST_URL not set");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}
