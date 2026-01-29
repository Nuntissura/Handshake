use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use serde_json::json;
use uuid::Uuid;

const DUMMY_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn valid_ans001_validation_payload(session_id: Uuid, message_id: Uuid) -> serde_json::Value {
    json!({
        "schema_version": "hsk.fr.runtime_chat@0.1",
        "event_id": "FR-EVT-RUNTIME-CHAT-102",
        "ts_utc": chrono::Utc::now().to_rfc3339(),
        "session_id": session_id.to_string(),
        "type": "runtime_chat_ans001_validation",
        "message_id": message_id.to_string(),
        "role": "assistant",
        "model_role": "frontend",
        "ans001_compliant": true,
        "violation_clauses": [],
        "body_sha256": DUMMY_SHA256,
        "ans001_sha256": DUMMY_SHA256
    })
}

#[tokio::test]
async fn records_runtime_chat_ans001_validation_event() {
    let session_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let payload = valid_ans001_validation_payload(session_id, message_id);

    let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("recorder init");
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::RuntimeChatAns001Validation,
        FlightRecorderActor::System,
        session_id,
        payload,
    );

    recorder.record_event(event).await.expect("record_event");

    let events = recorder
        .list_events(EventFilter {
            trace_id: Some(session_id),
            ..Default::default()
        })
        .await
        .expect("list_events");

    assert!(events
        .iter()
        .any(|evt| evt.event_type == FlightRecorderEventType::RuntimeChatAns001Validation));
}

#[test]
fn rejects_runtime_chat_ans001_validation_unknown_key() {
    let session_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let mut payload = valid_ans001_validation_payload(session_id, message_id);
    payload
        .as_object_mut()
        .expect("payload object")
        .insert("content".to_string(), json!("leaky"));

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::RuntimeChatAns001Validation,
        FlightRecorderActor::System,
        session_id,
        payload,
    );

    assert!(event.validate().is_err());
}

#[test]
fn rejects_runtime_chat_ans001_validation_path_field() {
    let session_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let mut payload = valid_ans001_validation_payload(session_id, message_id);
    payload.as_object_mut().expect("payload object").insert(
        "chat_log_path".to_string(),
        json!("C:/Users/me/AppData/sessions/.../chat.jsonl"),
    );

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::RuntimeChatAns001Validation,
        FlightRecorderActor::System,
        session_id,
        payload,
    );

    assert!(event.validate().is_err());
}

#[test]
fn rejects_runtime_chat_ans001_validation_invalid_sha256() {
    let session_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let mut payload = valid_ans001_validation_payload(session_id, message_id);
    payload
        .as_object_mut()
        .expect("payload object")
        .insert("body_sha256".to_string(), json!("not-a-sha"));

    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::RuntimeChatAns001Validation,
        FlightRecorderActor::System,
        session_id,
        payload,
    );

    assert!(event.validate().is_err());
}
