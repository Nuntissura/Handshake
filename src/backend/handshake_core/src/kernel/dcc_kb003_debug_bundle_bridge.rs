//! MT-070 Debug Bundle Bridge.
//!
//! Acceptance (MT-070.json): "diagnostics evidence is bounded and portable."
//!
//! Folds the existing diagnostics surface (see
//! `crate::diagnostics::DiagnosticSeverity` + `DiagnosticSource`) into a
//! KB003 evidence shape that:
//!
//! - is **bounded**: the bridge enforces a hard cap on the number of
//!   diagnostic entries it carries forward (default 256). Excess entries are
//!   summarized but not propagated.
//! - is **portable**: the bridge emits a typed JSON-serializable struct that
//!   does not reference live ledger handles; it can be exported alongside a
//!   sandbox artifact bundle and reopened on a different host.
//! - is **redaction-aware**: every entry carries an
//!   `exportable_by_default` flag so the
//!   [`crate::kernel::dcc_kb003_evidence_portability`] wrapper can partition
//!   the bundle on export.
//!
//! The bridge does not import the live `diagnostics` runtime ledger; it
//! takes a typed input that the runtime adapter constructs. This keeps the
//! kernel module compilable under the same feature gates as the rest of the
//! KB003 surface.
//!
//! Frontend renders via existing dcc-* IPC surface.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default bound on the number of entries the bridge carries into a KB003
/// evidence record. Aligns with the operator-readable budget on the DCC
/// debug surface.
pub const DEFAULT_DEBUG_BUNDLE_MAX_ENTRIES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DebugSeverity {
    Fatal,
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugDiagnosticEntryV1 {
    pub severity: DebugSeverity,
    pub source: String,
    pub message: String,
    pub recorded_at_utc: DateTime<Utc>,
    pub exportable_by_default: bool,
}

impl DebugDiagnosticEntryV1 {
    pub fn new(
        severity: DebugSeverity,
        source: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            source: source.into(),
            message: message.into(),
            recorded_at_utc: Utc::now(),
            exportable_by_default: true,
        }
    }

    pub fn confidential(mut self) -> Self {
        self.exportable_by_default = false;
        self
    }
}

/// Bounded, portable diagnostic bundle ready to fold into KB003 evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kb003DebugBundleV1 {
    pub schema_version: String,
    pub sandbox_run_id: String,
    pub max_entries: usize,
    pub truncated: bool,
    pub original_count: usize,
    pub entries: Vec<DebugDiagnosticEntryV1>,
}

impl Kb003DebugBundleV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.kb003_debug_bundle@1";

    /// Build a bridge from the runtime adapter's diagnostic list, enforcing
    /// the bound.
    pub fn from_entries(
        sandbox_run_id: impl Into<String>,
        entries: Vec<DebugDiagnosticEntryV1>,
        max_entries: usize,
    ) -> Self {
        let original_count = entries.len();
        let truncated = original_count > max_entries;
        let entries = if truncated {
            entries.into_iter().take(max_entries).collect()
        } else {
            entries
        };
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            sandbox_run_id: sandbox_run_id.into(),
            max_entries,
            truncated,
            original_count,
            entries,
        }
    }

    pub fn with_default_bound(
        sandbox_run_id: impl Into<String>,
        entries: Vec<DebugDiagnosticEntryV1>,
    ) -> Self {
        Self::from_entries(sandbox_run_id, entries, DEFAULT_DEBUG_BUNDLE_MAX_ENTRIES)
    }

    pub fn exportable_entries(&self) -> impl Iterator<Item = &DebugDiagnosticEntryV1> {
        self.entries.iter().filter(|e| e.exportable_by_default)
    }

    pub fn redacted_entries(&self) -> impl Iterator<Item = &DebugDiagnosticEntryV1> {
        self.entries.iter().filter(|e| !e.exportable_by_default)
    }

    /// Roundtrip-portable: the bundle has a stable, self-describing JSON
    /// form (no foreign handles).
    pub fn portable_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// DCC display row for the debug bundle bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003DebugBundleRowV1 {
    pub sandbox_run_id: String,
    pub entry_count: usize,
    pub truncated: bool,
    pub original_count: usize,
    pub fatal_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
}

impl DccKb003DebugBundleRowV1 {
    pub fn from_bundle(b: &Kb003DebugBundleV1) -> Self {
        let mut fatal = 0;
        let mut error = 0;
        let mut warning = 0;
        for e in &b.entries {
            match e.severity {
                DebugSeverity::Fatal => fatal += 1,
                DebugSeverity::Error => error += 1,
                DebugSeverity::Warning => warning += 1,
                _ => {}
            }
        }
        Self {
            sandbox_run_id: b.sandbox_run_id.clone(),
            entry_count: b.entries.len(),
            truncated: b.truncated,
            original_count: b.original_count,
            fatal_count: fatal,
            error_count: error,
            warning_count: warning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: usize) -> Vec<DebugDiagnosticEntryV1> {
        (0..n)
            .map(|i| {
                DebugDiagnosticEntryV1::new(
                    if i % 5 == 0 {
                        DebugSeverity::Error
                    } else {
                        DebugSeverity::Info
                    },
                    format!("plugin:p{i}"),
                    format!("msg {i}"),
                )
            })
            .collect()
    }

    #[test]
    fn bridge_is_bounded_to_max_entries() {
        let b = Kb003DebugBundleV1::from_entries("SBX-1", sample(500), 100);
        assert_eq!(b.entries.len(), 100);
        assert!(b.truncated);
        assert_eq!(b.original_count, 500);
    }

    #[test]
    fn bridge_is_portable_via_serde_roundtrip() {
        let b = Kb003DebugBundleV1::with_default_bound("SBX-1", sample(8));
        let json = b.portable_json().unwrap();
        let recovered: Kb003DebugBundleV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, b);
    }

    #[test]
    fn redaction_partitions_entries() {
        let mut entries = sample(4);
        entries[1] = entries[1].clone().confidential();
        let b = Kb003DebugBundleV1::from_entries("SBX-1", entries, 10);
        assert_eq!(b.exportable_entries().count(), 3);
        assert_eq!(b.redacted_entries().count(), 1);
    }

    #[test]
    fn dcc_row_counts_severities() {
        let entries = vec![
            DebugDiagnosticEntryV1::new(DebugSeverity::Fatal, "engine", "boom"),
            DebugDiagnosticEntryV1::new(DebugSeverity::Error, "engine", "bad"),
            DebugDiagnosticEntryV1::new(DebugSeverity::Warning, "engine", "warn"),
            DebugDiagnosticEntryV1::new(DebugSeverity::Info, "engine", "info"),
        ];
        let b = Kb003DebugBundleV1::with_default_bound("SBX-1", entries);
        let row = DccKb003DebugBundleRowV1::from_bundle(&b);
        assert_eq!(row.fatal_count, 1);
        assert_eq!(row.error_count, 1);
        assert_eq!(row.warning_count, 1);
        assert_eq!(row.entry_count, 4);
        assert!(!row.truncated);
    }
}
