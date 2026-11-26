//! Wake lanes over a posted message and a pool timer.
//!
//! A wake is a message posted to a window, because that is what every loop on the thread dispatches: ours, and the
//! operating system's own for moving, sizing, menus and dialogs. A timer message is not used: it is delivered only
//! when the queue holds nothing else. The pool timer's thread posts, sends a probe now and then, and does nothing else.
//!
//! Each post carries its number, masked with a secret of the lane, so a message that another program or another part
//! of this one posts or sends with the same number neither settles a post nor dispatches a wake (MT-214 W7, MT-215
//! HB-7). A lane opened under the number of a dropped one has a secret of its own, so the posts of the dropped lane
//! that are still in the queue dispatch nothing either (R2-N4).

use std::collections::hash_map::RandomState;
use std::ffi::c_void;
use std::hash::{BuildHasher, Hasher};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{FILETIME, HWND, LPARAM, TRUE};
use windows_sys::Win32::System::Threading::{
    CloseThreadpoolTimer, CreateThreadpoolTimer, GetCurrentThreadId, SetThreadpoolTimer,
    WaitForThreadpoolTimerCallbacks, PTP_CALLBACK_INSTANCE, PTP_TIMER,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetQueueStatus, PostMessageW, SendNotifyMessageW, QS_INPUT, WM_APP,
};

use crate::wake::{LaneId, WakeRefused};
use crate::wake_slot::{nanos, Step, Taken, WakeSlot};

/// Posted to the service window; the first parameter is the lane, the second the post's number with the lane's mask.
pub(super) const WM_WAKE: u32 = WM_APP + 1;

/// Sent to the service window by [`Pulse`]; the second parameter is the pulse's secret.
pub(super) const WM_PROBE: u32 = WM_APP + 2;

/// Signs that the loop runs, shared by the lanes of one loop (MT-214 R2-N3).
///
/// A dispatched wake is one. Where none comes, a probe asks: a message sent, not posted, to the service window. The
/// system dispatches a sent message inside every call that looks at the thread's queue (PeekMessage, Remarks), ours
/// and anybody else's, so a loop that runs answers it whatever it does with posts, and one that hangs does not. At
/// most one probe is under way.
pub(super) struct Pulse {
    window: HWND,
    thread: u32,
    epoch: Instant,
    secret: usize,
    /// When the loop last showed that it runs; 0 for never.
    life: AtomicU64,
    asking: AtomicBool,
}

impl Pulse {
    pub(super) fn new(window: HWND, thread: u32, epoch: Instant) -> Arc<Self> {
        Arc::new(Self {
            window,
            thread,
            epoch,
            secret: mask(u32::MAX),
            life: AtomicU64::new(0),
            asking: AtomicBool::new(false),
        })
    }

    fn now(&self) -> u64 {
        nanos(self.epoch.elapsed())
    }

    fn life(&self) -> u64 {
        self.life.load(Ordering::Acquire)
    }

    /// The loop runs: called on its thread.
    fn alive(&self) {
        self.life.fetch_max(self.now(), Ordering::AcqRel);
    }

    fn on_loop_thread(&self) -> bool {
        // SAFETY: no arguments.
        unsafe { GetCurrentThreadId() == self.thread }
    }

    /// Ask the loop whether it runs; its answer comes to [`Pulse::answered`].
    fn ask(&self) {
        if self.on_loop_thread() {
            self.alive();
            return;
        }
        if self.asking.swap(true, Ordering::AcqRel) {
            return;
        }
        // SAFETY: plain values. To a window of another thread it returns at once (SendNotifyMessage); to a window
        // that is gone it fails.
        let sent = unsafe { SendNotifyMessageW(self.window, WM_PROBE, 0, self.secret as LPARAM) };
        if sent == 0 {
            self.asking.store(false, Ordering::Release);
        }
    }

    /// The service window got a probe, on the loop's thread. Returns whether it was this pulse's.
    pub(super) fn answered(&self, token: LPARAM) -> bool {
        if token as usize != self.secret {
            return false;
        }
        self.alive();
        self.asking.store(false, Ordering::Release);
        true
    }
}

struct LaneState {
    id: u32,
    /// Waits a moment after a dispatch while input waits, so it cannot keep input out.
    yields: bool,
    /// What the post's number is masked with.
    mask: usize,
    open: AtomicBool,
    slot: Mutex<WakeSlot>,
    pulse: Arc<Pulse>,
}

impl LaneState {
    fn now(&self) -> u64 {
        self.pulse.now()
    }

    fn slot(&self) -> MutexGuard<'_, WakeSlot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Carry out a step. The slot stays locked, so no two threads set the timer against each other.
    fn carry_out(&self, slot: &mut WakeSlot, step: Step, now: u64, timer: PTP_TIMER) {
        let look_at = match step {
            Step::Idle => None,
            Step::LookAt(at) => Some(at),
            Step::Ask(at) => {
                self.pulse.ask();
                Some(at)
            }
            Step::Post(number) => {
                let token = (number as usize ^ self.mask) as LPARAM;
                // SAFETY: plain values. Posting to a window that is gone fails and is reported as refused.
                let taken = unsafe {
                    PostMessageW(self.pulse.window, WM_WAKE, self.id as usize, token)
                } != 0;
                Some(slot.posted(now, number, taken))
            }
        };
        set_timer(timer, look_at.map(|at| at.saturating_sub(now)));
    }

    /// Look at the slot now, and do what it says.
    fn step_now(&self, slot: &mut WakeSlot, timer: PTP_TIMER) {
        let now = self.now();
        let step = slot.step(now, self.pulse.life());
        self.carry_out(slot, step, now, timer);
    }

    /// The number a dispatched message names. On a 32-bit system the high half of a number is not carried, and the
    /// numbers of one lane stay below it.
    fn number(&self, token: LPARAM) -> u64 {
        (token as usize ^ self.mask) as u64
    }
}

fn set_timer(timer: PTP_TIMER, after: Option<u64>) {
    let Some(after) = after else {
        // SAFETY: `timer` is open; no due time means no more callbacks are queued.
        unsafe { SetThreadpoolTimer(timer, ptr::null(), 0, 0) };
        return;
    };
    // A negative due time is relative, in units of 100 ns.
    let due = -(i64::try_from(after / 100).unwrap_or(i64::MAX).max(1));
    let due = FILETIME {
        dwLowDateTime: due as u32,
        dwHighDateTime: (due >> 32) as u32,
    };
    // SAFETY: `timer` is open and `due` outlives the call.
    unsafe { SetThreadpoolTimer(timer, &due, 0, 0) };
}

unsafe extern "system" fn on_timer(
    _: PTP_CALLBACK_INSTANCE,
    context: *mut c_void,
    timer: PTP_TIMER,
) {
    // SAFETY: the context is the state the timer holds a reference to; it is released only after the last callback.
    let state = unsafe { &*context.cast::<LaneState>() };
    if !state.open.load(Ordering::Acquire) {
        return;
    }
    let mut slot = state.slot();
    state.step_now(&mut slot, timer);
}

/// The pool timer, and its reference to the state.
struct LaneTimer {
    timer: PTP_TIMER,
    context: *const LaneState,
}

// SAFETY: a pool timer may be used from any thread, and the context pointer is only given back when the timer closes.
unsafe impl Send for LaneTimer {}
// SAFETY: as above; every use through a shared reference is one pool call.
unsafe impl Sync for LaneTimer {}

impl Drop for LaneTimer {
    fn drop(&mut self) {
        // SAFETY: the timer is open. No callback is queued or running after the wait, so none can use the state that
        // is released after it.
        unsafe {
            SetThreadpoolTimer(self.timer, ptr::null(), 0, 0);
            WaitForThreadpoolTimerCallbacks(self.timer, TRUE);
            CloseThreadpoolTimer(self.timer);
            drop(Arc::from_raw(self.context));
        }
    }
}

/// A secret: the standard library's per-process random keys, which differ for every value made from them, mixed with
/// a number.
fn mask(id: u32) -> usize {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u32(id);
    hasher.finish() as usize
}

/// Whether the Operator's input waits in the calling thread's queue (GetQueueStatus: the high word is what the queue
/// holds now).
fn input_waits() -> bool {
    // SAFETY: a plain value; reads the calling thread's queue.
    let status = unsafe { GetQueueStatus(QS_INPUT) };
    (status >> 16) & QS_INPUT != 0
}

/// One lane. Any thread may hold it.
#[derive(Clone)]
pub(crate) struct Lane {
    state: Arc<LaneState>,
    timer: Arc<LaneTimer>,
}

impl Lane {
    /// A lane whose wakes go to the pulse's window. `None` when the operating system gives no timer.
    pub(super) fn open(id: u32, pulse: &Arc<Pulse>, yields: bool) -> Option<Self> {
        let state = Arc::new(LaneState {
            id,
            yields,
            mask: mask(id),
            open: AtomicBool::new(true),
            slot: Mutex::new(WakeSlot::default()),
            pulse: Arc::clone(pulse),
        });
        let context = Arc::into_raw(Arc::clone(&state));
        // SAFETY: the callback matches, and the context lives until the timer is closed.
        let timer = unsafe {
            CreateThreadpoolTimer(Some(on_timer), context.cast_mut().cast(), ptr::null())
        };
        if timer == 0 {
            // SAFETY: nothing else took the reference.
            drop(unsafe { Arc::from_raw(context) });
            return None;
        }
        Some(Self {
            state,
            timer: Arc::new(LaneTimer { timer, context }),
        })
    }

    pub(crate) fn id(&self) -> LaneId {
        LaneId(self.state.id)
    }

    pub(crate) fn on_loop_thread(&self) -> bool {
        self.state.pulse.on_loop_thread()
    }

    pub(crate) fn request(&self, value: u64, delay: Duration) -> Result<(), WakeRefused> {
        let mut slot = self.state.slot();
        if !self.state.open.load(Ordering::Acquire) {
            return Err(WakeRefused::Closed);
        }
        if self.on_loop_thread() {
            self.state.pulse.alive();
        }
        let now = self.state.now();
        let step = slot.request(now, value, nanos(delay), self.state.pulse.life());
        self.state.carry_out(&mut slot, step, now, self.timer.timer);
        Ok(())
    }

    pub(crate) fn cancel(&self) {
        let mut slot = self.state.slot();
        slot.cancel();
        set_timer(self.timer.timer, None);
    }

    /// No more wakes, from any holder.
    pub(crate) fn close(&self) {
        self.state.open.store(false, Ordering::Release);
        self.cancel();
    }

    /// The loop answered a probe: look now whether to post, on the loop's thread.
    pub(super) fn nudge(&self) {
        let mut slot = self.state.slot();
        if self.state.open.load(Ordering::Acquire) {
            self.state.step_now(&mut slot, self.timer.timer);
        }
    }

    /// The service window got a post of this lane, on the loop's thread. A message that is not one of its open posts
    /// gives nothing; any dispatched post shows that the loop runs.
    pub(super) fn take(&self, token: LPARAM) -> Option<Taken> {
        self.state.pulse.alive();
        let mut slot = self.state.slot();
        if !self.state.open.load(Ordering::Acquire) {
            return None;
        }
        let now = self.state.now();
        let number = self.state.number(token);
        let taken = slot.dispatched(now, number, self.state.yields && input_waits());
        // A wake that is not due yet keeps its timer; one that is due and has no post open is posted.
        self.state.step_now(&mut slot, self.timer.timer);
        taken
    }
}
