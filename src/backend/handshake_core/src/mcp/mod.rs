pub mod client;
pub mod discovery;
pub mod errors;
pub mod fr_events;
pub mod gate;
pub mod jsonrpc;
pub mod schema;
pub mod security;
pub mod transport;

pub use client::{JsonRpcMcpClient, PendingMeta};
pub use discovery::{McpResourceDescriptor, McpToolDescriptor};
pub use errors::{McpError, McpResult};
pub use gate::{
    ConsentDecision, ConsentProvider, GateConfig, GatedMcpClient, McpContext, ToolPolicy,
};
