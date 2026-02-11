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
