//! AC-001-2 / PT-001-B — torn-read-resistance (the single safety-critical proof) and
//! AC-001-3 — wait-free writer (a slow/absent reader never back-pressures the writer).
//!
//! Torn-read: a writer thread hammers `write_heartbeat` encoding the INVARIANT `counter ==
//! timestamp_nanos` on every write (so the pair is only valid if read as a unit). A reader thread
//! calls `read_heartbeat` thousands of times; EVERY successful read must satisfy `counter ==
//! timestamp_nanos`. A torn read (counter from write K, timestamp from write K+1) would break the
//! invariant and fail the test. This is what proves the seqlock is correct.
//!
//! Wait-free: with the reader NEVER reading, the writer completes a large fixed number of writes
//! within a tight time bound — it never waits on the reader.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagEvent, DiagRingReader, DiagRingWriter};

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
fn seqlock_torn_read_heartbeat_is_always_consistent() {
    let path = temp_ring_path("torn-hb");
    let _guard = PathGuard(path.clone());

    let writer = DiagRingWriter::create(&path, 256).expect("create ring");
    // Reader opens the SAME backing file (separate object).
    let reader = DiagRingReader::open(&path).expect("open ring");

    let stop = Arc::new(AtomicBool::new(false));
    let writer_iters: u64 = 2_000_000;

    let stop_w = Arc::clone(&stop);
    let writer_handle = std::thread::spawn(move || {
        // INVARIANT: counter == timestamp_nanos on every heartbeat write. A torn read would observe
        // a counter and timestamp from DIFFERENT writes and break this.
        let mut v: u64 = 1;
        for _ in 0..writer_iters {
            writer.write_heartbeat(v, v);
            v = v.wrapping_add(1);
            if v == 0 {
                v = 1; // never publish the all-zero "never written" sentinel as a real value
            }
        }
        stop_w.store(true, Ordering::Release);
        writer // keep the map alive until the reader is done
    });

    // Reader: spin reading the heartbeat until the writer signals stop, asserting the invariant on
    // every successful read. Count successful reads to prove we actually observed many.
    let mut successful_reads: u64 = 0;
    let mut none_reads: u64 = 0;
    let deadline = Instant::now() + Duration::from_secs(30);
    while !stop.load(Ordering::Acquire) {
        match reader.read_heartbeat() {
            Some(hb) => {
                assert_eq!(
                    hb.counter, hb.timestamp_nanos,
                    "TORN READ: counter {} != timestamp {} (seqlock failed to give a consistent pair)",
                    hb.counter, hb.timestamp_nanos
                );
                successful_reads += 1;
            }
            None => {
                // Allowed: the bounded retry can give up under a pathologically hot writer. It must
                // never return an INCONSISTENT pair, but it may return None.
                none_reads += 1;
            }
        }
        assert!(
            Instant::now() < deadline,
            "writer did not finish within 30s — possible livelock"
        );
    }
    // Drain a few more reads after stop to also exercise the settled (even-seq) path.
    for _ in 0..1000 {
        if let Some(hb) = reader.read_heartbeat() {
            assert_eq!(hb.counter, hb.timestamp_nanos, "TORN READ after stop");
            successful_reads += 1;
        }
    }

    let _writer = writer_handle.join().expect("writer thread");
    assert!(
        successful_reads > 1000,
        "expected many consistent reads, got {successful_reads} (none_reads={none_reads})"
    );
}

#[test]
fn writer_is_wait_free_when_reader_never_reads() {
    // AC-001-3: a slow/absent reader does not back-pressure the writer. Open a reader and NEVER call
    // it; the writer must still complete a large fixed number of writes within a tight time bound.
    let path = temp_ring_path("waitfree");
    let _guard = PathGuard(path.clone());

    let writer = DiagRingWriter::create(&path, 128).expect("create ring");
    // A reader exists but is intentionally idle (mapped, never reading) — proves no back-pressure.
    let _idle_reader = DiagRingReader::open(&path).expect("open idle reader");

    let writes: u64 = 5_000_000;
    let start = Instant::now();
    for i in 0..writes {
        writer.write(DiagEvent::heartbeat(1, i, i, 1_000 + i));
    }
    let elapsed = start.elapsed();

    // 5M wait-free writes into a memmap must be fast. Generous bound (10s) to stay green on slow/
    // loaded CI while still proving the writer NEVER blocks waiting for the idle reader (a blocking
    // design would hang here forever / time out the test harness).
    assert!(
        elapsed < Duration::from_secs(10),
        "writer took {elapsed:?} for {writes} writes — it must be wait-free and not wait on the reader"
    );
}

#[test]
fn seqlock_torn_read_records_are_always_consistent() {
    // Stronger record-level torn-read proof: the writer encodes counter_a == timestamp_nanos on
    // every RECORD write; the reader's read_last_n must only ever return records satisfying the
    // invariant (never a torn record assembled from two different writes).
    let path = temp_ring_path("torn-rec");
    let _guard = PathGuard(path.clone());

    let writer = DiagRingWriter::create(&path, 64).expect("create ring");
    let reader = DiagRingReader::open(&path).expect("open ring");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_w = Arc::clone(&stop);
    let writer_iters: u64 = 1_000_000;

    let writer_handle = std::thread::spawn(move || {
        for i in 1..=writer_iters {
            // counter_a == timestamp_nanos invariant.
            writer.write(DiagEvent::resource_sample(9, i, i, 0, 0, i));
        }
        stop_w.store(true, Ordering::Release);
        writer
    });

    let mut checked: u64 = 0;
    let deadline = Instant::now() + Duration::from_secs(30);
    while !stop.load(Ordering::Acquire) {
        for rec in reader.read_last_n(64) {
            assert_eq!(
                rec.counter_a, rec.timestamp_nanos,
                "TORN RECORD: counter_a {} != timestamp {}",
                rec.counter_a, rec.timestamp_nanos
            );
            checked += 1;
        }
        assert!(Instant::now() < deadline, "possible livelock in record reader");
    }
    let _writer = writer_handle.join().expect("writer thread");
    assert!(checked > 0, "reader observed no records to validate");
}
