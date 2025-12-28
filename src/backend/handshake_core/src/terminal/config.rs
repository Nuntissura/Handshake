use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum TerminalLogLevel {
    CommandsOnly,
    CommandsPlusRedactedOutput,
}

#[derive(Clone, Debug)]
pub struct TerminalConfig {
    pub default_timeout_ms: u64,
    pub kill_grace_ms: u64,
    pub max_output_bytes: u64,
    pub workspace_root: PathBuf,
    pub redaction_enabled: bool,
    pub logging_level: TerminalLogLevel,
}

impl TerminalConfig {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            default_timeout_ms: 180_000,
            kill_grace_ms: 10_000,
            max_output_bytes: 1_500_000,
            workspace_root,
            redaction_enabled: true,
            logging_level: TerminalLogLevel::CommandsOnly,
        }
    }

    pub fn with_defaults() -> Self {
        let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::new(root)
    }

    pub fn effective_timeout(&self, requested: Option<u64>) -> u64 {
        requested.unwrap_or(self.default_timeout_ms)
    }

    /// Clamps requested output limit into the safe bounds (1MB - 2MB recommended).
    pub fn effective_max_output(&self, requested: Option<u64>) -> u64 {
        let requested = requested.unwrap_or(self.max_output_bytes);
        requested.clamp(1_000_000, 2_000_000)
    }
}
