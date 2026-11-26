//! AC215-6: the hot path allocates nothing, beats during present stalls and the stall events included; and MT-215 F2: a beat before the
//! diagnostics recorder has its ring writer neither creates the recorder nor allocates, so the writer can still be
//! installed and the next beat reaches the ring.

mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagRingReader, DiagRingWriter, DEFAULT_CAPACITY};
use support::{RingFile, TimeLimit, VIRTUAL_LIMIT};
use ui_heartbeat::{
    EventLoopWake, FrameWait, ProcessClock, RecorderHeartbeat, UiHeartbeat, WakeOutcome,
    WakeRefused, WakeTicket, PRESENT_STALL,
};

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

/// Counts the calling thread's allocations.
struct Counting;

impl Counting {
    fn note() {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}

// SAFETY: every call is forwarded unchanged to the system allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::note();
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        Self::note();
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn allocations() -> u64 {
    ALLOCATIONS.with(Cell::get)
}

/// Holds the one wake in flight and can refuse. Allocates nothing itself.
#[derive(Default)]
struct SlotLoop {
    slot: Cell<Option<WakeTicket>>,
    refuse: Cell<bool>,
}

impl EventLoopWake for &SlotLoop {
    type Dispatched<'a> = WakeTicket;

    fn dispatched_ticket(&self, wake: &WakeTicket) -> Option<WakeTicket> {
        Some(*wake)
    }

    fn on_loop_thread(&self) -> bool {
        true
    }

    fn request_wake(&self, ticket: WakeTicket, _: Duration) -> Result<(), WakeRefused> {
        if self.refuse.get() {
            return Err(WakeRefused::Unavailable);
        }
        self.slot.set(Some(ticket));
        Ok(())
    }

    fn cancel_wakes(&self) {
        self.slot.set(None);
    }
}

/// Frames that stall when told to. Allocates nothing.
#[derive(Default)]
struct Stalls(Cell<Option<Duration>>);

impl FrameWait for &Stalls {
    fn longest_wait(&self) -> Option<Duration> {
        self.0.get()
    }
}

#[test]
fn wakes_turns_refusals_stalls_and_ignored_wakes_allocate_nothing() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let ring = RingFile::new("allocation");
    let writer = DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("ring");
    let event_loop = SlotLoop::default();
    let stalls = Stalls::default();
    let heartbeat = UiHeartbeat::with_frames(
        ProcessClock::since(Instant::now()),
        &event_loop,
        Some(writer),
        &stalls,
    );
    heartbeat.start().expect("starts");

    let mut beats = 0_u64;
    let mut beats_during_stalls = 0_u64;
    let before = allocations();
    for round in 0..10_000_u64 {
        event_loop.refuse.set(round % 7 == 0);
        // A stall begins every 100 rounds and lasts 40: its events are written to the ring on this thread.
        stalls
            .0
            .set((round % 100 >= 60).then_some(PRESENT_STALL + Duration::from_millis(round % 9)));
        if let Some(ticket) = event_loop.slot.take() {
            if let WakeOutcome::Beat(_) = heartbeat.on_wake(&ticket) {
                beats += 1;
                if heartbeat.present_stalled() {
                    beats_during_stalls += 1;
                }
            }
            heartbeat.on_wake(&ticket);
        }
        event_loop.refuse.set(false);
        heartbeat.on_loop_turn();
        heartbeat.on_system_resume();
    }
    let after = allocations();

    assert_eq!(after - before, 0, "allocations on the hot path");
    assert!(beats > 1_000, "the path was exercised: {beats}");
    let counters = heartbeat.counters();
    assert!(counters.refused_wakes > 0);
    assert!(counters.present_stalls > 10, "{counters:?}");
    assert!(
        beats_during_stalls > 1_000,
        "beats went on during the stalls (AC215-6): {beats_during_stalls}"
    );

    // The counter itself works.
    let probe = std::hint::black_box(Box::new(1_u64));
    assert!(allocations() > after);
    drop(probe);
    drop(heartbeat);
}

/// The native host's recorder, as far as the ordering goes (`handshake_native/src/diagnostics/recorder.rs`): created on
/// its first use, with a writer only if one was installed before, and allocating its buffer when it is created.
struct Recorder {
    writer: Option<DiagRingWriter>,
    _buffer: Vec<u8>,
}

static RECORDER: OnceLock<Recorder> = OnceLock::new();

fn global() -> &'static Recorder {
    RECORDER.get_or_init(|| Recorder {
        writer: None,
        _buffer: Vec::with_capacity(512),
    })
}

/// `diagnostics::heartbeat`.
fn recorder_heartbeat(counter: u64, timestamp_nanos: u64) {
    if let Some(writer) = &global().writer {
        writer.write_heartbeat(counter, timestamp_nanos);
    }
}

/// `diagnostics::has_ring_writer`: looks, creates nothing.
fn recorder_has_ring_writer() -> bool {
    RECORDER.get().is_some_and(|recorder| recorder.writer.is_some())
}

/// `diagnostics::install`: only into a recorder that does not exist yet.
fn install(writer: DiagRingWriter) -> bool {
    RECORDER
        .set(Recorder {
            writer: Some(writer),
            _buffer: Vec::with_capacity(512),
        })
        .is_ok()
}

/// MT-215 F2: the loop dispatched the first wake before the host installed the ring writer. That beat must not create
/// the recorder writerless, which would keep the ring silent for the whole session, nor allocate on the UI thread.
#[test]
fn a_beat_before_the_ring_writer_is_installed_leaves_the_recorder_to_be_installed() {
    let _limit = TimeLimit::new(VIRTUAL_LIMIT);
    let ring = RingFile::new("recorder-order");
    let writer = DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("ring");
    let reader = DiagRingReader::open(ring.path()).expect("reader");
    let event_loop = SlotLoop::default();
    let heartbeat = UiHeartbeat::new(
        ProcessClock::since(Instant::now()),
        &event_loop,
        RecorderHeartbeat::new(recorder_heartbeat, recorder_has_ring_writer),
    );
    heartbeat.start().expect("starts");

    let ticket = event_loop.slot.take().expect("the first wake");
    let before = allocations();
    let early = heartbeat.on_wake(&ticket);
    assert_eq!(allocations() - before, 0, "the early beat allocated");
    assert!(matches!(early, WakeOutcome::Beat(beat) if beat.counter == 1));
    assert!(RECORDER.get().is_none(), "the early beat created the recorder");

    assert!(install(writer), "the ring writer could not be installed any more");
    let ticket = event_loop.slot.take().expect("the next wake");
    assert!(matches!(heartbeat.on_wake(&ticket), WakeOutcome::Beat(_)));
    assert_eq!(
        reader.read_heartbeat().map(|beat| beat.counter),
        Some(2),
        "the first beat after the install reaches the ring"
    );
    drop(reader);
}
