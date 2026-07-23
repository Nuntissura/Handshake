//! Shared PostgreSQL support for the WP-KERNEL-009 knowledge integration
//! tests (MT-049..MT-064).
//!
//! Proof-path contract: tests run against REAL PostgreSQL only.
//! URL resolution order:
//!   1. `POSTGRES_TEST_URL` (explicit operator-provided cluster),
//!   2. `DATABASE_URL`,
//!   3. the Handshake-managed PostgreSQL runtime
//!      (`managed_postgres::ManagedPostgres::ensure_running`, default port
//!      5544, data dir `<repo>/Handshake_Artifacts/handshake-product/managed_pgdata`) — the
//!      product's own no-Docker, no-external-daemon cluster path.
//!
//! Every test gets a fresh isolated schema (`knowledge_test_<uuidv7>`) on that
//! cluster with the full migration chain applied, mirroring
//! `storage/tests.rs::postgres_backend_with_pool_from_env`. Schema setup and
//! migrations are serialized behind a process-wide async mutex because
//! concurrent `CREATE EXTENSION` / migration runs on one cluster race (the
//! same flake shows up in the pre-existing storage tests when run with high
//! parallelism).
//!
//! There is NO SQLite, in-memory, or mock fallback: when the PostgreSQL
//! binaries are genuinely absent the helper returns `None` and the caller
//! must `eprintln!` a SKIP marker and return (mirrors `atelier_pg_support`).
//! Every other failure panics so a broken cluster can never look green.

use handshake_core::managed_postgres::{
    ManagedPostgres, ManagedPostgresConfig, ManagedPostgresError,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::Database;
use sqlx::Connection;
use std::time::Duration;
use tokio::sync::{Mutex, OnceCell};
use uuid::Uuid;

static MANAGED_POSTGRES: OnceCell<Option<ManagedPostgres>> = OnceCell::const_new();
static SCHEMA_SETUP_LOCK: Mutex<()> = Mutex::const_new(());
const CLEANUP_PHASE_TIMEOUT: Duration = Duration::from_secs(3);
pub const PANIC_CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);

fn drop_schema_sql(schema: &str) -> String {
    format!(
        "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
        schema.replace('"', "\"\"")
    )
}

async fn drop_and_verify_schema(
    base_url: &str,
    schema: &str,
    application_name: &str,
) -> Result<(), String> {
    let mut conn =
        tokio::time::timeout(CLEANUP_PHASE_TIMEOUT, sqlx::PgConnection::connect(base_url))
            .await
            .map_err(|_| "connect for isolated schema teardown timed out".to_string())?
            .map_err(|error| format!("connect for isolated schema teardown: {error}"))?;
    tokio::time::timeout(
        CLEANUP_PHASE_TIMEOUT,
        sqlx::query(
            "SELECT pg_terminate_backend(pid) \
             FROM pg_stat_activity \
             WHERE datname = current_database() \
               AND application_name = $1 \
               AND pid <> pg_backend_pid()",
        )
        .bind(application_name)
        .execute(&mut conn),
    )
    .await
    .map_err(|_| "terminate isolated schema connections timed out".to_string())?
    .map_err(|error| format!("terminate isolated schema connections: {error}"))?;
    tokio::time::timeout(
        CLEANUP_PHASE_TIMEOUT,
        sqlx::query(&drop_schema_sql(schema)).execute(&mut conn),
    )
    .await
    .map_err(|_| "drop exact isolated knowledge schema timed out".to_string())?
    .map_err(|error| format!("drop exact isolated knowledge schema: {error}"))?;
    let remains: bool = tokio::time::timeout(
        CLEANUP_PHASE_TIMEOUT,
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(schema)
            .fetch_one(&mut conn),
    )
    .await
    .map_err(|_| "verify isolated knowledge schema teardown timed out".to_string())?
    .map_err(|error| format!("verify isolated knowledge schema teardown: {error}"))?;
    if remains {
        return Err("isolated knowledge schema still exists after teardown".to_string());
    }
    tokio::time::timeout(CLEANUP_PHASE_TIMEOUT, conn.close())
        .await
        .map_err(|_| "close isolated schema teardown connection timed out".to_string())?
        .map_err(|error| format!("close isolated schema teardown connection: {error}"))?;
    Ok(())
}

/// Resolve the base database URL (no schema isolation yet).
pub async fn base_database_url() -> Option<String> {
    for var in ["POSTGRES_TEST_URL", "DATABASE_URL"] {
        if let Some(url) = std::env::var(var)
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            return Some(url);
        }
    }

    let managed = MANAGED_POSTGRES
        .get_or_init(|| async {
            match ManagedPostgres::ensure_running(ManagedPostgresConfig::from_env()).await {
                Ok(managed) => Some(managed),
                Err(ManagedPostgresError::BinariesNotFound(detail)) => {
                    eprintln!(
                        "SKIP knowledge PostgreSQL proof: PostgreSQL binaries not found ({detail})"
                    );
                    None
                }
                Err(err) => panic!("Handshake-managed PostgreSQL startup failed: {err}"),
            }
        })
        .await;

    managed.as_ref().map(ManagedPostgres::database_url)
}

/// A per-test isolated knowledge database on the real cluster.
pub struct KnowledgePg {
    /// Concrete Postgres backend (KnowledgeStore + Database are implemented
    /// on it) connected with `search_path` pinned to the isolated schema.
    pub db: PostgresDatabase,
    /// The isolated schema name.
    pub schema: String,
    /// Connection URL pinned to the isolated schema.
    pub schema_url: String,
    base_url: String,
    application_name: String,
    torn_down: bool,
}

impl KnowledgePg {
    /// Open an extra raw connection into the same isolated schema for direct
    /// SQL assertions (constraint probing, catalog checks).
    pub async fn raw_connection(&self) -> sqlx::PgConnection {
        sqlx::PgConnection::connect(&self.schema_url)
            .await
            .expect("open raw connection into isolated knowledge schema")
    }

    /// Create a real workspace row (FK target for knowledge tables).
    pub async fn create_workspace(&self) -> String {
        let ctx = handshake_core::storage::WriteContext::human(None);
        let workspace = self
            .db
            .create_workspace(
                &ctx,
                handshake_core::storage::NewWorkspace {
                    name: format!("knowledge-ws-{}", Uuid::now_v7()),
                },
            )
            .await
            .expect("create workspace for knowledge tests");
        workspace.id
    }

    /// Close every owned pool, drop the exact isolated schema, and prove it no
    /// longer exists. Call only after loopback servers/raw connections using
    /// this fixture have been shut down.
    async fn teardown_inner(&mut self) {
        if self.torn_down {
            return;
        }
        let schema = self.schema.clone();
        let base_url = self.base_url.clone();
        let application_name = self.application_name.clone();
        // Polling close starts graceful pool shutdown, but a leaked or
        // outstanding PoolConnection must not make teardown wait forever.
        let _ = tokio::time::timeout(CLEANUP_PHASE_TIMEOUT, self.db.close()).await;
        let cleanup = drop_and_verify_schema(&base_url, &schema, &application_name).await;
        self.torn_down = true;
        cleanup.unwrap_or_else(|error| panic!("{error}"));
    }

    pub async fn teardown(mut self) {
        self.teardown_inner().await;
    }
}

impl Drop for KnowledgePg {
    fn drop(&mut self) {
        if self.torn_down {
            return;
        }
        let schema = self.schema.clone();
        let base_url = self.base_url.clone();
        let application_name = self.application_name.clone();
        let cleanup = std::thread::scope(|scope| {
            let handle = scope.spawn(|| -> Result<(), String> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| format!("build isolated schema teardown runtime: {error}"))?;
                runtime.block_on(async {
                    let _ = tokio::time::timeout(CLEANUP_PHASE_TIMEOUT, self.db.close()).await;
                    drop_and_verify_schema(&base_url, &schema, &application_name).await
                })
            });
            handle
                .join()
                .map_err(|_| "isolated schema teardown thread panicked".to_string())?
        });
        self.torn_down = true;
        if let Err(error) = cleanup {
            if std::thread::panicking() {
                eprintln!("KnowledgePg panic-path teardown failed: {error}");
            } else {
                panic!("KnowledgePg drop teardown failed: {error}");
            }
        }
    }
}

/// Build a fresh isolated schema + run all migrations on the real cluster.
///
/// Returns `None` only when PostgreSQL binaries are absent (caller must SKIP
/// loudly). Panics on every other failure.
pub async fn knowledge_pg() -> Option<KnowledgePg> {
    let url = base_database_url().await?;

    let _setup_guard = SCHEMA_SETUP_LOCK.lock().await;

    let schema = format!("knowledge_test_{}", Uuid::now_v7().simple());
    let mut conn = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect to PostgreSQL for schema setup");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await
        .expect("create isolated knowledge test schema");
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public")
        .execute(&mut conn)
        .await
        .expect("ensure pgcrypto extension");
    // Same digest shims storage/tests.rs installs: migrations reference
    // digest() unqualified and resolve it through the per-schema search_path.
    for shim in [
        format!(
            r#"
            CREATE OR REPLACE FUNCTION {schema}.digest(input text, algorithm text)
            RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE
            AS $$ SELECT public.digest(input::bytea, algorithm) $$
            "#
        ),
        format!(
            r#"
            CREATE OR REPLACE FUNCTION {schema}.digest(input bytea, algorithm text)
            RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE
            AS $$ SELECT public.digest(input, algorithm) $$
            "#
        ),
    ] {
        sqlx::query(&shim)
            .execute(&mut conn)
            .await
            .expect("install digest shim in isolated schema");
    }
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let application_name = schema.clone();
    let schema_url =
        format!("{url}{sep}options=-csearch_path%3D{schema}&application_name={application_name}");

    let db = PostgresDatabase::connect(&schema_url, 5)
        .await
        .expect("connect PostgresDatabase to isolated knowledge schema");
    db.run_migrations()
        .await
        .expect("run full migration chain in isolated knowledge schema");

    Some(KnowledgePg {
        db,
        schema,
        schema_url,
        base_url: url,
        application_name,
        torn_down: false,
    })
}
