use handshake_core::backend_runtime::{RuntimeHandle, RuntimeOptions};

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(10 * 1024 * 1024)
        .build()
        .expect("Failed building the Runtime")
        .block_on(tokio::task::LocalSet::new().run_until(async {
            let options = match RuntimeOptions::from_env().await {
                Ok(options) => options,
                Err(error) => {
                    tracing::error!(error = %error, "handshake_core failed to start");
                    std::process::exit(1);
                }
            };
            let mut runtime = RuntimeHandle::start(options, None);
            tokio::select! {
                result = runtime.wait() => {
                    if let Err(error) = result {
                        tracing::error!(error = %error, "handshake_core runtime failed");
                        std::process::exit(1);
                    }
                    return;
                }
                _ = shutdown_signal() => runtime.request_shutdown(),
            }
            if let Err(error) = runtime.wait().await {
                tracing::error!(error = %error, "handshake_core runtime failed");
                std::process::exit(1);
            }
        }));
}

/// WP-1 MT-013 (F1 graceful shutdown): resolves on the first OS shutdown signal
/// so `axum::serve(...).with_graceful_shutdown(...)` returns cleanly, letting the
/// ordered teardown (embedded-model STOP emit + ledger drain + lease release +
/// embedded-storage shutdown) run instead of being OS-killed mid-flight. Awaits Ctrl-C on
/// all platforms and SIGTERM additionally on Unix.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(target: "handshake_core", error = %err, "failed to install Ctrl-C handler");
            // If the handler cannot be installed, never resolve on this arm.
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(err) => {
                tracing::error!(target: "handshake_core", error = %err, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!(target: "handshake_core", "shutdown signal received; beginning graceful shutdown");
}

