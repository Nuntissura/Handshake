use std::collections::BTreeMap;

use handshake_core::model_runtime::{
    candle::CandleSteeringHooks, CaptureSpec, HookPoint, LayerIndex, ModelId, ModelRuntimeError,
    SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorValues,
};

fn manual_zero_vector(layer: LayerIndex, width: usize) -> SteeringVector {
    SteeringVector::try_new(
        None,
        "zero identity",
        layer,
        HookPoint::ResidStream,
        SteeringVectorValues::try_new(vec![0.0; width], 1.0).expect("zero vector values"),
        "zero vector should not alter the residual stream",
        Some(SteeringProvenance::Manual {
            author: "MT-082-test".to_string(),
            notes: "identity gate".to_string(),
        }),
    )
    .expect("valid steering vector")
}

fn manual_vector(
    layer: LayerIndex,
    values: Vec<f32>,
    intensity: f32,
    hook_point: HookPoint,
) -> SteeringVector {
    SteeringVector::try_new(
        None,
        "manual steering",
        layer,
        hook_point,
        SteeringVectorValues::try_new(values, intensity).expect("vector values"),
        "manual vector for Candle hook tests",
        Some(SteeringProvenance::Manual {
            author: "MT-082-test".to_string(),
            notes: "forward harness gate".to_string(),
        }),
    )
    .expect("valid steering vector")
}

#[tokio::test]
async fn candle_hooks_capture_resid_stream_and_reject_unsupported_hook_points() {
    let layer = LayerIndex::new(2);

    // MT-082: bare hooks WITHOUT scaffold opt-in must fail closed on capture —
    // they have no model forward and must not return synthetic activations
    // that masquerade as real model state (this is the production path for
    // the adapter fallback / default no-feature builds).
    let production_hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4);
    let fail_closed = production_hooks
        .capture(CaptureSpec {
            prompts: vec!["capture prompt".to_string()],
            layers: vec![layer],
            hook_point: HookPoint::ResidStream,
        })
        .await
        .expect_err("bare hooks must not synthesize real activations");
    assert!(
        matches!(
            fail_closed,
            ModelRuntimeError::SteeringHookError(ref m)
                if m.contains("cannot capture real residual activations")
        ),
        "{fail_closed}"
    );

    // Explicit test scaffolding opts in and gets deterministic synthetic rows.
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 4).with_scaffold_capture();
    let captured = hooks
        .capture(CaptureSpec {
            prompts: vec!["capture prompt".to_string()],
            layers: vec![layer],
            hook_point: HookPoint::ResidStream,
        })
        .await
        .expect("scaffold-enabled residual stream capture is supported");

    let layer_rows = captured
        .activations
        .get(&layer)
        .expect("capture includes requested layer");
    assert_eq!(layer_rows.len(), 1);
    assert_eq!(layer_rows[0].len(), 4);
    assert_eq!(captured.tokens_seen, 1);

    for hook_point in [HookPoint::MlpOut, HookPoint::AttnOut] {
        let error = hooks
            .capture(CaptureSpec {
                prompts: vec!["capture prompt".to_string()],
                layers: vec![layer],
                hook_point,
            })
            .await
            .expect_err("non-residual hook points are explicit unsupported capabilities");

        assert!(
            matches!(
                error,
                ModelRuntimeError::CapabilityNotSupported {
                    ref capability,
                    ref adapter
                } if capability.contains(match hook_point {
                    HookPoint::MlpOut => "mlp_out",
                    HookPoint::AttnOut => "attn_out",
                    HookPoint::ResidStream => "resid_stream",
                }) && adapter.contains("candle")
            ),
            "{error}"
        );
    }
}

#[tokio::test]
async fn candle_hook_registry_registers_toggles_and_isolates_vectors_per_model() {
    let layer = LayerIndex::new(1);
    let hooks_a = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 3);
    let hooks_b = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 3);

    let vector_id = hooks_a
        .register_vector(manual_zero_vector(layer, 3))
        .await
        .expect("register vector");

    let a_vectors = hooks_a.list_vectors();
    assert_eq!(a_vectors.len(), 1);
    assert_eq!(a_vectors[0].id, vector_id);
    assert_eq!(a_vectors[0].layer, layer);
    assert_eq!(
        hooks_b.try_list_vectors().expect("list hooks b"),
        Vec::new()
    );

    hooks_a
        .set_active(vec![vector_id])
        .await
        .expect("activate vector");
    assert_eq!(
        hooks_a.try_active_vector_ids().expect("active a"),
        vec![vector_id]
    );
    assert_eq!(
        hooks_b.try_active_vector_ids().expect("active b"),
        Vec::new()
    );

    hooks_a
        .unregister(vector_id)
        .await
        .expect("unregister vector");
    assert_eq!(
        hooks_a.try_list_vectors().expect("list after unregister"),
        Vec::new()
    );
    assert_eq!(
        hooks_a
            .try_active_vector_ids()
            .expect("active after unregister"),
        Vec::new()
    );
}

#[test]
fn candle_forward_harness_applies_zero_identity_and_nonzero_vector_shift() {
    let layer = LayerIndex::new(4);
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 3);
    let base = vec![0.25, -0.5, 1.5];
    let mut layers = BTreeMap::new();
    layers.insert(layer, vec![base.clone()]);
    layers.insert(LayerIndex::new(5), vec![vec![9.0, 9.0, 9.0]]);

    let zero_id = futures::executor::block_on(hooks.register_vector(manual_zero_vector(layer, 3)))
        .expect("register zero");
    futures::executor::block_on(hooks.set_active(vec![zero_id])).expect("activate zero");
    assert_eq!(
        hooks
            .run_resid_stream_forward_harness(layers.clone(), &[layer], &[])
            .expect("zero vector forward harness")
            .activations[&layer][0],
        base
    );
    assert_eq!(
        hooks
            .run_resid_stream_forward_harness(layers.clone(), &[layer], &[])
            .expect("multi-layer token count")
            .tokens_seen,
        1
    );

    let shift_id = futures::executor::block_on(hooks.register_vector(manual_vector(
        layer,
        vec![0.5, 0.0, -0.25],
        2.0,
        HookPoint::ResidStream,
    )))
    .expect("register shift");
    assert_eq!(
        hooks
            .run_resid_stream_forward_harness(layers, &[layer], &[shift_id])
            .expect("nonzero vector forward harness")
            .activations[&layer][0],
        vec![1.25, -0.5, 1.0]
    );
}

#[test]
fn candle_hooks_fail_closed_for_non_residual_injection_and_unknown_overrides() {
    let layer = LayerIndex::new(1);
    let hooks = CandleSteeringHooks::new_for_model(ModelId::new_v7(), 3);

    let register_error = futures::executor::block_on(hooks.register_vector(manual_vector(
        layer,
        vec![1.0, 0.0, 0.0],
        1.0,
        HookPoint::AttnOut,
    )))
    .expect_err("registering unsupported hook point fails closed");
    assert!(
        matches!(
            register_error,
            ModelRuntimeError::CapabilityNotSupported {
                ref capability,
                ref adapter
            } if capability.contains("attn_out") && adapter.contains("candle")
        ),
        "{register_error}"
    );

    let injection_error = hooks
        .apply_registered_vectors(layer, HookPoint::MlpOut, vec![0.0, 0.0, 0.0])
        .expect_err("injecting unsupported hook point fails closed");
    assert!(
        matches!(
            injection_error,
            ModelRuntimeError::CapabilityNotSupported {
                ref capability,
                ref adapter
            } if capability.contains("mlp_out") && adapter.contains("candle")
        ),
        "{injection_error}"
    );

    let unknown_error = hooks
        .snapshot_vectors_for_request(&[SteeringVectorId::new_v7()])
        .expect_err("unknown request override fails closed before generation");
    assert!(unknown_error
        .to_string()
        .contains("unknown steering vector"));
}

#[test]
fn candle_runtime_adapter_is_wired_to_candle_steering_hooks_not_scaffold_error() {
    let adapter_source = include_str!("../src/model_runtime/candle/adapter.rs");

    assert!(adapter_source.contains("CandleSteeringHooks"));
    assert!(!adapter_source.contains("candle_steering_hooks"));
}
