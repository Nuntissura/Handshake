---
file_id: integration-validation-appendage-20260516-mt-rerun-8
file_kind: validation_appendage
updated_at: 2026-05-16T08:05:00Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
candidate_worktree: D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-preuse-hardening-v1
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
candidate_head_sha: e09fa152
baseline_main_sha: e11ba597
mt_batch_verdict: FAIL
whole_wp_master_spec_validation: NOT_RUN
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T08:05:00Z" ingestable="true" summary="Integration Validator MT rerun before whole-WP Master Spec validation.">

Handshake (Product): reviewed the Kernel Builder candidate at `e09fa152` for the MTs that remained failed after the prior remediation loop: `MT-043`, `MT-045`, and `MT-049`.

Repo Governance: Operator waived the normal WP Validator / paperwork workflow for this pass. No merge, sync, staging, commit, or push was performed. Whole-WP coding-vs-current-Master-Spec validation was not run because the MT batch failed.

MECHANICAL_INTERVENTION_CLASSIFICATION:
- Runtime route drift: checked. The candidate branch is now current-main integration-ready: `main` is an ancestor of `HEAD`, branch is `0 5` ahead of main, and `git merge-tree main HEAD` returned cleanly.
- Notification/cursor drift: not used as a blocker because this pass was Operator-directed in chat.
- Session/ACP drift: accepted under Operator waiver; no WP Validator gate exists for this packet.
- Documentation/protocol drift: present but not the product blocker; `just memory-capture` is not exposed by the active root `justfile`.
- Scope/worktree drift: no new worktrees were created; review used existing `wtc-preuse-hardening-v1`.

</topic>

<topic id="checks-run" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T08:05:00Z" ingestable="true" summary="Commands and evidence used for MT rerun 8.">

CHECKS_RUN:
- `git -C ..\wtc-preuse-hardening-v1 status --short --branch`: candidate branch clean at `e09fa152`.
- `git -C ..\wtc-preuse-hardening-v1 merge-base --is-ancestor main HEAD`: PASS.
- `git -C ..\wtc-preuse-hardening-v1 rev-list --left-right --count main...HEAD`: `0 5`.
- `git -C ..\wtc-preuse-hardening-v1 merge-tree main HEAD`: clean merge tree.
- `git -C ..\wtc-preuse-hardening-v1 diff --check main...HEAD`: PASS.
- `cargo test --manifest-path .\src\backend\handshake_core\Cargo.toml kernel_dcc_session_spawn_tree --target-dir D:\Projects\LLM projects\Handshake\Handshake_Artifacts\handshake-cargo-target`: 4 matching tests passed.
- `cargo test --manifest-path .\src\backend\handshake_core\Cargo.toml kernel_product_screenshot_capture --target-dir D:\Projects\LLM projects\Handshake\Handshake_Artifacts\handshake-cargo-target`: 6 matching tests passed.
- `cargo test --manifest-path .\src\backend\handshake_core\Cargo.toml generated_status_projection --target-dir D:\Projects\LLM projects\Handshake\Handshake_Artifacts\handshake-cargo-target`: command completed successfully, but filters produced mostly zero-test bins and do not prove command-level MT-049 acceptance.

TEST_VERDICT: PARTIAL.

The focused backend projection tests pass, but MT acceptance is not satisfied because the failing gaps are integration/behavioral, not only projection-schema unit behavior.

</topic>

<topic id="mt-findings" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T08:05:00Z" ingestable="true" summary="MT-level findings for rerun 8.">

MT_BATCH_VERDICT: FAIL.

WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN.

MT-043 VERDICT: FAIL.

Finding: the backend projection can compute hierarchy/depth/child-counts when real runtime records are supplied, but the app/DCC composition path still synthesizes spawn runtime records from flat DCC session rows.

Evidence:
- `src/backend/handshake_core/src/kernel/session_spawn_tree_dcc.rs` defines `project_session_spawn_tree_dcc` and validates visible fields, runtime record refs, parent links, cascade-cancel support, spawn mode, and announce-back badges.
- `src/backend/handshake_core/tests/kernel_dcc_session_spawn_tree_tests.rs` proves backend hierarchy, depth, child count, spawn mode, announce-back badge, and negative-path validation for sample records.
- `app/src/lib/api.ts#getKernelDccProjection` / `buildSessionSpawnTreeDccRequest` still synthesizes frontend records: `parent_session_id` is forced to null, `spawn_mode` and `cascade_cancel_supported` are inferred from array index, and announce-back is emitted as synthetic `announce-back-ready`.
- `app/src/components/KernelDccProjectionView.tsx` renders projected fields when present, but the app path does not feed real runtime parentage, cascade-cancel, spawn-mode, or announce-back badge records into that projection.

Missing acceptance:
- Real spawn hierarchy from runtime records.
- Real child relationships rather than all-root synthetic records.
- Real cascade-cancel state.
- Real spawn mode.
- Real announce-back badges.

MT-045 VERDICT: FAIL.

Finding: candidate implements a typed screenshot capture projection/validation contract, but not an actual governed screenshot capture capability.

Evidence:
- `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs` defines `ProductScreenshotCaptureV1`, `validate_product_screenshot_capture`, and `project_product_screenshot_capture`.
- `src/backend/handshake_core/tests/kernel_product_screenshot_capture_tests.rs` covers schema/projection, metadata refs, durable receipt refs, and negative validation.
- `src/backend/handshake_core/src/kernel/action_catalog.rs` registers `kernel.product_screenshot_capture.project` as `AuthorityEffect::ProjectionOnly`.
- `src/backend/handshake_core/src/api/kernel.rs` has no executable screenshot capture API route for full-app, panel, or module capture.
- `app/src/lib/api.ts`, `app/src/App.tsx`, and `app/src/components/KernelDccProjectionView.tsx` do not expose a screenshot capture client or UI execution path.
- Tests do not capture a PNG, write metadata files, verify hashes, call a CLI/API endpoint, or perform visual DCC screenshot/debug proof.

Missing acceptance:
- Governed coder session can trigger full-app capture through a real CLI/API path.
- Governed validator session can capture/inspect panel screenshots.
- Module-level capture actually works.
- PNG and metadata artifact files are written and referenced.
- Tauri/React frontend capture path exists.
- Visual DCC screenshot/debug proof exists.

MT-049 VERDICT: FAIL.

Finding: projection/status targets are improved, but the exact acceptance command surface and current durable pass receipts are not established.

Evidence:
- `src/backend/handshake_core/src/kernel/generated_documentation_status_projection.rs` now points Task Board and traceability projection targets to `.GOV/roles_shared/records/TASK_BOARD.md#WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1` and `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md#WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`.
- `src/backend/handshake_core/tests/kernel_generated_documentation_status_projection_tests.rs` asserts the real Task Board and traceability target refs.
- `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs` defines receipt expectations for `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check`.
- Candidate `just --list` does not expose the exact `task-packet-stub-contracts` recipe.
- No e09fa152-specific durable receipt/exit-code evidence proves all three exact commands passed from the candidate/integration command surface.

Missing acceptance:
- Exact command `just task-packet-stub-contracts --all` is exposed or an explicit blocker is recorded by the candidate surface.
- Exact command `just build-order-sync` has current e09fa152 pass/blocker evidence.
- Exact command `just gov-check` has current e09fa152 pass/blocker evidence.
- Durable receipts bind the current candidate sha, workdir, command, exit code, and artifact refs.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T08:05:00Z" ingestable="true" summary="Single Kernel Builder remediation plan for rerun 8.">

Kernel Builder remediation plan:

1. Fix MT-043 by wiring the DCC/app composition path to real runtime session-spawn records, not synthetic flat session rows. Preserve parent-child links, depth source, cascade-cancel support, spawn mode, announce-back badges, runtime record refs, and Flight Recorder refs from the runtime authority. Add an app/API test that fails if `parent_session_id` is forced to null or if spawn mode / cascade-cancel / announce-back values are index-derived.

2. Fix MT-045 by implementing an executable governed screenshot capture path, not only a projection contract. Provide a CLI and/or API route that can capture full app, panel, and module screenshots, write PNG artifacts, write metadata, write durable receipts with screenshot hash, metadata hash, adapter exit status, command/API ref, scope, and artifact refs under `../Handshake_Artifacts/handshake-product/screenshots/`. Add tests or deterministic harness proof that exercises the real execution path for all three scopes.

3. Fix MT-049 by exposing or repairing the exact command surface required by the MT: `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check`. If a command is intentionally unavailable under the new command model, write a concrete blocker/equivalent-command mapping into the candidate evidence contract and make the MT acceptance explicit. Add e09fa152-successor receipts that bind command, workdir, candidate sha, exit code, and artifact/projection refs.

4. Keep the current-main integration status clean. Before returning, re-run `git merge-base --is-ancestor main HEAD`, `git rev-list --left-right --count main...HEAD`, `git merge-tree main HEAD`, and `git diff --check main...HEAD`.

5. Re-run the focused MT validation set after remediation. Passing backend projection tests are necessary but not sufficient; include app/API integration proof for `MT-043`, real capture execution proof for `MT-045`, and exact command/receipt proof for `MT-049`.

Only after `MT-043`, `MT-045`, and `MT-049` pass should Integration Validator proceed to whole-WP coding-vs-current-Master-Spec validation.

</topic>
