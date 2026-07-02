//! Daily journal panel — the editors <-> Calendar (Pillar 2) host surface (WP-KERNEL-012 MT-067, E10).
//!
//! ## What this is (the native port of `app/src/components/LoomDailyJournalPanel.tsx`)
//!
//! A native egui panel (no web view) that hosts the daily note view and its Calendar interop chrome:
//!
//! - a **date header with prev/next-day nav** — REUSES the MT-019 [`crate::rich_editor::daily_notes::date_nav::DateNavWidget`]
//!   (it does NOT build a second calendar/date-picker, RISK-4/MC-4),
//! - the linked **CalendarEvent chip** — clickable; a click emits [`DailyJournalEvent::FocusCalendarEvent`]
//!   (the host dispatches the [`crate::interop::CMD_FOCUS_CALENDAR_EVENT`] bus command targeting MT-030's
//!   calendar pane, bus-only — no calendar-pane internal import), and
//! - a read-only **"Edited during this block" correlation strip** — lists the
//!   [`crate::interop::ActivitySpan::edited_doc_ids`] as READ-ONLY document chips (RISK-5/MC-5: clicking a
//!   chip emits [`DailyJournalEvent::OpenDocument`] navigation ONLY — the panel never writes ActivitySpan
//!   data).
//!
//! ## Typed-blocker empty-states (the designed primary path in this build)
//!
//! Because handshake_core has NO `/calendar/` routes (VERIFIED — Calendar = Pillar 2), the
//! [`crate::interop::CalendarInteropService`] event + activity reads return
//! [`crate::interop::InteropError::EndpointUnavailable`]. This panel renders the typed empty-states for
//! both the CalendarEvent chip ([`crate::interop::InteropError::EVENT_UNAVAILABLE_MSG`]) and the activity
//! strip ([`crate::interop::InteropError::ACTIVITY_UNAVAILABLE_MSG`]) WHILE the daily-note header + nav stay
//! fully functional — the panel never dies on the absent calendar routes (AC-4). It never fabricates an
//! event or a span.
//!
//! ## AccessKit (HBR-SWARM) — the contract-named author_ids
//!
//! Each control carries a stable AccessKit author_id so a swarm agent drives the panel by id:
//! - [`DAILY_JOURNAL_PANEL_AUTHOR_ID`] (`daily-journal-panel`, `Role::GenericContainer`) — the outer container,
//! - [`DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID`] (`daily-journal-date-header`, `Role::Label`) — the date heading,
//! - the prev/next-day buttons are the MT-019 `journal-prev-day` / `journal-next-day` ids (the date nav is
//!   REUSED, so the contract's `daily-journal-prev-day`/`daily-journal-next-day` map to the existing nav),
//! - [`DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID`] (`daily-journal-calendar-event-chip`, `Role::Button`) —
//!   present only when an event resolves (else the unavailable empty-state),
//! - [`DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID`] (`daily-journal-activity-strip`, `Role::List`) — the strip, and
//! - per item [`activity_item_author_id`] (`daily-journal-activity-item-{doc_id}`, `Role::Button`).
//!
//! ## No hardcoded color (theme reuse)
//!
//! Every color is a [`crate::theme::HsPalette`] semantic token — NO `Color32` literal (the no-hardcode
//! invariant), so the panel tracks dark/light like every other surface.

use egui::accesskit;

use crate::accessibility;
use crate::interop::{ActivitySpan, CalendarEvent, DocId, InteropError};
use crate::rich_editor::daily_notes::date_nav::{DateNav, DateNavOutcome, DateNavWidget};
use crate::theme::HsPalette;

/// The outer panel container author_id (`Role::GenericContainer`).
pub const DAILY_JOURNAL_PANEL_AUTHOR_ID: &str = "daily-journal-panel";
/// The date-header label author_id (`Role::Label`).
pub const DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID: &str = "daily-journal-date-header";
/// The linked-CalendarEvent chip author_id (`Role::Button`; present only when an event resolves).
pub const DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID: &str = "daily-journal-calendar-event-chip";
/// The read-only activity correlation strip author_id (`Role::List`).
pub const DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID: &str = "daily-journal-activity-strip";
/// The per-item read-only document chip author_id PREFIX (`daily-journal-activity-item-{doc_id}`).
pub const DAILY_JOURNAL_ACTIVITY_ITEM_AUTHOR_ID_PREFIX: &str = "daily-journal-activity-item-";

/// The stable AccessKit author_id for one read-only activity document chip
/// (`daily-journal-activity-item-{doc_id}`). The `doc_id` is sanitized to `[a-z0-9-]` (the same
/// [`crate::project_tree::stable_part`] slug the canvas/loom ids use) so an arbitrary doc id yields a
/// safe, collision-resistant address a swarm agent can drive.
pub fn activity_item_author_id(doc_id: &DocId) -> String {
    format!(
        "{DAILY_JOURNAL_ACTIVITY_ITEM_AUTHOR_ID_PREFIX}{}",
        crate::project_tree::stable_part(doc_id.as_str())
    )
}

/// The read-only correlation state the panel renders for the activity strip: either the spans (read-only),
/// or the typed-blocker empty-state, or "not yet resolved". The panel NEVER holds a mutation path on this
/// (RISK-5/MC-5) — it is a render-only view of what the interop read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityCorrelation {
    /// No event resolved yet, so there is nothing to correlate (the chip is unavailable too).
    NoEvent,
    /// The spans were read (read-only). May be empty (an event with no edits).
    Spans(Vec<ActivitySpan>),
    /// The typed blocker: the `/calendar/activity-spans` route is absent — show the empty-state.
    Unavailable,
}

/// The state the daily journal panel renders (set by the host from the [`crate::interop::CalendarInteropService`]
/// reads). The panel is a pure VIEW over this — it performs NO IO and holds NO mutation path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyJournalState {
    /// The date navigation (REUSED MT-019 [`DateNav`] — prev/next/today + calendar popup).
    pub nav: DateNav,
    /// The resolved CalendarEvent for the date, when one resolves. `None` => render the unavailable chip
    /// empty-state (the typed-blocker reality in this build).
    pub event: Option<CalendarEvent>,
    /// Whether the CalendarEvent read hit the typed blocker (so the chip shows the unavailable empty-state
    /// rather than a blank). Distinct from `event: None` with no blocker (an event simply not found).
    pub event_unavailable: bool,
    /// The read-only activity correlation for the resolved event.
    pub activity: ActivityCorrelation,
}

impl DailyJournalState {
    /// Build a state for `nav` with no event resolved yet (the initial state before the interop reads).
    pub fn new(nav: DateNav) -> Self {
        Self {
            nav,
            event: None,
            event_unavailable: false,
            activity: ActivityCorrelation::NoEvent,
        }
    }

    /// Apply the typed-blocker outcome for both calendar reads (the designed primary path in this build):
    /// the CalendarEvent chip + the activity strip both show their unavailable empty-states, while the
    /// daily-note header + nav stay functional.
    pub fn set_calendar_unavailable(&mut self) {
        self.event = None;
        self.event_unavailable = true;
        self.activity = ActivityCorrelation::Unavailable;
    }

    /// Apply a resolved event + its read-only spans (the live path once the `/calendar/` routes land).
    pub fn set_event_with_spans(&mut self, event: CalendarEvent, spans: Vec<ActivitySpan>) {
        self.event = Some(event);
        self.event_unavailable = false;
        self.activity = ActivityCorrelation::Spans(spans);
    }
}

/// One typed outcome of a panel frame: the host drains it to dispatch the matching WP-011 command-bus
/// command (the panel never imports calendar-pane internals — bus-only, RISK-4/MC-4). `None` means no
/// action this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DailyJournalEvent {
    /// No action this frame.
    None,
    /// The displayed date changed (prev/next/today/calendar) — the host emits
    /// [`crate::interop::CMD_OPEN_DAILY_NOTE_FOR_DATE`] to open-or-create that date's note (and re-resolve
    /// the event). Carries the new date.
    DateNavigated(chrono::NaiveDate),
    /// The CalendarEvent chip was clicked — the host emits
    /// [`crate::interop::CMD_FOCUS_CALENDAR_EVENT`] targeting MT-030's calendar pane. Carries the event id.
    FocusCalendarEvent(String),
    /// A read-only activity document chip was clicked — the host emits
    /// [`crate::interop::calendar_interop::CMD_OPEN_DOCUMENT`] navigation (RISK-5/MC-5: navigation only,
    /// never a write).
    /// Carries the doc id.
    OpenDocument(DocId),
}

/// The daily journal panel (the contract's `DailyJournalPanel`). A thin egui VIEW over a
/// [`DailyJournalState`] — it renders the date header + nav (reused MT-019), the linked-event chip, and the
/// read-only activity strip, and returns the [`DailyJournalEvent`] the host drains to the command bus. It
/// holds NO IO and NO mutation path on calendar/activity data.
pub struct DailyJournalPanel;

impl DailyJournalPanel {
    /// Render the daily journal panel into `ui`, returning the action the host should route to the command
    /// bus. Emits the contract's AccessKit nodes (HBR-SWARM). NO network/disk IO happens here (render is
    /// pure); the host owns the [`crate::interop::CalendarInteropService`] reads.
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut DailyJournalState,
        palette: &HsPalette,
    ) -> DailyJournalEvent {
        let mut event = DailyJournalEvent::None;

        let container_id = egui::Id::new(DAILY_JOURNAL_PANEL_AUTHOR_ID);
        let resp = ui
            .scope_builder(egui::UiBuilder::new().id_salt(container_id), |ui| {
                // ── Date header (Role::Label) + prev/next-day nav (REUSED MT-019 DateNavWidget) ──────────
                let header_text = state.nav.current_display();
                let header_resp = ui.add(egui::Label::new(
                    egui::RichText::new(&header_text)
                        .color(palette.text)
                        .strong()
                        .heading(),
                ));
                let header_author = DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID.to_owned();
                let header_value = header_text.clone();
                ui.ctx()
                    .accesskit_node_builder(header_resp.id, move |node| {
                        node.set_role(accesskit::Role::Label);
                        node.set_author_id(header_author.clone());
                        node.set_label("Daily journal date".to_owned());
                        node.set_value(header_value.clone());
                    });

                // The MT-019 date nav (prev/next/today + calendar popup) — NOT a second date-picker.
                let nav_outcome = DateNavWidget::new(&mut state.nav, palette).show(ui);
                if let DateNavOutcome::Navigated(date) = nav_outcome {
                    event = DailyJournalEvent::DateNavigated(date);
                }

                ui.separator();

                // ── Linked CalendarEvent chip (Role::Button) — present only when an event resolves ───────
                if let Some(ev) = &state.event {
                    let chip_label = if ev.title.trim().is_empty() {
                        format!("Calendar event {}", ev.id)
                    } else {
                        ev.title.clone()
                    };
                    let chip = egui::Button::new(
                        egui::RichText::new(format!("📅 {chip_label}")).color(palette.accent),
                    );
                    let chip_resp = ui.add(chip);
                    accessibility::emit_interactive_node(
                        ui.ctx(),
                        chip_resp.id,
                        DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID,
                    );
                    if chip_resp.clicked() {
                        event = DailyJournalEvent::FocusCalendarEvent(ev.id.clone());
                    }
                } else if state.event_unavailable {
                    // The typed-blocker empty-state for the absent /calendar/events route (AC-4). The
                    // daily-note header + nav above stay functional — the panel does not die here.
                    ui.label(
                        egui::RichText::new(InteropError::EVENT_UNAVAILABLE_MSG)
                            .color(palette.text_subtle)
                            .italics(),
                    );
                }

                ui.add_space(4.0);

                // ── Read-only "Edited during this block" activity correlation strip (Role::List) ─────────
                let strip_resp = ui
                    .scope_builder(
                        egui::UiBuilder::new()
                            .id_salt(egui::Id::new(DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID)),
                        |ui| {
                            ui.label(
                                egui::RichText::new("Edited during this block")
                                    .color(palette.text_subtle)
                                    .small(),
                            );
                            match &state.activity {
                                ActivityCorrelation::Spans(spans) => {
                                    let doc_ids: Vec<DocId> = collect_edited_doc_ids(spans);
                                    if doc_ids.is_empty() {
                                        ui.label(
                                            egui::RichText::new(
                                                "No documents edited during this block",
                                            )
                                            .color(palette.text_subtle)
                                            .italics(),
                                        );
                                    } else {
                                        ui.horizontal_wrapped(|ui| {
                                            for doc_id in &doc_ids {
                                                // A read-only doc chip: clicking it NAVIGATES (RISK-5/MC-5)
                                                // — it never writes ActivitySpan data.
                                                let chip = egui::Button::new(
                                                    egui::RichText::new(doc_id.as_str())
                                                        .color(palette.text),
                                                );
                                                let chip_resp = ui.add(chip);
                                                accessibility::emit_interactive_node(
                                                    ui.ctx(),
                                                    chip_resp.id,
                                                    &activity_item_author_id(doc_id),
                                                );
                                                if chip_resp.clicked() {
                                                    event = DailyJournalEvent::OpenDocument(
                                                        doc_id.clone(),
                                                    );
                                                }
                                            }
                                        });
                                    }
                                }
                                ActivityCorrelation::Unavailable => {
                                    // The typed-blocker empty-state for the absent /calendar/activity-spans
                                    // route (AC-3 / AC-4). Never fabricates a span.
                                    ui.label(
                                        egui::RichText::new(InteropError::ACTIVITY_UNAVAILABLE_MSG)
                                            .color(palette.text_subtle)
                                            .italics(),
                                    );
                                }
                                ActivityCorrelation::NoEvent => {
                                    ui.label(
                                        egui::RichText::new(
                                            "No calendar event linked for this date",
                                        )
                                        .color(palette.text_subtle)
                                        .italics(),
                                    );
                                }
                            }
                        },
                    )
                    .response;
                let strip_author = DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID.to_owned();
                ui.ctx().accesskit_node_builder(strip_resp.id, move |node| {
                    node.set_role(accesskit::Role::List);
                    node.set_author_id(strip_author.clone());
                    node.set_label("Edited during this block".to_owned());
                });
            })
            .response;

        // The outer container node (Role::GenericContainer) — the MT-067 swarm address.
        let author = DAILY_JOURNAL_PANEL_AUTHOR_ID.to_owned();
        let value = state.nav.current_display();
        ui.ctx().accesskit_node_builder(resp.id, move |node| {
            node.set_role(accesskit::Role::GenericContainer);
            node.set_author_id(author.clone());
            node.set_label("Daily journal".to_owned());
            node.set_value(value.clone());
        });

        event
    }
}

/// Collect the read-only edited-document ids across the spans, de-duplicated, preserving first-seen order
/// (the read-only correlation strip's chip list). Pure — no mutation of span data (RISK-5/MC-5).
pub fn collect_edited_doc_ids(spans: &[ActivitySpan]) -> Vec<DocId> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for span in spans {
        for doc_id in &span.edited_doc_ids {
            if seen.insert(doc_id.clone()) {
                out.push(doc_id.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn nav(date: NaiveDate) -> DateNav {
        DateNav::new(date, date)
    }

    fn span(id: &str, docs: &[&str]) -> ActivitySpan {
        ActivitySpan {
            span_id: id.to_owned(),
            calendar_event_id: Some("E-1".to_owned()),
            started_utc: Utc::now(),
            ended_utc: Utc::now(),
            edited_doc_ids: docs.iter().map(|s| DocId((*s).to_owned())).collect(),
        }
    }

    #[test]
    fn activity_item_author_id_is_sanitized_and_prefixed() {
        let id = activity_item_author_id(&DocId("Doc With Spaces/42".to_owned()));
        assert!(id.starts_with(DAILY_JOURNAL_ACTIVITY_ITEM_AUTHOR_ID_PREFIX));
        assert!(!id.contains(' '), "the doc id is slugged");
        assert!(!id.contains('/'));
    }

    #[test]
    fn collect_edited_doc_ids_dedups_preserving_order() {
        let spans = vec![span("S1", &["A", "B"]), span("S2", &["B", "C"])];
        let docs = collect_edited_doc_ids(&spans);
        assert_eq!(
            docs,
            vec![DocId("A".into()), DocId("B".into()), DocId("C".into())],
            "deduped, first-seen order"
        );
    }

    #[test]
    fn set_calendar_unavailable_sets_both_empty_states() {
        let mut state = DailyJournalState::new(nav(d(2026, 6, 21)));
        state.set_calendar_unavailable();
        assert!(state.event.is_none());
        assert!(
            state.event_unavailable,
            "the chip shows the unavailable empty-state"
        );
        assert_eq!(state.activity, ActivityCorrelation::Unavailable);
    }

    #[test]
    fn set_event_with_spans_holds_read_only_view() {
        let mut state = DailyJournalState::new(nav(d(2026, 6, 21)));
        let ev = CalendarEvent {
            id: "E-1".into(),
            title: "Block".into(),
            start_utc: Utc::now(),
            end_utc: Utc::now(),
            all_day: false,
            daily_note_doc_id: None,
        };
        state.set_event_with_spans(ev.clone(), vec![span("S1", &["DOC-A"])]);
        assert_eq!(state.event.as_ref().unwrap().id, "E-1");
        assert!(!state.event_unavailable);
        match &state.activity {
            ActivityCorrelation::Spans(s) => assert_eq!(s.len(), 1),
            other => panic!("expected Spans, got {other:?}"),
        }
    }
}
