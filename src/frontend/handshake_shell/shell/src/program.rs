//! The shell program: the five crates composed into one process (MT-214 S5, AC214-10, AC214-11, AC214-12).
//!
//! Composition only. The window crate's loop, the surface crate's presenter on the central GPU owner, the input crate's
//! tracker, the accessibility crate's hub and this crate's heartbeat are wired to one another here; the program draws
//! nothing of its own beyond the presenter's cleared frame, holds no widget, pane, layout or domain state, and opens no
//! pane. What it shows is the shell frame: one quiet window and, in its accessibility tree, one named group.
//!
//! # Startup (AC214-10)
//!
//! [`run_program`] is the one place the device is opened, from a [`DeviceAuthority`]. The product reaches it only
//! through [`crate::run_product`], which refuses without `device.gpu.shell` under the kernel issuer, so a refused start
//! has opened no device, window or ring. A test or example binary reaches it with a [`crate::TestBinary`] and its
//! receipt says `product=false device_authority=none`. A device that is missing or rejected ends the program with
//! [`ExitKind::DeviceUnavailable`] and the owner's typed reason.
//!
//! # What the loop does
//!
//! - `LoopStarted`: the heartbeat opens on the ring (if the caller gave one) and starts, a wake lane takes the
//!   accessibility notices, and the root window is asked for, hidden.
//! - `Created`: the window's accessibility adapter is attached before the window is first shown, the shell frame's tree
//!   is given to it, and the window is shown without activation. The presenter has already given the window its
//!   surface and swapchain; frames are drawn only on damage (the surface crate's table).
//! - Every event: the input tracker and the accessibility hub follow it, the heartbeat takes a turn. Input has no
//!   consumer yet (no pane); the tracker's requests to the window (input method on or off, candidate area) are carried
//!   out.
//! - A tool's request through the accessibility tree wakes the loop; the requests are taken and counted. The shell frame
//!   declares no action, so none is carried out.
//! - Close of the root window, or the end the caller asked for: [`Shell::shutdown`].
//!
//! # Shutdown (AC214-11)
//!
//! `ShutdownBegan`: the accessibility adapters let go of their windows (the window crate refuses to destroy them
//! before), then the presenter releases surfaces, settles claims and closes the owner. The heartbeat keeps beating
//! through that drain. At `LoopEnding`, with the loop still running, the heartbeat has stopped and the program checks
//! what the owner reported: every claim settled, nothing retained or in flight, the device destroyed, every window
//! destroyed. Only then does it write the ring's `Shutdown` marker and ask the [`ShutdownWatcher`] to acknowledge
//! (MT-215 HB-3). A shutdown that is not settled is never acknowledged and ends with [`ExitKind::ShutdownUnsettled`].
//! A crash leaves the operating system's exception handler and the panic hook as they are: this crate installs neither.
//!
//! # Not here yet
//!
//! More than one window, device loss and rebind, and the readable unavailable state are MT-214 S6. The product's
//! authority and quota come from the kernel issuer and Core's ledger, which do not exist ([`crate::MISSING`]).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagPhase, DiagRingWriter, DiagSeverity};
use handshake_shell_accessibility::{AccessReport, Accessibility, SemanticFrame};
use handshake_shell_input::{InputReport, InputTracker};
use handshake_shell_surface::{PresentConfig, PresentHost, PresentReport, Shell, ShellApp};
use handshake_shell_window::{
    CommandSource, Control, DispatchedWake, Event, LogicalSize, LoopConfig, LoopError, Placement,
    RunReport, WakeLane, WindowClass, WindowCommand, WindowEvent, WindowKey, WindowSpec,
};
use handshake_vulkan_renderer::{ModuleId, NullDiagnostics, PolicyRevision, QuotaPolicy};

use crate::authority::DeviceAuthority;
use crate::heartbeat::ShellHeartbeat;
use crate::outcome::{conclude, may_acknowledge, shutdown_settled, ExitKind};
use crate::shutdown::{ShutdownWatcher, WatcherAck};

/// The module number the shell's claims carry. Attribution only; no canonical number is verified here.
const MODULE: ModuleId = ModuleId(214);
/// The name of the one group of the shell frame, in the window's accessibility tree.
const SHELL_FRAME_NAME: &str = "Handshake shell";
/// A wake of the notice lane: an assistive tool queued a request.
const NOTICE: u64 = 1;
/// A wake of the tick lane: look at the end the caller asked for.
const TICK: u64 = 2;
/// How often the end the caller asked for is looked at. Armed only when one was asked for.
const TICK_EVERY: Duration = Duration::from_millis(100);

/// When the program ends by itself. Neither is set for a run that lasts until the window is closed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EndAfter {
    /// This long after the loop started.
    pub time: Option<Duration>,
    /// Once this many frames were presented.
    pub frames_presented: Option<u64>,
}

/// How to run the program.
pub struct ProgramConfig {
    /// The window's title and its accessibility name.
    pub title: String,
    /// The window's size.
    pub size: LogicalSize,
    /// Quiet or Operator window. Quiet never takes the foreground, the keyboard or a place above other windows.
    pub class: WindowClass,
    /// Where the window goes; the primary monitor's centre when `None`.
    pub placement: Option<Placement>,
    /// The diagnostics ring the heartbeat writes, made by the caller. The heartbeat is silent without one.
    pub ring: Option<DiagRingWriter>,
    /// When the program ends by itself.
    pub end_after: EndAfter,
}

impl ProgramConfig {
    /// A quiet 960 by 600 window, centred on the primary monitor, until it is closed.
    pub fn new(ring: Option<DiagRingWriter>) -> Self {
        Self {
            title: "Handshake shell".to_owned(),
            size: LogicalSize::new(960.0, 600.0),
            class: WindowClass::Quiet,
            placement: None,
            ring,
            end_after: EndAfter::default(),
        }
    }
}

/// What the program counted, from the five crates. Mechanical numbers only: no title, name or content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramReport {
    /// The window crate's run.
    pub run: RunReport,
    /// The presenter's counters.
    pub present: PresentReport,
    /// The input tracker's counters.
    pub input: InputReport,
    /// The accessibility hub's counters.
    pub access: AccessReport,
    /// Typed inputs the tracker made. No pane takes them yet.
    pub inputs_typed: u64,
    /// Requests of assistive tools taken from the hub.
    pub access_requests: u64,
    /// Windows whose tree could not be attached to the platform, and the first reason.
    pub attach_refused: u64,
    /// The first attach or frame refusal's code, when there was one.
    pub access_code: Option<&'static str>,
    /// Windows the heartbeat could not watch.
    pub watch_refused: u64,
    /// Beats published.
    pub beats: u64,
    /// Beats later than the port's gap bound.
    pub overdue_beats: u64,
    /// Present stalls recorded.
    pub present_stalls: u64,
    /// Whether the shutdown was settled: every claim, nothing in flight, the device destroyed, every window gone.
    pub settled: bool,
    /// Whether the ring's shutdown marker was written.
    pub ring_marked: bool,
    /// What the watcher answered; `None` when the shutdown was not settled and nothing was asked.
    pub watcher: Option<WatcherAck>,
}

/// How the program ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramOutcome {
    /// The kind, with its exit code.
    pub kind: ExitKind,
    /// The code of the refusal or error behind it, when there is one.
    pub detail: Option<&'static str>,
    /// What was counted; `None` when the loop did not run.
    pub report: Option<ProgramReport>,
}

impl ProgramOutcome {
    fn without_loop(kind: ExitKind, detail: Option<&'static str>) -> Self {
        Self {
            kind,
            detail,
            report: None,
        }
    }

    /// The last line a binary prints. No content: names, codes and numbers.
    pub fn line(&self) -> String {
        let mut line = format!(
            "HSK_SHELL_END kind={} code={} detail={}",
            self.kind.name(),
            self.kind.code(),
            self.detail.unwrap_or("none")
        );
        if let Some(report) = &self.report {
            line.push_str(&format!(
                " windows_created={} windows_destroyed={} quiet_broken={} frames_presented={} frames_failed={} beats={} \
                 access_updates={} attach_refused={} settled={} ring_marked={} watcher={}",
                report.run.windows_created,
                report.run.windows_destroyed,
                report.run.quiet_broken,
                report.present.frames_presented,
                report.present.frames_failed,
                report.beats,
                report.access.updates_sent,
                report.attach_refused,
                report.settled,
                report.ring_marked,
                report.watcher.map_or("not_asked", WatcherAck::name)
            ));
        }
        line
    }
}

/// The quota the test binaries' claims run under: room for a few windows' claims and their frames. The product's comes
/// from the kernel issuer and Core's ledger, which do not exist.
fn policy_of(authority: &DeviceAuthority) -> QuotaPolicy {
    match authority {
        DeviceAuthority::Issued(issued) => issued.unreachable(),
        DeviceAuthority::Test(_) => QuotaPolicy::Configured {
            revision: PolicyRevision(1),
            limit_bytes: 512 << 20,
            max_claims: 4,
            max_in_flight_per_claim: 16,
        },
    }
}

/// Open the device under `authority` and run the shell until it closes. The only place the device is opened.
///
/// The ring writer in `config` is the heartbeat's. `watcher` is asked to acknowledge only a settled shutdown.
pub fn run_program(
    config: ProgramConfig,
    authority: DeviceAuthority,
    watcher: Box<dyn ShutdownWatcher>,
) -> ProgramOutcome {
    let present = PresentConfig::new(policy_of(&authority), MODULE);
    let app = Rc::new(ProgramApp::new(config, watcher));
    let host = match PresentHost::open(present, Arc::new(NullDiagnostics), app.clone()) {
        Ok(host) => host,
        Err(reason) => {
            return ProgramOutcome::without_loop(ExitKind::DeviceUnavailable, Some(reason.key()));
        }
    };
    let loop_config = LoopConfig {
        launched_by_operator: false,
        ..LoopConfig::default()
    };
    let run = handshake_shell_window::run(loop_config, host.clone());
    app.outcome(run, host.report())
}

/// What the program kept at the end of the loop, for its report.
#[derive(Clone, Copy, Default)]
struct Ended {
    settled: bool,
    ring_marked: bool,
    watcher: Option<WatcherAck>,
    beats: u64,
    overdue_beats: u64,
    present_stalls: u64,
}

#[derive(Clone, Copy, Default)]
struct Counts {
    inputs_typed: u64,
    access_requests: u64,
    attach_refused: u64,
    watch_refused: u64,
}

struct ProgramApp {
    config: Config,
    origin: Instant,
    watcher: Box<dyn ShutdownWatcher>,
    ring: RefCell<Option<DiagRingWriter>>,
    heartbeat: RefCell<Option<ShellHeartbeat<Option<DiagRingWriter>>>>,
    access: RefCell<Accessibility>,
    input: RefCell<InputTracker>,
    notice: RefCell<Option<WakeLane>>,
    tick: RefCell<Option<WakeLane>>,
    started: Cell<Option<Instant>>,
    root: Cell<Option<WindowKey>>,
    ending: Cell<bool>,
    failure: Cell<Option<ExitKind>>,
    detail: Cell<Option<&'static str>>,
    access_code: Cell<Option<&'static str>>,
    commands: Cell<u64>,
    counts: Cell<Counts>,
    ended: Cell<Ended>,
}

struct Config {
    title: String,
    size: LogicalSize,
    class: WindowClass,
    placement: Option<Placement>,
    end_after: EndAfter,
}

impl ProgramApp {
    fn new(config: ProgramConfig, watcher: Box<dyn ShutdownWatcher>) -> Self {
        let ProgramConfig {
            title,
            size,
            class,
            placement,
            ring,
            end_after,
        } = config;
        Self {
            config: Config {
                title,
                size,
                class,
                placement,
                end_after,
            },
            origin: Instant::now(),
            watcher,
            ring: RefCell::new(ring),
            heartbeat: RefCell::new(None),
            access: RefCell::new(Accessibility::new()),
            input: RefCell::new(InputTracker::new()),
            notice: RefCell::new(None),
            tick: RefCell::new(None),
            started: Cell::new(None),
            root: Cell::new(None),
            ending: Cell::new(false),
            failure: Cell::new(None),
            detail: Cell::new(None),
            access_code: Cell::new(None),
            commands: Cell::new(0),
            counts: Cell::new(Counts::default()),
            ended: Cell::new(Ended::default()),
        }
    }

    fn count(&self, change: impl FnOnce(&mut Counts)) {
        let mut counts = self.counts.get();
        change(&mut counts);
        self.counts.set(counts);
    }

    /// The next command's source: the shell itself, numbered.
    fn source(&self) -> CommandSource {
        let id = self.commands.get() + 1;
        self.commands.set(id);
        CommandSource::host(id)
    }

    /// The first failure ends the program: shutdown begins, and the kind is what the program ends with.
    fn fail(&self, shell: &Shell<'_>, kind: ExitKind, detail: &'static str) {
        if self.failure.get().is_none() {
            self.failure.set(Some(kind));
            self.detail.set(Some(detail));
        }
        self.end(shell, i32::from(kind.code()));
    }

    fn end(&self, shell: &Shell<'_>, exit_code: i32) {
        if !self.ending.replace(true) {
            let _ = shell.shutdown(exit_code);
        }
    }

    fn follow_input(&self, control: &Control, event: &Event) {
        let (typed, requests) = {
            let mut input = self.input.borrow_mut();
            let typed = input.feed(control.now(), event);
            (typed.len(), input.take_requests())
        };
        let typed = u64::try_from(typed).unwrap_or(u64::MAX);
        self.count(|counts| counts.inputs_typed = counts.inputs_typed.saturating_add(typed));
        for (window, request) in requests {
            let _ = control.command(&self.source(), window, request.command());
        }
    }

    fn started(&self, shell: &Shell<'_>) {
        let control = shell.control();
        self.started.set(Some(Instant::now()));
        // The heartbeat first: a watcher must see the loop from its first turn.
        let ring = self.ring.borrow_mut().take();
        match ShellHeartbeat::open(control, self.origin, ring) {
            Ok(heartbeat) => match heartbeat.start() {
                Ok(()) => *self.heartbeat.borrow_mut() = Some(heartbeat),
                Err(_) => {
                    *self.ring.borrow_mut() = heartbeat.into_sink();
                    self.fail(shell, ExitKind::HeartbeatRefused, "heartbeat.start_refused");
                    return;
                }
            },
            Err((refused, sink)) => {
                *self.ring.borrow_mut() = sink;
                self.fail(shell, ExitKind::HeartbeatRefused, refused.code());
                return;
            }
        }
        // Accessibility notices: a tool queues a request from its own thread, and the loop takes it.
        let (notice, tick) = match (control.open_wake_lane(), control.open_wake_lane()) {
            (Ok(notice), Ok(tick)) => (notice, tick),
            (Err(refused), _) | (_, Err(refused)) => {
                self.fail(shell, ExitKind::LoopFailed, refused.code());
                return;
            }
        };
        let waker = notice.waker();
        self.access.borrow_mut().set_notifier(move || {
            let _ = waker.wake(NOTICE);
        });
        *self.notice.borrow_mut() = Some(notice);
        *self.tick.borrow_mut() = Some(tick);
        self.arm_tick();
        self.open_root(shell);
    }

    fn open_root(&self, shell: &Shell<'_>) {
        let control = shell.control();
        let placement = self.config.placement.unwrap_or_else(|| {
            control
                .monitors()
                .into_iter()
                .find(|monitor| monitor.primary)
                .map_or(Placement::System, |monitor| Placement::Centered(monitor.key))
        });
        let spec = WindowSpec {
            title: self.config.title.clone(),
            inner: self.config.size,
            min_inner: None,
            placement,
            class: self.config.class,
        };
        match shell.open_window(&self.source(), spec) {
            Ok(window) => self.root.set(Some(window)),
            Err(refusal) => self.fail(shell, ExitKind::NoWindow, refusal.code()),
        }
    }

    fn arm_tick(&self) {
        let asked = self.config.end_after;
        if asked.time.is_some() || asked.frames_presented.is_some() {
            if let Some(lane) = self.tick.borrow().as_ref() {
                let _ = lane.request(TICK, TICK_EVERY);
            }
        }
    }

    /// A window exists, hidden: its tree reaches the platform before it is first shown, then it is shown without
    /// activation.
    fn created(&self, shell: &Shell<'_>, window: WindowKey) {
        let control = shell.control();
        let attached = self.access.borrow_mut().attach(control, window);
        if let Err(refusal) = attached {
            self.count(|counts| counts.attach_refused += 1);
            self.access_code.set(self.access_code.get().or(Some(refusal.code())));
        }
        let frame = SemanticFrame::named_group(&self.config.title, SHELL_FRAME_NAME);
        let given = self.access.borrow_mut().update(window, frame);
        if let Err(refusal) = given {
            self.access_code.set(self.access_code.get().or(Some(refusal.code())));
        }
        let shown = control.command(
            &self.source(),
            window,
            WindowCommand::Show { activate: false },
        );
        if let Err(refusal) = shown {
            self.fail(shell, ExitKind::NoWindow, refusal.code());
        }
    }

    fn close_requested(&self, shell: &Shell<'_>, window: WindowKey) {
        if self.root.get() == Some(window) {
            self.end(shell, 0);
        } else {
            // Another window of the program: its tree lets go first, then the presenter closes it.
            self.access.borrow_mut().close(window);
            let _ = shell.close_window(window);
        }
    }

    /// The heartbeat's turn for every event; a window that presents is watched from its creation.
    fn beat(&self, shell: &Shell<'_>, event: &Event) {
        let heartbeat = self.heartbeat.borrow();
        if let Some(heartbeat) = heartbeat.as_ref() {
            if heartbeat.on_shell_event(shell, event).is_err() {
                self.count(|counts| counts.watch_refused += 1);
            }
        }
    }

    fn take_access_requests(&self, control: &Control) {
        let taken = self.access.borrow_mut().take_requests(control.now());
        let taken = u64::try_from(taken.len()).unwrap_or(u64::MAX);
        self.count(|counts| counts.access_requests = counts.access_requests.saturating_add(taken));
    }

    fn tick(&self, shell: &Shell<'_>) {
        let asked = self.config.end_after;
        let timed_out = match (asked.time, self.started.get()) {
            (Some(time), Some(started)) => started.elapsed() >= time,
            _ => false,
        };
        let framed = asked
            .frames_presented
            .is_some_and(|frames| shell.report().frames_presented >= frames);
        if timed_out || framed {
            self.end(shell, 0);
        } else {
            self.arm_tick();
        }
    }

    /// `LoopEnding`, with the loop still running: the heartbeat has stopped. Check what the owner reported, and only a
    /// settled shutdown is marked in the ring and acknowledged by the watcher.
    fn ended(&self, shell: &Shell<'_>) {
        let mut ended = Ended::default();
        if let Some(heartbeat) = self.heartbeat.borrow_mut().take() {
            let counters = heartbeat.counters();
            ended.beats = counters.beats;
            ended.overdue_beats = counters.overdue_beats;
            ended.present_stalls = counters.present_stalls;
            *self.ring.borrow_mut() = heartbeat.into_sink();
        }
        let present = shell.report();
        let run = shell.control().report();
        ended.settled = shutdown_settled(
            present.shutdown.map(|report| report.is_settled()),
            present.claims_abandoned,
            run.windows_created,
            run.windows_destroyed,
        );
        if may_acknowledge(ended.settled) {
            if let Some(ring) = self.ring.borrow().as_ref() {
                let nanos = u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX);
                ring.write(DiagEvent::generic(
                    DiagEventCode::Shutdown,
                    DiagPhase::End,
                    DiagSeverity::Info,
                    0,
                    0,
                    ended.beats,
                    0,
                    0,
                    nanos,
                ));
                ended.ring_marked = true;
            }
            ended.watcher = Some(self.watcher.acknowledge_shutdown());
        }
        self.ended.set(ended);
    }

    fn outcome(&self, run: Result<RunReport, LoopError>, present: PresentReport) -> ProgramOutcome {
        let run = match run {
            Ok(run) => run,
            Err(error) => return ProgramOutcome::without_loop(ExitKind::LoopFailed, Some(error.code())),
        };
        let ended = self.ended.get();
        let counts = self.counts.get();
        let access = self.access.borrow().report();
        let input = self.input.borrow().report();
        let report = ProgramReport {
            run,
            present,
            input,
            access,
            inputs_typed: counts.inputs_typed,
            access_requests: counts.access_requests,
            attach_refused: counts.attach_refused,
            access_code: self.access_code.get(),
            watch_refused: counts.watch_refused,
            beats: ended.beats,
            overdue_beats: ended.overdue_beats,
            present_stalls: ended.present_stalls,
            settled: ended.settled,
            ring_marked: ended.ring_marked,
            watcher: ended.watcher,
        };
        let kind = conclude(ended.settled, self.failure.get(), ended.watcher);
        let detail = match kind {
            ExitKind::Clean | ExitKind::WatcherNotAcknowledged => None,
            _ => self.detail.get(),
        };
        ProgramOutcome {
            kind,
            detail,
            report: Some(report),
        }
    }
}

impl ShellApp for ProgramApp {
    fn on_event(&self, shell: &Shell<'_>, event: &Event) {
        let control = shell.control();
        self.follow_input(control, event);
        self.access.borrow_mut().feed(event);
        if matches!(event, Event::LoopStarted) {
            self.started(shell);
        }
        self.beat(shell, event);
        match event {
            Event::Window {
                window,
                event: WindowEvent::Created { .. },
            } => self.created(shell, *window),
            Event::Window {
                window,
                event: WindowEvent::CreateFailed { failure, .. },
            } if self.root.get() == Some(*window) => {
                self.fail(shell, ExitKind::NoWindow, failure.code());
            }
            Event::Window {
                window,
                event: WindowEvent::CloseRequested,
            } => self.close_requested(shell, *window),
            Event::ShutdownBegan => self.access.borrow_mut().close_all(),
            Event::LoopEnding => self.ended(shell),
            _ => {}
        }
    }

    fn on_wake(&self, shell: &Shell<'_>, wake: &DispatchedWake<'_>) {
        if let Some(heartbeat) = self.heartbeat.borrow().as_ref() {
            if heartbeat.on_wake(wake).is_some() {
                return;
            }
        }
        let on_lane = |lane: &RefCell<Option<WakeLane>>| {
            lane.borrow()
                .as_ref()
                .is_some_and(|lane| lane.id() == wake.lane())
        };
        if on_lane(&self.notice) && wake.value() == NOTICE {
            self.take_access_requests(shell.control());
        } else if on_lane(&self.tick) && wake.value() == TICK {
            self.tick(shell);
        }
    }
}
