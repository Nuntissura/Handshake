# Repo Governance Refactor Task Board

**Status:** Governance refactor complete; smoketest follow-on remediation open
**Scope:** Governance-only refactor tracking for `/.GOV/`  
**Authority:** `.GOV/roles_shared/docs/REPO_GOVERNANCE_REFACTOR_ROADMAP.md`

**Closeout record:** `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_CLOSEOUT.md`
**Changelog:** `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Notes

- This board tracks governance refactor items only.
- This board does not replace `.GOV/roles_shared/records/TASK_BOARD.md`.
- This board does not create or imply Work Packets.
- Status here is planning truth for governance refactor sequencing, not product execution truth.

## Status Keys

- `PLANNED` = defined but not yet implementation-ready
- `READY` = sequenced and safe to begin
- `IN_PROGRESS` = active refactor item
- `BLOCKED` = waiting on upstream item
- `DONE` = implemented and verified
- `HOLD` = intentionally deferred

## Board

| ID | Status | Workstream | Depends On | Primary Surfaces | Exit Signal |
|---|---|---|---|---|---|
| RGR-01 | DONE | Workflow Truth and Startup Gate | - | `roles_shared/checks`, `roles/orchestrator/scripts`, runtime ledgers | startup blocks split truth and false-ready state |
| RGR-02 | DONE | Transactional Prepare and Status Sync | RGR-01 | `roles/orchestrator/scripts`, `roles_shared/scripts/lib`, records/runtime | activation writes become coherent or fail cleanly |
| RGR-03 | DONE | Direct Review Boundary Enforcement | RGR-01, RGR-02 | communication helpers, receipt routing, communication checks | required coder-validator review becomes boundary-blocking |
| RGR-04 | DONE | Scope and Tool Spill Enforcement | RGR-01 | coder/validator checks, packet template, shared schemas | broad spill becomes visible and blockable |
| RGR-05 | DONE | Computed Policy and Evidence Gate | RGR-01, RGR-02, RGR-03 | shared records, schemas, policy checks, validator checks | final closure becomes computed rather than narrated |
| RGR-06 | DONE | Prevention Ladder, Legacy Cleanup, and Audit Capture | RGR-03, RGR-05 | deprecation surfaces, audit generators, session/runtime helpers | repeated escapes gain prevention assets and audit capture hardens |

## Audit Linkage Convention

- Governance maintenance items and changelog entries must link by stable audit IDs, not by Work Packet IDs.
- Current smoke-review driver:
  - `AUDIT_ID: AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
- Historical comparison driver:
  - `AUDIT_ID: AUDIT-20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`

## Post-Refactor Follow-On Board

| ID | Status | Workstream | Depends On | Evidence | Primary Surfaces | Exit Signal |
|---|---|---|---|---|---|---|
| RGF-01 | DONE | Chat-Visible Refinement Proof | - | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` / `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4` | `roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, `roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`, `roles/orchestrator/checks/orchestrator_gates.mjs` | signature cannot be requested until the full refinement block has been emitted as assistant-authored chat text |
| RGF-02 | DONE | Orchestrator Helper-Agent Product-Code Boundary | RGF-01 | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` / `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4` | `roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, `roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`, `roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`, `roles/orchestrator/checks/orchestrator_gates.mjs` | helper agents cannot write product code unless explicit operator approval is recorded in packet fields |
| RGF-03 | READY | Merge Progression Truth and Main Containment Gate | - | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` / `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4` | final review gates, closeout scripts, `TASK_BOARD.md`, runtime status surfaces | a validated PASS cannot close while the approved commit is absent from `main` unless status explicitly remains awaiting integration |
| RGF-04 | READY | Integration-Validator Topology Preflight and Atomic Closeout | RGF-03 | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` / `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4` | ACP broker control/status, final review scripts, session registry reconciliation | final review either finishes coherently or fails before partial closeout truth is written |
| RGF-05 | READY | Session-Control Self-Settlement and Orphan Prevention | RGF-04 | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` / `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4` | `SESSION_CONTROL_REQUESTS.jsonl`, `SESSION_CONTROL_RESULTS.jsonl`, broker outputs, repair helpers | every control request lands exactly one terminal result and orphaned rejected prompts stop requiring manual truth repair |
| RGF-06 | PLANNED | Historical Failure vs Live Smoketest Modeling | RGF-03 | `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT` + `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` | `TASK_BOARD.md`, `WP_TRACEABILITY_REGISTRY.md`, smoke-review naming, changelog linkage | failed historical closure and active smoketest lineage can coexist without split truth |

## Refactor Sequence (Historical)

1. `RGR-01`
2. `RGR-02`
3. `RGR-03`
4. `RGR-04`
5. `RGR-05`
6. `RGR-06`

## Follow-On Sequence

1. `RGF-03`
2. `RGF-04`
3. `RGF-05`
4. `RGF-06`

## Explicit Holds

- Product-code remediation now runs through product Work Packets only; this board must not create or imply governance WPs.
- New Master Spec edits: `HOLD` for this refactor track.
- Work Packet creation for this governance roadmap: `HOLD` unless the operating model changes later.
