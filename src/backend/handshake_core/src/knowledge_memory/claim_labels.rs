//! MT-125 ClaimAuthorityLabels (product surface).
//!
//! Labels a claim/fact as source, derived, model-suggested, operator-approved,
//! deprecated, superseded, or unsupported, with an enforced legal
//! label-transition table (negative paths covered by tests). The label storage
//! type and the guarded `set_memory_fact_authority_label` mutation live in
//! `storage::knowledge_memory`; this module re-exports the vocabulary plus a
//! small read-side classifier so product logic can reason about labels without
//! reaching into `storage`.

pub use crate::storage::knowledge_memory::{
    set_memory_fact_authority_label, MemoryClaimAuthorityLabel,
};

/// Read-side classification of an authority label for ranking/retrieval gates:
/// a label is "retrieval-trusted" when it represents currently-valid,
/// evidence-or-operator-backed authority, and is NOT trusted when it is
/// model-suggested-but-unconfirmed, deprecated, superseded, or unsupported.
///
/// This mirrors the translated-spec rule that probationary / unsupported
/// material must not be served as stable retrieval fact without confirmation.
pub fn is_retrieval_trusted(label: MemoryClaimAuthorityLabel) -> bool {
    matches!(
        label,
        MemoryClaimAuthorityLabel::Source
            | MemoryClaimAuthorityLabel::Derived
            | MemoryClaimAuthorityLabel::OperatorApproved
    )
}

/// Whether a label marks a fact as no-longer-current (deprecated/superseded) or
/// ungrounded (unsupported) — i.e. excluded from fresh retrieval entirely.
pub fn is_retired_label(label: MemoryClaimAuthorityLabel) -> bool {
    matches!(
        label,
        MemoryClaimAuthorityLabel::Deprecated
            | MemoryClaimAuthorityLabel::Superseded
            | MemoryClaimAuthorityLabel::Unsupported
    )
}
