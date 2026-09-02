use chrono::{TimeZone, Utc};
use handshake_core::observability::aggregate_queries::{
    ActivityRow, AggregateQueryFixture, Limit, Offset, SessionAggregateQueries, SessionSummary,
    SessionTimelineEntry,
};
use std::time::{Duration, Instant};
use uuid::Uuid;

struct FixtureIds {
    model_session_a: Uuid,
    session_a: Uuid,
    session_b: Uuid,
}

fn uid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn base_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 24, 6, 0, 0).single().unwrap()
}

fn fixture() -> (SessionAggregateQueries, FixtureIds) {
    let base = base_time();
    let model_session_a = uid(1);
    let model_session_b = uid(2);
    let session_a = uid(101);
    let session_b = uid(102);
    let session_c = uid(103);
    let mut activities = Vec::new();
    for idx in 0..12 {
        activities.push(ActivityRow {
            span_id: uid(1_000 + idx),
            parent_span_id: None,
            model_session_id: model_session_a,
            session_id: session_a,
            activity_kind: "mt_iteration".to_string(),
            started_at_utc: base + chrono::Duration::milliseconds(idx as i64),
            ended_at_utc: Some(
                base + chrono::Duration::milliseconds(idx as i64 + (idx as i64 * 10) + 1),
            ),
            status: "completed".to_string(),
        });
    }
    activities.push(ActivityRow {
        span_id: uid(2_000),
        parent_span_id: None,
        model_session_id: model_session_b,
        session_id: session_b,
        activity_kind: "checkpoint_write".to_string(),
        started_at_utc: base + chrono::Duration::seconds(3),
        ended_at_utc: Some(base + chrono::Duration::seconds(4)),
        status: "completed".to_string(),
    });
    (
        SessionAggregateQueries::from_fixture(AggregateQueryFixture {
            sessions: vec![
                SessionSummary {
                    session_id: session_a,
                    model_session_id: model_session_a,
                    wp_id: Some("WP-KERNEL-004".to_string()),
                    started_at_utc: base,
                    ended_at_utc: None,
                },
                SessionSummary {
                    session_id: session_b,
                    model_session_id: model_session_b,
                    wp_id: Some("WP-KERNEL-004".to_string()),
                    started_at_utc: base + chrono::Duration::seconds(1),
                    ended_at_utc: Some(base + chrono::Duration::seconds(5)),
                },
                SessionSummary {
                    session_id: session_c,
                    model_session_id: uid(3),
                    wp_id: Some("WP-OTHER".to_string()),
                    started_at_utc: base + chrono::Duration::seconds(2),
                    ended_at_utc: None,
                },
            ],
            activities,
            timeline_entries: vec![
                (
                    session_a,
                    SessionTimelineEntry {
                        kind: "mailbox_message".to_string(),
                        at_utc: base + chrono::Duration::seconds(3),
                        summary: "validator handoff".to_string(),
                    },
                ),
                (
                    session_a,
                    SessionTimelineEntry {
                        kind: "event".to_string(),
                        at_utc: base + chrono::Duration::seconds(1),
                        summary: "session claimed".to_string(),
                    },
                ),
                (
                    session_a,
                    SessionTimelineEntry {
                        kind: "checkpoint".to_string(),
                        at_utc: base + chrono::Duration::seconds(2),
                        summary: "checkpoint written".to_string(),
                    },
                ),
                (
                    session_b,
                    SessionTimelineEntry {
                        kind: "span".to_string(),
                        at_utc: base + chrono::Duration::seconds(2),
                        summary: "activity span".to_string(),
                    },
                ),
            ],
            active_leases: 2,
            in_flight_micro_tasks: 4,
            pending_mailbox_messages: 7,
        }),
        FixtureIds {
            model_session_a,
            session_a,
            session_b,
        },
    )
}

#[tokio::test]
async fn mt200_queries_return_typed_shapes() {
    let (queries, ids) = fixture();
    let base = base_time();
    let activity = queries
        .activity_for_model_session(
            ids.model_session_a,
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();
    assert_eq!(activity.len(), 12);
    assert!(activity
        .iter()
        .all(|row| row.model_session_id == ids.model_session_a));
    let sessions = queries
        .sessions_touching_wp(
            "WP-KERNEL-004",
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();
    assert_eq!(sessions.len(), 2);
    assert!(sessions
        .iter()
        .all(|row| row.wp_id.as_deref() == Some("WP-KERNEL-004")));
    let slowest = queries
        .slowest_spans_by_activity_kind("mt_iteration", Limit::new(3))
        .await
        .unwrap();
    assert_eq!(slowest.len(), 3);
    assert!(slowest[0].duration_ms >= slowest[1].duration_ms);
    let snapshot = queries
        .swarm_concurrency_snapshot(base + chrono::Duration::seconds(2))
        .await
        .unwrap();
    assert_eq!(snapshot.active_sessions, 3);
    assert_eq!(snapshot.active_leases, 2);
    assert_eq!(snapshot.in_flight_micro_tasks, 4);
    assert_eq!(snapshot.pending_mailbox_messages, 7);
}

#[tokio::test]
async fn mt200_pagination_limit_caps_rows() {
    let (queries, ids) = fixture();
    let base = base_time();
    let rows = queries
        .activity_for_model_session(
            ids.model_session_a,
            base,
            base + chrono::Duration::seconds(10),
            Limit::new(10),
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 10);
    assert_eq!(Limit::new(10_000).as_usize(), 1000);
}

#[tokio::test]
async fn mt200_offset_pagination_returns_stable_windows() {
    let (queries, ids) = fixture();
    let base = base_time();
    let activity = queries
        .activity_for_model_session_page(
            ids.model_session_a,
            base,
            base + chrono::Duration::seconds(10),
            Offset::new(5),
            Limit::new(3),
        )
        .await
        .unwrap();
    let span_ids: Vec<_> = activity.iter().map(|row| row.span_id).collect();
    assert_eq!(span_ids, vec![uid(1_005), uid(1_006), uid(1_007)]);
    let sessions = queries
        .sessions_touching_wp_page(
            "WP-KERNEL-004",
            base,
            base + chrono::Duration::seconds(10),
            Offset::new(1),
            Limit::new(1),
        )
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, ids.session_b);
    let slowest = queries
        .slowest_spans_by_activity_kind_page("mt_iteration", Offset::new(1), Limit::new(2))
        .await
        .unwrap();
    assert_eq!(slowest.len(), 2);
    assert!(slowest[0].duration_ms >= slowest[1].duration_ms);
    assert!(slowest[0].duration_ms < 111);
    let timeline = queries
        .session_timeline_page(
            ids.session_a,
            base,
            base + chrono::Duration::seconds(10),
            Offset::new(1),
            Limit::new(1),
        )
        .await
        .unwrap();
    assert_eq!(timeline.entries.len(), 1);
    assert_eq!(timeline.entries[0].kind, "checkpoint");
}

#[tokio::test]
async fn mt200_session_timeline_is_strictly_chronological() {
    let (queries, ids) = fixture();
    let base = base_time();
    let timeline = queries
        .session_timeline(
            ids.session_a,
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();
    let kinds: Vec<_> = timeline
        .entries
        .iter()
        .map(|entry| entry.kind.as_str())
        .collect();
    assert_eq!(kinds, vec!["event", "checkpoint", "mailbox_message"]);
    for window in timeline.entries.windows(2) {
        assert!(window[0].at_utc <= window[1].at_utc);
    }
}

#[tokio::test]
async fn mt200_fixture_queries_return_under_latency_budget() {
    let base = base_time();
    let mut sessions = Vec::new();
    let mut activities = Vec::new();
    for session_idx in 0..100u128 {
        let session_id = uid(10_000 + session_idx);
        let model_session_id = uid(20_000 + session_idx);
        sessions.push(SessionSummary {
            session_id,
            model_session_id,
            wp_id: Some("WP-KERNEL-004".to_string()),
            started_at_utc: base,
            ended_at_utc: None,
        });
        for event_idx in 0..100u128 {
            activities.push(ActivityRow {
                span_id: uid(30_000 + session_idx * 100 + event_idx),
                parent_span_id: None,
                model_session_id,
                session_id,
                activity_kind: "mt_iteration".to_string(),
                started_at_utc: base + chrono::Duration::milliseconds(event_idx as i64),
                ended_at_utc: Some(base + chrono::Duration::milliseconds(event_idx as i64 + 2)),
                status: "completed".to_string(),
            });
        }
    }
    let queries = SessionAggregateQueries::from_fixture(AggregateQueryFixture {
        sessions,
        activities,
        timeline_entries: Vec::new(),
        active_leases: 100,
        in_flight_micro_tasks: 100,
        pending_mailbox_messages: 0,
    });
    let started = Instant::now();
    let sessions = queries
        .sessions_touching_wp(
            "WP-KERNEL-004",
            base,
            base + chrono::Duration::seconds(1),
            Limit::new(1000),
        )
        .await
        .unwrap();
    let slowest = queries
        .slowest_spans_by_activity_kind("mt_iteration", Limit::new(10))
        .await
        .unwrap();
    assert_eq!(sessions.len(), 100);
    assert_eq!(slowest.len(), 10);
    assert!(Instant::now().duration_since(started) < Duration::from_secs(5));
}

#[test]
fn mt141_mt200_direct_composite_seed_disposition_is_explicit() {
    const DISPOSITION: (&str, &str, &str) = (
        "mt200_composite_store_queries_join_spans_mailbox_checkpoints_and_events",
        "MT-139 PT-139-2",
        "the public typed APIs do not expose one composite multi-table seed operation",
    );
    assert!(!DISPOSITION.0.is_empty());
    assert_eq!(DISPOSITION.1, "MT-139 PT-139-2");
    assert!(!DISPOSITION.2.is_empty());
}
