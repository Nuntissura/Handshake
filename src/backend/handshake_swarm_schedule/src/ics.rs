use chrono::{DateTime, Utc};
use icalendar::{Calendar, Component, Event, EventLike};

use super::driver::SwarmSchedule;

/// Export schedules as an RFC-5545 ICS calendar so the operator can subscribe in
/// any calendar app (CalDAV / Google / Apple) -- no calendar server needed. Each
/// schedule becomes a `VEVENT` whose description carries the cron + action and an
/// `X-HANDSHAKE-CRON` property carries the raw expression; `DTSTART` is set to
/// `generated_at` (injected so the export is deterministic).
pub fn schedules_to_ics(schedules: &[SwarmSchedule], generated_at: DateTime<Utc>) -> String {
    let mut calendar = Calendar::new();
    for schedule in schedules {
        let event = Event::new()
            .uid(&format!("handshake-swarm-{}@handshake", schedule.id))
            .summary(&schedule.summary)
            .description(&format!(
                "{}\ncron: {} (UTC)",
                schedule.action.human(),
                schedule.cron
            ))
            .add_property("X-HANDSHAKE-CRON", &schedule.cron)
            .starts(generated_at)
            .done();
        calendar.push(event);
    }
    calendar.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::ScheduledAction;
    use std::time::Duration;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-06-01T09:00:00Z")
            .expect("valid rfc3339")
            .with_timezone(&Utc)
    }

    #[test]
    fn schedules_to_ics_emits_vevent_with_cron_and_action() {
        let s = SwarmSchedule {
            id: "morning-research".to_string(),
            cron: "0 0 9 * * *".to_string(),
            summary: "Morning research swarm".to_string(),
            action: ScheduledAction::SpinUp {
                swarm_id: "research".to_string(),
                time_box: Some(Duration::from_secs(3600)),
            },
        };
        let ics = schedules_to_ics(&[s], fixed_now());
        assert!(ics.contains("BEGIN:VCALENDAR"), "{ics}");
        assert!(ics.contains("BEGIN:VEVENT"), "{ics}");
        assert!(ics.contains("Morning research swarm"), "{ics}");
        assert!(ics.contains("0 0 9 * * *"), "cron must appear: {ics}");
        assert!(ics.contains("research"), "swarm id must appear: {ics}");
        assert!(
            ics.contains("handshake-swarm-morning-research@handshake"),
            "uid: {ics}"
        );
    }
}
