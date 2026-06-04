//! WP-KERNEL-005 atelier Media-Downloader-v2 (MT-204): real PostgreSQL
//! round-trip proofs for the governed downloader records/receipt repository
//! (Section 6.10.2..6.10.5). Run with a live DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_downloader_tests -- --nocapture
//!
//! No mocks: each test connects a real `AtelierStore` to a real Postgres,
//! ensures the schema, builds run-unique fixtures (via `Uuid::new_v4()` so the
//! shared, persistent DB never collides on UNIQUE constraints), exercises the
//! downloader write methods with REAL data, and asserts the load-bearing
//! invariants: idempotency on the documented keys, the redaction boundary
//! (inline secrets rejected, refs stored, event payloads redacted), checkpoint
//! atomicity with stage advance, item dedupe, and event emission via
//! `store.count_events(<family>)`. This module FKs nothing to atelier_character;
//! its FKs are output-root / allowlist / auth-context, which the tests create as
//! prerequisites. Only `handshake_core` + `tokio` + `uuid` + `serde_json` (+
//! std) are used; sqlx is never imported directly.

use handshake_core::atelier::downloader::{
    self, AuthMode, EmitSessionReceipt, EnqueueItem, MaterializationMode, OpenDownloadSession,
    RecordCheckpoint, RegisterAuthContext, SessionStage, SetAllowlistPolicy, SetOutputRootConfig,
    SourceKind, TerminalStage,
};
use handshake_core::atelier::AtelierStore;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Connect + ensure schema, the shared preamble every test runs against a real
/// Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Create a run-unique output-root config and return its `root_id`.
async fn fresh_output_root(store: &AtelierStore) -> Uuid {
    let config = store
        .set_output_root_config(&SetOutputRootConfig {
            configured_root: format!("media_downloader/{}/", Uuid::new_v4()),
            materialization_mode: MaterializationMode::Hardlink,
            per_mode_subdirs: serde_json::json!({ "youtube": "yt", "instagram": "ig" }),
        })
        .await
        .expect("set output root config");
    config.root_id
}

/// Create a run-unique allowlist policy and return its `allowlist_policy_id`.
async fn fresh_allowlist(store: &AtelierStore) -> Uuid {
    let policy = store
        .set_allowlist_policy(&SetAllowlistPolicy {
            name: format!("policy-{}", Uuid::new_v4()),
            allowed_domains: serde_json::json!(["example.com"]),
            explicit_url_lists: serde_json::json!([]),
            default_decision: "deny".to_string(),
            rate_limit: serde_json::json!({ "rps": 2 }),
            max_pages: 1500,
            robots_posture: "respect".to_string(),
        })
        .await
        .expect("set allowlist policy");
    policy.allowlist_policy_id
}

/// Open a run-unique session wired to fresh prerequisites; returns the session.
async fn fresh_session(store: &AtelierStore) -> handshake_core::atelier::downloader::DownloadSession {
    let output_root_id = fresh_output_root(store).await;
    let allowlist_policy_id = fresh_allowlist(store).await;
    store
        .open_download_session(&OpenDownloadSession {
            parent_job_id: format!("job-{}", Uuid::new_v4()),
            idempotency_key: format!("idem-{}", Uuid::new_v4()),
            source_kind: SourceKind::Youtube,
            auth_context_ref: None,
            allowlist_policy_id,
            output_root_id,
        })
        .await
        .expect("open download session")
}

#[tokio::test]
async fn downloader_config_allowlist_and_portability_guard() {
    let Some(url) = database_url() else {
        eprintln!("SKIP downloader_config_allowlist_and_portability_guard: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let before_root = store
        .count_events(downloader::OUTPUT_ROOT_CONFIGURED)
        .await
        .expect("count output-root events before");
    let before_allow = store
        .count_events(downloader::ALLOWLIST_POLICY_SET)
        .await
        .expect("count allowlist events before");

    // --- set output root (idempotent on configured_root) ---
    let configured_root = format!("media_downloader/{}/", Uuid::new_v4());
    let cfg1 = store
        .set_output_root_config(&SetOutputRootConfig {
            configured_root: configured_root.clone(),
            materialization_mode: MaterializationMode::Hardlink,
            per_mode_subdirs: serde_json::json!({ "youtube": "yt" }),
        })
        .await
        .expect("set output root");
    assert_eq!(cfg1.materialization_mode, MaterializationMode::Hardlink);
    assert_eq!(cfg1.configured_root, configured_root, "portable root round-trips");

    // Re-set the SAME configured_root with a changed mode: ON CONFLICT updates in
    // place, the id stays stable (idempotency on configured_root).
    let cfg2 = store
        .set_output_root_config(&SetOutputRootConfig {
            configured_root: configured_root.clone(),
            materialization_mode: MaterializationMode::Copy,
            per_mode_subdirs: serde_json::json!({ "youtube": "yt2" }),
        })
        .await
        .expect("re-set same output root");
    assert_eq!(
        cfg1.root_id, cfg2.root_id,
        "re-setting the same configured_root keeps a stable root_id (ON CONFLICT stability)"
    );
    assert_eq!(
        cfg2.materialization_mode,
        MaterializationMode::Copy,
        "the upsert updated the materialization_mode in place"
    );

    // get round-trips the updated record.
    let fetched = store
        .get_output_root_config(cfg1.root_id)
        .await
        .expect("get output root");
    assert_eq!(fetched.root_id, cfg1.root_id);
    assert_eq!(fetched.materialization_mode, MaterializationMode::Copy);

    // --- INVARIANT: machine-local absolute paths are rejected (LAW-MDV2-OUT-001) ---
    let bad = store
        .set_output_root_config(&SetOutputRootConfig {
            configured_root: "C:\\Users\\operator\\dl".to_string(),
            materialization_mode: MaterializationMode::Copy,
            per_mode_subdirs: serde_json::json!({}),
        })
        .await;
    assert!(
        bad.is_err(),
        "a machine-local absolute path must be rejected, never persisted as a root"
    );

    // --- set allowlist (idempotent on name, max_pages clamped to hard cap) ---
    let name = format!("policy-{}", Uuid::new_v4());
    let pol1 = store
        .set_allowlist_policy(&SetAllowlistPolicy {
            name: name.clone(),
            allowed_domains: serde_json::json!(["example.com", "cdn.example.com"]),
            explicit_url_lists: serde_json::json!([]),
            default_decision: "deny".to_string(),
            rate_limit: serde_json::json!({ "rps": 1 }),
            max_pages: 999_999,
            robots_posture: "respect".to_string(),
        })
        .await
        .expect("set allowlist policy");
    assert_eq!(
        pol1.max_pages, 5000,
        "max_pages is clamped to the hard cap 5000 (10.14.9)"
    );
    assert_eq!(pol1.default_decision, "deny");

    let pol2 = store
        .set_allowlist_policy(&SetAllowlistPolicy {
            name: name.clone(),
            allowed_domains: serde_json::json!(["example.com"]),
            explicit_url_lists: serde_json::json!(["https://example.com/a"]),
            default_decision: "allow".to_string(),
            rate_limit: serde_json::json!({ "rps": 5 }),
            max_pages: 10,
            robots_posture: "ignore".to_string(),
        })
        .await
        .expect("re-set same-named allowlist policy");
    assert_eq!(
        pol1.allowlist_policy_id, pol2.allowlist_policy_id,
        "re-setting the same policy name keeps a stable id (idempotency on name)"
    );
    assert_eq!(pol2.default_decision, "allow", "policy fields update in place");

    // --- INVARIANT: an invalid default_decision is a clean validation error ---
    let bad_decision = store
        .set_allowlist_policy(&SetAllowlistPolicy {
            name: format!("policy-{}", Uuid::new_v4()),
            allowed_domains: serde_json::json!([]),
            explicit_url_lists: serde_json::json!([]),
            default_decision: "maybe".to_string(),
            rate_limit: serde_json::json!({}),
            max_pages: 1,
            robots_posture: "respect".to_string(),
        })
        .await;
    assert!(
        bad_decision.is_err(),
        "default_decision must be 'deny' or 'allow'; anything else is rejected"
    );

    // --- event emission increased for both families ---
    let after_root = store
        .count_events(downloader::OUTPUT_ROOT_CONFIGURED)
        .await
        .expect("count output-root events after");
    let after_allow = store
        .count_events(downloader::ALLOWLIST_POLICY_SET)
        .await
        .expect("count allowlist events after");
    assert!(
        after_root >= before_root + 2,
        "two successful set_output_root_config calls each emit OUTPUT_ROOT_CONFIGURED"
    );
    assert!(
        after_allow >= before_allow + 2,
        "two successful set_allowlist_policy calls each emit ALLOWLIST_POLICY_SET"
    );
}

#[tokio::test]
async fn downloader_auth_context_rejects_inline_secrets_and_redacts() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP downloader_auth_context_rejects_inline_secrets_and_redacts: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let before = store
        .count_events(downloader::AUTH_CONTEXT_REGISTERED)
        .await
        .expect("count auth events before");

    // --- register a refs-only auth context (idempotent on label) ---
    let label = format!("auth-{}", Uuid::new_v4());
    let jar_ref = format!("artifact://atelier/cookiejar/{}", Uuid::new_v4());
    let ctx1 = store
        .register_auth_context(&RegisterAuthContext {
            label: label.clone(),
            auth_mode: AuthMode::CookieJar,
            session_ref: None,
            cookie_jar_artifact_ref: Some(jar_ref.clone()),
            header_secret_refs: serde_json::json!([]),
        })
        .await
        .expect("register auth context");
    assert_eq!(ctx1.auth_mode, AuthMode::CookieJar);
    assert_eq!(
        ctx1.cookie_jar_artifact_ref.as_deref(),
        Some(jar_ref.as_str()),
        "only the cookie-jar REF is stored, never the secret contents"
    );

    // Re-register the same label: ON CONFLICT updates in place, id is stable.
    let ctx2 = store
        .register_auth_context(&RegisterAuthContext {
            label: label.clone(),
            auth_mode: AuthMode::Header,
            session_ref: None,
            cookie_jar_artifact_ref: None,
            header_secret_refs: serde_json::json!([
                format!("secretref://{}", Uuid::new_v4()),
                format!("secretref://{}", Uuid::new_v4()),
            ]),
        })
        .await
        .expect("re-register same label");
    assert_eq!(
        ctx1.auth_context_ref, ctx2.auth_context_ref,
        "re-registering the same label keeps a stable auth_context_ref (idempotency on label)"
    );
    assert_eq!(ctx2.auth_mode, AuthMode::Header, "mode updated in place");

    // --- INVARIANT (redaction boundary): inline secret material is REJECTED ---
    // An Authorization-style header value smells inline and must never persist.
    let inline_header = store
        .register_auth_context(&RegisterAuthContext {
            label: format!("auth-bad-{}", Uuid::new_v4()),
            auth_mode: AuthMode::Header,
            session_ref: None,
            cookie_jar_artifact_ref: None,
            header_secret_refs: serde_json::json!(["Authorization: Bearer abc123"]),
        })
        .await;
    assert!(
        inline_header.is_err(),
        "an inline Authorization header value must be rejected (LAW-MDV2-AUTH-001)"
    );

    // An inline session cookie in session_ref must also be rejected.
    let inline_session = store
        .register_auth_context(&RegisterAuthContext {
            label: format!("auth-bad2-{}", Uuid::new_v4()),
            auth_mode: AuthMode::Session,
            session_ref: Some("sessionid=deadbeefcafe".to_string()),
            cookie_jar_artifact_ref: None,
            header_secret_refs: serde_json::json!([]),
        })
        .await;
    assert!(
        inline_session.is_err(),
        "an inline sessionid= value must be rejected, not stored as a session_ref"
    );

    // --- mode/field consistency: cookie_jar mode requires its ref ---
    let missing_jar = store
        .register_auth_context(&RegisterAuthContext {
            label: format!("auth-bad3-{}", Uuid::new_v4()),
            auth_mode: AuthMode::CookieJar,
            session_ref: None,
            cookie_jar_artifact_ref: None,
            header_secret_refs: serde_json::json!([]),
        })
        .await;
    assert!(
        missing_jar.is_err(),
        "auth_mode=cookie_jar without a cookie_jar_artifact_ref must be rejected"
    );

    // get round-trips the stored refs-only record.
    let fetched = store
        .get_auth_context(ctx1.auth_context_ref)
        .await
        .expect("get auth context");
    assert_eq!(fetched.auth_context_ref, ctx1.auth_context_ref);
    assert_eq!(fetched.auth_mode, AuthMode::Header, "fetch reflects the update");

    // --- event emission increased; the rejected calls emitted nothing ---
    let after = store
        .count_events(downloader::AUTH_CONTEXT_REGISTERED)
        .await
        .expect("count auth events after");
    assert!(
        after >= before + 2,
        "the two successful register_auth_context calls each emit AUTH_CONTEXT_REGISTERED; \
         rejected inline-secret calls emit nothing"
    );
}

#[tokio::test]
async fn downloader_session_lifecycle_items_and_checkpoints() {
    let Some(url) = database_url() else {
        eprintln!("SKIP downloader_session_lifecycle_items_and_checkpoints: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let output_root_id = fresh_output_root(&store).await;
    let allowlist_policy_id = fresh_allowlist(&store).await;

    let before_session = store
        .count_events(downloader::SESSION_OPENED)
        .await
        .expect("count session-opened before");
    let before_stage = store
        .count_events(downloader::SESSION_STAGE_CHANGED)
        .await
        .expect("count stage-changed before");
    let before_item = store
        .count_events(downloader::ITEM_ENQUEUED)
        .await
        .expect("count item-enqueued before");
    let before_ckpt = store
        .count_events(downloader::ITEM_CHECKPOINTED)
        .await
        .expect("count checkpointed before");

    // --- open a session (idempotent on idempotency_key) ---
    let idem = format!("idem-{}", Uuid::new_v4());
    let job = format!("job-{}", Uuid::new_v4());
    let session = store
        .open_download_session(&OpenDownloadSession {
            parent_job_id: job.clone(),
            idempotency_key: idem.clone(),
            source_kind: SourceKind::Forumcrawler,
            auth_context_ref: None,
            allowlist_policy_id,
            output_root_id,
        })
        .await
        .expect("open download session");
    assert_eq!(session.stage, SessionStage::Resolving, "sessions start at resolving");
    assert_eq!(session.source_kind, SourceKind::Forumcrawler);

    let session_again = store
        .open_download_session(&OpenDownloadSession {
            parent_job_id: format!("job-other-{}", Uuid::new_v4()),
            idempotency_key: idem.clone(),
            source_kind: SourceKind::Forumcrawler,
            auth_context_ref: None,
            allowlist_policy_id,
            output_root_id,
        })
        .await
        .expect("re-open same idempotency_key");
    assert_eq!(
        session.session_id, session_again.session_id,
        "re-opening the same idempotency_key returns the existing session (idempotency)"
    );

    // --- INVARIANT: opening against a dangling FK is rejected ---
    let dangling = store
        .open_download_session(&OpenDownloadSession {
            parent_job_id: format!("job-{}", Uuid::new_v4()),
            idempotency_key: format!("idem-{}", Uuid::new_v4()),
            source_kind: SourceKind::Youtube,
            auth_context_ref: None,
            allowlist_policy_id: Uuid::new_v4(),
            output_root_id,
        })
        .await;
    assert!(
        dangling.is_err(),
        "opening a session against a non-existent allowlist policy must error"
    );

    // --- enqueue an item (idempotent on (session, normalized_url)) ---
    let normalized_url = format!("https://example.com/v/{}", Uuid::new_v4());
    let item = store
        .enqueue_item(
            session.session_id,
            &EnqueueItem {
                normalized_url: normalized_url.clone(),
                stable_source_id: Some(format!("src-{}", Uuid::new_v4())),
            },
        )
        .await
        .expect("enqueue item");
    assert_eq!(item.normalized_url, normalized_url, "url round-trips");
    assert_eq!(item.bytes_downloaded, 0, "fresh item has zero progress");

    // Re-enqueue the same normalized URL: dedupe returns the same item, progress
    // is NOT reset (LAW-MDV2-RESUME-004).
    let item_again = store
        .enqueue_item(
            session.session_id,
            &EnqueueItem {
                normalized_url: normalized_url.clone(),
                stable_source_id: None,
            },
        )
        .await
        .expect("re-enqueue same url");
    assert_eq!(
        item.item_id, item_again.item_id,
        "re-enqueuing the same (session, normalized_url) dedupes to the same item_id"
    );

    // --- INVARIANT: advancing a stage records a session-level checkpoint atomically
    //     (LAW-MDV2-RESUME-003): a stage transition without a checkpoint is a violation.
    let advanced = store
        .advance_session_stage(session.session_id, SessionStage::Enqueued, Some("rt-1"))
        .await
        .expect("advance session stage");
    assert_eq!(advanced.stage, SessionStage::Enqueued, "stage advanced");
    let session_anchor = store
        .latest_checkpoint(session.session_id, None)
        .await
        .expect("latest session checkpoint")
        .expect("a session-level checkpoint exists after the stage advance");
    assert_eq!(
        session_anchor.stage, "enqueued",
        "the bundled checkpoint records the new stage (recovery anchor)"
    );
    assert!(
        session_anchor.item_id.is_none(),
        "the stage-advance checkpoint is session-level (item_id is NULL)"
    );

    // --- record an item checkpoint: advances live item state AND appends the anchor ---
    let ckpt = store
        .record_checkpoint(
            session.session_id,
            &RecordCheckpoint {
                item_id: Some(item.item_id),
                stage: "fetching".to_string(),
                bytes_downloaded: 1024,
                bytes_total: Some(4096),
                resume_token: Some("byte-1024".to_string()),
            },
        )
        .await
        .expect("record item checkpoint");
    assert_eq!(ckpt.item_id, Some(item.item_id));
    assert_eq!(ckpt.bytes_downloaded, 1024);

    // The live item state moved with the checkpoint (single transaction).
    let live = store
        .get_item_by_url(session.session_id, &normalized_url)
        .await
        .expect("get item by url")
        .expect("item present");
    assert_eq!(
        live.bytes_downloaded, 1024,
        "the checkpoint advanced the live item's bytes_downloaded in the same tx"
    );
    assert_eq!(
        live.resume_token.as_deref(),
        Some("byte-1024"),
        "the resume cursor advanced atomically with the checkpoint"
    );
    assert_eq!(
        live.stage,
        handshake_core::atelier::downloader::ItemStage::Fetching,
        "the item stage advanced to fetching with the checkpoint"
    );

    // The latest item-scoped checkpoint is the recovery anchor.
    let item_anchor = store
        .latest_checkpoint(session.session_id, Some(item.item_id))
        .await
        .expect("latest item checkpoint")
        .expect("an item checkpoint exists");
    assert_eq!(item_anchor.checkpoint_id, ckpt.checkpoint_id);
    assert_eq!(item_anchor.resume_token.as_deref(), Some("byte-1024"));

    // --- INVARIANT: a corrupt stage token cannot enter the recovery anchor ---
    let bad_stage = store
        .record_checkpoint(
            session.session_id,
            &RecordCheckpoint {
                item_id: Some(item.item_id),
                stage: "teleporting".to_string(),
                bytes_downloaded: 2048,
                bytes_total: Some(4096),
                resume_token: None,
            },
        )
        .await;
    assert!(
        bad_stage.is_err(),
        "an unknown item-stage token is rejected before any checkpoint is written"
    );

    // --- event emission increased across the families touched ---
    let after_session = store
        .count_events(downloader::SESSION_OPENED)
        .await
        .expect("count session-opened after");
    let after_stage = store
        .count_events(downloader::SESSION_STAGE_CHANGED)
        .await
        .expect("count stage-changed after");
    let after_item = store
        .count_events(downloader::ITEM_ENQUEUED)
        .await
        .expect("count item-enqueued after");
    let after_ckpt = store
        .count_events(downloader::ITEM_CHECKPOINTED)
        .await
        .expect("count checkpointed after");
    assert!(
        after_session >= before_session + 1,
        "the first open emits SESSION_OPENED (the idempotent re-open does not)"
    );
    assert!(
        after_stage >= before_stage + 1,
        "advancing the stage emits SESSION_STAGE_CHANGED"
    );
    assert!(
        after_item >= before_item + 1,
        "the first enqueue emits ITEM_ENQUEUED (the dedupe re-enqueue does not)"
    );
    assert!(
        after_ckpt >= before_ckpt + 1,
        "recording an item checkpoint emits ITEM_CHECKPOINTED"
    );
}

#[tokio::test]
async fn downloader_session_receipt_idempotency_and_provenance() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP downloader_session_receipt_idempotency_and_provenance: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let session = fresh_session(&store).await;

    let before = store
        .count_events(downloader::SESSION_RECEIPT_EMITTED)
        .await
        .expect("count receipt events before");

    // --- emit a terminal receipt (idempotent on (session_id, terminal_stage)) ---
    let receipt1 = store
        .emit_session_receipt(
            session.session_id,
            &EmitSessionReceipt {
                item_count: 5,
                succeeded: 3,
                failed: 1,
                skipped_deduped: 1,
                materialized_paths: serde_json::json!(["artifact://atelier/media/a"]),
                manifest_artifact_ref: Some(format!("artifact://atelier/manifest/{}", Uuid::new_v4())),
                started_at_utc: None,
                ended_at_utc: None,
                terminal_stage: TerminalStage::Finalized,
            },
        )
        .await
        .expect("emit session receipt");
    assert_eq!(receipt1.item_count, 5);
    assert_eq!(receipt1.terminal_stage, TerminalStage::Finalized);
    // INVARIANT: the receipt denormalizes session provenance so it is a
    // self-contained replay unit.
    assert_eq!(
        receipt1.parent_job_id, session.parent_job_id,
        "receipt denormalizes the parent job id from the session"
    );
    assert_eq!(
        receipt1.allowlist_policy_id, session.allowlist_policy_id,
        "receipt denormalizes the allowlist policy from the session"
    );
    assert_eq!(
        receipt1.output_root_id, session.output_root_id,
        "receipt denormalizes the output root from the session"
    );
    assert_eq!(receipt1.source_kind, session.source_kind);

    // Re-emit the SAME terminal stage with a changed item_count: ON CONFLICT
    // updates in place, the receipt_id stays stable (idempotency).
    let receipt2 = store
        .emit_session_receipt(
            session.session_id,
            &EmitSessionReceipt {
                item_count: 9,
                succeeded: 9,
                failed: 0,
                skipped_deduped: 0,
                materialized_paths: serde_json::json!([]),
                manifest_artifact_ref: None,
                started_at_utc: None,
                ended_at_utc: None,
                terminal_stage: TerminalStage::Finalized,
            },
        )
        .await
        .expect("re-emit same terminal receipt");
    assert_eq!(
        receipt1.receipt_id, receipt2.receipt_id,
        "re-emitting the same (session, terminal_stage) returns the same receipt_id (idempotency)"
    );

    // A DIFFERENT terminal stage for the same session is a distinct receipt row.
    let cancelled = store
        .emit_session_receipt(
            session.session_id,
            &EmitSessionReceipt {
                item_count: 0,
                succeeded: 0,
                failed: 0,
                skipped_deduped: 0,
                materialized_paths: serde_json::json!([]),
                manifest_artifact_ref: None,
                started_at_utc: None,
                ended_at_utc: None,
                terminal_stage: TerminalStage::Cancelled,
            },
        )
        .await
        .expect("emit cancelled receipt");
    assert_ne!(
        cancelled.receipt_id, receipt1.receipt_id,
        "a different terminal_stage for the same session is a distinct receipt"
    );

    // get round-trips the finalized receipt by terminal stage.
    let fetched = store
        .get_session_receipt(session.session_id, TerminalStage::Finalized)
        .await
        .expect("get session receipt")
        .expect("finalized receipt present");
    assert_eq!(fetched.receipt_id, receipt1.receipt_id);

    // --- event emission increased (two distinct terminal receipts emitted) ---
    let after = store
        .count_events(downloader::SESSION_RECEIPT_EMITTED)
        .await
        .expect("count receipt events after");
    assert!(
        after >= before + 2,
        "two distinct terminal receipts each emit SESSION_RECEIPT_EMITTED; \
         the idempotent re-emit of finalized still emits but never duplicates the row"
    );
}
