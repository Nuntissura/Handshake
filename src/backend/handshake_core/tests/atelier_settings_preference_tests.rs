//! WP-CKC MT-042 — Atelier/CKC/PoseKit/Ingest operator-default preference proofs.
//!
//! These prove the new operator-facing default preferences round-trip through the
//! EXISTING atelier preference store on real PostgreSQL + EventLedger:
//!   * `set_preference` stores the value, `get_effective_preference` reads it back;
//!   * the `atelier.preference.set` EventLedger row carries the change;
//!   * `reset_preference_to_default` restores the registered default and flips the
//!     source back to `Default`;
//!   * enumerated keys reject out-of-vocabulary values (fail-closed).
//!
//! This is a NEW clean test binary (it does NOT extend `atelier_core_data_tests.rs`,
//! which carries an unrelated in-flight broken test that would fail the whole binary).
//! It SKIPs when PostgreSQL is unavailable, using the shared `atelier_pg_support`
//! managed-Postgres helper (no SQLite, no mock authority).

mod atelier_pg_support;

use handshake_core::atelier::settings::{
    PreferenceScope, PreferenceType, PreferenceValueSource, SetPreference,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use sha2::{Digest, Sha256};

const PREFERENCE_SET_EVENT_FAMILY: &str = "atelier.preference.set";

/// Connect + ensure schema against the shared managed Postgres.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Every WP-CKC MT-042 default: (key, value_type, sample operator value, registry default).
/// Sample values are all in-vocabulary / correctly typed so `set` succeeds.
fn mt042_preferences() -> Vec<(&'static str, PreferenceType, &'static str, &'static str)> {
    vec![
        (
            "atelier-ui.landing-tab",
            PreferenceType::String,
            "posekit",
            "castkit-codex",
        ),
        ("ckc.book-mode", PreferenceType::String, "story", "sheet"),
        (
            "posekit.framing-preset",
            PreferenceType::String,
            "portrait",
            "standard",
        ),
        (
            "posekit.framing-lens-mm",
            PreferenceType::Integer,
            "85",
            "50",
        ),
        (
            "posekit.framing-padding-top-px",
            PreferenceType::Integer,
            "12",
            "0",
        ),
        (
            "posekit.framing-padding-right-px",
            PreferenceType::Integer,
            "8",
            "0",
        ),
        (
            "posekit.framing-padding-bottom-px",
            PreferenceType::Integer,
            "16",
            "0",
        ),
        (
            "posekit.framing-padding-left-px",
            PreferenceType::Integer,
            "4",
            "0",
        ),
        ("posekit.marker-face", PreferenceType::Bool, "false", "true"),
        ("posekit.marker-body", PreferenceType::Bool, "false", "true"),
        ("posekit.marker-hands", PreferenceType::Bool, "true", "false"),
        (
            "ingest.batch-tags",
            PreferenceType::String,
            "studio, set-a",
            "event, outfit, source",
        ),
        (
            "ingest.default-policy",
            PreferenceType::String,
            "pass",
            "unsure",
        ),
    ]
}

#[tokio::test]
async fn mt042_defaults_set_get_event_and_reset_round_trip() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt042_defaults_set_get_event_and_reset_round_trip: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let scope = PreferenceScope::Global;

    for (key, value_type, sample, default_value) in mt042_preferences() {
        // --- set stores the operator value + emits atelier.preference.set ---
        let set_receipt = store
            .set_preference_with_receipt(&SetPreference {
                scope,
                key: key.to_string(),
                value_type,
                value: sample.to_string(),
                redacted: false,
            })
            .await
            .unwrap_or_else(|err| panic!("set preference {key}: {err}"));
        assert_eq!(
            set_receipt.event_family, PREFERENCE_SET_EVENT_FAMILY,
            "{key} set must emit atelier.preference.set"
        );
        assert_eq!(set_receipt.value_after, sample, "{key} value_after");
        assert_eq!(
            set_receipt.source_after,
            PreferenceValueSource::Operator,
            "{key} source_after"
        );
        assert_eq!(
            set_receipt.preference.default_value.as_deref(),
            Some(default_value),
            "{key} registered default_value"
        );

        // --- get_effective_preference returns the operator value ---
        let effective = store
            .get_effective_preference(scope, key)
            .await
            .unwrap_or_else(|err| panic!("get effective {key}: {err}"));
        assert_eq!(effective.value, sample, "{key} effective value");
        assert_eq!(
            effective.source,
            PreferenceValueSource::Operator,
            "{key} effective source"
        );

        // --- the EventLedger row carries the change ---
        // Bind to THIS set's own receipt_id so the assertion is deterministic on a
        // shared managed-PostgreSQL DB: `receipt_id` is a fresh Uuid::now_v7 per set, so
        // (family, aggregate_id, receipt_id) targets exactly the event this set produced —
        // never a concurrent/prior run's event on the same Global-scope row (test-isolation,
        // not a product bug). `receipt_id` is not a redacted field, so it survives the
        // event-payload sanitizer verbatim and is safe to match on.
        let payload: serde_json::Value = sqlx::query_scalar(
            r#"SELECT payload
               FROM atelier_event
               WHERE event_family = $1
                 AND aggregate_type = 'atelier_preference'
                 AND aggregate_id = $2
                 AND payload->>'receipt_id' = $3
               LIMIT 1"#,
        )
        .bind(PREFERENCE_SET_EVENT_FAMILY)
        .bind(set_receipt.preference.preference_id.to_string())
        .bind(set_receipt.receipt_id.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap_or_else(|err| panic!("load set event for {key}: {err}"));
        assert_eq!(payload["key"], serde_json::json!(key), "{key} event key");

        // The atelier EventLedger intentionally REDACTS every preference value field
        // (value / value_before / value_after / default_value) into a sha256 ref
        // before persisting the domain-event payload — see
        // `sanitize_atelier_event_payload` + `sensitive_event_replacement_key` in
        // `atelier/mod.rs` ("never leak secret values into the ledger"). So the row
        // carries `value_after_ref = sha256:<hex>` of the value, NOT a plaintext
        // `value_after`. Prove THIS set's value reached the ledger by matching the
        // redacted ref against sha256(sample) — equivalent to the product's
        // `event_ref_for_text(sample)`.
        let expected_value_after_ref =
            format!("sha256:{}", hex::encode(Sha256::digest(sample.as_bytes())));
        assert_eq!(
            payload["value_after_ref"],
            serde_json::json!(expected_value_after_ref),
            "{key} event value_after_ref"
        );

        // --- reset restores the registered default and flips source to Default ---
        let reset_receipt = store
            .reset_preference_to_default(scope, key)
            .await
            .unwrap_or_else(|err| panic!("reset preference {key}: {err}"));
        assert_eq!(
            reset_receipt.event_family, "atelier.preference.reset_to_default",
            "{key} reset event family"
        );
        assert_eq!(reset_receipt.value_after, default_value, "{key} reset value");
        assert_eq!(
            reset_receipt.source_after,
            PreferenceValueSource::Default,
            "{key} reset source"
        );

        let effective_after_reset = store
            .get_effective_preference(scope, key)
            .await
            .unwrap_or_else(|err| panic!("get effective after reset {key}: {err}"));
        assert_eq!(
            effective_after_reset.value, default_value,
            "{key} effective after reset"
        );
        assert_eq!(
            effective_after_reset.source,
            PreferenceValueSource::Default,
            "{key} effective source after reset"
        );
    }
}

#[tokio::test]
async fn mt042_enumerated_defaults_reject_out_of_vocabulary_values() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP mt042_enumerated_defaults_reject_out_of_vocabulary_values: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;
    let scope = PreferenceScope::Global;

    // Enumerated keys are fail-closed: an unknown token is a Validation error, never stored.
    let rejects = [
        ("ingest.default-policy", "maybe"),
        ("ckc.book-mode", "timeline"),
        ("posekit.framing-preset", "fisheye"),
        ("atelier-ui.landing-tab", "moodboard"),
    ];
    for (key, bad_value) in rejects {
        let result = store
            .set_preference_with_receipt(&SetPreference {
                scope,
                key: key.to_string(),
                value_type: PreferenceType::String,
                value: bad_value.to_string(),
                redacted: false,
            })
            .await;
        assert!(
            matches!(result, Err(AtelierError::Validation(_))),
            "{key} must reject out-of-vocabulary value {bad_value:?}, got {result:?}"
        );
    }

    // An unknown namespace is likewise rejected by the store's key validation.
    let unknown_namespace = store
        .set_preference_with_receipt(&SetPreference {
            scope,
            key: "not-a-namespace.some-key".to_string(),
            value_type: PreferenceType::String,
            value: "x".to_string(),
            redacted: false,
        })
        .await;
    assert!(
        matches!(unknown_namespace, Err(AtelierError::Validation(_))),
        "unknown namespace must be rejected, got {unknown_namespace:?}"
    );
}

#[tokio::test]
async fn mt042_actor_attribution_is_recorded_on_set_and_reset() {
    // WP-CKC MT-042 (F4): the `_as` write path records WHO changed the default on the
    // row (`updated_by`) and in the EventLedger payload; reset records `reset_by`.
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!("SKIP mt042_actor_attribution_is_recorded_on_set_and_reset: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let scope = PreferenceScope::Global;
    let key = "posekit.framing-preset";
    let actor = "argus-agent-042";

    let set_receipt = store
        .set_preference_with_receipt_as(
            &SetPreference {
                scope,
                key: key.to_string(),
                value_type: PreferenceType::String,
                value: "portrait".to_string(),
                redacted: false,
            },
            Some(actor),
        )
        .await
        .expect("set preference with actor");
    assert_eq!(
        set_receipt.preference.updated_by.as_deref(),
        Some(actor),
        "row updated_by must record the actor"
    );

    // Bind to this set's own revision_after for deterministic isolation on a shared DB
    // (a concurrent test also sets posekit.framing-preset on the same row).
    let set_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = $1
             AND aggregate_type = 'atelier_preference'
             AND aggregate_id = $2
             AND (payload->>'revision_after')::bigint = $3
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(PREFERENCE_SET_EVENT_FAMILY)
    .bind(set_receipt.preference.preference_id.to_string())
    .bind(set_receipt.revision_after)
    .fetch_one(store.pool())
    .await
    .expect("load set event payload");
    assert_eq!(
        set_payload["updated_by"],
        serde_json::json!(actor),
        "set event payload must record who changed the default"
    );

    // Reset records reset_by and returns the row to the NULL-attribution default.
    let reset_receipt = store
        .reset_preference_to_default_as(scope, key, Some(actor))
        .await
        .expect("reset preference with actor");
    assert_eq!(
        reset_receipt.preference.updated_by, None,
        "reset returns the row to the default (no operator override)"
    );
    let reset_payload: serde_json::Value = sqlx::query_scalar(
        r#"SELECT payload
           FROM atelier_event
           WHERE event_family = 'atelier.preference.reset_to_default'
             AND aggregate_type = 'atelier_preference'
             AND aggregate_id = $1
             AND (payload->>'revision_after')::bigint = $2
           ORDER BY created_at_utc DESC
           LIMIT 1"#,
    )
    .bind(reset_receipt.preference.preference_id.to_string())
    .bind(reset_receipt.revision_after)
    .fetch_one(store.pool())
    .await
    .expect("load reset event payload");
    assert_eq!(
        reset_payload["reset_by"],
        serde_json::json!(actor),
        "reset event payload must record who performed the reset"
    );
}
