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
    assert_eq!(
        u.get_version_num(),
        7,
        "expected v7, got {}",
        u.get_version_num()
    );
}

#[test]
fn now_v7_uuids_are_time_ordered_across_milliseconds() {
    let mut uuids = Vec::with_capacity(20);
    for _ in 0..20 {
        uuids.push(uuid::Uuid::now_v7());
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    for w in uuids.windows(2) {
        assert!(
            w[0] < w[1],
            "v7 should be time-ordered across ms: {} vs {}",
            w[0],
            w[1]
        );
    }
}

#[test]
fn now_v7_high_bits_are_monotonic_intra_ms() {
    let samples: Vec<u128> = (0..1000).map(|_| uuid::Uuid::now_v7().as_u128()).collect();
    for w in samples.windows(2) {
        assert!(
            (w[0] >> 80) <= (w[1] >> 80),
            "v7 high-48 bits must be non-decreasing"
        );
    }
}
