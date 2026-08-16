use std::{
    env,
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};

use surrealdb::{
    engine::local::{Db, RocksDb},
    Surreal,
};
use thiserror::Error;
use tokio::sync::{watch, Mutex, RwLock};

use super::{DefaultStorageGuard, StorageGuard};

mod blocks;
mod database;
mod documents;
mod schema;
#[cfg(feature = "surreal-test-support")]
mod test_inspector;
mod workspaces;

pub use database::SurrealDatabase;
pub use schema::{
    bootstrap_schema, SchemaBootstrapReport, EXPECTED_SCHEMA_INFO_SHA256,
    GENERATED_SURREALQL_SHA256, SCHEMA_REVISION, SCHEMA_VERSION, SOURCE_FORWARD_MIGRATION_COUNT,
    SOURCE_FORWARD_WAVE_MANIFEST_SHA256,
};
#[cfg(feature = "surreal-test-support")]
pub use test_inspector::{
    FieldCatalog, FieldSelector, IndexCatalog, ProjectedRow, RecordIdentity, ReferenceCatalog,
    RowFilter, ScalarValue, SchemaCatalogSnapshot, SurrealTestInspector, SurrealTestInspectorError,
    TableCatalog, TableSelector,
};

pub const HANDSHAKE_DATA_DIR_ENV: &str = "HANDSHAKE_DATA_DIR";
pub const DEFAULT_NAMESPACE: &str = "handshake";
pub const DEFAULT_DATABASE: &str = "primary";
pub const DEFAULT_STORE_DIRECTORY: &str = "handshake-surreal";
pub const DEFAULT_SHUTDOWN_WAIT: Duration = Duration::from_secs(30);

pub(crate) type SurrealClient = Surreal<Db>;

const LIFECYCLE_OPEN: u8 = 0;
const LIFECYCLE_CLOSING: u8 = 1;
const LIFECYCLE_CLOSED: u8 = 2;

#[derive(Debug, Error)]
pub enum SurrealStorageError {
    #[error("failed to prepare embedded database path {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("embedded database error: {0}")]
    Database(#[from] surrealdb::Error),
    #[error("embedded database is closed")]
    Closed,
    #[error("no platform-local application data directory is available")]
    MissingApplicationDataDirectory,
    #[error("embedded database data directory must not be empty")]
    EmptyDataDirectory,
    #[error("embedded database path {path} is incompatible with the SurrealDB 3.2 endpoint parser: {reason}")]
    IncompatibleEndpointPath { path: PathBuf, reason: &'static str },
    #[error(
        "embedded database context mismatch: expected namespace/database {expected_namespace}/{expected_database}, observed {actual_namespace}/{actual_database}"
    )]
    ContextMismatch {
        expected_namespace: String,
        expected_database: String,
        actual_namespace: String,
        actual_database: String,
    },
    #[error("shutdown cannot be called from inside an embedded database operation")]
    ReentrantShutdown,
    #[error("embedded database shutdown failed: {0}")]
    Shutdown(Arc<str>),
    #[error(
        "embedded database shutdown is still draining operations after {waited_ms} ms; closure continues in the background"
    )]
    ShutdownStillInProgress { waited_ms: u128 },
    #[error("embedded database shutdown wait timeout must be greater than zero")]
    InvalidShutdownWaitTimeout,
    #[error("embedded workspace record has an invalid shape: {reason}")]
    InvalidWorkspaceRecord { reason: &'static str },
    #[error("embedded document record has an invalid shape: {reason}")]
    InvalidDocumentRecord { reason: &'static str },
    #[error("embedded block record has an invalid shape: {reason}")]
    InvalidBlockRecord { reason: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurrealStorageConfig {
    path: PathBuf,
    namespace: String,
    database: String,
    shutdown_wait: Duration,
}

impl SurrealStorageConfig {
    pub fn from_env() -> Result<Self, SurrealStorageError> {
        let data_dir = match env::var_os(HANDSHAKE_DATA_DIR_ENV) {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            Some(_) => return Err(SurrealStorageError::EmptyDataDirectory),
            None => dirs::data_local_dir()
                .map(|path| path.join("handshake"))
                .ok_or(SurrealStorageError::MissingApplicationDataDirectory)?,
        };
        Self::for_data_dir(data_dir)
    }

    pub fn for_data_dir(data_dir: impl AsRef<Path>) -> Result<Self, SurrealStorageError> {
        let data_dir = data_dir.as_ref();
        if data_dir.as_os_str().is_empty() {
            return Err(SurrealStorageError::EmptyDataDirectory);
        }
        Self::for_store_path(data_dir.join(DEFAULT_STORE_DIRECTORY))
    }

    #[cfg(test)]
    pub fn with_path(path: impl Into<PathBuf>) -> Result<Self, SurrealStorageError> {
        Self::for_store_path(path.into())
    }

    fn for_store_path(path: impl Into<PathBuf>) -> Result<Self, SurrealStorageError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(SurrealStorageError::EmptyDataDirectory);
        }
        Ok(Self {
            path: absolute_path(path)?,
            namespace: DEFAULT_NAMESPACE.to_owned(),
            database: DEFAULT_DATABASE.to_owned(),
            shutdown_wait: DEFAULT_SHUTDOWN_WAIT,
        })
    }

    /// Sets how long each caller waits for the shared background close.
    ///
    /// The close continues safely if a caller times out. Zero is rejected
    /// because it would make every shutdown call observationally fail.
    pub fn with_shutdown_wait_timeout(
        mut self,
        timeout: Duration,
    ) -> Result<Self, SurrealStorageError> {
        if timeout.is_zero() {
            return Err(SurrealStorageError::InvalidShutdownWaitTimeout);
        }
        self.shutdown_wait = timeout;
        Ok(self)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn database(&self) -> &str {
        &self.database
    }
}

#[derive(Clone)]
pub struct SurrealStorage {
    inner: Arc<SurrealStorageInner>,
}

pub type SurrealOperation<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, SurrealStorageError>> + Send + 'a>>;

tokio::task_local! {
    static INSIDE_SURREAL_OPERATION: ();
}

/// A sealed, lease-bound view for ordinary typed data operations.
///
/// This facade deliberately has no raw SurrealQL, `Clone`, `Deref`, or
/// raw-client accessor. Namespace/database switching and schema administration
/// are available only through the crate-private admin lease.
/// Its methods await database work to completion, so no SDK handle can escape
/// the lifecycle lease held by [`SurrealStorage::with_data_operation`].
pub struct SurrealDataContext<'a> {
    client: &'a SurrealClient,
}

impl SurrealDataContext<'_> {
    pub(crate) async fn upsert_one<R, D>(
        &self,
        table: &str,
        id: &str,
        content: D,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        D: surrealdb::types::SurrealValue,
    {
        Ok(self.client.upsert((table, id)).content(content).await?)
    }

    pub async fn select_one<R>(
        &self,
        table: &str,
        id: &str,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
    {
        Ok(self.client.select((table, id)).await?)
    }

    /// Every row of `table`.
    pub(crate) async fn select_all<R>(&self, table: &str) -> Result<Vec<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
    {
        Ok(self.client.select(table).await?)
    }

    /// Deletes one record, returning it when it existed.
    pub(crate) async fn delete_one<R>(
        &self,
        table: &str,
        id: &str,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
    {
        Ok(self.client.delete((table, id)).await?)
    }

    /// Runs a parameterized SurrealQL statement and returns the rows of the
    /// FIRST result set.
    ///
    /// Caller values travel as BINDINGS and are never concatenated into the
    /// statement text, so this widens what the facade can express without
    /// giving callers a way to build an injectable query or to reach the SDK
    /// handle. The statement stays a `&'static str` for the same reason: a
    /// runtime-assembled query string cannot be passed in.
    ///
    /// Multiple statements separated by `;` execute in one round trip, so a
    /// caller that needs several statements to be atomic writes them as
    /// `BEGIN TRANSACTION; ...; COMMIT TRANSACTION;` here rather than issuing
    /// them separately.
    pub(crate) async fn query_values<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Vec<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        B: surrealdb::types::SurrealValue + Send,
    {
        let mut response = self
            .client
            .query(statement)
            .bind(surrealdb::types::SurrealValue::into_value(bindings))
            .await?
            .check()?;
        Ok(response.take(0)?)
    }

    /// [`Self::query_values`] returning only the first row.
    pub(crate) async fn query_first<R, B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        B: surrealdb::types::SurrealValue + Send,
    {
        Ok(self
            .query_values::<R, B>(statement, bindings)
            .await?
            .into_iter()
            .next())
    }

    /// Runs a parameterized statement for its effect and reports how many rows
    /// the first result set returned.
    ///
    /// Statements whose affected-row count matters must end in `RETURN AFTER`
    /// (or `RETURN BEFORE`), because SurrealDB reports affected rows by
    /// returning them. A conditional update that matched nothing therefore
    /// yields `0`, which is the signal callers use to detect a lost race.
    pub(crate) async fn execute_returning<B>(
        &self,
        statement: &'static str,
        bindings: B,
    ) -> Result<usize, SurrealStorageError>
    where
        B: surrealdb::types::SurrealValue + Send,
    {
        let rows = self
            .query_values::<surrealdb::types::Value, B>(statement, bindings)
            .await?;
        Ok(rows.len())
    }

    /// Creates a record only when its id is free, returning `Ok(None)` when it
    /// is already taken.
    ///
    /// This is the fail-closed counterpart of [`Self::upsert_one`] and replaces
    /// the PostgreSQL `INSERT ... ON CONFLICT DO NOTHING` plus unique-violation
    /// SQLSTATE handling that idempotency keys, mailbox lease acquisition and
    /// duplicate-outcome detection relied on. The existence test and the create
    /// run inside ONE statement, so two callers racing for the same id cannot
    /// both observe it as free.
    pub(crate) async fn create_if_absent<R, D>(
        &self,
        table: &'static str,
        id: &str,
        content: D,
    ) -> Result<Option<R>, SurrealStorageError>
    where
        R: surrealdb::types::SurrealValue,
        D: surrealdb::types::SurrealValue + Send,
    {
        // The derive expands to unqualified `SurrealValue` references, so the
        // trait has to be in scope here rather than named by full path.
        use surrealdb::types::SurrealValue;

        #[derive(SurrealValue)]
        struct CreateIfAbsentBindings<D> {
            tb: String,
            id: String,
            content: D,
        }

        let rows = self
            .query_values::<R, _>(
                "LET $record = type::thing($tb, $id); \
                 IF (SELECT VALUE id FROM $record) IS NONE \
                 { RETURN CREATE $record CONTENT $content; } \
                 ELSE { RETURN []; };",
                CreateIfAbsentBindings {
                    tb: table.to_owned(),
                    id: id.to_owned(),
                    content,
                },
            )
            .await?;
        Ok(rows.into_iter().next())
    }
}

struct SurrealAdminContext<'a> {
    client: &'a SurrealClient,
}

impl SurrealAdminContext<'_> {
    async fn query(
        &self,
        statement: impl Into<String> + Send,
    ) -> Result<surrealdb::IndexedResults, SurrealStorageError> {
        Ok(self.client.query(statement.into()).await?.check()?)
    }

    async fn query_bound<B>(
        &self,
        statement: impl Into<String> + Send,
        bindings: B,
    ) -> Result<surrealdb::IndexedResults, SurrealStorageError>
    where
        B: surrealdb::types::SurrealValue + Send,
    {
        Ok(self
            .client
            .query(statement.into())
            .bind(surrealdb::types::SurrealValue::into_value(bindings))
            .await?
            .check()?)
    }
}

type SharedShutdownResult = Result<(), Arc<str>>;

enum ShutdownCoordinatorState {
    Open,
    Closing {
        receiver: watch::Receiver<Option<SharedShutdownResult>>,
    },
    Closed,
}

struct SurrealStorageInner {
    config: SurrealStorageConfig,
    client: RwLock<Option<SurrealClient>>,
    guard: Arc<dyn StorageGuard>,
    lifecycle: AtomicU8,
    shutdown: Mutex<ShutdownCoordinatorState>,
}

impl SurrealStorage {
    /// Returns the feature-gated, read-only test inspection facade.
    ///
    /// The facade exposes only catalog-validated selectors and bound-value
    /// observations; it never exposes the underlying SDK client or raw query
    /// execution.
    #[cfg(feature = "surreal-test-support")]
    pub fn test_inspector(&self) -> SurrealTestInspector {
        SurrealTestInspector::new(self.clone())
    }

    pub async fn open(config: SurrealStorageConfig) -> Result<Self, SurrealStorageError> {
        Self::open_with_guard(config, Arc::new(DefaultStorageGuard)).await
    }

    pub async fn open_with_guard(
        mut config: SurrealStorageConfig,
        guard: Arc<dyn StorageGuard>,
    ) -> Result<Self, SurrealStorageError> {
        validate_supported_storage_path(config.path())?;
        std::fs::create_dir_all(config.path()).map_err(|source| SurrealStorageError::Io {
            path: config.path().to_path_buf(),
            source,
        })?;
        config.path =
            dunce::canonicalize(config.path()).map_err(|source| SurrealStorageError::Io {
                path: config.path().to_path_buf(),
                source,
            })?;
        validate_supported_storage_path(config.path())?;

        let client = Surreal::new::<RocksDb>(config.path()).await?;
        client
            .use_ns(config.namespace())
            .use_db(config.database())
            .await?;

        let mut context = client
            .query("RETURN session::ns(); RETURN session::db();")
            .await?
            .check()?;
        let actual_namespace: Option<String> = context.take(0)?;
        let actual_database: Option<String> = context.take(1)?;
        let actual_namespace = actual_namespace.unwrap_or_default();
        let actual_database = actual_database.unwrap_or_default();
        if actual_namespace != config.namespace() || actual_database != config.database() {
            return Err(SurrealStorageError::ContextMismatch {
                expected_namespace: config.namespace().to_owned(),
                expected_database: config.database().to_owned(),
                actual_namespace,
                actual_database,
            });
        }

        Ok(Self {
            inner: Arc::new(SurrealStorageInner {
                config,
                client: RwLock::new(Some(client)),
                guard,
                lifecycle: AtomicU8::new(LIFECYCLE_OPEN),
                shutdown: Mutex::new(ShutdownCoordinatorState::Open),
            }),
        })
    }

    pub async fn open_default() -> Result<Self, SurrealStorageError> {
        Self::open(SurrealStorageConfig::from_env()?).await
    }

    pub fn config(&self) -> &SurrealStorageConfig {
        &self.inner.config
    }

    /// Runs an operation while holding a shared lifecycle lease.
    ///
    /// The callback receives only a sealed facade. Holding the read lease across
    /// its returned future lets shutdown stop new work and drain every in-flight
    /// operation before dropping the final owned SDK handle.
    pub async fn with_data_operation<T, F>(&self, operation: F) -> Result<T, SurrealStorageError>
    where
        T: Send,
        F: for<'a> FnOnce(SurrealDataContext<'a>) -> SurrealOperation<'a, T>,
    {
        self.with_lease(|client| operation(SurrealDataContext { client }))
            .await
    }

    async fn with_admin_operation<T, F>(&self, operation: F) -> Result<T, SurrealStorageError>
    where
        T: Send,
        F: for<'a> FnOnce(SurrealAdminContext<'a>) -> SurrealOperation<'a, T>,
    {
        self.with_lease(|client| operation(SurrealAdminContext { client }))
            .await
    }

    async fn with_lease<T, F>(&self, operation: F) -> Result<T, SurrealStorageError>
    where
        T: Send,
        F: for<'a> FnOnce(&'a SurrealClient) -> SurrealOperation<'a, T>,
    {
        if self.inner.lifecycle.load(Ordering::Acquire) != LIFECYCLE_OPEN {
            return Err(SurrealStorageError::Closed);
        }
        let guard = self.inner.client.read().await;
        if self.inner.lifecycle.load(Ordering::Acquire) != LIFECYCLE_OPEN {
            return Err(SurrealStorageError::Closed);
        }
        let client = guard.as_ref().ok_or(SurrealStorageError::Closed)?;
        INSIDE_SURREAL_OPERATION.scope((), operation(client)).await
    }

    pub async fn is_closed(&self) -> bool {
        self.inner.lifecycle.load(Ordering::Acquire) == LIFECYCLE_CLOSED
    }

    pub fn is_accepting_operations(&self) -> bool {
        self.inner.lifecycle.load(Ordering::Acquire) == LIFECYCLE_OPEN
    }

    /// Flushes prior work through a query barrier and closes every wrapper clone.
    ///
    /// SurrealDB 3.2 exposes no explicit embedded-engine close method. This type
    /// therefore never hands out a cloned handle and drops its sole handle after
    /// draining operations. On Windows, completion is published only after an
    /// exclusive open of RocksDB's `LOCK` file proves the prior engine handle is
    /// gone. The locked SDK exposes no equivalent portable completion signal, so
    /// non-Windows builds do not claim that stronger release proof.
    /// Repeated and concurrent shutdown calls are intentionally idempotent.
    pub async fn shutdown(&self) -> Result<(), SurrealStorageError> {
        if INSIDE_SURREAL_OPERATION.try_with(|_| ()).is_ok() {
            return Err(SurrealStorageError::ReentrantShutdown);
        }

        let mut receiver = {
            let mut coordinator = self.inner.shutdown.lock().await;
            match &*coordinator {
                ShutdownCoordinatorState::Closed => return Ok(()),
                ShutdownCoordinatorState::Closing { receiver } => receiver.clone(),
                ShutdownCoordinatorState::Open => {
                    let (sender, receiver) = watch::channel(None);
                    *coordinator = ShutdownCoordinatorState::Closing {
                        receiver: receiver.clone(),
                    };
                    self.inner
                        .lifecycle
                        .store(LIFECYCLE_CLOSING, Ordering::Release);
                    let inner = Arc::clone(&self.inner);
                    tokio::spawn(async move {
                        SurrealStorage::run_shutdown(inner, sender).await;
                    });
                    receiver
                }
            }
        };

        let wait = async {
            loop {
                if let Some(result) = receiver.borrow().clone() {
                    return result.map_err(SurrealStorageError::Shutdown);
                }
                receiver.changed().await.map_err(|_| {
                    SurrealStorageError::Shutdown(Arc::from(
                        "shutdown coordinator exited without publishing a result",
                    ))
                })?;
            }
        };
        match tokio::time::timeout(self.inner.config.shutdown_wait, wait).await {
            Ok(result) => result,
            Err(_) => Err(SurrealStorageError::ShutdownStillInProgress {
                waited_ms: self.inner.config.shutdown_wait.as_millis(),
            }),
        }
    }

    async fn run_shutdown(
        inner: Arc<SurrealStorageInner>,
        sender: watch::Sender<Option<SharedShutdownResult>>,
    ) {
        let result = SurrealStorage::perform_shutdown(&inner)
            .await
            .map_err(|error| Arc::<str>::from(error.to_string()));

        let mut coordinator = inner.shutdown.lock().await;
        match &result {
            Ok(()) => {
                inner.lifecycle.store(LIFECYCLE_CLOSED, Ordering::Release);
                *coordinator = ShutdownCoordinatorState::Closed;
            }
            Err(_) => {
                // `perform_shutdown` can fail only before taking the sole client.
                // Once the client is taken, its release barrier never returns an
                // error, so OPEN always implies a usable client is still present.
                inner.lifecycle.store(LIFECYCLE_OPEN, Ordering::Release);
                *coordinator = ShutdownCoordinatorState::Open;
            }
        }
        let _ = sender.send(Some(result));
    }

    async fn perform_shutdown(inner: &SurrealStorageInner) -> Result<(), SurrealStorageError> {
        let mut guard = inner.client.write().await;
        let Some(client) = guard.as_ref() else {
            return Ok(());
        };
        client.query("RETURN true;").await?.check()?;
        let client = guard
            .take()
            .expect("the client was checked while holding the write lease");
        drop(guard);
        drop(client);
        wait_for_engine_release(&inner.config.path).await;
        Ok(())
    }
}

#[cfg(windows)]
async fn wait_for_engine_release(store_path: &Path) {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

    const ERROR_SHARING_VIOLATION: i32 = 32;
    const ERROR_LOCK_VIOLATION: i32 = 33;
    const INITIAL_BACKOFF: Duration = Duration::from_millis(5);
    const MAX_BACKOFF: Duration = Duration::from_millis(250);

    let lock_path = store_path.join("LOCK");
    let mut backoff = INITIAL_BACKOFF;
    let mut last_unexpected_error: Option<String> = None;

    loop {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .share_mode(0)
            .open(&lock_path)
        {
            Ok(probe) => {
                // RocksDB opens this file with the same zero-sharing contract.
                // Acquiring it proves the prior WinFileLock CloseHandle ran.
                drop(probe);
                return;
            }
            Err(error)
                if matches!(
                    error.raw_os_error(),
                    Some(ERROR_SHARING_VIOLATION) | Some(ERROR_LOCK_VIOLATION)
                ) => {}
            Err(error) => {
                let diagnostic = error.to_string();
                if last_unexpected_error.as_deref() != Some(diagnostic.as_str()) {
                    tracing::warn!(
                        lock_path = %lock_path.display(),
                        error = %error,
                        "unexpected error while waiting for the RocksDB lock-release proof; shutdown remains in progress"
                    );
                    last_unexpected_error = Some(diagnostic);
                }
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = backoff.saturating_mul(2).min(MAX_BACKOFF);
    }
}

#[cfg(not(windows))]
async fn wait_for_engine_release(_store_path: &Path) {
    // The locked SDK provides no router join or datastore-close acknowledgement.
    // A yield preserves the prior non-Windows behavior without claiming proof
    // equivalent to the Windows zero-sharing LOCK acquisition above.
    tokio::task::yield_now().await;
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, SurrealStorageError> {
    if path.is_absolute() {
        return Ok(path);
    }

    env::current_dir()
        .map(|current| current.join(&path))
        .map_err(|source| SurrealStorageError::Io { path, source })
}

fn validate_supported_storage_path(path: &Path) -> Result<(), SurrealStorageError> {
    let endpoint_path =
        path.to_str()
            .ok_or_else(|| SurrealStorageError::IncompatibleEndpointPath {
                path: path.to_path_buf(),
                reason: "the storage path is not valid Unicode",
            })?;

    #[cfg(windows)]
    if let Some(std::path::Component::Prefix(prefix)) = path.components().next() {
        match prefix.kind() {
            std::path::Prefix::UNC(_, _) | std::path::Prefix::VerbatimUNC(_, _) => {
                return Err(SurrealStorageError::IncompatibleEndpointPath {
                    path: path.to_path_buf(),
                    reason: "Windows network-share storage roots are not supported by the locked SurrealDB 3.2 and RocksDB integration",
                });
            }
            std::path::Prefix::DeviceNS(_) | std::path::Prefix::Verbatim(_) => {
                return Err(SurrealStorageError::IncompatibleEndpointPath {
                    path: path.to_path_buf(),
                    reason: "Windows device namespace storage roots are not supported",
                });
            }
            std::path::Prefix::VerbatimDisk(_) => {
                return Err(SurrealStorageError::IncompatibleEndpointPath {
                    path: path.to_path_buf(),
                    reason: "the path retains an SDK-incompatible Windows verbatim drive prefix",
                });
            }
            std::path::Prefix::Disk(_) => {}
        }
    }

    // SurrealDB 3.2 treats the first `?` in every local endpoint as the start
    // of query parameters. Validate before filesystem creation and again after
    // canonicalization so this delimiter can never redirect the storage root.
    if endpoint_path.contains('?') {
        return Err(SurrealStorageError::IncompatibleEndpointPath {
            path: path.to_path_buf(),
            reason: "the path contains the SurrealDB 3.2 endpoint query delimiter `?`",
        });
    }

    Ok(())
}

#[cfg(all(test, windows))]
mod windows_path_tests {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};

    use super::*;

    #[test]
    fn sdk_endpoint_path_accepts_ordinary_absolute_drive_path() {
        assert!(validate_supported_storage_path(Path::new(r"C:\handshake\data")).is_ok());
    }

    #[test]
    fn sdk_endpoint_path_rejects_unc_device_and_verbatim_prefixes() {
        assert!(matches!(
            validate_supported_storage_path(Path::new(r"\\server\share\handshake\data")),
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(matches!(
            validate_supported_storage_path(Path::new(r"\\?\UNC\server\share\handshake\data")),
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(matches!(
            validate_supported_storage_path(Path::new(r"\\?\C:\handshake\data")),
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(matches!(
            validate_supported_storage_path(Path::new(r"\\.\C:\handshake\data")),
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
    }

    #[tokio::test]
    async fn open_rejects_query_delimiter_before_creating_directory() {
        let temp = tempfile::tempdir().expect("create temporary root");
        let path = temp.path().join("unsafe?query");
        let config = SurrealStorageConfig::for_store_path(&path).expect("configure absolute path");
        assert!(matches!(
            SurrealStorage::open(config).await,
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(!path.exists(), "rejected path must not be created");
    }

    #[tokio::test]
    async fn open_rejects_non_unicode_path_before_creating_directory() {
        let temp = tempfile::tempdir().expect("create temporary root");
        let mut path = temp.path().to_path_buf();
        path.push(OsString::from_wide(&[b'x' as u16, 0xD800]));
        let config = SurrealStorageConfig::for_store_path(&path).expect("configure absolute path");
        assert!(matches!(
            SurrealStorage::open(config).await,
            Err(SurrealStorageError::IncompatibleEndpointPath { .. })
        ));
        assert!(!path.exists(), "rejected path must not be created");
    }

    #[tokio::test]
    async fn engine_release_waits_for_the_windows_exclusive_lock_to_be_released() {
        use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

        let temp = tempfile::tempdir().expect("create temporary root");
        let lock_path = temp.path().join("LOCK");
        let held_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .share_mode(0)
            .open(&lock_path)
            .expect("hold exclusive RocksDB-style lock");

        let store_path = temp.path().to_path_buf();
        let mut waiter = tokio::spawn(async move {
            wait_for_engine_release(&store_path).await;
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut waiter)
                .await
                .is_err(),
            "release proof must remain pending while the exclusive handle is held"
        );

        drop(held_lock);
        tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("release proof should finish after CloseHandle")
            .expect("release proof task should not panic");
    }
}
