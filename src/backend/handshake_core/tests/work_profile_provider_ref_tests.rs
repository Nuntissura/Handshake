//! WP-1 MT-014 — Work Profiles `provider_ref` resolver proofs.
//!
//! Engine-free + DB-free: the resolver is pure and the migration audit records
//! to a capturing Flight Recorder. These prove:
//!   * canonical ids (`local_runtime`, `openai_compat`) resolve unchanged,
//!   * the retired `ollama` daemon id migrates deterministically to
//!     `local_runtime`, SURFACED (never a silent in-place rewrite),
//!   * an unrecognized `provider_ref` resolves to a typed `Unknown` (not coerced
//!     to a default),
//!   * a migration emits a surfaced FR-EVT-PROFILE Flight Recorder event, while
//!     a canonical resolution records nothing.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::kernel::work_profiles::{
    record_provider_ref_migration, resolve_provider_ref, CanonicalProviderRef,
    ProviderRefResolution, PROVIDER_REF_MIGRATION_FR_EVENT,
};

#[derive(Default)]
struct CapturingRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

impl CapturingRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().expect("recorder lock").clone()
    }
}

#[async_trait]
impl FlightRecorder for CapturingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events.lock().expect("recorder lock").push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events())
    }
}

#[test]
fn mt014_provider_ref_resolves_canonical_ids() {
    assert_eq!(
        resolve_provider_ref("local_runtime"),
        ProviderRefResolution::Canonical(CanonicalProviderRef::LocalRuntime)
    );
    assert_eq!(
        resolve_provider_ref("openai_compat"),
        ProviderRefResolution::Canonical(CanonicalProviderRef::OpenAiCompat)
    );
    // Surrounding whitespace is tolerated (trimmed), still canonical.
    assert_eq!(
        resolve_provider_ref("  local_runtime  "),
        ProviderRefResolution::Canonical(CanonicalProviderRef::LocalRuntime)
    );
    assert_eq!(
        CanonicalProviderRef::LocalRuntime.as_str(),
        "local_runtime"
    );
    assert_eq!(CanonicalProviderRef::OpenAiCompat.as_str(), "openai_compat");
}

#[test]
fn mt014_provider_ref_migrates_ollama_to_local_runtime() {
    let resolution = resolve_provider_ref("ollama");
    match &resolution {
        ProviderRefResolution::Migrated {
            from,
            to,
            event_ref,
        } => {
            assert_eq!(from, "ollama");
            assert_eq!(*to, CanonicalProviderRef::LocalRuntime);
            assert!(
                event_ref.starts_with("FR-EVT-PROFILE-"),
                "migration must be surfaced via the FR-EVT-PROFILE- receipt convention, got {event_ref}"
            );
        }
        other => panic!("expected ollama -> local_runtime migration, got {other:?}"),
    }
    assert!(resolution.is_migration());
    assert_eq!(
        resolution.canonical(),
        Some(CanonicalProviderRef::LocalRuntime),
        "the migrated route resolves to a canonical provider id"
    );
}

#[test]
fn mt014_provider_ref_unknown_is_typed_not_coerced() {
    let resolution = resolve_provider_ref("provider://coder");
    assert_eq!(
        resolution,
        ProviderRefResolution::Unknown("provider://coder".to_string()),
        "an unrecognized provider_ref is typed Unknown, never silently coerced"
    );
    assert!(!resolution.is_migration());
    assert_eq!(resolution.canonical(), None);
}

#[tokio::test]
async fn mt014_provider_ref_migration_emits_surfaced_fr_evt_profile_event() {
    let recorder = CapturingRecorder::default();
    let resolution = resolve_provider_ref("ollama");

    record_provider_ref_migration(&recorder, "wp-1-default", "coder", &resolution)
        .await
        .expect("record migration");

    let events = recorder.events();
    assert_eq!(events.len(), 1, "the migration emits exactly one FR event");
    let event = &events[0];
    assert_eq!(event.payload["fr_event"], PROVIDER_REF_MIGRATION_FR_EVENT);
    assert!(
        event.payload["fr_event"]
            .as_str()
            .is_some_and(|v| v.starts_with("FR-EVT-PROFILE-")),
        "surfaced event mirrors the FR-EVT-PROFILE- receipt convention"
    );
    assert_eq!(event.payload["from"], "ollama");
    assert_eq!(event.payload["to"], "local_runtime");
    assert_eq!(event.payload["profile_id"], "wp-1-default");
    assert_eq!(event.payload["role_id"], "coder");
}

#[tokio::test]
async fn mt014_provider_ref_canonical_resolution_records_no_event() {
    let recorder = CapturingRecorder::default();

    // A canonical (non-migrated) resolution must NOT emit a migration event.
    record_provider_ref_migration(
        &recorder,
        "wp-1-default",
        "coder",
        &resolve_provider_ref("local_runtime"),
    )
    .await
    .expect("no-op record");
    // An Unknown resolution also emits nothing (it is surfaced as a validation
    // concern by the caller, not as a migration).
    record_provider_ref_migration(
        &recorder,
        "wp-1-default",
        "coder",
        &resolve_provider_ref("provider://coder"),
    )
    .await
    .expect("no-op record");

    assert!(
        recorder.events().is_empty(),
        "only a real migration is recorded; canonical/unknown are no-ops"
    );
}
