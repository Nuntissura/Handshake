//! The test binary's allocator, counting each thread's allocations, so a test can show that the heartbeat's beat path
//! on the loop's thread allocates nothing (MT-215 E6). Only the calling thread's count is read; other threads' do not
//! disturb it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

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

/// Allocations the calling thread has made so far.
pub fn allocations() -> u64 {
    ALLOCATIONS.with(Cell::get)
}

/// What a stretch of code on the calling thread allocated, summed over the times it ran.
#[derive(Default)]
pub struct Tally {
    allocations: Cell<u64>,
    runs: Cell<u64>,
}

impl Tally {
    /// Run `work`, and add what it allocated on this thread to the tally when `counts` says the run counts.
    pub fn count<T>(&self, work: impl FnOnce() -> T, counts: impl FnOnce(&T) -> bool) -> T {
        let before = allocations();
        let done = work();
        let made = allocations() - before;
        if counts(&done) {
            self.allocations.set(self.allocations.get() + made);
            self.runs.set(self.runs.get() + 1);
        }
        done
    }

    pub fn allocations(&self) -> u64 {
        self.allocations.get()
    }

    pub fn runs(&self) -> u64 {
        self.runs.get()
    }
}
