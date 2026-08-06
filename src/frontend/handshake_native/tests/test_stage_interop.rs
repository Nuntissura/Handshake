//! Bidirectional Editors <-> Stage (Pillar 17) interop proofs — WP-KERNEL-012 MT-066 (cluster E10).
//!
//! This suite proves the full bus-route -> privileged exact-byte capture -> retrieval -> embed round-trip.
//! The route leg remains bus-native; capture and retrieval use the managed handshake_core Stage routes.
//!
//! The default live proof self-seeds a managed workspace and drives the mounted native app and persisted
//! Flight Recorder path; mock tests retain deterministic 404/501 failure coverage.
//!
//! Proof map:
//! - PT-001 / AC-001: `route_payload_from_selection_and_canvas_node` — the Selection + CanvasNode payload
//!   builders produce the correct StageRoutePayload shape (workspace_id, source variant, correlation_id).
//! - PT-002 / AC-002: `route_to_stage_prebuilds_fr_receipt_and_stages_content` — the bus
//!   route prebuilds the MT-036 `route_to_stage` receipt AND stages the content the Stage pane shows;
//!   `live_route_round_trip_real_pg` is the managed-PG mounted round-trip.
//! - PT-003 / AC-003: `embed_back_inserts_mt014_nodeview_with_provenance` — a fetched artifact becomes an
//!   MT-014 `hsLink` embed atom carrying the SHA-256 manifest provenance descriptor.
//! - PT-004 / AC-004: `embed_back_endpoint_absent_404` + `embed_back_endpoint_absent_501` — the missing
//!   route maps to `EmbedBackEndpointAbsent` (the typed blocker) over a mock server (BROAD: 404 AND 501);
//!   no artifact fabricated.
//! - PT-005 / AC-006: `stage_pane_accesskit_nodes_present` — the live AccessKit tree carries `stage-pane`
//!   (GenericContainer), `stage-routed-content` (GenericContainer), and `stage-capture-embed-back` (Button)
//!   with the correct roles + nesting; saves a screenshot to the EXTERNAL artifact root.
//! - AC-005: `single_route_command_id_plus_embed_command` — exactly one route-to-stage command id (extends
//!   MT-033) + the added embed-stage-capture command id (grep gate over the catalog + the bus descriptors).
//! - AC-007: `no_sqlite_and_shared_backend_client` — PostgreSQL authority and one shared HTTP client;
//!   `assert_no_local_artifact_dir` guards artifact hygiene (CX-212E).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
use sha2::Digest;

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::context_menu_surfaces::node_menu_ids;
use handshake_native::graph::placement_author_id;
use handshake_native::interop::{
    build_from_canvas_node, build_from_selection, embed_artifact_as_nodeview, CanvasNodeRef,
    EditorSurfaceKind, InteractionBus, SharedSelection, StageArtifactRef, StageClient,
    StageInteropError, StageManifest, StageRouteSource, CMD_EMBED_STAGE_CAPTURE,
    CMD_ROUTE_TO_STAGE, STAGE_CAPTURE_REF_KIND,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::document_model::{DocPosition, Selection};
use handshake_native::stage_pane::{
    EmbedTarget, StagePane, STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID, STAGE_EMBED_BACK_STATUS_AUTHOR_ID,
    STAGE_PANE_AUTHOR_ID, STAGE_ROUTED_CONTENT_AUTHOR_ID, STAGE_ROUTE_RETRY_AUTHOR_ID,
    STAGE_ROUTE_STATUS_AUTHOR_ID,
};
use handshake_native::tab_bar::TabState;
use handshake_native::theme::HsTheme;

static BINDING_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[path = "interconnect_support/mod.rs"]
mod interconnect_support;
mod stage_binding_proof {
    //! Cross-executable serialization for tests that install the native MCP discovery binding.
    //!
    //! Callers reserve this guard before selecting or starting the managed backend, then publish the mounted
    //! app token after the app exists. The guard holds the product's canonical publication lock for its full
    //! lifetime, so another compliant publisher cannot replace the binding between install and Stage capture.

    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    pub struct StageBindingGuard {
        previous: Option<handshake_native::mcp::McpBinding>,
        installed: Option<handshake_native::mcp::McpBinding>,
        recovered_dead_owner: bool,
        dead_owner_evidence: Option<serde_json::Value>,
        binding_path: PathBuf,
        env_var: &'static str,
        previous_env: Option<std::ffi::OsString>,
        canonical_lock: Option<std::fs::File>,
    }

    impl StageBindingGuard {
        /// Establish the binding root and hold the canonical publication lock. This must happen before the
        /// managed backend is selected so an owned backend inherits the same root and an attached backend's
        /// packet-standard root cannot be concurrently displaced by another proof executable.
        pub fn reserve(scenario: &str) -> Self {
            Self::reserve_inner(scenario)
        }

        fn reserve_inner(scenario: &str) -> Self {
            #[cfg(target_os = "windows")]
            let env_var = "LOCALAPPDATA";
            #[cfg(not(target_os = "windows"))]
            let env_var = "XDG_DATA_HOME";

            let previous_env = std::env::var_os(env_var);
            let binding_root = PathBuf::from(
                std::env::var_os("HANDSHAKE_TEST_STAGE_BINDING_ROOT").expect(
                    "HANDSHAKE_TEST_STAGE_BINDING_ROOT is required; Stage proofs must never publish into live app-data",
                ),
            );
            assert!(
                binding_root.is_absolute(),
                "HANDSHAKE_TEST_STAGE_BINDING_ROOT must be an absolute isolated test root"
            );
            std::fs::create_dir_all(binding_root.join("handshake")).unwrap_or_else(|error| {
                panic!(
                    "create Stage binding root {}: {error}",
                    binding_root.display()
                )
            });
            restrict_directory_to_owner(&binding_root.join("handshake"));
            let binding_path = binding_root
                .join("handshake")
                .join(handshake_native::mcp::BINDING_FILE_NAME);
            let lock_path = binding_path
                .parent()
                .expect("binding path has parent")
                .join("swarm_mcp_binding.lock");
            let canonical_lock = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock_path)
                .unwrap_or_else(|error| {
                    panic!("open canonical Stage lock {}: {error}", lock_path.display())
                });
            let deadline = Instant::now() + Duration::from_secs(120);
            loop {
                match canonical_lock.try_lock() {
                    Ok(()) => break,
                    Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                    Err(error) => panic!(
                        "lock canonical Stage publication file {} within 120 seconds: {error}",
                        lock_path.display()
                    ),
                }
            }

            let dead_owner_evidence =
                (scenario == "mt066-rich-stage").then(|| seed_real_dead_owner(&binding_path));
            let current = read_binding(&binding_path);
            let recovered_dead_owner = current
                .as_ref()
                .is_some_and(|binding| !binding_owner_is_live(binding));
            let previous = match current {
                Some(binding) if !binding_owner_is_live(&binding) => {
                    // A crashed/killed publisher cannot reclaim this binding. Treat it as stale in the
                    // ordinary reserve path too, so teardown removes our replacement instead of restoring
                    // a credential whose recorded owner no longer exists.
                    None
                }
                binding => binding,
            };
            std::env::set_var(env_var, &binding_root);
            assert_eq!(
                handshake_native::mcp::binding_path(),
                binding_path,
                "reserved Stage root must be the product binding root"
            );

            Self {
                previous,
                installed: None,
                recovered_dead_owner,
                dead_owner_evidence,
                binding_path,
                env_var,
                previous_env,
                canonical_lock: Some(canonical_lock),
            }
        }

        pub fn publish(&mut self, session_token: &str) {
            assert!(
                self.installed.is_none(),
                "Stage binding may be published once"
            );
            let installed = handshake_native::mcp::McpBinding::for_current_process(
                "127.0.0.1:1".to_owned(),
                None,
                session_token.to_owned(),
            )
            .expect("current Stage binding process identity");
            publish_locked(&self.binding_path, &installed);
            self.installed = Some(installed.clone());
            assert_eq!(
                read_binding(&self.binding_path),
                Some(installed.clone()),
                "installed Stage binding readback drifted"
            );
        }

        /// Hand publication ownership to a real `SwarmMcpServer`. The isolated app-data root remains
        /// installed for the backend child, but the canonical lock must be released so the production
        /// server can publish its actual localhost endpoint and matching token.
        pub fn release_for_real_server(&mut self) {
            assert!(
                self.previous.is_none(),
                "the isolated Stage proof root must not displace a live MCP binding"
            );
            assert!(
                self.installed.is_none(),
                "a synthetic Stage binding must not precede the real Argus server"
            );
            drop(self.canonical_lock.take());
        }

        pub fn install(session_token: &str, scenario: &str) -> Self {
            let mut guard = Self::reserve(scenario);
            guard.publish(session_token);
            guard
        }

        pub fn recovered_dead_owner(&self) -> bool {
            self.recovered_dead_owner
        }

        pub fn dead_owner_evidence(&self) -> Option<&serde_json::Value> {
            self.dead_owner_evidence.as_ref()
        }
    }

    /// Publish a binding carrying the OS-issued birth identity of a real child process, then terminate
    /// and reap that exact child. The subsequent reserve path must detect this credential as dead-owner
    /// state rather than accepting or restoring it.
    fn seed_real_dead_owner(binding_path: &Path) -> serde_json::Value {
        #[cfg(target_os = "windows")]
        let mut command = {
            use std::os::windows::process::CommandExt as _;
            let mut command = std::process::Command::new("cmd.exe");
            command
                .creation_flags(0x0800_0000)
                .args(["/C", "ping -n 30 127.0.0.1 >NUL"]);
            command
        };
        #[cfg(not(target_os = "windows"))]
        let mut command = {
            let mut command = std::process::Command::new("sh");
            command.args(["-c", "sleep 30"]);
            command
        };
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let mut child = command.spawn().expect("spawn real dead-owner witness");
        let pid = child.id();
        let process_birth = handshake_native::mcp::binding::process_birth_identity(pid)
            .expect("read real child birth identity");
        let binding = handshake_native::mcp::McpBinding {
            tcp_addr: "127.0.0.1:1".to_owned(),
            pipe_name: None,
            token: "mt066-dead-owner-token".to_owned(),
            pid,
            process_birth: process_birth.clone(),
        };
        publish_locked(binding_path, &binding);
        child.kill().expect("terminate real dead-owner witness");
        child.wait().expect("reap real dead-owner witness");
        assert!(
            handshake_native::mcp::binding::process_birth_identity(pid).is_err(),
            "terminated binding owner must no longer be live"
        );
        serde_json::json!({
            "pid": pid,
            "process_birth": process_birth,
            "binding_path": binding_path,
            "owner_live_after_reap": false,
        })
    }

    impl Drop for StageBindingGuard {
        fn drop(&mut self) {
            let already_panicking = std::thread::panicking();
            let cleanup = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Some(installed) = self.installed.as_ref() {
                    let current = read_binding(&self.binding_path);
                    if current.as_ref() == Some(installed) {
                        match self.previous.as_ref() {
                            Some(previous) => publish_locked(&self.binding_path, previous),
                            None => match std::fs::remove_file(&self.binding_path) {
                                Ok(()) => {}
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                                Err(error) => panic!(
                                    "remove scoped Stage binding {}: {error}",
                                    self.binding_path.display()
                                ),
                            },
                        }
                    }
                    assert_eq!(
                        read_binding(&self.binding_path),
                        self.previous,
                        "Stage binding restoration did not reproduce the displaced canonical state"
                    );
                }
            }));

            match self.previous_env.take() {
                Some(value) => std::env::set_var(self.env_var, value),
                None => std::env::remove_var(self.env_var),
            }
            drop(self.canonical_lock.take());
            if cleanup.is_err() && !already_panicking {
                panic!(
                    "Stage binding cleanup failed; environment and publication lock were restored"
                );
            }
        }
    }

    fn read_binding(path: &Path) -> Option<handshake_native::mcp::McpBinding> {
        match std::fs::read(path) {
            Ok(bytes) => {
                Some(serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                    panic!("parse Stage binding {}: {error}", path.display())
                }))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => panic!("read Stage binding {}: {error}", path.display()),
        }
    }

    fn binding_owner_is_live(binding: &handshake_native::mcp::McpBinding) -> bool {
        handshake_native::mcp::binding::process_birth_identity(binding.pid)
            .ok()
            .as_ref()
            == Some(&binding.process_birth)
    }

    fn publish_locked(path: &Path, binding: &handshake_native::mcp::McpBinding) {
        let bytes = serde_json::to_vec_pretty(binding).expect("serialize Stage binding");
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).unwrap_or_else(|error| {
            panic!("create Stage binding temp {}: {error}", temporary.display())
        });
        let mut unpublished = UnpublishedStageBinding::new(temporary.clone());
        restrict_to_owner(&temporary);
        use std::io::Write as _;
        file.write_all(&bytes).unwrap_or_else(|error| {
            panic!("write Stage binding temp {}: {error}", temporary.display())
        });
        file.sync_all().unwrap_or_else(|error| {
            panic!("sync Stage binding temp {}: {error}", temporary.display())
        });
        drop(file);
        #[cfg(target_os = "windows")]
        replace_file(&temporary, path);
        #[cfg(not(target_os = "windows"))]
        std::fs::rename(&temporary, path).unwrap_or_else(|error| {
            panic!(
                "publish Stage binding {} -> {}: {error}",
                temporary.display(),
                path.display()
            )
        });
        unpublished.disarm();
    }

    struct UnpublishedStageBinding {
        path: PathBuf,
        armed: bool,
    }

    impl UnpublishedStageBinding {
        fn new(path: PathBuf) -> Self {
            Self { path, armed: true }
        }

        fn disarm(&mut self) {
            self.armed = false;
        }
    }

    impl Drop for UnpublishedStageBinding {
        fn drop(&mut self) {
            if self.armed {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }

    #[cfg(unix)]
    fn restrict_to_owner(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|error| panic!("restrict Stage binding {}: {error}", path.display()));
    }

    #[cfg(unix)]
    fn restrict_directory_to_owner(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .unwrap_or_else(|error| panic!("restrict Stage directory {}: {error}", path.display()));
        let mode = std::fs::metadata(path)
            .expect("inspect Stage directory")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "Stage directory must remain owner-only");
    }

    #[cfg(target_os = "windows")]
    fn restrict_to_owner(path: &Path) {
        use std::os::windows::process::CommandExt;
        let user = std::env::var("USERNAME").expect("USERNAME for Stage binding ACL");
        let status = std::process::Command::new("icacls")
            .arg(path)
            .arg("/inheritance:r")
            .arg("/grant:r")
            .arg(format!("{user}:F"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x0800_0000)
            .status()
            .unwrap_or_else(|error| panic!("run icacls for {}: {error}", path.display()));
        assert!(status.success(), "icacls rejected {}", path.display());
    }

    #[cfg(target_os = "windows")]
    fn restrict_directory_to_owner(path: &Path) {
        restrict_to_owner(path);
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    fn restrict_to_owner(_path: &Path) {
        panic!("owner-only Stage binding permissions unsupported on this platform");
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    fn restrict_directory_to_owner(_path: &Path) {
        panic!("owner-only Stage binding directories unsupported on this platform");
    }

    #[cfg(target_os = "windows")]
    fn replace_file(from: &Path, to: &Path) {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        extern "system" {
            fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
        }
        let from_wide = from
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let to_wide = to
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: both buffers are NUL-terminated and live for the duration of the call.
        let replaced = unsafe { MoveFileExW(from_wide.as_ptr(), to_wide.as_ptr(), 0x1 | 0x8) != 0 };
        assert!(
            replaced,
            "publish Stage binding {} -> {}: {}",
            from.display(),
            to.display(),
            std::io::Error::last_os_error()
        );
    }
}

#[test]
fn stage_binding_guard_restores_state_and_releases_lock_during_unwind() {
    let _env_lock = BINDING_ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = std::env::temp_dir().join(format!(
        "hsk_stage_binding_guard_unwind_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let previous_root = std::env::var_os("HANDSHAKE_TEST_STAGE_BINDING_ROOT");
    std::env::set_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT", &root);

    let binding_path = root
        .join("handshake")
        .join(handshake_native::mcp::BINDING_FILE_NAME);
    let lock_path = root.join("handshake").join("swarm_mcp_binding.lock");
    let unwind = std::panic::catch_unwind(|| {
        let _binding = stage_binding_proof::StageBindingGuard::install(
            &"f".repeat(64),
            "unwind-cleanup-proof",
        );
        assert!(binding_path.exists(), "test binding must be published");
        panic!("exercise Stage binding guard unwind cleanup");
    });

    match previous_root {
        Some(value) => std::env::set_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT", value),
        None => std::env::remove_var("HANDSHAKE_TEST_STAGE_BINDING_ROOT"),
    }
    assert!(unwind.is_err(), "test must exercise the unwinding path");
    assert!(
        !binding_path.exists(),
        "Stage binding guard must remove its publication while unwinding"
    );
    let temp_residue = std::fs::read_dir(root.join("handshake"))
        .expect("read Stage binding root after unwind")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "tmp"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    assert!(
        temp_residue.is_empty(),
        "Stage binding unwind left temp residue: {temp_residue:?}"
    );
    let released_lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("reopen Stage publication lock");
    released_lock
        .try_lock()
        .expect("Stage binding guard must release publication lock while unwinding");
    drop(released_lock);
    std::fs::remove_dir_all(&root).expect("remove Stage binding unwind test root");
}

struct LiveWorkspaceGuard<'a> {
    backend: &'a mut interconnect_support::LiveBackend,
    workspace_id: String,
    native_fr_event_ids: Vec<String>,
    stage_artifact_ids: Vec<String>,
    stage_job_ids: Vec<String>,
    stage_event_ids: Vec<String>,
    workspace_deleted: bool,
    /// MT-066 V4: which cleanup phases have already run to completion. Drop recovery consults
    /// this so an explicit `finish_and_assert_zero()` panic is not followed by a second identical
    /// panic + backtrace from `Drop`, which previously obscured the primary defect.
    native_fr_cleanup_done: bool,
    stage_side_effect_cleanup_done: bool,
}

impl LiveWorkspaceGuard<'_> {
    fn track_native_fr(&mut self, row: &serde_json::Value) {
        let event_id = row["event_id"]
            .as_str()
            .expect("native FR row carries event_id")
            .to_owned();
        uuid::Uuid::parse_str(&event_id).expect("native FR event_id is a UUID");
        if !self.native_fr_event_ids.contains(&event_id) {
            self.native_fr_event_ids.push(event_id);
        }
        // WP-KERNEL-012 MT-111: MT-109 made the DURABLE Flight Recorder id a workspace-scoped
        // DERIVATION of the client event id, while the EventLedger idempotency keys are still built
        // from the CLIENT id (now prefixed with the workspace). Track the client id too, so
        // `cleanup_native_fr_ledger` can still name the exact ledger rows this proof minted instead of
        // silently deleting nothing and then failing `finish_and_assert_zero`.
        if let Some(client_event_id) = row["payload"]["client_event_id"].as_str() {
            let client_event_id = client_event_id.to_owned();
            if !self.native_fr_event_ids.contains(&client_event_id) {
                self.native_fr_event_ids.push(client_event_id);
            }
        }
    }

    fn track_stage_artifact(&mut self, artifact: &StageArtifactRef) {
        if !self.stage_artifact_ids.contains(&artifact.artifact_id) {
            self.stage_artifact_ids.push(artifact.artifact_id.clone());
        }
        if let Some(job_id) = artifact.job_id.as_ref() {
            if !self.stage_job_ids.contains(job_id) {
                self.stage_job_ids.push(job_id.clone());
            }
        }
        if let Some(event_id) = artifact.event_ledger_event_id.as_ref() {
            if !self.stage_event_ids.contains(event_id) {
                self.stage_event_ids.push(event_id.clone());
            }
        }
    }

    fn cleanup_native_fr_ledger(&mut self) {
        // Discovering event ids over HTTP needs the MT-109 session; the SQL residue cleanup that
        // follows does not. During `Drop` recovery the mounted app may already be gone, taking its
        // published binding with it, so in that case skip only the authorized discovery step rather
        // than abandoning cleanup entirely. The strict `finish_and_assert_zero` path always has a
        // live binding and therefore always performs the read.
        let rows = match try_live_binding_session_token() {
            Ok(session_token) => self.backend.get_json_with_session_token(
                &format!("/api/flight_recorder?wsid={}", self.workspace_id),
                &session_token,
            ),
            Err(reason) if std::thread::panicking() => {
                eprintln!(
                    "MT-066 cleanup skipped authorized Flight Recorder discovery during unwinding \
                     ({reason}); scoped SQL residue cleanup still runs."
                );
                serde_json::Value::Array(Vec::new())
            }
            Err(reason) => panic!(
                "MT-109 capability-gated Flight Recorder read requires the live native-MCP binding: {reason}"
            ),
        };
        for row in rows.as_array().into_iter().flatten() {
            if matches!(
                row["payload"]["kind"].as_str(),
                Some("route_to_stage" | "stage_embed_back")
            ) {
                self.track_native_fr(row);
            }
        }
        if self.native_fr_event_ids.is_empty() {
            return;
        }
        let keys = self
            .native_fr_event_ids
            .iter()
            .flat_map(|event_id| {
                // MT-111: MT-109 partitioned the native-editor EventLedger idempotency keys by
                // workspace (`native-editor-fr-{pending,complete}:{workspace_id}:{client_event_id}`).
                // Both spellings are named so this fixture cleans up rows minted before and after that
                // change; every key is still scoped to THIS proof's own event ids.
                [
                    format!("native-editor-fr-pending:{event_id}"),
                    format!("native-editor-fr-complete:{event_id}"),
                    format!(
                        "native-editor-fr-pending:{}:{event_id}",
                        self.workspace_id
                    ),
                    format!(
                        "native-editor-fr-complete:{}:{event_id}",
                        self.workspace_id
                    ),
                ]
            })
            .map(|key| format!("'{}'", key.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        self.backend.run_fixture_sql(
            "mt066-native-fr-ledger-cleanup",
            &format!(
                "BEGIN; DELETE FROM kernel_event_ledger WHERE idempotency_key IN ({keys}); \
                 DO $native_fr_cleanup$ BEGIN IF EXISTS (SELECT 1 FROM kernel_event_ledger \
                 WHERE idempotency_key IN ({keys})) THEN RAISE EXCEPTION \
                 'MT-066 native FR EventLedger cleanup left fixture rows'; END IF; \
                 END $native_fr_cleanup$; COMMIT;"
            ),
        );
        self.native_fr_event_ids.clear();
    }

    fn sql_text_array(values: &[String]) -> String {
        if values.is_empty() {
            return "ARRAY[]::text[]".to_owned();
        }
        format!(
            "ARRAY[{}]::text[]",
            values
                .iter()
                .map(|value| format!("'{}'", value.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn cleanup_stage_side_effects_and_assert_zero(&mut self) {
        let workspace = self.workspace_id.replace('\'', "''");
        let artifacts = Self::sql_text_array(&self.stage_artifact_ids);
        let jobs = Self::sql_text_array(&self.stage_job_ids);
        let events = Self::sql_text_array(&self.stage_event_ids);
        let detach_ledger_references = Self::DETACH_LEDGER_REFERENCES_SQL;
        self.backend.run_fixture_sql(
            "mt066-stage-side-effect-cleanup",
            &format!(
                "BEGIN; \
                 CREATE TEMP TABLE mt066_stage_cleanup_artifacts ON COMMIT DROP AS \
                 SELECT artifact_id, job_id, event_ledger_event_id \
                 FROM stage_capture_artifacts \
                 WHERE artifact_id = ANY({artifacts}) OR workspace_id = '{workspace}'; \
                 CREATE TEMP TABLE mt066_stage_cleanup_events ON COMMIT DROP AS \
                 SELECT event_id FROM kernel_event_ledger \
                 WHERE event_id = ANY({events}) \
                    OR payload->>'workspace_id' = '{workspace}' \
                    OR idempotency_key LIKE 'stage-capture:{workspace}:%' \
                    OR idempotency_key LIKE 'stage-capture-decision:{workspace}:%' \
                    OR (source_component = 'stage_capture_api' \
                        AND payload->>'workspace_id' = '{workspace}') \
                 UNION SELECT event_ledger_event_id FROM mt066_stage_cleanup_artifacts \
                       WHERE event_ledger_event_id IS NOT NULL \
                 UNION SELECT payload->>'decision_event_id' FROM kernel_event_ledger \
                       WHERE event_id = ANY({events}) \
                          OR event_id IN (SELECT event_ledger_event_id \
                                          FROM mt066_stage_cleanup_artifacts \
                                          WHERE event_ledger_event_id IS NOT NULL); \
                 {detach_ledger_references} \
                 DELETE FROM stage_capture_artifacts \
                 WHERE artifact_id = ANY({artifacts}) OR workspace_id = '{workspace}'; \
                 DELETE FROM kernel_event_ledger \
                 WHERE event_id IN (SELECT event_id FROM mt066_stage_cleanup_events \
                                    WHERE event_id IS NOT NULL); \
                 DELETE FROM ai_jobs \
                 WHERE id = ANY({jobs}) \
                    OR id IN (SELECT job_id FROM mt066_stage_cleanup_artifacts \
                              WHERE job_id IS NOT NULL) \
                    OR (job_inputs::jsonb->>'workspace_id' = '{workspace}'); \
                 DO $stage_zero$ BEGIN \
                 IF EXISTS (SELECT 1 FROM stage_capture_artifacts \
                            WHERE artifact_id = ANY({artifacts}) OR workspace_id = '{workspace}') \
                 THEN RAISE EXCEPTION 'MT-066 Stage artifact cleanup left residue'; END IF; \
                 IF EXISTS (SELECT 1 FROM ai_jobs \
                            WHERE id = ANY({jobs}) \
                               OR job_inputs::jsonb->>'workspace_id' = '{workspace}') \
                 THEN RAISE EXCEPTION 'MT-066 Stage job cleanup left residue'; END IF; \
                 IF EXISTS (SELECT 1 FROM kernel_event_ledger \
                            WHERE event_id IN (SELECT event_id \
                                               FROM mt066_stage_cleanup_events \
                                               WHERE event_id IS NOT NULL) \
                               OR idempotency_key LIKE 'stage-capture:{workspace}:%' \
                               OR idempotency_key LIKE 'stage-capture-decision:{workspace}:%' \
                               OR payload->>'workspace_id' = '{workspace}' \
                               OR (source_component = 'stage_capture_api' \
                                   AND payload->>'workspace_id' = '{workspace}')) \
                 THEN RAISE EXCEPTION 'MT-066 Stage EventLedger cleanup left residue'; END IF; \
                 END $stage_zero$; COMMIT;"
            ),
        );
        self.stage_artifact_ids.clear();
        self.stage_job_ids.clear();
        self.stage_event_ids.clear();
    }

    /// MT-066 V4 remediation item 2: detach EVERY row that references the ledger events we are
    /// about to delete, derived DYNAMICALLY from `pg_constraint` rather than from a hard-coded list.
    ///
    /// The V3 failure deleted `kernel_event_ledger` rows while workspace-owned rows still pointed at
    /// them, so the delete tripped a foreign-key constraint. The validator explicitly warned against
    /// hard-coding only the two constraints observed in those runs
    /// (`loom_canvas_boards.event_ledger_event_id` and `loom_block_knowledge_bridge.index_event_id`).
    /// That warning is well founded: a live schema inspection during this remediation found
    /// **68** foreign keys targeting `kernel_event_ledger`, of which 53 are `RESTRICT` and 14 are
    /// `NO ACTION` — i.e. 67 of them can block a ledger delete. Any literal list would therefore be
    /// incomplete the day it is written and would silently rot as new tables are added.
    ///
    /// This walks `pg_constraint` at runtime, nulls every nullable referencing column, and deletes
    /// rows whose referencing column is NOT NULL (they cannot survive without their event). It stays
    /// scoped to the exact set of events this cleanup is about to remove — never a TRUNCATE, never a
    /// broad source-component delete — because parallel work-packet agents share this PostgreSQL
    /// server.
    ///
    /// Two properties are load-bearing and were BOTH wrong in the first V4 draft:
    ///
    /// 1. `kernel_event_ledger.event_id` is `text`, not `uuid`, and its live values are typed
    ///    `KE-<uuid>` strings. Every one of the 68 referencing columns is therefore also `text`.
    ///    Casting the scoped id array to `uuid[]` made PostgreSQL reject the statement outright with
    ///    `operator does not exist: text = uuid`, and a real `KE-…` id additionally fails
    ///    `invalid input syntax for type uuid`. The comparison must stay in `text`.
    /// 2. The detached set must be the SAME set the delete removes. Scoping the detach to only the
    ///    explicitly tracked ids while the delete also removes workspace-matched and
    ///    idempotency-key-matched events would leave exactly the RESTRICT window the V3 run tripped
    ///    over. It therefore drives `mt066_stage_cleanup_events`, the temp table built immediately
    ///    above it inside the same transaction.
    const DETACH_LEDGER_REFERENCES_SQL: &'static str = "DO $mt066_detach$ \
         DECLARE r RECORD; \
         BEGIN \
           FOR r IN \
             SELECT c.conrelid::regclass::text AS tbl, \
                    a.attname               AS col, \
                    a.attnotnull            AS notnull \
             FROM pg_constraint c \
             JOIN LATERAL unnest(c.conkey) AS k(attnum) ON true \
             JOIN pg_attribute a \
               ON a.attrelid = c.conrelid AND a.attnum = k.attnum \
             WHERE c.contype = 'f' \
               AND c.confrelid = 'kernel_event_ledger'::regclass \
           LOOP \
             IF r.notnull THEN \
               EXECUTE format('DELETE FROM %s WHERE %I IN (SELECT event_id FROM \
                               mt066_stage_cleanup_events WHERE event_id IS NOT NULL)', \
                              r.tbl, r.col); \
             ELSE \
               EXECUTE format('UPDATE %s SET %I = NULL WHERE %I IN (SELECT event_id FROM \
                               mt066_stage_cleanup_events WHERE event_id IS NOT NULL)', \
                              r.tbl, r.col, r.col); \
             END IF; \
           END LOOP; \
         END $mt066_detach$;";

    fn cleanup_all_and_assert_zero(&mut self) {
        if !self.native_fr_cleanup_done {
            self.cleanup_native_fr_ledger();
            self.native_fr_cleanup_done = true;
        }
        if !self.stage_side_effect_cleanup_done {
            self.cleanup_stage_side_effects_and_assert_zero();
            self.stage_side_effect_cleanup_done = true;
        }
    }

    fn delete_workspace_and_assert_absent(&mut self) {
        if self.workspace_deleted {
            return;
        }
        let status = self.backend.delete_workspace(&self.workspace_id);
        assert!(
            (200..300).contains(&status) || status == 404,
            "MT-066 managed workspace cleanup returned {status}"
        );
        match try_live_binding_session_token() {
            Ok(session_token) => {
                let rows = self.backend.get_json_with_session_token(
                    &format!("/api/flight_recorder?wsid={}", self.workspace_id),
                    &session_token,
                );
                assert!(
                    rows.as_array().is_some_and(Vec::is_empty),
                    "workspace DELETE must remove persistent Stage FlightRecorder projections: {rows}"
                );
            }
            // Same rule as `cleanup_native_fr_ledger`: only during unwinding, when the mounted app
            // has already taken its binding away, is the authorized readback skipped. The workspace
            // DELETE above still happened; only its FR readback assertion is unavailable.
            Err(reason) if std::thread::panicking() => eprintln!(
                "MT-066 cleanup skipped the authorized Flight Recorder readback assertion during \
                 unwinding ({reason}); the workspace DELETE itself still ran."
            ),
            Err(reason) => panic!(
                "MT-109 capability-gated Flight Recorder read requires the live native-MCP binding: {reason}"
            ),
        }
        self.workspace_deleted = true;
    }

    /// MT-066 V4 remediation item 4: canonical entity cleanup FIRST, then Stage/FR residue, then
    /// the read-only zero assertions.
    ///
    /// The V3 ordering ran the explicit SQL residue cleanup before the workspace DELETE, so it tried
    /// to remove `kernel_event_ledger` rows while workspace-owned rows (Canvas boards, knowledge
    /// bridge rows, and 65 other referencing columns) still pointed at them under RESTRICT. The
    /// product's own workspace DELETE API is the authoritative cascade boundary, so it goes first and
    /// is allowed to own everything it owns; only the residue the API does NOT own is then removed
    /// explicitly.
    fn finish_and_assert_zero(&mut self) {
        self.delete_workspace_and_assert_absent();
        self.cleanup_all_and_assert_zero();
    }
}

impl Drop for LiveWorkspaceGuard<'_> {
    /// MT-066 V4 remediation item 5: bounded, idempotent, NON-DUPLICATIVE recovery.
    ///
    /// Previously `Drop` unconditionally re-ran both cleanup phases. When `finish_and_assert_zero()`
    /// had already panicked, `Drop` re-ran the same failing SQL and emitted a SECOND identical panic
    /// and backtrace, which buried the primary defect under its own echo. Now each phase is guarded
    /// by the completion flags, so `Drop` performs a best-effort recovery only for phases that never
    /// ran, and it orders the workspace DELETE first for the same foreign-key reason as
    /// `finish_and_assert_zero`. A deterministic diagnostic is preserved either way.
    fn drop(&mut self) {
        let already_panicking = std::thread::panicking();

        let workspace_cleanup_result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.delete_workspace_and_assert_absent();
            }));
        let cleanup_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.cleanup_all_and_assert_zero();
        }));

        if already_panicking {
            // The primary failure is already unwinding. Do not raise a second panic on top of it;
            // emit a deterministic diagnostic so the recovery attempt is still visible in the log.
            if workspace_cleanup_result.is_err() || cleanup_result.is_err() {
                eprintln!(
                    "MT-066 LiveWorkspaceGuard drop-recovery could not fully clean workspace {} \
                     (workspace_delete_ok={}, residue_cleanup_ok={}); the ORIGINAL panic above is \
                     the primary defect and is not masked by this recovery attempt.",
                    self.workspace_id,
                    workspace_cleanup_result.is_ok(),
                    cleanup_result.is_ok()
                );
            }
            return;
        }

        if let Err(payload) = workspace_cleanup_result {
            std::panic::resume_unwind(payload);
        }
        if let Err(payload) = cleanup_result {
            std::panic::resume_unwind(payload);
        }
    }
}

/// WP-KERNEL-012 MT-109 put fail-closed capability middleware over the ENTIRE Flight Recorder route
/// group: `GET /api/flight_recorder` now requires a live native-MCP binding token whose recorded
/// owner is still the process the OS says it is, plus the `fr.read` capability.
///
/// These proofs already publish a REAL binding (`stage_binding_proof`), so every recorder read here
/// presents that exact on-disk credential — the same `x-hsk-session-token` the mounted native client
/// sends. Nothing about the authorization is weakened, bypassed, feature-gated, or stubbed: a
/// missing, forged, or stale binding still fails closed with `HSK-401-FR-SESSION`.
fn live_binding_session_token() -> String {
    try_live_binding_session_token().unwrap_or_else(|reason| {
        panic!("MT-109 capability-gated Flight Recorder read requires the live native-MCP binding: {reason}")
    })
}

/// Fallible twin of [`live_binding_session_token`]. The strict proof path uses the panicking form;
/// `Drop` recovery uses this one, because by the time a guard drops during unwinding the mounted app
/// (and therefore its published binding) may already be gone. Recovery must not be BLOCKED by that —
/// but it must also never invent a credential, so a missing binding simply skips the authorized HTTP
/// steps and leaves the SQL residue cleanup, which needs no session, to do its work.
fn try_live_binding_session_token() -> Result<String, String> {
    let path = handshake_native::mcp::binding_path();
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("read native-MCP binding {}: {error}", path.display()))?;
    let binding: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse native-MCP binding {}: {error}", path.display()))?;
    binding["token"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("native-MCP binding {} carries no token", path.display()))
}

/// Does this Flight Recorder row belong to the producer event the mounted app minted?
///
/// WP-KERNEL-012 MT-109 made the DURABLE recorder `event_id` a workspace-scoped DERIVATION of the
/// client event id — deliberately, so one workspace cannot pre-seed, read back, or reconcile another
/// workspace's row by guessing a client id. The id the mounted app actually minted now travels in
/// `payload.client_event_id`.
///
/// A proof holding a client-side receipt id must therefore compare against BOTH spellings. Comparing
/// only `event_id` is how this proof came to report "Flight Recorder row did not arrive" for rows
/// that had in fact arrived and been authorized (`fr.ingest.native_editor` -> `allow`). The legacy
/// `event_id` comparison is retained so rows minted before MT-109 still match.
fn fr_row_matches_producer_event(row: &serde_json::Value, producer_event_id: &str) -> bool {
    row["payload"]["client_event_id"].as_str() == Some(producer_event_id)
        || row["event_id"].as_str() == Some(producer_event_id)
}

fn wait_for_native_fr(
    backend: &interconnect_support::LiveBackend,
    workspace_id: &str,
    kind: &str,
    matches_fixture: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let rows = backend.get_json_with_session_token(
            &format!("/api/flight_recorder?wsid={workspace_id}"),
            &live_binding_session_token(),
        );
        if let Some(row) = rows.as_array().and_then(|rows| {
            rows.iter()
                .find(|row| row["payload"]["kind"].as_str() == Some(kind) && matches_fixture(row))
        }) {
            assert!(row["event_id"].as_str().is_some());
            assert_eq!(row["payload"]["workspace_id"].as_str(), Some(workspace_id));
            return row.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "automatic {kind} Flight Recorder row did not arrive within five seconds. \
             Recorder rows currently visible for this workspace: {rows}. \
             (If a {kind} row IS present above, the emit succeeded and the fixture predicate is \
             what failed — MT-109 made the durable FR `event_id` a workspace-scoped DERIVATION of \
             the client id, so matching on a client-side receipt id no longer works.)"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Artifact hygiene (CX-212E / SCREENSHOT RULE): all artifacts go to the EXTERNAL root ONLY.
// ════════════════════════════════════════════════════════════════════════════════════════════════

fn external_artifact_dir(subdir: &str) -> PathBuf {
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .expect("native crate must live below a worktree root")
                .join("Handshake_Artifacts")
        });
    assert!(
        root.is_absolute(),
        "HANDSHAKE_ARTIFACTS_ROOT must resolve to an absolute path"
    );
    root.join("handshake-test").join(subdir)
}

const MT066_RELEVANT_SOURCE_PATHS: &[&str] = &[
    "src/frontend/handshake_native/src/app.rs",
    "src/frontend/handshake_native/src/interop/stage_interop.rs",
    "src/frontend/handshake_native/src/manual_content_editors.rs",
    "src/frontend/handshake_native/src/project_tree.rs",
    "src/frontend/handshake_native/src/stage_pane.rs",
    "src/frontend/handshake_native/tests/interconnect_support/mod.rs",
    "src/frontend/handshake_native/tests/native_gui_support/canonical_argus_driver.rs",
    "src/frontend/handshake_native/tests/native_gui_support/screenshot_harness.rs",
    "src/frontend/handshake_native/tests/native_gui_support/screenshot_marker.rs",
    "src/frontend/handshake_native/tests/pg_proof_support/mod.rs",
    "src/frontend/handshake_native/tests/test_manual_content.rs",
    "src/frontend/handshake_native/tests/test_stage_interop.rs",
];

fn current_source_sha() -> String {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate must live at repo/src/frontend/handshake_native");
    let clean = std::process::Command::new("git")
        .args(["diff", "--quiet", "HEAD", "--"])
        .args(MT066_RELEVANT_SOURCE_PATHS)
        .current_dir(repo_root)
        .status()
        .expect("check MT-066 relevant source cleanliness");
    assert!(
        clean.success(),
        "MT-066 canonical proof refuses dirty relevant source; commit implementation and proof first"
    );
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .expect("resolve current source hash");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("source hash UTF-8")
        .trim()
        .to_owned()
}

fn current_runtime_source_tree() -> String {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate must live at repo/src/frontend/handshake_native");
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .current_dir(repo_root)
        .output()
        .expect("inspect complete MT-066 runtime source cleanliness");
    assert!(status.status.success());
    let unexpected = String::from_utf8(status.stdout)
        .expect("git status UTF-8")
        .lines()
        .filter(|line| {
            let path = line.get(3..).unwrap_or_default();
            !matches!(path, "AGENTS.md" | "CLAUDE.md")
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(
        unexpected.is_empty(),
        "MT-066 canonical proof refuses dirty/untracked transitive runtime source outside the known authority files: {unexpected:?}"
    );
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .current_dir(repo_root)
        .output()
        .expect("resolve complete committed runtime source tree");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("runtime source tree UTF-8")
        .trim()
        .to_owned()
}

fn current_proof_source_blob() -> String {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate must live at repo/src/frontend/handshake_native");
    let output = std::process::Command::new("git")
        .args([
            "rev-parse",
            "HEAD:src/frontend/handshake_native/tests/test_stage_interop.rs",
        ])
        .current_dir(repo_root)
        .output()
        .expect("resolve committed MT-066 proof blob");
    assert!(
        output.status.success(),
        "resolve committed MT-066 proof blob: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("proof blob UTF-8")
        .trim()
        .to_owned()
}

fn current_proof_source_blobs() -> serde_json::Map<String, serde_json::Value> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("native crate must live at repo/src/frontend/handshake_native");
    MT066_RELEVANT_SOURCE_PATHS
        .iter()
        .map(|path| {
            let spec = format!("HEAD:{path}");
            let output = std::process::Command::new("git")
                .args(["rev-parse", &spec])
                .current_dir(repo_root)
                .output()
                .unwrap_or_else(|error| {
                    panic!("resolve committed MT-066 source blob {path}: {error}")
                });
            assert!(
                output.status.success(),
                "resolve committed MT-066 source blob {path}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let blob = String::from_utf8(output.stdout)
                .expect("source blob UTF-8")
                .trim()
                .to_owned();
            (path.to_string(), serde_json::Value::String(blob))
        })
        .collect()
}

/// Assert NO repo-local artifact directory exists under the crate (the SCREENSHOT/TEST-ARTIFACT RULE).
/// Artifacts go to the external `Handshake_Artifacts/handshake-test` root ONLY; a stray `test_output/`
/// OR `tests/screenshots/` is a hygiene FAILURE.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// In-process mock HTTP server (the PROVEN MT-020/MT-037/MT-063 TcpListener pattern — no new dependency).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request, and
/// captures that request's line. Returns (base_url, join handle delivering the request line).
fn spawn_mock(
    status_line: &'static str,
    body: serde_json::Value,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request_line = read_request_line(&mut stream);
        let body_str = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
            body_str.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        request_line
    });
    (base_url, handle)
}

/// Read one HTTP request's request line off the stream (a GET has no body).
fn read_request_line(stream: &mut std::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if String::from_utf8_lossy(&buf).contains("\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    text.lines().next().unwrap_or("").to_string()
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

fn dark() -> handshake_native::theme::HsPalette {
    HsTheme::Dark.palette()
}

fn text_range(pane_id: &str, start: usize, end: usize, text: &str) -> SharedSelection {
    SharedSelection::TextRange {
        pane_id: std::sync::Arc::from(pane_id),
        surface: EditorSurfaceKind::RichText,
        start,
        end,
        text: text.to_owned(),
    }
}

fn evidence_artifact(id: &str) -> StageArtifactRef {
    let content_bytes = b"stage-capture-fixture".to_vec();
    let sha = format!("{:x}", sha2::Sha256::digest(&content_bytes));
    StageArtifactRef {
        artifact_id: id.to_owned(),
        workspace_id: "WS-1".to_owned(),
        sha256: sha.clone(),
        manifest: StageManifest {
            sha256: sha,
            manifest_ref: format!("manifest://{id}"),
            content_type: "image/png".to_owned(),
            size_bytes: content_bytes.len() as u64,
        },
        label: "Capture".to_owned(),
        content_path: String::new(),
        size_bytes: content_bytes.len() as u64,
        correlation_id: "stage-fixture-correlation".to_owned(),
        job_id: None,
        event_ledger_event_id: None,
        replayed: false,
        content_bytes,
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-001 / AC-001 — the route-leg payload builders (selection + canvas node).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn route_payload_from_selection_and_canvas_node() {
    // A TextRange selection -> Selection source with the materialized text + a source ref.
    let sel = text_range("pane-rich", 4, 17, "route this span");
    let payload = build_from_selection(&sel, "WS-1").expect("AC-001: selection payload builds");
    assert_eq!(payload.workspace_id, "WS-1");
    assert_eq!(payload.content_kind(), "selection");
    match &payload.source {
        StageRouteSource::Selection {
            source_pane_id,
            text,
            source_ref,
            ..
        } => {
            assert_eq!(source_pane_id, "pane-rich");
            assert_eq!(text, "route this span");
            assert_eq!(source_ref, "pane-rich:4-17");
        }
        other => panic!("AC-001: expected Selection source, got {other:?}"),
    }
    assert_eq!(payload.correlation_id, "stage-route-sel-pane-rich-4-17");

    // A canvas node -> CanvasNode source.
    let node = CanvasNodeRef {
        workspace_id: "WS-1".to_owned(),
        canvas_id: "CB-1".to_owned(),
        node_id: "N-9".to_owned(),
        node_kind: "loom_block".to_owned(),
        pane_id: "pane-canvas".to_owned(),
    };
    let cpayload = build_from_canvas_node(&node).expect("AC-001: canvas-node payload builds");
    assert_eq!(cpayload.content_kind(), "canvas_node");
    match &cpayload.source {
        StageRouteSource::CanvasNode {
            canvas_id,
            node_id,
            node_kind,
            ..
        } => {
            assert_eq!(canvas_id, "CB-1");
            assert_eq!(node_id, "N-9");
            assert_eq!(node_kind, "loom_block");
        }
        other => panic!("AC-001: expected CanvasNode source, got {other:?}"),
    }
    println!(
        "PT-001 payload builders OK: selection corr={} | canvas corr={}",
        payload.correlation_id, cpayload.correlation_id
    );
}

#[test]
fn route_contention_retains_capture_causal_attribution() {
    let mut pane = StagePane::new();
    let content = handshake_native::stage_pane::StageContent::Selection(
        "retained exact bytes".to_owned(),
        "pane-rich:0-20".to_owned(),
    );
    let route = handshake_native::interop::PendingStageRoute::new(
        content.clone(),
        "selection",
        Some("causal-stage-77".to_owned()),
        "pane-rich",
        "WS-1",
    );
    pane.set_route_busy(route.clone());
    assert_eq!(pane.route_retry, Some(route));
    let retained = pane.route_retry.as_ref().expect("retained Stage route");
    let request = handshake_native::interop::StageCaptureRequest::from_routed_content(
        &retained.content,
        retained
            .causal_action_id
            .as_deref()
            .expect("retained causal action id"),
    )
    .expect("retained route remains capturable");
    assert_eq!(request.correlation_id, "causal-stage-77");
    assert!(request.idempotency_key.starts_with("stage-capture:"));

    let changed = handshake_native::interop::StageCaptureRequest::from_routed_content(
        &handshake_native::stage_pane::StageContent::Selection(
            "changed bytes at same source".to_owned(),
            "pane-rich:0-20".to_owned(),
        ),
        "causal-stage-77",
    )
    .expect("changed routed content remains capturable");
    assert_ne!(
        request.idempotency_key, changed.idempotency_key,
        "exact-byte digest prevents a stable source correlation from conflicting after content changes"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-002 / AC-002 — route admission prebuilds the MT-036 receipt (shape) + stages content.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn route_to_stage_prebuilds_fr_receipt_and_stages_content() {
    use handshake_native::event_emitter::NativeEditorEvent;

    // The receipt the bus prebuilds is the EXACT MT-036 constructor (no new event kind, MC-005).
    // Prove its shape directly; the mounted shell emits this same identity only after Stage applies it.
    let ev = NativeEditorEvent::route_to_stage(
        "selection",
        "pane-rich",
        handshake_native::event_emitter::native_editor_actor_id("pane-rich"),
        "WS-1",
    );
    let native = ev.to_native_payload();
    assert_eq!(
        native["action"], "route_to_stage",
        "MC-005: the canonical MT-036 event kind"
    );
    assert_eq!(
        native["pane_id"], "pane-rich",
        "the source pane is the typed pane_id"
    );
    assert_eq!(
        native["payload"]["content_kind"], "selection",
        "content_kind travels in the payload"
    );
    assert_eq!(native["workspace_id"], "WS-1");

    // The bus route_to_stage stages the routed content and dispatches the
    // EXISTING CMD_ROUTE_TO_STAGE (the MT-033 command — extended, not duplicated). Run inside an egui ctx
    // so the dispatch path (which requests a repaint) has a context.
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        let mut bus = InteractionBus::new();
        bus.register_route_to_stage_command();
        let payload =
            build_from_selection(&text_range("pane-rich", 0, 5, "hello"), "WS-1").unwrap();
        let ack = handshake_native::interop::route_to_stage(ctx, &mut bus, &payload)
            .expect("route succeeds (bus-only, no backend POST)");
        assert!(
            ack.staged,
            "AC-002: the routed content was staged on the bus"
        );
        assert_eq!(ack.content_kind, "selection");
        // The complete pending route carries the Selection the Stage pane will render.
        let staged = bus
            .pending_stage_route()
            .expect("complete route staged for the Stage pane drain");
        match &staged.content {
            handshake_native::stage_pane::StageContent::Selection(text, src) => {
                assert_eq!(text, "hello");
                assert_eq!(src, "pane-rich:0-5");
            }
            other => panic!("AC-002: expected a Selection staged, got {other:?}"),
        }
    });

    // The Stage pane receives + renders the routed content (receive_routed_content) — the route-leg landing.
    let mut pane = StagePane::new();
    pane.receive_routed_content(handshake_native::stage_pane::StageContent::Selection(
        "hello".to_owned(),
        "pane-rich:0-5".to_owned(),
    ));
    assert!(
        pane.content.is_some(),
        "AC-002: the Stage pane shows the routed content"
    );
    assert!(pane.content.summary().contains("hello"));
    println!("PT-002 FR-shape + route wiring OK: receipt shape proven, content staged + received");
}

/// AC-002/003 LIVE round-trip against the managed PostgreSQL/EventLedger authority. The fixture creates
/// and removes its own workspace, so this proof has no operator-seeded workspace or `#[ignore]` gate.
#[test]
fn live_route_round_trip_real_pg() {
    let _binding_env_guard = BINDING_ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    use sha2::{Digest, Sha256};

    let mut stage_binding = stage_binding_proof::StageBindingGuard::reserve("mt066-rich-stage");
    assert!(
        stage_binding.recovered_dead_owner(),
        "MT-066 proof must recover a binding owned by a real reaped process"
    );
    let dead_owner_evidence = stage_binding
        .dead_owner_evidence()
        .expect("real dead-owner evidence")
        .clone();
    let source_sha = current_source_sha();
    let runtime_source_tree = current_runtime_source_tree();
    let proof_source_blob = current_proof_source_blob();
    let proof_source_blobs = current_proof_source_blobs();
    let artifact_dir = external_artifact_dir(&format!(
        "wp-kernel-012-mt-066/canonical-argus/run-{}-{}",
        &source_sha[..12],
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&artifact_dir)
        .expect("create external MT-066 canonical Argus artifact directory");
    let mut backend = interconnect_support::require_reachable_backend();
    let workspace = backend.create_workspace(&format!(
        "mt066-live-stage-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceGuard {
        backend: &mut backend,
        workspace_id: workspace_id.clone(),
        native_fr_event_ids: Vec::new(),
        stage_artifact_ids: Vec::new(),
        stage_job_ids: Vec::new(),
        stage_event_ids: Vec::new(),
        workspace_deleted: false,
        native_fr_cleanup_done: false,
        stage_side_effect_cleanup_done: false,
    };

    let exact_text = "MT-066 exact Stage bytes: café / LF\nsecond line";
    let expected_sha = format!("{:x}", Sha256::digest(exact_text.as_bytes()));

    // Fixture setup creates a real target document, but all feature actions below are driven through
    // the mounted production app. In particular, the test never constructs or POSTs an FR event.
    let created_doc = cleanup.backend.post_json(
        "/knowledge/documents",
        &serde_json::json!({
            "workspace_id": workspace_id,
            "title": "MT-066 mounted Stage embed target",
            "content_json": {"type":"doc","content":[{"type":"paragraph","content":[
                {"type":"text","text": exact_text}
            ]}]},
        }),
    );
    let document_id = created_doc["document"]["rich_document_id"]
        .as_str()
        .or_else(|| created_doc["rich_document_id"].as_str())
        .expect("target document create returns rich_document_id")
        .to_owned();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("MT-066 mounted runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&cleanup.backend.base, runtime.handle().clone());
    app.set_stage_embed_back_base_url_for_test(&cleanup.backend.base);
    app.bind_active_project_for_integration_test(workspace_id.clone());
    let pane_id = PaneId::from("pane-a");
    app.pane_registry().lock().unwrap().insert(PaneRecord::new(
        pane_id.clone(),
        PaneType::LoomWikiPage,
        workspace_id.clone(),
        Some(document_id.clone()),
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    let mut tab = TabState::new(PaneType::LoomWikiPage);
    tab.content_id = Some(document_id.clone());
    let bar = app
        .tab_bar_states_mut()
        .get_mut(&pane_id)
        .expect("default pane-a has a tab bar");
    bar.tabs = vec![tab];
    bar.active_index = 0;
    app.set_active_pane_for_test(Some(pane_id.clone()));
    let rich_state = app.mounted_rich_state();
    let stage = app.mounted_stage();
    stage_binding.release_for_real_server();
    let mut argus =
        CanonicalArgusDriver::bind_in_current_app_data(&app, "mt066-stage", app.mcp_token());
    let host_ctx = std::sync::Arc::new(std::sync::Mutex::new(None::<egui::Context>));
    let host_ctx_capture = std::sync::Arc::clone(&host_ctx);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_state(
            move |ctx, app: &mut HandshakeApp| {
                *host_ctx_capture
                    .lock()
                    .expect("capture MT-066 host context") = Some(ctx.clone());
                app.ui(ctx);
            },
            app,
        );

    // Frames load the real target note and bind the workspace-scoped NativeEditorEventEmitter to the
    // shared InteractionBus before the operator route begins.
    let mount_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        if rich_state.lock().unwrap().save.is_some() {
            break;
        }
        assert!(
            std::time::Instant::now() < mount_deadline,
            "mounted rich target did not finish loading within five seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // Select exact bytes in the mounted production editor, then dispatch the original shared
    // Route-to-Stage operator command exactly once. No Stage retry state is synthesized by the proof.
    {
        let mut state = rich_state.lock().unwrap();
        state.selection = Selection::text(
            DocPosition::new(vec![0, 0], 0),
            DocPosition::new(vec![0, 0], exact_text.chars().count()),
        );
        assert_eq!(
            state.selected_text().map(|(_, _, _, text)| text),
            Some(exact_text.to_owned()),
            "the mounted rich selection materializes the exact Stage bytes"
        );
    }
    harness
        .state_mut()
        .set_active_pane_for_test(Some(pane_id.clone()));
    let open_editors = argus.click_and_reinspect(&mut harness, "menu-editors");
    assert!(
        json_has_author_id(&open_editors.after, "menu.editors.route-to-stage"),
        "fresh Argus inspection observes the mounted Route selection to Stage leaf"
    );
    // WP-KERNEL-012 MT-027 V5 (commit d896bbd9) tightened the SHARED canonical Argus contract: at
    // `finish()` every dispatched action must be rebound to an authoritative terminal snapshot AND
    // carry at least one passing action-specific terminal predicate. MT-066's actions predate that
    // contract, so each one below now binds its own predicate. This strengthens the proof — the
    // predicate is recomputed against the terminal tree, it is never a pass flag.
    argus.assert_latest_terminal_predicate(
        &mut harness,
        "mt066-editors-menu-exposes-route-to-stage",
        |after| json_has_author_id(after, "menu.editors.route-to-stage"),
    );
    let ctx = host_ctx
        .lock()
        .expect("MT-066 host context lock")
        .clone()
        .expect("MT-066 host context captured");
    let interaction_bus = InteractionBus::get_or_init(&ctx);
    let blocker_causal_action_id = format!("mt066-route-blocker-{}", uuid::Uuid::new_v4().simple());
    let blocking_route = InteractionBus::with_try_lock(&interaction_bus, |bus| {
        bus.register_route_to_stage_command();
        assert!(
            bus.route_to_stage_correlated(
                &ctx,
                handshake_native::stage_pane::StageContent::Selection(
                    "MT-066 occupied route witness".to_owned(),
                    document_id.clone(),
                ),
                Some(&blocker_causal_action_id),
            ),
            "admit real canonical occupied route witness"
        );
        bus.pending_stage_route()
            .expect("occupied canonical route remains pending")
            .clone()
    })
    .expect("acquire canonical bus to install occupied route witness");
    stage
        .lock()
        .unwrap()
        .retain_route_receipt(blocking_route.receipt.clone());
    let busy_observation = argus.click_expect_typed_rejected_and_reinspect(
        &mut harness,
        "menu.editors.route-to-stage",
        "Route to Stage is busy",
    );
    assert_eq!(
        busy_observation.receipt_status, "rejected",
        "the occupied canonical route returns the causally bound typed busy rejection"
    );
    let busy_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        if stage.lock().unwrap().has_route_retry() {
            break;
        }
        assert!(
            std::time::Instant::now() < busy_deadline,
            "contended canonical route click did not reach retained busy state"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let busy_inspect = argus.assert_latest_terminal_predicate(
        &mut harness,
        "mt066-route-busy-retains-status-and-retry",
        |after| {
            json_has_author_id(after, STAGE_ROUTE_STATUS_AUTHOR_ID)
                && json_has_author_id(after, STAGE_ROUTE_RETRY_AUTHOR_ID)
        },
    );
    let busy_stage_snapshot = stage.lock().unwrap().clone();
    assert!(
        json_has_author_id(&busy_inspect, STAGE_ROUTE_STATUS_AUTHOR_ID)
            && json_has_author_id(&busy_inspect, STAGE_ROUTE_RETRY_AUTHOR_ID),
        "fresh canonical Argus inspection exposes both retained busy status and retry control; stage={busy_stage_snapshot:?}; inspect={busy_inspect}"
    );
    let retained_route = busy_stage_snapshot
        .route_retry
        .clone()
        .expect("contended route is retained");
    let retained_causal_action_id = retained_route
        .causal_action_id
        .clone()
        .expect("retained route carries immutable causal action id");
    let retained_route_event_id = retained_route.receipt.event_id.clone();
    let before_retry_rows = cleanup.backend.get_json_with_session_token(
        &format!("/api/flight_recorder?wsid={workspace_id}"),
        &live_binding_session_token(),
    );
    assert!(
        before_retry_rows
            .as_array()
            .is_some_and(|rows| rows.iter().all(|row| {
                row["payload"]["native_payload"]["causal_action_id"].as_str()
                    != Some(retained_causal_action_id.as_str())
            })),
        "bus contention must not fabricate a route Flight Recorder row"
    );
    let busy_screenshot_path = artifact_dir.join("mt066-stage-busy-harness-render.png");
    harness
        .render()
        .expect("MT-066 busy state requires a material harness render")
        .save(&busy_screenshot_path)
        .expect("save MT-066 busy harness render");
    stage
        .lock()
        .unwrap()
        .acknowledge_route_receipt(&blocking_route.receipt.event_id);
    let removed_blocker = InteractionBus::with_try_lock(&interaction_bus, |bus| {
        bus.ack_pending_stage_route(&blocking_route.receipt.event_id)
    })
    .flatten()
    .expect("remove exact occupied route witness before operator retry");
    assert_eq!(
        removed_blocker.receipt.event_id, blocking_route.receipt.event_id,
        "only the exact occupied route witness is removed"
    );

    let route_observation = argus.click_and_reinspect(&mut harness, STAGE_ROUTE_RETRY_AUTHOR_ID);
    assert!(
        !route_observation.receipt_status.is_empty(),
        "canonical Argus returns the retained-route retry receipt"
    );

    let route_surface_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let received = stage.lock().unwrap().content.clone();
        if matches!(
            received,
            handshake_native::stage_pane::StageContent::Selection(ref text, ref source)
                if text == exact_text && source == &document_id
        ) {
            break;
        }
        assert!(
            std::time::Instant::now() < route_surface_deadline,
            "mounted Stage pane did not receive exact routed bytes within five seconds"
        );
    }
    let route_row = wait_for_native_fr(&*cleanup.backend, &workspace_id, "route_to_stage", |row| {
        fr_row_matches_producer_event(row, retained_route_event_id.as_str())
            && row["payload"]["native_payload"]["content_kind"].as_str() == Some("selection")
    });
    cleanup.track_native_fr(&route_row);
    let route_rows_before_restart = cleanup.backend.get_json_with_session_token(
        &format!("/api/flight_recorder?wsid={workspace_id}"),
        &live_binding_session_token(),
    );
    let route_dispatches_before_restart = route_rows_before_restart
        .as_array()
        .expect("pre-restart Flight Recorder rows")
        .iter()
        .filter(|row| {
            row["payload"]["kind"].as_str() == Some("route_to_stage")
                && row["payload"]["native_payload"]["causal_action_id"].as_str()
                    == Some(retained_causal_action_id.as_str())
        })
        .count();
    assert_eq!(
        route_dispatches_before_restart, 1,
        "the mounted rich selection dispatches the shared Route-to-Stage command exactly once before restart"
    );
    assert_eq!(
        route_row["payload"]["native_payload"]["content_kind"].as_str(),
        Some("selection")
    );
    assert_eq!(
        route_row["payload"]["native_payload"]["causal_action_id"].as_str(),
        Some(retained_causal_action_id.as_str()),
        "route FR carries the mounted command's exact Stage correlation"
    );
    assert!(
        fr_row_matches_producer_event(&route_row, retained_route_event_id.as_str()),
        "route FR preserves the exact retained producer EventLedger identity (MT-109: the client id \
         lives in payload.client_event_id and the durable event_id is its workspace-scoped \
         derivation); row={route_row}"
    );
    assert_eq!(
        stage.lock().unwrap().causal_action_id.as_deref(),
        Some(retained_causal_action_id.as_str()),
        "retry preserves the retained route's exact causal action id"
    );
    let route_recovered_inspect = argus.assert_latest_terminal_predicate(
        &mut harness,
        "mt066-route-retry-renders-routed-content",
        |after| json_has_author_id(after, STAGE_ROUTED_CONTENT_AUTHOR_ID),
    );
    assert!(
        json_has_author_id(&route_recovered_inspect, STAGE_ROUTED_CONTENT_AUTHOR_ID),
        "fresh canonical Argus inspection observes routed content after retry"
    );

    // The canonical AccessKit button remains visible and collision-free on the mounted Stage surface.
    // This note-target live proof drives the equivalent operator-facing palette command; the mounted
    // Canvas live proof below drives the in-pane button itself, while `embed_back_button_press_signals_host`
    // independently proves that button emits the same host request.
    harness.run_steps(1);
    assert_eq!(
        harness
            .query_all_by(|node: &egui_kittest::kittest::AccessKitNode<'_>| {
                node.author_id() == Some(STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID)
            })
            .count(),
        1,
        "mounted Stage embed-back control must be collision-free"
    );
    let (absent_base, absent_request) = spawn_mock(
        "HTTP/1.1 404 Not Found",
        serde_json::json!({"error":"mt066_stage_endpoint_absent"}),
    );
    harness
        .state_mut()
        .set_stage_embed_back_base_url_for_test(&absent_base);
    let error_observation =
        argus.click_and_reinspect(&mut harness, STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID);
    let error_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(2);
        if matches!(
            stage.lock().unwrap().last_embed_back.as_ref(),
            Some(handshake_native::stage_pane::EmbedBackOutcome::EndpointAbsent { .. })
        ) {
            break;
        }
        assert!(
            std::time::Instant::now() < error_deadline,
            "mounted Stage endpoint-absent outcome did not complete within five seconds: {:?}",
            stage.lock().unwrap().last_embed_back
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        absent_request
            .join()
            .expect("join endpoint-absent witness")
            .starts_with("POST /workspaces/"),
        "typed endpoint-absent outcome comes from the real create request"
    );
    let error_inspect = argus.assert_latest_terminal_predicate(
        &mut harness,
        "mt066-capture-endpoint-absent-surfaces-typed-status",
        |after| json_has_author_id(after, STAGE_EMBED_BACK_STATUS_AUTHOR_ID),
    );
    assert!(
        json_has_author_id(&error_inspect, STAGE_EMBED_BACK_STATUS_AUTHOR_ID),
        "fresh canonical Argus inspection observes the terminal typed error"
    );
    assert!(
        !handshake_native::rich_editor::document_model::doc_json::to_content_json_value(
            &rich_state.lock().unwrap().doc,
        )
        .to_string()
        .contains(STAGE_CAPTURE_REF_KIND),
        "endpoint absence must not fabricate an HsLink"
    );

    let (old_backend_base, new_backend_base) = cleanup.backend.restart_owned();
    harness
        .state_mut()
        .set_backend_base_url_for_test(&new_backend_base, runtime.handle().clone());
    harness
        .state_mut()
        .set_stage_embed_back_base_url_for_test(&new_backend_base);
    let reloaded_after_restart = runtime
        .block_on(
            handshake_native::backend_client::RichDocClient::new(
                &new_backend_base,
                runtime.handle().clone(),
            )
            .load_document(&document_id),
        )
        .expect("reload the exact target document from the restarted backend");
    harness
        .state_mut()
        .apply_loaded_rich_document_to_view_for_test(pane_id.as_ref(), reloaded_after_restart)
        .expect("rebind the mounted rich target and SaveManager to the restarted backend");
    let project_tree_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(2);
        if !harness.state().left_rail().project_tree.is_loading()
            && harness.state().left_rail().project_tree.error().is_none()
        {
            break;
        }
        assert!(
            std::time::Instant::now() < project_tree_deadline,
            "project tree did not recover on the restarted managed backend: loading={}; error={:?}",
            harness.state().left_rail().project_tree.is_loading(),
            harness.state().left_rail().project_tree.error()
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let embed_observation =
        argus.click_and_reinspect(&mut harness, STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID);
    assert!(
        !embed_observation.receipt_status.is_empty(),
        "canonical Argus returns the Stage capture action receipt"
    );
    let embed_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(2);
        if matches!(
            stage.lock().unwrap().last_embed_back.as_ref(),
            Some(handshake_native::stage_pane::EmbedBackOutcome::Embedded { .. })
        ) {
            break;
        }
        assert!(
            std::time::Instant::now() < embed_deadline,
            "mounted Stage embed-back did not complete within five seconds; terminal outcome: {:?}; runtime state (panel_open,target_retained,request_pending,in_flight): {:?}",
            stage.lock().unwrap().last_embed_back,
            harness.state().stage_embed_runtime_state_for_test(),
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let (artifact_id, outcome_sha) = match stage.lock().unwrap().last_embed_back.clone() {
        Some(handshake_native::stage_pane::EmbedBackOutcome::Embedded {
            artifact_id,
            sha256,
            ..
        }) => (artifact_id, sha256),
        other => panic!("expected embedded Stage outcome, got {other:?}"),
    };
    assert_eq!(outcome_sha, expected_sha);
    let stage_token = harness.state().mcp_token();
    let client = StageClient::with_base_url(cleanup.backend.base.clone())
        .with_session_token(stage_token.as_hex());
    let artifact = rt()
        .block_on(client.fetch_stage_artifact(&workspace_id, &artifact_id))
        .expect("production Stage client retrieves and verifies exact persisted bytes");
    cleanup.track_stage_artifact(&artifact);
    assert_eq!(artifact.content_bytes, exact_text.as_bytes());
    assert_eq!(artifact.sha256, expected_sha);
    assert_eq!(
        artifact.correlation_id,
        route_row["payload"]["native_payload"]["causal_action_id"]
            .as_str()
            .expect("route causal action id")
    );
    assert!(
        artifact.job_id.is_some(),
        "capture is visible in Job History"
    );
    assert!(
        artifact.event_ledger_event_id.is_some(),
        "capture carries its EventLedger receipt"
    );

    let mounted_content =
        handshake_native::rich_editor::document_model::doc_json::to_content_json_value(
            &rich_state.lock().unwrap().doc,
        );
    let mounted_json = mounted_content.to_string();
    assert!(mounted_json.contains(&artifact_id));
    assert!(mounted_json.contains(STAGE_CAPTURE_REF_KIND));

    let embed_row = wait_for_native_fr(
        &*cleanup.backend,
        &workspace_id,
        "stage_embed_back",
        |row| {
            row["payload"]["native_payload"]["artifact_id"].as_str() == Some(artifact_id.as_str())
        },
    );
    cleanup.track_native_fr(&embed_row);
    assert_eq!(
        embed_row["payload"]["native_payload"]["artifact_id"].as_str(),
        Some(artifact_id.as_str())
    );
    assert_eq!(
        embed_row["payload"]["native_payload"]["sha256"].as_str(),
        Some(expected_sha.as_str())
    );
    assert_eq!(
        embed_row["payload"]["native_payload"]["manifest_ref"].as_str(),
        Some(artifact.manifest.manifest_ref.as_str())
    );
    assert_eq!(
        embed_row["payload"]["native_payload"]["causal_action_id"].as_str(),
        route_row["payload"]["native_payload"]["causal_action_id"].as_str(),
        "embed-back FR inherits the exact same causal action id as route-to-Stage"
    );
    let route_ts = route_row["payload"]["ts_utc"]
        .as_str()
        .expect("automatic route event timestamp");
    let embed_ts = embed_row["payload"]["ts_utc"]
        .as_str()
        .expect("automatic embed event timestamp");
    assert!(
        chrono::DateTime::parse_from_rfc3339(embed_ts).unwrap()
            > chrono::DateTime::parse_from_rfc3339(route_ts).unwrap(),
        "the exact embed-back event is strictly later than its exact route event"
    );
    std::thread::sleep(std::time::Duration::from_millis(100));
    let rows = cleanup.backend.get_json_with_session_token(
        &format!("/api/flight_recorder?wsid={workspace_id}"),
        &live_binding_session_token(),
    );
    let route_dispatches = rows
        .as_array()
        .expect("Flight Recorder rows")
        .iter()
        .filter(|row| {
            row["payload"]["kind"].as_str() == Some("route_to_stage")
                && row["payload"]["native_payload"]["causal_action_id"].as_str()
                    == Some(retained_causal_action_id.as_str())
        })
        .count();
    let exact_route_rows = rows
        .as_array()
        .expect("Flight Recorder rows")
        .iter()
        .filter(|row| fr_row_matches_producer_event(row, retained_route_event_id.as_str()))
        .count();
    assert_eq!(
        route_dispatches, 1,
        "backend restart must retain exactly one immutable Route-to-Stage receipt; post-restart rows={rows}"
    );
    assert_eq!(
        exact_route_rows, 1,
        "post-restart FR must retain exactly one row with the retained producer event id; rows={rows}"
    );
    let quiescence_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        let runtime_state = harness.state().stage_embed_runtime_state_for_test();
        let stage_state = stage.lock().unwrap();
        if !runtime_state.2
            && !runtime_state.3
            && !stage_state.has_route_retry()
            && !stage_state.has_pending_route_receipt()
            && matches!(
                stage_state.last_embed_back.as_ref(),
                Some(handshake_native::stage_pane::EmbedBackOutcome::Embedded { .. })
            )
        {
            break;
        }
        assert!(
            std::time::Instant::now() < quiescence_deadline,
            "MT-066 did not reach bounded quiescence: runtime={runtime_state:?}, stage={stage_state:?}"
        );
        drop(stage_state);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let recovered_inspect = argus.assert_latest_terminal_predicate(
        &mut harness,
        "mt066-capture-recovered-shows-embed-status-and-routed-content",
        |after| {
            json_has_author_id(after, STAGE_EMBED_BACK_STATUS_AUTHOR_ID)
                && json_has_author_id(after, STAGE_ROUTED_CONTENT_AUTHOR_ID)
                && !json_has_author_id(
                    after,
                    handshake_native::project_tree::PROJECT_TREE_RETRY_AUTHOR_ID,
                )
        },
    );
    assert!(
        json_has_author_id(&recovered_inspect, STAGE_EMBED_BACK_STATUS_AUTHOR_ID)
            && json_has_author_id(&recovered_inspect, STAGE_ROUTED_CONTENT_AUTHOR_ID),
        "fresh canonical Argus inspection observes the recovered terminal capture and routed content"
    );
    assert!(
        !json_has_author_id(
            &recovered_inspect,
            handshake_native::project_tree::PROJECT_TREE_RETRY_AUTHOR_ID
        ) && !recovered_inspect.to_string().contains("Load failed"),
        "recovered canonical Argus state must not retain the former backend's project-tree failure: {recovered_inspect}"
    );
    let recovered_screenshot_path = artifact_dir.join("mt066-stage-recovered-harness-render.png");
    harness
        .render()
        .expect("MT-066 recovered state requires a material harness render")
        .save(&recovered_screenshot_path)
        .expect("save MT-066 recovered harness render");
    let busy_png = std::fs::read(&busy_screenshot_path).expect("read busy PNG");
    let recovered_png = std::fs::read(&recovered_screenshot_path).expect("read recovered PNG");
    let busy_png_sha256 = format!("{:x}", Sha256::digest(&busy_png));
    let recovered_png_sha256 = format!("{:x}", Sha256::digest(&recovered_png));
    let busy_dimensions = image::GenericImageView::dimensions(
        &image::load_from_memory(&busy_png).expect("decode busy PNG"),
    );
    let recovered_dimensions = image::GenericImageView::dimensions(
        &image::load_from_memory(&recovered_png).expect("decode recovered PNG"),
    );
    let terminal_outcome = format!("{:?}", stage.lock().unwrap().last_embed_back);
    let evidence_path = artifact_dir.join("mt066-stage-canonical-argus.json");
    argus.finish();
    cleanup.finish_and_assert_zero();
    std::fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_id": "handshake.mt066-stage-canonical-argus-proof.v1",
            "test": "live_route_round_trip_real_pg",
            "status": "PASS",
            "recorded_at": chrono::Utc::now().to_rfc3339(),
            "source": {
                "source_sha": source_sha,
                "runtime_source_tree": runtime_source_tree,
                "proof_source_blob": proof_source_blob,
                "proof_source_blobs": proof_source_blobs,
                "relevant_source_clean": true,
                "transitive_runtime_source_clean": true,
                "global_worktree_clean": false,
                "known_unrelated_dirty_paths": ["AGENTS.md", "CLAUDE.md"],
            },
            "dead_owner_recovery": dead_owner_evidence,
            "backend_restart": {
                "old_base": old_backend_base,
                "new_base": new_backend_base,
                "post_restart_artifact_readback": true,
            },
            "state_matrix": {
                "route_busy": {
                    "method": "argus.click",
                    "target": "menu.editors.route-to-stage",
                    "receipt_id": busy_observation.receipt_id,
                    "receipt_status": busy_observation.receipt_status,
                    "agent_id": busy_observation.agent_id,
                    "fresh_inspect": busy_inspect,
                    "causal_action_id": retained_causal_action_id,
                    "backend_row_before_retry": false,
                },
                "route_recovered": {
                    "method": "argus.click",
                    "target": STAGE_ROUTE_RETRY_AUTHOR_ID,
                    "receipt_id": route_observation.receipt_id,
                    "receipt_status": route_observation.receipt_status,
                    "agent_id": route_observation.agent_id,
                    "fresh_inspect": route_recovered_inspect,
                    "retained_route_event_id": retained_route_event_id,
                    "flight_recorder_row": route_row,
                },
                "capture_error": {
                    "method": "argus.click",
                    "target": STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
                    "receipt_id": error_observation.receipt_id,
                    "receipt_status": error_observation.receipt_status,
                    "agent_id": error_observation.agent_id,
                    "typed_outcome": "EndpointAbsent",
                    "fresh_inspect": error_inspect,
                    "artifact_fabricated": false,
                },
                "capture_recovered": {
                    "method": "argus.click",
                    "target": STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
                    "receipt_id": embed_observation.receipt_id,
                    "receipt_status": embed_observation.receipt_status,
                    "agent_id": embed_observation.agent_id,
                    "terminal_outcome": terminal_outcome,
                    "fresh_inspect": recovered_inspect,
                    "artifact_id": artifact.artifact_id,
                    "artifact_sha256": artifact.sha256,
                    "manifest_ref": artifact.manifest.manifest_ref,
                    "job_id": artifact.job_id,
                    "event_ledger_event_id": artifact.event_ledger_event_id,
                    "flight_recorder_row": embed_row,
                },
            },
            "screenshots": [
                {
                    "state": "route_busy",
                    "path": busy_screenshot_path,
                    "capture_method": "egui_kittest Harness::render",
                    "sha256": busy_png_sha256,
                    "width": busy_dimensions.0,
                    "height": busy_dimensions.1,
                },
                {
                    "state": "capture_recovered",
                    "path": recovered_screenshot_path,
                    "capture_method": "egui_kittest Harness::render",
                    "sha256": recovered_png_sha256,
                    "width": recovered_dimensions.0,
                    "height": recovered_dimensions.1,
                }
            ],
            "cleanup": {
                "workspace_absent": true,
                "stage_artifacts_zero": true,
                "stage_jobs_zero": true,
                "event_ledger_fixture_rows_zero": true,
                "flight_recorder_fixture_rows_zero": true,
                "runtime_quiescent": true,
            }
        }))
        .expect("serialize MT-066 canonical Argus evidence"),
    )
    .expect("write MT-066 canonical Argus evidence");
    assert!(evidence_path.is_file());
    println!(
        "MT-066 PROVEN: busy->retry->route->typed endpoint error->owned backend restart->capture/embed/readback->cleanup; source_sha={source_sha}; evidence={}",
        evidence_path.display()
    );
}

/// Managed-PG proof for the real Canvas origin/target. The route starts on the shared Canvas bus, the
/// mounted Stage button performs capture + embed, a fresh backend read validates the structured card and
/// dereferenced exact bytes, and a repeated embed converges on the same single placement.
#[test]
fn mounted_canvas_embed_back_live_pg_is_structured_and_idempotent() {
    let _binding_env_guard = BINDING_ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut stage_binding = stage_binding_proof::StageBindingGuard::reserve("mt066-canvas-stage");
    let mut backend = interconnect_support::require_reachable_backend();
    let workspace = backend.create_workspace(&format!(
        "mt066-canvas-stage-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();
    let mut cleanup = LiveWorkspaceGuard {
        backend: &mut backend,
        workspace_id: workspace_id.clone(),
        native_fr_event_ids: Vec::new(),
        stage_artifact_ids: Vec::new(),
        stage_job_ids: Vec::new(),
        stage_event_ids: Vec::new(),
        workspace_deleted: false,
        native_fr_cleanup_done: false,
        stage_side_effect_cleanup_done: false,
    };
    let canvas = cleanup.backend.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({"title": "MT-066 Stage Canvas target"}),
    );
    let canvas_id = canvas["block_id"]
        .as_str()
        .expect("Canvas create returns block_id")
        .to_owned();
    let source_block = cleanup.backend.post_json(
        &format!("/workspaces/{workspace_id}/loom/blocks"),
        &serde_json::json!({
            "content_type": "note",
            "title": "MT-066 operator-routed Canvas source"
        }),
    );
    let source_block_id = source_block["block_id"]
        .as_str()
        .expect("Canvas source block create returns block_id")
        .to_owned();
    let source_placement = cleanup.backend.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}/placements"),
        &serde_json::json!({
            "placed_block_id": source_block_id,
            "x": 40.0,
            "y": 40.0,
            "w": 240.0,
            "h": 140.0
        }),
    );
    let source_placement_id = source_placement["placement_id"]
        .as_str()
        .expect("Canvas source placement create returns placement_id")
        .to_owned();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("MT-066 Canvas runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&cleanup.backend.base, runtime.handle().clone());
    app.set_stage_embed_back_base_url_for_test(&cleanup.backend.base);
    assert!(app.switch_project(&workspace_id));
    {
        let board = app.mounted_canvas_board();
        let mut board = board.lock().expect("mounted Canvas board lock");
        board.workspace_id = workspace_id.clone();
        board.canvas_block_id = canvas_id.clone();
    }
    assert!(
        app.dispatch_palette_action_for_test(handshake_native::command_registry::CMD_VIEW_CANVAS),
        "operator-facing View Canvas command mounts the production Canvas pane"
    );
    assert!(
        app.active_pane().is_some(),
        "View Canvas targets a live pane"
    );

    stage_binding.publish(app.mcp_token().as_hex());

    let host_ctx = std::sync::Arc::new(std::sync::Mutex::new(None::<egui::Context>));
    let host_ctx_capture = std::sync::Arc::clone(&host_ctx);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(
            move |ctx, app: &mut HandshakeApp| {
                *host_ctx_capture.lock().expect("capture mounted context") = Some(ctx.clone());
                app.ui(ctx);
            },
            app,
        );
    harness.run_steps(3);
    let _ctx = host_ctx
        .lock()
        .expect("mounted context lock")
        .clone()
        .expect("mounted context captured");
    let board_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        if harness
            .state()
            .mounted_canvas_board()
            .lock()
            .unwrap()
            .placements
            .iter()
            .any(|placement| placement.placement_id == source_placement_id)
        {
            break;
        }
        assert!(
            std::time::Instant::now() < board_deadline,
            "mounted Canvas did not load the seeded source placement within five seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    harness.run_steps(2);
    let source_card_author_id = placement_author_id(&source_placement_id);
    assert!(
        harness
            .root()
            .children_recursive()
            .any(|node| node.accesskit_node().author_id() == Some(source_card_author_id.as_str())),
        "the operator-routed Canvas card is present in the mounted AccessKit tree"
    );
    let source_card_screen = {
        let board = harness.state().mounted_canvas_board();
        let board = board.lock().unwrap();
        let card = board
            .placements
            .iter()
            .find(|placement| placement.placement_id == source_placement_id)
            .expect("source placement remains mounted");
        board
            .canvas_point_to_screen(egui::pos2(card.x + card.w * 0.5, card.y + card.h * 0.5))
            .expect("mounted Canvas reports its live screen rect")
    };
    harness.event(egui::Event::PointerMoved(source_card_screen));
    harness.event(egui::Event::PointerButton {
        pos: source_card_screen,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.event(egui::Event::PointerButton {
        pos: source_card_screen,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run_steps(1);
    let route_menu_author_id = format!("ctx-menu.{}", node_menu_ids::ROUTE_TO_STAGE);
    let route_menu = harness.get_by(|node| node.author_id() == Some(route_menu_author_id.as_str()));
    route_menu.click();

    let stage = harness.state().mounted_stage();
    let route_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        harness.run_steps(1);
        if matches!(
            stage.lock().unwrap().content,
            handshake_native::stage_pane::StageContent::Selection(ref text, ref source)
                if text == &format!("canvas node {source_block_id}")
                    && source == &format!("node://{canvas_id}/{source_block_id}")
        ) {
            break;
        }
        assert!(
            std::time::Instant::now() < route_deadline,
            "mounted Canvas route did not reach Stage within five seconds"
        );
    }
    harness
        .get_by(|node| node.author_id() == Some(STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID))
        .click();
    let embed_deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        harness.run_steps(2);
        if matches!(
            stage.lock().unwrap().last_embed_back,
            Some(handshake_native::stage_pane::EmbedBackOutcome::Embedded { .. })
        ) {
            break;
        }
        assert!(
            std::time::Instant::now() < embed_deadline,
            "mounted Canvas Stage embed did not complete within eight seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let (artifact_id, sha256) = match stage.lock().unwrap().last_embed_back.clone() {
        Some(handshake_native::stage_pane::EmbedBackOutcome::Embedded {
            artifact_id,
            sha256,
            ..
        }) => (artifact_id, sha256),
        other => panic!("expected Canvas Stage embed outcome, got {other:?}"),
    };
    let stage_client = StageClient::with_base_url(cleanup.backend.base.clone())
        .with_session_token(harness.state().mcp_token().as_hex());
    let artifact = rt()
        .block_on(stage_client.fetch_stage_artifact(&workspace_id, &artifact_id))
        .expect("fresh Stage dereference validates the Canvas reference");
    cleanup.track_stage_artifact(&artifact);
    assert_eq!(
        artifact.content_bytes,
        format!("canvas node {source_block_id}").as_bytes()
    );
    assert_eq!(artifact.sha256, sha256);

    let canvas_client = handshake_native::backend_client::CanvasBoardClient::new(
        cleanup.backend.base.clone(),
        runtime.handle().clone(),
    );
    let first_board = rt()
        .block_on(canvas_client.fetch_board_now(&workspace_id, &canvas_id))
        .expect("fresh Canvas reload after Stage embed");
    assert_eq!(
        first_board.placements.len(),
        2,
        "the operator source plus one structured Stage card persist"
    );
    let first_stage_placement = rt()
        .block_on(canvas_client.find_stage_capture_card_now(
            &workspace_id,
            &canvas_id,
            &artifact_id,
            &artifact.sha256,
            &artifact.manifest.manifest_ref,
            &artifact.correlation_id,
        ))
        .expect("structured Canvas Stage reference reload")
        .expect("fresh reload parses the exact structured provenance tuple");
    let first_placement_id = first_stage_placement.placement_id;

    let route_row = wait_for_native_fr(&*cleanup.backend, &workspace_id, "route_to_stage", |row| {
        row["payload"]["native_payload"]["content_kind"].as_str() == Some("canvas_node")
    });
    cleanup.track_native_fr(&route_row);
    let embed_row = wait_for_native_fr(
        &*cleanup.backend,
        &workspace_id,
        "stage_embed_back",
        |row| {
            row["payload"]["native_payload"]["artifact_id"].as_str() == Some(artifact_id.as_str())
        },
    );
    cleanup.track_native_fr(&embed_row);
    assert_eq!(
        route_row["payload"]["native_payload"]["causal_action_id"],
        embed_row["payload"]["native_payload"]["causal_action_id"]
    );
    assert_eq!(
        route_row["payload"]["native_payload"]["causal_action_id"].as_str(),
        Some(artifact.correlation_id.as_str()),
        "operator-facing Canvas route preserves its exact causal id through capture"
    );
    assert_eq!(
        embed_row["payload"]["native_payload"]["artifact_id"].as_str(),
        Some(artifact_id.as_str())
    );
    assert_eq!(
        embed_row["payload"]["native_payload"]["sha256"].as_str(),
        Some(artifact.sha256.as_str())
    );
    assert_eq!(
        embed_row["payload"]["native_payload"]["manifest_ref"].as_str(),
        Some(artifact.manifest.manifest_ref.as_str())
    );
    let route_ts = chrono::DateTime::parse_from_rfc3339(
        route_row["payload"]["ts_utc"]
            .as_str()
            .expect("Canvas route timestamp"),
    )
    .unwrap();
    let embed_ts = chrono::DateTime::parse_from_rfc3339(
        embed_row["payload"]["ts_utc"]
            .as_str()
            .expect("Canvas embed timestamp"),
    )
    .unwrap();
    assert!(
        route_ts < embed_ts,
        "Canvas route strictly precedes embed-back"
    );

    let parallel_canvas = cleanup.backend.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({"title": "MT-066 concurrent Stage target"}),
    );
    let parallel_canvas_id = parallel_canvas["block_id"]
        .as_str()
        .expect("parallel Canvas create returns block_id")
        .to_owned();
    let client_a = handshake_native::backend_client::CanvasBoardClient::with_http_client(
        cleanup.backend.base.clone(),
        runtime.handle().clone(),
        handshake_native::backend_client::build_backend_client(),
    );
    let client_b = handshake_native::backend_client::CanvasBoardClient::with_http_client(
        cleanup.backend.base.clone(),
        runtime.handle().clone(),
        handshake_native::backend_client::build_backend_client(),
    );
    let (parallel_a, parallel_b) = runtime.block_on(async {
        tokio::join!(
            client_a.ensure_stage_capture_card_now(
                &workspace_id,
                &parallel_canvas_id,
                &artifact_id,
                &artifact.sha256,
                &artifact.manifest.manifest_ref,
                &artifact.correlation_id,
                40.0,
                40.0,
                260.0,
                160.0,
            ),
            client_b.ensure_stage_capture_card_now(
                &workspace_id,
                &parallel_canvas_id,
                &artifact_id,
                &artifact.sha256,
                &artifact.manifest.manifest_ref,
                &artifact.correlation_id,
                40.0,
                40.0,
                260.0,
                160.0,
            )
        )
    });
    let parallel_a = parallel_a.expect("first concurrent Stage card request");
    let parallel_b = parallel_b.expect("second concurrent Stage card request");
    assert_eq!(parallel_a.placement_id, parallel_b.placement_id);
    assert_ne!(
        parallel_a.created_by_request, parallel_b.created_by_request,
        "exactly one concurrent caller creates while the other reconciles"
    );
    let parallel_board = runtime
        .block_on(canvas_client.fetch_board_now(&workspace_id, &parallel_canvas_id))
        .expect("fresh concurrent Canvas reload");
    assert_eq!(
        parallel_board.placements.len(),
        1,
        "simultaneous model actions converge on one canonical placement"
    );
    let workspace_sql = workspace_id.replace('\'', "''");
    let canvas_sql = parallel_canvas_id.replace('\'', "''");
    cleanup.backend.run_fixture_sql(
        "mt066-cross-client-stage-provenance-uniqueness",
        &format!(
            "DO $stage_race$ DECLARE placements bigint; documents bigint; blocks bigint; bridges bigint; BEGIN \
             SELECT COUNT(*) INTO placements FROM loom_canvas_placements \
             WHERE workspace_id = '{workspace_sql}' AND canvas_block_id = '{canvas_sql}' \
               AND stage_provenance_key IS NOT NULL; \
             SELECT COUNT(*) INTO documents FROM knowledge_rich_documents document \
             JOIN loom_canvas_placements placement \
               ON placement.placed_block_id = document.rich_document_id \
              AND placement.workspace_id = document.workspace_id \
             WHERE placement.workspace_id = '{workspace_sql}' \
               AND placement.canvas_block_id = '{canvas_sql}' \
               AND placement.stage_provenance_key IS NOT NULL; \
             SELECT COUNT(*) INTO blocks FROM loom_blocks block \
             JOIN loom_canvas_placements placement \
               ON placement.placed_block_id = block.block_id \
              AND placement.workspace_id = block.workspace_id \
             WHERE placement.workspace_id = '{workspace_sql}' \
               AND placement.canvas_block_id = '{canvas_sql}' \
               AND placement.stage_provenance_key IS NOT NULL; \
             SELECT COUNT(*) INTO bridges FROM loom_block_knowledge_bridge bridge \
             JOIN loom_canvas_placements placement \
               ON placement.placed_block_id = bridge.block_id \
              AND placement.workspace_id = bridge.workspace_id \
             WHERE placement.workspace_id = '{workspace_sql}' \
               AND placement.canvas_block_id = '{canvas_sql}' \
               AND placement.stage_provenance_key IS NOT NULL; \
             IF placements <> 1 OR documents <> 1 OR blocks <> 1 OR bridges <> 1 THEN \
               RAISE EXCEPTION 'Stage race residue placements=%, documents=%, blocks=%, bridges=%', \
                 placements, documents, blocks, bridges; \
             END IF; END $stage_race$;"
        ),
    );

    stage.lock().unwrap().last_embed_back = None;
    harness
        .get_by(|node| node.author_id() == Some(STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID))
        .click();
    let retry_deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        harness.run_steps(2);
        if matches!(
            stage.lock().unwrap().last_embed_back,
            Some(handshake_native::stage_pane::EmbedBackOutcome::Embedded { .. })
        ) {
            break;
        }
        assert!(
            std::time::Instant::now() < retry_deadline,
            "repeated Canvas Stage embed did not converge within eight seconds"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let retry_board = rt()
        .block_on(canvas_client.fetch_board_now(&workspace_id, &canvas_id))
        .expect("fresh Canvas reload after repeated embed");
    assert_eq!(
        retry_board.placements.len(),
        2,
        "retry creates no duplicate card"
    );
    assert_eq!(
        rt().block_on(canvas_client.find_stage_capture_card_now(
            &workspace_id,
            &canvas_id,
            &artifact_id,
            &artifact.sha256,
            &artifact.manifest.manifest_ref,
            &artifact.correlation_id,
        ))
        .expect("retry structured Canvas Stage reference reload")
        .expect("retry retains a Stage placement")
        .placement_id,
        first_placement_id,
        "retry retains the original placement identity"
    );

    let rebound_canvas = cleanup.backend.post_json(
        &format!("/workspaces/{workspace_id}/loom/canvas-boards"),
        &serde_json::json!({"title": "MT-066 rebound Canvas"}),
    );
    let rebound_canvas_id = rebound_canvas["block_id"]
        .as_str()
        .expect("rebound Canvas create returns block_id")
        .to_owned();
    harness
        .state()
        .mounted_canvas_board()
        .lock()
        .unwrap()
        .canvas_block_id = rebound_canvas_id.clone();
    harness.run_steps(2);
    stage.lock().unwrap().last_embed_back = None;
    harness
        .get_by(|node| node.author_id() == Some(STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID))
        .click();
    harness.run_steps(3);
    assert!(
        matches!(
            stage.lock().unwrap().last_embed_back,
            Some(handshake_native::stage_pane::EmbedBackOutcome::Failed(_))
        ),
        "rebinding the same pane to another Canvas invalidates the retained target"
    );
    let old_after_rebind = rt()
        .block_on(canvas_client.fetch_board_now(&workspace_id, &canvas_id))
        .expect("old Canvas reload after target rebind");
    let rebound_after_rebind = rt()
        .block_on(canvas_client.fetch_board_now(&workspace_id, &rebound_canvas_id))
        .expect("rebound Canvas reload after rejected stale embed");
    assert_eq!(old_after_rebind.placements.len(), 2);
    assert!(
        rebound_after_rebind.placements.is_empty(),
        "stale target rejection mutates neither the old nor replacement Canvas"
    );
    cleanup.finish_and_assert_zero();
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-003 / AC-003 — embed-back inserts an MT-014 NodeView carrying SHA-256 manifest provenance.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn embed_back_inserts_mt014_nodeview_with_provenance() {
    let artifact = evidence_artifact("ART-77");
    let view =
        embed_artifact_as_nodeview(&artifact).expect("AC-003: evidence-grade artifact embeds");

    // The inserted NodeView is the MT-014 embed atom (an hsLink by ref_kind), NOT a parallel type.
    assert_eq!(view.node.ref_kind, STAGE_CAPTURE_REF_KIND);
    assert_eq!(
        view.node.ref_kind, "stage_capture",
        "the MT-014 hsLink ref_kind discriminator"
    );
    assert_eq!(view.node.ref_value, "ART-77");
    // The provenance descriptor is present and matches the fetched artifact's sha256 (the contract shape).
    assert_eq!(view.provenance.source, "stage_capture");
    assert_eq!(view.provenance.artifact_id, "ART-77");
    assert_eq!(view.provenance.sha256, artifact.sha256);
    assert_eq!(view.provenance.manifest_ref, "manifest://ART-77");

    // The Stage pane's capture_and_embed_back inserts the NodeView into a live note target and records the
    // outcome with the SHA-256 anchor. The insert closure proves the NodeView reaches the document model.
    use std::cell::RefCell;
    use std::rc::Rc;
    let inserted: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = inserted.clone();
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-rich".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(
        Ok(artifact.clone()),
        &target,
        |candidate| candidate.pane_id() == "pane-rich", // target is live
        |view, _t| {
            cap.borrow_mut().push(view.node.ref_value.clone());
            Ok(())
        },
    );
    match outcome {
        handshake_native::stage_pane::EmbedBackOutcome::Embedded {
            artifact_id,
            sha256,
            target_pane,
        } => {
            assert_eq!(artifact_id, "ART-77");
            assert_eq!(sha256, artifact.sha256);
            assert_eq!(target_pane, "pane-rich");
        }
        other => panic!("AC-003: expected Embedded, got {other:?}"),
    }
    assert_eq!(
        inserted.borrow().as_slice(),
        ["ART-77"],
        "AC-003: the MT-014 NodeView reached the note"
    );
    println!(
        "PT-003 embed-back OK: MT-014 hsLink atom inserted into note, SHA-256 provenance preserved"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-004 / AC-004 — the missing embed-back route is the typed blocker (BROAD: 404 AND 501).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn embed_back_endpoint_absent_404() {
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 404 Not Found",
        serde_json::json!({"error": "not found"}),
    );
    let client = StageClient::with_base_url(base_url);
    let result = rt().block_on(async { client.fetch_stage_artifact("WS-1", "ART-1").await });
    let req_line = server.join().unwrap();

    // The probe is a GET (read-only) at the documented route.
    assert!(
        req_line.starts_with("GET "),
        "AC-004: fetch must issue a GET; got '{req_line}'"
    );
    assert!(
        req_line.contains("/workspaces/WS-1/stage/artifacts/ART-1"),
        "fetch must hit the documented embed-back route; got '{req_line}'"
    );
    match result {
        Err(StageInteropError::EmbedBackEndpointAbsent { probed_path }) => {
            assert!(
                probed_path.contains("/workspaces/WS-1/stage/artifacts/ART-1"),
                "AC-004: EmbedBackEndpointAbsent must name the probed path; got '{probed_path}'"
            );
            println!(
                "PT-004 typed blocker (404) OK: EmbedBackEndpointAbsent(probed='{probed_path}')"
            );
        }
        other => panic!("AC-004: a 404 must map to EmbedBackEndpointAbsent, got {other:?}"),
    }
}

#[test]
fn embed_back_endpoint_absent_501() {
    // BROAD detection (RISK-008/MC-008): a 501 Not Implemented is ALSO the typed blocker, not a generic
    // transport error.
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 501 Not Implemented",
        serde_json::json!({"error": "not implemented"}),
    );
    let client = StageClient::with_base_url(base_url);
    let result = rt().block_on(async { client.fetch_stage_artifact("WS-1", "ART-2").await });
    let _ = server.join();
    assert!(
        matches!(
            result,
            Err(StageInteropError::EmbedBackEndpointAbsent { .. })
        ),
        "AC-004: a 501 must ALSO map to EmbedBackEndpointAbsent (broad detection), got {result:?}"
    );
    println!("PT-004 typed blocker (501) OK: 501 -> EmbedBackEndpointAbsent (broad detection)");
}

/// The embed-back never fabricates an artifact: even when the Stage pane runs the embed-back over an
/// absent endpoint, the outcome is the typed blocker (surfaced, never a fake embed). No insert happens.
#[test]
fn embed_back_blocker_surfaces_no_fake_embed() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let inserted: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let cap = inserted.clone();
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-rich".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(
        Err(StageInteropError::EmbedBackEndpointAbsent {
            probed_path: "/workspaces/WS-1/stage/artifacts/ART-1".into(),
        }),
        &target,
        |_pid| true,
        |_view, _t| {
            *cap.borrow_mut() += 1;
            Ok(())
        },
    );
    assert!(
        outcome.is_endpoint_absent(),
        "AC-004: the blocker outcome is surfaced"
    );
    assert!(
        pane.has_embed_back_endpoint_absent_blocker(),
        "the host surfaces the blocker to the validator"
    );
    assert_eq!(
        *inserted.borrow(),
        0,
        "AC-004: NO artifact fabricated, NO insert on the typed blocker"
    );
    println!("PT-004 no-fake-embed OK: EmbedBackEndpointAbsent surfaced, zero inserts");
}

/// RISK-002/MC-002: an artifact with no SHA-256 / manifest provenance is REFUSED (ProvenanceMissing) — the
/// pane never embeds an unverifiable evidence-grade capture.
#[test]
fn embed_back_refuses_unverifiable_capture() {
    let mut artifact = evidence_artifact("ART-3");
    artifact.sha256 = String::new();
    artifact.manifest.sha256 = String::new();
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-rich".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(Ok(artifact), &target, |_pid| true, |_v, _t| Ok(()));
    assert_eq!(
        outcome,
        handshake_native::stage_pane::EmbedBackOutcome::ProvenanceMissing,
        "RISK-002/MC-002: an unverifiable artifact is refused, not embedded"
    );
}

/// RISK-007/MC-007: the embed target is re-resolved at embed time; a dangling target pane is refused.
#[test]
fn embed_back_refuses_dangling_target_pane() {
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-gone".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(
        Ok(evidence_artifact("ART-4")),
        &target,
        |_pid| false,
        |_v, _t| Ok(()),
    );
    assert_eq!(
        outcome,
        handshake_native::stage_pane::EmbedBackOutcome::TargetGone {
            pane_id: "pane-gone".to_owned()
        },
        "RISK-007/MC-007: a dangling embed target pane is refused"
    );
}

#[test]
fn embed_back_reports_failed_when_target_rejects_insert() {
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-rich".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(
        Ok(evidence_artifact("ART-insert-rejected")),
        &target,
        |_pid| true,
        |_view, _target| Err("document persistence rejected the embed".to_owned()),
    );
    assert_eq!(
        outcome,
        handshake_native::stage_pane::EmbedBackOutcome::Failed(
            "document persistence rejected the embed".to_owned()
        )
    );
    assert_eq!(pane.last_embed_back, Some(outcome));
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-005 / AC-006 — AccessKit nodes present with correct roles + nesting (+ screenshot).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn stage_pane_accesskit_nodes_present() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(380.0, 280.0))
        .wgpu()
        .build_ui(|ui| {
            let mut pane = StagePane::new();
            // Seed routed content so the routed-content region shows the route-leg landing.
            pane.receive_routed_content(handshake_native::stage_pane::StageContent::Selection(
                "routed selection".to_owned(),
                "pane-rich:0-16".to_owned(),
            ));
            pane.show_round_trip(ui, &dark());
        });
    harness.run();
    harness.run();

    let root = harness.root();

    // AC-006: the three contract-named nodes are present with the right roles.
    let pane_role = role_of(&root, STAGE_PANE_AUTHOR_ID);
    assert_eq!(
        pane_role.as_deref(),
        Some("GenericContainer"),
        "AC-006: '{STAGE_PANE_AUTHOR_ID}' must be Role::GenericContainer (got {pane_role:?})"
    );
    let routed_role = role_of(&root, STAGE_ROUTED_CONTENT_AUTHOR_ID);
    assert_eq!(
        routed_role.as_deref(),
        Some("GenericContainer"),
        "AC-006: '{STAGE_ROUTED_CONTENT_AUTHOR_ID}' must be Role::GenericContainer (got {routed_role:?})"
    );
    let btn_role = role_of(&root, STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID);
    assert_eq!(
        btn_role.as_deref(),
        Some("Button"),
        "AC-006: '{STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID}' must be Role::Button (got {btn_role:?})"
    );

    // Nesting: the routed-content region + the embed-back button are under the stage-pane container.
    assert!(
        author_under(&root, STAGE_ROUTED_CONTENT_AUTHOR_ID, STAGE_PANE_AUTHOR_ID),
        "AC-006: the routed-content region must nest under the stage-pane container"
    );
    assert!(
        author_under(
            &root,
            STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
            STAGE_PANE_AUTHOR_ID
        ),
        "AC-006: the embed-back button must nest under the stage-pane container"
    );

    println!(
        "PT-005 accesskit dump: {{\"{STAGE_PANE_AUTHOR_ID}\":\"{}\",\"{STAGE_ROUTED_CONTENT_AUTHOR_ID}\":\"{}\",\"{STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID}\":\"{}\"}}",
        pane_role.unwrap_or_default(),
        routed_role.unwrap_or_default(),
        btn_role.unwrap_or_default()
    );

    // Screenshot to the EXTERNAL root ONLY (best-effort pixel readback).
    if let Ok(image) = harness.render() {
        let ext_dir = external_artifact_dir("wp-kernel-012-mt-066");
        let _ = std::fs::create_dir_all(&ext_dir);
        let ext_path = ext_dir.join("MT-066-stage-round-trip.png");
        let saved = image.save(&ext_path).is_ok();
        println!(
            "PT-005 screenshot: {}x{} saved_ext={saved} ({})",
            image.width(),
            image.height(),
            ext_path.display()
        );
    } else {
        println!(
            "PT-005 screenshot: GPU readback unavailable on this host (structural proof stands)"
        );
    }

    assert_no_local_artifact_dir();
}

/// The embed-back button is driveable out-of-process: a click flips the pressed signal `show_round_trip`
/// returns (so the host runs the async fetch + capture_and_embed_back).
#[test]
fn embed_back_button_press_signals_host() {
    use std::cell::Cell;
    use std::rc::Rc;
    let pressed: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let flag = pressed.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 240.0))
        .build_ui(move |ui| {
            let mut pane = StagePane::new();
            pane.receive_routed_content(handshake_native::stage_pane::StageContent::Selection(
                "x".to_owned(),
                "pane-rich:0-1".to_owned(),
            ));
            if pane.show_round_trip(ui, &dark()) {
                flag.set(true);
            }
        });
    harness.run();
    harness
        .get_by(|n| n.author_id() == Some(STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID))
        .click();
    harness.run();
    assert!(
        pressed.get(),
        "AC-006: clicking stage-capture-embed-back signals the host to run embed-back"
    );
    println!("PT-005 button press OK: stage-capture-embed-back click -> host embed-back signal");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-005 — a single route-to-stage command id (extend MT-033) + the added embed-stage-capture command.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn single_route_command_id_plus_embed_command() {
    use handshake_native::command_registry::all_commands;

    // Exactly ONE route-to-stage command id in the palette catalog (MT-033 extended, NOT duplicated).
    let route_rows: Vec<_> = all_commands()
        .iter()
        .filter(|c| c.id == CMD_ROUTE_TO_STAGE)
        .collect();
    assert_eq!(
        route_rows.len(),
        1,
        "AC-005/MC-003: exactly one route-to-stage command id ({CMD_ROUTE_TO_STAGE}); MT-033 extended, not duplicated"
    );
    assert_eq!(CMD_ROUTE_TO_STAGE, "interop.route-to-stage");

    // The NEW embed-stage-capture command id is present exactly once.
    let embed_rows: Vec<_> = all_commands()
        .iter()
        .filter(|c| c.id == CMD_EMBED_STAGE_CAPTURE)
        .collect();
    assert_eq!(
        embed_rows.len(),
        1,
        "AC-005: the added embed-stage-capture command id is present"
    );
    assert_eq!(CMD_EMBED_STAGE_CAPTURE, "interop.embed-stage-capture");
    assert_eq!(embed_rows[0].label, "Embed Stage Capture");
    assert!(
        !embed_rows[0].disabled,
        "the embed-stage-capture command is enabled (palette-driven)"
    );

    // The runtime bus also carries exactly one route + one embed-stage-capture descriptor (the WRAP-not-
    // fork registration). The route command is the EXISTING MT-033 register; the embed command is the new
    // MT-066 register.
    let mut bus = InteractionBus::new();
    bus.register_route_to_stage_command();
    handshake_native::interop::register_embed_stage_capture_command(&mut bus);
    assert!(
        bus.commands().get(CMD_ROUTE_TO_STAGE).is_some(),
        "route-to-stage descriptor on the bus"
    );
    assert!(
        bus.commands().get(CMD_EMBED_STAGE_CAPTURE).is_some(),
        "embed-stage-capture descriptor on the bus"
    );
    println!(
        "AC-005 command surface OK: 1 route-to-stage id (extended) + 1 embed-stage-capture id"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-007 — PostgreSQL authority and one shared HTTP stack.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn no_sqlite_and_shared_backend_client() {
    // The MT-066 production sources must not touch SQLite/rusqlite; PostgreSQL/EventLedger remains the
    // durable authority.
    let sources: [(&str, &str); 2] = [
        (
            "stage_interop.rs",
            include_str!("../src/interop/stage_interop.rs"),
        ),
        ("stage_pane.rs", include_str!("../src/stage_pane.rs")),
    ];
    for (name, src) in sources {
        for store in ["sqlite", "rusqlite", "Sqlite", "SQLite"] {
            assert!(
                !src.contains(store),
                "AC-007: {name} must not reference '{store}' (PostgreSQL/EventLedger only)"
            );
        }
        for verb in [".put(", ".delete(", ".patch("] {
            assert!(
                !src.contains(verb),
                "AC-007: {name} must not introduce unrelated mutation verb '{verb}'"
            );
        }
    }
    // The stage_interop client reuses the shared backend pool + base url (no second HTTP stack).
    let interop_src = include_str!("../src/interop/stage_interop.rs");
    assert!(
        interop_src.contains("x-hsk-session-token")
            && !interop_src.contains("x-hsk-actor-kind")
            && !interop_src.contains("native-stage-action:"),
        "Stage must authenticate with the server-validated native session and must not assert actor privilege or fabricate approval ids"
    );
    assert!(
        interop_src.contains("shared_http_client") && interop_src.contains("BACKEND_BASE_URL"),
        "AC-007: the Stage client must reuse the shared backend_client pool + base url (no second stack)"
    );
    // The same client performs the privileged capture POST and exact-byte GETs.
    assert!(
        interop_src.contains(".post(self.url(&path))") && interop_src.contains(".get(&url)"),
        "AC-007: the Stage client must implement capture and retrieval through the shared client"
    );
    println!(
        "AC-007 gate OK: no sqlite/rusqlite; shared client performs Stage capture and retrieval"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-066-OP / operator reopen item 4 — the LIVE embed-back leg surfaces EndpointAbsent to the operator.
//
// The library layer (fetch_stage_artifact + capture_and_embed_back + the round-trip surface) is proven
// above. THIS proof closes the operator's reopen: pressing "Embed Stage Capture" in the REAL HandshakeApp
// must actually RUN the embed-back read off-thread and leave the honest `EmbedBackEndpointAbsent` typed
// blocker VISIBLE on the Stage round-trip surface — not a dead no-op. A mock backend answers the Stage
// privileged create POST with 404, so the live off-thread operation resolves to the
// typed blocker deterministically without a real backend.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn embed_stage_capture_operator_path_surfaces_endpoint_absent() {
    use handshake_native::app::{HandshakeApp, HealthDisplayState};
    use handshake_native::backend_client::HealthInfo;
    use handshake_native::stage_pane::{EmbedBackOutcome, StageContent};

    // A mock backend that answers the capture POST with 404, so the live off-thread operation resolves to
    // the honest EmbedBackEndpointAbsent typed blocker.
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 404 Not Found",
        serde_json::json!({"error": "no stage route in this build"}),
    );

    // A runtime-injected shell (so the shell's off-thread embed-back spawn has a handle), pointed at the
    // mock server for the Stage embed-back read ONLY (production uses BACKEND_BASE_URL).
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    app.set_stage_embed_back_base_url_for_test(&base_url);
    // Seed routed content so the round-trip surface is a real round-trip (the embed-back button enabled).
    app.mounted_stage().lock().unwrap().set_content_correlated(
        StageContent::Selection("routed selection".to_owned(), "pane-rich:0-16".to_owned()),
        Some("stage-route-test-causal".to_owned()),
    );
    let target_pane = PaneId::from("pane-a");
    let mut target_tab = TabState::new(PaneType::LoomWikiPage);
    target_tab.content_id = Some("DOC-STAGE-ENDPOINT-ABSENT".to_owned());
    let target_bar = app
        .tab_bar_states_mut()
        .get_mut(&target_pane)
        .expect("default pane-a tab bar");
    target_bar.tabs = vec![target_tab];
    target_bar.active_index = 0;
    app.set_active_pane_for_test(Some(target_pane));

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    // OPERATOR ACTION: dispatch the "Embed Stage Capture" palette command through the REAL palette arm
    // (the same arm a clicked/Enter-run palette row reaches). This sets the embed-back drain flag AND opens
    // the Stage round-trip dock.
    let fired = harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_EMBED_STAGE_CAPTURE);
    assert!(
        fired,
        "operator route: the Embed Stage Capture palette command dispatched observably"
    );

    // Drive frames so the drain spawns the off-thread create, the mock answers 404, and the typed outcome
    // lands on the SHARED Stage pane. Poll bounded until the blocker surfaces (the mock is a real socket).
    let stage = harness.state().mounted_stage();
    let mut outcome = None;
    for _ in 0..120 {
        harness.run_steps(2);
        if let Some(o) = stage.lock().unwrap().last_embed_back.clone() {
            outcome = Some(o);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let outcome = outcome
        .expect("operator reopen item 4: pressing Embed Stage Capture produced an embed-back outcome (no longer a dead no-op)");
    match &outcome {
        EmbedBackOutcome::EndpointAbsent { probed_path } => {
            assert!(
                probed_path.contains("/stage/artifacts"),
                "the typed blocker names the probed Stage embed-back route; got '{probed_path}'"
            );
            println!("PT-066-op typed blocker OK: EndpointAbsent(probed='{probed_path}')");
        }
        other => panic!(
            "operator reopen item 4: the ABSENT Stage embed-back route must surface the honest \
             EmbedBackEndpointAbsent typed blocker (never a fabricated success), got {other:?}"
        ),
    }
    assert!(
        stage
            .lock()
            .unwrap()
            .has_embed_back_endpoint_absent_blocker(),
        "the host surfaces the endpoint-absent blocker upward to the WP validator"
    );

    // The operator ENDS UP looking at the round-trip surface: its container node renders AND the
    // empty-state banner (the addressable STAGE_EMBED_BACK_STATUS_AUTHOR_ID node) carries the EndpointAbsent
    // summary in the live AccessKit tree.
    use handshake_native::stage_pane::STAGE_EMBED_BACK_STATUS_AUTHOR_ID;
    harness.run_steps(2);
    let root = harness.root();
    let container_present = root
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some(STAGE_PANE_AUTHOR_ID));
    assert!(
        container_present,
        "operator route: the Stage round-trip surface (show_round_trip container '{STAGE_PANE_AUTHOR_ID}') \
         renders after the operator opened Embed Stage Capture"
    );
    let banner = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(STAGE_EMBED_BACK_STATUS_AUTHOR_ID))
        .expect(
            "operator route: the addressable embed-back status banner \
             ('stage-embed-back-status') renders on the round-trip surface",
        );
    let banner_value = banner
        .accesskit_node()
        .value()
        .map(|v| v.to_owned())
        .unwrap_or_default();
    assert!(
        banner_value.contains("endpoint not present"),
        "operator route: the EndpointAbsent empty-state banner is OPERATOR-VISIBLE (its node value names \
         the typed blocker); got '{banner_value}'"
    );

    let req_line = server.join().unwrap();
    assert!(
        req_line.starts_with("POST ") && req_line.contains("/stage/artifacts"),
        "operator route: the live embed-back drain issued the POST at the documented route; got '{req_line}'"
    );
    println!(
        "PT-066-op operator embed-back OK: CMD_EMBED_STAGE_CAPTURE -> live off-thread read ({req_line}) \
         -> EndpointAbsent typed blocker surfaced on the round-trip banner"
    );
}

// ── small AccessKit tree helpers (the proven MT-063 helpers) ──────────────────────────────────────

/// The `{:?}` role string of the first node with `author_id`, if present.
fn role_of(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<String> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

/// True if a node addressed `child_author` has an ancestor addressed `ancestor_author`.
fn author_under(root: &egui_kittest::Node<'_>, child_author: &str, ancestor_author: &str) -> bool {
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() != Some(child_author) {
            continue;
        }
        let mut cur = node.parent();
        while let Some(p) = cur {
            if p.accesskit_node().author_id() == Some(ancestor_author) {
                return true;
            }
            cur = p.parent();
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// MT-066 V4 remediation item 6: focused regression fixtures for BOTH reproduced foreign-key paths.
//
// The V3 failure was a foreign-key violation while deleting `kernel_event_ledger` rows that
// workspace-owned rows still referenced. The validator named two concrete constraints and warned
// against hard-coding only those two. A live schema inspection during this remediation found 68
// foreign keys targeting `kernel_event_ledger` (53 RESTRICT, 14 NO ACTION), so `detach_ledger_references`
// derives the referencing set DYNAMICALLY from `pg_constraint`.
//
// These two fixtures pin the exact paths the validator reproduced. Each seeds a real referencing row
// against a real tracked ledger event, then requires cleanup to finish with ZERO scoped rows and NO
// constraint failure.
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Seeds a Canvas board whose `event_ledger_event_id` references a tracked event (the first
/// constraint the validator hit), then proves cleanup completes with zero residue.
#[test]
fn mt066_canvas_board_event_ledger_reference_does_not_block_cleanup() {
    let _binding_env_guard = BINDING_ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    mt066_ledger_reference_regression(
        "loom_canvas_boards",
        "event_ledger_event_id",
        "mt066-canvas-board-fk",
    );
}

/// Seeds a `loom_block_knowledge_bridge` row whose `index_event_id` references a tracked event (the
/// second constraint the validator hit), then proves cleanup completes with zero residue.
#[test]
fn mt066_knowledge_bridge_index_event_reference_does_not_block_cleanup() {
    let _binding_env_guard = BINDING_ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    mt066_ledger_reference_regression(
        "loom_block_knowledge_bridge",
        "index_event_id",
        "mt066-knowledge-bridge-fk",
    );
}

/// Shared body for the two regression fixtures.
///
/// Deliberately parameterised by (table, column) rather than duplicated, because the DEFECT class is
/// "some row references a tracked ledger event", not "these two tables specifically". Adding a third
/// path is one call, and the dynamic detach already covers the other 66 constraints.
fn mt066_ledger_reference_regression(table: &str, column: &str, label: &str) {
    // MT-109 made the Flight Recorder route group fail-closed, and this fixture's cleanup reads
    // `GET /api/flight_recorder`. Publish a REAL native-MCP binding for this exact process (real
    // pid, real OS-issued process-birth identity) BEFORE the owned backend is selected, so the child
    // inherits the same isolated app-data root and authenticates the genuine credential.
    let session_token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let _binding = stage_binding_proof::StageBindingGuard::install(&session_token, label);

    let mut backend = interconnect_support::require_reachable_backend();
    let workspace = backend.create_workspace(&format!("{label}-{}", uuid::Uuid::new_v4().simple()));
    let workspace_id = workspace["id"]
        .as_str()
        .expect("workspace create returns id")
        .to_owned();

    // A REAL ledger identity: `kernel_event_ledger.event_id` is `text` carrying `KE-<uuid>` values,
    // not a `uuid` column. Seeding a bare UUID here would have hidden the `::uuid[]` cast defect the
    // detach originally shipped with, so the fixture uses the production id shape on purpose.
    let event_id = format!("KE-{}", uuid::Uuid::new_v4());
    let block_id = format!("LB-{}", uuid::Uuid::new_v4().simple());
    let entity_id = format!("KEN-{}", uuid::Uuid::new_v4().simple());
    let ws_sql = workspace_id.replace('\'', "''");

    // The referencing row the validator actually hit. `loom_canvas_boards.event_ledger_event_id` and
    // `loom_block_knowledge_bridge.index_event_id` are both NOT NULL RESTRICT foreign keys, so the
    // row cannot exist without its event and the ledger delete cannot proceed while it does.
    let reference_sql = match table {
        "loom_canvas_boards" => format!(
            "INSERT INTO loom_canvas_boards \
               (block_id, workspace_id, board_state, event_ledger_event_id) \
             VALUES ('{block_id}', '{ws_sql}', \
                     jsonb_build_object('schema_id', 'hsk.loom_canvas_board@1'), '{event_id}'); "
        ),
        "loom_block_knowledge_bridge" => format!(
            "INSERT INTO knowledge_entities \
               (entity_id, workspace_id, entity_kind, entity_key, display_name) \
             VALUES ('{entity_id}', '{ws_sql}', 'loom_block', '{block_id}', \
                     'MT-066 regression entity'); \
             INSERT INTO loom_block_knowledge_bridge \
               (block_id, workspace_id, entity_id, index_event_id) \
             VALUES ('{block_id}', '{ws_sql}', '{entity_id}', '{event_id}'); "
        ),
        other => panic!("MT-066 regression fixture has no seed for referencing table {other}"),
    };

    // Seed the real ledger event, its owning Loom block, and the real referencing row in ONE
    // transaction. Every NOT NULL ledger column is populated: the first V4 draft omitted nine of
    // them and the seed died on `null value in column "event_version"`, which meant the fixture
    // could never reach the behaviour it claimed to prove.
    backend.run_fixture_sql(
        &format!("{label}-seed"),
        &format!(
            "BEGIN; \
             INSERT INTO kernel_event_ledger \
               (event_id, event_version, kernel_task_run_id, session_run_id, aggregate_type, \
                aggregate_id, idempotency_key, event_type, actor_kind, actor_id, payload_hash, \
                source_component, payload) \
             VALUES ('{event_id}', '1', 'mt066-fk-regression-task', 'mt066-fk-regression-session', \
                     'stage_capture', '{ws_sql}', 'stage-capture:{ws_sql}:{label}', \
                     'stage.capture.recorded', 'operator', 'mt066-fk-regression', \
                     'mt066-fk-regression-payload-hash', 'stage_capture_api', \
                     jsonb_build_object('workspace_id', '{ws_sql}', 'kind', '{label}')); \
             INSERT INTO loom_blocks (block_id, workspace_id, content_type, title) \
             VALUES ('{block_id}', '{ws_sql}', 'note', 'MT-066 regression block'); \
             {reference_sql} \
             COMMIT;"
        ),
    );

    let mut cleanup = LiveWorkspaceGuard {
        backend: &mut backend,
        workspace_id: workspace_id.clone(),
        native_fr_event_ids: Vec::new(),
        stage_artifact_ids: Vec::new(),
        stage_job_ids: Vec::new(),
        stage_event_ids: vec![event_id.clone()],
        workspace_deleted: false,
        native_fr_cleanup_done: false,
        stage_side_effect_cleanup_done: false,
    };

    // Prove the referencing constraint exists and points where the validator said it does. If the
    // schema ever drops it, this fixture must fail loudly rather than silently proving nothing.
    cleanup.backend.run_fixture_sql(
        &format!("{label}-assert-constraint-exists"),
        &format!(
            "DO $mt066_assert_fk$ BEGIN \
               IF NOT EXISTS ( \
                 SELECT 1 FROM pg_constraint c \
                 JOIN LATERAL unnest(c.conkey) AS k(attnum) ON true \
                 JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum \
                 WHERE c.contype = 'f' \
                   AND c.confrelid = 'kernel_event_ledger'::regclass \
                   AND c.conrelid = '{table}'::regclass \
                   AND a.attname = '{column}') \
               THEN RAISE EXCEPTION \
                 'MT-066 regression fixture is vacuous: {table}.{column} no longer references kernel_event_ledger'; \
               END IF; \
               IF NOT EXISTS (SELECT 1 FROM {table} WHERE {column} = '{event_id}') \
               THEN RAISE EXCEPTION \
                 'MT-066 regression fixture is vacuous: no {table}.{column} row references the tracked event'; \
               END IF; \
             END $mt066_assert_fk$;"
        ),
    );

    // The actual regression, in the EXACT order the V3 run failed: remove the Stage/Flight-Recorder
    // residue (which deletes `kernel_event_ledger` rows) while the workspace-owned referencing row is
    // still present. Without the dynamic detach this raises
    // "update or delete on table kernel_event_ledger violates foreign key constraint".
    cleanup.cleanup_all_and_assert_zero();
    // Then the canonical finish path. The completion flags make the residue phases idempotent, so
    // this performs the product-owned workspace DELETE and re-asserts absence.
    cleanup.finish_and_assert_zero();

    // Independent read-only confirmation that both the tracked event and its referencing row are
    // gone. `event_id` is text, so no cast: a `::uuid` cast here would reject a real `KE-…` id.
    cleanup.backend.run_fixture_sql(
        &format!("{label}-assert-zero"),
        &format!(
            "DO $mt066_zero$ BEGIN \
               IF EXISTS (SELECT 1 FROM kernel_event_ledger WHERE event_id = '{event_id}') \
               THEN RAISE EXCEPTION 'MT-066 {label}: tracked ledger event survived cleanup'; END IF; \
               IF EXISTS (SELECT 1 FROM {table} WHERE {column} = '{event_id}') \
               THEN RAISE EXCEPTION 'MT-066 {label}: referencing {table} row survived cleanup'; END IF; \
               IF EXISTS (SELECT 1 FROM loom_blocks WHERE block_id = '{block_id}') \
               THEN RAISE EXCEPTION 'MT-066 {label}: scoped Loom block survived cleanup'; END IF; \
             END $mt066_zero$;"
        ),
    );

    println!("MT-066 regression OK: {table}.{column} reference did not block scoped cleanup");
}
