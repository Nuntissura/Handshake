//! KB003 Validation runner module tree.
//!
//! Spans MT-030..MT-039 of `WP-KERNEL-003-Sandbox-Validation-Promotion-v1`:
//!
//! - `adapter_health` (MT-030)  — sandbox adapter health projection consumed
//!   by validation pre-flight so `UNSUPPORTED` isolation is visible before
//!   a run begins.
//! - `status`         (MT-029 carrier; full taxonomy used here) — typed
//!   `ValidationStatus` enum covering PASS / FAIL / BLOCKED / ADVISORY_ONLY /
//!   UNSUPPORTED / SKIPPED_WITH_REASON / ERROR.
//! - `descriptor`     — `ValidationDescriptor` trait + allowlist + concrete
//!   descriptors used by the runner.
//! - `run`            — `ValidationRun` lifecycle types.
//! - `report`         — `ValidationReport` carrying status, evidence, and
//!   artifact refs (consumed downstream by Batch E promotion+artifacts).
//! - `patch_proposal` (MT-031) — `PatchProposal` envelope with base ref +
//!   declared target ranges; proposals missing either field are rejected
//!   before validation starts.
//! - `candidate_range`(MT-032) — verifies actual changed paths/ranges are
//!   inside declared targets; unexpected edits surface as a typed rejection.
//! - `diff_capture`   (MT-033) — captures a candidate diff as a stable
//!   artifact (`SandboxDiff` content-hashed); identical inputs => identical
//!   hash.
//! - `artifact_bundle`(MT-034) — canonical artifact-bundle manifest with a
//!   deterministic bundle hash over canonicalized members.
//! - `log_capture`    (MT-035) — bounded stdout/stderr log capture stored as
//!   a `SandboxLog` artifact so evidence never lives only in scrollback.
//! - `environment_manifest` (MT-036) — non-sensitive runtime identifiers
//!   with a per-field allowlist; secrets cannot enter the manifest.
//! - `command_manifest`(MT-037) — exact commands/checks that ran, with intent
//!   tags so validators can replay or reason about them.
//! - `visual_evidence`(MT-038) — KB002 screenshot/DOM/log evidence linkage
//!   to validation reports (GUI lanes).
//! - `redaction_report`(MT-039) — exportable redaction report listing
//!   denied artifacts; default export is redacted.

pub mod adapter_health;
pub mod artifact_bundle;
pub mod candidate_range;
pub mod check_runner_adapter;
pub mod command_manifest;
pub mod descriptor;
pub mod diff_capture;
pub mod environment_manifest;
pub mod log_capture;
pub mod patch_proposal;
pub mod redaction_report;
pub mod report;
pub mod run;
pub mod status;
pub mod visual_evidence;

pub use check_runner_adapter::{
    ValidationCheckRunner, ValidationCheckRunnerError, ValidationContext,
};
pub use status::ValidationStatus;
