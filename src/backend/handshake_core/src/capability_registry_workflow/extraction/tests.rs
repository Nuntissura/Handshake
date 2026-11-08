use super::*;
use crate::llm::{LlmClient, ModelProfile, TokenUsage};
use crate::model_runtime::{activity::RuntimeActivityTracker, CancellationToken, ModelId};
use async_trait::async_trait;
use tokio::sync::Notify;

struct Worker {
    release: Option<std::sync::mpsc::Sender<()>>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.release.take();
        if let Some(join) = self.join.take() {
            join.join().expect("owned test worker settled");
        }
    }
}

struct NativeClient {
    model: ModelId,
    profile: ModelProfile,
    tracker: RuntimeActivityTracker,
    worker: Mutex<Option<Worker>>,
    token: Mutex<Option<CancellationToken>>,
    started: Notify,
    finished: Arc<Notify>,
}

impl NativeClient {
    fn new() -> Arc<Self> {
        let model = ModelId::new_v7();
        Arc::new(Self {
            model,
            profile: ModelProfile::new(model.to_string(), 4096),
            tracker: RuntimeActivityTracker::new(),
            worker: Mutex::new(None),
            token: Mutex::new(None),
            started: Notify::new(),
            finished: Arc::new(Notify::new()),
        })
    }

    fn finish_worker(&self) {
        drop(self.worker.lock().unwrap().take());
    }

    fn assert_cancelled_but_running(&self) {
        assert!(self.token.lock().unwrap().as_ref().unwrap().is_cancelled());
        assert_eq!(self.tracker.active_operations().len(), 1);
    }
}

#[async_trait]
impl LlmClient for NativeClient {
    async fn completion(
        &self,
        _: CompletionRequest,
    ) -> std::result::Result<CompletionResponse, LlmError> {
        panic!("the test must use the owned invocation entrypoint")
    }

    async fn completion_with_control(
        &self,
        _: CompletionRequest,
        control: LlmInvocationControl,
    ) -> std::result::Result<CompletionResponse, LlmError> {
        let (token, guard) = control
            .register_native_generation(&self.tracker, self.model)
            .map_err(|error| LlmError::ProviderError(error.to_string()))?;
        *self.token.lock().unwrap() = Some(token);
        let (release, wait) = std::sync::mpsc::channel();
        let finished = self.finished.clone();
        let join = std::thread::spawn(move || {
            let _ = wait.recv();
            drop(guard);
            finished.notify_one();
        });
        {
            let mut worker = self.worker.lock().unwrap();
            assert!(worker.is_none(), "one worker per test invocation");
            *worker = Some(Worker {
                release: Some(release),
                join: Some(join),
            });
        }
        self.started.notify_one();
        self.finished.notified().await;
        Ok(CompletionResponse {
            text: "component fixture response".into(),
            usage: TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn invocation(
    client: &Arc<NativeClient>,
    timeout_ms: u64,
) -> (crate::llm::LlmCompletionFuture, LlmInvocationControl) {
    client
        .clone()
        .owned_completion(
            CompletionRequest::new(
                Uuid::now_v7(),
                "component lifecycle assertion".into(),
                client.model.to_string(),
            ),
            invocation_deadline(timeout_ms).unwrap(),
        )
        .into_parts()
}

async fn start(client: &NativeClient, future: &mut crate::llm::LlmCompletionFuture) {
    tokio::select! {
        biased;
        _ = client.started.notified() => {}
        result = future => panic!("invocation ended before owned worker observation: {result:?}"),
    }
}

#[tokio::test]
async fn extraction_success_releases_job_only_after_observed_worker_settlement() {
    let client = NativeClient::new();
    let registry = Arc::new(InvocationRegistry::default());
    let job = Uuid::now_v7();
    let (mut future, control) = invocation(&client, 5_000);
    let lease = registry
        .retain(job, control.clone(), json!({"job": job}))
        .unwrap();
    start(&client, &mut future).await;
    assert_eq!(registry.retained.lock().unwrap().len(), 1);
    assert_eq!(control.resource_settlements().len(), 1);
    assert!(control.resource_settlements()[0].1.is_none());
    client.finish_worker();
    let response = future.await;
    assert!(response.is_ok());
    let drain = control.drain().await;
    assert!(matches!(
        drain,
        InvocationDrainOutcome::LocalResourcesSettled {
            deadline_exceeded: false,
            ..
        }
    ));
    {
        let mut observed = lease.invocation.observation.lock().unwrap();
        observed.completion = Some(Arc::new(response));
        observed.drain = Some(drain);
    }
    lease.release().unwrap();
    assert!(registry.retained.lock().unwrap().is_empty());
    assert!(client.tracker.active_operations().is_empty());
}

#[tokio::test]
async fn extraction_deadline_keeps_unsettled_job_and_rejects_replacement() {
    let client = NativeClient::new();
    let registry = Arc::new(InvocationRegistry::default());
    let job = Uuid::now_v7();
    let provenance = json!({"job": job, "source": "original"});
    let (mut future, control) = invocation(&client, 200);
    let lease = registry
        .retain(job, control.clone(), provenance.clone())
        .unwrap();
    start(&client, &mut future).await;
    let completion = future.await;
    assert!(completion
        .as_ref()
        .unwrap_err()
        .to_string()
        .contains("deadline expired"));
    client.assert_cancelled_but_running();
    let drain = control.drain().await;
    assert!(matches!(drain, InvocationDrainOutcome::Unknown { .. }));
    {
        let mut observed = lease.invocation.observation.lock().unwrap();
        observed.completion = Some(Arc::new(completion));
        observed.drain = Some(drain);
    }
    assert!(lease.release().is_err());
    assert!(registry
        .retain(job, control.clone(), json!({"source": "replacement"}))
        .is_err());
    let retained = registry.retained.lock().unwrap().get(&job).unwrap().clone();
    assert_eq!(retained.job_id, job);
    let observed = retained.observation.lock().unwrap();
    assert!(observed.scope_ended_without_release);
    assert_eq!(observed.provenance, provenance);
    drop(observed);
    client.finish_worker();
    assert!(matches!(
        control.drain().await,
        InvocationDrainOutcome::LocalResourcesSettled {
            deadline_exceeded: true,
            ..
        }
    ));
    assert_eq!(
        registry.retained.lock().unwrap().len(),
        1,
        "settlement does not invent reconciliation or retry"
    );
}

#[tokio::test]
async fn extraction_scope_drop_cancels_worker_without_releasing_its_owner() {
    let client = NativeClient::new();
    let registry = Arc::new(InvocationRegistry::default());
    let job = Uuid::now_v7();
    let (mut future, control) = invocation(&client, 5_000);
    let lease = registry
        .retain(job, control.clone(), json!({"job": job}))
        .unwrap();
    start(&client, &mut future).await;
    drop(future);
    drop(lease);
    drop(control);
    client.assert_cancelled_but_running();
    let retained = registry.retained.lock().unwrap().get(&job).unwrap().clone();
    assert!(
        retained
            .observation
            .lock()
            .unwrap()
            .scope_ended_without_release
    );
    assert!(retained.control.resource_settlements()[0].1.is_none());
    client.finish_worker();
    assert!(matches!(
        retained.control.drain().await,
        InvocationDrainOutcome::LocalResourcesSettled { .. }
    ));
    assert_eq!(registry.retained.lock().unwrap().len(), 1);
}
