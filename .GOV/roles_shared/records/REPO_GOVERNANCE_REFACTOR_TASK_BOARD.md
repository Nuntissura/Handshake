# Repo Governance Refactor Task Board

**Status:** Governance refactor complete  
**Scope:** Governance-only refactor tracking for `/.GOV/`  
**Authority:** `.GOV/roles_shared/docs/REPO_GOVERNANCE_REFACTOR_ROADMAP.md`

**Closeout record:** `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_CLOSEOUT.md`

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

## Immediate Sequence

1. `RGR-01`
2. `RGR-02`
3. `RGR-03`
4. `RGR-04`
5. `RGR-05`
6. `RGR-06`

## Explicit Holds

- Product-code remediation for the audited Schema Registry gaps: `HOLD` until the governance baseline above is in place.
- New Master Spec edits: `HOLD` for this refactor track.
- Work Packet creation for this governance roadmap: `HOLD` unless the operating model changes later.
