tokio::task_local! { pub static DOCUMENT_PHASE_REQUEST_ID: uuid::Uuid; }

// MT032-V11: correlation is generated here, never taken from a header or document. Observations
// contain fixed tags and timing only; the original future/result and error handling are unchanged.
pub async fn observe_document_phase<T>(
    phase: &'static str,
    future: impl std::future::Future<Output = T>,
    failed: impl FnOnce(&T) -> bool,
) -> T {
    let observation = DOCUMENT_PHASE_REQUEST_ID
        .try_with(|request_id| (*request_id, std::time::Instant::now()))
        .ok();
    if let Some((request_id, _)) = observation {
        tracing::info!(
            target: "handshake_core::knowledge_documents_api",
            request_id = %request_id, phase, event = "begin", elapsed_ms = 0_u64,
            "MT032_DOCUMENT_PHASE"
        );
    }
    let result = future.await;
    if let Some((request_id, started)) = observation {
        tracing::info!(
            target: "handshake_core::knowledge_documents_api",
            request_id = %request_id, phase,
            event = if failed(&result) { "error" } else { "end" },
            elapsed_ms = started.elapsed().as_millis() as u64,
            "MT032_DOCUMENT_PHASE"
        );
    }
    result
}
