//! MT-038: Visual evidence attachment.
//!
//! Acceptance: GUI reports can reference screenshots, DOM dumps, and log
//! evidence. The attachment record carries a structured `EvidenceKind`
//! discriminator plus an artifact reference. The default (un)exportable
//! posture is inherited from `Kb003ArtifactMetadata`
//! (`SandboxScreenshot` is *not* exportable by default — confidential).

use serde::{Deserialize, Serialize};

use crate::kernel::kb003_artifact_classes::{Kb003ArtifactClass, metadata_for};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceKind {
    Screenshot,
    DomDump,
    Log,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualEvidenceItem {
    pub kind: EvidenceKind,
    pub artifact_ref: String,
    pub artifact_class: Kb003ArtifactClass,
    pub exportable_by_default: bool,
    pub note: Option<String>,
}

impl VisualEvidenceItem {
    pub fn screenshot(artifact_ref: impl Into<String>) -> Self {
        Self::from_kind(
            EvidenceKind::Screenshot,
            Kb003ArtifactClass::SandboxScreenshot,
            artifact_ref,
        )
    }

    pub fn dom_dump(artifact_ref: impl Into<String>) -> Self {
        Self::from_kind(
            EvidenceKind::DomDump,
            Kb003ArtifactClass::SandboxManifest,
            artifact_ref,
        )
    }

    pub fn log(artifact_ref: impl Into<String>) -> Self {
        Self::from_kind(
            EvidenceKind::Log,
            Kb003ArtifactClass::SandboxLog,
            artifact_ref,
        )
    }

    fn from_kind(
        kind: EvidenceKind,
        artifact_class: Kb003ArtifactClass,
        artifact_ref: impl Into<String>,
    ) -> Self {
        let exportable_by_default = metadata_for(artifact_class)
            .map(|m| m.exportable_by_default)
            .unwrap_or(false);
        Self {
            kind,
            artifact_ref: artifact_ref.into(),
            artifact_class,
            exportable_by_default,
            note: None,
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VisualEvidenceAttachment {
    pub items: Vec<VisualEvidenceItem>,
}

impl VisualEvidenceAttachment {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, item: VisualEvidenceItem) {
        self.items.push(item);
    }
    pub fn has_screenshot(&self) -> bool {
        self.items
            .iter()
            .any(|i| i.kind == EvidenceKind::Screenshot)
    }
    pub fn has_dom_or_log(&self) -> bool {
        self.items
            .iter()
            .any(|i| matches!(i.kind, EvidenceKind::DomDump | EvidenceKind::Log))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_report_can_carry_screenshot_dom_and_log() {
        let mut a = VisualEvidenceAttachment::new();
        a.push(VisualEvidenceItem::screenshot("art://screen/abc.png"));
        a.push(VisualEvidenceItem::dom_dump("art://dom/abc.json"));
        a.push(VisualEvidenceItem::log("art://log/abc.txt"));
        assert!(a.has_screenshot());
        assert!(a.has_dom_or_log());
        assert_eq!(a.items.len(), 3);
    }

    #[test]
    fn screenshot_inherits_non_exportable_default() {
        let s = VisualEvidenceItem::screenshot("art://x.png");
        // KB003 artifact taxonomy: screenshots are export-gated.
        assert!(!s.exportable_by_default);
    }

    #[test]
    fn log_inherits_exportable_default() {
        let l = VisualEvidenceItem::log("art://x.log");
        assert!(l.exportable_by_default);
    }
}
