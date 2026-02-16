use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    connection_alive: Option<Arc<AtomicBool>>,
}

impl StdioTransport {
    pub fn new(cmd: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            cmd: cmd.into(),
            args,
            child: None,
            connection_alive: None,
        }
    }
}

struct ConnectionAliveDropGuard(Arc<AtomicBool>);

impl Drop for ConnectionAliveDropGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl McpTransport for StdioTransport {
    async fn connect(&mut self) -> McpResult<ConnectedTransport> {
        if let Some(mut existing) = self.child.take() {
            match existing.try_wait() {
                Ok(Some(_status)) => {
                    // child already exited; allow reconnect
                    self.connection_alive = None;
                }
                Ok(None) => {
                    let connection_severed = self
                        .connection_alive
                        .as_ref()
                        .map(|flag| !flag.load(Ordering::SeqCst))
                        .unwrap_or(false);
                    if connection_severed {
                        // pipes may be broken; kill + clear to allow reconnect
                        let _ = existing.start_kill();
                        self.connection_alive = None;
                    } else {
                        self.child = Some(existing);
                        return Err(McpError::Transport(
                            "StdioTransport already connected".to_string(),
                        ));
                    }
                }
                Err(e) => return Err(McpError::Transport(e.to_string())),
            }
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

        let connection_alive = Arc::new(AtomicBool::new(true));
        self.connection_alive = Some(Arc::clone(&connection_alive));

        let writer = tokio::spawn(async move {
            let _alive_guard = ConnectionAliveDropGuard(connection_alive);
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

        let connection_alive_reader = self
            .connection_alive
            .as_ref()
            .map(Arc::clone)
            .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
        let reader = tokio::spawn(async move {
            let _alive_guard = ConnectionAliveDropGuard(connection_alive_reader);
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
