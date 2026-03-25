# ROLE_WORKTREES (Local Worktree Policy)

This document defines the local worktree/branch policy for role-governed sessions.

This policy is intentionally drive-agnostic: use a dedicated workspace root and keep all worktrees under a single `Handshake Worktrees` directory.

## Recommended Layout (Drive-Agnostic)

Define:
- `<HANDSHAKE_ROOT>`: your chosen workspace root (example: `/workspace/handshake`)
- `<HANDSHAKE_WORKTREES>`: `<HANDSHAKE_ROOT>/Handshake Worktrees`

Recommended structure:

```text
<HANDSHAKE_ROOT>/
  Handshake Worktrees/
    handshake_main/        # main repo checkout (branch: main)
    wt-ilja/               # Operator role worktree (branch: user_ilja)
    wt-gov-kernel/         # Governance kernel worktree (branch: gov_kernel)
    wtc-.../               # Coder WP worktrees (branch: feat/WP-...)
    # WP Validator operates from the coder worktree (no separate worktree) [CX-212D]
    # Integration Validator operates from handshake_main on branch main [CX-212D]
```

Preferred session host:
- Prefer the VS Code session bridge to host repo-governed Coder, WP Validator, and Integration Validator terminals inside VS Code integrated terminals.
- Keep one dedicated VS Code terminal tab for `just operator-monitor` so the Operator can watch active WPs, heartbeats, and packet-scoped communications without using many floating terminal windows.
- Do not rely on ambient editor defaults for model choice or reasoning strength. New repo-governed launchers explicitly target `gpt-5.4` primary, `gpt-5.2` fallback, and `model_reasoning_effort=xhigh`.
- Launch requests are append-only in the external repo-governance `SESSION_LAUNCH_REQUESTS.jsonl` ledger; current launch state is projected in the matching external `ROLE_SESSION_REGISTRY.json` file.
- CLI escalation windows are allowed only after 2 plugin failures/timeouts for the same role/WP session, unless the Operator explicitly waives the plugin-first path.

If you are an AI assistant operating in this repo:
- You MUST read this file during session start (Pre-Flight) for your assigned role.
- You MUST verify you are operating from the correct worktree directory and branch for your role before any repo changes.
- If the required worktree/branch does not exist, you MUST STOP and request the Orchestrator/Operator to create it (see "Creation commands").
- IMPORTANT: Codex [CX-108] blocks rewrite/hide operations such as `git stash`, `git checkout`, `git switch`, `git merge`, `git rebase`, `git reset`, and `git clean` unless explicitly authorized in the same turn.
- Exception (WP auto-continue): when the Orchestrator has already recorded a PASS signature gate for a specific WP and the next deterministic step is `just worktree-add WP-{ID}`, `just orchestrator-worktree-and-packet WP-{ID}`, or `just orchestrator-prepare-and-packet WP-{ID}`, the Orchestrator MUST create that missing WP worktree/branch automatically. Do not bounce that routine post-signature setup back to the Operator for a second approval.
- `main` is the canonical integrated branch. `user_ilja` and `gov_kernel` on GitHub are backup branches and may diverge from `main`.
- Permanent non-main worktrees (`wt-ilja`, `wtc-*`) inherit product code and root-level LLM files from local `main`. Their matching GitHub branches are safety copies, not the refresh source for that base.
- Before destructive or state-hiding local git actions on a role/user/WP branch, push the current committed state to the matching GitHub backup branch.
- For WPs, the matching GitHub backup branch should be treated as the phase-boundary recovery branch, not just a pre-destruction safety sink.
- Minimum WP recovery milestones to preserve remotely are:
  - signed packet/refinement checkpoint
  - docs-only bootstrap claim checkpoint
  - docs-only skeleton checkpoint
  - skeleton approval checkpoint before implementation resumes
- Before deleting local branches/worktrees or performing broad topology cleanup, create an immutable out-of-repo snapshot with `just backup-snapshot`.
- Permanent protected branches/worktrees that must never be deleted by Codex: `main`, `user_ilja`, `gov_kernel`, `wt-ilja`, `wt-gov-kernel`.
- Use `.GOV/roles_shared/records/GIT_TOPOLOGY_REGISTRY.md` + `.GOV/roles_shared/docs/REPO_RESILIENCE.md` as the deterministic reference for the permanent checkout layout and backup commands.

## Role Worktrees (Default)

| Role | Worktree directory | Branch | GitHub backup branch |
| --- | --- | --- |
| OPERATOR (human) | `<HANDSHAKE_WORKTREES>/wt-ilja` | `user_ilja` | `origin/user_ilja` |
| ORCHESTRATOR / GOV_KERNEL | `<HANDSHAKE_WORKTREES>/wt-gov-kernel` | `gov_kernel` | `origin/gov_kernel` |
| CODER (agent) | WP-assigned worktree only (no default) | WP branch only (no default) | matching WP backup branch on GitHub |

Notes:
- CODER agents MUST work only in the WP-assigned worktree/branch created and recorded by the Orchestrator. They must not "pick" a worktree.
- WP Validator sessions operate from the coder worktree (`wtc-*` on `feat/WP-*`), diffs against `main` [CX-212D].
- Integration Validator sessions operate from `handshake_main` on branch `main` [CX-212D].
- WP Validator and Integration Validator local lanes do not mint separate GitHub WP backup branches. Coder, WP Validator, and Integration Validator reuse the single packet-declared WP backup branch on GitHub.
- WP assignment is recorded in `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json` as a `PREPARE` entry (via `just record-prepare ...`) with `branch` and `worktree_dir`.
- Orchestrator governance work uses `wt-gov-kernel` on `gov_kernel`. Integration Validator works from `handshake_main` on `main`.
- Permanent role/user branches are backup branches on GitHub. Their purpose is recoverability, not integration. They may be ahead of, equal to, or behind `main`.
- Refreshing a permanent non-main worktree has two distinct paths:
  - `just sync-all-role-worktrees` refreshes the local `main` branch across the permanent worktrees when all are clean.
  - `just reseed-permanent-worktree-from-main <worktree_id> "<approval>"` resets the checked-out permanent role/user branch to local `main` after a safety push + immutable snapshot, then repairs the `.GOV/` junction.
- A WP backup branch is temporary. Its URL may stop resolving after Operator-approved cleanup and that later 404 must not become a governance failure.

## Parallel Ownership Model (Current Law)

This repo is designed for parallel governed execution, but the parallel model is not "everyone edits everywhere."

### Ownership lanes

- `ORCHESTRATOR`: one governed coordinator lane for the repo, running from `wt-gov-kernel` on `gov_kernel`
- `CODER`: one governed product-execution lane per active WP, running only from the WP-assigned worktree/branch
- `WP_VALIDATOR`: one governed advisory validator lane per active WP, operating from the coder worktree for that WP
- `INTEGRATION_VALIDATOR`: one governed final-validation lane running from `handshake_main` on `main`

### Allowed parallel states

- Multiple active WPs may run in parallel if each WP has its own recorded worktree/branch mapping.
- Multiple coder sessions may run in parallel only when they are on different WPs with different worktrees.
- Multiple `WP_VALIDATOR` lanes may run in parallel only when they are attached to different WPs.
- `INTEGRATION_VALIDATOR` may run in parallel with active coder and `WP_VALIDATOR` lanes because it does not own a WP-specific worktree.

### Blocked or invalid states

- Two active WPs sharing the same WP-specific worktree.
- Detached or convenience WP-adjacent check/postwork/validator clones that are not the packet-declared coder worktree.
- Product edits from `wt-gov-kernel` or `wt-ilja`.
- A separate validator-only WP worktree for ordinary `WP_VALIDATOR` work.
- Treating `WP_VALIDATOR` as final merge authority for an orchestrator-managed WP.
- Concurrent steering for the same governed role/WP session.

### Same-WP lane rules

- Current governed session-control law is one governed role/WP lane per role. For one WP, the ordinary governed shape is:
  - one `CODER` lane
  - one `WP_VALIDATOR` lane
  - one `INTEGRATION_VALIDATOR` lane when final orchestrator-managed validation is needed
- If extra same-role analysis sessions are opened outside that ordinary shape, they are diagnostic only unless the Orchestrator explicitly closes/rebinds the governed lane. They must not act like authoritative replacement lanes by appending authoritative receipts, clearing another session's notifications, or presenting final authority on their own.

### File-lock rule

- Treat each active WP's `IN_SCOPE_PATHS` as the exclusive product file-lock set for that WP.
- Parallel work is valid only when those scope surfaces stay disjoint or when the packet/workflow explicitly coordinates the overlap.

## Verification Commands (run at session start)

- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git status -sb`
- `git worktree list`

Why this gate exists (CX-WT-001):
- Prevent work in the wrong directory/branch (especially accidental `main` or role-branch edits).
- Enforce WP isolation via dedicated worktrees/branches (no shared working trees across active WPs).
- Provide a verifiable snapshot for Operator/Validator using `.GOV/roles_shared/docs/ROLE_WORKTREES.md` + `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json` (`PREPARE` entries).

Next actions (CX-WT-001):
- If correct: proceed with the next protocol step (BOOTSTRAP / packet work).
- If incorrect/uncertain: STOP and ask Orchestrator/Operator to provide/create the correct worktree/branch (and record `PREPARE` in `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json` for WP work).

## Creation Commands

Role worktrees and manual repair flows require explicit authorization in the same turn when they rely on Codex [CX-108] blocked git operations.

From the main repo working tree (`<HANDSHAKE_WORKTREES>/handshake_main`):

- Ensure the permanent GitHub backup branches exist:
  - `just ensure-permanent-backup-branches`
- Sync the deterministic topology registry:
  - `just topology-registry-sync`
- Create an immutable out-of-repo snapshot:
  - `just backup-snapshot`

- Create OPERATOR worktree:
  - `git worktree add -b user_ilja ../wt-ilja main`
- Ensure ORCHESTRATOR governance kernel worktree exists:
  - If `origin/gov_kernel` exists:
    - `git worktree add -b gov_kernel ../wt-gov-kernel origin/gov_kernel`
  - Otherwise:
    - `git worktree add -b gov_kernel ../wt-gov-kernel main`
- After creating a permanent role/user branch locally, push it once to the matching GitHub backup branch and set upstream:
  - `git -C ../wt-gov-kernel push -u origin gov_kernel`
  - `git -C ../wt-ilja push -u origin user_ilja`

WP worktrees (Orchestrator action, not Coder):
- Post-signature default: after `just record-signature WP-{ID} ...` returns PASS, create the WP worktree/branch automatically. This is deterministic setup, not a second approval boundary.
- If the signature bundle already captured the workflow lane + execution owner, prefer `just orchestrator-prepare-and-packet WP-{ID}` as the default helper.
- If the signature was recorded without the full workflow tuple (legacy recovery), the only remaining operator decision is the missing workflow lane and/or coder lane; do not ask again for branch/worktree authorization.
- Create a WP worktree/branch:
  - `just worktree-add WP-{ID}`
- Validator worktrees [CX-212D]: WP Validator uses the coder worktree; Integration Validator uses handshake_main. No separate worktree creation needed.
- Launch the repo-governed CLI sessions:
  - `just launch-coder-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
  - `just launch-wp-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
  - `just launch-integration-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- View current launch state:
  - `just session-registry-status [WP-{ID}]`
- Create/preserve the matching GitHub backup branch for the WP when sync is authorized for the activation turn:
  - `just backup-push feat/WP-{ID} feat/WP-{ID}`
- Keep reusing that same WP backup branch at each recovery milestone so a clean restart can begin from the latest lawful WP phase boundary instead of a dirty local tree.
- Before deleting a WP worktree or WP backup branch after approval:
  - `just backup-snapshot`
- Record the execution owner (writes `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json`):
  - Prefer repo-relative `worktree_dir` values (example: `../wt-WP-{ID}`) to avoid drive-specific paths and quoting issues.
  - `just record-prepare WP-{ID} {Coder-A..Coder-Z} [branch] [worktree_dir]`
