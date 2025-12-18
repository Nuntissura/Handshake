use axum::{routing::get, Router};

use crate::AppState;

pub mod canvases;
pub mod jobs;
pub mod logs;
pub mod paths;
pub mod workspaces;

pub fn routes(state: AppState) -> Router {
    let workspace_routes = workspaces::routes(state.clone());
    let canvas_routes = canvases::routes(state.clone());
    let job_routes = jobs::routes(state.clone());
    let log_routes = Router::new()
        .route("/logs/tail", get(logs::tail_logs))
        .with_state(state.clone());

    workspace_routes
        .merge(canvas_routes)
        .merge(log_routes)
        .merge(job_routes)
}
