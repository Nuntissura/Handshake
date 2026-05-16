---
file_id: integration-validation-appendage-20260516-mt-rerun-6
file_kind: validation_appendage
updated_at: 2026-05-16T03:02:59Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
session_id: INTEGRATION_VALIDATOR-20260516-025633
candidate_worktree: ../wtc-preuse-hardening-v1
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
candidate_head_sha: 19bcc83bdcc7981d8da33b0f9c4201987696ba2d
baseline_main_sha: e11ba59793490028262089f782523eb51cf1f1f7
mt_batch_verdict: FAIL
whole_wp_master_spec_validation: NOT_RUN
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T03:02:59Z" ingestable="true" summary="Operator-waived MT rerun after Kernel Builder six-item remediation.">

Handshake Product validation reviewed the Kernel Builder remediation claims as leads, not proof.

This was the Operator-waived MT-first path. Whole-WP code-vs-current-Master-Spec validation was not run because the MT prerequisite still fails.

Candidate readiness issue:
- `../wtc-preuse-hardening-v1` is dirty on top of `19bcc83bdcc7981d8da33b0f9c4201987696ba2d`; 14 product files are modified and uncommitted.
- Dirty state alone blocks any merge/PASS closeout, even if the MT batch were otherwise clean.

Mechanical drift classification before this appendage:
- Runtime route drift: a spawn-tree projection API exists, but the DCC surface route consumed by the app does not carry that projection.
- Session/runtime-record drift: tests render fixture/runtime-record payloads but do not prove the active DCC route reads live runtime records.
- Documentation/protocol drift: the root `handshake_main` justfile lacks `task-packet-stub-contracts`, while the kernel justfile has it; product receipts do not state this workdir boundary.
- Scope/worktree drift: remediation is uncommitted in the WP worktree, so the candidate is not integration-ready.

</topic>

<topic id="checks-run" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T03:02:59Z" ingestable="true" summary="Commands and deterministic reads used for rerun 6.">

CHECKS_RUN:
- Opened governed Integration Validator session: `INTEGRATION_VALIDATOR-20260516-025633`.
- Read candidate git state: dirty branch, candidate HEAD `19bcc83bdcc7981d8da33b0f9c4201987696ba2d`.
- Ran focused Rust tests for `kernel_task_contract_lifecycle_tests`, `kernel_workflow_transition_registry_tests`, `kernel_product_screenshot_capture_tests`, and `kernel_mechanical_contract_generation_tests`: PASS.
- Ran API unit tests `trigger_dcc_action_requires_catalog_preview_gate` and `projects_session_spawn_tree_runtime_records_for_dcc`: PASS.
- Ran focused frontend DCC test `npm test -- --run src/components/KernelDccProjectionView.test.tsx`: PASS.
- Read `cargo-test-kernel-slice.log`: broad kernel slice remains environment-blocked by missing `POSTGRES_TEST_URL` in `kernel_crdt_persistence_tests`.
- Read artifact logs under `D:\Projects\LLM projects\Handshake\Handshake_Artifacts\kernel002-remediation\`.
- Ran deterministic route/use probes with `rg` for DCC trigger, spawn-tree projection, screenshot capture, and mechanical command receipts.
- Ran `just --list` in `handshake_main` and `wt-gov-kernel` to resolve the `task-packet-stub-contracts` recipe boundary.
- Ran `Test-Path` for product receipt script refs.

TEST_VERDICT: PARTIAL.

The focused tests pass, but they do not prove the two remaining product capabilities. The dirty worktree and missing Postgres environment remain readiness/proof gaps.

</topic>

<topic id="mt-verdicts" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T03:02:59Z" ingestable="true" summary="MT rerun 6 verdicts after the six-item remediation.">

MT_BATCH_VERDICT: FAIL.

WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN.

Verdicts:

1. MT-001 `PASS`: activation/pre-use gate now evaluates missing source imports and source hash mismatches. Evidence: `task_contract_lifecycle.rs` defines `evaluate_work_packet_activation_pre_use_gate`; focused lifecycle tests pass.

2. MT-018 `PASS`: workflow mutation transition registry now covers `WorkPacket`, `MicroTask`, `TaskBoardProjection`, `RoleMailboxQueue`, and `DevCommandCenterAction`; focused registry tests pass.

3. MT-024 `PASS_WITH_RISK`: backend/API/UI now has a governed DCC trigger path through `/kernel/dcc_actions/trigger`, and the backend rejects unregistered catalog actions. Residual risk: the UI helper falls back to `surface.work_items[0]` when no work item allows an action, so the frontend can send a mismatched work/action pair and depend on backend rejection instead of blocking locally. Evidence: `app/src/components/KernelDccProjectionView.tsx:25-28`.

4. MT-043 `FAIL`: the DCC app route still does not render the session spawn tree from backend-provided runtime records. `App.tsx` only loads `getKernelDccProjection()` and passes that surface into `KernelDccProjectionView`; the Rust `DccMvpRuntimeSurfaceV1` returned by `/kernel/dcc_projection` has no `spawn_tree_projection` field. The new `projectKernelSessionSpawnTreeDcc()` client helper is unused outside `api.ts`, and the component test injects `spawn_tree_projection` as a fixture. Evidence: `app/src/App.tsx:64-75`, `app/src/App.tsx:229`, `app/src/lib/api.ts:839`, `src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs:201`, `src/backend/handshake_core/src/api/kernel.rs:137`, `app/src/components/KernelDccProjectionView.test.tsx:150`.

5. MT-045 `FAIL`: product screenshot capture is still a data contract/projection validator, not a governed capture execution surface. The implementation validates caller-supplied screenshot refs, metadata refs, and execution proof refs; it does not expose a CLI/API/adapter that captures full-app, panel, and module screenshots and writes artifacts. The catalog action is `kernel.product_screenshot_capture.project`, not an execute/capture action. Evidence: `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs:99`, `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs:159`, `src/backend/handshake_core/src/kernel/action_catalog.rs:1816`, `src/backend/handshake_core/tests/kernel_product_screenshot_capture_tests.rs:165-179`.

6. MT-049 `FAIL`: exact command lines are now present, but the durable receipt contract is still not integration-grade. `handshake_main` cannot run `just task-packet-stub-contracts --all` because its justfile lacks the recipe; only `wt-gov-kernel` exposes it. The remediation artifact records a kernel-side stub generation, but the product receipt does not state the required workdir boundary. Worse, two receipt `script_ref` values point to paths that do not exist: `.GOV/roles_shared/scripts/wp/build-order-sync.mjs` and `.GOV/roles_shared/scripts/gov-check.mjs`. The actual files are `.GOV/roles_shared/scripts/build-order-sync.mjs` and `.GOV/roles_shared/checks/gov-check.mjs`. `gov-check` logs also record live governance blockers, so closeout cannot treat this as a clean pass. Evidence: `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs:702-713`, `just --list` in both worktrees, artifact logs `task-packet-stub-contracts-all.log`, `task-packet-stub-contracts-all.gov-kernel.log`, `gov-check.gov-kernel.cmd.log`, and `gov-check-verbose.gov-kernel.cmd.log`.

</topic>

<topic id="resolved-items-and-proof-gaps" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T03:02:59Z" ingestable="true" summary="Resolved prior blockers and remaining non-primary proof gaps.">

Resolved from rerun 5:
- MT-001 activation/pre-use fold-source blocker is addressed.
- MT-018 missing workflow transition rules are addressed.
- MT-024 has a backend/API/UI governed trigger path and backend preview gate; keep the frontend fallback as hardening work.

Remaining proof/readiness gaps:
- Candidate product changes are uncommitted.
- Broad Postgres-backed Kernel002 proof remains blocked by absent `POSTGRES_TEST_URL`.
- `D:\Projects\LLM projects\Handshake\Handshake Worktrees\Handshake_Artifacts\` still appears to contain accidental generated artifacts/caches. Do not delete without Operator approval; run artifact hygiene/cleanup only in the governed cleanup lane when product blockers are cleared.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T03:02:59Z" ingestable="true" summary="Single Kernel Builder remediation plan for rerun 6.">

Kernel Builder remediation plan:

1. Fix MT-043 by wiring session spawn tree data into the actual DCC route used by the app. Either add `spawn_tree_projection` to the Rust `DccMvpRuntimeSurfaceV1` returned by `/api/kernel/dcc_projection`, or have `App.tsx` fetch `/api/kernel/session_spawn_tree_dcc_projection` from authoritative runtime records and merge it into the displayed surface. Add an app-level test that opens Kernel DCC and proves hierarchy, child counts, depth, cascade cancel, spawn mode, announce-back badges, and runtime record refs come from the backend route, not only a component fixture.

2. Fix MT-045 by implementing a real governed screenshot capture execution path. Provide a product-owned CLI or API endpoint that invokes the capture adapter, captures full-app, panel, and module screenshots, writes screenshot and metadata artifacts, and emits durable receipts. Tests must fail if the implementation only validates pre-supplied refs.

3. Fix MT-049 durable receipt correctness. State the required workdir for `just task-packet-stub-contracts --all`, or expose the recipe from the integration worktree if that is the expected validator command. Correct invalid `script_ref` paths for `build-order-sync` and `gov-check`, and add a test that verifies receipt script refs resolve to real files. Keep the exact command-line receipts and include exit code/workdir in the artifact logs.

4. Harden MT-024 while touching DCC again: remove the `surface.work_items[0]` fallback for action triggering, require an allowed selected work item, and add a negative UI test for a catalog action with no allowed work item.

5. Commit the remediation in `../wtc-preuse-hardening-v1` so the worktree is clean, then rerun the focused Rust tests, focused API tests, focused DCC UI tests, product `git diff --check`, and the available kernel slice. If `POSTGRES_TEST_URL` is still absent, keep the environment blocker explicit with exact blocked tests.

6. Do not proceed to whole-WP Master Spec validation, merge, governance sync, or push until MT-043, MT-045, MT-049, and the dirty worktree are resolved.

</topic>
