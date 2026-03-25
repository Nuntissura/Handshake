# Repo Governance Task Item Template

Use this template for rows opened in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`.

## When To Use It

- the change is repo-governance maintenance only
- no Handshake product code is touched
- no Master Spec edit is required
- the work should be tracked without a Work Packet

## Stable IDs

- Evidence documents must carry `AUDIT_ID`
- Smoketest or workflow-proof reviews must also carry `SMOKETEST_REVIEW_ID`
- Board items should use the active board family already in use:
  - `RGR-XX` for roadmap/refactor items
  - `RGF-XX` for post-refactor follow-on items
  - or another explicitly declared board-local family

## Row Template

```md
| <ID> | <PLANNED|READY|IN_PROGRESS|BLOCKED|DONE|HOLD> | <Workstream> | <Depends On or NONE> | <AUDIT_ID / SMOKETEST_REVIEW_ID or N/A> | <Primary Surfaces> | <Exit Signal> |
```

## Item Authoring Rules

- `Workstream` should be a short noun phrase, not a narrative paragraph.
- `Depends On` should use item IDs, not prose-only references.
- `Evidence` should prefer stable IDs over file names alone.
- `Primary Surfaces` should name the files or folders expected to change.
- `Exit Signal` must describe a mechanically checkable completion state.

## Minimum Workflow

1. Open or update the evidence document with stable IDs.
2. Add or update the task-board row.
3. Apply the governance change.
4. Record the applied change in `REPO_GOVERNANCE_CHANGELOG.md`.
5. Run `just gov-check`.
