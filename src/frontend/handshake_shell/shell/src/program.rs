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
//! # Several windows (AC214-8)
//!
//! The root window and the detached windows ([`WindowKind`]: detached panes and the detached Settings window) share the
//! one presenter, and so one device and one owner ledger: each has its own claim, surface and accessibility tree.
//! [`ProgramConfig::plan`] opens and closes detached windows in order ([`Action`]); the UI layer's requests to detach or
//! close take the same road when it exists. Closing a detached window releases only its surface and claim: the others
//! keep presenting, and the program records that they did ([`ProgramReport::survivors_presented`]). Closing the root
//! window ends the program, with every other window closed in the drain. A window key is never used again in a run, and
//! the program checks it ([`ProgramReport::key_reuse`]).
//!
//! # Device loss and the unavailable state (AC214-9)
//!
//! The presenter recovers a lost device (the surface crate's module documentation): every window's surface is released,
//! the owner replaces the device, each claim moves onto it and each window draws again. This program only counts what it
//! is told ([`ProgramReport::present`]). When no device comes back it ends with [`ExitKind::DeviceUnavailable`] and an
//! [`UnavailableState`], as it does when no device opened at the start. The state is typed and readable; the drawn
//! presentation waits for the UI layer, and no other renderer is tried.
//!
//! # Not here yet
//!
//! The product's authority and quota come from the kernel issuer and Core's ledger, which do not exist
//! ([`crate::MISSING`]). The UI layer's requests to detach and close are not published; [`ProgramConfig::plan`] stands in.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagPhase, DiagRingWriter, DiagSeverity};
use handshake_shell_accessibility::{AccessReport, Accessibility, SemanticFrame};
use handshake_shell_input::{InputReport, InputTracker};
use handshake_shell_surface::{
    DeviceEvent, PresentConfig, PresentHost, PresentReport, Shell, ShellApp,
};
use handshake_shell_window::{
    CommandSource, Control, DispatchedWake, Event, LogicalSize, LoopConfig, LoopError, Placement,
    RunReport, WakeLane, WindowClass, WindowCommand, WindowEvent, WindowKey, WindowSpec,
};
use handshake_vulkan_renderer::{ModuleId, NullDiagnostics, PolicyRevision, QuotaPolicy};

use crate::authority::DeviceAuthority;
use crate::heartbeat::ShellHeartbeat;
use crate::outcome::{conclude, may_acknowledge, shutdown_settled, ExitKind};
use crate::shutdown::{ShutdownWatcher, WatcherAck};
use crate::unavailable::{unavailable_after, UnavailableState};
use crate::windows::{survivors_presented, Action, DetachedSpec, KindCounts, WindowBook, WindowKind};

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
    /// The root window's class. Quiet never takes the foreground, the keyboard or a place above other windows; the
    /// product's root is [`WindowClass::Operator`] ([`ProgramConfig::operator_root`], WP012-SHELL-ROOT-WINDOW-20261127).
    /// Detached panes and detached Settings stay quiet windows whatever this is.
    pub class: WindowClass,
    /// The Operator started this process to work in it: the root is activated once, with the launch authority, when it
    /// is first shown. Only for an Operator root of the product; a test binary's run ignores it.
    pub launched_by_operator: bool,
    /// Where the window goes; the primary monitor's centre when `None`.
    pub placement: Option<Placement>,
    /// The diagnostics ring the heartbeat writes, made by the caller. The heartbeat is silent without one.
    pub ring: Option<DiagRingWriter>,
    /// When the program ends by itself.
    pub end_after: EndAfter,
    /// What the program does by itself after it starts: detached windows opened and closed, in order (AC214-8).
    pub plan: Vec<Action>,
}

impl ProgramConfig {
    /// A quiet 960 by 600 window, centred on the primary monitor, until it is closed. What tests and examples use.
    pub fn new(ring: Option<DiagRingWriter>) -> Self {
        Self {
            title: "Handshake shell".to_owned(),
            size: LogicalSize::new(960.0, 600.0),
            class: WindowClass::Quiet,
            launched_by_operator: false,
            placement: None,
            ring,
            end_after: EndAfter::default(),
            plan: Vec::new(),
        }
    }
}

impl ProgramConfig {
    /// The product's root: an Operator window that takes the focus once, at the Operator's launch (the launch
    /// authority, used once), and whose close ends the process. WP012-SHELL-ROOT-WINDOW-20261127: "Operator window,
    /// closes app". Only the product binary calls this.
    pub fn operator_root(ring: Option<DiagRingWriter>) -> Self {
        Self {
            class: WindowClass::Operator,
            launched_by_operator: true,
            ..Self::new(ring)
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
    /// Windows the program opened, by kind.
    pub windows: KindCounts,
    /// Windows that were destroyed before the shutdown began.
    pub windows_closed_early: u64,
    /// Keys the window crate gave out twice in this run. Must be 0.
    pub key_reuse: u64,
    /// Whether every window that was open when the plan's last close ran presented a frame after it; `None` when no
    /// [`Action::AwaitSurvivors`] finished.
    pub survivors_presented: Option<bool>,
    /// Steps of the plan that named a window or a baseline that did not exist.
    pub plan_refused: u64,
    /// Whether every step of the plan ran.
    pub plan_done: bool,
    /// The most claims the owner's ledger held open at once, sampled as each window was created.
    pub claims_peak: u64,
    /// Claims the owner's one ledger admitted in the run.
    pub ledger_admitted: u64,
    /// Device generations the owner's ledger bound in the run: 1, plus one for each replacement after a loss.
    pub ledger_generations: u64,
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
    /// Set when the end is [`ExitKind::DeviceUnavailable`]: the typed, readable state (AC214-9).
    pub unavailable: Option<UnavailableState>,
}

impl ProgramOutcome {
    fn without_loop(kind: ExitKind, detail: Option<&'static str>) -> Self {
        Self {
            kind,
            detail,
            report: None,
            unavailable: None,
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
                 access_updates={} attach_refused={} settled={} ring_marked={} watcher={} windows_opened={} key_reuse={} \
                 device_losses={} device_rebinds={} windows_rebound={} windows_unrecovered={}",
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
                report.watcher.map_or("not_asked", WatcherAck::name),
                report.windows.total(),
                report.key_reuse,
                report.present.device_losses,
                report.present.device_rebinds,
                report.present.windows_rebound,
                report.present.windows_unrecovered
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
    mut config: ProgramConfig,
    authority: DeviceAuthority,
    watcher: Box<dyn ShutdownWatcher>,
) -> ProgramOutcome {
    // A test binary never has an Operator window or a launch activation, whatever its config says (HBR-QUIET).
    if matches!(authority, DeviceAuthority::Test(_)) {
        config.class = WindowClass::Quiet;
        config.launched_by_operator = false;
    }
    let launched_by_operator = config.launched_by_operator && config.class == WindowClass::Operator;
    let present = PresentConfig::new(policy_of(&authority), MODULE);
    let app = Rc::new(ProgramApp::new(config, watcher));
    let host = match PresentHost::open(present, Arc::new(NullDiagnostics), app.clone()) {
        Ok(host) => host,
        Err(reason) => {
            return ProgramOutcome {
                unavailable: Some(UnavailableState::at_start(reason)),
                ..ProgramOutcome::without_loop(ExitKind::DeviceUnavailable, Some(reason.key()))
            };
        }
    };
    let loop_config = LoopConfig {
        launched_by_operator,
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
    ledger_admitted: u64,
    ledger_generations: u64,
}

#[derive(Clone, Copy, Default)]
struct Counts {
    inputs_typed: u64,
    access_requests: u64,
    attach_refused: u64,
    watch_refused: u64,
    plan_refused: u64,
    claims_peak: u64,
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
    book: RefCell<WindowBook>,
    /// Each window's title, which is its accessibility name.
    titles: RefCell<BTreeMap<WindowKey, String>>,
    /// Windows the presenter has created: a close of one waits for it.
    created: RefCell<Vec<WindowKey>>,
    plan: RefCell<VecDeque<Action>>,
    /// (window number, frames the queue took) of the windows that stayed open at the last close of the plan.
    baseline: RefCell<Option<Vec<(u64, u64)>>>,
    survivors: Cell<Option<bool>>,
    unavailable: Cell<Option<UnavailableState>>,
    /// Windows the book had closed when the shutdown began: the rest were closed before it.
    closed_at_shutdown: Cell<u64>,
}

struct Config {
    title: String,
    size: LogicalSize,
    class: WindowClass,
    launched_by_operator: bool,
    placement: Option<Placement>,
    end_after: EndAfter,
}

impl ProgramApp {
    fn new(config: ProgramConfig, watcher: Box<dyn ShutdownWatcher>) -> Self {
        let ProgramConfig {
            title,
            size,
            class,
            launched_by_operator,
            placement,
            ring,
            end_after,
            plan,
        } = config;
        Self {
            config: Config {
                title,
                size,
                class,
                launched_by_operator,
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
            book: RefCell::new(WindowBook::default()),
            titles: RefCell::new(BTreeMap::new()),
            created: RefCell::new(Vec::new()),
            plan: RefCell::new(plan.into()),
            baseline: RefCell::new(None),
            survivors: Cell::new(None),
            unavailable: Cell::new(None),
            closed_at_shutdown: Cell::new(0),
        }
    }

    fn count(&self, change: impl FnOnce(&mut Counts)) {
        let mut counts = self.counts.get();
        change(&mut counts);
        self.counts.set(counts);
    }

    /// The next command's number.
    fn next_id(&self) -> u64 {
        let id = self.commands.get() + 1;
        self.commands.set(id);
        id
    }

    /// The next command's source: the shell itself, numbered.
    fn source(&self) -> CommandSource {
        CommandSource::host(self.next_id())
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
        if let Some(window) = self.open_window(shell, WindowKind::Root, spec) {
            self.root.set(Some(window));
        }
    }

    /// Open a window of `kind` through the presenter, and enter it in the book: a key used twice ends the program.
    fn open_window(&self, shell: &Shell<'_>, kind: WindowKind, spec: WindowSpec) -> Option<WindowKey> {
        let title = spec.title.clone();
        match shell.open_window(&self.source(), spec) {
            Ok(window) => {
                if self.book.borrow_mut().opened(window, kind) {
                    self.titles.borrow_mut().insert(window, title);
                    Some(window)
                } else {
                    self.fail(shell, ExitKind::LoopFailed, "window.key_reused");
                    None
                }
            }
            Err(refusal) => {
                self.fail(shell, ExitKind::NoWindow, refusal.code());
                None
            }
        }
    }

    fn open_detached(&self, shell: &Shell<'_>, detached: &DetachedSpec) {
        let spec = WindowSpec {
            title: detached.title.clone(),
            inner: detached.size,
            min_inner: None,
            placement: Placement::System,
            class: WindowClass::Quiet,
        };
        let _ = self.open_window(shell, detached.kind(), spec);
    }

    /// Close one detached window: its tree lets go, then the presenter releases its surface and claim and destroys it.
    /// The other windows are not touched.
    fn close_detached(&self, shell: &Shell<'_>, window: WindowKey) {
        self.access.borrow_mut().close(window);
        let _ = shell.close_window(window);
    }

    /// The (window number, frames the queue took) of every open window the presenter is not closing.
    fn presenting(&self, shell: &Shell<'_>) -> Vec<(u64, u64)> {
        let live = self.book.borrow().live_keys();
        live.into_iter()
            .filter_map(|window| {
                let presentation = shell.presentation(window)?;
                (!presentation.closing).then(|| {
                    let taken = presentation.progress.map_or(0, |progress| progress.presented);
                    (window.number(), taken)
                })
            })
            .collect()
    }

    /// One step of the plan; whether it is finished.
    fn plan_step(&self, shell: &Shell<'_>, action: &Action) -> bool {
        match action {
            Action::Open(spec) => {
                self.open_detached(shell, spec);
                true
            }
            Action::CloseDetached(n) => {
                let target = self.book.borrow().detached(*n);
                let Some(window) = target else {
                    self.count(|counts| counts.plan_refused += 1);
                    return true;
                };
                if !self.book.borrow().live_keys().contains(&window) {
                    self.count(|counts| counts.plan_refused += 1);
                    return true;
                }
                if !self.created.borrow().contains(&window) {
                    return false;
                }
                let others: Vec<(u64, u64)> = self
                    .presenting(shell)
                    .into_iter()
                    .filter(|(number, _)| *number != window.number())
                    .collect();
                self.close_detached(shell, window);
                // The windows that stay are asked for a frame: idle windows draw nothing, so this is what shows that the
                // close did not take their surfaces, claims or the device with it.
                for (number, _) in &others {
                    let _ = shell.damage(WindowKey::from_number(*number));
                }
                *self.baseline.borrow_mut() = Some(others);
                true
            }
            Action::AwaitCreated(windows) => self.created.borrow().len() >= *windows,
            Action::AwaitPresented(frames) => shell.report().frames_presented >= *frames,
            Action::AwaitSurvivors => {
                let Some(before) = self.baseline.borrow().clone() else {
                    self.count(|counts| counts.plan_refused += 1);
                    return true;
                };
                if survivors_presented(&before, &self.presenting(shell)) {
                    self.survivors.set(Some(true));
                    true
                } else {
                    false
                }
            }
        }
    }

    /// The plan, in order, as far as each step allows.
    fn run_plan(&self, shell: &Shell<'_>) {
        loop {
            let next = self.plan.borrow().front().cloned();
            let Some(action) = next else {
                break;
            };
            if !self.plan_step(shell, &action) {
                break;
            }
            self.plan.borrow_mut().pop_front();
        }
    }

    fn arm_tick(&self) {
        let asked = self.config.end_after;
        if asked.time.is_some() || asked.frames_presented.is_some() || !self.plan.borrow().is_empty() {
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
        let kind = self.book.borrow().kind_of(window);
        self.created.borrow_mut().push(window);
        let usage = shell.owner().usage();
        self.count(|counts| counts.claims_peak = counts.claims_peak.max(usage.open_claims));
        let title = self
            .titles
            .borrow()
            .get(&window)
            .cloned()
            .unwrap_or_else(|| self.config.title.clone());
        let group = match kind {
            Some(kind) if kind != WindowKind::Root => format!("{SHELL_FRAME_NAME} {}", kind.name()),
            _ => SHELL_FRAME_NAME.to_owned(),
        };
        let frame = SemanticFrame::named_group(&title, &group);
        let given = self.access.borrow_mut().update(window, frame);
        if let Err(refusal) = given {
            self.access_code.set(self.access_code.get().or(Some(refusal.code())));
        }
        // The root of an Operator-launched product takes the focus once, with the launch authority; every other show
        // is quiet. `take_launch_source` gives the launch once, so no second window can be activated by it.
        let launch = (self.config.launched_by_operator && self.root.get() == Some(window))
            .then(|| control.take_launch_source(self.next_id()))
            .flatten();
        let shown = match launch {
            Some(launch) => control.command(&launch, window, WindowCommand::Show { activate: true }),
            None => control.command(&self.source(), window, WindowCommand::Show { activate: false }),
        };
        if let Err(refusal) = shown {
            self.fail(shell, ExitKind::NoWindow, refusal.code());
        }
    }

    fn close_requested(&self, shell: &Shell<'_>, window: WindowKey) {
        if self.root.get() == Some(window) {
            self.end(shell, 0);
        } else {
            // Another window of the program: its tree lets go first, then the presenter closes it.
            self.close_detached(shell, window);
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
        self.run_plan(shell);
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
        let ledger = shell.owner().counters();
        ended.ledger_admitted = ledger.admitted;
        ended.ledger_generations = ledger.generations;
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
            windows: self.book.borrow().counts(),
            windows_closed_early: self
                .closed_at_shutdown
                .get(),
            key_reuse: self.book.borrow().reused(),
            survivors_presented: self.survivors.get(),
            plan_refused: counts.plan_refused,
            plan_done: self.plan.borrow().is_empty(),
            claims_peak: counts.claims_peak,
            ledger_admitted: ended.ledger_admitted,
            ledger_generations: ended.ledger_generations,
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
            unavailable: self.unavailable.get(),
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
            Event::Window {
                window,
                event: WindowEvent::Destroyed,
            } => self.book.borrow_mut().closed(*window),
            Event::ShutdownBegan => {
                self.closed_at_shutdown.set(self.book.borrow().closed_count());
                self.access.borrow_mut().close_all();
            }
            Event::LoopEnding => self.ended(shell),
            _ => {}
        }
    }

    fn on_device(&self, shell: &Shell<'_>, event: DeviceEvent) {
        // The presenter recovered or gave up; what it did to the windows is in its report. When no device is left the
        // program ends with the typed, readable state and nothing else draws.
        if let Some(state) = unavailable_after(event) {
            if self.unavailable.get().is_none() {
                self.unavailable.set(Some(state));
            }
            self.fail(shell, state.end(), state.code());
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
