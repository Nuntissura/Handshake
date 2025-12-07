use axum::Router;

use crate::AppState;

pub mod canvases;
pub mod workspaces;

pub fn routes(state: AppState) -> Router {
    let workspace_routes = workspaces::routes(state.clone());
    let canvas_routes = canvases::routes(state.clone());

    workspace_routes.merge(canvas_routes)
}
