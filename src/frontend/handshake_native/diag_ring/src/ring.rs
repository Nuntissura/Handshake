//! Seqlock single-producer / single-consumer (SPSC) shared-memory ring.
//!
//! This is the substrate of the whole three-tier diagnostic decoupling (Master Spec v02.196
//! §5.8.6 + §6.13.4). The Handshake UI thread is the single WRITER (Tier 2 internal_diagnostics);
//! the external Palmistry watcher is the single READER (Tier 3). The defining requirement: a
//! frozen, slow, or crashed reader must NEVER back-pressure or tear the writer, because the writer
//! is the UI thread, which must never block on diagnostics.
//!
//! # Why a seqlock (research basis)
//!
//! The seqlock is the Linux-kernel single-writer publish primitive (it is how the vDSO publishes
//! the wall clock without locks). The writer bumps a `u32` sequence counter to an ODD value BEFORE
//! it touches the payload and back to the next EVEN value AFTER, so:
//! - an odd sequence means "a write is in progress" — the reader must retry;
//! - if the sequence the reader observes BEFORE reading the payload differs from the one it observes
//!   AFTER, the reader raced a write — it must retry.
//!
//! Both the heartbeat slot and every record slot are published this way. The writer is wait-free
//! (it never waits on the reader; it overwrites unread slots by design), and the reader's retries
//! are BOUNDED so a pathologically hot writer cannot livelock it — after the bound the reader
//! returns the last consistent value / `None` rather than spinning forever.
//!
//! # Memory ordering
//!
//! The `seq` and `write_index` fields are real cross-process atomics overlaid on the mapped bytes
//! via [`AtomicU32::from_ptr`] / [`AtomicU64::from_ptr`]. The writer publishes with
//! [`Ordering::Release`] on the post-write `seq` store; the reader observes with
//! [`Ordering::Acquire`] on its `seq` loads. This Release/Acquire pair establishes the happens-
//! before edge that makes the payload writes (done between the two seq stores) visible to the
//! reader and prevents a torn read. The payload bytes themselves are copied with non-atomic
//! `copy_nonoverlapping`; correctness comes from the seqlock fencing around them plus the
//! single-writer discipline, exactly as in the kernel seqlock.
//!
//! # Cross-process sharing on Windows (and portably)
//!
//! `memmap2::MmapMut` maps a FILE, not a Win32 named section. For portable cross-process sharing
//! both processes map the SAME backing file: Handshake creates/truncates a named file under the OS
//! temp dir (e.g. `%TEMP%/handshake-diag-<session-id>.ring`) and maps it; Palmistry opens the same
//! path read-only-ish and maps it. They share the same physical pages. The backing FILE PATH (not a
//! Win32 object name) is what Handshake passes to Palmistry (MT-089/MT-094). This is the field-
//! standard portable memmap2 cross-process pattern; raw `CreateFileMappingW` with a `Global\` name
//! is non-portable and unnecessary.

use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use bytemuck::{Pod, Zeroable};
use memmap2::MmapMut;

use crate::schema::{DiagEvent, DIAG_EVENT_SIZE};

/// Magic constant the reader checks to refuse a foreign / garbage map. ASCII "HSK_DIAG".
pub const RING_MAGIC: u64 = 0x48534B5F44494147;

/// Layout version. Bumped by MT-096 / WP-016 when the on-disk layout changes; the reader refuses a
/// map whose `version` does not match its own compiled expectation.
pub const RING_VERSION: u32 = 1;

/// Default number of record slots (a power of two is not required; the ring uses modulo).
pub const DEFAULT_CAPACITY: u32 = 1024;

/// Bound on seqlock read retries before the reader gives up on a slot and returns the last
/// consistent value / `None`. Caps the cost of a pathologically hot writer so the reader cannot
/// livelock.
pub const SEQLOCK_READ_RETRY_BOUND: u32 = 16;

// ---- On-map field offsets ---------------------------------------------------------------------
//
// The mapped region is laid out by hand so each cross-process atomic sits at a known, correctly
// aligned offset. All multi-byte fields are naturally aligned. The header is 64 bytes so the first
// record slot starts 8-aligned.
//
//   offset  size  field
//        0     8  magic            (u64, plain — written once at create, only read after)
//        8     4  version          (u32, plain)
//       12     4  record_size      (u32, plain)
//       16     4  capacity         (u32, plain)
//       20     4  write_index      (u32, ATOMIC)
//       24     4  hb_seq           (u32, ATOMIC)        <- heartbeat seqlock
//       28     4  _hb_pad          (u32)
//       32     8  hb_counter       (u64, plain, seqlock-protected)
//       40     8  hb_timestamp     (u64, plain, seqlock-protected)
//       48    16  _header_pad      (reserved to round the header to 64 bytes)
//   HEADER_SIZE = 64
//
// Each record slot (RECORD_SLOT_SIZE = 64):
//   offset  size  field
//        0     4  slot_seq         (u32, ATOMIC)        <- per-record seqlock
//        4     4  _slot_pad        (u32)
//        8    56  event            (DiagEvent, plain POD, seqlock-protected)
const OFF_MAGIC: usize = 0;
const OFF_VERSION: usize = 8;
const OFF_RECORD_SIZE: usize = 12;
const OFF_CAPACITY: usize = 16;
const OFF_WRITE_INDEX: usize = 20;
const OFF_HB_SEQ: usize = 24;
const OFF_HB_COUNTER: usize = 32;
const OFF_HB_TIMESTAMP: usize = 40;

/// Size of the fixed header region in bytes.
pub const HEADER_SIZE: usize = 64;

/// Per-record slot: a 4-byte seqlock sequence + 4-byte pad + the 56-byte [`DiagEvent`] = 64 bytes.
const SLOT_SEQ_OFF: usize = 0;
const SLOT_EVENT_OFF: usize = 8;
/// Total bytes per record slot.
pub const RECORD_SLOT_SIZE: usize = 64;

// Compile-time layout sanity: the event must fit in the slot after the seq+pad prefix.
const _: () = assert!(SLOT_EVENT_OFF + DIAG_EVENT_SIZE <= RECORD_SLOT_SIZE);
const _: () = assert!(HEADER_SIZE.is_multiple_of(8));
const _: () = assert!(RECORD_SLOT_SIZE.is_multiple_of(8));

/// A consistent heartbeat reading returned by [`DiagRingReader::read_heartbeat`]. The `(counter,
/// timestamp_nanos)` pair is guaranteed to have been published by the writer AS A UNIT — never a
/// counter from one write paired with a timestamp from another. This is the single most safety-
/// critical read in Palmistry (a freeze probe reads ONLY this, without scanning records).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Heartbeat {
    /// Monotonic frame/heartbeat counter as published by the writer.
    pub counter: u64,
    /// Monotonic timestamp in nanoseconds as published by the writer.
    pub timestamp_nanos: u64,
}

/// Plain-old-data view of the header's non-atomic prefix, used only to write the initial header
/// bytes at create time. Atomic fields are accessed separately via [`AtomicU32::from_ptr`].
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct HeaderInit {
    magic: u64,
    version: u32,
    record_size: u32,
    capacity: u32,
    write_index: u32,
    hb_seq: u32,
    _hb_pad: u32,
    hb_counter: u64,
    hb_timestamp: u64,
    _header_pad: [u8; 16],
}

const _: () = assert!(core::mem::size_of::<HeaderInit>() == HEADER_SIZE);

/// Compute the exact mapped region size for a given capacity.
#[inline]
fn region_size(capacity: u32) -> usize {
    HEADER_SIZE + (capacity as usize) * RECORD_SLOT_SIZE
}

/// Open (creating + truncating to `len` if needed) the backing file at `path`.
fn open_backing_file(path: &Path, create: bool, len: u64) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(true).write(true);
    if create {
        opts.create(true).truncate(false);
    }
    let file = opts.open(path)?;
    if create {
        // Size the file to the exact region size BEFORE mapping (a too-small map is UB / SIGBUS).
        file.set_len(len)?;
    }
    Ok(file)
}

/// Get a `&AtomicU32` over the mapped bytes at `offset`. Caller guarantees `offset` is in-bounds
/// and 4-aligned (the layout constants above ensure this for every atomic field).
#[inline]
fn atomic_u32_at(map: &MmapMut, offset: usize) -> &AtomicU32 {
    debug_assert!(offset + 4 <= map.len());
    debug_assert!(offset.is_multiple_of(4));
    // SAFETY: `offset` is in-bounds and 4-aligned (layout constants); the byte lives in the shared
    // map for the lifetime of `self`; cross-process atomic access is the intended use.
    unsafe { AtomicU32::from_ptr(map.as_ptr().add(offset) as *mut u32) }
}

/// Get a `&AtomicU64` over the mapped bytes at `offset`. Same invariants as [`atomic_u32_at`] with
/// 8-alignment.
#[inline]
fn atomic_u64_at(map: &MmapMut, offset: usize) -> &AtomicU64 {
    debug_assert!(offset + 8 <= map.len());
    debug_assert!(offset.is_multiple_of(8));
    // SAFETY: see `atomic_u32_at`; `offset` is in-bounds and 8-aligned.
    unsafe { AtomicU64::from_ptr(map.as_ptr().add(offset) as *mut u64) }
}

/// The WRITER half (Handshake / Tier 2 internal_diagnostics). Single producer. Wait-free: it never
/// waits on the reader and overwrites unread slots by design (a slow reader loses old records; it
/// never stalls Handshake).
pub struct DiagRingWriter {
    map: MmapMut,
    capacity: u32,
    // The backing file is kept alive for the lifetime of the map (dropping it early is fine on most
    // OSes once mapped, but holding it is the safe portable choice).
    _file: File,
}

impl DiagRingWriter {
    /// Create a new ring backed by a file at `path` with `capacity` record slots, writing the
    /// header. Truncates/sizes the file to the exact region size first.
    pub fn create(path: &Path, capacity: u32) -> io::Result<Self> {
        if capacity == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ring capacity must be > 0",
            ));
        }
        let len = region_size(capacity) as u64;
        let file = open_backing_file(path, true, len)?;
        // SAFETY: the file was just sized to `len` >= region_size(capacity) via set_len.
        let mut map = unsafe { MmapMut::map_mut(&file)? };

        // Write the initial header (atomic fields start at 0 / even seq).
        let header = HeaderInit {
            magic: RING_MAGIC,
            version: RING_VERSION,
            record_size: DIAG_EVENT_SIZE as u32,
            capacity,
            write_index: 0,
            hb_seq: 0,
            _hb_pad: 0,
            hb_counter: 0,
            hb_timestamp: 0,
            _header_pad: [0; 16],
        };
        map[..HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(&header));
        // Zero the record region so a torn-looking slot can't appear from stale file bytes.
        for b in map[HEADER_SIZE..].iter_mut() {
            *b = 0;
        }
        map.flush()?;

        Ok(Self {
            map,
            capacity,
            _file: file,
        })
    }

    /// The number of record slots in this ring.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Write one diagnostic record. Wait-free seqlock publish into the slot at
    /// `write_index % capacity`, then advance `write_index`. Overwrites an unread slot by design.
    pub fn write(&self, event: DiagEvent) {
        let write_index = atomic_u32_at(&self.map, OFF_WRITE_INDEX);
        // Single producer => Relaxed load of our own counter is fine; we are the only writer.
        let idx = write_index.load(Ordering::Relaxed);
        let slot = (idx % self.capacity) as usize;
        let slot_base = HEADER_SIZE + slot * RECORD_SLOT_SIZE;
        let seq = atomic_u32_at(&self.map, slot_base + SLOT_SEQ_OFF);

        let cur = seq.load(Ordering::Relaxed);
        // Publish: bump to odd (write in progress).
        seq.store(cur.wrapping_add(1), Ordering::Release);
        // Write the payload between the odd and even seq stores.
        let event_off = slot_base + SLOT_EVENT_OFF;
        // SAFETY: event_off + DIAG_EVENT_SIZE <= slot_base + RECORD_SLOT_SIZE <= map.len(); the
        // destination is a plain byte region we exclusively own as the single writer.
        unsafe {
            let dst = self.map.as_ptr().add(event_off) as *mut u8;
            core::ptr::copy_nonoverlapping(
                bytemuck::bytes_of(&event).as_ptr(),
                dst,
                DIAG_EVENT_SIZE,
            );
        }
        // Bump to even again (write complete). Release so the reader's Acquire sees the payload.
        seq.store(cur.wrapping_add(2), Ordering::Release);

        // Advance write_index AFTER the slot is fully published, Release so a reader that loads the
        // new write_index (Acquire) is guaranteed to see the published slot.
        write_index.store(idx.wrapping_add(1), Ordering::Release);
    }

    /// Publish the reserved heartbeat slot (MT-084 calls this every egui frame). Same odd/even
    /// seqlock dance as a record, on the dedicated heartbeat fields in the header so a freeze probe
    /// reads ONLY the heartbeat without scanning records.
    pub fn write_heartbeat(&self, counter: u64, timestamp_nanos: u64) {
        let hb_seq = atomic_u32_at(&self.map, OFF_HB_SEQ);
        let hb_counter = atomic_u64_at(&self.map, OFF_HB_COUNTER);
        let hb_timestamp = atomic_u64_at(&self.map, OFF_HB_TIMESTAMP);

        let cur = hb_seq.load(Ordering::Relaxed);
        hb_seq.store(cur.wrapping_add(1), Ordering::Release); // odd: write in progress
        // Store the payload as a UNIT under the seqlock. Relaxed is fine here: the Release on the
        // post-write seq store below provides the publish ordering for the reader's Acquire.
        hb_counter.store(counter, Ordering::Relaxed);
        hb_timestamp.store(timestamp_nanos, Ordering::Relaxed);
        hb_seq.store(cur.wrapping_add(2), Ordering::Release); // even: write complete
    }
}

/// The READER half (Palmistry / Tier 3). Single consumer. Maps the SAME backing file the writer
/// created, validates the header, and reads under the seqlock retry protocol. Never back-pressures
/// the writer.
pub struct DiagRingReader {
    map: MmapMut,
    capacity: u32,
    _file: File,
}

impl DiagRingReader {
    /// Open an existing ring at `path`, validating `magic`, `version`, `record_size`, and a sane
    /// `capacity` against this crate's compiled expectations. Refuses a foreign / garbage / drifted
    /// map with an `Err` rather than reading garbage records.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = open_backing_file(path, false, 0)?;
        let file_len = file.metadata()?.len();
        if file_len < HEADER_SIZE as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ring backing file is smaller than the header",
            ));
        }
        // SAFETY: file is at least HEADER_SIZE bytes (checked above).
        let map = unsafe { MmapMut::map_mut(&file)? };

        // Validate the header before trusting any field.
        let magic = atomic_u64_at(&map, OFF_MAGIC).load(Ordering::Acquire);
        if magic != RING_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ring magic mismatch (foreign or garbage map)",
            ));
        }
        let version = atomic_u32_at(&map, OFF_VERSION).load(Ordering::Acquire);
        if version != RING_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ring version mismatch (incompatible layout)",
            ));
        }
        let record_size = atomic_u32_at(&map, OFF_RECORD_SIZE).load(Ordering::Acquire);
        if record_size as usize != DIAG_EVENT_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ring record_size mismatch (record layout drift)",
            ));
        }
        let capacity = atomic_u32_at(&map, OFF_CAPACITY).load(Ordering::Acquire);
        if capacity == 0 || region_size(capacity) as u64 > file_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ring capacity is invalid or larger than the backing file",
            ));
        }

        Ok(Self {
            map,
            capacity,
            _file: file,
        })
    }

    /// The number of record slots in this ring (from the validated header).
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Seqlock read of the heartbeat. Returns a self-consistent `(counter, timestamp)` pair, or
    /// `None` if the writer was so hot that the retry bound was exhausted without a clean read.
    /// This is the freeze-probe read: it touches ONLY the heartbeat fields.
    pub fn read_heartbeat(&self) -> Option<Heartbeat> {
        let hb_seq = atomic_u32_at(&self.map, OFF_HB_SEQ);
        let hb_counter = atomic_u64_at(&self.map, OFF_HB_COUNTER);
        let hb_timestamp = atomic_u64_at(&self.map, OFF_HB_TIMESTAMP);

        let mut retries = 0u32;
        loop {
            // First seq read (Acquire): if odd, a write is in progress -> retry (bounded).
            let s1 = hb_seq.load(Ordering::Acquire);
            if s1 & 1 == 0 {
                // Read the payload as a unit.
                let counter = hb_counter.load(Ordering::Relaxed);
                let timestamp_nanos = hb_timestamp.load(Ordering::Relaxed);
                // Second seq read (Acquire): if unchanged, the payload did not race a write.
                let s2 = hb_seq.load(Ordering::Acquire);
                if s1 == s2 {
                    return Some(Heartbeat {
                        counter,
                        timestamp_nanos,
                    });
                }
            }
            retries += 1;
            if retries >= SEQLOCK_READ_RETRY_BOUND {
                return None;
            }
            core::hint::spin_loop();
        }
    }

    /// Read one record slot under the seqlock retry protocol. Returns the consistent [`DiagEvent`]
    /// or `None` if the slot is mid-write / kept tearing past the retry bound (a slow loser, by
    /// design — the reader skips it rather than blocking the writer).
    fn read_slot(&self, slot: usize) -> Option<DiagEvent> {
        let slot_base = HEADER_SIZE + slot * RECORD_SLOT_SIZE;
        let seq = atomic_u32_at(&self.map, slot_base + SLOT_SEQ_OFF);
        let event_off = slot_base + SLOT_EVENT_OFF;

        let mut retries = 0u32;
        loop {
            let s1 = seq.load(Ordering::Acquire);
            if s1 & 1 == 0 {
                // Read the payload bytes into a local POD copy.
                let mut event = DiagEvent::zeroed();
                // SAFETY: event_off + DIAG_EVENT_SIZE <= map.len(); reading plain bytes into a POD
                // local; the seqlock check below rejects the value if it raced a write.
                unsafe {
                    let src = self.map.as_ptr().add(event_off);
                    core::ptr::copy_nonoverlapping(
                        src,
                        bytemuck::bytes_of_mut(&mut event).as_mut_ptr(),
                        DIAG_EVENT_SIZE,
                    );
                }
                let s2 = seq.load(Ordering::Acquire);
                if s1 == s2 {
                    // A seq of 0 means the slot was never written (still zeroed) — reject it so the
                    // caller does not surface a phantom all-zero record.
                    if s1 == 0 {
                        return None;
                    }
                    return Some(event);
                }
            }
            retries += 1;
            if retries >= SEQLOCK_READ_RETRY_BOUND {
                return None;
            }
            core::hint::spin_loop();
        }
    }

    /// Read up to the last `n` records, walking BACK from the current `write_index`, newest first.
    /// Skips any slot that is mid-write / torn / never-written. Returns only CONSISTENT records.
    pub fn read_last_n(&self, n: u32) -> Vec<DiagEvent> {
        let write_index = atomic_u32_at(&self.map, OFF_WRITE_INDEX).load(Ordering::Acquire);
        let cap = self.capacity;
        let want = n.min(cap).min(write_index);
        let mut out = Vec::with_capacity(want as usize);
        // The most recently written slot is (write_index - 1) % capacity; walk backwards from there.
        for back in 0..want {
            // write_index is the NEXT slot to write; last written is write_index-1.
            let logical = write_index.wrapping_sub(1).wrapping_sub(back);
            let slot = (logical % cap) as usize;
            if let Some(event) = self.read_slot(slot) {
                out.push(event);
            }
        }
        out
    }
}

/// Build a default backing-file path under the OS temp dir for `session_id`, with the session id in
/// the name so parallel Handshake instances do not collide. The PATH (not a Win32 object name) is
/// what Handshake passes to Palmistry.
pub fn default_backing_path(session_id: &str) -> std::path::PathBuf {
    // Sanitize the session id to a filesystem-safe token (no spaces / separators) per CX-109A.
    let safe: String = session_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    std::env::temp_dir().join(format!("handshake-diag-{safe}.ring"))
}

/// Internal helper used by tests to write a single byte at an offset (e.g. to corrupt magic for the
/// foreign-map refusal test). Not part of the public ring protocol.
#[doc(hidden)]
pub fn debug_corrupt_byte(path: &Path, offset: u64, value: u8) -> io::Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(&[value])?;
    file.flush()?;
    Ok(())
}
