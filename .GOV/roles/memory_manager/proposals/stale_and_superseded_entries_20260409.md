# Stale and Superseded Entry Report

- Date: 2026-04-09T21:23Z
- WP: WP-MEMORY-HYGIENE_2026-04-09T2115Z
- Reviewer: MEMORY_MANAGER (claude-opus-4-6)

## Stale File-Scope Entries (flagged)

Two entries reference files that no longer exist in the codebase:

| Memory ID | Topic | Gone File | Action Taken |
|---|---|---|---|
| #72 | SMOKE-FIND-20260405-01: ROLE_ORCHESTRATOR | `src/backend/handshake_core/src/governance_artifact_registry.rs` | Flagged via `memory-flag`, importance restored to 0.10 |
| #98 | storage/sqlite.rs | `storage/sqlite.rs` | Flagged via `memory-flag`, importance restored to 0.10 |

Both entries have zero access and are from WP-1-Test or early smoketest runs. They will naturally consolidate during future compaction cycles.

## Superseded Smoketest Generations

Multiple smoketest categories have entries spanning 3-4 run dates. Older entries are superseded by newer findings in the same category:

### ROLE_ORCHESTRATOR (3 entries -> 1 survivor)
- **#72** (20260405, imp=0.01, acc=0) — stale, flagged above
- **#90** (20260406, imp=0.03, acc=0) — superseded, will decay naturally
- **#669** (20260409, imp=0.89, acc=10) — **current authority**

### TOKEN_COST (4 entries -> 1 survivor)
- **#76** (20260405, imp=0.31, acc=43) — high access but superseded by #329
- **#79** (20260405, imp=0.02, acc=2) — superseded
- **#272** (20260408, imp=0.10, acc=0) — superseded
- **#329** (20260408, imp=0.88, acc=56) — **current authority**, maps to RGF-149 (DONE)

### ACP_RUNTIME (4 entries -> 2 survivors)
- **#268** (20260408, imp=0.42, acc=6) — partially superseded
- **#325** (20260408, imp=0.88, acc=56) — **current authority**, maps to RGF-150/151
- **#328** (20260408, imp=0.88, acc=56) — **current authority**, maps to RGF-148
- **#644** (20260409, imp=0.89, acc=10) — **fresh**, active

### WORKFLOW_DISCIPLINE (3 entries -> 2 survivors)
- **#74** (20260405, imp=0.02, acc=2) — superseded
- **#269** (20260408, imp=0.62, acc=34) — **active**, maps to RGF-163
- **#327** (20260408, imp=0.35, acc=5) — **active**, different aspect

### GOVERNANCE_DRIFT (2 entries -> 1 survivor)
- **#270** (20260408, imp=0.14, acc=0) — superseded
- **#330** (20260408, imp=0.88, acc=55) — **current authority**, maps to RGF-150

## Session-Close Semantic Duplicates

23 "Session closed: ROLE on WP-*" semantic entries found, all with zero access. The compaction pass merged many of these. Remaining ones will decay naturally via Ebbinghaus formula.

## Recommendation

No manual intervention needed beyond the two flags already applied. The compaction pass (313 merges, 23 prunes) resolved the bulk of the duplicate and low-value entry accumulation. Superseded smoketest entries will naturally decay through future compaction cycles, and the current authority entries are correctly positioned at high importance.
