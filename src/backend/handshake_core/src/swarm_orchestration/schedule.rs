//! rank-7: calendar-driven swarm scheduling.
//!
//! Models scheduled spin-up / teardown of swarm sessions, fires them on cron via
//! `tokio-cron-scheduler` against an INJECTABLE action callback (the app wires the
//! callback to `SwarmCoordinator::spawn_session` / `cancel`), and exports the
//! schedule as RFC-5545 ICS for CalDAV / Google / Apple import -- "links to
//! calendar functionality" without standing up a calendar server.
//!
//! A scheduled spin-up is a TIME-BOXED spawn (rank-7 `SpawnRequest::with_time_box`),
//! so the EXISTING lease+reaper path tears it down at its box -- no new teardown
//! code. Every fire is meant to emit the `FR-EVT-SWARM-SCHED-*` ids added in rank-3.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use icalendar::{Calendar, Component, Event, EventLike};
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

/// What a schedule fire does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduledAction {
    /// Spin up a swarm session, time-boxed (so the existing reaper reclaims it).
    SpinUp {
        swarm_id: String,
        time_box: Option<Duration>,
    },
    /// Tear down the sessions of a swarm.
    Teardown { swarm_id: String },
}

impl ScheduledAction {
    fn human(&self) -> String {
        match self {
            ScheduledAction::SpinUp { swarm_id, time_box } => {
                let boxed = time_box
                    .map(|d| format!(", time-box {}s", d.as_secs()))
                    .unwrap_or_default();
                format!("spin-up swarm '{swarm_id}'{boxed}")
            }
            ScheduledAction::Teardown { swarm_id } => format!("teardown swarm '{swarm_id}'"),
        }
    }
}

/// A calendar schedule entry: a (UTC) cron expression + the action it fires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmSchedule {
    pub id: String,
    /// 6-field cron (`sec min hour dom mon dow`), interpreted in UTC.
    pub cron: String,
    pub summary: String,
    pub action: ScheduledAction,
}

/// One scheduled fire delivered to the action callback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledFire {
    pub schedule_id: String,
    pub action: ScheduledAction,
}

/// Callback invoked on every scheduled fire. The app wires this to the
/// coordinator (`SpinUp` -> `spawn_session` with `with_time_box`; `Teardown` ->
/// `cancel`/`terminate`).
pub type ScheduleFireFn = Arc<dyn Fn(ScheduledFire) + Send + Sync>;

/// In-process cron scheduler for swarm spin-up / teardown, backed by
/// `tokio-cron-scheduler` (Tokio-native, UTC). Fires the injected callback on
/// each cron occurrence.
pub struct SwarmScheduler {
    inner: JobScheduler,
}

impl SwarmScheduler {
    /// Build the scheduler (does not start ticking until [`Self::start`]).
    pub async fn new() -> Result<Self, JobSchedulerError> {
        Ok(Self {
            inner: JobScheduler::new().await?,
        })
    }

    /// Register a schedule. Each cron occurrence delivers a [`ScheduledFire`] to
    /// `on_fire`. Errors only on an invalid cron expression.
    pub async fn add(
        &self,
        schedule: SwarmSchedule,
        on_fire: ScheduleFireFn,
    ) -> Result<(), JobSchedulerError> {
        let schedule_id = schedule.id.clone();
        let action = schedule.action.clone();
        let job = Job::new_async(schedule.cron.as_str(), move |_uuid, _lock| {
            // FnMut may fire repeatedly: clone the per-fire payload + callback
            // into each invocation's future.
            let fire = ScheduledFire {
                schedule_id: schedule_id.clone(),
                action: action.clone(),
            };
            let cb = on_fire.clone();
            Box::pin(async move {
                cb(fire);
            })
        })?;
        self.inner.add(job).await?;
        Ok(())
    }

    /// Start ticking (spawns the scheduler's background task).
    pub async fn start(&self) -> Result<(), JobSchedulerError> {
        self.inner.start().await
    }

    /// Stop ticking.
    pub async fn shutdown(&mut self) -> Result<(), JobSchedulerError> {
        self.inner.shutdown().await
    }
}

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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        assert!(ics.contains("handshake-swarm-morning-research@handshake"), "uid: {ics}");
    }

    #[tokio::test]
    async fn swarm_scheduler_fires_callback_on_cron() {
        let fired = Arc::new(AtomicUsize::new(0));
        let last_action = Arc::new(std::sync::Mutex::new(None));
        let f2 = fired.clone();
        let la = last_action.clone();

        let sched = SwarmScheduler::new().await.expect("scheduler");
        sched
            .add(
                SwarmSchedule {
                    id: "every-second".to_string(),
                    cron: "* * * * * *".to_string(), // every second (6-field)
                    summary: "tick".to_string(),
                    action: ScheduledAction::Teardown {
                        swarm_id: "x".to_string(),
                    },
                },
                Arc::new(move |fire: ScheduledFire| {
                    f2.fetch_add(1, Ordering::SeqCst);
                    *la.lock().unwrap() = Some(fire.action);
                }),
            )
            .await
            .expect("add schedule");
        sched.start().await.expect("start");

        tokio::time::sleep(Duration::from_millis(2500)).await;

        assert!(
            fired.load(Ordering::SeqCst) >= 1,
            "the cron schedule must fire at least once within 2.5s"
        );
        assert_eq!(
            *last_action.lock().unwrap(),
            Some(ScheduledAction::Teardown {
                swarm_id: "x".to_string()
            })
        );
    }
}
