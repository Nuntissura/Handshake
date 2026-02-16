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

struct PendingRequest {
    tx: oneshot::Sender<JsonRpcResponse>,
    meta: Option<PendingMeta>,
}

#[async_trait]
pub trait McpDispatcher: Send + Sync {
    async fn handle_notification(&self, notification: JsonRpcNotification);
    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse;
    async fn handle_response(&self, meta: Option<PendingMeta>, response: &JsonRpcResponse);
}

pub struct JsonRpcMcpClient {
    outgoing: mpsc::UnboundedSender<JsonRpcMessage>,
    pending: Arc<Mutex<HashMap<JsonRpcId, PendingRequest>>>,
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

        let pending: Arc<Mutex<HashMap<JsonRpcId, PendingRequest>>> =
            Arc::new(Mutex::new(HashMap::new()));
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
                            guard.remove(&resp.id)
                        };

                        if let Some(entry) = pending_entry {
                            if let Some(meta) = entry.meta.as_ref() {
                                if meta.method == "tools/call" {
                                    resp.result = resp
                                        .result
                                        .take()
                                        .map(|v| redactor.redact_value(&v, RedactionMode::SafeDefault, "mcp/tools_call/result").0);
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
                                            redactor.redact_value(
                                                &v,
                                                RedactionMode::SafeDefault,
                                                "mcp/tools_call/error/data",
                                            ).0
                                        });
                                    }
                                }
                            }
                            dispatcher.handle_response(entry.meta.clone(), &resp).await;
                            let _ = entry.tx.send(resp);
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
        let method_str = method.into();
        let request = JsonRpcRequest::new(id.clone(), method_str.clone(), params);

        let (tx, rx) = oneshot::channel::<JsonRpcResponse>();
        {
            let mut guard = self
                .pending
                .lock()
                .map_err(|_| McpError::Transport("pending map lock error".to_string()))?;
            guard.insert(
                id.clone(),
                PendingRequest {
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
        }

        self.outgoing
            .send(JsonRpcMessage::Request(request))
            .map_err(|e| McpError::Transport(e.to_string()))?;

        Ok(McpCall::new(
            id,
            rx,
            self.outgoing.clone(),
            Arc::clone(&self.pending),
        ))
    }
}

pub struct McpCall {
    id: JsonRpcId,
    rx: oneshot::Receiver<JsonRpcResponse>,
    cancel_sender: mpsc::UnboundedSender<JsonRpcMessage>,
    pending: Arc<Mutex<HashMap<JsonRpcId, PendingRequest>>>,
    completed: bool,
}

impl McpCall {
    fn new(
        id: JsonRpcId,
        rx: oneshot::Receiver<JsonRpcResponse>,
        cancel_sender: mpsc::UnboundedSender<JsonRpcMessage>,
        pending: Arc<Mutex<HashMap<JsonRpcId, PendingRequest>>>,
    ) -> Self {
        Self {
            id,
            rx,
            cancel_sender,
            pending,
            completed: false,
        }
    }
}

impl Drop for McpCall {
    fn drop(&mut self) {
        if self.completed {
            return;
        }

        if let Ok(mut guard) = self.pending.lock() {
            guard.remove(&self.id);
        }

        let _ = self.cancel_sender.send(JsonRpcMessage::Notification(
            JsonRpcNotification::cancelled(self.id.clone()),
        ));
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
