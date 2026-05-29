//! MT-071 MCP / Mechanical Engine (MEX) Evidence Export Bridge.
//!
//! Acceptance (MT-071.json): "MCP/MEX evidence does not use ad hoc bundle
//! schema."
//!
//! Tool / mechanical engine evidence (MCP call traces, ACP relay receipts,
//! deterministic engine runs) is folded into a typed KB003 evidence record
//! so downstream consumers (validation reports, promotion bundles) never
//! see an ad-hoc map. The bridge:
//!
//! - declares a typed [`MexEvidenceKind`] (mcp call / acp relay / engine
//!   determinism / engine probe) so callers cannot invent new categories
//!   without extending the enum.
//! - carries every entry with a stable artifact-class binding
//!   ([`Kb003ArtifactClass::SandboxManifest`]) so the redaction-aware
//!   exporter (MT-072 portability helper) handles it identically to other
//!   sandbox manifests.
//! - is portable via `serde_json` round-trip.
//!
//! Frontend renders via existing dcc-* IPC surface.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MexEvidenceKind {
    /// Model Context Protocol tool call trace.
    McpCallTrace,
    /// ACP broker mechanical relay receipt.
    AcpRelayReceipt,
    /// Deterministic engine run record (e.g. check runner output).
    EngineDeterminismRecord,
    /// Engine probe / capability discovery output.
    EngineProbe,
}

impl MexEvidenceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::McpCallTrace => "MCP_CALL_TRACE",
            Self::AcpRelayReceipt => "ACP_RELAY_RECEIPT",
            Self::EngineDeterminismRecord => "ENGINE_DETERMINISM_RECORD",
            Self::EngineProbe => "ENGINE_PROBE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MexEvidenceItemV1 {
    pub kind: MexEvidenceKind,
    pub artifact_ref: String,
    pub artifact_class: Kb003ArtifactClass,
    pub tool_or_engine_id: String,
    pub recorded_at_utc: DateTime<Utc>,
    pub redacted: bool,
}

impl MexEvidenceItemV1 {
    pub fn new(
        kind: MexEvidenceKind,
        artifact_ref: impl Into<String>,
        tool_or_engine_id: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            artifact_ref: artifact_ref.into(),
            // All MEX evidence is manifest-class so the redaction policy
            // matches sandbox manifests by default.
            artifact_class: Kb003ArtifactClass::SandboxManifest,
            tool_or_engine_id: tool_or_engine_id.into(),
            recorded_at_utc: Utc::now(),
            redacted: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kb003MexEvidenceV1 {
    pub schema_version: String,
    pub sandbox_run_id: String,
    pub items: Vec<MexEvidenceItemV1>,
}

impl Kb003MexEvidenceV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.kb003_mex_evidence@1";

    pub fn new(sandbox_run_id: impl Into<String>, items: Vec<MexEvidenceItemV1>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            sandbox_run_id: sandbox_run_id.into(),
            items,
        }
    }

    pub fn items_for(&self, kind: MexEvidenceKind) -> impl Iterator<Item = &MexEvidenceItemV1> {
        self.items.iter().filter(move |i| i.kind == kind)
    }

    pub fn portable_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_uses_typed_kind_not_ad_hoc_strings() {
        let item = MexEvidenceItemV1::new(MexEvidenceKind::McpCallTrace, "ART-1", "tool:fs.read");
        assert_eq!(item.kind, MexEvidenceKind::McpCallTrace);
        assert_eq!(item.artifact_class, Kb003ArtifactClass::SandboxManifest);
    }

    #[test]
    fn evidence_is_portable_via_serde_roundtrip() {
        let bundle = Kb003MexEvidenceV1::new(
            "SBX-1",
            vec![
                MexEvidenceItemV1::new(MexEvidenceKind::McpCallTrace, "ART-1", "tool:fs.read"),
                MexEvidenceItemV1::new(MexEvidenceKind::AcpRelayReceipt, "ART-2", "broker:acp"),
                MexEvidenceItemV1::new(
                    MexEvidenceKind::EngineDeterminismRecord,
                    "ART-3",
                    "engine:checkrunner",
                ),
            ],
        );
        let json = bundle.portable_json().unwrap();
        let recovered: Kb003MexEvidenceV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, bundle);
    }

    #[test]
    fn filter_by_kind_returns_only_matching_items() {
        let bundle = Kb003MexEvidenceV1::new(
            "SBX-1",
            vec![
                MexEvidenceItemV1::new(MexEvidenceKind::McpCallTrace, "ART-1", "x"),
                MexEvidenceItemV1::new(MexEvidenceKind::McpCallTrace, "ART-2", "x"),
                MexEvidenceItemV1::new(MexEvidenceKind::AcpRelayReceipt, "ART-3", "y"),
            ],
        );
        assert_eq!(bundle.items_for(MexEvidenceKind::McpCallTrace).count(), 2);
        assert_eq!(
            bundle.items_for(MexEvidenceKind::AcpRelayReceipt).count(),
            1
        );
    }
}
