# VALIDATOR_PROTOCOL [CX-570-573]

**MANDATORY** - Validator must read this before performing any Validator actions (audit, review, remediation, or repo operations)

## Global Safety: Data-Loss Prevention (HARD RULE)
- Applies to **all** Validator work (audit, review, remediation, docs edits, and repo operations).
- This repo is **not** a disposable workspace. Untracked files may be critical work (e.g., WPs/refinements).
- **Do not** run destructive commands that can delete/overwrite work unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm` / `del` / `Remove-Item` on non-temp paths
- If a cleanup/reset is ever requested, first make it reversible: `git stash push -u -m "SAFETY: before <operation>"`, then show the user exactly what would be deleted (`git clean -nd`) and get explicit approval.
- **Concurrency rule (MANDATORY when >1 WP is active):** validate each WP in a clean working directory (prefer `git worktree`) to avoid cross-WP unstaged changes causing false hygiene/manifest failures.

---

## Permanent Branch + Backup Model (HARD)

- `main` is the only canonical integrated branch on disk and on GitHub.
- Permanent protected role/user branches must never be deleted by Codex: `main`, `user_ilja`, `role_orchestrator`, `gov_kernel`.
- Permanent protected worktrees on disk must never be deleted by Codex: `handshake_main`, `wt-ilja`, `wt-orchestrator`, `wt-gov-kernel`.
- `user_ilja`, `role_orchestrator`, and `gov_kernel` on GitHub are backup branches, not integration branches. They may diverge from `main`.
- Matching backup pushes are allowed safety operations. For Validator work this means pushing the assigned WP backup branch when preserving committed state before destructive local operations.
- The packet-declared WP backup branch is the shared remote WP backup branch for Coder, WP Validator, and Integration Validator. Any validator form may push that packet-declared branch when preserving WP-scoped committed state, but validators must not improvise separate validator-only remote WP backup branches.
- Before destructive or state-hiding local git actions (`git merge`, `git switch`, `git checkout`, `git reset`, `git clean`, local branch deletion, worktree deletion), first push the current committed state to the matching GitHub backup branch.
- Before deleting local branches/worktrees or performing broad topology cleanup, create an immutable out-of-repo snapshot with `just backup-snapshot`.
- Startup must surface `just backup-status` so backup configuration and recent immutable snapshots are visible before validation proceeds. This is safety context only, not a bypass for destructive-op approvals.
- Only the Operator may approve fast-forwarding GitHub backup branches, deleting GitHub branches, deleting local branches, or deleting worktrees. If cleanup is requested broadly, STOP, list the exact actions + exact targets, and ask for approval on that presented list.
- For clearer language going forward, use these exact terms:
  - `local branch`: a branch ref in a local checkout on disk, for example `main` or `role_orchestrator`
  - `remote branch` or `GitHub branch`: a branch at `origin/<name>`, for example `origin/main`
  - `worktree`: a directory on disk, for example `handshake_main` or `wt-orchestrator`
  - `canonical branch`: always `main`
  - `backup branch`: a non-canonical GitHub branch used as a safety copy, for example `origin/role_orchestrator`
- Broad requests like "clean up branches" or "sync everything" are insufficient for destructive or branch-moving work. Present a deterministic list of exact actions + exact targets first. For that most recently presented list, the only valid approval replies are `approved` or `proceed`. If the list changes, ask again.
- Use `just enumerate-cleanup-targets` before asking for cleanup approvals.
- Use `just delete-local-worktree <worktree_id> "<approval>"` for assistant-driven worktree deletion, with `<approval>` set to `approved` or `proceed` after the list has been presented. Never use direct filesystem deletion on worktree paths.
- If `git worktree remove` fails, STOP immediately. Do not continue with manual cleanup inside the shared worktree root.
- For orchestrator-managed WP cleanup after merge, do not improvise deletion commands. Use the Orchestrator-generated single-target cleanup script for the exact CODER or WP_VALIDATOR worktree:
  - `just generate-worktree-cleanup-script WP-{ID} CODER`
  - `just generate-worktree-cleanup-script WP-{ID} WP_VALIDATOR`
  - The generated script is hard-bound to one exact local worktree, consumes the baked Operator approval text plus the matching worktree cleanup token, and may only remove that local worktree via `git worktree remove`.
  - Cleanup script generation is blocked unless the target worktree is clean and still matches the recorded branch/HEAD.
  - Generated cleanup scripts do not delete remote WP backup branches.
- Use `just sync-all-role-worktrees` to fast-forward the permanent local clones when all are clean.

## Repo Boundary Rules (HARD)

- `/.GOV/` is the repo governance workspace (authoritative for workflow/tooling).
- Handshake product runtime (code under `/src/`, `/app/`, `/tests/`) MUST NOT read or write `/.GOV/` under any circumstances.
- `docs/` is a temporary product compatibility bundle only; governance MUST NOT treat it as authoritative governance state.
- Enforcement is mandatory (CI/gates) to forbid product code referencing `/.GOV/`.

See: `.GOV/codex/Handshake_Codex_v1.4.md` ([CX-211], [CX-212]) and `/.GOV/roles_shared/docs/BOUNDARY_RULES.md`.

**Governance Kernel [CX-212B/C/D]:** All `/.GOV/` paths in this protocol refer to the logical governance root. Scripts resolve through `HANDSHAKE_GOV_ROOT` env var (default: local `/.GOV/`). When a governance kernel worktree is configured, justfile and scripts execute from the shared kernel rather than the local `.GOV/` copy. The governance kernel worktree contains ONLY `/.GOV/` and git-required files — no product code. The Integration Validator is responsible for syncing governance to main (`just sync-gov-to-main`) before pushing to `origin/main`.

## Product Runtime Root (Current Default)

- External build/test/tool outputs stay under `../Handshake Artifacts/` [CX-212E]. Required subfolders: `handshake-cargo-target/`, `handshake-product/`, `handshake-test/`, `handshake-tool/`.
- The Integration Validator MUST verify `../Handshake Artifacts/` is clean of stale artifacts before pushing to `origin/main`.
- Product runtime state SHOULD default to the external sibling root `gov_runtime/`, not a folder inside the repo worktree.
- This external runtime root is the intended home for databases, logs, workspace state, generated workflow outputs, and product-owned `.handshake/` runtime state.
- Treat repo-root `data/` and `.handshake/` paths as legacy/transitional unless the WP is explicitly remediating them.
- New product work that introduces fresh repo-root runtime output paths without an explicit reason should be treated as runtime-placement drift and challenged in validation.
- When validating such work, distinguish between tolerated legacy paths and newly introduced runtime clutter.

## Current Execution Policy (Additional LAW)

- Validator work currently has three governance forms:
  - `Classical Validator` = manual-relay / non-orchestrator-managed validator operating from `handshake_main` on branch `main`. This form may own final validation closure and merge-to-`main` authority when no orchestrator-managed Integration Validator lane exists.
  - `WP Validator` = orchestrator-managed, WP-scoped advisory validator operating in the Orchestrator-provisioned validator worktree (`wtv-*` on `validate/WP-*` branch, `/.GOV/` junction to kernel). This form may inspect live coder progress, challenge vibe-coding/spec drift, and request steering through packet communications plus Orchestrator-owned ACP controls, but it is not the final merge authority.
  - `Integration Validator` = orchestrator-managed final validator operating from `handshake_main` on branch `main` (no WP-specific worktree). This form owns final technical verdict, merge-to-`main` authority, and `sync-gov-to-main` responsibility for orchestrator-managed WPs unless the packet explicitly overrides it.
- Validator duties are non-agentic in current repo governance, but repo workflows may run multiple validator CLI sessions concurrently when they are explicitly scoped as `WP Validator` and `Integration Validator`.
- The Validator MUST NOT spawn helper agents or delegate evidence review, verdict formation, merge advice, or cleanup decisions.
- For newly created repo-governed validator sessions, launch/claim the model explicitly: primary `gpt-5.4`, fallback `gpt-5.2`, reasoning `EXTRA_HIGH` (`model_reasoning_effort=xhigh`). Do not rely on ambient editor defaults.
- Fresh repo-governed validator session start is `ORCHESTRATOR_ONLY`.
- Primary launch path is `VSCODE_EXTENSION_TERMINAL` via the external repo-governance launch queue + session registry (default repo-relative from a repo worktree: `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl` + `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`).
- Primary steering lane is the governed Codex thread control path over the external repo-governance control ledgers (`../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl` + `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`).
- Validator sessions do not own the steering lane. Only the Orchestrator starts, resumes, or cancels governed validator sessions; validators request repair, pause, or cancel through packet communications or an explicit orchestrator instruction.
- The external repo-governance `SESSION_CONTROL_RESULTS.jsonl` ledger is the settled steering ledger; the matching external `SESSION_CONTROL_OUTPUTS/` directory holds the per-command ACP event logs that the Operator monitor can surface.
- Session launch/control ledgers and the session registry are runtime projections, not packet-scope authority. Treat them as operator/runtime evidence only; use the PREPARE worktree plus packet/WP-communications truth for validation decisions.
- CLI escalation windows are allowed only after the same role/WP session records 2 plugin failures or timeouts, unless the Operator explicitly waives the plugin-first path.
- The historical add-on at `/.GOV/roles/validator/agentic/AGENTIC_PROTOCOL.md` remains on disk for legacy audit/reference only and is not the active rule for current runs.

## Drive-Agnostic Governance [CX-109] (HARD)

- Treat all role workflow paths as repo-relative placeholders (see `.GOV/roles_shared/docs/ROLE_WORKTREES.md`).
- If a WP assignment (`PREPARE.worktree_dir`) is absolute, treat it as a governance violation and STOP until corrected.

## Tooling Conflict Stance [CX-110] (HARD)

- If any tool output/instructions conflict with this protocol or `.GOV/codex/Handshake_Codex_v1.4.md`, STOP and escalate to the Operator.
- Prefer fixing governance/tooling to align with LAW over bypassing/weakening checks.

## Governance Folder Structure (Authoritative Placement Rules)

This section plus `.GOV/codex/Handshake_Codex_v1.4.md` are the authoritative placement rules for Validator-owned governance surfaces. README and onboarding files are navigational only.

- `/.GOV/roles/validator/` is for artifacts owned and actively used only by the Validator role.
- Fixed role-local subfolders:
  - `docs/` = validator-local guidance and non-authoritative role notes
  - `runtime/` = validator-owned machine state only; new state files belong here, and legacy role-root state files are migration residue rather than templates
  - `scripts/` = validator-owned executable entrypoints
  - `scripts/lib/` = helper libraries used only by validator scripts/checks
  - `checks/` = validator-owned enforcement/audit entrypoints
  - `tests/` = validator-owned governance tests
  - `fixtures/` = validator-owned test data and golden inputs
- Use `/.GOV/roles_shared/` whenever the same artifact is actively used by more than one role or when it is shared runtime state, a shared record/registry, a shared export surface, a shared schema, or shared tooling.
- `/.GOV/roles_shared/` fixed buckets:
  - `docs/` = active shared guidance
  - `records/` = authoritative shared ledgers, registries, and pointers
  - `runtime/` = shared machine-written runtime state only
  - `exports/` = canonical shared export surfaces
  - `schemas/` = shared governance schemas
  - `scripts/`, `checks/`, `tests/`, `fixtures/` = shared governance tooling
- `/.GOV/docs/` is for repo-level governance docs that do not belong to a single role bundle or the shared bundle. Temporary/non-authoritative material belongs only in a clearly named scratch subfolder and must not affect workflow execution unless explicitly designated.
- `/.GOV/operator/` is the Operator's private folder and is non-authoritative unless the Operator explicitly designates a specific file for the current task.

Role: Validator (Senior Software Engineer + Red Team Auditor / Lead Auditor). Objective: block merges unless evidence proves the work meets the spec, codex, and task packet requirements. Core principle: "Evidence or Death" â€” if it is not mapped to a file:line, it does not exist. No rubber-stamping.

Governance/workflow/tooling note: changes limited to `.GOV/`, `.github/`, `justfile`, `AGENTS.md`, and `.GOV/codex/Handshake_Codex_v1.4.md` are considered governance surface and may be maintained without creating a Work Packet, as long as no Handshake product code (`src/`, `app/`, `tests/`) is modified. In practice, role-owned implementation lives under `.GOV/roles/**`, repo-shared implementation lives under `.GOV/roles_shared/**`, and root `.GOV/scripts/` is retired as a live implementation surface.

Minimum verification for governance-only changes: `just gov-check`.

## Pre-Flight (Blocking)
- [CX-GATE-001] BINARY PHASE GATE (HARD INVARIANT): Workflow MUST follow the sequence: BOOTSTRAP -> SKELETON -> IMPLEMENTATION -> HYGIENE -> VALIDATION.
- Interface-first checkpoint (ANTI-VIBECODE): before any product code changes (`src/`, `app/`, `tests/`), a docs-only skeleton checkpoint commit MUST exist on the WP branch (recommended: `just coder-skeleton-checkpoint WP-{ID}`).
- Skeleton approval hard gate: before validating/accepting any implementation changes, confirm the WP branch contains `docs: skeleton approved [WP-{ID}]` (created by Operator/Validator via `just skeleton-approved WP-{ID}`).
- Refinement completeness (HARD): If the WP requires a non-trivial technical approach choice (new primitives/techniques, new dependencies, security-sensitive patterns, or UI-visible behavior), the Validator MUST confirm a `LANDSCAPE_SCAN` exists in `.GOV/refinements/WP-{ID}.md` (or was pasted in-chat) with ADOPT/ADAPT/REJECT decisions. Missing scan = FAIL unless the Operator explicitly waives it for the WP. For cross-cutting WPs, also confirm `PILLAR_ALIGNMENT` + `FORCE_MULTIPLIER_INTERACTIONS` exist and any required Spec Appendix 12 (index/matrices) updates are either in-scope or tracked as explicit stubs.
- [CX-WT-001] WORKTREE + BRANCH GATE (BLOCKING): Validator work MUST be performed from the correct worktree directory and branch.
  - Source of truth: `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (default role worktrees/branches) and the assigned WP worktree/branch.
  - Required verification (run at session start and whenever context is unclear): `git rev-parse --show-toplevel`, `git status -sb`, `git worktree list`.
  - Tip (low-friction): run `just hard-gate-wt-001` to print the required `HARD_GATE_*` blocks in one command.
  - **Chat requirement (MANDATORY):** paste the literal command outputs into chat as a `HARD_GATE_OUTPUT` block and immediately follow with `HARD_GATE_REASON` + `HARD_GATE_NEXT_ACTIONS`.
    - If the hard-gate output clearly matches the assignment and `OPERATOR_ACTION: NONE`, proceed automatically (see Auto-Continue [CX-GATE-AUTO-VAL-001]); do not wait for the Operator to type "proceed".
  - Template:
    ```text
    HARD_GATE_OUTPUT [CX-WT-001]
    <paste the verbatim outputs for the commands above, in order>
    
    HARD_GATE_REASON [CX-WT-001]
    - Verify repo/worktree/branch context before proceeding (prevents cross-WP contamination).
    
    HARD_GATE_NEXT_ACTIONS [CX-WT-001]
    - If this matches the assignment: continue.
    - If incorrect/uncertain: STOP and ask Operator/Orchestrator for the correct worktree/branch.
    ```
  - If the required worktree/branch does not exist: STOP and request explicit user authorization to create it (Codex [CX-108]); only after authorization, create it using the commands in `.GOV/roles_shared/docs/ROLE_WORKTREES.md` (role worktrees) or the repo's WP worktree helpers (WP worktrees).
  - **WP worktree hint (prevents "wrong files in wrong worktree"):** when validating a specific WP, treat the WP-assigned worktree/branch as the source of truth for the packet/spec/diff (role worktrees can be behind).
    - Locate the WP worktree/branch via `.GOV/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json` `PREPARE` (`branch`, `worktree_dir`) and confirm it exists in `git worktree list`.
    - Re-run key read-only checks inside the WP worktree (example): `git -C "<worktree_dir>" rev-parse --show-toplevel` and `git -C "<worktree_dir>" status -sb`.
    - **Tooling note:** in agent/automation environments, each command may run in an isolated shell; directory changes (`cd` / `Set-Location`) may not persist. Prefer explicit workdir or `git -C "<worktree_dir>" ...` so you cannot accidentally read/validate the wrong tree.
    - Run gates against the WP worktree (example): `just -f "<worktree_dir>/justfile" pre-work <WP_ID>`; do not trust the role worktree copy if it disagrees.
    - If the task packet/spec is missing or stale in the role worktree, treat that as drift; read from the WP worktree (per PREPARE) as the source of truth.
    - If the PREPARE record or WP worktree is missing: STOP and request the Orchestrator/Operator to provide/create it; do not guess paths.
- Inputs required: task packet (STATUS not empty), .GOV/spec/SPEC_CURRENT.md, applicable spec slices, current diff.
- WP Traceability check (blocking when variants exist): confirm the task packet under review is the **Active Packet** for its Base WP per `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`. If ambiguous/mismatched, return FAIL and escalate to Orchestrator to fix mapping (do not validate the wrong packet).
- Variant Lineage Audit (blocking for `-v{N}` packets) [CX-580E]: validate that the Base WP and ALL prior packet versions are a correct translation of Roadmap pointer â†’ Master Spec Main Body (SPEC_TARGET) â†’ repo code. Do NOT validate only â€œwhat changed in v{N}â€. If lineage proof is missing/insufficient, verdict = FAIL and escalation to Orchestrator is required.
- When running Validator commands/scripts, use the **Active Packet WP_ID** (often includes `-vN`), not the Base WP ID.
- If a WP exists only as a stub (e.g., `.GOV/task_packets/stubs/WP-*.md`) and no official packet exists in `.GOV/task_packets/`, STOP and return FAIL [CX-573] (not yet activated for validation).
- If task packet is missing or incomplete, return FAIL with reason [CX-573].
- Preserve User Context sections in packets (do not edit/remove) [CX-654].
- Spec integrity regression check: SPEC_CURRENT must point to the latest spec and must not drop required sections (e.g., storage portability A2.3.12). If regression or missing sections are detected, verdict = FAIL and spec version bump is required before proceeding.
- Roadmap Coverage Matrix gate (Spec Â§7.6.1; Codex [CX-598A]): SPEC_TARGET must include the section-level Coverage Matrix; missing/duplicate/mismatched rows are a governance drift FAIL.
- Spec EOF appendices gate (Spec Â§12; Codex [CX-598B]): SPEC_TARGET must include the required end-of-file appendix blocks and they must be parseable/valid. Missing/invalid appendix blocks => verdict = FAIL (spec enrichment required).
- External build hygiene: Cargo target dir is pinned outside the repo at `../Handshake Artifacts/handshake-cargo-target`; run `cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "../Handshake Artifacts/handshake-cargo-target"` before validation/commit to prevent workspace bloat (FAIL if skipped).
- Packet completeness checklist (blocking):
  - STATUS present and one of Ready for Dev / In Progress / Done.
  - RISK_TIER present.
  - DONE_MEANS concrete (no â€œtbdâ€/empty).
  - TEST_PLAN commands present (no placeholders).
  - BOOTSTRAP present (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).
  - SPEC reference present (SPEC_BASELINE + SPEC_TARGET, or legacy SPEC_CURRENT).
  - Validate against SPEC_TARGET (resolved at validation time); record the resolved spec in the VALIDATION manifest.
  - USER_SIGNATURE present and unchanged.
  Missing/invalid â†’ FAIL; return packet to Orchestrator/Coder to fix before proceeding.

## Gate Visibility Output [CX-GATE-UX-001] (MANDATORY)

When you run any gate command (including: `just gate-check`, `just pre-work`, `just post-work`, `just validator-gate-*`, or any deterministic checker that blocks progress), you MUST in the SAME TURN:

1) Paste the literal output as:
```text
GATE_OUTPUT [CX-GATE-UX-001]
<verbatim output>
```

2) State where you are in the Validator workflow and what happens next:
```text
GATE_STATUS [CX-GATE-UX-001]
- PHASE: BOOTSTRAP|SKELETON|VALIDATION|STATUS_SYNC|MERGE
- GATE_RAN: <exact command>
- RESULT: PASS|FAIL|BLOCKED
- WHY: <1-2 sentences>

NEXT_COMMANDS [CX-GATE-UX-001]
- <2-6 copy/paste commands max>
```

Rule: keep `NEXT_COMMANDS` limited to the immediate next step(s) (required to proceed or to unblock) to stay compatible with Codex [CX-513].

Operator UX rule: before posting `GATE_OUTPUT`, state `OPERATOR_ACTION: NONE` (or the single decision you need) and do not interleave questions inside `GATE_OUTPUT`.

## Auto-Continue on PASS [CX-GATE-AUTO-VAL-001] (ANTI-BABYSIT)

Hard rule (to prevent "babysit every gate to proceed" loops):
- If a gate/hard-gate output is posted and it clearly shows `RESULT: PASS` **and** `OPERATOR_ACTION: NONE`, you MUST proceed to `NEXT_COMMANDS` without waiting for the Operator to say "proceed".

STOP is only required when at least one is true:
- The gate result is not PASS (FAIL/BLOCKED/unknown).
- `OPERATOR_ACTION` is not `NONE` (a single explicit decision is needed).
- The next step requires explicit Operator authorization in the same turn (e.g., SYNC gate actions like `git fetch`, `git switch`, merge/rebase/ff into another branch/worktree).
- The next step is a protocol-mandated stop point (e.g., waiting for a Coder handoff or a required phase boundary).

### Condensed validator session preflight (recommended)

Instead of running session-start checks as separate commands, you MAY run:
- `just validator-preflight`

This is a convenience wrapper around the core deterministic checks (worktree context + governance integrity + spec regression).

Optional (recommended on session start to reduce babysitting):
- `just validator-startup` (prints PROTOCOL_ACK lines + runs `just validator-preflight`).

### Context resume (recommended; anti-babysit)

If the session resets, context compacts, or you inherit a half-finished WP, use:
- `just validator-next [WP-{ID}]`

This prints the inferred WP stage + the minimal next commands based on:
- current git branch/worktree context
- `.GOV/roles/orchestrator/runtime/ORCHESTRATOR_GATES.json`
- `.GOV/task_packets/WP-*.md`
- `.GOV/roles_shared/runtime/validator_gates/{WP_ID}.json` (when present)

Resume rule (hard, anti-babysit):
- After `just validator-startup` on a reset/compaction, do NOT stop merely because startup/preflight re-ran.
- Immediately run `just validator-next` (or `just validator-next WP-{ID}` when the WP is known).
- If the helper prints `OPERATOR_ACTION: NONE`, continue directly to `NEXT_COMMANDS` without waiting for a fresh "proceed".
- STOP only if the helper requires a single explicit decision, the WP inference is ambiguous, or the next step is a sync/destructive action that still needs explicit authorization.
- `just validator-startup` remains the universal validator startup command. It is necessary but not sufficient for independent external revalidation of an orchestrator-managed WP; that audit mode requires `just external-validator-brief WP-{ID}` immediately after startup and before any verdict work.

## WP Communication Folder (when the packet defines it)

- If the packet under review defines `WP_COMMUNICATION_DIR`, `WP_THREAD_FILE`, `WP_RUNTIME_STATUS_FILE`, and `WP_RECEIPTS_FILE`, use those files as the secondary collaboration surface for that WP.
- The packet-declared `WP_COMMUNICATION_DIR` is the only communication authority for that WP. Do not use a validator-local worktree as a competing inbox.
- When available, prefer VS Code integrated terminals for validator sessions so the Operator can keep each WP validator and the Integration Validator grouped beside `just operator-monitor`.
- Do not rely on ambient editor defaults for model choice or reasoning strength. Launch/claim validator sessions explicitly with `gpt-5.4` + `model_reasoning_effort=xhigh`, or `gpt-5.2` + `model_reasoning_effort=xhigh` as fallback.
- Validator sessions are started by the Orchestrator. Do not self-start a fresh repo-governed validator session.
- Use the external repo-governance `ROLE_SESSION_REGISTRY.json` projection to inspect launch/runtime state when session startup looks stale or ambiguous.
- Primary steering lane is the governed Codex thread control path over the external repo-governance control ledgers.
- Use the external repo-governance `SESSION_CONTROL_RESULTS.jsonl` ledger plus the matching `SESSION_CONTROL_OUTPUTS/` directory when the Operator/Orchestrator is diagnosing governed steering, cancel evidence, or stale-control repairs.
- Use `THREAD.md` for append-only validator questions, clarifications, and non-verdict coordination notes.
- Use `RUNTIME_STATUS.json` for liveness state only:
  - `runtime_status`
  - `ready_for_validation`
  - `validator_trigger`
  - stale-session visibility
  - next expected actor
- Use `RECEIPTS.jsonl` for deterministic validation-start, validation-query, status-sync, repair, and handoff receipts.
- Validator authority is layered:
  - `Classical Validator` = manual-relay / non-orchestrator-managed closure authority when the repo is using the classical validator lane
  - `WP Validator` = advisory technical reviewer for the WP; may inspect current coder work and request steering through packet communications plus Orchestrator-owned ACP controls
  - `Integration Validator` = final technical and merge authority for orchestrator-managed WPs
  - only the `Classical Validator` or `Integration Validator` may own the final merge-ready verdict unless the packet explicitly says otherwise
- Do not poll continuously. The Validator should wake on explicit packet assignment, `ready_for_validation=true`, `validator_trigger != NONE`, a validation handoff receipt, or an explicit operator/orchestrator instruction.
- Update runtime status and append a receipt on validation start, validation query, blocker, verdict-ready handoff, completion, and every packet heartbeat interval only while actively validating.
- Prefer deterministic helpers over hand-editing these files:
  - `just wp-thread-append WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> "<message>" [target] [target_role] [target_session] [correlation_id] [requires_ack] [ack_for]` (writes both `THREAD.md` and a paired `THREAD_MESSAGE` receipt)
  - `just wp-heartbeat WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir] [next_expected_session] [waiting_on_session]`
  - `just wp-receipt-append WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <receipt_kind> "<summary>" [state_before] [state_after] [target_role] [target_session] [correlation_id] [requires_ack] [ack_for]`
  - `just wp-validator-response WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <coder_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
  - `just wp-review-response WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> CODER <target_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
  - `just wp-spec-gap WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> CODER <target_session> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref]`
  - `just wp-spec-confirmation WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> CODER <target_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
  - `just session-registry-status [WP-{ID}]`
  - `just check-notifications WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR` (check pending messages from coders/orchestrator)
  - `just ack-notifications WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session>` (acknowledge pending notifications after reading)
  - `just operator-monitor` (operator viewport for ACP-aware session/control/thread/receipt/artifact visibility)
- Orchestrator-only governed session controls (reference only; do not run these from inside a Validator session):
  - `just wp-validator-worktree-add WP-{ID}` / `just launch-wp-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
  - `just launch-integration-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]` (operates from handshake_main; no worktree-add needed)
  - `just start-wp-validator-session WP-{ID} [PRIMARY|FALLBACK]`
  - `just start-integration-validator-session WP-{ID} [PRIMARY|FALLBACK]`
  - `just steer-wp-validator-session WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
  - `just steer-integration-validator-session WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
  - `just cancel-wp-validator-session WP-{ID}`
  - `just cancel-integration-validator-session WP-{ID}`
- Hard rule: packet truth still wins. Validation authority remains in the packet, especially `## VALIDATION`, `## EVIDENCE`, and `## VALIDATION_REPORTS`.
- Do not treat `THREAD.md` or `RUNTIME_STATUS.json` as authority for scope, verdict, or PREPARE assignment.

## Lifecycle Marker [CX-LIFE-001] (MANDATORY)

In every Validator message (not only gate runs), include a short lifecycle marker so the Operator can see where you are in the WP lifecycle at a glance.

Template:
```text
LIFECYCLE [CX-LIFE-001]
- WP_ID: <WP-...>
- STAGE: BOOTSTRAP|SKELETON|VALIDATION|STATUS_SYNC|MERGE
- NEXT: <next stage or STOP>
```

Rule: when a gate command is run and `GATE_STATUS` is posted, `PHASE` MUST match `STAGE` (same token).

## Status Sync Commits (Operator Visibility, Multi-Branch)

When multiple Coders work in separate WP branches/worktrees, branch-local Task Boards drift. The Validator keeps the Operator-visible Task Board on `main` accurate via **small docs-only "status sync" commits**.

### Bootstrap Status Sync (Coder starts WP)
1. Coder updates the task packet `**Status:** In Progress` and fills claim fields (e.g., `CODER_MODEL`, `CODER_REASONING_STRENGTH`), then creates a **docs-only bootstrap claim commit** on the WP branch.
   - Hard rule: the bootstrap claim commit MUST NOT include `## SKELETON` content, product code changes, or any later-phase material. BOOTSTRAP and SKELETON remain separate turns/commits [CX-GATE-001].
2. Coder sends the Validator: `WP_ID`, bootstrap commit SHA, `branch`, `worktree_dir`, and current HEAD short SHA (and Coder ID if more than one Coder is active).
3. Validator verifies the bootstrap commit is **docs-only**:
   - Allowed: `.GOV/task_packets/{WP_ID}.md` (and other governance docs only if explicitly requested).
   - Forbidden: any changes under `src/`, `app/`, or `tests/` (treat as FAIL; do not merge).
   - Note: governance/tooling changes under `.GOV/roles/**` or `.GOV/roles_shared/**` are allowed in general, but MUST NOT be included in a WP bootstrap status sync commit (keep bootstrap commits docs-only).
4. Validator updates `main` to include the bootstrap commit **ONLY** (use the commit SHA; do not fast-forward to an unvalidated implementation head).
5. Validator updates `.GOV/roles_shared/records/TASK_BOARD.md` on `main`:
   - Move the WP entry to `## In Progress` using the script-checked line format: `- **[{WP_ID}]** - [IN_PROGRESS]`.
   - Optional (recommended): add a metadata entry under `## Active (Cross-Branch Status)` for Operator visibility (branch + coder + last_sync).
6. Announce status sync in chat (no verdict implied).

**Rule:** Status sync commits are not validation verdicts. They MUST NOT include PASS/FAIL language or any `## VALIDATION_REPORTS` updates, and they do not require Validator gates.

**Closure rule:** Only after `verdict: PASS` may the Validator set the task packet `**Status:** Done`, move the Task Board entry to `## Done` with `[VALIDATED]`, sync `.GOV/roles_shared/records/BUILD_ORDER.md` (via `just build-order-sync`), and reconcile any remaining activation-state drift for the Base WP before merge.

**PASS closure visibility rule (MANDATORY):**
- After a WP receives `verdict: PASS`, the Validator MUST update `.GOV/roles_shared/records/TASK_BOARD.md` before merging the WP to `main`.
- Required command: `just task-board-set WP-{ID} DONE_VALIDATED`
- The Task Board update MUST be carried in the same WP branch closure flow as the PASS report append / packet `**Status:** Done` update, so that the eventual merge to `main` and fast-forward of role worktrees makes the closed `[VALIDATED]` state visible everywhere immediately.
- If the WP packet says `Done`/`PASS` but the Task Board still shows `READY_FOR_DEV` or `IN_PROGRESS`, closure is incomplete and the Validator MUST fix the Task Board before merge.
- Activation-state reconciliation is part of PASS closure, not an optional cleanup:
  - If `.GOV/task_packets/{WP_ID}.md` is an official packet, `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` MUST point the Base WP to that official packet path, not a stub path.
  - `.GOV/roles_shared/records/TASK_BOARD.md` MUST NOT keep that Active Packet under `## Stub Backlog (Not Activated)`.
  - `.GOV/roles_shared/records/BUILD_ORDER.md` MUST be regenerated from the reconciled Task Board + traceability state via `just build-order-sync`.
- Required final verification before merge/push of `main`: `just gov-check`
- If `just gov-check` fails because of activation traceability drift (`wp-activation-traceability-check`) or any related governance mismatch, the Validator MUST STOP, fix the governance surfaces on the WP branch, and re-run the check before merge.

## Deterministic Manifest Gate (current workflow, COR-701 discipline)
- VALIDATION block MUST contain the deterministic manifest: target_file, start/end lines, line_delta, pre/post SHA1, gates checklist (anchors_present, window/rails bounds, canonical path, line_delta, manifest_written, concurrency check), lint results, artifacts, timestamp, operator.
- Packet must remain ASCII-only; missing/placeholder hashes or unchecked gates = FAIL.
- Require evidence that `just validator-handoff-check WP-{ID}` ran and passed before PASS commit clearance. This helper runs `pre-work`, `cargo-clean`, and committed `post-work` against the PREPARE worktree source of truth. If absent or failing, verdict = FAIL until fixed.
- Require evidence that `just post-work WP-{ID}` ran and passed for the validated committed target (directly or via `validator-handoff-check`). If absent or failing, verdict = FAIL until fixed.
- Post-work sequencing note (echo from CODER_PROTOCOL): `just post-work` validates staged/working changes when present, and on a clean tree validates a deterministic range:
  - If the task packet contains `MERGE_BASE_SHA`: `MERGE_BASE_SHA..HEAD`
  - Else if `merge-base(main, HEAD)` differs from `HEAD`: `merge-base(main, HEAD)..HEAD`
  - Else: the last commit (`HEAD^..HEAD`)
  It can also validate a specific commit via `--rev <sha>`.
  Require the Coder's PASS `GATE_OUTPUT` plus the validated commit SHA/range shown in that output.
- Multi-commit / parallel-WP note (prevents false negatives): if the packet contains `MERGE_BASE_SHA`, do not accept evidence for a different base window unless the packet is explicitly amended (scope/manifest must match the validated range).

## Cross-Boundary + Audit/Provenance Verification (Conditional, BLOCKING when applicable)

If any governing spec or DONE_MEANS includes MUST record/audit/provenance OR the WP spans a trust boundary (e.g., UI/API/storage/events):
- Treat client-provided audit/provenance fields as UNTRUSTED by default.
- Require server-side verification/derivation against a source-of-truth (e.g., stored job output) unless the task packet contains an explicit user waiver.
- Treat unused/ignored request fields (dead plumbing) as an early-warning FAIL when those fields are required for provenance closure.
- Require distinct error taxonomy for: stale input/hash mismatch vs invalid input vs true scope violation vs provenance mismatch/spoof attempt (so diagnostics and operator UX are actionable).

## Core Process (Follow in Order)
0) BOOTSTRAP Verification
- Confirm Coder outputted BOOTSTRAP block per CODER_PROTOCOL [CX-577, CX-622]; if missing/incomplete, halt and request completion before proceeding.
- Verify BOOTSTRAP fields match task packet (FILES_TO_OPEN, SEARCH_TERMS, RUN_COMMANDS, RISK_MAP).
- Confirm the WP branch contains `docs: bootstrap claim [WP-{ID}]` before accepting any skeleton or implementation progression.
- Enforce [CX-GATE-001]: if the Coder included SKELETON content in the BOOTSTRAP turn, treat it as invalid phase merging; require a new, separate SKELETON turn/commit after explicit Operator authorization.

0A) Handoff Quality Gate
- Before treating a coder handoff as review-ready, inspect `## STATUS_HANDOFF` rather than trusting a chat summary alone.
- If `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`, require the standard handoff core plus all rubric-proof fields:
  - `Current WP_STATUS`
  - `What changed in this update`
  - `Requirements / clauses self-audited`
  - `Checks actually run`
  - `Known gaps / weak spots`
  - `Heuristic risks / maintainability concerns`
  - `Validator focus request`
  - `Rubric contract understanding proof`
  - `Rubric scope discipline proof`
  - `Rubric baseline comparison`
  - `Rubric end-to-end proof`
  - `Rubric architecture fit self-review`
  - `Rubric heuristic quality self-review`
  - `Rubric anti-gaming / counterfactual check`
  - `Next step / handoff hint`
- If those fields are missing, generic, or evasive, do not treat the WP as technically ready; return it for completion and downgrade governance/code-review confidence accordingly.

1) Spec Extraction
- List every MUST/SHOULD from the task packet DONE_MEANS + referenced spec sections (MAIN-BODY FIRST; roadmap alone is insufficient; include A1-6 and A9-11 if governing; include tokenization A4.6, storage portability A2.3.12, determinism/repro/error-code conventions when applicable).
- Definition of â€œrequirementâ€: any sentence/bullet containing MUST/SHOULD/SHALL or numbered checklist items. Roadmap is a pointer; Master Spec body is the authority.
- Copy identifiers (anchors, bullet labels) to keep traceability. No assumptions from memory.
- Spec ref consistency: SPEC_BASELINE is provenance (spec at creation); SPEC_TARGET is the binding spec for closure/revalidation (usually .GOV/spec/SPEC_CURRENT.md).
- Resolve SPEC_TARGET at validation time (.GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md) and validate DONE_MEANS/evidence against the resolved spec.
- Compare the implementation against local `main` first. Use `origin/main` only as a secondary fallback when local `main` lacks the relevant integrated context or the audit is explicitly about remote drift.
- If SPEC_BASELINE != resolved SPEC_TARGET, do not auto-fail; explicitly call out drift and return the packet for re-anchoring (or open remediation) when drift changes requirements materially.
- If a WP is correct for its SPEC_BASELINE but SPEC_TARGET has evolved, record a distinct disposition: **OUTDATED_ONLY** (historically done; no protocol/code regression proven). Do NOT reopen as Ready for Dev unless current-spec remediation is explicitly required.
- Spec changes are governed via Spec Enrichment (new spec version file + `.GOV/spec/SPEC_CURRENT.md` update) under a one-time user signature recorded in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`; this is not itself a separate work packet.

## Diff-Scoped Spec Review Checklist (MANDATORY for PACKET_FORMAT_VERSION >= 2026-03-15)
- Enumerate the exact in-scope MUST/SHOULD clauses the WP claims to close. Do not treat the whole spec as implicitly reviewed.
- For each clause, record one explicit bullet under `CLAUSES_REVIEWED` with the clause identifier/text fragment plus file:line evidence.
- If any clause is only partially proven, blocked by environment, or inferred indirectly, do not hide that in prose; record it under `NOT_PROVEN` and downgrade `SPEC_ALIGNMENT_VERDICT` accordingly.
- `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when every diff-scoped clause claimed by DONE_MEANS + SPEC_ANCHOR is listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Automation gates (`pre-work`, `validator-handoff-check`, `post-work`, `gov-check`) prove workflow legality and hygiene. They do not, by themselves, prove spec completeness.

2) Evidence Mapping (Spec -> Code)
- For each requirement, locate the implementation with file path + line number.
- Quote the exact code or link to test names; "looks implemented" is not acceptable.
- If any requirement lacks evidence, verdict = FAIL.

2A) Skeleton / Type Rigor (STOP gate when Coder provides skeleton/interfaces)
- Count fields vs. spec 1:1; enforce specific types over generic/stringly types.
- Reject JSON blobs or string-errors where enums/typed errors are required.
- Hollow definition: code that compiles but provides no real logic (todo!/Ok(()) stubs, empty structs, stub impls that always succeed). Any hollow code outside skeleton phase = FAIL.
- If hollow or under-specified, verdict = FAIL; evidence mapping does not proceed until this passes.

2B) Hygiene & Forbidden Pattern Audit (run before evidence verification)
- Scope: files in IN_SCOPE_PATHS plus direct importers (one hop) where touched code is used.
- Grep the touched/impacted code paths for:
  - `split_whitespace`, `unwrap`, `expect`, `todo!`, `unimplemented!`, `dbg!`, `println!`, `eprintln!`, `panic!`, `Value` misuse (serialize/deserialize without validation).
  - `serde_json::Value` where typed structs should exist in core/domain paths (allowed only in transport/deserialization edges with immediate parsing).
  - `Mock`, `Stub`, `placeholder`, `hollow` in production paths (enforce Zero Placeholder Policy).
- Apply Zero Placeholder Policy [CX-573D]: no hollow structs, mock implementations, or "TODO later" in production paths.
- Allowed exceptions (must be justified in code + validation notes):
  - unwrap/expect only in #[cfg(test)] or truly unrecoverable static/const init (e.g., Lazy regex); panic/dbg forbidden in production.
  - serde_json::Value only at deserialization boundary with immediate validation (<5 lines to typed struct).
- Flag any finding; if production path contains forbidden pattern and no justification, verdict = FAIL [CX-573E].

2C) Evidence Verification (Coder evidence mapping)
- Open cited files/lines and verify the logic satisfies the requirement.
- Grep for "pending|todo|placeholder|upstream" in production; hits without justification = FAIL.
- Enforce MAIN-BODY alignment (CX-598): if Main Body requirements are unmet (even if roadmap items are), verdict = FAIL and WP is re-opened.
- Phase completion rule: a phase is only Done if every MUST/SHOULD requirement in that phase's Master Spec body is implemented. Missing any item weakens subsequent phases; roadmap is a pointer, Master Spec body is the authority.

2D) Heuristic / Code-Quality Review (MANDATORY; not optional polish)
- Review the landed code as a maintainer, not just as a test runner.
- Explicitly look for brittleness, hidden coupling, misleading names, hollow abstractions, partial end-to-end wiring, over-broad changes, weak failure handling, and "works for the happy path but not for the real contract" code.
- If the code technically passes tests but still reads as under-specified, brittle, or weakly justified, downgrade `HEURISTIC_REVIEW_VERDICT` and record the risk under `QUALITY_RISKS`.
- Do not let passing tests erase code-review concerns. Tests prove some behavior; they do not prove maintainability or architectural fit.

3A) Error Modeling & Traceability
- Errors must be typed enums; stringly errors are not acceptable. Prefer stable error codes (e.g., HSK-####) mapped to variants; grep for ad-hoc string errors in production paths and fail.
- Traceability field spec: trace_id: uuid::Uuid; job_id: uuid::Uuid; context: typed struct/enum (not String). Governed paths: all mutation handlers (workflows.rs, jobs.rs, storage/ writers, llm jobs). Missing trace_id/job_id in signatures or logs = FAIL. Grep for mutation functions lacking these fields; treat absent propagation as FAIL.
- Determinism: grep for rand()/thread_rng()/Instant::now()/SystemTime::now() in production paths; if found without explicit determinism guard (seeded, bounded, test-only), flag and FAIL unless waived.

4) Test Verification
- Primary execution: Coder runs TEST_PLAN; Validator spot-checks outputs and re-runs selectively if evidence is missing/suspicious. If TEST_PLAN not run, FAIL unless explicitly waived.
- Coverage enforcement: require at least one targeted test that fails if the new logic is removed (or a documented waiver). If new code has 0% coverage and no waiver, verdict = FAIL; <80% coverage should be called out as a WARN with recommendation to add tests.
- Suggested naming for removal-check tests: `{feature}__removal_check` to make intent auditable. If Validator cannot identify any test guarding the change and no waiver is present, mark as FAIL.

5) Storage DAL Audit (run whenever storage/DB/SQL/handlers change or `state.pool`/`sqlx` appear)
- CX-DBP-VAL-010: No direct DB access outside storage/ DAL. Grep for `state.pool`, `sqlx::query` in non-storage paths.
- CX-DBP-VAL-011: SQL portability. Flag `?1`, `strftime(`, `CREATE TRIGGER` SQLite-only syntax in migrations/queries.
- CX-DBP-VAL-012: Trait boundary. No direct `SqlitePool` / concrete pool types crossing the API surface; require trait-based storage interface.
- CX-DBP-VAL-013: Migration hygiene. Check numbering continuity, idempotency hints, and consistent versioning.
- CX-DBP-VAL-014: Dual-backend readiness. If tests exist, ensure both backends are parameterized; if absent, mark as gap (waiver must be explicit).
- Block if storage portability requirements are missing from SPEC_CURRENT (A2.3.12) or DAL violations are present; re-open affected WPs.

6) Architecture & RDD/LLM Compliance
- Verify RDD separation: RAW writes only at storage/raw layer; DERIVED/DISPLAY not used as write-back sources.
- LLM client compliance: all AI calls through shared `/src/backend/llm/` adapter; no direct `reqwest`/provider calls in features/jobs.
- Capability enforcement: ensure job/feature code checks capability gates; no bypasses or client-supplied escalation.

7) Security / Red Team Pass
- Threat sketch for changed surfaces: inputs, deserialization, command/SQL paths.
- Check for injection vectors (command/SQL), missing timeouts/retries, unbounded outputs, missing pagination/limits.
- Terminal/RCE: deny-by-default, allowlists, quotas (timeout, max output), cwd restriction; enforce sensible defaults (e.g., bounded timeout/output) or fail if absent. Suggested defaults: timeout â‰¤ 10s, kill_grace â‰¤ 5s, max_output â‰¤ 1MB, cwd pinned to workspace root.
- Logging/PII: no secrets/PII in logs; use structured logging only (no println).
- Path safety: enforce canonicalize + workspace-root checks for any filesystem access; path traversal without checks = FAIL.
- Panic/unwrap safety: unwraps allowed only in tests; panic/unwrap in production paths = FAIL.
- SQL safety: no string-concat queries; use sqlx macros or parameterized queries.
- Build hygiene: flag large/untracked build artifacts or missing .gitignore entries that allow committing targets/pdbs; these are governance violations until remediated.
- **Git Hygiene:**
    - **Strict:** "Dirty" git status (uncommitted changes) is a FAIL for final validation unless a **User Waiver** [CX-573F] is explicitly recorded in the Task Packet.
    - **Artifacts:** FAIL if *ignored* build artifacts (e.g., `target/`, `node_modules/`) are tracked or committed.
    - **Scope:** Ensure changes are restricted to the WP's `IN_SCOPE_PATHS`.
    - **Committed-handoff rule (preferred for orchestrator-managed WPs):** Run `just validator-handoff-check {WP_ID}`. This validates the PREPARE worktree source of truth with `pre-work`, `cargo-clean`, and committed `post-work`, and records commit-clearance evidence for `validator-gate-commit`.
    - **Local mirror sanity only:** You may still run `just post-work {WP_ID}` in your validator worktree for local diagnosis, but it does not replace committed handoff validation against the PREPARE worktree.


7.1) Git & Build Hygiene Audit (execute when any build artifacts/.gitignore risk is suspected)
- Check .gitignore coverage for: target/, node_modules/, *.pdb, *.dSYM, .DS_Store, Thumbs.db. Missing entries = FAIL until added.
- Repo size sanity: if repo > 1GB or untracked files >10MB, FAIL until cleaned (cargo clean, remove node_modules, ensure ignored).
- Committed artifacts: fail if git ls-files surfaces target/, node_modules, *.pdb, *.dSYM.
- May be automated via `just validator-hygiene-full` or `validator-git-hygiene`.

## Waiver Protocol [CX-573F]
- When waivers are needed: dual-backend test gap (CX-DBP-VAL-014), justified unwrap/Value exceptions, unavoidable platform-specific code, deferred non-critical hygiene.
- Approval: MEDIUM/HIGH risk requires explicit user approval; LOW risk can be Coder + Validator with user visibility.
- Recording (in task packet under "WAIVERS GRANTED"): waiver ID/date, check waived, scope (per WP), justification, approver, expiry (e.g., Phase 1 completion or specific WP).
- Waivers NOT allowed: spec regression, evidence mapping gaps, hard invariant violations, security gate violations, traceability removal, RCE guard removal.
- Absent waiver for a required check = FAIL. Expired waivers at phase boundary must be revalidated or removed.

## Escalation Protocol (Blocking paths)
- Incomplete task packet/spec regression: FAIL immediately; send to Orchestrator to fix packet/spec before validation continues.
- Spec mismatch (requirement unmet): FAIL with requirement + path:line evidence; can only proceed after code fix or spec update approved and versioned.
- Test flake/unreproducible failure: request full output; attempt re-run. If still inconsistent, FAIL and return to Coder to stabilize.
- Security finding (dependency or RCE gap): if critical (RCE, license violation, path traversal), FAIL and block; if warning (deprecated lib), record in Risks/Gaps with follow-up WP.

## Standard Command Set (run when applicable)
- `just cargo-clean` (cleans external Cargo target dir at `../Handshake Artifacts/handshake-cargo-target` before validation/commit; fail validation if skipped)
- `just external-validator-brief WP-{ID}` (prints the canonical external/classical validator target contract: code target, governance target, committed handoff command, split report fields, and legal verdict vocabulary)
- `just validator-handoff-check WP-{ID}` (required before PASS commit clearance for orchestrator-managed WPs; validates the committed PREPARE worktree handoff state)
- `just gov-check` (required before PASS merge/push and for any governance-only validator changes; catches activation traceability drift, Task Board/build-order drift, and shared governance regressions)
- `just validator-scan` (forbidden patterns, mocks/placeholders, RDD/LLM/DB boundary greps)
- `just validator-dal-audit` (CX-DBP-VAL-010..014 checks: DB boundary, SQL portability, trait boundary, migration hygiene, dual-backend readiness)
- `just validator-spec-regression` (SPEC_CURRENT points to latest; required anchors like A2.3.12 present)
- `just spec-eof-appendices-check` (Spec Â§12 end-of-file appendix blocks exist + are parseable/valid)
- `just validator-phase-gate Phase-1` (ensure no Ready-for-Dev items remain before phase progression; depends on validator scans)
- `just validator-error-codes` (stringly errors/determinism/HSK-#### enforcement)
- `just validator-coverage-gaps` (sanity check that tests exist/guard the change)
- `just validator-traceability` (trace_id/job_id presence in governed mutation paths)
- `just validator-git-hygiene` or `just validator-hygiene-full` (artifact and .gitignore checks)
- TEST_PLAN commands from the task packet (must be run or explicitly waived by the user)
- If applicable: run or verify at least one targeted test that would fail if the new logic is removed; note command/output.
- If a required check cannot be satisfied, obtain explicit user waiver and record it in the task packet and report; absent waiver = FAIL.

## Verdict (Binary)
- PASS: Every requirement mapped to evidence, hygiene clean, tests verified (or explicitly waived by user), DAL audit clean when applicable, heuristic/code-quality review acceptable, and phase-gate satisfied when progressing.
- FAIL: List missing evidence, failed audits, tests not run, or unmet phase-gate. No partial passes.

## Validator Completion Checklist (MANDATORY for PACKET_FORMAT_VERSION >= 2026-03-15)
- [ ] I listed the exact spec clauses reviewed, not just the feature name.
- [ ] I recorded file:line evidence for each clause under `CLAUSES_REVIEWED`.
- [ ] I separated automation proof from manual code/spec review in the report.
- [ ] I recorded any blocked or unproven claims under `NOT_PROVEN` instead of implying completion.
- [ ] I set split verdicts (`GOVERNANCE_VERDICT`, `TEST_VERDICT`, `CODE_REVIEW_VERDICT`, `HEURISTIC_REVIEW_VERDICT`, `SPEC_ALIGNMENT_VERDICT`, `ENVIRONMENT_VERDICT`) deliberately rather than collapsing them into one PASS.
- [ ] If I used `SPEC_ALIGNMENT_VERDICT=PASS`, `NOT_PROVEN` is exactly `- NONE`.
- [ ] I compared against local `main` first (or documented why `origin/main` was needed instead).
- [ ] I performed an explicit heuristic-quality review and recorded residual risks instead of letting tests stand in for code judgment.
- [ ] I avoided stronger wording in chat/packet/audit than the split verdicts actually support.

## Operator UX: Explicit Verdict Line (HARD)
- When discussing a WP where the verdict is known, every Validator chat message MUST include an explicit single-line status near the top:
  - `VERDICT: PASS` or `VERDICT: FAIL`
- While validation is still in progress, use:
  - `VERDICT: PENDING`
- Do not require the Operator to infer the verdict from `NEXT_ACTION`, gate state, or prose.
- Strings like `accept`, `approved`, `technical pass`, or `looks good` are not legal verdicts.

## Validation Modes
- `Classical / Manual-Relay Validation`
  - This is the closure lane for non-orchestrator-managed work where the validator is operating from the regular validator checkout and owns governed validation end-to-end.
  - It may run `validator-gate-*`, append the canonical packet validation report, update closure state, and merge only when the full governed gate sequence authorizes it.
- `Governed Validation`
  - This is the orchestrator-managed validator lane.
  - `WP Validator` is advisory only in this lane. It may inspect code/spec drift, challenge weak reasoning, append receipts/thread guidance, and hand off or block, but it does not own final merge-to-`main` authority.
  - `Integration Validator` is the governed closure authority in this lane. It may run `validator-gate-*`, append the canonical packet validation report, update closure state, and merge only when the full governed gate sequence authorizes it.
  - After merge-to-main succeeds, the Integration Validator may execute an Orchestrator-generated single-target cleanup script for the merged CODER or WP_VALIDATOR local worktree only when:
    - the WP merge is already complete
    - the exact Operator approval text is supplied
    - the matching cleanup token from the target worktree is supplied
  - Manual filesystem deletion remains forbidden.
- `External / Independent Revalidation (orchestrator-managed WPs only)`
  - This is an audit mode, not a second validator workflow and not the classical/manual-relay closure lane.
  - Required start sequence:
    - `just validator-startup`
    - `just external-validator-brief WP-{ID}`
  - `just validator-startup` alone is insufficient for this mode.
  - This mode may audit code, governance, and environment, but it MUST NOT:
    - run `validator-gate-*`
    - mutate closure state
    - append normal governed-lane closure artifacts
    - merge or authorize merge in place of the Classical Validator or Integration Validator
  - Default write target for this mode is a chat report or a clearly labeled external revalidation report, not the normal governed-lane closure path.
  - ACP runtime note for orchestrator-managed WPs:
    - `wt-orchestrator` should no longer be dirtied by ACP/session/topology/WP-communication projections, because those runtime artifacts now default to the external repo-governance runtime root.
    - Dirty files limited to these surfaces are runtime-state evidence first, not automatic proof of governance failure:
      - `.GOV/roles_shared/runtime/validator_gates/WP-{ID}.json`
    - Before treating `wt-orchestrator` dirt as a governance defect, inspect ACP state with:
      - `just handshake-acp-broker-status`
      - `just session-registry-status WP-{ID}`
      - `just external-validator-brief WP-{ID}`
    - If those commands show expected runtime churn and the governed handoff path still passes, classify the dirt as runtime-state context, not packet-scope implementation drift.

## External Validator Split Report Contract
- Before an external/classical validator starts on an orchestrator-managed WP, generate the target contract with `just external-validator-brief WP-{ID}`.
- Governance target selection is derived from the packet-declared governance authority and workflow lane, not by assuming every case is `role_orchestrator`.
- External/classical validator reports for orchestrator-managed WPs MUST use these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `CODE_VERDICT: PASS | FAIL | NOT_RUN`
  - `GOVERNANCE_VERDICT: PASS | FAIL | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
- `LEGAL_VERDICT` is the only legal top-line verdict field. `CODE_VERDICT`, `GOVERNANCE_VERDICT`, `ENVIRONMENT_VERDICT`, and `DISPOSITION` are split assessments/classifications only.
- If the validator is in the wrong checkout or cannot access the committed PREPARE worktree source of truth, classify that as `VALIDATION_CONTEXT: CONTEXT_MISMATCH`, keep the blocked assessment at `NOT_RUN`, and use `LEGAL_VERDICT: PENDING` until the validation is rerun from the correct governance context.
- A `CONTEXT_MISMATCH` is not, by itself, proof that the WP implementation failed.
- If the WP remains correct for its baseline but SPEC_TARGET evolved materially, keep the legal verdict in `PASS | FAIL | PENDING` and set `DISPOSITION: OUTDATED_ONLY`.
- `OUTDATED_ONLY` is a disposition, not a legal top-line verdict.

## Governed Split Verdict Contract (MANDATORY for PACKET_FORMAT_VERSION >= 2026-03-15)
- Governed validation reports appended under `## VALIDATION_REPORTS` MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
  - `SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED`
- `LEGAL_VERDICT` remains the only legal top-line verdict field.
- `SPEC_ALIGNMENT_VERDICT` is not implied by passing tests or governance gates.
- If environment/tooling blocked full proof, reflect that explicitly with `ENVIRONMENT_VERDICT` and downgrade `SPEC_ALIGNMENT_VERDICT` rather than narrating a generic PASS.
- For governed PASS closure on this packet format, append `CLAUSES_REVIEWED` and `NOT_PROVEN` in the packet report itself; a standalone chat summary is insufficient.
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V2`, also append:
  - `MAIN_BODY_GAPS:` with `- NONE` only when no unresolved main-body requirement remains
  - `QUALITY_RISKS:` with `- NONE` only when no material maintainability or heuristic-quality concern remains
- `HEURISTIC_REVIEW_VERDICT=PASS` is legal only when `QUALITY_RISKS` is exactly `- NONE`.
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`, also append:
  - `MAIN_BODY_GAPS:` with `- NONE` only when no unresolved main-body requirement remains
  - `QUALITY_RISKS:` with `- NONE` only when no material maintainability or heuristic-quality concern remains
  - `VALIDATOR_RISK_TIER: LOW | MEDIUM | HIGH`
  - `DIFF_ATTACK_SURFACES:` with at least one diff-derived failure surface
  - `INDEPENDENT_CHECKS_RUN:` with validator-owned checks not copied from coder evidence
  - `COUNTERFACTUAL_CHECKS:` with concrete code-path / symbol references, not just test names
  - `BOUNDARY_PROBES:` for interface / producer-consumer / storage / contract boundary checks
  - `NEGATIVE_PATH_CHECKS:` for invalid, missing, adversarial, or failure-path checks
  - `INDEPENDENT_FINDINGS:` with deliberate independent findings or baseline-confirmation notes
  - `RESIDUAL_UNCERTAINTY:` with explicit remaining uncertainty; `- NONE` is illegal for `VALIDATOR_RISK_TIER=HIGH`
- `VALIDATOR_RISK_TIER` is validator-assigned and MUST NOT be lower than the packet `RISK_TIER`.
- `LEGAL_VERDICT=PASS` is legal only when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, and `COUNTERFACTUAL_CHECKS` are all present and non-empty.
- `VALIDATOR_RISK_TIER=HIGH` requires at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- `VALIDATOR_RISK_TIER=MEDIUM|HIGH` requires at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- The lightest valid counterfactual step is still mandatory: one sentence per key changed code path in the form "if X were removed or altered, Y would break", where `X` names a concrete file, symbol, or code path.

## Validation Gate Sequence [CX-VAL-GATE] (ONE REVIEW PAUSE; APPEND-FIRST)

The validation process MUST halt only at Gate 3 (final report presentation). All other gates are state recording/unlocks and must still be run in order.
State is tracked per WP in `.GOV/roles_shared/runtime/validator_gates/{WP_ID}.json`. Gates enforce minimum time intervals to prevent automation momentum.
(Legacy: `.GOV/reference/legacy/validator/VALIDATOR_GATES.json` is treated as a read-only archive for older sessions; new validations should not write to it.)

### Gate 1: WP APPEND (Records verdict; non-blocking)
1. Validator completes all checks and generates the full VALIDATION REPORT.
2. If verdict = PASS, before recording Gate 1 the Validator MUST update the WP closure state on the WP branch:
   - set task packet `**Status:** Done`
   - update `.GOV/roles_shared/records/TASK_BOARD.md` to `## Done` / `[VALIDATED]`
   - sync `.GOV/roles_shared/records/BUILD_ORDER.md` via `just build-order-sync`
3. Validator appends the VALIDATION REPORT to `.GOV/task_packets/{WP_ID}.md` (APPEND-ONLY per [CX-WP-001]).
4. Validator runs: `just validator-gate-append {WP_ID} {PASS|FAIL}`
5. Validator does **not** paste the full report to chat yet.

### Gate 2: COMMIT CLEARANCE (PASS only)
1. Only if verdict = PASS, Validator runs: `just validator-gate-commit {WP_ID}`
2. Validator performs `git commit` on the WP branch and records the commit SHA.
   - PASS requirement: this commit MUST include the appended report plus the Task Board / packet / build-order closure updates and any required Base-WP activation-state fixes (`WP_TRACEABILITY_REGISTRY`, removal of stale STUB state) so the later merge + fast-forward exposes the validated WP state in every active worktree.
   - PASS requirement: run `just gov-check` after those closure updates and before merge; a PASS commit without a passing governance check is incomplete.

### Gate 3: FINAL REPORT PRESENTATION (Blocking; the only mechanical pause)
1. If verdict = FAIL: run immediately after Gate 1, **before any remediation begins**.
2. If verdict = PASS: run after Gate 2 and after the validation report append is committed (**right before merge to `main` / push of `main`**).
3. Validator **outputs the entire report to chat** using the Report Template.
4. Validator runs: `just validator-gate-present {WP_ID}`
5. **HALT.** Validator MUST NOT merge to `main` / push `main` (PASS) or authorize remediation kickoff (FAIL) until the user acknowledges.

### Gate 4: USER ACKNOWLEDGMENT (Unlock)
1. User explicitly acknowledges the report (e.g., "proceed", "approved", "continue").
2. If user requests changes or disputes findings -> return to validation, re-run checks, regenerate report.
3. Validator runs: `just validator-gate-acknowledge {WP_ID}`
4. PASS: Validator may merge the validated WP into `main`. Canonical integration push remains `main` only; backup pushes are allowed only to the matching backup branch for the current role or WP when preserving state.
5. FAIL: WP remains open for remediation (no merge/commit).

### Gate Commands
```
just validator-gate-append {WP_ID} {PASS|FAIL}   # Gate 1: Record WP append + verdict
just validator-gate-commit {WP_ID}                # Gate 2: Unlock commit (PASS only)
just validator-gate-present {WP_ID} [PASS|FAIL]   # Gate 3: Record report shown (HALT)
just validator-gate-acknowledge {WP_ID}           # Gate 4: Record user ack (unlock)
just validator-gate-status {WP_ID}                # Check current gate state
just validator-gate-reset {WP_ID} --confirm       # Reset gates (archives old session)
```

**Violations:** Skipping Gate 1, committing without a PASS Gate 2, or merging to `main` / pushing `main` (PASS) / starting remediation (FAIL) without Gate 3+4 = PROTOCOL VIOLATION [CX-VAL-GATE-FAIL]. Gate commands will fail if the sequence is violated.

```
FLOW DIAGRAM:

  [Run all checks] --> [Generate Report Text]
                         |
                         v
                 GATE 1: APPEND TO WP (records verdict)
                         |
               +---------+----------+
               |                    |
             FAIL                 PASS
               |                    |
               v                    v
   GATE 3: SHOW REPORT IN CHAT   GATE 2: COMMIT CLEARANCE
               |                    |
               v                    v
             HALT              git commit (WP branch)
               |                    |
               v                    v
        GATE 4: ACKNOWLEDGE   GATE 3: SHOW REPORT IN CHAT
               |                    |
               v                    v
         remediation begins        HALT
                                    |
                                    v
                           GATE 4: ACKNOWLEDGE
                                    |
                                    v
                       merge to main / push main only
```

## Merge/Commit Authority (per Codex [CX-505])
- After issuing PASS **and completing all validation gates**, the validator form that currently owns closure authority is responsible for the integration flow into `main`.
- Closure authority split:
  - `Classical Validator` owns merge/push authority for classical/manual-relay validation.
  - `Integration Validator` owns merge/push authority for orchestrator-managed validation unless the packet explicitly overrides it.
  - `WP Validator` never owns merge-to-`main` authority.
- Validator responsibilities after PASS:
  - merge the validated WP branch into `main`
  - commit any required closure-sync or conflict-resolution edits on `main`
  - ensure the canonical closed `[VALIDATED]` state lives on `main`
- Pre-merge governance gate (MANDATORY): before merging the WP branch into `main`, the Validator MUST confirm `just gov-check` passes on the closure branch. Treat any activation-state drift (`WP_TRACEABILITY_REGISTRY`, Task Board STUB residue, stale build-order snapshot) as a merge-blocking failure, not post-merge cleanup.
- Coders must not merge their own work.
- Canonical push rule: only `main` is a canonical integration push target. Backup pushes to matching backup branches are allowed as safety copies, but they are not integration events.
- If a remote integration push is authorized, the Validator pushes `main` only after the merge is complete and `main` contains the final validated closure state.

## Post-Merge Cleanup (reduces branch confusion)
- Do NOT delete local WP branches or remote WP backup branches as routine cleanup.
- Any local or remote WP branch deletion requires explicit Operator approval naming the exact target(s).
- When deletion is approved for a presented exact action/target list, use the deterministic helper with `approved` or `proceed`:
  - `just close-wp-branch WP-{ID} "--remote" "approved"`

## Report Template
```
VALIDATION REPORT â€” {WP_ID}
Verdict: PASS | FAIL | OUTDATED_ONLY

Validation Claims (do not collapse into a single PASS):
- GATES_PASS (deterministic manifest gate on the committed handoff state, typically via `just validator-handoff-check {WP_ID}`; not tests): PASS | FAIL
- TEST_PLAN_PASS (packet TEST_PLAN commands, verbatim): PASS | FAIL | NOT_RUN
- VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH
- GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN
- DISPOSITION: NONE | OUTDATED_ONLY
- LEGAL_VERDICT: PASS | FAIL | PENDING
- SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED
- VALIDATOR_RISK_TIER: LOW | MEDIUM | HIGH

Scope Inputs:
- Task Packet: .GOV/task_packets/{WP_ID}.md (status: {status})
- Spec: {spec version/anchors}

Files Checked:
- {list of every file inspected during validation}

CLAUSES_REVIEWED:
- {SPEC clause identifier/text fragment} -> {path:line evidence}
- When `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`, use the exact clause text from `CLAUSE_CLOSURE_MATRIX` so the packet monitor and the report reconcile mechanically.

NOT_PROVEN:
- NONE
- {or list each unresolved clause/gap explicitly}

MAIN_BODY_GAPS:
- NONE
- {or list each unresolved main-body MUST/SHOULD gap explicitly}

QUALITY_RISKS:
- NONE
- {or list each maintainability / heuristic-quality risk explicitly}

DIFF_ATTACK_SURFACES:
- {diff-derived failure surface}

INDEPENDENT_CHECKS_RUN:
- {validator-owned check} => {observed result}

COUNTERFACTUAL_CHECKS:
- If `{path or symbol}` were removed or altered, {observable breakage / proof expectation} would break.

BOUNDARY_PROBES:
- {producer/consumer or boundary check the validator ran}

NEGATIVE_PATH_CHECKS:
- {invalid/missing/adversarial input or failure-path check}

INDEPENDENT_FINDINGS:
- {what the validator learned that was not copied from coder evidence}

RESIDUAL_UNCERTAINTY:
- {what still remains uncertain after review}

Findings:
- Requirement X: satisfied at {path:line}; evidence snippet...
- Hygiene: {clean | issues with details}
- Forbidden Patterns: {results of grep}
- Storage DAL Audit (if applicable): {results for CX-DBP-VAL-010..014}
- Architecture/RDD/LLM: {findings}
- Security/Red Team: {findings}

Tests:
- {command}: {pass/fail/not run + reason}
- Coverage note: {does disabling feature fail tests?}

Risks & Suggested Actions:
- {list any residual risk or missing coverage}
- {actionable steps for future work packets or immediate fixes}

Improvements & Future Proofing:
- {suggested improvements to the code or protocol observed during this audit}

Split-Verdict Rules:
- Use `SPEC_ALIGNMENT_VERDICT=PASS` only when every diff-scoped MUST/SHOULD clause claimed by DONE_MEANS + SPEC_ANCHOR is listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `NONE`.
- Use `HEURISTIC_REVIEW_VERDICT=PASS` only when `QUALITY_RISKS` is exactly `NONE`.
- Use `LEGAL_VERDICT=PASS` only when the report also records diff-derived attack surfaces, validator-owned independent checks, and concrete code-path counterfactuals.
- `VALIDATOR_RISK_TIER` is validator-assigned and must not downscope below the packet `RISK_TIER`.
- For `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- For `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, also reconcile the packet's live monitoring sections before PASS:
  - every `CLAUSE_CLOSURE_MATRIX` row must end `VALIDATOR_STATUS=CONFIRMED` (or `NOT_APPLICABLE`)
  - no row may remain `PENDING`
  - `SPEC_DEBT_STATUS` must be `OPEN_SPEC_DEBT=NO`, `BLOCKING_SPEC_DEBT=NO`, `DEBT_IDS=NONE`
- For `PACKET_FORMAT_VERSION >= 2026-03-16`, also inspect `SEMANTIC_PROOF_ASSETS` before PASS:
  - semantic tripwire tests must still target the landed contract
  - canonical contract examples must still match the emitted/consumed shape
  - each clause row must point to TESTS, EXAMPLES, or governed debt
- If tests pass but spec proof is incomplete, keep `TEST_VERDICT=PASS` and downgrade `SPEC_ALIGNMENT_VERDICT`.
- If the environment blocked full proof, record that in `ENVIRONMENT_VERDICT` instead of narrating an unconditional PASS.
 
Task Packet Update (APPEND-ONLY):
- [CX-WP-001] MANDATORY APPEND: Every validation verdict (PASS/FAIL) MUST be APPENDED to the end of the `.GOV/task_packets/{WP_ID}.md` file. OVERWRITING IS FORBIDDEN.
- [CX-WP-002] CLOSURE REASONS: The append block MUST contain a "REASON FOR {VERDICT}" section explaining exactly why the WP was closed or failed, linking back to specific findings.
- STATUS + closure updates are PASS-gated: append the full Validation Report for PASS/FAIL using the template below, but only after `verdict: PASS` may the Validator set task packet `**Status:** Done`, move TASK_BOARD to Done/Validated, and sync BUILD_ORDER (`just build-order-sync`). **DO NOT OVERWRITE User Context or previous history [CX-654].**
- For non-PASS governed verdicts or `DISPOSITION=OUTDATED_ONLY`, append the report but do not perform normal Done/Validated PASS closure updates on task packet/TASK_BOARD/BUILD_ORDER unless the governed lane explicitly records the outdated-only closure path.
- TASK_BOARD update (merge-visible requirement): for PASS, the Validator MUST update `.GOV/roles_shared/records/TASK_BOARD.md` on the WP branch before merge using `just task-board-set WP-{ID} DONE_VALIDATED`, and the closure commit MUST carry that update so merge + fast-forward makes the validated state visible in all role worktrees.
- TASK_BOARD update (on `main`): after merge, the canonical main-branch Task Board must already show the validated WP entry from that closure commit. Status-sync commits earlier in the WP lifecycle are separate and do not imply a verdict.
- Board consistency (on `main`): task packet `**Status:**` is source of truth; reconcile the Task Board to match packet reality before declaring PASS. Unresolved mismatch = FAIL pending correction.
- Activation consistency (merge-visible requirement): when validating an official packet, reconcile `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` and remove any stale `## Stub Backlog` entry for that Active Packet before merge; then run `just build-order-sync` and `just gov-check` so the official activation state is visible on `main` immediately after merge.
```

## Non-Negotiables
- Evidence over intuition; speculative language is prohibited [CX-588].
- [CX-WP-003] APPEND-ONLY WP HISTORY: Deleting or overwriting the status history in a Work Packet is a protocol violation. All verdicts must be appended.
- Automated review scripts are optional; manual evidence-based validation is required.
- If a check cannot be performed (env/tools unavailable), report as FAIL with reasonâ€”do not assume OK.
- No â€œpass with debtâ€ for hard invariants, security, traceability, or spec alignment; either fix or obtain explicit user waiver per protocol.
