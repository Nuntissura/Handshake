//! AC-001-1 / PT-001-A — round-trip + wrap-around proof, and AC-001-5 — foreign-map refusal.
//!
//! A `DiagRingWriter` writes N=2000 records (> capacity 1024, so the ring wraps) plus a heartbeat;
//! a SEPARATELY-constructed `DiagRingReader::open` on the SAME backing file reads the heartbeat and
//! the last 1024 records back, and the most-recent records match what was written. This is a real
//! second reader over the real shared map (no in-memory Vec stand-in), per the Spec-Realism Gate.

use std::path::PathBuf;

use handshake_diag_ring::ring::{debug_corrupt_byte, DEFAULT_CAPACITY};
use handshake_diag_ring::{DiagEvent, DiagRingReader, DiagRingWriter};

/// Unique backing-file path under the OS temp dir for this test (no spaces, CX-109A).
fn temp_ring_path(tag: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("handshake-diag-test-{tag}-{pid}-{nanos}.ring"))
}

struct PathGuard(PathBuf);
impl Drop for PathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn ring_roundtrip_wraps_and_reads_back() {
    let path = temp_ring_path("roundtrip");
    let _guard = PathGuard(path.clone());

    let capacity = DEFAULT_CAPACITY; // 1024
    let total: u64 = 2000; // > capacity => proves wrap-around

    // --- WRITER ---
    let writer = DiagRingWriter::create(&path, capacity).expect("create ring");
    assert_eq!(writer.capacity(), capacity);
    for i in 0..total {
        // Encode the global sequence index into sequence_id AND counter_a so the reader can verify
        // exactly which writes survived the wrap.
        let event = DiagEvent::resource_sample(
            /* thread_id */ 7,
            /* sequence_id */ i,
            /* cpu_milli (counter_a) */ i,
            /* rss_kb (counter_b) */ i * 2,
            /* metric_micros */ i * 3,
            /* timestamp_nanos */ 1_000 + i,
        );
        writer.write(event);
    }
    writer.write_heartbeat(/* counter */ 4242, /* timestamp_nanos */ 999_000);
    // Drop the writer to flush/close before the reader opens — proves a SEPARATE reader, not a
    // shared in-process object.
    drop(writer);

    // --- READER (separately constructed, same backing file) ---
    let reader = DiagRingReader::open(&path).expect("open ring");
    assert_eq!(reader.capacity(), capacity);

    // Heartbeat round-trips.
    let hb = reader.read_heartbeat().expect("heartbeat present");
    assert_eq!(hb.counter, 4242);
    assert_eq!(hb.timestamp_nanos, 999_000);

    // The last `capacity` records are the ones with sequence_id in [total-capacity, total).
    let records = reader.read_last_n(capacity);
    assert_eq!(
        records.len(),
        capacity as usize,
        "expected a full ring of {capacity} consistent records, got {}",
        records.len()
    );

    // read_last_n returns NEWEST first. The newest written had sequence_id = total-1.
    let newest = &records[0];
    assert_eq!(newest.sequence_id, total - 1, "newest record sequence_id");
    assert_eq!(newest.counter_a, total - 1);
    assert_eq!(newest.counter_b, (total - 1) * 2);
    assert_eq!(newest.timestamp_nanos, 1_000 + (total - 1));

    // The oldest surviving record (index capacity-1, newest-first) had sequence_id = total-capacity.
    let oldest_surviving = &records[capacity as usize - 1];
    assert_eq!(
        oldest_surviving.sequence_id,
        total - capacity as u64,
        "oldest surviving record after wrap"
    );

    // Every returned record must be one we actually wrote, in strict descending sequence order.
    for (offset, rec) in records.iter().enumerate() {
        let expected_seq = total - 1 - offset as u64;
        assert_eq!(
            rec.sequence_id, expected_seq,
            "record at newest-first offset {offset} should have sequence_id {expected_seq}"
        );
        assert_eq!(rec.thread_id, 7);
    }
}

#[test]
fn reader_refuses_foreign_map_bad_magic() {
    // AC-001-5: opening a backing file whose header magic does not match returns Err, not garbage.
    let path = temp_ring_path("badmagic");
    let _guard = PathGuard(path.clone());

    {
        let writer = DiagRingWriter::create(&path, 64).expect("create ring");
        writer.write(DiagEvent::heartbeat(1, 1, 1, 1));
    }
    // Corrupt the first magic byte.
    debug_corrupt_byte(&path, 0, 0x00).expect("corrupt magic");

    // DiagRingReader holds a live MmapMut and intentionally does not implement Debug, so match the
    // Result directly rather than via expect_err (which requires the Ok type to be Debug).
    match DiagRingReader::open(&path) {
        Ok(_) => panic!("must refuse a map with bad magic"),
        Err(err) => assert_eq!(err.kind(), std::io::ErrorKind::InvalidData),
    }
}

#[test]
fn reader_refuses_too_small_file() {
    // AC-001-5 extension: a file smaller than the header is refused, not mapped as garbage.
    let path = temp_ring_path("tiny");
    let _guard = PathGuard(path.clone());
    std::fs::write(&path, b"not a ring").expect("write tiny file");

    match DiagRingReader::open(&path) {
        Ok(_) => panic!("must refuse a file smaller than the header"),
        Err(err) => assert_eq!(err.kind(), std::io::ErrorKind::InvalidData),
    }
}
