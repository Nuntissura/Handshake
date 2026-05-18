<!-- GENERATED_PROJECTION: source=.GOV/task_packets/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1/refinement.json | authority=PRIMARY_MACHINE_READABLE_REFINEMENT_JSON | legacy_markdown_policy=SAFETY_NET_ONLY_DO_NOT_COPY_FORWARD per CX-908+CX-914+feedback_no_default_md_files. This .md is LEGACY_COMPAT_PROJECTION_ONLY and exists to satisfy the legacy record-refinement gate. Authority lives in refinement.json + packet.json. -->

## TECHNICAL_REFINEMENT

### METADATA
- WP_ID: WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1
- BASE_WP_ID: WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement
- REFINEMENT_FORMAT_VERSION: 2026-03-15
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- CREATED_AT: 2026-05-18T00:00:00.000Z
- UPDATED_AT: 2026-05-18T02:09:00.000Z
- SPEC_TARGET_RESOLVED: .GOV/spec/SPEC_CURRENT.md -> .GOV/spec/master-spec-v02.186/indexed-spec-manifest.json
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja180520260209
- USER_APPROVAL_EVIDENCE: Operator provided signature in chat on 2026-05-18 after reviewing REFINEMENT_HANDOFF_SUMMARY; globally unique per CX-585C verification (Grep across .GOV and full repo, no prior matches).
- STUB_WP_IDS: WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1 (folds 30 historical WP-1-* stubs per refinement.json kernel004_extension.stub_fold_preservation_map; cluster breakdown A=2, B=4, C=8, D=10, X=6)
- AUTHORITATIVE_CONTRACT_SCHEMA_ID: hsk.refinement_contract@1
- AUTHORITATIVE_CONTRACT_FILE: .GOV/task_packets/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1/refinement.json
- MARKDOWN_PROJECTION_STATUS: LEGACY_COMPAT_PROJECTION_ONLY
- RED_TEAM_REQUIRED: YES
- RED_TEAM_PROFILE: DETERMINISTIC_CONTRACT_MIGRATION_V1

### ACTIVATION_TOPOLOGY_REPAIR
- STATUS: ACTIVATION_READINESS
- IMPLEMENTATION_ROLE: KERNEL_BUILDER
- CODER_COMPATIBILITY_LANE: CODER_A
- WP_VALIDATOR_GATE: DISABLED
- COMMUNICATION_HEALTH_GATE: INTEGRATION_BATCH_REVIEW_BLOCKING
- VALIDATION_TOPOLOGY: INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1
- NOTE: Per kernel-builder protocol Activation Mode, the Kernel Builder served as Activation Manager for KERNEL-004 (refinement skeleton, hydrated refinement, spec v02.186 enrichment, signature capture).
- READINESS_STATUS: REFINEMENT_SIGNED_PACKET_HYDRATED_MT_AUTHORING_NEXT
- FINAL_CHECKS: 180-235 microtask authoring (Stage 4E); FEMS V1 API surface verification (B-FEMS-V1-INTEGRATION-SURFACE Stage 4D); registry sync; Integration Validator batch handoff after MT implementation. No product PASS/FAIL verdict at refinement-signature time.

### GAPS_IDENTIFIED
- KERNEL-004 closes the HANDSHAKE_BUILD_RULES.json v1.1.0 DESIGN_ONLY_WIRING_PENDING gap by wiring HBR enforcement into PACKET_ACCEPTANCE_MATRIX, gov-check, and validator-scan at build-time and handoff-time (Cluster A.1).
- KERNEL-004 closes the absent in-app Diagnostics panel + headless capture API gap that HBR-VIS-* rules assumed existed (Cluster A.2 Visual Debugger).
- KERNEL-004 closes the absent backend manipulation/navigation surface gap; programmatic state inspection + WriteBox-routed mutation lands as the §6.5 Inspector Plane (Cluster A.3).
- KERNEL-004 closes the non-hijacking GUI invariants gap (HBR-QUIET-001 hidden-window mode, HBR-QUIET-002 negative-test framework, HBR-QUIET-003 kernel_process_lifecycle Postgres ledger) per Cluster A.4.
- KERNEL-004 closes the swarm-agent harness gap (HBR-SWARM-001..004 N=8 minimum concurrent governed sessions) per Cluster A.5.
- KERNEL-004 closes the ModelManual currency gap (HBR-MAN-001/002/003 same-commit CI hook + no-context model harness + self-consistency grep) per Cluster A.6.
- KERNEL-004 closes the sandbox-architecture gap left open by KERNEL-003 by landing SandboxAdapter trait + 3 concrete adapters (WSL2+Podman default, WindowsNativeJailAdapter strict, DockerAdapter compat preserving KERNEL-003 integration) per Cluster B.
- KERNEL-004 closes the local-model-boxing gap by shipping ModelRuntime trait + LlamaCppRuntime + CandleRuntime + 8 PRODUCTION inference techniques + opt-in distillation pipeline per Cluster C.
- KERNEL-004 closes the Memory V0+ retrieval + self-improvement loop gap on top of FEMS V1 per Cluster D (Karpathy autoresearch one-iteration pattern, fixed ~30-item HBR test-packet corpus target, 60/20/20 train/dev/holdout + Goodhart sentinel).

### LANDSCAPE_SCAN (prior art / better approaches)
- TIMEBOX: External landscape scan via RW-1 through RW-6 parallel research workstreams completed 2026-05-18.
- SEARCH_SCOPE: 10+ sandbox candidates (Docker Desktop, Podman, nerdctl/containerd, Firecracker microVM, gVisor, WSL2 namespaces, Bubblewrap, Wasmtime/WASM, Deno sandbox, Handshake-native Rust+OS-primitives jail); 4 engine candidates (llama.cpp via llama-cpp-2 crate, candle, mistral.rs, others); 9 inference techniques; visual debugger substrates (Playwright, CDP-direct, Tauri-native devtools); non-hijacking GUI patterns (Tauri/Electron test focus discipline); self-improvement loop patterns (Karpathy autoresearch, DSPy MIPROv2, SWE-Bench-Pro, TextGrad).
- REFERENCES: refinement.json `kernel004_extension.research_basis_plan[*].synthesis_summary` for full RW-1..RW-6 synthesis; operator_decisions_locked block for E-1..E-5 + SANDBOX + Q-* decisions.
- PATTERNS_EXTRACTED: ADOPT WSL2+Podman as default sandbox (scored 32/40 highest viable on Windows); ADOPT WindowsNativeJailAdapter wrapping existing Rust crate (codex-windows-sandbox or rappct; never hand-rolled per OpenAI Codex 2026 pattern); PRESERVE DockerAdapter as compat-only-not-default to keep KERNEL-003 integration intact; ADOPT multi-engine adapter pattern (llama-cpp-2 default + candle for hook-requiring techniques); ADOPT Playwright via CDP against WebView2 for visual debugger with read-only inspector plane on localhost; ADOPT Tauri hidden-window config (visible=false focus=false focusable=false skipTaskbar=true alwaysOnBottom=true) + SetWinEventHook focus audit + WH_KEYBOARD_LL injection negative test; ADOPT Karpathy autoresearch one-iteration target -> eval -> propose -> review -> accept/reject; REJECT Firecracker/Bubblewrap/gVisor (Linux host only); REJECT Wasmtime (no native CUDA); REJECT Deno (--allow-ffi defeats the boundary); REJECT TextGrad (gradient surface incompatible with edit-by-text editable surface); REJECT Ollama/LM Studio as runtime authority (ExternalEngineImport pass-through lane only).
- DECISIONS ADOPT/ADAPT/REJECT: ADOPT v02.186 as authority bundle, ADOPT 3-adapter sandbox pattern (E-SANDBOX), ADOPT 2-adapter ModelRuntime pattern (E-3), ADOPT 8-of-9 PRODUCTION inference techniques (E-2 full feature parity Subquadratic), DEFER MoD to spec Phase 3 roadmap addition (E-1), DEFER EAGLE-3 to upstream llama.cpp PR #18039 merge (E-4), CONSTRAIN abliteration to offline-only (E-5).
- LICENSE/IP_NOTES: WindowsNativeJailAdapter wraps an existing Rust crate (codex-windows-sandbox or rappct); license audit MT required before adoption per R-10. llama-cpp-2 crate license (MIT) and candle license (MIT or Apache-2.0) verified compatible.
- SPEC_IMPACT: YES
- SPEC_IMPACT_REASON: 7 enrichment topics applied across modules 03, 04, 05, 06, 07-6, 10, 12 plus Codex CX-131; v02.185 -> v02.186 bundle bump per CX-105C copy-first; B-SPEC-ENRICHMENT resolved 2026-05-18.

### RESEARCH_CURRENCY
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_REASON: KERNEL-004 selects new external engines (llama-cpp-2, candle), new external sandbox technology (WSL2+Podman, codex-windows-sandbox/rappct), and new self-improvement loop reference patterns (Karpathy autoresearch, DSPy MIPROv2, SWE-Bench-Pro); current external evidence required.
- SOURCE_MAX_AGE_DAYS: 365
- SOURCE_LOG:
  - BIG_TECH: DeepMind Raposo et al. 2024 arXiv:2404.02258 Mixture-of-Depths paper.
  - UNIVERSITY: NeurIPS 2025 Mixture-of-Recursions paper (raymin0223/mixture_of_recursions).
  - PAPER: Karpathy autoresearch one-iteration target -> eval -> propose -> review -> accept/reject loop pattern reference.
  - GITHUB: utilityai/llama-cpp-rs (llama-cpp-2 Rust binding to llama.cpp; MIT license).
  - GITHUB: huggingface/candle (Hugging Face Rust ML framework; MIT/Apache-2.0).
  - GITHUB: sramshetty/mixture-of-depths unofficial training-focused MoD implementation (research code only).
  - OSS_DOC: Tauri WindowBuilder hidden-window config documentation + WebView2 CDP via Playwright reference.
  - GITHUB: raymin0223/mixture_of_recursions NeurIPS 2025 reference implementation.
  - GITHUB: llama.cpp PR #18039 (EAGLE-3 upstream merge in progress).
- RESEARCH_SYNTHESIS: All six research workstreams (RW-1 sandbox candidates, RW-2 engine candidates, RW-3 inference techniques, RW-4 visual debugger substrates, RW-5 non-hijacking GUI patterns, RW-6 self-improvement loop patterns) complete. Concrete insights: (1) WSL2+Podman scores 32/40 highest viable on Windows for the default sandbox; (2) llama-cpp-2 supplies production-ready transformer GGUF + LoRA hot-swap + KV cache + ngram/draft self-speculative decoding while candle supplies the hook-requiring substrate needed for activation steering / refusal vector / CAA / abliteration prep / subquadratic Mamba2/RWKV; (3) Playwright via CDP against WebView2 + per-test WEBVIEW2_USER_DATA_FOLDER isolation is the only field-tested debugger path that satisfies non-hijacking GUI invariants; (4) Karpathy autoresearch one-iteration loop with 60/20/20 train/dev/holdout encryption and a Goodhart sentinel is the safest first self-improvement loop. Operator decisions E-1 through E-5 + SANDBOX + Q-SELF-IMPROVE-TARGET + Q-DISTILL-CORPUS + Q-PROCESS-LEDGER-SCOPE + Q-MODELMANUAL-LOCATION captured in operator_decisions_locked extension block.
- RESEARCH_GAPS_TO_TRACK: WindowsNativeJailAdapter crate license audit (R-10) is a Stage 4E pre-implementation MT; FEMS V1 API drift (R-9, B-FEMS-V1-INTEGRATION-SURFACE) is a Stage 4D verification step before cluster D MT authoring; MoD production engine path on Windows+Rust may materialize after v02.186 (revisit triggers documented at §4.7.2 + §6.8).
- RESEARCH_CURRENCY_VERDICT: CURRENT

### RESEARCH_DEPTH
- ADOPT_PATTERNS:
  - WSL2+Podman rootless containerization with bind-mounted large GGUF models for near-native GPU pass-through (RW-1).
  - llama-cpp-2 Rust crate as default ModelRuntime adapter for transformer GGUF + LoRA hot-swap + KV cache + ngram/draft self-speculative decoding (RW-2 + RW-3).
  - candle-core + candle-transformers as hook-requiring substrate for activation steering + refusal vector + CAA + abliteration prep + subquadratic Mamba2/RWKV (RW-2 + RW-3).
  - Playwright via CDP against WebView2 with per-test WEBVIEW2_USER_DATA_FOLDER isolation (RW-4).
  - Tauri WindowBuilder hidden-window config + SetWinEventHook focus audit via wineventhook Rust crate + WH_KEYBOARD_LL low-level hook for LLKHF_INJECTED negative test (RW-5).
  - Karpathy autoresearch one-iteration target -> eval -> propose -> review -> accept/reject loop pattern with 60/20/20 train/dev/holdout encryption (RW-6).
- ADAPT_PATTERNS:
  - TransformerLens-style activation hook pattern in Rust over candle Tensor::register_hook-equivalent surface (adapted from Python TransformerLens for activation steering / refusal vector / CAA).
  - DSPy MIPROv2 multi-prompt optimization adapted to ModelManual capsule text + retrieval policy params editable surface.
  - SWE-Bench-Pro held-out task eval adapted to fixed ~30-item HBR test-packet corpus with multi-metric promotion floor (dev PASS + latency p95 + capsule bytes + holdout PASS).
  - kernel_process_lifecycle Postgres table adapted from Windows ETW + PROC_THREAD_ATTRIBUTE_PARENT_PROCESS attribution patterns.
- REJECT_PATTERNS:
  - REJECT Firecracker/Bubblewrap/gVisor (Linux host only, unsuitable for Windows host parity).
  - REJECT Wasmtime (no native CUDA).
  - REJECT Deno sandbox (--allow-ffi defeats the boundary).
  - REJECT mistral.rs PagedAttention (does not run on Windows).
  - REJECT Ollama daemon and LM Studio as authoritative ModelRuntime (4.6 normative path; 4.2.4 rewritten in v02.186 to remove Ollama-as-primary recommendation).
  - REJECT hand-rolling Win32 Job Objects + AppContainer + Restricted Tokens (use existing Rust crate per OpenAI Codex 2026 pattern).
  - REJECT TextGrad for V0 self-improvement loop (requires gradient surface incompatible with current edit-by-text editable surface).
- RESEARCH_DEPTH_VERDICT: PASS

### GITHUB_PROJECT_SCOUTING
- SEARCH_QUERIES:
  - llama-cpp-2 rust
  - candle-core candle-transformers
  - mistral.rs
  - codex-windows-sandbox
  - rappct windows sandbox rust
  - tauri-driver
  - wineventhook rust crate
  - sramshetty mixture-of-depths
  - raymin0223 mixture_of_recursions
  - Karpathy autoresearch
  - DSPy MIPROv2
  - SWE-Bench-Pro
- MATCHED_PROJECTS:
  - utilityai/llama-cpp-rs (llama-cpp-2 Rust binding to llama.cpp; MIT; default ModelRuntime adapter).
  - huggingface/candle (Hugging Face Rust ML framework; MIT/Apache-2.0; CandleRuntime).
  - codex-windows-sandbox + rappct (Windows Job Object + AppContainer + Restricted Token Rust crates; license audit pending per R-10).
  - tauri-apps/tauri-driver (Tauri WebDriver; CI fallback).
  - tauri-apps/wineventhook-rs (Rust crate for SetWinEventHook).
  - sramshetty/mixture-of-depths (unofficial training-focused MoD impl; research code only, not production).
  - raymin0223/mixture_of_recursions (NeurIPS 2025).
  - ggerganov/llama.cpp PR #18039 (EAGLE-3 upstream merge, not yet landed; KERNEL-004 ships ngram/draft self-spec today per E-4).
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS

### FLIGHT_RECORDER_INTERACTION
- Process lifecycle events (START + STOP rows per AC-PROCESS-LEDGER-SCOPE) emit to kernel_process_lifecycle Postgres table for every Handshake-spawned process (model engine, mechanical job, ASR worker, ComfyUI worker, sandbox container, plugin process, helper subprocess); reclaim hooks fire on session close, failure, staleness, operator cancel per CX-503D + CX-212G and write STOP rows with exit_code=-1.
- FR-EVT-LEDGER-OVERFLOW receipt emits on bounded mpsc channel saturation (perf-mitigated batched async writes); Diagnostics panel surfaces degraded-mode flag (never silent drop) per AC-PROCESS-LEDGER-PERF (§5.7.3).
- FR-EVT-LLM-INFER-* event family covers per-technique inference observability (LoRA mount/unmount, KV cache state, activation steering vector application, draft model speculative tokens, abliteration tool job lifecycle, subquadratic state-vector persistence/restore) per §4.7.
- FR-EVT-DISTILL-PII-DETECT receipt blocks promotion on PII scan failure per AC-DISTILL-CONTENT-REVIEW (§4.8.2); FR-EVT-DISTILL-LOOP-CAP receipt fires when HBR-SWARM-002 loop counter cap hits per AC-DISTILL-LOOP-SAFEGUARDS (§4.8.3).
- HBR_HANDOFF_GATE event emits on every governed inter-role transition (refinement->coder, coder->WP_VALIDATOR, WP_VALIDATOR->INTEGRATION_VALIDATOR, INTEGRATION_VALIDATOR->ORCHESTRATOR) per AC-HBR-HANDOFF-GATE (§5.6.2); failed HBR rule blocks the handoff. HBR_VIOLATION receipts use CX-130 schema fields (hbr_id, wp_id, mt_id, role, evaluation_point=build|handoff, evidence_pointer, violation_class).
- Visual regression receipts fire on baseline-hash drift per AC-VISUAL-DEBUGGER-MATRIX (§6.4.4); focus-audit ledger flags any foreground transition resolving to Handshake's pid or to any process spawned by Handshake per ProcessOwnershipLedger.

### RED_TEAM_ADVISORY
- RISK R-1 (Scope-consuming-product): KERNEL-004 is the largest WP in project history; risks the harness-consumes-product failure mode. CONTROL: Hard scope limit at the 30-stub-fold boundary; refinement explicitly forbids adding scope outside the folded stubs.
- RISK R-2 (PRODUCTION-tier research-frontier risk for MoD/Subquadratic): may have no PRODUCTION engine path in 2026. CONTROL: RW-3 resolved feasibility pre-signature; MoD spec-deferred per E-1; Subquadratic locked at full feature parity per E-2 with multi-quarter timeline disclosed.
- RISK R-3 (Sandbox-architecture-flip-orphans-KERNEL-003): non-Docker pick could strand KERNEL-003 Docker integration. CONTROL: DockerAdapter preserved as compat-only-not-default; explicit migration MTs route existing KERNEL-003 callers under SandboxAdapter trait.
- RISK R-4 (Spec-enrichment-becomes-its-own-project): v02.186 enrichment is wide. CONTROL: single coherent bundle bump per CX-105C; no incremental enrichment drift; B-SPEC-ENRICHMENT resolved 2026-05-18.
- RISK R-5 (LM-Studio/Ollama-lockout): no-third-party-wrapping invariant could lock out easiest local-model story. CONTROL: ExternalEngineImport non-authoritative pass-through lane (§3.6.4); operator may point at model file or local OpenAI-compatible HTTP endpoint without LM Studio or Ollama as runtime authority.
- RISK R-6 (Visual-debugger-parallel-mutation-path): debugger built outside KernelActionCatalogV1/WriteBoxV1 becomes a second authority-bypass write path. CONTROL: 3-layer parallel-mutation controls (compile-time trait separation via inspector_read crate with no &mut self methods; runtime localhost+secret+feature-flag binding; audit log via single /inspector/v1/replay-drive endpoint accepting only catalog action id + signed WriteBoxV1 envelope).
- RISK R-7 (Self-improvement-overfit): loop targeted at one measurable check will overfit unless held-out corpus guards promotion. CONTROL: 60/20/20 train/dev/holdout encrypted-at-rest + multi-metric promotion floor (dev PASS + latency p95 + capsule bytes + holdout PASS) + Goodhart sentinel (auto-pause if dev/holdout gap widens monotonically 3 iterations) + HBR-SWARM-002 loop counter cap + PromotionGate operator review.
- RISK R-8 (MoD-Subquad-feasibility): resolved by spec-deferral (MoD §6.8) and operator E-2 full-parity acceptance (Subquadratic).
- RISK R-9 (FEMS-V1-API-drift): B-FEMS-V1-INTEGRATION-SURFACE remains BLOCKING for cluster D MT authoring only; verification pass in Stage 4D before cluster D MT authoring begins.
- RISK R-10 (WindowsNativeJailAdapter-crate-license): license audit MT required before adopting codex-windows-sandbox or rappct crate as the strict-isolation adapter substrate.
- RISK R-11 (Process-ledger-perf-under-swarm): mitigated by batched async writes + per-process metadata cap + async write path + FR-EVT-LEDGER-OVERFLOW degraded mode (§5.7.3).
- RISK R-12 (Mojibake-in-spec-module-04): pre-existing UTF-8 double-encoding in legacy §4.2 content; Stage 2 subagent worked around it; cleanup is non-blocking governance debt.
- RISK R-13 (AC-cross-reference-drift): 52 AC-* IDs in spec are cross-referenced from refinement.json acceptance_criteria; renames or splits must update both surfaces in the same change.

### PRIMITIVES (traits/structs/enums)
- PRIMITIVES_TOUCHED (IDs):
  - PRIM-CandleRuntime
  - PRIM-DiagnosticsPanel
  - PRIM-DistillationCandidateReview
  - PRIM-DockerAdapter
  - PRIM-ExecPolicyExtended
  - PRIM-HBRGate
  - PRIM-HBRViolationReceipt
  - PRIM-InferenceLabUI
  - PRIM-LlamaCppRuntime
  - PRIM-LocalModelAdapter
  - PRIM-MemoryCapsule
  - PRIM-ModelManual
  - PRIM-ModelRuntime
  - PRIM-ModelRuntimeControlPanel
  - PRIM-ProcessOwnershipLedger
  - PRIM-SandboxAdapter
  - PRIM-SelfImprovementLoop
  - PRIM-SwarmTestHarness
  - PRIM-WSL2PodmanAdapter
  - PRIM-WindowsNativeJailAdapter
- PRIMITIVES_EXPOSED (IDs):
  - PRIM-CandleRuntime
  - PRIM-DiagnosticsPanel
  - PRIM-DistillationCandidateReview
  - PRIM-DockerAdapter
  - PRIM-ExecPolicyExtended
  - PRIM-HBRGate
  - PRIM-HBRViolationReceipt
  - PRIM-InferenceLabUI
  - PRIM-LlamaCppRuntime
  - PRIM-LocalModelAdapter
  - PRIM-MemoryCapsule
  - PRIM-ModelManual
  - PRIM-ModelRuntime
  - PRIM-ModelRuntimeControlPanel
  - PRIM-ProcessOwnershipLedger
  - PRIM-SandboxAdapter
  - PRIM-SelfImprovementLoop
  - PRIM-SwarmTestHarness
  - PRIM-WSL2PodmanAdapter
  - PRIM-WindowsNativeJailAdapter
- PRIMITIVES_CREATED (IDs):
  - PRIM-CandleRuntime
  - PRIM-DiagnosticsPanel
  - PRIM-DistillationCandidateReview
  - PRIM-DockerAdapter
  - PRIM-ExecPolicyExtended
  - PRIM-HBRGate
  - PRIM-HBRViolationReceipt
  - PRIM-InferenceLabUI
  - PRIM-LlamaCppRuntime
  - PRIM-LocalModelAdapter
  - PRIM-MemoryCapsule
  - PRIM-ModelManual
  - PRIM-ModelRuntime
  - PRIM-ModelRuntimeControlPanel
  - PRIM-ProcessOwnershipLedger
  - PRIM-SandboxAdapter
  - PRIM-SelfImprovementLoop
  - PRIM-SwarmTestHarness
  - PRIM-WSL2PodmanAdapter
  - PRIM-WindowsNativeJailAdapter
- PRIMITIVES_NEW_OR_UPDATED (IDs):
  - PRIM-CandleRuntime
  - PRIM-DiagnosticsPanel
  - PRIM-DistillationCandidateReview
  - PRIM-DockerAdapter
  - PRIM-ExecPolicyExtended
  - PRIM-HBRGate
  - PRIM-HBRViolationReceipt
  - PRIM-InferenceLabUI
  - PRIM-LlamaCppRuntime
  - PRIM-LocalModelAdapter
  - PRIM-MemoryCapsule
  - PRIM-ModelManual
  - PRIM-ModelRuntime
  - PRIM-ModelRuntimeControlPanel
  - PRIM-ProcessOwnershipLedger
  - PRIM-SandboxAdapter
  - PRIM-SelfImprovementLoop
  - PRIM-SwarmTestHarness
  - PRIM-WSL2PodmanAdapter
  - PRIM-WindowsNativeJailAdapter
- NOTES:
  - 20 new primitives added to spec 12.4 per SPEC-ENRICH-6-EOF-Appendices APPLIED 2026-05-18; see refinement.json for per-primitive module/spec_anchor mapping.

### PRIMITIVE_INDEX (Appendix 12.4)
- PRIMITIVE_INDEX_ACTION: UPDATED
- PRIMITIVE_INDEX_REASON: SPEC-ENRICH-6-EOF-Appendices APPLIED 2026-05-18 in v02.186 bundle adds 20 new primitives to §12.4 HS-APPX-PRIMITIVE-TOOL-TECH-MATRIX covering ModelRuntime/LocalModelAdapter/LlamaCppRuntime/CandleRuntime/SandboxAdapter + 3 sandbox adapters/ProcessOwnershipLedger/HBRGate/HBRViolationReceipt/MemoryCapsule/SelfImprovementLoop/DistillationCandidateReview/DiagnosticsPanel/ModelRuntimeControlPanel/InferenceLabUI/ModelManual/SwarmTestHarness/ExecPolicyExtended.
- PRIMITIVE_INDEX_UPDATE_NOTES: All 20 new PRIM IDs cross-link to the corresponding AC-* IDs in refinement.acceptance_criteria; no orphaned primitives. See refinement.json `kernel004_extension.spec_enrichment_topics[5]` and `approved_spec_enrichment[5]`.
- HIGH_SIGNAL_ORPHAN_PRIMITIVES_DISCOVERED: NONE

### APPENDIX_MAINTENANCE
- FEATURE_REGISTRY_ACTION: UPDATED
- FEATURE_REGISTRY_REASON: 7 enrichment topics applied across modules 03, 04, 05, 06, 07-6, 10, 12 plus Codex CX-131; manifest/INDEX/changelog/SPEC_CURRENT updated; v02.185 archived; spec-current-check PASS.
- UI_GUIDANCE_ACTION: UPDATED
- UI_GUIDANCE_REASON: Spec 10.12 Diagnostics Panel, 10.13 ModelRuntime Control Panel, 10.14 Inference Lab UI, and 10.15 ModelManual Surface all added in v02.186 bundle.
- INTERACTION_MATRIX_ACTION: UPDATED
- INTERACTION_MATRIX_REASON: Spec 12.6 adds 12 new IMX edges IMX-138..149 covering ModelRuntime <-> SandboxAdapter, MemoryCapsule <-> FEMS V1, SelfImprovementLoop <-> PromotionGate + EventLedger, HBRGate <-> PACKET_ACCEPTANCE_MATRIX + ValidationRunner.
- APPENDIX_MAINTENANCE_NOTES:
  - 52 AC-* IDs in spec are cross-referenced from refinement.acceptance_criteria.
  - R-13 mitigation requires renames/splits to update both surfaces in the same change.
  - 20 new primitives added to 12.4 PRIMITIVE_TOOL_TECH_MATRIX per SPEC-ENRICH-6.
  - 12 new IMX edges IMX-138..149 added to 12.6 INTERACTION_MATRIX per SPEC-ENRICH-6.
- APPENDIX_MAINTENANCE_VERDICT: NEEDS_SPEC_UPDATE

### MECHANICAL_ENGINE_ALIGNMENT
- ENGINE: Spatial | ENGINE_ID: engine.spatial | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Machinist | ENGINE_ID: engine.machinist | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Physics | ENGINE_ID: engine.physics | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Simulation | ENGINE_ID: engine.simulation | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Hardware | ENGINE_ID: engine.hardware | STATUS: TOUCHED | NOTES: KERNEL-004 touches GPU pass-through under WSL2PodmanAdapter and CandleRuntime CUDA backend.
- ENGINE: Director | ENGINE_ID: engine.director | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Composer | ENGINE_ID: engine.composer | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Artist | ENGINE_ID: engine.artist | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Publisher | ENGINE_ID: engine.publisher | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Sous Chef | ENGINE_ID: engine.sous_chef | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Food Safety | ENGINE_ID: engine.food_safety | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Logistics | ENGINE_ID: engine.logistics | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- ENGINE: Archivist | ENGINE_ID: engine.archivist | STATUS: TOUCHED | NOTES: ProcessOwnershipLedger + distillation candidate review preserve durable lifecycle artifacts.
- ENGINE: Librarian | ENGINE_ID: engine.librarian | STATUS: TOUCHED | NOTES: ModelManual + MemoryCapsule retrieval policy and audit log.
- ENGINE: Analyst | ENGINE_ID: engine.analyst | STATUS: TOUCHED | NOTES: SelfImprovementLoop measures validator PASS rate on fixed HBR test-packet corpus; Goodhart sentinel detection.
- ENGINE: Wrangler | ENGINE_ID: engine.wrangler | STATUS: TOUCHED | NOTES: SwarmTestHarness manages N concurrent governed sessions, lease contention, cancellation propagation.
- ENGINE: DBA | ENGINE_ID: engine.dba | STATUS: TOUCHED | NOTES: kernel_process_lifecycle Postgres table + batched async write path + perf-mitigated channel saturation handling.
- ENGINE: Sovereign | ENGINE_ID: engine.sovereign | STATUS: TOUCHED | NOTES: HBRGate enforces build-time and handoff-time policy; ProcessOwnershipLedger + SandboxAdapter authorize spawn paths.
- ENGINE: Guide | ENGINE_ID: engine.guide | STATUS: TOUCHED | NOTES: ModelManual no-context model harness + DiagnosticsPanel for operator-facing inspection.
- ENGINE: Context | ENGINE_ID: engine.context | STATUS: TOUCHED | NOTES: MemoryCapsule bounded retrieval + capsule generation policy per task type.
- ENGINE: Version | ENGINE_ID: engine.version | STATUS: TOUCHED | NOTES: Spec v02.186 bundle bump via CX-105C copy-first; ManualVersion bump tied to wired-surface diffs.
- ENGINE: Sandbox | ENGINE_ID: engine.sandbox | STATUS: TOUCHED | NOTES: SandboxAdapter trait + 3 adapters (WSL2Podman default, WindowsNativeJail strict, Docker compat) shared by model-written code, local-model inference, validation runners, swarm-agent harness.
- MECHANICAL_ENGINE_ALIGNMENT_VERDICT: OK
- ENGINES KERNEL-004 DIRECTLY IMPLEMENTS OR ALIGNS: ModelRuntime + SandboxAdapter + ProcessOwnershipLedger + HBRGate + SelfImprovementLoop + MemoryCapsule + DistillationCandidateReview + SwarmTestHarness + ModelManual + DiagnosticsPanel + InferenceLabUI + ModelRuntimeControlPanel (12 of the 22 spec engines).

### PILLAR_ALIGNMENT
- PILLAR: Flight Recorder | STATUS: TOUCHED | NOTES: FR-EVT-LLM-INFER-*, FR-EVT-LEDGER-OVERFLOW, FR-EVT-DISTILL-PII-DETECT, FR-EVT-DISTILL-LOOP-CAP, HBR_HANDOFF_GATE, HBR_VIOLATION receipts.
- PILLAR: Calendar | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- PILLAR: Monaco | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- PILLAR: Word clone | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- PILLAR: Excel clone | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- PILLAR: Locus | STATUS: TOUCHED | NOTES: Swarm-agent harness + ProcessOwnershipLedger visibility in work-graph projection.
- PILLAR: Loom | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- PILLAR: Work packets (product, not repo) | STATUS: TOUCHED | NOTES: KERNEL-004 folds 30 historical WP-1-* stubs into preserved typed contracts.
- PILLAR: Task board (product, not repo) | STATUS: TOUCHED | NOTES: KERNEL-004 moves from stub to active packet at Ready-for-Dev.
- PILLAR: MicroTask | STATUS: TOUCHED | NOTES: 180-235 MTs estimated; Stage 4E authors MT contracts post-signature.
- PILLAR: Command Center | STATUS: TOUCHED | NOTES: §10.12 Diagnostics Panel + §10.13 ModelRuntime Control Panel + §10.14 Inference Lab UI surfaces.
- PILLAR: Execution / Job Runtime | STATUS: TOUCHED | NOTES: ModelRuntime + SandboxAdapter + ProcessOwnershipLedger + SwarmTestHarness substrate.
- PILLAR: Spec to prompt | STATUS: TOUCHED | NOTES: ModelManual no-context model harness HBR-MAN-002.
- PILLAR: SQL to PostgreSQL shift readiness | STATUS: TOUCHED | NOTES: kernel_process_lifecycle Postgres table; no SQLite per CX-503R.
- PILLAR: LLM-friendly data | STATUS: TOUCHED | NOTES: MemoryCapsule + DistillationCandidateReview + ModelManual typed structures.
- PILLAR: Stage | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- PILLAR: Studio | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- PILLAR: Atelier/Lens | STATUS: NOT_TOUCHED | NOTES: Outside KERNEL-004 scope.
- PILLAR: Skill distillation / LoRA | STATUS: TOUCHED | NOTES: Distillation pipeline + LoRA hot-swap + opt-in corpus + content review.
- PILLAR: ACE | STATUS: TOUCHED | NOTES: MemoryCapsule preserves ACE trace refs as evidence handles where present.
- PILLAR: RAG | STATUS: TOUCHED | NOTES: RAG retrieval mode policy folded from WP-1-RAG-Retrieval-Mode-Policy under cluster D.
- PILLAR_ALIGNMENT_VERDICT: OK

### PILLAR_DECOMPOSITION
- PILLAR: Flight Recorder | CAPABILITY_SLICE: HBR and inference observability | SUBFEATURES: HBR_HANDOFF_GATE event, HBR_VIOLATION receipts, FR-EVT-LLM-INFER-*, FR-EVT-LEDGER-OVERFLOW, FR-EVT-DISTILL-* | PRIMITIVES_FEATURES: PRIM-HBRGate, PRIM-HBRViolationReceipt | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: WP-1-Session-Observability-Spans-FR (folded into Cluster X.4)
- PILLAR: Locus | CAPABILITY_SLICE: Process lifecycle work-graph visibility | SUBFEATURES: kernel_process_lifecycle Postgres table, swarm session visibility | PRIMITIVES_FEATURES: PRIM-ProcessOwnershipLedger, PRIM-SwarmTestHarness | MECHANICAL: engine.wrangler | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE
- PILLAR: Command Center | CAPABILITY_SLICE: DiagnosticsPanel + ModelRuntimeControlPanel + InferenceLabUI | SUBFEATURES: live WebView2 screenshot stream, DOM tree, Playwright command history, KV-cache occupancy display, LoRA stack composer, steering vector intensity pickers | PRIMITIVES_FEATURES: PRIM-DiagnosticsPanel, PRIM-ModelRuntimeControlPanel, PRIM-InferenceLabUI | MECHANICAL: engine.guide | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: WP-1-Visual-Debugging-Loop (folded into Cluster A.2)
- PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: SandboxAdapter + ModelRuntime substrate | SUBFEATURES: trait abstraction, 3 sandbox adapters, 2 model runtimes, 8 PRODUCTION inference techniques | PRIMITIVES_FEATURES: PRIM-SandboxAdapter, PRIM-ModelRuntime | MECHANICAL: engine.sandbox + engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: multiple folded under Cluster B + C
- PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: Distillation pipeline + LoRA hot-swap | SUBFEATURES: opt-in corpus, PII scan, license tagging, dedup, EventLedger replay -> teacher/student PEFT distillation -> distilled LoRA mountable | PRIMITIVES_FEATURES: PRIM-DistillationCandidateReview, PRIM-MemoryCapsule | MECHANICAL: engine.librarian + engine.analyst | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: WP-1-Distillation-v2 (folded into Cluster C.3)
- PILLAR: LLM-friendly data | CAPABILITY_SLICE: MemoryCapsule + ModelManual | SUBFEATURES: typed retrieval artifact, capsule generation policy per task type, audit log, ModelManual typed Rust structs + Tauri IPC | PRIMITIVES_FEATURES: PRIM-MemoryCapsule, PRIM-ModelManual | MECHANICAL: engine.context + engine.guide | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: WP-1-FEMS-* (folded into Cluster D)
- PILLAR_DECOMPOSITION_VERDICT: OK

### EXECUTION_RUNTIME_ALIGNMENT
- Capability: Model inference (load/generate/score/embed) | JobModel: WORKFLOW | Workflow: ModelRuntime.generate streaming TokenStream through chosen adapter | ToolSurface: MODEL_RUNTIME_TRAIT | ModelExposure: BOTH | CommandCenter: VISIBLE (§10.13 + §10.14) | FlightRecorder: FR-EVT-LLM-INFER-* | Locus: VISIBLE (per session) | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Engine-agnostic trait surface; callers never see llama_cpp_2::* or candle_core::* types.
- Capability: Sandbox process spawning | JobModel: MECHANICAL_TOOL | Workflow: SandboxAdapter.spawn through chosen adapter with bound capabilities | ToolSurface: SANDBOX_ADAPTER_TRAIT | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: process lifecycle events | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: 3 adapters operational; operator selects sandbox lane per WP.
- Capability: HBR gate enforcement | JobModel: WORKFLOW | Workflow: hbr-matrix-check.mjs gov-check sub-check at build time + HBR_HANDOFF_GATE event at handoff time | ToolSurface: HBR_GATE | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: HBR_VIOLATION receipts | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Typed receipts per CX-130 schema, free-form prose never the wire contract.
- Capability: Visual debugger run + capture | JobModel: ARTIFACT_PIPELINE | Workflow: Playwright via CDP against WebView2 + per-step screenshots + baseline-comparison overlay | ToolSurface: VISUAL_DEBUGGER | ModelExposure: BOTH | CommandCenter: VISIBLE (§10.12) | FlightRecorder: visual-regression receipts | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: WP-1-Visual-Debugging-Loop (folded) | Notes: Read-only inspector plane; mutations routed through /inspector/v1/replay-drive with signed WriteBoxV1 envelope only.
- Capability: Self-improvement loop iteration | JobModel: WORKFLOW | Workflow: Karpathy autoresearch target -> eval -> propose -> review -> accept/reject on fixed HBR test-packet corpus | ToolSurface: SELF_IMPROVEMENT_LOOP | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: FR-EVT-DISTILL-LOOP-CAP | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY + ENCRYPTED_HOLDOUT | Resolution: IN_THIS_WP | Stub: NONE | Notes: PromotionGate operator review required per promotion; Goodhart sentinel auto-pauses on 3 monotonic widening iterations.
- Capability: Swarm-agent harness | JobModel: WORKFLOW | Workflow: N concurrent governed sessions against same Handshake instance | ToolSurface: SWARM_TEST_HARNESS | ModelExposure: BOTH | CommandCenter: VISIBLE | FlightRecorder: lease/contention events | Locus: VISIBLE | StoragePosture: POSTGRES_ONLY | Resolution: IN_THIS_WP | Stub: NONE | Notes: N=8 minimum per HBR-SWARM-001/002/003/004.
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK

### PRIMITIVE_MATRIX
- MATRIX_SCAN_TIMEBOX: v02.186 12.6 update applied 2026-05-18 per SPEC-ENRICH-6-EOF-Appendices.
- MATRIX_SCAN_NOTES: 12 new interaction-matrix edges added to 12.6 covering ModelRuntime <-> SandboxAdapter (process boxing), MemoryCapsule <-> FEMS V1 (retrieval), SelfImprovementLoop <-> PromotionGate + EventLedger (promotion bridge), HBRGate <-> PACKET_ACCEPTANCE_MATRIX + ValidationRunner (gate evaluation), plus 8 additional edges spanning ProcessOwnershipLedger interactions, DiagnosticsPanel projection edges, InferenceLabUI knob persistence to ExecPolicyExtended, and SwarmTestHarness lease/contention edges.
- IMX_EDGE_IDS_ADDED_OR_UPDATED: IMX-138, IMX-139, IMX-140, IMX-141, IMX-142, IMX-143, IMX-144, IMX-145, IMX-146, IMX-147, IMX-148, IMX-149
- Edge: ModelRuntime <-> SandboxAdapter | IMX_ID: IMX-138 | NOTES: process boxing for model engines.
- Edge: MemoryCapsule <-> FEMS V1 | IMX_ID: IMX-139 | NOTES: retrieval surface integration.
- Edge: SelfImprovementLoop <-> PromotionGate | IMX_ID: IMX-140 | NOTES: operator-review promotion bridge.
- Edge: SelfImprovementLoop <-> EventLedger | IMX_ID: IMX-141 | NOTES: durable iteration receipts.
- Edge: HBRGate <-> PACKET_ACCEPTANCE_MATRIX | IMX_ID: IMX-142 | NOTES: build-time gate evaluation.
- Edge: HBRGate <-> ValidationRunner | IMX_ID: IMX-143 | NOTES: handoff-time gate evaluation.
- Edge: ProcessOwnershipLedger <-> SandboxAdapter | IMX_ID: IMX-144 | NOTES: spawn-time ledger registration.
- Edge: DiagnosticsPanel <-> ProcessOwnershipLedger | IMX_ID: IMX-145 | NOTES: focus-audit ledger projection.
- Edge: InferenceLabUI <-> ExecPolicyExtended | IMX_ID: IMX-146 | NOTES: knob persistence to settings.
- Edge: SwarmTestHarness <-> SessionBroker | IMX_ID: IMX-147 | NOTES: lease/contention edges.
- Edge: DistillationCandidateReview <-> EventLedger | IMX_ID: IMX-148 | NOTES: opt-in corpus replay.
- Edge: ModelManual <-> ModelRuntime | IMX_ID: IMX-149 | NOTES: surface authority + capability declaration.
- PRIMITIVE_MATRIX_VERDICT: OK
- PRIMITIVE_MATRIX_REASON: Cross-cluster primitive interactions captured as first-class interaction-matrix edges to make the integration observable to downstream WPs and to no-context model inspection.

### MATRIX_RESEARCH_RUBRIC
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_REASON: New cross-primitive combos (ModelRuntime + SandboxAdapter + ProcessOwnershipLedger; SelfImprovementLoop + PromotionGate + held-out corpus) require external pattern review.
- SOURCE_SCAN:
  - RW-2 engine + adapter pattern review.
  - RW-3 technique combos: LoRA + steering + KV cache; speculative decoding + LoRA; subquadratic state-vector persistence + cross-session restore.
  - RW-6 self-improvement + held-out corpus + Goodhart sentinel patterns from DSPy MIPROv2 + SWE-Bench-Pro.
- MATRIX_GROWTH_CANDIDATES:
  - All 12 new IMX edges IMX-138..149 captured in spec 12.6.
  - Future expansion when MoD lands (revisit 6.8 + 4.7.2).
  - Future expansion when EAGLE-3 upstream PR #18039 merges (E-4).
- ENGINEERING_TRICKS_CARRIED_OVER:
  - 3-layer parallel-mutation control pattern (compile-time + runtime + audit log) carried from RW-4.
  - 60/20/20 train/dev/holdout encryption pattern carried from RW-6 + SWE-Bench-Pro.
  - per-launch random localhost port + feature-flag gating pattern carried from RW-4 for inspector plane.
- MATRIX_RESEARCH_VERDICT: PASS

### FORCE_MULTIPLIER_EXPANSION
- COMBO_PRESSURE_MODE: AUTO
- HIGH_ROI_EXPANSION_CANDIDATES:
  - Combo: HBR enforcement + Codex CX-131 + role protocols | Pillars: Flight Recorder + MicroTask | Mechanical: engine.sovereign | Primitives/Features: PRIM-HBRGate, PRIM-HBRViolationReceipt | Resolution: IN_THIS_WP | Notes: Cluster A.1 + A.7 lands HBR as build-time and handoff-time gate with role-protocol linkage in 7 roles.
  - Combo: SandboxAdapter + ProcessOwnershipLedger + SwarmTestHarness | Pillars: Execution / Job Runtime + Locus | Mechanical: engine.sandbox + engine.wrangler + engine.dba | Primitives/Features: PRIM-SandboxAdapter, PRIM-ProcessOwnershipLedger, PRIM-SwarmTestHarness | Resolution: IN_THIS_WP | Notes: Sandbox process spawn always lands in ledger; swarm tests validate ledger consistency under concurrency.
  - Combo: ModelRuntime + KV cache + LoRA hot-swap | Pillars: Skill distillation / LoRA + Execution / Job Runtime | Mechanical: engine.context | Primitives/Features: PRIM-ModelRuntime, PRIM-LlamaCppRuntime | Resolution: IN_THIS_WP | Notes: LlamaCppRuntime carries KV-cache quantization + prefix sharing + LoRA mount/unmount at PRODUCTION maturity.
  - Combo: CandleRuntime + activation hooks + refusal vector + CAA + steering | Pillars: Execution / Job Runtime | Mechanical: engine.context | Primitives/Features: PRIM-CandleRuntime, PRIM-ExecPolicyExtended | Resolution: IN_THIS_WP | Notes: Single hook substrate carries 3 inference techniques (INF-3/4/5) + abliteration prep.
  - Combo: Self-improvement loop + MemoryCapsule + ModelManual editable surface | Pillars: LLM-friendly data + Skill distillation / LoRA | Mechanical: engine.analyst | Primitives/Features: PRIM-SelfImprovementLoop, PRIM-MemoryCapsule, PRIM-ModelManual | Resolution: IN_THIS_WP | Notes: Karpathy autoresearch pattern with held-out corpus discipline and operator-review promotion.
  - Combo: Visual debugger + Inspector plane + Diagnostics panel | Pillars: Command Center | Mechanical: engine.guide | Primitives/Features: PRIM-DiagnosticsPanel | Resolution: IN_THIS_WP | Notes: Single panel exposes screenshot stream + DOM tree + Playwright history + focus-audit ledger viewer.
  - Combo: Distillation pipeline + content review + opt-in corpus | Pillars: Skill distillation / LoRA | Mechanical: engine.librarian | Primitives/Features: PRIM-DistillationCandidateReview | Resolution: IN_THIS_WP | Notes: PII scan + license tagging + dedup before Skill Bank reference; honors operator adult-production privacy.
  - Combo: Inference Lab UI + ModelCapabilities-driven toggle visibility | Pillars: Command Center | Mechanical: engine.guide | Primitives/Features: PRIM-InferenceLabUI, PRIM-ModelRuntime | Resolution: IN_THIS_WP | Notes: Toggles hidden (not greyed) when adapter does not support the technique; before/after A/B comparison view saves to Work Profile.
  - Combo: Cloud-lane parity bridge + BYOK + official-CLI adapters | Pillars: Execution / Job Runtime | Mechanical: engine.context | Primitives/Features: PRIM-LocalModelAdapter (extended for cloud) | Resolution: IN_THIS_WP | Notes: OpenAI/Anthropic BYOK + Claude Code / Codex CLI bridge per HBR-INT-005 lane normalization.
  - Combo: ModelManual + same-commit CI hook + self-consistency grep | Pillars: Spec to prompt | Mechanical: engine.guide | Primitives/Features: PRIM-ModelManual | Resolution: IN_THIS_WP | Notes: HBR-MAN-001/002/003 prevents manual drift away from wired surface.
  - Combo: Non-hijacking GUI + hidden Tauri window + focus-audit + LL keyboard hook | Pillars: Command Center | Mechanical: engine.sovereign | Primitives/Features: PRIM-SwarmTestHarness, PRIM-ProcessOwnershipLedger | Resolution: IN_THIS_WP | Notes: HBR-QUIET-001/002/003/004 stack; foreground exception is opt-in per packet.
  - Combo: MoD spec-deferral + §6.8 preliminary research + Phase 3 roadmap reflection | Pillars: Spec to prompt | Mechanical: engine.version | Primitives/Features: NONE (research only) | Resolution: SPEC_DEFERRED_PER_E1 | Notes: Future implementation WP picks it up only when production engine path materializes on Windows+Rust.
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_REASON: All high-ROI additions are preserved in the four-cluster 30-stub fold; no new stubs created per AC-011 + GLOBAL-PRESERVATION-009..013.

### EXISTING_CAPABILITY_ALIGNMENT
- SCAN_SCOPE: Task Board, Build Order, traceability registry, KERNEL-001/002/003 packets, FEMS V1 packet, 30 folded WP-1-* stubs, reset brief, and v02.186 spec anchors.
- MATCHED_STUBS: 30 distinct WP-1-* stubs enumerated in stub_fold_preservation_map across clusters A(2)/B(4)/C(8)/D(10)/X(6) per refinement.json. All preserved_intent fields hydrated 2026-05-18 (no remaining TO_BE_FILLED_FROM_STUB_READ placeholders per AC-011 + GLOBAL-PRESERVATION-009..013).
- MATCHED_ACTIVE_PACKETS:
  - Artifact: WP-KERNEL-001-Event-Ledger-Session-Broker-v1 | BoardStatus: DONE | Intent: SUBSTRATE | Resolution: REUSE_EXISTING | Notes: ModelRuntime builds on dummy ModelAdapter; MemoryCapsule on ArtifactStore; SelfImprovementLoop on ValidationRunner + PromotionGate; HBRGate as new descriptor under ValidationRunner.
  - Artifact: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1 | BoardStatus: DONE (61 MTs merged) | Intent: SUBSTRATE | Resolution: REUSE_EXISTING | Notes: All backend manipulation routes through KernelActionCatalogV1 + WriteBoxV1; no parallel mutation paths.
  - Artifact: WP-KERNEL-003-Sandbox-Validation-Promotion-v1 | BoardStatus: DONE | Intent: SUBSTRATE | Resolution: REUSE_EXISTING | Notes: KERNEL-003 Docker integration preserved as DockerAdapter under SandboxAdapter trait per R-3 control.
  - Artifact: WP-1-Front-End-Memory-System-v1 | BoardStatus: VALIDATED/merged | Intent: SUBSTRATE | Resolution: REUSE_EXISTING | Notes: Cluster D builds on FEMS V1 surfaces; B-FEMS-V1-INTEGRATION-SURFACE verification pass in Stage 4D before cluster D MT authoring.
- MATCHED_COMPLETED_PACKETS: see foundation_in_main block in refinement.json for full substrate mapping.
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- EXISTING_CAPABILITY_ALIGNMENT_REASON: 30 historical stubs fold into KERNEL-004; substrate (KERNEL-001/002/003 + FEMS V1) reused rather than duplicated.

### UI_UX_RUBRIC
- UI_UX_APPLICABLE: YES
- UI_SURFACES (per §10.12-§10.15):
  - §10.12 Diagnostics Panel: live WebView2 screenshot stream (default 1 Hz), collapsible DOM tree, Playwright command history, per-step screenshots with baseline-comparison overlay, swarm-N session selector, console/network event stream, focus-audit ledger viewer (red flag on Handshake-pid foreground transitions).
  - §10.13 ModelRuntime Control Panel: per-loaded-model id + artifact path + SHA256, active adapter, KV-cache occupancy, LoRA stack (ordered), active steering vectors, ProcessOwnershipLedger row link, live perf stats.
  - §10.14 Inference Lab UI: 8 PRODUCTION technique toggles writing through to settings.exec_policy per model + Work Profile; before/after A/B comparison; LoRA stack composer; KV quant level + prefix-cache TTL; steering vector intensity + layer-index pickers; draft-model picker + speculative mode; subquadratic state-vector persistence + cross-session restore; MoD deferral badge with link to §6.8.
  - §10.15 ModelManual Surface: backend Rust source-of-truth at src/backend/handshake_core/src/model_manual/mod.rs as typed Rust structs; Tauri IPC reads (no direct file reads from renderer); on-demand MODEL_MANUAL.md projection via `just generate-model-manual-md` recipe.
- UI_CONTROLS:
  - Control: technique toggles | Type: toggle | Tooltip: Enable/disable per-technique with adapter-capability check | Notes: hidden (not greyed) when adapter does not support.
  - Control: LoRA stack composer | Type: ordered list editor | Tooltip: Per-LoRA id + strength | Notes: hot-swap via runtime mount/unmount without process restart.
  - Control: KV cache quant level | Type: dropdown | Tooltip: none / q4 / q8 / q4_q8_mix | Notes: per-adapter capability declaration.
  - Control: steering vector intensity | Type: slider | Tooltip: Per-layer steering vector intensity | Notes: requires CandleRuntime adapter.
  - Control: distillation opt-in | Type: action button | Tooltip: Mark session for distillation corpus eligibility at session-close | Notes: default opt-out per Q-DISTILL-CORPUS.
  - Control: pause/resume/step automation | Type: control bar | Tooltip: Inspector plane replay-drive with operator-authorized signed envelope | Notes: single /inspector/v1/replay-drive endpoint, signed WriteBoxV1 envelope required.
- UI_STATES (empty/loading/error): no models loaded, adapter unsupported, technique-not-supported (toggle hidden), validation blocked, ledger overflow (degraded mode banner), holdout corpus PASS regression (Goodhart sentinel pause), HBR violation receipt rendering.
- UI_MICROCOPY_NOTES: Use ModelRuntime, Adapter, Sandbox, Process Ledger, KV cache, LoRA stack, Steering vector, Capsule, Holdout, Goodhart sentinel, HBR violation, Replay-drive.
- UI_ACCESSIBILITY_NOTES: Stable element identifiers for action catalog rows, write-box rows, denial receipts, promotion previews, stale projection badges per AC-VISUAL-DEBUGGER-MATRIX + KERNEL-002 visual debugging contract.
- UI_UX_VERDICT: OK

### GUI_IMPLEMENTATION_ADVICE_RUBRIC
- GUI_ADVICE_REQUIRED: YES
- GUI_REFERENCE_SCAN: Visual debugger architecture from RW-4 (Playwright via CDP against WebView2 + tauri-driver CI fallback + read-only inspector plane on localhost + 3-layer parallel-mutation controls). Non-hijacking GUI patterns from RW-5 (Tauri WindowBuilder hidden config + SetWinEventHook focus audit via wineventhook + WH_KEYBOARD_LL LLKHF_INJECTED negative test + lint rule banning OS-input APIs outside operator_foreground module).
- HANDSHAKE_GUI_ADVICE:
  - Surface: §10.12 Diagnostics Panel | Control: live screenshot stream | Type: streaming view | Why: inspect WebView2 state without operator focus loss | Microcopy: Live WebView2 stream | Tooltip: Configurable cadence (default 1 Hz) via CDP Page.captureScreenshot.
  - Surface: §10.12 focus-audit ledger viewer | Control: violation row highlight | Type: filtered table | Why: detect HBR-QUIET-001 violations | Microcopy: Foreground transitions | Tooltip: Red flag on any transition to Handshake's pid or to any process spawned by Handshake per ProcessOwnershipLedger.
  - Surface: §10.13 ModelRuntime Control Panel | Control: unload action | Type: action button | Why: free GPU memory | Microcopy: Unload model | Tooltip: Cancels active generations and removes from KV cache.
  - Surface: §10.14 Inference Lab UI | Control: before/after A/B comparison | Type: side-by-side generation view | Why: tune knobs without destructive editing | Microcopy: A/B compare | Tooltip: Save resulting exec_policy as new Work Profile or overwrite base.
- HIDDEN_GUI_REQUIREMENTS: Adapter unsupported, technique-not-supported, ledger overflow degraded mode, Goodhart sentinel pause, HBR violation states all visible without raw JSON reading.
- GUI_ENGINEERING_TRICKS_TO_CARRY: Key rows by run_id + model_id + process_uuid + sandbox_adapter_id + capsule_id; bind every control to (model_id, work_profile_id) before enabling it; visual baselines content-addressed under .GOV/visual_baselines/.
- GUI_IMPLEMENTATION_ADVICE_VERDICT: PASS

### ROADMAP_PHASE_SPLIT
- PHASE_SPLIT_NEEDED: YES (per CX-128 Phase 3 reflection rule, NOT scope phasing within KERNEL-004)
- PHASE_SPLIT_REASON: Spec v02.186 §7-6 Phase 3 fixed-template fields receive 15 [ADD v02.186] reflection bullets per CX-128 weaving rule, including the MoD deferral entry pointing to §6.8 preliminary research subsection. KERNEL-004 itself executes its four-cluster fold as a single packet without internal phase split; only the spec roadmap reflects the spec-deferred MoD scope per E-1.
- PHASE_3_REFLECTION_BULLETS: 15 [ADD v02.186] bullets distributed across Phase 3 fixed-template fields (Goal, MUST deliver, Vertical slice, Key risks, Acceptance, Out of scope) per SPEC-ENRICH-7-MoD-Phase3-Roadmap APPLIED 2026-05-18.

### PACKET_HYDRATION (mandatory)
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- REQUESTOR: Operator
- AGENT_ID: Kernel Builder (also serves as Activation Manager per kernel-builder protocol Activation Mode)
- RISK_TIER: HIGH
- SPEC_ADD_MARKER_TARGET: [ADD v02.186]
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-KERNEL-001-Event-Ledger-Session-Broker, WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening, WP-KERNEL-003-Sandbox-Validation-Promotion, WP-1-Front-End-Memory-System
- BUILD_ORDER_BLOCKS: WP-1-Visual-Debugging-Loop, WP-1-Cross-Tool-Interaction-Conformance, WP-1-Session-Spawn-Contract, WP-1-Workspace-Safety-Parallel-Sessions, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Postgres-Dev-Test-Container-Matrix, WP-1-LLM-Provider-Registry, WP-1-Model-Swap-Protocol, WP-1-Model-Onboarding-ContextPacks, WP-1-ModelSession-Core-Scheduler, WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry, WP-1-MTE-LoRA-Wiring, WP-1-Distillation, WP-1-Layerwise-Inference-Foundations, WP-1-FEMS-Bitemporal-Indexing, WP-1-FEMS-Outcome-Feedback-Loop, WP-1-FEMS-Injection-Scoring-Graceful-Degradation, WP-1-FEMS-Pinned-Core-Memory, WP-1-FEMS-Hygiene-Manager-Job, WP-1-FEMS-Calibration-Dashboard, WP-1-FEMS-Acceptance-Replay-Eval, WP-1-RAG-Retrieval-Mode-Policy, WP-1-Retrieval-Trace-Bundle-Export, WP-1-Session-Spawn-Conversation-Distillation, WP-1-Role-Mailbox, WP-1-Role-Mailbox-Executor-Routing-Claim-Lease, WP-1-Role-Mailbox-Micro-Task-Loop-Control, WP-1-Micro-Task-Executor, WP-1-Session-Crash-Recovery-Checkpointing, WP-1-Session-Observability-Spans-FR
- SPEC_ANCHOR_PRIMARY: .GOV/spec/master-spec-v02.186/spec-modules/04-llm-infrastructure.md#4.6 (ModelRuntime + LocalModelAdapter)
- WHAT: Activate KERNEL-004 as the four-cluster max-fold packet (A HBR enforcement + tooling | B SandboxAdapter 3-pattern | C ModelRuntime 2-adapter + 8 PRODUCTION inference techniques + Distillation | D Memory V0+ + self-improvement loop | X preservation) with all 30 historical WP-1-* stubs folded under preserved_intent.
- WHY: Each cluster on its own would touch the same kernel/sandbox/model/memory/HBR surfaces; splitting forces three or more sequential migrations of the same product code, each redoing spec enrichment, activation review, sandbox/runtime choice. The max-fold is operator-locked because B/C architecture decisions and A enforcement decisions are mutually entangled.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/sandbox/**
  - src/backend/handshake_core/src/model_runtime/**
  - src/backend/handshake_core/src/process_ledger/**
  - src/backend/handshake_core/src/hbr/**
  - src/backend/handshake_core/src/distillation/**
  - src/backend/handshake_core/src/memory/**
  - src/backend/handshake_core/src/self_improve/**
  - src/backend/handshake_core/src/test_harness/**
  - src/backend/handshake_core/src/model_manual/**
  - src/backend/handshake_core/src/inspector_read/**
  - src/backend/handshake_core/src/operator_foreground/**
  - src/backend/handshake_core/tests/**
  - src/backend/handshake_core/Cargo.toml
  - src/backend/handshake_core/Cargo.lock
  - app/src/components/diagnostics/**
  - app/src/components/inference_lab/**
  - app/src/components/model_runtime_panel/**
  - app/src-tauri/src/**
  - app/src-tauri/Cargo.toml
  - app/src-tauri/tauri.conf.json
  - app/package.json
  - app/pnpm-lock.yaml
  - tests/**
  - .GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json
  - .GOV/roles_shared/scripts/hbr-matrix-check.mjs
  - .GOV/roles_shared/checks/hbr-matrix-check.mjs
  - justfile
- OUT_OF_SCOPE:
  - No SQLite authority, cache, offline, fallback, compatibility, or test fixture anywhere per CX-503R.
  - No condensing, merging, dropping, or renumbering MTs after Stage 4E authoring.
  - No WP Validator gate or session (validation_topology = INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1).
  - No Integration Validator launch / verdict / merge / pass-fail claim by KERNEL_BUILDER in this packet.
  - No operator-facing default Markdown projections; md is on-demand only per CX-908 + CX-914 + feedback_no_default_md_files.
  - No LM Studio or Ollama runtime authority (operator-locked invariant; spec §3.6).
  - No model-process spawn outside SandboxAdapter (spec §3.6.2).
  - No model-process spawn without ProcessOwnershipLedger registration (spec §5.7.2).
  - No edits outside the 30-stub fold boundary (refinement.kernel004_extension.stub_fold_preservation_map).
  - No Master Spec edits inside this packet; Master Spec v02.186 enrichment was applied during pre-signature Activation Mode (Stage 3); further spec changes require a separate CX-105A spec-writing role packet.
  - No Codex edits inside this packet; CX-131 was added during Stage 3; further Codex changes require explicit operator authorization.
  - No mutations through the backend inspector plane (spec §6.5; KernelActionCatalogV1 + WriteBoxV1 are the only mutation paths).
  - No OS-level keyboard injection, cursor movement, focus stealing, or foregrounding from automation surfaces (spec §6.6; HBR-QUIET-001/002).
  - No abliteration in a generation hot path (spec §4.7.4; abliteration is offline tool only).
  - No MoD implementation; MoD is spec-deferred to Phase 3 roadmap per operator decision E-1 and spec §6.8.
  - No worktree creation or switching by sub-agents (kernel-builder protocol SUBAGENTS section).
  - No commits on feat/WP-KERNEL-004-* branch that include /.GOV/ files (CX-212F).
- TEST_PLAN:
  ```bash
  cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_sandbox_adapter --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_model_runtime --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_process_ledger --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_hbr_gate --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_self_improve --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core kernel_swarm_harness --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo clippy --workspace --all-targets -- -D warnings
  just gov-check
  just spec-eof-appendices-check
  pnpm -C app test
  pnpm -C app run lint
  ```
- DONE_MEANS:
  - HANDSHAKE_BUILD_RULES.json v1.1.0 DESIGN_ONLY_WIRING_PENDING gap is closed by wiring HBR enforcement into PACKET_ACCEPTANCE_MATRIX, gov-check, and validator-scan at build-time and handoff-time (Cluster A.1).
  - HBR_HANDOFF_GATE event emits on every governed inter-role transition; failed HBR rule blocks the handoff with typed CX-130 schema HBR_VIOLATION receipts (AC-HBR-HANDOFF-GATE, AC-HBR-TYPED-RECEIPT).
  - SandboxAdapter trait + WSL2PodmanAdapter (default) + WindowsNativeJailAdapter (strict) + DockerAdapter (compat preserving KERNEL-003 integration) all pass adapter-conformance tests and boundary-escape negative tests (AC-SANDBOX-ADAPTER-TRAIT, AC-SANDBOX-ADAPTER-IMPLS, AC-SANDBOX-ADAPTER-CAP-DECL).
  - ModelRuntime trait + LlamaCppRuntime + CandleRuntime expose 8 PRODUCTION inference techniques behind engine-agnostic surface; no llama_cpp_2::* or candle_core::* types leak through trait surface (AC-MODEL-RUNTIME-TRAIT, AC-LLAMACPP-ADAPTER, AC-CANDLE-ADAPTER, AC-INFER-LAB-8-TECHNIQUES).
  - LocalModel process boxing invariant holds: every model-process spawn passes through SandboxAdapter and is registered in ProcessOwnershipLedger; no LM Studio or Ollama runtime authority (AC-LOCAL-MODEL-BOXING, AC-NO-OLLAMA-DAEMON, AC-EXTERNAL-ENGINE-IMPORT).
  - ProcessOwnershipLedger writes START + STOP rows for every Handshake-spawned process to kernel_process_lifecycle Postgres table; reclaim hooks fire on session close/failure/staleness/operator cancel; FR-EVT-LEDGER-OVERFLOW degraded mode never silently drops (AC-PROCESS-LEDGER-PRIMITIVE, AC-PROCESS-LEDGER-SCOPE, AC-PROCESS-LEDGER-PERF, AC-PROCESS-LEDGER-RECLAIM).
  - Diagnostics Panel (§10.12) + ModelRuntime Control Panel (§10.13) + Inference Lab UI (§10.14) + ModelManual Surface (§10.15) render without operator focus loss and expose stable element identifiers per AC-VISUAL-DEBUGGER-MATRIX.
  - Backend Inspector Plane is read-only (compile-time trait separation) and routes mutations only through /inspector/v1/replay-drive with signed WriteBoxV1 envelope (AC-INSPECTOR-PLANE-BIND, AC-INSPECTOR-PLANE-READONLY, AC-INSPECTOR-PLANE-MUTATION-ROUTING).
  - Non-hijacking GUI invariants verified: hidden Tauri window config, SetWinEventHook focus audit, WH_KEYBOARD_LL LLKHF_INJECTED negative test, lint rule banning OS-input APIs outside operator_foreground module (AC-QUIET-WINDOW-CONFIG, AC-QUIET-FOCUS-AUDIT, AC-QUIET-INJECT-NEG-TEST, AC-QUIET-API-LINT, AC-QUIET-FOREGROUND-EXCEPTION).
  - SwarmTestHarness runs N=8 minimum concurrent governed sessions against same Handshake instance without lease contention failures (AC-SWARM-HARNESS-PRIMITIVE, AC-SWARM-HARNESS-N8, AC-SWARM-HARNESS-INVARIANTS).
  - Self-improvement loop uses 60/20/20 train/dev/holdout encrypted-at-rest + multi-metric promotion floor + Goodhart sentinel (auto-pause on 3 monotonic widening iterations); HBR-SWARM-002 loop counter cap enforced; PromotionGate operator review required per promotion (AC-DISTILL-OPT-IN, AC-DISTILL-CONTENT-REVIEW, AC-DISTILL-LOOP-SAFEGUARDS, AC-DISTILL-EDITABLE-SURFACE).
  - ModelManual stays current: HBR-MAN-001 same-commit CI hook + HBR-MAN-002 no-context model harness + HBR-MAN-003 self-consistency grep all pass; MANUAL_VERSION bumps on wired-surface diffs (AC-MODEL-MANUAL-AUTHORITY, AC-MODEL-MANUAL-IPC, AC-MODEL-MANUAL-MD-ON-DEMAND, AC-MODEL-MANUAL-SELF-CONSISTENCY, AC-MODEL-MANUAL-CI-HOOK).
  - All 30 historical WP-1-* stubs preserved per refinement.kernel004_extension.stub_fold_preservation_map; no stub intent dropped per AC-011 + GLOBAL-PRESERVATION-009..013.
  - Closeout requests Integration Validator batch review; Kernel Builder does not self-claim PASS/FAIL.
- PRIMITIVES_EXPOSED:
  - PRIM-CandleRuntime
  - PRIM-DiagnosticsPanel
  - PRIM-DistillationCandidateReview
  - PRIM-DockerAdapter
  - PRIM-ExecPolicyExtended
  - PRIM-HBRGate
  - PRIM-HBRViolationReceipt
  - PRIM-InferenceLabUI
  - PRIM-LlamaCppRuntime
  - PRIM-LocalModelAdapter
  - PRIM-MemoryCapsule
  - PRIM-ModelManual
  - PRIM-ModelRuntime
  - PRIM-ModelRuntimeControlPanel
  - PRIM-ProcessOwnershipLedger
  - PRIM-SandboxAdapter
  - PRIM-SelfImprovementLoop
  - PRIM-SwarmTestHarness
  - PRIM-WSL2PodmanAdapter
  - PRIM-WindowsNativeJailAdapter
- PRIMITIVES_CREATED:
  - PRIM-CandleRuntime
  - PRIM-DiagnosticsPanel
  - PRIM-DistillationCandidateReview
  - PRIM-DockerAdapter
  - PRIM-ExecPolicyExtended
  - PRIM-HBRGate
  - PRIM-HBRViolationReceipt
  - PRIM-InferenceLabUI
  - PRIM-LlamaCppRuntime
  - PRIM-LocalModelAdapter
  - PRIM-MemoryCapsule
  - PRIM-ModelManual
  - PRIM-ModelRuntime
  - PRIM-ModelRuntimeControlPanel
  - PRIM-ProcessOwnershipLedger
  - PRIM-SandboxAdapter
  - PRIM-SelfImprovementLoop
  - PRIM-SwarmTestHarness
  - PRIM-WSL2PodmanAdapter
  - PRIM-WindowsNativeJailAdapter
- FILES_TO_OPEN:
  - .GOV/task_packets/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1/refinement.json
  - .GOV/task_packets/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1/packet.json
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/spec/master-spec-v02.186/spec-modules/03-local-first-infrastructure.md
  - .GOV/spec/master-spec-v02.186/spec-modules/04-llm-infrastructure.md
  - .GOV/spec/master-spec-v02.186/spec-modules/05-security-and-observability.md
  - .GOV/spec/master-spec-v02.186/spec-modules/06-mechanical-integrations.md
  - .GOV/spec/master-spec-v02.186/spec-modules/10-product-surfaces.md
  - .GOV/spec/master-spec-v02.186/spec-modules/12-end-of-file-appendices.md
  - .GOV/codex/Handshake_Codex_v1.4.md
  - .GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json
  - src/backend/handshake_core/src/kernel/action_catalog.rs
  - src/backend/handshake_core/Cargo.toml
  - app/src-tauri/tauri.conf.json
- SEARCH_TERMS:
  - KernelActionCatalogV1
  - WriteBoxV1
  - EventLedger
  - PromotionGate
  - ValidationRunner
  - FEMS V1
  - HANDSHAKE_BUILD_RULES
  - ModelRuntime
  - SandboxAdapter
  - ProcessOwnershipLedger
  - MemoryCapsule
  - SelfImprovementLoop
  - WebView2
  - Tauri
  - llama-cpp-2
  - candle
  - Playwright CDP
  - WSL2 Podman
  - HBR_HANDOFF_GATE
  - kernel_process_lifecycle
- RUN_COMMANDS:
  ```bash
  just pre-work WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1
  cargo build --workspace --release --target-dir ../Handshake_Artifacts/handshake-cargo-target
  cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
  just gov-check
  just hbr-matrix-check WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1
  just spec-eof-appendices-check
  just generate-model-manual-md
  ```
- RISK_MAP:
  - R-1 Scope-consuming-product (KERNEL-004 largest WP in project history risks harness-consumes-product) -> hard scope limit at 30-stub-fold boundary; refinement forbids scope additions outside folded stubs.
  - R-2 PRODUCTION-tier research-frontier risk for MoD/Subquadratic -> MoD spec-deferred per E-1; Subquadratic locked at full feature parity per E-2 with multi-quarter timeline disclosed eyes-open.
  - R-3 Sandbox-architecture-flip-orphans-KERNEL-003 -> DockerAdapter preserved as compat-only-not-default; explicit migration MTs route KERNEL-003 callers under SandboxAdapter trait.
  - R-4 Spec-enrichment-becomes-its-own-project -> single coherent v02.186 bundle bump per CX-105C copy-first; no incremental enrichment drift; B-SPEC-ENRICHMENT resolved 2026-05-18.
  - R-5 LM-Studio/Ollama-lockout -> ExternalEngineImport non-authoritative pass-through lane (§3.6.4); operator may point at model file or local OpenAI-compatible HTTP endpoint without LM Studio or Ollama as runtime authority.
  - R-6 Visual-debugger-parallel-mutation-path -> 3-layer parallel-mutation controls (compile-time trait separation via inspector_read crate; runtime localhost+secret+feature-flag binding; audit log via single /inspector/v1/replay-drive endpoint accepting only catalog action id + signed WriteBoxV1 envelope).
  - R-7 Self-improvement-overfit -> 60/20/20 train/dev/holdout encrypted-at-rest + multi-metric promotion floor (dev PASS + latency p95 + capsule bytes + holdout PASS) + Goodhart sentinel (auto-pause on 3 monotonic widening iterations) + HBR-SWARM-002 loop counter cap + PromotionGate operator review.
  - R-8 MoD-Subquad-feasibility -> resolved by spec-deferral (MoD §6.8) and operator E-2 full-parity acceptance (Subquadratic).
  - R-9 FEMS-V1-API-drift -> B-FEMS-V1-INTEGRATION-SURFACE remains BLOCKING for cluster D MT authoring only; verification pass in Stage 4D before cluster D MT authoring begins.
  - R-10 WindowsNativeJailAdapter-crate-license -> license audit MT required before adopting codex-windows-sandbox or rappct crate as the strict-isolation adapter substrate.
  - R-11 Process-ledger-perf-under-swarm -> batched async writes + per-process metadata cap + async write path + FR-EVT-LEDGER-OVERFLOW degraded-mode banner (§5.7.3).
  - R-12 Mojibake-in-spec-module-04 -> pre-existing UTF-8 double-encoding in legacy §4.2 content; Stage 2 worked around it; cleanup is non-blocking governance debt.
  - R-13 AC-cross-reference-drift -> 52 AC-* IDs in spec are cross-referenced from refinement.json acceptance_criteria; renames or splits must update both surfaces in the same change.

#### PACKET_HYDRATION ACCEPTANCE CRITERIA REFERENCE
- summary_criteria: 16 operator-readable summary criteria AC-001 through AC-016 in refinement.json `refinement.summary_criteria`.
- acceptance_criteria: 52 structured AC-* IDs in refinement.json `refinement.acceptance_criteria` (cross-referenced to spec v02.186 anchors).

#### PACKET_HYDRATION DEPENDENCIES
KERNEL-001 (EventLedger/SessionBroker/ContextBundle/dummy ModelAdapter/ToolGate/ArtifactStore/ValidationRunner/PromotionGate/TraceProjection); KERNEL-002 (KernelActionCatalogV1/WriteBoxV1/CRDT workspace/61 MTs merged); KERNEL-003 (Sandbox/ValidationRunner/PromotionGate/Docker integration preserved as DockerAdapter); FEMS V1 (typed memory + session integration + operator panel + flight-recorder events + bounded memory pack injection + review-gated writes). See refinement.json `kernel004_extension.foundation_in_main` for full mapping.

### CLAUSE_PROOF_PLAN
- CLAUSE_ROWS: 52 AC-* IDs in refinement.json `refinement.acceptance_criteria` serve as clause anchors with spec v02.186 cross-references. Each AC-* has cluster assignment (A/B/C/D/X), status (PENDING_IMPLEMENTATION), and spec_anchor. Selected clause anchors:
  - AC-HBR-AUTHORITY (Cluster A | §5.6.1): HANDSHAKE_BUILD_RULES.json single source-of-truth.
  - AC-HBR-HANDOFF-GATE (Cluster A | §5.6.2): build-time + handoff-time enforcement.
  - AC-HBR-TYPED-RECEIPT (Cluster A | §5.6.3): CX-130 schema.
  - AC-PROCESS-LEDGER-PRIMITIVE / SCOPE / PERF / RECLAIM (Cluster A | §5.7.1-5.7.5): kernel_process_lifecycle table.
  - AC-VISUAL-DEBUGGER-IMPL / IPC / MATRIX (Cluster A | §6.4): Playwright via CDP against WebView2.
  - AC-INSPECTOR-PLANE-BIND / READONLY / MUTATION-ROUTING (Cluster A | §6.5): localhost-only + read-only trait + single /inspector/v1/replay-drive endpoint.
  - AC-QUIET-WINDOW-CONFIG / FOCUS-AUDIT / INJECT-NEG-TEST / API-LINT / FOREGROUND-EXCEPTION (Cluster A | §6.6): non-hijacking GUI invariants.
  - AC-SWARM-HARNESS-PRIMITIVE / N8 / INVARIANTS (Cluster A | §6.7): N=8 minimum.
  - AC-SANDBOX-ADAPTER-TRAIT / IMPLS / CAP-DECL (Cluster B | §3.5): 3-adapter pattern.
  - AC-LOCAL-MODEL-BOXING / NO-OLLAMA-DAEMON / EXTERNAL-ENGINE-IMPORT (Cluster C | §3.6): model process boxing invariant.
  - AC-MODEL-RUNTIME-TRAIT / LLAMACPP-ADAPTER / CANDLE-ADAPTER / MODEL-CAP-DECL (Cluster C | §4.6): multi-engine adapter pattern.
  - AC-INFER-LAB-8-TECHNIQUES / MOD-DEFERRED / ABLITERATION-OFFLINE / MOD-RESEARCH-DOC / MOD-NO-STUB (Cluster C | §4.7): 8 PRODUCTION techniques + MoD spec-deferred.
  - AC-DISTILL-OPT-IN / CONTENT-REVIEW / LOOP-SAFEGUARDS / EDITABLE-SURFACE (Cluster D | §4.8): distillation pipeline + self-improvement loop.
  - AC-DIAG-PANEL-EXTENSION / SURFACES / INTERACTION (Cluster X | §10.12): Diagnostics Panel.
  - AC-MODEL-RUNTIME-CONTROL-PANEL / INFER-LAB-UI-TOGGLES / AB-COMPARE / MOD-BADGE (Cluster X | §10.13-§10.14): UI surfaces.
  - AC-MODEL-MANUAL-AUTHORITY / IPC / MD-ON-DEMAND / SELF-CONSISTENCY / CI-HOOK (Cluster X | §10.15): ModelManual surface.

### CONTRACT_SURFACES
- CONTRACT: PRIM-ModelRuntime trait + PRIM-LocalModelAdapter | PRODUCER: ModelRuntime adapter implementations | CONSUMER: kernel, Inference Lab UI, session broker, distillation pipeline | SERIALIZER_TRANSPORT: engine-agnostic Rust trait surface + Tauri IPC | VALIDATOR_READER: model runtime conformance tests + capability declaration tests | TRIPWIRE_TESTS: no llama_cpp_2::* or candle_core::* types leak through trait surface; per-model adapter binding immutable post-register | DRIFT_RISK: engine-specific types leak into callers.
- CONTRACT: PRIM-SandboxAdapter + 3 concrete adapters | PRODUCER: sandbox adapter implementations | CONSUMER: ModelRuntime, ValidationRunner, swarm-agent harness, model-written code | SERIALIZER_TRANSPORT: engine-agnostic Rust trait + AdapterCapabilities struct via Tauri IPC | VALIDATOR_READER: adapter conformance tests + boundary escape negative tests | TRIPWIRE_TESTS: no adapter-specific types in trait surface; capability declarations consumed by §4.6 and §5.7 | DRIFT_RISK: adapter implementation leaks call-site assumptions.
- CONTRACT: PRIM-ProcessOwnershipLedger + kernel_process_lifecycle Postgres table | PRODUCER: spawn paths through SandboxAdapter | CONSUMER: Diagnostics Panel, reclaim hooks, focus-audit ledger | SERIALIZER_TRANSPORT: Postgres rows + batched async mpsc writes + FR-EVT-LEDGER-OVERFLOW receipt | VALIDATOR_READER: ledger row presence + START/STOP pair tests + reclaim tests | TRIPWIRE_TESTS: every spawn path writes START + STOP rows; channel saturation triggers degraded-mode banner; no silent drop | DRIFT_RISK: spawn path bypasses ledger.
- CONTRACT: PRIM-HBRGate + PRIM-HBRViolationReceipt | PRODUCER: hbr-matrix-check.mjs + handoff-time validator | CONSUMER: gov-check, PACKET_ACCEPTANCE_MATRIX, role transitions | SERIALIZER_TRANSPORT: CX-130 schema + HBR_HANDOFF_GATE event | VALIDATOR_READER: HBR rule evaluation + receipt schema tests | TRIPWIRE_TESTS: failed HBR rule blocks handoff; free-form prose never the wire contract | DRIFT_RISK: HBR rule auto-emit drifts from registry.
- CONTRACT: PRIM-MemoryCapsule + PRIM-SelfImprovementLoop | PRODUCER: memory retrieval + iteration runner | CONSUMER: governed sessions, PromotionGate operator review | SERIALIZER_TRANSPORT: typed capsule artifact + audit log + Postgres rows | VALIDATOR_READER: capsule generation policy tests + Goodhart sentinel tests + holdout corpus regression tests | TRIPWIRE_TESTS: 60/20/20 train/dev/holdout split; multi-metric promotion floor; HBR-SWARM-002 loop counter cap | DRIFT_RISK: target overfits without held-out corpus.
- CONTRACT: PRIM-DiagnosticsPanel + PRIM-ModelRuntimeControlPanel + PRIM-InferenceLabUI + PRIM-ModelManual | PRODUCER: app/src surfaces + Rust source-of-truth | CONSUMER: operator, no-context model harness | SERIALIZER_TRANSPORT: Tauri IPC + typed Rust structs + on-demand md projection | VALIDATOR_READER: HBR-MAN-001 same-commit CI hook + HBR-MAN-002 no-context harness + HBR-MAN-003 self-consistency grep | TRIPWIRE_TESTS: manual name resolves to Rust symbol; MANUAL_VERSION bump on wired-surface diff; toggle visibility driven by ModelCapabilities | DRIFT_RISK: manual prose drifts away from wired surface.

### CODER_HANDOFF_BRIEF
- IMPLEMENTATION_ORDER: Cluster A (35-45 MTs) -> Cluster B (15-20 MTs) -> Cluster C (60-80 MTs) -> Cluster D (25-35 MTs) -> Cluster X (20-30 MTs); within-cluster ordering subject to dependency relationships authored in Stage 4E. Total estimated 180-235 MTs.
- AUTHORING_STAGE: Stage 4E authors MT contracts per packet.json `microtasks` block (declared_ids = ["PENDING_STAGE_4E_AUTHORING"]). Per validation_topology = INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1, Kernel Builder implements MTs back to back with no per-MT WP Validator gate.
- HOT_FILES: see packet.json scope.allowed_paths (src/backend/handshake_core/src/sandbox/**, model_runtime/**, process_ledger/**, hbr/**, distillation/**, memory/**, self_improve/**, test_harness/**, model_manual/**, inspector_read/**, operator_foreground/**; tests/**; Cargo.toml; Cargo.lock).
- TRIPWIRE_TESTS: see PACKET_HYDRATION PROOF COMMANDS above.
- CARRY_FORWARD_WARNINGS: Do not condense or remove any folded stub intent per AC-011 + GLOBAL-PRESERVATION-009..013. Do not introduce WP Validator gate. Do not use SQLite anywhere per CX-503R. Do not embed LM Studio or Ollama as runtime authority; ExternalEngineImport pass-through lane only. Do not create parallel mutation paths through the visual debugger; all backend manipulation routes through KernelActionCatalogV1 + WriteBoxV1. Do not hand-roll Win32 Job Objects + AppContainer + Restricted Tokens; wrap existing Rust crate (codex-windows-sandbox or rappct) per R-10 license audit MT first. Do not promote self-improvement loop changes without operator review and held-out corpus check per AC-DISTILL-LOOP-SAFEGUARDS. Do not allow PolicyScopedLocal sandbox labelling drift; preserve KERNEL-003 honesty.

### VALIDATOR_HANDOFF_BRIEF
- CLAUSES_TO_INSPECT: All 52 AC-* IDs in refinement.acceptance_criteria; 30 folded stub preserved_intent fields (no TO_BE_FILLED_FROM_STUB_READ remaining); HBR enforcement wiring (registry + gov-check sub-check + validator-scan + PACKET_ACCEPTANCE_MATRIX hydration); SandboxAdapter 3-pattern conformance + boundary escape negative tests; ModelRuntime 2-adapter pattern + per-model immutable binding; 8 PRODUCTION inference techniques + MoD spec-deferred; ProcessOwnershipLedger ALL Handshake-spawned processes + START/STOP pairs + reclaim + perf-mitigated channel saturation; self-improvement loop 60/20/20 + Goodhart sentinel + PromotionGate operator review; ModelManual same-commit CI hook + no-context harness + self-consistency grep; Codex CX-131 + 7 role-protocol linkage.
- FILES_TO_READ:
  - .GOV/task_packets/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1/refinement.json
  - .GOV/task_packets/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1/packet.json
  - .GOV/task_packets/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1/MT-*.json
  - .GOV/spec/SPEC_CURRENT.md -> .GOV/spec/master-spec-v02.186/**
  - .GOV/codex/Handshake_Codex_v1.4.md (CX-131)
  - .GOV/roles_shared/records/HANDSHAKE_BUILD_RULES.json (v1.1.0; 23 active rules)
  - src/backend/handshake_core/src/sandbox/**, model_runtime/**, process_ledger/**, hbr/**, distillation/**, memory/**, self_improve/**, test_harness/**, model_manual/**, inspector_read/**
- COMMANDS_TO_RUN: see PACKET_HYDRATION PROOF COMMANDS above.
- POST_MERGE_SPOTCHECKS: no-context manual path (HBR-MAN-002), sandbox boundary escape harness, ProcessOwnershipLedger consistency under swarm concurrency, self-improvement loop holdout regression proof, HBR gate handoff blocking proof, visual debugger inspector plane read-only proof.

### NOT_PROVEN_AT_REFINEMENT_TIME
- Product implementation has not started; Stage 4E MT authoring + implementation pending.
- B-FEMS-V1-INTEGRATION-SURFACE remains BLOCKING for cluster D MT authoring; Stage 4D verification pass required before cluster D MTs land.
- WindowsNativeJailAdapter crate license audit pending per R-10 before crate adoption.
- R-2 subquadratic multi-quarter timeline impact (6+ months disclosed and accepted eyes-open by operator E-2) not yet realized in implementation; revisit if RW-3-equivalent re-scan reveals new constraints.
- R-9 FEMS V1 API drift not yet verified against current main; Stage 4D inspection task.
- R-10 codex-windows-sandbox / rappct license terms not yet audited; Stage 4E pre-implementation MT.
- EAGLE-3 upstream merge (llama.cpp PR #18039) not yet landed; KERNEL-004 ships ngram/draft self-spec today per E-4 with EAGLE-3 upgrade post-merge.
- MoD production engine path on Windows+Rust not yet materialized; future implementation WP picks it up only when path exists (revisit triggers §4.7.2 + §6.8).
- Integration Validator verdict does not exist yet; assigned at INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1 review post-implementation.
- §10.12-§10.15 visual evidence waits on GUI implementation.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: v02.186 spec authority names ModelRuntime + LocalModelAdapter (§4.6), 8 PRODUCTION inference techniques (§4.7.1), MoD spec-deferral (§4.7.2), distillation pipeline (§4.8), SandboxAdapter + 3 adapters (§3.5), LocalModel process boxing invariant (§3.6), HBR Enforcement Authority (§5.6), ProcessOwnershipLedger (§5.7), Visual Debugger + Backend Inspector Plane + Non-Hijacking GUI Invariants + Swarm-Agent Harness + MoD Preliminary Research (§6.4-§6.8), Diagnostics Panel + ModelRuntime Control Panel + Inference Lab UI + ModelManual Surface (§10.12-§10.15), 20 new primitives + 12 new IMX edges (§12.4 + §12.6), 15 Phase 3 reflection bullets (§7-6 [ADD v02.186]); Codex CX-131 HARD_HBR_BUILD_HANDOFF_GATE; refinement supplies 52 measurable AC-* IDs cross-referenced to spec anchors; 30 historical WP-1-* stubs folded with non-empty preserved_intent fields.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: YES
- REASON_ENRICHMENT: Spec v02.186 enrichment bundle was applied 2026-05-18 pre-signature per CX-105C copy-first; 7 enrichment topics (SPEC-ENRICH-1 Sandbox-Adapter, SPEC-ENRICH-2 LLM-Infrastructure, SPEC-ENRICH-3 Security-Observability, SPEC-ENRICH-4 Mechanical-Integrations, SPEC-ENRICH-5 Product-Surfaces, SPEC-ENRICH-6 EOF-Appendices, SPEC-ENRICH-7 MoD-Phase3-Roadmap) APPLIED across modules 03/04/05/06/07-6/10/12; Codex CX-131 added; manifest/INDEX/changelog/SPEC_CURRENT updated; v02.185 archived; spec-current-check PASS. B-SPEC-ENRICHMENT resolved 2026-05-18. ENRICHMENT_NEEDED=YES because PRIMITIVE_INDEX / FEATURE_REGISTRY / UI_GUIDANCE / INTERACTION_MATRIX appendix actions in this refinement are marked UPDATED (per gate rule that UPDATED appendix actions require ENRICHMENT_NEEDED=YES until indexed modules + manifest + SPEC_CURRENT JSON are refreshed). The v02.186 bundle bump landed pre-signature and covers all 7 enrichment topics + Codex CX-131. See refinement.json `refinement.approved_spec_enrichment` for the 7 applied topics.
- PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES; includes appendix-only spec updates):
```md
SPEC-ENRICH-1 Sandbox-Adapter (.GOV/spec/master-spec-v02.186/spec-modules/03-local-first-infrastructure.md §3.5 Sandbox Adapter Layer + §3.6 LocalModel Process Boxing Invariant): APPLIED 2026-05-18. Defines SandboxAdapter trait, WSL2PodmanAdapter (default), WindowsNativeJailAdapter (strict, wraps existing Rust crate per OpenAI Codex 2026 pattern), DockerAdapter (compat preserving KERNEL-003 integration), per-job selection policy, and machine-readable adapter capability declarations. LocalModel process boxing invariant: every model process is a sandboxed, ledger-tracked child; no LM Studio or Ollama runtime authority; ExternalEngineImport is a non-authoritative pass-through lane only.

SPEC-ENRICH-2 LLM-Infrastructure (.GOV/spec/master-spec-v02.186/spec-modules/04-llm-infrastructure.md §4.6 ModelRuntime + LocalModelAdapter + §4.7 Inference Research Lab + §4.8 Distillation Pipeline): APPLIED 2026-05-18. Defines ModelRuntime trait, LocalModelAdapter invariant, LlamaCppRuntime (llama-cpp-2 default for transformer GGUF + LoRA hot-swap + KV cache + ngram/draft self-speculative decoding), CandleRuntime (hook-requiring substrate for activation steering + refusal vector + CAA + abliteration prep + subquadratic Mamba2/RWKV), 8 PRODUCTION inference techniques in §4.7.1, MoD spec-deferred per §4.7.2 + §6.8, abliteration offline-only per §4.7.4, distillation opt-in semantics + content-review pipeline + Memory V0+ self-improvement loop in §4.8.

SPEC-ENRICH-3 Security-Observability (.GOV/spec/master-spec-v02.186/spec-modules/05-security-and-observability.md §5.6 HBR Enforcement Authority + §5.7 ProcessOwnershipLedger): APPLIED 2026-05-18. HBR enforcement as build-time + handoff-time gate authority; HANDSHAKE_BUILD_RULES.json single source-of-truth (§5.6.1); two HARD evaluation points (§5.6.2); typed receipts using CX-130 schema (§5.6.3); rule-change procedure (§5.6.4); tooling (§5.6.5). ProcessOwnershipLedger primitive + schema (§5.7.1); HARD scope every Handshake-spawned process (§5.7.2); perf safeguards (§5.7.3); validator query surface (§5.7.4); reclaim hooks (§5.7.5); modification of §5.2.5 (§5.7.6).

SPEC-ENRICH-4 Mechanical-Integrations (.GOV/spec/master-spec-v02.186/spec-modules/06-mechanical-integrations.md §6.4 Visual Debugger + §6.5 Backend Inspector Plane + §6.6 Non-Hijacking GUI Invariants + §6.7 Swarm-Agent Harness + §6.8 MoD Preliminary Research): APPLIED 2026-05-18. Visual Debugger via Playwright over CDP against WebView2 with CI fallback + headless capture IPC + capture matrix + WP applicability. Backend Inspector Plane bind + feature gate + compile-time read-only invariant + endpoint families + mutation routing + runtime controls. Non-Hijacking GUI Invariants: Tauri quiet-mode config (HBR-QUIET-001), screenshots from never-shown windows, automation-first design (HBR-QUIET-002), focus audit subsystem, keyboard-injection negative test. Swarm-Agent Harness primitive + coordination contract + parameterization + scope distinction from §4.3.9 + surfaces. MoD informative preliminary research (deferred implementation).

SPEC-ENRICH-5 Product-Surfaces (.GOV/spec/master-spec-v02.186/spec-modules/10-product-surfaces.md §10.12 Diagnostics Panel + §10.13 ModelRuntime Control Panel + §10.14 Inference Lab UI + §10.15 ModelManual Surface): APPLIED 2026-05-18. Four new product surfaces with stable element identifiers for action catalog rows, write-box rows, denial receipts, promotion previews, and stale projection badges. ModelManual surface authority lives at src/backend/handshake_core/src/model_manual/mod.rs as typed Rust structs; Tauri IPC reads (no direct file reads from renderer); on-demand MODEL_MANUAL.md projection via `just generate-model-manual-md` recipe.

SPEC-ENRICH-6 EOF-Appendices (.GOV/spec/master-spec-v02.186/spec-modules/12-end-of-file-appendices.md §12.4 PRIMITIVE_TOOL_TECH_MATRIX + §12.6 INTERACTION_MATRIX): APPLIED 2026-05-18. 20 new primitives added to §12.4 (PRIM-ModelRuntime, PRIM-LocalModelAdapter, PRIM-LlamaCppRuntime, PRIM-CandleRuntime, PRIM-SandboxAdapter, PRIM-WSL2PodmanAdapter, PRIM-WindowsNativeJailAdapter, PRIM-DockerAdapter, PRIM-ProcessOwnershipLedger, PRIM-HBRGate, PRIM-HBRViolationReceipt, PRIM-MemoryCapsule, PRIM-SelfImprovementLoop, PRIM-DistillationCandidateReview, PRIM-DiagnosticsPanel, PRIM-ModelRuntimeControlPanel, PRIM-InferenceLabUI, PRIM-ModelManual, PRIM-SwarmTestHarness, PRIM-ExecPolicyExtended). 12 new IMX edges (IMX-138..149) covering ModelRuntime <-> SandboxAdapter, MemoryCapsule <-> FEMS V1, SelfImprovementLoop <-> PromotionGate + EventLedger, HBRGate <-> PACKET_ACCEPTANCE_MATRIX + ValidationRunner.

SPEC-ENRICH-7 MoD-Phase3-Roadmap (.GOV/spec/master-spec-v02.186/spec-modules/07-6-development-roadmap.md Phase 3 fixed-template fields): APPLIED 2026-05-18. 15 [ADD v02.186] reflection bullets distributed across Phase 3 fixed-template fields (Goal, MUST deliver, Vertical slice, Key risks, Acceptance, Out of scope) per CX-128 weaving rule, including the MoD deferral entry pointing to §6.8 preliminary research subsection.

Codex CX-131 (.GOV/codex/Handshake_Codex_v1.4.md): ADDED 2026-05-18. HARD_HBR_BUILD_HANDOFF_GATE entry establishing HBR enforcement as build-time + handoff-time gate authority across 7 governed role transitions.
```

### FEATURE_DISCOVERY_CHECKPOINT
- DISCOVERY_PRIMITIVES: 20 new primitives added to §12.4 per SPEC-ENRICH-6 APPLIED 2026-05-18: PRIM-ModelRuntime, PRIM-LocalModelAdapter, PRIM-LlamaCppRuntime, PRIM-CandleRuntime, PRIM-SandboxAdapter, PRIM-WSL2PodmanAdapter, PRIM-WindowsNativeJailAdapter, PRIM-DockerAdapter, PRIM-ProcessOwnershipLedger, PRIM-HBRGate, PRIM-HBRViolationReceipt, PRIM-MemoryCapsule, PRIM-SelfImprovementLoop, PRIM-DistillationCandidateReview, PRIM-DiagnosticsPanel, PRIM-ModelRuntimeControlPanel, PRIM-InferenceLabUI, PRIM-ModelManual, PRIM-SwarmTestHarness, PRIM-ExecPolicyExtended.
- DISCOVERY_STUBS: NONE_CREATED - 30 stubs FOLDED (no new stubs per AC-011 + GLOBAL-PRESERVATION-009..013; stub_fold_preservation_map preserves all 30 historical WP-1-* stub intents).
- DISCOVERY_MATRIX_EDGES: 12 new IMX edges added to §12.6 per SPEC-ENRICH-6 APPLIED 2026-05-18: IMX-138, IMX-139, IMX-140, IMX-141, IMX-142, IMX-143, IMX-144, IMX-145, IMX-146, IMX-147, IMX-148, IMX-149 (covering ModelRuntime <-> SandboxAdapter, MemoryCapsule <-> FEMS V1, SelfImprovementLoop <-> PromotionGate + EventLedger, HBRGate <-> PACKET_ACCEPTANCE_MATRIX + ValidationRunner, plus 8 additional cross-primitive edges).
- DISCOVERY_UI_CONTROLS: §10.12 Diagnostics Panel (live WebView2 screenshot stream, DOM tree, Playwright command history, per-step screenshots with baseline-comparison overlay, swarm-N session selector, console/network event stream, focus-audit ledger viewer); §10.13 ModelRuntime Control Panel (id + artifact path + SHA256, active adapter, KV-cache occupancy, LoRA stack, active steering vectors, ProcessOwnershipLedger row link, live perf stats; unload, swap-adapter, inspect engine internals); §10.14 Inference Lab UI (8 technique toggles, LoRA stack composer, KV quant level + prefix-cache TTL, steering vector intensity + layer-index pickers, draft-model picker + speculative mode, subquadratic state-vector persistence + cross-session restore, before/after A/B comparison, MoD deferral badge); §10.15 ModelManual Surface (typed Rust structs + Tauri IPC + on-demand md projection).
- DISCOVERY_SPEC_ENRICHMENT: NO_ENRICHMENT_NEEDED - v02.186 bump APPLIED 2026-05-18 covers all 7 enrichment topics + Codex CX-131.
- DISCOVERY_JUSTIFICATION: All sandbox + model runtime + inference technique + memory + self-improvement + HBR + visual debugger + non-hijacking GUI + swarm harness + ModelManual primitives and UI controls are preserved in the four-cluster fold; spec v02.186 enrichment landed pre-signature so no further spec bump required before implementation.

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/03-local-first-infrastructure.md#3.5
- CONTEXT_START_LINE: 20010
- CONTEXT_END_LINE: 20072
- CONTEXT_TOKEN: Sandbox Adapter Layer
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 3.5 Sandbox Adapter Layer (Normative) [ADD v02.186]
  Defines the SandboxAdapter trait, the three required adapter implementations, the per-job selection policy, and the machine-readable capability declarations that downstream surfaces (ProcessOwnershipLedger 5.7, ModelRuntime 4.6, Work Profile 4.3.7) consume.
  - SandboxAdapter: Rust trait exposing a uniform spawn/exec/fs-bind/net-policy/kill/status surface over heterogeneous OS-level isolation primitives.
  - WSL2+Podman: Rootless Podman containers running inside WSL2; near-native GPU passthrough on NVIDIA via WSL2 CUDA driver.
  - WindowsNativeJailAdapter: Win32 Job Objects + AppContainer + Restricted Tokens, wrapping an existing field-tested Rust crate.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/03-local-first-infrastructure.md#3.6
- CONTEXT_START_LINE: 20073
- CONTEXT_END_LINE: 20120
- CONTEXT_TOKEN: LocalModel Process Boxing Invariant
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 3.6 LocalModel Process Boxing Invariant (Normative) [ADD v02.186]
  ### 3.6.1 Hard invariant: no third-party model-server wrapper
  ### 3.6.2 Every model process is a sandboxed, ledger-tracked child
  ### 3.6.3 On-disk layout
  ### 3.6.4 ExternalEngineImport lane (compatibility, NOT authority)
  ```

#### ANCHOR 3
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/04-llm-infrastructure.md#4.6
- CONTEXT_START_LINE: 22404
- CONTEXT_END_LINE: 22471
- CONTEXT_TOKEN: ModelRuntime + LocalModelAdapter
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 4.6 ModelRuntime + LocalModelAdapter (Normative) [ADD v02.186]
  ### 4.6.1 `ModelRuntime` primitive contract
  ### 4.6.2 `LocalModelAdapter` invariant
  ### 4.6.3 Per-adapter machine-readable capability declarations
  ### 4.6.4 LlmClient routing rule
  ### 4.6.5 Engine selection at model-register time
  ```

#### ANCHOR 4
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/04-llm-infrastructure.md#4.7
- CONTEXT_START_LINE: 22472
- CONTEXT_END_LINE: 22526
- CONTEXT_TOKEN: Inference Research Lab
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 4.7 Inference Research Lab -- Scope and Production Boundary (Normative) [ADD v02.186]
  ### 4.7.1 Eight production techniques (KERNEL-004 scope)
  ### 4.7.2 One spec-deferred technique: Mixture-of-Depths (MoD)
  ### 4.7.3 Per-technique invariants
  ### 4.7.4 Abliteration boundary
  ```

#### ANCHOR 5
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/04-llm-infrastructure.md#4.8
- CONTEXT_START_LINE: 22527
- CONTEXT_END_LINE: 22600
- CONTEXT_TOKEN: Distillation Pipeline
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 4.8 Distillation Pipeline -- Governed-Session-Data Opt-in + Content Review (Normative) [ADD v02.186]
  ### 4.8.1 Opt-in semantics
  ### 4.8.2 Content-review pipeline
  ### 4.8.3 Memory V0+ self-improvement loop (cluster D)
  ### 4.8.4 Reference patterns adopted (and rejected)
  ### 4.8.5 Editable surface for first iteration
  ### 4.8.6 Sandbox + ProcessOwnership integration
  ```

#### ANCHOR 6
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/05-security-and-observability.md#5.6
- CONTEXT_START_LINE: 24353
- CONTEXT_END_LINE: 24399
- CONTEXT_TOKEN: HBR Enforcement as Build-time
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 5.6 HBR Enforcement as Build-time + Handoff-time Gate Authority (Normative) [ADD v02.186]
  ### 5.6.1 Single source-of-truth
  ### 5.6.2 Two HARD evaluation points
  ### 5.6.3 Typed receipts (no prose)
  ### 5.6.4 Rule-change procedure
  ### 5.6.5 Tooling
  ```

#### ANCHOR 7
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/05-security-and-observability.md#5.7
- CONTEXT_START_LINE: 24400
- CONTEXT_END_LINE: 24470
- CONTEXT_TOKEN: ProcessOwnershipLedger
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 5.7 ProcessOwnershipLedger (Normative) [ADD v02.186]
  ### 5.7.1 Primitive + schema
  ### 5.7.2 HARD scope: every Handshake-spawned process
  ### 5.7.3 Perf safeguards (HARD)
  ### 5.7.4 Validator query surface
  ### 5.7.5 Reclaim hooks
  ### 5.7.6 Modification of Section 5.2.5
  ```

#### ANCHOR 8
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/06-mechanical-integrations.md#6.4
- CONTEXT_START_LINE: 31063
- CONTEXT_END_LINE: 31125
- CONTEXT_TOKEN: Visual Debugger
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 6.4 Visual Debugger (Normative) [ADD v02.186]
  ### 6.4.1 Implementation: Playwright over CDP against WebView2
  ### 6.4.2 CI fallback
  ### 6.4.3 Headless screenshot + DOM capture IPC
  ### 6.4.4 Capture matrix support
  ### 6.4.5 WP applicability
  ```

#### ANCHOR 9
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/06-mechanical-integrations.md#6.5
- CONTEXT_START_LINE: 31126
- CONTEXT_END_LINE: 31173
- CONTEXT_TOKEN: Backend Inspector Plane
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 6.5 Backend Inspector Plane (Normative) [ADD v02.186]
  ### 6.5.1 Bind + feature gate
  ### 6.5.2 Compile-time read-only invariant
  ### 6.5.3 Endpoint families
  ### 6.5.4 Mutation routing
  ### 6.5.5 Runtime controls
  ```

#### ANCHOR 10
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/06-mechanical-integrations.md#6.6
- CONTEXT_START_LINE: 31174
- CONTEXT_END_LINE: 31255
- CONTEXT_TOKEN: Non-Hijacking GUI Interaction Invariants
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 6.6 Non-Hijacking GUI Interaction Invariants (Normative) [ADD v02.186]
  ### 6.6.1 Tauri quiet-mode config invariants (HARD per HBR-QUIET-001)
  ### 6.6.2 Screenshots from never-shown windows
  ### 6.6.3 Automation-first design (HARD per HBR-QUIET-002)
  ### 6.6.4 Focus audit subsystem
  ### 6.6.5 Keyboard-injection negative test (HBR-QUIET-002)
  ```

#### ANCHOR 11
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/06-mechanical-integrations.md#6.7
- CONTEXT_START_LINE: 31256
- CONTEXT_END_LINE: 31304
- CONTEXT_TOKEN: Swarm-Agent Harness
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 6.7 Swarm-Agent Harness (Normative) [ADD v02.186]
  ### 6.7.1 Primitive
  ### 6.7.2 Coordination contract
  ### 6.7.3 Parameterization
  ### 6.7.4 Scope distinction from Section 4.3.9
  ### 6.7.5 What the harness surfaces
  ```

#### ANCHOR 12
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/06-mechanical-integrations.md#6.8
- CONTEXT_START_LINE: 31305
- CONTEXT_END_LINE: 31360
- CONTEXT_TOKEN: Mixture-of-Depths Preliminary Research
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 6.8 Mixture-of-Depths Preliminary Research (Informative; deferred implementation) [ADD v02.186]
  ### 6.8.1 Context
  ### 6.8.2 Reference implementations
  ### 6.8.3 Comparison vs. shipped techniques
  ### 6.8.4 Open research questions
  ### 6.8.5 Explicit non-implementation
  ```

#### ANCHOR 13
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/10-product-surfaces.md#10.12
- CONTEXT_START_LINE: 63452
- CONTEXT_END_LINE: 63493
- CONTEXT_TOKEN: Diagnostics Panel
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 10.12 Diagnostics Panel (Visual Debugger Operator Surface) [ADD v02.186]
  Live WebView2 screenshot stream (default 1 Hz), collapsible DOM tree, Playwright command history, per-step screenshots with baseline-comparison overlay, swarm-N session selector, console/network event stream, focus-audit ledger viewer (red flag on Handshake-pid foreground transitions).
  ```

#### ANCHOR 14
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/10-product-surfaces.md#10.13
- CONTEXT_START_LINE: 63494
- CONTEXT_END_LINE: 63536
- CONTEXT_TOKEN: ModelRuntime Control Panel
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 10.13 ModelRuntime Control Panel [ADD v02.186]
  Per-loaded-model id + artifact path + SHA256, active adapter, KV-cache occupancy, LoRA stack (ordered), active steering vectors, ProcessOwnershipLedger row link, live perf stats.
  ```

#### ANCHOR 15
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/10-product-surfaces.md#10.14
- CONTEXT_START_LINE: 63537
- CONTEXT_END_LINE: 63579
- CONTEXT_TOKEN: Inference Lab UI
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 10.14 Inference Lab UI [ADD v02.186]
  8 PRODUCTION technique toggles writing through to settings.exec_policy per model + Work Profile; before/after A/B comparison; LoRA stack composer; KV quant level + prefix-cache TTL; steering vector intensity + layer-index pickers; draft-model picker + speculative mode; subquadratic state-vector persistence + cross-session restore; MoD deferral badge with link to §6.8.
  ```

#### ANCHOR 16
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/10-product-surfaces.md#10.15
- CONTEXT_START_LINE: 63580
- CONTEXT_END_LINE: 63650
- CONTEXT_TOKEN: ModelManual Surface
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 10.15 ModelManual Surface [ADD v02.186]
  Backend Rust source-of-truth at src/backend/handshake_core/src/model_manual/mod.rs as typed Rust structs; Tauri IPC reads (no direct file reads from renderer); on-demand MODEL_MANUAL.md projection via `just generate-model-manual-md` recipe.
  ```

#### ANCHOR 17
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/12-end-of-file-appendices.md#12.4
- CONTEXT_START_LINE: 75644
- CONTEXT_END_LINE: 75750
- CONTEXT_TOKEN: PRIMITIVE_TOOL_TECH_MATRIX
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 12.4 Appendix Block: PRIMITIVE_TOOL_TECH_MATRIX (Machine-readable) [CX-SPEC-APPX-011]
  20 new primitives added per SPEC-ENRICH-6 APPLIED 2026-05-18: PRIM-ModelRuntime, PRIM-LocalModelAdapter, PRIM-LlamaCppRuntime, PRIM-CandleRuntime, PRIM-SandboxAdapter, PRIM-WSL2PodmanAdapter, PRIM-WindowsNativeJailAdapter, PRIM-DockerAdapter, PRIM-ProcessOwnershipLedger, PRIM-HBRGate, PRIM-HBRViolationReceipt, PRIM-MemoryCapsule, PRIM-SelfImprovementLoop, PRIM-DistillationCandidateReview, PRIM-DiagnosticsPanel, PRIM-ModelRuntimeControlPanel, PRIM-InferenceLabUI, PRIM-ModelManual, PRIM-SwarmTestHarness, PRIM-ExecPolicyExtended.
  ```

#### ANCHOR 18
- SPEC_ANCHOR: .GOV/spec/master-spec-v02.186/spec-modules/12-end-of-file-appendices.md#12.6
- CONTEXT_START_LINE: 82537
- CONTEXT_END_LINE: 82650
- CONTEXT_TOKEN: INTERACTION_MATRIX
- EXCERPT_ASCII_ESCAPED:
  ```text
  ## 12.6 Appendix Block: INTERACTION_MATRIX (Feature/Primitive edges) [CX-SPEC-APPX-013]
  12 new IMX edges added per SPEC-ENRICH-6 APPLIED 2026-05-18: IMX-138..149 covering ModelRuntime <-> SandboxAdapter, MemoryCapsule <-> FEMS V1, SelfImprovementLoop <-> PromotionGate + EventLedger, HBRGate <-> PACKET_ACCEPTANCE_MATRIX + ValidationRunner.
  ```
