//! MT-121: Distillation session-close flag wiring.
//!
//! Per AC-DISTILL-OPT-IN + operator decision Q-DISTILL-CORPUS:
//! `governed_sessions.distill_corpus` is a per-session BOOLEAN that the
//! operator sets at session close. Default is `false`. The
//! [`SessionFlagStore`] trait abstracts the storage so the concrete
//! Postgres `ALTER TABLE ... ADD COLUMN distill_corpus BOOLEAN NOT NULL
//! DEFAULT false` migration + sqlx impl can land in a follow-on without
//! touching the lib semantics; the in-memory impl in this module backs
//! the unit tests.
//!
//! Operator-signature is required (HBR-INT-006: governance obligation
//! on session metadata writes). The signature is opaque to this
//! module; it is forwarded into the [`SessionFlagWriteRecord`] for the
//! flight recorder to audit.
//!
//! Adult-production discipline (GLOBAL-PRODUCTION-002..009): the
//! session-close UI surface (rendered by cluster-X follow-on) shows
//! the technical statement of consent only — "Include this session in
//! distillation corpus" — and never moralises, censors, or rewords.

use std::collections::HashMap;
use std::sync::RwLock;

use thiserror::Error;

/// One persisted toggle. Stored under (session_id) -> flag value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistillSessionFlag {
    pub session_id: String,
    pub distill_corpus: bool,
    pub updated_at_utc: String,
    pub updated_by_operator_signature: String,
}

/// Audit record forwarded to the flight recorder after a write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFlagWriteRecord {
    pub session_id: String,
    pub previous_flag: bool,
    pub new_flag: bool,
    pub operator_signature: String,
    pub written_at_utc: String,
}

#[derive(Debug, Error)]
pub enum SessionFlagError {
    #[error("session_id must not be empty")]
    EmptySessionId,
    #[error("operator_signature must not be empty (HBR-INT-006 governance obligation)")]
    EmptySignature,
    #[error("session flag storage error: {0}")]
    Storage(String),
}

/// Storage abstraction. Concrete impls:
/// - [`InMemorySessionFlagStore`] (in-module, used by unit tests).
/// - PostgresSessionFlagStore (follow-on; reads/writes
///   `governed_sessions.distill_corpus`).
pub trait SessionFlagStore {
    fn read(&self, session_id: &str) -> Result<Option<DistillSessionFlag>, SessionFlagError>;
    fn write(&self, flag: DistillSessionFlag) -> Result<(), SessionFlagError>;
}

/// In-memory impl backing unit tests + early integration.
pub struct InMemorySessionFlagStore {
    state: RwLock<HashMap<String, DistillSessionFlag>>,
}

impl Default for InMemorySessionFlagStore {
    fn default() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }
}

impl SessionFlagStore for InMemorySessionFlagStore {
    fn read(&self, session_id: &str) -> Result<Option<DistillSessionFlag>, SessionFlagError> {
        let guard = self
            .state
            .read()
            .map_err(|err| SessionFlagError::Storage(format!("read lock poisoned: {err}")))?;
        Ok(guard.get(session_id).cloned())
    }

    fn write(&self, flag: DistillSessionFlag) -> Result<(), SessionFlagError> {
        let mut guard = self
            .state
            .write()
            .map_err(|err| SessionFlagError::Storage(format!("write lock poisoned: {err}")))?;
        guard.insert(flag.session_id.clone(), flag);
        Ok(())
    }
}

/// Default-deny: if the session has no row, the flag is `false`.
/// MT-119 `CorpusExtractor` gates extraction on this value.
pub fn get_distill_flag(
    store: &dyn SessionFlagStore,
    session_id: &str,
) -> Result<bool, SessionFlagError> {
    if session_id.trim().is_empty() {
        return Err(SessionFlagError::EmptySessionId);
    }
    Ok(store
        .read(session_id)?
        .map(|f| f.distill_corpus)
        .unwrap_or(false))
}

/// Operator-driven mark. Requires a non-empty operator_signature per
/// HBR-INT-006. Returns the audit record for the flight recorder.
pub fn mark_for_distillation(
    store: &dyn SessionFlagStore,
    session_id: &str,
    flag: bool,
    operator_signature: &str,
    now_utc: &str,
) -> Result<SessionFlagWriteRecord, SessionFlagError> {
    if session_id.trim().is_empty() {
        return Err(SessionFlagError::EmptySessionId);
    }
    if operator_signature.trim().is_empty() {
        return Err(SessionFlagError::EmptySignature);
    }
    let previous_flag = store
        .read(session_id)?
        .map(|f| f.distill_corpus)
        .unwrap_or(false);
    store.write(DistillSessionFlag {
        session_id: session_id.to_string(),
        distill_corpus: flag,
        updated_at_utc: now_utc.to_string(),
        updated_by_operator_signature: operator_signature.to_string(),
    })?;
    Ok(SessionFlagWriteRecord {
        session_id: session_id.to_string(),
        previous_flag,
        new_flag: flag,
        operator_signature: operator_signature.to_string(),
        written_at_utc: now_utc.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_deny_when_session_has_no_row() {
        let store = InMemorySessionFlagStore::default();
        let flag = get_distill_flag(&store, "session-never-written").expect("read");
        assert!(!flag, "default must be false (opt-out)");
    }

    #[test]
    fn mark_then_read_round_trips_true() {
        let store = InMemorySessionFlagStore::default();
        let record = mark_for_distillation(
            &store,
            "session-1",
            true,
            "operator-ilja",
            "2026-05-20T03:00:00Z",
        )
        .expect("mark true");
        assert!(!record.previous_flag);
        assert!(record.new_flag);
        assert_eq!(record.operator_signature, "operator-ilja");

        let flag = get_distill_flag(&store, "session-1").expect("read");
        assert!(flag);
    }

    #[test]
    fn mark_then_unmark_round_trips_false() {
        let store = InMemorySessionFlagStore::default();
        mark_for_distillation(&store, "s", true, "op", "2026-05-20T03:00:00Z").unwrap();
        let record = mark_for_distillation(&store, "s", false, "op", "2026-05-20T03:01:00Z")
            .expect("unmark");
        assert!(record.previous_flag);
        assert!(!record.new_flag);
        let flag = get_distill_flag(&store, "s").expect("read");
        assert!(!flag);
    }

    #[test]
    fn empty_session_id_errors_on_read_and_write() {
        let store = InMemorySessionFlagStore::default();
        let err = get_distill_flag(&store, "  ").expect_err("empty session_id read");
        assert!(matches!(err, SessionFlagError::EmptySessionId));

        let err = mark_for_distillation(&store, "", true, "op", "2026-05-20T03:00:00Z")
            .expect_err("empty session_id write");
        assert!(matches!(err, SessionFlagError::EmptySessionId));
    }

    #[test]
    fn empty_signature_rejected_per_hbr_int_006() {
        let store = InMemorySessionFlagStore::default();
        let err = mark_for_distillation(&store, "s", true, "  ", "2026-05-20T03:00:00Z")
            .expect_err("empty signature");
        assert!(matches!(err, SessionFlagError::EmptySignature));
        // Default-deny preserved (no row written when signature missing).
        assert!(!get_distill_flag(&store, "s").unwrap());
    }

    #[test]
    fn flag_is_per_session_not_global() {
        let store = InMemorySessionFlagStore::default();
        mark_for_distillation(&store, "a", true, "op", "2026-05-20T03:00:00Z").unwrap();
        assert!(get_distill_flag(&store, "a").unwrap());
        assert!(!get_distill_flag(&store, "b").unwrap());
    }
}
