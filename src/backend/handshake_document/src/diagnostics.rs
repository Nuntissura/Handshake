tokio::task_local! { pub static DOCUMENT_PHASE_REQUEST_ID: uuid::Uuid; }

// Correlation is server-generated; the lower observer logs fixed labels/timings only.
pub async fn observe_document_phase<T>(
    phase: &'static str,
    future: impl std::future::Future<Output = T>,
    failed: impl FnOnce(&T) -> bool,
) -> T {
    use handshake_storage_support::diagnostics::{observe, DOCUMENT_REQUEST_ID};
    if DOCUMENT_REQUEST_ID.try_with(|_| ()).is_ok() {
        return observe(phase, 0, future, failed).await;
    }
    match DOCUMENT_PHASE_REQUEST_ID.try_with(ToString::to_string) {
        Ok(request_id) => {
            DOCUMENT_REQUEST_ID
                .scope(request_id, observe(phase, 0, future, failed))
                .await
        }
        Err(_) => future.await,
    }
}
