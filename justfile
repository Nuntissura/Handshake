set dotenv-load := false
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

GOV_ROOT := env_var_or_default('HANDSHAKE_GOV_ROOT', '.GOV')
MAIN_ROOT := "../handshake_main"

docs-check:
	node -e "['{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md', '{{MAIN_ROOT}}/AGENTS.md', '{{GOV_ROOT}}/README.md', '{{GOV_ROOT}}/roles/README.md', '{{GOV_ROOT}}/roles_shared/README.md', '{{GOV_ROOT}}/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles/coder/CODER_PROTOCOL.md', '{{GOV_ROOT}}/roles/validator/VALIDATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles_shared/docs/START_HERE.md', '{{GOV_ROOT}}/spec/SPEC_CURRENT.md', '{{GOV_ROOT}}/roles_shared/docs/ARCHITECTURE.md', '{{GOV_ROOT}}/roles_shared/docs/RUNBOOK_DEBUG.md', '{{GOV_ROOT}}/roles_shared/docs/REPO_RESILIENCE.md', '{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md', '{{GOV_ROOT}}/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md', '{{GOV_ROOT}}/docs/vscode-session-bridge/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md'].forEach(f => { if (!require('fs').existsSync(f)) { console.error('Missing: ' + f); process.exit(1); } })"

gov-check:
	just docs-check
	$env:HANDSHAKE_ACTIVE_REPO_ROOT=(Resolve-Path "{{MAIN_ROOT}}").Path; $env:HANDSHAKE_GOV_ROOT=(Resolve-Path "{{GOV_ROOT}}").Path; node {{GOV_ROOT}}/roles_shared/checks/gov-check.mjs

backup-status:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/backup-status.mjs

backup-snapshot label="manual" out_root="" nas_root="":
	node {{GOV_ROOT}}/roles_shared/scripts/topology/backup-snapshot.mjs --label "{{label}}" --out-root "{{out_root}}" --nas-root "{{nas_root}}"

sync-gov-to-main:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/sync-gov-to-main.mjs --main-worktree {{MAIN_ROOT}}

enumerate-cleanup-targets:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/enumerate-cleanup-targets.mjs

delete-local-worktree worktree_id approval:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/delete-local-worktree.mjs {{worktree_id}} --approve "{{approval}}"

worktree-add wp-id base="main" branch="" dir="":
	node {{GOV_ROOT}}/roles_shared/scripts/topology/worktree-add.mjs {{wp-id}} {{base}} {{branch}} {{dir}}

build-order-sync:
	node {{GOV_ROOT}}/roles_shared/scripts/build-order-sync.mjs

validator-spec-regression:
	node {{GOV_ROOT}}/roles/validator/checks/validator-spec-regression.mjs

launch-coder-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs CODER {{wp-id}} {{host}} {{model}}

launch-wp-validator-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs WP_VALIDATOR {{wp-id}} {{host}} {{model}}

launch-integration-validator-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs INTEGRATION_VALIDATOR {{wp-id}} {{host}} {{model}}

coder-worktree-add wp-id branch="" dir="":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs CODER {{wp-id}} {{branch}} {{dir}}

wp-validator-worktree-add wp-id branch="" dir="":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs WP_VALIDATOR {{wp-id}} {{branch}} {{dir}}

integration-validator-worktree-add wp-id branch="" dir="":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs INTEGRATION_VALIDATOR {{wp-id}} {{branch}} {{dir}}

session-start role wp-id model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs START_SESSION {{role}} {{wp-id}} "" {{model}}

session-send role wp-id prompt model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs SEND_PROMPT {{role}} {{wp-id}} "{{prompt}}" {{model}}

session-cancel role wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-cancel.mjs {{role}} {{wp-id}}

session-close role wp-id:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs CLOSE_SESSION {{role}} {{wp-id}}

session-registry-status wp-id="":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-registry-status.mjs {{wp-id}}

operator-monitor *args:
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/operator-monitor-tui.mjs {{args}}

operator-admin *args:
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/operator-monitor-tui.mjs --admin {{args}}

protocol-ack codex agents shared protocol:
	@node {{GOV_ROOT}}/roles_shared/scripts/protocol-ack.mjs "{{codex}}" "{{agents}}" "{{shared}}" "{{protocol}}"

hard-gate-wt-001:
	@echo 'LIFECYCLE [CX-LIFE-001]'
	@echo '- WP_ID: <WP-... or N/A>'
	@echo '- STAGE: <REFINEMENT|SIGNATURE|PREPARE|PACKET_CREATE|DELEGATION|STATUS_SYNC|MERGE>'
	@echo '- NEXT: <next stage or STOP>'
	@echo ''
	@echo 'OPERATOR_ACTION: <NONE|one explicit decision needed>'
	@echo 'STATE: <1 sentence>'
	@echo ''
	@echo 'HARD_GATE_OUTPUT [CX-WT-001]'
	@git rev-parse --show-toplevel
	@git status -sb
	@git worktree list
	@echo ''
	@echo 'HARD_GATE_REASON [CX-WT-001]'
	@echo '- Verify repo/worktree/branch context before proceeding (prevents cross-WP contamination).'
	@echo ''
	@echo 'HARD_GATE_NEXT_ACTIONS [CX-WT-001]'
	@echo '- If this matches the assignment: continue.'
	@echo '- If incorrect/uncertain: STOP and ask Operator/Orchestrator for the correct worktree/branch.'
	@echo ''
	@echo 'PHASE_STATUS [CX-GATE-UX-001]'
	@echo '- PHASE: <REFINEMENT|SIGNATURE|PREPARE|PACKET_CREATE|DELEGATION|STATUS_SYNC>'
	@echo '- DECISION: <PROCEED|STOP>'
	@echo '- NEXT_COMMANDS:'
	@echo '  - <cmd1>'
	@echo '  - <cmd2>'

orchestrator-preflight:
	@just hard-gate-wt-001
	@just gov-check
	@just orchestrator-startup-truth-check
	@just validator-spec-regression

validator-preflight:
	@just hard-gate-wt-001
	@just gov-check
	@just validator-spec-regression

coder-preflight:
	@just hard-gate-wt-001
	@just gov-check
	@just validator-spec-regression

orchestrator-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md"
	@just backup-status
	@just orchestrator-preflight
	@echo 'RESUME_HINT: After a reset/compaction, run `just orchestrator-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

validator-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/validator/VALIDATOR_PROTOCOL.md"
	@just backup-status
	@just validator-preflight
	@echo 'RESUME_HINT: After a reset/compaction, run `just validator-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

coder-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/coder/CODER_PROTOCOL.md"
	@just backup-status
	@just coder-preflight
	@echo 'RUBRIC_REQUIRED: Read `{{GOV_ROOT}}/roles/coder/docs/CODER_RUBRIC_V2.md` before the first WP-specific BOOTSTRAP or code change, and answer it in `## STATUS_HANDOFF` before validator handoff.'
	@echo 'RESUME_HINT: After a reset/compaction, run `just coder-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

orchestrator-startup-truth-check:
	@node {{GOV_ROOT}}/roles/orchestrator/checks/orchestrator-startup-truth-check.mjs

orchestrator-next wp-id="":
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/orchestrator-next.mjs {{wp-id}}

coder-next wp-id="":
	@node {{GOV_ROOT}}/roles/coder/scripts/coder-next.mjs {{wp-id}}

validator-next wp-id="":
	@node {{GOV_ROOT}}/roles/validator/scripts/validator-next.mjs {{wp-id}}

record-refinement wp-id detail="":
	@node {{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs refine {{wp-id}} "{{detail}}"

record-signature wp-id signature workflow_lane="" execution_lane="":
	@node {{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs sign {{wp-id}} {{signature}} {{workflow_lane}} {{execution_lane}}

record-prepare wp-id workflow_lane="" execution_lane="" branch="" worktree_dir="":
	@node {{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs prepare {{wp-id}} {{workflow_lane}} {{execution_lane}} {{branch}} {{worktree_dir}}

create-task-packet wp-id:
	@echo "Creating task packet: {{wp-id}}..."
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/create-task-packet.mjs {{wp-id}}
	@just build-order-sync

orchestrator-prepare-and-packet wp-id workflow_lane="" execution_lane="" label="pre-wp-launch":
	@just worktree-add {{wp-id}}
	@just record-prepare {{wp-id}} {{workflow_lane}} {{execution_lane}}
	@just create-task-packet {{wp-id}}
	@echo "[ORCHESTRATOR] Committing governance checkpoint on gov_kernel..."
	@git add -A
	@git diff --cached --quiet || git commit -m "gov: checkpoint packet+refinement+micro-tasks [{{wp-id}}]"
	@echo "[ORCHESTRATOR] Creating backup snapshot before coder launch..."
	@just backup-snapshot "{{label}}"
