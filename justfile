set dotenv-load := false
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

GOV_ROOT := env_var_or_default('HANDSHAKE_GOV_ROOT', '.GOV')
MAIN_ROOT := "../handshake_main"
ARTIFACT_ROOT := env_var_or_default('HANDSHAKE_ARTIFACT_ROOT', '../Handshake_Artifacts')
CARGO_TARGET_DIR := "{{ARTIFACT_ROOT}}/handshake-cargo-target"

docs-check:
	node -e "['{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md', '{{MAIN_ROOT}}/AGENTS.md', '{{GOV_ROOT}}/README.md', '{{GOV_ROOT}}/roles/README.md', '{{GOV_ROOT}}/roles_shared/README.md', '{{GOV_ROOT}}/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md', '{{GOV_ROOT}}/roles/coder/CODER_PROTOCOL.md', '{{GOV_ROOT}}/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles/validator/VALIDATOR_PROTOCOL.md', '{{GOV_ROOT}}/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md', '{{GOV_ROOT}}/roles_shared/docs/START_HERE.md', '{{GOV_ROOT}}/spec/SPEC_CURRENT.md', '{{GOV_ROOT}}/roles_shared/docs/ARCHITECTURE.md', '{{GOV_ROOT}}/roles_shared/docs/RUNBOOK_DEBUG.md', '{{GOV_ROOT}}/roles_shared/docs/REPO_RESILIENCE.md', '{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md', '{{GOV_ROOT}}/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md'].forEach(f => { if (!require('fs').existsSync(f)) { console.error('Missing: ' + f); process.exit(1); } })"

gov-check:
	just docs-check
	$env:HANDSHAKE_ACTIVE_REPO_ROOT=(Resolve-Path "{{MAIN_ROOT}}").Path; $env:HANDSHAKE_GOV_ROOT=(Resolve-Path "{{GOV_ROOT}}").Path; node "{{GOV_ROOT}}/roles_shared/checks/gov-check.mjs"

canonise-gov:
	@node "{{GOV_ROOT}}/roles_shared/scripts/checks/canonise-gov.mjs"

backup-status:
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/backup-status.mjs"

artifact-hygiene-check:
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/artifact-hygiene-check.mjs"

gov-flush:
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/gov-flush.mjs"

artifact-cleanup dry-run="":
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/artifact-cleanup.mjs" {{dry-run}}

backup-snapshot label="manual" out_root="" nas_root="":
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/backup-snapshot.mjs" --label "{{label}}" --out-root "{{out_root}}" --nas-root "{{nas_root}}"

sync-gov-to-main:
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/sync-gov-to-main.mjs" --main-worktree {{MAIN_ROOT}}

enumerate-cleanup-targets:
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/enumerate-cleanup-targets.mjs"

delete-local-worktree worktree_id approval:
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/delete-local-worktree.mjs" {{worktree_id}} --approve "{{approval}}"

retire-standalone-checkout checkout_id approval:
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/retire-standalone-checkout.mjs" {{checkout_id}} --approve "{{approval}}"

worktree-add wp-id base="main" branch="" dir="":
	node "{{GOV_ROOT}}/roles_shared/scripts/topology/worktree-add.mjs" {{wp-id}} {{base}} {{branch}} {{dir}}

build-order-sync:
	node "{{GOV_ROOT}}/roles_shared/scripts/build-order-sync.mjs"

ensure-wp-communications wp-id:
	node "{{GOV_ROOT}}/roles_shared/scripts/wp/ensure-wp-communications.mjs" {{wp-id}}

validator-spec-regression:
	node "{{GOV_ROOT}}/roles/validator/checks/validator-spec-regression.mjs"

cor701-sha file:
	node "{{GOV_ROOT}}/roles_shared/checks/cor701-sha.mjs" {{file}}

spec-eof-appendices-check:
	node "{{GOV_ROOT}}/roles_shared/checks/spec-eof-appendices-check.mjs"

wp-declared-topology-check wp-id:
	node "{{GOV_ROOT}}/roles_shared/checks/wp-declared-topology-check.mjs" {{wp-id}}

validator-policy-gate wp-id:
	node "{{GOV_ROOT}}/roles_shared/checks/computed-policy-gate-check.mjs" {{wp-id}}

post-run-audit-skeleton wp-id output="":
	node "{{GOV_ROOT}}/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs" {{wp-id}} {{if output != "" { "--output " + output } else { "" }}}

live-smoketest-review-init wp-id output="":
	@just workflow-dossier-init {{wp-id}} {{output}}

workflow-dossier-init wp-id output="" *FLAGS:
	node "{{GOV_ROOT}}/roles_shared/scripts/audit/workflow-dossier.mjs" init {{wp-id}} {{if output != "" { "--output " + output } else { "--auto-output" }}} {{FLAGS}}

workflow-dossier-note wp-id section summary *FLAGS:
	node "{{GOV_ROOT}}/roles_shared/scripts/audit/workflow-dossier.mjs" note {{wp-id}} {{section}} "{{summary}}" {{FLAGS}}

workflow-dossier-sync wp-id *FLAGS:
	node "{{GOV_ROOT}}/roles_shared/scripts/audit/workflow-dossier.mjs" sync {{wp-id}} {{FLAGS}}

workflow-dossier-inject-repomem wp-id *FLAGS:
	node "{{GOV_ROOT}}/roles_shared/scripts/audit/workflow-dossier.mjs" inject-repomem {{wp-id}} {{FLAGS}}

workflow-dossier-autofill-costs wp-id *FLAGS:
	node "{{GOV_ROOT}}/roles_shared/scripts/audit/workflow-dossier.mjs" autofill-costs {{wp-id}} {{FLAGS}}

launch-coder-session wp-id host="AUTO" model="PRIMARY" *FLAGS:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs" CODER {{wp-id}} {{host}} {{model}} {{FLAGS}}

launch-activation-manager-session wp-id host="AUTO" model="PRIMARY" *FLAGS:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs" ACTIVATION_MANAGER {{wp-id}} {{host}} {{model}} {{FLAGS}}

launch-wp-validator-session wp-id host="AUTO" model="PRIMARY" *FLAGS:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs" WP_VALIDATOR {{wp-id}} {{host}} {{model}} {{FLAGS}}

launch-integration-validator-session wp-id host="AUTO" model="PRIMARY" *FLAGS:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs" INTEGRATION_VALIDATOR {{wp-id}} {{host}} {{model}} {{FLAGS}}

start-coder-session wp-id model="PRIMARY" *FLAGS:
	@just session-start CODER {{wp-id}} {{model}} {{FLAGS}}

start-activation-manager-session wp-id model="PRIMARY" *FLAGS:
	@just session-start ACTIVATION_MANAGER {{wp-id}} {{model}} {{FLAGS}}

start-wp-validator-session wp-id model="PRIMARY" *FLAGS:
	@just session-start WP_VALIDATOR {{wp-id}} {{model}} {{FLAGS}}

start-integration-validator-session wp-id model="PRIMARY" *FLAGS:
	@just session-start INTEGRATION_VALIDATOR {{wp-id}} {{model}} {{FLAGS}}

steer-coder-session wp-id prompt model="PRIMARY" *FLAGS:
	@just session-send CODER {{wp-id}} "{{prompt}}" {{model}} {{FLAGS}}

steer-activation-manager-session wp-id prompt model="PRIMARY" *FLAGS:
	@just session-send ACTIVATION_MANAGER {{wp-id}} "{{prompt}}" {{model}} {{FLAGS}}

steer-wp-validator-session wp-id prompt model="PRIMARY" *FLAGS:
	@just session-send WP_VALIDATOR {{wp-id}} "{{prompt}}" {{model}} {{FLAGS}}

steer-integration-validator-session wp-id prompt model="PRIMARY" *FLAGS:
	@just session-send INTEGRATION_VALIDATOR {{wp-id}} "{{prompt}}" {{model}} {{FLAGS}}

cancel-coder-session wp-id *FLAGS:
	@just session-cancel CODER {{wp-id}} {{FLAGS}}

cancel-activation-manager-session wp-id *FLAGS:
	@just session-cancel ACTIVATION_MANAGER {{wp-id}} {{FLAGS}}

cancel-wp-validator-session wp-id *FLAGS:
	@just session-cancel WP_VALIDATOR {{wp-id}} {{FLAGS}}

cancel-integration-validator-session wp-id *FLAGS:
	@just session-cancel INTEGRATION_VALIDATOR {{wp-id}} {{FLAGS}}

close-coder-session wp-id *FLAGS:
	@just session-close CODER {{wp-id}} {{FLAGS}}

close-activation-manager-session wp-id *FLAGS:
	@just session-close ACTIVATION_MANAGER {{wp-id}} {{FLAGS}}

close-wp-validator-session wp-id *FLAGS:
	@just session-close WP_VALIDATOR {{wp-id}} {{FLAGS}}


close-integration-validator-session wp-id *FLAGS:
	@just session-close INTEGRATION_VALIDATOR {{wp-id}} {{FLAGS}}

coder-worktree-add wp-id branch="" dir="":
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs" CODER {{wp-id}} {{branch}} {{dir}}

wp-validator-worktree-add wp-id branch="" dir="":
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs" WP_VALIDATOR {{wp-id}} {{branch}} {{dir}}

integration-validator-worktree-add wp-id branch="" dir="":
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/role-session-worktree-add.mjs" INTEGRATION_VALIDATOR {{wp-id}} {{branch}} {{dir}}

session-start role wp-id model="PRIMARY" *FLAGS:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs" START_SESSION {{role}} {{wp-id}} "" {{model}} {{FLAGS}}

session-send role wp-id prompt model="PRIMARY" *FLAGS:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs" SEND_PROMPT {{role}} {{wp-id}} "{{prompt}}" {{model}} {{FLAGS}}

session-cancel role wp-id *FLAGS:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/session-control-cancel.mjs" {{role}} {{wp-id}} {{FLAGS}}

session-close role wp-id *FLAGS:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/session-control-command.mjs" CLOSE_SESSION {{role}} {{wp-id}} {{FLAGS}}

session-reclaim-terminals wp-id role="" batch="CURRENT_BATCH":
	node "{{GOV_ROOT}}/roles_shared/scripts/session/reclaim-owned-terminals.mjs" {{wp-id}} {{role}} {{batch}}

session-scan-orphan-terminals *args="":
	node "{{GOV_ROOT}}/roles_shared/scripts/session/scan-orphan-terminals.mjs" {{args}}

session-registry-status wp-id="":
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/session-registry-status.mjs" {{wp-id}}

active-lane-brief role wp-id json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/session/active-lane-brief-lib.mjs" {{role}} {{wp-id}} {{json}}

wp-token-usage wp-id:
	node "{{GOV_ROOT}}/roles_shared/scripts/session/wp-token-usage-report.mjs" {{wp-id}}

wp-timeline wp-id json="":
	node "{{GOV_ROOT}}/roles_shared/scripts/session/wp-timeline-report.mjs" {{wp-id}} {{json}}

wp-metrics wp-id *flags="":
	node "{{GOV_ROOT}}/roles_shared/scripts/session/wp-timeline-report.mjs" {{wp-id}} --metrics {{flags}}

wp-metrics-compare wp-a wp-b *flags="":
	node "{{GOV_ROOT}}/roles_shared/scripts/session/wp-timeline-report.mjs" --compare {{wp-a}} {{wp-b}} {{flags}}

wp-token-usage-settle wp-id reason="HISTORICAL_BACKFILL" settled-by="SYSTEM":
	node "{{GOV_ROOT}}/roles_shared/scripts/session/wp-token-usage-settle.mjs" {{wp-id}} {{reason}} {{settled-by}}

session-control-runtime-check:
	node "{{GOV_ROOT}}/roles_shared/checks/session-control-runtime-check.mjs"

handshake-acp-broker-status:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/session-control-broker.mjs" status

handshake-acp-broker-stop:
	node "{{GOV_ROOT}}/roles/orchestrator/scripts/session-control-broker.mjs" stop

operator-viewport *args:
	@node "{{GOV_ROOT}}/operator/scripts/operator-viewport-tui.mjs" {{args}}

operator-viewport-admin *args:
	@node "{{GOV_ROOT}}/operator/scripts/operator-viewport-tui.mjs" --admin {{args}}

operator-monitor *args:
	@node "{{GOV_ROOT}}/operator/scripts/operator-viewport-tui.mjs" {{args}}

operator-admin *args:
	@node "{{GOV_ROOT}}/operator/scripts/operator-viewport-tui.mjs" --admin {{args}}

protocol-ack codex agents shared protocol:
	@node "{{GOV_ROOT}}/roles_shared/scripts/protocol-ack.mjs" "{{codex}}" "{{agents}}" "{{shared}}" "{{protocol}}"

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
	@node "{{GOV_ROOT}}/roles_shared/scripts/session/scan-orphan-terminals.mjs"

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
	@just role-startup-topology-check --audit-permanent
	@just orchestrator-preflight
	@just memory-refresh
	@just memory-recall RESUME
	@just launch-memory-manager
	@echo ''
	@echo 'CHECKPOINT_REQUIRED: SESSION_OPEN'
	@echo 'Run: just repomem open "<what this session is about>" --role ORCHESTRATOR [--wp WP-ID]'
	@echo 'This is MANDATORY before any orchestrator-next, steer, relay, or packet commands.'
	@echo ''
	@echo 'RESUME_HINT: After a reset/compaction, run `just orchestrator-next [WP-{ID}] [--debug]` and continue automatically when OPERATOR_ACTION: NONE.'
	@echo 'WORKFLOW_DOSSIER: after `just orchestrator-prepare-and-packet WP-{ID}`, use role `just repomem ... --wp WP-{ID}` for decisions, failures, concerns, and discoveries; `phase-check CLOSEOUT` mechanically imports those memories into the dossier. Use `workflow-dossier-sync` only for mechanical telemetry snapshots.'
	@echo 'REPO_TIMEZONE: Europe/Brussels for human-facing governance timestamps; ACP/session ledgers remain UTC.'

classic-orchestrator-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md"
	@just backup-status
	@just role-startup-topology-check --audit-permanent
	@just orchestrator-preflight
	@just memory-refresh
	@just memory-recall RESUME --role CLASSIC_ORCHESTRATOR
	@just launch-memory-manager
	@echo ''
	@echo 'CHECKPOINT_REQUIRED: SESSION_OPEN'
	@echo 'Run: just repomem open "<what this session is about>" --role CLASSIC_ORCHESTRATOR [--wp WP-ID]'
	@echo 'This is MANDATORY before any refinement, packet, or manual-relay commands on MANUAL_RELAY.'
	@echo ''
	@echo 'RESUME_HINT: After a reset/compaction, use `just manual-relay-next WP-{ID} [--debug]` for active manual-lane relay truth, or the relevant pre-launch command for active refinement/packet work.'
	@echo 'LANE_OWNER: CLASSIC_ORCHESTRATOR owns MANUAL_RELAY end-to-end, including the old combined Orchestrator + Activation Manager pre-launch flow.'
	@echo 'WORKFLOW_DOSSIER: use role `just repomem ... --wp WP-{ID}` during the run; closeout imports WP-bound memories mechanically instead of requiring live dossier narration.'
	@echo 'REPO_TIMEZONE: Europe/Brussels for human-facing governance timestamps; ACP/session ledgers remain UTC.'

validator-startup role:
	@switch ("{{role}}".Trim().ToUpper()) { "WP_VALIDATOR" { just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md"; break } "INTEGRATION_VALIDATOR" { just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md"; break } "VALIDATOR" { just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/validator/VALIDATOR_PROTOCOL.md"; break } default { Write-Error 'Usage: just validator-startup WP_VALIDATOR|INTEGRATION_VALIDATOR|VALIDATOR'; exit 1 } }
	@just backup-status
	@just role-startup-topology-check
	@just validator-preflight
	@just memory-refresh
	@just memory-recall VALIDATOR_RESUME --role {{role}}
	@echo ''
	@echo 'CHECKPOINT_REQUIRED: SESSION_OPEN'
	@Write-Host 'Run: just repomem open "<what this session is about>" --role {{role}} --wp WP-ID'
	@echo 'WP-bound validator lanes reject repomem open unless both --role and --wp are supplied.'
	@echo 'DURABLE_RUN_NOTES: capture verdict reasoning, failures, risks, and discoveries with `just repomem decision|error|concern|insight ... --wp WP-ID`; closeout imports them into the dossier.'
	@echo ''
	@echo 'RESUME_HINT: After a reset/compaction, run `just validator-next {{role}} [WP-{ID}] [--debug]` and continue automatically when OPERATOR_ACTION: NONE.'

coder-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/coder/CODER_PROTOCOL.md"
	@just backup-status
	@just role-startup-topology-check
	@just coder-preflight
	@just memory-refresh
	@just memory-recall CODER_RESUME
	@echo 'RUBRIC_REQUIRED: Read `{{GOV_ROOT}}/roles/coder/docs/CODER_RUBRIC_V2.md` before the first WP-specific BOOTSTRAP or code change, and answer it in `## STATUS_HANDOFF` before validator handoff.'
	@echo ''
	@echo 'CHECKPOINT_REQUIRED: SESSION_OPEN'
	@echo 'Run: just repomem open "<what this session is about>" --role CODER --wp WP-ID'
	@echo 'DURABLE_RUN_NOTES: capture implementation choices, failures, risks, and discoveries with `just repomem decision|error|concern|insight ... --wp WP-ID`; closeout imports them into the dossier.'
	@echo ''
	@echo 'RESUME_HINT: After a reset/compaction, run `just coder-next [WP-{ID}]` and continue automatically when OPERATOR_ACTION: NONE.'

orchestrator-startup-truth-check:
	@node "{{GOV_ROOT}}/roles/orchestrator/checks/orchestrator-startup-truth-check.mjs"

orchestrator-next wp-id="" *FLAGS:
	@just repomem-gate
	@just memory-recall RESUME --wp {{wp-id}}
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/orchestrator-next.mjs" {{wp-id}} {{FLAGS}}

orchestrator-steer-next wp-id context model="PRIMARY" *FLAGS:
	@just repomem-gate
	@just memory-recall STEERING --wp {{wp-id}}
	@just repomem context "{{context}}" --trigger orchestrator-steer-next --wp {{wp-id}}
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/orchestrator-steer-next.mjs" {{wp-id}} {{model}} {{FLAGS}}

manual-relay-next wp-id *FLAGS:
	@just repomem-gate
	@just memory-recall RELAY --wp {{wp-id}}
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/manual-relay-next.mjs" {{wp-id}} {{FLAGS}}

manual-relay-dispatch wp-id context model="PRIMARY" *FLAGS:
	@just repomem-gate
	@just memory-recall RELAY --wp {{wp-id}}
	@just repomem context "{{context}}" --trigger manual-relay-dispatch --wp {{wp-id}}
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/manual-relay-dispatch.mjs" {{wp-id}} {{model}} {{FLAGS}}

coder-next wp-id="":
	@just memory-recall CODER_RESUME --wp {{wp-id}}
	@node "{{GOV_ROOT}}/roles/coder/scripts/coder-next.mjs" {{wp-id}}

coder-skeleton-checkpoint wp-id:
	@node "{{GOV_ROOT}}/roles/coder/checks/coder-skeleton-checkpoint.mjs" {{wp-id}}

skeleton-approved wp-id:
	@node "{{GOV_ROOT}}/roles_shared/checks/skeleton-approved.mjs" {{wp-id}}

backup-push local_branch="" remote_branch="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/topology/backup-push.mjs" {{local_branch}} {{remote_branch}}

validator-scan:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-scan.mjs"

product-scan:
	@just validator-scan

validator-dal-audit:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-dal-audit.mjs"

validator-git-hygiene:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-git-hygiene.mjs"

cargo-clean:
	cargo clean -p handshake_core --manifest-path src/backend/handshake_core/Cargo.toml --target-dir "{{CARGO_TARGET_DIR}}"

spec-debt-open wp-id clause notes blocking="NO":
	@node "{{GOV_ROOT}}/roles_shared/scripts/debt/spec-debt-open.mjs" {{wp-id}} "{{clause}}" "{{notes}}" {{blocking}}

spec-debt-sync wp-id:
	@node "{{GOV_ROOT}}/roles_shared/scripts/debt/spec-debt-sync.mjs" {{wp-id}}

validator-next role wp-id="" *FLAGS:
	@if ("{{role}}".Trim().ToUpper() -notin @("WP_VALIDATOR", "INTEGRATION_VALIDATOR", "VALIDATOR")) { Write-Error 'Usage: just validator-next WP_VALIDATOR|INTEGRATION_VALIDATOR|VALIDATOR [WP-ID] [--debug]'; exit 1 }
	@just memory-recall VALIDATOR_RESUME --role {{role}} --wp {{wp-id}}
	@$env:HANDSHAKE_VALIDATOR_ROLE="{{role}}"; node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles/validator/scripts/validator-next.mjs" --role {{role}} {{wp-id}} --raw-flags "{{FLAGS}}"

task-board-set wp-id status context reason="":
	@just repomem-gate
	@just repomem context "{{context}}" --trigger task-board-set --wp {{wp-id}}
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/task-board-set.mjs" {{wp-id}} {{status}} "{{reason}}"

integration-validator-context-brief wp-id *args:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs" {{wp-id}} --raw-flags "{{args}}"

external-validator-brief wp-id *args:
	@node "{{GOV_ROOT}}/roles/validator/checks/external-validator-brief.mjs" {{wp-id}} {{args}}

validator-gate-append wp-id verdict:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs" append {{wp-id}} {{verdict}}

validator-gate-commit wp-id:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs" commit {{wp-id}}

validator-gate-present wp-id verdict="":
	@node "{{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs" present-report {{wp-id}} {{verdict}}

validator-gate-acknowledge wp-id:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs" acknowledge {{wp-id}}

validator-gate-status wp-id:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs" status {{wp-id}}

validator-gate-reset wp-id confirm:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator_gates.mjs" reset {{wp-id}} {{confirm}}

validator-governance-snapshot:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-governance-snapshot.mjs"

validator-report-structure-check:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-report-structure-check.mjs"

validator-phase-gate phase="Phase-1":
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-phase-gate.mjs" {{phase}}

validator-error-codes:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-error-codes.mjs"

validator-coverage-gaps:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-coverage-gaps.mjs"

validator-traceability:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-traceability.mjs"

validator-hygiene-full:
	@node "{{GOV_ROOT}}/roles/validator/checks/validator-hygiene-full.mjs"

sync-all-role-worktrees:
	@node "{{GOV_ROOT}}/roles_shared/scripts/topology/sync-all-role-worktrees.mjs"

reseed-permanent-worktree-from-main worktree_id approval label="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/topology/reseed-permanent-worktree-from-main.mjs" {{worktree_id}} --approve "{{approval}}" {{if label != "" { "--label \"" + label + "\"" } else { "" }}}

generate-worktree-cleanup-script wp-id role:
	@node "{{GOV_ROOT}}/roles_shared/scripts/topology/generate-worktree-cleanup-script.mjs" {{wp-id}} {{role}}

close-wp-branch wp-id approval remote="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/topology/close-wp-branch.mjs" {{wp-id}} {{remote}} --approve "{{approval}}"

wp-heartbeat wp-id actor_role actor_session current_phase runtime_status next_expected_actor waiting_on validator_trigger="" last_event="" worktree_dir="" next_expected_session="" waiting_on_session="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-heartbeat.mjs" {{wp-id}} {{actor_role}} {{actor_session}} {{current_phase}} {{runtime_status}} {{next_expected_actor}} "{{waiting_on}}" "{{validator_trigger}}" "{{last_event}}" "{{worktree_dir}}" "{{next_expected_session}}" "{{waiting_on_session}}"

wp-validator-query wp-id actor_role actor_session wp_validator_session summary correlation_id="" spec_anchor="" packet_row_ref="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" VALIDATOR_QUERY {{wp-id}} {{actor_role}} {{actor_session}} WP_VALIDATOR {{wp_validator_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-review-request wp-id actor_role actor_session target_role target_session summary correlation_id="" spec_anchor="" packet_row_ref="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" REVIEW_REQUEST {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-validator-response wp-id actor_role actor_session coder_session summary correlation_id spec_anchor="" packet_row_ref="" ack_for="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" VALIDATOR_RESPONSE {{wp-id}} {{actor_role}} {{actor_session}} CODER {{coder_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "{{ack_for}}" '{{microtask_json}}'

wp-review-response wp-id actor_role actor_session target_role target_session summary correlation_id spec_anchor="" packet_row_ref="" ack_for="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" REVIEW_RESPONSE {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "{{ack_for}}" '{{microtask_json}}'

generate-refinement-rubric *args:
	@node "{{GOV_ROOT}}/roles_shared/scripts/generate-refinement-rubric.mjs" {{args}}

send-mt wp-id mt-id description model="PRIMARY" *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/session/send-mt-prompt.mjs" {{wp-id}} {{mt-id}} "{{description}}" {{model}} {{FLAGS}}

install-mt-hook wp-id:
	@node "{{GOV_ROOT}}/roles_shared/scripts/hooks/install-mt-hook.mjs" {{wp-id}}

install-validator-guard wp-id:
	@node "{{GOV_ROOT}}/roles_shared/scripts/hooks/install-validator-guard.mjs" {{wp-id}}

wp-lane-health wp-id:
	@node "{{GOV_ROOT}}/roles_shared/scripts/session/wp-lane-health.mjs" {{wp-id}}

wp-relay-watchdog wp-id="" *FLAGS:
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/wp-relay-watchdog.mjs" {{wp-id}} {{FLAGS}}

# DEPRECATED: legacy failure-memory commands — redirect to governance memory DB.
# Prefer: just memory-capture procedural "<fix>" --scope "<file>" --wp WP-{ID}
# Prefer: just memory-search "<query>"
failure-memory-record category file_surface error_pattern fix_pattern wp_id="":
	@echo "[DEPRECATED] Redirecting to governance memory DB. Prefer: just memory-capture procedural"
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/failure-memory.mjs" record "{{category}}" "{{file_surface}}" "{{error_pattern}}" "{{fix_pattern}}" "{{wp_id}}"

failure-memory-query query:
	@echo "[DEPRECATED] Redirecting to governance memory DB. Prefer: just memory-search"
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/failure-memory.mjs" query "{{query}}"

# --- Governance Memory System (RGF-115 through RGF-143) ---

memory-add type topic summary *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" add {{type}} "{{topic}}" "{{summary}}" --raw-flags "{{FLAGS}}"

memory-search query *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" search "{{query}}" --raw-flags "{{FLAGS}}"

memory-prime wp-id *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" prime {{wp-id}} --raw-flags "{{FLAGS}}"

memory-stats:
	@node "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" stats

memory-decay *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" decay --raw-flags "{{FLAGS}}"

memory-migrate-failure-memory:
	@node "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" migrate-failure-memory

memory-extract wp-id="--all":
	@node "{{GOV_ROOT}}/roles_shared/scripts/memory/memory-extract-from-receipts.mjs" {{wp-id}}

memory-extract-smoketests file="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/memory/memory-extract-from-smoketests.mjs" {{file}}

memory-compact *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/memory/memory-compact.mjs" {{FLAGS}}

memory-embed *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" embed --raw-flags "{{FLAGS}}"

memory-hybrid-search query *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" hybrid-search "{{query}}" --raw-flags "{{FLAGS}}"

memory-capture type insight *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" capture {{type}} "{{insight}}" --raw-flags "{{FLAGS}}"

memory-flag id reason:
	@node "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" flag {{id}} "{{reason}}"

memory-intent-snapshot intent *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" intent-snapshot "{{intent}}" --raw-flags "{{FLAGS}}"

memory-recall action *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/memory-recall.mjs" {{action}} --raw-flags "{{FLAGS}}"

shell-with-memory role command_family command *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/shell-with-memory.mjs" {{role}} {{command_family}} "{{command}}" --raw-flags "{{FLAGS}}"

begin-refinement wp-id intent:
	@just repomem-gate
	@just memory-recall REFINEMENT --wp {{wp-id}}
	@just repomem context "{{intent}}" --trigger begin-refinement --wp {{wp-id}}
	@just memory-intent-snapshot "{{intent}}" --wp {{wp-id}} --role ORCHESTRATOR --reason "entering refinement" --expected "refined scope with discovery primitives"
	@echo "[INTENT_GATE] Intent captured for {{wp-id}}. Proceed with refinement analysis, research, and design."

begin-research intent *FLAGS:
	@just repomem-gate
	@just repomem context "{{intent}}" --trigger begin-research
	@just memory-intent-snapshot "{{intent}}" --role ORCHESTRATOR {{FLAGS}}
	@echo "[INTENT_GATE] Intent captured. Proceed with research."

memory-debug-snapshot *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/governance-memory-cli.mjs" debug-snapshot --raw-flags "{{FLAGS}}"

memory-patterns *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/memory-patterns.mjs" --raw-flags "{{FLAGS}}"

repomem subcommand content="" *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/repomem.mjs" {{subcommand}} "{{content}}" --raw-flags "{{FLAGS}}"

repomem-gate:
	@node "{{GOV_ROOT}}/roles_shared/scripts/memory/repomem.mjs" gate

memory-refresh *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/scripts/memory/memory-refresh.mjs" --raw-flags "{{FLAGS}}"

launch-memory-manager *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles/memory_manager/scripts/launch-memory-manager.mjs" --raw-flags "{{FLAGS}}"

memory-manager-proposal wp-id actor-session summary backup_ref="" correlation_id="":
	@node "{{GOV_ROOT}}/roles/memory_manager/scripts/memory-manager-receipt.mjs" {{wp-id}} "{{actor-session}}" MEMORY_PROPOSAL "{{summary}}" "{{backup_ref}}" "{{correlation_id}}" ORCHESTRATOR

memory-manager-flag-receipt wp-id actor-session summary backup_ref="" correlation_id="":
	@node "{{GOV_ROOT}}/roles/memory_manager/scripts/memory-manager-receipt.mjs" {{wp-id}} "{{actor-session}}" MEMORY_FLAG "{{summary}}" "{{backup_ref}}" "{{correlation_id}}" ORCHESTRATOR

memory-manager-rgf-candidate wp-id actor-session summary backup_ref="" correlation_id="":
	@node "{{GOV_ROOT}}/roles/memory_manager/scripts/memory-manager-receipt.mjs" {{wp-id}} "{{actor-session}}" MEMORY_RGF_CANDIDATE "{{summary}}" "{{backup_ref}}" "{{correlation_id}}" ORCHESTRATOR

memory-manager-startup:
	@just protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md"
	@just backup-status
	@just role-startup-topology-check
	@just launch-memory-manager --force
	@just memory-recall RESUME
	@echo ''
	@echo 'CHECKPOINT_REQUIRED: SESSION_OPEN'
	@echo 'Run: just repomem open "<what this session is about>" --role MEMORY_MANAGER'
	@echo ''

role-startup-topology-check *FLAGS:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/checks/role-startup-topology-check.mjs" --raw-flags "{{FLAGS}}"

launch-memory-manager-session host="AUTO" model="PRIMARY":
	@just launch-memory-manager --force
	@node -e "const ts = new Date().toISOString().replace(/[:.]/g,'').slice(0,15)+'Z'; const {spawnSync}=require('child_process'); spawnSync('node', ['{{GOV_ROOT}}/roles/orchestrator/scripts/launch-cli-session.mjs','MEMORY_MANAGER','WP-MEMORY-HYGIENE_'+ts,'{{host}}','{{model}}'], {stdio:'inherit'});"

activation-manager action wp-id="" *FLAGS:
	@if ("{{action}}" -eq "startup") { just --quiet protocol-ack "{{GOV_ROOT}}/codex/Handshake_Codex_v1.4.md" "{{MAIN_ROOT}}/AGENTS.md" "{{GOV_ROOT}}/roles_shared/docs/TOOLING_GUARDRAILS.md" "{{GOV_ROOT}}/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md"; just --quiet backup-status; just --quiet role-startup-topology-check; just --quiet gov-check; just --quiet memory-refresh; just --quiet memory-recall RESUME --role ACTIVATION_MANAGER }
	@node "{{GOV_ROOT}}/roles/activation_manager/scripts/activation-manager.mjs" {{action}} {{wp-id}} {{FLAGS}}; if ("{{action}}" -eq "readiness" -and $LASTEXITCODE -eq 2) { exit 0 } else { exit $LASTEXITCODE }

session-stall-scan role wp-id:
	@node "{{GOV_ROOT}}/roles_shared/scripts/session/session-stall-scan.mjs" {{role}} {{wp-id}}

mt-board wp-id:
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/mt-board.mjs" board {{wp-id}}

mt-claim wp-id session-key:
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/mt-board.mjs" claim {{wp-id}} {{session-key}}

mt-complete wp-id mt-id:
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/mt-board.mjs" complete {{wp-id}} {{mt-id}}

mt-populate wp-id:
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/mt-board.mjs" populate {{wp-id}}

closeout-repair wp-id *FLAGS:
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/closeout-repair.mjs" {{wp-id}} {{FLAGS}}

wp-closeout-format wp-id merged-main-commit:
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-closeout-format.mjs" {{wp-id}} {{merged-main-commit}}

record-refinement wp-id detail="":
	@node "{{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs" refine {{wp-id}} "{{detail}}"

record-signature wp-id signature workflow_lane="" execution_lane="":
	@node "{{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs" sign {{wp-id}} {{signature}} {{workflow_lane}} {{execution_lane}}

record-prepare wp-id workflow_lane="" execution_lane="" branch="" worktree_dir="":
	@node "{{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs" prepare {{wp-id}} {{workflow_lane}} {{execution_lane}} {{branch}} {{worktree_dir}}

record-role-model-profiles wp-id orchestrator_profile="" coder_profile="" wp_validator_profile="" integration_validator_profile="" activation_manager_profile="":
	@node "{{GOV_ROOT}}/roles/orchestrator/checks/orchestrator_gates.mjs" profiles {{wp-id}} {{orchestrator_profile}} {{coder_profile}} {{wp_validator_profile}} {{integration_validator_profile}} {{activation_manager_profile}}

create-task-packet wp-id context:
	@just repomem-gate
	@just memory-recall PACKET_CREATE --wp {{wp-id}}
	@just repomem context "{{context}}" --trigger create-task-packet --wp {{wp-id}}
	@echo "Creating task packet: {{wp-id}}..."
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/create-task-packet.mjs" {{wp-id}}
	@just build-order-sync

wp-traceability-set base_wp_id active_packet_wp_id context:
	@just repomem-gate
	@just repomem context "{{context}}" --trigger wp-traceability-set --wp {{base_wp_id}}
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/wp-traceability-set.mjs" {{base_wp_id}} {{active_packet_wp_id}}

wp-thread-append wp-id actor_role actor_session message target="" target_role="" target_session="" correlation_id="" requires_ack="" ack_for="" spec_anchor="" packet_row_ref="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-thread-append.mjs" {{wp-id}} {{actor_role}} {{actor_session}} "{{message}}" "{{target}}" "{{target_role}}" "{{target_session}}" "{{correlation_id}}" "{{requires_ack}}" "{{ack_for}}" "{{spec_anchor}}" "{{packet_row_ref}}"

wp-receipt-append wp-id actor_role actor_session receipt_kind summary state_before="" state_after="" target_role="" target_session="" correlation_id="" requires_ack="" ack_for="" spec_anchor="" packet_row_ref="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-receipt-append.mjs" {{wp-id}} {{actor_role}} {{actor_session}} {{receipt_kind}} "{{summary}}" "{{state_before}}" "{{state_after}}" "{{target_role}}" "{{target_session}}" "{{correlation_id}}" "{{requires_ack}}" "{{ack_for}}" "{{spec_anchor}}" "{{packet_row_ref}}"

wp-invalidity-flag wp-id actor_role actor_session invalidity_code summary spec_anchor="" packet_row_ref="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-invalidity-flag.mjs" {{wp-id}} {{actor_role}} {{actor_session}} {{invalidity_code}} "{{summary}}" "{{spec_anchor}}" "{{packet_row_ref}}"

wp-operator-rule-restatement wp-id actor_role actor_session summary spec_anchor="" packet_row_ref="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-operator-rule-restatement.mjs" {{wp-id}} {{actor_role}} {{actor_session}} "{{summary}}" "{{spec_anchor}}" "{{packet_row_ref}}"

wp-review-exchange receipt_kind wp-id actor_role actor_session target_role target_session summary correlation_id="" spec_anchor="" packet_row_ref="" ack_for="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" {{receipt_kind}} {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "{{ack_for}}" '{{microtask_json}}'

wp-spec-gap wp-id actor_role actor_session target_role target_session summary correlation_id="" spec_anchor="" packet_row_ref="" microtask_json="":
	@just wp-review-exchange SPEC_GAP {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-spec-confirmation wp-id actor_role actor_session target_role target_session summary correlation_id spec_anchor="" packet_row_ref="" ack_for="" microtask_json="":
	@just wp-review-exchange SPEC_CONFIRMATION {{wp-id}} {{actor_role}} {{actor_session}} {{target_role}} {{target_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "{{ack_for}}" '{{microtask_json}}'

wp-validator-kickoff wp-id actor_session coder_session summary spec_anchor="" packet_row_ref="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" VALIDATOR_KICKOFF {{wp-id}} WP_VALIDATOR {{actor_session}} CODER {{coder_session}} "{{summary}}" "" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-coder-intent wp-id actor_session validator_session summary correlation_id spec_anchor="" packet_row_ref="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" CODER_INTENT {{wp-id}} CODER {{actor_session}} WP_VALIDATOR {{validator_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-coder-handoff wp-id actor_session validator_session summary correlation_id="" spec_anchor="" packet_row_ref="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" CODER_HANDOFF {{wp-id}} CODER {{actor_session}} WP_VALIDATOR {{validator_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-validator-review wp-id actor_session coder_session summary correlation_id spec_anchor="" packet_row_ref="" microtask_json="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-review-exchange.mjs" VALIDATOR_REVIEW {{wp-id}} WP_VALIDATOR {{actor_session}} CODER {{coder_session}} "{{summary}}" "{{correlation_id}}" "{{spec_anchor}}" "{{packet_row_ref}}" "" '{{microtask_json}}'

wp-communication-health-check wp-id stage="STATUS" role="" session="":
	@node "{{GOV_ROOT}}/roles_shared/checks/wp-communication-health-check.mjs" {{wp-id}} {{stage}} {{role}} "{{session}}"

phase-check phase wp-id role="" session="" *args:
	@node "{{GOV_ROOT}}/roles_shared/scripts/lib/node-argv-proxy.mjs" "{{GOV_ROOT}}/roles_shared/checks/phase-check.mjs" {{phase}} {{wp-id}} {{role}} "{{session}}" --raw-flags "{{args}}"

check-notifications wp-id role="" session="":
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-check-notifications.mjs" {{wp-id}} {{role}} "{{session}}"

ack-notifications wp-id role session:
	@node "{{GOV_ROOT}}/roles_shared/scripts/wp/wp-check-notifications.mjs" {{wp-id}} {{role}} --ack --session={{session}}

orchestrator-prepare-and-packet wp-id workflow_lane="" execution_lane="" label="pre-wp-launch":
	@just worktree-add {{wp-id}}
	@just install-mt-hook {{wp-id}}
	@just memory-recall DELEGATION --wp {{wp-id}}
	@node "{{GOV_ROOT}}/roles/orchestrator/scripts/orchestrator-prepare-and-packet.mjs" {{wp-id}} {{workflow_lane}} {{execution_lane}}
	@echo "[ORCHESTRATOR] Committing governance checkpoint on gov_kernel..."
	@git add -A
	@git diff --cached --quiet; if ($LASTEXITCODE -ne 0) { git commit -m "gov: checkpoint packet+refinement+micro-tasks [{{wp-id}}]" }
	@echo "[ORCHESTRATOR] Creating backup snapshot before coder launch..."
	@just backup-snapshot "{{label}}"
