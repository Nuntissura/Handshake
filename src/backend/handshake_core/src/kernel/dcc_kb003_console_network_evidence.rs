//! MT-074 Console and Network Evidence.
//!
//! Acceptance (MT-074.json): "GUI validation failures are diagnosable."
//!
//! Captures browser/app console messages and network exchanges in a typed
//! record set that makes GUI validation failures diagnosable from durable
//! records alone — no live devtools, no terminal scrollback. Each entry has
//! enough detail (status, url, console level, message) that a no-context
//! reviewer can identify the failure class and the request that caused it.
//!
//! Records are bounded; the runtime adapter trims at capture time, the
//! kernel record carries `truncated` + `original_count` so the
//! truncation is visible.
//!
//! Frontend renders via existing dcc-* IPC surface; no app/** edits.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Console log level mapped from browser/web-view runtimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConsoleLevel {
    Log,
    Info,
    Warning,
    Error,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleEntryV1 {
    pub level: ConsoleLevel,
    pub message: String,
    pub origin: Option<String>,
    pub recorded_at_utc: DateTime<Utc>,
}

impl ConsoleEntryV1 {
    pub fn new(level: ConsoleLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            origin: None,
            recorded_at_utc: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetworkExchangeOutcome {
    Success,
    HttpError,
    Aborted,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkExchangeV1 {
    pub method: String,
    pub url: String,
    pub status_code: Option<u16>,
    pub outcome: NetworkExchangeOutcome,
    pub started_at_utc: DateTime<Utc>,
    pub duration_ms: u64,
    pub error_detail: Option<String>,
}

impl NetworkExchangeV1 {
    pub fn success(method: impl Into<String>, url: impl Into<String>, status_code: u16, duration_ms: u64) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            status_code: Some(status_code),
            outcome: NetworkExchangeOutcome::Success,
            started_at_utc: Utc::now(),
            duration_ms,
            error_detail: None,
        }
    }
    pub fn http_error(method: impl Into<String>, url: impl Into<String>, status_code: u16, duration_ms: u64) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            status_code: Some(status_code),
            outcome: NetworkExchangeOutcome::HttpError,
            started_at_utc: Utc::now(),
            duration_ms,
            error_detail: None,
        }
    }
    pub fn blocked(method: impl Into<String>, url: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            status_code: None,
            outcome: NetworkExchangeOutcome::Blocked,
            started_at_utc: Utc::now(),
            duration_ms: 0,
            error_detail: Some(detail.into()),
        }
    }

    pub fn is_failure(&self) -> bool {
        matches!(
            self.outcome,
            NetworkExchangeOutcome::HttpError
                | NetworkExchangeOutcome::Aborted
                | NetworkExchangeOutcome::Blocked
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kb003ConsoleNetworkEvidenceV1 {
    pub schema_version: &'static str,
    pub sandbox_run_id: String,
    pub console_truncated: bool,
    pub console_original_count: usize,
    pub network_truncated: bool,
    pub network_original_count: usize,
    pub console: Vec<ConsoleEntryV1>,
    pub network: Vec<NetworkExchangeV1>,
}

impl Kb003ConsoleNetworkEvidenceV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.kb003_console_network_evidence@1";

    pub fn new(
        sandbox_run_id: impl Into<String>,
        console: Vec<ConsoleEntryV1>,
        network: Vec<NetworkExchangeV1>,
        console_cap: usize,
        network_cap: usize,
    ) -> Self {
        let co = console.len();
        let no = network.len();
        let console_trunc = co > console_cap;
        let net_trunc = no > network_cap;
        let console = if console_trunc { console.into_iter().take(console_cap).collect() } else { console };
        let network = if net_trunc { network.into_iter().take(network_cap).collect() } else { network };
        Self {
            schema_version: Self::SCHEMA_VERSION,
            sandbox_run_id: sandbox_run_id.into(),
            console_truncated: console_trunc,
            console_original_count: co,
            network_truncated: net_trunc,
            network_original_count: no,
            console,
            network,
        }
    }

    /// Diagnosability: a GUI validation failure can be diagnosed from this
    /// record when there is at least one error-class console entry or a
    /// failing network exchange. The check enforces that callers attach
    /// meaningful evidence rather than empty bundles when the GUI fails.
    pub fn has_failure_evidence(&self) -> bool {
        let any_console_error = self
            .console
            .iter()
            .any(|c| matches!(c.level, ConsoleLevel::Error | ConsoleLevel::Warning));
        let any_network_failure = self.network.iter().any(|n| n.is_failure());
        any_console_error || any_network_failure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_evidence_detected_from_console_error() {
        let ev = Kb003ConsoleNetworkEvidenceV1::new(
            "SBX-1",
            vec![ConsoleEntryV1::new(ConsoleLevel::Error, "uncaught TypeError")],
            vec![],
            100,
            100,
        );
        assert!(ev.has_failure_evidence());
    }

    #[test]
    fn failure_evidence_detected_from_network_5xx() {
        let ev = Kb003ConsoleNetworkEvidenceV1::new(
            "SBX-1",
            vec![],
            vec![NetworkExchangeV1::http_error("GET", "/api/x", 500, 42)],
            100,
            100,
        );
        assert!(ev.has_failure_evidence());
    }

    #[test]
    fn no_failure_evidence_when_clean() {
        let ev = Kb003ConsoleNetworkEvidenceV1::new(
            "SBX-1",
            vec![ConsoleEntryV1::new(ConsoleLevel::Info, "ok")],
            vec![NetworkExchangeV1::success("GET", "/api/x", 200, 12)],
            100,
            100,
        );
        assert!(!ev.has_failure_evidence());
    }

    #[test]
    fn bounded_capture_records_truncation() {
        let console: Vec<_> = (0..200)
            .map(|i| ConsoleEntryV1::new(ConsoleLevel::Info, format!("msg{i}")))
            .collect();
        let ev = Kb003ConsoleNetworkEvidenceV1::new("SBX-1", console, vec![], 50, 50);
        assert!(ev.console_truncated);
        assert_eq!(ev.console_original_count, 200);
        assert_eq!(ev.console.len(), 50);
    }

    #[test]
    fn blocked_network_exchange_is_failure() {
        let n = NetworkExchangeV1::blocked("GET", "https://x", "CSP blocked");
        assert!(n.is_failure());
        assert_eq!(n.outcome, NetworkExchangeOutcome::Blocked);
    }
}
