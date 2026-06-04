use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};

use crate::process_ledger::{
    record_spawn, LedgerBatcher, ProcessEngineKind, ProcessOwnershipRecordId, SpawnMeta,
};

use super::{LoadSpec, ModelId, ModelRuntimeError, ProviderKind, RuntimeBinding};

pub const MODEL_PROCESS_METADATA_CAP_BYTES: usize = 4 * 1024;
pub const FR_EVT_MODEL_PROCESS_METADATA_CAPPED: &str = "FR-EVT-MODEL-PROCESS-METADATA-CAPPED";

pub trait ModelProcessRollback {
    fn kill_spawned_process(&self, pid: u32) -> Result<(), ModelRuntimeError>;
}

#[derive(Clone)]
pub struct ModelProcessLedgerRegistrar {
    ledger: LedgerBatcher,
    registered_pids: Arc<Mutex<HashSet<u32>>>,
}

impl ModelProcessLedgerRegistrar {
    pub fn new(ledger: LedgerBatcher) -> Self {
        Self {
            ledger,
            registered_pids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn register_model_process<R>(
        &self,
        spec: &LoadSpec,
        pid: u32,
        engine_kind: ProcessEngineKind,
        context: ModelProcessSpawnContext,
        rollback: &R,
    ) -> Result<Option<ProcessOwnershipRecordId>, ModelRuntimeError>
    where
        R: ModelProcessRollback + ?Sized,
    {
        if spec.provider == ProviderKind::ExternalCompat {
            return Ok(None);
        }

        if let Err(error) =
            validate_model_runtime_process_engine_kind(engine_kind, context.runtime_binding)
        {
            return Err(self.rollback_error(rollback, pid, error.to_string()));
        }

        {
            let mut registered_pids = self.registered_pids.lock().map_err(|_| {
                ModelRuntimeError::LoadError("model process pid registry lock poisoned".to_string())
            })?;
            if !registered_pids.insert(pid) {
                return Err(self.rollback_error(
                    rollback,
                    pid,
                    format!("model process pid already registered: {pid}"),
                ));
            }
        }

        let meta = spawn_meta_from_context(spec, pid, engine_kind, context);
        match record_spawn(&self.ledger, meta) {
            Ok(record_id) => Ok(Some(record_id)),
            Err(error) => {
                if let Ok(mut registered_pids) = self.registered_pids.lock() {
                    registered_pids.remove(&pid);
                }
                Err(self.rollback_error(
                    rollback,
                    pid,
                    format!("process ledger START write failed for model pid {pid}: {error}"),
                ))
            }
        }
    }

    fn rollback_error<R>(&self, rollback: &R, pid: u32, reason: String) -> ModelRuntimeError
    where
        R: ModelProcessRollback + ?Sized,
    {
        match rollback.kill_spawned_process(pid) {
            Ok(()) => ModelRuntimeError::LoadError(reason),
            Err(kill_error) => ModelRuntimeError::LoadError(format!(
                "{reason}; rollback kill failed for pid {pid}: {kill_error}"
            )),
        }
    }
}

pub fn validate_model_runtime_process_engine_kind(
    engine_kind: ProcessEngineKind,
    runtime_binding: RuntimeBinding,
) -> Result<(), ModelRuntimeError> {
    if engine_kind == ProcessEngineKind::AbliterationTool {
        return Err(ModelRuntimeError::LoadError(
            "AbliterationTool is offline-only and cannot be registered as a ModelRuntime process; review the artifact and re-register it under a regular runtime binding"
                .to_string(),
        ));
    }

    if !engine_kind.is_regular_model_runtime_engine() {
        return Err(ModelRuntimeError::LoadError(format!(
            "ModelRuntime process requires regular local engine kind llama_cpp or candle; got {}",
            engine_kind.as_str()
        )));
    }

    let expected_engine_kind = runtime_binding.process_engine_kind();
    if engine_kind != expected_engine_kind {
        return Err(ModelRuntimeError::LoadError(format!(
            "runtime_binding={} requires process_engine_kind={}, got {}",
            runtime_binding.adapter_id(),
            expected_engine_kind.as_str(),
            engine_kind.as_str()
        )));
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelProcessSpawnContext {
    pub model_id: ModelId,
    pub runtime_binding: RuntimeBinding,
    pub parent_session_id: String,
    pub started_at_utc: DateTime<Utc>,
    pub sandbox_adapter: String,
    pub owner_role: String,
    pub owner_wp: Option<String>,
    pub role_id: Option<String>,
    pub wp_id: Option<String>,
    pub mt_id: Option<String>,
    pub work_profile_id: Option<String>,
    pub metadata_blob: Value,
}

impl ModelProcessSpawnContext {
    pub fn new(
        model_id: ModelId,
        runtime_binding: RuntimeBinding,
        parent_session_id: impl Into<String>,
        sandbox_adapter: impl Into<String>,
    ) -> Self {
        Self {
            model_id,
            runtime_binding,
            parent_session_id: parent_session_id.into(),
            started_at_utc: Utc::now(),
            sandbox_adapter: sandbox_adapter.into(),
            owner_role: "KERNEL_BUILDER".to_string(),
            owner_wp: None,
            role_id: None,
            wp_id: None,
            mt_id: None,
            work_profile_id: None,
            metadata_blob: json!({}),
        }
    }

    pub fn with_owner<S>(mut self, owner_role: impl Into<String>, owner_wp: Option<S>) -> Self
    where
        S: Into<String>,
    {
        self.owner_role = owner_role.into();
        self.owner_wp = owner_wp.map(Into::into);
        self.wp_id = self.owner_wp.clone();
        self
    }

    pub fn with_role_id(mut self, role_id: impl Into<String>) -> Self {
        self.role_id = Some(role_id.into());
        self
    }

    pub fn with_mt_id(mut self, mt_id: impl Into<String>) -> Self {
        self.mt_id = Some(mt_id.into());
        self
    }

    pub fn with_work_profile_id(mut self, work_profile_id: impl Into<String>) -> Self {
        self.work_profile_id = Some(work_profile_id.into());
        self
    }

    pub fn with_metadata_blob(mut self, metadata_blob: Value) -> Self {
        self.metadata_blob = metadata_blob;
        self
    }
}

fn spawn_meta_from_context(
    spec: &LoadSpec,
    pid: u32,
    engine_kind: ProcessEngineKind,
    context: ModelProcessSpawnContext,
) -> SpawnMeta {
    let mut metadata = metadata_object(context.metadata_blob);
    metadata.insert("model_id".to_string(), json!(context.model_id.to_string()));
    metadata.insert(
        "runtime_binding".to_string(),
        json!(context.runtime_binding.adapter_id()),
    );
    metadata.insert(
        "process_engine_kind".to_string(),
        json!(engine_kind.as_str()),
    );
    metadata.insert(
        "provider".to_string(),
        json!(format!("{:?}", spec.provider)),
    );
    metadata.insert(
        "runtime_kind".to_string(),
        json!(format!("{:?}", spec.runtime_kind)),
    );
    metadata.insert(
        "artifact_path".to_string(),
        json!(spec.artifact_path.to_string_lossy().to_string()),
    );

    let mut meta = SpawnMeta::new(pid, engine_kind, context.owner_role);
    meta.model_id = Some(context.model_id.to_string());
    meta.runtime_binding = Some(context.runtime_binding.adapter_id().to_string());
    meta.parent_session_id = Some(context.parent_session_id);
    meta.started_at_utc = context.started_at_utc;
    meta.sandbox_adapter = Some(context.sandbox_adapter);
    meta.model_artifact_sha256 = Some(spec.sha256_expected.clone());
    meta.work_profile_id = context.work_profile_id;
    meta.owner_wp = context.owner_wp;
    meta.role_id = context.role_id;
    meta.wp_id = context.wp_id;
    meta.mt_id = context.mt_id;
    meta.metadata_blob = cap_model_process_metadata(Value::Object(metadata));
    meta
}

fn metadata_object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => {
            let mut map = Map::new();
            map.insert("metadata_blob".to_string(), other);
            map
        }
    }
}

fn cap_model_process_metadata(value: Value) -> Value {
    let bytes = serde_json::to_vec(&value).unwrap_or_default();
    if bytes.len() <= MODEL_PROCESS_METADATA_CAP_BYTES {
        return value;
    }

    json!({
        "capped": true,
        "original_bytes": bytes.len(),
        "cap_bytes": MODEL_PROCESS_METADATA_CAP_BYTES,
        "warning_event": FR_EVT_MODEL_PROCESS_METADATA_CAPPED,
    })
}
