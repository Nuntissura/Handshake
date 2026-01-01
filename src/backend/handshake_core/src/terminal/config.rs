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
    pub allowed_cwd_roots: Vec<PathBuf>,
    pub allowed_command_patterns: Vec<String>,
    pub denied_command_patterns: Vec<String>,
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
            allowed_cwd_roots: Vec::new(),
            allowed_command_patterns: Vec::new(),
            denied_command_patterns: Vec::new(),
            redaction_enabled: true,
            logging_level: TerminalLogLevel::CommandsOnly,
        }
    }

    pub fn with_defaults() -> Self {
        let root = match env::current_dir() {
            Ok(path) => path,
            Err(_) => PathBuf::from("."),
        };
        Self::new(root)
    }

    pub fn effective_timeout(&self, requested: Option<u64>) -> u64 {
        match requested {
            Some(value) => value,
            None => self.default_timeout_ms,
        }
    }

    /// Resolves requested output limit, defaulting to configured max when omitted.
    pub fn effective_max_output(&self, requested: Option<u64>) -> u64 {
        match requested {
            Some(value) => value,
            None => self.max_output_bytes,
        }
    }
}
