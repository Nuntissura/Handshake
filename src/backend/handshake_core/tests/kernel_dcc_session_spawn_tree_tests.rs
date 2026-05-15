use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    session_spawn_tree_dcc::{
        project_session_spawn_tree_dcc, validate_session_spawn_tree_dcc,
        SessionAnnounceBackBadgeV1, SessionRuntimeState, SessionSpawnMode,
        SessionSpawnRuntimeRecordV1, SessionSpawnTreeDccV1, SessionSpawnTreeVisibleField,
    },
};

#[test]
fn kernel_dcc_session_spawn_tree_projects_hierarchy_depth_and_child_counts() {
    let tree = sample_tree();
    validate_session_spawn_tree_dcc(&tree).expect("spawn tree validates");

    let projection = project_session_spawn_tree_dcc(&tree).expect("projection builds");

    let root = projection
        .nodes
        .iter()
        .find(|node| node.session_id == "session.root")
        .expect("root node");
    assert_eq!(root.depth, 0);
    assert_eq!(root.child_count, 2);
    assert_eq!(root.active_child_count, 2);
    assert!(root.cascade_cancel_available);

    let grandchild = projection
        .nodes
        .iter()
        .find(|node| node.session_id == "session.child.a.1")
        .expect("grandchild node");
    assert_eq!(grandchild.depth, 2);
    assert_eq!(
        grandchild.parent_session_id.as_deref(),
        Some("session.child.a")
    );
    assert_eq!(projection.max_depth, 2);
    assert!(!projection.mutates_runtime_records);
}

#[test]
fn kernel_dcc_session_spawn_tree_projects_spawn_modes_and_announce_back_badges() {
    let projection = project_session_spawn_tree_dcc(&sample_tree()).expect("projection builds");

    let child = projection
        .nodes
        .iter()
        .find(|node| node.session_id == "session.child.a")
        .expect("child node");

    assert_eq!(child.spawn_mode, SessionSpawnMode::SessionPersistent);
    assert!(child
        .announce_back_badges
        .contains(&"announce-back pending".to_string()));
    assert!(projection
        .visible_fields
        .contains(&SessionSpawnTreeVisibleField::CascadeCancel));
    assert!(projection
        .runtime_record_refs
        .iter()
        .all(|record_ref| record_ref.starts_with("runtime://session-spawn/")));
}

#[test]
fn kernel_dcc_session_spawn_tree_rejects_drift_and_unrenderable_records() {
    let mut tree = sample_tree();
    tree.visible_fields
        .retain(|field| *field != SessionSpawnTreeVisibleField::CascadeCancel);
    tree.runtime_records[0].runtime_record_ref.clear();
    tree.runtime_records[1].parent_session_id = Some("session.missing".to_string());
    tree.runtime_records[2].session_id = tree.runtime_records[3].session_id.clone();

    let errors = validate_session_spawn_tree_dcc(&tree).expect_err("unsafe tree must fail");

    assert!(errors.iter().any(|error| error.field == "visible_fields"));
    assert!(errors
        .iter()
        .any(|error| error.field == "runtime_records.runtime_record_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "runtime_records.parent_session_id"));
    assert!(errors
        .iter()
        .any(|error| error.field == "runtime_records.session_id"));
}

#[test]
fn kernel_dcc_session_spawn_tree_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.session_spawn_tree_dcc.project")
        .expect("session spawn tree DCC projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "session_spawn_tree_runtime_records"));
}

fn sample_tree() -> SessionSpawnTreeDccV1 {
    SessionSpawnTreeDccV1 {
        schema_id: "hsk.kernel.session_spawn_tree_dcc@1".to_string(),
        tree_id: "session-spawn-tree-mt043".to_string(),
        folded_stub_ids: vec!["WP-1-Session-Spawn-Tree-DCC-Visualization-v1".to_string()],
        panel_id: "dcc.session_spawn_tree".to_string(),
        visible_fields: vec![
            SessionSpawnTreeVisibleField::SpawnHierarchy,
            SessionSpawnTreeVisibleField::ChildCounts,
            SessionSpawnTreeVisibleField::SpawnDepth,
            SessionSpawnTreeVisibleField::CascadeCancel,
            SessionSpawnTreeVisibleField::SpawnMode,
            SessionSpawnTreeVisibleField::AnnounceBackBadges,
        ],
        runtime_records: vec![
            record(
                "session.root",
                None,
                SessionSpawnMode::SessionPersistent,
                true,
                vec![],
            ),
            record(
                "session.child.a",
                Some("session.root"),
                SessionSpawnMode::SessionPersistent,
                true,
                vec![badge(
                    "badge.child.a",
                    "session.child.a",
                    "announce-back pending",
                )],
            ),
            record(
                "session.child.b",
                Some("session.root"),
                SessionSpawnMode::OneShot,
                false,
                vec![badge("badge.child.b", "session.child.b", "announced")],
            ),
            record(
                "session.child.a.1",
                Some("session.child.a"),
                SessionSpawnMode::OneShot,
                false,
                vec![],
            ),
        ],
        product_authority_refs: vec![
            "kernel.dcc_mvp_runtime_surface".to_string(),
            "kernel.role_mailbox_inbox_evidence_bridge".to_string(),
            "kernel.session_anti_pattern_registry".to_string(),
            "flight_recorder.session_spawn".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Session-Spawn-Tree-DCC-Visualization-v1.contract.json"
                .to_string(),
        ],
    }
}

fn record(
    session_id: &str,
    parent_session_id: Option<&str>,
    spawn_mode: SessionSpawnMode,
    cascade_cancel_supported: bool,
    announce_back_badges: Vec<SessionAnnounceBackBadgeV1>,
) -> SessionSpawnRuntimeRecordV1 {
    SessionSpawnRuntimeRecordV1 {
        session_id: session_id.to_string(),
        parent_session_id: parent_session_id.map(str::to_string),
        role_id: "CODER".to_string(),
        spawn_mode,
        runtime_state: SessionRuntimeState::Active,
        cascade_cancel_supported,
        announce_back_badges,
        runtime_record_ref: format!("runtime://session-spawn/{session_id}"),
        flight_recorder_ref: format!("FR-EVT-SESSION-SPAWN-{}", session_id.replace('.', "-")),
    }
}

fn badge(badge_id: &str, session_id: &str, label: &str) -> SessionAnnounceBackBadgeV1 {
    SessionAnnounceBackBadgeV1 {
        badge_id: badge_id.to_string(),
        session_id: session_id.to_string(),
        label: label.to_string(),
        mailbox_route: format!("role-mailbox://{session_id}/announce-back"),
    }
}
