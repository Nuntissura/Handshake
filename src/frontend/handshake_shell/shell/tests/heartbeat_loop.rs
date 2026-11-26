//! The heartbeat on a real loop of the window crate, on a private desktop that receives no input, with no window of its
//! own: beats reach a real version 2 ring read from another thread; a wake of another lane never beats, whatever value
//! it carries (MT-215 HB-7); posts of the heartbeat's wake that another loop throws away are posted again by the lane,
//! with no loop turn to replace them (F1); a UI thread that blocks goes stale for longer than the watcher's threshold
//! while another thread posts forged wake messages, and comes back with its cadence and no burst. A second heartbeat on
//! the loop is refused (MT-215 E3), and the beat path allocates nothing (E6). One test, because a process has one loop.
//!
//! The host forwards no event but the loop's start and end to the heartbeat, so nothing but a dispatched wake of its
//! lane can make it beat or ask for a wake.
#![cfg(windows)]
#![allow(unsafe_code)]

#[path = "../../surface/tests/support/mod.rs"]
mod support;

#[path = "../../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs"]
mod limits;

mod allocations;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use handshake_diag_ring::ring::{DiagRingReaderV2, DiagRingWriterV2};
use handshake_diag_ring::DEFAULT_CAPACITY;
use handshake_shell::{OpenRefused, ShellHeartbeat};
use handshake_shell_window::{Control, DispatchedWake, Event, LoopConfig, WakeLane};
use ui_heartbeat::{WakeOutcome, MAX_BEAT_GAP, WAKE_DELAY};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, PeekMessageW, PostMessageW, MSG, PM_REMOVE, WM_APP,
};

use allocations::Tally;
use limits::{RingFile, TimeLimit};
use support::desktop::{on_private_desktop, run_script, spawn_on_this_desktop, wide};

const LIMIT: Duration = Duration::from_secs(120);
/// Ends the test before the private desktop's watchdog does, so its ring file is removed first (MT-215 E7).
const TIME_LIMIT: Duration = Duration::from_secs(110);
const STEP_LIMIT: Duration = Duration::from_secs(20);
const TICK: Duration = Duration::from_millis(10);
const NONCE: [u8; 16] = [0x15; 16];

/// The window crate's wake message (`window/src/platform/wake.rs`), as another program would post it.
const WM_WAKE: u32 = WM_APP + 1;

/// Palmistry's freeze threshold (`palmistry/src/freeze_detect.rs` FREEZE_THRESHOLD).
const WATCHER_THRESHOLD: Duration = Duration::from_millis(5_000);

/// How long the UI thread blocks: past the watcher's threshold.
const BLOCK: Duration = Duration::from_millis(6_000);

/// How long another loop throws the wake posts away: past three posts of the heartbeat's wake (0, 0.5 and 1.5 s).
const THROWN_AWAY_FOR: Duration = Duration::from_millis(2_000);

fn service_window() -> HWND {
    let service = wide("hsk_shell_service");
    // SAFETY: the name outlives the call; read only.
    let hwnd = unsafe { FindWindowW(service.as_ptr(), ptr::null()) };
    assert_ne!(hwnd, 0);
    hwnd
}

/// A loop that takes the wake posts out of the queue and throws them away, as somebody else's loop may.
fn throw_wake_posts_away(span: Duration) -> usize {
    let service = service_window();
    let until = Instant::now() + span;
    let mut thrown = 0;
    while Instant::now() < until {
        // SAFETY: a message structure of this function; only wake messages of the loop's own window are taken.
        unsafe {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, service, WM_WAKE, WM_WAKE, PM_REMOVE) != 0 {
                thrown += 1;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    thrown
}

/// The heartbeat counter as another thread sees it, sampled every 20 ms until stopped.
struct Sampler {
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<(Instant, u64)>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Sampler {
    fn start(path: PathBuf) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let (until, into) = (Arc::clone(&stop), Arc::clone(&samples));
        let thread = std::thread::spawn(move || {
            let reader = DiagRingReaderV2::open(&path, NONCE).expect("the sampler maps the ring");
            while !until.load(Ordering::Acquire) {
                let counter = reader.read_heartbeat().map_or(0, |beat| beat.counter);
                into.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push((Instant::now(), counter));
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        Self {
            stop,
            samples,
            thread: Some(thread),
        }
    }

    fn samples(&self) -> Vec<(Instant, u64)> {
        self.samples
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn finish(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        self.finish();
    }
}

type Run = Box<dyn FnMut(&Test, &Control) -> bool>;

struct Step {
    name: &'static str,
    run: Run,
}

fn step(name: &'static str, run: impl FnMut(&Test, &Control) -> bool + 'static) -> Step {
    Step {
        name,
        run: Box::new(run),
    }
}

struct Test {
    writer: RefCell<Option<DiagRingWriterV2>>,
    reader: DiagRingReaderV2,
    heartbeat: RefCell<Option<ShellHeartbeat<DiagRingWriterV2>>>,
    lane: RefCell<Option<WakeLane>>,
    steps: RefCell<VecDeque<Step>>,
    started: Cell<Instant>,
    sampler: RefCell<Sampler>,
    other_wakes: Cell<u64>,
    /// The value this lane's next wake carries.
    next_value: Cell<u64>,
    finished: Cell<bool>,
    /// Allocations on the loop's thread inside the heartbeat's beats (MT-215 E6).
    beats: Tally,
}

impl Test {
    fn counter(&self) -> u64 {
        self.reader.read_heartbeat().map_or(0, |beat| beat.counter)
    }

    fn with_heartbeat<T>(&self, use_it: impl FnOnce(&ShellHeartbeat<DiagRingWriterV2>) -> T) -> T {
        use_it(self.heartbeat.borrow().as_ref().expect("the heartbeat runs"))
    }

    fn tick(&self, value: u64) {
        if let Some(lane) = self.lane.borrow().as_ref() {
            lane.request(value, TICK).expect("the lane takes a wake");
        }
    }

    fn event(&self, control: &Control, event: Event) {
        match event {
            Event::LoopStarted => {
                let writer = self.writer.borrow_mut().take().expect("the ring writer");
                let heartbeat = ShellHeartbeat::open(control, Instant::now(), writer)
                    .map_err(|(refused, _)| refused)
                    .expect("a lane for the heartbeat");
                heartbeat.start().expect("starts on the loop's thread");
                assert!(heartbeat.is_running());
                // One heartbeat per loop (MT-215 E3): a second is refused before it takes a lane, its sink handed back.
                let second = ShellHeartbeat::open(control, Instant::now(), None::<DiagRingWriterV2>);
                match second {
                    Err((OpenRefused::AlreadyOpen, sink)) => assert!(sink.is_none()),
                    Err((other, _)) => panic!("a second heartbeat refused for {other:?}"),
                    Ok(_) => panic!("a second heartbeat opened on the loop"),
                }
                *self.heartbeat.borrow_mut() = Some(heartbeat);
                *self.lane.borrow_mut() = Some(control.open_wake_lane().expect("a lane"));
                self.started.set(Instant::now());
                self.tick(0);
            }
            Event::ShutdownBegan => self.sampler.borrow_mut().finish(),
            Event::LoopEnding => {
                self.with_heartbeat(|heartbeat| heartbeat.on_event(&Event::LoopEnding));
                self.lane.borrow_mut().take();
            }
            _ => {}
        }
    }

    fn wake(&self, control: &Control, wake: &DispatchedWake<'_>) {
        let routed = self.with_heartbeat(|heartbeat| {
            self.beats
                .count(|| heartbeat.on_wake(wake), Option::is_some)
        });
        if let Some(outcome) = routed {
            assert!(
                matches!(outcome, WakeOutcome::Beat(_)),
                "a wake of the heartbeat's lane, dispatched once: {outcome:?}"
            );
            return;
        }
        // Another lane's wake, whatever it carries, is never the heartbeat's (HB-7).
        self.other_wakes.set(self.other_wakes.get() + 1);
        if self.finished.get() {
            return;
        }
        let mut steps = self.steps.borrow_mut();
        let Some(step) = steps.front_mut() else {
            drop(steps);
            self.finished.set(true);
            control.begin_shutdown().expect("shutdown begins");
            self.tick(0);
            return;
        };
        if (step.run)(self, control) {
            steps.pop_front();
            self.started.set(Instant::now());
        } else {
            assert!(
                self.started.get().elapsed() < STEP_LIMIT,
                "step \"{}\" did not finish within {STEP_LIMIT:?}; {:?}",
                step.name,
                self.with_heartbeat(ShellHeartbeat::counters)
            );
        }
        drop(steps);
        self.tick(self.next_value.get());
    }
}

fn steps() -> Vec<Step> {
    let mark = Rc::new(Cell::new((0_u64, Instant::now())));
    let replaced = Rc::new(Cell::new(0_u64));
    let block = Rc::new(Cell::new((Instant::now(), Instant::now(), 0_u64)));
    let (mark_a, mark_b, mark_c, mark_d) = (
        Rc::clone(&mark),
        Rc::clone(&mark),
        Rc::clone(&mark),
        Rc::clone(&mark),
    );
    let (replaced_a, replaced_b) = (Rc::clone(&replaced), Rc::clone(&replaced));
    let (block_a, block_b) = (Rc::clone(&block), Rc::clone(&block));
    vec![
        step("the loop's wakes beat into the ring", |test, _| test.counter() >= 5),
        step("note the beat", move |test, _| {
            mark_a.set((test.counter(), Instant::now()));
            true
        }),
        step(
            "wakes of another lane carrying the heartbeat's next tickets beat nothing",
            move |test, _| {
                // The ticket the heartbeat waits for is about one more than its requests so far. This lane's next
                // wakes carry that number and the ones around it, one per tick.
                let counters = test.with_heartbeat(ShellHeartbeat::counters);
                let next = 1 + counters.beats + counters.replaced_wakes;
                let (_, since) = mark_b.get();
                let elapsed = since.elapsed();
                let offset = (elapsed.as_millis() / TICK.as_millis()) as u64;
                test.next_value.set(next.saturating_sub(3) + offset % 8);
                let beats = test.counter() - mark_b.get().0;
                let most = elapsed.as_millis() as u64 / WAKE_DELAY.as_millis() as u64 + 2;
                assert!(beats <= most, "{beats} beats in {elapsed:?}: more than the heartbeat's own");
                assert_eq!(counters.ignored_wakes, 0, "the port never saw another lane's wake");
                elapsed >= Duration::from_millis(500)
            },
        ),
        step("another loop throws the wake posts away", move |test, _| {
            test.next_value.set(u64::MAX);
            replaced_a.set(test.with_heartbeat(ShellHeartbeat::counters).replaced_wakes);
            let before = test.counter();
            let thrown = throw_wake_posts_away(THROWN_AWAY_FOR);
            assert!(thrown >= 3, "{thrown} posts thrown away");
            assert_eq!(test.counter(), before, "nothing beats while the thread is in here");
            mark_c.set((before, Instant::now()));
            true
        }),
        step("the lane posts the wake again and it beats", move |test, _| {
            let (before, _) = mark_d.get();
            if test.counter() <= before {
                return false;
            }
            assert_eq!(
                test.with_heartbeat(ShellHeartbeat::counters).replaced_wakes,
                replaced_b.get(),
                "the port replaced nothing: the lane's own post came through"
            );
            true
        }),
        step(
            "the UI thread blocks while another thread posts forged wakes",
            move |test, _| {
                let at_start = test.counter();
                let forging = Arc::new(AtomicBool::new(true));
                let still = Arc::clone(&forging);
                let service = service_window() as isize;
                let forger = spawn_on_this_desktop(move || {
                    let mut posted = 0_u64;
                    while still.load(Ordering::Acquire) {
                        for lane in 0..8_usize {
                            for token in [0_isize, 1, 2, 3, 7, 42, isize::MAX, -1] {
                                // SAFETY: plain values, to the loop's window.
                                if unsafe { PostMessageW(service as HWND, WM_WAKE, lane, token) } != 0 {
                                    posted += 1;
                                }
                            }
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    posted
                });
                let began = Instant::now();
                std::thread::sleep(BLOCK);
                let ended = Instant::now();
                forging.store(false, Ordering::Release);
                let posted = forger.join().expect("the forger");
                assert!(posted > 100, "{posted} forged posts");
                assert_eq!(test.counter(), at_start);
                block_a.set((began, ended, at_start));
                true
            },
        ),
        step("it was stale past the watcher's threshold and comes back without a burst", move |test, _| {
            let (began, ended, at_start) = block_b.get();
            let now = test.counter();
            if now <= at_start {
                return false;
            }
            if ended.elapsed() < Duration::from_secs(1) {
                return false;
            }
            let samples = test.sampler.borrow().samples();
            let during: Vec<u64> = samples
                .iter()
                .filter(|(at, _)| *at >= began + MAX_BEAT_GAP && *at <= ended)
                .map(|(_, counter)| *counter)
                .collect();
            assert!(during.len() > 100, "the other thread kept reading: {}", during.len());
            assert!(during.iter().all(|counter| *counter == at_start), "it beat while blocked");
            assert!(ended - began >= WATCHER_THRESHOLD);
            let after = samples
                .iter()
                .filter(|(at, _)| *at > ended)
                .map(|(_, counter)| *counter)
                .max()
                .unwrap_or(at_start);
            let most = ended.elapsed().as_millis() as u64 / WAKE_DELAY.as_millis() as u64 + 3;
            assert!(
                now - at_start <= most && after - at_start <= most,
                "{} beats within {:?} of the block: forged posts beat",
                now - at_start,
                ended.elapsed()
            );
            true
        }),
    ]
}

#[test]
fn the_heartbeat_beats_from_the_loops_wakes_only() {
    let _limit = TimeLimit::new(TIME_LIMIT);
    // Out here, so a failure on the loop's thread still unwinds to it and removes the file (MT-215 E7).
    let ring = RingFile::new("loop");
    let path = ring.path().to_owned();
    on_private_desktop(LIMIT, move || {
        let writer = DiagRingWriterV2::create(&path, DEFAULT_CAPACITY, NONCE).expect("the ring");
        let reader = DiagRingReaderV2::open(&path, NONCE).expect("the reader");
        let test = Rc::new(Test {
            writer: RefCell::new(Some(writer)),
            reader,
            heartbeat: RefCell::default(),
            lane: RefCell::default(),
            steps: RefCell::new(steps().into()),
            started: Cell::new(Instant::now()),
            sampler: RefCell::new(Sampler::start(path.clone())),
            other_wakes: Cell::new(0),
            next_value: Cell::new(0),
            finished: Cell::new(false),
            beats: Tally::default(),
        });
        let (events, wakes) = (Rc::clone(&test), Rc::clone(&test));
        let report = run_script(
            LoopConfig::default(),
            move |control, event| {
                let shutting_down = matches!(event, Event::ShutdownBegan);
                events.event(control, event);
                if shutting_down {
                    control.finish(0).expect("the loop ends");
                }
            },
            move |control, wake| wakes.wake(control, wake),
        )
        .expect("the loop ran");

        assert!(test.steps.borrow().is_empty(), "every step ran");
        assert_eq!(report.quiet_broken, 0);
        assert_eq!(report.windows_created, 0);
        let heartbeat = test.heartbeat.borrow_mut().take().expect("the heartbeat");
        assert!(!heartbeat.is_running(), "stopped at the end of the loop");
        let counters = heartbeat.counters();
        assert!(counters.beats >= 15, "{counters:?}");
        assert_eq!(counters.ignored_wakes, 0, "{counters:?}");
        assert_eq!(counters.replaced_wakes, 0, "{counters:?}");
        assert!(counters.overdue_beats >= 2, "the two long handlers were seen: {counters:?}");
        assert!(!counters.loop_closed);
        assert!(test.beats.runs() >= 15, "beats counted: {}", test.beats.runs());
        assert_eq!(
            test.beats.allocations(),
            0,
            "allocations on the beat path over {} beats",
            test.beats.runs()
        );
        let last = heartbeat.last_beat();
        let writer = heartbeat.into_sink();
        assert_eq!(test.reader.read_heartbeat(), last);
        drop(writer);
        drop(test);
    });
    drop(ring);
}
