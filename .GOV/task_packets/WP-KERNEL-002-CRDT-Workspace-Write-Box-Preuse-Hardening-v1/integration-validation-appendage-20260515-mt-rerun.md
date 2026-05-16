---
file_id: integration-validation-appendage-20260515-mt-rerun
file_kind: validation_appendage
updated_at: "2026-05-15T08:55:00Z"
wp: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
role: "INTEGRATION_VALIDATOR"
status: "fail"
---

<topic id="summary" status="fail" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T08:55:00Z" summary="MT rerun still fails before whole-WP Master Spec validation.">

# Integration Validator MT Rerun Appendage

VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PARTIAL
TEST_VERDICT: FAIL
CODE_REVIEW_VERDICT: FAIL
HEURISTIC_REVIEW_VERDICT: FAIL
SPEC_ALIGNMENT_VERDICT: NOT_RUN
ENVIRONMENT_VERDICT: BLOCKED
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

Operator explicitly instructed an Integration Validator MT rerun for Kernel002 outside the usual WP Validator/microtask relay flow. Per instruction, whole-WP coding-vs-current-Master-Spec validation remains NOT_RUN because the MT batch did not pass.

FINAL_BATCH_DECISION:
- MT_BATCH_VERDICT: FAIL
- WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN
- MERGE_TO_MAIN: NOT_AUTHORIZED
- PUSH_OR_SYNC_GOV_TO_MAIN: NOT_AUTHORIZED

</topic>

<topic id="checks-run" status="complete" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T08:55:00Z" summary="Independent checks used for the MT rerun.">

# Independent Checks Run

- `git status --short --branch`: on `main`, matching `origin/main`; existing untracked prior appendage was untouched.
- `git diff --stat facce56f879d4ee990f62566b12a8b26d8bc61d7..HEAD -- src/backend/handshake_core/src/kernel src/backend/handshake_core/tests README.md app tests`: Kernel002 product diff present on main, with the c68250cc tail commit reverted by e11ba597.
- `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --no-default-features --features kernel-runtime --target-dir "../Handshake_Artifacts/handshake-cargo-target"`: PASS.
- `cargo check --manifest-path src/backend/handshake_core/Cargo.toml --no-default-features --features runtime-full --target-dir "../Handshake_Artifacts/handshake-cargo-target"`: PASS with existing warnings.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --no-default-features --features runtime-full --test kernel_runtime_tests --target-dir "../Handshake_Artifacts/handshake-cargo-target"`: PASS, 7 tests.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --no-default-features --features runtime-full --test kernel_event_ledger_tests --target-dir "../Handshake_Artifacts/handshake-cargo-target"`: FAIL, `product_sqlite_leakage_tripwire`.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --no-default-features --features runtime-full --test kernel_promotion_trace_tests --target-dir "../Handshake_Artifacts/handshake-cargo-target"`: PASS, 9 tests.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --no-default-features --features runtime-full --test kernel_flight_recorder_tests --target-dir "../Handshake_Artifacts/handshake-cargo-target"`: PASS, 2 tests.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --no-default-features --features runtime-full --test kernel_end_to_end_tests --target-dir "../Handshake_Artifacts/handshake-cargo-target"`: FAIL/BLOCKED, all 7 tests require `POSTGRES_TEST_URL`.
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --no-default-features --features runtime-full --test kernel_postgres_event_ledger_tests --target-dir "../Handshake_Artifacts/handshake-cargo-target"`: FAIL/BLOCKED, all 7 tests require `POSTGRES_TEST_URL`.
- `just gov-check`: PASS.
- `node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --check`: PASS, 189 stub contracts.
- `just build-order-sync`: PASS, already up to date.
- `node .GOV/roles_shared/scripts/topology/artifact-hygiene-check.mjs`: FAIL, external artifact hygiene debt: `test-logs` unknown noncanonical artifact directory and reclaimable `kernel-mt017-proof-harness-target`.
- `rg`/`Test-Path` evidence checks: tail contract modules and tail contract symbols remain absent on live main.

</topic>

<topic id="failed-mts" status="fail" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T08:55:00Z" summary="Failed MT findings from the rerun.">

# Failed MTs

- MT-002: FAIL. Reset/no-SQLite posture is still not clean because `kernel_event_ledger_tests::product_sqlite_leakage_tripwire` fails against `src/backend/handshake_core/src/kernel/action_catalog.rs`.
- MT-005: FAIL. Live action catalog still has only 37 action ids and does not contain registered tail action families for MT-051 through MT-061. It also still carries SQLite/locus_sqlite hooks or text that trip the no-SQLite gate.
- MT-022: FAIL/NOT_PROVEN. Postgres residual scope remains unproven because both Postgres-backed proof suites are blocked by missing `POSTGRES_TEST_URL`, and the no-SQLite tripwire still fails.
- MT-023: FAIL. Locus reset migration remains blocked by the same SQLite/locus_sqlite leakage through `action_catalog.rs`, even though Locus reset code contains validation language against SQLite authority.
- MT-050: FAIL/NOT_PROVEN. The pre-use acceptance builder/validator exist in source, but `rg` found no test references to `build_kernel002_pre_use_acceptance_run` or `validate_pre_use_kernel_acceptance_run`, and Postgres-backed end-to-end acceptance remains environment-blocked.
- MT-051: FAIL. `task_contract_lifecycle.rs` is absent on live main, and `StubContractV1`, `WorkPacketContractV1`, and `MicroTaskContractV1` were not found in live source/tests.
- MT-052: FAIL. `work_packet_full_detail_authority.rs` is absent on live main.
- MT-053: FAIL. `mechanical_contract_generation.rs` is absent on live main.
- MT-054: FAIL. `local_model_microtask_loop.rs` is absent on live main.
- MT-055: FAIL. Generated documentation/status projection loop from contracts remains not proven after the tail contract-runtime revert.
- MT-056: FAIL. `CoderHandoffContractV1` was not found in live source/tests.
- MT-057: FAIL. `ValidatorVerdictContractV1` and `MediationInstructionContractV1` were not found in live source/tests.
- MT-058: FAIL. `IssueReportContractV1`, `BugReportContractV1`, `GapReportContractV1`, and `OutOfScopeReportContractV1` were not found in live source/tests.
- MT-059: FAIL. `RemediationMicroTaskContractV1` was not found in live source/tests.
- MT-060: FAIL. Loop scheduler / next-coder dispatch contract implementation remains not found in live source/tests.
- MT-061: FAIL. Locus work graph projection for MT validation loops remains not found in live source/tests.

MT-049 is no longer a batch blocker in this rerun: `gov-check`, stub-contract check, and build-order sync passed. Artifact hygiene still has closeout debt before any future final push.

MT-001, MT-003, MT-004, MT-006 through MT-021, and MT-024 through MT-048 had no additional blocking finding in this MT rerun, but they remain subject to whole-WP Master Spec validation only after all failed MTs pass.

</topic>

<topic id="evidence" status="complete" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T08:55:00Z" summary="Concrete evidence refs for the rerun failures.">

# Evidence

- Current main is `e11ba59793490028262089f782523eb51cf1f1f7`, `Revert "feat: preserve preuse contract runtime modules"`.
- Reverted tail commit evidence: `git show --name-status e11ba597 -- src/backend/handshake_core/src/kernel` deletes `local_model_microtask_loop.rs`, `mechanical_contract_generation.rs`, `task_contract_lifecycle.rs`, and `work_packet_full_detail_authority.rs`; it also removes their exports from `kernel/mod.rs`.
- Restored-first-slice implementation evidence remains from `24ceabdb`, but tail implementation from `c68250cc` is not present at HEAD.
- Missing tail module paths on live main: `src/backend/handshake_core/src/kernel/local_model_microtask_loop.rs`, `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs`, `src/backend/handshake_core/src/kernel/task_contract_lifecycle.rs`, `src/backend/handshake_core/src/kernel/work_packet_full_detail_authority.rs`.
- Missing tail contract symbol search returned no matches for `StubContractV1`, `WorkPacketContractV1`, `MicroTaskContractV1`, `CoderHandoffContractV1`, `ValidatorVerdictContractV1`, `MediationInstructionContractV1`, `IssueReportContractV1`, `BugReportContractV1`, `GapReportContractV1`, `OutOfScopeReportContractV1`, or `RemediationMicroTaskContractV1`.
- The no-SQLite tripwire assertion is in `src/backend/handshake_core/tests/kernel_event_ledger_tests.rs:201`.
- The current tripwire hits `src/backend/handshake_core/src/kernel/action_catalog.rs`; relevant strings are `hook("sqlite_boundary")` at line 687, the `SQLite boundaries` description at line 692, and `hook("locus_sqlite_authority_removed")` at line 731.
- Pre-use acceptance functions exist at `src/backend/handshake_core/src/kernel/pre_use_kernel_acceptance_run.rs:103` and `src/backend/handshake_core/src/kernel/pre_use_kernel_acceptance_run.rs:240`, but no executable test references were found.
- Postgres environment blockers are explicit at `src/backend/handshake_core/tests/kernel_end_to_end_tests.rs:31`, `src/backend/handshake_core/tests/kernel_end_to_end_tests.rs:43`, and `src/backend/handshake_core/tests/kernel_postgres_event_ledger_tests.rs:15`.
- Action catalog count from live source: 37 action ids, ending at `kernel.markdown_mirror_sync_drift_guard.project`; no MT-051 through MT-061 action families are registered.

DIFF_ATTACK_SURFACES:
- Reverted tail contract-runtime modules.
- Packet-declared 61 MTs vs live module/action catalog coverage.
- No-SQLite reset invariant vs descriptive SQLite/locus_sqlite strings in authority code.
- Postgres proof suites depending on unavailable `POSTGRES_TEST_URL`.
- Pre-use acceptance builder/validator existing without executable coverage.

COUNTERFACTUAL_CHECKS:
- If `action_catalog.rs` retains SQLite/locus_sqlite strings, `product_sqlite_leakage_tripwire` continues to fail.
- If `task_contract_lifecycle.rs` remains absent, MT-051 cannot supply live stub/WP/MT lifecycle contracts.
- If `mechanical_contract_generation.rs` remains absent, MT-053 cannot supply deterministic stub promotion or WP-to-MT extraction.
- If `local_model_microtask_loop.rs` remains absent, MT-054 and MT-060 cannot supply the fresh-context loop and dispatch policy.
- If `work_packet_full_detail_authority.rs` remains absent, MT-052 cannot prove no-context packet execution/regeneration.
- If pre-use acceptance functions remain untested, MT-050 can regress while source-level symbols still exist.

BOUNDARY_PROBES:
- Packet microtask declarations vs live exported kernel modules.
- Action catalog registrations vs MT-051 through MT-061 required contract/action families.
- Test law vs action catalog reset-boundary wording.
- Postgres-backed runtime proof vs local environment configuration.
- Pre-use acceptance source functions vs executable test coverage.

NEGATIVE_PATH_CHECKS:
- Ran no-SQLite leakage tripwire; it fails on `action_catalog.rs`.
- Ran Postgres-required tests without `POSTGRES_TEST_URL`; they fail closed instead of silently passing.
- Verified missing tail files with `Test-Path`; all four required tail files returned false.
- Searched for tail contract symbols; no live source/test matches were found.

RESIDUAL_UNCERTAINTY:
- Whole-WP Master Spec validation was not run because failed MTs block that stage by Operator instruction.
- Postgres-backed behavior remains unproven until a real `POSTGRES_TEST_URL` is configured and the Postgres suites pass.
- First-slice MTs may still fail deeper current-Master-Spec review later.
- Artifact hygiene has external cleanup debt before any final push, but no push is authorized in this failed MT state.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T08:55:00Z" summary="Single combined MT remediation plan for Kernel Builder relay.">

# Combined MT Remediation Plan

1. Restore the c68250cc tail implementation or rebuild equivalent live modules on main: `task_contract_lifecycle.rs`, `work_packet_full_detail_authority.rs`, `mechanical_contract_generation.rs`, and `local_model_microtask_loop.rs`; re-export them from `kernel/mod.rs`.
2. Restore or rebuild explicit product contracts and action catalog entries for MT-051 through MT-061: stub/WP/MT lifecycle, full-detail packet authority, deterministic promotion/extraction, local-model loop, generated projections, coder handoff, validator verdict/mediation, issue reports, remediation generation, dispatch, and Locus validation-loop projection.
3. Repair the no-SQLite tripwire outcome. Either remove/rename SQLite/locus_sqlite strings from Kernel V1 authority code while preserving reset intent, or narrow the tripwire to actual dependencies/imports and add a test proving descriptive reset metadata cannot become authority.
4. Add executable tests that call `build_kernel002_pre_use_acceptance_run` and `validate_pre_use_kernel_acceptance_run`, including failure cases for missing manual topics, missing action refs, missing write boxes, direct authority mutation, and missing evidence.
5. Configure `POSTGRES_TEST_URL` and rerun `kernel_end_to_end_tests` plus `kernel_postgres_event_ledger_tests`; keep these as NOT_PROVEN until they pass against real Postgres.
6. Keep MT-049 clean by rerunning the stub-contract check, `just build-order-sync`, and `just gov-check` after code changes.
7. Before any eventual final push, clear or classify the external artifact-hygiene debt through the governed artifact cleanup path; do not manually delete artifact roots.
8. Ask Integration Validator to rerun the MT batch. Only if all MTs pass should whole-WP coding-vs-current-Master-Spec validation begin.

</topic>
