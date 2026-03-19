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
- Permanent protected worktrees on disk must never be deleted by Codex: `handshake_main`, `wt-ilja`, `wt-orchestrator`, `wt-validator`.
- `user_ilja`, `role_orchestrator`, and `role_validator` on GitHub are backup branches, not integration branches. They may diverge from `main`.
- Before any destructive or state-hiding local git action (`git merge`, `git switch`, `git checkout`, `git reset`, `git clean`, local branch deletion, worktree deletion), first push the current committed branch state to its matching GitHub backup branch.
- Before deleting local branches/worktrees or doing broad topology cleanup, create an immutable out-of-repo snapshot with `just backup-snapshot`.
- Role startup now includes `just backup-status` so Codex can see whether local/NAS backup roots are configured and whether recent immutable snapshots exist. Treat that visibility as safety context, not as authorization to skip destructive-op approvals.
- Only the Operator may approve fast-forwarding GitHub backup branches, deleting GitHub branches, deleting local branches, or deleting worktrees. If cleanup is requested broadly, stop, list the exact actions + exact targets, and ask for approval on that presented list.
- For clearer language going forward, use these exact terms:
  - `local branch`: a branch ref in a local checkout on disk, for example `main` or `role_validator`
  - `remote branch` or `GitHub branch`: a branch at `origin/<name>`, for example `origin/main`
  - `worktree`: a directory on disk, for example `handshake_main` or `wt-validator`
  - `canonical branch`: always `main`
  - `backup branch`: a non-canonical GitHub branch used as a safety copy, for example `origin/role_validator`
- Broad requests like "clean up branches" or "sync everything" are insufficient. Present a deterministic list of exact actions + exact targets first. For that most recently presented list, the only valid approval replies are `approved` or `proceed`. If the action/target list changes, ask again.
- Use `just enumerate-cleanup-targets` to print current exact targets and proposed cleanup actions.
- Use `just delete-local-worktree <worktree_id> "<approval>"` for assistant-driven worktree deletion, with `<approval>` set to `approved` or `proceed` after the action/target list has been presented. Never delete worktree directories directly with `rm`, `del`, or `Remove-Item`.
- If `git worktree remove` fails, STOP. Do not switch to manual filesystem cleanup inside the shared worktree root.
- Use `just sync-all-role-worktrees` to fast-forward the permanent local clones safely when all are clean.

### Governance-only work (no WP required)
- Governance/workflow/tooling-only maintenance does NOT require a Work Packet or USER_SIGNATURE when the planned diff is strictly limited to governance surface files:
  - `/.GOV/**`
  - `/.github/**`
  - `/justfile`
  - `/.GOV/codex/Handshake_Codex_v1.4.md`
  - `/AGENTS.md`
- Hard rule: if any Handshake product code is touched (`/src/`, `/app/`, `/tests/`), STOP and require a WP.
- Minimum verification for governance-only changes: `just gov-check`.

### Safety commit gate (prevents packet/refinement loss)
- After creating a WP task packet + refinement and obtaining `USER_SIGNATURE`, create a checkpoint commit on the WP branch that includes:
  - `.GOV/task_packets/WP-{ID}.md`
  - `.GOV/refinements/WP-{ID}.md`

### WP communication artifacts
- Official packets may define `WP_COMMUNICATION_DIR` under the external repo-governance runtime root (default repo-relative from a worktree: `../../Handshake Runtime/repo-governance/roles_shared/WP_COMMUNICATIONS/WP-{ID}/`; overridable via `HANDSHAKE_GOV_RUNTIME_ROOT` or `HANDSHAKE_RUNTIME_ROOT`).
- These files are governance-only collaboration helpers:
  - `THREAD.md` for append-only freeform discussion
  - `RUNTIME_STATUS.json` for liveness, validator-trigger, waiting-state, next-actor watch state, and bounded loop counters
  - `RECEIPTS.jsonl` for deterministic assignment, status, heartbeat, steering, repair, validation, and handoff receipts
- The task packet remains authoritative for scope, packet status, PREPARE assignment, acceptance, and verdict.
- If packet and communication artifacts disagree, the packet wins.
- These richer artifacts apply to both `MANUAL_RELAY` and `ORCHESTRATOR_MANAGED` workflow lanes.
- The packet-declared `WP_COMMUNICATION_DIR` is the only communication authority for that WP. Do not improvise role-local inboxes.
- When available, prefer VS Code integrated terminals as the host for multi-session role work. Use `just operator-monitor` as the overview surface instead of treating role-local terminal buffers as authority.
- Repo-governed multi-session launch is plugin-first: queue VS Code bridge requests through the external repo-governance launch queue (default repo-relative: `../../Handshake Runtime/repo-governance/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`), project current state in the external session registry (`../../Handshake Runtime/repo-governance/roles_shared/ROLE_SESSION_REGISTRY.json`), and keep heartbeat as fallback only.
- Only the Orchestrator may start repo-governed Coder, WP Validator, and Integration Validator sessions. Coder/Validator sessions may resume work, but they do not self-start a fresh repo-governed session.
- CLI escalation windows are allowed only after the same role/WP session records 2 plugin failures or timeouts.
- For newly created stubs/packets, repo-governed CLI session policy is explicit: primary model `gpt-5.4`, fallback `gpt-5.2`, reasoning strength `EXTRA_HIGH`, launcher config `model_reasoning_effort=xhigh`.
- Do not rely on whatever model/reasoning defaults happen to be active in an editor or local CLI profile. Launch or claim the session explicitly.
- Repo policy for new repo-governed sessions disallows Codex model aliases in packet claim fields; the CLI tool may still be `codex`.
- Freeform packet-scoped messages should be appended with `just wp-thread-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> "<message>" [target]`; this writes both the thread entry and a paired structured receipt.
- For new WP communication writes, validator sessions must identify themselves as `WP_VALIDATOR` or `INTEGRATION_VALIDATOR` in `THREAD.md`, `RUNTIME_STATUS.json`, and `RECEIPTS.jsonl`. Legacy generic `VALIDATOR` entries are compatibility-only and should not be emitted by new governed sessions.
- When useful for parallel governed sessions, communication receipts and thread entries may carry structured routing metadata such as `target_role`, `target_session`, `correlation_id`, `requires_ack`, `ack_for`, `spec_anchor`, and `packet_row_ref`, and runtime status may carry `next_expected_session`, `waiting_on_session`, and an `open_review_items` projection for unresolved coder/validator exchanges.
- Authority split for semi-autonomous work:
  - Orchestrator = workflow authority
  - WP Validator = advisory technical reviewer
  - Integration Validator = final technical and merge authority

### Current role execution policy
- Orchestrator is non-agentic and single-session, but may coordinate and launch multiple external CLI sessions.
- Validator duties are non-agentic, but repo governance may run multiple validator CLI sessions concurrently when they are scoped as WP Validator and Integration Validator sessions.
- Only the Primary Coder may use coder sub-agents, and only when the packet explicitly records operator approval.
- Shared launch/watch contract: `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`.
</INSTRUCTIONS>
