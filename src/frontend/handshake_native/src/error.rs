//! Native GUI error type.

#[derive(Debug, Clone)]
pub enum AppError {
    /// HTTP transport / non-success status talking to the backend.
    Http(String),
    /// Response body could not be parsed.
    Parse(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Http(e) => write!(f, "http: {e}"),
            AppError::Parse(e) => write!(f, "parse: {e}"),
        }
    }
}

impl std::error::Error for AppError {}
