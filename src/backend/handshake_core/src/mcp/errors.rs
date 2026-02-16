use thiserror::Error;

pub type McpResult<T> = Result<T, McpError>;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("HSK-MCP-500-TRANSPORT: {0}")]
    Transport(String),
    #[error("HSK-MCP-400-PROTOCOL: {0}")]
    Protocol(String),
    #[error("HSK-MCP-400-JSON: {0}")]
    Json(String),
    #[error("HSK-MCP-400-SCHEMA: {details}")]
    SchemaValidation { details: String },
    #[error("HSK-MCP-403-CAPABILITY: {0}")]
    CapabilityDenied(String),
    #[error("HSK-MCP-403-CONSENT: {0}")]
    ConsentDenied(String),
    #[error("HSK-MCP-408-TIMEOUT: {0}")]
    Timeout(String),
    #[error("HSK-MCP-403-SECURITY: {0}")]
    SecurityViolation(String),
    #[error("HSK-MCP-404-TOOL: {0}")]
    UnknownTool(String),
    #[error("HSK-MCP-500-FLIGHT-RECORDER: {0}")]
    FlightRecorder(String),
}

impl From<serde_json::Error> for McpError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}
