//! Runtime proof for the managed-PostgreSQL lifecycle (task #9): Handshake
//! starts its own hidden cluster (no popup window, no Docker), waits until it
//! accepts connections, and stops it cleanly. The test drives a REAL cluster on
//! a fresh temp data dir + dedicated port via the real PostgreSQL binaries; it
//! self-skips when the binaries are not discoverable (e.g. CI without Postgres).

use std::path::PathBuf;
use std::time::Duration;

use handshake_core::managed_postgres::{
    ManagedPostgres, ManagedPostgresConfig, ManagedPostgresError,
};
use sqlx::Connection;
use tokio::net::TcpStream;
use uuid::Uuid;

fn temp_data_dir() -> PathBuf {
    std::env::temp_dir().join(format!("hsk-managed-pg-it-{}", Uuid::new_v4()))
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
