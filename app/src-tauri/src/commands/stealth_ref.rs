//! Stealth Reference Window IPC.
//!
//! Read commands are backed by the shared embedded SurrealDB AtelierStore and
//! EventLedger handle. There is no in-memory success path: if the authority
//! cannot initialize, commands return a typed error instead of
//! pretending that stealth-ref state is durable.

use handshake_core::atelier::{
    stealth_window::{ContentRef, ResolvedContentRef, StealthRefStatus, StealthReferenceWindow},
    AtelierStore,
};
use tauri::State;
use uuid::Uuid;

const STEALTH_REF_DENIED: &str = "stealth_ref_denied";

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StealthRefListWindowsRequest {
    pub session_token: Option<String>,
    pub status: Option<StealthRefStatus>,
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StealthRefListRefsRequest {
    pub session_token: Option<String>,
    pub window_ref_id: Uuid,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StealthRefResolveRefRequest {
    pub session_token: Option<String>,
    pub window_ref_id: Uuid,
    pub ref_id: Uuid,
}

enum StealthRefBackend {
    Unavailable { reason: String },
    Surreal { store: AtelierStore },
}

pub struct StealthRefIpcState {
    backend: StealthRefBackend,
}

impl Default for StealthRefIpcState {
    fn default() -> Self {
        Self {
            backend: StealthRefBackend::Unavailable {
                reason: "embedded SurrealDB stealth-ref authority has not initialized".to_string(),
            },
        }
    }
}

impl StealthRefIpcState {
    pub fn with_store(store: AtelierStore) -> Self {
        Self {
            backend: StealthRefBackend::Surreal { store },
        }
    }

    fn store(&self) -> Result<AtelierStore, String> {
        match &self.backend {
            StealthRefBackend::Unavailable { reason } => {
                Err(format!("stealth_ref_surreal_unavailable: {reason}"))
            }
            StealthRefBackend::Surreal { store } => Ok(store.clone()),
        }
    }

    fn authenticated_actor(session_token: Option<&str>) -> Result<String, String> {
        handshake_core::api::stage::authenticate_native_session_token(session_token)
            .map(|session| session.actor_id)
            .map_err(|_| STEALTH_REF_DENIED.to_owned())
    }

    async fn assert_window_owner(
        store: &AtelierStore,
        actor_id: &str,
        window_ref_id: Uuid,
    ) -> Result<(), String> {
        let window = store
            .get_stealth_window(window_ref_id)
            .await
            .map_err(|_| STEALTH_REF_DENIED.to_owned())?;
        if window.owner_actor != actor_id {
            return Err(STEALTH_REF_DENIED.to_owned());
        }
        Ok(())
    }

    async fn list_windows(
        &self,
        session_token: Option<&str>,
        status: Option<StealthRefStatus>,
        limit: Option<i64>,
    ) -> Result<Vec<StealthReferenceWindow>, String> {
        let actor_id = Self::authenticated_actor(session_token)?;
        let store = self.store()?;
        store
            .list_stealth_windows(&actor_id, status, limit.unwrap_or(100))
            .await
            .map_err(|error| error.to_string())
    }

    async fn list_refs(
        &self,
        session_token: Option<&str>,
        window_ref_id: Uuid,
    ) -> Result<Vec<ContentRef>, String> {
        let actor_id = Self::authenticated_actor(session_token)?;
        let store = self.store()?;
        Self::assert_window_owner(&store, &actor_id, window_ref_id).await?;
        store
            .list_stealth_refs(window_ref_id)
            .await
            .map_err(|error| error.to_string())
    }

    async fn resolve_ref(
        &self,
        session_token: Option<&str>,
        window_ref_id: Uuid,
        ref_id: Uuid,
    ) -> Result<ResolvedContentRef, String> {
        let actor_id = Self::authenticated_actor(session_token)?;
        let store = self.store()?;
        Self::assert_window_owner(&store, &actor_id, window_ref_id).await?;
        store
            .resolve_stealth_ref(window_ref_id, ref_id)
            .await
            .map_err(|error| error.to_string())
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_stealth_ref_list_windows(
    request: StealthRefListWindowsRequest,
    state: State<'_, StealthRefIpcState>,
) -> Result<Vec<StealthReferenceWindow>, String> {
    state
        .list_windows(
            request.session_token.as_deref(),
            request.status,
            request.limit,
        )
        .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_stealth_ref_list_refs(
    request: StealthRefListRefsRequest,
    state: State<'_, StealthRefIpcState>,
) -> Result<Vec<ContentRef>, String> {
    state
        .list_refs(request.session_token.as_deref(), request.window_ref_id)
        .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_stealth_ref_resolve_ref(
    request: StealthRefResolveRefRequest,
    state: State<'_, StealthRefIpcState>,
) -> Result<ResolvedContentRef, String> {
    state
        .resolve_ref(
            request.session_token.as_deref(),
            request.window_ref_id,
            request.ref_id,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::atelier::stealth_window::{
        ContentRefKind, NewContentRef, NewStealthWindow, QuietFlags, VisibilityFlag,
    };
    use handshake_core::storage::surreal::{
        bootstrap_atelier_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
    };
    use handshake_core::storage::Database;
    use std::sync::{Arc, Mutex};

    static BINDING_ENV_LOCK: Mutex<()> = Mutex::new(());

    struct BindingEnvGuard {
        previous: Option<std::ffi::OsString>,
        path: std::path::PathBuf,
    }

    impl Drop for BindingEnvGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.previous.take() {
                std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous);
            } else {
                std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn install_binding(token: &str) -> BindingEnvGuard {
        let path =
            std::env::temp_dir().join(format!("hsk-stealth-ref-binding-{}.json", Uuid::now_v7()));
        std::fs::write(
            &path,
            serde_json::to_vec(
                &handshake_core::api::stage::current_process_native_session_binding(token),
            )
            .expect("serialize native session binding"),
        )
        .expect("write native session binding");
        let guard = BindingEnvGuard {
            previous: std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE"),
            path: path.clone(),
        };
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
        guard
    }

    #[tokio::test]
    async fn state_lists_refs_and_resolves_through_embedded_surreal_with_actor_scope() {
        let _env_lock = BINDING_ENV_LOCK.lock().expect("binding env lock");
        let token_a = "a".repeat(64);
        let token_b = "b".repeat(64);
        let _binding_guard = install_binding(&token_a);
        let directory = tempfile::tempdir().expect("temporary stealth-ref authority root");
        let storage = SurrealStorage::open(
            SurrealStorageConfig::for_data_dir(directory.path())
                .expect("configure isolated stealth-ref SurrealDB"),
        )
        .await
        .expect("open isolated stealth-ref SurrealDB");
        bootstrap_atelier_schema(&storage)
            .await
            .expect("bootstrap isolated stealth-ref SurrealDB schema");
        let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(storage.clone()));
        let store = AtelierStore::with_event_ledger(storage.clone(), database);
        store.ensure_schema().await.expect("ensure atelier schema");
        let state = StealthRefIpcState::with_store(store.clone());

        let actor_id =
            handshake_core::api::stage::authenticate_native_session_token(Some(&token_a))
                .expect("authenticate owner session")
                .actor_id;
        let foreign_actor = format!("foreign-account-{}", Uuid::new_v4());
        let window = store
            .create_stealth_window(&NewStealthWindow {
                owner_actor: actor_id.clone(),
                title: format!("stealth-ipc-window-{}", Uuid::new_v4()),
                visibility: VisibilityFlag::OffScreenOnly,
                quiet: QuietFlags::default(),
                layout: None,
            })
            .await
            .expect("create owned stealth window");
        let foreign_window = store
            .create_stealth_window(&NewStealthWindow {
                owner_actor: foreign_actor.clone(),
                title: format!("stealth-ipc-window-{}", Uuid::new_v4()),
                visibility: VisibilityFlag::OffScreenOnly,
                quiet: QuietFlags::default(),
                layout: None,
            })
            .await
            .expect("create foreign stealth window");
        let content_ref = store
            .add_stealth_ref(
                window.window_ref_id,
                &NewContentRef {
                    ref_kind: ContentRefKind::Artifact,
                    resolver: format!("artifact-manifest-{}", Uuid::new_v4()),
                    content_sha256: format!("sha256-{}", Uuid::new_v4()),
                    redaction_state: true,
                },
            )
            .await
            .expect("add stealth content ref");

        let windows = state
            .list_windows(Some(&token_a), Some(StealthRefStatus::Open), Some(25))
            .await
            .expect("list actor stealth windows through IPC state");
        assert!(
            windows
                .iter()
                .any(|candidate| candidate.window_ref_id == window.window_ref_id),
            "actor-scoped list includes the owned stealth window"
        );
        assert!(
            !windows
                .iter()
                .any(|candidate| candidate.window_ref_id == foreign_window.window_ref_id),
            "actor-scoped list excludes foreign stealth windows"
        );

        let refs = state
            .list_refs(Some(&token_a), window.window_ref_id)
            .await
            .expect("list refs for owned stealth window");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_id, content_ref.ref_id);

        let resolved = state
            .resolve_ref(Some(&token_a), window.window_ref_id, content_ref.ref_id)
            .await
            .expect("resolve owned stealth ref");
        assert_eq!(resolved.ref_id, content_ref.ref_id);
        assert!(!resolved.payload_included);

        let missing = state.list_refs(None, window.window_ref_id).await;
        let forged = state
            .list_refs(Some(&"f".repeat(64)), window.window_ref_id)
            .await;
        let cross_account = state
            .list_refs(Some(&token_a), foreign_window.window_ref_id)
            .await;
        for denied in [missing, forged, cross_account] {
            assert_eq!(
                denied.expect_err("request must be denied"),
                STEALTH_REF_DENIED,
                "authentication and ownership denials have one non-enumerating shape"
            );
        }

        std::fs::write(
            &_binding_guard.path,
            serde_json::to_vec(
                &handshake_core::api::stage::current_process_native_session_binding(&token_b),
            )
            .expect("serialize rotated binding"),
        )
        .expect("rotate native session binding");
        assert_eq!(
            state
                .list_refs(Some(&token_a), window.window_ref_id)
                .await
                .expect_err("rotated session must be revoked"),
            STEALTH_REF_DENIED
        );
        let replacement_windows = state
            .list_windows(Some(&token_b), Some(StealthRefStatus::Open), Some(25))
            .await
            .expect("replacement session authenticates");
        assert!(
            replacement_windows.is_empty(),
            "replacement principal cannot enumerate the prior principal's windows"
        );

        assert!(
            serde_json::from_value::<StealthRefListRefsRequest>(serde_json::json!({
                "session_token": token_b,
                "actor_id": actor_id,
                "window_ref_id": window.window_ref_id,
            }))
            .is_err(),
            "forged actor metadata is rejected by the public request contract"
        );
        storage
            .shutdown()
            .await
            .expect("close embedded stealth-ref authority");
    }
}
