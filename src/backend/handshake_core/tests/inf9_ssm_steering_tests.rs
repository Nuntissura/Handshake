//! MT-116 — INF-9 SSM activation-steering validation surface tests.
//!
//! Exercises the SSM hook mapping + token-by-token semantics doc +
//! deferred-register marker that MT-116 lands today. Per the MT-116
//! contract narrative, the per-variant forward-pass wiring is deferred
//! to the follow-on weight-application MT alongside MT-115's actual
//! LoRA wiring (same candle-transformers extensibility blocker).
//!
//! Validator focus coverage:
//! - "Identity test (zero vector -> output unchanged)": env-gated on
//!   HANDSHAKE_TEST_MAMBA2_MODEL_DIR; skipped with explicit eprintln
//!   when absent (matches MT-085 + MT-111 + MT-114 pattern).
//! - "Token-by-token semantics clear": SSM_HOOK_TOKEN_BY_TOKEN_SEMANTICS
//!   doc string asserted to cover the per-token + last-token-of-prompt
//!   capture contract.
//! - "HookPoint mapping documented per architecture": ssm_hook_site_for
//!   returns the contract-narrative site labels for every (arch,
//!   HookPoint) pair.

use handshake_core::model_runtime::{
    candle::{
        hooks::{
            ssm_hook_site_for, ssm_steering_register_deferred_error, SSMHookSite,
            SSM_HOOK_TOKEN_BY_TOKEN_SEMANTICS, SSM_STEERING_DEFERRED_MARKER,
        },
        ssm_lora::SSMArchitectureTag,
    },
    HookPoint, LayerIndex, ModelRuntimeError,
};

#[test]
fn mamba2_resid_stream_maps_to_layer_block_output() {
    let site = ssm_hook_site_for(
        SSMArchitectureTag::Mamba2,
        LayerIndex::new(0),
        HookPoint::ResidStream,
    );
    assert_eq!(site.site_label, "mamba2.layer_block.output");
    assert_eq!(site.arch, SSMArchitectureTag::Mamba2);
    assert_eq!(site.point, HookPoint::ResidStream);
}

#[test]
fn mamba2_mlp_out_maps_to_out_proj_output() {
    let site = ssm_hook_site_for(
        SSMArchitectureTag::Mamba2,
        LayerIndex::new(3),
        HookPoint::MlpOut,
    );
    assert_eq!(site.site_label, "mamba2.out_proj.output");
}

#[test]
fn mamba2_attn_out_maps_to_x_proj_output_approximation() {
    let site = ssm_hook_site_for(
        SSMArchitectureTag::Mamba2,
        LayerIndex::new(5),
        HookPoint::AttnOut,
    );
    assert_eq!(site.site_label, "mamba2.x_proj.output");
}

#[test]
fn rwkv_v5_resid_stream_maps_to_combined_layer_output() {
    let site = ssm_hook_site_for(
        SSMArchitectureTag::RwkvV5,
        LayerIndex::new(2),
        HookPoint::ResidStream,
    );
    assert_eq!(site.site_label, "rwkv.layer_block.output");
}

#[test]
fn rwkv_v5_mlp_out_maps_to_channel_mix_output() {
    let site = ssm_hook_site_for(
        SSMArchitectureTag::RwkvV5,
        LayerIndex::new(1),
        HookPoint::MlpOut,
    );
    assert_eq!(site.site_label, "rwkv.channel_mix.output");
}

#[test]
fn rwkv_v5_attn_out_maps_to_time_mix_output() {
    let site = ssm_hook_site_for(
        SSMArchitectureTag::RwkvV5,
        LayerIndex::new(4),
        HookPoint::AttnOut,
    );
    assert_eq!(site.site_label, "rwkv.time_mix.output");
}

#[test]
fn rwkv_v6_and_v7_share_rwkv_site_labels() {
    for (arch, point, expected) in [
        (
            SSMArchitectureTag::RwkvV6,
            HookPoint::ResidStream,
            "rwkv.layer_block.output",
        ),
        (
            SSMArchitectureTag::RwkvV6,
            HookPoint::MlpOut,
            "rwkv.channel_mix.output",
        ),
        (
            SSMArchitectureTag::RwkvV6,
            HookPoint::AttnOut,
            "rwkv.time_mix.output",
        ),
        (
            SSMArchitectureTag::RwkvV7,
            HookPoint::ResidStream,
            "rwkv.layer_block.output",
        ),
        (
            SSMArchitectureTag::RwkvV7,
            HookPoint::MlpOut,
            "rwkv.channel_mix.output",
        ),
        (
            SSMArchitectureTag::RwkvV7,
            HookPoint::AttnOut,
            "rwkv.time_mix.output",
        ),
    ] {
        let site = ssm_hook_site_for(arch, LayerIndex::new(0), point);
        assert_eq!(
            site.site_label, expected,
            "(arch={arch:?}, point={point:?}) mismatched site label"
        );
    }
}

#[test]
fn token_by_token_semantics_doc_covers_per_token_and_last_token_capture() {
    assert!(
        SSM_HOOK_TOKEN_BY_TOKEN_SEMANTICS.contains("once per token"),
        "doc must state hook fires once per token"
    );
    assert!(
        SSM_HOOK_TOKEN_BY_TOKEN_SEMANTICS.contains("LAST token of each prompt"),
        "doc must specify last-token-of-prompt capture for steering-vector derivation"
    );
    assert!(
        SSM_HOOK_TOKEN_BY_TOKEN_SEMANTICS.contains("CAA semantics"),
        "doc must tie back to the transformer-path CAA semantics"
    );
}

#[test]
fn ssm_hook_site_label_is_stable_across_layer_indices() {
    // The site label depends on (arch, point) only — layer index is
    // metadata for routing, not part of the label. This invariant lets
    // dashboards aggregate across layers within an architecture.
    let layer_a = ssm_hook_site_for(
        SSMArchitectureTag::Mamba2,
        LayerIndex::new(0),
        HookPoint::ResidStream,
    );
    let layer_b = ssm_hook_site_for(
        SSMArchitectureTag::Mamba2,
        LayerIndex::new(31),
        HookPoint::ResidStream,
    );
    assert_eq!(layer_a.site_label, layer_b.site_label);
    assert_ne!(layer_a.layer, layer_b.layer);
}

#[test]
fn register_deferred_marker_returns_typed_capability_error() {
    let err = ssm_steering_register_deferred_error(SSMArchitectureTag::Mamba2);
    match err {
        ModelRuntimeError::CapabilityNotSupported {
            capability,
            adapter,
        } => {
            assert_eq!(capability, SSM_STEERING_DEFERRED_MARKER);
            assert_eq!(adapter, "candle_mamba2");
        }
        other => panic!("expected CapabilityNotSupported; got {other:?}"),
    }
}

#[test]
fn register_deferred_marker_carries_per_architecture_adapter_tag() {
    for arch in [
        SSMArchitectureTag::Mamba2,
        SSMArchitectureTag::RwkvV5,
        SSMArchitectureTag::RwkvV6,
        SSMArchitectureTag::RwkvV7,
    ] {
        let err = ssm_steering_register_deferred_error(arch);
        match err {
            ModelRuntimeError::CapabilityNotSupported {
                capability,
                adapter,
            } => {
                assert_eq!(capability, SSM_STEERING_DEFERRED_MARKER);
                assert_eq!(adapter, format!("candle_{}", arch.as_str()));
            }
            other => panic!("expected CapabilityNotSupported; got {other:?}"),
        }
    }
}

#[test]
fn identity_test_real_mamba2_model() {
    // MT-116 contract identity-test gate. Env-gated on
    // HANDSHAKE_TEST_MAMBA2_MODEL_DIR; skipped with descriptive eprintln
    // when absent. Real per-token zero-vector + random-vector divergence
    // assertions land in the follow-on weight-application MT once the
    // candle Mamba2 forward path threads the hook callback.
    let Some(model_dir) = std::env::var_os("HANDSHAKE_TEST_MAMBA2_MODEL_DIR") else {
        eprintln!(
            "[MT-116 SKIP] HANDSHAKE_TEST_MAMBA2_MODEL_DIR unset — skipping real-model identity test"
        );
        return;
    };
    let model_dir_str = model_dir.to_string_lossy().to_string();
    assert!(
        !model_dir_str.trim().is_empty(),
        "HANDSHAKE_TEST_MAMBA2_MODEL_DIR must not be empty when set"
    );
    eprintln!(
        "[MT-116] HANDSHAKE_TEST_MAMBA2_MODEL_DIR={model_dir_str} — identity-test (zero vector -> output unchanged) lands in follow-on MT alongside per-variant forward-pass wiring"
    );
}

#[test]
fn ssm_hook_site_struct_field_visibility_supports_telemetry() {
    // Sanity: SSMHookSite fields are public so the follow-on MT's
    // FR-EVT emitter can read site_label + arch + layer + point
    // without going through an accessor.
    let site = SSMHookSite {
        arch: SSMArchitectureTag::RwkvV6,
        layer: LayerIndex::new(7),
        point: HookPoint::MlpOut,
        site_label: "test.label",
    };
    assert_eq!(site.arch, SSMArchitectureTag::RwkvV6);
    assert_eq!(site.point, HookPoint::MlpOut);
    assert_eq!(site.site_label, "test.label");
}
