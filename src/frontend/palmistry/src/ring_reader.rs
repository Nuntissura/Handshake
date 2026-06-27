//! The Palmistry RING READER (MT-090, §6.13.4 — the ZERO-COOPERATION passive observer).
//!
//! Master Spec v02.196 §6.13.4: *"Shared-memory ring-buffer reader ... requires zero cooperation from
//! Handshake: it reads the ring even when the UI thread is frozen, because the ring is plain shared
//! memory and the reader never asks the (possibly hung) writer to do anything."*
//!
//! This is the data source for freeze detection (MT-091) and the survivor store (MT-093). It maps the
//! SAME backing file the Handshake writer created (the MT-081 ring) and reads Handshake's heartbeat +
//! last-N diagnostic events with NO cooperation from the writer.
//!
//! # What this module ADDS over MT-081 (and what it deliberately does NOT)
//!
//! The seqlock read, torn-read resistance, and `magic`/`version`/`record_size`/`capacity` validation
//! are ALL in the MT-081 [`handshake_diag_ring::DiagRingReader`]. This wrapper is intentionally thin:
//! it adds only the two Palmistry-side concerns the watcher needs and the ring crate does not own:
//!
//! 1. **Open-retry with bounded backoff** ([`PalmistryRingReader::open_with_retry`]). Palmistry is
//!    launched *with* Handshake (MT-094) and may WIN the startup race — the ring file may not exist (or
//!    not yet carry a valid header) at the instant Palmistry opens it. Rather than crash or busy-spin,
//!    it retries with a bounded backoff until the ring appears + opens with a valid header, or a bounded
//!    deadline elapses (AC-010-3 / RISK-010-3).
//! 2. **Re-open if the backing file is replaced** ([`PalmistryRingReader::reopen`]). A new Handshake
//!    session creates a fresh ring at the same path; the watcher can drop its stale map and re-open the
//!    current backing file without restarting the process.
//!
//! # The ZERO-COOPERATION invariant (HARD, §6.13.4)
//!
//! Every read path here delegates to the wait-free seqlock reads of the mapped bytes. There is NO call
//! into the writer, NO lock the writer also takes, NO socket request, and NO wait on the writer. That is
//! WHY a frozen / stalled / dead writer cannot block or break this reader: the last heartbeat + last-N
//! events the writer published before it froze remain in plain shared memory and stay readable. That
//! readable-but-stale state is exactly what MT-091 keys on to detect a freeze (the heartbeat counter
//! stops advancing, but the last value is still observable).
//!
//! # The TYPED-ALLOWLIST on the read side (§6.13.8)
//!
//! This reader returns ONLY the typed [`Heartbeat`] (two `u64`s) and [`DiagEvent`] (all fixed-width
//! integers) from MT-081. There is no string-content read API and structurally cannot be one — the ring
//! carries no text (the `DiagEvent` POD type forbids it at compile time, see `schema.rs`), so there is
//! nothing free-text to read out (AC-010-4).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagEvent, DiagRingReader, Heartbeat};

/// Default backoff slept between open attempts during the startup race. Small enough to be ready
/// quickly once Handshake creates the ring, large enough that the retry loop never busy-spins a core
/// (HBR-QUIET): a watcher that pins a CPU polling for a not-yet-existent file is the opposite of a quiet
/// background observer.
pub const DEFAULT_OPEN_RETRY_BACKOFF: Duration = Duration::from_millis(50);

/// Default bounded deadline for the open-retry loop. After this elapses without the ring appearing +
/// opening cleanly, [`PalmistryRingReader::open_with_retry`] gives up with the LAST open error rather
/// than blocking forever. One second comfortably covers Handshake's ring creation in the launch race
/// while still bounding a misconfigured / never-created ring to a quick, clear failure.
pub const DEFAULT_OPEN_RETRY_DEADLINE: Duration = Duration::from_millis(1000);

/// A thin Palmistry-side wrapper over the MT-081 [`DiagRingReader`]. Owns the open backing-file path so
/// it can [`reopen`](PalmistryRingReader::reopen) the current ring if Handshake replaces the file, and
/// delegates every read to the wait-free seqlock reader (zero cooperation with the writer).
pub struct PalmistryRingReader {
    /// The MT-081 reader (the seqlock + header-validation engine). All reads go through this.
    reader: DiagRingReader,
    /// The backing-file path, retained so a re-open after a file replacement can target the same path.
    path: PathBuf,
}

impl PalmistryRingReader {
    /// Open the ring at `path` ONCE, with no retry. Returns an error if the file does not exist yet or
    /// does not validate as a ring (foreign / garbage / partially-written header). Use
    /// [`open_with_retry`](PalmistryRingReader::open_with_retry) at startup where a race with Handshake
    /// is expected.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let reader = DiagRingReader::open(path)?;
        Ok(Self {
            reader,
            path: path.to_path_buf(),
        })
    }

    /// Open the ring at `path`, retrying with bounded backoff until it appears + validates or `deadline`
    /// elapses (AC-010-3 / RISK-010-3). Palmistry may win the startup race against Handshake, so the
    /// ring file may be absent or mid-creation at first; this retries instead of crashing or busy-
    /// spinning. Sleeps `backoff` between attempts (so it never pins a CPU) and re-checks. On success it
    /// returns the ready reader; on timeout it returns the LAST open error so the failure names the real
    /// reason (e.g. "magic mismatch" vs "file not found").
    ///
    /// `now` is injected so tests can drive the clock deterministically; production passes
    /// [`Instant::now`].
    pub fn open_with_retry(
        path: &Path,
        backoff: Duration,
        deadline: Duration,
        mut now: impl FnMut() -> Instant,
        mut sleep: impl FnMut(Duration),
    ) -> std::io::Result<Self> {
        let start = now();
        loop {
            let last_err = match DiagRingReader::open(path) {
                Ok(reader) => {
                    return Ok(Self {
                        reader,
                        path: path.to_path_buf(),
                    });
                }
                Err(err) => err,
            };
            // Bounded: stop once the deadline has elapsed. Checked AFTER an attempt so at least one open
            // is always tried even with a zero deadline. On timeout, surface the LAST open error so the
            // failure names the real reason (e.g. file-not-found vs magic-mismatch).
            if now().duration_since(start) >= deadline {
                return Err(last_err);
            }
            // Sleep the bounded backoff before retrying — NEVER busy-spin (HBR-QUIET / RISK-010-3).
            sleep(backoff);
        }
    }

    /// Convenience wrapper over [`open_with_retry`](PalmistryRingReader::open_with_retry) using the
    /// production clock + real sleep and the default backoff/deadline. This is the path the watch loop
    /// uses at startup.
    pub fn open_with_default_retry(path: &Path) -> std::io::Result<Self> {
        Self::open_with_retry(
            path,
            DEFAULT_OPEN_RETRY_BACKOFF,
            DEFAULT_OPEN_RETRY_DEADLINE,
            Instant::now,
            std::thread::sleep,
        )
    }

    /// Re-open the SAME path, replacing the current map. Used when Handshake replaces the backing file
    /// (a fresh session creates a new ring at the same path); the watcher swaps to the current map
    /// without restarting. On failure the existing reader is left untouched and the error is returned,
    /// so a transient re-open failure never destroys a working map.
    pub fn reopen(&mut self) -> std::io::Result<()> {
        let reader = DiagRingReader::open(&self.path)?;
        self.reader = reader;
        Ok(())
    }

    /// The backing-file path this reader maps.
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The number of record slots in the mapped ring (from the validated MT-081 header).
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.reader.capacity()
    }

    /// Read Handshake's last published heartbeat — a self-consistent `(counter, timestamp)` pair, or
    /// `None` if no clean read was obtained within the bounded seqlock retry (a pathologically hot
    /// writer) OR the heartbeat was never written. ZERO COOPERATION: this is a pure seqlock read of the
    /// shared heartbeat fields — it never calls into, locks against, or waits on the writer, so a frozen
    /// writer's LAST heartbeat stays readable (AC-010-1 / AC-010-2). This is the freeze-probe read
    /// MT-091 polls: a frozen Handshake's heartbeat counter stops advancing but its last value remains
    /// observable here.
    #[inline]
    pub fn read_heartbeat(&self) -> Option<Heartbeat> {
        self.reader.read_heartbeat()
    }

    /// Read up to the last `n` diagnostic events, newest first, skipping any slot that is mid-write /
    /// torn / never-written (the MT-081 reader returns only CONSISTENT records). ZERO COOPERATION: a
    /// pure seqlock walk of the shared record slots — no cooperation with the writer — so the last-N
    /// events the writer published before it froze stay readable (AC-010-1 / AC-010-2). MT-093 reads
    /// these for the survivor store. Returns ONLY typed [`DiagEvent`] (integers) — there is no free-text
    /// path (AC-010-4).
    #[inline]
    pub fn read_last_events(&self, n: u32) -> Vec<DiagEvent> {
        self.reader.read_last_n(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_diag_ring::ring::DEFAULT_CAPACITY;
    use handshake_diag_ring::DiagRingWriter;
    use std::cell::Cell;

    /// Build a unique temp ring path for a test (no collisions across parallel tests).
    fn temp_ring_path(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("hsk-mt090-unit-{label}-{}-{nanos}.ring", std::process::id()))
    }

    struct PathGuard(PathBuf);
    impl Drop for PathGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn open_reads_heartbeat_and_events() {
        let path = temp_ring_path("open");
        let _g = PathGuard(path.clone());
        let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).unwrap();
        writer.write_heartbeat(7, 12_345);
        writer.write(DiagEvent::heartbeat(1, 1, 7, 12_345));
        writer.write(DiagEvent::resource_sample(1, 2, 500, 1024, 0, 99));

        let reader = PalmistryRingReader::open(&path).unwrap();
        let hb = reader.read_heartbeat().expect("heartbeat readable");
        assert_eq!(hb.counter, 7);
        assert_eq!(hb.timestamp_nanos, 12_345);
        let events = reader.read_last_events(8);
        assert_eq!(events.len(), 2, "two records written, both readable");
        // Newest first.
        assert_eq!(events[0].sequence_id, 2);
        assert_eq!(events[1].sequence_id, 1);
    }

    #[test]
    fn open_with_retry_succeeds_once_the_ring_appears() {
        // Simulate the startup race: the ring does NOT exist for the first few attempts, then a writer
        // creates it. The retry loop must keep trying (bounded) and then succeed — proving it does not
        // crash on a missing ring and does not give up before the deadline.
        let path = temp_ring_path("retry-race");
        let _g = PathGuard(path.clone());

        let attempt = Cell::new(0u32);
        // A virtual clock that advances by the backoff each sleep so the deadline math is deterministic.
        let clock = Cell::new(Instant::now());
        let create_on_attempt = 3u32;
        let path_for_sleep = path.clone();

        let result = PalmistryRingReader::open_with_retry(
            &path,
            Duration::from_millis(50),
            Duration::from_millis(10_000),
            || clock.get(),
            |d| {
                // On each "sleep", advance the virtual clock and, at the chosen attempt, create the ring
                // (modeling Handshake winning the race a few backoffs in).
                clock.set(clock.get() + d);
                let n = attempt.get() + 1;
                attempt.set(n);
                if n == create_on_attempt {
                    let w = DiagRingWriter::create(&path_for_sleep, DEFAULT_CAPACITY).unwrap();
                    w.write_heartbeat(42, 1);
                }
            },
        );

        let reader = result.expect("open_with_retry must succeed once the ring appears");
        assert_eq!(reader.read_heartbeat().unwrap().counter, 42);
        assert!(attempt.get() >= create_on_attempt, "it retried until the ring appeared");
    }

    #[test]
    fn open_with_retry_is_bounded_and_returns_last_error_on_timeout() {
        // The ring never appears: the loop must STOP at the deadline (not block forever) and surface the
        // last open error (a real "not found" reason), not a generic hang.
        let path = temp_ring_path("retry-timeout");
        let _g = PathGuard(path.clone()); // never created

        let clock = Cell::new(Instant::now());
        let sleeps = Cell::new(0u32);
        let result = PalmistryRingReader::open_with_retry(
            &path,
            Duration::from_millis(50),
            Duration::from_millis(200),
            || clock.get(),
            |d| {
                clock.set(clock.get() + d);
                sleeps.set(sleeps.get() + 1);
            },
        );

        assert!(result.is_err(), "a never-created ring must time out, not succeed");
        // Bounded: with a 200ms deadline and 50ms backoff it does ~4 sleeps then stops — NOT unbounded.
        assert!(
            sleeps.get() <= 5,
            "open-retry must be bounded by the deadline, did {} sleeps",
            sleeps.get()
        );
    }

    #[test]
    fn reopen_picks_up_a_replaced_backing_file() {
        let path = temp_ring_path("reopen");
        let _g = PathGuard(path.clone());
        {
            let w = DiagRingWriter::create(&path, DEFAULT_CAPACITY).unwrap();
            w.write_heartbeat(1, 1);
        }
        let mut reader = PalmistryRingReader::open(&path).unwrap();
        assert_eq!(reader.read_heartbeat().unwrap().counter, 1);

        // A new session replaces the ring with a fresh one at the same path.
        {
            let w = DiagRingWriter::create(&path, DEFAULT_CAPACITY).unwrap();
            w.write_heartbeat(999, 2);
        }
        reader.reopen().expect("reopen the replaced backing file");
        assert_eq!(
            reader.read_heartbeat().unwrap().counter,
            999,
            "after reopen the reader sees the NEW ring's heartbeat"
        );
    }
}
