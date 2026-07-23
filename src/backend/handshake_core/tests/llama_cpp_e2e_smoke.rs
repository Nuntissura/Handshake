use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[cfg(feature = "llama-cpp-runtime-engine")]
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command, Stdio},
};

use async_trait::async_trait;
use futures::StreamExt;
use handshake_core::{
    flight_recorder::{
        events_llm_infer::{FR_EVT_LLM_INFER_END, FR_EVT_LLM_INFER_START, FR_EVT_LLM_INFER_TOKEN},
        EventFilter, FlightRecorder, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
    },
    model_runtime::{
        llama_cpp::LlamaCppRuntime, BaseModelTag, CancellationToken, FinishReason, GenPrompt,
        GenerateRequest, KvCacheOps, KvCachePolicy, KvQuantSupport, LicenseTag, LoadSpec,
        LoraDescriptor, LoraId, LoraStrength, ModelCapabilities, ModelRuntime, ModelRuntimeError,
        ProviderKind, RuntimeKind, SamplingParams, SpeculativeMode,
    },
};
use sha2::{Digest, Sha256};

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
use handshake_core::model_runtime::llama_cpp::LLAMA_CPP_NATIVE_FEATURE_DISABLED;

const TESTS_README_JSON: &str = include_str!("README.json");

#[cfg(feature = "llama-cpp-runtime-engine")]
const MT013_REAL_LLAMA_PROOF_NONCE_ENV: &str = "HANDSHAKE_MT013_REAL_LLAMA_PROOF_NONCE";
#[cfg(feature = "llama-cpp-runtime-engine")]
const MT013_FROZEN_SOURCE_WORKTREE_SHA256_ENV: &str =
    "HANDSHAKE_MT013_FROZEN_SOURCE_WORKTREE_SHA256";
#[cfg(feature = "llama-cpp-runtime-engine")]
const MT013_COMPILED_WORKTREE_MANIFEST: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/mt013-compiled-worktree-manifest-v2.txt"
));

#[cfg(feature = "llama-cpp-runtime-engine")]
static LLAMA_CPP_E2E_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(feature = "llama-cpp-runtime-engine")]
#[test]
fn mt013_compile_derived_worktree_manifest_rejects_tamper() {
    assert_mt013_manifest_v2_schema(MT013_COMPILED_WORKTREE_MANIFEST);
    let mut tampered = MT013_COMPILED_WORKTREE_MANIFEST.to_vec();
    let last = tampered
        .last_mut()
        .expect("compile-derived worktree manifest is non-empty");
    *last ^= 1;
    assert_ne!(
        MT013_COMPILED_WORKTREE_MANIFEST,
        tampered.as_slice(),
        "a one-byte manifest mutation must fail the exact compile-closure comparison"
    );
    assert_ne!(
        Sha256::digest(MT013_COMPILED_WORKTREE_MANIFEST).as_slice(),
        Sha256::digest(&tampered).as_slice(),
        "a one-byte manifest mutation must change the published compile-derived digest"
    );
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn assert_mt013_manifest_v2_schema(manifest: &[u8]) {
    let manifest = std::str::from_utf8(manifest).expect("manifest must be UTF-8");
    let mut lines = manifest.lines();
    assert_eq!(
        lines.next(),
        Some("hsk.mt013_compiled_worktree_manifest@2"),
        "compile-derived closure must use the explicit present/absent v2 schema"
    );
    let rows = lines.collect::<Vec<_>>();
    assert!(
        !rows.is_empty(),
        "compile-derived closure must have members"
    );
    let mut prior_path = None;
    for row in rows {
        let fields = row.split('\t').collect::<Vec<_>>();
        let path = match fields.first().copied() {
            Some("P") => {
                assert_eq!(fields.len(), 8, "malformed present manifest row: {row}");
                assert!(
                    matches!(fields[1], "tracked" | "untracked"),
                    "invalid present member class: {row}"
                );
                if fields[1] == "tracked" {
                    assert_index_posture(fields[2], fields[3], fields[4], row);
                } else {
                    assert_eq!(&fields[2..5], &["-", "-", "-"], "{row}");
                }
                fields[6]
                    .parse::<u64>()
                    .unwrap_or_else(|_| panic!("invalid present member length: {row}"));
                assert_git_object_id(fields[7], row);
                fields[5]
            }
            Some("A") => {
                assert_eq!(fields.len(), 6, "malformed absent manifest row: {row}");
                assert!(
                    matches!(fields[1], "skip-worktree" | "gitlink"),
                    "invalid absent member class: {row}"
                );
                assert_index_posture(fields[2], fields[3], fields[4], row);
                fields[5]
            }
            _ => panic!("unknown manifest row type: {row}"),
        };
        assert!(!path.is_empty(), "manifest path must not be empty: {row}");
        if let Some(prior) = prior_path {
            assert!(prior < path, "manifest paths must be unique and sorted");
        }
        prior_path = Some(path);
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn assert_index_posture(status_tag: &str, mode: &str, object_id: &str, row: &str) {
    assert!(
        status_tag.len() == 1 && status_tag.as_bytes()[0].is_ascii_alphabetic(),
        "invalid index status tag in manifest row: {row}"
    );
    assert!(
        mode.len() == 6 && mode.bytes().all(|byte| matches!(byte, b'0'..=b'7')),
        "invalid index mode in manifest row: {row}"
    );
    assert_git_object_id(object_id, row);
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn assert_git_object_id(value: &str, row: &str) {
    assert!(
        matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "invalid Git object id in manifest row: {row}"
    );
}

#[tokio::test]
#[ignore = "requires HANDSHAKE_TEST_GGUF_PATH and optional HANDSHAKE_TEST_LORA_PATH"]
async fn llama_cpp_e2e_smoke_load_generate_lora_kv_spec_score_embed_unload() {
    assert_tests_readme_registry_entry();

    let gguf_path = fixture_gguf_path()
        .expect("HARD FAIL: authoritative llama.cpp proof requires HANDSHAKE_TEST_GGUF_PATH");
    let base_tag = std::env::var("HANDSHAKE_TEST_LORA_BASE_TAG")
        .unwrap_or_else(|_| "llama-cpp-e2e-base".to_string());
    let recorder = Arc::new(E2eEventRecorder::default());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mut runtime =
        LlamaCppRuntime::with_flight_recorder(KvCachePolicy::default(), flight_recorder);
    let sha256 = sha256_file(&gguf_path);

    #[cfg(feature = "llama-cpp-runtime-engine")]
    let proof_context = prepare_mt013_llama_proof();

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        let err = runtime
            .load(load_spec(&gguf_path, sha256.clone(), &base_tag))
            .await
            .expect_err("native-disabled builds validate then reject the real fixture");
        assert!(
            err.to_string().contains(LLAMA_CPP_NATIVE_FEATURE_DISABLED),
            "{err}"
        );
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let _guard = llama_cpp_e2e_test_guard();
        let model_id = runtime
            .load(load_spec(&gguf_path, sha256.clone(), &base_tag))
            .await
            .expect("native-enabled build loads representative GGUF");

        let actual_capabilities = runtime
            .capabilities(model_id)
            .expect("loaded model reports capabilities");
        let embedding_dimension = actual_capabilities
            .embedding_dimension
            .expect("an embedding-enabled model reports its model-derived dimension");
        assert!(
            embedding_dimension > 0,
            "an embedding-enabled model reports a non-zero dimension"
        );
        let mut expected_loaded_capabilities = expected_capabilities();
        expected_loaded_capabilities.embedding_dimension = Some(embedding_dimension);
        assert_eq!(actual_capabilities, &expected_loaded_capabilities);

        let mut generation_count = 0_usize;
        let mut longest_generation = 0_usize;

        let baseline = collect_generation(
            &runtime,
            generate_request(model_id, baseline_prompt(), 32, Some(42), None),
        )
        .await
        .expect("baseline generation streams");
        assert_nonempty_generation("baseline", &baseline);
        generation_count += 1;
        longest_generation = longest_generation.max(baseline.generated_token_count());

        if let Some(lora_path) = optional_lora_path() {
            let lora_id = LoraId::new_v7();
            let stack = runtime
                .lora_stack(model_id)
                .expect("loaded model exposes LoRA stack");
            stack
                .mount(
                    lora_descriptor(&lora_path, &base_tag, lora_id),
                    LoraStrength::try_new(0.75).expect("valid LoRA strength"),
                )
                .await
                .expect("operator-supplied LoRA fixture mounts");
            assert_eq!(stack.list_active().len(), 1);

            let mut lora_request =
                generate_request(model_id, baseline_prompt(), 32, Some(42), None);
            lora_request.lora_overrides = vec![lora_id];
            let lora_generation = collect_generation(&runtime, lora_request)
                .await
                .expect("LoRA-backed generation streams");
            assert_nonempty_generation("lora", &lora_generation);
            if lora_difference_assertion_enabled() {
                assert_ne!(
                    lora_generation.token_ids, baseline.token_ids,
                    "curated LoRA fixture should change the deterministic token stream"
                );
            }
            generation_count += 1;
            longest_generation = longest_generation.max(lora_generation.generated_token_count());

            stack.unmount(lora_id).await.expect("unmount LoRA");
            assert!(stack.list_active().is_empty());
        } else {
            eprintln!(
                "SKIPPED llama_cpp_e2e_smoke LoRA substep: HANDSHAKE_TEST_LORA_PATH is not set"
            );
        }

        let cache = runtime
            .llama_cpp_kv_cache(model_id)
            .expect("loaded model exposes native KV cache ops");
        assert_eq!(cache.quantization(), KvQuantSupport::None);
        cache
            .set_quantization(KvQuantSupport::Q4)
            .expect("q4 KV quantization is supported");
        assert_eq!(cache.quantization(), KvQuantSupport::Q4);

        let kv_prompt_tokens = runtime
            .tokenize_prompt(model_id, kv_prompt())
            .expect("loaded model tokenizes KV prompt");
        assert!(
            kv_prompt_tokens.len() > 1,
            "KV prompt needs at least one prefix token and one suffix token"
        );
        let prefix_len = 20_usize.min(kv_prompt_tokens.len() - 1);
        let prefix = cache
            .prefix_commit(&kv_prompt_tokens[..prefix_len])
            .expect("prefix commit captures native KV state");
        assert_eq!(cache.occupancy().prefix_cache_entries, 1);

        let mut first_kv_request = generate_request(model_id, kv_prompt(), 16, Some(77), None);
        first_kv_request.kv_prefix_handle = Some(prefix.clone());
        let first_kv = collect_generation(&runtime, first_kv_request)
            .await
            .expect("KV-prefix generation streams");
        assert_nonempty_generation("kv first", &first_kv);
        generation_count += 1;
        longest_generation = longest_generation.max(first_kv.generated_token_count());

        cache
            .prefix_restore(&prefix)
            .expect("known prefix restores before deterministic replay");
        let mut second_kv_request = generate_request(model_id, kv_prompt(), 16, Some(77), None);
        second_kv_request.kv_prefix_handle = Some(prefix);
        let second_kv = collect_generation(&runtime, second_kv_request)
            .await
            .expect("KV-prefix replay streams");
        assert_eq!(
            second_kv, first_kv,
            "same prompt, seed, quantization, and KV prefix must replay deterministically"
        );
        generation_count += 1;
        longest_generation = longest_generation.max(second_kv.generated_token_count());

        let ngram_prompt = select_ngram_prompt(&runtime, model_id)
            .expect("fixture tokenizer must support a repeated 4-token ngram prompt");
        let ngram_baseline = collect_generation(
            &runtime,
            generate_request(model_id, ngram_prompt, 32, Some(42), None),
        )
        .await
        .expect("non-speculative ngram baseline streams");
        generation_count += 1;
        longest_generation = longest_generation.max(ngram_baseline.generated_token_count());

        let ngram_spec = collect_generation(
            &runtime,
            generate_request(
                model_id,
                ngram_prompt,
                32,
                Some(42),
                Some(SpeculativeMode::Ngram {
                    lookback: 4,
                    max_draft: 4,
                }),
            ),
        )
        .await
        .expect("ngram speculative generation streams");
        assert_eq!(
            ngram_spec, ngram_baseline,
            "ngram speculative mode must preserve deterministic generation output"
        );
        generation_count += 1;
        longest_generation = longest_generation.max(ngram_spec.generated_token_count());

        let spec_stats = runtime
            .last_speculative_stats(model_id)
            .expect("speculative stats lookup succeeds")
            .expect("speculative generation records stats");
        assert!(
            spec_stats.draft_calls > 0,
            "repeated ngram prompt should execute at least one draft round, got {spec_stats:?}"
        );
        assert!(
            spec_stats.accepted_tokens + spec_stats.rejected_tokens > 0,
            "ngram speculative generation should verify proposed tokens, got {spec_stats:?}"
        );

        let ngram_baseline_after_spec = collect_generation(
            &runtime,
            generate_request(model_id, ngram_prompt, 32, Some(42), None),
        )
        .await
        .expect("post-speculative baseline generation streams");
        assert_eq!(
            ngram_baseline_after_spec, ngram_baseline,
            "non-speculative generation should remain deterministic after ngram speculation"
        );
        assert_eq!(
            runtime
                .last_speculative_stats(model_id)
                .expect("post-baseline speculative stats lookup succeeds"),
            Some(Default::default()),
            "non-speculative generation should reset speculative stats"
        );
        generation_count += 1;
        longest_generation =
            longest_generation.max(ngram_baseline_after_spec.generated_token_count());

        let score = runtime
            .score(model_id, (1_u32..=10).collect())
            .await
            .expect("score computes token log probabilities");
        assert_eq!(score.token_logprobs.len(), 9);
        assert!(
            score.token_logprobs.iter().all(|value| value.is_finite()),
            "token logprobs must be finite: {:?}",
            score.token_logprobs
        );
        assert!(
            score.token_logprobs.iter().all(|value| *value <= 0.0),
            "log probabilities must be non-positive: {:?}",
            score.token_logprobs
        );
        assert!(
            score.mean_logprob.is_finite() && score.mean_logprob <= 0.0,
            "mean log probability must be finite and non-positive: {:?}",
            score.mean_logprob
        );

        let embedding = runtime
            .embed(model_id, "llama.cpp e2e embedding smoke")
            .await
            .expect("a model advertising embedding support must produce an embedding");
        assert_eq!(
            embedding.vector.len(),
            embedding_dimension,
            "embedding output length must match the model-derived capability dimension"
        );
        assert!(embedding.vector.iter().all(|value| value.is_finite()));

        let events = wait_for_generation_events(&recorder, generation_count).await;
        assert_has_event_id(&events, FR_EVT_LLM_INFER_START);
        assert_has_event_id(&events, FR_EVT_LLM_INFER_END);
        if longest_generation >= 16 {
            assert_has_event_id(&events, FR_EVT_LLM_INFER_TOKEN);
        }
        if spec_stats.accepted_tokens > 0 {
            assert_has_event_type(&events, FlightRecorderEventType::LlmInferenceSpecAccept);
        }
        if spec_stats.rejected_tokens > 0 {
            assert_has_event_type(&events, FlightRecorderEventType::LlmInferenceSpecReject);
        }
        assert_events_do_not_leak_prompt_or_token_text(&events);

        runtime.unload(model_id).await.expect("unload model");
        assert!(
            runtime.capabilities(model_id).is_err(),
            "unloaded model must be absent from runtime model map"
        );
        assert!(
            runtime.perf_stats(model_id).is_err(),
            "unloaded model must have no perf stats handle"
        );

        publish_mt013_llama_proof(
            &proof_context,
            &gguf_path,
            &sha256,
            model_id,
            generation_count,
            longest_generation,
            embedding_dimension,
            events.len(),
        );
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
struct Mt013LlamaProofContext {
    proof_dir: PathBuf,
    proof_path: PathBuf,
    provenance_path: PathBuf,
    proof_nonce: String,
    frozen_source_worktree_sha256: String,
    compiled_source_worktree_sha256: String,
    producer_source_sha256: String,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn prepare_mt013_llama_proof() -> Mt013LlamaProofContext {
    let artifacts_root = std::env::var_os("HANDSHAKE_ARTIFACTS_DIR")
        .map(PathBuf::from)
        .expect("MT-013 real llama.cpp proof requires HANDSHAKE_ARTIFACTS_DIR");
    assert!(
        artifacts_root.is_absolute(),
        "HANDSHAKE_ARTIFACTS_DIR must be an absolute external root"
    );
    let repo_root = discover_test_repo_root();
    assert!(
        !artifacts_root.starts_with(&repo_root),
        "MT-013 llama.cpp proof root must be external to repo {}",
        repo_root.display()
    );
    let proof_dir = artifacts_root
        .join("handshake-test")
        .join("wp1-final-audit");
    fs::create_dir_all(&proof_dir).expect("create MT-013 llama.cpp proof directory");
    let proof_path = proof_dir.join("mt013-real-llama-cpp-runtime-proof-v1.json");
    let provenance_path = proof_dir.join("mt013-real-llama-cpp-runtime-proof-v1.provenance.json");
    for stale_path in [&proof_path, &provenance_path] {
        if stale_path.exists() {
            fs::remove_file(stale_path)
                .unwrap_or_else(|error| panic!("remove stale {}: {error}", stale_path.display()));
        }
    }

    let proof_nonce = uuid::Uuid::parse_str(
        &std::env::var(MT013_REAL_LLAMA_PROOF_NONCE_ENV).unwrap_or_else(|_| {
            panic!("MT-013 real llama.cpp proof requires {MT013_REAL_LLAMA_PROOF_NONCE_ENV}")
        }),
    )
    .expect("MT-013 real llama.cpp proof nonce must be a UUID")
    .to_string();
    let frozen_source_worktree_sha256 = std::env::var(MT013_FROZEN_SOURCE_WORKTREE_SHA256_ENV)
        .unwrap_or_else(|_| {
            panic!("MT-013 real llama.cpp proof requires {MT013_FROZEN_SOURCE_WORKTREE_SHA256_ENV}")
        });
    require_canonical_sha256("frozen source/worktree", &frozen_source_worktree_sha256);
    let current_worktree_manifest = build_worktree_manifest(&repo_root);
    assert_eq!(
        MT013_COMPILED_WORKTREE_MANIFEST,
        current_worktree_manifest.as_slice(),
        "HARD FAIL: executing MT-013 binary's compile-derived full-worktree manifest differs from the current tracked+untracked non-ignored source/config/lockfile closure"
    );
    let compiled_source_worktree_sha256 =
        hex::encode(Sha256::digest(MT013_COMPILED_WORKTREE_MANIFEST));
    let actual_frozen_source_worktree_sha256 =
        hex::encode(Sha256::digest(&current_worktree_manifest));
    assert_eq!(
        frozen_source_worktree_sha256, actual_frozen_source_worktree_sha256,
        "HARD FAIL: supplied frozen source/worktree digest does not match the exact versioned manifest of present tracked+untracked non-ignored bytes and explicit absent tracked posture"
    );
    assert_eq!(
        frozen_source_worktree_sha256, compiled_source_worktree_sha256,
        "HARD FAIL: frozen and compile-derived source/worktree manifest digests differ"
    );
    require_canonical_sha256(
        "compiled source/worktree closure",
        &compiled_source_worktree_sha256,
    );
    let producer_source_sha256 =
        hex::encode(Sha256::digest(include_bytes!("llama_cpp_e2e_smoke.rs")));
    let current_producer_source =
        fs::read(repo_root.join("src/backend/handshake_core/tests/llama_cpp_e2e_smoke.rs"))
            .expect("HARD FAIL: read current MT-013 proof-producing source");
    assert_eq!(
        hex::encode(Sha256::digest(current_producer_source)),
        producer_source_sha256,
        "HARD FAIL: executing MT-013 test binary was compiled from different producer source bytes than the frozen worktree"
    );

    Mt013LlamaProofContext {
        proof_dir,
        proof_path,
        provenance_path,
        proof_nonce,
        frozen_source_worktree_sha256,
        compiled_source_worktree_sha256,
        producer_source_sha256,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[allow(clippy::too_many_arguments)]
fn publish_mt013_llama_proof(
    context: &Mt013LlamaProofContext,
    gguf_path: &Path,
    gguf_sha256: &str,
    model_id: handshake_core::model_runtime::ModelId,
    generation_count: usize,
    longest_generation: usize,
    embedding_dimension: usize,
    flight_recorder_event_count: usize,
) {
    require_canonical_sha256("GGUF", gguf_sha256);
    let completed_at = chrono::Utc::now();
    let proof = serde_json::json!({
        "schema_id": "hsk.mt013_real_llama_cpp_runtime_proof@1",
        "proof_nonce": context.proof_nonce.clone(),
        "producer_test_id": "llama_cpp_e2e_smoke_load_generate_lora_kv_spec_score_embed_unload",
        "producer_completed_at_utc": completed_at.to_rfc3339(),
        "producer_completed_at_unix_ms": completed_at.timestamp_millis(),
        "frozen_source_worktree_sha256": context.frozen_source_worktree_sha256.clone(),
        "compiled_source_worktree_sha256": context.compiled_source_worktree_sha256.clone(),
        "compiled_source_closure": "build.rs-generated and binary-embedded manifest of present git tracked + untracked non-ignored worktree bytes via Git object IDs, with explicit absent tracked index posture, including source, configuration, and lockfiles",
        "producer_source_sha256": context.producer_source_sha256.clone(),
        "gguf": {
            "path": gguf_path,
            "sha256": gguf_sha256,
            "length_bytes": fs::metadata(gguf_path).expect("inspect proof GGUF").len(),
        },
        "result": {
            "status": "PASS",
            "passed": 1,
            "failed": 0,
            "model_id": model_id,
            "generation_count": generation_count,
            "longest_generation_tokens": longest_generation,
            "embedding_dimension": embedding_dimension,
            "flight_recorder_event_count": flight_recorder_event_count,
            "unload_verified": true,
        },
    });
    let proof_bytes =
        serde_json::to_vec_pretty(&proof).expect("serialize MT-013 llama.cpp proof artifact");
    let proof_sha256 = hex::encode(Sha256::digest(&proof_bytes));
    let provenance = serde_json::json!({
        "schema_id": "hsk.mt013_real_llama_cpp_runtime_provenance@1",
        "proof_nonce": context.proof_nonce.clone(),
        "producer_test_id": "llama_cpp_e2e_smoke_load_generate_lora_kv_spec_score_embed_unload",
        "producer_completed_at_utc": completed_at.to_rfc3339(),
        "producer_completed_at_unix_ms": completed_at.timestamp_millis(),
        "frozen_source_worktree_sha256": context.frozen_source_worktree_sha256.clone(),
        "compiled_source_worktree_sha256": context.compiled_source_worktree_sha256.clone(),
        "compiled_source_closure": "build.rs-generated and binary-embedded manifest of present git tracked + untracked non-ignored worktree bytes via Git object IDs, with explicit absent tracked index posture, including source, configuration, and lockfiles",
        "producer_source_sha256": context.producer_source_sha256.clone(),
        "gguf_sha256": gguf_sha256,
        "artifact_sha256": proof_sha256,
        "result": "PASS",
    });
    let proof_temp = context.proof_dir.join(format!(
        "mt013-real-llama-cpp-runtime-proof-v1.{}.tmp",
        context.proof_nonce
    ));
    let provenance_temp = context.proof_dir.join(format!(
        "mt013-real-llama-cpp-runtime-proof-v1.provenance.{}.tmp",
        context.proof_nonce
    ));
    fs::write(&proof_temp, &proof_bytes).expect("write temporary MT-013 llama.cpp proof");
    fs::rename(&proof_temp, &context.proof_path)
        .expect("publish MT-013 llama.cpp proof atomically");
    let published_proof = fs::read(&context.proof_path)
        .expect("read back published MT-013 llama.cpp proof before commit marker");
    assert_eq!(
        hex::encode(Sha256::digest(&published_proof)),
        proof_sha256,
        "published MT-013 llama.cpp proof changed before provenance commit"
    );
    let published_json: serde_json::Value = serde_json::from_slice(&published_proof)
        .expect("published MT-013 llama.cpp proof remains valid JSON");
    assert_eq!(
        published_json["proof_nonce"].as_str(),
        Some(context.proof_nonce.as_str())
    );
    assert_eq!(published_json["result"]["status"], "PASS");

    // The provenance file is the commit marker and is deliberately published
    // last. A crash can leave an uncommitted proof, but can never leave a PASS
    // provenance pointing at a missing or unread-back proof artifact.
    fs::write(
        &provenance_temp,
        serde_json::to_vec_pretty(&provenance).expect("serialize MT-013 llama.cpp provenance"),
    )
    .expect("write temporary MT-013 llama.cpp provenance commit marker");
    fs::rename(&provenance_temp, &context.provenance_path)
        .expect("publish MT-013 llama.cpp provenance commit marker atomically");
    eprintln!(
        "[MT-013_REAL_LLAMA_CPP_PROOF] {}",
        context.proof_path.display()
    );
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn require_canonical_sha256(label: &str, value: &str) {
    assert!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{label} sha256 must be 64 lowercase hexadecimal characters"
    );
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn discover_test_repo_root() -> PathBuf {
    let mut candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if candidate.join(".git").exists() {
            return candidate;
        }
        assert!(
            candidate.pop(),
            "cannot discover worktree root from CARGO_MANIFEST_DIR"
        );
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn build_worktree_manifest(repo_root: &Path) -> Vec<u8> {
    let tracked_output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files", "--cached", "--stage", "-v", "-z"])
        .output()
        .expect("HARD FAIL: git is required to enumerate tracked MT-013 manifest members");
    assert!(
        tracked_output.status.success(),
        "HARD FAIL: tracked git ls-files failed: {}",
        String::from_utf8_lossy(&tracked_output.stderr)
    );
    let mut members = BTreeMap::<String, Option<ManifestTrackedIndexEntry>>::new();
    for entry in tracked_output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let entry = String::from_utf8(entry.to_vec()).expect("tracked path must be UTF-8");
        let (metadata, path) = entry
            .split_once('\t')
            .unwrap_or_else(|| panic!("HARD FAIL: malformed tracked index entry: {entry:?}"));
        let mut fields = metadata.split_whitespace();
        let status_tag = fields
            .next()
            .and_then(|value| value.chars().next())
            .expect("tracked index status tag");
        let mode = fields.next().expect("tracked index mode").to_string();
        let index_object_id = fields.next().expect("tracked index object id").to_string();
        let stage = fields.next().expect("tracked index stage");
        assert_eq!(
            stage, "0",
            "HARD FAIL: unresolved index stage {stage} for {path}"
        );
        assert!(fields.next().is_none(), "unexpected index metadata");
        assert!(
            members
                .insert(
                    path.to_string(),
                    Some(ManifestTrackedIndexEntry {
                        status_tag,
                        mode,
                        index_object_id,
                    })
                )
                .is_none(),
            "HARD FAIL: duplicate tracked path {path}"
        );
    }
    let untracked_output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()
        .expect("HARD FAIL: git is required to enumerate untracked MT-013 manifest members");
    assert!(
        untracked_output.status.success(),
        "HARD FAIL: untracked git ls-files failed: {}",
        String::from_utf8_lossy(&untracked_output.stderr)
    );
    for path in untracked_output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = String::from_utf8(path.to_vec()).expect("untracked path must be UTF-8");
        assert!(
            members.insert(path.clone(), None).is_none(),
            "HARD FAIL: path is both tracked and untracked: {path}"
        );
    }
    assert!(!members.is_empty(), "HARD FAIL: frozen worktree is empty");
    assert!(
        members
            .keys()
            .all(|path| !path.chars().any(|ch| matches!(ch, '\n' | '\r' | '\t'))),
        "HARD FAIL: manifest paths must not contain control separators"
    );

    let mut present_paths = Vec::new();
    let mut absent_tracked = BTreeMap::new();
    for (path, tracked) in &members {
        let absolute = repo_root.join(path);
        match fs::symlink_metadata(&absolute) {
            Ok(_) if tracked.as_ref().is_some_and(|entry| entry.mode == "160000") => {
                absent_tracked.insert(path.clone(), "gitlink".to_string());
            }
            Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => {
                present_paths.push(path.clone());
            }
            Ok(_) => panic!(
                "HARD FAIL: closure member is present but not a file or symlink: {}",
                absolute.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => match tracked {
                Some(entry) if entry.mode == "160000" => {
                    absent_tracked.insert(path.clone(), "gitlink".to_string());
                }
                Some(entry) if matches!(entry.status_tag, 'S' | 's') => {
                    absent_tracked.insert(path.clone(), "skip-worktree".to_string());
                }
                Some(_) => panic!(
                    "HARD FAIL: tracked member absent without skip-worktree: {}",
                    absolute.display()
                ),
                None => panic!(
                    "HARD FAIL: untracked member disappeared during enumeration: {}",
                    absolute.display()
                ),
            },
            Err(error) => panic!("HARD FAIL: inspect {}: {error}", absolute.display()),
        }
    }

    let mut hash_child = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["hash-object", "--no-filters", "--stdin-paths"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("HARD FAIL: spawn git hash-object");
    let mut hash_input = Vec::new();
    for path in &present_paths {
        writeln!(&mut hash_input, "{path}").expect("write in-memory hash-object input");
    }
    let mut hash_stdin = hash_child.stdin.take().expect("hash-object stdin");
    let write_handle = std::thread::spawn(move || hash_stdin.write_all(&hash_input));
    let wait_result = hash_child.wait_with_output();
    let write_result = write_handle
        .join()
        .unwrap_or_else(|_| panic!("HARD FAIL: hash-object input writer thread panicked"));
    let hashes = wait_result.expect("HARD FAIL: wait for git hash-object");
    if let Err(write_error) = write_result {
        let first_disappeared = present_paths
            .iter()
            .find(|path| fs::symlink_metadata(repo_root.join(path)).is_err());
        panic!(
            "HARD FAIL: git hash-object input closed while hashing present closure members; \
             first_disappeared={first_disappeared:?}; write_error={write_error}; stderr={}",
            String::from_utf8_lossy(&hashes.stderr)
        );
    }
    if !hashes.status.success() {
        let first_disappeared = present_paths
            .iter()
            .find(|path| fs::symlink_metadata(repo_root.join(path)).is_err());
        panic!(
            "HARD FAIL: git hash-object failed for present closure members; \
             first_disappeared={first_disappeared:?}; status={}; stderr={}",
            hashes.status,
            String::from_utf8_lossy(&hashes.stderr)
        );
    }
    let object_ids = String::from_utf8(hashes.stdout)
        .expect("git object ids must be UTF-8")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(
        present_paths.len(),
        object_ids.len(),
        "HARD FAIL: present manifest hash count drift"
    );

    let present_object_ids = present_paths
        .iter()
        .cloned()
        .zip(object_ids)
        .collect::<BTreeMap<_, _>>();
    let mut manifest = b"hsk.mt013_compiled_worktree_manifest@2\n".to_vec();
    for (path, tracked) in &members {
        let absolute = repo_root.join(path);
        if let Some(absence_kind) = absent_tracked.get(path) {
            let tracked = tracked
                .as_ref()
                .expect("only tracked entries may be explicitly absent");
            writeln!(
                &mut manifest,
                "A\t{absence_kind}\t{}\t{}\t{}\t{path}",
                tracked.status_tag, tracked.mode, tracked.index_object_id
            )
            .expect("write absent manifest row");
        } else {
            let object_id = present_object_ids
                .get(path)
                .expect("every present member has a worktree object id");
            let length = fs::symlink_metadata(&absolute)
                .unwrap_or_else(|error| panic!("inspect present {}: {error}", absolute.display()))
                .len();
            match tracked {
                Some(tracked) => writeln!(
                    &mut manifest,
                    "P\ttracked\t{}\t{}\t{}\t{path}\t{length}\t{object_id}",
                    tracked.status_tag, tracked.mode, tracked.index_object_id
                ),
                None => writeln!(
                    &mut manifest,
                    "P\tuntracked\t-\t-\t-\t{path}\t{length}\t{object_id}"
                ),
            }
            .expect("write present manifest row");
        }
    }
    manifest
}

#[cfg(feature = "llama-cpp-runtime-engine")]
struct ManifestTrackedIndexEntry {
    status_tag: char,
    mode: String,
    index_object_id: String,
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn llama_cpp_e2e_test_guard() -> std::sync::MutexGuard<'static, ()> {
    LLAMA_CPP_E2E_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn fixture_gguf_path() -> Option<PathBuf> {
    std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(PathBuf::from)
}

fn optional_lora_path() -> Option<PathBuf> {
    std::env::var_os("HANDSHAKE_TEST_LORA_PATH").map(PathBuf::from)
}

fn lora_difference_assertion_enabled() -> bool {
    std::env::var_os("HANDSHAKE_TEST_LORA_EXPECT_DIFFERENT").is_some()
}

fn baseline_prompt() -> &'static str {
    "Handshake local model runtime smoke test:"
}

fn kv_prompt() -> &'static str {
    "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega continuation"
}

const NGRAM_PROMPT_CANDIDATES: &[&str] = &[
    " alpha beta gamma delta alpha beta gamma delta alpha beta gamma delta",
    " red blue green yellow red blue green yellow red blue green yellow",
    " one two three four one two three four one two three four",
    " test test test test test test test test test test test test",
    " repeat repeat repeat repeat repeat repeat repeat repeat repeat repeat repeat repeat",
];

fn generate_request(
    id: handshake_core::model_runtime::ModelId,
    prompt: &str,
    max_tokens: u32,
    seed: Option<u32>,
    speculative_mode: Option<SpeculativeMode>,
) -> GenerateRequest {
    GenerateRequest {
        id,
        prompt: GenPrompt::from(prompt),
        sampling: SamplingParams {
            seed,
            ..SamplingParams::default()
        },
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens,
        stop_sequences: Vec::new(),
        speculative_mode,
        structured_decoding: None,
    }
}

fn load_spec(artifact_path: &Path, sha256_expected: String, base_tag: &str) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected,
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::None,
            prefix_cache_ttl_seconds: 300,
            max_bytes: None,
        },
        declared_capabilities: expected_capabilities(),
        provider: ProviderKind::Local,
        engine_origin: Some(base_tag.to_string()),
        external_engine_import: None,
    }
}

fn expected_capabilities() -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_embedding: true,
        embedding_dimension: None,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q4Q8Mix,
        supports_activation_steering: false,
        supports_subquadratic: false,
        supports_speculative_draft: true,
        supports_eagle3: false,
    }
}

fn lora_descriptor(path: &Path, base_tag: &str, id: LoraId) -> LoraDescriptor {
    LoraDescriptor {
        id,
        artifact_path: path.to_path_buf(),
        sha256: sha256_bytes(path),
        rank: 1,
        target_modules: vec!["q_proj".to_string()],
        base_model_compat: BaseModelTag::new(base_tag),
        license_tag: LicenseTag::new("operator-local"),
    }
}

async fn collect_generation(
    runtime: &LlamaCppRuntime,
    req: GenerateRequest,
) -> Result<GenerationTrace, ModelRuntimeError> {
    let mut stream = runtime.generate(req);
    let mut token_ids = Vec::new();
    let mut text = String::new();
    let mut finish = None;
    while let Some(item) = stream.next().await {
        let token = item?;
        token_ids.push(token.token_id);
        text.push_str(&token.text);
        if token.finish_reason.is_some() && finish.is_none() {
            finish = token.finish_reason;
        }
    }
    Ok(GenerationTrace {
        token_ids,
        text,
        finish,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GenerationTrace {
    token_ids: Vec<u32>,
    text: String,
    finish: Option<FinishReason>,
}

impl GenerationTrace {
    fn generated_token_count(&self) -> usize {
        self.token_ids.len()
    }
}

fn assert_nonempty_generation(label: &str, trace: &GenerationTrace) {
    assert!(
        !trace.token_ids.is_empty(),
        "{label} generation should emit at least one token"
    );
    assert!(
        matches!(
            trace.finish,
            Some(FinishReason::Length) | Some(FinishReason::Stop)
        ),
        "{label} generation should finish with length or stop, got {:?}",
        trace.finish
    );
}

fn select_ngram_prompt(
    runtime: &LlamaCppRuntime,
    model_id: handshake_core::model_runtime::ModelId,
) -> Option<&'static str> {
    NGRAM_PROMPT_CANDIDATES.iter().copied().find(|candidate| {
        runtime
            .tokenize_prompt(model_id, candidate)
            .map(|tokens| has_repeated_suffix_ngram_with_draft(&tokens, 4))
            .unwrap_or(false)
    })
}

fn has_repeated_suffix_ngram_with_draft(tokens: &[u32], lookback: usize) -> bool {
    if tokens.len() <= lookback {
        return false;
    }

    let key_start = tokens.len() - lookback;
    let key = &tokens[key_start..];
    (0..key_start).rev().any(|candidate_start| {
        let candidate_end = candidate_start + lookback;
        &tokens[candidate_start..candidate_end] == key && candidate_end < tokens.len()
    })
}

#[derive(Default)]
struct E2eEventRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait]
impl FlightRecorder for E2eEventRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self
            .events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .clone())
    }
}

async fn wait_for_generation_events(
    recorder: &E2eEventRecorder,
    expected_end_events: usize,
) -> Vec<FlightRecorderEvent> {
    for _ in 0..40 {
        let events = recorder
            .list_events(EventFilter::default())
            .await
            .expect("list e2e events");
        let end_count = events
            .iter()
            .filter(|event| event.payload["event_id"] == FR_EVT_LLM_INFER_END)
            .count();
        if end_count >= expected_end_events {
            return events;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    recorder
        .list_events(EventFilter::default())
        .await
        .expect("list e2e events after wait")
}

fn assert_has_event_id(events: &[FlightRecorderEvent], event_id: &str) {
    assert!(
        events
            .iter()
            .any(|event| event.payload["event_id"] == event_id),
        "missing flight recorder event family {event_id}; got {:?}",
        event_ids(events)
    );
}

fn assert_has_event_type(events: &[FlightRecorderEvent], event_type: FlightRecorderEventType) {
    assert!(
        events.iter().any(|event| event.event_type == event_type),
        "missing flight recorder event type {event_type:?}; got {:?}",
        events
            .iter()
            .map(|event| event.event_type.clone())
            .collect::<Vec<_>>()
    );
}

fn assert_events_do_not_leak_prompt_or_token_text(events: &[FlightRecorderEvent]) {
    for event in events {
        let payload = event.payload.to_string();
        for forbidden in [
            baseline_prompt(),
            kv_prompt(),
            "llama.cpp e2e embedding smoke",
        ] {
            assert!(
                !payload.contains(forbidden),
                "flight recorder payload leaked prompt/token text: {payload}"
            );
        }
        for forbidden in NGRAM_PROMPT_CANDIDATES {
            assert!(
                !payload.contains(forbidden),
                "flight recorder payload leaked ngram prompt text: {payload}"
            );
        }
    }
}

fn event_ids(events: &[FlightRecorderEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| {
            event
                .payload
                .get("event_id")
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn assert_tests_readme_registry_entry() {
    let registry: serde_json::Value =
        serde_json::from_str(TESTS_README_JSON).expect("tests/README.json parses");
    let tests = registry["tests"]
        .as_array()
        .expect("tests/README.json has a tests array");
    let entry = tests
        .iter()
        .find(|entry| entry["test_id"] == "llama_cpp_e2e_smoke")
        .expect("tests/README.json lists llama_cpp_e2e_smoke");

    assert_eq!(entry["test_file"], "tests/llama_cpp_e2e_smoke.rs");
    assert!(
        entry["run_command"]
            .as_str()
            .expect("llama_cpp_e2e_smoke run_command is a string")
            .contains("--features llama-cpp-runtime-engine"),
        "README run command must enable the native llama.cpp feature"
    );
    assert_json_array_contains(entry, "required_env", "HANDSHAKE_TEST_GGUF_PATH");
    assert_json_array_contains(entry, "optional_env", "HANDSHAKE_TEST_LORA_PATH");
    assert_json_array_contains(
        entry,
        "coverage",
        "kv_quantization_prefix_commit_restore_replay",
    );
    assert_json_array_contains(
        entry,
        "coverage",
        "flight_recorder_generation_and_spec_events",
    );
    assert!(
        entry["skip_policy"]
            .as_str()
            .expect("llama_cpp_e2e_smoke skip_policy is a string")
            .contains("hard-fails"),
        "README skip policy must document the authoritative GGUF hard-fail contract"
    );
    assert!(
        entry["artifact_policy"]
            .as_str()
            .expect("llama_cpp_e2e_smoke artifact_policy is a string")
            .contains("../Handshake_Artifacts/handshake-cargo-target"),
        "README artifact policy must keep build output under Handshake_Artifacts"
    );
}

fn assert_json_array_contains(entry: &serde_json::Value, key: &str, expected: &str) {
    let values = entry[key]
        .as_array()
        .unwrap_or_else(|| panic!("llama_cpp_e2e_smoke {key} must be an array"));
    assert!(
        values.iter().any(|value| value == expected),
        "llama_cpp_e2e_smoke README {key} missing {expected}: {values:?}"
    );
}

fn sha256_file(path: &Path) -> String {
    hex::encode(sha256_bytes(path))
}

fn sha256_bytes(path: &Path) -> [u8; 32] {
    let bytes = fs::read(path).expect("read fixture");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}
