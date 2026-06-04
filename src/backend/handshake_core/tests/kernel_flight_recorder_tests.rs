use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    RecorderError,
};
use handshake_core::kernel::{
    flight_recorder_mirror_event, KernelActor, KernelEvent, KernelEventType, NewKernelEvent,
};
use serde_json::json;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
struct MemoryFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait::async_trait]
impl FlightRecorder for MemoryFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self
            .events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .clone())
    }
}

#[test]
fn flight_recorder_kernel_mirror_is_diagnostic_not_authority() {
    let kernel_event = KernelEvent::from_new(
        NewKernelEvent::builder(
            "KTR-FR-MIRROR",
            "SR-FR-MIRROR",
            KernelEventType::PromotionDecided,
            KernelActor::PromotionGate("promotion-gate".to_string()),
        )
        .correlation_id("corr-fr-mirror")
        .payload(json!({"decision": "approved"}))
        .build()
        .expect("kernel event"),
    );

    let mirror = flight_recorder_mirror_event(&kernel_event);

    mirror
        .validate()
        .expect("valid Flight Recorder diagnostic event");
    assert_eq!(mirror.event_type, FlightRecorderEventType::Diagnostic);
    assert_eq!(mirror.payload["diagnostic_id"], "kernel_event_mirror");
    assert_eq!(mirror.payload["authority_source"], "postgres_event_ledger");
    assert_eq!(mirror.payload["projection_only"], true);
    assert_eq!(mirror.payload["kernel_event_id"], kernel_event.event_id);
    assert_eq!(mirror.payload["kernel_task_run_id"], "KTR-FR-MIRROR");
    assert_eq!(mirror.payload["session_run_id"], "SR-FR-MIRROR");
}

#[tokio::test]
async fn flight_recorder_kernel_mirror_persists_through_sink() {
    let recorder = MemoryFlightRecorder::default();
    let kernel_event = KernelEvent::from_new(
        NewKernelEvent::builder(
            "KTR-FR-SINK",
            "SR-FR-SINK",
            KernelEventType::ValidationRecorded,
            KernelActor::ValidationRunner("validation-runner".to_string()),
        )
        .correlation_id("corr-fr-sink")
        .payload(json!({"validation_id": "VAL-FR-SINK"}))
        .build()
        .expect("kernel event"),
    );

    recorder
        .record_event(flight_recorder_mirror_event(&kernel_event))
        .await
        .expect("record mirror event");
    let events = recorder
        .list_events(EventFilter::default())
        .await
        .expect("list mirror events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, FlightRecorderEventType::Diagnostic);
    assert_eq!(events[0].payload["diagnostic_id"], "kernel_event_mirror");
    assert_eq!(events[0].payload["projection_only"], true);
    assert_eq!(events[0].payload["kernel_event_id"], kernel_event.event_id);
}

#[test]
fn validates_llm_infer_spec_accept_and_reject_events() {
    let trace_id = Uuid::now_v7();
    let model_id = "model-target";

    for (event_type, event_id, payload_type, accepted_tokens, rejected_tokens) in [
        (
            FlightRecorderEventType::LlmInferenceSpecAccept,
            "FR-EVT-LLM-INFER-SPEC-ACCEPT",
            "llm_infer.spec_accept",
            2_u64,
            0_u64,
        ),
        (
            FlightRecorderEventType::LlmInferenceSpecReject,
            "FR-EVT-LLM-INFER-SPEC-REJECT",
            "llm_infer.spec_reject",
            0_u64,
            1_u64,
        ),
    ] {
        let mut event = FlightRecorderEvent::new(
            event_type,
            FlightRecorderActor::System,
            trace_id,
            json!({
                "schema_version": "hsk.fr.llm_infer_spec@0.1",
                "event_id": event_id,
                "type": payload_type,
                "trace_id": trace_id.to_string(),
                "model_id": model_id,
                "draft_model_id": null,
                "adapter": "llama_cpp",
                "mode": "ngram",
                "round_index": 1,
                "accepted_tokens": accepted_tokens,
                "rejected_tokens": rejected_tokens,
                "generated_tokens": accepted_tokens + rejected_tokens,
                "draft_calls": 1
            }),
        );
        event.model_id = Some(model_id.to_string());

        event
            .validate()
            .expect("llm inference speculative event validates");
    }
}

#[test]
fn rejects_llm_infer_spec_event_with_wrong_event_id() {
    let trace_id = Uuid::now_v7();
    let mut event = FlightRecorderEvent::new(
        FlightRecorderEventType::LlmInferenceSpecAccept,
        FlightRecorderActor::System,
        trace_id,
        json!({
            "schema_version": "hsk.fr.llm_infer_spec@0.1",
            "event_id": "FR-EVT-LLM-INFER-SPEC-REJECT",
            "type": "llm_infer.spec_accept",
            "trace_id": trace_id.to_string(),
            "model_id": "model-target",
            "draft_model_id": null,
            "adapter": "llama_cpp",
            "mode": "ngram",
            "round_index": 1,
            "accepted_tokens": 1,
            "rejected_tokens": 0,
            "generated_tokens": 1,
            "draft_calls": 1
        }),
    );
    event.model_id = Some("model-target".to_string());

    assert!(event.validate().is_err());
}
