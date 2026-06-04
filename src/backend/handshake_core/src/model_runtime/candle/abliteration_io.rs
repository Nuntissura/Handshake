#![cfg(feature = "candle-runtime-engine")]

use std::collections::HashMap;

use crate::distillation::abliterate::{
    is_abliteration_target_module, orthogonalise_weight_in_place, AbliterationConfig,
    AbliterationError,
};

pub fn run_abliteration_model_io(
    config: &AbliterationConfig,
    direction: &[f32],
) -> Result<Vec<String>, AbliterationError> {
    let device = candle_core::Device::Cpu;
    let input_tensors =
        candle_core::safetensors::load(&config.base_model_path, &device).map_err(|err| {
            AbliterationError::Io(format!(
                "Candle safetensors load({}) failed: {err}",
                config.base_model_path.display()
            ))
        })?;

    if input_tensors.is_empty() {
        return Err(AbliterationError::Io(format!(
            "base safetensors {} contains no tensors",
            config.base_model_path.display()
        )));
    }

    let mut output_tensors: HashMap<String, candle_core::Tensor> = HashMap::new();
    let mut orthogonalised_weight_keys: Vec<String> = Vec::new();

    for (name, tensor) in input_tensors.into_iter() {
        if !is_abliteration_target_module(&name) {
            output_tensors.insert(name, tensor);
            continue;
        }

        let transformed = orthogonalise_tensor(&name, &tensor, direction)?;
        orthogonalised_weight_keys.push(name.clone());
        output_tensors.insert(name, transformed);
    }

    orthogonalised_weight_keys.sort();

    // Refuse to write a no-op artifact: if zero target modules matched
    // it almost certainly means the operator pointed at the wrong file
    // or the refusal direction width is wrong. Better to fail loudly
    // than to ship a derived model that is byte-identical to the input.
    if orthogonalised_weight_keys.is_empty() {
        return Err(AbliterationError::WeightTransform(format!(
            "no target modules matched in {}; expected at least one tensor whose key contains \
             `.o_proj.` or `.down_proj.` and ends with `.weight`",
            config.base_model_path.display()
        )));
    }

    candle_core::safetensors::save(&output_tensors, &config.out_model_path).map_err(|err| {
        AbliterationError::Io(format!(
            "Candle safetensors save({}) failed: {err}",
            config.out_model_path.display()
        ))
    })?;

    Ok(orthogonalised_weight_keys)
}

fn orthogonalise_tensor(
    name: &str,
    tensor: &candle_core::Tensor,
    direction: &[f32],
) -> Result<candle_core::Tensor, AbliterationError> {
    let shape = tensor.shape();
    let dims = shape.dims();
    if dims.len() != 2 {
        return Err(AbliterationError::WeightTransform(format!(
            "target module {name} is not a 2-D Linear weight (shape={dims:?}); abliteration \
             only orthogonalises Linear-layer weights",
        )));
    }
    let rows = dims[0];
    let cols = dims[1];
    if cols != direction.len() {
        return Err(AbliterationError::WeightTransform(format!(
            "target module {name} has cols={cols} but refusal direction has length \
             {}; cannot orthogonalise",
            direction.len()
        )));
    }
    if tensor.dtype() != candle_core::DType::F32 {
        return Err(AbliterationError::WeightTransform(format!(
            "target module {name} dtype={:?} is not F32; cast to F32 before abliterating",
            tensor.dtype()
        )));
    }
    let cpu_tensor = tensor.to_device(&candle_core::Device::Cpu).map_err(|err| {
        AbliterationError::WeightTransform(format!("tensor {name} to_device(CPU) failed: {err}"))
    })?;
    let mut data: Vec<f32> = cpu_tensor
        .flatten_all()
        .and_then(|t| t.to_vec1::<f32>())
        .map_err(|err| {
            AbliterationError::WeightTransform(format!(
                "tensor {name} flatten/to_vec1 failed: {err}"
            ))
        })?;

    orthogonalise_weight_in_place(&mut data, rows, cols, direction)?;

    candle_core::Tensor::from_vec(data, (rows, cols), &candle_core::Device::Cpu).map_err(|err| {
        AbliterationError::WeightTransform(format!(
            "rebuilding tensor {name} from orthogonalised data failed: {err}"
        ))
    })
}
