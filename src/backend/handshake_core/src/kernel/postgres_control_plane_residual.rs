use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const POSTGRES_CONTROL_PLANE_SHIFT_BUNDLE_STUB_ID: &str =
    "WP-1-Postgres-Control-Plane-Shift-Bundle-v1";

pub const FOLDED_POSTGRES_CONTROL_PLANE_STUBS: [&str; 7] = [
    "WP-1-Postgres-Dev-Test-Container-Matrix-v1",
    "WP-1-Postgres-Control-Plane-Leases-Backpressure-v1",
    "WP-1-ModelSession-Postgres-Queue-Workers-v1",
    "WP-1-FEMS-Postgres-Memory-Store-v1",
    "WP-1-Workflow-Engine-Postgres-Durable-Execution-v1",
    "WP-1-DCC-Postgres-Control-Plane-Projections-v1",
    "WP-1-SQLite-Cache-Offline-Boundaries-v1",
];

pub const EXCLUDED_LOOM_STUBS: [&str; 5] = [
    "WP-1-Loom-Storage-Portability-v4",
    "WP-1-Loom-MVP-v1",
    "WP-1-Video-Archive-Loom-Integration-v1",
    "WP-1-Media-Downloader-Loom-Bridge-v1",
    "WP-1-Loom-Preview-VideoPosterFrames-v1",
];

const REQUIRED_RESIDUAL_KINDS: [PostgresResidualObligationKind; 11] = [
    PostgresResidualObligationKind::LiveServiceProof,
    PostgresResidualObligationKind::MigrationSeed,
    PostgresResidualObligationKind::ProofCommandMatrix,
    PostgresResidualObligationKind::QueueLeaseBackpressure,
    PostgresResidualObligationKind::ModelSessionQueue,
    PostgresResidualObligationKind::FemsMemoryStore,
    PostgresResidualObligationKind::WorkflowDurableExecution,
    PostgresResidualObligationKind::DccProjection,
    PostgresResidualObligationKind::SqliteBoundary,
    PostgresResidualObligationKind::CrossSubsystemIntegration,
    PostgresResidualObligationKind::CarryForwardDebt,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PostgresResidualObligationKind {
    LiveServiceProof,
    MigrationSeed,
    ProofCommandMatrix,
    StorageAuthority,
    SqliteBoundary,
    QueueLeaseBackpressure,
    ModelSessionQueue,
    FemsMemoryStore,
    WorkflowDurableExecution,
    DccProjection,
    CrossSubsystemIntegration,
    CarryForwardDebt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PostgresResidualDisposition {
    ImplementedInKernel002,
    MappedToKernel003,
    MappedToKernel004,
    EnvironmentBlocked,
    DebtLedger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageAuthorityMode {
    PostgresRequired,
    SqliteCacheOnly,
    DerivedProjectionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SqliteBoundaryRole {
    Cache,
    OfflineReplica,
    EmbeddedDemo,
    SearchIndex,
    ReadOnlyProjection,
    AuthorityWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostgresResidualObligationV1 {
    pub obligation_id: String,
    pub source_stub_id: String,
    pub kind: PostgresResidualObligationKind,
    pub disposition: PostgresResidualDisposition,
    pub target_kernel_wp_id: Option<String>,
    pub storage_authority_mode: StorageAuthorityMode,
    pub proof_command: Option<String>,
    pub environment_blocker: Option<String>,
    pub sqlite_allowed_roles: Vec<SqliteBoundaryRole>,
    pub acceptance_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

impl PostgresResidualObligationV1 {
    pub fn with_sqlite_roles(mut self, roles: Vec<SqliteBoundaryRole>) -> Self {
        self.sqlite_allowed_roles = roles;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostgresControlPlaneResidualScopeV1 {
    pub schema_id: String,
    pub scope_id: String,
    pub bundle_stub_id: String,
    pub reopens_old_bundle: bool,
    pub obligations: Vec<PostgresResidualObligationV1>,
    pub excluded_stub_ids: Vec<String>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresControlPlaneResidualProjectionV1 {
    pub schema_id: String,
    pub scope_id: String,
    pub bundle_stub_id: String,
    pub reopens_old_bundle: bool,
    pub folded_stub_ids: Vec<String>,
    pub residual_kinds: Vec<PostgresResidualObligationKind>,
    pub kernel002_obligation_ids: Vec<String>,
    pub kernel003_obligation_ids: Vec<String>,
    pub kernel004_obligation_ids: Vec<String>,
    pub environment_blocked_obligation_ids: Vec<String>,
    pub debt_obligation_ids: Vec<String>,
    pub excluded_stub_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresControlPlaneResidualValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_postgres_control_plane_residual_scope(
    scope: &PostgresControlPlaneResidualScopeV1,
) -> Result<(), Vec<PostgresControlPlaneResidualValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &scope.schema_id);
    require_non_empty(&mut errors, "scope_id", &scope.scope_id);
    require_non_empty(&mut errors, "bundle_stub_id", &scope.bundle_stub_id);
    require_vec(&mut errors, "obligations", &scope.obligations);
    require_vec(&mut errors, "excluded_stub_ids", &scope.excluded_stub_ids);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &scope.product_authority_refs,
    );
    require_vec(&mut errors, "folded_source_refs", &scope.folded_source_refs);

    if scope.bundle_stub_id != POSTGRES_CONTROL_PLANE_SHIFT_BUNDLE_STUB_ID {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "bundle_stub_id",
            message: "scope must bind the folded Postgres control-plane shift bundle",
        });
    }

    if scope.reopens_old_bundle {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "reopens_old_bundle",
            message: "residual scope must not reopen the superseded Postgres bundle",
        });
    }

    if !contains_text(
        &scope.folded_source_refs,
        POSTGRES_CONTROL_PLANE_SHIFT_BUNDLE_STUB_ID,
    ) {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "folded_source_refs",
            message: "folded bundle source must be preserved",
        });
    }

    for folded_stub in FOLDED_POSTGRES_CONTROL_PLANE_STUBS {
        if !scope
            .obligations
            .iter()
            .any(|obligation| obligation.source_stub_id == folded_stub)
        {
            errors.push(PostgresControlPlaneResidualValidationError {
                field: "obligations.source_stub_id",
                message: "every transitive folded Postgres stub must be represented",
            });
        }
    }

    for excluded_stub in EXCLUDED_LOOM_STUBS {
        if !contains_exact(&scope.excluded_stub_ids, excluded_stub) {
            errors.push(PostgresControlPlaneResidualValidationError {
                field: "excluded_stub_ids",
                message: "Loom stubs must remain excluded from this residual scope",
            });
        }
    }

    for required_kind in REQUIRED_RESIDUAL_KINDS {
        if !scope
            .obligations
            .iter()
            .any(|obligation| obligation.kind == required_kind)
        {
            errors.push(PostgresControlPlaneResidualValidationError {
                field: "obligations.kind",
                message: "required Postgres residual kind is not represented",
            });
        }
    }

    let mut obligation_ids = HashSet::new();
    for obligation in &scope.obligations {
        if !obligation_ids.insert(obligation.obligation_id.as_str()) {
            errors.push(PostgresControlPlaneResidualValidationError {
                field: "obligation_id",
                message: "obligation ids must be unique",
            });
        }
        validate_obligation(&mut errors, obligation);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_postgres_control_plane_residual_scope(
    scope: &PostgresControlPlaneResidualScopeV1,
) -> Result<
    PostgresControlPlaneResidualProjectionV1,
    Vec<PostgresControlPlaneResidualValidationError>,
> {
    validate_postgres_control_plane_residual_scope(scope)?;

    let mut residual_kinds = Vec::new();
    for obligation in &scope.obligations {
        if !residual_kinds.contains(&obligation.kind) {
            residual_kinds.push(obligation.kind);
        }
    }

    Ok(PostgresControlPlaneResidualProjectionV1 {
        schema_id: "hsk.kernel.postgres_control_plane_residual_projection@1".to_string(),
        scope_id: scope.scope_id.clone(),
        bundle_stub_id: scope.bundle_stub_id.clone(),
        reopens_old_bundle: scope.reopens_old_bundle,
        folded_stub_ids: FOLDED_POSTGRES_CONTROL_PLANE_STUBS
            .iter()
            .map(|stub| (*stub).to_string())
            .collect(),
        residual_kinds,
        kernel002_obligation_ids: obligation_ids_for_target(scope, "WP-KERNEL-002"),
        kernel003_obligation_ids: obligation_ids_for_target(scope, "WP-KERNEL-003"),
        kernel004_obligation_ids: obligation_ids_for_target(scope, "WP-KERNEL-004"),
        environment_blocked_obligation_ids: obligation_ids_for_disposition(
            scope,
            PostgresResidualDisposition::EnvironmentBlocked,
        ),
        debt_obligation_ids: obligation_ids_for_disposition(
            scope,
            PostgresResidualDisposition::DebtLedger,
        ),
        excluded_stub_ids: scope.excluded_stub_ids.clone(),
    })
}

fn validate_obligation(
    errors: &mut Vec<PostgresControlPlaneResidualValidationError>,
    obligation: &PostgresResidualObligationV1,
) {
    require_non_empty(errors, "obligation_id", &obligation.obligation_id);
    require_non_empty(errors, "source_stub_id", &obligation.source_stub_id);
    require_vec(errors, "acceptance_refs", &obligation.acceptance_refs);
    require_vec(errors, "folded_source_refs", &obligation.folded_source_refs);

    if EXCLUDED_LOOM_STUBS
        .iter()
        .any(|stub| obligation.source_stub_id == *stub)
        || obligation.source_stub_id.contains("Loom")
    {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "source_stub_id",
            message: "Loom obligations must stay outside the Postgres residual scope",
        });
    }

    if !contains_text(
        &obligation.folded_source_refs,
        POSTGRES_CONTROL_PLANE_SHIFT_BUNDLE_STUB_ID,
    ) {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "folded_source_refs",
            message: "obligation must cite the folded Postgres bundle source",
        });
    }

    if option_is_empty(obligation.proof_command.as_deref()) {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "proof_command",
            message: "residual obligation must preserve a proof command or proof target",
        });
    }

    validate_disposition_target(errors, obligation);
    validate_storage_authority(errors, obligation);
}

fn validate_disposition_target(
    errors: &mut Vec<PostgresControlPlaneResidualValidationError>,
    obligation: &PostgresResidualObligationV1,
) {
    let target = obligation.target_kernel_wp_id.as_deref();
    match obligation.disposition {
        PostgresResidualDisposition::ImplementedInKernel002 => {
            require_target_prefix(errors, target, "WP-KERNEL-002")
        }
        PostgresResidualDisposition::MappedToKernel003 => {
            require_target_prefix(errors, target, "WP-KERNEL-003")
        }
        PostgresResidualDisposition::MappedToKernel004 => {
            require_target_prefix(errors, target, "WP-KERNEL-004")
        }
        PostgresResidualDisposition::EnvironmentBlocked => {
            require_target_prefix(errors, target, "WP-KERNEL-002");
            if option_is_empty(obligation.environment_blocker.as_deref()) {
                errors.push(PostgresControlPlaneResidualValidationError {
                    field: "environment_blocker",
                    message: "environment-blocked proof must record the deterministic blocker",
                });
            }
        }
        PostgresResidualDisposition::DebtLedger => {
            require_target_prefix(errors, target, "WP-KERNEL-");
        }
    }
}

fn validate_storage_authority(
    errors: &mut Vec<PostgresControlPlaneResidualValidationError>,
    obligation: &PostgresResidualObligationV1,
) {
    if obligation.kind != PostgresResidualObligationKind::SqliteBoundary
        && obligation.storage_authority_mode == StorageAuthorityMode::SqliteCacheOnly
    {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "storage_authority_mode",
            message: "Postgres-required residuals cannot use SQLite as authority",
        });
    }

    if postgres_required_kind(obligation.kind)
        && obligation.storage_authority_mode != StorageAuthorityMode::PostgresRequired
    {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "storage_authority_mode",
            message: "Postgres-required residuals must remain Postgres-required",
        });
    }

    if obligation.kind == PostgresResidualObligationKind::SqliteBoundary {
        if obligation.storage_authority_mode != StorageAuthorityMode::SqliteCacheOnly {
            errors.push(PostgresControlPlaneResidualValidationError {
                field: "storage_authority_mode",
                message: "SQLite boundary obligations must be explicitly cache/offline only",
            });
        }
        if obligation.sqlite_allowed_roles.is_empty() {
            errors.push(PostgresControlPlaneResidualValidationError {
                field: "sqlite_allowed_roles",
                message: "SQLite boundary must enumerate allowed non-authority roles",
            });
        }
        if obligation
            .sqlite_allowed_roles
            .contains(&SqliteBoundaryRole::AuthorityWrite)
        {
            errors.push(PostgresControlPlaneResidualValidationError {
                field: "sqlite_allowed_roles",
                message: "SQLite boundary cannot include authority writes",
            });
        }
    } else if !obligation.sqlite_allowed_roles.is_empty() {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "sqlite_allowed_roles",
            message: "SQLite roles are allowed only on SQLite boundary obligations",
        });
    }
}

fn postgres_required_kind(kind: PostgresResidualObligationKind) -> bool {
    matches!(
        kind,
        PostgresResidualObligationKind::LiveServiceProof
            | PostgresResidualObligationKind::MigrationSeed
            | PostgresResidualObligationKind::StorageAuthority
            | PostgresResidualObligationKind::QueueLeaseBackpressure
            | PostgresResidualObligationKind::ModelSessionQueue
            | PostgresResidualObligationKind::FemsMemoryStore
            | PostgresResidualObligationKind::WorkflowDurableExecution
            | PostgresResidualObligationKind::CrossSubsystemIntegration
    )
}

fn obligation_ids_for_target(
    scope: &PostgresControlPlaneResidualScopeV1,
    target_prefix: &str,
) -> Vec<String> {
    scope
        .obligations
        .iter()
        .filter(|obligation| {
            obligation
                .target_kernel_wp_id
                .as_deref()
                .is_some_and(|target| target.starts_with(target_prefix))
        })
        .map(|obligation| obligation.obligation_id.clone())
        .collect()
}

fn obligation_ids_for_disposition(
    scope: &PostgresControlPlaneResidualScopeV1,
    disposition: PostgresResidualDisposition,
) -> Vec<String> {
    scope
        .obligations
        .iter()
        .filter(|obligation| obligation.disposition == disposition)
        .map(|obligation| obligation.obligation_id.clone())
        .collect()
}

fn require_non_empty(
    errors: &mut Vec<PostgresControlPlaneResidualValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(PostgresControlPlaneResidualValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<PostgresControlPlaneResidualValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(PostgresControlPlaneResidualValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn require_target_prefix(
    errors: &mut Vec<PostgresControlPlaneResidualValidationError>,
    target: Option<&str>,
    expected_prefix: &'static str,
) {
    if target.is_none_or(|target| !target.starts_with(expected_prefix)) {
        errors.push(PostgresControlPlaneResidualValidationError {
            field: "target_kernel_wp_id",
            message: "residual obligation must map to the expected kernel work packet",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}

fn option_is_empty(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}
