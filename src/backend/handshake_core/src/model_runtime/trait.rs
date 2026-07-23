use async_trait::async_trait;
use std::time::Duration;

use super::CancellationToken;
use super::{
    activity::RuntimeQuiesceError, error::ModelRuntimeError, Embedding, GenerateRequest,
    KvCacheHandle, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
    RuntimeArtifactIntegrityReceipt, Score, SteeringHookHandle, TokenStream,
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

    /// Stop admitting detached work and await every worker owned by this runtime.
    ///
    /// The default fails closed. Runtimes with no detached work may explicitly
    /// return `Ok(())`; any runtime that starts a thread, `spawn_blocking` job,
    /// or detached task must override this method with a real barrier.
    async fn quiesce(&self, _timeout: Duration) -> Result<(), RuntimeQuiesceError> {
        Err(RuntimeQuiesceError::Unsupported {
            adapter: self.adapter_name().to_string(),
        })
    }

    /// Fail-closed model-scoped admission/cancellation barrier. Implementations
    /// must leave sibling model admission open.
    async fn quiesce_model(
        &self,
        _id: ModelId,
        _timeout: Duration,
    ) -> Result<(), RuntimeQuiesceError> {
        Err(RuntimeQuiesceError::Unsupported {
            adapter: self.adapter_name().to_string(),
        })
    }

    fn resume_model_admission(&self, _id: ModelId) -> Result<(), RuntimeQuiesceError> {
        Err(RuntimeQuiesceError::Unsupported {
            adapter: self.adapter_name().to_string(),
        })
    }

    /// Return the receipt for the exact behavior-bearing artifact bytes used
    /// to construct a loaded model. Runtimes that cannot prove this invariant
    /// fail closed through the default implementation.
    fn artifact_integrity(
        &self,
        _id: ModelId,
    ) -> Result<RuntimeArtifactIntegrityReceipt, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "exact_artifact_integrity_receipt".to_string(),
            adapter: self.adapter_name().to_string(),
        })
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError>;

    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError>;

    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError>;

    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError>;

    fn cancel(&self, token: CancellationToken);
}
