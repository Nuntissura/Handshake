//! ModelManual source-row merge and drift guard (WP-KERNEL-005 MT-183,
//! MT-185, MT-186, MT-187).
//!
//! - MT-185/186/187: executable merge of Core/Data, Pose/ComfyUI, and
//!   Diagnostics-owned manual source rows into the Diagnostics manual dataset
//!   by normalized id; any expected source row that is absent is marked as a
//!   blocker instead of being fabricated. Merge runs persist to
//!   `atelier_model_manual_row_merge` and mirror through the EventLedger.
//! - MT-183: executable negative drift checks that cross-check every Wired
//!   manual command against the registered kernel action catalog and the
//!   registered IPC/Tauri route surface, flag orphan manual rows / orphan
//!   feature-group commands / id-normalization collisions, and detect a
//!   wired-surface diff without a `MANUAL_VERSION` bump across persisted runs
//!   (HBR-MAN-001). Guard runs persist to `atelier_model_manual_drift_guard`.
//!
//! This module lives under `src/atelier` (not `src/model_manual`) because the
//! desktop app includes `model_manual/mod.rs` by `#[path]` and must stay free
//! of `crate::atelier` / `crate::kernel` dependencies.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::kernel::action_catalog::{kernel002_action_catalog, KernelActionCatalogV1};
use crate::model_manual::{CommandReference, CommandStatus, Manual};

use super::{AtelierError, AtelierResult, AtelierStore};

pub mod model_manual_merge_event_family {
    pub const MANUAL_ROW_MERGE_RECORDED: &str = "atelier.model_manual.row_merge_recorded";
    pub const MANUAL_DRIFT_GUARD_RECORDED: &str = "atelier.model_manual.drift_guard_recorded";

    pub const ALL: &[&str] = &[MANUAL_ROW_MERGE_RECORDED, MANUAL_DRIFT_GUARD_RECORDED];
}

// ---------------------------------------------------------------------------
// Shared id normalization
// ---------------------------------------------------------------------------

/// Normalize a manual command id for merge keying: trim, lowercase, map
/// `-`/`.`/space separators to `_`, collapse separator runs, and strip
/// leading/trailing separators. Distinct raw ids that collapse onto the same
/// normalized id are a normalization collision and must be surfaced, never
/// silently merged.
pub fn normalize_manual_command_id(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_separator = false;
    for ch in raw.trim().chars() {
        let mapped = match ch {
            'A'..='Z' => ch.to_ascii_lowercase(),
            '-' | '.' | ' ' => '_',
            other => other,
        };
        if mapped == '_' {
            if prev_separator {
                continue;
            }
            prev_separator = true;
        } else {
            prev_separator = false;
        }
        out.push(mapped);
    }
    out.trim_matches('_').to_string()
}

fn command_status_token(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::Wired => "wired",
        CommandStatus::Planned => "planned",
    }
}

/// Index the manual command-reference rows by normalized id. Distinct raw ids
/// colliding on the same normalized id are reported through `on_collision`.
fn build_row_index<'m>(
    manual: &'m Manual,
    mut on_collision: impl FnMut(&'m CommandReference, &'m CommandReference, &str),
) -> BTreeMap<String, &'m CommandReference> {
    let mut index: BTreeMap<String, &'m CommandReference> = BTreeMap::new();
    for row in manual.command_reference {
        let normalized = normalize_manual_command_id(row.id);
        match index.get(normalized.as_str()) {
            Some(existing) if existing.id != row.id => {
                on_collision(existing, row, &normalized);
            }
            Some(_) => {}
            None => {
                index.insert(normalized, row);
            }
        }
    }
    index
}

// ---------------------------------------------------------------------------
// MT-185 / MT-186 / MT-187: manual source-row merge
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManualMergeSourceKind {
    /// MT-185: atelier Core/Data manual source rows (MT-052..MT-060,
    /// MT-073..MT-075 feature groups).
    CoreData,
    /// MT-186: Pose/ComfyUI manual source rows (MT-122..MT-125 feature
    /// groups).
    PoseComfy,
    /// MT-187: Diagnostics-owned manual/action/state/error/bundle rows.
    DiagnosticsOwned,
}

impl ManualMergeSourceKind {
    pub fn as_token(self) -> &'static str {
        match self {
            ManualMergeSourceKind::CoreData => "core_data",
            ManualMergeSourceKind::PoseComfy => "pose_comfy",
            ManualMergeSourceKind::DiagnosticsOwned => "diagnostics_owned",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "core_data" => Ok(ManualMergeSourceKind::CoreData),
            "pose_comfy" => Ok(ManualMergeSourceKind::PoseComfy),
            "diagnostics_owned" => Ok(ManualMergeSourceKind::DiagnosticsOwned),
            other => Err(AtelierError::Validation(format!(
                "unknown manual merge source kind: {other}"
            ))),
        }
    }
}

/// Expected Core/Data manual source feature groups (`MT label`, `group id`).
/// These mirror the `WP-KERNEL-005 atelier (Core-Data) surfaces` block in
/// `src/model_manual/content.rs`; a group or row missing from the manual is a
/// blocker, never fabricated.
const CORE_DATA_SOURCE_GROUPS: &[(&str, &str)] = &[
    ("MT-052", "atelier_character_core"),
    ("MT-053", "atelier_media_intake"),
    ("MT-054", "atelier_collections_contact_sheets"),
    ("MT-055", "atelier_documents_scripts"),
    ("MT-056", "atelier_moodboards"),
    ("MT-057", "atelier_relationships"),
    ("MT-058", "atelier_search_tags_similarity"),
    ("MT-059/MT-073..MT-075", "atelier_exports"),
    ("MT-060", "atelier_reset_recovery"),
];

/// Expected Pose/ComfyUI manual source feature groups (MT-122..MT-125).
const POSE_COMFY_SOURCE_GROUPS: &[(&str, &str)] = &[
    ("MT-122", "atelier_pose_context_and_rig"),
    ("MT-123", "atelier_pose_sidecar_and_identity"),
    ("MT-124", "atelier_comfy_workflow_receipts"),
    ("MT-125", "atelier_pose_comfy_deferred_boundaries"),
];

/// Expected Diagnostics-owned manual rows (`row_kind`, `command id`): the
/// manual structure reads, the action-catalog command map, inspector state
/// reads, the problem-store error surface, and the debug bundle export.
const DIAGNOSTICS_OWNED_SOURCE_ROWS: &[(&str, &str)] = &[
    ("manual", "model_manual_get"),
    ("manual", "model_manual_list_commands"),
    ("manual", "model_manual_search"),
    ("action", "kernel_action_catalog_view"),
    ("state", "kernel_inspector_session_state"),
    ("state", "kernel_inspector_event_ledger_tail"),
    ("error", "diagnostics_problem_store_query"),
    ("bundle", "diagnostics_debug_bundle_export"),
];

pub const MERGE_BLOCKER_MISSING_SOURCE_ROW: &str = "missing_source_row";
pub const MERGE_BLOCKER_MISSING_FEATURE_GROUP: &str = "missing_feature_group";
pub const MERGE_BLOCKER_ID_NORMALIZATION_COLLISION: &str = "id_normalization_collision";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergedManualRow {
    /// Source anchor: the MT label of the source group (Core/Pose merges) or
    /// the `row_kind` (Diagnostics-owned merge).
    pub source_mt: String,
    pub source_group_id: String,
    pub raw_command_id: String,
    pub normalized_command_id: String,
    /// Manual status token of the merged source row (`wired` / `planned`).
    pub status: String,
    /// Diagnostics-owned row classification
    /// (`manual`/`action`/`state`/`error`/`bundle`); `None` for Core/Pose.
    pub row_kind: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualMergeBlocker {
    pub source_mt: String,
    pub source_group_id: String,
    pub expected_id: String,
    pub reason: String,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualRowMergeOutcome {
    pub source_kind: ManualMergeSourceKind,
    pub manual_version: String,
    pub merged_rows: Vec<MergedManualRow>,
    pub blockers: Vec<ManualMergeBlocker>,
}

/// Execute the manual source-row merge for one source kind against a manual
/// dataset. The merge normalizes ids, pulls each expected source row into the
/// merged dataset, and marks every expected-but-absent feature group or
/// command row as a blocker rather than fabricating it (MT-185/186/187).
pub fn merge_manual_source_rows(
    manual: &Manual,
    source_kind: ManualMergeSourceKind,
) -> ManualRowMergeOutcome {
    let mut blockers: Vec<ManualMergeBlocker> = Vec::new();
    let row_index = build_row_index(manual, |existing, row, normalized| {
        blockers.push(ManualMergeBlocker {
            source_mt: "command_reference".to_string(),
            source_group_id: "command_reference".to_string(),
            expected_id: normalized.to_string(),
            reason: MERGE_BLOCKER_ID_NORMALIZATION_COLLISION.to_string(),
            detail: format!(
                "manual rows {} and {} collide on normalized id {normalized}",
                existing.id, row.id
            ),
        });
    });

    let mut merged: BTreeMap<String, MergedManualRow> = BTreeMap::new();
    match source_kind {
        ManualMergeSourceKind::CoreData | ManualMergeSourceKind::PoseComfy => {
            let source_groups = match source_kind {
                ManualMergeSourceKind::CoreData => CORE_DATA_SOURCE_GROUPS,
                _ => POSE_COMFY_SOURCE_GROUPS,
            };
            for (source_mt, group_id) in source_groups {
                let Some(group) = manual
                    .feature_groups
                    .iter()
                    .find(|group| group.id == *group_id)
                else {
                    blockers.push(ManualMergeBlocker {
                        source_mt: (*source_mt).to_string(),
                        source_group_id: (*group_id).to_string(),
                        expected_id: (*group_id).to_string(),
                        reason: MERGE_BLOCKER_MISSING_FEATURE_GROUP.to_string(),
                        detail: format!(
                            "expected {source_mt} source feature group {group_id} is absent from the manual"
                        ),
                    });
                    continue;
                };
                for raw_command_id in group.commands {
                    let normalized = normalize_manual_command_id(raw_command_id);
                    if let Some(previous) = merged.get(normalized.as_str()) {
                        if previous.raw_command_id != *raw_command_id {
                            blockers.push(ManualMergeBlocker {
                                source_mt: (*source_mt).to_string(),
                                source_group_id: (*group_id).to_string(),
                                expected_id: normalized.clone(),
                                reason: MERGE_BLOCKER_ID_NORMALIZATION_COLLISION.to_string(),
                                detail: format!(
                                    "group commands {} and {} collide on normalized id {normalized}",
                                    previous.raw_command_id, raw_command_id
                                ),
                            });
                        }
                        continue;
                    }
                    match row_index.get(normalized.as_str()) {
                        Some(row) => {
                            merged.insert(
                                normalized.clone(),
                                MergedManualRow {
                                    source_mt: (*source_mt).to_string(),
                                    source_group_id: (*group_id).to_string(),
                                    raw_command_id: (*raw_command_id).to_string(),
                                    normalized_command_id: normalized,
                                    status: command_status_token(row.status).to_string(),
                                    row_kind: None,
                                },
                            );
                        }
                        None => blockers.push(ManualMergeBlocker {
                            source_mt: (*source_mt).to_string(),
                            source_group_id: (*group_id).to_string(),
                            expected_id: normalized.clone(),
                            reason: MERGE_BLOCKER_MISSING_SOURCE_ROW.to_string(),
                            detail: format!(
                                "group {group_id} references {raw_command_id} but no manual row resolves to {normalized}"
                            ),
                        }),
                    }
                }
            }
        }
        ManualMergeSourceKind::DiagnosticsOwned => {
            for (row_kind, expected_command_id) in DIAGNOSTICS_OWNED_SOURCE_ROWS {
                let normalized = normalize_manual_command_id(expected_command_id);
                match row_index.get(normalized.as_str()) {
                    Some(row) => {
                        merged.insert(
                            normalized.clone(),
                            MergedManualRow {
                                source_mt: (*row_kind).to_string(),
                                source_group_id: "diagnostics_owned".to_string(),
                                raw_command_id: (*expected_command_id).to_string(),
                                normalized_command_id: normalized,
                                status: command_status_token(row.status).to_string(),
                                row_kind: Some((*row_kind).to_string()),
                            },
                        );
                    }
                    None => blockers.push(ManualMergeBlocker {
                        source_mt: (*row_kind).to_string(),
                        source_group_id: "diagnostics_owned".to_string(),
                        expected_id: normalized.clone(),
                        reason: MERGE_BLOCKER_MISSING_SOURCE_ROW.to_string(),
                        detail: format!(
                            "expected diagnostics-owned {row_kind} row {expected_command_id} has no manual row resolving to {normalized}"
                        ),
                    }),
                }
            }
        }
    }

    ManualRowMergeOutcome {
        source_kind,
        manual_version: manual.version.to_string(),
        merged_rows: merged.into_values().collect(),
        blockers,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualRowMergeRecord {
    pub run_id: Uuid,
    pub source_kind: ManualMergeSourceKind,
    pub manual_version: String,
    pub merged_rows: Vec<MergedManualRow>,
    pub blockers: Vec<ManualMergeBlocker>,
    pub created_at_utc: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// MT-183: manual drift guard
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManualDriftKind {
    /// A Wired manual command declares an IPC channel or Tauri command that is
    /// not a registered kernel action-catalog action and not a registered
    /// IPC/Tauri route.
    WiredUnresolvedSurface,
    /// A feature group references a command id with no manual row.
    MissingManualRow,
    /// A manual row is not referenced by any feature group.
    OrphanManualRow,
    /// Two distinct ids collapse onto the same normalized id.
    IdNormalizationCollision,
    /// The wired surface changed across persisted guard runs without a
    /// `MANUAL_VERSION` bump (HBR-MAN-001).
    ManualVersionNotBumped,
}

impl ManualDriftKind {
    pub fn as_token(self) -> &'static str {
        match self {
            ManualDriftKind::WiredUnresolvedSurface => "wired_unresolved_surface",
            ManualDriftKind::MissingManualRow => "missing_manual_row",
            ManualDriftKind::OrphanManualRow => "orphan_manual_row",
            ManualDriftKind::IdNormalizationCollision => "id_normalization_collision",
            ManualDriftKind::ManualVersionNotBumped => "manual_version_not_bumped",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualDriftFinding {
    pub drift_kind: ManualDriftKind,
    pub command_id: Option<String>,
    pub detail: String,
}

/// The registered surfaces a Wired manual command may resolve to: kernel
/// action-catalog action ids plus registered IPC routes and Tauri commands.
#[derive(Clone, Debug, Default)]
pub struct RegisteredSurfaceIndex {
    action_ids: BTreeSet<String>,
    ipc_routes: BTreeSet<String>,
    tauri_commands: BTreeSet<String>,
}

impl RegisteredSurfaceIndex {
    pub fn from_kernel_catalog(catalog: &KernelActionCatalogV1) -> Self {
        Self {
            action_ids: catalog
                .actions
                .iter()
                .map(|action| action.action_id.to_string())
                .collect(),
            ipc_routes: BTreeSet::new(),
            tauri_commands: BTreeSet::new(),
        }
    }

    pub fn with_ipc_routes<I, S>(mut self, routes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.ipc_routes.extend(routes.into_iter().map(Into::into));
        self
    }

    pub fn with_tauri_commands<I, S>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tauri_commands
            .extend(commands.into_iter().map(Into::into));
        self
    }

    /// The current Handshake runtime surface: the registered KERNEL-002 action
    /// catalog plus the registered desktop IPC bridge channels / HTTP routes /
    /// Tauri commands (`app/src-tauri/src/lib.rs` invoke handlers and the
    /// backend axum routes). A newly wired manual command whose surface is not
    /// registered here is exactly the drift this guard exists to catch: either
    /// the runtime registration or this registry must move in the same commit.
    pub fn handshake_runtime_default() -> Self {
        Self::from_kernel_catalog(&kernel002_action_catalog())
            .with_ipc_routes([
                "kernel.model_manual.get",
                "kernel.model_manual.list_commands",
                "kernel.model_manual.search",
                "kernel.diagnostics.capture",
                "kernel.visual_debug.dom_snapshot",
                "kernel.visual_debug.console_stream.start",
                "kernel.visual_debug.console_stream.stop",
                "kernel.inspector.read",
                "kernel.inspector.port",
                "kernel.inspector.list_sessions",
                "kernel.inspector.session_state",
                "kernel.inspector.event_ledger_tail",
                "kernel.inspector.process_ledger_active",
                "kernel.inspector.trace_projection",
                "kernel.inspector.loaded_models",
                "/inspector/v1/replay-drive",
                "kernel.swarm.run",
                "kernel.sandbox.run",
                "kernel.sandbox.health",
                "kernel.model_runtime.generate",
                "kernel.model_runtime.register_model",
                "kernel.inference_lab.apply_technique",
                "kernel.memory_capsule.build",
                "kernel.memory_calibration.snapshot",
                "kernel.self_improvement.run_iteration",
                "kernel.distillation.review_candidate",
                "/atelier/intake/batches",
                "/atelier/intake/batches/:batch_id/items",
                "/atelier/ai-tag-suggestions",
                "/atelier/image-import/url",
            ])
            .with_tauri_commands([
                "model_manual_get",
                "model_manual_list_commands",
                "model_manual_search",
                "kernel_diagnostics_capture",
                "kernel_visual_debug_dom_snapshot",
                "kernel_visual_debug_console_stream_start",
                "kernel_visual_debug_console_stream_stop",
                "kernel_inspector_read",
                "kernel_inspector_port",
                "kernel_inspector_list_sessions",
                "kernel_inspector_session_state",
                "kernel_inspector_event_ledger_tail",
                "kernel_inspector_process_ledger_active",
                "kernel_inspector_trace_projection",
                "kernel_inspector_loaded_models",
                "kernel_swarm_run",
                "kernel_sandbox_run",
                "kernel_sandbox_health",
                "kernel_model_runtime_generate",
                "kernel_model_runtime_register_model",
                "kernel_inference_lab_apply_technique",
                "kernel_memory_capsule_build",
                "kernel_memory_calibration_snapshot",
                "kernel_self_improvement_run_iteration",
                "kernel_distillation_review_candidate",
            ])
    }

    fn resolves_ipc_channel(&self, channel: &str) -> bool {
        self.action_ids.contains(channel) || self.ipc_routes.contains(channel)
    }

    fn resolves_tauri_command(&self, command: &str) -> bool {
        // An empty Tauri registry means the caller did not supply one (for
        // example a backend-only context); Tauri resolution is then skipped.
        self.tauri_commands.is_empty() || self.tauri_commands.contains(command)
    }
}

/// Run the static manual drift checks (MT-183): wired-surface resolution,
/// orphan feature-group commands (missing manual rows), orphan manual rows,
/// and id-normalization collisions. The cross-run `MANUAL_VERSION` bump check
/// is stateful and lives in [`AtelierStore::record_manual_drift_guard_run`].
pub fn run_manual_drift_guard(
    manual: &Manual,
    surfaces: &RegisteredSurfaceIndex,
) -> Vec<ManualDriftFinding> {
    let mut findings: Vec<ManualDriftFinding> = Vec::new();

    let row_index = build_row_index(manual, |existing, row, normalized| {
        findings.push(ManualDriftFinding {
            drift_kind: ManualDriftKind::IdNormalizationCollision,
            command_id: Some(row.id.to_string()),
            detail: format!(
                "manual rows {} and {} collide on normalized id {normalized}",
                existing.id, row.id
            ),
        });
    });

    let mut grouped_ids: BTreeSet<String> = BTreeSet::new();
    for group in manual.feature_groups {
        for raw_command_id in group.commands {
            let normalized = normalize_manual_command_id(raw_command_id);
            if !row_index.contains_key(normalized.as_str()) {
                findings.push(ManualDriftFinding {
                    drift_kind: ManualDriftKind::MissingManualRow,
                    command_id: Some((*raw_command_id).to_string()),
                    detail: format!(
                        "feature group {} references {raw_command_id} but no manual row resolves to {normalized}",
                        group.id
                    ),
                });
            }
            grouped_ids.insert(normalized);
        }
    }

    for row in manual.command_reference {
        if !grouped_ids.contains(normalize_manual_command_id(row.id).as_str()) {
            findings.push(ManualDriftFinding {
                drift_kind: ManualDriftKind::OrphanManualRow,
                command_id: Some(row.id.to_string()),
                detail: format!(
                    "manual row {} is not referenced by any feature group",
                    row.id
                ),
            });
        }
    }

    for row in manual.command_reference {
        if row.status != CommandStatus::Wired {
            continue;
        }
        if let Some(channel) = row.ipc_channel {
            if !surfaces.resolves_ipc_channel(channel) {
                findings.push(ManualDriftFinding {
                    drift_kind: ManualDriftKind::WiredUnresolvedSurface,
                    command_id: Some(row.id.to_string()),
                    detail: format!(
                        "wired command {} declares ipc_channel {channel} which is neither a registered action-catalog action nor a registered IPC route",
                        row.id
                    ),
                });
            }
        }
        if let Some(command) = row.tauri_command {
            if !surfaces.resolves_tauri_command(command) {
                findings.push(ManualDriftFinding {
                    drift_kind: ManualDriftKind::WiredUnresolvedSurface,
                    command_id: Some(row.id.to_string()),
                    detail: format!(
                        "wired command {} declares tauri_command {command} which is not a registered Tauri command",
                        row.id
                    ),
                });
            }
        }
    }

    findings
}

/// Deterministic fingerprint of the manual's wired surface (id, declared
/// routes, and schema fields of every Wired row). A fingerprint change across
/// persisted guard runs with an unchanged manual version is the HBR-MAN-001
/// drift the guard must flag.
pub fn wired_surface_fingerprint(manual: &Manual) -> String {
    let mut lines: Vec<String> = manual
        .command_reference
        .iter()
        .filter(|row| row.status == CommandStatus::Wired)
        .map(|row| {
            format!(
                "{}|{}|{}|{}",
                row.id,
                row.ipc_channel.unwrap_or("-"),
                row.tauri_command.unwrap_or("-"),
                row.schema_fields.join(",")
            )
        })
        .collect();
    lines.sort();
    format!(
        "sha256:{}",
        hex::encode(Sha256::digest(lines.join("\n").as_bytes()))
    )
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualDriftGuardRecord {
    pub run_id: Uuid,
    pub guard_scope: String,
    pub manual_version: String,
    pub wired_surface_sha256: String,
    pub wired_surface_changed: bool,
    pub findings: Vec<ManualDriftFinding>,
    pub created_at_utc: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

impl AtelierStore {
    /// Persist an executed manual source-row merge run (MT-185/186/187) and
    /// mirror it through the canonical EventLedger family.
    pub async fn record_manual_row_merge(
        &self,
        outcome: &ManualRowMergeOutcome,
    ) -> AtelierResult<ManualRowMergeRecord> {
        let manual_version = outcome.manual_version.trim();
        if manual_version.is_empty() || manual_version != outcome.manual_version {
            return Err(AtelierError::Validation(
                "manual_version must not be empty or padded".into(),
            ));
        }
        if outcome.merged_rows.is_empty() && outcome.blockers.is_empty() {
            return Err(AtelierError::Validation(
                "manual row merge produced neither merged rows nor blockers; refusing to record a no-op run".into(),
            ));
        }

        let run_id = Uuid::now_v7();
        let merged_rows = serde_json::to_value(&outcome.merged_rows)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;
        let blockers = serde_json::to_value(&outcome.blockers)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_model_manual_row_merge (
                   run_id, source_kind, manual_version, merged_row_count,
                   blocker_count, merged_rows, blockers
               )
               VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb)
               RETURNING run_id, source_kind, manual_version, merged_rows,
                         blockers, created_at_utc"#,
        )
        .bind(run_id)
        .bind(outcome.source_kind.as_token())
        .bind(manual_version)
        .bind(outcome.merged_rows.len() as i32)
        .bind(outcome.blockers.len() as i32)
        .bind(merged_rows)
        .bind(blockers)
        .fetch_one(&mut *tx)
        .await?;
        let record = manual_row_merge_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            model_manual_merge_event_family::MANUAL_ROW_MERGE_RECORDED,
            "atelier_model_manual_row_merge",
            &record.run_id.to_string(),
            serde_json::json!({
                "run_id": record.run_id,
                "source_kind": record.source_kind.as_token(),
                "manual_version": record.manual_version,
                "merged_row_count": record.merged_rows.len(),
                "blocker_count": record.blockers.len(),
                "blockers": record.blockers,
                "schema": "hsk.atelier.model_manual_row_merge@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    pub async fn get_manual_row_merge(&self, run_id: Uuid) -> AtelierResult<ManualRowMergeRecord> {
        let row = sqlx::query(
            r#"SELECT run_id, source_kind, manual_version, merged_rows,
                      blockers, created_at_utc
               FROM atelier_model_manual_row_merge
               WHERE run_id = $1"#,
        )
        .bind(run_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => manual_row_merge_from_row(&row),
            None => Err(AtelierError::NotFound(format!(
                "manual row merge run_id={run_id}"
            ))),
        }
    }

    pub async fn latest_manual_row_merge(
        &self,
        source_kind: ManualMergeSourceKind,
    ) -> AtelierResult<Option<ManualRowMergeRecord>> {
        let row = sqlx::query(
            r#"SELECT run_id, source_kind, manual_version, merged_rows,
                      blockers, created_at_utc
               FROM atelier_model_manual_row_merge
               WHERE source_kind = $1
               ORDER BY created_at_utc DESC, run_id DESC
               LIMIT 1"#,
        )
        .bind(source_kind.as_token())
        .fetch_optional(self.pool())
        .await?;
        row.map(|row| manual_row_merge_from_row(&row)).transpose()
    }

    /// Execute and persist a manual drift-guard run (MT-183): the static
    /// checks from [`run_manual_drift_guard`] plus the stateful HBR-MAN-001
    /// check that a wired-surface diff against the previous persisted run in
    /// the same `guard_scope` was accompanied by a `MANUAL_VERSION` bump.
    pub async fn record_manual_drift_guard_run(
        &self,
        guard_scope: &str,
        manual: &Manual,
        surfaces: &RegisteredSurfaceIndex,
    ) -> AtelierResult<ManualDriftGuardRecord> {
        if guard_scope.trim().is_empty() || guard_scope.trim() != guard_scope {
            return Err(AtelierError::Validation(
                "guard_scope must not be empty or padded".into(),
            ));
        }

        let mut findings = run_manual_drift_guard(manual, surfaces);
        let fingerprint = wired_surface_fingerprint(manual);

        let mut tx = self.pool().begin().await?;
        let previous = sqlx::query(
            r#"SELECT manual_version, wired_surface_sha256
               FROM atelier_model_manual_drift_guard
               WHERE guard_scope = $1
               ORDER BY created_at_utc DESC, run_id DESC
               LIMIT 1"#,
        )
        .bind(guard_scope)
        .fetch_optional(&mut *tx)
        .await?;

        let mut wired_surface_changed = false;
        if let Some(previous) = previous {
            let previous_version: String = previous.get("manual_version");
            let previous_fingerprint: String = previous.get("wired_surface_sha256");
            wired_surface_changed = previous_fingerprint != fingerprint;
            if wired_surface_changed && previous_version == manual.version {
                findings.push(ManualDriftFinding {
                    drift_kind: ManualDriftKind::ManualVersionNotBumped,
                    command_id: None,
                    detail: format!(
                        "wired surface diff detected for scope {guard_scope} but MANUAL_VERSION stayed at {}; HBR-MAN-001 requires the bump in the same commit",
                        manual.version
                    ),
                });
            }
        }

        let run_id = Uuid::now_v7();
        let findings_json = serde_json::to_value(&findings)
            .map_err(|err| AtelierError::Validation(err.to_string()))?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_model_manual_drift_guard (
                   run_id, guard_scope, manual_version, wired_surface_sha256,
                   wired_surface_changed, finding_count, findings
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb)
               RETURNING run_id, guard_scope, manual_version,
                         wired_surface_sha256, wired_surface_changed,
                         findings, created_at_utc"#,
        )
        .bind(run_id)
        .bind(guard_scope)
        .bind(manual.version)
        .bind(&fingerprint)
        .bind(wired_surface_changed)
        .bind(findings.len() as i32)
        .bind(findings_json)
        .fetch_one(&mut *tx)
        .await?;
        let record = manual_drift_guard_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            model_manual_merge_event_family::MANUAL_DRIFT_GUARD_RECORDED,
            "atelier_model_manual_drift_guard",
            &record.run_id.to_string(),
            serde_json::json!({
                "run_id": record.run_id,
                "guard_scope": record.guard_scope,
                "manual_version": record.manual_version,
                "wired_surface_sha256": record.wired_surface_sha256,
                "wired_surface_changed": record.wired_surface_changed,
                "finding_count": record.findings.len(),
                "findings": record.findings,
                "schema": "hsk.atelier.model_manual_drift_guard@1",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    pub async fn get_manual_drift_guard_run(
        &self,
        run_id: Uuid,
    ) -> AtelierResult<ManualDriftGuardRecord> {
        let row = sqlx::query(
            r#"SELECT run_id, guard_scope, manual_version, wired_surface_sha256,
                      wired_surface_changed, findings, created_at_utc
               FROM atelier_model_manual_drift_guard
               WHERE run_id = $1"#,
        )
        .bind(run_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(row) => manual_drift_guard_from_row(&row),
            None => Err(AtelierError::NotFound(format!(
                "manual drift guard run_id={run_id}"
            ))),
        }
    }
}

fn manual_row_merge_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ManualRowMergeRecord> {
    let source_kind: String = row.get("source_kind");
    Ok(ManualRowMergeRecord {
        run_id: row.get("run_id"),
        source_kind: ManualMergeSourceKind::from_token(&source_kind)?,
        manual_version: row.get("manual_version"),
        merged_rows: typed_jsonb(row.get("merged_rows"))?,
        blockers: typed_jsonb(row.get("blockers"))?,
        created_at_utc: row.get("created_at_utc"),
    })
}

fn manual_drift_guard_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<ManualDriftGuardRecord> {
    Ok(ManualDriftGuardRecord {
        run_id: row.get("run_id"),
        guard_scope: row.get("guard_scope"),
        manual_version: row.get("manual_version"),
        wired_surface_sha256: row.get("wired_surface_sha256"),
        wired_surface_changed: row.get("wired_surface_changed"),
        findings: typed_jsonb(row.get("findings"))?,
        created_at_utc: row.get("created_at_utc"),
    })
}

fn typed_jsonb<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> AtelierResult<T> {
    serde_json::from_value(value)
        .map_err(|err| AtelierError::Validation(format!("malformed persisted JSON column: {err}")))
}
