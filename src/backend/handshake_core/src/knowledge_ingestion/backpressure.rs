//! MT-092 LargeFileBackpressure: typed size/line thresholds with explicit
//! deferral — an oversize source is DEFERRED with a reason receipt
//! (`deferred` / `OVERSIZE`), never loaded whole into extraction and never
//! silently skipped.
//!
//! Config surface: [`IngestionLimits`] with compiled defaults; the API run
//! trigger may override per request. Per-kind byte ceilings exist because a
//! 15 MiB PDF is normal while a 15 MiB markdown file is pathological.
//! The engine checks limits from file METADATA (size) before reading file
//! content, so an oversize file never occupies memory (no-OOM guarantee).

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::kinds::IngestionSourceKind;

/// Size/line thresholds for ingestion.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IngestionLimits {
    /// Default per-file byte ceiling.
    pub max_bytes: u64,
    /// Byte ceiling for PDFs (binary, page-structured: higher).
    pub max_pdf_bytes: u64,
    /// Line ceiling for text sources.
    pub max_lines: u64,
}

impl Default for IngestionLimits {
    fn default() -> Self {
        Self {
            // 2 MiB of text is ~500k tokens — far beyond a useful source.
            max_bytes: 2 * 1024 * 1024,
            // PDFs carry fonts/images around their text layer.
            max_pdf_bytes: 20 * 1024 * 1024,
            max_lines: 50_000,
        }
    }
}

/// Typed deferral verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackpressureDeferral {
    pub reason: BackpressureReason,
    pub limit: u64,
    pub actual: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackpressureReason {
    OversizeBytes,
    OversizeLines,
}

impl BackpressureReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OversizeBytes => "oversize_bytes",
            Self::OversizeLines => "oversize_lines",
        }
    }
}

impl BackpressureDeferral {
    /// Receipt `error_detail` payload for the deferral.
    pub fn detail_json(&self) -> serde_json::Value {
        json!({
            "reason": self.reason.as_str(),
            "limit": self.limit,
            "actual": self.actual,
        })
    }
}

impl IngestionLimits {
    /// Byte ceiling for a kind.
    pub fn byte_limit_for(&self, kind: IngestionSourceKind) -> u64 {
        match kind {
            IngestionSourceKind::Pdf => self.max_pdf_bytes,
            _ => self.max_bytes,
        }
    }

    /// Metadata-stage check (BEFORE reading content): file size only.
    pub fn check_size(
        &self,
        kind: IngestionSourceKind,
        size_bytes: u64,
    ) -> Result<(), BackpressureDeferral> {
        let limit = self.byte_limit_for(kind);
        if size_bytes > limit {
            return Err(BackpressureDeferral {
                reason: BackpressureReason::OversizeBytes,
                limit,
                actual: size_bytes,
            });
        }
        Ok(())
    }

    /// Content-stage check for text sources: line count.
    pub fn check_lines(&self, line_count: u64) -> Result<(), BackpressureDeferral> {
        if line_count > self.max_lines {
            return Err(BackpressureDeferral {
                reason: BackpressureReason::OversizeLines,
                limit: self.max_lines,
                actual: line_count,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_per_kind_limits() {
        let limits = IngestionLimits::default();
        assert_eq!(
            limits.byte_limit_for(IngestionSourceKind::MarkdownText),
            2 * 1024 * 1024
        );
        assert_eq!(
            limits.byte_limit_for(IngestionSourceKind::Pdf),
            20 * 1024 * 1024
        );

        assert!(limits
            .check_size(IngestionSourceKind::MarkdownText, 1024)
            .is_ok());
        let deferral = limits
            .check_size(IngestionSourceKind::MarkdownText, 3 * 1024 * 1024)
            .expect_err("oversize must defer");
        assert_eq!(deferral.reason, BackpressureReason::OversizeBytes);
        assert_eq!(deferral.limit, 2 * 1024 * 1024);
        assert_eq!(deferral.actual, 3 * 1024 * 1024);
        // A 3 MiB PDF is fine.
        assert!(limits
            .check_size(IngestionSourceKind::Pdf, 3 * 1024 * 1024)
            .is_ok());
    }

    #[test]
    fn line_limits_defer_typed() {
        let limits = IngestionLimits::default();
        assert!(limits.check_lines(100).is_ok());
        let deferral = limits.check_lines(60_001).expect_err("too many lines");
        assert_eq!(deferral.reason, BackpressureReason::OversizeLines);
        let detail = deferral.detail_json();
        assert_eq!(detail["reason"], "oversize_lines");
        assert_eq!(detail["actual"], 60_001);
    }
}
