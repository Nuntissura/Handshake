//! Fail-closed closure gate for the complete MT-108 native GUI Argus matrix.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::screenshot_harness::screenshot_marker::{self, ScreenshotStatus};

const REQUIRED_SCENARIOS: &[&str] = &[
    "diagnostics_panel",
    "find_bar",
    "formatting_toolbar",
    "slash_menu",
    "outline_pane",
    "rich_find_replace",
    "runtime_chat",
    "editor_host_code",
    "editor_host_rich",
    "canvas_host",
    "graph_host",
    "folders_host",
    "tags_host",
    "sidebar_host",
    "outgoing_links_host",
    "relevant_memory_host",
    "atelier_host",
    "stage_host",
    "calendar_host",
    "block_collections_host",
    "diff_merge_host",
    "loom_search_host",
    "find_in_files_host",
    "wiki_projection_host",
    "manual_host",
    "flight_recorder_host",
    "settings_dialog",
    "fems_proposal_dialog",
    "command_palette",
    "quick_switcher",
    "context_menu_enabled",
    "context_menu_disabled",
    "locus_reference_navigation",
];

#[derive(Debug, serde::Deserialize)]
struct Matrix {
    schema_id: String,
    wp_id: String,
    mt_id: String,
    excluded_non_gui_commands: Vec<ExcludedCommand>,
    rows: Vec<MatrixRow>,
}

#[derive(Debug, serde::Deserialize)]
struct ExcludedCommand {
    command_id: String,
    reason: String,
}

#[derive(Debug, serde::Deserialize)]
struct MatrixRow {
    scenario_id: String,
    surface: String,
    edge_state_tag: String,
    proof_kind: String,
    test_binary: String,
    test_name: String,
    ignored: bool,
    #[serde(default)]
    headless_test_binary: Option<String>,
    #[serde(default)]
    headless_test_name: Option<String>,
    #[serde(default)]
    headless_ignored: bool,
    capture_required: bool,
    #[serde(default)]
    expected_author_ids: Vec<String>,
    #[serde(default)]
    expected_author_id_prefixes: Vec<String>,
    #[serde(default)]
    allowed_methods: Vec<String>,
    route: Option<String>,
    action_author_id: Option<String>,
    action_value: Option<String>,
    action_semantic: Option<String>,
    post_action_target: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct LegacyArgusEvidence {
    schema_id: String,
    run_id: String,
    surface: String,
    process_correlation_id: String,
    process_scenario_id: String,
    process_id: u32,
    receipt_id: u64,
    receipt_status: String,
    terminal_observed_sequence: u64,
    screenshot_outcome_id: String,
    screenshot_status: String,
    screenshot_frame_path: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct MatrixTrace {
    schema_id: String,
    run_id: String,
    scenario_id: String,
    surface: String,
    edge_state_tag: String,
    source_sha: String,
    process_correlation_id: String,
    process_id: u32,
    client_session_id: String,
    method: String,
    target: String,
    action_value: Option<String>,
    target_selected_before: Option<bool>,
    target_selected_after: Option<bool>,
    receipt_id: u64,
    receipt_status: String,
    terminal_observed_sequence: u64,
    agent_id: String,
    before: serde_json::Value,
    after: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
struct ProcessIdentity {
    pid: u32,
    start_time_utc: String,
    executable: String,
}

#[derive(Debug, serde::Deserialize)]
struct ExternalProcessReceipt {
    schema_id: String,
    run_id: String,
    source_sha: String,
    process_correlation_id: String,
    child_pid: u32,
    owned_process_tree: Vec<ProcessIdentity>,
    test_process_pid: Option<u32>,
    test_process_start_time_utc: Option<String>,
    test_process_executable: Option<String>,
    child_started_at_utc: String,
    deadline_at_utc: String,
    deadline_seconds: u64,
    command_executable: String,
    command_arguments: Vec<String>,
    working_directory: String,
    scenario_id: String,
    status: String,
    exit_code: Option<i32>,
    cleanup_verified: bool,
    survivor_count_at_receipt: u64,
}

pub fn verify() -> std::io::Result<()> {
    let matrix: Matrix =
        serde_json::from_str(include_str!("../mt108_argus_matrix.json")).map_err(io_other)?;
    validate_matrix(&matrix)?;

    let run_id = required_env("HANDSHAKE_ARGUS_MATRIX_RUN_ID")?;
    let screenshot_run_id = required_env("HANDSHAKE_SCREENSHOT_RUN_ID")?;
    if run_id != screenshot_run_id {
        return Err(std::io::Error::other(
            "matrix and screenshot run identities differ",
        ));
    }
    let source_sha = required_env("HANDSHAKE_ARGUS_MATRIX_SOURCE_SHA")?;
    validate_current_source(&source_sha)?;
    let run_dir = screenshot_marker::marker_dir();

    let snapshot = std::fs::read_to_string(run_dir.join("mt108_argus_matrix.json"))?;
    if snapshot.as_bytes() != include_str!("../mt108_argus_matrix.json").as_bytes() {
        return Err(std::io::Error::other(
            "executed matrix snapshot differs from the compiled source-controlled manifest",
        ));
    }

    let legacy: Vec<LegacyArgusEvidence> = read_jsonl(&run_dir.join("argus-seven-surface.jsonl"))?;
    let traces: Vec<MatrixTrace> = read_jsonl(&run_dir.join("canonical-argus-matrix.jsonl"))?;
    let screenshots: Vec<screenshot_marker::ScreenshotMarker> =
        read_jsonl(&run_dir.join("screenshot_marker.jsonl"))?;
    let processes: Vec<ExternalProcessReceipt> =
        read_jsonl(&run_dir.join("external_process_receipts.jsonl"))?;

    validate_processes(&matrix, &processes, &run_id, &source_sha)?;
    validate_legacy(&matrix, &legacy, &processes, &run_id)?;
    validate_traces(&matrix, &traces, &processes, &run_id, &source_sha)?;
    validate_screenshots(
        &matrix,
        &screenshots,
        &processes,
        &legacy,
        &traces,
        &run_id,
        &run_dir,
    )?;
    Ok(())
}

fn validate_matrix(matrix: &Matrix) -> std::io::Result<()> {
    if matrix.schema_id != "hsk.native_gui.argus_surface_matrix@1"
        || matrix.wp_id != "WP-KERNEL-012"
        || matrix.mt_id != "MT-108"
    {
        return Err(std::io::Error::other(
            "MT-108 matrix schema or ownership identity drift",
        ));
    }
    let excluded = matrix
        .excluded_non_gui_commands
        .iter()
        .map(|row| row.command_id.as_str())
        .collect::<HashSet<_>>();
    if excluded != HashSet::from(["terminal.open-workspace", "model-session.launch-workspace"])
        || matrix
            .excluded_non_gui_commands
            .iter()
            .any(|row| row.reason.trim().is_empty())
    {
        return Err(std::io::Error::other(
            "MT-108 matrix must explicitly disposition the two non-GUI process-launch commands",
        ));
    }
    let required = REQUIRED_SCENARIOS.iter().copied().collect::<HashSet<_>>();
    let actual = matrix
        .rows
        .iter()
        .map(|row| row.scenario_id.as_str())
        .collect::<HashSet<_>>();
    if matrix.rows.len() != required.len() || actual != required {
        return Err(std::io::Error::other(format!(
            "MT-108 matrix must contain the complete required scenario set; got {actual:?}"
        )));
    }
    for row in &matrix.rows {
        if row.surface.trim().is_empty()
            || row.edge_state_tag.trim().is_empty()
            || row.test_binary.trim().is_empty()
            || row.test_name.trim().is_empty()
            || (row.headless_test_binary.is_some() != row.headless_test_name.is_some())
            || row
                .headless_test_binary
                .as_deref()
                .is_some_and(str::is_empty)
            || row.headless_test_name.as_deref().is_some_and(str::is_empty)
            || !row.capture_required
            || (row.expected_author_ids.is_empty() && row.expected_author_id_prefixes.is_empty())
            || row
                .expected_author_ids
                .iter()
                .any(|author_id| author_id.trim().is_empty())
            || row
                .expected_author_id_prefixes
                .iter()
                .any(|prefix| prefix.trim().is_empty())
            || row
                .allowed_methods
                .iter()
                .any(|method| !matches!(method.as_str(), "argus.click" | "argus.set_value"))
            || (row.action_value.is_some()
                && !row.allowed_methods.is_empty()
                && row.allowed_methods != ["argus.set_value"])
            || !matches!(
                row.proof_kind.as_str(),
                "legacy_surface" | "host_route" | "canonical_driver" | "locus_live"
            )
        {
            return Err(std::io::Error::other(format!(
                "matrix row {:?} has incomplete proof/capture contract",
                row.scenario_id
            )));
        }
    }
    Ok(())
}

fn validate_current_source(expected_sha: &str) -> std::io::Result<()> {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root_output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(crate_root)
        .output()?;
    if !repo_root_output.status.success() {
        return Err(std::io::Error::other(
            "unable to resolve repository root for MT-108 source binding",
        ));
    }
    let repo_root = PathBuf::from(String::from_utf8_lossy(&repo_root_output.stdout).trim());
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(crate_root)
        .output()?;
    if !head.status.success()
        || String::from_utf8_lossy(&head.stdout).trim() != expected_sha
        || expected_sha.len() != 40
    {
        return Err(std::io::Error::other(
            "compiled proof source is not bound to the expected committed HEAD",
        ));
    }
    let status = std::process::Command::new("git")
        .args([
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--",
            ".",
            ":(exclude)AGENTS.md",
            ":(exclude)CLAUDE.md",
        ])
        .current_dir(&repo_root)
        .output()?;
    if !status.status.success() || !status.stdout.is_empty() {
        return Err(std::io::Error::other(format!(
            "MT-108 compiled/configured repository inputs are dirty during proof: {}",
            String::from_utf8_lossy(&status.stdout)
        )));
    }
    Ok(())
}

fn validate_processes(
    matrix: &Matrix,
    rows: &[ExternalProcessReceipt],
    run_id: &str,
    source_sha: &str,
) -> std::io::Result<()> {
    if rows.len() != matrix.rows.len() * 2 + 1 {
        return Err(std::io::Error::other(format!(
            "process proof requires two lifecycle rows per matrix scenario plus the running verifier; got {}",
            rows.len()
        )));
    }
    let by_scenario = rows.iter().fold(
        HashMap::<&str, Vec<&ExternalProcessReceipt>>::new(),
        |mut grouped, row| {
            grouped
                .entry(row.scenario_id.as_str())
                .or_default()
                .push(row);
            grouped
        },
    );
    for row in rows {
        if row.schema_id != "hsk.native_gui.external_process_receipt@3"
            || row.run_id != run_id
            || row.source_sha != source_sha
            || row.process_correlation_id.trim().is_empty()
            || row.child_pid == 0
            || row.owned_process_tree.is_empty()
            || row.child_started_at_utc.trim().is_empty()
            || row.deadline_at_utc.trim().is_empty()
            || row.deadline_seconds == 0
            || row.command_executable != "cargo"
            || !Path::new(&row.working_directory).is_absolute()
        {
            return Err(std::io::Error::other(format!(
                "process receipt {:?} has source/process/command identity drift",
                row.scenario_id
            )));
        }
        if !row.owned_process_tree.iter().any(|identity| {
            identity.pid == row.child_pid
                && identity.start_time_utc == row.child_started_at_utc
                && !identity.executable.trim().is_empty()
        }) {
            return Err(std::io::Error::other(format!(
                "process receipt {:?} does not retain its exact root PID/start identity",
                row.scenario_id
            )));
        }
    }
    for contract in &matrix.rows {
        let lifecycle = by_scenario
            .get(contract.scenario_id.as_str())
            .cloned()
            .unwrap_or_default();
        let started = lifecycle
            .iter()
            .filter(|row| row.status == "STARTED")
            .copied()
            .collect::<Vec<_>>();
        let completed = lifecycle
            .iter()
            .filter(|row| row.status == "COMPLETED")
            .copied()
            .collect::<Vec<_>>();
        let expected = expected_arguments(
            contract,
            !crate::screenshot_harness::screenshot_marker::gpu_screenshot_enabled(),
        );
        if lifecycle.len() != 2
            || started.len() != 1
            || completed.len() != 1
            || started[0].process_correlation_id != completed[0].process_correlation_id
            || started[0].child_pid != completed[0].child_pid
            || started[0].command_arguments != expected
            || completed[0].command_arguments != expected
            || started[0].exit_code.is_some()
            || completed[0].exit_code != Some(0)
            || !completed[0].cleanup_verified
            || completed[0].survivor_count_at_receipt != 0
        {
            return Err(std::io::Error::other(format!(
                "matrix scenario {:?} lacks one exact correlated zero-exit lifecycle",
                contract.scenario_id
            )));
        }
        validate_test_process_identity(completed[0])?;
    }
    let verifier = by_scenario
        .get("manifest_verifier")
        .cloned()
        .unwrap_or_default();
    let expected_verifier = expected_verifier_arguments();
    if verifier.len() != 1
        || verifier[0].status != "STARTED"
        || verifier[0].exit_code.is_some()
        || verifier[0].command_arguments != expected_verifier
    {
        return Err(std::io::Error::other(
            "running manifest verifier lacks its one exact STARTED lifecycle row",
        ));
    }
    Ok(())
}

fn validate_test_process_identity(row: &ExternalProcessReceipt) -> std::io::Result<()> {
    let (Some(pid), Some(start), Some(executable)) = (
        row.test_process_pid,
        row.test_process_start_time_utc.as_deref(),
        row.test_process_executable.as_deref(),
    ) else {
        return Err(std::io::Error::other(format!(
            "completed scenario {:?} lacks test executable PID/start identity",
            row.scenario_id
        )));
    };
    if !row.owned_process_tree.iter().any(|identity| {
        identity.pid == pid && identity.start_time_utc == start && identity.executable == executable
    }) {
        return Err(std::io::Error::other(format!(
            "scenario {:?} test executable is not in the owned process tree",
            row.scenario_id
        )));
    }
    Ok(())
}

fn validate_legacy(
    matrix: &Matrix,
    rows: &[LegacyArgusEvidence],
    processes: &[ExternalProcessReceipt],
    run_id: &str,
) -> std::io::Result<()> {
    let capture_expected = crate::screenshot_harness::screenshot_marker::gpu_screenshot_enabled();
    let contracts = matrix
        .rows
        .iter()
        .filter(|row| row.proof_kind == "legacy_surface")
        .collect::<Vec<_>>();
    if rows.len() != contracts.len() {
        return Err(std::io::Error::other(format!(
            "legacy Argus proof requires {} rows; got {}",
            contracts.len(),
            rows.len()
        )));
    }
    for contract in contracts {
        let evidence = rows
            .iter()
            .find(|row| row.process_scenario_id == contract.scenario_id)
            .ok_or_else(|| {
                std::io::Error::other(format!(
                    "legacy scenario {:?} has no evidence row",
                    contract.scenario_id
                ))
            })?;
        let process = completed_process(processes, &contract.scenario_id)?;
        if evidence.schema_id != "hsk.native_gui.argus_surface_evidence@4"
            || evidence.run_id != run_id
            || evidence.surface.trim().is_empty()
            || evidence.process_correlation_id != process.process_correlation_id
            || Some(evidence.process_id) != process.test_process_pid
            || evidence.receipt_id == 0
            || evidence.terminal_observed_sequence == 0
            || !matches!(
                evidence.receipt_status.as_str(),
                "applied" | "indeterminate"
            )
            || evidence.screenshot_outcome_id.trim().is_empty()
            || (capture_expected
                && (evidence.screenshot_status != "CAPTURED"
                    || evidence.screenshot_frame_path.is_none()))
            || (!capture_expected
                && (evidence.screenshot_status != "DEFERRED"
                    || evidence.screenshot_frame_path.is_some()))
        {
            return Err(std::io::Error::other(format!(
                "legacy scenario {:?} lacks correlated action/receipt/capture proof",
                contract.scenario_id
            )));
        }
    }
    Ok(())
}

fn validate_traces(
    matrix: &Matrix,
    rows: &[MatrixTrace],
    processes: &[ExternalProcessReceipt],
    run_id: &str,
    source_sha: &str,
) -> std::io::Result<()> {
    let contracts = matrix
        .rows
        .iter()
        .filter(|row| row.proof_kind != "legacy_surface")
        .collect::<Vec<_>>();
    let expected = contracts
        .iter()
        .map(|row| row.scenario_id.as_str())
        .collect::<HashSet<_>>();
    let actual = rows
        .iter()
        .map(|row| row.scenario_id.as_str())
        .collect::<HashSet<_>>();
    if actual != expected {
        return Err(std::io::Error::other(format!(
            "canonical trace scenarios differ from the manifest; got {actual:?}"
        )));
    }
    for contract in contracts {
        let process = completed_process(processes, &contract.scenario_id)?;
        let scenario_rows = rows
            .iter()
            .filter(|row| row.scenario_id == contract.scenario_id)
            .collect::<Vec<_>>();
        if scenario_rows.is_empty() {
            return Err(std::io::Error::other(format!(
                "scenario {:?} has no canonical action trace",
                contract.scenario_id
            )));
        }
        let mut contract_state_observed = false;
        for row in scenario_rows {
            let declared_host_semantic = contract.proof_kind != "host_route"
                || (matches!(
                    contract.action_semantic.as_deref(),
                    Some("reactivate_host_tab" | "refresh_preserves_control" | "set_search_query")
                ) && contract.post_action_target.as_deref() == Some("present"));
            let host_action_is_bound = if contract.proof_kind == "host_route" {
                let expected_state_is_observed =
                    if contract.action_semantic.as_deref() == Some("reactivate_host_tab") {
                        contract
                            .expected_author_ids
                            .iter()
                            .all(|author_id| json_has_author_id(&row.after, author_id))
                    } else {
                        contract
                            .expected_author_ids
                            .iter()
                            .all(|author_id| json_has_author_id(&row.before, author_id))
                            && contract
                                .expected_author_ids
                                .iter()
                                .all(|author_id| json_has_author_id(&row.after, author_id))
                    };
                contract
                    .route
                    .as_ref()
                    .is_some_and(|route| !route.trim().is_empty())
                    && (if let Some(action_author_id) = contract.action_author_id.as_deref() {
                        row.target == action_author_id
                    } else {
                        handshake_native::tab_bar::is_tab_author_id(&row.target)
                    })
                    && (contract.action_semantic.as_deref() != Some("reactivate_host_tab")
                        || (row.target_selected_before == Some(false)
                            && row.target_selected_after == Some(true)))
                    && expected_state_is_observed
            } else {
                true
            };
            let row_observes_contract_state = json_observes_expected_author_state(
                &contract.expected_author_ids,
                &contract.expected_author_id_prefixes,
                &row.before,
                &row.after,
            ) && host_action_is_bound;
            if row.schema_id != "hsk.native_gui.canonical_argus_matrix_trace@1"
                || row.run_id != run_id
                || row.surface != contract.surface
                || row.edge_state_tag != contract.edge_state_tag
                || row.source_sha != source_sha
                || row.process_correlation_id != process.process_correlation_id
                || Some(row.process_id) != process.test_process_pid
                || row.client_session_id.trim().is_empty()
                || !action_shape_is_valid(
                    &contract.allowed_methods,
                    contract.action_value.as_deref(),
                    &row.method,
                    row.action_value.as_deref(),
                    &row.after,
                    &row.target,
                )
                || row.target.trim().is_empty()
                || !json_has_author_id(&row.before, &row.target)
                || row.receipt_id == 0
                || row.terminal_observed_sequence == 0
                || !matches!(row.receipt_status.as_str(), "applied" | "indeterminate")
                || row.agent_id.trim().is_empty()
                || row.before.is_null()
                || row.after.is_null()
                || !declared_host_semantic
            {
                return Err(std::io::Error::other(format!(
                    "scenario {:?} has an invalid source/process/action/reinspection trace",
                    contract.scenario_id
                )));
            }
            contract_state_observed |= row_observes_contract_state;
        }
        if !contract_state_observed {
            return Err(std::io::Error::other(format!(
                "scenario {:?} has no trace row proving its declared expected state and action",
                contract.scenario_id
            )));
        }
    }
    Ok(())
}

fn validate_screenshots(
    matrix: &Matrix,
    rows: &[screenshot_marker::ScreenshotMarker],
    processes: &[ExternalProcessReceipt],
    legacy: &[LegacyArgusEvidence],
    traces: &[MatrixTrace],
    run_id: &str,
    run_dir: &Path,
) -> std::io::Result<()> {
    let canonical_run_dir = std::fs::canonicalize(run_dir)?;
    let capture_expected = crate::screenshot_harness::screenshot_marker::gpu_screenshot_enabled();
    let mut outcomes = HashSet::new();
    let mut process_event_sequences = HashSet::new();
    for contract in &matrix.rows {
        let expected_scenario = format!("matrix:{}", contract.scenario_id);
        let matching = rows
            .iter()
            .filter(|row| row.scenario_id == expected_scenario)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(std::io::Error::other(format!(
                "scenario {:?} has no material screenshot marker",
                contract.scenario_id
            )));
        }
        let receipt_terminal_sequence = |marker: &screenshot_marker::ScreenshotMarker| {
            if contract.proof_kind == "legacy_surface" {
                legacy
                    .iter()
                    .find(|evidence| {
                        evidence.process_scenario_id == contract.scenario_id
                            && Some(evidence.receipt_id) == marker.action_receipt_id
                    })
                    .map(|evidence| evidence.terminal_observed_sequence)
            } else {
                traces
                    .iter()
                    .find(|trace| {
                        trace.scenario_id == contract.scenario_id
                            && Some(trace.receipt_id) == marker.action_receipt_id
                    })
                    .map(|trace| trace.terminal_observed_sequence)
            }
        };
        let first_bound_event_sequence = matching
            .iter()
            .filter_map(|marker| {
                receipt_terminal_sequence(marker)
                    .filter(|terminal| marker.proof_event_sequence > *terminal)
                    .map(|_| marker.proof_event_sequence)
            })
            .min()
            .ok_or_else(|| {
                std::io::Error::other(format!(
                    "scenario {:?} has no screenshot bound to a canonical action receipt",
                    contract.scenario_id
                ))
            })?;
        for marker in matching {
            let process = completed_process(processes, &contract.scenario_id)?;
            let receipt_valid = screenshot_receipt_phase_is_valid(
                marker.action_receipt_id,
                receipt_terminal_sequence(marker),
                marker.proof_event_sequence,
                first_bound_event_sequence,
            );
            if marker.schema_id != screenshot_marker::SCREENSHOT_MARKER_SCHEMA_ID
                || marker.run_id != run_id
                || marker.mt_id != "MT-108"
                || marker.source_sha.as_deref() != Some(process.source_sha.as_str())
                || marker.process_correlation_id.as_deref()
                    != Some(process.process_correlation_id.as_str())
                || marker.process_scenario_id.as_deref() != Some(contract.scenario_id.as_str())
                || Some(marker.process_id) != process.test_process_pid
                || marker.proof_event_sequence == 0
                || !process_event_sequences.insert((marker.process_id, marker.proof_event_sequence))
                || !receipt_valid
                || marker.gpu_screenshot_enabled != capture_expected
                || !outcomes.insert(marker.outcome_id.as_str())
            {
                return Err(std::io::Error::other(format!(
                    "scenario {:?} has a deferred, blocked, duplicate, or identity-drifted screenshot",
                    contract.scenario_id
                )));
            }
            if capture_expected
                && (marker.status != ScreenshotStatus::Captured
                    || marker.frame_width.map_or(true, |width| width < 320)
                    || marker.frame_height.map_or(true, |height| height < 180))
            {
                return Err(std::io::Error::other(format!(
                    "scenario {:?} lacks a material captured frame",
                    contract.scenario_id
                )));
            }
            if !capture_expected
                && (marker.status != ScreenshotStatus::Deferred
                    || marker.frame_path.is_some()
                    || marker.frame_width.is_some()
                    || marker.frame_height.is_some())
            {
                return Err(std::io::Error::other(format!(
                    "headless scenario {:?} lacks a typed DEFERRED no-frame marker",
                    contract.scenario_id
                )));
            }
            if !capture_expected {
                continue;
            }
            let frame = PathBuf::from(marker.frame_path.as_deref().ok_or_else(|| {
                std::io::Error::other("CAPTURED screenshot marker has no frame path")
            })?);
            let canonical_frame = std::fs::canonicalize(&frame)?;
            if !canonical_frame.starts_with(&canonical_run_dir) {
                return Err(std::io::Error::other(format!(
                    "captured frame escaped the run directory: {}",
                    canonical_frame.display()
                )));
            }
            let image = image::load_from_memory_with_format(
                &std::fs::read(&canonical_frame)?,
                image::ImageFormat::Png,
            )
            .map_err(io_other)?;
            if Some(image.width()) != marker.frame_width
                || Some(image.height()) != marker.frame_height
                || image.width() < 320
                || image.height() < 180
            {
                return Err(std::io::Error::other(format!(
                    "captured PNG dimensions {}x{} differ from the material marker dimensions {:?}x{:?}",
                    image.width(),
                    image.height(),
                    marker.frame_width,
                    marker.frame_height
                )));
            }
        }
    }
    let known = matrix
        .rows
        .iter()
        .map(|row| format!("matrix:{}", row.scenario_id))
        .collect::<HashSet<_>>();
    if rows.iter().any(|row| !known.contains(&row.scenario_id)) {
        return Err(std::io::Error::other(
            "screenshot marker file contains a scenario outside the exact manifest",
        ));
    }
    Ok(())
}

fn completed_process<'a>(
    rows: &'a [ExternalProcessReceipt],
    scenario: &str,
) -> std::io::Result<&'a ExternalProcessReceipt> {
    rows.iter()
        .find(|row| row.scenario_id == scenario && row.status == "COMPLETED")
        .ok_or_else(|| {
            std::io::Error::other(format!(
                "scenario {scenario:?} has no completed process receipt"
            ))
        })
}

fn expected_arguments(row: &MatrixRow, headless: bool) -> Vec<String> {
    let (test_binary, test_name, ignored) = if headless {
        match (
            row.headless_test_binary.as_ref(),
            row.headless_test_name.as_ref(),
        ) {
            (Some(binary), Some(name)) => (binary, name, row.headless_ignored),
            _ => (&row.test_binary, &row.test_name, row.ignored),
        }
    } else {
        (&row.test_binary, &row.test_name, row.ignored)
    };
    let mut arguments = vec![
        "test".to_owned(),
        "--features".to_owned(),
        "integration,wgpu_screenshots".to_owned(),
        "--no-fail-fast".to_owned(),
        "-j".to_owned(),
        "2".to_owned(),
        "--test".to_owned(),
        test_binary.clone(),
        test_name.clone(),
        "--".to_owned(),
    ];
    if ignored {
        arguments.push("--ignored".to_owned());
    }
    arguments.extend(["--exact".to_owned(), "--nocapture".to_owned()]);
    arguments
}

fn expected_verifier_arguments() -> Vec<String> {
    vec![
        "test".to_owned(),
        "--features".to_owned(),
        "integration,wgpu_screenshots".to_owned(),
        "--no-fail-fast".to_owned(),
        "-j".to_owned(),
        "2".to_owned(),
        "--test".to_owned(),
        "test_mt108_argus_aggregate".to_owned(),
        "mt108_verify_argus_evidence_manifest".to_owned(),
        "--".to_owned(),
        "--ignored".to_owned(),
        "--exact".to_owned(),
        "--nocapture".to_owned(),
    ]
}

fn required_env(name: &str) -> std::io::Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| std::io::Error::other(format!("required verifier env {name} is absent")))
}

fn read_jsonl<T: DeserializeOwned>(path: &Path) -> std::io::Result<Vec<T>> {
    std::fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(io_other))
        .collect()
}

fn io_other(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

fn json_has_author_id(value: &serde_json::Value, expected: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.get("author_id").and_then(serde_json::Value::as_str) == Some(expected)
                || map
                    .values()
                    .any(|child| json_has_author_id(child, expected))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_has_author_id(child, expected)),
        _ => false,
    }
}

fn json_has_author_id_value(value: &serde_json::Value, author_id: &str, expected: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            (map.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id)
                && map.get("value").and_then(serde_json::Value::as_str) == Some(expected))
                || map
                    .values()
                    .any(|child| json_has_author_id_value(child, author_id, expected))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_has_author_id_value(child, author_id, expected)),
        _ => false,
    }
}

fn action_shape_is_valid(
    declared_methods: &[String],
    declared_action_value: Option<&str>,
    observed_method: &str,
    observed_action_value: Option<&str>,
    after: &serde_json::Value,
    target: &str,
) -> bool {
    let method_is_allowed = if declared_methods.is_empty() {
        observed_method
            == if declared_action_value.is_some() {
                "argus.set_value"
            } else {
                "argus.click"
            }
    } else {
        declared_methods
            .iter()
            .any(|method| method == observed_method)
    };
    if !method_is_allowed {
        return false;
    }
    match observed_method {
        "argus.click" => observed_action_value.is_none() && declared_action_value.is_none(),
        "argus.set_value" => observed_action_value.is_some_and(|observed| {
            declared_action_value.is_none_or(|declared| declared == observed)
                && json_has_author_id_value(after, target, observed)
        }),
        _ => false,
    }
}

fn json_has_author_id_prefix(value: &serde_json::Value, expected_prefix: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.get("author_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|author_id| author_id.starts_with(expected_prefix))
                || map
                    .values()
                    .any(|child| json_has_author_id_prefix(child, expected_prefix))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_has_author_id_prefix(child, expected_prefix)),
        _ => false,
    }
}

fn json_observes_expected_author_state(
    expected_author_ids: &[String],
    expected_author_id_prefixes: &[String],
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> bool {
    expected_author_ids.iter().all(|author_id| {
        json_has_author_id(before, author_id) || json_has_author_id(after, author_id)
    }) && expected_author_id_prefixes.iter().all(|prefix| {
        json_has_author_id_prefix(before, prefix) || json_has_author_id_prefix(after, prefix)
    })
}

fn screenshot_receipt_phase_is_valid(
    action_receipt_id: Option<u64>,
    terminal_observed_sequence: Option<u64>,
    marker_event_sequence: u64,
    first_bound_event_sequence: u64,
) -> bool {
    if action_receipt_id.is_some() {
        terminal_observed_sequence.is_some_and(|terminal| marker_event_sequence > terminal)
    } else {
        marker_event_sequence < first_bound_event_sequence
    }
}

#[cfg(test)]
mod tests {
    use super::{
        action_shape_is_valid, expected_arguments, expected_verifier_arguments,
        json_observes_expected_author_state, screenshot_receipt_phase_is_valid, validate_matrix,
        Matrix,
    };

    #[test]
    fn expected_commands_retain_the_mandated_feature_and_concurrency_shape() {
        let matrix: Matrix = serde_json::from_str(include_str!("../mt108_argus_matrix.json"))
            .expect("matrix parses");
        let arguments = expected_arguments(&matrix.rows[0], false);
        assert_eq!(
            &arguments[..6],
            [
                "test",
                "--features",
                "integration,wgpu_screenshots",
                "--no-fail-fast",
                "-j",
                "2",
            ]
        );
        let wiki = matrix
            .rows
            .iter()
            .find(|row| row.scenario_id == "wiki_projection_host")
            .expect("wiki row exists");
        let headless = expected_arguments(wiki, true);
        assert_eq!(headless[7], "test_mt108_argus_matrix");
        assert_eq!(headless[8], "mt108_argus_wiki_projection_headless_route");
        assert!(headless.iter().any(|argument| argument == "--ignored"));
        let gpu = expected_arguments(wiki, false);
        assert_eq!(gpu[7], "test_wiki_page_panel_argus");
        assert_eq!(
            gpu[8],
            "mt025_mounted_wiki_current_source_pg_gpu_argus_edit_cancel_save_readback"
        );
        assert_eq!(
            &expected_verifier_arguments()[..6],
            [
                "test",
                "--features",
                "integration,wgpu_screenshots",
                "--no-fail-fast",
                "-j",
                "2",
            ]
        );
    }

    #[test]
    fn live_generated_matrix_rows_declare_stable_author_id_prefixes() {
        let matrix: Matrix = serde_json::from_str(include_str!("../mt108_argus_matrix.json"))
            .expect("matrix parses");
        validate_matrix(&matrix).expect("matrix contract is valid");
        for (scenario, expected_prefixes) in [
            (
                "folders_host",
                &["folder-tree.node.lfd-", "folder-tree.color.lfd-"][..],
            ),
            (
                "sidebar_host",
                &[
                    "sidebar.pin.",
                    "sidebar.favorite.",
                    "sidebar.backlink.",
                    "sidebar.unlinked.",
                ][..],
            ),
        ] {
            let row = matrix
                .rows
                .iter()
                .find(|row| row.scenario_id == scenario)
                .expect("live-generated scenario exists");
            assert!(row.expected_author_ids.is_empty());
            assert_eq!(
                row.expected_author_id_prefixes,
                expected_prefixes
                    .iter()
                    .map(|prefix| (*prefix).to_owned())
                    .collect::<Vec<_>>()
            );
        }
        let wiki = matrix
            .rows
            .iter()
            .find(|row| row.scenario_id == "wiki_projection_host")
            .expect("wiki scenario exists");
        assert_eq!(wiki.allowed_methods, ["argus.click", "argus.set_value"]);
    }

    #[test]
    fn multi_action_contract_validates_each_observed_action_shape() {
        let allowed = vec!["argus.click".to_owned(), "argus.set_value".to_owned()];
        let after = serde_json::json!({
            "author_id": "wiki.edit-area.dynamic",
            "value": "persisted draft"
        });
        assert!(action_shape_is_valid(
            &allowed,
            None,
            "argus.click",
            None,
            &after,
            "wiki.edit.dynamic",
        ));
        assert!(action_shape_is_valid(
            &allowed,
            None,
            "argus.set_value",
            Some("persisted draft"),
            &after,
            "wiki.edit-area.dynamic",
        ));
        assert!(!action_shape_is_valid(
            &allowed,
            None,
            "argus.set_value",
            Some("different draft"),
            &after,
            "wiki.edit-area.dynamic",
        ));
        assert!(!action_shape_is_valid(
            &[],
            None,
            "argus.set_value",
            Some("persisted draft"),
            &after,
            "wiki.edit-area.dynamic",
        ));
    }

    #[test]
    fn multi_action_scenario_accepts_navigation_rows_before_one_contract_state_row() {
        let expected_ids = vec![
            "fems-propose-dialog".to_owned(),
            "fems-propose-confirm".to_owned(),
        ];
        let navigation = serde_json::json!({"author_id": "menu-edit"});
        let contract_state = serde_json::json!({
            "children": [
                {"author_id": "fems-propose-dialog"},
                {"author_id": "fems-propose-confirm"}
            ]
        });

        assert!(!json_observes_expected_author_state(
            &expected_ids,
            &[],
            &navigation,
            &navigation,
        ));
        assert!(json_observes_expected_author_state(
            &expected_ids,
            &[],
            &navigation,
            &contract_state,
        ));
    }

    #[test]
    fn screenshot_phase_requires_a_bound_action_or_an_earlier_pre_action_marker() {
        assert!(screenshot_receipt_phase_is_valid(None, None, 10, 20));
        assert!(!screenshot_receipt_phase_is_valid(None, None, 20, 20));
        assert!(!screenshot_receipt_phase_is_valid(Some(7), None, 30, 20));
        assert!(!screenshot_receipt_phase_is_valid(
            Some(7),
            Some(25),
            24,
            30
        ));
        assert!(screenshot_receipt_phase_is_valid(Some(7), Some(25), 26, 26));
    }
}
