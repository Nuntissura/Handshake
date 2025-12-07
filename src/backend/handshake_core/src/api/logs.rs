use axum::{extract::Query, http::StatusCode, Json};
use serde::Deserialize;
use std::fs;

use crate::{api::paths::repo_root, models::ErrorResponse};

#[derive(Deserialize)]
pub struct TailParams {
    limit: Option<usize>,
}

#[derive(serde::Serialize)]
pub struct LogTailResponse {
    lines: Vec<String>,
}

pub async fn tail_logs(
    params: Query<TailParams>,
) -> Result<Json<LogTailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = params.limit.unwrap_or(200).min(1000);
    let log_path = repo_root()
        .join("data")
        .join("logs")
        .join("handshake_core.log");

    let contents = match fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Json(LogTailResponse { lines: Vec::new() }));
        }
        Err(err) => {
            tracing::error!(target: "handshake_core", route = "/logs/tail", error = %err, "failed to read log file");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "failed_to_read_logs",
                }),
            ));
        }
    };

    let mut lines: Vec<String> = contents.lines().map(|l| l.to_string()).collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }

    Ok(Json(LogTailResponse { lines }))
}
