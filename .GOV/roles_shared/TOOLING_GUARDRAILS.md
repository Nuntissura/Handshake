# TOOLING_GUARDRAILS

Hard rules live in Codex, AGENTS, role protocols, and checks. This file is append-only shared tooling memory for recurring repo bad habits and recurring system/tool limitations.

Rules:
- Append only. Add new entries; do not rewrite or delete old ones.
- Add only recurring patterns, not one-off task incidents.
- Keep each entry short.
- Use `Do`, `Don't`, `Why`, and `Context`.
- Prefer patterns that are likely to recur across sessions, roles, WPs, or models.
- Repeated low-severity friction belongs here if it wastes time often enough.

### TG-001
- Do:
  - Use `just delete-local-worktree <worktree_id> "<approval>"` for local worktree deletion.
- Don't:
  - Do not use `Remove-Item`, `rm`, `del`, or manual fallback delete on repo/worktree paths.
- Why:
  - Malformed targets or bad git state can widen cleanup into repo-loss or disk-loss.
- Context:
  - If `git worktree remove` fails, stop and repair tooling/state instead of deleting by hand.

### TG-002
- Do:
  - Keep committed governance paths repo-relative or environment-derived.
- Don't:
  - Do not hardcode drive letters, UNC roots, or machine-local home paths.
- Why:
  - Governance must survive different machines, drives, and worktree roots.
- Context:
  - Resolve from repo root or script location, not from the current shell drive.

### TG-003
- Do:
  - Use explicit tool `workdir` or `git -C "<path>" ...` for worktree-scoped commands.
- Don't:
  - Do not rely on `cd` or `Set-Location` persisting across automated tool calls.
- Why:
  - Automation shells are often isolated.
- Context:
  - Wrong worktree reads or writes corrupt evidence and validation state.

### TG-004
- Do:
  - Retry with a repo-relative patch path first.
- Don't:
  - Do not assume patch content is wrong when the header path is the failing part.
- Why:
  - Windows path handling can fail before content is applied.
- Context:
  - Recurring in Windows-hosted WP creation/patch flows: valid patch content, failing long header path.

### Append Template
- Do:
  - <short required action>
- Don't:
  - <short forbidden action>
- Why:
  - <short reason>
- Context:
  - <short tool failure or state pattern>
