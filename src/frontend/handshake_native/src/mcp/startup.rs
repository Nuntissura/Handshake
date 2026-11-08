//! The native process's existing MCP channel, started before renderer construction.
//! Channel possession is not an account session, renderer grant or GPU issuer binding.

use std::sync::{Arc, Mutex};

use super::{ActionChannel, McpBinding, SessionToken, SwarmMcpServer, WindowSnapshotRegistry};
use crate::accessibility::UiTreeSnapshot;

pub(crate) const BACKGROUND_RUNTIME_SHUTDOWN_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(10);

#[derive(Clone)]
pub(crate) struct McpStartupState {
    pub(crate) token: SessionToken,
    pub(crate) snapshot: Arc<Mutex<UiTreeSnapshot>>,
    pub(crate) channel: Arc<Mutex<ActionChannel>>,
    pub(crate) windows: Arc<Mutex<WindowSnapshotRegistry>>,
    pub(crate) durability: super::server::SharedArgusDurabilityBinding,
}

impl McpStartupState {
    pub(crate) fn new() -> Self {
        let snapshot = Arc::new(Mutex::new(empty_snapshot()));
        let channel = Arc::new(Mutex::new(ActionChannel::new()));
        let windows = Arc::new(Mutex::new(WindowSnapshotRegistry::new(
            snapshot.clone(),
            channel.clone(),
        )));
        Self {
            token: SessionToken::generate(),
            snapshot,
            channel,
            windows,
            durability: Arc::new(Mutex::new(None)),
        }
    }
}

/// Owns startup resources until the app takes them, including renderer-construction failure cleanup.
pub struct NativeMcpStartup {
    runtime: Option<tokio::runtime::Runtime>,
    state: McpStartupState,
    server: Option<SwarmMcpServer>,
}

impl NativeMcpStartup {
    pub fn start() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("build tokio runtime");
        let mut startup = Self {
            runtime: Some(runtime),
            state: McpStartupState::new(),
            server: None,
        };
        let state = &startup.state;
        let result = startup.runtime.as_ref().expect("startup runtime").block_on(
            SwarmMcpServer::bind_with_windows_and_durability(
                state.token.clone(),
                state.snapshot.clone(),
                state.channel.clone(),
                SwarmMcpServer::os_window_capture(),
                state.windows.clone(),
                state.durability.clone(),
            ),
        );
        match result {
            Ok(server) => {
                tracing::info!(tcp = %server.tcp_addr(), pipe = ?server.pipe_name(), "MCP swarm server bound");
                startup.server = Some(server);
            }
            Err(error) => {
                tracing::warn!(%error, "MCP swarm server bind failed; model-steering transport disabled this session");
            }
        }
        startup
    }

    /// Existing discovery facts only. This does not issue renderer or protected-resource authority.
    pub fn binding(&self) -> Option<&McpBinding> {
        self.server.as_ref().map(SwarmMcpServer::binding)
    }

    pub(crate) fn state(&self) -> McpStartupState {
        self.state.clone()
    }

    pub(crate) fn take_runtime(&mut self) -> tokio::runtime::Runtime {
        self.runtime.take().expect("startup runtime transfers once")
    }

    pub(crate) fn take_server(&mut self) -> Option<SwarmMcpServer> {
        self.server.take()
    }
}

impl Drop for NativeMcpStartup {
    fn drop(&mut self) {
        if let Some(mut server) = self.server.take() {
            shutdown_server(&mut server);
        }
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(BACKGROUND_RUNTIME_SHUTDOWN_TIMEOUT);
        }
    }
}

pub(crate) fn shutdown_server(server: &mut SwarmMcpServer) {
    for _ in 0..3 {
        server.shutdown();
        if server.binding_removed() {
            break;
        }
        std::thread::yield_now();
    }
}

/// The placeholder UI-tree snapshot the MCP slot (MT-027) holds before the first frame publishes the
/// real live tree. A single `Window` root with `widget_count` 1, so `list_widgets` over the wire before
/// the first frame returns a well-formed (if empty) snapshot rather than a lock on uninitialized state.
pub(crate) fn empty_snapshot() -> crate::accessibility::UiTreeSnapshot {
    crate::accessibility::UiTreeSnapshot {
        root: crate::accessibility::UiTreeNode {
            id: "node:root".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children: Vec::new(),
        },
        captured_at_utc: "0.000000000Z".to_owned(),
        widget_count: 1,
        // Nothing has been laid out yet, so there is no coordinate space to declare. A reader must not
        // treat the placeholder as window-relative geometry.
        viewport: None,
    }
}
