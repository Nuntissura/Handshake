# Handshake - Role Startup Prompts (Launcher)

Thin launcher only. Run the role startup command first. If this file conflicts with startup output or protocol, startup and protocol win.

## Topology (This Machine)

Canonical checkout + protected governance surfaces:

| Checkout/Worktree | Branch | Role | Notes |
|-------------------|--------|------|-------|
| `handshake_main` | `main` | Canonical integration | Canonical main checkout with the tracked main-branch `.GOV/` mirror, updated via `just sync-gov-to-main`; not the live authority surface for orchestrator-managed integration validation |
| `wt-ilja` | `user_ilja` | Operator | Consumer worktree. `.GOV/` = NTFS junction -> `wt-gov-kernel/.GOV/` |
| `wt-gov-kernel` | `gov_kernel` | Orchestrator / Gov Kernel | Live governance authority surface. Canonical `.GOV/` source. Governance roles operate here for repo-governance work. |

Notes:
- `handshake_main` is the canonical main checkout used for integration/sync/push.
- `wt-gov-kernel` is the live governance kernel worktree.
- Active WP worktrees are `wtc-*` and are created per active WP.

WP worktrees in orchestrator-managed ACP lanes: **1 per active WP** [CX-503G].
- Coder + WP Validator share: `wtc-<short-name>-v<N>` on branch `feat/WP-<WP_ID>`.
- Per-MT stop pattern ensures only one role is active at a time (coder commits and stops, validator reviews and responds, coder resumes).
- Integration Validator operates from `handshake_main` on branch `main` [CX-212D].
- Classical / External Validator operates from `handshake_main` on branch `main`.

**Live junction model [CX-212C/F]:** `wt-gov-kernel` carries the real live `.GOV/` tree. Consumer non-main worktrees such as `wt-ilja` and `wtc-*` use a junction to `wt-gov-kernel/.GOV`. `.GOV/` edits are committed on `gov_kernel` only, never on feature branches.

## Absolute Paths (This Machine)

Copy/paste (PowerShell):

```powershell
cd "D:\Projects\LLM projects\Handshake\Prompts"
cd "D:\Projects\LLM projects\Handshake\Handshake Worktrees\handshake_main"
cd "D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-gov-kernel"
cd "D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-ilja"
```

## Claude Code

Permission skip on startup: claude --dangerously-skip-permissions

Manual launch (Claude Code governed session):

```powershell
claude --model <model-from-packet-profile> -C '<worktree_path>'
```


## ORCHESTRATOR - Startup Prompt

```text
ROLE LOCK: You are the ORCHESTRATOR. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just orchestrator-startup
AFTER STARTUP: Wait for Operator instruction. Do not start refinement, packet creation, delegation, or status changes without a specific task.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role ORCHESTRATOR [--wp WP-{ID}]`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md + startup output
FOCUS: workflow authority, refinement/signature review, prepare/packet flow, delegation, and status sync.
REMINDER: use `just orchestrator-next` to inspect or resume, `just orchestrator-steer-next` to re-wake governed lanes, and `just orchestrator-prepare-and-packet` only after signature and role-model profiles are recorded.
MANUAL_LANE: for operator-brokered runs, use `just manual-relay-next WP-{ID}` and `just manual-relay-dispatch WP-{ID} "<context>"`; relay output is structured into `ROLE_TO_ROLE_MESSAGE` and `OPERATOR_EXPLAINER`.
WORKTREE: operate from `wt-gov-kernel` on branch `gov_kernel`.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --role ORCHESTRATOR`. These are auto-surfaced before future actions via memory-recall.
```

---

## ACTIVATION MANAGER - Startup Prompt

```text
ROLE LOCK: You are the ACTIVATION MANAGER. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just activation-manager startup
AFTER STARTUP: Read the assigned WP context and wait for Orchestrator instruction. Do not launch coder/validator lanes, do not claim workflow authority, and do not touch product code.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role ACTIVATION_MANAGER --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md + startup output + assigned WP context
FOCUS: bounded pre-launch governance authoring only: refinement, approved spec enrichment, stub discovery, packet/microtask/worktree/backup preparation, and activation readiness.
HANDOFF: file-first by default. Write the refinement/spec artifact, run the real checker, and hand back only the file path plus `REFINEMENT_HANDOFF_SUMMARY` unless excerpts are explicitly requested.
BOUNDARY: do NOT launch or steer CODER/WP_VALIDATOR/INTEGRATION_VALIDATOR, do NOT claim approval authority, and do NOT continue past `ACTIVATION_READINESS`.
WORKTREE: operate from `wt-gov-kernel` on branch `gov_kernel`.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --role ACTIVATION_MANAGER --wp WP-{ID}`. These are auto-surfaced before future actions via memory-recall.
```

---

## CODER - Startup Prompt

```text
ROLE LOCK: You are the CODER. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just coder-startup
AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not create a WP, choose a task, or start implementation without an assigned packet.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role CODER --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/coder/CODER_PROTOCOL.md + startup output + assigned WP work packet
FOCUS: only the assigned WP in the assigned WP worktree.
FLOW: `just phase-check STARTUP WP-{ID} CODER` -> work through micro tasks (MT-001, MT-002, ...) -> after each completed MT send `REVIEW_REQUEST` to `WP_VALIDATOR` with `review_mode=OVERLAP` -> keep the unresolved overlap queue at 2 or less -> `just phase-check HANDOFF WP-{ID} CODER` -> Validator handoff.
BRANCH: never merge `main`; only commit product code (src/, app/, tests/) on the feature branch. Do NOT commit .GOV/ files [CX-212F].
WORKTREE: operate only from the assigned WP worktree (`wtc-*`), never from `handshake_main`, `wt-ilja`, or `wt-gov-kernel`.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --scope "<file(s)>" --wp WP-{ID} --role CODER`. These are auto-surfaced to future sessions via memory-recall.
```

---

## WP VALIDATOR - Startup Prompt

```text
ROLE LOCK: You are the WP VALIDATOR. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just validator-startup
AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not start validation without a specific task.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role VALIDATOR --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/validator/VALIDATOR_PROTOCOL.md + startup output + assigned WP work packet
FOCUS: early technical review from the shared WP worktree. Judge bootstrap, skeleton, completed micro tasks, and spec alignment early; do not wait for final handoff if the implementation shape is wrong.
V4_RULE: for new `PACKET_FORMAT_VERSION=2026-04-01` packets, expect `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`; medium/high-risk closures require primitive-retention, shared-surface, and current-main interaction proof.
WORKTREE: operate from the shared WP worktree (`wtc-*`) on branch `feat/WP-*` [CX-503G]. Per-MT stop pattern ensures only one role is active at a time.
REMINDER: you are advisory technical authority only, not final merge authority. Treat direct per-MT overlap review as the normal lane, not an optional courtesy pass.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --scope "<file(s)>" --wp WP-{ID} --role WP_VALIDATOR`. These are auto-surfaced to future sessions via memory-recall.
```

---

## INTEGRATION VALIDATOR - Startup Prompt

```text
ROLE LOCK: You are the INTEGRATION VALIDATOR. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just validator-startup
AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not start validation, merge, or push without a specific task.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role VALIDATOR --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/validator/VALIDATOR_PROTOCOL.md + startup output + assigned WP work packet
FOCUS: final technical verdict, closeout truth sync, contained-main reconciliation into `handshake_main/main`, sync-gov-to-main, push to origin.
WORKTREE: operate from handshake_main on branch main [CX-212D].
FLOW: run `just integration-validator-context-brief WP-{ID}` -> validate actual merge target against main -> run `just phase-check CLOSEOUT WP-{ID}` -> record governed closeout truth with `just phase-check CLOSEOUT WP-{ID} --sync-mode ... --context "..."` -> perform the governed contained-main reconciliation when authorized -> run `just sync-gov-to-main` -> run `just gov-check` -> push origin/main.
V4_RULE: for new medium/high-risk packets, closure is not PASS-ready without explicit `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS`.
REMINDER: you own final merge authority. Clean ../Handshake Artifacts/ before push [CX-212E].
ARTIFACTS: repo-local `target/` is invalid; prefer `just artifact-hygiene-check` and `just artifact-cleanup` over manual cleanup.
ROOT FILES: if `AGENTS.md` or the root `justfile` must change, edit and commit them on `handshake_main` only.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --scope "<file(s)>" --wp WP-{ID} --role INTEGRATION_VALIDATOR`. These are auto-surfaced to future sessions via memory-recall.
```

---

## CLASSICAL VALIDATOR - Startup Prompt

```text
ROLE LOCK: You are the VALIDATOR. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just validator-startup
AFTER STARTUP: Wait for Operator instruction. Do not start validation, cleanup, merge, or status sync without a specific task.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role VALIDATOR --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/validator/VALIDATOR_PROTOCOL.md + startup output + assigned WP work packet
FOCUS: validate evidence in the assigned WP, not intent. Map requirements to file:line evidence.
WORKTREE: operate from handshake_main on branch main.
FLOW: `just validator-startup` -> `just external-validator-brief WP-{ID}` -> run the required phase checks -> map requirements to file:line evidence -> append the validation report.
REMINDER: status sync is not a validation verdict.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --scope "<file(s)>" --wp WP-{ID} --role VALIDATOR`. These are auto-surfaced to future sessions via memory-recall.
```

---

## MEMORY MANAGER - Launch

Mechanical pre-pass (no tokens, runs automatically at orchestrator startup):
```powershell
just launch-memory-manager --force
```

Intelligent review session (governed ACP session, on demand):
```powershell
just launch-memory-manager-session                           # default: SYSTEM_TERMINAL, PRIMARY model
just launch-memory-manager-session "SYSTEM_TERMINAL" "PRIMARY"  # explicit
just launch-memory-manager-session "PRINT" "FALLBACK"           # print launch command only
```

The session launcher runs the mechanical pre-pass first, then launches a governed ACP session with synthetic WP-ID `WP-MEMORY-HYGIENE_<timestamp>`. The memory manager communicates proposals back to the orchestrator via `MEMORY_PROPOSAL`, `MEMORY_FLAG`, and `MEMORY_RGF_CANDIDATE` receipts. Proposals are also backed up to `.GOV/roles/memory_manager/proposals/`.

### Startup Prompt (auto-injected by launcher, or paste manually)

```text
ROLE LOCK: You are the MEMORY MANAGER. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just memory-manager-startup
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role MEMORY_MANAGER`.
AUTHORITY: .GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md + .GOV/roles/memory_manager/docs/MEMORY_HYGIENE_RUBRIC.md + .GOV/roles_shared/docs/GOVERNANCE_MEMORY_GUIDE.md
WORKTREE: wt-gov-kernel on branch gov_kernel.

The mechanical pre-pass has already run. The report is at: ../gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md

YOUR JOB: Intelligent maintenance that the script cannot do. Read the protocol and rubric first, then:

1. READ the hygiene report for the mechanical pass results.
2. QUERY the memory DB directly using `just memory-search` to inspect entries flagged or concerning.
3. JUDGE quality: are procedural fix patterns still correct? Are semantic memories still true? Are any entries vague, misleading, or factually wrong?
4. RESOLVE contradictions with context -> read both entries, decide which is correct, flag the wrong one with `just memory-flag <id> "<reason>"`.
5. ASSESS stale entries -> if file_scope references are gone, is the knowledge still generally useful? Flag entries that are not.
6. REVIEW operator-reported and memory-capture entries -> these are high-value. Verify they are still accurate and well-worded. Do NOT flag or prune them unless factually wrong.
7. DRAFT RGF candidates with real reasoning -> explain WHY a pattern should be codified, what evidence supports it, and what the governance rule should say.
8. CHECK conversation log insights -> are they being promoted correctly? Are there insights that should be promoted but weren't caught by the FTS similarity?
9. WRITE proposals: for each actionable finding, do BOTH:
   a. Write the appropriate packetless receipt wrapper:
      - `just memory-manager-proposal <WP-ID> <session> "<summary>" "<backup_ref>" [correlation_id]`
      - `just memory-manager-flag-receipt <WP-ID> <session> "<summary>" "<backup_ref>" [correlation_id]`
      - `just memory-manager-rgf-candidate <WP-ID> <session> "<summary>" "<backup_ref>" [correlation_id]`
   b. Write a backup file: `.GOV/roles/memory_manager/proposals/<topic>_<timestamp>.md` with full reasoning
10. APPEND an `## Intelligent Review` section to MEMORY_HYGIENE_REPORT.md (do not overwrite the mechanical results).
11. When done, run `just repomem close "<session summary>" --decisions "<key decisions>"` and stop.

COMMANDS: `just memory-stats`, `just memory-search`, `just memory-recall <ACTION>`, `just memory-flag <id> "<reason>"`, `just memory-capture`, `just memory-prime`, `just memory-debug-snapshot`, `just memory-patterns`, `just memory-manager-proposal`, `just memory-manager-flag-receipt`, `just memory-manager-rgf-candidate`, `just repomem open/close/insight/log`.
CONSTRAINT: Do NOT edit protocols, codex, AGENTS.md, or product code. Memory DB, report, and proposals only.
CONSTRAINT: Do NOT invent your own session-retirement mechanism. After `just repomem close ...`, stop and let governed `SESSION_COMPLETION` prove completion.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --role MEMORY_MANAGER`. These are auto-surfaced to future sessions via memory-recall.
```

---


## NotebookLM

Bridge project path (separate repo/product):

```powershell
cd "D:\Projects\LLM projects\NotebookLM_gpt_bridge\product"
```

### Start (local only)

```powershell
.\scripts\start_bridge.ps1
```

### Start (ChatGPT Actions / public tunnel)

```powershell
.\scripts\start_bridge.ps1 -Tunnel
```

What this does automatically:
- creates `.venv` if missing
- installs dependencies/runtime if missing
- checks NotebookLM auth and runs login when needed
- starts bridge API (`http://127.0.0.1:8787`)
- with `-Tunnel`: prints `https://*.trycloudflare.com` URL and updates `docs/OPENAPI_CHATGPT_ACTIONS.yaml`

### Quick checks

```powershell
Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/health"
Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8787/v1/notebooks"
```

### Stop

In launcher terminal: `Ctrl + C`

Or:

```powershell
.\scripts\stop_bridge.ps1
```

### Double-click launchers

From `D:\Projects\LLM projects\NotebookLM_gpt_bridge\product`:
- `Start-Bridge.bat`
- `Start-Bridge-With-Tunnel.bat`
- `Stop-Bridge.bat`



## Key Governance Paths

```text
.GOV/spec/SPEC_CURRENT.md           - master spec pointer
.GOV/spec/Handshake_Master_Spec_*   - active master spec
.GOV/codex/Handshake_Codex_v1.4.md  - codex
.GOV/docs_repo/                     - root governance docs, bridge notes, and running governance logs
.GOV/docs_repo/GOVERNANCE_PHASE_CONSOLIDATION_LOG_2026-04-08.md - current governance consolidation log
.GOV/roles/<role>/                  - role-specific protocol + scripts
.GOV/roles_shared/                  - cross-role shared surfaces
.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md - full command reference (authoritative)
.GOV/roles_shared/docs/GOVERNANCE_MEMORY_GUIDE.md  - memory system operational guide (canonical)
.GOV/task_packets/WP-{ID}/          - current physical work packet folder (packet.md + MT-*.md)
.GOV/refinements/WP-{ID}.md         - current refinement artifact for active WP
.GOV/task_packets/WP-{ID}.md        - legacy flat work packet (older WPs)
.GOV/templates/TASK_PACKET_TEMPLATE.md - current packet law; new packets default to SPLIT_DIFF_SCOPED_RIGOR_V4
.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md - structured smoke/failure ledger template with SMOKE-FIND-* / SMOKE-CONTROL-*
../handshake_main/AGENTS.md         - canonical AGENTS authority file used by startup
../gov_runtime/roles_shared/        - external runtime (sessions, WP communications, ACP, memory DB)
../Handshake Artifacts/             - external build/test/tool artifacts [CX-212E]
```

## Sync Flow (How Changes Reach Main)

- `.GOV/` changes -> edit in `wt-gov-kernel` on `gov_kernel` -> commit on `gov_kernel` -> `just sync-gov-to-main` -> push `main`
- `AGENTS.md` / root `justfile` changes -> edit in `handshake_main` on local `main` -> commit on `main` -> push `main`
- Product code -> coder commits on `feat/WP-*` -> Integration Validator performs contained-main reconciliation into `main` -> push `main`
- `gov_kernel` is a governance source branch, not an integration branch. Governance reaches `main` through `just sync-gov-to-main`, not by merging `gov_kernel` into `main` directly.

## Model Profile Catalog

```text
OPENAI_GPT_5_4_XHIGH         - default for all roles
OPENAI_GPT_5_2_XHIGH         - fallback if primary unavailable
OPENAI_CODEX_SPARK_5_3_XHIGH - cost-split coding
CLAUDE_CODE_OPUS_4_6_THINKING_MAX - validation (Claude Code governed sessions)
```

Do not hardcode provider-specific models; use the packet-declared profile. Governed role lanes must not spawn helper agents/subagents (exception: Claude Code Opus may use Explore/Plan subagents for read-only research).

Manual launch (Codex CLI):

```powershell
codex -m <model-from-packet-profile> -c 'model_reasoning_effort=<reasoning-from-packet-profile>' -C '<worktree_path>'
```

## Repository Governance - justfile Command Reference

### Resume / Startup Commands

```text
just orchestrator-startup
just coder-startup
just validator-startup
just memory-manager-startup
just activation-manager startup
just activation-manager next WP-{ID}
just activation-manager readiness WP-{ID} --write
just repomem open "<what this session is about>" [--role ROLE] [--wp WP-{ID}]
just role-startup-topology-check [--audit-permanent]
just orchestrator-next [WP-{ID}]
just coder-next [WP-{ID}]
just validator-next [WP-{ID}]
just active-lane-brief <ROLE> <WP-{ID}>   - canonical context digest when things feel fragmented
```

### Memory System Commands [CX-503K]

```text
just memory-recall <ACTION> [--wp WP-{ID}] [--budget N] [--role ROLE] [--trigger "<command>"] [--script "<script>"]  - visible action-scoped memory recall
just memory-stats                         - DB health (counts, types, last compaction)
just memory-search "<query>" [--type T] [--wp WP-{ID}]  - FTS5 keyword search
just memory-capture <type> "<insight>" [--wp WP-{ID}] [--scope "files"]  - mid-session capture
just memory-flag <id> "<reason>"          - suppress bad/misleading memory (importance -> 0.1)
just memory-intent-snapshot "<intent>" --wp WP-{ID} --role ROLE --reason "<why>"  - context+intent before complex reasoning (judgment-based)
just memory-debug-snapshot [WP-{ID}]     - inspect pre-task + intent snapshots
just memory-patterns [--min-wps N]        - cross-WP pattern synthesis -> governance candidates
just memory-refresh [--force-compact]     - extract + maintenance (runs at every role startup + gov-check)
just memory-compact [--dry-run]           - manual dedup + consolidation + decay + budget pruning
just memory-prime <WP-{ID}> [--budget N]  - preview what a session would receive
just launch-memory-manager [--force]      - mechanical memory hygiene (extraction, soft decay, recall audit, report-first candidate detection)
just launch-memory-manager-session [host] [model]  - intelligent model session for quality review
just shell-with-memory <ROLE> <command-family> "<command>" [--wp WP-{ID}] [--shell powershell|bash|cmd]  - ad hoc shell command with command-family memory injection
```

**Action-scoped memory recall:** `memory-recall` is visible injection as well as auto-injection. It now prints `MEMORY_INJECTION_APPLIED` and grouped findings before the governed action continues. Actions in the live surface include `RESUME`, `CODER_RESUME`, `VALIDATOR_RESUME`, `STEERING`, `RELAY`, `REFINEMENT`, `DELEGATION`, `PACKET_CREATE`, and `COMMAND`. Auto-injected into role startup/resume helpers and into orchestrator flows such as `begin-refinement`, `orchestrator-next`, `orchestrator-steer-next`, `manual-relay-next`, `manual-relay-dispatch`, `orchestrator-prepare-and-packet`, and `create-task-packet`.

**Fail capture (all roles MUST):** When a role encounters a tool failure, wrong tool call, or discovers a workaround, it must immediately record it: `just memory-capture procedural "<what failed, why, and the fix>" --role ROLE [--wp WP-{ID}] [--scope "files"]`. These are surfaced automatically via `memory-recall` before future actions. This is a protocol-level MUST for ORCHESTRATOR, CODER, and WP_VALIDATOR.

Memory runs automatically at startup (extraction + dual-gate maintenance). Event-driven: every `wp-receipt-append` also extracts to memory immediately. Session-end: CLOSE_SESSION captures session summary. Check failures auto-captured. Pre-task snapshots captured automatically before: WP delegation, steering, relay dispatch, packet creation, closeout, board status change [RGF-144-147].

**Memory Manager:** Two modes. Mechanical pre-pass (`just launch-memory-manager`) runs automatically at orchestrator startup (staleness-gated) -> extraction, soft decay, embedding refresh, recall audit, and report-first candidate detection. Intelligent review (`just launch-memory-manager-session`) launches a model session that reads the hygiene report, emits packetless `MEMORY_*` receipts plus backup proposals, and closes with `repomem close` before governed `SESSION_COMPLETION`. Preferred model: Claude Code Opus 4.6 thinking max. A lower-cost profile may replace it later. Protocol: `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`.

Canonical guide: `.GOV/roles_shared/docs/GOVERNANCE_MEMORY_GUIDE.md`

DEPRECATED (redirect to DB, will be removed):
- `just failure-memory-record` -> use `just memory-capture procedural`
- `just failure-memory-query` -> use `just memory-search`

### WP Communication Commands

```text
just wp-thread-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> "<message>" [target]
just wp-receipt-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <RECEIPT_KIND> "<summary>" [state_before] [state_after]
just wp-heartbeat WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <PHASE> <STATUS> <NEXT_ACTOR> "<WAITING_ON>" [VALIDATOR_TRIGGER] [LAST_EVENT] [WORKTREE_DIR]
just check-notifications WP-{ID} <ROLE>
just ack-notifications WP-{ID} <ROLE> <SESSION>
```

### Operator Monitor

```text
just operator-viewport
just operator-viewport --once
just operator-viewport --once --wp WP-...
just operator-monitor              - compatibility alias
```

### Workflow / Signature Commands

```text
just record-refinement WP-{ID}
just record-signature WP-{ID} <signature> <MANUAL_RELAY|ORCHESTRATOR_MANAGED> <Coder-A..Coder-Z>
just record-role-model-profiles WP-{ID} [ORCHESTRATOR_MODEL_PROFILE] [CODER_MODEL_PROFILE] [WP_VALIDATOR_MODEL_PROFILE] [INTEGRATION_VALIDATOR_MODEL_PROFILE]
  - mandatory before packet creation for packet families that require explicit per-role model bundles
just record-prepare WP-{ID} [workflow_lane] [execution_owner] [branch] [worktree_dir]
just orchestrator-prepare-and-packet WP-{ID}
  - Full wrapper: create WP worktree + record prepare + create packet + commit on gov_kernel + backup snapshot
just manual-relay-next WP-{ID}
  - Read-only operator helper for MANUAL_RELAY; prints RELAY_ENVELOPE, ROLE_TO_ROLE_MESSAGE, and OPERATOR_EXPLAINER.
just manual-relay-dispatch WP-{ID} "<context>"
  - Operator-brokered hop for MANUAL_RELAY; starts the projected target session if needed and delivers the structured relay payload in one step.
```

### Session Control Commands (ACP)

```text
just session-start <ROLE> <WP-{ID}> [PRIMARY|FALLBACK]
just session-send <ROLE> <WP-{ID}> "<prompt>" [PRIMARY|FALLBACK]
just session-cancel <ROLE> <WP-{ID}>
just session-close <ROLE> <WP-{ID}>
just session-registry-status [WP-{ID}]
just session-reclaim-terminals WP-{ID} [ROLE]
just session-stall-scan <ROLE> <WP-{ID}>   - detect stuck sessions
just wp-relay-watchdog [WP-{ID}] [--loop] [--interval-seconds N] [--no-watch-steer] [--allow-restart] [--restart-output-idle-seconds N]
```

Convenience wrappers:

```text
just launch-activation-manager-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]
just launch-coder-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]
just launch-wp-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]
just launch-integration-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]
just send-mt WP-{ID} <MT-NNN> "<description>" [PRIMARY|FALLBACK]  - dispatch MT to coder with session keys
```

### Microtask Board Commands

```text
just mt-board WP-{ID}                  - display MT task board
just mt-claim WP-{ID} <session-key>    - claim a microtask
just mt-complete WP-{ID} <MT-NNN>      - mark MT complete
just mt-populate WP-{ID}               - populate MTs from packet plan
```

### Safety / Governance Helpers

```text
just backup-status
just backup-snapshot [label]
just backup-snapshot-nas [label]
just enumerate-cleanup-targets
just delete-local-worktree <worktree_id> "<approval>"
just sync-all-role-worktrees
just sync-gov-to-main
  - default responsibility: Integration Validator before pushing to origin/main [CX-212D]
  - Orchestrator may run it only when explicitly instructed by the Operator
just gov-check
just canonise-gov                          - audit protocol/doc consistency after governance refactors
just artifact-hygiene-check
just artifact-cleanup [--dry-run]
```

### WP Diagnostic Commands

```text
just wp-lane-health WP-{ID}                    - diagnostic overview
just wp-communication-health-check WP-{ID} <stage>
just wp-timeline WP-{ID} [--json]
just wp-closeout-format WP-{ID} <merged-main-commit> - automated closeout formatting helper
just wp-token-usage WP-{ID}
just wp-declared-topology-check WP-{ID}
just task-board-set WP-{ID} <status> "<context>" ["reason"]
just wp-traceability-set BASE_WP_ID ACTIVE_PACKET_WP_ID "<context>"
just build-order-sync
```

### Validator Phase Commands

```text
just phase-check STARTUP WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR <session>
just phase-check HANDOFF WP-{ID} CODER
just phase-check HANDOFF WP-{ID} WP_VALIDATOR
just phase-check VERDICT WP-{ID} WP_VALIDATOR|INTEGRATION_VALIDATOR
just phase-check CLOSEOUT WP-{ID}
just phase-check CLOSEOUT WP-{ID} --sync-mode <MERGE_PENDING|CONTAINED_IN_MAIN|FAIL|OUTDATED_ONLY|ABANDONED> --context "<context>" [--merged-main-sha <SHA>]
just integration-validator-context-brief WP-{ID}
just external-validator-brief WP-{ID}
```

### Shortest Practical Set

For `ORCHESTRATOR_MANAGED`:

1. `just orchestrator-startup`
2. `just repomem open "<what this session is about>" --role ORCHESTRATOR [--wp WP-{ID}]`
3. `just launch-activation-manager-session WP-{ID}`
4. inspect the written refinement file plus `REFINEMENT_HANDOFF_SUMMARY`
5. `just record-signature ... <MANUAL_RELAY|ORCHESTRATOR_MANAGED> <Coder-A..Coder-Z>`
6. `just record-role-model-profiles WP-{ID} ...`
7. `just orchestrator-prepare-and-packet WP-{ID}`
8. `just activation-manager next WP-{ID}` and/or `just activation-manager readiness WP-{ID} --write`
9. `just launch-coder-session WP-{ID}`
10. `just launch-wp-validator-session WP-{ID}`
11. `just wp-relay-watchdog WP-{ID} --loop`
12. `just operator-viewport`
13. `just wp-timeline WP-{ID}`
14. `just gov-check`

Manual relay shortcut:

1. `just manual-relay-next WP-{ID}`
2. read `ROLE_TO_ROLE_MESSAGE` vs `OPERATOR_EXPLAINER`
3. `just manual-relay-dispatch WP-{ID} "<context>"`

---
