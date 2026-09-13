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

    fn require_actor(actor_id: &str) -> Result<&str, String> {
        let trimmed = actor_id.trim();
        if trimmed.is_empty() {
            return Err("stealth_ref_actor_required: actor_id must not be empty".to_string());
        }
        Ok(trimmed)
    }

    async fn assert_window_owner(
        store: &AtelierStore,
        actor_id: &str,
        window_ref_id: Uuid,
    ) -> Result<(), String> {
        let window = store
            .get_stealth_window(window_ref_id)
            .await
            .map_err(|error| error.to_string())?;
        if window.owner_actor != actor_id {
            return Err(format!(
                "stealth_ref_forbidden: actor {actor_id} cannot access stealth window {window_ref_id}"
            ));
        }
        Ok(())
    }

    async fn list_windows(
        &self,
        actor_id: &str,
        status: Option<StealthRefStatus>,
        limit: Option<i64>,
    ) -> Result<Vec<StealthReferenceWindow>, String> {
        let actor_id = Self::require_actor(actor_id)?;
        let store = self.store()?;
        store
            .list_stealth_windows(actor_id, status, limit.unwrap_or(100))
            .await
            .map_err(|error| error.to_string())
    }

    async fn list_refs(
        &self,
        actor_id: &str,
        window_ref_id: Uuid,
    ) -> Result<Vec<ContentRef>, String> {
        let actor_id = Self::require_actor(actor_id)?;
        let store = self.store()?;
        Self::assert_window_owner(&store, actor_id, window_ref_id).await?;
        store
            .list_stealth_refs(window_ref_id)
            .await
            .map_err(|error| error.to_string())
    }

    async fn resolve_ref(
        &self,
        actor_id: &str,
        window_ref_id: Uuid,
        ref_id: Uuid,
    ) -> Result<ResolvedContentRef, String> {
        let actor_id = Self::require_actor(actor_id)?;
        let store = self.store()?;
        Self::assert_window_owner(&store, actor_id, window_ref_id).await?;
        store
            .resolve_stealth_ref(window_ref_id, ref_id)
            .await
            .map_err(|error| error.to_string())
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_stealth_ref_list_windows(
    actor_id: String,
    status: Option<StealthRefStatus>,
    limit: Option<i64>,
    state: State<'_, StealthRefIpcState>,
) -> Result<Vec<StealthReferenceWindow>, String> {
    state.list_windows(&actor_id, status, limit).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_stealth_ref_list_refs(
    actor_id: String,
    window_ref_id: Uuid,
    state: State<'_, StealthRefIpcState>,
) -> Result<Vec<ContentRef>, String> {
    state.list_refs(&actor_id, window_ref_id).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_stealth_ref_resolve_ref(
    actor_id: String,
    window_ref_id: Uuid,
    ref_id: Uuid,
    state: State<'_, StealthRefIpcState>,
) -> Result<ResolvedContentRef, String> {
    state.resolve_ref(&actor_id, window_ref_id, ref_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::atelier::stealth_window::{
        ContentRefKind, NewContentRef, NewStealthWindow, QuietFlags, VisibilityFlag,
    };

    #[tokio::test]
    async fn state_lists_refs_and_resolves_through_embedded_surreal_with_actor_scope() {
        let directory = tempfile::tempdir().expect("temporary stealth-ref authority root");
        let data_dir = directory.path().to_string_lossy();
        let config = handshake_core::storage::ControlPlaneStorageConfig::resolve(
            Some("surreal_embedded"),
            Some(&data_dir),
        )
        .expect("resolve embedded SurrealDB configuration");
        let control_plane =
            handshake_core::storage::init_control_plane_storage_with_config(&config)
                .await
                .expect("initialize embedded SurrealDB control plane storage");
        let store =
            AtelierStore::with_event_ledger(control_plane.surreal.clone(), control_plane.database);
        store.ensure_schema().await.expect("ensure atelier schema");
        let state = StealthRefIpcState::with_store(store.clone());

        let actor_id = format!("operator-{}", Uuid::new_v4());
        let foreign_actor = format!("operator-{}", Uuid::new_v4());
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
            .list_windows(&actor_id, Some(StealthRefStatus::Open), Some(25))
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
            .list_refs(&actor_id, window.window_ref_id)
            .await
            .expect("list refs for owned stealth window");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_id, content_ref.ref_id);

        let resolved = state
            .resolve_ref(&actor_id, window.window_ref_id, content_ref.ref_id)
            .await
            .expect("resolve owned stealth ref");
        assert_eq!(resolved.ref_id, content_ref.ref_id);
        assert!(!resolved.payload_included);

        let denied = state
            .list_refs(&actor_id, foreign_window.window_ref_id)
            .await;
        assert!(
            denied
                .expect_err("foreign actor window should be inaccessible")
                .contains("stealth_ref_forbidden"),
            "foreign-window access returns a typed forbidden error"
        );
        control_plane
            .surreal
            .shutdown()
            .await
            .expect("close embedded stealth-ref authority");
    }
}
