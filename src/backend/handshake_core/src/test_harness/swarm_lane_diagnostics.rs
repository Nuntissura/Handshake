//! Joint Native diagnostics fixture. Invocation remains separately governed.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;

use futures::task::AtomicWaker;
use tokio::runtime::Handle;
use tokio::task::{JoinError, JoinHandle};
use tokio::time::Instant;

use crate::swarm_orchestration::model_lane::ModelLaneDiagnosticsProjection;
use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;

pub struct FixtureInputs {
    pub original_deadline: Instant,
    pub artifact_owner: PathBuf,
}

pub struct NativeFixtureCallbackInputs {
    pub origin: String,
    pub raw_channel_token: String,
    pub issued_auth_context: serde_json::Value,
    pub run_id: String,
    pub scope: ExactResourceScopeAttribution,
    pub diagnostics: ModelLaneDiagnosticsProjection,
    pub public_wire: serde_json::Value,
    pub denied_run_id: String,
    pub expected_denied_status: axum::http::StatusCode,
    pub original_deadline: Instant,
    pub cancellation: tokio::sync::watch::Receiver<bool>,
    pub requests: NativeRequestTasks,
}

/// Runs in the caller's original future. Dropping that future or its process is unsupported.
pub async fn with_swarm_lane_diagnostics_native_fixture<F, Fut>(inputs: FixtureInputs, callback: F)
    -> Result<(), String>
where F: FnOnce(NativeFixtureCallbackInputs) -> Fut,
      Fut: Future<Output = Result<(), String>> {
    let mut callback = Some(callback);
    let artifact_owner = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CheckedArtifactOwner::prepare(&inputs.artifact_owner)
    })) {
        Ok(Ok(owner)) => owner,
        Ok(Err(error)) => return Err(early_callback_failure(callback.take(), error)),
        Err(cause) => return Err(early_callback_failure(
            callback.take(), panic_message("fixture artifact preflight", &cause),
        )),
    };
    let created = std::panic::AssertUnwindSafe(NativeLiveFrame::create(inputs)).catch_unwind().await;
    let mut frame = match created {
        Ok(Ok(frame)) => frame,
        Ok(Err(error)) => return Err(early_callback_failure(callback.take(), error)),
        Err(cause) => return Err(early_callback_failure(
            callback.take(), panic_message("fixture creation", &cause),
        )),
    };
    let (mut failures, wire) = frame.run_native(callback.take().expect("one owned callback")).await;
    // All client and server joins precede removal of any authority/store owner.
    frame.fixture.published_router.take();
    let logout = std::panic::AssertUnwindSafe(frame.fixture.logout()).catch_unwind().await;
    if let Err(cause) = logout {
        failures.push(panic_message("public logout", &cause));
    }
    let mut cleanup_pending = frame.fixture.authenticated.is_some();
    for recorder in &mut frame.fixture.recorders {
        if let Err(error) = recorder.close_retaining() {
            cleanup_pending = true;
            failures.push(error);
        }
    }
    if let Some(lease) = frame.fixture.runtime_lease.take() {
        frame.fixture.lease_release_unknown = Some(lease.descriptor().clone());
        match std::panic::AssertUnwindSafe(lease.release()).catch_unwind().await {
            Ok(Ok(())) => { frame.fixture.lease_release_unknown = None; }
            Ok(Err(error)) => { cleanup_pending = true; failures.push(error.to_string()); }
            Err(cause) => {
                cleanup_pending = true;
                failures.push(panic_message("runtime lease release outcome Unknown", &cause));
            }
        }
    }
    if !cleanup_pending {
        if let Some(channel) = frame.fixture.native_channel.as_mut() {
            if let Err(error) = channel.close_directory() {
                cleanup_pending = true;
                failures.push(error);
            }
        }
    }
    if !cleanup_pending {
        frame.fixture.native_channel.take();
        frame.fixture.bootstrap.take();
        frame.fixture.database.take();
        match std::panic::AssertUnwindSafe(frame.fixture.isolated.cleanup()).catch_unwind().await {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => { cleanup_pending = true; failures.push(error.to_string()); }
            Err(cause) => {
                cleanup_pending = true;
                failures.push(panic_message("isolated store cleanup", &cause));
            }
        }
    }
    if cleanup_pending {
        // This failed frame retains its actual remaining owners and every original error.
        // No ordinary Err return may unwind unfinished custody.
        let retained = (&mut frame, &mut failures, &wire);
        futures::future::pending::<()>().await;
        let _ = retained;
    }
    if Instant::now() >= frame.fixture.original_deadline {
        failures.push("joint fixture exceeded original deadline before publication".into());
    }
    if !failures.is_empty() { return Err(failures.join("; ")); }
    let wire = wire.ok_or("completed Native fixture has no actual projection")?;
    publish_completed_projection(&artifact_owner, &wire)
}

fn early_callback_failure<F>(callback: Option<F>, primary: String) -> String {
    let mut failures = vec![primary];
    dispose_callback_value(callback, "preflight or creation callback disposal", &mut failures);
    failures.join("; ")
}

struct CheckedArtifactOwner {
    root: PathBuf,
    directory: PathBuf,
}

impl CheckedArtifactOwner {
    fn prepare(requested: &std::path::Path) -> Result<Self, String> {
        let root = super::surreal_test_store_support::canonical_artifacts_root_for_proof()
            .map_err(|error| error.to_string())?;
        let directory = root.join("handshake-test").join("wp1-final-audit");
        if requested != directory {
            return Err("fixture artifact owner differs from admitted directory".into());
        }
        let owner = Self { root, directory };
        // Inspect every existing descendant before creating any missing directory.
        owner.check_descendants(true)?;
        let mut path = owner.root.clone();
        for component in ["handshake-test", "wp1-final-audit"] {
            Self::check_directory(&path)?;
            path.push(component);
            match std::fs::symlink_metadata(&path) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    std::fs::create_dir(&path).map_err(|error| error.to_string())?;
                }
                Err(error) => return Err(error.to_string()),
            }
            Self::check_directory(&path)?;
        }
        owner.revalidate()?;
        Ok(owner)
    }

    fn check_directory(path: &std::path::Path) -> Result<(), String> {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        let mut redirected = metadata.file_type().is_symlink();
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            redirected |= metadata.file_attributes() & 0x400 != 0;
        }
        if !metadata.is_dir() || redirected {
            return Err("joint artifact path is not a real non-reparse directory".into());
        }
        if dunce::canonicalize(path).map_err(|error| error.to_string())? != path {
            return Err("joint artifact descendant resolves away from its canonical root".into());
        }
        Ok(())
    }

    fn check_descendants(&self, allow_missing: bool) -> Result<(), String> {
        Self::check_directory(&self.root)?;
        let mut path = self.root.clone();
        for component in ["handshake-test", "wp1-final-audit"] {
            path.push(component);
            match std::fs::symlink_metadata(&path) {
                Ok(_) => Self::check_directory(&path)?,
                Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }

    fn revalidate(&self) -> Result<(), String> {
        let root = super::surreal_test_store_support::canonical_artifacts_root_for_proof()
            .map_err(|error| error.to_string())?;
        if root != self.root || self.directory != root.join("handshake-test").join("wp1-final-audit") {
            return Err("joint artifact canonical root binding changed".into());
        }
        self.check_descendants(false)
    }
}

struct RecorderOwner {
    wrapper: Option<Arc<crate::flight_recorder::duckdb::DuckDbFlightRecorder>>,
    connection_arc: Option<Arc<Mutex<duckdb::Connection>>>,
    connection: Option<duckdb::Connection>,
    native_unknown: bool,
}

impl RecorderOwner {
    fn new(wrapper: Arc<crate::flight_recorder::duckdb::DuckDbFlightRecorder>) -> Self {
        let connection_arc = Some(wrapper.connection());
        Self { wrapper: Some(wrapper), connection_arc, connection: None, native_unknown: false }
    }

    fn close_retaining(&mut self) -> Result<(), String> {
        if self.native_unknown { return Err("recorder native close outcome Unknown".into()); }
        if let Some(wrapper) = self.wrapper.take() {
            match Arc::try_unwrap(wrapper) {
                Ok(wrapper) => drop(wrapper),
                Err(wrapper) => {
                    self.wrapper = Some(wrapper);
                    return Err("recorder wrapper still shared; original wrapper retained".into());
                }
            }
        }
        if let Some(connection) = self.connection_arc.take() {
            let mutex = match Arc::try_unwrap(connection) {
                Ok(mutex) => mutex,
                Err(connection) => {
                    self.connection_arc = Some(connection);
                    return Err("recorder connection still shared; original Arc retained".into());
                }
            };
            match mutex.into_inner() {
                Ok(connection) => self.connection = Some(connection),
                Err(error) => {
                    self.connection = Some(error.into_inner());
                    return Err("recorder mutex poisoned; original connection retained".into());
                }
            }
        }
        if let Some(connection) = self.connection.take() {
            self.native_unknown = true;
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| connection.close())) {
                Ok(Ok(())) => self.native_unknown = false,
                Ok(Err((connection, error))) => {
                    self.connection = Some(connection);
                    self.native_unknown = false;
                    return Err(format!("recorder native close: {error}"));
                }
                Err(cause) => return Err(panic_message("recorder native close outcome Unknown", &cause)),
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeRequestId(usize);

impl NativeRequestId {
    pub fn index(self) -> usize { self.0 }
}

pub enum NativeRequestStartError<F> {
    Closed(F),
    Unavailable(F),
    Capacity(F),
}

impl<F> NativeRequestStartError<F> {
    pub fn into_future(self) -> F {
        match self {
            Self::Closed(future) | Self::Unavailable(future) | Self::Capacity(future) => future,
        }
    }
}

impl<F> std::fmt::Display for NativeRequestStartError<F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Closed(_) => "Native request admission closed",
            Self::Unavailable(_) => "Native request owner unavailable",
            Self::Capacity(_) => "Native request owner capacity unavailable",
        })
    }
}

impl<F> std::fmt::Debug for NativeRequestStartError<F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

#[derive(Clone)]
pub struct NativeRequestTasks {
    shared: Arc<RequestShared>,
}

struct RequestShared {
    state: Mutex<RequestState>,
    changed: AtomicWaker,
    runtime: Handle,
}

struct RequestState {
    open: bool,
    slots: Vec<RequestSlot>,
}

struct RequestSlot {
    handle: Option<JoinHandle<()>>,
    terminal: Option<Result<(), JoinError>>,
    spawn_unknown: bool,
}

impl NativeRequestTasks {
    pub fn spawn_retained<F>(&self, future: F) -> Result<NativeRequestId, NativeRequestStartError<F>>
    where F: Future<Output = ()> + Send + 'static {
        let Ok(mut state) = self.shared.state.lock() else {
            return Err(NativeRequestStartError::Unavailable(future));
        };
        if !state.open {
            return Err(NativeRequestStartError::Closed(future));
        }
        if state.slots.try_reserve(1).is_err() {
            return Err(NativeRequestStartError::Capacity(future));
        }
        let id = NativeRequestId(state.slots.len());
        state.slots.push(RequestSlot { handle: None, terminal: None, spawn_unknown: true });
        let spawned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.shared.runtime.spawn(future)
        }));
        match spawned {
            Ok(handle) => {
                let slot = &mut state.slots[id.0];
                slot.handle = Some(handle);
                slot.spawn_unknown = false;
                drop(state);
                self.shared.changed.wake();
                Ok(id)
            }
            Err(cause) => {
                state.open = false;
                drop(state);
                self.shared.changed.wake();
                std::panic::resume_unwind(cause)
            }
        }
    }
}

struct NativeRequestTaskOwner {
    tasks: NativeRequestTasks,
}

struct NativeLiveFrame {
    fixture: DiagnosticsFixture,
    requests: NativeRequestTaskOwner,
    cancellation: tokio::sync::watch::Sender<bool>,
    server_stop: Option<tokio::sync::oneshot::Sender<()>>,
    server: Option<JoinHandle<std::io::Result<()>>>,
    server_terminal: Option<Result<std::io::Result<()>, JoinError>>,
    server_spawn_unknown: bool,
}

impl NativeLiveFrame {
    async fn run_native<F, Fut>(&mut self, callback: F) -> (Vec<String>, Option<Value>)
    where F: FnOnce(NativeFixtureCallbackInputs) -> Fut,
          Fut: Future<Output = Result<(), String>> {
        let mut callback = Some(callback);
        if let Some(failure) = self.fixture.startup_failure.take() {
            let mut failures = vec![failure];
            dispose_callback_value(callback.take(), "unused Native callback", &mut failures);
            return (failures, None);
        }
        let cancel = self.cancellation.clone();
        let deadline = self.fixture.original_deadline;
        let (prepared_result, expired) = {
            let prepared = std::panic::AssertUnwindSafe(self.prepare_callback()).catch_unwind();
            tokio::pin!(prepared);
            let mut expired = false;
            let result = tokio::select! {
                result = &mut prepared => result,
                _ = tokio::time::sleep_until(deadline) => {
                    expired = true;
                    cancel.send_replace(true);
                    prepared.await
                }
            };
            (result, expired)
        };
        let mut failures = Vec::new();
        if expired { failures.push("backend preparation exceeded original deadline".into()); }
        let wire = match prepared_result {
            Ok(inputs) => {
                let wire = inputs.public_wire.clone();
                if expired {
                    self.requests.close();
                    dispose_callback_value(inputs, "expired Native callback inputs", &mut failures);
                    dispose_callback_value(callback.take(), "expired Native callback", &mut failures);
                } else if let Err(error) = self.call_native(
                    callback.take().expect("one callback invocation"), inputs,
                ).await {
                    failures.push(error);
                }
                Some(wire)
            }
            Err(cause) => {
                failures.push(panic_message("backend floor and live setup", &cause));
                dispose_callback_value(callback.take(), "unconstructed Native callback", &mut failures);
                None
            }
        };
        failures.extend(self.settle_requests_and_server().await);
        (failures, wire)
    }

    async fn create(inputs: FixtureInputs) -> Result<Self, String> {
        let requests = NativeRequestTaskOwner::new()?;
        let (cancellation, _) = tokio::sync::watch::channel(false);
        let fixture = DiagnosticsFixture::create(inputs.artifact_owner, inputs.original_deadline).await;
        Ok(Self {
            fixture, requests, cancellation, server_stop: None, server: None,
            server_terminal: None, server_spawn_unknown: false,
        })
    }

    async fn start_server(&mut self) -> Result<String, String> {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await
            .map_err(|error| format!("Native fixture listener: {error}"))?;
        let address = listener.local_addr().map_err(|error| error.to_string())?;
        let router = self.fixture.diagnostics_router();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        self.server_stop = Some(stop);
        self.server_spawn_unknown = true;
        let spawned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tokio::spawn(async move {
                axum::serve(listener, router).with_graceful_shutdown(async {
                    let _ = stopped.await;
                }).await
            })
        }));
        match spawned {
            Ok(handle) => {
                self.server = Some(handle);
                self.server_spawn_unknown = false;
                Ok(format!("http://{address}"))
            }
            Err(cause) => std::panic::resume_unwind(cause),
        }
    }

    async fn prepare_callback(&mut self) -> NativeFixtureCallbackInputs {
        exercise_recorded_backend_projection(&mut self.fixture).await;
        // The corruption controls above intentionally invalidate their own rows.
        // The live reader receives a newly produced canonical positive, never that stale wire.
        let input = CanonicalDiagnosticsInputs::new(&self.fixture);
        record_and_replay_identical_inputs(&self.fixture, &input).await;
        record_diagnostic_posture_and_status(&self.fixture, &input).await;
        assert_canonical_context_handoff(&self.fixture, &input, None).await;
        let (denied_run_id, denied_status) =
            assert_genuine_foreign_run_http_denial(&mut self.fixture, &input.run.run_id).await;
        assert_eq!(denied_status, StatusCode::NOT_FOUND.as_u16());
        let router = self.fixture.diagnostics_router();
        let diagnostics = assert_public_projection_matches_canonical(
            &self.fixture, &router, &input.run.run_id,
        ).await;
        let (status, public_wire) = self.fixture.public_diagnostics(&router, Some(&input.run.run_id)).await;
        assert_eq!(status, StatusCode::OK);
        drop(router);
        let origin = self.start_server().await.expect("own actual protected TCP server");
        let authenticated = self.fixture.authenticated.as_ref().expect("owned genuine session");
        NativeFixtureCallbackInputs {
            origin,
            raw_channel_token: self.fixture.native_channel.as_ref().expect("owned native binding").token.clone(),
            issued_auth_context: self.fixture.issued_auth_context.as_ref().expect("actual issued response").clone(),
            run_id: input.run.run_id,
            scope: ExactResourceScopeAttribution::try_from_resource_scope(
                authenticated.scope.as_ref().expect("actual original workspace"),
            ).expect("actual five-ID attribution"),
            diagnostics, public_wire, denied_run_id,
            expected_denied_status: StatusCode::NOT_FOUND,
            original_deadline: self.fixture.original_deadline,
            cancellation: self.cancellation.subscribe(),
            requests: self.requests.tasks.clone(),
        }
    }

    async fn call_native<F, Fut>(&mut self, callback: F, inputs: NativeFixtureCallbackInputs)
        -> Result<(), String>
    where F: FnOnce(NativeFixtureCallbackInputs) -> Fut,
          Fut: Future<Output = Result<(), String>> {
        let constructed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(inputs)));
        let future = match constructed {
            Ok(future) => future,
            Err(cause) => return Err(panic_message("Native callback construction", &cause)),
        };
        let mut future = Box::pin(future);
        let mut deadline_reached = false;
        let result = {
            // Poll only a borrowed pin; this catch wrapper never owns Fut's destructor.
            let observed = std::panic::AssertUnwindSafe(future.as_mut()).catch_unwind();
            tokio::pin!(observed);
            tokio::select! {
                result = &mut observed => result,
                _ = tokio::time::sleep_until(self.fixture.original_deadline) => {
                    deadline_reached = true;
                    self.requests.close();
                    self.cancellation.send_replace(true);
                    // The original future and every owner stay in this frame until it settles.
                    observed.await
                }
            }
        };
        let mut failures = Vec::new();
        match result {
            Err(cause) => failures.push(panic_message("Native callback polling", &cause)),
            Ok(Err(error)) => failures.push(error),
            Ok(Ok(())) if deadline_reached || Instant::now() >= self.fixture.original_deadline =>
                failures.push("Native callback exceeded original deadline".into()),
            Ok(Ok(())) => {}
        }
        dispose_callback_value(future, "Native callback future disposal", &mut failures);
        if failures.is_empty() { Ok(()) } else { Err(failures.join("; ")) }
    }

    async fn settle_requests_and_server(&mut self) -> Vec<String> {
        self.requests.close();
        self.cancellation.send_replace(true);
        let mut failures = self.requests.join_all_retained().await;
        if let Some(stop) = self.server_stop.take() {
            let _ = stop.send(());
        }
        if self.server_spawn_unknown {
            // No recovery authority can turn an uncertain spawn into an observed join.
            futures::future::pending::<()>().await;
        }
        if self.server_terminal.is_none() {
            if let Some(handle) = self.server.as_mut() {
                self.server_terminal = Some(handle.await);
            }
        }
        let server_failure = match self.server_terminal.as_ref() {
            Some(Ok(Ok(()))) | None => None,
            Some(Ok(Err(error))) => Some(format!("Native server drain outcome Unknown: {error}")),
            Some(Err(error)) => Some(format!("Native server join outcome Unknown: {error}")),
        };
        if let Some(failure) = server_failure {
            failures.push(failure);
            // A terminal server task error is not evidence that accepted requests drained.
            // Retain this original frame and all remaining authority/store owners.
            let retained = (&mut *self, &mut failures);
            futures::future::pending::<()>().await;
            let _ = retained;
        }
        failures
    }
}

fn dispose_callback_value<T>(value: T, stage: &str, failures: &mut Vec<String>) {
    if let Err(cause) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(value))) {
        failures.push(panic_message(stage, &cause));
    }
}

fn panic_message(stage: &str, cause: &PanicPayload) -> String {
    let message = cause.downcast_ref::<String>().map(String::as_str)
        .or_else(|| cause.downcast_ref::<&'static str>().copied())
        .unwrap_or("non-string panic payload");
    format!("{stage}: {message}")
}

fn publish_completed_projection(owner: &CheckedArtifactOwner, wire: &serde_json::Value)
    -> Result<(), String> {
    use std::io::Write;
    owner.revalidate()?;
    let nonce = std::env::var("HANDSHAKE_MT017_NATIVE_JOINT_DIAGNOSTICS_PROOF_NONCE")
        .map_err(|error| error.to_string())?;
    if nonce.trim().is_empty() { return Err("joint proof nonce is empty".into()); }
    if wire["schema_id"] != "hsk.model_lane_diagnostics_projection@3" {
        return Err("joint projection schema differs".into());
    }
    let scope = wire.get("resource_scope").filter(|scope| scope.is_object())
        .ok_or("joint projection has no public resource scope")?.clone();
    let run_id = wire["run"]["run_id"].as_str().ok_or("joint projection has no run ID")?;
    let bytes = serde_json::to_vec_pretty(wire).map_err(|error| error.to_string())?;
    let milliseconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?.as_millis();
    let milliseconds = u64::try_from(milliseconds).map_err(|error| error.to_string())?;
    let provenance = serde_json::json!({
        "schema_id": "hsk.mt017_native_joint_diagnostics_provenance@1",
        "proof_nonce": nonce,
        "projection_schema_id": "hsk.model_lane_diagnostics_projection@3",
        "artifact_sha256": hex::encode(sha2::Sha256::digest(&bytes)),
        "producer_target": "test_swarm_lane_diagnostics_argus",
        "producer_test_id": "native_consumes_backend_generated_schema_v3_projection_artifact",
        "producer_status": "passed_joint_backend_native_assertions_and_cleanup",
        "producer_completed_at_unix_ms": milliseconds,
        "run_id": run_id,
        "resource_scope": scope,
    });
    let provenance = serde_json::to_vec_pretty(&provenance).map_err(|error| error.to_string())?;
    for (name, content) in [
        ("mt017_native_joint_diagnostics_projection.json", bytes),
        ("mt017_native_joint_diagnostics_projection.provenance.json", provenance),
    ] {
        owner.revalidate()?;
        let path = owner.directory.join(name);
        let mut output = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)
            .map_err(|error| format!("joint artifact create: {error}"))?;
        output.write_all(&content).map_err(|error| error.to_string())?;
        output.sync_all().map_err(|error| error.to_string())?;
        if std::fs::read(&path).map_err(|error| error.to_string())? != content {
            return Err("joint artifact readback differs".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod retained_request_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test(flavor = "current_thread")]
    async fn closed_admission_returns_the_unpolled_original_future() {
        let owner = NativeRequestTaskOwner::new().unwrap();
        owner.close();
        let polls = Arc::new(AtomicUsize::new(0));
        let inside = polls.clone();
        let future = async move { inside.fetch_add(1, Ordering::SeqCst); };
        let rejected = owner.tasks.spawn_retained(future).unwrap_err();
        assert!(matches!(&rejected, NativeRequestStartError::Closed(_)));
        assert_eq!(polls.load(Ordering::SeqCst), 0);
        rejected.into_future().await;
        assert_eq!(polls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn completed_handle_and_terminal_are_retained_and_not_repolled() {
        let mut owner = NativeRequestTaskOwner::new().unwrap();
        let polls = Arc::new(AtomicUsize::new(0));
        let inside = polls.clone();
        let id = owner.tasks.spawn_retained(async move {
            inside.fetch_add(1, Ordering::SeqCst);
        }).unwrap();
        assert_eq!(id.index(), 0);
        owner.close();
        assert!(owner.join_all_retained().await.is_empty());
        assert!(owner.join_all_retained().await.is_empty());
        let state = owner.tasks.shared.state.lock().unwrap();
        assert!(state.slots[0].handle.is_some());
        assert!(matches!(&state.slots[0].terminal, Some(Ok(()))));
        assert_eq!(polls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn callback_unwind_cannot_discard_an_installed_request_handle() {
        let mut owner = NativeRequestTaskOwner::new().unwrap();
        let completed = Arc::new(AtomicUsize::new(0));
        let inside = completed.clone();
        let capability = owner.tasks.clone();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            capability.spawn_retained(async move {
                tokio::task::yield_now().await;
                inside.fetch_add(1, Ordering::SeqCst);
            }).unwrap();
            panic!("owned callback construction control");
        }));
        assert!(caught.is_err());
        owner.close();
        assert!(owner.join_all_retained().await.is_empty());
        assert_eq!(completed.load(Ordering::SeqCst), 1);
        assert_eq!(owner.tasks.shared.state.lock().unwrap().slots.len(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn task_panic_is_cached_as_failure_and_never_becomes_success() {
        let mut owner = NativeRequestTaskOwner::new().unwrap();
        owner.tasks.spawn_retained(async { panic!("request panic control"); }).unwrap();
        owner.close();
        let first = owner.join_all_retained().await;
        assert_eq!(first.len(), 1);
        assert_eq!(owner.join_all_retained().await, first);
        let state = owner.tasks.shared.state.lock().unwrap();
        assert!(state.slots[0].handle.is_some());
        assert!(matches!(&state.slots[0].terminal, Some(Err(_))));
    }
}

impl NativeRequestTaskOwner {
    fn new() -> Result<Self, String> {
        let runtime = Handle::try_current().map_err(|error| error.to_string())?;
        Ok(Self { tasks: NativeRequestTasks { shared: Arc::new(RequestShared {
            state: Mutex::new(RequestState { open: true, slots: Vec::new() }),
            changed: AtomicWaker::new(), runtime,
        }) } })
    }

    fn close(&self) {
        let mut state = self.tasks.shared.state.lock().unwrap_or_else(|error| error.into_inner());
        state.open = false;
        drop(state);
        self.tasks.shared.changed.wake();
    }

    async fn join_all_retained(&mut self) -> Vec<String> {
        futures::future::poll_fn(|context| {
            self.tasks.shared.changed.register(context.waker());
            let mut state = self.tasks.shared.state.lock().unwrap_or_else(|error| error.into_inner());
            let mut pending = false;
            for slot in &mut state.slots {
                if slot.spawn_unknown {
                    pending = true;
                    continue;
                }
                if slot.terminal.is_none() {
                    if let Some(handle) = slot.handle.as_mut() {
                        match Pin::new(handle).poll(context) {
                            Poll::Ready(result) => slot.terminal = Some(result),
                            Poll::Pending => pending = true,
                        }
                    } else {
                        pending = true;
                    }
                }
            }
            if pending {
                Poll::Pending
            } else {
                Poll::Ready(state.slots.iter().enumerate().filter_map(|(index, slot)| {
                    slot.terminal.as_ref().and_then(|result| result.as_ref().err())
                        .map(|error| format!("Native request {index}: {error}"))
                }).collect())
            }
        }).await
    }
}

use std::path::Path;
use std::sync::MutexGuard;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use crate::api::{authority, stage, workspaces};
use crate::api::account_scope::ProductLocalResourceScope;
use crate::capabilities::CapabilityRegistry;
use crate::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError};
use crate::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use crate::llm::DisabledLlmClient;
use crate::process_ledger::{acquire_embedded_runtime_instance_lease, EmbeddedRuntimeInstanceLease};
use crate::workflows::{SessionRegistry, SessionSchedulerConfig};
use crate::storage::Database;
use crate::storage::surreal::resource_authority::{
    AuthorizationRequest, ResourceAction, ResourceKind,
};
use crate::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use crate::test_harness::crdt_workspace::AuthenticatedModelLaneTestOperation;
use crate::swarm_orchestration::model_lane::{
    ModelLaneError, ModelLaneStore,
    NewModelLane, NewModelLaneContextBundleArtifactBinding, NewModelLaneMessage, NewModelLaneRun,
};
use crate::{AccountBootstrapState, AppState};
use futures::FutureExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

use super::surreal_test_store_support;
use super::surreal_test_store_support::IsolatedSurrealTestStore;

static NATIVE_BINDING: Mutex<()> = Mutex::new(());
const RECEIPT_LIMIT: usize = 256;

struct UnavailableAncillaryServices;

#[async_trait::async_trait]
impl FlightRecorder for UnavailableAncillaryServices {
    async fn record_event(&self, _: FlightRecorderEvent) -> Result<(), RecorderError> {
        Err(RecorderError::SinkError("ancillary recorder unavailable in canonical diagnostics proof".into()))
    }
    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Err(RecorderError::SinkError("ancillary recorder unavailable in canonical diagnostics proof".into()))
    }
    async fn list_events(&self, _: EventFilter) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Err(RecorderError::SinkError("ancillary recorder unavailable in canonical diagnostics proof".into()))
    }
}

#[async_trait::async_trait]
impl DiagnosticsStore for UnavailableAncillaryServices {
    async fn record_diagnostic(&self, _: Diagnostic) -> Result<(), crate::storage::StorageError> {
        Err(ancillary_unavailable())
    }
    async fn list_problems(&self, _: DiagFilter) -> Result<Vec<ProblemGroup>, crate::storage::StorageError> {
        Err(ancillary_unavailable())
    }
    async fn get_diagnostic(&self, _: Uuid) -> Result<Diagnostic, crate::storage::StorageError> {
        Err(ancillary_unavailable())
    }
    async fn list_diagnostics(&self, _: DiagFilter) -> Result<Vec<Diagnostic>, crate::storage::StorageError> {
        Err(ancillary_unavailable())
    }
}

fn ancillary_unavailable() -> crate::storage::StorageError {
    crate::storage::StorageError::Database("ancillary diagnostic store unavailable".into())
}

struct NativeChannel {
    _lock: MutexGuard<'static, ()>,
    _directory: Option<tempfile::TempDir>,
    directory_path: PathBuf,
    previous: Option<std::ffi::OsString>,
    token: String,
}

impl NativeChannel {
    fn create(root: &Path) -> Self {
        let lock = NATIVE_BINDING.lock().unwrap_or_else(|error| error.into_inner());
        let directory = tempfile::tempdir_in(root).expect("own native channel below proof artifacts");
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let path = directory.path().join("native-binding.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&stage::current_process_native_session_binding(&token))
                .expect("serialize actual process and birth identity"),
        )
        .expect("write owned native binding");
        let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
        std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", &path);
        let directory_path = directory.path().to_owned();
        Self { _lock: lock, _directory: Some(directory), directory_path, previous, token }
    }

    fn close_directory(&mut self) -> Result<(), String> {
        if let Some(directory) = self._directory.as_ref() {
            // A failed removal keeps the original TempDir, lock, token and environment owner.
            std::fs::remove_dir_all(directory.path())
                .map_err(|error| format!("native binding directory cleanup: {error}"))?;
        }
        if self.directory_path.exists() {
            return Err("native binding directory remains owned after cleanup".into());
        }
        Ok(())
    }
}

impl Drop for NativeChannel {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous);
        } else {
            std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE");
        }
    }
}

struct AuthenticatedWorkspace {
    session_token: String,
    workspace_id: String,
    scope: Option<ResourceScope>,
}

type PanicPayload = Box<dyn std::any::Any + Send>;

struct CleanupFailure {
    stage: &'static str,
    cause: PanicPayload,
}

struct CleanupReport {
    failures: Vec<CleanupFailure>,
    unresolved_vault: Option<AuthenticatedWorkspace>,
}

struct DiagnosticsFailure {
    primary: Option<PanicPayload>,
    cleanup: CleanupReport,
}

struct DiagnosticsFixture {
    isolated: IsolatedSurrealTestStore,
    bootstrap: Option<AccountBootstrapState>,
    database: Option<Arc<dyn Database>>,
    capabilities: Arc<CapabilityRegistry>,
    native_channel: Option<NativeChannel>,
    authenticated: Option<AuthenticatedWorkspace>,
    published_router: Option<Router>,
    runtime_lease: Option<EmbeddedRuntimeInstanceLease>,
    lease_release_unknown: Option<crate::process_ledger::EmbeddedRuntimeInstanceDescriptor>,
    startup_failure: Option<String>,
    artifact_owner: PathBuf,
    original_deadline: Instant,
    issued_auth_context: Option<Value>,
    recorders: Vec<RecorderOwner>,
}

impl DiagnosticsFixture {
    async fn create(artifact_owner: PathBuf, original_deadline: Instant) -> Self {
        let capabilities = Arc::new(CapabilityRegistry::new());
        capabilities.profile_by_id("Operator").expect("loaded canonical Operator profile");
        let isolated = IsolatedSurrealTestStore::create().await.expect("own canonical diagnostics store");
        let mut fixture = Self {
            isolated, bootstrap: None, database: None, capabilities,
            native_channel: None, authenticated: None, published_router: None, runtime_lease: None,
            lease_release_unknown: None, startup_failure: None,
            artifact_owner, original_deadline, issued_auth_context: None,
            recorders: Vec::new(),
        };
        let startup = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let database: Arc<dyn Database> = Arc::new(crate::storage::surreal::SurrealDatabase::new(fixture.isolated.storage().clone()));
            fixture.database = Some(database.clone());
            fixture.bootstrap = Some(AccountBootstrapState::new(
                database, fixture.isolated.storage().clone(), fixture.capabilities.clone(),
            ));
        }));
        if let Err(cause) = startup {
            fixture.startup_failure = Some(panic_message("post-store fixture startup", &cause));
        }
        fixture
    }

    fn router(&self) -> (Router, tokio::sync::mpsc::Receiver<authority::WorkspaceActivationRequest>) {
        let state = self.bootstrap.as_ref().expect("open bootstrap authority").clone();
        let (sender, receiver) = tokio::sync::mpsc::channel(16);
        (
            authority::bootstrap_routes(state.clone(), sender)
                .merge(workspaces::bootstrap_routes(state)),
            receiver,
        )
    }

    async fn authenticate_workspace(&mut self) {
        assert!(self.authenticated.is_none(), "retain one explicit session owner");
        let root = self.artifact_owner.clone();
        std::fs::create_dir_all(&root).expect("create owned diagnostics proof directory");
        self.native_channel = Some(NativeChannel::create(&root));
        let channel = self.native_channel.as_ref().expect("owned channel").token.clone();
        let (router, activation) = self.router();
        let credentials = json!({
            "account_name": "Synthetic isolated diagnostics operator",
            "password": "Synthetic isolated diagnostics password without external access",
        });
        let status = router_json(&router, "GET", "/authority/setup", &channel, None, Value::Null).await;
        assert_eq!(status.0, StatusCode::OK, "public setup status");
        if status.1["setup_required"].as_bool().expect("typed setup status") {
            let setup = router_json(&router, "POST", "/authority/setup", &channel, None, credentials.clone()).await;
            assert_eq!(setup.0, StatusCode::OK, "public owner setup");
            assert_eq!(setup.1["setup_required"], false);
        }
        let login = router_json(&router, "POST", "/authority/login", &channel, None, credentials).await;
        assert_eq!(login.0, StatusCode::OK, "public password login");
        let session = router_json(&router, "POST", "/authority/session", &channel, None, json!({
            "account_id": string(&login.1, "account_id"),
            "principal_id": string(&login.1, "principal_id"),
            "access_space_id": string(&login.1, "access_space_id"),
            "authentication_token": string(&login.1, "token"),
        })).await;
        assert_eq!(session.0, StatusCode::OK, "public durable session and native vault exchange");
        self.issued_auth_context = Some(session.1.clone());
        let token = string(&session.1, "session_token").to_owned();
        self.authenticated = Some(AuthenticatedWorkspace {
            session_token: token.clone(), workspace_id: String::new(), scope: None,
        });
        for field in ["account_id", "principal_id", "access_space_id"] {
            assert_eq!(string(&session.1, field), string(&login.1, field));
        }
        let scope = ResourceScope::new(
            OwnerAccountId::from_uuid(uuid_field(&session.1, "account_id")),
            ActorPrincipalId::from_uuid(uuid_field(&session.1, "principal_id")),
        )
        .with_session(AuthenticatedSessionRef::from_uuid(uuid_field(&session.1, "session_id")))
        .with_access_space(AccessSpaceRef::from_uuid(uuid_field(&session.1, "access_space_id")));
        let workspace = router_json(&router, "POST", "/workspaces", &channel, Some(&token),
            json!({"name": "Synthetic isolated diagnostics workspace"})).await;
        assert_eq!(workspace.0, StatusCode::CREATED, "public atomic workspace/resource/Create grant producer");
        let workspace_id = string(&workspace.1, "id").to_owned();
        let authenticated = self.authenticated.as_mut().expect("session remains owned for cleanup");
        authenticated.scope = Some(scope.with_workspace(
            WorkspaceScopeRef::new(workspace_id.clone()).expect("actual workspace ID"),
        ));
        authenticated.workspace_id = workspace_id;
        drop(router);
        drop(activation);
        self.runtime_lease = Some(acquire_embedded_runtime_instance_lease(
            Uuid::now_v7(), format!("diagnostics-proof:{}", self.isolated.namespace()),
        ).expect("retain a genuine embedded runtime instance lease"));
        self.activate_router(self.app_state()).await;
    }

    async fn authorized_operation(
        &self,
        action: ResourceAction,
        capability_id: &str,
    ) -> AuthenticatedModelLaneTestOperation {
        let authenticated = self.authenticated.as_ref().expect("public session established");
        let scope = authenticated.scope.as_ref().expect("public workspace producer established");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(scope)
            .expect("all five scope fields come from public responses");
        let channel = &self.native_channel.as_ref().expect("owned native binding").token;
        AuthenticatedModelLaneTestOperation::authorize(
            self.isolated.storage().clone(),
            AuthorizationRequest {
                session_token: authenticated.session_token.clone(),
                channel_binding_hash: Some(hex::encode(Sha256::digest(channel.as_bytes()))),
                capability_id: capability_id.to_owned(),
                resource_kind: ResourceKind::Workspace,
                external_resource_id: authenticated.workspace_id.clone(),
                action,
            },
            exact,
        ).await.expect("canonical public workspace grant authorizes this operation")
    }

    fn model_lane_store(&self) -> ModelLaneStore {
        let scope = self.authenticated.as_ref().expect("public session")
            .scope.as_ref().expect("public workspace").clone();
        ModelLaneStore::new_scoped_with_lifecycle(
            self.isolated.storage().clone(), scope,
            self.isolated.storage().resource_access_lifecycle(),
        )
    }

    fn prepare_diagnostics_router(&self, state: AppState) -> Router {
        assert!(Arc::ptr_eq(&state.storage, self.database.as_ref().expect("same canonical database")));
        assert_eq!(state.surreal.resource_access_lifecycle(),
            self.isolated.storage().resource_access_lifecycle());
        let scope = self.authenticated.as_ref().expect("public session")
            .scope.as_ref().expect("public workspace");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(scope)
            .expect("actual five-field public setup outputs");
        let host_selection = ProductLocalResourceScope::from_exact(exact)
            .expect("server selection from actual workspace producer");
        crate::api::diagnostics::routes(state.clone())
            .merge(crate::api::knowledge_documents::routes(state))
            .layer(axum::Extension(host_selection))
    }

    async fn create_document(&self) -> String {
        let authenticated = self.authenticated.as_ref().expect("genuine document creator");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(
            authenticated.scope.as_ref().expect("actual workspace"),
        ).expect("complete original scope");
        let request = Request::builder().method("POST").uri("/knowledge/documents")
            .header("content-type", "application/json")
            .header("x-hsk-channel-binding-token", &self.native_channel.as_ref().expect("native channel").token)
            .header("x-hsk-session-token", &authenticated.session_token)
            .header("x-hsk-actor-id", exact.actor_principal_id.to_string())
            .header("x-hsk-actor-kind", "operator")
            .header("x-hsk-kernel-task-run-id", format!("diagnostics-document-{}", Uuid::now_v7()))
            .header("x-hsk-session-run-id", exact.authenticated_session_id.to_string())
            .body(Body::from(serde_json::to_vec(&json!({
                "workspace_id": authenticated.workspace_id,
                "title": format!("Diagnostics canonical CRDT {}", Uuid::now_v7()),
            })).expect("public document create body"))).expect("public document request");
        let response = self.diagnostics_router().oneshot(request).await.expect("public document creation route");
        assert_eq!(response.status(), StatusCode::OK, "genuine workspace Create must produce the document and its grants");
        let bytes = to_bytes(response.into_body(), 128 * 1024).await.expect("bounded document response");
        let body: Value = serde_json::from_slice(&bytes).expect("public document response JSON");
        string(&body["document"], "rich_document_id").to_owned()
    }

    async fn document_update_operation(&self, document_id: &str) -> AuthenticatedModelLaneTestOperation {
        let authenticated = self.authenticated.as_ref().expect("genuine document editor");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(
            authenticated.scope.as_ref().expect("actual workspace"),
        ).expect("complete original scope");
        let channel = &self.native_channel.as_ref().expect("same native channel").token;
        AuthenticatedModelLaneTestOperation::authorize(self.isolated.storage().clone(), AuthorizationRequest {
            session_token: authenticated.session_token.clone(),
            channel_binding_hash: Some(hex::encode(Sha256::digest(channel.as_bytes()))),
            capability_id: "fs.write".into(), resource_kind: ResourceKind::RichDocument,
            external_resource_id: document_id.to_owned(), action: ResourceAction::Update,
        }, exact).await.expect("actual public-created document grants authorize original Update")
    }

    async fn account_events(&self, aggregate_type: &str, aggregate_id: &str) -> Vec<crate::kernel::KernelEvent> {
        let session = &self.authenticated.as_ref().expect("genuine account EventLedger reader").session_token;
        let channel = &self.native_channel.as_ref().expect("original native channel").token;
        let channel_hash = hex::encode(Sha256::digest(channel.as_bytes()));
        self.isolated.storage().list_account_kernel_events_for_aggregate(
            session, &channel_hash, aggregate_type, aggregate_id,
        ).await.expect("independent authenticated account EventLedger readback")
    }

    fn diagnostics_router(&self) -> Router {
        self.published_router.as_ref().expect("genuine activation published this protected router").clone()
    }

    async fn activate_router(&mut self, state: AppState) {
        let prepared = self.prepare_diagnostics_router(state);
        let authenticated = self.authenticated.as_ref().expect("actual session for activation");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(
            authenticated.scope.as_ref().expect("actual workspace for activation"),
        ).expect("complete public scope");
        let token = authenticated.session_token.clone();
        let workspace_id = authenticated.workspace_id.clone();
        let channel = self.native_channel.as_ref().expect("retained native binding").token.clone();
        let runtime = self.runtime_lease.as_ref().expect("retained real lease").descriptor().clone();
        let (router, mut receiver) = self.router();
        let operation_id = Uuid::now_v7();
        let request = router_json(&router, "POST", "/authority/session", &channel, Some(&token),
            json!({"operation_id": operation_id, "workspace_id": workspace_id}));
        let publication_slot = &mut self.published_router;
        let publication = async {
            let activation = receiver.recv().await.expect("public workspace activation request");
            assert_eq!(activation.scope().exact(), &exact);
            assert_eq!(activation.operation_id(), operation_id);
            activation.authorize_for_publication().await.expect("reauthorize actual activation publication");
            activation.acknowledge_with(&runtime, || {
                *publication_slot = Some(prepared);
                Ok(())
            }).await.expect("durable activation publishes the actual protected router");
        };
        let ((status, acknowledgement), ()) = tokio::join!(request, publication);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(acknowledgement["operation_id"], json!(operation_id));
        assert!(self.published_router.is_some());
    }

    fn app_state(&self) -> AppState {
        let recorder = Arc::new(UnavailableAncillaryServices);
        AppState {
            storage: self.database.as_ref().expect("same canonical database").clone(),
            surreal: self.isolated.storage().clone(),
            flight_recorder: recorder.clone(),
            diagnostics: recorder.clone(),
            llm_client: Arc::new(DisabledLlmClient::new_recorded(
                "diagnostics-no-inference".to_owned(),
                "diagnostics inspection does not invoke a model".to_owned(), recorder,
            )),
            capability_registry: self.capabilities.clone(),
            session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        }
    }

    async fn public_diagnostics(&self, router: &Router, run_id: Option<&str>) -> (StatusCode, Value) {
        let path = match run_id {
            Some(run_id) => format!("/swarm/model-lanes/diagnostics/{run_id}"),
            None => "/swarm/model-lanes/diagnostics/latest".to_owned(),
        };
        router_json(router, "GET", &path,
            &self.native_channel.as_ref().expect("owned channel").token,
            Some(&self.authenticated.as_ref().expect("genuine session").session_token),
            Value::Null,
        ).await
    }

    async fn logout(&mut self) {
        if let Some(authenticated) = self.authenticated.as_ref() {
            let channel = &self.native_channel.as_ref().expect("native binding outlives vault cleanup").token;
            let (router, activation) = self.router();
            let response = router_json(&router, "POST", "/authority/logout", channel,
                Some(&authenticated.session_token), Value::Null).await;
            assert_eq!(response.0, StatusCode::OK, "public logout revokes session and removes its vault credential");
            drop(router);
            drop(activation);
            self.authenticated.take();
        }
    }

    async fn reopen(&mut self) {
        let authenticated = self.authenticated.as_ref().expect("retain genuine session across reopen");
        let expected_scope = authenticated.scope.clone();
        let expected_workspace = authenticated.workspace_id.clone();
        let namespace = self.isolated.namespace().to_owned();
        let database_name = self.isolated.database().to_owned();
        let path = self.isolated.store_path().to_owned();
        drop(self.published_router.take());
        drop(self.bootstrap.take());
        drop(self.database.take());
        self.isolated.reopen().await.expect("close all derived handles and reopen canonical store");
        assert_eq!(self.isolated.namespace(), namespace);
        assert_eq!(self.isolated.database(), database_name);
        assert_eq!(self.isolated.store_path(), path);
        let database: Arc<dyn Database> = Arc::new(crate::storage::surreal::SurrealDatabase::new(self.isolated.storage().clone()));
        self.bootstrap = Some(AccountBootstrapState::new(
            database.clone(), self.isolated.storage().clone(), self.capabilities.clone(),
        ));
        self.database = Some(database);
        let (router, activation) = self.router();
        let authenticated = self.authenticated.as_ref().expect("original session custody retained");
        let current = router_json(&router, "GET", "/authority/session",
            &self.native_channel.as_ref().expect("same process-bound native channel").token,
            Some(&authenticated.session_token), Value::Null).await;
        assert_eq!(current.0, StatusCode::OK, "original persisted session reauthenticates after reopen");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(
            expected_scope.as_ref().expect("original complete workspace scope"),
        ).expect("original exact scope");
        assert_eq!(string(&current.1, "account_id"), exact.owner_account_id.to_string());
        assert_eq!(string(&current.1, "principal_id"), exact.actor_principal_id.to_string());
        assert_eq!(string(&current.1, "session_id"), exact.authenticated_session_id.to_string());
        assert_eq!(string(&current.1, "access_space_id"), exact.access_space_id.to_string());
        assert_eq!(authenticated.workspace_id, expected_workspace);
        drop(router);
        drop(activation);
        self.activate_router(self.app_state()).await;
    }

}

async fn router_json(
    router: &Router, method: &str, path: &str, channel: &str,
    session: Option<&str>, value: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path)
        .header("content-type", "application/json")
        .header("x-hsk-channel-binding-token", channel);
    if let Some(session) = session {
        request = request.header("x-hsk-session-token", session);
    }
    let request = request.body(Body::from(serde_json::to_vec(&value).expect("serialize public request")))
        .expect("construct public request");
    let response = router.clone().oneshot(request).await.expect("dispatch public router");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 128 * 1024).await.expect("bounded public response");
    (status, serde_json::from_slice(&bytes).expect("public JSON response"))
}

fn string<'a>(value: &'a Value, field: &str) -> &'a str {
    value[field].as_str().filter(|value| !value.is_empty()).expect("required public response field")
}

fn uuid_field(value: &Value, field: &str) -> Uuid {
    Uuid::parse_str(string(value, field)).expect("public response UUID")
}

struct CanonicalDiagnosticsInputs {
    run: NewModelLaneRun,
    lane: NewModelLane,
    message: NewModelLaneMessage,
    payload: NewModelLaneContextBundleArtifactBinding,
}

impl CanonicalDiagnosticsInputs {
    fn new(fixture: &DiagnosticsFixture) -> Self {
        let label = Uuid::new_v4().simple().to_string();
        let scope = fixture.authenticated.as_ref().expect("genuine public session")
            .scope.as_ref().expect("genuine public workspace");
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(scope).expect("complete actual scope");
        let session = exact.authenticated_session_id.to_string();
        let run_id = format!("diagnostics-run-{label}");
        let lane_id = format!("diagnostics-lane-{label}");
        let trace_id = Uuid::new_v4().to_string();
        let stream = format!("model-lane://diagnostics/{label}");
        let owner = "diagnostics-fixture-operator";
        let locus = json!({
            "work_packet_id": "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1",
            "micro_task_id": "MT-008", "task_board_id": "task-board://wp-1",
            "coordinator_session_id": session, "session_id": session,
            "model_session_id": session, "owner_session": owner,
            "locus_binding_ref": format!("locus://diagnostics/{label}"),
        });
        let payload_json = json!({"message": "isolated diagnostics canonical payload"});
        let payload_hash = hex::encode(Sha256::digest(serde_json::to_vec(&payload_json).expect("payload bytes")));
        let payload_ref = format!("artifact://diagnostics/{label}/payload");
        let run = serde_json::from_value(json!({
            "run_id": run_id, "trace_id": trace_id, "run_span_id": format!("run-span-{label}"),
            "coordinator_session_id": session, "routing_policy": "operator_lane",
            "context_bundle_id": format!("context-diagnostics-{label}"), "lane_ids": [lane_id],
            "event_ledger_stream_id": stream, "artifact_namespace": format!("artifact://diagnostics/{label}"),
            "work_packet_id": "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1",
            "micro_task_id": "MT-008", "task_board_id": "task-board://wp-1", "owner_session": owner,
            "idempotency_key": format!("diagnostics-run-{label}"), "replay_order_key": format!("{label}:0001"),
            "recovery_state": "restartable", "locus_binding": locus,
            "memory_pack_ref": format!("memory-pack://diagnostics/{label}"),
            "memory_pack_hash": hex::encode(Sha256::digest(b"isolated diagnostics memory fixture")),
            "determinism_mode": "strict", "budget_summary_ref": format!("budget://diagnostics/{label}"),
            "selected_model_id": "human-operator", "candidate_model_ids": ["human-operator"],
            "procedural_review_status": "not-required-human-operator", "rejection_reason_refs": [],
        })).expect("typed public run input");
        let lane = serde_json::from_value(json!({
            "lane_id": lane_id, "run_id": run_id, "trace_id": trace_id,
            "lane_span_id": format!("lane-span-{label}"), "event_ledger_stream_id": stream,
            "kind": "human_operator", "role": "operator", "backend": "human",
            "session_id": session, "model_session_id": session, "adapter_id": "human",
            "runtime_binding": "human", "launch_authority": "operator", "provider_kind": "human",
            "capability_token_ids": [format!("capability://diagnostics/{label}")],
            "effective_capability_snapshot_ref": format!("capability://diagnostics/{label}/snapshot"),
            "capability_negotiation_ref": format!("capability://diagnostics/{label}/negotiation"),
            "provider_feature_profile_ref": "provider://human", "requested_execution_policy_ref": "execution://human",
            "effective_execution_policy_ref": "execution://human", "tool_gate_decision_refs": [format!("tool-gate://diagnostics/{label}")],
            "status": "ready", "recovery_state": "restartable", "restart_generation": 0,
            "cancellation_ref": format!("cancel://diagnostics/{label}"), "reclaim_policy_ref": "reclaim://human",
            "terminal_status_mapping_ref": "terminal://human", "no_os_process_reason_ref": "human-participant-no-os-process",
            "work_packet_id": "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1", "micro_task_id": "MT-008",
            "task_board_id": "task-board://wp-1", "owner_session": owner, "locus_binding": locus,
        })).expect("typed public human lane input");
        let now = chrono::Utc::now().to_rfc3339();
        let message = serde_json::from_value(json!({
            "message_id": format!("diagnostics-message-{label}"), "run_id": run_id, "trace_id": trace_id,
            "message_span_id": format!("message-span-{label}"), "parent_span_id": format!("lane-span-{label}"),
            "linked_span_contexts": [], "from_lane_id": lane_id, "to_lane": {"target_kind":"coordinator"},
            "kind": "proposal", "payload_ref": payload_ref, "payload_sha256": payload_hash,
            "event_ledger_stream_id": stream, "summary": "isolated diagnostics proposal", "authority": "advisory",
            "tool_gate_decision_refs": [], "coordinator_session_id": session,
            "work_packet_id": "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1", "micro_task_id": "MT-008",
            "task_board_id": "task-board://wp-1", "owner_session": owner, "locus_binding": locus,
            "idempotency_key": format!("diagnostics-message-{label}"), "replay_order_key": format!("{label}:0003"),
            "created_at_utc": now, "diagnostic_payload": {},
        })).expect("typed public message input");
        let payload = serde_json::from_value(json!({
            "artifact_binding_id": format!("diagnostics-payload-{label}"), "run_id": run_id, "trace_id": trace_id,
            "artifact_ref": payload_ref, "artifact_sha256": payload_hash, "content_hash": payload_hash,
            "artifact_kind": "diagnostics_payload", "artifact_manifest_ref": format!("artifact-manifest://diagnostics/{label}"),
            "artifact_payload_ref": payload_ref, "payload_json": payload_json, "event_ledger_stream_id": stream,
            "work_packet_id": "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1", "micro_task_id": "MT-008",
            "task_board_id": "task-board://wp-1", "owner_session": owner,
            "idempotency_key": format!("diagnostics-payload-{label}"), "created_at_utc": now, "diagnostic_payload": {},
        })).expect("typed public payload binding input");
        Self { run, lane, message, payload }
    }
}

async fn record_and_replay_identical_inputs(
    fixture: &DiagnosticsFixture,
    input: &CanonicalDiagnosticsInputs,
) {
    let operation = fixture.authorized_operation(ResourceAction::Create, "fs.write").await;
    let store = fixture.model_lane_store();
    let run = operation.run(store.record_run(input.run.clone())).await.expect("canonical run creation");
    let lane = operation.run(store.record_lane(input.lane.clone())).await.expect("canonical lane creation");
    let message = operation.run(store.record_message_with_payload_binding(
        input.message.clone(), input.payload.clone(),
    )).await.expect("atomic canonical payload/message creation");
    assert_eq!(run.run_id, input.run.run_id);
    assert_eq!(lane.run_id, run.run_id);
    assert_eq!(message.run_id, run.run_id);
    assert_eq!(message.from_lane_id, lane.lane_id);
    assert_eq!(message.payload_ref, input.payload.artifact_ref);
    assert_eq!(message.payload_sha256, input.payload.artifact_sha256);
    assert!(run.event_ledger_seq > 0);
    assert!(lane.event_ledger_seq > run.event_ledger_seq);
    assert!(message.event_ledger_seq > lane.event_ledger_seq);
    for (kind, id, event_id, sequence) in [
        ("model_lane_run", run.run_id.as_str(), run.event_ledger_event_id.as_str(), run.event_ledger_seq),
        ("model_lane", lane.lane_id.as_str(), lane.event_ledger_event_id.as_str(), lane.event_ledger_seq),
        ("model_lane_message", message.message_id.as_str(), message.event_ledger_event_id.as_str(), message.event_ledger_seq),
    ] {
        let events = fixture.account_events(kind, id).await;
        let matching = events.iter().filter(|event| event.event_id == event_id).collect::<Vec<_>>();
        assert_eq!(matching.len(), 1, "exact canonical event binding");
        assert_eq!(matching[0].event_sequence, sequence);
    }
    let before = operation.run(store.test_scoped_authority_receipts(&run.run_id, RECEIPT_LIMIT))
        .await.expect("scoped canonical receipt inventory");
    assert!(before.len() >= 4, "run, lane, payload binding and message all have canonical receipts");
    assert_eq!(operation.run(store.record_run(input.run.clone())).await.expect("run retry"), run);
    assert_eq!(operation.run(store.record_lane(input.lane.clone())).await.expect("lane retry"), lane);
    assert_eq!(operation.run(store.record_message_with_payload_binding(
        input.message.clone(), input.payload.clone(),
    )).await.expect("atomic payload/message retry"), message);
    let after = operation.run(store.test_scoped_authority_receipts(&run.run_id, RECEIPT_LIMIT))
        .await.expect("scoped inventory after retry");
    assert_eq!(before.len(), after.len(), "retry adds no receipt rows");
    assert_eq!(receipt_identities(&before), receipt_identities(&after), "retry duplicates no canonical authority");
}

async fn record_diagnostic_posture_and_status(
    fixture: &DiagnosticsFixture,
    input: &CanonicalDiagnosticsInputs,
) {
    use crate::swarm_orchestration::model_lane::{
        NewModelLaneDiagnosticTierStatus, NewModelLaneMtRuntimeStatus,
    };
    let operation = fixture.authorized_operation(ResourceAction::Create, "fs.write").await;
    let store = fixture.model_lane_store();
    for tier in ["flight_recorder", "internal_diagnostics", "palmistry"] {
        let status: NewModelLaneDiagnosticTierStatus = serde_json::from_value(json!({
            "diagnostic_status_id": format!("{}-{tier}", input.run.run_id),
            "behavior_id": "HBR-INT-009", "run_id": input.run.run_id, "tier": tier,
            "state": "deferred_with_reason", "reason": "backend diagnostics conformance does not mount native readers",
            "evidence_ref": format!("model-lane-run://{}", input.run.run_id),
            "follow_up_ref": "usermanual://swarm-lane-diagnostics/native-reader-proof",
            "event_ledger_stream_id": input.run.event_ledger_stream_id,
            "work_packet_id": "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1", "micro_task_id": "MT-008",
            "task_board_id": "task-board://wp-1", "owner_session": input.run.owner_session,
            "idempotency_key": format!("{}-{tier}", input.run.run_id), "diagnostic_payload": {},
        })).expect("typed explicit deferred tier input");
        let recorded = operation.run(store.record_diagnostic_tier_status(status.clone())).await.expect("canonical tier recording");
        assert_eq!(recorded.inner, status);
    }
    let status: NewModelLaneMtRuntimeStatus = serde_json::from_value(json!({
        "mt_status_id": format!("{}-mt-status", input.run.run_id), "run_id": input.run.run_id,
        "work_packet_id": "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1", "micro_task_id": "MT-008",
        "task_board_id": "task-board://wp-1", "status": "blocked",
        "blocker_ref": "usermanual://swarm-lane-diagnostics/native-reader-proof",
        "proof_status_ref": "backend-source-acceptance-not-yet-executed",
        "event_ledger_stream_id": input.run.event_ledger_stream_id, "owner_session": input.run.owner_session,
        "idempotency_key": format!("{}-mt-status", input.run.run_id), "diagnostic_payload": {},
    })).expect("typed blocked MT runtime status");
    let recorded = operation.run(store.record_mt_runtime_status(status.clone())).await.expect("canonical MT runtime status");
    assert_eq!(recorded.inner, status);
}

async fn assert_canonical_navigation_drilldowns(
    fixture: &DiagnosticsFixture,
    input: &CanonicalDiagnosticsInputs,
) {
    let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let run = operation.run(store.navigation_by_run(&input.run.run_id)).await.expect("canonical run navigation");
    let lane = operation.run(store.navigation_by_lane(&input.lane.lane_id)).await.expect("canonical lane navigation");
    let message = operation.run(store.navigation_by_message(&input.message.message_id)).await.expect("canonical message navigation");
    for navigation in [&run, &lane, &message] {
        assert_eq!(navigation.run.as_ref().expect("canonical selected run").run_id, input.run.run_id);
        assert!(navigation.lanes.iter().all(|row| row.source.run_id == input.run.run_id));
        assert!(navigation.messages.iter().all(|row| row.source.run_id == input.run.run_id));
        assert!(!navigation.event_ledger_refs.is_empty(), "canonical navigation has visible source receipts");
    }
    assert_eq!(lane.lanes.len(), 1);
    assert_eq!(lane.lanes[0].lane_id, input.lane.lane_id);
    assert_eq!(message.messages.len(), 1);
    assert_eq!(message.messages[0].message_id, input.message.message_id);
    let payload = operation.run(store.navigation_by_artifact_or_context(
        Some(&input.payload.artifact_ref), None, Some(&input.run.run_id),
    )).await;
    if input.message.crdt_update_ref.is_none() {
        assert!(matches!(payload, Err(ModelLaneError::NotFound(_))),
            "ordinary advisory payload cannot acquire a CRDT navigation witness");
    } else {
    let payload = payload.expect("real CRDT authority enables canonical payload drilldown");
    assert_eq!(payload.run.as_ref().expect("payload selected run").run_id, input.run.run_id);
    assert_eq!(payload.artifacts.len(), 1);
    assert_eq!(payload.artifacts[0].source.aggregate_id, input.payload.artifact_binding_id);
    }
}

fn receipt_identities(
    receipts: &[crate::swarm_orchestration::model_lane::ModelLaneScopedAuthorityReceipt],
) -> std::collections::BTreeSet<(String, String, String, i64, String)> {
    assert!(receipts.len() <= RECEIPT_LIMIT, "canonical bounded inspector must fail instead of truncating");
    receipts.iter().map(|receipt| (
        receipt.record_kind.clone(), receipt.aggregate_id.clone(), receipt.event_id.clone(),
        receipt.event_ledger_seq, receipt.payload_hash.clone(),
    )).collect()
}

fn assert_public_scope_fingerprints(value: &Value, scope: &ResourceScope) {
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(scope).expect("exact actual scope");
    let raw = [
        exact.owner_account_id.to_string(), exact.actor_principal_id.to_string(),
        exact.authenticated_session_id.to_string(), exact.access_space_id.to_string(),
        exact.workspace_id.to_string(),
    ];
    let fields = [
        "owner_account_fingerprint", "actor_principal_fingerprint", "authenticated_session_fingerprint",
        "access_space_fingerprint", "workspace_fingerprint",
    ];
    let fingerprints = fields.map(|field| string(&value["resource_scope"], field));
    assert_eq!(fingerprints.iter().copied().collect::<std::collections::BTreeSet<_>>().len(), 5);
    for fingerprint in fingerprints {
        assert_eq!(fingerprint.len(), 64);
        assert!(fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
    let serialized = serde_json::to_string(value).expect("actual public projection serialization");
    for identifier in raw {
        assert!(!serialized.contains(&identifier), "public projection discloses a raw scope identifier");
    }
    assert_eq!(value["resource_scope"]["visibility"], "private_exact_scope_only");
    assert_eq!(value["resource_scope"]["denial_posture"], "foreign_scope_is_absent_restricted_metadata_withheld");
}

async fn assert_five_independent_scope_denials(
    fixture: &DiagnosticsFixture,
    run_id: &str,
) {
    let original = fixture.authenticated.as_ref().expect("public session")
        .scope.as_ref().expect("public workspace");
    let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let positive = operation.run(store.diagnostics_projection(run_id)).await.expect("same-scope positive projection");
    let before = operation.run(store.test_scoped_authority_receipts(run_id, RECEIPT_LIMIT)).await.expect("canonical state before denial");
    let mut mismatches = std::array::from_fn::<_, 5, _>(|_| original.clone());
    mismatches[0].owner_account_id = OwnerAccountId::from_uuid(Uuid::new_v4());
    mismatches[1].actor_principal_id = ActorPrincipalId::from_uuid(Uuid::new_v4());
    mismatches[2].authenticated_session = Some(AuthenticatedSessionRef::from_uuid(Uuid::new_v4()));
    mismatches[3].access_space = Some(AccessSpaceRef::from_uuid(Uuid::new_v4()));
    mismatches[4].workspace = Some(WorkspaceScopeRef::new(Uuid::new_v4().to_string()).expect("foreign workspace token"));
    for scope in mismatches {
        let foreign = ModelLaneStore::new_scoped_with_lifecycle(
            fixture.isolated.storage().clone(), scope,
            fixture.isolated.storage().resource_access_lifecycle(),
        );
        assert_scope_denial(operation.run(foreign.diagnostics_projection(run_id)).await);
        assert_scope_denial(operation.run(foreign.latest_diagnostics_projection()).await);
        assert_scope_denial(operation.run(foreign.navigation_by_run(run_id)).await);
    }
    assert_eq!(operation.run(store.diagnostics_projection(run_id)).await.expect("positive after denied reads"), positive);
    let after = operation.run(store.test_scoped_authority_receipts(run_id, RECEIPT_LIMIT)).await.expect("canonical state after denial");
    assert_eq!(before.len(), after.len());
    assert_eq!(receipt_identities(&before), receipt_identities(&after), "denied reads mutate no canonical authority");
}

async fn assert_public_projection_matches_canonical(
    fixture: &DiagnosticsFixture,
    router: &Router,
    run_id: &str,
) -> ModelLaneDiagnosticsProjection {
    let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let canonical = operation.run(store.diagnostics_projection(run_id)).await.expect("canonical diagnostics projection");
    let (status, public) = fixture.public_diagnostics(router, Some(run_id)).await;
    assert_eq!(status, StatusCode::OK, "actual public diagnostics route");
    assert_public_scope_fingerprints(&public, fixture.authenticated.as_ref().expect("session")
        .scope.as_ref().expect("workspace"));
    assert_eq!(public["schema_id"], canonical.schema_id);
    assert_eq!(public["surface_contract_id"], canonical.surface_contract_id);
    assert_eq!(public["run"]["run_id"], canonical.run.run_id);
    assert_eq!(public["lanes"].as_array().expect("public lanes").len(), canonical.lanes.len());
    assert_eq!(public["messages"].as_array().expect("public messages").len(), canonical.messages.len());
    for message in &canonical.messages {
        let row = public["messages"].as_array().expect("public messages").iter()
            .find(|row| row["message_id"] == message.message_id).expect("exact canonical public message");
        assert_eq!(row["payload_ref"], message.payload_ref);
        assert_eq!(row["payload_sha256"], message.payload_sha256);
        assert_eq!(row["event_ledger_event_id"], message.event_ledger_event_id);
        assert_eq!(row["event_ledger_seq"], message.event_ledger_seq);
    }
    canonical
}

fn assert_scope_denial<T>(result: Result<T, ModelLaneError>) {
    assert!(matches!(result,
        Err(ModelLaneError::ScopeDenied(_)) | Err(ModelLaneError::AuthorityDenied(_))
            | Err(ModelLaneError::NotFound(_))
    ), "scope negative must deny through authority rather than a storage/JSON failure");
}

async fn assert_deterministic_replay_and_missing_run(
    fixture: &DiagnosticsFixture,
    input: &CanonicalDiagnosticsInputs,
) {
    let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let first = operation.run(store.replay_run(&input.run.run_id)).await.expect("canonical replay");
    let second = operation.run(store.replay_run(&input.run.run_id)).await.expect("repeat canonical replay");
    assert_eq!(serde_json::to_value(&first).expect("replay serialization"),
        serde_json::to_value(&second).expect("replay serialization"));
    assert_eq!(first.run.inner, input.run);
    let lane = first.lanes.iter().find(|lane| lane.lane_id == input.lane.lane_id).expect("same-run lane");
    let message = first.messages.iter().find(|message| message.message_id == input.message.message_id)
        .expect("same-lane canonical message");
    assert_eq!(lane.inner, input.lane);
    assert_eq!(message.inner, input.message);
    assert_eq!(message.payload_sha256, input.payload.content_hash);
    let missing = format!("missing-run-{}", Uuid::new_v4());
    let before = operation.run(store.test_scoped_authority_receipts(&input.run.run_id, RECEIPT_LIMIT)).await.expect("before missing reads");
    assert!(matches!(operation.run(store.replay_run(&missing)).await, Err(ModelLaneError::NotFound(_))));
    assert!(matches!(operation.run(store.diagnostics_projection(&missing)).await, Err(ModelLaneError::NotFound(_))));
    assert_scope_denial(operation.run(store.navigation_by_run(&missing)).await);
    let after = operation.run(store.test_scoped_authority_receipts(&input.run.run_id, RECEIPT_LIMIT)).await.expect("after missing reads");
    assert_eq!(before.len(), after.len());
    assert_eq!(receipt_identities(&before), receipt_identities(&after));
}

async fn assert_stale_and_revoked_contexts(
    fixture: &DiagnosticsFixture,
    run_id: &str,
) {
    let scope = fixture.authenticated.as_ref().expect("public session")
        .scope.as_ref().expect("actual workspace").clone();
    let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let positive = operation.run(store.diagnostics_projection(run_id)).await.expect("positive before stale/revoked contexts");
    let before = operation.run(store.test_scoped_authority_receipts(run_id, RECEIPT_LIMIT)).await.expect("before revoked reads");
    for denied in [
        ModelLaneStore::new_test_stale_access(fixture.isolated.storage().clone(), scope.clone()),
        ModelLaneStore::new_test_revoked_access(fixture.isolated.storage().clone(), scope),
    ] {
        assert_scope_denial(operation.run(denied.diagnostics_projection(run_id)).await);
        assert_scope_denial(operation.run(denied.replay_run(run_id)).await);
        assert_scope_denial(operation.run(denied.navigation_by_run(run_id)).await);
    }
    assert_eq!(operation.run(store.diagnostics_projection(run_id)).await.expect("same-store positive after negatives"), positive);
    let after = operation.run(store.test_scoped_authority_receipts(run_id, RECEIPT_LIMIT)).await.expect("after revoked reads");
    assert_eq!(before.len(), after.len());
    assert_eq!(receipt_identities(&before), receipt_identities(&after));
}

async fn assert_lineage_tamper_fails_without_partial_projection(
    fixture: &DiagnosticsFixture,
    run_id: &str,
) {
    use crate::swarm_orchestration::model_lane::ModelLaneAuthorityTestCorruption;
    let create = fixture.authorized_operation(ResourceAction::Create, "fs.write").await;
    let read = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    read.run(store.diagnostics_projection(run_id)).await.expect("positive before deliberate lineage tamper");
    create.run(store.test_corrupt_scoped_authority(
        "run", run_id, ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
    )).await.expect("owned scoped lineage tamper control");
    let before = read.run(store.test_scoped_authority_receipts(run_id, RECEIPT_LIMIT)).await.expect("post-tamper baseline");
    assert!(matches!(read.run(store.diagnostics_projection(run_id)).await,
        Err(ModelLaneError::IntegrityViolation(_))), "tampered lineage must yield typed integrity failure");
    let after = read.run(store.test_scoped_authority_receipts(run_id, RECEIPT_LIMIT)).await.expect("state after integrity denial");
    assert_eq!(before.len(), after.len());
    assert_eq!(receipt_identities(&before), receipt_identities(&after));
}

async fn assert_mixed_scope_concurrent_reads(fixture: &DiagnosticsFixture, run_id: &str) {
    let read = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let mut wrong_scope = fixture.authenticated.as_ref().expect("session")
        .scope.as_ref().expect("workspace").clone();
    wrong_scope.actor_principal_id = ActorPrincipalId::from_uuid(Uuid::new_v4());
    let foreign = ModelLaneStore::new_scoped_with_lifecycle(
        fixture.isolated.storage().clone(), wrong_scope,
        fixture.isolated.storage().resource_access_lifecycle(),
    );
    let before = read.run(store.test_scoped_authority_receipts(run_id, RECEIPT_LIMIT)).await.expect("before concurrent reads");
    let (positive, denied) = tokio::join!(
        read.run(store.diagnostics_projection(run_id)),
        read.run(foreign.diagnostics_projection(run_id)),
    );
    assert_eq!(positive.expect("authorized concurrent projection").run.run_id, run_id);
    assert_scope_denial(denied);
    let after = read.run(store.test_scoped_authority_receipts(run_id, RECEIPT_LIMIT)).await.expect("after concurrent reads");
    assert_eq!(before.len(), after.len());
    assert_eq!(receipt_identities(&before), receipt_identities(&after));
}

async fn assert_same_store_reopen_preserves_projection(
    fixture: &mut DiagnosticsFixture,
    run_id: &str,
    context_bundle_id: Option<&str>,
) {
    let (before, context_before) = {
        let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
        let store = fixture.model_lane_store();
        let projection = operation.run(store.diagnostics_projection(run_id)).await.expect("projection before durable reopen");
        let context = match context_bundle_id {
            Some(id) => operation.run(store.replay_context_bundle_handoffs(run_id, id))
                .await.expect("canonical context references before durable reopen"),
            None => Vec::new(),
        };
        (projection, context)
    };
    fixture.reopen().await;
    let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let after = operation.run(store.diagnostics_projection(run_id)).await.expect("projection after durable reopen");
    assert_eq!(before, after, "same namespace/database/session preserves diagnostic ordering and references");
    if let Some(id) = context_bundle_id {
        assert!(!context_before.is_empty(), "restart control requires genuine persisted context handoffs");
        let context_after = operation.run(store.replay_context_bundle_handoffs(run_id, id))
            .await.expect("canonical context references after SAME-store reopen");
        assert_eq!(context_before, context_after, "CRDT metadata, payload references and event joins survive reopen");
    }
}

async fn assert_unavailable_authority_is_not_recorder_fallback(
    fixture: &mut DiagnosticsFixture,
    run_id: &str,
) {
    let (before, unavailable_result) = {
        let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
        let store = fixture.model_lane_store();
        let before = operation.run(store.diagnostics_projection(run_id)).await.expect("positive before authority shutdown");
        let router = fixture.diagnostics_router();
        let unavailable_result = std::panic::AssertUnwindSafe(async {
            let direct = operation.run(async {
                fixture.isolated.storage().shutdown().await.expect("close authority after genuine operation authorization");
                store.diagnostics_projection(run_id).await
            }).await;
            assert!(matches!(direct, Err(ModelLaneError::Surreal(_)) | Err(ModelLaneError::Storage(_))),
                "closed durable authority produces a typed storage failure");
            let response = fixture.public_diagnostics(&router, Some(run_id)).await;
            assert_eq!(response.0, StatusCode::FORBIDDEN, "whole-authority shutdown also removes authentication authority");
            assert!(response.1.get("run").is_none(), "unavailable response contains no partial/recorder projection");
        }).catch_unwind().await;
        drop(router);
        (before, unavailable_result)
    };
    let reopen = std::panic::AssertUnwindSafe(fixture.reopen()).catch_unwind().await;
    match (unavailable_result, reopen) {
        (Ok(()), Ok(())) => {}
        (Err(primary), Ok(())) => std::panic::resume_unwind(primary),
        (result, Err(cause)) => std::panic::panic_any(DiagnosticsFailure {
            primary: result.err(),
            cleanup: CleanupReport {
                failures: vec![CleanupFailure { stage: "reopen genuine diagnostics authority", cause }],
                unresolved_vault: None,
            },
        }),
    }
    let operation = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    assert_eq!(operation.run(store.diagnostics_projection(run_id)).await.expect("same-store positive after unavailable authority"), before);
}

async fn exercise_recorded_backend_projection(fixture: &mut DiagnosticsFixture) -> Value {
    fixture.authenticate_workspace().await;
    let input = CanonicalDiagnosticsInputs::new(fixture);
    record_and_replay_identical_inputs(fixture, &input).await;
    record_diagnostic_posture_and_status(fixture, &input).await;
    let public = {
        let router = fixture.diagnostics_router();
        assert_public_projection_matches_canonical(fixture, &router, &input.run.run_id).await;
        let latest = fixture.public_diagnostics(&router, None).await;
        assert_eq!(latest.0, StatusCode::OK);
        assert_eq!(latest.1["run"]["run_id"], input.run.run_id);
        assert_public_scope_fingerprints(&latest.1, fixture.authenticated.as_ref().expect("session")
            .scope.as_ref().expect("workspace"));
        latest.1
    };
    assert_canonical_navigation_drilldowns(fixture, &input).await;
    assert_canonical_context_handoff(fixture, &input, None).await;
    let (crdt_run_id, crdt_context_id) = assert_authenticated_crdt_payload_navigation(fixture).await;
    assert_five_independent_scope_denials(fixture, &input.run.run_id).await;
    assert_deterministic_replay_and_missing_run(fixture, &input).await;
    assert_stale_and_revoked_contexts(fixture, &input.run.run_id).await;
    assert_mixed_scope_concurrent_reads(fixture, &input.run.run_id).await;
    assert_genuine_foreign_run_http_denial(fixture, &input.run.run_id).await;
    assert_same_store_reopen_preserves_projection(fixture, &input.run.run_id, None).await;
    assert_same_store_reopen_preserves_projection(fixture, &crdt_run_id, Some(&crdt_context_id)).await;
    assert_unavailable_authority_is_not_recorder_fallback(fixture, &input.run.run_id).await;
    assert_lineage_tamper_fails_without_partial_projection(fixture, &input.run.run_id).await;
    assert_incomplete_legacy_scope_is_absent(fixture).await;
    assert_real_recorder_only_event_is_not_authority(fixture).await;
    public
}

async fn assert_genuine_foreign_run_http_denial(
    fixture: &mut DiagnosticsFixture,
    original_run_id: &str,
) -> (String, u16) {
    let authenticated = fixture.authenticated.as_ref().expect("retain the issued session owner");
    let original_workspace = authenticated.workspace_id.clone();
    let original_scope = authenticated.scope.as_ref().expect("public original workspace scope").clone();
    let (bootstrap_router, activation_receiver) = fixture.router();
    let created = router_json(&bootstrap_router, "POST", "/workspaces",
        &fixture.native_channel.as_ref().expect("same native channel").token,
        Some(&authenticated.session_token),
        json!({"name": format!("Diagnostics foreign workspace {}", Uuid::now_v7())}),
    ).await;
    drop(bootstrap_router);
    drop(activation_receiver);
    assert_eq!(created.0, StatusCode::CREATED, "public producer creates a separate protected workspace and grants");
    let foreign_workspace = string(&created.1, "id").to_owned();
    assert_ne!(foreign_workspace, original_workspace);
    let foreign_scope = original_scope.clone().with_workspace(
        WorkspaceScopeRef::new(foreign_workspace.clone()).expect("actual second workspace response"),
    );
    fn select_public_workspace(fixture: &mut DiagnosticsFixture, workspace: &str, scope: &ResourceScope) {
        let authenticated = fixture.authenticated.as_mut().expect("same issued session remains owned");
        authenticated.workspace_id = workspace.to_owned();
        authenticated.scope = Some(scope.clone());
    }
    select_public_workspace(fixture, &foreign_workspace, &foreign_scope);
    let result = std::panic::AssertUnwindSafe(async {
        fixture.activate_router(fixture.app_state()).await;
        let foreign = CanonicalDiagnosticsInputs::new(fixture);
        record_and_replay_identical_inputs(fixture, &foreign).await;
        record_diagnostic_posture_and_status(fixture, &foreign).await;
        let foreign_before = {
            let router = fixture.diagnostics_router();
            assert_public_projection_matches_canonical(fixture, &router, &foreign.run.run_id).await;
            let positive = fixture.public_diagnostics(&router, Some(&foreign.run.run_id)).await;
            assert_eq!(positive.0, StatusCode::OK, "the denied run genuinely exists and is readable in its own workspace");
            assert_eq!(positive.1["run"]["run_id"], foreign.run.run_id);
            let read = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
            let store = fixture.model_lane_store();
            let projection = read.run(store.diagnostics_projection(&foreign.run.run_id)).await.expect("foreign positive canonical rows");
            let receipts = read.run(store.test_scoped_authority_receipts(&foreign.run.run_id, RECEIPT_LIMIT))
                .await.expect("complete bounded foreign receipt inventory");
            assert!(receipts.len() >= 4);
            let events = serde_json::to_value(fixture.account_events("model_lane_run", &foreign.run.run_id).await)
                .expect("authenticated foreign EventLedger snapshot");
            (projection, receipt_identities(&receipts), events)
        };
        select_public_workspace(fixture, &original_workspace, &original_scope);
        fixture.activate_router(fixture.app_state()).await;
        let observed_status = {
            let router = fixture.diagnostics_router();
            let positive = fixture.public_diagnostics(&router, Some(original_run_id)).await;
            assert_eq!(positive.0, StatusCode::OK, "real original activation restores the original run positive");
            let read = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
            let store = fixture.model_lane_store();
            let before = read.run(store.test_scoped_authority_receipts(original_run_id, RECEIPT_LIMIT))
                .await.expect("original canonical baseline before denied HTTP reads");
            let events_before = serde_json::to_value(fixture.account_events("model_lane_run", original_run_id).await)
                .expect("authenticated original EventLedger baseline");
            let denied = fixture.public_diagnostics(&router, Some(&foreign.run.run_id)).await;
            let missing = fixture.public_diagnostics(&router, Some(&format!("absent-{}", Uuid::now_v7()))).await;
            assert_eq!(denied.0, StatusCode::NOT_FOUND, "scoped by-ID NotFound maps to the public 404 contract");
            assert_eq!(denied, missing, "an actual foreign run has the same status and complete body as an absent run");
            let withheld = serde_json::to_string(&denied.1).expect("public denial JSON");
            for protected in [&foreign.run.run_id, &foreign.lane.lane_id, &foreign.message.message_id,
                &foreign.payload.artifact_ref, &foreign_workspace] {
                assert!(!withheld.contains(protected), "denial must not disclose foreign identifiers or payload references");
            }
            assert!(denied.1.get("run").is_none());
            assert!(denied.1.get("lanes").is_none());
            assert!(denied.1.get("messages").is_none());
            let after = read.run(store.test_scoped_authority_receipts(original_run_id, RECEIPT_LIMIT))
                .await.expect("original canonical inventory after denied HTTP reads");
            assert_eq!(receipt_identities(&before), receipt_identities(&after));
            assert_eq!(events_before, serde_json::to_value(fixture.account_events("model_lane_run", original_run_id).await)
                .expect("authenticated original EventLedger after denial"));
            assert_eq!(fixture.public_diagnostics(&router, Some(original_run_id)).await, positive,
                "denied and absent HTTP reads preserve the authorized public projection");
            denied.0.as_u16()
        };
        select_public_workspace(fixture, &foreign_workspace, &foreign_scope);
        fixture.activate_router(fixture.app_state()).await;
        {
            let read = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
            let store = fixture.model_lane_store();
            let projection = read.run(store.diagnostics_projection(&foreign.run.run_id)).await.expect("foreign canonical rows after denial");
            let receipts = read.run(store.test_scoped_authority_receipts(&foreign.run.run_id, RECEIPT_LIMIT))
                .await.expect("foreign complete bounded inventory after denial");
            assert_eq!(projection, foreign_before.0);
            assert_eq!(receipt_identities(&receipts), foreign_before.1);
            assert_eq!(serde_json::to_value(fixture.account_events("model_lane_run", &foreign.run.run_id).await)
                .expect("authenticated foreign EventLedger after denial"), foreign_before.2);
        }
        (foreign.run.run_id, observed_status)
    }).catch_unwind().await;
    select_public_workspace(fixture, &original_workspace, &original_scope);
    let restore = std::panic::AssertUnwindSafe(fixture.activate_router(fixture.app_state())).catch_unwind().await;
    match (result, restore) {
        (Ok(observed), Ok(())) => observed,
        (Err(primary), Ok(())) => std::panic::resume_unwind(primary),
        (result, Err(cause)) => std::panic::panic_any(DiagnosticsFailure {
            primary: result.err(),
            cleanup: CleanupReport {
                failures: vec![CleanupFailure { stage: "restore genuine original workspace activation", cause }],
                unresolved_vault: None,
            },
        }),
    }
}

async fn assert_authenticated_crdt_payload_navigation(fixture: &DiagnosticsFixture) -> (String, String) {
    use crate::test_harness::crdt_workspace::build_surreal_admissible_crdt_posture_authenticated;
    let document_id = fixture.create_document().await;
    let workspace_id = &fixture.authenticated.as_ref().expect("genuine workspace").workspace_id;
    let create = fixture.authorized_operation(ResourceAction::Create, "fs.write").await;
    let document_update = fixture.document_update_operation(&document_id).await;
    let read = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let label = format!("diagnostics-{}", Uuid::now_v7());
    let posture = build_surreal_admissible_crdt_posture_authenticated(
        &store, workspace_id, &label, "model://local/diagnostics-fixture", &document_id,
        &create, &document_update,
    ).await.expect("public authenticated CRDT producers acquire real lease, snapshot, proposal review and update");
    assert_eq!(posture.document_id, document_id);
    assert_eq!(posture.proposal.review_state, "promoted");
    assert_eq!(posture.proposal.applied_update_id.as_deref(), Some(posture.update.update_id.as_str()));
    assert_eq!(posture.update.update_sha256, hex::encode(Sha256::digest(&posture.yjs_update_bytes)));
    assert_eq!(posture.proposal.diff_sha256, hex::encode(Sha256::digest(&posture.approved_diff_bytes)));
    assert_ne!(posture.update.update_sha256, posture.proposal.diff_sha256);
    let replay = read.run(store.replay_run(&posture.run_id)).await.expect("canonical CRDT run and lane readback");
    let run = replay.run.inner;
    let lane = replay.lanes.into_iter().find(|lane| lane.lane_id == posture.lane_id)
        .expect("canonical CRDT source lane").inner;
    let payload_json = json!({
        "document_id": document_id, "proposal_id": posture.proposal.proposal_id,
        "approved_diff_sha256": posture.proposal.diff_sha256, "update_sha256": posture.update.update_sha256,
        "update_bytes_ref": posture.update.update_bytes_ref, "base_snapshot_ref": posture.snapshot.snapshot_bytes_ref,
        "state_vector": posture.message.crdt_state_vector,
    });
    let payload_hash = hex::encode(Sha256::digest(serde_json::to_vec(&payload_json).expect("actual CRDT payload JSON")));
    let mut message = posture.message;
    message.payload_sha256 = payload_hash.clone();
    let payload: NewModelLaneContextBundleArtifactBinding = serde_json::from_value(json!({
        "artifact_binding_id": format!("diagnostics-crdt-payload-{label}"), "run_id": run.run_id,
        "trace_id": run.trace_id, "artifact_ref": message.payload_ref,
        "artifact_sha256": payload_hash, "content_hash": payload_hash,
        "artifact_kind": "diagnostics_crdt_payload", "artifact_manifest_ref": format!("artifact-manifest://{label}"),
        "artifact_payload_ref": message.payload_ref, "payload_json": payload_json,
        "event_ledger_stream_id": run.event_ledger_stream_id, "work_packet_id": run.work_packet_id,
        "micro_task_id": "MT-008", "task_board_id": run.task_board_id, "owner_session": run.owner_session,
        "idempotency_key": format!("diagnostics-crdt-payload-{label}"),
        "created_at_utc": chrono::Utc::now().to_rfc3339(), "diagnostic_payload": {},
    })).expect("real hashed CRDT payload binding input");
    let recorded = create.run(store.record_message_with_payload_binding(message.clone(), payload.clone())).await
        .expect("atomic CRDT-bound message and real payload producer");
    let binding = recorded.crdt_authority_binding.as_ref().expect("server-derived canonical CRDT authority binding");
    assert_eq!(binding.document_id, document_id);
    assert_eq!(binding.update_id, posture.update.update_id);
    assert_eq!(binding.update_bytes_ref, posture.update.update_bytes_ref);
    assert_eq!(binding.base_snapshot_ref, posture.snapshot.snapshot_bytes_ref);
    assert_eq!(binding.state_vector, message.crdt_state_vector.as_ref().expect("actual state vector").as_str());
    let metadata = json!({
        "schema_id": "hsk.model_lane_crdt_payload@1", "document_id": binding.document_id,
        "workspace_id": binding.workspace_id, "actor_id": binding.actor_id, "actor_kind": binding.actor_kind,
        "lane_id": binding.lane_id, "crdt_site_id": binding.crdt_site_id, "update_seq": binding.update_seq,
        "update_bytes_ref": binding.update_bytes_ref, "update_sha256": posture.update.update_sha256,
        "state_vector": binding.state_vector, "base_snapshot_ref": binding.base_snapshot_ref,
        "materialized_projection_hash": binding.materialized_projection_hash,
        "replay_metadata": {"format": "yjs_update_v1", "yjs_compatible": true,
            "replay_order_key": posture.update.replay_order_key,
            "schema_version": posture.update.replay_schema_version,
            "dependency_update_ids": posture.update.dependency_update_ids},
        "promotion_gate_ref": format!("promotion-gate://model-lane-message/{}", recorded.message_id),
        "promotion_receipt_ref": null,
        "validation_runner_ref": format!("eventledger://{}", posture.update.event_ledger_event_id),
        "authority_effect": "advisory_only",
    });
    let input = CanonicalDiagnosticsInputs { run, lane, message, payload };
    assert_canonical_navigation_drilldowns(fixture, &input).await;
    let context_id = assert_canonical_context_handoff(fixture, &input, Some(metadata)).await;
    let router = fixture.diagnostics_router();
    assert_public_projection_matches_canonical(fixture, &router, &input.run.run_id).await;
    (input.run.run_id, context_id)
}

async fn assert_canonical_context_handoff(
    fixture: &DiagnosticsFixture, input: &CanonicalDiagnosticsInputs, crdt_metadata: Option<Value>,
) -> String {
    use crate::swarm_orchestration::model_lane::{
        model_lane_context_bundle_id_for_handoff, NewModelLaneContextBundleHandoff,
    };
    let label = Uuid::now_v7().to_string();
    let mut handoff: NewModelLaneContextBundleHandoff = serde_json::from_value(json!({
        "handoff_id": format!("diagnostics-handoff-{label}"), "context_bundle_id": "pending-derived-identity",
        "run_id": input.run.run_id, "trace_id": input.run.trace_id,
        "handoff_span_id": format!("handoff-span-{label}"), "parent_span_id": input.message.message_span_id,
        "linked_span_contexts": [input.lane.lane_span_id],
        "downstream_lane_id": input.lane.lane_id, "source_lane_id": input.lane.lane_id,
        "source_message_id": input.message.message_id, "artifact_ref": input.payload.artifact_ref,
        "artifact_sha256": input.payload.artifact_sha256, "content_hash": input.payload.content_hash,
        "source_kind": "proposal", "authority_state": input.message.authority, "selection_state": "unresolved",
        "reason_code": "advisory-context-awaits-selection", "decision_ref": null, "reviewer_ref": null,
        "replay_hint": "canonical-context-handoff", "crdt_payload": crdt_metadata, "loom_refs": [], "memory_pack_refs": [],
        "event_ledger_stream_id": input.run.event_ledger_stream_id,
        "work_packet_id": input.run.work_packet_id, "micro_task_id": "MT-008",
        "task_board_id": input.run.task_board_id, "owner_session": input.run.owner_session,
        "idempotency_key": format!("diagnostics-handoff-{label}"), "replay_order_key": format!("{label}:handoff"),
        "created_at_utc": chrono::Utc::now().to_rfc3339(), "diagnostic_payload": {},
    })).expect("typed advisory handoff from actual canonical inputs");
    handoff.context_bundle_id = model_lane_context_bundle_id_for_handoff(&handoff)
        .expect("derive actual context identity from the public canonical hash function");
    let create = fixture.authorized_operation(ResourceAction::Create, "fs.write").await;
    let read = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    let recorded = create.run(store.record_context_bundle_handoff(handoff.clone())).await
        .expect("real atomic ContextBundle handoff producer validates source rows and artifact hashes");
    assert_eq!(recorded.inner, handoff);
    let events = fixture.account_events("model_lane_context_bundle_handoff", &recorded.handoff_id).await;
    let matching = events.iter().filter(|event| event.event_id == recorded.event_ledger_event_id).collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].event_sequence, recorded.event_ledger_seq);
    let before = read.run(store.test_scoped_authority_receipts(&input.run.run_id, RECEIPT_LIMIT))
        .await.expect("complete bounded receipt inventory after handoff");
    assert_eq!(create.run(store.record_context_bundle_handoff(handoff)).await.expect("identical handoff retry"), recorded);
    let replayed = read.run(store.replay_context_bundle_handoffs(&input.run.run_id, &recorded.context_bundle_id))
        .await.expect("canonical context replay");
    assert_eq!(replayed, vec![recorded.clone()]);
    let consumed = read.run(store.consume_context_bundle_for_downstream(
        &input.run.run_id, &recorded.context_bundle_id, &input.lane.lane_id,
    )).await.expect("same-scope canonical downstream context read");
    assert_eq!(consumed.records, replayed);
    assert_eq!(consumed.run_id, input.run.run_id);
    assert_eq!(consumed.context_bundle_id, recorded.context_bundle_id);
    assert_eq!(consumed.downstream_lane_id, input.lane.lane_id);
    assert_eq!(consumed.records[0].artifact_ref, input.message.payload_ref);
    assert_eq!(consumed.records[0].artifact_sha256, input.message.payload_sha256);
    let navigation = read.run(store.navigation_by_artifact_or_context(
        None, Some(&recorded.context_bundle_id), Some(&input.run.run_id),
    )).await;
    if recorded.crdt_payload.is_some() {
        let navigation = navigation.expect("genuine CRDT source enables canonical context drilldown");
        assert_eq!(navigation.run.as_ref().expect("context selected run").run_id, input.run.run_id);
        assert_eq!(navigation.context_handoffs.len(), 1);
        assert_eq!(navigation.context_handoffs[0].aggregate_id, recorded.handoff_id);
        assert_eq!(navigation.context_handoffs[0].event_ledger_event_id, recorded.event_ledger_event_id);
        assert_eq!(navigation.context_handoffs[0].event_ledger_seq, recorded.event_ledger_seq);
        assert_eq!(consumed.records[0].crdt_payload, recorded.crdt_payload);
    } else {
        assert!(matches!(navigation, Err(ModelLaneError::NotFound(_))),
            "plain advisory handoff cannot acquire the CRDT source witness required by navigation");
    }
    let after = read.run(store.test_scoped_authority_receipts(&input.run.run_id, RECEIPT_LIMIT))
        .await.expect("retry, replay, consume and navigation do not append authority");
    assert_eq!(receipt_identities(&before), receipt_identities(&after));
    recorded.context_bundle_id.clone()
}

async fn assert_incomplete_legacy_scope_is_absent(fixture: &DiagnosticsFixture) {
    use crate::swarm_orchestration::model_lane::ModelLaneAuthorityTestCorruption;
    let input = CanonicalDiagnosticsInputs::new(fixture);
    record_and_replay_identical_inputs(fixture, &input).await;
    record_diagnostic_posture_and_status(fixture, &input).await;
    let create = fixture.authorized_operation(ResourceAction::Create, "fs.write").await;
    let read = fixture.authorized_operation(ResourceAction::Read, "diagnostics.read").await;
    let store = fixture.model_lane_store();
    read.run(store.diagnostics_projection(&input.run.run_id)).await.expect("complete-scope positive before legacy control");
    create.run(store.test_corrupt_scoped_authority(
        "run", &input.run.run_id, ModelLaneAuthorityTestCorruption::IncompleteAttribution,
    )).await.expect("owned legacy incomplete-scope control");
    assert_scope_denial(read.run(store.diagnostics_projection(&input.run.run_id)).await);
    assert_scope_denial(read.run(store.replay_run(&input.run.run_id)).await);
    let router = fixture.diagnostics_router();
    let existing = fixture.public_diagnostics(&router, Some(&input.run.run_id)).await;
    let missing = fixture.public_diagnostics(&router, Some(&format!("absent-{}", Uuid::new_v4()))).await;
    assert_eq!(existing, missing, "legacy attribution failure cannot disclose run existence");
}

async fn assert_real_recorder_only_event_is_not_authority(fixture: &mut DiagnosticsFixture) {
    use crate::flight_recorder::duckdb::{system_event, DuckDbFlightRecorder};
    let absent = format!("recorder-only-diagnostics-{}", Uuid::new_v4());
    let path = fixture.native_channel.as_ref().expect("owned recorder output")
        .directory_path.join("explicit-recorder-only-negative.duckdb");
    let recorder = Arc::new(DuckDbFlightRecorder::new_on_path(&path, 7)
        .expect("original real recorder-only negative requires its actual engine"));
    fixture.recorders.push(RecorderOwner::new(recorder.clone()));
    let mut event = system_event("explicit recorder-only test input, not a canonical run", "diagnostics-conformance");
    event.job_id = Some(absent.clone());
    event.payload["run_id"] = json!(absent);
    let event_id = event.event_id;
    recorder.record_event(event).await.expect("real production recorder INSERT");
    let actual = recorder.list_events(EventFilter { event_id: Some(event_id), ..EventFilter::default() })
        .await.expect("real recorder readback");
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].event_id, event_id);
    assert_eq!(actual[0].payload["run_id"], absent);
    assert!(fixture.account_events("model_lane_run", &absent).await.is_empty());
    let mut state = fixture.app_state();
    state.flight_recorder = recorder.clone();
    state.diagnostics = recorder;
    fixture.activate_router(state).await;
    let router = fixture.diagnostics_router();
    let response = fixture.public_diagnostics(&router, Some(&absent)).await;
    assert_eq!(response.0, StatusCode::NOT_FOUND);
    assert!(response.1.get("run").is_none(), "real recorder rows cannot supply canonical projection authority");
}
