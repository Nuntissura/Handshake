use handshake_core::storage::tests::{
    embedded_test_backend, run_loom_storage_conformance, run_loom_traversal_performance_probe,
    run_storage_conformance,
};

#[tokio::test]
async fn surreal_storage_conformance() {
    let backend = embedded_test_backend()
        .await
        .expect("failed to init embedded SurrealDB backend");
    let result = {
        let db = backend.database.clone();
        run_storage_conformance(db).await
    };
    result.expect("embedded SurrealDB storage conformance");
    backend
        .close_and_remove()
        .await
        .expect("embedded SurrealDB storage cleanup");
}

#[tokio::test]
async fn surreal_loom_storage_conformance() {
    let backend = embedded_test_backend()
        .await
        .expect("failed to init embedded SurrealDB backend");
    let result = {
        let db = backend.database.clone();
        run_loom_storage_conformance(db).await
    };
    result.expect("embedded SurrealDB loom storage conformance");
    backend
        .close_and_remove()
        .await
        .expect("embedded SurrealDB storage cleanup");
}

#[tokio::test]
async fn surreal_loom_traversal_performance_target() {
    let backend = embedded_test_backend()
        .await
        .expect("failed to init embedded SurrealDB backend");
    let result = {
        let db = backend.database.clone();
        run_loom_traversal_performance_probe(db).await
    };
    result.expect("embedded SurrealDB loom traversal performance");
    backend
        .close_and_remove()
        .await
        .expect("embedded SurrealDB storage cleanup");
}
