# ROLE_WORKTREES (Local Worktree Policy)

This document defines the local worktree/branch policy for role-governed sessions.

This policy is intentionally drive-agnostic: use a dedicated workspace root and keep all worktrees under a single "Handshake Worktrees" directory.

## Recommended Layout (Drive-Agnostic)

Define:
- `<HANDSHAKE_ROOT>`: your chosen workspace root (example: `P:\Handshake`)
- `<HANDSHAKE_WORKTREES>`: `<HANDSHAKE_ROOT>\Handshake Worktrees`

Recommended structure:

```text
<HANDSHAKE_ROOT>\
  Handshake Worktrees\
    handshake_main\        # main repo checkout (branch: main)
    wt-ilja\               # Operator role worktree (branch: user_ilja)
    wt-orchestrator\       # Orchestrator role worktree (branch: role_orchestrator)
    wt-validator\          # Validator role worktree (branch: role_validator)
    wt-WP-...\             # WP worktrees (branch: feat/WP-...)
```

If you are an AI assistant operating in this repo:
- You MUST read this file during session start (Pre-Flight) for your assigned role.
- You MUST verify you are operating from the correct worktree directory and branch for your role before any repo changes.
- If the required worktree/branch does not exist, you MUST STOP and request the Orchestrator/Operator to create it (see "Creation commands").
- IMPORTANT: Codex [CX-108] blocks rewrite/hide operations such as `git stash`, `git checkout`, `git switch`, `git merge`, `git rebase`, `git reset`, and `git clean` unless explicitly authorized in the same turn.
- Exception (WP auto-continue): when the Orchestrator has already recorded a PASS signature gate for a specific WP and the next deterministic step is `just worktree-add WP-{ID}`, `just orchestrator-worktree-and-packet WP-{ID}`, or `just orchestrator-prepare-and-packet WP-{ID} {Orchestrator-Agentic|Coder-A|Coder-B}`, the Orchestrator MUST create that missing WP worktree/branch automatically. Do not bounce that routine post-signature setup back to the Operator for a second approval.

## Role Worktrees (Default)

| Role | Worktree directory | Branch |
| --- | --- | --- |
| OPERATOR (human) | `<HANDSHAKE_WORKTREES>\wt-ilja` | `user_ilja` |
| ORCHESTRATOR | `<HANDSHAKE_WORKTREES>\wt-orchestrator` | `role_orchestrator` |
| VALIDATOR | `<HANDSHAKE_WORKTREES>\wt-validator` | `role_validator` |
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

## Creation Commands

Role worktrees and manual repair flows require explicit authorization in the same turn when they rely on Codex [CX-108] blocked git operations.

From the main repo working tree (`<HANDSHAKE_WORKTREES>\handshake_main`):

- Create ORCHESTRATOR worktree:
  - If `origin/role_orchestrator` exists:
    - `git worktree add -b role_orchestrator ..\wt-orchestrator origin/role_orchestrator`
  - Legacy fallback (if `origin/user_orchestrator` exists):
    - `git worktree add -b role_orchestrator ..\wt-orchestrator origin/user_orchestrator`
  - Otherwise:
    - `git worktree add -b role_orchestrator ..\wt-orchestrator main`
- Create VALIDATOR worktree:
  - `git worktree add -b role_validator ..\wt-validator main`
- Create OPERATOR worktree:
  - `git worktree add -b user_ilja ..\wt-ilja main`

WP worktrees (Orchestrator action, not Coder):
- Post-signature default: after `just record-signature WP-{ID} ...` returns PASS, create the WP worktree/branch automatically. This is deterministic setup, not a second approval boundary.
- If the signature bundle already captured the execution lane, prefer `just orchestrator-prepare-and-packet WP-{ID} {Orchestrator-Agentic|Coder-A|Coder-B}` as the default helper.
- If the signature was recorded without an execution lane (legacy recovery), the only remaining operator decision is the execution lane choice; do not ask again for branch/worktree authorization.
- Create a WP worktree/branch:
  - `just worktree-add WP-{ID}`
- Record the execution owner (writes `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json`):
  - Prefer repo-relative `worktree_dir` values (example: `../wt-WP-{ID}`) to avoid drive-specific paths and quoting issues.
  - `just record-prepare WP-{ID} {Orchestrator-Agentic|Coder-A|Coder-B} [branch] [worktree_dir]`
