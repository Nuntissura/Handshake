use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub const FOLDED_SESSION_SPAWN_TREE_DCC_STUB_ID: &str =
    "WP-1-Session-Spawn-Tree-DCC-Visualization-v1";

const REQUIRED_VISIBLE_FIELDS: [SessionSpawnTreeVisibleField; 6] = [
    SessionSpawnTreeVisibleField::SpawnHierarchy,
    SessionSpawnTreeVisibleField::ChildCounts,
    SessionSpawnTreeVisibleField::SpawnDepth,
    SessionSpawnTreeVisibleField::CascadeCancel,
    SessionSpawnTreeVisibleField::SpawnMode,
    SessionSpawnTreeVisibleField::AnnounceBackBadges,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionSpawnTreeVisibleField {
    SpawnHierarchy,
    ChildCounts,
    SpawnDepth,
    CascadeCancel,
    SpawnMode,
    AnnounceBackBadges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionSpawnMode {
    OneShot,
    SessionPersistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionRuntimeState {
    Active,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAnnounceBackBadgeV1 {
    pub badge_id: String,
    pub session_id: String,
    pub label: String,
    pub mailbox_route: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSpawnRuntimeRecordV1 {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub role_id: String,
    pub spawn_mode: SessionSpawnMode,
    pub runtime_state: SessionRuntimeState,
    pub cascade_cancel_supported: bool,
    pub announce_back_badges: Vec<SessionAnnounceBackBadgeV1>,
    pub runtime_record_ref: String,
    pub flight_recorder_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSpawnTreeDccV1 {
    pub schema_id: String,
    pub tree_id: String,
    pub folded_stub_ids: Vec<String>,
    pub panel_id: String,
    pub visible_fields: Vec<SessionSpawnTreeVisibleField>,
    pub runtime_records: Vec<SessionSpawnRuntimeRecordV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpawnTreeNodeProjectionV1 {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub role_id: String,
    pub depth: usize,
    pub child_count: usize,
    pub active_child_count: usize,
    pub spawn_mode: SessionSpawnMode,
    pub runtime_state: SessionRuntimeState,
    pub cascade_cancel_available: bool,
    pub announce_back_badges: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpawnTreeDccProjectionV1 {
    pub schema_id: String,
    pub tree_id: String,
    pub panel_id: String,
    pub visible_fields: Vec<SessionSpawnTreeVisibleField>,
    pub nodes: Vec<SessionSpawnTreeNodeProjectionV1>,
    pub max_depth: usize,
    pub cascade_cancel_session_ids: Vec<String>,
    pub announce_back_badge_count: usize,
    pub runtime_record_refs: Vec<String>,
    pub mutates_runtime_records: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpawnTreeDccValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_session_spawn_tree_dcc(
    tree: &SessionSpawnTreeDccV1,
) -> Result<(), Vec<SessionSpawnTreeDccValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &tree.schema_id);
    require_non_empty(&mut errors, "tree_id", &tree.tree_id);
    require_non_empty(&mut errors, "panel_id", &tree.panel_id);
    require_vec(&mut errors, "folded_stub_ids", &tree.folded_stub_ids);
    require_vec(&mut errors, "visible_fields", &tree.visible_fields);
    require_vec(&mut errors, "runtime_records", &tree.runtime_records);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &tree.product_authority_refs,
    );
    require_vec(&mut errors, "folded_source_refs", &tree.folded_source_refs);

    if !contains_exact(&tree.folded_stub_ids, FOLDED_SESSION_SPAWN_TREE_DCC_STUB_ID) {
        errors.push(SessionSpawnTreeDccValidationError {
            field: "folded_stub_ids",
            message: "spawn tree DCC must preserve the folded stub id",
        });
    }
    if !contains_text(
        &tree.folded_source_refs,
        FOLDED_SESSION_SPAWN_TREE_DCC_STUB_ID,
    ) {
        errors.push(SessionSpawnTreeDccValidationError {
            field: "folded_source_refs",
            message: "spawn tree DCC must preserve the folded source reference",
        });
    }

    validate_visible_fields(&mut errors, tree);
    validate_authority_refs(&mut errors, tree);
    validate_runtime_records(&mut errors, tree);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_session_spawn_tree_dcc(
    tree: &SessionSpawnTreeDccV1,
) -> Result<SessionSpawnTreeDccProjectionV1, Vec<SessionSpawnTreeDccValidationError>> {
    validate_session_spawn_tree_dcc(tree)?;

    let children_by_parent = children_by_parent(tree);
    let record_by_id: HashMap<&str, &SessionSpawnRuntimeRecordV1> = tree
        .runtime_records
        .iter()
        .map(|record| (record.session_id.as_str(), record))
        .collect();

    let mut nodes: Vec<SessionSpawnTreeNodeProjectionV1> = tree
        .runtime_records
        .iter()
        .map(|record| {
            let children = children_by_parent
                .get(record.session_id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let active_child_count = children
                .iter()
                .filter(|child_id| {
                    record_by_id
                        .get(child_id.as_str())
                        .is_some_and(|child| child.runtime_state == SessionRuntimeState::Active)
                })
                .count();

            SessionSpawnTreeNodeProjectionV1 {
                session_id: record.session_id.clone(),
                parent_session_id: record.parent_session_id.clone(),
                role_id: record.role_id.clone(),
                depth: spawn_depth(record.session_id.as_str(), &record_by_id),
                child_count: children.len(),
                active_child_count,
                spawn_mode: record.spawn_mode,
                runtime_state: record.runtime_state,
                cascade_cancel_available: record.cascade_cancel_supported,
                announce_back_badges: record
                    .announce_back_badges
                    .iter()
                    .map(|badge| badge.label.clone())
                    .collect(),
            }
        })
        .collect();

    nodes.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.session_id.cmp(&right.session_id))
    });

    let max_depth = nodes.iter().map(|node| node.depth).max().unwrap_or(0);
    let cascade_cancel_session_ids = nodes
        .iter()
        .filter(|node| node.cascade_cancel_available)
        .map(|node| node.session_id.clone())
        .collect();
    let announce_back_badge_count = tree
        .runtime_records
        .iter()
        .map(|record| record.announce_back_badges.len())
        .sum();

    Ok(SessionSpawnTreeDccProjectionV1 {
        schema_id: "hsk.kernel.session_spawn_tree_dcc_projection@1".to_string(),
        tree_id: tree.tree_id.clone(),
        panel_id: tree.panel_id.clone(),
        visible_fields: tree.visible_fields.clone(),
        nodes,
        max_depth,
        cascade_cancel_session_ids,
        announce_back_badge_count,
        runtime_record_refs: tree
            .runtime_records
            .iter()
            .map(|record| record.runtime_record_ref.clone())
            .collect(),
        mutates_runtime_records: false,
    })
}

fn validate_visible_fields(
    errors: &mut Vec<SessionSpawnTreeDccValidationError>,
    tree: &SessionSpawnTreeDccV1,
) {
    for required_field in REQUIRED_VISIBLE_FIELDS {
        if !tree.visible_fields.contains(&required_field) {
            errors.push(SessionSpawnTreeDccValidationError {
                field: "visible_fields",
                message: "DCC spawn tree must expose hierarchy, child counts, depth, cascade cancel, spawn mode, and announce-back badges",
            });
        }
    }
}

fn validate_authority_refs(
    errors: &mut Vec<SessionSpawnTreeDccValidationError>,
    tree: &SessionSpawnTreeDccV1,
) {
    for required_ref in [
        "kernel.dcc_mvp_runtime_surface",
        "kernel.role_mailbox_inbox_evidence_bridge",
        "kernel.session_anti_pattern_registry",
        "flight_recorder.session_spawn",
    ] {
        if !contains_exact(&tree.product_authority_refs, required_ref) {
            errors.push(SessionSpawnTreeDccValidationError {
                field: "product_authority_refs",
                message: "spawn tree DCC must cite DCC runtime, Role Mailbox, anti-pattern, and session Flight Recorder authorities",
            });
        }
    }
}

fn validate_runtime_records(
    errors: &mut Vec<SessionSpawnTreeDccValidationError>,
    tree: &SessionSpawnTreeDccV1,
) {
    let mut session_ids = HashSet::new();
    for record in &tree.runtime_records {
        if !session_ids.insert(record.session_id.as_str()) {
            errors.push(SessionSpawnTreeDccValidationError {
                field: "runtime_records.session_id",
                message: "spawn runtime session ids must be unique",
            });
        }
    }

    for record in &tree.runtime_records {
        require_non_empty(errors, "runtime_records.session_id", &record.session_id);
        require_non_empty(errors, "runtime_records.role_id", &record.role_id);
        require_non_empty(
            errors,
            "runtime_records.runtime_record_ref",
            &record.runtime_record_ref,
        );
        require_non_empty(
            errors,
            "runtime_records.flight_recorder_ref",
            &record.flight_recorder_ref,
        );

        if !record
            .runtime_record_ref
            .starts_with("runtime://session-spawn/")
        {
            errors.push(SessionSpawnTreeDccValidationError {
                field: "runtime_records.runtime_record_ref",
                message: "spawn tree DCC records must come from runtime session-spawn records",
            });
        }
        if !record
            .flight_recorder_ref
            .starts_with("FR-EVT-SESSION-SPAWN-")
        {
            errors.push(SessionSpawnTreeDccValidationError {
                field: "runtime_records.flight_recorder_ref",
                message: "spawn tree DCC records must cite session-spawn Flight Recorder events",
            });
        }

        if let Some(parent_session_id) = &record.parent_session_id {
            if parent_session_id == &record.session_id {
                errors.push(SessionSpawnTreeDccValidationError {
                    field: "runtime_records.parent_session_id",
                    message: "spawn runtime record cannot parent itself",
                });
            }
            if !session_ids.contains(parent_session_id.as_str()) {
                errors.push(SessionSpawnTreeDccValidationError {
                    field: "runtime_records.parent_session_id",
                    message: "spawn runtime record parent must exist in the tree",
                });
            }
        }

        validate_badges(errors, record);
    }

    if !tree
        .runtime_records
        .iter()
        .any(|record| record.cascade_cancel_supported)
    {
        errors.push(SessionSpawnTreeDccValidationError {
            field: "runtime_records.cascade_cancel_supported",
            message: "spawn tree DCC must expose at least one cascade cancel affordance",
        });
    }

    validate_no_cycles(errors, tree);
}

fn validate_badges(
    errors: &mut Vec<SessionSpawnTreeDccValidationError>,
    record: &SessionSpawnRuntimeRecordV1,
) {
    let mut badge_ids = HashSet::new();
    for badge in &record.announce_back_badges {
        if !badge_ids.insert(badge.badge_id.as_str()) {
            errors.push(SessionSpawnTreeDccValidationError {
                field: "runtime_records.announce_back_badges.badge_id",
                message: "announce-back badge ids must be unique per session",
            });
        }
        require_non_empty(
            errors,
            "runtime_records.announce_back_badges.badge_id",
            &badge.badge_id,
        );
        require_non_empty(
            errors,
            "runtime_records.announce_back_badges.label",
            &badge.label,
        );
        require_non_empty(
            errors,
            "runtime_records.announce_back_badges.mailbox_route",
            &badge.mailbox_route,
        );
        if badge.session_id != record.session_id {
            errors.push(SessionSpawnTreeDccValidationError {
                field: "runtime_records.announce_back_badges.session_id",
                message: "announce-back badges must bind to their runtime session",
            });
        }
        if !badge.mailbox_route.starts_with("role-mailbox://") {
            errors.push(SessionSpawnTreeDccValidationError {
                field: "runtime_records.announce_back_badges.mailbox_route",
                message: "announce-back badges must link Role Mailbox routes",
            });
        }
    }
}

fn validate_no_cycles(
    errors: &mut Vec<SessionSpawnTreeDccValidationError>,
    tree: &SessionSpawnTreeDccV1,
) {
    let parent_by_id: HashMap<&str, Option<&str>> = tree
        .runtime_records
        .iter()
        .map(|record| {
            (
                record.session_id.as_str(),
                record.parent_session_id.as_deref(),
            )
        })
        .collect();

    for record in &tree.runtime_records {
        let mut seen = HashSet::new();
        let mut cursor = Some(record.session_id.as_str());
        while let Some(session_id) = cursor {
            if !seen.insert(session_id) {
                errors.push(SessionSpawnTreeDccValidationError {
                    field: "runtime_records.parent_session_id",
                    message: "spawn tree parent links must not form cycles",
                });
                break;
            }
            cursor = parent_by_id.get(session_id).copied().flatten();
        }
    }
}

fn children_by_parent(tree: &SessionSpawnTreeDccV1) -> HashMap<&str, Vec<String>> {
    let mut children: HashMap<&str, Vec<String>> = HashMap::new();
    for record in &tree.runtime_records {
        if let Some(parent_session_id) = record.parent_session_id.as_deref() {
            children
                .entry(parent_session_id)
                .or_default()
                .push(record.session_id.clone());
        }
    }
    for child_ids in children.values_mut() {
        child_ids.sort();
    }
    children
}

fn spawn_depth(
    session_id: &str,
    record_by_id: &HashMap<&str, &SessionSpawnRuntimeRecordV1>,
) -> usize {
    let mut depth = 0usize;
    let mut cursor = record_by_id
        .get(session_id)
        .and_then(|record| record.parent_session_id.as_deref());
    while let Some(parent_session_id) = cursor {
        depth += 1;
        cursor = record_by_id
            .get(parent_session_id)
            .and_then(|record| record.parent_session_id.as_deref());
    }
    depth
}

fn require_non_empty(
    errors: &mut Vec<SessionSpawnTreeDccValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(SessionSpawnTreeDccValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<SessionSpawnTreeDccValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(SessionSpawnTreeDccValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}
