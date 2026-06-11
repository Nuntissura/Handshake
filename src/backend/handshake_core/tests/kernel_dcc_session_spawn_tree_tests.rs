use chrono::Utc;
use handshake_core::api::kernel::{
    derive_session_spawn_runtime_evidence, SessionSpawnRuntimeEvidence,
};
use handshake_core::flight_recorder::{
    FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    session_spawn_tree_dcc::{
        project_session_spawn_tree_dcc, validate_session_spawn_tree_dcc,
        SessionAnnounceBackBadgeV1, SessionRuntimeState, SessionSpawnMode,
        SessionSpawnRuntimeRecordV1, SessionSpawnTreeDccV1, SessionSpawnTreeVisibleField,
    },
};
use handshake_core::storage::{ModelSession, ModelSessionState};
use serde_json::json;
use uuid::Uuid;

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

// --- MT-043 runtime-derivation coverage --------------------------------------
//
// These tests pin `derive_session_spawn_runtime_evidence` to the documented
// behavior of the api/kernel.rs evidence builder:
//   1. announce-back badges originate from Flight Recorder
//      `SessionSpawnAnnounceBack` events whose payload child_session_id is in
//      the active session set;
//   2. cascade-cancel is derived from explicit capability_grants on the
//      ModelSession, *not* hardcoded for "every persistent session";
//   3. cascade-cancel is additively derived from Flight Recorder
//      `SessionCascadeCancel` events whose payload root_session_id matches an
//      in-scope session.
// Each test fails if a future change reverts to hardcoded synthesis.

#[test]
fn kernel_dcc_session_spawn_tree_mt043_announce_back_badges_derive_from_flight_recorder_payload_fields(
) {
    let parent = model_session_for_test("session.parent", Vec::new());
    let child = model_session_for_test("session.child", Vec::new());
    let other = model_session_for_test("session.other-window", Vec::new());

    let spawn_accept_event = flight_recorder_event(
        FlightRecorderEventType::SessionSpawnAccepted,
        json!({"child_session_id": "session.child"}),
    );
    let parent_spawn_event = flight_recorder_event(
        FlightRecorderEventType::SessionCreated,
        json!({"session_id": "session.parent"}),
    );
    let announce_event = flight_recorder_event(
        FlightRecorderEventType::SessionSpawnAnnounceBack,
        json!({
            "child_session_id": "session.child",
            "status": "delivered",
            "mailbox_message_id": "mailbox:role:announce-back-1",
        }),
    );
    // Out-of-scope payload — must be filtered.
    let out_of_scope_event = flight_recorder_event(
        FlightRecorderEventType::SessionSpawnAnnounceBack,
        json!({
            "child_session_id": "session.unknown",
            "status": "delivered",
            "mailbox_message_id": "mailbox:role:announce-back-other",
        }),
    );

    let evidence: SessionSpawnRuntimeEvidence = derive_session_spawn_runtime_evidence(
        &[parent, child, other],
        &[
            parent_spawn_event,
            spawn_accept_event,
            announce_event,
            out_of_scope_event,
        ],
    );

    let badges = evidence
        .announce_back_badges
        .get("session.child")
        .expect("announce-back badge expected for session.child");
    assert_eq!(badges.len(), 1);
    let badge = &badges[0];
    assert_eq!(badge.session_id, "session.child");
    assert!(badge.label.contains("delivered"));
    assert!(badge.mailbox_route.contains("session.child"));
    assert!(badge.mailbox_route.contains("announce-back"));
    assert!(badge.mailbox_route.contains("mailbox/role/announce-back-1"));
    assert!(!evidence
        .announce_back_badges
        .contains_key("session.unknown"));
}

#[test]
fn kernel_dcc_session_spawn_tree_mt043_cascade_cancel_derives_from_session_capability_grants() {
    let cancellable = model_session_for_test(
        "session.cancellable",
        vec!["session.cascade_cancel".to_string()],
    );
    let other = model_session_for_test("session.other", Vec::new());

    let evidence = derive_session_spawn_runtime_evidence(&[cancellable, other], &[]);

    assert!(evidence
        .cascade_cancel_session_ids
        .contains("session.cancellable"));
    assert!(!evidence
        .cascade_cancel_session_ids
        .contains("session.other"));
}

#[test]
fn kernel_dcc_session_spawn_tree_mt043_cascade_cancel_event_adds_root_session_to_evidence() {
    let root = model_session_for_test("session.root", Vec::new());
    let child = model_session_for_test("session.child", Vec::new());

    let cascade_cancel_event = flight_recorder_event(
        FlightRecorderEventType::SessionCascadeCancel,
        json!({"root_session_id": "session.root"}),
    );
    let unrelated_cancel_event = flight_recorder_event(
        FlightRecorderEventType::SessionCascadeCancel,
        json!({"root_session_id": "session.not-in-scope"}),
    );

    let evidence = derive_session_spawn_runtime_evidence(
        &[root, child],
        &[cascade_cancel_event, unrelated_cancel_event],
    );

    assert!(evidence.cascade_cancel_session_ids.contains("session.root"));
    assert!(!evidence
        .cascade_cancel_session_ids
        .contains("session.not-in-scope"));
}

fn model_session_for_test(session_id: &str, capability_grants: Vec<String>) -> ModelSession {
    let now = Utc::now();
    ModelSession {
        session_id: session_id.to_string(),
        parent_session_id: None,
        spawn_depth: 0,
        state: ModelSessionState::Active,
        model_id: "gpt-5.5".to_string(),
        backend: "codex".to_string(),
        parameter_class: "default".to_string(),
        role: "CODER".to_string(),
        wp_id: None,
        mt_id: None,
        work_profile_id: None,
        execution_mode: "session_persistent".to_string(),
        memory_policy: "default".to_string(),
        consent_receipt_id: None,
        capability_grants,
        capability_token_ids: None,
        job_id: None,
        checkpoint_artifact_id: None,
        last_checkpoint_at: None,
        checkpoint_count: 0,
        merge_back_artifact: None,
        agent: None,
        purpose: None,
        close_reason: None,
        closed_by_actor: None,
        closed_at: None,
        created_at: now,
        updated_at: now,
    }
}

fn flight_recorder_event(
    event_type: FlightRecorderEventType,
    payload: serde_json::Value,
) -> FlightRecorderEvent {
    FlightRecorderEvent::new(
        event_type,
        FlightRecorderActor::System,
        Uuid::now_v7(),
        payload,
    )
}
