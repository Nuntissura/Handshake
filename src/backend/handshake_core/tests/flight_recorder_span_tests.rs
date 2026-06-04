//! WP-KERNEL-004 cluster X.4 (MT-196..MT-200) integration tests.

use handshake_core::flight_recorder::fr_event_registry::{FrEventId, FrEventRegistry};
use handshake_core::flight_recorder::spans::{
    ActivityKind, ActivitySpan, AttributeValue, FrSpanRecorder, SpanContext, SpanGuard, SpanId,
    StubFrRecorder,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn mt_196_span_id_is_v7() {
    let id = SpanId::new_v7();
    assert_eq!(id.as_uuid().get_version_num(), 7);
}

#[test]
fn mt_196_session_span_drops_emit_started_and_ended() {
    let recorder = Arc::new(StubFrRecorder::new());
    {
        let (_g, _s) = SpanGuard::start_session(
            Uuid::now_v7(),
            Uuid::now_v7(),
            BTreeMap::new(),
            Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
        );
    }
    let events = recorder.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].0, FrEventId::SpanStarted);
    assert_eq!(events[1].0, FrEventId::SpanEnded);
}

#[test]
fn mt_196_activity_span_correlation_to_parent() {
    let recorder = Arc::new(StubFrRecorder::new());
    let parent = SpanId::new_v7();
    {
        let mut attrs = BTreeMap::new();
        attrs.insert(
            "label".to_string(),
            AttributeValue::String("iter-1".to_string()),
        );
        let (_g, _s) = SpanGuard::start_activity(
            parent,
            ActivityKind::MtIteration,
            attrs,
            Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
        );
    }
    let events = recorder.events.lock().unwrap();
    let started = &events[0].1;
    assert_eq!(
        started.get("parent_span_id").and_then(|v| v.as_str()),
        Some(parent.as_uuid().to_string()).as_deref()
    );
}

#[tokio::test]
async fn mt_196_activity_span_uses_task_local_parent_context() {
    let recorder = Arc::new(StubFrRecorder::new());
    let parent = SpanId::new_v7();

    let activity_span = SpanContext::scope(parent, async {
        let (_guard, span) = ActivitySpan::start(
            ActivityKind::CheckpointWrite,
            BTreeMap::new(),
            Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
        )
        .expect("task-local parent should allow activity span start");
        span
    })
    .await;

    assert_eq!(activity_span.parent_span_id, parent);
}

#[test]
fn mt_196_span_attributes_are_bounded() {
    let recorder = Arc::new(StubFrRecorder::new());
    let mut attrs = BTreeMap::new();
    for i in 0..17 {
        attrs.insert(format!("k{i}"), AttributeValue::Bool(true));
    }

    let result = SpanGuard::try_start_session(
        Uuid::now_v7(),
        Uuid::now_v7(),
        attrs,
        Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
    );

    assert!(result.is_err(), "spans must reject more than 16 attributes");
}

#[test]
fn mt_198_fr_event_id_round_trip() {
    for id in FrEventId::all() {
        let s = id.as_str();
        let back = FrEventId::from_str_id(s).unwrap();
        assert_eq!(*id, back);
    }
}

#[test]
fn mt_198_unknown_id_returns_error() {
    assert!(FrEventId::from_str_id("FR-EVT-UNKNOWN").is_err());
}

#[test]
fn mt_198_registry_aligns_with_enum() {
    let r = FrEventRegistry::from_rust_enum();
    assert_eq!(r.events.len(), FrEventId::all().len());
    // Every event id is non-empty and uses FR-EVT- prefix.
    for e in r.events {
        assert!(e.id.starts_with("FR-EVT-"), "{}", e.id);
    }
}

#[test]
fn mt_199_span_failure_emits_failed_event() {
    let recorder = Arc::new(StubFrRecorder::new());
    {
        let (mut guard, _) = SpanGuard::start_session(
            Uuid::now_v7(),
            Uuid::now_v7(),
            BTreeMap::new(),
            Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
        );
        guard.fail("simulated failure");
    }
    let events = recorder.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].0, FrEventId::SpanFailed);
}

#[test]
fn mt_199_nested_spans_emit_in_correct_order() {
    let recorder = Arc::new(StubFrRecorder::new());
    {
        let (session_guard, session_span) = SpanGuard::start_session(
            Uuid::now_v7(),
            Uuid::now_v7(),
            BTreeMap::new(),
            Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
        );
        {
            let (_a, _) = SpanGuard::start_activity(
                session_span.span_id,
                ActivityKind::MtIteration,
                BTreeMap::new(),
                Arc::clone(&recorder) as Arc<dyn FrSpanRecorder>,
            );
        }
        drop(session_guard);
    }
    let events = recorder.events.lock().unwrap();
    assert_eq!(
        events.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec![
            FrEventId::SpanStarted,
            FrEventId::SpanStarted,
            FrEventId::SpanEnded,
            FrEventId::SpanEnded
        ]
    );
}
