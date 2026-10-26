use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use crate::bundles::redactor::SecretRedactor;
use crate::bundles::schemas::RedactionMode;

use super::errors::{McpError, McpResult};
use super::jsonrpc::{
    JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use super::transport::{ConnectedTransport, McpTransport, TransportTasks};

#[derive(Clone, Debug)]
pub struct PendingMeta {
    pub started_at: Instant,
    pub method: String,
    pub ctx: Option<super::gate::McpContext>,
    pub tool_name: Option<String>,
    pub capability_id: Option<String>,
}

/// Local observations only; a matched response does not prove process termination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpCallerState {
    Waiting,
    Completed,
    CallerRetired,
    TransportClosed,
    ClientDropped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpCancellationState {
    NotRequested,
    Enqueued,
    EnqueueFailed,
    NotPermittedInitialize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpRemoteObservation {
    Unknown,
    MatchedResponse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McpCallObservation {
    pub caller: McpCallerState,
    pub cancellation: McpCancellationState,
    pub remote: McpRemoteObservation,
}

/// An observation capability for one already-owned call, not a scoped lookup API.
/// After caller retirement, unmatched late responses cannot upgrade Unknown.
/// Transport enqueue does not establish dispatch, remote cancellation or child exit.
#[derive(Clone)]
pub struct McpCallObservationHandle(Arc<Mutex<McpCallObservation>>);

impl McpCallObservationHandle {
    pub fn snapshot(&self) -> McpResult<McpCallObservation> {
        self.0.lock().map(|state| *state)
            .map_err(|_| McpError::Transport("request observation lock error".into()))
    }

    fn update(&self, change: impl FnOnce(&mut McpCallObservation)) {
        let mut state = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        change(&mut state);
    }
}

struct PendingRequest {
    tx: oneshot::Sender<JsonRpcResponse>,
    meta: Option<PendingMeta>,
    observation: McpCallObservationHandle,
    cancellation_permitted: bool,
    response_observed: bool,
}

struct PendingRequests {
    open: bool,
    entries: HashMap<JsonRpcId, PendingRequest>,
}

impl PendingRequests {
    fn remove_owned(
        &mut self,
        id: &JsonRpcId,
        observation: &McpCallObservationHandle,
    ) -> Option<PendingRequest> {
        if self.entries.get(id).is_some_and(|entry|
            Arc::ptr_eq(&entry.observation.0, &observation.0)) {
            self.entries.remove(id)
        } else {
            None
        }
    }
}

#[async_trait]
pub trait McpDispatcher: Send + Sync {
    async fn handle_notification(&self, notification: JsonRpcNotification);
    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse;
    async fn handle_response(&self, meta: Option<PendingMeta>, response: &JsonRpcResponse);
}

pub struct JsonRpcMcpClient {
    outgoing: mpsc::UnboundedSender<JsonRpcMessage>,
    pending: Arc<Mutex<PendingRequests>>,
    next_id: AtomicI64,
    _transport_tasks: TransportTasks,
    _dispatcher_task: tokio::task::JoinHandle<()>,
}

impl JsonRpcMcpClient {
    pub async fn connect<T: McpTransport>(
        transport: &mut T,
        dispatcher: Arc<dyn McpDispatcher>,
    ) -> McpResult<Self> {
        let ConnectedTransport { io, tasks } = transport.connect().await?;
        let outgoing = io.outgoing;
        let mut incoming = io.incoming;

        let pending: Arc<Mutex<PendingRequests>> =
            Arc::new(Mutex::new(PendingRequests { open: true, entries: HashMap::new() }));
        let pending_for_task = Arc::clone(&pending);
        let outgoing_for_task = outgoing.clone();
        let redactor = SecretRedactor::new();

        let dispatcher_task = tokio::spawn(async move {
            while let Some(msg) = incoming.recv().await {
                match msg {
                    JsonRpcMessage::Response(mut resp) => {
                        let pending_entry = {
                            let mut guard = match pending_for_task.lock() {
                                Ok(g) => g,
                                Err(poisoned) => poisoned.into_inner(),
                            };
                            guard.entries.get_mut(&resp.id).and_then(|entry| {
                                if entry.response_observed { return None; }
                                entry.response_observed = true;
                                entry.observation.update(|state| {
                                    state.remote = McpRemoteObservation::MatchedResponse;
                                });
                                Some((entry.meta.clone(), entry.observation.clone()))
                            })
                        };

                        if let Some((meta, observation)) = pending_entry {
                            if let Some(meta) = meta.as_ref() {
                                if meta.method == "tools/call" {
                                    resp.result = resp.result.take().map(|v| {
                                        redactor
                                            .redact_value(
                                                &v,
                                                RedactionMode::SafeDefault,
                                                "mcp/tools_call/result",
                                            )
                                            .0
                                    });
                                    if let Some(error) = resp.error.as_mut() {
                                        let (msg, _) = redactor.redact_value(
                                            &Value::String(error.message.clone()),
                                            RedactionMode::SafeDefault,
                                            "mcp/tools_call/error/message",
                                        );
                                        if let Value::String(msg) = msg {
                                            error.message = msg;
                                        }
                                        error.data = error.data.take().map(|v| {
                                            redactor
                                                .redact_value(
                                                    &v,
                                                    RedactionMode::SafeDefault,
                                                    "mcp/tools_call/error/data",
                                                )
                                                .0
                                        });
                                    }
                                }
                            }
                            dispatcher.handle_response(meta, &resp).await;
                            let mut pending = pending_for_task.lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            if let Some(entry) = pending.remove_owned(&resp.id, &observation) {
                                // Keep terminal-cause custody under the admission lock until delivery.
                                let _ = entry.tx.send(resp);
                            }
                        } else {
                            dispatcher.handle_response(None, &resp).await;
                        }
                    }
                    JsonRpcMessage::Notification(notif) => {
                        dispatcher.handle_notification(notif).await;
                    }
                    JsonRpcMessage::Request(req) => {
                        let resp = dispatcher.handle_request(req).await;
                        let _ = outgoing_for_task.send(JsonRpcMessage::Response(resp));
                    }
                }
            }
            retire_pending(&pending_for_task, McpCallerState::TransportClosed);
        });

        Ok(Self {
            outgoing,
            pending,
            next_id: AtomicI64::new(1),
            _transport_tasks: tasks,
            _dispatcher_task: dispatcher_task,
        })
    }

    pub fn send_notification(
        &self,
        method: impl Into<String>,
        params: Option<Value>,
    ) -> McpResult<()> {
        let notif = JsonRpcNotification::new(method, params);
        self.outgoing
            .send(JsonRpcMessage::Notification(notif))
            .map_err(|e| McpError::Transport(e.to_string()))
    }

    pub fn send_request(
        &self,
        method: impl Into<String>,
        params: Option<Value>,
        meta: Option<PendingMeta>,
    ) -> McpResult<McpCall> {
        let id = JsonRpcId::Number(self.next_id.fetch_add(1, Ordering::SeqCst));
        self.send_request_with_id(id, method, params, meta)
    }

    pub fn reserve_progress_token(&self) -> String {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        format!("mcp-progress-{id}")
    }

    pub fn send_request_with_id(
        &self,
        id: JsonRpcId,
        method: impl Into<String>,
        params: Option<Value>,
        meta: Option<PendingMeta>,
    ) -> McpResult<McpCall> {
        let method_str = method.into();
        let request = JsonRpcRequest::new(id.clone(), method_str.clone(), params);

        let observation = McpCallObservationHandle(Arc::new(Mutex::new(McpCallObservation {
            caller: McpCallerState::Waiting,
            cancellation: McpCancellationState::NotRequested,
            remote: McpRemoteObservation::Unknown,
        })));
        let (tx, rx) = oneshot::channel::<JsonRpcResponse>();
        {
            let mut guard = self
                .pending
                .lock()
                .map_err(|_| McpError::Transport("pending map lock error".to_string()))?;
            if !guard.open {
                return Err(McpError::Transport("request admission closed".into()));
            }
            if guard.entries.contains_key(&id) {
                return Err(McpError::Protocol(format!(
                    "json-rpc request id already in use: {:?}",
                    id
                )));
            }
            guard.entries.insert(
                id.clone(),
                PendingRequest {
                    observation: observation.clone(),
                    cancellation_permitted: method_str != "initialize",
                    response_observed: false,
                    tx,
                    meta: meta.or_else(|| {
                        Some(PendingMeta {
                            started_at: Instant::now(), // WAIVER [CX-573E] duration/timeout bookkeeping only
                            method: method_str.clone(),
                            ctx: None,
                            tool_name: None,
                            capability_id: None,
                        })
                    }),
                },
            );
            if let Err(error) = self.outgoing.send(JsonRpcMessage::Request(request)) {
                guard.remove_owned(&id, &observation);
                return Err(McpError::Transport(error.to_string()));
            }
        }

        Ok(McpCall::new(
            id,
            rx,
            self.outgoing.clone(),
            Arc::clone(&self.pending),
            observation,
        ))
    }
}

fn retire_pending(
    pending: &Mutex<PendingRequests>,
    reason: McpCallerState,
) {
    let mut pending = pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.open = false;
    for (_, entry) in pending.entries.drain() {
        entry.observation.update(|state| state.caller = reason);
        // Dropping the sender completes the local waiter; remote work remains unknown.
    }
}

impl Drop for JsonRpcMcpClient {
    fn drop(&mut self) {
        self._dispatcher_task.abort();
        retire_pending(&self.pending, McpCallerState::ClientDropped);
    }
}

pub struct McpCall {
    id: JsonRpcId,
    rx: oneshot::Receiver<JsonRpcResponse>,
    cancel_sender: mpsc::UnboundedSender<JsonRpcMessage>,
    pending: Arc<Mutex<PendingRequests>>,
    completed: bool,
    observation: McpCallObservationHandle,
}

impl McpCall {
    fn new(
        id: JsonRpcId,
        rx: oneshot::Receiver<JsonRpcResponse>,
        cancel_sender: mpsc::UnboundedSender<JsonRpcMessage>,
        pending: Arc<Mutex<PendingRequests>>,
        observation: McpCallObservationHandle,
    ) -> Self {
        Self {
            id,
            rx,
            cancel_sender,
            pending,
            completed: false,
            observation,
        }
    }

    pub fn observation_handle(&self) -> McpCallObservationHandle {
        self.observation.clone()
    }
}

impl Drop for McpCall {
    fn drop(&mut self) {
        if self.completed {
            return;
        }

        let mut pending = self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(entry) = pending.remove_owned(&self.id, &self.observation) else {
            drop(pending);
            self.observation.update(|state| {
                if state.caller == McpCallerState::Waiting {
                    state.caller = McpCallerState::CallerRetired;
                }
            });
            return;
        };
        let cancellation = if entry.response_observed {
            McpCancellationState::NotRequested
        } else if !entry.cancellation_permitted {
            McpCancellationState::NotPermittedInitialize
        } else if self.cancel_sender.send(JsonRpcMessage::Notification(
            JsonRpcNotification::cancelled(self.id.clone()),
        )).is_ok() {
            McpCancellationState::Enqueued
        } else {
            McpCancellationState::EnqueueFailed
        };
        // A reused ID may be admitted only after its old cancellation is enqueued.
        drop(pending);
        self.observation.update(|state| {
            state.caller = McpCallerState::CallerRetired;
            state.cancellation = cancellation;
            // Enqueue success is not remote cancellation acknowledgement.
        });
    }
}

impl Future for McpCall {
    type Output = McpResult<Value>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let rx = Pin::new(&mut self.rx);
        match rx.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.completed = true;
                self.observation.update(|state| {
                    if state.caller == McpCallerState::Waiting {
                        state.caller = if result.is_ok() { McpCallerState::Completed }
                            else { McpCallerState::TransportClosed };
                    }
                });
                match result {
                    Ok(resp) => match resp.into_result() {
                        Ok(v) => Poll::Ready(Ok(v)),
                        Err(e) => Poll::Ready(Err(McpError::Protocol(format!(
                            "json-rpc error {}: {}",
                            e.code, e.message
                        )))),
                    },
                    Err(e) => Poll::Ready(Err(McpError::Transport(e.to_string()))),
                }
            }
        }
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use super::super::transport::{TransportIo, TransportTasks};

    struct Dispatcher;
    #[async_trait]
    impl McpDispatcher for Dispatcher {
        async fn handle_notification(&self, _: JsonRpcNotification) {}
        async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
            JsonRpcResponse::ok(request.id, Value::Null)
        }
        async fn handle_response(&self, _: Option<PendingMeta>, _: &JsonRpcResponse) {}
    }

    struct ChannelTransport(Option<TransportIo>);
    #[async_trait]
    impl McpTransport for ChannelTransport {
        async fn connect(&mut self) -> McpResult<ConnectedTransport> {
            Ok(ConnectedTransport { io: self.0.take().unwrap(), tasks: TransportTasks::new(vec![]) })
        }
    }

    async fn client_with_dispatcher(dispatcher: Arc<dyn McpDispatcher>) -> (
        JsonRpcMcpClient, mpsc::UnboundedReceiver<JsonRpcMessage>, mpsc::UnboundedSender<JsonRpcMessage>,
    ) {
        let (outgoing, sent) = mpsc::unbounded_channel();
        let (responses, incoming) = mpsc::unbounded_channel();
        let mut transport = ChannelTransport(Some(TransportIo { outgoing, incoming }));
        (JsonRpcMcpClient::connect(&mut transport, dispatcher).await.unwrap(), sent, responses)
    }

    async fn client() -> (JsonRpcMcpClient, mpsc::UnboundedReceiver<JsonRpcMessage>,
        mpsc::UnboundedSender<JsonRpcMessage>) {
        client_with_dispatcher(Arc::new(Dispatcher)).await
    }

    #[tokio::test]
    async fn failed_send_releases_both_request_entrypoints() {
        let (client, sent, _responses) = client().await;
        drop(sent);
        assert!(client.send_request("tools/list", None, None).is_err());
        assert!(client.send_request_with_id(JsonRpcId::String("explicit".into()), "tools/list", None, None).is_err());
        assert!(client.pending.lock().unwrap().entries.is_empty());
    }

    #[tokio::test]
    async fn dropped_call_reports_only_enqueued_cancellation_and_unknown_remote() {
        let (client, mut sent, _responses) = client().await;
        let call = client.send_request("tools/call", None, None).unwrap();
        let observation = call.observation_handle();
        assert!(matches!(sent.recv().await, Some(JsonRpcMessage::Request(_))));
        drop(call);
        assert!(matches!(sent.recv().await, Some(JsonRpcMessage::Notification(notification))
            if notification.method == "notifications/cancelled"));
        assert_eq!(observation.snapshot().unwrap(), McpCallObservation {
            caller: McpCallerState::CallerRetired, cancellation: McpCancellationState::Enqueued,
            remote: McpRemoteObservation::Unknown,
        });
    }

    #[tokio::test]
    async fn initialize_drop_never_emits_protocol_invalid_cancellation() {
        let (client, mut sent, _responses) = client().await;
        let call = client.send_request("initialize", None, None).unwrap();
        let observation = call.observation_handle();
        assert!(matches!(sent.recv().await, Some(JsonRpcMessage::Request(_))));
        drop(call);
        assert!(matches!(sent.try_recv(), Err(mpsc::error::TryRecvError::Empty)));
        assert_eq!(observation.snapshot().unwrap().cancellation, McpCancellationState::NotPermittedInitialize);
        assert_eq!(observation.snapshot().unwrap().remote, McpRemoteObservation::Unknown);
    }

    #[tokio::test]
    async fn matched_response_is_observed_without_a_later_drop_cancellation() {
        let (client, mut sent, responses) = client().await;
        let call = client.send_request("tools/list", None, None).unwrap();
        let observation = call.observation_handle();
        let Some(JsonRpcMessage::Request(request)) = sent.recv().await else { panic!("request missing") };
        responses.send(JsonRpcMessage::Response(JsonRpcResponse::ok(request.id, serde_json::json!({"tools":[]})))).unwrap();
        assert_eq!(call.await.unwrap(), serde_json::json!({"tools":[]}));
        assert_eq!(observation.snapshot().unwrap().remote, McpRemoteObservation::MatchedResponse);
        assert_eq!(observation.snapshot().unwrap().caller, McpCallerState::Completed);
        assert_eq!(observation.snapshot().unwrap().cancellation, McpCancellationState::NotRequested);
        assert!(matches!(sent.try_recv(), Err(mpsc::error::TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn eof_and_client_destruction_retire_waiters_without_remote_settlement() {
        let (first, _sent, responses) = client().await;
        let call = first.send_request("tools/list", None, None).unwrap();
        let observation = call.observation_handle();
        drop(responses);
        assert!(call.await.is_err());
        assert_eq!(observation.snapshot().unwrap().caller, McpCallerState::TransportClosed);
        assert_eq!(observation.snapshot().unwrap().remote, McpRemoteObservation::Unknown);
        assert!(first.pending.lock().unwrap().entries.is_empty());
        let (second, _sent, _responses) = client().await;
        let call = second.send_request("tools/list", None, None).unwrap();
        let observation = call.observation_handle();
        drop(second);
        assert!(call.await.is_err());
        assert_eq!(observation.snapshot().unwrap().caller, McpCallerState::ClientDropped);
        assert_eq!(observation.snapshot().unwrap().remote, McpRemoteObservation::Unknown);
    }

    #[tokio::test]
    async fn eof_atomically_closes_both_request_admission_paths() {
        let (client, mut sent, responses) = client().await;
        let call = client.send_request("tools/list", None, None).unwrap();
        assert!(matches!(sent.recv().await, Some(JsonRpcMessage::Request(_))));
        drop(responses);
        assert!(call.await.is_err());
        assert!(matches!(client.send_request("tools/list", None, None), Err(McpError::Transport(_))));
        assert!(matches!(client.send_request_with_id(JsonRpcId::Number(50), "tools/list", None, None), Err(McpError::Transport(_))));
        assert!(matches!(sent.try_recv(), Err(mpsc::error::TryRecvError::Empty)));
    }

    struct DeliveryBarrier(Arc<tokio::sync::Notify>);
    #[async_trait]
    impl McpDispatcher for DeliveryBarrier {
        async fn handle_notification(&self, _: JsonRpcNotification) { self.0.notify_one(); }
        async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
            JsonRpcResponse::ok(request.id, Value::Null)
        }
        async fn handle_response(&self, _: Option<PendingMeta>, _: &JsonRpcResponse) {}
    }

    #[tokio::test]
    async fn unpolled_old_call_drop_cannot_remove_or_cancel_reused_id() {
        let delivered = Arc::new(tokio::sync::Notify::new());
        let (client, mut sent, responses) = client_with_dispatcher(Arc::new(DeliveryBarrier(delivered.clone()))).await;
        let id = JsonRpcId::String("reused".into());
        let old = client.send_request_with_id(id.clone(), "tools/list", None, None).unwrap();
        let old_observation = old.observation_handle();
        assert!(matches!(sent.recv().await, Some(JsonRpcMessage::Request(_))));
        responses.send(JsonRpcMessage::Response(JsonRpcResponse::ok(id.clone(), Value::Null))).unwrap();
        responses.send(JsonRpcMessage::Notification(JsonRpcNotification::new("test/delivery-barrier", None))).unwrap();
        delivered.notified().await;
        let new = client.send_request_with_id(id.clone(), "tools/list", None, None).unwrap();
        let new_observation = new.observation_handle();
        assert!(matches!(sent.recv().await, Some(JsonRpcMessage::Request(_))));
        drop(old);
        assert_eq!(old_observation.snapshot().unwrap().caller, McpCallerState::CallerRetired);
        assert_eq!(old_observation.snapshot().unwrap().remote, McpRemoteObservation::MatchedResponse);
        assert_eq!(old_observation.snapshot().unwrap().cancellation, McpCancellationState::NotRequested);
        assert!(matches!(sent.try_recv(), Err(mpsc::error::TryRecvError::Empty)));
        assert_eq!(new_observation.snapshot().unwrap().caller, McpCallerState::Waiting);
        responses.send(JsonRpcMessage::Response(JsonRpcResponse::ok(id, serde_json::json!({"new":true})))).unwrap();
        assert_eq!(new.await.unwrap(), serde_json::json!({"new":true}));
        assert_eq!(new_observation.snapshot().unwrap().caller, McpCallerState::Completed);
    }

    struct BlockResponse {
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    }
    #[async_trait]
    impl McpDispatcher for BlockResponse {
        async fn handle_notification(&self, _: JsonRpcNotification) {}
        async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
            JsonRpcResponse::ok(request.id, Value::Null)
        }
        async fn handle_response(&self, _: Option<PendingMeta>, _: &JsonRpcResponse) {
            self.entered.notify_one();
            self.release.notified().await;
        }
    }

    #[tokio::test]
    async fn client_drop_keeps_terminal_cause_during_blocked_response_callback() {
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let (client, mut sent, responses) = client_with_dispatcher(Arc::new(BlockResponse {
            entered: entered.clone(), release,
        })).await;
        let call = client.send_request("tools/list", None, None).unwrap();
        let observation = call.observation_handle();
        let Some(JsonRpcMessage::Request(request)) = sent.recv().await else { panic!("request missing") };
        responses.send(JsonRpcMessage::Response(JsonRpcResponse::ok(request.id, Value::Null))).unwrap();
        entered.notified().await;
        assert_eq!(observation.snapshot().unwrap().remote, McpRemoteObservation::MatchedResponse);
        assert_eq!(observation.snapshot().unwrap().caller, McpCallerState::Waiting);
        drop(client);
        assert!(call.await.is_err());
        assert_eq!(observation.snapshot().unwrap().caller, McpCallerState::ClientDropped);
        assert_eq!(observation.snapshot().unwrap().remote, McpRemoteObservation::MatchedResponse);
        assert_eq!(observation.snapshot().unwrap().cancellation, McpCancellationState::NotRequested);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_retirement_enqueues_cancel_before_reused_id_request() {
        let (client, mut sent, responses) = client().await;
        let client = Arc::new(client);
        // Repeat the exact concurrent public operation, without sleeps or child processes.
        for index in 0..64 {
            let id = JsonRpcId::Number(index);
            let old = client.send_request_with_id(id.clone(), "tools/call", None, None).unwrap();
            let old_observation = old.observation_handle();
            assert!(matches!(sent.recv().await, Some(JsonRpcMessage::Request(request)) if request.id == id));
            let start = Arc::new(tokio::sync::Barrier::new(2));
            let retire_start = start.clone();
            let retirement = tokio::spawn(async move {
                retire_start.wait().await;
                drop(old);
            });
            let next_client = client.clone();
            let next_id = id.clone();
            let readmission = tokio::spawn(async move {
                start.wait().await;
                loop {
                    match next_client.send_request_with_id(next_id.clone(), "tools/list", None, None) {
                        Ok(call) => break call,
                        Err(McpError::Protocol(_)) => tokio::task::yield_now().await,
                        Err(error) => panic!("unexpected readmission failure: {error}"),
                    }
                }
            });
            retirement.await.unwrap();
            let new = readmission.await.unwrap();
            let new_observation = new.observation_handle();
            let Some(JsonRpcMessage::Notification(cancel)) = sent.recv().await else {
                panic!("reused-ID request overtook its old cancellation")
            };
            assert_eq!(cancel.method, "notifications/cancelled");
            assert_eq!(cancel.params.as_ref().and_then(|params| params.get("requestId")), Some(&id.to_value()));
            assert!(matches!(sent.recv().await, Some(JsonRpcMessage::Request(request)) if request.id == id));
            assert_eq!(old_observation.snapshot().unwrap().cancellation, McpCancellationState::Enqueued);
            assert_eq!(old_observation.snapshot().unwrap().remote, McpRemoteObservation::Unknown);
            assert_eq!(new_observation.snapshot().unwrap().caller, McpCallerState::Waiting);
            responses.send(JsonRpcMessage::Response(JsonRpcResponse::ok(id, serde_json::json!({"new":true})))).unwrap();
            assert_eq!(new.await.unwrap(), serde_json::json!({"new":true}));
            assert_eq!(new_observation.snapshot().unwrap().cancellation, McpCancellationState::NotRequested);
            assert!(matches!(sent.try_recv(), Err(mpsc::error::TryRecvError::Empty)));
        }
    }
}
