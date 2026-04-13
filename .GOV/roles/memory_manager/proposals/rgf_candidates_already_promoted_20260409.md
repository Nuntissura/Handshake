# RGF Candidate Review: 5 Mechanical Candidates Already Promoted

- Date: 2026-04-09T21:23Z
- WP: WP-MEMORY-HYGIENE_2026-04-09T2115Z
- Reviewer: MEMORY_MANAGER (claude-opus-4-6)

## Summary

The mechanical pre-pass identified 5 high-access smoketest entries as RGF promotion candidates. Intelligent review cross-referenced these against the RGF task board and determined **all 5 have already been promoted and completed**. No new RGF proposals are needed for these entries.

## Candidate-to-RGF Mapping

| Memory ID | Smoketest | Category | Access Count | Already Promoted To | RGF Status |
|---|---|---|---|---|---|
| #324 | SMOKE-FIND-20260408-01 | SCRIPT_OR_CHECK | 56 | RGF-150: Single-Writer Lifecycle Truth | DONE |
| #325 | SMOKE-FIND-20260408-02 | ACP_RUNTIME | 56 | RGF-150/151: Active Review Projection | DONE |
| #328 | SMOKE-FIND-20260408-05 | ACP_RUNTIME | 56 | RGF-148: Pre-Start Communication Mesh Gate | DONE |
| #329 | SMOKE-FIND-20260408-06 | TOKEN_COST | 56 | RGF-149: Phase-Level Composite Entrypoints | DONE |
| #330 | SMOKE-FIND-20260408-07 | GOVERNANCE_DRIFT | 55 | RGF-150: Single-Writer Lifecycle Truth | DONE |

## Evidence

Each smoketest finding's "fix direction" aligns precisely with the corresponding RGF's deliverable:

- **#324** called for "closeout sync the single writer for header truth" -> RGF-150 delivered "receipt-driven review reconciliation is now the single writer for review-stage lifecycle truth"
- **#325** called for "route-health projection to treat closed-contained WPs as terminal-green" -> RGF-151 delivered "superseded crash-recovery residue is compacted behind explicit history views"
- **#328** called for "COMM_MESH_STARTUP gate with per-role handshake receipts" -> RGF-148 delivered exactly this
- **#329** called for "consolidate checks and tests into phase-level composite commands" -> RGF-149 delivered "each lifecycle phase exposes one stable operator command"
- **#330** called for "collapse governance truth onto one authority surface per WP lifecycle stage" -> RGF-150 delivered "packet headers, runtime status, task board, and route-health views only mirror that computed authority"

## Recommendation

These memory entries remain valuable as historical evidence for their corresponding RGFs. No action needed beyond this documentation. The mechanical candidate detection correctly identified high-signal entries but lacked the cross-referencing capability to check the RGF task board.

## Improvement Suggestion

The mechanical memory-manager script could cross-reference RGF candidates against `REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` to filter out already-promoted findings. This would reduce false-positive candidate reports in future runs.
