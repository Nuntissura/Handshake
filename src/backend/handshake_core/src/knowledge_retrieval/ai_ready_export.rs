//! WP-KERNEL-009 MT-141 AIReadyEvidenceExportBridge.
//!
//! Folded WP-1-AIReady-Index-Evidence-Export-v1 intent: "Define bounded export
//! anchors for AI-ready index artifacts and rebuild/update evidence";
//! "preserve manifest, retention, provenance, backend-portability, and
//! reconstructability semantics"; "Context bundle and retrieval-debug evidence
//! must reuse the canonical AI-ready export dialect instead of inventing a
//! second export format." Spec 2.3.14 AI-Ready Data Architecture.
//!
//! This bridge produces a BOUNDED export manifest over a compiled context
//! bundle and its retrieval traces (committed tables 0141). It reuses the
//! AI-ready field dialect — `artifact` handle, `source_hashes`, `provenance`,
//! `retention`, `reconstructable` — so retrieval/debug evidence speaks the SAME
//! export contract as the rest of AI-Ready Data, rather than a parallel format.
//! The manifest is a projection (it cites authority rows by id + hash); it is
//! not a second copy of authority.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::storage::knowledge::{KnowledgeContextBundle, KnowledgeRetrievalTrace};

/// The retention class of an exported evidence manifest (AI-ready dialect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionClass {
    /// Evidence that must be retained for replay/audit (default for traces).
    Durable,
    /// Evidence safe to compact once superseded.
    Compactable,
}

impl RetentionClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::Compactable => "compactable",
        }
    }
}

/// A bounded AI-ready export manifest for retrieval evidence. Carries stable
/// handles + hashes, never the full bundle/trace payloads (bounded-export
/// contract). The `reconstructable` flag asserts whether the cited authority
/// rows are sufficient to rebuild the evidence — the folded-stub
/// reconstructability requirement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiReadyEvidenceManifest {
    /// Export dialect marker — the canonical AI-ready export contract.
    pub dialect: String,
    pub workspace_id: String,
    /// The bundle this manifest exports.
    pub bundle_id: String,
    /// The bundle content hash (the artifact handle into authority).
    pub bundle_content_hash: String,
    /// Hashes of the trace artifacts cited (one per trace).
    pub trace_ids: Vec<String>,
    /// Provenance: the kernel run + session that built the bundle.
    pub provenance: Value,
    pub retention: RetentionClass,
    /// True when the cited authority rows (bundle + traces, by id + hash) are
    /// sufficient to reconstruct this evidence without the manifest.
    pub reconstructable: bool,
    /// Bounded marker: this manifest does not embed full payloads.
    pub payload_bounded: bool,
}

/// Build a bounded export manifest for a compiled bundle and its traces. Uses
/// only authority-row handles + hashes; the manifest is reconstructable from
/// PostgreSQL because every cited row is addressable by its stable id.
pub fn build_evidence_manifest(
    bundle: &KnowledgeContextBundle,
    traces: &[KnowledgeRetrievalTrace],
) -> AiReadyEvidenceManifest {
    let trace_ids: Vec<String> = traces.iter().map(|t| t.trace_id.clone()).collect();
    // Reconstructable when the bundle has a content hash and every trace is
    // bound back to this bundle (so a replay can re-derive the evidence).
    let reconstructable = !bundle.context_hash.is_empty()
        && traces
            .iter()
            .all(|t| t.bundle_id.as_deref() == Some(bundle.bundle_id.as_str()));

    AiReadyEvidenceManifest {
        dialect: "ai_ready_evidence_export@1".to_string(),
        workspace_id: bundle.workspace_id.clone(),
        bundle_id: bundle.bundle_id.clone(),
        bundle_content_hash: bundle.context_hash.clone(),
        trace_ids,
        provenance: json!({
            "kernel_task_run_id": bundle.kernel_task_run_id,
            "session_run_id": bundle.session_run_id,
            "build_receipt_event_id": bundle.build_receipt_event_id,
            "created_at": bundle.created_at,
        }),
        retention: RetentionClass::Durable,
        reconstructable,
        payload_bounded: true,
    }
}

impl AiReadyEvidenceManifest {
    pub fn as_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::Value;

    fn bundle(hash: &str) -> KnowledgeContextBundle {
        KnowledgeContextBundle {
            bundle_id: format!("CTX-{}", &hash[..16]),
            workspace_id: "ws".to_string(),
            kernel_task_run_id: "ktr".to_string(),
            session_run_id: "sr".to_string(),
            allowed_context: Value::Null,
            context_hash: hash.to_string(),
            query_text: Some("q".to_string()),
            token_budget: Some(100),
            tokens_used: Some(50),
            build_receipt_event_id: Some("EVT-b".to_string()),
            created_at: Utc::now(),
        }
    }

    fn trace(trace_id: &str, bundle_id: Option<&str>) -> KnowledgeRetrievalTrace {
        KnowledgeRetrievalTrace {
            trace_id: trace_id.to_string(),
            workspace_id: "ws".to_string(),
            retrieval_mode: crate::storage::knowledge::KnowledgeRetrievalMode::DirectLoad,
            mode_reason: "exact id".to_string(),
            query_text: Some("q".to_string()),
            bundle_id: bundle_id.map(ToString::to_string),
            decisions: Value::Null,
            trace_receipt_event_id: Some("EVT-t".to_string()),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn manifest_uses_ai_ready_dialect_and_is_bounded() {
        let b = bundle(&"a".repeat(64));
        let manifest = build_evidence_manifest(&b, &[trace("KRT-1", Some(&b.bundle_id))]);
        assert_eq!(manifest.dialect, "ai_ready_evidence_export@1");
        assert!(manifest.payload_bounded);
        assert_eq!(manifest.retention, RetentionClass::Durable);
    }

    #[test]
    fn reconstructable_when_traces_bind_to_bundle() {
        let b = bundle(&"b".repeat(64));
        let manifest = build_evidence_manifest(&b, &[trace("KRT-1", Some(&b.bundle_id))]);
        assert!(manifest.reconstructable);
    }

    #[test]
    fn not_reconstructable_when_trace_unbound() {
        let b = bundle(&"c".repeat(64));
        let manifest = build_evidence_manifest(&b, &[trace("KRT-1", None)]);
        assert!(!manifest.reconstructable);
    }
}
