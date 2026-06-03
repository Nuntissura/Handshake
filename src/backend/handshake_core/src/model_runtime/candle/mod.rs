#[cfg(feature = "candle-runtime-engine")]
pub mod abliteration_io;
pub mod adapter;
pub mod device;
#[cfg(feature = "candle-runtime-engine")]
pub mod generate;
pub mod hooks;
#[cfg(feature = "candle-runtime-engine")]
pub mod instrumented_llama;
#[cfg(feature = "candle-runtime-engine")]
pub mod lora_impl;
#[cfg(feature = "candle-runtime-engine")]
pub mod mamba2;
#[cfg(feature = "candle-runtime-engine")]
pub mod rwkv_v5;
#[cfg(feature = "candle-runtime-engine")]
pub mod rwkv_v6;
#[cfg(feature = "candle-runtime-engine")]
pub mod rwkv_v7;
#[cfg(feature = "candle-runtime-engine")]
pub mod score_embed;
pub mod ssm_lora;
#[cfg(feature = "candle-runtime-engine")]
pub mod ssm_state;
pub mod state_vector;
pub mod tokenizer;
#[cfg(feature = "candle-runtime-engine")]
pub mod transformer;

#[cfg(feature = "candle-runtime-engine")]
pub use abliteration_io::run_abliteration_model_io;
pub use adapter::{
    load_local_candle_model, validate_candle_load_spec, CandleRuntime, LoadedCandleModel,
    CANDLE_NATIVE_FEATURE_DISABLED,
};
pub use device::{
    select_candle_device, CandleDeviceKind, CandleDevicePreference, CandleDeviceSelection,
};
pub use hooks::{CandleSteeringHooks, CANDLE_DEFAULT_RESIDUAL_WIDTH};
#[cfg(feature = "candle-runtime-engine")]
pub use mamba2::CandleMamba2Model;
#[cfg(feature = "candle-runtime-engine")]
pub use rwkv_v5::CandleRwkvV5Model;
#[cfg(feature = "candle-runtime-engine")]
pub use rwkv_v6::CandleRwkvV6Model;
#[cfg(feature = "candle-runtime-engine")]
pub use rwkv_v7::CandleRwkvV7Model;
#[cfg(feature = "candle-runtime-engine")]
pub use ssm_state::{snapshot_to_tensor, tensor_to_snapshot, LockedSsmStateSource, SsmStateSource};
pub use state_vector::{
    load_from_artifact_store, load_from_artifact_store_into_handle, load_from_bytes,
    load_into_handle, persist_to_artifact_store, persist_to_bytes, SSMStateSnapshot,
    SSMStateVariant, SSMTensorSnapshot, StateVectorHandle, StateVectorId, StateVectorOps,
    StateVectorPersistEnvelope, StateVectorPersistMetadata, StateVectorPersistRecord,
    StateVectorSnapshotRecord, STATE_VECTOR_PERSIST_ACTION_ID,
    STATE_VECTOR_PERSIST_ENVELOPE_VERSION, STATE_VECTOR_PERSIST_WRITE_BOX_SCHEMA_ID,
};
pub use tokenizer::{
    cache_tokenizer_if_present, tokenizer_json_path_for_artifact, CandleTokenizerCache,
};
#[cfg(feature = "candle-runtime-engine")]
pub use transformer::{CandleLlamaModel, TransformerModel};
