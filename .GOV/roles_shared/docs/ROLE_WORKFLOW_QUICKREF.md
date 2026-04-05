# Role Workflow Quick Reference (Drive-Agnostic + Operator UX)

This doc is a compact index of the role-governed workflow so the Operator can quickly sanity-check:
- what each role is supposed to do,
- which files are authoritative,
- which commands are governance-only vs product-scanning,
- and where drive-specific paths are forbidden.

## Drive-Agnostic Rules (Repo Governance)

- Authority: `.GOV/codex/Handshake_Codex_v1.4.md` [CX-109], [CX-110].
- Role worktree layout is defined in `.GOV/roles_shared/docs/ROLE_WORKTREES.md` using placeholders:
  - `<HANDSHAKE_ROOT>` (example: `/workspace/handshake`)
  - `<HANDSHAKE_WORKTREES>` = `<HANDSHAKE_ROOT>/Handshake Worktrees`
- WP worktree assignments MUST be recorded as repo-relative paths:
  - `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json` `PREPARE.worktree_dir` should be like `../wt-WP-...`
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

## High-Signal Governance References

- Final validator authority split: `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- Direct-review contract and session-repair rules: `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
- Legacy packet remediation policy: `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- Runtime placement and archival law: `.GOV/roles_shared/README.md`
- Parallel ownership/worktree model: `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
- Canonical command surface: `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
- Golden governed workflow examples: `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`

## Governance vs Product Checks

Governance-only (does not scan `src/` or `app/`):
- `just gov-check`
- Governance-only maintenance does not require a Work Packet or USER_SIGNATURE (Codex [CX-111]).
- Shared repo tooling notes live in `.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md`; use it as short append-only shared tooling memory, not as a second LAW surface.

Product-scanning / product-boundary enforcement:
- `just product-scan` / `just validator-scan` (forbidden patterns and product-boundary enforcement in product sources)
- There is no sanctioned `just codex-check` or `just validate` recipe in the live command surface. Run the explicit frontend/backend `TEST_PLAN` commands your WP requires instead.

## Session Host + Operator Monitor

- When available, prefer VS Code integrated terminals for multi-session work instead of many floating desktop terminals.
- Do not rely on ambient editor defaults for repo-governed session model choice or reasoning strength. New packets/stubs assume `gpt-5.4` primary, `gpt-5.2` fallback, and `model_reasoning_effort=xhigh`.
- Repo-governed role-session start is `ORCHESTRATOR_ONLY`.
- Primary launch path is the VS Code session bridge over the external repo-governance launch queue + session registry (default repo-relative: `../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl` + `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`).
- Primary steering lane is the governed Codex thread control path over the external repo-governance control ledgers (`../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl` + `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`).
- CLI escalation windows are allowed only after 2 plugin failures/timeouts for the same role/WP session.
- Recommended VS Code tabs:
  - `ORCH`
  - `CODER <WP_ID>`
  - `WPVAL <WP_ID>`
  - `INTVAL`
  - `MONITOR`
- `just operator-viewport` is the canonical Operator viewport for canonical board drift, governed ACP state, and broker/session lifecycle actions.
- `just operator-monitor` remains a compatibility alias.
- When the canonical `main` task board is available, the monitor uses that board for counts, filter buckets, and WP list selection. The current worktree board is still surfaced as the mirror/drift comparison source.
- Default `SESSIONS` view is governed-session-first: repo-governed ACP sessions are shown as first-class active sessions, with packet runtime sessions shown separately.
- `EVENTS` shows the merged governed ACP output stream for the selected WP across its governed role sessions.
- `TIMELINE` merges thread entries, receipts, governed control requests/results, and ACP events in timestamp order for the selected WP.
- `just session-start <ROLE> WP-...` starts a steerable governed thread for that role/WP session.
- `just session-send <ROLE> WP-... "<prompt>"` resumes that governed thread and records append-only request/result artifacts.
- `just session-cancel <ROLE> WP-...` requests cancellation of the currently running governed command for that role/WP session.
- `just wp-thread-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> "<message>" [target]` appends a freeform message to the packet-declared `WP_COMMUNICATION_DIR` and writes a paired `THREAD_MESSAGE` receipt.
- `just external-validator-brief WP-...` prints the canonical external/classical validation target contract, including startup order, split verdict fields, disposition, and the legal verdict vocabulary.
- `just backup-push feat/WP-{ID} feat/WP-{ID}` preserves the WP phase-boundary recovery branch; use it after bootstrap claim, skeleton checkpoint, skeleton approval, and before destructive/state-hiding local git actions.
- `just generate-worktree-cleanup-script WP-{ID} CODER|WP_VALIDATOR` emits a single-target post-merge cleanup script plus manifest. The script is hard-bound to one exact local worktree, requires both the baked Operator approval text and the matching worktree cleanup token, and refuses generation when the target worktree is dirty.

## Role: Orchestrator

Authoritative inputs:
- `.GOV/spec/SPEC_CURRENT.md` (current binding spec pointer)
- `.GOV/roles_shared/records/TASK_BOARD.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json` (mechanical gate state)
- Active WP packet + `WP_COMMUNICATIONS` artifacts when declared

Primary commands:
- `just record-refinement WP-...`
- `just record-signature WP-... <sig> <MANUAL_RELAY|ORCHESTRATOR_MANAGED> <Coder-A..Coder-Z>`
- `just worktree-add WP-...`
- `just record-prepare WP-... [<MANUAL_RELAY|ORCHESTRATOR_MANAGED>] [<Coder-A..Coder-Z>] [branch] [worktree_dir]`
- `just create-task-packet WP-...`
- for `PACKET_FORMAT_VERSION >= 2026-04-01`, inspect the packet law bundle immediately after creation: `DATA_CONTRACT_PROFILE`, `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`, and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`
- on that packet family, coder handoff must include anti-vibe + signed-scope-debt self-audit, and validator PASS requires both lists to be exactly `- NONE`
- if `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, keep `DATA_CONTRACT_MONITORING` credible from the start; validator closeout later requires concrete `DATA_CONTRACT_PROOF` plus `DATA_CONTRACT_GAPS`
- `just orchestrator-prepare-and-packet WP-... [<MANUAL_RELAY|ORCHESTRATOR_MANAGED>] [<Coder-A..Coder-Z>]`
- `just coder-worktree-add WP-...`
- `just wp-validator-worktree-add WP-...`
- `just integration-validator-worktree-add WP-...`
- `just launch-coder-session WP-... [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-wp-validator-session WP-... [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-integration-validator-session WP-... [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just manual-relay-next WP-...`
- `just manual-relay-dispatch WP-... [PRIMARY|FALLBACK]`
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
- `just active-lane-brief <ROLE> WP-... [--json]`
- `just orchestrator-steer-next WP-... [PRIMARY|FALLBACK]`
- `just manual-relay-next WP-...`
- `just manual-relay-dispatch WP-... [PRIMARY|FALLBACK]`
- `just pre-work WP-...`
- `just wp-heartbeat WP-... ORCHESTRATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir]`
- `just wp-receipt-append WP-... ORCHESTRATOR <session> <receipt_kind> "<summary>"`
- `just wp-thread-append WP-... ORCHESTRATOR <session> "<message>" [target]`
- `just operator-viewport`
- Heartbeat note: `wp-heartbeat` is liveness-only. `next_actor` / `waiting_on` must match current runtime route and cannot be used to steer the lane.
- `just session-registry-status WP-...` now also surfaces derived stalled-relay state for the filtered WP.
- If relay state is `ESCALATED`, use `just orchestrator-steer-next WP-...` instead of waiting silently.
- `just orchestrator-steer-next` now performs a one-hop wakeup: if the projected target session is not running yet, it starts that governed session and immediately injects the typed route payload in the same invocation.
- Inside the monitor:
  - `c` closes governed sessions for the selected WP after a role prompt + confirmation.
  - `b` stops the ACP broker after confirmation, but only if no governed runs are active.

Role rule:
- The Orchestrator is one non-agentic coordinator CLI session. It coordinates and launches repo-governed CLI sessions, but does not spawn Orchestrator or Validator helper agents.
- The Orchestrator is workflow authority. It does not become final technical authority, but it may execute `just sync-gov-to-main` and `origin/main` push when the Operator explicitly instructs it to do so; that mechanical execution does not replace validator technical authority.

## Role: Coder

Primary commands:
- `just pre-work WP-...`
- Implement only within `IN_SCOPE_PATHS`
- Hygiene: `just product-scan`, `just validator-dal-audit`, `just validator-git-hygiene`
- Workflow closure evidence: `just post-work WP-...`
- Session start/steering: `just start-coder-session WP-...`, `just steer-coder-session WP-... "<prompt>"`
- `just active-lane-brief CODER WP-... [--json]`
- `just wp-heartbeat WP-... CODER <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir]`
- `just wp-receipt-append WP-... CODER <session> <receipt_kind> "<summary>"`
- `just wp-thread-append WP-... CODER <session> "<message>" [target]`
- Heartbeat note: `wp-heartbeat` is liveness-only. Use receipts/notifications to change routing, not heartbeat.
- If context/routing feels fragmented, use `just active-lane-brief CODER WP-...` instead of rereading packet/runtime/session truth separately.
- `just active-lane-brief` now also names the declared active/next MT when microtask files exist, so coder work should resume at MT granularity instead of broad WP scope.
- Use `just check-notifications WP-... CODER <your-session>` so you only consume notifications targeted to your governed session.

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
- `just product-scan` (product boundary enforcement)
- Session start/steering: `just start-wp-validator-session WP-...`, `just steer-wp-validator-session WP-... "<prompt>"`
- `just active-lane-brief WP_VALIDATOR|INTEGRATION_VALIDATOR WP-... [--json]`
- `just wp-heartbeat WP-... WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <phase> <runtime_status> <next_actor> "<waiting_on>" [validator_trigger] [last_event] [worktree_dir] [next_expected_session] [waiting_on_session]`
- `just wp-receipt-append WP-... WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <receipt_kind> "<summary>" [state_before] [state_after] [target_role] [target_session] [correlation_id] [requires_ack] [ack_for]`
- `just wp-thread-append WP-... WP_VALIDATOR|INTEGRATION_VALIDATOR <session> "<message>" [target] [target_role] [target_session] [correlation_id] [requires_ack] [ack_for]`
- Heartbeat note: `wp-heartbeat` is liveness-only. Route changes must come from receipts/notifications or closeout projection.
- If context/routing feels fragmented, use `just active-lane-brief WP_VALIDATOR|INTEGRATION_VALIDATOR WP-...` instead of rereading packet/runtime/session truth separately.
- The same brief now surfaces declared active/next MT truth so validator overlap review can target the correct microtask without re-deriving it from raw receipts.
- Use `just check-notifications WP-... WP_VALIDATOR|INTEGRATION_VALIDATOR <your-session>` so you only consume notifications targeted to your governed session.
- `just wp-review-exchange VALIDATOR_QUERY WP-... CODER <session> WP_VALIDATOR <wp_validator_session> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref] [ack_for] [microtask_json]`
- `just wp-validator-response WP-... WP_VALIDATOR|INTEGRATION_VALIDATOR <session> <coder_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for] [microtask_json]`
- `just wp-review-exchange REVIEW_REQUEST WP-... <ACTOR_ROLE> <session> <TARGET_ROLE> <target_session> "<summary>" [correlation_id] [spec_anchor] [packet_row_ref] [ack_for] [microtask_json]`
- `just wp-review-response WP-... <ACTOR_ROLE> <session> <TARGET_ROLE> <target_session> "<summary>" <correlation_id> [spec_anchor] [packet_row_ref] [ack_for] [microtask_json]`
- optional final `microtask_json` may carry `{ "scope_ref": "...", "file_targets": ["..."], "proof_commands": ["..."], "risk_focus": "...", "expected_receipt_kind": "..." }`

Governance-only work:
- `just gov-check`

File-touch map:
- `.GOV/roles_shared/docs/VALIDATOR_FILE_TOUCH_MAP.md`

Role rule:
- Validator duties are non-agentic, but repo workflows may run multiple validator CLI sessions when they are explicitly scoped as WP Validator and Integration Validator sessions.
- Validator authority is layered: WP Validator is advisory; Integration Validator owns final technical and merge authority unless the packet explicitly overrides it.
- Validator sessions are started by the Orchestrator; validators do not self-start new repo-governed sessions.
- For orchestrator-managed WPs, PASS commit clearance now depends on committed handoff validation against the PREPARE worktree source of truth, recorded via `just validator-handoff-check WP-...`.
