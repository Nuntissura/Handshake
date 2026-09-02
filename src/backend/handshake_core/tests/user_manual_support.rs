//! UserManual-local embedded SurrealDB integration scope and loopback server.

use axum::Router;
use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::user_manual::store::UserManualStore;

use crate::surreal_test_store_support::EmbeddedSurrealTestScope;

pub struct UserManualTestScope {
    embedded: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
}

impl UserManualTestScope {
    pub async fn create() -> Self {
        let mut embedded = EmbeddedSurrealTestScope::create()
            .await
            .expect("create isolated embedded UserManual scope");
        let storage = embedded
            .activate_storage()
            .await
            .expect("activate embedded UserManual storage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap embedded UserManual schema");
        Self { embedded, storage }
    }

    pub fn storage(&self) -> SurrealStorage {
        self.storage.clone()
    }

    pub fn store(&self) -> UserManualStore {
        UserManualStore::new(self.storage())
    }

    pub async fn cleanup(mut self) {
        self.storage
            .shutdown()
            .await
            .expect("close embedded UserManual storage");
        drop(self.storage);
        self.embedded
            .shutdown_storage_for_reopen()
            .await
            .expect("release embedded UserManual storage handle");
        self.embedded
            .cleanup()
            .await
            .expect("clean embedded UserManual scope");
    }
}

/// Serve a router on a quiet loopback listener.
pub async fn start_server(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("test api server");
    });
    (format!("http://{addr}"), server)
}
