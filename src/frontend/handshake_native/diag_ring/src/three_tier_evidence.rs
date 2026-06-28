//! `three_tier_evidence` — the HBR-INT-009 THREE-TIER DIAGNOSTIC EVIDENCE FORMAT
//! (WP-KERNEL-012 MT-095, Master Spec v02.196 §5.8 internal_diagnostics + §6.13 Palmistry + CX-981).
//!
//! # What this is
//!
//! The CODER_PROTOCOL "HBR-INT-009 duty" requires that for EVERY observable runtime behavior an MT
//! touches, the build evidence records each of the three diagnostic tiers as `WIRED`,
//! `NOT_APPLICABLE`-with-reason, or `DEFERRED`-with-reason — and *never silently omits a tier*. Until
//! now that obligation lived only as the prose `hbr_int_009_tier_obligations` array hand-written into
//! each MT contract. This module is the MACHINE-READABLE, VALIDATOR-CHECKABLE form of that obligation:
//!
//! - [`ThreeTierDiagnosticWiringRecord`] — the typed record a WP/MT test run EMITS to prove its
//!   per-tier wiring for one observable behavior.
//! - [`ThreeTierDiagnosticWiringRecord::emit`] — atomically writes
//!   [`EVIDENCE_FILE_NAME`] (`test_run_with_three_tier_diagnostic_wiring_record.json`) to an external
//!   artifact directory. It validates first, so a malformed record physically cannot be written.
//! - [`ThreeTierDiagnosticWiringRecord::validate`] — enforces the HBR-INT-009 rules: all three tiers
//!   present exactly once (the "never silently skip a tier" rule), every `WIRED` row carries a
//!   non-empty `proof_ref`, every `NOT_APPLICABLE`/`DEFERRED` row carries a non-empty `reason`.
//!
//! # Why this lives in `handshake-diag-ring` (placement rationale, AC-015-6)
//!
//! This crate is the ONLY crate that BOTH product binaries already depend on: the Handshake binary
//! (`handshake-native`, Tier-2 writer) and the external Palmistry watcher (Tier-3 reader). Those two
//! binaries deliberately share **no** dependency edge with each other (see
//! `handshake_native::diagnostics::survivor_forward` — "the only shared crate is
//! `handshake-diag-ring`"). The three-tier evidence format must be emittable by ANY WP's test run —
//! a handshake-native test, a palmistry test, the WP-KERNEL-016 retrofit, or a future
//! backend/frontend WP — so it belongs in the shared light substrate, not inside either heavy binary.
//! Placing it here means future emitters depend on a tiny `serde`/`serde_json` crate, never on
//! `egui`/`wgpu`. (The handshake-native `diagnostics` module was the considered alternative; it was
//! rejected because palmistry and non-frontend WPs could not then emit without a new dependency edge
//! into the frontend binary.)
//!
//! # Privacy note — this is NOT a `DiagEvent`
//!
//! [`crate::schema::DiagEvent`] is a cross-process shared-memory POD record whose typed allowlist
//! forbids ALL text (no `String`/`Vec`/blob) so it can never smuggle project content across the ring
//! into Palmistry. This record is a DIFFERENT concern: it is a BUILD/TEST-TIME governance evidence
//! FILE authored by developers and test runs. It never enters the shared-memory ring. It carries only
//! governance identifiers — work-packet / microtask ids, a tier status, a human reason, and a
//! `proof_ref` pointing at a test or MT — and NO project/user content. The two types are intentionally
//! distinct: `DiagEvent` = runtime telemetry crossing into Palmistry; this = a JSON wiring-posture
//! record describing how a feature is diagnostically wired.
//!
//! # Governance-side validator (the OTHER half — NOT authored here)
//!
//! This module is the PRODUCT-side half: schema + emitter + [`ThreeTierDiagnosticWiringRecord::validate`].
//! The GOVERNANCE-side gate that consumes the emitted file and marks WP acceptance rows is a
//! validator/governance-lane item (an RGF / validator-protocol update under `.GOV/`), which is
//! `forbidden_paths` for the product coder lane and therefore is raised as a typed handoff, not edited
//! here. The contract that governance gate must implement is:
//!
//! 1. For each retrofitted/observable surface in a WP, locate its emitted
//!    `test_run_with_three_tier_diagnostic_wiring_record.json` under the WP's external artifact dir.
//! 2. Deserialize it with [`ThreeTierDiagnosticWiringRecord::from_json_str`] (or [`load`](ThreeTierDiagnosticWiringRecord::load)).
//! 3. Run [`ThreeTierDiagnosticWiringRecord::validate`]. A non-empty error list MUST fail the WP row
//!    (a missing tier, an empty reason, or an empty proof_ref blocks acceptance).
//! 4. Map the three tier rows onto the WP's `hbr_int_009_tier_obligations` acceptance rows, asserting
//!    the emitted `tier`/`status` spellings equal the contract vocabulary (see [`congruence`]).
//! 5. Record the per-tier `WIRED`/`NOT_APPLICABLE`/`DEFERRED` posture in the WP acceptance matrix.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Canonical file name an emitted three-tier evidence record is written to. The governance-side gate
/// looks for exactly this name under a WP's external artifact directory. Named verbatim per the
/// MT-095 contract.
pub const EVIDENCE_FILE_NAME: &str = "test_run_with_three_tier_diagnostic_wiring_record.json";

/// The three diagnostic tiers of Handshake's decoupled diagnostic model (Master Spec v02.196 §5.8 +
/// §6.13). The `#[serde(rename = ...)]` spellings are CONGRUENT (AC-015-3) with the `tier` vocabulary
/// of the `hbr_int_009_tier_obligations` array used in every WP-KERNEL-012 MT contract, so a model
/// reading either the prose contract or this machine record understands the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagTier {
    /// Tier 1 — the kept-as-is backend business-event ledger (Flight Recorder).
    #[serde(rename = "FLIGHT_RECORDER")]
    FlightRecorder,
    /// Tier 2 — Handshake-native in-app self-diagnostics (panic hook, heartbeat, frame-time,
    /// CPU/RSS/GPU, the open diagnostic-event API).
    #[serde(rename = "INTERNAL_DIAGNOSTICS")]
    InternalDiagnostics,
    /// Tier 3 — the external out-of-process watcher that survives freezes/crashes (Palmistry).
    #[serde(rename = "PALMISTRY")]
    Palmistry,
}

impl DiagTier {
    /// The three tiers in canonical order. A well-formed record accounts for every one of these
    /// exactly once; [`ThreeTierDiagnosticWiringRecord::validate`] iterates this set to enforce the
    /// "never silently skip a tier" rule.
    pub const ALL: [DiagTier; 3] = [
        DiagTier::FlightRecorder,
        DiagTier::InternalDiagnostics,
        DiagTier::Palmistry,
    ];

    /// The canonical contract spelling (the `hbr_int_009_tier_obligations` `tier` value). This is the
    /// SAME string `serde` emits via the `rename` above; [`congruence`] asserts they never drift.
    #[must_use]
    pub const fn contract_spelling(self) -> &'static str {
        match self {
            DiagTier::FlightRecorder => "FLIGHT_RECORDER",
            DiagTier::InternalDiagnostics => "INTERNAL_DIAGNOSTICS",
            DiagTier::Palmistry => "PALMISTRY",
        }
    }
}

impl fmt::Display for DiagTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.contract_spelling())
    }
}

/// The wiring status of one tier for one observable behavior. The `#[serde(rename = ...)]` spellings
/// are CONGRUENT (AC-015-3) with the `status` vocabulary of the `hbr_int_009_tier_obligations` array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WiringStatus {
    /// The tier is actively wired for this behavior; `proof_ref` MUST point at the proving test/MT.
    #[serde(rename = "WIRED")]
    Wired,
    /// The tier does not apply to this behavior; `reason` MUST explain why (e.g. "per-frame liveness
    /// signal deliberately not written to the Tier-1 business ledger").
    #[serde(rename = "NOT_APPLICABLE")]
    NotApplicable,
    /// The tier is intended but not yet shipped in this worktree; `reason` MUST explain the deferral +
    /// integration follow-up (the CODER_PROTOCOL "DEFERRED-with-reason, never silent skip" rule).
    #[serde(rename = "DEFERRED")]
    Deferred,
}

impl WiringStatus {
    /// The canonical contract spelling (the `hbr_int_009_tier_obligations` `status` value).
    #[must_use]
    pub const fn contract_spelling(self) -> &'static str {
        match self {
            WiringStatus::Wired => "WIRED",
            WiringStatus::NotApplicable => "NOT_APPLICABLE",
            WiringStatus::Deferred => "DEFERRED",
        }
    }
}

impl fmt::Display for WiringStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.contract_spelling())
    }
}

/// One tier's wiring evidence for one observable behavior. Construct via [`TierWiring::wired`],
/// [`TierWiring::not_applicable`], or [`TierWiring::deferred`] so the `status` and its required
/// companion field (`proof_ref` for `WIRED`, `reason` otherwise) can never be mismatched at the call
/// site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierWiring {
    /// Which tier this row describes.
    pub tier: DiagTier,
    /// Wiring status for the tier.
    pub status: WiringStatus,
    /// Human reason — REQUIRED (non-empty) for `NOT_APPLICABLE`/`DEFERRED`, empty for `WIRED`.
    pub reason: String,
    /// Proof reference (a test name / MT id) — REQUIRED (non-empty) for `WIRED`, empty otherwise.
    pub proof_ref: String,
}

impl TierWiring {
    /// A `WIRED` tier row. `proof_ref` should name the proving test or MT (e.g. `"MT-084 test_heartbeat.rs"`).
    #[must_use]
    pub fn wired(tier: DiagTier, proof_ref: impl Into<String>) -> Self {
        Self {
            tier,
            status: WiringStatus::Wired,
            reason: String::new(),
            proof_ref: proof_ref.into(),
        }
    }

    /// A `NOT_APPLICABLE` tier row with a required explanatory `reason`.
    #[must_use]
    pub fn not_applicable(tier: DiagTier, reason: impl Into<String>) -> Self {
        Self {
            tier,
            status: WiringStatus::NotApplicable,
            reason: reason.into(),
            proof_ref: String::new(),
        }
    }

    /// A `DEFERRED` tier row with a required explanatory `reason` (deferral + follow-up).
    #[must_use]
    pub fn deferred(tier: DiagTier, reason: impl Into<String>) -> Self {
        Self {
            tier,
            status: WiringStatus::Deferred,
            reason: reason.into(),
            proof_ref: String::new(),
        }
    }
}

/// A single observable runtime behavior's three-tier diagnostic wiring record — the machine-readable
/// HBR-INT-009 evidence a WP/MT test run emits and the governance gate validates. One record == one
/// observable behavior; each of the three tiers appears exactly once in `tiers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreeTierDiagnosticWiringRecord {
    /// The work-packet id that emitted this record (e.g. `"WP-KERNEL-012-..."`).
    pub wp_id: String,
    /// The microtask id that emitted this record (e.g. `"MT-095"`).
    pub mt_id: String,
    /// The observable runtime behavior this record covers (e.g. `"ui_thread_heartbeat"`).
    pub observable_behavior: String,
    /// RFC3339 UTC timestamp of the run that produced this record. Use [`run_at_now`].
    pub run_at: String,
    /// The three tier wiring rows. Validated to contain each [`DiagTier`] exactly once.
    pub tiers: Vec<TierWiring>,
}

impl ThreeTierDiagnosticWiringRecord {
    /// Construct a record from its parts. Prefer passing exactly three [`TierWiring`] rows (one per
    /// tier); [`validate`](Self::validate) enforces that invariant.
    #[must_use]
    pub fn new(
        wp_id: impl Into<String>,
        mt_id: impl Into<String>,
        observable_behavior: impl Into<String>,
        run_at: impl Into<String>,
        tiers: Vec<TierWiring>,
    ) -> Self {
        Self {
            wp_id: wp_id.into(),
            mt_id: mt_id.into(),
            observable_behavior: observable_behavior.into(),
            run_at: run_at.into(),
            tiers,
        }
    }

    /// Enforce the HBR-INT-009 rules. Returns every problem found (not just the first) so a caller can
    /// fix all at once:
    ///
    /// - run metadata (`wp_id`, `mt_id`, `observable_behavior`, `run_at`) is non-empty;
    /// - each of the three tiers ([`DiagTier::ALL`]) is present EXACTLY once — a missing tier
    ///   ([`ValidationError::MissingTier`]) is the "never silently skip it" failure; a duplicated tier
    ///   ([`ValidationError::DuplicateTier`]) means two conflicting rows for one tier;
    /// - every `WIRED` row carries a non-empty `proof_ref` ([`ValidationError::MissingProofRef`]);
    /// - every `NOT_APPLICABLE`/`DEFERRED` row carries a non-empty `reason`
    ///   ([`ValidationError::MissingReason`]).
    ///
    /// `Ok(())` means the record is well-formed evidence the governance gate can mark rows from.
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Run metadata must be present (empty ids/behavior would make the evidence un-attributable).
        for (field, value) in [
            ("wp_id", &self.wp_id),
            ("mt_id", &self.mt_id),
            ("observable_behavior", &self.observable_behavior),
            ("run_at", &self.run_at),
        ] {
            if value.trim().is_empty() {
                errors.push(ValidationError::EmptyMetadata(field));
            }
        }

        // All three tiers present exactly once — the core "never silently skip a tier" rule.
        for tier in DiagTier::ALL {
            let count = self.tiers.iter().filter(|row| row.tier == tier).count();
            match count {
                0 => errors.push(ValidationError::MissingTier(tier)),
                1 => {}
                _ => errors.push(ValidationError::DuplicateTier(tier)),
            }
        }

        // Each row's required companion field must be non-empty (no empty evidence).
        for row in &self.tiers {
            match row.status {
                WiringStatus::Wired => {
                    if row.proof_ref.trim().is_empty() {
                        errors.push(ValidationError::MissingProofRef(row.tier));
                    }
                }
                WiringStatus::NotApplicable | WiringStatus::Deferred => {
                    if row.reason.trim().is_empty() {
                        errors.push(ValidationError::MissingReason(row.tier));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Serialize to pretty JSON with a trailing newline (POSIX-friendly text artifact).
    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        let mut s = serde_json::to_string_pretty(self)?;
        s.push('\n');
        Ok(s)
    }

    /// Parse a record from a JSON string (the governance gate's read path).
    pub fn from_json_str(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }

    /// Read + parse a record from a file (the governance gate's read path). A parse error is mapped to
    /// [`io::ErrorKind::InvalidData`] so the caller handles one error type.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        Self::from_json_str(&text)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    /// Validate then ATOMICALLY write [`EVIDENCE_FILE_NAME`] into `out_dir`, returning the written
    /// path. The directory is created if needed. Because [`validate`](Self::validate) runs first, a
    /// malformed record CANNOT be emitted — the "never silently skip a tier" guarantee holds at the
    /// write boundary, not just at read time.
    ///
    /// Atomicity: the JSON is written to a unique temp file in the same directory and then renamed
    /// over the destination, so a reader never observes a half-written file.
    pub fn emit(&self, out_dir: impl AsRef<Path>) -> Result<PathBuf, EmitError> {
        self.validate().map_err(EmitError::Invalid)?;

        let dir = out_dir.as_ref();
        fs::create_dir_all(dir)?;

        let json = self.to_json_pretty()?;
        let final_path = dir.join(EVIDENCE_FILE_NAME);

        // Unique temp name in the SAME dir so the rename is a same-volume atomic replace.
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp_path = dir.join(format!(
            ".{}.tmp.{}.{}",
            EVIDENCE_FILE_NAME,
            std::process::id(),
            nanos
        ));

        fs::write(&tmp_path, json.as_bytes())?;
        // std::fs::rename replaces an existing destination on both Windows and Unix.
        if let Err(err) = fs::rename(&tmp_path, &final_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(EmitError::Io(err));
        }
        Ok(final_path)
    }
}

/// One reason a [`ThreeTierDiagnosticWiringRecord`] is not well-formed HBR-INT-009 evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A required run-metadata field was empty (the `&str` names the field).
    EmptyMetadata(&'static str),
    /// A tier was omitted entirely — the "never silently skip a tier" failure.
    MissingTier(DiagTier),
    /// A tier appeared more than once (two conflicting rows for one tier).
    DuplicateTier(DiagTier),
    /// A `WIRED` tier carried no `proof_ref` (WIRED with no proof is empty evidence).
    MissingProofRef(DiagTier),
    /// A `NOT_APPLICABLE`/`DEFERRED` tier carried no `reason` (status with no justification).
    MissingReason(DiagTier),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::EmptyMetadata(field) => {
                write!(f, "run metadata field `{field}` is empty")
            }
            ValidationError::MissingTier(tier) => {
                write!(f, "tier {tier} is missing (every tier must be accounted for)")
            }
            ValidationError::DuplicateTier(tier) => {
                write!(f, "tier {tier} appears more than once")
            }
            ValidationError::MissingProofRef(tier) => {
                write!(f, "tier {tier} is WIRED but carries no proof_ref")
            }
            ValidationError::MissingReason(tier) => {
                write!(
                    f,
                    "tier {tier} is NOT_APPLICABLE/DEFERRED but carries no reason"
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Failure modes of [`ThreeTierDiagnosticWiringRecord::emit`].
#[derive(Debug)]
pub enum EmitError {
    /// The record failed [`ThreeTierDiagnosticWiringRecord::validate`]; nothing was written.
    Invalid(Vec<ValidationError>),
    /// Serializing the record to JSON failed.
    Serialize(serde_json::Error),
    /// A filesystem operation (create dir / write temp / rename) failed.
    Io(io::Error),
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::Invalid(errors) => {
                write!(f, "record failed validation:")?;
                for err in errors {
                    write!(f, " [{err}]")?;
                }
                Ok(())
            }
            EmitError::Serialize(err) => write!(f, "serialize evidence record: {err}"),
            EmitError::Io(err) => write!(f, "write evidence record: {err}"),
        }
    }
}

impl std::error::Error for EmitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EmitError::Invalid(_) => None,
            EmitError::Serialize(err) => Some(err),
            EmitError::Io(err) => Some(err),
        }
    }
}

impl From<io::Error> for EmitError {
    fn from(err: io::Error) -> Self {
        EmitError::Io(err)
    }
}

impl From<serde_json::Error> for EmitError {
    fn from(err: serde_json::Error) -> Self {
        EmitError::Serialize(err)
    }
}

/// Current time as an RFC3339 UTC timestamp string (`YYYY-MM-DDThh:mm:ssZ`) for
/// [`ThreeTierDiagnosticWiringRecord::run_at`]. Computed from the system clock with no external date
/// crate so this stays a pure-substrate dependency-light helper.
#[must_use]
pub fn run_at_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_rfc3339_utc(secs)
}

/// Format a UNIX-epoch-seconds value as an RFC3339 UTC timestamp (`YYYY-MM-DDThh:mm:ssZ`).
///
/// Uses Howard Hinnant's `civil_from_days` algorithm (public-domain) to convert the day index to a
/// proleptic-Gregorian date, so leap years and month lengths are exact without a calendar crate.
#[must_use]
pub fn format_rfc3339_utc(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let secs_of_day = unix_secs % 86_400;
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;

    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Convert a day index (days since 1970-01-01) to a `(year, month, day)` proleptic-Gregorian date.
/// Howard Hinnant's `civil_from_days` (public domain). `month` is 1..=12, `day` is 1..=31.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // day-of-era, [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // year-of-era, [0, 399]
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day-of-year (Mar-based), [0, 365]
    let mp = (5 * doy + 2) / 153; // month-of-year (Mar-based), [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

/// The canonical contract vocabulary, exposed so a congruence check (AC-015-3) can assert the
/// serialized enum spellings never drift from the `hbr_int_009_tier_obligations` array used in the
/// WP-KERNEL-012 MT contracts. The tuples are `(serde-serialized spelling, contract spelling)` — they
/// must be identical.
pub mod congruence {
    use super::{DiagTier, WiringStatus};

    /// The three `tier` spellings the `hbr_int_009_tier_obligations` array uses.
    pub const CONTRACT_TIERS: [&str; 3] = ["FLIGHT_RECORDER", "INTERNAL_DIAGNOSTICS", "PALMISTRY"];

    /// The three `status` spellings the `hbr_int_009_tier_obligations` array uses.
    pub const CONTRACT_STATUSES: [&str; 3] = ["WIRED", "NOT_APPLICABLE", "DEFERRED"];

    /// `(enum, contract spelling)` pairs for the tiers, for an exhaustive congruence assertion.
    #[must_use]
    pub fn tier_pairs() -> [(DiagTier, &'static str); 3] {
        [
            (DiagTier::FlightRecorder, "FLIGHT_RECORDER"),
            (DiagTier::InternalDiagnostics, "INTERNAL_DIAGNOSTICS"),
            (DiagTier::Palmistry, "PALMISTRY"),
        ]
    }

    /// `(enum, contract spelling)` pairs for the statuses, for an exhaustive congruence assertion.
    #[must_use]
    pub fn status_pairs() -> [(WiringStatus, &'static str); 3] {
        [
            (WiringStatus::Wired, "WIRED"),
            (WiringStatus::NotApplicable, "NOT_APPLICABLE"),
            (WiringStatus::Deferred, "DEFERRED"),
        ]
    }
}

#[cfg(test)]
mod tests {
    //! In-crate unit tests for the pure logic (no filesystem). The artifact-emitting + external-root
    //! proofs live in `tests/test_three_tier_evidence.rs` so they run from the crate dir with the
    //! external artifact root.
    use super::*;

    fn complete_tiers() -> Vec<TierWiring> {
        vec![
            TierWiring::not_applicable(DiagTier::FlightRecorder, "Tier-1 business ledger n/a"),
            TierWiring::wired(DiagTier::InternalDiagnostics, "MT-084 test_heartbeat.rs"),
            TierWiring::wired(DiagTier::Palmistry, "MT-091 freeze read"),
        ]
    }

    fn complete_record() -> ThreeTierDiagnosticWiringRecord {
        ThreeTierDiagnosticWiringRecord::new(
            "WP-KERNEL-012",
            "MT-095",
            "ui_thread_heartbeat",
            "2026-06-28T00:00:00Z",
            complete_tiers(),
        )
    }

    #[test]
    fn valid_record_passes() {
        assert!(complete_record().validate().is_ok());
    }

    #[test]
    fn missing_tier_fails() {
        let mut rec = complete_record();
        rec.tiers.retain(|t| t.tier != DiagTier::Palmistry);
        let errs = rec.validate().expect_err("missing PALMISTRY must fail");
        assert!(errs.contains(&ValidationError::MissingTier(DiagTier::Palmistry)));
    }

    #[test]
    fn duplicate_tier_fails() {
        let mut rec = complete_record();
        rec.tiers
            .push(TierWiring::wired(DiagTier::InternalDiagnostics, "dupe"));
        let errs = rec.validate().expect_err("duplicate tier must fail");
        assert!(errs.contains(&ValidationError::DuplicateTier(DiagTier::InternalDiagnostics)));
    }

    #[test]
    fn wired_without_proof_ref_fails() {
        let mut rec = complete_record();
        // Force a WIRED row with a blank proof_ref.
        for row in &mut rec.tiers {
            if row.tier == DiagTier::InternalDiagnostics {
                row.proof_ref = "   ".to_string();
            }
        }
        let errs = rec.validate().expect_err("WIRED w/o proof_ref must fail");
        assert!(errs.contains(&ValidationError::MissingProofRef(DiagTier::InternalDiagnostics)));
    }

    #[test]
    fn not_applicable_without_reason_fails() {
        let mut rec = complete_record();
        for row in &mut rec.tiers {
            if row.tier == DiagTier::FlightRecorder {
                row.reason = String::new();
            }
        }
        let errs = rec.validate().expect_err("NOT_APPLICABLE w/o reason must fail");
        assert!(errs.contains(&ValidationError::MissingReason(DiagTier::FlightRecorder)));
    }

    #[test]
    fn deferred_without_reason_fails() {
        let rec = ThreeTierDiagnosticWiringRecord::new(
            "WP-KERNEL-012",
            "MT-095",
            "some_behavior",
            "2026-06-28T00:00:00Z",
            vec![
                TierWiring::not_applicable(DiagTier::FlightRecorder, "n/a"),
                TierWiring::deferred(DiagTier::InternalDiagnostics, ""),
                TierWiring::wired(DiagTier::Palmistry, "MT-091"),
            ],
        );
        let errs = rec.validate().expect_err("DEFERRED w/o reason must fail");
        assert!(errs.contains(&ValidationError::MissingReason(DiagTier::InternalDiagnostics)));
    }

    #[test]
    fn empty_metadata_fails() {
        let rec = ThreeTierDiagnosticWiringRecord::new("", "MT-095", "b", "t", complete_tiers());
        let errs = rec.validate().expect_err("empty wp_id must fail");
        assert!(errs.contains(&ValidationError::EmptyMetadata("wp_id")));
    }

    #[test]
    fn serde_spellings_are_congruent_with_contract_vocabulary() {
        // AC-015-3: the serialized enum spellings MUST equal the hbr_int_009_tier_obligations vocab.
        for (tier, spelling) in congruence::tier_pairs() {
            let json = serde_json::to_string(&tier).expect("serialize tier");
            assert_eq!(json, format!("\"{spelling}\""), "tier spelling drift");
            assert_eq!(tier.contract_spelling(), spelling);
        }
        for (status, spelling) in congruence::status_pairs() {
            let json = serde_json::to_string(&status).expect("serialize status");
            assert_eq!(json, format!("\"{spelling}\""), "status spelling drift");
            assert_eq!(status.contract_spelling(), spelling);
        }
    }

    #[test]
    fn rfc3339_formatter_is_correct_at_known_epochs() {
        // Hand-verifiable anchors: epoch 0, one day, and one second before a day boundary.
        assert_eq!(format_rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_rfc3339_utc(86_399), "1970-01-01T23:59:59Z");
        assert_eq!(format_rfc3339_utc(86_400), "1970-01-02T00:00:00Z");
        // A leap-year boundary: 2020-02-29 exists. 2020-02-29T00:00:00Z = 1582934400.
        assert_eq!(format_rfc3339_utc(1_582_934_400), "2020-02-29T00:00:00Z");
    }

    #[test]
    fn emit_refuses_invalid_record() {
        // The gate holds at the WRITE boundary: an invalid record cannot be emitted.
        let mut rec = complete_record();
        rec.tiers.retain(|t| t.tier != DiagTier::Palmistry);
        let dir = std::env::temp_dir().join(format!(
            "handshake-3tier-invalid-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        match rec.emit(&dir) {
            Err(EmitError::Invalid(errs)) => {
                assert!(errs.contains(&ValidationError::MissingTier(DiagTier::Palmistry)));
            }
            other => panic!("emit must refuse an invalid record, got {other:?}"),
        }
        // No file should have been written.
        assert!(!dir.join(EVIDENCE_FILE_NAME).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
