---
file_id: integration-validation-appendage-20260515-mt-rerun-3
file_kind: validation_appendage
updated_at: 2026-05-15T20:27:27Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1-replay
candidate_sha: 5e430a2dfef10d0b0d5fe5c48116cf2f8023e93d
baseline_main_sha: e11ba59793490028262089f782523eb51cf1f1f7
mt_batch_verdict: FAIL
whole_wp_master_spec_validation: NOT_RUN
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T20:27:27Z" ingestable="true" summary="Operator-directed MT rerun before Master Spec validation.">

Handshake (Product): MT rerun validation was performed against candidate `5e430a2dfef10d0b0d5fe5c48116cf2f8023e93d` in `wtc-preuse-hardening-v1-replay`.

Repo Governance: The Operator waived the usual WP Validator and microtask relay ceremony for this review. This appendage records the Integration Validator MT judgment only. Whole-WP coding-vs-current-Master-Spec validation was intentionally not run because the MT batch did not pass.

MECHANICAL_INTERVENTION_CLASSIFICATION:
- Runtime route drift: checked. Candidate branch is now one commit ahead of current `main`; `main` is an ancestor of `HEAD`.
- Scope/worktree drift: checked. Product review ran in `wtc-preuse-hardening-v1-replay`; appendage write ran from `handshake_main`.
- Session/ACP drift: accepted by Operator waiver. Lack of WP Validator pass is not used as a product blocker in this appendage.
- Documentation/protocol drift: present but not the primary product blocker. Startup/gov-check still has a topology-bundle failure in `handshake_main`; final governance sync remains deferred until product MTs pass.
- Implementation/proof drift: confirmed. Rust tests prove many in-memory constructors/validators, but several MT contracts still fail durable machine-readable, digest-backed, and live authority-path expectations.

</topic>

<topic id="checks-run" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T20:27:27Z" ingestable="true" summary="Validation commands and direct evidence reads.">

INDEPENDENT_CHECKS_RUN:
- `git status --short --branch` in `handshake_main` and `wtc-preuse-hardening-v1-replay`.
- `git merge-base --is-ancestor main HEAD`: PASS.
- `git merge-tree main HEAD`: clean tree result, no conflict output.
- `git diff --name-status main..HEAD`: add/modify only across Kernel002 product surfaces.
- `cargo check --no-default-features --features runtime-full`: PASS with warnings.
- Focused prior-failure tests: `kernel_event_ledger_tests`, `kernel_action_catalog_tests`, `kernel_postgres_control_plane_residual_tests`, `kernel_locus_work_tracking_reset_tests`, `kernel_pre_use_acceptance_run_tests`: PASS.
- Tail contract tests for MT-051 through MT-061: PASS.
- Full non-Postgres-blocked `kernel_*_tests.rs` set, excluding `kernel_end_to_end_tests` and `kernel_postgres_event_ledger_tests` because they require `POSTGRES_TEST_URL`: PASS, 64 test targets.
- Direct source probes for `Serialize`/`Deserialize`, bad MT-055 paths, synthetic `source_hash` values, and queued-only pre-use promotion behavior.

TEST_VERDICT: PARTIAL_PASS_WITH_PRODUCT_FINDINGS.

The test suite is not enough to pass this MT batch because the uncovered blockers are contract-shape and proof-quality failures, not simple constructor-validation failures.

</topic>

<topic id="mt-verdicts" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T20:27:27Z" ingestable="true" summary="MT-level verdicts for the rerun.">

MT_BATCH_VERDICT: FAIL.

WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN.

MT_VERDICTS:
- MT-002: PASS. Reset/no-SQLite posture is proven by `kernel_reset_invariants_tests` and `kernel_event_ledger_tests`.
- MT-005: PASS. Action catalog now exposes required model-facing actions and the no-SQLite catalog tripwire passes.
- MT-022: PASS/PARTIAL_ENV. Postgres residual scope and no-SQLite authority mapping pass non-live tests; live DB proof remains environment-gated by `POSTGRES_TEST_URL`.
- MT-023: PASS. Locus reset migration tests pass, including legacy-local rejection and projection behavior.
- MT-049: DEFERRED_GOVERNANCE. Current-main mechanical proof passes; packet/task-board recorded proof remains deferred under Operator waiver and must be updated during final governance sync if product passes.
- MT-050: FAIL. Pre-use acceptance remains synthetic and queued-only; it does not prove an actual EventLedger-backed promotion or denial authority path.
- MT-051: FAIL. Stub/WP/MT lifecycle contracts are Rust-only in-memory records and are not serializable durable machine contracts.
- MT-052: FAIL. Full-detail/source-plan authority contract is not serializable durable machine authority.
- MT-053: FAIL. Mechanical generation records provenance/source hashes as descriptor strings rather than digest-backed proof.
- MT-054: PASS. Local-model fresh-context MT loop derives Serde and covers bundle, actions, write boxes, retry, handoff, requeue, memory, receipts, and outcomes.
- MT-055: FAIL. Generated documentation/status projection uses wrong `microtasks/MT-055.*` paths and is not serializable durable machine authority.
- MT-056: FAIL. Coder handoff validation request contract is not serializable and uses synthetic source hashes.
- MT-057: FAIL. Validator verdict/mediation contract is not serializable durable machine authority.
- MT-058: FAIL. Finding report contracts are not serializable and use synthetic source hashes.
- MT-059: FAIL. Remediation generation contract is not serializable and uses synthetic source hashes.
- MT-060: FAIL. Loop scheduler contract/projection is not serializable durable machine authority.
- MT-061: FAIL. Locus MT validation work graph contract is not serializable and uses synthetic source hashes.

</topic>

<topic id="blocking-findings" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T20:27:27Z" ingestable="true" summary="Concrete blockers preventing MT batch pass.">

FINDING-1: Tail MT contracts are mostly not durable machine-readable product contracts.

Evidence:
- Existing authority-style kernel records derive Serde: `src/backend/handshake_core/src/kernel/action_envelope.rs:1`, `src/backend/handshake_core/src/kernel/write_boxes.rs:3`.
- MT-054 follows that pattern: `src/backend/handshake_core/src/kernel/local_model_microtask_loop.rs:3` imports Serde and its contract structs derive `Serialize, Deserialize`.
- Focused tail modules such as `pre_use_kernel_acceptance_run.rs`, `task_contract_lifecycle.rs`, `work_packet_full_detail_authority.rs`, `generated_documentation_status_projection.rs`, `coder_handoff_validation_request.rs`, `validator_verdict_mediation_contract.rs`, `validator_finding_report_contract.rs`, `remediation_work_generation_contract.rs`, `mt_loop_scheduler_contract.rs`, and `locus_mt_validation_work_graph.rs` define public contract/projection structs without `Serialize, Deserialize`.

Why this blocks: MT-051 through MT-061 repeatedly require machine-readable contracts/projections. In-memory Rust-only records cannot be persisted, emitted, compared, or consumed by no-context models and runtime tooling as durable machine contracts.

FINDING-2: MT-055 has a concrete bad path in the generated documentation/status projection contract.

Evidence:
- `src/backend/handshake_core/src/kernel/generated_documentation_status_projection.rs:166` points to `.GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/microtasks/MT-055.json`.
- `src/backend/handshake_core/src/kernel/generated_documentation_status_projection.rs:200` points to `.GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/microtasks/MT-055.md#status`.
- Deterministic path check: `microtasks/MT-055.json` and `microtasks/MT-055.md` are absent; root `MT-055.json` and `MT-055.md` exist.

Why this blocks: the projection contract is supposed to regenerate status/docs from machine-readable authority. It currently references non-existent authority/projection paths for MT-055.

FINDING-3: Several provenance/source-hash fields are synthetic labels, not digest-backed proof.

Evidence:
- `src/backend/handshake_core/src/kernel/generated_documentation_status_projection.rs:669` formats `sha256:{source_id}:kernel002-mt055`.
- `src/backend/handshake_core/src/kernel/coder_handoff_validation_request.rs:660` formats `sha256:{path}:mt056`.
- `src/backend/handshake_core/src/kernel/validator_finding_report_contract.rs:802` formats `sha256:{source_ref}:mt058`.
- `src/backend/handshake_core/src/kernel/locus_mt_validation_work_graph.rs:636` formats `sha256:{source_ref}`.

Why this blocks: MT-051, MT-053, MT-056, MT-058, MT-059, and MT-061 require source hashes/provenance that preserve and prove source identity. A string prefixed with `sha256:` is not a digest unless it is actually computed from the referenced source.

FINDING-4: MT-050 pre-use acceptance proves a preview/queue, not the actual promotion/denial authority path.

Evidence:
- `src/backend/handshake_core/src/kernel/pre_use_kernel_acceptance_run.rs:147` sets promotion lifecycle to `PromotionQueued`.
- `src/backend/handshake_core/src/kernel/pre_use_kernel_acceptance_run.rs:158` keeps `event_ledger_ref: None`.
- `src/backend/handshake_core/src/kernel/pre_use_kernel_acceptance_run.rs:172` and `:182` leave event mappings empty.
- `src/backend/handshake_core/src/kernel/pre_use_kernel_acceptance_run.rs:395` validates that no EventLedger ref exists.
- `src/backend/handshake_core/src/kernel/pre_use_kernel_acceptance_run.rs:444` treats queued status plus empty mappings as acceptable.

Why this blocks: MT-050 acceptance says a no-context model can submit, trigger validation, receive promotion/denial, view DCC projections, and inspect evidence. A queued-only synthetic run does not prove the actual EventLedger-backed authority path.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T20:27:27Z" ingestable="true" summary="Single combined remediation plan for Kernel Builder relay.">

Kernel Builder remediation plan:

1. Add Serde support to every Kernel002 tail contract/projection type that represents a machine-readable product contract or projection. At minimum cover MT-050, MT-051, MT-052, MT-053, MT-055, MT-056, MT-057, MT-058, MT-059, MT-060, and MT-061 modules. Match the existing `action_envelope.rs`, `write_boxes.rs`, and `local_model_microtask_loop.rs` pattern.
2. Add round-trip serialization tests for each remediated contract/projection module. Tests must serialize to JSON, deserialize back, and assert stable schema id, contract id, authority refs, source refs, lifecycle/verdict/report state, and projection fields.
3. Replace synthetic `sha256:*` provenance strings with real digest computation or a shared digest-backed source-ref type. Tests must fail on fake labels such as `sha256:<path>:mt056` when no digest was computed from the source content.
4. Fix MT-055 source and target paths from `.../microtasks/MT-055.*` to the actual packet-root `.../MT-055.*`, unless the code first creates and uses a real `microtasks/` authority directory. Add a path existence/authority-ref test.
5. Upgrade MT-050 pre-use acceptance so at least one path proves real promotion or denial authority through EventLedger evidence. If a live DB is unavailable, split the contract honestly: `PREVIEW_QUEUE_PROOF` may pass separately, but MT-050 must not claim full pre-use acceptance without an EventLedger-backed promotion/denial receipt or an explicit environment-blocked result.
6. Preserve the current passes for MT-002, MT-005, MT-022, MT-023, MT-054, and current-main integration. Do not regress the no-SQLite tripwire, action catalog, Postgres residual, Locus reset, and clean merge proof.
7. After remediation, request another Integration Validator MT rerun. Only if the MT batch passes should whole-WP coding-vs-current-Master-Spec validation start.

Expected recheck commands:
- `cargo check --no-default-features --features runtime-full`
- Focused tests for changed modules.
- Full non-Postgres-blocked `kernel_*_tests.rs` batch.
- Source probes for `Serialize, Deserialize`, digest-backed provenance, MT-055 paths, and EventLedger-backed MT-050 evidence.
- Current-main interaction checks: `git merge-base --is-ancestor main HEAD`, `git merge-tree main HEAD`, and `git diff --name-status main..HEAD`.

</topic>
