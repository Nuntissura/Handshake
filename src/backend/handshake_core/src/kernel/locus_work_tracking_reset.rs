use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub const FOLDED_LOCUS_WORK_TRACKING_STUB_ID: &str = "WP-1-Locus-Work-Tracking-System-Phase1-v1";

const REQUIRED_CAPABILITY_KINDS: [LocusTrackingCapabilityKind; 7] = [
    LocusTrackingCapabilityKind::WorkPacketTracking,
    LocusTrackingCapabilityKind::MicroTaskTracking,
    LocusTrackingCapabilityKind::DependencyGraph,
    LocusTrackingCapabilityKind::Occupancy,
    LocusTrackingCapabilityKind::ReadyQuery,
    LocusTrackingCapabilityKind::TaskBoardProjection,
    LocusTrackingCapabilityKind::FlightRecorder,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusTrackingCapabilityKind {
    WorkPacketTracking,
    MicroTaskTracking,
    DependencyGraph,
    Occupancy,
    ReadyQuery,
    TaskBoardProjection,
    FlightRecorder,
    SpecRouterIntegration,
    MtExecutorIntegration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusAuthorityMode {
    PostgresAuthority,
    EventLedgerAuthority,
    CrdtWorkspaceAuthority,
    ProjectionOnly,
    LegacySqliteAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusMicroTaskStatus {
    Pending,
    Ready,
    Claimed,
    Running,
    Blocked,
    GateReview,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocusFlightRecorderEventKind {
    WorkPacketCreated,
    WorkPacketUpdated,
    WorkPacketGated,
    WorkPacketClosed,
    MicroTaskRegistered,
    MicroTaskStarted,
    MicroTaskIterationRecorded,
    MicroTaskCompleted,
    DependencyAdded,
    DependencyRemoved,
    OccupancyBound,
    OccupancyUnbound,
    TaskBoardSynced,
    ReadyQueryExecuted,
    FlightRecorderAppended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusTrackingCapabilityV1 {
    pub capability_id: String,
    pub kind: LocusTrackingCapabilityKind,
    pub authority_mode: LocusAuthorityMode,
    pub operation_ids: Vec<String>,
    pub flight_recorder_event_types: Vec<String>,
    pub authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusWorkPacketRecordV1 {
    pub wp_id: String,
    pub title: String,
    pub status: String,
    pub micro_task_ids: Vec<String>,
    pub authority_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusMicroTaskRecordV1 {
    pub mt_id: String,
    pub wp_id: String,
    pub status: LocusMicroTaskStatus,
    pub depends_on: Vec<String>,
    pub active_session_ids: Vec<String>,
    pub iteration_ids: Vec<String>,
    pub authority_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusWorkTrackingResetContractV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub folded_stub_id: String,
    pub sqlite_authority_allowed: bool,
    pub capabilities: Vec<LocusTrackingCapabilityV1>,
    pub work_packets: Vec<LocusWorkPacketRecordV1>,
    pub micro_tasks: Vec<LocusMicroTaskRecordV1>,
    pub task_board_projection_id: String,
    pub flight_recorder_event_types: Vec<String>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocusTaskBoardRowV1 {
    pub wp_id: String,
    pub mt_id: String,
    pub status: LocusMicroTaskStatus,
    pub blocked_by: Vec<String>,
    pub active_session_ids: Vec<String>,
    pub authority_projection: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocusWorkTrackingResetValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_locus_work_tracking_reset_contract(
    contract: &LocusWorkTrackingResetContractV1,
) -> Result<(), Vec<LocusWorkTrackingResetValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &contract.schema_id);
    require_non_empty(&mut errors, "contract_id", &contract.contract_id);
    require_non_empty(&mut errors, "folded_stub_id", &contract.folded_stub_id);
    require_non_empty(
        &mut errors,
        "task_board_projection_id",
        &contract.task_board_projection_id,
    );
    require_vec(&mut errors, "capabilities", &contract.capabilities);
    require_vec(&mut errors, "work_packets", &contract.work_packets);
    require_vec(&mut errors, "micro_tasks", &contract.micro_tasks);
    require_vec(
        &mut errors,
        "flight_recorder_event_types",
        &contract.flight_recorder_event_types,
    );
    require_vec(
        &mut errors,
        "product_authority_refs",
        &contract.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &contract.folded_source_refs,
    );

    if contract.folded_stub_id != FOLDED_LOCUS_WORK_TRACKING_STUB_ID {
        errors.push(LocusWorkTrackingResetValidationError {
            field: "folded_stub_id",
            message: "contract must bind the folded Locus work tracking stub",
        });
    }

    if contract.sqlite_authority_allowed {
        errors.push(LocusWorkTrackingResetValidationError {
            field: "sqlite_authority_allowed",
            message: "legacy SQLite authority must be replaced by product authority",
        });
    }

    if !contains_text(
        &contract.folded_source_refs,
        FOLDED_LOCUS_WORK_TRACKING_STUB_ID,
    ) {
        errors.push(LocusWorkTrackingResetValidationError {
            field: "folded_source_refs",
            message: "folded Locus stub source must be preserved",
        });
    }

    for required_ref in [
        "kernel.postgres_control_plane",
        "kernel.event_ledger",
        "kernel.crdt_workspace",
    ] {
        if !contains_exact(&contract.product_authority_refs, required_ref) {
            errors.push(LocusWorkTrackingResetValidationError {
                field: "product_authority_refs",
                message: "Postgres, EventLedger, and CRDT authority refs are required",
            });
        }
    }

    validate_capabilities(&mut errors, &contract.capabilities);
    validate_work_records(&mut errors, contract);

    for event_type in &contract.flight_recorder_event_types {
        if let Err(error) = validate_locus_flight_recorder_event_type(event_type) {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn query_locus_ready_micro_tasks(
    contract: &LocusWorkTrackingResetContractV1,
    wp_id: &str,
) -> Result<Vec<LocusMicroTaskRecordV1>, Vec<LocusWorkTrackingResetValidationError>> {
    validate_locus_work_tracking_reset_contract(contract)?;

    let completed: HashSet<&str> = contract
        .micro_tasks
        .iter()
        .filter(|micro_task| micro_task.wp_id == wp_id)
        .filter(|micro_task| micro_task.status == LocusMicroTaskStatus::Complete)
        .map(|micro_task| micro_task.mt_id.as_str())
        .collect();

    Ok(contract
        .micro_tasks
        .iter()
        .filter(|micro_task| micro_task.wp_id == wp_id)
        .filter(|micro_task| {
            matches!(
                micro_task.status,
                LocusMicroTaskStatus::Pending | LocusMicroTaskStatus::Ready
            )
        })
        .filter(|micro_task| micro_task.active_session_ids.is_empty())
        .filter(|micro_task| {
            micro_task
                .depends_on
                .iter()
                .all(|dependency| completed.contains(dependency.as_str()))
        })
        .cloned()
        .collect())
}

pub fn project_locus_task_board_rows(
    contract: &LocusWorkTrackingResetContractV1,
    wp_id: &str,
) -> Result<Vec<LocusTaskBoardRowV1>, Vec<LocusWorkTrackingResetValidationError>> {
    validate_locus_work_tracking_reset_contract(contract)?;

    let completed: HashSet<&str> = contract
        .micro_tasks
        .iter()
        .filter(|micro_task| micro_task.wp_id == wp_id)
        .filter(|micro_task| micro_task.status == LocusMicroTaskStatus::Complete)
        .map(|micro_task| micro_task.mt_id.as_str())
        .collect();

    Ok(contract
        .micro_tasks
        .iter()
        .filter(|micro_task| micro_task.wp_id == wp_id)
        .map(|micro_task| LocusTaskBoardRowV1 {
            wp_id: micro_task.wp_id.clone(),
            mt_id: micro_task.mt_id.clone(),
            status: micro_task.status,
            blocked_by: micro_task
                .depends_on
                .iter()
                .filter(|dependency| !completed.contains(dependency.as_str()))
                .cloned()
                .collect(),
            active_session_ids: micro_task.active_session_ids.clone(),
            authority_projection: "projection-only".to_string(),
        })
        .collect())
}

pub fn validate_locus_flight_recorder_event_type(
    event_type: &str,
) -> Result<LocusFlightRecorderEventKind, LocusWorkTrackingResetValidationError> {
    match event_type {
        "locus.wp.created" => Ok(LocusFlightRecorderEventKind::WorkPacketCreated),
        "locus.wp.updated" => Ok(LocusFlightRecorderEventKind::WorkPacketUpdated),
        "locus.wp.gated" => Ok(LocusFlightRecorderEventKind::WorkPacketGated),
        "locus.wp.closed" => Ok(LocusFlightRecorderEventKind::WorkPacketClosed),
        "locus.mt.registered" => Ok(LocusFlightRecorderEventKind::MicroTaskRegistered),
        "locus.mt.started" => Ok(LocusFlightRecorderEventKind::MicroTaskStarted),
        "locus.mt.iteration_recorded" => {
            Ok(LocusFlightRecorderEventKind::MicroTaskIterationRecorded)
        }
        "locus.mt.completed" => Ok(LocusFlightRecorderEventKind::MicroTaskCompleted),
        "locus.dep.added" => Ok(LocusFlightRecorderEventKind::DependencyAdded),
        "locus.dep.removed" => Ok(LocusFlightRecorderEventKind::DependencyRemoved),
        "locus.occupancy.bound" => Ok(LocusFlightRecorderEventKind::OccupancyBound),
        "locus.occupancy.unbound" => Ok(LocusFlightRecorderEventKind::OccupancyUnbound),
        "locus.task_board.synced" => Ok(LocusFlightRecorderEventKind::TaskBoardSynced),
        "locus.query.ready" => Ok(LocusFlightRecorderEventKind::ReadyQueryExecuted),
        "locus.flight_recorder.appended" => {
            Ok(LocusFlightRecorderEventKind::FlightRecorderAppended)
        }
        _ => Err(LocusWorkTrackingResetValidationError {
            field: "flight_recorder_event_type",
            message: "unknown Locus Flight Recorder event type",
        }),
    }
}

fn validate_capabilities(
    errors: &mut Vec<LocusWorkTrackingResetValidationError>,
    capabilities: &[LocusTrackingCapabilityV1],
) {
    for required_kind in REQUIRED_CAPABILITY_KINDS {
        if !capabilities
            .iter()
            .any(|capability| capability.kind == required_kind)
        {
            errors.push(LocusWorkTrackingResetValidationError {
                field: "capabilities.kind",
                message: "required Locus tracking capability is missing",
            });
        }
    }

    let mut capability_ids = HashSet::new();
    for capability in capabilities {
        if !capability_ids.insert(capability.capability_id.as_str()) {
            errors.push(LocusWorkTrackingResetValidationError {
                field: "capability_id",
                message: "capability ids must be unique",
            });
        }

        require_non_empty(errors, "capability_id", &capability.capability_id);
        require_vec(errors, "operation_ids", &capability.operation_ids);
        require_vec(
            errors,
            "flight_recorder_event_types",
            &capability.flight_recorder_event_types,
        );
        require_vec(errors, "authority_refs", &capability.authority_refs);
        require_vec(errors, "folded_source_refs", &capability.folded_source_refs);

        if capability.authority_mode == LocusAuthorityMode::LegacySqliteAuthority {
            errors.push(LocusWorkTrackingResetValidationError {
                field: "authority_mode",
                message: "SQLite authority is not allowed after the Locus reset migration",
            });
        }

        if !authority_mode_matches_kind(capability.kind, capability.authority_mode) {
            errors.push(LocusWorkTrackingResetValidationError {
                field: "authority_mode",
                message: "capability authority mode must be Postgres/EventLedger/CRDT compatible",
            });
        }

        if !contains_text(
            &capability.folded_source_refs,
            FOLDED_LOCUS_WORK_TRACKING_STUB_ID,
        ) {
            errors.push(LocusWorkTrackingResetValidationError {
                field: "folded_source_refs",
                message: "capability must cite the folded Locus source",
            });
        }

        for event_type in &capability.flight_recorder_event_types {
            if let Err(error) = validate_locus_flight_recorder_event_type(event_type) {
                errors.push(error);
            }
        }
    }
}

fn validate_work_records(
    errors: &mut Vec<LocusWorkTrackingResetValidationError>,
    contract: &LocusWorkTrackingResetContractV1,
) {
    let micro_task_by_id: HashMap<&str, &LocusMicroTaskRecordV1> = contract
        .micro_tasks
        .iter()
        .map(|micro_task| (micro_task.mt_id.as_str(), micro_task))
        .collect();

    for work_packet in &contract.work_packets {
        require_non_empty(errors, "work_packets.wp_id", &work_packet.wp_id);
        require_non_empty(errors, "work_packets.title", &work_packet.title);
        require_non_empty(errors, "work_packets.status", &work_packet.status);
        require_non_empty(
            errors,
            "work_packets.authority_ref",
            &work_packet.authority_ref,
        );
        require_vec(
            errors,
            "work_packets.micro_task_ids",
            &work_packet.micro_task_ids,
        );

        for mt_id in &work_packet.micro_task_ids {
            if !micro_task_by_id.contains_key(mt_id.as_str()) {
                errors.push(LocusWorkTrackingResetValidationError {
                    field: "work_packets.micro_task_ids",
                    message: "work packet references an unknown microtask",
                });
            }
        }
    }

    for micro_task in &contract.micro_tasks {
        require_non_empty(errors, "micro_tasks.mt_id", &micro_task.mt_id);
        require_non_empty(errors, "micro_tasks.wp_id", &micro_task.wp_id);
        require_non_empty(
            errors,
            "micro_tasks.authority_ref",
            &micro_task.authority_ref,
        );

        for dependency_id in &micro_task.depends_on {
            match micro_task_by_id.get(dependency_id.as_str()) {
                Some(dependency) if dependency.wp_id == micro_task.wp_id => {}
                _ => errors.push(LocusWorkTrackingResetValidationError {
                    field: "micro_tasks.depends_on",
                    message: "dependency must reference a microtask in the same work packet",
                }),
            }
        }

        if matches!(
            micro_task.status,
            LocusMicroTaskStatus::Running | LocusMicroTaskStatus::Claimed
        ) && micro_task.active_session_ids.is_empty()
        {
            errors.push(LocusWorkTrackingResetValidationError {
                field: "active_session_ids",
                message: "claimed or running microtasks must expose occupancy",
            });
        }

        if micro_task.status == LocusMicroTaskStatus::Complete
            && micro_task.iteration_ids.is_empty()
        {
            errors.push(LocusWorkTrackingResetValidationError {
                field: "iteration_ids",
                message: "completed microtasks must preserve MT Executor iteration records",
            });
        }
    }

    if has_dependency_cycle(&contract.micro_tasks) {
        errors.push(LocusWorkTrackingResetValidationError {
            field: "micro_tasks.depends_on",
            message: "dependency graph must remain acyclic",
        });
    }
}

fn authority_mode_matches_kind(
    kind: LocusTrackingCapabilityKind,
    authority_mode: LocusAuthorityMode,
) -> bool {
    match kind {
        LocusTrackingCapabilityKind::WorkPacketTracking
        | LocusTrackingCapabilityKind::ReadyQuery
        | LocusTrackingCapabilityKind::SpecRouterIntegration => {
            authority_mode == LocusAuthorityMode::PostgresAuthority
        }
        LocusTrackingCapabilityKind::MicroTaskTracking
        | LocusTrackingCapabilityKind::Occupancy
        | LocusTrackingCapabilityKind::MtExecutorIntegration => {
            authority_mode == LocusAuthorityMode::CrdtWorkspaceAuthority
        }
        LocusTrackingCapabilityKind::DependencyGraph
        | LocusTrackingCapabilityKind::FlightRecorder => {
            authority_mode == LocusAuthorityMode::EventLedgerAuthority
        }
        LocusTrackingCapabilityKind::TaskBoardProjection => {
            authority_mode == LocusAuthorityMode::ProjectionOnly
        }
    }
}

fn has_dependency_cycle(micro_tasks: &[LocusMicroTaskRecordV1]) -> bool {
    let graph: HashMap<&str, Vec<&str>> = micro_tasks
        .iter()
        .map(|micro_task| {
            (
                micro_task.mt_id.as_str(),
                micro_task
                    .depends_on
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    graph
        .keys()
        .any(|node| visit_dependency_node(*node, &graph, &mut visiting, &mut visited))
}

fn visit_dependency_node<'a>(
    node: &'a str,
    graph: &HashMap<&'a str, Vec<&'a str>>,
    visiting: &mut HashSet<&'a str>,
    visited: &mut HashSet<&'a str>,
) -> bool {
    if visited.contains(node) {
        return false;
    }
    if !visiting.insert(node) {
        return true;
    }

    if let Some(dependencies) = graph.get(node) {
        for dependency in dependencies {
            if graph.contains_key(dependency)
                && visit_dependency_node(dependency, graph, visiting, visited)
            {
                return true;
            }
        }
    }

    visiting.remove(node);
    visited.insert(node);
    false
}

fn require_non_empty(
    errors: &mut Vec<LocusWorkTrackingResetValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(LocusWorkTrackingResetValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<LocusWorkTrackingResetValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(LocusWorkTrackingResetValidationError {
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
