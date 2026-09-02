use handshake_core::storage::tests::{embedded_test_backend, run_calendar_storage_conformance};

#[tokio::test]
async fn calendar_storage_conformance() {
    let backend = embedded_test_backend()
        .await
        .expect("open isolated embedded calendar backend");
    let db = std::sync::Arc::clone(&backend.database);

    run_calendar_storage_conformance(db)
        .await
        .expect("embedded calendar storage conformance");

    backend
        .close_and_remove()
        .await
        .expect("close embedded calendar backend");
}
