//! The Relevant Memory side panel (WP-KERNEL-012 MT-063, cluster E9 — FEMS interop).
//!
//! ## What this is (the provenance-first inline memory surface)
//!
//! [`RelevantMemoryPanel`] is an egui side-panel widget that CONSUMES the FEMS retrieval capsule
//! ([`crate::fems::memory_client::MemoryPack`]) keyed on the active document/selection context and
//! renders it provenance-first: items are grouped by [`MemoryKind`] (Episodic / Semantic / Procedural
//! section headers) and each item row shows its summary, a kind badge, and a "Go to source" affordance
//! that — for a navigable item — builds a navigation target from the item's
//! [`crate::fems::memory_client::MemorySource`] and routes it through the shell navigation bus (the
//! MT-030 nav seam reused over the WP-011 `command_registry` + `event_bus`, NOT a new channel). An item
//! whose source does not resolve renders with a DISABLED source link rather than a dead/clickable one
//! (RISK-003/MC-003).
//!
//! ## The navigation seam (MT-030, reused — not the MT-037 knowledge-docs client)
//!
//! The MT-063 contract literally names "the MT-037 navigation bus / `nav_bus.navigate_to(target)`".
//! The KERNEL_BUILDER gate corrected this: MT-037 is the knowledge-documents client; the navigation
//! seam is the MT-030 [`crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget`] /
//! `on_open: FnMut(NavTarget)` pattern the Outgoing Links pane established. This module keeps the
//! contract's `nav_bus.navigate_to(target)` SHAPE — [`NavigationBus`] is a tiny trait with a
//! `navigate_to` method — while binding it to the established MT-030 `FnMut` seam via
//! [`FnNavigationBus`], so the host wires the real bus closure at E11 (MT-069) exactly like every other
//! pane. NO new navigation channel/sender/registry is created here (RISK-006-style fork avoidance).
//!
//! ## Empty-state / typed-blocker (RISK-005/MC-002, AC-005)
//!
//! When the panel holds the typed blocker [`MemoryClientError::EndpointMissing`] (the DESIGNED primary
//! path in this build — the FEMS read route is absent), it renders a calm empty-state banner
//! ("Relevant Memory unavailable — FEMS read endpoint not present in this build") instead of the list,
//! and exposes the blocker via [`RelevantMemoryPanel::blocker`] / [`RelevantMemoryPanel::take_blocker`]
//! so the host surfaces it upward to the WP validator. It NEVER panics and NEVER silently no-ops. When
//! the pack is present but has no items, it renders a neutral "No relevant memory for this context".
//!
//! ## AccessKit (HBR-SWARM, AC-007)
//!
//! - [`RELEVANT_MEMORY_PANEL_AUTHOR_ID`] (`relevant-memory-panel`, `Role::GenericContainer`) on the
//!   outer frame.
//! - [`RELEVANT_MEMORY_LIST_AUTHOR_ID`] (`relevant-memory-list`, `Role::List`) on the items container.
//! - each item row -> `mem-item-{id}` ([`mem_item_author_id`], `Role::ListItem`).
//! - each source link -> `mem-source-{id}` ([`mem_source_author_id`], `Role::Button`).
//!
//! Item ids are SANITIZED + DEDUPED per pack so a duplicate/odd server id cannot collide AccessKit
//! addresses (RISK-007/MC-006). These per-item ids live in egui's hashed id space (the MT-007 tab-node
//! precedent) — they are NOT enumerated in the fixed `DECLARED_IDENTITIES` band.

use egui::accesskit;

use crate::accessibility::emit_interactive_node;
use crate::fems::memory_client::{
    MemoryClientError, MemoryContext, MemoryItem, MemoryKind, MemoryPack, MemorySource,
};
use crate::project_tree::stable_part;
use crate::theme::HsPalette;

/// AccessKit author_id for the outer panel frame (`Role::GenericContainer`).
pub const RELEVANT_MEMORY_PANEL_AUTHOR_ID: &str = "relevant-memory-panel";

/// AccessKit author_id for the items container (`Role::List`).
pub const RELEVANT_MEMORY_LIST_AUTHOR_ID: &str = "relevant-memory-list";

/// AccessKit author_id PREFIX for one item row (`mem-item-{id}`, `Role::ListItem`).
pub const MEM_ITEM_AUTHOR_PREFIX: &str = "mem-item-";

/// AccessKit author_id PREFIX for one item's source link (`mem-source-{id}`, `Role::Button`).
pub const MEM_SOURCE_AUTHOR_PREFIX: &str = "mem-source-";

/// The calm empty-state banner shown when the FEMS read endpoint is absent (the typed blocker).
pub const ENDPOINT_MISSING_BANNER: &str =
    "Relevant Memory unavailable — FEMS read endpoint not present in this build";

/// The neutral state shown when a pack is present but has no items for the current context.
pub const NO_MEMORY_TEXT: &str = "No relevant memory for this context";

/// Build the stable AccessKit author_id for an item row (`mem-item-{id}`). The id is sanitized to
/// `[a-z0-9-]` via the same [`stable_part`] slug the pane/canvas ids use, so an arbitrary server id
/// yields a safe, collision-resistant address (RISK-007/MC-006).
pub fn mem_item_author_id(item_id: &str) -> String {
    format!("{MEM_ITEM_AUTHOR_PREFIX}{}", stable_part(item_id))
}

/// Build the stable AccessKit author_id for an item's source link (`mem-source-{id}`).
pub fn mem_source_author_id(item_id: &str) -> String {
    format!("{MEM_SOURCE_AUTHOR_PREFIX}{}", stable_part(item_id))
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Navigation target + bus seam (MT-030 reuse).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The navigation target a "Go to source" click resolves to, built from a [`MemorySource`] with the
/// Pillar 12 precedence: prefer `uri`, else `document_id` (+ optional `byte_range`), else `event_id`. A
/// source with none of these is NON-NAVIGABLE — [`Self::from_source`] returns `None` and the row's
/// source link is rendered disabled (RISK-003/MC-003), so a dead link can never be clicked or panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryNavTarget {
    /// Open a resolvable URI (`loom://...` / `atelier://...`) — the highest-precedence target.
    Uri { uri: String },
    /// Open a document, optionally scrolled to a byte range.
    Document {
        document_id: String,
        byte_range: Option<(usize, usize)>,
    },
    /// Open / reveal an EventLedger event by id.
    Event { event_id: String },
}

impl MemoryNavTarget {
    /// Build the navigation target from a [`MemorySource`] using the Pillar 12 precedence. Returns
    /// `None` when the source has no resolvable field (a non-navigable item — RISK-003/MC-003). This is
    /// the SAFE construction: it never panics and never produces a dead link.
    pub fn from_source(source: &MemorySource) -> Option<Self> {
        if let Some(uri) = &source.uri {
            return Some(MemoryNavTarget::Uri { uri: uri.clone() });
        }
        if let Some(document_id) = &source.document_id {
            return Some(MemoryNavTarget::Document {
                document_id: document_id.clone(),
                byte_range: source.byte_range,
            });
        }
        if let Some(event_id) = &source.event_id {
            return Some(MemoryNavTarget::Event {
                event_id: event_id.clone(),
            });
        }
        None
    }

    /// A short human/agent-readable description of the target (used for the AccessKit value + tooltip).
    pub fn describe(&self) -> String {
        match self {
            MemoryNavTarget::Uri { uri } => uri.clone(),
            MemoryNavTarget::Document { document_id, byte_range } => match byte_range {
                Some((s, e)) => format!("{document_id} [{s}..{e}]"),
                None => document_id.clone(),
            },
            MemoryNavTarget::Event { event_id } => format!("event:{event_id}"),
        }
    }
}

/// The navigation bus seam the panel routes a "Go to source" click through. Mirrors the MT-063
/// contract's `nav_bus.navigate_to(target)` shape while staying decoupled from the concrete shell bus —
/// the host wires the real MT-030 navigation closure at E11 (MT-069). Use [`FnNavigationBus`] to adapt
/// the established `FnMut(MemoryNavTarget)` seam (the same `on_open` pattern the Outgoing Links pane
/// uses) without forking a new navigation channel.
pub trait NavigationBus {
    /// Route a resolved navigation target to the shell navigation seam.
    fn navigate_to(&mut self, target: MemoryNavTarget);
}

/// Adapter that turns any `FnMut(MemoryNavTarget)` into a [`NavigationBus`], so the host can wire the
/// established MT-030 `on_open` closure (the Outgoing Links pane pattern) with no new channel. This is
/// the bridge that keeps the contract's `nav_bus.navigate_to(target)` shape and the MT-030 `FnMut` seam
/// as the SAME path.
pub struct FnNavigationBus<F: FnMut(MemoryNavTarget)>(pub F);

impl<F: FnMut(MemoryNavTarget)> NavigationBus for FnNavigationBus<F> {
    fn navigate_to(&mut self, target: MemoryNavTarget) {
        (self.0)(target);
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The panel widget.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The Relevant Memory side panel widget. Held by the host (in `app.rs`). Its `current` pack /
/// `blocker` are filled OFF the render path by the host (after [`crate::fems::memory_client::MemoryClient::fetch_pack`]
/// resolves on the shared async runtime); [`Self::show`] renders that CACHED state ONLY and performs NO
/// IO (so a render frame can never block on the network).
#[derive(Debug, Clone, Default)]
pub struct RelevantMemoryPanel {
    /// The last context a fetch was requested for. The debounce anchor: [`Self::refresh_for_context`]
    /// skips a refresh when the new context equals this (RISK-004/MC-004).
    last_context: Option<MemoryContext>,
    /// The capsule currently displayed (filled by the host after a fetch resolves).
    current: Option<MemoryPack>,
    /// True while a fetch is in flight (renders a spinner).
    in_flight: bool,
    /// The typed blocker, if the last fetch failed. `EndpointMissing` drives the empty-state banner and
    /// is surfaced upward (RISK-005/MC-002, AC-005).
    blocker: Option<MemoryClientError>,
}

impl RelevantMemoryPanel {
    /// A fresh, empty panel.
    pub fn new() -> Self {
        Self::default()
    }

    /// The last context a fetch was requested for (the debounce anchor).
    pub fn last_context(&self) -> Option<&MemoryContext> {
        self.last_context.as_ref()
    }

    /// The capsule currently displayed, if any.
    pub fn current(&self) -> Option<&MemoryPack> {
        self.current.as_ref()
    }

    /// True while a fetch is in flight.
    pub fn in_flight(&self) -> bool {
        self.in_flight
    }

    /// The current typed blocker, if any (read-only peek; the host uses [`Self::take_blocker`] to
    /// surface it once to the validator).
    pub fn blocker(&self) -> Option<&MemoryClientError> {
        self.blocker.as_ref()
    }

    /// True when the panel holds the `EndpointMissing` typed blocker (the empty-state banner is shown
    /// and the blocker must be surfaced to the WP validator). Convenience for the host's handoff path.
    pub fn has_endpoint_missing_blocker(&self) -> bool {
        self.blocker
            .as_ref()
            .map(MemoryClientError::is_endpoint_missing)
            .unwrap_or(false)
    }

    /// Take the typed blocker out (so the host surfaces it upward exactly once). Leaves the panel in its
    /// rendered state — the banner stays because the empty-state render keys off `current.is_none()`
    /// plus a sticky banner flag is not needed; the host re-sets the blocker if a later fetch fails
    /// again.
    pub fn take_blocker(&mut self) -> Option<MemoryClientError> {
        self.blocker.take()
    }

    /// Debounced trigger: request a new pack ONLY when `ctx` differs from the last requested context
    /// (RISK-004/MC-004). Returns `true` if a refresh should fire (the host then calls
    /// [`crate::fems::memory_client::MemoryClient::fetch_pack`] off the render path and feeds the result
    /// back via [`Self::set_pack`] / [`Self::set_blocker`]); returns `false` when the context is
    /// unchanged (the refresh is skipped — no redundant endpoint traffic). On a fire, it records the new
    /// context and marks `in_flight`.
    pub fn refresh_for_context(&mut self, ctx: MemoryContext) -> bool {
        if self.last_context.as_ref() == Some(&ctx) {
            return false;
        }
        self.last_context = Some(ctx);
        self.in_flight = true;
        true
    }

    /// Host hook: install a successfully-fetched pack (clears the blocker + in-flight).
    pub fn set_pack(&mut self, pack: MemoryPack) {
        self.current = Some(pack);
        self.blocker = None;
        self.in_flight = false;
    }

    /// Host hook: install a typed blocker from a failed fetch (clears in-flight; the pack is cleared so
    /// the empty-state / banner shows for `EndpointMissing`).
    pub fn set_blocker(&mut self, err: MemoryClientError) {
        self.blocker = Some(err);
        self.current = None;
        self.in_flight = false;
    }

    /// Render the panel into `ui`. `nav_bus` is the navigation seam the host wires to the MT-030 nav bus
    /// (via [`FnNavigationBus`]). Renders the CACHED state ONLY — NO network/disk IO here. Always emits
    /// the outer `relevant-memory-panel` (`GenericContainer`) AccessKit node so a swarm agent can find
    /// the panel by stable id even in the empty-state / blocker / in-flight states (AC-007).
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette, nav_bus: &mut dyn NavigationBus) {
        let panel_id = egui::Id::new(RELEVANT_MEMORY_PANEL_AUTHOR_ID);
        let resp = ui
            .scope_builder(egui::UiBuilder::new().id_salt(panel_id), |ui| {
                ui.label(egui::RichText::new("Relevant Memory").strong().color(palette.text));
                ui.separator();
                self.show_body(ui, palette, nav_bus);
            })
            .response;
        // The outer container node (Role::GenericContainer) — owns role + label + author_id.
        let author = RELEVANT_MEMORY_PANEL_AUTHOR_ID.to_owned();
        ui.ctx().accesskit_node_builder(resp.id, move |node| {
            node.set_role(accesskit::Role::GenericContainer);
            node.set_author_id(author.clone());
            node.set_label("Relevant Memory".to_owned());
        });
    }

    /// Render the panel body (the state machine: blocker banner / spinner / empty / list).
    fn show_body(&mut self, ui: &mut egui::Ui, palette: &HsPalette, nav_bus: &mut dyn NavigationBus) {
        // 1) Typed blocker: the EndpointMissing empty-state banner (RISK-005/MC-002, AC-005). Other
        //    blocker variants render their typed message (still no panic, still no silent no-op).
        if let Some(err) = &self.blocker {
            if err.is_endpoint_missing() {
                ui.colored_label(palette.text_subtle, ENDPOINT_MISSING_BANNER);
            } else {
                ui.colored_label(palette.error_text, format!("Relevant Memory error: {err}"));
            }
            return;
        }

        // 2) In-flight: a spinner (no perpetual-spinner risk — the host clears in_flight on resolve).
        if self.in_flight && self.current.is_none() {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new());
                ui.colored_label(palette.text_subtle, "Loading relevant memory…");
            });
            return;
        }

        // 3) A pack is present.
        let Some(pack) = &self.current else {
            // No fetch has resolved yet and nothing is in flight: a neutral idle hint (not an error).
            ui.colored_label(palette.text_subtle, "Place the caret in a document to surface memory.");
            return;
        };

        if pack.items.is_empty() {
            // Neutral empty state (a valid pack with no items for this context).
            ui.colored_label(palette.text_subtle, NO_MEMORY_TEXT);
            return;
        }

        // Advisory token-budget metadata SURFACED (never recomputed). A subtle over-budget signal.
        if let Some(estimate) = pack.token_estimate {
            let txt = if pack.over_token_budget() {
                format!("~{estimate} tokens (over the 500 budget)")
            } else {
                format!("~{estimate} tokens")
            };
            let color = if pack.over_token_budget() { palette.error_text } else { palette.text_subtle };
            ui.colored_label(color, txt);
        }
        if pack.truncated {
            ui.colored_label(palette.text_subtle, "(truncated to 24 items)");
        }

        // The items container (Role::List) wraps all grouped sections.
        let list_id = egui::Id::new(RELEVANT_MEMORY_LIST_AUTHOR_ID);
        let list_resp = ui
            .scope_builder(egui::UiBuilder::new().id_salt(list_id), |ui| {
                // Dedup item ids per pack so duplicate/odd ids cannot collide AccessKit addresses
                // (RISK-007/MC-006). The first occurrence of an id wins; later duplicates are skipped.
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                for kind in MemoryKind::ORDER {
                    let kind_items: Vec<&MemoryItem> = pack
                        .items_of_kind(kind)
                        .filter(|it| seen.insert(mem_item_author_id(&it.id)))
                        .collect();
                    if kind_items.is_empty() {
                        continue;
                    }
                    let header = format!("{} ({})", kind.section_label(), kind_items.len());
                    egui::CollapsingHeader::new(egui::RichText::new(header).color(palette.text))
                        .id_salt(format!("relevant-memory-section-{}", kind.wire()))
                        .default_open(true)
                        .show(ui, |ui| {
                            for item in kind_items {
                                render_item_row(ui, palette, item, nav_bus);
                            }
                        });
                }
            })
            .response;
        let list_author = RELEVANT_MEMORY_LIST_AUTHOR_ID.to_owned();
        ui.ctx().accesskit_node_builder(list_resp.id, move |node| {
            node.set_role(accesskit::Role::List);
            node.set_author_id(list_author.clone());
            node.set_label("Relevant memory items".to_owned());
        });
    }
}

/// Render one item row: the kind badge, the summary, and a provenance "Go to source" affordance. The
/// source affordance is ALWAYS present (provenance-first); for a navigable item it is an enabled button
/// that, on click, builds a [`MemoryNavTarget`] from the source and routes it through `nav_bus`; for a
/// NON-navigable item (no resolvable source) it is a DISABLED button — never a dead/clickable link
/// (RISK-003/MC-003). Emits the `mem-item-{id}` (`Role::ListItem`) and `mem-source-{id}` (`Role::Button`)
/// AccessKit nodes (AC-007).
fn render_item_row(
    ui: &mut egui::Ui,
    palette: &HsPalette,
    item: &MemoryItem,
    nav_bus: &mut dyn NavigationBus,
) {
    let item_id = egui::Id::new(mem_item_author_id(&item.id));
    let row_resp = ui
        .scope_builder(egui::UiBuilder::new().id_salt(item_id), |ui| {
            ui.horizontal_wrapped(|ui| {
                // Kind badge.
                ui.label(
                    egui::RichText::new(format!("[{}]", item.kind.badge()))
                        .color(palette.accent)
                        .small(),
                );
                // Summary (always shown — provenance-first).
                ui.label(egui::RichText::new(&item.summary).color(palette.text));
                if let Some(score) = item.score {
                    ui.label(
                        egui::RichText::new(format!("{score:.2}"))
                            .color(palette.text_subtle)
                            .small(),
                    );
                }
            });

            // The provenance "Go to source" affordance — ALWAYS present.
            let nav_target = MemoryNavTarget::from_source(&item.source);
            let navigable = nav_target.is_some();
            let btn = egui::Button::new(
                egui::RichText::new("Go to source").color(if navigable {
                    palette.accent
                } else {
                    palette.text_subtle
                }),
            )
            .small();
            // A non-navigable item gets a DISABLED button (no dead link, no panic — RISK-003/MC-003).
            let src_resp = ui.add_enabled(navigable, btn);
            // The source-link AccessKit node (Role::Button) — addressable by mem-source-{id} even when
            // disabled, so an agent can read which sources are non-navigable.
            emit_interactive_node(ui.ctx(), src_resp.id, &mem_source_author_id(&item.id));
            if navigable {
                if let Some(target) = nav_target {
                    let hover = target.describe();
                    let src_resp = src_resp.on_hover_text(hover);
                    if src_resp.clicked() {
                        nav_bus.navigate_to(target);
                    }
                }
            }
        })
        .response;

    // The item-row AccessKit node (Role::ListItem) — owns role + label + author_id.
    let author = mem_item_author_id(&item.id);
    let label = item.summary.clone();
    ui.ctx().accesskit_node_builder(row_resp.id, move |node| {
        node.set_role(accesskit::Role::ListItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fems::memory_client::MemorySource;

    fn src_uri() -> MemorySource {
        MemorySource { uri: Some("loom://block/x".into()), ..Default::default() }
    }
    fn src_doc() -> MemorySource {
        MemorySource { document_id: Some("D9".into()), byte_range: Some((10, 40)), ..Default::default() }
    }
    fn src_evt() -> MemorySource {
        MemorySource { event_id: Some("EV-7".into()), ..Default::default() }
    }

    /// Navigation precedence: uri > document_id+range > event_id; an all-absent source is None.
    #[test]
    fn nav_target_precedence_and_non_navigable() {
        // uri wins even when other fields are present.
        let both = MemorySource {
            uri: Some("loom://u".into()),
            document_id: Some("D".into()),
            event_id: Some("E".into()),
            ..Default::default()
        };
        assert_eq!(MemoryNavTarget::from_source(&both), Some(MemoryNavTarget::Uri { uri: "loom://u".into() }));
        assert_eq!(
            MemoryNavTarget::from_source(&src_doc()),
            Some(MemoryNavTarget::Document { document_id: "D9".into(), byte_range: Some((10, 40)) })
        );
        assert_eq!(
            MemoryNavTarget::from_source(&src_evt()),
            Some(MemoryNavTarget::Event { event_id: "EV-7".into() })
        );
        // Non-navigable -> None (RISK-003/MC-003): no dead link, no panic.
        assert_eq!(MemoryNavTarget::from_source(&MemorySource::default()), None);
    }

    /// Debounce: an unchanged context skips the refresh; a changed one fires it.
    #[test]
    fn refresh_debounces_on_unchanged_context() {
        let mut panel = RelevantMemoryPanel::new();
        let ctx = MemoryContext::from_focus("W", Some("D".into()), None, Some(1));
        assert!(panel.refresh_for_context(ctx.clone()), "first context must fire");
        assert!(panel.in_flight());
        assert!(!panel.refresh_for_context(ctx.clone()), "same context must be skipped (debounce)");
        let ctx2 = MemoryContext::from_focus("W", Some("D".into()), None, Some(2));
        assert!(panel.refresh_for_context(ctx2), "a changed context must fire again");
    }

    /// set_pack / set_blocker drive the state machine (mutually exclusive).
    #[test]
    fn state_machine_pack_vs_blocker() {
        let mut panel = RelevantMemoryPanel::new();
        panel.refresh_for_context(MemoryContext::for_workspace("W"));
        assert!(panel.in_flight());
        panel.set_blocker(MemoryClientError::EndpointMissing { probed_path: "/p".into() });
        assert!(!panel.in_flight(), "blocker clears in_flight");
        assert!(panel.current().is_none(), "blocker clears the pack");
        assert!(panel.has_endpoint_missing_blocker());
        // A later successful fetch clears the blocker.
        panel.set_pack(MemoryPack::empty("k"));
        assert!(panel.blocker().is_none());
        assert!(panel.current().is_some());
    }

    /// take_blocker hands the typed blocker out exactly once.
    #[test]
    fn take_blocker_surfaces_once() {
        let mut panel = RelevantMemoryPanel::new();
        panel.set_blocker(MemoryClientError::EndpointMissing { probed_path: "/p".into() });
        let taken = panel.take_blocker();
        assert!(taken.is_some_and(|e| e.is_endpoint_missing()));
        assert!(panel.take_blocker().is_none(), "second take is None");
    }

    /// Author-id builders sanitize odd ids and stay collision-distinct per id (RISK-007/MC-006).
    #[test]
    fn author_ids_sanitized_and_distinct() {
        let a = mem_item_author_id("item/with weird:chars");
        assert!(a.starts_with("mem-item-"));
        assert!(!a.contains('/') && !a.contains(' ') && !a.contains(':'));
        let s = mem_source_author_id("item/with weird:chars");
        assert!(s.starts_with("mem-source-"));
        // Two different ids produce two different addresses.
        assert_ne!(mem_item_author_id("a"), mem_item_author_id("b"));
    }

    /// FnNavigationBus adapts a closure into the NavigationBus trait (the MT-030 seam bridge).
    #[test]
    fn fn_navigation_bus_routes() {
        let mut captured: Vec<MemoryNavTarget> = Vec::new();
        {
            let mut bus = FnNavigationBus(|t: MemoryNavTarget| captured.push(t));
            bus.navigate_to(MemoryNavTarget::from_source(&src_uri()).unwrap());
            bus.navigate_to(MemoryNavTarget::from_source(&src_doc()).unwrap());
        }
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0], MemoryNavTarget::Uri { uri: "loom://block/x".into() });
    }
}
