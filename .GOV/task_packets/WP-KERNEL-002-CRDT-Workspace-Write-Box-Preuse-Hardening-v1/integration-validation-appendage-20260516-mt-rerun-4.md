---
file_id: integration-validation-appendage-20260516-mt-rerun-4
file_kind: validation_appendage
updated_at: 2026-05-15T22:41:24Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
candidate_worktree: D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-preuse-hardening-v1
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
candidate_head_sha: 0ec31c074177aed4d87807387f2d055de3590e3c
baseline_main_sha: e11ba59793490028262089f782523eb51cf1f1f7
mt_batch_verdict: FAIL
whole_wp_master_spec_validation: NOT_RUN
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T22:41:24Z" ingestable="true" summary="Kernel Builder remediation revalidation before Master Spec validation.">

Handshake (Product): revalidation checked the Kernel Builder remediation in `wtc-preuse-hardening-v1`. The working tree contains the remediation as uncommitted changes on top of stale branch commit `0ec31c074177aed4d87807387f2d055de3590e3c`.

Repo Governance: no merge, sync, staging, commit, or push was performed. Whole-WP coding-vs-current-Master-Spec validation was not run because the MT batch and current-main integration checks did not pass.

MECHANICAL_INTERVENTION_CLASSIFICATION:
- Runtime route drift: present. Candidate `HEAD` is stale against current `main`.
- Scope/worktree drift: present. The remediation is in dirty working tree state, not a clean candidate commit.
- Session/ACP drift: accepted under Operator waiver, not used as the product blocker.
- Documentation/protocol drift: present but secondary to product and integration blockers.
- Current-main drift: blocking. `main` is not an ancestor of candidate `HEAD`; branch is 8 behind / 1 ahead and `git merge-tree main HEAD` reports conflicts.

</topic>

<topic id="checks-run" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T22:41:24Z" ingestable="true" summary="Commands and evidence used for rerun 4.">

CHECKS_RUN:
- `git status --short --branch` in candidate and `handshake_main`.
- `git rev-parse HEAD main origin/main`.
- `git merge-base --is-ancestor main HEAD`: FAIL.
- `git rev-list --left-right --count main...HEAD`: `8 1`.
- `git merge-tree main HEAD`: FAIL, conflicts in migration, kernel catalog/module, storage, lib, workflows, workspace safety, and tests.
- `git diff --name-status main...HEAD`: stale committed branch would add many Kernel002 files but also conflicts with current-main work.
- `git diff --name-status -- ...`: remediation is uncommitted dirty work.
- Static probes for Serde, source hashes, MT-055 paths, and MT-050 EventLedger promotion/denial proof.
- `cargo check --manifest-path src\backend\handshake_core\Cargo.toml`: PASS with warnings.
- Focused MT-050 through MT-061 cargo test batch: TIMED_OUT after 604 seconds; abandoned cargo/rustc child processes were stopped.

TEST_VERDICT: PARTIAL.

Some compile proof exists, but focused test proof is inconclusive due to timeout. The remaining blockers are visible through deterministic source and git checks.

</topic>

<topic id="blocking-findings" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T22:41:24Z" ingestable="true" summary="Blocking product and integration findings.">

MT_BATCH_VERDICT: FAIL.

WHOLE_WP_MASTER_SPEC_VALIDATION: NOT_RUN.

FINDING-1: Candidate is not current-main integration-ready.

Evidence:
- Candidate branch `feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1` is at `0ec31c074177aed4d87807387f2d055de3590e3c`; current `main` and `origin/main` are `e11ba59793490028262089f782523eb51cf1f1f7`.
- `git merge-base --is-ancestor main HEAD`: FAIL.
- `git rev-list --left-right --count main...HEAD`: `8 1`.
- `git merge-tree main HEAD`: reports conflicts in `0017_skill_bank_distillation.sql`, `action_catalog.rs`, `kernel/mod.rs`, `role_mailbox_claim_lease.rs`, `lib.rs`, `postgres.rs`, and `micro_task_executor_tests.rs`, with sidecar evidence of additional conflict patterns in runtime/storage/workflow files.
- Dirty candidate state includes many tracked modifications plus untracked `src/backend/handshake_core/tests/kernel_tail_contract_json_roundtrip_tests.rs`.

Why this blocks: final-lane validation cannot treat a dirty, stale, merge-conflicting worktree as PASS-ready, and Master Spec validation against that candidate would not represent the code that can actually merge to current `main`.

FINDING-2: MT-055 still contains bad generated status target paths.

Evidence:
- `generated_documentation_status_projection.rs` fixed the MT-055 packet-root path, but still uses generated Task Board / traceability target paths such as `.GOV/task_packets/.../task-board.row` and `.GOV/traceability/...rows`.
- The repo authority surfaces are `.GOV/roles_shared/records/TASK_BOARD.md` and `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.
- Sidecar `Test-Path` confirmed the candidate target paths are absent while the authority paths exist.

Why this blocks: MT-055 acceptance requires packet status, MT status, Task Board rows, traceability rows, DCC work views, mirror docs, and operator summaries to regenerate from machine-readable authority. Nonexistent target paths cannot prove that projection contract.

FINDING-3: Provenance is still partly hardcoded and label/length-based.

Evidence:
- Static read shows several modules now compute some `sha256:` values with `sha256_hex`, which is an improvement.
- Remaining blockers are hardcoded provenance/hash strings in `task_contract_lifecycle.rs` and `mechanical_contract_generation.rs` that tests only validate by kind/length rather than by recomputing from the referenced source content.

Why this blocks: previous remediation asked for digest-backed source/provenance hashes, not arbitrary strings that satisfy a length gate. MT-051 and MT-053 still need proof that source hashes actually bind to source content.

FINDING-4: MT-050 promotion is EventLedger-backed, but denied acceptance outcome is not.

Evidence:
- `pre_use_kernel_acceptance_run.rs` now supports `PreUseKernelAcceptanceOutcome::Promoted`, `WriteBoxLifecycleState::Promoted`, non-empty `event_mappings`, and `event_ledger_ref`.
- The promoted branch validates EventLedger-backed evidence.
- The denied branch under `promotion_or_denial_observed` only checks `KernelActionResultStatus::Denied`; direct-edit denial separately requires EventLedger mapping, but that is not the same as proving a denied promotion/acceptance outcome.

Why this blocks: MT-050 acceptance requires promotion/denial evidence for the pre-use path. Direct-edit denial evidence does not cover the denied acceptance branch.

</topic>

<topic id="passed-improvements" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T22:41:24Z" ingestable="true" summary="Remediation items that improved since rerun 3.">

IMPROVEMENTS_CONFIRMED:
- Serde derives are present across the targeted MT-050 through MT-061 contract/projection modules.
- JSON round-trip coverage exists for MT-050, MT-055, MT-057, MT-058, and for MT-051/052/053/056/059/060/061 in `kernel_tail_contract_json_roundtrip_tests.rs`.
- MT-055 packet-root `MT-055.json` and `MT-055.md` refs were fixed.
- MT-050 promoted acceptance now has `Promoted`, non-empty EventLedger mapping, and `event_ledger_ref`.
- `cargo check --manifest-path src\backend\handshake_core\Cargo.toml` passes with warnings.

</topic>

<topic id="combined-remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T22:41:24Z" ingestable="true" summary="Single Kernel Builder remediation plan for rerun 4.">

Kernel Builder remediation plan:

1. Rebase/replay the remediation onto current `main` (`e11ba59793490028262089f782523eb51cf1f1f7`) and produce a clean candidate commit. Preserve current-main Kernel001/product surfaces, especially `0018_kernel_event_ledger.sql`, `0019_kernel_session_queue.sql`, `src/api/kernel.rs`, `src/kernel/proof.rs`, `src/kernel/session_broker.rs`, `kernel_event_ledger_tests.rs`, and `kernel_postgres_event_ledger_tests.rs`.
2. Include the untracked `kernel_tail_contract_json_roundtrip_tests.rs` in the candidate commit or intentionally replace it with equivalent tracked coverage.
3. Resolve merge conflicts in the current-main conflict set. Recheck with `git merge-base --is-ancestor main HEAD`, `git merge-tree main HEAD`, and `git diff --name-status main..HEAD`; PASS-ready state must show no conflicts and no unintended deletes of current-main product files.
4. Fix MT-055 generated status projection targets so Task Board and traceability rows point to real authority/projection paths. Do not leave `.GOV/task_packets/.../task-board.row` or `.GOV/traceability/...rows` unless the code creates and validates those real paths/contracts.
5. Replace remaining hardcoded provenance/hash literals in `task_contract_lifecycle.rs` and `mechanical_contract_generation.rs` with digest computation from actual source bytes or a shared digest-backed source-ref helper. Add negative tests that fail if a fake static digest or length-only value is supplied.
6. Extend MT-050 denied acceptance outcome validation so a denied promotion/acceptance result must include EventLedger-backed evidence, not only direct-edit denial evidence. Add tests for promoted, queued, and denied acceptance branches.
7. Rerun focused MT-050 through MT-061 tests with a narrower batching strategy to avoid timeout, then run `cargo check --manifest-path src\backend\handshake_core\Cargo.toml --target-dir D:\Projects\LLM projects\Handshake\Handshake_Artifacts\handshake-cargo-target`.

Only after those pass should Integration Validator start whole-WP coding-vs-current-Master-Spec validation.

</topic>
