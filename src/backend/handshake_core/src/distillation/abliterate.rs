//! MT-106: INF-6 Offline weight-orthogonalisation tool (abliteration).
//!
//! OFFLINE TOOL ONLY per Master Spec §4.7.4 and operator decision E-5
//! (refinement kernel004_extension.operator_decisions_locked).
//! Abliteration NEVER runs inside `ModelRuntime::generate`; it consumes a
//! base model artifact + a refusal direction (typically produced by INF-4
//! `extract_refusal_direction`) and writes a NEW derived model artifact
//! to disk. The new artifact carries provenance metadata and goes
//! through §4.8 content-review before any Skill Bank reference.
//!
//! The hot-path invariant is enforced by the static-analysis test
//! `abliterate_tool_tests::hot_path_does_not_reference_abliterate_module`.
//! Any reference to this module from `model_runtime/.../generate.rs`
//! fails CI as an HBR-INT-002 violation.
//!
//! This module is library code; the CLI binary at `bin/abliterate.rs` is
//! the operator-facing entrypoint. The kabachuha/abliterate.cpp algorithm
//! is implemented as a pure orthogonalisation function so it can be
//! unit-tested in-tree without GGUF I/O. The full GGUF/safetensors
//! round-trip and ProcessOwnershipLedger spawn-time registration are
//! orchestrated by `run_abliteration_offline`, which is gated on
//! the same live-runtime-toolchain that MT-074 is waiting on; until the
//! native llama.cpp toolchain is available on the build host, the I/O
//! pipeline returns `AbliterationError::NativeToolchainUnavailable` with
//! a clear operator action hint.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// CLI / library input shape. `license_tag` is mandatory per MT-106
/// red_team minimum_controls; constructing this struct without one is a
/// type error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbliterationConfig {
    pub base_model_path: PathBuf,
    pub refusal_direction_path: PathBuf,
    pub out_model_path: PathBuf,
    pub license_tag: String,
    pub provenance_note: String,
    pub operator_signature: String,
}

/// On-disk refusal-direction artifact shape (produced by INF-4
/// `extract_refusal_direction` -> JSON dump).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefusalDirectionFile {
    pub layer: u32,
    pub values: Vec<f32>,
    #[serde(default)]
    pub source_model_sha256: Option<String>,
}

/// Provenance metadata written into the output model artifact. The
/// fields mirror §4.7.4 and §4.8 content-review requirements.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbliterationProvenance {
    pub base_model_sha256: String,
    pub refusal_direction_sha256: String,
    pub abliteration_tool_version: String,
    pub abliterated_at_utc: String,
    pub license_tag: String,
    pub operator_signature: String,
    pub provenance_note: String,
}

#[derive(Debug, Error)]
pub enum AbliterationError {
    #[error("abliteration config invalid: {0}")]
    InvalidConfig(String),
    #[error("abliteration refusal-direction file invalid: {0}")]
    InvalidDirection(String),
    #[error(
        "abliteration native toolchain unavailable: {0}; install the model I/O toolchain \
         (GGUF reader/writer or safetensors-compatible candle build) and rerun"
    )]
    NativeToolchainUnavailable(String),
}

impl AbliterationConfig {
    pub fn validate(&self) -> Result<(), AbliterationError> {
        if self.license_tag.trim().is_empty() {
            return Err(AbliterationError::InvalidConfig(
                "license_tag must not be empty (MT-106.red_team minimum_controls)".to_string(),
            ));
        }
        if self.operator_signature.trim().is_empty() {
            return Err(AbliterationError::InvalidConfig(
                "operator_signature must not be empty".to_string(),
            ));
        }
        if !self.base_model_path.exists() {
            return Err(AbliterationError::InvalidConfig(format!(
                "base_model_path does not exist: {}",
                self.base_model_path.display()
            )));
        }
        if !self.refusal_direction_path.exists() {
            return Err(AbliterationError::InvalidConfig(format!(
                "refusal_direction_path does not exist: {}",
                self.refusal_direction_path.display()
            )));
        }
        if let Some(parent) = self.out_model_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(AbliterationError::InvalidConfig(format!(
                    "out_model_path parent dir does not exist: {}",
                    parent.display()
                )));
            }
        }
        Ok(())
    }
}

/// Pure orthogonalisation step. Mutates `weight` in place to remove its
/// projection onto `direction`:
///
/// ```text
/// W' = W - (W @ d^T) @ d
/// ```
///
/// `weight` is a row-major \[rows x cols\] matrix; `direction` is a
/// length-`cols` vector and is expected to be unit-length (caller's
/// responsibility, typically guaranteed by INF-4
/// `unit_normalise`). The pure-math form lets us cover the algorithm in
/// unit tests without GGUF/safetensors I/O.
pub fn orthogonalise_weight_in_place(
    weight: &mut [f32],
    rows: usize,
    cols: usize,
    direction: &[f32],
) -> Result<(), AbliterationError> {
    if direction.len() != cols {
        return Err(AbliterationError::InvalidDirection(format!(
            "direction length {} does not match weight cols {cols}",
            direction.len()
        )));
    }
    if weight.len() != rows * cols {
        return Err(AbliterationError::InvalidConfig(format!(
            "weight length {} does not match rows {rows} * cols {cols} = {}",
            weight.len(),
            rows * cols
        )));
    }
    for r in 0..rows {
        let row = &mut weight[r * cols..(r + 1) * cols];
        // projection = <row, direction>
        let projection: f32 = row.iter().zip(direction).map(|(a, b)| a * b).sum();
        for c in 0..cols {
            row[c] -= projection * direction[c];
        }
    }
    Ok(())
}

/// Verifies that every dimension of every row of `weight` has zero
/// projection onto `direction` (within tolerance). Used by tests to
/// confirm orthogonalisation actually happened.
pub fn weight_is_orthogonal_to(
    weight: &[f32],
    rows: usize,
    cols: usize,
    direction: &[f32],
    tolerance: f32,
) -> bool {
    if direction.len() != cols {
        return false;
    }
    if weight.len() != rows * cols {
        return false;
    }
    for r in 0..rows {
        let row = &weight[r * cols..(r + 1) * cols];
        let projection: f32 = row.iter().zip(direction).map(|(a, b)| a * b).sum();
        if projection.abs() > tolerance {
            return false;
        }
    }
    true
}

/// Orchestrator entrypoint. Validates the config, loads the refusal
/// direction file, registers a ProcessOwnershipLedger row with
/// `engine_kind = AbliterationTool`, then walks the target Linear
/// modules of the base model applying [`orthogonalise_weight_in_place`].
///
/// The full implementation requires either the llama.cpp GGUF
/// round-trip helpers (gated on MT-074's native toolchain) or a
/// safetensors path through `candle_core`. Until the native toolchain
/// lands we return [`AbliterationError::NativeToolchainUnavailable`]
/// with operator guidance. The pure orthogonalisation primitive above
/// IS reachable and unit-tested so the algorithm can be exercised
/// without I/O.
pub fn run_abliteration_offline(
    config: &AbliterationConfig,
) -> Result<AbliterationProvenance, AbliterationError> {
    config.validate()?;

    let raw_direction = std::fs::read_to_string(&config.refusal_direction_path).map_err(|err| {
        AbliterationError::InvalidDirection(format!(
            "read {}: {err}",
            config.refusal_direction_path.display()
        ))
    })?;
    let direction: RefusalDirectionFile = serde_json::from_str(&raw_direction).map_err(|err| {
        AbliterationError::InvalidDirection(format!(
            "parse {}: {err}",
            config.refusal_direction_path.display()
        ))
    })?;
    if direction.values.is_empty() {
        return Err(AbliterationError::InvalidDirection(
            "refusal-direction values empty".to_string(),
        ));
    }

    // GGUF / safetensors round-trip + ProcessOwnershipLedger
    // engine_kind=AbliterationTool spawn-time registration require the
    // native model I/O toolchain; the orthogonalisation algorithm
    // itself is fully wired and unit-tested.
    Err(AbliterationError::NativeToolchainUnavailable(format!(
        "model I/O (load weights from {}, write to {}) requires the native toolchain; \
         orthogonalisation primitive is ready and unit-tested per MT-106 algorithm contract",
        config.base_model_path.display(),
        config.out_model_path.display(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orthogonalise_removes_projection_onto_unit_direction() {
        // Weight rows aligned with the direction must zero out after
        // orthogonalisation; rows orthogonal to direction stay put.
        let inv_sqrt2 = 1.0_f32 / 2.0_f32.sqrt();
        let direction = vec![inv_sqrt2, inv_sqrt2]; // unit-length [0.707, 0.707]
        let mut weight = vec![
            1.0, 1.0, // aligned with direction; should collapse to 0
            1.0, -1.0, // orthogonal to direction; should be preserved
            2.0, 0.0, // partial projection
        ];
        orthogonalise_weight_in_place(&mut weight, 3, 2, &direction).expect("ortho");
        // Row 0: was on the direction, projection=sqrt(2), W - proj*d = 0
        assert!(weight[0].abs() < 1e-5);
        assert!(weight[1].abs() < 1e-5);
        // Row 1: orthogonal, untouched.
        assert!((weight[2] - 1.0).abs() < 1e-5);
        assert!((weight[3] - (-1.0)).abs() < 1e-5);
        // Row 2: [2, 0], projection on [0.707,0.707] = 1.414, W' = [2,0] -
        //   1.414 * [0.707,0.707] = [1, -1].
        assert!((weight[4] - 1.0).abs() < 1e-5);
        assert!((weight[5] - (-1.0)).abs() < 1e-5);

        assert!(weight_is_orthogonal_to(&weight, 3, 2, &direction, 1e-5));
    }

    #[test]
    fn orthogonalise_rejects_dimension_mismatch() {
        let mut weight = vec![1.0_f32; 6];
        let err = orthogonalise_weight_in_place(&mut weight, 3, 2, &[1.0])
            .expect_err("dim mismatch");
        assert!(format!("{err}").contains("direction length"));
    }

    #[test]
    fn config_validate_requires_license_tag_and_signature() {
        let temp = std::env::temp_dir();
        // Use the temp dir itself for existence checks; we only care
        // that validate() catches the license / signature fields.
        let cfg = AbliterationConfig {
            base_model_path: temp.clone(),
            refusal_direction_path: temp.clone(),
            out_model_path: temp.join("out.gguf"),
            license_tag: "".to_string(),
            provenance_note: "test".to_string(),
            operator_signature: "operator-test".to_string(),
        };
        let err = cfg.validate().expect_err("empty license rejected");
        assert!(format!("{err}").contains("license_tag"));

        let cfg2 = AbliterationConfig {
            base_model_path: temp.clone(),
            refusal_direction_path: temp.clone(),
            out_model_path: temp.join("out.gguf"),
            license_tag: "Permissive".to_string(),
            provenance_note: "test".to_string(),
            operator_signature: "".to_string(),
        };
        let err = cfg2.validate().expect_err("empty signature rejected");
        assert!(format!("{err}").contains("signature"));
    }

    #[test]
    fn run_returns_native_toolchain_unavailable_until_io_is_wired() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let base = temp_dir.path().join("base.gguf");
        let direction_path = temp_dir.path().join("dir.json");
        let out = temp_dir.path().join("out.gguf");
        std::fs::write(&base, b"fake gguf bytes").unwrap();
        std::fs::write(
            &direction_path,
            serde_json::to_string(&RefusalDirectionFile {
                layer: 14,
                values: vec![0.707, 0.707],
                source_model_sha256: Some("ff".repeat(32)),
            })
            .unwrap(),
        )
        .unwrap();

        let cfg = AbliterationConfig {
            base_model_path: base,
            refusal_direction_path: direction_path,
            out_model_path: out,
            license_tag: "Permissive".to_string(),
            provenance_note: "test run".to_string(),
            operator_signature: "operator-test".to_string(),
        };
        let err = run_abliteration_offline(&cfg).expect_err("native toolchain unavailable");
        assert!(matches!(
            err,
            AbliterationError::NativeToolchainUnavailable(_)
        ));
    }
}
