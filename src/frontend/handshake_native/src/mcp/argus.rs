//! Canonical Argus method names and their compatibility aliases.
//!
//! Product-facing clients use the four `argus.*` names. The pre-Argus MCP spellings remain accepted
//! at the wire boundary so older clients keep working, but callers inside the product can normalize a
//! request once and use the canonical method throughout leasing, dispatch, attribution, and manuals.

use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Weak};
use std::time::{Duration, Instant};

pub const ACTION_RECEIPT_TIMEOUT: Duration = Duration::from_secs(10);
pub const ACTION_RECEIPT_DURABILITY_TIMEOUT: Duration = Duration::from_secs(5);
pub const ACTION_RECEIPT_TRANSPORT_MARGIN: Duration = Duration::from_secs(2);
pub const ACTION_RECEIPT_TRANSPORT_TIMEOUT: Duration = Duration::from_secs(20);

/// A handler signal owned by one existing ActionChannel transaction. Context data stores weak
/// reservations only; it never owns receipts or terminalizes them.
#[derive(Debug)]
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
}

#[derive(Debug, Clone)]
struct ActionEffectReservation {
    ticket: Weak<ActionEffectTicket>,
    author_id: String,
    dispatched_frame: u64,
}

impl ActionEffectTicket {
    fn is_live(&self) -> bool {
        self.live.load(Ordering::Acquire) && Instant::now() < self.deadline
    }

    pub(crate) fn release(&self) {
        self.live.store(false, Ordering::Release);
        self.acknowledged.store(false, Ordering::Release);
    }

    pub(crate) fn acknowledged_after_frame(&self, ctx: &egui::Context, receipt_id: u64) -> bool {
        self.receipt_id == receipt_id
            && self.context == *ctx
            && self.viewport == egui::ViewportId::ROOT
            && ctx.viewport_id() == self.viewport
            && self.is_live()
            && self.acknowledged.load(Ordering::Acquire)
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
    });
    ctx.data_mut(|data| {
        let id = egui::Id::new("argus.action-effect-reservations");
        let mut reservations = data.get_temp::<Vec<ActionEffectReservation>>(id).unwrap_or_default();
        // A released weak handle is unroutable, but its current-frame tombstone prevents the
        // identical old AccessKit event from being attributed to a new receipt/channel epoch.
        reservations.retain(|entry| entry.dispatched_frame == ticket.dispatched_frame
            || entry.ticket.upgrade().is_some_and(|ticket| ticket.is_live()));
        if reservations.len() >= 256 || reservations.iter().any(|entry| {
            entry.author_id == author_id
        }) {
            return None;
        }
        reservations.push(ActionEffectReservation {
            ticket: Arc::downgrade(&ticket),
            author_id: author_id.to_owned(),
            dispatched_frame: ticket.dispatched_frame,
        });
        data.insert_temp(id, reservations);
        Some(ticket)
    })
}

/// Signal the exact dispatched request only after the named production handler applies its effect.
/// A signal remains nonterminal until the channel observes a later completed live frame.
pub fn acknowledge_action_effect(ctx: &egui::Context, author_id: &str) {
    let viewport = ctx.viewport_id();
    if viewport != egui::ViewportId::ROOT {
        return;
    }
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

/// Canonical read-the-live-AccessKit-tree method.
pub const ARGUS_INSPECT_METHOD: &str = "argus.inspect";
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
