set dotenv-load := false
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

GOV_ROOT := env_var_or_default('HANDSHAKE_GOV_ROOT', '.GOV')
MAIN_ROOT := "../handshake_main"
CARGO_TARGET_DIR := "../Handshake Artifacts/handshake-cargo-target"

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

retire-standalone-checkout checkout_id approval:
	node {{GOV_ROOT}}/roles_shared/scripts/topology/retire-standalone-checkout.mjs {{checkout_id}} --approve "{{approval}}"

worktree-add wp-id base="main" branch="" dir="":
	node {{GOV_ROOT}}/roles_shared/scripts/topology/worktree-add.mjs {{wp-id}} {{base}} {{branch}} {{dir}}

build-order-sync:
	node {{GOV_ROOT}}/roles_shared/scripts/build-order-sync.mjs

validator-spec-regression:
	node {{GOV_ROOT}}/roles/validator/checks/validator-spec-regression.mjs

gate-check wp-id:
	node {{GOV_ROOT}}/roles_shared/checks/gate-check.mjs {{wp-id}}

spec-eof-appendices-check:
	node {{GOV_ROOT}}/roles_shared/checks/spec-eof-appendices-check.mjs

validator-packet-complete wp-id:
	node {{GOV_ROOT}}/roles/validator/checks/validator-packet-complete.mjs {{wp-id}}

wp-declared-topology-check wp-id:
	node {{GOV_ROOT}}/roles_shared/checks/wp-declared-topology-check.mjs {{wp-id}}

validator-policy-gate wp-id:
	node {{GOV_ROOT}}/roles_shared/checks/computed-policy-gate-check.mjs {{wp-id}}

post-run-audit-skeleton wp-id output="":
	node {{GOV_ROOT}}/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs {{wp-id}} {{if output != "" { "--output " + output } else { "" }}}

launch-coder-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs CODER {{wp-id}} {{host}} {{model}}

launch-wp-validator-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs WP_VALIDATOR {{wp-id}} {{host}} {{model}}

launch-integration-validator-session wp-id host="AUTO" model="PRIMARY":
	node {{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs INTEGRATION_VALIDATOR {{wp-id}} {{host}} {{model}}

start-coder-session wp-id model="PRIMARY":
	@just session-start CODER {{wp-id}} {{model}}

start-wp-validator-session wp-id model="PRIMARY":
	@just session-start WP_VALIDATOR {{wp-id}} {{model}}

start-integration-validator-session wp-id model="PRIMARY":
	@just session-start INTEGRATION_VALIDATOR {{wp-id}} {{model}}

steer-coder-session wp-id prompt model="PRIMARY":
	@just session-send CODER {{wp-id}} "{{prompt}}" {{model}}

steer-wp-validator-session wp-id prompt model="PRIMARY":
	@just session-send WP_VALIDATOR {{wp-id}} "{{prompt}}" {{model}}

steer-integration-validator-session wp-id prompt model="PRIMARY":
	@just session-send INTEGRATION_VALIDATOR {{wp-id}} "{{prompt}}" {{model}}

cancel-coder-session wp-id:
	@just session-cancel CODER {{wp-id}}

cancel-wp-validator-session wp-id:
	@just session-cancel WP_VALIDATOR {{wp-id}}

cancel-integration-validator-session wp-id:
	@just session-cancel INTEGRATION_VALIDATOR {{wp-id}}

close-coder-session wp-id:
	@just session-close CODER {{wp-id}}

close-wp-validator-session wp-id:
	@just session-close WP_VALIDATOR {{wp-id}}

close-integration-validator-session wp-id:
	@just session-close INTEGRATION_VALIDATOR {{wp-id}}

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

active-lane-brief role wp-id json="":
	@node {{GOV_ROOT}}/roles_shared/checks/active-lane-brief.mjs {{role}} {{wp-id}} {{json}}

wp-token-usage wp-id:
	node {{GOV_ROOT}}/roles_shared/scripts/session/wp-token-usage-report.mjs {{wp-id}}

wp-token-usage-settle wp-id reason="HISTORICAL_BACKFILL" settled-by="SYSTEM":
	node {{GOV_ROOT}}/roles_shared/scripts/session/wp-token-usage-settle.mjs {{wp-id}} {{reason}} {{settled-by}}

session-control-runtime-check:
	node {{GOV_ROOT}}/roles_shared/checks/session-control-runtime-check.mjs

handshake-acp-broker-status:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-broker.mjs status

handshake-acp-broker-stop:
	node {{GOV_ROOT}}/roles/orchestrator/scripts/session-control-broker.mjs stop

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

orchestrator-steer-next wp-id model="PRIMARY":
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/orchestrator-steer-next.mjs {{wp-id}} {{model}}

coder-next wp-id="":
	@node {{GOV_ROOT}}/roles/coder/scripts/coder-next.mjs {{wp-id}}

pre-work wp-id *args:
	@node {{GOV_ROOT}}/roles/coder/checks/pre-work.mjs {{wp-id}} {{args}}

post-work wp-id *args:
	@node {{GOV_ROOT}}/roles/coder/checks/post-work.mjs {{wp-id}} {{args}}

coder-skeleton-checkpoint wp-id:
	@node {{GOV_ROOT}}/roles/coder/checks/coder-skeleton-checkpoint.mjs {{wp-id}}

skeleton-approved wp-id:
	@node {{GOV_ROOT}}/roles_shared/checks/skeleton-approved.mjs {{wp-id}}

backup-push local_branch="" remote_branch="":
	@node {{GOV_ROOT}}/roles_shared/scripts/topology/backup-push.mjs {{local_branch}} {{remote_branch}}

validator-scan:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-scan.mjs

product-scan:
	@just validator-scan

validator-dal-audit:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-dal-audit.mjs

validator-git-hygiene:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-git-hygiene.mjs

cargo-clean:
	cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "{{CARGO_TARGET_DIR}}"

spec-debt-open wp-id clause notes blocking="NO":
	@node {{GOV_ROOT}}/roles_shared/scripts/debt/spec-debt-open.mjs {{wp-id}} "{{clause}}" "{{notes}}" {{blocking}}

spec-debt-sync wp-id:
	@node {{GOV_ROOT}}/roles_shared/scripts/debt/spec-debt-sync.mjs {{wp-id}}

validator-next wp-id="":
	@node {{GOV_ROOT}}/roles/validator/scripts/validator-next.mjs {{wp-id}}

task-board-set wp-id status reason="":
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/task-board-set.mjs {{wp-id}} {{status}} "{{reason}}"

validator-handoff-check wp-id *args:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-handoff-check.mjs {{wp-id}} {{args}}

integration-validator-closeout-check wp-id:
	@node {{GOV_ROOT}}/roles/validator/checks/integration-validator-closeout-check.mjs {{wp-id}}

integration-validator-closeout-sync wp-id mode merged_main_sha="":
	@node {{GOV_ROOT}}/roles/validator/scripts/integration-validator-closeout-sync.mjs {{wp-id}} {{mode}} {{merged_main_sha}}

integration-validator-context-brief wp-id *args:
	@node {{GOV_ROOT}}/roles/validator/checks/integration-validator-context-brief.mjs {{wp-id}} {{args}}

external-validator-brief wp-id *args:
	@node {{GOV_ROOT}}/roles/validator/checks/external-validator-brief.mjs {{wp-id}} {{args}}

validator-gate-append wp-id verdict:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs append {{wp-id}} {{verdict}}

validator-gate-commit wp-id:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs commit {{wp-id}}

validator-gate-present wp-id verdict="":
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs present-report {{wp-id}} {{verdict}}

validator-gate-acknowledge wp-id:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs acknowledge {{wp-id}}

validator-gate-status wp-id:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs status {{wp-id}}

validator-gate-reset wp-id confirm:
	@node {{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs reset {{wp-id}} {{confirm}}

validator-governance-snapshot:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-governance-snapshot.mjs

validator-report-structure-check:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-report-structure-check.mjs

validator-phase-gate phase="Phase-1":
	@node {{GOV_ROOT}}/roles/validator/checks/validator-phase-gate.mjs {{phase}}

validator-error-codes:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-error-codes.mjs

validator-coverage-gaps:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-coverage-gaps.mjs

validator-traceability:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-traceability.mjs

validator-hygiene-full:
	@node {{GOV_ROOT}}/roles/validator/checks/validator-hygiene-full.mjs

sync-all-role-worktrees:
	@node {{GOV_ROOT}}/roles_shared/scripts/topology/sync-all-role-worktrees.mjs

reseed-permanent-worktree-from-main worktree_id approval label="":
	@node {{GOV_ROOT}}/roles_shared/scripts/topology/reseed-permanent-worktree-from-main.mjs {{worktree_id}} --approve "{{approval}}" {{if label != "" { "--label \"" + label + "\"" } else { "" }}}

generate-worktree-cleanup-script wp-id role:
	@node {{GOV_ROOT}}/roles_shared/scripts/topology/generate-worktree-cleanup-script.mjs {{wp-id}} {{role}}

close-wp-branch wp-id approval remote="":
	@node {{GOV_ROOT}}/roles_shared/scripts/topology/close-wp-branch.mjs {{wp-id}} {{remote}} --approve "{{approval}}"

wp-heartbeat wp-id actor_role actor_session current_phase runtime_status next_expected_actor waiting_on validator_trigger="" last_event="" worktree_dir="" next_expected_session="" waiting_on_session="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-heartbeat.mjs {{wp-id}} {{actor_role}} {{actor_session}} {{current_phase}} {{runtime_status}} {{next_expected_actor}} "{{waiting_on}}" "{{validator_trigger}}" "{{last_event}}" "{{worktree_dir}}" "{{next_expected_session}}" "{{waiting_on_session}}"

wp-validator-response wp-id actor_role actor_session coder_session summary correlation_id spec_anchor="" packet_row_ref="" ack_for="" microtask_json="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs VALIDATOR_RESPONSE {{wp-id}} {{actor_role}} {{actor_session}} CODER {{coder_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "{{ack_for}}" '{{microtask_json}}'

wp-review-response wp-id actor_role actor_session target_role target_session summary correlation_id spec_anchor="" packet_row_ref="" ack_for="" microtask_json="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs REVIEW_RESPONSE {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "{{ack_for}}" '{{microtask_json}}'

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

wp-thread-append wp-id actor_role actor_session message target="" target_role="" target_session="" correlation_id="" requires_ack="" ack_for="" spec_anchor="" packet_row_ref="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-thread-append.mjs {{wp-id}} {{actor_role}} {{actor_session}} "{{message}}" "{{target}}" "{{target_role}}" "{{target_session}}" "{{correlation_id}}" "{{requires_ack}}" "{{ack_for}}" "{{spec_anchor}}" "{{packet_row_ref}}"

wp-receipt-append wp-id actor_role actor_session receipt_kind summary state_before="" state_after="" target_role="" target_session="" correlation_id="" requires_ack="" ack_for="" spec_anchor="" packet_row_ref="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-receipt-append.mjs {{wp-id}} {{actor_role}} {{actor_session}} {{receipt_kind}} "{{summary}}" "{{state_before}}" "{{state_after}}" "{{target_role}}" "{{target_session}}" "{{correlation_id}}" "{{requires_ack}}" "{{ack_for}}" "{{spec_anchor}}" "{{packet_row_ref}}"

wp-invalidity-flag wp-id actor_role actor_session invalidity_code summary spec_anchor="" packet_row_ref="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-invalidity-flag.mjs {{wp-id}} {{actor_role}} {{actor_session}} {{invalidity_code}} "{{summary}}" "{{spec_anchor}}" "{{packet_row_ref}}"

wp-operator-rule-restatement wp-id actor_role actor_session summary spec_anchor="" packet_row_ref="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-operator-rule-restatement.mjs {{wp-id}} {{actor_role}} {{actor_session}} "{{summary}}" "{{spec_anchor}}" "{{packet_row_ref}}"

wp-review-exchange receipt_kind wp-id actor_role actor_session target_role target_session summary correlation_id="" spec_anchor="" packet_row_ref="" ack_for="" microtask_json="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs {{receipt_kind}} {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "{{ack_for}}" '{{microtask_json}}'

wp-spec-gap wp-id actor_role actor_session target_role target_session summary correlation_id="" spec_anchor="" packet_row_ref="" microtask_json="":
	@just wp-review-exchange SPEC_GAP {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-spec-confirmation wp-id actor_role actor_session target_role target_session summary correlation_id spec_anchor="" packet_row_ref="" ack_for="" microtask_json="":
	@just wp-review-exchange SPEC_CONFIRMATION {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "{{ack_for}}" '{{microtask_json}}'

wp-validator-kickoff wp-id actor_session coder_session summary spec_anchor="" packet_row_ref="" microtask_json="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs VALIDATOR_KICKOFF {{wp-id}} WP_VALIDATOR {{actor_session}} CODER {{coder_session}} "{{summary}}" "" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-coder-intent wp-id actor_session validator_session summary correlation_id spec_anchor="" packet_row_ref="" microtask_json="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs CODER_INTENT {{wp-id}} CODER {{actor_session}} WP_VALIDATOR {{validator_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-coder-handoff wp-id actor_session validator_session summary correlation_id="" spec_anchor="" packet_row_ref="" microtask_json="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs CODER_HANDOFF {{wp-id}} CODER {{actor_session}} WP_VALIDATOR {{validator_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-validator-review wp-id actor_session coder_session summary correlation_id spec_anchor="" packet_row_ref="" microtask_json="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs VALIDATOR_REVIEW {{wp-id}} WP_VALIDATOR {{actor_session}} CODER {{coder_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-communication-health-check wp-id stage="STATUS":
	@node {{GOV_ROOT}}/roles_shared/checks/wp-communication-health-check.mjs {{wp-id}} {{stage}}

check-notifications wp-id role="":
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-check-notifications.mjs {{wp-id}} {{role}}

ack-notifications wp-id role session:
	@node {{GOV_ROOT}}/roles_shared/scripts/wp/wp-check-notifications.mjs {{wp-id}} {{role}} --ack --session={{session}}

orchestrator-prepare-and-packet wp-id workflow_lane="" execution_lane="" label="pre-wp-launch":
	@just worktree-add {{wp-id}}
	@node {{GOV_ROOT}}/roles/orchestrator/scripts/orchestrator-prepare-and-packet.mjs {{wp-id}} {{workflow_lane}} {{execution_lane}}
	@echo "[ORCHESTRATOR] Committing governance checkpoint on gov_kernel..."
	@git add -A
	@git diff --cached --quiet; if ($LASTEXITCODE -ne 0) { git commit -m "gov: checkpoint packet+refinement+micro-tasks [{{wp-id}}]" }
	@echo "[ORCHESTRATOR] Creating backup snapshot before coder launch..."
	@just backup-snapshot "{{label}}"
