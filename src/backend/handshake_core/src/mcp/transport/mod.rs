pub mod duplex;
pub mod reconnect;
pub mod stdio;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::mcp::errors::McpResult;
use crate::mcp::jsonrpc::JsonRpcMessage;

pub use reconnect::{AutoReconnectTransport, ReconnectConfig};

pub struct TransportIo {
    pub outgoing: mpsc::UnboundedSender<JsonRpcMessage>,
    pub incoming: mpsc::UnboundedReceiver<JsonRpcMessage>,
}

pub struct TransportTasks {
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl TransportTasks {
    pub fn new(handles: Vec<tokio::task::JoinHandle<()>>) -> Self {
        Self { handles }
    }
}

impl Drop for TransportTasks {
    fn drop(&mut self) {
        for handle in self.handles.drain(..) {
            handle.abort();
        }
    }
}

pub struct ConnectedTransport {
    pub io: TransportIo,
    pub tasks: TransportTasks,
}

#[async_trait]
pub trait McpTransport: Send {
    async fn connect(&mut self) -> McpResult<ConnectedTransport>;
}
