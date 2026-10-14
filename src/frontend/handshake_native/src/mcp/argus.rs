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

    pub(crate) fn release(&self) {
        self.live.store(false, Ordering::Release);
        self.acknowledged.store(false, Ordering::Release);
        self.secret_mutation.store(false, Ordering::Release);
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
            ticket.acknowledged.store(true, Ordering::Release);
        }
    }
}

pub(crate) fn acknowledge_secret_set_value(
    ctx: &egui::Context, author_id: &str, widget_id: egui::Id, owned_value: &str,
) {
    if !crate::accessibility::is_sensitive_author_id(author_id)
        || !matches!(author_id, "settings.cloud.byok.anthropic.key" | "settings.cloud.byok.openai.key") {
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
    pub(crate) fn validate(&self, revision: u64) -> Result<(), String> {
        if !self.live.load(Ordering::Acquire) || self.closing.load(Ordering::Acquire) {
            return Err("Argus window is closed or closing".to_owned());
        }
        if !self.published.load(Ordering::Acquire) {
            return Err("Argus window has no published live snapshot".to_owned());
        }
        if revision != self.revision() { return Err("Argus window revision is stale".to_owned()); }
        Ok(())
    }
}

#[derive(Clone)]
pub struct WindowSnapshotRegistry {
    inner: Arc<Mutex<WindowRegistryState>>,
    next_revision: Arc<std::sync::atomic::AtomicU64>,
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
        Self { inner: Arc::new(Mutex::new(WindowRegistryState { windows: std::collections::HashMap::from([(MAIN_WINDOW_ID.to_owned(), root)]), next_epoch: 2 })), next_revision: Arc::new(std::sync::atomic::AtomicU64::new(1)) }
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
        if production_ctx.is_some_and(|ctx| ctx.viewport_id() != egui::ViewportId::ROOT) { return; }
        if let Ok(root) = self.get(MAIN_WINDOW_ID) {
            if let Ok(revision) = self.next_revision.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1)) {
                root.revision.store(revision, Ordering::Release);
                root.published.store(true, Ordering::Release);
            } else { root.live.store(false, Ordering::Release); }
        }
    }

    fn publish_revision(&self, window: &RegisteredWindow) -> bool {
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
}
