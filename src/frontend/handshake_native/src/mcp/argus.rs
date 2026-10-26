//! Canonical Argus method names and their compatibility aliases.
//!
//! Product-facing clients use the four `argus.*` names. The pre-Argus MCP spellings remain accepted
//! at the wire boundary so older clients keep working, but callers inside the product can normalize a
//! request once and use the canonical method throughout leasing, dispatch, attribution, and manuals.

use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Weak, Mutex};
use std::time::{Duration, Instant};

pub const ACTION_RECEIPT_TIMEOUT: Duration = Duration::from_secs(10);
pub const ACTION_RECEIPT_DURABILITY_TIMEOUT: Duration = Duration::from_secs(5);
pub const ACTION_RECEIPT_TRANSPORT_MARGIN: Duration = Duration::from_secs(2);
pub const ACTION_RECEIPT_TRANSPORT_TIMEOUT: Duration = Duration::from_secs(20);

/// A handler signal owned by one existing ActionChannel transaction. Context data stores weak
/// reservations only; it never owns receipts or terminalizes them.
pub(crate) struct ActionEffectTicket {
    context: egui::Context,
    viewport: egui::ViewportId,
    receipt_id: u64,
    author_id: String,
    request: egui::accesskit::ActionRequest,
    dispatched_frame: u64,
    deadline: Instant,
    live: AtomicBool,
    acknowledged: AtomicBool,
    secret_mutation: AtomicBool,
    allow_target_transition: AtomicBool,
    epoch: u64,
}

#[derive(Debug, Clone)]
struct ActionEffectReservation {
    ticket: Weak<ActionEffectTicket>,
    author_id: String,
    dispatched_frame: u64,
    viewport: egui::ViewportId,
}

impl ActionEffectTicket {
    fn is_live(&self) -> bool {
        self.live.load(Ordering::Acquire) && Instant::now() < self.deadline
    }

    pub(crate) fn secret_mutation_acknowledged(&self) -> bool {
        self.secret_mutation.load(Ordering::Acquire)
    }

    pub(crate) fn allows_target_transition(&self) -> bool {
        self.allow_target_transition.load(Ordering::Acquire)
    }

    pub(crate) fn release(&self) {
        self.live.store(false, Ordering::Release);
        self.acknowledged.store(false, Ordering::Release);
        self.secret_mutation.store(false, Ordering::Release);
        self.allow_target_transition.store(false, Ordering::Release);
    }

    pub(crate) fn acknowledged_after_frame(&self, ctx: &egui::Context, receipt_id: u64) -> bool {
        self.receipt_id == receipt_id
            && self.context == *ctx
            && ctx.viewport_id() == self.viewport
            && self.is_live()
            && self.acknowledged.load(Ordering::Acquire)
            && ctx.cumulative_frame_nr_for(self.viewport) > self.dispatched_frame
    }

    pub(crate) fn acknowledged_after_registered_frame(&self, ctx: &egui::Context, receipt_id: u64, window: &RegisteredWindow) -> bool {
        self.receipt_id == receipt_id && self.context == *ctx && self.epoch == window.epoch
            && self.viewport == window.descriptor.viewport_id && window.live.load(Ordering::Acquire)
            && self.is_live() && self.acknowledged.load(Ordering::Acquire)
            && ctx.cumulative_frame_nr_for(self.viewport) > self.dispatched_frame
    }
}

/// Admit only the actual root raw-input viewport. Unknown/child viewports never acquire a root
/// reservation or receive an injected event through this bridge.
pub(crate) fn register_action_effect(
    ctx: &egui::Context,
    viewport: egui::ViewportId,
    receipt_id: u64,
    author_id: &str,
    request: &egui::accesskit::ActionRequest,
    deadline: Instant,
) -> Option<Arc<ActionEffectTicket>> {
    if viewport != egui::ViewportId::ROOT || Instant::now() >= deadline {
        return None;
    }
    register_admitted_action_effect(ctx, viewport, 0, receipt_id, author_id, request, deadline)
}

pub(crate) fn register_admitted_action_effect(
    ctx: &egui::Context,
    viewport: egui::ViewportId,
    epoch: u64,
    receipt_id: u64,
    author_id: &str,
    request: &egui::accesskit::ActionRequest,
    deadline: Instant,
) -> Option<Arc<ActionEffectTicket>> {
    if Instant::now() >= deadline { return None; }
    let ticket = Arc::new(ActionEffectTicket {
        context: ctx.clone(),
        viewport,
        receipt_id,
        author_id: author_id.to_owned(),
        request: request.clone(),
        dispatched_frame: ctx.cumulative_frame_nr_for(viewport),
        deadline,
        live: AtomicBool::new(true),
        acknowledged: AtomicBool::new(false),
        secret_mutation: AtomicBool::new(false),
        allow_target_transition: AtomicBool::new(false),
        epoch,
    });
    ctx.data_mut(|data| {
        let id = egui::Id::new("argus.action-effect-reservations");
        let mut reservations = data.get_temp::<Vec<ActionEffectReservation>>(id).unwrap_or_default();
        // A released weak handle is unroutable, but its current-frame tombstone prevents the
        // identical old AccessKit event from being attributed to a new receipt/channel epoch.
        reservations.retain(|entry| entry.viewport != viewport || entry.dispatched_frame == ticket.dispatched_frame
            || entry.ticket.upgrade().is_some_and(|ticket| ticket.is_live()));
        if reservations.len() >= 32 * 256 || reservations.iter().filter(|entry| entry.viewport == viewport).count() >= 256 || reservations.iter().any(|entry| {
            entry.viewport == viewport && entry.author_id == author_id
        }) {
            return None;
        }
        reservations.push(ActionEffectReservation {
            ticket: Arc::downgrade(&ticket),
            author_id: author_id.to_owned(),
            dispatched_frame: ticket.dispatched_frame,
            viewport,
        });
        data.insert_temp(id, reservations);
        Some(ticket)
    })
}

/// Signal the exact dispatched request only after the named production handler applies its effect.
/// A signal remains nonterminal until the channel observes a later completed live frame.
pub fn acknowledge_action_effect(ctx: &egui::Context, author_id: &str) {
    acknowledge_action_effect_with_policy(ctx, author_id, false);
}

/// Opt in only after a click's applied host effect intentionally disables or removes its control.
pub(crate) fn acknowledge_action_effect_allow_target_transition(ctx: &egui::Context, author_id: &str) {
    acknowledge_action_effect_with_policy(ctx, author_id, true);
}

fn acknowledge_action_effect_with_policy(ctx: &egui::Context, author_id: &str, allow_transition: bool) {
    let viewport = ctx.viewport_id();
    let reservations = ctx.data(|data| {
        data.get_temp::<Vec<ActionEffectReservation>>(egui::Id::new("argus.action-effect-reservations"))
            .unwrap_or_default()
    });
    for ticket in reservations.iter().filter_map(|entry| entry.ticket.upgrade()) {
        if ticket.context == *ctx && ticket.viewport == viewport && ticket.author_id == author_id
            && ticket.is_live() && ctx.cumulative_frame_nr_for(viewport) == ticket.dispatched_frame
            && ctx.input(|input| input.events.iter().any(|event| {
                matches!(event, egui::Event::AccessKitActionRequest(request)
                    if request == &ticket.request)
            }))
        {
            if allow_transition && ticket.request.action == egui::accesskit::Action::Click {
                ticket.allow_target_transition.store(true, Ordering::Release);
            }
            ticket.acknowledged.store(true, Ordering::Release);
        }
    }
}

pub(crate) fn acknowledge_secret_set_value(
    ctx: &egui::Context, author_id: &str, widget_id: egui::Id, owned_value: &str,
) {
    if !crate::accessibility::is_sensitive_author_id(author_id)
        || !matches!(author_id, "settings.cloud.byok.anthropic.key" | "settings.cloud.byok.openai.key" | "account.password") {
        return;
    }
    let viewport = ctx.viewport_id();
    let reservations = ctx.data(|data| data.get_temp::<Vec<ActionEffectReservation>>(
        egui::Id::new("argus.action-effect-reservations")).unwrap_or_default());
    for ticket in reservations.iter().filter_map(|entry| entry.ticket.upgrade()) {
        if ticket.context != *ctx || ticket.viewport != viewport || ticket.author_id != author_id
            || !ticket.is_live() || ctx.cumulative_frame_nr_for(viewport) != ticket.dispatched_frame
            || ticket.request.action != egui::accesskit::Action::SetValue
            || ticket.request.target.0 != widget_id.value() {
            continue;
        }
        let exact_mutation = matches!(&ticket.request.data,
            Some(egui::accesskit::ActionData::Value(requested)) if requested.as_ref() == owned_value)
            && ctx.input(|input| input.events.iter().any(|event|
                matches!(event, egui::Event::AccessKitActionRequest(request) if request == &ticket.request)));
        if exact_mutation {
            ticket.secret_mutation.store(true, Ordering::Release);
            ticket.acknowledged.store(true, Ordering::Release);
        }
    }
}

impl std::fmt::Debug for ActionEffectTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionEffectTicket").field("receipt_id", &self.receipt_id)
            .field("author_id", &self.author_id).field("viewport", &self.viewport)
            .field("epoch", &self.epoch).finish_non_exhaustive()
    }
}

impl Drop for ActionEffectTicket {
    fn drop(&mut self) { super::action::wipe_action_request(&mut self.request); }
}

pub const MAIN_WINDOW_ID: &str = "main";

#[derive(Debug, Clone)]
pub struct ArgusWindowDescriptor {
    pub window_id: String,
    pub viewport_id: egui::ViewportId,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ArgusWindowListing {
    pub window_id: String,
    pub viewport_id: String,
    pub title: String,
    pub epoch: u64,
    pub revision: u64,
    pub snapshot_available: bool,
    pub closing: bool,
}

#[derive(Clone)]
pub(crate) struct RegisteredWindow {
    pub descriptor: ArgusWindowDescriptor,
    pub epoch: u64,
    pub snapshot: Arc<Mutex<crate::accessibility::UiTreeSnapshot>>,
    pub channel: Arc<Mutex<super::action::ActionChannel>>,
    pub live: Arc<AtomicBool>,
    revision: Arc<std::sync::atomic::AtomicU64>,
    published: Arc<AtomicBool>,
    closing: Arc<AtomicBool>,
}

impl RegisteredWindow {
    pub(crate) fn revision(&self) -> u64 { self.revision.load(Ordering::Acquire) }
    fn validate_live_publication(&self) -> Result<(), String> {
        if !self.live.load(Ordering::Acquire) || self.closing.load(Ordering::Acquire) {
            return Err("Argus window is closed or closing".to_owned());
        }
        if !self.published.load(Ordering::Acquire) {
            return Err("Argus window has no published live snapshot".to_owned());
        }
        Ok(())
    }
    pub(crate) fn validate(&self, revision: u64) -> Result<(), String> {
        self.validate_live_publication()?;
        if revision != self.revision() { return Err("Argus window revision is stale".to_owned()); }
        Ok(())
    }
}

/// Admitted window/privacy lifetime, independent from ordinary frame publications.
#[derive(Clone)]
pub(crate) struct WindowCaptureTicket {
    registry: WindowSnapshotRegistry,
    window: RegisteredWindow,
    context_generation: u64,
    account: Option<Arc<crate::local_account::AuthenticatedContext>>,
}

pub(crate) struct WorkspaceActivationReadbackTicket {
    privacy: WindowCaptureTicket,
    binding: Option<Arc<crate::workspace_activation_observation::WorkspaceActivationReadback>>,
}

impl WorkspaceActivationReadbackTicket {
    fn is_current(&self) -> bool {
        if self.privacy.validate().is_err() { return false; }
        let context = self.privacy.registry.capture_context.lock().unwrap_or_else(|e| e.into_inner());
        if context.generation != self.privacy.context_generation { return false; }
        match (&self.binding, &context.activation_readback) {
            (None, None) => true,
            (Some(binding), Some(current)) if Arc::ptr_eq(binding, current) => {
                context.account.as_ref().is_some_and(|account| binding.matches_owner(account, &context.workspace_id))
            }
            _ => false,
        }
    }

    pub(crate) async fn fetch(&self) -> serde_json::Value {
        if !self.is_current() { return serde_json::json!({"status":"stale"}); }
        let value = match &self.binding {
            Some(binding) => binding.fetch().await,
            None => serde_json::json!({"status":"unavailable","reason":"no_admitted_original"}),
        };
        if !self.is_current() { return serde_json::json!({"status":"stale"}); }
        value
    }
}

impl WindowCaptureTicket {
    /// Returns an observed UI publication revision, not an exact renderer-image revision.
    pub(crate) fn validate(&self) -> Result<u64, String> {
        let generation = self.registry.capture_context.lock().unwrap_or_else(|e| e.into_inner());
        if generation.generation != self.context_generation {
            return Err("Argus capture privacy context was retired".to_owned());
        }
        if self.account.as_ref().is_some_and(|account| !account.is_active()) {
            return Err("Argus capture account was revoked".to_owned());
        }
        let state = self.registry.inner.lock().unwrap_or_else(|e| e.into_inner());
        validate_capture_registration(&state, &self.window)?;
        Ok(self.window.revision())
    }
}

fn validate_capture_registration(state: &WindowRegistryState, window: &RegisteredWindow) -> Result<(), String> {
    let current = state.windows.get(&window.descriptor.window_id)
        .ok_or_else(|| "Argus capture window is no longer registered".to_owned())?;
    if current.epoch != window.epoch
        || current.descriptor.viewport_id != window.descriptor.viewport_id
        || current.descriptor.title != window.descriptor.title
        || !Arc::ptr_eq(&current.live, &window.live)
        || !Arc::ptr_eq(&current.snapshot, &window.snapshot)
        || !Arc::ptr_eq(&current.channel, &window.channel)
    {
        return Err("Argus capture window registration changed".to_owned());
    }
    window.validate_live_publication()
}

#[derive(Clone)]
pub struct WindowSnapshotRegistry {
    inner: Arc<Mutex<WindowRegistryState>>,
    next_revision: Arc<std::sync::atomic::AtomicU64>,
    capture_context: Arc<Mutex<CaptureContext>>,
}

#[derive(Default)]
struct CaptureContext {
    generation: u64,
    account: Option<Arc<crate::local_account::AuthenticatedContext>>,
    workspace_id: String,
    activation_readback: Option<Arc<crate::workspace_activation_observation::WorkspaceActivationReadback>>,
}

impl CaptureContext {
    fn retire(&mut self, state: &WindowRegistryState) {
        self.activation_readback = None;
        let next = self.generation.checked_add(1);
        for window in state.windows.values() {
            window.published.store(false, Ordering::Release);
            if next.is_none() { window.live.store(false, Ordering::Release); }
        }
        if let Some(next) = next { self.generation = next; }
    }
}

struct WindowRegistryState {
    windows: std::collections::HashMap<String, RegisteredWindow>,
    next_epoch: u64,
}

impl WindowSnapshotRegistry {
    pub fn new(snapshot: Arc<Mutex<crate::accessibility::UiTreeSnapshot>>, channel: Arc<Mutex<super::action::ActionChannel>>) -> Self {
        let root = RegisteredWindow {
            descriptor: ArgusWindowDescriptor { window_id: MAIN_WINDOW_ID.to_owned(), viewport_id: egui::ViewportId::ROOT, title: super::screenshot::HANDSHAKE_WINDOW_TITLE.to_owned() },
            epoch: 1, snapshot, channel, live: Arc::new(AtomicBool::new(true)),
            revision: Arc::new(std::sync::atomic::AtomicU64::new(0)), published: Arc::new(AtomicBool::new(false)), closing: Arc::new(AtomicBool::new(false)),
        };
        Self { inner: Arc::new(Mutex::new(WindowRegistryState { windows: std::collections::HashMap::from([(MAIN_WINDOW_ID.to_owned(), root)]), next_epoch: 2 })), next_revision: Arc::new(std::sync::atomic::AtomicU64::new(1)), capture_context: Arc::new(Mutex::new(CaptureContext::default())) }
    }

    pub(crate) fn admit_capture(&self, window: &RegisteredWindow, expected_revision: u64) -> Result<WindowCaptureTicket, String> {
        self.admit_capture_revision(window, Some(expected_revision)).map(|(_, ticket)| ticket)
    }

    pub(crate) fn admit_workspace_activation_readback(
        &self, window: &RegisteredWindow, expected_revision: u64,
    ) -> Result<WorkspaceActivationReadbackTicket, String> {
        if window.descriptor.window_id != MAIN_WINDOW_ID {
            return Err("Workspace activation readback requires the main window".to_owned());
        }
        let privacy = self.admit_capture(window, expected_revision)?;
        let binding = self.capture_context.lock().unwrap_or_else(|e| e.into_inner()).activation_readback.clone();
        let ticket = WorkspaceActivationReadbackTicket { privacy, binding };
        if !ticket.is_current() { return Err("Workspace activation readback context is stale".to_owned()); }
        Ok(ticket)
    }

    pub(crate) fn bind_workspace_activation_readback(
        &self, account: Arc<crate::local_account::AuthenticatedContext>, workspace_id: &str,
        operation_id: uuid::Uuid, generation: u64, backend: &str,
    ) -> bool {
        let mut context = self.capture_context.lock().unwrap_or_else(|e| e.into_inner());
        if context.workspace_id != workspace_id
            || !context.account.as_ref().is_some_and(|current| Arc::ptr_eq(current, &account))
        { context.activation_readback = None; return false; }
        if context.activation_readback.as_ref().is_some_and(|current| {
            current.matches_original(&account, workspace_id, operation_id, generation, backend)
        }) { return true; }
        context.activation_readback = None;
        let Ok(binding) = crate::workspace_activation_observation::WorkspaceActivationReadback::new(
            account, workspace_id.to_owned(), operation_id, generation, backend.to_owned(),
        ) else { return false; };
        context.activation_readback = Some(Arc::new(binding));
        true
    }

    pub(crate) fn clear_workspace_activation_readback(&self) {
        self.capture_context.lock().unwrap_or_else(|e| e.into_inner()).activation_readback = None;
    }

    /// Selectorless compatibility captures admit the actual current main publication atomically.
    pub(crate) fn admit_current_capture(&self, window: &RegisteredWindow) -> Result<(u64, WindowCaptureTicket), String> {
        self.admit_capture_revision(window, None)
    }

    fn admit_capture_revision(&self, window: &RegisteredWindow, expected_revision: Option<u64>) -> Result<(u64, WindowCaptureTicket), String> {
        let generation = self.capture_context.lock().unwrap_or_else(|e| e.into_inner());
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        validate_capture_registration(&state, window)?;
        let revision = window.revision();
        window.validate(expected_revision.unwrap_or(revision))?;
        if generation.account.as_ref().is_some_and(|account| !account.is_active()) {
            return Err("Argus capture account was revoked".to_owned());
        }
        Ok((revision, WindowCaptureTicket {
            registry: self.clone(), window: window.clone(), context_generation: generation.generation,
            account: generation.account.clone(),
        }))
    }

    /// Call before any account/private-state mutation. New admission waits for a fresh publication.
    pub fn retire_capture_context(&self) {
        let mut context = self.capture_context.lock().unwrap_or_else(|e| e.into_inner());
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        context.retire(&state);
    }

    /// Bind only the host's existing account revocation guard and actual workspace owner.
    /// Tickets read the same guard directly, without waiting for a UI/account poll.
    pub fn bind_capture_owner(
        &self, account: Option<Arc<crate::local_account::AuthenticatedContext>>, workspace_id: &str,
    ) {
        let mut context = self.capture_context.lock().unwrap_or_else(|e| e.into_inner());
        let same_account = match (&context.account, &account) {
            (Some(previous), Some(current)) => Arc::ptr_eq(previous, current),
            (None, None) => true,
            _ => false,
        };
        if same_account && context.workspace_id == workspace_id { return; }
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        context.retire(&state);
        context.account = account;
        context.workspace_id = workspace_id.to_owned();
    }

    pub fn register(&self, descriptor: ArgusWindowDescriptor) -> Result<u64, String> {
        let root = self.get(MAIN_WINDOW_ID)?;
        let seed = root.snapshot.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if descriptor.window_id.is_empty() || descriptor.title.is_empty() { return Err("Argus window descriptor is empty".to_owned()); }
        if let Some(window) = state.windows.get(&descriptor.window_id) {
            if window.descriptor.viewport_id != descriptor.viewport_id || window.descriptor.title != descriptor.title {
                return Err("Argus window descriptor changed without unregister".to_owned());
            }
            return Ok(window.epoch);
        }
        if descriptor.viewport_id == egui::ViewportId::ROOT || state.windows.len() >= 32
            || state.windows.values().any(|w| w.descriptor.viewport_id == descriptor.viewport_id) {
            return Err("Argus viewport is already registered or registry is full".to_owned());
        }
        let epoch = state.next_epoch;
        state.next_epoch = epoch.checked_add(1).ok_or("Argus window epoch exhausted")?;
        let window = RegisteredWindow {
            descriptor: descriptor.clone(), epoch,
            snapshot: Arc::new(Mutex::new(seed)),
            channel: Arc::new(Mutex::new(super::action::ActionChannel::new())),
            live: Arc::new(AtomicBool::new(true)), revision: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            published: Arc::new(AtomicBool::new(false)), closing: Arc::new(AtomicBool::new(false)),
        };
        state.windows.insert(descriptor.window_id, window);
        Ok(epoch)
    }

    pub(crate) fn get(&self, window_id: &str) -> Result<RegisteredWindow, String> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).windows.get(window_id).cloned().ok_or_else(|| "Unknown Argus window".to_owned())
    }

    fn by_viewport(&self, viewport: egui::ViewportId) -> Option<RegisteredWindow> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).windows.values().find(|w| w.descriptor.viewport_id == viewport).cloned()
    }

    pub fn list(&self) -> Vec<ArgusWindowListing> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut windows: Vec<_> = state.windows.values().map(|w| ArgusWindowListing {
            window_id: w.descriptor.window_id.clone(),
            viewport_id: format!("{:?}", w.descriptor.viewport_id),
            title: w.descriptor.title.clone(),
            epoch: w.epoch,
            revision: w.revision(),
            snapshot_available: w.published.load(Ordering::Acquire),
            closing: w.closing.load(Ordering::Acquire),
        }).collect();
        windows.sort_by(|a, b| a.window_id.cmp(&b.window_id));
        windows
    }

    pub fn install_output_plugin(&self, ctx: &egui::Context) {
        let key = egui::Id::new("argus.registered-window-output-installed");
        let install = ctx.data_mut(|d| { if d.get_temp::<bool>(key).unwrap_or(false) { false } else { d.insert_temp(key, true); true } });
        if install { ctx.add_plugin(ArgusOutputPlugin { windows: self.clone(), passes: Vec::new() }); }
    }

    pub fn note_root_publication_locked(&self, production_ctx: Option<&egui::Context>) {
        let _capture_context = self.capture_context.lock().unwrap_or_else(|e| e.into_inner());
        if production_ctx.is_some_and(|ctx| ctx.viewport_id() != egui::ViewportId::ROOT) { return; }
        if let Ok(root) = self.get(MAIN_WINDOW_ID) {
            if let Ok(revision) = self.next_revision.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1)) {
                root.revision.store(revision, Ordering::Release);
                root.published.store(true, Ordering::Release);
            } else { root.live.store(false, Ordering::Release); }
        }
    }

    fn publish_revision(&self, window: &RegisteredWindow) -> bool {
        let _capture_context = self.capture_context.lock().unwrap_or_else(|e| e.into_inner());
        match self.next_revision.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1)) {
            Ok(revision) => { window.revision.store(revision, Ordering::Release); window.published.store(true, Ordering::Release); true }
            Err(_) => { window.live.store(false, Ordering::Release); false }
        }
    }

    pub fn pump_viewport_input(&self, ctx: &egui::Context, window_id: &str, admit_new: bool) -> Result<bool, String> {
        let window = self.get(window_id)?;
        if window.descriptor.viewport_id == egui::ViewportId::ROOT || ctx.viewport_id() != window.descriptor.viewport_id { return Err("Argus child viewport identity mismatch".to_owned()); }
        if !window.live.load(Ordering::Acquire) || !window.published.load(Ordering::Acquire) { return Ok(false); }
        let snapshot = window.snapshot.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let events = {
            let mut channel = window.channel.lock().unwrap_or_else(|e| e.into_inner());
            let operator_input = ctx.input(|input| input.events.iter().any(|event| matches!(event,
                egui::Event::Cut | egui::Event::Text(_) | egui::Event::Paste(_)
                | egui::Event::Ime(egui::ImeEvent::Commit(_)) | egui::Event::AccessKitActionRequest(_)
                | egui::Event::Key { pressed: true, .. } | egui::Event::PointerButton { .. }
                | egui::Event::PointerMoved(_) | egui::Event::MouseMoved(_) | egui::Event::MouseWheel { .. }
                | egui::Event::Touch { .. })));
            if operator_input { channel.reject_queued_for_operator_input(); Vec::new() }
            else if admit_new && !window.closing.load(Ordering::Acquire) {
                channel.drain_revalidated_into_registered_events(&snapshot, ctx, &window)
            } else { Vec::new() }
        };
        if !events.is_empty() { ctx.input_mut(|input| input.events.extend(events)); }
        Ok(self.has_pending_actions(window_id))
    }

    pub fn has_pending_actions(&self, window_id: &str) -> bool {
        self.get(window_id).map(|w| w.channel.lock().unwrap_or_else(|e| e.into_inner()).pending() > 0).unwrap_or(false)
    }

    pub fn begin_close(&self, window_id: &str) {
        if let Ok(window) = self.get(window_id) {
            let channel = window.channel.lock().unwrap_or_else(|e| e.into_inner());
            window.closing.store(true, Ordering::Release);
            drop(channel);
        }
    }

    pub fn is_close_ready(&self, window_id: &str) -> bool { !self.has_pending_actions(window_id) }

    pub fn unregister(&self, window_id: &str) {
        if window_id == MAIN_WINDOW_ID { return; }
        let window = self.inner.lock().unwrap_or_else(|e| e.into_inner()).windows.remove(window_id);
        if let Some(window) = window {
            let mut channel = window.channel.lock().unwrap_or_else(|e| e.into_inner());
            window.live.store(false, Ordering::Release);
            channel.cancel_pending_actions();
            drop(channel);
            super::screenshot::clear_registered_window_handle(&window);
        }
    }

    pub(crate) fn cancel_all(&self) {
        let windows: Vec<_> = self.inner.lock().unwrap_or_else(|e| e.into_inner()).windows.values().cloned().collect();
        for window in windows {
            let mut channel = window.channel.lock().unwrap_or_else(|e| e.into_inner());
            window.live.store(false, Ordering::Release);
            window.closing.store(true, Ordering::Release);
            channel.cancel_pending_actions();
            drop(channel);
            super::screenshot::clear_registered_window_handle(&window);
        }
    }
}

pub struct ArgusOutputPlugin {
    windows: WindowSnapshotRegistry,
    passes: Vec<(egui::ViewportId, Option<(egui::Context, crate::accessibility::SnapshotViewport)>)>,
}

impl egui::Plugin for ArgusOutputPlugin {
    fn debug_name(&self) -> &'static str { "argus-registered-window-output" }
    fn input_hook(&mut self, input: &mut egui::RawInput) { self.passes.push((input.viewport_id, None)); }
    fn on_end_pass(&mut self, ctx: &egui::Context) {
        if let Some((viewport, context)) = self.passes.last_mut() {
            if *viewport == ctx.viewport_id() {
                let rect = ctx.input(|input| input.viewport_rect());
                if rect.is_finite() && rect.width() > 0.0 && rect.height() > 0.0 {
                    *context = Some((ctx.clone(), crate::accessibility::SnapshotViewport {
                        x: rect.min.x, y: rect.min.y, w: rect.width(), h: rect.height(), source: crate::accessibility::ViewportSource::LiveWindow,
                    }));
                }
            }
        }
    }
    fn output_hook(&mut self, output: &mut egui::FullOutput) {
        let Some((viewport, Some((ctx, measured_viewport)))) = self.passes.pop() else { return; };
        if viewport == egui::ViewportId::ROOT { return; }
        let Some(window) = self.windows.by_viewport(viewport) else { return; };
        if !window.live.load(Ordering::Acquire) { return; }
        let Some(update) = output.platform_output.accesskit_update.as_ref() else { return; };
        let snapshot = crate::accessibility::collect_ui_tree_snapshot_with_viewport(update, Some(measured_viewport));
        let mut slot = window.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        let mut channel = window.channel.lock().unwrap_or_else(|e| e.into_inner());
        channel.acknowledge_after_registered_render(&snapshot, &ctx, &window);
        *slot = snapshot;
        if !self.windows.publish_revision(&window) { channel.cancel_pending_actions(); }
        drop(channel);
        drop(slot);
        if self.windows.has_pending_actions(&window.descriptor.window_id) { ctx.request_repaint_of(viewport); }
    }
}

/// Canonical read-the-live-AccessKit-tree method.
pub const ARGUS_INSPECT_METHOD: &str = "argus.inspect";
pub const ARGUS_LIST_WINDOWS_METHOD: &str = "argus.list_windows";
/// Canonical activate-an-addressed-control method.
pub const ARGUS_CLICK_METHOD: &str = "argus.click";
pub const ARGUS_SHOW_CONTEXT_MENU_METHOD: &str = "argus.show_context_menu";
/// Canonical replace-an-addressed-control's whole value method.
pub const ARGUS_SET_VALUE_METHOD: &str = "argus.set_value";
/// Canonical focus-safe pixel capture method.
pub const ARGUS_SCREENSHOT_METHOD: &str = "argus.screenshot";

/// Legacy aliases retained only at the compatibility boundary.
pub const LEGACY_INSPECT_METHOD: &str = "list_widgets";
pub const LEGACY_CLICK_METHOD: &str = "click_widget";
pub const LEGACY_SET_VALUE_METHOD: &str = "set_value";
pub const LEGACY_SCREENSHOT_METHOD: &str = "screenshot";

/// The closed set of Argus operations after wire-name normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgusMethod {
    ListWindows,
    Inspect,
    Click,
    ShowContextMenu,
    SetValue,
    Screenshot,
}

impl ArgusMethod {
    /// Accept a canonical method or its legacy alias.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        match name {
            ARGUS_LIST_WINDOWS_METHOD => Some(Self::ListWindows),
            ARGUS_INSPECT_METHOD | LEGACY_INSPECT_METHOD => Some(Self::Inspect),
            ARGUS_CLICK_METHOD | LEGACY_CLICK_METHOD => Some(Self::Click),
            ARGUS_SHOW_CONTEXT_MENU_METHOD => Some(Self::ShowContextMenu),
            ARGUS_SET_VALUE_METHOD | LEGACY_SET_VALUE_METHOD => Some(Self::SetValue),
            ARGUS_SCREENSHOT_METHOD | LEGACY_SCREENSHOT_METHOD => Some(Self::Screenshot),
            _ => None,
        }
    }

    /// Stable product-facing name used in manuals, receipts, and attributed action logs.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::ListWindows => ARGUS_LIST_WINDOWS_METHOD,
            Self::Inspect => ARGUS_INSPECT_METHOD,
            Self::Click => ARGUS_CLICK_METHOD,
            Self::ShowContextMenu => ARGUS_SHOW_CONTEXT_MENU_METHOD,
            Self::SetValue => ARGUS_SET_VALUE_METHOD,
            Self::Screenshot => ARGUS_SCREENSHOT_METHOD,
        }
    }

    /// Whether this operation mutates an addressed live control.
    pub const fn is_mutating(self) -> bool {
        matches!(self, Self::Click | Self::ShowContextMenu | Self::SetValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_transport_budget_exceeds_wait_and_durability_with_margin() {
        assert!(
            ACTION_RECEIPT_TRANSPORT_TIMEOUT
                > ACTION_RECEIPT_TIMEOUT
                    + ACTION_RECEIPT_DURABILITY_TIMEOUT
                    + ACTION_RECEIPT_TRANSPORT_MARGIN
        );
    }

    #[test]
    fn canonical_and_legacy_names_normalize_to_the_same_operations() {
        for (canonical, legacy, expected) in [
            (
                ARGUS_INSPECT_METHOD,
                LEGACY_INSPECT_METHOD,
                ArgusMethod::Inspect,
            ),
            (ARGUS_CLICK_METHOD, LEGACY_CLICK_METHOD, ArgusMethod::Click),
            (
                ARGUS_SET_VALUE_METHOD,
                LEGACY_SET_VALUE_METHOD,
                ArgusMethod::SetValue,
            ),
            (
                ARGUS_SCREENSHOT_METHOD,
                LEGACY_SCREENSHOT_METHOD,
                ArgusMethod::Screenshot,
            ),
        ] {
            assert_eq!(ArgusMethod::from_wire_name(canonical), Some(expected));
            assert_eq!(ArgusMethod::from_wire_name(legacy), Some(expected));
            assert_eq!(expected.canonical_name(), canonical);
        }
        assert_eq!(ArgusMethod::from_wire_name("argus.unknown"), None);
    }

    fn capture_registry() -> WindowSnapshotRegistry {
        let snapshot = crate::accessibility::UiTreeSnapshot {
            root: crate::accessibility::UiTreeNode {
                id: "root".into(), author_id: None, node_id: 1, role: "Window".into(),
                label: None, value: None, disabled: false, actions: Vec::new(),
                bounds: None, children: Vec::new(),
            },
            captured_at_utc: "0Z".into(), viewport: None, widget_count: 1,
        };
        let registry = WindowSnapshotRegistry::new(Arc::new(Mutex::new(snapshot)),
            Arc::new(Mutex::new(super::super::action::ActionChannel::new())));
        registry.note_root_publication_locked(None);
        registry
    }

    #[test]
    fn capture_ticket_allows_next_publication_but_rejects_initial_stale_revision() {
        let registry = capture_registry();
        let window = registry.get(MAIN_WINDOW_ID).unwrap();
        let before = window.revision();
        let ticket = registry.admit_capture(&window, before).unwrap();
        registry.note_root_publication_locked(None);
        assert_eq!(ticket.validate().unwrap(), before + 1);
        assert!(registry.admit_capture(&window, before).is_err());
    }

    #[test]
    fn capture_ticket_rejects_closing_and_reregistered_instances() {
        let registry = capture_registry();
        let root = registry.get(MAIN_WINDOW_ID).unwrap();
        let root_ticket = registry.admit_capture(&root, root.revision()).unwrap();
        registry.begin_close(MAIN_WINDOW_ID);
        assert!(root_ticket.validate().is_err());

        let descriptor = ArgusWindowDescriptor {
            window_id: "child".into(), viewport_id: egui::ViewportId::from_hash_of("child"),
            title: "Child".into(),
        };
        registry.register(descriptor.clone()).unwrap();
        let child = registry.get("child").unwrap();
        registry.publish_revision(&child);
        let ticket = registry.admit_capture(&child, child.revision()).unwrap();
        registry.unregister("child");
        registry.register(descriptor).unwrap();
        let replacement = registry.get("child").unwrap();
        registry.publish_revision(&replacement);
        assert_ne!(child.epoch, replacement.epoch);
        assert!(ticket.validate().is_err());
        assert!(registry.admit_capture(&replacement, replacement.revision()).is_ok());
    }

    #[test]
    fn privacy_retirement_blocks_old_publication_and_cannot_be_revived_by_republish() {
        let registry = capture_registry();
        let window = registry.get(MAIN_WINDOW_ID).unwrap();
        let old_revision = window.revision();
        let ticket = registry.admit_capture(&window, old_revision).unwrap();
        registry.retire_capture_context();
        assert!(window.live.load(Ordering::Acquire), "privacy retirement is distinct from window closure");
        assert!(registry.admit_capture(&window, old_revision).is_err(),
            "a fresh generation cannot admit the previous private publication");
        registry.note_root_publication_locked(None);
        let current = registry.get(MAIN_WINDOW_ID).unwrap();
        assert!(Arc::ptr_eq(&current.live, &window.live));
        assert!(ticket.validate().is_err(), "same-instance republish must not revive the retired ticket");
        assert!(registry.admit_capture(&current, current.revision()).is_ok());
    }


    #[test]
    fn capture_ticket_observes_shared_account_revocation_without_ui_reset() {
        let registry = capture_registry();
        let account = crate::local_account::mock_account_context("http://127.0.0.1:9911");
        registry.bind_capture_owner(Some(account.clone()), "workspace");
        registry.note_root_publication_locked(None);
        let window = registry.get(MAIN_WINDOW_ID).unwrap();
        let ticket = registry.admit_capture(&window, window.revision()).unwrap();
        account.invalidate();
        registry.note_root_publication_locked(None);
        assert!(ticket.validate().is_err(), "the actual shared account guard invalidates an in-flight capture");
        assert!(registry.admit_capture(&window, window.revision()).is_err(),
            "ordinary republishing cannot authorize a revoked account");
    }

    #[test]
    fn workspace_owner_change_unpublishes_and_retires_same_window_capture() {
        let registry = capture_registry();
        let account = crate::local_account::mock_account_context("http://127.0.0.1:9911");
        registry.bind_capture_owner(Some(account.clone()), "workspace-a");
        registry.note_root_publication_locked(None);
        let window = registry.get(MAIN_WINDOW_ID).unwrap();
        let ticket = registry.admit_capture(&window, window.revision()).unwrap();
        registry.bind_capture_owner(Some(account), "workspace-b");
        assert!(registry.admit_capture(&window, window.revision()).is_err());
        registry.note_root_publication_locked(None);
        assert!(ticket.validate().is_err(), "same-window republish cannot cross the real workspace owner");
        assert!(registry.admit_capture(&window, window.revision()).is_ok());
    }


    #[test]
    fn account_guard_replacement_retires_unrevoked_same_identity_ticket() {
        let registry = capture_registry();
        let first = crate::local_account::mock_account_context("http://127.0.0.1:9911");
        registry.bind_capture_owner(Some(first.clone()), "workspace");
        registry.note_root_publication_locked(None);
        let window = registry.get(MAIN_WINDOW_ID).unwrap();
        let ticket = registry.admit_capture(&window, window.revision()).unwrap();
        let replacement = crate::local_account::mock_account_context("http://127.0.0.1:9911");
        assert_eq!(first.account_id, replacement.account_id);
        registry.bind_capture_owner(Some(replacement), "workspace");
        registry.note_root_publication_locked(None);
        assert!(first.is_active(), "the old guard need not be revoked for owner replacement to retire it");
        assert!(ticket.validate().is_err());
        assert!(registry.admit_capture(&window, window.revision()).is_ok());
    }

    #[tokio::test]
    async fn activation_readback_requires_admitted_owner_and_retires_original_tickets() {
        let registry = capture_registry();
        let window = registry.get(MAIN_WINDOW_ID).unwrap();
        let ticket = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
        assert_eq!(ticket.fetch().await["status"], "unavailable");
        let account = crate::local_account::mock_account_context("http://127.0.0.1:9911");
        let operation = uuid::Uuid::new_v4();
        assert!(!registry.bind_workspace_activation_readback(account.clone(), "workspace", operation, 1,
            "http://127.0.0.1:9911"));
        registry.bind_capture_owner(Some(account.clone()), "workspace");
        registry.note_root_publication_locked(None);
        assert!(!registry.bind_workspace_activation_readback(account.clone(), "other-workspace", operation, 1,
            "http://127.0.0.1:9911"));
        assert!(!registry.bind_workspace_activation_readback(account.clone(), "workspace", uuid::Uuid::nil(), 1,
            "http://127.0.0.1:9911"));
        assert!(registry.bind_workspace_activation_readback(account.clone(), "workspace", operation, 1,
            "http://127.0.0.1:9911"));
        let ticket = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
        assert!(ticket.is_current());
        assert!(registry.bind_workspace_activation_readback(account.clone(), "workspace", operation, 1,
            "http://127.0.0.1:9911"));
        assert!(ticket.is_current(), "rebinding the identical original does not invalidate a live read");
        registry.note_root_publication_locked(None);
        assert!(ticket.is_current(), "ordinary frames preserve the exact original read authority");
        registry.clear_workspace_activation_readback();
        assert_eq!(ticket.fetch().await["status"], "stale");
        assert!(registry.bind_workspace_activation_readback(account.clone(), "workspace", operation, 1,
            "http://127.0.0.1:9911"));
        assert!(!ticket.is_current(), "clear then rebind cannot revive the previous ticket");
        let replaced = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
        assert!(registry.bind_workspace_activation_readback(account.clone(), "workspace", uuid::Uuid::new_v4(), 2,
            "http://127.0.0.1:9911"));
        assert!(!replaced.is_current(), "a new original cannot revive an in-flight read");
        let reset = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
        registry.retire_capture_context();
        registry.note_root_publication_locked(None);
        assert!(!reset.is_current());
        assert!(registry.capture_context.lock().unwrap().activation_readback.is_none());
        assert!(registry.bind_workspace_activation_readback(account.clone(), "workspace", operation, 3,
            "http://127.0.0.1:9911"));
        let revoked = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
        account.invalidate();
        assert_eq!(revoked.fetch().await["status"], "stale");
    }

    #[test]
    fn activation_readback_rejects_same_id_new_guard_workspace_and_closing_window() {
        let registry = capture_registry();
        let first = crate::local_account::mock_account_context("http://127.0.0.1:9911");
        registry.bind_capture_owner(Some(first.clone()), "workspace");
        registry.note_root_publication_locked(None);
        let window = registry.get(MAIN_WINDOW_ID).unwrap();
        assert!(registry.bind_workspace_activation_readback(first.clone(), "workspace", uuid::Uuid::new_v4(), 1,
            "http://127.0.0.1:9911"));
        let ticket = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
        let replacement = crate::local_account::mock_account_context("http://127.0.0.1:9911");
        assert_eq!(first.account_id, replacement.account_id);
        registry.bind_capture_owner(Some(replacement.clone()), "workspace");
        registry.note_root_publication_locked(None);
        assert!(!ticket.is_current());
        assert!(registry.bind_workspace_activation_readback(replacement.clone(), "workspace", uuid::Uuid::new_v4(), 2,
            "http://127.0.0.1:9911"));
        let ticket = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
        registry.bind_capture_owner(Some(replacement.clone()), "workspace-2");
        registry.note_root_publication_locked(None);
        assert!(!ticket.is_current());
        assert!(registry.bind_workspace_activation_readback(replacement, "workspace-2", uuid::Uuid::new_v4(), 3,
            "http://127.0.0.1:9911"));
        let ticket = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
        registry.begin_close(MAIN_WINDOW_ID);
        assert!(!ticket.is_current());
    }

    async fn serve_readback_reply(
        listener: tokio::net::TcpListener, operation: uuid::Uuid, body: String,
        received: tokio::sync::oneshot::Sender<()>, release: tokio::sync::oneshot::Receiver<()>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(2), listener.accept()).await.unwrap().unwrap();
        let mut request = Vec::new();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 8192);
                let byte = stream.read_u8().await.unwrap();
                request.push(byte);
            }
        }).await.unwrap();
        let request = String::from_utf8(request).unwrap();
        assert!(request.starts_with(&format!("GET /authority/workspaces/workspace/activation/{operation} HTTP/1.1\r\n")));
        assert!(request.contains(&format!("x-hsk-session-token: {}\r\n", "a".repeat(64))));
        assert!(request.contains(&format!("x-hsk-channel-binding-token: {}\r\n", "b".repeat(64))));
        received.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), release).await.unwrap().unwrap();
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        tokio::time::timeout(Duration::from_secs(2), stream.write_all(response.as_bytes())).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn activation_readback_fetches_original_once_and_revalidates_after_await() {
        for (retire_while_pending, mismatched_original) in [(false, false), (true, false), (false, true)] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let account = crate::local_account::mock_account_context(&base);
            let registry = capture_registry();
            registry.bind_capture_owner(Some(account.clone()), "workspace");
            registry.note_root_publication_locked(None);
            let operation = uuid::Uuid::new_v4();
            assert!(registry.bind_workspace_activation_readback(account.clone(), "workspace", operation, 7, &base));
            let window = registry.get(MAIN_WINDOW_ID).unwrap();
            let ticket = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
            let body = serde_json::json!({"schema_id":"hsk.workspace_activation.observation@1",
                "operation_id":if mismatched_original { uuid::Uuid::new_v4() } else { operation },"owner_account_id":account.account_id,
                "actor_principal_id":account.principal_id,"authenticated_session_id":account.session_id,
                "access_space_id":account.access_space_id,"workspace_id":"workspace",
                "readiness":{"status":"unavailable"},"outcome":{"status":"unresolved"}}).to_string();
            let (received_tx, received_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = tokio::sync::oneshot::channel();
            let server = tokio::spawn(serve_readback_reply(listener, operation, body, received_tx, release_rx));
            let fetch = tokio::spawn(async move { ticket.fetch().await });
            tokio::time::timeout(Duration::from_secs(2), received_rx).await.unwrap().unwrap();
            let concurrent = registry.admit_workspace_activation_readback(&window, window.revision()).unwrap();
            assert_eq!(concurrent.fetch().await["status"], "busy");
            if retire_while_pending { registry.retire_capture_context(); }
            release_tx.send(()).unwrap();
            let value = tokio::time::timeout(Duration::from_secs(2), fetch).await.unwrap().unwrap();
            if retire_while_pending {
                assert_eq!(value, serde_json::json!({"status":"stale"}));
            } else if mismatched_original {
                assert_eq!(value["status"], "error");
                assert_eq!(value["reason"], "invalid_observation");
                assert!(value.get("expected").is_none() && value.get("observation").is_none());
            } else {
                assert_eq!(value["status"], "observed");
                assert_eq!(value["source"], "fresh_canonical_get");
                assert_eq!(value["cached"], false);
                assert_eq!(value["expected"]["operation_id"], operation.to_string());
                assert_eq!(value["observation"]["outcome"]["status"], "unresolved",
                    "a canonical read must not project applied success");
            }
            let encoded = value.to_string();
            assert!(!encoded.contains(&"a".repeat(64)) && !encoded.contains(&"b".repeat(64)));
            tokio::time::timeout(Duration::from_secs(2), server).await.unwrap().unwrap();
        }
    }

}
