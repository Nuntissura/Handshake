//! MT-115 — INF-9 LoRA-for-SSM validation surface tests.
//!
//! Exercises `candle::ssm_lora::*` validators per the MT-115 red_team
//! minimum controls:
//! - "Architecture tag validation enforced": validate_target_modules
//!   returns Err for foreign architecture / unrecognised module names.
//! - "Per-architecture target-module name list explicit (Mamba2 vs
//!   RWKV)": exact-name + per-layer-indexed forms accepted per
//!   architecture.
//!
//! The actual mount/unmount weight-application path is deferred to a
//! follow-on MT (per the MT-115 implementation_record). Tests here cover
//! the surface that MT-115 lands today.

use handshake_core::model_runtime::{
    candle::ssm_lora::{
        ssm_lora_mount_deferred_error, validate_target_modules_for_architecture,
        SSMArchitectureTag, MAMBA2_VALID_TARGET_MODULES, RWKV_VALID_TARGET_MODULES,
        SSM_LORA_MOUNT_DEFERRED_MARKER, SSM_LORA_PEFT_FORMULA,
    },
    ModelRuntimeError,
};

#[test]
fn mamba2_accepts_exact_target_module_names() {
    let modules = MAMBA2_VALID_TARGET_MODULES
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    validate_target_modules_for_architecture(SSMArchitectureTag::Mamba2, &modules)
        .expect("Mamba2 must accept its own valid target_modules verbatim");
}

#[test]
fn mamba2_rejects_rwkv_target_module_names() {
    let rwkv_modules = vec!["time_mix.key.weight".to_string()];
    let err = validate_target_modules_for_architecture(SSMArchitectureTag::Mamba2, &rwkv_modules)
        .expect_err("Mamba2 must reject RWKV-style target_modules");
    assert!(
        matches!(err, ModelRuntimeError::LoraStackError(ref message) if message.contains("not valid for SSM architecture mamba2")),
        "expected LoraStackError mentioning mamba2; got {err:?}"
    );
}

#[test]
fn mamba2_rejects_attention_target_module_names() {
    // Transformer-style attention/MLP names should be rejected — Mamba2
    // has no attention surface.
    let attn_modules = vec!["q_proj".to_string(), "k_proj".to_string()];
    let err = validate_target_modules_for_architecture(SSMArchitectureTag::Mamba2, &attn_modules)
        .expect_err("Mamba2 must reject transformer attention names");
    assert!(matches!(err, ModelRuntimeError::LoraStackError(_)));
}

#[test]
fn mamba2_rejects_empty_target_modules() {
    let empty: Vec<String> = Vec::new();
    let err = validate_target_modules_for_architecture(SSMArchitectureTag::Mamba2, &empty)
        .expect_err("empty target_modules must be rejected");
    assert!(
        matches!(err, ModelRuntimeError::LoraStackError(ref message) if message.contains("must not be empty"))
    );
}

#[test]
fn mamba2_rejects_whitespace_only_module_name() {
    let modules = vec!["   ".to_string()];
    let err = validate_target_modules_for_architecture(SSMArchitectureTag::Mamba2, &modules)
        .expect_err("whitespace-only module must be rejected");
    assert!(matches!(err, ModelRuntimeError::LoraStackError(_)));
}

#[test]
fn rwkv_v5_accepts_exact_target_module_names() {
    let modules = RWKV_VALID_TARGET_MODULES
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    validate_target_modules_for_architecture(SSMArchitectureTag::RwkvV5, &modules)
        .expect("RWKV v5 must accept its own valid target_modules verbatim");
}

#[test]
fn rwkv_v5_accepts_per_layer_indexed_form() {
    let modules = vec![
        "time_mix.0.key.weight".to_string(),
        "time_mix.5.value.weight".to_string(),
        "channel_mix.12.receptance.weight".to_string(),
    ];
    validate_target_modules_for_architecture(SSMArchitectureTag::RwkvV5, &modules)
        .expect("RWKV v5 must accept the per-layer-indexed form module.{N}.weight");
}

#[test]
fn rwkv_v5_rejects_mamba2_target_module_names() {
    let mamba_modules = vec!["in_proj".to_string()];
    let err = validate_target_modules_for_architecture(SSMArchitectureTag::RwkvV5, &mamba_modules)
        .expect_err("RWKV v5 must reject Mamba2-style target_modules");
    assert!(
        matches!(err, ModelRuntimeError::LoraStackError(ref message) if message.contains("not valid for SSM architecture rwkv_v5")),
        "expected LoraStackError mentioning rwkv_v5; got {err:?}"
    );
}

#[test]
fn rwkv_v5_rejects_non_numeric_layer_index() {
    let modules = vec!["time_mix.first.key.weight".to_string()];
    let err = validate_target_modules_for_architecture(SSMArchitectureTag::RwkvV5, &modules)
        .expect_err("non-numeric layer index must be rejected");
    assert!(matches!(err, ModelRuntimeError::LoraStackError(_)));
}

#[test]
fn rwkv_v6_and_v7_share_rwkv_module_list() {
    // The contract narrative ties v5/v6/v7 to the same time-mix /
    // channel-mix Linear surface; v6 reorganises decay tensors and v7
    // adjusts decay-vector eval, but the LoRA-targetable Linears stay
    // identical. Verify both variants accept the v5 module list.
    let modules = RWKV_VALID_TARGET_MODULES
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    validate_target_modules_for_architecture(SSMArchitectureTag::RwkvV6, &modules)
        .expect("RWKV v6 must accept the shared RWKV target_modules");
    validate_target_modules_for_architecture(SSMArchitectureTag::RwkvV7, &modules)
        .expect("RWKV v7 must accept the shared RWKV target_modules");
}

#[test]
fn architecture_tag_serializes_to_snake_case_strings() {
    assert_eq!(SSMArchitectureTag::Mamba2.as_str(), "mamba2");
    assert_eq!(SSMArchitectureTag::RwkvV5.as_str(), "rwkv_v5");
    assert_eq!(SSMArchitectureTag::RwkvV6.as_str(), "rwkv_v6");
    assert_eq!(SSMArchitectureTag::RwkvV7.as_str(), "rwkv_v7");
}

#[test]
fn peft_formula_is_documented_and_uniform_with_transformer_path() {
    // Single-source-of-truth doc string. The follow-on weight-
    // application MT and the operator-facing manual both quote this.
    assert!(SSM_LORA_PEFT_FORMULA.contains("y = base.forward(x)"));
    assert!(SSM_LORA_PEFT_FORMULA.contains("scaling = lora_alpha / rank"));
}

#[test]
fn mount_deferral_marker_returns_typed_capability_error() {
    let err = ssm_lora_mount_deferred_error(SSMArchitectureTag::Mamba2);
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, ref adapter }
            if capability == SSM_LORA_MOUNT_DEFERRED_MARKER && adapter == "candle_mamba2"
        ),
        "expected deferral marker with adapter=candle_mamba2; got {err:?}"
    );
}

#[test]
fn mount_deferral_marker_carries_per_architecture_adapter_tag() {
    for arch in [
        SSMArchitectureTag::Mamba2,
        SSMArchitectureTag::RwkvV5,
        SSMArchitectureTag::RwkvV6,
        SSMArchitectureTag::RwkvV7,
    ] {
        let err = ssm_lora_mount_deferred_error(arch);
        match err {
            ModelRuntimeError::CapabilityNotSupported {
                capability,
                adapter,
            } => {
                assert_eq!(capability, SSM_LORA_MOUNT_DEFERRED_MARKER);
                assert_eq!(adapter, format!("candle_{}", arch.as_str()));
            }
            other => panic!("expected CapabilityNotSupported; got {other:?}"),
        }
    }
}
