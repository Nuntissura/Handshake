<INSTRUCTIONS>
## Handshake Repo Guardrails (HARD RULES)

### No destructive cleanup
- Do NOT run destructive commands that can delete/overwrite work (especially untracked files) unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If any cleanup/reset is requested, make it reversible first: `git stash push -u -m "SAFETY: before <operation>"`, then show what would be deleted (`git clean -nd`) and wait for explicit approval.

### Branching & concurrency
- Default: one WP = one feature branch (e.g., `feat/WP-{ID}`).
- When more than one coder/WP is active concurrently, use `git worktree` per active WP (separate working directories). Do NOT share a single working tree across concurrent WPs.
- `main` is the only canonical integrated branch on disk and on GitHub.
- Permanent protected role/user branches and their corresponding permanent worktrees must never be deleted by Codex: `main`, `user_ilja`, `role_orchestrator`, `role_validator`.
- `user_ilja`, `role_orchestrator`, and `role_validator` on GitHub are backup branches, not integration branches. They may diverge from `main`.
- Before any destructive or state-hiding local git action (`git merge`, `git switch`, `git checkout`, `git reset`, `git clean`, local branch deletion, worktree deletion), first push the current committed branch state to its matching GitHub backup branch.
- Only the Operator may approve fast-forwarding GitHub backup branches, deleting GitHub branches, deleting local branches, or deleting worktrees. If cleanup is requested broadly, stop and ask for an approval command naming the exact targets.

### Governance-only work (no WP required)
- Governance/workflow/tooling-only maintenance does NOT require a Work Packet or USER_SIGNATURE when the planned diff is strictly limited to governance surface files:
  - `/.GOV/**`
  - `/.github/**`
  - `/justfile`
  - `/Handshake Codex v1.4.md`
  - `/AGENTS.md`
- Hard rule: if any Handshake product code is touched (`/src/`, `/app/`, `/tests/`), STOP and require a WP.
- Minimum verification for governance-only changes: `just gov-check`.

### Safety commit gate (prevents packet/refinement loss)
- After creating a WP task packet + refinement and obtaining `USER_SIGNATURE`, create a checkpoint commit on the WP branch that includes:
  - `.GOV/task_packets/WP-{ID}.md`
  - `.GOV/refinements/WP-{ID}.md`
</INSTRUCTIONS>
