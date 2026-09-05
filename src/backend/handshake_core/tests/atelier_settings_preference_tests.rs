//! WP-CKC MT-042 (SurrealDB port, MT-062) — Atelier/CKC/PoseKit/Ingest operator-default
//! preference proofs on the embedded SurrealDB store + EventLedger.
//!
//! These prove the operator-facing default preferences round-trip through the EXISTING
//! atelier preference store:
//!   * `set_preference_with_receipt` stores the value, `get_effective_preference` reads it back;
//!   * the `atelier.preference.set` EventLedger row carries the change (values redacted to
//!     `sha256:` refs by the ledger sanitizer);
//!   * `reset_preference_to_default` restores the registered default and flips the source back
//!     to `Default`;
//!   * enumerated keys reject out-of-vocabulary values (fail-closed);
//!   * the `_as` write paths record WHO changed / reset the default (row + event payload).
//!
//! Every test runs against its own isolated embedded store; nothing skips.

mod atelier_surreal_support;

use atelier_surreal_support::AtelierSurrealHarness;
use handshake_core::atelier::settings::{
    PreferenceScope, PreferenceType, PreferenceValueSource, SetPreference,
};
use handshake_core::atelier::AtelierError;
use handshake_core::kernel::{KernelEvent, KernelEventType};
use handshake_core::storage::Database;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

const PREFERENCE_SET_EVENT_FAMILY: &str = "atelier.preference.set";
const PREFERENCE_RESET_EVENT_FAMILY: &str = "atelier.preference.reset_to_default";

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
        (
            "posekit.marker-hands",
            PreferenceType::Bool,
            "true",
            "false",
        ),
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

/// The atelier domain events recorded for one preference row, filtered to one event family.
async fn preference_events(
    database: &Arc<dyn Database>,
    preference_id: Uuid,
    event_family: &str,
) -> Vec<KernelEvent> {
    database
        .list_kernel_events_for_aggregate("atelier_preference", &preference_id.to_string())
        .await
        .expect("list atelier_preference EventLedger rows")
        .into_iter()
        .filter(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"] == event_family
        })
        .collect()
}

/// The event payload written by exactly one set/reset receipt (matched on the receipt's own
/// `receipt_id`, which the ledger sanitizer never redacts).
async fn preference_event_payload_for_receipt(
    database: &Arc<dyn Database>,
    preference_id: Uuid,
    event_family: &str,
    receipt_id: Uuid,
) -> serde_json::Value {
    let events = preference_events(database, preference_id, event_family).await;
    events
        .iter()
        .map(|event| event.payload["atelier_payload"].clone())
        .find(|payload| payload["receipt_id"] == serde_json::json!(receipt_id))
        .unwrap_or_else(|| {
            panic!(
                "no {event_family} event carries receipt_id={receipt_id} for preference {preference_id}; \
                 have {} events",
                events.len()
            )
        })
}

fn sha256_ref(text: &str) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(text.as_bytes())))
}

#[tokio::test]
async fn mt042_defaults_set_get_event_and_reset_round_trip() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
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
        // The atelier EventLedger REDACTS every preference value field (value / value_before /
        // value_after / default_value) into a sha256 ref before persisting the domain-event
        // payload (see `sanitize_atelier_event_payload` in `atelier/mod.rs`), so the row carries
        // `value_after_ref = sha256:<hex>` rather than the plaintext value.
        let payload = preference_event_payload_for_receipt(
            &harness.database,
            set_receipt.preference.preference_id,
            PREFERENCE_SET_EVENT_FAMILY,
            set_receipt.receipt_id,
        )
        .await;
        assert_eq!(payload["key"], serde_json::json!(key), "{key} event key");
        assert_eq!(
            payload["value_after_ref"],
            serde_json::json!(sha256_ref(sample)),
            "{key} event value_after_ref"
        );
        assert!(
            payload.get("value_after").is_none(),
            "{key} plaintext value_after must not reach the ledger: {payload}"
        );
        assert_eq!(
            payload["revision_after"],
            serde_json::json!(set_receipt.revision_after),
            "{key} event revision_after"
        );

        // --- reset restores the registered default and flips source to Default ---
        let reset_receipt = store
            .reset_preference_to_default(scope, key)
            .await
            .unwrap_or_else(|err| panic!("reset preference {key}: {err}"));
        assert_eq!(
            reset_receipt.event_family, PREFERENCE_RESET_EVENT_FAMILY,
            "{key} reset event family"
        );
        assert_eq!(
            reset_receipt.value_after, default_value,
            "{key} reset value"
        );
        assert_eq!(
            reset_receipt.source_after,
            PreferenceValueSource::Default,
            "{key} reset source"
        );
        assert!(
            reset_receipt.revision_after > set_receipt.revision_after,
            "{key} reset must bump the revision (set={} reset={})",
            set_receipt.revision_after,
            reset_receipt.revision_after
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

    // The operator-safe projection lists every MT-042 default (unset keys fall back to registry
    // defaults, set-then-reset keys are back on their default).
    let projection = store
        .list_preference_projection(scope, true)
        .await
        .expect("list preference projection");
    for (key, _, _, default_value) in mt042_preferences() {
        let row = projection
            .iter()
            .find(|row| row.key == key)
            .unwrap_or_else(|| panic!("projection must list {key}"));
        assert_eq!(row.value, default_value, "{key} projection value");
        assert_eq!(
            row.source,
            PreferenceValueSource::Default,
            "{key} projection source"
        );
    }

    harness.shutdown().await;
}

#[tokio::test]
async fn mt042_enumerated_defaults_reject_out_of_vocabulary_values() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
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
        let effective = store
            .get_effective_preference(scope, key)
            .await
            .unwrap_or_else(|err| panic!("get effective {key} after rejection: {err}"));
        assert_eq!(
            effective.source,
            PreferenceValueSource::Default,
            "{key} rejected value must not create an authority row"
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

    // A declared type that disagrees with the registry definition is rejected too.
    let wrong_type = store
        .set_preference_with_receipt(&SetPreference {
            scope,
            key: "posekit.framing-lens-mm".to_string(),
            value_type: PreferenceType::Integer,
            value: "not-a-number".to_string(),
            redacted: false,
        })
        .await;
    assert!(
        matches!(wrong_type, Err(AtelierError::Validation(_))),
        "non-integer lens value must be rejected, got {wrong_type:?}"
    );

    harness.shutdown().await;
}

#[tokio::test]
async fn mt042_actor_attribution_is_recorded_on_set_and_reset() {
    // WP-CKC MT-042 (F4): the `_as` write path records WHO changed the default on the
    // row (`updated_by`) and in the EventLedger payload; reset records `reset_by`.
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
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

    let set_payload = preference_event_payload_for_receipt(
        &harness.database,
        set_receipt.preference.preference_id,
        PREFERENCE_SET_EVENT_FAMILY,
        set_receipt.receipt_id,
    )
    .await;
    assert_eq!(
        set_payload["updated_by"],
        serde_json::json!(actor),
        "set event payload must record who changed the default"
    );
    assert_eq!(
        set_payload["revision_after"],
        serde_json::json!(set_receipt.revision_after)
    );

    // Reset records reset_by and returns the row to the unattributed default.
    let reset_receipt = store
        .reset_preference_to_default_as(scope, key, Some(actor))
        .await
        .expect("reset preference with actor");
    assert_eq!(
        reset_receipt.preference.updated_by, None,
        "reset returns the row to the default (no operator override)"
    );
    let reset_payload = preference_event_payload_for_receipt(
        &harness.database,
        reset_receipt.preference.preference_id,
        PREFERENCE_RESET_EVENT_FAMILY,
        reset_receipt.receipt_id,
    )
    .await;
    assert_eq!(
        reset_payload["reset_by"],
        serde_json::json!(actor),
        "reset event payload must record who performed the reset"
    );
    assert_eq!(
        reset_payload["revision_after"],
        serde_json::json!(reset_receipt.revision_after)
    );

    // One row, two revisions, two events: set + reset never fork the preference row.
    let set_events = preference_events(
        &harness.database,
        set_receipt.preference.preference_id,
        PREFERENCE_SET_EVENT_FAMILY,
    )
    .await;
    let reset_events = preference_events(
        &harness.database,
        set_receipt.preference.preference_id,
        PREFERENCE_RESET_EVENT_FAMILY,
    )
    .await;
    assert_eq!(set_events.len(), 1, "exactly one set event for this row");
    assert_eq!(
        reset_events.len(),
        1,
        "exactly one reset event for this row"
    );
    assert_eq!(
        set_receipt.preference.preference_id,
        reset_receipt.preference.preference_id
    );

    harness.shutdown().await;
}
