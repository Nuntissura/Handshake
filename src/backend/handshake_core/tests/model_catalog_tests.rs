//! WP-1 MT-014 — shared, enumerable, labeled model catalog proofs.
//!
//! Engine-free (no llama.cpp / Candle model load, no PostgreSQL): the catalog is
//! an in-memory shared handle over a `ModelRegistry`, and the selection-decision
//! audit records to a capturing Flight Recorder. These prove:
//!   * enumeration reflects the configured local model with a label + a STABLE
//!     cross-session anchor (artifact sha256) alongside the per-boot UUIDv7,
//!   * empty registry -> empty list,
//!   * unknown model_id -> stable sentinel label (never panic/blank),
//!   * recording a model-selection decision emits an auditable EventLedger
//!     (Tier-1 Flight Recorder) event distinct from a launch/inference event.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;

use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::{
    BaseModelTag, ModelCapabilities, ModelCatalog, ModelId, ModelRegistration, ModelRegistry,
    OperatorId, ProviderKind, RuntimeBinding, MODEL_SELECTION_FR_EVENT, UNKNOWN_MODEL_LABEL,
};

const TEST_BASE_MODEL_TAG: &str = "Qwen2.5-Coder-7B";
const TEST_SHA256: [u8; 32] = [7u8; 32];

/// A capturing Flight Recorder that validates + retains every event, so tests
/// can assert the auditable selection-decision event shape.
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
        // Prove the emitted event is schema-valid, exactly like the real sink.
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

/// Builds a one-entry registry (the boot shape: registered + marked loaded) and
/// returns the shared catalog plus the registered model_id string.
fn registered_catalog() -> (Arc<ModelCatalog>, String) {
    let model_id = ModelId::new_v7();
    let registration = ModelRegistration {
        model_id,
        artifact_path: PathBuf::from("/models/qwen2.5-coder-7b.gguf"),
        sha256: TEST_SHA256,
        runtime_binding: RuntimeBinding::LlamaCpp,
        declared_capabilities: ModelCapabilities::default(),
        base_model_tag: BaseModelTag::new(TEST_BASE_MODEL_TAG),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("test-operator"),
        provider: ProviderKind::Local,
    };
    let mut registry = ModelRegistry::default();
    registry.register(registration).expect("register");
    registry.mark_loaded(model_id).expect("mark loaded");
    let catalog = ModelCatalog::from_registry(Arc::new(registry));
    (catalog, model_id.to_string())
}

fn registration_with_capabilities(
    model_id: ModelId,
    tag: &str,
    sha_byte: u8,
    capabilities: ModelCapabilities,
) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: PathBuf::from(format!("fixtures/models/{tag}.gguf")),
        sha256: [sha_byte; 32],
        runtime_binding: RuntimeBinding::Candle,
        declared_capabilities: capabilities,
        base_model_tag: BaseModelTag::new(tag),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("test-operator"),
        provider: ProviderKind::Local,
    }
}

#[test]
fn mt014_catalog_enumerates_and_labels_configured_model() {
    let (catalog, model_id) = registered_catalog();

    let entries = catalog.list();
    assert_eq!(entries.len(), 1, "one configured local model enumerated");
    let entry = &entries[0];

    assert_eq!(entry.model_id, model_id, "per-boot UUIDv7 identity present");
    assert_eq!(
        entry.display_name, TEST_BASE_MODEL_TAG,
        "labeled with the display name"
    );
    assert_eq!(entry.base_model_tag, TEST_BASE_MODEL_TAG);
    assert_eq!(entry.runtime_binding, "llama_cpp");
    assert_eq!(
        entry.artifact_path, "/models/qwen2.5-coder-7b.gguf",
        "catalog entry exposes the local artifact path needed by launch routing"
    );
    assert!(entry.ready, "loaded boot model enumerates as READY");

    // STABLE cross-session anchor is present ALONGSIDE the per-boot UUIDv7, and
    // is a distinct identity (the artifact sha256, not the re-minted uuid).
    assert!(
        !entry.artifact_sha256.is_empty(),
        "stable cross-session anchor present"
    );
    assert_ne!(
        entry.artifact_sha256, entry.model_id,
        "stable anchor is the artifact sha256, distinct from the per-boot uuid"
    );
    assert_eq!(
        catalog.stable_anchor(&model_id).as_deref(),
        Some(entry.artifact_sha256.as_str()),
        "stable_anchor() agrees with the enumerated entry"
    );
    assert_eq!(catalog.label_for(&model_id), TEST_BASE_MODEL_TAG);
}

#[test]
fn mt014_catalog_empty_registry_is_empty_list() {
    let empty = ModelCatalog::empty();
    assert!(empty.list().is_empty(), "empty registry -> empty list");
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());

    // A lookup against an empty catalog degrades to the sentinel, never a panic.
    let some_id = ModelId::new_v7().to_string();
    assert_eq!(empty.label_for(&some_id), UNKNOWN_MODEL_LABEL);
    assert_eq!(empty.stable_anchor(&some_id), None);
    assert!(empty.entry(&some_id).is_none());
}

#[test]
fn mt014_catalog_unknown_model_id_sentinel_label() {
    let (catalog, _model_id) = registered_catalog();

    // A well-formed but unregistered UUIDv7.
    let unknown = ModelId::new_v7().to_string();
    assert_eq!(
        catalog.label_for(&unknown),
        UNKNOWN_MODEL_LABEL,
        "unknown model_id -> stable sentinel label"
    );
    assert_eq!(catalog.stable_anchor(&unknown), None);
    assert!(catalog.entry(&unknown).is_none());

    // A non-UUID garbage id must ALSO degrade to the sentinel, never panic/blank.
    assert_eq!(catalog.label_for("not-a-uuid"), UNKNOWN_MODEL_LABEL);
    assert_eq!(catalog.label_for(""), UNKNOWN_MODEL_LABEL);
}

#[tokio::test]
async fn mt014_catalog_records_selection_decision_event() {
    let (catalog, model_id) = registered_catalog();
    let recorder = CapturingRecorder::default();

    catalog
        .record_selection_decision(
            &recorder,
            &model_id,
            "operator",
            "operator selected the embedded local model",
        )
        .await
        .expect("record selection decision");

    let events = recorder.events();
    assert_eq!(events.len(), 1, "exactly one auditable selection event");
    let event = &events[0];

    assert_eq!(
        event.payload["fr_event"], MODEL_SELECTION_FR_EVENT,
        "stable auditable selection event key (distinct from launch/inference)"
    );
    assert_eq!(event.payload["type"], "model_selection_recorded");
    assert_eq!(event.payload["selected_model_id"], model_id);
    assert_eq!(event.payload["display_name"], TEST_BASE_MODEL_TAG);
    assert_eq!(event.payload["actor"], "operator");

    // The STABLE cross-session anchor is recorded so the decision remains
    // correlatable across restarts (the per-boot uuid is not enough).
    let anchor = catalog.stable_anchor(&model_id).expect("anchor");
    assert_eq!(
        event.payload["stable_anchor_sha256"].as_str(),
        Some(anchor.as_str()),
        "selection audit records the stable cross-session anchor"
    );
    // The event carries the selected model id for correlation.
    assert_eq!(event.model_id.as_deref(), Some(model_id.as_str()));
}

#[test]
fn mt016_catalog_selects_ready_embedding_capable_model_distinct_from_chat() {
    let chat_id = ModelId::new_v7();
    let wrong_dim_id = ModelId::new_v7();
    let unloaded_embed_id = ModelId::new_v7();
    let embed_id = ModelId::new_v7();

    let mut registry = ModelRegistry::default();
    registry
        .register(registration_with_capabilities(
            chat_id,
            "chat-model",
            1,
            ModelCapabilities::default(),
        ))
        .expect("chat registration");
    registry.mark_loaded(chat_id).expect("chat ready");
    registry
        .register(registration_with_capabilities(
            wrong_dim_id,
            "embedding-896",
            2,
            ModelCapabilities {
                supports_embedding: true,
                embedding_dimension: Some(896),
                ..Default::default()
            },
        ))
        .expect("wrong-dim registration");
    registry.mark_loaded(wrong_dim_id).expect("wrong dim ready");
    registry
        .register(registration_with_capabilities(
            unloaded_embed_id,
            "embedding-unloaded",
            3,
            ModelCapabilities {
                supports_embedding: true,
                embedding_dimension: Some(768),
                ..Default::default()
            },
        ))
        .expect("unloaded embedding registration");
    registry
        .register(registration_with_capabilities(
            embed_id,
            "embedding-768",
            4,
            ModelCapabilities {
                supports_embedding: true,
                embedding_dimension: Some(768),
                ..Default::default()
            },
        ))
        .expect("embedding registration");
    registry.mark_loaded(embed_id).expect("embedding ready");

    let catalog = ModelCatalog::from_registry(Arc::new(registry));
    let selected = catalog
        .embedding_model_for_dim(768)
        .expect("ready 768-dim embedding model selected");

    assert_eq!(selected.model_id, embed_id.to_string());
    assert_ne!(
        selected.model_id,
        chat_id.to_string(),
        "chat model must not be selected as an embedding model"
    );
    let selected_space = selected
        .embedding_space_id()
        .expect("embedding-capable entry exposes stable vector-space id");
    assert_eq!(
        selected_space,
        format!("embedspace:{}:dim:768", selected.artifact_sha256),
        "embedding-space id is stable artifact sha256 + dimension, not per-boot uuid"
    );
    assert_ne!(
        selected_space, selected.model_id,
        "durable vector-space id must be distinct from per-boot routing model id"
    );
    assert!(selected.supports_embedding);
    assert_eq!(selected.embedding_dimension, Some(768));
    assert!(selected.ready);
    assert_eq!(
        catalog.embedding_model_for_dim(1024),
        None,
        "wrong requested dimension must not select a model"
    );
}
