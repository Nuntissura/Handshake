//! MT-005 retained PostgreSQL assertion family; embedded target supersession.
//!
//! All eight original named scenarios execute once in
//! `model_lane_context_bundle_surreal_tests` using these shared assertions.
//! Original fixture/source bytes and assertion mapping are retained in the
//! round08 external raw snapshots. Original source pin:
//! 98efa4d019c4f670186cd3ec1f048ece3adf0dcd, blob
//! 82ffce98b81eafecab9420fc14db23e25ce00b52.
//! No retired PostgreSQL transport or duplicate runnable scenario is present.

// A: original PG 54–129; context_bundle_record_and_consume_share_one_canonical_lock_order.
macro_rules! lock_order_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (different, $($assertion:tt)*) => { assert_ne!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use lock_order_assert;

// B: original PG 130–214; context_bundle_consumer_blocks_on_run_before_acquiring_downstream_lane.
macro_rules! run_first_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (different, $($assertion:tt)*) => { assert_ne!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use run_first_assert;

// C: original PG 215–442; context_bundle_derivation_rejects_run_lane_and_non_crdt_message_ledger_drift.
macro_rules! derivation_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (different, $($assertion:tt)*) => { assert_ne!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use derivation_assert;

// D: original PG 443–894; model_lane_context_bundle_consume_preserves_exact_scope_and_denies_foreign_or_mixed_sources.
macro_rules! scope_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (different, $($assertion:tt)*) => { assert_ne!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use scope_assert;

// E: original PG 895–1147; model_lane_context_bundle_persists_selection_state_and_replays.
macro_rules! selection_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (different, $($assertion:tt)*) => { assert_ne!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use selection_assert;

// F: original PG 1148–1488; model_lane_context_bundle_missing_artifact_ref_fails_closed.
macro_rules! denial_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (different, $($assertion:tt)*) => { assert_ne!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use denial_assert;

// G: original PG 1490–2378; model_lane_context_bundle_crdt_state_vector_and_loom_refs_are_replayable.
macro_rules! crdt_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (different, $($assertion:tt)*) => { assert_ne!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use crdt_assert;

// H: original PG 2380–2419; model_lane_context_bundle_rejects_fabricated_crdt_authority_before_persistence.
macro_rules! fabricated_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (different, $($assertion:tt)*) => { assert_ne!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use fabricated_assert;

