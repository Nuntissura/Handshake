//! WP-KERNEL-005 atelier Sourcing-Spec + Handler Version Matrix: live
//! PostgreSQL round-trip proofs for the spec-to-handler binding pipeline
//! (sourcing.rs, MT-201 / MT-005). Run with a live DATABASE_URL, e.g.
//!   DATABASE_URL=postgres://postgres@127.0.0.1:5544/handshake \
//!     cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --test atelier_sourcing_tests -- --nocapture
//!
//! No mocks: each test connects the real `AtelierStore` to a live Postgres,
//! ensures the schema, registers real sourcing specs / publishes real handler
//! matrix entries, exercises the binding + ingestion methods, and asserts the
//! load-bearing invariants: spec_hash idempotency, matrix immutability, the
//! bound-vs-mismatch binding decision, the version-mismatch receipt on a
//! non-binding decision, ingestion fresh-vs-deduped idempotency, secret
//! redaction in the stored spec, and event emission via `count_events`.
//! Tables persist between runs, so spec ids / pins / handler families are made
//! unique per run via `Uuid::new_v4()`. Only `handshake_core` + `tokio` +
//! `uuid` + `serde_json` (+ std) are used; sqlx is never imported directly.

use handshake_core::atelier::sourcing::{
    sourcing_event_family, HandlerStatus, IdempotencyClass, IngestionOutcome, MismatchReason,
    NewHandlerVersionEntry, NewSourcingSpec, SideEffect,
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

/// Register a run-unique sourcing spec pinned to `pin` for the given family.
/// `params` is the spec body params object (may carry secret-bearing keys to
/// prove redaction).
async fn register_spec(
    store: &AtelierStore,
    handler_family: &str,
    pin: &str,
    schema_version: &str,
    params: serde_json::Value,
    required_capabilities: Vec<String>,
) -> handshake_core::atelier::sourcing::SourcingSpecRecord {
    store
        .register_sourcing_spec(&NewSourcingSpec {
            sourcing_spec_id: format!("spec-{}", Uuid::new_v4()),
            schema_version: schema_version.to_string(),
            source_kind: "media_url".to_string(),
            source_ref: format!("artifact://atelier/source/{}", Uuid::new_v4()),
            handler_family: handler_family.to_string(),
            handler_version_pin: pin.to_string(),
            params_json: params,
            required_capabilities,
            idempotency_key: None,
        })
        .await
        .expect("register sourcing spec")
}

#[tokio::test]
async fn atelier_sourcing_rejects_legacy_runtime_source_refs() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_sourcing_rejects_legacy_runtime_source_refs: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let err = store
        .register_sourcing_spec(&NewSourcingSpec {
            sourcing_spec_id: format!("spec-{}", Uuid::new_v4()),
            schema_version: "1.2.0".to_string(),
            source_kind: "media_url".to_string(),
            source_ref: "http://localhost:9000/source.json".to_string(),
            handler_family: format!("media_downloader-{}", Uuid::new_v4()),
            handler_version_pin: "^1.0.0".to_string(),
            params_json: serde_json::json!({ "params": { "url": "https://example.invalid/clip.mp4" } }),
            required_capabilities: vec!["net.fetch".to_string()],
            idempotency_key: None,
        })
        .await;
    assert!(err.is_err(), "localhost sourcing refs must be rejected");
}

#[tokio::test]
async fn atelier_sourcing_spec_idempotency_and_secret_redaction() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_sourcing_spec_idempotency_and_secret_redaction: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let family = format!("media_downloader-{}", Uuid::new_v4());
    let before = store
        .count_events(sourcing_event_family::SOURCING_SPEC_REGISTERED)
        .await
        .expect("count spec-registered events before");

    // --- register a spec whose params carry a secret-bearing key ---
    let params = serde_json::json!({
        "params": {
            "url": "https://example.invalid/clip.mp4",
            "auth_token": "super-secret-token-value",
            "nested": { "cookie": "sid=abc123" }
        }
    });
    let spec = register_spec(
        &store,
        &family,
        "^1.0.0",
        "1.2.0",
        params.clone(),
        vec!["net.fetch".to_string()],
    )
    .await;

    // --- INVARIANT: secret values are redacted in the stored record ---
    let stored = serde_json::to_string(&spec.params_json).expect("serialize stored params");
    assert!(
        !stored.contains("super-secret-token-value"),
        "auth_token value must be redacted in the stored spec params"
    );
    assert!(
        !stored.contains("sid=abc123"),
        "nested cookie value must be redacted in the stored spec params"
    );
    assert!(
        stored.contains("[REDACTED]"),
        "redacted secret values are replaced by the [REDACTED] placeholder"
    );
    // Non-secret values survive untouched.
    assert!(
        stored.contains("https://example.invalid/clip.mp4"),
        "non-secret params values are preserved"
    );

    // --- round-trip: fetch by canonical spec_hash returns the same record ---
    let fetched = store
        .get_sourcing_spec_by_hash(&spec.spec_hash)
        .await
        .expect("get spec by hash")
        .expect("spec present by hash");
    assert_eq!(
        fetched.record_id, spec.record_id,
        "fetch by spec_hash returns the registered record"
    );
    assert_eq!(fetched.handler_family, family);
    assert_eq!(fetched.handler_version_pin, "^1.0.0");

    // --- IDEMPOTENCY: re-registering the same canonical spec returns the same
    // record_id (idempotent on spec_hash), even with a different spec id ---
    let again = store
        .register_sourcing_spec(&NewSourcingSpec {
            sourcing_spec_id: spec.sourcing_spec_id.clone(),
            schema_version: "1.2.0".to_string(),
            source_kind: "media_url".to_string(),
            source_ref: fetched.source_ref.clone(),
            handler_family: family.clone(),
            handler_version_pin: "^1.0.0".to_string(),
            params_json: params,
            required_capabilities: vec!["net.fetch".to_string()],
            idempotency_key: None,
        })
        .await
        .expect("re-register identical canonical spec");
    assert_eq!(
        again.record_id, spec.record_id,
        "re-registering the same canonical spec must not duplicate the record"
    );
    assert_eq!(
        again.spec_hash, spec.spec_hash,
        "canonical spec_hash is stable across identical registrations"
    );

    // --- EVENT EMISSION: at least one spec_registered event was recorded ---
    let after = store
        .count_events(sourcing_event_family::SOURCING_SPEC_REGISTERED)
        .await
        .expect("count spec-registered events after");
    assert!(
        after > before,
        "registering a fresh spec must emit a SOURCING_SPEC_REGISTERED event"
    );
}

#[tokio::test]
async fn atelier_sourcing_yaml_authoring_validates_schema_and_canonicalizes() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_sourcing_yaml_authoring_validates_schema_and_canonicalizes: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    let sourcing_spec_id = format!("spec-{}", Uuid::new_v4());
    let source_ref = format!("artifact://atelier/source/{}", Uuid::new_v4());
    let family = format!("yaml-media-downloader-{}", Uuid::new_v4());
    let valid_yaml = format!(
        r#"sourcing_spec_id: "{sourcing_spec_id}"
schema_version: "1.2.0"
source:
  kind: media_url
  ref: "{source_ref}"
handler_family: "{family}"
handler_version_pin: "^1.0.0"
params:
  url: "https://example.invalid/clip.mp4"
  auth_token: "super-secret-token-value"
  nested:
    cookie: "sid=abc123"
required_capabilities:
  - net.fetch
idempotency_key: "ingest-A"
"#
    );

    let record = store
        .register_sourcing_spec_yaml(&valid_yaml)
        .await
        .expect("register sourcing spec from YAML");

    assert_eq!(record.sourcing_spec_id, sourcing_spec_id);
    assert_eq!(record.source_kind, "media_url");
    assert_eq!(record.source_ref, source_ref);
    assert_eq!(record.handler_family, family);
    assert_eq!(record.handler_version_pin, "^1.0.0");
    assert_eq!(
        record.required_capabilities,
        vec!["net.fetch".to_string()],
        "required capabilities round-trip from YAML as an array"
    );
    assert_eq!(record.idempotency_key.as_deref(), Some("ingest-A"));
    assert_eq!(
        record.spec_hash.len(),
        64,
        "YAML registration records the canonical SHA-256 spec identity"
    );
    assert!(
        record.spec_hash.chars().all(|c| c.is_ascii_hexdigit()),
        "spec_hash is stored as hex SHA-256"
    );

    let stored = serde_json::to_string(&record.params_json).expect("serialize stored params");
    assert!(
        !stored.contains("super-secret-token-value") && !stored.contains("sid=abc123"),
        "secret-bearing YAML params must be redacted before storage"
    );
    assert!(
        stored.contains("[REDACTED]") && stored.contains("https://example.invalid/clip.mp4"),
        "redacted secret values and non-secret params are both visible in storage"
    );

    let same_spec_different_order = format!(
        r#"handler_version_pin: "^1.0.0"
handler_family: "{family}"
required_capabilities:
  - net.fetch
params:
  nested:
    cookie: "sid=abc123"
  auth_token: "super-secret-token-value"
  url: "https://example.invalid/clip.mp4"
source:
  ref: "{source_ref}"
  kind: media_url
schema_version: "1.2.0"
sourcing_spec_id: "{sourcing_spec_id}"
idempotency_key: "ingest-A"
"#
    );
    let again = store
        .register_sourcing_spec_yaml(&same_spec_different_order)
        .await
        .expect("re-register equivalent YAML");
    assert_eq!(
        again.record_id, record.record_id,
        "equivalent YAML with different key order must dedupe to the canonical record"
    );
    assert_eq!(again.spec_hash, record.spec_hash);

    let event_count = store
        .count_events_for_aggregate(
            sourcing_event_family::SOURCING_SPEC_REGISTERED,
            "atelier_sourcing_spec",
            &record.spec_hash,
        )
        .await
        .expect("count YAML spec event for aggregate");
    assert_eq!(
        event_count, 1,
        "only the fresh canonical YAML spec registration emits an event"
    );

    let malformed_yaml = format!(
        r#"sourcing_spec_id: "spec-{}"
schema_version: "1.2.0"
source:
  kind: media_url
  ref: "artifact://atelier/source/{}"
handler_family: "{family}"
handler_version_pin: "^1.0.0"
params:
  url: "https://example.invalid/clip.mp4"
required_capabilities: net.fetch
"#,
        Uuid::new_v4(),
        Uuid::new_v4()
    );
    assert!(
        store
            .register_sourcing_spec_yaml(&malformed_yaml)
            .await
            .is_err(),
        "required_capabilities must be rejected when YAML supplies a scalar instead of an array"
    );

    let unknown_field_yaml = format!(
        r#"sourcing_spec_id: "spec-{}"
schema_version: "1.2.0"
source:
  kind: media_url
  ref: "artifact://atelier/source/{}"
handler_family: "{family}"
handler_version_pin: "^1.0.0"
params:
  url: "https://example.invalid/clip.mp4"
required_capabilities:
  - net.fetch
surprise: true
"#,
        Uuid::new_v4(),
        Uuid::new_v4()
    );
    assert!(
        store
            .register_sourcing_spec_yaml(&unknown_field_yaml)
            .await
            .is_err(),
        "unknown top-level YAML fields must be rejected by the JSON Schema"
    );
}

#[tokio::test]
async fn atelier_handler_matrix_immutability_and_event() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_handler_matrix_immutability_and_event: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let family = format!("asr-{}", Uuid::new_v4());

    // --- publish a matrix entry ---
    let entry = store
        .publish_handler_version(&NewHandlerVersionEntry {
            handler_family: family.clone(),
            handler_version: "1.4.0".to_string(),
            schema_version_min: "1.0.0".to_string(),
            schema_version_max: "1.9.9".to_string(),
            side_effect: SideEffect::Write,
            idempotency: IdempotencyClass::IdempotentWithKey,
            required_capabilities: vec!["asr.transcribe".to_string()],
            determinism: "D2".to_string(),
            status: HandlerStatus::Active,
            job_profile_ref: format!("job://asr/{}", Uuid::new_v4()),
        })
        .await
        .expect("publish handler version");
    assert_eq!(entry.side_effect.as_token(), "WRITE");
    assert_eq!(entry.idempotency.as_token(), "IDEMPOTENT_WITH_KEY");
    assert_eq!(entry.status.as_token(), "ACTIVE");

    // --- round-trip fetch ---
    let fetched = store
        .get_handler_version(&family, "1.4.0")
        .await
        .expect("get handler version")
        .expect("handler version present");
    assert_eq!(fetched.entry_id, entry.entry_id);

    // --- IDEMPOTENCY / IMMUTABILITY: re-publishing the same (family, version)
    // with DIFFERENT attributes returns the original entry unchanged ---
    let republished = store
        .publish_handler_version(&NewHandlerVersionEntry {
            handler_family: family.clone(),
            handler_version: "1.4.0".to_string(),
            schema_version_min: "2.0.0".to_string(),
            schema_version_max: "2.9.9".to_string(),
            side_effect: SideEffect::Execute,
            idempotency: IdempotencyClass::NonIdempotent,
            required_capabilities: vec!["danger.exec".to_string()],
            determinism: "D0".to_string(),
            status: HandlerStatus::Sunset,
            job_profile_ref: "job://hijack".to_string(),
        })
        .await
        .expect("re-publish same (family, version)");
    assert_eq!(
        republished.entry_id, entry.entry_id,
        "re-publishing the same (family, version) returns the existing entry"
    );
    assert_eq!(
        republished.side_effect,
        SideEffect::Write,
        "matrix entries are immutable: side_effect must NOT change on re-publish"
    );
    assert_eq!(
        republished.status,
        HandlerStatus::Active,
        "matrix entries are immutable: status must NOT change on re-publish"
    );
    assert_eq!(
        republished.schema_version_min, "1.0.0",
        "matrix entries are immutable: schema bounds must NOT change on re-publish"
    );
    let bad_republish = store
        .publish_handler_version(&NewHandlerVersionEntry {
            handler_family: family.clone(),
            handler_version: "1.4.0".to_string(),
            schema_version_min: "1.0.0".to_string(),
            schema_version_max: "1.9.9".to_string(),
            side_effect: SideEffect::Write,
            idempotency: IdempotencyClass::IdempotentWithKey,
            required_capabilities: vec!["asr.transcribe".to_string()],
            determinism: "D2".to_string(),
            status: HandlerStatus::Active,
            job_profile_ref: "electron://handler/profile".to_string(),
        })
        .await;
    assert!(
        bad_republish.is_err(),
        "legacy runtime job_profile_ref must be rejected before immutable-entry fast path"
    );

    let bad_fresh = store
        .publish_handler_version(&NewHandlerVersionEntry {
            handler_family: format!("asr-{}", Uuid::new_v4()),
            handler_version: "1.4.0".to_string(),
            schema_version_min: "1.0.0".to_string(),
            schema_version_max: "1.9.9".to_string(),
            side_effect: SideEffect::Write,
            idempotency: IdempotencyClass::IdempotentWithKey,
            required_capabilities: vec!["asr.transcribe".to_string()],
            determinism: "D2".to_string(),
            status: HandlerStatus::Active,
            job_profile_ref: "localhost:9000/profile".to_string(),
        })
        .await;
    assert!(
        bad_fresh.is_err(),
        "bare localhost job_profile_ref must be rejected on fresh publish"
    );

    // --- EVENT EMISSION: published exactly one NEW entry (re-publish is silent) ---
    let published = store
        .count_events_for_aggregate(
            sourcing_event_family::HANDLER_MATRIX_ENTRY_PUBLISHED,
            "atelier_handler_version_matrix",
            &entry.entry_id.to_string(),
        )
        .await
        .expect("count matrix-published events for entry");
    assert_eq!(
        published, 1,
        "only the genuinely new matrix entry emits an event; re-publish is a silent no-op"
    );
}

#[tokio::test]
async fn atelier_binding_bound_path_and_idempotent_ingestion() {
    let Some(url) = database_url() else {
        eprintln!("SKIP atelier_binding_bound_path_and_idempotent_ingestion: DATABASE_URL not set");
        return;
    };
    let store = connected_store(&url).await;

    let family = format!("export-{}", Uuid::new_v4());
    // Publish an ACTIVE handler version 2.1.0 supporting schema [2.0.0, 2.9.9].
    store
        .publish_handler_version(&NewHandlerVersionEntry {
            handler_family: family.clone(),
            handler_version: "2.1.0".to_string(),
            schema_version_min: "2.0.0".to_string(),
            schema_version_max: "2.9.9".to_string(),
            side_effect: SideEffect::Read,
            idempotency: IdempotencyClass::Idempotent,
            required_capabilities: vec!["export.write".to_string()],
            determinism: "D1".to_string(),
            status: HandlerStatus::Active,
            job_profile_ref: format!("job://export/{}", Uuid::new_v4()),
        })
        .await
        .expect("publish bindable handler version");

    // Spec pins ^2.0.0, schema 2.3.0 -> falls inside the matrix range.
    let spec = register_spec(
        &store,
        &family,
        "^2.0.0",
        "2.3.0",
        serde_json::json!({ "params": { "format": "markdown" } }),
        vec!["export.spec".to_string()],
    )
    .await;

    let bind_before = store
        .count_events(sourcing_event_family::SOURCING_BINDING_DECIDED)
        .await
        .expect("count binding events before");

    // --- BOUND decision: grant the union of spec + handler capabilities ---
    let snapshot = Uuid::new_v4();
    let granted = vec!["export.spec".to_string(), "export.write".to_string()];
    let decision = store
        .decide_binding(&spec.spec_hash, &granted, snapshot)
        .await
        .expect("decide binding");

    // --- INVARIANT: a satisfiable pin binds to the concrete handler version ---
    assert!(decision.bound, "spec pin ^2.0.0 must bind to handler 2.1.0");
    assert_eq!(
        decision.resolved_handler_version.as_deref(),
        Some("2.1.0"),
        "binding resolves to the published ACTIVE version satisfying the pin"
    );
    assert!(
        decision.capability_satisfied,
        "the granted capability union satisfies spec + handler requirements"
    );
    assert_eq!(
        decision.matrix_snapshot_id, snapshot,
        "the binding records the matrix snapshot it resolved against (replay)"
    );

    // --- round-trip the decision by id ---
    let fetched_decision = store
        .get_binding_decision(decision.decision_id)
        .await
        .expect("get binding decision");
    assert_eq!(fetched_decision.decision_id, decision.decision_id);
    assert!(fetched_decision.bound);

    // A bound decision has NO version-mismatch receipt.
    let no_receipt = store
        .get_version_mismatch_receipt(decision.decision_id)
        .await
        .expect("query mismatch receipt for bound decision");
    assert!(
        no_receipt.is_none(),
        "a bound decision must not produce a version-mismatch receipt"
    );

    let bind_after = store
        .count_events(sourcing_event_family::SOURCING_BINDING_DECIDED)
        .await
        .expect("count binding events after");
    assert!(
        bind_after > bind_before,
        "a binding decision emits a SOURCING_BINDING_DECIDED event"
    );

    // --- idempotent ingestion against the bound decision ---
    let ing_before = store
        .count_events(sourcing_event_family::SOURCING_INGESTION_RECEIPTED)
        .await
        .expect("count ingestion events before");
    let refs = vec![format!("artifact://atelier/out/{}", Uuid::new_v4())];

    let fresh = store
        .record_ingestion_receipt(decision.decision_id, Some("batch-A"), &refs, 1, 0)
        .await
        .expect("record fresh ingestion receipt");
    assert_eq!(
        fresh.outcome,
        IngestionOutcome::Fresh,
        "a first ingestion under this identity is fresh"
    );
    assert_eq!(fresh.handler_version, "2.1.0");
    assert_eq!(
        fresh.artifact_manifest_refs, refs,
        "manifest refs round-trip"
    );
    let bad_replay_refs = vec!["artifact://atelier/.GOV/out".to_string()];
    let bad_replay = store
        .record_ingestion_receipt(
            decision.decision_id,
            Some("batch-A"),
            &bad_replay_refs,
            1,
            0,
        )
        .await;
    assert!(
        bad_replay.is_err(),
        ".GOV replay refs must be rejected before ingestion-key dedupe fast path"
    );
    let bad_manifest = vec!["artifact://atelier/.GOV/out".to_string()];
    let bad_manifest_err = store
        .record_ingestion_receipt(decision.decision_id, Some("batch-B"), &bad_manifest, 1, 0)
        .await;
    assert!(
        bad_manifest_err.is_err(),
        ".GOV ingestion artifact manifest refs must be rejected"
    );

    // --- IDEMPOTENCY: a repeat under the same identity dedupes to the prior
    // receipt (same receipt_id, outcome flips to deduped) ---
    let deduped = store
        .record_ingestion_receipt(decision.decision_id, Some("batch-A"), &refs, 1, 0)
        .await
        .expect("re-record ingestion receipt");
    assert_eq!(
        deduped.receipt_id, fresh.receipt_id,
        "a repeat ingestion under the same identity returns the prior receipt id"
    );
    assert_eq!(
        deduped.outcome,
        IngestionOutcome::Deduped,
        "the repeat ingestion is reported as deduped, not a new side-effecting run"
    );

    let ing_after = store
        .count_events(sourcing_event_family::SOURCING_INGESTION_RECEIPTED)
        .await
        .expect("count ingestion events after");
    assert!(
        ing_after >= ing_before + 2,
        "both the fresh and the deduped ingestion emit a SOURCING_INGESTION_RECEIPTED event"
    );
}

#[tokio::test]
async fn atelier_binding_mismatch_produces_receipt_and_blocks_ingestion() {
    let Some(url) = database_url() else {
        eprintln!(
            "SKIP atelier_binding_mismatch_produces_receipt_and_blocks_ingestion: DATABASE_URL not set"
        );
        return;
    };
    let store = connected_store(&url).await;

    // A family with a published 1.x handler, but the spec pins ^3.0.0 -> no
    // version satisfies the pin -> NoMatchingVersion rejection.
    let family = format!("external_tool-{}", Uuid::new_v4());
    store
        .publish_handler_version(&NewHandlerVersionEntry {
            handler_family: family.clone(),
            handler_version: "1.0.0".to_string(),
            schema_version_min: "1.0.0".to_string(),
            schema_version_max: "1.9.9".to_string(),
            side_effect: SideEffect::Execute,
            idempotency: IdempotencyClass::NonIdempotent,
            required_capabilities: vec![],
            determinism: "D3".to_string(),
            status: HandlerStatus::Active,
            job_profile_ref: format!("job://tool/{}", Uuid::new_v4()),
        })
        .await
        .expect("publish 1.x handler version");

    let spec = register_spec(
        &store,
        &family,
        "^3.0.0",
        "1.2.0",
        serde_json::json!({ "params": { "cmd": "noop" } }),
        vec![],
    )
    .await;

    let reject_before = store
        .count_events(sourcing_event_family::VERSION_MISMATCH_REJECTED)
        .await
        .expect("count rejection events before");

    let snapshot = Uuid::new_v4();
    let decision = store
        .decide_binding(&spec.spec_hash, &[], snapshot)
        .await
        .expect("decide binding (mismatch path)");

    // --- INVARIANT: the unsatisfiable pin does NOT bind ---
    assert!(
        !decision.bound,
        "pin ^3.0.0 cannot bind when only handler 1.0.0 exists"
    );
    assert!(
        decision.resolved_handler_version.is_none(),
        "a non-binding decision resolves no handler version"
    );

    // --- INVARIANT: a non-binding decision carries a version-mismatch receipt
    // with a machine-readable reason (first-class evidence, not a swallowed error) ---
    let receipt = store
        .get_version_mismatch_receipt(decision.decision_id)
        .await
        .expect("query mismatch receipt")
        .expect("non-binding decision must have a mismatch receipt");
    assert_eq!(receipt.decision_id, decision.decision_id);
    assert_eq!(
        receipt.reason,
        MismatchReason::NoMatchingVersion,
        "no handler version satisfied the pin -> no_matching_version"
    );
    assert_eq!(receipt.requested_pin, "^3.0.0");
    assert!(
        receipt.evaluated_versions.contains(&"1.0.0".to_string()),
        "the receipt records the candidate versions it evaluated"
    );
    assert_eq!(
        receipt.matrix_snapshot_id, snapshot,
        "the receipt pins the matrix snapshot for replay"
    );

    // --- INVARIANT: ingestion against a non-binding decision is rejected ---
    let blocked = store
        .record_ingestion_receipt(decision.decision_id, None, &[], 0, 0)
        .await;
    assert!(
        blocked.is_err(),
        "a rejected spec must never ingest: recording against a non-binding decision errors"
    );

    // --- EVENT EMISSION: the rejection emitted a VERSION_MISMATCH_REJECTED event ---
    let reject_after = store
        .count_events(sourcing_event_family::VERSION_MISMATCH_REJECTED)
        .await
        .expect("count rejection events after");
    assert!(
        reject_after > reject_before,
        "a version-mismatch rejection emits a VERSION_MISMATCH_REJECTED event"
    );
}
