//! MT-123 proof: killed-process reclamation and concurrent-owner protection.

mod knowledge_pg_support;

use knowledge_pg_support::{
    base_database_url, current_process_lease_identity, knowledge_pg,
    reclaim_orphaned_knowledge_schemas,
};
use sqlx::Connection;
use std::io::{BufRead, Write};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

const CHILD_MODE_ENV: &str = "HANDSHAKE_MT123_CHILD_MODE";
const CHILD_SCHEMA_PREFIX: &str = "MT123_ORPHAN_SCHEMA=";
const CHILD_MODE_KILLED: &str = "killed";
const CHILD_MODE_LIVE: &str = "live";

#[test]
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
fn mt123_host_owner_identity_is_stable_and_uuid_shaped() {
    let first =
        current_process_lease_identity().expect("resolve first MT-123 host/process owner identity");
    let second = current_process_lease_identity()
        .expect("resolve repeated MT-123 host/process owner identity");
    assert_eq!(first.machine_id, second.machine_id);
    assert_eq!(first.pid, second.pid);
    assert_eq!(first.process_birth, second.process_birth);
    assert_ne!(first.machine_id, Uuid::nil());
    assert_eq!(first.machine_id.simple().to_string().len(), 32);
}

async fn schema_exists(base_url: &str, schema: &str) -> bool {
    let mut conn = sqlx::PgConnection::connect(base_url)
        .await
        .expect("connect for MT-123 schema existence proof");
    let exists =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
            .bind(schema)
            .fetch_one(&mut conn)
            .await
            .expect("query MT-123 schema existence");
    conn.close()
        .await
        .expect("close MT-123 schema existence connection");
    exists
}

#[tokio::test]
#[ignore = "MT-123 subprocess helper; launched and killed by the parent proof"]
async fn mt123_child_holds_schema_until_killed() {
    let Ok(mode) = std::env::var(CHILD_MODE_ENV) else {
        return;
    };
    if mode != CHILD_MODE_KILLED && mode != CHILD_MODE_LIVE {
        return;
    }
    let pg = knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-123 child proof");
    println!("{CHILD_SCHEMA_PREFIX}{}", pg.schema);
    std::io::stdout()
        .flush()
        .expect("flush MT-123 child schema marker");
    if mode == CHILD_MODE_KILLED {
        std::future::pending::<()>().await;
    }

    tokio::task::spawn_blocking(|| {
        let mut release = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut release)
            .expect("read MT-123 live-child release signal");
        assert_eq!(
            release.trim(),
            "release",
            "unexpected live-child release signal"
        );
    })
    .await
    .expect("join MT-123 live-child release reader");
    pg.teardown().await;
}

#[tokio::test]
async fn mt123_killed_process_orphan_is_reclaimed_automatically() {
    let Some(base_url) = base_database_url().await else {
        eprintln!("SKIP MT-123 killed-process proof: PostgreSQL binaries unavailable");
        return;
    };

    let executable = std::env::current_exe().expect("resolve current MT-123 test executable");
    let mut child = Command::new(executable)
        .arg("--exact")
        .arg("mt123_child_holds_schema_until_killed")
        .arg("--ignored")
        .arg("--nocapture")
        .env(CHILD_MODE_ENV, CHILD_MODE_KILLED)
        .env("POSTGRES_TEST_URL", &base_url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn MT-123 schema-owner child process");
    let stdout = child
        .stdout
        .take()
        .expect("capture MT-123 child process stdout");
    let mut lines = BufReader::new(stdout).lines();

    let schema_result = tokio::time::timeout(Duration::from_secs(300), async {
        while let Some(line) = lines
            .next_line()
            .await
            .expect("read MT-123 child process output")
        {
            if let Some(marker) = line.find(CHILD_SCHEMA_PREFIX) {
                return line[(marker + CHILD_SCHEMA_PREFIX.len())..]
                    .trim()
                    .to_string();
            }
        }
        panic!("MT-123 child exited before publishing its schema");
    })
    .await;
    let schema = match schema_result {
        Ok(schema) => schema,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            panic!("MT-123 child did not publish its schema within 300 seconds");
        }
    };

    child
        .kill()
        .await
        .expect("kill MT-123 child process without running Drop");
    child
        .wait()
        .await
        .expect("reap killed MT-123 child process");

    assert!(
        schema_exists(&base_url, &schema).await,
        "an aborted process must reproduce the real leaked-schema failure before reclamation"
    );

    let mut reclaimed = false;
    for _ in 0..80 {
        let report = reclaim_orphaned_knowledge_schemas(&base_url)
            .await
            .expect("run MT-123 startup reclamation");
        assert!(
            report.failures.is_empty(),
            "MT-123 reclamation failures: {:?}",
            report.failures
        );
        if report
            .reclaimed
            .iter()
            .any(|candidate| candidate == &schema)
        {
            reclaimed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    assert!(reclaimed, "the killed process orphan was not reclaimed");
    assert!(
        !schema_exists(&base_url, &schema).await,
        "the reclaimed killed-process schema must be absent from pg_namespace"
    );
}

#[tokio::test]
async fn mt123_reclaimer_defers_an_unlocked_owner_with_a_fresh_heartbeat() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP MT-123 reconnect-window proof: PostgreSQL binaries unavailable");
        return;
    };
    let Some(base_url) = base_database_url().await else {
        panic!("managed PostgreSQL disappeared during MT-123 reconnect-window proof");
    };
    let schema = format!("knowledge_test_{}", Uuid::now_v7().simple());
    let owner_id = Uuid::now_v7();
    let owner =
        current_process_lease_identity().expect("resolve MT-123 reconnect-window owner identity");
    let mut conn = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect for MT-123 reconnect-window proof");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await
        .expect("create MT-123 reconnect-window schema");
    sqlx::query(
        "INSERT INTO public.handshake_knowledge_test_schema_leases_v2 \
         (schema_name, owner_id, owner_application_name, owner_machine_id, owner_pid, \
          owner_process_birth) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&schema)
    .bind(owner_id)
    .bind("mt123_owner_reconnecting")
    .bind(owner.machine_id)
    .bind(owner.pid)
    .bind(owner.process_birth)
    .execute(&mut conn)
    .await
    .expect("record MT-123 reconnect-window lease");

    let report = reclaim_orphaned_knowledge_schemas(&base_url)
        .await
        .expect("run MT-123 reclamation during reconnect window");
    assert!(
        report.failures.is_empty(),
        "MT-123 reconnect-window reclamation failures: {:?}",
        report.failures
    );
    assert!(
        report
            .deferred_fresh_heartbeat
            .iter()
            .any(|candidate| candidate == &schema),
        "an unlocked lease with a fresh heartbeat must be deferred while its owner reconnects"
    );
    assert!(
        schema_exists(&base_url, &schema).await,
        "the fresh-heartbeat reconnect window must protect the schema"
    );

    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&mut conn)
        .await
        .expect("drop MT-123 reconnect-window schema");
    sqlx::query(
        "DELETE FROM public.handshake_knowledge_test_schema_leases_v2 WHERE schema_name = $1",
    )
    .bind(&schema)
    .execute(&mut conn)
    .await
    .expect("remove MT-123 reconnect-window lease");
    conn.close()
        .await
        .expect("close MT-123 reconnect-window connection");
    pg.teardown().await;
}

#[tokio::test]
async fn mt123_reclaimer_preserves_a_stale_unlocked_lease_owned_by_this_live_process() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP MT-123 live-process fencing proof: PostgreSQL binaries unavailable");
        return;
    };
    let Some(base_url) = base_database_url().await else {
        panic!("managed PostgreSQL disappeared during MT-123 live-process fencing proof");
    };
    let schema = format!("knowledge_test_{}", Uuid::now_v7().simple());
    let owner_id = Uuid::now_v7();
    let owner = current_process_lease_identity()
        .expect("resolve current live process identity for MT-123 fencing proof");
    let mut conn = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect for MT-123 live-process fencing proof");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await
        .expect("create MT-123 live-process fencing schema");
    sqlx::query(
        "INSERT INTO public.handshake_knowledge_test_schema_leases_v2 \
         (schema_name, owner_id, owner_application_name, owner_machine_id, owner_pid, \
          owner_process_birth, heartbeat_at) \
         VALUES ($1, $2, $3, $4, $5, $6, clock_timestamp() - interval '1 hour')",
    )
    .bind(&schema)
    .bind(owner_id)
    .bind("mt123_stale_disconnected_live_process")
    .bind(owner.machine_id)
    .bind(owner.pid)
    .bind(owner.process_birth)
    .execute(&mut conn)
    .await
    .expect("record MT-123 stale unlocked live-process lease");

    let report = reclaim_orphaned_knowledge_schemas(&base_url)
        .await
        .expect("run MT-123 reclamation against stale live-process lease");
    assert!(
        report.failures.is_empty(),
        "MT-123 stale live-process reclamation failures: {:?}",
        report.failures
    );
    assert!(
        report
            .protected_live
            .iter()
            .any(|candidate| candidate == &schema),
        "a stale unlocked lease must remain protected while its exact owner process is alive"
    );
    assert!(
        schema_exists(&base_url, &schema).await,
        "positive owner-death fencing must preserve a disconnected live process's schema"
    );

    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&mut conn)
        .await
        .expect("drop MT-123 live-process fencing schema");
    sqlx::query(
        "DELETE FROM public.handshake_knowledge_test_schema_leases_v2 WHERE schema_name = $1",
    )
    .bind(&schema)
    .execute(&mut conn)
    .await
    .expect("remove MT-123 live-process fencing lease");
    conn.close()
        .await
        .expect("close MT-123 live-process fencing connection");
    pg.teardown().await;
}

#[tokio::test]
async fn mt123_reclaimer_defers_a_stale_owner_from_another_machine() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP MT-123 foreign-machine fencing proof: PostgreSQL binaries unavailable");
        return;
    };
    let Some(base_url) = base_database_url().await else {
        panic!("managed PostgreSQL disappeared during MT-123 foreign-machine fencing proof");
    };
    let schema = format!("knowledge_test_{}", Uuid::now_v7().simple());
    let owner_id = Uuid::now_v7();
    let owner = current_process_lease_identity()
        .expect("resolve current identity for MT-123 foreign-machine fencing proof");
    let foreign_machine_id = loop {
        let candidate = Uuid::now_v7();
        if candidate != owner.machine_id {
            break candidate;
        }
    };
    let mut conn = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect for MT-123 foreign-machine fencing proof");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await
        .expect("create MT-123 foreign-machine fencing schema");
    sqlx::query(
        "INSERT INTO public.handshake_knowledge_test_schema_leases_v2 \
         (schema_name, owner_id, owner_application_name, owner_machine_id, owner_pid, \
          owner_process_birth, heartbeat_at) \
         VALUES ($1, $2, $3, $4, $5, $6, clock_timestamp() - interval '1 hour')",
    )
    .bind(&schema)
    .bind(owner_id)
    .bind("mt123_stale_foreign_machine")
    .bind(foreign_machine_id)
    .bind(owner.pid)
    .bind(owner.process_birth)
    .execute(&mut conn)
    .await
    .expect("record MT-123 stale foreign-machine lease");

    let report = reclaim_orphaned_knowledge_schemas(&base_url)
        .await
        .expect("run MT-123 reclamation against foreign-machine lease");
    assert!(
        report.failures.is_empty(),
        "MT-123 foreign-machine reclamation failures: {:?}",
        report.failures
    );
    assert!(
        report
            .deferred_foreign_machine
            .iter()
            .any(|candidate| candidate == &schema),
        "a foreign machine identity must defer instead of querying a coincidental local PID"
    );
    assert!(
        schema_exists(&base_url, &schema).await,
        "a foreign-machine lease must remain intact for its owning machine"
    );

    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&mut conn)
        .await
        .expect("drop MT-123 foreign-machine fencing schema");
    sqlx::query(
        "DELETE FROM public.handshake_knowledge_test_schema_leases_v2 WHERE schema_name = $1",
    )
    .bind(&schema)
    .execute(&mut conn)
    .await
    .expect("remove MT-123 foreign-machine fencing lease");
    conn.close()
        .await
        .expect("close MT-123 foreign-machine fencing connection");
    pg.teardown().await;
}

#[tokio::test]
async fn mt123_reclaimer_protects_a_concurrent_live_owner() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP MT-123 live-owner proof: PostgreSQL binaries unavailable");
        return;
    };
    let schema = pg.schema.clone();
    let Some(base_url) = base_database_url().await else {
        panic!("managed PostgreSQL disappeared during MT-123 live-owner proof");
    };

    let report = reclaim_orphaned_knowledge_schemas(&base_url)
        .await
        .expect("run MT-123 reclamation beside a live owner");
    assert!(
        report.failures.is_empty(),
        "MT-123 live-owner reclamation failures: {:?}",
        report.failures
    );
    assert!(
        report
            .protected_live
            .iter()
            .any(|candidate| candidate == &schema),
        "the advisory ownership predicate must classify the live schema as protected"
    );
    assert!(
        schema_exists(&base_url, &schema).await,
        "reclamation must not drop a concurrently owned schema"
    );

    pg.teardown().await;
    assert!(
        !schema_exists(&base_url, &schema).await,
        "ordinary fixture teardown must still remove its own schema"
    );
}

#[tokio::test]
async fn mt123_reclaimer_protects_a_concurrent_process_owner() {
    let Some(base_url) = base_database_url().await else {
        eprintln!("SKIP MT-123 cross-process live-owner proof: PostgreSQL binaries unavailable");
        return;
    };
    let executable = std::env::current_exe().expect("resolve current MT-123 test executable");
    let mut child = Command::new(executable)
        .arg("--exact")
        .arg("mt123_child_holds_schema_until_killed")
        .arg("--ignored")
        .arg("--nocapture")
        .env(CHILD_MODE_ENV, CHILD_MODE_LIVE)
        .env("POSTGRES_TEST_URL", &base_url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn MT-123 live schema-owner child process");
    let stdout = child
        .stdout
        .take()
        .expect("capture MT-123 live child process stdout");
    let mut lines = BufReader::new(stdout).lines();
    let schema = tokio::time::timeout(Duration::from_secs(300), async {
        while let Some(line) = lines
            .next_line()
            .await
            .expect("read MT-123 live child process output")
        {
            if let Some(marker) = line.find(CHILD_SCHEMA_PREFIX) {
                return line[(marker + CHILD_SCHEMA_PREFIX.len())..]
                    .trim()
                    .to_string();
            }
        }
        panic!("MT-123 live child exited before publishing its schema");
    })
    .await
    .expect("MT-123 live child did not publish its schema within 300 seconds");

    let report = reclaim_orphaned_knowledge_schemas(&base_url)
        .await
        .expect("run MT-123 reclamation beside a separate live process");
    assert!(
        report.failures.is_empty(),
        "MT-123 cross-process reclamation failures: {:?}",
        report.failures
    );
    assert!(
        report
            .protected_live
            .iter()
            .any(|candidate| candidate == &schema),
        "the advisory ownership predicate must protect another process/worktree"
    );
    assert!(
        schema_exists(&base_url, &schema).await,
        "reclamation must not drop another process's live schema"
    );

    let mut stdin = child
        .stdin
        .take()
        .expect("open MT-123 live-child release channel");
    stdin
        .write_all(b"release\n")
        .await
        .expect("signal MT-123 live child to teardown cleanly");
    stdin
        .shutdown()
        .await
        .expect("close MT-123 live-child release channel");
    let status = tokio::time::timeout(Duration::from_secs(300), child.wait())
        .await
        .expect("MT-123 live child did not teardown within 300 seconds")
        .expect("wait for MT-123 live child teardown");
    assert!(
        status.success(),
        "MT-123 live child teardown failed: {status}"
    );
    assert!(
        !schema_exists(&base_url, &schema).await,
        "the separate live owner must remove its schema during clean teardown"
    );
}

#[tokio::test]
async fn mt123_reclaimer_defers_a_prior_postmaster_generation() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP MT-123 prior-postmaster proof: PostgreSQL binaries unavailable");
        return;
    };
    let Some(base_url) = base_database_url().await else {
        panic!("managed PostgreSQL disappeared during MT-123 prior-postmaster proof");
    };
    let schema = format!("knowledge_test_{}", Uuid::now_v7().simple());
    let owner_id = Uuid::now_v7();
    let owner =
        current_process_lease_identity().expect("resolve MT-123 prior-postmaster owner identity");
    let mut conn = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect for MT-123 prior-postmaster proof");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await
        .expect("create MT-123 prior-postmaster schema");
    sqlx::query(
        "INSERT INTO public.handshake_knowledge_test_schema_leases_v2 \
         (schema_name, owner_id, owner_application_name, owner_machine_id, owner_pid, \
          owner_process_birth, heartbeat_at, server_started_at) \
         VALUES ($1, $2, $3, $4, $5, $6, clock_timestamp() - interval '1 hour', \
                 pg_postmaster_start_time() - interval '1 second')",
    )
    .bind(&schema)
    .bind(owner_id)
    .bind("mt123_prior_postmaster_owner")
    .bind(owner.machine_id)
    .bind(owner.pid)
    .bind(owner.process_birth)
    .execute(&mut conn)
    .await
    .expect("record MT-123 prior-postmaster lease");

    let report = reclaim_orphaned_knowledge_schemas(&base_url)
        .await
        .expect("run MT-123 reclamation for prior-postmaster lease");
    assert!(
        report.failures.is_empty(),
        "MT-123 prior-postmaster reclamation failures: {:?}",
        report.failures
    );
    assert!(
        report
            .deferred_server_restart
            .iter()
            .any(|candidate| candidate == &schema),
        "a released lock from an older postmaster is not proof of owner death"
    );
    assert!(
        schema_exists(&base_url, &schema).await,
        "a prior-postmaster lease must be preserved for explicit recovery"
    );

    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&mut conn)
        .await
        .expect("drop MT-123 prior-postmaster proof schema");
    sqlx::query(
        "DELETE FROM public.handshake_knowledge_test_schema_leases_v2 WHERE schema_name = $1",
    )
    .bind(&schema)
    .execute(&mut conn)
    .await
    .expect("remove MT-123 prior-postmaster proof lease");
    conn.close()
        .await
        .expect("close MT-123 prior-postmaster connection");
    pg.teardown().await;
}
