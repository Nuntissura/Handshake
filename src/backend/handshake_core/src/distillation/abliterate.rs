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
//! unit-tested in-tree without I/O. The full safetensors round-trip and
//! optional ProcessOwnershipLedger spawn-time registration are
//! orchestrated by `run_abliteration_offline`, which delegates native
//! safetensors I/O to the Candle adapter boundary (operator decision:
//! option_c per WP-KERNEL-004 wp_validator_final_disposition; avoids the
//! llama-cpp-2 + LLVM/libclang dependency that blocked MT-074).
//!
//! Target modules per kabachuha/abliterate.cpp pattern and MT-106
//! implementation_notes: every Linear weight tensor whose name contains
//! `o_proj` (attention output projection) or `down_proj` (MLP down
//! projection). All other tensors are copied through unchanged. The
//! refusal direction must match the last-dim ("cols") of the weight; any
//! target tensor whose shape disagrees is rejected.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "candle-runtime-engine")]
use std::io::Read;

#[cfg(feature = "candle-runtime-engine")]
use chrono::Utc;
#[cfg(feature = "candle-runtime-engine")]
use sha2::{Digest, Sha256};

#[cfg(feature = "candle-runtime-engine")]
use crate::process_ledger::{
    record_spawn, LedgerBatcher, ProcessEngineKind, ProcessOwnershipRecordId, SpawnMeta,
};

/// Tool version recorded in `AbliterationProvenance.abliteration_tool_version`.
pub const ABLITERATION_TOOL_VERSION: &str = "handshake-abliterate-mt106-v1";

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
    /// Target weight tensor keys that were actually orthogonalised. Lets
    /// downstream callers (and the integration test) confirm the
    /// algorithm ran against the expected Linear modules rather than
    /// silently no-op-ing.
    #[serde(default)]
    pub orthogonalised_weight_keys: Vec<String>,
    /// Process ledger row id when a `LedgerBatcher` was provided to
    /// `run_abliteration_offline`; `None` when the caller (typically the
    /// CLI on a host without a running ledger writer) elected not to
    /// register a row.
    #[serde(default)]
    pub process_ledger_record_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum AbliterationError {
    #[error("abliteration config invalid: {0}")]
    InvalidConfig(String),
    #[error("abliteration refusal-direction file invalid: {0}")]
    InvalidDirection(String),
    #[error("abliteration safetensors I/O failed: {0}")]
    Io(String),
    #[error("abliteration weight transform failed: {0}")]
    WeightTransform(String),
    #[error("abliteration process-ledger registration failed: {0}")]
    LedgerRegistration(String),
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
/// unit tests without safetensors I/O.
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

/// Returns `true` when a safetensors tensor key names an attention
/// output projection (`o_proj`) or MLP down projection (`down_proj`)
/// Linear weight. These are the target modules per the kabachuha
/// abliteration recipe + MT-106 implementation_notes.
///
/// The match is intentionally permissive about the prefix (`model.layers.N.`
/// vs `transformer.h.N.`, etc.) but requires the tensor be a `.weight`,
/// not a `.bias`, so biases pass through unchanged.
pub fn is_abliteration_target_module(key: &str) -> bool {
    if !key.ends_with(".weight") {
        return false;
    }
    key.contains(".o_proj.") || key.contains(".down_proj.")
}

/// Orchestrator entrypoint. Validates the config, loads the refusal
/// direction file, optionally registers a ProcessOwnershipLedger row
/// with `engine_kind = AbliterationTool`, walks the tensors of the base
/// safetensors artifact, applies [`orthogonalise_weight_in_place`] to
/// every tensor accepted by [`is_abliteration_target_module`], and
/// writes the resulting tensor set to `out_model_path`.
///
/// This is the offline-tool entrypoint and is gated on the
/// `candle-runtime-engine` cargo feature. The bin/abliterate.rs binary
/// declares the same `required-features` so operators always invoke it
/// with the Candle backend wired in.
///
/// `ledger` is optional: production callers pass a live
/// `LedgerBatcher` so the abliteration job appears in the
/// ProcessOwnershipLedger; the CLI on a host without an attached
/// Postgres writer can pass `None`. The integration test exercises the
/// `Some` path so the engine_kind=AbliterationTool row registration is
/// actually written, not just declared.
#[cfg(feature = "candle-runtime-engine")]
pub fn run_abliteration_offline(
    config: &AbliterationConfig,
    ledger: Option<&LedgerBatcher>,
) -> Result<AbliterationProvenance, AbliterationError> {
    config.validate()?;

    // Refusal-direction load + parse.
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

    // Hashes go into provenance. Computed from raw file bytes so the
    // operator can audit them against the on-disk artifacts after the
    // run.
    let base_model_sha256 = sha256_of_file(&config.base_model_path)?;
    let refusal_direction_sha256 = sha256_of_bytes(raw_direction.as_bytes());

    // Optional spawn-time ProcessOwnershipLedger row. We register
    // BEFORE doing the heavy I/O so the row exists even if the
    // safetensors round-trip fails partway through — that matches the
    // "ProcessOwnershipLedger is the truth" invariant from MT-069.
    let process_ledger_record_id = if let Some(batcher) = ledger {
        let pid = std::process::id();
        let owner_wp = Some("WP-KERNEL-004".to_string());
        let mut meta = SpawnMeta::new(pid, ProcessEngineKind::AbliterationTool, "ABLITERATE_CLI");
        meta.owner_wp = owner_wp.clone();
        meta.wp_id = owner_wp;
        meta.mt_id = Some("MT-106".to_string());
        meta.model_artifact_sha256 = Some(base_model_sha256.clone());
        meta.metadata_blob = serde_json::json!({
            "tool": "abliterate",
            "tool_version": ABLITERATION_TOOL_VERSION,
            "base_model_path": config.base_model_path.display().to_string(),
            "out_model_path": config.out_model_path.display().to_string(),
            "refusal_direction_sha256": refusal_direction_sha256,
            "license_tag": config.license_tag,
        });
        let record_id = record_spawn(batcher, meta).map_err(|err| {
            AbliterationError::LedgerRegistration(format!(
                "record_spawn(engine_kind=AbliterationTool) failed: {err}"
            ))
        })?;
        Some(format_record_id(record_id))
    } else {
        None
    };

    let orthogonalised_weight_keys =
        crate::model_runtime::candle::run_abliteration_model_io(config, &direction.values)?;

    let provenance = AbliterationProvenance {
        base_model_sha256,
        refusal_direction_sha256,
        abliteration_tool_version: ABLITERATION_TOOL_VERSION.to_string(),
        abliterated_at_utc: Utc::now().to_rfc3339(),
        license_tag: config.license_tag.clone(),
        operator_signature: config.operator_signature.clone(),
        provenance_note: config.provenance_note.clone(),
        orthogonalised_weight_keys,
        process_ledger_record_id,
    };

    // Provenance also goes next to the output artifact so downstream
    // §4.8 content-review pipelines can read it without re-deriving the
    // hash chain. Path is `{out_model_path}.provenance.json`.
    let provenance_path = provenance_sidecar_path(&config.out_model_path);
    let provenance_bytes = serde_json::to_vec_pretty(&provenance)
        .map_err(|err| AbliterationError::Io(format!("serialize provenance failed: {err}")))?;
    std::fs::write(&provenance_path, &provenance_bytes).map_err(|err| {
        AbliterationError::Io(format!(
            "write provenance sidecar {} failed: {err}",
            provenance_path.display()
        ))
    })?;

    Ok(provenance)
}

/// Path of the JSON provenance sidecar written alongside the output
/// safetensors artifact. Exposed so the integration test (and any
/// downstream content-review reader) can locate it without
/// re-deriving the convention.
pub fn provenance_sidecar_path(out_model_path: &std::path::Path) -> PathBuf {
    let mut s = out_model_path.as_os_str().to_owned();
    s.push(".provenance.json");
    PathBuf::from(s)
}

#[cfg(feature = "candle-runtime-engine")]
fn format_record_id(record_id: ProcessOwnershipRecordId) -> String {
    record_id.as_uuid().to_string()
}

#[cfg(feature = "candle-runtime-engine")]
fn sha256_of_file(path: &std::path::Path) -> Result<String, AbliterationError> {
    let mut file = std::fs::File::open(path).map_err(|err| {
        AbliterationError::Io(format!("open {} for hashing failed: {err}", path.display()))
    })?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf).map_err(|err| {
            AbliterationError::Io(format!("read {} for hashing failed: {err}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(feature = "candle-runtime-engine")]
fn sha256_of_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
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
        let err =
            orthogonalise_weight_in_place(&mut weight, 3, 2, &[1.0]).expect_err("dim mismatch");
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
            out_model_path: temp.join("out.safetensors"),
            license_tag: "".to_string(),
            provenance_note: "test".to_string(),
            operator_signature: "operator-test".to_string(),
        };
        let err = cfg.validate().expect_err("empty license rejected");
        assert!(format!("{err}").contains("license_tag"));

        let cfg2 = AbliterationConfig {
            base_model_path: temp.clone(),
            refusal_direction_path: temp.clone(),
            out_model_path: temp.join("out.safetensors"),
            license_tag: "Permissive".to_string(),
            provenance_note: "test".to_string(),
            operator_signature: "".to_string(),
        };
        let err = cfg2.validate().expect_err("empty signature rejected");
        assert!(format!("{err}").contains("signature"));
    }

    #[test]
    fn target_module_classifier_matches_o_proj_and_down_proj_weights_only() {
        assert!(is_abliteration_target_module(
            "model.layers.0.self_attn.o_proj.weight"
        ));
        assert!(is_abliteration_target_module(
            "model.layers.7.mlp.down_proj.weight"
        ));
        assert!(is_abliteration_target_module(
            "transformer.h.0.attn.o_proj.weight"
        ));
        // Biases pass through unchanged: abliteration only touches Linear weights.
        assert!(!is_abliteration_target_module(
            "model.layers.0.self_attn.o_proj.bias"
        ));
        // Other Linear weights are unaffected by the abliteration pass.
        assert!(!is_abliteration_target_module(
            "model.layers.0.self_attn.q_proj.weight"
        ));
        assert!(!is_abliteration_target_module(
            "model.layers.0.mlp.up_proj.weight"
        ));
        // Non-weight tensors (norm scales, embeddings) pass through.
        assert!(!is_abliteration_target_module("model.embed_tokens.weight"));
        assert!(!is_abliteration_target_module(
            "model.layers.0.input_layernorm.weight"
        ));
    }

    #[test]
    fn provenance_sidecar_path_appends_suffix() {
        let out = std::path::PathBuf::from("/tmp/abl/out.safetensors");
        let prov = provenance_sidecar_path(&out);
        assert_eq!(
            prov,
            std::path::PathBuf::from("/tmp/abl/out.safetensors.provenance.json")
        );
    }
}
