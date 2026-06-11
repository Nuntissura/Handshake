//! Stealth Reference Window IPC.
//!
//! Read commands are backed by the real PostgreSQL AtelierStore plus the kernel
//! EventLedger database handle. There is no in-memory success path: if the app
//! cannot initialize Postgres, commands return a typed error instead of
//! pretending that stealth-ref state is durable.

use handshake_core::{
    atelier::{
        stealth_window::{
            ContentRef, ResolvedContentRef, StealthRefStatus, StealthReferenceWindow,
        },
        AtelierStore,
    },
    storage::init_control_plane_storage,
};
use tauri::State;
use uuid::Uuid;

enum StealthRefBackend {
    Unavailable { reason: String },
    Postgres { store: AtelierStore },
}

pub struct StealthRefIpcState {
    backend: StealthRefBackend,
}

impl Default for StealthRefIpcState {
    fn default() -> Self {
        Self {
            backend: StealthRefBackend::Unavailable {
                reason: "Postgres stealth-ref state has not been initialized".to_string(),
            },
        }
    }
}

impl StealthRefIpcState {
    pub fn with_store(store: AtelierStore) -> Self {
        Self {
            backend: StealthRefBackend::Postgres { store },
        }
    }

    pub fn from_env_or_unavailable() -> Self {
        let state = tauri::async_runtime::block_on(async {
            let control_plane = init_control_plane_storage()
                .await
                .map_err(|error| error.to_string())?;
            let store = AtelierStore::with_event_ledger(
                control_plane.postgres_pool,
                control_plane.database,
            );
            store
                .ensure_schema()
                .await
                .map_err(|error| error.to_string())?;
            Ok::<Self, String>(Self::with_store(store))
        });

        match state {
            Ok(state) => state,
            Err(error) => Self {
                backend: StealthRefBackend::Unavailable {
                    reason: format!("Postgres stealth-ref state unavailable: {error}"),
                },
            },
        }
    }

    fn store(&self) -> Result<AtelierStore, String> {
        match &self.backend {
            StealthRefBackend::Unavailable { reason } => {
                Err(format!("stealth_ref_postgres_unavailable: {reason}"))
            }
            StealthRefBackend::Postgres { store } => Ok(store.clone()),
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

    fn database_url() -> Option<String> {
        std::env::var("DATABASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
    }

    #[tokio::test]
    async fn state_lists_refs_and_resolves_through_live_postgres_with_actor_scope() {
        let Some(_) = database_url() else {
            eprintln!("SKIP stealth_ref IPC state test: DATABASE_URL not set");
            return;
        };

        let control_plane = init_control_plane_storage()
            .await
            .expect("initialize live Postgres control plane storage");
        let store =
            AtelierStore::with_event_ledger(control_plane.postgres_pool, control_plane.database);
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
    }
}
