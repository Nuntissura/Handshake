---
file_id: integration-validation-appendage-20260515-mt-batch
file_kind: validation_appendage
updated_at: "2026-05-15T00:54:00Z"
wp: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
role: "INTEGRATION_VALIDATOR"
status: "fail"
---

<topic id="summary" status="fail" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T00:54:00Z" summary="Operator-waived MT batch review failed before whole-WP Master Spec validation.">

# Integration Validator MT Batch Appendage

VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: FAIL
TEST_VERDICT: FAIL
CODE_REVIEW_VERDICT: FAIL
HEURISTIC_REVIEW_VERDICT: FAIL
SPEC_ALIGNMENT_VERDICT: NOT_RUN
ENVIRONMENT_VERDICT: BLOCKED
DISPOSITION: NONE
LEGAL_VERDICT: PENDING
SPEC_CONFIDENCE: NONE
WORKFLOW_VALIDITY: PARTIAL
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: NOT_PROVEN
INTEGRATION_READINESS: NOT_READY
DOMAIN_GOAL_COMPLETION: INCOMPLETE
MECHANICAL_TRACK_VERDICT: FAIL
SPEC_RETENTION_TRACK_VERDICT: NOT_RUN
VALIDATOR_RISK_TIER: HIGH

Operator explicitly waived the usual WP Validator/microtask relay flow for Kernel002 and requested Integration Validator batch review of declared MTs first. This appendage records MT-level blockers only. Whole-WP coding-vs-current-Master-Spec validation was intentionally NOT_RUN because the MT batch did not pass.

FINAL_BATCH_DECISION:
- MT_BATCH_VERDICT: FAIL
- WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN
- MERGE_TO_MAIN: NOT_AUTHORIZED
- PUSH_OR_SYNC_GOV_TO_MAIN: NOT_AUTHORIZED

</topic>

<topic id="checks-run" status="complete" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T00:54:00Z" summary="Independent compile, test, static, and governance checks used for the batch review.">

# Independent Checks Run

- `cargo check --no-default-features --features kernel-runtime` from `src/backend/handshake_core` with external `CARGO_TARGET_DIR`: PASS.
- `cargo check --no-default-features --features runtime-full` from `src/backend/handshake_core` with external `CARGO_TARGET_DIR`: PASS with warnings.
- `cargo test --no-default-features --features runtime-full --test kernel_runtime_tests`: PASS, 7 tests.
- `cargo test --no-default-features --features runtime-full --test kernel_event_ledger_tests`: FAIL, `product_sqlite_leakage_tripwire`.
- `cargo test --no-default-features --features runtime-full --test kernel_promotion_trace_tests`: PASS, 9 tests.
- `cargo test --no-default-features --features runtime-full --test kernel_flight_recorder_tests`: PASS, 2 tests.
- `cargo test --no-default-features --features runtime-full --test kernel_end_to_end_tests`: BLOCKED/FAIL, all tests panic with `POSTGRES_TEST_URL not set`.
- `cargo test --no-default-features --features runtime-full --test kernel_postgres_event_ledger_tests`: BLOCKED/FAIL, all tests panic with `POSTGRES_TEST_URL not set`.
- `just gov-check`: FAIL, packet-truth-bundle-check, computed-policy-gate-check, and topology-bundle-check.
- `Test-Path src/backend/handshake_core/src/kernel/{local_model_microtask_loop,mechanical_contract_generation,task_contract_lifecycle,work_packet_full_detail_authority}.rs`: all false.
- `git status --short --branch`: on `main`, clean before this appendage.
- Repo-local `target/` directory search: none found. Cargo output was directed to `../Handshake_Artifacts/handshake-cargo-target`.

</topic>

<topic id="failed-mts" status="fail" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T00:54:00Z" summary="Failed MT findings from the batch review.">

# Failed MTs

- MT-002: FAIL. Reset/no-SQLite posture is not clean under the checked-in tripwire; kernel authority code still contains SQLite/locus_sqlite tokens.
- MT-005: FAIL. Live action catalog has 37 action ids and lacks registered action families for MT-051 through MT-061 lifecycle, verdict, report, remediation, dispatch, and Locus validation-loop surfaces. The same catalog triggers the SQLite leakage tripwire.
- MT-022: FAIL. Postgres residual scope cannot pass while the action catalog still exposes SQLite boundary strings and Postgres proof tests are environment-blocked.
- MT-023: FAIL. Locus reset boundary cannot pass while `locus_sqlite_authority_removed` appears in kernel authority code and the tripwire fails.
- MT-049: FAIL. Fresh `just gov-check` fails packet truth, computed policy gate, and topology bundle checks.
- MT-050: FAIL/NOT_PROVEN. No executable test references the pre-use acceptance builder/validator, and real Postgres-backed end-to-end acceptance tests are blocked by missing `POSTGRES_TEST_URL`.
- MT-051: FAIL. Live main lacks `task_contract_lifecycle.rs` and the `StubContractV1`/`WorkPacketContractV1`/`MicroTaskContractV1` implementation.
- MT-052: FAIL. Live main lacks `work_packet_full_detail_authority.rs`.
- MT-053: FAIL. Live main lacks `mechanical_contract_generation.rs`.
- MT-054: FAIL. Live main lacks `local_model_microtask_loop.rs`.
- MT-055: FAIL. Generated documentation/status projection loop depended on the reverted tail contract/runtime implementation and remains unimplemented in live main.
- MT-056: FAIL. `CoderHandoffContractV1` implementation/search evidence is absent in live main.
- MT-057: FAIL. `ValidatorVerdictContractV1` and `MediationInstructionContractV1` implementation/search evidence is absent in live main.
- MT-058: FAIL. Machine-readable issue/bug/gap/out-of-scope report contracts are absent in live main.
- MT-059: FAIL. Remediation MT/packet generation contracts are absent in live main.
- MT-060: FAIL. Loop scheduler/next-coder dispatch contract is absent in live main.
- MT-061: FAIL. Locus validation-loop graph projection contract is absent in live main.

MT-001, MT-003, MT-004, MT-006 through MT-021, and MT-024 through MT-048 had no additional blocking finding in this batch pass, but they remain subject to whole-WP Master Spec validation after the failed MTs are remediated.

</topic>

<topic id="evidence" status="complete" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T00:54:00Z" summary="Concrete evidence refs for the MT failures.">

# Evidence

- MT-051 through MT-061 are declared in `.GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json:107-117`.
- Live module exports stop before the tail contract modules in `src/backend/handshake_core/src/kernel/mod.rs:10-60`.
- Missing files on live main: `src/backend/handshake_core/src/kernel/local_model_microtask_loop.rs`, `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs`, `src/backend/handshake_core/src/kernel/task_contract_lifecycle.rs`, `src/backend/handshake_core/src/kernel/work_packet_full_detail_authority.rs`.
- The no-SQLite tripwire is in `src/backend/handshake_core/tests/kernel_event_ledger_tests.rs:187-201`.
- Tokens causing the tripwire failure are in `src/backend/handshake_core/src/kernel/action_catalog.rs:687`, `:692`, and `:731`.
- Pre-use acceptance source exists at `src/backend/handshake_core/src/kernel/pre_use_kernel_acceptance_run.rs:103` and `:240`, but `rg` found no test references to `build_kernel002_pre_use_acceptance_run` or `validate_pre_use_kernel_acceptance_run`.
- Commit `c68250cc` added the tail contract-runtime modules; commit `e11ba597` reverted that commit on `main`.

DIFF_ATTACK_SURFACES:
- Reverted tail commit.
- Packet-to-live-module mismatch.
- Storage boundary and no-SQLite tripwire mismatch.
- Postgres proof environment dependency.
- Governance projection/check failure.

COUNTERFACTUAL_CHECKS:
- If `action_catalog.rs` keeps `sqlite_boundary` and `locus_sqlite_authority_removed`, `product_sqlite_leakage_tripwire` continues to fail.
- If `task_contract_lifecycle.rs` is absent, MT-051 cannot define live stub/WP/MT lifecycle contracts.
- If `mechanical_contract_generation.rs` is absent, MT-053 cannot provide deterministic stub promotion or WP-to-MT extraction.
- If `local_model_microtask_loop.rs` is absent, MT-054 and MT-060 cannot provide the fresh-context loop and dispatch policy.
- If `work_packet_full_detail_authority.rs` is absent, MT-052 cannot prove no-context packet execution/regeneration.

BOUNDARY_PROBES:
- Packet declaration vs live exported modules.
- Test law vs action catalog strings.
- Postgres test environment vs EventLedger proof.
- Governance contract vs projection/check state.

NEGATIVE_PATH_CHECKS:
- Ran no-SQLite leakage tripwire; it fails on `action_catalog.rs`.
- Ran Postgres-required tests without `POSTGRES_TEST_URL`; they fail closed instead of silently passing.
- Verified absent tail files with `Test-Path`.

RESIDUAL_UNCERTAINTY:
- Whole-WP Master Spec validation was not run because failed MTs block that stage by Operator instruction.
- Postgres-backed behavior remains unproven until `POSTGRES_TEST_URL` is configured and the Postgres suites pass.
- First-slice MTs may still fail deeper Master Spec review later.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T00:54:00Z" summary="Single combined MT remediation plan for Kernel Builder relay.">

# Combined MT Remediation Plan

1. Restore the c68250cc tail implementation or rebuild equivalent modules on main: `task_contract_lifecycle.rs`, `work_packet_full_detail_authority.rs`, `mechanical_contract_generation.rs`, and `local_model_microtask_loop.rs`; re-export them from `kernel/mod.rs`.
2. Implement explicit product contracts and action catalog entries for MT-051 through MT-061: stub/WP/MT lifecycle, full-detail packet authority, deterministic promotion/extraction, local-model loop, generated projections, coder handoff, validator verdict/mediation, issue reports, remediation generation, dispatch, and Locus validation-loop projection.
3. Repair the no-SQLite tripwire outcome. Either remove/rename SQLite/locus_sqlite strings from kernel authority code while preserving reset intent, or narrow the tripwire to actual dependencies/imports and add a test proving descriptive reset metadata cannot become authority.
4. Add tests that call the MT-050 pre-use acceptance builder/validator and fail on missing manual topics, missing action refs, missing write boxes, direct authority mutation, and missing evidence.
5. Configure `POSTGRES_TEST_URL` and rerun `kernel_end_to_end_tests` plus `kernel_postgres_event_ledger_tests`; keep environment-blocked output as NOT_PROVEN until those pass.
6. Repair `just gov-check` failures: packet truth bundle, computed policy gate, and topology bundle.
7. After those remediations pass, ask Integration Validator to rerun the MT batch. Only if all MTs pass should whole-WP coding-vs-current-Master-Spec validation start.

</topic>
