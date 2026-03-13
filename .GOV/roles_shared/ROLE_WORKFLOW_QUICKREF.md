# Role Workflow Quick Reference (Drive-Agnostic + Operator UX)

This doc is a compact index of the role-governed workflow so the Operator can quickly sanity-check:
- what each role is supposed to do,
- which files are authoritative,
- which commands are governance-only vs product-scanning,
- and where drive-specific paths are forbidden.

## Drive-Agnostic Rules (Repo Governance)

- Authority: `Handshake Codex v1.4.md` [CX-109], [CX-110].
- Role worktree layout is defined in `.GOV/roles_shared/ROLE_WORKTREES.md` using placeholders:
  - `<HANDSHAKE_ROOT>` (example: `P:\Handshake`)
  - `<HANDSHAKE_WORKTREES>` = `<HANDSHAKE_ROOT>\Handshake Worktrees`
- WP worktree assignments MUST be recorded as repo-relative paths:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` `PREPARE.worktree_dir` should be like `../wt-WP-...`
  - Absolute paths (for example `<DRIVE>:\...` or `\\server\share\...`) are forbidden for `worktree_dir` and are blocked by the Orchestrator gate.

## Operator UX (Chat Output Order)

All roles SHOULD follow a strict ordering to avoid interleaving narrative with evidence:
1) `ROLE=...` header line (per Operator instructions)
2) `LIFECYCLE [CX-LIFE-001]`
3) `OPERATOR_ACTION: NONE` (or one explicit decision needed)
4) `STATE: ...` (1 line)
5) Short `FINDINGS:` bullets (2-6 lines max)
6) If a gate command ran: paste `GATE_OUTPUT [CX-GATE-UX-001]` as a single verbatim block
7) Then `GATE_STATUS [CX-GATE-UX-001]` + `NEXT_COMMANDS [CX-GATE-UX-001]` (copy/paste ready)

## Resume After Reset / Compaction

Use the role-specific read-only resume helper immediately after `just <role>-startup` when a session resets or context compacts:
- Orchestrator: `just orchestrator-next [WP-{ID}]`
- Coder: `just coder-next [WP-{ID}]`
- Validator: `just validator-next [WP-{ID}]`

Rule:
- If the helper prints `OPERATOR_ACTION: NONE`, continue directly to `NEXT_COMMANDS`.
- Do not wait for a fresh "proceed" after a startup/preflight rerun unless the helper says a single explicit decision is required.
- Read the helper's `CONFIDENCE: HIGH|MEDIUM|LOW` line as the inference strength for the resumed WP selection.

## Governance vs Product Checks

Governance-only (does not scan `src/` or `app/`):
- `just gov-check`
- Governance-only maintenance does not require a Work Packet or USER_SIGNATURE (Codex [CX-111]).
- Shared repo tooling notes live in `.GOV/roles_shared/TOOLING_GUARDRAILS.md`; use it as short append-only shared tooling memory, not as a second LAW surface.

Product-scanning / product-boundary enforcement:
- `just codex-check` (includes hard boundary checks for `.GOV` references in product code)
- `just product-scan` (alias) / `just validator-scan` (forbidden patterns in product sources)
- `just validate` (full product hygiene: frontend + backend tests, etc.)

## Session Host + Operator Monitor

- When available, prefer VS Code integrated terminals for multi-session work instead of many floating desktop terminals.
- Do not rely on ambient editor defaults for repo-governed session model choice or reasoning strength. New packets/stubs assume `gpt-5.4` primary, `gpt-5.2` fallback, and `model_reasoning_effort=xhigh`.
- Repo-governed role-session start is `ORCHESTRATOR_ONLY`.
- Primary launch path is the VS Code session bridge over `.GOV/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl` + `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`.
- Primary steering lane is the governed Codex thread control path over `.GOV/roles_shared/SESSION_CONTROL_REQUESTS.jsonl` + `.GOV/roles_shared/SESSION_CONTROL_RESULTS.jsonl`.
- CLI escalation windows are allowed only after 2 plugin failures/timeouts for the same role/WP session.
- Recommended VS Code tabs:
  - `ORCH`
  - `CODER <WP_ID>`
  - `WPVAL <WP_ID>`
  - `INTVAL`
  - `MONITOR`
- `just operator-monitor` is the Operator viewport.
- When the canonical `main` task board is available, the monitor uses that board for counts, filter buckets, and WP list selection. The current worktree board is still surfaced as the mirror/drift comparison source.
- Default `SESSIONS` view is governed-session-first: repo-governed ACP sessions are shown as first-class active sessions, with packet runtime sessions shown separately.
- `EVENTS` shows the merged governed ACP output stream for the selected WP across its governed role sessions.
- `TIMELINE` merges thread entries, receipts, governed control requests/results, and ACP events in timestamp order for the selected WP.
- `just session-start <ROLE> WP-...` starts a steerable governed thread for that role/WP session.
- `just session-send <ROLE> WP-... "<prompt>"` resumes that governed thread and records append-only request/result artifacts.
- `just session-cancel <ROLE> WP-...` requests cancellation of the currently running governed command for that role/WP session.
- `just wp-thread-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> "<message>" [target]` appends a freeform message to the packet-declared `WP_COMMUNICATION_DIR` and writes a paired `THREAD_MESSAGE` receipt.

## Role: Orchestrator

Authoritative inputs:
- `.GOV/roles_shared/SPEC_CURRENT.md` (current binding spec pointer)
- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` (mechanical gate state)
- Active WP packet + `WP_COMMUNICATIONS` artifacts when declared

Primary commands:
- `just record-refinement WP-...`
- `just record-signature WP-... <sig> <MANUAL_RELAY|ORCHESTRATOR_MANAGED> <Coder-A..Coder-Z>`
- `just worktree-add WP-...`
- `just record-prepare WP-... [<MANUAL_RELAY|ORCHESTRATOR_MANAGED>] [<Coder-A..Coder-Z>] [branch] [worktree_dir]`
- `just create-task-packet WP-...`
- `just orchestrator-worktree-and-packet WP-...`
- `just orchestrator-prepare-and-packet WP-... [<MANUAL_RELAY|ORCHESTRATOR_MANAGED>] [<Coder-A..Coder-Z>]`
- `just coder-worktree-add WP-...`
- `just wp-validator-worktree-add WP-...`
- `just integration-validator-worktree-add WP-...`
- `just launch-coder-session WP-... [AUTO|PRINT|CURRENT|WINDOWS_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-wp-validator-session WP-... [AUTO|PRINT|CURRENT|WINDOWS_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-integration-validator-session WP-... [AUTO|PRINT|CURRENT|WINDOWS_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just start-coder-session WP-... [PRIMARY|FALLBACK]`
- `just start-wp-validator-session WP-... [PRIMARY|FALLBACK]`
- `just start-integration-validator-session WP-... [PRIMARY|FALLBACK]`
- `just steer-coder-session WP-... "<prompt>" [PRIMARY|FALLBACK]`
- `just cancel-coder-session WP-...`
- `just steer-wp-validator-session WP-... "<prompt>" [PRIMARY|FALLBACK]`
- `just cancel-wp-validator-session WP-...`
- `just steer-integration-validator-session WP-... "<prompt>" [PRIMARY|FALLBACK]`
- `just cancel-integration-validator-session WP-...`
- `just session-start <ROLE> WP-... [PRIMARY|FALLBACK]`
- `just session-send <ROLE> WP-... "<prompt>" [PRIMARY|FALLBACK]`
- `just session-cancel <ROLE> WP-...`
- `just session-registry-status [WP-...]`
- `just pre-work WP-...`
- `just wp-heartbeat WP-... ORCHESTRATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir]`
- `just wp-receipt-append WP-... ORCHESTRATOR <session> <receipt_kind> "<summary>"`
- `just wp-thread-append WP-... ORCHESTRATOR <session> "<message>" [target]`
- `just operator-monitor`

Role rule:
- The Orchestrator is one non-agentic coordinator CLI session. It coordinates and launches repo-governed CLI sessions, but does not spawn Orchestrator or Validator helper agents.
- The Orchestrator is workflow authority. It does not become final technical or merge authority.

## Role: Coder

Primary commands:
- `just pre-work WP-...`
- Implement only within `IN_SCOPE_PATHS`
- Hygiene: `just product-scan`, `just validator-dal-audit`, `just validator-git-hygiene`
- Workflow closure evidence: `just post-work WP-...`
- Session start/steering: `just start-coder-session WP-...`, `just steer-coder-session WP-... "<prompt>"`
- `just wp-heartbeat WP-... CODER <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir]`
- `just wp-receipt-append WP-... CODER <session> <receipt_kind> "<summary>"`
- `just wp-thread-append WP-... CODER <session> "<message>" [target]`

Role rule:
- Only the Primary Coder may use sub-agents, and only when the packet explicitly allows it.
- Coders coordinate through the packet-declared `WP_COMMUNICATION_DIR`, not through role-local inboxes.
- Coders do not self-start fresh repo-governed sessions; they continue in sessions started by the Orchestrator or in an Orchestrator-opened CLI escalation window.

## Role: Validator

Primary commands (per WP validation):
- `just gate-check WP-...`
- `just validator-handoff-check WP-...`
- `just post-work WP-...` (local mirror sanity only unless you are explicitly validating the committed PREPARE target)
- `just validator-dal-audit`
- `just validator-git-hygiene`
- `just codex-check` (product boundary enforcement)
- Session start/steering: `just start-wp-validator-session WP-...`, `just steer-wp-validator-session WP-... "<prompt>"`
- `just wp-heartbeat WP-... VALIDATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir]`
- `just wp-receipt-append WP-... VALIDATOR <session> <receipt_kind> "<summary>"`
- `just wp-thread-append WP-... VALIDATOR <session> "<message>" [target]`

Governance-only work:
- `just gov-check`

File-touch map:
- `.GOV/roles_shared/VALIDATOR_FILE_TOUCH_MAP.md`

Role rule:
- Validator duties are non-agentic, but repo workflows may run multiple validator CLI sessions when they are explicitly scoped as WP Validator and Integration Validator sessions.
- Validator authority is layered: WP Validator is advisory; Integration Validator owns final technical and merge authority unless the packet explicitly overrides it.
- Validator sessions are started by the Orchestrator; validators do not self-start new repo-governed sessions.
- For orchestrator-managed WPs, PASS commit clearance now depends on committed handoff validation against the PREPARE worktree source of truth, recorded via `just validator-handoff-check WP-...`.
