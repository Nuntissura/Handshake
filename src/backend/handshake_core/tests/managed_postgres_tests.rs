//! Runtime proof for the managed-PostgreSQL lifecycle (task #9): Handshake
//! starts its own hidden cluster (no popup window, no Docker), waits until it
//! accepts connections, and stops it cleanly. The test drives a REAL cluster on
//! a fresh temp data dir + dedicated port via the real PostgreSQL binaries; it
//! self-skips when the binaries are not discoverable (e.g. CI without Postgres).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use handshake_core::managed_postgres::{
    ManagedPostgres, ManagedPostgresConfig, ManagedPostgresError,
};
use handshake_core::process_ledger::{
    acquire_embedded_runtime_instance_lease, reclaim_pidless_embedded_orphans,
    resolve_embedded_runtime_host_scope_with_managed_local,
    verify_proven_local_postgres_endpoint_pool,
    EMBEDDED_RUNTIME_MANAGED_LOCAL_HOST_SCOPE_V2_PREFIX,
};
use sqlx::Connection;
use tokio::net::TcpStream;
use tokio::process::Command;
use uuid::Uuid;

mod knowledge_pg_support;

fn temp_data_dir() -> PathBuf {
    std::env::temp_dir().join(format!("hsk-managed-pg-it-{}", Uuid::new_v4()))
}

fn unused_loopback_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .expect("bind ephemeral loopback port")
        .local_addr()
        .expect("read ephemeral loopback address")
        .port()
}

#[tokio::test]
async fn task_owned_test_harness_normal_exit_joins_exact_managed_postgres_stop() {
    const CHILD_ENV: &str = "HANDSHAKE_MANAGED_PG_EXIT_CHILD";
    if std::env::var(CHILD_ENV).ok().as_deref() == Some("1") {
        let managed = knowledge_pg_support::task_owned_managed_postgres().await;
        assert!(managed.is_managed());
        assert!(managed.proven_local_endpoint().is_some());
        return;
    }

    let data_dir = temp_data_dir();
    let port = unused_loopback_port();
    let mut cleanup =
        TestClusterCleanup::new_for_port(pg_ctl_test_path(Path::new("")), data_dir.clone(), port);
    let current_exe = std::env::current_exe().expect("resolve current managed-PG test binary");
    let mut child = quiet_test_command(&current_exe);
    child
        .kill_on_drop(true)
        .arg("--exact")
        .arg("task_owned_test_harness_normal_exit_joins_exact_managed_postgres_stop")
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .env("HANDSHAKE_MANAGED_PG_ENABLED", "1")
        .env("HANDSHAKE_MANAGED_PG_DATA_DIR", &data_dir)
        .env("HANDSHAKE_MANAGED_PG_PORT", port.to_string())
        .env("HANDSHAKE_TEST_PG_DATABASE_TEMPLATE", "1")
        .env_remove("POSTGRES_TEST_URL")
        .env_remove("DATABASE_URL")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = tokio::time::timeout(Duration::from_secs(180), child.output())
        .await
        .expect("child managed-PG exit-cleanup proof exceeded 180 seconds")
        .expect("run child managed-PG exit-cleanup proof");
    assert!(
        output.status.success(),
        "child managed-PG proof failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        TcpStream::connect(("127.0.0.1", port)).await.is_err(),
        "task-owned PostgreSQL port remained ready after normal child exit"
    );
    assert!(
        !data_dir.join("postmaster.pid").exists(),
        "task-owned PostgreSQL postmaster.pid remained after normal child exit"
    );
    cleanup.disarm();
    std::fs::remove_dir_all(&data_dir)
        .expect("remove exact stopped task-owned PostgreSQL test data directory");
}

fn pg_ctl_test_path(bin_dir: &Path) -> PathBuf {
    let executable = if cfg!(windows) {
        "pg_ctl.exe"
    } else {
        "pg_ctl"
    };
    if !bin_dir.as_os_str().is_empty() {
        return bin_dir.join(executable);
    }
    if let Some(pgbin) = std::env::var_os("PGBIN").filter(|value| !value.is_empty()) {
        return PathBuf::from(pgbin).join(executable);
    }
    #[cfg(windows)]
    {
        let default = PathBuf::from("C:/Program Files/PostgreSQL/16/bin").join(executable);
        if default.is_file() {
            return default;
        }
    }
    PathBuf::from(executable)
}

fn quiet_test_command(program: &Path) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

struct TestClusterCleanup {
    pg_ctl: PathBuf,
    data_dir: PathBuf,
    expected_port: Option<u16>,
    armed: bool,
}

impl TestClusterCleanup {
    fn new(pg_ctl: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            pg_ctl,
            data_dir,
            expected_port: None,
            armed: true,
        }
    }

    fn new_for_port(pg_ctl: PathBuf, data_dir: PathBuf, expected_port: u16) -> Self {
        Self {
            pg_ctl,
            data_dir,
            expected_port: Some(expected_port),
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TestClusterCleanup {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(expected_port) = self.expected_port {
            let pid_file = self.data_dir.join("postmaster.pid");
            if pid_file.is_file() {
                let actual_port = std::fs::read_to_string(&pid_file)
                    .ok()
                    .and_then(|contents| contents.lines().nth(3)?.trim().parse::<u16>().ok());
                if actual_port != Some(expected_port) {
                    eprintln!(
                        "refusing contingency cleanup for {}: postmaster.pid port {:?} does not match exact test port {}",
                        self.data_dir.display(),
                        actual_port,
                        expected_port
                    );
                    return;
                }
            }
        }
        let mut command = std::process::Command::new(&self.pg_ctl);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let spawned = command
            .arg("-D")
            .arg(&self.data_dir)
            .arg("stop")
            .arg("-m")
            .arg("fast")
            .arg("-w")
            .arg("-t")
            .arg("10")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = spawned else {
            return;
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Ok(None) | Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
    }
}

async fn force_stop_test_cluster(pg_ctl: &Path, data_dir: &Path) -> std::io::Result<()> {
    let status = tokio::time::timeout(
        Duration::from_secs(50),
        quiet_test_command(pg_ctl)
            .kill_on_drop(true)
            .arg("-D")
            .arg(data_dir)
            .arg("stop")
            .arg("-m")
            .arg("fast")
            .arg("-w")
            .arg("-t")
            .arg("45")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    )
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "pg_ctl stop timed out"))??;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "pg_ctl stop exited with {status}"
        )))
    }
}

#[tokio::test]
async fn managed_postgres_starts_accepts_connections_and_stops() {
    let data_dir = temp_data_dir();
    let config = ManagedPostgresConfig {
        enabled: true,
        data_dir: data_dir.clone(),
        // Dedicated port distinct from the operator cluster (5544) so the test
        // never collides with a manually-run instance.
        port: 5546,
        bin_dir: PathBuf::new(), // empty -> discovery (PGBIN / PG16 default / PATH)
        database: "handshake_mpg_it".to_string(),
        superuser: "postgres".to_string(),
        startup_timeout: Duration::from_secs(45),
    };

    let managed = match ManagedPostgres::ensure_running(config).await {
        Ok(m) => m,
        Err(ManagedPostgresError::BinariesNotFound(detail)) => {
            eprintln!(
                "SKIP managed_postgres lifecycle test: PostgreSQL binaries not found ({detail})"
            );
            let _ = std::fs::remove_dir_all(&data_dir);
            return;
        }
        Err(err) => panic!("ensure_running failed: {err}"),
    };
    let mut cleanup = TestClusterCleanup::new(pg_ctl_test_path(Path::new("")), data_dir.clone());

    // A fresh temp data dir means we initdb + start the cluster ourselves:
    // reaching Ok proves initdb succeeded, pg_ctl started the postmaster, and
    // the internal pg_isready poll observed the server accepting connections.
    assert!(
        managed.is_managed(),
        "a fresh temp data dir must be Handshake-started (is_managed), not adopted"
    );
    assert!(managed.is_enabled(), "config was enabled");
    assert!(
        managed.os_pid().is_some(),
        "a Handshake-started cluster must expose its postmaster PID"
    );

    let url = managed.database_url();
    assert_eq!(
        url, "postgres://postgres@127.0.0.1:5546/handshake_mpg_it",
        "database_url must reflect the configured superuser/port/database"
    );

    // Idempotency: a second ensure_running against the SAME running cluster must
    // adopt it (pg_isready already exit 0) rather than double-start.
    let adopt_config = ManagedPostgresConfig {
        enabled: true,
        data_dir: data_dir.clone(),
        port: 5546,
        bin_dir: PathBuf::new(),
        database: "handshake_mpg_it".to_string(),
        superuser: "postgres".to_string(),
        startup_timeout: Duration::from_secs(45),
    };
    let adopted = ManagedPostgres::ensure_running(adopt_config)
        .await
        .expect("second ensure_running against a live cluster must succeed");
    assert!(
        !adopted.is_managed(),
        "an already-running cluster must be adopted, never double-started"
    );

    // Clean teardown.
    managed.stop().await.expect("managed stop must succeed");
    cleanup.disarm();

    let _ = std::fs::remove_dir_all(&data_dir);
}

/// Real crash-recovery provenance proof: lifecycle A starts the configured
/// data-dir cluster and disappears without calling stop. Lifecycle B must adopt
/// that surviving postmaster without acquiring shutdown ownership, while still
/// receiving an opaque proven-local endpoint token that derives the exact same
/// distinguishable v2 host scope.
#[tokio::test]
async fn crash_survivor_is_nonowning_but_proven_local_with_stable_v2_scope() {
    let data_dir = temp_data_dir();
    let port = 5552;
    let discovered_bin_dir = ManagedPostgresConfig::from_env().bin_dir;
    let config = ManagedPostgresConfig {
        enabled: true,
        data_dir: data_dir.clone(),
        port,
        bin_dir: discovered_bin_dir.clone(),
        database: "handshake_mpg_crash_adoption".to_owned(),
        superuser: "postgres".to_owned(),
        startup_timeout: Duration::from_secs(45),
    };

    let lifecycle_a = match ManagedPostgres::ensure_running(config.clone()).await {
        Ok(managed) => managed,
        Err(ManagedPostgresError::BinariesNotFound(detail)) => {
            let _ = std::fs::remove_dir_all(&data_dir);
            panic!(
                "managed-postgres crash-adoption proof requires real PostgreSQL binaries: {detail}"
            );
        }
        Err(error) => panic!("lifecycle A failed to start managed PostgreSQL: {error}"),
    };
    let pg_ctl = pg_ctl_test_path(&discovered_bin_dir);
    let mut cleanup = TestClusterCleanup::new(pg_ctl.clone(), data_dir.clone());
    assert!(
        lifecycle_a.is_managed(),
        "fresh lifecycle A must own the postmaster it starts"
    );
    let scope_a = resolve_embedded_runtime_host_scope_with_managed_local(
        &lifecycle_a.database_url(),
        None,
        lifecycle_a.proven_local_endpoint(),
    )
    .expect("started-here lifecycle must derive a proven-local v2 host scope");
    assert!(scope_a.starts_with(EMBEDDED_RUNTIME_MANAGED_LOCAL_HOST_SCOPE_V2_PREFIX));

    // A reachable local port is not enough. Pointing the lifecycle at a
    // different real directory while A owns the SQL endpoint must fail on the
    // endpoint-reported data_directory before any provisioning SQL runs.
    let wrong_data_dir = temp_data_dir();
    std::fs::create_dir_all(&wrong_data_dir).expect("create mismatched data-dir fixture");
    let mut wrong_config = config.clone();
    wrong_config.data_dir = wrong_data_dir.clone();
    let mismatch = ManagedPostgres::ensure_running(wrong_config)
        .await
        .expect_err("reachable SQL endpoint with a different data directory must not prove");
    match mismatch {
        ManagedPostgresError::LocalEndpointProofFailed(detail) => assert!(
            detail.contains("SQL endpoint data_directory")
                && detail.contains("does not match configured data directory"),
            "mismatch must be diagnosed from the real SQL endpoint: {detail}"
        ),
        other => panic!("expected local-endpoint proof failure, got {other}"),
    }
    let _ = std::fs::remove_dir_all(&wrong_data_dir);

    // Model a hard Handshake crash: dropping the handle does not run the
    // orderly stop path, so the postmaster remains alive in the configured
    // data directory.
    drop(lifecycle_a);
    TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("postmaster must survive lifecycle A disappearing without stop");

    // Use a path alias for the same directory. Scope stability must come from
    // PostgreSQL's system_identifier, while canonical proof still accepts the
    // equivalent data-dir spelling.
    let mut adoption_config = config.clone();
    adoption_config.data_dir = data_dir.join(".");
    let lifecycle_b = match ManagedPostgres::ensure_running(adoption_config).await {
        Ok(managed) => managed,
        Err(error) => {
            let _ = force_stop_test_cluster(&pg_ctl, &data_dir).await;
            panic!("lifecycle B failed to adopt crash-surviving PostgreSQL: {error}");
        }
    };
    assert!(
        !lifecycle_b.is_managed(),
        "adoption must not grant lifecycle B shutdown ownership"
    );
    assert!(
        lifecycle_b.proven_local_endpoint().is_some(),
        "pg_ctl/postmaster.pid/data-dir/port proof must survive non-owning adoption"
    );
    let scope_b = resolve_embedded_runtime_host_scope_with_managed_local(
        &lifecycle_b.database_url(),
        None,
        lifecycle_b.proven_local_endpoint(),
    )
    .expect("adopted lifecycle must derive a proven-local v2 host scope");
    assert_eq!(
        scope_a, scope_b,
        "crash adoption of the same data-dir endpoint must preserve host scope"
    );

    // Drive the recovered scope through the real PostgreSQL reclaimer too. A
    // released UDP lease represents the crashed embedded runtime; the adopted
    // scope must select and close its stale START row in the isolated ledger.
    let schema = format!("mt013_crash_adopt_{}", Uuid::now_v7().simple());
    let mut admin = sqlx::PgConnection::connect(&lifecycle_b.database_url())
        .await
        .expect("connect adopted PostgreSQL for crash-reclaim fixture");
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&mut admin)
        .await
        .expect("create isolated crash-reclaim schema");
    drop(admin);
    let schema_url = format!(
        "{}?options=-csearch_path%3D{}",
        lifecycle_b.database_url(),
        schema
    );
    let pool = sqlx::PgPool::connect(&schema_url)
        .await
        .expect("connect isolated crash-reclaim schema");
    verify_proven_local_postgres_endpoint_pool(
        &pool,
        lifecycle_b
            .proven_local_endpoint()
            .expect("adopted endpoint proof remains available"),
    )
    .await
    .expect("live control-plane pool must match adopted system_identifier proof");
    sqlx::raw_sql(include_str!(
        "../migrations/0021_kernel_process_lifecycle.sql"
    ))
    .execute(&pool)
    .await
    .expect("apply lifecycle authority migration");
    sqlx::raw_sql(
        r#"
        CREATE TABLE kernel_pidless_embedded_reclaim_cursor (
            host_scope_id TEXT PRIMARY KEY,
            last_instance_id TEXT,
            updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT pg_catalog.clock_timestamp()
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("create durable pid-less reclaim cursor");

    let lease = acquire_embedded_runtime_instance_lease(Uuid::now_v7(), &scope_b)
        .expect("acquire crash fixture runtime lease");
    let stale_descriptor = lease.descriptor().clone();
    drop(lease);
    let stale_process_uuid = Uuid::now_v7();
    let reclaim_cutoff = chrono::Utc::now();
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle
            (process_uuid, os_pid, parent_session_id, engine_kind, started_at,
             stopped_at, owner_role, metadata_jsonb)
        VALUES ($1, NULL, NULL, 'candle', $2, NULL, 'mt013-crash-adoption', $3)
        "#,
    )
    .bind(stale_process_uuid)
    .bind(reclaim_cutoff - chrono::Duration::minutes(5))
    .bind(stale_descriptor.metadata_fields())
    .execute(&pool)
    .await
    .expect("insert stale crash-survivor ledger row");
    let reclaim_report = reclaim_pidless_embedded_orphans(&pool, reclaim_cutoff, &scope_b)
        .await
        .expect("adopted proven-local scope must run real stale-row reclaim");
    assert!(
        reclaim_report.is_complete(),
        "crash-adoption reclaim must complete: {reclaim_report:?}"
    );
    assert_eq!(reclaim_report.closed_rows, 1);
    let stopped_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT stopped_at FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(stale_process_uuid)
    .fetch_one(&pool)
    .await
    .expect("read reclaimed crash-survivor row");
    assert!(stopped_at.is_some(), "stale crash row must be terminal");

    lifecycle_b
        .stop()
        .await
        .expect("non-owning adopted stop remains an idempotent no-op");
    TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("adopted stop must not terminate the surviving postmaster");
    pool.close().await;
    drop(lifecycle_b);

    // A no longer exists to perform orderly shutdown, so the integration test
    // invokes pg_ctl directly only for fixture cleanup.
    force_stop_test_cluster(&pg_ctl, &data_dir)
        .await
        .expect("crash-adoption fixture cleanup must stop the postmaster");
    cleanup.disarm();
    let _ = std::fs::remove_dir_all(&data_dir);
}

/// An adopted cluster may be accepting TCP connections while the Handshake
/// application database is absent.  Exercise that exact real-binary path:
/// start a pristine managed cluster, prove the second database is absent, then
/// adopt the running cluster with that missing database as the requested target
/// and connect to it afterwards.
#[tokio::test]
async fn managed_postgres_adoption_provisions_missing_application_database_and_connects() {
    let data_dir = temp_data_dir();
    let port = 5548;
    let seed_database = "handshake_mpg_adoption_seed";
    let adopted_database = "handshake_mpg_adoption_target";
    let config = ManagedPostgresConfig {
        enabled: true,
        data_dir: data_dir.clone(),
        port,
        bin_dir: PathBuf::new(),
        database: seed_database.to_owned(),
        superuser: "postgres".to_owned(),
        startup_timeout: Duration::from_secs(45),
    };

    let managed = match ManagedPostgres::ensure_running(config).await {
        Ok(managed) => managed,
        Err(ManagedPostgresError::BinariesNotFound(detail)) => {
            eprintln!(
                "SKIP managed_postgres adoption provisioning test: PostgreSQL binaries not found ({detail})"
            );
            let _ = std::fs::remove_dir_all(&data_dir);
            return;
        }
        Err(err) => panic!("seed managed cluster failed: {err}"),
    };
    let mut cleanup = TestClusterCleanup::new(pg_ctl_test_path(Path::new("")), data_dir.clone());

    let admin_url = format!("postgres://postgres@127.0.0.1:{port}/postgres");
    let mut admin = sqlx::PgConnection::connect(&admin_url)
        .await
        .expect("fresh managed cluster postgres maintenance database must be connectable");
    let target_exists_before_adoption: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(adopted_database)
            .fetch_one(&mut admin)
            .await
            .expect("database existence query must succeed");
    assert!(
        !target_exists_before_adoption,
        "the adopted target must be absent before ensure_running provisions it"
    );
    drop(admin);

    let adopted = ManagedPostgres::ensure_running(ManagedPostgresConfig {
        enabled: true,
        data_dir: data_dir.clone(),
        port,
        bin_dir: PathBuf::new(),
        database: adopted_database.to_owned(),
        superuser: "postgres".to_owned(),
        startup_timeout: Duration::from_secs(45),
    })
    .await
    .expect("adopting a live cluster must create and verify its missing application database");
    assert!(
        !adopted.is_managed(),
        "the second lifecycle handle must adopt the already-running cluster"
    );
    assert_eq!(
        adopted.database_url(),
        format!("postgres://postgres@127.0.0.1:{port}/{adopted_database}")
    );

    let mut target = sqlx::PgConnection::connect(&adopted.database_url())
        .await
        .expect("the adopted lifecycle must leave the newly provisioned target connectable");
    let one: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&mut target)
        .await
        .expect("a direct query against the newly provisioned target must succeed");
    assert_eq!(one, 1);
    drop(target);

    managed
        .stop()
        .await
        .expect("the original managed lifecycle handle must stop the cluster");
    cleanup.disarm();
    let _ = std::fs::remove_dir_all(&data_dir);
}

/// A failed application-database provision happens after a fresh cluster has
/// started.  The lifecycle must stop that owned postmaster before returning;
/// otherwise a later test/product launch could silently adopt a stranded
/// process as though it were a healthy managed cluster.
#[tokio::test]
async fn managed_postgres_failed_provision_stops_owned_cluster() {
    let data_dir = temp_data_dir();
    let port = 5550;
    let result = ManagedPostgres::ensure_running(ManagedPostgresConfig {
        enabled: true,
        data_dir: data_dir.clone(),
        port,
        bin_dir: PathBuf::new(),
        // An embedded NUL is rejected by the real psql process spawn after
        // pg_ctl has started the cluster, deterministically exercising the
        // provision-failure cleanup path without a mock binary or database.
        database: "handshake_mpg_provision\0failure".to_owned(),
        superuser: "postgres".to_owned(),
        startup_timeout: Duration::from_secs(45),
    })
    .await;

    if let Err(ManagedPostgresError::BinariesNotFound(detail)) = &result {
        eprintln!(
            "SKIP managed-postgres provision-cleanup test: PostgreSQL binaries not found ({detail})"
        );
        let _ = std::fs::remove_dir_all(&data_dir);
        return;
    }
    assert!(
        result.is_err(),
        "an invalid psql argument must fail application-database provisioning"
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        match TcpStream::connect(("127.0.0.1", port)).await {
            Err(_) => break,
            Ok(stream) => {
                drop(stream);
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "failed provisioning must not leave the owned postmaster listening on {port}"
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn managed_postgres_disabled_does_not_spawn() {
    let config = ManagedPostgresConfig {
        enabled: false,
        data_dir: temp_data_dir(),
        port: 5547,
        bin_dir: PathBuf::new(),
        database: "handshake".to_string(),
        superuser: "postgres".to_string(),
        startup_timeout: Duration::from_secs(5),
    };

    let managed = ManagedPostgres::ensure_running(config)
        .await
        .expect("disabled config must succeed without spawning anything");
    assert!(!managed.is_enabled(), "config was disabled");
    assert!(
        !managed.is_managed(),
        "disabled lifecycle must not start a cluster"
    );
    assert!(managed.os_pid().is_none(), "nothing was spawned");
    // database_url is still derivable so the caller can use an external server.
    assert_eq!(
        managed.database_url(),
        "postgres://postgres@127.0.0.1:5547/handshake"
    );
}
