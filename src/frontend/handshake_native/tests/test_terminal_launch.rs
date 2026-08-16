//! WP-KERNEL-012 MT-100: native terminal launch affordance.
//!
//! The backend PTY runtime exists, but the native frontend has no HTTP terminal-session route today.
//! These tests prove the cwd+wrapper request is typed and the product exposes an honest blocker rather
//! than a fake terminal session.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
use handshake_native::app::{
    HandshakeApp, HealthDisplayState, TERMINAL_LAUNCH_STATUS_AUTHOR_ID,
    WORKSPACE_ROOT_APPLY_AUTHOR_ID, WORKSPACE_ROOT_DIALOG_AUTHOR_ID, WORKSPACE_ROOT_PATH_AUTHOR_ID,
};
use handshake_native::backend_client::{
    HealthInfo, TerminalLaunchClient, TerminalLaunchError, TERMINAL_LAUNCH_IPC_CHANNEL,
    TERMINAL_LAUNCH_IPC_OWNER, TERMINAL_LAUNCH_PROBED_PATH,
};
use handshake_native::command_registry::{
    all_commands, effective_disabled, CommandKind, EditorMenuEnableContext,
    CMD_MODEL_SESSION_LAUNCH_WORKSPACE, CMD_TERMINAL_OPEN_WORKSPACE,
    TERMINAL_OPEN_WORKSPACE_STABLE_ID,
};
use handshake_native::project_tabs::{ProjectItem, WorkspaceRootError};

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

fn test_root(relative: impl AsRef<Path>) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(relative)
        .canonicalize()
        .expect("checked-in test root exists")
}

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }))
}

fn bind_default_root(app: &mut HandshakeApp) -> String {
    let root = test_root("");
    app.bind_active_workspace_root(&root)
        .expect("default workspace accepts canonical test root");
    root.to_str().expect("test path is Unicode").to_owned()
}

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

fn set_value_by_author(harness: &mut Harness<'_, HandshakeApp>, author_id: &str, value: &str) {
    let node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("missing SetValue target {author_id}"));
    assert!(
        node.accesskit_node()
            .data()
            .supports_action(egui::accesskit::Action::SetValue),
        "{author_id} must advertise canonical SetValue"
    );
    let node_id = node.accesskit_node().id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::SetValue,
            target: node_id,
            data: Some(egui::accesskit::ActionData::Value(value.into())),
        },
    ));
    harness.run_steps(2);
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
    assert!(!effective_disabled(
        row,
        EditorMenuEnableContext::unavailable()
    ));
    assert!(row.description.contains("EndpointMissing"));
    assert!(row.description.contains("/terminal/sessions"));
    assert!(row.description.contains("Tauri IPC-only"));
}

#[test]
fn run_menu_terminal_click_surfaces_endpoint_missing_status_node() {
    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_eframe(|cc| HandshakeApp::new(cc));
    let expected_root = bind_default_root(harness.state_mut());
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
    assert!(
        status.contains(&expected_root),
        "operator-visible blocker reports the exact requested workspace root: {status}"
    );

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
    let expected_root = bind_default_root(harness.state_mut());
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
    assert!(status.contains(&expected_root));
}

#[test]
fn two_workspaces_route_terminal_and_model_seed_to_their_own_canonical_roots() {
    let root_a = test_root("src");
    let root_b = test_root("tests");
    assert_ne!(
        root_a, root_b,
        "proof requires two genuinely different roots"
    );
    let process_cwd = std::env::current_dir().expect("process cwd is readable");
    assert!(
        root_a != process_cwd || root_b != process_cwd,
        "at least one chosen workspace root must differ from process cwd"
    );

    let projects = vec![
        ProjectItem::new("workspace-a", "Workspace A")
            .try_with_filesystem_root(&root_a)
            .expect("workspace A root"),
        ProjectItem::new("workspace-b", "Workspace B")
            .try_with_filesystem_root(&root_b)
            .expect("workspace B root"),
    ];
    let mut app = ok_app();
    app.project_tabs_mut().apply_fetched(projects);
    app.project_tabs_mut().apply_fetched(vec![
        ProjectItem::new("workspace-a", "Workspace A refreshed"),
        ProjectItem::new("workspace-b", "Workspace B refreshed"),
    ]);

    let expected_a = root_a.to_str().expect("Unicode path").to_owned();
    app.bind_active_project_for_integration_test("workspace-a");
    let (terminal_a, model_a) = app
        .active_workspace_launch_folders_for_test()
        .expect("workspace A resolves");
    assert_eq!(terminal_a, expected_a);
    assert_eq!(model_a, expected_a);
    assert!(app.dispatch_palette_action_for_test(CMD_TERMINAL_OPEN_WORKSPACE));
    assert!(app
        .terminal_launch_status_for_test()
        .is_some_and(|status| status.contains(&expected_a)));
    assert!(app.dispatch_palette_action_for_test(CMD_MODEL_SESSION_LAUNCH_WORKSPACE));
    assert_eq!(
        app.model_session_workspace_folder_for_test(),
        Some(expected_a.as_str())
    );
    app.close_model_session_launch_dialog_for_test();

    let expected_b = root_b.to_str().expect("Unicode path").to_owned();
    app.bind_active_project_for_integration_test("workspace-b");
    let (terminal_b, model_b) = app
        .active_workspace_launch_folders_for_test()
        .expect("workspace B resolves");
    assert_eq!(terminal_b, expected_b);
    assert_eq!(model_b, expected_b);
    assert_ne!(terminal_a, terminal_b);
    assert!(app.dispatch_palette_action_for_test(CMD_TERMINAL_OPEN_WORKSPACE));
    assert!(app
        .terminal_launch_status_for_test()
        .is_some_and(|status| status.contains(&expected_b)));
    assert!(app.dispatch_palette_action_for_test(CMD_MODEL_SESSION_LAUNCH_WORKSPACE));
    assert_eq!(
        app.model_session_workspace_folder_for_test(),
        Some(expected_b.as_str())
    );
}

#[test]
fn model_launch_request_keeps_the_workspace_captured_when_dialog_opened() {
    let root_a = test_root("src");
    let root_b = test_root("tests");
    let mut app = ok_app();
    app.project_tabs_mut().apply_fetched(vec![
        ProjectItem::new("workspace-a", "Workspace A")
            .try_with_filesystem_root(&root_a)
            .expect("workspace A root"),
        ProjectItem::new("workspace-b", "Workspace B")
            .try_with_filesystem_root(&root_b)
            .expect("workspace B root"),
    ]);
    app.bind_active_project_for_integration_test("workspace-a");
    assert!(app.dispatch_palette_action_for_test(CMD_MODEL_SESSION_LAUNCH_WORKSPACE));

    app.bind_active_project_for_integration_test("workspace-b");
    let request = app
        .model_session_launch_request_for_test()
        .expect("open model dialog builds a request from its captured context");
    assert_eq!(request.workspace_id, "workspace-a");
    assert_eq!(
        request.workspace_folder,
        root_a.to_str().expect("Unicode root A")
    );
    assert_ne!(
        request.workspace_folder,
        root_b.to_str().expect("Unicode root B")
    );
}

#[test]
fn file_open_workspace_dialog_is_the_reachable_no_context_root_recovery_path() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(2);

    harness.get_by_label("FILE").click();
    harness.run_steps(2);
    harness.get_by_label("Open Workspace…").click();
    harness.run_steps(2);
    assert!(harness.state().workspace_root_dialog_open_for_test());
    let nodes = live_author_nodes(&harness);
    assert!(nodes
        .iter()
        .any(|(id, role, _)| id == WORKSPACE_ROOT_DIALOG_AUTHOR_ID && role == "Dialog"));

    let expected_root = test_root("src");
    let expected_text = expected_root.to_str().expect("Unicode path");
    set_value_by_author(&mut harness, WORKSPACE_ROOT_PATH_AUTHOR_ID, expected_text);
    let apply = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(WORKSPACE_ROOT_APPLY_AUTHOR_ID))
        .expect("workspace-root Apply node")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: apply,
            data: None,
        },
    ));
    harness.run_steps(3);

    assert!(!harness.state().workspace_root_dialog_open_for_test());
    let (terminal, model) = harness
        .state()
        .active_workspace_launch_folders_for_test()
        .expect("FILE recovery binds the canonical root");
    assert_eq!(terminal, expected_text);
    assert_eq!(model, expected_text);
}

#[test]
fn workspace_root_dialog_has_material_visual_proof() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(2);
    harness.get_by_label("FILE").click();
    harness.run_steps(2);
    harness.get_by_label("Open Workspace…").click();
    harness.run_steps(2);

    let Some(image) = harness.render_proof_frame(
        "MT-125 workspace-root dialog must be readable on the real native app host",
    ) else {
        return;
    };
    assert_eq!((image.width(), image.height()), (900, 760));
    let out_dir = Path::new("../../../../Handshake_Artifacts/handshake-test/wp-kernel-012-mt-125");
    std::fs::create_dir_all(out_dir).expect("create MT-125 visual-proof directory");
    let out_path = out_dir.join("workspace-root-dialog.png");
    image
        .save(&out_path)
        .expect("save MT-125 workspace-root dialog screenshot");
    println!(
        "MT-125 workspace-root dialog screenshot: {}",
        out_path.display()
    );
}

#[test]
fn workspace_removed_while_root_dialog_is_open_fails_typed_without_panic() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(2);
    harness.get_by_label("FILE").click();
    harness.run_steps(2);
    harness.get_by_label("Open Workspace…").click();
    harness.run_steps(2);

    harness
        .state_mut()
        .project_tabs_mut()
        .apply_fetched(vec![ProjectItem::new("replacement", "Replacement")]);
    let existing_root = test_root("src");
    set_value_by_author(
        &mut harness,
        WORKSPACE_ROOT_PATH_AUTHOR_ID,
        existing_root.to_str().expect("Unicode fixture root"),
    );
    let apply = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(WORKSPACE_ROOT_APPLY_AUTHOR_ID))
        .expect("workspace-root Apply node")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: apply,
            data: None,
        },
    ));
    harness.run_steps(3);

    assert!(
        harness.state().workspace_root_dialog_open_for_test(),
        "failed bind remains recoverable in the same dialog"
    );
    let nodes = live_author_nodes(&harness);
    assert!(nodes.iter().any(|(id, role, label)| {
        id == handshake_native::app::WORKSPACE_ROOT_STATUS_AUTHOR_ID
            && role == "Status"
            && label.as_deref().is_some_and(|value| {
                value.contains("WorkspaceNotOpen workspace_id=default-project")
            })
    }));
}

#[test]
fn workspace_without_root_fails_typed_and_operator_visible_without_dialog_or_cwd_fallback() {
    let relative_error = ProjectItem::new("relative", "Relative")
        .try_with_filesystem_root("relative/workspace")
        .expect_err("relative roots cannot be interpreted through process cwd");
    assert!(matches!(
        relative_error,
        WorkspaceRootError::NotAbsolute { ref workspace_id, .. }
            if workspace_id == "relative"
    ));

    let mut app = ok_app();
    let err = app
        .active_workspace_launch_folders_for_test()
        .expect_err("default workspace is intentionally unbound");
    assert!(matches!(
        err,
        WorkspaceRootError::Missing { ref workspace_id }
            if workspace_id == "default-project"
    ));

    assert!(app.dispatch_palette_action_for_test(CMD_TERMINAL_OPEN_WORKSPACE));
    let terminal_status = app
        .terminal_launch_status_for_test()
        .expect("terminal root blocker is visible");
    assert!(terminal_status.contains("WorkspaceRootMissing"));
    assert!(terminal_status.contains("FILE > Open Workspace"));
    assert!(app.workspace_root_dialog_open_for_test());
    assert!(!terminal_status.contains(&std::env::current_dir().unwrap().display().to_string()));

    // Use an independent unbound app so model-session recovery is not satisfied by terminal recovery.
    let mut app = ok_app();
    assert!(app.dispatch_palette_action_for_test(CMD_MODEL_SESSION_LAUNCH_WORKSPACE));
    let model_status = app
        .model_session_launch_status_for_test()
        .expect("model root blocker is visible");
    assert!(model_status.contains("WorkspaceRootMissing"));
    assert!(model_status.contains("FILE > Open Workspace"));
    assert!(app.workspace_root_dialog_open_for_test());
    assert!(!app.model_session_launch_dialog_open_for_test());
}
