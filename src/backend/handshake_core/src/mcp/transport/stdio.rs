use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use super::{ConnectedTransport, McpTransport, TransportIo, TransportTasks};
use crate::mcp::errors::{McpError, McpResult};
use crate::mcp::jsonrpc::JsonRpcMessage;

pub struct StdioTransport {
    cmd: String,
    args: Vec<String>,
    child: Option<Child>,
}

impl StdioTransport {
    pub fn new(cmd: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            cmd: cmd.into(),
            args,
            child: None,
        }
    }
}

#[async_trait::async_trait]
impl McpTransport for StdioTransport {
    async fn connect(&mut self) -> McpResult<ConnectedTransport> {
        if self.child.is_some() {
            return Err(McpError::Transport(
                "StdioTransport already connected".to_string(),
            ));
        }

        let mut command = Command::new(&self.cmd);
        command
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = command
            .spawn()
            .map_err(|e| McpError::Transport(e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("child stdin missing".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("child stdout missing".to_string()))?;

        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<JsonRpcMessage>();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<JsonRpcMessage>();

        let writer = tokio::spawn(async move {
            let mut writer = BufWriter::new(stdin);
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
            let mut lines = BufReader::new(stdout).lines();
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

        self.child = Some(child);
        Ok(ConnectedTransport {
            io: TransportIo {
                outgoing: outgoing_tx,
                incoming: incoming_rx,
            },
            tasks: TransportTasks::new(vec![writer, reader]),
        })
    }
}
