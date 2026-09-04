use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::future::Future;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage, SurrealStorageConfig};
use handshake_core::storage::{NewWorkspace, Workspace, WriteContext};
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::types::{SurrealValue, Value as SurrealValueData};
use surrealdb::Surreal;
use uuid::Uuid;

pub const TEST_STORE_ROOT_ENV: &str = "HANDSHAKE_SURREAL_TEST_STORE_ROOT";
pub const TEST_STORE_STALE_AGE_MS_ENV: &str = "HANDSHAKE_SURREAL_TEST_STORE_STALE_AGE_MS";
pub const DEFAULT_STALE_AGE: Duration = Duration::ZERO;
// Liveness bound for embedded scope allocate/remove/close, not a proof gate. 10s was tuned on an
// idle machine; a fresh canonical bootstrap costs ~820s and concurrent proof stores saturate disk,
// so allocate/remove/close routinely exceeded 10s and failed tests for machine load rather than
// product behaviour.
pub const DEFAULT_EMBEDDED_SCOPE_TIMEOUT: Duration = Duration::from_secs(120);

const SCOPE_PREFIX: &str = "surreal-test-store-";
const OWNER_MARKER_SUFFIX: &str = ".owner";
const QUARANTINE_SUFFIX: &str = ".reclaiming";
const IDENTITY_HEX_LEN: usize = 32;
const SCOPE_NAME_LEN: usize = SCOPE_PREFIX.len() + IDENTITY_HEX_LEN + 1 + IDENTITY_HEX_LEN;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreIdentity {
    pub id: Uuid,
    pub token: Uuid,
}

impl StoreIdentity {
    fn generate() -> Self {
        Self {
            id: Uuid::now_v7(),
            token: Uuid::new_v4(),
        }
    }

    fn scope_name(&self) -> String {
        format!("{SCOPE_PREFIX}{}-{}", self.id.simple(), self.token.simple())
    }

    fn marker_name(&self) -> String {
        format!("{}{OWNER_MARKER_SUFFIX}", self.scope_name())
    }

    fn marker_body(&self) -> String {
        format!("{}\n{}\n", self.id.simple(), self.token.simple())
    }

    fn parse_scope_name(name: &str) -> Option<Self> {
        if name.len() != SCOPE_NAME_LEN || !name.starts_with(SCOPE_PREFIX) {
            return None;
        }
        let suffix = &name[SCOPE_PREFIX.len()..];
        let (id, token) = suffix.split_once('-')?;
        if !is_lower_hex(id) || !is_lower_hex(token) {
            return None;
        }
        Some(Self {
            id: Uuid::parse_str(id).ok()?,
            token: Uuid::parse_str(token).ok()?,
        })
    }
}

fn is_lower_hex(value: &str) -> bool {
    value.len() == IDENTITY_HEX_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Default)]
pub struct SweepReport {
    pub reclaimed: Vec<PathBuf>,
    pub reclaimed_bytes: u64,
    pub reclaimed_owner_markers: Vec<PathBuf>,
    pub skipped_live: Vec<PathBuf>,
    pub skipped_recent: Vec<PathBuf>,
    pub skipped_unproven: Vec<PathBuf>,
    pub rejected_unsafe: Vec<(PathBuf, String)>,
    pub errors: Vec<(PathBuf, String)>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StoreBacklogMeasurement {
    pub scope_count: usize,
    pub contained_data_bytes: u64,
}

pub struct IsolatedSurrealTestStore {
    root: PathBuf,
    scope_path: PathBuf,
    marker_path: PathBuf,
    identity: StoreIdentity,
    storage: Option<SurrealStorage>,
    // Field order is intentional: Rust drops fields in declaration order, so the
    // store closes before the zero-share ownership marker is released on unwind.
    owner_marker: Option<File>,
    startup_sweep: SweepReport,
}

impl IsolatedSurrealTestStore {
    pub async fn create() -> io::Result<Self> {
        let minimum_age = configured_stale_age()?;
        Self::create_in_with_policy(configured_test_store_root(), minimum_age).await
    }

    pub async fn create_in(root: impl AsRef<Path>) -> io::Result<Self> {
        Self::create_in_with_policy(root, DEFAULT_STALE_AGE).await
    }

    pub async fn create_in_with_policy(
        root: impl AsRef<Path>,
        minimum_age: Duration,
    ) -> io::Result<Self> {
        let root = prepare_root(root.as_ref())?;
        let startup_sweep = sweep_prepared_root(&root, minimum_age)?;
        let identity = StoreIdentity::generate();
        let scope_path = root.join(identity.scope_name());
        let marker_path = root.join(identity.marker_name());
        let mut owner_marker = create_held_owner_marker(&marker_path)?;
        if let Err(error) = owner_marker
            .write_all(identity.marker_body().as_bytes())
            .and_then(|()| owner_marker.sync_data())
        {
            drop(owner_marker);
            let _ = fs::remove_file(&marker_path);
            return Err(error);
        }
        if let Err(error) = fs::create_dir(&scope_path) {
            drop(owner_marker);
            let _ = fs::remove_file(&marker_path);
            return Err(error);
        }

        let config = match SurrealStorageConfig::for_data_dir(scope_path.join("data")) {
            Ok(config) => config,
            Err(error) => {
                let _ =
                    remove_owned_scope(&root, &scope_path, &marker_path, &identity, owner_marker);
                return Err(storage_error(error));
            }
        };
        let storage = match SurrealStorage::open(config).await {
            Ok(storage) => storage,
            Err(error) => {
                let _ =
                    remove_owned_scope(&root, &scope_path, &marker_path, &identity, owner_marker);
                return Err(storage_error(error));
            }
        };
        if let Err(bootstrap_error) = bootstrap_schema(&storage).await {
            if let Err(shutdown_error) = storage.shutdown().await {
                // The close barrier failed, so retain the zero-share marker until
                // process exit rather than exposing a potentially live engine.
                std::mem::forget(owner_marker);
                drop(storage);
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "schema bootstrap failed ({bootstrap_error}); fail-safe shutdown also failed ({shutdown_error})"
                    ),
                ));
            }
            drop(storage);
            let _ = remove_owned_scope(&root, &scope_path, &marker_path, &identity, owner_marker);
            return Err(storage_error(bootstrap_error));
        }

        Ok(Self {
            root,
            scope_path,
            marker_path,
            identity,
            storage: Some(storage),
            owner_marker: Some(owner_marker),
            startup_sweep,
        })
    }

    pub fn is_accepting_operations(&self) -> bool {
        self.storage
            .as_ref()
            .expect("isolated test store remains open until graceful shutdown")
            .is_accepting_operations()
    }

    pub async fn create_workspace_probe(&self, name: &str) -> io::Result<Workspace> {
        self.storage
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "isolated store is closed"))?
            .create_workspace(
                &WriteContext::system(Some("mt123-surreal-test-store".to_owned())),
                NewWorkspace {
                    name: name.to_owned(),
                },
            )
            .await
            .map_err(storage_error)
    }

    pub async fn get_workspace_probe(&self, record_id: &str) -> io::Result<Option<Workspace>> {
        self.storage
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "isolated store is closed"))?
            .get_workspace(record_id)
            .await
            .map_err(storage_error)
    }

    pub fn scope_path(&self) -> &Path {
        &self.scope_path
    }

    pub fn startup_sweep_report(&self) -> &SweepReport {
        &self.startup_sweep
    }

    pub fn owner_marker_path_for_proof(&self) -> &Path {
        &self.marker_path
    }

    async fn close_storage_barrier(&mut self) -> io::Result<()> {
        if let Some(storage) = self.storage.as_ref() {
            storage.shutdown().await.map_err(storage_error)?;
        }
        drop(self.storage.take());
        Ok(())
    }

    pub async fn shutdown_and_cleanup(mut self) -> io::Result<()> {
        self.close_storage_barrier().await?;
        let marker = self
            .owner_marker
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "owner marker is missing"))?;
        remove_owned_scope(
            &self.root,
            &self.scope_path,
            &self.marker_path,
            &self.identity,
            marker,
        )
        .map(|_| ())
    }

    pub async fn leave_closed_orphan_for_proof(mut self) -> io::Result<PathBuf> {
        self.close_storage_barrier().await?;
        drop(self.owner_marker.take());
        Ok(self.scope_path.clone())
    }

    pub async fn leave_interrupted_quarantine_for_proof(mut self) -> io::Result<PathBuf> {
        self.close_storage_barrier().await?;
        let quarantine_path = self.root.join(format!(
            "{}{}",
            self.identity.scope_name(),
            QUARANTINE_SUFFIX
        ));
        fs::rename(&self.scope_path, &quarantine_path)?;
        drop(self.owner_marker.take());
        Ok(quarantine_path)
    }

    pub async fn quarantine_while_owner_is_held_for_proof(&mut self) -> io::Result<PathBuf> {
        self.close_storage_barrier().await?;
        let quarantine_path = self.root.join(format!(
            "{}{}",
            self.identity.scope_name(),
            QUARANTINE_SUFFIX
        ));
        fs::rename(&self.scope_path, &quarantine_path)?;
        self.scope_path = quarantine_path.clone();
        Ok(quarantine_path)
    }

    pub fn release_quarantine_owner_for_proof(mut self) -> PathBuf {
        drop(self.owner_marker.take());
        self.scope_path.clone()
    }
}

impl Drop for IsolatedSurrealTestStore {
    fn drop(&mut self) {
        if self.storage.is_some() {
            // Plain drop, unwind, or cancellation cannot prove the asynchronous
            // close barrier completed. Retain the zero-share marker handle until
            // process exit so a sweeper cannot reclaim storage still closing.
            if let Some(marker) = self.owner_marker.take() {
                std::mem::forget(marker);
            }
        }
    }
}

const EMBEDDED_NAMESPACE_PREFIX: &str = "hs_test_ns_";
const EMBEDDED_DATABASE_PREFIX: &str = "hs_test_db_";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddedSurrealCleanupDiagnostics {
    pub namespace: String,
    pub database: String,
    pub store_path: PathBuf,
    pub timeout: Duration,
    pub database_absent: bool,
    pub namespace_absent_after_reopen: bool,
    pub elapsed: Duration,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddedSurrealCleanupAttempt {
    pub namespace: String,
    pub database: String,
    pub succeeded: bool,
    pub diagnostics: EmbeddedSurrealCleanupDiagnostics,
}

const FOREIGN_SURVIVAL_SENTINEL_TABLE: &str = "mt024_scope_sentinel";
const FOREIGN_SURVIVAL_SENTINEL_ID: &str = "foreign_survival";
const FOREIGN_SURVIVAL_SENTINEL_VALUE: &str = "survives-earlier-cleanup";

#[derive(Clone, Debug, Eq, PartialEq, SurrealValue)]
struct ForeignSurvivalSentinel {
    value: String,
}

/// One exact, allocator-owned namespace/database in a private embedded store.
///
/// Cleanup authority comes only from generated identifiers. Catalog listings
/// are verification inputs and are never used to select deletion targets.
pub struct EmbeddedSurrealTestScope {
    root: PathBuf,
    scope_path: PathBuf,
    marker_path: PathBuf,
    store_path: PathBuf,
    identity: StoreIdentity,
    namespace: String,
    database: String,
    client: Option<Surreal<Db>>,
    storage: Option<SurrealStorage>,
    owner_marker: Option<File>,
    timeout: Duration,
    storage_shutdown_timeout: Duration,
    successful_cleanup: Option<EmbeddedSurrealCleanupDiagnostics>,
    last_cleanup: Option<EmbeddedSurrealCleanupDiagnostics>,
}

impl EmbeddedSurrealCleanupDiagnostics {
    fn pending(scope: &EmbeddedSurrealTestScope) -> Self {
        Self {
            namespace: scope.namespace.clone(),
            database: scope.database.clone(),
            store_path: scope.store_path.clone(),
            timeout: scope.timeout,
            database_absent: false,
            namespace_absent_after_reopen: false,
            elapsed: Duration::ZERO,
            error: None,
        }
    }
}

impl EmbeddedSurrealTestScope {
    pub async fn create() -> io::Result<Self> {
        Self::create_in(configured_test_store_root()).await
    }

    pub async fn create_in(root: impl AsRef<Path>) -> io::Result<Self> {
        Self::create_in_with_timeout(root, DEFAULT_EMBEDDED_SCOPE_TIMEOUT).await
    }

    pub async fn create_in_with_timeout(
        root: impl AsRef<Path>,
        timeout: Duration,
    ) -> io::Result<Self> {
        if timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "embedded SurrealDB scope timeout must be greater than zero",
            ));
        }
        let root = prepare_root(root.as_ref())?;
        let _ = sweep_prepared_root(&root, DEFAULT_STALE_AGE)?;
        let identity = StoreIdentity::generate();
        let scope_path = root.join(identity.scope_name());
        let marker_path = root.join(identity.marker_name());
        let store_path = scope_path.join("embedded");
        let namespace = format!("{EMBEDDED_NAMESPACE_PREFIX}{}", identity.id.simple());
        let database = format!("{EMBEDDED_DATABASE_PREFIX}{}", identity.token.simple());
        validate_embedded_identifier(&namespace, EMBEDDED_NAMESPACE_PREFIX)?;
        validate_embedded_identifier(&database, EMBEDDED_DATABASE_PREFIX)?;

        let mut owner_marker = create_held_owner_marker(&marker_path)?;
        if let Err(error) = owner_marker
            .write_all(identity.marker_body().as_bytes())
            .and_then(|()| owner_marker.sync_data())
        {
            drop(owner_marker);
            let _ = fs::remove_file(&marker_path);
            return Err(error);
        }
        if let Err(error) = fs::create_dir(&scope_path) {
            drop(owner_marker);
            let _ = fs::remove_file(&marker_path);
            return Err(error);
        }

        let client = match open_embedded_root(&store_path, timeout, "allocate").await {
            Ok(client) => client,
            Err(error) => {
                let _ =
                    remove_owned_scope(&root, &scope_path, &marker_path, &identity, owner_marker);
                return Err(error);
            }
        };
        let allocation = async {
            checked_query(
                &client,
                format!("DEFINE NAMESPACE {namespace};"),
                timeout,
                "define namespace",
            )
            .await?;
            bounded_sdk(
                timeout,
                "select namespace",
                client.use_ns(namespace.as_str()),
            )
            .await?;
            checked_query(
                &client,
                format!("DEFINE DATABASE {database};"),
                timeout,
                "define database",
            )
            .await?;
            bounded_sdk(timeout, "select database", client.use_db(database.as_str())).await?;
            verify_context(&client, &namespace, &database, timeout).await
        }
        .await;
        if let Err(error) = allocation {
            let _ = close_embedded_client(client, &store_path, timeout, "failed allocation").await;
            let _ = remove_owned_scope(&root, &scope_path, &marker_path, &identity, owner_marker);
            return Err(error);
        }

        Ok(Self {
            root,
            scope_path,
            marker_path,
            store_path,
            identity,
            namespace,
            database,
            client: Some(client),
            storage: None,
            owner_marker: Some(owner_marker),
            timeout,
            storage_shutdown_timeout: timeout,
            successful_cleanup: None,
            last_cleanup: None,
        })
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn database(&self) -> &str {
        &self.database
    }

    pub fn store_path(&self) -> &Path {
        &self.store_path
    }

    pub async fn write_foreign_survival_sentinel(&self) -> io::Result<()> {
        let client = self.client.as_ref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "embedded test scope is closed")
        })?;
        let stored: Option<ForeignSurvivalSentinel> = bounded_sdk(
            self.timeout,
            "write fixed foreign-survival sentinel",
            client
                .upsert((
                    FOREIGN_SURVIVAL_SENTINEL_TABLE,
                    FOREIGN_SURVIVAL_SENTINEL_ID,
                ))
                .content(ForeignSurvivalSentinel {
                    value: FOREIGN_SURVIVAL_SENTINEL_VALUE.to_owned(),
                }),
        )
        .await?;
        if stored.as_ref().map(|row| row.value.as_str()) != Some(FOREIGN_SURVIVAL_SENTINEL_VALUE) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fixed foreign-survival sentinel was not stored",
            ));
        }
        Ok(())
    }

    pub async fn foreign_survival_sentinel_exists(&self) -> io::Result<bool> {
        let client = self.client.as_ref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "embedded test scope is closed")
        })?;
        let stored: Option<ForeignSurvivalSentinel> = bounded_sdk(
            self.timeout,
            "read fixed foreign-survival sentinel",
            client.select((
                FOREIGN_SURVIVAL_SENTINEL_TABLE,
                FOREIGN_SURVIVAL_SENTINEL_ID,
            )),
        )
        .await?;
        Ok(stored.as_ref().map(|row| row.value.as_str()) == Some(FOREIGN_SURVIVAL_SENTINEL_VALUE))
    }

    pub fn last_cleanup_diagnostics(&self) -> Option<&EmbeddedSurrealCleanupDiagnostics> {
        self.last_cleanup.as_ref()
    }

    pub fn set_storage_shutdown_timeout_for_proof(&mut self, timeout: Duration) -> io::Result<()> {
        if timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "embedded SurrealDB scope timeout must be greater than zero",
            ));
        }
        if self.storage.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "storage shutdown timeout cannot change while production storage is active",
            ));
        }
        self.storage_shutdown_timeout = timeout;
        Ok(())
    }

    /// Activates the production storage wrapper on this allocator's exact scope.
    pub async fn activate_storage(&mut self) -> io::Result<SurrealStorage> {
        if let Some(storage) = &self.storage {
            return Ok(storage.clone());
        }
        self.close_direct_client().await?;
        let config = SurrealStorageConfig::for_scoped_store(
            &self.store_path,
            self.namespace.clone(),
            self.database.clone(),
        )
        .and_then(|config| config.with_shutdown_wait_timeout(self.storage_shutdown_timeout))
        .map_err(storage_error)?;
        let storage = SurrealStorage::open(config).await.map_err(storage_error)?;
        self.storage = Some(storage.clone());
        Ok(storage)
    }

    /// Closes the retained production wrapper and every returned shared clone.
    /// An in-flight escaped operation produces a bounded, observable error.
    pub async fn shutdown_storage_for_reopen(&mut self) -> io::Result<()> {
        if let Some(storage) = self.storage.as_ref() {
            storage.shutdown().await.map_err(storage_error)?;
        }
        drop(self.storage.take());
        Ok(())
    }

    async fn close_direct_client(&mut self) -> io::Result<()> {
        let Some(client) = self.client.take() else {
            return Ok(());
        };
        close_embedded_client(client, &self.store_path, self.timeout, "close for reopen").await
    }

    pub async fn close_for_reopen(&mut self) -> io::Result<()> {
        self.shutdown_storage_for_reopen().await?;
        self.close_direct_client().await
    }

    pub async fn reopen(&mut self) -> io::Result<()> {
        if self.successful_cleanup.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "cleaned embedded test scope cannot be reopened",
            ));
        }
        if self.client.is_some() {
            return Ok(());
        }
        let client = open_embedded_root(&self.store_path, self.timeout, "reopen").await?;
        let selection =
            select_existing_scope(&client, &self.namespace, &self.database, self.timeout).await;
        if let Err(error) = selection {
            let _ = close_embedded_client(client, &self.store_path, self.timeout, "failed reopen")
                .await;
            return Err(error);
        }
        self.client = Some(client);
        Ok(())
    }

    pub async fn cleanup(&mut self) -> io::Result<EmbeddedSurrealCleanupDiagnostics> {
        if let Some(receipt) = &self.successful_cleanup {
            return Ok(receipt.clone());
        }
        let started = Instant::now();
        let mut diagnostics = EmbeddedSurrealCleanupDiagnostics::pending(self);
        let result = self.cleanup_once(&mut diagnostics).await;
        diagnostics.elapsed = started.elapsed();
        if let Err(error) = &result {
            diagnostics.error = Some(error.to_string());
        }
        self.last_cleanup = Some(diagnostics.clone());
        if result.is_ok() {
            self.successful_cleanup = Some(diagnostics.clone());
        }
        result.map(|()| diagnostics)
    }

    async fn cleanup_once(
        &mut self,
        diagnostics: &mut EmbeddedSurrealCleanupDiagnostics,
    ) -> io::Result<()> {
        self.close_for_reopen().await?;
        let client = open_embedded_root(&self.store_path, self.timeout, "cleanup reopen").await?;
        let namespaces_before = catalog_names(
            &client,
            "INFO FOR ROOT;",
            "namespaces",
            self.timeout,
            "read root catalog before cleanup",
        )
        .await?;
        if namespaces_before.contains(&self.namespace) {
            bounded_sdk(
                self.timeout,
                "select exact cleanup namespace",
                client.use_ns(self.namespace.as_str()),
            )
            .await?;
            checked_query(
                &client,
                format!("REMOVE DATABASE IF EXISTS {};", self.database),
                self.timeout,
                "remove exact database",
            )
            .await?;
            let databases_after = catalog_names(
                &client,
                "INFO FOR NS;",
                "databases",
                self.timeout,
                "verify database absence",
            )
            .await?;
            diagnostics.database_absent = !databases_after.contains(&self.database);
            if !diagnostics.database_absent {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("database {} remained after exact cleanup", self.database),
                ));
            }
            checked_query(
                &client,
                format!("REMOVE NAMESPACE IF EXISTS {};", self.namespace),
                self.timeout,
                "remove exact namespace",
            )
            .await?;
        } else {
            diagnostics.database_absent = true;
        }
        close_embedded_client(client, &self.store_path, self.timeout, "cleanup mutation").await?;

        let verifier =
            open_embedded_root(&self.store_path, self.timeout, "absence verifier").await?;
        let namespaces_after = catalog_names(
            &verifier,
            "INFO FOR ROOT;",
            "namespaces",
            self.timeout,
            "independent root reread",
        )
        .await?;
        diagnostics.namespace_absent_after_reopen = !namespaces_after.contains(&self.namespace);
        if !diagnostics.namespace_absent_after_reopen {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("namespace {} remained after exact cleanup", self.namespace),
            ));
        }
        close_embedded_client(verifier, &self.store_path, self.timeout, "absence verifier").await?;

        let marker = self.owner_marker.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "embedded scope owner marker is missing",
            )
        })?;
        remove_owned_scope(
            &self.root,
            &self.scope_path,
            &self.marker_path,
            &self.identity,
            marker,
        )?;
        Ok(())
    }
}

impl Drop for EmbeddedSurrealTestScope {
    fn drop(&mut self) {
        if self.successful_cleanup.is_none() {
            let _ = writeln!(
                io::stderr().lock(),
                "embedded_surreal_cleanup_pending namespace={} database={} store={} diagnostics={:?}",
                self.namespace,
                self.database,
                self.store_path.display(),
                self.last_cleanup
            );
            if let Some(marker) = self.owner_marker.take() {
                std::mem::forget(marker);
            }
        }
    }
}

/// Gives every exact scope its own bounded cleanup attempt and continues after errors.
pub async fn cleanup_embedded_surreal_scopes(
    scopes: &mut [EmbeddedSurrealTestScope],
) -> Vec<EmbeddedSurrealCleanupAttempt> {
    let mut attempts = Vec::with_capacity(scopes.len());
    for scope in scopes {
        let namespace = scope.namespace.clone();
        let database = scope.database.clone();
        let result = scope.cleanup().await;
        let succeeded = result.is_ok();
        let diagnostics = result.unwrap_or_else(|error| {
            scope.last_cleanup.clone().unwrap_or_else(|| {
                let mut diagnostics = EmbeddedSurrealCleanupDiagnostics::pending(scope);
                diagnostics.error = Some(error.to_string());
                diagnostics
            })
        });
        attempts.push(EmbeddedSurrealCleanupAttempt {
            namespace,
            database,
            succeeded,
            diagnostics,
        });
    }
    attempts
}

fn validate_embedded_identifier(value: &str, prefix: &str) -> io::Result<()> {
    if value.strip_prefix(prefix).is_some_and(is_lower_hex) {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "embedded SurrealDB identifier is not allocator-generated",
    ))
}

async fn bounded_sdk<T, E, F>(timeout: Duration, label: &str, operation: F) -> io::Result<T>
where
    E: std::fmt::Display,
    F: std::future::IntoFuture<Output = Result<T, E>>,
{
    match tokio::time::timeout(timeout, std::future::IntoFuture::into_future(operation)).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("embedded SurrealDB {label} failed: {error}"),
        )),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "embedded SurrealDB {label} exceeded its independent {} ms timeout",
                timeout.as_millis()
            ),
        )),
    }
}

async fn open_embedded_root(
    store_path: &Path,
    timeout: Duration,
    label: &str,
) -> io::Result<Surreal<Db>> {
    fs::create_dir_all(store_path)?;
    bounded_sdk(timeout, label, Surreal::new::<RocksDb>(store_path)).await
}

async fn checked_query(
    client: &Surreal<Db>,
    statement: String,
    timeout: Duration,
    label: &str,
) -> io::Result<()> {
    let response = bounded_sdk(timeout, label, client.query(statement)).await?;
    response.check().map_err(storage_error)?;
    Ok(())
}

async fn catalog_names(
    client: &Surreal<Db>,
    statement: &str,
    field: &str,
    timeout: Duration,
    label: &str,
) -> io::Result<BTreeSet<String>> {
    let response = bounded_sdk(timeout, label, client.query(statement)).await?;
    let mut response = response.check().map_err(storage_error)?;
    let info: SurrealValueData = response.take(0).map_err(storage_error)?;
    let SurrealValueData::Object(info) = info else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("embedded SurrealDB {label} returned a non-object catalog"),
        ));
    };
    let Some(SurrealValueData::Object(entries)) = info.get(field) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("embedded SurrealDB {label} omitted object field {field}"),
        ));
    };
    Ok(entries.keys().cloned().collect())
}

async fn select_existing_scope(
    client: &Surreal<Db>,
    namespace: &str,
    database: &str,
    timeout: Duration,
) -> io::Result<()> {
    let namespaces = catalog_names(
        client,
        "INFO FOR ROOT;",
        "namespaces",
        timeout,
        "verify namespace before reopen",
    )
    .await?;
    if !namespaces.contains(namespace) {
        return Err(missing_catalog_entry("namespace", namespace));
    }
    bounded_sdk(
        timeout,
        "select reopened namespace",
        client.use_ns(namespace),
    )
    .await?;
    let databases = catalog_names(
        client,
        "INFO FOR NS;",
        "databases",
        timeout,
        "verify database before reopen",
    )
    .await?;
    if !databases.contains(database) {
        return Err(missing_catalog_entry("database", database));
    }
    bounded_sdk(timeout, "select reopened database", client.use_db(database)).await?;
    verify_context(client, namespace, database, timeout).await
}

async fn verify_context(
    client: &Surreal<Db>,
    namespace: &str,
    database: &str,
    timeout: Duration,
) -> io::Result<()> {
    let response = bounded_sdk(
        timeout,
        "verify selected context",
        client.query("RETURN session::ns(); RETURN session::db();"),
    )
    .await?;
    let mut response = response.check().map_err(storage_error)?;
    let actual_namespace: Option<String> = response.take(0).map_err(storage_error)?;
    let actual_database: Option<String> = response.take(1).map_err(storage_error)?;
    if actual_namespace.as_deref() != Some(namespace)
        || actual_database.as_deref() != Some(database)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "embedded context mismatch: expected {namespace}/{database}, observed {}/{}",
                actual_namespace.as_deref().unwrap_or("<none>"),
                actual_database.as_deref().unwrap_or("<none>")
            ),
        ));
    }
    Ok(())
}

async fn close_embedded_client(
    client: Surreal<Db>,
    store_path: &Path,
    timeout: Duration,
    label: &str,
) -> io::Result<()> {
    checked_query(
        &client,
        "RETURN true;".to_owned(),
        timeout,
        &format!("{label} barrier"),
    )
    .await?;
    drop(client);
    wait_for_embedded_release(store_path, timeout, label).await
}

#[cfg(windows)]
async fn wait_for_embedded_release(
    store_path: &Path,
    timeout: Duration,
    label: &str,
) -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;

    let started = Instant::now();
    let lock_path = store_path.join("LOCK");
    let mut backoff = Duration::from_millis(5);
    let mut last_error = None;
    loop {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&lock_path)
        {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => last_error = Some(error.to_string()),
        }
        if started.elapsed() >= timeout {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "embedded SurrealDB {label} release exceeded {} ms at {}: {}",
                    timeout.as_millis(),
                    lock_path.display(),
                    last_error.as_deref().unwrap_or("unknown lock error")
                ),
            ));
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_millis(250));
    }
}

#[cfg(not(windows))]
async fn wait_for_embedded_release(
    _store_path: &Path,
    _timeout: Duration,
    _label: &str,
) -> io::Result<()> {
    tokio::task::yield_now().await;
    Ok(())
}

fn missing_catalog_entry(kind: &str, name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("allocated embedded SurrealDB {kind} {name} is absent"),
    )
}

fn storage_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error.to_string())
}

pub fn configured_test_store_root() -> PathBuf {
    std::env::var_os(TEST_STORE_ROOT_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("handshake-surreal-test-stores"))
}

fn configured_stale_age() -> io::Result<Duration> {
    let Some(value) = std::env::var_os(TEST_STORE_STALE_AGE_MS_ENV) else {
        return Ok(DEFAULT_STALE_AGE);
    };
    let value = value.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{TEST_STORE_STALE_AGE_MS_ENV} must be valid Unicode"),
        )
    })?;
    let milliseconds = value.parse::<u64>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid {TEST_STORE_STALE_AGE_MS_ENV}: {error}"),
        )
    })?;
    Ok(Duration::from_millis(milliseconds))
}

pub fn remaining_leak_modes() -> &'static [&'static str] {
    &[
        "a scope younger than the configured stale age remains until a later normal creation",
        "an ownership marker that remains locked is treated as live and is never reclaimed",
        "permission, metadata, clock, containment, identity, or reparse ambiguity is skipped fail-closed",
        "platforms without Windows zero-share ownership proof skip candidates as unproven",
        "an interrupted quarantine remains until the next normal creation resumes removal",
    ]
}

pub fn sweep_stale_orphans(
    root: impl AsRef<Path>,
    minimum_age: Duration,
) -> io::Result<SweepReport> {
    let root = prepare_root(root.as_ref())?;
    sweep_prepared_root(&root, minimum_age)
}

fn sweep_prepared_root(root: &Path, minimum_age: Duration) -> io::Result<SweepReport> {
    let mut report = SweepReport::default();

    for entry in fs::read_dir(root)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report.errors.push((root.to_path_buf(), error.to_string()));
                continue;
            }
        };
        let scope_path = entry.path();
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            report
                .rejected_unsafe
                .push((scope_path, "candidate name is not valid Unicode".to_owned()));
            continue;
        };
        if let Some(scope_name) = name.strip_suffix(QUARANTINE_SUFFIX) {
            let Some(identity) = StoreIdentity::parse_scope_name(scope_name) else {
                continue;
            };
            reclaim_quarantined_scope(root, &scope_path, &identity, &mut report);
            continue;
        }
        let Some(identity) = StoreIdentity::parse_scope_name(&name) else {
            continue;
        };

        if let Err(reason) = validate_scope(root, &scope_path, &identity) {
            report.rejected_unsafe.push((scope_path, reason));
            continue;
        }

        let age = match fs::metadata(&scope_path)
            .and_then(|metadata| metadata.modified())
            .and_then(|modified| modified.elapsed().map_err(storage_error))
        {
            Ok(age) => age,
            Err(error) => {
                report.errors.push((scope_path, error.to_string()));
                continue;
            }
        };
        if age < minimum_age {
            report.skipped_recent.push(scope_path);
            continue;
        }

        let marker_path = root.join(identity.marker_name());
        match probe_owner(root, &marker_path, &identity) {
            OwnerProbe::Live => report.skipped_live.push(scope_path),
            OwnerProbe::Unproven => report.skipped_unproven.push(scope_path),
            OwnerProbe::Unsafe(reason) => report.rejected_unsafe.push((scope_path, reason)),
            OwnerProbe::Stale(marker) => {
                match remove_owned_scope(root, &scope_path, &marker_path, &identity, marker) {
                    Ok(bytes) => {
                        report.reclaimed_bytes = report.reclaimed_bytes.saturating_add(bytes);
                        report.reclaimed.push(scope_path);
                    }
                    Err(error) => report.errors.push((scope_path, error.to_string())),
                }
            }
        }
    }

    reclaim_unpaired_owner_markers(root, &mut report)?;

    Ok(report)
}

fn reclaim_unpaired_owner_markers(root: &Path, report: &mut SweepReport) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let marker_path = entry.path();
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        let Some(scope_name) = name.strip_suffix(OWNER_MARKER_SUFFIX) else {
            continue;
        };
        let Some(identity) = StoreIdentity::parse_scope_name(scope_name) else {
            continue;
        };
        let paired_paths = [
            root.join(scope_name),
            root.join(format!("{scope_name}{QUARANTINE_SUFFIX}")),
        ];
        let mut has_pair = false;
        let mut pair_check_failed = false;
        for paired_path in paired_paths {
            match fs::symlink_metadata(&paired_path) {
                Ok(_) => {
                    has_pair = true;
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    report.errors.push((paired_path, error.to_string()));
                    pair_check_failed = true;
                    break;
                }
            }
        }
        if has_pair || pair_check_failed {
            continue;
        }

        match probe_unpaired_owner(root, &marker_path, &identity) {
            OwnerProbe::Live => report.skipped_live.push(marker_path),
            OwnerProbe::Unproven => report.skipped_unproven.push(marker_path),
            OwnerProbe::Unsafe(reason) => report.rejected_unsafe.push((marker_path, reason)),
            OwnerProbe::Stale(marker) => {
                drop(marker);
                match fs::remove_file(&marker_path) {
                    Ok(()) => report.reclaimed_owner_markers.push(marker_path),
                    Err(error) => report.errors.push((marker_path, error.to_string())),
                }
            }
        }
    }
    Ok(())
}

fn reclaim_quarantined_scope(
    root: &Path,
    quarantine_path: &Path,
    identity: &StoreIdentity,
    report: &mut SweepReport,
) {
    if quarantine_path.parent() != Some(root)
        || quarantine_path.file_name().and_then(|name| name.to_str())
            != Some(format!("{}{}", identity.scope_name(), QUARANTINE_SUFFIX).as_str())
    {
        report.rejected_unsafe.push((
            quarantine_path.to_path_buf(),
            "quarantine path is not the exact root-level identity path".to_owned(),
        ));
        return;
    }
    let metadata = match fs::symlink_metadata(quarantine_path) {
        Ok(metadata) => metadata,
        Err(error) => {
            report
                .errors
                .push((quarantine_path.to_path_buf(), error.to_string()));
            return;
        }
    };
    if !metadata.is_dir() || is_symlink_or_reparse(&metadata) {
        report.rejected_unsafe.push((
            quarantine_path.to_path_buf(),
            "quarantine candidate is not a real directory".to_owned(),
        ));
        return;
    }

    let marker_path = root.join(identity.marker_name());
    match probe_owner(root, &marker_path, identity) {
        OwnerProbe::Live => report.skipped_live.push(quarantine_path.to_path_buf()),
        OwnerProbe::Unproven => report.skipped_unproven.push(quarantine_path.to_path_buf()),
        OwnerProbe::Unsafe(reason) => report
            .rejected_unsafe
            .push((quarantine_path.to_path_buf(), reason)),
        OwnerProbe::Stale(marker) => {
            // Byte accounting is observational only. Deletion safety comes from
            // remove_dir_all's handle-relative Windows traversal, not this walk.
            let bytes = inspect_contained_tree(quarantine_path)
                .map(|tree| tree.contained_bytes)
                .unwrap_or(0);
            match fs::remove_dir_all(quarantine_path) {
                Ok(()) => {
                    drop(marker);
                    match fs::remove_file(&marker_path) {
                        Ok(()) => {
                            report.reclaimed_bytes = report.reclaimed_bytes.saturating_add(bytes);
                            report.reclaimed.push(quarantine_path.to_path_buf());
                        }
                        Err(error) => report.errors.push((marker_path, error.to_string())),
                    }
                }
                Err(error) => report
                    .errors
                    .push((quarantine_path.to_path_buf(), error.to_string())),
            }
        }
    }
}

fn require_matching_owner_marker(root: &Path, identity: &StoreIdentity) -> io::Result<()> {
    let marker_path = root.join(identity.marker_name());
    match probe_owner(root, &marker_path, identity) {
        OwnerProbe::Live | OwnerProbe::Stale(_) => Ok(()),
        OwnerProbe::Unproven => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "ownership marker cannot be proven on this platform",
        )),
        OwnerProbe::Unsafe(reason) => Err(io::Error::new(io::ErrorKind::PermissionDenied, reason)),
    }
}

pub fn measure_owned_scopes(root: impl AsRef<Path>) -> io::Result<StoreBacklogMeasurement> {
    let root = prepare_root(root.as_ref())?;
    let mut measurement = StoreBacklogMeasurement::default();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        let Some(identity) = StoreIdentity::parse_scope_name(&name) else {
            continue;
        };
        let scope_path = entry.path();
        validate_scope(&root, &scope_path, &identity)
            .map_err(|reason| io::Error::new(io::ErrorKind::PermissionDenied, reason))?;
        require_matching_owner_marker(&root, &identity)?;
        let tree = inspect_contained_tree(&scope_path)
            .map_err(|reason| io::Error::new(io::ErrorKind::PermissionDenied, reason))?;
        measurement.scope_count += 1;
        measurement.contained_data_bytes = measurement
            .contained_data_bytes
            .saturating_add(tree.contained_bytes);
    }
    Ok(measurement)
}

fn prepare_root(root: &Path) -> io::Result<PathBuf> {
    if root.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "test store root must not be empty",
        ));
    }
    fs::create_dir_all(root)?;
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || is_symlink_or_reparse(&metadata) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "test store root must be a real directory without a reparse boundary",
        ));
    }
    dunce::canonicalize(root)
}

fn validate_scope(root: &Path, scope_path: &Path, identity: &StoreIdentity) -> Result<(), String> {
    if scope_path.file_name().and_then(|name| name.to_str()) != Some(identity.scope_name().as_str())
    {
        return Err("scope name does not match its parsed identity".to_owned());
    }
    if scope_path.parent() != Some(root) {
        return Err("scope is not an immediate child of the configured root".to_owned());
    }
    let metadata = fs::symlink_metadata(scope_path).map_err(|error| error.to_string())?;
    if !metadata.is_dir() || is_symlink_or_reparse(&metadata) {
        return Err("scope is not a real directory or crosses a reparse boundary".to_owned());
    }
    let canonical_scope = dunce::canonicalize(scope_path).map_err(|error| error.to_string())?;
    if canonical_scope.parent() != Some(root) {
        return Err("canonical scope escapes the configured root".to_owned());
    }
    Ok(())
}

struct ContainedTree {
    contained_bytes: u64,
}

fn inspect_contained_tree(scope_path: &Path) -> Result<ContainedTree, String> {
    let canonical_scope = dunce::canonicalize(scope_path).map_err(|error| error.to_string())?;
    let mut tree = ContainedTree { contained_bytes: 0 };
    inspect_directory(&canonical_scope, &canonical_scope, &mut tree)?;
    Ok(tree)
}

fn inspect_directory(
    canonical_scope: &Path,
    directory: &Path,
    tree: &mut ContainedTree,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if is_symlink_or_reparse(&metadata) {
            return Err(format!(
                "nested reparse or symlink boundary rejected: {}",
                path.display()
            ));
        }
        if !path.starts_with(canonical_scope) || path == canonical_scope {
            return Err(format!(
                "nested path escapes its owned scope: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            inspect_directory(canonical_scope, &path, tree)?;
        } else if metadata.is_file() {
            tree.contained_bytes = tree.contained_bytes.saturating_add(metadata.len());
        } else {
            return Err(format!(
                "unsupported nested filesystem entry: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn remove_owned_scope(
    root: &Path,
    scope_path: &Path,
    marker_path: &Path,
    identity: &StoreIdentity,
    mut marker: File,
) -> io::Result<u64> {
    validate_scope(root, scope_path, identity)
        .map_err(|reason| io::Error::new(io::ErrorKind::PermissionDenied, reason))?;
    validate_marker_path(root, marker_path, identity)
        .map_err(|reason| io::Error::new(io::ErrorKind::PermissionDenied, reason))?;
    validate_open_marker(&mut marker, identity)
        .map_err(|reason| io::Error::new(io::ErrorKind::PermissionDenied, reason))?;
    let tree = inspect_contained_tree(scope_path)
        .map_err(|reason| io::Error::new(io::ErrorKind::PermissionDenied, reason))?;
    let quarantine_path = root.join(format!("{}{}", identity.scope_name(), QUARANTINE_SUFFIX));
    fs::rename(scope_path, &quarantine_path)?;

    // Rust 1.97's Windows implementation performs handle-relative traversal and
    // does not follow symlinks, including under concurrent replacement races.
    // The same-root rename first removes the candidate from the active namespace;
    // an interrupted deletion remains discoverable as a quarantine candidate.
    fs::remove_dir_all(&quarantine_path)?;

    drop(marker);
    fs::remove_file(marker_path)?;
    Ok(tree.contained_bytes)
}

enum OwnerProbe {
    Live,
    Stale(File),
    Unproven,
    Unsafe(String),
}

#[cfg(windows)]
fn probe_unpaired_owner(root: &Path, marker_path: &Path, identity: &StoreIdentity) -> OwnerProbe {
    use std::os::windows::fs::OpenOptionsExt;

    if let Err(reason) = validate_marker_path(root, marker_path, identity) {
        return OwnerProbe::Unsafe(reason);
    }
    match OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(marker_path)
    {
        Ok(marker) => OwnerProbe::Stale(marker),
        Err(error) if matches!(error.raw_os_error(), Some(32) | Some(33)) => OwnerProbe::Live,
        Err(error) => OwnerProbe::Unsafe(error.to_string()),
    }
}

#[cfg(not(windows))]
fn probe_unpaired_owner(
    _root: &Path,
    _marker_path: &Path,
    _identity: &StoreIdentity,
) -> OwnerProbe {
    OwnerProbe::Unproven
}

#[cfg(windows)]
fn create_held_owner_marker(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(path)
}

#[cfg(not(windows))]
fn create_held_owner_marker(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
}

#[cfg(windows)]
fn probe_owner(root: &Path, marker_path: &Path, identity: &StoreIdentity) -> OwnerProbe {
    use std::os::windows::fs::OpenOptionsExt;

    if let Err(reason) = validate_marker_path(root, marker_path, identity) {
        return OwnerProbe::Unsafe(reason);
    }
    let mut marker = match OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(marker_path)
    {
        Ok(marker) => marker,
        Err(error) if matches!(error.raw_os_error(), Some(32) | Some(33)) => {
            return OwnerProbe::Live;
        }
        Err(error) => return OwnerProbe::Unsafe(error.to_string()),
    };
    match validate_open_marker(&mut marker, identity) {
        Ok(()) => OwnerProbe::Stale(marker),
        Err(reason) => OwnerProbe::Unsafe(reason),
    }
}

#[cfg(not(windows))]
fn probe_owner(_root: &Path, _marker_path: &Path, _identity: &StoreIdentity) -> OwnerProbe {
    OwnerProbe::Unproven
}

fn validate_marker_path(
    root: &Path,
    marker_path: &Path,
    identity: &StoreIdentity,
) -> Result<(), String> {
    if marker_path.parent() != Some(root)
        || marker_path.file_name().and_then(|name| name.to_str())
            != Some(identity.marker_name().as_str())
    {
        return Err("ownership marker is not the exact root-level identity sidecar".to_owned());
    }
    let path_metadata = fs::symlink_metadata(marker_path).map_err(|error| error.to_string())?;
    if !path_metadata.is_file() || is_symlink_or_reparse(&path_metadata) {
        return Err("ownership marker is not a real file or crosses a reparse boundary".to_owned());
    }
    Ok(())
}

fn validate_open_marker(marker: &mut File, identity: &StoreIdentity) -> Result<(), String> {
    if !marker
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("opened ownership marker is not a file".to_owned());
    }
    marker
        .seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let mut body = String::new();
    marker
        .read_to_string(&mut body)
        .map_err(|error| error.to_string())?;
    if body != identity.marker_body() {
        return Err("ownership marker identity does not match the scope name".to_owned());
    }
    Ok(())
}

fn is_symlink_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    is_reparse(metadata)
}

#[cfg(windows)]
fn is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_scope_identity_round_trips() {
        let identity = StoreIdentity::generate();
        assert_eq!(
            StoreIdentity::parse_scope_name(&identity.scope_name()),
            Some(identity)
        );
        assert!(StoreIdentity::parse_scope_name("surreal-test-store-not-an-identity").is_none());
        assert!(StoreIdentity::parse_scope_name(
            "surreal-test-store-0123456789ABCDEF0123456789ABCDEF-0123456789abcdef0123456789abcdef"
        )
        .is_none());
    }

    #[test]
    #[cfg(windows)]
    fn partial_unpaired_marker_is_reclaimed_without_a_scope() {
        let root = tempfile::tempdir().expect("create marker recovery root");
        let root = prepare_root(root.path()).expect("prepare marker recovery root");
        let identity = StoreIdentity::generate();
        let marker_path = root.join(identity.marker_name());
        let mut marker = create_held_owner_marker(&marker_path).expect("create partial marker");
        marker.write_all(b"partial").expect("write partial marker");
        marker.sync_data().expect("sync partial marker");
        drop(marker);

        let report = sweep_prepared_root(&root, Duration::ZERO).expect("recover partial marker");
        assert_eq!(report.reclaimed_owner_markers, vec![marker_path.clone()]);
        assert!(!marker_path.exists());
    }
}
