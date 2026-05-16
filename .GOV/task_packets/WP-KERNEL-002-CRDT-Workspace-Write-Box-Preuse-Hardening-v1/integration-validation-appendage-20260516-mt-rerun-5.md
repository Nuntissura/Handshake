---
file_id: integration-validation-appendage-20260516-mt-rerun-5
file_kind: validation_appendage
updated_at: 2026-05-16T00:28:04Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
session_id: INTEGRATION_VALIDATOR-20260516-001722
candidate_worktree: ../wtc-preuse-hardening-v1
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
candidate_head_sha: 19bcc83bdcc7981d8da33b0f9c4201987696ba2d
baseline_main_sha: e11ba59793490028262089f782523eb51cf1f1f7
mt_batch_verdict: FAIL
whole_wp_master_spec_validation: NOT_RUN
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T00:28:04Z" ingestable="true" summary="Operator-waived MT batch revalidation before Master Spec validation.">

Handshake Product validation reviewed the Kernel002 candidate in `../wtc-preuse-hardening-v1` after the Kernel Builder remediation round. This run intentionally used the Operator-waived MT-first path instead of the normal WP Validator gate.

Candidate readiness improved versus rerun 4:
- `git -C ../wtc-preuse-hardening-v1 status --short --branch`: clean worktree on `feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`.
- `git -C ../wtc-preuse-hardening-v1 merge-base --is-ancestor main HEAD`: PASS.
- `git -C ../wtc-preuse-hardening-v1 rev-list --left-right --count main...HEAD`: `0 4`.
- `git -C ../wtc-preuse-hardening-v1 merge-tree main HEAD`: clean merge-tree, tree `fe538663051f23493993f91b9aee56e7f9608331`.
- `git -C ../wtc-preuse-hardening-v1 diff --check main...HEAD`: PASS.

MT batch still fails. Whole-WP code-vs-current-Master-Spec validation was not run because the Operator-gated prerequisite was not satisfied.

</topic>

<topic id="checks-run" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T00:28:04Z" ingestable="true" summary="Commands and review probes used for rerun 5.">

CHECKS_RUN:
- Read authority stack before work: `AGENTS.md`, `../wt-gov-kernel/.GOV/codex/Handshake_Codex_v1.4.md`, `../wt-gov-kernel/.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`.
- Opened governed session: `just repomem open ... --role INTEGRATION_VALIDATOR --wp WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`.
- Read packet/refinement machine contracts and MT JSON contracts.
- Used four read-only subagent range reviews: MT-001..015, MT-016..030, MT-031..045, MT-046..061. Subagents were instructed not to edit, create worktrees, switch branches, or run destructive commands.
- Ran merge/readiness probes listed in `validation-context`.
- Ran `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --target-dir ..\Handshake_Artifacts\handshake-cargo-target kernel_`.
- Ran the same Rust test slice with explicit skips for two Postgres-gated CRDT persistence tests.
- Checked `POSTGRES_TEST_URL`; it is absent in this environment.
- Locally spot-checked source paths for MT-001, MT-018, and MT-024 after subagent findings.

TEST_VERDICT: PARTIAL.

Rust compilation succeeds, and many Kernel002 tests pass. Full kernel test proof is blocked by absent `POSTGRES_TEST_URL`; several Postgres-backed tests panic with `ENVIRONMENT_BLOCKED` before the full slice can complete. This is recorded as an environment proof gap, not as the primary product remediation because the MT batch already has product/proof blockers.

</topic>

<topic id="mt-blocking-findings" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T00:28:04Z" ingestable="true" summary="MT batch blockers found in rerun 5.">

MT_BATCH_VERDICT: FAIL.

WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN.

Blocking MT findings:

1. MT-001 `NOT_PROVEN`: fold preservation manifest exists and tests cover source count/hash failures, but `verify_observed_sources` is only test-consumed. Acceptance says activation cannot proceed if a source file is missing or hash mismatch is unexplained. Evidence path: `src/backend/handshake_core/src/kernel/fold_manifest.rs`.

2. MT-018 `FAIL`: `WorkflowMutationKind` declares `WorkPacket`, `MicroTask`, `TaskBoardProjection`, `RoleMailboxQueue`, and `DevCommandCenterAction`, but registered transition rules cover only `MicroTask` and `RoleMailboxQueue` paths. Acceptance says every workflow mutation has a rule. Evidence path: `src/backend/handshake_core/src/kernel/workflow_transition_registry.rs`.

3. MT-024 `FAIL`: DCC projection API/UI is preview/read-only, but no governed action trigger path exists from API/UI through the catalog. Evidence paths: `src/backend/handshake_core/src/api/kernel.rs`, `app/src/components/KernelDccProjectionView.tsx`, `app/src/components/KernelDccProjectionView.test.tsx`.

4. MT-043 `NOT_PROVEN`: backend DCC session spawn tree projection and tests exist, but there is no live API/UI evidence that the DCC shows hierarchy, child counts, depth, cascade cancel, spawn mode, or announce-back badges from runtime records. Evidence path: `src/backend/handshake_core/src/kernel/session_spawn_tree_dcc.rs`.

5. MT-045 `NOT_PROVEN`: typed screenshot capture requests/artifact refs exist, but no actual governed session capture adapter/CLI/API execution proof was found for full-app, panel, and module screenshots plus metadata/artifact refs. Evidence path: `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs`.

6. MT-049 `NOT_PROVEN`: no durable evidence was found that exact acceptance commands `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check` passed or hit a concrete blocker. The mechanical contract references `--check`, not `--all`. Evidence path: `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs`.

Environment proof gap:
- `POSTGRES_TEST_URL` is absent, blocking full proof for Postgres-backed Kernel002 tests including CRDT persistence, CRDT snapshot, CRDT promotion bridge, kernel end-to-end, and Postgres EventLedger tests.

</topic>

<topic id="passed-or-improved-items" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T00:28:04Z" ingestable="true" summary="Items that passed static MT review or improved from prior reruns.">

Improvements confirmed from prior failures:
- Candidate is now clean, ahead of current `main`, and merge-tree clean.
- MT-050 denied acceptance EventLedger evidence appears addressed.
- MT-051 and MT-053 digest-backed provenance appear addressed; tests now reject fake digests.
- MT-055 generated status projection paths appear addressed in the candidate review.

Static MT review passed for these MTs: MT-002, MT-003, MT-004, MT-005, MT-006, MT-007, MT-008, MT-009, MT-010, MT-011, MT-012, MT-013, MT-014, MT-015, MT-016, MT-017, MT-019, MT-020, MT-021, MT-022, MT-023, MT-025, MT-026, MT-027, MT-028, MT-029, MT-030, MT-031, MT-032, MT-033, MT-034, MT-035, MT-036, MT-037, MT-038, MT-039, MT-040, MT-041, MT-042, MT-044, MT-046, MT-047, MT-048, MT-050, MT-051, MT-052, MT-053, MT-054, MT-055, MT-056, MT-057, MT-058, MT-059, MT-060, MT-061.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T00:28:04Z" ingestable="true" summary="Single Kernel Builder remediation plan for rerun 5.">

Kernel Builder remediation plan:

1. Wire MT-001 fold source verification into the activation/pre-use gate. Missing source files or pre-fold hash mismatches must block activation, not only fail unit tests.
2. Fix MT-018 workflow transition coverage. Either add transition rules and tests for `WorkPacket`, `TaskBoardProjection`, and `DevCommandCenterAction`, or narrow the public mutation enum/contract so every declared mutation kind has an implemented rule.
3. Fix MT-024 DCC governed action triggering. Add an API/UI path that triggers governed catalog actions through preview/gate enforcement, or explicitly reduce the MT contract to preview-only before validation.
4. Prove or implement MT-043 live DCC session spawn tree rendering. The proof must show hierarchy, child counts, depth, cascade cancel, spawn mode, and announce-back badges rendered from runtime records, not only backend projection structs.
5. Prove or implement MT-045 governed screenshot capture. Add or demonstrate the adapter/CLI/API path that writes full-app, panel, and module screenshots with metadata/artifact refs.
6. Fix MT-049 mechanical command evidence. Provide command receipts/log artifacts for `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check`, or record exact blockers. Align the implementation if `--check` is the real command and `--all` is not supported.
7. For full MT proof, either provide `POSTGRES_TEST_URL` and rerun the Postgres-backed kernel tests, or record a formal environment blocker with exact tests blocked and non-Postgres proof coverage.
8. After remediation, rerun the MT batch before any whole-WP Master Spec validation.

</topic>
