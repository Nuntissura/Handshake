//! WP-1 MT-002/MT-003 v6 proof-target supersession.
//!
//! The four original PostgreSQL scenarios execute exactly once through the
//! named wrappers in `model_lane_schema_surreal_tests`. Their substantive
//! assertion expressions, messages, and actual durable fixture observations
//! are supplied to these shared assertion helpers by those wrappers.
//! No PostgreSQL transport, runnable duplicate scenario, or mock proof lives here.
//! Historical original source is retained at pinned WP-1 commit
//! 98efa4d019c4f670186cd3ec1f048ece3adf0dcd (blob e4057580ff7e7eab212a39d97387b354ac2aaf4a).

macro_rules! replay_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use replay_assert;

macro_rules! runtime_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use runtime_assert;

macro_rules! terminal_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use terminal_assert;

macro_rules! denial_assert {
    (equal, $($assertion:tt)*) => { assert_eq!($($assertion)*); };
    (condition, $($assertion:tt)*) => { assert!($($assertion)*); };
}
pub(crate) use denial_assert;

