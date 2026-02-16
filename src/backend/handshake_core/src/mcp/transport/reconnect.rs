use std::collections::VecDeque;
use std::time::Duration;

use tokio::sync::mpsc;

use super::{ConnectedTransport, McpTransport, TransportIo, TransportTasks};
use crate::mcp::errors::{McpError, McpResult};
use crate::mcp::jsonrpc::JsonRpcMessage;

#[derive(Clone, Debug)]
pub struct ReconnectConfig {
    pub enabled: bool,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            max_attempts: None,
        }
    }
}

pub struct AutoReconnectTransport<T: McpTransport> {
    inner: Option<T>,
    config: ReconnectConfig,
}

impl<T: McpTransport> AutoReconnectTransport<T> {
    pub fn new(inner: T, config: ReconnectConfig) -> Self {
        Self {
            inner: Some(inner),
            config,
        }
    }
}

#[async_trait::async_trait]
impl<T: McpTransport + 'static> McpTransport for AutoReconnectTransport<T> {
    async fn connect(&mut self) -> McpResult<ConnectedTransport> {
        let mut inner = self.inner.take().ok_or_else(|| {
            McpError::Transport("AutoReconnectTransport already connected".to_string())
        })?;

        let connected = inner.connect().await?;

        let (stable_outgoing_tx, stable_outgoing_rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
        let (stable_incoming_tx, stable_incoming_rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
        let config = self.config.clone();

        let manager = tokio::spawn(async move {
            run_reconnector(
                inner,
                config,
                stable_outgoing_rx,
                stable_incoming_tx,
                connected,
            )
            .await;
        });

        Ok(ConnectedTransport {
            io: TransportIo {
                outgoing: stable_outgoing_tx,
                incoming: stable_incoming_rx,
            },
            tasks: TransportTasks::new(vec![manager]),
        })
    }
}

fn backoff_delay(config: &ReconnectConfig, attempt: u32) -> Duration {
    let base_ms = config.base_delay.as_millis().max(1);
    let max_ms = config.max_delay.as_millis().max(base_ms);
    let shift = attempt.min(63);
    let factor = 1u128 << shift;
    let ms = base_ms.saturating_mul(factor).min(max_ms);
    let ms_u64 = ms.min(u64::MAX as u128) as u64;
    Duration::from_millis(ms_u64)
}

async fn run_reconnector<T: McpTransport>(
    mut inner: T,
    config: ReconnectConfig,
    mut stable_outgoing_rx: mpsc::UnboundedReceiver<JsonRpcMessage>,
    stable_incoming_tx: mpsc::UnboundedSender<JsonRpcMessage>,
    mut connected: ConnectedTransport,
) {
    let mut backlog: VecDeque<JsonRpcMessage> = VecDeque::new();

    loop {
        let transport_outgoing = connected.io.outgoing;
        let mut transport_incoming = connected.io.incoming;
        let transport_tasks = connected.tasks;

        'connected: loop {
            while let Some(msg) = backlog.pop_front() {
                if let Err(err) = transport_outgoing.send(msg) {
                    backlog.push_front(err.0);
                    break 'connected;
                }
            }

            tokio::select! {
                msg_opt = stable_outgoing_rx.recv() => {
                    let Some(msg) = msg_opt else {
                        return;
                    };
                    if let Err(err) = transport_outgoing.send(msg) {
                        backlog.push_back(err.0);
                        break 'connected;
                    }
                }
                incoming_opt = transport_incoming.recv() => {
                    match incoming_opt {
                        Some(msg) => {
                            if stable_incoming_tx.send(msg).is_err() {
                                return;
                            }
                        }
                        None => break 'connected,
                    }
                }
            }
        };

        drop(transport_tasks);
        drop(transport_incoming);
        drop(transport_outgoing);

        if !config.enabled {
            return;
        }

        let mut attempt: u32 = 0;
        loop {
            if let Some(max_attempts) = config.max_attempts {
                if attempt >= max_attempts {
                    return;
                }
            }
            tokio::time::sleep(backoff_delay(&config, attempt)).await;
            match inner.connect().await {
                Ok(next_connected) => {
                    connected = next_connected;
                    break;
                }
                Err(_) => {
                    attempt = attempt.saturating_add(1);
                }
            }
        }
    }
}
