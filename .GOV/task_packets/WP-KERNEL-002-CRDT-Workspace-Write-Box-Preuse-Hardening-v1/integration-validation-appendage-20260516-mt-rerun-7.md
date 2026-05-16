---
file_id: integration-validation-appendage-20260516-mt-rerun-7
file_kind: validation_appendage
updated_at: 2026-05-16T06:25:09Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
session_id: INTEGRATION_VALIDATOR-20260516-061746
candidate_worktree: ../wtc-preuse-hardening-v1
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
candidate_head_sha: e09fa1523b281db5a42e8db0c42ed3edeb0afbdb
baseline_main_sha: e11ba59793490028262089f782523eb51cf1f1f7
mt_batch_verdict: FAIL
whole_wp_master_spec_validation: NOT_RUN
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T06:25:09Z" ingestable="true" summary="Operator-waived MT rerun after commit e09fa152.">

Handshake Product validation reviewed committed remediation `e09fa1523b281db5a42e8db0c42ed3edeb0afbdb` as evidence, not as truth.

Candidate state:
- `../wtc-preuse-hardening-v1` is clean.
- Candidate HEAD is `e09fa1523b281db5a42e8db0c42ed3edeb0afbdb`.
- Merge base with `main` is `e11ba59793490028262089f782523eb51cf1f1f7`.
- Remediation delta from rerun-6 baseline `19bcc83bdcc7981d8da33b0f9c4201987696ba2d` touches 15 files and 1,867 inserted lines.
- `git diff --check main...HEAD`: PASS.

Whole-WP code-vs-current-Master-Spec validation was not run because the Operator-gated MT prerequisite still fails.

</topic>

<topic id="adversarial-review-artifacts" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T06:25:09Z" ingestable="true" summary="Adversarial review surfaces and checks.">

DIFF_ATTACK_SURFACES:
- MT-043 producer/consumer seam: app DCC surface producer vs session-spawn runtime-record consumer.
- MT-045 implementation seam: typed screenshot proof model vs actual capture execution adapter/API/CLI.
- MT-049 command receipt seam: product receipt command/workdir/script refs vs real Just recipe availability in the WP/integration worktree.
- MT-024 negative UI seam: catalog action row vs allowed selected work item.

INDEPENDENT_CHECKS_RUN:
- Reread authority surfaces and adversarial-code-review workflow.
- `just integration-validator-context-brief WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`.
- `git status --short --branch`; `git rev-parse HEAD`; `git diff --stat main...HEAD`; `git diff --check main...HEAD`.
- `rg` route/source probes across `app/src/lib/api.ts`, `app/src/components/KernelDccProjectionView.tsx`, `src/backend/handshake_core/src/api/kernel.rs`, `product_screenshot_capture.rs`, `action_catalog.rs`, and mechanical receipt tests.
- `just --list` in `../wtc-preuse-hardening-v1` to verify the claimed exact command path.
- Focused tests:
  - `cargo fmt --manifest-path src/backend/handshake_core/Cargo.toml -- --check`: PASS.
  - `pnpm test -- --run src/lib/api.test.ts src/components/KernelDccProjectionView.test.tsx`: PASS, 6 tests.
  - `pnpm test -- --run src/App.test.tsx -t "loads the backend Kernel DCC projection when opened"`: PASS, 1 selected test.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake_Artifacts/handshake-cargo-target" --test kernel_product_screenshot_capture_tests`: PASS, 6 tests.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake_Artifacts/handshake-cargo-target" --test kernel_mechanical_contract_generation_tests`: PASS, 7 tests.
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake_Artifacts/handshake-cargo-target" --lib api::kernel::tests`: PASS, 2 tests.

COUNTERFACTUAL_CHECKS:
- If `app/src/lib/api.ts::buildSessionSpawnTreeDccRequest` stopped fabricating `runtime_records` from `surface.sessions`, the current app path would have no runtime-backed spawn tree source.
- If `product_screenshot_capture.rs` validation structs were removed, no product screenshot capture endpoint or adapter would remain; the implementation does not contain a separate executor.
- If the root justfile is used from the WP worktree, `just task-packet-stub-contracts --all` is not available even though the durable receipt says `workdir_ref = repo-root://`.

BOUNDARY_PROBES:
- API/client boundary: `getKernelDccProjection()` fetches `/api/kernel/dcc_projection`, then posts client-generated records to `/api/kernel/session_spawn_tree_dcc_projection`; it does not read a backend runtime-record source.
- Product contract/runtime boundary: screenshot tests verify modeled `ProductScreenshotDurableReceiptV1` and `ProductScreenshotExecutionProofV1`, not a command/API that writes screenshot files.
- Command/workdir boundary: `just --list` in the WP worktree exposes `build-order-sync` and `gov-check`, but not `task-packet-stub-contracts`.

NEGATIVE_PATH_CHECKS:
- Frontend DCC negative path now passes: unallowed catalog action does not fall back to the first work item.
- Screenshot validation negative path passes for malformed modeled refs, but no negative execution-path test can exist because there is no executor.
- Mechanical receipt script-ref existence test passes, but it does not verify Just recipe availability from the declared repo-root workdir.

</topic>

<topic id="mt-verdicts" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T06:25:09Z" ingestable="true" summary="MT rerun 7 verdicts.">

MT_BATCH_VERDICT: FAIL.

WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN.

Verdicts:

1. MT-001 `PASS`: activation/pre-use fold-source gate remains addressed.

2. MT-018 `PASS`: workflow transition registry coverage remains addressed.

3. MT-024 `PASS`: the first-work fallback was removed and the negative UI test proves unallowed catalog actions do not trigger the API. Evidence: `app/src/components/KernelDccProjectionView.tsx:25`, `app/src/components/KernelDccProjectionView.test.tsx:265-287`.

4. MT-043 `FAIL`: the app still does not prove spawn hierarchy, child counts, depth, cascade cancel, spawn mode, and announce-back badges from real runtime records. `getKernelDccProjection()` now posts a spawn-tree request, but `buildSessionSpawnTreeDccRequest()` fabricates records from `surface.sessions`, sets `parent_session_id: null` for every session, invents `spawn_mode` from array index, and invents announce-back badges for the first session. This cannot prove a hierarchy or real parent-child runtime state. Evidence: `app/src/lib/api.ts:837`, `app/src/lib/api.ts:869-885`.

5. MT-045 `FAIL`: screenshot capture still models execution/write receipts rather than executing capture. The only product action is `kernel.product_screenshot_capture.project` with `AuthorityEffect::ProjectionOnly` and `ReadOnlyProjectionBox`; tests pass by validating caller-supplied paths, receipt refs, and `api://kernel.product_screenshot_capture.execute` strings. There is no route, CLI, or adapter that captures a screenshot and writes artifact/metadata/receipt files. Evidence: `src/backend/handshake_core/src/kernel/action_catalog.rs:1816-1833`, `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs:132`, `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs:194`, `src/backend/handshake_core/tests/kernel_product_screenshot_capture_tests.rs:247`.

6. MT-049 `FAIL`: script refs are corrected and tested for path existence, but the exact durable command still cannot be run from the declared repo-root workdir in the WP worktree. `just --list` in `../wtc-preuse-hardening-v1` shows `build-order-sync` and `gov-check`, but no `task-packet-stub-contracts`; the mechanical test checks only `.GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs` existence, not the Just recipe surface. Evidence: root `justfile` has `build-order-sync` and `gov-check` only, `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs:704`, `src/backend/handshake_core/tests/kernel_mechanical_contract_generation_tests.rs:82-106`.

Environment proof gap:
- `POSTGRES_TEST_URL` remains absent, so broad Postgres-backed Kernel002 proof remains environment-blocked.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T06:25:09Z" ingestable="true" summary="Single remediation plan after rerun 7.">

Kernel Builder remediation plan:

1. Fix MT-043 by using actual runtime spawn records, not client-derived DCC session rows. The record source must include parent session id, spawn mode, runtime state, cascade-cancel support, announce-back badges, runtime record refs, and flight recorder refs. The app test must prove a non-root child with `parent_session_id` from backend/runtime data renders with depth and child counts; all-root fabricated records are not sufficient.

2. Fix MT-045 by adding an actual governed screenshot capture executor. It can be a product API or CLI, but it must call a capture adapter, write full-app/panel/module screenshot files, write metadata, write durable receipt files with hashes/exit status, and be tested so pre-supplied refs alone cannot pass.

3. Fix MT-049 by aligning the command receipt with a real executable surface. Either expose `task-packet-stub-contracts` from the canonical repo-root justfile used by WP/integration validators, or change the receipt to a workdir that actually has the recipe and make that workdir explicit. Add a test that invokes or mechanically resolves the Just recipe, not only the script file path.

4. Keep MT-024 as-is unless changing DCC again regresses the no-fallback behavior; preserve the negative UI test.

5. After remediation, rerun the focused checks plus a direct command-surface proof for `just task-packet-stub-contracts --all` from the declared workdir. Do not start whole-WP Master Spec validation until MT-043, MT-045, and MT-049 pass.

</topic>
