# Handshake - Role Startup Prompts (Launcher)

Thin launcher only. This file lives under `.GOV/operator/docs_local/` and is a staging/operator convenience sheet, not canonical law. Run the role startup command first. If this file conflicts with startup output or protocol, startup and protocol win.

Startup shell timeout: for every `FIRST COMMAND` startup call, use an extended shell timeout of at least `600000` ms / 10 minutes. Startup loads authority, topology, deterministic checks, memory refresh, and resume recall; a shorter shell timeout can truncate the required context before the role is actually ready.

## Topology (This Machine)

Canonical checkout + protected governance surfaces:

| Checkout/Worktree | Branch | Role | Notes |
|-------------------|--------|------|-------|
| `handshake_main` | `main` | Canonical integration | Canonical main checkout with the tracked main-branch `.GOV/` mirror, updated via `just sync-gov-to-main`; not the live authority surface for orchestrator-managed integration validation |
| `wt-ilja` | `user_ilja` | Operator | Consumer worktree. `.GOV/` = NTFS junction -> `wt-gov-kernel/.GOV/` |
| `wt-gov-kernel` | `gov_kernel` | Orchestrator / Gov Kernel | Live governance authority surface. Canonical `.GOV/` source. Governance roles operate here for repo-governance work. |

Notes:
- `handshake_main` is the canonical main checkout used for integration/sync/push.
- `handshake_main` is also the main-only authoring surface for the canonical root `AGENTS.md` and root `justfile`.
- `wt-gov-kernel` is the live governance kernel worktree.
- `wt-gov-kernel` may carry a kernel-local launcher `justfile`, but it does not replace main ownership of the canonical root file.
- Active WP worktrees are `wtc-*` and are created per active WP.

WP worktrees in orchestrator-managed ACP lanes: **1 per active WP** [CX-503G].
- Coder + WP Validator share: `wtc-<short-name>-v<N>` on branch `feat/WP-<WP_ID>`.
- Per-MT stop pattern ensures only one role is active at a time (coder commits and stops, validator reviews and responds, coder resumes).
- Integration Validator operates from `handshake_main` on branch `main` [CX-212D].
- Classical / External Validator operates from `handshake_main` on branch `main`.

**Live junction model [CX-212C/F]:** `wt-gov-kernel` carries the real live `.GOV/` tree. Consumer non-main worktrees such as `wt-ilja` and `wtc-*` use a junction to `wt-gov-kernel/.GOV`. `.GOV/` edits are committed on `gov_kernel` only, never on feature branches.

## Workspace Anchors

Prefer repo-relative or workspace-relative forms on governance surfaces:

```text
../handshake_main
../wt-gov-kernel
../wt-ilja
.GOV/...
../gov_runtime/...
../Handshake_Artifacts/...
```

Do not paste host absolute paths into packets, diagnostics, workflow dossiers, monitor output, or startup guidance. If a governed surface emits an absolute host path, treat it as governance drift and repair the emitting surface.

## Recent Governance Delta (2026-04-25 to 2026-04-30)

Checked from git history and repomem session-close decisions. Operator-facing changes:

- Startup now includes Memory-Manager-curated startup briefs. Run `just role-startup-brief <ROLE>` to print the shared and role-specific action cards; briefs are operational memory only and lose to protocols, packets, and live runtime truth.
- Repomem role coverage is hardened. Every governed mutation needs a substantive `repomem open` with role, and WP-bound lanes also need `--wp`. `repomem close` needs `--decisions`. Tool failures and workarounds must immediately use `memory-capture procedural`.
- Workflow Dossier is diagnostic by design: raw ACP/session telemetry and Orchestrator postmortem can coexist. Durable role notes belong in repomem and are imported at closeout; do not over-normalize dossier raw output.
- `ORCHESTRATOR_MANAGED` pre-launch is split: Activation Manager owns refinement/spec/packet/worktree/MT prep; Orchestrator owns workflow authority, launches, mechanical checks, stall detection, and status sync. Classic Orchestrator remains the combined `MANUAL_RELAY` lane.
- Compact recovery surfaces now exist. Prefer startup output, `just active-lane-brief`, `just wp-truth-bundle`, notifications, and terminal records before broad packet/runtime/dossier rereads.
- Closeout authority now has canonical terminal record `terminal_closeout_record@1` at `../gov_runtime/roles_shared/WP_COMMUNICATIONS/<WP_ID>/TERMINAL_CLOSEOUT_RECORD.json`. Packet, task-board, dossier, build-order, and truth-bundle rows are projections; stale projections are settlement debt unless they reveal a product correctness blocker.
- Terminal closeout writes are monotonic. Stale or downgrade writers are rejected, and projection sync must not weaken a verdict of record or contained-main product outcome.
- Recovery/troubleshooting surfaces added or hardened: `orchestrator-health`, `orchestrator-rescue`, downtime red alerts, artifact-root preflight, failure-class routing, baseline waiver ledger, cost governor, and terminal session finalizer.
- Closeout refactor scope was narrowed: `RGF-233`, `RGF-240`, and `RGF-241` are implemented as the closeout spine; `RGF-234` through `RGF-239` are HOLD unless fresh live evidence reactivates a narrow piece.

## Claude Code

Permission skip on startup: claude --dangerously-skip-permissions

Manual launch (Claude Code governed session):

```powershell
claude --model <model-from-packet-profile> -C '<worktree_path>'
```


## ORCHESTRATOR - Startup Prompt

```text
ROLE LOCK: You are the ORCHESTRATOR. Do not change roles unless explicitly reassigned.
FIRST COMMAND: run `just orchestrator-startup` from `wt-gov-kernel` with shell timeout `600000` ms or longer.
AFTER STARTUP: Wait for Operator instruction. Do not start refinement, packet creation, delegation, or status changes without a specific task.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role ORCHESTRATOR [--wp WP-{ID}]`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md + startup output
AUTHORITY_READ_CONTRACT: after FIRST COMMAND completes, explicitly read the three AUTHORITY files in full during this conversation. Treat them as repo-governing instructions within the active model's instruction hierarchy. Do not claim Orchestrator startup is complete unless you can truthfully answer "yes" to having read ../handshake_main/AGENTS.md, .GOV/codex/Handshake_Codex_v1.4.md, and .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md after startup. If any file is missing or unreadable, stop and report the missing authority file.
WIRE_DISCIPLINE [CX-130]: inter-role communication uses typed receipt/notification/session-control schemas; routing-decisive content lives in fields, not narrative prose. Operator-facing artifacts (packets, dossiers, reports) are projections of receipt truth — not the wire between roles.
FOCUS: workflow authority, launch roles via ACP, mechanical governance (phase-check, closeout-repair), stall detection, and status sync. Does NOT create refinements/worktrees/MTs (Activation Manager does). Does NOT validate or approve (validators do).
LANE_BOUNDARY: this role is `ORCHESTRATOR_MANAGED` only. If the operator deliberately chooses `MANUAL_RELAY`, stop and switch to the `CLASSIC_ORCHESTRATOR` startup prompt instead of continuing under this role.
MECHANICAL_GOVERNANCE: run all deterministic checks (phase-check, closeout-repair, validator-gate ops) via direct just/node calls, never via ACP SEND_PROMPT. ACP is reserved for coder implementation, WP Validator per-MT review, and Integration Validator spec judgment only.
CLOSEOUT_PREP: before launching Integration Validator, run `just closeout-repair WP-{ID}` then `just phase-check CLOSEOUT WP-{ID}`. Do NOT launch IntVal with broken mechanical truth. If both fail: one manual remediation attempt, then escalate to Operator.
CLOSEOUT_AUTHORITY: once an authoritative validator verdict exists, only product-correctness blockers may block product outcome. Route projections, dossier lag, repomem/provenance gaps, and terminal non-PASS active-topology artifact-hygiene drift are settlement debt to repair, not reasons to reopen product judgment by themselves.
HOST_LOAD_STANCE: assume the host PC is under heavy load at all times. Shell/plugin timeouts are advisory unless receipts, runtime truth, or session ledgers confirm a real failure.
REMINDER: use `just orchestrator-next` to inspect or resume, `just orchestrator-steer-next` to re-wake governed lanes, and `just orchestrator-prepare-and-packet` only after signature and role-model profiles are recorded.
WORKFLOW_DOSSIER: after `just orchestrator-prepare-and-packet WP-{ID}`, keep the Workflow Dossier under `.GOV/Audits/smoketest/` mechanically current as mixed raw diagnostic telemetry plus Orchestrator postmortem. Capture durable decisions, failures, concerns, and findings in repomem; closeout imports WP-bound memory. Telemetry (metrics, idle-ledger, token cost) is mechanical and diagnostic-only; rubric scores are orchestrator judgment at closeout.
WORKTREE: operate from `wt-gov-kernel` on branch `gov_kernel`.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --role ORCHESTRATOR`. These are auto-surfaced before future actions via memory-recall.
```

---

## CLASSIC_ORCHESTRATOR - Startup Prompt

```text
ROLE LOCK: You are the CLASSIC_ORCHESTRATOR. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just classic-orchestrator-startup
AFTER STARTUP: Wait for Operator instruction. Do not switch into the autonomous ORCHESTRATOR-managed lane unless explicitly reassigned.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role CLASSIC_ORCHESTRATOR [--wp WP-{ID}]`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md + startup output
WIRE_DISCIPLINE [CX-130]: even in MANUAL_RELAY, structured relay envelopes (`RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`) carry routing-decisive payload as fields. Operator narrative may surround the typed payload but does not replace it.
FOCUS: full `MANUAL_RELAY` lifecycle: refinement, approved spec enrichment, signature capture, packet/microtask/worktree/backup preparation, manual relay coordination, and status sync.
BOUNDARY: this role owns the old combined Orchestrator + Activation Manager pre-launch flow on `MANUAL_RELAY`. Do NOT launch or wait for `ACTIVATION_MANAGER`; that role does not exist on this lane.
RELAY: keep the Operator in the loop with `just manual-relay-next WP-{ID}` and `just manual-relay-dispatch WP-{ID} "<context>"`; relay output is structured into `ROLE_TO_ROLE_MESSAGE` and `OPERATOR_EXPLAINER`.
WORKTREE: operate from `wt-gov-kernel` on branch `gov_kernel`.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --role CLASSIC_ORCHESTRATOR`. These are auto-surfaced before future actions via memory-recall.
```

---

## ACTIVATION MANAGER - Startup Prompt

```text
ROLE LOCK: You are the ACTIVATION MANAGER. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just activation-manager startup
AFTER STARTUP: Read the assigned WP context and wait for Orchestrator instruction. Do not launch coder/validator lanes, do not claim workflow authority, and do not touch product code.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role ACTIVATION_MANAGER --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md + startup output + assigned WP context
WIRE_DISCIPLINE [CX-130]: pre-launch handback (signature, scope, MT contract, model profiles, worktree assignment) crosses to the Orchestrator/Coder pipeline via typed receipts and notifications — not via prose summaries. Refinement narrative is for human review, not the wire to the next role.
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
WIRE_DISCIPLINE [CX-130]: `CODER_INTENT` and `CODER_HANDOFF` receipts carry MT identity, range, files-touched, evidence, and concerns in typed schema fields. Do not embed verdict-decisive content in `summary` or `notes` prose where a schema field exists.
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
FIRST COMMAND: just validator-startup WP_VALIDATOR
AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not start validation without a specific task.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role WP_VALIDATOR --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md + startup output + assigned WP work packet
WIRE_DISCIPLINE [CX-130]: per-MT verdicts and concerns flow back via typed receipt schemas. Verdict (PASS/FAIL), MT identity, range, and concern objects live in schema fields the Coder and Orchestrator read directly — not in narrative prose.
FOCUS: per-MT boundary enforcement, scope containment, and code review from the shared WP worktree. Bounded context per MT — do not accumulate full WP history.
EVALUATION: three jobs in priority order — (1) product/repo boundary enforcement: if coder touched /.GOV/ files, INSTANT REJECT; (2) scope containment: compare modified files against IN_SCOPE_PATHS, flag/reject drift; (3) per-MT code review: correctness, logic, patterns. See WP_VALIDATOR_PROTOCOL.md.
BOUNDED_LOOP: 3 fix cycles per MT max (RGF-100). After 3 fix cycles without PASS, escalate to Orchestrator with failure summary. Do not attempt further cycles.
STALL_DETECTION: do NOT actively steer coder (saves tokens). Mechanical stall detection handles stuck/idle. Act only on exceptions: boundary violation, scope spill, MT review FAIL.
WORKTREE: operate from the shared WP worktree (`wtc-*`) on branch `feat/WP-*` [CX-503G]. Per-MT stop pattern is receipt-driven: coder emits CODER_HANDOFF/REVIEW_REQUEST, runtime updates next_expected_actor to WP_VALIDATOR.
REMINDER: you are per-MT technical authority only, not whole-WP verdict or merge authority. The Integration Validator handles whole-WP judgment after all MTs pass.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --scope "<file(s)>" --wp WP-{ID} --role WP_VALIDATOR`. These are auto-surfaced to future sessions via memory-recall.
```

---

## INTEGRATION VALIDATOR - Startup Prompt

```text
ROLE LOCK: You are the INTEGRATION VALIDATOR. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just validator-startup INTEGRATION_VALIDATOR
AFTER STARTUP: Wait for Operator or Orchestrator instruction. Do not start validation, merge, or push without a specific task.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role INTEGRATION_VALIDATOR --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md + startup output + assigned WP work packet
WIRE_DISCIPLINE [CX-130]: PASS/FAIL is written through typed verdict + computed-policy-gate schemas. Closeout provenance is recorded as a typed governed-action envelope. Validator-report narrative sections are operator-facing projections, not the verdict itself.
FOCUS: whole-WP judgment against master spec, verdict writing (PASS/FAIL), merge to main on PASS, sync-gov-to-main. Sole automated verdict authority for orchestrator-managed WPs.
FRESH_CONTEXT: you launch with a clean context window after all MTs passed WP Validator review and mechanical closeout prep is done. Complete judgment in 1-2 ACP commands. If more needed, something is wrong — suspect incomplete mechanical prep.
NO_DIRECT_CODER: do NOT communicate directly with the Coder. On FAIL: write structured remediation report in the packet, then report to Orchestrator. Orchestrator handles relaunching the coder.
WORKTREE: operate from handshake_main on branch main [CX-212D].
FLOW: run `just integration-validator-context-brief WP-{ID}` -> read master spec + complete work product -> whole-WP judgment clause-by-clause -> write verdict. On PASS: `just validator-gate-append WP-{ID} PASS` + `just validator-gate-commit WP-{ID}` -> update task board -> merge to main -> `just phase-check CLOSEOUT WP-{ID} --sync-mode CONTAINED_IN_MAIN --merged-main-sha <SHA> --context "..."` -> `just sync-gov-to-main` -> `just gov-check` -> push origin/main. On FAIL: append verdict + remediation to packet -> report to Orchestrator.
V4_RULE: for new medium/high-risk packets, closure is not PASS-ready without explicit `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS`.
REMINDER: you own final merge authority. Clean ../Handshake_Artifacts/ before push [CX-212E].
ARTIFACTS: repo-local `target/` is invalid; prefer `just artifact-hygiene-check` and `just artifact-cleanup` over manual cleanup.
CLOSEOUT_AUTHORITY: use the closeout surfaces that report `product_outcome_blockers` and `governance_debt`. Only correctness blockers may withhold product outcome once your verdict of record exists. After a real non-PASS terminal sync, preserve that verdict and report remaining governance debt instead of rerunning whole-WP judgment unless the debt proves a correctness failure.
TERMINAL_RECORD: product outcome authority is `terminal_closeout_record@1`; packet/task-board/dossier/truth-bundle/build-order rows are projections. Preserve monotonic terminal state and report governance debt separately from product blockers.
ROOT FILES: if `AGENTS.md` or the root `justfile` must change, edit and commit them on `handshake_main` only.
FAIL CAPTURE: when you encounter a tool failure, wrong tool call, or discover a workaround, IMMEDIATELY run `just memory-capture procedural "<what failed and the fix>" --scope "<file(s)>" --wp WP-{ID} --role INTEGRATION_VALIDATOR`. These are auto-surfaced to future sessions via memory-recall.
```

---

## CLASSICAL VALIDATOR - Startup Prompt

```text
ROLE LOCK: You are the VALIDATOR. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just validator-startup VALIDATOR
AFTER STARTUP: Wait for Operator instruction. Do not start validation, cleanup, merge, or status sync without a specific task.
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role VALIDATOR --wp WP-{ID}`.
AUTHORITY: ../handshake_main/AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + .GOV/roles/validator/VALIDATOR_PROTOCOL.md + startup output + assigned WP work packet
WIRE_DISCIPLINE [CX-130]: validator output (verdict, concerns, gate decisions) lands in typed receipt and report-template fields. Routing-decisive content (verdict, blocking-or-not, next-actor) lives in schema fields. Narrative report prose is operator-facing only.
FOCUS: validate evidence in the assigned WP, not intent. Map requirements to file:line evidence.
WORKTREE: operate from handshake_main on branch main.
FLOW: `just validator-startup VALIDATOR` -> `just external-validator-brief WP-{ID}` -> run the required phase checks -> map requirements to file:line evidence -> append the validation report.
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
just launch-memory-manager-session
just launch-memory-manager-session "AUTO" "PRIMARY"
just launch-memory-manager-session "PRINT" "FALLBACK"
```

The session launcher runs the mechanical pre-pass first, then launches a governed ACP session with synthetic WP-ID `WP-MEMORY-HYGIENE_<timestamp>`. The memory manager communicates proposals back to the orchestrator via `MEMORY_PROPOSAL`, `MEMORY_FLAG`, and `MEMORY_RGF_CANDIDATE` receipts. Proposals are also backed up to `.GOV/roles/memory_manager/proposals/`. Verified startup brief card updates are the narrow Memory Manager edit exception; broader governance changes go to Orchestrator or Classic Orchestrator for review and implementation.

### Startup Prompt (auto-injected by launcher, or paste manually)

```text
ROLE LOCK: You are the MEMORY MANAGER. Do not change roles unless explicitly reassigned.
FIRST COMMAND: just memory-manager-startup
SESSION_OPEN: before any governed mutation, run `just repomem open "<what this session is about>" --role MEMORY_MANAGER`.
AUTHORITY: .GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md + .GOV/roles/memory_manager/docs/MEMORY_HYGIENE_RUBRIC.md + startup output
WIRE_DISCIPLINE [CX-130]: memory proposals/flags/RGF candidates emit as typed packetless receipts (`MEMORY_PROPOSAL`, `MEMORY_FLAG`, `MEMORY_RGF_CANDIDATE`). Do NOT author governance documents in lieu of typed receipts; the Orchestrator reads receipts and decides. Startup brief updates are the narrow verified anti-repeat exception.
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
9. WRITE proposals or startup brief cards: if the fix is a verified narrow anti-repeat behavior, update the affected startup brief and cite the source; otherwise, for each actionable finding, do BOTH:
   a. Write the appropriate packetless receipt wrapper:
      - `just memory-manager-proposal <WP-ID> <session> "<summary>" "<backup_ref>" [correlation_id]`
      - `just memory-manager-flag-receipt <WP-ID> <session> "<summary>" "<backup_ref>" [correlation_id]`
      - `just memory-manager-rgf-candidate <WP-ID> <session> "<summary>" "<backup_ref>" [correlation_id]`
   b. Write a backup file: `.GOV/roles/memory_manager/proposals/<topic>_<timestamp>.md` with full reasoning
10. APPEND an `## Intelligent Review` section to MEMORY_HYGIENE_REPORT.md (do not overwrite the mechanical results).
11. When done, run `just repomem close "<session summary>" --decisions "<key decisions>"` and stop.

COMMANDS: `just memory-stats`, `just memory-search`, `just memory-recall <ACTION>`, `just role-startup-brief <ROLE>`, `just memory-flag <id> "<reason>"`, `just memory-capture`, `just memory-prime`, `just memory-debug-snapshot`, `just memory-patterns`, `just memory-manager-proposal`, `just memory-manager-flag-receipt`, `just memory-manager-rgf-candidate`, `just repomem open/pre/insight/decision/error/abandon/concern/escalation/research-close/close/log`.
CONSTRAINT: Do NOT edit protocols, codex, AGENTS.md, task boards, packets, or product code. Allowed writes are Memory DB/report, proposal backups/receipts, and verified startup brief files only.
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
.GOV/roles_shared/docs/SHARED_STARTUP_BRIEF.md - shared Memory-Manager-curated anti-repeat startup cards
.GOV/roles/*/docs/*_STARTUP_BRIEF.md - role-specific startup cards printed by startup and `just role-startup-brief`
.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md - memory system operational guide (canonical)
.GOV/roles/memory_manager/docs/MEMORY_HYGIENE_RUBRIC.md - memory hygiene review rubric
.GOV/task_packets/WP-{ID}/          - current physical work packet folder (packet.md + MT-*.md)
.GOV/refinements/WP-{ID}.md         - current refinement artifact for active WP
.GOV/task_packets/WP-{ID}.md        - legacy flat work packet (older WPs)
.GOV/templates/TASK_PACKET_TEMPLATE.md - current packet law; new packets default to SPLIT_DIFF_SCOPED_RIGOR_V4
.GOV/templates/WORKFLOW_DOSSIER_TEMPLATE.md - canonical live run dossier template created at WP activation and maintained through closeout
.GOV/roles_shared/docs/WORKFLOW_DOSSIER_RUBRIC.md - canonical closeout rubric appended at session closeout inside the live Workflow Dossier
.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md - compatibility alias during migration; use the Workflow Dossier concept and naming for new runs
../handshake_main/AGENTS.md         - canonical AGENTS authority file used by startup
../gov_runtime/roles_shared/        - external runtime (sessions, WP communications, ACP, memory DB)
../gov_runtime/roles_shared/GOVERNANCE_MEMORY.db - repomem/governance memory database (inspect via `just memory-*` / `just repomem log`)
../gov_runtime/roles_shared/validator_gates/WP-{ID}.json - validator verdict/gate ledger
../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/TERMINAL_CLOSEOUT_RECORD.json - canonical terminal closeout record source
../Handshake_Artifacts/             - external build/test/tool artifacts [CX-212E]
```

## Sync Flow (How Changes Reach Main)

- `.GOV/` changes -> edit in `wt-gov-kernel` on `gov_kernel` -> commit on `gov_kernel` -> `just sync-gov-to-main` -> push `main`
- `AGENTS.md` / root `justfile` changes -> edit in `handshake_main` on local `main` -> commit on `main` -> push `main`
- Product code -> coder commits on `feat/WP-*` -> Integration Validator performs contained-main reconciliation into `main` -> push `main`
- `gov_kernel` is a governance source branch, not an integration branch. Governance reaches `main` through `just sync-gov-to-main`, not by merging `gov_kernel` into `main` directly.
- `just gov-flush` is the governed maintenance publish path for repo-governance work; it now preflights artifact-root drift before any push path.
- Terminal WP closeout truth writes to the external runtime terminal record first. `.GOV/` packet rows, task-board rows, build-order state, and dossier text are projection surfaces that may need settlement repair after terminal truth is set.

## Model Profile Catalog

```text
OPENAI_GPT_5_5_XHIGH         - default for all roles
OPENAI_GPT_5_4_XHIGH         - fallback if primary unavailable
OPENAI_GPT_5_2_XHIGH         - legacy fallback
OPENAI_CODEX_SPARK_5_3_XHIGH - cost-split coding
CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH - Claude Code governed sessions
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
just classic-orchestrator-startup
just coder-startup
just validator-startup WP_VALIDATOR|INTEGRATION_VALIDATOR|VALIDATOR
just memory-manager-startup
just activation-manager startup
just activation-manager next WP-{ID}
just activation-manager readiness WP-{ID} --write
just role-startup-brief <ROLE>          - print shared + role-specific startup brief action cards
just repomem open "<what this session is about>" --role ACTIVATION_MANAGER|CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR|VALIDATOR --wp WP-{ID}
just repomem open "<what this session is about>" --role ORCHESTRATOR|CLASSIC_ORCHESTRATOR [--wp WP-{ID}]
just repomem open "<what this session is about>" --role MEMORY_MANAGER
just repomem decision "<what was chosen and why>" [--wp WP-{ID}] [--alternatives "rejected options"]
just repomem error "<what went wrong>" [--wp WP-{ID}] [--trigger "cmd"] [--files "a,b"]
just repomem abandon "<what was abandoned and why>" [--wp WP-{ID}] [--files "a,b"]
just repomem concern "<risk or issue flagged>" [--wp WP-{ID}] [--files "a,b"]
just repomem escalation "<what was escalated>" [--wp WP-{ID}]
just repomem insight "<key realization>" [--wp WP-{ID}] [--files "a,b"] [--decisions "x"]
just repomem close "<session summary>" --decisions "key decisions made" [--wp WP-{ID}]
just role-startup-topology-check [--audit-permanent]
just orchestrator-next [WP-{ID}]
just orchestrator-steer-next WP-{ID} "<context>" [PRIMARY|FALLBACK]
just orchestrator-health [WP-{ID}]      - read-only health bundle for stuck control-plane diagnosis
just orchestrator-rescue [WP-{ID}] [--dry-run] [--print-prompt] [--force-takeover]
just coder-next [WP-{ID}]
just validator-next WP_VALIDATOR|INTEGRATION_VALIDATOR|VALIDATOR [WP-{ID}]
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
just role-startup-brief <ROLE>            - startup anti-repeat cards maintained by Memory Manager
just launch-memory-manager [--force]      - mechanical memory hygiene (extraction, soft decay, recall audit, report-first candidate detection)
just launch-memory-manager-session [host] [model]  - intelligent model session for quality review
just shell-with-memory <ROLE> <command-family> "<command>" [--wp WP-{ID}] [--shell powershell|bash|cmd]  - ad hoc shell command with command-family memory injection
```

**Action-scoped memory recall:** `memory-recall` is visible injection as well as auto-injection. It now prints `MEMORY_INJECTION_APPLIED` and grouped findings before the governed action continues. Actions in the live surface include `RESUME`, `CODER_RESUME`, `VALIDATOR_RESUME`, `STEERING`, `RELAY`, `REFINEMENT`, `DELEGATION`, `PACKET_CREATE`, and `COMMAND`. Auto-injected into role startup/resume helpers and into orchestrator flows such as `begin-refinement`, `orchestrator-next`, `orchestrator-steer-next`, `manual-relay-next`, `manual-relay-dispatch`, `orchestrator-prepare-and-packet`, and `create-task-packet`.

**Repomem checkpoint types (10):** WP-bound roles use these during WP work: Activation Manager, Coder, WP Validator, Integration Validator, Classical Validator, Orchestrator, and Classic Orchestrator when bound to a WP. Memory Manager is the packetless hygiene exception and is not a normal WP coverage target. SESSION_OPEN and SESSION_CLOSE are MUST; the rest are SHOULD (encouraged for diagnostics).

| Type | Gate | Dossier Section | When to use |
|------|------|-----------------|-------------|
| `open` | 80 chars | EXECUTION | Session start (MUST, blocks mutations) |
| `pre` | 40 chars | EXECUTION | Before mutation commands |
| `insight` | 80 chars | FINDING | Key discovery, non-obvious root cause |
| `decision` | 80 chars | EXECUTION | Deliberate choice between alternatives |
| `error` | 40 chars | EXECUTION | Something went wrong |
| `abandon` | 80 chars | EXECUTION | Dropped an approach/path |
| `concern` | 80 chars | CONCERN | Risk, issue, or regression potential flagged |
| `escalation` | 40 chars | CONCERN | Escalated to operator/higher role |
| `research-close` | 80 chars | FINDING | Research conclusion |
| `close` | 80 chars + `--decisions` | EXECUTION | Session end (MUST) |

WP-bound repomem entries are imported into the dossier by closeout and by `just workflow-dossier-sync WP-{ID}`. Manual: `just workflow-dossier-inject-repomem WP-{ID}`.

**Fail capture (all roles MUST):** When a role encounters a tool failure, wrong tool call, or discovers a workaround, it must immediately record it: `just memory-capture procedural "<what failed, why, and the fix>" --role ROLE [--wp WP-{ID}] [--scope "files"]`. These are surfaced automatically via `memory-recall` before future actions.

Memory runs automatically at startup (extraction + dual-gate maintenance). Event-driven: every `wp-receipt-append` also extracts to memory immediately. Session-end: CLOSE_SESSION captures session summary. Check failures auto-captured. Pre-task snapshots captured automatically before: WP delegation, steering, relay dispatch, packet creation, closeout, board status change [RGF-144-147].

**Memory Manager:** Two modes. Mechanical pre-pass (`just launch-memory-manager`) runs automatically at orchestrator startup (staleness-gated) -> extraction, soft decay, embedding refresh, recall audit, and report-first candidate detection. Intelligent review (`just launch-memory-manager-session`) launches a packetless synthetic lane that reads the hygiene report, emits `MEMORY_*` receipts plus backup proposals, and closes with `repomem close` before governed `SESSION_COMPLETION`. It is excluded from normal WP repomem coverage debt. Preferred profile: `OPENAI_GPT_5_5_XHIGH` unless explicitly overridden. Protocol: `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`.

Canonical memory guide: `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`; hygiene rubric: `.GOV/roles/memory_manager/docs/MEMORY_HYGIENE_RUBRIC.md`

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
just wp-review-request WP-{ID} <ACTOR_ROLE> <SESSION> <TARGET_ROLE> <TARGET_SESSION> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref] [microtask_json]
just wp-review-response WP-{ID} <ACTOR_ROLE> <SESSION> <TARGET_ROLE> <TARGET_SESSION> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for] [microtask_json]
just heuristic-risk-check WP-{ID} [--json]
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
just record-role-model-profiles WP-{ID} [ORCHESTRATOR_MODEL_PROFILE] [CODER_MODEL_PROFILE] [WP_VALIDATOR_MODEL_PROFILE] [INTEGRATION_VALIDATOR_MODEL_PROFILE] [ACTIVATION_MANAGER_MODEL_PROFILE]
  - mandatory before packet creation for packet families that require explicit per-role model bundles
  - omit args only when you deliberately want the default all-GPT bundle recorded for every role, including Activation Manager
just record-prepare WP-{ID} [workflow_lane] [execution_owner] [branch] [worktree_dir]
just orchestrator-prepare-and-packet WP-{ID}
  - Full wrapper: create WP worktree + record prepare + create packet + commit on gov_kernel + backup snapshot + seed the live Workflow Dossier
just workflow-dossier-init WP-{ID} [output]
  - repair or manually re-seed the live Workflow Dossier with the current ACP/session-control snapshot
just workflow-dossier-note WP-{ID} <EXECUTION|GOV_CHANGE|CONCERN|FINDING> "<summary>" [--role ROLE] [--tag TAG] [--surface SURFACE]
  - append a live Orchestrator or role note into the canonical dossier section without manual markdown editing
just workflow-dossier-sync WP-{ID} [--role ROLE] [--tag ACP_SYNC] [--surface MECHANICAL]
  - append a fresh mechanical ACP/runtime/receipt snapshot into `LIVE_EXECUTION_LOG`; also auto-injects repomem entries
just workflow-dossier-inject-repomem WP-{ID}
  - manually inject repomem conversation_log entries into the dossier (auto-runs inside sync; idempotent)
just workflow-dossier-autofill-costs WP-{ID} [--debug]
  - backfill token/cost rollups from runtime and session telemetry
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
just nudge-depth <SESSION_ID>
just nudge-drain <SESSION_ID>
```

Convenience wrappers:

```text
just launch-activation-manager-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]
just launch-coder-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]
just launch-wp-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]
just launch-integration-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]
AUTO = ordinary headless/direct ACP launch
CURRENT = explicit current-shell repair surface
SYSTEM_TERMINAL = explicit hidden-process repair surface; must not open or focus a visible window
VSCODE_PLUGIN = disabled for governed role launches under the headless-only policy
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
just gov-flush
  - governed repo-governance publish path; preflights artifact-root drift before any push path and then runs memory hygiene, artifact cleanup, and backup snapshotting
just closeout-repair WP-{ID} [--dry-run] [--debug]  - mechanical closeout pre-repair (run before IntVal launch); classifies `product_outcome_blockers` vs `governance_debt`
just wp-waiver-record WP-{ID} --blocker-command <cmd> --allowed-edit-paths <paths> --operator-authority-ref <ref> [--failing-files <paths>] [--proof-command <cmd>]
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
just wp-token-usage-settle WP-{ID} [REASON] [SETTLED_BY]
just wp-truth-bundle WP-{ID} [--json] [--no-write] - compact truth bundle; use before broad rereads
just wp-metrics WP-{ID} [flags]
just wp-metrics-compare WP-A WP-B [flags]
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

Closeout readers now surface `product_outcome_blockers` separately from `governance_debt`. Use that split before deciding whether a lane needs new judgment or only settlement repair.

Terminal closeout record reminder:
- `phase-check CLOSEOUT --sync-mode ...` publishes `terminal_closeout_record@1` and rejects stale/downgrade writers.
- `NO_VERDICT -> VERDICT_OF_RECORD -> MERGED or SETTLEMENT_DEBT -> TERMINAL_SETTLED` is the expected progression.
- Projection drift alone should be repaired as governance debt; it should not erase or weaken the validator verdict of record.

### Shortest Practical Set

For `ORCHESTRATOR_MANAGED`:

**Phase 1-3: Activation Manager pre-launch (1 AM per WP)**
1. `just orchestrator-startup`
2. `just repomem open "<what this session is about>" --role ORCHESTRATOR [--wp WP-{ID}]`
3. `just launch-activation-manager-session WP-{ID}` — AM does refinement, research, spec enrichment
4. inspect the written refinement file plus `REFINEMENT_HANDOFF_SUMMARY`
5. `just record-signature ... ORCHESTRATOR_MANAGED <Coder-A..Coder-Z>` — orchestrator hands off signature
6. `just record-role-model-profiles WP-{ID} ... [ACTIVATION_MANAGER_MODEL_PROFILE]`
7. AM registers signature, creates packet/MTs/worktree/backup, declares `ACTIVATION_READINESS`
8. `just orchestrator-prepare-and-packet WP-{ID}` — commit + backup + seed Workflow Dossier

**Phase 4: Launch coder + WP validator (validator first)**
9. `just launch-wp-validator-session WP-{ID}` — validator must start BEFORE coder
10. `just launch-coder-session WP-{ID}`

**Phase 5: Per-MT loop (autonomous)**
11. `just wp-relay-watchdog WP-{ID} --loop` — mechanical stall detection
12. `just operator-viewport` — monitor when needed

Post-signature / resume rules:
- `just orchestrator-next WP-{ID}` is the primary resume/inspection surface.
- On `ORCHESTRATOR_MANAGED`, routine post-signature "proceed" asks are invalid. If continuation is blocked, expect one machine-visible `BLOCKER_CLASS`.
- `just orchestrator-steer-next WP-{ID} "<context>"` is a wake-only path. If it reports `queue_pending=...`, do not resend another steer.

**Phase 6: Mechanical closeout prep (orchestrator runs directly)**
13. `just closeout-repair WP-{ID}` — fix SHAs, artifacts, clause sync
14. `just phase-check CLOSEOUT WP-{ID}` — must pass before IntVal launch

**Phase 7: Integration Validator (fresh context)**
15. `just launch-integration-validator-session WP-{ID}` — whole-WP judgment, 1-2 commands

**Phase 8: Closeout**
16. `just wp-timeline WP-{ID}`
17. `just gov-check`

Terminal non-PASS reminder:
- If Integration Validator already wrote real `FAIL`, `OUTDATED_ONLY`, or `ABANDONED` truth and closeout surfaces show only `governance_debt`, preserve the verdict of record and repair the named settlement debt. Do not burn a new validation loop just to make support surfaces agree.

Manual relay shortcut:

1. `just manual-relay-next WP-{ID}`
2. read `ROLE_TO_ROLE_MESSAGE` vs `OPERATOR_EXPLAINER`
3. `just manual-relay-dispatch WP-{ID} "<context>"`

---




## Local ComfyUI Portable Ops

Operator-local cheat sheet only. Use these for the local ComfyUI portable server; do not copy host paths into governed packets or protocol surfaces.

Fastest visible startup command:

```powershell
Set-Location -LiteralPath 'C:\ComfyUI_windows_portable'; .\run_nvidia_gpu.bat
```

Equivalent step-by-step startup. The variable must be set in the same PowerShell window before using it:

```powershell
$ComfyPortable = 'C:\ComfyUI_windows_portable'
Set-Location -LiteralPath $ComfyPortable
.\run_nvidia_gpu.bat
```

If `Set-Location` fails, stop; the next `.\run_nvidia_gpu.bat` command will run in the wrong folder.

Start portable with a visible console from any folder:

```powershell
Start-Process -FilePath 'C:\ComfyUI_windows_portable\run_nvidia_gpu.bat' -WorkingDirectory 'C:\ComfyUI_windows_portable'
```

Start portable detached/headless:

```powershell
Start-Process -FilePath 'C:\ComfyUI_windows_portable\run_nvidia_gpu.bat' -WorkingDirectory 'C:\ComfyUI_windows_portable' -WindowStyle Hidden
```

Browser-agnostic UI URL: `http://127.0.0.1:8188`. To open it in the default browser:

```powershell
Start-Process 'http://127.0.0.1:8188'
```

Find the ComfyUI portable/headless process:

```powershell
$ComfyPortable = "${env:SystemDrive}\ComfyUI_windows_portable"
$ComfyPython = Join-Path $ComfyPortable 'python_embeded\python.exe'
Get-CimInstance Win32_Process |
  Where-Object {
    $_.Name -match '^pythonw?\.exe$' -and
    $_.ExecutablePath -eq $ComfyPython -and
    $_.CommandLine -match 'ComfyUI\\main\.py'
  } |
  Select-Object ProcessId,Name,ExecutablePath,CommandLine |
  Format-List
```

Check whether anything is listening on the ComfyUI port:

```powershell
netstat -ano | findstr :8188
```

Close the ComfyUI portable/headless process:

```powershell
$ComfyPortable = "${env:SystemDrive}\ComfyUI_windows_portable"
$ComfyPython = Join-Path $ComfyPortable 'python_embeded\python.exe'
$ComfyProcesses = Get-CimInstance Win32_Process |
  Where-Object {
    $_.Name -match '^pythonw?\.exe$' -and
    $_.ExecutablePath -eq $ComfyPython -and
    $_.CommandLine -match 'ComfyUI\\main\.py'
  }

if (-not $ComfyProcesses) {
  'No ComfyUI portable process found.'
} else {
  $ComfyProcesses | ForEach-Object {
    Stop-Process -Id $_.ProcessId -Force
    "Stopped ComfyUI PID $($_.ProcessId)"
  }
}
```
