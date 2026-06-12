//! WP-KERNEL-009 MT-142 ContextPackRecorderBridge.
//!
//! Folded WP-1-ContextPack-Recorder-Visibility-v1 intent: "ContextPack build,
//! reuse, refresh, freshness allow/deny, and require-rebuild decisions are
//! first-class Flight Recorder evidence"; "pack hash, source hash, freshness
//! policy, and job/spec-router/debug-bundle linkage must be stable and bounded";
//! "payloads must not dump full retrieval context by default". Spec 2.6.6.7.14.5
//! `ContextPackRecord` (pack_id, target, source_hashes, builder, version).
//!
//! This bridge emits BOUNDED, recorder-visible EventLedger receipts for the
//! ContextPack lifecycle. It records the decision (build / reuse / refresh /
//! freshness allow|deny / require-rebuild), the pack hash, the source hashes,
//! and the freshness policy — but NEVER the full pack payload (the spec/red-team
//! control against leaking retrieval context). Authority is the EventLedger
//! receipt; the pack payload itself lives elsewhere.
//!
//! Receipts reuse the committed `ContextBundleRecorded` EventLedger type with a
//! `context_pack_decision` payload kind, so no new kernel event variant is
//! introduced (the retrieval group does not edit `kernel/mod.rs`).

use serde_json::{json, Value};

use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::{Database, StorageError, StorageResult};

/// A recorder-visible ContextPack lifecycle decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPackDecision {
    /// A new pack was built.
    Build,
    /// An existing fresh pack was reused (cache hit).
    Reuse,
    /// A pack was refreshed (rebuilt because sources changed).
    Refresh,
    /// Freshness check allowed reuse.
    FreshnessAllow,
    /// Freshness check denied reuse (sources newer than the pack).
    FreshnessDeny,
    /// A rebuild is required before the pack may be used.
    RequireRebuild,
}

impl ContextPackDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Reuse => "reuse",
            Self::Refresh => "refresh",
            Self::FreshnessAllow => "freshness_allow",
            Self::FreshnessDeny => "freshness_deny",
            Self::RequireRebuild => "require_rebuild",
        }
    }
}

/// The bounded, recorder-visible metadata of one ContextPack decision. This is
/// exactly what is persisted — no full pack payload, only stable hashes +
/// bounded policy metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextPackDecisionRecord {
    pub decision: ContextPackDecision,
    /// Stable pack id (spec ContextPackRecord.pack_id).
    pub pack_id: String,
    /// What the pack is about (entity/source ref string).
    pub target_ref: String,
    /// Stable pack-artifact hash (bounded — a hash, not the artifact).
    pub pack_hash: String,
    /// Hashes of the underlying sources at pack build/check time (bounded).
    pub source_hashes: Vec<String>,
    /// Freshness policy name applied (e.g. `reuse_if_sources_unchanged`).
    pub freshness_policy: String,
    /// Linkage to the job / spec-router flow that triggered this (correlation).
    pub linkage: Option<String>,
}

impl ContextPackDecisionRecord {
    /// The bounded EventLedger payload. Carries the decision + hashes + policy,
    /// explicitly NOT the pack content.
    pub fn to_bounded_payload(&self) -> Value {
        json!({
            "kind": "context_pack_decision",
            "decision": self.decision.as_str(),
            "pack_id": self.pack_id,
            "target_ref": self.target_ref,
            "pack_hash": self.pack_hash,
            "source_hashes": self.source_hashes,
            "freshness_policy": self.freshness_policy,
            "linkage": self.linkage,
            // Explicit marker that no full retrieval context is dumped here.
            "payload_bounded": true,
        })
    }
}

/// Emit a recorder-visible ContextPack decision receipt to the EventLedger.
/// Returns the receipt event id (the stable evidence handle).
pub async fn record_context_pack_decision(
    db: &PostgresDatabase,
    actor: KernelActor,
    kernel_task_run_id: &str,
    session_run_id: &str,
    record: &ContextPackDecisionRecord,
) -> StorageResult<String> {
    if record.pack_hash.trim().is_empty() {
        return Err(StorageError::Validation(
            "context pack decision requires a stable pack_hash",
        ));
    }
    let mut builder = NewKernelEvent::builder(
        kernel_task_run_id.to_string(),
        session_run_id.to_string(),
        KernelEventType::ContextBundleRecorded,
        actor,
    )
    .aggregate("context_pack", record.pack_id.as_str())
    .source_component("knowledge_retrieval_context_pack_recorder")
    .payload(record.to_bounded_payload());
    if let Some(linkage) = &record.linkage {
        builder = builder.correlation_id(linkage.clone());
    }
    let event = builder
        .build()
        .map_err(|_| StorageError::Validation("context pack receipt build failed"))?;
    let stored = db.append_kernel_event(event).await?;
    Ok(stored.event_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_strings_are_stable() {
        assert_eq!(
            ContextPackDecision::FreshnessDeny.as_str(),
            "freshness_deny"
        );
        assert_eq!(
            ContextPackDecision::RequireRebuild.as_str(),
            "require_rebuild"
        );
        assert_eq!(ContextPackDecision::Reuse.as_str(), "reuse");
    }

    #[test]
    fn bounded_payload_excludes_full_context_and_marks_bounded() {
        let record = ContextPackDecisionRecord {
            decision: ContextPackDecision::Reuse,
            pack_id: "PACK-1".to_string(),
            target_ref: "entity:E1".to_string(),
            pack_hash: "deadbeef".to_string(),
            source_hashes: vec!["s1".to_string(), "s2".to_string()],
            freshness_policy: "reuse_if_sources_unchanged".to_string(),
            linkage: Some("job-42".to_string()),
        };
        let payload = record.to_bounded_payload();
        assert_eq!(payload["payload_bounded"], true);
        assert_eq!(payload["decision"], "reuse");
        assert_eq!(payload["pack_hash"], "deadbeef");
        assert_eq!(payload["source_hashes"].as_array().unwrap().len(), 2);
        // No "context" / "payload" full-content key is present.
        assert!(payload.get("context").is_none());
        assert!(payload.get("pack_payload").is_none());
    }
}
