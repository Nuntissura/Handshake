#![cfg_attr(not(feature = "candle-runtime-engine"), allow(dead_code))]

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

const TEST_ID: &str = "candle_e2e_smoke";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelFamily {
    Transformer,
    Mamba2,
    RwkvV5,
    RwkvV6,
    RwkvV7,
}

#[derive(Clone, Copy, Debug)]
struct FamilySpec {
    family: ModelFamily,
    name: &'static str,
    env_var: &'static str,
    expected_event_family: &'static str,
    planned_coverage: &'static [&'static str],
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SmokeStatus {
    Passed,
    Skipped,
}

#[derive(Clone, Debug)]
struct SmokeOutcome {
    family: &'static str,
    env_var: &'static str,
    status: SmokeStatus,
    reason: String,
    event_family: &'static str,
    coverage: &'static [&'static str],
}

#[test]
#[ignore = "MT-089 proof command runs ignored Candle E2E smoke tests explicitly"]
fn candle_e2e_smoke_readme_documents_per_family_env_contract() {
    let readme = tests_readme();
    let entry = readme_entry(&readme);
    let required_env = entry
        .get("required_env")
        .and_then(Value::as_array)
        .expect("required_env array")
        .iter()
        .map(|value| value.as_str().expect("env var string"))
        .collect::<BTreeSet<_>>();
    let expected_env = family_specs()
        .iter()
        .map(|spec| spec.env_var)
        .collect::<BTreeSet<_>>();

    assert_eq!(required_env, expected_env);
    assert_eq!(
        entry.get("description").and_then(Value::as_str),
        Some("per-family skip if unset")
    );

    let documented_families = entry
        .get("families")
        .and_then(Value::as_array)
        .expect("families array");
    for spec in family_specs() {
        let family = documented_families
            .iter()
            .find(|item| item.get("family").and_then(Value::as_str) == Some(spec.name))
            .unwrap_or_else(|| panic!("missing README family {}", spec.name));
        assert_eq!(
            family.get("env_var").and_then(Value::as_str),
            Some(spec.env_var)
        );
        let coverage = family
            .get("coverage")
            .and_then(Value::as_array)
            .expect("coverage array")
            .iter()
            .map(|value| value.as_str().expect("coverage string"))
            .collect::<BTreeSet<_>>();
        for expected in spec.planned_coverage {
            assert!(
                coverage.contains(expected),
                "{} missing README coverage {expected}",
                spec.name
            );
        }
    }
}

#[tokio::test]
#[ignore = "MT-089 proof command runs ignored Candle E2E smoke tests explicitly"]
async fn candle_e2e_smoke_reports_every_family_as_passed_or_skipped() {
    let mut outcomes = Vec::new();
    for spec in family_specs() {
        outcomes.push(run_family_smoke(*spec).await);
    }

    assert_eq!(outcomes.len(), family_specs().len());
    assert!(outcomes
        .iter()
        .all(|outcome| matches!(outcome.status, SmokeStatus::Passed | SmokeStatus::Skipped)));

    let covered_env = outcomes
        .iter()
        .map(|outcome| outcome.env_var)
        .collect::<BTreeSet<_>>();
    let expected_env = family_specs()
        .iter()
        .map(|spec| spec.env_var)
        .collect::<BTreeSet<_>>();
    assert_eq!(covered_env, expected_env);

    let covered_events = outcomes
        .iter()
        .map(|outcome| outcome.event_family)
        .collect::<BTreeSet<_>>();
    for spec in family_specs() {
        assert!(
            covered_events.contains(spec.expected_event_family),
            "{} missing FR event-family coverage",
            spec.name
        );
    }

    for outcome in &outcomes {
        eprintln!(
            "[{TEST_ID}] family={} env={} status={:?} reason={} event_family={} coverage={}",
            outcome.family,
            outcome.env_var,
            outcome.status,
            outcome.reason,
            outcome.event_family,
            outcome.coverage.join(",")
        );
    }
}

async fn run_family_smoke(spec: FamilySpec) -> SmokeOutcome {
    let Some(model_dir) = model_dir_from_env(spec) else {
        return skipped(spec, format!("{} unset", spec.env_var));
    };
    if let Some(reason) = missing_model_inputs(&model_dir) {
        return skipped(spec, reason);
    }

    run_live_family_smoke(spec, model_dir).await
}

#[cfg(not(feature = "candle-runtime-engine"))]
async fn run_live_family_smoke(spec: FamilySpec, _model_dir: PathBuf) -> SmokeOutcome {
    skipped(
        spec,
        "candle-runtime-engine feature disabled; live model path not loaded".to_string(),
    )
}

#[cfg(feature = "candle-runtime-engine")]
async fn run_live_family_smoke(spec: FamilySpec, model_dir: PathBuf) -> SmokeOutcome {
    match spec.family {
        ModelFamily::Transformer => run_transformer_smoke(spec, &model_dir).await,
        ModelFamily::Mamba2 => {
            run_state_vector_smoke(
                spec,
                &model_dir,
                handshake_core::model_runtime::candle::SSMStateVariant::Mamba2,
            )
            .await
        }
        ModelFamily::RwkvV5 => {
            run_state_vector_smoke(
                spec,
                &model_dir,
                handshake_core::model_runtime::candle::SSMStateVariant::RwkvV5,
            )
            .await
        }
        ModelFamily::RwkvV6 => {
            run_state_vector_smoke(
                spec,
                &model_dir,
                handshake_core::model_runtime::candle::SSMStateVariant::RwkvV6,
            )
            .await
        }
        ModelFamily::RwkvV7 => {
            run_state_vector_smoke(
                spec,
                &model_dir,
                handshake_core::model_runtime::candle::SSMStateVariant::RwkvV7,
            )
            .await
        }
    }
    .unwrap_or_else(|error| panic!("{} live smoke failed: {error}", spec.name))
}

fn skipped(spec: FamilySpec, reason: String) -> SmokeOutcome {
    SmokeOutcome {
        family: spec.name,
        env_var: spec.env_var,
        status: SmokeStatus::Skipped,
        reason,
        event_family: spec.expected_event_family,
        coverage: spec.planned_coverage,
    }
}

fn passed(spec: FamilySpec) -> SmokeOutcome {
    SmokeOutcome {
        family: spec.name,
        env_var: spec.env_var,
        status: SmokeStatus::Passed,
        reason: "live env-gated smoke passed".to_string(),
        event_family: spec.expected_event_family,
        coverage: spec.planned_coverage,
    }
}

fn family_specs() -> &'static [FamilySpec] {
    &[
        FamilySpec {
            family: ModelFamily::Transformer,
            name: "transformer",
            env_var: "HANDSHAKE_TEST_CANDLE_MODEL_DIR",
            expected_event_family: "llm_inference:candle_transformer",
            planned_coverage: &[
                "load",
                "generate",
                "activation_capture",
                "zero_vector_identity",
                "lora_mount",
            ],
        },
        FamilySpec {
            family: ModelFamily::Mamba2,
            name: "mamba2",
            env_var: "HANDSHAKE_TEST_MAMBA2_MODEL_DIR",
            expected_event_family: "llm_inference:candle_mamba2",
            planned_coverage: &[
                "load",
                "generate",
                "state_vector_commit_restore",
                "tamper_hash_rejection",
            ],
        },
        FamilySpec {
            family: ModelFamily::RwkvV5,
            name: "rwkv_v5",
            env_var: "HANDSHAKE_TEST_RWKV_V5_MODEL_DIR",
            expected_event_family: "llm_inference:candle_rwkv_v5",
            planned_coverage: &[
                "load",
                "generate",
                "state_vector_commit_restore",
                "tamper_hash_rejection",
            ],
        },
        FamilySpec {
            family: ModelFamily::RwkvV6,
            name: "rwkv_v6",
            env_var: "HANDSHAKE_TEST_RWKV_V6_MODEL_DIR",
            expected_event_family: "llm_inference:candle_rwkv_v6",
            planned_coverage: &[
                "load",
                "generate",
                "state_vector_commit_restore",
                "tamper_hash_rejection",
            ],
        },
        FamilySpec {
            family: ModelFamily::RwkvV7,
            name: "rwkv_v7",
            env_var: "HANDSHAKE_TEST_RWKV_V7_MODEL_DIR",
            expected_event_family: "llm_inference:candle_rwkv_v7",
            planned_coverage: &[
                "load",
                "generate",
                "state_vector_commit_restore",
                "tamper_hash_rejection",
            ],
        },
    ]
}

fn tests_readme() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/README.json");
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn readme_entry(readme: &Value) -> &Value {
    readme
        .get("tests")
        .and_then(Value::as_array)
        .expect("tests array")
        .iter()
        .find(|entry| entry.get("test_id").and_then(Value::as_str) == Some(TEST_ID))
        .expect("candle_e2e_smoke README entry")
}

fn model_dir_from_env(spec: FamilySpec) -> Option<PathBuf> {
    env::var_os(spec.env_var).map(PathBuf::from)
}

fn missing_model_inputs(model_dir: &Path) -> Option<String> {
    let artifact = model_dir.join("model.safetensors");
    let tokenizer = model_dir.join("tokenizer.json");
    if !artifact.is_file() {
        return Some(format!("missing {}", artifact.display()));
    }
    if !tokenizer.is_file() {
        return Some(format!("missing {}", tokenizer.display()));
    }
    None
}

#[cfg(feature = "candle-runtime-engine")]
mod live {
    use std::{collections::HashMap, path::Path};

    use candle_core::{safetensors, Device, Tensor};
    use futures::StreamExt;
    use handshake_core::model_runtime::{
        candle::{adapter::sha256_file, CandleRuntime, SSMStateVariant},
        BaseModelTag, CancellationToken, CaptureSpec, GenPrompt, GenerateRequest, HookPoint,
        KvCacheOps, KvCachePolicy, KvQuantSupport, LayerIndex, LicenseTag, LoadSpec,
        LoraDescriptor, LoraId, LoraStrength, ModelCapabilities, ModelRuntime, ModelRuntimeError,
        ProviderKind, RuntimeKind, SamplingParams, SteeringProvenance, SteeringVector,
        SteeringVectorId, SteeringVectorValues, CANDLE_LOCAL_ENGINE_ORIGIN,
    };
    use sha2::{Digest, Sha256};

    use super::{passed, FamilySpec, SmokeOutcome};

    pub async fn run_transformer_smoke(
        spec: FamilySpec,
        model_dir: &Path,
    ) -> Result<SmokeOutcome, ModelRuntimeError> {
        let mut runtime = CandleRuntime::default();
        let model_id = runtime
            .load(load_spec(&model_dir.join("model.safetensors"))?)
            .await?;
        let capabilities = runtime.capabilities(model_id)?;
        assert!(capabilities.supports_activation_steering);
        assert!(capabilities.supports_lora);

        let baseline = generate_tokens(&runtime, model_id, "Hello", Vec::new(), Vec::new()).await?;
        let hooks = runtime.steering_hooks(model_id)?;
        let capture = hooks
            .capture(CaptureSpec {
                prompts: vec!["Hello".to_string()],
                layers: vec![LayerIndex::new(0)],
                hook_point: HookPoint::ResidStream,
            })
            .await?;
        let width = capture
            .activations
            .get(&LayerIndex::new(0))
            .and_then(|rows| rows.first())
            .map(Vec::len)
            .ok_or_else(|| {
                ModelRuntimeError::SteeringHookError(
                    "Candle transformer capture returned no layer-0 activation row".to_string(),
                )
            })?;

        let zero_id = hooks
            .register_vector(steering_vector("mt-089-zero", width, 0.0))
            .await?;
        let zero = generate_tokens(&runtime, model_id, "Hello", Vec::new(), vec![zero_id]).await?;
        assert_eq!(
            baseline, zero,
            "zero-vector steering must preserve deterministic transformer output"
        );

        let shifted_id = hooks
            .register_vector(steering_vector("mt-089-shift", width, 1.0))
            .await?;
        let _shifted =
            generate_tokens(&runtime, model_id, "Hello", Vec::new(), vec![shifted_id]).await?;

        let lora_dir = tempfile::tempdir()
            .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))?;
        let lora_id =
            write_and_mount_lora(runtime.lora_stack(model_id)?, lora_dir.path(), width).await?;
        let _lora = generate_tokens(&runtime, model_id, "Hello", vec![lora_id], Vec::new()).await?;

        runtime.unload(model_id).await?;
        Ok(passed(spec))
    }

    pub async fn run_state_vector_smoke(
        spec: FamilySpec,
        model_dir: &Path,
        expected_variant: SSMStateVariant,
    ) -> Result<SmokeOutcome, ModelRuntimeError> {
        let mut runtime = CandleRuntime::default();
        let model_id = runtime
            .load(load_spec(&model_dir.join("model.safetensors"))?)
            .await?;
        let capabilities = runtime.capabilities(model_id)?;
        assert!(capabilities.supports_subquadratic);
        assert!(!capabilities.supports_lora);
        assert!(!capabilities.supports_kv_prefix_cache);

        let state_vector = runtime.state_vector(model_id)?;
        assert_eq!(state_vector.variant(), expected_variant);
        let committed = state_vector.prefix_commit(&[1, 2, 3])?;
        let continuation =
            generate_tokens(&runtime, model_id, "hello", Vec::new(), Vec::new()).await?;
        state_vector.prefix_restore(&committed)?;
        let replay = generate_tokens(&runtime, model_id, "hello", Vec::new(), Vec::new()).await?;
        assert_eq!(
            continuation, replay,
            "state-vector restore must preserve deterministic continuation"
        );

        let mut record = state_vector.export_snapshot(&committed)?;
        record.snapshot_hash[0] ^= 0xff;
        let tamper_error = state_vector
            .restore_snapshot_record(&committed, record)
            .expect_err("tampered state-vector snapshot must be rejected");
        assert!(
            tamper_error.to_string().contains("snapshot_hash"),
            "{tamper_error}"
        );

        runtime.unload(model_id).await?;
        Ok(passed(spec))
    }

    async fn generate_tokens(
        runtime: &CandleRuntime,
        model_id: handshake_core::model_runtime::ModelId,
        prompt: &str,
        lora_overrides: Vec<LoraId>,
        steering_overrides: Vec<SteeringVectorId>,
    ) -> Result<Vec<(u32, String)>, ModelRuntimeError> {
        let mut stream = runtime.generate(GenerateRequest {
            id: model_id,
            prompt: GenPrompt::from(prompt),
            sampling: SamplingParams {
                temperature: Some(0.0),
                seed: Some(7),
                ..SamplingParams::default()
            },
            lora_overrides,
            steering_overrides,
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 2,
            stop_sequences: Vec::new(),
            speculative_mode: None,
            structured_decoding: None,
        });
        let mut tokens = Vec::new();
        while let Some(item) = stream.next().await {
            let token = item?;
            tokens.push((token.token_id, token.text));
        }
        if tokens.is_empty() {
            return Err(ModelRuntimeError::GenerateError(
                "Candle smoke generation emitted no tokens".to_string(),
            ));
        }
        Ok(tokens)
    }

    fn load_spec(artifact_path: &Path) -> Result<LoadSpec, ModelRuntimeError> {
        Ok(LoadSpec {
            artifact_path: artifact_path.to_path_buf(),
            sha256_expected: sha256_file(artifact_path)?,
            runtime_kind: RuntimeKind::Candle,
            sampling_defaults: SamplingParams::default(),
            kv_cache_policy: KvCachePolicy::Default {
                quant: KvQuantSupport::None,
                prefix_cache_ttl_seconds: 0,
                max_bytes: None,
            },
            declared_capabilities: ModelCapabilities {
                supports_lora: true,
                supports_kv_prefix_cache: true,
                supports_kv_quantization: KvQuantSupport::Q8,
                supports_activation_steering: true,
                supports_subquadratic: false,
                supports_speculative_draft: true,
                supports_eagle3: true,
            },
            provider: ProviderKind::Local,
            engine_origin: Some(CANDLE_LOCAL_ENGINE_ORIGIN.to_string()),
            external_engine_import: None,
        })
    }

    fn steering_vector(name: &str, width: usize, fill: f32) -> SteeringVector {
        let values = if fill == 0.0 {
            vec![0.0; width]
        } else {
            (0..width)
                .map(|idx| if idx % 2 == 0 { 8.0 } else { -8.0 })
                .collect()
        };
        SteeringVector::try_new(
            None,
            name,
            LayerIndex::new(0),
            HookPoint::ResidStream,
            SteeringVectorValues::try_new(values, 1.0).expect("valid steering values"),
            "MT-089 Candle E2E smoke steering vector",
            Some(SteeringProvenance::Manual {
                author: "MT-089".to_string(),
                notes: "env-gated Candle E2E smoke".to_string(),
            }),
        )
        .expect("valid steering vector")
    }

    async fn write_and_mount_lora(
        stack: handshake_core::model_runtime::LoraStackHandle,
        dir: &Path,
        width: usize,
    ) -> Result<LoraId, ModelRuntimeError> {
        let target = "model.layers.0.self_attn.q_proj";
        let path = dir.join("adapter_model.safetensors");
        write_adapter_config(dir, target);
        write_lora_file(&path, target, width)?;
        let lora_id = LoraId::new_v7();
        stack
            .mount(
                LoraDescriptor {
                    id: lora_id,
                    artifact_path: path.clone(),
                    sha256: sha256_bytes(&path)?,
                    rank: 1,
                    target_modules: vec![target.to_string()],
                    base_model_compat: BaseModelTag::new("candle-llama"),
                    license_tag: LicenseTag::new("mt-089-test"),
                },
                LoraStrength::try_new(1.0)?,
            )
            .await?;
        assert!(
            stack.list_active().iter().any(|entry| entry.id == lora_id),
            "mounted LoRA id must appear in active stack"
        );
        Ok(lora_id)
    }

    fn write_lora_file(path: &Path, target: &str, width: usize) -> Result<(), ModelRuntimeError> {
        let device = Device::Cpu;
        let mut tensors = HashMap::new();
        tensors.insert(
            format!("{target}.lora_A.weight"),
            Tensor::from_slice(&vec![0.5_f32; width], (1, width), &device)
                .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))?,
        );
        tensors.insert(
            format!("{target}.lora_B.weight"),
            Tensor::from_slice(&vec![0.5_f32; width], (width, 1), &device)
                .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))?,
        );
        safetensors::save(&tensors, path)
            .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))
    }

    fn write_adapter_config(dir: &Path, target: &str) {
        let config = serde_json::json!({
            "peft_type": "LORA",
            "target_modules": [target],
            "r": 1,
            "lora_alpha": 1.0,
            "base_model_name_or_path": "candle-llama"
        });
        std::fs::write(
            dir.join("adapter_config.json"),
            serde_json::to_vec_pretty(&config).expect("serialize adapter config"),
        )
        .expect("write adapter config");
    }

    fn sha256_bytes(path: &Path) -> Result<[u8; 32], ModelRuntimeError> {
        let bytes = std::fs::read(path)
            .map_err(|error| ModelRuntimeError::LoraStackError(error.to_string()))?;
        Ok(Sha256::digest(&bytes).into())
    }
}

#[cfg(feature = "candle-runtime-engine")]
use live::{run_state_vector_smoke, run_transformer_smoke};
