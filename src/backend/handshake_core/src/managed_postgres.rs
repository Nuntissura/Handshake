//! Managed PostgreSQL lifecycle (managed-PG-lifecycle feature, task #9).
//!
//! Handshake can auto-start a hidden, embedded PostgreSQL cluster on startup,
//! wait until it accepts connections, ensure the application database exists,
//! and stop it again on shutdown. This removes the requirement that an operator
//! manually start PostgreSQL before launching Handshake. No Docker is involved
//! and no SQLite fallback is used; this drives a real local `postgres` install.
//!
//! HBR-QUIET: every child process this module spawns (`initdb`, `pg_ctl`,
//! `pg_isready`, `psql`) is launched with the Windows `CREATE_NO_WINDOW`
//! creation flag so no console window pops while Handshake runs the cluster in
//! the background. This mirrors the exact convention used by the cloud CLI
//! bridge (`model_runtime::cloud::official_cli_bridge`).
//!
//! [GLOBAL-PORTABILITY] disk-agnostic: defaults never hardcode a drive letter
//! or user-profile path. The cluster data directory is resolved relative to the
//! crate manifest by walking up to the repo root (mirroring
//! `init_flight_recorder`'s root resolution in `main.rs`), and every value is
//! overridable through environment variables so the project can be moved to
//! another folder or disk without code changes.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use thiserror::Error;
use tokio::process::Command;
use tokio::time::{sleep, Instant};

/// Environment variable that toggles the managed cluster on/off.
pub const MANAGED_PG_ENABLED_ENV: &str = "HANDSHAKE_MANAGED_PG_ENABLED";
/// Environment variable overriding the TCP port the managed cluster listens on.
pub const MANAGED_PG_PORT_ENV: &str = "HANDSHAKE_MANAGED_PG_PORT";
/// Environment variable overriding the cluster data directory.
pub const MANAGED_PG_DATA_DIR_ENV: &str = "HANDSHAKE_MANAGED_PG_DATA_DIR";
/// Environment variable overriding the directory that holds the PG binaries.
pub const MANAGED_PG_BIN_ENV: &str = "HANDSHAKE_MANAGED_PG_BIN";
/// Standard PostgreSQL environment variable pointing at the binary directory.
pub const PGBIN_ENV: &str = "PGBIN";

/// Default managed listen port. Chosen off the standard 5432 so a managed
/// instance does not clash with a pre-existing operator-run PostgreSQL.
pub const DEFAULT_MANAGED_PG_PORT: u16 = 5544;
/// Default application database created inside the managed cluster.
pub const DEFAULT_DATABASE: &str = "handshake";
/// Default cluster superuser (created by `initdb -U`).
pub const DEFAULT_SUPERUSER: &str = "postgres";
/// Default time to wait for the cluster to begin accepting connections.
pub const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Errors raised while managing the embedded PostgreSQL lifecycle.
#[derive(Debug, Error)]
pub enum ManagedPostgresError {
    /// An underlying IO / process-spawn failure.
    #[error("managed postgres io error: {0}")]
    Io(#[from] std::io::Error),
    /// The cluster did not start accepting connections before the timeout.
    #[error("managed postgres did not accept connections within {0:?}")]
    Timeout(Duration),
    /// `initdb` exited non-zero while creating the cluster.
    #[error("initdb failed: {0}")]
    InitDbFailed(String),
    /// `pg_ctl ... start` exited non-zero.
    #[error("pg_ctl start failed: {0}")]
    StartFailed(String),
    /// The required PostgreSQL binaries could not be located.
    #[error("postgres binaries not found: {0}")]
    BinariesNotFound(String),
}

/// Disk-agnostic configuration for the managed PostgreSQL cluster.
#[derive(Clone, Debug)]
pub struct ManagedPostgresConfig {
    /// When `false` the lifecycle is a no-op and Handshake uses external PG.
    pub enabled: bool,
    /// Cluster data directory (`-D`). Created and `initdb`'d if empty.
    pub data_dir: PathBuf,
    /// TCP port the cluster listens on.
    pub port: u16,
    /// Directory containing `pg_ctl` / `initdb` / `pg_isready` / `psql`.
    /// Empty triggers binary discovery (see [`resolve_bin`]).
    pub bin_dir: PathBuf,
    /// Application database ensured to exist after startup.
    pub database: String,
    /// Cluster superuser created by `initdb`.
    pub superuser: String,
    /// How long to wait for the cluster to accept connections.
    pub startup_timeout: Duration,
}

impl ManagedPostgresConfig {
    /// Build a configuration from the environment with disk-agnostic defaults.
    ///
    /// [GLOBAL-PORTABILITY] the data directory default is resolved relative to
    /// the crate manifest (walking up to the repo root), never a hardcoded
    /// absolute path. Every field is overridable via environment variable.
    ///
    /// `bin_dir` is chosen with this precedence: `HANDSHAKE_MANAGED_PG_BIN`
    /// (operator override) > `PGBIN` > exe-relative bundled dir
    /// `<exe_dir>/bundled/postgres` (auto-discovered for an installed app, only
    /// when its `pg_ctl` actually exists) > empty (which lets [`resolve_bin`]
    /// fall through to `PGBIN` / the Windows default install path / `PATH`).
    pub fn from_env() -> Self {
        let enabled = std::env::var(MANAGED_PG_ENABLED_ENV)
            .ok()
            .map(|v| {
                let v = v.trim().to_ascii_lowercase();
                !(v == "0" || v == "false" || v == "no" || v == "off")
            })
            .unwrap_or(true);

        let port = std::env::var(MANAGED_PG_PORT_ENV)
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(DEFAULT_MANAGED_PG_PORT);

        let data_dir = std::env::var(MANAGED_PG_DATA_DIR_ENV)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(default_data_dir);

        // bin_dir precedence (highest first):
        //   1. HANDSHAKE_MANAGED_PG_BIN  (operator override; MANAGED_PG_BIN_ENV)
        //   2. PGBIN                     (standard PostgreSQL override)
        //   3. exe-relative bundled dir  (<exe_dir>/bundled/postgres) — used ONLY
        //      when its pg_ctl actually exists, so an installed app auto-discovers
        //      its bundled cluster without any env export. A random system PG is
        //      thereby beaten by the bundled one; an incomplete bundle (pg_ctl
        //      present, sibling binary missing) still hard-errors in resolve_bin
        //      step 1 rather than silently using a different-version system PG.
        //   4. empty -> resolve_bin falls through to PGBIN / Windows default / PATH.
        // Operator env always wins; bundled discovery is exe-relative and
        // disk-agnostic (no hardcoded absolute path).
        //
        // Each candidate is validated for non-emptiness INDEPENDENTLY via
        // `nonempty_env`. A set-but-empty HANDSHAKE_MANAGED_PG_BIN must NOT
        // shadow PGBIN: it correctly falls through to the next candidate instead
        // of short-circuiting the chain with an empty `Some("")`. The env-fed
        // candidates and the bundled fallback are combined by the pure
        // `resolve_bin_dir` helper so the precedence/fall-through logic is
        // unit-testable without mutating global process environment.
        let bin_dir = resolve_bin_dir(
            nonempty_env(MANAGED_PG_BIN_ENV),
            nonempty_env(PGBIN_ENV),
            bundled_bin_dir_from_current_exe(),
        );

        let config = Self {
            enabled,
            data_dir,
            port,
            bin_dir,
            database: DEFAULT_DATABASE.to_string(),
            superuser: DEFAULT_SUPERUSER.to_string(),
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
        };

        tracing::info!(
            target: "handshake_core::managed_postgres",
            enabled = config.enabled,
            port = config.port,
            data_dir = %config.data_dir.display(),
            bin_dir = %config.bin_dir.display(),
            database = %config.database,
            "Managed PostgreSQL config initialized"
        );

        config
    }
}

/// Read an environment variable as a non-empty `PathBuf` candidate.
///
/// Returns `None` when the variable is unset, empty, or whitespace-only, so an
/// explicitly-set-but-empty variable (`Some("")`) does NOT short-circuit a
/// `.or_else(...)` precedence chain. This lets each `bin_dir` candidate
/// (`HANDSHAKE_MANAGED_PG_BIN`, then `PGBIN`) be validated independently: an
/// empty higher-precedence variable falls through to the next candidate instead
/// of winning with an empty value. The raw (non-trimmed) value is used to build
/// the path so a deliberately-spaced directory name is preserved; only the
/// empty/whitespace decision is trim-based.
fn nonempty_env(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
}

/// Combine the `bin_dir` candidates in precedence order into the resolved dir.
///
/// Precedence (highest first): `managed_pg_bin` (HANDSHAKE_MANAGED_PG_BIN) >
/// `pgbin` (PGBIN) > `bundled` (exe-relative bundled dir) > empty
/// `PathBuf::default()` (which lets [`resolve_bin`] fall through to PGBIN / the
/// Windows default install path / PATH).
///
/// Pure and unit-testable: callers pass the already-non-empty-validated
/// candidates (see [`nonempty_env`]), so this never reads the environment and
/// has no global-env race. Because empty candidates are represented as `None`,
/// an empty higher-precedence variable correctly falls through to the next
/// candidate rather than short-circuiting with an empty value.
fn resolve_bin_dir(
    managed_pg_bin: Option<PathBuf>,
    pgbin: Option<PathBuf>,
    bundled: Option<PathBuf>,
) -> PathBuf {
    managed_pg_bin.or(pgbin).or(bundled).unwrap_or_default()
}

/// Resolve the default cluster data directory disk-agnostically.
///
/// Mirrors `init_flight_recorder`'s root resolution: the crate manifest lives
/// at `<repo>/src/backend/handshake_core`, so walking three parents yields the
/// repo root. The managed cluster data then lives under a sibling
/// `Handshake_Artifacts/handshake-product/managed_pgdata` path. If the root
/// cannot be resolved (unexpected layout), fall back to a relative path under
/// the manifest.
fn default_data_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    match root_dir {
        // The shared `Handshake_Artifacts/` root is a SIBLING of the repo root
        // (it lives in the worktrees container, `root.parent()`), never inside
        // the worktree. Climbing only to `root` placed `managed_pgdata` inside
        // the worktree; go one level further up to reach the sibling.
        Some(root) => {
            let base = root.parent().map(Path::to_path_buf);
            base.unwrap_or(root)
                .join("Handshake_Artifacts")
                .join("handshake-product")
                .join("managed_pgdata")
        }
        None => manifest_dir
            .join("Handshake_Artifacts")
            .join("handshake-product")
            .join("managed_pgdata"),
    }
}

/// A handle to the (possibly managed) PostgreSQL cluster.
#[derive(Debug)]
pub struct ManagedPostgres {
    config: ManagedPostgresConfig,
    /// Postmaster OS pid when this instance actually started the cluster.
    /// `None` when disabled, or when an already-running cluster was adopted.
    os_pid: Option<u32>,
    /// `true` only when this instance started the cluster and therefore owns
    /// its shutdown. `false` for disabled or adopted/already-running clusters.
    started_here: bool,
}

impl ManagedPostgres {
    /// Ensure a PostgreSQL cluster is running and the app database exists.
    ///
    /// This is idempotent: if a cluster is already accepting connections on the
    /// configured port it is adopted (never double-started) and shutdown is not
    /// owned by this handle. When disabled, returns an external/disabled handle
    /// whose [`database_url`](Self::database_url) is still derivable.
    pub async fn ensure_running(
        config: ManagedPostgresConfig,
    ) -> Result<Self, ManagedPostgresError> {
        // 1. Disabled -> external state; caller uses an externally-run PG.
        if !config.enabled {
            tracing::info!(
                target: "handshake_core::managed_postgres",
                "Managed PostgreSQL disabled; using external cluster"
            );
            return Ok(Self {
                config,
                os_pid: None,
                started_here: false,
            });
        }

        // 2. Locate the binaries (BinariesNotFound if pg_ctl is missing).
        let pg_ctl = resolve_bin(&config.bin_dir, "pg_ctl")?;
        let initdb = resolve_bin(&config.bin_dir, "initdb")?;
        let pg_isready = resolve_bin(&config.bin_dir, "pg_isready")?;
        let psql = resolve_bin(&config.bin_dir, "psql")?;
        // Validation-only resolve of the server binary `postgres`. `pg_ctl`
        // launches `postgres` (the postmaster); if a bundle/install ships
        // `pg_ctl` but is missing `postgres`(.exe), discovery + the four
        // client-tool resolves above would pass and the bundle would then fail
        // OPAQUELY when `pg_ctl start` cannot find the server. Resolving it here
        // makes an incomplete bundle fail LOUDLY at startup with
        // `BinariesNotFound`, matching the "incomplete bundle fails loudly"
        // design intent. For an empty `bin_dir` (non-bundled install) this
        // defers to PATH exactly like the others, so non-bundled installs are
        // unaffected; only an explicit/bundled `bin_dir` missing `postgres`
        // hard-errors. (`bundled_bin_dir` stays anchored on `pg_ctl` only — it
        // must NOT require `postgres`, which would risk a silent fallback to a
        // system PG.) The handle is discarded; spawning goes through `pg_ctl`.
        let _postgres_server = resolve_bin(&config.bin_dir, "postgres")?;

        // 3. Already accepting connections -> adopt, never double-start.
        if is_ready(&pg_isready, config.port).await {
            ensure_database(&psql, &config).await?;
            tracing::info!(
                target: "handshake_core::managed_postgres",
                port = config.port,
                database = %config.database,
                "PostgreSQL already accepting connections; adopting existing cluster"
            );
            return Ok(Self {
                config,
                os_pid: None,
                started_here: false,
            });
        }

        // A previous Handshake/test process may have already launched this
        // cluster but not reached pg_isready yet. Do not call `pg_ctl start`
        // against an initialized data dir with a live postmaster marker; wait
        // for readiness and adopt it instead. This prevents no-env tests from
        // racing each other into a false "pg_ctl start failed" path.
        if cluster_initialized(&config.data_dir) && config.data_dir.join("postmaster.pid").exists()
        {
            wait_until_ready(&pg_isready, config.port, config.startup_timeout).await?;
            ensure_database(&psql, &config).await?;
            tracing::info!(
                target: "handshake_core::managed_postgres",
                port = config.port,
                database = %config.database,
                data_dir = %config.data_dir.display(),
                "PostgreSQL postmaster already present; adopting after readiness wait"
            );
            return Ok(Self {
                config,
                os_pid: None,
                started_here: false,
            });
        }

        // 4. initdb if the data directory has no cluster (no PG_VERSION file).
        if !cluster_initialized(&config.data_dir) {
            if let Some(parent) = config.data_dir.parent() {
                std::fs::create_dir_all(parent)?;
            }
            run_initdb(&initdb, &config).await?;
        }

        // 5. Start the cluster detached and poll until ready or timeout.
        start_cluster(&pg_ctl, &config).await?;
        wait_until_ready(&pg_isready, config.port, config.startup_timeout).await?;

        let os_pid = read_postmaster_pid(&config.data_dir);

        // 6. Ensure the application database exists (ignore "already exists").
        ensure_database(&psql, &config).await?;

        tracing::info!(
            target: "handshake_core::managed_postgres",
            port = config.port,
            os_pid = os_pid.unwrap_or(0),
            database = %config.database,
            "Managed PostgreSQL ready"
        );

        Ok(Self {
            config,
            os_pid,
            started_here: true,
        })
    }

    /// Connection URL: `postgres://<superuser>@127.0.0.1:<port>/<database>`.
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}@127.0.0.1:{}/{}",
            self.config.superuser, self.config.port, self.config.database
        )
    }

    /// Postmaster OS pid, when this handle started the cluster.
    pub fn os_pid(&self) -> Option<u32> {
        self.os_pid
    }

    /// `true` when this handle owns the running cluster (started it here).
    /// `false` for disabled/external or adopted already-running clusters.
    pub fn is_managed(&self) -> bool {
        self.started_here
    }

    /// Whether the managed lifecycle is enabled for this configuration.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Stop the cluster with `pg_ctl ... stop -m fast`.
    ///
    /// Idempotent and ownership-scoped: only stops the cluster when this handle
    /// actually started it ([`is_managed`](Self::is_managed)). Disabled or
    /// adopted clusters are left untouched.
    pub async fn stop(&self) -> Result<(), ManagedPostgresError> {
        if !self.started_here {
            tracing::debug!(
                target: "handshake_core::managed_postgres",
                "stop() is a no-op for unmanaged/external cluster"
            );
            return Ok(());
        }

        let pg_ctl = match resolve_bin(&self.config.bin_dir, "pg_ctl") {
            Ok(path) => path,
            Err(err) => {
                // Binaries vanished after start; nothing we can do, but do not
                // hard-fail shutdown over a missing binary.
                tracing::warn!(
                    target: "handshake_core::managed_postgres",
                    error = %err,
                    "pg_ctl not found at shutdown; skipping stop"
                );
                return Ok(());
            }
        };

        let output = no_window(Command::new(&pg_ctl))
            .arg("-D")
            .arg(&self.config.data_dir)
            .arg("stop")
            .arg("-m")
            .arg("fast")
            .output()
            .await?;

        if output.status.success() {
            tracing::info!(
                target: "handshake_core::managed_postgres",
                "Managed PostgreSQL stopped"
            );
        } else {
            // Already stopped / not running is an acceptable idempotent outcome.
            tracing::warn!(
                target: "handshake_core::managed_postgres",
                stderr = %String::from_utf8_lossy(&output.stderr).trim(),
                "pg_ctl stop returned non-zero (treating as already stopped)"
            );
        }
        Ok(())
    }
}

/// Apply the HBR-QUIET no-window creation flag on Windows.
///
/// Mirrors `official_cli_bridge.rs`: `tokio::process::Command` re-exposes the
/// `creation_flags` method via the Windows `CommandExt` trait, so backgrounded
/// child processes never pop a console window. On non-Windows platforms this is
/// a transparent pass-through.
fn no_window(mut cmd: Command) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Platform executable name (`<name>.exe` on Windows).
fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Pure, testable discovery of the bundled PostgreSQL bin dir for an installed
/// Handshake app.
///
/// An installed Handshake stages its managed-postgres binaries at the
/// exe-relative path `<exe_dir>/bundled/postgres/` (see
/// `installer/windows/BUNDLED_DEPS_POLICY.md`, bundle-layout topic). This
/// function returns `Some(<exe_dir>/bundled/postgres)` ONLY when that directory
/// actually contains the anchor binary `pg_ctl` (`pg_ctl.exe` on Windows);
/// otherwise it returns `None`, so a missing or non-bundled install never
/// produces a bogus path.
///
/// It is pure: it takes `exe_dir` explicitly and reads no environment and never
/// calls `current_exe`, so it is unit-testable with a temp directory. The thin
/// wrapper [`bundled_bin_dir_from_current_exe`] feeds it the real exe directory.
///
/// [GLOBAL-PORTABILITY] disk-agnostic: the path is derived relative to the exe
/// directory the caller supplies; no drive letter or user-profile path is
/// hardcoded.
fn bundled_bin_dir(exe_dir: &Path) -> Option<PathBuf> {
    let candidate = exe_dir.join("bundled").join("postgres");
    if candidate.join(exe_name("pg_ctl")).is_file() {
        Some(candidate)
    } else {
        None
    }
}

/// Thin wrapper resolving the bundled PostgreSQL bin dir from the running exe.
///
/// Calls `std::env::current_exe()` and feeds its parent directory to
/// [`bundled_bin_dir`]. Returns `None` on any error (no current exe, no parent,
/// or no bundled `pg_ctl` present) and never panics, so it is safe to use as a
/// best-effort discovery fallback during config construction.
fn bundled_bin_dir_from_current_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    bundled_bin_dir(exe_dir)
}

/// Resolve a PostgreSQL binary by name.
///
/// Discovery order:
/// 1. `config.bin_dir` (explicit override) if non-empty. This carries, in
///    descending precedence as resolved by [`ManagedPostgresConfig::from_env`],
///    the `HANDSHAKE_MANAGED_PG_BIN` override, then `PGBIN`, then the
///    exe-relative bundled dir `<exe_dir>/bundled/postgres` auto-discovered for
///    an installed app (only when its `pg_ctl` exists). A non-empty `bin_dir`
///    HARD-ERRORS with [`ManagedPostgresError::BinariesNotFound`] if the binary
///    is absent there — it never falls through. This is intentional: an
///    incomplete bundle (e.g. `pg_ctl` present but `initdb` missing) must fail
///    loudly rather than silently mixing in a different-version system PG.
/// 2. `PGBIN` environment variable (only reached when `bin_dir` is empty).
/// 3. `C:/Program Files/PostgreSQL/16/bin` on Windows (common install path).
/// 4. Bare name on `PATH` (resolved by the OS at spawn time).
///
/// Returns [`ManagedPostgresError::BinariesNotFound`] only when a resolvable
/// directory candidate exists but the binary is absent there; if no directory
/// candidate matches, the bare name is returned to defer to `PATH`. The caller
/// resolves the required `pg_ctl` first, so a truly missing toolchain surfaces
/// as `BinariesNotFound` for `pg_ctl`.
fn resolve_bin(bin_dir: &Path, name: &str) -> Result<PathBuf, ManagedPostgresError> {
    let exe = exe_name(name);

    // 1. Explicit bin_dir override.
    if !bin_dir.as_os_str().is_empty() {
        let candidate = bin_dir.join(&exe);
        if candidate.is_file() {
            return Ok(candidate);
        }
        return Err(ManagedPostgresError::BinariesNotFound(format!(
            "{} not found in configured bin_dir {}",
            exe,
            bin_dir.display()
        )));
    }

    // 2. PGBIN environment variable.
    if let Ok(pgbin) = std::env::var(PGBIN_ENV) {
        let pgbin = pgbin.trim();
        if !pgbin.is_empty() {
            let candidate = Path::new(pgbin).join(&exe);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    // 3. Common Windows default install path.
    #[cfg(windows)]
    {
        let candidate = Path::new("C:/Program Files/PostgreSQL/16/bin").join(&exe);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    // 4. Fall back to PATH resolution at spawn time, except for the anchor
    //    binary pg_ctl: if nothing has been found by now and pg_ctl itself is
    //    not on PATH, the caller should learn the toolchain is missing.
    if name == "pg_ctl" && which_on_path(&exe).is_none() {
        return Err(ManagedPostgresError::BinariesNotFound(format!(
            "{exe} not found in bin_dir, PGBIN, default install path, or PATH"
        )));
    }
    Ok(PathBuf::from(exe))
}

/// Minimal PATH lookup for an executable name (no external crates).
fn which_on_path(exe: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// A cluster is initialized when its data directory holds a `PG_VERSION` file.
fn cluster_initialized(data_dir: &Path) -> bool {
    data_dir.join("PG_VERSION").is_file()
}

/// Run `pg_isready -h 127.0.0.1 -p <port>` and report whether it exited 0.
async fn is_ready(pg_isready: &Path, port: u16) -> bool {
    match no_window(Command::new(pg_isready))
        .arg("-h")
        .arg("127.0.0.1")
        .arg("-p")
        .arg(port.to_string())
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Run `initdb -D <data_dir> -U <superuser> --auth=trust --encoding=UTF8`.
async fn run_initdb(
    initdb: &Path,
    config: &ManagedPostgresConfig,
) -> Result<(), ManagedPostgresError> {
    tracing::info!(
        target: "handshake_core::managed_postgres",
        data_dir = %config.data_dir.display(),
        "Initializing PostgreSQL cluster (initdb)"
    );
    let output = no_window(Command::new(initdb))
        .arg("-D")
        .arg(&config.data_dir)
        .arg("-U")
        .arg(&config.superuser)
        .arg("--auth=trust")
        .arg("--encoding=UTF8")
        .output()
        .await?;
    if !output.status.success() {
        return Err(ManagedPostgresError::InitDbFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

/// Start the cluster detached.
///
/// `pg_ctl -D <data_dir> -o "-p <port>" -l <data_dir>/postgres.log start`.
/// The blocking `-w` flag is deliberately omitted because it can hang on
/// Windows; readiness is established afterward by polling `pg_isready`.
async fn start_cluster(
    pg_ctl: &Path,
    config: &ManagedPostgresConfig,
) -> Result<(), ManagedPostgresError> {
    let log_path = config.data_dir.join("postgres.log");
    tracing::info!(
        target: "handshake_core::managed_postgres",
        port = config.port,
        log = %log_path.display(),
        "Starting PostgreSQL cluster (pg_ctl start)"
    );
    // CRITICAL (Windows): `pg_ctl start` launches the long-lived postmaster,
    // which inherits the parent's stdio handles and keeps them open for its
    // whole lifetime. Capturing stdout/stderr via `.output()` would therefore
    // block forever waiting for an EOF that never comes (the postmaster never
    // closes the pipe). Redirect the child's stdio to null so no pipe is
    // inherited, and use `.status()` — pg_ctl (started without the blocking
    // `-w`) exits promptly once the postmaster is launched. Startup diagnostics
    // are still captured in the `-l` log file.
    let status = no_window(Command::new(pg_ctl))
        .arg("-D")
        .arg(&config.data_dir)
        .arg("-o")
        .arg(format!("-p {}", config.port))
        .arg("-l")
        .arg(&log_path)
        .arg("start")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;
    if !status.success() {
        let log_hint = std::fs::read_to_string(&log_path)
            .ok()
            .map(|s| s.lines().rev().take(5).collect::<Vec<_>>().join(" | "))
            .unwrap_or_default();
        return Err(ManagedPostgresError::StartFailed(format!(
            "pg_ctl start exited with {status}; recent log: {log_hint}"
        )));
    }
    Ok(())
}

/// Poll `pg_isready` until success or the startup timeout elapses.
async fn wait_until_ready(
    pg_isready: &Path,
    port: u16,
    timeout: Duration,
) -> Result<(), ManagedPostgresError> {
    let deadline = Instant::now() + timeout;
    let poll_interval = Duration::from_millis(250);
    loop {
        if is_ready(pg_isready, port).await {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(ManagedPostgresError::Timeout(timeout));
        }
        sleep(poll_interval).await;
    }
}

/// Read the postmaster pid from `<data_dir>/postmaster.pid` (first line).
fn read_postmaster_pid(data_dir: &Path) -> Option<u32> {
    let contents = std::fs::read_to_string(data_dir.join("postmaster.pid")).ok()?;
    contents.lines().next()?.trim().parse().ok()
}

/// Ensure the application database exists.
///
/// Connects as the superuser to the maintenance database `postgres` and issues
/// `CREATE DATABASE <database>`. A pre-existing database (PostgreSQL error
/// "already exists") is treated as success so the call is idempotent.
async fn ensure_database(
    psql: &Path,
    config: &ManagedPostgresConfig,
) -> Result<(), ManagedPostgresError> {
    let sql = format!("CREATE DATABASE \"{}\"", config.database);
    let output = no_window(Command::new(psql))
        .arg("-h")
        .arg("127.0.0.1")
        .arg("-p")
        .arg(config.port.to_string())
        .arg("-U")
        .arg(&config.superuser)
        .arg("-d")
        .arg("postgres")
        .arg("-v")
        .arg("ON_ERROR_STOP=0")
        .arg("-c")
        .arg(&sql)
        .output()
        .await?;

    if output.status.success() {
        tracing::info!(
            target: "handshake_core::managed_postgres",
            database = %config.database,
            "Ensured application database exists"
        );
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("already exists") {
        return Ok(());
    }

    // Database creation failed for some other reason. The cluster is up, so do
    // not fail the whole lifecycle; surface a warning and let storage init give
    // the authoritative connection error if the db is truly missing.
    tracing::warn!(
        target: "handshake_core::managed_postgres",
        database = %config.database,
        stderr = %String::from_utf8_lossy(&output.stderr).trim(),
        "CREATE DATABASE returned non-zero (continuing)"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_url_uses_superuser_loopback_port_database() {
        let pg = ManagedPostgres {
            config: ManagedPostgresConfig {
                enabled: true,
                data_dir: PathBuf::from("pgdata"),
                port: 5544,
                bin_dir: PathBuf::new(),
                database: "handshake".to_string(),
                superuser: "postgres".to_string(),
                startup_timeout: DEFAULT_STARTUP_TIMEOUT,
            },
            os_pid: Some(1234),
            started_here: true,
        };
        assert_eq!(
            pg.database_url(),
            "postgres://postgres@127.0.0.1:5544/handshake"
        );
        assert!(pg.is_managed());
        assert!(pg.is_enabled());
        assert_eq!(pg.os_pid(), Some(1234));
    }

    #[tokio::test]
    async fn disabled_config_returns_external_handle_without_spawning() {
        let config = ManagedPostgresConfig {
            enabled: false,
            data_dir: PathBuf::from("pgdata"),
            port: 6000,
            bin_dir: PathBuf::new(),
            database: "handshake".to_string(),
            superuser: "postgres".to_string(),
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
        };
        let pg = ManagedPostgres::ensure_running(config)
            .await
            .expect("disabled lifecycle must not error");
        assert!(!pg.is_managed());
        assert!(!pg.is_enabled());
        assert_eq!(pg.os_pid(), None);
        assert_eq!(
            pg.database_url(),
            "postgres://postgres@127.0.0.1:6000/handshake"
        );
        // stop() on an unmanaged handle is a no-op and must not error.
        pg.stop()
            .await
            .expect("stop must be a no-op when unmanaged");
    }

    #[test]
    fn from_env_defaults_are_disk_agnostic() {
        // default_data_dir must resolve to a relative-rooted path (no panic)
        // and end with the managed_pgdata leaf, never a hardcoded drive root.
        let data_dir = default_data_dir();
        assert!(data_dir.ends_with("managed_pgdata"));
        assert!(data_dir
            .components()
            .any(|component| component.as_os_str() == "handshake-product"));
    }

    #[test]
    fn exe_name_adds_exe_on_windows_only() {
        let resolved = exe_name("pg_ctl");
        if cfg!(windows) {
            assert_eq!(resolved, "pg_ctl.exe");
        } else {
            assert_eq!(resolved, "pg_ctl");
        }
    }

    #[test]
    fn missing_binary_in_explicit_bin_dir_is_binaries_not_found() {
        let bin_dir = PathBuf::from("definitely-not-a-real-pg-bin-dir-xyz");
        let err = resolve_bin(&bin_dir, "pg_ctl").unwrap_err();
        matches!(err, ManagedPostgresError::BinariesNotFound(_));
    }

    #[test]
    fn bundled_bin_dir_some_when_pg_ctl_present() {
        // A temp dir laid out like an installed app:
        // <exe_dir>/bundled/postgres/pg_ctl(.exe)
        let temp = tempfile::tempdir().expect("tempdir");
        let exe_dir = temp.path();
        let pg_dir = exe_dir.join("bundled").join("postgres");
        std::fs::create_dir_all(&pg_dir).expect("create bundled/postgres");
        std::fs::write(pg_dir.join(exe_name("pg_ctl")), b"#!stub").expect("write pg_ctl");

        let resolved = bundled_bin_dir(exe_dir);
        assert_eq!(
            resolved.as_deref(),
            Some(pg_dir.as_path()),
            "bundled_bin_dir must return the exe-relative bundled/postgres path when pg_ctl exists there"
        );
    }

    #[test]
    fn bundled_bin_dir_none_when_no_bundled_dir() {
        // A temp dir with NO bundled/postgres subtree at all.
        let temp = tempfile::tempdir().expect("tempdir");
        let resolved = bundled_bin_dir(temp.path());
        assert!(
            resolved.is_none(),
            "bundled_bin_dir must return None when no bundled/postgres dir exists"
        );
    }

    #[test]
    fn bundled_bin_dir_none_when_dir_present_but_pg_ctl_absent() {
        // bundled/postgres exists but the anchor binary pg_ctl is missing:
        // an incomplete/empty stage must NOT be treated as a usable bundle.
        let temp = tempfile::tempdir().expect("tempdir");
        let pg_dir = temp.path().join("bundled").join("postgres");
        std::fs::create_dir_all(&pg_dir).expect("create bundled/postgres");
        // Intentionally write a sibling but NOT pg_ctl, to prove the check keys
        // specifically on pg_ctl, not on directory existence.
        std::fs::write(pg_dir.join(exe_name("initdb")), b"#!stub").expect("write initdb");

        let resolved = bundled_bin_dir(temp.path());
        assert!(
            resolved.is_none(),
            "bundled_bin_dir must return None when bundled/postgres lacks pg_ctl"
        );
    }

    #[test]
    fn empty_managed_pg_bin_falls_through_to_pgbin() {
        // FIX MINOR #2 regression guard: a set-but-empty HANDSHAKE_MANAGED_PG_BIN
        // (modeled as `None` by `nonempty_env`) must NOT shadow PGBIN; the chain
        // must fall through to the valid PGBIN directory. Exercises the same pure
        // helper `from_env` calls, with no global-env mutation (no race).
        let pgbin = PathBuf::from("some/pgbin/dir");
        let resolved = resolve_bin_dir(
            /* managed_pg_bin (empty) */ None,
            /* pgbin (valid)         */ Some(pgbin.clone()),
            /* bundled               */ Some(PathBuf::from("bundled/should/not/win")),
        );
        assert_eq!(
            resolved, pgbin,
            "empty MANAGED_PG_BIN must fall through to the valid PGBIN dir"
        );
    }

    #[test]
    fn managed_pg_bin_wins_over_pgbin_and_bundled() {
        // Precedence is unchanged: a valid MANAGED_PG_BIN beats PGBIN and bundled.
        let managed = PathBuf::from("managed/override/dir");
        let resolved = resolve_bin_dir(
            Some(managed.clone()),
            Some(PathBuf::from("pgbin/dir")),
            Some(PathBuf::from("bundled/dir")),
        );
        assert_eq!(resolved, managed, "MANAGED_PG_BIN must win when set");
    }

    #[test]
    fn pgbin_wins_over_bundled_when_managed_absent() {
        let pgbin = PathBuf::from("pgbin/dir");
        let resolved = resolve_bin_dir(
            None,
            Some(pgbin.clone()),
            Some(PathBuf::from("bundled/dir")),
        );
        assert_eq!(resolved, pgbin, "PGBIN must win over bundled when set");
    }

    #[test]
    fn bundled_used_when_no_env_candidates() {
        let bundled = PathBuf::from("bundled/dir");
        let resolved = resolve_bin_dir(None, None, Some(bundled.clone()));
        assert_eq!(
            resolved, bundled,
            "bundled dir must be used when neither env candidate is set"
        );
    }

    #[test]
    fn empty_when_no_candidates_defers_to_resolve_bin_path_fallthrough() {
        // No env candidates and no bundled dir -> empty PathBuf, which signals
        // `resolve_bin` to fall through to PGBIN / default install path / PATH.
        let resolved = resolve_bin_dir(None, None, None);
        assert_eq!(
            resolved,
            PathBuf::new(),
            "no candidates must yield an empty bin_dir (PATH fall-through)"
        );
        assert!(resolved.as_os_str().is_empty());
    }

    #[test]
    fn nonempty_env_returns_none_for_unset_and_empty() {
        // Use a uniquely-named key that no other test sets, to avoid races.
        // Unset -> None.
        let key = "HANDSHAKE_TEST_NONEMPTY_ENV_UNSET_XYZ";
        std::env::remove_var(key);
        assert!(nonempty_env(key).is_none(), "unset var must yield None");
    }
}
