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
//! Each schema also owns a durable lease row with the owner machine, PID, and
//! process-birth identity plus a session-level PostgreSQL advisory lock. A
//! killed process leaves the row but releases the lock. Startup reclaims only
//! when the lease is local to this machine and the exact recorded process is
//! positively dead or its PID has been reused; every unverifiable case defers.
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
use serde::{Deserialize, Serialize};
use sqlx::Connection;
use std::io::ErrorKind;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{watch, Mutex, OnceCell};
use tokio::task::JoinHandle;
use uuid::Uuid;

static MANAGED_POSTGRES: OnceCell<Option<ManagedPostgres>> = OnceCell::const_new();
static SCHEMA_SETUP_LOCK: Mutex<()> = Mutex::const_new(());
const CLEANUP_PHASE_TIMEOUT: Duration = Duration::from_secs(3);
const RECLAIM_DROP_TIMEOUT: Duration = Duration::from_secs(120);
const SCHEMA_SETUP_LOCK_TIMEOUT: Duration = Duration::from_secs(300);
const LEASE_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
const LEASE_RECONNECT_INTERVAL: Duration = Duration::from_secs(1);
const LEASE_STALE_SECONDS: i64 = 10;
pub const PANIC_CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);
static LOCAL_MACHINE_ID: OnceLock<Result<Uuid, String>> = OnceLock::new();

const LEASE_TABLE_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS public.handshake_knowledge_test_schema_leases_v2 (
        schema_name text PRIMARY KEY,
        owner_id uuid NOT NULL,
        owner_application_name text NOT NULL,
        owner_machine_id uuid NOT NULL,
        owner_pid bigint NOT NULL CHECK (owner_pid > 0 AND owner_pid <= 4294967295),
        owner_process_birth jsonb NOT NULL,
        created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
        heartbeat_at timestamptz NOT NULL DEFAULT clock_timestamp(),
        server_started_at timestamptz NOT NULL DEFAULT pg_postmaster_start_time(),
        CONSTRAINT handshake_knowledge_test_schema_name_v2
            CHECK (schema_name ~ '^knowledge_test_[0-9a-f]{32}$')
    )
"#;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ProcessBirthIdentity {
    Windows {
        creation_time_100ns: u64,
    },
    Linux {
        boot_id: String,
        start_time_ticks: u64,
    },
    MacOs {
        start_time_seconds: u64,
        start_time_microseconds: u64,
    },
}

enum ProcessState {
    Alive(ProcessBirthIdentity),
    Dead,
    Unverifiable,
}

#[derive(Debug, Clone)]
pub struct SchemaOwnerIdentity {
    pub machine_id: Uuid,
    pub pid: i64,
    pub process_birth: serde_json::Value,
}

fn machine_anchor_uuid(raw: &str) -> Result<Uuid, String> {
    let value = raw.trim();
    let machine_id = Uuid::parse_str(value)
        .map_err(|error| format!("host machine identity is not a UUID: {error}"))?;
    if machine_id.is_nil() {
        return Err("host machine identity must not be the nil UUID".to_string());
    }
    Ok(machine_id)
}

#[cfg(windows)]
fn read_host_machine_anchor() -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = Command::new("reg.exe")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
            "/reg:64",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("query Windows MachineGuid: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "query Windows MachineGuid exited with {}",
            output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("decode Windows MachineGuid output: {error}"))?;
    stdout
        .lines()
        .find(|line| line.contains("MachineGuid"))
        .and_then(|line| line.split_whitespace().last())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "Windows MachineGuid was absent from reg.exe output".to_string())
}

#[cfg(target_os = "linux")]
fn read_host_machine_anchor() -> Result<String, String> {
    std::fs::read_to_string("/etc/machine-id")
        .map_err(|error| format!("read Linux /etc/machine-id: {error}"))
}

#[cfg(target_os = "macos")]
fn read_host_machine_anchor() -> Result<String, String> {
    use std::process::{Command, Stdio};

    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| format!("query macOS IOPlatformUUID: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "query macOS IOPlatformUUID exited with {}",
            output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("decode macOS IOPlatformUUID output: {error}"))?;
    let line = stdout
        .lines()
        .find(|line| line.contains("IOPlatformUUID"))
        .ok_or_else(|| "macOS IOPlatformUUID was absent from ioreg output".to_string())?;
    line.split_once('=')
        .map(|(_, value)| value.trim().trim_matches('"').to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "macOS IOPlatformUUID value was empty".to_string())
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn read_host_machine_anchor() -> Result<String, String> {
    Err("host machine identity is unsupported on this platform".to_string())
}

fn local_machine_id() -> Result<Uuid, String> {
    LOCAL_MACHINE_ID
        .get_or_init(|| read_host_machine_anchor().and_then(|raw| machine_anchor_uuid(&raw)))
        .as_ref()
        .copied()
        .map_err(Clone::clone)
}

#[cfg(windows)]
fn inspect_process(pid: u32) -> ProcessState {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    const SYNCHRONIZE_RIGHT: u32 = 0x0010_0000;
    if pid == 0 {
        return ProcessState::Unverifiable;
    }
    let handle = unsafe {
        OpenProcess(
            SYNCHRONIZE_RIGHT | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return if std::io::Error::last_os_error().raw_os_error() == Some(87) {
            ProcessState::Dead
        } else {
            ProcessState::Unverifiable
        };
    }
    let wait = unsafe { WaitForSingleObject(handle, 0) };
    if wait == 0 {
        unsafe {
            let _ = CloseHandle(handle);
        }
        return ProcessState::Dead;
    }
    if wait != WAIT_TIMEOUT {
        unsafe {
            let _ = CloseHandle(handle);
        }
        return ProcessState::Unverifiable;
    }
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let queried =
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } != 0;
    unsafe {
        let _ = CloseHandle(handle);
    }
    if !queried {
        return ProcessState::Unverifiable;
    }
    ProcessState::Alive(ProcessBirthIdentity::Windows {
        creation_time_100ns: (u64::from(creation.dwHighDateTime) << 32)
            | u64::from(creation.dwLowDateTime),
    })
}

#[cfg(target_os = "linux")]
fn inspect_process(pid: u32) -> ProcessState {
    if pid == 0 {
        return ProcessState::Unverifiable;
    }
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => stat,
        Err(error) if error.kind() == ErrorKind::NotFound => return ProcessState::Dead,
        Err(_) => return ProcessState::Unverifiable,
    };
    let boot_id = match std::fs::read_to_string("/proc/sys/kernel/random/boot_id") {
        Ok(boot_id) => boot_id.trim().to_owned(),
        Err(_) => return ProcessState::Unverifiable,
    };
    if boot_id.is_empty() {
        return ProcessState::Unverifiable;
    }
    let Some((_, tail)) = stat.rsplit_once(") ") else {
        return ProcessState::Unverifiable;
    };
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let Some(state) = fields.first().and_then(|field| field.as_bytes().first()) else {
        return ProcessState::Unverifiable;
    };
    if matches!(*state, b'Z' | b'X' | b'x') {
        return ProcessState::Dead;
    }
    let Some(start_time_ticks) = fields.get(19).and_then(|field| field.parse().ok()) else {
        return ProcessState::Unverifiable;
    };
    ProcessState::Alive(ProcessBirthIdentity::Linux {
        boot_id,
        start_time_ticks,
    })
}

#[cfg(target_os = "macos")]
fn inspect_process(pid: u32) -> ProcessState {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct ProcBsdInfo {
        pbi_flags: u32,
        pbi_status: u32,
        pbi_xstatus: u32,
        pbi_pid: u32,
        pbi_ppid: u32,
        pbi_uid: u32,
        pbi_gid: u32,
        pbi_ruid: u32,
        pbi_rgid: u32,
        pbi_svuid: u32,
        pbi_svgid: u32,
        pbi_reserved: u32,
        pbi_comm: [u8; 16],
        pbi_name: [u8; 32],
        pbi_nfiles: u32,
        pbi_pgid: u32,
        pbi_pjobc: u32,
        e_tdev: u32,
        e_tpgid: u32,
        pbi_nice: i32,
        pbi_start_tvsec: u64,
        pbi_start_tvusec: u64,
    }

    #[link(name = "proc")]
    extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut c_void,
            buffer_size: i32,
        ) -> i32;
    }

    const PROC_PIDTBSDINFO: i32 = 3;
    const SZOMB: u32 = 5;
    const PROC_FLAG_INEXIT: u32 = 4;
    if pid == 0 {
        return ProcessState::Unverifiable;
    }
    let mut info = ProcBsdInfo::default();
    let expected_size = std::mem::size_of::<ProcBsdInfo>();
    let Ok(pid) = i32::try_from(pid) else {
        return ProcessState::Unverifiable;
    };
    let Ok(buffer_size) = i32::try_from(expected_size) else {
        return ProcessState::Unverifiable;
    };
    let queried = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDTBSDINFO,
            0,
            std::ptr::from_mut(&mut info).cast::<c_void>(),
            buffer_size,
        )
    };
    if queried <= 0 {
        return if std::io::Error::last_os_error().raw_os_error() == Some(3) {
            ProcessState::Dead
        } else {
            ProcessState::Unverifiable
        };
    }
    if queried != buffer_size || info.pbi_pid != pid as u32 {
        return ProcessState::Unverifiable;
    }
    if info.pbi_status == SZOMB || info.pbi_flags & PROC_FLAG_INEXIT != 0 {
        return ProcessState::Dead;
    }
    if info.pbi_start_tvsec == 0 || info.pbi_start_tvusec >= 1_000_000 {
        return ProcessState::Unverifiable;
    }
    ProcessState::Alive(ProcessBirthIdentity::MacOs {
        start_time_seconds: info.pbi_start_tvsec,
        start_time_microseconds: info.pbi_start_tvusec,
    })
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn inspect_process(_pid: u32) -> ProcessState {
    ProcessState::Unverifiable
}

pub fn current_process_lease_identity() -> Result<SchemaOwnerIdentity, String> {
    let pid = std::process::id();
    let ProcessState::Alive(process_birth) = inspect_process(pid) else {
        return Err("current process birth identity is not verifiable".to_string());
    };
    Ok(SchemaOwnerIdentity {
        machine_id: local_machine_id()?,
        pid: i64::from(pid),
        process_birth: serde_json::to_value(process_birth)
            .map_err(|error| format!("serialize current process birth identity: {error}"))?,
    })
}

#[derive(Debug, Default)]
pub struct SchemaReclaimReport {
    pub reaper_busy: bool,
    pub scanned: usize,
    pub reclaimed: Vec<String>,
    pub protected_live: Vec<String>,
    pub deferred_fresh_heartbeat: Vec<String>,
    pub deferred_server_restart: Vec<String>,
    pub deferred_foreign_machine: Vec<String>,
    pub deferred_unverifiable_owner: Vec<String>,
    pub removed_missing_leases: Vec<String>,
    pub failures: Vec<(String, String)>,
}

struct SchemaLease {
    stop: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl SchemaLease {
    fn request_stop(&self) {
        let _ = self.stop.send(true);
    }

    async fn stop(mut self) {
        self.request_stop();
        if tokio::time::timeout(CLEANUP_PHASE_TIMEOUT, &mut self.task)
            .await
            .is_err()
        {
            self.task.abort();
        }
    }

    fn abort(self) {
        self.request_stop();
        self.task.abort();
    }
}

impl Drop for SchemaLease {
    fn drop(&mut self) {
        self.request_stop();
        self.task.abort();
    }
}

fn valid_knowledge_schema_name(schema: &str) -> bool {
    let Some(suffix) = schema.strip_prefix("knowledge_test_") else {
        return false;
    };
    suffix.len() == 32
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

async fn set_application_name(
    conn: &mut sqlx::PgConnection,
    application_name: &str,
) -> Result<(), String> {
    sqlx::query("SELECT set_config('application_name', $1, false)")
        .bind(application_name)
        .execute(conn)
        .await
        .map_err(|error| format!("set schema lease application_name: {error}"))?;
    Ok(())
}

async fn lock_schema_lease(conn: &mut sqlx::PgConnection, schema: &str) -> Result<(), String> {
    sqlx::query(
        "SELECT pg_advisory_lock(hashtextextended('handshake:knowledge-schema:' || $1, 0))",
    )
    .bind(schema)
    .execute(conn)
    .await
    .map_err(|error| format!("acquire schema ownership lock: {error}"))?;
    Ok(())
}

async fn reconnect_schema_lease(
    base_url: &str,
    schema: &str,
    owner_id: Uuid,
    owner_application_name: &str,
) -> Result<sqlx::PgConnection, String> {
    let mut conn = sqlx::PgConnection::connect(base_url)
        .await
        .map_err(|error| format!("reconnect schema lease: {error}"))?;
    set_application_name(&mut conn, owner_application_name).await?;
    lock_schema_lease(&mut conn, schema).await?;
    let renewed = sqlx::query(
        "UPDATE public.handshake_knowledge_test_schema_leases_v2 \
         SET heartbeat_at = clock_timestamp(), \
             server_started_at = pg_postmaster_start_time() \
         WHERE schema_name = $1 \
           AND owner_id = $2 \
           AND EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)",
    )
    .bind(schema)
    .bind(owner_id)
    .execute(&mut conn)
    .await
    .map_err(|error| format!("renew schema lease after reconnect: {error}"))?;
    if renewed.rows_affected() != 1 {
        return Err("schema lease disappeared before reconnect".to_string());
    }
    Ok(conn)
}

async fn run_schema_lease(
    mut conn: sqlx::PgConnection,
    base_url: String,
    schema: String,
    owner_id: Uuid,
    owner_application_name: String,
    mut stop: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            changed = stop.changed() => {
                if changed.is_err() || *stop.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(LEASE_HEARTBEAT_INTERVAL) => {
                let heartbeat = sqlx::query(
                    "UPDATE public.handshake_knowledge_test_schema_leases_v2 \
                     SET heartbeat_at = clock_timestamp(), \
                         server_started_at = pg_postmaster_start_time() \
                     WHERE schema_name = $1 AND owner_id = $2",
                )
                .bind(&schema)
                .bind(owner_id)
                .execute(&mut conn)
                .await;
                if matches!(heartbeat, Ok(ref result) if result.rows_affected() == 1) {
                    continue;
                }

                drop(conn);
                loop {
                    tokio::select! {
                        changed = stop.changed() => {
                            if changed.is_err() || *stop.borrow() {
                                return;
                            }
                        }
                        _ = tokio::time::sleep(LEASE_RECONNECT_INTERVAL) => {
                            match reconnect_schema_lease(
                                &base_url,
                                &schema,
                                owner_id,
                                &owner_application_name,
                            )
                            .await
                            {
                                Ok(reconnected) => {
                                    conn = reconnected;
                                    break;
                                }
                                Err(error) if error == "schema lease disappeared before reconnect" => {
                                    return;
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
        }
    }
    let _ = sqlx::query(
        "SELECT pg_advisory_unlock(hashtextextended('handshake:knowledge-schema:' || $1, 0))",
    )
    .bind(&schema)
    .execute(&mut conn)
    .await;
    let _ = conn.close().await;
}

async fn start_schema_lease(base_url: &str, schema: &str) -> Result<SchemaLease, String> {
    let owner_id = Uuid::now_v7();
    let owner = current_process_lease_identity()?;
    let owner_application_name = format!("knowledge_lease_{}", owner_id.simple());
    let mut conn = sqlx::PgConnection::connect(base_url)
        .await
        .map_err(|error| format!("connect schema lease: {error}"))?;
    set_application_name(&mut conn, &owner_application_name).await?;
    lock_schema_lease(&mut conn, schema).await?;
    sqlx::query(
        "INSERT INTO public.handshake_knowledge_test_schema_leases_v2 \
         (schema_name, owner_id, owner_application_name, owner_machine_id, owner_pid, \
          owner_process_birth) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(schema)
    .bind(owner_id)
    .bind(&owner_application_name)
    .bind(owner.machine_id)
    .bind(owner.pid)
    .bind(owner.process_birth)
    .execute(&mut conn)
    .await
    .map_err(|error| format!("record schema lease: {error}"))?;

    let (stop_tx, stop_rx) = watch::channel(false);
    let task = tokio::spawn(run_schema_lease(
        conn,
        base_url.to_string(),
        schema.to_string(),
        owner_id,
        owner_application_name,
        stop_rx,
    ));
    Ok(SchemaLease {
        stop: stop_tx,
        task,
    })
}

fn drop_schema_sql(schema: &str) -> String {
    format!(
        "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
        schema.replace('"', "\"\"")
    )
}

async fn unlock_schema(conn: &mut sqlx::PgConnection, schema: &str) {
    let _ = sqlx::query(
        "SELECT pg_advisory_unlock(hashtextextended('handshake:knowledge-schema:' || $1, 0))",
    )
    .bind(schema)
    .execute(conn)
    .await;
}

/// Reclaim only schemas carrying a durable v2 MT-123 lease whose exact local
/// owner process is proven dead or PID-reused. Legacy unleased schemas and v1
/// leases are intentionally outside this automatic path: they need the
/// one-time backup/emptiness gate required by MT-123.
pub async fn reclaim_orphaned_knowledge_schemas(
    base_url: &str,
) -> Result<SchemaReclaimReport, String> {
    let mut report = SchemaReclaimReport::default();
    let mut conn = sqlx::PgConnection::connect(base_url)
        .await
        .map_err(|error| format!("connect knowledge schema reclaimer: {error}"))?;
    set_application_name(&mut conn, "handshake_knowledge_schema_reaper").await?;

    let owns_reaper_lock: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_lock(\
            hashtextextended('handshake:knowledge-schema-reaper:v2', 0)\
         )",
    )
    .fetch_one(&mut conn)
    .await
    .map_err(|error| format!("probe knowledge schema reaper lock: {error}"))?;
    if !owns_reaper_lock {
        report.reaper_busy = true;
        conn.close()
            .await
            .map_err(|error| format!("close busy knowledge schema reclaimer: {error}"))?;
        return Ok(report);
    }

    sqlx::query(LEASE_TABLE_SQL)
        .execute(&mut conn)
        .await
        .map_err(|error| format!("ensure knowledge schema lease table: {error}"))?;

    let local_machine_id = local_machine_id()?;
    let candidates: Vec<(String, Uuid, bool, bool, bool, Uuid, i64, serde_json::Value)> =
        sqlx::query_as(
            "SELECT l.schema_name, \
                l.owner_id, \
                EXISTS (SELECT 1 FROM pg_namespace n WHERE n.nspname = l.schema_name), \
                l.server_started_at = pg_postmaster_start_time(), \
                l.heartbeat_at <= clock_timestamp() - ($1::bigint * interval '1 second'), \
                l.owner_machine_id, \
                l.owner_pid, \
                l.owner_process_birth \
         FROM public.handshake_knowledge_test_schema_leases_v2 l \
         ORDER BY l.created_at, l.schema_name",
        )
        .bind(LEASE_STALE_SECONDS)
        .fetch_all(&mut conn)
        .await
        .map_err(|error| format!("list knowledge schema leases: {error}"))?;
    report.scanned = candidates.len();

    for (
        schema,
        owner_id,
        schema_exists,
        same_server,
        _,
        owner_machine_id,
        owner_pid,
        owner_process_birth,
    ) in candidates
    {
        if !valid_knowledge_schema_name(&schema) {
            report
                .failures
                .push((schema, "lease has an invalid schema name".to_string()));
            continue;
        }
        let owns_reclaim_lock: bool = match sqlx::query_scalar(
            "SELECT pg_try_advisory_lock(\
                hashtextextended('handshake:knowledge-schema:' || $1, 0)\
             )",
        )
        .bind(&schema)
        .fetch_one(&mut conn)
        .await
        {
            Ok(owns) => owns,
            Err(error) => {
                report
                    .failures
                    .push((schema, format!("probe schema ownership lock: {error}")));
                continue;
            }
        };
        if !owns_reclaim_lock {
            report.protected_live.push(schema);
            continue;
        }
        if !schema_exists {
            match sqlx::query(
                "DELETE FROM public.handshake_knowledge_test_schema_leases_v2 \
                 WHERE schema_name = $1 AND owner_id = $2",
            )
            .bind(&schema)
            .bind(owner_id)
            .execute(&mut conn)
            .await
            {
                Ok(_) => report.removed_missing_leases.push(schema.clone()),
                Err(error) => report.failures.push((
                    schema.clone(),
                    format!("remove lease for missing schema: {error}"),
                )),
            }
            unlock_schema(&mut conn, &schema).await;
            continue;
        }

        // A PostgreSQL restart releases every session advisory lock, including
        // locks held for owners whose OS process/worktree is still alive. A
        // timeout is not proof of owner death, so never auto-drop a lease from
        // an older postmaster generation. A surviving owner will reconnect and
        // refresh `server_started_at`; a dead owner remains conservatively
        // deferred for an explicit operator-controlled sweep.
        if !same_server {
            report.deferred_server_restart.push(schema.clone());
            unlock_schema(&mut conn, &schema).await;
            continue;
        }

        let safety: Option<(bool, bool)> = sqlx::query_as(
            "SELECT l.heartbeat_at <= clock_timestamp() \
                        - ($3::bigint * interval '1 second'), \
                    EXISTS (\
                        SELECT 1 FROM pg_stat_activity a \
                        WHERE a.datname = current_database() \
                          AND a.pid <> pg_backend_pid() \
                          AND (a.application_name = $1 \
                               OR a.application_name = l.owner_application_name)\
                    ) \
             FROM public.handshake_knowledge_test_schema_leases_v2 l \
             WHERE l.schema_name = $1 AND l.owner_id = $2",
        )
        .bind(&schema)
        .bind(owner_id)
        .bind(LEASE_STALE_SECONDS)
        .fetch_optional(&mut conn)
        .await
        .map_err(|error| format!("recheck schema ownership predicate: {error}"))?;

        let Some((heartbeat_stale, has_live_session)) = safety else {
            unlock_schema(&mut conn, &schema).await;
            continue;
        };
        if has_live_session {
            report.protected_live.push(schema.clone());
            unlock_schema(&mut conn, &schema).await;
            continue;
        }
        // Advisory locks disappear when their connection drops. A still-live
        // owner therefore has a brief reconnect window where the reaper can
        // acquire the lock. Never treat that window as proof of abandonment:
        // the durable heartbeat must also be stale before deletion.
        if !heartbeat_stale {
            report.deferred_fresh_heartbeat.push(schema.clone());
            unlock_schema(&mut conn, &schema).await;
            continue;
        }

        if owner_machine_id != local_machine_id {
            report.deferred_foreign_machine.push(schema.clone());
            unlock_schema(&mut conn, &schema).await;
            continue;
        }
        let recorded_birth =
            match serde_json::from_value::<ProcessBirthIdentity>(owner_process_birth) {
                Ok(recorded_birth) => recorded_birth,
                Err(_) => {
                    report.deferred_unverifiable_owner.push(schema.clone());
                    unlock_schema(&mut conn, &schema).await;
                    continue;
                }
            };
        let Ok(owner_pid) = u32::try_from(owner_pid) else {
            report.deferred_unverifiable_owner.push(schema.clone());
            unlock_schema(&mut conn, &schema).await;
            continue;
        };
        match inspect_process(owner_pid) {
            ProcessState::Alive(actual_birth) if actual_birth == recorded_birth => {
                report.protected_live.push(schema.clone());
                unlock_schema(&mut conn, &schema).await;
                continue;
            }
            ProcessState::Alive(_) | ProcessState::Dead => {}
            ProcessState::Unverifiable => {
                report.deferred_unverifiable_owner.push(schema.clone());
                unlock_schema(&mut conn, &schema).await;
                continue;
            }
        }

        let drop_result = tokio::time::timeout(
            RECLAIM_DROP_TIMEOUT,
            sqlx::query(&drop_schema_sql(&schema)).execute(&mut conn),
        )
        .await;
        match drop_result {
            Ok(Ok(_)) => {
                let delete_result = sqlx::query(
                    "DELETE FROM public.handshake_knowledge_test_schema_leases_v2 \
                     WHERE schema_name = $1 AND owner_id = $2",
                )
                .bind(&schema)
                .bind(owner_id)
                .execute(&mut conn)
                .await;
                match delete_result {
                    Ok(_) => report.reclaimed.push(schema.clone()),
                    Err(error) => report
                        .failures
                        .push((schema.clone(), format!("remove reclaimed lease: {error}"))),
                }
            }
            Ok(Err(error)) => report
                .failures
                .push((schema.clone(), format!("drop orphaned schema: {error}"))),
            Err(_) => report
                .failures
                .push((schema.clone(), "drop orphaned schema timed out".to_string())),
        }
        unlock_schema(&mut conn, &schema).await;
    }

    let _ = sqlx::query(
        "SELECT pg_advisory_unlock(\
            hashtextextended('handshake:knowledge-schema-reaper:v2', 0)\
         )",
    )
    .execute(&mut conn)
    .await;
    conn.close()
        .await
        .map_err(|error| format!("close knowledge schema reclaimer: {error}"))?;
    Ok(report)
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
        RECLAIM_DROP_TIMEOUT,
        sqlx::query(&drop_schema_sql(schema)).execute(&mut conn),
    )
    .await
    .map_err(|_| "drop exact isolated knowledge schema timed out".to_string())?
    .map_err(|error| format!("drop exact isolated knowledge schema: {error}"))?;
    sqlx::query(
        "DELETE FROM public.handshake_knowledge_test_schema_leases_v2 WHERE schema_name = $1",
    )
    .bind(schema)
    .execute(&mut conn)
    .await
    .map_err(|error| format!("remove isolated knowledge schema lease: {error}"))?;
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
    lease: Option<SchemaLease>,
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
        if let Some(lease) = self.lease.take() {
            lease.stop().await;
        }
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
        if let Some(lease) = self.lease.take() {
            lease.abort();
        }
        let db = &self.db;
        let cleanup = std::thread::scope(|scope| {
            let handle = scope.spawn(move || -> Result<(), String> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| format!("build isolated schema teardown runtime: {error}"))?;
                runtime.block_on(async {
                    let _ = tokio::time::timeout(CLEANUP_PHASE_TIMEOUT, db.close()).await;
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
    let mut conn = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect to PostgreSQL for schema setup");
    set_application_name(&mut conn, "handshake_knowledge_schema_setup")
        .await
        .expect("identify isolated schema setup connection");
    tokio::time::timeout(
        SCHEMA_SETUP_LOCK_TIMEOUT,
        sqlx::query(
            "SELECT pg_advisory_lock(\
                hashtextextended('handshake:knowledge-schema-setup:v1', 0)\
             )",
        )
        .execute(&mut conn),
    )
    .await
    .expect("wait for cross-process isolated schema setup lock")
    .expect("acquire cross-process isolated schema setup lock");

    let reclaim = reclaim_orphaned_knowledge_schemas(&url)
        .await
        .expect("reclaim abandoned isolated knowledge schemas");
    if !reclaim.reclaimed.is_empty() {
        eprintln!(
            "MT-123 reclaimed {} abandoned knowledge test schema(s)",
            reclaim.reclaimed.len()
        );
    }
    if reclaim.reaper_busy {
        eprintln!("MT-123 reclamation already active in another process; schema setup continues");
    }
    if !reclaim.failures.is_empty() {
        panic!(
            "MT-123 schema reclamation failed closed with {} failure(s): {:?}",
            reclaim.failures.len(),
            reclaim.failures
        );
    }

    let schema = format!("knowledge_test_{}", Uuid::now_v7().simple());
    let lease = start_schema_lease(&url, &schema)
        .await
        .expect("acquire isolated knowledge schema lease");
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
    sqlx::query(
        "SELECT pg_advisory_unlock(\
            hashtextextended('handshake:knowledge-schema-setup:v1', 0)\
         )",
    )
    .execute(&mut conn)
    .await
    .expect("release cross-process isolated schema setup lock");
    conn.close()
        .await
        .expect("close isolated schema setup connection");

    Some(KnowledgePg {
        db,
        schema,
        schema_url,
        base_url: url,
        application_name,
        lease: Some(lease),
        torn_down: false,
    })
}
