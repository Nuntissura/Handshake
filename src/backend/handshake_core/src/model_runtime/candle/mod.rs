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
pub mod ssm_lora;
pub mod state_vector;
pub mod tokenizer;
#[cfg(feature = "candle-runtime-engine")]
pub mod transformer;

#[cfg(feature = "candle-runtime-engine")]
pub use abliteration_io::run_abliteration_model_io;
pub use adapter::{validate_candle_load_spec, CandleRuntime, CANDLE_NATIVE_FEATURE_DISABLED};
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
pub use state_vector::{
    SSMStateSnapshot, SSMStateVariant, SSMTensorSnapshot, StateVectorHandle, StateVectorId,
    StateVectorOps, StateVectorSnapshotRecord,
};
pub use tokenizer::{
    cache_tokenizer_if_present, tokenizer_json_path_for_artifact, CandleTokenizerCache,
};
#[cfg(feature = "candle-runtime-engine")]
pub use transformer::{CandleLlamaModel, TransformerModel};
