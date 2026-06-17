//! Evidence portability wrapper used by MT-070/071/072/074 evidence
//! emitters when an operator exports evidence to share with another host.
//!
//! Acceptance theme (worker prompt): "redaction-aware export." Wraps the
//! validation `RedactionReport::partition_default_policy` so KB003 evidence
//! callers do not reach into the validation module directly.
//!
//! Frontend renders via existing dcc-* IPC surface.

use serde::{Deserialize, Serialize};

use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
use crate::kernel::validation::redaction_report::{RedactionEntry, RedactionReport};

/// Member of an export request: an artifact ref plus its class. The
/// portability wrapper partitions these against the artifact taxonomy's
/// default exportability policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableMemberV1 {
    pub artifact_ref: String,
    pub artifact_class: Kb003ArtifactClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003PortableExportV1 {
    pub schema_version: String,
    pub sandbox_run_id: String,
    pub exported: Vec<PortableMemberV1>,
    pub redacted_report: RedactionReport,
}

impl DccKb003PortableExportV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.kb003_portable_export@1";

    /// Partition `members` against the default redaction policy and produce
    /// the export + report. Non-exportable members are listed in the report
    /// with a stable reason so the operator can see what was withheld.
    pub fn partition(sandbox_run_id: impl Into<String>, members: Vec<PortableMemberV1>) -> Self {
        let raw: Vec<(String, Kb003ArtifactClass)> = members
            .into_iter()
            .map(|m| (m.artifact_ref, m.artifact_class))
            .collect();
        let (kept, redacted): (Vec<(String, Kb003ArtifactClass)>, Vec<RedactionEntry>) =
            RedactionReport::partition_default_policy(&raw);
        let exported = kept
            .into_iter()
            .map(|(artifact_ref, artifact_class)| PortableMemberV1 {
                artifact_ref,
                artifact_class,
            })
            .collect();
        let redacted_report = RedactionReport::new(redacted);
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            sandbox_run_id: sandbox_run_id.into(),
            exported,
            redacted_report,
        }
    }

    /// Round-trip portable JSON form.
    pub fn portable_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshots_are_redacted_by_default() {
        // SandboxScreenshot has exportable_by_default = false.
        let members = vec![
            PortableMemberV1 {
                artifact_ref: "ART-log-1".into(),
                artifact_class: Kb003ArtifactClass::SandboxLog,
            },
            PortableMemberV1 {
                artifact_ref: "ART-shot-1".into(),
                artifact_class: Kb003ArtifactClass::SandboxScreenshot,
            },
            PortableMemberV1 {
                artifact_ref: "ART-manifest-1".into(),
                artifact_class: Kb003ArtifactClass::SandboxManifest,
            },
        ];
        let export = DccKb003PortableExportV1::partition("SBX-1", members);
        assert!(
            export
                .exported
                .iter()
                .any(|m| m.artifact_class == Kb003ArtifactClass::SandboxLog)
        );
        assert!(
            export
                .exported
                .iter()
                .any(|m| m.artifact_class == Kb003ArtifactClass::SandboxManifest)
        );
        // Screenshot is withheld and reported.
        assert!(
            !export
                .exported
                .iter()
                .any(|m| m.artifact_class == Kb003ArtifactClass::SandboxScreenshot)
        );
        assert!(
            export
                .redacted_report
                .entries
                .iter()
                .any(|e| e.artifact_class == Kb003ArtifactClass::SandboxScreenshot)
        );
    }

    #[test]
    fn empty_input_yields_empty_export_and_empty_report() {
        let export = DccKb003PortableExportV1::partition("SBX-1", vec![]);
        assert!(export.exported.is_empty());
        assert!(export.redacted_report.entries.is_empty());
    }

    #[test]
    fn portable_via_serde_roundtrip() {
        let export = DccKb003PortableExportV1::partition(
            "SBX-1",
            vec![PortableMemberV1 {
                artifact_ref: "ART-1".into(),
                artifact_class: Kb003ArtifactClass::SandboxLog,
            }],
        );
        let json = export.portable_json().unwrap();
        let recovered: DccKb003PortableExportV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, export);
    }
}
