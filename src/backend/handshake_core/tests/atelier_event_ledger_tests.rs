//! WP-KERNEL-005 atelier EventLedger integration proof.
//!
//! This test uses the embedded SurrealDB backend and proves an atelier
//! mutation appends to the canonical kernel EventLedger, not only the atelier
//! compatibility projection.

#[allow(dead_code)]
mod atelier_surreal_support;

use handshake_core::atelier::intake::{
    intake_event_family, IntakeBatchMode, IntakeProfileMode, NewIntakeBatch,
};
use handshake_core::atelier::search::{search_event_family, TagType};
use handshake_core::atelier::settings::{PreferenceScope, PreferenceType, SetPreference};
use handshake_core::atelier::stealth_window::stealth_ref_event_family::{
    STEALTH_REF_ADDED, STEALTH_REF_CAPTURED, STEALTH_REF_REMOVED, STEALTH_REF_REORDERED,
    STEALTH_REF_WINDOW_CLOSED, STEALTH_REF_WINDOW_CREATED,
};
use handshake_core::atelier::stealth_window::{
    ContentRefKind, NewContentRef, NewStealthWindow, QuietFlags, VisibilityFlag,
};
use handshake_core::atelier::{event_family, AtelierStore, NewCharacter, NewSheetVersion};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
};
use handshake_core::kernel::{KernelActor, KernelEventType};
use handshake_core::storage::Database;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct MemoryFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait::async_trait]
impl FlightRecorder for MemoryFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self
            .events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .clone())
    }
}

#[derive(Clone, Default)]
struct FailingFlightRecorder;

#[async_trait::async_trait]
impl FlightRecorder for FailingFlightRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Err(RecorderError::SinkError("forced recorder failure".into()))
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(vec![])
    }
}

async fn connected_store_with_ledger() -> (
    AtelierStore,
    Arc<dyn Database>,
    atelier_surreal_support::AtelierSurrealHarness,
) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    (harness.atelier.clone(), harness.database.clone(), harness)
}

async fn connected_store_with_observability() -> (
    AtelierStore,
    Arc<dyn Database>,
    Arc<MemoryFlightRecorder>,
    atelier_surreal_support::AtelierSurrealHarness,
) {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    let flight_recorder = Arc::new(MemoryFlightRecorder::default());
    let store = AtelierStore::with_observability(
        harness.storage.clone(),
        harness.database.clone(),
        flight_recorder.clone(),
    );
    (store, harness.database.clone(), flight_recorder, harness)
}

#[tokio::test]
async fn atelier_character_create_appends_kernel_event_ledger_row() {
    let (store, database, harness) = connected_store_with_ledger().await;

    let public_id = format!("event-ledger-character-{}", Uuid::new_v4());
    let character = store
        .create_character(&NewCharacter {
            public_id: public_id.clone(),
            display_name: "Event Ledger Character".to_string(),
        })
        .await
        .expect("create character");

    drop(store);
    drop(database);
    let data_dir = harness.close_for_reopen().await;
    let reopened = atelier_surreal_support::AtelierSurrealHarness::open_existing(data_dir).await;
    let events = reopened
        .database
        .list_kernel_events_for_aggregate("atelier_character", &character.public_id)
        .await
        .expect("list kernel events for atelier character after reopen");

    assert_eq!(events.len(), 1, "one kernel event for the new character");
    let event = &events[0];
    assert!(
        event.event_sequence > 0,
        "kernel EventLedger assigns a sequence"
    );
    assert_eq!(
        event.event_type,
        KernelEventType::AtelierDomainEventRecorded
    );
    assert_eq!(event.source_component, "atelier");
    assert_eq!(
        event.actor,
        KernelActor::System("atelier".to_string()),
        "atelier domain events are emitted by the atelier system actor"
    );
    assert_eq!(event.aggregate_type, "atelier_character");
    assert_eq!(event.aggregate_id, public_id);
    assert_eq!(
        event
            .payload
            .get("event_family")
            .and_then(|value| value.as_str()),
        Some(event_family::CHARACTER_CREATED)
    );
    assert_eq!(
        event
            .payload
            .get("atelier_payload")
            .and_then(|payload| payload.get("public_id"))
            .and_then(|value| value.as_str()),
        Some(character.public_id.as_str())
    );
    assert!(
        event
            .payload
            .get("atelier_event_id")
            .and_then(|value| value.as_str())
            .is_some(),
        "kernel payload links back to the atelier projection event id"
    );
    assert!(
        event.idempotency_key.starts_with("atelier-event:"),
        "kernel idempotency key is atelier-event scoped"
    );
    reopened.shutdown().await;
}

#[tokio::test]
async fn atelier_event_boundary_sanitizes_identity_source_and_author_payloads() {
    let (store, database, flight_recorder, _harness) = connected_store_with_observability().await;

    let aggregate_id = format!("mt-007-boundary-{}", Uuid::new_v4());
    let character_internal_id = Uuid::new_v4();
    let raw_character_id = character_internal_id.to_string();
    let source_path = "file:///C:/Sensitive Character/source-sheet.txt";
    let source_provenance = "C:\\Sensitive Character\\source-image.png";
    let display_name = "Sensitive Character Display Name";
    let author = "Sensitive Sheet Author";
    let file_name = "Sensitive Character - source-sheet.txt";
    let pack_path = "exports/Sensitive Character/sheet.pdf";
    let source_ref = "operator-imports/Sensitive Character/pose.png";
    let reference_ref = "identity/Sensitive Character/front.png";
    let ingestion_key = "handler|version|spec|Sensitive Character Ingest";
    let normalized_url = "https://example.test/Sensitive-Character/source.png";
    let artifact_manifest_ref = "manifests/Sensitive Character/source.json";
    let preference_value = "Sensitive Character Preference Value";
    let requested_by = "Sensitive Character Requester";
    let raw_values = [
        raw_character_id.as_str(),
        source_path,
        source_provenance,
        display_name,
        author,
        file_name,
        pack_path,
        source_ref,
        reference_ref,
        ingestion_key,
        normalized_url,
        artifact_manifest_ref,
        preference_value,
        requested_by,
    ];

    store
        .record_event(
            "atelier.mt007.boundary_probe",
            "atelier_mt007_probe",
            &aggregate_id,
            serde_json::json!({
                "character_internal_id": character_internal_id,
                "character_ids": [character_internal_id],
                "source_path": source_path,
                "source_provenance": source_provenance,
                "display_name": display_name,
                "author": author,
                "file_name": file_name,
                "pack_path": pack_path,
                "source_ref": source_ref,
                "reference_ref": reference_ref,
                "ingestion_key": ingestion_key,
                "normalized_url": normalized_url,
                "artifact_manifest_refs": [artifact_manifest_ref],
                "value_before": preference_value,
                "value_after": preference_value,
                "default_value": preference_value,
                "requested_by": requested_by,
                "nested": {
                    "character_internal_id": character_internal_id,
                    "source_path": source_path,
                    "author": author,
                    "pack_path": pack_path,
                    "source_ref": source_ref,
                    "reference_ref": reference_ref,
                    "ingestion_key": ingestion_key,
                    "normalized_url": normalized_url,
                },
            }),
        )
        .await
        .expect("record sanitized boundary probe event");

    let atelier_payload = database
        .list_kernel_events_for_aggregate("atelier_mt007_probe", &aggregate_id)
        .await
        .expect("read sanitized atelier payload")
        .into_iter()
        .find(|event| event.payload["event_family"] == "atelier.mt007.boundary_probe")
        .and_then(|event| event.payload.get("atelier_payload").cloned())
        .expect("sanitized atelier payload event");
    assert_eq!(
        atelier_payload.get("character_internal_id"),
        None,
        "atelier projection must not retain raw character_internal_id key"
    );
    assert_eq!(
        atelier_payload.get("source_path"),
        None,
        "atelier projection must not retain raw source_path key"
    );
    assert_eq!(
        atelier_payload.get("source_provenance"),
        None,
        "atelier projection must not retain raw source_provenance key"
    );
    assert_eq!(
        atelier_payload.get("author"),
        None,
        "atelier projection must not retain raw author key"
    );
    assert!(
        atelier_payload["character_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "character_internal_id is projected as a content-addressed ref"
    );
    assert!(
        atelier_payload["source_path_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "source_path is projected as a content-addressed ref"
    );
    assert!(
        atelier_payload["nested"]["author_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "nested author is projected as a content-addressed ref"
    );
    assert!(
        atelier_payload["pack_path_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "pack_path is projected as a content-addressed ref"
    );
    assert!(
        atelier_payload["source_ref_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "source_ref is projected as a content-addressed ref"
    );
    assert!(
        atelier_payload["reference_ref_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "reference_ref is projected as a content-addressed ref"
    );
    assert!(
        atelier_payload["ingestion_key_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "ingestion_key is projected as a content-addressed ref"
    );
    assert!(
        atelier_payload["normalized_url_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "normalized_url is projected as a content-addressed ref"
    );

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_mt007_probe", &aggregate_id)
        .await
        .expect("list kernel events for sanitized boundary probe");
    assert_eq!(kernel_events.len(), 1);

    let flight_events = flight_recorder
        .list_events(EventFilter::default())
        .await
        .expect("list flight recorder events");
    let mirror = flight_events
        .iter()
        .find(|event| {
            event.event_type == FlightRecorderEventType::Diagnostic
                && event.payload["diagnostic_id"] == "atelier_domain_event"
                && event.payload["event_family"] == "atelier.mt007.boundary_probe"
        })
        .expect("sanitized boundary probe has Flight Recorder mirror");

    for (surface, payload) in [
        ("atelier_event", &atelier_payload),
        ("kernel_event_ledger", &kernel_events[0].payload),
        ("flight_recorder", &mirror.payload),
    ] {
        let serialized =
            serde_json::to_string(payload).expect("serialize sanitized event surface payload");
        for raw_value in raw_values {
            assert!(
                !serialized.contains(raw_value),
                "{surface} must not leak raw MT-007 value {raw_value:?}"
            );
        }
    }
}

#[tokio::test]
async fn atelier_intake_batch_event_uses_batch_id_and_hashes_source_identity() {
    let (store, database, flight_recorder, _harness) = connected_store_with_observability().await;

    let unique = Uuid::new_v4();
    let idempotency_key = format!("source-drop/Sensitive Character/{unique}");
    let source_label = format!("Sensitive Character Source Folder {unique}");
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: idempotency_key.clone(),
            source_label: source_label.clone(),
            source_ref: None,
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        })
        .await
        .expect("open intake batch with source-like identity text");

    let raw_aggregate_events = database
        .list_kernel_events_for_aggregate("atelier_intake_batch", &idempotency_key)
        .await
        .expect("list kernel events for raw intake idempotency key");
    assert!(
        raw_aggregate_events.is_empty(),
        "intake batch events must not use source-like idempotency_key as aggregate_id"
    );

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_intake_batch", &batch.batch_id.to_string())
        .await
        .expect("list kernel events for intake batch id");
    assert_eq!(kernel_events.len(), 1);
    assert_eq!(
        kernel_events[0].payload["event_family"],
        intake_event_family::INTAKE_BATCH_CREATED
    );

    let atelier_payload = database
        .list_kernel_events_for_aggregate("atelier_intake_batch", &batch.batch_id.to_string())
        .await
        .expect("read intake batch atelier event")
        .into_iter()
        .find(|event| event.payload["event_family"] == intake_event_family::INTAKE_BATCH_CREATED)
        .and_then(|event| event.payload.get("atelier_payload").cloned())
        .expect("intake batch atelier payload event");
    assert!(
        atelier_payload["idempotency_key_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "idempotency_key is projected as a content-addressed ref"
    );
    assert!(
        atelier_payload["source_label_ref"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:")),
        "source_label is projected as a content-addressed ref"
    );

    let flight_events = flight_recorder
        .list_events(EventFilter::default())
        .await
        .expect("list flight recorder events");
    let mirror = flight_events
        .iter()
        .find(|event| {
            event.event_type == FlightRecorderEventType::Diagnostic
                && event.payload["diagnostic_id"] == "atelier_domain_event"
                && event.payload["event_family"] == intake_event_family::INTAKE_BATCH_CREATED
        })
        .expect("intake batch has Flight Recorder diagnostic mirror");

    for (surface, payload) in [
        ("atelier_event", &atelier_payload),
        ("kernel_event_ledger", &kernel_events[0].payload),
        ("flight_recorder", &mirror.payload),
    ] {
        let serialized =
            serde_json::to_string(payload).expect("serialize intake event surface payload");
        assert!(
            !serialized.contains(&idempotency_key),
            "{surface} must not leak raw intake idempotency_key"
        );
        assert!(
            !serialized.contains(&source_label),
            "{surface} must not leak raw intake source_label"
        );
    }

    let closed = store
        .close_intake_batch(batch.batch_id)
        .await
        .expect("close intake batch with source-like identity text");
    assert_eq!(closed.status.as_str(), "closed");

    let raw_close_aggregate_events = database
        .list_kernel_events_for_aggregate("atelier_intake_batch", &idempotency_key)
        .await
        .expect("list kernel close events for raw intake idempotency key");
    assert!(
        raw_close_aggregate_events.is_empty(),
        "intake batch close events must not use source-like idempotency_key as aggregate_id"
    );
    let closed_kernel_events = database
        .list_kernel_events_for_aggregate("atelier_intake_batch", &batch.batch_id.to_string())
        .await
        .expect("list kernel events for closed intake batch id");
    assert!(
        closed_kernel_events
            .iter()
            .any(|event| event.payload["event_family"] == intake_event_family::INTAKE_BATCH_CLOSED),
        "intake close event must be scoped by batch_id"
    );
}

#[tokio::test]
async fn atelier_tag_events_use_ref_aggregates_and_hash_character_identity() {
    let (store, database, flight_recorder, _harness) = connected_store_with_observability().await;

    let unique = Uuid::new_v4();
    let character = store
        .create_character(&NewCharacter {
            public_id: format!("mt-007-tags-{unique}"),
            display_name: format!("Sensitive Tag Character {unique}"),
        })
        .await
        .expect("create character for tag event proof");
    store
        .tag_character(
            character.internal_id,
            "Sensitive Manual Tag",
            TagType::Manual,
        )
        .await
        .expect("tag character");
    store
        .untag_character(character.internal_id, "Sensitive Manual Tag")
        .await
        .expect("untag character");
    store
        .recompute_derived_tags(character.internal_id, &HashMap::new())
        .await
        .expect("recompute derived tags");

    let raw_aggregate_events = database
        .list_kernel_events_for_aggregate(
            "atelier_character_tag",
            &character.internal_id.to_string(),
        )
        .await
        .expect("list tag events for raw character id aggregate");
    assert!(
        raw_aggregate_events.is_empty(),
        "tag events must not use character internal_id as aggregate_id"
    );

    let tag_events: Vec<serde_json::Value> = database
        .list_kernel_events_for_aggregate("atelier_character_tag", "bulk")
        .await
        .expect("read tag EventLedger events")
        .into_iter()
        .filter(|event| {
            [
                search_event_family::CHARACTER_TAGGED,
                search_event_family::CHARACTER_UNTAGGED,
                search_event_family::DERIVED_TAGS_RECOMPUTED,
            ]
            .contains(&event.payload["event_family"].as_str().unwrap_or_default())
        })
        .map(|event| event.payload["atelier_payload"].clone())
        .collect();
    assert!(
        tag_events.iter().any(|payload| {
            payload
                .get("character_ref")
                .and_then(|value| value.as_str())
                .is_some_and(|value| value.starts_with("sha256:"))
        }),
        "tag event payloads must project character_internal_id as character_ref"
    );

    let raw_character_id = character.internal_id.to_string();
    let flight_events = flight_recorder
        .list_events(EventFilter::default())
        .await
        .expect("list flight recorder events");
    for (surface, serialized) in [
        (
            "atelier_event",
            serde_json::to_string(&tag_events).expect("serialize tag atelier events"),
        ),
        (
            "flight_recorder",
            serde_json::to_string(&flight_events).expect("serialize tag flight events"),
        ),
    ] {
        assert!(
            !serialized.contains(&raw_character_id),
            "{surface} must not leak raw tag character internal_id"
        );
    }
}

#[tokio::test]
async fn atelier_embedded_constructor_routes_mutations_to_kernel_event_ledger() {
    let (store, database, _harness) = connected_store_with_ledger().await;

    let public_id = format!("connect-ledger-character-{}", Uuid::new_v4());
    let character = store
        .create_character(&NewCharacter {
            public_id: public_id.clone(),
            display_name: "Connect Constructor Ledger Character".to_string(),
        })
        .await
        .expect("create character through connect constructor");

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_character", &character.public_id)
        .await
        .expect("list kernel events for connect constructor mutation");
    assert_eq!(
        kernel_events.len(),
        1,
        "AtelierStore::connect must not create private atelier_event-only mutations"
    );
    assert_eq!(
        kernel_events[0].event_type,
        KernelEventType::AtelierDomainEventRecorded
    );
    assert_eq!(
        kernel_events[0]
            .payload
            .get("event_family")
            .and_then(|value| value.as_str()),
        Some(event_family::CHARACTER_CREATED)
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::CHARACTER_CREATED,
                "atelier_character",
                &character.public_id,
            )
            .await
            .expect("count embedded atelier projection linkage"),
        1,
        "embedded atelier projection must retain the canonical event linkage"
    );
    assert_eq!(
        kernel_events[0].payload["event_family"],
        event_family::CHARACTER_CREATED
    );
}

#[tokio::test]
async fn atelier_embedded_new_constructor_routes_mutations_to_kernel_event_ledger() {
    let (store, database, _harness) = connected_store_with_ledger().await;

    let public_id = format!("new-ledger-character-{}", Uuid::new_v4());
    let character = store
        .create_character(&NewCharacter {
            public_id: public_id.clone(),
            display_name: "New Constructor Ledger Character".to_string(),
        })
        .await
        .expect("create character through new constructor");

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_character", &character.public_id)
        .await
        .expect("list kernel events for new constructor mutation");
    assert_eq!(
        kernel_events.len(),
        1,
        "AtelierStore::new must not create private atelier_event-only mutations"
    );
    assert_eq!(
        kernel_events[0].event_type,
        KernelEventType::AtelierDomainEventRecorded
    );
    assert_eq!(
        kernel_events[0]
            .payload
            .get("event_family")
            .and_then(|value| value.as_str()),
        Some(event_family::CHARACTER_CREATED)
    );
    assert_eq!(
        store
            .count_events_for_aggregate(
                event_family::CHARACTER_CREATED,
                "atelier_character",
                &character.public_id,
            )
            .await
            .expect("count embedded atelier projection linkage"),
        1,
        "embedded atelier projection must retain the canonical event linkage"
    );
    assert_eq!(
        kernel_events[0].payload["event_family"],
        event_family::CHARACTER_CREATED
    );
}

#[tokio::test]
async fn atelier_character_and_sheet_events_do_not_leak_identity_text() {
    let (store, database, _harness) = connected_store_with_ledger().await;

    let unique = Uuid::new_v4();
    let public_id = format!("identity-decoupled-character-{unique}");
    let display_name = format!("Leaky Display Name {unique}");
    let sheet_author = format!("Sensitive Sheet Author {unique}");
    let character = store
        .create_character(&NewCharacter {
            public_id: public_id.clone(),
            display_name: display_name.clone(),
        })
        .await
        .expect("create identity-decoupled character");
    let sheet = store
        .append_sheet_version(&NewSheetVersion {
            character_internal_id: character.internal_id,
            raw_text: format!("sheet text {unique}"),
            author: sheet_author.clone(),
            tool: Some("identity-decoupled-event-test".to_string()),
        })
        .await
        .expect("append sheet version");

    let character_events = database
        .list_kernel_events_for_aggregate("atelier_character", &character.public_id)
        .await
        .expect("list character kernel events");
    assert_eq!(
        character_events.len(),
        1,
        "one character event is scoped to this run's public_id"
    );
    let character_payload = serde_json::to_string(&character_events[0].payload)
        .expect("serialize character kernel payload");
    assert!(
        character_payload.contains(&character.public_id),
        "character event keeps stable public_id for replay"
    );
    assert!(
        !character_payload.contains(&display_name),
        "character event payload must not leak display_name identity text"
    );
    assert!(
        !character_payload.contains(&character.internal_id.to_string()),
        "character event payload must not leak storage internal_id"
    );
    assert!(
        character_events[0]
            .payload
            .get("atelier_payload")
            .and_then(|payload| payload.get("display_name"))
            .is_none(),
        "display_name must not be an atelier_payload field"
    );

    let sheet_events = database
        .list_kernel_events_for_aggregate("atelier_sheet_version", &sheet.version_id.to_string())
        .await
        .expect("list sheet kernel events");
    let sheet_event = sheet_events
        .iter()
        .find(|event| {
            event.payload["event_family"] == event_family::SHEET_VERSION_APPENDED
                && event.payload["atelier_payload"]["version_id"] == sheet.version_id.to_string()
        })
        .expect("sheet append event for this version");
    let sheet_payload =
        serde_json::to_string(&sheet_event.payload).expect("serialize sheet kernel payload");
    assert!(
        sheet_payload.contains(&sheet.version_id.to_string()),
        "sheet event keeps stable version_id for replay"
    );
    assert!(
        !sheet_payload.contains(&character.internal_id.to_string()),
        "sheet event payload must not leak storage internal_id"
    );
    assert!(
        !sheet_payload.contains(&sheet_author),
        "sheet event payload must not leak raw author identity text"
    );
    assert!(
        sheet_event
            .payload
            .get("atelier_payload")
            .and_then(|payload| payload.get("author"))
            .is_none(),
        "author must not be an atelier_payload field"
    );

    let internal_id_scoped_sheet_events = database
        .list_kernel_events_for_aggregate(
            "atelier_sheet_version",
            &character.internal_id.to_string(),
        )
        .await
        .expect("list internal-id scoped sheet kernel events");
    assert!(
        internal_id_scoped_sheet_events.is_empty(),
        "sheet events must be scoped by version_id, not character internal_id"
    );
}

#[tokio::test]
async fn atelier_settings_set_appends_kernel_event_and_flight_recorder_mirror() {
    let (store, database, flight_recorder, _harness) = connected_store_with_observability().await;

    let key = format!("feature-toggles.atelier-diagnostics-{}", Uuid::new_v4());
    let receipt = store
        .set_preference_with_receipt(&SetPreference {
            scope: PreferenceScope::Global,
            key: key.clone(),
            value_type: PreferenceType::Bool,
            value: "true".to_string(),
            redacted: false,
        })
        .await
        .expect("set preference with receipt");

    let kernel_events = database
        .list_kernel_events_for_aggregate(
            "atelier_preference",
            &receipt.preference.preference_id.to_string(),
        )
        .await
        .expect("list kernel events for atelier preference");

    assert_eq!(kernel_events.len(), 1, "one kernel event for the setting");
    let kernel_event = &kernel_events[0];
    assert_eq!(
        kernel_event.event_type,
        KernelEventType::AtelierDomainEventRecorded
    );
    assert_eq!(
        kernel_event
            .payload
            .get("event_family")
            .and_then(|value| value.as_str()),
        Some("atelier.preference.set")
    );

    let flight_events = flight_recorder
        .list_events(EventFilter::default())
        .await
        .expect("list flight recorder events");
    let mirror = flight_events
        .iter()
        .find(|event| {
            event.event_type == FlightRecorderEventType::Diagnostic
                && event.payload["diagnostic_id"] == "atelier_domain_event"
                && event.payload["event_family"] == "atelier.preference.set"
        })
        .expect("settings mutation has Flight Recorder diagnostic mirror");
    assert_eq!(mirror.payload["authority_source"], "surreal_event_ledger");
    assert_eq!(mirror.payload["projection_only"], true);
    assert_eq!(mirror.payload["kernel_event_id"], kernel_event.event_id);
    assert_eq!(
        mirror.payload["kernel_event_sequence"],
        kernel_event.event_sequence
    );
    assert_eq!(mirror.payload["aggregate_type"], "atelier_preference");
    assert_eq!(
        mirror.payload["aggregate_id"],
        receipt.preference.preference_id.to_string()
    );
}

#[tokio::test]
async fn atelier_stealth_mutations_append_kernel_events_and_flight_recorder_mirrors() {
    let (store, database, flight_recorder, _harness) = connected_store_with_observability().await;

    let window = store
        .create_stealth_window(&NewStealthWindow {
            owner_actor: format!("operator-{}", Uuid::new_v4()),
            title: format!("stealth-observability-{}", Uuid::new_v4()),
            visibility: VisibilityFlag::OffScreenOnly,
            quiet: QuietFlags::default(),
            layout: None,
        })
        .await
        .expect("create stealth window");
    let ref0 = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: format!("artifact-manifest-{}", Uuid::new_v4()),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add first content ref");
    let ref1 = store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Screenshot,
                resolver: format!("artifact-manifest-{}", Uuid::new_v4()),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await
        .expect("add second content ref");
    store
        .reorder_stealth_refs(window.window_ref_id, &[ref1.ref_id, ref0.ref_id], None)
        .await
        .expect("reorder stealth refs");
    store
        .remove_stealth_ref(window.window_ref_id, ref1.ref_id)
        .await
        .expect("remove stealth ref");
    store
        .record_stealth_capture(
            window.window_ref_id,
            &format!("artifact-manifest-{}", Uuid::new_v4()),
            &format!("sha256-{}", Uuid::new_v4()),
        )
        .await
        .expect("record stealth capture");
    store
        .close_stealth_window(window.window_ref_id)
        .await
        .expect("close stealth window");

    let aggregate_id = window.window_ref_id.to_string();
    let expected_families: HashSet<&str> = [
        STEALTH_REF_WINDOW_CREATED,
        STEALTH_REF_ADDED,
        STEALTH_REF_REORDERED,
        STEALTH_REF_REMOVED,
        STEALTH_REF_CAPTURED,
        STEALTH_REF_WINDOW_CLOSED,
    ]
    .into_iter()
    .collect();
    let registered_families: HashSet<&str> = event_family::ALL.iter().copied().collect();
    assert!(
        expected_families.is_subset(&registered_families),
        "shared atelier event-family registry includes stealth families"
    );

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_stealth_window", &aggregate_id)
        .await
        .expect("list kernel events for stealth window");
    assert_eq!(
        kernel_events.len(),
        7,
        "create + two adds + reorder + remove + capture + close append kernel events"
    );
    let kernel_families: HashSet<&str> = kernel_events
        .iter()
        .filter_map(|event| {
            assert_eq!(
                event.event_type,
                KernelEventType::AtelierDomainEventRecorded
            );
            assert_eq!(event.source_component, "atelier");
            assert_eq!(event.aggregate_type, "atelier_stealth_window");
            assert_eq!(event.aggregate_id, aggregate_id);
            event
                .payload
                .get("event_family")
                .and_then(|value| value.as_str())
        })
        .collect();
    assert_eq!(
        kernel_families, expected_families,
        "kernel ledger contains every stealth mutation/capture family"
    );
    for event in &kernel_events {
        let family = event
            .payload
            .get("event_family")
            .and_then(|value| value.as_str())
            .expect("kernel event family");
        let atelier_payload = event
            .payload
            .get("atelier_payload")
            .expect("atelier payload");
        assert_eq!(
            atelier_payload["window_ref_id"], aggregate_id,
            "{family} carries window_ref_id"
        );
        assert_eq!(
            atelier_payload["owner_actor"], window.owner_actor,
            "{family} carries owner_actor"
        );
        assert!(
            atelier_payload["revision"].is_i64(),
            "{family} carries resulting revision"
        );
        match family {
            STEALTH_REF_ADDED => {
                let ref_id = atelier_payload["ref_id"]
                    .as_str()
                    .expect("added event ref_id");
                assert!(
                    ref_id == ref0.ref_id.to_string() || ref_id == ref1.ref_id.to_string(),
                    "added event ref_id names an added ref"
                );
            }
            STEALTH_REF_REMOVED => {
                assert_eq!(atelier_payload["ref_id"], ref1.ref_id.to_string());
            }
            STEALTH_REF_REORDERED => {
                let ordered_ref_ids = atelier_payload["ordered_ref_ids"]
                    .as_array()
                    .expect("reorder event ordered_ref_ids");
                assert_eq!(
                    ordered_ref_ids,
                    &vec![
                        serde_json::json!(ref1.ref_id),
                        serde_json::json!(ref0.ref_id)
                    ]
                );
            }
            STEALTH_REF_CAPTURED => {
                assert!(
                    atelier_payload["capture_id"].is_string(),
                    "capture event carries capture_id"
                );
            }
            _ => {}
        }
    }

    let flight_events = flight_recorder
        .list_events(EventFilter::default())
        .await
        .expect("list flight recorder events");
    let flight_families: HashSet<&str> = flight_events
        .iter()
        .filter(|event| {
            event.event_type == FlightRecorderEventType::Diagnostic
                && event.payload["diagnostic_id"] == "atelier_domain_event"
                && event.payload["aggregate_type"] == "atelier_stealth_window"
                && event.payload["aggregate_id"] == aggregate_id
                && event.payload["authority_source"] == "surreal_event_ledger"
                && event.payload["projection_only"] == true
                && event.payload["kernel_event_id"].is_string()
                && event.payload["kernel_event_sequence"].is_i64()
        })
        .filter_map(|event| event.payload["event_family"].as_str())
        .collect();
    assert_eq!(
        flight_families, expected_families,
        "Flight Recorder mirrors every stealth mutation/capture family"
    );
}

#[tokio::test]
async fn atelier_stealth_mutations_roll_back_when_flight_recorder_fails() {
    let harness = atelier_surreal_support::AtelierSurrealHarness::create().await;
    let good_store = AtelierStore::with_observability(
        harness.storage.clone(),
        harness.database.clone(),
        Arc::new(MemoryFlightRecorder::default()),
    );
    let failing_store = AtelierStore::with_observability(
        harness.storage.clone(),
        harness.database.clone(),
        Arc::new(FailingFlightRecorder),
    );

    let owner_actor = format!("operator-{}", Uuid::new_v4());
    let title = format!("stealth-rollback-{}", Uuid::new_v4());
    let create_result = failing_store
        .create_stealth_window(&NewStealthWindow {
            owner_actor: owner_actor.clone(),
            title: title.clone(),
            visibility: VisibilityFlag::OffScreenOnly,
            quiet: QuietFlags::default(),
            layout: None,
        })
        .await;
    assert!(
        create_result.is_err(),
        "create returns an error when Flight Recorder evidence fails"
    );
    let leaked_windows = good_store
        .list_stealth_windows(&owner_actor, None, 100)
        .await
        .expect("list leaked windows")
        .into_iter()
        .filter(|window| window.owner_actor == owner_actor && window.title == title)
        .count() as i64;
    assert_eq!(
        leaked_windows, 0,
        "failed create leaves no durable stealth window row"
    );
    let orphan_kernel_events = good_store
        .count_events(STEALTH_REF_WINDOW_CREATED)
        .await
        .expect("count orphan kernel events after failed create");
    assert_eq!(
        orphan_kernel_events, 0,
        "failed create must not leave a canonical kernel event without domain state"
    );
    let orphan_projection_events = good_store
        .count_events(STEALTH_REF_WINDOW_CREATED)
        .await
        .expect("count orphan projection events after failed create");
    assert_eq!(
        orphan_projection_events, 0,
        "failed create must not leave an atelier_event projection without domain state"
    );

    let window = good_store
        .create_stealth_window(&NewStealthWindow {
            owner_actor,
            title: format!("stealth-good-{}", Uuid::new_v4()),
            visibility: VisibilityFlag::OffScreenOnly,
            quiet: QuietFlags::default(),
            layout: None,
        })
        .await
        .expect("create good stealth window");
    let revision_before_failures = window.revision;

    let add_result = failing_store
        .add_stealth_ref(
            window.window_ref_id,
            &NewContentRef {
                ref_kind: ContentRefKind::Artifact,
                resolver: format!("artifact-manifest-{}", Uuid::new_v4()),
                content_sha256: format!("sha256-{}", Uuid::new_v4()),
                redaction_state: true,
            },
        )
        .await;
    assert!(
        add_result.is_err(),
        "add_ref returns an error when Flight Recorder evidence fails"
    );
    assert!(
        good_store
            .list_stealth_refs(window.window_ref_id)
            .await
            .expect("list refs after failed add")
            .is_empty(),
        "failed add_ref leaves no durable ref row"
    );
    assert_eq!(
        good_store
            .get_stealth_window(window.window_ref_id)
            .await
            .expect("get window after failed add")
            .revision,
        revision_before_failures,
        "failed add_ref does not advance revision"
    );

    let capture_result = failing_store
        .record_stealth_capture(
            window.window_ref_id,
            &format!("artifact-manifest-{}", Uuid::new_v4()),
            &format!("sha256-{}", Uuid::new_v4()),
        )
        .await;
    assert!(
        capture_result.is_err(),
        "capture returns an error when Flight Recorder evidence fails"
    );
    assert!(
        good_store
            .list_stealth_captures(window.window_ref_id)
            .await
            .expect("list captures after failed capture")
            .is_empty(),
        "failed capture leaves no durable capture receipt"
    );

    let close_result = failing_store
        .close_stealth_window(window.window_ref_id)
        .await;
    assert!(
        close_result.is_err(),
        "close returns an error when Flight Recorder evidence fails"
    );
    let after_failed_close = good_store
        .get_stealth_window(window.window_ref_id)
        .await
        .expect("get window after failed close");
    assert_eq!(
        after_failed_close.status,
        handshake_core::atelier::stealth_window::StealthRefStatus::Open,
        "failed close keeps the window open"
    );
    assert_eq!(
        after_failed_close.revision, revision_before_failures,
        "failed capture/close do not advance revision"
    );
}
