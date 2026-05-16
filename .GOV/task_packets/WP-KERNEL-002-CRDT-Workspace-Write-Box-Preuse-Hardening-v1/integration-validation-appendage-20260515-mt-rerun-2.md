---
file_id: integration-validation-appendage-20260515-mt-rerun-2
file_kind: validation_appendage
updated_at: "2026-05-15T10:58:00Z"
wp: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
role: "INTEGRATION_VALIDATOR"
status: "fail"
---

<topic id="summary" status="fail" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T10:58:00Z" summary="Second MT rerun is blocked by current-main interaction failure before whole-WP Master Spec validation.">

# Integration Validator MT Rerun 2 Appendage

VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PARTIAL
TEST_VERDICT: NOT_RUN_ON_MERGE_RESULT
CODE_REVIEW_VERDICT: FAIL
HEURISTIC_REVIEW_VERDICT: FAIL
SPEC_ALIGNMENT_VERDICT: NOT_RUN
ENVIRONMENT_VERDICT: NOT_APPLICABLE_TO_THIS_BLOCKER
DISPOSITION: NONE
LEGAL_VERDICT: PENDING
SPEC_CONFIDENCE: NONE
WORKFLOW_VALIDITY: OPERATOR_WAIVED_STANDARD_WP_VALIDATOR_FLOW
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: NOT_PROVEN
INTEGRATION_READINESS: NOT_READY
DOMAIN_GOAL_COMPLETION: INCOMPLETE
MECHANICAL_TRACK_VERDICT: FAIL
SPEC_RETENTION_TRACK_VERDICT: NOT_RUN
VALIDATOR_RISK_TIER: HIGH

Kernel Builder reported remediation commit `0ec31c07` on `origin/feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`. Branch-local evidence shows the previous tail MT surfaces were restored, but the branch is not based on current `main` and fails current-main interaction checks before whole-WP Master Spec validation may begin.

FINAL_BATCH_DECISION:
- MT_BATCH_VERDICT: FAIL
- WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN
- MERGE_TO_MAIN: NOT_AUTHORIZED
- PUSH_OR_SYNC_GOV_TO_MAIN: NOT_AUTHORIZED

</topic>

<topic id="checks-run" status="complete" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T10:58:00Z" summary="Checks used for rerun 2.">

# Independent Checks Run

- `git fetch origin feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`: PASS, remote branch updated to `0ec31c07`.
- `git status --short --branch`: on `main` at `e11ba59793490028262089f782523eb51cf1f1f7`; prior untracked validation appendages preserved.
- `git merge-base --is-ancestor e11ba59793490028262089f782523eb51cf1f1f7 origin/feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`: FAIL, current main is not an ancestor of the product branch.
- `git diff --name-status main..origin/feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1 -- src/backend/handshake_core/src/kernel src/backend/handshake_core/tests`: FAIL evidence, the branch would remove current-main Kernel001 files/tests if accepted as-is.
- `git merge-tree main origin/feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`: FAIL, merge conflicts in migration, kernel catalog/module files, storage, lib, and tests.
- Branch-local symbol scan: PASS for previously missing tail contract families (`ValidatorVerdictContractV1`, `MediationInstructionContractV1`, finding report contracts, remediation contract, and related modules exist on the branch).
- Branch-local action catalog count: 56 action ids, including tail action families through `kernel.locus_mt_validation_work_graph.project`.
- Branch-local pre-use acceptance test scan: PASS, tests reference `build_kernel002_pre_use_acceptance_run` and `validate_pre_use_kernel_acceptance_run`.

</topic>

<topic id="failed-mts" status="fail" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T10:58:00Z" summary="Failed MT/current-main interaction findings from rerun 2.">

# Failed MT / Interaction Findings

- CURRENT_MAIN_INTERACTION_CHECKS: FAIL. The product branch is not current with `main`, and the merge result is not mechanically clean.
- MT-022: FAIL/NOT_PROVEN in the integration lane. The branch-local remediation may pass its own Postgres suites, but the branch would remove current-main Postgres proof suites named in the previous validator checks unless the work is replayed onto current `main`.
- MT-049: FAIL in the integration lane. Branch-local governance checks are not enough while the candidate cannot be cleanly integrated with current `main`; projection/build-order/gov-check proof must be rerun after the branch is rebased or replayed onto current `main`.
- V4 closure evidence: FAIL. A high-risk packet is not PASS-ready without `CURRENT_MAIN_INTERACTION_CHECKS`; this check currently fails.

Branch-local remediation evidence for prior failed MT-050 through MT-061 appears present, but it cannot be accepted as the MT batch passing until the same code is integrated onto current `main` without deleting existing Kernel001 surfaces/tests and without merge conflicts.

</topic>

<topic id="evidence" status="complete" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T10:58:00Z" summary="Concrete evidence refs for the current-main interaction failure.">

# Evidence

- Product branch commit: `0ec31c07 feat: restore Kernel002 preuse runtime contracts`.
- Current main commit: `e11ba59793490028262089f782523eb51cf1f1f7 Revert "feat: preserve preuse contract runtime modules"`.
- `git merge-base --is-ancestor e11ba597... origin/feat/...`: returned nonzero; current `main` is not an ancestor of the feature branch.
- `git diff --name-status main..origin/feat/...` reports deletes of current-main Kernel001 runtime/test surfaces:
  - `src/backend/handshake_core/src/kernel/context_bundle.rs`
  - `src/backend/handshake_core/src/kernel/model_adapter.rs`
  - `src/backend/handshake_core/src/kernel/promotion.rs`
  - `src/backend/handshake_core/src/kernel/proof.rs`
  - `src/backend/handshake_core/src/kernel/session_broker.rs`
  - `src/backend/handshake_core/src/kernel/trace_projection.rs`
  - `src/backend/handshake_core/tests/kernel_end_to_end_tests.rs`
  - `src/backend/handshake_core/tests/kernel_event_ledger_tests.rs`
  - `src/backend/handshake_core/tests/kernel_flight_recorder_tests.rs`
  - `src/backend/handshake_core/tests/kernel_postgres_event_ledger_tests.rs`
  - `src/backend/handshake_core/tests/kernel_promotion_trace_tests.rs`
  - `src/backend/handshake_core/tests/kernel_runtime_tests.rs`
- `git merge-tree main origin/feat/...` reports conflicts in:
  - `src/backend/handshake_core/migrations/0017_skill_bank_distillation.sql`
  - `src/backend/handshake_core/src/kernel/action_catalog.rs`
  - `src/backend/handshake_core/src/kernel/mod.rs`
  - `src/backend/handshake_core/src/kernel/role_mailbox_claim_lease.rs`
  - `src/backend/handshake_core/src/lib.rs`
  - `src/backend/handshake_core/src/storage/postgres.rs`
  - `src/backend/handshake_core/tests/micro_task_executor_tests.rs`

DIFF_ATTACK_SURFACES:
- Branch based below current `main`.
- Product branch replacing rather than layering on current Kernel001 runtime surfaces.
- Tail MT remediation present branch-locally but not proven on the current-main merge result.
- Postgres proof suites exist branch-locally/main-locally but are not preserved as a merged integrated surface yet.

COUNTERFACTUAL_CHECKS:
- If `context_bundle.rs` is removed by the merge result, Kernel001 context bundle runtime coverage regresses even if Kernel002 tail tests pass branch-locally.
- If `kernel_event_ledger_tests.rs` is removed by the merge result, the prior no-SQLite and EventLedger proof gates disappear instead of being satisfied.
- If `kernel/mod.rs` is accepted from the branch without resolving the add/add conflict, exports can lose either Kernel001 runtime modules or Kernel002 tail modules.
- If `postgres.rs` is accepted without conflict resolution, branch-local Postgres proof does not establish current-main compatibility.

BOUNDARY_PROBES:
- Branch ancestry vs current `main`.
- Branch-local file set vs current-main file set.
- Merge-tree conflict surface vs claimed ready-for-validation state.
- Action catalog branch-local tail coverage vs integrated main coverage.

NEGATIVE_PATH_CHECKS:
- Ran non-ancestor check; it fails.
- Ran merge-tree; it fails with conflicts.
- Ran deletion scan from `main..feature`; it shows current-main Kernel001 runtime/tests would be removed if treated as a replacement tree.

RESIDUAL_UNCERTAINTY:
- I did not run whole-WP Master Spec validation because current-main interaction failed before that stage.
- I did not run branch-local tests in this pass because branch-local pass claims do not resolve the integration blocker.
- After rebase/replay, the full MT test set, Postgres suites, artifact hygiene, and whole-WP Master Spec validation still need rerun on the integrated candidate.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T10:58:00Z" summary="Single combined remediation plan for Kernel Builder relay.">

# Combined Remediation Plan

1. Rebase or replay commit `0ec31c07` onto current `main` at `e11ba59793490028262089f782523eb51cf1f1f7`; do not force-push over unrelated work without preserving the current branch state.
2. Resolve merge conflicts by preserving current-main Kernel001 runtime surfaces and tests while adding Kernel002 tail modules/actions/tests:
   - keep `context_bundle.rs`, `model_adapter.rs`, `promotion.rs`, `proof.rs`, `session_broker.rs`, and `trace_projection.rs`
   - keep current-main `kernel_*` proof suites, especially EventLedger, runtime, promotion trace, flight recorder, end-to-end, and Postgres tests
   - add the Kernel002 tail modules and tests from `0ec31c07`
3. Resolve conflicts in migration `0017_skill_bank_distillation.sql`, `action_catalog.rs`, `kernel/mod.rs`, `role_mailbox_claim_lease.rs`, `lib.rs`, `postgres.rs`, and `micro_task_executor_tests.rs` as additive integration, not replacement.
4. Re-run the focused MT gates on the rebased/replayed branch:
   - kernel action catalog and tail contract tests
   - pre-use acceptance tests
   - no-SQLite/EventLedger tripwire
   - all Kernel001 proof tests that current main already has
5. Re-run Postgres proof suites with `POSTGRES_TEST_URL` on the integrated candidate, not just the old branch-local tree.
6. Re-run `just gov-check`, stub-contract check, `just build-order-sync`, and artifact hygiene after integration.
7. Return the new integrated commit SHA to Integration Validator for another MT rerun. Whole-WP Master Spec validation remains blocked until this current-main interaction failure is cleared.

</topic>
