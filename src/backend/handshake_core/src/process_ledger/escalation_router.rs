//! Contract-path re-export for MT-188 escalation routing.
//!
//! The implementation lives with the cluster X.2 executor primitives under
//! `mt_executor::outcome`; this module keeps the MT contract import path stable.

pub use crate::mt_executor::outcome::{
    enact_decision, EscalationDecision, EscalationMailboxPoster, EscalationRouter,
};
