use handshake_core::flight_recorder::{
    FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use serde_json::json;
use uuid::Uuid;

const DUMMY_SHA256_LOWER: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn valid_model_swap_requested_payload() -> serde_json::Value {
    json!({
        "type": "model_swap_requested",
        "request_id": "550e8400-e29b-41d4-a716-446655440000",
        "current_model_id": "ollama:llama3.1",
        "target_model_id": "ollama:llama3.1-instruct",
        "role": "orchestrator",
        "reason": "escalation",
        "swap_strategy": "unload_reload",
        "max_vram_mb": 4096,
        "max_ram_mb": 8192,
        "timeout_ms": 1000,
        "state_persist_refs": [
            "artifact:550e8400-e29b-41d4-a716-446655440001:/data/model_swap/state.json"
        ],
        "state_hash": DUMMY_SHA256_LOWER,
        "context_compile_ref": "artifact:550e8400-e29b-41d4-a716-446655440002:/data/ace/context",
        "wp_id": "WP-1-Model-Swap-Protocol-v1",
        "mt_id": "MT-0001"
    })
}

#[test]
fn accepts_model_swap_requested_payload() {
    let payload = valid_model_swap_requested_payload();
    let trace_id = Uuid::new_v4();
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::ModelSwapRequested,
        FlightRecorderActor::System,
        trace_id,
        payload,
    );

    assert!(event.validate().is_ok());
}

#[test]
fn rejects_model_swap_requested_wrong_type() {
    let mut payload = valid_model_swap_requested_payload();
    payload
        .as_object_mut()
        .expect("payload object")
        .insert("type".to_string(), json!("model_swap_failed"));

    let trace_id = Uuid::new_v4();
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::ModelSwapRequested,
        FlightRecorderActor::System,
        trace_id,
        payload,
    );

    assert!(event.validate().is_err());
}

#[test]
fn rejects_model_swap_requested_invalid_role() {
    let mut payload = valid_model_swap_requested_payload();
    payload
        .as_object_mut()
        .expect("payload object")
        .insert("role".to_string(), json!("invalid"));

    let trace_id = Uuid::new_v4();
    let event = FlightRecorderEvent::new(
        FlightRecorderEventType::ModelSwapRequested,
        FlightRecorderActor::System,
        trace_id,
        payload,
    );

    assert!(event.validate().is_err());
}
