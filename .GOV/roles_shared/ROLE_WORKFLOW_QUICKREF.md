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

Product-scanning / product-boundary enforcement:
- `just codex-check` (includes hard boundary checks for `.GOV` references in product code)
- `just product-scan` (alias) / `just validator-scan` (forbidden patterns in product sources)
- `just validate` (full product hygiene: frontend + backend tests, etc.)

## Session Host + Operator Monitor

- When available, prefer VS Code integrated terminals for multi-session work instead of many floating desktop terminals.
- Recommended VS Code tabs:
  - `ORCH`
  - `CODER <WP_ID>`
  - `WPVAL <WP_ID>`
  - `INTVAL`
  - `MONITOR`
- `just operator-monitor` provides the overview surface for active WPs, authorities, heartbeats, and WP-scoped communications.
- `just wp-thread-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> "<message>" [target]` appends a freeform message to the packet-declared `WP_COMMUNICATION_DIR`.

## Role: Orchestrator

Authoritative inputs:
- `.GOV/roles_shared/SPEC_CURRENT.md` (current binding spec pointer)
- `.GOV/roles_shared/TASK_BOARD.md`
- `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json` (mechanical gate state)
- Active WP packet + `WP_COMMUNICATIONS` artifacts when declared

Primary commands:
- `just record-refinement WP-...`
- `just record-signature WP-... <sig> <MANUAL_RELAY|ORCHESTRATOR_MANAGED> <Coder-A|Coder-B>`
- `just worktree-add WP-...`
- `just record-prepare WP-... [<MANUAL_RELAY|ORCHESTRATOR_MANAGED>] [<Coder-A|Coder-B>] [branch] [worktree_dir]`
- `just create-task-packet WP-...`
- `just orchestrator-worktree-and-packet WP-...`
- `just orchestrator-prepare-and-packet WP-... [<MANUAL_RELAY|ORCHESTRATOR_MANAGED>] [<Coder-A|Coder-B>]`
- `just pre-work WP-...`
- `just wp-heartbeat WP-... ORCHESTRATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir]`
- `just wp-receipt-append WP-... ORCHESTRATOR <session> <receipt_kind> "<summary>"`
- `just wp-thread-append WP-... ORCHESTRATOR <session> "<message>" [target]`
- `just operator-monitor`

Role rule:
- The Orchestrator is non-agentic. It coordinates sessions and governance state, but does not spawn Orchestrator or Validator helper agents.
- The Orchestrator is workflow authority. It does not become final technical or merge authority.

## Role: Coder

Primary commands:
- `just pre-work WP-...`
- Implement only within `IN_SCOPE_PATHS`
- Hygiene: `just product-scan`, `just validator-dal-audit`, `just validator-git-hygiene`
- Workflow closure evidence: `just post-work WP-...`
- `just wp-heartbeat WP-... CODER <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir]`
- `just wp-receipt-append WP-... CODER <session> <receipt_kind> "<summary>"`
- `just wp-thread-append WP-... CODER <session> "<message>" [target]`

Role rule:
- Only the Primary Coder may use sub-agents, and only when the packet explicitly allows it.
- Coders coordinate through the packet-declared `WP_COMMUNICATION_DIR`, not through role-local inboxes.

## Role: Validator

Primary commands (per WP validation):
- `just gate-check WP-...`
- `just post-work WP-...`
- `just validator-dal-audit`
- `just validator-git-hygiene`
- `just codex-check` (product boundary enforcement)
- `just wp-heartbeat WP-... VALIDATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir]`
- `just wp-receipt-append WP-... VALIDATOR <session> <receipt_kind> "<summary>"`
- `just wp-thread-append WP-... VALIDATOR <session> "<message>" [target]`

Governance-only work:
- `just gov-check`

File-touch map:
- `.GOV/roles_shared/VALIDATOR_FILE_TOUCH_MAP.md`

Role rule:
- The Validator is non-agentic. Validation work must remain in the Validator session and packet evidence, not delegated to helper agents.
- Validator authority is layered: WP Validator is advisory; Integration Validator owns final technical and merge authority unless the packet explicitly overrides it.
