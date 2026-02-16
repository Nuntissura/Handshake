use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, DuplexStream};
use tokio::sync::mpsc;

use super::{ConnectedTransport, McpTransport, TransportIo, TransportTasks};
use crate::mcp::errors::{McpError, McpResult};
use crate::mcp::jsonrpc::JsonRpcMessage;

pub struct DuplexTransport {
    stream: Option<DuplexStream>,
}

impl DuplexTransport {
    pub fn new(stream: DuplexStream) -> Self {
        Self {
            stream: Some(stream),
        }
    }
}

#[async_trait::async_trait]
impl McpTransport for DuplexTransport {
    async fn connect(&mut self) -> McpResult<ConnectedTransport> {
        let stream = self
            .stream
            .take()
            .ok_or_else(|| McpError::Transport("DuplexTransport already connected".to_string()))?;

        let (read_half, write_half) = tokio::io::split(stream);
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<JsonRpcMessage>();

        let writer = tokio::spawn(async move {
            let mut writer = BufWriter::new(write_half);
            while let Some(msg) = outgoing_rx.recv().await {
                let line = match serde_json::to_string(&msg) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
                if writer.flush().await.is_err() {
                    break;
                }
            }
        });

        let reader = tokio::spawn(async move {
            let mut lines = BufReader::new(read_half).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let msg = match serde_json::from_str::<JsonRpcMessage>(&line) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if incoming_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        Ok(ConnectedTransport {
            io: TransportIo {
                outgoing: outgoing_tx,
                incoming: incoming_rx,
            },
            tasks: TransportTasks::new(vec![writer, reader]),
        })
    }
}
