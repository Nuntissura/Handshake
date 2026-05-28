use serde_json::json;
use uuid::Uuid;

use crate::{
    flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType},
    model_runtime::{FinishReason, ModelId},
};

pub const FR_EVT_LLM_INFER_START: &str = "FR-EVT-LLM-INFER-START";
pub const FR_EVT_LLM_INFER_TOKEN: &str = "FR-EVT-LLM-INFER-TOKEN";
pub const FR_EVT_LLM_INFER_END: &str = "FR-EVT-LLM-INFER-END";
pub const FR_EVT_LLM_INFER_LORA_MOUNT: &str = "FR-EVT-LLM-INFER-LORA-MOUNT";
pub const FR_EVT_LLM_INFER_LORA_UNMOUNT: &str = "FR-EVT-LLM-INFER-LORA-UNMOUNT";
pub const FR_EVT_LLM_INFER_LORA_SWAP: &str = "FR-EVT-LLM-INFER-LORA-SWAP";
pub const FR_EVT_LLM_INFER_KV_EVICT: &str = "FR-EVT-LLM-INFER-KV-EVICT";
pub const FR_EVT_LLM_INFER_KV_SET_QUANTIZATION: &str = "FR-EVT-LLM-INFER-KV-SET-QUANTIZATION";
pub const FR_EVT_LLM_INFER_KV_PREFIX_COMMIT: &str = "FR-EVT-LLM-INFER-KV-PREFIX-COMMIT";
pub const FR_EVT_LLM_INFER_KV_PREFIX_RESTORE: &str = "FR-EVT-LLM-INFER-KV-PREFIX-RESTORE";
pub const FR_EVT_LLM_INFER_CANCEL: &str = "FR-EVT-LLM-INFER-CANCEL";
pub const FR_EVT_LLM_INFER_CAPS_LOOKUP: &str = "FR-EVT-LLM-INFER-CAPS-LOOKUP";
pub const LLM_INFER_TOKEN_SAMPLE_INTERVAL: u32 = 16;

pub fn new_llm_infer_request_id() -> Uuid {
    Uuid::now_v7()
}

pub fn should_emit_token_event(token_index: u32) -> bool {
    token_index > 0 && token_index % LLM_INFER_TOKEN_SAMPLE_INTERVAL == 0
}

pub fn infer_start_event(
    model_id: ModelId,
    request_id: Uuid,
    tokens_in_prompt: u64,
    _prompt_preview: &str,
    adapter: &str,
) -> FlightRecorderEvent {
    llm_infer_event(
        model_id,
        request_id,
        json!({
            "schema_version": "hsk.fr.llm_infer@0.1",
            "event_id": FR_EVT_LLM_INFER_START,
            "type": "llm_inference",
            "phase": "start",
            "trace_id": request_id.to_string(),
            "request_id": request_id.to_string(),
            "model_call_correlation_id": request_id.to_string(),
            "model_id": model_id.to_string(),
            "adapter": adapter,
            "tokens_in_prompt": tokens_in_prompt,
            "ordered_index": 0_u64,
            "token_usage": {
                "prompt_tokens": tokens_in_prompt,
                "completion_tokens": 0_u64,
                "total_tokens": tokens_in_prompt
            }
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn infer_token_event(
    model_id: ModelId,
    request_id: Uuid,
    token_index: u32,
    token_id: u32,
    _token_text: &str,
    latency_ms: u64,
    adapter: &str,
) -> FlightRecorderEvent {
    llm_infer_event(
        model_id,
        request_id,
        json!({
            "schema_version": "hsk.fr.llm_infer@0.1",
            "event_id": FR_EVT_LLM_INFER_TOKEN,
            "type": "llm_inference",
            "phase": "token",
            "trace_id": request_id.to_string(),
            "request_id": request_id.to_string(),
            "model_call_correlation_id": request_id.to_string(),
            "model_id": model_id.to_string(),
            "adapter": adapter,
            "token_index": token_index,
            "token_id": token_id,
            "latency_ms": latency_ms,
            "ordered_index": token_index,
            "token_usage": {
                "prompt_tokens": 0_u64,
                "completion_tokens": token_index,
                "total_tokens": token_index
            }
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn infer_end_event(
    model_id: ModelId,
    request_id: Uuid,
    tokens_in_prompt: u64,
    tokens_generated: u32,
    total_ms: u64,
    prompt_eval_ms: u64,
    gen_eval_ms: u64,
    finish_reason: FinishReason,
    adapter: &str,
) -> FlightRecorderEvent {
    let completion_tokens = u64::from(tokens_generated);
    llm_infer_event(
        model_id,
        request_id,
        json!({
            "schema_version": "hsk.fr.llm_infer@0.1",
            "event_id": FR_EVT_LLM_INFER_END,
            "type": "llm_inference",
            "phase": "end",
            "trace_id": request_id.to_string(),
            "request_id": request_id.to_string(),
            "model_call_correlation_id": request_id.to_string(),
            "model_id": model_id.to_string(),
            "adapter": adapter,
            "tokens_generated": tokens_generated,
            "total_ms": total_ms,
            "prompt_eval_ms": prompt_eval_ms,
            "gen_eval_ms": gen_eval_ms,
            "finish_reason": finish_reason_label(finish_reason),
            "ordered_index": u64::from(tokens_generated).saturating_add(1),
            "token_usage": {
                "prompt_tokens": tokens_in_prompt,
                "completion_tokens": completion_tokens,
                "total_tokens": tokens_in_prompt.saturating_add(completion_tokens)
            }
        }),
    )
}

fn llm_infer_event(
    model_id: ModelId,
    request_id: Uuid,
    payload: serde_json::Value,
) -> FlightRecorderEvent {
    FlightRecorderEvent::new(
        FlightRecorderEventType::LlmInference,
        FlightRecorderActor::System,
        request_id,
        payload,
    )
    .with_model_id(model_id.to_string())
}

fn finish_reason_label(reason: FinishReason) -> &'static str {
    match reason {
        FinishReason::Stop => "stop",
        FinishReason::Length => "length",
        FinishReason::Cancelled => "cancelled",
        FinishReason::Error => "error",
    }
}
