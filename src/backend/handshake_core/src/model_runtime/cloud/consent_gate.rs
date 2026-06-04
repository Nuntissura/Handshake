//! MT-128 (part 2/2): Per-lane operator consent gate.
//!
//! Per MT-128 implementation_notes + AC-MODEL-RUNTIME-TRAIT: when
//! `settings.exec_policy.cloud_consent_per_session=true`, the first
//! cloud call within a session for a given lane must be explicitly
//! consented to by the operator. The gate is per-session +
//! per-lane: consent for the OpenAI lane in session A does NOT
//! grant consent for the Anthropic lane in session A, nor for the
//! OpenAI lane in session B.
//!
//! The gate state is a pure in-memory map; the UI prompt + decision
//! capture flow live in the cluster-X session-close panel
//! (follow-on). The gate's contract is: `check_or_prompt(session,
//! lane, consent_provider)` returns Ok(()) if the operator has
//! consented or consents now via the provider; otherwise an
//! ConsentDenied error that the caller surfaces.

use std::collections::HashMap;
use std::sync::RwLock;

use thiserror::Error;

/// Two-tuple key into the consent map: (session_id, lane_id).
type ConsentKey = (String, String);

#[derive(Debug, Error)]
pub enum ConsentGateError {
    #[error("session id must not be empty")]
    EmptySessionId,
    #[error("lane id must not be empty")]
    EmptyLaneId,
    #[error("operator denied cloud consent for lane {lane} in session {session}")]
    ConsentDenied { session: String, lane: String },
    #[error("consent provider error: {0}")]
    ProviderError(String),
    #[error("internal consent gate lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Operator decision returned by the consent provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentDecision {
    /// Operator approved the cloud send for this lane in this
    /// session. Subsequent calls in the same (session, lane) pair
    /// pass without re-prompting.
    Approved,
    /// Operator denied the cloud send. The gate stores the denial
    /// so subsequent calls in the same (session, lane) pair short-
    /// circuit without re-prompting.
    Denied,
}

/// Abstraction over the operator-prompted decision capture. The
/// production impl wires the UI (cluster-X session-close panel);
/// the in-process impls below cover unit tests.
pub trait ConsentProvider: Send + Sync {
    fn prompt_for_decision(
        &self,
        session_id: &str,
        lane: &str,
    ) -> Result<ConsentDecision, ConsentGateError>;
}

pub struct ConsentGate {
    decisions: RwLock<HashMap<ConsentKey, ConsentDecision>>,
}

impl Default for ConsentGate {
    fn default() -> Self {
        Self {
            decisions: RwLock::new(HashMap::new()),
        }
    }
}

impl ConsentGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns Ok(()) if the operator has consented to the (session,
    /// lane) pair (either previously or just now via the provider).
    /// Returns ConsentDenied if the operator denied either now or
    /// in a prior prompt for the same (session, lane) pair.
    pub fn check_or_prompt(
        &self,
        session_id: &str,
        lane: &str,
        provider: &dyn ConsentProvider,
    ) -> Result<(), ConsentGateError> {
        if session_id.trim().is_empty() {
            return Err(ConsentGateError::EmptySessionId);
        }
        if lane.trim().is_empty() {
            return Err(ConsentGateError::EmptyLaneId);
        }
        let key = (session_id.to_string(), lane.to_string());
        {
            let guard = self
                .decisions
                .read()
                .map_err(|err| ConsentGateError::LockPoisoned(err.to_string()))?;
            if let Some(decision) = guard.get(&key) {
                return match decision {
                    ConsentDecision::Approved => Ok(()),
                    ConsentDecision::Denied => Err(ConsentGateError::ConsentDenied {
                        session: session_id.to_string(),
                        lane: lane.to_string(),
                    }),
                };
            }
        }
        // No prior decision; prompt now.
        let decision = provider.prompt_for_decision(session_id, lane)?;
        {
            let mut guard = self
                .decisions
                .write()
                .map_err(|err| ConsentGateError::LockPoisoned(err.to_string()))?;
            guard.insert(key, decision);
        }
        match decision {
            ConsentDecision::Approved => Ok(()),
            ConsentDecision::Denied => Err(ConsentGateError::ConsentDenied {
                session: session_id.to_string(),
                lane: lane.to_string(),
            }),
        }
    }

    /// Forget the decision for a (session, lane) pair. Useful for
    /// tests + for the session-close cleanup path that drops
    /// per-session consent when the session ends.
    pub fn forget(&self, session_id: &str, lane: &str) -> Result<(), ConsentGateError> {
        if session_id.trim().is_empty() {
            return Err(ConsentGateError::EmptySessionId);
        }
        if lane.trim().is_empty() {
            return Err(ConsentGateError::EmptyLaneId);
        }
        let mut guard = self
            .decisions
            .write()
            .map_err(|err| ConsentGateError::LockPoisoned(err.to_string()))?;
        guard.remove(&(session_id.to_string(), lane.to_string()));
        Ok(())
    }

    /// Drop ALL consent decisions for a session at session close
    /// time. Operator consent is per-session-per-lane and does NOT
    /// persist across session boundaries.
    pub fn drop_session(&self, session_id: &str) -> Result<(), ConsentGateError> {
        if session_id.trim().is_empty() {
            return Err(ConsentGateError::EmptySessionId);
        }
        let mut guard = self
            .decisions
            .write()
            .map_err(|err| ConsentGateError::LockPoisoned(err.to_string()))?;
        guard.retain(|(s, _), _| s != session_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct StaticProvider {
        decision: ConsentDecision,
        prompts: Mutex<u32>,
    }
    impl ConsentProvider for StaticProvider {
        fn prompt_for_decision(
            &self,
            _session_id: &str,
            _lane: &str,
        ) -> Result<ConsentDecision, ConsentGateError> {
            *self.prompts.lock().unwrap() += 1;
            Ok(self.decision)
        }
    }

    struct DeniedProvider;
    impl ConsentProvider for DeniedProvider {
        fn prompt_for_decision(
            &self,
            _session_id: &str,
            _lane: &str,
        ) -> Result<ConsentDecision, ConsentGateError> {
            Ok(ConsentDecision::Denied)
        }
    }

    #[test]
    fn first_call_in_session_prompts_provider_then_caches_approval() {
        let gate = ConsentGate::new();
        let provider = StaticProvider {
            decision: ConsentDecision::Approved,
            prompts: Mutex::new(0),
        };
        gate.check_or_prompt("session-1", "openai", &provider)
            .expect("approved");
        gate.check_or_prompt("session-1", "openai", &provider)
            .expect("cached approval");
        gate.check_or_prompt("session-1", "openai", &provider)
            .expect("cached approval");
        // Provider was called exactly once across three checks.
        assert_eq!(*provider.prompts.lock().unwrap(), 1);
    }

    #[test]
    fn denial_short_circuits_subsequent_calls_without_re_prompting() {
        let gate = ConsentGate::new();
        let provider = StaticProvider {
            decision: ConsentDecision::Denied,
            prompts: Mutex::new(0),
        };
        let err = gate
            .check_or_prompt("session-1", "openai", &provider)
            .expect_err("denied");
        assert!(matches!(err, ConsentGateError::ConsentDenied { .. }));
        // Second call must NOT re-prompt; it must short-circuit to denied.
        let err = gate
            .check_or_prompt("session-1", "openai", &provider)
            .expect_err("cached denial");
        assert!(matches!(err, ConsentGateError::ConsentDenied { .. }));
        assert_eq!(*provider.prompts.lock().unwrap(), 1);
    }

    #[test]
    fn consent_is_per_session_per_lane_not_global() {
        let gate = ConsentGate::new();
        let approved = StaticProvider {
            decision: ConsentDecision::Approved,
            prompts: Mutex::new(0),
        };
        gate.check_or_prompt("session-A", "openai", &approved)
            .expect("approved");

        // Different session, same lane -> must re-prompt.
        let denied = DeniedProvider;
        let err = gate
            .check_or_prompt("session-B", "openai", &denied)
            .expect_err("session-B denies");
        assert!(matches!(err, ConsentGateError::ConsentDenied { .. }));

        // Same session, different lane -> must re-prompt.
        let err = gate
            .check_or_prompt("session-A", "anthropic", &denied)
            .expect_err("anthropic lane denies");
        assert!(matches!(err, ConsentGateError::ConsentDenied { .. }));
    }

    #[test]
    fn drop_session_clears_all_lanes_for_a_session() {
        let gate = ConsentGate::new();
        let approved = StaticProvider {
            decision: ConsentDecision::Approved,
            prompts: Mutex::new(0),
        };
        gate.check_or_prompt("session-1", "openai", &approved)
            .unwrap();
        gate.check_or_prompt("session-1", "anthropic", &approved)
            .unwrap();
        gate.drop_session("session-1").unwrap();
        // After drop_session, provider must be re-prompted.
        let prior_prompts = *approved.prompts.lock().unwrap();
        gate.check_or_prompt("session-1", "openai", &approved)
            .unwrap();
        assert_eq!(*approved.prompts.lock().unwrap(), prior_prompts + 1);
    }

    #[test]
    fn empty_session_or_lane_id_rejected() {
        let gate = ConsentGate::new();
        let provider = StaticProvider {
            decision: ConsentDecision::Approved,
            prompts: Mutex::new(0),
        };
        assert!(matches!(
            gate.check_or_prompt("", "openai", &provider).unwrap_err(),
            ConsentGateError::EmptySessionId
        ));
        assert!(matches!(
            gate.check_or_prompt("session", " ", &provider).unwrap_err(),
            ConsentGateError::EmptyLaneId
        ));
    }
}
