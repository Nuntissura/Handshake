use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub const FOLDED_MARKDOWN_MIRROR_SYNC_DRIFT_GUARD_STUB_ID: &str =
    "WP-1-Markdown-Mirror-Sync-Drift-Guard-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MirrorSurfaceKind {
    WorkPacket,
    TaskBoard,
    RoleMailbox,
    DccQueue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MirrorAuthorityMode {
    CanonicalRecordAuthority,
    GeneratedProjectionOnly,
    AdvisorySidecarOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MirrorDriftState {
    Synchronized,
    Stale,
    AdvisoryEdit,
    NormalizationRequired,
    ManualResolutionRequired,
    MissingMirror,
    TemplateMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarkdownMirrorDriftSource {
    CanonicalFieldChange,
    AdvisoryHumanEdit,
    MissingMirrorGeneration,
    TemplateMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarkdownMirrorReconciliationActionKind {
    RegenerateMirror,
    CaptureAdvisoryEdit,
    NormalizeAdvisoryEdit,
    QueueManualResolution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownMirrorContractV1 {
    pub contract_id: String,
    pub surface_kind: MirrorSurfaceKind,
    pub canonical_record_ref: String,
    pub mirror_path: String,
    pub mirror_hash_ref: String,
    pub generation_template_ref: String,
    pub authority_mode: MirrorAuthorityMode,
    pub deterministic_regeneration: bool,
    pub append_only_note_sidecar_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownMirrorDriftRecordV1 {
    pub drift_id: String,
    pub contract_id: String,
    pub state: MirrorDriftState,
    pub source: MarkdownMirrorDriftSource,
    pub canonical_hash_ref: String,
    pub mirror_hash_ref: String,
    pub diff_evidence_ref: String,
    pub advisory_ref: Option<String>,
    pub manual_resolution_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownMirrorReconciliationActionV1 {
    pub action_id: String,
    pub contract_id: String,
    pub kind: MarkdownMirrorReconciliationActionKind,
    pub action_catalog_id: String,
    pub write_box_ref: String,
    pub evidence_ref: String,
    pub authority_mutation: bool,
    pub requires_operator_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownMirrorProjectionBannerV1 {
    pub banner_id: String,
    pub contract_id: String,
    pub visible_state: MirrorDriftState,
    pub banner_text: String,
    pub generated_from_canonical: bool,
    pub stale_visible: bool,
    pub advisory_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownMirrorSyncDriftGuardV1 {
    pub schema_id: String,
    pub guard_id: String,
    pub folded_stub_ids: Vec<String>,
    pub mirror_contracts: Vec<MarkdownMirrorContractV1>,
    pub drift_records: Vec<MarkdownMirrorDriftRecordV1>,
    pub reconciliation_actions: Vec<MarkdownMirrorReconciliationActionV1>,
    pub projection_banners: Vec<MarkdownMirrorProjectionBannerV1>,
    pub dcc_queue_ref: String,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownMirrorSyncDriftGuardProjectionV1 {
    pub schema_id: String,
    pub guard_id: String,
    pub synchronized_contract_ids: Vec<String>,
    pub stale_contract_ids: Vec<String>,
    pub advisory_contract_ids: Vec<String>,
    pub manual_resolution_contract_ids: Vec<String>,
    pub dcc_queue_item_ids: Vec<String>,
    pub banner_ids: Vec<String>,
    pub allowed_action_ids: Vec<String>,
    pub mirror_is_authority: bool,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownMirrorSyncDriftGuardValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_markdown_mirror_sync_drift_guard(
    guard: &MarkdownMirrorSyncDriftGuardV1,
) -> Result<(), Vec<MarkdownMirrorSyncDriftGuardValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &guard.schema_id);
    require_non_empty(&mut errors, "guard_id", &guard.guard_id);
    require_vec(&mut errors, "folded_stub_ids", &guard.folded_stub_ids);
    require_vec(&mut errors, "mirror_contracts", &guard.mirror_contracts);
    require_vec(&mut errors, "drift_records", &guard.drift_records);
    require_vec(
        &mut errors,
        "reconciliation_actions",
        &guard.reconciliation_actions,
    );
    require_vec(&mut errors, "projection_banners", &guard.projection_banners);
    require_non_empty(&mut errors, "dcc_queue_ref", &guard.dcc_queue_ref);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &guard.product_authority_refs,
    );
    require_vec(&mut errors, "folded_source_refs", &guard.folded_source_refs);

    if !contains_exact(
        &guard.folded_stub_ids,
        FOLDED_MARKDOWN_MIRROR_SYNC_DRIFT_GUARD_STUB_ID,
    ) {
        errors.push(MarkdownMirrorSyncDriftGuardValidationError {
            field: "folded_stub_ids",
            message: "markdown mirror guard must preserve the folded stub id",
        });
    }
    if !contains_text(
        &guard.folded_source_refs,
        FOLDED_MARKDOWN_MIRROR_SYNC_DRIFT_GUARD_STUB_ID,
    ) {
        errors.push(MarkdownMirrorSyncDriftGuardValidationError {
            field: "folded_source_refs",
            message: "markdown mirror guard must preserve the folded source reference",
        });
    }
    if !guard.dcc_queue_ref.starts_with("dcc://mirror-queue/") {
        errors.push(MarkdownMirrorSyncDriftGuardValidationError {
            field: "dcc_queue_ref",
            message: "markdown mirror guard must expose a typed DCC mirror queue ref",
        });
    }

    validate_authority_refs(&mut errors, guard);
    validate_contracts(&mut errors, guard);
    validate_drift_records(&mut errors, guard);
    validate_reconciliation_actions(&mut errors, guard);
    validate_projection_banners(&mut errors, guard);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_markdown_mirror_sync_drift_guard(
    guard: &MarkdownMirrorSyncDriftGuardV1,
) -> Result<
    MarkdownMirrorSyncDriftGuardProjectionV1,
    Vec<MarkdownMirrorSyncDriftGuardValidationError>,
> {
    validate_markdown_mirror_sync_drift_guard(guard)?;

    Ok(MarkdownMirrorSyncDriftGuardProjectionV1 {
        schema_id: "hsk.kernel.markdown_mirror_sync_drift_guard_projection@1".to_string(),
        guard_id: guard.guard_id.clone(),
        synchronized_contract_ids: synchronized_contract_ids(guard),
        stale_contract_ids: state_contract_ids(guard, &[MirrorDriftState::Stale]),
        advisory_contract_ids: state_contract_ids(
            guard,
            &[
                MirrorDriftState::AdvisoryEdit,
                MirrorDriftState::NormalizationRequired,
            ],
        ),
        manual_resolution_contract_ids: manual_resolution_contract_ids(guard),
        dcc_queue_item_ids: guard
            .drift_records
            .iter()
            .filter(|record| record.state != MirrorDriftState::Synchronized)
            .map(|record| format!("{}:{}", guard.dcc_queue_ref, record.drift_id))
            .collect(),
        banner_ids: guard
            .projection_banners
            .iter()
            .map(|banner| banner.banner_id.clone())
            .collect(),
        allowed_action_ids: allowed_action_ids(guard),
        mirror_is_authority: false,
        mutates_authority: false,
    })
}

fn validate_authority_refs(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    guard: &MarkdownMirrorSyncDriftGuardV1,
) {
    for required_ref in [
        "kernel.mirror_advisory",
        "kernel.action_catalog",
        "dcc.mirror_advisory_queue",
        "locus.sync_controller",
        "projection_banners",
    ] {
        if !contains_exact(&guard.product_authority_refs, required_ref) {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "product_authority_refs",
                message: "markdown mirror guard must cite mirror advisory, catalog, DCC queue, sync controller, and projection banner authorities",
            });
        }
    }
}

fn validate_contracts(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    guard: &MarkdownMirrorSyncDriftGuardV1,
) {
    let mut contract_ids = HashSet::new();
    for contract in &guard.mirror_contracts {
        if !contract_ids.insert(contract.contract_id.as_str()) {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "mirror_contracts.contract_id",
                message: "markdown mirror contract ids must be unique",
            });
        }
        require_non_empty(
            errors,
            "mirror_contracts.contract_id",
            &contract.contract_id,
        );
        require_non_empty(
            errors,
            "mirror_contracts.canonical_record_ref",
            &contract.canonical_record_ref,
        );
        require_non_empty(
            errors,
            "mirror_contracts.mirror_path",
            &contract.mirror_path,
        );
        require_non_empty(
            errors,
            "mirror_contracts.mirror_hash_ref",
            &contract.mirror_hash_ref,
        );
        require_non_empty(
            errors,
            "mirror_contracts.generation_template_ref",
            &contract.generation_template_ref,
        );
        require_non_empty(
            errors,
            "mirror_contracts.append_only_note_sidecar_ref",
            &contract.append_only_note_sidecar_ref,
        );

        if !contract.canonical_record_ref.starts_with("canonical://") {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "mirror_contracts.canonical_record_ref",
                message: "markdown mirrors must project canonical structured records",
            });
        }
        if !contract.mirror_path.ends_with(".md") {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "mirror_contracts.mirror_path",
                message: "markdown mirror paths must be Markdown projections",
            });
        }
        if !contract.mirror_hash_ref.starts_with("sha256://mirror/") {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "mirror_contracts.mirror_hash_ref",
                message: "markdown mirrors must carry deterministic mirror hashes",
            });
        }
        if !contract
            .generation_template_ref
            .starts_with("template://markdown-mirror/")
        {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "mirror_contracts.generation_template_ref",
                message: "markdown mirrors must cite a deterministic generation template",
            });
        }
        if !contract.deterministic_regeneration {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "mirror_contracts.deterministic_regeneration",
                message:
                    "markdown mirrors must regenerate deterministically from canonical records",
            });
        }
        if contract.authority_mode != MirrorAuthorityMode::CanonicalRecordAuthority
            && contract.authority_mode != MirrorAuthorityMode::GeneratedProjectionOnly
        {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "mirror_contracts.authority_mode",
                message: "markdown mirrors and note sidecars must not become authority",
            });
        }
        if !contract
            .append_only_note_sidecar_ref
            .starts_with("notes-sidecar://")
        {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "mirror_contracts.append_only_note_sidecar_ref",
                message: "human notes must be isolated in append-only sidecars",
            });
        }
    }
}

fn validate_drift_records(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    guard: &MarkdownMirrorSyncDriftGuardV1,
) {
    let contract_ids: HashSet<&str> = guard
        .mirror_contracts
        .iter()
        .map(|contract| contract.contract_id.as_str())
        .collect();
    let mut drift_ids = HashSet::new();

    for record in &guard.drift_records {
        if !drift_ids.insert(record.drift_id.as_str()) {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "drift_records.drift_id",
                message: "markdown mirror drift ids must be unique",
            });
        }
        require_non_empty(errors, "drift_records.drift_id", &record.drift_id);
        require_non_empty(errors, "drift_records.contract_id", &record.contract_id);
        require_non_empty(
            errors,
            "drift_records.canonical_hash_ref",
            &record.canonical_hash_ref,
        );
        require_non_empty(
            errors,
            "drift_records.mirror_hash_ref",
            &record.mirror_hash_ref,
        );
        require_non_empty(
            errors,
            "drift_records.diff_evidence_ref",
            &record.diff_evidence_ref,
        );

        if !contract_ids.contains(record.contract_id.as_str()) {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "drift_records.contract_id",
                message: "drift records must reference declared mirror contracts",
            });
        }
        if !record.canonical_hash_ref.starts_with("sha256://canonical/") {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "drift_records.canonical_hash_ref",
                message: "drift records must cite canonical hashes",
            });
        }
        if !record.mirror_hash_ref.starts_with("sha256://mirror/") {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "drift_records.mirror_hash_ref",
                message: "drift records must cite mirror hashes",
            });
        }
        if !record
            .diff_evidence_ref
            .starts_with("evidence://mirror-drift/")
        {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "drift_records.diff_evidence_ref",
                message: "drift records must carry operator-visible drift evidence",
            });
        }
        if advisory_required(record.state) && record.advisory_ref.is_none() {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "drift_records.advisory_ref",
                message: "manual Markdown edits must be surfaced as advisory refs",
            });
        }
        if let Some(advisory_ref) = &record.advisory_ref {
            if !advisory_ref.starts_with("advisory://mirror/") {
                errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                    field: "drift_records.advisory_ref",
                    message: "manual Markdown advisory refs must be typed mirror advisory refs",
                });
            }
        }
        if record.state == MirrorDriftState::ManualResolutionRequired
            && !record.manual_resolution_required
        {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "drift_records.manual_resolution_required",
                message: "manual-resolution drift must block overwrite-safe regeneration",
            });
        }
    }
}

fn validate_reconciliation_actions(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    guard: &MarkdownMirrorSyncDriftGuardV1,
) {
    let contract_ids: HashSet<&str> = guard
        .mirror_contracts
        .iter()
        .map(|contract| contract.contract_id.as_str())
        .collect();
    let mut action_ids = HashSet::new();
    let mut actions_by_contract: HashMap<&str, usize> = HashMap::new();

    for action in &guard.reconciliation_actions {
        if !action_ids.insert(action.action_id.as_str()) {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "reconciliation_actions.action_id",
                message: "markdown mirror reconciliation action ids must be unique",
            });
        }
        require_non_empty(
            errors,
            "reconciliation_actions.action_id",
            &action.action_id,
        );
        require_non_empty(
            errors,
            "reconciliation_actions.contract_id",
            &action.contract_id,
        );
        require_non_empty(
            errors,
            "reconciliation_actions.action_catalog_id",
            &action.action_catalog_id,
        );
        require_non_empty(
            errors,
            "reconciliation_actions.write_box_ref",
            &action.write_box_ref,
        );
        require_non_empty(
            errors,
            "reconciliation_actions.evidence_ref",
            &action.evidence_ref,
        );

        if !contract_ids.contains(action.contract_id.as_str()) {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "reconciliation_actions.contract_id",
                message: "reconciliation actions must reference declared mirror contracts",
            });
        }
        if action.authority_mutation {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "reconciliation_actions.authority_mutation",
                message: "mirror reconciliation must not mutate authority outside promotion",
            });
        }
        if !action.evidence_ref.starts_with("evidence://mirror-drift/") {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "reconciliation_actions.evidence_ref",
                message: "reconciliation actions must carry mirror drift evidence",
            });
        }
        validate_action_route(errors, action);
        *actions_by_contract
            .entry(action.contract_id.as_str())
            .or_insert(0) += 1;
    }

    for record in guard
        .drift_records
        .iter()
        .filter(|record| record.state != MirrorDriftState::Synchronized)
    {
        if !actions_by_contract.contains_key(record.contract_id.as_str()) {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "reconciliation_actions",
                message:
                    "each non-synchronized mirror drift record must have a reconciliation action",
            });
        }
    }
}

fn validate_action_route(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    action: &MarkdownMirrorReconciliationActionV1,
) {
    match action.kind {
        MarkdownMirrorReconciliationActionKind::RegenerateMirror => {
            if action.action_catalog_id != "kernel.markdown_mirror_sync_drift_guard.project" {
                errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                    field: "reconciliation_actions.action_catalog_id",
                    message: "regeneration must route through the markdown mirror guard projection action",
                });
            }
            if !action
                .write_box_ref
                .starts_with("projection-regenerator://markdown-mirror/")
            {
                errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                    field: "reconciliation_actions.write_box_ref",
                    message: "regeneration must use a projection regenerator ref",
                });
            }
        }
        MarkdownMirrorReconciliationActionKind::CaptureAdvisoryEdit => {
            if action.action_catalog_id != "kernel.mirror_advisory.capture" {
                errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                    field: "reconciliation_actions.action_catalog_id",
                    message: "manual mirror edits must route through mirror advisory capture",
                });
            }
            require_mirror_advisory_box_ref(errors, action);
        }
        MarkdownMirrorReconciliationActionKind::NormalizeAdvisoryEdit => {
            if action.action_catalog_id != "kernel.mirror_advisory.normalize" {
                errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                    field: "reconciliation_actions.action_catalog_id",
                    message:
                        "approved advisory edits must route through mirror advisory normalization",
                });
            }
            require_mirror_advisory_box_ref(errors, action);
        }
        MarkdownMirrorReconciliationActionKind::QueueManualResolution => {
            if action.action_catalog_id != "kernel.markdown_mirror_sync_drift_guard.project" {
                errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                    field: "reconciliation_actions.action_catalog_id",
                    message: "manual-resolution queueing must stay projection-only",
                });
            }
        }
    }
}

fn require_mirror_advisory_box_ref(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    action: &MarkdownMirrorReconciliationActionV1,
) {
    if !action
        .write_box_ref
        .starts_with("write-box://mirror-advisory/")
    {
        errors.push(MarkdownMirrorSyncDriftGuardValidationError {
            field: "reconciliation_actions.write_box_ref",
            message: "manual mirror edit handling must use mirror advisory write boxes",
        });
    }
}

fn validate_projection_banners(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    guard: &MarkdownMirrorSyncDriftGuardV1,
) {
    let drift_by_contract: HashMap<&str, MirrorDriftState> = guard
        .drift_records
        .iter()
        .map(|record| (record.contract_id.as_str(), record.state))
        .collect();
    let mut banner_ids = HashSet::new();

    for banner in &guard.projection_banners {
        if !banner_ids.insert(banner.banner_id.as_str()) {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "projection_banners.banner_id",
                message: "projection banner ids must be unique",
            });
        }
        require_non_empty(errors, "projection_banners.banner_id", &banner.banner_id);
        require_non_empty(
            errors,
            "projection_banners.contract_id",
            &banner.contract_id,
        );
        require_non_empty(
            errors,
            "projection_banners.banner_text",
            &banner.banner_text,
        );

        if !banner.generated_from_canonical {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "projection_banners.generated_from_canonical",
                message: "projection banners must be derived from canonical mirror state",
            });
        }
        if let Some(state) = drift_by_contract.get(banner.contract_id.as_str()) {
            if *state != banner.visible_state {
                errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                    field: "projection_banners.visible_state",
                    message: "projection banners must expose the current drift state",
                });
            }
        } else {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "projection_banners.contract_id",
                message: "projection banners must reference a drift-bearing mirror contract",
            });
        }
        if banner.visible_state == MirrorDriftState::Stale && !banner.stale_visible {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "projection_banners.stale_visible",
                message: "stale mirror banners must make staleness visible",
            });
        }
        if advisory_required(banner.visible_state) && !banner.advisory_visible {
            errors.push(MarkdownMirrorSyncDriftGuardValidationError {
                field: "projection_banners.advisory_visible",
                message: "advisory mirror banners must expose pending normalization",
            });
        }
    }
}

fn synchronized_contract_ids(guard: &MarkdownMirrorSyncDriftGuardV1) -> Vec<String> {
    let drift_states: HashMap<&str, MirrorDriftState> = guard
        .drift_records
        .iter()
        .map(|record| (record.contract_id.as_str(), record.state))
        .collect();

    guard
        .mirror_contracts
        .iter()
        .filter(|contract| {
            drift_states
                .get(contract.contract_id.as_str())
                .map(|state| *state == MirrorDriftState::Synchronized)
                .unwrap_or(true)
        })
        .map(|contract| contract.contract_id.clone())
        .collect()
}

fn state_contract_ids(
    guard: &MarkdownMirrorSyncDriftGuardV1,
    states: &[MirrorDriftState],
) -> Vec<String> {
    guard
        .drift_records
        .iter()
        .filter(|record| states.contains(&record.state))
        .map(|record| record.contract_id.clone())
        .collect()
}

fn manual_resolution_contract_ids(guard: &MarkdownMirrorSyncDriftGuardV1) -> Vec<String> {
    guard
        .drift_records
        .iter()
        .filter(|record| {
            record.state == MirrorDriftState::ManualResolutionRequired
                || record.manual_resolution_required
        })
        .map(|record| record.contract_id.clone())
        .collect()
}

fn allowed_action_ids(guard: &MarkdownMirrorSyncDriftGuardV1) -> Vec<String> {
    let mut seen = HashSet::new();
    guard
        .reconciliation_actions
        .iter()
        .filter(|action| seen.insert(action.action_catalog_id.as_str()))
        .map(|action| action.action_catalog_id.clone())
        .collect()
}

fn advisory_required(state: MirrorDriftState) -> bool {
    matches!(
        state,
        MirrorDriftState::AdvisoryEdit
            | MirrorDriftState::NormalizationRequired
            | MirrorDriftState::ManualResolutionRequired
    )
}

fn require_non_empty(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(MarkdownMirrorSyncDriftGuardValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<MarkdownMirrorSyncDriftGuardValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(MarkdownMirrorSyncDriftGuardValidationError {
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
