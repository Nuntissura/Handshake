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
//!
//! # Locks (MT-215 N2, AC215-6)
//!
//! The slot's lock guards arithmetic and nothing else: no thread posts, sends, sets the timer or makes any other call
//! into the system while it holds it. A decision is made under the lock (counting a post as taken before it is made, so
//! that no second thread decides on the same number), and carried out after it is let go. The loop's thread, which beats,
//! never waits for the lock: it takes it with `try_lock`, and where a pool thread holds it, leaves what it wanted in the
//! lane's `Handoff`, which the holder works through before it lets go and looks at again after. The timer is set
//! outside the lock from the newest decision's `want`, so an overtaken thread sets the newer one. Pool threads (the
//! timer's callback, a waker of another thread) may wait for the lock, for one decision at a time.

use std::collections::hash_map::RandomState;
use std::ffi::c_void;
use std::hash::{BuildHasher, Hasher};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{FILETIME, HWND, LPARAM, TRUE};
use windows_sys::Win32::System::Threading::{
    CloseThreadpoolTimer, CreateThreadpoolTimer, GetCurrentThreadId, SetThreadpoolTimer,
    WaitForThreadpoolTimerCallbacks, PTP_CALLBACK_INSTANCE, PTP_TIMER,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetQueueStatus, PeekMessageW, PostMessageW, SendNotifyMessageW, MSG, PM_NOREMOVE, PM_NOYIELD,
    PM_QS_POSTMESSAGE, QS_INPUT, WM_APP,
};

use crate::handoff::{Handoff, Mail};
use crate::wake::{LaneId, WakeRefused};
use crate::wake_slot::{nanos, Step, Taken, WakeSlot};

/// Posted to the service window; the first parameter is the lane, the second the post's number with the lane's mask.
pub(super) const WM_WAKE: u32 = WM_APP + 1;

/// Sent to the service window by [`Pulse`]; the second parameter is the pulse's secret.
pub(super) const WM_PROBE: u32 = WM_APP + 2;

/// Signs that the loop takes posts out of its queue, shared by the lanes of one loop (MT-214 R2-N3, E3-F4).
///
/// A dispatched wake is one. Where none comes, a probe asks: a message sent, not posted, to the service window. The
/// system dispatches a sent message inside every call that looks at the thread's queue (PeekMessage, Remarks), ours
/// and anybody else's, so a loop that runs answers it whatever it does with posts, and one that hangs does not. A
/// thread that waits in a SendMessage to another thread answers it too: "the sending thread will process incoming
/// nonqueued messages while waiting" (SendMessage, Remarks), and takes nothing out of its queue meanwhile. So an
/// answer counts only while no wake post waits in the queue ([`posts_wait`]): the posts were taken out and thrown
/// away, and the next one is useful. While one waits, the answer says nothing a post would change. Neither does a
/// request made on the loop's thread, which may run inside such a wait. At most one probe is under way, and it is
/// always sent from a pool thread: sent from the loop's thread it would be answered inside the call that sends it.
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

    /// The loop took a post out of its queue: called on its thread.
    fn alive(&self) {
        self.life.fetch_max(self.now(), Ordering::AcqRel);
    }

    fn on_loop_thread(&self) -> bool {
        // SAFETY: no arguments.
        unsafe { GetCurrentThreadId() == self.thread }
    }

    /// Ask the loop whether it runs; its answer comes to [`Pulse::answered`]. Not on the loop's thread.
    fn ask(&self) {
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

    /// The service window got a probe, on the loop's thread. Returns whether it was this pulse's, answered while no
    /// wake post waited: a sign that the loop takes posts out of its queue (E3-F4).
    pub(super) fn answered(&self, token: LPARAM) -> bool {
        if token as usize != self.secret {
            return false;
        }
        let counts = !posts_wait(self.window);
        if counts {
            self.alive();
        }
        self.asking.store(false, Ordering::Release);
        counts
    }
}

/// Whether a wake post waits in the calling thread's queue, for `window`. Takes nothing out and processes only posted
/// messages (PeekMessage: `PM_NOREMOVE`, `PM_QS_POSTMESSAGE`, a filter of one message).
fn posts_wait(window: HWND) -> bool {
    // SAFETY: a message structure that outlives the call; nothing is taken out of the queue.
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        let flags = PM_NOREMOVE | PM_NOYIELD | PM_QS_POSTMESSAGE;
        PeekMessageW(&mut message, window, WM_WAKE, WM_WAKE, flags) != 0
    }
}

/// What a decision under the slot's lock asks its thread to do once it has let go: operating-system calls, which are
/// never made under the lock.
#[derive(Clone, Copy, Default)]
struct Act {
    /// Post the wake with this number.
    post: Option<u64>,
    /// Post a dispatched message again (it was dispatched while the slot was held).
    repost: Option<LPARAM>,
    /// Ask the loop whether it runs.
    ask: bool,
}

/// The timer when no look is wanted.
const NO_TIMER: u64 = u64::MAX;

/// How many times one thread works through what others left for it, before it leaves the rest to the timer.
const PUMPS: usize = 4;

struct LaneState {
    id: u32,
    /// Waits a moment after a dispatch while input waits, so it cannot keep input out.
    yields: bool,
    /// What the post's number is masked with.
    mask: usize,
    open: AtomicBool,
    /// The wake's state. Held for arithmetic on it only: no thread makes an operating-system call, posts, sets the timer
    /// or sends while it holds this (MT-215 N2).
    slot: Mutex<WakeSlot>,
    pulse: Arc<Pulse>,
    /// What the loop's thread leaves when it cannot take the slot.
    hand: Handoff,
    /// When the newest decision wants the timer to fire (the lane's clock), or [`NO_TIMER`].
    want: AtomicU64,
}

impl LaneState {
    fn now(&self) -> u64 {
        self.pulse.now()
    }

    /// The slot, waiting for it. Pool threads only: the callback and the wakers of other threads. A holder holds it for
    /// arithmetic, never across a call into the system, so the wait is short, and nobody is held up by it who must not
    /// wait.
    fn slot(&self) -> MutexGuard<'_, WakeSlot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Work through what others left, as the holder of the slot.
    fn drain(&self, slot: &mut WakeSlot, now: u64, act: &mut Act) {
        match self.hand.take_mail() {
            Some(Mail::Request { value, due }) => slot.arm_at(value, due),
            Some(Mail::Cancel) => slot.cancel(),
            None => {}
        }
        if let Some(number) = self.hand.take_refused() {
            slot.unposted(now, number);
        }
        if let Some(token) = self.hand.take_retake() {
            act.repost = Some(token as LPARAM);
        }
        self.hand.take_look();
    }

    /// Decide what is needed now, and count it as done: a post is counted as taken before it is made, so no second
    /// thread decides on the same number. Pure: no call into the system.
    fn decide(&self, slot: &mut WakeSlot, now: u64, act: &mut Act) {
        if !self.open.load(Ordering::Acquire) {
            slot.cancel();
            self.want.store(NO_TIMER, Ordering::Release);
            return;
        }
        let want = match slot.step(now, self.pulse.life()) {
            Step::Idle => NO_TIMER,
            Step::LookAt(at) => at,
            // Asked on the loop's thread, the pool asks at once.
            Step::Ask(_) if self.pulse.on_loop_thread() => now,
            Step::Ask(at) => {
                act.ask = true;
                at
            }
            Step::Post(number) => {
                act.post = Some(number);
                slot.posted(now, number, true)
            }
        };
        self.want.store(want, Ordering::Release);
    }

    /// `op` on the slot, then what is needed now, as the holder: drained first, so `op` sees what was left before it.
    fn settle<T>(
        &self,
        slot: &mut WakeSlot,
        op: impl FnOnce(&mut WakeSlot, u64) -> T,
    ) -> (T, Act) {
        let now = self.now();
        let mut act = Act::default();
        self.drain(slot, now, &mut act);
        let out = op(slot, now);
        self.decide(slot, now, &mut act);
        (out, act)
    }

    /// [`Self::settle`] with the slot, waiting for it. Pool threads only.
    fn locked<T>(&self, op: impl FnOnce(&mut WakeSlot, u64) -> T) -> (T, Act) {
        let mut slot = self.slot();
        self.settle(&mut slot, op)
    }

    /// [`Self::settle`] with the slot if it is free right now; `None` if another thread holds it. Never waits: the loop's
    /// thread takes the slot this way and leaves what it wanted with [`Handoff`] when it is not free.
    fn try_locked<T>(&self, op: impl FnOnce(&mut WakeSlot, u64) -> T) -> Option<(T, Act)> {
        let mut slot = match self.slot.try_lock() {
            Ok(slot) => slot,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        };
        Some(self.settle(&mut slot, op))
    }

    /// Make the calls into the system that a decision asked for, outside the lock, and bring the timer to what the
    /// newest decision wants.
    fn perform(&self, act: Act, timer: PTP_TIMER) {
        if let Some(token) = act.repost {
            // SAFETY: plain values. A refusal is made good by the post that is still open.
            unsafe { PostMessageW(self.pulse.window, WM_WAKE, self.id as usize, token) };
        }
        if let Some(number) = act.post {
            let token = (number as usize ^ self.mask) as LPARAM;
            // SAFETY: plain values. Posting to a window that is gone fails and is reported as refused.
            let taken =
                unsafe { PostMessageW(self.pulse.window, WM_WAKE, self.id as usize, token) } != 0;
            if !taken {
                self.hand.refuse(number);
            }
        }
        if act.ask {
            self.pulse.ask();
        }
        self.arm(timer);
    }

    /// Set the timer to what the newest decision wants. Decisions are made one at a time under the lock but their
    /// timers are set outside it, so a thread whose timer was overtaken sets the newer one: it looks again at `want`
    /// after the call, and the thread that made the newer decision sets it too. Bounded.
    fn arm(&self, timer: PTP_TIMER) {
        for _ in 0..PUMPS {
            let want = self.want.load(Ordering::Acquire);
            set_timer(timer, (want != NO_TIMER).then(|| want.saturating_sub(self.now())));
            if self.want.load(Ordering::Acquire) == want {
                return;
            }
        }
    }

    /// [`Self::perform`], and what others left meanwhile, for the loop's thread: never waits. What it cannot take the
    /// slot for, the thread that holds it works through.
    fn pump_loop(&self, timer: PTP_TIMER, mut act: Act) {
        for _ in 0..PUMPS {
            self.perform(act, timer);
            if !self.hand.pending_after_release() {
                return;
            }
            let Some(((), next)) = self.try_locked(|_, _| ()) else {
                return;
            };
            act = next;
        }
        self.leave_to_timer(timer);
    }

    /// [`Self::perform`], and what others left meanwhile, for a pool thread, which may wait for the slot.
    fn pump_pool(&self, timer: PTP_TIMER, mut act: Act) {
        for _ in 0..PUMPS {
            self.perform(act, timer);
            if !self.hand.pending_after_release() {
                return;
            }
            let ((), next) = self.locked(|_, _| ());
            act = next;
        }
        self.leave_to_timer(timer);
    }

    /// Too much was left to work through now: the timer fires at once and does the rest.
    fn leave_to_timer(&self, timer: PTP_TIMER) {
        self.want.store(self.now(), Ordering::Release);
        self.arm(timer);
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
    let ((), act) = state.locked(|_, _| ());
    state.pump_pool(timer, act);
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
///
/// The loop's thread never waits for the slot (MT-215 N2, AC215-6): [`Lane::take`], [`Lane::request`], [`Lane::cancel`]
/// and [`Lane::nudge`] take it with `try_lock` and, when a pool thread holds it, leave what they wanted with the
/// [`Handoff`], which the holder works through before and after it lets go.
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
            hand: Handoff::default(),
            want: AtomicU64::new(NO_TIMER),
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
        if !self.state.open.load(Ordering::Acquire) {
            return Err(WakeRefused::Closed);
        }
        if !self.on_loop_thread() {
            return self.request_from_a_pool_thread(value, delay);
        }
        let due = self.state.now().saturating_add(nanos(delay));
        match self.state.try_locked(|slot, _| slot.arm_at(value, due)) {
            Some(((), act)) => self.state.pump_loop(self.timer.timer, act),
            None => {
                self.state.hand.put(Mail::Request { value, due });
                self.state.pump_loop(self.timer.timer, Act::default());
            }
        }
        Ok(())
    }

    /// [`Lane::request`] from a thread that is not the loop's, which may wait for the slot.
    fn request_from_a_pool_thread(&self, value: u64, delay: Duration) -> Result<(), WakeRefused> {
        let due = self.state.now().saturating_add(nanos(delay));
        let ((), act) = self.state.locked(|slot, _| slot.arm_at(value, due));
        self.state.pump_pool(self.timer.timer, act);
        Ok(())
    }

    /// [`Lane::cancel`] from a thread that is not the loop's, which may wait for the slot.
    fn cancel_from_a_pool_thread(&self) {
        let ((), act) = self.state.locked(|slot, _| slot.cancel());
        self.state.pump_pool(self.timer.timer, act);
    }

    pub(crate) fn cancel(&self) {
        if !self.on_loop_thread() {
            self.cancel_from_a_pool_thread();
            return;
        }
        match self.state.try_locked(|slot, _| slot.cancel()) {
            Some(((), act)) => self.state.pump_loop(self.timer.timer, act),
            None => {
                self.state.hand.put(Mail::Cancel);
                self.state.pump_loop(self.timer.timer, Act::default());
            }
        }
    }

    /// No more wakes, from any holder.
    pub(crate) fn close(&self) {
        self.state.open.store(false, Ordering::Release);
        self.cancel();
    }

    /// The loop answered a probe: look now whether to post, on the loop's thread.
    pub(super) fn nudge(&self) {
        if !self.state.open.load(Ordering::Acquire) {
            return;
        }
        match self.state.try_locked(|_, _| ()) {
            Some(((), act)) => self.state.pump_loop(self.timer.timer, act),
            None => {
                self.state.hand.look();
                self.state.pump_loop(self.timer.timer, Act::default());
            }
        }
    }

    /// The service window got a post of this lane, on the loop's thread. A message that is not one of its open posts
    /// gives nothing; any dispatched post shows that the loop runs.
    ///
    /// Never waits (MT-215 N2): when a pool thread holds the slot the message is posted again at once and comes round
    /// once more, a moment later, instead of the loop's thread waiting for the slot.
    pub(super) fn take(&self, token: LPARAM) -> Option<Taken> {
        self.state.pulse.alive();
        if !self.state.open.load(Ordering::Acquire) {
            return None;
        }
        let number = self.state.number(token);
        let yields = self.state.yields && input_waits();
        match self
            .state
            .try_locked(|slot, now| slot.dispatched(now, number, yields))
        {
            // A wake that is not due yet keeps its timer; one that is due and has no post open is posted.
            Some((taken, act)) => {
                self.state.pump_loop(self.timer.timer, act);
                taken
            }
            None => {
                self.state.hand.retake(token as u64);
                self.state.pump_loop(self.timer.timer, Act::default());
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lane on this thread, as the loop's thread holds it, whose window is a handle nothing owns: a post to it is
    /// refused, so the refusal path runs too, and nothing is queued anywhere.
    fn lane_on_this_thread() -> Lane {
        // SAFETY: no arguments.
        let thread = unsafe { GetCurrentThreadId() };
        let window: HWND = 0x7777;
        let pulse = Pulse::new(window, thread, Instant::now());
        Lane::open(5, &pulse, false).expect("a pool timer")
    }

    /// MT-215 N2 (AC215-6): while a pool thread holds the slot, the loop's thread asks, cancels and takes a dispatched
    /// post without waiting; what it asked for is done by the next holder. A blocking lock here would hang the test.
    #[test]
    fn the_loop_thread_never_waits_for_a_held_slot() {
        let lane = lane_on_this_thread();
        let held = lane.state.slot();
        assert!(lane.request(1, Duration::ZERO).is_ok(), "a request with the slot held");
        assert!(lane.state.hand.pending_after_release(), "it was left for the holder");
        assert!(lane.take(0x1234).is_none(), "a dispatch with the slot held is not lost, nor waited for");
        assert!(lane.state.hand.take_retake().is_some(), "it comes round again");
        lane.cancel();
        lane.nudge();
        drop(held);

        // The next holder works through what was left: the cancel was newest, so nothing is armed.
        lane.nudge();
        assert!(!lane.state.hand.pending_after_release());
        assert_eq!(lane.state.slot().step(lane.state.now(), 0), Step::Idle, "the cancel won");
        assert!(lane.request(2, Duration::ZERO).is_ok());
        assert_ne!(
            lane.state.slot().step(lane.state.now(), 0),
            Step::Idle,
            "a request with the slot free is armed at once"
        );
    }

    /// A request left while the slot was held is armed by whoever takes the slot next, with its own due time.
    #[test]
    fn what_was_left_for_the_holder_is_done_when_the_slot_is_next_taken() {
        let lane = lane_on_this_thread();
        let held = lane.state.slot();
        assert!(lane.request(9, Duration::from_secs(3_600)).is_ok());
        drop(held);
        lane.nudge();
        let step = lane.state.slot().step(lane.state.now(), 0);
        assert!(
            matches!(step, Step::LookAt(at) if at > lane.state.now() + nanos(Duration::from_secs(3_000))),
            "armed for the hour it was asked for: {step:?}"
        );
        assert!(lane.state.want.load(Ordering::Acquire) > lane.state.now());
    }
}
