//! AC215-6: the hot path allocates nothing.

mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagRingWriter, DEFAULT_CAPACITY};
use support::RingFile;
use ui_heartbeat::{
    EventLoopWake, ProcessClock, UiHeartbeat, WakeOutcome, WakeRefused, WakeTicket,
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

/// Holds the one wake in flight and can refuse. Allocates nothing itself.
#[derive(Default)]
struct SlotLoop {
    slot: Cell<Option<WakeTicket>>,
    refuse: Cell<bool>,
}

impl EventLoopWake for &SlotLoop {
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

#[test]
fn wakes_turns_refusals_and_ignored_wakes_allocate_nothing() {
    let ring = RingFile::new("allocation");
    let writer = DiagRingWriter::create(ring.path(), DEFAULT_CAPACITY).expect("ring");
    let event_loop = SlotLoop::default();
    let heartbeat = UiHeartbeat::new(
        ProcessClock::since(Instant::now()),
        &event_loop,
        Some(writer),
    );
    heartbeat.start().expect("starts");

    let mut beats = 0_u64;
    let before = ALLOCATIONS.with(Cell::get);
    for round in 0..10_000_u64 {
        event_loop.refuse.set(round % 7 == 0);
        if let Some(ticket) = event_loop.slot.take() {
            if let WakeOutcome::Beat(_) = heartbeat.on_wake(ticket) {
                beats += 1;
            }
            heartbeat.on_wake(ticket);
        }
        event_loop.refuse.set(false);
        heartbeat.on_loop_turn();
        heartbeat.on_system_resume();
    }
    let after = ALLOCATIONS.with(Cell::get);

    assert_eq!(after - before, 0, "allocations on the hot path");
    assert!(beats > 1_000, "the path was exercised: {beats}");
    assert!(heartbeat.counters().refused_wakes > 0);

    // The counter itself works.
    let probe = std::hint::black_box(Box::new(1_u64));
    assert!(ALLOCATIONS.with(Cell::get) > after);
    drop(probe);
    drop(heartbeat);
}
