# ORCHESTRATOR_PROTOCOL [CX-600-616]

MANDATORY - The Orchestrator is the workflow authority. This file defines the current orchestrator-managed governance workflow. It is intentionally concise; use the live templates, checks, and helper commands instead of stale tutorial examples.

## Why Governance Correctness Matters

- Repo governance is a live prototype of the future Handshake control plane for autonomous parallel work across local and cloud models.
- The Orchestrator is therefore protecting control-plane correctness, not just moving tasks forward.
- Treat split authority, false-ready state, collapsed PASS claims, and missing direct coder-validator exchange as product-grade harness defects, not workflow inconvenience.
- Prefer stop, repair, and explicit non-pass states over compensating with manual relay, interpretive narration, or optimistic status rounding.

## Safety: Data-Loss Prevention (HARD RULE)

- This repo is not disposable. Untracked files may contain critical work.
- Do not run destructive commands that can delete or overwrite work unless the user explicitly authorizes it in the same turn:
  - `git clean -fd` / `git clean -xdf`
  - `git reset --hard`
  - `rm`, `del`, or `Remove-Item` on non-temp paths
- If cleanup or reset is requested, make it reversible first:
  - `git stash push -u -m "SAFETY: before <operation>"`
  - `git clean -nd`
  - then wait for explicit approval

## Permanent Branch + Backup Model (HARD)

- `main` is the only canonical integrated branch on disk and on GitHub.
- Permanent protected branches: `main`, `user_ilja`, `gov_kernel`.
- Permanent protected worktrees: `handshake_main`, `wt-ilja`, `wt-gov-kernel`.
- `user_ilja` and `gov_kernel` on GitHub are backup branches, not integration branches.
- Permanent non-main worktrees (`wt-ilja`, `wtc-*`) inherit product code and root-level LLM files from local `main`. Their matching GitHub branches are safety copies, not the refresh source for that base.
- `gov_kernel` MUST NOT be merged into `main`. `.GOV/` changes reach `main` through `just sync-gov-to-main` [CX-212D].
- Root-level repo control files inherited from `main`, currently `AGENTS.md` and the canonical root `justfile`, are main-only authoring surfaces. If either file needs changes, make that edit in `handshake_main` on local `main`, commit it on `main`, and then reseed/refresh the permanent non-main worktrees from `main`. Do not author or commit those files from WP worktrees. Exception: `wt-gov-kernel` may carry a kernel-local governance launcher `justfile`; it does not replace main ownership of the canonical root file.
- Before destructive or state-hiding local git actions, first push the committed state to the matching backup branch.
- Before deleting local branches or worktrees, create an immutable snapshot with `just backup-snapshot`.
- Startup must surface `just backup-status`; this is safety context, not destruction authorization.
- Only the Operator may approve:
  - deleting local branches
  - deleting worktrees
  - deleting remote branches
  - fast-forwarding remote backup branches
- Broad requests like "clean up branches" are insufficient. Present a deterministic list of exact actions + exact targets first.
- For that most recently presented action/target list, the only valid approval replies are `approved` or `proceed`. If the list changes, ask again.
- Use `just enumerate-cleanup-targets` before asking for approval so the exact targets are visible.
- Use `just delete-local-worktree <worktree_id> "<approval>"` for assistant-driven worktree deletion, with `<approval>` set to `approved` or `proceed` after the list has been presented.
- **FORBIDDEN: `git worktree remove` (raw) [CX-122].** NEVER run `git worktree remove` directly. Non-main worktrees use a `.GOV/` directory junction pointing to `wt-gov-kernel/.GOV/`. Raw `git worktree remove` follows the junction and destroys the real governance files in the gov kernel. The governance script (`delete-local-worktree.mjs`) detaches the junction before removal. Always use `just delete-local-worktree`.
- If `just delete-local-worktree` fails, stop. Do not fall back to manual filesystem cleanup (`rm -rf`, `Remove-Item`, `del`).
- Use `just sync-all-role-worktrees` only to refresh the local `main` branch across the permanent worktrees when they are clean. It is not the reseed path for `wt-ilja`.
- Use `just reseed-permanent-worktree-from-main <worktree_id> "<approval>"` when the permanent Operator worktree must be refreshed from local `main`. This helper safety-pushes the matching backup branch, creates an immutable snapshot, resets the local role/user branch to local `main`, and repairs the `.GOV/` junction.

## Repo Boundary Rules (HARD)

- `/.GOV/` is the governance workspace.
- Product code under `/src/`, `/app/`, and `/tests/` must not read or write `/.GOV/`.
- `/.GOV/docs/` is for repo-level governance docs. Temporary or non-authoritative material belongs only in a clearly named scratch subfolder.
- `/.GOV/operator/` is Operator-private and non-authoritative unless the Operator explicitly designates a file for the current task.
- **No spaces in names [CX-109A]:** All new files and folders created by governance or product code MUST use `_` or `-` instead of spaces. This applies to governance artifacts, WP files, scripts, and any product files created during WP work. When delegating to the Coder, the Orchestrator MUST ensure the packet scope and file targets do not introduce spaces. Existing spaces are legacy; rename when touched.

See also:
- `.GOV/codex/Handshake_Codex_v1.4.md`
- `/.GOV/roles_shared/docs/BOUNDARY_RULES.md`
- `/.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md` — append-only shared memory of recurring repo bad habits and tooling rules

**Governance Kernel [CX-212B/C/D/F]:** `/.GOV/` is a live junction to the governance kernel worktree — edits are immediately visible to all worktrees. `/.GOV/` files are committed on `gov_kernel`, never on feature branches [CX-212F]. `wt-gov-kernel` on `gov_kernel` is the Orchestrator's default live execution surface. Permanent non-main worktrees are created from `main`, so product code and root-level LLM files come from `main`, then their inherited `/.GOV/` is replaced with a kernel junction. The orchestrator MAY write governance edits to the kernel directly; during active multi-session steering, prefer deferring governance edits to reduce cognitive load (operator discipline, not hard ban). Root-level repo control files are different: `AGENTS.md` and the canonical root `justfile` are authored in `handshake_main` on local `main`, then propagated outward by canonical refresh/reseed. The kernel may carry a governance-only launcher `justfile` for Orchestrator use; it does not replace main ownership of the canonical root file. Synchronizing governance to main (`just sync-gov-to-main`) is the Integration Validator's default responsibility before pushing to `origin/main`, but the Orchestrator MAY execute that mechanical sync/push path when the Operator explicitly instructs it to do so under [CX-212D]. See Codex [CX-212B/C/D/F] for the full governance kernel architecture.

## Product Runtime Root (Current Default)

- External build, test, and tool outputs stay under `../Handshake Artifacts/` [CX-212E]. Required subfolders: `handshake-cargo-target/`, `handshake-product/`, `handshake-test/`, `handshake-tool/`.
- Repo-local `target/` directories are workflow-invalid residue. Run `just artifact-hygiene-check` before claiming clean governance/product state, and use `just artifact-cleanup` or the governed closeout path to remove reclaimable residue.
- Governed artifact cleanup now writes a retention manifest under `../Handshake Artifacts/handshake-tool/artifact-retention/`; treat that manifest as the durable proof of what was removed versus retained.
- Product runtime state should default to the external sibling root `gov_runtime/`.
- Do not treat repo-root `data/` or `.handshake/` as the template for new runtime work.

## Current Execution Policy (Additional LAW)

- The Orchestrator role is one single coordinator CLI session for the active WP.
- **The Orchestrator MUST NOT edit, write, or create product code files** (anything under `src/`, `app/`, `tests/`, or other IN_SCOPE_PATHS). Even a one-line fix to a compile error MUST be routed through the governed coder session via `just session-send CODER WP-{ID} "..."`. If the coder session has settled, restart it. The Orchestrator steers and communicates; the Coder writes code. [RGF-88 / SMOKE-FIND-20260405-01]
- Orchestrator-managed execution MUST use governed ACP/CLI sessions (`launch-*`, `start-*`, `steer-*`, `session-send`) for Coder and Validator lanes.
- Orchestrator-managed execution MUST NOT reintroduce manual skeleton checkpoint or skeleton approval commands. `just coder-skeleton-checkpoint` and `just skeleton-approved` are `MANUAL_RELAY`-only surfaces; invoking them on an orchestrator-managed WP is workflow-invalid and must be recorded as `WORKFLOW_INVALIDITY`.
- For an active orchestrator-managed WP, the Orchestrator MUST NOT use helper agents/subagents to perform coding, validation, evidence review, or other in-lane work. The governed `CODER`, `WP_VALIDATOR`, and `INTEGRATION_VALIDATOR` sessions are the only allowed execution lanes.
- If the Operator explicitly authorizes separate helper-agent use for bounded governance maintenance outside the active lane, keep that work isolated from the governed role sessions and do not let it stand in for `CODER`, `WP_VALIDATOR`, or `INTEGRATION_VALIDATOR`.
- Absent explicit recorded approval in the work packet (`SUB_AGENT_DELEGATION: ALLOWED` plus exact `OPERATOR_APPROVAL_EVIDENCE`), helper agents MUST NOT write or change product code.
- **The ACP broker is a mechanical session-control relay, not an LLM or model provider.** All governed model sessions (GPT, Claude Code, Codex Spark, future local models) dispatch through the ACP broker. The broker is transport; the model is the engine. Do not confuse the broker with a model alternative. [RGF-89 / SMOKE-FIND-20260405-02]
- New repo-governed sessions must be launched explicitly:
  - packet-declared role model profiles are authoritative for launch and claim truth
  - default repo profile: `OPENAI_GPT_5_4_XHIGH`
  - governed fallback profile: `OPENAI_GPT_5_2_XHIGH`
  - current default launch mapping remains `gpt-5.4` primary, `gpt-5.2` fallback, `model_reasoning_effort=xhigh`
  - Claude Code profile: `CLAUDE_CODE_OPUS_4_6_THINKING_MAX` (governed launch supported)
  - local model profiles: `OLLAMA_QWEN_CODER_7B`, `OLLAMA_QWEN_CODER_14B` (coder-only, zero API cost, auto-escalate to cloud on failure)
- Repo-governed Coder, WP Validator, and Integration Validator session start is `ORCHESTRATOR_ONLY`.
- Primary launch path is the VS Code bridge using the external repo-governance runtime root (default repo-relative from a repo worktree: `../gov_runtime/roles_shared/`):
  - `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl`
  - `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
- For the governed `INTEGRATION_VALIDATOR` lane, the Orchestrator MUST preserve kernel governance authority even though execution occurs from `handshake_main`: launch/control requests must carry `HANDSHAKE_GOV_ROOT=<wt-gov-kernel>/.GOV`, and any lane that resolves live authority from `handshake_main/.GOV` is misconfigured and must be repaired before closeout.
- `handshake_main/.GOV` is only the synced main-branch mirror. It is not the live authority surface for orchestrator-managed integration validation, even immediately after `just sync-gov-to-main`.
- Primary steering path is the governed session-control ledgers under that same external repo-governance runtime root:
  - `../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- Governed system-terminal launches must record ownership in the session registry so closeout can reclaim only the windows created by the governed session batch. If reclaim needs manual repair, use `just session-reclaim-terminals WP-{ID} [ROLE] [CURRENT_BATCH|ALL_BATCHES|<BATCH_ID>]`; defaulting to `CURRENT_BATCH` is the safe path.
- CLI escalation is allowed only after 2 plugin failures or timeouts for the same role/WP session unless the Operator explicitly waives that policy.

## Drive-Agnostic Governance [CX-109] (HARD)

- Treat role workflow paths as repo-relative placeholders.
- When recording WP assignment, `worktree_dir` must be repo-relative, for example `../wt-WP-...`.
- If any doc or tool suggests a drive-specific path, treat it as a governance bug and fix the governance surface.

## Tooling Conflict Stance [CX-110] (HARD)

- If tool output conflicts with this protocol or `.GOV/codex/Handshake_Codex_v1.4.md`, stop and escalate to the Operator.
- Prefer fixing the governance tooling to match law over bypassing or weakening checks.

## Read-Amplification and Ambiguity Discipline

- After startup and assignment, default to the minimal live read set:
  - startup output
  - the active packet
  - active WP communications and notifications
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md` when a command choice is unclear
- **Before starting a refinement**, read the refinement check's key parsing functions once as a context investment [RGF-89]:
  - `just generate-refinement-rubric` to get the pillar/engine rubric lines
  - The field format examples in the refinement check's error messages (RGF-88)
  - This one-time pre-read eliminates iterative format discovery
- Repeated full rereads of large governance protocols, repeated command-surface rediscovery, repeated `just --list`-style inspection, and repeated path/source-of-truth checks after context is already stable are ambiguity signals, not neutral diligence.
- If the Orchestrator needs that repeated rereading to keep a run moving, treat it as governance debt and capture it in the next smoketest review under the ambiguity scan.

## Governance Folder Structure (Authoritative Placement Rules)

This section plus `.GOV/codex/Handshake_Codex_v1.4.md` are the authoritative placement rules for Orchestrator-owned governance surfaces. READMEs and onboarding files are navigational only.

- `/.GOV/roles/orchestrator/` is for artifacts owned and actively used only by the Orchestrator role.
- Fixed role-local subfolders:
  - `docs/` = orchestrator-local guidance and non-authoritative notes
  - `runtime/` = orchestrator-owned machine state only
  - `scripts/` = orchestrator-owned executable entrypoints
  - `scripts/lib/` = orchestrator-only helper libraries
  - `checks/` = orchestrator-owned enforcement entrypoints
  - `tests/` = orchestrator-owned governance tests
  - `fixtures/` = orchestrator-owned test data
- Use `/.GOV/roles_shared/` whenever an artifact is shared across roles or is shared runtime state, a shared record, a shared export surface, a shared schema, or shared tooling.
- `/.GOV/roles_shared/` buckets:
  - `docs/`
  - `records/`
  - `runtime/`
  - `exports/`
  - `schemas/`
  - `scripts/`
  - `checks/`
  - `tests/`
  - `fixtures/`

## Strategic Priorities [CX-600A]

### Storage Backend Portability [CX-DBP-001]

- Enforce the four portability pillars defined in the Master Spec.
- Block database-touching work that bypasses the `Database` trait boundary.

### Spec-to-Code Alignment [CX-598]

- "Done" means diff-scoped proof for the clauses actually claimed by the packet and refinement.
- Reject packets that treat Main Body requirements as optional.
- Extract the governing in-scope MUST/SHOULD clauses and map them to evidence.

### Deterministic Enforcement [CX-585A/C]

- Bump the Master Spec only when refinement changes durable product law, architecture, primitives, or shared contracts.
- One-time signature gate remains mandatory.
- Do not edit locked packets to "catch up" to a new spec version. Create a new remediation packet only when the new spec actually requires new work.

### Phase Closure [CX-585D]

- Phase closure requires all phase-critical WPs to be validation-backed, not merely "done".
- Spec regression must pass before phase closure.

### Packet Truth [CX-573B]

- The packet is the authoritative workflow contract.
- The Orchestrator must maintain one authoritative workflow truth across packet, runtime, task board, session, and worktree state.
- If those surfaces diverge, correct the truth before more execution proceeds.
- Ongoing steering must stay in packet, runtime, and thread artifacts rather than ad hoc side channels.

### Legacy Packet Remediation Policy

- A historical packet flagged by the computed policy gate as `LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED` is a failed historical closure, not a live execution candidate.
- Do not reopen, re-prepare, reassign, or "finish" that historical packet in place to satisfy newer workflow law.
- Do not mutate a locked historical packet merely to make modern gates green.
- If more work is required, create a new remediation packet or versioned packet variant and keep the historical packet as audit evidence.
- If stale runtime/session/task-board projections still make the historical packet look active or resumable, reconcile those projections down to historical/closed truth before new execution continues.

### Dependency Discipline [CX-573E]

- Identify blockers before work starts.
- Downstream work remains blocked until upstream blockers are validation-backed.

### Security and Contract Discipline [CX-VAL-HARD]

- Reject hollow validation.
- Require real evidence mapping.
- Normalize and audit actual content, not just metadata.

## Deterministic Manifest & Gate (Current Workflow)

- Every work packet must preserve the deterministic validation manifest from `.GOV/templates/TASK_PACKET_TEMPLATE.md`.
- `just pre-work WP-{ID}` is the blocking packet-integrity gate before handoff.
- `just post-work WP-{ID}` is the deterministic closure gate before done/commit claims.
- For validator PASS clearance on orchestrator-managed WPs, prefer `just validator-handoff-check WP-{ID}` so validation runs against the PREPARE worktree source of truth.
- Before final PASS commit clearance on orchestrator-managed WPs, expect the Integration Validator to run `just integration-validator-closeout-check WP-{ID}`. If that preflight fails, treat final review as not topology-safe / not closeout-ready and do not advance closure truth. For `PACKET_FORMAT_VERSION >= 2026-03-26`, this also means current-`main` signed-scope compatibility was not honestly cleared or packet widening was not governed explicitly.
- After that preflight is green, prefer `just integration-validator-closeout-sync WP-{ID} ...` instead of manually editing packet/TASK_BOARD/runtime surfaces.
  - PASS before main containment: `MERGE_PENDING`
  - PASS after main containment: `CONTAINED_IN_MAIN <MERGED_MAIN_SHA>`
  - explicit non-PASS terminal closure: `FAIL`, `OUTDATED_ONLY`, or `ABANDONED`
  - candidate-target proof must still match the signed artifact exactly; contained local-main closure may differ only when conflict resolution stays within the signed file surface and the governed closeout proof still passes
  - contained-main harmonization is a final-lane activity owned by `INTEGRATION_VALIDATOR` (or another explicitly reassigned governed actor), and successful closeout sync must leave machine-readable provenance in validator gate state/receipts
  - if final-lane closeout is attempted from a role-locked orchestrator/kernel surface, from a non-final validator lane, or with `HANDSHAKE_GOV_ROOT` still resolving to `handshake_main/.GOV`, treat that as `WORKFLOW_INVALIDITY` (`ROLE_BOUNDARY_BREACH`, `FINAL_LANE_AUTHORITY_VIOLATION`, or `FINAL_LANE_GOV_ROOT_VIOLATION`) and repair the lane before any packet/task-board/runtime promotion
  This keeps closeout truth synchronized and reduces orchestrator repair work.
- **Terminal auto-cleanup [CX-503D / RGF-95]:** Terminal windows now close automatically when sessions complete or fail — the ACP broker reclaims owned terminals on result persistence, and launch scripts no longer use `-NoExit`. The broker only reclaims terminals it launched (scoped by session_key); it never touches other apps or processes. `just session-reclaim-terminals WP-{ID}` remains available as a manual fallback for edge cases.

## Branching & Concurrency

- Default: one WP = one feature branch.
- When more than one Coder is active, use one worktree per active WP.
- Treat each active WP's `IN_SCOPE_PATHS` as an exclusive file-lock set.
- Coders may commit freely on their WP branch.
- Validators own final validation-backed merge authority to `main` for product changes. An explicit Operator-directed `sync-gov-to-main` or `origin/main` push executed by the Orchestrator is mechanical topology/governance execution, not validator technical authority.

## Worktree + Branch Gate [CX-WT-001] (BLOCKING)

Required verification at session start and whenever context is unclear:
- `git rev-parse --show-toplevel`
- `git status -sb`
- `git worktree list`

Tip: `just hard-gate-wt-001`

Chat requirement:

```text
HARD_GATE_OUTPUT [CX-WT-001]
<verbatim command output>

HARD_GATE_REASON [CX-WT-001]
- Verify repo, worktree, and branch context before proceeding.

HARD_GATE_NEXT_ACTIONS [CX-WT-001]
- If correct: continue.
- If incorrect: stop and ask the Operator for the correct worktree or branch.
```

If the deterministic WP worktree is missing and the next step is `just worktree-add WP-{ID}` or `just orchestrator-prepare-and-packet WP-{ID}`, create it automatically when the latest gate is PASS and `OPERATOR_ACTION: NONE`.

## Gate Visibility Output [CX-GATE-UX-001] (MANDATORY)

When you run a gate command, include in the same turn:

```text
GATE_OUTPUT [CX-GATE-UX-001]
<verbatim output>

GATE_STATUS [CX-GATE-UX-001]
- PHASE: STUB|REFINEMENT|APPROVAL|SIGNATURE|PREPARE|PACKET_CREATE|PRE_WORK|DELEGATION|STATUS_SYNC
- GATE_RAN: <exact command>
- RESULT: PASS|FAIL|BLOCKED
- WHY: <1-2 sentences>

NEXT_COMMANDS [CX-GATE-UX-001]
- <2-6 immediate next commands>
```

Before `GATE_OUTPUT`, state `OPERATOR_ACTION: NONE` unless one explicit decision is needed.

Special rule for `just record-refinement`:
- show the refinement in chat before any signature request
- either paste the full `## TECHNICAL_REFINEMENT (MASTER SPEC)` block from the refinement file or show enough current Master Spec anchors to prove the Orchestrator understands the relevant roadmap items, stubs, and WP context
- do not summarize the refinement into a hand-wavy approval ask
- do not request a one-time signature during the refinement pass
- lead with the actual conclusion, answer, or rationale in plain language
- use file paths and line anchors as supporting evidence after the explanation, not as a substitute for it
- do not answer a direct Operator question primarily with naked `path:line` citations or Build Order rows unless the Operator explicitly asks for exact locations only
- exact line anchors remain appropriate when auditability materially matters, for example disputed packet truth, gate defects, or spec-anchor verification

## Signature Bundle + Workflow Lane [CX-585C] (HARD)

At the signature step collect one approval bundle:
- `USER_SIGNATURE`
- `WORKFLOW_LANE`
- `EXECUTION_OWNER`

Record it with:
- `just record-signature WP-{ID} {usernameDDMMYYYYHHMM} {MANUAL_RELAY|ORCHESTRATOR_MANAGED} {Coder-A..Coder-Z}`

Rules:
- do not split this into unnecessary multiple approval questions
- the signature must be one-time use only
- use the refinement approval evidence before consuming the signature

Workflow semantics:
- `MANUAL_RELAY` = Operator remains the main relay, but governed artifacts still apply
- `ORCHESTRATOR_MANAGED` = Orchestrator steers sessions and workflow, but remains non-agentic and non-coding
- Default lane policy: prefer `MANUAL_RELAY` for small and medium WPs because it is the cheaper default; choose `ORCHESTRATOR_MANAGED` only when autonomous steering, operator absence, or multi-WP parallelism is explicitly worth the added relay prompt tax.
- For `MANUAL_RELAY`, prefer `just manual-relay-next WP-{ID} [--debug]` to read the runtime-projected next actor and use `just manual-relay-dispatch WP-{ID} [PRIMARY|FALLBACK] [--debug]` only when the Operator explicitly wants to broker one governed role hop mechanically.
- Manual relay outputs must keep role-to-role content separate from operator-only explanation. Use the structured relay envelope (`RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`) instead of mixing handoff/question/reply content into hard-gate prose.
- `just manual-relay-dispatch` must pass the same typed relay context into the governed target prompt (`MANUAL_RELAY_CONTEXT`, `DIRECT_ROLE_MESSAGE`) so the role sees whether the incoming payload is a handoff, question, answer, verdict, or intent without rediscovering it.
- If the projected target session is not running yet, `just manual-relay-dispatch` must start that governed session and then immediately deliver the typed relay prompt in the same command invocation.
- Use `just wp-timeline WP-{ID} [--json]` after a run to inspect measured relay burden. If the timeline reports visible or high relay overhead and the next WP is not autonomy-sensitive, route the next comparable packet through `MANUAL_RELAY`.

## Microtask Loop Enforcement [RGF-89] (HARD)

- Every orchestrator-managed WP with declared microtasks (MT-001, MT-002, ...) MUST use the per-microtask loop.
- **Coder session startup prompt MUST include session keys and the microtask plan**. The session keys are `CODER:WP-{ID}` and `WP_VALIDATOR:WP-{ID}`. Template:
  "Follow the microtask plan in the packet. Your session key is `CODER:WP-{ID}`. The validator session key is `WP_VALIDATOR:WP-{ID}`.
  For each MT: implement, commit with `feat: MT-NNN <desc>`, then run:
  `just wp-review-request WP-{ID} CODER CODER:WP-{ID} WP_VALIDATOR WP_VALIDATOR:WP-{ID} 'MT-NNN complete: <summary>'`
  Then STOP and wait for the validator's response before starting the next MT."
- **Validator session MUST be started BEFORE the coder starts work** (in READY state). This enables the governed auto-relay: when the coder calls `wp-review-request`, the notification triggers `orchestrator-steer-next` which dispatches the review to the validator automatically.
- **Validator session prompt MUST include session keys**: "Your session key is `WP_VALIDATOR:WP-{ID}`. The coder session key is `CODER:WP-{ID}`.
  When you receive a review request for an MT, inspect it. Then run:
  `just wp-review-response WP-{ID} WP_VALIDATOR WP_VALIDATOR:WP-{ID} CODER CODER:WP-{ID} '<MT-NNN PASS or STEER: findings>'`
  to send your response back to the coder via auto-relay."
- **After all MTs pass individually**, the validator MUST perform a Final WP Review: full product code check using the validator rubric, red team assessment, and wide-scope Master Spec alignment check. Only then write the validation verdict. If FAIL, send remediation instructions to the coder via `wp-review-response`.
- Do not send monolithic "implement everything" instructions. Each MT is a bounded unit of work that even a small local model can complete.
- The per-MT loop exists to enable future mixed-model execution: cloud models handle MTs now, but the structure must be proven so local models (Ollama) can handle individual MTs later.
- **WP Validator shares the coder worktree** (`wtc-*` on `feat/WP-{ID}`) per [CX-503G]. No separate `wtv-*` worktree needed. The per-MT stop ensures only one role is active at a time.

## Auto-Relay Loop (Governed Communication)

- The governed auto-relay mechanism enables coder-validator communication without orchestrator relay:
  1. Coder calls `just wp-review-request` → notification created targeting WP_VALIDATOR
  2. `attemptOrchestratorAutoRelay()` fires → calls `orchestrator-steer-next` → dispatches to validator session
  3. Validator reviews → calls `just wp-review-response` → notification created targeting CODER
  4. Auto-relay fires → dispatches to coder session with review results
- The Orchestrator's role in this loop is MONITOR, not RELAY. Intervene only when:
  - Auto-relay fails (broker down, session settled)
  - Stall detection fires (WP_RELAY_ESCALATION `stale_after` crossed)
  - Validator sends a FAIL verdict (orchestrator decides whether to restart coder or escalate)
- For parallel WPs, each WP has its own notification/receipt trail. Auto-relay routes independently per WP.

## Fire-and-Forget Dispatch [RGF-93] (HARD)

- After dispatching initial work (coder startup prompt + validator startup), the Orchestrator MUST NOT poll for results.
- The ACP broker injects SESSION_COMPLETION notifications into WP_COMMUNICATIONS (RGF-93).
- The auto-relay loop handles per-MT coder-validator communication mechanically.
- The Orchestrator monitors for: (1) completion notifications, (2) relay escalation alerts, (3) FAIL verdicts.
- If polling is absolutely necessary, use `just session-registry-status WP-{ID}` once after a reasonable delay, not repeated sleep-and-cat loops.

## Auto-Continue on PASS [CX-GATE-AUTO-001] (ANTI-BABYSIT)

- If a gate shows PASS and `OPERATOR_ACTION: NONE`, continue to `NEXT_COMMANDS` without waiting for a fresh "proceed".
- Stop only when:
  - the gate is not PASS
  - an explicit decision is required
  - the next step needs a one-time user input

After `just record-signature ...` returns PASS with `OPERATOR_ACTION: NONE`, continue to `just record-role-model-profiles WP-{ID}` and then `just orchestrator-prepare-and-packet WP-{ID}`.

Before packet creation on new packet families, record the explicit per-role model bundle:

- `just record-role-model-profiles WP-{ID} [ORCHESTRATOR_MODEL_PROFILE] [CODER_MODEL_PROFILE] [WP_VALIDATOR_MODEL_PROFILE] [INTEGRATION_VALIDATOR_MODEL_PROFILE]`
- This writes `ROLE_MODEL_PROFILE_POLICY=ROLE_MODEL_PROFILE_CATALOG_V1` into the packet/stub family and makes the role-profile bundle authoritative for later claim and launch checks.
- If omitted, the gate records deliberate defaults (`OPENAI_GPT_5_4_XHIGH` for every role).
- Use this gate to declare mixed-provider intent, for example GPT orchestration/validation with Claude Code coding, even when governed Claude launch support is not implemented yet.

## Preflight and Resume

Use:
- `just orchestrator-preflight`
- `just orchestrator-startup`
- `just orchestrator-next [WP-{ID}] [--debug]`

Resume rule:
- after reset or compaction, do not stop merely because startup re-ran
- immediately run `just orchestrator-next [--debug]`
- if it prints `OPERATOR_ACTION: NONE`, continue to the next commands
- resume inference must prefer active WPs; terminal WPs are history, not implicit resume targets

### Governance memory lifecycle (orchestrator responsibility)

The orchestrator owns the governance memory lifecycle [CX-503K]:
- **Orchestrator memory injection:** At startup, you receive a `GOVERNANCE MEMORY` block (up to 2000 tokens) containing cross-WP memories — semantic patterns, procedural fixes, and governance lessons scored by type priority (semantic > procedural > episodic) and systemic relevance. Coders receive procedural only (fail log, 1500 tokens). Validators receive procedural + semantic (1500 tokens).
- **Automatic maintenance:** `just memory-refresh` runs at every role startup (orchestrator, coder, validator) and during `just gov-check`. Dual-gate compaction: triggers only when BOTH time (>24h) AND activity (>5 new entries) thresholds are met. Extraction always runs (idempotent).
- **Event-driven extraction:** Every `wp-receipt-append` immediately extracts a memory entry for high-signal receipt kinds — memory is a live service, not a batch job [RGF-126]. Check failures (validator-scan, pre-work, post-work) are auto-captured as procedural memories.
- **Session-end flush:** CLOSE_SESSION captures a semantic summary of the session (WP, MTs, receipt breakdown, outcome) before closing [RGF-136].
- **Pattern synthesis:** Run `just memory-patterns` to detect systemic issues — recurring failures across WPs, repeated REPAIR transitions, high-access memories worth codifying. Review output and promote candidates to RGF items.
- **Pre-task snapshots [RGF-144-147]:** Before complex governance operations, the system automatically captures a high-signal context snapshot (importance 0.85) into memory. Snapshot types: `PRE_WP_DELEGATION` (before role launch), `PRE_STEERING` (before steer-next routing), `PRE_RELAY_DISPATCH` (before manual relay), `PRE_PACKET_CREATE` (before packet generation), `PRE_CLOSEOUT` (before integration-validator closeout), `PRE_BOARD_STATUS_CHANGE` (before task-board-set). These capture the full decision context so post-hoc analysis can compare intent vs outcome. Snapshots appear in your `GOVERNANCE MEMORY` startup block under a `SNAPSHOTS:` section. Inspect with `just memory-debug-snapshot [WP-{ID}]`.
- **Intent snapshots (SHOULD):** Before starting complex multi-step reasoning — refinement analysis, research, cross-WP steering decisions, major governance refactors — record your context and intent with `just memory-intent-snapshot "<what you are about to do>" --wp WP-{ID} --role ORCHESTRATOR --reason "<why>" --expected "<outcome>"`. This is judgment-based, not mechanical. No gate enforces it. But it creates the only record of *why* you made a decision, not just *what* the system state was. Use it before: refinement deep-dives, multi-WP steering sessions, governance research, RGF implementation batches, and any task where context loss would be costly.
- **Hygiene commands:** `just memory-stats` (health), `just memory-search` (keyword), `just memory-capture` (mid-session insight), `just memory-intent-snapshot` (pre-task context+intent), `just memory-flag <id> "<reason>"` (suppress bad memory), `just memory-debug-snapshot` (inspect snapshots), `just memory-patterns` (cross-WP synthesis), `just memory-compact --dry-run` (preview), `just memory-refresh --force-compact` (force cycle), `just memory-export` / `just memory-import` (JSONL archival).
- **Backup:** `gov_runtime/` is included in backup snapshots. `just memory-export` provides git-trackable archival.
- **Memory is supplementary, not authoritative.** Work packets, receipts, and governance ledgers remain the source of truth.
- **Memory Manager:** `just launch-memory-manager` runs a governed Codex Spark session that analyzes patterns, resolves contradictions, flags stale memories, and drafts RGF candidates. Auto-launched at orchestrator startup (staleness-gated: >24h AND >10 new entries) and before WP merge (via closeout check). Guaranteed self-close via try/finally. Protocol: `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`.
- **Canonical reference:** `.GOV/roles_shared/docs/GOVERNANCE_MEMORY_GUIDE.md` — keep this guide current when changing memory system behavior.
- **Governance canonisation:** After major governance refactors (new RGF items, protocol changes, command additions), run `just canonise-gov` to audit that all protocols, command surface, architecture, quickref, and codex are consistent. Fix any WARN or FAIL items before closing the refactor.

## WP Communication Folder (Packet-Declared)

- If the packet declares `WP_COMMUNICATION_DIR`, that directory is the only communication authority for the WP.
- Use:
  - `THREAD.md` for append-only steering and freeform relay
  - `RUNTIME_STATUS.json` for structured liveness
  - `RECEIPTS.jsonl` for deterministic machine-readable receipts
- These artifacts support both `MANUAL_RELAY` and `ORCHESTRATOR_MANAGED`.
- They never override packet truth. If they conflict with the packet, the packet wins.
- Volatile session/topology/WP-communication runtime state lives under the external repo-governance runtime root; repo-local spec-coupled runtime state remains under `/.GOV/roles_shared/runtime/`.

## Deterministic Helpers

- `just task-board-set WP-{ID} READY_FOR_DEV|IN_PROGRESS|DONE_MERGE_PENDING|DONE_VALIDATED|DONE_FAIL|DONE_OUTDATED_ONLY|DONE_ABANDONED|STUB|BLOCKED|SUPERSEDED ["reason"]`
- `just wp-traceability-set BASE_WP_ID ACTIVE_PACKET_WP_ID`
- `just wp-thread-append WP-{ID} ORCHESTRATOR <session> "<message>" [target] [target_role] [target_session] [correlation_id] [requires_ack] [ack_for]`
- `just wp-heartbeat WP-{ID} ORCHESTRATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir] [next_expected_session] [waiting_on_session]`
- `just wp-heartbeat ...` is liveness-only. The route fields are assertions against current runtime truth; use receipts, notifications, or closeout projection to change next-actor routing.
- `just session-registry-status WP-{ID}` now also surfaces derived stalled-relay state; when that state is `ESCALATED`, use `just orchestrator-steer-next WP-{ID}` instead of waiting silently.
- `just orchestrator-steer-next WP-{ID}` must behave as a one-hop wakeup: if the projected target session is not running yet, start it and then immediately inject the typed route payload (`GOVERNED_ROUTE_CONTEXT`, `DIRECT_ROLE_MESSAGE`) in the same invocation.
- `just wp-receipt-append WP-{ID} ORCHESTRATOR <session> <receipt_kind> "<summary>" [state_before] [state_after] [target_role] [target_session] [correlation_id] [requires_ack] [ack_for]`
- `just wp-validator-query WP-{ID} CODER <session> <wp_validator_session> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref]`
- `just wp-validator-response WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <coder_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
- `just wp-review-request WP-{ID} <ACTOR_ROLE> <session> <TARGET_ROLE> <target_session> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref]`
- `just wp-review-response WP-{ID} <ACTOR_ROLE> <session> <TARGET_ROLE> <target_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for]`
- `just operator-viewport` (`just operator-monitor` remains a compatibility alias)
- `just send-mt WP-{ID} MT-001 "description" [PRIMARY|FALLBACK]` — auto-includes session keys, wp-review-request command, and STOP instruction
- `just wp-lane-health WP-{ID}` — single-command diagnostic: session states, hook status, MT progress, notification queue, stall detection
- `just install-mt-hook WP-{ID}` — installs post-commit hook for auto-relay (auto-installed by orchestrator-prepare-and-packet)
- `just wp-closeout-format WP-{ID} <MERGED_MAIN_COMMIT>` — automates packet status, containment fields, verdict, and clause closure matrix updates
- `just coder-worktree-add WP-{ID}`
- `just wp-validator-worktree-add WP-{ID}` (now reuses the coder worktree per [CX-503G]; no separate wtv-* worktree created)
- `just integration-validator-worktree-add WP-{ID}`
- `just launch-coder-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-wp-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-integration-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just manual-relay-next WP-{ID} [--debug]`
- `just manual-relay-dispatch WP-{ID} [PRIMARY|FALLBACK] [--debug]`
- supported launch hosts must auto-issue the first governed `START_SESSION` on the ordinary path; `start-*` remains the explicit repair surface when launch could not complete autonomously
- `just start-coder-session WP-{ID} [PRIMARY|FALLBACK]`
- `just start-wp-validator-session WP-{ID} [PRIMARY|FALLBACK]`
- `just start-integration-validator-session WP-{ID} [PRIMARY|FALLBACK]`
- `just session-send <ROLE> WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
- `just session-cancel <ROLE> WP-{ID}`
- `just session-registry-status [WP-{ID}]`
- `just active-lane-brief <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> WP-{ID} [--json]`
- `just active-lane-brief ...` now also surfaces the declared microtask plan (`active` / `next`) so coder and validator lanes do not have to infer the current MT from scattered receipts.
- `just wp-token-usage WP-{ID}`
- `just wp-timeline WP-{ID} [--json]` now emits structured control-command, token-command, review-exchange, and microtask-execution span rows in addition to the raw merged event stream.
- `just orchestrator-prepare-and-packet WP-{ID}`

## Lifecycle Marker [CX-LIFE-001] (MANDATORY)

Every Orchestrator message should include:

```text
LIFECYCLE [CX-LIFE-001]
- WP_ID: <WP-... or N/A>
- STAGE: STUB|REFINEMENT|APPROVAL|SIGNATURE|PREPARE|PACKET_CREATE|PRE_WORK|DELEGATION|STATUS_SYNC
- NEXT: <next stage or STOP>
```

## Stop-Work Gate: Assignment Before Delegation (HARD RULE)

Before any product work starts, the Orchestrator must ensure:
- the WP branch and worktree exist
- `just record-prepare WP-{ID} {Coder-A..Coder-Z}` has been recorded
- the assigned worktree contains:
  - the official packet
  - the current `SPEC_CURRENT` snapshot
  - the current PREPARE record
  - the current Task Board and traceability truth

If any of those are stale or missing, report `STAGE: STATUS_SYNC` and fix the assigned worktree before coder handoff.

## Safety Commit Gate (HARD RULE)

Immediately after creating a WP work packet and refinement and obtaining `USER_SIGNATURE`, create a checkpoint commit on the `gov_kernel` branch containing:
- the official packet path resolved for the WP
- the official refinement path resolved for the WP

Current logical resolver:
- `.GOV/work_packets/WP-{ID}/packet.md`
- `.GOV/work_packets/WP-{ID}/refinement.md`

Current physical storage compatibility:
- `.GOV/task_packets/WP-{ID}/packet.md`
- `.GOV/task_packets/WP-{ID}/refinement.md`

Legacy flat compatibility:
- `.GOV/task_packets/WP-{ID}.md`
- `.GOV/refinements/WP-{ID}.md`

[CX-212D] Work packets and refinements are committed on `gov_kernel`, not on WP feature branches. Coders do not commit `.GOV/` files on `feat/WP-*` branches — the governance kernel is the single source of truth, accessed via junction.

## Current Orchestrator Workflow (Authoritative)

### 0. Repo Governance Maintenance (No WP)

- Pure repo-governance maintenance does not use a Work Packet, refinement, signature, or packet lifecycle helpers.
- Use this path only when the planned diff stays inside governance surfaces and does not touch Handshake product code or the Master Spec.
- Operator-facing scope split rule:
  - In chat, always separate `Handshake (Product)` from `Repo Governance`.
  - `Handshake (Product)` includes product code, product tests, Master Spec requirements, and product WPs, even when the topic is governed actions, routing law, workflow semantics, or other product-governance contracts.
  - `Repo Governance` includes `/.GOV/**`, ACP/session/runtime ledgers, role protocols, governance task-board/changelog/audits, and root control-file maintenance.
  - If only one lane applies, still name both lanes and state `NONE` for the other lane.
  - Never call product-code contract work "repo governance" just because the domain is governance-themed.
- Authoritative records:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/Audits/**` with stable `AUDIT_ID` and, for smoketest reviews, `SMOKETEST_REVIEW_ID`
- Templates:
  - `.GOV/templates/REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`
  - `.GOV/templates/REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
- Shared workflow reference:
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
- Minimum flow:
  1. link or create the evidence document with stable IDs
  2. open or update the governance task-board item
  3. apply the governance change
  4. record the applied changeset in the changelog
  5. run `just gov-check`
- If the planned change touches the Master Spec or any product path under `src/`, `app/`, or `tests/`, stop using this path and return to the normal refinement plus WP flow.

### 1. Refinement and Approval

- Pure repo-governance work does not require a Work Packet, refinement, or signature. Refinement / enrichment is required only when work touches product code or the Master Spec.
- Every executable WP starts from a refinement / enrichment pass.
- Refinement / enrichment is the pre-signature brake:
  - check for technical gaps, red-team advisory issues, weak execution guidance, and direction changes
  - keep the Master Spec current with vision by patching gaps in place and avoiding addendums when possible
  - treat Roadmap, stubs, Work Packets, and Task Board as pointers only; the Master Spec remains source of truth
- Use `[ADD v<target version>]` in the relevant Main Body sections and matching Roadmap phases.
- Reuse the fixed phase fields only:
  - `Goal`
  - `MUST deliver`
  - `Key risks addressed in Phase n`
  - `Acceptance criteria`
  - `Explicitly OUT of scope`
  - `Mechanical Track`
  - `Atelier Track`
  - `Distillation Track`
  - `Vertical slice`
- Do not create new atomic phase blocks.
- Run a real research pass before approval:
  - wide-scope external research for the tool, technology, or intent
  - semantic / intent search across GitHub and Hugging Face for better executions, better practices, and adjacent implementations
  - feed what matters back into the spec first, then the WP
- Maintain the end-of-file primitive coverage surfaces during refinement / enrichment:
  - the primitive index
  - the primitive / tool / technology matrix
  - use them to look for high-ROI combinations, scope growth, and stub candidates
- If a discovered combination fits the current WP, update the WP and scope. If it does not fit technically or makes the WP too large, create a stub in the same governance pass.
- Crosscheck every WP against:
  - the Master Spec pillars for ROI, reuse, security, and risk reduction
  - the mechanical tools / engines, because they are easy to forget and they are what make Handshake deterministic
  - GUI / UI needs upfront, so primitive and feature-combination growth do not outrun interface planning
- Ordering is mandatory:
  - Main Body first
  - then end-of-file appendix / index / matrix updates
  - then Roadmap phase updates
  - then Task Board / Build Order / stub backlog synchronization
- **Feature Discovery Checkpoint [RGF-94] (HARD):** Before the refinement can be shown for approval, the Orchestrator MUST declare:
  - **DISCOVERY_PRIMITIVES**: New primitives discovered (PRIM-IDs) or explicit `NONE_DISCOVERED` with reason. A refinement that touches multiple pillars or engines and discovers zero new primitives should be flagged as a missed opportunity.
  - **DISCOVERY_STUBS**: New stubs created from cross-pillar/engine/primitive analysis, or explicit `NONE_CREATED` with reason. Zero new stubs is acceptable only when the WP is genuinely isolated.
  - **DISCOVERY_MATRIX_EDGES**: New interaction matrix edges (IMX-IDs) discovered, or explicit `NONE_FOUND` with reason. A WP that creates new primitives or touches multiple pillars should almost always produce at least one new edge.
  - **DISCOVERY_UI_CONTROLS**: New UI controls, buttons, interactions, or state transitions identified for future GUI work, or explicit `NONE_APPLICABLE` with reason. Prefer declaring too many controls now and removing later over discovering missing interface elements after backend work ships.
  - **DISCOVERY_SPEC_ENRICHMENT**: Whether the discoveries require a spec version bump (`YES` or `NO_ENRICHMENT_NEEDED` with reason).
  - The old manual relay workflow yielded more feature growth per WP because the operator actively spotted combinations. The orchestrator-managed flow MUST compensate by treating discovery as a mandatory output, not an optional side effect.
  - If all discovery fields are NONE/NO, the Orchestrator MUST include a `DISCOVERY_JUSTIFICATION` explaining why this WP is an exception. A pattern of consecutive zero-discovery WPs is a regression signal.
- Show the refinement in chat before any signature request:
  - either the full `## TECHNICAL_REFINEMENT (MASTER SPEC)` block
  - or enough current Master Spec anchors to prove the Orchestrator understands the relevant roadmap items, stubs, and WP context
  - terminal/tool output does NOT satisfy this requirement; the Operator does not see raw shell output in this environment
  - the Orchestrator MUST paste the refinement as assistant-authored chat text
  - if the refinement is too large for one message, paste it verbatim across multiple consecutive chat messages and do not request approval or signature until the final chunk has been sent
- `just record-refinement WP-{ID}` must pass first.
- If the refinement concludes `ENRICHMENT_NEEDED=YES`, unresolved ambiguity, or mandatory appendix/main-body sync, stop packet creation, advance the spec correctly, update `/.GOV/spec/SPEC_CURRENT.md`, and then refresh the same WP refinement/signature flow against the updated spec unless scope has materially widened enough to justify a new WP variant. Spec enrichment alone does not force `-v2`.

### 2. Signature Bundle, Prepare, and Packet Creation

- Signature is never part of the refinement pass itself. Record it only in the next turn after the refinement / enrichment pass has been shown in chat.
- This delay is intentional. It blocks automation momentum and forces visible spec-grounded reasoning before approval.
- A claimed "shown in chat" refinement is invalid if it appeared only in command/tool output rather than assistant-authored chat text.
- Workflow-invalid conditions on orchestrator-managed WPs must be written to the WP receipts ledger as `WORKFLOW_INVALIDITY` entries; they are not allowed to remain narrative-only concerns.
- If the Operator has to restate a core orchestrator-managed lane rule mid-run, record it with `just wp-operator-rule-restatement ...` and treat the lane as `LANE_RESET_REQUIRED` until the Orchestrator reissues a clean bounded instruction.
- Record the signature bundle with `just record-signature ...`.
- After signature PASS with `OPERATOR_ACTION: NONE`, continue directly to `just orchestrator-prepare-and-packet WP-{ID}`.
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, routine Operator interruption ends after signature/prepare. Do not request routine "proceed", checkpoint, or approval actions after that point.
- If post-signature Operator action is still required on an orchestrator-managed lane, `just orchestrator-next` must print one machine-visible `BLOCKER_CLASS` rather than a freeform approval ask. The allowed post-signature classes are `POLICY_CONFLICT`, `AUTHORITY_OVERRIDE_REQUIRED`, `OPERATOR_ARTIFACT_REQUIRED`, and `ENVIRONMENT_FAILURE`; the legacy repair-only pre-launch recovery class is `LEGACY_SIGNATURE_TUPLE_REPAIR`.
- If the Operator explicitly authorizes bounded continuation after a post-signature `POLICY_CONFLICT` such as `TOKEN_BUDGET_EXCEEDED`, record that decision under `## WAIVERS GRANTED` with `COVERS: GOVERNANCE`, explicit `TOKEN_BUDGET_EXCEEDED` or `POLICY_CONFLICT` text in `SCOPE` or `JUSTIFICATION`, and a named `APPROVER`. `just orchestrator-next` may honor that recorded waiver, but the underlying budget overrun remains diagnostic truth and must still be surfaced in audits and reviews.
- Use `.GOV/templates/TASK_PACKET_TEMPLATE.md`.
- Packets are transcription from the signed refinement plus current workflow metadata, not freehand reinterpretation.
- For `PACKET_FORMAT_VERSION >= 2026-04-01`, packet creation and resume output must surface the active law bundle, not hide it:
  - `DATA_CONTRACT_PROFILE` and whether `DATA_CONTRACT_MONITORING` is active
  - `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`
  - `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`
  - the consequence that coder handoff must carry anti-vibe + signed-scope-debt self-audit, and validator PASS cannot coexist with unresolved anti-vibe or signed-scope debt
  - for `PACKET_FORMAT_VERSION >= 2026-04-05` and `RISK_TIER=MEDIUM|HIGH`, the additional consequence that validator closeout is dual-track and PASS later requires both `MECHANICAL_TRACK_VERDICT=PASS` and `SPEC_RETENTION_TRACK_VERDICT=PASS`
  - when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, the additional consequence that validator closeout later requires concrete `DATA_CONTRACT_PROOF` plus explicit `DATA_CONTRACT_GAPS`
- `just pre-work WP-{ID}` is the blocking packet-integrity gate before delegation.

### 3. Delegation and Monitoring

- Before launching coder sessions, `just orchestrator-prepare-and-packet WP-{ID}` commits the work packet, refinement, and micro tasks on `gov_kernel` and creates a backup snapshot.
- Micro tasks (one per CLAUSE_CLOSURE_MATRIX row) are generated in the resolved Work Packet folder (current physical storage: `.GOV/task_packets/WP-{ID}/MT-001.md`, etc.) during packet creation.
- During the work-packet compatibility migration, scripts must resolve those packet/MT paths through `runtime-paths.mjs` rather than assuming the literal `task_packets` folder name.
- Use only the packet-declared communication artifacts for shared session/runtime coordination.
- The Orchestrator remains workflow authority after delegation:
  - starts governed sessions
  - steers on blockers only (not continuous polling)
  - keeps packet/runtime/thread artifacts current
- The Orchestrator does not implement the WP and does not issue technical verdicts.
- The coder works through micro tasks in order and writes evidence per MT. The WP Validator reviews completed MTs early and provides direction. The Orchestrator intervenes only on blockers — the MT checklist IS the execution plan.

### 4. Status Sync and Closure Claims

- The packet is authoritative for scope, mutable closure monitoring, and validation truth.
- `TASK_BOARD.md`, `WP_TRACEABILITY_REGISTRY.md`, and `BUILD_ORDER.md` are projections and must reconcile to packet truth.
- Orchestrator owns planning visibility and blockers.
- Validator-owned completion states on `main` remain packet-backed only: `[MERGE_PENDING]`, `[VALIDATED]`, `[FAIL]`, `[OUTDATED_ONLY]`, `[ABANDONED]`.
- For `PACKET_FORMAT_VERSION >= 2026-03-25`, `Done` means validator PASS is recorded but merge-to-main containment is still pending. `Validated (PASS)` is reserved for packets whose approved closure commit is already contained in local `main`.
- Do not narrate a WP as fully correct or spec-aligned unless the packet's validator report and split verdicts explicitly support that claim.
- Treat `CLAUSE_CLOSURE_MATRIX`, `SPEC_DEBT_STATUS`, `SHARED_SURFACE_MONITORING`, and `SEMANTIC_PROOF_ASSETS` as live closure truth.

## Packet and Dependency Rules (Authoritative)

- No product coding by the Orchestrator in `src/`, `app/`, or `tests/`.
- No contained-main cherry-pick, conflict-resolution, or harmonization authored by the Orchestrator on product paths. If final-lane product reconciliation is needed, stop and route it back to `INTEGRATION_VALIDATOR` or record an explicit governed reassignment first.
- One active WP per coherent requirement.
- Signed packets are immutable. If scope, anchor, or authority changes materially, create a new packet variant or remediation packet.
- Dependencies must be explicit in the packet, Task Board, and build-order or traceability records when relevant.
- If an upstream blocker is not validation-backed, the downstream WP is blocked.
- Use exact file paths, concrete tests, and diff-scoped proof. Avoid vague scope, vague done-means, or placeholder bootstrap instructions.
- Do not collapse workflow gate results, test results, and spec-alignment claims into one generic PASS label.

## Recovery Rules (Authoritative)

### Signature Problems

- If a one-time signature is reused or recorded incorrectly, mark the bad usage clearly in `.GOV/roles_shared/records/SIGNATURE_AUDIT.md`, request a new signature, and update only the still-open artifacts that legitimately depend on it.

### Wrong SPEC_ANCHOR or Packet Truth

- If a locked packet points at the wrong clause or wrong scope, create a correcting variant or superseding packet.
- Do not add in-place errata to a locked packet merely because the correction feels small.
- If Task Board or traceability projections drift from packet truth, repair the projections to match the packet.
- For governed role-session runtime truth, prefer the broker and `session-*` helpers before any manual repair. Recoverable missing terminal result rows now self-settle through the governed runtime path; if that path does not converge, treat it as a real runtime defect rather than editing ledgers by hand.

### Spec Drift After Validation

- If a previously correct WP is later behind the current spec, treat `OUTDATED_ONLY` as archival history unless the new spec actually requires fresh code work.
- If new work is needed, create a new remediation WP instead of reopening the old packet as if it were still active execution.
- If the old packet is blocked by `LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED`, treat that as a historical failure that requires a new remediation packet/version rather than an in-place revive.

## Orchestrator Lean Mode (HARD RULE — Token Discipline)

During active WP execution (any WP is IN_PROGRESS with live coder or validator sessions):

- Issue only steering commands and status checks. Do not write audits, summaries, explanations, or postmortem reasoning until all active WPs reach a verdict boundary (PASS, FAIL, or explicit STOP).
- Do not relay messages between coder and validator. Coders and WP validators MUST communicate directly, and for the required review lane they MUST use the structured direct-review helpers (`just wp-validator-kickoff`, `just wp-coder-intent`, `just wp-coder-handoff`, `just wp-validator-review`). `just wp-thread-append` is for soft coordination only. The orchestrator is not a message broker.
- Do not narrate recovery steps. Fix blockers silently and continue steering.
- Do not write audit prose mid-run. Audits and reviews belong after the run reaches a stable state, not while active sessions are consuming tokens.

Rationale: the parallel smoke tests proved that orchestrator relay + mid-run narration consumed extreme token budgets. Direct coder<->validator communication and lean orchestrator posture are mandatory for sustainable parallel work.

## Direct Coder <-> WP Validator Communication (HARD RULE)

- The orchestrator MUST instruct both coder and WP validator to communicate directly at session start. This is already embedded in `buildStartupPrompt()`.
- For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED` packets with `PACKET_FORMAT_VERSION >= 2026-03-21`, the packet MUST declare `COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1` and `COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING`.
- Required structured receipts for that contract are:
  - `VALIDATOR_KICKOFF` (`WP_VALIDATOR -> CODER`)
  - `CODER_INTENT` (`CODER -> WP_VALIDATOR`, correlated to kickoff)
  - `CODER_HANDOFF` (`CODER -> WP_VALIDATOR`)
  - `VALIDATOR_REVIEW` (`WP_VALIDATOR -> CODER`, correlated to handoff)
- In orchestrator-managed lanes, the WP Validator is the first technical judge for coder BOOTSTRAP, SKELETON, and completed micro tasks. The Orchestrator should not babysit those phases unless the validator raises a real blocker.
- The initial `VALIDATOR_KICKOFF -> CODER_INTENT -> VALIDATOR_RESPONSE|SPEC_GAP|VALIDATOR_QUERY` exchange is the normal bootstrap/skeleton steering surface. Use it to correct weak scope, wrong data shapes, or shallow micro-task plans before implementation hardens, and treat validator clearance there as mandatory on governed lanes.
- The WP Validator may also review completed narrow slices in parallel while the coder advances the next declared microtask, but only through structured `REVIEW_REQUEST` / resolution receipts with bounded overlap backlog. Full `CODER_HANDOFF` still waits for the overlap queue to drain.
- For `PACKET_FORMAT_VERSION >= 2026-03-22`, `VERDICT` also requires one direct coder <-> integration-validator review pair recorded in receipts with matching `correlation_id` / `ack_for`.
- Review-tracked receipt appends now auto-write notifications for the explicit target role, notify `ORCHESTRATOR` on validator-authored assessment receipts as a governance checkpoint, include the assessment result (`PASS`/`FAIL`/`ASSESSED`) plus the validator's reason in that checkpoint summary, and auto-project the next actor / validator wake state back into `RUNTIME_STATUS.json`. Watch that projected route; do not replace it with manual narrative steering unless a real repair is required.
- Before a coder can mark handoff-ready, `just wp-communication-health-check WP-{ID} KICKOFF` MUST pass.
- Before validator handoff review begins, `just wp-communication-health-check WP-{ID} HANDOFF` MUST pass.
- Before PASS commit clearance, `just wp-communication-health-check WP-{ID} VERDICT` MUST pass.
- The orchestrator should monitor WP communications to verify direct traffic is happening, and steer correction if it is not.

## Worktree Budget (HARD RULE)

- Maximum WP-specific worktrees per WP: 1 [CX-503G].
- The Coder and WP Validator share the same worktree (`wtc-*` on `feat/WP-*`). The per-MT stop pattern ensures only one role is active at a time (coder commits and stops, validator reviews and responds, coder resumes). Governance uses the `.GOV/` junction to the kernel.
- The Integration Validator operates from `handshake_main` on branch `main` — no WP-specific worktree.
- Do not create ad-hoc temp worktrees (detached checkouts, merge worktrees, revalidation worktrees) outside the governed naming scheme.
- After a WP reaches VALIDATED or MERGED, require governed cleanup of WP-specific worktrees before starting new WPs.
- All worktrees must be created under the shared worktree root so `just enumerate-cleanup-targets` can find them. Off-root worktree creation is forbidden.
- `worktree-concurrency-check` enforces this budget as part of `gov-check`.

## WP Worktree Creation Rules [CX-212D] (HARD RULE)

- WP worktrees (`wtc-*`) are created from `main` but MUST NOT retain a git-tracked `/.GOV/` directory. Legacy `wtv-*` worktrees from the old 2-per-WP model are cleanup candidates.
- After `git worktree add`, the creation script MUST:
  1. Remove the inherited `/.GOV/` directory from the new worktree.
  2. Create a junction (`mklink /J` on Windows, symlink on Unix) from `/.GOV/` to `../wt-gov-kernel/.GOV`.
- This ensures WP worktrees always read live governance from the kernel and never have a stale `/.GOV/` copy.
- The `worktree-add.mjs` script enforces this automatically.

## Gov-to-Main Sync Responsibility [CX-212D] (HARD RULE)

- `just sync-gov-to-main` copies the governance kernel `/.GOV/` into `handshake_main` and auto-commits.
- `just sync-gov-to-main` must sync from committed kernel truth. If `wt-gov-kernel/.GOV` is dirty, fix or commit `gov_kernel` first; do not mirror an uncommitted kernel snapshot into `main`.
- This is the Integration Validator's default responsibility, to be run before pushing to `origin/main`.
- The Orchestrator MAY run `just sync-gov-to-main` and push `origin/main` only when explicitly instructed by the Operator.
- That Orchestrator exception is mechanical execution only. It does not grant final technical verdict authority or permission to invent a new product merge decision.
- The `main` worktree retains a real (non-junction) `/.GOV/` copy as a stable backup.

## Notification System (HARD RULE — Message Delivery)

- Every thread message with a `@target` or explicit `target_role` writes a notification to `NOTIFICATIONS.jsonl` in the WP communications directory.
- Every review exchange (REVIEW_REQUEST, VALIDATOR_QUERY, SPEC_GAP, etc.) writes a notification to the target role.
- Roles check pending notifications after startup and before each handoff/verdict using `just check-notifications {wpId} {ROLE}`.
- Roles acknowledge notifications after reading using `just ack-notifications {wpId} {ROLE} {session}`.
- The orchestrator should monitor notification counts via the Operator Monitor TUI (PENDING NOTIFICATIONS in the OVERVIEW detail view) and steer correction if notifications pile up without acknowledgment.
- Startup prompts already embed NOTIFICATIONS (MANDATORY) instructions for all three governed roles. Do not remove or weaken these instructions.

## Pre-Smoke Validation Gate (RECOMMENDED)

Before launching an orchestrator-managed session with multiple parallel WPs, run:
1. `just gov-check` — governance must be clean before starting
2. Verify all session control tooling paths resolve correctly
3. Verify all required worktree base branches exist
4. Verify the ACP broker is responsive or can be started

This prevents the mid-smoke governance repair that consumed excessive context in previous smoke tests.

## Orchestrator Non-Negotiables

Do not:
- create a packet without a real Main Body `SPEC_ANCHOR`
- edit locked packets in place
- delegate when `just pre-work` fails
- let planning projections drift from packet truth
- broadcast a collapsed single PASS claim for workflow, tests, and spec correctness
- relay messages between coder and validator (direct communication is mandatory)
- create ad-hoc temp worktrees outside the governed naming scheme
- write audit prose during active WP execution

Do:
- keep refinement, packet, traceability, build-order, and Task Board aligned
- use the current packet template and deterministic helpers
- keep external session/topology/WP-communication runtime state under the repo-governance runtime root and keep repo-local spec-coupled runtime state under `/.GOV/roles_shared/runtime/`
- keep role-owned state under `/.GOV/roles/orchestrator/runtime/`
- stop and escalate when tooling or docs conflict with active law
- verify direct coder<->validator communication is happening before allowing handoff
- enforce worktree budget limits per WP
- monitor pending notification counts and steer roles that ignore their notifications
