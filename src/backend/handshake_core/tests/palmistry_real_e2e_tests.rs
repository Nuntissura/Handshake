//! Ignored real-process proof for Palmistry's backend-restart ownership path.
//!
//! This test intentionally launches the production `handshake_core` binary and
//! the real `handshake-native.exe`. The native process therefore owns the TCP
//! connection used by `/internal-diagnostics/palmistry/start`; the test never
//! calls Palmistry's private launch helpers or bypasses native-owner validation.
//!
//! Required environment:
//! - `CARGO_BIN_EXE_handshake_core` (normally supplied by Cargo with
//!   `--features app-runtime`);
//! - `HANDSHAKE_NATIVE_BIN`, canonical path to the real `handshake-native.exe`;
//! - `HANDSHAKE_PALMISTRY_BIN`, canonical path to the real `palmistry.exe`;
//! - `HANDSHAKE_PALMISTRY_SHA256`, lowercase SHA-256 pin for that exact binary;
//! - `POSTGRES_TEST_URL`, an operator-provided isolated PostgreSQL database;
//! - `HANDSHAKE_PALMISTRY_E2E_ISOLATED=1`, explicit acknowledgement that the
//!   database is disposable test authority.
//!
//! The backend has a production-fixed listener at `127.0.0.1:37501`, so this
//! target must run serially and fails closed when that port is already occupied.

#[cfg(not(target_os = "windows"))]
#[test]
#[ignore = "Windows-only real handshake-native/Palmistry/PostgreSQL lifecycle proof"]
fn palmistry_real_launch_restart_reattach_survivor_recovery_guarded_reclaim_writes_durable_stop() {
    panic!("ENVIRONMENT_BLOCKED: real Palmistry lifecycle proof requires Windows");
}

#[cfg(target_os = "windows")]
mod windows_e2e {
    use reqwest::Client;
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use sqlx::{postgres::PgPoolOptions, PgPool, Row};
    use std::{
        fs::{self, File},
        io,
        net::{SocketAddr, TcpStream},
        os::windows::process::CommandExt,
        path::{Path, PathBuf},
        process::{Child, Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    use tempfile::TempDir;
    use uuid::Uuid;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME, HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{
            GetProcessTimes, OpenProcess, TerminateProcess, WaitForSingleObject,
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
        },
    };

    const BACKEND_ADDR: &str = "127.0.0.1:37501";
    const BACKEND_BASE_URL: &str = "http://127.0.0.1:37501";
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const RING_HEADER_BYTES: usize = 128;
    const RING_SLOT_BYTES: usize = 128 * 1024;
    const SLOT_HEADER_BYTES: usize = 44;
    const OFFSET_ACTIVE_SLOT: usize = 12;
    const OFFSET_GENERATION: usize = 16;
    const PROCESS_WAIT_MS: u32 = 5_000;
    const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

    type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    #[derive(Clone)]
    struct TestConfig {
        backend_bin: PathBuf,
        native_bin: PathBuf,
        palmistry_bin: PathBuf,
        palmistry_sha256: String,
        postgres_url: String,
        diagnostics_dir: PathBuf,
        host_scope_id: String,
    }

    #[derive(Clone, Debug)]
    struct VerifierIdentity {
        session_id: Uuid,
        launch_nonce: Uuid,
        process_uuid: Uuid,
        parent_pid: u32,
        watcher_pid: u32,
        watcher_creation_time_100ns: u64,
    }

    struct ChildGuard {
        label: String,
        child: Child,
        stdout_path: PathBuf,
        stderr_path: PathBuf,
    }

    impl ChildGuard {
        fn id(&self) -> u32 {
            self.child.id()
        }

        fn ensure_running(&mut self) -> TestResult<()> {
            if let Some(status) = self.child.try_wait()? {
                return Err(test_error(format!(
                    "{} exited early with {status}; stdout={}; stderr={}",
                    self.label,
                    read_log(&self.stdout_path),
                    read_log(&self.stderr_path)
                )));
            }
            Ok(())
        }

        fn terminate(&mut self) -> TestResult<()> {
            if self.child.try_wait()?.is_some() {
                return Ok(());
            }
            self.child.kill()?;
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(10) {
                if self.child.try_wait()?.is_some() {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(test_error(format!(
                "{} did not exit after its exact child handle was terminated",
                self.label
            )))
        }
    }

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.terminate();
        }
    }

    struct ExactWatcherGuard {
        pid: u32,
        creation_time_100ns: u64,
        armed: bool,
    }

    impl ExactWatcherGuard {
        fn new(identity: &VerifierIdentity) -> Self {
            Self {
                pid: identity.watcher_pid,
                creation_time_100ns: identity.watcher_creation_time_100ns,
                armed: true,
            }
        }

        fn disarm_after_exit(&mut self) -> TestResult<()> {
            if exact_process_is_alive(self.pid, self.creation_time_100ns)? {
                return Err(test_error(format!(
                    "watcher {} with creation identity {} is still alive",
                    self.pid, self.creation_time_100ns
                )));
            }
            self.armed = false;
            Ok(())
        }
    }

    impl Drop for ExactWatcherGuard {
        fn drop(&mut self) {
            if self.armed {
                let _ = terminate_exact_process(self.pid, self.creation_time_100ns);
            }
        }
    }

    struct Harness {
        config: TestConfig,
        artifact_root: TempDir,
        backend: Option<ChildGuard>,
        natives: Vec<ChildGuard>,
        watchers: Vec<ExactWatcherGuard>,
        backend_generation: u32,
    }

    impl Drop for Harness {
        fn drop(&mut self) {
            for native in &mut self.natives {
                let _ = native.terminate();
            }
            if let Some(backend) = &mut self.backend {
                let _ = backend.terminate();
            }
            for watcher in &mut self.watchers {
                if watcher.armed {
                    let _ = terminate_exact_process(watcher.pid, watcher.creation_time_100ns);
                    watcher.armed = false;
                }
            }
            terminate_watchers_from_ready_files(&self.config.diagnostics_dir);
        }
    }

    impl Harness {
        fn new(config: TestConfig, artifact_root: TempDir) -> Self {
            Self {
                config,
                artifact_root,
                backend: None,
                natives: Vec::new(),
                watchers: Vec::new(),
                backend_generation: 0,
            }
        }

        fn start_backend(&mut self) -> TestResult<()> {
            if let Some(mut previous) = self.backend.take() {
                previous.terminate()?;
            }
            self.backend_generation += 1;
            let label = format!("handshake_core-{}", self.backend_generation);
            let stdout_path = self
                .artifact_root
                .path()
                .join(format!("{label}-stdout.log"));
            let stderr_path = self
                .artifact_root
                .path()
                .join(format!("{label}-stderr.log"));
            let stdout = File::create(&stdout_path)?;
            let stderr = File::create(&stderr_path)?;
            let mut command = Command::new(&self.config.backend_bin);
            command
                .env("DATABASE_URL", &self.config.postgres_url)
                .env("HANDSHAKE_STORAGE_MODE", "postgres_primary")
                .env("HANDSHAKE_CONTROL_PLANE_REQUIRES_POSTGRES", "1")
                .env("HANDSHAKE_MANAGED_PG_ENABLED", "0")
                .env("HANDSHAKE_HOST_SCOPE_ID", &self.config.host_scope_id)
                .env("HANDSHAKE_LLM_PROVIDER", "local_runtime")
                .env_remove("HANDSHAKE_LOCAL_MODEL_PATH")
                .env_remove("HANDSHAKE_LOCAL_MODEL_SHA256")
                .env("HANDSHAKE_DIAGNOSTICS_DIR", &self.config.diagnostics_dir)
                .env("HANDSHAKE_PALMISTRY_BIN", &self.config.palmistry_bin)
                .env("HANDSHAKE_PALMISTRY_SHA256", &self.config.palmistry_sha256)
                .env("HSK_HANDSHAKE_NATIVE_EXE", &self.config.native_bin)
                .env("RUST_LOG", "handshake_core=info")
                .stdin(Stdio::null())
                .stdout(Stdio::from(stdout))
                .stderr(Stdio::from(stderr))
                .creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
            let child = command.spawn()?;
            self.backend = Some(ChildGuard {
                label,
                child,
                stdout_path,
                stderr_path,
            });
            Ok(())
        }

        fn hard_kill_backend(&mut self) -> TestResult<()> {
            let mut backend = self
                .backend
                .take()
                .ok_or_else(|| test_error("backend child is not running"))?;
            backend.terminate()
        }

        fn start_native(&mut self, label: &str) -> TestResult<usize> {
            let stdout_path = self
                .artifact_root
                .path()
                .join(format!("{label}-stdout.log"));
            let stderr_path = self
                .artifact_root
                .path()
                .join(format!("{label}-stderr.log"));
            let stdout = File::create(&stdout_path)?;
            let stderr = File::create(&stderr_path)?;
            let mut command = Command::new(&self.config.native_bin);
            command
                .env("HANDSHAKE_DIAGNOSTICS_DIR", &self.config.diagnostics_dir)
                .env("RUST_LOG", "handshake_native=info")
                .stdin(Stdio::null())
                .stdout(Stdio::from(stdout))
                .stderr(Stdio::from(stderr))
                .creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
            let child = command.spawn()?;
            self.natives.push(ChildGuard {
                label: label.to_owned(),
                child,
                stdout_path,
                stderr_path,
            });
            Ok(self.natives.len() - 1)
        }

        fn native_pid(&self, index: usize) -> u32 {
            self.natives[index].id()
        }

        fn native_mut(&mut self, index: usize) -> &mut ChildGuard {
            &mut self.natives[index]
        }

        fn track_watcher(&mut self, identity: &VerifierIdentity) -> usize {
            self.watchers.push(ExactWatcherGuard::new(identity));
            self.watchers.len() - 1
        }

        fn backend_mut(&mut self) -> TestResult<&mut ChildGuard> {
            self.backend
                .as_mut()
                .ok_or_else(|| test_error("backend child is not running"))
        }
    }

    pub(crate) async fn run_real_e2e() -> TestResult<()> {
        ensure_backend_port_is_free()?;
        let artifact_root = tempfile::Builder::new()
            .prefix("palmistry-real-e2e-")
            .tempdir()?;
        let diagnostics_dir = artifact_root.path().join("diagnostics");
        fs::create_dir_all(&diagnostics_dir)?;
        let config = TestConfig::from_env(diagnostics_dir)?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&config.postgres_url)
            .await?;
        let mut harness = Harness::new(config, artifact_root);

        harness.start_backend()?;
        wait_for_backend_health(harness.backend_mut()?).await?;
        let preexisting_active_verifiers: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM palmistry_durable_verifier WHERE retired_at IS NULL",
        )
        .fetch_one(&pool)
        .await?;
        if preexisting_active_verifiers != 0 {
            return Err(test_error(format!(
                "POSTGRES_TEST_URL is not isolated: found {preexisting_active_verifiers} pre-existing active Palmistry verifier rows"
            )));
        }

        let source_native = harness.start_native("handshake-native-source")?;
        let source_parent_pid = harness.native_pid(source_native);
        let source = wait_for_active_verifier_for_parent(
            &pool,
            source_parent_pid,
            harness.native_mut(source_native),
            Duration::from_secs(90),
        )
        .await?;
        if source.parent_pid != source_parent_pid {
            return Err(test_error("source verifier parent PID mismatch"));
        }
        let source_watcher = harness.track_watcher(&source);
        let source_ring = harness
            .config
            .diagnostics_dir
            .join(format!("ring-{}.bin", source.session_id));
        wait_for_palmistry_started_events(
            &source_ring,
            1,
            harness.native_mut(source_native),
            Duration::from_secs(20),
        )
        .await?;

        harness.hard_kill_backend()?;
        if !exact_process_is_alive(source.watcher_pid, source.watcher_creation_time_100ns)? {
            return Err(test_error(
                "Palmistry watcher did not survive the first backend hard stop",
            ));
        }
        harness.native_mut(source_native).ensure_running()?;

        harness.start_backend()?;
        wait_for_backend_health(harness.backend_mut()?).await?;
        wait_for_palmistry_started_events(
            &source_ring,
            2,
            harness.native_mut(source_native),
            Duration::from_secs(45),
        )
        .await?;
        let reattached = active_verifier_for_parent(&pool, source_parent_pid)
            .await?
            .ok_or_else(|| {
                test_error("source verifier was not active after authenticated native reattach")
            })?;
        if reattached.process_uuid != source.process_uuid
            || reattached.launch_nonce != source.launch_nonce
            || reattached.watcher_pid != source.watcher_pid
            || reattached.watcher_creation_time_100ns != source.watcher_creation_time_100ns
        {
            return Err(test_error(
                "backend restart did not preserve the exact durable Palmistry identity",
            ));
        }

        harness.native_mut(source_native).terminate()?;
        let survivor = wait_for_source_survivor(
            &harness.config.diagnostics_dir.join("survivors"),
            source.session_id,
            Duration::from_secs(30),
        )
        .await?;
        let record_id = survivor
            .get("record_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| test_error("source survivor record_id is missing or invalid"))?;
        if survivor.get("kind").and_then(Value::as_str) != Some("unexpected_exit") {
            return Err(test_error(
                "source watcher did not write an unexpected_exit survivor",
            ));
        }

        let importer_native = harness.start_native("handshake-native-importer")?;
        let importer_parent_pid = harness.native_pid(importer_native);
        let importer = wait_for_active_verifier_for_parent(
            &pool,
            importer_parent_pid,
            harness.native_mut(importer_native),
            Duration::from_secs(90),
        )
        .await?;
        let importer_watcher = harness.track_watcher(&importer);

        wait_for_recovery_terminal_proof(
            &pool,
            &source,
            &harness
                .config
                .diagnostics_dir
                .join("recovered")
                .join(format!("{record_id}.ack")),
            Duration::from_secs(90),
        )
        .await?;
        harness.watchers[source_watcher].disarm_after_exit()?;

        let source_terminal = process_terminal_state(&pool, source.process_uuid)
            .await?
            .ok_or_else(|| test_error("source ProcessOwnershipLedger row is missing"))?;
        if source_terminal.0 != "reclaim" {
            return Err(test_error(format!(
                "source STOP reason was {:?}, not guarded Reclaim",
                source_terminal.0
            )));
        }
        if !source_terminal.1 {
            return Err(test_error("source STOP is not durable"));
        }
        if !verifier_is_retired(&pool, &source).await? {
            return Err(test_error("source verifier was not retired exactly"));
        }

        let importer_shutdown = harness
            .config
            .diagnostics_dir
            .join(format!("shutdown-{}.signal", importer.session_id));
        fs::write(&importer_shutdown, b"palmistry_e2e_cleanup\n")?;
        wait_for_durable_stop_and_retirement(&pool, &importer, Duration::from_secs(30)).await?;
        harness.watchers[importer_watcher].disarm_after_exit()?;
        harness.native_mut(importer_native).terminate()?;
        harness.hard_kill_backend()?;
        Ok(())
    }

    impl TestConfig {
        fn from_env(diagnostics_dir: PathBuf) -> TestResult<Self> {
            if std::env::var("HANDSHAKE_PALMISTRY_E2E_ISOLATED").as_deref() != Ok("1") {
                return Err(test_error(
                    "ENVIRONMENT_BLOCKED: set HANDSHAKE_PALMISTRY_E2E_ISOLATED=1 only for a disposable isolated POSTGRES_TEST_URL",
                ));
            }
            let backend_bin = option_env!("CARGO_BIN_EXE_handshake_core")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("CARGO_BIN_EXE_handshake_core").map(PathBuf::from)
                })
                .ok_or_else(|| {
                    test_error(
                        "ENVIRONMENT_BLOCKED: CARGO_BIN_EXE_handshake_core missing; use --features app-runtime",
                    )
                })?;
            let backend_bin = canonical_file(backend_bin, "CARGO_BIN_EXE_handshake_core")?;
            let native_bin =
                canonical_file_env("HANDSHAKE_NATIVE_BIN", Some("handshake-native.exe"))?;
            let palmistry_bin =
                canonical_file_env("HANDSHAKE_PALMISTRY_BIN", Some("palmistry.exe"))?;
            let palmistry_sha256 = std::env::var("HANDSHAKE_PALMISTRY_SHA256").map_err(|_| {
                test_error("ENVIRONMENT_BLOCKED: HANDSHAKE_PALMISTRY_SHA256 is required")
            })?;
            if palmistry_sha256.len() != 64
                || !palmistry_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(test_error(
                    "HANDSHAKE_PALMISTRY_SHA256 must be exactly 64 lowercase hex characters",
                ));
            }
            let actual_sha256 = sha256_file(&palmistry_bin)?;
            if actual_sha256 != palmistry_sha256 {
                return Err(test_error(format!(
                    "HANDSHAKE_PALMISTRY_SHA256 mismatch: expected {palmistry_sha256}, actual {actual_sha256}"
                )));
            }
            let postgres_url = std::env::var("POSTGRES_TEST_URL").map_err(|_| {
                test_error("ENVIRONMENT_BLOCKED: explicit isolated POSTGRES_TEST_URL is required")
            })?;
            if !postgres_url.starts_with("postgres://")
                && !postgres_url.starts_with("postgresql://")
            {
                return Err(test_error(
                    "POSTGRES_TEST_URL must be a PostgreSQL connection URL",
                ));
            }
            Ok(Self {
                backend_bin,
                native_bin,
                palmistry_bin,
                palmistry_sha256,
                postgres_url,
                diagnostics_dir,
                host_scope_id: format!("palmistry-real-e2e-{}", Uuid::now_v7()),
            })
        }
    }

    async fn wait_for_backend_health(backend: &mut ChildGuard) -> TestResult<()> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(2))
            .build()?;
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(120) {
            backend.ensure_running()?;
            if let Ok(response) = client
                .get(format!("{BACKEND_BASE_URL}/health"))
                .send()
                .await
            {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Err(test_error(format!(
            "backend did not become healthy; stdout={}; stderr={}",
            read_log(&backend.stdout_path),
            read_log(&backend.stderr_path)
        )))
    }

    async fn wait_for_active_verifier_for_parent(
        pool: &PgPool,
        parent_pid: u32,
        native: &mut ChildGuard,
        timeout: Duration,
    ) -> TestResult<VerifierIdentity> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            native.ensure_running()?;
            if let Some(identity) = active_verifier_for_parent(pool, parent_pid).await? {
                return Ok(identity);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(test_error(format!(
            "no active Palmistry verifier appeared for native PID {parent_pid}; stdout={}; stderr={}",
            read_log(&native.stdout_path),
            read_log(&native.stderr_path)
        )))
    }

    async fn active_verifier_for_parent(
        pool: &PgPool,
        parent_pid: u32,
    ) -> TestResult<Option<VerifierIdentity>> {
        let row = sqlx::query(
            "SELECT session_id, launch_nonce, process_uuid, parent_pid, watcher_pid, \
             watcher_creation_time_100ns FROM palmistry_durable_verifier \
             WHERE parent_pid = $1 AND retired_at IS NULL",
        )
        .bind(i64::from(parent_pid))
        .fetch_optional(pool)
        .await?;
        row.map(|row| -> TestResult<VerifierIdentity> {
            Ok(VerifierIdentity {
                session_id: row.try_get("session_id")?,
                launch_nonce: row.try_get("launch_nonce")?,
                process_uuid: row.try_get("process_uuid")?,
                parent_pid: u32::try_from(row.try_get::<i64, _>("parent_pid")?)?,
                watcher_pid: u32::try_from(row.try_get::<i64, _>("watcher_pid")?)?,
                watcher_creation_time_100ns: u64::try_from(
                    row.try_get::<i64, _>("watcher_creation_time_100ns")?,
                )?,
            })
        })
        .transpose()
    }

    async fn wait_for_palmistry_started_events(
        ring: &Path,
        expected: usize,
        native: &mut ChildGuard,
        timeout: Duration,
    ) -> TestResult<()> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            native.ensure_running()?;
            if read_ring_snapshot(ring)
                .ok()
                .map(|snapshot| palmistry_started_event_count(&snapshot) >= expected)
                .unwrap_or(false)
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(test_error(format!(
            "diagnostics ring did not expose {expected} authenticated Palmistry started events"
        )))
    }

    fn read_ring_snapshot(path: &Path) -> TestResult<Value> {
        let bytes = fs::read(path)?;
        let minimum = RING_HEADER_BYTES + (RING_SLOT_BYTES * 2);
        if bytes.len() < minimum || bytes.get(0..8) != Some(b"HSKIDG01") {
            return Err(test_error("invalid diagnostics ring"));
        }
        let active_slot = u32::from_le_bytes(
            bytes[OFFSET_ACTIVE_SLOT..OFFSET_ACTIVE_SLOT + 4]
                .try_into()
                .map_err(|_| test_error("invalid ring active slot"))?,
        ) as usize;
        if active_slot > 1 {
            return Err(test_error("invalid diagnostics ring active slot"));
        }
        let generation = u64::from_le_bytes(
            bytes[OFFSET_GENERATION..OFFSET_GENERATION + 8]
                .try_into()
                .map_err(|_| test_error("invalid ring generation"))?,
        );
        let offset = RING_HEADER_BYTES + (active_slot * RING_SLOT_BYTES);
        let slot_generation = u64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| test_error("invalid ring slot generation"))?,
        );
        let len = u32::from_le_bytes(
            bytes[offset + 8..offset + 12]
                .try_into()
                .map_err(|_| test_error("invalid ring payload length"))?,
        ) as usize;
        if generation == 0
            || slot_generation != generation
            || len > RING_SLOT_BYTES - SLOT_HEADER_BYTES
        {
            return Err(test_error("unstable diagnostics ring snapshot"));
        }
        let payload_start = offset + SLOT_HEADER_BYTES;
        let payload = &bytes[payload_start..payload_start + len];
        let expected_hash = &bytes[offset + 12..offset + SLOT_HEADER_BYTES];
        if Sha256::digest(payload).as_slice() != expected_hash {
            return Err(test_error("diagnostics ring payload hash mismatch"));
        }
        Ok(serde_json::from_slice(payload)?)
    }

    fn palmistry_started_event_count(snapshot: &Value) -> usize {
        snapshot
            .get("events")
            .and_then(Value::as_array)
            .map(|events| {
                events
                    .iter()
                    .filter(|event| {
                        event.get("mechanism").and_then(Value::as_str) == Some("palmistry")
                            && event.get("state").and_then(Value::as_str) == Some("started")
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    async fn wait_for_source_survivor(
        survivor_dir: &Path,
        source_session_id: Uuid,
        timeout: Duration,
    ) -> TestResult<Value> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if let Ok(entries) = fs::read_dir(survivor_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_survivor = path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .map(|value| value.starts_with("survivor-") && value.ends_with(".json"))
                        .unwrap_or(false);
                    if !is_survivor {
                        continue;
                    }
                    let Ok(bytes) = fs::read(&path) else {
                        continue;
                    };
                    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
                        continue;
                    };
                    if value
                        .get("session_id")
                        .and_then(Value::as_str)
                        .and_then(|value| Uuid::parse_str(value).ok())
                        == Some(source_session_id)
                    {
                        return Ok(value);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(test_error(
            "surviving Palmistry watcher did not write a source survivor",
        ))
    }

    async fn wait_for_recovery_terminal_proof(
        pool: &PgPool,
        source: &VerifierIdentity,
        recovered_ack: &Path,
        timeout: Duration,
    ) -> TestResult<()> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            let terminal = process_terminal_state(pool, source.process_uuid).await?;
            if recovered_ack.is_file()
                && terminal.as_ref() == Some(&("reclaim".to_owned(), true))
                && verifier_is_retired(pool, source).await?
                && !exact_process_is_alive(source.watcher_pid, source.watcher_creation_time_100ns)?
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(test_error(
            "recovery did not prove ACK + guarded Reclaim + durable STOP + exact verifier retirement",
        ))
    }

    async fn process_terminal_state(
        pool: &PgPool,
        process_uuid: Uuid,
    ) -> TestResult<Option<(String, bool)>> {
        let row = sqlx::query(
            "SELECT COALESCE(stop_reason, '') AS stop_reason, stopped_at IS NOT NULL AS stopped \
             FROM kernel_process_lifecycle WHERE process_uuid = $1",
        )
        .bind(process_uuid)
        .fetch_optional(pool)
        .await?;
        row.map(|row| -> TestResult<(String, bool)> {
            Ok((row.try_get("stop_reason")?, row.try_get("stopped")?))
        })
        .transpose()
    }

    async fn verifier_is_retired(pool: &PgPool, source: &VerifierIdentity) -> TestResult<bool> {
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT COALESCE(bool_and(retired_at IS NOT NULL), false) \
             FROM palmistry_durable_verifier \
             WHERE session_id = $1 AND launch_nonce = $2 AND process_uuid = $3",
        )
        .bind(source.session_id)
        .bind(source.launch_nonce)
        .bind(source.process_uuid)
        .fetch_one(pool)
        .await?)
    }

    async fn wait_for_durable_stop_and_retirement(
        pool: &PgPool,
        identity: &VerifierIdentity,
        timeout: Duration,
    ) -> TestResult<()> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if process_terminal_state(pool, identity.process_uuid)
                .await?
                .map(|(_, stopped)| stopped)
                .unwrap_or(false)
                && verifier_is_retired(pool, identity).await?
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(test_error(format!(
            "process {} did not write durable STOP and retire its exact verifier",
            identity.process_uuid
        )))
    }

    fn ensure_backend_port_is_free() -> TestResult<()> {
        let address: SocketAddr = BACKEND_ADDR.parse()?;
        if TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok() {
            return Err(test_error(format!(
                "ENVIRONMENT_BLOCKED: backend port {BACKEND_ADDR} is already occupied"
            )));
        }
        Ok(())
    }

    fn canonical_file_env(name: &str, expected_name: Option<&str>) -> TestResult<PathBuf> {
        let value = std::env::var_os(name)
            .ok_or_else(|| test_error(format!("ENVIRONMENT_BLOCKED: {name} is required")))?;
        let path = canonical_file(PathBuf::from(value), name)?;
        if let Some(expected_name) = expected_name {
            let actual_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if !actual_name.eq_ignore_ascii_case(expected_name) {
                return Err(test_error(format!(
                    "{name} must resolve to a file named {expected_name}"
                )));
            }
        }
        Ok(path)
    }

    fn canonical_file(path: PathBuf, label: &str) -> TestResult<PathBuf> {
        let canonical = fs::canonicalize(path).map_err(|error| {
            test_error(format!(
                "ENVIRONMENT_BLOCKED: cannot canonicalize {label}: {error}"
            ))
        })?;
        if !canonical.is_file() {
            return Err(test_error(format!(
                "ENVIRONMENT_BLOCKED: {label} is not a file"
            )));
        }
        Ok(canonical)
    }

    fn sha256_file(path: &Path) -> TestResult<String> {
        Ok(hex::encode(Sha256::digest(fs::read(path)?)))
    }

    fn read_log(path: &Path) -> String {
        fs::read(path)
            .map(|value| {
                let start = value.len().saturating_sub(8 * 1024);
                String::from_utf8_lossy(&value[start..]).into_owned()
            })
            .unwrap_or_else(|error| format!("<unreadable {}: {error}>", path.display()))
    }

    fn terminate_watchers_from_ready_files(diagnostics_dir: &Path) {
        let Ok(entries) = fs::read_dir(diagnostics_dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_ready = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.starts_with("ready-") && value.ends_with(".json"))
                .unwrap_or(false);
            if !is_ready {
                continue;
            }
            let Ok(bytes) = fs::read(path) else {
                continue;
            };
            let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
                continue;
            };
            let Some(pid) = value
                .get("watcher_pid")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
            else {
                continue;
            };
            let Some(creation_time_100ns) = value
                .get("watcher_creation_time_100ns")
                .and_then(Value::as_u64)
            else {
                continue;
            };
            let _ = terminate_exact_process(pid, creation_time_100ns);
        }
    }

    fn exact_process_is_alive(pid: u32, creation_time_100ns: u64) -> TestResult<bool> {
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE_ACCESS,
                0,
                pid,
            )
        };
        if handle.is_null() {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(87) {
                Ok(false)
            } else {
                Err(Box::new(error))
            };
        }
        let result = (|| -> TestResult<bool> {
            if process_creation_time_100ns(handle)? != creation_time_100ns {
                return Ok(false);
            }
            match unsafe { WaitForSingleObject(handle, 0) } {
                WAIT_OBJECT_0 => Ok(false),
                WAIT_TIMEOUT => Ok(true),
                WAIT_FAILED => Err(Box::new(io::Error::last_os_error())),
                value => Err(test_error(format!(
                    "unexpected WaitForSingleObject result {value} for PID {pid}"
                ))),
            }
        })();
        unsafe {
            CloseHandle(handle);
        }
        result
    }

    fn terminate_exact_process(pid: u32, creation_time_100ns: u64) -> TestResult<()> {
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | SYNCHRONIZE_ACCESS,
                0,
                pid,
            )
        };
        if handle.is_null() {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(87) {
                Ok(())
            } else {
                Err(Box::new(error))
            };
        }
        let result = (|| -> TestResult<()> {
            if process_creation_time_100ns(handle)? != creation_time_100ns {
                return Ok(());
            }
            match unsafe { WaitForSingleObject(handle, 0) } {
                WAIT_OBJECT_0 => return Ok(()),
                WAIT_TIMEOUT => {}
                WAIT_FAILED => return Err(Box::new(io::Error::last_os_error())),
                value => {
                    return Err(test_error(format!(
                        "unexpected WaitForSingleObject result {value} for PID {pid}"
                    )))
                }
            }
            if unsafe { TerminateProcess(handle, 0xE000_0001) } == 0 {
                return Err(Box::new(io::Error::last_os_error())
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            match unsafe { WaitForSingleObject(handle, PROCESS_WAIT_MS) } {
                WAIT_OBJECT_0 => Ok(()),
                WAIT_TIMEOUT => Err(test_error(format!(
                    "exact watcher PID {pid} did not terminate within {PROCESS_WAIT_MS} ms"
                ))),
                WAIT_FAILED => Err(Box::new(io::Error::last_os_error())),
                value => Err(test_error(format!(
                    "unexpected WaitForSingleObject result {value} for PID {pid}"
                ))),
            }
        })();
        unsafe {
            CloseHandle(handle);
        }
        result
    }

    fn process_creation_time_100ns(handle: HANDLE) -> TestResult<u64> {
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        if unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } == 0
        {
            return Err(Box::new(io::Error::last_os_error()));
        }
        Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
    }

    fn test_error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(io::Error::other(message.into()))
    }
}

#[cfg(target_os = "windows")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires real Windows handshake_core, handshake-native.exe, pinned palmistry.exe, and isolated PostgreSQL"]
async fn palmistry_real_launch_restart_reattach_survivor_recovery_guarded_reclaim_writes_durable_stop(
) {
    if let Err(error) = windows_e2e::run_real_e2e().await {
        panic!("{error}");
    }
}
