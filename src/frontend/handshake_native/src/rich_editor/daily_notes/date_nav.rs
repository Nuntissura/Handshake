//! Date navigation for the daily-notes / journal surface (WP-KERNEL-012 MT-019).
//!
//! [`DateNav`] is a PURE date-arithmetic + calendar-grid state holder (no egui), so prev/next/today
//! and the month grid are FULLY unit-testable with NO GUI. [`DateNavWidget`] is the thin egui view
//! over it (prev/next/today buttons + the current-date display + a calendar popup) that emits the
//! stable AccessKit author_ids the ACs assert.
//!
//! ## Date arithmetic (KERNEL_BUILDER gate)
//!
//! - `chrono::Local::now().date_naive()` for today (`today()` is deprecated in chrono 0.4.23+).
//! - prev/next via `NaiveDate::checked_add_days(Days::new(1))` (MC-005: 2026-01-31 + 1 = 2026-02-01,
//!   leap years handled by chrono — NOT manual day-of-month math).
//!
//! ## Calendar grid (MC-004)
//!
//! The month grid allocates EXACTLY 6 rows × 7 columns regardless of the month's real week count, so
//! the popup never reflows between months (a month starting on Saturday spans 6 rows; February in a
//! common year fits in 4-5 — both render as a fixed 6×7 grid). [`month_grid`] returns the 42 cells
//! (leading/trailing days are `None`) so a kittest can assert the day numbers + the fixed cell count.

use chrono::{Datelike, Days, NaiveDate};

use egui::accesskit;

use crate::accessibility;
use crate::theme::HsPalette;

/// The stable AccessKit author_ids for the date-nav controls (the AC-11 contract ids). Exposed as
/// consts so the panel, the tests, and the User Manual reference the SAME strings.
pub const PREV_DAY_ID: &str = "journal-prev-day";
pub const NEXT_DAY_ID: &str = "journal-next-day";
pub const TODAY_ID: &str = "journal-today";
pub const CALENDAR_TOGGLE_ID: &str = "journal-calendar-toggle";
pub const DATE_DISPLAY_ID: &str = "journal-date-display";

/// The fixed number of calendar grid rows (MC-004: 6 rows regardless of month, so no popup reflow).
pub const CALENDAR_ROWS: usize = 6;
/// The number of calendar grid columns (7 days a week).
pub const CALENDAR_COLS: usize = 7;
/// The total fixed calendar cell count (6 × 7 = 42).
pub const CALENDAR_CELLS: usize = CALENDAR_ROWS * CALENDAR_COLS;

/// The `YYYY-MM-DD` storage format the backend journal endpoint expects.
pub const DATE_STORAGE_FMT: &str = "%Y-%m-%d";

/// The PURE date-navigation state: the currently displayed date, "today" (injectable so a test pins a
/// fixed today), and whether the calendar popup is open. No egui — fully unit-testable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateNav {
    /// The date currently displayed (and opened in the journal).
    pub current: NaiveDate,
    /// "Today" — injected (a fixed mock today in tests; `Local::now().date_naive()` in production).
    pub today: NaiveDate,
    /// Whether the calendar picker popup is open.
    pub calendar_open: bool,
}

impl DateNav {
    /// Build a nav at `current` with the given `today`.
    pub fn new(current: NaiveDate, today: NaiveDate) -> Self {
        Self {
            current,
            today,
            calendar_open: false,
        }
    }

    /// Production constructor: current = today = `chrono::Local::now().date_naive()` (the contract
    /// note — `Local::today()` is deprecated).
    pub fn today_now() -> Self {
        let today = chrono::Local::now().date_naive();
        Self::new(today, today)
    }

    /// The current date as the backend `YYYY-MM-DD` string.
    pub fn current_storage(&self) -> String {
        self.current.format(DATE_STORAGE_FMT).to_string()
    }

    /// The current date as a human-readable header (e.g. "Thursday, June 19, 2026").
    pub fn current_display(&self) -> String {
        self.current.format("%A, %B %-d, %Y").to_string()
    }

    /// Move to the previous day (MC-005: chrono `checked_sub_days`, NOT manual math — wraps month/year/
    /// leap correctly). Returns the new current date.
    pub fn prev_day(&mut self) -> NaiveDate {
        if let Some(d) = self.current.checked_sub_days(Days::new(1)) {
            self.current = d;
        }
        self.current
    }

    /// Move to the next day (MC-005: chrono `checked_add_days`). Returns the new current date.
    pub fn next_day(&mut self) -> NaiveDate {
        if let Some(d) = self.current.checked_add_days(Days::new(1)) {
            self.current = d;
        }
        self.current
    }

    /// Jump to today. Returns the new current date.
    pub fn jump_today(&mut self) -> NaiveDate {
        self.current = self.today;
        self.current
    }

    /// Navigate directly to a chosen date (a calendar-cell click). Closes the popup.
    pub fn navigate_to(&mut self, date: NaiveDate) {
        self.current = date;
        self.calendar_open = false;
    }

    /// True when `date` is the currently displayed date (the calendar highlights it).
    pub fn is_current(&self, date: NaiveDate) -> bool {
        date == self.current
    }

    /// True when `date` is today (the calendar highlights it distinctly from the current date).
    pub fn is_today(&self, date: NaiveDate) -> bool {
        date == self.today
    }

    /// The 6×7 = 42 calendar cells for the MONTH containing `current` (MC-004: a FIXED 6-row grid so
    /// the popup never reflows). Leading cells before the 1st and trailing cells after the last day are
    /// `None`; in-month cells carry their `NaiveDate`. The first column is Sunday (the React/Obsidian
    /// convention). The grid always returns exactly [`CALENDAR_CELLS`] entries.
    pub fn month_grid(&self) -> Vec<Option<NaiveDate>> {
        month_grid(self.current.year(), self.current.month())
    }
}

/// Compute the FIXED 6×7 calendar grid (42 cells) for `(year, month)`. The first cell is the Sunday on
/// or before the 1st of the month; in-month days carry their date, out-of-month leading/trailing cells
/// are `None`. Returns exactly [`CALENDAR_CELLS`] entries (MC-004: no month-dependent reflow).
///
/// Uses `NaiveDate::from_ymd_opt(year, month, 1).weekday()` for the first-day column offset (the
/// contract impl-note), with Sunday as column 0.
pub fn month_grid(year: i32, month: u32) -> Vec<Option<NaiveDate>> {
    let mut cells = Vec::with_capacity(CALENDAR_CELLS);
    let Some(first) = NaiveDate::from_ymd_opt(year, month, 1) else {
        // An invalid (year, month) yields an all-empty fixed grid rather than a panic.
        return vec![None; CALENDAR_CELLS];
    };
    // Sunday = column 0. chrono's `num_days_from_sunday()` gives 0 for Sunday … 6 for Saturday.
    let lead = first.weekday().num_days_from_sunday() as usize;
    // The number of days in this month: the day-before the 1st of the next month.
    let days_in_month = days_in_month(year, month);

    for cell in 0..CALENDAR_CELLS {
        if cell < lead {
            cells.push(None); // leading out-of-month cell.
        } else {
            let day_index = cell - lead; // 0-based day within the month.
            if day_index < days_in_month as usize {
                cells.push(NaiveDate::from_ymd_opt(year, month, day_index as u32 + 1));
            } else {
                cells.push(None); // trailing out-of-month cell.
            }
        }
    }
    debug_assert_eq!(cells.len(), CALENDAR_CELLS);
    cells
}

/// The number of days in `(year, month)`, computed from the first of the NEXT month minus one day
/// (chrono-correct for leap years, NOT a hardcoded `[31, 28, …]` table).
pub fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("valid first-of-month");
    first_next
        .checked_sub_days(Days::new(1))
        .map(|d| d.day())
        .unwrap_or(28)
}

/// The outcome of one [`DateNavWidget`] frame: which navigation the operator triggered (if any), so
/// the panel re-opens the journal for the new date. `None` means no navigation this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateNavOutcome {
    /// No navigation this frame.
    None,
    /// The displayed date changed to this date — re-open the journal.
    Navigated(NaiveDate),
}

/// The egui view over a [`DateNav`]: prev/next/today buttons, the current-date display, and a calendar
/// toggle that opens a 6×7 month grid popup. Borrows the nav by `&mut` so a click mutates it in place,
/// and returns a [`DateNavOutcome`] so the panel knows when to re-open the journal.
pub struct DateNavWidget<'a> {
    nav: &'a mut DateNav,
    palette: &'a HsPalette,
}

impl<'a> DateNavWidget<'a> {
    /// Build the widget over a borrowed nav + palette.
    pub fn new(nav: &'a mut DateNav, palette: &'a HsPalette) -> Self {
        Self { nav, palette }
    }

    /// Render the header row, returning the navigation outcome. Each control carries a stable AccessKit
    /// author_id (AC-11) so a swarm agent drives date navigation by id.
    pub fn show(mut self, ui: &mut egui::Ui) -> DateNavOutcome {
        let mut outcome = DateNavOutcome::None;
        ui.horizontal(|ui| {
            // ← Previous day.
            let prev = ui.button("◀");
            accessibility::emit_interactive_node(ui.ctx(), prev.id, PREV_DAY_ID);
            if prev.clicked() {
                outcome = DateNavOutcome::Navigated(self.nav.prev_day());
            }

            // The current-date display (an interactive label → addressable, and clicking it toggles the
            // calendar, mirroring the calendar-icon affordance for convenience).
            let display = self.nav.current_display();
            let date_resp = ui.add(egui::Label::new(
                egui::RichText::new(&display).color(self.palette.text).strong(),
            ).sense(egui::Sense::click()));
            accessibility::emit_interactive_node(ui.ctx(), date_resp.id, DATE_DISPLAY_ID);
            if date_resp.clicked() {
                self.nav.calendar_open = !self.nav.calendar_open;
            }

            // Next day →.
            let next = ui.button("▶");
            accessibility::emit_interactive_node(ui.ctx(), next.id, NEXT_DAY_ID);
            if next.clicked() {
                outcome = DateNavOutcome::Navigated(self.nav.next_day());
            }

            // 📅 calendar toggle.
            let cal = ui.button("📅");
            accessibility::emit_interactive_node(ui.ctx(), cal.id, CALENDAR_TOGGLE_ID);
            if cal.clicked() {
                self.nav.calendar_open = !self.nav.calendar_open;
            }

            // Today.
            let today = ui.button("Today");
            accessibility::emit_interactive_node(ui.ctx(), today.id, TODAY_ID);
            if today.clicked() {
                outcome = DateNavOutcome::Navigated(self.nav.jump_today());
            }
        });

        // The calendar popup (a fixed 6×7 month grid) when open.
        if self.nav.calendar_open {
            if let Some(picked) = self.render_calendar(ui) {
                self.nav.navigate_to(picked);
                outcome = DateNavOutcome::Navigated(picked);
            }
        }

        outcome
    }

    /// Render the 6×7 month-grid calendar popup. Returns `Some(date)` when a day is clicked. Allocates
    /// EXACTLY 6 rows (MC-004) so the popup never reflows between months; the current date + today are
    /// highlighted distinctly. Each day cell is an interactive AccessKit node (`journal-calendar-day-{n}`).
    fn render_calendar(&mut self, ui: &mut egui::Ui) -> Option<NaiveDate> {
        let mut picked = None;
        let cells = self.nav.month_grid();
        let month_label = self.nav.current.format("%B %Y").to_string();

        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_max_width(260.0);
            ui.label(egui::RichText::new(&month_label).color(self.palette.text).strong());
            // Weekday header row (Sun..Sat).
            ui.horizontal(|ui| {
                for wd in ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"] {
                    ui.add_sized(
                        egui::vec2(30.0, 18.0),
                        egui::Label::new(egui::RichText::new(wd).color(self.palette.text_subtle).small()),
                    );
                }
            });
            // EXACTLY 6 rows × 7 columns (MC-004): iterate the fixed 42-cell grid.
            egui::Grid::new("journal-calendar-grid")
                .num_columns(CALENDAR_COLS)
                .spacing(egui::vec2(2.0, 2.0))
                .show(ui, |ui| {
                    for (i, cell) in cells.iter().enumerate() {
                        match cell {
                            Some(date) => {
                                let is_current = self.nav.is_current(*date);
                                let is_today = self.nav.is_today(*date);
                                let mut text = egui::RichText::new(format!("{}", date.day()));
                                if is_current {
                                    // The displayed date: accent fill via a strong accent color.
                                    text = text.color(self.palette.accent).strong();
                                } else if is_today {
                                    text = text.color(self.palette.accent_soft).strong();
                                } else {
                                    text = text.color(self.palette.text);
                                }
                                let resp = ui.add_sized(
                                    egui::vec2(30.0, 24.0),
                                    egui::Button::new(text).frame(is_current || is_today),
                                );
                                accessibility::emit_interactive_node(
                                    ui.ctx(),
                                    resp.id,
                                    &format!("journal-calendar-day-{}", date.day()),
                                );
                                if resp.clicked() {
                                    picked = Some(*date);
                                }
                            }
                            None => {
                                // An out-of-month cell: blank, fixed size so the grid keeps its shape.
                                ui.add_sized(egui::vec2(30.0, 24.0), egui::Label::new(""));
                            }
                        }
                        if (i + 1) % CALENDAR_COLS == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
        picked
    }
}

/// The AccessKit role for a date-nav control (a button). Exposed so the panel/tests can assert it.
pub const NAV_BUTTON_ROLE: accesskit::Role = accesskit::Role::Button;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn prev_day_crosses_year_boundary() {
        // AC-3-ish + MC-005: prev from 2026-01-01 goes to 2025-12-31 (year wrap).
        let mut nav = DateNav::new(d(2026, 1, 1), d(2026, 6, 19));
        assert_eq!(nav.prev_day(), d(2025, 12, 31));
    }

    #[test]
    fn next_day_returns_from_prev() {
        // AC-4: next from 2026-06-18 returns to 2026-06-19.
        let mut nav = DateNav::new(d(2026, 6, 18), d(2026, 6, 19));
        assert_eq!(nav.next_day(), d(2026, 6, 19));
    }

    #[test]
    fn prev_day_from_known_date() {
        // AC-3: prev day from 2026-06-19 → 2026-06-18.
        let mut nav = DateNav::new(d(2026, 6, 19), d(2026, 6, 19));
        assert_eq!(nav.prev_day(), d(2026, 6, 18));
    }

    #[test]
    fn today_button_jumps_to_fixed_today() {
        // AC-5: Today from any date returns to the (fixed mock) today 2026-06-19.
        let mut nav = DateNav::new(d(2025, 3, 4), d(2026, 6, 19));
        assert_eq!(nav.jump_today(), d(2026, 6, 19));
    }

    #[test]
    fn checked_add_days_handles_month_and_leap_boundaries_mc005() {
        // MC-005: 2026-01-31 + 1 = 2026-02-01 (NOT a nonexistent 2026-01-32 / 2026-02-31).
        let mut jan = DateNav::new(d(2026, 1, 31), d(2026, 6, 19));
        assert_eq!(jan.next_day(), d(2026, 2, 1));
        // 2026-02-28 + 1 = 2026-03-01 (2026 is not a leap year).
        let mut feb = DateNav::new(d(2026, 2, 28), d(2026, 6, 19));
        assert_eq!(feb.next_day(), d(2026, 3, 1));
        // 2024-02-28 + 1 = 2024-02-29 (2024 IS a leap year).
        let mut leap = DateNav::new(d(2024, 2, 28), d(2024, 6, 19));
        assert_eq!(leap.next_day(), d(2024, 2, 29));
        assert_eq!(leap.next_day(), d(2024, 3, 1));
    }

    #[test]
    fn days_in_month_is_chrono_correct() {
        assert_eq!(days_in_month(2026, 2), 28); // common Feb
        assert_eq!(days_in_month(2024, 2), 29); // leap Feb
        assert_eq!(days_in_month(2026, 6), 30); // June
        assert_eq!(days_in_month(2026, 1), 31); // January
        assert_eq!(days_in_month(2026, 12), 31); // December (year-wrap path)
    }

    #[test]
    fn month_grid_is_always_42_cells_mc004() {
        // MC-004: the grid is ALWAYS 6×7 = 42 cells regardless of month, so the popup never reflows.
        for (y, m) in [(2026, 6), (2023, 10), (2026, 2), (2024, 2), (2026, 12)] {
            assert_eq!(month_grid(y, m).len(), CALENDAR_CELLS, "month {y}-{m} must have 42 cells");
        }
    }

    #[test]
    fn month_grid_june_2026_has_30_in_month_days_at_correct_offset() {
        // AC-6: June 2026 has 30 day cells with correct numbers. June 1 2026 is a Monday → column 1
        // (Sunday=0), so cell[0] is None (the leading Sunday) and cell[1] is June 1.
        let grid = month_grid(2026, 6);
        let in_month: Vec<u32> = grid.iter().flatten().map(|d| d.day()).collect();
        assert_eq!(in_month.len(), 30, "June has 30 days");
        assert_eq!(in_month.first(), Some(&1));
        assert_eq!(in_month.last(), Some(&30));
        // June 1, 2026 is a Monday → lead offset 1.
        assert!(grid[0].is_none(), "the leading Sunday cell is empty");
        assert_eq!(grid[1].map(|d| d.day()), Some(1), "June 1 sits at column 1 (Monday)");
    }

    #[test]
    fn six_row_month_october_2023_fits_in_the_fixed_grid_mc004() {
        // MC-004: October 2023 starts on Sunday and has 31 days → it spans 6 rows. The fixed 42-cell
        // grid holds all 31 in-month cells (the red-team 6-row-month case).
        let grid = month_grid(2023, 10);
        let in_month = grid.iter().flatten().count();
        assert_eq!(in_month, 31);
        assert_eq!(grid.len(), CALENDAR_CELLS, "still exactly 42 cells (no reflow)");
        // Oct 1, 2023 is a Sunday → it sits at column 0.
        assert_eq!(grid[0].map(|d| d.day()), Some(1));
    }

    #[test]
    fn current_storage_and_display_formats() {
        let nav = DateNav::new(d(2026, 6, 19), d(2026, 6, 19));
        assert_eq!(nav.current_storage(), "2026-06-19");
        // June 19, 2026 is a Friday.
        assert_eq!(nav.current_display(), "Friday, June 19, 2026");
    }

    #[test]
    fn is_current_and_is_today_highlight_logic() {
        let nav = DateNav::new(d(2026, 6, 18), d(2026, 6, 19));
        assert!(nav.is_current(d(2026, 6, 18)));
        assert!(!nav.is_current(d(2026, 6, 19)));
        assert!(nav.is_today(d(2026, 6, 19)));
        assert!(!nav.is_today(d(2026, 6, 18)));
    }

    #[test]
    fn navigate_to_closes_calendar() {
        let mut nav = DateNav::new(d(2026, 6, 18), d(2026, 6, 19));
        nav.calendar_open = true;
        nav.navigate_to(d(2026, 7, 4));
        assert_eq!(nav.current, d(2026, 7, 4));
        assert!(!nav.calendar_open, "navigating closes the calendar popup");
    }

    #[test]
    fn weekday_offset_uses_sunday_as_column_zero() {
        // A month starting on Sunday has lead 0; one starting on Saturday has lead 6.
        // July 2023 starts on Saturday → cell[6] is July 1.
        let july = month_grid(2023, 7);
        assert_eq!(july[6].map(|d| d.day()), Some(1));
        assert!((0..6).all(|i| july[i].is_none()), "Saturday start → 6 leading empties");
        // Sanity: the Sunday convention matches chrono.
        assert_eq!(d(2023, 7, 1).weekday(), Weekday::Sat);
    }
}
