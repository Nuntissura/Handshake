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
    wt-orchestrator/       # Orchestrator role worktree (branch: role_orchestrator)
    wt-validator/          # Validator role worktree (branch: role_validator)
    wt-gov-kernel/         # Governance kernel worktree (branch: gov_kernel)
    wt-WP-.../             # Coder WP worktrees (branch: feat/WP-...)
    wt-WPV-WP-.../         # WP Validator worktrees (branch: validate/WP-...)
    wt-INTV-WP-.../        # Integration Validator worktrees (branch: integrate/WP-...)
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
- `main` is the canonical integrated branch. `user_ilja`, `role_orchestrator`, and `role_validator` on GitHub are backup branches and may diverge from `main`.
- Before destructive or state-hiding local git actions on a role/user/WP branch, push the current committed state to the matching GitHub backup branch.
- For WPs, the matching GitHub backup branch should be treated as the phase-boundary recovery branch, not just a pre-destruction safety sink.
- Minimum WP recovery milestones to preserve remotely are:
  - signed packet/refinement checkpoint
  - docs-only bootstrap claim checkpoint
  - docs-only skeleton checkpoint
  - skeleton approval checkpoint before implementation resumes
- Before deleting local branches/worktrees or performing broad topology cleanup, create an immutable out-of-repo snapshot with `just backup-snapshot`.
- Permanent protected branches/worktrees that must never be deleted by Codex: `main`, `user_ilja`, `role_orchestrator`, `role_validator`, `gov_kernel`, `wt-ilja`, `wt-orchestrator`, `wt-validator`, `wt-gov-kernel`.
- Use `.GOV/roles_shared/records/GIT_TOPOLOGY_REGISTRY.md` + `.GOV/roles_shared/docs/REPO_RESILIENCE.md` as the deterministic reference for the permanent checkout layout and backup commands.

## Role Worktrees (Default)

| Role | Worktree directory | Branch | GitHub backup branch |
| --- | --- | --- |
| OPERATOR (human) | `<HANDSHAKE_WORKTREES>/wt-ilja` | `user_ilja` | `origin/user_ilja` |
| ORCHESTRATOR | `<HANDSHAKE_WORKTREES>/wt-orchestrator` | `role_orchestrator` | `origin/role_orchestrator` |
| VALIDATOR | `<HANDSHAKE_WORKTREES>/wt-validator` | `role_validator` | `origin/role_validator` |
| GOV_KERNEL | `<HANDSHAKE_WORKTREES>/wt-gov-kernel` | `gov_kernel` | `origin/gov_kernel` |
| CODER (agent) | WP-assigned worktree only (no default) | WP branch only (no default) | matching WP backup branch on GitHub |

Notes:
- CODER agents MUST work only in the WP-assigned worktree/branch created and recorded by the Orchestrator. They must not "pick" a worktree.
- WP Validator sessions SHOULD use `validate/WP-...` + `../wt-WPV-WP-...`.
- Integration Validator sessions SHOULD use `integrate/WP-...` + `../wt-INTV-WP-...`.
- WP Validator and Integration Validator local lanes do not mint separate GitHub WP backup branches. Coder, WP Validator, and Integration Validator reuse the single packet-declared WP backup branch on GitHub.
- WP assignment is recorded in `.GOV/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json` as a `PREPARE` entry (via `just record-prepare ...`) with `branch` and `worktree_dir`.
- ORCHESTRATOR/VALIDATOR role work (governance/validation work outside a specific WP worktree) uses the dedicated role worktrees above.
- Permanent role/user branches are backup branches on GitHub. Their purpose is recoverability, not integration. They may be ahead of, equal to, or behind `main`.
- A WP backup branch is temporary. Its URL may stop resolving after Operator-approved cleanup and that later 404 must not become a governance failure.

## Verification Commands (run at session start)

- `pwd`
- `git rev-parse --show-toplevel`
- `git rev-parse --abbrev-ref HEAD`
- `git status -sb`
- `git worktree list`

Why this gate exists (CX-WT-001):
- Prevent work in the wrong directory/branch (especially accidental `main` or role-branch edits).
- Enforce WP isolation via dedicated worktrees/branches (no shared working trees across active WPs).
- Provide a verifiable snapshot for Operator/Validator using `.GOV/roles_shared/docs/ROLE_WORKTREES.md` + `.GOV/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json` (`PREPARE` entries).

Next actions (CX-WT-001):
- If correct: proceed with the next protocol step (BOOTSTRAP / packet work).
- If incorrect/uncertain: STOP and ask Orchestrator/Operator to provide/create the correct worktree/branch (and record `PREPARE` in `.GOV/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json` for WP work).

## Creation Commands

Role worktrees and manual repair flows require explicit authorization in the same turn when they rely on Codex [CX-108] blocked git operations.

From the main repo working tree (`<HANDSHAKE_WORKTREES>/handshake_main`):

- Ensure the permanent GitHub backup branches exist:
  - `just ensure-permanent-backup-branches`
- Sync the deterministic topology registry:
  - `just topology-registry-sync`
- Create an immutable out-of-repo snapshot:
  - `just backup-snapshot`

- Create ORCHESTRATOR worktree:
  - If `origin/role_orchestrator` exists:
    - `git worktree add -b role_orchestrator ../wt-orchestrator origin/role_orchestrator`
  - Legacy fallback (if `origin/user_orchestrator` exists):
    - `git worktree add -b role_orchestrator ../wt-orchestrator origin/user_orchestrator`
  - Otherwise:
    - `git worktree add -b role_orchestrator ../wt-orchestrator main`
- Create VALIDATOR worktree:
  - `git worktree add -b role_validator ../wt-validator main`
- Create OPERATOR worktree:
  - `git worktree add -b user_ilja ../wt-ilja main`
- After creating a permanent role/user branch locally, push it once to the matching GitHub backup branch and set upstream:
  - `git -C ../wt-orchestrator push -u origin role_orchestrator`
  - `git -C ../wt-validator push -u origin role_validator`
  - `git -C ../wt-ilja push -u origin user_ilja`

WP worktrees (Orchestrator action, not Coder):
- Post-signature default: after `just record-signature WP-{ID} ...` returns PASS, create the WP worktree/branch automatically. This is deterministic setup, not a second approval boundary.
- If the signature bundle already captured the workflow lane + execution owner, prefer `just orchestrator-prepare-and-packet WP-{ID}` as the default helper.
- If the signature was recorded without the full workflow tuple (legacy recovery), the only remaining operator decision is the missing workflow lane and/or coder lane; do not ask again for branch/worktree authorization.
- Create a WP worktree/branch:
  - `just worktree-add WP-{ID}`
- Create the validator worktrees/branches for the same WP:
  - `just wp-validator-worktree-add WP-{ID}`
  - `just integration-validator-worktree-add WP-{ID}`
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
- Record the execution owner (writes `.GOV/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json`):
  - Prefer repo-relative `worktree_dir` values (example: `../wt-WP-{ID}`) to avoid drive-specific paths and quoting issues.
  - `just record-prepare WP-{ID} {Coder-A..Coder-Z} [branch] [worktree_dir]`
