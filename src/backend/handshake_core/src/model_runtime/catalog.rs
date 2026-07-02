//! Shared, enumerable, labeled model catalog (WP-1 MT-014).
//!
//! MT-003 wires the default `LlmClient` to the embedded `ModelRuntime` but the
//! `ModelRegistry` it registers the model into is a throwaway single-entry map
//! buried as a private field inside `LocalRouter` — not shared, not enumerable,
//! and its per-boot UUIDv7 has no exposed label join. That blocks diagnostics
//! labeling (MT-008), the operator model-picker (MT-012), and any surface that
//! needs to *list* the configured local model.
//!
//! [`ModelCatalog`] is the thin, shared enumeration/label surface over that same
//! boot `ModelRegistry` (the SAME `Arc<ModelRegistry>` `LocalRouter` routes
//! through — NOT a second registry world). It provides:
//!
//!   * [`ModelCatalog::list`] — enumerate the configured local model(s) with a
//!     label (`display_name`/`base_model_tag`) and READY state. Empty registry
//!     yields an empty list (never a panic/blank).
//!   * A STABLE CROSS-SESSION anchor (`artifact_sha256`) exposed ALONGSIDE the
//!     per-boot UUIDv7. `ModelId::new_v7()` is re-minted every boot, so a
//!     diagnostic keyed only on the UUIDv7 fragment cannot correlate the same
//!     artifact across restarts; the artifact sha256 (and `base_model_tag`) do.
//!   * [`ModelCatalog::label_for`] — resolve a `model_id` to a human label; an
//!     UNKNOWN id yields a stable sentinel label ([`UNKNOWN_MODEL_LABEL`]),
//!     never a panic or an empty string.
//!   * [`ModelCatalog::record_selection_decision`] — record a model-selection
//!     decision as an auditable Flight Recorder (Tier-1 business-event ledger)
//!     event, distinct from a launch/inference event, per master-spec
//!     §4.3.9.4.4 ("record the selection decision as an auditable event").
//!
//! Durability posture (per pre-impl review F3): this is an IN-MEMORY shared
//! handle re-derived deterministically from env config at boot — NOT a new
//! Postgres registry table and NOT SQLite. Only the SELECTION decision is
//! persisted (as an EventLedger/Flight-Recorder event), not the registry itself.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
};

use super::{ModelId, ModelRegistration, ModelRegistry};

/// Stable sentinel label for a `model_id` that is not present in the catalog.
/// Diagnostics render this instead of the raw opaque UUID or a blank so an
/// unlabelable model is visibly typed rather than silently empty.
pub const UNKNOWN_MODEL_LABEL: &str = "unknown model";

/// Stable Flight Recorder event key for a recorded model-selection decision.
/// Distinct from launch/inference events (`llm_inference`, session lifecycle).
pub const MODEL_SELECTION_FR_EVENT: &str = "FR-EVT-MODEL-SELECTION-RECORDED";

/// One enumerable, labeled model entry projected from a [`ModelRegistration`].
///
/// Carries BOTH the per-boot identity (`model_id`, a re-minted UUIDv7) and the
/// stable cross-session anchor (`artifact_sha256`) so a consumer can group by
/// whichever identity its lifetime needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCatalogEntry {
    /// Per-boot UUIDv7 identity (re-minted every boot; keys routing this run).
    pub model_id: String,
    /// Human label for the model. Derived from `base_model_tag` today (the boot
    /// registration carries the operator's display name as the base model tag);
    /// exposed as a distinct field so a future dedicated display name can
    /// diverge without breaking consumers.
    pub display_name: String,
    /// The registration's base model tag (label; non-unique across entries).
    pub base_model_tag: String,
    /// STABLE cross-session anchor: the model artifact sha256 (hex). Survives
    /// restarts even though `model_id` is re-minted, so diagnostics/pickers can
    /// correlate "the same artifact" across boots.
    pub artifact_sha256: String,
    /// Which runtime adapter hosts the model (`llama_cpp` | `candle`).
    pub runtime_binding: String,
    /// READY state (master-spec §4.3.9.4.4): the model is loaded and routable
    /// this run. Derived from the registry's loaded-model marker.
    pub ready: bool,
}

/// Shared, enumerable, labeled view over the boot [`ModelRegistry`].
///
/// Holds the SAME `Arc<ModelRegistry>` that `LocalRouter` routes through, so the
/// catalog can never drift from the registry that actually dispatches. Cheap to
/// clone (an `Arc` bump).
#[derive(Clone)]
pub struct ModelCatalog {
    registry: Arc<ModelRegistry>,
}

impl ModelCatalog {
    /// Wraps a shared boot registry as an enumeration/label surface.
    pub fn from_registry(registry: Arc<ModelRegistry>) -> Arc<Self> {
        Arc::new(Self { registry })
    }

    /// An empty catalog (no configured local models). Deterministic empty-list
    /// behavior for the no-local-model boot path and for consumers/tests.
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            registry: Arc::new(ModelRegistry::default()),
        })
    }

    /// Enumerate every registered local model, labeled. Empty registry yields an
    /// empty `Vec` (never a panic). Ordering follows `ModelRegistry::list`
    /// (stable, model_id-sorted).
    pub fn list(&self) -> Vec<ModelCatalogEntry> {
        self.registry
            .list()
            .into_iter()
            .map(|reg| self.to_entry(reg))
            .collect()
    }

    /// The number of models the catalog can enumerate.
    pub fn len(&self) -> usize {
        self.registry.list().len()
    }

    /// Whether the catalog has no configured local models.
    pub fn is_empty(&self) -> bool {
        self.registry.list().is_empty()
    }

    /// Resolve a `model_id` (UUIDv7 string) to a human label. An unparseable or
    /// unknown id yields the stable [`UNKNOWN_MODEL_LABEL`] sentinel — never a
    /// panic, never an empty string.
    pub fn label_for(&self, model_id: &str) -> String {
        match self.lookup(model_id) {
            Some(reg) => reg.base_model_tag.as_str().to_string(),
            None => UNKNOWN_MODEL_LABEL.to_string(),
        }
    }

    /// The stable cross-session anchor (artifact sha256, hex) for a `model_id`,
    /// or `None` when the id is unknown. Consumers that must correlate a model
    /// across restarts key on this, not on the per-boot UUIDv7.
    pub fn stable_anchor(&self, model_id: &str) -> Option<String> {
        self.lookup(model_id).map(|reg| hex::encode(reg.sha256))
    }

    /// The full labeled entry for a `model_id`, or `None` when unknown.
    pub fn entry(&self, model_id: &str) -> Option<ModelCatalogEntry> {
        self.lookup(model_id).map(|reg| self.to_entry(reg))
    }

    /// Record a model-selection decision as an auditable EventLedger (Tier-1
    /// Flight Recorder business-event) event, per master-spec §4.3.9.4.4. This
    /// is DISTINCT from a launch/inference event: it captures the decision to
    /// select a model, not the act of running one. The stable cross-session
    /// anchor is recorded alongside the per-boot id so the decision remains
    /// correlatable across restarts. An unknown `model_id` is recorded with the
    /// sentinel label + `unknown` anchor rather than rejected, so the audit
    /// trail is never silently dropped.
    pub async fn record_selection_decision(
        &self,
        recorder: &dyn FlightRecorder,
        selected_model_id: &str,
        actor: &str,
        reason: &str,
    ) -> Result<(), RecorderError> {
        let anchor = self
            .stable_anchor(selected_model_id)
            .unwrap_or_else(|| "unknown".to_string());
        let label = self.label_for(selected_model_id);
        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            Uuid::now_v7(),
            json!({
                "fr_event": MODEL_SELECTION_FR_EVENT,
                "type": "model_selection_recorded",
                "selected_model_id": selected_model_id,
                "stable_anchor_sha256": anchor,
                "display_name": label,
                "actor": actor,
                "reason": reason,
            }),
        )
        .with_model_id(selected_model_id);
        recorder.record_event(event).await
    }

    fn lookup(&self, model_id: &str) -> Option<&ModelRegistration> {
        let uuid = Uuid::parse_str(model_id.trim()).ok()?;
        self.registry.lookup(ModelId::from(uuid))
    }

    fn to_entry(&self, reg: &ModelRegistration) -> ModelCatalogEntry {
        let tag = reg.base_model_tag.as_str().to_string();
        ModelCatalogEntry {
            model_id: reg.model_id.to_string(),
            display_name: tag.clone(),
            base_model_tag: tag,
            artifact_sha256: hex::encode(reg.sha256),
            runtime_binding: reg.runtime_binding.adapter_id().to_string(),
            ready: self.registry.is_loaded(reg.model_id),
        }
    }
}
