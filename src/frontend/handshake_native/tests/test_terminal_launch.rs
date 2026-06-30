//! WP-KERNEL-012 MT-100: native terminal launch affordance.
//!
//! The backend PTY runtime exists, but the native frontend has no HTTP terminal-session route today.
//! These tests prove the cwd+wrapper request is typed and the product exposes an honest blocker rather
//! than a fake terminal session.

use egui_kittest::{
    kittest::{NodeT, Queryable},
    Harness,
};
use handshake_native::app::{HandshakeApp, TERMINAL_LAUNCH_STATUS_AUTHOR_ID};
use handshake_native::backend_client::{
    TerminalLaunchClient, TerminalLaunchError, TERMINAL_LAUNCH_IPC_CHANNEL,
    TERMINAL_LAUNCH_IPC_OWNER, TERMINAL_LAUNCH_PROBED_PATH,
};
use handshake_native::command_registry::{
    all_commands, effective_disabled, CommandKind, CMD_TERMINAL_OPEN_WORKSPACE,
    TERMINAL_OPEN_WORKSPACE_STABLE_ID,
};

fn live_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

#[test]
fn terminal_launch_client_returns_endpoint_missing_without_fake_session() {
    let client = TerminalLaunchClient::new("http://127.0.0.1:37501");
    let err = client
        .open_workspace_terminal("D:/Projects/Handshake/repo")
        .expect_err("native terminal launch is a typed blocker until an HTTP route exists");

    assert!(err.is_endpoint_missing());
    let request = err.request();
    assert_eq!(request.cwd, "D:/Projects/Handshake/repo");
    assert!(
        !request.shell.trim().is_empty(),
        "shell wrapper must be carried even while the route is blocked"
    );
    assert_eq!(request.rows, 24);
    assert_eq!(request.cols, 80);

    match err {
        TerminalLaunchError::EndpointMissing {
            probed_path,
            probed_url,
            ipc_channel,
            ipc_owner,
            request,
        } => {
            assert_eq!(probed_path, TERMINAL_LAUNCH_PROBED_PATH);
            assert_eq!(probed_url, "http://127.0.0.1:37501/terminal/sessions");
            assert_eq!(ipc_channel, TERMINAL_LAUNCH_IPC_CHANNEL);
            assert_eq!(ipc_owner, TERMINAL_LAUNCH_IPC_OWNER);
            assert_eq!(request.cwd, "D:/Projects/Handshake/repo");
        }
    }
}

#[test]
fn terminal_launch_command_is_addressable_and_runs_to_blocker_status() {
    let row = all_commands()
        .iter()
        .find(|cmd| cmd.id == CMD_TERMINAL_OPEN_WORKSPACE)
        .expect("terminal workspace launch command is present");

    assert_eq!(row.kind, CommandKind::App);
    assert_eq!(row.stable_id, TERMINAL_OPEN_WORKSPACE_STABLE_ID);
    assert_eq!(row.label, "Terminal: Open in Workspace Folder");
    assert!(!row.disabled);
    assert!(!effective_disabled(row, true));
    assert!(row.description.contains("EndpointMissing"));
    assert!(row.description.contains("/terminal/sessions"));
    assert!(row.description.contains("Tauri IPC-only"));
}

#[test]
fn run_menu_terminal_click_surfaces_endpoint_missing_status_node() {
    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_eframe(|cc| HandshakeApp::new(cc));
    harness.run_steps(4);

    harness.get_by_label("RUN").click();
    harness.run_steps(2);
    harness
        .get_by_label("Open Terminal in Workspace Folder")
        .click();
    harness.run_steps(2);

    let status = harness
        .state()
        .terminal_launch_status_for_test()
        .expect("terminal click records a visible typed status");
    assert!(status.contains("EndpointMissing"));
    assert!(status.contains("/terminal/sessions"));
    assert!(status.contains("kernel_terminal_create_session"));

    let nodes = live_author_nodes(&harness);
    let (_, role, label) = nodes
        .iter()
        .find(|(author_id, _, _)| author_id == TERMINAL_LAUNCH_STATUS_AUTHOR_ID)
        .unwrap_or_else(|| {
            panic!(
                "terminal launch status node '{TERMINAL_LAUNCH_STATUS_AUTHOR_ID}' must be live: {nodes:?}"
            )
        });
    assert_eq!(role, "Status");
    let label = label
        .as_deref()
        .expect("terminal status node carries label");
    assert!(label.contains("EndpointMissing"));
    assert!(label.contains("/terminal/sessions"));
}

#[test]
fn palette_terminal_dispatch_surfaces_same_endpoint_missing_status() {
    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_eframe(|cc| HandshakeApp::new(cc));
    harness.run_steps(4);

    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test(CMD_TERMINAL_OPEN_WORKSPACE),
        "palette dispatch returns an observable typed blocker"
    );

    let status = harness
        .state()
        .terminal_launch_status_for_test()
        .expect("palette dispatch records terminal status");
    assert!(status.contains("EndpointMissing"));
    assert!(status.contains("/terminal/sessions"));
    assert!(status.contains("kernel_terminal_create_session"));
}
