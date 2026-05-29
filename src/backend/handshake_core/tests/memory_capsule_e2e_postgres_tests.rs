//! MT-147 end-to-end Postgres integration test for the capsule build+inject chain.
//!
//! Composes the full MT-143 -> MT-144 -> MT-145 -> MT-146 spine over real
//! Postgres:
//!   CapsuleBuilder (MT-143 real adapter, fixture-backed FEMS)
//!     -> CapsuleInjector (MT-144 wired into the model-call boundary)
//!     -> CapsuleRecorder -> PostgresKernelActionSubmitter (MT-145 real catalog
//!         + kernel_event_ledger)
//!     -> MemoryIpcService over PostgresMemoryCapsuleStore (MT-146 durable
//!         list/get/suppress)
//!
//! Spec-Realism Gate compliance:
//!  - Sub-rule 1: no LiveXxxUnavailable / todo / unimplemented paths.
//!  - Sub-rule 2: real resource = Postgres via `postgres_backend_from_env`,
//!    matching the convention in `tests/kernel_postgres_event_ledger_tests.rs`.
//!    `#[ignore]`-gated; run with `cargo test -- --ignored`. When
//!    `POSTGRES_TEST_URL` is unset the test starts a bounded Docker Postgres
//!    fixture and cleans it up through a guard.
//!  - Sub-rule 3: a separate validator session signs off on the behaviour.

use std::{
    cell::RefCell,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use handshake_core::{
    memory::{
        CapsuleBuilder, CapsuleFlightRecorderEvent, CapsulePolicyTable, CapsuleRecord,
        CapsuleRecorder, FemsError, FemsFlightRecorder, FemsFlightRecorderError, FemsRetriever,
        GetCapsuleRequest, InjectionDecision, ListRecentCapsulesRequest, MemoryCapsuleIpcStore,
        ModelCallContext, PostgresKernelActionSubmitter, PostgresMemoryCapsuleStore, RetrievedItem,
        SuppressItemRequest, TaskType, MEMORY_CAPSULE_AGGREGATE_TYPE,
        MEMORY_CAPSULE_RECORD_ACTION_ID,
    },
    storage::{postgres::PostgresDatabase, Database, StorageResult},
};
use serde::Deserialize;
use sqlx::Connection;
use uuid::Uuid;

const QUERY: &str = "how do I add a new HBR rule applicability tag";
const ROLE_ID: &str = "KERNEL_BUILDER";
const SESSION_ID: &str = "KERNEL_BUILDER-MT-147-POSTGRES";
const FIXTURE_RELATIVE_PATH: &str = "tests/fixtures/memory_capsule_e2e/sample_fems_items.json";
const POSTGRES_READY_TIMEOUT: Duration = Duration::from_secs(45);

async fn postgres_backend_from_url(url: &str) -> StorageResult<Arc<dyn Database>> {
    let mut conn = sqlx::PgConnection::connect(url).await?;
    let schema = format!("memory_capsule_e2e_{}", Uuid::now_v7().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");

    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;
    Ok(db.into_arc())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts real Postgres; run explicitly with `cargo test -- --ignored`"]
async fn capsule_builder_injector_recorder_and_ipc_compose_over_real_postgres() {
    let fixture = PostgresFixture::start().expect("postgres fixture");
    let db = postgres_backend_from_url(fixture.url())
        .await
        .unwrap_or_else(|error| {
            panic!(
                "failed to init postgres backend: {error:?}\n{}",
                fixture.diagnostics()
            )
        });

    let fixture_items = load_fixture_items();
    let fems = TestFemsAdapter::new(fixture_items);
    let policy_table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &policy_table);

    // MT-143: build the capsule from the FEMS fixture adapter.
    let _built_capsule = builder
        .build(handshake_core::memory::BuildContext {
            task_type: TaskType::KernelBuilderMtImplementation,
            query: QUERY.to_string(),
            role_id: ROLE_ID.to_string(),
            session_id: SESSION_ID.to_string(),
            override_policy: None,
        })
        .expect("CapsuleBuilder must succeed against the MT-147 fixture");

    // MT-144: inject the capsule into the model-call context boundary. The
    // recording flight-recorder lets us observe the injection event.
    let flight_recorder = RecordingFemsFlightRecorder::default();
    let injector = handshake_core::memory::CapsuleInjector::new(&builder, &flight_recorder);
    let model_call_context = ModelCallContext::eligible(
        TaskType::KernelBuilderMtImplementation,
        QUERY,
        ROLE_ID,
        SESSION_ID,
    );
    let decision = injector
        .inject_for_call(&model_call_context)
        .expect("CapsuleInjector must produce an Inject decision over the fixture");
    let injected_capsule = match decision {
        InjectionDecision::Inject { capsule, .. } => capsule,
        InjectionDecision::Skip { reason } => {
            panic!("expected Inject decision, got Skip {reason:?}")
        }
    };

    // MT-145: record the capsule through the real Postgres-backed kernel
    // action catalog dispatcher.
    let submitter = PostgresKernelActionSubmitter::with_db(Arc::clone(&db));
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };
    let record = CapsuleRecord::from_capsule(&injected_capsule, Utc::now(), SESSION_ID, ROLE_ID);
    let _receipt = recorder.record(record.clone()).expect("recorder.record");

    // Confirm the ledger now carries the catalog-action event.
    let events = db
        .list_kernel_events_for_aggregate(
            MEMORY_CAPSULE_AGGREGATE_TYPE,
            &record.capsule_id.to_string(),
        )
        .await
        .expect("list ledger events for capsule aggregate");
    assert!(events.iter().any(|event| event
        .payload
        .get("catalog_action_id")
        .and_then(|v| v.as_str())
        == Some(MEMORY_CAPSULE_RECORD_ACTION_ID)));

    // MT-146: list/get/suppress over the durable Postgres-backed IPC store.
    let store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&db));
    store
        .save_capsule_record(record.clone())
        .expect("store.save_capsule_record");

    let service =
        handshake_core::memory::MemoryIpcService::new(&store, &submitter, &flight_recorder);
    let list = service
        .list_recent(ListRecentCapsulesRequest { limit: 50 })
        .expect("list_recent");
    assert!(list
        .capsules
        .iter()
        .any(|capsule| capsule.capsule_id == record.capsule_id));

    let fetched = service
        .get(GetCapsuleRequest {
            capsule_id: record.capsule_id,
        })
        .expect("get");
    assert_eq!(fetched.record.capsule_id, record.capsule_id);

    // Exercise suppression and confirm durability across a simulated restart.
    let included_item_id = record
        .audit_log
        .entries
        .iter()
        .find(|entry| entry.included)
        .map(|entry| entry.item_id.clone())
        .expect("fixture must produce at least one included audit entry");
    let _suppression = service
        .suppress_item(SuppressItemRequest {
            capsule_id: record.capsule_id,
            item_id: included_item_id.clone(),
            reason: "MT-147 postgres E2E suppression".to_string(),
            actor_id: ROLE_ID.to_string(),
            session_id: SESSION_ID.to_string(),
        })
        .expect("suppress_item");

    let restarted_store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&db));
    let restarted_service = handshake_core::memory::MemoryIpcService::new(
        &restarted_store,
        &submitter,
        &flight_recorder,
    );
    let after_restart = restarted_service
        .get(GetCapsuleRequest {
            capsule_id: record.capsule_id,
        })
        .expect("get after restart");
    let suppressed_entry = after_restart
        .record
        .audit_log
        .entries
        .iter()
        .find(|entry| entry.item_id == included_item_id)
        .expect("suppressed item must survive restart");
    assert!(!suppressed_entry.included);
    assert!(suppressed_entry.suppression_reason.is_some());
}

#[derive(Default)]
struct TestFemsAdapter {
    items: Vec<RetrievedItem>,
    calls: RefCell<Vec<(String, u32)>>,
}

impl TestFemsAdapter {
    fn new(items: Vec<RetrievedItem>) -> Self {
        Self {
            items,
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl FemsRetriever for TestFemsAdapter {
    fn retrieve(&self, query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        self.calls.borrow_mut().push((query.to_string(), top_k));
        Ok(self.items.clone())
    }
}

#[derive(Default)]
struct RecordingFemsFlightRecorder {
    events: RefCell<Vec<CapsuleFlightRecorderEvent>>,
}

impl FemsFlightRecorder for RecordingFemsFlightRecorder {
    fn record_event(
        &self,
        event: CapsuleFlightRecorderEvent,
    ) -> Result<(), FemsFlightRecorderError> {
        self.events.borrow_mut().push(event);
        Ok(())
    }
}

fn load_fixture_items() -> Vec<RetrievedItem> {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(Path::new(FIXTURE_RELATIVE_PATH));
    let raw = fs::read_to_string(&fixture_path).unwrap_or_else(|error| {
        panic!(
            "MT-147 fixture file is required at {}: {error}",
            fixture_path.display()
        )
    });
    let fixture: FixtureFile = serde_json::from_str(&raw).unwrap_or_else(|error| {
        panic!(
            "MT-147 fixture file must match strict JSON contract at {}: {error}",
            fixture_path.display()
        )
    });
    // Sanity-check the fixture is the same one the in-memory MT-147 e2e uses so
    // we are exercising the spec-required path, not an ad-hoc test fixture.
    assert_eq!(fixture.schema_version, "sample_fems_items.v1");
    assert_eq!(
        fixture.fixture_id,
        "mt-147-memory-capsule-e2e-sample-fems-items"
    );
    assert_eq!(fixture.wp_id, "WP-KERNEL-004");
    assert_eq!(fixture.mt_id, "MT-147");
    fixture.items
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    schema_version: String,
    fixture_id: String,
    wp_id: String,
    mt_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    intended_task_type: String,
    items: Vec<RetrievedItem>,
}

struct PostgresFixture {
    url: String,
    container_name: Option<String>,
}

impl PostgresFixture {
    fn start() -> Result<Self, Box<dyn Error>> {
        if let Ok(url) = std::env::var("POSTGRES_TEST_URL") {
            if !url.trim().is_empty() {
                return Ok(Self {
                    url,
                    container_name: None,
                });
            }
        }

        let suffix = Uuid::now_v7().to_string().replace('-', "");
        let container_name = format!("handshake-mt147-pg-{}", &suffix[..12]);
        let run = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-d",
                "--name",
                &container_name,
                "-e",
                "POSTGRES_USER=handshake",
                "-e",
                "POSTGRES_PASSWORD=handshake",
                "-e",
                "POSTGRES_DB=handshake_test",
                "-e",
                "POSTGRES_INITDB_ARGS=--nosync",
                "--tmpfs",
                "/var/lib/postgresql/data:rw,noexec,nosuid,size=256m",
                "-p",
                "127.0.0.1::5432",
                "postgres:16-alpine",
                "postgres",
                "-c",
                "fsync=off",
                "-c",
                "full_page_writes=off",
                "-c",
                "synchronous_commit=off",
            ])
            .output()?;
        assert_success(run, "docker run postgres:16-alpine");
        let mut guard = PostgresContainerGuard::new(container_name);
        let container_name = guard.name();

        let port_output = Command::new("docker")
            .args(["port", container_name, "5432/tcp"])
            .output()?;
        assert_success(port_output.clone(), "docker port postgres");
        let port_line = String::from_utf8(port_output.stdout)?;
        let port = port_line
            .trim()
            .rsplit(':')
            .next()
            .filter(|value| !value.is_empty())
            .ok_or("docker port output did not contain a mapped port")?;

        let deadline = Instant::now() + POSTGRES_READY_TIMEOUT;
        loop {
            let ready = Command::new("docker")
                .args([
                    "exec",
                    container_name,
                    "pg_isready",
                    "-U",
                    "handshake",
                    "-d",
                    "handshake_test",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if ready.success() {
                break;
            }
            if Instant::now() >= deadline {
                let logs = Command::new("docker")
                    .args(["logs", "--tail", "120", container_name])
                    .output()
                    .ok()
                    .map(output_text)
                    .unwrap_or_else(|| "docker logs unavailable".to_string());
                let _ = Command::new("docker")
                    .args(["stop", container_name])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                return Err(format!(
                    "timed out waiting {:?} for PostgreSQL test container {container_name}\n{logs}",
                    POSTGRES_READY_TIMEOUT
                )
                .into());
            }
            thread::sleep(Duration::from_millis(500));
        }

        Ok(Self {
            url: format!(
                "postgres://handshake:handshake@127.0.0.1:{port}/handshake_test?sslmode=disable"
            ),
            container_name: Some(guard.release()),
        })
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn diagnostics(&self) -> String {
        let Some(container_name) = &self.container_name else {
            return "external POSTGRES_TEST_URL supplied; no fixture container diagnostics available"
                .to_string();
        };

        let inspect = Command::new("docker")
            .args([
                "inspect",
                container_name,
                "--format",
                "Status={{.State.Status}} Health={{if .State.Health}}{{.State.Health.Status}}{{end}} Exit={{.State.ExitCode}} Error={{.State.Error}}",
            ])
            .output()
            .ok()
            .map(output_text)
            .unwrap_or_else(|| "docker inspect unavailable".to_string());
        let logs = Command::new("docker")
            .args(["logs", "--tail", "160", container_name])
            .output()
            .ok()
            .map(output_text)
            .unwrap_or_else(|| "docker logs unavailable".to_string());

        format!("container: {container_name}\ninspect:\n{inspect}\nlogs:\n{logs}")
    }
}

impl Drop for PostgresFixture {
    fn drop(&mut self) {
        if let Some(container_name) = &self.container_name {
            let _ = Command::new("docker")
                .args(["stop", container_name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

struct PostgresContainerGuard {
    container_name: Option<String>,
}

impl PostgresContainerGuard {
    fn new(container_name: String) -> Self {
        Self {
            container_name: Some(container_name),
        }
    }

    fn name(&self) -> &str {
        self.container_name
            .as_deref()
            .expect("container guard must hold a name")
    }

    fn release(&mut self) -> String {
        self.container_name
            .take()
            .expect("container guard must hold a name")
    }
}

impl Drop for PostgresContainerGuard {
    fn drop(&mut self) {
        if let Some(container_name) = &self.container_name {
            let _ = Command::new("docker")
                .args(["stop", container_name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

fn assert_success(output: Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn output_text(output: Output) -> String {
    format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
