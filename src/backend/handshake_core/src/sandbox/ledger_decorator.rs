use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

use crate::process_ledger::{
    cap_metadata_jsonb, LedgerBatcher, ProcessEngineKind, ProcessStart, ProcessStop,
};

use super::{
    AdapterCapabilities, BindMode, Command, ExecResult, NetPolicy, ProcessHandle, ProcessSpec,
    ProcessStatus, SandboxAdapter, SandboxAdapterError, Signal, SnapshotRef,
};

#[derive(Clone)]
pub struct LedgerDecorator {
    inner: Arc<dyn SandboxAdapter>,
    ledger: LedgerBatcher,
    starts: Arc<Mutex<HashMap<Uuid, ProcessStart>>>,
    stopped: Arc<Mutex<HashSet<Uuid>>>,
}

impl LedgerDecorator {
    pub fn new(inner: Arc<dyn SandboxAdapter>, ledger: LedgerBatcher) -> Self {
        Self {
            inner,
            ledger,
            starts: Arc::new(Mutex::new(HashMap::new())),
            stopped: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn start_event(&self, spec: &ProcessSpec, handle: &ProcessHandle) -> ProcessStart {
        let metadata = cap_metadata_jsonb(&spec.metadata);
        let role_id = spec
            .metadata
            .get("role_id")
            .cloned()
            .unwrap_or_else(|| "KERNEL_BUILDER".to_string());
        let wp_id = spec.metadata.get("wp_id").cloned();
        let engine_kind = spec
            .metadata
            .get("engine_kind")
            .and_then(|value| ProcessEngineKind::try_from(value.as_str()).ok())
            .unwrap_or(ProcessEngineKind::SandboxContainer);

        let mut event = ProcessStart::new(engine_kind, role_id.clone(), wp_id.clone())
            .with_process_uuid(handle.id)
            .with_sandbox_adapter_id(handle.adapter_id.as_str().to_string())
            .with_sandbox_internal_id(handle.sandbox_internal_id.clone())
            .with_role_id(role_id)
            .with_sandbox_capabilities_snapshot(capabilities_snapshot(&self.inner.capabilities()))
            .with_metadata_jsonb(metadata.value);

        if let Some(pid) = handle.pid {
            event = event.with_os_pid(pid);
        }
        if let Some(parent_session_id) = spec.metadata.get("parent_session_id") {
            event = event.with_parent_session_id(parent_session_id.clone());
        }
        if let Some(parent_process_id) = spec
            .metadata
            .get("parent_process_id")
            .and_then(|value| Uuid::parse_str(value).ok())
        {
            event = event.with_parent_process_id(parent_process_id);
        }
        if let Some(wp_id) = wp_id {
            event = event.with_wp_id(wp_id);
        }
        if let Some(mt_id) = spec.metadata.get("mt_id") {
            event = event.with_mt_id(mt_id.clone());
        }
        if let Some(work_profile_id) = spec
            .metadata
            .get("work_profile_id")
            .or_else(|| spec.metadata.get("work_profile_override"))
        {
            event = event.with_work_profile_id(work_profile_id.clone());
        }
        if let Some(model_artifact_sha256) = spec.metadata.get("model_artifact_sha256") {
            event = event.with_model_artifact_sha256(model_artifact_sha256.clone());
        }

        event
    }

    fn record_stop_once(
        &self,
        handle: &ProcessHandle,
        exit_code: Option<i32>,
        stop_reason: impl Into<String>,
    ) -> Result<(), SandboxAdapterError> {
        if !self
            .stopped
            .lock()
            .map_err(|error| SandboxAdapterError::AdapterUnavailable {
                adapter_id: handle.adapter_id.clone(),
                reason: format!("ledger decorator state poisoned: {error}"),
            })?
            .insert(handle.id)
        {
            return Ok(());
        }

        let start = self
            .starts
            .lock()
            .map_err(|error| SandboxAdapterError::AdapterUnavailable {
                adapter_id: handle.adapter_id.clone(),
                reason: format!("ledger decorator state poisoned: {error}"),
            })?
            .get(&handle.id)
            .cloned()
            .unwrap_or_else(|| {
                ProcessStart::new(ProcessEngineKind::SandboxContainer, "KERNEL_BUILDER", None)
                    .with_process_uuid(handle.id)
                    .with_sandbox_adapter_id(handle.adapter_id.as_str().to_string())
                    .with_sandbox_internal_id(handle.sandbox_internal_id.clone())
                    .with_sandbox_capabilities_snapshot(capabilities_snapshot(
                        &self.inner.capabilities(),
                    ))
            });
        let stop = ProcessStop::from_start(&start, exit_code).with_stop_reason(stop_reason.into());
        self.ledger
            .record_stop(stop)
            .map_err(|error| SandboxAdapterError::AdapterUnavailable {
                adapter_id: handle.adapter_id.clone(),
                reason: format!("process ledger STOP write failed: {error}"),
            })
    }
}

#[async_trait]
impl SandboxAdapter for LedgerDecorator {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        let handle = self.inner.spawn(spec.clone()).await?;
        let start = self.start_event(&spec, &handle);
        self.ledger.record_start(start.clone()).map_err(|error| {
            SandboxAdapterError::SpawnFailed {
                adapter_id: handle.adapter_id.clone(),
                reason: format!("process ledger START write failed: {error}"),
            }
        })?;
        self.starts
            .lock()
            .map_err(|error| SandboxAdapterError::SpawnFailed {
                adapter_id: handle.adapter_id.clone(),
                reason: format!("ledger decorator state poisoned: {error}"),
            })?
            .insert(handle.id, start);
        Ok(handle)
    }

    async fn exec(
        &self,
        handle: &ProcessHandle,
        cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        self.inner.exec(handle, cmd).await
    }

    async fn fs_bind(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
        mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        self.inner
            .fs_bind(handle, host_path, guest_path, mode)
            .await
    }

    async fn copy_in(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.copy_in(handle, host_path, guest_path).await
    }

    async fn copy_out(
        &self,
        handle: &ProcessHandle,
        guest_path: PathBuf,
        host_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.copy_out(handle, guest_path, host_path).await
    }

    async fn net_policy(
        &self,
        handle: &ProcessHandle,
        policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.net_policy(handle, policy).await
    }

    async fn kill(
        &self,
        handle: &ProcessHandle,
        signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.kill(handle, signal).await?;
        self.record_stop_once(handle, None, format!("kill:{}", signal_name(signal)))
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        let status = self.inner.status(handle).await?;
        match status {
            ProcessStatus::Exited { code } => {
                self.record_stop_once(handle, Some(code), "status:exited")?;
            }
            ProcessStatus::Killed { by_signal } => {
                self.record_stop_once(
                    handle,
                    None,
                    format!("status:killed:{}", signal_name(by_signal)),
                )?;
            }
            ProcessStatus::Running
            | ProcessStatus::Orphaned
            | ProcessStatus::FailedToStart { .. } => {}
        }
        Ok(status)
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        self.inner.exit_code(handle).await
    }

    async fn snapshot(&self, handle: &ProcessHandle) -> Result<SnapshotRef, SandboxAdapterError> {
        // Delegate to the wrapped adapter so snapshot capability is preserved
        // through the ledger decorator (e.g. a wrapped CloudHypervisorAdapter).
        self.inner.snapshot(handle).await
    }

    async fn restore(&self, snapshot: &SnapshotRef) -> Result<ProcessHandle, SandboxAdapterError> {
        let handle = self.inner.restore(snapshot).await?;
        // A restored instance is a freshly-running sandbox; record a START so
        // the process ledger keeps a STOP partner for it on kill/status.
        let start = ProcessStart::new(ProcessEngineKind::SandboxContainer, "KERNEL_BUILDER", None)
            .with_process_uuid(handle.id)
            .with_sandbox_adapter_id(handle.adapter_id.as_str().to_string())
            .with_sandbox_internal_id(handle.sandbox_internal_id.clone())
            .with_sandbox_capabilities_snapshot(capabilities_snapshot(&self.inner.capabilities()));
        self.ledger.record_start(start.clone()).map_err(|error| {
            SandboxAdapterError::SpawnFailed {
                adapter_id: handle.adapter_id.clone(),
                reason: format!("process ledger START write failed (restore): {error}"),
            }
        })?;
        self.starts
            .lock()
            .map_err(|error| SandboxAdapterError::SpawnFailed {
                adapter_id: handle.adapter_id.clone(),
                reason: format!("ledger decorator state poisoned: {error}"),
            })?
            .insert(handle.id, start);
        Ok(handle)
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.inner.capabilities()
    }
}

fn signal_name(signal: Signal) -> &'static str {
    match signal {
        Signal::Term => "term",
        Signal::Kill => "kill",
        Signal::Int => "int",
    }
}

fn capabilities_snapshot(capabilities: &AdapterCapabilities) -> serde_json::Value {
    serde_json::to_value(capabilities).unwrap_or_else(|_| {
        let mut fallback = BTreeMap::new();
        fallback.insert("adapter_id", capabilities.adapter_id.as_str().to_string());
        json!(fallback)
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::process_ledger::{LedgerBatcherConfig, NoopOverflowSink};

    use super::*;
    use crate::sandbox::{
        default_no_op_capabilities, AdapterId, IsolationStrength, IsolationTier, ThroughputClass,
    };

    #[derive(Default)]
    struct RecordingCopyAdapter {
        copied_in: Mutex<Vec<(Uuid, PathBuf, PathBuf)>>,
        copied_out: Mutex<Vec<(Uuid, PathBuf, PathBuf)>>,
    }

    impl RecordingCopyAdapter {
        fn unavailable(&self) -> SandboxAdapterError {
            SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new("copy-capable"),
                reason: "recording copy adapter only supports copy operations".to_string(),
            }
        }
    }

    #[async_trait]
    impl SandboxAdapter for RecordingCopyAdapter {
        async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn exec(
            &self,
            _handle: &ProcessHandle,
            _cmd: Command,
        ) -> Result<ExecResult, SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn fs_bind(
            &self,
            _handle: &ProcessHandle,
            _host_path: PathBuf,
            _guest_path: PathBuf,
            _mode: BindMode,
        ) -> Result<(), SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn copy_in(
            &self,
            handle: &ProcessHandle,
            host_path: PathBuf,
            guest_path: PathBuf,
        ) -> Result<(), SandboxAdapterError> {
            self.copied_in
                .lock()
                .expect("copy_in call ledger")
                .push((handle.id, host_path, guest_path));
            Ok(())
        }

        async fn copy_out(
            &self,
            handle: &ProcessHandle,
            guest_path: PathBuf,
            host_path: PathBuf,
        ) -> Result<(), SandboxAdapterError> {
            self.copied_out
                .lock()
                .expect("copy_out call ledger")
                .push((handle.id, guest_path, host_path));
            Ok(())
        }

        async fn net_policy(
            &self,
            _handle: &ProcessHandle,
            _policy: NetPolicy,
        ) -> Result<(), SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn kill(
            &self,
            _handle: &ProcessHandle,
            _signal: Signal,
        ) -> Result<(), SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn status(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<ProcessStatus, SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn exit_code(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<Option<i32>, SandboxAdapterError> {
            Err(self.unavailable())
        }

        fn capabilities(&self) -> AdapterCapabilities {
            let mut capabilities = default_no_op_capabilities();
            capabilities.adapter_id = AdapterId::new("copy-capable");
            capabilities.runtime_available = true;
            capabilities.filesystem_isolation_strength = IsolationStrength::Strong;
            capabilities.network_isolation_strength = IsolationStrength::Strong;
            capabilities.stdio_throughput_class = ThroughputClass::Medium;
            capabilities.isolation_tier = IsolationTier::Tier1Container;
            capabilities
        }
    }

    fn test_ledger() -> LedgerBatcher {
        let (batcher, _drain) = LedgerBatcher::manual_for_tests(
            LedgerBatcherConfig::default(),
            Arc::new(NoopOverflowSink),
        )
        .expect("manual ledger batcher");
        batcher
    }

    #[tokio::test]
    async fn copy_in_and_copy_out_delegate_through_ledger_decorator() {
        let inner = Arc::new(RecordingCopyAdapter::default());
        let adapter = LedgerDecorator::new(inner.clone(), test_ledger());
        let handle = ProcessHandle::new(AdapterId::new("copy-capable"), None, "copy-handle");

        adapter
            .copy_in(
                &handle,
                PathBuf::from("fixtures/input.txt"),
                PathBuf::from("/guest/input.txt"),
            )
            .await
            .expect("copy_in delegates");
        adapter
            .copy_out(
                &handle,
                PathBuf::from("/guest/output.txt"),
                PathBuf::from("fixtures/output.txt"),
            )
            .await
            .expect("copy_out delegates");

        assert_eq!(
            *inner.copied_in.lock().expect("copy_in calls"),
            vec![(
                handle.id,
                PathBuf::from("fixtures/input.txt"),
                PathBuf::from("/guest/input.txt")
            )]
        );
        assert_eq!(
            *inner.copied_out.lock().expect("copy_out calls"),
            vec![(
                handle.id,
                PathBuf::from("/guest/output.txt"),
                PathBuf::from("fixtures/output.txt")
            )]
        );
    }
}
