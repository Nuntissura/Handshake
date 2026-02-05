# ROLE_WORKTREES (Local Worktree Policy)

This document defines the local worktree/branch policy used on Ilja's machine for role-governed sessions.

If you are an AI assistant operating in this repo:
- You MUST read this file during session start (Pre-Flight) for your assigned role.
- You MUST verify you are operating from the correct worktree directory and branch for your role before any repo changes.
- If the required worktree/branch does not exist, you MUST STOP and request the Orchestrator/Operator to create it (see "Creation commands").
- IMPORTANT: Creating worktrees/branches uses `git` operations that are blocked unless the user explicitly authorizes them in the same turn (Codex [CX-108]). If not authorized, STOP and request authorization.

## Role Worktrees (Ilja)

| Role | Worktree directory | Branch |
| --- | --- | --- |
| OPERATOR (human) | `D:\Projects\LLM projects\wt-ilja` | `user_ilja` |
| ORCHESTRATOR | `D:\Projects\LLM projects\wt-orchestrator` | `user_orchestrator` |
| VALIDATOR | `D:\Projects\LLM projects\wt-validator` | `user_validator` |
| CODER (agent) | WP-assigned worktree only (no default) | WP branch only (no default) |

Notes:
- CODER agents MUST work only in the WP-assigned worktree/branch created and recorded by the Orchestrator. They must not "pick" a worktree.
- WP assignment is recorded in `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` as a `PREPARE` entry (via `just record-prepare ...`) with `branch` and `worktree_dir`.
- ORCHESTRATOR/VALIDATOR role work (governance/validation work outside a specific WP worktree) uses the dedicated role worktrees above.

## Verification Commands (run at session start)

- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git status -sb`
- `git worktree list`

Why this gate exists (CX-WT-001):
- Prevent work in the wrong directory/branch (especially accidental `main` or role-branch edits).
- Enforce WP isolation via dedicated worktrees/branches (no shared working trees across active WPs).
- Provide a verifiable snapshot for Operator/Validator using `.GOV/roles_shared/ROLE_WORKTREES.md` + `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` (`PREPARE` entries).

Next actions (CX-WT-001):
- If correct: proceed with the next protocol step (BOOTSTRAP / packet work).
- If incorrect/uncertain: STOP and ask Orchestrator/Operator to provide/create the correct worktree/branch (and record `PREPARE` in `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` for WP work).

## Creation Commands (only if explicitly authorized in the same turn)

From the main repo working tree:

- Create ORCHESTRATOR worktree:
  - `git worktree add -b user_orchestrator "D:\Projects\LLM projects\wt-orchestrator" main`
- Create VALIDATOR worktree:
  - `git worktree add -b user_validator "D:\Projects\LLM projects\wt-validator" main`

WP worktrees (Orchestrator action, not Coder):
- Create a WP worktree/branch:
  - `just worktree-add WP-{ID}`
- Record the coder assignment (writes `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`):
  - `just record-prepare WP-{ID} {Coder-A|Coder-B} [branch] [worktree_dir]`

