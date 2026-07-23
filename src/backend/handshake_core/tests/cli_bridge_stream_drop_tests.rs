use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use handshake_core::flight_recorder::events_llm_infer::{
    FR_EVT_LLM_INFER_END, FR_EVT_LLM_INFER_START,
};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::cloud::{
    AllowlistedCliBridgeConfig, CliBridgeConfig, CliBridgeModelRuntime, CliCancellationContext,
    CliInvocationContext, CliInvocationReceipt, CliKind, CliModelAllowlist, CliOutputFormat,
    CliSubprocessSpawner, CloudLaneObservability, OfficialCliBridgeError,
};
use handshake_core::model_runtime::{
    GenPrompt, GenerateRequest, KvCachePolicy, LoadSpec, ModelCapabilities, ModelId, ModelRuntime,
    ProviderKind, RuntimeKind, SamplingParams,
};
use handshake_core::sandbox::{IsolationTier, NetPolicy, RequiredCapability, TrustClass};

struct SilentCancelAwareSpawner {
    started: Arc<AtomicBool>,
    observed_cancel: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
}

#[derive(Default)]
struct CollectingRecorder {
    payloads: Mutex<Vec<serde_json::Value>>,
}

#[async_trait]
impl FlightRecorder for CollectingRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.payloads
            .lock()
            .expect("payload lock")
            .push(event.payload);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

impl CliSubprocessSpawner for SilentCancelAwareSpawner {
    fn spawn(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        unreachable!("streaming path required")
    }

    fn spawn_streaming_cancellable(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        _model_name: &str,
        _prompt: &str,
        _chunk_sender: &tokio::sync::mpsc::Sender<Vec<u8>>,
        cancellation: &CliCancellationContext,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        self.started.store(true, Ordering::SeqCst);
        while !cancellation.is_cancelled() {
            std::thread::sleep(Duration::from_millis(5));
        }
        self.observed_cancel.store(true, Ordering::SeqCst);
        self.finished.store(true, Ordering::SeqCst);
        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout: String::new(),
            pid: Some(42),
            exit_code: None,
            cancelled: true,
        })
    }
}

fn invocation_context() -> CliInvocationContext {
    let mut context = CliInvocationContext::new("TEST_ROLE", "test-model");
    context.owner_wp = Some("WP-TEST".to_string());
    context.role_id = Some("TEST_ROLE".to_string());
    context.wp_id = Some("WP-TEST".to_string());
    context.mt_id = Some("MT-003".to_string());
    context.session_id = Some("test-model#0".to_string());
    context.parent_session_id = Some("parent-test".to_string());
    context.trace_id = Some("trace-test".to_string());
    context.requested_trust_class = Some(TrustClass::Trusted);
    context.requested_isolation_tier = Some(IsolationTier::Tier1Container);
    context.requested_sandbox_capabilities =
        Some(BTreeSet::from([RequiredCapability::HighStdioThroughput]));
    context.requested_net_policy = Some(NetPolicy::HostInherited);
    context.requested_execution_policy_ref =
        Some(handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF.to_string());
    context.swarm_id = Some("stream-drop-swarm".to_string());
    context.worktree_id = Some("stream-drop-worktree".to_string());
    context
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_unpolled_silent_cli_stream_cancels_and_finishes_spawner() {
    let started = Arc::new(AtomicBool::new(false));
    let observed_cancel = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let spawner = Arc::new(SilentCancelAwareSpawner {
        started: Arc::clone(&started),
        observed_cancel: Arc::clone(&observed_cancel),
        finished: Arc::clone(&finished),
    });
    let config = CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: std::env::current_exe().expect("current executable"),
        args_template: vec!["{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    };
    let config = AllowlistedCliBridgeConfig::new(
        config,
        CliModelAllowlist::new(vec!["test-model".to_string()]).expect("test allowlist"),
    );
    let mut runtime =
        CliBridgeModelRuntime::new(spawner, config).with_invocation_context(invocation_context());
    let model_id = runtime
        .load(LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::OfficialCli,
            engine_origin: Some("test-model".to_string()),
            external_engine_import: None,
        })
        .await
        .expect("register official CLI model");

    let stream = runtime.generate(GenerateRequest {
        id: model_id,
        prompt: GenPrompt::new("remain silent"),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: Default::default(),
        max_tokens: 8,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    });

    tokio::time::timeout(Duration::from_secs(1), async {
        while !started.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("silent spawner starts");
    drop(stream);

    tokio::time::timeout(Duration::from_secs(1), async {
        while !finished.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("silent spawner finishes after stream drop");
    assert!(observed_cancel.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_polled_silent_cli_stream_emits_exactly_one_infer_end() {
    let started = Arc::new(AtomicBool::new(false));
    let observed_cancel = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let recorder = Arc::new(CollectingRecorder::default());
    let spawner = Arc::new(SilentCancelAwareSpawner {
        started: Arc::clone(&started),
        observed_cancel: Arc::clone(&observed_cancel),
        finished: Arc::clone(&finished),
    });
    let config = AllowlistedCliBridgeConfig::new(
        CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: std::env::current_exe().expect("current executable"),
            args_template: vec!["{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: HashMap::new(),
            working_dir: None,
            timeout_seconds: 30,
        },
        CliModelAllowlist::new(vec!["test-model".to_string()]).expect("test allowlist"),
    );
    let mut runtime = CliBridgeModelRuntime::new(spawner, config)
        .with_invocation_context(invocation_context())
        .with_lane_observability(Arc::new(CloudLaneObservability {
            flight_recorder: recorder.clone() as Arc<dyn FlightRecorder>,
            consent: None,
        }));
    let model_id = runtime
        .load(LoadSpec {
            artifact_path: PathBuf::new(),
            sha256_expected: String::new(),
            runtime_kind: RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::default(),
            declared_capabilities: ModelCapabilities::default(),
            provider: ProviderKind::OfficialCli,
            engine_origin: Some("test-model".to_string()),
            external_engine_import: None,
        })
        .await
        .expect("register official CLI model");
    let mut stream = runtime.generate(GenerateRequest {
        id: model_id,
        prompt: GenPrompt::new("remain silent after START"),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: Default::default(),
        max_tokens: 8,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    });

    tokio::time::timeout(Duration::from_millis(100), stream.next())
        .await
        .expect_err("silent stream remains pending after its first poll");
    drop(stream);

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let ids = recorder
                .payloads
                .lock()
                .expect("payload lock")
                .iter()
                .filter_map(|payload| payload.get("event_id").and_then(|value| value.as_str()))
                .map(str::to_string)
                .collect::<Vec<_>>();
            if ids.iter().any(|id| id == FR_EVT_LLM_INFER_END) && finished.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("drop emits END and cancels the silent spawner");

    let ids = recorder
        .payloads
        .lock()
        .expect("payload lock")
        .iter()
        .filter_map(|payload| payload.get("event_id").and_then(|value| value.as_str()))
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        ids.iter()
            .filter(|id| id.as_str() == FR_EVT_LLM_INFER_START)
            .count(),
        1
    );
    assert_eq!(
        ids.iter()
            .filter(|id| id.as_str() == FR_EVT_LLM_INFER_END)
            .count(),
        1
    );
    assert!(observed_cancel.load(Ordering::SeqCst));
}
