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
- Permanent protected role/user branches and their corresponding permanent worktrees must never be deleted by Codex: `main`, `user_ilja`, `gov_kernel`.
- Permanent protected worktrees on disk must never be deleted by Codex: `handshake_main`, `wt-ilja`, `wt-gov-kernel`.
- `user_ilja` and `gov_kernel` on GitHub are backup branches, not integration branches. They may diverge from `main`.
- Permanent non-main worktrees (`wt-ilja`, `wtc-*`) get their non-`.GOV/` base from local `main`. Their matching GitHub branches are safety copies, not the source of product-code or root-file refresh.
- Before any destructive or state-hiding local git action (`git merge`, `git switch`, `git checkout`, `git reset`, `git clean`, local branch deletion, worktree deletion), first push the current committed branch state to its matching GitHub backup branch.
- Before deleting local branches/worktrees or doing broad topology cleanup, create an immutable out-of-repo snapshot with `just backup-snapshot`.
- Role startup now includes `just backup-status` so Codex can see whether local/NAS backup roots are configured and whether recent immutable snapshots exist. Treat that visibility as safety context, not as authorization to skip destructive-op approvals.
- Only the Operator may approve fast-forwarding GitHub backup branches, deleting GitHub branches, deleting local branches, or deleting worktrees. If cleanup is requested broadly, stop, list the exact actions + exact targets, and ask for approval on that presented list.
- For clearer language going forward, use these exact terms:
  - `local branch`: a branch ref in a local checkout on disk, for example `main` or `gov_kernel`
  - `remote branch` or `GitHub branch`: a branch at `origin/<name>`, for example `origin/main`
  - `worktree`: a directory on disk, for example `handshake_main` or `wt-gov-kernel`
  - `canonical branch`: always `main`
  - `backup branch`: a non-canonical GitHub branch used as a safety copy, for example `origin/gov_kernel`
- Broad requests like "clean up branches" or "sync everything" are insufficient. Present a deterministic list of exact actions + exact targets first. For that most recently presented list, the only valid approval replies are `approved` or `proceed`. If the action/target list changes, ask again.
- Use `just enumerate-cleanup-targets` to print current exact targets and proposed cleanup actions.
- Use `just delete-local-worktree <worktree_id> "<approval>"` for assistant-driven worktree deletion, with `<approval>` set to `approved` or `proceed` after the action/target list has been presented. Never delete worktree directories directly with `rm`, `del`, or `Remove-Item`.
- **FORBIDDEN: `git worktree remove` (raw).** NEVER run `git worktree remove` directly. Non-main worktrees use a `.GOV/` directory junction pointing to `wt-gov-kernel/.GOV/`. Raw `git worktree remove` follows the junction and destroys the real governance files. The governance script (`delete-local-worktree.mjs`) detaches the junction before removal. Always use `just delete-local-worktree`. [CX-122]
- If `git worktree remove` or `just delete-local-worktree` fails, STOP. Do not switch to manual filesystem cleanup inside the shared worktree root.
- Use `just sync-all-role-worktrees` only to refresh the local `main` branch across the permanent worktrees when all are clean. It is not the helper for reseeding `wt-ilja` from `main`.
- Use `just reseed-permanent-worktree-from-main <worktree_id> "<approval>"` for governed refresh of the permanent Operator worktree from local `main`. This safety-pushes the matching backup branch, creates an immutable snapshot, resets the local role/user branch to local `main`, and repairs the `.GOV/` junction.
- Root-level repo control files inherited from `main`, currently `AGENTS.md` and the root `justfile`, are main-only authoring surfaces. Do not author or commit them from `wt-ilja`, `wt-gov-kernel`, or any `wtc-*` WP worktree. Exception: `wt-gov-kernel` may carry a governance-only kernel launcher `justfile`; that file is not the canonical root `justfile`.

### Governance-only work (no WP required)
- Governance/workflow/tooling-only maintenance does NOT require a Work Packet or USER_SIGNATURE when the planned diff is strictly limited to governance surface files:
  - `/.GOV/**`
  - `/.github/**`
  - `/justfile`
  - `/.GOV/codex/Handshake_Codex_v1.4.md`
  - `/AGENTS.md`
- Hard rule: if any Handshake product code is touched (`/src/`, `/app/`, `/tests/`), STOP and require a WP.
- Governance surface minimization rule:
  - Prefer extending an existing phase-owned command, role-owned surface, or primary debug artifact instead of creating a new public `just` recipe, standalone check, standalone script, or duplicate operator-facing doc path.
  - Target shape: one real public command per phase or authority boundary and one primary artifact/debug surface per phase or boundary.
  - Thin wrappers, compatibility aliases, and duplicate public helpers are governance debt, not neutral convenience.
  - For governance scripts and public recipes specifically, prefer one canonical public script per phase or authority boundary. If a new script would share the same owner, inputs, primary artifact/debug surface, and usual invocation path as an existing script, extend the existing script instead of adding a sibling.
  - Bias toward fewer larger canonical scripts over multiple small public wrappers that always travel together anyway.
  - Keep a separate public script only when authority owner, side-effect class, runtime/topology assumptions, primary debug artifact, or operator usefulness materially differs. Internal helper libs are still allowed; the real target is fewer public entrypoints.
  - If a new live governance surface is genuinely required, record why the existing surface is insufficient, who owns the new surface, what the primary debug artifact is, and whether any older surface is being retired or intentionally kept distinct.
  - Do not retire an old public governance surface until the replacement is confirmed as tracked and usable in the active topology.
- Build/test/tool outputs MUST live at the external sibling root `../Handshake Artifacts/` (subfolders: `handshake-cargo-target/`, `handshake-product/`, `handshake-test/`, `handshake-tool/`). Repo-local `target/` directories are governance violations.
- When old governance scripts/tests are retired during repo-governance cleanup, move them to an operator-designated external archive root outside the repo for safekeeping and posterity instead of hard-deleting them. Keep that archive location out of runtime assumptions; record the concrete path in the relevant audit/log for the cleanup wave.
- Operator-facing scope split rule:
  - Always separate `Handshake (Product)` from `Repo Governance` in chat.
  - `Handshake (Product)` includes product code, product tests, Master Spec requirements, and product WPs, even when the topic is governed actions, workflow law, or other product-governance contracts.
  - `Repo Governance` includes `/.GOV/**`, ACP/session/runtime ledgers, role protocols, governance task-board/changelog/audits, and root control-file maintenance.
  - If only one lane applies, still name both lanes and state `NONE` for the other lane.
  - Do not use `governance` alone for product-code contract work.
  - Lead with the actual answer or finding in plain English. Use file paths and line anchors as evidence after the explanation, not instead of it, unless exact locations are the main point.
- **Product Reference is navigation only:** `.GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md` is a quick-ref summary. All decisions, technical advice, and implementation guidance MUST be derived from the Master Spec (`SPEC_CURRENT.md`), never from the Product Reference [CX-403].
- Minimum verification for governance-only changes: `just gov-check`.
- Use `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md` for the no-WP governance record flow.
- Governance-maintenance records:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/Audits/**` with stable `AUDIT_ID` and, for smoketest or workflow-proof reviews, `SMOKETEST_REVIEW_ID`
- Governance-maintenance templates:
  - `.GOV/templates/REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`
  - `.GOV/templates/REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`

### Safety commit gate (prevents packet/refinement loss)
- After creating a WP work packet + refinement and obtaining `USER_SIGNATURE`, the orchestrator creates a checkpoint commit on `gov_kernel` (not on the feature branch) [CX-212F].
- Work packets live in `.GOV/task_packets/WP-{ID}/packet.md` (new) or `.GOV/task_packets/WP-{ID}.md` (legacy).
- Refinements live in `.GOV/task_packets/WP-{ID}/refinement.md` (new, co-located) or `.GOV/refinements/WP-{ID}.md` (legacy).

### WP communication artifacts
- Official packets may define `WP_COMMUNICATION_DIR` under the external repo-governance runtime root (default repo-relative from a worktree: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/`; overridable via `HANDSHAKE_GOV_RUNTIME_ROOT` or `HANDSHAKE_RUNTIME_ROOT`).
- These files are governance-only collaboration helpers:
  - `THREAD.md` for append-only freeform discussion
  - `RUNTIME_STATUS.json` for liveness, validator-trigger, waiting-state, next-actor watch state, and bounded loop counters
  - `RECEIPTS.jsonl` for deterministic assignment, status, heartbeat, steering, repair, validation, and handoff receipts
- The work packet remains authoritative for scope, packet status, PREPARE assignment, acceptance, and verdict.
- If packet and communication artifacts disagree, the packet wins.
- These richer artifacts apply to both `MANUAL_RELAY` and `ORCHESTRATOR_MANAGED` workflow lanes.
- The packet-declared `WP_COMMUNICATION_DIR` is the only communication authority for that WP. Do not improvise role-local inboxes.
- When available, prefer VS Code integrated terminals as the host for multi-session role work. Use `just operator-monitor` as the overview surface instead of treating role-local terminal buffers as authority.
- Repo-governed multi-session launch is plugin-first: queue VS Code bridge requests through the external repo-governance launch queue (default repo-relative: `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`), project current state in the external session registry (`../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`), and keep heartbeat as fallback only.
- Only the Orchestrator may start repo-governed Coder, WP Validator, and Integration Validator sessions. Coder/Validator sessions may resume work, but they do not self-start a fresh repo-governed session.
- CLI escalation windows are allowed only after the same role/WP session records 2 plugin failures or timeouts.
- For newly created stubs/packets, repo-governed CLI session policy is explicit: Model selection uses the per-role model-profile catalog (`ROLE_MODEL_PROFILE_POLICY=ROLE_MODEL_PROFILE_CATALOG_V1`). Supported profiles: `OPENAI_GPT_5_4_XHIGH` (default), `OPENAI_GPT_5_2_XHIGH` (fallback), `OPENAI_CODEX_SPARK_5_3_XHIGH` (cost-split coding), `CLAUDE_CODE_OPUS_4_6_THINKING_MAX` (validation). Do not hardcode provider-specific models; use the packet-declared profile.
- Do not rely on whatever model/reasoning defaults happen to be active in an editor or local CLI profile. Launch or claim the session explicitly.
- The ACP broker is a mechanical session-control relay, not an LLM or model provider. All governed model sessions (GPT, Claude Code, Codex Spark) dispatch through the broker. The broker is transport; the model is the engine.
- Repo policy for new repo-governed sessions disallows Codex model aliases in packet claim fields; the CLI tool may still be `codex`.
- Freeform packet-scoped messages should be appended with `just wp-thread-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> "<message>" [target]`; this writes both the thread entry and a paired structured receipt.
- For new WP communication writes, validator sessions must identify themselves as `WP_VALIDATOR` or `INTEGRATION_VALIDATOR` in `THREAD.md`, `RUNTIME_STATUS.json`, and `RECEIPTS.jsonl`. Legacy generic `VALIDATOR` entries are compatibility-only and should not be emitted by new governed sessions.
- When useful for parallel governed sessions, communication receipts and thread entries may carry structured routing metadata such as `target_role`, `target_session`, `correlation_id`, `requires_ack`, `ack_for`, `spec_anchor`, and `packet_row_ref`, and runtime status may carry `next_expected_session`, `waiting_on_session`, and an `open_review_items` projection for unresolved coder/validator exchanges.
- Authority split for semi-autonomous work:
  - Orchestrator = workflow authority
  - WP Validator = advisory technical reviewer
  - Integration Validator = final technical and merge authority

### Governance memory system [CX-503K]
- Governance memory is a cross-session, cross-WP knowledge system stored in `gov_runtime/roles_shared/GOVERNANCE_MEMORY.db` (SQLite). It is NOT a source of truth — work packets, receipts, and governance ledgers remain authoritative. Memory is supplementary context.
- **Role-scoped injection:** Coder startup receives **procedural memories only** (the fail log, up to 1500 tokens). Validator startup receives **procedural + semantic** (fail log + governance context, up to 1500 tokens). Orchestrator startup receives **full cross-WP memory** (all types, governance-weighted, up to 2000 tokens). Session diversification caps at 3 memories per source session. Treat injected memories as hints — the packet and code win over memory.
- **Memory types:** procedural (fix patterns — the fail log), semantic (distilled facts), episodic (session events).
- **Population:** Event-driven: every `wp-receipt-append` immediately extracts high-signal receipts to memory. Batch: `just memory-refresh` runs at every role startup (orchestrator, coder, validator) + during `just gov-check`. Session-end: CLOSE_SESSION captures a session summary. Check failures: validator-scan, validator-handoff-check, pre-work, and post-work failures are auto-captured. Manual: `just memory-capture <type> "<insight>"`.
- **Maintenance:** Dual-gate compaction (time + activity thresholds). Write-time novelty scoring prevents duplicate patterns. New procedural memories supersede matching old ones. Contradiction detection flags conflicting semantic memories. Connectivity-weighted decay resists pruning well-linked memories. Hard cap of 500 active entries. No LLM required for any operation.
- **Role responsibilities:**
  - Orchestrator: owns memory lifecycle. Auto-launches the Memory Manager at startup (staleness-gated) and before WP merge (via closeout check).
  - Memory Manager: governed ACP session on Codex Spark (reasoning extra-high). Analyzes patterns, resolves contradictions, flags stale memories, drafts RGF candidates. Self-terminates after tasks. Protocol: `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`.
  - Coder: receives fail log at startup. `just pre-work` surfaces fail log for the WP. Can capture insights via `just memory-capture procedural "<insight>"`.
  - Validator: receives fail log + context at startup. Check failures auto-captured. Can capture insights via `just memory-capture semantic "<insight>"`.
  - All roles: memory is supplementary — the work packet is the execution authority
- **Backup:** `gov_runtime/` is included in backup snapshots. `just memory-export` provides git-trackable JSONL archival; `just memory-import` restores from export.
- **Intent-gated entry points (Orchestrator, SHOULD):** Before refinement, research, or complex multi-step reasoning, use `just begin-refinement WP-{ID} "<intent>"` or `just begin-research "<intent>" --wp WP-{ID}` to capture an intent snapshot (importance 0.9) before proceeding. `orchestrator-next` emits an `INTENT_SNAPSHOT [RGF-147]` reminder at REFINEMENT stages. The raw `just memory-intent-snapshot` command remains available for ad-hoc use.
- **Canonical reference:** `.GOV/roles_shared/docs/GOVERNANCE_MEMORY_GUIDE.md` — the operational guide for the full memory system.

### Current role execution policy
- Orchestrators MUST NEVER edit product code under `src/`, `app/`, or `tests/`. Even one-line fixes must be routed through the governed coder session via `just session-send`. This is a hard role boundary [CX-580A].
- Orchestrator is non-agentic and single-session, but may coordinate and launch multiple external CLI sessions.
- Validator duties are non-agentic, but repo governance may run multiple validator CLI sessions concurrently when they are scoped as WP Validator and Integration Validator sessions.
- Only the Primary Coder may use coder sub-agents, and only when the packet explicitly records operator approval.
- Shared launch/watch contract: `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`.
- The ACP broker is a mechanical session-control relay, not an LLM or model provider. All governed sessions dispatch through it. The broker auto-reclaims terminal windows on session completion and injects completion notifications [CX-503D, RGF-93, RGF-95].
- Per-MT auto-relay: coder commits trigger a git post-commit hook that fires `wp-review-request` mechanically. The auto-relay dispatches to the validator without orchestrator involvement. Validators respond via `wp-review-response` which auto-relays back to the coder [CX-503C].
- Key orchestrator commands: `just send-mt` (dispatch MT with session keys), `just wp-lane-health` (diagnostic), `just wp-closeout-format` (automated closeout), `just install-mt-hook` (auto-relay hook).
- Validators SHOULD actively challenge code (adversarial review), not just confirm it works [CX-503J].

### Governance kernel architecture [CX-212B/C/D/F]
- **Live junction model:** `/.GOV/` in every non-main worktree is a junction (symlink) to `../wt-gov-kernel/.GOV`. Edits to any `/.GOV/` file are live and immediately visible to all worktrees. There is no branch isolation for governance files.
- **Commit rule [CX-212F]:** `/.GOV/` files are NEVER committed on feature branches (`feat/WP-*`). Governance changes are committed on `gov_kernel` by the orchestrator. Only non-`/.GOV/` files (product code: `src/`, `app/`, `tests/`) are committed on feature branches.
- The governance kernel worktree (`wt-gov-kernel`, branch `gov_kernel`) is the Orchestrator's default live execution surface. It contains `/.GOV/`, git-required files, and may carry a governance-only kernel launcher `justfile`. It must not contain product code.
- `handshake_main` (branch `main`) has a real `/.GOV/` copy as a stable backup, synced from the kernel by the Integration Validator by default, or by the Orchestrator when explicitly instructed by the Operator, using `just sync-gov-to-main`.
- `wt-ilja` (branch `user_ilja`) is the permanent non-main user worktree created from `main`, so product code and root-level LLM files (`justfile`, `AGENTS.md`) come from `main`. Its inherited `/.GOV/` is then replaced with a junction to `../wt-gov-kernel/.GOV`.
- Coder worktrees (`wtc-*`) follow the same pattern: create from `main`, then replace inherited `/.GOV/` with a junction by the worktree creation script.
- WP worktree budget: 1 per WP. Coder and WP Validator share the same `wtc-*` worktree on `feat/WP-{ID}` [CX-503G]. No separate `wtv-*` worktree needed — governance uses `.GOV/` junction. The per-MT stop pattern ensures only one role is active at a time. Integration Validator operates from `handshake_main` on `main`.
- The Orchestrator MAY write to the governance kernel. During active multi-session steering, prefer deferring governance edits to reduce cognitive load.
- Coders and WP Validators read governance through their junction and MUST NOT edit `/.GOV/` directly.
- Sync responsibilities:
  - Orchestrator: edits `/.GOV/` in the kernel, commits on `gov_kernel`; may also run `just sync-gov-to-main` and push `origin/main` when explicitly instructed by the Operator for mechanical governance/topology execution. This does not transfer validator technical authority.
  - Integration Validator: default owner of `just sync-gov-to-main` before pushing to `origin/main`.
  - Coders/WP Validators: read governance through their junction; do NOT edit or commit `/.GOV/` files.
</INSTRUCTIONS>
