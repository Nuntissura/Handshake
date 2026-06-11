---
file_id: WP-KERNEL-004-REMEDIATION-INSTRUCTIONS-V1
file_kind: kernel_builder_remediation_brief
updated_at: 2026-05-28T00:30:00Z
wp_id: WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1
authored_by: INTEGRATION_VALIDATOR (Opus 4.7) Phase-1 retrospective
authority: PROSE_BRIEF_NON_NORMATIVE
canonical_authority: hardening_report.json + RECEIPTS.jsonl
---

<topic id="overview" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="Where remediation work starts and what counts as done">

# WP-KERNEL-004 Phase-1 Kernel Builder Remediation Brief

## Purpose
Phase-1 Integration Validator pass is complete. 79 PASS / 52 CONCERN / 19 FAIL across 183 MTs with 4 CRITICAL + 64 HIGH + 114 MEDIUM + 181 LOW findings. This brief is your starting point for remediation before Phase-2 (whole product-code vs Master Spec for intent + correctness).

## Three artifact surfaces — what to use when

| Surface | Path | What it gives you |
|---|---|---|
| **Bundle / triage** | `.GOV/task_packets/WP-KERNEL-004-.../hardening_report.json` | Top-30 remediation priorities, all 68 CRIT+HIGH with file_refs and remediation_hints, cluster + subscope breakdowns, hbr_unsatisfied_by_rule index |
| **Per-MT condensed** | `.GOV/task_packets/WP-KERNEL-004-.../MT-NNN.json` → `integration_validator_review` block | Per-MT verdict + per-axis statuses + finding counts; `handoff.review_response_receipt_id` is the UUID to find the full receipt |
| **Per-MT full** | `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-004-.../RECEIPTS.jsonl` | Authoritative `REVIEW_RESPONSE` receipts (`verb=MT_VERDICT`, `track=JUDGMENT_OOB_IV_PHASE1_PRE_REMEDIATION`) — full concerns array with category/severity/file_refs/spec_anchor/remediation_hint per finding |

## Working tree
All product code is on branch `feat/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1` in worktree `wtc-kernel-004-fold-v1`. `.GOV/` is junctioned from `wt-gov-kernel/.GOV/`, so any governance file edit lands in both immediately.

</topic>

<topic id="workflow" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="How to pick MTs, fix, and declare done">

## Recommended workflow

1. **Read** `hardening_report.json` — `critical_and_high_findings` is severity-sorted; `remediation_priorities[0..29]` is the top-30 ranked list.
2. **Group your run** — fix related findings together; many High themes are 2–4 MTs sharing the same code area (see "Themes" topic below).
3. **For each MT** — read `MT-NNN.json.integration_validator_review` for a quick verdict + status panel; if you need full finding detail, grep `RECEIPTS.jsonl` for the MT's `handoff.review_response_receipt_id`.
4. **Fix** — code lives in `wtc-kernel-004-fold-v1` on `feat/WP-KERNEL-004-...`. Commit per normal MT discipline (no `.GOV/` edits on feature branches).
5. **Declare MT remediated** — emit a new typed receipt to `RECEIPTS.jsonl` (see "completion-signal" topic).
6. **Do NOT** edit `hardening_report.json` or rewrite prior `integration_validator_review` blocks. Phase-2 IV will re-judge after your wave lands.

## Heuristic risk MTs
Some MTs are `HEURISTIC_RISK=YES` (e.g., MT-046 windows-jail). RGF-100 bounded fix loop applies: after 3 same-threshold repair attempts, escalate to strategy redesign rather than another loop.

</topic>

<topic id="priority-order" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="Severity ladder and parallelization">

## Priority ladder

1. **4 CRITICAL** — fix first. All four are foundational; downstream MTs depend on them or build on incorrect ground.
2. **64 HIGH** — grouped into ~15 themes (next topic). Many themes can be parallel-fixed by separate sessions if you have agent swarm capacity.
3. **114 MEDIUM** — most are paperwork drift, test-tightening, or capability-honesty edits. Grep `RECEIPTS.jsonl` for full list per MT.
4. **181 LOW** — mostly hygiene / comments / naming. Batch at the end.

## Cluster health (use as workload signal)
- **A** (HBR enforcement + tooling): 21 PASS, 17 CONCERN, 2 FAIL — moderate work, mostly tightening
- **B** (Sandbox): 7 PASS, 11 CONCERN, 0 FAIL — concerns concentrated in parity/escape catalog
- **C** (Local model + Inference): 29 PASS, 18 CONCERN, **15 FAIL** — biggest workload, mostly cloud lane + SSM parity + steering wiring
- **D** (Memory + Self-improvement): 13 PASS, 3 CONCERN, 2 FAIL — concentrated in promotion gate + hygiene pin gap
- **X** (Preservation): 9 PASS, 3 CONCERN, 0 FAIL — least work; MT-186 process reclaim is the key one

</topic>

<topic id="critical-findings" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="The 4 CRITICAL findings — fix first">

## CRITICAL 1 / 4 — MT-088 (C) StateVectorHandle decoupled from live SSM model state

**Detail.** `InMemoryStateVectorOps.current_snapshot` Mutex is never populated from the live model. `CandleMamba2Model.state` / `CandleRwkvV{5,6,7}Model.state` get mutated by `Model::forward` but have no path to (a) extract a tensor-byte snapshot into `SSMTensorSnapshot`, or (b) inject one back into `State::new`. `prefix_commit` and `prefix_restore` on the SSM handle therefore operate on a placeholder empty snapshot. The e2e smoke "passes" only because `reset_generation_state()` runs before every forward, so identical prompts produce identical output regardless of whether commit/restore ran.

**Files.** `src/backend/handshake_core/src/model_runtime/candle/adapter.rs:204-292` (4 SSM construction sites), `state_vector.rs:548-573`, `state_vector.rs:612-653`, `mamba2.rs:78-86`, `rwkv_v5.rs:77-84`.

**Remediation.** Add `extract_snapshot(&self) -> Result<SSMStateSnapshot, ...>` and `restore_snapshot(&mut self, snapshot: SSMStateSnapshot) -> Result<(), ...>` to a new `SsmModel` sub-trait (or `TransformerModel` for SSM variants). Have `CandleMamba2Model::extract_snapshot` serialise `self.state.conv_states + self.state.ssm_states` to `SSMTensorSnapshot` bytes. Have `CandleRuntime`'s state-vector handle hold a back-reference to the model's `Arc<Mutex<...>>` so `prefix_commit` pulls from live state and `prefix_restore` writes back.

**Verify.** Add a test: load tiny Mamba2 → forward 4 tokens → `prefix_commit` → reset → `prefix_restore` → forward same 1-token continuation → assert logits == post-4-token-forward logits. This also fixes MT-088 e2e structural defect and MT-089 (which inherits the same flaw) and unblocks MT-117 cross-session restore.

---

## CRITICAL 2 / 4 — MT-107 (C) Owned abliterate-review files not in feat branch

**Detail.** `git ls-tree -r feat/WP-KERNEL-004-...` returns no `abliterate_review.rs` and no `abliterate_review_tests.rs`. `git log --all --oneline -- '**abliterate_review*'` returns zero file-touching commits. The contract cites passing test logs at a `Handshake_Artifacts/wp-kernel-004-test-runs/` path that does not exist on disk. MT was marked READY_FOR_VALIDATION without landing code.

**Files.** `src/backend/handshake_core/src/distillation/abliterate_review.rs` (MISSING), `src/backend/handshake_core/tests/abliterate_review_tests.rs` (MISSING), `src/backend/handshake_core/src/distillation/mod.rs` (no `abliterate_review` mod entry).

**Remediation.** Pick one:
- (a) Reopen MT-107 as NEEDS_REIMPLEMENTATION and land `abliterate_review.rs` + tests on feat per the contract's `implementation_notes`. Same for MT-108 which has the same defect (`abliteration_offline_invariant_tests.rs` MISSING).
- (b) If operator decision E-5 ("no curated abliterated models in registry") now applies hard enough that no in-process review gate is needed, retire MT-107 with an explicit packet revision and a recorded operator decision pointing back to E-5. **Do not leave it in READY_FOR_VALIDATION with no code in branch.**

---

## CRITICAL 3 / 4 — MT-012 (A) Live HBR-MAN-003 scan fails on `memory_calibration_snapshot`

**Detail.** Running `node hbr-man-003-scan.mjs --repo-root ../wtc-kernel-004-fold-v1 --gov-root ../wt-gov-kernel/.GOV` exits 2 with:
- `{ manual_id: memory_calibration_snapshot, name: memory_calibration_snapshot, kind: command_name, reason: no_source_match }`
- 6 schema_field failures for `signals.{bloat, stale_dominance, trust_drift, embedding_gap, degradation_rate, hygiene_lag}`

The actual Rust function is `pub async fn kernel_memory_calibration_snapshot` at `app/src-tauri/src/commands/memory_calibration.rs:103`, and the signal fields exist inside a `signals` field of the snapshot struct (dotted path). This is the live manual; HBR-MAN-003 is currently violated.

**Files.** `src/backend/handshake_core/src/model_manual/content.rs`, `app/src-tauri/src/commands/memory_calibration.rs`.

**Remediation.** Update `content.rs` so `name='kernel_memory_calibration_snapshot'` and `schema_fields = ['bloat','stale_dominance','trust_drift','embedding_gap','degradation_rate','hygiene_lag','signal_errors']`. (Optional follow-up MT: extend scanner to recognize dotted paths — but minimal fix is the rename.)

---

## CRITICAL 4 / 4 — MT-030 (A) Keyless WriteBoxV1 signature

**Detail.** `expected_write_box_v1_signature(signer, write_box) = sha256(canonical_json({schema_id, signer, write_box}))`. The signer string is part of the input — **no private key, no HMAC, no per-run secret**. The verifier in `Kernel002WriteBoxEnvelopeVerifier` compares envelope signature to the recomputed hash and accepts. Any local process can forge a passing envelope under any signer name. This is the agent-corrupts-agent path for A.3.

**Files.** `src/backend/handshake_core/src/inspector_read/replay_drive.rs`.

**Remediation.** Use HMAC-SHA256 with a per-run shared secret (the same secret as the MT-029 per-run header — fix MT-029 in the same change). Store the secret in operator-visible state at launch; rotate per run. Reject any envelope whose HMAC does not match. The IV reviewer noted the MT-030 claim "uses KERNEL-002 verifier (not a duplicated implementation)" is false — either land a real KERNEL-002 HMAC verifier and have `replay_drive` call it, or update the claim.

</topic>

<topic id="high-themes" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="HIGH findings grouped into ~15 fix themes; each theme is a coherent unit of remediation work">

## HIGH theme map (group fixes by theme for efficient sessions)

### Theme 1 — Sandbox NetPolicy::LoopbackOnly silent coercion (B)
MTs: **MT-044, MT-047, MT-057** | Files: `kernel/sandbox/wsl2_podman/podman_cli.rs:259-274`, `sandbox/docker/kernel_003_bridge.rs:243-257`, `tests/fixtures/cross_adapter_parity/net_loopback_only`
Fix: Implement slirp4netns with `allow_host_loopback` for LoopbackOnly OR return `SandboxAdapterError::NetPolicyUnsupported` loudly at spawn. Tighten the parity fixture `expected.exit_code_class` from "any" to "zero" so the silent coercion fails the parity gate.

### Theme 2 — Sandbox capability overclaim + KERNEL-003 phantom (B)
MTs: **MT-046, MT-047, MT-050, MT-054** | Files: `kernel/sandbox/windows_native_jail/capabilities.rs:6-17`, `sandbox/docker/kernel_003_bridge.rs:37-56`
Fix: Set `gpu_passthrough=GpuPassthrough::None` for both target/runtime caps until real GPU pass-through ships. Re-frame MT-047/050/054 contracts: KERNEL-003 had no `DocketAdapter` to refactor (`non_executing_stub_no_docket_adapter_found`); reword goal as "establish canonical sandbox-routed validation lane" rather than "preserve parity vs prior".

### Theme 3 — Sandbox escape catalog gaps (B)
MTs: **MT-052, MT-058** | Files: `tests/work_packet_scope_binder_tests.rs`, `test_harness/escape_attempts.rs:190, :337`
Fix: Add wsl2-integration-gated test for OS-level EROFS enforcement of binder output. Remove `documented_weaker_enforcement_adapters` for `ESC-FS-SYMLINK-TRAVERSAL` OR switch to a tmpfs/ephemeral mount where symlink-out fails. Add the `handshake-foreground-inject-probe.exe` helper under `tests/win32-helpers/` so `ESC-WIN32-FOREGROUND-INJECT` actually runs.

### Theme 4 — C cluster production wiring (steering + cloud lane FR-EVT)
MTs: **MT-096, MT-125, MT-126, MT-127, MT-128, MT-130** | Files: `app/src-tauri/src/lib.rs:861-895`, `model_runtime/cloud/openai_byok.rs`, `anthropic_byok.rs`, `official_cli_bridge.rs`, `consent_gate.rs`, `tests/cloud_local_parity_smoke.rs`
Fix: (a) The model-load flow MUST call `ModelRuntimeState::attach_live_runtime(model_id, runtime)` so the deployed Tauri binds a real `CandleRuntime` — currently `#[cfg(test)]`-only, so every steering command returns `capture_not_available` in production. (b) Call `infer_start_event` / `infer_token_event` / `infer_end_event` from cloud + CLI bridge runtimes — currently they emit only their own `CloudInvocationAuditRow`, and the FR helper hardcodes `adapter: "llama_cpp"`. (c) Pass `ConsentGate + session_id` into the cloud-runtime call path and consult before HTTP — `cloud_consent_per_session=true` is currently toothless. After (a)+(b) land, rewrite `cloud_local_parity_smoke.rs::fr_event_payload_smoke` to capture events through a shared `FlightRecorder` instead of mutating a payload clone.

### Theme 5 — C cluster refusal/activation operator-gating (Arditi 2024 fidelity)
MTs: **MT-097, MT-100, MT-101, MT-102** | Files: `model_runtime/techniques/refusal_vector.rs:121-14X`, `techniques/steering_vector_store.rs`, `app/src/components/inference_lab/RefusalVectorWizard.tsx`
Fix: (a) `apply_vector_to_activation` is purely additive (`base + steering*intensity`). For `ContrastiveTechnique::RefusalVector`, branch into actual orthogonalisation: `new_resid = resid - (resid·dir)*dir`. (b) `ReviewStatus::Pending` is written but never enforced before `set_active`; add `approve(id, approver)` transition and gate activation. (c) Refusal vectors require `operator_acknowledgment` step before Save fires — surface a confirmation panel: "[ ] I understand this vector is designed to disable safety refusal". (d) Per-layer bar chart shows L2 norm of unit vector (always 1.0) — wire a second IPC to measure with runtime.

### Theme 6 — C cluster SSM full-parity (operator E-2 breach)
MTs: **MT-088, MT-089, MT-115, MT-116, MT-118** | Files: `model_runtime/candle/state_vector.rs`, `mamba2.rs`, `rwkv_v{5,6,7}.rs`, `model_runtime/candle/ssm_lora.rs`, `candle/hooks.rs:553-654`, `tests/inf9_subquadratic_full_parity_smoke.rs`
Fix: After CRITICAL-1 (MT-088 state vector) lands, author the follow-on weight-application MT covering LoRA-for-SSM + activation-steering-for-SSM forward-pass wiring (hook into candle-transformers, same crate work unblocks both MT-115 and MT-116). Then gate WP-KERNEL-004 INF-9 acceptance on (a) the follow-on MT lands with `supports_lora=true` + `supports_activation_steering=true` for SSM variants, (b) MT-118 parity smoke STEP-1/2/3 stop emitting DEFERRED markers and actually exercise parity.

### Theme 7 — C cluster distillation pipeline holes
MTs: **MT-122, MT-123** | Files: `distillation/peft_pipeline.rs:~248-260`, `distillation/candidate_registry.rs`
Fix: Wrap `PythonPeftTrainerExecutor::run()` in a `SandboxAdapter` spawn that `env_clear()` + selectively injects only `HANDSHAKE_DISTILL_*` env vars. Register a `ProcessOwnershipLedger` row. Wire `LoraStackOps::mount` → `CandidateRegistry::mount_status` so PromotionGate mount-side enforcement actually runs.

### Theme 8 — C cluster correctness drift vs research
MTs: **MT-082, MT-095, MT-100 (Arditi reproduction)** | Files: `model_runtime/candle/hooks.rs:407-414`, `llama_cpp/kv_cache_impl.rs:392`, `tests/inf4_refusal_vector_tests.rs:431-481`
Fix: Remove bare `CandleSteeringHooks::capture` or route all callers through `CandleRuntimeSteeringHookOps` (real model forward) — the bare path returns synthetic prompt-hashed rows. Wire `BLAKE3::from_derived` inside both adapter `prefix_commit` impls so cache entries store `registered_at_utc` and handle replay-resistance (currently dead code at live path). Adopt MT-099 pattern for Arditi end-to-end reproduction: deterministic dyn ModelRuntime mock with synthetic refusal direction + synthetic completions.

### Theme 9 — D cluster promotion gate integrity
MTs: **MT-154, MT-155** | Files: `self_improve/promotion_gate_adapter.rs`, `self_improve/ipc.rs`
Fix: In `LoopPromotionGate::submit`, after `self.gate.submit(request.clone())` returns ticket, assert `ticket.iteration_id == request.iteration_id`. Replace the `Pending` branch in `approve_promotion` with `Err(LoopIpcError::Gate(GateError::ReviewPending))` and leave pending_reviews/pending_tickets intact — operator IPC must NOT synthesize approval. Validate `request.iteration_id == ticket.iteration_id` in `submit_for_review`.

### Theme 10 — D cluster hygiene pin gap
MTs: **MT-159, MT-160** | Files: `memory/hygiene.rs:711-733, :490`
Fix: Project pin/unpin actions back into the bitemporal item payload via a follow-up ledger event, OR extend `HygieneItemView` to query the pin aggregate. Add a live test: pin via `PinIpcService` → list via `PostgresBitemporalMemoryIndex.list_items` → assert pin visible → hygiene skip.

### Theme 11 — D cluster redaction policy
MTs: **MT-165** | Files: `memory/trace_export.rs`
Fix: After loading artifact bytes, if `policy.redact_pii || policy.redact_credentials`, interpret bytes as UTF-8 and run through the redactor BEFORE tar embedding. Either ship the NER pipeline (wire to existing redactor in `content_review.rs`) or update the contract + ACs to "regex-only V1, NER V2".

### Theme 12 — A cluster HBR registry version drift
MTs: **MT-001, MT-002, MT-004** | Files: `.GOV/roles_shared/scripts/hbr-registry-loader.mjs:138-145`, `hbr-matrix-hydrate.mjs:158-177`, `hbr/handoff_gate.rs:74-110`
Fix: Update self-test to 29 active rules (current registry is v1.3.0 with STOP pillar added; tests still assert v1.1.0 24-rule shape). Add invariant: `PROVED → PENDING` downgrade is explicitly blocked. Add STOP pillar `evidence_kinds` to the CoderToWpValidator transition (or document STOP as build-time-only).

### Theme 13 — A cluster process ledger writer/reclaim safety
MTs: **MT-007, MT-008** | Files: `process_ledger/writer.rs:347, :352, :379-383`, `process_ledger/reclaim.rs:247-252`
Fix: Replace silent error drops in `run_writer` with `tracing` log + typed `FR_EVT_LEDGER_STORE_FAILED` event + bounded retry. In reclaim, hold the transaction open across the entire reclaim loop (pass `tx` into `kill()` / `append_stop()`) OR use `SELECT ... FOR UPDATE SKIP LOCKED` and re-acquire per-row inside the loop.

### Theme 14 — A cluster HBR-VIS / QUIET test completeness
MTs: **MT-019, MT-020, MT-027** | Files: `operator_foreground/foreground_exception.rs:262-3XX`, `.GOV/roles_shared/scripts/automation-first-audit.mjs`, `tests/visual/a2_smoke.spec.ts:117-141`
Fix: `ControlledWindow.bounded_window()` is a logging shell — either instantiate a real Tauri WebviewWindow with `.visible(true).focused(true)` constrained to `max_duration` via `tokio::time::timeout`, OR explicitly mark MT-019 as declaration-only with a follow-up MT (don't half-build the spec). Extend `automation-first-audit` (or pair with a Rust integration test) to spawn the app in test mode, enumerate Tauri commands via IPC, and assert no SetForegroundWindow indirection. In `a2_smoke.spec.ts`, spawn the Tauri app with a Rust harness that starts `FocusAuditHandle` around the Playwright run so `handshake_owned_events` is real, not hardcoded `[]`.

### Theme 15 — A cluster inspector authentication
MTs: **MT-029, MT-030** | See CRITICAL-4. MT-029 needs the per-run secret header (spec §6.5.5); MT-030 needs the HMAC verifier. Same per-launch UUID-v7 secret serves both. Fix together.

### Theme 16 — A cluster swarm evidence shape-only
MTs: **MT-035, MT-037** | Files: `tests/swarm_n8_perf_tests.rs`, `tests/swarm_invariants_tests.rs`
Fix: Replace `generate_attempts_concurrently + apply_attempts` (closed-form simulator) with N=8 `SwarmHarness` sessions dispatching through a real `KernelActionCatalogV1` against a real CRDT workspace. Replace the AtomicBool poll-loop cancellation invariant with `SwarmHarness::new(8, CancelMidMutationScenario).run()` issuing real `MutateViaCatalog` steps. Loop-counter HBR-SWARM-002 is genuinely tested — preserve.

### Theme 17 — X cluster process orphan + escalation
MTs: **MT-186** | Files: `mt_executor/cancellation.rs`
Fix: Make `ProcessOwnershipLedger` reclaim a built-in step inside `MtCanceller::force` (always runs, even if caller forgets). Add `request_with_force_after` helper that flips the cancellation token cooperatively, awaits drain up to `force_after_secs`, then escalates to forced kill + ledger reclaim.

</topic>

<topic id="untracked-files" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="Several MTs have correct code that simply isn't committed yet">

## Files-untracked-in-git cleanup

These MTs were marked CONCERN/FAIL primarily because their owned files exist on disk but were never committed to `feat/WP-KERNEL-004-...`. Most of the code is correct; just commit it. Do **one chained commit** per MT (or one across the group) to satisfy HBR-MAN-001 same-commit gate.

| MT | Files (commit on feat) |
|---|---|
| MT-009 | `src/backend/handshake_core/tests/hbr_e2e_smoke_test.rs` |
| MT-010 | `src/backend/handshake_core/src/model_manual/` (entire module + tests) |
| MT-012 | `.GOV/roles_shared/checks/hbr-man-003-scan.mjs` + the renamed `content.rs` from CRITICAL-3 |
| MT-013 | `app/src-tauri/src/manual.rs` + the full MT-013 deliverables set |

After commits land, rerun MT-011 paired-diff hook to confirm zero false-positive flood on the newly-tracked Rust files. If false-positives persist, see Theme 12's MT-011 hint: narrow detector to struct-definition bodies only.

</topic>

<topic id="dont-touch" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="Findings that are NOT failures — leave them alone or treat as already-correct">

## Adversarial review confirmed these are NOT problems — do NOT touch

The Phase-1 reviewers tried to break these and could not. Treat as load-bearing positives:

- **HBR-QUIET hard rule held** — no code path pops a window or steals focus under any audit. Don't introduce one during remediation.
- **LM Studio / Ollama wrapping as runtime authority** — invariant intact (multi-casing + loopback-port reject + origin-compaction). Don't add an ExternalEngineImport path that smells like authority delegation.
- **No API key leakage in logs / FR events** — `Debug` redaction verified; keys only in Authorization/x-api-key headers.
- **No shell injection in CLI bridge** — argv-array spawn, no shell. Don't introduce string-interpolation in `LiveCliSpawner`.
- **WindowsNativeJailAdapter wraps `rappct`** — NOT hand-rolled. Don't introduce raw windows-sys jail primitives.
- **C.9 OFFLINE-ONLY abliteration discipline preserved** — no curated abliterated-model registry path exists. Don't add one.
- **C.10 EAGLE-3 stub-only** — `EAGLE3_AVAILABLE = false` const; typed `eagle3_deferred` error; no active impl. Don't promote the stub.
- **MoD discipline clean** — Mixture-of-Depths is spec-deferred per operator E-1; no MoD code in scope. Don't add it.
- **HBR-SWARM-002 loop counter** — genuinely tested against production `HbrSwarmLoopCounter`. Don't regress.
- **HBR-INT-008 UUIDv7** — used throughout (`now_v7()`). Keep.

</topic>

<topic id="completion-signal" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="How to declare an MT remediated">

## How to declare an MT remediated

Per WP_VALIDATOR_PROTOCOL receipt-wire convention (CX-130):

1. Land code + tests on `feat/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1` in `wtc-kernel-004-fold-v1`.
2. Emit a new typed receipt to `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-004-.../RECEIPTS.jsonl` via `just wp-receipt-append` or the equivalent helper:
   - `actor_role`: `CODER`
   - `actor_session`: your kernel-builder session id
   - `receipt_kind`: `CODER_HANDOFF` (re-submitting for validation) or `STATUS` (in-progress)
   - `verb`: `MT_REMEDIATION_REQUIRED` if your fix is partial / waiting on input, or rely on the next IV pass to emit a fresh `MT_VERDICT`
   - `verb_body.mt_id` + `verb_body.remediated_findings: [<finding_titles>]` + `verb_body.commit_sha`
   - `correlation_id`: a new UUIDv7
   - `ack_for`: the Phase-1 receipt correlation_id from `MT.json.handoff.review_response_receipt_id` (links the remediation back to the original verdict)
3. Update `MT.json.lifecycle.status` to `READY_FOR_VALIDATION_PHASE2` (the existing convention; bump `updated_at_utc`).
4. Do **not** touch `MT.json.integration_validator_review` or `MT.json.handoff.review_response_receipt_id` — those are Phase-1 history. Phase-2 IV will overwrite during its judgment.

</topic>

<topic id="phase-2" status="active" wp="WP-KERNEL-004" owner="INTEGRATION_VALIDATOR" summary="What happens after remediation">

## Phase 2 (after kernel-builder declares wave complete)

A fresh Integration Validator session will do a whole-product-code vs Master Spec pass for **intent + correctness** (not per-MT). Phase 2 catches cross-MT consistency gaps no per-MT review can see and verifies the WP delivers a coherent design rather than 183 disconnected fixes. Phase 2 will:

- Re-judge every MT that emitted a `MT_REMEDIATION_REQUIRED` or `CODER_HANDOFF` Phase-1 ack receipt
- Audit cross-cluster boundary contracts (e.g., does the steering wiring in MT-096 actually integrate with the SSM state in MT-088 after both are fixed?)
- Verify the `hardening_report.json` priority-30 are all closed or operator-waived
- Emit a fresh whole-WP `INTEGRATION_VERDICT` receipt (PASS / FAIL) and update `lifecycle.validator_audit_summary` in `packet.json`

Trigger when the wave is ready by emitting an `INTEGRATION_VALIDATOR_PHASE2_REQUESTED` receipt or notifying the operator directly.

</topic>
