---
file_id: WP-KERNEL-004-PHASE2-REMEDIATION-INSTRUCTIONS-V1
file_kind: integration_validator_phase2_remediation_brief
updated_at: 2026-05-30T00:00:00Z
wp_id: WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1
authored_by: INTEGRATION_VALIDATOR (Opus 4.8, 1M ctx) Phase-2 post-remediation verdict
authority: PROSE_BRIEF_NON_NORMATIVE
canonical_authority: PHASE2_VERDICT.json (this directory) + RECEIPTS.jsonl
verdict: FAIL
candidate_sha: f0c6faf0d5bdb9600bed999a98b9bd2f06eb998d
---

<topic id="overview" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="Phase-2 verdict, candidate, and the one root cause behind most blockers">

# WP-KERNEL-004 Phase-2 Integration Validator Remediation Brief

## Verdict: FAIL

Phase-1 (Opus 4.7) produced `hardening_report.json` (4 CRITICAL + 64 HIGH + 114 MEDIUM + 181 LOW across 183 MTs). A kernel-builder landed a large remediation wave (candidate commit `f0c6faf0`, 430 files, +96k lines). This Phase-2 pass re-judged that candidate: per-subscope verification of every Phase-1 CRIT+HIGH finding against the current code, the Spec-Realism Gate applied per MT, each sub-review adversarially refuted by an independent verifier, plus 4 cross-cluster coherence checks. Result: **30 blockers across 24 MTs survived adversarial refutation. FAIL.**

The machine-readable record with full file:line evidence per blocker is `PHASE2_VERDICT.json` in this directory. This brief is the human/no-context execution guide.

## How to work this brief (no prior context required)
- Product code lives in worktree `wtc-kernel-004-fold-v1` on branch `feat/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1`, candidate `f0c6faf0`. `.GOV/` is junctioned from `wt-gov-kernel/.GOV/`; do NOT edit `.GOV/` on the feature branch.
- All build/test output MUST go to `../Handshake_Artifacts/` (set `CARGO_TARGET_DIR` or `--target-dir`). There is currently a gitignored leftover `Handshake_Artifacts/` INSIDE the worktree from a prior session — leave it or have the operator authorize deletion; never `rm` it yourself.
- Fix the **THE-ONE-THING** topic FIRST. It unblocks the entire C-cluster Inference Lab set (MT-088/089/095/096/097/098/100/102) at once. Then work the cluster themes.
- A blocker is only "remediated" when the spec behavior runs in the DEPLOYED binary (not just a unit test with a fake adapter) AND a proof command exercises the real path.

## Decisive root cause (read this twice)
The remediation built features at the **library / unit-test / React-UI layer but never wired them into the deployed production runtime + IPC path.** The unit tests pass because they call `attach_live_runtime(model_id, fake)` directly — the exact API the real model-load flow never calls. This is the SAME failure class that caused the original 27-MT reopen: the diff satisfies the implementer's own tests, not the Master Spec behavior at runtime. The Spec-Realism Gate (sub-rule 1: no deferred-live escape on the production path; sub-rule 2: proof must touch the real resource) is failed by most C-cluster MTs for this reason.

</topic>

<topic id="objective-gates" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="What passed and what the test signal really means">

## Objective gates (not the FAIL basis, but context)

- **Build:** PASS. `cargo build --bin handshake` default features, exit 0.
- **Backend `cargo test`:** PASS **except** `cloud_hypervisor_adapter_tests` x3, which fail ONLY under full-suite parallel host-load (many microVMs booting at once vs. a 40s `wait_for_tick` budget). Run isolated single-threaded, **all 9 pass** with real microVM evidence (`snapshot_restore_preserves_running_state`: TICK before=1, after=4; double-restore n0=1/r1=2/r2=2; fs-bind write-back; nonzero exit propagation). Classification: **test-robustness CONCERN, not a product defect.** Cluster-B microVM snapshot/restore genuinely works. Recommended fix: widen `wait_for_tick` budget or serialize CH tests (`#[serial]` / a CH-test mutex) so the suite is deterministic under load.
- **App/Tauri `cargo test`:** NOT RUN this pass — non-decisive. The passing unit tests are part of the problem (they attach fake adapters); the FAIL is established by reading production wiring, which tests cannot exonerate.
- **Artifact hygiene:** CLEAN (no tracked build outputs, no repo-local `target/`). One gitignored in-tree `Handshake_Artifacts/handshake-cargo-target` leftover (untracked, cannot reach `main`) — cleanable pre-merge.

</topic>

<topic id="the-one-thing" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="Wire attach_live_runtime into the real model-load flow — unblocks all of C-cluster steering">

## THE-ONE-THING: bind a real runtime in the deployed app (fixes MT-096/097/100/102 + makes MT-088/089/095/098 testable for real)

**What's wrong (IV-verified directly).** `ModelRuntimeState::attach_live_runtime` is `#[allow(dead_code)] // future production load flow` at `app/src-tauri/src/commands/model_runtime.rs:71`. Every one of its 14 call sites is a `#[tokio::test]` function (verified: `steering.rs:644/765`, `refusal.rs:236/385`, `caa.rs:275/404`, `lora.rs:416/495/546`, `kv_cache.rs:436/509`, `model_runtime.rs:494` all under `#[tokio::test]`). `app/src-tauri/src/lib.rs` — the production Tauri command registration / model-load flow — **never calls `attach_live_runtime` or `detach_live_runtime`.** Consequence in the shipped binary: every steering / refusal / CAA / capability IPC command hits the "no live runtime" branch and returns `capture_not_available`. INF-3 (activation steering) and INF-4 (refusal vector) — declared PRODUCTION in `refinement.json` `inference_technique_matrix` — are non-functional in the product.

**Fix.** In the production model-load command (the Tauri command that loads a model and constructs the `CandleRuntime` adapter — see `app/src-tauri/src/lib.rs` and the model-load path), after `load()` completes and the adapter exposes activation hooks (MT-082), call `ModelRuntimeState::attach_live_runtime(model_id, Arc::new(real_candle_runtime))`. Symmetrically call `detach_live_runtime(model_id)` in the unload command. Remove the `#[allow(dead_code)]` once a real caller exists. The adapter passed must be the REAL `CandleRuntime` (the one with forward-pass hooks), not a fake.

**Proof.** Add an app-level integration test that drives the actual load command (not a direct `attach_live_runtime(fake)`), then issues a steering capture IPC and asserts a real (non-`capture_not_available`) result. The acceptance is: with no special test wiring, loading a hook-bearing model makes `steering`/`refusal`/`caa` commands return real captures in the deployed command set.

</topic>

<topic id="cluster-c-inference" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="C-cluster Inference Lab blockers (most are downstream of THE-ONE-THING)">

## Cluster C — Local Model / Inference Lab

### MT-088 / MT-089 (CRITICAL-1, only half-fixed) — SSM state restore has no runtime effect
- **Where:** `src/backend/handshake_core/src/model_runtime/candle/state_vector.rs`, `ssm_state.rs`, `model_runtime/candle/generate.rs:123`, `mamba2.rs`, `rwkv_v{5,6,7}.rs`.
- **What's wrong:** The snapshot types + `extract_snapshot`/`restore_snapshot` + HMAC/serde round-trip DID land (good). But `generate()` calls `reset_generation_state()` UNCONDITIONALLY before every forward pass (`generate.rs:123`), so for SSM models `prefix_restore` writes live state that is immediately wiped before generation. Restore therefore has **zero semantic effect** in production. The only test proving the wiring uses a `MockSsmSource`, and the e2e identity test (zero-vector steering → output unchanged) is env-gated and never runs in default CI.
- **Fix:** Make `generate()` NOT reset SSM state when a restored/committed prefix snapshot is active; thread the restored state into the model's `State` before the forward pass. Add a real (non-mock, non-env-gated default-CI) test: load tiny Mamba2 → forward 4 tokens → `prefix_commit` → `prefix_restore` → forward 1-token continuation → assert logits equal the post-4-token-forward logits.
- **Proof:** the above test green in default `cargo test` (not env-gated).

### MT-096 / MT-097 (steering capture + review-gate) — downstream of THE-ONE-THING
- **Where:** `model_runtime/techniques/steering_vector_store.rs`, `refusal_vector.rs`, `app/src-tauri/src/commands/steering.rs`.
- **What's wrong:** (a) production never binds a runtime so capture returns `capture_not_available` (see THE-ONE-THING). (b) `ReviewStatus::Pending` is written but NEVER enforced before `set_active`; any vector (Pending or not) can be activated on the production path.
- **Fix:** after THE-ONE-THING, add an `approve(id, approver)` transition and gate `set_active` so a `Pending` vector cannot be activated until approved. Enforce this in the PRODUCTION IPC path, not just the UI.
- **Proof:** test that activating a `Pending` vector via the IPC command errors; activating only succeeds post-`approve`.

### MT-100 / MT-102 (refusal-disable operator gate) — UI-only, production path ungated
- **Where:** `app/src/components/inference_lab/RefusalVectorWizard.tsx`, `app/src-tauri/src/commands/refusal.rs`.
- **What's wrong:** the "I understand this disables safety refusal" acknowledgement + operator gate exist ONLY in the React UI. The production IPC `refusal activate` command itself is ungated, so a non-UI caller activates a refusal-disabling vector with no gate. Both MTs were marked FIXED on UI evidence.
- **Fix:** enforce the operator-acknowledgement + review-gate in the Rust IPC command (server side), not only in TSX. The acknowledgement token must be required by the command handler.
- **Proof:** test that the `refusal activate` command rejects without the acknowledgement/approval, independent of any UI.

### MT-095 — KV-cache TTL / stale-snapshot replay not enforced at live restore
- **Where:** `model_runtime/llama_cpp/kv_cache_impl.rs:389-43x`, the per-adapter KV stacks.
- **What's wrong:** the contract's adversarial path (expired TTL → `Err`) is not enforced at the LIVE restore path; per-adapter KV stacks never store/validate the MT-095 binding, and a direct facade bypass remains.
- **Fix:** enforce TTL + binding validation inside `prefix_commit`/`prefix_restore` on the live path; reject expired/mismatched snapshots loudly.
- **Proof:** live (or realistic) test that a past-TTL restore returns `Err`.

### MT-098 — A/B compare UI absent
- **Where:** Inference Lab UI; spec 10.14.2 side-by-side before/after generation.
- **What's wrong:** only a static help placeholder exists; the spec'd side-by-side before/after generation comparison is not implemented.
- **Fix:** implement the AB-compare surface per spec 10.14.2, wired to real generation (depends on THE-ONE-THING).

</topic>

<topic id="cluster-b-sandbox" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="B-cluster sandbox blockers: network-policy honesty, capability overclaim, escape-catalog stubs">

## Cluster B — Sandbox

### MT-044 / MT-047 (IV-verified) — LoopbackOnly silently becomes deny-all
- **Where:** `src/backend/handshake_core/src/sandbox/wsl2_podman/podman_cli.rs:259-274` (`network_mode`), `sandbox/docker/kernel_003_bridge.rs:~243`.
- **What's wrong:** `NetPolicy::LoopbackOnly => Ok("none")` — identical to `DenyAll` — on BOTH default adapters. A workload that expects to reach a host `127.0.0.1` service (typical LoopbackOnly use) silently gets no network, no error. Violates spec §3.5.7 network-policy honesty.
- **Fix:** implement `slirp4netns` with `allow_host_loopback` for `LoopbackOnly` (the spec'd path), OR return `SandboxAdapterError::NetPolicyApplyFailed` loudly at spawn and require callers to opt into `DenyAll` explicitly. Apply to BOTH adapters.
- **Proof:** test asserting LoopbackOnly either reaches host loopback OR fails loud (not silent none). Tighten the parity fixture (see MT-057).

### MT-046 — GPU passthrough overclaim
- **Where:** `kernel/sandbox/windows_native_jail/capabilities.rs:6-17`.
- **What's wrong:** `windows_native_jail_target_capabilities()` declares `gpu_passthrough = GpuPassthrough::VendorAgnostic` with ZERO GPU plumbing. Violates the truthful-declaration requirement (spec 3.5.4 / 3.6 line 876).
- **Fix:** set `gpu_passthrough = GpuPassthrough::None` for both target + runtime caps until real GPU passthrough ships.
- **Proof:** capability test asserts `None`.

### MT-052 — EROFS enforcement test is an empty stub
- **Where:** `tests/work_packet_scope_binder_tests.rs`.
- **What's wrong:** the contract test (d) + red_team minimum_control #3 (OS-level EROFS enforcement of binder output) is an empty stub; the binder's security claim is unproven.
- **Fix:** add a wsl2-integration-gated test that proves OS-level EROFS enforcement of binder output (write attempt to the read-only mount fails at the OS).

### MT-057 — parity harness auto-pass stubs
- **Where:** `tests/fixtures/cross_adapter_parity/*`, the parity harness.
- **What's wrong:** `stdout_bytes` parity assertion is a self-acknowledged auto-pass stub (strict-parity minimum_control unmet); the `net_loopback_only` fixture uses `expected.exit_code_class="any"`, so it cannot detect the MT-044/047 LoopbackOnly→DenyAll coercion.
- **Fix:** implement real `stdout_bytes` strict parity; change the `net_loopback_only` fixture `exit_code_class` from `any` to `zero` so the silent coercion fails the gate.

### MT-058 — HBR-QUIET acid test cannot run (missing probe binary)
- **Where:** `test_harness/escape_attempts.rs:190,:337`; missing `tests/win32-helpers/handshake-foreground-inject-probe.exe`.
- **What's wrong:** `ESC-WIN32-FOREGROUND-INJECT` (the HBR-QUIET-001 acid test) references a probe binary that does not exist anywhere in the repo, so the test cannot run. `ESC-FS-SYMLINK-TRAVERSAL` is marked `documented_weaker_enforcement_adapters` rather than enforced.
- **Fix:** add the `handshake-foreground-inject-probe.exe` helper under `tests/win32-helpers/` so the foreground-inject escape actually executes; remove the weaker-enforcement doc-exemption for symlink traversal OR switch to a tmpfs/ephemeral mount where symlink-out fails.

</topic>

<topic id="cluster-a-hbr" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="A-cluster HBR VIS/QUIET/SWARM evidence is shape-only; inspector auth partial">

## Cluster A — HBR enforcement evidence

### MT-019 — ControlledWindow is a logging-only stub (SPEC-REALISM sub-rule 1)
- **Where:** `operator_foreground/foreground_exception.rs:262-3xx`.
- **What's wrong:** `ControlledWindow::bounded_window()` returns a struct whose `visible()`/`focused()` methods hardcode `true`; no real Tauri window is created. Violates spec §6.6.7 and the MT-019 contract requirement of a SHOWN, FOCUSED, bounded window.
- **Fix:** instantiate a real Tauri `WebviewWindow` with `.visible(true).focused(true)` constrained to `max_duration` via `tokio::time::timeout`, OR explicitly re-scope MT-019 as declaration-only with a follow-up MT (operator decision). Do not leave a hardcoded-true stub asserting spec behavior.

### MT-020 — automation-first audit is static-only (SPEC-REALISM sub-rule 1)
- **Where:** `.GOV/roles_shared/scripts/automation-first-audit.mjs` + the three contract-required runtime probes.
- **What's wrong:** the audit is static regex source analysis only; the three contract-required RUNTIME probes (IPC mock call, IPC call under live focus-audit, raw `SetForegroundWindow` indirection check) do not run.
- **Fix:** pair with a Rust integration test that spawns the app in test mode, enumerates Tauri commands via IPC, and asserts no `SetForegroundWindow` indirection at runtime.

### MT-027 — focus-audit hardcodes empty events
- **Where:** `tests/visual/a2_smoke.spec.ts:117-141`.
- **What's wrong:** the smoke hardcodes `focus_audit_summary.handshake_owned_events: []` and asserts against the hardcode — it measures nothing.
- **Fix:** spawn the Tauri app with a Rust harness that starts a real `FocusAuditHandle` around the Playwright run so `handshake_owned_events` is real, not `[]`.

### MT-035 / MT-037 — swarm evidence is framework surrogate (SPEC-REALISM)
- **Where:** `tests/swarm_n8_perf_tests.rs`, `tests/swarm_invariants_tests.rs`.
- **What's wrong:** N=8 perf acceptance counters and 2 of 4 swarm invariants (lock/lease, cancellation) are test-framework surrogates measuring nothing about the real platform (a closed-form simulator + an AtomicBool poll-loop). (Loop-counter HBR-SWARM-002 IS genuinely tested — preserve it.)
- **Fix:** replace the simulator with N=8 real `SwarmHarness` sessions dispatching through a real `KernelActionCatalogV1` against a real CRDT workspace; replace the AtomicBool cancellation invariant with a real `CancelMidMutationScenario` issuing real `MutateViaCatalog` steps.

### MT-029 — inspector auth partial (spec §6.5.5)
- **Where:** `inspector_read/replay_drive.rs`.
- **What's wrong:** the per-run shared-secret header is enforced ONLY on the `/inspector/v1/replay-drive` POST (write) endpoint, not on the read endpoints (spec §6.5.5 requires it on reads too). Reject audit-log does not capture `peer_addr` (spec requires every reject logged with timestamp, route, peer_addr, reason). Note: the HMAC verifier itself (CRITICAL-4) DID land and is good; the per-run secret is `Uuid::now_v7()` (time-ordered, ~48 predictable bits) — acceptable under the spec's looser wording but weaker than "unforgeable"; consider a CSPRNG secret.
- **Fix:** enforce the secret header on read endpoints; log `peer_addr` on every reject.

### MT-011 — config_key detector false-positive flood (Phase-1 HIGH, still unresolved)
- **Where:** `hbr-man-001-paired-diff.mjs` detector.
- **What's wrong:** the `config_key` detector false-positive flood (the original HIGH) is REAL and UNRESOLVED on the canonical CI path.
- **Fix:** narrow the detector to struct-definition bodies only (per Phase-1 Theme 12 hint).

</topic>

<topic id="cluster-cd-gates" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="Promotion gate / process ownership not enforced on production paths; orphan reclaim deferred">

## Cluster C/D/X — gates on production paths

### MT-122 / MT-123 — PromotionGate + ProcessOwnership absent from production
- **Where:** `distillation/peft_pipeline.rs:~248-260,:350`, `distillation/candidate_registry.rs`.
- **What's wrong:** `mount_with_promotion_gate()` is never called on the production LoRA mount path; `CandidateRegistry::mount_status` is not wired to `LoraStackOps::mount`, so AC-DISTILL-LOOP-SAFEGUARDS (the sole, HARD acceptance criterion for MT-123) is unenforced in production. `ProcessOwnershipLedger` registration in `PythonPeftTrainerExecutor` is CONDITIONAL on an optional `with_process_ledger()` — when not attached, no ledger row is recorded (coherence check `distill-sandbox` = PARTIAL). (`TrainerUnavailable` itself is an acceptable typed error for a genuinely-absent external trainer; the blocker is the unconditional ledger + gate wiring, not the typed error.)
- **Fix:** wire `LoraStackOps::mount` → `CandidateRegistry::mount_status` → `PromotionGate` enforcement on the production mount path; make `ProcessOwnershipLedger` registration unconditional inside the trainer spawn (not an optional builder).
- **Proof:** test that a mount of an unpromoted candidate is rejected on the production path; that a trainer spawn always records a ledger row.

### MT-186 — forced-cancellation orphan reclaim + escalation deferred to caller
- **Where:** `mt_executor/cancellation.rs`.
- **What's wrong:** `ProcessOwnershipLedger` reclaim is deferred to a caller-registered `MtCancellationCleanupHook` (red_team minimum_control #1 requires forced cancellation to ALWAYS reclaim or fail loudly — orphan risk if the caller forgets); the `force_after_secs` cooperative→forced escalation timer is also deferred to the caller (MT-186.json:33 requires `MtCanceller::force` to fire it built-in).
- **Fix:** make ledger reclaim a built-in step inside `MtCanceller::force` (always runs); add a `request_with_force_after` helper that flips the cooperative cancellation token, awaits drain up to `force_after_secs`, then escalates to forced kill + ledger reclaim.
- **Proof:** test asserting forced cancellation always reclaims (no orphan) and the escalation timer fires built-in.

</topic>

<topic id="not-blockers" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="Things adversarial verification cleared — do NOT spend remediation effort here">

## NOT blockers — do not waste effort (adversarial verification cleared these)

23 sub-reviewer claims were OVERTURNED by adversarial verification as fabricated requirements or over-harsh reads. Examples:
- **MT-007** (process-ledger writer): the demand for an `FR_EVT_LEDGER_STORE_FAILED` event is FABRICATED — spec §5.7.3 only mandates `FR-EVT-LEDGER-OVERFLOW`, which IS implemented. Silent flush-error discard is a CONCERN-class observability gap, not a spec violation. (Optional hardening, not a blocker.)
- **MT-008** (reclaim FOR UPDATE scope): the code-level race is real but is CONCERN-class against the actual spec wording, not a HARD blocker.
- **MT-159 / MT-160** (hygiene pin visibility, D.3): adversarial rated the subscope REVIEW_TOO_HARSH (claims overturned) — treat as CONCERN pending operator judgment, not a hard blocker.
- **CH test failures:** host-load timing flakiness, NOT a product defect (microVM snapshot/restore genuinely works isolated). Fix = test determinism (serialize CH tests), not product code.
- Confirmed-correct positives from Phase-1 "don't-touch" still hold: EAGLE-3 stub (spec-deferred), MoD (operator E-1 deferred), OFFLINE-ONLY abliteration discipline, LM Studio/Ollama wrapping invariants, HBR-SWARM-002 loop counter, UUIDv7 usage, no API-key leakage, no shell injection in CLI bridge, `WindowsNativeJailAdapter` wraps `rappct`. Do not regress these.

See `PHASE2_VERDICT.json` `concern_class_overturned` for the full overturned list.

</topic>

<topic id="completion-signal" status="active" wp="WP-KERNEL-004" owner="KERNEL_BUILDER" summary="How to declare the Phase-2 wave complete">

## How to declare the Phase-2 remediation wave complete

1. Land code + tests on `feat/WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1` in `wtc-kernel-004-fold-v1` (no `.GOV/` edits on the feature branch).
2. For each remediated blocker, the bar is: the spec behavior runs in the DEPLOYED binary AND a non-mock proof command exercises the real path (Spec-Realism Gate sub-rules 1 & 2).
3. Re-run: `cargo build`, full backend `cargo test` (serialize CH tests), app/Tauri `cargo test`, and the THE-ONE-THING production-load integration test.
4. Emit a `CODER_HANDOFF` receipt to `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-004-.../RECEIPTS.jsonl` with `verb_body.remediated_blockers: [<mt ids>]` and `verb_body.commit_sha`, then request a fresh Integration Validator Phase-3 pass.
5. Do NOT edit `PHASE2_VERDICT.json` or this brief — they are Phase-2 history. A fresh IV will re-judge.

</topic>
