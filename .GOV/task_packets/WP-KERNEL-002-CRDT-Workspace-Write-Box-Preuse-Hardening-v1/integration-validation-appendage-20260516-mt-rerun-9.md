---
file_id: integration-validation-appendage-20260516-mt-rerun-9
file_kind: validation_appendage
updated_at: 2026-05-16T09:35:00Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
candidate_worktree: D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-preuse-hardening-v1
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
candidate_base_head_sha: e09fa152
candidate_state: DIRTY_WORKTREE_REMEDIATION
baseline_main_sha: e11ba597
mt_batch_verdict: FAIL
whole_wp_master_spec_validation: NOT_RUN
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T09:35:00Z" ingestable="true" summary="Integration Validator rerun 9 after Kernel Builder dirty-worktree remediation.">

Handshake (Product): reviewed the Kernel Builder remediation present as dirty working-tree changes on top of `e09fa152` in `wtc-preuse-hardening-v1`. The MT batch still fails.

Repo Governance: Operator waived normal WP Validator workflow. No merge, sync, staging, commit, push, or whole-WP Master Spec validation was performed.

MECHANICAL_INTERVENTION_CLASSIFICATION:
- Runtime route drift: confirmed for `MT-043`; backend emits records from runtime sessions, but announce-back and cascade-cancel semantics are still partly synthesized/inferred rather than backed by explicit runtime records.
- Scope/implementation drift: confirmed for `MT-045`; the implementation persists caller-supplied PNG bytes rather than actually capturing full-app/panel/module screenshots.
- Proof/receipt drift: confirmed for `MT-049`; command receipt contracts are declared, but current-candidate receipt artifacts and exact exit-code evidence were not found.
- Documentation/protocol drift: secondary; handoff prose claims remediation success, but structured proof is incomplete.
- Scope/worktree drift: present; the reviewed remediation is dirty working-tree state, not a committed candidate.

</topic>

<topic id="checks-run" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T09:35:00Z" ingestable="true" summary="Checks and inspected evidence for rerun 9.">

CHECKS_RUN:
- `git -C ..\wtc-preuse-hardening-v1 status --short --branch`: dirty remediation over `e09fa152`.
- `git -C ..\wtc-preuse-hardening-v1 merge-base --is-ancestor main HEAD`: PASS.
- `git -C ..\wtc-preuse-hardening-v1 rev-list --left-right --count main...HEAD`: `0 5`.
- `git -C ..\wtc-preuse-hardening-v1 diff --check main...HEAD`: PASS.
- Focused source searches over `app/src/lib/api.ts`, `app/src/lib/api.test.ts`, `src/backend/handshake_core/src/api/kernel.rs`, `src/backend/handshake_core/src/kernel/action_catalog.rs`, `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs`, `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs`, and relevant tests.
- Read-only subagent review for `MT-043`, `MT-045`, and `MT-049`.

TOOL_FAILURES:
- Broad recursive artifact searches under `D:\Projects\LLM projects\Handshake\Handshake_Artifacts` timed out. This did not change the verdict because targeted source and receipt-path inspection already showed the relevant proof gaps.
- `just memory-capture` is unavailable in the active root `justfile`; the prior attempt to use it failed with `Justfile does not contain recipe memory-capture`.

TEST_VERDICT: NOT_FULLY_PROVEN.

The Kernel Builder-reported tests may pass, but acceptance remains blocked by independent source/proof review.

</topic>

<topic id="mt-findings" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T09:35:00Z" ingestable="true" summary="MT-level findings after remediation.">

MT_BATCH_VERDICT: FAIL.

WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN.

MT-043 VERDICT: FAIL.

Finding: DCC no longer fabricates flat/index-derived records in the frontend, and backend now derives session spawn records from runtime `ModelSession` rows with Flight Recorder refs. However, announce-back badges are still synthesized and cascade-cancel support is still inferred from topology rather than read from explicit runtime capability/event records.

Evidence:
- `app/src/lib/api.ts` now passes backend-provided `session_spawn_runtime_records` through `buildSessionSpawnTreeDccRequest`.
- `src/backend/handshake_core/src/api/kernel.rs#session_spawn_runtime_records_from_sessions` builds records from runtime sessions.
- `src/backend/handshake_core/src/api/kernel.rs#announce_back_badges` emits a hardcoded `announce-back pending` badge for child sessions based on `parent_session_id`.
- `src/backend/handshake_core/src/api/kernel.rs#session_spawn_runtime_records_from_sessions` sets `cascade_cancel_supported` from `active_child_count > 0`.
- `src/backend/handshake_core/src/kernel/session_spawn_tree_dcc.rs#project_session_spawn_tree_dcc` can project hierarchy, child counts, depth, spawn mode, cascade-cancel availability, badges, and runtime refs when the input records are valid.

Missing acceptance:
- Announce-back badges must be backed by real runtime/Flight Recorder announce-back records or explicit mailbox/runtime badge records, not hardcoded from `parent_session_id`.
- Cascade-cancel support must be backed by an explicit runtime capability/state/event if this MT's "from runtime records" requirement is interpreted strictly.

MT-045 VERDICT: FAIL.

Finding: artifact writing exists, but actual screenshot capture is not implemented. The API accepts caller-supplied `png_base64` and persists those bytes; it does not capture the app/window/panel/module itself. The claimed CLI surface is a validated string, not an executable command path.

Evidence:
- `src/backend/handshake_core/src/api/kernel.rs` defines screenshot execute input with `png_base64`, `adapter_exit_status`, `captured_at_utc`, and `command_or_api_ref`.
- `src/backend/handshake_core/src/api/kernel.rs#execute_product_screenshot_capture_api` decodes caller-supplied PNG bytes and passes them to the writer.
- `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs#execute_product_screenshot_capture` writes `ProductScreenshotAdapterCaptureV1.png_bytes`.
- `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs` writes PNG, metadata JSON, receipt JSON, `screenshot_sha256`, `metadata_sha256`, adapter exit status, scope, and refs.
- `src/backend/handshake_core/src/api/kernel.rs` registers `/kernel/product_screenshot_capture/execute`.
- `src/backend/handshake_core/src/kernel/action_catalog.rs` registers `kernel.product_screenshot_capture.execute`.
- No implementation of a real `handshake screenshot capture` CLI or app/panel/module capture adapter was found in `src`, `app`, or `justfile`.

Missing acceptance:
- Actual full-app screenshot capture.
- Actual panel screenshot capture.
- Actual module screenshot capture.
- Executable governed CLI adapter or equivalent capture adapter path.
- App/panel/module target refs must select a capture source, not only validate ref prefixes.

MT-049 VERDICT: FAIL.

Finding: exact command receipt contracts are declared, and command exposure appears improved, but current-candidate command receipt proof remains unproven. No writer/exposed artifacts were found that bind the three command results to actual exit codes, stdout/stderr refs, candidate SHA, and blocker JSON.

Evidence:
- `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs` defines durable receipt rows for `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check`.
- `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs` declares `hsk.kernel.current_candidate_command_receipt@1`, `artifact://command-receipts/{slug}.json`, stdout/stderr refs, and `.blockers.json`.
- `src/backend/handshake_core/tests/kernel_mechanical_contract_generation_tests.rs` tests receipt metadata presence.
- `justfile` exposes `gov-check` and `build-order-sync`; topology includes `just:task-packet-stub-contracts`.
- WP communication receipts contain prose claims about checks, but no per-command current-candidate receipt artifact path with actual exit code and stdout/stderr/blocker refs was verified.

Missing acceptance:
- Actual per-command receipt JSON for `just task-packet-stub-contracts --all`.
- Actual per-command receipt JSON for `just build-order-sync`.
- Actual per-command receipt JSON for `just gov-check`.
- Actual exit code, stdout/stderr artifact refs, candidate SHA, and blocker summary bound to the dirty remediation/candidate state.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T09:35:00Z" ingestable="true" summary="Single remediation plan for Kernel Builder after rerun 9.">

Kernel Builder remediation plan:

1. `MT-043`: replace synthetic announce-back badges with data from explicit runtime/Flight Recorder announce-back records or persisted mailbox/runtime badge records. Replace `cascade_cancel_supported: active_child_count > 0` with an explicit runtime capability/state/event source, or document and encode why topology-derived cascade-cancel is the intended runtime record. Add tests that fail when badges are generated only from `parent_session_id` or cascade-cancel is generated only from child count.

2. `MT-045`: implement an actual capture adapter. The API should not rely solely on caller-supplied `png_base64` as proof of capture. Add a real governed CLI/API path that obtains screenshot bytes from the app/window/panel/module source, then writes PNG, metadata, and receipt artifacts. The accepted `cli://handshake screenshot capture ...` surface must correspond to an executable command or be replaced with the real executable surface. Add tests or harness proof that capture occurs for full-app, panel, and module targets.

3. `MT-049`: add or expose the current-candidate command receipt writer. For each exact acceptance command, produce a machine-readable receipt artifact that records command line, workdir, candidate SHA, actual exit code, expected exit code, stdout/stderr artifact refs, blocker artifact refs when nonzero, and projection/artifact refs. Do not rely on handoff prose as command proof.

4. Commit the remediation before the next final validation handoff. Current review was against dirty working-tree changes over `e09fa152`; final-lane merge/sync cannot proceed from uncommitted product remediation.

5. Preserve current-main readiness: `main` must remain an ancestor, `rev-list main...HEAD` should remain `0 N`, `merge-tree main HEAD` must remain clean, and `git diff --check main...HEAD` must pass.

Only after `MT-043`, `MT-045`, and `MT-049` pass should Integration Validator proceed to whole-WP coding-vs-current-Master-Spec validation.

</topic>
