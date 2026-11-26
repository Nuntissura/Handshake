//! The heartbeat on the presenter's real loop with a real window on this machine's device, quiet, at the bottom of a
//! private desktop that receives no input, in a process of its own (MT-215 HB-4). The window that presents is watched
//! from the moment it exists and beats go on while it presents; once the GPU owner revokes the window's client, its
//! frames fail, the next one is wanted and never presented, and the ring holds one typed stall event while the beats
//! go on, fresh, as the loop keeps dispatching (AC215-6); once the window cannot show a frame any more (minimised), the
//! stall ends with one recovery event. The beat path, watched windows included, allocates nothing (E6).
#![cfg(windows)]
#![allow(unsafe_code)]

#[path = "../../surface/tests/support/mod.rs"]
mod support;

#[path = "../../../handshake_host_ports/ui_heartbeat/tests/support/limits.rs"]
mod limits;

mod allocations;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use handshake_diag_ring::ring::{DiagRingReaderV2, DiagRingWriterV2};
use handshake_diag_ring::{DiagEventCode, DiagPhase, DEFAULT_CAPACITY};
use handshake_shell::ShellHeartbeat;
use handshake_shell_surface::{PresentConfig, PresentHost, Shell, ShellApp};
use handshake_shell_window::{
    CommandSource, DispatchedWake, Event, LogicalSize, LoopConfig, PhysicalPosition, Placement,
    ShellHost, WakeLane, WindowClass, WindowCommand, WindowEvent, WindowKey, WindowSpec,
};
use handshake_vulkan_renderer::{ClientId, ModuleId};
use ui_heartbeat::{WakeOutcome, PRESENT_STALL, WAKE_DELAY};

use allocations::Tally;
use limits::{RingFile, TimeLimit};
use support::desktop::{must_stay_inactive, on_private_desktop, run_script, stop_watching};
use support::gpu_env::{in_own_process, ran_in_its_own_process};

const LIMIT: Duration = Duration::from_secs(100);
/// Ends the test before the private desktop's watchdog and the parent process's deadline do, so its ring file is
/// removed first (MT-215 E7).
const TIME_LIMIT: Duration = Duration::from_secs(90);
const STEP_LIMIT: Duration = Duration::from_secs(15);
const TICK: Duration = Duration::from_millis(10);
const MODULE: ModuleId = ModuleId(215);
const NONCE: [u8; 16] = [0x24; 16];

type Run = Box<dyn FnMut(&Shell<'_>, &App) -> bool>;

struct Step {
    name: &'static str,
    run: Run,
}

fn step(name: &'static str, run: impl FnMut(&Shell<'_>, &App) -> bool + 'static) -> Step {
    Step {
        name,
        run: Box::new(run),
    }
}

struct App {
    writer: RefCell<Option<DiagRingWriterV2>>,
    reader: DiagRingReaderV2,
    client_base: u64,
    heartbeat: RefCell<Option<ShellHeartbeat<DiagRingWriterV2>>>,
    lane: RefCell<Option<WakeLane>>,
    steps: RefCell<VecDeque<Step>>,
    started: Cell<Instant>,
    window: Cell<Option<WindowKey>>,
    hwnd: Cell<isize>,
    shut: Cell<bool>,
    /// Allocations on the loop's thread inside the heartbeat's beats (MT-215 E6).
    beats: Tally,
}

impl App {
    fn counter(&self) -> u64 {
        self.reader.read_heartbeat().map_or(0, |beat| beat.counter)
    }

    fn with_heartbeat<T>(&self, use_it: impl FnOnce(&ShellHeartbeat<DiagRingWriterV2>) -> T) -> T {
        use_it(self.heartbeat.borrow().as_ref().expect("the heartbeat runs"))
    }

    fn window(&self) -> WindowKey {
        self.window.get().expect("the window")
    }

    /// Stall events of the ring in `phase`.
    fn stall_events(&self, phase: DiagPhase) -> usize {
        self.reader
            .read_last_n(64)
            .iter()
            .filter(|record| {
                record.event.event_code == DiagEventCode::SlowFrame.as_u16()
                    && record.event.phase_marker == phase.as_u8()
            })
            .count()
    }

    fn next_tick(&self) {
        if let Some(lane) = self.lane.borrow().as_ref() {
            lane.request(0, TICK).expect("the lane takes a wake");
        }
    }
}

impl ShellApp for App {
    fn on_event(&self, shell: &Shell<'_>, event: &Event) {
        match event {
            Event::LoopStarted => {
                let writer = self.writer.borrow_mut().take().expect("the ring writer");
                let heartbeat = ShellHeartbeat::open(shell.control(), Instant::now(), writer)
                    .map_err(|(refused, _)| refused)
                    .expect("a lane for the heartbeat");
                heartbeat.start().expect("starts");
                *self.heartbeat.borrow_mut() = Some(heartbeat);
                *self.lane.borrow_mut() = Some(shell.control().open_wake_lane().expect("a lane"));
                let spec = WindowSpec {
                    title: "heartbeat present".to_owned(),
                    inner: LogicalSize::new(200.0, 120.0),
                    min_inner: None,
                    placement: Placement::At(PhysicalPosition::new(40, 40)),
                    class: WindowClass::Quiet,
                };
                shell
                    .open_window(&CommandSource::host(1), spec)
                    .expect("a window");
            }
            Event::Window {
                window,
                event: WindowEvent::Created { .. },
            } => {
                let hwnd = shell
                    .control()
                    .native_window(*window)
                    .expect("a token")
                    .win32()
                    .hwnd
                    .get();
                must_stay_inactive(hwnd);
                self.hwnd.set(hwnd);
                self.window.set(Some(*window));
            }
            Event::Window {
                event: WindowEvent::Destroyed,
                ..
            } => stop_watching(self.hwnd.get()),
            _ => {}
        }
        if let Some(heartbeat) = self.heartbeat.borrow().as_ref() {
            heartbeat
                .on_shell_event(shell, event)
                .expect("the window is watched");
        }
        if let Event::Window {
            window,
            event: WindowEvent::Created { .. },
        } = event
        {
            shell
                .control()
                .command(
                    &CommandSource::host(2),
                    *window,
                    WindowCommand::Show { activate: false },
                )
                .expect("shown");
            self.started.set(Instant::now());
            self.next_tick();
        }
        if matches!(event, Event::LoopEnding) {
            self.lane.borrow_mut().take();
        }
    }

    fn on_wake(&self, shell: &Shell<'_>, wake: &DispatchedWake<'_>) {
        let routed = self.with_heartbeat(|heartbeat| {
            self.beats
                .count(|| heartbeat.on_wake(wake), Option::is_some)
        });
        if let Some(outcome) = routed {
            assert!(
                matches!(outcome, WakeOutcome::Beat(_)),
                "every wake of the heartbeat's lane beats (AC215-6): {outcome:?}"
            );
            return;
        }
        if self.shut.get() {
            return;
        }
        let mut steps = self.steps.borrow_mut();
        let Some(step) = steps.front_mut() else {
            drop(steps);
            self.shut.set(true);
            shell.shutdown(0).expect("shutdown begins");
            return;
        };
        if (step.run)(shell, self) {
            steps.pop_front();
            self.started.set(Instant::now());
        } else {
            assert!(
                self.started.get().elapsed() < STEP_LIMIT,
                "step \"{}\" did not finish within {STEP_LIMIT:?}; {:?} {:?}",
                step.name,
                self.with_heartbeat(ShellHeartbeat::counters),
                shell.report()
            );
        }
        drop(steps);
        self.next_tick();
    }
}

fn steps() -> Vec<Step> {
    let mark = Rc::new(Cell::new((0_u64, Instant::now())));
    let (note, idle, stalled, quiet, back) = (
        Rc::clone(&mark),
        Rc::clone(&mark),
        Rc::clone(&mark),
        Rc::clone(&mark),
        Rc::clone(&mark),
    );
    vec![
        step("the window presents and is watched", |shell, app| {
            let presented = shell
                .presentation(app.window())
                .and_then(|presentation| presentation.progress)
                .map_or(0, |progress| progress.presented);
            presented >= 1 && app.with_heartbeat(ShellHeartbeat::watched) == 1 && app.counter() >= 3
        }),
        step("note the beat", move |_, app| {
            note.set((app.counter(), Instant::now()));
            true
        }),
        step("a presenting window keeps the beat", move |_, app| {
            let (from, since) = idle.get();
            assert!(!app.with_heartbeat(ShellHeartbeat::present_stalled));
            since.elapsed() >= PRESENT_STALL + Duration::from_millis(500) && app.counter() >= from + 10
        }),
        step("the owner revokes the window's client and a frame is wanted", |shell, app| {
            let client = ClientId(app.client_base + app.window().number());
            let revoked = shell.owner().revoke_client(MODULE, client);
            assert!(revoked >= 1, "the window's claim was revoked");
            shell.damage(app.window()).expect("damaged");
            true
        }),
        step("the stall is recorded", move |_, app| {
            if !app.with_heartbeat(ShellHeartbeat::present_stalled) {
                return false;
            }
            stalled.set((app.counter(), Instant::now()));
            true
        }),
        step("beats go on while the loop dispatches and nothing presents", move |_, app| {
            let (at, since) = quiet.get();
            let elapsed = since.elapsed();
            if elapsed < Duration::from_secs(1) {
                return false;
            }
            assert!(app.with_heartbeat(ShellHeartbeat::present_stalled), "the stall goes on");
            // A beat every WAKE_DELAY on an idle loop; a third of them leaves room for a slow, emulated machine, while a
            // heartbeat held back for the stall would show none.
            let least = elapsed.as_millis() as u64 / WAKE_DELAY.as_millis() as u64 / 3;
            let beats = app.counter() - at;
            assert!(
                beats >= least,
                "{beats} beats in {elapsed:?} of a stalled present: the heartbeat must stay fresh (AC215-6)"
            );
            let counters = app.with_heartbeat(ShellHeartbeat::counters);
            assert_eq!(counters.present_stalls, 1);
            assert_eq!(app.stall_events(DiagPhase::Degraded), 1, "the stall is told by its event");
            assert_eq!(app.stall_events(DiagPhase::Recovered), 0);
            true
        }),
        step("the window is minimised and wants no frame it can show", |shell, app| {
            shell
                .control()
                .command(&CommandSource::host(3), app.window(), WindowCommand::Minimize)
                .expect("minimised");
            true
        }),
        step("the stall ends, once", move |_, app| {
            let (at, _) = back.get();
            if app.with_heartbeat(ShellHeartbeat::present_stalled) || app.counter() <= at {
                return false;
            }
            assert_eq!(app.stall_events(DiagPhase::Degraded), 1);
            assert_eq!(app.stall_events(DiagPhase::Recovered), 1);
            true
        }),
    ]
}

#[test]
fn a_stalled_present_keeps_the_beat_and_is_recorded_once_each_way() {
    in_own_process!();
    // In the test's own process only, so the parent never ends it.
    let _limit = TimeLimit::new(TIME_LIMIT);
    // Out here, so a failure on the loop's thread still unwinds to it and removes the file (MT-215 E7).
    let ring = RingFile::new("present");
    let path = ring.path().to_owned();
    on_private_desktop(LIMIT, move || {
        let writer = DiagRingWriterV2::create(&path, DEFAULT_CAPACITY, NONCE).expect("the ring");
        let reader = DiagRingReaderV2::open(&path, NONCE).expect("the reader");
        let config = PresentConfig::new(support::policy(), MODULE);
        let app = Rc::new(App {
            writer: RefCell::new(Some(writer)),
            reader,
            client_base: config.client_base,
            heartbeat: RefCell::default(),
            lane: RefCell::default(),
            steps: RefCell::new(steps().into()),
            started: Cell::new(Instant::now()),
            window: Cell::new(None),
            hwnd: Cell::new(0),
            shut: Cell::new(false),
            beats: Tally::default(),
        });
        let log = Arc::new(support::EventLog::default());
        let host = match PresentHost::open(config, log, app.clone()) {
            Ok(host) => host,
            Err(reason) => {
                support::skip_without_device(reason);
                return;
            }
        };
        let (events, wakes) = (Rc::clone(&host), Rc::clone(&host));
        let run = run_script(
            LoopConfig::default(),
            move |control, event| events.on_event(control, event),
            move |control, wake| wakes.on_wake(control, wake),
        )
        .expect("the loop ran");
        assert!(app.steps.borrow().is_empty(), "every step ran");
        assert_eq!(run.quiet_broken, 0);
        let heartbeat = app.heartbeat.borrow_mut().take().expect("the heartbeat");
        assert!(!heartbeat.is_running());
        assert_eq!(heartbeat.watched(), 0, "the destroyed window is watched no more");
        let counters = heartbeat.counters();
        assert_eq!(counters.present_stalls, 1, "{counters:?}");
        assert_eq!(counters.ignored_wakes, 0, "{counters:?}");
        assert!(app.beats.runs() >= 10, "beats counted: {}", app.beats.runs());
        assert_eq!(
            app.beats.allocations(),
            0,
            "allocations on the beat path with a watched window, over {} beats",
            app.beats.runs()
        );
        println!(
            "HSK_SHELL_HEARTBEAT beats={} stalls={} overdue={} counted_allocations={}",
            counters.beats,
            counters.present_stalls,
            counters.overdue_beats,
            app.beats.allocations()
        );
        drop(heartbeat.into_sink());
        drop(host);
        drop(app);
    });
    drop(ring);
}
