---
file_id: integration-validation-appendage-20260515-spec-fail
file_kind: validation_appendage
updated_at: "2026-05-15T15:27:19Z"
wp: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
role: "INTEGRATION_VALIDATOR"
status: "fail"
---

<topic id="summary" status="fail" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T15:27:19Z" summary="MT batch checks passed, but whole-WP Master Spec validation failed.">

# Integration Validator Spec-Fail Appendage

VALIDATION_CONTEXT: OK
MT_BATCH_VERDICT: PASS
WHOLE_WP_MASTER_SPEC_VERDICT: FAIL
MERGE_TO_MAIN: NOT_AUTHORIZED
PUSH_OR_SYNC_GOV_TO_MAIN: NOT_AUTHORIZED
WORKFLOW_VALIDITY: OPERATOR_WAIVED_STANDARD_WP_VALIDATOR_FLOW
SCOPE_VALIDITY: IN_SCOPE
VALIDATOR_RISK_TIER: HIGH

Independent MT rerun accepted the rebased product candidate at `5e430a2dfef10d0b0d5fe5c48116cf2f8023e93d` for batch-test purposes. The candidate is based on current `origin/main` and merge-tree conflict scanning was clean. The implementation still fails the resolved current Master Spec requirements for executable Kernel V1 WriteBox, CRDT persistence, promotion, direct-edit denial evidence, and DCC exposure.

</topic>

<topic id="checks-run" status="complete" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T15:27:19Z" summary="Independent checks used before whole-WP judgment.">

# Checks Run

- `git fetch origin feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`: PASS, remote candidate resolved to `5e430a2dfef10d0b0d5fe5c48116cf2f8023e93d`.
- `git merge-base --is-ancestor origin/main origin/feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`: PASS.
- `git merge-tree origin/main origin/feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`: PASS, no conflict output.
- Replay worktree `../wtc-preuse-hardening-v1-replay`: fast-forwarded to `5e430a2d`.
- All `kernel_*.rs` integration test binaries: PASS, 66 binaries.
- `storage_conformance`, `calendar_storage_tests`, `mcp_e2e_tests`, `micro_task_executor_tests`: PASS with `runtime-full,duckdb-flight-recorder` and `POSTGRES_TEST_URL`.
- `model_session_scheduler_tests`: PASS serially with `-- --test-threads=1`.
- `just gov-check`: PASS.
- `node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --check`: PASS.
- `just build-order-sync`: PASS, already up to date.
- `node .GOV/roles_shared/scripts/topology/artifact-hygiene-check.mjs`: PASS.
- `just phase-check VERDICT ...`: GOVERNANCE DEBT, failed on stale open review/notification route for `INTEGRATION_VALIDATOR_UNASSIGNED`; product spec FAIL below does not depend on this governance debt.

</topic>

<topic id="spec-findings" status="fail" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T15:27:19Z" summary="Whole-WP Master Spec findings.">

# Spec Findings

## FAIL-001 WriteBoxV1 envelope is incomplete

Master Spec section `2.3.13.10` requires WriteBoxV1 to carry stable write box id, workspace id, actor id/kind, CRDT site id, target record refs, base snapshot or state vector refs, intent summary, operation payload refs, schema version, validation state, denial or promotion receipt refs, and replay metadata.

The candidate defines a `WriteBoxCommon` plus specialized boxes, but the shared contract lacks several required fields: CRDT site id, target record refs, base snapshot refs, intent summary, operation payload refs, schema version, denial/promotion receipt refs, and replay metadata. Evidence: `src/backend/handshake_core/src/kernel/write_boxes.rs:78`, `src/backend/handshake_core/src/kernel/write_boxes.rs:98`, `src/backend/handshake_core/src/kernel/write_boxes.rs:142`, `src/backend/handshake_core/src/kernel/write_boxes.rs:295`.

## FAIL-002 CRDT persistence is descriptor-only, not storage-backed

Master Spec sections `2.3.13.10` and `3.3` require restart-replayable CRDT update storage, snapshots, state vectors, Postgres/EventLedger authority linkage, and no hidden SQLite authority path.

The candidate defines `CrdtUpdateRecordV1`, `CrdtSnapshotRecordV1`, and a `kernel_crdt_postgres_update_log_contract`, but there is no Postgres storage migration or storage trait implementation for `kernel_crdt_updates` or snapshots in `src/backend/handshake_core/src/storage/postgres.rs`. Evidence: `src/backend/handshake_core/src/kernel/crdt/persistence.rs:29`, `src/backend/handshake_core/src/kernel/crdt/persistence.rs:91`, `src/backend/handshake_core/src/kernel/crdt/persistence.rs:164`, `src/backend/handshake_core/src/kernel/crdt/snapshot.rs:16`; deterministic source search found no `kernel_crdt_updates` storage implementation under `src/backend/handshake_core/src/storage/`.

## FAIL-003 Promotion bridge does not append the required EventLedger events or cover required failure receipts

Master Spec section `2.3.13.10` requires promotion to read a validated write box, confirm actor eligibility and authority class, verify schema and state-vector freshness, reject duplicate/stale requests by idempotency key, and append promotion-request, promotion-accepted, or promotion-rejected events to the Postgres EventLedger. Duplicate promotion, stale state vector, simultaneous operator/model promotion, validation failure after merge, Postgres write failure, and projection rebuild failure must each leave replayable receipts.

The candidate bridge returns an optional `EventLedgerMapping` for accepted promotion and `None` for rejected promotion. It does not perform an EventLedger append, does not emit promotion-request/rejected mappings, and does not model the required failure receipt set. Evidence: `src/backend/handshake_core/src/kernel/crdt/promotion_bridge.rs:75`, `src/backend/handshake_core/src/kernel/crdt/promotion_bridge.rs:81`, `src/backend/handshake_core/src/kernel/crdt/promotion_bridge.rs:139`, `src/backend/handshake_core/src/kernel/crdt/promotion_bridge.rs:180`.

## FAIL-004 Direct-edit denial evidence is not the required durable WriteBoxDirectEditDeniedV1 evidence

Master Spec section `2.3.13.10` requires direct edit denials to produce durable `WriteBoxDirectEditDeniedV1` evidence with actor, target, attempted action, denial reason, recovery instruction, and linked UI or API response.

The candidate denial type carries denial id, trace id, code, reason, replacement action ids, evidence refs, receipt mappings, and event mappings, but it does not include the explicit actor, target, attempted action, recovery instruction, or linked UI/API response fields required for durable denial evidence. Evidence: `src/backend/handshake_core/src/kernel/action_envelope.rs:104`, `src/backend/handshake_core/src/kernel/direct_edit_guard.rs:280`.

## FAIL-005 DCC required Kernel Action Catalog / Write Box projections are incomplete

Master Spec section `10.11.5.28` requires typed DCC projections for action catalog viewer, write box queue, direct-edit denial view, promotion preview, projection freshness badges, and stable element identifiers for those rows/previews/badges.

The candidate has a backend `DccPanelKind::ActionCatalog`, `ProposalState`, and `ApprovalPreview`, but no explicit write-box queue rows, direct-edit denial view, promotion preview state, projection freshness badges, or stable element identifiers for the required surfaces. The app layer has no corresponding DCC implementation changes for KernelActionCatalog/WriteBox exposure. Evidence: `src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs:22`, `src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs:59`, `src/backend/handshake_core/src/kernel/dcc_mvp_runtime_surface.rs:121`; deterministic app search found no KernelActionCatalog/write-box/direct-edit/promotion-preview UI implementation under `app/`.

</topic>

<topic id="remediation-plan" status="open" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-15T15:27:19Z" summary="Single remediation plan for Kernel Builder.">

# Remediation Plan

1. Replace the current partial write-box structs with a real `WriteBoxV1` envelope or equivalent product contract containing every field required by section `2.3.13.10`; update validators and tests to fail when any required field is absent.
2. Add Postgres-backed CRDT update and snapshot persistence: migrations, storage trait methods, append/list/replay APIs, uniqueness constraints, state-vector fields, replay metadata JSON, and tests that persist/replay after reconnect using `POSTGRES_TEST_URL`.
3. Promote through the actual EventLedger path: promotion request, accepted, and rejected events must be appended with idempotency enforcement and replayable receipts for duplicate, stale, simultaneous, validation-failed, Postgres-write-failed, and projection-rebuild-failed cases.
4. Replace generic `KernelActionDenialV1` direct-edit evidence or extend it into `WriteBoxDirectEditDeniedV1` with actor, target, attempted action, denial reason, recovery instruction, linked UI/API response, receipt refs, and EventLedger refs.
5. Complete the DCC projection surface for section `10.11.5.28`: action catalog rows, write-box queue rows, direct-edit denial rows, promotion preview rows, freshness badges, stable element ids, and app/backend tests proving controls cannot directly mutate EventLedger authority.
6. Rerun the same independent validation set: full `kernel_*` batch, named runtime-full suites, scheduler serial, Postgres suites, `gov-check`, stub-contract check, build-order sync, artifact hygiene, plus new storage/promotion/DCC tests that fail on the gaps above.

</topic>
