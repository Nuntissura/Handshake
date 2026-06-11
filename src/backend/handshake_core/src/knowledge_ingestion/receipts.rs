//! MT-085 ExtractionReceiptModel: typed receipts for every extraction
//! attempt — success, partial, failed, deferred, skipped, blocked.
//!
//! A receipt is per-ATTEMPT evidence (`knowledge_ingestion_receipts`, 0162):
//! extractor identity+version, typed error class, span/redaction counts, the
//! raw content hash at extraction time (MT-084 fidelity hash), duration, and
//! an EventLedger receipt ref. The per-source rollup
//! (`knowledge_sources.parser_status/extraction_status/
//! last_index_receipt_event_id`) stays in the storage layer; receipts are
//! the audit trail beneath it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{IngestionError, IngestionResult};

/// Outcome status of one extraction attempt.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionStatus {
    /// All extractable content produced spans.
    Success,
    /// Some content produced spans, some failed (never silent: the error
    /// class and detail say what was lost).
    Partial,
    /// Nothing usable was extracted.
    Failed,
    /// Extraction was not attempted now for a typed reason (backpressure).
    Deferred,
    /// Source intentionally not extracted (unsupported format).
    Skipped,
    /// Extraction was blocked by policy (secret preflight).
    Blocked,
}

impl ExtractionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Deferred => "deferred",
            Self::Skipped => "skipped",
            Self::Blocked => "blocked",
        }
    }
}

impl std::str::FromStr for ExtractionStatus {
    type Err = IngestionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "success" => Ok(Self::Success),
            "partial" => Ok(Self::Partial),
            "failed" => Ok(Self::Failed),
            "deferred" => Ok(Self::Deferred),
            "skipped" => Ok(Self::Skipped),
            "blocked" => Ok(Self::Blocked),
            other => Err(IngestionError::Validation(format!(
                "invalid extraction status: {other}"
            ))),
        }
    }
}

/// Typed error classes (MT-085 contract vocabulary: unsupported format,
/// missing text layer, OCR needed, plus the failure modes of the other
/// group MTs). String forms match the 0162 CHECK constraint.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IngestionErrorClass {
    UnsupportedFormat,
    NoTextLayer,
    OcrNeeded,
    Encrypted,
    MalformedCue,
    ParseError,
    Oversize,
    SecretBlocked,
    IoError,
    Internal,
}

impl IngestionErrorClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            Self::NoTextLayer => "NO_TEXT_LAYER",
            Self::OcrNeeded => "OCR_NEEDED",
            Self::Encrypted => "ENCRYPTED",
            Self::MalformedCue => "MALFORMED_CUE",
            Self::ParseError => "PARSE_ERROR",
            Self::Oversize => "OVERSIZE",
            Self::SecretBlocked => "SECRET_BLOCKED",
            Self::IoError => "IO_ERROR",
            Self::Internal => "INTERNAL",
        }
    }

    /// Classes that should land in the repair queue (MT-094): retryable or
    /// operator-repairable conditions, as opposed to permanent skips.
    pub fn is_repairable(&self) -> bool {
        !matches!(self, Self::UnsupportedFormat | Self::SecretBlocked)
    }
}

impl std::str::FromStr for IngestionErrorClass {
    type Err = IngestionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "UNSUPPORTED_FORMAT" => Ok(Self::UnsupportedFormat),
            "NO_TEXT_LAYER" => Ok(Self::NoTextLayer),
            "OCR_NEEDED" => Ok(Self::OcrNeeded),
            "ENCRYPTED" => Ok(Self::Encrypted),
            "MALFORMED_CUE" => Ok(Self::MalformedCue),
            "PARSE_ERROR" => Ok(Self::ParseError),
            "OVERSIZE" => Ok(Self::Oversize),
            "SECRET_BLOCKED" => Ok(Self::SecretBlocked),
            "IO_ERROR" => Ok(Self::IoError),
            "INTERNAL" => Ok(Self::Internal),
            other => Err(IngestionError::Validation(format!(
                "invalid ingestion error class: {other}"
            ))),
        }
    }
}

/// Durable row of `knowledge_ingestion_receipts`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExtractionReceipt {
    pub receipt_id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub ingestion_run_token: Option<String>,
    pub extractor_id: String,
    pub extractor_version: String,
    pub status: ExtractionStatus,
    pub error_class: Option<IngestionErrorClass>,
    pub error_detail: Option<Value>,
    pub spans_produced: i32,
    pub spans_failed: i32,
    pub redaction_count: i32,
    pub content_hash: String,
    pub duration_ms: i64,
    pub receipt_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Insert payload for [`ExtractionReceipt`].
#[derive(Clone, Debug)]
pub struct NewExtractionReceipt {
    pub workspace_id: String,
    pub source_id: String,
    pub ingestion_run_token: Option<String>,
    pub extractor_id: String,
    pub extractor_version: String,
    pub status: ExtractionStatus,
    pub error_class: Option<IngestionErrorClass>,
    pub error_detail: Option<Value>,
    pub spans_produced: i32,
    pub spans_failed: i32,
    pub redaction_count: i32,
    /// Raw fidelity hash (MT-084) of the content at extraction time.
    pub content_hash: String,
    pub duration_ms: i64,
}

impl NewExtractionReceipt {
    /// Typed shape validation mirroring the 0162 CHECK constraints, so
    /// callers fail fast before touching the database.
    pub fn validate(&self) -> IngestionResult<()> {
        if self.extractor_id.trim().is_empty() || self.extractor_version.trim().is_empty() {
            return Err(IngestionError::Validation(
                "extraction receipt requires extractor_id and extractor_version".to_string(),
            ));
        }
        let success = matches!(self.status, ExtractionStatus::Success);
        if success && self.error_class.is_some() {
            return Err(IngestionError::Validation(
                "success receipts must not carry an error class".to_string(),
            ));
        }
        if !success && self.error_class.is_none() {
            return Err(IngestionError::Validation(format!(
                "{} receipts must carry a typed error class",
                self.status.as_str()
            )));
        }
        if self.spans_produced < 0 || self.spans_failed < 0 || self.redaction_count < 0 {
            return Err(IngestionError::Validation(
                "receipt counts must be non-negative".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> NewExtractionReceipt {
        NewExtractionReceipt {
            workspace_id: "ws".into(),
            source_id: "KSRC-00000000000000000000000000000000".into(),
            ingestion_run_token: None,
            extractor_id: "markdown_note_v1".into(),
            extractor_version: "1".into(),
            status: ExtractionStatus::Success,
            error_class: None,
            error_detail: None,
            spans_produced: 3,
            spans_failed: 0,
            redaction_count: 0,
            content_hash: "a".repeat(64),
            duration_ms: 12,
        }
    }

    #[test]
    fn non_success_requires_error_class_and_success_forbids_it() {
        assert!(base().validate().is_ok());

        let mut failed = base();
        failed.status = ExtractionStatus::Failed;
        assert!(failed.validate().is_err(), "failed without class must fail");
        failed.error_class = Some(IngestionErrorClass::ParseError);
        assert!(failed.validate().is_ok());

        let mut success_with_class = base();
        success_with_class.error_class = Some(IngestionErrorClass::Internal);
        assert!(success_with_class.validate().is_err());
    }

    #[test]
    fn error_classes_round_trip_and_classify_repairability() {
        for class in [
            IngestionErrorClass::UnsupportedFormat,
            IngestionErrorClass::NoTextLayer,
            IngestionErrorClass::OcrNeeded,
            IngestionErrorClass::Encrypted,
            IngestionErrorClass::MalformedCue,
            IngestionErrorClass::ParseError,
            IngestionErrorClass::Oversize,
            IngestionErrorClass::SecretBlocked,
            IngestionErrorClass::IoError,
            IngestionErrorClass::Internal,
        ] {
            let parsed: IngestionErrorClass = class.as_str().parse().expect("round trip");
            assert_eq!(parsed, class);
        }
        assert!(IngestionErrorClass::NoTextLayer.is_repairable());
        assert!(IngestionErrorClass::Oversize.is_repairable());
        assert!(!IngestionErrorClass::UnsupportedFormat.is_repairable());
        assert!(!IngestionErrorClass::SecretBlocked.is_repairable());
    }
}
