use async_trait::async_trait;

use super::CancellationToken;
use super::{
    error::ModelRuntimeError, Embedding, GenerateRequest, KvCacheHandle, LoadSpec, LoraStackHandle,
    ModelCapabilities, ModelId, Score, SteeringHookHandle, TokenStream,
};

#[async_trait]
pub trait ModelRuntime: Send + Sync {
    fn adapter_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError>;

    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError>;

    fn generate(&self, req: GenerateRequest) -> TokenStream;

    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, ModelRuntimeError>;

    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError>;

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError>;

    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError>;

    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError>;

    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError>;

    fn cancel(&self, token: CancellationToken);
}
