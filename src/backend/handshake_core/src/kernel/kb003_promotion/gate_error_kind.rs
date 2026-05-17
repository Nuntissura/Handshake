//! H4 fix (KB003 remediation): normalised storage-error taxonomy used by
//! `PromotionRejectionReason::canonical_hash_projection`.
//!
//! `PromotionRejectionReason::PostgresFailure` carries `storage_error: String`
//! which contains transient line numbers, connection-id fragments, server
//! timestamps, and other wobble that varies between retries. Including that
//! raw string in `payload_hash` caused two retries of the SAME logical
//! failure to produce DIFFERENT hashes and thus false
//! `Kb003StorageError::IdempotencyConflict` reports.
//!
//! Solution: bucket every storage error into a small, stable enum. A
//! deadlock retried looks like a deadlock; a unique-violation retried looks
//! like a unique-violation; raw text differences are absorbed.
//!
//! The full raw error string is still surfaced for observability via
//! `PromotionReceiptV1::storage_error_detail` (non-hashed).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormalisedStorageErrorKind {
    Deadlock,
    UniqueViolation,
    ConnectionLost,
    SerializationFailure,
    IntegrityViolation,
    Unknown,
}

impl NormalisedStorageErrorKind {
    /// Classify by a pre-extracted error message. Same algorithm as
    /// `classify_storage_error` but avoids the `&dyn std::error::Error`
    /// allocation when the caller already has a `String`.
    pub fn classify_message(msg: &str) -> Self {
        let lower = msg.to_ascii_lowercase();
        if lower.contains("deadlock") {
            return Self::Deadlock;
        }
        if lower.contains("unique") || lower.contains("duplicate key") {
            return Self::UniqueViolation;
        }
        if lower.contains("connection")
            && (lower.contains("lost") || lower.contains("closed") || lower.contains("refused"))
        {
            return Self::ConnectionLost;
        }
        if lower.contains("serialization") || lower.contains("could not serialize") {
            return Self::SerializationFailure;
        }
        if lower.contains("foreign key")
            || lower.contains("integrity")
            || lower.contains("not-null")
        {
            return Self::IntegrityViolation;
        }
        Self::Unknown
    }
}

/// Classify a `std::error::Error` into a `NormalisedStorageErrorKind`. Kept
/// as a free function (per H4 contract) and as a method on the enum so
/// callers can pick whichever form is more convenient.
pub fn classify_storage_error(e: &dyn std::error::Error) -> NormalisedStorageErrorKind {
    NormalisedStorageErrorKind::classify_message(&e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadlock_messages_classify_consistently() {
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("deadlock detected"),
            NormalisedStorageErrorKind::Deadlock
        );
        assert_eq!(
            NormalisedStorageErrorKind::classify_message(
                "Postgres: DEADLOCK on transaction 4711"
            ),
            NormalisedStorageErrorKind::Deadlock
        );
    }

    #[test]
    fn unique_violation_messages_classify_consistently() {
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("duplicate key value violates unique constraint"),
            NormalisedStorageErrorKind::UniqueViolation
        );
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("unique constraint failed"),
            NormalisedStorageErrorKind::UniqueViolation
        );
    }

    #[test]
    fn connection_lost_classify() {
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("connection lost to server"),
            NormalisedStorageErrorKind::ConnectionLost
        );
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("connection refused"),
            NormalisedStorageErrorKind::ConnectionLost
        );
    }

    #[test]
    fn serialization_failure_classify() {
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("could not serialize access"),
            NormalisedStorageErrorKind::SerializationFailure
        );
    }

    #[test]
    fn integrity_violation_classify() {
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("foreign key violation"),
            NormalisedStorageErrorKind::IntegrityViolation
        );
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("integrity constraint"),
            NormalisedStorageErrorKind::IntegrityViolation
        );
    }

    #[test]
    fn unknown_classify_default() {
        assert_eq!(
            NormalisedStorageErrorKind::classify_message("planet exploded"),
            NormalisedStorageErrorKind::Unknown
        );
    }

    // The whole point of H4: two retries of the SAME logical failure with
    // DIFFERENT raw text wobble must classify identically.
    #[test]
    fn retries_with_text_wobble_classify_identically() {
        let first = "deadlock detected at line 412, tx 991";
        let second = "Deadlock Detected at line 538, tx 1004";
        assert_eq!(
            NormalisedStorageErrorKind::classify_message(first),
            NormalisedStorageErrorKind::classify_message(second)
        );
    }
}
