//! MT-039: Redaction report.
//!
//! Acceptance: default export is redacted and denied artifacts are listed.
//! `RedactionReport` is an exportable companion to the artifact bundle that
//! enumerates which artifacts were redacted (kept out of the export) and why.
//! It is classified as `Kb003ArtifactClass::RedactionNote` whose taxonomy
//! row has `exportable_by_default = false`; the report itself is what
//! operators consult to know what was withheld.

use serde::{Deserialize, Serialize};

use crate::kernel::kb003_artifact_classes::{metadata_for, Kb003ArtifactClass};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionEntry {
    pub artifact_ref: String,
    pub artifact_class: Kb003ArtifactClass,
    pub reason: String,
}

impl RedactionEntry {
    pub fn new(
        artifact_ref: impl Into<String>,
        artifact_class: Kb003ArtifactClass,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            artifact_ref: artifact_ref.into(),
            artifact_class,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionReport {
    pub artifact_class: Kb003ArtifactClass,
    pub entries: Vec<RedactionEntry>,
}

impl RedactionReport {
    /// Build a redaction report. Any artifact whose taxonomy row sets
    /// `exportable_by_default = false` and which is therefore withheld from
    /// the default export should be listed here with a typed reason.
    pub fn new(entries: Vec<RedactionEntry>) -> Self {
        Self {
            artifact_class: Kb003ArtifactClass::RedactionNote,
            entries,
        }
    }

    /// Filter a candidate export list against the default exportability
    /// policy. Returns (exported, redacted). Redacted entries are returned
    /// as `RedactionEntry` records so the caller can build a report.
    pub fn partition_default_policy(
        members: &[(String, Kb003ArtifactClass)],
    ) -> (Vec<(String, Kb003ArtifactClass)>, Vec<RedactionEntry>) {
        let mut exported = Vec::new();
        let mut redacted = Vec::new();
        for (artifact_ref, class) in members {
            let exportable = metadata_for(*class)
                .map(|m| m.exportable_by_default)
                .unwrap_or(false);
            if exportable {
                exported.push((artifact_ref.clone(), *class));
            } else {
                redacted.push(RedactionEntry::new(
                    artifact_ref.clone(),
                    *class,
                    "artifact class non-exportable by default policy",
                ));
            }
        }
        (exported, redacted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_export_is_redacted_and_lists_denied_artifacts() {
        let candidates = vec![
            ("art://log/1".to_string(), Kb003ArtifactClass::SandboxLog),
            (
                "art://shot/1".to_string(),
                Kb003ArtifactClass::SandboxScreenshot,
            ),
            ("art://red/1".to_string(), Kb003ArtifactClass::RedactionNote),
            (
                "art://report/1".to_string(),
                Kb003ArtifactClass::ValidationReport,
            ),
        ];
        let (exported, redacted) = RedactionReport::partition_default_policy(&candidates);
        // Logs and reports exportable by default; screenshots and redaction
        // notes are not.
        let exported_refs: Vec<&str> = exported.iter().map(|(r, _)| r.as_str()).collect();
        assert!(exported_refs.contains(&"art://log/1"));
        assert!(exported_refs.contains(&"art://report/1"));
        assert!(!exported_refs.contains(&"art://shot/1"));
        assert!(!exported_refs.contains(&"art://red/1"));

        let report = RedactionReport::new(redacted);
        assert_eq!(report.artifact_class, Kb003ArtifactClass::RedactionNote);
        // Denied artifacts are listed.
        let redacted_refs: Vec<&str> = report
            .entries
            .iter()
            .map(|e| e.artifact_ref.as_str())
            .collect();
        assert!(redacted_refs.contains(&"art://shot/1"));
        assert!(redacted_refs.contains(&"art://red/1"));
        assert_eq!(report.entries.len(), 2);
    }

    #[test]
    fn empty_candidates_yields_empty_report() {
        let (exported, redacted) = RedactionReport::partition_default_policy(&[]);
        assert!(exported.is_empty());
        assert!(redacted.is_empty());
        let report = RedactionReport::new(redacted);
        assert!(report.entries.is_empty());
    }
}
