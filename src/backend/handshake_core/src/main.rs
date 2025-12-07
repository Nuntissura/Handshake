use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::{sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, SqlitePool};
use std::{net::SocketAddr, path::Path, process, str::FromStr};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    component: &'static str,
    version: &'static str,
    db_status: String,
}

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
}

#[tokio::main]
async fn main() {
    let addr: SocketAddr = ([127, 0, 0, 1], 37501).into();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let pool = init_db().await.expect("failed to init database");
    let state = AppState { pool };

    let app = Router::new()
        .route("/health", get(health))
        .with_state(state)
        .layer(cors);

    println!(
        "handshake_core listening on {} (pid {})",
        addr,
        process::id()
    );

    let listener = TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app).await.expect("server error");
}

async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    let data_dir = Path::new("data");
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).map_err(|err| {
            eprintln!("failed to create data directory {:?}: {}", data_dir, err);
            sqlx::Error::Io(err)
        })?;
    }

    let db_url = "sqlite://data/handshake.db";

    let connect_options = SqliteConnectOptions::from_str(db_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("database ready at {}", db_url);

    Ok(pool)
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_check = sqlx::query("SELECT 1").execute(&state.pool).await;
    let db_status = match db_check {
        Ok(_) => "ok",
        Err(err) => {
            eprintln!("/health db check error: {}", err);
            "error"
        }
    };

    let overall_status = if db_status == "ok" { "ok" } else { "error" };

    println!("/health 200");

    Json(HealthResponse {
        status: overall_status.to_string(),
        component: "handshake_core",
        version: env!("CARGO_PKG_VERSION"),
        db_status: db_status.to_string(),
    })
}
