//! Backend HTTP client. Reuses the EXISTING handshake_core backend over its real HTTP API
//! (GET /health) — the native app never starts or embeds the backend; it assumes it is running.
//! Deserializes via serde_json::Value to avoid a build dependency on the handshake_core crate.

use crate::error::AppError;
use std::time::Duration;

/// handshake_core listens here (hardcoded in handshake_core/src/main.rs).
pub const HEALTH_URL: &str = "http://127.0.0.1:37501/health";

#[derive(Debug, Clone)]
pub struct HealthInfo {
    pub status: String,
    pub db_status: String,
    pub migration_version: Option<i64>,
}

/// GET /health with a 5s timeout (CONTROL-2). Non-success status or a parse failure is an error,
/// never a panic.
pub async fn fetch_health(url: &str) -> Result<HealthInfo, AppError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("non-success status {}", resp.status())));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    Ok(HealthInfo {
        status: v.get("status").and_then(|x| x.as_str()).unwrap_or("unknown").to_string(),
        db_status: v.get("db_status").and_then(|x| x.as_str()).unwrap_or("unknown").to_string(),
        migration_version: v.get("migration_version").and_then(|x| x.as_i64()),
    })
}
