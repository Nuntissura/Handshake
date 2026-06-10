//! WP-KERNEL-005 MT-133: Diagnostics Product Anchor Verification.
//!
//! Kernel-diagnostics counterpart of the pose/media anchor verification work
//! (`atelier::source_evidence::pose_media_anchor_verification_matrix`, MT-082).
//! Where MT-082 verified pose/media anchors, MT-133 verifies the Handshake
//! *kernel diagnostics* product anchors the contract names:
//!
//!   sessions, command catalog, Workflow Engine, DCC, Locus, Flight Recorder,
//!   visual capture, build diagnostics.
//!
//! Two entry points:
//!   * [`kernel_diagnostics_anchor_verification_matrix`] -- the declared anchor
//!     matrix (what product paths each surface is expected to be backed by).
//!   * [`verify_kernel_diagnostics_anchor_matrix`] -- the *verified* matrix: it
//!     re-checks every declared path against the real product source tree and
//!     downgrades any anchor whose paths are missing to
//!     `BLOCKED_MISSING_ANCHOR` (with the missing paths in `blocking_reason`)
//!     and its source row to `BLOCKED` with a `gap_reason`. Nothing is assumed:
//!     a surface only stays `VERIFIED` when its anchors exist on disk.
//!
//! Persistence reuses the canonical source-evidence store
//! (`AtelierStore::record_source_evidence_matrix`), so recording this matrix
//! lands in live PostgreSQL and emits the
//! `atelier.source_evidence.matrix_recorded` EventLedger family.

use std::path::Path;

use crate::atelier::source_evidence::{
    AnchorVerificationStatus, NewAnchorVerificationRecord, NewSourceEvidenceMatrix,
    NewSourceEvidenceRecord, SourceMaturityStatus,
};

/// Stable matrix id for the MT-133 kernel-diagnostics anchor verification.
pub const KERNEL_DIAGNOSTICS_ANCHOR_VERIFICATION_MATRIX_ID: &str =
    "wp-kernel-005.kernel-diagnostics.anchor-verification@1";

/// The kernel diagnostic surfaces the MT-133 contract names, in contract order.
pub const KERNEL_DIAGNOSTIC_SURFACES: &[&str] = &[
    "sessions",
    "command-catalog",
    "workflow-engine",
    "dcc",
    "locus",
    "flight-recorder",
    "visual-capture",
    "build-diagnostics",
];

/// Declared anchor specification for one kernel diagnostic surface.
struct KernelDiagnosticSurfaceSpec {
    /// Stable surface token (member of [`KERNEL_DIAGNOSTIC_SURFACES`]).
    surface: &'static str,
    source_label: &'static str,
    product_area: &'static str,
    /// The primary product module backing the surface.
    expected_product_path: &'static str,
    /// Every product path (modules + projections) that must exist for the
    /// anchor to be `VERIFIED`. Includes `expected_product_path`.
    verified_product_paths: &'static [&'static str],
    /// Runtime proof surfaces (test files) for the product modules.
    proof_refs: &'static [&'static str],
}

const KERNEL_DIAGNOSTIC_SURFACE_SPECS: &[KernelDiagnosticSurfaceSpec] = &[
    KernelDiagnosticSurfaceSpec {
        surface: "sessions",
        source_label: "Session runtime product surface (broker, transcript, checkpoint)",
        product_area: "kernel.sessions",
        expected_product_path: "src/backend/handshake_core/src/kernel/session_broker.rs",
        verified_product_paths: &[
            "src/backend/handshake_core/src/kernel/session_broker.rs",
            "src/backend/handshake_core/src/session_transcript/mod.rs",
            "src/backend/handshake_core/src/session_checkpoint/mod.rs",
        ],
        proof_refs: &["src/backend/handshake_core/tests/atelier_command_log_session_tests.rs"],
    },
    KernelDiagnosticSurfaceSpec {
        surface: "command-catalog",
        source_label: "Kernel action/command catalog product surface",
        product_area: "kernel.command_catalog",
        expected_product_path: "src/backend/handshake_core/src/kernel/action_catalog.rs",
        verified_product_paths: &["src/backend/handshake_core/src/kernel/action_catalog.rs"],
        proof_refs: &["src/backend/handshake_core/tests/kernel_action_catalog_tests.rs"],
    },
    KernelDiagnosticSurfaceSpec {
        surface: "workflow-engine",
        source_label: "Workflow Engine product surface",
        product_area: "kernel.workflow_engine",
        expected_product_path: "src/backend/handshake_core/src/workflows.rs",
        verified_product_paths: &[
            "src/backend/handshake_core/src/workflows.rs",
            "src/backend/handshake_core/src/kernel/workflow_transition_registry.rs",
        ],
        proof_refs: &[
            "src/backend/handshake_core/tests/kernel_workflow_transition_registry_tests.rs",
            "src/backend/handshake_core/tests/atelier_workflow_spec_tests.rs",
        ],
    },
    KernelDiagnosticSurfaceSpec {
        surface: "dcc",
        source_label: "DCC runtime surface and app projection",
        product_area: "kernel.dcc",
        expected_product_path: "src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs",
        verified_product_paths: &[
            "src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs",
            "src/backend/handshake_core/src/kernel/dcc_layout_projection_registry.rs",
            "app/src/components/KernelDccProjectionView.tsx",
        ],
        proof_refs: &[
            "src/backend/handshake_core/tests/kernel_dcc_mvp_runtime_surface_tests.rs",
            "src/backend/handshake_core/tests/kernel_dcc_layout_projection_registry_tests.rs",
        ],
    },
    KernelDiagnosticSurfaceSpec {
        surface: "locus",
        source_label: "Locus work-tracking product surface",
        product_area: "kernel.locus",
        expected_product_path: "src/backend/handshake_core/src/locus/mod.rs",
        verified_product_paths: &[
            "src/backend/handshake_core/src/locus/mod.rs",
            "src/backend/handshake_core/src/locus/task_board.rs",
            "src/backend/handshake_core/src/kernel/locus_mt_validation_work_graph.rs",
        ],
        proof_refs: &[
            "src/backend/handshake_core/tests/kernel_locus_mt_validation_work_graph_tests.rs",
        ],
    },
    KernelDiagnosticSurfaceSpec {
        surface: "flight-recorder",
        source_label: "Flight Recorder product surface and app projection",
        product_area: "kernel.flight_recorder",
        expected_product_path: "src/backend/handshake_core/src/flight_recorder/mod.rs",
        verified_product_paths: &[
            "src/backend/handshake_core/src/flight_recorder/mod.rs",
            "src/backend/handshake_core/src/flight_recorder/fr_event_registry.rs",
            "app/src/components/FlightRecorderView.tsx",
        ],
        proof_refs: &["src/backend/handshake_core/tests/kernel_flight_recorder_tests.rs"],
    },
    KernelDiagnosticSurfaceSpec {
        surface: "visual-capture",
        source_label: "Visual capture / visual debugging product surface",
        product_area: "kernel.visual_capture",
        expected_product_path:
            "src/backend/handshake_core/src/kernel/product_screenshot_capture.rs",
        verified_product_paths: &[
            "src/backend/handshake_core/src/kernel/product_screenshot_capture.rs",
            "src/backend/handshake_core/src/kernel/visual_debugging_loop.rs",
        ],
        proof_refs: &[
            "src/backend/handshake_core/tests/kernel_product_screenshot_capture_tests.rs",
            "src/backend/handshake_core/tests/kernel_visual_debugging_loop_tests.rs",
        ],
    },
    KernelDiagnosticSurfaceSpec {
        surface: "build-diagnostics",
        source_label: "Build diagnostics product surface (diagnostics schema + HBR gates)",
        product_area: "kernel.build_diagnostics",
        expected_product_path: "src/backend/handshake_core/src/diagnostics/mod.rs",
        verified_product_paths: &[
            "src/backend/handshake_core/src/diagnostics/mod.rs",
            "src/backend/handshake_core/src/hbr/mod.rs",
            "src/backend/handshake_core/src/hbr/violation.rs",
        ],
        proof_refs: &[
            "src/backend/handshake_core/tests/hbr_violation_tests.rs",
            "src/backend/handshake_core/tests/atelier_diagnostics_typed_surfaces_tests.rs",
        ],
    },
];

fn surface_source_id(surface: &str) -> String {
    format!("MT-133.{surface}")
}

fn surface_anchor_id(surface: &str) -> String {
    format!("ANCHOR-MT-133-{surface}")
}

fn surface_source_ref(surface: &str) -> String {
    format!("source://handshake/kernel-diagnostics/{surface}")
}

/// The declared MT-133 kernel-diagnostics anchor matrix: one source row and
/// one anchor row per contract surface, all declared `VERIFIED` against the
/// product paths in [`KERNEL_DIAGNOSTIC_SURFACE_SPECS`]. Use
/// [`verify_kernel_diagnostics_anchor_matrix`] to actually check those paths
/// against a real source tree before recording.
pub fn kernel_diagnostics_anchor_verification_matrix(
    recorded_by: impl Into<String>,
) -> NewSourceEvidenceMatrix {
    build_matrix(recorded_by.into(), None)
}

/// Filesystem-verified MT-133 matrix: every declared product path is resolved
/// against `repo_root` (the repository root that contains
/// `src/backend/handshake_core`). Surfaces whose anchors all exist stay
/// `VERIFIED`/`DONE`; surfaces with any missing path are downgraded to
/// `BLOCKED_MISSING_ANCHOR`/`BLOCKED` and the missing paths are written into
/// `blocking_reason` / `gap_reason` so a no-context model can see exactly
/// which product anchor is absent.
pub fn verify_kernel_diagnostics_anchor_matrix(
    repo_root: &Path,
    recorded_by: impl Into<String>,
) -> NewSourceEvidenceMatrix {
    build_matrix(recorded_by.into(), Some(repo_root))
}

fn build_matrix(recorded_by: String, repo_root: Option<&Path>) -> NewSourceEvidenceMatrix {
    let mut sources = Vec::with_capacity(KERNEL_DIAGNOSTIC_SURFACE_SPECS.len());
    let mut anchors = Vec::with_capacity(KERNEL_DIAGNOSTIC_SURFACE_SPECS.len());

    for spec in KERNEL_DIAGNOSTIC_SURFACE_SPECS {
        let declared_paths: Vec<String> = spec
            .verified_product_paths
            .iter()
            .map(|path| path.to_string())
            .collect();
        let proof_refs: Vec<String> = spec.proof_refs.iter().map(|path| path.to_string()).collect();

        let missing_paths: Vec<String> = match repo_root {
            None => Vec::new(),
            Some(root) => declared_paths
                .iter()
                .chain(proof_refs.iter())
                .filter(|path| !root.join(path.as_str()).exists())
                .cloned()
                .collect(),
        };

        if missing_paths.is_empty() {
            sources.push(NewSourceEvidenceRecord {
                source_id: surface_source_id(spec.surface),
                source_label: spec.source_label.to_string(),
                source_ref: surface_source_ref(spec.surface),
                product_area: spec.product_area.to_string(),
                maturity_status: SourceMaturityStatus::Done,
                implementation_status: "verified_product_path".to_string(),
                evidence_refs: declared_paths.clone(),
                proof_refs,
                gap_reason: None,
            });
            anchors.push(NewAnchorVerificationRecord {
                anchor_id: surface_anchor_id(spec.surface),
                source_id: surface_source_id(spec.surface),
                anchor_label: format!("{} product anchor", spec.source_label),
                expected_product_path: spec.expected_product_path.to_string(),
                verification_status: AnchorVerificationStatus::Verified,
                verified_product_paths: declared_paths,
                blocking_reason: None,
            });
        } else {
            let missing_list = missing_paths.join(", ");
            let present_paths: Vec<String> = declared_paths
                .iter()
                .filter(|path| !missing_paths.contains(path))
                .cloned()
                .collect();
            sources.push(NewSourceEvidenceRecord {
                source_id: surface_source_id(spec.surface),
                source_label: spec.source_label.to_string(),
                source_ref: surface_source_ref(spec.surface),
                product_area: spec.product_area.to_string(),
                maturity_status: SourceMaturityStatus::Blocked,
                implementation_status: "missing_product_path".to_string(),
                evidence_refs: declared_paths,
                proof_refs,
                gap_reason: Some(format!(
                    "missing kernel-diagnostics product anchors: {missing_list}"
                )),
            });
            anchors.push(NewAnchorVerificationRecord {
                anchor_id: surface_anchor_id(spec.surface),
                source_id: surface_source_id(spec.surface),
                anchor_label: format!("{} product anchor", spec.source_label),
                expected_product_path: spec.expected_product_path.to_string(),
                verification_status: AnchorVerificationStatus::BlockedMissingAnchor,
                verified_product_paths: present_paths,
                blocking_reason: Some(format!(
                    "missing kernel-diagnostics product anchors: {missing_list}"
                )),
            });
        }
    }

    NewSourceEvidenceMatrix {
        matrix_id: KERNEL_DIAGNOSTICS_ANCHOR_VERIFICATION_MATRIX_ID.to_string(),
        recorded_by,
        sources,
        anchors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_matrix_covers_every_contract_surface() {
        let matrix = kernel_diagnostics_anchor_verification_matrix("unit-test");
        assert_eq!(matrix.sources.len(), KERNEL_DIAGNOSTIC_SURFACES.len());
        assert_eq!(matrix.anchors.len(), KERNEL_DIAGNOSTIC_SURFACES.len());
        for surface in KERNEL_DIAGNOSTIC_SURFACES {
            assert!(
                matrix
                    .anchors
                    .iter()
                    .any(|anchor| anchor.anchor_id == surface_anchor_id(surface)),
                "missing declared anchor for surface {surface}"
            );
        }
    }

    #[test]
    fn verification_downgrades_missing_anchors_to_blocked() {
        let bogus_root = std::env::temp_dir().join("hsk-mt133-definitely-missing-root");
        let matrix = verify_kernel_diagnostics_anchor_matrix(&bogus_root, "unit-test");
        assert!(
            matrix.anchors.iter().all(|anchor| {
                anchor.verification_status == AnchorVerificationStatus::BlockedMissingAnchor
            }),
            "anchors must not be VERIFIED against a source tree that lacks the product paths"
        );
        assert!(matrix.anchors.iter().all(|anchor| {
            anchor
                .blocking_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("missing kernel-diagnostics"))
        }));
        assert!(matrix
            .sources
            .iter()
            .all(|source| source.maturity_status == SourceMaturityStatus::Blocked));
    }
}
