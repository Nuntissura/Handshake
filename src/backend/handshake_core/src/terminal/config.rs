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
    pub const SESSION_SCOPED_DENIED_COMMAND_PATTERNS: [&str; 4] = [
        r"(?i)\bgit\s+reset\s+--hard\b",
        r"(?i)\bgit\s+clean\s+-fd\b",
        r"(?i)\brm\s+(-[^\s]+\s+)*-rf\b",
        r"(?i)\.handshake[\\/]+gov",
    ];

    pub fn with_session_scoped_denies(session_id: Option<&str>) -> Self {
        let mut cfg = Self::with_defaults();
        if !session_id.map(str::trim).unwrap_or("").is_empty() {
            cfg.denied_command_patterns = Self::session_denied_command_patterns();
        }
        cfg
    }

    pub fn with_session_scoped_denies_and_allowed_roots(
        session_id: Option<&str>,
        allowed_cwd_roots: Vec<PathBuf>,
    ) -> Self {
        let mut cfg = Self::with_session_scoped_denies(session_id);
        cfg.allowed_cwd_roots = allowed_cwd_roots;
        cfg
    }

    pub fn session_denied_command_patterns() -> Vec<String> {
        Self::SESSION_SCOPED_DENIED_COMMAND_PATTERNS
            .iter()
            .map(|pattern| pattern.to_string())
            .collect()
    }

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

#[cfg(test)]
mod tests {
    use super::TerminalConfig;
    use std::path::PathBuf;

    #[test]
    fn with_session_scoped_denies_injects_patterns() {
        let cfg = TerminalConfig::with_session_scoped_denies(Some("session-123"));
        assert_eq!(cfg.denied_command_patterns, TerminalConfig::session_denied_command_patterns());
    }

    #[test]
    fn with_session_scoped_denies_injects_allowed_roots() {
        let cfg = TerminalConfig::with_session_scoped_denies_and_allowed_roots(
            Some("session-123"),
            vec![PathBuf::from("src"), PathBuf::from("tests")],
        );
        assert_eq!(
            cfg.allowed_cwd_roots,
            vec![PathBuf::from("src"), PathBuf::from("tests")]
        );
    }
}
