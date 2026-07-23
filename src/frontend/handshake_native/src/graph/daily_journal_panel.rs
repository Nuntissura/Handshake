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
//! ## Typed-blocker empty-states
//!
//! The live Calendar routes normally populate the event chip and activity strip. If either route is
//! unavailable, [`crate::interop::CalendarInteropService`] returns
//! [`crate::interop::InteropError::EndpointUnavailable`], and this panel renders typed empty-states for
//! both the CalendarEvent chip ([`crate::interop::InteropError::EVENT_UNAVAILABLE_MSG`]) and the activity
//! strip ([`crate::interop::InteropError::ACTIVITY_UNAVAILABLE_MSG`]) while the daily-note header + nav stay
//! functional (AC-4). It never fabricates an event or a span.
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
use crate::rich_editor::daily_notes::date_nav::{
    DateNav, DateNavOutcome, DateNavWidget, DAILY_JOURNAL_DATE_NAV_AUTHOR_IDS,
};
use crate::theme::HsPalette;

/// The outer panel container author_id (`Role::GenericContainer`).
pub const DAILY_JOURNAL_PANEL_AUTHOR_ID: &str = "daily-journal-panel";
/// The date-header label author_id (`Role::Label`).
pub const DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID: &str = "daily-journal-date-header";
/// The linked-CalendarEvent chip author_id (`Role::Button`; present only when an event resolves).
pub const DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID: &str = "daily-journal-calendar-event-chip";
pub const DAILY_JOURNAL_NORMALIZATION_BADGE_AUTHOR_ID: &str =
    "daily-journal-calendar-normalization-badge";
pub const DAILY_JOURNAL_LEGACY_BADGE_AUTHOR_ID: &str = "daily-journal-calendar-legacy-badge";
/// The read-only activity correlation strip author_id (`Role::List`).
pub const DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID: &str = "daily-journal-activity-strip";
/// The per-item read-only document chip author_id PREFIX (`daily-journal-activity-item-{doc_id}`).
pub const DAILY_JOURNAL_ACTIVITY_ITEM_AUTHOR_ID_PREFIX: &str = "daily-journal-activity-item-";
pub const CALENDAR_EVENT_PANE_AUTHOR_ID: &str = "calendar-event-pane";
pub const CALENDAR_EVENT_DETAILS_TAB_AUTHOR_ID: &str = "calendar-event-tab-details";
pub const CALENDAR_EVENT_NOTES_TAB_AUTHOR_ID: &str = "calendar-event-tab-notes";
pub const CALENDAR_EVENT_ACTIVITY_TAB_AUTHOR_ID: &str = "calendar-event-tab-activity";
pub const CALENDAR_EVENT_DETAILS_AUTHOR_ID: &str = "calendar-event-details";
pub const CALENDAR_EVENT_NOTES_AUTHOR_ID: &str = "calendar-event-notes";
pub const CALENDAR_EVENT_ACTIVITY_AUTHOR_ID: &str = "calendar-event-activity";
pub const CALENDAR_EVENT_NORMALIZATION_BADGE_AUTHOR_ID: &str = "calendar-event-normalization-badge";
pub const CALENDAR_EVENT_LEGACY_BADGE_AUTHOR_ID: &str = "calendar-event-legacy-badge";

fn emit_status_badge(ui: &mut egui::Ui, text: &str, author_id: &'static str, palette: &HsPalette) {
    let response = ui.label(
        egui::RichText::new(text)
            .small()
            .strong()
            .color(palette.accent),
    );
    ui.ctx().accesskit_node_builder(response.id, move |node| {
        node.set_role(accesskit::Role::Label);
        node.set_author_id(author_id.to_owned());
        node.set_label(text.to_owned());
    });
}

pub fn calendar_event_span_author_id(span_id: &str) -> String {
    format!(
        "calendar-event-span-{}",
        crate::project_tree::stable_part(span_id)
    )
}

pub fn calendar_event_primary_doc_author_id(doc_id: &DocId) -> String {
    format!(
        "calendar-event-primary-doc-{}",
        crate::project_tree::stable_part(doc_id.as_str())
    )
}

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
    /// The typed failure: the `/calendar/activity-spans` read failed — show the exact activity-only
    /// recovery state without changing the already-resolved calendar event.
    Failed(CalendarReadFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarProjectionState {
    Idle,
    WaitingForDailyNote,
    DailyNoteError,
    Loading,
    NoEvent,
    Event,
    Failed(CalendarReadFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarReadFailure {
    EndpointUnavailable,
    RetryExhausted,
    InvalidResponse,
    RequestFailed,
}

impl CalendarReadFailure {
    pub fn event_message(&self) -> &'static str {
        match self {
            Self::EndpointUnavailable => InteropError::EVENT_UNAVAILABLE_MSG,
            Self::RetryExhausted => "Calendar event request failed after bounded retries",
            Self::InvalidResponse => "Calendar event response was invalid",
            Self::RequestFailed => "Calendar event request failed",
        }
    }

    pub fn activity_message(&self) -> &'static str {
        match self {
            Self::EndpointUnavailable => InteropError::ACTIVITY_UNAVAILABLE_MSG,
            Self::RetryExhausted => "Activity correlation request failed after bounded retries",
            Self::InvalidResponse => "Activity correlation response was invalid",
            Self::RequestFailed => "Activity correlation request failed",
        }
    }
}

/// The state the daily journal panel renders (set by the host from the [`crate::interop::CalendarInteropService`]
/// reads). The panel is a pure VIEW over this — it performs NO IO and holds NO mutation path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyJournalState {
    /// The date navigation (REUSED MT-019 [`DateNav`] — prev/next/today + calendar popup).
    pub nav: DateNav,
    /// The resolved CalendarEvent for the date, when one resolves. `None` means either no event exists
    /// for the selected day or the typed unavailable state is active (distinguished below).
    pub event: Option<CalendarEvent>,
    /// One closed projection state. Contradictory combinations such as loading + failed or no-event +
    /// unavailable cannot be represented.
    pub projection: CalendarProjectionState,
    /// The read-only activity correlation for the resolved event.
    pub activity: ActivityCorrelation,
}

impl DailyJournalState {
    /// Build a state for `nav` with no event resolved yet (the initial state before the interop reads).
    pub fn new(nav: DateNav) -> Self {
        Self {
            nav,
            event: None,
            projection: CalendarProjectionState::Idle,
            activity: ActivityCorrelation::NoEvent,
        }
    }

    /// Select `date` and clear Calendar-derived rows from the previously selected day. The host calls
    /// this before issuing the new `(workspace, date)` request, so rapid navigation never displays a
    /// previous day's event or ActivitySpans while the current request is in flight.
    pub fn prepare_date(&mut self, date: chrono::NaiveDate) {
        self.nav.navigate_to(date);
        self.event = None;
        self.projection = CalendarProjectionState::WaitingForDailyNote;
        self.activity = ActivityCorrelation::NoEvent;
    }

    pub fn begin_calendar_load(&mut self, date: chrono::NaiveDate) {
        self.nav.navigate_to(date);
        self.event = None;
        self.projection = CalendarProjectionState::Loading;
        self.activity = ActivityCorrelation::NoEvent;
    }

    pub fn set_daily_note_error(&mut self) {
        self.event = None;
        self.projection = CalendarProjectionState::DailyNoteError;
        self.activity = ActivityCorrelation::NoEvent;
    }

    /// Apply a successful empty event list for the selected day. This is terminal success, not a
    /// failure and not an in-flight state.
    pub fn set_no_event(&mut self) {
        self.event = None;
        self.projection = CalendarProjectionState::NoEvent;
        self.activity = ActivityCorrelation::NoEvent;
    }

    /// Apply the typed-blocker outcome for the CalendarEvent read: the event chip + activity strip both
    /// show unavailable empty-states, while the daily-note header + nav stay functional.
    pub fn set_calendar_unavailable(&mut self) {
        self.set_calendar_failure(CalendarReadFailure::EndpointUnavailable);
    }

    pub fn set_calendar_failure(&mut self, failure: CalendarReadFailure) {
        self.event = None;
        self.projection = CalendarProjectionState::Failed(failure.clone());
        self.activity = ActivityCorrelation::Failed(failure);
    }

    /// Apply an ActivitySpan-only typed blocker without discarding the already resolved CalendarEvent.
    /// The event-to-daily-note binding and its navigation chip remain usable while only the correlation
    /// strip reports that activity data is unavailable.
    pub fn set_activity_unavailable(&mut self, event: CalendarEvent) {
        self.set_activity_failure(event, CalendarReadFailure::EndpointUnavailable);
    }

    pub fn set_activity_failure(&mut self, event: CalendarEvent, failure: CalendarReadFailure) {
        self.event = Some(event);
        self.projection = CalendarProjectionState::Event;
        self.activity = ActivityCorrelation::Failed(failure);
    }

    /// Apply a resolved event + its read-only spans from the live `/calendar/` routes.
    pub fn set_event_with_spans(&mut self, event: CalendarEvent, spans: Vec<ActivitySpan>) {
        self.event = Some(event);
        self.projection = CalendarProjectionState::Event;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalendarEventDetailTab {
    #[default]
    Details,
    Notes,
    Activity,
}

/// Content-addressed CalendarEvent destination. It renders the exact event already resolved by the
/// journal loader and never substitutes a different event when the requested id is unavailable.
pub struct CalendarEventDetailPanel;

impl CalendarEventDetailPanel {
    pub fn show(
        ui: &mut egui::Ui,
        event_id: &str,
        state: &DailyJournalState,
        active_tab: &mut CalendarEventDetailTab,
        palette: &HsPalette,
    ) -> DailyJournalEvent {
        let mut outcome = DailyJournalEvent::None;
        let response = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt(egui::Id::new(CALENDAR_EVENT_PANE_AUTHOR_ID)),
                |ui| {
                    ui.label(
                        egui::RichText::new("Calendar Event")
                            .heading()
                            .strong()
                            .color(palette.text),
                    );
                    ui.horizontal(|ui| {
                        for (tab, label, author_id) in [
                            (
                                CalendarEventDetailTab::Details,
                                "Details",
                                CALENDAR_EVENT_DETAILS_TAB_AUTHOR_ID,
                            ),
                            (
                                CalendarEventDetailTab::Notes,
                                "Notes",
                                CALENDAR_EVENT_NOTES_TAB_AUTHOR_ID,
                            ),
                            (
                                CalendarEventDetailTab::Activity,
                                "Activity",
                                CALENDAR_EVENT_ACTIVITY_TAB_AUTHOR_ID,
                            ),
                        ] {
                            let button = ui.selectable_label(*active_tab == tab, label);
                            accessibility::emit_interactive_node(ui.ctx(), button.id, author_id);
                            if button.clicked() {
                                *active_tab = tab;
                            }
                        }
                    });
                    ui.separator();

                    let Some(event) = state.event.as_ref().filter(|event| event.id == event_id)
                    else {
                        ui.label(
                            egui::RichText::new(format!(
                                "Calendar event details unavailable for {event_id}"
                            ))
                            .color(palette.text_subtle)
                            .italics(),
                        );
                        return;
                    };

                    match *active_tab {
                        CalendarEventDetailTab::Details => {
                            if event.is_legacy_incomplete() {
                                emit_status_badge(
                                    ui,
                                    "Legacy temporal data — reimport required",
                                    CALENDAR_EVENT_LEGACY_BADGE_AUTHOR_ID,
                                    palette,
                                );
                            } else if event.has_dst_normalization() {
                                emit_status_badge(
                                    ui,
                                    "DST overlap normalized",
                                    CALENDAR_EVENT_NORMALIZATION_BADGE_AUTHOR_ID,
                                    palette,
                                );
                            }
                            let body = format!(
                                "{}\nEvent ID: {}\n{}",
                                event.title,
                                event.id,
                                event.temporal_summary()
                            );
                            let details = ui.label(egui::RichText::new(&body).color(palette.text));
                            let value = body.clone();
                            ui.ctx().accesskit_node_builder(details.id, move |node| {
                                node.set_role(accesskit::Role::GenericContainer);
                                node.set_author_id(CALENDAR_EVENT_DETAILS_AUTHOR_ID.to_owned());
                                node.set_label("Calendar event details".to_owned());
                                node.set_value(value.clone());
                            });
                        }
                        CalendarEventDetailTab::Notes => {
                            let notes = ui
                                .scope_builder(
                                    egui::UiBuilder::new()
                                        .id_salt(egui::Id::new(CALENDAR_EVENT_NOTES_AUTHOR_ID)),
                                    |ui| match &event.daily_note_doc_id {
                                        Some(doc_id) => {
                                            ui.label(
                                                egui::RichText::new("Primary document")
                                                    .color(palette.text_subtle),
                                            );
                                            let button = ui.button(doc_id.as_str());
                                            accessibility::emit_interactive_node(
                                                ui.ctx(),
                                                button.id,
                                                &calendar_event_primary_doc_author_id(doc_id),
                                            );
                                            if button.clicked() {
                                                outcome =
                                                    DailyJournalEvent::OpenDocument(doc_id.clone());
                                            }
                                        }
                                        None => {
                                            ui.label(
                                                egui::RichText::new("No primary document linked")
                                                    .color(palette.text_subtle)
                                                    .italics(),
                                            );
                                        }
                                    },
                                )
                                .response;
                            let doc_value = event
                                .daily_note_doc_id
                                .as_ref()
                                .map(|id| id.as_str().to_owned())
                                .unwrap_or_default();
                            ui.ctx().accesskit_node_builder(notes.id, move |node| {
                                node.set_role(accesskit::Role::GenericContainer);
                                node.set_author_id(CALENDAR_EVENT_NOTES_AUTHOR_ID.to_owned());
                                node.set_label("Calendar event notes".to_owned());
                                node.set_value(doc_value.clone());
                            });
                        }
                        CalendarEventDetailTab::Activity => {
                            let activity = ui
                                .scope_builder(
                                    egui::UiBuilder::new()
                                        .id_salt(egui::Id::new(CALENDAR_EVENT_ACTIVITY_AUTHOR_ID)),
                                    |ui| match &state.activity {
                                        ActivityCorrelation::Spans(spans) if spans.is_empty() => {
                                            ui.label(
                                                egui::RichText::new("No activity correlated")
                                                    .color(palette.text_subtle)
                                                    .italics(),
                                            );
                                        }
                                        ActivityCorrelation::Spans(spans) => {
                                            for span in spans.iter().filter(|span| {
                                                span.calendar_event_id.as_deref() == Some(event_id)
                                            }) {
                                                let span_value = format!(
                                                    "{} — {}",
                                                    span.started_utc.to_rfc3339(),
                                                    span.ended_utc
                                                        .map(|ended| ended.to_rfc3339())
                                                        .unwrap_or_else(|| "In progress".to_owned())
                                                );
                                                let span_row = ui.label(
                                                    egui::RichText::new(format!(
                                                        "{}  {}",
                                                        span.span_id, span_value
                                                    ))
                                                    .color(palette.text),
                                                );
                                                let span_author =
                                                    calendar_event_span_author_id(&span.span_id);
                                                let span_label = span.span_id.clone();
                                                ui.ctx().accesskit_node_builder(
                                                    span_row.id,
                                                    move |node| {
                                                        node.set_role(accesskit::Role::ListItem);
                                                        node.set_author_id(span_author.clone());
                                                        node.set_label(span_label.clone());
                                                        node.set_value(span_value.clone());
                                                    },
                                                );
                                                ui.indent(&span.span_id, |ui| {
                                                    for doc_id in &span.edited_doc_ids {
                                                        let button = ui.button(doc_id.as_str());
                                                        accessibility::emit_interactive_node(
                                                            ui.ctx(),
                                                            button.id,
                                                            &activity_item_author_id(doc_id),
                                                        );
                                                        if button.clicked() {
                                                            outcome =
                                                                DailyJournalEvent::OpenDocument(
                                                                    doc_id.clone(),
                                                                );
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        ActivityCorrelation::Failed(failure) => {
                                            ui.label(
                                                egui::RichText::new(failure.activity_message())
                                                    .color(palette.text_subtle)
                                                    .italics(),
                                            );
                                        }
                                        ActivityCorrelation::NoEvent => {
                                            ui.label(
                                                egui::RichText::new("No activity correlated")
                                                    .color(palette.text_subtle)
                                                    .italics(),
                                            );
                                        }
                                    },
                                )
                                .response;
                            ui.ctx().accesskit_node_builder(activity.id, move |node| {
                                node.set_role(accesskit::Role::List);
                                node.set_author_id(CALENDAR_EVENT_ACTIVITY_AUTHOR_ID.to_owned());
                                node.set_label("Calendar event activity".to_owned());
                            });
                        }
                    }
                },
            )
            .response;
        let value = event_id.to_owned();
        ui.ctx().accesskit_node_builder(response.id, move |node| {
            node.set_role(accesskit::Role::GenericContainer);
            node.set_author_id(CALENDAR_EVENT_PANE_AUTHOR_ID.to_owned());
            node.set_label("Calendar event".to_owned());
            node.set_value(value.clone());
        });
        outcome
    }
}

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
                let nav_outcome = DateNavWidget::new(&mut state.nav, palette)
                    .with_author_ids(DAILY_JOURNAL_DATE_NAV_AUTHOR_IDS)
                    .show(ui);
                if let DateNavOutcome::Navigated(date) = nav_outcome {
                    event = DailyJournalEvent::DateNavigated(date);
                }

                ui.separator();

                // ── Linked CalendarEvent chip (Role::Button) — present only when an event resolves ───────
                if state.projection == CalendarProjectionState::WaitingForDailyNote {
                    ui.label(
                        egui::RichText::new("Waiting for daily note…")
                            .color(palette.text_subtle)
                            .italics(),
                    );
                } else if state.projection == CalendarProjectionState::DailyNoteError {
                    ui.label(
                        egui::RichText::new(
                            "Calendar binding paused; see the daily-note error below",
                        )
                        .color(palette.text_subtle)
                        .italics(),
                    );
                } else if state.projection == CalendarProjectionState::Loading {
                    ui.label(
                        egui::RichText::new("Loading calendar event and activity…")
                            .color(palette.text_subtle)
                            .italics(),
                    );
                } else if let Some(ev) = &state.event {
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
                    if ev.is_legacy_incomplete() {
                        emit_status_badge(
                            ui,
                            "Legacy temporal data — reimport required",
                            DAILY_JOURNAL_LEGACY_BADGE_AUTHOR_ID,
                            palette,
                        );
                    } else if ev.has_dst_normalization() {
                        emit_status_badge(
                            ui,
                            "DST overlap normalized",
                            DAILY_JOURNAL_NORMALIZATION_BADGE_AUTHOR_ID,
                            palette,
                        );
                    }
                } else if let CalendarProjectionState::Failed(failure) = &state.projection {
                    // The typed-blocker empty-state for an unavailable /calendar/events read (AC-4).
                    // The daily-note header + nav above stay functional — the panel does not die here.
                    ui.label(
                        egui::RichText::new(failure.event_message())
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
                            if state.projection == CalendarProjectionState::WaitingForDailyNote {
                                ui.label(
                                    egui::RichText::new("Waiting for daily note…")
                                        .color(palette.text_subtle)
                                        .italics(),
                                );
                                return;
                            }
                            if state.projection == CalendarProjectionState::DailyNoteError {
                                ui.label(
                                    egui::RichText::new(
                                        "Activity correlation paused until the daily note opens",
                                    )
                                    .color(palette.text_subtle)
                                    .italics(),
                                );
                                return;
                            }
                            if state.projection == CalendarProjectionState::Loading {
                                ui.label(
                                    egui::RichText::new("Loading activity correlation…")
                                        .color(palette.text_subtle)
                                        .italics(),
                                );
                                return;
                            }
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
                                ActivityCorrelation::Failed(failure) => {
                                    // Typed empty-state for an unavailable /calendar/activity-spans read
                                    // (AC-3 / AC-4). Never fabricates a span.
                                    ui.label(
                                        egui::RichText::new(failure.activity_message())
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
    use crate::interop::calendar_interop::CalendarEventTemporal;
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
            ended_utc: Some(Utc::now()),
            edited_doc_ids: docs.iter().map(|s| DocId((*s).to_owned())).collect(),
        }
    }

    fn timed_event(daily_note_doc_id: Option<DocId>) -> CalendarEvent {
        let now = Utc::now();
        CalendarEvent {
            id: "E-1".into(),
            title: "Block".into(),
            temporal: CalendarEventTemporal::Timed {
                start_utc: now,
                end_utc: now + chrono::Duration::hours(1),
                start_local: now.naive_utc().to_string(),
                end_local: (now + chrono::Duration::hours(1)).naive_utc().to_string(),
                tzid: "UTC".into(),
                was_floating: false,
                normalization_note: None,
            },
            daily_note_doc_id,
            view_tzid: "UTC".into(),
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
        assert_eq!(
            state.projection,
            CalendarProjectionState::Failed(CalendarReadFailure::EndpointUnavailable)
        );
        assert_eq!(
            state.activity,
            ActivityCorrelation::Failed(CalendarReadFailure::EndpointUnavailable)
        );
    }

    #[test]
    fn set_activity_unavailable_preserves_resolved_event() {
        let mut state = DailyJournalState::new(nav(d(2026, 6, 21)));
        let ev = timed_event(Some(DocId("DOC-2026-06-21".into())));
        state.set_activity_unavailable(ev.clone());
        assert_eq!(state.event.as_ref(), Some(&ev));
        assert_eq!(state.projection, CalendarProjectionState::Event);
        assert_eq!(
            state.activity,
            ActivityCorrelation::Failed(CalendarReadFailure::EndpointUnavailable)
        );
    }

    #[test]
    fn set_event_with_spans_holds_read_only_view() {
        let mut state = DailyJournalState::new(nav(d(2026, 6, 21)));
        let ev = timed_event(None);
        state.set_event_with_spans(ev.clone(), vec![span("S1", &["DOC-A"])]);
        assert_eq!(state.event.as_ref().unwrap().id, "E-1");
        assert_eq!(state.projection, CalendarProjectionState::Event);
        match &state.activity {
            ActivityCorrelation::Spans(s) => assert_eq!(s.len(), 1),
            other => panic!("expected Spans, got {other:?}"),
        }
    }

    #[test]
    fn successful_empty_event_list_is_terminal_no_event_not_loading() {
        let mut state = DailyJournalState::new(nav(d(2026, 6, 21)));
        state.prepare_date(d(2026, 6, 21));
        assert_eq!(
            state.projection,
            CalendarProjectionState::WaitingForDailyNote
        );
        state.begin_calendar_load(d(2026, 6, 21));
        assert_eq!(state.projection, CalendarProjectionState::Loading);
        state.set_no_event();
        assert_eq!(state.projection, CalendarProjectionState::NoEvent);
        assert!(state.event.is_none());
        assert_eq!(state.activity, ActivityCorrelation::NoEvent);
    }
}
