use std::time::Duration;

use thiserror::Error;
use tokio::{process::Command, time::timeout};

#[derive(Debug)]
pub struct TerminalOutput {
    pub stdout: String,
    pub stderr: String,
    pub status_code: i32,
}

#[derive(Error, Debug)]
pub enum TerminalError {
    #[error("Invalid terminal request: {0}")]
    Invalid(String),
    #[error("Terminal command failed: {0}")]
    Exec(String),
    #[error("Terminal command timed out after {0} ms")]
    Timeout(u64),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct TerminalService;

impl TerminalService {
    pub async fn run(
        program: &str,
        args: &[String],
        timeout_ms: Option<u64>,
    ) -> Result<TerminalOutput, TerminalError> {
        if program.trim().is_empty() {
            return Err(TerminalError::Invalid(
                "program is required for terminal execution".to_string(),
            ));
        }

        let mut command = Command::new(program);
        command.args(args);

        let effective_timeout = match timeout_ms {
            Some(ms) => ms,
            None => 30_000,
        };
        let duration = Duration::from_millis(effective_timeout);
        let output = timeout(duration, command.output())
            .await
            .map_err(|_| TerminalError::Timeout(duration.as_millis() as u64))??;

        let status_code = match output.status.code() {
            Some(code) => code,
            None => -1,
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(TerminalOutput {
            stdout,
            stderr,
            status_code,
        })
    }
}
