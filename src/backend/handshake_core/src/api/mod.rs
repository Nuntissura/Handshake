use axum::{routing::get, Router};

use crate::AppState;

pub mod bundles;
pub mod canvases;
pub mod diagnostics;
pub mod flight_recorder;
pub mod governance_pack;
pub mod jobs;
pub mod logs;
pub mod loom;
pub mod paths;
pub mod role_mailbox;
pub mod workspaces;

pub fn routes(state: AppState) -> Router {
    let workspace_routes = workspaces::routes(state.clone());
    let canvas_routes = canvases::routes(state.clone());
    let job_routes = jobs::routes(state.clone());
    let loom_routes = loom::routes(state.clone());
    let flight_recorder_routes = flight_recorder::routes(state.clone());
    let diagnostics_routes = diagnostics::routes(state.clone());
    let bundle_routes = bundles::routes(state.clone());
    let governance_pack_routes = governance_pack::routes(state.clone());
    let role_mailbox_routes = role_mailbox::routes(state.clone());
    let log_routes = Router::new()
        .route("/logs/tail", get(logs::tail_logs))
        .with_state(state.clone());

    workspace_routes
        .merge(canvas_routes)
        .merge(log_routes)
        .merge(job_routes)
        .merge(loom_routes)
        .merge(diagnostics_routes)
        .merge(flight_recorder_routes)
        .merge(bundle_routes)
        .merge(governance_pack_routes)
        .merge(role_mailbox_routes)
}
