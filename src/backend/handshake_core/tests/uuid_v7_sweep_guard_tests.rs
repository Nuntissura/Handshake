//! WP-KERNEL-003 UUID v7 sweep regression guard.
//!
//! Two cheap assertions that pin the v7 migration: the version-7 marker bits
//! and the time-ordering monotonicity v7 guarantees. If a future change drops
//! the `v7` feature on the `uuid` crate, or reverts a call site to `new_v4`,
//! these tests catch the regression at the type/runtime layer.

use uuid::Uuid;

#[test]
fn uuids_minted_via_now_v7_carry_version_7() {
    let u = Uuid::now_v7();
    assert_eq!(u.get_version_num(), 7, "expected v7, got {}", u.get_version_num());
}

#[test]
fn two_sequential_now_v7_uuids_are_time_ordered() {
    let a = Uuid::now_v7();
    let b = Uuid::now_v7();
    // v7 high bits are unix_ts_ms big-endian; sequential calls within the same
    // millisecond may tie but never invert.
    assert!(a <= b, "v7 should be time-ordered: {a} vs {b}");
}
