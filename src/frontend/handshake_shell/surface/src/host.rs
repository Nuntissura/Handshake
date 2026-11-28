//! The presenter on the window crate's loop: it attaches a surface when a window it opened is created, answers that
//! window's frames, configures, suspends and ends swapchains as the window changes, and keeps one wake lane of its own
//! for what has to happen later (an image that was not free, an old swapchain or a capture the owner has to observe,
//! a window waiting for its target, the end of the loop). The GPU owner runs no thread or timer, so that lane is what
//! lets it look; nothing on the frame path waits for the device.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use handshake_shell_window::{
    CommandSource, Control, DispatchedWake, Event, FrameOutcome, FrameTicket, PhysicalSize,
    Refusal, ShellHost, WakeLane, WindowEvent, WindowKey,
};
use handshake_vulkan_renderer::{
    AuthorizeRefusal, DeviceFault, GpuOwner, OwnerDiagnostics, OwnerOptions, PresentRefusal,
    PresentResult, Unavailable,
};

use crate::decide::{self, Again, Need, Schedule, Swap, Tried};
use crate::error::PresentError;
use crate::readback::{FrameReadback, ReadbackError};
use crate::recovery::{DeviceEvent, Recovery};
use crate::shell::{PresentConfig, PresentReport, Shell, ShellApp, WindowPresentation};
use crate::slot::{Capture, Slot};
use crate::target::ShellWindowTarget;
use crate::watch::PresentWatch;

mod ending;
mod loss;

/// The window crate's host that presents. Run it with `handshake_shell_window::run(config, host)`.
pub struct PresentHost {
    core: Core,
    app: Rc<dyn ShellApp>,
}

impl PresentHost {
    /// Open the process's GPU owner with presentation, on a real hardware Vulkan device, for `app`.
    ///
    /// # Errors
    ///
    /// [`Unavailable`]: no acceptable device, or none that presents to windows here
    /// ([`Unavailable::PresentationUnsupported`]). There is no other renderer to fall back to.
    pub fn open(
        config: PresentConfig,
        diagnostics: Arc<dyn OwnerDiagnostics>,
        app: Rc<dyn ShellApp>,
    ) -> Result<Rc<Self>, Unavailable> {
        let options = OwnerOptions { presentation: true };
        let owner = GpuOwner::open_with(config.policy, options, diagnostics)?;
        Ok(Rc::new(Self {
            core: Core::new(owner, config),
            app,
        }))
    }

    /// The GPU owner.
    pub fn owner(&self) -> &GpuOwner {
        &self.core.owner
    }

    /// The presenter's counters so far.
    pub fn report(&self) -> PresentReport {
        self.core.report()
    }
}

impl ShellHost for PresentHost {
    fn on_event(&self, control: &Control, event: Event) {
        match &event {
            Event::LoopStarted => self.core.started(control),
            Event::Window { window, event } => self.core.window_event(control, *window, event),
            _ => {}
        }
        let shell = Shell {
            control,
            core: &self.core,
        };
        self.app.on_event(&shell, &event);
        if matches!(event, Event::ShutdownBegan) {
            self.core.tear_down(control);
        }
        self.tell_device(control);
        self.core.rearm();
    }

    fn on_wake(&self, control: &Control, wake: &DispatchedWake<'_>) {
        if self.core.is_own(wake) {
            self.core.tick(control);
        } else {
            let shell = Shell {
                control,
                core: &self.core,
            };
            self.app.on_wake(&shell, wake);
        }
        self.tell_device(control);
        self.core.rearm();
    }
}

impl PresentHost {
    /// Tell the application what happened to the device since it was last told (AC214-9).
    fn tell_device(&self, control: &Control) {
        for event in self.core.take_device_events() {
            let shell = Shell {
                control,
                core: &self.core,
            };
            self.app.on_device(&shell, event);
        }
    }
}

pub(crate) struct Core {
    owner: GpuOwner,
    config: PresentConfig,
    /// The owner proves the end of old swapchains by their own present fences; otherwise later presents prove it.
    present_fences: bool,
    /// Proofs that ended at their own deadline (E9-F1 F3); read around the shutdown's proof.
    deadline_ends: Cell<u64>,
    lane: RefCell<Option<WakeLane>>,
    armed: Cell<Option<Instant>>,
    schedule: RefCell<Schedule>,
    /// Windows asked for through the shell and not created yet.
    expected: RefCell<BTreeSet<WindowKey>>,
    slots: RefCell<BTreeMap<WindowKey, Rc<RefCell<Slot>>>>,
    /// Windows asked for through the shell that got no surface: their frames are answered as unchanged.
    unpresented: RefCell<BTreeSet<WindowKey>>,
    /// Windows that are gone, whose claims still settle.
    ending: RefCell<Vec<Slot>>,
    report: Cell<PresentReport>,
    released: Arc<AtomicU64>,
    /// Looks at the owner left, and how many were taken in a row.
    looks: Cell<(u32, u32)>,
    finish_tries: Cell<u32>,
    exit_code: Cell<Option<i32>>,
    torn_down: Cell<bool>,
    commands: Cell<u64>,
    /// How a lost device is being recovered (AC214-9).
    recovery: Cell<Recovery>,
    /// What the application has not been told yet.
    device_events: RefCell<Vec<DeviceEvent>>,
}

impl Core {
    fn new(owner: GpuOwner, config: PresentConfig) -> Self {
        let present_fences = owner
            .identity()
            .extensions
            .contains(&"VK_EXT_swapchain_maintenance1");
        Self {
            owner,
            config,
            present_fences,
            deadline_ends: Cell::new(0),
            lane: RefCell::new(None),
            armed: Cell::new(None),
            schedule: RefCell::new(Schedule::default()),
            expected: RefCell::new(BTreeSet::new()),
            slots: RefCell::new(BTreeMap::new()),
            unpresented: RefCell::new(BTreeSet::new()),
            ending: RefCell::new(Vec::new()),
            report: Cell::new(PresentReport {
                present_fences,
                ..PresentReport::default()
            }),
            released: Arc::new(AtomicU64::new(0)),
            looks: Cell::new((0, 0)),
            finish_tries: Cell::new(0),
            exit_code: Cell::new(None),
            torn_down: Cell::new(false),
            commands: Cell::new(0),
            recovery: Cell::new(Recovery::new()),
            device_events: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn owner(&self) -> &GpuOwner {
        &self.owner
    }

    fn count(&self, change: impl FnOnce(&mut PresentReport)) {
        let mut report = self.report.get();
        change(&mut report);
        self.report.set(report);
    }

    fn error(&self, slot: &mut Slot, error: PresentError) {
        slot.error = Some(error);
        self.count(|report| {
            report.errors += 1;
            report.last_error = Some(error);
        });
    }

    pub(crate) fn report(&self) -> PresentReport {
        let mut report = self.report.get();
        report.targets_released = self.released.load(Ordering::Acquire);
        report
    }

    fn source(&self) -> CommandSource {
        let id = self.commands.get().wrapping_add(1);
        self.commands.set(id);
        CommandSource::host(id)
    }

    fn slot(&self, key: WindowKey) -> Option<Rc<RefCell<Slot>>> {
        self.slots.borrow().get(&key).cloned()
    }

    fn all_slots(&self) -> Vec<Rc<RefCell<Slot>>> {
        self.slots.borrow().values().cloned().collect()
    }

    fn started(&self, control: &Control) {
        // No wake is dispatched before the loop runs (MT-215 F2), so the lane is opened here.
        if let Ok(lane) = control.open_wake_lane() {
            *self.lane.borrow_mut() = Some(lane);
        }
    }

    fn is_own(&self, wake: &DispatchedWake<'_>) -> bool {
        self.lane
            .borrow()
            .as_ref()
            .is_some_and(|lane| lane.id() == wake.lane())
    }

    pub(crate) fn expect_window(&self, key: WindowKey) {
        self.expected.borrow_mut().insert(key);
    }

    pub(crate) fn set_exit(&self, code: i32) {
        self.exit_code.set(Some(code));
    }

    fn window_event(&self, control: &Control, key: WindowKey, event: &WindowEvent) {
        match event {
            WindowEvent::Created { state, .. } => {
                if self.expected.borrow_mut().remove(&key) {
                    self.attach(control, key, state.inner);
                }
            }
            WindowEvent::CreateFailed { .. } => {
                self.expected.borrow_mut().remove(&key);
            }
            WindowEvent::Destroyed => {
                self.unpresented.borrow_mut().remove(&key);
                self.schedule.borrow_mut().cancel(Need::Frame(key));
                let gone = self.slots.borrow_mut().remove(&key);
                // Its claim settles once the owner observed the end of its work; the looks go on for it.
                if let Some(slot) = gone.and_then(|slot| Rc::try_unwrap(slot).ok()) {
                    let slot = slot.into_inner();
                    if slot.holds_claim() {
                        self.ending.borrow_mut().push(slot);
                        self.want_looks();
                    }
                }
            }
            WindowEvent::FrameDue { ticket, .. } => self.frame(control, key, *ticket),
            other => self.changed(control, key, other),
        }
        self.publish(control, key);
    }

    /// Before the application hears of the window: a client, a claim, a surface and a swapchain.
    fn attach(&self, control: &Control, key: WindowKey, inner: PhysicalSize) {
        let Some(token) = control.native_window(key) else {
            self.unpresented.borrow_mut().insert(key);
            self.count(|report| {
                report.errors += 1;
                report.last_error = Some(PresentError::NoToken);
            });
            return;
        };
        let target = ShellWindowTarget::counted(token, Arc::clone(&self.released));
        match Slot::open(&self.owner, &self.config, key, Box::new(target)) {
            Ok(mut slot) => {
                self.count(|report| report.surfaces_made += 1);
                // A refusal stays the window's error; its first frame configures again.
                let _ = self.configure(&mut slot, inner);
                self.slots
                    .borrow_mut()
                    .insert(key, Rc::new(RefCell::new(slot)));
            }
            Err(error) => {
                self.unpresented.borrow_mut().insert(key);
                self.count(|report| {
                    report.errors += 1;
                    report.last_error = Some(error);
                });
            }
        }
    }

    fn configure(&self, slot: &mut Slot, inner: PhysicalSize) -> Result<(), PresentRefusal> {
        self.count(|report| report.configures += 1);
        match slot.configure(inner, &self.config) {
            Ok(replaced) => {
                if replaced {
                    self.count(|report| report.swapchains_replaced += 1);
                    let images = slot.swapchain().map_or(0, |info| info.images);
                    slot.retire_frames = if self.present_fences {
                        0
                    } else {
                        decide::retire_frames(images)
                    };
                    self.want_looks();
                }
                Ok(())
            }
            Err(refusal) => {
                self.error(slot, PresentError::Configure(refusal));
                Err(refusal)
            }
        }
    }

    /// A window event other than a frame.
    fn changed(&self, control: &Control, key: WindowKey, event: &WindowEvent) {
        let Some(slot) = self.slot(key) else {
            return;
        };
        let damage = {
            let mut slot = slot.borrow_mut();
            if slot.closing || !slot.has_surface() {
                return;
            }
            let steps = decide::on_window_event(slot.swap, event);
            if steps.suspend {
                match slot.suspend() {
                    Ok(()) => self.count(|report| report.suspends += 1),
                    Err(refusal) => self.error(&mut slot, PresentError::Configure(refusal)),
                }
            }
            if steps.stale {
                slot.swap = Swap::Stale;
            }
            steps.damage
        };
        if damage {
            let _ = control.request_frame(key);
        }
    }

    /// A frame of a window with a surface: configure when needed, acquire without waiting, clear, present, answer.
    fn frame(&self, control: &Control, key: WindowKey, ticket: FrameTicket) {
        // Frames of windows the presenter did not open are the application's to answer.
        let Some(slot) = self.slot(key) else {
            if self.unpresented.borrow().contains(&key) {
                let _ = control.frame_done(key, ticket, FrameOutcome::Unchanged);
            }
            return;
        };
        let mut slot = slot.borrow_mut();
        self.count(|report| report.frames_due += 1);
        if slot.closing || !slot.has_surface() {
            let _ = control.frame_done(key, ticket, FrameOutcome::Unchanged);
            return;
        }
        let tried = if slot.swap == Swap::Ready {
            slot.draw(&self.config.frame)
        } else {
            let inner = control.window_state(key).map(|state| state.inner);
            match self.configure(&mut slot, inner.unwrap_or_default()) {
                Err(refusal) => Tried::Refused(refusal),
                Ok(()) if slot.swap == Swap::Ready => slot.draw(&self.config.frame),
                Ok(()) => Tried::Refused(PresentRefusal::Suspended),
            }
        };
        let mut after = decide::after_frame(tried);
        if after.outcome == FrameOutcome::Presented {
            self.note_presented();
        }
        self.count(|report| match tried {
            Tried::Shown(PresentResult::Presented) => report.frames_presented += 1,
            Tried::Shown(PresentResult::Suboptimal) => {
                report.frames_presented += 1;
                report.frames_suboptimal += 1;
            }
            Tried::Refused(PresentRefusal::NotReady) => {
                report.not_ready += 1;
                report.frames_failed += 1;
            }
            _ if after.outcome == FrameOutcome::Failed => report.frames_failed += 1,
            _ => {}
        });
        // No free image is a wait, not a failure: bounded on its own, by about a second of retries.
        let not_ready = tried == Tried::Refused(PresentRefusal::NotReady);
        if not_ready {
            slot.not_ready = slot.not_ready.saturating_add(1);
        } else {
            slot.not_ready = 0;
            if after.outcome == FrameOutcome::Failed {
                slot.failures = slot.failures.saturating_add(1);
            } else {
                slot.failures = 0;
            }
        }
        if tried == Tried::Shown(PresentResult::Suboptimal) {
            slot.suboptimal = slot.suboptimal.saturating_add(1);
            if slot.suboptimal > 2 {
                // Stays stale: the next change or damage configures again.
                after.again = Again::No;
            }
        } else if after.outcome == FrameOutcome::Presented {
            slot.suboptimal = 0;
        }
        if after.stop {
            let error = match tried {
                Tried::Refused(refusal) => PresentError::Frame(refusal),
                _ => PresentError::NotClearable,
            };
            self.error(&mut slot, error);
        } else if slot.failures >= decide::MAX_FAILURES || slot.not_ready >= decide::MAX_NOT_READY {
            after.again = Again::No;
            self.error(&mut slot, PresentError::Failing);
        }
        let _ = control.frame_done(key, ticket, after.outcome);
        if after.stale {
            slot.swap = Swap::Stale;
        }
        if after.new_surface {
            self.remake(control, &mut slot);
        }
        if matches!(slot.capture, Capture::Pending(_)) {
            self.want_looks();
        }
        drop(slot);
        if after.lost {
            // After the slot is let go: the recovery releases every window's surface, this one's too.
            self.device_lost(loss_generation(tried));
        }
        match after.again {
            Again::No => {}
            Again::Now => {
                let _ = control.request_frame(key);
            }
            Again::After(delay) => self
                .schedule
                .borrow_mut()
                .arm(Need::Frame(key), Instant::now() + delay),
        }
    }

    fn remake(&self, control: &Control, slot: &mut Slot) {
        let Some(token) = control.native_window(slot.key) else {
            self.error(slot, PresentError::NoToken);
            return;
        };
        let target = ShellWindowTarget::counted(token, Arc::clone(&self.released));
        match slot.remake_surface(Box::new(target)) {
            Ok(()) => self.count(|report| {
                report.surfaces_released += 1;
                report.surfaces_made += 1;
                report.surfaces_remade += 1;
            }),
            Err(error) => self.error(slot, error),
        }
    }

    /// Something needs the owner looked at again: start the looks anew.
    fn want_looks(&self) {
        self.looks.set((decide::POLL_LOOKS, 0));
        let at = Instant::now() + decide::poll_delay(0);
        self.schedule.borrow_mut().arm(Need::Poll, at);
    }

    /// The presenter's wake.
    fn tick(&self, control: &Control) {
        self.armed.set(None);
        let due = self.schedule.borrow_mut().take_due(Instant::now());
        for need in due {
            match need {
                Need::Frame(key) => {
                    self.count(|report| report.timed_frames += 1);
                    let _ = control.request_frame(key);
                }
                Need::Poll => self.look(control),
                Need::Finish => self.try_finish(control),
                Need::Rebind => self.rebind_step(control),
            }
        }
        for key in self.slots.borrow().keys() {
            self.publish(control, *key);
        }
    }

    /// Let the owner end what ended, then do what that allows: frames that end old swapchains, captures, closing windows.
    fn look(&self, control: &Control) {
        self.count(|report| report.polls += 1);
        if let Err(DeviceFault::Lost { generation }) = self.owner.poll() {
            self.device_lost(Some(generation));
        }
        let slots = self.all_slots();
        let existing = self.owner.memory_use().map_or(0, |usage| usage.swapchains);
        let live = slots
            .iter()
            .filter(|slot| slot.try_borrow().is_ok_and(|slot| slot.holds_swapchain()))
            .count() as u64;
        // Old swapchains, of any window or of released surfaces, still exist.
        let ending = existing > live;
        let mut again = ending && self.present_fences;
        let mut retiring = false;
        for slot in &slots {
            let Ok(mut slot) = slot.try_borrow_mut() else {
                again = true;
                continue;
            };
            if !ending {
                slot.retire_frames = 0;
            } else if slot.retire_frames > 0 && slot.swap == Swap::Ready && !slot.closing {
                slot.retire_frames -= 1;
                retiring = true;
                self.count(|report| report.retire_frames += 1);
                let _ = control.request_frame(slot.key);
            }
            if matches!(slot.capture, Capture::Pending(_)) {
                again |= slot.poll_capture();
                if let Capture::Done(result) = &slot.capture {
                    let error = result.as_ref().err().copied();
                    self.count(|report| report.captures += u64::from(error.is_none()));
                    if let Some(error) = error {
                        self.error(&mut slot, PresentError::Capture(error));
                    }
                }
            }
            if slot.closing {
                again |= self.close_step(control, &mut slot);
            }
        }
        again |= self.settle_ending();
        let (left, step) = self.looks.get();
        if !(again || retiring) || left == 0 {
            return;
        }
        self.looks.set((left - 1, step + 1));
        let delay = if retiring {
            decide::RETIRE_FRAME_DELAY
        } else {
            decide::poll_delay(step)
        };
        self.schedule
            .borrow_mut()
            .arm(Need::Poll, Instant::now() + delay);
    }

    pub(crate) fn want_capture(&self, key: WindowKey) -> Result<(), Refusal> {
        let slot = self.slot(key).ok_or(Refusal::UnknownWindow)?;
        let mut slot = slot.borrow_mut();
        if slot.closing || !slot.has_surface() {
            return Err(Refusal::UnknownWindow);
        }
        slot.capture = Capture::Wanted;
        Ok(())
    }

    pub(crate) fn take_capture(
        &self,
        key: WindowKey,
    ) -> Option<Result<FrameReadback, ReadbackError>> {
        let slot = self.slot(key)?;
        let mut slot = slot.borrow_mut();
        match std::mem::replace(&mut slot.capture, Capture::None) {
            Capture::Done(result) => Some(result),
            other => {
                slot.capture = other;
                None
            }
        }
    }

    pub(crate) fn presentation(&self, key: WindowKey) -> Option<WindowPresentation> {
        let slot = self.slot(key)?;
        let slot = slot.try_borrow().ok()?;
        Some(WindowPresentation {
            swapchain: slot.swapchain(),
            suspended: slot.swap == Swap::Suspended,
            closing: slot.closing,
            progress: slot.watch.read().present,
            last_error: slot.error,
        })
    }

    pub(crate) fn watch(&self, key: WindowKey) -> Option<PresentWatch> {
        let slot = self.slot(key)?;
        let watch = slot.try_borrow().ok()?.watch.clone();
        Some(watch)
    }

    /// Tell the window's watch how long a frame has been waiting.
    pub(crate) fn publish(&self, control: &Control, key: WindowKey) {
        let Some(slot) = self.slot(key) else {
            return;
        };
        let Ok(slot) = slot.try_borrow() else {
            return;
        };
        let waiting = control
            .frame_progress(key)
            .and_then(|progress| progress.waiting);
        slot.watch.publish(waiting);
    }

    /// Ask the lane for the earliest need, when that is not what it was asked for already.
    fn rearm(&self) {
        let next = self.schedule.borrow().next();
        if next == self.armed.get() {
            return;
        }
        let lane = self.lane.borrow();
        let Some(lane) = lane.as_ref() else {
            return;
        };
        match next {
            Some(at) => {
                if lane
                    .request(0, at.saturating_duration_since(Instant::now()))
                    .is_ok()
                {
                    self.armed.set(Some(at));
                }
            }
            None => {
                lane.cancel();
                self.armed.set(None);
            }
        }
    }
}

/// The generation a frame's refusal names as lost, when it names one.
fn loss_generation(tried: Tried) -> Option<u64> {
    match tried {
        Tried::Refused(
            PresentRefusal::Device(DeviceFault::Lost { generation })
            | PresentRefusal::NotAuthorized(AuthorizeRefusal::Lost { generation }),
        ) => Some(generation),
        _ => None,
    }
}
