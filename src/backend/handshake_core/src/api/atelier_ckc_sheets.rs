//! WP-CKC-posekit-overhaul SurrealDB port — CKC `sheets` lane router.
//!
//! Skeleton: the lane owner replaces this file with the ported handlers. Shared helpers come from
//! `super::atelier` (`atelier_store`, `atelier_error`, `internal_error`, `calling_actor`,
//! `artifact_byte_read_error`, `ErrorResponse`, `LIST_CAP`). Storage authority is the embedded
//! SurrealDB store through `AtelierStore`; no relational fallback exists.

use axum::Router;

use crate::AppState;

pub fn routes(state: AppState) -> Router {
    Router::new().with_state(state)
}
