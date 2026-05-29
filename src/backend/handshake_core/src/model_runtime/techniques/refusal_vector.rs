//! MT-100: INF-4 Refusal Vector extraction + runtime ablation API
//!
//! Implements Arditi et al. 2024 ("Refusal in LLMs is Mediated by a Single
//! Direction") at the runtime layer, on top of the activation steering plumbing
//! from MT-082..MT-083 + MT-096:
//!
//! - [`extract_refusal_direction`] captures residual-stream activations for a
//!   harmful-instruction pool and a harmless-instruction pool and returns the
//!   per-layer `mean(harmful) - mean(harmless)` direction, normalised to unit
//!   length. Operators pass in their own pools (no assistant-side curation per
//!   GLOBAL-PRODUCTION-005..009).
//! - [`ablate_at_inference`] registers one such direction as a [`SteeringVector`]
//!   with [`SteeringProvenance::Contrastive`] carrying
//!   [`ContrastiveTechnique::RefusalVector`]. The intensity is fixed at
//!   [`REFUSAL_ABLATION_INTENSITY`] = -1.0, which the runtime treats as a
//!   marker to *orthogonalise* the residual against the direction
//!   (`new_resid = resid - (resid · dir) * dir`) rather than additively steer.
//!   Unregistering the vector restores base behaviour - the ablation is
//!   reversible at inference time. INF-6 (offline abliteration) is the
//!   weight-orthogonalising cousin and is NOT reversible.

use std::collections::BTreeMap;

use crate::model_runtime::{
    techniques::activation_steering::{capture, register_steering_vector},
    ContrastiveTechnique, HookPoint, LayerIndex, ModelId, ModelRuntime, ModelRuntimeError,
    SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorValues,
};

/// Intensity value that flags a steering vector as a *refusal ablation* rather
/// than an additive steering edit. The runtime SteeringHookOps consumer reads
/// `provenance.technique == RefusalVector` together with this intensity to
/// switch into the orthogonalisation code path. Held as `-1.0` so that any
/// adapter which has not yet wired the ablation branch falls back to a
/// well-defined "subtract the direction" behaviour rather than amplifying the
/// refusal direction.
pub const REFUSAL_ABLATION_INTENSITY: f32 = -1.0;

/// A refusal direction at a specific transformer layer. `values` is unit-length
/// (L2 norm = 1) so the runtime ablation step is dimensionally a clean
/// projection without re-normalisation.
#[derive(Clone, Debug, PartialEq)]
pub struct RefusalDirection {
    pub layer: LayerIndex,
    pub values: Vec<f32>,
}

/// Captures activations for both pools and returns the per-layer refusal
/// direction. Order of `layers` is preserved in the output.
///
/// The contract requires reuse of the activation_steering capture path (no
/// parallel hook layer); this function calls [`capture`] twice and combines
/// the per-pool means at each layer.
pub async fn extract_refusal_direction(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    harmful_prompts: Vec<String>,
    harmless_prompts: Vec<String>,
    layers: Vec<LayerIndex>,
) -> Result<Vec<RefusalDirection>, ModelRuntimeError> {
    if harmful_prompts.is_empty() {
        return Err(ModelRuntimeError::SteeringHookError(
            "refusal direction extraction requires at least one harmful prompt".to_string(),
        ));
    }
    if harmless_prompts.is_empty() {
        return Err(ModelRuntimeError::SteeringHookError(
            "refusal direction extraction requires at least one harmless prompt".to_string(),
        ));
    }
    if layers.is_empty() {
        return Err(ModelRuntimeError::SteeringHookError(
            "refusal direction extraction requires at least one layer".to_string(),
        ));
    }

    let harmful = capture(runtime, model_id, harmful_prompts, layers.clone()).await?;
    let harmless = capture(runtime, model_id, harmless_prompts, layers.clone()).await?;

    let mut directions = Vec::with_capacity(layers.len());
    for layer in &layers {
        let harmful_mean = mean_activations(&harmful.activations, *layer)?;
        let harmless_mean = mean_activations(&harmless.activations, *layer)?;
        if harmful_mean.len() != harmless_mean.len() {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "refusal direction extraction: harmful/harmless activation widths differ at layer {} ({} vs {})",
                layer.as_u32(),
                harmful_mean.len(),
                harmless_mean.len(),
            )));
        }
        let raw: Vec<f32> = harmful_mean
            .into_iter()
            .zip(harmless_mean)
            .map(|(h, n)| h - n)
            .collect();
        let normalised = unit_normalise(&raw).ok_or_else(|| {
            ModelRuntimeError::SteeringHookError(format!(
                "refusal direction at layer {} has zero L2 norm; harmful and harmless pools are indistinguishable at this layer",
                layer.as_u32(),
            ))
        })?;
        directions.push(RefusalDirection {
            layer: *layer,
            values: normalised,
        });
    }
    Ok(directions)
}

/// Registers a refusal direction as a [`SteeringVector`] with
/// [`ContrastiveTechnique::RefusalVector`] provenance and intensity
/// [`REFUSAL_ABLATION_INTENSITY`]. The runtime treats this combination as the
/// ablation orthogonalisation request. Reversible via the existing
/// `unregister` and `set_active` steering APIs.
///
/// The `harmful_prompts` / `harmless_prompts` arguments are stored verbatim
/// in the vector provenance for audit; per GLOBAL-PRODUCTION-005..009 the
/// caller's wording is preserved without UI- or library-side sanitisation.
#[allow(clippy::too_many_arguments)]
pub async fn ablate_at_inference(
    runtime: &dyn ModelRuntime,
    model_id: ModelId,
    name: impl Into<String>,
    description: impl Into<String>,
    direction: RefusalDirection,
    harmful_prompts: Vec<String>,
    harmless_prompts: Vec<String>,
) -> Result<SteeringVectorId, ModelRuntimeError> {
    let values = SteeringVectorValues::try_new(direction.values, REFUSAL_ABLATION_INTENSITY)?;
    let provenance = SteeringProvenance::Contrastive {
        positive_prompts: harmful_prompts,
        negative_prompts: harmless_prompts,
        technique: ContrastiveTechnique::RefusalVector,
    };
    let vector = SteeringVector::try_new(
        None,
        name,
        direction.layer,
        HookPoint::ResidStream,
        values,
        description,
        Some(provenance),
    )?;
    register_steering_vector(runtime, model_id, vector).await
}

fn mean_activations(
    rows_by_layer: &BTreeMap<LayerIndex, Vec<Vec<f32>>>,
    layer: LayerIndex,
) -> Result<Vec<f32>, ModelRuntimeError> {
    let rows = rows_by_layer.get(&layer).ok_or_else(|| {
        ModelRuntimeError::SteeringHookError(format!(
            "refusal direction: capture did not return activations for layer {}",
            layer.as_u32()
        ))
    })?;
    if rows.is_empty() {
        return Err(ModelRuntimeError::SteeringHookError(format!(
            "refusal direction: capture returned zero activations at layer {}",
            layer.as_u32()
        )));
    }
    let width = rows[0].len();
    if width == 0 {
        return Err(ModelRuntimeError::SteeringHookError(format!(
            "refusal direction: zero-width activations at layer {}",
            layer.as_u32()
        )));
    }
    let mut sum = vec![0.0_f32; width];
    for row in rows {
        if row.len() != width {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "refusal direction: activation row width {} != expected {width} at layer {}",
                row.len(),
                layer.as_u32(),
            )));
        }
        if row.iter().any(|value| !value.is_finite()) {
            return Err(ModelRuntimeError::SteeringHookError(format!(
                "refusal direction: non-finite activation at layer {}",
                layer.as_u32()
            )));
        }
        for (acc, value) in sum.iter_mut().zip(row) {
            *acc += *value;
        }
    }
    let count = rows.len() as f32;
    for value in sum.iter_mut() {
        *value /= count;
    }
    Ok(sum)
}

fn unit_normalise(values: &[f32]) -> Option<Vec<f32>> {
    if values.is_empty() {
        return None;
    }
    let norm_sq: f32 = values.iter().map(|v| v * v).sum();
    let norm = norm_sq.sqrt();
    if !norm.is_finite() || norm == 0.0 {
        return None;
    }
    Some(values.iter().map(|v| v / norm).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_activations_returns_per_dim_mean() {
        let mut rows = BTreeMap::new();
        rows.insert(
            LayerIndex::new(7),
            vec![vec![1.0, 2.0, 3.0], vec![3.0, 4.0, 5.0]],
        );
        let mean = mean_activations(&rows, LayerIndex::new(7)).expect("mean");
        assert_eq!(mean, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn mean_activations_rejects_width_mismatch() {
        let mut rows = BTreeMap::new();
        rows.insert(
            LayerIndex::new(7),
            vec![vec![1.0, 2.0], vec![3.0, 4.0, 5.0]],
        );
        let err = mean_activations(&rows, LayerIndex::new(7)).expect_err("width mismatch");
        assert!(format!("{err:?}").contains("row width"), "{err:?}");
    }

    #[test]
    fn unit_normalise_produces_unit_length() {
        let normalised = unit_normalise(&[3.0, 4.0]).expect("normalise");
        let norm: f32 = normalised.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "norm={norm}");
    }

    #[test]
    fn unit_normalise_rejects_zero_vector() {
        assert!(unit_normalise(&[0.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn refusal_ablation_intensity_is_minus_one() {
        // The runtime ablation hook keys on (technique == RefusalVector,
        // intensity == REFUSAL_ABLATION_INTENSITY) to switch to the
        // orthogonalisation path; pin the constant so the contract cannot
        // drift silently.
        assert!((REFUSAL_ABLATION_INTENSITY - (-1.0)).abs() < f32::EPSILON);
    }
}
