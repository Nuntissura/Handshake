# AUDIT_20260409_MEMORY_MANAGER_MECHANICAL_CONSERVATISM

- AUDIT_ID: `AUDIT-20260409-MEMORY-MANAGER-MECHANICAL-CONSERVATISM`
- STATUS: APPLIED
- DATE: `2026-04-09`
- SCOPE: repo-governance maintenance only
- DRIVER: operator follow-on after review of the Memory Manager mechanical pre-pass

## Problem

The Memory Manager mechanical lane was doing more than deterministic hygiene. During automatic startup/closeout runs it could:

1. prune memories through the shared decay path
2. demote stale file-scope memories directly
3. resolve contradictions by mechanically picking a winner
4. age-consolidate old low-access memories automatically

Those are judgment calls, not safe default automation.

## Applied Change

- introduced a small `memory-manager-policy.mjs` module to make the conservative mechanical policy explicit and testable
- changed the mechanical pass to use soft decay only (`pruneThreshold=0`) so automatic runs no longer prune memories
- converted stale file-scope handling into report-only candidates
- converted contradiction handling into report-only candidates for intelligent review
- converted age-based consolidation into report-only candidates for intelligent review
- kept deterministic restorative work such as supersession-chain repair and recall-effectiveness restoration
- aligned the Memory Manager protocol and report template language with the new conservative behavior

## Surfaces

- `.GOV/roles/memory_manager/scripts/launch-memory-manager.mjs`
- `.GOV/roles/memory_manager/scripts/memory-manager-policy.mjs`
- `.GOV/roles/memory_manager/tests/memory-manager-policy.test.mjs`
- `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Verification

- `node --test .GOV/roles/memory_manager/tests/memory-manager-policy.test.mjs`
- `node .GOV/roles/memory_manager/scripts/launch-memory-manager.mjs --force`
- report check: `MEMORY_HYGIENE_REPORT.md` now shows `pruned=0 (mechanical prune disabled)` and report-only candidate counts

## Outcome

The mechanical Memory Manager lane is now conservative and report-first. Automatic runs still clean and calibrate the memory system, but destructive or judgment-heavy decisions are deferred to the intelligent review session.
