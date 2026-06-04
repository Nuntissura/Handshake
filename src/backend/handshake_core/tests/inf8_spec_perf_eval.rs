//! MT-111 — INF-8 perf eval suite + EAGLE-3 upgrade-hook contract tests.
//!
//! HONESTY NOTE (post-validation remediation):
//!
//! The previous version of the two perf-eval bodies printed a "scaffolded"
//! eprintln and `return`ed EVEN WHEN HANDSHAKE_TEST_GGUF_PATH /
//! HANDSHAKE_TEST_DRAFT_GGUF_PATH were set. The 1.1x speedup and 30%
//! accept-rate thresholds were therefore NEVER exercised on any host — a
//! guaranteed false PASS. That stub has been deleted.
//!
//! What the perf eval now does:
//!
//! - It only runs when the `llama-cpp-runtime-engine` Cargo feature is
//!   compiled in (otherwise the tests carry `#[ignore]` and are not claimed to
//!   prove anything). When the feature IS present:
//!     - If the required GGUF env fixtures are unset, the test skips cleanly
//!       (no fixtures staged → nothing to measure → no claim).
//!     - If the fixtures ARE set, the test loads the real model(s) through the
//!       real `LlamaCppRuntime`, runs real baseline vs speculative generation,
//!       and asserts the real measured `speedup` / `accept_rate` against the
//!       contract thresholds. There is no path that returns a PASS without a
//!       real measurement.
//!
//! - `inf8_spec_validate_request_accepts_and_rejects_real_modes` is an
//!   always-run in-CI gate that exercises the REAL `validate_speculative_request`
//!   product logic (ngram lookback/max_draft bounds, draft-model self-reference
//!   rejection, EAGLE-3 always-unsupported). It does NOT claim any perf metric;
//!   it proves the speculative-request validation surface is wired and correct.
//!
//! EAGLE-3 upgrade-hook contract tests (always run) are unchanged: they assert
//! the single-source-of-truth PR URL, the typed PendingUpstream error, the
//! const/accessor agreement, and the canonical event-id spellings.

#[cfg(feature = "llama-cpp-runtime-engine")]
use std::path::PathBuf;

use handshake_core::model_runtime::llama_cpp::eagle3_hook::{
    eagle3_available, eagle3_decode, EagleError, EAGLE3_AVAILABLE, EAGLE3_UPSTREAM_PR_URL,
};
use handshake_core::model_runtime::techniques::speculative_decoding;

// ----------------------------------------------------------------------------
// EAGLE-3 upgrade-hook contract tests (always run).
// ----------------------------------------------------------------------------

#[test]
fn eagle3_hook_contract_single_source_of_truth_for_pr_url() {
    // The PR URL must be the same single string across all citations.
    // If a future MT flips EAGLE3_AVAILABLE without updating the URL,
    // or vice versa, this test catches the drift.
    assert_eq!(
        EAGLE3_UPSTREAM_PR_URL,
        "https://github.com/ggerganov/llama.cpp/pull/18039"
    );
}

#[test]
fn eagle3_hook_contract_typed_error_carries_pr_url() {
    let err = eagle3_decode(&[1, 2, 3], 4).expect_err("Eagle3 unavailable today");
    let EagleError::PendingUpstream { pr_url } = err;
    assert_eq!(pr_url, EAGLE3_UPSTREAM_PR_URL);
}

#[test]
fn eagle3_hook_contract_consts_agree_with_runtime_accessor() {
    assert_eq!(EAGLE3_AVAILABLE, eagle3_available());
    assert!(
        !eagle3_available(),
        "MT-111 contract: eagle3_available() must return false until the post-PR-#18039 upgrade MT flips EAGLE3_AVAILABLE"
    );
}

#[test]
fn eagle3_hook_contract_technique_surface_re_exports_canonical_event_ids() {
    // The MT-109 technique surface re-exposes SPEC-* event constants
    // so the frontend deferral path + the FR registry round-trip
    // (already tested in speculative_decoding_technique_tests) share
    // a single string spelling. This test would catch a rename in
    // either place by failing the equality.
    assert_eq!(
        speculative_decoding::FR_EVT_LLM_INFER_SPEC_MODE_CHANGE,
        "FR-EVT-LLM-INFER-SPEC-MODE-CHANGE"
    );
    assert_eq!(
        speculative_decoding::FR_EVT_LLM_INFER_SPEC_STATS,
        "FR-EVT-LLM-INFER-SPEC-STATS"
    );
}

// ----------------------------------------------------------------------------
// Always-run in-CI gate over REAL speculative-request validation logic.
//
// This does NOT measure speedup or accept_rate (those need a real model — see
// the env-gated tests below). It proves the production
// `validate_speculative_request` surface accepts well-formed ngram/draft modes
// and rejects the malformed / unsupported ones, exercising real product code
// rather than a fabricated metric.
// ----------------------------------------------------------------------------

#[test]
fn inf8_spec_validate_request_accepts_and_rejects_real_modes() {
    use handshake_core::model_runtime::{
        llama_cpp::speculative::validate_speculative_request, CancellationToken, GenPrompt,
        GenerateRequest, ModelId, SpeculativeMode,
    };

    fn request_with(id: ModelId, mode: Option<SpeculativeMode>) -> GenerateRequest {
        GenerateRequest {
            id,
            prompt: GenPrompt::new("perf eval probe"),
            sampling: Default::default(),
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: 16,
            stop_sequences: Vec::new(),
            speculative_mode: mode,
            structured_decoding: None,
        }
    }

    let target = ModelId::new_v7();
    let draft = ModelId::new_v7();

    // None → always ok.
    validate_speculative_request(&request_with(target, None)).expect("no speculation is valid");

    // Well-formed ngram → ok.
    validate_speculative_request(&request_with(
        target,
        Some(SpeculativeMode::Ngram {
            lookback: 4,
            max_draft: 8,
        }),
    ))
    .expect("well-formed ngram is valid");

    // ngram lookback 0 → rejected.
    validate_speculative_request(&request_with(
        target,
        Some(SpeculativeMode::Ngram {
            lookback: 0,
            max_draft: 8,
        }),
    ))
    .expect_err("ngram lookback 0 must be rejected");

    // ngram max_draft 0 → rejected.
    validate_speculative_request(&request_with(
        target,
        Some(SpeculativeMode::Ngram {
            lookback: 4,
            max_draft: 0,
        }),
    ))
    .expect_err("ngram max_draft 0 must be rejected");

    // Well-formed draft-model (draft != target) → ok.
    validate_speculative_request(&request_with(
        target,
        Some(SpeculativeMode::DraftModel {
            draft_id: draft,
            max_draft: 8,
        }),
    ))
    .expect("distinct draft model is valid");

    // Draft model == target → rejected.
    validate_speculative_request(&request_with(
        target,
        Some(SpeculativeMode::DraftModel {
            draft_id: target,
            max_draft: 8,
        }),
    ))
    .expect_err("draft model identical to target must be rejected");

    // EAGLE-3 → always unsupported on llama.cpp until PR #18039 lands.
    validate_speculative_request(&request_with(
        target,
        Some(SpeculativeMode::Eagle3 { max_draft: 8 }),
    ))
    .expect_err("eagle3 must be unsupported on llama.cpp today");
}

// ----------------------------------------------------------------------------
// Perf eval (env-gated; real measurement only — never a silent PASS).
//
// These run only when the `llama-cpp-runtime-engine` feature is compiled in
// (otherwise `#[ignore]`). With the feature on:
//   - env unset  -> clean skip (no fixtures, no claim)
//   - env set    -> real load + real generation + real assertion
// There is no path that reports a PASS without measuring the metric.
// ----------------------------------------------------------------------------

/// Contract thresholds (MT-111). Modest regression gates, not benchmark
/// targets; real gains are model-pair specific and operator-tunable.
#[cfg(feature = "llama-cpp-runtime-engine")]
const SPEEDUP_THRESHOLD: f64 = 1.1;
#[cfg(feature = "llama-cpp-runtime-engine")]
const ACCEPT_RATE_THRESHOLD: f64 = 0.30;
/// Fixed eval prompt set + seed for reproducibility.
#[cfg(feature = "llama-cpp-runtime-engine")]
const EVAL_PROMPTS: &[&str] = &[
    "Explain in one sentence why the sky appears blue.",
    "List three prime numbers greater than ten.",
    "Summarise the plot of Romeo and Juliet in two sentences.",
    "What is the capital of France and why is it famous?",
    "Describe the water cycle briefly.",
    "Give a short definition of recursion in programming.",
    "Name two common uses of speculative decoding.",
    "Write a one-line haiku about winter.",
];
#[cfg(feature = "llama-cpp-runtime-engine")]
const EVAL_SEED: u32 = 1234;
#[cfg(feature = "llama-cpp-runtime-engine")]
const EVAL_MAX_TOKENS: u32 = 48;

#[cfg(feature = "llama-cpp-runtime-engine")]
async fn load_target_gguf(
    runtime: &mut handshake_core::model_runtime::llama_cpp::LlamaCppRuntime,
    path: &std::path::Path,
    supports_speculative_draft: bool,
) -> handshake_core::model_runtime::ModelId {
    use handshake_core::model_runtime::{
        llama_cpp::gguf_loader::sha256_file, KvCachePolicy, LoadSpec, ModelCapabilities,
        ModelRuntime, ProviderKind, RuntimeKind, SamplingParams,
    };

    let sha = sha256_file(path).expect("compute sha256 of staged GGUF fixture");
    let spec = LoadSpec {
        artifact_path: path.to_path_buf(),
        sha256_expected: sha,
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities {
            supports_speculative_draft,
            ..Default::default()
        },
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    };
    runtime
        .load(spec)
        .await
        .expect("load staged GGUF fixture through the real LlamaCppRuntime")
}

/// Run generation for every eval prompt and return total tokens emitted and
/// the wall-clock duration. Uses the real `ModelRuntime::generate` stream.
#[cfg(feature = "llama-cpp-runtime-engine")]
async fn run_eval_set(
    runtime: &handshake_core::model_runtime::llama_cpp::LlamaCppRuntime,
    target_id: handshake_core::model_runtime::ModelId,
    speculative_mode: Option<handshake_core::model_runtime::SpeculativeMode>,
) -> (u64, std::time::Duration) {
    use futures::StreamExt;
    use handshake_core::model_runtime::{
        CancellationToken, GenPrompt, GenerateRequest, ModelRuntime, SamplingParams,
    };

    let mut total_tokens = 0_u64;
    let started = std::time::Instant::now();
    for prompt in EVAL_PROMPTS {
        let request = GenerateRequest {
            id: target_id,
            prompt: GenPrompt::new(*prompt),
            sampling: SamplingParams {
                seed: Some(EVAL_SEED),
                ..Default::default()
            },
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: None,
            cancel: CancellationToken::new(),
            max_tokens: EVAL_MAX_TOKENS,
            stop_sequences: Vec::new(),
            speculative_mode: speculative_mode.clone(),
            structured_decoding: None,
        };
        let mut stream = runtime.generate(request);
        while let Some(item) = stream.next().await {
            let token = item.expect("generation token");
            total_tokens += 1;
            if token.finish_reason.is_some() {
                break;
            }
        }
    }
    (total_tokens, started.elapsed())
}

#[tokio::test]
#[cfg_attr(not(feature = "llama-cpp-runtime-engine"), ignore)]
async fn inf8_spec_perf_eval_speedup_at_least_1_1x_with_ngram() {
    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        // Without the engine feature there is no runtime to measure against;
        // the test is #[ignore]d so this body never executes. Present so the
        // file compiles in both feature configurations.
        unreachable!("ignored without llama-cpp-runtime-engine feature");
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        use handshake_core::model_runtime::{
            llama_cpp::LlamaCppRuntime, KvCachePolicy, SpeculativeMode,
        };

        let Some(target_path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(PathBuf::from)
        else {
            eprintln!(
                "[inf8_spec_perf_eval skipped]: HANDSHAKE_TEST_GGUF_PATH unset — no target GGUF \
                 staged, so the ngram speedup gate has nothing to measure. (No PASS is claimed.)"
            );
            return;
        };
        assert!(
            target_path.is_file(),
            "HANDSHAKE_TEST_GGUF_PATH={} is not a file; cannot measure speedup",
            target_path.display()
        );

        let mut runtime = LlamaCppRuntime::new(KvCachePolicy::default());
        // ngram speculation needs the adapter to declare draft support.
        let target_id = load_target_gguf(&mut runtime, &target_path, true).await;

        // Baseline: no speculation.
        let (baseline_tokens, baseline_dur) = run_eval_set(&runtime, target_id, None).await;
        // ngram speculative decoding.
        let (ngram_tokens, ngram_dur) = run_eval_set(
            &runtime,
            target_id,
            Some(SpeculativeMode::Ngram {
                lookback: 4,
                max_draft: 8,
            }),
        )
        .await;

        assert!(
            baseline_tokens > 0 && ngram_tokens > 0,
            "eval produced no tokens (baseline={baseline_tokens}, ngram={ngram_tokens})"
        );
        let baseline_tps = baseline_tokens as f64 / baseline_dur.as_secs_f64().max(f64::MIN_POSITIVE);
        let ngram_tps = ngram_tokens as f64 / ngram_dur.as_secs_f64().max(f64::MIN_POSITIVE);
        let speedup = ngram_tps / baseline_tps.max(f64::MIN_POSITIVE);

        eprintln!(
            "[inf8_spec_perf_eval ngram] baseline={baseline_tps:.2} tok/s, ngram={ngram_tps:.2} \
             tok/s, speedup={speedup:.3}x (threshold {SPEEDUP_THRESHOLD}x)"
        );
        assert!(
            speedup >= SPEEDUP_THRESHOLD,
            "ngram speculative speedup {speedup:.3}x did not clear {SPEEDUP_THRESHOLD}x"
        );
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "llama-cpp-runtime-engine"), ignore)]
async fn inf8_spec_perf_eval_accept_rate_at_least_30_percent_with_draft_model() {
    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        unreachable!("ignored without llama-cpp-runtime-engine feature");
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        use handshake_core::model_runtime::{
            llama_cpp::LlamaCppRuntime, KvCachePolicy, SpeculativeMode,
        };

        let Some(target_path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(PathBuf::from)
        else {
            eprintln!(
                "[inf8_spec_perf_eval skipped]: HANDSHAKE_TEST_GGUF_PATH unset — draft-model \
                 accept-rate gate requires both target + draft GGUF fixtures. (No PASS is claimed.)"
            );
            return;
        };
        let Some(draft_path) = std::env::var_os("HANDSHAKE_TEST_DRAFT_GGUF_PATH").map(PathBuf::from)
        else {
            eprintln!(
                "[inf8_spec_perf_eval skipped]: HANDSHAKE_TEST_DRAFT_GGUF_PATH unset — set to a \
                 compatible draft GGUF to enable the 30% accept-rate gate. (No PASS is claimed.)"
            );
            return;
        };
        assert!(
            target_path.is_file(),
            "HANDSHAKE_TEST_GGUF_PATH={} is not a file",
            target_path.display()
        );
        assert!(
            draft_path.is_file(),
            "HANDSHAKE_TEST_DRAFT_GGUF_PATH={} is not a file",
            draft_path.display()
        );

        // Both models live in the same runtime instance so the generate path
        // can resolve the draft model by id.
        let mut runtime = LlamaCppRuntime::new(KvCachePolicy::default());
        let target_id = load_target_gguf(&mut runtime, &target_path, true).await;
        let draft_id = load_target_gguf(&mut runtime, &draft_path, true).await;

        let (tokens, _dur) = run_eval_set(
            &runtime,
            target_id,
            Some(SpeculativeMode::DraftModel {
                draft_id,
                max_draft: 8,
            }),
        )
        .await;
        assert!(tokens > 0, "draft-model eval produced no tokens");

        // Read the REAL speculative stats accumulated by the generate path and
        // compute accept_rate = accepted_tokens / generated(proposed) tokens.
        let stats = runtime
            .last_speculative_stats(target_id)
            .expect("speculative stats readable")
            .expect("draft-model generation recorded speculative stats");
        assert!(
            stats.generated_tokens > 0,
            "no draft tokens were proposed; accept-rate is undefined (stats={stats:?})"
        );
        let accept_rate = stats.accepted_tokens as f64 / stats.generated_tokens as f64;
        eprintln!(
            "[inf8_spec_perf_eval draft] proposed={}, accepted={}, accept_rate={accept_rate:.3} \
             (threshold {ACCEPT_RATE_THRESHOLD})",
            stats.generated_tokens, stats.accepted_tokens
        );
        assert!(
            accept_rate >= ACCEPT_RATE_THRESHOLD,
            "draft-model accept_rate {accept_rate:.3} did not clear {ACCEPT_RATE_THRESHOLD}"
        );
    }
}
