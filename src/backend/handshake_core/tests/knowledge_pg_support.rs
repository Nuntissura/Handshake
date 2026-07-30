//! Shared PostgreSQL support for the WP-KERNEL-009 knowledge integration
//! tests (MT-049..MT-064).
//!
//! Proof-path contract: tests run against REAL PostgreSQL only.
//! URL resolution order:
//!   1. `POSTGRES_TEST_URL` (explicit operator-provided cluster),
//!   2. `DATABASE_URL`,
//!   3. the Handshake-managed PostgreSQL runtime
//!      (`managed_postgres::ManagedPostgres::ensure_running`, default port
//!      5544, data dir `<repo>/Handshake_Artifacts/managed_pgdata`) — the
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

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Mutex as StdMutex, OnceLock},
};

use handshake_core::managed_postgres::{
    ManagedPostgres, ManagedPostgresConfig, ManagedPostgresError,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::Database;
use sqlx::{postgres::PgPoolOptions, Connection};
use tokio::sync::{Mutex, OnceCell};
use uuid::Uuid;

static MANAGED_POSTGRES: OnceCell<Option<ManagedPostgres>> = OnceCell::const_new();
static SCHEMA_SETUP_LOCK: Mutex<()> = Mutex::const_new(());
static MIGRATED_TEMPLATE_DATABASE: OnceCell<String> = OnceCell::const_new();
static TEMPLATE_HARNESS_CLEANUP: OnceLock<StdMutex<Option<TemplateHarnessCleanup>>> =
    OnceLock::new();
static TEMPLATE_HARNESS_CLEANUP_REGISTERED: OnceLock<()> = OnceLock::new();
/// Isolated `knowledge_test_*` schemas this process created, paired with the
/// base URL they live on, so they can be dropped at process exit.
static OWNED_TEST_SCHEMAS: OnceLock<StdMutex<Vec<(String, String)>>> = OnceLock::new();
static SCHEMA_CLEANUP_REGISTERED: OnceLock<()> = OnceLock::new();

const DATABASE_TEMPLATE_MODE_ENV: &str = "HANDSHAKE_TEST_PG_DATABASE_TEMPLATE";
const TEMPLATE_AUTHORITY_SCHEMA: &str = "handshake_test_template_authority";

struct TemplateHarnessCleanup {
    psql: PathBuf,
    port: u16,
    superuser: String,
    maintenance_database: String,
    databases: Vec<String>,
    managed: &'static ManagedPostgres,
}

extern "C" {
    fn atexit(callback: extern "C" fn()) -> i32;
}

extern "C" fn clean_template_harness_at_process_exit() {
    let Some(cleanup_slot) = TEMPLATE_HARNESS_CLEANUP.get() else {
        eprintln!("PostgreSQL template-harness cleanup missing global slot");
        std::process::abort();
    };
    let mut cleanup_slot = match cleanup_slot.lock() {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("PostgreSQL template-harness cleanup lock poisoned at process exit");
            error.into_inner()
        }
    };
    let Some(mut cleanup) = cleanup_slot.take() else {
        eprintln!("PostgreSQL template-harness cleanup is not armed");
        std::process::abort();
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cleanup.run())) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            eprintln!("PostgreSQL template-harness cleanup failed: {error}");
            std::process::abort();
        }
        Err(_) => {
            eprintln!("PostgreSQL template-harness cleanup panicked");
            std::process::abort();
        }
    }
}

/// Record an isolated schema so it is dropped when this test process exits.
///
/// Without this, every `knowledge_pg()` call leaks a `knowledge_test_<uuid>`
/// schema (each carrying the full migrated table set) into the shared database
/// forever. The leak is cumulative across every test process on the machine:
/// once several hundred accumulate, `pg_catalog` grows to six figures of
/// relations, autovacuum ANALYZE workers thrash it continuously, and the
/// migration chain that a single test runs degrades from seconds to hours --
/// which silently converts ordinary proof runs into environment failures.
///
/// Cleanup is deliberately best-effort: a test run must never fail, abort, or
/// hang because teardown could not reach the database. Anything that could not
/// be dropped is reported by name so the residue stays visible instead of
/// silently accumulating again.
fn register_test_schema_for_cleanup(base_url: &str, schema: &str) {
    let registry = OWNED_TEST_SCHEMAS.get_or_init(|| StdMutex::new(Vec::new()));
    match registry.lock() {
        Ok(mut owned) => owned.push((base_url.to_string(), schema.to_string())),
        Err(poisoned) => poisoned.into_inner().push((base_url.to_string(), schema.to_string())),
    }
    SCHEMA_CLEANUP_REGISTERED.get_or_init(|| {
        // SAFETY: the callback has C ABI, never unwinds (it catches), captures
        // no stack state, and reads only process-lifetime statics.
        let registered = unsafe { atexit(drop_owned_test_schemas_at_process_exit) };
        if registered != 0 {
            eprintln!(
                "WARNING: could not register isolated-schema cleanup; \
                 knowledge_test_* schemas from this process will leak"
            );
        }
    });
}

extern "C" fn drop_owned_test_schemas_at_process_exit() {
    let Some(registry) = OWNED_TEST_SCHEMAS.get() else {
        return;
    };
    let owned = match registry.lock() {
        Ok(mut guard) => std::mem::take(&mut *guard),
        Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
    };
    if owned.is_empty() {
        return;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Group by base URL so all schemas on one database drop in a single
        // psql invocation.
        let mut by_url: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for (url, schema) in owned {
            by_url.entry(url).or_default().push(schema);
        }
        let psql = postgres_tool_path(Path::new(""), "psql");
        for (url, schemas) in by_url {
            let mut command = std::process::Command::new(&psql);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                command.creation_flags(CREATE_NO_WINDOW);
            }
            command
                .arg("-X")
                .arg("-w")
                .arg("-q")
                // Fail a blocked DROP fast instead of hanging teardown: this
                // process may still be closing its own pooled connections.
                .env("PGOPTIONS", "-c lock_timeout=5000")
                .stdin(Stdio::null());
            for schema in &schemas {
                command
                    .arg("-c")
                    .arg(format!("DROP SCHEMA IF EXISTS {schema} CASCADE"));
            }
            // The connection target MUST be passed as `-d <url>`, never as a
            // bare positional argument. psql's grammar is
            // `psql [OPTION]... [DBNAME [USERNAME]]`, so a leading positional
            // URL makes every following flag a surplus positional: psql prints
            // `extra command-line argument "-c" ignored`, connects, executes
            // NOTHING, and still exits 0. The first version of this cleanup did
            // exactly that and was a silent no-op.
            command.arg("-d").arg(&url);
            match bounded_command_output(command, std::time::Duration::from_secs(60)) {
                // psql exits 0 even when an individual statement fails, so the
                // exit status alone cannot be trusted: scan the output for a
                // reported error before declaring the drop successful.
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let reported_error = stdout.contains("ERROR:")
                        || stderr.contains("ERROR:")
                        || stderr.contains("extra command-line argument");
                    if !output.status.success() || reported_error {
                        eprintln!(
                            "WARNING: isolated-schema cleanup left schema(s) behind ({}): {}{}",
                            output.status, stdout, stderr
                        );
                    }
                }
                Err(error) => eprintln!(
                    "WARNING: isolated-schema cleanup could not run ({error}); \
                     leaked schemas: {}",
                    schemas.join(", ")
                ),
            }
        }
    }));
    if result.is_err() {
        eprintln!("WARNING: isolated-schema cleanup panicked; schemas may remain");
    }
}

fn postgres_executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn postgres_tool_path(bin_dir: &Path, name: &str) -> PathBuf {
    let executable = postgres_executable_name(name);
    if !bin_dir.as_os_str().is_empty() {
        return bin_dir.join(executable);
    }
    if let Some(pgbin) = std::env::var_os("PGBIN").filter(|value| !value.is_empty()) {
        return PathBuf::from(pgbin).join(executable);
    }
    #[cfg(windows)]
    {
        let default = PathBuf::from("C:/Program Files/PostgreSQL/16/bin").join(&executable);
        if default.is_file() {
            return default;
        }
    }
    PathBuf::from(executable)
}

impl TemplateHarnessCleanup {
    fn run(&mut self) -> Result<(), String> {
        let mut command = std::process::Command::new(&self.psql);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command
            .arg("-X")
            .arg("-w")
            .arg("-h")
            .arg("127.0.0.1")
            .arg("-p")
            .arg(self.port.to_string())
            .arg("-U")
            .arg(&self.superuser)
            .arg("-d")
            .arg(&self.maintenance_database)
            .arg("-v")
            .arg("ON_ERROR_STOP=1")
            .stdin(Stdio::null());
        for database in self.databases.iter().rev() {
            command.arg("-c").arg(format!(
                "DROP DATABASE IF EXISTS {} WITH (FORCE)",
                generated_database_identifier(database)
            ));
        }
        let output = bounded_command_output(command, std::time::Duration::from_secs(120))?;
        let mut failures = Vec::new();
        if !output.status.success() {
            failures.push(format!(
                "psql exited with {}: {}{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        if let Err(error) = self.managed.stop_blocking() {
            failures.push(format!(
                "identity-gated managed PostgreSQL stop failed: {error}"
            ));
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}

fn bounded_command_output(
    mut command: std::process::Command,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to launch PostgreSQL cleanup command: {error}"))?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|error| format!("failed to collect PostgreSQL cleanup: {error}"));
            }
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "PostgreSQL cleanup command timed out after {timeout:?}"
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "failed while waiting for PostgreSQL cleanup: {error}"
                ));
            }
        }
    }
}

fn initialize_template_harness_cleanup() {
    TEMPLATE_HARNESS_CLEANUP_REGISTERED.get_or_init(|| {
        for variable in ["POSTGRES_TEST_URL", "DATABASE_URL"] {
            assert!(
                std::env::var(variable)
                    .ok()
                    .is_none_or(|value| value.trim().is_empty()),
                "{DATABASE_TEMPLATE_MODE_ENV} requires the process-owned local managed PostgreSQL cluster, not {variable}"
            );
        }
        assert!(
            std::env::var_os("HANDSHAKE_MANAGED_PG_DATA_DIR").is_some()
                && std::env::var_os("HANDSHAKE_MANAGED_PG_PORT").is_some(),
            "{DATABASE_TEMPLATE_MODE_ENV} requires explicit process-dedicated HANDSHAKE_MANAGED_PG_DATA_DIR and HANDSHAKE_MANAGED_PG_PORT values"
        );
        let managed = MANAGED_POSTGRES
            .get()
            .and_then(Option::as_ref)
            .expect("template mode requires initialized managed PostgreSQL");
        assert!(
            managed.is_managed() && managed.proven_local_endpoint().is_some(),
            "template mode requires this test process to start and prove its dedicated managed PostgreSQL cluster"
        );
        let config = ManagedPostgresConfig::from_env();
        let cleanup = TemplateHarnessCleanup {
            psql: postgres_tool_path(&config.bin_dir, "psql"),
            port: config.port,
            superuser: config.superuser.clone(),
            maintenance_database: config.database.clone(),
            databases: Vec::new(),
            managed,
        };
        let cleanup_slot = TEMPLATE_HARNESS_CLEANUP.get_or_init(|| StdMutex::new(None));
        *cleanup_slot
            .lock()
            .expect("lock template-harness cleanup registry") = Some(cleanup);
        // SAFETY: the callback has C ABI, never unwinds, captures no stack
        // state, and reads only process-lifetime statics.
        let registered = unsafe { atexit(clean_template_harness_at_process_exit) };
        assert_eq!(
            registered, 0,
            "failed to register PostgreSQL template-harness cleanup"
        );
    });
}

fn register_owned_database_cleanup(database: &str) {
    generated_database_identifier(database);
    let cleanup_slot = TEMPLATE_HARNESS_CLEANUP
        .get()
        .expect("template-harness cleanup must be initialized");
    cleanup_slot
        .lock()
        .expect("lock template-harness cleanup registry")
        .as_mut()
        .expect("template-harness cleanup remains armed")
        .databases
        .push(database.to_string());
}

fn generated_database_identifier(database: &str) -> String {
    let suffix = database
        .strip_prefix("knowledge_test_")
        .or_else(|| database.strip_prefix("hsk_test_template_"))
        .expect("owned test database must use an exact harness prefix");
    assert!(
        suffix.len() == 32
            && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
            && Uuid::parse_str(suffix).is_ok(),
        "owned test database must end in one UUID simple-form identifier"
    );
    format!("\"{database}\"")
}

/// Resolve the base database URL (no schema isolation yet).
pub async fn base_database_url() -> Option<String> {
    if database_template_mode_enabled() {
        return Some(task_owned_managed_postgres().await.database_url());
    }

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

/// Start and return the process-owned managed PostgreSQL used by database
/// template mode, arming joined identity-gated exit cleanup immediately.
///
/// This helper is intentionally fail-closed: template mode is a real-resource
/// proof path, so missing binaries or an adopted/external endpoint are errors,
/// never SKIP conditions.
pub async fn task_owned_managed_postgres() -> &'static ManagedPostgres {
    assert!(
        database_template_mode_enabled(),
        "task_owned_managed_postgres requires {DATABASE_TEMPLATE_MODE_ENV}=1"
    );
    for variable in ["POSTGRES_TEST_URL", "DATABASE_URL"] {
        assert!(
            std::env::var(variable)
                .ok()
                .is_none_or(|value| value.trim().is_empty()),
            "task-owned managed PostgreSQL forbids external {variable}"
        );
    }
    assert!(
        std::env::var_os("HANDSHAKE_MANAGED_PG_DATA_DIR").is_some()
            && std::env::var_os("HANDSHAKE_MANAGED_PG_PORT").is_some(),
        "task-owned managed PostgreSQL requires explicit data-dir and port"
    );

    let managed = MANAGED_POSTGRES
        .get_or_init(|| async {
            Some(
                ManagedPostgres::ensure_running(ManagedPostgresConfig::from_env())
                    .await
                    .expect("task-owned managed PostgreSQL must start"),
            )
        })
        .await
        .as_ref()
        .expect("task-owned managed PostgreSQL handle must exist");
    assert!(
        managed.is_managed() && managed.proven_local_endpoint().is_some(),
        "task-owned managed PostgreSQL must be started and proven by this process"
    );
    initialize_template_harness_cleanup();
    managed
}

/// A per-test isolated knowledge database on the real cluster.
pub struct KnowledgePg {
    /// Concrete Postgres backend (KnowledgeStore + Database are implemented
    /// on it) connected with `search_path` pinned to the isolated schema.
    pub db: PostgresDatabase,
    /// The isolated schema name.
    pub schema: String,
    /// Connection URL for the isolated database without a forced search path.
    pub database_url: String,
    /// Connection URL pinned to the isolated schema.
    pub schema_url: String,
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
}

/// Build a fresh isolated schema + run all migrations on the real cluster.
///
/// Returns `None` only when PostgreSQL binaries are absent (caller must SKIP
/// loudly). Panics on every other failure.
pub async fn knowledge_pg() -> Option<KnowledgePg> {
    let url = base_database_url().await?;

    let _setup_guard = SCHEMA_SETUP_LOCK.lock().await;

    if database_template_mode_enabled() {
        return Some(knowledge_pg_from_database_template(&url).await);
    }

    let schema = format!("knowledge_test_{}", Uuid::now_v7().simple());
    let mut conn = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect to PostgreSQL for schema setup");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await
        .expect("create isolated knowledge test schema");
    // Register for teardown immediately after creation, so a schema is dropped
    // even if migrations or the test itself later panic.
    register_test_schema_for_cleanup(&url, &schema);
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
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");

    let db = PostgresDatabase::connect(&schema_url, 5)
        .await
        .expect("connect PostgresDatabase to isolated knowledge schema");
    db.run_migrations()
        .await
        .expect("run full migration chain in isolated knowledge schema");

    Some(KnowledgePg {
        db,
        schema,
        database_url: url,
        schema_url,
    })
}

fn database_template_mode_enabled() -> bool {
    std::env::var(DATABASE_TEMPLATE_MODE_ENV)
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !(value.is_empty()
                || value == "0"
                || value == "false"
                || value == "no"
                || value == "off")
        })
        .unwrap_or(false)
}

async fn knowledge_pg_from_database_template(base_url: &str) -> KnowledgePg {
    let template_database = MIGRATED_TEMPLATE_DATABASE
        .get_or_init(|| async { create_migrated_template_database(base_url).await })
        .await;
    let database = format!("knowledge_test_{}", Uuid::now_v7().simple());
    let schema = database.clone();

    let mut admin = sqlx::PgConnection::connect(base_url)
        .await
        .expect("connect to PostgreSQL for isolated template clone");
    sqlx::query(&format!(
        "CREATE DATABASE {} WITH TEMPLATE {}",
        generated_database_identifier(&database),
        generated_database_identifier(template_database)
    ))
    .execute(&mut admin)
    .await
    .expect("clone migrated PostgreSQL test template");
    register_owned_database_cleanup(&database);
    drop(admin);

    let database_url = postgres_database_url(base_url, &database, None);
    let mut clone_admin = sqlx::PgConnection::connect(&database_url)
        .await
        .expect("connect isolated cloned PostgreSQL database");
    sqlx::query(&format!(
        "ALTER SCHEMA {TEMPLATE_AUTHORITY_SCHEMA} RENAME TO {schema}"
    ))
    .execute(&mut clone_admin)
    .await
    .expect("rename cloned authority schema");
    drop(clone_admin);

    let schema_url = postgres_database_url(base_url, &database, Some(&schema));
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&schema_url)
        .await
        .expect("connect PostgresDatabase to cloned isolated authority");
    let db = PostgresDatabase::new(pool);

    KnowledgePg {
        db,
        schema,
        database_url,
        schema_url,
    }
}

async fn create_migrated_template_database(base_url: &str) -> String {
    initialize_template_harness_cleanup();
    let template_database = format!("hsk_test_template_{}", Uuid::now_v7().simple());
    let mut admin = sqlx::PgConnection::connect(base_url)
        .await
        .expect("connect to PostgreSQL for migrated template creation");
    let server_version_num: i32 =
        sqlx::query_scalar("SELECT pg_catalog.current_setting('server_version_num')::INTEGER")
            .fetch_one(&mut admin)
            .await
            .expect("read PostgreSQL server version for template harness");
    assert!(
        server_version_num >= 130_000,
        "template mode requires PostgreSQL 13+ for bounded DROP DATABASE WITH (FORCE); got {server_version_num}"
    );
    sqlx::query(&format!(
        "CREATE DATABASE {} WITH TEMPLATE template0",
        generated_database_identifier(&template_database)
    ))
    .execute(&mut admin)
    .await
    .expect("create empty PostgreSQL test template database");
    register_owned_database_cleanup(&template_database);
    drop(admin);

    let template_url = postgres_database_url(base_url, &template_database, None);
    let mut bootstrap = sqlx::PgConnection::connect(&template_url)
        .await
        .expect("connect empty PostgreSQL test template");
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public")
        .execute(&mut bootstrap)
        .await
        .expect("ensure pgcrypto extension in PostgreSQL test template");
    sqlx::query(&format!("CREATE SCHEMA {TEMPLATE_AUTHORITY_SCHEMA}"))
        .execute(&mut bootstrap)
        .await
        .expect("create authority schema in PostgreSQL test template");
    for shim in [
        format!(
            r#"
            CREATE OR REPLACE FUNCTION {TEMPLATE_AUTHORITY_SCHEMA}.digest(input text, algorithm text)
            RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE
            AS $$ SELECT public.digest(input::bytea, algorithm) $$
            "#
        ),
        format!(
            r#"
            CREATE OR REPLACE FUNCTION {TEMPLATE_AUTHORITY_SCHEMA}.digest(input bytea, algorithm text)
            RETURNS bytea LANGUAGE SQL IMMUTABLE PARALLEL SAFE
            AS $$ SELECT public.digest(input, algorithm) $$
            "#
        ),
    ] {
        sqlx::query(&shim)
            .execute(&mut bootstrap)
            .await
            .expect("install digest shim in PostgreSQL test template");
    }
    drop(bootstrap);

    let template_schema_url = postgres_database_url(
        base_url,
        &template_database,
        Some(TEMPLATE_AUTHORITY_SCHEMA),
    );
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&template_schema_url)
        .await
        .expect("connect migration pool to PostgreSQL test template");
    let db = PostgresDatabase::new(pool.clone());
    db.run_migrations()
        .await
        .expect("run full migration chain once in PostgreSQL test template");
    drop(db);
    pool.close().await;

    let mut admin = sqlx::PgConnection::connect(base_url)
        .await
        .expect("reconnect PostgreSQL after test template migration");
    let active_template_sessions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pg_catalog.pg_stat_activity WHERE datname = $1")
            .bind(&template_database)
            .fetch_one(&mut admin)
            .await
            .expect("count live sessions before sealing PostgreSQL test template");
    assert_eq!(
        active_template_sessions, 0,
        "PostgreSQL test template must have zero live sessions before cloning"
    );
    sqlx::query(&format!(
        "ALTER DATABASE {} WITH ALLOW_CONNECTIONS = FALSE",
        generated_database_identifier(&template_database)
    ))
    .execute(&mut admin)
    .await
    .expect("seal migrated PostgreSQL test template against direct connections");

    template_database
}

fn postgres_database_url(base_url: &str, database: &str, schema: Option<&str>) -> String {
    let mut url = reqwest::Url::parse(base_url).expect("PostgreSQL base URL must be valid");
    url.set_path(&format!("/{database}"));
    let existing_pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        query.extend_pairs(existing_pairs);
        if let Some(schema) = schema {
            query.append_pair("options[search_path]", schema);
        }
    }
    url.to_string()
}
