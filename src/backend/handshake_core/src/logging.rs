use std::{fs, io, path::PathBuf};

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logging() {
    let root_dir = match repo_root() {
        Ok(path) => path,
        Err(err) => {
            tracing::error!(target: "handshake_core", error = %err, "failed to resolve repo root for logging");
            return;
        }
    };
    let log_dir = root_dir.join("data").join("logs");
    let log_dir_result = fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::never(&log_dir, "handshake_core.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Keep guard alive for the lifetime of the process.
    Box::leak(Box::new(_guard));

    let level = match std::env::var("HS_LOG_LEVEL") {
        Ok(val) => val,
        Err(_) => "info".to_string(),
    };
    let filter_layer = match EnvFilter::try_new(level) {
        Ok(layer) => layer,
        Err(_) => EnvFilter::new("info"),
    };

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_writer(non_blocking)
        .json();

    let stdout_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(stdout_layer)
        .init();

    if let Err(err) = log_dir_result {
        tracing::error!(target: "handshake_core", log_dir = %log_dir.display(), error = %err, "failed to create log directory");
    }
}

fn repo_root() -> Result<PathBuf, io::Error> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "failed to resolve repo root"))
}
