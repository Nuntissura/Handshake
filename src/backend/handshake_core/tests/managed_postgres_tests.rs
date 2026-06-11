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
            eprintln!("SKIP managed_postgres lifecycle test: PostgreSQL binaries not found ({detail})");
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
