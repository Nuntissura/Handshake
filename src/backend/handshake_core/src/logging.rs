use std::{fs, path::PathBuf};

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logging() {
    let root_dir = repo_root();
    let log_dir = root_dir.join("data").join("logs");
    let log_dir_result = fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::never(&log_dir, "handshake_core.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Keep guard alive for the lifetime of the process.
    Box::leak(Box::new(_guard));

    let level = std::env::var("HS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let filter_layer = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));

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
        tracing::error!(
            target: "handshake_core",
            "failed to create log directory {:?}: {}",
            log_dir,
            err
        );
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("failed to resolve repo root")
}
