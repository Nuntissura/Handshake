use std::{
    error::Error,
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, Row};
use tempfile::TempDir;
use uuid::Uuid;

use handshake_core::{
    hbr::{
        handoff_gate::{
            HandoffEventLedger, HandoffEventLedgerError, HandoffGate, HandoffRule,
            HandoffTransition, HbrAcceptanceMatrix, HbrMatrixRow, HbrPacket,
        },
        registry::HbrRegistry,
        violation::{
            EvaluationPoint, HbrViolation, HbrViolationRole, ViolationClass, ViolationSink,
        },
    },
    kernel::{KernelEvent, KernelEventType, NewKernelEvent},
    process_ledger::{
        LedgerOverflowEvent, PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerError,
        ProcessLedgerOverflowSink, ProcessLedgerWriter, ProcessStart, ProcessStop,
    },
};

const WP_ID: &str =
    "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";
const SESSION_ID: &str = "KERNEL_BUILDER-20260518-012310";
const POSTGRES_READY_TIMEOUT: Duration = Duration::from_secs(300);

#[tokio::test]
async fn hbr_e2e_smoke_test() -> Result<(), Box<dyn Error>> {
    let repo_root = repo_root();
    let registry_path = repo_root.join(".GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json");
    let registry = HbrRegistry::load_from_path(&registry_path)?;
    assert_eq!(registry.version, "1.3.0");

    let registry_json: Value = serde_json::from_str(&fs::read_to_string(&registry_path)?)?;
    assert_eq!(
        registry_json["enforcement"]["implementation_status"],
        "ACTIVE"
    );
    assert!(!registry_json["enforcement"]["implementation_owner"]
        .as_str()
        .unwrap_or_default()
        .contains("Until that wiring lands"));

    assert_success(
        run_node(
            &repo_root,
            ".GOV/roles_shared/scripts/hbr-registry-loader.mjs",
            ["--validate"],
        )?,
        "hbr-registry-loader --validate",
    );

    let packet_dir = TempDir::new()?;
    let packet_path = packet_dir.path().join("packet.json");
    fs::write(
        &packet_path,
        serde_json::to_string_pretty(&fixture_packet())? + "\n",
    )?;
    assert_success(
        run_node_os(
            &repo_root,
            ".GOV/roles_shared/scripts/hbr-matrix-hydrate.mjs",
            [
                OsString::from("--packet"),
                packet_path.as_os_str().to_os_string(),
                OsString::from("--added-at-utc"),
                OsString::from("2026-05-18T00:00:00.000Z"),
            ],
        )?,
        "hbr-matrix-hydrate",
    );

    let hydrated: Value = serde_json::from_str(&fs::read_to_string(&packet_path)?)?;
    let hbr_rows = hydrated["acceptance_matrix"]["hbr"]
        .as_array()
        .expect("hydrated acceptance_matrix.hbr rows");
    assert!(
        hbr_rows.iter().any(|row| row["hbr_id"] == "HBR-INT-001"),
        "hydrator must emit HBR-INT-001 for observable product behavior"
    );

    let mut proved_packet = hydrated.clone();
    prove_all_hbr_rows(&mut proved_packet);
    let proved_path = packet_dir.path().join("packet-proved.json");
    fs::write(
        &proved_path,
        serde_json::to_string_pretty(&proved_packet)? + "\n",
    )?;
    assert_success(
        run_node_os(
            &repo_root,
            ".GOV/roles_shared/checks/hbr-matrix-check.mjs",
            [
                OsString::from("--packet"),
                proved_path.as_os_str().to_os_string(),
            ],
        )?,
        "hbr-matrix-check proved packet",
    );

    let mut failing_packet = proved_packet.clone();
    set_hbr_row_pending(&mut failing_packet, "HBR-INT-001");
    let failing_path = packet_dir.path().join("packet-failing.json");
    fs::write(
        &failing_path,
        serde_json::to_string_pretty(&failing_packet)? + "\n",
    )?;
    let matrix_failure = run_node_os(
        &repo_root,
        ".GOV/roles_shared/checks/hbr-matrix-check.mjs",
        [
            OsString::from("--packet"),
            failing_path.as_os_str().to_os_string(),
        ],
    )?;
    assert_eq!(
        matrix_failure.status.code(),
        Some(2),
        "hbr-matrix-check failure packet should exit 2\npacket:\n{}\nstdout:\n{}\nstderr:\n{}",
        serde_json::to_string_pretty(&failing_packet)?,
        String::from_utf8_lossy(&matrix_failure.stdout),
        String::from_utf8_lossy(&matrix_failure.stderr)
    );
    assert!(
        String::from_utf8_lossy(&matrix_failure.stderr).contains("HBR-INT-001"),
        "matrix failure should name HBR-INT-001"
    );

    assert_gov_check_umbrella_reflects_hbr_matrix_failure(&repo_root, &failing_packet)?;

    let handoff_ledger = InMemoryHandoffLedger::default();
    let handoff_block = HandoffGate::new(
        handoff_ledger.clone(),
        vec![HandoffRule::new(
            "HBR-INT-001",
            "test_run_with_ledger_replay",
        )],
    )
    .evaluate(
        &HbrPacket {
            wp_id: WP_ID.to_string(),
            acceptance_matrix: HbrAcceptanceMatrix {
                hbr: vec![HbrMatrixRow {
                    hbr_id: "HBR-INT-001".to_string(),
                    status: "PENDING".to_string(),
                    evidence_pointer: None,
                    validator_verdict: None,
                }],
                hbr_not_applicable: Vec::new(),
            },
        },
        HandoffTransition::CoderToWpValidator,
    )
    .await
    .expect_err("PENDING HBR-INT-001 must block handoff");
    assert_eq!(handoff_block.failing_rules[0].hbr_id, "HBR-INT-001");
    let handoff_events = handoff_ledger.events();
    assert_eq!(handoff_events.len(), 1);
    assert_eq!(
        handoff_events[0].event_type,
        KernelEventType::HbrHandoffGate
    );
    assert_eq!(handoff_events[0].payload["verdict"]["kind"], "Block");

    let violation_sink = InMemoryViolationSink::default();
    let violation = HbrViolation::new(
        "HBR-INT-001",
        WP_ID,
        Some("MT-009"),
        HbrViolationRole::KernelBuilder,
        EvaluationPoint::Build,
        Some("test://hbr-e2e-smoke/matrix-failure"),
        ViolationClass::MissingEvidence,
        Some(SESSION_ID),
        Some("MT-009 deliberate failure-path proof"),
    );
    violation.emit(&violation_sink)?;
    let canonical_violation = violation_sink.single_row();
    let normalized = run_node_with_stdin(
        &repo_root,
        ".GOV/roles_shared/scripts/hbr-violation-emit.mjs",
        ["--normalize-stdin"],
        canonical_violation.as_bytes(),
    )?;
    assert_success(normalized.clone(), "hbr-violation-emit normalize");
    assert_eq!(String::from_utf8(normalized.stdout)?, canonical_violation);

    let postgres = PostgresFixture::start()?;
    let pool = match connect_postgres_with_retry(postgres.url()).await {
        Ok(pool) => pool,
        Err(error) => {
            return Err(format!(
                "{error}\nPostgreSQL fixture diagnostics:\n{}",
                postgres.diagnostics()
            )
            .into());
        }
    };
    let store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    store.apply_migration().await?;
    let (writer, drain) =
        ProcessLedgerWriter::new_manual(8, Arc::new(InMemoryOverflowSink::default()))?;
    let start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "KERNEL_BUILDER",
        Some(WP_ID.to_string()),
    )
    .with_parent_session_id("SR-HBR-E2E-SMOKE")
    .with_sandbox_adapter_id("sandbox-adapter-hbr-e2e")
    .with_work_profile_id("work-profile-hbr-e2e");
    let stop = ProcessStop::from_start(&start, Some(0));
    writer.append_start(start.clone())?;
    writer.append_stop(stop)?;
    drain.drain_available_to(store.clone()).await?;

    let row = sqlx::query(
        r#"
        SELECT engine_kind, owner_wp, stopped_at IS NOT NULL AS has_stop
        FROM kernel_process_lifecycle
        WHERE process_uuid = $1::uuid
        "#,
    )
    .bind(start.process_uuid.to_string())
    .fetch_one(&pool)
    .await?;
    let engine_kind: String = row.get("engine_kind");
    let owner_wp: Option<String> = row.get("owner_wp");
    let has_stop: bool = row.get("has_stop");
    assert_eq!(engine_kind, ProcessEngineKind::HelperSubprocess.as_str());
    assert_eq!(owner_wp.as_deref(), Some(WP_ID));
    assert!(has_stop);

    Ok(())
}

fn repo_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join(".GOV").exists() {
            return current;
        }
        assert!(current.pop(), "repo root with .GOV not found");
    }
}

fn fixture_packet() -> Value {
    json!({
        "wp_id": WP_ID,
        "scope": {
            "allowed_paths": [
                "src/backend/handshake_core/src/process_ledger/writer.rs",
                "src/backend/handshake_core/src/process_ledger/reclaim.rs",
                "src/backend/handshake_core/src/hbr/handoff_gate.rs",
                "src/backend/handshake_core/tests/hbr_e2e_smoke_test.rs"
            ]
        },
        "hbr": {
            "tags_declared": [
                "observable_behavior",
                "process_lifecycle",
                "automation_surface",
                "manual_diff",
                "self_consistency"
            ],
            "not_applicable_overrides": []
        },
        "acceptance_matrix": {
            "schema_version": 1,
            "hbr": [],
            "hbr_not_applicable": []
        }
    })
}

fn prove_all_hbr_rows(packet: &mut Value) {
    let rows = packet["acceptance_matrix"]["hbr"]
        .as_array_mut()
        .expect("hbr rows");
    for row in rows {
        let hbr_id = row["hbr_id"].as_str().unwrap_or("HBR-UNKNOWN").to_string();
        row["status"] = json!("PROVED");
        row["evidence_pointer"] = json!(format!("test://hbr-e2e-smoke/{hbr_id}"));
        row["validator_verdict"] = json!("PROVED");
    }
}

fn set_hbr_row_pending(packet: &mut Value, hbr_id: &str) {
    let rows = packet["acceptance_matrix"]["hbr"]
        .as_array_mut()
        .expect("hbr rows");
    let row = rows
        .iter_mut()
        .find(|row| row["hbr_id"] == hbr_id)
        .unwrap_or_else(|| panic!("missing hydrated row {hbr_id}"));
    row["status"] = json!("PENDING");
    row["evidence_pointer"] = Value::Null;
    row["validator_verdict"] = Value::Null;
}

fn assert_gov_check_umbrella_reflects_hbr_matrix_failure(
    repo_root: &Path,
    failing_packet: &Value,
) -> Result<(), Box<dyn Error>> {
    let root = TempDir::new()?;
    let packet_dir = root.path().join(".GOV/task_packets/WP-HBR-E2E-FAIL");
    fs::create_dir_all(&packet_dir)?;
    fs::write(
        packet_dir.join("packet.json"),
        serde_json::to_string_pretty(failing_packet)? + "\n",
    )?;

    let output = Command::new("node")
        .arg(repo_root.join(".GOV/roles_shared/checks/gov-check.mjs"))
        .arg("--json")
        .current_dir(root.path())
        .env("HANDSHAKE_ACTIVE_REPO_ROOT", root.path())
        .env("HANDSHAKE_GOV_ROOT", root.path().join(".GOV"))
        .env("HANDSHAKE_GOV_CHECK_TEST_MODE", "1")
        .env("HANDSHAKE_GOV_CHECK_ONLY", "hbr-matrix-check")
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hbr-matrix-check") && stdout.contains("\"verdict\": \"FAIL\""),
        "gov-check JSON output should identify hbr-matrix-check failure: {stdout}"
    );
    Ok(())
}

fn run_node<const N: usize>(
    repo_root: &Path,
    script: &str,
    args: [&str; N],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new("node");
    command
        .arg(repo_root.join(script))
        .args(args)
        .current_dir(repo_root);
    Ok(command.output()?)
}

fn run_node_os<const N: usize>(
    repo_root: &Path,
    script: &str,
    args: [OsString; N],
) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new("node");
    command
        .arg(repo_root.join(script))
        .args(args)
        .current_dir(repo_root);
    Ok(command.output()?)
}

fn run_node_with_stdin<const N: usize>(
    repo_root: &Path,
    script: &str,
    args: [&str; N],
    stdin: &[u8],
) -> Result<Output, Box<dyn Error>> {
    let mut child = Command::new("node")
        .arg(repo_root.join(script))
        .args(args)
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.as_mut().expect("stdin").write_all(stdin)?;
    Ok(child.wait_with_output()?)
}

fn assert_success(output: Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn connect_postgres_with_retry(url: &str) -> Result<sqlx::PgPool, Box<dyn Error>> {
    let deadline = Instant::now() + POSTGRES_READY_TIMEOUT;
    loop {
        match PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(error) => {
                if Instant::now() >= deadline {
                    return Err(format!(
                        "timed out waiting {:?} for host connection to PostgreSQL test fixture: {error}",
                        POSTGRES_READY_TIMEOUT
                    )
                    .into());
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

#[derive(Clone, Default)]
struct InMemoryHandoffLedger {
    events: Arc<Mutex<Vec<NewKernelEvent>>>,
}

#[async_trait]
impl HandoffEventLedger for InMemoryHandoffLedger {
    async fn append_handoff_event(
        &self,
        event: NewKernelEvent,
    ) -> Result<KernelEvent, HandoffEventLedgerError> {
        self.events.lock().expect("events lock").push(event.clone());
        Ok(KernelEvent::from_new(event))
    }
}

impl InMemoryHandoffLedger {
    fn events(&self) -> Vec<NewKernelEvent> {
        self.events.lock().expect("events lock").clone()
    }
}

#[derive(Default)]
struct InMemoryViolationSink {
    rows: Mutex<Vec<String>>,
}

impl InMemoryViolationSink {
    fn single_row(&self) -> String {
        let rows = self.rows.lock().expect("violation rows lock");
        assert_eq!(rows.len(), 1);
        rows[0].clone()
    }
}

impl ViolationSink for InMemoryViolationSink {
    fn write_violation(&self, canonical_jsonl: &str) -> Result<(), std::io::Error> {
        self.rows
            .lock()
            .expect("violation rows lock")
            .push(canonical_jsonl.to_string());
        Ok(())
    }
}

#[derive(Clone, Default)]
struct InMemoryOverflowSink {
    events: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
}

impl ProcessLedgerOverflowSink for InMemoryOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.events.lock().expect("overflow lock").push(event);
        Ok(())
    }
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
        let container_name = format!("handshake-mt009-pg-{}", &suffix[..12]);
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
                    .map(|output| {
                        format!(
                            "stdout:\n{}\nstderr:\n{}",
                            String::from_utf8_lossy(&output.stdout),
                            String::from_utf8_lossy(&output.stderr)
                        )
                    })
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

fn output_text(output: Output) -> String {
    format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
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
